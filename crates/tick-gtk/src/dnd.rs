//! Drag and drop (`DESIGN.md` 4.1, 4.4, 4.6) and its keyboard equivalents.
//!
//! - Task rows drag within a project list: between rows (an accent insertion line inset by
//!   the target depth), into a section (onto its header or its "Add task" row), or onto the
//!   right half of a row to nest under it (the row gets the inset accent outline). Moving
//!   the pointer sideways changes the depth at the line in 32 px steps (4.6), between the
//!   depth of the row below and one level under the row above.
//! - In Upcoming and Today a task dropped into another day's group moves to that day and
//!   keeps its time (4.7, 4.20).
//! - Hovering a collapsed sidebar project for 600 ms while dragging expands it (4.1).
//! - Tasks drop onto a sidebar project to move there ("Moved to ..." with Undo).
//! - Projects drag in the sidebar: the top or bottom third of a project row places it before
//!   or after, the middle third makes it a sub-project.
//! - Keyboard: Alt+Shift+Up/Down/Right/Left for tasks (here) and projects (`manage.rs`).
//!
//! Model refreshes wait while a drag is in progress, so the dragged row keeps its widget.

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;
use tick_core::store::TaskPlace;

use crate::row_object::TickRow;
use crate::rows::{Row, ViewId};
use crate::sidebar::{ProjectMove, SidebarItem};
use crate::window::TickWindow;

/// Drag payload prefixes (a `String` value).
pub const TASK: &str = "tick-task:";
pub const PROJECT: &str = "tick-project:";

/// Sub-task indentation per level (4.4).
const INDENT: f64 = 32.0;
/// Sidebar indentation per level (4.1).
const SIDEBAR_INDENT: f64 = 18.0;
/// Hover time before a collapsed sidebar project opens during a drag (4.1).
const HOVER_EXPAND: std::time::Duration = std::time::Duration::from_millis(600);

thread_local! {
    /// Where the drag in progress started, in window coordinates (4.6 depth steps).
    static ORIGIN: std::cell::Cell<Option<(f64, f64)>> = const { std::cell::Cell::new(None) };
}

/// The depth at an insertion line (4.6): the default for the spot, moved by whole 32 px
/// steps of sideways pointer movement, kept between the depth of the row below (`min`) and
/// one level under the row above (`max`).
pub(crate) fn line_depth(default: usize, dx: f64, min: usize, max: usize) -> Option<usize> {
    if min > max {
        return None;
    }
    // Whole steps only: a little sideways drift during a vertical move changes nothing.
    let steps = (dx / INDENT).trunc() as i64;
    let d = (default as i64 + steps).clamp(min as i64, max as i64);
    Some(d as usize)
}

/// Where a dragged task would land in the task list.
#[derive(Debug, Clone)]
enum TaskTarget {
    /// Between rows: into `place`, after `after`; the line at `y` (list coordinates).
    Place {
        place: TaskPlace,
        after: Option<String>,
        y: f64,
        x: f64,
        width: f64,
    },
    /// Onto a row: nest as its last sub-task.
    Into { parent: String, row: gtk::Widget },
    /// Into another day's group (Upcoming, Today): reschedule to `date`.
    Day {
        date: chrono::NaiveDate,
        y: f64,
        x: f64,
        width: f64,
    },
}

/// Where a dragged project would land in the sidebar.
#[derive(Debug, Clone)]
enum ProjectTarget {
    Place {
        parent: Option<String>,
        after: Option<String>,
        y: f64,
        x: f64,
        width: f64,
    },
    Into {
        parent: String,
        row: gtk::Widget,
    },
}

/// The list row widget (a direct child of `list`) under (x, y), with its bounds. Picked like
/// a click: a list view also keeps row widgets that are not on screen (recycled, or holding
/// focus), whose bounds can overlap the pointer.
fn row_at(list: &gtk::Widget, x: f64, y: f64) -> Option<(gtk::Widget, gtk::graphene::Rect)> {
    let mut w = list.pick(x, y, gtk::PickFlags::DEFAULT)?;
    while w.parent().as_ref() != Some(list) {
        w = w.parent()?;
    }
    let b = w.compute_bounds(list)?;
    Some((w, b))
}

fn row_key(row: &gtk::Widget) -> Option<String> {
    row.first_child()
        .and_downcast::<adw::Bin>()
        .and_then(|b| b.child())
        .map(|w| w.widget_name().to_string())
}

fn payload(v: &glib::Value) -> Option<String> {
    v.get::<String>().ok()
}

