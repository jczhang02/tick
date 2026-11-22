//! E2E harness fixture. Not part of Tick and never installed.
//!
//! A minimal GTK app with a text entry and a button that sends a `GNotification`. The
//! `tick-e2e` self-tests use it to check what Tick cannot exercise yet: typing into an
//! entry (including CJK text) and notification delivery and actions.

use adw::prelude::*;
use gtk::{gio, glib};

fn main() -> glib::ExitCode {
    let app = adw::Application::builder()
        .application_id("dev.chengrui.Tick.E2eFixture")
        .build();
    app.connect_startup(|_| {
        if let Some(settings) = gtk::Settings::default() {
            settings.set_gtk_enable_animations(false);
        }
    });
    app.connect_activate(build);
    app.run()
}

fn build(app: &adw::Application) {
    let status = gtk::Label::new(Some("idle"));

    let entry = gtk::Entry::new();
    entry.update_property(&[gtk::accessible::Property::Label("Probe entry")]);

    let send = gtk::Button::with_label("Send notification");
    send.connect_clicked(glib::clone!(
        #[weak]
        app,
        move |_| {
            let n = gio::Notification::new("Reminder");
            n.set_body(Some("Buy milk"));
            n.add_button_with_target_value("Complete", "app.complete", Some(&"42".to_variant()));
            n.set_default_action("app.open");
            app.send_notification(Some("reminder-42"), &n);
        }
    ));

    let complete = gio::ActionEntry::builder("complete")
        .parameter_type(Some(glib::VariantTy::STRING))
        .activate(glib::clone!(
            #[weak]
            status,
            move |_: &adw::Application, _, param| {
                let target = param.and_then(|p| p.get::<String>()).unwrap_or_default();
                status.set_label(&format!("completed {target}"));
            }
        ))
        .build();
    let open = gio::ActionEntry::builder("open")
        .activate(glib::clone!(
            #[weak]
            status,
            move |_: &adw::Application, _, _| status.set_label("opened")
        ))
        .build();
    app.add_action_entries([complete, open]);

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_top(24)
        .margin_start(24)
        .margin_end(24)
        .build();
    content.append(&entry);
    content.append(&send);
    content.append(&status);

    let view = adw::ToolbarView::new();
    view.add_top_bar(&adw::HeaderBar::new());
    view.set_content(Some(&content));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Fixture")
        .content(&view)
        .build();
    window.maximize();
    window.present();
}
