//! The task list: one `GtkListView` over a flat model of [`TickRow`]s whose factory builds
//! a widget per row kind (`DESIGN.md` 7.1, 7.4 pitfall 14). Only visible rows are built.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};

use crate::i18n::{fill, gettext, ncount};
use crate::row_object::TickRow;
use crate::rows::{
    DayHeader, DueState, EmptyState, MetaDue, Row, SectionHeader, TaskRow, ViewHeader, color_class,
};

/// What a row asks the window to do.
#[derive(Debug, Clone, PartialEq)]
pub enum RowAction {
    /// The priority ring was toggled.
    Ring {
        id: String,
        active: bool,
    },
    ToggleSubtasks(String),
    ToggleSection(String),
    /// An "Add task" ghost row: open quick add with this text.
    Ghost(String),
    /// A click on a task row: open it in the detail pane.
    Open(String),
    /// Right-click or long-press on a row: its context menu at (x, y) in list coordinates.
    Menu {
        key: String,
        x: f64,
        y: f64,
    },
    /// The inline section editor: save (`id` `None`: a new section) or cancel (4.24).
    SectionCommit {
        id: Option<String>,
        name: String,
    },
    SectionCancel,
    /// The in-place project title editor (4.2): start, save, cancel.
    RenameTitle,
    TitleCommit(String),
    TitleCancel,
    /// A task row drag started or ended.
    DragBegin,
    DragEnd,
    /// The Search view's entry changed (4.15).
    SearchChanged(String),
    /// Enter in the search entry (its text): open the first result.
    SearchActivate(String),
    /// Down in the search entry: focus the first result.
    SearchDown,
    /// Escape in the empty search entry: leave Search.
    SearchEscape,
    /// "Load More" in the Completed view (4.21).
    LoadMore,
}

