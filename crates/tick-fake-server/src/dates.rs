//! A small date parser for due strings and quick add.
//!
//! Supported: `today`, `tomorrow`, weekday names (next occurrence after today), `next week`,
//! `in N days`, `YYYY-MM-DD`, `every day`, `every <weekday>`, `every week`, `every month`,
//! each optionally followed by `at HH:MM` or `at 5pm`. The real server understands far more;
//! the docs only link to the Help Center for the full grammar.

use chrono::{Datelike, Duration, NaiveDate, NaiveTime, Weekday};
use serde_json::{Value, json};

use crate::State;
use crate::order;

fn weekday(s: &str) -> Option<Weekday> {
    Some(match s {
        "monday" | "mon" => Weekday::Mon,
        "tuesday" | "tue" | "tues" => Weekday::Tue,
        "wednesday" | "wed" => Weekday::Wed,
        "thursday" | "thu" | "thur" | "thurs" => Weekday::Thu,
        "friday" | "fri" => Weekday::Fri,
        "saturday" | "sat" => Weekday::Sat,
        "sunday" | "sun" => Weekday::Sun,
        _ => return None,
    })
}

/// The next `wd` strictly after `from` (a plain weekday name in Todoist means the coming
/// one; whether "monday" on a Monday means today is not documented, we pick next week).
fn next_weekday(from: NaiveDate, wd: Weekday, include_today: bool) -> NaiveDate {
    let mut d = if include_today {
        from
    } else {
        from + Duration::days(1)
    };
    while d.weekday() != wd {
        d += Duration::days(1);
    }
    d
}

fn parse_time(words: &[&str]) -> Option<(NaiveTime, usize)> {
    if words.first() != Some(&"at") {
        return None;
    }
    let t = words.get(1)?;
    let (num, pm, am) = if let Some(n) = t.strip_suffix("pm") {
        (n, true, false)
    } else if let Some(n) = t.strip_suffix("am") {
        (n, false, true)
    } else {
        (*t, false, false)
    };
    let (h, m) = match num.split_once(':') {
        Some((h, m)) => (h.parse::<u32>().ok()?, m.parse::<u32>().ok()?),
        None => (num.parse::<u32>().ok()?, 0),
    };
    let h = match (pm, am, h) {
        (true, _, 12) => 12,
        (true, _, h) => h + 12,
        (_, true, 12) => 0,
        _ => h,
    };
    Some((NaiveTime::from_hms_opt(h, m, 0)?, 2))
}

/// A parsed date phrase: date, optional time, recurring flag, words consumed.
#[derive(Debug, Clone, PartialEq)]
pub struct Parsed {
    pub date: NaiveDate,
    pub time: Option<NaiveTime>,
    pub recurring: bool,
    pub consumed: usize,
}

/// Parse a date phrase at the start of `words` (lower-case).
pub fn parse_phrase(today: NaiveDate, words: &[&str]) -> Option<Parsed> {
    let w0 = *words.first()?;
    let (date, recurring, n) = match w0 {
        "today" | "tod" => (today, false, 1),
        "tomorrow" | "tom" => (today + Duration::days(1), false, 1),
        "next" if words.get(1) == Some(&"week") => {
            (next_weekday(today, Weekday::Mon, false), false, 2)
        }
        "in" => {
            let n: i64 = words.get(1)?.parse().ok()?;
            match *words.get(2)? {
                "day" | "days" => (today + Duration::days(n), false, 3),
                "week" | "weeks" => (today + Duration::days(7 * n), false, 3),
                _ => return None,
            }
        }
        "every" | "daily" | "weekly" | "monthly" => {
            let (kind, n) = if w0 == "every" {
                (*words.get(1)?, 2)
            } else {
                (w0, 1)
            };
            match kind {
                "day" | "daily" => (today, true, n),
                "week" | "weekly" => (today, true, n),
                "month" | "monthly" => (today, true, n),
                other => (next_weekday(today, weekday(other)?, true), true, n),
            }
        }
        other => {
            if let Some(wd) = weekday(other) {
                (next_weekday(today, wd, false), false, 1)
            } else if let Ok(d) = NaiveDate::parse_from_str(other, "%Y-%m-%d") {
                (d, false, 1)
            } else {
                return None;
            }
        }
    };
    let (time, extra) = match parse_time(&words[n..]) {
        Some((t, k)) => (Some(t), k),
        None => (None, 0),
    };
    Some(Parsed {
        date,
        time,
        recurring,
        consumed: n + extra,
    })
}

