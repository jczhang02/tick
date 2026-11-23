//! Editing tasks from the window: the detail pane (`DESIGN.md` 4.8), the pickers (4.9,
//! 4.10) for the pane and for task rows (T, L, V, 1 to 4, the context menu), deleting with
//! Undo, and the generic Undo toast for moves and archives (4.12).

use std::rc::Rc;

use adw::prelude::*;
use adw::subclass::prelude::*;
use chrono::Weekday;
use gtk::{gio, glib};
use serde_json::{Value, json};
use tick_core::store::Task;
use tick_core::{Command, MoveTarget};

use crate::detail::{self, DetailEvent, Field, TaskDetail};
use crate::format::DateSettings;
use crate::i18n::{fill, gettext};
use crate::pickers::{self, DateChoice, DateInfo, LabelOption, MoveOption, Person};
use crate::row_object::TickRow;
use crate::rows::Row;
use crate::test_mode;
use crate::window::TickWindow;

/// Where a picker points: a widget (a detail field row), or a task row of the list.
pub enum Anchor {
    Widget(gtk::Widget),
    /// The row with this key.
    Row(String),
}

/// The visible toast's Undo for changes other than completions (move, delete, archive).
pub struct UndoAction {
    pub toast: adw::Toast,
    pub undo: Rc<dyn Fn(&TickWindow)>,
    /// Runs when the toast goes away without Undo (a held delete is released).
    pub gone: Rc<dyn Fn(&TickWindow)>,
}

/// Arguments of `due` for a picked date (4.9): a calendar day keeps the task's existing
/// `time` as the floating `YYYY-MM-DDTHH:MM:SS`; typed text goes to the server as `string`.
pub fn due_args(time: Option<chrono::NaiveTime>, choice: &DateChoice) -> Value {
    match choice {
        DateChoice::Date(d) => match time {
            Some(t) => {
                json!({ "date": format!("{}T{}", d.format("%Y-%m-%d"), t.format("%H:%M:%S")) })
            }
            None => json!({ "date": d.format("%Y-%m-%d").to_string() }),
        },
        DateChoice::Text(s) => json!({ "string": s }),
        DateChoice::Clear => Value::Null,
    }
}

/// Arguments of `deadline` (`docs/api/pages/deadlines.md`: a plain date, no time).
pub fn deadline_args(choice: &DateChoice) -> Value {
    match choice {
        DateChoice::Date(d) => json!({ "date": d.format("%Y-%m-%d").to_string() }),
        DateChoice::Text(_) | DateChoice::Clear => Value::Null,
    }
}

impl TickWindow {
    pub(crate) fn setup_detail(&self) {
        let imp = self.imp();
        let weak = self.downgrade();
        let detail = TaskDetail::new(Rc::new(move |e| {
            let weak = weak.clone();
            // After the emitting widget's handler returns: acting may rebuild it.
            glib::idle_add_local_once(move || {
                if let Some(w) = weak.upgrade() {
                    w.on_detail_event(e);
                }
            });
        }));
        imp.detail_bin.set_child(Some(&detail.root));
        imp.detail_split.add_css_class("tick-detail-split");
        imp.detail_view.add_bottom_bar(&detail.composer);
        let _ = imp.detail.set(detail);
        // Closing the overlay (a click on the list in B1-B3) closes the task too.
        imp.detail_split.connect_show_sidebar_notify(|split| {
            if !split.shows_sidebar()
                && let Some(w) = split.root().and_downcast::<TickWindow>()
                && w.imp().open_task.borrow().is_some()
            {
                w.close_task(false);
            }
        });

        let actions = [
            ("detail-complete", Self::detail_complete as fn(&TickWindow)),
            ("detail-move", |w| {
                if let Some(id) = w.open_task_id()
                    && let Some(a) = w.detail().field_anchor(Field::Move)
                {
                    w.open_picker(Field::Move, &id, Anchor::Widget(a));
                }
            }),
            ("detail-delete", |w| {
                if let Some(id) = w.open_task_id() {
                    w.delete_task(&id);
                }
            }),
            ("task-open", |w| {
                if let Some(id) = w.selected_task() {
                    w.open_task(&id, true);
                }
            }),
            ("task-complete", |w| w.complete_selected()),
            ("task-date", |w| w.pick_for_selected(Field::Date)),
            ("task-deadline", |w| w.pick_for_selected(Field::Deadline)),
            ("task-priority", |w| w.pick_for_selected(Field::Priority)),
            ("task-labels", |w| w.pick_for_selected(Field::Labels)),
            ("task-move", |w| w.pick_for_selected(Field::Move)),
            ("task-delete", |w| {
                if let Some(id) = w.selected_task() {
                    w.delete_task(&id);
                }
            }),
        ];
        for (name, f) in actions {
            let a = gio::SimpleAction::new(name, None);
            let weak = self.downgrade();
            a.connect_activate(move |_, _| {
                if let Some(w) = weak.upgrade() {
                    f(&w);
                }
            });
            self.add_action(&a);
        }
    }

