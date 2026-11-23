//! Optimistic local application of Sync API commands.
//!
//! Each command type mirrors what the docs say the server does (see the matching
//! `docs/api/pages/sync_*.md`). Effects that need the server (date parsing of free-text due
//! strings, the next date of a recurring task) are approximated: the row is marked pending
//! and the server's version replaces it after the next sync.
//!
//! `apply_command` must be replayable: the engine calls it again for every still-queued
//! command after resetting rows to server state.

use rusqlite::{Connection, OptionalExtension, Transaction};
use serde_json::{Value, json};

use super::{Changes, Kind, Origin, Result, delete_local, load_raw, meta_get, put_object};
use crate::clock::Now;
use crate::command::{Command, QUICK_ADD};
use crate::order_key;

fn str_arg<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
}

fn put(tx: &Transaction<'_>, kind: Kind, v: &Value, now: &Now, ch: &mut Changes) -> Result<()> {
    put_object(tx, kind, v, Origin::Local, now, ch)
}

pub(crate) fn user_id(conn: &Connection) -> Result<Option<String>> {
    let Some(raw) = meta_get(conn, "user")? else {
        return Ok(None);
    };
    let v: Value = serde_json::from_str(&raw)?;
    Ok(v.get("id").and_then(|x| match x {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }))
}

pub(crate) fn inbox_id(conn: &Connection) -> Result<Option<String>> {
    if let Some(raw) = meta_get(conn, "user")? {
        let v: Value = serde_json::from_str(&raw)?;
        if let Some(id) = v.get("inbox_project_id").and_then(Value::as_str)
            && !id.is_empty()
        {
            return Ok(Some(id.to_owned()));
        }
    }
    Ok(conn
        .query_row(
            "SELECT id FROM projects WHERE inbox_project = 1 AND local_deleted = 0 LIMIT 1",
            [],
            |r| r.get(0),
        )
        .optional()?)
}

fn next_child_order(
    tx: &Transaction<'_>,
    project_id: &str,
    section_id: Option<&str>,
    parent_id: Option<&str>,
) -> Result<i64> {
    let max: Option<i64> = tx
        .prepare_cached(
            "SELECT MAX(child_order) FROM items WHERE project_id = ?1
             AND section_id IS ?2 AND parent_id IS ?3 AND local_deleted = 0",
        )?
        .query_row(rusqlite::params![project_id, section_id, parent_id], |r| {
            r.get(0)
        })?;
    Ok(max.unwrap_or(0) + 1)
}

/// A key placing an item last among its siblings, or `None` when the siblings have no keys
/// (then `child_order` decides the position).
fn last_item_key(
    tx: &Transaction<'_>,
    project_id: &str,
    section_id: Option<&str>,
    parent_id: Option<&str>,
    exclude: &str,
) -> Result<Option<String>> {
    let max: Option<String> = tx
        .prepare_cached(
            "SELECT MAX(order_key) FROM items WHERE project_id = ?1
             AND section_id IS ?2 AND parent_id IS ?3 AND local_deleted = 0 AND checked = 0
             AND id != ?4",
        )?
        .query_row(
            rusqlite::params![project_id, section_id, parent_id, exclude],
            |r| r.get(0),
        )?;
    Ok(max.map(|m| order_key::key_between(Some(&m), None)))
}

fn last_project_key(
    tx: &Transaction<'_>,
    parent_id: Option<&str>,
    exclude: &str,
) -> Result<Option<String>> {
    let max: Option<String> = tx
        .prepare_cached(
            "SELECT MAX(order_key) FROM projects WHERE parent_id IS ?1 AND local_deleted = 0
             AND id != ?2",
        )?
        .query_row(rusqlite::params![parent_id, exclude], |r| r.get(0))?;
    Ok(max.map(|m| order_key::key_between(Some(&m), None)))
}

