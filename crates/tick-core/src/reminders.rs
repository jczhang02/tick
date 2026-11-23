//! When time-based reminders fire (`SPEC.md` Data 5).
//!
//! A reminder fires at one instant: an `absolute` reminder at its own `due.date`, a
//! `relative` one `minute_offset` minutes before its task's due time (the task needs a due
//! time, `docs/api/pages/sync_reminders.md`). Location reminders are ignored.
//!
//! Each firing is remembered by a key made of the reminder id and the instant
//! (`fired_reminders`, schema v3), so a reminder notifies once across syncs and restarts,
//! and fires again when its instant moves (a recurring task's next date, an edited
//! reminder). A reminder whose instant passed while Tick was not running is still shown at
//! the next start when it is at most [`GRACE`] old; older ones are dropped silently.

use chrono::{DateTime, Duration, FixedOffset, Utc};
use rusqlite::params;
use serde_json::Value;

use crate::clock::{Now, resolve_due_date};
use crate::model::Reminder;
use crate::store::{Result, Store, Task, meta_get};

/// How late a missed reminder may still be shown (one hour).
pub const GRACE: Duration = Duration::minutes(60);

/// Fired keys older than this are pruned.
const KEEP_FIRED: Duration = Duration::days(14);

/// A reminder that should notify now.
#[derive(Debug, Clone, PartialEq)]
pub struct DueReminder {
    /// Identifies this firing: reminder id and instant.
    pub key: String,
    pub reminder_id: String,
    pub task: Task,
    pub fire_at: DateTime<Utc>,
    pub urgent: bool,
}

/// What [`Store::reminder_schedule`] found.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Schedule {
    /// Unfired reminders whose instant is in `[now - GRACE, now]`, oldest first.
    pub due: Vec<DueReminder>,
    /// The next instant after `now` at which a reminder fires.
    pub next: Option<DateTime<Utc>>,
}

/// The local wall clock of a due `date` as an instant (`offset` is the local offset).
fn instant(date: &str, offset: FixedOffset) -> Option<DateTime<Utc>> {
    let local = resolve_due_date(date, offset)?;
    let time = local.time?;
    let naive = local.date.and_time(time);
    Some(
        naive
            .and_local_timezone(offset)
            .single()?
            .with_timezone(&Utc),
    )
}

/// When reminder `r` of `task` fires, if it is a time-based reminder that can fire.
pub fn fire_time(r: &Reminder, task: &Task, offset: FixedOffset) -> Option<DateTime<Utc>> {
    match r.kind.as_str() {
        "absolute" => instant(r.due.as_ref()?.date.as_deref()?, offset),
        "relative" => {
            let due = task.item.due.as_ref()?.date.as_deref()?;
            let at = instant(due, offset)?;
            Some(at - Duration::minutes(r.minute_offset.unwrap_or(0)))
        }
        _ => None,
    }
}

fn key_of(reminder_id: &str, at: DateTime<Utc>) -> String {
    format!("{reminder_id}@{}", at.format("%Y-%m-%dT%H:%M:%SZ"))
}

impl Store {
    /// Whether the account wants desktop reminders: `user_settings.reminder_desktop`, `true`
    /// when the server did not send it.
    pub fn desktop_reminders_enabled(&self) -> Result<bool> {
        let raw = meta_get(self.connection(), "user_settings")?;
        let off = raw
            .and_then(|r| serde_json::from_str::<Value>(&r).ok())
            .and_then(|v| v.get("reminder_desktop").and_then(Value::as_bool))
            == Some(false);
        Ok(!off)
    }

    /// Reminders to show at `now`, and when the next one is due.
    pub fn reminder_schedule(&self, now: &Now) -> Result<Schedule> {
        let user = self.user()?.map(|u| u.id).filter(|id| !id.is_empty());
        let now_utc = now.utc();
        let mut out = Schedule::default();
        for (r, task) in self.all_reminders()? {
            if r.is_deleted {
                continue;
            }
            // Reminders of a shared task can be meant for someone else.
            if let (Some(uid), Some(me)) = (r.notify_uid.as_deref(), user.as_deref())
                && uid != me
            {
                continue;
            }
            let Some(at) = fire_time(&r, &task, now.offset()) else {
                continue;
            };
            if at > now_utc {
                out.next = Some(out.next.map_or(at, |n| n.min(at)));
                continue;
            }
            if now_utc - at > GRACE {
                continue;
            }
            let key = key_of(&r.id, at);
            if self.reminder_fired(&key)? {
                continue;
            }
            out.due.push(DueReminder {
                key,
                reminder_id: r.id.clone(),
                task,
                fire_at: at,
                urgent: r.is_urgent,
            });
        }
        out.due
            .sort_by(|a, b| a.fire_at.cmp(&b.fire_at).then(a.key.cmp(&b.key)));
        Ok(out)
    }