    pub(crate) fn detail(&self) -> Rc<TaskDetail> {
        self.imp().detail.get().expect("detail pane").clone()
    }

    pub(crate) fn open_task_id(&self) -> Option<String> {
        self.imp().open_task.borrow().clone()
    }

    /// Open task `id` in the detail pane (click, Enter), selecting its row.
    pub(crate) fn open_task(&self, id: &str, focus: bool) {
        let imp = self.imp();
        let id = self.live_id(id);
        imp.open_task.replace(Some(id.clone()));
        self.refresh_detail();
        if imp.open_task.borrow().is_none() {
            return;
        }
        imp.detail_split.set_show_sidebar(true);
        if let Some(pos) = self.row_position(&id)
            && let Some(sel) = imp.selection.get()
            && sel.selected() != pos
        {
            sel.set_selected(pos);
        }
        if focus {
            let weak = self.downgrade();
            glib::idle_add_local_once(move || {
                if let Some(w) = weak.upgrade() {
                    w.detail().focus_title();
                }
            });
        }
    }

    /// Close the pane; `refocus` returns keyboard focus to the task's row (Escape).
    pub(crate) fn close_task(&self, refocus: bool) {
        let imp = self.imp();
        let id = imp.open_task.take();
        self.detail().set_data(None);
        imp.detail_split.set_show_sidebar(false);
        if refocus && let Some(id) = id {
            let pos = self.row_position(&id).or(self.selected_position());
            self.keep_focus_in_list(pos.unwrap_or(0), true);
        }
    }

    /// Reload the open task after a change (local or synced). A task that is gone (deleted
    /// here or elsewhere) closes the pane.
    pub(crate) fn refresh_detail(&self) {
        let imp = self.imp();
        let Some(old) = imp.open_task.borrow().clone() else {
            return;
        };
        let id = self.live_id(&old);
        if id != old {
            // The server replaced a temp id: still the same task for the pane's editors.
            self.detail().rename_id(&old, &id);
        }
        imp.open_task.replace(Some(id.clone()));
        let data = {
            let store = imp.store.borrow();
            let Some(store) = store.as_ref() else { return };
            detail::load(store, &id)
        };
        match data {
            Ok(Some(d)) => self.detail().set_data(Some(d)),
            Ok(None) => self.close_task(false),
            Err(e) => eprintln!("tick: reading the task: {e}"),
        }
    }

    fn detail_complete(&self) {
        if let Some(id) = self.open_task_id() {
            self.begin_complete(&id);
        }
    }

    fn on_detail_event(&self, e: DetailEvent) {
        match e {
            DetailEvent::Commands(cmds) => self.enqueue(cmds),
            DetailEvent::Pick { field, anchor } => {
                if let Some(id) = self.open_task_id() {
                    self.open_picker(field, &id, Anchor::Widget(anchor));
                }
            }
            DetailEvent::Clear(field) => {
                if let Some(id) = self.open_task_id() {
                    self.clear_field(&id, field);
                }
            }
            DetailEvent::Complete { id, done } => {
                if done {
                    self.begin_complete(&id);
                } else {
                    self.reopen(&id);
                }
            }
            DetailEvent::Open(id) => self.open_task(&id, false),
            DetailEvent::OpenUri(uri) => self.open_uri(&uri),
            DetailEvent::DeleteComment(id) => self.confirm_delete_comment(&id),
        }
    }

