//! The persisted offline command queue.

use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde_json::Value;

use super::{Changes, Kind, Result, Store, local, reset_local};
use crate::clock::Now;
use crate::command::Command;
use crate::model::CommandError;

/// A command waiting to be sent.
#[derive(Debug, Clone, PartialEq)]
pub struct QueuedCommand {
    pub seq: i64,
    pub command: Command,
    pub created_at: String,
    pub attempts: i64,
    pub last_error: Option<String>,
    /// Wall-clock Unix ms before which the command is not sent (undo window), see
    /// [`Store::enqueue_deferred`].
    pub not_before: Option<i64>,
}

impl QueuedCommand {
    /// Whether the command may be sent at wall-clock time `now_ms`.
    pub fn is_due(&self, now_ms: i64) -> bool {
        self.not_before.is_none_or(|t| t <= now_ms)
    }
}

/// A queued command whose server-side attempts (retryable command errors, unreported
/// commands, rejected quick adds) reach this count is moved to `failed_commands`, so one
/// command cannot hold the whole queue in backoff forever. Network failures do not count.
pub const MAX_ATTEMPTS: i64 = 10;

/// Suggested [`Store::enqueue_deferred`] delay for completing and deleting tasks (`SPEC.md`:
/// an "Undo" toast for about 5 s).
pub const UNDO_DELAY: std::time::Duration = std::time::Duration::from_secs(5);

/// How long [`Store::resolve_id`] and enqueue-time rewriting remember a temp id.
const TEMP_ID_TTL_MS: i64 = 30 * 24 * 3600 * 1000;

/// Wall-clock time in Unix ms. Deliberately not the store clock: `TICK_NOW` freezes that,
/// and a deferred command must still become due.
pub fn wall_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

/// A command the server rejected for good. Kept so the UI can show the error.
#[derive(Debug, Clone, PartialEq)]
pub struct FailedCommand {
    pub seq: i64,
    pub command: Command,
    pub error: String,
    pub detail: Option<CommandError>,
    pub failed_at: String,
}

pub(crate) fn load_queue(conn: &Connection) -> Result<Vec<QueuedCommand>> {
    let mut stmt = conn.prepare_cached(
        "SELECT seq, uuid, kind, temp_id, args, created_at, attempts, last_error, not_before
         FROM queue ORDER BY seq",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, Option<String>>(3)?,
            r.get::<_, String>(4)?,
            r.get::<_, String>(5)?,
            r.get::<_, i64>(6)?,
            r.get::<_, Option<String>>(7)?,
            r.get::<_, Option<i64>>(8)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (seq, uuid, kind, temp_id, args, created_at, attempts, last_error, not_before) = row?;
        out.push(QueuedCommand {
            seq,
            command: Command {
                kind,
                uuid,
                temp_id,
                args: serde_json::from_str(&args)?,
            },
            created_at,
            attempts,
            last_error,
            not_before,
        });
    }
    Ok(out)
}

pub(crate) fn delete_queued(tx: &Transaction<'_>, uuid: &str) -> Result<()> {
    tx.prepare_cached("DELETE FROM queue WHERE uuid = ?1")?
        .execute([uuid])?;
    Ok(())
}

pub(crate) fn mark_attempt(tx: &Transaction<'_>, uuid: &str, error: Option<&str>) -> Result<()> {
    tx.prepare_cached(
        "UPDATE queue SET attempts = attempts + 1, last_error = COALESCE(?2, last_error)
         WHERE uuid = ?1",
    )?
    .execute(params![uuid, error])?;
    Ok(())
}

