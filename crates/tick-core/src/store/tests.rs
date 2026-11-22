use std::sync::Arc;

use serde_json::{Value, json};

use super::*;
use crate::command::Command;
use crate::model::SyncResponse;

const NOW: &str = "2026-03-10T09:00:00+08:00";

fn store() -> Store {
    let mut s = Store::open_in_memory().unwrap();
    s.set_clock(Arc::new(|| Now::fixed(NOW).unwrap()));
    s
}

fn resp(v: Value) -> SyncResponse {
    serde_json::from_value(v).unwrap()
}

fn item(id: &str, project: &str, content: &str, extra: Value) -> Value {
    let mut v = json!({"id": id, "project_id": project, "content": content, "description": "",
                       "priority": 1, "checked": false, "is_deleted": false, "labels": [],
                       "child_order": 1, "due": null, "parent_id": null, "section_id": null});
    if let (Some(o), Some(e)) = (v.as_object_mut(), extra.as_object()) {
        for (k, x) in e {
            o.insert(k.clone(), x.clone());
        }
    }
    v
}

fn due(date: &str) -> Value {
    json!({"date": date, "string": date, "is_recurring": false, "timezone": null, "lang": "en"})
}

/// A store after a full sync with a small account.
fn seeded() -> Store {
    let mut s = store();
    s.apply_sync_response(
        &[],
        &resp(json!({
            "full_sync": true, "sync_token": "t1",
            "user": {"id": "u1", "full_name": "Me", "inbox_project_id": "inbox", "token": "SECRET"},
            "projects": [
                {"id": "inbox", "name": "Inbox", "inbox_project": true, "child_order": 0},
                {"id": "work", "name": "Work", "child_order": 1, "order_key": "a1"},
                {"id": "sub", "name": "Sub", "parent_id": "work", "child_order": 1},
                {"id": "home", "name": "Home", "child_order": 2, "order_key": "a2"},
                {"id": "old", "name": "Old", "child_order": 3, "is_archived": true}
            ],
            "sections": [
                {"id": "s2", "project_id": "work", "name": "Later", "section_order": 2},
                {"id": "s1", "project_id": "work", "name": "Now", "section_order": 1}
            ],
            "labels": [{"id": "l1", "name": "errand", "item_order": 1}],
            "items": [
                item("a", "inbox", "Buy milk", json!({"due": due("2026-03-10"), "labels": ["errand"]})),
                item("b", "inbox", "Overdue thing", json!({"due": due("2026-03-08"), "priority": 4})),
                item("c", "work", "Plan the 季度 review", json!({"section_id": "s1", "order_key": "a2", "child_order": 1})),
                item("d", "work", "First by key", json!({"section_id": "s1", "order_key": "a1", "child_order": 2})),
                item("e", "work", "Child of d", json!({"section_id": "s1", "parent_id": "d"})),
                item("f", "work", "No section", json!({"due": due("2026-03-12T15:00:00")})),
                item("g", "home", "学习 Rust 所有权", json!({"description": "借用检查 and lifetimes", "due": due("2026-03-11")})),
                item("h", "sub", "Sub task", json!({"due": due("2026-03-20")})),
                item("i", "home", "Timed today", json!({"due": due("2026-03-10T08:00:00")}))
            ]
        })),
    )
    .unwrap();
    s
}

#[test]
fn migrations_are_versioned_and_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("x/cache.sqlite3");
    {
        let s = Store::open(&path).unwrap();
        s.set_setting("theme", "dark").unwrap();
    }
    let s = Store::open(&path).unwrap();
    let v: i64 = s
        .connection()
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(v, 1);
    let mode: String = s
        .connection()
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .unwrap();
    assert_eq!(mode, "wal");
    assert_eq!(s.setting("theme").unwrap().as_deref(), Some("dark"));
    s.remove_setting("theme").unwrap();
    assert_eq!(s.setting("theme").unwrap(), None);
}