impl TickWindow {
    pub(crate) fn setup_dnd(&self) {
        let imp = self.imp();
        // The insertion line: a 6 px dot at its start, then the 2 px line (4.4).
        for line in [imp.drop_line.get(), imp.sidebar_drop_line.get()] {
            line.set_orientation(gtk::Orientation::Horizontal);
            line.set_height_request(6);
            let dot = gtk::Box::builder()
                .width_request(6)
                .height_request(6)
                .valign(gtk::Align::Center)
                .css_classes(["tick-drop-dot"])
                .build();
            let rule = gtk::Box::builder()
                .hexpand(true)
                .valign(gtk::Align::Center)
                .css_classes(["tick-drop-line"])
                .build();
            line.append(&dot);
            line.append(&rule);
        }

        // Task list target.
        let target = gtk::DropTarget::new(glib::Type::STRING, gtk::gdk::DragAction::MOVE);
        target.set_preload(true);
        let weak = self.downgrade();
        target.connect_motion(move |t, x, y| {
            let Some(w) = weak.upgrade() else {
                return gtk::gdk::DragAction::empty();
            };
            let id = t
                .value()
                .and_then(|v| payload(&v))
                .and_then(|p| p.strip_prefix(TASK).map(str::to_owned));
            w.note_drag_pointer(&w.imp().task_list.get().upcast(), x, y);
            match id.and_then(|id| w.task_target(&id, x, y)) {
                Some(target) => {
                    w.show_task_target(&target);
                    gtk::gdk::DragAction::MOVE
                }
                None => {
                    w.hide_drop_feedback();
                    gtk::gdk::DragAction::empty()
                }
            }
        });
        let weak = self.downgrade();
        target.connect_leave(move |_| {
            if let Some(w) = weak.upgrade() {
                w.hide_drop_feedback();
            }
        });
        let weak = self.downgrade();
        target.connect_drop(move |_, v, x, y| {
            let Some(w) = weak.upgrade() else {
                return false;
            };
            w.hide_drop_feedback();
            let Some(id) = payload(v).and_then(|p| p.strip_prefix(TASK).map(str::to_owned)) else {
                return false;
            };
            let Some(target) = w.task_target(&id, x, y) else {
                return false;
            };
            w.drop_task(&id, target);
            true
        });
        imp.task_list.add_controller(target);

        // Sidebar target: tasks onto projects, projects among projects.
        let Some(sb) = imp.sidebar.get() else { return };
        let target = gtk::DropTarget::new(glib::Type::STRING, gtk::gdk::DragAction::MOVE);
        target.set_preload(true);
        let weak = self.downgrade();
        target.connect_motion(move |t, x, y| {
            let Some(w) = weak.upgrade() else {
                return gtk::gdk::DragAction::empty();
            };
            let Some(p) = t.value().and_then(|v| payload(&v)) else {
                return gtk::gdk::DragAction::empty();
            };
            if let Some(sb) = w.imp().sidebar.get() {
                w.note_drag_pointer(&sb.list().clone().upcast(), x, y);
            }
            w.hover_expand(x, y);
            let ok = if p.starts_with(TASK) {
                match w.sidebar_project_at(x, y) {
                    Some((_, row)) => {
                        w.show_into(&row);
                        w.imp().sidebar_drop_line.set_visible(false);
                        true
                    }
                    None => false,
                }
            } else if let Some(id) = p.strip_prefix(PROJECT) {
                match w.project_target(id, x, y) {
                    Some(target) => {
                        w.show_project_target(&target);
                        true
                    }
                    None => false,
                }
            } else {
                false
            };
            if ok {
                gtk::gdk::DragAction::MOVE
            } else {
                w.hide_drop_feedback();
                gtk::gdk::DragAction::empty()
            }
        });
        let weak = self.downgrade();
        target.connect_leave(move |_| {
            if let Some(w) = weak.upgrade() {
                w.hide_drop_feedback();
                w.cancel_hover_expand();
            }
        });
        let weak = self.downgrade();
        target.connect_drop(move |_, v, x, y| {
            let Some(w) = weak.upgrade() else {
                return false;
            };
            w.hide_drop_feedback();
            w.cancel_hover_expand();
            let Some(p) = payload(v) else { return false };
            if let Some(id) = p.strip_prefix(TASK) {
                let Some((project, _)) = w.sidebar_project_at(x, y) else {
                    return false;
                };
                let id = id.to_owned();
                // After the drag finished: the move rebuilds rows.
                glib::idle_add_local_once(move || w.move_task_to(&id, &project, None));
                return true;
            }
            if let Some(id) = p.strip_prefix(PROJECT) {
                let Some(target) = w.project_target(id, x, y) else {
                    return false;
                };
                let id = id.to_owned();
                glib::idle_add_local_once(move || match target {
                    ProjectTarget::Place { parent, after, .. } => {
                        w.place_project(&id, parent.as_deref(), after.as_deref())
                    }
                    ProjectTarget::Into { parent, .. } => {
                        let last = w
                            .store()
                            .projects_tree()
                            .unwrap_or_default()
                            .into_iter()
                            .filter(|n| {
                                n.project.parent_id.as_deref() == Some(parent.as_str())
                                    && n.project.id != id
                            })
                            .map(|n| n.project.id)
                            .next_back();
                        w.place_project(&id, Some(&parent), last.as_deref());
                    }
                });
                return true;
            }
            false
        });
        sb.list().add_controller(target);
    }

