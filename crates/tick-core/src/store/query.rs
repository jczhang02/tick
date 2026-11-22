//! Read queries used by the UI.

use std::collections::{HashMap, HashSet};

use chrono::{Duration, NaiveDate};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;

use super::{Result, Store, meta_get};
use crate::clock::{LocalDue, Now, resolve_due_date};
use crate::filter::{self, EvalContext, EvalTask, Group};
use crate::model::{
    Collaborator, Filter, Item, Label, Note, Project, Reminder, Section, User, UserSettings,
};

/// Sibling order: `order_key` (byte-wise, the docs' "lexicographic") when present, otherwise
/// `child_order`. The docs say keys may be `null` for objects "not yet migrated"; those sort
/// after keyed siblings by `child_order`.
const SIBLING_ORDER: &str = "order_key IS NULL, order_key, child_order, id";

const ACTIVE: &str = "local_deleted = 0 AND checked = 0";

/// Rows a view shows: active tasks plus recurring tasks closed locally whose `item_close` the
/// server has not answered yet ("completed-pending", `DESIGN.md` 4.3). The server moves such a
/// task to its next date and reopens it, so hiding it would make the row blink out and back.
const VISIBLE: &str =
    "local_deleted = 0 AND (checked = 0 OR (local_pending = 1 AND due_recurring = 1))";

/// A task as the UI sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    pub item: Item,
    /// A queued command changed this task and the server has not confirmed it yet.
    pub pending: bool,
    /// The due date in local time.
    pub due: Option<LocalDue>,
}

impl std::ops::Deref for Task {
    type Target = Item;
    fn deref(&self) -> &Item {
        &self.item
    }
}

/// A task with its sub-tasks.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskNode {
    pub task: Task,
    pub children: Vec<TaskNode>,
}