/// A frameless inline editor: Enter or focus-out commits, Escape cancels. `select` selects
/// the text on focus (a rename).
fn inline_entry(
    text: &str,
    placeholder: &str,
    classes: &[&str],
    select: bool,
    commit: impl Fn(String) + 'static,
    cancel: impl Fn() + 'static,
) -> gtk::Entry {
    let entry = gtk::Entry::builder()
        .text(text)
        .placeholder_text(placeholder)
        .hexpand(true)
        .css_classes(classes.iter().map(|c| c.to_string()).collect::<Vec<_>>())
        .build();
    entry.update_property(&[gtk::accessible::Property::Label(placeholder)]);
    let done = Rc::new(Cell::new(false));
    let commit = Rc::new(commit);
    let cancel = Rc::new(cancel);
    {
        let done = done.clone();
        let commit = commit.clone();
        entry.connect_activate(move |e| {
            if !done.replace(true) {
                commit(e.text().trim().to_owned());
            }
        });
    }
    let keys = gtk::EventControllerKey::new();
    {
        let done = done.clone();
        let cancel = cancel.clone();
        keys.connect_key_pressed(move |_, key, _, _| {
            if key == gtk::gdk::Key::Escape {
                if !done.replace(true) {
                    cancel();
                }
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
    }
    entry.add_controller(keys);
    let focus = gtk::EventControllerFocus::new();
    {
        let done = done.clone();
        focus.connect_leave(move |c| {
            // Focus-out after the row was already finished (or while it is rebuilt) is not a
            // user decision.
            let Some(e) = c.widget().and_downcast::<gtk::Entry>() else {
                return;
            };
            if !e.is_mapped() || done.replace(true) {
                return;
            }
            let text = e.text().trim().to_owned();
            if text.is_empty() {
                cancel();
            } else {
                commit(text);
            }
        });
    }
    entry.add_controller(focus);
    entry.connect_map(move |e| {
        let e = e.clone();
        glib::idle_add_local_once(move || {
            e.grab_focus();
            if select {
                e.select_region(0, -1);
            } else {
                e.set_position(-1);
            }
        });
    });
    entry
}

/// Whether `w` (picked under the pointer) is inside a button of the row (ring, chevron).
fn in_button(w: Option<gtk::Widget>, row: &gtk::Widget) -> bool {
    let mut cur = w;
    while let Some(c) = cur {
        if &c == row {
            return false;
        }
        if c.is::<gtk::Button>() || c.is::<gtk::CheckButton>() {
            return true;
        }
        cur = c.parent();
    }
    false
}

/// Open the task on a click outside the row's buttons.
fn open_on_click(root: &gtk::Box, id: &str, ctx: &RenderCtx) {
    let click = gtk::GestureClick::new();
    click.set_button(gtk::gdk::BUTTON_PRIMARY);
    let dispatch = ctx.dispatch.clone();
    let id_c = id.to_owned();
    click.connect_released(move |g, n, x, y| {
        let Some(w) = g.widget() else { return };
        if n != 1 || in_button(w.pick(x, y, gtk::PickFlags::DEFAULT), &w) {
            return;
        }
        dispatch(RowAction::Open(id_c.clone()));
    });
    root.add_controller(click);
}

/// Open on click, context menu on right-click and long-press (4.4).
fn row_gestures(root: &gtk::Box, id: &str, key: &str, ctx: &RenderCtx) {
    open_on_click(root, id, ctx);
    let menu_at = {
        let dispatch = ctx.dispatch.clone();
        let key = key.to_owned();
        move |w: &gtk::Widget, x: f64, y: f64| {
            let Some(list) = w.ancestor(gtk::ListView::static_type()) else {
                return;
            };
            if let Some(p) = w.compute_point(&list, &gtk::graphene::Point::new(x as f32, y as f32))
            {
                dispatch(RowAction::Menu {
                    key: key.clone(),
                    x: f64::from(p.x()),
                    y: f64::from(p.y()),
                });
            }
        }
    };
    let right = gtk::GestureClick::new();
    right.set_button(gtk::gdk::BUTTON_SECONDARY);
    let m = menu_at.clone();
    right.connect_pressed(move |g, _, x, y| {
        g.set_state(gtk::EventSequenceState::Claimed);
        if let Some(w) = g.widget() {
            m(&w, x, y);
        }
    });
    root.add_controller(right);
    let long = gtk::GestureLongPress::new();
    long.set_touch_only(true);
    long.connect_pressed(move |g, x, y| {
        if let Some(w) = g.widget() {
            menu_at(&w, x, y);
        }
    });
    root.add_controller(long);
}

pub type Dispatch = Rc<dyn Fn(RowAction)>;

/// Shared render options.
#[derive(Clone)]
pub struct RenderCtx {
    pub dispatch: Dispatch,
    /// `TickWindow:narrow` (B2 and B3): the view header's top margin is 24, not 48.
    pub narrow: Rc<Cell<bool>>,
    /// Set when the first task row is bound (first data frame, test mode).
    pub task_bound: Rc<Cell<bool>>,
    /// The view header's height, for the header bar title hand-off (4.2).
    pub header_height: Rc<Cell<i32>>,
    /// The search entry takes focus when it is next built (the Search view was opened).
    pub focus_search: Rc<Cell<bool>>,
}

/// Connect the factory to `list` and return the model and its selection.
pub fn setup(list: &gtk::ListView, ctx: RenderCtx) -> (gio::ListStore, gtk::SingleSelection) {
    let store = gio::ListStore::new::<TickRow>();
    let selection = gtk::SingleSelection::new(Some(store.clone()));
    selection.set_autoselect(false);
    selection.set_can_unselect(true);
    list.set_model(Some(&selection));

    let factory = gtk::SignalListItemFactory::new();
    let handlers: Rc<RefCell<HashMap<usize, (TickRow, glib::SignalHandlerId)>>> = Rc::default();
    factory.connect_setup(|_, item| {
        let Some(li) = item.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        li.set_child(Some(&adw::Bin::new()));
    });
    let bind_handlers = handlers.clone();
    factory.connect_bind(move |_, item| {
        let Some(li) = item.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let Some(obj) = li.item().and_downcast::<TickRow>() else {
            return;
        };
        render(li, &obj, &ctx);
        let weak = li.downgrade();
        let ctx = ctx.clone();
        let id = obj.connect_revision_notify(move |obj| {
            if let Some(li) = weak.upgrade() {
                render(&li, obj, &ctx);
            }
        });
        let key = li.as_ptr() as usize;
        if let Some((old, old_id)) = bind_handlers.borrow_mut().insert(key, (obj, id)) {
            old.disconnect(old_id);
        }
    });
    factory.connect_unbind(move |_, item| {
        let key = item.as_ptr() as usize;
        if let Some((obj, id)) = handlers.borrow_mut().remove(&key) {
            obj.disconnect(id);
        }
    });
    list.set_factory(Some(&factory));
    (store, selection)
}

fn render(li: &gtk::ListItem, obj: &TickRow, ctx: &RenderCtx) {
    let Some(bin) = li.child().and_downcast::<adw::Bin>() else {
        return;
    };
    let row = obj.row().clone();
    // The search entry is never rebuilt while it shows: typing would lose focus (4.15).
    if let Row::SearchHeader { count, .. } = &row
        && let Some(existing) = bin.child()
        && existing.widget_name() == obj.key()
    {
        update_search_count(&existing, *count);
        return;
    }
    let (selectable, interactive) = match &row {
        Row::Task(_) => (true, true),
        Row::SectionHeader(_) | Row::Ghost { .. } => (false, true),
        Row::SectionEditor { .. } => (false, false),
        _ => (false, false),
    };
    li.set_selectable(selectable);
    li.set_activatable(interactive);
    li.set_focusable(interactive);
    let (label, description) = accessible_text(&row);
    li.set_accessible_label(&label);
    li.set_accessible_description(&description);
    let widget = match &row {
        Row::ViewHeader(h) => view_header(h, ctx),
        Row::SectionHeader(s) => section_header(s, ctx),
        Row::GroupHeader { title, .. } => group_header(title),
        Row::DayHeader(d) => day_header(d),
        Row::MonthHeader { title, .. } => month_header(title),
        Row::Task(t) if t.read_only => {
            ctx.task_bound.set(true);
            let w = task_row(t, ctx);
            // Open and Reopen in its menu; nothing to drag (4.21).
            if let Some(b) = w.downcast_ref::<gtk::Box>() {
                row_gestures(b, &t.id, &t.key, ctx);
            }
            w
        }
        Row::Task(t) => {
            ctx.task_bound.set(true);
            let w = task_row(t, ctx);
            if let Some(b) = w.downcast_ref::<gtk::Box>() {
                row_gestures(b, &t.id, &t.key, ctx);
            }
            let d1 = ctx.dispatch.clone();
            let d2 = ctx.dispatch.clone();
            let shown = t.clone();
            let weak = w.downgrade();
            crate::dnd::drag_source(
                &w,
                format!("{}{}", crate::dnd::TASK, t.id),
                move || {
                    let width = weak.upgrade().map_or(-1, |w| w.width());
                    drag_icon(&shown, width)
                },
                move || d1(RowAction::DragBegin),
                move || d2(RowAction::DragEnd),
            );
            w
        }
        Row::Ghost { prefill, .. } => ghost_row(prefill, ctx),
        Row::Empty(e) => empty_row(e),
        Row::Loading { text, .. } => loading_row(text),
        Row::SourceLine { text, spinner, .. } => source_line(text, *spinner),
        Row::SectionEditor { id, name, .. } => section_editor(id.clone(), name, ctx),
        Row::SearchHeader { query, count } => search_header(query, *count, ctx),
        Row::LoadMore { loading, sensitive } => load_more_row(*loading, *sensitive, ctx),
    };
    // The key lets keyboard navigation map the focused row back to its model position.
    widget.set_widget_name(&obj.key());
    bin.set_child(Some(&widget));
}

fn accessible_text(row: &Row) -> (String, String) {
    match row {
        Row::Task(t) => (t.title.clone(), t.meta_description()),
        Row::SectionHeader(s) => {
            let state = if s.collapsed {
                gettext("collapsed")
            } else {
                gettext("expanded")
            };
            (
                fill(
                    &gettext("{name}, {count}, {state}"),
                    &[
                        ("name", &s.name),
                        ("count", &ncount("{n} task", "{n} tasks", s.count)),
                        ("state", &state),
                    ],
                ),
                String::new(),
            )
        }
        Row::Ghost { .. } => (gettext("Add task"), String::new()),
        Row::ViewHeader(h) => (h.title.clone(), String::new()),
        Row::GroupHeader { title, .. } | Row::MonthHeader { title, .. } => {
            (title.clone(), String::new())
        }
        Row::DayHeader(d) => (d.label.clone(), d.dim.clone().unwrap_or_default()),
        Row::Empty(e) => (e.title.clone(), e.description.clone()),
        Row::Loading { text, .. } | Row::SourceLine { text, .. } => (text.clone(), String::new()),
        Row::SectionEditor { .. } => (gettext("Section name"), String::new()),
        Row::SearchHeader { .. } => (gettext("Search"), String::new()),
        Row::LoadMore { .. } => (gettext("Load More"), String::new()),
    }
}

fn label(text: &str, classes: &[&str]) -> gtk::Label {
    gtk::Label::builder()
        .label(text)
        .xalign(0.0)
        .css_classes(classes.iter().map(|c| c.to_string()).collect::<Vec<_>>())
        .build()
}

fn icon(name: &str, classes: &[&str]) -> gtk::Image {
    gtk::Image::builder()
        .icon_name(name)
        .accessible_role(gtk::AccessibleRole::Presentation)
        .css_classes(classes.iter().map(|c| c.to_string()).collect::<Vec<_>>())
        .build()
}

fn dot(color: &str, size_class: &str) -> gtk::Widget {
    gtk::Box::builder()
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Center)
        .accessible_role(gtk::AccessibleRole::Presentation)
        .css_classes([size_class.to_owned(), color_class(color)])
        .build()
        .upcast()
}

fn view_header(h: &ViewHeader, ctx: &RenderCtx) -> gtk::Widget {
    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .margin_top(if ctx.narrow.get() { 24 } else { 48 })
        .margin_start(12)
        .css_classes(["tick-view-header"])
        .build();
    let title_box = gtk::Box::new(gtk::Orientation::Horizontal, 9);
    if let Some(c) = &h.color {
        title_box.append(&dot(c, "tick-dot-12"));
    }
    if let Some(i) = h.icon {
        let img = icon(i, &[]);
        img.set_pixel_size(20);
        title_box.append(&img);
    }
    if h.editing {
        let d1 = ctx.dispatch.clone();
        let d2 = ctx.dispatch.clone();
        let entry = inline_entry(
            &h.title,
            &gettext("Project name"),
            &["title-1", "tick-title-entry"],
            true,
            move |name| d1(RowAction::TitleCommit(name)),
            move || d2(RowAction::TitleCancel),
        );
        title_box.append(&entry);
    } else {
        let title = label(&h.title, &["title-1"]);
        title.set_wrap(true);
        title.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        if h.renamable {
            // Double-click renames the project in place (4.2).
            let click = gtk::GestureClick::new();
            let dispatch = ctx.dispatch.clone();
            click.connect_pressed(move |_, n, _, _| {
                if n == 2 {
                    dispatch(RowAction::RenameTitle);
                }
            });
            title.add_controller(click);
        }
        title_box.append(&title);
    }
    root.append(&title_box);

    let sub = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .margin_top(3)
        .build();
    if let Some(s) = &h.subtitle {
        sub.append(&label(s, &["body", "dimmed"]));
    }
    if let Some(q) = &h.query {
        let ql = label(q, &["body", "monospace", "dimmed"]);
        ql.set_ellipsize(gtk::pango::EllipsizeMode::End);
        ql.set_selectable(false);
        sub.append(&ql);
    }
    // A zero count repeats the empty state below it (4.2).
    if h.count > 0 {
        let count = label(
            &ncount("{n} task", "{n} tasks", h.count),
            &["body", "dimmed", "numeric"],
        );
        // After a monospace query, 12 px reads as part of the query: 24 (space-5) parts them.
        if h.query.is_some() {
            count.set_margin_start(12);
        }
        sub.append(&count);
    }
    if !h.avatars.is_empty() {
        let avatars = gtk::Box::new(gtk::Orientation::Horizontal, 3);
        for name in &h.avatars {
            let a = adw::Avatar::new(20, Some(name), true);
            a.upcast_ref::<gtk::Widget>()
                .update_property(&[gtk::accessible::Property::Label(name)]);
            avatars.append(&a);
        }
        sub.append(&avatars);
    }
    if sub.first_child().is_some() {
        root.append(&sub);
    }
    if let Some((text, spinner)) = &h.source {
        // Where server filter results come from: part of the header, 6 px under the
        // subtitle (4.21, 4.22).
        let line = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .margin_top(6)
            .build();
        if *spinner {
            let s = adw::Spinner::new();
            s.set_size_request(16, 16);
            line.append(&s);
        }
        line.append(&label(text, &["caption", "dimmed"]));
        root.append(&line);
    }
    let height = ctx.header_height.clone();
    root.connect_map(move |w| {
        let w = w.clone();
        let height = height.clone();
        glib::idle_add_local_once(move || height.set(w.height() + 48 + 24));
    });
    root.upcast()
}

fn hairline(margin_start: i32) -> gtk::Widget {
    gtk::Box::builder()
        .margin_start(margin_start)
        .margin_top(6)
        .css_classes(["tick-section-rule"])
        .accessible_role(gtk::AccessibleRole::Presentation)
        .build()
        .upcast()
}

fn section_header(s: &SectionHeader, ctx: &RenderCtx) -> gtk::Widget {
    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .css_classes(["tick-section-header"])
        .build();
    let line = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let chevron = gtk::Button::builder()
        .icon_name("pan-down-symbolic")
        .valign(gtk::Align::Center)
        .css_classes(["flat", "tick-icon-44", "tick-chevron", "dimmed"])
        .build();
    if s.collapsed {
        chevron.add_css_class("collapsed");
    }
    let name = if s.collapsed {
        fill(&gettext("Expand {name}"), &[("name", &s.name)])
    } else {
        fill(&gettext("Collapse {name}"), &[("name", &s.name)])
    };
    chevron.update_property(&[gtk::accessible::Property::Label(&name)]);
    let dispatch = ctx.dispatch.clone();
    let id = s.id.clone();
    chevron.connect_clicked(move |_| dispatch(RowAction::ToggleSection(id.clone())));
    line.append(&chevron);
    // The hairline hangs 6 px under the name's ink (2 px under its label box, which
    // includes the descender space; 4.5), inside the 44 px header line: the name column
    // is centered as if the rule (3 px with its gap) were not there.
    let column = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .valign(gtk::Align::Center)
        .margin_top(3)
        .hexpand(true)
        .build();
    let name_row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let title = label(&s.name, &["heading"]);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    name_row.append(&title);
    let count = label(&s.count.to_string(), &["caption", "dimmed", "numeric"]);
    count.set_margin_start(9);
    name_row.append(&count);
    column.append(&name_row);
    let rule = hairline(0);
    rule.set_margin_top(2);
    column.append(&rule);
    line.append(&column);
    // Rename, Add Task Here, Delete Section (4.5): shown on hover and focus, always in AT.
    let menu = gio::Menu::new();
    let target = Some(s.id.to_variant());
    let item = |label: &str, action: &str| {
        let i = gio::MenuItem::new(Some(label), None);
        i.set_action_and_target_value(Some(action), target.as_ref());
        i
    };
    menu.append_item(&item(&gettext("Rename"), "win.section-rename"));
    menu.append_item(&item(&gettext("Add Task Here"), "win.section-add-task"));
    let danger = gio::Menu::new();
    danger.append_item(&item(&gettext("Delete Section..."), "win.section-delete"));
    menu.append_section(None, &danger);
    let button = gtk::MenuButton::builder()
        .icon_name("view-more-symbolic")
        .menu_model(&menu)
        .valign(gtk::Align::Center)
        .css_classes(["flat", "tick-icon-44", "tick-reveal"])
        .tooltip_text(gettext("Section Menu"))
        .build();
    button.update_property(&[gtk::accessible::Property::Label(&fill(
        &gettext("{name} section menu"),
        &[("name", &s.name)],
    ))]);
    crate::menus::name_button(&button);
    line.append(&button);
    root.append(&line);
    root.upcast()
}

/// The inline section editor (4.24): the section header layout with an entry for the name.
fn section_editor(id: Option<String>, name: &str, ctx: &RenderCtx) -> gtk::Widget {
    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .css_classes(["tick-section-header"])
        .build();
    if id.is_none() {
        // A new section is the project's last row: room under it, so scrolling it into view
        // does not leave it on the window's bottom edge (4.24).
        root.set_margin_bottom(48);
    }
    let line = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    // The chevron column, as in a header (a new section has nothing to collapse yet).
    let chevron = gtk::Image::builder()
        .icon_name("pan-down-symbolic")
        .width_request(44)
        .height_request(44)
        .css_classes(["dimmed"])
        .accessible_role(gtk::AccessibleRole::Presentation)
        .build();
    line.append(&chevron);
    // The name column ends where a header's menu button starts, so the hairline does too.
    // The hairline hangs 2 px lower than a header's, 2 px under the focus ring (which pads
    // the text by 6 px, style.css); the top margin balances the box so the name stays on the
    // header's line.
    let column = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .valign(gtk::Align::Center)
        .margin_top(9)
        .margin_end(44)
        .hexpand(true)
        .build();
    let d1 = ctx.dispatch.clone();
    let d2 = ctx.dispatch.clone();
    let rename = id.is_some();
    let entry = inline_entry(
        name,
        &gettext("Section name"),
        &["heading", "tick-entry", "tick-section-entry"],
        rename,
        move |name| {
            d1(RowAction::SectionCommit {
                id: id.clone(),
                name,
            })
        },
        move || d2(RowAction::SectionCancel),
    );
    column.append(&entry);
    let rule = hairline(0);
    rule.set_margin_top(8);
    column.append(&rule);
    line.append(&column);
    root.append(&line);
    root.upcast()
}

fn group_header(title: &str) -> gtk::Widget {
    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .css_classes(["tick-section-header"])
        .build();
    let l = label(title, &["heading", "monospace"]);
    l.set_margin_start(12);
    l.set_valign(gtk::Align::End);
    l.set_vexpand(true);
    l.set_ellipsize(gtk::pango::EllipsizeMode::End);
    root.append(&l);
    root.append(&hairline(12));
    root.upcast()
}

fn day_header(d: &DayHeader) -> gtk::Widget {
    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .css_classes(["tick-day-header"])
        .build();
    if d.first {
        root.add_css_class("first");
    }
    if d.after_month {
        root.add_css_class("after-month");
    }
    let line = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .vexpand(true)
        .valign(gtk::Align::End)
        .build();
    match d.number {
        Some(n) => {
            let num = label(&n.to_string(), &["title-2", "numeric", "tick-day-number"]);
            num.set_xalign(1.0);
            line.append(&num);
        }
        None => {
            let i = icon("tick-overdue-symbolic", &["tick-overdue-icon"]);
            i.set_pixel_size(20);
            i.set_halign(gtk::Align::End);
            i.set_valign(gtk::Align::Center);
            let holder = adw::Bin::builder()
                .css_classes(["tick-day-number"])
                .child(&i)
                .build();
            line.append(&holder);
        }
    }
    let text = gtk::Box::new(gtk::Orientation::Horizontal, 9);
    text.set_valign(gtk::Align::Baseline);
    text.append(&label(&d.label, &["heading"]));
    if let Some(dim) = &d.dim {
        let classes: &[&str] = if d.number.is_none() {
            &["body", "dimmed", "numeric"]
        } else {
            &["body", "dimmed"]
        };
        text.append(&label(dim, classes));
    }
    line.append(&text);
    root.append(&line);
    // From the day number (right-aligned in its 32 px column), not from the row edge.
    root.append(&hairline(6));
    root.upcast()
}

fn month_header(title: &str) -> gtk::Widget {
    let l = label(title, &["title-4", "tick-month-header"]);
    l.set_margin_start(12);
    l.set_margin_bottom(6);
    l.upcast()
}

fn meta_item(icon_name: Option<&str>, text: &str, classes: &[&str]) -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Horizontal, 3);
    if let Some(i) = icon_name {
        let img = icon(i, classes);
        img.set_valign(gtk::Align::Center);
        b.append(&img);
    }
    b.append(&label(text, classes));
    b
}

