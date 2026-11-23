//! Local SQLite cache.
//!
//! Every synced object lives in a table with a few indexed columns derived from its JSON,
//! plus:
//! - `raw`: the current effective JSON (server state with queued local commands applied),
//! - `server_raw`: the last JSON the server sent (`NULL` for objects created locally),
//! - `local_pending`: the row was changed by a queued command,
//! - `local_deleted`: a queued command deleted the row (hidden from queries).
//!
//! After every sync response the engine resets locally changed rows to `server_raw` and
//! replays the still-queued commands in order. The same mechanism serves optimistic updates,
//! "server wins" conflict handling and rollback of rejected commands.

mod apply;
mod local;
mod place;
mod query;
mod queue;
mod schema;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde_json::Value;

use crate::clock::{self, Now};

pub use apply::{CompletedRange, completed_stamp};
pub use place::{DeleteCounts, TaskPlace};
pub use query::*;
pub use queue::{FailedCommand, MAX_ATTEMPTS, QueuedCommand, UNDO_DELAY, wall_ms};

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, StoreError>;

/// Object kinds stored in the cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    Item,
    Project,
    Section,
    Label,
    Filter,
    Note,
    Reminder,
    Collaborator,
    CollaboratorState,
    User,
    CompletedInfo,
    CompletedItem,
    Queue,
}

impl Kind {
    /// Kinds with a generic object table (`id` primary key, raw/server_raw columns).
    pub(crate) const OBJECT_KINDS: [Kind; 8] = [
        Kind::Item,
        Kind::Project,
        Kind::Section,
        Kind::Label,
        Kind::Filter,
        Kind::Note,
        Kind::Reminder,
        Kind::Collaborator,
    ];

    pub(crate) fn table(self) -> &'static str {
        match self {
            Kind::Item => "items",
            Kind::Project => "projects",
            Kind::Section => "sections",
            Kind::Label => "labels",
            Kind::Filter => "filters",
            Kind::Note => "notes",
            Kind::Reminder => "reminders",
            Kind::Collaborator => "collaborators",
            Kind::CollaboratorState => "collaborator_states",
            Kind::User => "meta",
            Kind::CompletedInfo => "completed_info",
            Kind::CompletedItem => "completed_items",
            Kind::Queue => "queue",
        }
    }
}

/// What a store operation changed, for UI refreshes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Changes {
    /// Everything may have changed (full sync).
    pub full: bool,
    pub objects: BTreeSet<(Kind, String)>,
    pub kinds: BTreeSet<Kind>,
    /// Temp ids the server replaced, as `(temp_id, real_id)`. A view that still shows a
    /// temp id (for example the detail pane of a just-created task) switches to the real id.
    /// Commands enqueued with the old temp id are rewritten anyway (see `Store::enqueue`).
    pub id_map: Vec<(String, String)>,
}

impl Changes {
    pub fn add(&mut self, kind: Kind, id: &str) {
        self.kinds.insert(kind);
        self.objects.insert((kind, id.to_owned()));
    }
    pub fn touch(&mut self, kind: Kind) {
        self.kinds.insert(kind);
    }
    pub fn merge(&mut self, other: Changes) {
        self.full |= other.full;
        self.objects.extend(other.objects);
        self.kinds.extend(other.kinds);
        for pair in other.id_map {
            if !self.id_map.contains(&pair) {
                self.id_map.push(pair);
            }
        }
    }
    pub fn is_empty(&self) -> bool {
        !self.full && self.kinds.is_empty() && self.id_map.is_empty()
    }
    /// The real id for `id` if this change replaced it, else `id`.
    pub fn resolve<'a>(&'a self, id: &'a str) -> &'a str {
        self.id_map
            .iter()
            .find(|(t, _)| t == id)
            .map_or(id, |(_, r)| r.as_str())
    }
    pub fn ids(&self, kind: Kind) -> impl Iterator<Item = &str> {
        self.objects
            .iter()
            .filter(move |(k, _)| *k == kind)
            .map(|(_, id)| id.as_str())
    }
}

pub type Clock = Arc<dyn Fn() -> Now + Send + Sync>;

/// The cache. Open one per thread: the UI thread reads, the sync engine writes; WAL mode keeps
/// reads from blocking on writes.
pub struct Store {
    conn: Connection,
    clock: Clock,
    path: Option<PathBuf>,
}

impl std::fmt::Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Store").field("path", &self.path).finish()
    }
}

/// `$XDG_DATA_HOME/tick/cache.sqlite3` (falls back to `~/.local/share`).
pub fn default_path() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("tick").join("cache.sqlite3")
}

