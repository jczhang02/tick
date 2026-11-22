//! The quick add dialog (`DESIGN.md` 4.11): free text that the server parses, with local
//! highlighting of `#project`, `@label`, `/section` and `p1`-`p4` tokens.

use std::cell::{Cell, RefCell};

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;
use gtk::pango;
use tick_core::highlight::{self, TokenKind};

use crate::i18n::{fill, gettext};
use crate::rows::color_class;

/// Called with the text and whether the dialog stays open for another task (Ctrl+Enter).
type SubmitFn = Box<dyn Fn(String, bool)>;

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/dev/chengrui/Tick/ui/quick_add.ui")]
    pub struct TickQuickAdd {
        #[template_child]
        pub entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub chips: TemplateChild<adw::WrapBox>,
        #[template_child]
        pub hint: TemplateChild<gtk::Label>,
        #[template_child]
        pub cancel_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub add_button: TemplateChild<gtk::Button>,
        /// Known projects (name, API color) and label names, for highlighting and chips.
        pub projects: RefCell<Vec<(String, String)>>,
        pub labels: RefCell<Vec<String>>,
        pub offline: Cell<bool>,
        pub submit: RefCell<Option<SubmitFn>>,
        pub submitted: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TickQuickAdd {
        const NAME: &'static str = "TickQuickAdd";
        type Type = super::TickQuickAdd;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for TickQuickAdd {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            let weak = obj.downgrade();
            self.entry.connect_changed(move |_| {
                if let Some(d) = weak.upgrade() {
                    d.refresh();
                }
            });
            let weak = obj.downgrade();
            self.entry.connect_activate(move |_| {
                if let Some(d) = weak.upgrade() {
                    d.submit(false);
                }
            });
            let keys = gtk::EventControllerKey::new();
            let weak = obj.downgrade();
            keys.connect_key_pressed(move |_, key, _, state| {
                let enter = matches!(key, gtk::gdk::Key::Return | gtk::gdk::Key::KP_Enter);
                if enter && state.contains(gtk::gdk::ModifierType::CONTROL_MASK) {
                    if let Some(d) = weak.upgrade() {
                        d.submit(true);
                    }
                    return glib::Propagation::Stop;
                }
                glib::Propagation::Proceed
            });
            self.entry.add_controller(keys);
            let weak = obj.downgrade();
            self.add_button.connect_clicked(move |_| {
                if let Some(d) = weak.upgrade() {
                    d.submit(false);
                }
            });
            let weak = obj.downgrade();
            self.cancel_button.connect_clicked(move |_| {
                if let Some(d) = weak.upgrade() {
                    d.close();
                }
            });
            let style = adw::StyleManager::default();
            let weak = obj.downgrade();
            style.connect_dark_notify(move |_| {
                if let Some(d) = weak.upgrade() {
                    d.refresh();
                }
            });
            let weak = obj.downgrade();
            style.connect_accent_color_notify(move |_| {
                if let Some(d) = weak.upgrade() {
                    d.refresh();
                }
            });
            obj.set_focus(Some(&*self.entry));
            obj.refresh();
        }
    }

    impl WidgetImpl for TickQuickAdd {}
    impl AdwDialogImpl for TickQuickAdd {}
}

glib::wrapper! {
    pub struct TickQuickAdd(ObjectSubclass<imp::TickQuickAdd>)
        @extends adw::Dialog, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::ShortcutManager;
}

/// Priority colors per theme (2.4.3); p4 is the concrete `--tick-p4` over the dialog.
fn priority_rgb(p: u8, dark: bool) -> (u16, u16, u16) {
    let hex = match (p, dark) {
        (1, false) => 0xbb2f28,
        (2, false) => 0xc36400,
        (3, false) => 0x246fe0,
        (_, false) => 0x6b6b6e,
        (1, true) => 0xff7f70,
        (2, true) => 0xeb8909,
        (3, true) => 0x55a0ff,
        (_, true) => 0xc0c0c1,
    };
    let c = |shift: u32| (((hex >> shift) & 0xff) as u16) * 257;
    (c(16), c(8), c(0))
}

impl TickQuickAdd {
    pub fn new(projects: Vec<(String, String)>, labels: Vec<String>, offline: bool) -> Self {
        let d: TickQuickAdd = glib::Object::new();
        d.imp().projects.replace(projects);
        d.imp().labels.replace(labels);
        d.imp().offline.set(offline);
        d.refresh();
        d
    }