fn due_classes(d: &MetaDue) -> &'static [&'static str] {
    match d.state {
        DueState::Overdue => &["tick-due", "overdue"],
        DueState::Today => &["tick-due", "today"],
        DueState::Other => &["tick-due", "dimmed"],
    }
}

fn task_row(t: &TaskRow, ctx: &RenderCtx) -> gtk::Widget {
    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .margin_start(32 * t.depth as i32)
        .build();

    let ring = gtk::CheckButton::builder()
        .valign(gtk::Align::Start)
        .css_classes(["tick-ring".to_owned(), format!("p{}", t.p)])
        .active(t.checked)
        .build();
    let ring_name = if t.read_only {
        // The history is read-only (4.3 "disabled"): reopen from the detail pane or menu.
        ring.set_sensitive(false);
        ring.add_css_class("tick-done");
        gettext("Completed")
    } else if t.checked {
        gettext("Mark as not done")
    } else {
        gettext("Complete task")
    };
    ring.update_property(&[
        gtk::accessible::Property::Label(&ring_name),
        gtk::accessible::Property::Description(&t.title),
    ]);
    let dispatch = ctx.dispatch.clone();
    let id = t.id.clone();
    ring.connect_toggled(move |b| {
        dispatch(RowAction::Ring {
            id: id.clone(),
            active: b.is_active(),
        })
    });
    root.append(&ring);

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .hexpand(true)
        .margin_top(12)
        .margin_bottom(12)
        .build();
    let title = label(&t.title, &["body"]);
    title.set_wrap(true);
    title.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    title.set_lines(3);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    if t.completing {
        title.add_css_class("tick-struck");
        title.add_css_class("dimmed");
    }
    content.append(&title);
    if let Some(d) = &t.description {
        let desc = label(d, &["caption", "dimmed"]);
        desc.set_single_line_mode(true);
        desc.set_ellipsize(gtk::pango::EllipsizeMode::End);
        desc.set_margin_top(2);
        content.append(&desc);
    }
    if let Some(meta) = meta_line(t) {
        content.append(&meta);
    }
    root.append(&content);
    let trailing = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(6)
        .valign(gtk::Align::Center)
        .margin_start(12)
        .build();
    if let Some(msg) = &t.failed {
        let warn = icon("dialog-warning-symbolic", &["tick-failed"]);
        warn.set_tooltip_text(Some(msg));
        trailing.append(&warn);
    }
    if let Some(name) = &t.assignee {
        let a = adw::Avatar::new(20, Some(name), true);
        a.upcast_ref::<gtk::Widget>()
            .update_property(&[gtk::accessible::Property::Label(&fill(
                &gettext("Assigned to {name}"),
                &[("name", name)],
            ))]);
        trailing.append(&a);
    }
    if t.has_children {
        let chevron = gtk::Button::builder()
            .icon_name("pan-down-symbolic")
            .css_classes(["flat", "tick-icon-44", "tick-chevron", "dimmed"])
            .build();
        let name = if t.collapsed {
            chevron.add_css_class("collapsed");
            gettext("Expand sub-tasks")
        } else {
            gettext("Collapse sub-tasks")
        };
        chevron.update_property(&[gtk::accessible::Property::Label(&name)]);
        let dispatch = ctx.dispatch.clone();
        let id = t.id.clone();
        chevron.connect_clicked(move |_| dispatch(RowAction::ToggleSubtasks(id.clone())));
        trailing.append(&chevron);
    } else if t.chevron_slot {
        // Keep the chevron column so avatars line up with the parents' (4.4).
        let slot = gtk::Box::builder()
            .width_request(44)
            .accessible_role(gtk::AccessibleRole::Presentation)
            .build();
        trailing.append(&slot);
    }
    if trailing.first_child().is_some() {
        root.append(&trailing);
    }

    if t.fading {
        // libadwaita skips an animation whose widget is not mapped (it jumps to the end):
        // start the fade once the row is on screen. Test mode uses no duration (5.3).
        let duration = if crate::test_mode::enabled() { 0 } else { 250 };
        let target = adw::PropertyAnimationTarget::new(&root, "opacity");
        let anim = adw::TimedAnimation::new(&root, 1.0, 0.0, duration, target);
        anim.set_easing(adw::Easing::EaseOutCubic);
        anim.set_follow_enable_animations_setting(false);
        if root.is_mapped() {
            anim.play();
        } else {
            let pending = std::cell::RefCell::new(Some(anim));
            root.connect_map(move |_| {
                if let Some(a) = pending.borrow_mut().take() {
                    a.play();
                }
            });
        }
    }
    root.upcast()
}

