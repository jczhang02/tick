//! Todoist API v1 resource types.
//!
//! Field names follow `docs/api/pages/sync_*.md`. Every type is tolerant: missing fields take
//! defaults, IDs accept strings or numbers, and unknown fields are kept in `extra` so that
//! serializing a value back yields the full object the server sent.

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};

/// Deserialize an ID that may be a JSON string or a number (the docs show `workspace_id` as
/// an integer in an example and as a string in the property table).
pub fn de_id<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(de_opt_id(d)?.unwrap_or_default())
}

/// Optional variant of [`de_id`]; empty strings become `None`.
pub fn de_opt_id<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    let v = Option::<Value>::deserialize(d)?;
    Ok(match v {
        Some(Value::String(s)) if !s.is_empty() => Some(s),
        Some(Value::Number(n)) => Some(n.to_string()),
        _ => None,
    })
}

/// Booleans sometimes arrive as `0`/`1` in older payloads (for example `is_forward`).
fn de_bool<'de, D: Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    let v = Option::<Value>::deserialize(d)?;
    Ok(match v {
        Some(Value::Bool(b)) => b,
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0) != 0,
        _ => false,
    })
}

fn de_i64<'de, D: Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
    let v = Option::<Value>::deserialize(d)?;
    Ok(match v {
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0),
        Some(Value::String(s)) => s.parse().unwrap_or(0),
        _ => 0,
    })
}

fn de_opt_string<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    let v = Option::<Value>::deserialize(d)?;
    Ok(match v {
        Some(Value::String(s)) => Some(s),
        Some(Value::Number(n)) => Some(n.to_string()),
        _ => None,
    })
}

fn de_string<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(de_opt_string(d)?.unwrap_or_default())
}