/// Move a queued command to `failed_commands`.
pub(crate) fn fail_queued(
    tx: &Transaction<'_>,
    q: &QueuedCommand,
    error: &str,
    detail: Option<&CommandError>,
    now: &Now,
) -> Result<()> {
    tx.prepare_cached(
        "INSERT OR REPLACE INTO failed_commands
         (seq, uuid, kind, temp_id, args, created_at, error, error_json, failed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )?
    .execute(params![
        q.seq,
        q.command.uuid,
        q.command.kind,
        q.command.temp_id,
        serde_json::to_string(&q.command.args)?,
        q.created_at,
        error,
        detail.map(serde_json::to_string).transpose()?,
        now.utc_string(),
    ])?;
    delete_queued(tx, &q.command.uuid)
}

/// Rewrite `from` to `to` in every queued command.
pub(crate) fn remap_queue(tx: &Transaction<'_>, from: &str, to: &str) -> Result<()> {
    for mut q in load_queue(tx)? {
        if q.command.remap_id(from, to) {
            tx.prepare_cached("UPDATE queue SET args = ?2, temp_id = ?3 WHERE seq = ?1")?
                .execute(params![
                    q.seq,
                    serde_json::to_string(&q.command.args)?,
                    q.command.temp_id
                ])?;
        }
    }
    Ok(())
}

/// Remember that the server replaced `temp` with `real` (see [`Store::resolve_id`]).
pub(crate) fn record_temp_id(tx: &Transaction<'_>, temp: &str, real: &str) -> Result<()> {
    if temp == real {
        return Ok(());
    }
    tx.prepare_cached(
        "INSERT INTO temp_ids (temp, real, created_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(temp) DO UPDATE SET real = excluded.real, created_at = excluded.created_at",
    )?
    .execute(params![temp, real, wall_ms()])?;
    Ok(())
}

/// Forget mappings older than [`TEMP_ID_TTL_MS`].
pub(crate) fn prune_temp_ids(tx: &Transaction<'_>) -> Result<()> {
    tx.prepare_cached("DELETE FROM temp_ids WHERE created_at < ?1")?
        .execute([wall_ms() - TEMP_ID_TTL_MS])?;
    Ok(())
}

/// Strings in a command that may be ids: string values anywhere in `args` (including
/// nested arrays such as `items[].id`), object keys (`ids_to_orders`) and `temp_id`.
fn id_candidates(command: &Command) -> Vec<String> {
    fn walk(v: &Value, out: &mut Vec<String>) {
        match v {
            Value::String(s) => out.push(s.clone()),
            Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
            Value::Object(m) => {
                for (k, x) in m {
                    out.push(k.clone());
                    walk(x, out);
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(&command.args, &mut out);
    out.extend(command.temp_id.clone());
    // Temp ids are UUIDs (`Command::create`); skip everything else without a lookup.
    out.retain(|s| s.len() == 36 && s.matches('-').count() == 4);
    out.sort();
    out.dedup();
    out
}

/// Rewrite temp ids the server already replaced. A view can still hold a temp id after the
/// sync that resolved it (the local row was replaced by the real one); without this the
/// command would be a local no-op and fail on the server with `INVALID_TEMPID`.
/// Returns the `(temp, real)` pairs it applied.
fn rewrite_resolved_temp_ids(
    tx: &Transaction<'_>,
    command: &mut Command,
) -> Result<Vec<(String, String)>> {
    let mut stmt = tx.prepare_cached("SELECT real FROM temp_ids WHERE temp = ?1")?;
    let mut applied = Vec::new();
    for id in id_candidates(command) {
        let real: Option<String> = stmt.query_row([&id], |r| r.get(0)).optional()?;
        if let Some(real) = real
            && command.remap_id(&id, &real)
        {
            applied.push((id, real));
        }
    }
    Ok(applied)
}

/// Reset local changes to server state and replay the queue on top.
pub(crate) fn rebuild_local(tx: &Transaction<'_>, now: &Now, changes: &mut Changes) -> Result<()> {
    reset_local(tx, now, changes)?;
    replay_queue(tx, now, changes)
}

pub(crate) fn replay_queue(tx: &Transaction<'_>, now: &Now, changes: &mut Changes) -> Result<()> {
    for q in load_queue(tx)? {
        local::apply_command(tx, &q.command, now, changes)?;
    }
    Ok(())
}

impl Store {
    /// Apply a command optimistically and persist it in the queue, in one transaction.
    pub fn enqueue(&mut self, command: Command) -> Result<Changes> {
        self.enqueue_many(vec![command])
    }

    /// Enqueue several commands atomically (for example a move plus a reorder).
    ///
    /// Ids the server already replaced are rewritten first (see [`Store::resolve_id`]); the
    /// returned [`Changes::id_map`] lists them so the caller can update what it shows.
    pub fn enqueue_many(&mut self, commands: Vec<Command>) -> Result<Changes> {
        self.enqueue_at(commands, None)
    }

    /// Enqueue commands that are not sent before `delay` has passed (wall clock), so an
    /// "Undo" toast can still call [`Store::discard_queued`]: a sent `item_delete` cannot be
    /// undone. Use [`UNDO_DELAY`] for completing and deleting tasks. The local effect is
    /// immediate. Later commands queue behind them (order is kept), and the engine wakes up
    /// when they become due ([`Store::next_deferred_due`]).
    pub fn enqueue_deferred(
        &mut self,
        commands: Vec<Command>,
        delay: std::time::Duration,
    ) -> Result<Changes> {
        let at = wall_ms().saturating_add(i64::try_from(delay.as_millis()).unwrap_or(i64::MAX));
        self.enqueue_at(commands, Some(at))
    }

    fn enqueue_at(&mut self, commands: Vec<Command>, not_before: Option<i64>) -> Result<Changes> {
        let now = self.now();
        let tx = self.write_tx()?;
        let mut changes = Changes::default();
        for mut command in commands {
            for pair in rewrite_resolved_temp_ids(&tx, &mut command)? {
                if !changes.id_map.contains(&pair) {
                    changes.id_map.push(pair);
                }
            }
            tx.prepare_cached(
                "INSERT INTO queue (uuid, kind, temp_id, args, created_at, not_before)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )?
            .execute(params![
                command.uuid,
                command.kind,
                command.temp_id,
                serde_json::to_string(&command.args)?,
                now.utc_string(),
                not_before,
            ])?;
            local::apply_command(&tx, &command, &now, &mut changes)?;
        }
        changes.touch(Kind::Queue);
        tx.commit()?;
        Ok(changes)
    }

    /// Commands waiting to be sent, oldest first.
    pub fn pending_commands(&self) -> Result<Vec<QueuedCommand>> {
        load_queue(&self.conn)
    }

    /// Time until the earliest deferred command becomes due, if any is still waiting.
    pub fn next_deferred_due(&self) -> Result<Option<std::time::Duration>> {
        let now = wall_ms();
        let at: Option<i64> = self.conn.query_row(
            "SELECT MIN(not_before) FROM queue WHERE not_before > ?1",
            [now],
            |r| r.get(0),
        )?;
        Ok(at.map(|t| std::time::Duration::from_millis((t - now).max(0) as u64)))
    }

    /// Make every deferred command due now (for example when the app quits while an undo
    /// toast is still showing).
    pub fn release_deferred(&self) -> Result<()> {
        self.conn.execute(
            "UPDATE queue SET not_before = NULL WHERE not_before IS NOT NULL",
            [],
        )?;
        Ok(())
    }

    pub fn pending_count(&self) -> Result<usize> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM queue", [], |r| r.get(0))?;
        Ok(n as usize)
    }

    /// Commands the server rejected, newest first.
    pub fn failed_commands(&self) -> Result<Vec<FailedCommand>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT seq, uuid, kind, temp_id, args, error, error_json, failed_at
             FROM failed_commands ORDER BY seq DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, Option<String>>(6)?,
                r.get::<_, String>(7)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (seq, uuid, kind, temp_id, args, error, detail, failed_at) = row?;
            out.push(FailedCommand {
                seq,
                command: Command {
                    kind,
                    uuid,
                    temp_id,
                    args: serde_json::from_str::<Value>(&args)?,
                },
                error,
                detail: detail.and_then(|d| serde_json::from_str(&d).ok()),
                failed_at,
            });
        }
        Ok(out)
    }

    /// Forget a failed command after the UI showed it.
    pub fn dismiss_failed(&self, seq: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM failed_commands WHERE seq = ?1", [seq])?;
        Ok(())
    }

    /// Drop a queued command without sending it (for example "discard change"), rolling back
    /// its optimistic effect.
    pub fn discard_queued(&mut self, uuid: &str) -> Result<Changes> {
        let now = self.now();
        let tx = self.write_tx()?;
        let mut changes = Changes::default();
        delete_queued(&tx, uuid)?;
        rebuild_local(&tx, &now, &mut changes)?;
        changes.touch(Kind::Queue);
        tx.commit()?;
        Ok(changes)
    }
}