/// A task's meta line (4.4): due, deadline, sub-tasks, comments, labels, project, each with
/// its 12 px icon; `.dimmed` on individual items only (7.4 pitfall 1). `None` when empty.
/// Shared by the task list and the detail pane's sub-task rows (4.8).
pub fn meta_line(t: &TaskRow) -> Option<adw::WrapBox> {
    let meta = adw::WrapBox::builder()
        .child_spacing(12)
        .line_spacing(2)
        .margin_top(2)
        .css_classes(["caption", "tick-meta"])
        .build();
    let mut has_meta = false;
    if let Some(at) = &t.done_at {
        meta.append(&label(at, &["dimmed", "numeric"]));
        has_meta = true;
    }
    if t.completed_pending {
        let b = gtk::Box::new(gtk::Orientation::Horizontal, 3);
        let spinner = adw::Spinner::new();
        spinner.set_size_request(12, 12);
        b.append(&spinner);
        b.append(&label(&gettext("Completing"), &["dimmed"]));
        meta.append(&b);
        has_meta = true;
    }
    if let Some(d) = &t.due {
        let icon_name = if d.recurring {
            "media-playlist-repeat-symbolic"
        } else {
            "tick-due-symbolic"
        };
        meta.append(&meta_item(Some(icon_name), &d.text, due_classes(d)));
        has_meta = true;
    }
    if let Some((text, overdue)) = &t.deadline {
        let classes: &[&str] = if *overdue {
            &["tick-deadline", "overdue"]
        } else {
            &["tick-deadline", "dimmed"]
        };
        meta.append(&meta_item(Some("tick-deadline-symbolic"), text, classes));
        has_meta = true;
    }
    if let Some((done, total)) = t.subtasks {
        meta.append(&meta_item(
            Some("tick-subtasks-symbolic"),
            &format!("{done}/{total}"),
            &["dimmed", "numeric"],
        ));
        has_meta = true;
    }
    if t.comments > 0 {
        meta.append(&meta_item(
            Some("tick-comment-symbolic"),
            &t.comments.to_string(),
            &["dimmed", "numeric"],
        ));
        has_meta = true;
    }
    if !t.labels.is_empty() {
        let mut text = t
            .labels
            .iter()
            .take(2)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        if t.labels.len() > 2 {
            text.push_str(&format!(" +{}", t.labels.len() - 2));
        }
        meta.append(&meta_item(Some("tick-label-symbolic"), &text, &["dimmed"]));
        has_meta = true;
    }
    if let Some(p) = &t.project {
        let b = gtk::Box::new(gtk::Orientation::Horizontal, 3);
        b.append(&dot(&p.color, "tick-dot-8"));
        b.append(&label(&p.name, &["dimmed"]));
        meta.append(&b);
        has_meta = true;
    }
    has_meta.then_some(meta)
}

