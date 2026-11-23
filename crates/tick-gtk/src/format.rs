//! Dates, times and counts as `DESIGN.md` 6.3 shows them. No GTK here, so it is unit-tested.
//!
//! Every word goes through gettext and every layout is a translatable template with named
//! placeholders, so Chinese can write `11 月 28 日` from the same data.

use chrono::{Datelike, NaiveDate, NaiveTime, Timelike, Weekday};

use crate::i18n::{fill, gettext, pgettext};

/// The user's Todoist display settings (`date_format`, `time_format` of the user object).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DateSettings {
    /// `date_format = 1`: "Nov 28" instead of "28 Nov".
    pub month_first: bool,
    /// `time_format = 1`: "2:00 PM" instead of "14:00".
    pub twelve_hour: bool,
}

impl DateSettings {
    pub fn from_user(user: Option<&tick_core::model::User>) -> DateSettings {
        let flag = |key: &str| {
            user.and_then(|u| u.extra.get(key))
                .and_then(serde_json::Value::as_i64)
                == Some(1)
        };
        DateSettings {
            month_first: flag("date_format"),
            twelve_hour: flag("time_format"),
        }
    }
}

pub fn weekday_short(d: Weekday) -> String {
    match d {
        Weekday::Mon => pgettext("weekday", "Mon"),
        Weekday::Tue => pgettext("weekday", "Tue"),
        Weekday::Wed => pgettext("weekday", "Wed"),
        Weekday::Thu => pgettext("weekday", "Thu"),
        Weekday::Fri => pgettext("weekday", "Fri"),
        Weekday::Sat => pgettext("weekday", "Sat"),
        Weekday::Sun => pgettext("weekday", "Sun"),
    }
}

pub fn weekday_long(d: Weekday) -> String {
    match d {
        Weekday::Mon => pgettext("weekday", "Monday"),
        Weekday::Tue => pgettext("weekday", "Tuesday"),
        Weekday::Wed => pgettext("weekday", "Wednesday"),
        Weekday::Thu => pgettext("weekday", "Thursday"),
        Weekday::Fri => pgettext("weekday", "Friday"),
        Weekday::Sat => pgettext("weekday", "Saturday"),
        Weekday::Sun => pgettext("weekday", "Sunday"),
    }
}

fn month_short(m: u32) -> String {
    match m {
        1 => pgettext("month", "Jan"),
        2 => pgettext("month", "Feb"),
        3 => pgettext("month", "Mar"),
        4 => pgettext("month", "Apr"),
        5 => pgettext("month", "May"),
        6 => pgettext("month", "Jun"),
        7 => pgettext("month", "Jul"),
        8 => pgettext("month", "Aug"),
        9 => pgettext("month", "Sep"),
        10 => pgettext("month", "Oct"),
        11 => pgettext("month", "Nov"),
        _ => pgettext("month", "Dec"),
    }
}

fn month_long(m: u32) -> String {
    match m {
        1 => pgettext("month", "January"),
        2 => pgettext("month", "February"),
        3 => pgettext("month", "March"),
        4 => pgettext("month", "April"),
        5 => pgettext("month", "May"),
        6 => pgettext("month", "June"),
        7 => pgettext("month", "July"),
        8 => pgettext("month", "August"),
        9 => pgettext("month", "September"),
        10 => pgettext("month", "October"),
        11 => pgettext("month", "November"),
        _ => pgettext("month", "December"),
    }
}