fn de_strings<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    let v = Option::<Value>::deserialize(d)?;
    Ok(match v {
        Some(Value::Array(a)) => a
            .into_iter()
            .filter_map(|x| match x {
                Value::String(s) => Some(s),
                Value::Number(n) => Some(n.to_string()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    })
}

fn default_priority() -> i64 {
    1
}

/// A due date (`docs/api/pages/due-dates.md`).
///
/// `date` is one of `YYYY-MM-DD` (full day), `YYYY-MM-DDTHH:MM:SS[.ffffff]` (floating) or
/// `YYYY-MM-DDTHH:MM:SS[.ffffff]Z` (fixed time zone, stored in UTC). It is `None` only for an
/// optimistic local value created from a free-text `string` that the server has not parsed yet.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Due {
    #[serde(default, deserialize_with = "de_opt_string")]
    pub date: Option<String>,
    #[serde(default, deserialize_with = "de_opt_string")]
    pub timezone: Option<String>,
    #[serde(default, deserialize_with = "de_string")]
    pub string: String,
    #[serde(default, deserialize_with = "de_opt_string")]
    pub lang: Option<String>,
    #[serde(default, deserialize_with = "de_bool")]
    pub is_recurring: bool,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Due {
    /// True when the due date carries a time of day.
    pub fn has_time(&self) -> bool {
        self.date.as_deref().is_some_and(|d| d.contains('T'))
    }
}

/// A deadline (`docs/api/pages/deadlines.md`): date only, never recurring.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Deadline {
    #[serde(default, deserialize_with = "de_string")]
    pub date: String,
    #[serde(default, deserialize_with = "de_opt_string")]
    pub lang: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A task duration.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TaskDuration {
    #[serde(default, deserialize_with = "de_i64")]
    pub amount: i64,
    #[serde(default, deserialize_with = "de_string")]
    pub unit: String,
}

/// Task priority as the API stores it: `4` is the most urgent (shown as `p1` in clients).
pub type ApiPriority = i64;

/// Convert an API priority (4 = urgent) to the client label number (1 = urgent).
pub fn api_priority_to_p(api: ApiPriority) -> u8 {
    (5 - api.clamp(1, 4)) as u8
}

/// Convert a client label number (`p1` = 1) to the API value (4).
pub fn p_to_api_priority(p: u8) -> ApiPriority {
    5 - i64::from(p.clamp(1, 4))
}

/// A task (`items` in the Sync API, `docs/api/pages/sync_tasks.md`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Item {
    #[serde(deserialize_with = "de_id")]
    pub id: String,
    #[serde(default, deserialize_with = "de_opt_id")]
    pub user_id: Option<String>,
    #[serde(default, deserialize_with = "de_id")]
    pub project_id: String,
    #[serde(default, deserialize_with = "de_string")]
    pub content: String,
    #[serde(default, deserialize_with = "de_string")]
    pub description: String,
    #[serde(default = "default_priority", deserialize_with = "de_i64")]
    pub priority: ApiPriority,
    #[serde(default)]
    pub due: Option<Due>,
    #[serde(default)]
    pub deadline: Option<Deadline>,
    #[serde(default, deserialize_with = "de_opt_id")]
    pub parent_id: Option<String>,
    #[serde(default, deserialize_with = "de_i64")]
    pub child_order: i64,
    #[serde(default, deserialize_with = "de_opt_string")]
    pub order_key: Option<String>,
    #[serde(default, deserialize_with = "de_opt_id")]
    pub section_id: Option<String>,
    #[serde(default, deserialize_with = "de_i64")]
    pub day_order: i64,
    #[serde(default, deserialize_with = "de_bool")]
    pub is_collapsed: bool,
    #[serde(default, deserialize_with = "de_strings")]
    pub labels: Vec<String>,
    #[serde(default, deserialize_with = "de_opt_id")]
    pub added_by_uid: Option<String>,
    #[serde(default, deserialize_with = "de_opt_id")]
    pub assigned_by_uid: Option<String>,
    #[serde(default, deserialize_with = "de_opt_id")]
    pub responsible_uid: Option<String>,
    #[serde(default, deserialize_with = "de_bool")]
    pub checked: bool,
    #[serde(default, deserialize_with = "de_bool")]
    pub is_deleted: bool,
    #[serde(default, deserialize_with = "de_opt_string")]
    pub added_at: Option<String>,
    #[serde(default, deserialize_with = "de_opt_string")]
    pub updated_at: Option<String>,
    #[serde(default, deserialize_with = "de_opt_string")]
    pub completed_at: Option<String>,
    #[serde(default)]
    pub duration: Option<TaskDuration>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Item {
    /// Client-side priority number: `1` (urgent) ... `4` (none).
    pub fn p(&self) -> u8 {
        api_priority_to_p(self.priority)
    }
}

/// A project (`docs/api/pages/sync_projects.md`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Project {
    #[serde(deserialize_with = "de_id")]
    pub id: String,
    #[serde(default, deserialize_with = "de_string")]
    pub name: String,
    #[serde(default, deserialize_with = "de_string")]
    pub description: String,
    #[serde(default, deserialize_with = "de_string")]
    pub color: String,
    #[serde(default, deserialize_with = "de_opt_string")]
    pub icon: Option<String>,
    #[serde(default, deserialize_with = "de_opt_id")]
    pub parent_id: Option<String>,
    #[serde(default, deserialize_with = "de_i64")]
    pub child_order: i64,
    #[serde(default, deserialize_with = "de_opt_string")]
    pub order_key: Option<String>,
    #[serde(default, deserialize_with = "de_bool")]
    pub is_collapsed: bool,
    #[serde(default, deserialize_with = "de_bool")]
    pub shared: bool,
    #[serde(default, deserialize_with = "de_bool")]
    pub can_assign_tasks: bool,
    #[serde(default, deserialize_with = "de_bool")]
    pub is_deleted: bool,
    #[serde(default, deserialize_with = "de_bool")]
    pub is_archived: bool,
    #[serde(default, deserialize_with = "de_bool")]
    pub is_favorite: bool,
    #[serde(default, deserialize_with = "de_bool")]
    pub is_frozen: bool,
    #[serde(default, deserialize_with = "de_opt_string")]
    pub view_style: Option<String>,
    #[serde(default, deserialize_with = "de_opt_string")]
    pub role: Option<String>,
    /// Only sent (as `true`) for the Inbox project.
    #[serde(default, deserialize_with = "de_bool")]
    pub inbox_project: bool,
    #[serde(default, deserialize_with = "de_opt_id")]
    pub workspace_id: Option<String>,
    #[serde(default, deserialize_with = "de_opt_id")]
    pub folder_id: Option<String>,
    #[serde(default, deserialize_with = "de_opt_string")]
    pub created_at: Option<String>,
    #[serde(default, deserialize_with = "de_opt_string")]
    pub updated_at: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A section (`docs/api/pages/sync_sections.md`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Section {
    #[serde(deserialize_with = "de_id")]
    pub id: String,
    #[serde(default, deserialize_with = "de_string")]
    pub name: String,
    #[serde(default, deserialize_with = "de_opt_string")]
    pub description: Option<String>,
    #[serde(default, deserialize_with = "de_id")]
    pub project_id: String,
    #[serde(default, deserialize_with = "de_i64")]
    pub section_order: i64,
    #[serde(default, deserialize_with = "de_opt_string")]
    pub order_key: Option<String>,
    #[serde(default, deserialize_with = "de_bool")]
    pub is_collapsed: bool,
    #[serde(default, deserialize_with = "de_opt_id")]
    pub user_id: Option<String>,
    #[serde(default, deserialize_with = "de_bool")]
    pub is_deleted: bool,
    #[serde(default, deserialize_with = "de_bool")]
    pub is_archived: bool,
    #[serde(default, deserialize_with = "de_opt_string")]
    pub archived_at: Option<String>,
    #[serde(default, deserialize_with = "de_opt_string")]
    pub added_at: Option<String>,
    #[serde(default, deserialize_with = "de_opt_string")]
    pub updated_at: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A personal label (`docs/api/pages/sync_labels.md`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Label {
    #[serde(deserialize_with = "de_id")]
    pub id: String,
    #[serde(default, deserialize_with = "de_string")]
    pub name: String,
    #[serde(default, deserialize_with = "de_string")]
    pub color: String,
    #[serde(default, deserialize_with = "de_i64")]
    pub item_order: i64,
    #[serde(default, deserialize_with = "de_opt_string")]
    pub order_key: Option<String>,
    #[serde(default, deserialize_with = "de_bool")]
    pub is_deleted: bool,
    #[serde(default, deserialize_with = "de_bool")]
    pub is_favorite: bool,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A saved filter (`docs/api/pages/sync_filters.md`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Filter {
    #[serde(deserialize_with = "de_id")]
    pub id: String,
    #[serde(default, deserialize_with = "de_string")]
    pub name: String,
    #[serde(default, deserialize_with = "de_string")]
    pub query: String,
    #[serde(default, deserialize_with = "de_string")]
    pub description: String,
    #[serde(default, deserialize_with = "de_string")]
    pub color: String,
    #[serde(default, deserialize_with = "de_i64")]
    pub item_order: i64,
    #[serde(default, deserialize_with = "de_opt_string")]
    pub order_key: Option<String>,
    #[serde(default, deserialize_with = "de_bool")]
    pub is_deleted: bool,
    #[serde(default, deserialize_with = "de_bool")]
    pub is_favorite: bool,
    #[serde(default, deserialize_with = "de_bool")]
    pub is_frozen: bool,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A file attached to a comment.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FileAttachment {
    #[serde(default, deserialize_with = "de_opt_string")]
    pub file_name: Option<String>,
    #[serde(default, deserialize_with = "de_opt_string")]
    pub file_type: Option<String>,
    #[serde(default, deserialize_with = "de_opt_string")]
    pub file_url: Option<String>,
    #[serde(default)]
    pub file_size: Option<i64>,
    #[serde(default, deserialize_with = "de_opt_string")]
    pub upload_state: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A task comment (`notes`, `docs/api/pages/sync_comments.md`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Note {
    #[serde(deserialize_with = "de_id")]
    pub id: String,
    #[serde(default, deserialize_with = "de_opt_id")]
    pub posted_uid: Option<String>,
    #[serde(default, deserialize_with = "de_id")]
    pub item_id: String,
    #[serde(default, deserialize_with = "de_string")]
    pub content: String,
    #[serde(default)]
    pub file_attachment: Option<FileAttachment>,
    #[serde(default)]
    pub uids_to_notify: Option<Vec<String>>,
    #[serde(default, deserialize_with = "de_bool")]
    pub is_deleted: bool,
    #[serde(default, deserialize_with = "de_opt_string")]
    pub posted_at: Option<String>,
    #[serde(default)]
    pub reactions: Option<Value>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A reminder (`docs/api/pages/sync_reminders.md`). Only time-based reminders matter to Tick,
/// but location reminders deserialize too.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Reminder {
    #[serde(deserialize_with = "de_id")]
    pub id: String,
    #[serde(default, deserialize_with = "de_opt_id")]
    pub notify_uid: Option<String>,
    #[serde(default, deserialize_with = "de_id")]
    pub item_id: String,
    /// `relative`, `absolute` or `location`.
    #[serde(default, rename = "type", deserialize_with = "de_string")]
    pub kind: String,
    #[serde(default)]
    pub due: Option<Due>,
    #[serde(default)]
    pub minute_offset: Option<i64>,
    #[serde(default, deserialize_with = "de_bool")]
    pub is_deleted: bool,
    #[serde(default, deserialize_with = "de_bool")]
    pub is_urgent: bool,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A collaborator (`docs/api/pages/sync_sharing.md`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Collaborator {
    #[serde(deserialize_with = "de_id")]
    pub id: String,
    #[serde(default, deserialize_with = "de_string")]
    pub email: String,
    #[serde(default, deserialize_with = "de_string")]
    pub full_name: String,
    #[serde(default, deserialize_with = "de_opt_string")]
    pub timezone: Option<String>,
    #[serde(default, deserialize_with = "de_opt_string")]
    pub image_id: Option<String>,
    #[serde(default, deserialize_with = "de_bool")]
    pub is_agent: bool,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A "user to shared project" mapping. It has no `id`: the key is `(project_id, user_id)`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CollaboratorState {
    #[serde(default, deserialize_with = "de_id")]
    pub project_id: String,
    #[serde(default, deserialize_with = "de_id")]
    pub user_id: String,
    /// `active` or `invited`.
    #[serde(default, deserialize_with = "de_string")]
    pub state: String,
    #[serde(default, deserialize_with = "de_bool")]
    pub is_deleted: bool,
    #[serde(default, deserialize_with = "de_opt_string")]
    pub role: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Completed counts for an active project, section or parent item (`completed_info`).
/// Exactly one of the three IDs is set.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CompletedInfo {
    #[serde(default, deserialize_with = "de_opt_id")]
    pub project_id: Option<String>,
    #[serde(default, deserialize_with = "de_opt_id")]
    pub section_id: Option<String>,
    #[serde(default, deserialize_with = "de_opt_id")]
    pub item_id: Option<String>,
    #[serde(default, deserialize_with = "de_i64")]
    pub completed_items: i64,
    #[serde(default)]
    pub archived_sections: Option<i64>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl CompletedInfo {
    /// `(kind, id)` key: kind is `project`, `section` or `item`.
    pub fn key(&self) -> Option<(&'static str, &str)> {
        if let Some(id) = &self.project_id {
            Some(("project", id))
        } else if let Some(id) = &self.section_id {
            Some(("section", id))
        } else {
            self.item_id.as_deref().map(|id| ("item", id))
        }
    }
}

/// Time zone information of the user.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TzInfo {
    #[serde(default, deserialize_with = "de_opt_string")]
    pub timezone: Option<String>,
    #[serde(default, deserialize_with = "de_opt_string")]
    pub gmt_string: Option<String>,
    #[serde(default, deserialize_with = "de_i64")]
    pub hours: i64,
    #[serde(default, deserialize_with = "de_i64")]
    pub minutes: i64,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// The user object (`docs/api/pages/sync_user.md`). The `token` field the server sends is
/// stripped before the object is cached (see [`User::sanitized_json`]).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct User {
    #[serde(default, deserialize_with = "de_id")]
    pub id: String,
    #[serde(default, deserialize_with = "de_string")]
    pub email: String,
    #[serde(default, deserialize_with = "de_string")]
    pub full_name: String,
    #[serde(default, deserialize_with = "de_opt_id")]
    pub inbox_project_id: Option<String>,
    #[serde(default, deserialize_with = "de_opt_string")]
    pub image_id: Option<String>,
    #[serde(default, deserialize_with = "de_opt_string")]
    pub lang: Option<String>,
    #[serde(default)]
    pub tz_info: Option<TzInfo>,
    #[serde(default, deserialize_with = "de_i64")]
    pub start_day: i64,
    #[serde(default, deserialize_with = "de_bool")]
    pub is_premium: bool,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl User {
    /// Remove secrets (the API token) from a raw user object before persisting it.
    pub fn sanitize(value: &mut Value) {
        if let Some(obj) = value.as_object_mut() {
            obj.remove("token");
        }
    }
}

/// User settings (`docs/api/pages/sync_user.md`, "User settings").
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UserSettings {
    #[serde(default, deserialize_with = "de_bool")]
    pub reminder_push: bool,
    #[serde(default, deserialize_with = "de_bool")]
    pub reminder_desktop: bool,
    #[serde(default, deserialize_with = "de_bool")]
    pub reminder_email: bool,
    #[serde(default, deserialize_with = "de_bool")]
    pub completed_sound_desktop: bool,
    #[serde(default, deserialize_with = "de_bool")]
    pub completed_sound_mobile: bool,
    #[serde(default, deserialize_with = "de_bool")]
    pub reset_recurring_subtasks: bool,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Error object for one command in `sync_status` (`docs/api/pages/sync_overview.md`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CommandError {
    #[serde(default, deserialize_with = "de_opt_string")]
    pub error_tag: Option<String>,
    #[serde(default)]
    pub error_code: Option<i64>,
    #[serde(default, deserialize_with = "de_string")]
    pub error: String,
    #[serde(default)]
    pub http_code: Option<i64>,
    #[serde(default)]
    pub error_extra: Option<Map<String, Value>>,
}

impl CommandError {
    /// The docs say `400` command errors are terminal and other statuses such as `403`/`409`
    /// may be terminal too. The docs' own example (`error_code` 15, "Invalid temporary id")
    /// carries no `http_code` at all. We therefore treat every error as terminal unless its
    /// `http_code` is `429` or `5xx`; otherwise a dropped `*_add` would wedge its dependents.
    pub fn is_retryable(&self) -> bool {
        matches!(self.http_code, Some(429) | Some(500..=599))
    }

    /// `error_extra.retry_after` in seconds, when present.
    pub fn retry_after(&self) -> Option<u64> {
        self.error_extra.as_ref()?.get("retry_after")?.as_u64()
    }

    /// One-line description for the UI.
    pub fn message(&self) -> String {
        match &self.error_tag {
            Some(tag) => format!("{} ({tag})", self.error),
            None => self.error.clone(),
        }
    }
}

/// The status of one command: `"ok"` or an error object.
#[derive(Debug, Clone, PartialEq)]
pub enum CommandStatus {
    Ok,
    Error(CommandError),
}

impl<'de> Deserialize<'de> for CommandStatus {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = Value::deserialize(d)?;
        match v {
            Value::String(s) if s == "ok" => Ok(CommandStatus::Ok),
            Value::Object(_) => serde_json::from_value(v)
                .map(CommandStatus::Error)
                .map_err(serde::de::Error::custom),
            other => Ok(CommandStatus::Error(CommandError {
                error: format!("unexpected command status {other}"),
                ..Default::default()
            })),
        }
    }
}

impl Serialize for CommandStatus {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            CommandStatus::Ok => s.serialize_str("ok"),
            CommandStatus::Error(e) => e.serialize(s),
        }
    }
}

/// A `/api/v1/sync` response. Resource arrays are kept as raw JSON values; the store parses
/// each into its typed struct and keeps the raw object next to the indexed columns.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SyncResponse {
    #[serde(default)]
    pub sync_token: Option<String>,
    #[serde(default)]
    pub full_sync: bool,
    #[serde(default)]
    pub full_sync_date_utc: Option<String>,
    #[serde(default)]
    pub sync_status: std::collections::HashMap<String, CommandStatus>,
    #[serde(default)]
    pub temp_id_mapping: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub user: Option<Value>,
    #[serde(default)]
    pub user_settings: Option<Value>,
    #[serde(default)]
    pub projects: Option<Vec<Value>>,
    #[serde(default)]
    pub items: Option<Vec<Value>>,
    #[serde(default)]
    pub sections: Option<Vec<Value>>,
    #[serde(default)]
    pub labels: Option<Vec<Value>>,
    #[serde(default)]
    pub filters: Option<Vec<Value>>,
    #[serde(default)]
    pub notes: Option<Vec<Value>>,
    #[serde(default)]
    pub reminders: Option<Vec<Value>>,
    #[serde(default)]
    pub collaborators: Option<Vec<Value>>,
    /// The overview example names this `collaborator_states`, its property table says
    /// `collaborators_states`. Accept both.
    #[serde(default, alias = "collaborators_states")]
    pub collaborator_states: Option<Vec<Value>>,
    #[serde(default)]
    pub completed_info: Option<Vec<Value>>,
}

/// One page of `GET /api/v1/tasks/completed/by_completion_date`.
/// The body key is `items`; `next_cursor` is omitted on the last page.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct CompletedPage {
    #[serde(default)]
    pub items: Vec<Value>,
    #[serde(default)]
    pub next_cursor: Option<String>,
}

/// One page of a paginated list endpoint such as `GET /api/v1/tasks/filter`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Page {
    #[serde(default)]
    pub results: Vec<Value>,
    #[serde(default)]
    pub next_cursor: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn item_from_docs_example_keeps_unknown_fields() {
        let v = json!({
            "id": "6X7rM8997g3RQmvh", "user_id": "2671355", "project_id": "6Jf8VQXxpwv56VQ7",
            "content": "Buy Milk", "description": "", "priority": 4, "due": null, "deadline": null,
            "parent_id": null, "child_order": 1, "order_key": "a1V", "section_id": "3Ty8VQXxpwv28PK3",
            "day_order": -1, "is_collapsed": false, "labels": ["Food", "Shopping"],
            "added_by_uid": "2671355", "assigned_by_uid": "2671355", "responsible_uid": null,
            "checked": false, "is_deleted": false, "added_at": "2025-01-21T21:28:43.841504Z",
            "updated_at": "2025-01-21T21:28:43Z", "completed_at": null,
            "duration": {"amount": 15, "unit": "minute"}, "future_field": {"x": 1}
        });
        let item: Item = serde_json::from_value(v).unwrap();
        assert_eq!(item.p(), 1);
        assert_eq!(item.labels, vec!["Food", "Shopping"]);
        assert_eq!(item.extra.get("future_field"), Some(&json!({"x": 1})));
        let back = serde_json::to_value(&item).unwrap();
        assert_eq!(back["future_field"], json!({"x": 1}));
        assert_eq!(back["order_key"], json!("a1V"));
    }

    #[test]
    fn numeric_ids_and_missing_fields() {
        let p: Project =
            serde_json::from_value(json!({"id": 12, "name": "X", "workspace_id": 12345})).unwrap();
        assert_eq!(p.id, "12");
        assert_eq!(p.workspace_id.as_deref(), Some("12345"));
        assert!(!p.inbox_project);
        let item: Item = serde_json::from_value(json!({"id": "1", "labels": null})).unwrap();
        assert!(item.labels.is_empty());
        assert_eq!(item.priority, 1);
        assert_eq!(item.p(), 4);
    }

    #[test]
    fn command_status_variants() {
        let r: SyncResponse = serde_json::from_value(json!({
            "sync_status": {
                "a": "ok",
                "b": {"error_code": 15, "error": "Invalid temporary id"},
                "c": {"error_tag": "X", "error_code": 20, "error": "e", "http_code": 429,
                      "error_extra": {"retry_after": 3}}
            },
            "collaborators_states": [{"project_id": "1", "user_id": "2", "state": "active"}]
        }))
        .unwrap();
        assert_eq!(r.sync_status["a"], CommandStatus::Ok);
        let CommandStatus::Error(b) = &r.sync_status["b"] else {
            panic!()
        };
        assert!(!b.is_retryable());
        let CommandStatus::Error(c) = &r.sync_status["c"] else {
            panic!()
        };
        assert!(c.is_retryable());
        assert_eq!(c.retry_after(), Some(3));
        assert_eq!(r.collaborator_states.unwrap().len(), 1);
    }

    #[test]
    fn priority_mapping() {
        assert_eq!(api_priority_to_p(4), 1);
        assert_eq!(api_priority_to_p(1), 4);
        assert_eq!(p_to_api_priority(1), 4);
        assert_eq!(p_to_api_priority(4), 1);
    }
}