    /// A drag started from a task row or a sidebar project: hold model refreshes.
    pub(crate) fn drag_began(&self) {
        self.imp().dragging.set(true);
    }

    /// The drag ended (dropped or cancelled): refresh what waited.
    pub(crate) fn drag_ended(&self) {
        let imp = self.imp();
        imp.dragging.set(false);
        self.hide_drop_feedback();
        self.cancel_hover_expand();
        ORIGIN.with(|o| o.set(None));
        crate::test_mode::clear_drag_icon();
        if imp.refresh_pending.replace(false) {
            self.refresh_rows();
            self.refresh_sidebar();
        }
    }

    /// For the test-mode screenshot of a drag: where the pointer is, in window coordinates.
    fn note_drag_pointer(&self, from: &gtk::Widget, x: f64, y: f64) {
        if !crate::test_mode::enabled() {
            return;
        }
        if let Some(p) = from.compute_point(self, &gtk::graphene::Point::new(x as f32, y as f32)) {
            crate::test_mode::set_drag_pointer(f64::from(p.x()), f64::from(p.y()));
        }
    }

    fn project_of_view(&self) -> Option<String> {
        match self.current_view() {
            ViewId::Project(id) => Some(id),
            ViewId::Inbox => self.store().inbox_project_id().ok().flatten(),
            _ => None,
        }
    }

    fn row_for_key(&self, key: &str) -> Option<TickRow> {
        let store = self.imp().rows.get()?;
        (0..store.n_items())
            .filter_map(|i| store.item(i).and_downcast::<TickRow>())
            .find(|r| r.key() == key)
    }

    /// Whether `id` is `ancestor` or one of its sub-tasks.
    fn within(&self, id: &str, ancestor: &str) -> bool {
        let store = self.store();
        let mut cur = Some(id.to_owned());
        let mut n = 0;
        while let Some(c) = cur {
            if c == ancestor {
                return true;
            }
            n += 1;
            if n > 64 {
                break;
            }
            cur = store
                .task(&c)
                .ok()
                .flatten()
                .and_then(|t| t.parent_id.clone());
        }
        false
    }

    /// The drop target for task `id` at (x, y) in the task list, if the view takes drops
    /// (project lists, the Inbox included; day groups in Upcoming and Today).
    fn task_target(&self, id: &str, x: f64, y: f64) -> Option<TaskTarget> {
        if matches!(self.current_view(), ViewId::Upcoming | ViewId::Today) {
            return self.day_target(id, x, y);
        }
        let project = self.project_of_view()?;
        let list: gtk::Widget = self.imp().task_list.get().upcast();
        let (row_w, b) = row_at(&list, x, y)?;
        let key = row_key(&row_w)?;
        let row = self.row_for_key(&key)?;
        let (bx, by, bw, bh) = (
            f64::from(b.x()),
            f64::from(b.y()),
            f64::from(b.width()),
            f64::from(b.height()),
        );
        let line = |place: TaskPlace, after: Option<String>, y: f64, depth: usize| {
            let indent = INDENT * depth.min(crate::rows::MAX_DEPTH) as f64;
            Some(TaskTarget::Place {
                place,
                after,
                y,
                x: bx + indent,
                width: (bw - indent).max(1.0),
            })
        };
        let r = row.row().clone();
        match r {
            Row::Task(t) => {
                let upper = y < by + bh / 2.0;
                let own = self.within(&t.id, id);
                // Right half of the content, lower half: nest under it (4.4).
                if !upper && !own && x > bx + bw / 2.0 {
                    return Some(TaskTarget::Into {
                        parent: t.id.clone(),
                        row: row_w,
                    });
                }
                let (place, after, depth) = self.gap_target(id, &t, upper, own, x)?;
                line(place, after, if upper { by } else { by + bh }, depth)
            }
            Row::SectionHeader(s) => {
                let place = TaskPlace {
                    project_id: project,
                    section_id: Some(s.id.clone()),
                    parent_id: None,
                };
                line(place, None, by + bh, 0)
            }
            Row::Ghost { key, .. } => {
                let section = key.strip_prefix("ghost:section:").map(str::to_owned);
                if section.is_none() && key != "ghost:root" {
                    return None;
                }
                let place = TaskPlace {
                    project_id: project,
                    section_id: section,
                    parent_id: None,
                };
                let last = {
                    let store = self.store();
                    // The last active sibling of that list.
                    store
                        .project_view(&place.project_id)
                        .ok()
                        .flatten()
                        .and_then(|pv| {
                            let nodes = match &place.section_id {
                                None => pv.tasks,
                                Some(s) => pv
                                    .sections
                                    .into_iter()
                                    .find(|g| g.section.id == *s)
                                    .map(|g| g.tasks)
                                    .unwrap_or_default(),
                            };
                            nodes.into_iter().map(|n| n.task.item.id).rfind(|n| n != id)
                        })
                };
                line(place, last, by, 0)
            }
            _ => None,
        }
    }