#[test]
fn full_sync_and_queries() {
    let s = seeded();
    assert_eq!(s.sync_token().unwrap().as_deref(), Some("t1"));
    let raw: String = s
        .connection()
        .query_row("SELECT value FROM meta WHERE key = 'user'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert!(!raw.contains("SECRET"));

    let today = s.today().unwrap();
    let ids = |v: &[Task]| v.iter().map(|t| t.id.clone()).collect::<Vec<_>>();
    assert_eq!(ids(&today.overdue), vec!["b"]);
    // Full-day tasks sort before timed ones on the same day.
    assert_eq!(ids(&today.today), vec!["a", "i"]);

    let up = s.upcoming(7).unwrap();
    assert_eq!(up.days.len(), 7);
    assert_eq!(ids(&up.overdue), vec!["b"]);
    assert_eq!(ids(&up.days[0].tasks), vec!["a", "i"]);
    assert_eq!(ids(&up.days[1].tasks), vec!["g"]);
    assert_eq!(ids(&up.days[2].tasks), vec!["f"]);
    assert!(up.days[3].tasks.is_empty());
    assert!(up.days.iter().all(|d| !d.tasks.iter().any(|t| t.id == "h")));

    let view = s.project_view("work").unwrap().unwrap();
    assert_eq!(view.sections[0].section.name, "Now");
    assert_eq!(view.sections[1].section.name, "Later");
    let now_tasks = TaskNode::flatten(&view.sections[0].tasks);
    let order: Vec<(usize, &str)> = now_tasks.iter().map(|(d, t)| (*d, t.id.as_str())).collect();
    assert_eq!(order, vec![(0, "d"), (1, "e"), (0, "c")]);
    assert_eq!(view.tasks.len(), 1);

    let tree = s.projects_tree().unwrap();
    let names: Vec<(usize, &str, usize)> = tree
        .iter()
        .map(|p| (p.depth, p.project.name.as_str(), p.active_count))
        .collect();
    assert_eq!(
        names,
        vec![
            (0, "Inbox", 2),
            (0, "Work", 4),
            (1, "Sub", 1),
            (0, "Home", 2)
        ]
    );
    assert!(tree[1].has_children);
    assert_eq!(s.archived_projects().unwrap()[0].name, "Old");
    assert_eq!(s.label_view("ERRAND").unwrap()[0].id, "a");
    assert_eq!(s.labels().unwrap()[0].active_count, 1);
    assert_eq!(s.inbox().unwrap().unwrap().tasks.len(), 2);
}

#[test]
fn search_uses_fts_and_handles_chinese_and_short_terms() {
    let mut s = seeded();
    let ids = |v: Vec<Task>| v.into_iter().map(|t| t.item.id).collect::<Vec<_>>();
    assert_eq!(ids(s.search("milk", 50).unwrap()), vec!["a"]);
    assert_eq!(ids(s.search("MILK buy", 50).unwrap()), vec!["a"]);
    assert_eq!(ids(s.search("所有权", 50).unwrap()), vec!["g"]);
    // Two-character Chinese term: shorter than a trigram, LIKE fallback.
    assert_eq!(ids(s.search("季度", 50).unwrap()), vec!["c"]);
    assert_eq!(ids(s.search("借用", 50).unwrap()), vec!["g"]);
    assert_eq!(ids(s.search("lifetimes", 50).unwrap()), vec!["g"]);
    assert!(s.search("   ", 50).unwrap().is_empty());
    assert!(s.search("nothing-matches", 50).unwrap().is_empty());
    assert!(s.search("50%", 50).unwrap().is_empty());
    // Completed tasks drop out of the index; edits are reindexed.
    s.enqueue(Command::item_close("a")).unwrap();
    assert!(s.search("milk", 50).unwrap().is_empty());
    s.enqueue(Command::item_update(
        "b",
        json!({"content": "Renamed overdue"}),
    ))
    .unwrap();
    assert_eq!(ids(s.search("renamed", 50).unwrap()), vec!["b"]);
    assert!(s.search("Overdue thing", 50).unwrap().is_empty());
}

#[test]
fn enqueue_applies_optimistically_and_persists() {
    let mut s = seeded();
    let add = Command::item_add(json!({"content": "New", "section_id": "s2"}));
    let temp = add.temp_id.clone().unwrap();
    let changes = s.enqueue(add).unwrap();
    assert!(changes.ids(Kind::Item).any(|i| i == temp));
    let t = s.task(&temp).unwrap().unwrap();
    assert!(t.pending);
    assert_eq!(t.project_id, "work", "project comes from the section");
    assert_eq!(s.pending_count().unwrap(), 1);
    let sub = Command::item_add(json!({"content": "Sub", "parent_id": temp}));
    let sub_id = sub.temp_id.clone().unwrap();
    s.enqueue(sub).unwrap();
    assert_eq!(s.subtasks(&temp).unwrap()[0].id, sub_id);
    s.enqueue(Command::item_move(
        &temp,
        crate::MoveTarget::Project("home".into()),
    ))
    .unwrap();
    assert_eq!(s.task(&sub_id).unwrap().unwrap().project_id, "home");
    assert_eq!(s.task(&sub_id).unwrap().unwrap().section_id, None);
    s.enqueue(Command::item_delete(&temp)).unwrap();
    assert!(s.task(&temp).unwrap().is_none());
    assert!(s.task(&sub_id).unwrap().is_none());
    let queued = s.pending_commands().unwrap();
    assert_eq!(
        queued
            .iter()
            .map(|q| q.command.kind.as_str())
            .collect::<Vec<_>>(),
        vec!["item_add", "item_add", "item_move", "item_delete"]
    );
    // Discarding the delete brings the task back.
    let del = queued[3].command.uuid.clone();
    s.discard_queued(&del).unwrap();
    assert!(s.task(&temp).unwrap().is_some());
}

#[test]
fn sync_response_settles_queue_and_maps_temp_ids() {
    let mut s = seeded();
    let add = Command::item_add(json!({"content": "Draft", "project_id": "inbox"}));
    let temp = add.temp_id.clone().unwrap();
    s.enqueue(add.clone()).unwrap();
    let note = Command::note_add(&temp, "hello");
    s.enqueue(note.clone()).unwrap();
    let sent = s.pending_commands().unwrap();
    // The server only processed the first command (e.g. the batch was split).
    let applied = s
        .apply_sync_response(
            &sent[..1],
            &resp(json!({
                "sync_token": "t2",
                "sync_status": {add.uuid.clone(): "ok"},
                "temp_id_mapping": {temp.clone(): "real1"},
                "items": [item("real1", "inbox", "Draft", json!({}))]
            })),
        )
        .unwrap();
    assert_eq!(applied.acknowledged, 1);
    assert!(s.task(&temp).unwrap().is_none());
    assert!(!s.task("real1").unwrap().unwrap().pending);
    let rest = s.pending_commands().unwrap();
    assert_eq!(rest.len(), 1);
    assert_eq!(rest[0].command.args["item_id"], "real1");
    let notes = s.comments("real1").unwrap();
    assert_eq!(
        notes.len(),
        1,
        "the queued comment was replayed on the real id"
    );
    assert_eq!(s.sync_token().unwrap().as_deref(), Some("t2"));
}

#[test]
fn terminal_error_rolls_back_to_server_copy() {
    let mut s = seeded();
    let upd = Command::item_update("a", json!({"content": "Edited", "priority": 4}));
    s.enqueue(upd.clone()).unwrap();
    assert_eq!(s.task("a").unwrap().unwrap().content, "Edited");
    let applied = s
        .apply_sync_response(
            &s.pending_commands().unwrap(),
            &resp(json!({
                "sync_token": "t2",
                "sync_status": {upd.uuid.clone(): {"error_tag": "ITEM_NOT_FOUND", "error_code": 22,
                                                    "error": "Item not found", "http_code": 400}}
            })),
        )
        .unwrap();
    assert_eq!(applied.failed.len(), 1);
    let t = s.task("a").unwrap().unwrap();
    assert_eq!(t.content, "Buy milk");
    assert_eq!(t.priority, 1);
    assert!(!t.pending);
    assert_eq!(s.pending_count().unwrap(), 0);
    let failed = s.failed_commands().unwrap();
    assert_eq!(failed[0].command.uuid, upd.uuid);
    assert!(failed[0].error.contains("Item not found"));
    s.dismiss_failed(failed[0].seq).unwrap();
    assert!(s.failed_commands().unwrap().is_empty());
}

#[test]
fn retryable_error_keeps_command_and_effect() {
    let mut s = seeded();
    let upd = Command::item_update("a", json!({"content": "Edited"}));
    s.enqueue(upd.clone()).unwrap();
    let applied = s
        .apply_sync_response(
            &s.pending_commands().unwrap(),
            &resp(json!({
                "sync_token": "t2",
                "sync_status": {upd.uuid.clone(): {"error": "later", "http_code": 503}},
                "items": [item("a", "inbox", "Server edit", json!({"description": "srv"}))]
            })),
        )
        .unwrap();
    assert_eq!(applied.retry.len(), 1);
    let t = s.task("a").unwrap().unwrap();
    assert_eq!(t.content, "Edited");
    assert_eq!(t.description, "srv");
    let q = s.pending_commands().unwrap();
    assert_eq!(q[0].attempts, 1);
    assert_eq!(q[0].last_error.as_deref(), Some("later"));
}

#[test]
fn full_sync_replaces_data_but_keeps_queued_adds() {
    let mut s = seeded();
    let add = Command::item_add(json!({"content": "Unsent"}));
    let temp = add.temp_id.clone().unwrap();
    s.enqueue(add).unwrap();
    s.apply_sync_response(
        &[],
        &resp(json!({
            "full_sync": true, "sync_token": "t9",
            "projects": [{"id": "inbox", "name": "Inbox", "inbox_project": true}],
            "items": [item("z", "inbox", "Only one", json!({}))]
        })),
    )
    .unwrap();
    assert!(s.task("a").unwrap().is_none());
    assert!(s.project("work").unwrap().is_none());
    assert!(s.task("z").unwrap().is_some());
    assert!(s.task(&temp).unwrap().unwrap().pending);
    // Sections were not part of the response, so they are untouched.
    assert!(s.section("s1").unwrap().is_some());
}

#[test]
fn quick_add_placeholder_is_replaced() {
    let mut s = seeded();
    let q = Command::quick_add("Buy eggs tomorrow p1");
    let temp = q.temp_id.clone().unwrap();
    s.enqueue(q).unwrap();
    s.enqueue(Command::item_update(
        &temp,
        json!({"description": "a dozen"}),
    ))
    .unwrap();
    let placeholder = s.task(&temp).unwrap().unwrap();
    assert_eq!(placeholder.project_id, "inbox");
    assert_eq!(placeholder.content, "Buy eggs tomorrow p1");
    let queued = s.pending_commands().unwrap();
    s.apply_quick_add_result(
        &queued[0],
        &item("q1", "inbox", "Buy eggs", json!({"priority": 4})),
    )
    .unwrap();
    assert!(s.task(&temp).unwrap().is_none());
    let t = s.task("q1").unwrap().unwrap();
    assert_eq!(t.description, "a dozen");
    assert!(t.pending);
    assert_eq!(s.pending_commands().unwrap()[0].command.args["id"], "q1");
}

#[test]
fn labels_projects_sections_locally() {
    let mut s = seeded();
    s.enqueue(Command::label_update("l1", json!({"name": "shopping"})))
        .unwrap();
    assert_eq!(s.task("a").unwrap().unwrap().labels, vec!["shopping"]);
    s.enqueue(Command::label_delete("l1")).unwrap();
    assert!(s.task("a").unwrap().unwrap().labels.is_empty());
    assert!(s.labels().unwrap().is_empty());

    s.enqueue(Command::project_archive("work")).unwrap();
    let tree: Vec<String> = s
        .projects_tree()
        .unwrap()
        .into_iter()
        .map(|p| p.project.id)
        .collect();
    assert_eq!(tree, vec!["inbox", "home"]);
    s.enqueue(Command::project_unarchive("sub")).unwrap();
    assert_eq!(s.project("sub").unwrap().unwrap().parent_id, None);

    s.enqueue(Command::section_delete("s1")).unwrap();
    assert!(s.task("c").unwrap().is_none());
    s.enqueue(Command::project_delete("home")).unwrap();
    assert!(s.task("g").unwrap().is_none());
    assert!(s.project("home").unwrap().is_none());

    let p = Command::project_add(json!({"name": "New", "parent_id": "sub"}));
    let pid = p.temp_id.clone().unwrap();
    s.enqueue(p).unwrap();
    s.enqueue(Command::project_move(&pid, None)).unwrap();
    assert_eq!(s.project(&pid).unwrap().unwrap().parent_id, None);
    let sec = Command::section_add(&pid, "Inbox zero");
    s.enqueue(sec).unwrap();
    assert_eq!(s.sections(&pid).unwrap()[0].name, "Inbox zero");

    let f = Command::filter_add(json!({"name": "Mine", "query": "assigned to: me"}));
    let fid = f.temp_id.clone().unwrap();
    s.enqueue(f).unwrap();
    s.enqueue(Command::filter_update(&fid, json!({"query": "p1"})))
        .unwrap();
    assert_eq!(s.filters().unwrap()[0].query, "p1");
    s.enqueue(Command::filter_delete(&fid)).unwrap();
    assert!(s.filters().unwrap().is_empty());

    let r = Command::reminder_add(json!({"item_id": "a", "type": "absolute",
                                          "due": {"date": "2026-03-10T10:00:00"}}));
    let rid = r.temp_id.clone().unwrap();
    s.enqueue(r).unwrap();
    assert_eq!(s.reminders("a").unwrap().len(), 1);
    assert_eq!(s.all_reminders().unwrap().len(), 1);
    s.enqueue(Command::reminder_update(&rid, json!({"minute_offset": 15})))
        .unwrap();
    s.enqueue(Command::reminder_delete(&rid)).unwrap();
    assert!(s.reminders("a").unwrap().is_empty());

    let n = Command::note_add("a", "first");
    let nid = n.temp_id.clone().unwrap();
    s.enqueue(n).unwrap();
    s.enqueue(Command::note_update(&nid, "edited")).unwrap();
    assert_eq!(s.comments("a").unwrap()[0].content, "edited");
    assert_eq!(s.comment_counts().unwrap()["a"], 1);
    s.enqueue(Command::note_delete(&nid)).unwrap();
    assert!(s.comments("a").unwrap().is_empty());
}

#[test]
fn complete_and_uncomplete_locally() {
    let mut s = seeded();
    s.enqueue(Command::item_close("d")).unwrap();
    assert!(s.task("d").unwrap().unwrap().checked);
    assert!(
        s.task("e").unwrap().unwrap().checked,
        "sub-tasks complete too"
    );
    s.enqueue(Command::item_uncomplete("e")).unwrap();
    assert!(!s.task("e").unwrap().unwrap().checked);
    assert!(
        !s.task("d").unwrap().unwrap().checked,
        "ancestors are reopened"
    );

    // A recurring task closes as "completed-pending" until the server moves its date.
    s.apply_sync_response(
        &[],
        &resp(
            json!({"sync_token": "t3", "items": [item("r", "inbox", "Daily", json!({
            "due": {"date": "2026-03-10", "string": "every day", "is_recurring": true}}))]}),
        ),
    )
    .unwrap();
    s.enqueue(Command::item_close("r")).unwrap();
    let r = s.task("r").unwrap().unwrap();
    assert!(r.checked && r.pending);

    // Reopening a task from the completed history cache.
    s.cache_completed(&[
        json!({"id": "old", "project_id": "inbox", "content": "Archived",
                               "checked": true, "completed_at": "2026-03-09T10:00:00Z"}),
    ])
    .unwrap();
    assert_eq!(s.completed_history(None, 10).unwrap()[0].item.id, "old");
    s.enqueue(Command::item_uncomplete("old")).unwrap();
    assert!(!s.task("old").unwrap().unwrap().checked);
}

#[test]
fn reorder_and_move_show_in_views() {
    let mut s = seeded();
    let order = |s: &Store, project: &str, section: usize| -> Vec<String> {
        let view = s.project_view(project).unwrap().unwrap();
        let nodes = if section == 0 {
            &view.tasks
        } else {
            &view.sections[section - 1].tasks
        };
        nodes.iter().map(|n| n.task.id.clone()).collect()
    };
    assert_eq!(order(&s, "work", 1), vec!["d", "c"]);
    // Keys present: one item_update with a key before "d".
    let cmd = s.reorder_command("c", None).unwrap().unwrap();
    assert_eq!(cmd.kind, "item_update");
    s.enqueue(cmd).unwrap();
    assert_eq!(order(&s, "work", 1), vec!["c", "d"]);
    // The deprecated item_reorder also shows, even though siblings have keys.
    s.enqueue(Command::item_reorder(&[("d".into(), 1), ("c".into(), 2)]))
        .unwrap();
    assert_eq!(order(&s, "work", 1), vec!["d", "c"]);

    // No keys in the Inbox: the helper falls back to item_reorder.
    assert_eq!(order(&s, "inbox", 0), vec!["a", "b"]);
    let cmd = s.reorder_command("a", Some("b")).unwrap().unwrap();
    assert_eq!(cmd.kind, "item_reorder");
    s.enqueue(cmd).unwrap();
    assert_eq!(order(&s, "inbox", 0), vec!["b", "a"]);
    assert!(s.reorder_command("a", Some("zzz")).unwrap().is_none());

    // A moved task lands last among keyed siblings, not at its old key's position.
    s.apply_sync_response(
        &[],
        &resp(json!({"sync_token": "t5", "items": [
            item("x1", "work", "Later one", json!({"section_id": "s2", "order_key": "a5"})),
            item("x2", "work", "Later two", json!({"section_id": "s2", "order_key": "a6"}))
        ]})),
    )
    .unwrap();
    s.enqueue(Command::item_move(
        "d",
        crate::MoveTarget::Section("s2".into()),
    ))
    .unwrap();
    assert_eq!(order(&s, "work", 2), vec!["x1", "x2", "d"]);
    // Projects too.
    s.enqueue(Command::project_move("sub", None)).unwrap();
    let names: Vec<String> = s
        .projects_tree()
        .unwrap()
        .into_iter()
        .map(|p| p.project.name)
        .collect();
    assert_eq!(names, vec!["Inbox", "Work", "Home", "Sub"]);
}

#[test]
fn filter_view_evaluates_locally() {
    let s = seeded();
    let groups = s
        .filter_view("today | overdue, ##Work, @errand & p4")
        .unwrap();
    let ids = |r: &FilterGroupResult| match r {
        FilterGroupResult::Local(v) => v.iter().map(|t| t.id.clone()).collect::<Vec<_>>(),
        other => panic!("{other:?}"),
    };
    assert_eq!(ids(&groups[0].result), vec!["b", "a", "i"]);
    let mut work = ids(&groups[1].result);
    work.sort();
    assert_eq!(work, vec!["c", "d", "e", "f", "h"]);
    assert_eq!(ids(&groups[2].result), vec!["a"]);
}