    /// Uncheck a task: take back a completion in progress, else reopen it.
    fn reopen(&self, id: &str) {
        self.uncheck(id);
        let still = self
            .store()
            .task(id)
            .ok()
            .flatten()
            .is_some_and(|t| t.checked && !t.pending);
        if still {
            self.enqueue(vec![Command::item_uncomplete(id)]);
        }
    }

    fn clear_field(&self, id: &str, field: Field) {
        let args = match field {
            Field::Date => json!({ "due": null }),
            Field::Deadline => json!({ "deadline": null }),
            Field::Priority => json!({ "priority": 1 }),
            Field::Labels => json!({ "labels": [] }),
            Field::Assignee => json!({ "responsible_uid": null }),
            Field::Move => return,
        };
        self.enqueue(vec![Command::item_update(id, args)]);
    }

    pub(crate) fn set_priority(&self, id: &str, p: u8) {
        let api = tick_core::model::p_to_api_priority(p);
        self.enqueue(vec![Command::item_update(id, json!({ "priority": api }))]);
    }

    fn pick_for_selected(&self, field: Field) {
        let Some(row) = self
            .imp()
            .selection
            .get()
            .and_then(|s| s.selected_item())
            .and_downcast::<TickRow>()
        else {
            return;
        };
        let Some(id) = row.task_id() else { return };
        self.open_picker(field, &id, Anchor::Row(row.key()));
    }

    /// The rectangle of the row with `key` in the task list's coordinates.
    fn row_rect(&self, key: &str) -> Option<gtk::gdk::Rectangle> {
        let list = self.imp().task_list.get();
        let mut child = list.first_child();
        while let Some(c) = child {
            let name = c
                .first_child()
                .and_downcast::<adw::Bin>()
                .and_then(|b| b.child())
                .map(|w| w.widget_name().to_string());
            if name.as_deref() == Some(key)
                && let Some(b) = c.compute_bounds(&list)
            {
                // Point at the start of the title (after the ring), as a menu would.
                return Some(gtk::gdk::Rectangle::new(
                    b.x() as i32 + 44,
                    b.y() as i32,
                    (b.width() as i32 - 44).max(1),
                    b.height() as i32,
                ));
            }
            child = c.next_sibling();
        }
        None
    }

    /// Show a picker at `anchor`. `below`: always under (or over) the anchor, never beside
    /// it (the project crumb's "Move to...", whose list reads from the crumb down).
    fn present_popover(&self, pop: &gtk::Popover, anchor: Anchor, below: bool) {
        let (parent, rect): (gtk::Widget, Option<gtk::gdk::Rectangle>) = match anchor {
            Anchor::Widget(w) => (w, None),
            Anchor::Row(key) => {
                let list = self.imp().task_list.get();
                let rect = self.row_rect(&key);
                (list.upcast(), rect)
            }
        };
        // Where the anchor is in the window.
        let area = match rect {
            Some(r) => parent.compute_bounds(self).map(|b| {
                gtk::graphene::Rect::new(
                    b.x() + r.x() as f32,
                    b.y() + r.y() as f32,
                    r.width() as f32,
                    r.height() as f32,
                )
            }),
            None => parent.compute_bounds(self),
        };
        let height = self.height() as f32;
        match area {
            // A detail field row with room on its left (the pane sits at the window's end
            // edge): open towards the list, where the popover can slide to fit vertically.
            Some(a) if rect.is_none() && a.x() >= 360.0 && !below => {
                pop.set_position(gtk::PositionType::Left);
            }
            // Otherwise below (or above) the anchor, scrolling inside when it is taller than
            // the room there (a 630 px date picker under a row of a 800 px window).
            Some(a) => {
                let below = height - (a.y() + a.height()) - 12.0;
                let above = a.y() - 12.0;
                let (pos, room) = if below >= above {
                    (gtk::PositionType::Bottom, below)
                } else {
                    (gtk::PositionType::Top, above)
                };
                pop.set_position(pos);
                pickers::limit_height(pop, (room as i32 - 24).max(200));
            }
            None => pop.set_position(gtk::PositionType::Bottom),
        }
        pickers::present(pop, &parent, rect);
    }