    /// The real depth of task `id` and its ancestors (`[id, parent, ...]`).
    fn task_chain(&self, id: &str) -> Vec<String> {
        let store = self.store();
        let mut out = Vec::new();
        let mut cur = Some(id.to_owned());
        while let Some(c) = cur {
            if out.len() > 64 {
                break;
            }
            cur = store
                .task(&c)
                .ok()
                .flatten()
                .and_then(|t| t.parent_id.clone());
            out.push(c);
        }
        out
    }

    /// Sideways pointer movement since the drag started, at `x` in list coordinates.
    fn drag_dx(&self, x: f64) -> f64 {
        let Some((ox, _)) = ORIGIN.with(std::cell::Cell::get) else {
            return 0.0;
        };
        let list = self.imp().task_list.get();
        list.compute_point(self, &gtk::graphene::Point::new(x as f32, 0.0))
            .map_or(0.0, |p| f64::from(p.x()) - ox)
    }

    /// A drop between two rows, before (`upper`) or after row `t`: where the dragged task
    /// `id` goes and at which depth (4.6). `own`: `t` is the dragged task or inside it.
    fn gap_target(
        &self,
        id: &str,
        t: &crate::rows::TaskRow,
        upper: bool,
        own: bool,
        x: f64,
    ) -> Option<(TaskPlace, Option<String>, usize)> {
        let store_rows = self.imp().rows.get()?;
        let rows: Vec<TickRow> = (0..store_rows.n_items())
            .filter_map(|i| store_rows.item(i).and_downcast::<TickRow>())
            .collect();
        let idx = rows.iter().position(|r| r.key() == t.key)?;
        let gap = if upper { idx } else { idx + 1 };
        // Neighbouring task rows of the gap in the same group, the dragged subtree aside.
        let task_of = |r: &TickRow| r.row().task().map(|x| x.id.clone());
        let mut above = None;
        for r in rows[..gap].iter().rev() {
            match task_of(r) {
                Some(tid) if self.within(&tid, id) => continue,
                Some(tid) => {
                    above = Some(tid);
                    break;
                }
                None => break,
            }
        }
        let mut below = None;
        for r in &rows[gap..] {
            match task_of(r) {
                Some(tid) if self.within(&tid, id) => continue,
                Some(tid) => {
                    below = Some(tid);
                    break;
                }
                None => break,
            }
        }
        let depth_of = |tid: &str| self.task_chain(tid).len().saturating_sub(1);
        let row_depth = depth_of(&t.id);
        let default = if own {
            depth_of(id)
        } else if !upper && t.has_children && !t.collapsed {
            row_depth + 1
        } else {
            row_depth
        };
        let min = below.as_deref().map_or(0, depth_of);
        let max = above.as_deref().map_or(0, |a| depth_of(a) + 1);
        let d = line_depth(default, self.drag_dx(x), min, max)?;
        // Back on its own row at its own depth: nothing to move.
        if own && d == depth_of(id) {
            return None;
        }
        let project = self.project_of_view()?;
        match above {
            Some(a) => {
                let chain = self.task_chain(&a);
                let da = chain.len() - 1;
                let (parent, after) = if d == da + 1 {
                    (Some(a.clone()), None)
                } else {
                    let anc = chain.get(da - d)?.clone();
                    (chain.get(da - d + 1).cloned(), Some(anc))
                };
                let section = {
                    let store = self.store();
                    store
                        .task(&a)
                        .ok()
                        .flatten()
                        .and_then(|x| x.section_id.clone())
                };
                Some((
                    TaskPlace {
                        project_id: project,
                        section_id: section,
                        parent_id: parent,
                    },
                    after,
                    d,
                ))
            }
            None => {
                let anchor = below.unwrap_or_else(|| t.id.clone());
                let section = {
                    let store = self.store();
                    store
                        .task(&anchor)
                        .ok()
                        .flatten()
                        .and_then(|x| x.section_id.clone())
                };
                Some((
                    TaskPlace {
                        project_id: project,
                        section_id: section,
                        parent_id: None,
                    },
                    None,
                    0,
                ))
            }
        }
    }

