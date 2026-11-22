//! HTTP client for the Todoist API v1.
//!
//! - `POST /api/v1/sync` is form-encoded (`docs/api/pages/sync.md`); `resource_types` and
//!   `commands` are JSON strings inside the form.
//! - `POST /api/v1/tasks/quick` takes a JSON body `{text, ...}` and returns the task.
//! - `GET /api/v1/tasks/completed/by_completion_date` needs `since` and `until`, returns
//!   `{items, next_cursor?}`.
//! - `GET /api/v1/tasks/filter` returns `{results, next_cursor}`; one query, no commas.

use std::time::Duration;

use serde_json::Value;

use crate::command::Command;
use crate::model::{CompletedPage, Page, SyncResponse};

pub const DEFAULT_API_BASE: &str = "https://api.todoist.com";

/// Resource types Tick reads. `collaborator_states` arrive with `collaborators`.
pub const RESOURCE_TYPES: &[&str] = &[
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

/// API base URL from `TICK_API_BASE`, default `https://api.todoist.com`.
pub fn api_base_from_env() -> String {
    std::env::var("TICK_API_BASE")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_API_BASE.to_owned())
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ApiError {
    /// 401: the token is invalid or revoked. Never retried.
    #[error("unauthorized: the API token was rejected")]
    Unauthorized,
    /// 403.
    #[error("forbidden: {0}")]
    Forbidden(String),
    /// 429. `retry_after` from the `Retry-After` header or `error_extra.retry_after`.
    #[error("rate limited (retry after {retry_after:?} s)")]
    RateLimited { retry_after: Option<u64> },
    /// 5xx.
    #[error("server error {status}")]
    Server {
        status: u16,
        retry_after: Option<u64>,
    },
    /// Other 4xx (400, 404, ...).
    #[error("request failed with {status}: {message}")]
    Request { status: u16, message: String },
    /// No response: DNS, connect, reset, timeout.
    #[error("network error: {0}")]
    Network(String),
    /// The response body was not what the docs describe.
    #[error("unexpected response: {0}")]
    Decode(String),
}

impl ApiError {
    /// Whether waiting and retrying the same request can succeed.
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            ApiError::RateLimited { .. } | ApiError::Server { .. } | ApiError::Network(_)
        )
    }

    pub fn retry_after(&self) -> Option<u64> {
        match self {
            ApiError::RateLimited { retry_after } | ApiError::Server { retry_after, .. } => {
                *retry_after
            }
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    base: String,
    token: String,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("base", &self.base)
            .finish_non_exhaustive()
    }
}

