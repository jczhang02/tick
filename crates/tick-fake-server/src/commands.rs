//! Sync API command execution (`docs/api/pages/sync_*.md`).

use std::collections::HashMap;

use serde_json::{Map, Value, json};

use crate::State;
use crate::dates;
use crate::order;

/// A `sync_status` error object shaped like the docs' example.
pub(crate) fn cmd_error(tag: &str, code: i64, msg: &str, http: i64) -> Value {
    json!({"error_tag": tag, "error_code": code, "error": msg, "http_code": http,
           "error_extra": {}})
}

fn not_found(what: &str) -> Value {
    let tag = format!("{}_NOT_FOUND", what.to_uppercase());
    // Docs: "ITEM_NOT_FOUND is non-retryable during sync" (a 400 command error).
    cmd_error(&tag, 22, &format!("{what} not found"), 400)
}

fn missing(arg: &str) -> Value {
    let mut e = cmd_error("ARGUMENT_MISSING", 19, "Required argument is missing", 400);
    e["error_extra"] = json!({"argument": arg});
    e
}

fn invalid(arg: &str, why: &str) -> Value {
    let mut e = cmd_error("INVALID_ARGUMENT_VALUE", 20, "Invalid argument value", 400);
    e["error_extra"] = json!({"argument": arg, "explanation": why});
    e
}

fn looks_like_temp_id(s: &str) -> bool {
    s.len() >= 32 && s.chars().filter(|c| *c == '-').count() == 4
}

type R<T> = Result<T, Value>;

struct Ctx<'a> {
    st: &'a mut State,
    temp: &'a mut HashMap<String, String>,
}

impl Ctx<'_> {
    /// Resolve an ID argument (temp ids from this request) and check the object exists.
    fn resolve(&self, r: &str, what: &str, raw: &str) -> R<String> {
        let id = self
            .temp
            .get(raw)
            .cloned()
            .unwrap_or_else(|| raw.to_owned());
        if self.st.get(r, &id).is_some() {
            return Ok(id);
        }
        if looks_like_temp_id(raw) {
            // Docs example: {"error_code": 15, "error": "Invalid temporary id"}.
            return Err(json!({"error_tag": "INVALID_TEMPID", "error_code": 15,
                              "error": "Invalid temporary id", "http_code": 400,
                              "error_extra": {}}));
        }
        Err(not_found(what))
    }

    fn arg_id(&self, args: &Value, key: &str, r: &str, what: &str) -> R<String> {
        let raw = id_str(args.get(key)).ok_or_else(|| missing(key))?;
        self.resolve(r, what, &raw)
    }

    fn opt_id(&self, args: &Value, key: &str, r: &str, what: &str) -> R<Option<String>> {
        match id_str(args.get(key)) {
            Some(raw) => self.resolve(r, what, &raw).map(Some),
            None => Ok(None),
        }
    }
}

