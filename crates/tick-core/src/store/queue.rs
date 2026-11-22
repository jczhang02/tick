//! The persisted offline command queue.

use rusqlite::{Connection, Transaction, params};
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
        "SELECT seq, uuid, kind, temp_id, args, created_at, attempts, last_error
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
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (seq, uuid, kind, temp_id, args, created_at, attempts, last_error) = row?;
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
    pub fn enqueue_many(&mut self, commands: Vec<Command>) -> Result<Changes> {
        let now = self.now();
        let tx = self.conn.transaction()?;
        let mut changes = Changes::default();
        for command in commands {
            tx.prepare_cached(
                "INSERT INTO queue (uuid, kind, temp_id, args, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            )?
            .execute(params![
                command.uuid,
                command.kind,
                command.temp_id,
                serde_json::to_string(&command.args)?,
                now.utc_string()
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
        let tx = self.conn.transaction()?;
        let mut changes = Changes::default();
        delete_queued(&tx, uuid)?;
        rebuild_local(&tx, &now, &mut changes)?;
        changes.touch(Kind::Queue);
        tx.commit()?;
        Ok(changes)
    }
}
