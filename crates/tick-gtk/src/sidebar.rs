//! The sidebar (`DESIGN.md` 4.1): a `GtkListView` with `.navigation-sidebar` over a
//! `GtkTreeListModel` of system views, favorites, the project tree, labels and filters.
//!
//! Search and Completed (milestone 4) and the "Add Project/Label/Filter" buttons (their
//! dialogs come later) are not shown yet.

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
    Header { section: Section, collapsed: bool },
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

/// What the sidebar asks the window to do.
pub enum SidebarEvent {
    /// Open a view; `push` also shows the content page on narrow windows.
    Open { view: ViewId, push: bool },
    /// A section header was collapsed or expanded (persist it).
    SectionToggled,
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
        sidebar
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
            ItemKind::Header { section, collapsed } => {
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
            }
        }
        expander.set_child(Some(&content));
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
