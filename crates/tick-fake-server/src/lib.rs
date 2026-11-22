//! In-memory Todoist API v1 subset for tests.
//!
//! Modeled on `docs/api/` (the OpenAPI spec and doc pages), not on Tick's client:
//! - `POST /api/v1/sync`: form-encoded `sync_token`, `resource_types` and `commands` (both
//!   JSON strings). Real incremental semantics: every object carries the revision of its last
//!   change; a token returns only objects changed since, deletions as `is_deleted: true`.
//!   Commands are idempotent by `uuid`, `temp_id`s resolve within a request and are reported
//!   in `temp_id_mapping`, per-command errors are reported in `sync_status`.
//! - `POST /api/v1/tasks/quick`: parses `#project`, `@label`, `/section`, `p1`-`p4`,
//!   `// description` and a small date set (see `dates.rs`).
//! - `GET /api/v1/tasks/completed/by_completion_date`: cursor pagination, `{items, next_cursor?}`.
//! - `GET /api/v1/tasks/filter`: a minimal filter subset, `400` for the rest.
//! - Bearer token check on `/api/`.
//!
//! Control endpoints under `/__test/` seed and dump state, expose the request log, and
//! inject failures (offline, status codes with `retry_after`, errors after commit,
//! per-command errors).

// Handlers return `Result<_, Response>` for early exits; the size of `Response` does not
// matter for a test server.
#![allow(clippy::result_large_err)]

mod commands;
pub mod dates;
mod filter;
mod order;
pub mod seed;

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll};

use axum::body::{Body, Bytes};
use axum::extract::{Query, State as AxState};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, FixedOffset, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

pub const DEFAULT_TOKEN: &str = "test-token";

/// Resource types stored as id -> object maps.
pub const RESOURCES: &[&str] = &[
    "projects",
    "items",
    "sections",
    "labels",
    "filters",
    "notes",
    "reminders",
    "collaborators",
];

/// An object with the revision of its last change.
#[derive(Debug, Clone, Serialize)]
pub struct Obj {
    pub v: Value,
    pub rev: u64,
}

/// A failure to inject.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Failure {
    /// Turn the server "offline": API requests get their connection dropped until
    /// [`Failure::Online`].
    Offline,
    Online,
    /// Answer the next `count` API requests (optionally only for `path`) with `status`,
    /// adding `Retry-After` and `error_extra.retry_after` when given.
    Status {
        status: u16,
        #[serde(default)]
        retry_after: Option<u64>,
        #[serde(default = "one")]
        count: u32,
        #[serde(default)]
        path: Option<String>,
        /// Only requests whose decoded body (as logged, serialized to JSON) contains this.
        #[serde(default)]
        body_contains: Option<String>,
    },
    /// Process the next `/sync` request, then answer with `status` instead of the result
    /// (the client cannot tell whether its commands ran).
    ErrorAfterCommit {
        #[serde(default = "s503")]
        status: u16,
        #[serde(default = "one")]
        count: u32,
    },
    /// Fail matching commands with `error` (a `sync_status` error object). Matches on
    /// `command_type` and, if given, a substring of the command's JSON args.
    CommandError {
        command_type: String,
        #[serde(default)]
        args_contain: Option<String>,
        #[serde(default)]
        error: Option<Value>,
        #[serde(default = "one")]
        count: u32,
    },
}

/// Request body limit for POST requests (`docs/api/pages/request-limits.md`).
pub const MAX_BODY_BYTES: usize = 1024 * 1024;

fn one() -> u32 {
    1
}
fn s503() -> u16 {
    503
}

#[derive(Debug, Clone, Serialize)]
pub struct LoggedRequest {
    pub method: String,
    pub path: String,
    pub query: String,
    /// Decoded form fields (`commands` and `resource_types` parsed as JSON) or JSON body.
    pub body: Value,
    pub status: u16,
    /// Arrival time in ms since the server state was created (monotonic clock).
    pub at_ms: u64,
}

#[derive(Debug, Clone)]
struct Processed {
    status: Value,
    temp: Option<(String, String)>,
}