impl Store {
    /// Open (and migrate) the cache at `path`, creating parent directories.
    pub fn open(path: impl AsRef<Path>) -> Result<Store> {
        let path = path.as_ref();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let conn = Connection::open(path)?;
        Store::init(conn, Some(path.to_owned()))
    }

    /// Open the cache at [`default_path`].
    pub fn open_default() -> Result<Store> {
        Store::open(default_path())
    }

    /// An in-memory store (tests).
    pub fn open_in_memory() -> Result<Store> {
        Store::init(Connection::open_in_memory()?, None)
    }

    fn init(conn: Connection, path: Option<PathBuf>) -> Result<Store> {
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        if path.is_some() {
            conn.pragma_update(None, "journal_mode", "WAL")?;
        }
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "OFF")?;
        schema::migrate(&conn)?;
        Ok(Store {
            conn,
            clock: Arc::new(clock::now),
            path,
        })
    }

    /// Replace the clock (tests, `TICK_NOW` is read by the default clock).
    pub fn set_clock(&mut self, clock: Clock) {
        self.clock = clock;
    }

    pub fn now(&self) -> Now {
        (self.clock)()
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Direct access for advanced queries and tests.
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// How long a write waits for another connection's write transaction (default 5 s).
    /// The UI connection may want a shorter wait and retry from the main loop instead.
    pub fn set_busy_timeout(&self, timeout: std::time::Duration) -> Result<()> {
        self.conn.busy_timeout(timeout)?;
        Ok(())
    }

    /// Start a write transaction. `IMMEDIATE` takes the write lock up front: a deferred
    /// transaction that starts with a read and upgrades later fails with `SQLITE_BUSY` under
    /// WAL (without waiting for `busy_timeout`) when the other connection committed between.
    pub(crate) fn write_tx(&mut self) -> Result<Transaction<'_>> {
        Ok(self
            .conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?)
    }

    /// The real id for an id the server replaced (a temp id of a synced create), else `id`.
    pub fn resolve_id(&self, id: &str) -> Result<String> {
        Ok(self
            .conn
            .query_row("SELECT real FROM temp_ids WHERE temp = ?1", [id], |r| {
                r.get(0)
            })
            .optional()?
            .unwrap_or_else(|| id.to_owned()))
    }

    // ---- settings (UI preferences) ----

    pub fn setting(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| {
                r.get(0)
            })
            .optional()?)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings(key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn remove_setting(&self, key: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM settings WHERE key = ?1", [key])?;
        Ok(())
    }

    // ---- sync metadata ----

    pub fn sync_token(&self) -> Result<Option<String>> {
        meta_get(&self.conn, "sync_token")
    }

    /// Forget the sync token so the next sync is a full sync.
    pub fn clear_sync_token(&self) -> Result<()> {
        self.conn
            .execute("DELETE FROM meta WHERE key = 'sync_token'", [])?;
        Ok(())
    }

    /// Delete all cached data (logout). Settings are kept.
    pub fn clear_all(&mut self) -> Result<()> {
        let tx = self.write_tx()?;
        clear_cached(&tx)?;
        tx.commit()?;
        Ok(())
    }
}

/// Every cached object and the sync state (queue, history, filter caches, reminder state).
fn clear_cached(tx: &Transaction<'_>) -> Result<()> {
    for kind in Kind::OBJECT_KINDS {
        tx.execute(&format!("DELETE FROM {}", kind.table()), [])?;
    }
    tx.execute_batch(
        "DELETE FROM item_labels; DELETE FROM collaborator_states; DELETE FROM completed_info;
         DELETE FROM completed_items; DELETE FROM filter_cache; DELETE FROM queue;
         DELETE FROM failed_commands; DELETE FROM meta; DELETE FROM temp_ids;
         DELETE FROM fired_reminders;",
    )?;
    Ok(())
}

impl Store {
    /// Log out: delete every cached object, the queue, reminder state and the settings
    /// except the device preferences (`color-scheme`, `run-at-login`), in one transaction
    /// (a failure leaves the cache as it was, not half logged out).
    pub fn clear_account(&mut self) -> Result<()> {
        let tx = self.write_tx()?;
        clear_cached(&tx)?;
        tx.execute(
            "DELETE FROM settings WHERE key NOT IN ('color-scheme', 'run-at-login')",
            [],
        )?;
        tx.commit()?;
        Ok(())
    }
}

pub(crate) fn meta_get(conn: &Connection, key: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0))
        .optional()?)
}

pub(crate) fn meta_set(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

// ---- generic object rows ----

/// Where a row write comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Origin {
    Server,
    Local,
}

