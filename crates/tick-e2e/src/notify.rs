//! Recording fakes of `org.gtk.Notifications` and `org.freedesktop.Notifications`.
//!
//! The harness owns both names on the private bus before the app starts, records every
//! notification, and can invoke a notification's buttons the way GNOME Shell would.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Context as _;
use zbus::zvariant::{OwnedValue, Value};

use crate::{Result, poll};

/// Which D-Bus API delivered a notification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    /// `org.gtk.Notifications.AddNotification` (GLib's "gtk" backend).
    Gtk,
    /// `org.freedesktop.Notifications.Notify`.
    Freedesktop,
}

/// A notification button.
#[derive(Clone, Debug)]
pub struct NotificationButton {
    pub label: String,
    /// Gtk: the detailed action (`app.complete`). Freedesktop: the action key.
    pub action: String,
    /// Gtk: the action target, if any.
    pub target: Option<Arc<OwnedValue>>,
}

/// A recorded notification.
#[derive(Clone, Debug)]
pub struct Notification {
    pub backend: Backend,
    /// Gtk: the notification id given by the app. Freedesktop: the server-assigned id.
    pub id: String,
    /// Gtk: the app id. Freedesktop: the app name.
    pub app: String,
    pub title: String,
    pub body: String,
    pub buttons: Vec<NotificationButton>,
    /// Gtk: `default-action` and its target. Freedesktop: the `default` action key.
    pub default_action: Option<NotificationButton>,
    /// Set when the app withdrew it.
    pub removed: bool,
}

#[derive(Default)]
struct Store {
    sent: Vec<Notification>,
    next_id: u32,
}

type Shared = Arc<Mutex<Store>>;

/// The fake notification servers of a session.
pub struct Notifications {
    store: Shared,
    bus: zbus::Connection,
}

impl Notifications {
    pub(crate) async fn install(bus: &zbus::Connection, own_gtk: bool) -> Result<Self> {
        let store = Shared::default();
        let server = bus.object_server();
        server
            .at("/org/gtk/Notifications", GtkServer(store.clone()))
            .await?;
        server
            .at("/org/freedesktop/Notifications", FdoServer(store.clone()))
            .await?;
        if own_gtk {
            bus.request_name("org.gtk.Notifications").await?;
        }
        bus.request_name("org.freedesktop.Notifications").await?;
        Ok(Self {
            store,
            bus: bus.clone(),
        })
    }

    /// Every delivery so far, oldest first. Re-sending an id (which replaces the
    /// notification on screen) adds another entry.
    pub fn sent(&self) -> Vec<Notification> {
        self.store.lock().expect("store lock").sent.clone()
    }

    /// Waits for a notification matching `pred`.
    pub async fn wait_for(
        &self,
        timeout: Duration,
        pred: impl Fn(&Notification) -> bool,
    ) -> Result<Notification> {
        let pred = &pred;
        poll(timeout, "notification", || async move {
            self.sent().into_iter().find(|n| pred(n))
        })
        .await
    }

    /// Clicks the button labeled `label`, as the shell would.
    pub async fn invoke(&self, n: &Notification, label: &str) -> Result<()> {
        let button = n
            .buttons
            .iter()
            .find(|b| b.label == label)
            .with_context(|| format!("no button {label:?} in {:?}", n.buttons))?;
        self.trigger(n, button).await
    }

    /// Clicks the notification body (its default action).
    pub async fn activate_default(&self, n: &Notification) -> Result<()> {
        let button = n.default_action.as_ref().context("no default action")?;
        self.trigger(n, button).await
    }

    async fn trigger(&self, n: &Notification, button: &NotificationButton) -> Result<()> {
        match n.backend {
            Backend::Gtk => {
                // GNOME Shell activates the app action over org.freedesktop.Application.
                let name = button
                    .action
                    .strip_prefix("app.")
                    .with_context(|| format!("not an app action: {}", button.action))?;
                let params: Vec<Value<'_>> = match &button.target {
                    Some(t) => vec![Value::from(t.try_clone()?)],
                    None => vec![],
                };
                let platform: HashMap<&str, Value<'_>> = HashMap::new();
                let path = format!("/{}", n.app.replace('.', "/"));
                self.bus
                    .call_method(
                        Some(n.app.as_str()),
                        path.as_str(),
                        Some("org.freedesktop.Application"),
                        "ActivateAction",
                        &(name, params, platform),
                    )
                    .await
                    .with_context(|| format!("activating {}", button.action))?;
            }
            Backend::Freedesktop => {
                let id: u32 = n.id.parse()?;
                self.bus
                    .emit_signal(
                        None::<()>,
                        "/org/freedesktop/Notifications",
                        "org.freedesktop.Notifications",
                        "ActionInvoked",
                        &(id, button.action.as_str()),
                    )
                    .await?;
            }
        }
        Ok(())
    }
}