    pub fn connect_submit<F: Fn(String, bool) + 'static>(&self, f: F) {
        self.imp().submit.replace(Some(Box::new(f)));
    }

    /// Prefill (a project, section or date) or a kept draft; the cursor goes to the end.
    pub fn set_text(&self, text: &str) {
        let entry = &self.imp().entry;
        entry.set_text(text);
        entry.set_position(-1);
    }

    pub fn text(&self) -> String {
        self.imp().entry.text().to_string()
    }

    /// Whether the dialog closed through a submit (the draft is not kept then).
    pub fn was_submitted(&self) -> bool {
        self.imp().submitted.get()
    }

    pub fn set_offline(&self, offline: bool) {
        self.imp().offline.set(offline);
        self.refresh();
    }

    fn submit(&self, add_another: bool) {
        let text = self.text().trim().to_owned();
        if text.is_empty() {
            return;
        }
        if let Some(f) = self.imp().submit.borrow().as_ref() {
            f(text, add_another);
        }
        if add_another {
            self.imp().entry.set_text("");
        } else {
            self.imp().submitted.set(true);
            self.close();
        }
    }

    fn refresh(&self) {
        let imp = self.imp();
        let text = imp.entry.text().to_string();
        imp.add_button.set_sensitive(!text.trim().is_empty());
        let projects = imp.projects.borrow();
        let labels = imp.labels.borrow();
        let project_names: Vec<&str> = projects.iter().map(|(n, _)| n.as_str()).collect();
        let label_names: Vec<&str> = labels.iter().map(String::as_str).collect();
        let spans = highlight::highlight_with_names(&text, &project_names, &label_names);

        // Background tints with Pango attributes (pitfall 9: concrete colors).
        let style = adw::StyleManager::default();
        let dark = style.is_dark();
        let accent = style.accent_color_rgba();
        let accent = (
            (accent.red() * 65535.0) as u16,
            (accent.green() * 65535.0) as u16,
            (accent.blue() * 65535.0) as u16,
        );
        let attrs = pango::AttrList::new();
        for s in &spans {
            let (r, g, b) = match s.kind {
                TokenKind::Priority(p) => priority_rgb(p, dark),
                _ => accent,
            };
            let mut bg = pango::AttrColor::new_background(r, g, b);
            bg.set_start_index(s.start as u32);
            bg.set_end_index(s.end as u32);
            attrs.insert(bg);
            let mut alpha = pango::AttrInt::new_background_alpha((0.16 * 65535.0) as u16);
            alpha.set_start_index(s.start as u32);
            alpha.set_end_index(s.end as u32);
            attrs.insert(alpha);
        }
        imp.entry.set_attributes(&attrs);

        // Chips (display only, hidden from AT).
        while let Some(c) = imp.chips.first_child() {
            imp.chips.remove(&c);
        }
        let mut unknown_project = None;
        for s in &spans {
            let chip = gtk::Box::builder()
                .spacing(6)
                .css_classes(["tick-token-chip", "caption"])
                .accessible_role(gtk::AccessibleRole::Presentation)
                .build();
            let mut name = s.value.clone();
            match s.kind {
                TokenKind::Project => {
                    match projects
                        .iter()
                        .find(|(n, _)| n.eq_ignore_ascii_case(&s.value))
                    {
                        Some((n, color)) => {
                            name = n.clone();
                            chip.append(
                                &gtk::Box::builder()
                                    .valign(gtk::Align::Center)
                                    .css_classes(["tick-dot-8".to_owned(), color_class(color)])
                                    .build(),
                            );
                        }
                        None => {
                            unknown_project.get_or_insert_with(|| s.value.clone());
                            chip.append(
                                &gtk::Image::builder()
                                    .icon_name("dialog-warning-symbolic")
                                    .pixel_size(12)
                                    .css_classes(["tick-warning"])
                                    .build(),
                            );
                        }
                    }
                }
                TokenKind::Label => {
                    chip.append(
                        &gtk::Image::builder()
                            .icon_name("tick-label-symbolic")
                            .pixel_size(12)
                            .build(),
                    );
                }
                TokenKind::Section => {
                    name = format!("/{name}");
                }
                TokenKind::Priority(p) => {
                    chip.append(
                        &gtk::Box::builder()
                            .valign(gtk::Align::Center)
                            .css_classes(["tick-chip-ring".to_owned(), format!("p{p}")])
                            .build(),
                    );
                    name = fill(&gettext("Priority {n}"), &[("n", &p.to_string())]);
                }
            }
            chip.append(&gtk::Label::new(Some(&name)));
            imp.chips.append(&chip);
        }
        imp.chips.set_visible(!spans.is_empty());

        let hint = if let Some(p) = unknown_project {
            fill(
                &gettext("No project named \"{name}\". The task goes to the Inbox."),
                &[("name", &p)],
            )
        } else if imp.offline.get() {
            gettext("Offline. The task will be added when you're back online.")
        } else {
            gettext("Enter to add, Esc to close")
        };
        imp.hint.set_label(&hint);
    }
}