    fn date_info(&self, current: Option<chrono::NaiveDate>) -> DateInfo {
        let store = self.store();
        let user = store.user().ok().flatten();
        let setting = |k: &str| {
            user.as_ref()
                .and_then(|u| u.extra.get(k))
                .and_then(Value::as_i64)
        };
        DateInfo {
            today: store.now().today(),
            weekend: DateInfo::weekday_from_setting(setting("weekend_start_day"), Weekday::Sat),
            next_week: DateInfo::weekday_from_setting(setting("next_week"), Weekday::Mon),
            current,
            settings: DateSettings::from_user(user.as_ref()),
        }
    }

    /// Open the picker for `field` of task `id`.
    pub(crate) fn open_picker(&self, field: Field, id: &str, anchor: Anchor) {
        let Some(task) = self.store().task(id).ok().flatten() else {
            return;
        };
        let id = task.id.clone();
        let weak = self.downgrade();
        let pop = match field {
            Field::Date => {
                let info = self.date_info(task.due.map(|d| d.date));
                pickers::date_picker(info, false, move |c| {
                    let Some(w) = weak.upgrade() else { return };
                    // The temp id may have been replaced while the picker was open.
                    let id = w.live_id(&id);
                    let Some(t) = w.store().task(&id).ok().flatten() else {
                        return;
                    };
                    w.enqueue(vec![Command::item_update(
                        &id,
                        json!({ "due": due_args(t.due.and_then(|d| d.time), &c) }),
                    )]);
                })
            }
            Field::Deadline => {
                let current = task.item.deadline.as_ref().and_then(|d| {
                    chrono::NaiveDate::parse_from_str(d.date.get(..10)?, "%Y-%m-%d").ok()
                });
                let info = self.date_info(current);
                pickers::date_picker(info, true, move |c| {
                    if let Some(w) = weak.upgrade() {
                        w.enqueue(vec![Command::item_update(
                            &id,
                            json!({ "deadline": deadline_args(&c) }),
                        )]);
                    }
                })
            }
            Field::Priority => pickers::priority_picker(task.item.p(), move |p| {
                if let Some(w) = weak.upgrade() {
                    w.set_priority(&id, p);
                }
            }),
            Field::Labels => {
                let mut options: Vec<LabelOption> = self
                    .store()
                    .labels()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|l| LabelOption {
                        checked: task.labels.contains(&l.label.name),
                        name: l.label.name,
                        color: l.label.color,
                    })
                    .collect();
                // Labels on the task without a personal label (shared ones) stay toggleable.
                for n in &task.labels {
                    if !options.iter().any(|o| o.name == *n) {
                        options.push(LabelOption {
                            name: n.clone(),
                            color: "charcoal".into(),
                            checked: true,
                        });
                    }
                }
                let weak2 = weak.clone();
                let id2 = id.clone();
                pickers::label_picker(
                    options,
                    move |name, checked| {
                        if let Some(w) = weak.upgrade() {
                            w.toggle_label(&id, &name, checked);
                        }
                    },
                    move |name| {
                        if let Some(w) = weak2.upgrade() {
                            w.create_label_on(&id2, &name);
                        }
                    },
                )
            }
            Field::Assignee => {
                let (people, me) = {
                    let store = self.store();
                    let mut people: Vec<Person> = store
                        .collaborators(&task.project_id)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|c| Person {
                            id: Some(c.id),
                            name: c.full_name,
                        })
                        .collect();
                    let me = store.user().ok().flatten();
                    if let Some(u) = &me
                        && !people.iter().any(|p| p.id.as_deref() == Some(&u.id))
                    {
                        people.insert(
                            0,
                            Person {
                                id: Some(u.id.clone()),
                                name: u.full_name.clone(),
                            },
                        );
                    }
                    (people, me)
                };
                let _ = me;
                pickers::assignee_picker(people, task.item.responsible_uid.clone(), move |uid| {
                    if let Some(w) = weak.upgrade() {
                        w.enqueue(vec![Command::item_update(
                            &id,
                            json!({ "responsible_uid": uid }),
                        )]);
                    }
                })
            }
            Field::Move => {
                let options = self.move_options(&task);
                pickers::move_picker(options, move |o| {
                    if let Some(w) = weak.upgrade() {
                        w.move_task_to(&id, &o.project_id, o.section_id.as_deref());
                    }
                })
            }
        };
        self.present_popover(&pop, anchor, field == Field::Move);
    }

    fn move_options(&self, task: &Task) -> Vec<MoveOption> {
        let store = self.store();
        let mut out = Vec::new();
        for n in store.projects_tree().unwrap_or_default() {
            let p = n.project;
            let name = if p.inbox_project {
                gettext("Inbox")
            } else {
                p.name.clone()
            };
            out.push(MoveOption {
                project_id: p.id.clone(),
                section_id: None,
                name,
                color: p.color.clone(),
                depth: n.depth,
                current: task.project_id == p.id && task.section_id.is_none(),
                inbox: p.inbox_project,
            });
            for s in store.sections(&p.id).unwrap_or_default() {
                if s.is_archived {
                    continue;
                }
                out.push(MoveOption {
                    current: task.section_id.as_deref() == Some(s.id.as_str()),
                    project_id: p.id.clone(),
                    section_id: Some(s.id),
                    name: s.name,
                    color: p.color.clone(),
                    depth: n.depth + 1,
                    inbox: false,
                });
            }
        }
        out
    }

    fn toggle_label(&self, id: &str, name: &str, checked: bool) {
        // The picker holds the id it opened with: follow a replaced temp id.
        let id = &self.live_id(id);
        let Some(task) = self.store().task(id).ok().flatten() else {
            return;
        };
        let mut labels = task.labels.clone();
        if checked {
            if !labels.iter().any(|l| l == name) {
                labels.push(name.to_owned());
            }
        } else {
            labels.retain(|l| l != name);
        }
        self.enqueue(vec![Command::item_update(id, json!({ "labels": labels }))]);
    }

    fn create_label_on(&self, id: &str, name: &str) {
        let id = &self.live_id(id);
        let Some(task) = self.store().task(id).ok().flatten() else {
            return;
        };
        let mut labels = task.labels.clone();
        labels.push(name.to_owned());
        self.enqueue(vec![
            Command::label_add(json!({ "name": name })),
            Command::item_update(id, json!({ "labels": labels })),
        ]);
    }

    /// The command that puts a task back where it was (Undo of a move).
    fn move_back(task: &Task) -> Command {
        let target = if let Some(p) = &task.parent_id {
            MoveTarget::Parent(p.clone())
        } else if let Some(s) = &task.section_id {
            MoveTarget::Section(s.clone())
        } else {
            MoveTarget::Project(task.project_id.clone())
        };
        Command::item_move(&task.id, target)
    }

    /// Move a task to a project (or a section of it) with the "Moved to ..." Undo toast
    /// (4.12): "Move to..." (V) and a drop on a sidebar project.
    pub(crate) fn move_task_to(&self, id: &str, project_id: &str, section_id: Option<&str>) {
        let id = &self.live_id(id);
        let (task, name) = {
            let store = self.store();
            let Some(task) = store.task(id).ok().flatten() else {
                return;
            };
            let project = store.project(project_id).ok().flatten();
            let mut name = match &project {
                Some(p) if p.inbox_project => gettext("Inbox"),
                Some(p) => p.name.clone(),
                None => return,
            };
            if let Some(s) = section_id.and_then(|s| store.section(s).ok().flatten()) {
                name = format!("{name} / {}", s.name);
            }
            (task, name)
        };
        let unchanged = task.parent_id.is_none()
            && task.project_id == project_id
            && task.section_id.as_deref() == section_id;
        if unchanged {
            return;
        }
        let target = match section_id {
            Some(s) => MoveTarget::Section(s.to_owned()),
            None => MoveTarget::Project(project_id.to_owned()),
        };
        self.enqueue(vec![Command::item_move(id, target)]);
        let back = Self::move_back(&task);
        self.show_undo(
            &fill(&gettext("Moved to {name}"), &[("name", &name)]),
            move |w| w.enqueue(vec![back.clone()]),
            |_| {},
        );
    }

    /// Delete a task with the "Task deleted" Undo toast (4.12): the delete is held back
    /// while Undo can still discard it.
    pub(crate) fn delete_task(&self, id: &str) {
        let id = self.live_id(id);
        let cmd = Command::item_delete(&id);
        let uuid = cmd.uuid.clone();
        let result = self
            .store()
            .enqueue_deferred(vec![cmd], crate::window::undo_hold());
        let changes = match result {
            Ok(c) => c,
            Err(e) => {
                eprintln!("tick: deleting a task: {e}");
                return;
            }
        };
        let focus_pos = self.row_position(&id);
        if self.open_task_id().as_deref() == Some(id.as_str()) {
            self.close_task(false);
        }
        self.after_local_change(&changes);
        if let Some(pos) = focus_pos
            && gtk::prelude::GtkWindowExt::focus(self)
                .is_none_or(|f| f.is_ancestor(&*self.imp().task_list))
        {
            self.keep_focus_in_list(pos, true);
        }
        let u1 = uuid.clone();
        self.show_undo(
            &gettext("Task deleted"),
            move |w| {
                let discarded = w.store().discard_queued(&u1);
                match discarded {
                    Ok(Some(c)) => w.after_local_change(&c),
                    Ok(None) | Err(_) => {
                        let toast = adw::Toast::builder()
                            .title(gettext(
                                "The task was already deleted and could not be restored",
                            ))
                            .timeout(5)
                            .build();
                        w.show_toast(&toast);
                    }
                }
            },
            move |w| {
                if let Err(e) = w
                    .store()
                    .defer_queued(std::slice::from_ref(&uuid), std::time::Duration::ZERO)
                {
                    eprintln!("tick: releasing a delete: {e}");
                }
                w.refresh_sync();
                if let Some(e) = w.imp().engine.borrow().as_ref() {
                    e.sync_now();
                }
            },
        );
    }

    /// Show an Undo toast for a change that is not a completion. `undo` runs on "Undo" or
    /// Ctrl+Z; `gone` when the toast goes away without Undo.
    pub(crate) fn show_undo(
        &self,
        title: &str,
        undo: impl Fn(&TickWindow) + 'static,
        gone: impl Fn(&TickWindow) + 'static,
    ) {
        let toast = adw::Toast::builder()
            .title(title)
            .button_label(gettext("Undo"))
            .timeout(5)
            .build();
        let weak = self.downgrade();
        toast.connect_button_clicked(move |_| {
            if let Some(w) = weak.upgrade() {
                w.undo();
            }
        });
        let weak = self.downgrade();
        toast.connect_dismissed(move |t| {
            let Some(w) = weak.upgrade() else { return };
            let mine = {
                let mut u = w.imp().undo_action.borrow_mut();
                if u.as_ref().is_some_and(|a| &a.toast == t) {
                    u.take()
                } else {
                    None
                }
            };
            if let Some(a) = mine {
                (a.gone)(&w);
            }
        });
        // The previous toast's Undo ends here: let its change go (a second delete must not
        // leave the first held for the whole Undo hold, with the queue waiting behind it).
        let previous = self.imp().undo_action.take();
        if let Some(p) = previous {
            (p.gone)(self);
        }
        self.imp().undo_action.replace(Some(UndoAction {
            toast: toast.clone(),
            undo: Rc::new(undo),
            gone: Rc::new(gone),
        }));
        self.show_toast(&toast);
    }

    /// Run the visible generic Undo (Ctrl+Z or its button). Returns false when there is none.
    pub(crate) fn run_undo_action(&self) -> bool {
        let Some(a) = self.imp().undo_action.take() else {
            return false;
        };
        a.toast.dismiss();
        (a.undo)(self);
        true
    }

    pub(crate) fn open_uri(&self, uri: &str) {
        // Only web and mail links (and attachment URLs, which are https) ever open.
        if !crate::markdown::safe_link(uri) {
            eprintln!("tick: not opening a link with an unsupported scheme");
            return;
        }
        if test_mode::enabled() {
            // The isolated test session has no URI handler: report what would open.
            eprintln!("TICK_OPEN_URI={uri}");
            return;
        }
        gtk::UriLauncher::new(uri).launch(Some(self), None::<&gio::Cancellable>, |r| {
            if let Err(e) = r {
                eprintln!("tick: opening a link: {e}");
            }
        });
    }

    fn confirm_delete_comment(&self, id: &str) {
        let dialog = adw::AlertDialog::new(
            Some(&gettext("Delete comment?")),
            Some(&gettext("This can't be undone.")),
        );
        dialog.add_response("cancel", &gettext("Cancel"));
        dialog.add_response("delete", &gettext("Delete Comment"));
        dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        crate::menus::quiet_scrollers(&dialog);
        let weak = self.downgrade();
        let id = id.to_owned();
        dialog.connect_response(None, move |_, r| {
            if r == "delete"
                && let Some(w) = weak.upgrade()
            {
                w.enqueue(vec![Command::note_delete(&id)]);
            }
        });
        dialog.present(Some(self));
    }

    /// The task row context menu (4.4) at `rect` in the list's coordinates.
    pub(crate) fn task_menu(&self, rect: gtk::gdk::Rectangle) {
        let menu = gio::Menu::new();
        let main = gio::Menu::new();
        main.append(Some(&gettext("Open")), Some("win.task-open"));
        main.append(Some(&gettext("Complete")), Some("win.task-complete"));
        main.append(Some(&gettext("Date...")), Some("win.task-date"));
        main.append(Some(&gettext("Priority...")), Some("win.task-priority"));
        main.append(Some(&gettext("Labels...")), Some("win.task-labels"));
        main.append(Some(&gettext("Move to...")), Some("win.task-move"));
        menu.append_section(None, &main);
        let danger = gio::Menu::new();
        danger.append(Some(&gettext("Delete Task")), Some("win.task-delete"));
        menu.append_section(None, &danger);
        let pop = gtk::PopoverMenu::from_model(Some(&menu));
        crate::menus::name_items(&pop);
        pop.set_has_arrow(false);
        pop.set_position(gtk::PositionType::Bottom);
        let list = self.imp().task_list.get();
        pickers::present(pop.upcast_ref(), &list, Some(rect));
    }

    /// The context menu of the selected row, under it (Menu key, Shift+F10).
    pub(crate) fn selected_task_menu(&self) {
        let Some(row) = self
            .imp()
            .selection
            .get()
            .and_then(|s| s.selected_item())
            .and_downcast::<TickRow>()
        else {
            return;
        };
        if !matches!(&*row.row(), Row::Task(_)) {
            return;
        }
        if let Some(rect) = self.row_rect(&row.key()) {
            self.task_menu(rect);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveTime};

    fn day() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 25).unwrap()
    }

    #[test]
    fn a_picked_day_keeps_the_time_as_a_floating_date() {
        let t = NaiveTime::from_hms_opt(14, 0, 0);
        assert_eq!(
            due_args(t, &DateChoice::Date(day())),
            json!({ "date": "2026-11-25T14:00:00" })
        );
        let t = NaiveTime::from_hms_opt(9, 5, 30);
        assert_eq!(
            due_args(t, &DateChoice::Date(day())),
            json!({ "date": "2026-11-25T09:05:30" })
        );
        assert_eq!(
            due_args(None, &DateChoice::Date(day())),
            json!({ "date": "2026-11-25" })
        );
    }

    #[test]
    fn typed_text_and_clearing() {
        assert_eq!(
            due_args(None, &DateChoice::Text("every monday".into())),
            json!({ "string": "every monday" })
        );
        // Text keeps no time of its own: the server parses it.
        assert_eq!(
            due_args(
                NaiveTime::from_hms_opt(14, 0, 0),
                &DateChoice::Text("fri".into())
            ),
            json!({ "string": "fri" })
        );
        assert_eq!(due_args(None, &DateChoice::Clear), Value::Null);
    }

    #[test]
    fn deadlines_are_plain_dates() {
        assert_eq!(
            deadline_args(&DateChoice::Date(day())),
            json!({ "date": "2026-11-25" })
        );
        assert_eq!(deadline_args(&DateChoice::Clear), Value::Null);
        assert_eq!(deadline_args(&DateChoice::Text("x".into())), Value::Null);
    }
}
