//! Schema and `PRAGMA user_version` migrations.

use rusqlite::Connection;

use super::Result;

const COMMON: &str = "raw TEXT NOT NULL, server_raw TEXT,
    local_pending INTEGER NOT NULL DEFAULT 0, local_deleted INTEGER NOT NULL DEFAULT 0";

pub(super) fn v1() -> String {
    format!(
        "
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);

CREATE TABLE items (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL DEFAULT '',
    section_id TEXT,
    parent_id TEXT,
    content TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    priority INTEGER NOT NULL DEFAULT 1,
    due_date TEXT,
    due_day TEXT,
    due_sort TEXT,
    due_recurring INTEGER NOT NULL DEFAULT 0,
    deadline_date TEXT,
    child_order INTEGER NOT NULL DEFAULT 0,
    order_key TEXT COLLATE BINARY,
    day_order INTEGER NOT NULL DEFAULT 0,
    responsible_uid TEXT,
    checked INTEGER NOT NULL DEFAULT 0,
    is_collapsed INTEGER NOT NULL DEFAULT 0,
    added_at TEXT,
    completed_at TEXT,
    {COMMON}
);
CREATE INDEX items_project ON items(project_id, section_id);
CREATE INDEX items_due ON items(due_day);
CREATE INDEX items_parent ON items(parent_id);
CREATE INDEX items_pending ON items(local_pending) WHERE local_pending = 1;

CREATE TABLE item_labels (
    item_id TEXT NOT NULL,
    name TEXT NOT NULL,
    PRIMARY KEY (item_id, name)
) WITHOUT ROWID;
CREATE INDEX item_labels_name ON item_labels(name COLLATE NOCASE);

CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL DEFAULT '',
    parent_id TEXT,
    child_order INTEGER NOT NULL DEFAULT 0,
    order_key TEXT COLLATE BINARY,
    is_archived INTEGER NOT NULL DEFAULT 0,
    is_favorite INTEGER NOT NULL DEFAULT 0,
    inbox_project INTEGER NOT NULL DEFAULT 0,
    shared INTEGER NOT NULL DEFAULT 0,
    {COMMON}
);

CREATE TABLE sections (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL DEFAULT '',
    name TEXT NOT NULL DEFAULT '',
    section_order INTEGER NOT NULL DEFAULT 0,
    order_key TEXT COLLATE BINARY,
    is_archived INTEGER NOT NULL DEFAULT 0,
    {COMMON}
);
CREATE INDEX sections_project ON sections(project_id);

CREATE TABLE labels (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL DEFAULT '',
    item_order INTEGER NOT NULL DEFAULT 0,
    order_key TEXT COLLATE BINARY,
    is_favorite INTEGER NOT NULL DEFAULT 0,
    {COMMON}
);

CREATE TABLE filters (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL DEFAULT '',
    item_order INTEGER NOT NULL DEFAULT 0,
    order_key TEXT COLLATE BINARY,
    is_favorite INTEGER NOT NULL DEFAULT 0,
    {COMMON}
);

CREATE TABLE notes (
    id TEXT PRIMARY KEY,
    item_id TEXT NOT NULL DEFAULT '',
    posted_at TEXT,
    {COMMON}
);
CREATE INDEX notes_item ON notes(item_id);

CREATE TABLE reminders (
    id TEXT PRIMARY KEY,
    item_id TEXT NOT NULL DEFAULT '',
    kind TEXT,
    {COMMON}
);
CREATE INDEX reminders_item ON reminders(item_id);

CREATE TABLE collaborators (
    id TEXT PRIMARY KEY,
    full_name TEXT,
    {COMMON}
);

CREATE TABLE collaborator_states (
    project_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT '',
    raw TEXT NOT NULL,
    PRIMARY KEY (project_id, user_id)
) WITHOUT ROWID;

