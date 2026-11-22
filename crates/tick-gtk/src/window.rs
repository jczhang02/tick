//! The main window: layout and breakpoints (`DESIGN.md` 3), the sync engine wiring, view
//! switching, completing with Undo, quick add and the keyboard shortcuts.

use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;
use std::time::{Duration, Instant};

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};
use tick_core::engine::ConnState;
use tick_core::store::wall_ms;
use tick_core::{Changes, Command, EngineEvent, EngineHandle, Kind, Store, SyncStatus};

use crate::i18n::{fill, gettext, ncount};
use crate::login::{LoginError, TickLoginPage};
use crate::quick_add::TickQuickAdd;
use crate::row_object::{TickRow, update_store};
use crate::rows::{self, Row, UiState, ViewId};
use crate::sidebar::{Section, Sidebar, SidebarEvent};
use crate::sync_status::{SyncIndicator, SyncView};
use crate::task_list::{self, RenderCtx, RowAction};
use crate::test_mode;

/// Delay between checking a task and its fade-out (5.1 `hold`).
const HOLD: Duration = Duration::from_millis(800);
/// Completed row fade-out (5.1 `exit`).
const EXIT: Duration = Duration::from_millis(250);
/// How long a pending `g` waits for the second key of a "go to" shortcut.
const SEQUENCE_TIMEOUT: Duration = Duration::from_millis(1500);
/// How long a change queued while online may wait before the sync indicator shows it.
const PENDING_GRACE: Duration = Duration::from_secs(2);
/// Longest time a completion waits for its "Undo" toast to go away before it is sent
/// anyway (libadwaita pauses a toast's timeout while the pointer is over it). After that,
/// Undo reopens the task with `item_uncomplete`.
const UNDO_HOLD: Duration = Duration::from_secs(60);

/// [`UNDO_HOLD`], or `TICK_TEST_UNDO_HOLD_MS` in test mode (E2E: Undo after the close left).
fn undo_hold() -> Duration {
    test_mode::enabled()
        .then(|| std::env::var("TICK_TEST_UNDO_HOLD_MS").ok()?.parse().ok())
        .flatten()
        .map_or(UNDO_HOLD, Duration::from_millis)
}

/// Completions the visible "Undo" toast can take back.
pub struct UndoGroup {
    toast: adw::Toast,
    commands: Vec<UndoEntry>,
}

