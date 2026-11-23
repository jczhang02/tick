//! The sidebar (`DESIGN.md` 4.1): a `GtkListView` with `.navigation-sidebar` over a
//! `GtkTreeListModel` of system views, favorites, the project tree, labels and filters.
//!
//! While a task or project is dragged over a collapsed project, the project expands after
//! 600 ms ([`Sidebar::hover_drag`]).

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};

use crate::i18n::{fill, gettext, ncount};
use crate::rows::{SidebarData, SidebarEntry, SidebarProject, ViewId, color_class};

/// Collapsible sidebar sections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Section {
    Favorites,
    Projects,
    Labels,
    Filters,
}

impl Section {
    fn title(self) -> String {
        match self {
            Section::Favorites => gettext("Favorites"),
            Section::Projects => gettext("Projects"),
            Section::Labels => gettext("Labels"),
            Section::Filters => gettext("Filters"),
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Section::Favorites => "favorites",
            Section::Projects => "projects",
            Section::Labels => "labels",
            Section::Filters => "filters",
        }
    }

    pub fn from_key(k: &str) -> Option<Section> {
        Some(match k {
            "favorites" => Section::Favorites,
            "projects" => Section::Projects,
            "labels" => Section::Labels,
            "filters" => Section::Filters,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ItemKind {
    /// `empty`: the section has no entries yet; its "Add ..." button stays visible (4.1).
    Header {
        section: Section,
        collapsed: bool,
        empty: bool,
    },
    View(ViewId),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemData {
    pub kind: ItemKind,
    pub name: String,
    pub icon: Option<&'static str>,
    pub color: Option<String>,
    pub count: Option<usize>,
    pub shared: bool,
}

mod imp {
    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::SidebarItem)]
    pub struct SidebarItem {
        pub data: RefCell<Option<ItemData>>,
        pub children: RefCell<Option<gio::ListStore>>,
        #[property(get)]
        pub revision: Cell<u64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SidebarItem {
        const NAME: &'static str = "TickSidebarItem";
        type Type = super::SidebarItem;
    }

    #[glib::derived_properties]
    impl ObjectImpl for SidebarItem {}
}

glib::wrapper! {
    pub struct SidebarItem(ObjectSubclass<imp::SidebarItem>);
}

impl SidebarItem {
    fn new(data: ItemData, children: Option<gio::ListStore>) -> SidebarItem {
        let obj: SidebarItem = glib::Object::new();
        obj.imp().data.replace(Some(data));
        obj.imp().children.replace(children);
        obj
    }

    pub fn data(&self) -> ItemData {
        self.imp().data.borrow().clone().expect("sidebar item data")
    }

    fn set_data(&self, data: ItemData) {
        if self.imp().data.borrow().as_ref() == Some(&data) {
            return;
        }
        self.imp().data.replace(Some(data));
        let imp = self.imp();
        imp.revision.set(imp.revision.get() + 1);
        self.notify_revision();
    }

    fn children(&self) -> Option<gio::ListStore> {
        self.imp().children.borrow().clone()
    }

    pub fn view(&self) -> Option<ViewId> {
        match self.data().kind {
            ItemKind::View(v) => Some(v),
            ItemKind::Header { .. } => None,
        }
    }
}

/// Keyboard moves of a project (4.1): the keyboard path of dragging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectMove {
    Up,
    Down,
    /// Make it a sub-project of the sibling above.
    In,
    /// One level out.
    Out,
}

/// What the sidebar asks the window to do.
pub enum SidebarEvent {
    /// Open a view; `push` also shows the content page on narrow windows.
    Open {
        view: ViewId,
        push: bool,
    },
    /// A section header was collapsed or expanded (persist it).
    SectionToggled,
    /// "Add Project", "Add Label", "Add Filter".
    Add(Section),
    /// The context menu of a row (a view) or a section header, at `rect` in the list's
    /// coordinates.
    Menu {
        kind: ItemKind,
        rect: gtk::gdk::Rectangle,
    },
    /// Alt+Shift+arrows on a project row.
    MoveProject {
        id: String,
        how: ProjectMove,
    },
    /// A project drag started or ended.
    DragBegin,
    DragEnd,
}

pub struct Sidebar {
    list: gtk::ListView,
    root: gio::ListStore,
    tree: gtk::TreeListModel,
    selection: gtk::SingleSelection,
    pub collapsed: RefCell<HashSet<Section>>,
    data: RefCell<Option<SidebarData>>,
    events: Rc<dyn Fn(SidebarEvent)>,
    suppress: Cell<bool>,
    current: RefCell<Option<ViewId>>,
}

/// One flattened entry used to detect structural changes (everything but counts).
fn shape(d: &SidebarData) -> Vec<String> {
    fn projects(list: &[SidebarProject], out: &mut Vec<String>) {
        for p in list {
            out.push(format!(
                "p:{}:{}:{}:{}:{}",
                p.id, p.name, p.color, p.depth, p.shared
            ));
            projects(&p.children, out);
            out.push("/".into());
        }
    }
    let mut out = Vec::new();
    for f in &d.favorites {
        out.push(format!("f:{:?}:{}:{:?}", f.view, f.name, f.color));
    }
    projects(&d.projects, &mut out);
    for l in &d.labels {
        out.push(format!("l:{:?}:{}", l.view, l.name));
    }
    for f in &d.filters {
        out.push(format!("q:{:?}:{}", f.view, f.name));
    }
    out
}

fn count(n: usize) -> Option<usize> {
    Some(n)
}

impl Sidebar {
    pub fn new(list: gtk::ListView, events: Rc<dyn Fn(SidebarEvent)>) -> Rc<Sidebar> {
        let root = gio::ListStore::new::<SidebarItem>();
        let tree = gtk::TreeListModel::new(root.clone(), false, false, |item| {
            item.downcast_ref::<SidebarItem>()
                .and_then(SidebarItem::children)
                .map(|s| s.upcast())
        });
        let selection = gtk::SingleSelection::new(Some(tree.clone()));
        selection.set_autoselect(false);
        selection.set_can_unselect(true);
        list.set_model(Some(&selection));
        let sidebar = Rc::new(Sidebar {
            list,
            root,
            tree,
            selection,
            collapsed: RefCell::default(),
            data: RefCell::default(),
            events,
            suppress: Cell::new(false),
            current: RefCell::default(),
        });
        sidebar.setup_factory();
        let weak = Rc::downgrade(&sidebar);
        sidebar
            .selection
            .connect_selection_changed(move |sel, _, _| {
                let Some(s) = weak.upgrade() else { return };
                if s.suppress.get() {
                    return;
                }
                let view = sel
                    .selected_item()
                    .and_downcast::<gtk::TreeListRow>()
                    .and_then(|r| r.item())
                    .and_downcast::<SidebarItem>()
                    .and_then(|i| i.view());
                if let Some(view) = view {
                    (s.events)(SidebarEvent::Open { view, push: false });
                }
            });
        let weak = Rc::downgrade(&sidebar);
        sidebar.list.connect_activate(move |_, pos| {
            let Some(s) = weak.upgrade() else { return };
            s.activate(pos);
        });
        // Menu key / Shift+F10: the focused row's context menu; Alt+Shift+arrows move the
        // focused project (4.1).
        let keys = gtk::EventControllerKey::new();
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = Rc::downgrade(&sidebar);
        keys.connect_key_pressed(move |_, key, _, mods| {
            use gtk::gdk::{Key, ModifierType};
            let Some(s) = weak.upgrade() else {
                return glib::Propagation::Proceed;
            };
            let Some((item, widget)) = s.focused_item() else {
                return glib::Propagation::Proceed;
            };
            let shift = mods.contains(ModifierType::SHIFT_MASK);
            let alt = mods.contains(ModifierType::ALT_MASK);
            if key == Key::Menu || (key == Key::F10 && shift && !alt) {
                let rect = widget
                    .compute_bounds(&s.list)
                    .map(|b| {
                        gtk::gdk::Rectangle::new(
                            b.x() as i32 + 12,
                            b.y() as i32,
                            (b.width() as i32 - 24).max(1),
                            b.height() as i32,
                        )
                    })
                    .unwrap_or_else(|| gtk::gdk::Rectangle::new(0, 0, 1, 1));
                (s.events)(SidebarEvent::Menu {
                    kind: item.data().kind,
                    rect,
                });
                return glib::Propagation::Stop;
            }
            if shift
                && alt
                && let Some(ViewId::Project(id)) = item.view()
            {
                let how = match key {
                    Key::Up | Key::KP_Up => ProjectMove::Up,
                    Key::Down | Key::KP_Down => ProjectMove::Down,
                    Key::Right | Key::KP_Right => ProjectMove::In,
                    Key::Left | Key::KP_Left => ProjectMove::Out,
                    _ => return glib::Propagation::Proceed,
                };
                (s.events)(SidebarEvent::MoveProject { id, how });
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        sidebar.list.add_controller(keys);
        sidebar
    }

    /// The item of the focused row and the row widget.
    fn focused_item(&self) -> Option<(SidebarItem, gtk::Widget)> {
        let root = self.list.root()?;
        let mut w = gtk::prelude::RootExt::focus(&root)?;
        let list: gtk::Widget = self.list.clone().upcast();
        while w.parent().as_ref() != Some(&list) {
            w = w.parent()?;
        }
        let item = w
            .first_child()
            .and_downcast::<gtk::TreeExpander>()?
            .list_row()?
            .item()
            .and_downcast::<SidebarItem>()?;
        Some((item, w))
    }

    /// Give keyboard focus to `view`'s row (after a keyboard move rebuilt the tree).
    pub fn focus_view(&self, view: &ViewId) {
        let pos = (0..self.tree.n_items())
            .find(|i| self.item_at(*i).and_then(|it| it.view()).as_ref() == Some(view));
        if let Some(p) = pos {
            self.list.scroll_to(p, gtk::ListScrollFlags::FOCUS, None);
            self.list.grab_focus();
        }
    }

    /// The rectangle of `view`'s row in the list's coordinates (a picker points at it).
    pub fn row_rect(&self, view: &ViewId) -> Option<gtk::gdk::Rectangle> {
        let mut child = self.list.first_child();
        while let Some(c) = child {
            let item = c
                .first_child()
                .and_downcast::<gtk::TreeExpander>()
                .and_then(|e| e.list_row())
                .and_then(|r| r.item())
                .and_downcast::<SidebarItem>();
            if item.and_then(|i| i.view()).as_ref() == Some(view)
                && let Some(b) = c.compute_bounds(&self.list)
            {
                return Some(gtk::gdk::Rectangle::new(
                    b.x() as i32 + 12,
                    b.y() as i32,
                    (b.width() as i32 - 24).max(1),
                    b.height() as i32,
                ));
            }
            child = c.next_sibling();
        }
        None
    }

    /// Show `TickWindow:narrow` on rows: the "Add ..." buttons stay visible (4.1).
    pub fn set_narrow(&self, narrow: bool) {
        if narrow {
            self.list.add_css_class("tick-narrow");
        } else {
            self.list.remove_css_class("tick-narrow");
        }
    }

    fn item_at(&self, pos: u32) -> Option<SidebarItem> {
        self.tree
            .item(pos)
            .and_downcast::<gtk::TreeListRow>()
            .and_then(|r| r.item())
            .and_downcast::<SidebarItem>()
    }

    fn activate(self: &Rc<Self>, pos: u32) {
        let Some(item) = self.item_at(pos) else {
            return;
        };
        match item.data().kind {
            ItemKind::Header { section, .. } => self.toggle(section),
            ItemKind::View(view) => (self.events)(SidebarEvent::Open { view, push: true }),
        }
    }

    pub fn toggle(self: &Rc<Self>, section: Section) {
        {
            let mut c = self.collapsed.borrow_mut();
            if !c.remove(&section) {
                c.insert(section);
            }
        }
        let data = self.data.borrow().clone();
        if let Some(d) = data {
            self.rebuild(&d);
        }
        (self.events)(SidebarEvent::SectionToggled);
    }

    fn setup_factory(self: &Rc<Self>) {
        let factory = gtk::SignalListItemFactory::new();
        let handlers: Rc<RefCell<HashMap<usize, (SidebarItem, glib::SignalHandlerId)>>> =
            Rc::default();
        factory.connect_setup(|_, item| {
            let Some(li) = item.downcast_ref::<gtk::ListItem>() else {
                return;
            };
            let expander = gtk::TreeExpander::new();
            expander.set_indent_for_icon(true);
            expander.set_indent_for_depth(true);
            li.set_child(Some(&expander));
        });
        let weak = Rc::downgrade(self);
        let bind_handlers = handlers.clone();
        factory.connect_bind(move |_, item| {
            let Some(li) = item.downcast_ref::<gtk::ListItem>() else {
                return;
            };
            let Some(row) = li.item().and_downcast::<gtk::TreeListRow>() else {
                return;
            };
            let Some(obj) = row.item().and_downcast::<SidebarItem>() else {
                return;
            };
            let Some(s) = weak.upgrade() else { return };
            s.render(li, &row, &obj);
            let li_weak = li.downgrade();
            let row_weak = row.downgrade();
            let weak = weak.clone();
            let id = obj.connect_revision_notify(move |obj| {
                if let (Some(li), Some(row), Some(s)) =
                    (li_weak.upgrade(), row_weak.upgrade(), weak.upgrade())
                {
                    s.render(&li, &row, obj);
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
        self.list.set_factory(Some(&factory));
    }

    fn render(self: &Rc<Self>, li: &gtk::ListItem, row: &gtk::TreeListRow, obj: &SidebarItem) {
        let Some(expander) = li.child().and_downcast::<gtk::TreeExpander>() else {
            return;
        };
        expander.set_list_row(Some(row));
        let d = obj.data();
        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .css_classes(["tick-sidebar-row-content"])
            .build();
        match &d.kind {
            ItemKind::Header {
                section,
                collapsed,
                empty,
            } => {
                li.set_selectable(false);
                li.set_activatable(true);
                li.set_focusable(true);
                let state = if *collapsed {
                    gettext("collapsed")
                } else {
                    gettext("expanded")
                };
                li.set_accessible_label(&fill(
                    &gettext("{name}, {state}"),
                    &[("name", &d.name), ("state", &state)],
                ));
                li.set_accessible_description("");
                let title = gtk::Label::builder()
                    .label(&d.name)
                    .xalign(0.0)
                    .hexpand(true)
                    .css_classes(["caption-heading", "dimmed"])
                    .build();
                content.append(&title);
                let add_name = match section {
                    Section::Projects => Some(gettext("Add Project")),
                    Section::Labels => Some(gettext("Add Label")),
                    Section::Filters => Some(gettext("Add Filter")),
                    Section::Favorites => None,
                };
                if let Some(name) = add_name {
                    let add = gtk::Button::builder()
                        .icon_name("list-add-symbolic")
                        .valign(gtk::Align::Center)
                        .css_classes(["flat", "tick-icon-44"])
                        .tooltip_text(&name)
                        .build();
                    // An empty section shows its button at rest: it is the only visible way
                    // to create the first project, label or filter (4.1).
                    if !*empty {
                        add.add_css_class("tick-reveal");
                    }
                    add.update_property(&[gtk::accessible::Property::Label(&name)]);
                    let weak = Rc::downgrade(self);
                    let section = *section;
                    add.connect_clicked(move |_| {
                        if let Some(s) = weak.upgrade() {
                            (s.events)(SidebarEvent::Add(section));
                        }
                    });
                    content.append(&add);
                }
                let chevron = gtk::Image::builder()
                    .icon_name("pan-down-symbolic")
                    .accessible_role(gtk::AccessibleRole::Presentation)
                    .css_classes(["dimmed"])
                    .build();
                // The chevron sits in a box so the rotation transition has a stable node.
                let holder = gtk::Box::builder().css_classes(["tick-chevron"]).build();
                if *collapsed {
                    holder.add_css_class("collapsed");
                }
                holder.append(&chevron);
                content.append(&holder);
                content.add_css_class("tick-sidebar-header");
                let click = gtk::GestureClick::new();
                let weak = Rc::downgrade(self);
                let section = *section;
                click.connect_released(move |g, _, _, _| {
                    g.set_state(gtk::EventSequenceState::Claimed);
                    if let Some(s) = weak.upgrade() {
                        s.toggle(section);
                    }
                });
                content.add_controller(click);
                if section != Section::Favorites {
                    self.menu_gestures(&content, d.kind.clone());
                }
            }
            ItemKind::View(view) => {
                li.set_selectable(true);
                li.set_activatable(true);
                li.set_focusable(true);
                let n = d.count.unwrap_or(0);
                let name = if n > 0 {
                    fill(
                        &gettext("{name}, {count}"),
                        &[
                            ("name", &d.name),
                            ("count", &ncount("{n} task", "{n} tasks", n)),
                        ],
                    )
                } else {
                    d.name.clone()
                };
                li.set_accessible_label(&name);
                li.set_accessible_description("");
                let lead = adw::Bin::builder()
                    .width_request(16)
                    .accessible_role(gtk::AccessibleRole::Presentation)
                    .build();
                if let Some(color) = &d.color {
                    lead.set_child(Some(
                        &gtk::Box::builder()
                            .halign(gtk::Align::Center)
                            .valign(gtk::Align::Center)
                            .css_classes(["tick-dot".to_owned(), color_class(color)])
                            .build(),
                    ));
                } else if let Some(icon) = d.icon {
                    lead.set_child(Some(
                        &gtk::Image::builder()
                            .icon_name(icon)
                            .accessible_role(gtk::AccessibleRole::Presentation)
                            .build(),
                    ));
                }
                content.append(&lead);
                let title = gtk::Label::builder()
                    .label(&d.name)
                    .xalign(0.0)
                    .hexpand(!d.shared)
                    .ellipsize(gtk::pango::EllipsizeMode::End)
                    .css_classes(["body"])
                    .build();
                content.append(&title);
                if d.shared {
                    let users = gtk::Image::builder()
                        .icon_name("system-users-symbolic")
                        .pixel_size(12)
                        .hexpand(true)
                        .halign(gtk::Align::Start)
                        .accessible_role(gtk::AccessibleRole::Presentation)
                        .css_classes(["dimmed"])
                        .build();
                    content.append(&users);
                }
                if n > 0 {
                    content.append(
                        &gtk::Label::builder()
                            .label(n.to_string())
                            .css_classes(["caption", "numeric", "dimmed", "tick-count"])
                            .build(),
                    );
                }
                if row.is_expandable() {
                    let action = if row.is_expanded() {
                        gettext("Collapse {name}")
                    } else {
                        gettext("Expand {name}")
                    };
                    expander.update_property(&[gtk::accessible::Property::Label(&fill(
                        &action,
                        &[("name", &d.name)],
                    ))]);
                }
                let click = gtk::GestureClick::new();
                let weak = Rc::downgrade(self);
                let view = view.clone();
                click.connect_released(move |_, _, _, _| {
                    if let Some(s) = weak.upgrade() {
                        (s.events)(SidebarEvent::Open {
                            view: view.clone(),
                            push: true,
                        });
                    }
                });
                content.add_controller(click);
                if let ItemKind::View(ViewId::Project(id)) = &d.kind {
                    let (w1, w2) = (Rc::downgrade(self), Rc::downgrade(self));
                    let name = d.name.clone();
                    let color = d.color.clone().unwrap_or_default();
                    crate::dnd::drag_source(
                        content.upcast_ref(),
                        format!("{}{}", crate::dnd::PROJECT, id),
                        move || project_drag_icon(&name, &color),
                        move || {
                            if let Some(s) = w1.upgrade() {
                                (s.events)(SidebarEvent::DragBegin);
                            }
                        },
                        move || {
                            if let Some(s) = w2.upgrade() {
                                (s.events)(SidebarEvent::DragEnd);
                            }
                        },
                    );
                }
                if !matches!(
                    &d.kind,
                    ItemKind::View(
                        ViewId::Inbox
                            | ViewId::Today
                            | ViewId::Upcoming
                            | ViewId::Search
                            | ViewId::Completed
                    )
                ) {
                    self.menu_gestures(&content, d.kind.clone());
                }
            }
        }
        expander.set_child(Some(&content));
    }

    /// Right-click and long-press open the row's context menu (4.1).
    fn menu_gestures(self: &Rc<Self>, content: &gtk::Box, kind: ItemKind) {
        let open = {
            let weak = Rc::downgrade(self);
            move |w: &gtk::Widget, x: f64, y: f64| {
                let Some(s) = weak.upgrade() else { return };
                if let Some(p) =
                    w.compute_point(&s.list, &gtk::graphene::Point::new(x as f32, y as f32))
                {
                    (s.events)(SidebarEvent::Menu {
                        kind: kind.clone(),
                        rect: gtk::gdk::Rectangle::new(p.x() as i32, p.y() as i32, 1, 1),
                    });
                }
            }
        };
        let right = gtk::GestureClick::new();
        right.set_button(gtk::gdk::BUTTON_SECONDARY);
        let o = open.clone();
        right.connect_pressed(move |g, _, x, y| {
            g.set_state(gtk::EventSequenceState::Claimed);
            if let Some(w) = g.widget() {
                o(&w, x, y);
            }
        });
        content.add_controller(right);
        let long = gtk::GestureLongPress::new();
        long.set_touch_only(true);
        long.connect_pressed(move |g, x, y| {
            if let Some(w) = g.widget() {
                open(&w, x, y);
            }
        });
        content.add_controller(long);
    }

    /// Show new data: counts update in place; structural changes rebuild the model and
    /// keep the expanded projects.
    pub fn update(self: &Rc<Self>, data: SidebarData) {
        let same = self
            .data
            .borrow()
            .as_ref()
            .is_some_and(|old| shape(old) == shape(&data));
        if same {
            self.update_counts(&data);
        } else {
            self.rebuild(&data);
        }
        self.data.replace(Some(data));
    }

    fn update_counts(&self, d: &SidebarData) {
        let mut counts: HashMap<ViewId, usize> = HashMap::new();
        counts.insert(ViewId::Inbox, d.inbox);
        counts.insert(ViewId::Today, d.today);
        fn walk(list: &[SidebarProject], counts: &mut HashMap<ViewId, usize>) {
            for p in list {
                counts.insert(ViewId::Project(p.id.clone()), p.count);
                walk(&p.children, counts);
            }
        }
        walk(&d.projects, &mut counts);
        for e in d.labels.iter().chain(&d.filters).chain(&d.favorites) {
            counts.insert(e.view.clone(), e.count);
        }
        fn apply(store: &gio::ListStore, counts: &HashMap<ViewId, usize>) {
            for i in 0..store.n_items() {
                let Some(item) = store.item(i).and_downcast::<SidebarItem>() else {
                    continue;
                };
                let mut data = item.data();
                if let ItemKind::View(v) = &data.kind
                    && data.count.is_some()
                    && let Some(n) = counts.get(v)
                {
                    data.count = Some(*n);
                    item.set_data(data);
                }
                if let Some(children) = item.children() {
                    apply(&children, counts);
                }
            }
        }
        apply(&self.root, &counts);
    }

    fn expanded_projects(&self) -> Option<HashSet<String>> {
        self.data.borrow().as_ref()?;
        let mut out = HashSet::new();
        for i in 0..self.tree.n_items() {
            let Some(row) = self.tree.item(i).and_downcast::<gtk::TreeListRow>() else {
                continue;
            };
            if row.is_expanded()
                && let Some(ViewId::Project(id)) = row
                    .item()
                    .and_downcast::<SidebarItem>()
                    .and_then(|i| i.view())
            {
                out.insert(id);
            }
        }
        Some(out)
    }

    fn rebuild(self: &Rc<Self>, d: &SidebarData) {
        let expanded = self.expanded_projects();
        let collapsed = self.collapsed.borrow().clone();
        let mut items = vec![
            view_item(
                ViewId::Inbox,
                gettext("Inbox"),
                "tick-inbox-symbolic",
                count(d.inbox),
            ),
            view_item(
                ViewId::Today,
                gettext("Today"),
                "tick-today-symbolic",
                count(d.today),
            ),
            view_item(
                ViewId::Upcoming,
                gettext("Upcoming"),
                "x-office-calendar-symbolic",
                None,
            ),
            view_item(
                ViewId::Search,
                gettext("Search"),
                "system-search-symbolic",
                None,
            ),
            view_item(
                ViewId::Completed,
                gettext("Completed"),
                "object-select-symbolic",
                None,
            ),
        ];
        let mut section = |section: Section, entries: Vec<SidebarItem>| {
            // Only Favorites hides when empty (4.1): the other headers say where projects,
            // labels and filters will appear, and hold their "Add ..." buttons.
            if entries.is_empty() && section == Section::Favorites {
                return;
            }
            let is_collapsed = collapsed.contains(&section);
            items.push(SidebarItem::new(
                ItemData {
                    kind: ItemKind::Header {
                        section,
                        collapsed: is_collapsed,
                        empty: entries.is_empty(),
                    },
                    name: section.title(),
                    icon: None,
                    color: None,
                    count: None,
                    shared: false,
                },
                None,
            ));
            if !is_collapsed {
                items.extend(entries);
            }
        };
        section(
            Section::Favorites,
            d.favorites.iter().map(entry_item).collect(),
        );
        section(
            Section::Projects,
            d.projects.iter().map(project_item).collect(),
        );
        section(Section::Labels, d.labels.iter().map(entry_item).collect());
        section(Section::Filters, d.filters.iter().map(entry_item).collect());
        self.suppress.set(true);
        self.root.splice(0, self.root.n_items(), &items);
        // Expand projects: keep what the user had open; new ones follow their API state.
        let mut collapsed_projects = HashSet::new();
        fn collect(list: &[SidebarProject], out: &mut HashSet<String>) {
            for p in list {
                if p.collapsed {
                    out.insert(p.id.clone());
                }
                collect(&p.children, out);
            }
        }
        collect(&d.projects, &mut collapsed_projects);
        let mut i = 0;
        while i < self.tree.n_items() {
            if let Some(row) = self.tree.item(i).and_downcast::<gtk::TreeListRow>()
                && row.is_expandable()
                && let Some(ViewId::Project(id)) = row
                    .item()
                    .and_downcast::<SidebarItem>()
                    .and_then(|i| i.view())
            {
                let open = match &expanded {
                    Some(set) => set.contains(&id) || !collapsed_projects.contains(&id),
                    None => !collapsed_projects.contains(&id),
                };
                row.set_expanded(open);
            }
            i += 1;
        }
        self.suppress.set(false);
        if let Some(v) = self.current.borrow().clone() {
            self.select_quiet(&v);
        }
    }

    /// Mark `view` as the current one without opening it again.
    pub fn select(&self, view: &ViewId) {
        self.current.replace(Some(view.clone()));
        self.select_quiet(view);
    }

    fn select_quiet(&self, view: &ViewId) {
        self.suppress.set(true);
        let pos = (0..self.tree.n_items())
            .find(|i| self.item_at(*i).and_then(|it| it.view()).as_ref() == Some(view));
        match pos {
            Some(p) => self.selection.set_selected(p),
            None => self.selection.set_selected(gtk::INVALID_LIST_POSITION),
        }
        self.suppress.set(false);
    }

    /// The display name of a view (for the window title before its rows load).
    pub fn name_of(&self, view: &ViewId) -> Option<String> {
        (0..self.tree.n_items())
            .filter_map(|i| self.item_at(i))
            .find(|it| it.view().as_ref() == Some(view))
            .map(|it| it.data().name)
    }

    pub fn list(&self) -> &gtk::ListView {
        &self.list
    }
}

/// The drag icon of a project row: dot and name on the floating surface (4.1).
fn project_drag_icon(name: &str, color: &str) -> gtk::Widget {
    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .css_classes(["tick-drag-icon", "tick-project-drag"])
        .build();
    root.append(
        &gtk::Box::builder()
            .valign(gtk::Align::Center)
            .css_classes(["tick-dot".to_owned(), color_class(color)])
            .build(),
    );
    root.append(&gtk::Label::new(Some(name)));
    root.upcast()
}

fn view_item(view: ViewId, name: String, icon: &'static str, count: Option<usize>) -> SidebarItem {
    SidebarItem::new(
        ItemData {
            kind: ItemKind::View(view),
            name,
            icon: Some(icon),
            color: None,
            count,
            shared: false,
        },
        None,
    )
}

fn entry_item(e: &SidebarEntry) -> SidebarItem {
    let icon = match e.view {
        ViewId::Label(_) => Some("tick-label-symbolic"),
        ViewId::Filter(_) => Some("tick-filter-symbolic"),
        _ => None,
    };
    SidebarItem::new(
        ItemData {
            kind: ItemKind::View(e.view.clone()),
            name: e.name.clone(),
            icon,
            color: e.color.clone(),
            count: Some(e.count),
            shared: false,
        },
        None,
    )
}

fn project_item(p: &SidebarProject) -> SidebarItem {
    let children = (!p.children.is_empty()).then(|| {
        let store = gio::ListStore::new::<SidebarItem>();
        let items: Vec<SidebarItem> = p.children.iter().map(project_item).collect();
        store.extend_from_slice(&items);
        store
    });
    SidebarItem::new(
        ItemData {
            kind: ItemKind::View(ViewId::Project(p.id.clone())),
            name: p.name.clone(),
            icon: None,
            color: Some(p.color.clone()),
            count: Some(p.count),
            shared: p.shared,
        },
        children,
    )
}