    /// Upcoming and Today: the day group under (x, y), if it is another day than the task's
    /// (4.7, 4.20). The Overdue group takes no drops.
    fn day_target(&self, id: &str, x: f64, y: f64) -> Option<TaskTarget> {
        let list: gtk::Widget = self.imp().task_list.get().upcast();
        let (row_w, b) = row_at(&list, x, y)?;
        let key = row_key(&row_w)?;
        let store_rows = self.imp().rows.get()?;
        let rows: Vec<TickRow> = (0..store_rows.n_items())
            .filter_map(|i| store_rows.item(i).and_downcast::<TickRow>())
            .collect();
        let idx = rows.iter().position(|r| r.key() == key)?;
        // The group's day header: this row's, or the nearest one above it.
        let header = rows[..=idx].iter().rev().find_map(|r| match &*r.row() {
            Row::DayHeader(d) => Some(d.key.clone()),
            Row::ViewHeader(_) => Some(String::new()),
            _ => None,
        })?;
        let date =
            chrono::NaiveDate::parse_from_str(header.strip_prefix("day:")?, "%Y-%m-%d").ok()?;
        let current = self.store().task(id).ok().flatten()?.due.map(|d| d.date);
        if current == Some(date) {
            return None;
        }
        let (by, bh) = (f64::from(b.y()), f64::from(b.height()));
        let upper = y < by + bh / 2.0;
        Some(TaskTarget::Day {
            date,
            y: if upper { by } else { by + bh },
            x: f64::from(b.x()),
            width: f64::from(b.width()),
        })
    }

    fn show_task_target(&self, target: &TaskTarget) {
        let imp = self.imp();
        match target {
            TaskTarget::Place { y, x, width, .. } => {
                self.clear_into();
                let list: gtk::Widget = imp.task_list.get().upcast();
                let overlay: gtk::Widget = imp.list_overlay.get().upcast();
                self.place_line(&imp.drop_line, &list, &overlay, *x, *y, *width);
            }
            TaskTarget::Into { row, .. } => {
                imp.drop_line.set_visible(false);
                self.show_into(row);
            }
            TaskTarget::Day { y, x, width, .. } => {
                self.clear_into();
                let list: gtk::Widget = imp.task_list.get().upcast();
                let overlay: gtk::Widget = imp.list_overlay.get().upcast();
                self.place_line(&imp.drop_line, &list, &overlay, *x, *y, *width);
            }
        }
    }

    fn place_line(
        &self,
        line: &gtk::Box,
        from: &gtk::Widget,
        overlay: &gtk::Widget,
        x: f64,
        y: f64,
        width: f64,
    ) {
        let Some(p) = from.compute_point(overlay, &gtk::graphene::Point::new(x as f32, y as f32))
        else {
            return;
        };
        line.set_margin_start(p.x().max(0.0) as i32);
        line.set_margin_top((p.y() - 3.0).max(0.0) as i32);
        line.set_width_request(width as i32);
        line.set_visible(true);
    }

    fn show_into(&self, row: &gtk::Widget) {
        let imp = self.imp();
        let same = imp.drop_into.borrow().as_ref() == Some(row);
        if same {
            return;
        }
        self.clear_into();
        row.add_css_class("tick-drop-into");
        imp.drop_into.replace(Some(row.clone()));
    }

    fn clear_into(&self) {
        if let Some(r) = self.imp().drop_into.take() {
            r.remove_css_class("tick-drop-into");
        }
    }