/// The whole fake account.
#[derive(Debug)]
pub struct State {
    pub token: String,
    /// Fixed "now" (from `TICK_NOW` or set by tests); `None` = system clock in UTC.
    pub now: Option<DateTime<FixedOffset>>,
    pub(crate) rev: u64,
    pub(crate) next_id: u64,
    pub(crate) user: Value,
    pub(crate) user_rev: u64,
    pub(crate) user_settings: Value,
    pub(crate) res: BTreeMap<&'static str, BTreeMap<String, Obj>>,
    pub(crate) collaborator_states: Vec<Obj>,
    /// Completion records (task snapshots with `completed_at`), oldest first.
    pub(crate) completed: Vec<Value>,
    pub(crate) completed_rev: u64,
    processed: HashMap<String, Processed>,
    pub offline: bool,
    failures: VecDeque<Failure>,
    pub log: Vec<LoggedRequest>,
    /// Time origin of [`LoggedRequest::at_ms`].
    epoch: std::time::Instant,
}

impl State {
    pub fn new(token: &str) -> State {
        let now = std::env::var("TICK_NOW")
            .ok()
            .and_then(|s| DateTime::parse_from_rfc3339(s.trim()).ok());
        let mut s = State {
            token: token.to_owned(),
            now,
            rev: 0,
            next_id: 1000,
            user: Value::Null,
            user_rev: 0,
            user_settings: Value::Null,
            res: RESOURCES.iter().map(|r| (*r, BTreeMap::new())).collect(),
            collaborator_states: Vec::new(),
            completed: Vec::new(),
            completed_rev: 0,
            processed: HashMap::new(),
            offline: false,
            failures: VecDeque::new(),
            log: Vec::new(),
            epoch: std::time::Instant::now(),
        };
        seed::empty(&mut s);
        s
    }