    fn reminder_fired(&self, key: &str) -> Result<bool> {
        let n: i64 = self.connection().query_row(
            "SELECT COUNT(*) FROM fired_reminders WHERE key = ?1",
            [key],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    /// Remember that these firings were shown (they never show again), and forget old ones.
    pub fn mark_reminders_fired(&mut self, fired: &[DueReminder]) -> Result<()> {
        let now = self.now().utc();
        let tx = self.write_tx()?;
        for f in fired {
            tx.execute(
                "INSERT OR IGNORE INTO fired_reminders (key, reminder_id, fire_at, fired_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    f.key,
                    f.reminder_id,
                    f.fire_at.timestamp_millis(),
                    now.timestamp_millis()
                ],
            )?;
        }
        tx.execute(
            "DELETE FROM fired_reminders WHERE fire_at < ?1",
            [(now - KEEP_FIRED).timestamp_millis()],
        )?;
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::model::SyncResponse;

    const NOW: &str = "2026-11-22T09:00:00+00:00";

    fn now() -> Now {
        Now::fixed(NOW).unwrap()
    }

    fn apply(s: &mut Store, full: bool, v: Value) {
        let mut v = v;
        v["full_sync"] = json!(full);
        v["sync_token"] = json!("t");
        let resp: SyncResponse = serde_json::from_value(v).unwrap();
        s.apply_sync_response(&[], &resp).unwrap();
    }

    fn task(id: &str, date: &str, checked: bool) -> Value {
        json!({"id": id, "content": id, "project_id": "p", "checked": checked,
               "is_deleted": false, "child_order": 1,
               "due": {"date": date, "is_recurring": false, "string": date}})
    }

    fn store(reminders: Value) -> Store {
        let mut s = Store::open_in_memory().unwrap();
        s.set_clock(std::sync::Arc::new(now));
        apply(
            &mut s,
            true,
            json!({
                "user": {"id": "me", "full_name": "Me"},
                "projects": [{"id": "p", "name": "P"}],
                "items": [task("t1", "2026-11-22T10:00:00", false), task("t2", "2026-11-22", false)],
                "reminders": reminders,
            }),
        );
        s
    }

    #[test]
    fn absolute_utc_and_floating_dates() {
        let s = store(json!([
            {"id": "r1", "item_id": "t1", "type": "absolute",
             "due": {"date": "2026-11-22T08:50:00.000000Z"}},
            {"id": "r2", "item_id": "t1", "type": "absolute", "due": {"date": "2026-11-22T08:55:00"}},
            {"id": "r3", "item_id": "t1", "type": "absolute", "due": {"date": "2026-11-22T09:30:00"}}
        ]));
        let sch = s.reminder_schedule(&now()).unwrap();
        let ids: Vec<&str> = sch.due.iter().map(|d| d.reminder_id.as_str()).collect();
        assert_eq!(ids, ["r1", "r2"]);
        assert_eq!(
            sch.next.unwrap().to_rfc3339(),
            "2026-11-22T09:30:00+00:00",
            "{sch:?}"
        );
    }

    #[test]
    fn relative_needs_a_due_time() {
        let s = store(json!([
            // 10:00 - 70 min = 08:50: due now.
            {"id": "r1", "item_id": "t1", "type": "relative", "minute_offset": 70},
            // A full-day task has no time: never fires.
            {"id": "r2", "item_id": "t2", "type": "relative", "minute_offset": 30},
            {"id": "r3", "item_id": "t1", "type": "location", "name": "Office"}
        ]));
        let sch = s.reminder_schedule(&now()).unwrap();
        assert_eq!(sch.due.len(), 1, "{sch:?}");
        assert_eq!(sch.due[0].reminder_id, "r1");
        assert_eq!(sch.due[0].fire_at.to_rfc3339(), "2026-11-22T08:50:00+00:00");
        assert_eq!(sch.next, None);
    }

    #[test]
    fn fires_once_and_respects_the_grace_window() {
        let mut s = store(json!([
            {"id": "old", "item_id": "t1", "type": "absolute", "due": {"date": "2026-11-22T07:59:00"}},
            {"id": "edge", "item_id": "t1", "type": "absolute", "due": {"date": "2026-11-22T08:00:00"}}
        ]));
        let sch = s.reminder_schedule(&now()).unwrap();
        assert_eq!(sch.due.len(), 1, "{sch:?}");
        assert_eq!(sch.due[0].reminder_id, "edge");
        s.mark_reminders_fired(&sch.due).unwrap();
        assert!(s.reminder_schedule(&now()).unwrap().due.is_empty());
        // A sync that resends it changes nothing.
        apply(
            &mut s,
            false,
            json!({"reminders": [{"id": "edge", "item_id": "t1", "type": "absolute",
                                  "due": {"date": "2026-11-22T08:00:00"}}]}),
        );
        assert!(s.reminder_schedule(&now()).unwrap().due.is_empty());
        // Moved to a new instant, it fires again.
        apply(
            &mut s,
            false,
            json!({"reminders": [{"id": "edge", "item_id": "t1", "type": "absolute",
                                  "due": {"date": "2026-11-22T08:30:00"}}]}),
        );
        assert_eq!(s.reminder_schedule(&now()).unwrap().due.len(), 1);
    }

    #[test]
    fn skips_other_users_and_completed_tasks() {
        let mut s = store(json!([
            {"id": "r1", "item_id": "t1", "type": "absolute", "notify_uid": "you",
             "due": {"date": "2026-11-22T08:55:00"}},
            {"id": "r2", "item_id": "t1", "type": "absolute", "notify_uid": "me",
             "due": {"date": "2026-11-22T08:55:00"}}
        ]));
        let sch = s.reminder_schedule(&now()).unwrap();
        assert_eq!(sch.due.len(), 1);
        assert_eq!(sch.due[0].reminder_id, "r2");
        apply(
            &mut s,
            false,
            json!({"items": [task("t1", "2026-11-22T10:00:00", true)]}),
        );
        assert!(s.reminder_schedule(&now()).unwrap().due.is_empty());
    }

    #[test]
    fn desktop_reminders_setting() {
        let mut s = store(json!([]));
        assert!(s.desktop_reminders_enabled().unwrap());
        apply(
            &mut s,
            false,
            json!({"user_settings": {"reminder_desktop": false}}),
        );
        assert!(!s.desktop_reminders_enabled().unwrap());
    }
}
