//! What each view shows, as a flat list of rows (`DESIGN.md` 7.1: one model of row items
//! with a kind). Plain Rust over the store, without GTK, so the view rules are unit-tested.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use chrono::{Datelike, NaiveDate, NaiveTime};
use tick_core::Now;
use tick_core::model::Project;
use tick_core::store::{FilterGroupResult, Result, Store, Task, TaskNode};

use crate::format::{self, DateSettings};
use crate::i18n::{fill, gettext, ncount, pgettext};

/// Days the Upcoming view looks ahead.
const UPCOMING_DAYS: u32 = 730;
/// Upcoming always shows this many days, even when they are empty (4.7).
const UPCOMING_FIXED_DAYS: usize = 7;
/// Sub-task indentation is capped visually at this depth (4.4).
pub const MAX_DEPTH: usize = 4;
/// Sorts full-day tasks after timed ones on the same day.
const LAST_TIME: NaiveTime = NaiveTime::from_hms_opt(23, 59, 59).unwrap();

/// A list the user can open from the sidebar.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ViewId {
    Inbox,
    Today,
    Upcoming,
    Project(String),
    /// By label id.
    Label(String),
    /// By filter id.
    Filter(String),
}

impl ViewId {
    /// Value for the `last-view` setting.
    pub fn to_setting(&self) -> String {
        match self {
            ViewId::Inbox => "inbox".into(),
            ViewId::Today => "today".into(),
            ViewId::Upcoming => "upcoming".into(),
            ViewId::Project(id) => format!("project:{id}"),
            ViewId::Label(id) => format!("label:{id}"),
            ViewId::Filter(id) => format!("filter:{id}"),
        }
    }