    pub fn now(&self) -> DateTime<FixedOffset> {
        self.now
            .unwrap_or_else(|| Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap()))
    }

    pub(crate) fn now_utc_string(&self) -> String {
        self.now()
            .with_timezone(&Utc)
            .format("%Y-%m-%dT%H:%M:%S%.6fZ")
            .to_string()
    }

    pub(crate) fn bump(&mut self) -> u64 {
        self.rev += 1;
        self.rev
    }

    /// A fresh object ID in the style of real IDs (16 base-32-ish characters).
    pub fn new_id(&mut self) -> String {
        self.next_id += 1;
        const ALPHA: &[u8] = b"23456789cfghjmpqrvwxCFGHJMPQRVWX";
        let mut n = self.next_id;
        let mut s = Vec::new();
        while n > 0 {
            s.push(ALPHA[(n % 32) as usize]);
            n /= 32;
        }
        while s.len() < 10 {
            s.push(b'2');
        }
        s.reverse();
        format!("6X{}", String::from_utf8(s).unwrap())
    }

    pub fn user_id(&self) -> String {
        self.user["id"].as_str().unwrap_or("").to_owned()
    }

    pub fn inbox_id(&self) -> String {
        self.user["inbox_project_id"]
            .as_str()
            .unwrap_or("")
            .to_owned()
    }

    pub(crate) fn map(&self, r: &str) -> &BTreeMap<String, Obj> {
        &self.res[r]
    }

    pub(crate) fn map_mut(&mut self, r: &str) -> &mut BTreeMap<String, Obj> {
        self.res.get_mut(r).expect("resource")
    }

    /// A live (not deleted) object.
    pub fn get(&self, r: &str, id: &str) -> Option<&Value> {
        self.map(r)
            .get(id)
            .map(|o| &o.v)
            .filter(|v| !v["is_deleted"].as_bool().unwrap_or(false))
    }

    /// Insert or replace an object, bumping its revision.
    pub fn put(&mut self, r: &str, v: Value) {
        let id = v["id"].as_str().expect("object id").to_owned();
        let rev = self.bump();
        self.map_mut(r).insert(id, Obj { v, rev });
    }

    /// Change fields of an object, bumping its revision.
    pub fn update(&mut self, r: &str, id: &str, f: impl FnOnce(&mut Value)) {
        let rev = self.bump();
        if let Some(o) = self.map_mut(r).get_mut(id) {
            f(&mut o.v);
            o.rev = rev;
            if r == "items" || r == "projects" || r == "sections" {
                o.v["updated_at"] = json!(Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string());
            }
        }
    }

    pub(crate) fn mark_deleted(&mut self, r: &str, id: &str) {
        self.update(r, id, |v| v["is_deleted"] = json!(true));
    }

    /// Live objects of a resource.
    pub fn live(&self, r: &str) -> impl Iterator<Item = &Value> {
        self.map(r)
            .values()
            .map(|o| &o.v)
            .filter(|v| !v["is_deleted"].as_bool().unwrap_or(false))
    }

    pub fn set_user(&mut self, user: Value) {
        self.user = user;
        self.user_rev = self.bump();
    }

    pub fn set_user_settings(&mut self, settings: Value) {
        self.user_settings = settings;
        self.user_rev = self.bump();
    }

    pub fn add_collaborator_state(&mut self, project_id: &str, user_id: &str, state: &str) {
        let rev = self.bump();
        self.collaborator_states.push(Obj {
            v: json!({"project_id": project_id, "user_id": user_id, "state": state,
                      "is_deleted": false}),
            rev,
        });
    }

    pub fn add_completed(&mut self, record: Value) {
        self.completed.push(record);
        self.completed_rev = self.bump();
    }

    /// Drop all data (keeps the token and clock).
    pub fn reset(&mut self) {
        let token = self.token.clone();
        let now = self.now;
        *self = State::new(&token);
        self.now = now;
        seed::empty(self);
    }

    /// The whole state as JSON (for `/__test/state` and assertions).
    pub fn dump(&self) -> Value {
        let mut out = Map::new();
        out.insert("rev".into(), json!(self.rev));
        out.insert("user".into(), self.user.clone());
        out.insert("user_settings".into(), self.user_settings.clone());
        for r in RESOURCES {
            out.insert(
                (*r).into(),
                Value::Array(self.map(r).values().map(|o| o.v.clone()).collect()),
            );
        }
        out.insert(
            "collaborator_states".into(),
            Value::Array(
                self.collaborator_states
                    .iter()
                    .map(|o| o.v.clone())
                    .collect(),
            ),
        );
        out.insert("completed".into(), Value::Array(self.completed.clone()));
        Value::Object(out)
    }

    /// Live, active tasks (not completed, not deleted).
    pub fn active_items(&self) -> Vec<&Value> {
        self.live("items")
            .filter(|v| !v["checked"].as_bool().unwrap_or(false))
            .collect()
    }

    /// Find a live object by a field value.
    pub fn find(&self, r: &str, field: &str, value: &str) -> Option<&Value> {
        self.live(r).find(|v| v[field].as_str() == Some(value))
    }

    pub fn inject(&mut self, f: Failure) {
        match f {
            Failure::Offline => self.offline = true,
            Failure::Online => self.offline = false,
            other => self.failures.push_back(other),
        }
    }

    pub fn clear_failures(&mut self) {
        self.failures.clear();
        self.offline = false;
    }

    fn take_status_failure(&mut self, path: &str, body: &str) -> Option<(u16, Option<u64>)> {
        let idx = self.failures.iter().position(|f| match f {
            Failure::Status {
                path: p,
                body_contains,
                ..
            } => {
                p.as_deref().is_none_or(|p| p == path)
                    && body_contains.as_deref().is_none_or(|n| body.contains(n))
            }
            _ => false,
        })?;
        let Failure::Status {
            status,
            retry_after,
            count,
            ..
        } = self.failures[idx].clone()
        else {
            return None;
        };
        if count <= 1 {
            self.failures.remove(idx);
        } else if let Failure::Status { count: c, .. } = &mut self.failures[idx] {
            *c -= 1;
        }
        Some((status, retry_after))
    }

    fn take_after_commit(&mut self) -> Option<u16> {
        let idx = self
            .failures
            .iter()
            .position(|f| matches!(f, Failure::ErrorAfterCommit { .. }))?;
        let Failure::ErrorAfterCommit { status, count } = self.failures[idx].clone() else {
            return None;
        };
        if count <= 1 {
            self.failures.remove(idx);
        } else if let Failure::ErrorAfterCommit { count: c, .. } = &mut self.failures[idx] {
            *c -= 1;
        }
        Some(status)
    }

    pub(crate) fn take_command_error(&mut self, kind: &str, args: &Value) -> Option<Value> {
        let args_s = args.to_string();
        let idx = self.failures.iter().position(|f| match f {
            Failure::CommandError {
                command_type,
                args_contain,
                ..
            } => {
                command_type == kind
                    && args_contain
                        .as_deref()
                        .is_none_or(|needle| args_s.contains(needle))
            }
            _ => false,
        })?;
        let Failure::CommandError { error, count, .. } = self.failures[idx].clone() else {
            return None;
        };
        if count <= 1 {
            self.failures.remove(idx);
        } else if let Failure::CommandError { count: c, .. } = &mut self.failures[idx] {
            *c -= 1;
        }
        Some(error.unwrap_or_else(|| {
            commands::cmd_error("INVALID_ARGUMENT_VALUE", 20, "Invalid argument value", 400)
        }))
    }
}

pub type Shared = Arc<Mutex<State>>;

fn lock(s: &Shared) -> MutexGuard<'_, State> {
    s.lock().unwrap_or_else(|e| e.into_inner())
}