/// After a `child_order` change, give the sibling group fresh local keys in `child_order`
/// order so views (which sort by `order_key` first) show the new order until the server's
/// keys arrive.
fn rekey_item_group(tx: &Transaction<'_>, id: &str, now: &Now, ch: &mut Changes) -> Result<()> {
    let Some(v) = load_raw(tx, Kind::Item, id)? else {
        return Ok(());
    };
    let mut stmt = tx.prepare_cached(
        "SELECT id, child_order, order_key FROM items WHERE project_id = ?1
         AND section_id IS ?2 AND parent_id IS ?3 AND local_deleted = 0 AND checked = 0",
    )?;
    let mut sibs: Vec<(String, i64, Option<String>)> = stmt
        .query_map(
            rusqlite::params![
                v["project_id"].as_str().unwrap_or(""),
                v["section_id"].as_str(),
                v["parent_id"].as_str()
            ],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?
        .collect::<std::result::Result<_, _>>()?;
    if sibs.iter().all(|(_, _, k)| k.is_none()) {
        return Ok(());
    }
    sibs.sort_by(|a, b| a.1.cmp(&b.1).then(a.2.cmp(&b.2)));
    let keys = order_key::keys_after(None, sibs.len());
    for ((sid, _, old), key) in sibs.into_iter().zip(keys) {
        if old.as_deref() != Some(key.as_str()) {
            set_fields(tx, Kind::Item, &sid, &[("order_key", json!(key))], now, ch)?;
        }
    }
    Ok(())
}

fn ids_where(tx: &Transaction<'_>, sql: &str, param: &str) -> Result<Vec<String>> {
    let mut stmt = tx.prepare_cached(sql)?;
    let rows = stmt.query_map([param], |r| r.get(0))?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

/// IDs of the task and all its descendants (depth first).
fn item_subtree(tx: &Transaction<'_>, id: &str) -> Result<Vec<String>> {
    let mut out = vec![id.to_owned()];
    let mut i = 0;
    while i < out.len() {
        let children = ids_where(
            tx,
            "SELECT id FROM items WHERE parent_id = ?1 AND local_deleted = 0",
            &out[i],
        )?;
        out.extend(children);
        i += 1;
    }
    Ok(out)
}

fn project_subtree(tx: &Transaction<'_>, id: &str) -> Result<Vec<String>> {
    let mut out = vec![id.to_owned()];
    let mut i = 0;
    while i < out.len() {
        let children = ids_where(
            tx,
            "SELECT id FROM projects WHERE parent_id = ?1 AND local_deleted = 0",
            &out[i],
        )?;
        out.extend(children);
        i += 1;
    }
    Ok(out)
}

/// Normalize a `due` argument: `null` clears; an object with only `string` stays without a
/// `date` until the server parses it.
fn normalize_due(due: &Value) -> Value {
    match due {
        Value::Object(m) => {
            let mut m = m.clone();
            m.entry("is_recurring").or_insert(json!(false));
            if !m.contains_key("string") {
                let s = m
                    .get("date")
                    .cloned()
                    .unwrap_or(Value::String(String::new()));
                m.insert("string".into(), s);
            }
            if !m.contains_key("date") {
                m.insert("date".into(), Value::Null);
            }
            m.entry("timezone").or_insert(Value::Null);
            Value::Object(m)
        }
        _ => Value::Null,
    }
}

fn merge(target: &mut Value, args: &Value, skip: &[&str]) {
    let (Some(t), Some(a)) = (target.as_object_mut(), args.as_object()) else {
        return;
    };
    for (k, v) in a {
        if skip.contains(&k.as_str()) {
            continue;
        }
        let v = match k.as_str() {
            "due" => normalize_due(v),
            "responsible_uid" if v.as_str() == Some("") => Value::Null,
            _ => v.clone(),
        };
        t.insert(k.clone(), v);
    }
}

fn update(
    tx: &Transaction<'_>,
    kind: Kind,
    args: &Value,
    now: &Now,
    ch: &mut Changes,
) -> Result<()> {
    let Some(id) = str_arg(args, "id") else {
        return Ok(());
    };
    let Some(mut v) = load_raw(tx, kind, id)? else {
        return Ok(());
    };
    merge(&mut v, args, &["id"]);
    put(tx, kind, &v, now, ch)
}

fn set_fields(
    tx: &Transaction<'_>,
    kind: Kind,
    id: &str,
    fields: &[(&str, Value)],
    now: &Now,
    ch: &mut Changes,
) -> Result<()> {
    let Some(mut v) = load_raw(tx, kind, id)? else {
        return Ok(());
    };
    if let Some(m) = v.as_object_mut() {
        for (k, val) in fields {
            m.insert((*k).to_owned(), val.clone());
        }
    }
    put(tx, kind, &v, now, ch)
}

fn add_object(
    tx: &Transaction<'_>,
    kind: Kind,
    cmd: &Command,
    defaults: Value,
    skip: &[&str],
    now: &Now,
    ch: &mut Changes,
) -> Result<Option<Value>> {
    let Some(temp_id) = cmd.temp_id.as_deref() else {
        return Ok(None);
    };
    let mut v = defaults;
    merge(&mut v, &cmd.args, skip);
    v["id"] = json!(temp_id);
    put(tx, kind, &v, now, ch)?;
    Ok(Some(v))
}

/// Apply one command to the local rows. Unknown command types and references to missing
/// objects are ignored (the server is the authority).
pub(crate) fn apply_command(
    tx: &Transaction<'_>,
    cmd: &Command,
    now: &Now,
    ch: &mut Changes,
) -> Result<()> {
    let a = &cmd.args;
    let ts = json!(now.utc_string());
    match cmd.kind.as_str() {
        "item_add" | QUICK_ADD => {
            let mut parent_project = None;
            let mut parent_section = None;
            if let Some(pid) = str_arg(a, "parent_id")
                && let Some(p) = load_raw(tx, Kind::Item, pid)?
            {
                parent_project = p.get("project_id").cloned();
                parent_section = p.get("section_id").cloned();
            }
            let mut section_project = None;
            if let Some(sid) = str_arg(a, "section_id")
                && let Some(s) = load_raw(tx, Kind::Section, sid)?
            {
                section_project = s.get("project_id").cloned();
            }
            let project = parent_project
                .or(section_project)
                .or_else(|| a.get("project_id").cloned().filter(|v| !v.is_null()))
                .or(inbox_id(tx)?.map(Value::String))
                .unwrap_or(json!(""));
            let mut v = json!({
                "user_id": user_id(tx)?, "project_id": project, "content": "", "description": "",
                "priority": 1, "due": null, "deadline": null, "parent_id": null,
                "section_id": parent_section.unwrap_or(Value::Null), "child_order": 0,
                "day_order": -1, "is_collapsed": false, "labels": [], "added_by_uid": user_id(tx)?,
                "responsible_uid": null, "checked": false, "is_deleted": false,
                "added_at": ts, "updated_at": ts, "completed_at": null, "duration": null,
            });
            if cmd.kind == QUICK_ADD {
                v["content"] = a.get("text").cloned().unwrap_or(json!(""));
            } else {
                merge(
                    &mut v,
                    a,
                    &[
                        "auto_reminder",
                        "auto_parse_labels",
                        "project_id",
                        "temp_id",
                    ],
                );
            }
            let Some(temp_id) = cmd.temp_id.as_deref() else {
                return Ok(());
            };
            v["id"] = json!(temp_id);
            if a.get("child_order").is_none() {
                let order = next_child_order(
                    tx,
                    v["project_id"].as_str().unwrap_or(""),
                    v["section_id"].as_str(),
                    v["parent_id"].as_str(),
                )?;
                v["child_order"] = json!(order);
            }
            if a.get("order_key").is_none() {
                let key = last_item_key(
                    tx,
                    v["project_id"].as_str().unwrap_or(""),
                    v["section_id"].as_str(),
                    v["parent_id"].as_str(),
                    temp_id,
                )?;
                v["order_key"] = json!(key);
            }
            put(tx, Kind::Item, &v, now, ch)?;
        }
        "item_update" => update(tx, Kind::Item, a, now, ch)?,
        "item_move" => {
            let Some(id) = str_arg(a, "id") else {
                return Ok(());
            };
            let (project, section, parent): (Option<String>, Option<String>, Option<String>) =
                if let Some(pid) = str_arg(a, "parent_id") {
                    let Some(p) = load_raw(tx, Kind::Item, pid)? else {
                        return Ok(());
                    };
                    (
                        p["project_id"].as_str().map(str::to_owned),
                        p["section_id"].as_str().map(str::to_owned),
                        Some(pid.to_owned()),
                    )
                } else if let Some(sid) = str_arg(a, "section_id") {
                    let Some(s) = load_raw(tx, Kind::Section, sid)? else {
                        return Ok(());
                    };
                    (
                        s["project_id"].as_str().map(str::to_owned),
                        Some(sid.to_owned()),
                        None,
                    )
                } else if let Some(pid) = str_arg(a, "project_id") {
                    (Some(pid.to_owned()), None, None)
                } else {
                    return Ok(());
                };
            let project = project.unwrap_or_default();
            let order = next_child_order(tx, &project, section.as_deref(), parent.as_deref())?;
            // Docs: the task becomes the last one of its new siblings.
            let key = last_item_key(tx, &project, section.as_deref(), parent.as_deref(), id)?;
            set_fields(
                tx,
                Kind::Item,
                id,
                &[
                    ("project_id", json!(project)),
                    ("section_id", json!(section)),
                    ("parent_id", json!(parent)),
                    ("child_order", json!(order)),
                    ("order_key", json!(key)),
                ],
                now,
                ch,
            )?;
            for child in item_subtree(tx, id)?.into_iter().skip(1) {
                set_fields(
                    tx,
                    Kind::Item,
                    &child,
                    &[
                        ("project_id", json!(project)),
                        ("section_id", json!(section)),
                    ],
                    now,
                    ch,
                )?;
            }
        }
        "item_delete" => {
            if let Some(id) = str_arg(a, "id") {
                for x in item_subtree(tx, id)? {
                    delete_local(tx, Kind::Item, &x, ch)?;
                }
            }
        }
        "item_close" | "item_complete" => {
            let Some(id) = str_arg(a, "id") else {
                return Ok(());
            };
            let Some(item) = load_raw(tx, Kind::Item, id)? else {
                return Ok(());
            };
            let recurring = item
                .get("due")
                .and_then(|d| d.get("is_recurring"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            // A recurring task closed with item_close moves to its next date on the server;
            // until then it shows as completed-pending. Sub-tasks are left alone in that case.
            let targets = if recurring && cmd.kind == "item_close" {
                vec![id.to_owned()]
            } else {
                item_subtree(tx, id)?
            };
            let completed_at = a.get("date_completed").cloned().unwrap_or(ts.clone());
            for x in targets {
                set_fields(
                    tx,
                    Kind::Item,
                    &x,
                    &[
                        ("checked", json!(true)),
                        ("completed_at", completed_at.clone()),
                    ],
                    now,
                    ch,
                )?;
            }
        }
        "item_uncomplete" => {
            let Some(id) = str_arg(a, "id") else {
                return Ok(());
            };
            if load_raw(tx, Kind::Item, id)?.is_none() {
                // Completed tasks are not part of the active data; restore from the history
                // cache when we have it.
                let raw: Option<String> = tx
                    .query_row("SELECT raw FROM completed_items WHERE id = ?1", [id], |r| {
                        r.get(0)
                    })
                    .optional()?;
                if let Some(raw) = raw {
                    let mut v: Value = serde_json::from_str(&raw)?;
                    v["checked"] = json!(false);
                    v["completed_at"] = Value::Null;
                    put(tx, Kind::Item, &v, now, ch)?;
                }
                return Ok(());
            }
            // The task and its ancestors become active again.
            let mut cur = Some(id.to_owned());
            while let Some(x) = cur {
                let Some(v) = load_raw(tx, Kind::Item, &x)? else {
                    break;
                };
                set_fields(
                    tx,
                    Kind::Item,
                    &x,
                    &[("checked", json!(false)), ("completed_at", Value::Null)],
                    now,
                    ch,
                )?;
                cur = v
                    .get("parent_id")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
            }
        }
        "item_reorder" => {
            if let Some(items) = a.get("items").and_then(Value::as_array) {
                let mut groups = std::collections::BTreeMap::new();
                for it in items {
                    if let (Some(id), Some(order)) = (str_arg(it, "id"), it.get("child_order")) {
                        set_fields(
                            tx,
                            Kind::Item,
                            id,
                            &[("child_order", order.clone())],
                            now,
                            ch,
                        )?;
                        if let Some(v) = load_raw(tx, Kind::Item, id)? {
                            let key = format!(
                                "{}|{}|{}",
                                v["project_id"].as_str().unwrap_or(""),
                                v["section_id"].as_str().unwrap_or(""),
                                v["parent_id"].as_str().unwrap_or("")
                            );
                            groups.entry(key).or_insert_with(|| id.to_owned());
                        }
                    }
                }
                for id in groups.into_values() {
                    rekey_item_group(tx, &id, now, ch)?;
                }
            }
        }
        "item_update_day_orders" => {
            if let Some(map) = a.get("ids_to_orders").and_then(Value::as_object) {
                for (id, order) in map {
                    set_fields(tx, Kind::Item, id, &[("day_order", order.clone())], now, ch)?;
                }
            }
        }

        "note_add" => {
            let me = user_id(tx)?;
            add_object(
                tx,
                Kind::Note,
                cmd,
                json!({"posted_uid": me, "item_id": "", "content": "", "file_attachment": null,
                       "uids_to_notify": null, "is_deleted": false, "posted_at": ts,
                       "reactions": null}),
                &[],
                now,
                ch,
            )?;
        }
        "note_update" => update(tx, Kind::Note, a, now, ch)?,
        "note_delete" => {
            if let Some(id) = str_arg(a, "id") {
                delete_local(tx, Kind::Note, id, ch)?;
            }
        }

        "project_add" => {
            let parent = str_arg(a, "parent_id");
            let max: Option<i64> = tx
                .prepare_cached(
                    "SELECT MAX(child_order) FROM projects WHERE parent_id IS ?1 AND local_deleted = 0",
                )?
                .query_row([parent], |r| r.get(0))?;
            let key = last_project_key(tx, parent, cmd.temp_id.as_deref().unwrap_or(""))?;
            add_object(
                tx,
                Kind::Project,
                cmd,
                json!({"name": "", "color": "charcoal", "parent_id": null, "order_key": key,
                       "child_order": max.unwrap_or(0) + 1, "is_collapsed": false, "shared": false,
                       "can_assign_tasks": false, "is_deleted": false, "is_archived": false,
                       "is_favorite": false, "view_style": "list", "created_at": ts,
                       "updated_at": ts}),
                &[],
                now,
                ch,
            )?;
        }
        "project_update" => update(tx, Kind::Project, a, now, ch)?,
        "project_move" => {
            if let Some(id) = str_arg(a, "id") {
                let parent = a.get("parent_id").cloned().unwrap_or(Value::Null);
                let key = last_project_key(tx, parent.as_str(), id)?;
                set_fields(
                    tx,
                    Kind::Project,
                    id,
                    &[("parent_id", parent), ("order_key", json!(key))],
                    now,
                    ch,
                )?;
            }
        }
        "project_reorder" => {
            if let Some(list) = a.get("projects").and_then(Value::as_array) {
                for p in list {
                    if let (Some(id), Some(order)) = (str_arg(p, "id"), p.get("child_order")) {
                        set_fields(
                            tx,
                            Kind::Project,
                            id,
                            &[("child_order", order.clone()), ("order_key", Value::Null)],
                            now,
                            ch,
                        )?;
                    }
                }
            }
        }
        "project_archive" => {
            if let Some(id) = str_arg(a, "id") {
                for p in project_subtree(tx, id)? {
                    set_fields(
                        tx,
                        Kind::Project,
                        &p,
                        &[("is_archived", json!(true))],
                        now,
                        ch,
                    )?;
                }
            }
        }
        "project_unarchive" => {
            if let Some(id) = str_arg(a, "id") {
                // Docs: the project is unarchived alone, becomes a root project and is placed
                // at the end of the other root projects.
                let key = last_project_key(tx, None, id)?;
                set_fields(
                    tx,
                    Kind::Project,
                    id,
                    &[
                        ("is_archived", json!(false)),
                        ("parent_id", Value::Null),
                        ("order_key", json!(key)),
                    ],
                    now,
                    ch,
                )?;
            }
        }
        "project_delete" => {
            if let Some(id) = str_arg(a, "id") {
                for p in project_subtree(tx, id)? {
                    for item in ids_where(
                        tx,
                        "SELECT id FROM items WHERE project_id = ?1 AND local_deleted = 0",
                        &p,
                    )? {
                        delete_local(tx, Kind::Item, &item, ch)?;
                    }
                    for s in ids_where(
                        tx,
                        "SELECT id FROM sections WHERE project_id = ?1 AND local_deleted = 0",
                        &p,
                    )? {
                        delete_local(tx, Kind::Section, &s, ch)?;
                    }
                    delete_local(tx, Kind::Project, &p, ch)?;
                }
            }
        }

        "section_add" => {
            let project = str_arg(a, "project_id").unwrap_or("");
            let max: Option<i64> = tx
                .prepare_cached(
                    "SELECT MAX(section_order) FROM sections WHERE project_id = ?1 AND local_deleted = 0",
                )?
                .query_row([project], |r| r.get(0))?;
            add_object(
                tx,
                Kind::Section,
                cmd,
                json!({"name": "", "project_id": project, "section_order": max.unwrap_or(0) + 1,
                       "is_collapsed": false, "user_id": user_id(tx)?, "is_deleted": false,
                       "is_archived": false, "archived_at": null, "added_at": ts,
                       "updated_at": ts, "description": null}),
                &[],
                now,
                ch,
            )?;
        }
        "section_update" => update(tx, Kind::Section, a, now, ch)?,
        "section_move" => {
            if let (Some(id), Some(project)) = (str_arg(a, "id"), str_arg(a, "project_id")) {
                set_fields(
                    tx,
                    Kind::Section,
                    id,
                    &[("project_id", json!(project))],
                    now,
                    ch,
                )?;
                for item in ids_where(
                    tx,
                    "SELECT id FROM items WHERE section_id = ?1 AND local_deleted = 0",
                    id,
                )? {
                    set_fields(
                        tx,
                        Kind::Item,
                        &item,
                        &[("project_id", json!(project))],
                        now,
                        ch,
                    )?;
                }
            }
        }
        "section_archive" | "section_unarchive" => {
            if let Some(id) = str_arg(a, "id") {
                let archived = cmd.kind == "section_archive";
                set_fields(
                    tx,
                    Kind::Section,
                    id,
                    &[("is_archived", json!(archived))],
                    now,
                    ch,
                )?;
            }
        }
        "section_delete" => {
            if let Some(id) = str_arg(a, "id") {
                // Docs: "Delete a section and all its child tasks."
                for item in ids_where(
                    tx,
                    "SELECT id FROM items WHERE section_id = ?1 AND local_deleted = 0",
                    id,
                )? {
                    delete_local(tx, Kind::Item, &item, ch)?;
                }
                delete_local(tx, Kind::Section, id, ch)?;
            }
        }

        "label_add" => {
            let max: Option<i64> = tx.query_row(
                "SELECT MAX(item_order) FROM labels WHERE local_deleted = 0",
                [],
                |r| r.get(0),
            )?;
            add_object(
                tx,
                Kind::Label,
                cmd,
                json!({"name": "", "color": "charcoal", "item_order": max.unwrap_or(0) + 1,
                       "is_deleted": false, "is_favorite": false}),
                &[],
                now,
                ch,
            )?;
        }
        "label_update" => {
            let Some(id) = str_arg(a, "id") else {
                return Ok(());
            };
            let old = load_raw(tx, Kind::Label, id)?;
            update(tx, Kind::Label, a, now, ch)?;
            // Tasks reference labels by name, so a rename updates them too.
            if let (Some(old), Some(new_name)) = (old, str_arg(a, "name"))
                && let Some(old_name) = old.get("name").and_then(Value::as_str)
                && old_name != new_name
            {
                rename_label_on_items(tx, old_name, Some(new_name), now, ch)?;
            }
        }
        "label_delete" => {
            let Some(id) = str_arg(a, "id") else {
                return Ok(());
            };
            if let Some(old) = load_raw(tx, Kind::Label, id)?
                && let Some(name) = old.get("name").and_then(Value::as_str)
                && str_arg(a, "cascade") != Some("none")
            {
                rename_label_on_items(tx, name, None, now, ch)?;
            }
            delete_local(tx, Kind::Label, id, ch)?;
        }

        "filter_add" => {
            let max: Option<i64> = tx.query_row(
                "SELECT MAX(item_order) FROM filters WHERE local_deleted = 0",
                [],
                |r| r.get(0),
            )?;
            add_object(
                tx,
                Kind::Filter,
                cmd,
                json!({"name": "", "query": "", "description": "", "color": "charcoal",
                       "item_order": max.unwrap_or(0) + 1, "is_deleted": false,
                       "is_favorite": false, "is_frozen": false}),
                &[],
                now,
                ch,
            )?;
        }
        "filter_update" => update(tx, Kind::Filter, a, now, ch)?,
        "filter_delete" => {
            if let Some(id) = str_arg(a, "id") {
                delete_local(tx, Kind::Filter, id, ch)?;
            }
        }

        "reminder_add" => {
            let me = user_id(tx)?;
            add_object(
                tx,
                Kind::Reminder,
                cmd,
                json!({"notify_uid": me, "item_id": "", "type": "absolute", "due": null,
                       "minute_offset": null, "is_deleted": false, "is_urgent": false}),
                &[],
                now,
                ch,
            )?;
        }
        "reminder_update" => update(tx, Kind::Reminder, a, now, ch)?,
        "reminder_delete" => {
            if let Some(id) = str_arg(a, "id") {
                delete_local(tx, Kind::Reminder, id, ch)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn rename_label_on_items(
    tx: &Transaction<'_>,
    old: &str,
    new: Option<&str>,
    now: &Now,
    ch: &mut Changes,
) -> Result<()> {
    let ids = ids_where(
        tx,
        "SELECT l.item_id FROM item_labels l JOIN items i ON i.id = l.item_id
         WHERE l.name = ?1 AND i.local_deleted = 0",
        old,
    )?;
    for id in ids {
        let Some(mut v) = load_raw(tx, Kind::Item, &id)? else {
            continue;
        };
        let labels: Vec<Value> = v
            .get("labels")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|l| match (l.as_str(), new) {
                (Some(n), Some(new)) if n == old => Some(json!(new)),
                (Some(n), None) if n == old => None,
                _ => Some(l),
            })
            .collect();
        v["labels"] = Value::Array(labels);
        put(tx, Kind::Item, &v, now, ch)?;
    }
    Ok(())
}
