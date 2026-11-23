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

    // Drag and drop probe: dragging "Drag source" onto "Drop target" sets the status.
    let source = gtk::Label::builder()
        .label("Drag source")
        .height_request(60)
        .build();
    let drag = gtk::DragSource::new();
    drag.set_actions(gtk::gdk::DragAction::MOVE);
    drag.connect_prepare(|_, _, _| {
        Some(gtk::gdk::ContentProvider::for_value(&"probe-42".to_value()))
    });
    drag.connect_drag_begin(glib::clone!(
        #[weak]
        status,
        move |_, d| {
            status.set_label("dragging");
            let icon = gtk::DragIcon::for_drag(d);
            icon.set_child(Some(&gtk::Label::new(Some("Drag icon"))));
        }
    ));
    source.add_controller(drag);
    let target = gtk::Label::builder()
        .label("Drop target")
        .height_request(60)
        .build();
    let drop = gtk::DropTarget::new(glib::Type::STRING, gtk::gdk::DragAction::MOVE);
    drop.connect_drop(glib::clone!(
        #[weak]
        status,
        #[upgrade_or]
        false,
        move |_, v, _, _| {
            status.set_label(&format!(
                "dropped {}",
                v.get::<String>().unwrap_or_default()
            ));
            true
        }
    ));
    target.add_controller(drop);

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
    content.append(&source);
    content.append(&target);

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