/// Docs-shaped error body (`docs/api/pages/authorization.md`).
fn error_response(
    status: u16,
    tag: &str,
    code: i64,
    msg: &str,
    retry_after: Option<u64>,
) -> Response {
    let mut extra = Map::new();
    if let Some(r) = retry_after {
        extra.insert("retry_after".into(), json!(r));
    }
    let body = json!({
        "error_tag": tag, "error_code": code, "error": msg, "http_code": status,
        "error_extra": extra,
    });
    let mut resp = (
        StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        Json(body),
    )
        .into_response();
    if let Some(r) = retry_after {
        resp.headers_mut()
            .insert(header::RETRY_AFTER, HeaderValue::from(r));
    }
    if status == 401 {
        resp.headers_mut().insert(
            header::WWW_AUTHENTICATE,
            HeaderValue::from_static("Bearer error=\"invalid_token\""),
        );
    }
    resp
}

fn status_error(status: u16, retry_after: Option<u64>) -> Response {
    let (tag, code, msg) = match status {
        401 => ("UNAUTHORIZED", 477, "Unauthorized"),
        403 => ("FORBIDDEN", 403, "Forbidden"),
        429 => ("TOO_MANY_REQUESTS", 429, "Too many requests"),
        503 => ("SERVICE_UNAVAILABLE", 503, "Service unavailable"),
        _ if status >= 500 => ("INTERNAL_SERVER_ERROR", 500, "Internal server error"),
        _ => ("BAD_REQUEST", 400, "Bad request"),
    };
    error_response(status, tag, code, msg, retry_after)
}

/// docs/api/pages/request-limits.md: "There is a 1MiB HTTP request body limit on POST
/// requests." The status and error body for a larger request are not documented; `413` with
/// the usual error shape is our choice.
fn too_large() -> Response {
    error_response(
        413,
        "REQUEST_TOO_LARGE",
        413,
        "Request body exceeds the 1 MiB limit",
        None,
    )
}

pub(crate) fn bad_request(msg: &str) -> Response {
    error_response(400, "INVALID_ARGUMENT_VALUE", 20, msg, None)
}

/// A body that fails on its first poll: hyper aborts the connection, so the client sees a
/// network error (the "offline" simulation; control endpoints keep working).
struct BrokenBody;

impl axum::body::HttpBody for BrokenBody {
    type Data = Bytes;
    type Error = std::io::Error;
    fn poll_frame(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Bytes>, Self::Error>>> {
        Poll::Ready(Some(Err(std::io::Error::new(
            std::io::ErrorKind::ConnectionReset,
            "fake server is offline",
        ))))
    }
}

fn dropped_connection() -> Response {
    let mut resp = Response::new(Body::new(BrokenBody));
    resp.headers_mut()
        .insert(header::CONNECTION, HeaderValue::from_static("close"));
    resp
}

enum Gate {
    Pass,
    Reject(Response),
}

/// Common checks for `/api/` requests: log, offline, injected status, bearer token.
fn gate(st: &mut State, method: &Method, uri: &Uri, headers: &HeaderMap, body: Value) -> Gate {
    let body_text = body.to_string();
    let at_ms = st.epoch.elapsed().as_millis() as u64;
    st.log.push(LoggedRequest {
        at_ms,
        method: method.to_string(),
        path: uri.path().to_owned(),
        query: uri.query().unwrap_or("").to_owned(),
        body,
        status: 0,
    });
    let set_status = |st: &mut State, s: u16| {
        if let Some(l) = st.log.last_mut() {
            l.status = s;
        }
    };
    if st.offline {
        return Gate::Reject(dropped_connection());
    }
    if let Some((status, retry_after)) = st.take_status_failure(uri.path(), &body_text) {
        set_status(st, status);
        return Gate::Reject(status_error(status, retry_after));
    }
    let auth = headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    if auth != format!("Bearer {}", st.token) {
        set_status(st, 401);
        return Gate::Reject(status_error(401, None));
    }
    Gate::Pass
}

fn finish(st: &mut State, resp: Response) -> Response {
    if let Some(l) = st.log.last_mut() {
        l.status = resp.status().as_u16();
    }
    resp
}

fn decode_form(body: &Bytes) -> HashMap<String, String> {
    form_urlencoded_parse(body)
}

/// Minimal `application/x-www-form-urlencoded` decoder.
fn form_urlencoded_parse(body: &[u8]) -> HashMap<String, String> {
    fn decode(s: &str) -> String {
        let bytes = s.as_bytes();
        let mut out = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b'+' => out.push(b' '),
                b'%' if i + 2 < bytes.len() => match u8::from_str_radix(&s[i + 1..i + 3], 16) {
                    Ok(b) => {
                        out.push(b);
                        i += 2;
                    }
                    Err(_) => out.push(b'%'),
                },
                b => out.push(b),
            }
            i += 1;
        }
        String::from_utf8_lossy(&out).into_owned()
    }
    let s = String::from_utf8_lossy(body);
    s.split('&')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let (k, v) = p.split_once('=').unwrap_or((p, ""));
            (decode(k), decode(v))
        })
        .collect()
}

