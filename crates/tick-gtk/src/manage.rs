//! Managing projects, sections, labels and filters (`DESIGN.md` 4.19, 4.23 to 4.25): the
//! edit dialogs with the color picker, confirmations, context and view menus, archive with
//! Undo, the inline section and title editors, and the keyboard moves of projects.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};
use serde_json::json;
use tick_core::Command;
use tick_core::filter::{self, Group};

use crate::i18n::{fill, gettext, ncount, pgettext};
use crate::rows::{SectionEdit, ViewId, color_class};
use crate::sidebar::{ItemKind, ProjectMove, Section};
use crate::window::TickWindow;

/// The 20 API colors in API order (`docs/api/pages/colors.md`).
pub const COLORS: [&str; 20] = [
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

/// A color's display name (4.23.1).
pub fn color_name(c: &str) -> String {
    match c {
        "berry_red" => pgettext("color", "Berry Red"),
        "red" => pgettext("color", "Red"),
        "orange" => pgettext("color", "Orange"),
        "yellow" => pgettext("color", "Yellow"),
        "olive_green" => pgettext("color", "Olive Green"),
        "lime_green" => pgettext("color", "Lime Green"),
        "green" => pgettext("color", "Green"),
        "mint_green" => pgettext("color", "Mint Green"),
        "teal" => pgettext("color", "Teal"),
        "sky_blue" => pgettext("color", "Sky Blue"),
        "light_blue" => pgettext("color", "Light Blue"),
        "blue" => pgettext("color", "Blue"),
        "grape" => pgettext("color", "Grape"),
        "violet" => pgettext("color", "Violet"),
        "lavender" => pgettext("color", "Lavender"),
        "magenta" => pgettext("color", "Magenta"),
        "salmon" => pgettext("color", "Salmon"),
        "grey" => pgettext("color", "Grey"),
        "taupe" => pgettext("color", "Taupe"),
        _ => pgettext("color", "Charcoal"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    Project,
    Label,
    Filter,
}

/// Which field an edit dialog focuses (Rename and Color open Edit on their field, 4.23).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogFocus {
    Name,
    Color,
}

/// The query hint of the filter dialog (4.23), from the local parser.
pub fn query_hint(query: &str) -> String {
    let q = query.trim();
    if q.is_empty() {
        return gettext("Examples: \"today | overdue\", \"#Work & p1\"");
    }
    if filter::parse(q)
        .iter()
        .any(|g| matches!(g, Group::Unsupported { .. }))
    {
        gettext("Part of this query is checked by Todoist. Results need a connection.")
    } else {
        gettext("Runs on this computer.")
    }
}

/// What an edit dialog starts from.
struct Existing {
    id: String,
    name: String,
    query: String,
    color: String,
    favorite: bool,
    parent: Option<String>,
}

impl TickWindow {
    pub(crate) fn setup_manage(&self) {
        let with_id: &[(&str, fn(&TickWindow, &str))] = &[
            ("project-rename", |w, id| w.rename_project(id)),
            ("project-color", |w, id| {
                w.open_entity_dialog(EntityKind::Project, Some(id), DialogFocus::Color)
            }),
            ("project-favorite", |w, id| {
                w.toggle_favorite(EntityKind::Project, id)
            }),
            ("project-move", |w, id| w.project_move_picker(id)),
            ("project-archive", |w, id| w.archive_project(id)),
            ("project-delete", |w, id| w.confirm_delete_project(id)),
            ("label-edit", |w, id| {
                w.open_entity_dialog(EntityKind::Label, Some(id), DialogFocus::Name)
            }),
            ("label-favorite", |w, id| {
                w.toggle_favorite(EntityKind::Label, id)
            }),
            ("label-delete", |w, id| w.confirm_delete_label(id)),
            ("filter-edit", |w, id| {
                w.open_entity_dialog(EntityKind::Filter, Some(id), DialogFocus::Name)
            }),
            ("filter-favorite", |w, id| {
                w.toggle_favorite(EntityKind::Filter, id)
            }),
            ("filter-delete", |w, id| w.confirm_delete_filter(id)),
            ("add-section", |w, id| w.add_section(id)),
            ("section-rename", |w, id| w.rename_section(id)),
            ("section-delete", |w, id| w.confirm_delete_section(id)),
            ("section-add-task", |w, id| w.add_task_in_section(id)),
            ("sidebar-toggle", |w, key| {
                if let (Some(sb), Some(section)) = (w.imp().sidebar.get(), Section::from_key(key)) {
                    sb.toggle(section);
                }
            }),
        ];
        for (name, f) in with_id {
            let f = *f;
            let a = gio::SimpleAction::new(name, Some(glib::VariantTy::STRING));
            let weak = self.downgrade();
            a.connect_activate(move |_, p| {
                if let (Some(w), Some(id)) = (weak.upgrade(), p.and_then(|p| p.get::<String>())) {
                    f(&w, &id);
                }
            });
            self.add_action(&a);
        }
        let plain: &[(&str, fn(&TickWindow))] = &[
            ("add-project", |w| {
                w.open_entity_dialog(EntityKind::Project, None, DialogFocus::Name)
            }),
            ("add-label", |w| {
                w.open_entity_dialog(EntityKind::Label, None, DialogFocus::Name)
            }),
            ("add-filter", |w| {
                w.open_entity_dialog(EntityKind::Filter, None, DialogFocus::Name)
            }),
            ("view-rename", |w| w.rename_title()),
        ];
        for (name, f) in plain {
            let f = *f;
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

    pub(crate) fn add_from_sidebar(&self, section: Section) {
        let kind = match section {
            Section::Projects => EntityKind::Project,
            Section::Labels => EntityKind::Label,
            Section::Filters => EntityKind::Filter,
            Section::Favorites => return,
        };
        self.open_entity_dialog(kind, None, DialogFocus::Name);
    }

    /// The menu of a project, label or filter (sidebar rows and the view menu, 4.19).
    fn entity_menu(&self, view: &ViewId, in_view_menu: bool) -> Option<gio::Menu> {
        let item = |label: &str, action: &str, target: &str| {
            let i = gio::MenuItem::new(Some(label), None);
            i.set_action_and_target_value(Some(action), Some(&target.to_variant()));
            i
        };
        let menu = gio::Menu::new();
        let main = gio::Menu::new();
        let danger = gio::Menu::new();
        let store = self.store();
        let fav_label = |fav: bool| {
            if fav {
                gettext("Unfavorite")
            } else {
                gettext("Favorite")
            }
        };
        match view {
            ViewId::Project(id) => {
                let p = store.project(id).ok().flatten()?;
                if p.inbox_project {
                    return None;
                }
                if in_view_menu {
                    main.append(Some(&gettext("Rename")), Some("win.view-rename"));
                } else {
                    main.append_item(&item(&gettext("Rename"), "win.project-rename", id));
                }
                main.append_item(&item(&gettext("Color"), "win.project-color", id));
                if in_view_menu {
                    main.append_item(&item(&gettext("Add Section"), "win.add-section", id));
                }
                main.append_item(&item(&fav_label(p.is_favorite), "win.project-favorite", id));
                if !in_view_menu {
                    main.append_item(&item(&gettext("Move to..."), "win.project-move", id));
                }
                main.append_item(&item(&gettext("Archive"), "win.project-archive", id));
                danger.append_item(&item(
                    &gettext("Delete Project..."),
                    "win.project-delete",
                    id,
                ));
            }
            ViewId::Label(id) => {
                let l = store
                    .labels()
                    .ok()?
                    .into_iter()
                    .find(|l| l.label.id == *id)?;
                main.append_item(&item(&gettext("Edit"), "win.label-edit", id));
                main.append_item(&item(
                    &fav_label(l.label.is_favorite),
                    "win.label-favorite",
                    id,
                ));
                danger.append_item(&item(&gettext("Delete Label..."), "win.label-delete", id));
            }
            ViewId::Filter(id) => {
                let f = store.filters().ok()?.into_iter().find(|f| f.id == *id)?;
                main.append_item(&item(&gettext("Edit"), "win.filter-edit", id));
                main.append_item(&item(&fav_label(f.is_favorite), "win.filter-favorite", id));
                danger.append_item(&item(&gettext("Delete Filter..."), "win.filter-delete", id));
            }
            ViewId::Inbox | ViewId::Today | ViewId::Upcoming => return None,
        }
        menu.append_section(None, &main);
        menu.append_section(None, &danger);
        Some(menu)
    }

    /// The sidebar context menu (4.1) at `rect` in the sidebar list's coordinates.
    pub(crate) fn sidebar_menu(&self, kind: &ItemKind, rect: gtk::gdk::Rectangle) {
        let menu = match kind {
            ItemKind::Header { section, collapsed } => {
                let menu = gio::Menu::new();
                let (label, action) = match section {
                    Section::Projects => (gettext("Add Project"), "win.add-project"),
                    Section::Labels => (gettext("Add Label"), "win.add-label"),
                    Section::Filters => (gettext("Add Filter"), "win.add-filter"),
                    Section::Favorites => return,
                };
                menu.append(Some(&label), Some(action));
                let toggle = gio::MenuItem::new(
                    Some(&if *collapsed {
                        gettext("Expand")
                    } else {
                        gettext("Collapse")
                    }),
                    None,
                );
                toggle.set_action_and_target_value(
                    Some("win.sidebar-toggle"),
                    Some(&section.key().to_variant()),
                );
                menu.append_item(&toggle);
                menu
            }
            ItemKind::View(view) => match self.entity_menu(view, false) {
                Some(m) => m,
                None => return,
            },
        };
        let Some(sb) = self.imp().sidebar.get() else {
            return;
        };
        let pop = gtk::PopoverMenu::from_model(Some(&menu));
        pop.set_has_arrow(false);
        pop.set_position(gtk::PositionType::Bottom);
        crate::pickers::present(pop.upcast_ref(), sb.list(), Some(rect));
    }

    /// Show the view menu for the current view (4.19); hidden for system views.
    pub(crate) fn update_view_menu(&self) {
        let view = self.current_view();
        let menu = self.entity_menu(&view, true);
        let button = self.imp().view_menu_button.get();
        button.set_visible(menu.is_some());
        button.set_menu_model(menu.as_ref());
    }

    // ---- dialogs (4.23) ----

    fn existing(&self, kind: EntityKind, id: &str) -> Option<Existing> {
        let store = self.store();
        match kind {
            EntityKind::Project => store.project(id).ok().flatten().map(|p| Existing {
                id: p.id,
                name: p.name,
                query: String::new(),
                color: p.color,
                favorite: p.is_favorite,
                parent: p.parent_id,
            }),
            EntityKind::Label => store
                .labels()
                .ok()?
                .into_iter()
                .find(|l| l.label.id == id)
                .map(|l| Existing {
                    id: l.label.id,
                    name: l.label.name,
                    query: String::new(),
                    color: l.label.color,
                    favorite: l.label.is_favorite,
                    parent: None,
                }),
            EntityKind::Filter => store
                .filters()
                .ok()?
                .into_iter()
                .find(|f| f.id == id)
                .map(|f| Existing {
                    id: f.id,
                    name: f.name,
                    query: f.query,
                    color: f.color,
                    favorite: f.is_favorite,
                    parent: None,
                }),
        }
    }

    /// Parent choices for a project: "None", then the project tree as paths, without the
    /// project itself and its descendants (4.23).
    fn parent_choices(&self, exclude: Option<&str>) -> Vec<(Option<String>, String)> {
        let tree = self.store().projects_tree().unwrap_or_default();
        let mut out = vec![(None, gettext("None"))];
        let mut path: Vec<String> = Vec::new();
        let mut skip_depth: Option<usize> = None;
        for n in tree {
            if n.project.inbox_project {
                continue;
            }
            if let Some(d) = skip_depth {
                if n.depth > d {
                    continue;
                }
                skip_depth = None;
            }
            path.truncate(n.depth);
            path.push(n.project.name.clone());
            if exclude == Some(n.project.id.as_str()) {
                skip_depth = Some(n.depth);
                continue;
            }
            out.push((Some(n.project.id.clone()), path.join(" / ")));
        }
        out
    }

    /// Open the New/Edit dialog for a project, label or filter (4.23).
    pub(crate) fn open_entity_dialog(
        &self,
        kind: EntityKind,
        id: Option<&str>,
        focus: DialogFocus,
    ) {
        let existing = id.and_then(|id| self.existing(kind, id));
        if id.is_some() && existing.is_none() {
            return;
        }
        let editing = existing.is_some();
        let title = match (kind, editing) {
            (EntityKind::Project, false) => gettext("New Project"),
            (EntityKind::Project, true) => gettext("Edit Project"),
            (EntityKind::Label, false) => gettext("New Label"),
            (EntityKind::Label, true) => gettext("Edit Label"),
            (EntityKind::Filter, false) => gettext("New Filter"),
            (EntityKind::Filter, true) => gettext("Edit Filter"),
        };
        let dialog = adw::Dialog::builder()
            .title(&title)
            .content_width(420)
            .css_classes(["tick-entity-dialog"])
            .build();
        let header = adw::HeaderBar::builder()
            .show_start_title_buttons(false)
            .show_end_title_buttons(false)
            .build();
        let cancel = gtk::Button::builder()
            .label(gettext("Cancel"))
            .css_classes(["flat"])
            .build();
        let save = gtk::Button::builder()
            .label(if editing {
                gettext("Save")
            } else {
                gettext("Add")
            })
            .css_classes(["suggested-action"])
            .sensitive(false)
            .build();
        header.pack_start(&cancel);
        header.pack_end(&save);
        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin_start(18)
            .margin_end(18)
            .margin_top(6)
            .margin_bottom(24)
            .build();
        let list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(["boxed-list"])
            .build();
        let name = adw::EntryRow::builder()
            .title(gettext("Name"))
            .activates_default(true)
            .build();
        list.append(&name);
        let query = (kind == EntityKind::Filter).then(|| {
            let q = adw::EntryRow::builder()
                .title(gettext("Query"))
                .activates_default(true)
                .css_classes(["monospace"])
                .build();
            list.append(&q);
            q
        });
        let parents = (kind == EntityKind::Project)
            .then(|| self.parent_choices(existing.as_ref().map(|e| e.id.as_str())));
        let parent_row = parents.as_ref().map(|choices| {
            let names: Vec<&str> = choices.iter().map(|(_, n)| n.as_str()).collect();
            let model = gtk::StringList::new(&names);
            let row = adw::ComboRow::builder()
                .title(gettext("Parent Project"))
                .model(&model)
                .build();
            let current = existing.as_ref().and_then(|e| e.parent.clone());
            if let Some(i) = choices.iter().position(|(id, _)| *id == current) {
                row.set_selected(i as u32);
            }
            list.append(&row);
            row
        });
        let favorite = adw::SwitchRow::builder()
            .title(gettext("Add to Favorites"))
            .build();
        list.append(&favorite);
        content.append(&list);
        let hint = gtk::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .margin_top(6)
            .css_classes(["caption", "dimmed"])
            .visible(kind == EntityKind::Filter)
            .build();
        content.append(&hint);
        let error = gtk::Label::builder()
            .label(gettext("A label with this name already exists."))
            .xalign(0.0)
            .wrap(true)
            .margin_top(6)
            .css_classes(["caption", "error"])
            .visible(false)
            .build();
        content.append(&error);
        let color_heading = gtk::Label::builder()
            .label(gettext("Color"))
            .xalign(0.0)
            .margin_top(18)
            .css_classes(["heading"])
            .build();
        content.append(&color_heading);
        let grid = adw::WrapBox::builder()
            .child_spacing(0)
            .line_spacing(0)
            .margin_top(6)
            .build();
        grid.upcast_ref::<gtk::Widget>()
            .update_property(&[gtk::accessible::Property::Label(&gettext("Color"))]);
        let start_color = existing
            .as_ref()
            .map(|e| e.color.clone())
            .filter(|c| COLORS.contains(&c.as_str()))
            .unwrap_or_else(|| "charcoal".into());
        let selected = Rc::new(RefCell::new(start_color.clone()));
        let color_label = gtk::Label::builder()
            .label(color_name(&start_color))
            .xalign(0.0)
            .margin_top(6)
            .css_classes(["caption", "dimmed"])
            .build();
        let mut group: Option<gtk::CheckButton> = None;
        let mut first_checked = None;
        for c in COLORS {
            let dot = gtk::Box::builder()
                .halign(gtk::Align::Center)
                .valign(gtk::Align::Center)
                .accessible_role(gtk::AccessibleRole::Presentation)
                .css_classes(["tick-dot".to_owned(), color_class(c)])
                .build();
            let b = gtk::CheckButton::builder()
                .child(&dot)
                .css_classes(["tick-swatch"])
                .build();
            b.update_property(&[gtk::accessible::Property::Label(&color_name(c))]);
            if let Some(g) = &group {
                b.set_group(Some(g));
            } else {
                group = Some(b.clone());
            }
            if c == start_color {
                b.set_active(true);
                first_checked = Some(b.clone());
            }
            let selected = selected.clone();
            let color_label = color_label.downgrade();
            b.connect_toggled(move |b| {
                if b.is_active() {
                    selected.replace(c.to_owned());
                    if let Some(l) = color_label.upgrade() {
                        l.set_label(&color_name(c));
                    }
                }
            });
            grid.append(&b);
        }
        content.append(&grid);
        content.append(&color_label);
        let view = adw::ToolbarView::new();
        view.add_top_bar(&header);
        let scroller = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .propagate_natural_height(true)
            .child(&content)
            .build();
        view.set_content(Some(&scroller));
        dialog.set_child(Some(&view));
        dialog.set_default_widget(Some(&save));

        if let Some(e) = &existing {
            name.set_text(&e.name);
            favorite.set_active(e.favorite);
            if let Some(q) = &query {
                q.set_text(&e.query);
            }
        }

        // Validation: Name (and Query) not empty; label names unique (case-insensitive).
        let other_labels: Vec<String> = if kind == EntityKind::Label {
            let own = existing.as_ref().map(|e| e.id.clone());
            self.store()
                .labels()
                .unwrap_or_default()
                .into_iter()
                .filter(|l| Some(&l.label.id) != own.as_ref())
                .map(|l| l.label.name.to_lowercase())
                .collect()
        } else {
            Vec::new()
        };
        let validate = {
            let name = name.downgrade();
            let query = query.as_ref().map(|q| q.downgrade());
            let save = save.downgrade();
            let hint = hint.downgrade();
            let error = error.downgrade();
            move || {
                let (Some(name), Some(save)) = (name.upgrade(), save.upgrade()) else {
                    return;
                };
                let n = name.text().trim().to_owned();
                let q = query
                    .as_ref()
                    .and_then(|q| q.upgrade())
                    .map(|q| q.text().to_string());
                let dup = other_labels.contains(&n.to_lowercase());
                if dup {
                    name.add_css_class("error");
                } else {
                    name.remove_css_class("error");
                }
                if let Some(e) = error.upgrade() {
                    e.set_visible(dup);
                }
                if let (Some(h), Some(q)) = (hint.upgrade(), &q) {
                    h.set_label(&query_hint(q));
                }
                let ok = !n.is_empty() && !dup && q.as_ref().is_none_or(|q| !q.trim().is_empty());
                save.set_sensitive(ok);
            }
        };
        let validate = Rc::new(validate);
        validate();
        {
            let v = validate.clone();
            name.connect_changed(move |_| v());
        }
        if let Some(q) = &query {
            let v = validate.clone();
            q.connect_changed(move |_| v());
        }

        {
            let dialog = dialog.downgrade();
            cancel.connect_clicked(move |_| {
                if let Some(d) = dialog.upgrade() {
                    d.close();
                }
            });
        }
        {
            let weak = self.downgrade();
            let dialog_w = dialog.downgrade();
            let name = name.downgrade();
            let query = query.as_ref().map(|q| q.downgrade());
            let favorite = favorite.downgrade();
            let parent_row = parent_row.as_ref().map(|p| p.downgrade());
            let existing = existing.map(Rc::new);
            save.connect_clicked(move |b| {
                if !b.is_sensitive() {
                    return;
                }
                let (Some(w), Some(d), Some(name), Some(favorite)) = (
                    weak.upgrade(),
                    dialog_w.upgrade(),
                    name.upgrade(),
                    favorite.upgrade(),
                ) else {
                    return;
                };
                let n = name.text().trim().to_owned();
                if n.is_empty() {
                    return;
                }
                let q = query
                    .as_ref()
                    .and_then(|q| q.upgrade())
                    .map(|q| q.text().trim().to_owned())
                    .unwrap_or_default();
                let parent = parent_row.as_ref().and_then(|p| p.upgrade()).and_then(|p| {
                    parents
                        .as_ref()
                        .and_then(|c| c.get(p.selected() as usize))
                        .and_then(|(id, _)| id.clone())
                });
                let color = selected.borrow().clone();
                w.save_entity(
                    kind,
                    existing.as_deref(),
                    &n,
                    &q,
                    &color,
                    favorite.is_active(),
                    parent,
                );
                d.close();
            });
        }
        // Focus: the name field, or the checked swatch (Rename / Color from a menu).
        let name_w = name.downgrade();
        dialog.connect_map(move |_| {
            let name_w = name_w.clone();
            let swatch = first_checked.clone();
            glib::idle_add_local_once(move || match focus {
                DialogFocus::Color => {
                    if let Some(s) = swatch {
                        s.grab_focus();
                    }
                }
                DialogFocus::Name => {
                    if let Some(n) = name_w.upgrade() {
                        n.grab_focus();
                    }
                }
            });
        });
        dialog.present(Some(self));
    }

    #[allow(clippy::too_many_arguments)]
    fn save_entity(
        &self,
        kind: EntityKind,
        existing: Option<&Existing>,
        name: &str,
        query: &str,
        color: &str,
        favorite: bool,
        parent: Option<String>,
    ) {
        let mut cmds = Vec::new();
        let mut open = None;
        match (kind, existing) {
            (EntityKind::Project, None) => {
                let mut args = json!({"name": name, "color": color, "is_favorite": favorite});
                if let Some(p) = &parent {
                    args["parent_id"] = json!(p);
                }
                let c = Command::project_add(args);
                open = c.temp_id.clone().map(ViewId::Project);
                cmds.push(c);
            }
            (EntityKind::Project, Some(e)) => {
                cmds.push(Command::project_update(
                    &e.id,
                    json!({"name": name, "color": color, "is_favorite": favorite}),
                ));
                if parent != e.parent {
                    cmds.push(Command::project_move(&e.id, parent.as_deref()));
                }
            }
            (EntityKind::Label, None) => cmds.push(Command::label_add(
                json!({"name": name, "color": color, "is_favorite": favorite}),
            )),
            (EntityKind::Label, Some(e)) => cmds.push(Command::label_update(
                &e.id,
                json!({"name": name, "color": color, "is_favorite": favorite}),
            )),
            (EntityKind::Filter, None) => cmds.push(Command::filter_add(
                json!({"name": name, "query": query, "color": color, "is_favorite": favorite}),
            )),
            (EntityKind::Filter, Some(e)) => cmds.push(Command::filter_update(
                &e.id,
                json!({"name": name, "query": query, "color": color, "is_favorite": favorite}),
            )),
        }
        self.enqueue(cmds);
        if let Some(v) = open {
            self.open_view(v, true);
        }
        self.update_view_menu();
    }

    fn toggle_favorite(&self, kind: EntityKind, id: &str) {
        let Some(e) = self.existing(kind, id) else {
            return;
        };
        let args = json!({ "is_favorite": !e.favorite });
        let cmd = match kind {
            EntityKind::Project => Command::project_update(id, args),
            EntityKind::Label => Command::label_update(id, args),
            EntityKind::Filter => Command::filter_update(id, args),
        };
        self.enqueue(vec![cmd]);
        self.update_view_menu();
    }

    /// Rename from a project menu: in place when it is the open view, else the dialog (4.19).
    fn rename_project(&self, id: &str) {
        if self.current_view() == ViewId::Project(id.to_owned()) {
            self.rename_title();
        } else {
            self.open_entity_dialog(EntityKind::Project, Some(id), DialogFocus::Name);
        }
    }

    /// Start renaming the open project's title in place (4.2).
    pub(crate) fn rename_title(&self) {
        if !matches!(self.current_view(), ViewId::Project(_)) {
            return;
        }
        self.imp().ui.borrow_mut().renaming_title = true;
        self.refresh_rows();
    }

    pub(crate) fn commit_title(&self, name: Option<String>) {
        self.imp().ui.borrow_mut().renaming_title = false;
        if let (ViewId::Project(id), Some(n)) = (self.current_view(), name) {
            let old = self.store().project(&id).ok().flatten().map(|p| p.name);
            if !n.is_empty() && old.as_deref() != Some(n.as_str()) {
                self.enqueue(vec![Command::project_update(&id, json!({ "name": n }))]);
                return;
            }
        }
        self.refresh_rows();
        self.focus_first_task_now();
    }

    // ---- sections (4.24) ----

    fn add_section(&self, project_id: &str) {
        if self.current_view() != ViewId::Project(project_id.to_owned()) {
            self.open_view(ViewId::Project(project_id.to_owned()), true);
        }
        self.imp().ui.borrow_mut().section_edit = Some(SectionEdit {
            project_id: project_id.to_owned(),
            id: None,
        });
        self.refresh_rows();
        self.scroll_to_key("section-editor:new");
    }

    fn rename_section(&self, id: &str) {
        let Some(s) = self.store().section(id).ok().flatten() else {
            return;
        };
        self.imp().ui.borrow_mut().section_edit = Some(SectionEdit {
            project_id: s.project_id.clone(),
            id: Some(id.to_owned()),
        });
        self.refresh_rows();
    }

    pub(crate) fn commit_section(&self, id: Option<String>, name: Option<String>) {
        let edit = self.imp().ui.borrow_mut().section_edit.take();
        let Some(edit) = edit else {
            self.refresh_rows();
            return;
        };
        match (id, name.filter(|n| !n.is_empty())) {
            (None, Some(n)) => {
                self.enqueue(vec![Command::section_add(&edit.project_id, &n)]);
            }
            (Some(id), Some(n)) => {
                let old = self.store().section(&id).ok().flatten().map(|s| s.name);
                if old.as_deref() != Some(n.as_str()) {
                    self.enqueue(vec![Command::section_update(&id, json!({ "name": n }))]);
                } else {
                    self.refresh_rows();
                }
            }
            _ => self.refresh_rows(),
        }
    }

    fn add_task_in_section(&self, id: &str) {
        let store = self.store();
        let Some(s) = store.section(id).ok().flatten() else {
            return;
        };
        let project = store
            .project(&s.project_id)
            .ok()
            .flatten()
            .filter(|p| !p.inbox_project)
            .map(|p| format!("#{} ", crate::rows::quick_add_name(&p.name)))
            .unwrap_or_default();
        drop(store);
        self.open_quick_add(Some(format!(
            "{project}/{} ",
            crate::rows::quick_add_name(&s.name)
        )));
    }

    fn scroll_to_key(&self, key: &str) {
        let imp = self.imp();
        if let Some(store) = imp.rows.get()
            && let Some(pos) = (0..store.n_items()).find(|i| {
                store
                    .item(*i)
                    .and_downcast::<crate::row_object::TickRow>()
                    .is_some_and(|r| r.key() == key)
            })
        {
            imp.task_list
                .scroll_to(pos, gtk::ListScrollFlags::NONE, None);
        }
    }

    // ---- archive and delete (4.25) ----

    fn archive_project(&self, id: &str) {
        let Some(p) = self.store().project(id).ok().flatten() else {
            return;
        };
        if p.inbox_project {
            return;
        }
        let was_open = self.current_view() == ViewId::Project(id.to_owned());
        self.enqueue(vec![Command::project_archive(id)]);
        if was_open {
            self.open_view(ViewId::Inbox, false);
        }
        let id = id.to_owned();
        self.show_undo(
            &fill(&gettext("Archived \"{name}\""), &[("name", &p.name)]),
            move |w| {
                w.enqueue(vec![Command::project_unarchive(&id)]);
                if was_open {
                    w.open_view(ViewId::Project(id.clone()), false);
                }
            },
            |_| {},
        );
    }

    fn confirm(
        &self,
        heading: &str,
        body: &str,
        button: &str,
        on_yes: impl Fn(&TickWindow) + 'static,
    ) {
        let dialog = adw::AlertDialog::new(Some(heading), Some(body));
        dialog.add_response("cancel", &gettext("Cancel"));
        dialog.add_response("delete", button);
        dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        let weak = self.downgrade();
        dialog.connect_response(None, move |_, r| {
            if r == "delete"
                && let Some(w) = weak.upgrade()
            {
                on_yes(&w);
            }
        });
        dialog.present(Some(self));
    }

    fn confirm_delete_project(&self, id: &str) {
        let (p, counts) = {
            let store = self.store();
            let Some(p) = store.project(id).ok().flatten() else {
                return;
            };
            (p, store.project_delete_counts(id).unwrap_or_default())
        };
        // "Its 12 tasks and 2 sub-projects will be deleted too." A clause whose count is 0
        // is dropped (4.25).
        let tasks = ncount("{n} task", "{n} tasks", counts.tasks);
        let subs = ncount("{n} sub-project", "{n} sub-projects", counts.subprojects);
        let first = match (counts.tasks, counts.subprojects) {
            (0, 0) => String::new(),
            (_, 0) => fill(
                &gettext("Its {tasks} will be deleted too."),
                &[("tasks", &tasks)],
            ),
            (0, _) => fill(
                &gettext("Its {projects} will be deleted too."),
                &[("projects", &subs)],
            ),
            _ => fill(
                &gettext("Its {tasks} and {projects} will be deleted too."),
                &[("tasks", &tasks), ("projects", &subs)],
            ),
        };
        let body = [first, gettext("This can't be undone.")]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        let id = id.to_owned();
        self.confirm(
            &fill(&gettext("Delete \"{name}\"?"), &[("name", &p.name)]),
            &body,
            &gettext("Delete Project"),
            move |w| {
                w.enqueue(vec![Command::project_delete(&id)]);
            },
        );
    }

    fn confirm_delete_section(&self, id: &str) {
        let (s, n) = {
            let store = self.store();
            let Some(s) = store.section(id).ok().flatten() else {
                return;
            };
            (s, store.section_task_count(id).unwrap_or(0))
        };
        let mut body = String::new();
        if n > 0 {
            body = fill(
                &gettext("Its {tasks} will be deleted too."),
                &[("tasks", &ncount("{n} task", "{n} tasks", n))],
            );
            body.push(' ');
        }
        body.push_str(&gettext("This can't be undone."));
        let id = id.to_owned();
        self.confirm(
            &fill(&gettext("Delete section \"{name}\"?"), &[("name", &s.name)]),
            &body,
            &gettext("Delete Section"),
            move |w| w.enqueue(vec![Command::section_delete(&id)]),
        );
    }

    fn confirm_delete_label(&self, id: &str) {
        let Some(l) = self
            .store()
            .labels()
            .unwrap_or_default()
            .into_iter()
            .find(|l| l.label.id == id)
        else {
            return;
        };
        let body = if l.active_count > 0 {
            fill(
                &gettext("It will be removed from {tasks}. The tasks stay."),
                &[("tasks", &ncount("{n} task", "{n} tasks", l.active_count))],
            )
        } else {
            gettext("No tasks use it.")
        };
        let id = id.to_owned();
        self.confirm(
            &fill(
                &gettext("Delete label \"{name}\"?"),
                &[("name", &l.label.name)],
            ),
            &body,
            &gettext("Delete Label"),
            move |w| w.enqueue(vec![Command::label_delete(&id)]),
        );
    }

    fn confirm_delete_filter(&self, id: &str) {
        let Some(f) = self
            .store()
            .filters()
            .unwrap_or_default()
            .into_iter()
            .find(|f| f.id == id)
        else {
            return;
        };
        let id = id.to_owned();
        self.confirm(
            &fill(&gettext("Delete filter \"{name}\"?"), &[("name", &f.name)]),
            &gettext("Only the filter is deleted. No tasks change."),
            &gettext("Delete Filter"),
            move |w| w.enqueue(vec![Command::filter_delete(&id)]),
        );
    }

    // ---- moving projects (4.1) ----

    /// Siblings of project `id` (same parent, display order) and its parent.
    fn project_siblings(&self, id: &str) -> Option<(Option<String>, Vec<String>)> {
        let tree = self.store().projects_tree().ok()?;
        let me = tree.iter().find(|n| n.project.id == id)?;
        let parent = me.project.parent_id.clone();
        let sibs = tree
            .iter()
            .filter(|n| !n.project.inbox_project && n.project.parent_id == parent)
            .map(|n| n.project.id.clone())
            .collect();
        Some((parent, sibs))
    }

    /// Put project `id` under `parent` after `after`, with the Undo toast every project move
    /// shows (4.1).
    pub(crate) fn place_project(&self, id: &str, parent: Option<&str>, after: Option<&str>) {
        let Some((old_parent, sibs)) = self.project_siblings(id) else {
            return;
        };
        let pos = sibs.iter().position(|s| s == id);
        let old_after = pos.and_then(|p| p.checked_sub(1)).map(|p| sibs[p].clone());
        let cmds = self
            .store()
            .place_project_commands(id, parent, after)
            .unwrap_or_default();
        if cmds.is_empty() {
            return;
        }
        let name = self
            .store()
            .project(id)
            .ok()
            .flatten()
            .map(|p| p.name)
            .unwrap_or_default();
        self.enqueue(cmds);
        let view = ViewId::Project(id.to_owned());
        if let Some(sb) = self.imp().sidebar.get() {
            sb.focus_view(&view);
        }
        let id = id.to_owned();
        self.show_undo(
            &fill(&gettext("Moved \"{name}\""), &[("name", &name)]),
            move |w| {
                let back = w
                    .store()
                    .place_project_commands(&id, old_parent.as_deref(), old_after.as_deref())
                    .unwrap_or_default();
                w.enqueue(back);
            },
            |_| {},
        );
    }

    pub(crate) fn move_project_key(&self, id: &str, how: ProjectMove) {
        let Some((parent, sibs)) = self.project_siblings(id) else {
            return;
        };
        let Some(pos) = sibs.iter().position(|s| s == id) else {
            return;
        };
        match how {
            ProjectMove::Up if pos > 0 => {
                let after = pos.checked_sub(2).map(|p| sibs[p].clone());
                self.place_project(id, parent.as_deref(), after.as_deref());
            }
            ProjectMove::Down if pos + 1 < sibs.len() => {
                self.place_project(id, parent.as_deref(), Some(&sibs[pos + 1].clone()));
            }
            ProjectMove::In if pos > 0 => {
                let new_parent = sibs[pos - 1].clone();
                let last_child = self
                    .store()
                    .projects_tree()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|n| n.project.parent_id.as_deref() == Some(new_parent.as_str()))
                    .map(|n| n.project.id)
                    .last();
                self.place_project(id, Some(&new_parent), last_child.as_deref());
            }
            ProjectMove::Out => {
                let Some(parent) = parent else { return };
                let grand = self
                    .store()
                    .project(&parent)
                    .ok()
                    .flatten()
                    .and_then(|p| p.parent_id);
                self.place_project(id, grand.as_deref(), Some(&parent));
            }
            _ => {}
        }
    }

    /// "Move to..." for a project: a picker of possible parents (4.1).
    fn project_move_picker(&self, id: &str) {
        let choices = self.parent_choices(Some(id));
        let current = self
            .store()
            .project(id)
            .ok()
            .flatten()
            .and_then(|p| p.parent_id);
        let options: Vec<crate::pickers::MoveOption> = choices
            .into_iter()
            .map(|(pid, name)| crate::pickers::MoveOption {
                current: pid == current,
                project_id: pid.unwrap_or_default(),
                section_id: None,
                name,
                color: "charcoal".into(),
                depth: 0,
            })
            .collect();
        let weak = self.downgrade();
        let id_c = id.to_owned();
        let pop = crate::pickers::move_picker(options, move |o| {
            if let Some(w) = weak.upgrade() {
                let parent = (!o.project_id.is_empty()).then_some(o.project_id.as_str());
                // Last among the new siblings.
                let last = w
                    .store()
                    .projects_tree()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|n| {
                        !n.project.inbox_project
                            && n.project.parent_id.as_deref() == parent
                            && n.project.id != id_c
                    })
                    .map(|n| n.project.id)
                    .last();
                w.place_project(&id_c, parent, last.as_deref());
            }
        });
        let Some(sb) = self.imp().sidebar.get() else {
            return;
        };
        pop.set_position(gtk::PositionType::Right);
        let rect = sb.row_rect(&ViewId::Project(id.to_owned()));
        crate::pickers::present(&pop, sb.list(), rect);
    }

    /// Follow a temp id the server replaced in the open view (a just-created project).
    pub(crate) fn remap_view(&self, id_map: &[(String, String)]) {
        let imp = self.imp();
        let view = imp.view.borrow().clone();
        let new = match &view {
            Some(ViewId::Project(id)) => id_map
                .iter()
                .find(|(t, _)| t == id)
                .map(|(_, r)| ViewId::Project(r.clone())),
            Some(ViewId::Label(id)) => id_map
                .iter()
                .find(|(t, _)| t == id)
                .map(|(_, r)| ViewId::Label(r.clone())),
            Some(ViewId::Filter(id)) => id_map
                .iter()
                .find(|(t, _)| t == id)
                .map(|(_, r)| ViewId::Filter(r.clone())),
            _ => None,
        };
        if let Some(v) = new {
            imp.view.replace(Some(v.clone()));
            self.save_setting("last-view", &v.to_setting());
            if let Some(sb) = imp.sidebar.get() {
                sb.select(&v);
            }
            self.update_view_menu();
        }
        let mut ui = imp.ui.borrow_mut();
        if let Some(e) = ui.section_edit.as_mut()
            && let Some((_, r)) = id_map.iter().find(|(t, _)| *t == e.project_id)
        {
            e.project_id = r.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_hints() {
        assert!(query_hint("").starts_with("Examples"));
        assert_eq!(query_hint("today | p1"), "Runs on this computer.");
        assert!(query_hint("due before: tomorrow").starts_with("Part of this query"));
    }

    #[test]
    fn every_color_has_a_name() {
        for c in COLORS {
            assert!(!color_name(c).is_empty());
        }
        assert_eq!(color_name("unknown"), "Charcoal");
    }
}
