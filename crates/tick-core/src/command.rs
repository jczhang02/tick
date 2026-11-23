//! Sync API commands (`docs/api/pages/sync_overview.md`, "Write resources").
//!
//! A command is `{type, uuid, temp_id?, args}`. The constructors below cover the command
//! types Tick uses; each takes `args` exactly as documented in the corresponding
//! `docs/api/pages/sync_*.md` page. [`Command::quick_add`] is a local-only pseudo command that
//! is never sent to `/sync`: the engine submits it to `POST /api/v1/tasks/quick`.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

/// Local pseudo command type for offline quick add.
pub const QUICK_ADD: &str = "tick_quick_add";

/// Command types whose `temp_id` names the created object.
pub const CREATE_TYPES: &[&str] = &[
    "item_add",
    "project_add",
    "section_add",
    "label_add",
    "filter_add",
    "note_add",
    "reminder_add",
    QUICK_ADD,
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Command {
    #[serde(rename = "type")]
    pub kind: String,
    pub uuid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temp_id: Option<String>,
    pub args: Value,
}

pub fn new_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn obj(args: Value) -> Value {
    match args {
        Value::Object(_) => args,
        Value::Null => Value::Object(Map::new()),
        other => json!({ "value": other }),
    }
}

impl Command {
    /// A command without a temp id.
    pub fn new(kind: &str, args: Value) -> Command {
        Command {
            kind: kind.to_owned(),
            uuid: new_uuid(),
            temp_id: None,
            args: obj(args),
        }
    }

    /// A create command with a fresh temp id. The temp id doubles as the local row id until
    /// the server's `temp_id_mapping` replaces it.
    pub fn create(kind: &str, args: Value) -> Command {
        Command {
            kind: kind.to_owned(),
            uuid: new_uuid(),
            temp_id: Some(new_uuid()),
            args: obj(args),
        }
    }

    fn with_id(kind: &str, id: &str, mut fields: Value) -> Command {
        let mut fields = obj(fields.take());
        fields["id"] = json!(id);
        Command::new(kind, fields)
    }

    // ---- tasks (sync_tasks.md) ----

    /// `item_add`: `args` must contain `content`; optional `project_id`, `section_id`,
    /// `parent_id`, `due`, `deadline`, `priority`, `labels`, `description`, ...
    pub fn item_add(args: Value) -> Command {
        Command::create("item_add", args)
    }
    /// `item_update`: any of `content`, `description`, `due`, `deadline`, `priority`,
    /// `labels`, `responsible_uid`, `is_collapsed`, `order_key`, `day_order`, `duration`.
    pub fn item_update(id: &str, fields: Value) -> Command {
        Command::with_id("item_update", id, fields)
    }
    /// `item_move`: exactly one of `parent_id`, `section_id`, `project_id`.
    pub fn item_move(id: &str, target: MoveTarget) -> Command {
        let args = match target {
            MoveTarget::Parent(p) => json!({ "parent_id": p }),
            MoveTarget::Section(s) => json!({ "section_id": s }),
            MoveTarget::Project(p) => json!({ "project_id": p }),
        };
        Command::with_id("item_move", id, args)
    }
    pub fn item_delete(id: &str) -> Command {
        Command::with_id("item_delete", id, json!({}))
    }
    /// `item_close`: what official clients do on "complete" (recurring tasks advance).
    pub fn item_close(id: &str) -> Command {
        Command::with_id("item_close", id, json!({}))
    }
    pub fn item_complete(id: &str) -> Command {
        Command::with_id("item_complete", id, json!({}))
    }
    /// `item_uncomplete`: the v1 name for reopening a completed task.
    pub fn item_uncomplete(id: &str) -> Command {
        Command::with_id("item_uncomplete", id, json!({}))
    }
    /// Deprecated `item_reorder` with `child_order` values; prefer
    /// [`Command::item_update`] with `order_key` when siblings have keys.
    pub fn item_reorder(orders: &[(String, i64)]) -> Command {
        let items: Vec<Value> = orders
            .iter()
            .map(|(id, o)| json!({"id": id, "child_order": o}))
            .collect();
        Command::new("item_reorder", json!({ "items": items }))
    }

    // ---- comments (sync_comments.md) ----
    pub fn note_add(item_id: &str, content: &str) -> Command {
        Command::create("note_add", json!({"item_id": item_id, "content": content}))
    }
    pub fn note_update(id: &str, content: &str) -> Command {
        Command::with_id("note_update", id, json!({ "content": content }))
    }
    pub fn note_delete(id: &str) -> Command {
        Command::with_id("note_delete", id, json!({}))
    }

    // ---- projects (sync_projects.md) ----
    pub fn project_add(args: Value) -> Command {
        Command::create("project_add", args)
    }
    pub fn project_update(id: &str, fields: Value) -> Command {
        Command::with_id("project_update", id, fields)
    }
    /// `project_move`: `parent_id` `None` moves the project to the root.
    pub fn project_move(id: &str, parent_id: Option<&str>) -> Command {
        Command::with_id("project_move", id, json!({ "parent_id": parent_id }))
    }
    /// Deprecated `project_reorder` with `child_order` values, for siblings without
    /// `order_key`s.
    pub fn project_reorder(orders: &[(String, i64)]) -> Command {
        let projects: Vec<Value> = orders
            .iter()
            .map(|(id, o)| json!({"id": id, "child_order": o}))
            .collect();
        Command::new("project_reorder", json!({ "projects": projects }))
    }
    pub fn project_archive(id: &str) -> Command {
        Command::with_id("project_archive", id, json!({}))
    }
    pub fn project_unarchive(id: &str) -> Command {
        Command::with_id("project_unarchive", id, json!({}))
    }
    pub fn project_delete(id: &str) -> Command {
        Command::with_id("project_delete", id, json!({}))
    }

    // ---- sections (sync_sections.md) ----
    pub fn section_add(project_id: &str, name: &str) -> Command {
        Command::create(
            "section_add",
            json!({"project_id": project_id, "name": name}),
        )
    }
    pub fn section_update(id: &str, fields: Value) -> Command {
        Command::with_id("section_update", id, fields)
    }
    pub fn section_delete(id: &str) -> Command {
        Command::with_id("section_delete", id, json!({}))
    }

    // ---- labels (sync_labels.md) ----
    pub fn label_add(args: Value) -> Command {
        Command::create("label_add", args)
    }
    pub fn label_update(id: &str, fields: Value) -> Command {
        Command::with_id("label_update", id, fields)
    }
    pub fn label_delete(id: &str) -> Command {
        Command::with_id("label_delete", id, json!({}))
    }

    // ---- filters (sync_filters.md) ----
    pub fn filter_add(args: Value) -> Command {
        Command::create("filter_add", args)
    }
    pub fn filter_update(id: &str, fields: Value) -> Command {
        Command::with_id("filter_update", id, fields)
    }
    pub fn filter_delete(id: &str) -> Command {
        Command::with_id("filter_delete", id, json!({}))
    }

    // ---- reminders (sync_reminders.md) ----
    pub fn reminder_add(args: Value) -> Command {
        Command::create("reminder_add", args)
    }
    pub fn reminder_update(id: &str, fields: Value) -> Command {
        Command::with_id("reminder_update", id, fields)
    }
    pub fn reminder_delete(id: &str) -> Command {
        Command::with_id("reminder_delete", id, json!({}))
    }

    /// Offline quick add: the raw text is kept and submitted to `/api/v1/tasks/quick` later.
    pub fn quick_add(text: &str) -> Command {
        Command::create(QUICK_ADD, json!({ "text": text }))
    }

    pub fn is_local_only(&self) -> bool {
        self.kind == QUICK_ADD
    }

    /// Replace every occurrence of `from` as an ID in this command (args values, nested
    /// arrays such as `items[].id`, and object keys such as `ids_to_orders`).
    pub fn remap_id(&mut self, from: &str, to: &str) -> bool {
        let mut changed = remap_value(&mut self.args, from, to);
        if self.temp_id.as_deref() == Some(from) {
            self.temp_id = Some(to.to_owned());
            changed = true;
        }
        changed
    }
}

/// Where `item_move` puts a task.
#[derive(Debug, Clone, PartialEq)]
pub enum MoveTarget {
    Parent(String),
    Section(String),
    Project(String),
}

fn remap_value(v: &mut Value, from: &str, to: &str) -> bool {
    match v {
        Value::String(s) if s == from => {
            *s = to.to_owned();
            true
        }
        Value::Array(a) => {
            let mut changed = false;
            for x in a {
                changed |= remap_value(x, from, to);
            }
            changed
        }
        Value::Object(m) => {
            let mut changed = false;
            if m.contains_key(from)
                && let Some(x) = m.remove(from)
            {
                m.insert(to.to_owned(), x);
                changed = true;
            }
            for (_, x) in m.iter_mut() {
                changed |= remap_value(x, from, to);
            }
            changed
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_like_docs() {
        let c = Command::item_close("2995104339");
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["type"], "item_close");
        assert_eq!(v["args"], json!({"id": "2995104339"}));
        assert!(v.get("temp_id").is_none());
        let a = Command::item_add(json!({"content": "Buy Milk"}));
        assert!(a.temp_id.is_some());
    }

    #[test]
    fn remap_nested() {
        let mut c = Command::item_reorder(&[("tmp".into(), 1), ("x".into(), 2)]);
        assert!(c.remap_id("tmp", "real"));
        assert_eq!(c.args["items"][0]["id"], "real");
        let mut d = Command::new(
            "item_update_day_orders",
            json!({"ids_to_orders": {"tmp": 1}}),
        );
        assert!(d.remap_id("tmp", "real"));
        assert_eq!(d.args["ids_to_orders"]["real"], 1);
        let mut e = Command::item_update("x", json!({"content": "tmp"}));
        // Content equal to an id is also rewritten; temp ids are UUIDs so this cannot clash.
        assert!(e.remap_id("tmp", "real"));
    }
}
