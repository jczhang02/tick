//! A fake `org.freedesktop.portal.Desktop` with the Background portal, for "Run at Login".
//!
//! Opt-in ([`crate::SessionBuilder::fake_portal`]): owning the portal name also answers
//! (with errors) the Settings portal calls GTK and libadwaita make at startup, which the
//! other tests should not see.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use zbus::message::Header;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

use crate::{Result, poll};

/// What the fake portal does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortalMode {
    /// `RequestBackground` succeeds and grants what was asked.
    Grant,
    /// `RequestBackground` answers "denied" (response 1).
    Deny,
    /// The portal name has an owner, but no Background interface (an error for every call).
    NoBackground,
}

/// One `RequestBackground` call.
#[derive(Clone, Debug, PartialEq)]
pub struct BackgroundCall {
    pub autostart: bool,
    pub commandline: Vec<String>,
    pub reason: String,
}

type Calls = Arc<Mutex<Vec<BackgroundCall>>>;

/// The fake portal of a session.
pub struct Portal {
    calls: Calls,
}

impl Portal {
    pub(crate) async fn install(bus: &zbus::Connection, mode: PortalMode) -> Result<Self> {
        let calls = Calls::default();
        if mode != PortalMode::NoBackground {
            bus.object_server()
                .at(
                    "/org/freedesktop/portal/desktop",
                    Background {
                        calls: calls.clone(),
                        grant: mode == PortalMode::Grant,
                    },
                )
                .await?;
        }
        bus.request_name("org.freedesktop.portal.Desktop").await?;
        Ok(Self { calls })
    }

    /// Every `RequestBackground` call so far.
    pub fn calls(&self) -> Vec<BackgroundCall> {
        self.calls.lock().expect("calls lock").clone()
    }

    /// Waits for the `n`-th call (1-based).
    pub async fn wait_calls(&self, n: usize, timeout: Duration) -> Result<Vec<BackgroundCall>> {
        poll(timeout, "RequestBackground", || async move {
            let c = self.calls();
            (c.len() >= n).then_some(c)
        })
        .await
    }
}

struct Background {
    calls: Calls,
    grant: bool,
}

fn get<T: TryFrom<OwnedValue>>(map: &HashMap<String, OwnedValue>, key: &str) -> Option<T> {
    map.get(key)
        .and_then(|v| v.try_clone().ok())
        .and_then(|v| T::try_from(v).ok())
}

#[zbus::interface(name = "org.freedesktop.portal.Background")]
impl Background {
    async fn request_background(
        &self,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] conn: &zbus::Connection,
        _parent_window: String,
        options: HashMap<String, OwnedValue>,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        let sender = header.sender().map(|s| s.to_string()).unwrap_or_default();
        let token: String = get(&options, "handle_token").unwrap_or_else(|| "t".into());
        let autostart: bool = get(&options, "autostart").unwrap_or(false);
        let commandline: Vec<String> = get(&options, "commandline").unwrap_or_default();
        let reason: String = get(&options, "reason").unwrap_or_default();
        self.calls.lock().expect("calls lock").push(BackgroundCall {
            autostart,
            commandline,
            reason,
        });
        let path = format!(
            "/org/freedesktop/portal/desktop/request/{}/{token}",
            sender.trim_start_matches(':').replace('.', "_")
        );
        let object = OwnedObjectPath::try_from(path.clone())
            .map_err(|e| zbus::fdo::Error::InvalidArgs(e.to_string()))?;
        let conn = conn.clone();
        let grant = self.grant;
        // The answer comes after the reply, as from the real portal.
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let mut results: HashMap<&str, Value<'_>> = HashMap::new();
            results.insert("background", Value::from(grant));
            results.insert("autostart", Value::from(grant && autostart));
            let code: u32 = if grant { 0 } else { 1 };
            let _ = conn
                .emit_signal(
                    Some(sender.as_str()),
                    path.as_str(),
                    "org.freedesktop.portal.Request",
                    "Response",
                    &(code, results),
                )
                .await;
        });
        Ok(object)
    }

    #[zbus(property)]
    fn version(&self) -> u32 {
        2
    }
}