/// The drag icon (4.4): the row as it is drawn in the list (ring, title, meta line and
/// trailing avatar) on the floating surface, at the row's width. Nothing in it reacts.
fn drag_icon(t: &TaskRow, width: i32) -> gtk::Widget {
    let shown = TaskRow {
        depth: 0,
        fading: false,
        completing: false,
        has_children: false,
        chevron_slot: false,
        ..t.clone()
    };
    let ctx = RenderCtx {
        dispatch: Rc::new(|_| {}),
        narrow: Rc::default(),
        task_bound: Rc::default(),
        header_height: Rc::default(),
        focus_search: Rc::default(),
    };
    let row = task_row(&shown, &ctx);
    row.set_can_target(false);
    row.set_hexpand(true);
    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .width_request(width)
        .css_classes(["tick-drag-icon"])
        .build();
    root.append(&row);
    root.upcast()
}

fn ghost_row(prefill: &str, ctx: &RenderCtx) -> gtk::Widget {
    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .height_request(44)
        .build();
    let plus = icon("list-add-symbolic", &["dimmed"]);
    plus.set_size_request(44, 44);
    root.append(&plus);
    root.append(&label(&gettext("Add task"), &["dimmed"]));
    let click = gtk::GestureClick::new();
    let dispatch = ctx.dispatch.clone();
    let prefill = prefill.to_owned();
    click.connect_released(move |g, _, _, _| {
        g.set_state(gtk::EventSequenceState::Claimed);
        dispatch(RowAction::Ghost(prefill.clone()));
    });
    root.add_controller(click);
    root.upcast()
}