    fn hide_drop_feedback(&self) {
        let imp = self.imp();
        imp.drop_line.set_visible(false);
        imp.sidebar_drop_line.set_visible(false);
        self.clear_into();
    }

    fn drop_task(&self, id: &str, target: TaskTarget) {
        let cmds = match target {
            TaskTarget::Day { date, .. } => {
                // A new day, the same time (4.7).
                let time = self
                    .store()
                    .task(id)
                    .ok()
                    .flatten()
                    .and_then(|t| t.due.and_then(|d| d.time));
                let due = crate::edit::due_args(time, &crate::pickers::DateChoice::Date(date));
                vec![tick_core::Command::item_update(
                    id,
                    serde_json::json!({ "due": due }),
                )]
            }
            TaskTarget::Place { place, after, .. } => self
                .store()
                .place_task_commands(id, &place, after.as_deref())
                .unwrap_or_default(),
            TaskTarget::Into { parent, .. } => {
                let (place, last) = {
                    let store = self.store();
                    let Some(p) = store.task(&parent).ok().flatten() else {
                        return;
                    };
                    let last = store
                        .child_ids(&parent)
                        .unwrap_or_default()
                        .into_iter()
                        .rfind(|c| c != id);
                    (
                        TaskPlace {
                            parent_id: Some(parent.clone()),
                            ..TaskPlace::of(&p)
                        },
                        last,
                    )
                };
                self.store()
                    .place_task_commands(id, &place, last.as_deref())
                    .unwrap_or_default()
            }
        };
        if cmds.is_empty() {
            return;
        }
        let weak = self.downgrade();
        // After the drag finished: the change rebuilds rows.
        glib::idle_add_local_once(move || {
            if let Some(w) = weak.upgrade() {
                w.enqueue(cmds);
            }
        });
    }

    // ---- sidebar ----

    /// While dragging over the sidebar: a collapsed project under the pointer opens after
    /// [`HOVER_EXPAND`] (a timer, since motion stops while the pointer rests).
    fn hover_expand(&self, x: f64, y: f64) {
        let Some(sb) = self.imp().sidebar.get() else {
            return;
        };
        let list: gtk::Widget = sb.list().clone().upcast();
        let row = row_at(&list, x, y)
            .and_then(|(w, _)| w.first_child().and_downcast::<gtk::TreeExpander>())
            .and_then(|e| e.list_row())
            .filter(|r| r.is_expandable() && !r.is_expanded());
        let imp = self.imp();
        let same = match (&row, imp.hover_row.borrow().as_ref()) {
            (Some(r), Some((h, _))) => h.upgrade().as_ref() == Some(r),
            _ => false,
        };
        if same {
            return;
        }
        self.cancel_hover_expand();
        let Some(row) = row else { return };
        let weak = row.downgrade();
        let win = self.downgrade();
        let id = glib::timeout_add_local_once(HOVER_EXPAND, move || {
            if let Some(w) = win.upgrade() {
                w.imp().hover_row.replace(None);
            }
            if let Some(r) = weak.upgrade() {
                r.set_expanded(true);
            }
        });
        imp.hover_row.replace(Some((row.downgrade(), id)));
    }

    fn cancel_hover_expand(&self) {
        if let Some((_, id)) = self.imp().hover_row.take() {
            id.remove();
        }
    }

    /// The project row (Inbox included) under `y` in the sidebar list.
    fn sidebar_project_at(&self, x: f64, y: f64) -> Option<(String, gtk::Widget)> {
        let sb = self.imp().sidebar.get()?;
        let list: gtk::Widget = sb.list().clone().upcast();
        let (row, _) = row_at(&list, x, y)?;
        let item = sidebar_item(&row)?;
        match item.view()? {
            ViewId::Project(id) => Some((id, row)),
            ViewId::Inbox => Some((self.store().inbox_project_id().ok().flatten()?, row)),
            _ => None,
        }
    }

