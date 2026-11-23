//! The task detail pane (`DESIGN.md` 4.8): project crumb, ring and title, Markdown
//! description, field rows, sub-tasks and comments, with the comment composer in the
//! bottom bar.
//!
//! The pane is built once and updated in place ([`TaskDetail::set_data`]) after every local
//! change and sync: a focused title or description, an open comment editor and the field
//! row an open popover points at are never replaced, so the user keeps typing while syncs
//! arrive. Edits leave as [`DetailEvent`]s; the window turns them into queued commands.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use gtk::glib;
use serde_json::json;
use tick_core::Command;

use crate::i18n::{fill, gettext};
use crate::markdown;
use crate::rows::{DueState, color_class};

/// The editable fields that open a picker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Date,
    Deadline,
    Priority,
    Labels,
    Assignee,
    Move,
}

/// What the pane asks the window to do.
pub enum DetailEvent {
    /// Queue these commands (optimistic, synced).
    Commands(Vec<Command>),
    /// Open the picker for `field`, anchored to `anchor`.
    Pick { field: Field, anchor: gtk::Widget },
    /// The clear button of a field row.
    Clear(Field),
    /// The ring of the open task or of a sub-task.
    Complete { id: String, done: bool },
    /// Open another task (a sub-task).
    Open(String),
    /// Open a link or attachment.
    OpenUri(String),
    /// Ask before deleting a comment.
    DeleteComment(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldDue {
    pub text: String,
    /// The recurrence as typed ("every monday"), for recurring tasks.
    pub recurrence: Option<String>,
    pub state: DueState,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubtaskRow {
    pub id: String,
    pub title: String,
    pub p: u8,
    pub checked: bool,
    pub due: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommentRow {
    pub id: String,
    pub author: String,
    /// Written by the user (editable).
    pub own: bool,
    pub time: String,
    pub text: String,
    /// File name and URL of an existing attachment.
    pub attachment: Option<(String, String)>,
}

/// Everything the pane shows, computed from the store by the window.
#[derive(Debug, Clone, PartialEq)]
pub struct DetailData {
    pub id: String,
    pub title: String,
    pub description: String,
    pub p: u8,
    pub checked: bool,
    /// "Vacation / Preparation" or "Inbox".
    pub crumb: String,
    pub color: String,
    pub shared: bool,
    pub due: Option<FieldDue>,
    pub deadline: Option<(String, bool)>,
    pub labels: Vec<String>,
    pub assignee: Option<String>,
    pub subtasks: Vec<SubtaskRow>,
    /// (done, total) direct sub-tasks.
    pub subtask_counts: Option<(usize, usize)>,
    pub comments: Vec<CommentRow>,
}

struct FieldRow {
    root: gtk::Box,
    button: gtk::Button,
    icon_holder: adw::Bin,
    icon: gtk::Image,
    value: gtk::Label,
    extra: gtk::Label,
    clear: gtk::Button,
}

pub struct TaskDetail {
    pub root: gtk::ScrolledWindow,
    pub composer: gtk::Box,
    events: Rc<dyn Fn(DetailEvent)>,
    data: RefCell<Option<DetailData>>,
    crumb_dot: gtk::Box,
    crumb_label: gtk::Label,
    crumb: gtk::Button,
    ring: gtk::CheckButton,
    ring_handler: RefCell<Option<glib::SignalHandlerId>>,
    title: gtk::TextView,
    desc_stack: gtk::Stack,
    desc_label: gtk::Label,
    desc_view: gtk::TextView,
    fields: Vec<(Field, FieldRow)>,
    sub_heading: gtk::Label,
    sub_count: gtk::Label,
    sub_list: gtk::Box,
    sub_entry: gtk::Entry,
    comments_count: gtk::Label,
    comments: gtk::Box,
    /// The composer's multi-line text (Enter sends, Shift+Enter breaks the line; 4.8).
    comment_view: gtk::TextView,
    /// A comment being edited (its rows are not rebuilt meanwhile).
    editing_comment: RefCell<Option<String>>,
    /// The monospace family for code spans (2.3).
    mono: RefCell<String>,
    reverting: Cell<bool>,
}

fn label(text: &str, classes: &[&str]) -> gtk::Label {
    gtk::Label::builder()
        .label(text)
        .xalign(0.0)
        .css_classes(classes.iter().map(|c| c.to_string()).collect::<Vec<_>>())
        .build()
}

fn icon(name: &str) -> gtk::Image {
    gtk::Image::builder()
        .icon_name(name)
        .accessible_role(gtk::AccessibleRole::Presentation)
        .build()
}

fn buffer_text(v: &gtk::TextView) -> String {
    let b = v.buffer();
    b.text(&b.start_iter(), &b.end_iter(), false).to_string()
}

fn field_name(f: Field) -> String {
    match f {
        Field::Date => gettext("Date"),
        Field::Deadline => gettext("Deadline"),
        Field::Priority => gettext("Priority"),
        Field::Labels => gettext("Labels"),
        Field::Assignee => gettext("Assignee"),
        Field::Move => gettext("Project"),
    }
}

/// The field's icon; the priority row shows a 16 px ring instead (4.8).
fn field_icon(f: Field) -> Option<&'static str> {
    match f {
        Field::Date => Some("tick-due-symbolic"),
        Field::Deadline => Some("tick-deadline-symbolic"),
        Field::Priority => None,
        Field::Labels => Some("tick-label-symbolic"),
        Field::Assignee => Some("system-users-symbolic"),
        Field::Move => Some("tick-project-symbolic"),
    }
}

/// The `markdown` font family from libadwaita's monospace setting (2.3).
fn monospace_family() -> String {
    let name = adw::StyleManager::default().monospace_font_name();
    let desc = gtk::pango::FontDescription::from_string(&name);
    desc.family()
        .map(|f| f.to_string())
        .filter(|f| !f.is_empty())
        .unwrap_or_else(|| "monospace".into())
}

impl TaskDetail {
    pub fn new(events: Rc<dyn Fn(DetailEvent)>) -> Rc<TaskDetail> {
        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin_start(18)
            .margin_end(18)
            .margin_top(6)
            .margin_bottom(24)
            .css_classes(["tick-detail"])
            .build();

        // Project crumb -> "Move to..." (4.8).
        let crumb_box = gtk::Box::new(gtk::Orientation::Horizontal, 9);
        let crumb_dot = gtk::Box::builder()
            .valign(gtk::Align::Center)
            .accessible_role(gtk::AccessibleRole::Presentation)
            .css_classes(["tick-dot-8"])
            .build();
        crumb_box.append(&crumb_dot);
        let crumb_label = label("", &["body"]);
        crumb_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        crumb_box.append(&crumb_label);
        let crumb = gtk::Button::builder()
            .child(&crumb_box)
            .halign(gtk::Align::Start)
            .css_classes(["flat", "tick-crumb"])
            .build();
        // Pickers anchor to this box: a popover inside a button would inherit its bold labels.
        let crumb_holder = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        crumb_holder.append(&crumb);
        content.append(&crumb_holder);

        // Ring + title.
        let head = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .margin_top(6)
            .build();
        let ring = gtk::CheckButton::builder()
            .valign(gtk::Align::Start)
            .css_classes(["tick-ring", "p4"])
            .build();
        head.append(&ring);
        let title = gtk::TextView::builder()
            .wrap_mode(gtk::WrapMode::WordChar)
            .hexpand(true)
            .accepts_tab(false)
            .top_margin(9)
            .bottom_margin(9)
            .left_margin(9)
            .right_margin(9)
            .css_classes(["title-3", "tick-title-edit"])
            .build();
        title.update_property(&[gtk::accessible::Property::Label(&gettext("Task title"))]);
        head.append(&title);
        content.append(&head);

        // Description: rendered Markdown, or a plain-text editor (4.8).
        let desc_label = gtk::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .wrap_mode(gtk::pango::WrapMode::WordChar)
            .use_markup(true)
            .focusable(true)
            .css_classes(["body", "tick-description"])
            .build();
        // A label's accessible name is its text ("Add a description" when empty).
        let desc_view = gtk::TextView::builder()
            .wrap_mode(gtk::WrapMode::WordChar)
            .accepts_tab(false)
            .top_margin(6)
            .bottom_margin(6)
            .left_margin(9)
            .right_margin(9)
            .css_classes(["body", "tick-description-edit"])
            .build();
        desc_view.update_property(&[gtk::accessible::Property::Label(&gettext("Description"))]);
        let desc_stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::Crossfade)
            .transition_duration(if crate::test_mode::enabled() { 0 } else { 150 })
            .vhomogeneous(false)
            .interpolate_size(false)
            .margin_start(44)
            .build();
        desc_stack.add_named(&desc_label, Some("view"));
        desc_stack.add_named(&desc_view, Some("edit"));
        content.append(&desc_stack);

        // Field rows.
        let fields_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin_top(12)
            .build();
        let mut fields = Vec::new();
        for f in [
            Field::Date,
            Field::Deadline,
            Field::Priority,
            Field::Labels,
            Field::Assignee,
        ] {
            let row = Self::field_row(f);
            fields_box.append(&row.root);
            fields.push((f, row));
        }
        content.append(&fields_box);

        // Sub-tasks.
        let sub_head = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(9)
            .margin_top(24)
            .margin_start(12)
            .build();
        let sub_heading = label(&gettext("Sub-tasks"), &["heading"]);
        sub_head.append(&sub_heading);
        let sub_count = label("", &["caption", "dimmed", "numeric"]);
        sub_head.append(&sub_count);
        content.append(&sub_head);
        let sub_list = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin_top(6)
            .build();
        content.append(&sub_list);
        let sub_entry = gtk::Entry::builder()
            .placeholder_text(gettext("Sub-task name"))
            .css_classes(["tick-entry"])
            .visible(false)
            .margin_start(44)
            .build();
        sub_entry.update_property(&[gtk::accessible::Property::Label(&gettext("Sub-task name"))]);
        content.append(&sub_entry);
        let sub_ghost = gtk::Button::builder()
            .css_classes(["flat", "tick-ghost-button"])
            .build();
        let ghost_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let plus = icon("list-add-symbolic");
        plus.set_size_request(44, 44);
        plus.add_css_class("dimmed");
        ghost_box.append(&plus);
        ghost_box.append(&label(&gettext("Add sub-task"), &["body", "dimmed"]));
        sub_ghost.set_child(Some(&ghost_box));
        sub_ghost.update_property(&[gtk::accessible::Property::Label(&gettext("Add sub-task"))]);
        content.append(&sub_ghost);

        // Comments.
        let com_head = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(9)
            .margin_top(24)
            .margin_start(12)
            .build();
        com_head.append(&label(&gettext("Comments"), &["heading"]));
        let comments_count = label("", &["caption", "dimmed", "numeric"]);
        com_head.append(&comments_count);
        content.append(&com_head);
        // The 24 px avatars and the 8 px gap after them put the comment text on the title's
        // column (12 + 24 + 8 = 44, 4.8).
        let comments = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .margin_top(6)
            .margin_start(12)
            .build();
        content.append(&comments);

        let clamp = adw::Clamp::builder()
            .maximum_size(420)
            .child(&content)
            .build();
        let root = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .focusable(false)
            .child(&clamp)
            .build();

        // Bottom bar: the composer (4.8).
        let composer = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .margin_start(12)
            .margin_end(12)
            .margin_top(6)
            .margin_bottom(6)
            .css_classes(["tick-composer"])
            .build();
        // A multi-line field drawn like an entry: a text view (grows to 6 lines, then
        // scrolls) with a placeholder label on top while it is empty.
        let comment_view = gtk::TextView::builder()
            .wrap_mode(gtk::WrapMode::WordChar)
            .accepts_tab(false)
            .hexpand(true)
            .top_margin(12)
            .bottom_margin(12)
            .left_margin(9)
            .right_margin(9)
            .css_classes(["body"])
            .build();
        comment_view.update_property(&[gtk::accessible::Property::Label(&gettext(
            "Write a comment",
        ))]);
        // No scrollbar: a vertical one would hold the field at its own minimum height (58 px),
        // above the 44 px of one line; the text still scrolls with the cursor and the wheel.
        let comment_scroller = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::External)
            .propagate_natural_height(true)
            .max_content_height(140)
            .focusable(false)
            .child(&comment_view)
            .build();
        let placeholder = gtk::Label::builder()
            .label(gettext("Write a comment"))
            .xalign(0.0)
            .halign(gtk::Align::Start)
            .valign(gtk::Align::Start)
            .margin_start(9)
            .margin_top(12)
            .can_target(false)
            .accessible_role(gtk::AccessibleRole::Presentation)
            .css_classes(["body", "dimmed"])
            .build();
        let field = gtk::Overlay::builder()
            .child(&comment_scroller)
            .hexpand(true)
            .css_classes(["tick-composer-field"])
            .build();
        field.add_overlay(&placeholder);
        {
            let placeholder = placeholder.downgrade();
            comment_view.buffer().connect_changed(move |b| {
                if let Some(p) = placeholder.upgrade() {
                    p.set_visible(b.char_count() == 0);
                }
            });
        }
        composer.append(&field);
        let send = gtk::Button::builder()
            .icon_name("tick-send-symbolic")
            .valign(gtk::Align::Center)
            .css_classes(["flat", "tick-icon-44"])
            .tooltip_text(gettext("Send Comment"))
            .build();
        send.update_property(&[gtk::accessible::Property::Label(&gettext("Send Comment"))]);
        composer.append(&send);

        let detail = Rc::new(TaskDetail {
            root,
            composer,
            events,
            data: RefCell::default(),
            crumb_dot,
            crumb_label,
            crumb,
            ring,
            ring_handler: RefCell::default(),
            title,
            desc_stack,
            desc_label,
            desc_view,
            fields,
            sub_heading,
            sub_count,
            sub_list,
            sub_entry,
            comments_count,
            comments,
            comment_view,
            editing_comment: RefCell::default(),
            mono: RefCell::new(monospace_family()),
            reverting: Cell::new(false),
        });
        detail.connect(&send, &sub_ghost);
        detail
    }

    fn field_row(f: Field) -> FieldRow {
        let root = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .css_classes(["tick-field"])
            .build();
        let inner = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let icon_w = gtk::Image::builder()
            .accessible_role(gtk::AccessibleRole::Presentation)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .build();
        if let Some(name) = field_icon(f) {
            icon_w.set_icon_name(Some(name));
        }
        let icon_holder = adw::Bin::builder()
            .width_request(16)
            .accessible_role(gtk::AccessibleRole::Presentation)
            .child(&icon_w)
            .build();
        inner.append(&icon_holder);
        let name = label(&field_name(f), &["body", "dimmed"]);
        name.set_width_request(96);
        inner.append(&name);
        let values = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .valign(gtk::Align::Center)
            .hexpand(true)
            .build();
        let value = label("", &["body"]);
        value.set_ellipsize(gtk::pango::EllipsizeMode::End);
        values.append(&value);
        let extra = label("", &["caption", "dimmed"]);
        extra.set_visible(false);
        extra.set_ellipsize(gtk::pango::EllipsizeMode::End);
        values.append(&extra);
        inner.append(&values);
        let button = gtk::Button::builder()
            .child(&inner)
            .hexpand(true)
            .css_classes(["flat", "tick-field-row"])
            .build();
        root.append(&button);
        let clear = gtk::Button::builder()
            .icon_name("edit-clear-symbolic")
            .valign(gtk::Align::Center)
            .css_classes(["flat", "tick-icon-44", "tick-reveal"])
            .tooltip_text(fill(
                &gettext("Clear {field}"),
                &[("field", &field_name(f))],
            ))
            .build();
        clear.update_property(&[gtk::accessible::Property::Label(&fill(
            &gettext("Clear {field}"),
            &[("field", &field_name(f))],
        ))]);
        root.append(&clear);
        FieldRow {
            root,
            button,
            icon_holder,
            icon: icon_w,
            value,
            extra,
            clear,
        }
    }

    fn emit(&self, e: DetailEvent) {
        (self.events)(e);
    }

    /// The open task's id.
    pub fn task_id(&self) -> Option<String> {
        self.data.borrow().as_ref().map(|d| d.id.clone())
    }

    /// Commands for the open task.
    fn commands(&self, f: impl FnOnce(&str) -> Vec<Command>) {
        if let Some(id) = self.task_id() {
            let cmds = f(&id);
            if !cmds.is_empty() {
                self.emit(DetailEvent::Commands(cmds));
            }
        }
    }

    fn connect(self: &Rc<Self>, send: &gtk::Button, sub_ghost: &gtk::Button) {
        let weak = Rc::downgrade(self);
        self.crumb.connect_clicked(move |b| {
            if let Some(d) = weak.upgrade()
                && let Some(anchor) = b.parent()
            {
                d.emit(DetailEvent::Pick {
                    field: Field::Move,
                    anchor,
                });
            }
        });
        for (f, row) in &self.fields {
            let f = *f;
            let weak = Rc::downgrade(self);
            let anchor = row.root.clone();
            row.button.connect_clicked(move |_| {
                if let Some(d) = weak.upgrade() {
                    d.emit(DetailEvent::Pick {
                        field: f,
                        anchor: anchor.clone().upcast(),
                    });
                }
            });
            let weak = Rc::downgrade(self);
            row.clear.connect_clicked(move |_| {
                if let Some(d) = weak.upgrade() {
                    d.emit(DetailEvent::Clear(f));
                }
            });
        }
        let weak = Rc::downgrade(self);
        let id = self.ring.connect_toggled(move |r| {
            if let Some(d) = weak.upgrade()
                && let Some(id) = d.task_id()
            {
                d.emit(DetailEvent::Complete {
                    id,
                    done: r.is_active(),
                });
            }
        });
        self.ring_handler.replace(Some(id));

        // Title: Enter commits (no newlines), Escape reverts, focus-out commits (4.8).
        let keys = gtk::EventControllerKey::new();
        let weak = Rc::downgrade(self);
        keys.connect_key_pressed(move |_, key, _, _| {
            let Some(d) = weak.upgrade() else {
                return glib::Propagation::Proceed;
            };
            match key {
                gtk::gdk::Key::Return | gtk::gdk::Key::KP_Enter => {
                    d.commit_title();
                    glib::Propagation::Stop
                }
                gtk::gdk::Key::Escape => {
                    d.revert_title();
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        });
        self.title.add_controller(keys);
        let focus = gtk::EventControllerFocus::new();
        let weak = Rc::downgrade(self);
        focus.connect_leave(move |_| {
            if let Some(d) = weak.upgrade() {
                d.commit_title();
            }
        });
        self.title.add_controller(focus);

        // Description: click (not on a link) or Enter edits; Ctrl+Enter or focus-out saves.
        let click = gtk::GestureClick::new();
        let weak = Rc::downgrade(self);
        click.connect_released(move |g, _, _, _| {
            let Some(d) = weak.upgrade() else { return };
            if d.desc_label.current_uri().is_some() {
                return;
            }
            g.set_state(gtk::EventSequenceState::Claimed);
            d.edit_description();
        });
        self.desc_label.add_controller(click);
        let keys = gtk::EventControllerKey::new();
        let weak = Rc::downgrade(self);
        keys.connect_key_pressed(move |_, key, _, _| {
            let Some(d) = weak.upgrade() else {
                return glib::Propagation::Proceed;
            };
            if matches!(key, gtk::gdk::Key::Return | gtk::gdk::Key::KP_Enter) {
                d.edit_description();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        self.desc_label.add_controller(keys);
        let weak = Rc::downgrade(self);
        self.desc_label.connect_activate_link(move |_, uri| {
            if let Some(d) = weak.upgrade() {
                d.emit(DetailEvent::OpenUri(uri.to_owned()));
            }
            glib::Propagation::Stop
        });
        let keys = gtk::EventControllerKey::new();
        let weak = Rc::downgrade(self);
        keys.connect_key_pressed(move |_, key, _, mods| {
            let Some(d) = weak.upgrade() else {
                return glib::Propagation::Proceed;
            };
            match key {
                gtk::gdk::Key::Return | gtk::gdk::Key::KP_Enter
                    if mods.contains(gtk::gdk::ModifierType::CONTROL_MASK) =>
                {
                    d.commit_description(true);
                    glib::Propagation::Stop
                }
                gtk::gdk::Key::Escape => {
                    d.reverting.set(true);
                    d.show_description_view(true);
                    d.reverting.set(false);
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        });
        self.desc_view.add_controller(keys);
        let focus = gtk::EventControllerFocus::new();
        let weak = Rc::downgrade(self);
        focus.connect_leave(move |_| {
            if let Some(d) = weak.upgrade()
                && !d.reverting.get()
            {
                d.commit_description(false);
            }
        });
        self.desc_view.add_controller(focus);

        // Sub-tasks: "Add sub-task" shows an entry; Enter adds and keeps it for the next.
        let weak = Rc::downgrade(self);
        sub_ghost.connect_clicked(move |_| {
            if let Some(d) = weak.upgrade() {
                d.sub_entry.set_visible(true);
                d.sub_entry.grab_focus();
            }
        });
        let weak = Rc::downgrade(self);
        self.sub_entry.connect_activate(move |e| {
            let Some(d) = weak.upgrade() else { return };
            let text = e.text().trim().to_owned();
            if text.is_empty() {
                e.set_visible(false);
                return;
            }
            e.set_text("");
            d.commands(|id| vec![Command::item_add(json!({"content": text, "parent_id": id}))]);
        });
        let keys = gtk::EventControllerKey::new();
        let weak = Rc::downgrade(self);
        keys.connect_key_pressed(move |_, key, _, _| {
            if key == gtk::gdk::Key::Escape
                && let Some(d) = weak.upgrade()
            {
                d.sub_entry.set_text("");
                d.sub_entry.set_visible(false);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        self.sub_entry.add_controller(keys);

        // Composer: Enter or Ctrl+Enter sends, Shift+Enter breaks the line (4.8).
        let send_comment = {
            let weak = Rc::downgrade(self);
            move || {
                let Some(d) = weak.upgrade() else { return };
                let text = buffer_text(&d.comment_view).trim().to_owned();
                if text.is_empty() {
                    return;
                }
                d.comment_view.buffer().set_text("");
                d.commands(|id| vec![Command::note_add(id, &text)]);
            }
        };
        let keys = gtk::EventControllerKey::new();
        let s = send_comment.clone();
        keys.connect_key_pressed(move |_, key, _, mods| {
            if matches!(key, gtk::gdk::Key::Return | gtk::gdk::Key::KP_Enter)
                && !mods.contains(gtk::gdk::ModifierType::SHIFT_MASK)
            {
                s();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        self.comment_view.add_controller(keys);
        send.connect_clicked(move |_| send_comment());

        let weak = Rc::downgrade(self);
        adw::StyleManager::default().connect_monospace_font_name_notify(move |_| {
            if let Some(d) = weak.upgrade() {
                d.mono.replace(monospace_family());
                d.render_description();
            }
        });
    }

    fn commit_title(&self) {
        let Some(data) = self.data.borrow().clone() else {
            return;
        };
        let text = buffer_text(&self.title)
            .replace('\n', " ")
            .trim()
            .to_owned();
        if text.is_empty() {
            self.revert_title();
            return;
        }
        if text != data.title {
            self.data.borrow_mut().as_mut().unwrap().title = text.clone();
            self.emit(DetailEvent::Commands(vec![Command::item_update(
                &data.id,
                json!({ "content": text }),
            )]));
        }
    }

    fn revert_title(&self) {
        if let Some(d) = self.data.borrow().as_ref() {
            self.title.buffer().set_text(&d.title);
        }
    }

    fn edit_description(&self) {
        let Some(d) = self.data.borrow().clone() else {
            return;
        };
        self.desc_view.buffer().set_text(&d.description);
        self.desc_stack.set_visible_child_name("edit");
        self.desc_view.grab_focus();
    }

    fn show_description_view(&self, focus: bool) {
        self.desc_stack.set_visible_child_name("view");
        if focus {
            self.desc_label.grab_focus();
        }
    }

    fn commit_description(&self, focus: bool) {
        if self.desc_stack.visible_child_name().as_deref() != Some("edit") {
            return;
        }
        let Some(data) = self.data.borrow().clone() else {
            return;
        };
        let text = buffer_text(&self.desc_view).trim_end().to_owned();
        if text != data.description {
            if let Some(d) = self.data.borrow_mut().as_mut() {
                d.description = text.clone();
            }
            self.render_description();
            self.emit(DetailEvent::Commands(vec![Command::item_update(
                &data.id,
                json!({ "description": text }),
            )]));
        }
        self.show_description_view(focus);
    }

    fn render_description(&self) {
        let Some(d) = self.data.borrow().clone() else {
            return;
        };
        if d.description.trim().is_empty() {
            self.desc_label
                .set_markup(&markdown::escape(&gettext("Add a description")));
            self.desc_label.add_css_class("dimmed");
        } else {
            self.desc_label
                .set_markup(&markdown::to_pango(&d.description, &self.mono.borrow()));
            self.desc_label.remove_css_class("dimmed");
        }
    }

    /// The open task's temp id `old` was replaced by the server's `new` (`id_map`): it is
    /// the same task, so nothing the user is editing is reset. Call before [`Self::set_data`]
    /// with the data read under the new id.
    pub fn rename_id(&self, old: &str, new: &str) {
        if let Some(d) = self.data.borrow_mut().as_mut()
            && d.id == old
        {
            d.id = new.to_owned();
        }
    }

    /// Show `data` (or nothing). A different task resets every editor; the same task is
    /// updated in place, leaving what the user is editing alone.
    pub fn set_data(self: &Rc<Self>, data: Option<DetailData>) {
        let Some(data) = data else {
            self.data.replace(None);
            return;
        };
        let old = self.data.borrow().clone();
        if old.as_ref() == Some(&data) {
            return;
        }
        let same_task = old.as_ref().is_some_and(|o| o.id == data.id);
        self.data.replace(Some(data.clone()));
        if !same_task {
            self.desc_stack.set_visible_child_name("view");
            self.sub_entry.set_visible(false);
            self.sub_entry.set_text("");
            self.comment_view.buffer().set_text("");
            self.editing_comment.replace(None);
            self.root.vadjustment().set_value(0.0);
        }

        // Crumb.
        self.crumb_label.set_label(&data.crumb);
        self.crumb_dot
            .set_css_classes(&["tick-dot-8", &color_class(&data.color)]);
        self.crumb
            .update_property(&[gtk::accessible::Property::Label(&fill(
                &gettext("Move to... (now {project})"),
                &[("project", &data.crumb)],
            ))]);

        // Ring.
        if let Some(h) = self.ring_handler.borrow().as_ref() {
            self.ring.block_signal(h);
            self.ring.set_active(data.checked);
            self.ring.unblock_signal(h);
        }
        self.ring
            .set_css_classes(&["tick-ring", &format!("p{}", data.p)]);
        let ring_name = if data.checked {
            gettext("Mark as not done")
        } else {
            gettext("Complete task")
        };
        self.ring.update_property(&[
            gtk::accessible::Property::Label(&ring_name),
            gtk::accessible::Property::Description(&data.title),
        ]);

        // Title: never under the user's cursor.
        // Being edited: focused with uncommitted text (after Enter the buffer equals the
        // committed title, so a rollback from the server still shows).
        let editing_title = same_task
            && self.title.has_focus()
            && old
                .as_ref()
                .is_some_and(|o| buffer_text(&self.title) != o.title);
        if !editing_title && buffer_text(&self.title) != data.title {
            self.title.buffer().set_text(&data.title);
        }

        // Description.
        if !(same_task && self.desc_stack.visible_child_name().as_deref() == Some("edit")) {
            self.render_description();
        }

        // Fields.
        for (f, row) in &self.fields {
            self.update_field(*f, row, &data);
        }

        // Sub-tasks and comments: rebuilt when they changed, unless being edited.
        if !same_task || old.as_ref().map(|o| &o.subtasks) != Some(&data.subtasks) {
            self.rebuild_subtasks(&data);
        }
        self.sub_count.set_label(
            &data
                .subtask_counts
                .map(|(d, t)| format!("{d}/{t}"))
                .unwrap_or_default(),
        );
        self.sub_heading.set_label(&gettext("Sub-tasks"));
        self.comments_count.set_label(&if data.comments.is_empty() {
            String::new()
        } else {
            data.comments.len().to_string()
        });
        let comments_changed = old.as_ref().map(|o| &o.comments) != Some(&data.comments);
        if (!same_task || comments_changed) && self.editing_comment.borrow().is_none() {
            self.rebuild_comments(&data);
        }
    }

    fn update_field(&self, f: Field, row: &FieldRow, d: &DetailData) {
        row.value.set_css_classes(&["body"]);
        row.extra.set_visible(false);
        let (text, has, classes): (String, bool, Vec<&str>) = match f {
            Field::Date => match &d.due {
                Some(due) => {
                    if let Some(r) = &due.recurrence {
                        row.extra.set_label(r);
                        row.extra.set_visible(true);
                    }
                    row.icon.set_icon_name(Some(if due.recurrence.is_some() {
                        "media-playlist-repeat-symbolic"
                    } else {
                        "tick-due-symbolic"
                    }));
                    let class = match due.state {
                        DueState::Overdue => vec!["tick-due", "overdue"],
                        DueState::Today => vec!["tick-due", "today"],
                        DueState::Other => vec![],
                    };
                    (due.text.clone(), true, class)
                }
                None => {
                    row.icon.set_icon_name(Some("tick-due-symbolic"));
                    (gettext("No date"), false, vec![])
                }
            },
            Field::Deadline => match &d.deadline {
                Some((t, overdue)) => (
                    t.clone(),
                    true,
                    if *overdue {
                        vec!["tick-deadline", "overdue"]
                    } else {
                        vec![]
                    },
                ),
                None => (gettext("No deadline"), false, vec![]),
            },
            Field::Priority => {
                row.icon
                    .set_css_classes(&["tick-ring-16", &format!("p{}", d.p)]);
                (
                    fill(&gettext("Priority {n}"), &[("n", &d.p.to_string())]),
                    d.p != 4,
                    vec![],
                )
            }
            Field::Labels => {
                if d.labels.is_empty() {
                    (gettext("No labels"), false, vec![])
                } else {
                    (d.labels.join(", "), true, vec![])
                }
            }
            Field::Assignee => {
                row.root.set_visible(d.shared);
                // The person's 20 px avatar in the icon column (4.8); the icon when unassigned.
                match &d.assignee {
                    Some(n) => {
                        // Decoration: the row's name already says who (accessible-role is
                        // construct-only).
                        let avatar = glib::Object::builder::<adw::Avatar>()
                            .property("size", 20)
                            .property("text", n)
                            .property("show-initials", true)
                            .property("accessible-role", gtk::AccessibleRole::Presentation)
                            .build();
                        avatar.add_css_class("tick-field-avatar");

                        row.icon_holder.set_child(Some(&avatar));
                        (n.clone(), true, vec![])
                    }
                    None => {
                        row.icon_holder.set_child(Some(&row.icon));
                        (gettext("Unassigned"), false, vec![])
                    }
                }
            }
            Field::Move => (d.crumb.clone(), true, vec![]),
        };
        row.value.set_label(&text);
        if !has {
            row.value.add_css_class("dimmed");
        }
        for c in classes {
            row.value.add_css_class(c);
        }
        row.clear.set_visible(has);
        row.button
            .update_property(&[gtk::accessible::Property::Label(&fill(
                &gettext("{field}: {value}"),
                &[("field", &field_name(f)), ("value", &text)],
            ))]);
    }

    fn rebuild_subtasks(self: &Rc<Self>, d: &DetailData) {
        while let Some(c) = self.sub_list.first_child() {
            self.sub_list.remove(&c);
        }
        for s in &d.subtasks {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            let ring = gtk::CheckButton::builder()
                .valign(gtk::Align::Start)
                .active(s.checked)
                .css_classes(["tick-ring".to_owned(), format!("p{}", s.p)])
                .build();
            ring.update_property(&[
                gtk::accessible::Property::Label(&if s.checked {
                    gettext("Mark as not done")
                } else {
                    gettext("Complete task")
                }),
                gtk::accessible::Property::Description(&s.title),
            ]);
            let weak = Rc::downgrade(self);
            let id = s.id.clone();
            ring.connect_toggled(move |r| {
                if let Some(d) = weak.upgrade() {
                    d.emit(DetailEvent::Complete {
                        id: id.clone(),
                        done: r.is_active(),
                    });
                }
            });
            row.append(&ring);
            let text = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .valign(gtk::Align::Center)
                .build();
            let t = label(&s.title, &["body"]);
            t.set_wrap(true);
            t.set_wrap_mode(gtk::pango::WrapMode::WordChar);
            text.append(&t);
            if let Some(due) = &s.due {
                text.append(&label(due, &["caption", "dimmed"]));
            }
            let open = gtk::Button::builder()
                .child(&text)
                .hexpand(true)
                .css_classes(["flat", "tick-subtask-open"])
                .build();
            open.update_property(&[gtk::accessible::Property::Label(&fill(
                &gettext("Open {title}"),
                &[("title", &s.title)],
            ))]);
            let weak = Rc::downgrade(self);
            let id = s.id.clone();
            open.connect_clicked(move |_| {
                if let Some(d) = weak.upgrade() {
                    d.emit(DetailEvent::Open(id.clone()));
                }
            });
            row.append(&open);
            self.sub_list.append(&row);
        }
    }

    fn rebuild_comments(self: &Rc<Self>, d: &DetailData) {
        while let Some(c) = self.comments.first_child() {
            self.comments.remove(&c);
        }
        for c in &d.comments {
            self.comments.append(&self.comment_widget(c));
        }
    }

    fn comment_widget(self: &Rc<Self>, c: &CommentRow) -> gtk::Widget {
        let root = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .css_classes(["tick-comment"])
            .build();
        let avatar = adw::Avatar::new(24, Some(&c.author), true);
        avatar.set_valign(gtk::Align::Start);
        avatar.set_margin_top(10);
        avatar
            .upcast_ref::<gtk::Widget>()
            .update_property(&[gtk::accessible::Property::Label(&c.author)]);
        root.append(&avatar);
        let body = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .hexpand(true)
            .build();
        let head = gtk::Box::new(gtk::Orientation::Horizontal, 9);
        let name = label(&c.author, &["heading"]);
        name.set_ellipsize(gtk::pango::EllipsizeMode::End);
        head.append(&name);
        let time = label(&c.time, &["caption", "dimmed", "numeric"]);
        time.set_hexpand(true);
        head.append(&time);
        if c.own {
            let menu = comment_menu();
            let button = gtk::MenuButton::builder()
                .icon_name("view-more-symbolic")
                .menu_model(&menu)
                .css_classes(["flat", "tick-icon-44", "tick-reveal"])
                .tooltip_text(gettext("Comment Actions"))
                .build();
            button.update_property(&[gtk::accessible::Property::Label(&gettext(
                "Comment Actions",
            ))]);
            crate::menus::name_button(&button);
            let actions = gtk::gio::SimpleActionGroup::new();
            let weak = Rc::downgrade(self);
            let id = c.id.clone();
            let text = c.text.clone();
            let body_weak = body.downgrade();
            let edit = gtk::gio::ActionEntry::builder("edit")
                .activate(move |_: &gtk::gio::SimpleActionGroup, _, _| {
                    if let (Some(d), Some(b)) = (weak.upgrade(), body_weak.upgrade()) {
                        d.edit_comment(&b, &id, &text);
                    }
                })
                .build();
            let weak = Rc::downgrade(self);
            let id = c.id.clone();
            let delete = gtk::gio::ActionEntry::builder("delete")
                .activate(move |_: &gtk::gio::SimpleActionGroup, _, _| {
                    if let Some(d) = weak.upgrade() {
                        d.emit(DetailEvent::DeleteComment(id.clone()));
                    }
                })
                .build();
            actions.add_action_entries([edit, delete]);
            root.insert_action_group("comment", Some(&actions));
            head.append(&button);
        } else {
            // Keep the header height when there is no menu button.
            head.set_size_request(-1, 44);
        }
        body.append(&head);
        if !c.text.is_empty() {
            let text = gtk::Label::builder()
                .label(&c.text)
                .xalign(0.0)
                .wrap(true)
                .wrap_mode(gtk::pango::WrapMode::WordChar)
                .selectable(true)
                .css_classes(["body", "tick-comment-text"])
                .build();
            body.append(&text);
        }
        if let Some((file, url)) = &c.attachment {
            let b = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            b.append(&icon("tick-attachment-symbolic"));
            let l = label(file, &["body"]);
            l.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
            b.append(&l);
            let button = gtk::Button::builder()
                .child(&b)
                .css_classes(["flat", "tick-attachment"])
                .margin_top(6)
                .build();
            button.update_property(&[gtk::accessible::Property::Label(&fill(
                &gettext("Open attachment {name}"),
                &[("name", file)],
            ))]);
            let weak = Rc::downgrade(self);
            let url = url.clone();
            button.connect_clicked(move |_| {
                if let Some(d) = weak.upgrade() {
                    d.emit(DetailEvent::OpenUri(url.clone()));
                }
            });
            body.append(&button);
        }
        root.append(&body);
        root.upcast()
    }

    /// Edit a comment in place: Enter saves, Escape cancels.
    fn edit_comment(self: &Rc<Self>, body: &gtk::Box, id: &str, text: &str) {
        self.editing_comment.replace(Some(id.to_owned()));
        // Replace the text label with an entry.
        let mut child = body.first_child();
        let mut old_label = None;
        while let Some(c) = child {
            if c.has_css_class("tick-comment-text") {
                old_label = Some(c.clone());
            }
            child = c.next_sibling();
        }
        let entry = gtk::Entry::builder()
            .text(text)
            .css_classes(["tick-entry"])
            .build();
        entry.update_property(&[gtk::accessible::Property::Label(&gettext("Edit comment"))]);
        match &old_label {
            Some(l) => {
                body.insert_child_after(&entry, Some(l));
                l.set_visible(false);
            }
            None => body.append(&entry),
        }
        entry.grab_focus();
        let finish = {
            let weak = Rc::downgrade(self);
            let id = id.to_owned();
            let original = text.to_owned();
            let entry_weak = entry.downgrade();
            let done = Rc::new(Cell::new(false));
            move |save: bool| {
                let Some(d) = weak.upgrade() else { return };
                if done.replace(true) {
                    return;
                }
                let new = entry_weak
                    .upgrade()
                    .map(|e| e.text().trim().to_owned())
                    .unwrap_or_default();
                d.editing_comment.replace(None);
                if save && !new.is_empty() && new != original {
                    d.emit(DetailEvent::Commands(vec![Command::note_update(&id, &new)]));
                } else if let Some(data) = d.data.borrow().clone() {
                    d.rebuild_comments(&data);
                }
            }
        };
        let f = finish.clone();
        entry.connect_activate(move |_| f(true));
        let keys = gtk::EventControllerKey::new();
        let f = finish.clone();
        keys.connect_key_pressed(move |_, key, _, _| {
            if key == gtk::gdk::Key::Escape {
                f(false);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        entry.add_controller(keys);
    }

    /// Focus the title (a newly opened task, keyboard users).
    pub fn focus_title(&self) {
        self.title.grab_focus();
    }

    /// Whether keyboard focus is in one of the pane's editors (Escape reverts there).
    pub fn is_editing(&self, focus: &gtk::Widget) -> bool {
        focus.is::<gtk::TextView>() || focus.is::<gtk::Text>()
    }

    /// The field row widget for `field` (a picker anchors to it).
    pub fn field_anchor(&self, field: Field) -> Option<gtk::Widget> {
        if field == Field::Move {
            return self.crumb.parent();
        }
        self.fields
            .iter()
            .find(|(f, _)| *f == field)
            .map(|(_, r)| r.root.clone().upcast())
    }
}

fn comment_menu() -> gtk::gio::Menu {
    let menu = gtk::gio::Menu::new();
    menu.append(Some(&gettext("Edit")), Some("comment.edit"));
    let danger = gtk::gio::Menu::new();
    danger.append(Some(&gettext("Delete Comment...")), Some("comment.delete"));
    menu.append_section(None, &danger);
    menu
}

/// A date in a field row, where there is room: "Today", "Tomorrow", "Yesterday", else the
/// weekday and the date ("Tue 24 Nov", "11 月 24 日 周二"; 4.8, 6.3).
fn field_date(
    date: chrono::NaiveDate,
    today: chrono::NaiveDate,
    settings: crate::format::DateSettings,
) -> String {
    use crate::format;
    match (date - today).num_days() {
        -1..=1 => format::date_short(date, today, settings),
        _ => fill(
            &gettext("{weekday} {date}"),
            &[
                (
                    "weekday",
                    &format::weekday_short(chrono::Datelike::weekday(&date)),
                ),
                ("date", &format::date_plain(date, today, settings)),
            ],
        ),
    }
}

/// Read the open task's pane data from the store; `None` when the task is gone.
pub fn load(store: &tick_core::Store, id: &str) -> tick_core::store::Result<Option<DetailData>> {
    use crate::format::{self, DateSettings};
    let Some(t) = store.task(id)? else {
        return Ok(None);
    };
    let now = store.now();
    let today = now.today();
    let user = store.user()?;
    let settings = DateSettings::from_user(user.as_ref());
    let me = user.as_ref().map(|u| u.id.clone());
    let project = store.project(&t.project_id)?;
    let section = match t.section_id.as_deref() {
        Some(s) => store.section(s)?,
        None => None,
    };
    let (pname, color, shared) = match &project {
        Some(p) if p.inbox_project => (gettext("Inbox"), p.color.clone(), p.shared),
        Some(p) => (p.name.clone(), p.color.clone(), p.shared),
        None => (String::new(), "charcoal".into(), false),
    };
    let crumb = match &section {
        Some(s) => format!("{pname} / {}", s.name),
        None => pname,
    };
    let person = |uid: &str| -> Option<String> {
        if me.as_deref() == Some(uid) {
            return user.as_ref().map(|u| u.full_name.clone());
        }
        store.collaborator(uid).ok().flatten().map(|c| c.full_name)
    };
    let due = t.due.map(|d| {
        let state = if d.is_overdue(&now) {
            DueState::Overdue
        } else if d.date == today {
            DueState::Today
        } else {
            DueState::Other
        };
        let date = field_date(d.date, today, settings);
        let text = match d.time {
            Some(tm) => fill(
                &crate::i18n::pgettext("due", "{date} {time}"),
                &[("date", &date), ("time", &format::time(tm, settings))],
            ),
            None => date,
        };
        let recurrence = t
            .item
            .due
            .as_ref()
            .filter(|d| d.is_recurring)
            .map(|d| d.string.clone())
            .filter(|s| !s.is_empty());
        FieldDue {
            text,
            recurrence,
            state,
        }
    });
    // A due typed as text and not parsed yet (offline): show the text.
    let due = due.or_else(|| {
        let d = t.item.due.as_ref()?;
        (!d.string.is_empty()).then(|| FieldDue {
            text: d.string.clone(),
            recurrence: None,
            state: DueState::Other,
        })
    });
    let deadline = t.item.deadline.as_ref().and_then(|dl| {
        let date = chrono::NaiveDate::parse_from_str(dl.date.get(..10)?, "%Y-%m-%d").ok()?;
        Some((field_date(date, today, settings), date < today))
    });
    let subtasks = store
        .subtasks(id)?
        .into_iter()
        .map(|s| SubtaskRow {
            id: s.id.clone(),
            title: s.content.clone(),
            p: s.item.p(),
            checked: s.checked,
            due: s
                .due
                .and_then(|d| format::due(d.date, d.time, today, settings, false)),
        })
        .collect();
    let subtask_counts = store
        .subtask_counts()?
        .get(id)
        .filter(|(open, done)| open + done > 0)
        .map(|(open, done)| (*done, open + done));
    let comments = store
        .comments(id)?
        .into_iter()
        .map(|n| {
            let author = n
                .posted_uid
                .as_deref()
                .and_then(person)
                .unwrap_or_else(|| gettext("Someone"));
            let time = n
                .posted_at
                .as_deref()
                .and_then(|p| chrono::DateTime::parse_from_rfc3339(p).ok())
                .map(|p| {
                    let local = p.with_timezone(&now.offset());
                    format::due(
                        local.date_naive(),
                        Some(local.time()),
                        today,
                        settings,
                        false,
                    )
                    .unwrap_or_default()
                })
                .unwrap_or_default();
            let attachment = n.file_attachment.as_ref().and_then(|a| {
                Some((
                    a.file_name.clone().unwrap_or_else(|| gettext("Attachment")),
                    a.file_url.clone()?,
                ))
            });
            CommentRow {
                own: n.posted_uid.is_some() && n.posted_uid == me,
                id: n.id,
                author,
                time,
                text: n.content,
                attachment,
            }
        })
        .collect();
    Ok(Some(DetailData {
        id: t.id.clone(),
        title: t.content.clone(),
        description: t.description.clone(),
        p: t.item.p(),
        checked: t.checked,
        crumb,
        color,
        shared,
        due,
        deadline,
        labels: t.labels.clone(),
        assignee: t.item.responsible_uid.as_deref().and_then(person),
        subtasks,
        subtask_counts,
        comments,
    }))
}
