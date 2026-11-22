//! A minimal filter evaluator for `GET /api/v1/tasks/filter`.
//!
//! Supports `&`, `|`, `!`, parentheses, `today`, `tomorrow`, `overdue`, `no date`, `p1`-`p4`,
//! `#project`, `@label` (both with `*` wildcards) and `due before: YYYY-MM-DD`. Everything
//! else, and comma-separated queries (the docs say "Multiple filters (using the comma `,`
//! operator) are not supported"), is rejected with `400`.

use chrono::NaiveDate;
use serde_json::Value;

use crate::State;

#[derive(Debug)]
enum E {
    And(Box<E>, Box<E>),
    Or(Box<E>, Box<E>),
    Not(Box<E>),
    Atom(String),
}

fn tokenize(q: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in q.chars() {
        if matches!(c, '(' | ')' | '&' | '|') || (c == '!' && cur.trim().is_empty()) {
            if !cur.trim().is_empty() {
                out.push(cur.trim().to_owned());
            }
            cur.clear();
            out.push(c.to_string());
        } else {
            cur.push(c);
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_owned());
    }
    out
}

struct P {
    t: Vec<String>,
    i: usize,
}

impl P {
    fn peek(&self) -> Option<&str> {
        self.t.get(self.i).map(String::as_str)
    }
    fn or(&mut self) -> Result<E, String> {
        let mut l = self.and()?;
        while self.peek() == Some("|") {
            self.i += 1;
            l = E::Or(Box::new(l), Box::new(self.and()?));
        }
        Ok(l)
    }
    fn and(&mut self) -> Result<E, String> {
        let mut l = self.un()?;
        while self.peek() == Some("&") {
            self.i += 1;
            l = E::And(Box::new(l), Box::new(self.un()?));
        }
        Ok(l)
    }
    fn un(&mut self) -> Result<E, String> {
        let t = self.t.get(self.i).cloned().ok_or("unexpected end")?;
        self.i += 1;
        match t.as_str() {
            "!" => Ok(E::Not(Box::new(self.un()?))),
            "(" => {
                let e = self.or()?;
                if self.peek() != Some(")") {
                    return Err("missing )".into());
                }
                self.i += 1;
                Ok(e)
            }
            ")" | "&" | "|" => Err(format!("unexpected {t}")),
            _ => {
                check_atom(&t)?;
                Ok(E::Atom(t))
            }
        }
    }
}

fn check_atom(a: &str) -> Result<(), String> {
    let l = a.to_lowercase();
    let ok = matches!(
        l.as_str(),
        "today" | "tomorrow" | "overdue" | "no date" | "p1" | "p2" | "p3" | "p4"
    ) || (l.starts_with('#') && l.len() > 1)
        || (l.starts_with('@') && l.len() > 1)
        || l.strip_prefix("due before:")
            .is_some_and(|d| NaiveDate::parse_from_str(d.trim(), "%Y-%m-%d").is_ok());
    if ok {
        Ok(())
    } else {
        Err(format!("unsupported filter: {a}"))
    }
}

fn wildcard(pattern: &str, s: &str) -> bool {
    fn glob(p: &[char], s: &[char]) -> bool {
        match p.first() {
            None => s.is_empty(),
            Some('*') => (0..=s.len()).any(|i| glob(&p[1..], &s[i..])),
            Some(c) => s.first() == Some(c) && glob(&p[1..], &s[1..]),
        }
    }
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let s: Vec<char> = s.to_lowercase().chars().collect();
    glob(&p, &s)
}

fn due_date(v: &Value) -> Option<NaiveDate> {
    let d = v["due"]["date"].as_str()?;
    NaiveDate::parse_from_str(&d[..10.min(d.len())], "%Y-%m-%d").ok()
}

fn eval(st: &State, e: &E, v: &Value, today: NaiveDate) -> bool {
    match e {
        E::And(a, b) => eval(st, a, v, today) && eval(st, b, v, today),
        E::Or(a, b) => eval(st, a, v, today) || eval(st, b, v, today),
        E::Not(a) => !eval(st, a, v, today),
        E::Atom(a) => {
            let l = a.to_lowercase();
            match l.as_str() {
                "today" => due_date(v) == Some(today),
                "tomorrow" => due_date(v) == Some(today + chrono::Duration::days(1)),
                "overdue" => due_date(v).is_some_and(|d| d < today),
                "no date" => v["due"].is_null(),
                "p1" => v["priority"] == 4,
                "p2" => v["priority"] == 3,
                "p3" => v["priority"] == 2,
                "p4" => v["priority"] == 1,
                _ => {
                    if let Some(name) = a.strip_prefix('#') {
                        let pid = v["project_id"].as_str().unwrap_or("");
                        st.get("projects", pid)
                            .and_then(|p| p["name"].as_str())
                            .is_some_and(|n| wildcard(name.trim(), n))
                    } else if let Some(name) = a.strip_prefix('@') {
                        v["labels"].as_array().is_some_and(|ls| {
                            ls.iter()
                                .filter_map(Value::as_str)
                                .any(|l| wildcard(name.trim(), l))
                        })
                    } else if let Some(d) = l.strip_prefix("due before:") {
                        let limit = NaiveDate::parse_from_str(d.trim(), "%Y-%m-%d").ok();
                        matches!((due_date(v), limit), (Some(x), Some(y)) if x < y)
                    } else {
                        false
                    }
                }
            }
        }
    }
}

/// Active tasks matching `query`, ordered by due date then child order.
pub fn run(st: &State, query: &str) -> Result<Vec<Value>, String> {
    if query.contains(',') {
        return Err("Multiple filters (using the comma operator) are not supported".into());
    }
    let mut p = P {
        t: tokenize(query),
        i: 0,
    };
    let e = p.or()?;
    if p.i != p.t.len() {
        return Err("unexpected trailing input".into());
    }
    let today = st.now().date_naive();
    let mut hits: Vec<Value> = st
        .active_items()
        .into_iter()
        .filter(|v| eval(st, &e, v, today))
        .cloned()
        .collect();
    hits.sort_by(|a, b| {
        a["due"]["date"]
            .as_str()
            .unwrap_or("9999")
            .cmp(b["due"]["date"].as_str().unwrap_or("9999"))
            .then(a["child_order"].as_i64().cmp(&b["child_order"].as_i64()))
    });
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::wildcard;

    #[test]
    fn wildcards() {
        assert!(wildcard("home*", "Home office"));
        assert!(wildcard("*office", "home office"));
        assert!(wildcard("h*e", "house"));
        assert!(!wildcard("home*", "work"));
        assert!(wildcard("work", "Work"));
    }
}