    fn project_target(&self, id: &str, x: f64, y: f64) -> Option<ProjectTarget> {
        let sb = self.imp().sidebar.get()?;
        let list: gtk::Widget = sb.list().clone().upcast();
        let (row, b) = row_at(&list, x, y)?;
        let item = sidebar_item(&row)?;
        let ViewId::Project(target) = item.view()? else {
            return None;
        };
        let depth = row
            .first_child()
            .and_downcast::<gtk::TreeExpander>()
            .and_then(|e| e.list_row())
            .map_or(0, |r| r.depth());
        let tree = self.store().projects_tree().ok()?;
        let node = tree.iter().find(|n| n.project.id == target)?;
        if node.project.inbox_project {
            return None;
        }
        // Not onto itself or into its own sub-projects.
        let mut cur = Some(target.clone());
        while let Some(c) = cur {
            if c == id {
                return None;
            }
            cur = tree
                .iter()
                .find(|n| n.project.id == c)
                .and_then(|n| n.project.parent_id.clone());
        }
        let (by, bh, bx, bw) = (
            f64::from(b.y()),
            f64::from(b.height()),
            f64::from(b.x()),
            f64::from(b.width()),
        );
        let parent = node.project.parent_id.clone();
        let sibs: Vec<String> = tree
            .iter()
            .filter(|n| {
                !n.project.inbox_project && n.project.parent_id == parent && n.project.id != id
            })
            .map(|n| n.project.id.clone())
            .collect();
        let pos = sibs.iter().position(|s| *s == target)?;
        let x = bx + 12.0 + SIDEBAR_INDENT * depth as f64;
        let width = (bw - 24.0 - SIDEBAR_INDENT * depth as f64).max(1.0);
        let third = bh / 3.0;
        if y < by + third {
            Some(ProjectTarget::Place {
                parent,
                after: pos.checked_sub(1).map(|p| sibs[p].clone()),
                y: by,
                x,
                width,
            })
        } else if y < by + 2.0 * third {
            Some(ProjectTarget::Into {
                parent: target,
                row,
            })
        } else {
            Some(ProjectTarget::Place {
                parent,
                after: Some(target),
                y: by + bh,
                x,
                width,
            })
        }
    }

    fn show_project_target(&self, target: &ProjectTarget) {
        let imp = self.imp();
        match target {
            ProjectTarget::Place { y, x, width, .. } => {
                self.clear_into();
                let Some(sb) = imp.sidebar.get() else { return };
                let list: gtk::Widget = sb.list().clone().upcast();
                let overlay: gtk::Widget = imp.sidebar_overlay.get().upcast();
                self.place_line(&imp.sidebar_drop_line, &list, &overlay, *x, *y, *width);
            }
            ProjectTarget::Into { row, .. } => {
                imp.sidebar_drop_line.set_visible(false);
                self.show_into(row);
            }
        }
    }

    // ---- keyboard equivalents (4.4) ----

    /// Alt+Shift+Up/Down/Right/Left on the selected task: reorder among siblings, nest under
    /// the task above, un-nest. Only in project lists.
    pub(crate) fn move_task_key(&self, how: ProjectMove) -> bool {
        if self.project_of_view().is_none() {
            return false;
        }
        let Some(id) = self.selected_task() else {
            return false;
        };
        let (task, sibs) = {
            let store = self.store();
            let Some(t) = store.task(&id).ok().flatten() else {
                return false;
            };
            (t, store.task_siblings(&id).unwrap_or_default())
        };
        let Some(pos) = sibs.iter().position(|s| *s == id) else {
            return false;
        };
        let place = TaskPlace::of(&task);
        let cmds = {
            let store = self.store();
            match how {
                ProjectMove::Up if pos > 0 => {
                    let after = pos.checked_sub(2).map(|p| sibs[p].clone());
                    store.place_task_commands(&id, &place, after.as_deref())
                }
                ProjectMove::Down if pos + 1 < sibs.len() => {
                    store.place_task_commands(&id, &place, Some(&sibs[pos + 1]))
                }
                ProjectMove::In if pos > 0 => {
                    let parent = sibs[pos - 1].clone();
                    let last = store.child_ids(&parent).unwrap_or_default().pop();
                    let p = TaskPlace {
                        parent_id: Some(parent),
                        ..place.clone()
                    };
                    store.place_task_commands(&id, &p, last.as_deref())
                }
                ProjectMove::Out => match &task.parent_id {
                    Some(parent) => match store.task(parent).ok().flatten() {
                        Some(p) => store.place_task_commands(&id, &TaskPlace::of(&p), Some(parent)),
                        None => Ok(Vec::new()),
                    },
                    None => Ok(Vec::new()),
                },
                _ => Ok(Vec::new()),
            }
            .unwrap_or_default()
        };
        if cmds.is_empty() {
            return true;
        }
        self.enqueue(cmds);
        // Keep keyboard focus on the moved task.
        if let Some(pos) = self.row_position(&id) {
            self.imp().task_list.scroll_to(
                pos,
                gtk::ListScrollFlags::FOCUS | gtk::ListScrollFlags::SELECT,
                None,
            );
        }
        true
    }
}

