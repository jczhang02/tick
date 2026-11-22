mod test_mode;
mod window;

use adw::prelude::*;
use gtk::{gio, glib};

const APP_ID: &str = "dev.chengrui.Tick";

fn main() -> glib::ExitCode {
    gio::resources_register_include!("resources.gresource").expect("failed to register resources");

    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_startup(startup);
    app.connect_activate(activate);
    app.run()
}

fn startup(app: &adw::Application) {
    let test_mode = test_mode::enabled();

    // TICK_COLOR_SCHEME overrides the saved theme choice (screenshots, tests).
    match std::env::var("TICK_COLOR_SCHEME").as_deref() {
        Ok("light") => adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceLight),
        Ok("dark") => adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceDark),
        _ => {}
    }

    if test_mode {
        if let Some(settings) = gtk::Settings::default() {
            settings.set_gtk_enable_animations(false);
        }
        test_mode::install_actions(app);
    }

    let quit = gio::ActionEntry::builder("quit")
        .activate(|app: &adw::Application, _, _| app.quit())
        .build();
    app.add_action_entries([quit]);
    app.set_accels_for_action("app.quit", &["<Control>q"]);
}

fn activate(app: &adw::Application) {
    // Closing the window only hides it (the process keeps running for sync and
    // reminders), so a second activation presents the existing window.
    if let Some(window) = app.windows().into_iter().next() {
        window.present();
        return;
    }

    let window = window::TickWindow::new(app);
    // The hidden window stays registered with the application, which keeps the
    // process alive until app.quit.
    window.set_hide_on_close(true);
    if test_mode::enabled() {
        window.maximize();
        test_mode::report_first_frame(&window);
    }
    window.present();
}