async fn sync_handler(
    AxState(state): AxState<Shared>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let form = decode_form(&body);
    let mut logged = Map::new();
    for (k, v) in &form {
        let parsed = if k == "commands" || k == "resource_types" {
            serde_json::from_str(v).unwrap_or(Value::String(v.clone()))
        } else {
            Value::String(v.clone())
        };
        logged.insert(k.clone(), parsed);
    }
    let mut st = lock(&state);
    if let Gate::Reject(r) = gate(&mut st, &method, &uri, &headers, Value::Object(logged)) {
        return r;
    }
    if body.len() > MAX_BODY_BYTES {
        return finish(&mut st, too_large());
    }
    let resp = match run_sync(&mut st, &form) {
        Ok(v) => {
            if let Some(status) = st.take_after_commit() {
                status_error(status, None)
            } else {
                Json(v).into_response()
            }
        }
        Err(r) => r,
    };
    finish(&mut st, resp)
}

fn parse_resource_types(s: Option<&String>) -> Result<Vec<String>, Response> {
    let Some(s) = s else { return Ok(Vec::new()) };
    let list: Vec<String> =
        serde_json::from_str(s).map_err(|_| bad_request("resource_types must be a JSON array"))?;
    let mut all: Vec<String> = Vec::new();
    let known = [
        "user",
        "user_settings",
        "projects",
        "items",
        "sections",
        "labels",
        "filters",
        "notes",
        "reminders",
        "collaborators",
        "completed_info",
    ];
    for t in &list {
        if t == "all" {
            all.extend(known.iter().map(|s| s.to_string()));
        } else if !t.starts_with('-') {
            all.push(t.clone());
        }
    }
    for t in &list {
        if let Some(ex) = t.strip_prefix('-') {
            all.retain(|x| x != ex);
        }
    }
    all.dedup();
    Ok(all)
}

/// Opaque sync tokens: `fk<rev>.<salt>`. Unknown tokens trigger a full sync.
fn parse_token(t: &str) -> Option<u64> {
    t.strip_prefix("fk")?.split('.').next()?.parse().ok()
}

fn make_token(rev: u64) -> String {
    format!(
        "fk{rev}.{:x}",
        rev.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 40
    )
}

