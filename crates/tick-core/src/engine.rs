//! The sync engine.
//!
//! [`SyncEngine`] owns the HTTP client and its own [`Store`] connection. One
//! [`SyncEngine::sync_once`] call:
//! 1. walks the queue in order up to the first deferred command (undo window): queued quick
//!    adds go to `POST /api/v1/tasks/quick`, runs of other commands go to `/sync` in batches
//!    of at most 100 commands and [`MAX_BATCH_BYTES`] form-encoded, together with the
//!    incremental read (a full read when there is no `sync_token` yet),
//! 2. applies each response (see [`Store::apply_sync_response`]): acknowledged commands
//!    leave the queue, `temp_id_mapping` is applied to still-queued commands and remembered
//!    for commands enqueued later, terminal command errors are dropped and recorded, local
//!    changes are reset to server state and the remaining queue is replayed on top ("server
//!    wins"),
//! 3. reports changes (including `id_map`) and status over an `async-channel`.
//!
//! Transient failures (`429`, `5xx`, retryable command errors) return
//! [`EngineError::Transient`] with the server's `retry_after`; network failures mark the
//! engine offline and keep the queue. A command that keeps failing is dropped after
//! [`MAX_ATTEMPTS`] answered attempts. When the server refuses a whole `/sync` request
//! (`400`/`403`/`413`), the engine checks that a request without commands passes, then
//! bisects the batch and drops the one command that is refused on its own. [`Engine::spawn`]
//! runs the loop on a tokio runtime in a background thread with the cadence from `SPEC.md`;
//! inside a backoff window no request of any kind goes out.

use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;

use crate::api::{ApiError, Client, RESOURCE_TYPES};
use crate::command::QUICK_ADD;
use crate::store::{Changes, Clock, Kind, MAX_ATTEMPTS, QueuedCommand, Store, StoreError, wall_ms};

/// Maximum commands per `/sync` request (`docs/api/pages/request-limits.md`).
pub const MAX_BATCH: usize = 100;

/// Budget for the form-encoded `commands` field of one `/sync` request. The documented limit
/// is 1 MiB for the whole request body (`docs/api/pages/request-limits.md`); the rest of the
/// body (`sync_token`, `resource_types`) is small.
pub const MAX_BATCH_BYTES: usize = 900 * 1024;

/// Length of `s` once `application/x-www-form-urlencoded` (what `reqwest` sends): unreserved
/// bytes and the space (`+`) stay one byte, everything else becomes `%XX`.
fn form_encoded_len(s: &str) -> usize {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || matches!(b, b'*' | b'-' | b'.' | b'_' | b' ') {
                1
            } else {
                3
            }
        })
        .sum()
}

/// The next `/sync` batch: a prefix of the queue up to the first quick add or deferred
/// command, at most `cap` commands and [`MAX_BATCH_BYTES`] encoded (always at least one
/// command, so an oversized one is sent alone and rejected on its own).
fn next_batch(queue: &[QueuedCommand], now_ms: i64, cap: usize) -> Vec<QueuedCommand> {
    let mut out = Vec::new();
    // `[` and `]`.
    let mut bytes = 6;
    for q in queue {
        if out.len() >= cap.min(MAX_BATCH) || q.command.kind == QUICK_ADD || !q.is_due(now_ms) {
            break;
        }
        let json = serde_json::to_string(&q.command).unwrap_or_default();
        // Plus the separating comma.
        let len = form_encoded_len(&json) + 3;
        if !out.is_empty() && bytes + len > MAX_BATCH_BYTES {
            break;
        }
        bytes += len;
        out.push(q.clone());
    }
    out
}