CREATE TABLE completed_info (
    kind TEXT NOT NULL,
    id TEXT NOT NULL,
    completed_items INTEGER NOT NULL DEFAULT 0,
    raw TEXT NOT NULL,
    PRIMARY KEY (kind, id)
) WITHOUT ROWID;

-- Completed history cache (GET /tasks/completed/by_completion_date).
CREATE TABLE completed_items (
    id TEXT PRIMARY KEY,
    project_id TEXT,
    completed_at TEXT NOT NULL DEFAULT '',
    raw TEXT NOT NULL
);
CREATE INDEX completed_items_at ON completed_items(completed_at);

-- Server results for filter queries the local evaluator does not support.
CREATE TABLE filter_cache (
    query TEXT PRIMARY KEY,
    item_ids TEXT NOT NULL,
    fetched_at TEXT NOT NULL
);

CREATE TABLE queue (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL,
    temp_id TEXT,
    args TEXT NOT NULL,
    created_at TEXT NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT
);

CREATE TABLE failed_commands (
    seq INTEGER PRIMARY KEY,
    uuid TEXT NOT NULL,
    kind TEXT NOT NULL,
    temp_id TEXT,
    args TEXT NOT NULL,
    created_at TEXT NOT NULL,
    error TEXT NOT NULL,
    error_json TEXT,
    failed_at TEXT NOT NULL
);

-- Search over active tasks. The trigram tokenizer gives substring matches for any script
-- (including Chinese, which unicode61 does not segment). Queries shorter than three
-- characters fall back to LIKE (see query.rs).
CREATE VIRTUAL TABLE items_fts USING fts5(content, description, tokenize = 'trigram');

CREATE TRIGGER items_fts_ai AFTER INSERT ON items
WHEN new.checked = 0 AND new.local_deleted = 0 BEGIN
    INSERT INTO items_fts(rowid, content, description)
    VALUES (new.rowid, new.content, new.description);
END;
CREATE TRIGGER items_fts_ad AFTER DELETE ON items BEGIN
    DELETE FROM items_fts WHERE rowid = old.rowid;
END;
CREATE TRIGGER items_fts_au AFTER UPDATE OF content, description, checked, local_deleted ON items
BEGIN
    DELETE FROM items_fts WHERE rowid = old.rowid;
    INSERT INTO items_fts(rowid, content, description)
    SELECT new.rowid, new.content, new.description
    WHERE new.checked = 0 AND new.local_deleted = 0;
END;
"
    )
}

/// v2: temp id mappings that outlive the queued commands, and deferred sending.
const V2: &str = "
-- temp_id -> real id from `temp_id_mapping` and quick-add results. Commands enqueued after
-- the mapping arrived (for example by a view that still holds the temp id) are rewritten
-- with it before they are queued.
CREATE TABLE temp_ids (
    temp TEXT PRIMARY KEY,
    real TEXT NOT NULL,
    -- Wall-clock Unix ms; old rows are pruned.
    created_at INTEGER NOT NULL
) WITHOUT ROWID;

-- Wall-clock time (Unix ms) before which a queued command is not sent (undo window).
ALTER TABLE queue ADD COLUMN not_before INTEGER;
";

/// Bring the schema to the latest version.
///
/// Each step runs in an immediate transaction and re-reads `user_version` inside it, so two
/// connections opening the same cache at startup (UI and engine) cannot both run a step.
pub(super) fn migrate(conn: &Connection) -> Result<()> {
    let migrations: Vec<String> = vec![v1(), V2.to_owned()];
    for (idx, sql) in migrations.iter().enumerate() {
        let target = idx as i64 + 1;
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version >= target {
            continue;
        }
        conn.execute_batch("BEGIN IMMEDIATE")?;
        let step = (|| -> Result<()> {
            let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
            if version < target {
                conn.execute_batch(sql)?;
                conn.pragma_update(None, "user_version", target)?;
            }
            Ok(())
        })();
        match step {
            Ok(()) => conn.execute_batch("COMMIT")?,
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK");
                return Err(e);
            }
        }
    }
    Ok(())
}
