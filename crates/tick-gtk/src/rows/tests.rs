use std::sync::Arc;

use serde_json::{Value, json};
use tick_core::{Command, Now, Store};

use super::*;

/// Sunday 22 November 2026, 09:00 UTC.
const NOW: &str = "2026-11-22T09:00:00+00:00";

fn due(date: &str) -> Value {
    json!({"date": date, "string": date, "is_recurring": false, "timezone": null, "lang": "en"})
}

fn item(id: &str, project: &str, content: &str, extra: Value) -> Value {
    let mut v = json!({"id": id, "project_id": project, "content": content, "description": "",
                       "priority": 1, "checked": false, "is_deleted": false, "labels": [],
                       "child_order": 1, "due": null, "parent_id": null, "section_id": null,
                       "day_order": -1});
    if let (Some(o), Some(e)) = (v.as_object_mut(), extra.as_object()) {
        for (k, x) in e {
            o.insert(k.clone(), x.clone());
        }
    }
    v
}

fn store() -> Store {
    let mut s = Store::open_in_memory().unwrap();
    s.set_clock(Arc::new(|| Now::fixed(NOW).unwrap()));
    let resp = json!({
        "full_sync": true, "sync_token": "t1",
        "user": {"id": "me", "full_name": "Sam Rivera", "inbox_project_id": "inbox",
                 "date_format": 0, "time_format": 0},
        "projects": [
            {"id": "inbox", "name": "Inbox", "inbox_project": true, "child_order": 0},
            {"id": "work", "name": "Work Stuff", "color": "blue", "child_order": 1,
             "shared": true, "is_favorite": true},
            {"id": "home", "name": "Home", "color": "not-a-color", "child_order": 2},
            {"id": "garden", "name": "Garden", "color": "green", "parent_id": "home",
             "child_order": 1}
        ],
        "sections": [
            {"id": "s1", "project_id": "work", "name": "This week", "section_order": 1},
            {"id": "s2", "project_id": "work", "name": "Backlog", "section_order": 2}
        ],
        "labels": [
            {"id": "l1", "name": "phone", "item_order": 1},
            {"id": "l2", "name": "errand", "item_order": 2, "is_favorite": true}
        ],
        "filters": [
            {"id": "f1", "name": "Urgent", "query": "p1", "item_order": 1},
            {"id": "f2", "name": "Odd", "query": "due after: 2026-11-01", "item_order": 2}
        ],
        "collaborators": [
            {"id": "maya", "full_name": "Maya Lopez", "email": "maya@example.com"}
        ],
        "collaborator_states": [
            {"project_id": "work", "user_id": "maya", "state": "active"},
            {"project_id": "work", "user_id": "me", "state": "active"}
        ],
        "notes": [
            {"id": "n1", "item_id": "w1", "content": "Numbers are in", "posted_at": "2026-11-21T10:00:00Z"},
            {"id": "n2", "item_id": "w1", "content": "Thanks", "posted_at": "2026-11-21T11:00:00Z"}
        ],
        "items": [
            item("in1", "inbox", "Call the dentist", json!({"due": due("2026-11-22"), "priority": 3, "labels": ["phone"], "child_order": 1})),
            item("in2", "inbox", "Renew passport", json!({"due": due("2026-11-20"), "priority": 4,
                 "deadline": {"date": "2026-12-02"}, "description": "Bring **two photos**\n- form", "child_order": 2})),
            item("in3", "inbox", "Read the article", json!({"child_order": 3})),
            item("w1", "work", "Plan the deck", json!({"section_id": "s1", "due": due("2026-11-22T14:00:00"),
                 "priority": 4, "responsible_uid": "maya", "child_order": 1})),
            item("w2", "work", "Collect metrics", json!({"section_id": "s1", "parent_id": "w1", "child_order": 1})),
            item("w3", "work", "Clean up flags", json!({"section_id": "s2", "labels": ["errand"], "child_order": 1})),
            item("w4", "work", "Root task", json!({"due": due("2026-11-25"), "child_order": 1})),
            item("h1", "home", "Water the plants", json!({"due": {"date": "2026-11-23", "string": "every monday",
                 "is_recurring": true}, "child_order": 1})),
            item("h2", "garden", "Plant bulbs", json!({"due": due("2026-12-05"), "child_order": 1}))
        ]
    });
    s.apply_sync_response(&[], &serde_json::from_value(resp).unwrap())
        .unwrap();
    s
}

fn keys(c: &Content) -> Vec<String> {
    c.rows.iter().map(Row::key).collect()
}

fn task<'a>(c: &'a Content, id: &str) -> &'a TaskRow {
    c.rows
        .iter()
        .filter_map(Row::task)
        .find(|t| t.id == id)
        .unwrap_or_else(|| panic!("no task {id} in {:?}", keys(c)))
}

