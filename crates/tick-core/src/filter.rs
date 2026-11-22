//! Local filter language: parser and evaluator for a subset of Todoist filter queries.
//!
//! The docs in `docs/api/` only link to the Help Center for filter syntax, so the semantics
//! below are Tick's reading of Todoist's documented behavior; each construct has a test.
//!
//! Supported (case-insensitive):
//! - operators `&` (and), `|` (or), `!` (not), parentheses; `&` binds tighter than `|`,
//! - `,` at the top level separates independent queries, shown as separate groups,
//! - `#Project` (exact name), `##Project` (the project and its sub-projects),
//! - `/Section` (section name in any project),
//! - `@label`, `no labels`,
//! - `p1`..`p4` and `priority 1`..`priority 4`, `no priority` (= p4),
//! - `today`, `tomorrow`, `overdue` (also `od`), `no date`, `recurring`,
//! - `next N days` / `N days`: due from today through today + N - 1,
//! - `search: text` (substring of the title, case-insensitive),
//! - `assigned to: me|others|<name>`, `assigned`.
//!
//! Anything else (dates like `due before: ...`, wildcards `*`, `created:`, ...) makes its
//! group [`Group::Unsupported`], so the UI can ask the server (`GET /api/v1/tasks/filter`)
//! instead of showing a silently wrong list.

use std::collections::HashMap;

use chrono::Duration;

use crate::clock::{LocalDue, Now};
use crate::model::Item;