/// The empty state (4.13): the layout of a compact `AdwStatusPage` (64 px dimmed icon,
/// `.title-2` title, `.body .dimmed` description). `AdwStatusPage` itself wraps its content
/// in a scrolled window, which a list row clips to its minimum height and exposes as an
/// unnamed focusable scroll pane.
fn empty_row(e: &EmptyState) -> gtk::Widget {
    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .halign(gtk::Align::Center)
        .css_classes(["tick-empty-row"])
        .build();
    let image = icon(e.icon, &["dimmed"]);
    image.set_pixel_size(64);
    image.set_margin_bottom(12);
    root.append(&image);
    let title = label(&e.title, &["title-2"]);
    title.set_wrap(true);
    title.set_justify(gtk::Justification::Center);
    title.set_xalign(0.5);
    root.append(&title);
    let description = label(&e.description, &["body", "dimmed"]);
    description.set_wrap(true);
    description.set_justify(gtk::Justification::Center);
    description.set_xalign(0.5);
    root.append(&description);
    if let Some(id) = &e.edit_filter {
        // The rejected server filter (4.22): fix the query in the Edit Filter dialog.
        let button = gtk::Button::builder()
            .label(gettext("Edit Filter"))
            .halign(gtk::Align::Center)
            .margin_top(12)
            .css_classes(["pill"])
            .action_name("win.filter-edit")
            .action_target(&id.to_variant())
            .build();
        root.append(&button);
    }
    root.upcast()
}

