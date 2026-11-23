//! The single source of "now".
//!
//! `TICK_NOW` (RFC 3339) fixes the clock for deterministic tests and screenshots. Its UTC
//! offset is also used as the local offset, so "today" does not depend on the machine's time
//! zone (CI runs in UTC). With `TICK_NOW_ADVANCE=1` that clock runs from `TICK_NOW` on
//! (reminder timers in E2E tests). Without `TICK_NOW` the system clock and local zone are
//! used.

use chrono::{
    DateTime, Duration, FixedOffset, Local, NaiveDate, NaiveDateTime, NaiveTime, Offset, Utc,
};

/// A point in time together with the offset used to interpret floating and UTC due dates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Now {
    pub instant: DateTime<FixedOffset>,
}

impl Now {
    /// Build from a fixed RFC 3339 instant (tests).
    pub fn fixed(rfc3339: &str) -> Option<Now> {
        DateTime::parse_from_rfc3339(rfc3339)
            .ok()
            .map(|instant| Now { instant })
    }

    pub fn from_instant(instant: DateTime<FixedOffset>) -> Now {
        Now { instant }
    }

    /// Wall-clock local date and time.
    pub fn local(&self) -> NaiveDateTime {
        self.instant.naive_local()
    }

    pub fn today(&self) -> NaiveDate {
        self.local().date()
    }

    pub fn offset(&self) -> FixedOffset {
        *self.instant.offset()
    }

    pub fn utc(&self) -> DateTime<Utc> {
        self.instant.with_timezone(&Utc)
    }

    /// The instant formatted like the API's timestamps (`2025-01-21T21:28:43.841504Z`).
    pub fn utc_string(&self) -> String {
        self.utc().format("%Y-%m-%dT%H:%M:%S%.6fZ").to_string()
    }

    /// Return the same clock moved by `d`.
    pub fn plus(&self, d: Duration) -> Now {
        Now {
            instant: self.instant + d,
        }
    }
}

/// When this process first read the clock (`TICK_NOW_ADVANCE`).
static FIRST_READ: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

/// Read the clock, honoring `TICK_NOW` (and `TICK_NOW_ADVANCE=1`: the fixed clock starts at
/// `TICK_NOW` when the process first reads it and then runs, so timers can be tested).
pub fn now() -> Now {
    if let Ok(s) = std::env::var("TICK_NOW")
        && let Some(n) = Now::fixed(s.trim())
    {
        if std::env::var("TICK_NOW_ADVANCE").is_ok_and(|v| v == "1") {
            let elapsed = FIRST_READ.get_or_init(std::time::Instant::now).elapsed();
            return n.plus(Duration::from_std(elapsed).unwrap_or_default());
        }
        return n;
    }
    let local = Local::now();
    Now {
        instant: local.with_timezone(&local.offset().fix()),
    }
}

/// A due date resolved to local wall-clock time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LocalDue {
    pub date: NaiveDate,
    pub time: Option<NaiveTime>,
}

impl LocalDue {
    /// Sort key: full-day dates sort before timed tasks on the same day.
    pub fn sort_key(&self) -> String {
        match self.time {
            Some(t) => format!("{}T{}", self.date.format("%Y-%m-%d"), t.format("%H:%M:%S")),
            None => self.date.format("%Y-%m-%d").to_string(),
        }
    }

    /// True if the due moment is in the past: a full-day date before today, or a timed due
    /// before the current local time.
    pub fn is_overdue(&self, now: &Now) -> bool {
        match self.time {
            None => self.date < now.today(),
            Some(t) => self.date.and_time(t) < now.local(),
        }
    }
}

/// Resolve the `date` field of a due object (`docs/api/pages/due-dates.md`) to local time.
/// Full-day and floating dates are already local; fixed-zone dates end in `Z` and are UTC.
pub fn resolve_due_date(date: &str, offset: FixedOffset) -> Option<LocalDue> {
    let date = date.trim();
    if date.len() == 10 {
        return NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .ok()
            .map(|d| LocalDue {
                date: d,
                time: None,
            });
    }
    if let Some(utc) = date.strip_suffix('Z') {
        let naive = parse_naive_datetime(utc)?;
        let local = naive.and_utc().with_timezone(&offset).naive_local();
        return Some(LocalDue {
            date: local.date(),
            time: Some(local.time()),
        });
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(date) {
        let local = dt.with_timezone(&offset).naive_local();
        return Some(LocalDue {
            date: local.date(),
            time: Some(local.time()),
        });
    }
    let naive = parse_naive_datetime(date)?;
    Some(LocalDue {
        date: naive.date(),
        time: Some(naive.time()),
    })
}

fn parse_naive_datetime(s: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M"))
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_now_uses_its_own_offset() {
        let n = Now::fixed("2026-03-10T23:30:00+08:00").unwrap();
        assert_eq!(n.today(), NaiveDate::from_ymd_opt(2026, 3, 10).unwrap());
        assert_eq!(n.utc_string(), "2026-03-10T15:30:00.000000Z");
    }

    #[test]
    fn resolves_three_due_formats() {
        let off = FixedOffset::east_opt(8 * 3600).unwrap();
        let d = resolve_due_date("2026-03-10", off).unwrap();
        assert_eq!(d.time, None);
        let f = resolve_due_date("2026-03-10T12:00:00.000000", off).unwrap();
        assert_eq!(f.time, NaiveTime::from_hms_opt(12, 0, 0));
        let z = resolve_due_date("2026-03-10T20:00:00.000000Z", off).unwrap();
        assert_eq!(z.date, NaiveDate::from_ymd_opt(2026, 3, 11).unwrap());
        assert_eq!(z.time, NaiveTime::from_hms_opt(4, 0, 0));
        assert_eq!(z.sort_key(), "2026-03-11T04:00:00");
        assert!(resolve_due_date("garbage", off).is_none());
    }

    #[test]
    fn overdue_rules() {
        let n = Now::fixed("2026-03-10T09:00:00+00:00").unwrap();
        let off = n.offset();
        assert!(resolve_due_date("2026-03-09", off).unwrap().is_overdue(&n));
        assert!(!resolve_due_date("2026-03-10", off).unwrap().is_overdue(&n));
        assert!(
            resolve_due_date("2026-03-10T08:00:00", off)
                .unwrap()
                .is_overdue(&n)
        );
        assert!(
            !resolve_due_date("2026-03-10T10:00:00", off)
                .unwrap()
                .is_overdue(&n)
        );
    }
}
