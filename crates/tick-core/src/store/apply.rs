//! Applying server data: `/sync` responses, quick-add results, completed history and
//! server filter results.

use rusqlite::{Transaction, params};
use serde_json::Value;

use super::queue::{self, QueuedCommand, rebuild_local};
use super::{Changes, Kind, Origin, Result, Store, meta_set, put_object, remove_object};
use crate::clock::Now;
use crate::model::{CommandError, CommandStatus, CompletedInfo, SyncResponse, User};

/// What happened to the commands of one `/sync` request.
#[derive(Debug, Clone, Default)]
pub struct SyncApplied {
    pub changes: Changes,
    /// Commands the server accepted.
    pub acknowledged: usize,
    /// Commands rejected for good (moved to `failed_commands`).
    pub failed: Vec<(QueuedCommand, CommandError)>,
    /// Commands that failed with a retryable error and stay queued.
    pub retry: Vec<(QueuedCommand, CommandError)>,
}

fn id_of(v: &Value, key: &str) -> Option<String> {
    match v.get(key)? {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn is_deleted(v: &Value) -> bool {
    match v.get("is_deleted") {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0) != 0,
        _ => false,
    }
}

fn apply_objects(
    tx: &Transaction<'_>,
    kind: Kind,
    objects: &[Value],
    full: bool,
    now: &Now,
    changes: &mut Changes,
) -> Result<()> {
    if full {
        tx.execute(&format!("DELETE FROM {}", kind.table()), [])?;
        if kind == Kind::Item {
            tx.execute("DELETE FROM item_labels", [])?;
        }
        changes.full = true;
        changes.touch(kind);
    }
    for v in objects {
        let Some(id) = id_of(v, "id") else { continue };
        if is_deleted(v) {
            remove_object(tx, kind, &id, changes)?;
        } else {
            put_object(tx, kind, v, Origin::Server, now, changes)?;
        }
    }
    Ok(())
}

/// Apply resource data from a sync response (no command handling).
fn apply_resources(
    tx: &Transaction<'_>,
    resp: &SyncResponse,
    now: &Now,
    changes: &mut Changes,
) -> Result<()> {
    let full = resp.full_sync;
    if let Some(user) = &resp.user {
        let mut user = user.clone();
        User::sanitize(&mut user);
        meta_set(tx, "user", &serde_json::to_string(&user)?)?;
        changes.touch(Kind::User);
    }
    if let Some(settings) = &resp.user_settings {
        meta_set(tx, "user_settings", &serde_json::to_string(settings)?)?;
        changes.touch(Kind::User);
    }
    let lists: [(Kind, &Option<Vec<Value>>); 8] = [
        (Kind::Project, &resp.projects),
        (Kind::Section, &resp.sections),
        (Kind::Item, &resp.items),
        (Kind::Label, &resp.labels),
        (Kind::Filter, &resp.filters),
        (Kind::Note, &resp.notes),
        (Kind::Reminder, &resp.reminders),
        (Kind::Collaborator, &resp.collaborators),
    ];
    for (kind, list) in lists {
        if let Some(list) = list {
            apply_objects(tx, kind, list, full, now, changes)?;
        }
    }
    if full && (resp.collaborators.is_some() || resp.collaborator_states.is_some()) {
        tx.execute("DELETE FROM collaborator_states", [])?;
    }
    if let Some(states) = &resp.collaborator_states {
        for v in states {
            let (Some(p), Some(u)) = (id_of(v, "project_id"), id_of(v, "user_id")) else {
                continue;
            };
            if is_deleted(v) {
                tx.prepare_cached(
                    "DELETE FROM collaborator_states WHERE project_id = ?1 AND user_id = ?2",
                )?
                .execute(params![p, u])?;
            } else {
                let state = v.get("state").and_then(Value::as_str).unwrap_or("");
                tx.prepare_cached(
                    "INSERT INTO collaborator_states (project_id, user_id, state, raw)
                     VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT(project_id, user_id) DO UPDATE SET state = excluded.state,
                     raw = excluded.raw",
                )?
                .execute(params![p, u, state, serde_json::to_string(v)?])?;
            }
            changes.add(Kind::CollaboratorState, &format!("{p}:{u}"));
        }
    }
    if let Some(infos) = &resp.completed_info {
        if full {
            tx.execute("DELETE FROM completed_info", [])?;
        }
        for v in infos {
            let Ok(info) = serde_json::from_value::<CompletedInfo>(v.clone()) else {
                continue;
            };
            let Some((kind, id)) = info.key() else {
                continue;
            };
            tx.prepare_cached(
                "INSERT INTO completed_info (kind, id, completed_items, raw) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(kind, id) DO UPDATE SET completed_items = excluded.completed_items,
                 raw = excluded.raw",
            )?
            .execute(params![kind, id, info.completed_items, serde_json::to_string(v)?])?;
            changes.add(Kind::CompletedInfo, &format!("{kind}:{id}"));
        }
    }
    if let Some(token) = &resp.sync_token {
        meta_set(tx, "sync_token", token)?;
    }
    if full {
        meta_set(tx, "last_full_sync", &now.utc_string())?;
    }
    Ok(())
}

impl Store {
    /// Apply one `/sync` response. `sent` are the queued commands that went with the request.
    ///
    /// In one transaction: settle the sent commands from `sync_status`, rewrite temp ids in
    /// still-queued commands, reset local changes to server state (or clear everything on a
    /// full sync), apply the server's objects, and replay the remaining queue on top
    /// ("server wins").
    pub fn apply_sync_response(
        &mut self,
        sent: &[QueuedCommand],
        resp: &SyncResponse,
    ) -> Result<SyncApplied> {
        let now = self.now();
        let tx = self.conn.transaction()?;
        let mut out = SyncApplied::default();
        let mut changes = Changes::default();

        for q in sent {
            match resp.sync_status.get(&q.command.uuid) {
                Some(CommandStatus::Ok) => {
                    queue::delete_queued(&tx, &q.command.uuid)?;
                    out.acknowledged += 1;
                }
                Some(CommandStatus::Error(e)) if e.is_retryable() => {
                    queue::mark_attempt(&tx, &q.command.uuid, Some(&e.message()))?;
                    out.retry.push((q.clone(), e.clone()));
                }
                Some(CommandStatus::Error(e)) => {
                    queue::fail_queued(&tx, q, &e.message(), Some(e), &now)?;
                    out.failed.push((q.clone(), e.clone()));
                }
                None => {
                    // The server did not report on this command; it stays queued and is sent
                    // again with the same uuid (safe thanks to uuid idempotency).
                    queue::mark_attempt(&tx, &q.command.uuid, None)?;
                }
            }
            changes.touch(Kind::Queue);
        }

        for (temp, real) in &resp.temp_id_mapping {
            queue::remap_queue(&tx, temp, real)?;
        }

        if !resp.full_sync {
            super::reset_local(&tx, &now, &mut changes)?;
        }
        apply_resources(&tx, resp, &now, &mut changes)?;
        queue::replay_queue(&tx, &now, &mut changes)?;
        tx.commit()?;
        out.changes = changes;
        Ok(out)
    }

    /// A queued quick add was accepted: `item` is the created task.
    pub fn apply_quick_add_result(&mut self, q: &QueuedCommand, item: &Value) -> Result<Changes> {
        let now = self.now();
        let tx = self.conn.transaction()?;
        let mut changes = Changes::default();
        queue::delete_queued(&tx, &q.command.uuid)?;
        if let (Some(temp), Some(real)) = (&q.command.temp_id, id_of(item, "id")) {
            queue::remap_queue(&tx, temp, &real)?;
        }
        super::reset_local(&tx, &now, &mut changes)?;
        put_object(&tx, Kind::Item, item, Origin::Server, &now, &mut changes)?;
        queue::replay_queue(&tx, &now, &mut changes)?;
        changes.touch(Kind::Queue);
        tx.commit()?;
        Ok(changes)
    }

    /// A queued command failed outside `/sync` (for example a rejected quick add).
    pub fn fail_command(
        &mut self,
        q: &QueuedCommand,
        error: &str,
        detail: Option<&CommandError>,
    ) -> Result<Changes> {
        let now = self.now();
        let tx = self.conn.transaction()?;
        let mut changes = Changes::default();
        queue::fail_queued(&tx, q, error, detail, &now)?;
        rebuild_local(&tx, &now, &mut changes)?;
        changes.touch(Kind::Queue);
        tx.commit()?;
        Ok(changes)
    }

    /// Record a transient error on a queued command (kept for retry).
    pub fn note_attempt(&mut self, uuid: &str, error: &str) -> Result<()> {
        let tx = self.conn.transaction()?;
        queue::mark_attempt(&tx, uuid, Some(error))?;
        tx.commit()?;
        Ok(())
    }

    /// Store a page of completed tasks in the history cache.
    pub fn cache_completed(&mut self, items: &[Value]) -> Result<Changes> {
        let tx = self.conn.transaction()?;
        let mut changes = Changes::default();
        for v in items {
            let Some(id) = id_of(v, "id") else { continue };
            let completed_at = v
                .get("completed_at")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();
            tx.prepare_cached(
                "INSERT INTO completed_items (id, project_id, completed_at, raw) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(id) DO UPDATE SET project_id = excluded.project_id,
                 completed_at = excluded.completed_at, raw = excluded.raw",
            )?
            .execute(params![
                id,
                id_of(v, "project_id"),
                completed_at,
                serde_json::to_string(v)?
            ])?;
            changes.add(Kind::CompletedItem, &id);
        }
        tx.commit()?;
        Ok(changes)
    }

    /// Cache the server's result for a filter query the local evaluator cannot handle.
    /// The tasks are also upserted so the view can render them.
    pub fn cache_filter_results(&mut self, query: &str, items: &[Value]) -> Result<Changes> {
        let now = self.now();
        let tx = self.conn.transaction()?;
        let mut changes = Changes::default();
        let mut ids = Vec::new();
        for v in items {
            let Some(id) = id_of(v, "id") else { continue };
            // Do not clobber a row a queued command changed; the next sync settles it.
            let pending: bool = tx
                .query_row(
                    "SELECT local_pending FROM items WHERE id = ?1",
                    [&id],
                    |r| r.get::<_, i64>(0),
                )
                .map(|p| p != 0)
                .unwrap_or(false);
            if !pending {
                put_object(&tx, Kind::Item, v, Origin::Server, &now, &mut changes)?;
            }
            ids.push(id);
        }
        tx.execute(
            "INSERT INTO filter_cache (query, item_ids, fetched_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(query) DO UPDATE SET item_ids = excluded.item_ids,
             fetched_at = excluded.fetched_at",
            params![query, serde_json::to_string(&ids)?, now.utc_string()],
        )?;
        tx.commit()?;
        Ok(changes)
    }
}