/// One completion of an [`UndoGroup`].
pub struct UndoEntry {
    /// The task id (remapped when the server replaces a temp id).
    id: String,
    /// The queued `item_close`.
    uuid: String,
    recurring: bool,
}

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(resource = "/dev/chengrui/Tick/ui/window.ui")]
    #[properties(wrapper_type = super::TickWindow)]
    pub struct TickWindow {
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub split_view: TemplateChild<adw::NavigationSplitView>,
        #[template_child]
        pub sidebar_add_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub sidebar_list: TemplateChild<gtk::ListView>,
        #[template_child]
        pub content_page: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub detail_split: TemplateChild<adw::OverlaySplitView>,
        #[template_child]
        pub window_title: TemplateChild<adw::WindowTitle>,
        #[template_child]
        pub sync_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub list_scroller: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub task_list: TemplateChild<gtk::ListView>,
        #[template_child]
        pub detail_back_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub detail_close_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub login_page: TemplateChild<TickLoginPage>,

        /// B2 and B3 (3.3): list rows bind to it (setters cannot reach factory widgets).
        #[property(get, set = Self::set_narrow)]
        pub narrow: Cell<bool>,

        pub store: RefCell<Option<Store>>,
        pub engine: RefCell<Option<EngineHandle>>,
        pub view: RefCell<Option<ViewId>>,
        pub ui: RefCell<UiState>,
        pub content: RefCell<Option<rows::Content>>,
        pub rows: OnceCell<gio::ListStore>,
        pub selection: OnceCell<gtk::SingleSelection>,
        pub sidebar: OnceCell<Rc<Sidebar>>,
        pub sync: OnceCell<Rc<SyncIndicator>>,
        pub narrow_cell: Rc<Cell<bool>>,
        pub task_bound: Rc<Cell<bool>>,
        pub header_height: Rc<Cell<i32>>,
        pub status: RefCell<SyncStatus>,
        pub known_failed: Cell<Option<usize>>,
        pub last_toast: RefCell<Option<adw::Toast>>,
        pub undo: RefCell<Option<UndoGroup>>,
        pub login_token: RefCell<Option<String>>,
        pub g_pending: Cell<Option<Instant>>,
        pub draft: RefCell<String>,
        pub title_shown: Cell<bool>,
        pub title_anim: RefCell<Option<adw::TimedAnimation>>,
        /// Since when changes are queued while online (the indicator waits, see
        /// `refresh_sync`).
        pub pending_since: Cell<Option<Instant>>,
        /// The previously focused widget (see `setup_focus_guard`).
        pub last_focus: RefCell<Option<glib::WeakRef<gtk::Widget>>>,
    }

    impl TickWindow {
        fn set_narrow(&self, narrow: bool) {
            if self.narrow.replace(narrow) == narrow {
                return;
            }
            self.narrow_cell.set(narrow);
            // The view header's top margin depends on it.
            if let Some(header) = self
                .rows
                .get()
                .and_then(|s| s.item(0))
                .and_downcast::<TickRow>()
            {
                header.touch();
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TickWindow {
        const NAME: &'static str = "TickWindow";
        type Type = super::TickWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            TickLoginPage::ensure_type();
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for TickWindow {}
    impl WidgetImpl for TickWindow {}
    impl WindowImpl for TickWindow {}
    impl ApplicationWindowImpl for TickWindow {}
    impl AdwApplicationWindowImpl for TickWindow {}
}

glib::wrapper! {
    pub struct TickWindow(ObjectSubclass<imp::TickWindow>)
        @extends adw::ApplicationWindow, gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

/// Where the window starts.
pub enum Start {
    /// Show the cached data; the token is known or still loading.
    Main,
    /// No cached account: show the login page (a keyring token may still switch to main).
    Login,
}

impl TickWindow {
    /// Builds the window around the already opened cache and renders it, without any
    /// network call (`docs/architecture.md`, startup).
    pub fn new(app: &adw::Application, store: Store, start: Start) -> Self {
        let win: TickWindow = glib::Object::builder().property("application", app).build();
        win.imp().store.replace(Some(store));
        win.setup_breakpoints();
        win.setup_lists();
        win.setup_actions();
        win.setup_keys();
        win.setup_focus_guard();
        win.setup_login();
        win.setup_title_handoff();
        win.load_settings();
        match start {
            Start::Main => win.show_main(),
            Start::Login => win.show_login(),
        }
        let initial = win
            .setting("last-view")
            .and_then(|s| ViewId::from_setting(&s))
            .unwrap_or(ViewId::Today);
        win.open_view(initial, false);
        win.refresh_sidebar();
        win.refresh_sync();
        win
    }

    fn store(&self) -> std::cell::RefMut<'_, Store> {
        std::cell::RefMut::map(self.imp().store.borrow_mut(), |s| {
            s.as_mut().expect("store")
        })
    }

    fn setting(&self, key: &str) -> Option<String> {
        self.store().setting(key).ok().flatten()
    }

    fn save_setting(&self, key: &str, value: &str) {
        if let Err(e) = self.store().set_setting(key, value) {
            eprintln!("tick: saving setting {key}: {e}");
        }
    }

    /// True when the first visible task row has been built (or the view has none).
    pub fn data_frame_ready(&self) -> bool {
        let has_tasks = self
            .imp()
            .content
            .borrow()
            .as_ref()
            .is_some_and(|c| c.rows.iter().any(|r| matches!(r, Row::Task(_))));
        let showing_main = self.imp().stack.visible_child_name().as_deref() == Some("main");
        // An unmapped list (narrow layout, sidebar page shown) has no rows to wait for.
        !showing_main
            || !has_tasks
            || !self.imp().task_list.is_mapped()
            || self.imp().task_bound.get()
    }

    // ---- setup ----

    fn setup_breakpoints(&self) {
        let imp = self.imp();
        let parse = |c: &str| adw::BreakpointCondition::parse(c).expect("breakpoint condition");
        let t = true.to_value();
        let f = false.to_value();
        // Setters do not stack (7.4 pitfall 19): each breakpoint carries its full set.
        let b1 = adw::Breakpoint::new(parse("max-width: 980sp"));
        b1.add_setter(&*imp.detail_split, "collapsed", Some(&t));
        let b2 = adw::Breakpoint::new(parse("max-width: 720sp"));
        let b3 = adw::Breakpoint::new(parse("max-width: 500sp"));
        for b in [&b2, &b3] {
            b.add_setter(&*imp.detail_split, "collapsed", Some(&t));
            b.add_setter(&*imp.split_view, "collapsed", Some(&t));
            b.add_setter(&*imp.sidebar_add_button, "visible", Some(&t));
            b.add_setter(self, "narrow", Some(&t));
        }
        b3.add_setter(
            &*imp.detail_split,
            "min-sidebar-width",
            Some(&0.0f64.to_value()),
        );
        b3.add_setter(
            &*imp.detail_split,
            "max-sidebar-width",
            Some(&500.0f64.to_value()),
        );
        b3.add_setter(
            &*imp.detail_split,
            "sidebar-width-fraction",
            Some(&1.0f64.to_value()),
        );
        b3.add_setter(&*imp.detail_back_button, "visible", Some(&t));
        b3.add_setter(&*imp.detail_close_button, "visible", Some(&f));
        self.add_breakpoint(b1);
        self.add_breakpoint(b2);
        self.add_breakpoint(b3);
    }

    fn setup_lists(&self) {
        let imp = self.imp();
        let weak = self.downgrade();
        let dispatch: task_list::Dispatch = Rc::new(move |action| {
            // Act after the emitting widget's signal returns: acting may rebuild the row.
            let weak = weak.clone();
            glib::idle_add_local_once(move || {
                if let Some(w) = weak.upgrade() {
                    w.on_row_action(action);
                }
            });
        });
        let ctx = RenderCtx {
            dispatch,
            narrow: imp.narrow_cell.clone(),
            task_bound: imp.task_bound.clone(),
            header_height: imp.header_height.clone(),
        };
        let (store, selection) = task_list::setup(&imp.task_list, ctx);
        let _ = imp.rows.set(store);
        let _ = imp.selection.set(selection);
        let weak = self.downgrade();
        imp.task_list.connect_activate(move |_, pos| {
            if let Some(w) = weak.upgrade() {
                w.on_row_activated(pos);
            }
        });
        // Up/Down (and J/K) move between interactive rows, skipping the view, day and month
        // headers; GtkListView would stop at a row that cannot take focus.
        let keys = gtk::EventControllerKey::new();
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = self.downgrade();
        keys.connect_key_pressed(move |_, key, _, mods| {
            use gtk::gdk::{Key, ModifierType};
            let Some(w) = weak.upgrade() else {
                return glib::Propagation::Proceed;
            };
            if mods.intersects(
                ModifierType::CONTROL_MASK | ModifierType::ALT_MASK | ModifierType::SHIFT_MASK,
            ) {
                return glib::Propagation::Proceed;
            }
            let down = match key {
                Key::Down | Key::KP_Down | Key::j => true,
                Key::Up | Key::KP_Up | Key::k => false,
                _ => return glib::Propagation::Proceed,
            };
            if w.move_focus(down) {
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        imp.task_list.add_controller(keys);

        let weak = self.downgrade();
        let sidebar = Sidebar::new(
            imp.sidebar_list.get(),
            Rc::new(move |event| {
                let Some(w) = weak.upgrade() else { return };
                match event {
                    SidebarEvent::Open { view, push } => w.open_view(view, push),
                    SidebarEvent::SectionToggled => w.save_sidebar_collapsed(),
                }
            }),
        );
        let _ = imp.sidebar.set(sidebar);

        let weak = self.downgrade();
        let weak2 = self.downgrade();
        let sync = SyncIndicator::new(
            imp.sync_button.get(),
            move || {
                if let Some(w) = weak.upgrade()
                    && let Some(e) = w.imp().engine.borrow().as_ref()
                {
                    e.sync_now();
                }
            },
            move || {
                if let Some(w) = weak2.upgrade() {
                    w.dismiss_errors();
                }
            },
        );
        let _ = imp.sync.set(sync);
    }

    fn setup_actions(&self) {
        let quick_add = gio::ActionEntry::builder("quick-add")
            .activate(|w: &TickWindow, _, _| w.open_quick_add(None))
            .build();
        let undo = gio::ActionEntry::builder("undo")
            .activate(|w: &TickWindow, _, _| w.undo())
            .build();
        let close_task = gio::ActionEntry::builder("close-task")
            .activate(|w: &TickWindow, _, _| w.imp().detail_split.set_show_sidebar(false))
            .build();
        self.add_action_entries([quick_add, undo, close_task]);

        // Sync cadence (SPEC): on focus, and foreground vs background by visibility.
        self.connect_is_active_notify(|w| {
            if w.is_active()
                && let Some(e) = w.imp().engine.borrow().as_ref()
            {
                e.set_foreground(true);
                e.sync_now();
            }
        });
        self.connect_visible_notify(|w| {
            if let Some(e) = w.imp().engine.borrow().as_ref() {
                e.set_foreground(w.is_visible());
            }
        });
    }

    fn setup_keys(&self) {
        // Single-key shortcuts (Todoist web) in the bubble phase: a focused text field
        // consumes the keys first, so they never fire while typing.
        let keys = gtk::EventControllerKey::new();
        let weak = self.downgrade();
        keys.connect_key_pressed(move |_, key, _, mods| {
            let Some(w) = weak.upgrade() else {
                return glib::Propagation::Proceed;
            };
            if w.on_key(key, mods) {
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        self.add_controller(keys);
    }

    /// When the focused widget goes away (a completed row leaves the list), GTK hands focus
    /// to the next focusable widget, often the toast's button, and a toast with focus
    /// never times out. Move such focus back to the list. Tab or a click into the toast
    /// (the previous widget still shown) is left alone.
    fn setup_focus_guard(&self) {
        self.connect_focus_widget_notify(|w| {
            let new = gtk::prelude::GtkWindowExt::focus(w);
            let old = w
                .imp()
                .last_focus
                .replace(new.as_ref().map(ObjectExt::downgrade))
                .and_then(|o| o.upgrade());
            let Some(new) = new else { return };
            if !in_toast(&new) {
                return;
            }
            let vanished = old.is_none_or(|o| !o.is_mapped() || o.root().is_none());
            if !vanished {
                return;
            }
            let weak = w.downgrade();
            glib::idle_add_local_once(move || {
                if let Some(w) = weak.upgrade() {
                    let selected = w.selected_position();
                    w.keep_focus_in_list(selected.unwrap_or(0), false);
                }
            });
        });
    }

    fn setup_login(&self) {
        let weak = self.downgrade();
        self.imp().login_page.connect_submit(move |_, token| {
            if let Some(w) = weak.upgrade() {
                w.try_login(token);
            }
        });
    }

    fn setup_title_handoff(&self) {
        let imp = self.imp();
        let weak = self.downgrade();
        imp.list_scroller
            .vadjustment()
            .connect_value_changed(move |adj| {
                let Some(w) = weak.upgrade() else { return };
                let threshold = f64::from(w.imp().header_height.get().max(80));
                w.set_title_shown(adj.value() > threshold);
            });
    }

    /// Header bar title hand-off (4.2): fade the small title in once the large one is gone.
    fn set_title_shown(&self, shown: bool) {
        let imp = self.imp();
        if imp.title_shown.replace(shown) == shown {
            return;
        }
        if let Some(a) = imp.title_anim.take() {
            a.pause();
        }
        let title = imp.window_title.get();
        let to = if shown { 1.0 } else { 0.0 };
        if test_mode::enabled() {
            title.set_opacity(to);
            return;
        }
        let target = adw::PropertyAnimationTarget::new(&title, "opacity");
        let anim = adw::TimedAnimation::new(&title, title.opacity(), to, 150, target);
        anim.set_easing(adw::Easing::EaseOutCubic);
        anim.set_follow_enable_animations_setting(false);
        anim.play();
        imp.title_anim.replace(Some(anim));
    }

    fn load_settings(&self) {
        if let Some(s) = self.setting("sidebar-collapsed") {
            let set: HashSet<Section> = s.split(',').filter_map(Section::from_key).collect();
            if let Some(sb) = self.imp().sidebar.get() {
                sb.collapsed.replace(set);
            }
        }
        if let Some(s) = self.setting("collapsed-sections") {
            self.imp().ui.borrow_mut().collapsed_sections = s
                .split(',')
                .filter(|x| !x.is_empty())
                .map(str::to_owned)
                .collect();
        }
    }

    fn save_sidebar_collapsed(&self) {
        let Some(sb) = self.imp().sidebar.get() else {
            return;
        };
        let value = sb
            .collapsed
            .borrow()
            .iter()
            .map(|s| s.key())
            .collect::<Vec<_>>()
            .join(",");
        self.save_setting("sidebar-collapsed", &value);
    }

    // ---- pages ----

    fn show_main(&self) {
        self.imp().stack.set_visible_child_name("main");
    }

    fn show_login(&self) {
        let imp = self.imp();
        imp.stack.set_visible_child_name("login");
        imp.login_page.focus_entry();
    }

    /// The app quits: completions held for an "Undo" toast go out on the next sync instead
    /// of waiting out the hold.
    pub fn release_held(&self) {
        if let Err(e) = self.store().release_deferred() {
            eprintln!("tick: releasing held changes: {e}");
        }
    }

    /// The token is known: start syncing (after the first frame, see `main.rs`).
    pub fn start_engine(&self, token: String) {
        let imp = self.imp();
        if let Some(e) = imp.engine.borrow().as_ref() {
            e.set_token(&token);
            return;
        }
        // The engine opens its own connection to the same file. An in-memory cache (no file
        // could be opened) cannot be shared: syncing into another database would never show
        // up here, so do not sync at all.
        let path = imp
            .store
            .borrow()
            .as_ref()
            .and_then(|s| s.path().map(std::path::Path::to_path_buf));
        let Some(path) = path else {
            eprintln!("tick: no cache file, not syncing");
            return;
        };
        let config = tick_core::EngineConfig::from_env(token, path);
        let (handle, events) = match tick_core::Engine::spawn(config) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("tick: cannot start the sync engine: {e}");
                return;
            }
        };
        handle.set_foreground(self.is_visible());
        imp.engine.replace(Some(handle));
        let weak = self.downgrade();
        glib::spawn_future_local(async move {
            while let Ok(ev) = events.recv().await {
                let Some(w) = weak.upgrade() else { break };
                w.on_engine_event(ev);
            }
        });
        // A filter view opened from the cache may need its server part now.
        self.fetch_filter_if_needed();
    }

    /// The token lookup finished (keyring, after the first frame).
    pub fn token_loaded(&self, token: Option<String>) {
        match token {
            Some(t) => {
                self.show_main();
                self.start_engine(t);
            }
            None => self.show_login(),
        }
    }

    fn try_login(&self, token: String) {
        self.imp().login_token.replace(Some(token.clone()));
        self.start_engine(token);
    }

    // ---- engine events ----

    fn on_engine_event(&self, ev: EngineEvent) {
        match ev {
            EngineEvent::Changed(c) => self.on_changes(&c),
            EngineEvent::Status(s) => self.on_status(s),
            EngineEvent::FilterLoaded { .. } => {
                self.imp().ui.borrow_mut().filter_loading.clear();
                self.refresh_rows();
                self.refresh_sidebar();
            }
            EngineEvent::Error(f) => {
                if let tick_core::FailedRequest::Filter(q) = &f.request {
                    {
                        let mut ui = self.imp().ui.borrow_mut();
                        ui.filter_loading.remove(q);
                        if f.rejected {
                            ui.filter_rejected.insert(q.clone());
                        }
                    }
                    self.refresh_rows();
                    let has_cache = self.imp().content.borrow().as_ref().is_some_and(|c| {
                        c.rows.iter().any(|r| matches!(r, Row::SourceLine { .. }))
                    });
                    if !f.rejected && !f.offline && has_cache {
                        let toast = adw::Toast::builder()
                            .title(gettext("Couldn't update this filter"))
                            .button_label(gettext("Retry"))
                            .timeout(8)
                            .build();
                        let weak = self.downgrade();
                        toast.connect_button_clicked(move |_| {
                            if let Some(w) = weak.upgrade() {
                                w.fetch_filter_if_needed();
                            }
                        });
                        self.show_toast(&toast);
                    }
                }
            }
            EngineEvent::CompletedLoaded { .. } => {}
        }
    }

    fn on_changes(&self, c: &Changes) {
        self.remap_ids(&c.id_map);
        let only_queue = !c.full && c.kinds.iter().all(|k| *k == Kind::Queue);
        if !only_queue {
            self.refresh_rows_with(&c.id_map);
            // Sidebar names and counts (filter counts evaluate every saved filter) depend
            // only on these kinds; comments, reminders and completed history do not.
            const SIDEBAR: [Kind; 7] = [
                Kind::Item,
                Kind::Project,
                Kind::Section,
                Kind::Label,
                Kind::Filter,
                Kind::Collaborator,
                Kind::User,
            ];
            if c.full || c.kinds.iter().any(|k| SIDEBAR.contains(k)) {
                self.refresh_sidebar();
            }
        }
        self.refresh_sync();
    }

    fn on_status(&self, s: SyncStatus) {
        let imp = self.imp();
        let was_offline = imp.ui.borrow().offline;
        // A sync attempt (Syncing) does not end the offline state: only an answer does.
        // Otherwise the indicator (and its open popover) would vanish on every retry.
        let offline = match s.state {
            ConnState::Offline => true,
            ConnState::Online | ConnState::Unauthorized => false,
            ConnState::Idle | ConnState::Syncing => was_offline,
        };
        imp.ui.borrow_mut().offline = offline;
        imp.status.replace(s.clone());

        // A login attempt (or a revoked token) is settled by the first answered sync.
        let on_login = imp.stack.visible_child_name().as_deref() == Some("login");
        if on_login && imp.login_token.borrow().is_some() {
            match s.state {
                ConnState::Online => self.login_succeeded(),
                ConnState::Unauthorized => imp.login_page.show_error(LoginError::Rejected),
                ConnState::Offline => imp.login_page.show_error(LoginError::Unreachable),
                ConnState::Idle | ConnState::Syncing => {}
            }
        } else if s.state == ConnState::Unauthorized && !on_login {
            imp.login_page.show_error(LoginError::Rejected);
            self.show_login();
        }

        if was_offline != offline {
            self.refresh_rows();
            if !offline {
                self.fetch_filter_if_needed();
            }
        }
        self.refresh_sync();
        self.toast_new_failures(s.failed);
    }

    fn login_succeeded(&self) {
        let imp = self.imp();
        let Some(token) = imp.login_token.take() else {
            return;
        };
        if !test_mode::enabled() {
            // The keyring may block (unlock prompt): never on the main thread.
            let save = gio::spawn_blocking(move || tick_core::token::store_blocking(&token));
            glib::spawn_future_local(async move {
                match save.await {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => eprintln!("tick: could not store the token: {e}"),
                    Err(_) => eprintln!("tick: the keyring worker panicked"),
                }
            });
        }
        imp.login_page.reset();
        self.show_main();
        self.refresh();
    }

    fn toast_new_failures(&self, failed: usize) {
        let imp = self.imp();
        let known = imp.known_failed.replace(Some(failed));
        let Some(known) = known else {
            return;
        };
        if failed <= known {
            return;
        }
        let newest = self
            .store()
            .failed_commands()
            .ok()
            .and_then(|f| f.into_iter().next());
        let Some(cmd) = newest else { return };
        let title = self.subject_of(&cmd.command).unwrap_or_default();
        let toast = adw::Toast::builder()
            .title(fill(
                &gettext("Couldn't save \"{title}\""),
                &[("title", &title)],
            ))
            .button_label(gettext("Details"))
            .timeout(8)
            .build();
        let weak = self.downgrade();
        toast.connect_button_clicked(move |_| {
            if let Some(w) = weak.upgrade()
                && let Some(s) = w.imp().sync.get()
            {
                s.popup();
            }
        });
        self.show_toast(&toast);
    }

    // ---- views ----

    pub fn open_view(&self, view: ViewId, push: bool) {
        let imp = self.imp();
        let same = imp.view.borrow().as_ref() == Some(&view);
        if !same {
            imp.view.replace(Some(view.clone()));
            self.save_setting("last-view", &view.to_setting());
            imp.ui.borrow_mut().filter_rejected.clear();
            if let Some(sel) = imp.selection.get() {
                sel.set_selected(gtk::INVALID_LIST_POSITION);
            }
            // A different view replaces every row (and starts at the top).
            if let Some(store) = imp.rows.get() {
                store.remove_all();
            }
            imp.task_bound.set(false);
            imp.list_scroller.vadjustment().set_value(0.0);
            self.set_title_shown(false);
            self.refresh_rows();
            self.fetch_filter_if_needed();
        }
        if let Some(sb) = imp.sidebar.get() {
            sb.select(&view);
        }
        if push && imp.split_view.is_collapsed() {
            imp.split_view.set_show_content(true);
        }
    }

    fn current_view(&self) -> ViewId {
        self.imp().view.borrow().clone().unwrap_or(ViewId::Today)
    }

    fn fetch_filter_if_needed(&self) {
        let imp = self.imp();
        let query = imp
            .content
            .borrow()
            .as_ref()
            .and_then(|c| c.server_query.clone());
        let Some(q) = query else { return };
        let engine = imp.engine.borrow();
        let Some(e) = engine.as_ref() else { return };
        if imp.ui.borrow().offline {
            return;
        }
        imp.ui.borrow_mut().filter_loading.insert(q.clone());
        e.fetch_filter(&q);
        drop(engine);
        self.refresh_rows();
    }

    fn refresh(&self) {
        self.refresh_rows();
        self.refresh_sidebar();
        self.refresh_sync();
    }

    fn refresh_rows(&self) {
        self.refresh_rows_with(&[]);
    }

    fn selected_position(&self) -> Option<u32> {
        let sel = self.imp().selection.get()?;
        let pos = sel.selected();
        (pos != gtk::INVALID_LIST_POSITION).then_some(pos)
    }

    fn selected_task(&self) -> Option<String> {
        self.imp()
            .selection
            .get()?
            .selected_item()
            .and_downcast::<TickRow>()?
            .task_id()
    }

    fn refresh_rows_with(&self, id_map: &[(String, String)]) {
        let imp = self.imp();
        let view = self.current_view();
        self.update_failed_map();
        let built = {
            let store = imp.store.borrow();
            let Some(store) = store.as_ref() else { return };
            let ui = imp.ui.borrow();
            rows::build(store, &view, &ui)
        };
        let content = match built {
            Ok(c) => c,
            Err(e) => {
                eprintln!("tick: reading the cache: {e}");
                return;
            }
        };
        if !content.exists {
            // The project, label or filter is gone (deleted elsewhere): fall back to Today.
            if view != ViewId::Today {
                self.open_view(ViewId::Today, false);
            }
            return;
        }
        let selected = self.selected_task().map(|id| {
            id_map
                .iter()
                .find(|(t, _)| *t == id)
                .map_or(id.clone(), |(_, r)| r.clone())
        });
        imp.content_page.set_title(&content.title);
        imp.window_title.set_title(&content.title);
        if let Some(store) = imp.rows.get() {
            // Filling an empty list (a new view) with the list view detached is much faster:
            // the view then lays out once instead of reacting to the whole splice.
            let detach = store.n_items() == 0 && content.rows.len() > 64;
            if detach {
                imp.task_list.set_model(None::<&gtk::SelectionModel>);
            }
            update_store(store, content.rows.clone());
            if detach {
                imp.task_list.set_model(imp.selection.get());
            }
            if let (Some(id), Some(sel)) = (selected, imp.selection.get()) {
                let current = self.selected_task();
                if current.as_deref() != Some(id.as_str())
                    && let Some(pos) = (0..store.n_items()).find(|i| {
                        store
                            .item(*i)
                            .and_downcast::<TickRow>()
                            .and_then(|r| r.task_id())
                            .as_deref()
                            == Some(id.as_str())
                    })
                {
                    sel.set_selected(pos);
                }
            }
        }
        imp.content.replace(Some(content));
    }

    fn update_failed_map(&self) {
        let failed = self.store().failed_commands().unwrap_or_default();
        let mut map = std::collections::HashMap::new();
        for f in failed {
            let args = &f.command.args;
            let id = args
                .get("id")
                .and_then(|v| v.as_str())
                .map(str::to_owned)
                .or(f.command.temp_id.clone());
            if let Some(id) = id {
                let msg = f
                    .detail
                    .as_ref()
                    .map_or(f.error.clone(), |d| d.error.clone());
                map.entry(id).or_insert(msg);
            }
        }
        self.imp().ui.borrow_mut().failed = map;
    }

    fn refresh_sidebar(&self) {
        let imp = self.imp();
        let data = {
            let store = imp.store.borrow();
            let Some(store) = store.as_ref() else { return };
            rows::sidebar(store)
        };
        match data {
            Ok(d) => {
                if let Some(sb) = imp.sidebar.get() {
                    sb.update(d);
                }
            }
            Err(e) => eprintln!("tick: reading the sidebar: {e}"),
        }
    }

    /// A short description of a queued or failed change ("Complete "Buy milk"").
    fn describe(&self, cmd: &Command) -> String {
        let subject = self.subject_of(cmd).unwrap_or_default();
        let template = match cmd.kind.as_str() {
            "item_close" | "item_complete" => gettext("Complete \"{title}\""),
            "item_uncomplete" => gettext("Reopen \"{title}\""),
            "item_add" | tick_core::command::QUICK_ADD => gettext("Add \"{title}\""),
            "item_delete" => gettext("Delete \"{title}\""),
            "item_move" => gettext("Move \"{title}\""),
            k if k.starts_with("project_") => gettext("Change project \"{title}\""),
            k if k.starts_with("section_") => gettext("Change section \"{title}\""),
            k if k.starts_with("label_") => gettext("Change label \"{title}\""),
            k if k.starts_with("filter_") => gettext("Change filter \"{title}\""),
            _ => gettext("Change \"{title}\""),
        };
        fill(&template, &[("title", &subject)])
    }

    /// The name of what a command changes (task title, project name, quick-add text).
    fn subject_of(&self, cmd: &Command) -> Option<String> {
        let args = &cmd.args;
        let str_arg = |k: &str| args.get(k).and_then(|v| v.as_str()).map(str::to_owned);
        if cmd.kind == tick_core::command::QUICK_ADD {
            return str_arg("text");
        }
        if let Some(c) = str_arg("content").filter(|_| cmd.kind == "item_add") {
            return Some(c);
        }
        if let Some(n) = str_arg("name") {
            return Some(n);
        }
        let id = str_arg("id").or(cmd.temp_id.clone())?;
        let store = self.store();
        if cmd.kind.starts_with("item_") {
            return store.task(&id).ok().flatten().map(|t| t.content.clone());
        }
        if cmd.kind.starts_with("project_") {
            return store.project(&id).ok().flatten().map(|p| p.name);
        }
        if cmd.kind.starts_with("section_") {
            return store.section(&id).ok().flatten().map(|s| s.name);
        }
        None
    }

    fn refresh_sync(&self) {
        let imp = self.imp();
        let (pending, failed) = {
            let store = self.store();
            let now = wall_ms();
            let pending: Vec<Command> = store
                .pending_commands()
                .unwrap_or_default()
                .into_iter()
                .filter(|q| q.is_due(now))
                .map(|q| q.command)
                .collect();
            let failed = store.failed_commands().unwrap_or_default();
            (pending, failed)
        };
        let mut view = SyncView {
            offline: imp.ui.borrow().offline,
            pending: pending.iter().map(|c| self.describe(c)).collect(),
            failed: failed
                .iter()
                .map(|f| {
                    let msg = f
                        .detail
                        .as_ref()
                        .map_or(f.error.clone(), |d| d.error.clone());
                    (self.describe(&f.command), msg)
                })
                .collect(),
        };
        // Online, a queued change normally leaves within a moment: show it only when it is
        // still waiting after PENDING_GRACE, so the indicator does not flash after every
        // edit. Offline it shows at once.
        if !view.offline && view.failed.is_empty() && !view.pending.is_empty() {
            let since = match imp.pending_since.get() {
                Some(t) => t,
                None => {
                    let now = Instant::now();
                    imp.pending_since.set(Some(now));
                    let weak = self.downgrade();
                    glib::timeout_add_local_once(PENDING_GRACE, move || {
                        if let Some(w) = weak.upgrade() {
                            w.refresh_sync();
                        }
                    });
                    now
                }
            };
            if since.elapsed() < PENDING_GRACE {
                view.pending.clear();
            }
        } else {
            imp.pending_since.set(None);
        }
        if let Some(s) = imp.sync.get() {
            s.update(&view);
        }
    }

    fn dismiss_errors(&self) {
        let failed = self.store().failed_commands().unwrap_or_default();
        for f in failed {
            let _ = self.store().dismiss_failed(f.seq);
        }
        self.imp().known_failed.set(Some(0));
        self.refresh();
    }

    // ---- rows ----

    fn on_row_activated(&self, pos: u32) {
        let Some(row) = self
            .imp()
            .rows
            .get()
            .and_then(|s| s.item(pos))
            .and_downcast::<TickRow>()
        else {
            return;
        };
        let r = row.row().clone();
        match r {
            Row::SectionHeader(s) => self.toggle_section(&s.id),
            Row::Ghost { prefill, .. } => self.open_quick_add(Some(prefill)),
            // Opening the detail pane is milestone 3; activation only selects for now.
            _ => {}
        }
    }

    fn on_row_action(&self, action: RowAction) {
        match action {
            RowAction::Ring { id, active } => {
                if active {
                    self.begin_complete(&id);
                } else {
                    self.uncheck(&id);
                }
            }
            RowAction::ToggleSubtasks(id) => self.toggle_subtasks(&id),
            RowAction::ToggleSection(id) => self.toggle_section(&id),
            RowAction::Ghost(prefill) => self.open_quick_add(Some(prefill)),
        }
    }

    fn toggle_section(&self, id: &str) {
        let value = {
            let mut ui = self.imp().ui.borrow_mut();
            if !ui.collapsed_sections.remove(id) {
                ui.collapsed_sections.insert(id.to_owned());
            }
            ui.collapsed_sections
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(",")
        };
        self.save_setting("collapsed-sections", &value);
        self.refresh_rows();
    }

    fn toggle_subtasks(&self, id: &str) {
        let collapsed = self
            .store()
            .task(id)
            .ok()
            .flatten()
            .is_some_and(|t| t.is_collapsed);
        self.enqueue(vec![Command::item_update(
            id,
            serde_json::json!({ "is_collapsed": !collapsed }),
        )]);
    }

    fn enqueue(&self, commands: Vec<Command>) {
        let result = self.store().enqueue_many(commands);
        match result {
            Ok(c) => self.after_local_change(&c),
            Err(e) => eprintln!("tick: saving a change: {e}"),
        }
    }

    fn after_local_change(&self, c: &Changes) {
        self.remap_ids(&c.id_map);
        self.refresh_rows_with(&c.id_map);
        self.refresh_sidebar();
        self.refresh_sync();
        if let Some(e) = self.imp().engine.borrow().as_ref() {
            e.sync_now();
        }
    }

    /// The id a task has now: a temp id the server already replaced resolves to the real
    /// one (a hold timer or the Undo group may still hold the temp id).
    fn live_id(&self, id: &str) -> String {
        self.store()
            .resolve_id(id)
            .unwrap_or_else(|_| id.to_owned())
    }

    /// Follow temp ids the server replaced in the completion state and the Undo group.
    fn remap_ids(&self, id_map: &[(String, String)]) {
        if id_map.is_empty() {
            return;
        }
        let imp = self.imp();
        {
            let mut ui = imp.ui.borrow_mut();
            let ui = &mut *ui;
            for (temp, real) in id_map {
                for set in [&mut ui.completing, &mut ui.fading] {
                    if set.remove(temp) {
                        set.insert(real.clone());
                    }
                }
            }
        }
        if let Some(g) = imp.undo.borrow_mut().as_mut() {
            for entry in &mut g.commands {
                if let Some((_, real)) = id_map.iter().find(|(t, _)| *t == entry.id) {
                    entry.id = real.clone();
                }
            }
        }
    }

    /// The current id of a task in the completion hold, or `None` when it left the hold
    /// (unchecked). `id` may be a temp id the server replaced during the hold.
    fn completing_id(&self, id: &str) -> Option<String> {
        let real = self.live_id(id);
        if real != id {
            self.remap_ids(&[(id.to_owned(), real.clone())]);
        }
        self.imp()
            .ui
            .borrow()
            .completing
            .contains(&real)
            .then_some(real)
    }

    /// The ring was checked (or E pressed): hold, fade, then queue the close with an Undo
    /// window (4.4, 4.12). Recurring tasks are queued at once and stay as completed-pending.
    pub fn begin_complete(&self, id: &str) {
        let id = self.live_id(id);
        let Some(task) = self.store().task(&id).ok().flatten() else {
            return;
        };
        if task.checked || self.imp().ui.borrow().completing.contains(&id) {
            // Nothing changes, but the ring was toggled: show the data again.
            self.touch_task_row(&id);
            return;
        }
        let recurring = task.item.due.as_ref().is_some_and(|d| d.is_recurring);
        if recurring || (test_mode::enabled() && !test_mode::keep_hold()) {
            // Test mode skips the hold and the fade (5.3) so E2E timing is deterministic,
            // unless TICK_TEST_HOLD=1 asks for the real sequence.
            self.finish_complete(&id, recurring);
            return;
        }
        self.imp().ui.borrow_mut().completing.insert(id.clone());
        self.refresh_rows();
        let weak = self.downgrade();
        glib::timeout_add_local_once(HOLD, move || {
            let Some(w) = weak.upgrade() else { return };
            // The task may have got its real id from the server during the hold.
            let Some(id) = w.completing_id(&id) else {
                return; // unchecked during the hold
            };
            w.imp().ui.borrow_mut().fading.insert(id.clone());
            w.refresh_rows();
            let weak = w.downgrade();
            glib::timeout_add_local_once(EXIT, move || {
                if let Some(w) = weak.upgrade()
                    && let Some(id) = w.completing_id(&id)
                {
                    w.finish_complete(&id, false);
                }
            });
        });
    }

    fn finish_complete(&self, id: &str, recurring: bool) {
        {
            let mut ui = self.imp().ui.borrow_mut();
            ui.completing.remove(id);
            ui.fading.remove(id);
        }
        // Keyboard focus in the list stays in the list: without this, removing the focused
        // row hands focus to the toast's Undo button, which also pauses the toast's timeout.
        let imp = self.imp();
        let focus_pos = gtk::prelude::GtkWindowExt::focus(self)
            .filter(|f| f.is_ancestor(&*imp.task_list))
            .map(|_| self.row_position(id).unwrap_or(0));
        // The selection moves on only when the completed task was the selected one (E).
        let was_selected = self.selected_task().as_deref() == Some(id);
        let cmd = Command::item_close(id);
        let uuid = cmd.uuid.clone();
        // Held back while the Undo toast shows (see `push_undo`).
        let result = self.store().enqueue_deferred(vec![cmd], undo_hold());
        let changes = match result {
            Ok(c) => c,
            Err(e) => {
                eprintln!("tick: completing a task: {e}");
                self.refresh_rows();
                return;
            }
        };
        self.after_local_change(&changes);
        if let Some(pos) = focus_pos {
            self.keep_focus_in_list(pos, was_selected);
        }
        self.push_undo(UndoEntry {
            id: id.to_owned(),
            uuid,
            recurring,
        });
    }

    /// Show (or grow) the "Undo" toast. Every close it covers stays queued while it shows:
    /// they are deferred by [`UNDO_HOLD`] again each time the group grows, and released
    /// when the toast goes away without Undo.
    fn push_undo(&self, entry: UndoEntry) {
        let imp = self.imp();
        let mut commands = imp
            .undo
            .borrow_mut()
            .take()
            .map(|g| g.commands)
            .unwrap_or_default();
        commands.push(entry);
        let uuids: Vec<String> = commands.iter().map(|e| e.uuid.clone()).collect();
        if let Err(e) = self.store().defer_queued(&uuids, undo_hold()) {
            eprintln!("tick: holding completions for Undo: {e}");
        }
        let title = if commands.len() == 1 {
            gettext("Task completed")
        } else {
            ncount("{n} task completed", "{n} tasks completed", commands.len())
        };
        // A new toast instead of retitling: whether libadwaita restarts the timeout on
        // `set_timeout` is unverified (4.12).
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
            let group = {
                let mut undo = w.imp().undo.borrow_mut();
                if undo.as_ref().is_some_and(|g| &g.toast == t) {
                    undo.take()
                } else {
                    None // replaced by a grown group, or undone
                }
            };
            // Undo is no longer reachable: send the completions now.
            if let Some(g) = group {
                w.release(&g);
            }
        });
        imp.undo.replace(Some(UndoGroup {
            toast: toast.clone(),
            commands,
        }));
        self.show_toast(&toast);
    }

    /// Let the completions of a group go to the server.
    fn release(&self, group: &UndoGroup) {
        let uuids: Vec<String> = group.commands.iter().map(|e| e.uuid.clone()).collect();
        if let Err(e) = self.store().defer_queued(&uuids, Duration::ZERO) {
            eprintln!("tick: releasing completions: {e}");
        }
        self.refresh_sync();
        if let Some(e) = self.imp().engine.borrow().as_ref() {
            e.sync_now();
        }
    }

    /// Undo the completions of the visible toast (its button or Ctrl+Z).
    fn undo(&self) {
        let Some(group) = self.imp().undo.take() else {
            return;
        };
        group.toast.dismiss();
        let mut all = Changes::default();
        let mut reopen = Vec::new();
        let mut lost = 0;
        for entry in &group.commands {
            // Bound first: the store must not stay borrowed inside the arms.
            let discarded = self.store().discard_queued(&entry.uuid);
            match discarded {
                Ok(Some(c)) => all.merge(c),
                // Already sent (the toast outlived UNDO_HOLD): reopen it on the server.
                Ok(None) if !entry.recurring => {
                    reopen.push(Command::item_uncomplete(&self.live_id(&entry.id)));
                }
                // A recurring task already moved to its next date; reopening would not move
                // it back.
                Ok(None) => lost += 1,
                Err(e) => {
                    eprintln!("tick: undo: {e}");
                    lost += 1;
                }
            }
        }
        if !reopen.is_empty() {
            let reopened = self.store().enqueue_many(reopen);
            match reopened {
                Ok(c) => all.merge(c),
                Err(e) => eprintln!("tick: undo: {e}"),
            }
        }
        self.after_local_change(&all);
        if lost > 0 {
            let toast = adw::Toast::builder()
                .title(ncount(
                    "{n} task was already saved and could not be restored",
                    "{n} tasks were already saved and could not be restored",
                    lost,
                ))
                .timeout(5)
                .build();
            self.show_toast(&toast);
        }
    }

    fn uncheck(&self, id: &str) {
        let id = self.live_id(id);
        let cancelled = {
            let mut ui = self.imp().ui.borrow_mut();
            ui.fading.remove(&id);
            ui.completing.remove(&id)
        };
        if cancelled {
            self.refresh_rows();
            return;
        }
        // A completed-pending recurring task: take its close back while Undo is possible.
        let uuid = self.imp().undo.borrow().as_ref().and_then(|g| {
            g.commands
                .iter()
                .find(|e| e.id == id)
                .map(|e| e.uuid.clone())
        });
        match uuid {
            Some(uuid) => {
                if let Some(g) = self.imp().undo.borrow_mut().as_mut() {
                    g.commands.retain(|e| e.id != id);
                }
                let discarded = self.store().discard_queued(&uuid);
                match discarded {
                    Ok(Some(c)) => self.after_local_change(&c),
                    Ok(None) => self.touch_task_row(&id),
                    Err(e) => eprintln!("tick: undo: {e}"),
                }
            }
            None => self.touch_task_row(&id),
        }
    }

    /// Position of task `id` in the task list.
    fn row_position(&self, id: &str) -> Option<u32> {
        let store = self.imp().rows.get()?;
        (0..store.n_items()).find(|i| {
            store
                .item(*i)
                .and_downcast::<TickRow>()
                .and_then(|r| r.task_id())
                .as_deref()
                == Some(id)
        })
    }

    /// Rebuild the row of task `id` from unchanged data (undo a ring toggle that had no
    /// effect).
    fn touch_task_row(&self, id: &str) {
        let Some(store) = self.imp().rows.get() else {
            return;
        };
        for i in 0..store.n_items() {
            if let Some(r) = store.item(i).and_downcast::<TickRow>()
                && r.task_id().as_deref() == Some(id)
            {
                r.touch();
            }
        }
    }

    fn complete_selected(&self) {
        let Some(row) = self
            .imp()
            .selection
            .get()
            .and_then(|s| s.selected_item())
            .and_downcast::<TickRow>()
        else {
            return;
        };
        let task = row.row().task().cloned();
        if let Some(t) = task
            && !t.checked
        {
            self.begin_complete(&t.id);
        }
    }

    // ---- toasts ----

    /// Toasts never stack (4.12): dismiss the visible one first.
    fn show_toast(&self, toast: &adw::Toast) {
        if let Some(prev) = self.imp().last_toast.replace(Some(toast.clone())) {
            prev.dismiss();
        }
        self.imp().toast_overlay.add_toast(toast.clone());
    }

    // ---- quick add ----

    pub fn open_quick_add(&self, prefill: Option<String>) {
        let imp = self.imp();
        if imp.stack.visible_child_name().as_deref() != Some("main")
            || self.visible_dialog().is_some()
        {
            return;
        }
        let (projects, labels) = {
            let store = self.store();
            let projects = store
                .projects_tree()
                .unwrap_or_default()
                .into_iter()
                .filter(|n| !n.project.inbox_project)
                .map(|n| (n.project.name.clone(), n.project.color.clone()))
                .collect();
            let labels = store
                .labels()
                .unwrap_or_default()
                .into_iter()
                .map(|l| l.label.name)
                .collect();
            (projects, labels)
        };
        let dialog = TickQuickAdd::new(projects, labels, imp.ui.borrow().offline);
        let draft = imp.draft.take();
        let text = match prefill {
            Some(p) => p,
            None if !draft.is_empty() => draft,
            None => imp
                .content
                .borrow()
                .as_ref()
                .map(|c| c.prefill.clone())
                .unwrap_or_default(),
        };
        dialog.set_text(&text);
        let weak = self.downgrade();
        dialog.connect_submit(move |text, _another| {
            if let Some(w) = weak.upgrade() {
                w.quick_add(&text);
            }
        });
        // Focus goes back where it was (a task row), else to the list, not to whatever
        // header button GTK picks.
        let before = gtk::prelude::GtkWindowExt::focus(self).map(|f| f.downgrade());
        let weak = self.downgrade();
        dialog.connect_closed(move |d| {
            let Some(w) = weak.upgrade() else { return };
            if !d.was_submitted() {
                // Escape keeps the draft for the next opening in this session.
                w.imp().draft.replace(d.text());
            }
            let before = before.clone();
            let weak = w.downgrade();
            glib::idle_add_local_once(move || {
                let Some(w) = weak.upgrade() else { return };
                match before.and_then(|b| b.upgrade()) {
                    Some(b) if b.is_mapped() && b.root().is_some() => {
                        b.grab_focus();
                    }
                    _ => {
                        let selected = w.selected_position();
                        w.keep_focus_in_list(selected.unwrap_or(0), selected.is_some());
                    }
                }
            });
        });
        dialog.present(Some(self));
    }

    fn quick_add(&self, text: &str) {
        let cmd = Command::quick_add(text);
        let temp_id = cmd.temp_id.clone().unwrap_or_default();
        let result = self.store().enqueue(cmd);
        match result {
            Ok(c) => self.after_local_change(&c),
            Err(e) => {
                eprintln!("tick: quick add: {e}");
                return;
            }
        }
        if self.current_view() != ViewId::Inbox {
            let toast = adw::Toast::builder()
                .title(gettext("Task added"))
                .button_label(gettext("Open"))
                .timeout(4)
                .build();
            let weak = self.downgrade();
            toast.connect_button_clicked(move |_| {
                let Some(w) = weak.upgrade() else { return };
                let project = {
                    let store = w.store();
                    let id = store.resolve_id(&temp_id).unwrap_or(temp_id.clone());
                    let inbox = store.inbox_project_id().ok().flatten();
                    store
                        .task(&id)
                        .ok()
                        .flatten()
                        .map(|t| t.project_id.clone())
                        .filter(|p| Some(p) != inbox.as_ref())
                };
                let view = project.map_or(ViewId::Inbox, ViewId::Project);
                w.open_view(view, true);
            });
            self.show_toast(&toast);
        }
    }

    // ---- keyboard ----

    fn on_key(&self, key: gtk::gdk::Key, mods: gtk::gdk::ModifierType) -> bool {
        use gtk::gdk::{Key, ModifierType};
        if mods.intersects(
            ModifierType::CONTROL_MASK | ModifierType::ALT_MASK | ModifierType::SUPER_MASK,
        ) {
            return false;
        }
        let imp = self.imp();
        if imp.stack.visible_child_name().as_deref() != Some("main")
            || self.visible_dialog().is_some()
        {
            return false;
        }
        if gtk::prelude::GtkWindowExt::focus(self).is_some_and(|f| f.is::<gtk::Text>()) {
            return false;
        }
        let pending_g = imp
            .g_pending
            .take()
            .is_some_and(|t| t.elapsed() < SEQUENCE_TIMEOUT);
        let goto = |view: ViewId| {
            self.open_view(view, true);
            self.focus_first_task();
            true
        };
        match key.to_lower() {
            Key::q => {
                self.open_quick_add(None);
                true
            }
            Key::g => {
                imp.g_pending.set(Some(Instant::now()));
                true
            }
            Key::i if pending_g => goto(ViewId::Inbox),
            Key::t if pending_g => goto(ViewId::Today),
            Key::u if pending_g => goto(ViewId::Upcoming),
            Key::e => {
                self.complete_selected();
                true
            }
            _ => false,
        }
    }

    /// Move focus to the next interactive row (task, section header, "Add task") above or
    /// below the focused one. Returns false when there is none (GTK handles the key).
    fn move_focus(&self, down: bool) -> bool {
        let imp = self.imp();
        let list: gtk::Widget = imp.task_list.get().upcast();
        let Some(mut w) = gtk::prelude::GtkWindowExt::focus(self) else {
            return false;
        };
        // Up to the row widget, a direct child of the list view.
        while w.parent().as_ref() != Some(&list) {
            let Some(p) = w.parent() else { return false };
            w = p;
        }
        let key = w
            .first_child()
            .and_downcast::<adw::Bin>()
            .and_then(|b| b.child())
            .map(|c| c.widget_name().to_string());
        let Some(key) = key else { return false };
        let Some(store) = imp.rows.get() else {
            return false;
        };
        let rows: Vec<TickRow> = (0..store.n_items())
            .filter_map(|i| store.item(i).and_downcast::<TickRow>())
            .collect();
        let Some(pos) = rows.iter().position(|r| r.key() == key) else {
            return false;
        };
        let interactive = |r: &TickRow| {
            matches!(
                &*r.row(),
                Row::Task(_) | Row::SectionHeader(_) | Row::Ghost { .. }
            )
        };
        let next = if down {
            (pos + 1..rows.len()).find(|i| interactive(&rows[*i]))
        } else {
            (0..pos).rev().find(|i| interactive(&rows[*i]))
        };
        let Some(next) = next else { return false };
        let mut flags = gtk::ListScrollFlags::FOCUS;
        if matches!(&*rows[next].row(), Row::Task(_)) {
            flags |= gtk::ListScrollFlags::SELECT;
        }
        imp.task_list.scroll_to(next as u32, flags, None);
        true
    }

    /// Move keyboard focus to the first task of the list (after "go to" shortcuts).
    fn focus_first_task(&self) {
        // After the rows of the new view are built.
        let weak = self.downgrade();
        glib::idle_add_local_once(move || {
            if let Some(w) = weak.upgrade() {
                w.focus_first_task_now();
            }
        });
    }

    fn focus_first_task_now(&self) {
        let _ = self.focus_task_near(0, true);
    }

    /// Focus (and select, if `select`) the first task row at or after position `from`,
    /// else the last one before it.
    fn focus_task_near(&self, from: u32, select: bool) -> bool {
        self.focus_row_near(from, select, |r| matches!(r, Row::Task(_)))
    }

    /// Like [`Self::focus_task_near`], for keyboard focus that was in the list: when no task
    /// is left, an "Add task" row or section header, else no focus at all. Never the toast
    /// (GTK moves focus there when the focused row goes away, and a focused toast never
    /// times out).
    fn keep_focus_in_list(&self, from: u32, select: bool) {
        if self.focus_task_near(from, select) {
            return;
        }
        let interactive =
            |r: &Row| matches!(r, Row::Task(_) | Row::SectionHeader(_) | Row::Ghost { .. });
        if !self.focus_row_near(from, false, interactive) {
            gtk::prelude::GtkWindowExt::set_focus(self, None::<&gtk::Widget>);
        }
    }

    fn focus_row_near(&self, from: u32, select: bool, want: impl Fn(&Row) -> bool) -> bool {
        let imp = self.imp();
        let Some(store) = imp.rows.get() else {
            return false;
        };
        let is_task = |i: &u32| {
            store
                .item(*i)
                .and_downcast::<TickRow>()
                .is_some_and(|r| want(&r.row()))
        };
        let n = store.n_items();
        let found = (from.min(n)..n)
            .find(is_task)
            .or_else(|| (0..from.min(n)).rev().find(is_task));
        if let Some(pos) = found {
            let mut flags = gtk::ListScrollFlags::FOCUS;
            if select {
                flags |= gtk::ListScrollFlags::SELECT;
            }
            imp.task_list.scroll_to(pos, flags, None);
            imp.task_list.grab_focus();
            return true;
        }
        false
    }
}

/// Whether `w` is inside a toast (libadwaita's private `AdwToastWidget`).
fn in_toast(w: &gtk::Widget) -> bool {
    let mut cur = Some(w.clone());
    while let Some(c) = cur {
        if c.type_().name() == "AdwToastWidget" {
            return true;
        }
        cur = c.parent();
    }
    false
}