/// Whether the server refused a request as a whole in a way that resending it unchanged
/// cannot fix (`400`, `403`, `413`, ...). Transient (`429`, `5xx`), network and `401` errors
/// are handled elsewhere.
fn rejects_request(e: &ApiError) -> bool {
    matches!(
        e,
        ApiError::Forbidden(_)
            | ApiError::Request {
                status: 400 | 413 | 414 | 422,
                ..
            }
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnState {
    Idle,
    Syncing,
    Online,
    Offline,
    /// The token was rejected (401); syncing stops until a new token is set.
    Unauthorized,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SyncStatus {
    pub state: ConnState,
    /// Commands waiting to be sent (including offline quick adds).
    pub pending: usize,
    /// Commands the server rejected (see [`Store::failed_commands`]).
    pub failed: usize,
    pub last_error: Option<String>,
    /// UTC time of the last successful sync.
    pub last_sync: Option<String>,
}

impl Default for SyncStatus {
    fn default() -> Self {
        SyncStatus {
            state: ConnState::Idle,
            pending: 0,
            failed: 0,
            last_error: None,
            last_sync: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum EngineEvent {
    /// Objects changed in the store; refresh the affected models.
    Changed(Changes),
    Status(SyncStatus),
    /// A page of completed history was cached. `since` is the start of the window the page
    /// came from (RFC 3339, UTC): the history is complete from there on.
    CompletedLoaded {
        count: usize,
        has_more: bool,
        since: String,
    },
    /// Server results for an unsupported filter query were cached.
    FilterLoaded {
        query: String,
    },
    /// A history or filter request failed (shown as a toast or an error state).
    Error(RequestFailure),
}

/// Which background request a [`RequestFailure`] belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailedRequest {
    /// A page of completed history.
    Completed,
    /// Server results for a filter query (the full query passed to
    /// [`EngineHandle::fetch_filter`]).
    Filter(String),
}

/// A failed history or filter request, structured so the UI can choose between "offline",
/// "the server rejected this" and "try again".
#[derive(Debug, Clone, PartialEq)]
pub struct RequestFailure {
    pub request: FailedRequest,
    pub message: String,
    /// No connection (or the token was rejected): the request can work later unchanged.
    pub offline: bool,
    /// The server refused the request itself (`4xx` other than `401` and `429`), for
    /// example a filter query it cannot parse. Retrying the same request will not help.
    pub rejected: bool,
}

impl RequestFailure {
    fn new(req: &SideRequest, e: &EngineError) -> RequestFailure {
        let (offline, rejected) = match e {
            EngineError::Api(ApiError::Network(_) | ApiError::Unauthorized) => (true, false),
            EngineError::Api(ApiError::Request { .. } | ApiError::Forbidden(_)) => (false, true),
            _ => (false, false),
        };
        RequestFailure {
            request: match req {
                SideRequest::Completed { .. } => FailedRequest::Completed,
                SideRequest::Filter(q) => FailedRequest::Filter(q.clone()),
            },
            message: e.to_string(),
            offline,
            rejected,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error(transparent)]
    Api(#[from] ApiError),
    #[error(transparent)]
    Store(#[from] StoreError),
    /// Retry later; `retry_after` is the server's hint in seconds.
    #[error("temporary failure, retry later ({message})")]
    Transient {
        retry_after: Option<u64>,
        message: String,
    },
}

impl EngineError {
    pub fn retry_after(&self) -> Option<u64> {
        match self {
            EngineError::Api(e) => e.retry_after(),
            EngineError::Transient { retry_after, .. } => *retry_after,
            EngineError::Store(_) => None,
        }
    }
    pub fn is_transient(&self) -> bool {
        match self {
            EngineError::Api(e) => e.is_transient(),
            EngineError::Transient { .. } => true,
            EngineError::Store(_) => false,
        }
    }
}

/// Result of one successful sync cycle.
#[derive(Debug, Clone, Default)]
pub struct SyncReport {
    pub changes: Changes,
    pub requests: usize,
    pub acknowledged: usize,
    pub full_sync: bool,
    /// Error messages of commands the server rejected in this cycle.
    pub failed: Vec<String>,
}

/// State of the completed-history pager.
#[derive(Debug, Clone)]
struct CompletedPager {
    since: chrono::DateTime<chrono::Utc>,
    until: chrono::DateTime<chrono::Utc>,
    cursor: Option<String>,
    /// Upper bound (`completed_at`, exclusive) of what the next page covers: the window's
    /// end for its first page, else the oldest completion of the previous page.
    upper: String,
    windows: u32,
}

/// Days per completed-history window (`SPEC.md`: the last 30 days first).
const COMPLETED_WINDOW_DAYS: i64 = 30;
/// How far back "Load more" goes (24 windows, about two years).
const COMPLETED_MAX_WINDOWS: u32 = 24;

pub struct SyncEngine {
    store: Store,
    client: Client,
    events: Option<async_channel::Sender<EngineEvent>>,
    status: SyncStatus,
    completed: Option<CompletedPager>,
}

fn rfc3339(t: chrono::DateTime<chrono::Utc>) -> String {
    t.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// An instant written like the API's `completed_at` (`2025-01-21T21:28:43.841504Z`), so
/// cached rows compare with it as strings.
fn completed_stamp(t: chrono::DateTime<chrono::Utc>) -> String {
    t.format("%Y-%m-%dT%H:%M:%S%.6fZ").to_string()
}

impl SyncEngine {
    pub fn new(store: Store, client: Client) -> SyncEngine {
        SyncEngine {
            store,
            client,
            events: None,
            status: SyncStatus::default(),
            completed: None,
        }
    }

    pub fn with_events(mut self, tx: async_channel::Sender<EngineEvent>) -> SyncEngine {
        self.events = Some(tx);
        self
    }

    pub fn store(&self) -> &Store {
        &self.store
    }

    pub fn store_mut(&mut self) -> &mut Store {
        &mut self.store
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Replace the HTTP client (new token); clears the unauthorized state.
    pub fn set_client(&mut self, client: Client) {
        self.client = client;
        if self.status.state == ConnState::Unauthorized {
            self.status.state = ConnState::Idle;
        }
    }

    pub fn status(&self) -> &SyncStatus {
        &self.status
    }

    fn emit(&self, e: EngineEvent) {
        if let Some(tx) = &self.events {
            let _ = tx.try_send(e);
        }
    }

    fn emit_changes(&self, c: &Changes) {
        if !c.is_empty() {
            self.emit(EngineEvent::Changed(c.clone()));
        }
    }

    fn refresh_counts(&mut self) {
        self.status.pending = self.store.pending_count().unwrap_or(0);
        self.status.failed = self.store.failed_commands().map_or(0, |f| f.len());
    }

    fn set_state(&mut self, state: ConnState, error: Option<String>) {
        self.status.state = state;
        if error.is_some() || state == ConnState::Online {
            self.status.last_error = error;
        }
        self.refresh_counts();
        self.emit(EngineEvent::Status(self.status.clone()));
    }

    fn fail(&mut self, e: EngineError) -> EngineError {
        let state = match &e {
            EngineError::Api(ApiError::Network(_)) => ConnState::Offline,
            EngineError::Api(ApiError::Unauthorized) => ConnState::Unauthorized,
            _ => ConnState::Online,
        };
        self.status.state = state;
        self.status.last_error = Some(e.to_string());
        self.refresh_counts();
        self.emit(EngineEvent::Status(self.status.clone()));
        e
    }

    /// One sync cycle: push the queue (quick adds and command batches, in order) and read
    /// changes. Always performs at least one `/sync` read.
    pub async fn sync_once(&mut self) -> Result<SyncReport, EngineError> {
        self.set_state(ConnState::Syncing, None);
        match self.sync_inner().await {
            Ok(report) => {
                self.status.last_sync = Some(self.store.now().utc_string());
                let err = report.failed.last().cloned();
                self.set_state(ConnState::Online, None);
                if err.is_some() {
                    self.status.last_error = err;
                    self.emit(EngineEvent::Status(self.status.clone()));
                }
                Ok(report)
            }
            Err(e) => Err(self.fail(e)),
        }
    }

    async fn sync_inner(&mut self) -> Result<SyncReport, EngineError> {
        let mut report = SyncReport::default();
        let mut read_done = false;
        // Prefix size cap for the next `/sync` batch. Halved when the server rejects a whole
        // request, doubled again after a request goes through.
        let mut cap = MAX_BATCH;
        // Whether a command-less request was tried after a request-level rejection.
        let mut read_checked = false;
        // Every iteration settles at least one command, halves `cap` or ends the cycle.
        // Halvings are bounded by log2(MAX_BATCH) plus the doublings, which follow
        // successes, so the bound is computed once from the initial queue length.
        let limit = 2 * self.store.pending_count()? + 2 * MAX_BATCH.ilog2() as usize + 4;
        for _ in 0..limit {
            let now_ms = wall_ms();
            let queue = self.store.pending_commands()?;
            let Some(first) = queue.first() else { break };
            if !first.is_due(now_ms) {
                // Deferred (undo window): keep the order, send nothing behind it yet.
                break;
            }
            if first.command.kind == QUICK_ADD {
                self.push_quick_add(first.clone(), &mut report).await?;
                continue;
            }
            let batch = next_batch(&queue, now_ms, cap);
            match self.sync_request(&batch, &mut report).await {
                Ok((settled, retry)) => {
                    read_done = true;
                    if let Some(e) = retry {
                        return Err(e);
                    }
                    if settled == 0 {
                        // The server did not settle any command (no sync_status entries);
                        // stop instead of spinning. They are sent again next cycle with the
                        // same uuids.
                        break;
                    }
                    cap = (cap * 2).min(MAX_BATCH);
                }
                Err(EngineError::Api(e)) if rejects_request(&e) => {
                    // The server refused the whole request (400/403/413, not a per-command
                    // error). If a request without commands is refused too, no command is to
                    // blame: keep the queue and let the caller back off.
                    if !read_checked {
                        read_checked = true;
                        if let (_, Some(retry)) = self.sync_request(&[], &mut report).await? {
                            return Err(retry);
                        }
                        read_done = true;
                    }
                    if batch.len() > 1 {
                        // Bisect: send the first half (a prefix keeps the queue order).
                        cap = batch.len() / 2;
                        continue;
                    }
                    // One command that the server refuses on its own: drop it, roll back
                    // its local effect and let the rest of the queue through.
                    let q = &batch[0];
                    let msg = format!("{} failed: {e}", q.command.kind);
                    let changes = self.store.fail_command(q, &msg, None)?;
                    report.failed.push(msg);
                    self.emit_changes(&changes);
                    report.changes.merge(changes);
                }
                Err(e) => return Err(e),
            }
        }
        if !read_done && let (_, Some(e)) = self.sync_request(&[], &mut report).await? {
            return Err(e);
        }
        Ok(report)
    }

    /// One `/sync` request. Returns how many sent commands were settled (acknowledged or
    /// dropped) and `Some(error)` when commands failed with a retryable error (their effect
    /// stays queued).
    async fn sync_request(
        &mut self,
        batch: &[QueuedCommand],
        report: &mut SyncReport,
    ) -> Result<(usize, Option<EngineError>), EngineError> {
        let token = self.store.sync_token()?.unwrap_or_else(|| "*".to_owned());
        let commands: Vec<_> = batch.iter().map(|q| q.command.clone()).collect();
        let resp = self.client.sync(&token, RESOURCE_TYPES, &commands).await?;
        report.requests += 1;
        report.full_sync |= resp.full_sync;
        let applied = self.store.apply_sync_response(batch, &resp)?;
        report.acknowledged += applied.acknowledged;
        let settled = applied.acknowledged + applied.failed.len();
        for (q, e) in &applied.failed {
            report
                .failed
                .push(format!("{} failed: {}", q.command.kind, e.message()));
        }
        self.emit_changes(&applied.changes);
        report.changes.merge(applied.changes);
        if let Some((_, e)) = applied.retry.first() {
            return Ok((
                settled,
                Some(EngineError::Transient {
                    retry_after: e.retry_after(),
                    message: e.message(),
                }),
            ));
        }
        Ok((settled, None))
    }

    async fn push_quick_add(
        &mut self,
        q: QueuedCommand,
        report: &mut SyncReport,
    ) -> Result<(), EngineError> {
        let text = q
            .command
            .args
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or("");
        match self.client.quick_add(text, &q.command.uuid).await {
            Ok(item) => {
                let changes = self.store.apply_quick_add_result(&q, &item)?;
                report.acknowledged += 1;
                self.emit_changes(&changes);
                report.changes.merge(changes);
                Ok(())
            }
            // Offline: keep it; going offline is not an attempt.
            Err(e @ ApiError::Network(_)) => Err(e.into()),
            Err(ApiError::Unauthorized) => Err(ApiError::Unauthorized.into()),
            Err(e) if rejects_request(&e) => {
                // Rejected for good (400, 403, too large): drop it so it does not block the
                // rest of the queue. The text stays visible in the failed list.
                self.drop_quick_add(&q, format!("quick add failed: {e}"), report)
            }
            // 429/5xx are about the API, not about this text (like request-level errors on
            // `/sync`): retry without counting an attempt.
            Err(e) if e.is_transient() => Err(e.into()),
            Err(e) => {
                // An unreadable answer (for example a captive portal page) or another 4xx:
                // retry later, but not forever.
                if q.attempts + 1 >= MAX_ATTEMPTS {
                    let msg = format!("quick add failed after {MAX_ATTEMPTS} attempts: {e}");
                    return self.drop_quick_add(&q, msg, report);
                }
                let _ = self.store.note_attempt(&q.command.uuid, &e.to_string());
                Err(EngineError::Transient {
                    retry_after: e.retry_after(),
                    message: e.to_string(),
                })
            }
        }
    }

    fn drop_quick_add(
        &mut self,
        q: &QueuedCommand,
        msg: String,
        report: &mut SyncReport,
    ) -> Result<(), EngineError> {
        let changes = self.store.fail_command(q, &msg, None)?;
        report.failed.push(msg);
        self.emit_changes(&changes);
        report.changes.merge(changes);
        Ok(())
    }

    /// Run [`SyncEngine::sync_once`], waiting and retrying after transient errors
    /// (`retry_after` when the server gives one, else exponential backoff from `base`).
    /// Network errors are returned immediately (the caller decides when to try again).
    pub async fn sync_with_retry(
        &mut self,
        max_attempts: u32,
        base: Duration,
    ) -> Result<SyncReport, EngineError> {
        let mut attempt = 0;
        loop {
            attempt += 1;
            match self.sync_once().await {
                Ok(r) => return Ok(r),
                Err(e)
                    if attempt < max_attempts
                        && e.is_transient()
                        && !matches!(e, EngineError::Api(ApiError::Network(_))) =>
                {
                    tokio::time::sleep(backoff_delay(&e, attempt, base)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Fetch one page of completed history into the cache. `restart` begins again with the
    /// last 30 days; otherwise the next page ("Load more") is fetched.
    pub async fn fetch_completed(&mut self, restart: bool) -> Result<(usize, bool), EngineError> {
        let now = self.store.now().utc();
        if restart || self.completed.is_none() {
            let until = now + chrono::Duration::minutes(1);
            self.completed = Some(CompletedPager {
                since: now - chrono::Duration::days(COMPLETED_WINDOW_DAYS),
                until,
                cursor: None,
                upper: completed_stamp(until),
                windows: 1,
            });
        }
        let mut pager = self.completed.clone().expect("pager");
        if pager.windows > COMPLETED_MAX_WINDOWS {
            return Ok((0, false));
        }
        let page = self
            .client
            .completed_by_completion_date(
                &rfc3339(pager.since),
                &rfc3339(pager.until),
                pager.cursor.as_deref(),
                50,
            )
            .await
            .map_err(|e| self.fail(e.into()))?;
        let count = page.items.len();
        let window_since = rfc3339(pager.since);
        let next = page.next_cursor.filter(|c| !c.is_empty());
        // The page holds every completion from its oldest one (or, as the window's last
        // page, from the window's start) up to `upper`: cached rows in that range that it
        // does not hold are gone on the server.
        let oldest = page
            .items
            .iter()
            .filter_map(|v| v.get("completed_at").and_then(Value::as_str))
            .min()
            .map(str::to_owned);
        let lower = match (&next, &oldest) {
            (None, _) => completed_stamp(pager.since),
            (Some(_), Some(o)) => o.clone(),
            (Some(_), None) => pager.upper.clone(),
        };
        let changes = self
            .store
            .cache_completed_range(&page.items, Some((&lower, &pager.upper)))?;
        self.emit_changes(&changes);
        match next {
            Some(c) => {
                pager.cursor = Some(c);
                pager.upper = lower;
            }
            None => {
                pager.until = pager.since;
                pager.since -= chrono::Duration::days(COMPLETED_WINDOW_DAYS);
                pager.cursor = None;
                pager.upper = completed_stamp(pager.until);
                pager.windows += 1;
            }
        }
        let has_more = pager.windows <= COMPLETED_MAX_WINDOWS;
        self.completed = Some(pager);
        self.emit(EngineEvent::CompletedLoaded {
            count,
            has_more,
            since: window_since,
        });
        Ok((count, has_more))
    }

    /// Fetch every page of a completed-history window ending now (used by tests and by a
    /// full refresh). Returns the number of tasks cached.
    pub async fn fetch_completed_window(&mut self, days: i64) -> Result<usize, EngineError> {
        let now = self.store.now().utc();
        let since = rfc3339(now - chrono::Duration::days(days));
        let until = rfc3339(now + chrono::Duration::minutes(1));
        let mut cursor: Option<String> = None;
        let mut total = 0;
        loop {
            let page = self
                .client
                .completed_by_completion_date(&since, &until, cursor.as_deref(), 50)
                .await?;
            total += page.items.len();
            let changes = self.store.cache_completed(&page.items)?;
            self.emit_changes(&changes);
            match page.next_cursor.filter(|c| !c.is_empty()) {
                Some(c) => cursor = Some(c),
                None => break,
            }
        }
        Ok(total)
    }

    /// Ask the server for the parts of a filter query the local evaluator cannot handle
    /// (`GET /api/v1/tasks/filter`, one request per comma-separated part) and cache them.
    pub async fn fetch_filter(&mut self, query: &str) -> Result<Changes, EngineError> {
        let mut all = Changes::default();
        for g in crate::filter::parse(query) {
            if let crate::filter::Group::Unsupported { text, .. } = g {
                let items = self
                    .client
                    .tasks_filter_all(&text)
                    .await
                    .map_err(|e| self.fail(e.into()))?;
                let changes = self.store.cache_filter_results(&text, &items)?;
                self.emit(EngineEvent::FilterLoaded {
                    query: text.clone(),
                });
                all.merge(changes);
            }
        }
        all.touch(Kind::Filter);
        self.emit_changes(&all);
        Ok(all)
    }
}

/// Delay before the next attempt after a transient error.
pub fn backoff_delay(e: &EngineError, attempt: u32, base: Duration) -> Duration {
    if let Some(s) = e.retry_after() {
        return Duration::from_secs(s);
    }
    let factor = 2u32.saturating_pow(attempt.saturating_sub(1).min(10));
    (base * factor).min(Duration::from_secs(300))
}

// ---- background engine ----

/// Configuration for [`Engine::spawn`].
#[derive(Clone)]
pub struct EngineConfig {
    pub api_base: String,
    pub token: String,
    pub db_path: PathBuf,
    /// Sync cadence while the window is visible (`SPEC.md`: 15 s; `TICK_SYNC_INTERVAL_MS`).
    pub foreground_interval: Duration,
    /// Sync cadence in the background (`SPEC.md`: 5 min).
    pub background_interval: Duration,
    pub clock: Option<Clock>,
}

impl EngineConfig {
    /// Defaults from the environment (`TICK_API_BASE`, `TICK_SYNC_INTERVAL_MS`,
    /// `TICK_BACKGROUND_SYNC_MS`).
    pub fn from_env(token: String, db_path: PathBuf) -> EngineConfig {
        let ms = |key: &str, default: Duration| {
            std::env::var(key)
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .map_or(default, Duration::from_millis)
        };
        let fg = ms("TICK_SYNC_INTERVAL_MS", Duration::from_secs(15));
        let bg = ms("TICK_BACKGROUND_SYNC_MS", Duration::from_secs(300));
        EngineConfig {
            api_base: crate::api::api_base_from_env(),
            token,
            db_path,
            foreground_interval: fg,
            background_interval: bg,
            clock: None,
        }
    }
}

#[derive(Debug)]
enum Request {
    SyncNow,
    Foreground(bool),
    LoadCompleted { restart: bool },
    FetchFilter(String),
    SetToken(String),
    Shutdown,
}

/// Handle to a running background engine. Cheap to clone; dropping all handles does not
/// stop the engine, call [`EngineHandle::shutdown`].
#[derive(Clone, Debug)]
pub struct EngineHandle {
    tx: async_channel::Sender<Request>,
}

impl EngineHandle {
    /// Sync as soon as possible (after an edit, on focus).
    pub fn sync_now(&self) {
        let _ = self.tx.try_send(Request::SyncNow);
    }
    /// Switch between the foreground (15 s) and background (5 min) cadence.
    pub fn set_foreground(&self, foreground: bool) {
        let _ = self.tx.try_send(Request::Foreground(foreground));
    }
    /// Load the first (`restart`) or next page of completed history.
    pub fn load_completed(&self, restart: bool) {
        let _ = self.tx.try_send(Request::LoadCompleted { restart });
    }
    /// Fetch server results for the unsupported parts of a filter query.
    pub fn fetch_filter(&self, query: &str) {
        let _ = self.tx.try_send(Request::FetchFilter(query.to_owned()));
    }
    /// Use a new API token (after the user pasted one) and sync.
    pub fn set_token(&self, token: &str) {
        let _ = self.tx.try_send(Request::SetToken(token.to_owned()));
    }
    pub fn shutdown(&self) {
        let _ = self.tx.try_send(Request::Shutdown);
    }
}

pub struct Engine;

impl Engine {
    /// Start the engine on its own thread with a tokio runtime. Returns the handle and the
    /// event receiver (use it from the UI with `glib::spawn_future_local`).
    pub fn spawn(
        config: EngineConfig,
    ) -> Result<(EngineHandle, async_channel::Receiver<EngineEvent>), StoreError> {
        let mut store = Store::open(&config.db_path)?;
        if let Some(clock) = &config.clock {
            store.set_clock(clock.clone());
        }
        let (req_tx, req_rx) = async_channel::unbounded();
        let (ev_tx, ev_rx) = async_channel::unbounded();
        std::thread::Builder::new()
            .name("tick-sync".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("tokio runtime");
                rt.block_on(run(config, store, req_rx, ev_tx));
            })
            .map_err(StoreError::Io)?;
        Ok((EngineHandle { tx: req_tx }, ev_rx))
    }
}

/// A request besides syncing that also talks to the API.
#[derive(Debug, Clone, PartialEq)]
enum SideRequest {
    Completed { restart: bool },
    Filter(String),
}

async fn run_side(engine: &mut SyncEngine, req: &SideRequest) -> Result<(), EngineError> {
    match req {
        SideRequest::Completed { restart } => engine.fetch_completed(*restart).await.map(|_| ()),
        SideRequest::Filter(q) => engine.fetch_filter(q).await.map(|_| ()),
    }
}

/// Backoff state of the background loop.
struct Backoff {
    attempt: u32,
    /// No request starts before this instant, whatever triggers it, and the server's
    /// `retry_after` is never shortened.
    not_before: Option<tokio::time::Instant>,
}

impl Backoff {
    fn active(&self) -> Option<tokio::time::Instant> {
        self.not_before
            .filter(|nb| tokio::time::Instant::now() < *nb)
    }
    fn reset(&mut self) {
        self.attempt = 0;
        self.not_before = None;
    }
    /// Start (or extend) a backoff window after `e`; returns its end.
    fn after(&mut self, e: &EngineError) -> tokio::time::Instant {
        self.attempt += 1;
        let at =
            tokio::time::Instant::now() + backoff_delay(e, self.attempt, Duration::from_secs(2));
        let at = self.not_before.map_or(at, |nb| nb.max(at));
        self.not_before = Some(at);
        at
    }
}

async fn run(
    config: EngineConfig,
    store: Store,
    rx: async_channel::Receiver<Request>,
    events: async_channel::Sender<EngineEvent>,
) {
    use tokio::time::Instant;
    let client = Client::new(config.api_base.clone(), config.token.clone());
    let mut engine = SyncEngine::new(store, client).with_events(events.clone());
    let mut foreground = true;
    let mut backoff = Backoff {
        attempt: 0,
        not_before: None,
    };
    // When the next periodic sync is due (the first one right away).
    let mut next_sync = Instant::now();
    // History/filter requests that arrived inside a backoff window; run when it ends.
    let mut deferred: Vec<SideRequest> = Vec::new();
    loop {
        // Wake up when a deferred command (undo window) becomes due, whatever set
        // `next_sync` (switching to the 5 min cadence must not hold back a delete).
        if let Ok(Some(due)) = engine.store().next_deferred_due() {
            next_sync = next_sync.min(Instant::now() + due + Duration::from_millis(20));
        }
        let wait = next_sync.saturating_duration_since(Instant::now());
        let mut want_sync = false;
        let mut side = None;
        match tokio::time::timeout(wait, rx.recv()).await {
            Err(_) => want_sync = true, // timer
            Ok(Err(_)) => return,       // all handles dropped
            Ok(Ok(Request::Shutdown)) => return,
            Ok(Ok(Request::SyncNow)) => want_sync = true,
            Ok(Ok(Request::Foreground(f))) => {
                let was = foreground;
                foreground = f;
                want_sync = f && !was;
                if !f {
                    next_sync = Instant::now() + config.background_interval;
                }
            }
            Ok(Ok(Request::LoadCompleted { restart })) => {
                side = Some(SideRequest::Completed { restart });
            }
            Ok(Ok(Request::FetchFilter(q))) => side = Some(SideRequest::Filter(q)),
            Ok(Ok(Request::SetToken(t))) => {
                engine.set_client(Client::new(config.api_base.clone(), t));
                backoff.reset();
                want_sync = true;
            }
        }
        if let Some(req) = side {
            if engine.status().state == ConnState::Unauthorized {
                let e = EngineError::Api(ApiError::Unauthorized);
                let _ = events.try_send(EngineEvent::Error(RequestFailure::new(&req, &e)));
            } else if let Some(nb) = backoff.active() {
                // Inside a 429/5xx window: run it when the window ends.
                if !deferred.contains(&req) {
                    deferred.push(req);
                }
                next_sync = next_sync.min(nb);
            } else if let Err(e) = run_side(&mut engine, &req).await {
                if e.is_transient() && !matches!(e, EngineError::Api(ApiError::Network(_))) {
                    next_sync = next_sync.max(backoff.after(&e));
                }
                let _ = events.try_send(EngineEvent::Error(RequestFailure::new(&req, &e)));
            }
            continue;
        }
        if !want_sync {
            continue;
        }
        let interval = if foreground {
            config.foreground_interval
        } else {
            config.background_interval
        };
        let now = Instant::now();
        if let Some(nb) = backoff.active() {
            next_sync = nb;
            continue;
        }
        if engine.status().state == ConnState::Unauthorized {
            next_sync = now + interval;
            continue;
        }
        // The window is over: first the requests that waited for it.
        let mut limited = false;
        for req in std::mem::take(&mut deferred) {
            if limited {
                deferred.push(req);
                continue;
            }
            if let Err(e) = run_side(&mut engine, &req).await {
                if e.is_transient() && !matches!(e, EngineError::Api(ApiError::Network(_))) {
                    next_sync = backoff.after(&e);
                    limited = true;
                    deferred.push(req);
                    continue;
                }
                let _ = events.try_send(EngineEvent::Error(RequestFailure::new(&req, &e)));
            }
        }
        if limited {
            continue;
        }
        match engine.sync_once().await {
            Ok(_) => {
                backoff.reset();
                next_sync = Instant::now() + interval;
            }
            Err(EngineError::Api(ApiError::Network(_))) => {
                // Offline: retry periodically; a user action may try earlier.
                next_sync = Instant::now() + interval.min(Duration::from_secs(30));
            }
            Err(EngineError::Api(ApiError::Unauthorized)) => next_sync = Instant::now() + interval,
            Err(e) => {
                // 429/5xx, and also errors that resending cannot fix right away (a read the
                // server refuses, a local database error): back off instead of hammering.
                next_sync = backoff.after(&e);
            }
        }
    }
}
