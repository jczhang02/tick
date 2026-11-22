//! The sync engine.
//!
//! [`SyncEngine`] owns the HTTP client and its own [`Store`] connection. One
//! [`SyncEngine::sync_once`] call:
//! 1. walks the queue in order: queued quick adds go to `POST /api/v1/tasks/quick`, runs of
//!    other commands go to `/sync` in batches of at most 100 together with the incremental
//!    read (a full read when there is no `sync_token` yet),
//! 2. applies each response (see [`Store::apply_sync_response`]): acknowledged commands
//!    leave the queue, `temp_id_mapping` is applied to still-queued commands, terminal
//!    command errors are dropped and recorded, local changes are reset to server state and
//!    the remaining queue is replayed on top ("server wins"),
//! 3. reports changes and status over an `async-channel`.
//!
//! Transient failures (`429`, `5xx`, retryable command errors) return
//! [`EngineError::Transient`] with the server's `retry_after`; network failures mark the
//! engine offline and keep the queue. [`Engine::spawn`] runs the loop on a tokio runtime in
//! a background thread with the cadence from `SPEC.md`.

use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;

use crate::api::{ApiError, Client, RESOURCE_TYPES};
use crate::command::QUICK_ADD;
use crate::store::{Changes, Clock, Kind, QueuedCommand, Store, StoreError};