fn id_str(v: Option<&Value>) -> Option<String> {
    match v? {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// Execute one command. Returns the temp id mapping it produced, if any.
pub(crate) fn execute(
    st: &mut State,
    c: &Value,
    temp: &mut HashMap<String, String>,
) -> Result<Option<(String, String)>, Value> {
    let kind = c["type"].as_str().unwrap_or("");
    let args = c.get("args").cloned().unwrap_or(json!({}));
    if c["uuid"].as_str().unwrap_or("").is_empty() {
        return Err(missing("uuid"));
    }
    if let Some(e) = st.take_command_error(kind, &args) {
        return Err(e);
    }
    let temp_id = c["temp_id"].as_str().map(str::to_owned);
    let mut ctx = Ctx { st, temp };
    let created = match kind {
        "item_add" => Some(item_add(&mut ctx, &args)?),
        "item_update" => {
            item_update(&mut ctx, &args)?;
            None
        }
        "item_move" => {
            item_move(&mut ctx, &args)?;
            None
        }
        "item_delete" => {
            let id = ctx.arg_id(&args, "id", "items", "item")?;
            delete_items(ctx.st, &id);
            None
        }
        "item_close" => {
            let id = ctx.arg_id(&args, "id", "items", "item")?;
            close_item(ctx.st, &id, true);
            None
        }
        "item_complete" => {
            let id = ctx.arg_id(&args, "id", "items", "item")?;
            let when = args["date_completed"].as_str().map(str::to_owned);
            complete_tree(ctx.st, &id, when);
            None
        }
        "item_update_date_complete" => {
            let id = ctx.arg_id(&args, "id", "items", "item")?;
            if let Some(due) = args.get("due").filter(|d| d.is_object()) {
                let due = dates::normalize_due(ctx.st, due).map_err(|e| invalid("due", &e))?;
                ctx.st.update("items", &id, |v| v["due"] = due);
            } else {
                close_item(ctx.st, &id, true);
            }
            None
        }
        "item_uncomplete" => {
            let raw = id_str(args.get("id")).ok_or_else(|| missing("id"))?;
            let id = ctx.temp.get(&raw).cloned().unwrap_or(raw);
            if !ctx.st.map("items").contains_key(&id) {
                // A task completed long ago is only in the completed archive (seeded
                // history): reopening brings it back as an active task.
                let Some(rec) = ctx
                    .st
                    .completed
                    .iter()
                    .rev()
                    .find(|c| c["id"].as_str() == Some(id.as_str()))
                    .cloned()
                else {
                    return Err(not_found("item"));
                };
                let mut v = rec;
                if let Some(o) = v.as_object_mut() {
                    o.remove("completed_by_uid");
                }
                v["checked"] = json!(true);
                v["is_deleted"] = json!(false);
                ctx.st.put("items", v);
            }
            uncomplete(ctx.st, &id);
            None
        }
        "item_reorder" => {
            let items = args["items"]
                .as_array()
                .cloned()
                .ok_or_else(|| missing("items"))?;
            let mut touched = Vec::new();
            for it in items {
                let id = ctx.arg_id(&it, "id", "items", "item")?;
                let order = it["child_order"]
                    .as_i64()
                    .ok_or_else(|| missing("child_order"))?;
                ctx.st
                    .update("items", &id, |v| v["child_order"] = json!(order));
                touched.push(id);
            }
            for id in touched {
                order::renumber_item_keys_from_child_order(ctx.st, &id);
            }
            None
        }
        "item_update_day_orders" => {
            let map = args["ids_to_orders"]
                .as_object()
                .cloned()
                .ok_or_else(|| missing("ids_to_orders"))?;
            for (raw, order) in map {
                let id = ctx.resolve("items", "item", &raw)?;
                ctx.st.update("items", &id, |v| v["day_order"] = order);
            }
            None
        }

        "note_add" => {
            let item_id = ctx.arg_id(&args, "item_id", "items", "item")?;
            let content = args["content"].as_str().ok_or_else(|| missing("content"))?;
            let id = ctx.st.new_id();
            let v = json!({
                "id": id, "posted_uid": ctx.st.user_id(), "item_id": item_id,
                "content": content, "file_attachment": args.get("file_attachment").cloned().unwrap_or(Value::Null),
                "uids_to_notify": args.get("uids_to_notify").cloned().unwrap_or(Value::Null),
                "is_deleted": false, "posted_at": ctx.st.now_utc_string(), "reactions": null,
            });
            ctx.st.put("notes", v);
            Some(id)
        }
        "note_update" => {
            let id = ctx.arg_id(&args, "id", "notes", "note")?;
            let content = args["content"]
                .as_str()
                .ok_or_else(|| missing("content"))?
                .to_owned();
            let attachment = args.get("file_attachment").cloned();
            ctx.st.update("notes", &id, |v| {
                v["content"] = json!(content);
                if let Some(a) = attachment {
                    v["file_attachment"] = a;
                }
            });
            None
        }
        "note_delete" => {
            let id = ctx.arg_id(&args, "id", "notes", "note")?;
            ctx.st.mark_deleted("notes", &id);
            None
        }

        "project_add" => {
            let name = args["name"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| missing("name"))?;
            let parent = ctx.opt_id(&args, "parent_id", "projects", "project")?;
            let id = ctx.st.new_id();
            let (child_order, order_key) = order::next_project_position(ctx.st, parent.as_deref());
            let now = ctx.st.now_utc_string();
            let mut v = json!({
                "id": id, "name": name, "description": "", "workspace_id": null,
                "color": args.get("color").cloned().unwrap_or(json!("charcoal")),
                "icon": args.get("icon").cloned().unwrap_or(Value::Null),
                "parent_id": parent, "child_order": child_order, "order_key": order_key,
                "is_collapsed": false, "shared": false, "can_assign_tasks": false,
                "is_deleted": false, "is_archived": false,
                "is_favorite": args.get("is_favorite").cloned().unwrap_or(json!(false)),
                "is_frozen": false,
                "view_style": args.get("view_style").cloned().unwrap_or(json!("list")),
                "role": null, "folder_id": null, "created_at": now, "updated_at": now,
            });
            if let Some(o) = args.get("child_order") {
                v["child_order"] = o.clone();
            }
            ctx.st.put("projects", v);
            Some(id)
        }
        "project_update" => {
            let id = ctx.arg_id(&args, "id", "projects", "project")?;
            let fields = pick(
                &args,
                &[
                    "name",
                    "color",
                    "icon",
                    "is_collapsed",
                    "is_favorite",
                    "view_style",
                    "order_key",
                    "description",
                    "status",
                ],
            );
            if fields
                .get("name")
                .is_some_and(|n| n.as_str().is_none_or(str::is_empty))
            {
                return Err(invalid("name", "name must not be empty"));
            }
            let reorder = fields.contains_key("order_key");
            ctx.st.update("projects", &id, |v| merge(v, &fields));
            if reorder {
                order::renumber_projects(ctx.st, &id);
            }
            None
        }
        "project_move" => {
            let id = ctx.arg_id(&args, "id", "projects", "project")?;
            let parent = ctx.opt_id(&args, "parent_id", "projects", "project")?;
            if let Some(p) = &parent
                && project_ancestors(ctx.st, p).contains(&id)
            {
                return Err(invalid(
                    "parent_id",
                    "a project cannot be moved into itself",
                ));
            }
            let (child_order, order_key) = order::next_project_position(ctx.st, parent.as_deref());
            ctx.st.update("projects", &id, |v| {
                v["parent_id"] = json!(parent);
                v["child_order"] = json!(child_order);
                v["order_key"] = json!(order_key);
            });
            None
        }
        "project_archive" => {
            let id = ctx.arg_id(&args, "id", "projects", "project")?;
            if ctx.st.inbox_id() == id {
                return Err(invalid("id", "the Inbox cannot be archived"));
            }
            for p in project_subtree(ctx.st, &id) {
                ctx.st
                    .update("projects", &p, |v| v["is_archived"] = json!(true));
            }
            None
        }
        "project_unarchive" => {
            let raw = id_str(args.get("id")).ok_or_else(|| missing("id"))?;
            let id = ctx.temp.get(&raw).cloned().unwrap_or(raw);
            if !ctx.st.map("projects").contains_key(&id) {
                return Err(not_found("project"));
            }
            let (child_order, order_key) = order::next_project_position(ctx.st, None);
            ctx.st.update("projects", &id, |v| {
                v["is_archived"] = json!(false);
                v["parent_id"] = Value::Null;
                v["child_order"] = json!(child_order);
                v["order_key"] = json!(order_key);
            });
            None
        }
        "project_delete" => {
            let id = ctx.arg_id(&args, "id", "projects", "project")?;
            if ctx.st.inbox_id() == id {
                return Err(invalid("id", "the Inbox cannot be deleted"));
            }
            for p in project_subtree(ctx.st, &id) {
                let items: Vec<String> = ctx
                    .st
                    .live("items")
                    .filter(|v| v["project_id"].as_str() == Some(p.as_str()))
                    .filter_map(|v| v["id"].as_str().map(str::to_owned))
                    .collect();
                for i in items {
                    delete_items(ctx.st, &i);
                }
                let sections: Vec<String> = ctx
                    .st
                    .live("sections")
                    .filter(|v| v["project_id"].as_str() == Some(p.as_str()))
                    .filter_map(|v| v["id"].as_str().map(str::to_owned))
                    .collect();
                for s in sections {
                    ctx.st.mark_deleted("sections", &s);
                }
                ctx.st.mark_deleted("projects", &p);
            }
            None
        }
        "project_reorder" => {
            let list = args["projects"]
                .as_array()
                .cloned()
                .ok_or_else(|| missing("projects"))?;
            let mut last = None;
            for p in list {
                let id = ctx.arg_id(&p, "id", "projects", "project")?;
                let order = p["child_order"]
                    .as_i64()
                    .ok_or_else(|| missing("child_order"))?;
                ctx.st
                    .update("projects", &id, |v| v["child_order"] = json!(order));
                last = Some(id);
            }
            if let Some(id) = last {
                order::renumber_project_keys_from_child_order(ctx.st, &id);
            }
            None
        }

        "section_add" => {
            let name = args["name"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| missing("name"))?;
            let project = ctx.arg_id(&args, "project_id", "projects", "project")?;
            let (order, key) = order::next_section_position(ctx.st, &project);
            let id = ctx.st.new_id();
            let now = ctx.st.now_utc_string();
            ctx.st.put(
                "sections",
                json!({
                    "id": id, "name": name,
                    "description": args.get("description").cloned().unwrap_or(Value::Null),
                    "project_id": project, "section_order": args.get("section_order").cloned().unwrap_or(json!(order)),
                    "order_key": key, "is_collapsed": false, "user_id": ctx.st.user_id(),
                    "is_deleted": false, "is_archived": false, "archived_at": null,
                    "added_at": now, "updated_at": now,
                }),
            );
            Some(id)
        }
        "section_update" => {
            let id = ctx.arg_id(&args, "id", "sections", "section")?;
            let fields = pick(&args, &["name", "description", "order_key", "is_collapsed"]);
            ctx.st.update("sections", &id, |v| merge(v, &fields));
            None
        }
        "section_move" => {
            let id = ctx.arg_id(&args, "id", "sections", "section")?;
            let project = ctx.arg_id(&args, "project_id", "projects", "project")?;
            ctx.st
                .update("sections", &id, |v| v["project_id"] = json!(project));
            let items: Vec<String> = ctx
                .st
                .live("items")
                .filter(|v| v["section_id"].as_str() == Some(id.as_str()))
                .filter_map(|v| v["id"].as_str().map(str::to_owned))
                .collect();
            for i in items {
                ctx.st
                    .update("items", &i, |v| v["project_id"] = json!(project));
            }
            None
        }
        "section_reorder" => {
            let list = args["sections"]
                .as_array()
                .cloned()
                .ok_or_else(|| missing("sections"))?;
            for s in list {
                let id = ctx.arg_id(&s, "id", "sections", "section")?;
                let order = s["section_order"]
                    .as_i64()
                    .ok_or_else(|| missing("section_order"))?;
                ctx.st
                    .update("sections", &id, |v| v["section_order"] = json!(order));
            }
            None
        }
        "section_delete" => {
            let id = ctx.arg_id(&args, "id", "sections", "section")?;
            // Docs: "Delete a section and all its child tasks."
            let items: Vec<String> = ctx
                .st
                .live("items")
                .filter(|v| v["section_id"].as_str() == Some(id.as_str()))
                .filter_map(|v| v["id"].as_str().map(str::to_owned))
                .collect();
            for i in items {
                delete_items(ctx.st, &i);
            }
            ctx.st.mark_deleted("sections", &id);
            None
        }
        "section_archive" | "section_unarchive" => {
            let id = ctx.arg_id(&args, "id", "sections", "section")?;
            let archived = kind == "section_archive";
            let now = ctx.st.now_utc_string();
            ctx.st.update("sections", &id, |v| {
                v["is_archived"] = json!(archived);
                v["archived_at"] = if archived { json!(now) } else { Value::Null };
            });
            None
        }

        "label_add" => {
            let name = args["name"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| missing("name"))?;
            if ctx.st.find("labels", "name", name).is_some() {
                return Err(cmd_error(
                    "LABEL_ALREADY_EXISTS",
                    72,
                    "Label already exists",
                    400,
                ));
            }
            let id = ctx.st.new_id();
            let order = ctx.st.live("labels").count() as i64 + 1;
            ctx.st.put(
                "labels",
                json!({"id": id, "name": name,
                       "color": args.get("color").cloned().unwrap_or(json!("charcoal")),
                       "item_order": args.get("item_order").cloned().unwrap_or(json!(order)),
                       "order_key": order::nth_key(order as usize),
                       "is_deleted": false,
                       "is_favorite": args.get("is_favorite").cloned().unwrap_or(json!(false))}),
            );
            Some(id)
        }
        "label_update" => {
            let id = ctx.arg_id(&args, "id", "labels", "label")?;
            let old = ctx
                .st
                .get("labels", &id)
                .and_then(|v| v["name"].as_str())
                .unwrap_or("")
                .to_owned();
            let fields = pick(
                &args,
                &["name", "color", "item_order", "order_key", "is_favorite"],
            );
            ctx.st.update("labels", &id, |v| merge(v, &fields));
            if let Some(new) = fields.get("name").and_then(Value::as_str)
                && new != old
            {
                rename_label(ctx.st, &old, Some(new));
            }
            None
        }
        "label_delete" => {
            let id = ctx.arg_id(&args, "id", "labels", "label")?;
            let name = ctx
                .st
                .get("labels", &id)
                .and_then(|v| v["name"].as_str())
                .unwrap_or("")
                .to_owned();
            if args["cascade"].as_str() != Some("none") {
                rename_label(ctx.st, &name, None);
            }
            ctx.st.mark_deleted("labels", &id);
            None
        }

        "filter_add" => {
            let name = args["name"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| missing("name"))?;
            let query = args["query"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| missing("query"))?;
            let id = ctx.st.new_id();
            let order = ctx.st.live("filters").count() as i64 + 1;
            ctx.st.put(
                "filters",
                json!({"id": id, "name": name, "query": query,
                       "description": args.get("description").cloned().unwrap_or(json!("")),
                       "color": args.get("color").cloned().unwrap_or(json!("charcoal")),
                       "item_order": args.get("item_order").cloned().unwrap_or(json!(order)),
                       "order_key": order::nth_key(order as usize),
                       "is_deleted": false,
                       "is_favorite": args.get("is_favorite").cloned().unwrap_or(json!(false)),
                       "is_frozen": false}),
            );
            Some(id)
        }
        "filter_update" => {
            let id = ctx.arg_id(&args, "id", "filters", "filter")?;
            let fields = pick(
                &args,
                &[
                    "name",
                    "query",
                    "description",
                    "color",
                    "item_order",
                    "order_key",
                    "is_favorite",
                ],
            );
            ctx.st.update("filters", &id, |v| merge(v, &fields));
            None
        }
        "filter_delete" => {
            let id = ctx.arg_id(&args, "id", "filters", "filter")?;
            ctx.st.mark_deleted("filters", &id);
            None
        }

        "reminder_add" => {
            let item_id = ctx.arg_id(&args, "item_id", "items", "item")?;
            let rtype = args["type"]
                .as_str()
                .ok_or_else(|| missing("type"))?
                .to_owned();
            let due = match args.get("due").filter(|d| d.is_object()) {
                Some(d) => dates::normalize_due(ctx.st, d).map_err(|e| invalid("due", &e))?,
                None => Value::Null,
            };
            if rtype == "absolute" && due.is_null() && args.get("minute_offset").is_none() {
                return Err(missing("due"));
            }
            let id = ctx.st.new_id();
            let mut v = json!({
                "id": id, "notify_uid": args.get("notify_uid").cloned().unwrap_or(json!(ctx.st.user_id())),
                "item_id": item_id, "type": rtype, "due": due,
                "minute_offset": args.get("minute_offset").cloned().unwrap_or(Value::Null),
                "is_deleted": false, "is_urgent": args.get("is_urgent").cloned().unwrap_or(json!(false)),
            });
            for k in ["name", "loc_lat", "loc_long", "loc_trigger", "radius"] {
                if let Some(x) = args.get(k) {
                    v[k] = x.clone();
                }
            }
            ctx.st.put("reminders", v);
            Some(id)
        }
        "reminder_update" => {
            let id = ctx.arg_id(&args, "id", "reminders", "reminder")?;
            let mut fields = pick(
                &args,
                &[
                    "notify_uid",
                    "type",
                    "minute_offset",
                    "is_urgent",
                    "name",
                    "loc_lat",
                    "loc_long",
                    "loc_trigger",
                    "radius",
                ],
            );
            if let Some(d) = args.get("due").filter(|d| d.is_object()) {
                let due = dates::normalize_due(ctx.st, d).map_err(|e| invalid("due", &e))?;
                fields.insert("due".into(), due);
            }
            ctx.st.update("reminders", &id, |v| merge(v, &fields));
            None
        }
        "reminder_delete" => {
            let id = ctx.arg_id(&args, "id", "reminders", "reminder")?;
            ctx.st.mark_deleted("reminders", &id);
            None
        }
        _ => {
            return Err(cmd_error(
                "INVALID_COMMAND",
                1,
                &format!("Unknown command type: {kind}"),
                400,
            ));
        }
    };
    Ok(match (temp_id, created) {
        (Some(t), Some(real)) => {
            ctx.temp.insert(t.clone(), real.clone());
            Some((t, real))
        }
        _ => None,
    })
}

fn pick(args: &Value, keys: &[&str]) -> Map<String, Value> {
    let mut out = Map::new();
    for k in keys {
        if let Some(v) = args.get(*k) {
            out.insert((*k).to_owned(), v.clone());
        }
    }
    out
}

fn merge(v: &mut Value, fields: &Map<String, Value>) {
    for (k, x) in fields {
        v[k] = x.clone();
    }
}

fn item_add(ctx: &mut Ctx<'_>, args: &Value) -> R<String> {
    let content = args["content"].as_str().ok_or_else(|| missing("content"))?;
    if content.trim().is_empty() {
        return Err(invalid("content", "content must not be empty"));
    }
    let parent = ctx.opt_id(args, "parent_id", "items", "item")?;
    let section = ctx.opt_id(args, "section_id", "sections", "section")?;
    let mut project = ctx.opt_id(args, "project_id", "projects", "project")?;
    let mut section = section;
    if let Some(p) = &parent {
        let pv = ctx.st.get("items", p).cloned().unwrap_or_default();
        project = pv["project_id"].as_str().map(str::to_owned);
        section = pv["section_id"].as_str().map(str::to_owned);
    } else if let Some(s) = &section {
        project = ctx
            .st
            .get("sections", s)
            .and_then(|v| v["project_id"].as_str())
            .map(str::to_owned);
    }
    let project = project.unwrap_or_else(|| ctx.st.inbox_id());
    let priority = match args.get("priority") {
        Some(p) => match p.as_i64() {
            Some(n @ 1..=4) => n,
            _ => return Err(invalid("priority", "priority must be between 1 and 4")),
        },
        None => 1,
    };
    let due = match args.get("due") {
        Some(d) if d.is_object() => {
            dates::normalize_due(ctx.st, d).map_err(|e| invalid("due", &e))?
        }
        _ => Value::Null,
    };
    let deadline = match args.get("deadline") {
        Some(d) if d.is_object() => json!({"date": d["date"], "lang": "en"}),
        _ => Value::Null,
    };
    let id = ctx.st.new_id();
    let (child_order, order_key) =
        order::next_item_position(ctx.st, &project, section.as_deref(), parent.as_deref());
    let me = ctx.st.user_id();
    let now = ctx.st.now_utc_string();
    let v = json!({
        "id": id, "user_id": me, "project_id": project, "content": content,
        "description": args.get("description").cloned().unwrap_or(json!("")),
        "priority": priority, "due": due, "deadline": deadline,
        "parent_id": parent, "child_order": args.get("child_order").cloned().unwrap_or(json!(child_order)),
        "order_key": args.get("order_key").cloned().unwrap_or(json!(order_key)),
        "section_id": section, "day_order": args.get("day_order").cloned().unwrap_or(json!(-1)),
        "is_collapsed": args.get("is_collapsed").cloned().unwrap_or(json!(false)),
        "labels": args.get("labels").cloned().unwrap_or(json!([])),
        "added_by_uid": me, "assigned_by_uid": me,
        "responsible_uid": id_str(args.get("responsible_uid")),
        "checked": false, "is_deleted": false, "added_at": now, "updated_at": now,
        "completed_at": null, "duration": args.get("duration").cloned().unwrap_or(Value::Null),
        "note_count": 0,
    });
    ctx.st.put("items", v);
    Ok(id)
}

fn item_update(ctx: &mut Ctx<'_>, args: &Value) -> R<()> {
    let id = ctx.arg_id(args, "id", "items", "item")?;
    // Docs: updating the parent, moving, completing or uncompleting is not supported by
    // item_update; such fields are ignored.
    let mut fields = pick(
        args,
        &[
            "content",
            "description",
            "priority",
            "is_collapsed",
            "labels",
            "assigned_by_uid",
            "responsible_uid",
            "day_order",
            "duration",
            "order_key",
        ],
    );
    if let Some(p) = fields.get("priority")
        && !matches!(p.as_i64(), Some(1..=4))
    {
        return Err(invalid("priority", "priority must be between 1 and 4"));
    }
    if fields
        .get("content")
        .is_some_and(|c| c.as_str().is_none_or(|s| s.trim().is_empty()))
    {
        return Err(invalid("content", "content must not be empty"));
    }
    if let Some(r) = fields.get("responsible_uid")
        && r.as_str() == Some("")
    {
        fields.insert("responsible_uid".into(), Value::Null);
    }
    if let Some(d) = args.get("due") {
        let due = if d.is_object() {
            dates::normalize_due(ctx.st, d).map_err(|e| invalid("due", &e))?
        } else {
            Value::Null
        };
        fields.insert("due".into(), due);
    }
    if let Some(d) = args.get("deadline") {
        let dl = if d.is_object() {
            json!({"date": d["date"], "lang": "en"})
        } else {
            Value::Null
        };
        fields.insert("deadline".into(), dl);
    }
    let reorder = fields.contains_key("order_key");
    if reorder {
        // Docs: a key already used by a sibling is adjusted to sort right after it.
        let key = fields["order_key"].as_str().unwrap_or("").to_owned();
        let adjusted = order::adjust_item_key(ctx.st, &id, &key);
        fields.insert("order_key".into(), json!(adjusted));
    }
    ctx.st.update("items", &id, |v| merge(v, &fields));
    if reorder {
        order::renumber_item_child_order_from_keys(ctx.st, &id);
    }
    Ok(())
}

fn item_move(ctx: &mut Ctx<'_>, args: &Value) -> R<()> {
    let id = ctx.arg_id(args, "id", "items", "item")?;
    let set: Vec<&str> = ["parent_id", "section_id", "project_id"]
        .into_iter()
        .filter(|k| args.get(*k).is_some_and(|v| !v.is_null()))
        .collect();
    if set.len() != 1 {
        return Err(invalid(
            "parent_id",
            "exactly one of parent_id, section_id or project_id must be set",
        ));
    }
    let (project, section, parent) = match set[0] {
        "parent_id" => {
            let p = ctx.arg_id(args, "parent_id", "items", "item")?;
            if item_subtree(ctx.st, &id).contains(&p) {
                return Err(invalid(
                    "parent_id",
                    "a task cannot become its own descendant",
                ));
            }
            let pv = ctx.st.get("items", &p).cloned().unwrap_or_default();
            (
                pv["project_id"].as_str().unwrap_or("").to_owned(),
                pv["section_id"].as_str().map(str::to_owned),
                Some(p),
            )
        }
        "section_id" => {
            let s = ctx.arg_id(args, "section_id", "sections", "section")?;
            let p = ctx
                .st
                .get("sections", &s)
                .and_then(|v| v["project_id"].as_str())
                .unwrap_or("")
                .to_owned();
            (p, Some(s), None)
        }
        _ => (
            ctx.arg_id(args, "project_id", "projects", "project")?,
            None,
            None,
        ),
    };
    let (child_order, key) =
        order::next_item_position(ctx.st, &project, section.as_deref(), parent.as_deref());
    ctx.st.update("items", &id, |v| {
        v["project_id"] = json!(project);
        v["section_id"] = json!(section);
        v["parent_id"] = json!(parent);
        v["child_order"] = json!(child_order);
        v["order_key"] = json!(key);
    });
    for child in item_subtree(ctx.st, &id).into_iter().skip(1) {
        ctx.st.update("items", &child, |v| {
            v["project_id"] = json!(project);
            v["section_id"] = json!(section);
        });
    }
    Ok(())
}

pub(crate) fn item_subtree(st: &State, id: &str) -> Vec<String> {
    let mut out = vec![id.to_owned()];
    let mut i = 0;
    while i < out.len() {
        let cur = out[i].clone();
        let kids: Vec<String> = st
            .live("items")
            .filter(|v| v["parent_id"].as_str() == Some(cur.as_str()))
            .filter_map(|v| v["id"].as_str().map(str::to_owned))
            .collect();
        out.extend(kids);
        i += 1;
    }
    out
}

fn project_subtree(st: &State, id: &str) -> Vec<String> {
    let mut out = vec![id.to_owned()];
    let mut i = 0;
    while i < out.len() {
        let cur = out[i].clone();
        let kids: Vec<String> = st
            .live("projects")
            .filter(|v| v["parent_id"].as_str() == Some(cur.as_str()))
            .filter_map(|v| v["id"].as_str().map(str::to_owned))
            .collect();
        out.extend(kids);
        i += 1;
    }
    out
}

fn project_ancestors(st: &State, id: &str) -> Vec<String> {
    let mut out = vec![id.to_owned()];
    let mut cur = st
        .get("projects", id)
        .and_then(|v| v["parent_id"].as_str())
        .map(str::to_owned);
    while let Some(p) = cur {
        if out.contains(&p) {
            break;
        }
        cur = st
            .get("projects", &p)
            .and_then(|v| v["parent_id"].as_str())
            .map(str::to_owned);
        out.push(p);
    }
    out
}

fn delete_items(st: &mut State, id: &str) {
    for x in item_subtree(st, id) {
        let notes: Vec<String> = st
            .live("notes")
            .filter(|v| v["item_id"].as_str() == Some(x.as_str()))
            .filter_map(|v| v["id"].as_str().map(str::to_owned))
            .collect();
        for n in notes {
            st.mark_deleted("notes", &n);
        }
        let reminders: Vec<String> = st
            .live("reminders")
            .filter(|v| v["item_id"].as_str() == Some(x.as_str()))
            .filter_map(|v| v["id"].as_str().map(str::to_owned))
            .collect();
        for r in reminders {
            st.mark_deleted("reminders", &r);
        }
        st.mark_deleted("items", &x);
    }
}

fn record_completion(st: &mut State, id: &str, at: &str) {
    if let Some(v) = st.get("items", id).cloned() {
        let mut rec = v;
        rec["checked"] = json!(true);
        rec["completed_at"] = json!(at);
        rec["completed_by_uid"] = json!(st.user_id());
        st.add_completed(rec);
    }
}

/// `item_close`: recurring tasks move to their next date, others complete with sub-tasks.
pub(crate) fn close_item(st: &mut State, id: &str, record: bool) {
    let v = st.get("items", id).cloned().unwrap_or_default();
    let recurring = v["due"]["is_recurring"].as_bool().unwrap_or(false);
    if recurring {
        let now = st.now_utc_string();
        if record {
            record_completion(st, id, &now);
        }
        let next = dates::next_occurrence(st, &v["due"]);
        st.update("items", id, |x| x["due"] = next);
    } else {
        complete_tree(st, id, None);
    }
}

fn complete_tree(st: &mut State, id: &str, when: Option<String>) {
    let at = when.unwrap_or_else(|| st.now_utc_string());
    for x in item_subtree(st, id) {
        let already = st
            .get("items", &x)
            .is_some_and(|v| v["checked"].as_bool().unwrap_or(false));
        if already {
            continue;
        }
        record_completion(st, &x, &at);
        let at = at.clone();
        st.update("items", &x, |v| {
            v["checked"] = json!(true);
            v["completed_at"] = json!(at);
        });
    }
}

fn uncomplete(st: &mut State, id: &str) {
    let mut cur = Some(id.to_owned());
    while let Some(x) = cur {
        let Some(v) = st.map("items").get(&x).map(|o| o.v.clone()) else {
            break;
        };
        if v["checked"].as_bool().unwrap_or(false) {
            st.update("items", &x, |v| {
                v["checked"] = json!(false);
                v["completed_at"] = Value::Null;
                v["is_deleted"] = json!(false);
            });
            st.completed
                .retain(|c| c["id"].as_str() != Some(x.as_str()));
            st.completed_rev = st.bump();
        }
        cur = v["parent_id"].as_str().map(str::to_owned);
    }
}

fn rename_label(st: &mut State, old: &str, new: Option<&str>) {
    let ids: Vec<String> = st
        .live("items")
        .filter(|v| {
            v["labels"]
                .as_array()
                .is_some_and(|l| l.iter().any(|x| x.as_str() == Some(old)))
        })
        .filter_map(|v| v["id"].as_str().map(str::to_owned))
        .collect();
    for id in ids {
        st.update("items", &id, |v| {
            let labels: Vec<Value> = v["labels"]
                .as_array()
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
        });
    }
}