fn run_sync(st: &mut State, form: &HashMap<String, String>) -> Result<Value, Response> {
    let Some(token) = form.get("sync_token") else {
        return Err(bad_request("sync_token is required"));
    };
    let types = parse_resource_types(form.get("resource_types"))?;
    let mut out = Map::new();

    if let Some(cmds) = form.get("commands") {
        let cmds: Vec<Value> =
            serde_json::from_str(cmds).map_err(|_| bad_request("commands must be a JSON array"))?;
        if cmds.len() > 100 {
            return Err(error_response(
                400,
                "TOO_MANY_COMMANDS",
                36,
                "Too many commands in one request (maximum 100)",
                None,
            ));
        }
        let mut status = Map::new();
        let mut mapping = Map::new();
        let mut temp: HashMap<String, String> = HashMap::new();
        for c in cmds {
            let uuid = c["uuid"].as_str().unwrap_or("").to_owned();
            // Docs: "Todoist will not execute a command that has same UUID as a previously
            // executed command." What a repeated uuid reports is not documented; we replay
            // the stored result, including its temp_id mapping.
            if let Some(p) = st.processed.get(&uuid).cloned() {
                if let Some((t, r)) = &p.temp {
                    mapping.insert(t.clone(), json!(r));
                    temp.insert(t.clone(), r.clone());
                }
                status.insert(uuid, p.status);
                continue;
            }
            let result = commands::execute(st, &c, &mut temp);
            let (s, t) = match result {
                Ok(t) => (json!("ok"), t),
                Err(e) => (e, None),
            };
            if let Some((tid, real)) = &t {
                mapping.insert(tid.clone(), json!(real));
            }
            // Only executed commands are remembered; a failed command may be retried.
            if !uuid.is_empty() && s == json!("ok") {
                st.processed.insert(
                    uuid.clone(),
                    Processed {
                        status: s.clone(),
                        temp: t,
                    },
                );
            }
            status.insert(uuid, s);
        }
        out.insert("sync_status".into(), Value::Object(status));
        out.insert("temp_id_mapping".into(), Value::Object(mapping));
    }

    // Read semantics. Where the docs are silent, the choices are:
    // - A token we did not issue (or from before a reset) gets a full sync.
    // - Full sync ("all active resource data") leaves out deleted objects, completed tasks
    //   and archived projects and sections; the docs do not say whether archived objects are
    //   part of it.
    // - Incremental sync returns every object changed since the token, including deleted
    //   ones (`is_deleted: true`) and tasks completed since (`checked: true`).
    // - Collaborator states are returned under `collaborator_states` (the overview example);
    //   its property table calls them `collaborators_states`. They come with `collaborators`.
    // - `completed_info` is computed per project from the completion records.
    let since = parse_token(token).filter(|r| *r <= st.rev);
    let full = since.is_none();
    let since = since.unwrap_or(0);
    let want = |t: &str| types.iter().any(|x| x == t);
    if want("user") && (full || st.user_rev > since) {
        out.insert("user".into(), st.user.clone());
    }
    if want("user_settings") && (full || st.user_rev > since) {
        out.insert("user_settings".into(), st.user_settings.clone());
    }
    for r in RESOURCES {
        if !want(r) {
            continue;
        }
        let list: Vec<Value> = st
            .map(r)
            .values()
            .filter(|o| {
                if full {
                    let v = &o.v;
                    !v["is_deleted"].as_bool().unwrap_or(false)
                        && !(*r == "items" && v["checked"].as_bool().unwrap_or(false))
                        && !(*r == "projects" && v["is_archived"].as_bool().unwrap_or(false))
                        && !(*r == "sections" && v["is_archived"].as_bool().unwrap_or(false))
                } else {
                    o.rev > since
                }
            })
            .map(|o| o.v.clone())
            .collect();
        out.insert((*r).into(), Value::Array(list));
    }
    if want("collaborators") {
        let states: Vec<Value> = st
            .collaborator_states
            .iter()
            .filter(|o| {
                if full {
                    !o.v["is_deleted"].as_bool().unwrap_or(false)
                } else {
                    o.rev > since
                }
            })
            .map(|o| o.v.clone())
            .collect();
        out.insert("collaborator_states".into(), Value::Array(states));
    }
    if want("completed_info") && (full || st.completed_rev > since) {
        out.insert("completed_info".into(), completed_info(st));
    }
    out.insert("full_sync".into(), json!(full));
    if full {
        out.insert("full_sync_date_utc".into(), json!(st.now_utc_string()));
    }
    out.insert("sync_token".into(), json!(make_token(st.rev)));
    Ok(Value::Object(out))
}

fn completed_info(st: &State) -> Value {
    let mut per_project: BTreeMap<String, i64> = BTreeMap::new();
    for c in &st.completed {
        if let Some(p) = c["project_id"].as_str() {
            *per_project.entry(p.to_owned()).or_default() += 1;
        }
    }
    Value::Array(
        per_project
            .into_iter()
            .map(|(p, n)| json!({"project_id": p, "completed_items": n}))
            .collect(),
    )
}

#[derive(Deserialize)]
struct QuickBody {
    text: String,
    #[serde(default)]
    note: Option<String>,
}

async fn quick_handler(
    AxState(state): AxState<Shared>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let parsed: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    let mut st = lock(&state);
    if let Gate::Reject(r) = gate(&mut st, &method, &uri, &headers, parsed.clone()) {
        return r;
    }
    if body.len() > MAX_BODY_BYTES {
        return finish(&mut st, too_large());
    }
    let resp = match serde_json::from_value::<QuickBody>(parsed) {
        Ok(q) if !q.text.trim().is_empty() => {
            let item = dates::quick_add(&mut st, &q.text, q.note.as_deref());
            Json(item).into_response()
        }
        _ => bad_request("text is required"),
    };
    finish(&mut st, resp)
}

/// Completed range limit: the docs say "up to 3 months"; we allow 92 days.
const MAX_COMPLETED_DAYS: i64 = 92;

fn make_cursor(offset: usize) -> String {
    format!(
        "o{offset}.{:x}",
        (offset as u64).wrapping_mul(2654435761) & 0xffff_ffff
    )
}

fn parse_cursor(c: &str) -> Option<usize> {
    c.strip_prefix('o')?.split('.').next()?.parse().ok()
}