    pub fn from_setting(s: &str) -> Option<ViewId> {
        Some(match s.split_once(':') {
            None => match s {
                "inbox" => ViewId::Inbox,
                "today" => ViewId::Today,
                "upcoming" => ViewId::Upcoming,
                _ => return None,
            },
            Some(("project", id)) => ViewId::Project(id.into()),
            Some(("label", id)) => ViewId::Label(id.into()),
            Some(("filter", id)) => ViewId::Filter(id.into()),
            Some(_) => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ViewHeader {
    pub title: String,
    /// Date (Today) or nothing.
    pub subtitle: Option<String>,
    /// Filter query, shown monospace before the count.
    pub query: Option<String>,
    pub count: usize,
    /// Project color dot (API color name).
    pub color: Option<String>,
    /// Icon before the title (labels).
    pub icon: Option<&'static str>,
    /// Member names of a shared project (up to 3 avatars).
    pub avatars: Vec<String>,
    /// Where server filter results come from, and whether they are loading (4.22). Shown
    /// under the subtitle, as part of the header.
    pub source: Option<(String, bool)>,
    /// The project title is being renamed in place (4.2).
    pub editing: bool,
    /// The title can be renamed in place (project views).
    pub renamable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SectionHeader {
    pub id: String,
    pub name: String,
    pub count: usize,
    pub collapsed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DayHeader {
    pub key: String,
    /// Day of month; `None` for the Overdue group.
    pub number: Option<u32>,
    /// "Overdue", "Today", "Tomorrow" or a weekday.
    pub label: String,
    /// Dimmed text after the label: weekday (Today, Tomorrow) or the count (Overdue).
    pub dim: Option<String>,
    /// Directly under the view header: no space above (the header has its own).
    pub first: bool,
    /// Directly under a month header, which is its heading: no space above, tighter.
    pub after_month: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EmptyState {
    pub icon: &'static str,
    pub title: String,
    pub description: String,
    /// A button under the text: label and the filter to edit ("Edit Filter", 4.22).
    pub edit_filter: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DueState {
    Overdue,
    Today,
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetaDue {
    pub text: String,
    pub state: DueState,
    pub recurring: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetaProject {
    /// "Vacation" or "Vacation / Packing".
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskRow {
    /// Unique row key (a task can appear in two filter groups).
    pub key: String,
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    /// 1 (urgent) to 4.
    pub p: u8,
    pub depth: usize,
    pub has_children: bool,
    /// The view has parents: keep an empty 44 px chevron slot so trailing avatars align.
    pub chevron_slot: bool,
    pub collapsed: bool,
    /// Ring filled: completing (hold) or completed-pending (recurring, waiting for sync).
    pub checked: bool,
    /// Non-recurring completion hold: title struck through and dimmed (4.4).
    pub completing: bool,
    /// The completed row fades out (after the hold).
    pub fading: bool,
    /// Recurring task closed locally, waiting for the server's next date.
    pub completed_pending: bool,
    pub recurring: bool,
    pub due: Option<MetaDue>,
    /// Deadline text and whether it is overdue.
    pub deadline: Option<(String, bool)>,
    /// (done, total) sub-tasks.
    pub subtasks: Option<(usize, usize)>,
    pub comments: usize,
    pub labels: Vec<String>,
    pub project: Option<MetaProject>,
    pub assignee: Option<String>,
    /// The server rejected a change to this task.
    pub failed: Option<String>,
}

impl TaskRow {
    /// The meta line read as text, for the row's accessible description (4.4).
    pub fn meta_description(&self) -> String {
        let mut parts = Vec::new();
        if self.completed_pending {
            parts.push(gettext("Completing"));
        }
        if let Some(d) = &self.due {
            parts.push(fill(&gettext("Due {date}"), &[("date", &d.text)]));
        }
        if let Some((d, _)) = &self.deadline {
            parts.push(fill(&gettext("deadline {date}"), &[("date", d)]));
        }
        if let Some((done, total)) = self.subtasks {
            parts.push(fill(
                &ncount("{done} of {n} sub-task", "{done} of {n} sub-tasks", total),
                &[("done", &done.to_string())],
            ));
        }
        if self.comments > 0 {
            parts.push(ncount("{n} comment", "{n} comments", self.comments));
        }
        if !self.labels.is_empty() {
            parts.push(fill(
                &gettext("labels {names}"),
                &[("names", &self.labels.join(", "))],
            ));
        }
        if let Some(p) = &self.project {
            parts.push(fill(&gettext("project {name}"), &[("name", &p.name)]));
        }
        if let Some(a) = &self.assignee {
            parts.push(fill(&gettext("Assigned to {name}"), &[("name", a)]));
        }
        if let Some(f) = &self.failed {
            parts.push(fill(&gettext("Not saved: {message}"), &[("message", f)]));
        }
        // Translators: separator between the parts of a task's meta line read aloud.
        parts.join(&pgettext("list separator", ", "))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Row {
    ViewHeader(ViewHeader),
    SectionHeader(SectionHeader),
    /// A comma-separated part of a filter query (non-collapsible heading).
    GroupHeader {
        key: String,
        title: String,
    },
    DayHeader(DayHeader),
    MonthHeader {
        key: String,
        title: String,
    },
    Task(Box<TaskRow>),
    /// "Add task": opens quick add with `prefill`.
    Ghost {
        key: String,
        prefill: String,
    },
    Empty(EmptyState),
    Loading {
        key: String,
        text: String,
    },
    /// Where server filter results come from (4.22).
    SourceLine {
        key: String,
        text: String,
        spinner: bool,
    },
    /// The inline section name editor (4.24): a new section (`id` `None`) or a rename.
    SectionEditor {
        key: String,
        id: Option<String>,
        name: String,
    },
}

impl Row {
    /// Identity across refreshes: an unchanged key keeps the row object (and selection).
    pub fn key(&self) -> String {
        match self {
            Row::ViewHeader(_) => "header".into(),
            Row::SectionHeader(s) => format!("section:{}", s.id),
            Row::GroupHeader { key, .. }
            | Row::MonthHeader { key, .. }
            | Row::Ghost { key, .. }
            | Row::Loading { key, .. }
            | Row::SourceLine { key, .. }
            | Row::SectionEditor { key, .. } => key.clone(),
            Row::DayHeader(d) => d.key.clone(),
            Row::Task(t) => t.key.clone(),
            Row::Empty(_) => "empty".into(),
        }
    }

    pub fn task(&self) -> Option<&TaskRow> {
        match self {
            Row::Task(t) => Some(t),
            _ => None,
        }
    }
}

/// UI state the rows depend on.
#[derive(Debug, Default)]
pub struct UiState {
    pub collapsed_sections: HashSet<String>,
    /// Tasks in the completion hold (non-recurring).
    pub completing: HashSet<String>,
    pub fading: HashSet<String>,
    /// Task id -> server message of a rejected change.
    pub failed: HashMap<String, String>,
    pub offline: bool,
    /// Filter queries whose server result is being fetched.
    pub filter_loading: HashSet<String>,
    /// Filter queries the server refused.
    pub filter_rejected: HashSet<String>,
    /// The inline section editor (4.24), if open.
    pub section_edit: Option<SectionEdit>,
    /// The project title is being renamed in place (4.2).
    pub renaming_title: bool,
}

/// An open inline section editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionEdit {
    pub project_id: String,
    /// The section being renamed; `None` for a new one.
    pub id: Option<String>,
}

/// A view's rows plus what the window needs around them.
#[derive(Debug, Clone, PartialEq)]
pub struct Content {
    pub title: String,
    pub rows: Vec<Row>,
    pub task_count: usize,
    /// Quick-add prefill for this view (`#Project `).
    pub prefill: String,
    /// The filter query when part of it needs the server (`Engine::fetch_filter`).
    pub server_query: Option<String>,
    /// Whether the view still exists (a deleted project falls back to Today).
    pub exists: bool,
}

/// Escape a name for quick-add text (`#My\ Project`).
pub fn quick_add_name(name: &str) -> String {
    name.replace(' ', "\\ ")
}

/// API color name as a CSS class suffix; unknown names render as `charcoal` (7.4 pitfall 11).
pub fn color_class(color: &str) -> String {
    const KNOWN: [&str; 20] = [
        "berry_red",
        "red",
        "orange",
        "yellow",
        "olive_green",
        "lime_green",
        "green",
        "mint_green",
        "teal",
        "sky_blue",
        "light_blue",
        "blue",
        "grape",
        "violet",
        "lavender",
        "magenta",
        "salmon",
        "charcoal",
        "grey",
        "taupe",
    ];
    let c = if KNOWN.contains(&color) {
        color
    } else {
        "charcoal"
    };
    format!("tick-color-{c}")
}

/// Per-refresh lookups shared by every task row.
struct Lookup<'s> {
    store: &'s Store,
    now: Now,
    today: NaiveDate,
    settings: DateSettings,
    projects: HashMap<String, Project>,
    sections: RefCell<HashMap<String, Option<String>>>,
    people: RefCell<HashMap<String, Option<String>>>,
    comments: HashMap<String, usize>,
    subtasks: HashMap<String, (usize, usize)>,
}

/// How task rows render in a view.
#[derive(Clone, Copy)]
struct RowMode<'a> {
    /// Today and Upcoming: the date part of the due item is dropped (4.4).
    time_only: bool,
    /// The project item is hidden in the task's own project view.
    own_project: Option<&'a str>,
    /// Key prefix (filter groups).
    group: Option<usize>,
}

impl<'s> Lookup<'s> {
    fn new(store: &'s Store) -> Result<Lookup<'s>> {
        let now = store.now();
        let user = store.user()?;
        let mut projects: HashMap<String, Project> = store
            .projects_tree()?
            .into_iter()
            .map(|n| (n.project.id.clone(), n.project))
            .collect();
        for p in store.archived_projects()? {
            projects.insert(p.id.clone(), p);
        }
        Ok(Lookup {
            store,
            today: now.today(),
            now,
            settings: DateSettings::from_user(user.as_ref()),
            projects,
            sections: RefCell::default(),
            people: RefCell::default(),
            comments: store.comment_counts()?,
            subtasks: store.subtask_counts()?,
        })
    }

    fn section_name(&self, id: &str) -> Option<String> {
        self.sections
            .borrow_mut()
            .entry(id.to_owned())
            .or_insert_with(|| self.store.section(id).ok().flatten().map(|s| s.name))
            .clone()
    }

    fn person(&self, uid: &str) -> Option<String> {
        self.people
            .borrow_mut()
            .entry(uid.to_owned())
            .or_insert_with(|| {
                if let Ok(Some(c)) = self.store.collaborator(uid) {
                    return Some(c.full_name);
                }
                match self.store.user() {
                    Ok(Some(u)) if u.id == uid => Some(u.full_name),
                    _ => None,
                }
            })
            .clone()
    }

    fn task_row(
        &self,
        t: &Task,
        depth: usize,
        has_children: bool,
        mode: RowMode<'_>,
        ui: &UiState,
    ) -> TaskRow {
        let recurring = t.item.due.as_ref().is_some_and(|d| d.is_recurring);
        let completed_pending = t.checked && t.pending;
        let completing = ui.completing.contains(&t.id);
        let due = t.due.and_then(|d| {
            let overdue = d.is_overdue(&self.now);
            let state = if overdue {
                DueState::Overdue
            } else if d.date == self.today && !mode.time_only {
                DueState::Today
            } else {
                DueState::Other
            };
            let time_only = mode.time_only && !overdue;
            format::due(d.date, d.time, self.today, self.settings, time_only).map(|text| MetaDue {
                text,
                state,
                recurring,
            })
        });
        let deadline = t.item.deadline.as_ref().and_then(|dl| {
            let date = NaiveDate::parse_from_str(dl.date.get(..10)?, "%Y-%m-%d").ok()?;
            Some((
                format::date_short(date, self.today, self.settings),
                date < self.today,
            ))
        });
        let subtasks = self
            .subtasks
            .get(&t.id)
            .filter(|(open, done)| open + done > 0)
            .map(|(open, done)| (*done, open + done));
        let project_obj = self.projects.get(&t.project_id);
        let project = if mode.own_project == Some(t.project_id.as_str()) {
            None
        } else {
            project_obj.map(|p| {
                let name = if p.inbox_project {
                    gettext("Inbox")
                } else {
                    p.name.clone()
                };
                let name = match t.section_id.as_deref().and_then(|s| self.section_name(s)) {
                    Some(section) => format!("{name} / {section}"),
                    None => name,
                };
                MetaProject {
                    name,
                    color: p.color.clone(),
                }
            })
        };
        let assignee = match (project_obj, t.item.responsible_uid.as_deref()) {
            (Some(p), Some(uid)) if p.shared => self.person(uid),
            _ => None,
        };
        let description = t
            .description
            .lines()
            .map(strip_markdown)
            .find(|l| !l.is_empty());
        TaskRow {
            key: match mode.group {
                Some(g) => format!("task:{g}:{}", t.id),
                None => format!("task:{}", t.id),
            },
            id: t.id.clone(),
            title: t.content.clone(),
            description,
            p: t.item.p(),
            depth: depth.min(MAX_DEPTH),
            has_children,
            chevron_slot: false,
            collapsed: has_children && t.is_collapsed,
            checked: completed_pending || completing,
            completing: completing && !completed_pending,
            fading: ui.fading.contains(&t.id),
            completed_pending,
            recurring,
            due,
            deadline,
            subtasks,
            comments: self.comments.get(&t.id).copied().unwrap_or(0),
            labels: t.labels.clone(),
            project,
            assignee,
            failed: ui.failed.get(&t.id).cloned(),
        }
    }
}

/// The first line of a description without Markdown markers (4.4).
fn strip_markdown(line: &str) -> String {
    let mut l = line.trim();
    for prefix in ["- ", "* ", "+ ", "> ", "#"] {
        if let Some(rest) = l.strip_prefix(prefix) {
            l = rest.trim_start_matches('#').trim();
        }
    }
    // Ordered list markers ("1. ").
    if let Some((num, rest)) = l.split_once(". ")
        && !num.is_empty()
        && num.chars().all(|c| c.is_ascii_digit())
    {
        l = rest;
    }
    let mut out = String::with_capacity(l.len());
    let mut chars = l.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' | '_' | '`' => {}
            // [text](url) -> text
            '[' => {
                let mut text = String::new();
                for c in chars.by_ref() {
                    if c == ']' {
                        break;
                    }
                    text.push(c);
                }
                if chars.peek() == Some(&'(') {
                    for c in chars.by_ref() {
                        if c == ')' {
                            break;
                        }
                    }
                }
                out.push_str(&text);
            }
            _ => out.push(c),
        }
    }
    out.trim().to_owned()
}

/// Tasks a count includes: completed-pending ones are done as far as the user is concerned.
fn active(tasks: &[Task]) -> usize {
    tasks.iter().filter(|t| !t.checked).count()
}

fn count_nodes(nodes: &[TaskNode]) -> usize {
    nodes
        .iter()
        .map(|n| usize::from(!n.task.checked) + count_nodes(&n.children))
        .sum()
}

fn push_tree(
    rows: &mut Vec<Row>,
    nodes: &[TaskNode],
    depth: usize,
    lk: &Lookup<'_>,
    mode: RowMode<'_>,
    ui: &UiState,
) {
    for n in nodes {
        let has_children = !n.children.is_empty();
        let row = lk.task_row(&n.task, depth, has_children, mode, ui);
        let collapsed = row.collapsed;
        rows.push(Row::Task(Box::new(row)));
        if !collapsed {
            push_tree(rows, &n.children, depth + 1, lk, mode, ui);
        }
    }
}

fn empty(icon: &'static str, title: String, description: String) -> Row {
    Row::Empty(EmptyState {
        icon,
        title,
        description,
        edit_filter: None,
    })
}

fn header(title: &str, count: usize) -> ViewHeader {
    ViewHeader {
        title: title.to_owned(),
        subtitle: None,
        query: None,
        count,
        color: None,
        icon: None,
        avatars: Vec::new(),
        source: None,
        editing: false,
        renamable: false,
    }
}

fn iso(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

/// The detail pane's sub-tasks as compact task rows (4.8): the list row's meta line (due
/// states, deadline, counts, labels) without the description line and the project item, which
/// the parent's crumb already shows.
pub fn subtask_rows(store: &Store, parent: &Task) -> Result<Vec<(Task, TaskRow)>> {
    let lk = Lookup::new(store)?;
    let mode = RowMode {
        time_only: false,
        own_project: Some(parent.project_id.as_str()),
        group: None,
    };
    let ui = UiState::default();
    Ok(store
        .subtasks(&parent.id)?
        .into_iter()
        .map(|t| {
            let has_children = lk.subtasks.contains_key(&t.id);
            let mut row = lk.task_row(&t, 0, has_children, mode, &ui);
            row.description = None;
            row.project = None;
            (t, row)
        })
        .collect())
}

/// Build the rows of `view`.
pub fn build(store: &Store, view: &ViewId, ui: &UiState) -> Result<Content> {
    let lk = Lookup::new(store)?;
    let content = match view {
        ViewId::Inbox => match store.inbox_project_id()? {
            Some(id) => project(&lk, &id, true, ui),
            None => Ok(Content {
                title: gettext("Inbox"),
                rows: vec![Row::ViewHeader(header(&gettext("Inbox"), 0)), inbox_empty()],
                task_count: 0,
                prefill: String::new(),
                server_query: None,
                exists: true,
            }),
        },
        ViewId::Today => today(&lk, ui),
        ViewId::Upcoming => upcoming(&lk, ui),
        ViewId::Project(id) => project(&lk, id, false, ui),
        ViewId::Label(id) => label(&lk, id, ui),
        ViewId::Filter(id) => filter(&lk, id, ui),
    }?;
    Ok(finish(content))
}

/// Layout details that depend on neighbouring rows.
fn finish(mut c: Content) -> Content {
    // A source line right under the header belongs to the header (4.21, 4.22).
    if let Some(Row::SourceLine { .. }) = c.rows.get(1)
        && let Row::SourceLine { text, spinner, .. } = c.rows.remove(1)
        && let Some(Row::ViewHeader(h)) = c.rows.first_mut()
    {
        h.source = Some((text, spinner));
    }
    if let Some(Row::DayHeader(d)) = c.rows.get_mut(1) {
        d.first = true;
    }
    // Trailing avatars line up when some rows have a sub-task chevron (4.4).
    let parents = c
        .rows
        .iter()
        .any(|r| matches!(r, Row::Task(t) if t.has_children));
    for r in &mut c.rows {
        if let Row::Task(t) = r {
            t.chevron_slot =
                parents && !t.has_children && (t.assignee.is_some() || t.failed.is_some());
        }
    }
    c
}

fn inbox_empty() -> Row {
    empty(
        "tick-inbox-symbolic",
        gettext("Inbox is empty"),
        gettext("New tasks land here. Press Q to add one."),
    )
}

fn project(lk: &Lookup<'_>, id: &str, inbox: bool, ui: &UiState) -> Result<Content> {
    let Some(pv) = lk.store.project_view(id)? else {
        return Ok(Content {
            title: String::new(),
            rows: Vec::new(),
            task_count: 0,
            prefill: String::new(),
            server_query: None,
            exists: false,
        });
    };
    let inbox = inbox || pv.project.inbox_project;
    let title = if inbox {
        gettext("Inbox")
    } else {
        pv.project.name.clone()
    };
    let total = count_nodes(&pv.tasks)
        + pv.sections
            .iter()
            .map(|s| count_nodes(&s.tasks))
            .sum::<usize>();
    let mut head = header(&title, total);
    if !inbox {
        head.color = Some(pv.project.color.clone());
        head.renamable = true;
        head.editing = ui.renaming_title;
    }
    if pv.project.shared {
        head.avatars = lk
            .store
            .collaborators(id)?
            .into_iter()
            .take(3)
            .map(|c| c.full_name)
            .collect();
    }
    let prefill = if inbox {
        String::new()
    } else {
        format!("#{} ", quick_add_name(&pv.project.name))
    };
    let mut rows = vec![Row::ViewHeader(head)];
    let any_task = !pv.tasks.is_empty() || pv.sections.iter().any(|s| !s.tasks.is_empty());
    if !any_task && pv.sections.is_empty() {
        rows.push(if inbox {
            inbox_empty()
        } else {
            empty(
                "tick-project-symbolic",
                gettext("No tasks yet"),
                gettext("Press Q to add the first one."),
            )
        });
    } else {
        let mode = RowMode {
            time_only: false,
            own_project: Some(&pv.project.id),
            group: None,
        };
        push_tree(&mut rows, &pv.tasks, 0, lk, mode, ui);
        if !pv.tasks.is_empty() || pv.sections.is_empty() {
            rows.push(Row::Ghost {
                key: "ghost:root".into(),
                prefill: prefill.clone(),
            });
        }
        for s in &pv.sections {
            let collapsed = ui.collapsed_sections.contains(&s.section.id);
            let renaming = ui
                .section_edit
                .as_ref()
                .is_some_and(|e| e.id.as_deref() == Some(s.section.id.as_str()));
            if renaming {
                rows.push(Row::SectionEditor {
                    key: format!("section-editor:{}", s.section.id),
                    id: Some(s.section.id.clone()),
                    name: s.section.name.clone(),
                });
            } else {
                rows.push(Row::SectionHeader(SectionHeader {
                    id: s.section.id.clone(),
                    name: s.section.name.clone(),
                    count: count_nodes(&s.tasks),
                    collapsed,
                }));
            }
            if !collapsed {
                push_tree(&mut rows, &s.tasks, 0, lk, mode, ui);
                let project = if inbox {
                    String::new()
                } else {
                    format!("#{} ", quick_add_name(&pv.project.name))
                };
                rows.push(Row::Ghost {
                    key: format!("ghost:section:{}", s.section.id),
                    prefill: format!("{project}/{} ", quick_add_name(&s.section.name)),
                });
            }
        }
    }
    if ui
        .section_edit
        .as_ref()
        .is_some_and(|e| e.id.is_none() && e.project_id == pv.project.id)
    {
        // A new section's editor goes at the end of the project (4.24), after the empty state.
        rows.push(Row::SectionEditor {
            key: "section-editor:new".into(),
            id: None,
            name: String::new(),
        });
    }
    Ok(Content {
        title,
        rows,
        task_count: total,
        prefill,
        server_query: None,
        exists: true,
    })
}

/// Today order (4.20): `day_order` ascending (the default `-1` included), then timed tasks
/// by time, then full-day ones, then priority (p1 first) and `child_order`.
fn today_order(a: &Task, b: &Task) -> std::cmp::Ordering {
    let time = |t: &Task| t.due.and_then(|d| d.time).unwrap_or(LAST_TIME);
    a.day_order
        .cmp(&b.day_order)
        .then(time(a).cmp(&time(b)))
        .then(b.priority.cmp(&a.priority))
        .then(a.child_order.cmp(&b.child_order))
        .then(a.id.cmp(&b.id))
}

/// Overdue order (4.20): oldest date first; on the same day timed tasks by time, then
/// priority, then `child_order`.
fn overdue_order(a: &Task, b: &Task) -> std::cmp::Ordering {
    let date = |t: &Task| t.due.map(|d| d.date);
    let time = |t: &Task| t.due.and_then(|d| d.time).unwrap_or(LAST_TIME);
    date(a)
        .cmp(&date(b))
        .then(time(a).cmp(&time(b)))
        .then(b.priority.cmp(&a.priority))
        .then(a.child_order.cmp(&b.child_order))
        .then(a.id.cmp(&b.id))
}

fn day_header(lk: &Lookup<'_>, date: NaiveDate) -> DayHeader {
    let diff = (date - lk.today).num_days();
    let (label, dim) = match diff {
        0 => (gettext("Today"), Some(format::weekday_long(date.weekday()))),
        1 => (
            gettext("Tomorrow"),
            Some(format::weekday_long(date.weekday())),
        ),
        _ => (format::weekday_long(date.weekday()), None),
    };
    DayHeader {
        key: format!("day:{}", iso(date)),
        number: Some(date.day()),
        label,
        dim,
        first: false,
        after_month: false,
    }
}

fn overdue_header(count: usize) -> DayHeader {
    DayHeader {
        key: "day:overdue".into(),
        number: None,
        label: gettext("Overdue"),
        dim: Some(count.to_string()),
        first: false,
        after_month: false,
    }
}

fn today(lk: &Lookup<'_>, ui: &UiState) -> Result<Content> {
    let mut tv = lk.store.today()?;
    tv.overdue.sort_by(overdue_order);
    tv.today.sort_by(today_order);
    let count = active(&tv.overdue) + active(&tv.today);
    let title = gettext("Today");
    let mut head = header(&title, count);
    head.subtitle = Some(format::long_date(lk.today, lk.settings));
    let mut rows = vec![Row::ViewHeader(head)];
    let mode = RowMode {
        time_only: true,
        own_project: None,
        group: None,
    };
    if tv.overdue.is_empty() && tv.today.is_empty() {
        rows.push(empty(
            "weather-clear-symbolic",
            gettext("All done for today"),
            gettext("Enjoy the rest of your day."),
        ));
    } else {
        if !tv.overdue.is_empty() {
            rows.push(Row::DayHeader(overdue_header(tv.overdue.len())));
            for t in &tv.overdue {
                rows.push(Row::Task(Box::new(lk.task_row(t, 0, false, mode, ui))));
            }
            rows.push(Row::DayHeader(day_header(lk, lk.today)));
        }
        for t in &tv.today {
            rows.push(Row::Task(Box::new(lk.task_row(t, 0, false, mode, ui))));
        }
        rows.push(Row::Ghost {
            key: "ghost:today".into(),
            prefill: format!("{} ", iso(lk.today)),
        });
    }
    Ok(Content {
        title,
        rows,
        task_count: count,
        prefill: String::new(),
        server_query: None,
        exists: true,
    })
}

fn upcoming(lk: &Lookup<'_>, ui: &UiState) -> Result<Content> {
    let uv = lk.store.upcoming(UPCOMING_DAYS)?;
    let count = active(&uv.overdue) + uv.days.iter().map(|d| active(&d.tasks)).sum::<usize>();
    let title = gettext("Upcoming");
    let mut rows = vec![Row::ViewHeader(header(&title, count))];
    let mode = RowMode {
        time_only: true,
        own_project: None,
        group: None,
    };
    if uv.overdue.is_empty() && uv.days.iter().all(|d| d.tasks.is_empty()) {
        rows.push(empty(
            "x-office-calendar-symbolic",
            gettext("Nothing scheduled"),
            gettext("Tasks with a date show up here, day by day."),
        ));
    } else {
        if !uv.overdue.is_empty() {
            let mut overdue = uv.overdue.clone();
            overdue.sort_by(overdue_order);
            rows.push(Row::DayHeader(overdue_header(overdue.len())));
            for t in &overdue {
                rows.push(Row::Task(Box::new(lk.task_row(t, 0, false, mode, ui))));
            }
        }
        let mut month = (lk.today.year(), lk.today.month());
        for (i, day) in uv.days.iter().enumerate() {
            if i >= UPCOMING_FIXED_DAYS && day.tasks.is_empty() {
                continue;
            }
            let m = (day.date.year(), day.date.month());
            if m != month {
                month = m;
                rows.push(Row::MonthHeader {
                    key: format!("month:{}-{:02}", m.0, m.1),
                    title: format::month_header(day.date, lk.today),
                });
            }
            let mut header = day_header(lk, day.date);
            header.after_month = matches!(rows.last(), Some(Row::MonthHeader { .. }));
            rows.push(Row::DayHeader(header));
            let mut tasks = day.tasks.clone();
            tasks.sort_by(today_order);
            for t in &tasks {
                rows.push(Row::Task(Box::new(lk.task_row(t, 0, false, mode, ui))));
            }
            // "Add task" only where a day has nothing yet, and under today (4.7): a ghost row
            // after every group would break the calendar rhythm.
            if !day.tasks.is_empty() && day.date != lk.today {
                continue;
            }
            rows.push(Row::Ghost {
                key: format!("ghost:{}", iso(day.date)),
                prefill: format!("{} ", iso(day.date)),
            });
        }
    }
    Ok(Content {
        title,
        rows,
        task_count: count,
        prefill: String::new(),
        server_query: None,
        exists: true,
    })
}

fn label(lk: &Lookup<'_>, id: &str, ui: &UiState) -> Result<Content> {
    let Some(info) = lk.store.labels()?.into_iter().find(|l| l.label.id == id) else {
        return Ok(Content {
            title: String::new(),
            rows: Vec::new(),
            task_count: 0,
            prefill: String::new(),
            server_query: None,
            exists: false,
        });
    };
    let name = info.label.name;
    let tasks = lk.store.label_view(&name)?;
    let mut head = header(&name, active(&tasks));
    head.icon = Some("tick-label-symbolic");
    let mut rows = vec![Row::ViewHeader(head)];
    if tasks.is_empty() {
        rows.push(empty(
            "tick-label-symbolic",
            fill(&gettext("No tasks with @{name}"), &[("name", &name)]),
            fill(
                &gettext("Add @{name} to a task to see it here."),
                &[("name", &name)],
            ),
        ));
    }
    let mode = RowMode {
        time_only: false,
        own_project: None,
        group: None,
    };
    for t in &tasks {
        let mut row = lk.task_row(t, 0, false, mode, ui);
        // The view already names the label (4.4): show only the others.
        row.labels.retain(|l| *l != name);
        rows.push(Row::Task(Box::new(row)));
    }
    Ok(Content {
        title: name.clone(),
        rows,
        task_count: active(&tasks),
        prefill: format!("@{} ", quick_add_name(&name)),
        server_query: None,
        exists: true,
    })
}

fn filter(lk: &Lookup<'_>, id: &str, ui: &UiState) -> Result<Content> {
    let Some(f) = lk.store.filters()?.into_iter().find(|f| f.id == id) else {
        return Ok(Content {
            title: String::new(),
            rows: Vec::new(),
            task_count: 0,
            prefill: String::new(),
            server_query: None,
            exists: false,
        });
    };
    let groups = lk.store.filter_view(&f.query)?;
    let multi = groups.len() > 1;
    let mut body = Vec::new();
    let mut count = 0;
    let mut server_query = None;
    let mut all_local = true;
    let mut any_rows = false;
    let rejected = ui.filter_rejected.contains(&f.query);
    let loading = ui.filter_loading.contains(&f.query);
    for (g, group) in groups.iter().enumerate() {
        if multi {
            body.push(Row::GroupHeader {
                key: format!("group:{g}"),
                title: group.query.clone(),
            });
        }
        let mode = RowMode {
            time_only: false,
            own_project: None,
            group: multi.then_some(g),
        };
        match &group.result {
            FilterGroupResult::Local(tasks) => {
                count += active(tasks);
                any_rows |= !tasks.is_empty();
                for t in tasks {
                    body.push(Row::Task(Box::new(lk.task_row(t, 0, false, mode, ui))));
                }
            }
            FilterGroupResult::Server { cached, .. } => {
                all_local = false;
                server_query = Some(f.query.clone());
                match cached {
                    Some(tasks) => {
                        let fetched = lk
                            .store
                            .filter_fetched_at(&group.query)?
                            .and_then(|at| format::local_time_of(&at, &lk.now, lk.settings))
                            .unwrap_or_default();
                        let (text, spinner) = if ui.offline {
                            (
                                fill(
                                    &gettext("Offline. Showing results from {time}."),
                                    &[("time", &fetched)],
                                ),
                                false,
                            )
                        } else if loading {
                            (gettext("Updating"), true)
                        } else {
                            (
                                fill(
                                    &gettext("Results from Todoist, updated {time}"),
                                    &[("time", &fetched)],
                                ),
                                false,
                            )
                        };
                        body.push(Row::SourceLine {
                            key: format!("source:{g}"),
                            text,
                            spinner,
                        });
                        count += active(tasks);
                        for t in tasks {
                            body.push(Row::Task(Box::new(lk.task_row(t, 0, false, mode, ui))));
                        }
                    }
                    None if rejected => body.push(Row::Empty(EmptyState {
                        icon: "dialog-warning-symbolic",
                        title: gettext("Todoist couldn't read this filter"),
                        description: gettext("Check the query and try again."),
                        edit_filter: Some(f.id.clone()),
                    })),
                    None if ui.offline => body.push(empty(
                        "network-offline-symbolic",
                        gettext("Can't show this filter offline"),
                        gettext("Tick can't evaluate part of this query on this computer. Connect to see results."),
                    )),
                    None => body.push(Row::Loading {
                        key: format!("loading:{g}"),
                        text: gettext("Loading results"),
                    }),
                }
            }
        }
    }
    let mut head = header(&f.name, count);
    head.query = Some(f.query.clone());
    let mut rows = vec![Row::ViewHeader(head)];
    if !any_rows && all_local {
        rows.push(empty(
            "tick-filter-symbolic",
            gettext("No matching tasks"),
            gettext("Nothing matches this filter right now."),
        ));
    } else {
        rows.extend(body);
    }
    Ok(Content {
        title: f.name.clone(),
        rows,
        task_count: count,
        prefill: String::new(),
        server_query,
        exists: true,
    })
}

// ---- sidebar ----

#[derive(Debug, Clone, PartialEq)]
pub struct SidebarProject {
    pub id: String,
    pub name: String,
    pub color: String,
    pub depth: usize,
    pub count: usize,
    pub shared: bool,
    pub collapsed: bool,
    pub children: Vec<SidebarProject>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SidebarEntry {
    pub view: ViewId,
    pub name: String,
    pub count: usize,
    /// Project color for project favorites.
    pub color: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SidebarData {
    pub inbox: usize,
    pub today: usize,
    pub favorites: Vec<SidebarEntry>,
    pub projects: Vec<SidebarProject>,
    pub labels: Vec<SidebarEntry>,
    pub filters: Vec<SidebarEntry>,
}

/// Sidebar contents with counts (4.1). Counts exclude completed-pending tasks.
pub fn sidebar(store: &Store) -> Result<SidebarData> {
    let tree = store.projects_tree()?;
    let inbox_id = store.inbox_project_id()?;
    let today_count = store.today_count()?;
    let mut inbox = 0;
    let mut favorites = Vec::new();
    // Rebuild the tree from the flattened, depth-annotated list.
    let mut roots: Vec<SidebarProject> = Vec::new();
    for node in &tree {
        let p = &node.project;
        if Some(&p.id) == inbox_id.as_ref() || p.inbox_project {
            inbox = node.active_count;
            continue;
        }
        if p.is_favorite {
            favorites.push(SidebarEntry {
                view: ViewId::Project(p.id.clone()),
                name: p.name.clone(),
                count: node.active_count,
                color: Some(p.color.clone()),
            });
        }
        let entry = SidebarProject {
            id: p.id.clone(),
            name: p.name.clone(),
            color: p.color.clone(),
            depth: node.depth,
            count: node.active_count,
            shared: p.shared,
            collapsed: p.is_collapsed,
            children: Vec::new(),
        };
        // Walk down the last children to the parent depth.
        let mut level = &mut roots;
        for _ in 0..node.depth {
            if level.is_empty() {
                break;
            }
            let last = level.len() - 1;
            level = &mut level[last].children;
        }
        level.push(entry);
    }
    let mut labels = Vec::new();
    for l in store.labels()? {
        let e = SidebarEntry {
            view: ViewId::Label(l.label.id.clone()),
            name: l.label.name.clone(),
            count: l.active_count,
            color: None,
        };
        if l.label.is_favorite {
            favorites.push(e.clone());
        }
        labels.push(e);
    }
    let mut filters = Vec::new();
    let saved = store.filters()?;
    let queries: Vec<&str> = saved.iter().map(|f| f.query.as_str()).collect();
    // One pass over the tasks for every filter.
    let counts = store.filter_counts(&queries)?;
    for (f, count) in saved.iter().zip(counts) {
        let e = SidebarEntry {
            view: ViewId::Filter(f.id.clone()),
            name: f.name.clone(),
            count,
            color: None,
        };
        if f.is_favorite {
            favorites.push(e.clone());
        }
        filters.push(e);
    }
    Ok(SidebarData {
        inbox,
        today: today_count,
        favorites,
        projects: roots,
        labels,
        filters,
    })
}

#[cfg(test)]
mod tests;