fn s(v: &Value, k: &str) -> Option<String> {
    match v.get(k)? {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn i(v: &Value, k: &str) -> i64 {
    match v.get(k) {
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0),
        Some(Value::Bool(b)) => i64::from(*b),
        Some(Value::String(s)) => s.parse().unwrap_or(0),
        _ => 0,
    }
}

fn b(v: &Value, k: &str) -> i64 {
    i64::from(match v.get(k) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0) != 0,
        _ => false,
    })
}

type Col = (&'static str, rusqlite::types::Value);

fn text(v: Option<String>) -> rusqlite::types::Value {
    v.map_or(rusqlite::types::Value::Null, rusqlite::types::Value::Text)
}

fn int(v: i64) -> rusqlite::types::Value {
    rusqlite::types::Value::Integer(v)
}

/// Derived columns per kind.
fn columns(kind: Kind, v: &Value, now: &Now) -> Vec<Col> {
    match kind {
        Kind::Item => {
            let due_date = v.get("due").and_then(|d| s(d, "date"));
            let resolved = due_date
                .as_deref()
                .and_then(|d| clock::resolve_due_date(d, now.offset()));
            vec![
                (
                    "project_id",
                    text(s(v, "project_id").or(Some(String::new()))),
                ),
                ("section_id", text(s(v, "section_id"))),
                ("parent_id", text(s(v, "parent_id"))),
                ("content", text(Some(s(v, "content").unwrap_or_default()))),
                (
                    "description",
                    text(Some(s(v, "description").unwrap_or_default())),
                ),
                (
                    "priority",
                    int(match i(v, "priority") {
                        0 => 1,
                        p => p,
                    }),
                ),
                ("due_date", text(due_date.clone())),
                (
                    "due_day",
                    text(resolved.map(|r| r.date.format("%Y-%m-%d").to_string())),
                ),
                ("due_sort", text(resolved.map(|r| r.sort_key()))),
                (
                    "due_recurring",
                    int(v.get("due").map_or(0, |d| b(d, "is_recurring"))),
                ),
                (
                    "deadline_date",
                    text(v.get("deadline").and_then(|d| s(d, "date"))),
                ),
                ("child_order", int(i(v, "child_order"))),
                ("order_key", text(s(v, "order_key"))),
                ("day_order", int(i(v, "day_order"))),
                ("responsible_uid", text(s(v, "responsible_uid"))),
                ("checked", int(b(v, "checked"))),
                ("is_collapsed", int(b(v, "is_collapsed"))),
                ("added_at", text(s(v, "added_at"))),
                ("completed_at", text(s(v, "completed_at"))),
            ]
        }
        Kind::Project => vec![
            ("name", text(Some(s(v, "name").unwrap_or_default()))),
            ("parent_id", text(s(v, "parent_id"))),
            ("child_order", int(i(v, "child_order"))),
            ("order_key", text(s(v, "order_key"))),
            ("is_archived", int(b(v, "is_archived"))),
            ("is_favorite", int(b(v, "is_favorite"))),
            ("inbox_project", int(b(v, "inbox_project"))),
            ("shared", int(b(v, "shared"))),
        ],
        Kind::Section => vec![
            (
                "project_id",
                text(Some(s(v, "project_id").unwrap_or_default())),
            ),
            ("name", text(Some(s(v, "name").unwrap_or_default()))),
            ("section_order", int(i(v, "section_order"))),
            ("order_key", text(s(v, "order_key"))),
            ("is_archived", int(b(v, "is_archived"))),
        ],
        Kind::Label | Kind::Filter => vec![
            ("name", text(Some(s(v, "name").unwrap_or_default()))),
            ("item_order", int(i(v, "item_order"))),
            ("order_key", text(s(v, "order_key"))),
            ("is_favorite", int(b(v, "is_favorite"))),
        ],
        Kind::Note => vec![
            ("item_id", text(Some(s(v, "item_id").unwrap_or_default()))),
            ("posted_at", text(s(v, "posted_at"))),
        ],
        Kind::Reminder => vec![
            ("item_id", text(Some(s(v, "item_id").unwrap_or_default()))),
            ("kind", text(s(v, "type"))),
        ],
        Kind::Collaborator => vec![("full_name", text(s(v, "full_name")))],
        _ => Vec::new(),
    }
}

/// Insert or update one object row.
pub(crate) fn put_object(
    tx: &Transaction<'_>,
    kind: Kind,
    v: &Value,
    origin: Origin,
    now: &Now,
    changes: &mut Changes,
) -> Result<()> {
    let id = s(v, "id").ok_or_else(|| StoreError::Invalid(format!("{kind:?} without id")))?;
    let cols = columns(kind, v, now);
    let raw = serde_json::to_string(v)?;
    let table = kind.table();
    let names: Vec<&str> = cols.iter().map(|(n, _)| *n).collect();
    let mut sql = format!("INSERT INTO {table} (id, raw, server_raw, local_pending, local_deleted");
    for n in &names {
        sql.push_str(", ");
        sql.push_str(n);
    }
    sql.push_str(") VALUES (?1, ?2, ?3, ?4, 0");
    for idx in 0..names.len() {
        sql.push_str(&format!(", ?{}", idx + 5));
    }
    sql.push_str(") ON CONFLICT(id) DO UPDATE SET raw = excluded.raw, local_deleted = 0, local_pending = excluded.local_pending");
    if origin == Origin::Server {
        sql.push_str(", server_raw = excluded.server_raw");
    }
    for n in &names {
        sql.push_str(&format!(", {n} = excluded.{n}"));
    }
    let mut stmt = tx.prepare_cached(&sql)?;
    let server_raw = match origin {
        Origin::Server => Some(raw.clone()),
        Origin::Local => None,
    };
    let pending = i64::from(origin == Origin::Local);
    let mut values: Vec<rusqlite::types::Value> = vec![
        rusqlite::types::Value::Text(id.clone()),
        rusqlite::types::Value::Text(raw),
        text(server_raw),
        int(pending),
    ];
    values.extend(cols.into_iter().map(|(_, v)| v));
    stmt.execute(rusqlite::params_from_iter(values))?;
    if kind == Kind::Item {
        tx.prepare_cached("DELETE FROM item_labels WHERE item_id = ?1")?
            .execute([&id])?;
        if let Some(Value::Array(labels)) = v.get("labels") {
            let mut ins = tx.prepare_cached(
                "INSERT OR IGNORE INTO item_labels(item_id, name) VALUES (?1, ?2)",
            )?;
            for l in labels.iter().filter_map(Value::as_str) {
                ins.execute(params![id, l])?;
            }
        }
    }
    changes.add(kind, &id);
    Ok(())
}

/// Remove a row because the server says it is deleted.
pub(crate) fn remove_object(
    tx: &Transaction<'_>,
    kind: Kind,
    id: &str,
    changes: &mut Changes,
) -> Result<()> {
    tx.prepare_cached(&format!("DELETE FROM {} WHERE id = ?1", kind.table()))?
        .execute([id])?;
    if kind == Kind::Item {
        tx.prepare_cached("DELETE FROM item_labels WHERE item_id = ?1")?
            .execute([id])?;
    }
    changes.add(kind, id);
    Ok(())
}

/// Hide a row because a queued command deletes it.
pub(crate) fn delete_local(
    tx: &Transaction<'_>,
    kind: Kind,
    id: &str,
    changes: &mut Changes,
) -> Result<()> {
    let n = tx
        .prepare_cached(&format!(
            "UPDATE {} SET local_deleted = 1, local_pending = 1 WHERE id = ?1 AND local_deleted = 0",
            kind.table()
        ))?
        .execute([id])?;
    if n > 0 {
        changes.add(kind, id);
    }
    Ok(())
}

/// The effective JSON of a visible row.
pub(crate) fn load_raw(tx: &Connection, kind: Kind, id: &str) -> Result<Option<Value>> {
    let raw: Option<String> = tx
        .prepare_cached(&format!(
            "SELECT raw FROM {} WHERE id = ?1 AND local_deleted = 0",
            kind.table()
        ))?
        .query_row([id], |r| r.get(0))
        .optional()?;
    Ok(match raw {
        Some(r) => Some(serde_json::from_str(&r)?),
        None => None,
    })
}

/// Reset every locally changed row to the server copy (or drop it if the server never had
/// it). Called before replaying the queue.
pub(crate) fn reset_local(tx: &Transaction<'_>, now: &Now, changes: &mut Changes) -> Result<()> {
    for kind in Kind::OBJECT_KINDS {
        let rows: Vec<(String, Option<String>)> = {
            let mut stmt = tx.prepare_cached(&format!(
                "SELECT id, server_raw FROM {} WHERE local_pending = 1",
                kind.table()
            ))?;
            stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<std::result::Result<_, _>>()?
        };
        for (id, server_raw) in rows {
            match server_raw {
                Some(raw) => {
                    let v: Value = serde_json::from_str(&raw)?;
                    put_object(tx, kind, &v, Origin::Server, now, changes)?;
                }
                None => remove_object(tx, kind, &id, changes)?,
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