fn sidebar_item(row: &gtk::Widget) -> Option<SidebarItem> {
    row.first_child()
        .and_downcast::<gtk::TreeExpander>()?
        .list_row()?
        .item()
        .and_downcast::<SidebarItem>()
}

/// The margin around a drag icon's surface, for its floating shadow.
const ICON_MARGIN: i32 = 20;

/// A drag source for a row: `payload` on drag, `icon` as the drag icon (floating surface,
/// 4.4), and the source row dimmed to 40 % while it is dragged.
pub fn drag_source(
    widget: &gtk::Widget,
    payload: String,
    icon: impl Fn() -> gtk::Widget + 'static,
    on_begin: impl Fn() + 'static,
    on_end: impl Fn() + 'static,
) {
    let source = gtk::DragSource::new();
    source.set_actions(gtk::gdk::DragAction::MOVE);
    let hot = std::rc::Rc::new(std::cell::Cell::new((0.0, 0.0)));
    let h = hot.clone();
    source.connect_prepare(move |_, x, y| {
        h.set((x, y));
        Some(gtk::gdk::ContentProvider::for_value(&payload.to_value()))
    });
    let on_end: std::rc::Rc<dyn Fn()> = std::rc::Rc::new(on_end);
    let w = widget.downgrade();
    source.connect_drag_begin(move |_, drag| {
        let icon_widget = icon();
        // Room around the floating surface for its shadow (2.5): the drag icon's surface is
        // exactly its child's size, and a shadow past it would be cut off.
        icon_widget.set_margin_start(ICON_MARGIN);
        icon_widget.set_margin_end(ICON_MARGIN);
        icon_widget.set_margin_top(ICON_MARGIN);
        icon_widget.set_margin_bottom(ICON_MARGIN);
        let (hx, hy0) = hot.get();
        // Where the drag started, in window coordinates (sideways steps, 4.6).
        let origin = w.upgrade().and_then(|w| {
            let root = w.root()?;
            w.compute_point(&root, &gtk::graphene::Point::new(hx as f32, hy0 as f32))
        });
        ORIGIN.with(|o| o.set(origin.map(|p| (f64::from(p.x()), f64::from(p.y())))));
        let di = gtk::DragIcon::for_drag(drag);
        di.set_child(Some(&icon_widget));
        // The icon hangs just below the pointer, so the insertion line at the pointer stays
        // visible (a centered icon would cover it).
        let (hx, hy) = (hx as i32 + ICON_MARGIN, ICON_MARGIN - 6);
        drag.set_hotspot(hx, hy);
        crate::test_mode::set_drag_icon(di.upcast_ref(), f64::from(hx), f64::from(hy));
        // The source row (the list's row widget) stays in place at 40 % (4.4).
        let row = w
            .upgrade()
            .and_then(|w| w.parent())
            .and_then(|b| b.parent());
        if let Some(row) = &row {
            row.add_css_class("tick-drag-source");
        }
        on_begin();
        // The end is reported by the drag itself: the source widget may be gone by then.
        let done = std::rc::Rc::new(std::cell::Cell::new(false));
        let finish = {
            let on_end = on_end.clone();
            let row = row.map(|r| r.downgrade());
            move || {
                if done.replace(true) {
                    return;
                }
                if let Some(r) = row.as_ref().and_then(|r| r.upgrade()) {
                    r.remove_css_class("tick-drag-source");
                }
                on_end();
            }
        };
        let f = finish.clone();
        drag.connect_dnd_finished(move |_| f());
        let f = finish.clone();
        drag.connect_cancel(move |_, _| f());
        drag.connect_drop_performed(move |_| finish());
    });
    widget.add_controller(source);
}

#[cfg(test)]
mod tests {
    use super::line_depth;

    #[test]
    fn sideways_steps_of_32_px() {
        // No movement: the spot's own depth.
        assert_eq!(line_depth(0, 0.0, 0, 1), Some(0));
        // Whole steps only; one level at most under the row above.
        assert_eq!(line_depth(0, 16.0, 0, 1), Some(0));
        assert_eq!(line_depth(0, 33.0, 0, 1), Some(1));
        assert_eq!(line_depth(0, 100.0, 0, 1), Some(1));
        assert_eq!(line_depth(2, -40.0, 0, 3), Some(1));
        assert_eq!(line_depth(2, -200.0, 1, 3), Some(1));
        assert_eq!(line_depth(1, 10.0, 0, 2), Some(1));
        // No valid depth.
        assert_eq!(line_depth(0, 0.0, 2, 1), None);
    }
}