fn string_of(map: &HashMap<String, OwnedValue>, key: &str) -> String {
    map.get(key)
        .and_then(|v| <&str>::try_from(&**v).ok().map(str::to_owned))
        .unwrap_or_default()
}

fn target_of(map: &HashMap<String, OwnedValue>, key: &str) -> Option<Arc<OwnedValue>> {
    map.get(key).and_then(|v| v.try_clone().ok()).map(Arc::new)
}

struct GtkServer(Shared);

#[zbus::interface(name = "org.gtk.Notifications")]
impl GtkServer {
    fn add_notification(
        &self,
        app_id: String,
        id: String,
        notification: HashMap<String, OwnedValue>,
    ) {
        let buttons = notification
            .get("buttons")
            .and_then(|v| v.try_clone().ok())
            .and_then(|v| Vec::<HashMap<String, OwnedValue>>::try_from(v).ok())
            .unwrap_or_default()
            .iter()
            .map(|b| NotificationButton {
                label: string_of(b, "label"),
                action: string_of(b, "action"),
                target: target_of(b, "target"),
            })
            .collect();
        let default_action =
            notification
                .contains_key("default-action")
                .then(|| NotificationButton {
                    label: String::new(),
                    action: string_of(&notification, "default-action"),
                    target: target_of(&notification, "default-action-target"),
                });
        let mut store = self.0.lock().expect("store lock");
        store.sent.push(Notification {
            backend: Backend::Gtk,
            id,
            app: app_id,
            title: string_of(&notification, "title"),
            body: string_of(&notification, "body"),
            buttons,
            default_action,
            removed: false,
        });
    }

    fn remove_notification(&self, app_id: String, id: String) {
        let mut store = self.0.lock().expect("store lock");
        for n in &mut store.sent {
            if n.backend == Backend::Gtk && n.app == app_id && n.id == id {
                n.removed = true;
            }
        }
    }
}

struct FdoServer(Shared);

#[zbus::interface(name = "org.freedesktop.Notifications")]
impl FdoServer {
    #[allow(clippy::too_many_arguments)]
    fn notify(
        &self,
        app_name: String,
        replaces_id: u32,
        _app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        _hints: HashMap<String, OwnedValue>,
        _expire_timeout: i32,
    ) -> u32 {
        let mut store = self.0.lock().expect("store lock");
        let id = if replaces_id != 0 {
            replaces_id
        } else {
            store.next_id += 1;
            store.next_id
        };
        let mut buttons = Vec::new();
        let mut default_action = None;
        for pair in actions.chunks(2) {
            let key = pair[0].clone();
            let label = pair.get(1).cloned().unwrap_or_default();
            let button = NotificationButton {
                label,
                action: key,
                target: None,
            };
            if button.action == "default" {
                default_action = Some(button);
            } else {
                buttons.push(button);
            }
        }
        store.sent.push(Notification {
            backend: Backend::Freedesktop,
            id: id.to_string(),
            app: app_name,
            title: summary,
            body,
            buttons,
            default_action,
            removed: false,
        });
        id
    }

    fn close_notification(&self, id: u32) {
        let mut store = self.0.lock().expect("store lock");
        for n in &mut store.sent {
            if n.backend == Backend::Freedesktop && n.id == id.to_string() {
                n.removed = true;
            }
        }
    }

    fn get_capabilities(&self) -> Vec<String> {
        vec!["actions".into(), "body".into()]
    }

    fn get_server_information(&self) -> (String, String, String, String) {
        (
            "tick-e2e".into(),
            "tick".into(),
            env!("CARGO_PKG_VERSION").into(),
            "1.2".into(),
        )
    }
}