fn page_params(q: &HashMap<String, String>) -> Result<(usize, usize), Response> {
    let limit = match q.get("limit") {
        Some(l) => l
            .parse::<usize>()
            .ok()
            .filter(|l| *l > 0 && *l <= 200)
            .ok_or_else(|| bad_request("limit must be between 1 and 200"))?,
        None => 50,
    };
    let offset = match q.get("cursor").filter(|c| !c.is_empty()) {
        Some(c) => parse_cursor(c).ok_or_else(|| bad_request("invalid cursor"))?,
        None => 0,
    };
    Ok((limit, offset))
}

async fn completed_handler(
    AxState(state): AxState<Shared>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    Query(q): Query<HashMap<String, String>>,
) -> Response {
    let mut st = lock(&state);
    if let Gate::Reject(r) = gate(&mut st, &method, &uri, &headers, json!(q)) {
        return r;
    }
    let resp = (|| {
        let parse = |k: &str| {
            q.get(k)
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|d| d.with_timezone(&Utc))
                .ok_or_else(|| bad_request(&format!("{k} is required (RFC 3339)")))
        };
        let since = parse("since")?;
        let until = parse("until")?;
        if until < since || (until - since).num_days() > MAX_COMPLETED_DAYS {
            return Err(bad_request("the date range is limited to 3 months"));
        }
        let (limit, offset) = page_params(&q)?;
        let mut hits: Vec<&Value> = st
            .completed
            .iter()
            .filter(|c| {
                c["completed_at"]
                    .as_str()
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                    .map(|d| d.with_timezone(&Utc))
                    .is_some_and(|d| d >= since && d < until)
            })
            .filter(|c| {
                q.get("project_id")
                    .is_none_or(|p| c["project_id"].as_str() == Some(p))
            })
            .collect();
        hits.sort_by(|a, b| {
            b["completed_at"]
                .as_str()
                .cmp(&a["completed_at"].as_str())
                .then(a["id"].as_str().cmp(&b["id"].as_str()))
        });
        let page: Vec<Value> = hits
            .iter()
            .skip(offset)
            .take(limit)
            .map(|v| (*v).clone())
            .collect();
        let mut out = json!({ "items": page });
        if offset + limit < hits.len() {
            out["next_cursor"] = json!(make_cursor(offset + limit));
        }
        Ok(Json(out).into_response())
    })()
    .unwrap_or_else(|r| r);
    finish(&mut st, resp)
}

async fn filter_handler(
    AxState(state): AxState<Shared>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    Query(q): Query<HashMap<String, String>>,
) -> Response {
    let mut st = lock(&state);
    if let Gate::Reject(r) = gate(&mut st, &method, &uri, &headers, json!(q)) {
        return r;
    }
    let resp = (|| {
        let query = q
            .get("query")
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| bad_request("query is required"))?;
        let (limit, offset) = page_params(&q)?;
        let hits = filter::run(&st, query).map_err(|e| bad_request(&e))?;
        let page: Vec<Value> = hits.iter().skip(offset).take(limit).cloned().collect();
        let next = if offset + limit < hits.len() {
            json!(make_cursor(offset + limit))
        } else {
            Value::Null
        };
        Ok::<_, Response>(Json(json!({"results": page, "next_cursor": next})).into_response())
    })()
    .unwrap_or_else(|r| r);
    finish(&mut st, resp)
}

// ---- control endpoints ----

#[derive(Deserialize)]
struct SeedRequest {
    #[serde(default)]
    preset: Option<String>,
    #[serde(default)]
    count: Option<usize>,
    #[serde(default)]
    fixture: Option<Value>,
}

async fn seed_handler(AxState(state): AxState<Shared>, Json(req): Json<Value>) -> Response {
    let mut st = lock(&state);
    // Either {"preset": "demo"|"empty"|"bulk", "count": N} or a fixture object with
    // resource arrays (`projects`, `items`, ...), optionally wrapped as {"fixture": {...}}.
    let parsed: Option<SeedRequest> = serde_json::from_value(req.clone()).ok();
    let result = match parsed {
        Some(SeedRequest {
            preset: Some(p),
            count,
            ..
        }) => seed::apply_preset(&mut st, &p, count.unwrap_or(1000)),
        Some(SeedRequest {
            fixture: Some(f), ..
        }) => seed::apply_fixture(&mut st, &f),
        _ => seed::apply_fixture(&mut st, &req),
    };
    match result {
        Ok(()) => Json(json!({"ok": true, "rev": st.rev})).into_response(),
        Err(e) => bad_request(&e),
    }
}

