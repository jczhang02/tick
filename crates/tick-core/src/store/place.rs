//! Commands that place a task or a project at a position: drag and drop and its keyboard
//! equivalents (`DESIGN.md` 4.1, 4.4).
//!
//! The docs recommend fractional `order_key`s (`sync_tasks.md` "Update a task",
//! `sync_projects.md` "Update a project"): one `*_update` with a key between the new
//! neighbors. `item_move` and `project_move` put the object last among its new siblings, so a
//! move to a position is a move plus a key. Lists whose siblings lack keys ("not yet
//! migrated") fall back to the deprecated `item_reorder` / `project_reorder` renumbering.

use rusqlite::params;
use serde_json::json;

use super::{Result, Store};
use crate::command::{Command, MoveTarget};
use crate::order_key;
use crate::store::query::Task;

/// The list a task joins: its project, section and parent task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskPlace {
    pub project_id: String,
    pub section_id: Option<String>,
    pub parent_id: Option<String>,
}

impl TaskPlace {
    pub fn of(task: &Task) -> TaskPlace {
        TaskPlace {
            project_id: task.project_id.clone(),
            section_id: task.section_id.clone(),
            parent_id: task.parent_id.clone(),
        }
    }
}

/// Counts a delete confirmation names (`DESIGN.md` 4.25).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DeleteCounts {
    /// Active tasks that go with it.
    pub tasks: usize,
    /// Sub-projects (projects only).
    pub subprojects: usize,
}