fn date_args(date: NaiveDate) -> [(&'static str, String); 7] {
    [
        ("day", date.day().to_string()),
        ("mon", month_short(date.month())),
        ("month", date.month().to_string()),
        ("month_name", month_long(date.month())),
        ("year", date.year().to_string()),
        ("weekday", weekday_long(date.weekday())),
        ("wd", weekday_short(date.weekday())),
    ]
}

fn fill_date(template: &str, date: NaiveDate) -> String {
    let args = date_args(date);
    let refs: Vec<(&str, &str)> = args.iter().map(|(k, v)| (*k, v.as_str())).collect();
    fill(template, &refs)
}

/// A calendar date without "Today"/"Tomorrow" ("28 Nov", "Nov 28, 2027").
pub fn date_plain(date: NaiveDate, today: NaiveDate, s: DateSettings) -> String {
    let template = match (date.year() == today.year(), s.month_first) {
        // Translators: a date in the current year, day first. {day} = 28, {mon} = Nov,
        // {month} = 11. Chinese: "{month} 月 {day} 日".
        (true, false) => pgettext("date", "{day} {mon}"),
        // Translators: a date in the current year, month first (user setting).
        (true, true) => pgettext("date", "{mon} {day}"),
        // Translators: a date in another year, day first.
        (false, false) => pgettext("date", "{day} {mon} {year}"),
        // Translators: a date in another year, month first (user setting).
        (false, true) => pgettext("date", "{mon} {day}, {year}"),
    };
    fill_date(&template, date)
}

/// A date relative to today: "Today", "Tomorrow", "Yesterday", a short weekday within the
/// next 6 days, otherwise [`date_plain`] (6.3).
pub fn date_short(date: NaiveDate, today: NaiveDate, s: DateSettings) -> String {
    match (date - today).num_days() {
        0 => gettext("Today"),
        1 => gettext("Tomorrow"),
        -1 => gettext("Yesterday"),
        2..=6 => weekday_short(date.weekday()),
        _ => date_plain(date, today, s),
    }
}

/// A time of day: "14:00" or "2:00 PM".
pub fn time(t: NaiveTime, s: DateSettings) -> String {
    let minute = format!("{:02}", t.minute());
    if s.twelve_hour {
        let hour = match t.hour() % 12 {
            0 => 12,
            h => h,
        };
        let template = if t.hour() < 12 {
            // Translators: 12-hour time before noon. Chinese: "上午 {hour}:{minute}".
            pgettext("time", "{hour}:{minute} AM")
        } else {
            // Translators: 12-hour time after noon. Chinese: "下午 {hour}:{minute}".
            pgettext("time", "{hour}:{minute} PM")
        };
        fill(
            &template,
            &[("hour", &hour.to_string()), ("minute", &minute)],
        )
    } else {
        format!("{:02}:{minute}", t.hour())
    }
}

/// Due text for the meta line. `time_only` drops the date part (Today and Upcoming views);
/// then a full-day date has no text.
pub fn due(
    date: NaiveDate,
    t: Option<NaiveTime>,
    today: NaiveDate,
    s: DateSettings,
    time_only: bool,
) -> Option<String> {
    match (time_only, t) {
        (true, None) => None,
        (true, Some(t)) => Some(time(t, s)),
        (false, None) => Some(date_short(date, today, s)),
        (false, Some(t)) => Some(fill(
            // Translators: a due date with a time, for example "Tomorrow 14:00".
            &pgettext("due", "{date} {time}"),
            &[("date", &date_short(date, today, s)), ("time", &time(t, s))],
        )),
    }
}

/// The Today view subtitle: "Sunday 22 November" / "Sunday, November 22".
pub fn long_date(date: NaiveDate, s: DateSettings) -> String {
    let template = if s.month_first {
        // Translators: the Today view subtitle, month first. Chinese: "{month} 月 {day} 日 {wd}".
        pgettext("date", "{weekday}, {month_name} {day}")
    } else {
        // Translators: the Today view subtitle, day first. Chinese: "{month} 月 {day} 日 {wd}".
        pgettext("date", "{weekday} {day} {month_name}")
    };
    fill_date(&template, date)
}

/// Upcoming month header: "December", with the year when it is not the current year.
pub fn month_header(date: NaiveDate, today: NaiveDate) -> String {
    if date.year() == today.year() {
        // Translators: Upcoming month header. Chinese: "{month} 月".
        fill_date(&pgettext("date", "{month_name}"), date)
    } else {
        // Translators: Upcoming month header in another year. Chinese: "{year} 年 {month} 月".
        fill_date(&pgettext("date", "{month_name} {year}"), date)
    }
}

/// Month and year, as in the date picker's calendar heading: "November 2026", "2026 年 11 月".
pub fn month_year(date: NaiveDate) -> String {
    fill_date(&pgettext("date", "{month_name} {year}"), date)
}

/// `HH:MM` of a UTC timestamp in the clock's offset (filter source line).
pub fn local_time_of(utc: &str, now: &tick_core::Now, s: DateSettings) -> Option<String> {
    let t = chrono::DateTime::parse_from_rfc3339(utc).ok()?;
    Some(time(t.with_timezone(&now.offset()).time(), s))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn relative_and_absolute_dates() {
        let today = d(2026, 11, 22);
        let s = DateSettings::default();
        assert_eq!(date_short(today, today, s), "Today");
        assert_eq!(date_short(d(2026, 11, 23), today, s), "Tomorrow");
        assert_eq!(date_short(d(2026, 11, 21), today, s), "Yesterday");
        assert_eq!(date_short(d(2026, 11, 25), today, s), "Wed");
        assert_eq!(date_short(d(2026, 11, 28), today, s), "Sat");
        assert_eq!(date_short(d(2026, 11, 29), today, s), "29 Nov");
        assert_eq!(date_short(d(2027, 11, 28), today, s), "28 Nov 2027");
        let mf = DateSettings {
            month_first: true,
            twelve_hour: true,
        };
        assert_eq!(date_short(d(2026, 11, 29), today, mf), "Nov 29");
        assert_eq!(date_short(d(2027, 11, 28), today, mf), "Nov 28, 2027");
    }

    #[test]
    fn times_and_due_text() {
        let today = d(2026, 11, 22);
        let t = NaiveTime::from_hms_opt(14, 0, 0);
        let s = DateSettings::default();
        assert_eq!(
            due(d(2026, 11, 23), t, today, s, false).unwrap(),
            "Tomorrow 14:00"
        );
        assert_eq!(due(today, t, today, s, true).unwrap(), "14:00");
        assert_eq!(due(today, None, today, s, true), None);
        let h12 = DateSettings {
            month_first: false,
            twelve_hour: true,
        };
        assert_eq!(time(t.unwrap(), h12), "2:00 PM");
        assert_eq!(
            time(NaiveTime::from_hms_opt(0, 5, 0).unwrap(), h12),
            "12:05 AM"
        );
    }

    #[test]
    fn long_dates_and_months() {
        let today = d(2026, 11, 22);
        let s = DateSettings::default();
        assert_eq!(long_date(today, s), "Sunday 22 November");
        let mf = DateSettings {
            month_first: true,
            twelve_hour: false,
        };
        assert_eq!(long_date(today, mf), "Sunday, November 22");
        assert_eq!(month_header(d(2026, 12, 1), today), "December");
        assert_eq!(month_header(d(2027, 1, 1), today), "January 2027");
    }
}