impl Client {
    pub fn new(base: impl Into<String>, token: impl Into<String>) -> Client {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .user_agent(concat!("Tick/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("HTTP client");
        Client {
            http,
            base: base.into().trim_end_matches('/').to_owned(),
            token: token.into(),
        }
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    /// One `/sync` round trip: read resources since `sync_token` (`*` for a full sync) and
    /// push `commands` (at most 100, the documented limit).
    pub async fn sync(
        &self,
        sync_token: &str,
        resource_types: &[&str],
        commands: &[Command],
    ) -> Result<SyncResponse, ApiError> {
        let mut form: Vec<(&str, String)> = vec![
            ("sync_token", sync_token.to_owned()),
            (
                "resource_types",
                serde_json::to_string(resource_types).expect("json"),
            ),
        ];
        if !commands.is_empty() {
            form.push(("commands", serde_json::to_string(commands).expect("json")));
        }
        let req = self
            .http
            .post(self.url("/api/v1/sync"))
            .bearer_auth(&self.token)
            .form(&form);
        let body = send(req).await?;
        serde_json::from_value(body).map_err(|e| ApiError::Decode(e.to_string()))
    }

    /// `POST /api/v1/tasks/quick`; returns the created task object.
    pub async fn quick_add(&self, text: &str) -> Result<Value, ApiError> {
        let req = self
            .http
            .post(self.url("/api/v1/tasks/quick"))
            .bearer_auth(&self.token)
            .json(&serde_json::json!({ "text": text }));
        send(req).await
    }

    /// One page of completed tasks between `since` (inclusive) and `until` (exclusive), both
    /// RFC 3339 UTC. The docs limit the range to about three months.
    pub async fn completed_by_completion_date(
        &self,
        since: &str,
        until: &str,
        cursor: Option<&str>,
        limit: u32,
    ) -> Result<CompletedPage, ApiError> {
        let mut q: Vec<(&str, String)> = vec![
            ("since", since.to_owned()),
            ("until", until.to_owned()),
            ("limit", limit.min(200).to_string()),
        ];
        if let Some(c) = cursor {
            q.push(("cursor", c.to_owned()));
        }
        let req = self
            .http
            .get(self.url("/api/v1/tasks/completed/by_completion_date"))
            .bearer_auth(&self.token)
            .query(&q);
        let body = send(req).await?;
        serde_json::from_value(body).map_err(|e| ApiError::Decode(e.to_string()))
    }

    /// One page of `GET /api/v1/tasks/filter` for a single (comma-free) query.
    pub async fn tasks_filter(
        &self,
        query: &str,
        cursor: Option<&str>,
        limit: u32,
    ) -> Result<Page, ApiError> {
        let mut q: Vec<(&str, String)> = vec![
            ("query", query.to_owned()),
            ("limit", limit.min(200).to_string()),
        ];
        if let Some(c) = cursor {
            q.push(("cursor", c.to_owned()));
        }
        let req = self
            .http
            .get(self.url("/api/v1/tasks/filter"))
            .bearer_auth(&self.token)
            .query(&q);
        let body = send(req).await?;
        serde_json::from_value(body).map_err(|e| ApiError::Decode(e.to_string()))
    }

    /// All pages of `GET /api/v1/tasks/filter`.
    pub async fn tasks_filter_all(&self, query: &str) -> Result<Vec<Value>, ApiError> {
        let mut out = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let page = self.tasks_filter(query, cursor.as_deref(), 200).await?;
            out.extend(page.results);
            match page.next_cursor {
                Some(c) if !c.is_empty() => cursor = Some(c),
                _ => break,
            }
        }
        Ok(out)
    }
}

fn retry_after_header(resp: &reqwest::Response) -> Option<u64> {
    resp.headers()
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse()
        .ok()
}

async fn send(req: reqwest::RequestBuilder) -> Result<Value, ApiError> {
    let resp = req
        .send()
        .await
        .map_err(|e| ApiError::Network(error_chain(&e)))?;
    let status = resp.status();
    let header_retry = retry_after_header(&resp);
    let text = resp
        .text()
        .await
        .map_err(|e| ApiError::Network(error_chain(&e)))?;
    if status.is_success() {
        return serde_json::from_str(&text).map_err(|e| ApiError::Decode(e.to_string()));
    }
    let body: Option<Value> = serde_json::from_str(&text).ok();
    let body_retry = body
        .as_ref()
        .and_then(|b| b.get("error_extra"))
        .and_then(|e| e.get("retry_after"))
        .and_then(Value::as_u64);
    let message = body
        .as_ref()
        .and_then(|b| b.get("error"))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| text.chars().take(200).collect());
    let retry_after = header_retry.or(body_retry);
    Err(match status.as_u16() {
        401 => ApiError::Unauthorized,
        403 => ApiError::Forbidden(message),
        429 => ApiError::RateLimited { retry_after },
        s @ 500..=599 => ApiError::Server {
            status: s,
            retry_after,
        },
        s => ApiError::Request { status: s, message },
    })
}

fn error_chain(e: &dyn std::error::Error) -> String {
    let mut s = e.to_string();
    let mut cur = e.source();
    while let Some(c) = cur {
        s.push_str(": ");
        s.push_str(&c.to_string());
        cur = c.source();
    }
    s
}