impl Store {
    /// Active siblings in `place` in display order, without `exclude`.
    fn siblings(&self, place: &TaskPlace, exclude: &str) -> Result<Vec<(String, Option<String>)>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, order_key FROM items WHERE project_id = ?1 AND section_id IS ?2
             AND parent_id IS ?3 AND id != ?4 AND local_deleted = 0 AND checked = 0
             ORDER BY order_key IS NULL, order_key, child_order, id",
        )?;
        let rows = stmt.query_map(
            params![place.project_id, place.section_id, place.parent_id, exclude],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Whether `ancestor` is `id` or one of its parents.
    fn is_ancestor_or_self(&self, ancestor: &str, id: &str) -> Result<bool> {
        let mut cur = Some(id.to_owned());
        let mut guard = 0;
        while let Some(c) = cur {
            if c == ancestor {
                return Ok(true);
            }
            guard += 1;
            if guard > 64 {
                break;
            }
            cur = self.task(&c)?.and_then(|t| t.parent_id.clone());
        }
        Ok(false)
    }

    /// The commands that put task `id` into `place`, right after its sibling `after`
    /// (`None`: first). Empty when nothing would change or the place is invalid (the task
    /// itself or one of its descendants as parent, an unknown `after`).
    pub fn place_task_commands(
        &self,
        id: &str,
        place: &TaskPlace,
        after: Option<&str>,
    ) -> Result<Vec<Command>> {
        let Some(task) = self.task(id)? else {
            return Ok(Vec::new());
        };
        if let Some(p) = &place.parent_id
            && self.is_ancestor_or_self(id, p)?
        {
            return Ok(Vec::new());
        }
        let siblings = self.siblings(place, id)?;
        let pos = match after {
            None => 0,
            Some(a) => match siblings.iter().position(|(s, _)| s == a) {
                Some(i) => i + 1,
                None => return Ok(Vec::new()),
            },
        };
        let mut out = Vec::new();
        let same = TaskPlace::of(&task) == *place;
        if !same {
            let target = if let Some(p) = &place.parent_id {
                MoveTarget::Parent(p.clone())
            } else if let Some(s) = &place.section_id {
                MoveTarget::Section(s.clone())
            } else {
                MoveTarget::Project(place.project_id.clone())
            };
            out.push(Command::item_move(id, target));
            // item_move makes it the last sibling: nothing more to say when that is the spot.
            if pos == siblings.len() {
                return Ok(out);
            }
        } else {
            // Already there?
            let current = self.siblings(place, "")?;
            let me = current.iter().position(|(s, _)| s == id);
            let before = me
                .and_then(|i| i.checked_sub(1))
                .map(|i| current[i].0.as_str());
            if me.is_some() && before == after {
                return Ok(Vec::new());
            }
        }
        if siblings.iter().all(|(_, k)| k.is_some()) {
            let prev = pos
                .checked_sub(1)
                .and_then(|i| siblings.get(i))
                .and_then(|(_, k)| k.as_deref());
            let next = siblings.get(pos).and_then(|(_, k)| k.as_deref());
            let key = order_key::key_between(prev, next);
            out.push(Command::item_update(id, json!({ "order_key": key })));
        } else {
            let mut ids: Vec<String> = siblings.into_iter().map(|(s, _)| s).collect();
            ids.insert(pos, id.to_owned());
            let orders: Vec<(String, i64)> = ids
                .into_iter()
                .enumerate()
                .map(|(i, s)| (s, i as i64 + 1))
                .collect();
            out.push(Command::item_reorder(&orders));
        }
        Ok(out)
    }

    /// Non-archived projects under `parent` in display order, without `exclude` and the Inbox.
    fn project_siblings(
        &self,
        parent: Option<&str>,
        exclude: &str,
    ) -> Result<Vec<(String, Option<String>)>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, order_key FROM projects WHERE parent_id IS ?1 AND id != ?2
             AND local_deleted = 0 AND is_archived = 0 AND inbox_project = 0
             ORDER BY order_key IS NULL, order_key, child_order, id",
        )?;
        let rows = stmt.query_map(params![parent, exclude], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Whether `ancestor` is project `id` or one of its parents.
    fn is_project_ancestor_or_self(&self, ancestor: &str, id: &str) -> Result<bool> {
        let mut cur = Some(id.to_owned());
        let mut guard = 0;
        while let Some(c) = cur {
            if c == ancestor {
                return Ok(true);
            }
            guard += 1;
            if guard > 64 {
                break;
            }
            cur = self.project(&c)?.and_then(|p| p.parent_id.clone());
        }
        Ok(false)
    }

    /// The commands that put project `id` under `parent` (`None`: a root project), right
    /// after its sibling `after` (`None`: first). Empty when nothing would change or the
    /// parent is the project itself or one of its descendants.
    pub fn place_project_commands(
        &self,
        id: &str,
        parent: Option<&str>,
        after: Option<&str>,
    ) -> Result<Vec<Command>> {
        let Some(project) = self.project(id)? else {
            return Ok(Vec::new());
        };
        if project.inbox_project {
            return Ok(Vec::new());
        }
        if let Some(p) = parent
            && self.is_project_ancestor_or_self(id, p)?
        {
            return Ok(Vec::new());
        }
        let siblings = self.project_siblings(parent, id)?;
        let pos = match after {
            None => 0,
            Some(a) => match siblings.iter().position(|(s, _)| s == a) {
                Some(i) => i + 1,
                None => return Ok(Vec::new()),
            },
        };
        let mut out = Vec::new();
        if project.parent_id.as_deref() != parent {
            out.push(Command::project_move(id, parent));
            if pos == siblings.len() {
                return Ok(out);
            }
        } else {
            let current = self.project_siblings(parent, "")?;
            let me = current.iter().position(|(s, _)| s == id);
            let before = me
                .and_then(|i| i.checked_sub(1))
                .map(|i| current[i].0.as_str());
            if me.is_some() && before == after {
                return Ok(Vec::new());
            }
        }
        if siblings.iter().all(|(_, k)| k.is_some()) {
            let prev = pos
                .checked_sub(1)
                .and_then(|i| siblings.get(i))
                .and_then(|(_, k)| k.as_deref());
            let next = siblings.get(pos).and_then(|(_, k)| k.as_deref());
            let key = order_key::key_between(prev, next);
            out.push(Command::project_update(id, json!({ "order_key": key })));
        } else {
            let mut ids: Vec<String> = siblings.into_iter().map(|(s, _)| s).collect();
            ids.insert(pos, id.to_owned());
            let orders: Vec<(String, i64)> = ids
                .into_iter()
                .enumerate()
                .map(|(i, s)| (s, i as i64 + 1))
                .collect();
            out.push(Command::project_reorder(&orders));
        }
        Ok(out)
    }

    /// What deleting project `id` takes with it: active tasks in it and its sub-projects, and
    /// the number of sub-projects.
    pub fn project_delete_counts(&self, id: &str) -> Result<DeleteCounts> {
        let mut ids = vec![id.to_owned()];
        let mut i = 0;
        while i < ids.len() {
            let mut stmt = self.conn.prepare_cached(
                "SELECT id FROM projects WHERE parent_id = ?1 AND local_deleted = 0",
            )?;
            let kids: Vec<String> = stmt
                .query_map([&ids[i]], |r| r.get(0))?
                .collect::<std::result::Result<_, _>>()?;
            ids.extend(kids);
            i += 1;
        }
        let mut tasks = 0;
        for p in &ids {
            let n: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM items WHERE project_id = ?1 AND local_deleted = 0
                 AND checked = 0",
                [p],
                |r| r.get(0),
            )?;
            tasks += n as usize;
        }
        Ok(DeleteCounts {
            tasks,
            subprojects: ids.len() - 1,
        })
    }

    /// Active tasks in section `id` (deleted with it).
    pub fn section_task_count(&self, id: &str) -> Result<usize> {
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM items WHERE section_id = ?1 AND local_deleted = 0
             AND checked = 0",
            [id],
            |r| r.get(0),
        )?;
        Ok(n as usize)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::{Value, json};

    use super::*;
    use crate::clock::Now;
    use crate::model::SyncResponse;

    fn store(items: Value, projects: Value) -> Store {
        let mut s = Store::open_in_memory().unwrap();
        s.set_clock(Arc::new(|| {
            Now::fixed("2026-11-22T09:00:00+00:00").unwrap()
        }));
        let resp: SyncResponse = serde_json::from_value(json!({
            "full_sync": true, "sync_token": "t",
            "projects": projects, "items": items,
            "sections": [{"id": "s1", "project_id": "p1", "name": "S", "section_order": 1,
                          "order_key": "a0"}],
        }))
        .unwrap();
        s.apply_sync_response(&[], &resp).unwrap();
        s
    }

    fn item(id: &str, key: Option<&str>, section: Option<&str>, parent: Option<&str>) -> Value {
        json!({"id": id, "project_id": "p1", "content": id, "child_order": 1,
               "order_key": key, "section_id": section, "parent_id": parent,
               "checked": false, "is_deleted": false, "priority": 1, "labels": []})
    }

    fn projects() -> Value {
        json!([
            {"id": "in", "name": "Inbox", "inbox_project": true, "child_order": 0},
            {"id": "p1", "name": "Work", "child_order": 1, "order_key": "a0"},
            {"id": "p2", "name": "Home", "child_order": 2, "order_key": "a1"},
            {"id": "p3", "name": "Sub", "child_order": 1, "order_key": "a0", "parent_id": "p2"},
        ])
    }

    fn root() -> TaskPlace {
        TaskPlace {
            project_id: "p1".into(),
            section_id: None,
            parent_id: None,
        }
    }

    #[test]
    fn reorder_within_a_list_is_one_key_update() {
        let s = store(
            json!([
                item("a", Some("a0"), None, None),
                item("b", Some("a1"), None, None),
                item("c", Some("a2"), None, None)
            ]),
            projects(),
        );
        let cmds = s.place_task_commands("c", &root(), Some("a")).unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].kind, "item_update");
        let key = cmds[0].args["order_key"].as_str().unwrap();
        assert!(key > "a0" && key < "a1", "{key}");
        // Already in place: nothing to do.
        assert!(
            s.place_task_commands("b", &root(), Some("a"))
                .unwrap()
                .is_empty()
        );
        // First.
        let cmds = s.place_task_commands("c", &root(), None).unwrap();
        assert!(cmds[0].args["order_key"].as_str().unwrap() < "a0");
    }

    #[test]
    fn moving_to_a_section_is_a_move_plus_a_key() {
        let s = store(
            json!([
                item("a", Some("a0"), None, None),
                item("x", Some("a0"), Some("s1"), None),
                item("y", Some("a1"), Some("s1"), None)
            ]),
            projects(),
        );
        let place = TaskPlace {
            section_id: Some("s1".into()),
            ..root()
        };
        let cmds = s.place_task_commands("a", &place, None).unwrap();
        assert_eq!(cmds[0].kind, "item_move");
        assert_eq!(cmds[0].args["section_id"], "s1");
        assert!(cmds[1].args["order_key"].as_str().unwrap() < "a0");
        // Last: item_move alone puts it there.
        let cmds = s.place_task_commands("a", &place, Some("y")).unwrap();
        assert_eq!(cmds.len(), 1);
    }

    #[test]
    fn nesting_under_a_descendant_is_refused() {
        let s = store(
            json!([
                item("a", Some("a0"), None, None),
                item("b", Some("a0"), None, Some("a"))
            ]),
            projects(),
        );
        let place = TaskPlace {
            parent_id: Some("b".into()),
            ..root()
        };
        assert!(s.place_task_commands("a", &place, None).unwrap().is_empty());
        let place = TaskPlace {
            parent_id: Some("a".into()),
            ..root()
        };
        assert!(s.place_task_commands("a", &place, None).unwrap().is_empty());
    }

    #[test]
    fn lists_without_keys_fall_back_to_reorder() {
        let s = store(
            json!([item("a", None, None, None), item("b", None, None, None)]),
            projects(),
        );
        let cmds = s.place_task_commands("a", &root(), Some("b")).unwrap();
        assert_eq!(cmds[0].kind, "item_reorder");
        assert_eq!(cmds[0].args["items"][0]["id"], "b");
        assert_eq!(cmds[0].args["items"][1]["id"], "a");
    }

    #[test]
    fn projects_reorder_and_nest() {
        let s = store(json!([]), projects());
        let cmds = s.place_project_commands("p2", None, None).unwrap();
        assert_eq!(cmds.len(), 1);
        assert!(cmds[0].args["order_key"].as_str().unwrap() < "a0");
        let cmds = s.place_project_commands("p1", Some("p2"), None).unwrap();
        assert_eq!(cmds[0].kind, "project_move");
        assert_eq!(cmds[0].args["parent_id"], "p2");
        assert!(cmds[1].args["order_key"].as_str().unwrap() < "a0");
        // Into its own sub-project: refused. The Inbox never moves.
        assert!(
            s.place_project_commands("p2", Some("p3"), None)
                .unwrap()
                .is_empty()
        );
        assert!(
            s.place_project_commands("in", None, None)
                .unwrap()
                .is_empty()
        );
        // Out to the root, last.
        let cmds = s.place_project_commands("p3", None, Some("p2")).unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].args["parent_id"], Value::Null);
    }

    #[test]
    fn delete_counts() {
        let s = store(
            json!([
                item("a", Some("a0"), None, None),
                item("x", Some("a0"), Some("s1"), None)
            ]),
            projects(),
        );
        assert_eq!(
            s.project_delete_counts("p2").unwrap(),
            DeleteCounts {
                tasks: 0,
                subprojects: 1
            }
        );
        assert_eq!(s.project_delete_counts("p1").unwrap().tasks, 2);
        assert_eq!(s.section_task_count("s1").unwrap(), 1);
    }
}