fn loading_row(text: &str) -> gtk::Widget {
    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_top(36)
        .halign(gtk::Align::Center)
        .build();
    let spinner = adw::Spinner::new();
    spinner.set_size_request(32, 32);
    root.append(&spinner);
    root.append(&label(text, &["dimmed"]));
    root.upcast()
}

fn source_line(text: &str, spinner: bool) -> gtk::Widget {
    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(6)
        .margin_start(12)
        .margin_bottom(6)
        .build();
    if spinner {
        let s = adw::Spinner::new();
        s.set_size_request(16, 16);
        root.append(&s);
    }
    root.append(&label(text, &["caption", "dimmed"]));
    root.upcast()
}

/// The Search view's header (4.15): a 44 px search entry in the place of the large title,
/// with the result count under it as the view header's subtitle (4.2).
fn search_header(query: &str, count: usize, ctx: &RenderCtx) -> gtk::Widget {
    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .margin_top(if ctx.narrow.get() { 24 } else { 48 })
        .margin_start(12)
        .margin_end(12)
        .css_classes(["tick-view-header", "tick-search-header"])
        .build();
    let entry = gtk::SearchEntry::builder()
        .placeholder_text(gettext("Search tasks"))
        .text(query)
        .hexpand(true)
        .css_classes(["tick-entry", "title-4", "tick-search-entry"])
        .build();
    entry.update_property(&[gtk::accessible::Property::Label(&gettext("Search tasks"))]);
    let dispatch = ctx.dispatch.clone();
    entry.connect_search_changed(move |e| dispatch(RowAction::SearchChanged(e.text().into())));
    let dispatch = ctx.dispatch.clone();
    entry.connect_activate(move |e| dispatch(RowAction::SearchActivate(e.text().into())));
    let dispatch = ctx.dispatch.clone();
    // Escape (stop-search) clears the entry, or leaves Search when it is empty.
    entry.connect_stop_search(move |e| {
        if e.text().is_empty() {
            dispatch(RowAction::SearchEscape);
        } else {
            e.set_text("");
        }
    });
    let keys = gtk::EventControllerKey::new();
    let dispatch = ctx.dispatch.clone();
    keys.connect_key_pressed(move |_, key, _, mods| {
        if mods.is_empty() && matches!(key, gtk::gdk::Key::Down | gtk::gdk::Key::KP_Down) {
            dispatch(RowAction::SearchDown);
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    entry.add_controller(keys);
    let focus = ctx.focus_search.clone();
    entry.connect_map(move |e| {
        if focus.replace(false) {
            let e = e.clone();
            glib::idle_add_local_once(move || {
                e.grab_focus();
                e.set_position(-1);
            });
        }
    });
    root.append(&entry);
    let count_label = label("", &["body", "dimmed", "numeric"]);
    count_label.set_widget_name("search-count");
    count_label.set_margin_top(6);
    root.append(&count_label);
    update_search_count(root.upcast_ref(), count);
    let height = ctx.header_height.clone();
    root.connect_map(move |w| {
        let w = w.clone();
        let height = height.clone();
        glib::idle_add_local_once(move || height.set(w.height() + 48 + 24));
    });
    root.upcast()
}

/// Show `count` results under the search entry (hidden when 0: the empty state says it).
fn update_search_count(header: &gtk::Widget, count: usize) {
    let Some(label) = header.last_child().and_downcast::<gtk::Label>() else {
        return;
    };
    label.set_label(&ncount("{n} task", "{n} tasks", count));
    label.set_visible(count > 0);
}

/// "Load More" (4.21): a full-width flat button, 44 px tall; a spinner and "Loading" while
/// the page loads.
fn load_more_row(loading: bool, sensitive: bool, ctx: &RenderCtx) -> gtk::Widget {
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(9)
        .halign(gtk::Align::Center)
        .build();
    if loading {
        let spinner = adw::Spinner::new();
        spinner.set_size_request(16, 16);
        content.append(&spinner);
        content.append(&label(&gettext("Loading"), &["dimmed"]));
    } else {
        content.append(&label(&gettext("Load More"), &[]));
    }
    let button = gtk::Button::builder()
        .child(&content)
        .hexpand(true)
        .sensitive(sensitive)
        .margin_top(12)
        .margin_bottom(24)
        .css_classes(["flat", "tick-load-more"])
        .build();
    let name = if loading {
        gettext("Loading")
    } else {
        gettext("Load More")
    };
    button.update_property(&[gtk::accessible::Property::Label(&name)]);
    let dispatch = ctx.dispatch.clone();
    button.connect_clicked(move |_| dispatch(RowAction::LoadMore));
    button.upcast()
}
