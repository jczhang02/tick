mod window;

use adw::prelude::*;
use gtk::{gio, glib};

const APP_ID: &str = "dev.chengrui.Tick";

fn main() -> glib::ExitCode {
    gio::resources_register_include!("resources.gresource").expect("failed to register resources");

    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(|app| {
        let window = window::TickWindow::new(app);
        if std::env::var_os("TICK_TEST_MODE").is_some() {
            window.maximize();
        }
        window.present();
    });
    app.run()
}