async fn state_handler(AxState(state): AxState<Shared>) -> Response {
    Json(lock(&state).dump()).into_response()
}

async fn requests_handler(AxState(state): AxState<Shared>) -> Response {
    Json(json!(lock(&state).log)).into_response()
}

async fn clear_requests_handler(AxState(state): AxState<Shared>) -> Response {
    lock(&state).log.clear();
    Json(json!({"ok": true})).into_response()
}

async fn fail_handler(AxState(state): AxState<Shared>, Json(f): Json<Value>) -> Response {
    let list: Vec<Value> = match f {
        Value::Array(a) => a,
        other => vec![other],
    };
    let mut st = lock(&state);
    for f in list {
        match serde_json::from_value::<Failure>(f) {
            Ok(f) => st.inject(f),
            Err(e) => return bad_request(&format!("invalid failure: {e}")),
        }
    }
    Json(json!({"ok": true})).into_response()
}

async fn clear_fail_handler(AxState(state): AxState<Shared>) -> Response {
    lock(&state).clear_failures();
    Json(json!({"ok": true})).into_response()
}

async fn reset_handler(AxState(state): AxState<Shared>) -> Response {
    lock(&state).reset();
    Json(json!({"ok": true})).into_response()
}

#[derive(Deserialize)]
struct NowRequest {
    now: Option<String>,
}

async fn now_handler(AxState(state): AxState<Shared>, Json(req): Json<NowRequest>) -> Response {
    let mut st = lock(&state);
    match req.now.as_deref().map(DateTime::parse_from_rfc3339) {
        Some(Ok(d)) => st.now = Some(d),
        Some(Err(_)) => return bad_request("now must be RFC 3339"),
        None => st.now = None,
    }
    Json(json!({"ok": true})).into_response()
}

/// The axum router over shared state.
pub fn router(state: Shared) -> Router {
    Router::new()
        .route("/api/v1/sync", post(sync_handler))
        .route("/api/v1/tasks/quick", post(quick_handler))
        .route(
            "/api/v1/tasks/completed/by_completion_date",
            get(completed_handler),
        )
        .route("/api/v1/tasks/filter", get(filter_handler))
        .route("/__test/seed", post(seed_handler))
        .route("/__test/state", get(state_handler))
        .route(
            "/__test/requests",
            get(requests_handler).delete(clear_requests_handler),
        )
        .route(
            "/__test/fail",
            post(fail_handler).delete(clear_fail_handler),
        )
        .route("/__test/reset", post(reset_handler))
        .route("/__test/now", post(now_handler))
        // Accept larger bodies so the handlers can answer oversized requests the documented
        // way (and log them) instead of axum rejecting them first.
        .layer(axum::extract::DefaultBodyLimit::max(16 * MAX_BODY_BYTES))
        .with_state(state)
}

/// A running fake server.
pub struct FakeServer {
    addr: SocketAddr,
    state: Shared,
    task: tokio::task::JoinHandle<()>,
}

impl FakeServer {
    /// Start on `127.0.0.1:port` (`0` = any free port) with an empty account and the default
    /// token. Must be called inside a tokio runtime.
    pub async fn start(port: u16) -> std::io::Result<FakeServer> {
        FakeServer::start_with(port, State::new(DEFAULT_TOKEN)).await
    }

    pub async fn start_with(port: u16, state: State) -> std::io::Result<FakeServer> {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
        let addr = listener.local_addr()?;
        let state: Shared = Arc::new(Mutex::new(state));
        let app = router(state.clone());
        let task = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        Ok(FakeServer { addr, state, task })
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Value for `TICK_API_BASE`.
    pub fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }

    pub fn token(&self) -> String {
        lock(&self.state).token.clone()
    }

    /// Direct access to the state (tests).
    pub fn state(&self) -> MutexGuard<'_, State> {
        lock(&self.state)
    }

    pub fn shared(&self) -> Shared {
        self.state.clone()
    }

    pub fn inject(&self, f: Failure) {
        self.state().inject(f);
    }

    pub fn set_offline(&self, offline: bool) {
        self.state().offline = offline;
    }

    pub fn requests(&self) -> Vec<LoggedRequest> {
        self.state().log.clone()
    }

    /// Requests to one path.
    pub fn requests_to(&self, path: &str) -> Vec<LoggedRequest> {
        self.requests()
            .into_iter()
            .filter(|r| r.path == path)
            .collect()
    }

    pub fn stop(self) {
        self.task.abort();
    }
}

impl Drop for FakeServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}