fn today(st: &State) -> NaiveDate {
    st.now().date_naive()
}

fn format_date(p: &Parsed) -> String {
    match p.time {
        Some(t) => format!("{}T{}", p.date.format("%Y-%m-%d"), t.format("%H:%M:%S")),
        None => p.date.format("%Y-%m-%d").to_string(),
    }
}

/// Normalize a `due` argument object into a full due object
/// (`docs/api/pages/due-dates.md`, "Create or update due dates").
pub fn normalize_due(st: &State, due: &Value) -> Result<Value, String> {
    let string = due["string"].as_str().map(str::to_owned);
    let lang = due["lang"].as_str().unwrap_or("en").to_owned();
    if let Some(date) = due["date"].as_str() {
        let valid = NaiveDate::parse_from_str(date, "%Y-%m-%d").is_ok()
            || chrono::NaiveDateTime::parse_from_str(
                date.trim_end_matches('Z'),
                "%Y-%m-%dT%H:%M:%S%.f",
            )
            .is_ok();
        if !valid {
            return Err(format!("invalid date {date:?}"));
        }
        let recurring = string
            .as_deref()
            .map(|s| s.to_lowercase())
            .is_some_and(|s| s.starts_with("every") || s == "daily" || s == "weekly");
        let timezone = if date.ends_with('Z') {
            due["timezone"]
                .as_str()
                .map(|s| json!(s))
                .unwrap_or_else(|| st.user["tz_info"]["timezone"].clone())
        } else {
            Value::Null
        };
        return Ok(json!({
            "date": date, "timezone": timezone,
            "string": string.unwrap_or_else(|| date.replace('T', " ")),
            "lang": lang, "is_recurring": recurring,
        }));
    }
    let Some(s) = string else {
        return Err("due needs a date or a string".into());
    };
    let lower = s.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();
    match parse_phrase(today(st), &words) {
        Some(p) if p.consumed == words.len() => Ok(json!({
            "date": format_date(&p), "timezone": null, "string": s, "lang": lang,
            "is_recurring": p.recurring,
        })),
        _ => Err(format!("cannot parse due string {s:?}")),
    }
}

/// The next date of a recurring due (used by `item_close`).
pub fn next_occurrence(st: &State, due: &Value) -> Value {
    let mut due = due.clone();
    let date_s = due["date"].as_str().unwrap_or("").to_owned();
    let (date_part, time_part) = match date_s.split_once('T') {
        Some((d, t)) => (d.to_owned(), Some(t.to_owned())),
        None => (date_s.clone(), None),
    };
    let Ok(date) = NaiveDate::parse_from_str(&date_part, "%Y-%m-%d") else {
        return due;
    };
    let s = due["string"].as_str().unwrap_or("").to_lowercase();
    let words: Vec<&str> = s.split_whitespace().collect();
    let kind = match words.as_slice() {
        ["every", k, ..] => *k,
        [k, ..] => *k,
        [] => "day",
    };
    let step = |d: NaiveDate| -> NaiveDate {
        match kind {
            "week" | "weekly" => d + Duration::days(7),
            "month" | "monthly" => d
                .checked_add_months(chrono::Months::new(1))
                .unwrap_or(d + Duration::days(30)),
            other if weekday(other).is_some() => d + Duration::days(7),
            _ => d + Duration::days(1),
        }
    };
    let today = today(st);
    let mut next = step(date);
    while next <= today && next < date + Duration::days(3650) {
        if kind == "day" || kind == "daily" {
            next = today + Duration::days(1);
            break;
        }
        next = step(next);
    }
    let new_date = match time_part {
        Some(t) => format!("{}T{}", next.format("%Y-%m-%d"), t),
        None => next.format("%Y-%m-%d").to_string(),
    };
    due["date"] = json!(new_date);
    due
}