#[test]
fn today_has_overdue_then_today_groups() {
    let s = store();
    let c = build(&s, &ViewId::Today, &UiState::default()).unwrap();
    assert_eq!(
        keys(&c),
        [
            "header",
            "day:overdue",
            "task:in2",
            "day:2026-11-22",
            "task:w1",
            "task:in1",
            "ghost:today"
        ]
    );
    let Row::ViewHeader(h) = &c.rows[0] else {
        panic!()
    };
    assert_eq!(h.subtitle.as_deref(), Some("Sunday 22 November"));
    assert_eq!(h.count, 3);
    // Today shows times only; overdue keeps its date in red.
    assert_eq!(task(&c, "w1").due.as_ref().unwrap().text, "14:00");
    assert_eq!(task(&c, "in1").due, None);
    let overdue = task(&c, "in2").due.clone().unwrap();
    assert_eq!(
        (overdue.text.as_str(), overdue.state),
        ("20 Nov", DueState::Overdue)
    );
    assert_eq!(task(&c, "in2").p, 1);
    assert_eq!(
        task(&c, "in2").description.as_deref(),
        Some("Bring two photos")
    );
    // Shared project: assignee name; cross-project view: project with section.
    let w1 = task(&c, "w1");
    assert_eq!(w1.assignee.as_deref(), Some("Maya Lopez"));
    assert_eq!(w1.project.as_ref().unwrap().name, "Work Stuff / This week");
    assert_eq!(w1.comments, 2);
    assert_eq!(w1.subtasks, Some((0, 1)));
    let Some(Row::Ghost { prefill, .. }) = c.rows.last() else {
        panic!()
    };
    assert_eq!(prefill, "2026-11-22 ");
}

#[test]
fn today_without_overdue_has_no_group_headers() {
    let mut s = store();
    s.enqueue(Command::item_close("in2")).unwrap();
    let c = build(&s, &ViewId::Today, &UiState::default()).unwrap();
    assert_eq!(keys(&c), ["header", "task:w1", "task:in1", "ghost:today"]);
}

#[test]
fn upcoming_groups_days() {
    let s = store();
    let c = build(&s, &ViewId::Upcoming, &UiState::default()).unwrap();
    assert_eq!(
        keys(&c),
        [
            "header",
            "day:overdue",
            "task:in2",
            "day:2026-11-22",
            "task:w1",
            "task:in1",
            "ghost:2026-11-22",
            "day:2026-11-23",
            "task:h1",
            "ghost:2026-11-23",
            "day:2026-11-24",
            "ghost:2026-11-24",
            "day:2026-11-25",
            "task:w4",
            "ghost:2026-11-25",
            "day:2026-11-26",
            "ghost:2026-11-26",
            "day:2026-11-27",
            "ghost:2026-11-27",
            "day:2026-11-28",
            "ghost:2026-11-28",
            "month:2026-12",
            "day:2026-12-05",
            "task:h2",
            "ghost:2026-12-05",
        ]
    );
    let day = |k: &str| {
        c.rows
            .iter()
            .find_map(|r| match r {
                Row::DayHeader(d) if d.key == k => Some(d.clone()),
                _ => None,
            })
            .unwrap()
    };
    let today = day("day:2026-11-22");
    assert_eq!(
        (today.number, today.label.as_str(), today.dim.as_deref()),
        (Some(22), "Today", Some("Sunday"))
    );
    let other = day("day:2026-11-25");
    assert_eq!((other.label.as_str(), other.dim), ("Wednesday", None));
    assert!(task(&c, "h1").recurring);
    assert_eq!(task(&c, "h1").due, None, "full-day tasks show no time");
}

#[test]
fn project_sections_and_subtasks() {
    let s = store();
    let c = build(&s, &ViewId::Project("work".into()), &UiState::default()).unwrap();
    assert_eq!(
        keys(&c),
        [
            "header",
            "task:w4",
            "ghost:root",
            "section:s1",
            "task:w1",
            "task:w2",
            "ghost:section:s1",
            "section:s2",
            "task:w3",
            "ghost:section:s2"
        ]
    );
    assert_eq!(c.prefill, "#Work\\ Stuff ");
    assert_eq!(task(&c, "w2").depth, 1);
    assert!(task(&c, "w1").has_children);
    assert_eq!(task(&c, "w1").project, None, "own project is not repeated");
    let Row::ViewHeader(h) = &c.rows[0] else {
        panic!()
    };
    assert_eq!(h.count, 4);
    assert_eq!(h.color.as_deref(), Some("blue"));
    assert_eq!(h.avatars, ["Maya Lopez"]);
    let ghost = c
        .rows
        .iter()
        .find(|r| r.key() == "ghost:section:s1")
        .unwrap();
    assert_eq!(
        ghost,
        &Row::Ghost {
            key: "ghost:section:s1".into(),
            prefill: "#Work\\ Stuff /This\\ week ".into()
        }
    );

    // Collapsed section: header stays, tasks go; collapsed parent hides its children.
    let mut ui = UiState::default();
    ui.collapsed_sections.insert("s1".into());
    let c = build(&s, &ViewId::Project("work".into()), &ui).unwrap();
    assert!(keys(&c).contains(&"section:s1".to_owned()));
    assert!(!keys(&c).contains(&"task:w1".to_owned()));
    let mut s = s;
    s.enqueue(Command::item_update("w1", json!({"is_collapsed": true})))
        .unwrap();
    let c = build(&s, &ViewId::Project("work".into()), &UiState::default()).unwrap();
    assert!(task(&c, "w1").collapsed);
    assert!(!keys(&c).contains(&"task:w2".to_owned()));
}

