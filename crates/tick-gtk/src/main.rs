mod format;
mod i18n;
mod login;
mod markdown;
mod quick_add;
mod row_object;
mod rows;
mod sidebar;
mod style;
mod sync_status;
mod task_list;
mod test_mode;
mod window;

use std::cell::RefCell;

use adw::prelude::*;
use gtk::{gio, glib};
use tick_core::Store;

use crate::i18n::gettext;
use crate::window::{Start, TickWindow};

const APP_ID: &str = "dev.chengrui.Tick";

thread_local! {
    /// The cache, opened in `startup` (the theme is read from it before the first paint)
    /// and handed to the window in `activate`.
    static STORE: RefCell<Option<Store>> = const { RefCell::new(None) };
}

fn main() -> glib::ExitCode {
    i18n::init();
    gio::resources_register_include!("resources.gresource").expect("failed to register resources");

    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_startup(startup);
    app.connect_shutdown(|app| {
        for w in app.windows() {
            if let Ok(w) = w.downcast::<TickWindow>() {
                w.release_held();
            }
        }
    });
    app.connect_activate(activate);
    app.run()
}

fn open_store() -> Store {
    match Store::open_default() {
        Ok(s) => s,
        Err(e) => {
            // A temporary file, not memory: the sync engine opens the same path with its own
            // connection, so UI and engine must share a file. The runtime dir is private to
            // the user and cleared at logout.
            let tmp = glib::user_runtime_dir().join(format!("tick-{}.sqlite3", std::process::id()));
            eprintln!(
                "tick: cannot open the cache, using {} for this session: {e}",
                tmp.display()
            );
            Store::open(&tmp).unwrap_or_else(|e| {
                eprintln!("tick: cannot open a temporary cache either, syncing is off: {e}");
                Store::open_in_memory().expect("in-memory store")
            })
        }
    }
}

fn startup(app: &adw::Application) {
    let test_mode = test_mode::enabled();
    let store = open_store();

    // Theme before the first paint (7.4 pitfall 12): the saved choice, unless
    // TICK_COLOR_SCHEME overrides it (screenshots, tests).
    let saved = store.setting("color-scheme").ok().flatten();
    let scheme = std::env::var("TICK_COLOR_SCHEME").ok().or(saved);
    let scheme = match scheme.as_deref() {
        Some("light") => adw::ColorScheme::ForceLight,
        Some("dark") => adw::ColorScheme::ForceDark,
        _ => adw::ColorScheme::Default,
    };
    adw::StyleManager::default().set_color_scheme(scheme);
    style::install_font_provider();
    STORE.with(|s| s.replace(Some(store)));

    if test_mode {
        if let Some(settings) = gtk::Settings::default() {
            settings.set_gtk_enable_animations(false);
        }
        test_mode::install_actions(app);
    }

    let quit = gio::ActionEntry::builder("quit")
        .activate(|app: &adw::Application, _, _| app.quit())
        .build();
    let about = gio::ActionEntry::builder("about")
        .activate(|app: &adw::Application, _, _| show_about(app))
        .build();
    app.add_action_entries([quit, about]);
    app.set_accels_for_action("app.quit", &["<Control>q"]);
    app.set_accels_for_action("win.undo", &["<Control>z"]);
}

fn show_about(app: &adw::Application) {
    let about = adw::AboutDialog::builder()
        .application_name(gettext("Tick"))
        .application_icon("tick-app-symbolic")
        .comments(gettext("A Todoist client"))
        .version(env!("CARGO_PKG_VERSION"))
        .license_type(gtk::License::Gpl30)
        .website("https://github.com/jczhang02/tick")
        .build();
    about.present(app.active_window().as_ref());
}

fn activate(app: &adw::Application) {
    // Closing the window only hides it (the process keeps running for sync and
    // reminders), so a second activation presents the existing window.
    if let Some(window) = app.windows().into_iter().next() {
        window.present();
        return;
    }

    let store = STORE.with(|s| s.take()).unwrap_or_else(open_store);
    let env_token = tick_core::token::env_token();
    let has_account = store.user().ok().flatten().is_some();
    let start = if env_token.is_some() || has_account {
        Start::Main
    } else {
        Start::Login
    };
    // Models are filled from the cache here, before the window is presented.
    let window = TickWindow::new(app, store, start);
    // The hidden window stays registered with the application, which keeps the
    // process alive until app.quit.
    window.set_hide_on_close(true);
    if test_mode::enabled() {
        window.maximize();
        test_mode::report_first_frame(&window);
    }
    window.present();

    // No network before the first frame (docs/architecture.md, data flow 1): the token
    // (keyring on a worker thread) and the sync engine start after the first paint.
    let weak = window.downgrade();
    after_first_frame(&window, move || {
        let Some(window) = weak.upgrade() else { return };
        if let Some(token) = env_token {
            window.start_engine(token);
            return;
        }
        if test_mode::enabled() {
            // Tests never touch a Secret Service (docs/testing.md, fact 8).
            window.token_loaded(None);
            return;
        }
        let load = gio::spawn_blocking(tick_core::token::load_blocking);
        glib::spawn_future_local(async move {
            let token = match load.await {
                Ok(Ok(t)) => t,
                Ok(Err(e)) => {
                    eprintln!("tick: reading the keyring: {e}");
                    None
                }
                Err(_) => None,
            };
            if let Some(window) = weak.upgrade() {
                window.token_loaded(token);
            }
        });
    });
}

/// Runs `f` once, after the window's first paint (an idle callback alone can run before it:
/// on Wayland the first frame waits for the compositor's configure).
fn after_first_frame(window: &TickWindow, f: impl FnOnce() + 'static) {
    let f = std::rc::Rc::new(RefCell::new(Some(f)));
    let Some(clock) = window.frame_clock() else {
        glib::idle_add_local_once(move || {
            if let Some(f) = f.borrow_mut().take() {
                f();
            }
        });
        return;
    };
    let handler = std::rc::Rc::new(std::cell::Cell::new(None));
    let handler_in = handler.clone();
    let id = clock.connect_after_paint(move |clock| {
        if let Some(id) = handler_in.take() {
            clock.disconnect(id);
        }
        // Not inside the paint cycle.
        let f = f.clone();
        glib::idle_add_local_once(move || {
            if let Some(f) = f.borrow_mut().take() {
                f();
            }
        });
    });
    handler.set(Some(id));
}