/// Split text into words, honoring `\ ` escapes (`#My\ Project`).
fn words(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(n) = chars.next() {
                    cur.push(n);
                }
            }
            c if c.is_whitespace() => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Match the longest known name (case-insensitive, may contain spaces) starting at
/// `words[i]` (whose sigil is already stripped). Returns (name, words consumed).
fn match_name(first: &str, rest: &[String], names: &[String]) -> Option<(String, usize)> {
    let mut best: Option<(String, usize)> = None;
    for name in names {
        let parts: Vec<&str> = name.split_whitespace().collect();
        if parts.is_empty() || parts.len() > rest.len() + 1 {
            continue;
        }
        let mut candidate = vec![first];
        candidate.extend(rest.iter().take(parts.len() - 1).map(String::as_str));
        if candidate.join(" ").to_lowercase() == name.to_lowercase()
            && best.as_ref().is_none_or(|(_, n)| parts.len() > *n)
        {
            best = Some((name.clone(), parts.len()));
        }
    }
    best
}

/// `POST /api/v1/tasks/quick`: create a task from quick-add text and return it.
pub fn quick_add(st: &mut State, text: &str, note: Option<&str>) -> Value {
    let (main, description) = match text.find(" //") {
        Some(i) => (&text[..i], text[i + 3..].trim().to_owned()),
        None => (text, String::new()),
    };
    let words = words(main);
    let projects: Vec<String> = st
        .live("projects")
        .filter_map(|v| v["name"].as_str().map(str::to_owned))
        .collect();
    let labels: Vec<String> = st
        .live("labels")
        .filter_map(|v| v["name"].as_str().map(str::to_owned))
        .collect();
    let mut content: Vec<String> = Vec::new();
    let mut project: Option<String> = None;
    let mut section_name: Option<String> = None;
    let mut item_labels: Vec<String> = Vec::new();
    let mut priority = 1;
    let mut due = Value::Null;
    let today = today(st);
    let mut i = 0;
    while i < words.len() {
        let w = &words[i];
        let rest = &words[i + 1..];
        if let Some(name) = w.strip_prefix('#').filter(|n| !n.is_empty()) {
            if let Some((matched, n)) = match_name(name, rest, &projects) {
                project = st
                    .live("projects")
                    .find(|v| v["name"].as_str() == Some(matched.as_str()))
                    .and_then(|v| v["id"].as_str())
                    .map(str::to_owned);
                i += n;
                continue;
            }
        } else if let Some(name) = w.strip_prefix('@').filter(|n| !n.is_empty()) {
            let (label, n) = match_name(name, rest, &labels).unwrap_or((name.to_owned(), 1));
            if st.find("labels", "name", &label).is_none() {
                let id = st.new_id();
                let order = st.live("labels").count() + 1;
                st.put(
                    "labels",
                    json!({"id": id, "name": label, "color": "charcoal",
                           "item_order": order, "order_key": order::nth_key(order),
                           "is_deleted": false, "is_favorite": false}),
                );
            }
            if !item_labels.contains(&label) {
                item_labels.push(label);
            }
            i += n;
            continue;
        } else if let Some(name) = w.strip_prefix('/').filter(|n| !n.is_empty()) {
            section_name = Some(name.to_owned());
            i += 1;
            continue;
        } else if let Some(p) = match w.to_lowercase().as_str() {
            "p1" | "!!1" => Some(4),
            "p2" | "!!2" => Some(3),
            "p3" | "!!3" => Some(2),
            "p4" | "!!4" => Some(1),
            _ => None,
        } {
            priority = p;
            i += 1;
            continue;
        } else if due.is_null() {
            let lower: Vec<String> = words[i..].iter().map(|s| s.to_lowercase()).collect();
            let refs: Vec<&str> = lower.iter().map(String::as_str).collect();
            if let Some(p) = parse_phrase(today, &refs) {
                let phrase = words[i..i + p.consumed].join(" ");
                due = json!({"date": format_date(&p), "timezone": null, "string": phrase,
                             "lang": "en", "is_recurring": p.recurring});
                i += p.consumed;
                continue;
            }
        }
        content.push(w.clone());
        i += 1;
    }
    let project = project.unwrap_or_else(|| st.inbox_id());
    // Docs: sections need a project; an unknown section stays in the content.
    let section = section_name.as_ref().and_then(|name| {
        st.live("sections")
            .find(|v| {
                v["project_id"].as_str() == Some(project.as_str())
                    && v["name"]
                        .as_str()
                        .is_some_and(|n| n.eq_ignore_ascii_case(name))
            })
            .and_then(|v| v["id"].as_str())
            .map(str::to_owned)
    });
    if section.is_none()
        && let Some(name) = section_name
    {
        content.push(format!("/{name}"));
    }
    let id = st.new_id();
    let (child_order, order_key) =
        order::next_item_position(st, &project, section.as_deref(), None);
    let me = st.user_id();
    let now = st.now_utc_string();
    let item = json!({
        "id": id, "user_id": me, "project_id": project, "content": content.join(" "),
        "description": description, "priority": priority, "due": due, "deadline": null,
        "parent_id": null, "child_order": child_order, "order_key": order_key,
        "section_id": section, "day_order": -1, "is_collapsed": false, "labels": item_labels,
        "added_by_uid": me, "assigned_by_uid": me, "responsible_uid": null, "checked": false,
        "is_deleted": false, "added_at": now, "updated_at": now, "completed_at": null,
        "duration": null, "note_count": i64::from(note.is_some()),
    });
    st.put("items", item.clone());
    if let Some(n) = note.filter(|n| !n.is_empty()) {
        let nid = st.new_id();
        st.put(
            "notes",
            json!({"id": nid, "posted_uid": me, "item_id": id, "content": n,
                   "file_attachment": null, "uids_to_notify": null, "is_deleted": false,
                   "posted_at": now, "reactions": null}),
        );
    }
    item
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn phrases() {
        // 2026-03-10 is a Tuesday.
        let t = d("2026-03-10");
        let p = |s: &str| {
            let w: Vec<&str> = s.split_whitespace().collect();
            parse_phrase(t, &w).unwrap()
        };
        assert_eq!(p("today").date, t);
        assert_eq!(p("tomorrow").date, d("2026-03-11"));
        assert_eq!(p("monday").date, d("2026-03-16"));
        assert_eq!(p("tuesday").date, d("2026-03-17"));
        assert_eq!(p("fri").date, d("2026-03-13"));
        assert!(p("every day").recurring);
        assert_eq!(p("every tuesday").date, t);
        assert_eq!(p("every friday").date, d("2026-03-13"));
        assert_eq!(p("in 3 days").date, d("2026-03-13"));
        assert_eq!(p("next week").date, d("2026-03-16"));
        let at = p("tomorrow at 5pm");
        assert_eq!(at.time, NaiveTime::from_hms_opt(17, 0, 0));
        assert_eq!(at.consumed, 3);
        assert!(parse_phrase(t, &["someday"]).is_none());
    }

    #[test]
    fn quick_add_parses_tokens() {
        let mut st = State::new("t");
        st.now = chrono::DateTime::parse_from_rfc3339("2026-03-10T09:00:00+00:00").ok();
        st.put(
            "projects",
            json!({"id": "pw", "name": "Side Project", "is_deleted": false}),
        );
        let item = quick_add(
            &mut st,
            "Buy milk tomorrow #Side Project @errand p1 // 2 liters",
            None,
        );
        assert_eq!(item["content"], "Buy milk");
        assert_eq!(item["project_id"], "pw");
        assert_eq!(item["priority"], 4);
        assert_eq!(item["labels"], json!(["errand"]));
        assert_eq!(item["due"]["date"], "2026-03-11");
        assert_eq!(item["description"], "2 liters");
        assert!(st.find("labels", "name", "errand").is_some());
        let item = quick_add(&mut st, "Water plants every day #Nope", None);
        assert_eq!(item["content"], "Water plants #Nope");
        assert_eq!(item["due"]["is_recurring"], true);
        assert_eq!(item["project_id"], st.inbox_id());
    }
}