#[test]
fn inbox_label_and_filters() {
    let s = store();
    let c = build(&s, &ViewId::Inbox, &UiState::default()).unwrap();
    assert_eq!(
        keys(&c),
        ["header", "task:in1", "task:in2", "task:in3", "ghost:root"]
    );
    assert_eq!(c.title, "Inbox");

    let c = build(&s, &ViewId::Label("l1".into()), &UiState::default()).unwrap();
    assert_eq!(keys(&c), ["header", "task:in1"]);
    assert_eq!(c.title, "phone");

    let c = build(&s, &ViewId::Filter("f1".into()), &UiState::default()).unwrap();
    assert_eq!(keys(&c), ["header", "task:in2", "task:w1"]);
    assert_eq!(c.server_query, None);

    // Unsupported locally: loading row, then cached server results with a source line.
    let mut s = s;
    let c = build(&s, &ViewId::Filter("f2".into()), &UiState::default()).unwrap();
    assert_eq!(keys(&c), ["header", "loading:0"]);
    assert_eq!(c.server_query.as_deref(), Some("due after: 2026-11-01"));
    let offline = UiState {
        offline: true,
        ..UiState::default()
    };
    let c = build(&s, &ViewId::Filter("f2".into()), &offline).unwrap();
    assert!(matches!(&c.rows[1], Row::Empty(e) if e.icon == "network-offline-symbolic"));
    let w4 = s.task("w4").unwrap().unwrap();
    s.cache_filter_results(
        "due after: 2026-11-01",
        &[serde_json::to_value(&w4.item).unwrap()],
    )
    .unwrap();
    let c = build(&s, &ViewId::Filter("f2".into()), &UiState::default()).unwrap();
    // A single group: the source line is part of the view header (4.22).
    assert_eq!(keys(&c), ["header", "task:w4"]);
    let Row::ViewHeader(h) = &c.rows[0] else {
        panic!()
    };
    assert_eq!(
        h.source,
        Some(("Results from Todoist, updated 09:00".to_owned(), false))
    );
}

#[test]
fn completing_states() {
    let mut s = store();
    let mut ui = UiState::default();
    ui.completing.insert("in1".into());
    let c = build(&s, &ViewId::Inbox, &ui).unwrap();
    let t = task(&c, "in1");
    assert!(t.checked && t.completing && !t.completed_pending);

    // A recurring task closed locally stays as completed-pending.
    s.enqueue(Command::item_close("h1")).unwrap();
    let c = build(&s, &ViewId::Project("home".into()), &UiState::default()).unwrap();
    let t = task(&c, "h1");
    assert!(t.checked && t.completed_pending && !t.completing);
    assert!(t.meta_description().starts_with("Completing"));
}

#[test]
fn sidebar_tree_and_counts() {
    let s = store();
    let d = sidebar(&s).unwrap();
    assert_eq!((d.inbox, d.today), (3, 3));
    let names: Vec<_> = d.projects.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["Work Stuff", "Home"]);
    assert_eq!(d.projects[1].children[0].name, "Garden");
    assert_eq!(d.projects[0].count, 4);
    let favs: Vec<_> = d.favorites.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(favs, ["Work Stuff", "errand"]);
    assert_eq!(d.labels[0].count, 1);
    assert_eq!(d.filters[0].count, 2);
    assert_eq!(color_class("not-a-color"), "tick-color-charcoal");
}

#[test]
fn meta_description_reads_the_meta_line() {
    let s = store();
    let c = build(&s, &ViewId::Today, &UiState::default()).unwrap();
    assert_eq!(
        task(&c, "w1").meta_description(),
        "Due 14:00, 0 of 1 sub-task, 2 comments, project Work Stuff / This week, \
         Assigned to Maya Lopez"
    );
    assert_eq!(
        task(&c, "in2").meta_description(),
        "Due 20 Nov, deadline 2 Dec, project Inbox"
    );
}

#[test]
fn view_ids_round_trip() {
    for v in [
        ViewId::Inbox,
        ViewId::Today,
        ViewId::Upcoming,
        ViewId::Project("p".into()),
        ViewId::Label("l".into()),
        ViewId::Filter("f".into()),
    ] {
        assert_eq!(ViewId::from_setting(&v.to_setting()), Some(v));
    }
    assert_eq!(ViewId::from_setting("bogus"), None);
}