impl TaskNode {
    /// Depth-first (depth, task) pairs, as an indented list shows them.
    pub fn flatten(nodes: &[TaskNode]) -> Vec<(usize, &Task)> {
        fn walk<'a>(nodes: &'a [TaskNode], depth: usize, out: &mut Vec<(usize, &'a Task)>) {
            for n in nodes {
                out.push((depth, &n.task));
                walk(&n.children, depth + 1, out);
            }
        }
        let mut out = Vec::new();
        walk(nodes, 0, &mut out);
        out
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SectionGroup {
    pub section: Section,
    pub tasks: Vec<TaskNode>,
}

/// A project's tasks: those without a section first, then each section in order.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectView {
    pub project: Project,
    pub tasks: Vec<TaskNode>,
    pub sections: Vec<SectionGroup>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TodayView {
    pub overdue: Vec<Task>,
    pub today: Vec<Task>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DayGroup {
    pub date: NaiveDate,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UpcomingView {
    pub overdue: Vec<Task>,
    /// One entry per day from today, empty days included.
    pub days: Vec<DayGroup>,
}

/// One comma-separated part of a filter view.
#[derive(Debug, Clone, PartialEq)]
pub enum FilterGroupResult {
    /// Evaluated locally.
    Local(Vec<Task>),
    /// The local evaluator does not support this query; the UI should fetch it with
    /// `GET /api/v1/tasks/filter` (see `Engine::fetch_filter`). `cached` is the last
    /// server result, if any.
    Server {
        reason: String,
        cached: Option<Vec<Task>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct FilterGroupView {
    pub query: String,
    pub result: FilterGroupResult,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectNode {
    pub project: Project,
    pub depth: usize,
    pub has_children: bool,
    /// Active tasks directly in this project.
    pub active_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LabelInfo {
    pub label: Label,
    pub active_count: usize,
}

/// A completed task from the history cache.
#[derive(Debug, Clone, PartialEq)]
pub struct CompletedTask {
    pub item: Item,
    pub completed_at: String,
}

fn parse<T: serde::de::DeserializeOwned>(raw: &str) -> Result<T> {
    Ok(serde_json::from_str(raw)?)
}

fn to_task(raw: &str, pending: bool, now: &Now) -> Result<Task> {
    let item: Item = parse(raw)?;
    let due = item
        .due
        .as_ref()
        .and_then(|d| d.date.as_deref())
        .and_then(|d| resolve_due_date(d, now.offset()));
    Ok(Task { item, pending, due })
}

fn tasks(conn: &Connection, sql: &str, p: impl rusqlite::Params, now: &Now) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare_cached(sql)?;
    let rows = stmt.query_map(p, |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
    let mut out = Vec::new();
    for row in rows {
        let (raw, pending) = row?;
        // One malformed row must not blank a whole view.
        match to_task(&raw, pending != 0, now) {
            Ok(t) => out.push(t),
            Err(e) => eprintln!("tick-core: skipping unreadable task row: {e}"),
        }
    }
    Ok(out)
}

fn objects<T: serde::de::DeserializeOwned>(
    conn: &Connection,
    sql: &str,
    p: impl rusqlite::Params,
) -> Result<Vec<T>> {
    let mut stmt = conn.prepare_cached(sql)?;
    let rows = stmt.query_map(p, |r| r.get::<_, String>(0))?;
    let mut out = Vec::new();
    for raw in rows {
        match parse(&raw?) {
            Ok(v) => out.push(v),
            Err(e) => eprintln!("tick-core: skipping unreadable row: {e}"),
        }
    }
    Ok(out)
}

/// Build trees from tasks already sorted in sibling order. Tasks whose parent is not in
/// the list become roots.
pub fn build_tree(tasks: Vec<Task>) -> Vec<TaskNode> {
    let ids: HashSet<String> = tasks.iter().map(|t| t.id.clone()).collect();
    let mut children: HashMap<String, Vec<Task>> = HashMap::new();
    let mut roots = Vec::new();
    for t in tasks {
        match t.parent_id.clone() {
            Some(p) if ids.contains(&p) && p != t.id => children.entry(p).or_default().push(t),
            _ => roots.push(t),
        }
    }
    fn attach(t: Task, children: &mut HashMap<String, Vec<Task>>, depth: usize) -> TaskNode {
        let kids = if depth > 64 {
            Vec::new()
        } else {
            children.remove(&t.id).unwrap_or_default()
        };
        TaskNode {
            children: kids
                .into_iter()
                .map(|k| attach(k, children, depth + 1))
                .collect(),
            task: t,
        }
    }
    roots
        .into_iter()
        .map(|t| attach(t, &mut children, 0))
        .collect()
}

fn by_due(a: &Task, b: &Task) -> std::cmp::Ordering {
    let ka = a.due.map(|d| d.sort_key());
    let kb = b.due.map(|d| d.sort_key());
    ka.cmp(&kb)
        .then(a.day_order.cmp(&b.day_order))
        .then(b.priority.cmp(&a.priority))
        .then(a.child_order.cmp(&b.child_order))
        .then(a.id.cmp(&b.id))
}

impl Store {
    // ---- single objects ----

    pub fn task(&self, id: &str) -> Result<Option<Task>> {
        let now = self.now();
        Ok(tasks(
            &self.conn,
            "SELECT raw, local_pending FROM items WHERE id = ?1 AND local_deleted = 0",
            [id],
            &now,
        )?
        .pop())
    }

    /// Direct active sub-tasks of a task, in order.
    pub fn subtasks(&self, id: &str) -> Result<Vec<Task>> {
        let now = self.now();
        tasks(
            &self.conn,
            &format!(
                "SELECT raw, local_pending FROM items WHERE parent_id = ?1 AND {ACTIVE}
                 ORDER BY {SIBLING_ORDER}"
            ),
            [id],
            &now,
        )
    }

    pub fn project(&self, id: &str) -> Result<Option<Project>> {
        Ok(objects(
            &self.conn,
            "SELECT raw FROM projects WHERE id = ?1 AND local_deleted = 0",
            [id],
        )?
        .pop())
    }

    pub fn section(&self, id: &str) -> Result<Option<Section>> {
        Ok(objects(
            &self.conn,
            "SELECT raw FROM sections WHERE id = ?1 AND local_deleted = 0",
            [id],
        )?
        .pop())
    }

    pub fn user(&self) -> Result<Option<User>> {
        meta_get(&self.conn, "user")?
            .map(|raw| parse(&raw))
            .transpose()
    }

    pub fn user_settings(&self) -> Result<Option<UserSettings>> {
        meta_get(&self.conn, "user_settings")?
            .map(|raw| parse(&raw))
            .transpose()
    }

    pub fn inbox_project_id(&self) -> Result<Option<String>> {
        super::local::inbox_id(&self.conn)
    }

    // ---- views ----

    /// The Inbox project view.
    pub fn inbox(&self) -> Result<Option<ProjectView>> {
        match self.inbox_project_id()? {
            Some(id) => self.project_view(&id),
            None => Ok(None),
        }
    }

    /// Today: overdue tasks plus tasks due today (Todoist includes overdue in Today).
    pub fn today(&self) -> Result<TodayView> {
        let now = self.now();
        // `due_day` was computed with the offset of the clock at write time; fixed-zone
        // dues can move by a day if the offset changed since, so prefilter one day wider
        // and decide on the due date resolved now.
        let limit = (now.today() + Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();
        let mut list = tasks(
            &self.conn,
            &format!(
                "SELECT raw, local_pending FROM items
                 WHERE {VISIBLE} AND due_day IS NOT NULL AND due_day <= ?1"
            ),
            [&limit],
            &now,
        )?;
        list.retain(|t| t.due.is_some_and(|d| d.date <= now.today()));
        list.sort_by(by_due);
        let (overdue, today): (Vec<_>, Vec<_>) = list
            .into_iter()
            .partition(|t| t.due.is_some_and(|d| d.date < now.today()));
        Ok(TodayView { overdue, today })
    }

    /// Number of active tasks in Today (overdue included), for a sidebar count. Uses the
    /// stored local due day, so a fixed-zone task can be off by a day after the offset
    /// changed; [`Store::today`] decides exactly.
    pub fn today_count(&self) -> Result<usize> {
        let today = self.now().today().format("%Y-%m-%d").to_string();
        let n: i64 = self.conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM items
                 WHERE {ACTIVE} AND due_day IS NOT NULL AND due_day <= ?1"
            ),
            [today],
            |r| r.get(0),
        )?;
        Ok(n as usize)
    }

    /// Upcoming: overdue tasks, then `days` day groups starting today.
    pub fn upcoming(&self, days: u32) -> Result<UpcomingView> {
        let now = self.now();
        let start = now.today();
        let end = start + Duration::days(i64::from(days));
        let mut list = tasks(
            &self.conn,
            &format!(
                "SELECT raw, local_pending FROM items
                 WHERE {VISIBLE} AND due_day IS NOT NULL AND due_day < ?1"
            ),
            // One day wider, see `today`.
            [(end + Duration::days(1)).format("%Y-%m-%d").to_string()],
            &now,
        )?;
        list.sort_by(by_due);
        let mut overdue = Vec::new();
        let mut groups: Vec<DayGroup> = (0..days)
            .map(|d| DayGroup {
                date: start + Duration::days(i64::from(d)),
                tasks: Vec::new(),
            })
            .collect();
        for t in list {
            let Some(due) = t.due else { continue };
            if due.date < start {
                overdue.push(t);
            } else {
                let idx = (due.date - start).num_days() as usize;
                if let Some(g) = groups.get_mut(idx) {
                    g.tasks.push(t);
                }
            }
        }
        Ok(UpcomingView {
            overdue,
            days: groups,
        })
    }

    /// A project's sections in order with their tasks as trees.
    pub fn project_view(&self, project_id: &str) -> Result<Option<ProjectView>> {
        let now = self.now();
        let Some(project) = self.project(project_id)? else {
            return Ok(None);
        };
        let sections: Vec<Section> = objects(
            &self.conn,
            "SELECT raw FROM sections WHERE project_id = ?1 AND local_deleted = 0
                 AND is_archived = 0
                 ORDER BY order_key IS NULL, order_key, section_order, id",
            [project_id],
        )?;
        let all = tasks(
            &self.conn,
            &format!(
                "SELECT raw, local_pending FROM items WHERE project_id = ?1 AND {VISIBLE}
                 ORDER BY {SIBLING_ORDER}"
            ),
            [project_id],
            &now,
        )?;
        let section_ids: HashSet<&str> = sections.iter().map(|s| s.id.as_str()).collect();
        let mut by_section: HashMap<Option<String>, Vec<Task>> = HashMap::new();
        for t in all {
            let key = t
                .section_id
                .clone()
                .filter(|s| section_ids.contains(s.as_str()));
            by_section.entry(key).or_default().push(t);
        }
        let root = build_tree(by_section.remove(&None).unwrap_or_default());
        let sections = sections
            .into_iter()
            .map(|s| SectionGroup {
                tasks: build_tree(by_section.remove(&Some(s.id.clone())).unwrap_or_default()),
                section: s,
            })
            .collect();
        Ok(Some(ProjectView {
            project,
            tasks: root,
            sections,
        }))
    }

    /// Active tasks with a label (case-insensitive name).
    pub fn label_view(&self, name: &str) -> Result<Vec<Task>> {
        let now = self.now();
        let mut list = tasks(
            &self.conn,
            &format!(
                "SELECT raw, local_pending FROM items WHERE {VISIBLE} AND id IN
                 (SELECT item_id FROM item_labels WHERE name = ?1 COLLATE NOCASE)"
            ),
            [name],
            &now,
        )?;
        list.sort_by(by_due);
        Ok(list)
    }

    fn eval_context(&self) -> Result<EvalContext> {
        let mut ctx = EvalContext {
            me: self.user()?.map(|u| u.id).filter(|s| !s.is_empty()),
            ..Default::default()
        };
        let mut stmt = self
            .conn
            .prepare_cached("SELECT id, name, parent_id FROM projects WHERE local_deleted = 0")?;
        for row in stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })? {
            let (id, name, parent) = row?;
            ctx.projects.insert(id, (name.to_lowercase(), parent));
        }
        let mut stmt = self
            .conn
            .prepare_cached("SELECT id, name FROM sections WHERE local_deleted = 0")?;
        for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
            let (id, name) = row?;
            ctx.sections.insert(id, name.to_lowercase());
        }
        let mut stmt = self
            .conn
            .prepare_cached("SELECT id, full_name FROM collaborators WHERE local_deleted = 0")?;
        for row in stmt.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
        })? {
            let (id, name) = row?;
            ctx.people
                .insert(id, name.unwrap_or_default().to_lowercase());
        }
        if let Some(u) = self.user()? {
            ctx.people.insert(u.id.clone(), u.full_name.to_lowercase());
        }
        Ok(ctx)
    }

    /// Evaluate a filter query locally. Each comma-separated part is a separate group;
    /// parts the local evaluator does not support carry the cached server result.
    pub fn filter_view(&self, query: &str) -> Result<Vec<FilterGroupView>> {
        let now = self.now();
        let groups = filter::parse(query);
        let mut out = Vec::new();
        let mut active: Option<Vec<Task>> = None;
        let mut ctx: Option<EvalContext> = None;
        for g in groups {
            match g {
                Group::Supported { text, expr } => {
                    if active.is_none() {
                        active = Some(tasks(
                            &self.conn,
                            &format!("SELECT raw, local_pending FROM items WHERE {VISIBLE}"),
                            [],
                            &now,
                        )?);
                        ctx = Some(self.eval_context()?);
                    }
                    let (Some(all), Some(ctx)) = (&active, &ctx) else {
                        unreachable!()
                    };
                    let mut hits: Vec<Task> = all
                        .iter()
                        .filter(|t| {
                            filter::eval(
                                &expr,
                                &EvalTask {
                                    item: &t.item,
                                    due: t.due,
                                },
                                ctx,
                                &now,
                            )
                        })
                        .cloned()
                        .collect();
                    hits.sort_by(by_due);
                    out.push(FilterGroupView {
                        query: text,
                        result: FilterGroupResult::Local(hits),
                    });
                }
                Group::Unsupported { text, reason } => {
                    let cached = self.cached_filter_results(&text)?;
                    out.push(FilterGroupView {
                        query: text,
                        result: FilterGroupResult::Server { reason, cached },
                    });
                }
            }
        }
        Ok(out)
    }

    /// The last server result for a filter query, if cached.
    pub fn cached_filter_results(&self, query: &str) -> Result<Option<Vec<Task>>> {
        let now = self.now();
        let ids: Option<String> = self
            .conn
            .query_row(
                "SELECT item_ids FROM filter_cache WHERE query = ?1",
                [query],
                |r| r.get(0),
            )
            .optional()?;
        let Some(ids) = ids else { return Ok(None) };
        let ids: Vec<String> = serde_json::from_str(&ids)?;
        let mut out = Vec::new();
        for id in ids {
            if let Some(t) = tasks(
                &self.conn,
                &format!("SELECT raw, local_pending FROM items WHERE id = ?1 AND {VISIBLE}"),
                [&id],
                &now,
            )?
            .pop()
            {
                out.push(t);
            }
        }
        Ok(Some(out))
    }

    /// When the cached server result of a filter query was fetched (UTC, as stored by
    /// [`Store::cache_filter_results`]), if it is cached.
    pub fn filter_fetched_at(&self, query: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row(
                "SELECT fetched_at FROM filter_cache WHERE query = ?1",
                [query],
                |r| r.get(0),
            )
            .optional()?)
    }

    /// Search active task titles and descriptions. Terms are ANDed; each matches as a
    /// substring (FTS5 trigram index for terms of three or more characters, `LIKE` for
    /// shorter ones, which the trigram tokenizer cannot match).
    pub fn search(&self, text: &str, limit: usize) -> Result<Vec<Task>> {
        let now = self.now();
        let terms: Vec<&str> = text.split_whitespace().collect();
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let long: Vec<&str> = terms
            .iter()
            .copied()
            .filter(|t| t.chars().count() >= 3)
            .collect();
        let short: Vec<&str> = terms
            .iter()
            .copied()
            .filter(|t| t.chars().count() < 3)
            .collect();
        let mut sql = format!("SELECT raw, local_pending FROM items WHERE {ACTIVE}");
        let mut params: Vec<String> = Vec::new();
        if !long.is_empty() {
            let q = long
                .iter()
                .map(|t| format!("\"{}\"", t.replace('"', "\"\"")))
                .collect::<Vec<_>>()
                .join(" AND ");
            params.push(q);
            sql.push_str(&format!(
                " AND rowid IN (SELECT rowid FROM items_fts WHERE items_fts MATCH ?{})",
                params.len()
            ));
        }
        for t in short {
            let escaped = t
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_");
            params.push(format!("%{escaped}%"));
            let n = params.len();
            sql.push_str(&format!(
                " AND (content LIKE ?{n} ESCAPE '\\' OR description LIKE ?{n} ESCAPE '\\')"
            ));
        }
        sql.push_str(&format!(
            " ORDER BY (due_sort IS NULL), due_sort, priority DESC, {SIBLING_ORDER} LIMIT {limit}"
        ));
        tasks(
            &self.conn,
            &sql,
            rusqlite::params_from_iter(params.iter()),
            &now,
        )
    }

    /// Completed history from the cache, newest first. `before` pages by `completed_at`.
    pub fn completed_history(
        &self,
        before: Option<&str>,
        limit: usize,
    ) -> Result<Vec<CompletedTask>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT raw, completed_at FROM completed_items
             WHERE (?1 IS NULL OR completed_at < ?1)
             ORDER BY completed_at DESC, id LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![before, limit as i64], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (raw, completed_at) = row?;
            out.push(CompletedTask {
                item: parse(&raw)?,
                completed_at,
            });
        }
        Ok(out)
    }

    // ---- sidebar ----

    /// Non-archived projects as a flattened tree in display order (Inbox first), with
    /// active task counts.
    pub fn projects_tree(&self) -> Result<Vec<ProjectNode>> {
        self.project_tree_where("is_archived = 0")
    }

    pub fn archived_projects(&self) -> Result<Vec<Project>> {
        objects(
            &self.conn,
            "SELECT raw FROM projects WHERE local_deleted = 0 AND is_archived = 1
             ORDER BY name COLLATE NOCASE",
            [],
        )
    }

    fn project_tree_where(&self, cond: &str) -> Result<Vec<ProjectNode>> {
        let projects: Vec<Project> = objects(
            &self.conn,
            &format!(
                "SELECT raw FROM projects WHERE local_deleted = 0 AND {cond}
                 ORDER BY inbox_project DESC, {SIBLING_ORDER}"
            ),
            [],
        )?;
        let mut counts: HashMap<String, usize> = HashMap::new();
        let mut stmt = self.conn.prepare_cached(&format!(
            "SELECT project_id, COUNT(*) FROM items WHERE {ACTIVE} GROUP BY project_id"
        ))?;
        for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))? {
            let (id, n) = row?;
            counts.insert(id, n as usize);
        }
        let ids: HashSet<&str> = projects.iter().map(|p| p.id.as_str()).collect();
        let mut children: HashMap<&str, Vec<&Project>> = HashMap::new();
        let mut roots = Vec::new();
        for p in &projects {
            match p.parent_id.as_deref() {
                Some(parent) if ids.contains(parent) && parent != p.id => {
                    children.entry(parent).or_default().push(p)
                }
                _ => roots.push(p),
            }
        }
        let mut out = Vec::new();
        fn walk(
            list: &[&Project],
            depth: usize,
            children: &HashMap<&str, Vec<&Project>>,
            counts: &HashMap<String, usize>,
            out: &mut Vec<ProjectNode>,
        ) {
            for p in list {
                let kids = children.get(p.id.as_str());
                out.push(ProjectNode {
                    project: (*p).clone(),
                    depth,
                    has_children: kids.is_some_and(|k| !k.is_empty()),
                    active_count: counts.get(&p.id).copied().unwrap_or(0),
                });
                if let Some(k) = kids
                    && depth < 64
                {
                    walk(k, depth + 1, children, counts, out);
                }
            }
        }
        walk(&roots, 0, &children, &counts, &mut out);
        Ok(out)
    }

    pub fn sections(&self, project_id: &str) -> Result<Vec<Section>> {
        objects(
            &self.conn,
            "SELECT raw FROM sections WHERE project_id = ?1 AND local_deleted = 0
             ORDER BY order_key IS NULL, order_key, section_order, id",
            [project_id],
        )
    }

    /// Personal labels in order, with active task counts.
    pub fn labels(&self) -> Result<Vec<LabelInfo>> {
        let labels: Vec<Label> = objects(
            &self.conn,
            "SELECT raw FROM labels WHERE local_deleted = 0
             ORDER BY order_key IS NULL, order_key, item_order, id",
            [],
        )?;
        let mut counts: HashMap<String, usize> = HashMap::new();
        let mut stmt = self.conn.prepare_cached(&format!(
            "SELECT lower(l.name), COUNT(*) FROM item_labels l JOIN items i ON i.id = l.item_id
             WHERE i.{} AND i.checked = 0 GROUP BY lower(l.name)",
            "local_deleted = 0"
        ))?;
        for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))? {
            let (n, c) = row?;
            counts.insert(n, c as usize);
        }
        Ok(labels
            .into_iter()
            .map(|label| LabelInfo {
                active_count: counts.get(&label.name.to_lowercase()).copied().unwrap_or(0),
                label,
            })
            .collect())
    }

    pub fn filters(&self) -> Result<Vec<Filter>> {
        objects(
            &self.conn,
            "SELECT raw FROM filters WHERE local_deleted = 0
             ORDER BY order_key IS NULL, order_key, item_order, id",
            [],
        )
    }

    /// Active collaborators of a shared project.
    pub fn collaborators(&self, project_id: &str) -> Result<Vec<Collaborator>> {
        objects(
            &self.conn,
            "SELECT c.raw FROM collaborators c JOIN collaborator_states s ON s.user_id = c.id
             WHERE s.project_id = ?1 AND s.state = 'active' AND c.local_deleted = 0
             ORDER BY c.full_name COLLATE NOCASE",
            [project_id],
        )
    }

    pub fn collaborator(&self, user_id: &str) -> Result<Option<Collaborator>> {
        Ok(objects(
            &self.conn,
            "SELECT raw FROM collaborators WHERE id = ?1 AND local_deleted = 0",
            [user_id],
        )?
        .pop())
    }

    /// Comments of a task, oldest first.
    pub fn comments(&self, item_id: &str) -> Result<Vec<Note>> {
        objects(
            &self.conn,
            "SELECT raw FROM notes WHERE item_id = ?1 AND local_deleted = 0
             ORDER BY posted_at, id",
            [item_id],
        )
    }

    pub fn comment_counts(&self) -> Result<HashMap<String, usize>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT item_id, COUNT(*) FROM notes WHERE local_deleted = 0 GROUP BY item_id",
        )?;
        let mut out = HashMap::new();
        for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))? {
            let (id, n) = row?;
            out.insert(id, n as usize);
        }
        Ok(out)
    }

    /// Sub-task counts per parent task id: `(open, done)`. `open` counts active direct
    /// sub-tasks; `done` is the server's `completed_info` count for the parent plus sub-tasks
    /// completed locally that the server has not confirmed yet.
    pub fn subtask_counts(&self) -> Result<HashMap<String, (usize, usize)>> {
        let mut out: HashMap<String, (usize, usize)> = HashMap::new();
        let mut stmt = self.conn.prepare_cached(
            "SELECT parent_id, SUM(checked = 0), SUM(checked = 1 AND local_pending = 1)
             FROM items WHERE local_deleted = 0 AND parent_id IS NOT NULL GROUP BY parent_id",
        )?;
        for row in stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })? {
            let (id, open, done) = row?;
            out.insert(id, (open as usize, done as usize));
        }
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, completed_items FROM completed_info WHERE kind = 'item'
             AND completed_items > 0",
        )?;
        for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))? {
            let (id, n) = row?;
            out.entry(id).or_default().1 += n as usize;
        }
        Ok(out)
    }

    /// Reminders of a task.
    pub fn reminders(&self, item_id: &str) -> Result<Vec<Reminder>> {
        objects(
            &self.conn,
            "SELECT raw FROM reminders WHERE item_id = ?1 AND local_deleted = 0 ORDER BY id",
            [item_id],
        )
    }

    /// All time-based reminders of active tasks (for the background notifier).
    pub fn all_reminders(&self) -> Result<Vec<(Reminder, Task)>> {
        let reminders: Vec<Reminder> = objects(
            &self.conn,
            "SELECT raw FROM reminders WHERE local_deleted = 0 AND kind IN ('absolute', 'relative')
             ORDER BY id",
            [],
        )?;
        let mut out = Vec::new();
        for r in reminders {
            if let Some(t) = self.task(&r.item_id)?
                && !t.checked
            {
                out.push((r, t));
            }
        }
        Ok(out)
    }

    /// Completed counts from `completed_info` (`kind` is `project`, `section` or `item`).
    pub fn completed_count(&self, kind: &str, id: &str) -> Result<i64> {
        Ok(self
            .conn
            .query_row(
                "SELECT completed_items FROM completed_info WHERE kind = ?1 AND id = ?2",
                params![kind, id],
                |r| r.get(0),
            )
            .optional()?
            .unwrap_or(0))
    }

    /// The command that moves task `id` right after its sibling `after` (`None` = first)
    /// within its current list (same project, section and parent).
    ///
    /// When the siblings carry `order_key`s this is one `item_update` with a key between
    /// the new neighbors (what the docs recommend); otherwise it is the deprecated
    /// `item_reorder` renumbering `child_order` for the whole list. Returns `None` when `id`
    /// or `after` is not in the list.
    pub fn reorder_command(
        &self,
        id: &str,
        after: Option<&str>,
    ) -> Result<Option<crate::command::Command>> {
        let now = self.now();
        let Some(task) = self.task(id)? else {
            return Ok(None);
        };
        let siblings: Vec<Task> = tasks(
            &self.conn,
            &format!(
                "SELECT raw, local_pending FROM items WHERE project_id = ?1 AND section_id IS ?2
                 AND parent_id IS ?3 AND id != ?4 AND {ACTIVE} ORDER BY {SIBLING_ORDER}"
            ),
            params![task.project_id, task.section_id, task.parent_id, id],
            &now,
        )?;
        let pos = match after {
            None => 0,
            Some(a) => match siblings.iter().position(|t| t.id == a) {
                Some(i) => i + 1,
                None => return Ok(None),
            },
        };
        if siblings.iter().all(|t| t.order_key.is_some()) {
            let prev = pos
                .checked_sub(1)
                .and_then(|i| siblings.get(i))
                .and_then(|t| t.order_key.as_deref());
            let next = siblings.get(pos).and_then(|t| t.order_key.as_deref());
            let key = crate::order_key::key_between(prev, next);
            return Ok(Some(crate::command::Command::item_update(
                id,
                serde_json::json!({ "order_key": key }),
            )));
        }
        let mut ids: Vec<String> = siblings.into_iter().map(|t| t.item.id).collect();
        ids.insert(pos, id.to_owned());
        let orders: Vec<(String, i64)> = ids
            .into_iter()
            .enumerate()
            .map(|(i, id)| (id, i as i64 + 1))
            .collect();
        Ok(Some(crate::command::Command::item_reorder(&orders)))
    }

    /// Raw JSON of any cached object (debugging and forward compatibility).
    pub fn raw_object(&self, kind: super::Kind, id: &str) -> Result<Option<Value>> {
        super::load_raw(&self.conn, kind, id)
    }
}