#[derive(Debug, Clone, PartialEq)]
pub enum Atom {
    Project {
        name: String,
        with_subprojects: bool,
    },
    Section(String),
    Label(String),
    NoLabels,
    /// Client priority number: 1 = urgent.
    Priority(u8),
    Today,
    Tomorrow,
    Overdue,
    NoDate,
    Recurring,
    /// Due from today through today + N - 1.
    NextDays(u32),
    Search(String),
    AssignedToMe,
    AssignedToOthers,
    AssignedTo(String),
    Assigned,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Atom(Atom),
    Not(Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
}

/// One comma-separated part of a query.
#[derive(Debug, Clone, PartialEq)]
pub enum Group {
    Supported { text: String, expr: Expr },
    Unsupported { text: String, reason: String },
}

impl Group {
    pub fn text(&self) -> &str {
        match self {
            Group::Supported { text, .. } | Group::Unsupported { text, .. } => text,
        }
    }
}

/// Parse a full query into its comma-separated groups.
pub fn parse(query: &str) -> Vec<Group> {
    split_groups(query)
        .into_iter()
        .map(|text| match parse_group(&text) {
            Ok(expr) => Group::Supported { text, expr },
            Err(reason) => Group::Unsupported { text, reason },
        })
        .collect()
}

fn split_groups(query: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    let mut chars = query.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                cur.push(c);
                if let Some(n) = chars.next() {
                    cur.push(n);
                }
            }
            '(' => {
                depth += 1;
                cur.push(c);
            }
            ')' => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth == 0 => {
                out.push(cur.trim().to_owned());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    out.push(cur.trim().to_owned());
    out
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    LParen,
    RParen,
    And,
    Or,
    Not,
    Word(String),
}

fn tokenize(s: &str) -> Result<Vec<Tok>, String> {
    let mut toks = Vec::new();
    let mut word = String::new();
    let flush = |word: &mut String, toks: &mut Vec<Tok>| {
        let w = word.trim();
        if !w.is_empty() {
            toks.push(Tok::Word(w.to_owned()));
        }
        word.clear();
    };
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some(n) => word.push(n),
                None => return Err("dangling escape".into()),
            },
            '(' | ')' | '&' | '|' => {
                flush(&mut word, &mut toks);
                toks.push(match c {
                    '(' => Tok::LParen,
                    ')' => Tok::RParen,
                    '&' => Tok::And,
                    _ => Tok::Or,
                });
            }
            '!' if word.trim().is_empty() => {
                word.clear();
                toks.push(Tok::Not);
            }
            ',' => return Err("comma inside parentheses".into()),
            _ => word.push(c),
        }
    }
    flush(&mut word, &mut toks);
    Ok(toks)
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }
    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).cloned();
        self.pos += 1;
        t
    }
    fn or(&mut self) -> Result<Expr, String> {
        let mut lhs = self.and()?;
        while self.peek() == Some(&Tok::Or) {
            self.pos += 1;
            let rhs = self.and()?;
            lhs = Expr::Or(Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }
    fn and(&mut self) -> Result<Expr, String> {
        let mut lhs = self.unary()?;
        while self.peek() == Some(&Tok::And) {
            self.pos += 1;
            let rhs = self.unary()?;
            lhs = Expr::And(Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }
    fn unary(&mut self) -> Result<Expr, String> {
        match self.next() {
            Some(Tok::Not) => Ok(Expr::Not(Box::new(self.unary()?))),
            Some(Tok::LParen) => {
                let e = self.or()?;
                match self.next() {
                    Some(Tok::RParen) => Ok(e),
                    _ => Err("missing closing parenthesis".into()),
                }
            }
            Some(Tok::Word(w)) => Ok(Expr::Atom(parse_atom(&w)?)),
            Some(t) => Err(format!("unexpected {t:?}")),
            None => Err("unexpected end of query".into()),
        }
    }
}

fn parse_group(text: &str) -> Result<Expr, String> {
    if text.trim().is_empty() {
        return Err("empty query".into());
    }
    let toks = tokenize(text)?;
    let mut p = Parser { toks, pos: 0 };
    let e = p.or()?;
    if p.pos < p.toks.len() {
        return Err(format!("unexpected {:?}", p.toks[p.pos]));
    }
    Ok(e)
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// ASCII case-insensitive `strip_prefix` that slices the original string (lowercasing can
/// change byte lengths, for example `İ`).
fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    let n = prefix.len();
    (s.len() >= n && s.is_char_boundary(n) && s[..n].eq_ignore_ascii_case(prefix)).then(|| &s[n..])
}

fn parse_atom(word: &str) -> Result<Atom, String> {
    let w = collapse_ws(word);
    let lower = w.to_lowercase();
    let unsupported = || Err(format!("unsupported filter: {w}"));
    if w.contains('*') {
        return unsupported();
    }
    if let Some(rest) = w.strip_prefix("##") {
        let name = rest.trim();
        if name.is_empty() {
            return unsupported();
        }
        return Ok(Atom::Project {
            name: name.to_owned(),
            with_subprojects: true,
        });
    }
    if let Some(rest) = w.strip_prefix('#') {
        let name = rest.trim();
        if name.is_empty() {
            return unsupported();
        }
        return Ok(Atom::Project {
            name: name.to_owned(),
            with_subprojects: false,
        });
    }
    if let Some(rest) = w.strip_prefix('@') {
        let name = rest.trim();
        if name.is_empty() {
            return unsupported();
        }
        return Ok(Atom::Label(name.to_owned()));
    }
    if let Some(rest) = w.strip_prefix('/') {
        let name = rest.trim();
        if name.is_empty() {
            return unsupported();
        }
        return Ok(Atom::Section(name.to_owned()));
    }
    if let Some(rest) = strip_prefix_ci(&w, "search:") {
        let text = rest.trim();
        if text.is_empty() {
            return unsupported();
        }
        return Ok(Atom::Search(text.to_owned()));
    }
    if let Some(rest) = strip_prefix_ci(&w, "assigned to") {
        let who = rest.trim().trim_start_matches(':').trim();
        return match who.to_lowercase().as_str() {
            "" => unsupported(),
            "me" => Ok(Atom::AssignedToMe),
            "others" => Ok(Atom::AssignedToOthers),
            _ => Ok(Atom::AssignedTo(who.to_owned())),
        };
    }
    match lower.as_str() {
        "p1" | "priority 1" => return Ok(Atom::Priority(1)),
        "p2" | "priority 2" => return Ok(Atom::Priority(2)),
        "p3" | "priority 3" => return Ok(Atom::Priority(3)),
        "p4" | "priority 4" | "no priority" => return Ok(Atom::Priority(4)),
        "today" | "tod" => return Ok(Atom::Today),
        "tomorrow" | "tom" => return Ok(Atom::Tomorrow),
        "overdue" | "od" => return Ok(Atom::Overdue),
        "no date" | "no due date" => return Ok(Atom::NoDate),
        "no labels" | "no label" => return Ok(Atom::NoLabels),
        "recurring" => return Ok(Atom::Recurring),
        "assigned" => return Ok(Atom::Assigned),
        _ => {}
    }
    let days = lower.strip_prefix("next ").unwrap_or(&lower);
    if let Some(n) = days
        .strip_suffix(" days")
        .or_else(|| days.strip_suffix(" day"))
        .and_then(|n| n.trim().parse::<u32>().ok())
    {
        return Ok(Atom::NextDays(n));
    }
    unsupported()
}

/// What the evaluator needs to know about projects, sections and people.
#[derive(Debug, Clone, Default)]
pub struct EvalContext {
    /// Project id -> (lower-case name, parent id).
    pub projects: HashMap<String, (String, Option<String>)>,
    /// Section id -> lower-case name.
    pub sections: HashMap<String, String>,
    /// User id -> lower-case full name (collaborators).
    pub people: HashMap<String, String>,
    pub me: Option<String>,
}

/// A task as seen by the evaluator.
#[derive(Debug, Clone, Copy)]
pub struct EvalTask<'a> {
    pub item: &'a Item,
    pub due: Option<LocalDue>,
}

pub fn eval(expr: &Expr, task: &EvalTask<'_>, ctx: &EvalContext, now: &Now) -> bool {
    match expr {
        Expr::Not(e) => !eval(e, task, ctx, now),
        Expr::And(a, b) => eval(a, task, ctx, now) && eval(b, task, ctx, now),
        Expr::Or(a, b) => eval(a, task, ctx, now) || eval(b, task, ctx, now),
        Expr::Atom(a) => eval_atom(a, task, ctx, now),
    }
}

fn eval_atom(atom: &Atom, t: &EvalTask<'_>, ctx: &EvalContext, now: &Now) -> bool {
    let item = t.item;
    let today = now.today();
    match atom {
        Atom::Project {
            name,
            with_subprojects,
        } => {
            let name = name.to_lowercase();
            let mut cur = Some(item.project_id.clone());
            let mut guard = 0;
            while let Some(pid) = cur {
                let Some((pname, parent)) = ctx.projects.get(&pid) else {
                    return false;
                };
                if *pname == name {
                    return true;
                }
                if !with_subprojects || guard > 64 {
                    return false;
                }
                guard += 1;
                cur = parent.clone();
            }
            false
        }
        Atom::Section(name) => item
            .section_id
            .as_ref()
            .and_then(|s| ctx.sections.get(s))
            .is_some_and(|s| *s == name.to_lowercase()),
        Atom::Label(name) => item
            .labels
            .iter()
            .any(|l| l.eq_ignore_ascii_case(name) || l.to_lowercase() == name.to_lowercase()),
        Atom::NoLabels => item.labels.is_empty(),
        Atom::Priority(p) => item.p() == *p,
        Atom::Today => t.due.is_some_and(|d| d.date == today),
        Atom::Tomorrow => t.due.is_some_and(|d| d.date == today + Duration::days(1)),
        Atom::Overdue => t.due.is_some_and(|d| d.is_overdue(now)),
        Atom::NoDate => item.due.is_none(),
        Atom::Recurring => item.due.as_ref().is_some_and(|d| d.is_recurring),
        Atom::NextDays(n) => t
            .due
            .is_some_and(|d| d.date >= today && d.date < today + Duration::days(i64::from(*n))),
        Atom::Search(text) => {
            let text = text.to_lowercase();
            item.content.to_lowercase().contains(&text)
        }
        Atom::AssignedToMe => item.responsible_uid.is_some() && item.responsible_uid == ctx.me,
        Atom::AssignedToOthers => item.responsible_uid.is_some() && item.responsible_uid != ctx.me,
        Atom::AssignedTo(name) => {
            let name = name.to_lowercase();
            item.responsible_uid
                .as_ref()
                .and_then(|u| ctx.people.get(u))
                .is_some_and(|full| *full == name || full.split_whitespace().next() == Some(&name))
        }
        Atom::Assigned => item.responsible_uid.is_some(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::resolve_due_date;
    use serde_json::json;

    fn atom(q: &str) -> Atom {
        match parse(q).remove(0) {
            Group::Supported {
                expr: Expr::Atom(a),
                ..
            } => a,
            other => panic!("{other:?}"),
        }
    }

    fn unsupported(q: &str) -> bool {
        parse(q)
            .iter()
            .any(|g| matches!(g, Group::Unsupported { .. }))
    }

    #[test]
    fn atoms() {
        assert_eq!(
            atom("#Work"),
            Atom::Project {
                name: "Work".into(),
                with_subprojects: false
            }
        );
        assert_eq!(
            atom("##Work"),
            Atom::Project {
                name: "Work".into(),
                with_subprojects: true
            }
        );
        assert_eq!(
            atom("#Shopping  list"),
            Atom::Project {
                name: "Shopping list".into(),
                with_subprojects: false
            }
        );
        assert_eq!(atom("@home"), Atom::Label("home".into()));
        assert_eq!(atom("/Meetings"), Atom::Section("Meetings".into()));
        assert_eq!(atom("P1"), Atom::Priority(1));
        assert_eq!(atom("priority 3"), Atom::Priority(3));
        assert_eq!(atom("no priority"), Atom::Priority(4));
        assert_eq!(atom("Today"), Atom::Today);
        assert_eq!(atom("tod"), Atom::Today);
        assert_eq!(atom("tomorrow"), Atom::Tomorrow);
        assert_eq!(atom("od"), Atom::Overdue);
        assert_eq!(atom("overdue"), Atom::Overdue);
        assert_eq!(atom("no date"), Atom::NoDate);
        assert_eq!(atom("no labels"), Atom::NoLabels);
        assert_eq!(atom("recurring"), Atom::Recurring);
        assert_eq!(atom("next 7 days"), Atom::NextDays(7));
        assert_eq!(atom("3 days"), Atom::NextDays(3));
        assert_eq!(atom("search: Buy milk"), Atom::Search("Buy milk".into()));
        assert_eq!(atom("Search:meeting"), Atom::Search("meeting".into()));
        // Lowercasing changes the byte length of some characters; slicing must not panic.
        assert_eq!(
            atom("search: İstanbul trip"),
            Atom::Search("İstanbul trip".into())
        );
        assert!(unsupported("İİİİ search: x"));
        assert_eq!(atom("assigned to: İlker"), Atom::AssignedTo("İlker".into()));
        assert_eq!(atom("assigned to: me"), Atom::AssignedToMe);
        assert_eq!(atom("assigned to me"), Atom::AssignedToMe);
        assert_eq!(atom("assigned to: others"), Atom::AssignedToOthers);
        assert_eq!(atom("assigned to: Alice"), Atom::AssignedTo("Alice".into()));
        assert_eq!(atom("assigned"), Atom::Assigned);
        assert_eq!(
            atom("#Q\\&A"),
            Atom::Project {
                name: "Q&A".into(),
                with_subprojects: false
            }
        );
    }

    #[test]
    fn precedence_and_structure() {
        let g = parse("today | overdue & p1");
        let Group::Supported { expr, .. } = &g[0] else {
            panic!()
        };
        assert_eq!(
            *expr,
            Expr::Or(
                Box::new(Expr::Atom(Atom::Today)),
                Box::new(Expr::And(
                    Box::new(Expr::Atom(Atom::Overdue)),
                    Box::new(Expr::Atom(Atom::Priority(1)))
                ))
            )
        );
        let g = parse("(today | overdue) & !@waiting");
        let Group::Supported { expr, .. } = &g[0] else {
            panic!()
        };
        assert_eq!(
            *expr,
            Expr::And(
                Box::new(Expr::Or(
                    Box::new(Expr::Atom(Atom::Today)),
                    Box::new(Expr::Atom(Atom::Overdue))
                )),
                Box::new(Expr::Not(Box::new(Expr::Atom(Atom::Label(
                    "waiting".into()
                )))))
            )
        );
        let g = parse("!!p1");
        let Group::Supported { expr, .. } = &g[0] else {
            panic!()
        };
        assert_eq!(
            *expr,
            Expr::Not(Box::new(Expr::Not(Box::new(Expr::Atom(Atom::Priority(1))))))
        );
    }

    #[test]
    fn comma_groups() {
        let g = parse("today, overdue & #Work ,  p1");
        assert_eq!(g.len(), 3);
        assert_eq!(g[0].text(), "today");
        assert_eq!(g[1].text(), "overdue & #Work");
        assert_eq!(g[2].text(), "p1");
        // One unsupported group does not spoil the others.
        let g = parse("today, due before: May 5");
        assert!(matches!(g[0], Group::Supported { .. }));
        assert!(matches!(g[1], Group::Unsupported { .. }));
    }

    #[test]
    fn unsupported_inputs() {
        for q in [
            "due before: tomorrow",
            "created: today",
            "@home*",
            "#",
            "(today",
            "today)",
            "today &",
            "& today",
            "",
            "   ",
            "today | ",
            "shared",
            "next week",
            "(today, p1)",
            "search:",
            "assigned to:",
            "view all",
        ] {
            assert!(unsupported(q), "{q:?} should be unsupported");
        }
    }

    fn item(v: serde_json::Value) -> Item {
        serde_json::from_value(v).unwrap()
    }

    fn ctx() -> EvalContext {
        let mut c = EvalContext {
            me: Some("me".into()),
            ..Default::default()
        };
        c.projects.insert("w".into(), ("work".into(), None));
        c.projects
            .insert("ws".into(), ("sub".into(), Some("w".into())));
        c.projects
            .insert("wss".into(), ("deeper".into(), Some("ws".into())));
        c.projects.insert("h".into(), ("home".into(), None));
        c.sections.insert("s1".into(), "meetings".into());
        c.people.insert("u2".into(), "alice smith".into());
        c
    }

    fn matches(q: &str, it: &Item) -> bool {
        let now = Now::fixed("2026-03-10T09:00:00+00:00").unwrap();
        let due = it
            .due
            .as_ref()
            .and_then(|d| d.date.as_deref())
            .and_then(|d| resolve_due_date(d, now.offset()));
        let t = EvalTask { item: it, due };
        let groups = parse(q);
        let Group::Supported { expr, .. } = &groups[0] else {
            panic!("{q} unsupported")
        };
        eval(expr, &t, &ctx(), &now)
    }

    #[test]
    fn evaluate_projects_and_sections() {
        let deep = item(json!({"id": "1", "project_id": "wss", "section_id": "s1"}));
        assert!(!matches("#Work", &deep));
        assert!(matches("##Work", &deep));
        assert!(matches("##sub", &deep));
        assert!(matches("#Deeper", &deep));
        assert!(!matches("##Home", &deep));
        assert!(matches("/meetings", &deep));
        assert!(matches("!#Home", &deep));
    }

    #[test]
    fn evaluate_dates() {
        let today = item(json!({"id": "1", "due": {"date": "2026-03-10"}}));
        let past_time = item(json!({"id": "2", "due": {"date": "2026-03-10T08:00:00"}}));
        let yesterday = item(json!({"id": "3", "due": {"date": "2026-03-09"}}));
        let tomorrow = item(json!({"id": "4", "due": {"date": "2026-03-11"}}));
        let in6 = item(json!({"id": "5", "due": {"date": "2026-03-16"}}));
        let in7 = item(json!({"id": "6", "due": {"date": "2026-03-17"}}));
        let none = item(json!({"id": "7"}));
        assert!(matches("today", &today));
        assert!(!matches("overdue", &today));
        assert!(matches("today", &past_time));
        assert!(matches("overdue", &past_time));
        assert!(matches("overdue", &yesterday));
        assert!(!matches("today", &yesterday));
        assert!(matches("tomorrow", &tomorrow));
        assert!(matches("next 7 days", &today));
        assert!(matches("next 7 days", &in6));
        assert!(!matches("next 7 days", &in7));
        assert!(!matches("next 7 days", &yesterday));
        assert!(matches("no date", &none));
        assert!(!matches("no date", &today));
        assert!(!matches("today | overdue", &none));
        assert!(matches("today | overdue", &yesterday));
    }

    #[test]
    fn evaluate_labels_priority_search_assignee() {
        let t = item(
            json!({"id": "1", "content": "Buy MILK today", "labels": ["Home", "errand"],
                            "priority": 4, "responsible_uid": "u2",
                            "due": {"date": "2026-03-12", "is_recurring": true, "string": "every day"}}),
        );
        let plain = item(json!({"id": "2", "content": "x", "responsible_uid": "me"}));
        assert!(matches("@home", &t));
        assert!(matches("@Errand & p1", &t));
        assert!(!matches("no labels", &t));
        assert!(matches("no labels", &plain));
        assert!(matches("p4", &plain));
        assert!(matches("search: milk", &t));
        assert!(!matches("search: bread", &t));
        assert!(matches("recurring", &t));
        assert!(matches("assigned to: others", &t));
        assert!(matches("assigned to: Alice", &t));
        assert!(matches("assigned to: alice smith", &t));
        assert!(!matches("assigned to: me", &t));
        assert!(matches("assigned to: me", &plain));
        assert!(matches("assigned", &plain));
        assert!(matches("(@home | @office) & !p2", &t));
    }
}
