//! "Run at Login" through the Background portal (`org.freedesktop.portal.Background`,
//! `RequestBackground` with `autostart`; `docs/architecture.md`).
//!
//! The request follows the portal's Request pattern: subscribe to `Response` on the request
//! object path derived from our unique name and a handle token, then call. A missing
//! portal, a call error, a denied request or no answer within [`TIMEOUT`] is reported as
//! an error the caller shows; nothing is retried.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use gtk::prelude::*;
use gtk::{gio, glib};

const PORTAL: &str = "org.freedesktop.portal.Desktop";
const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";
/// How long to wait for the portal's answer (it may ask the user first).
const TIMEOUT: Duration = Duration::from_secs(60);

/// Why the portal did not grant the request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortalError {
    /// No portal answers on the session bus (or it has no Background interface).
    Unavailable(String),
    /// The portal (or the user) said no.
    Denied,
}

/// The caller's callback, called once.
type Done = Box<dyn FnOnce(Result<bool, PortalError>)>;

/// Ask for background running with autostart on (`autostart`) or off. `done` receives
/// whether autostart is now enabled.
pub fn request_background(
    app: &gtk::Application,
    autostart: bool,
    reason: &str,
    done: impl FnOnce(Result<bool, PortalError>) + 'static,
) {
    let done: Rc<RefCell<Option<Done>>> = Rc::new(RefCell::new(Some(Box::new(done))));
    let finish = {
        let done = done.clone();
        move |r: Result<bool, PortalError>| {
            if let Some(f) = done.borrow_mut().take() {
                f(r);
            }
        }
    };
    let Some(conn) = gio::prelude::ApplicationExt::dbus_connection(app) else {
        finish(Err(PortalError::Unavailable("no session bus".into())));
        return;
    };
    let sender = conn
        .unique_name()
        .map(|n| n.trim_start_matches(':').replace('.', "_"))
        .unwrap_or_default();
    let token = format!("tick{}", glib::random_int());
    let path = format!("{PORTAL_PATH}/request/{sender}/{token}");

    // Subscribed before the call, so a fast answer is not missed.
    let subscription: Rc<RefCell<Option<gio::SignalSubscription>>> = Rc::default();
    let sub = conn.subscribe_to_signal(
        None,
        Some("org.freedesktop.portal.Request"),
        Some("Response"),
        Some(&path),
        None,
        gio::DBusSignalFlags::NONE,
        {
            let finish = finish.clone();
            let subscription = subscription.clone();
            move |signal| {
                let params = signal.parameters;
                let code = params.child_value(0).get::<u32>().unwrap_or(2);
                let results = glib::VariantDict::new(Some(&params.child_value(1)));
                let enabled = results
                    .lookup::<bool>("autostart")
                    .ok()
                    .flatten()
                    .unwrap_or(autostart);
                subscription.borrow_mut().take();
                finish(if code == 0 {
                    Ok(enabled)
                } else {
                    Err(PortalError::Denied)
                });
            }
        },
    );
    subscription.replace(Some(sub));

    let options = glib::VariantDict::new(None);
    options.insert_value("handle_token", &token.to_variant());
    options.insert_value("reason", &reason.to_variant());
    options.insert_value("autostart", &autostart.to_variant());
    options.insert_value(
        "commandline",
        &vec!["tick".to_owned(), "--background".to_owned()].to_variant(),
    );
    options.insert_value("dbus-activatable", &false.to_variant());
    let args = glib::Variant::tuple_from_iter(["".to_variant(), options.end()]);
    let on_error = {
        let finish = finish.clone();
        let subscription = subscription.clone();
        move |e: String| {
            subscription.borrow_mut().take();
            finish(Err(PortalError::Unavailable(e)));
        }
    };
    let on_error2 = on_error.clone();
    conn.call(
        Some(PORTAL),
        PORTAL_PATH,
        "org.freedesktop.portal.Background",
        "RequestBackground",
        Some(&args),
        Some(glib::VariantTy::new("(o)").expect("type")),
        gio::DBusCallFlags::NONE,
        10_000,
        None::<&gio::Cancellable>,
        move |r| {
            if let Err(e) = r {
                on_error(e.to_string());
            }
        },
    );
    glib::timeout_add_local_once(TIMEOUT, move || {
        if done.borrow().is_some() {
            on_error2("the portal did not answer".into());
        }
    });
}