/// Maximum commands per `/sync` request (`docs/api/pages/request-limits.md`).
pub const MAX_BATCH: usize = 100;

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
    /// A page of completed history was cached.
    CompletedLoaded {
        count: usize,
        has_more: bool,
    },
    /// Server results for an unsupported filter query were cached.
    FilterLoaded {
        query: String,
    },
    /// A background request failed (shown as a toast).
    Error(String),
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
        // Bound the loop: every iteration either removes commands from the queue or ends.
        let mut guard = 0;
        loop {
            guard += 1;
            let queue = self.store.pending_commands()?;
            let Some(first) = queue.first() else { break };
            if guard > queue.len() + 4 {
                break;
            }
            if first.command.kind == QUICK_ADD {
                self.push_quick_add(first.clone(), &mut report).await?;
                continue;
            }
            let batch: Vec<QueuedCommand> = queue
                .iter()
                .take_while(|q| q.command.kind != QUICK_ADD)
                .take(MAX_BATCH)
                .cloned()
                .collect();
            let before = queue.len();
            let retry = self.sync_request(&batch, &mut report).await?;
            read_done = true;
            if let Some(e) = retry {
                return Err(e);
            }
            if self.store.pending_count()? >= before {
                // The server did not settle any command (no sync_status entries); stop
                // instead of spinning. They are sent again next cycle with the same uuids.
                break;
            }
        }
        if !read_done && let Some(e) = self.sync_request(&[], &mut report).await? {
            return Err(e);
        }
        Ok(report)
    }

    /// One `/sync` request. Returns `Some(error)` when commands failed with a retryable
    /// error (their effect stays queued).
    async fn sync_request(
        &mut self,
        batch: &[QueuedCommand],
        report: &mut SyncReport,
    ) -> Result<Option<EngineError>, EngineError> {
        let token = self.store.sync_token()?.unwrap_or_else(|| "*".to_owned());
        let commands: Vec<_> = batch.iter().map(|q| q.command.clone()).collect();
        let resp = self.client.sync(&token, RESOURCE_TYPES, &commands).await?;
        report.requests += 1;
        report.full_sync |= resp.full_sync;
        let applied = self.store.apply_sync_response(batch, &resp)?;
        report.acknowledged += applied.acknowledged;
        for (q, e) in &applied.failed {
            report
                .failed
                .push(format!("{} failed: {}", q.command.kind, e.message()));
        }
        self.emit_changes(&applied.changes);
        report.changes.merge(applied.changes);
        if let Some((_, e)) = applied.retry.first() {
            return Ok(Some(EngineError::Transient {
                retry_after: e.retry_after(),
                message: e.message(),
            }));
        }
        Ok(None)
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
            Err(e) if e.is_transient() => {
                let _ = self.store.note_attempt(&q.command.uuid, &e.to_string());
                Err(e.into())
            }
            Err(ApiError::Unauthorized) => Err(ApiError::Unauthorized.into()),
            Err(e) => {
                // Rejected for good (400, 403, an unreadable response, ...): drop it so it
                // does not block the rest of the queue.
                let msg = format!("quick add failed: {e}");
                let changes = self.store.fail_command(&q, &msg, None)?;
                report.failed.push(msg);
                self.emit_changes(&changes);
                report.changes.merge(changes);
                Ok(())
            }
        }
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
            self.completed = Some(CompletedPager {
                since: now - chrono::Duration::days(COMPLETED_WINDOW_DAYS),
                until: now + chrono::Duration::minutes(1),
                cursor: None,
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
        let changes = self.store.cache_completed(&page.items)?;
        self.emit_changes(&changes);
        match page.next_cursor.filter(|c| !c.is_empty()) {
            Some(c) => pager.cursor = Some(c),
            None => {
                pager.until = pager.since;
                pager.since -= chrono::Duration::days(COMPLETED_WINDOW_DAYS);
                pager.cursor = None;
                pager.windows += 1;
            }
        }
        let has_more = pager.windows <= COMPLETED_MAX_WINDOWS;
        self.completed = Some(pager);
        self.emit(EngineEvent::CompletedLoaded { count, has_more });
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
                let items = self.client.tasks_filter_all(&text).await?;
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
    /// Defaults from the environment (`TICK_API_BASE`, `TICK_SYNC_INTERVAL_MS`).
    pub fn from_env(token: String, db_path: PathBuf) -> EngineConfig {
        let fg = std::env::var("TICK_SYNC_INTERVAL_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .map_or(Duration::from_secs(15), Duration::from_millis);
        EngineConfig {
            api_base: crate::api::api_base_from_env(),
            token,
            db_path,
            foreground_interval: fg,
            background_interval: Duration::from_secs(300),
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
    let mut attempt = 0u32;
    // When the next periodic sync is due (the first one right away).
    let mut next_sync = Instant::now();
    // Backoff after 429/5xx: no sync starts before this instant, whatever triggers it, and
    // the server's `retry_after` is never shortened.
    let mut not_before: Option<Instant> = None;
    loop {
        let wait = next_sync.saturating_duration_since(Instant::now());
        let mut want_sync = false;
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
                if let Err(e) = engine.fetch_completed(restart).await {
                    let _ = events.try_send(EngineEvent::Error(e.to_string()));
                }
            }
            Ok(Ok(Request::FetchFilter(q))) => {
                if let Err(e) = engine.fetch_filter(&q).await {
                    let _ = events.try_send(EngineEvent::Error(e.to_string()));
                }
            }
            Ok(Ok(Request::SetToken(t))) => {
                engine.set_client(Client::new(config.api_base.clone(), t));
                not_before = None;
                want_sync = true;
            }
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
        if let Some(nb) = not_before
            && now < nb
        {
            next_sync = nb;
            continue;
        }
        if engine.status().state == ConnState::Unauthorized {
            next_sync = now + interval;
            continue;
        }
        match engine.sync_once().await {
            Ok(_) => {
                attempt = 0;
                not_before = None;
                next_sync = Instant::now() + interval;
            }
            Err(EngineError::Api(ApiError::Network(_))) => {
                // Offline: retry periodically; a user action may try earlier.
                next_sync = Instant::now() + interval.min(Duration::from_secs(30));
            }
            Err(e) if e.is_transient() => {
                attempt += 1;
                let delay = backoff_delay(&e, attempt, Duration::from_secs(2));
                not_before = Some(Instant::now() + delay);
                next_sync = Instant::now() + delay;
            }
            Err(_) => next_sync = Instant::now() + interval,
        }
    }
}
