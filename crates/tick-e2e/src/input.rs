//! Real input through mutter's `org.gnome.Mutter.RemoteDesktop` on the private bus.

use std::time::Duration;

use anyhow::Context as _;
use tokio::sync::Mutex;
use zbus::zvariant::OwnedObjectPath;

use crate::{Result, keysym};

const BTN_LEFT: i32 = 0x110;
const BTN_RIGHT: i32 = 0x111;
/// Pause between key press/release and between keys (docs/testing.md).
const KEY_DELAY: Duration = Duration::from_millis(50);
/// Relative motion that clamps the pointer to the top-left corner of the monitor.
const CLAMP: f64 = -5000.0;
const SESSION_IFACE: &str = "org.gnome.Mutter.RemoteDesktop.Session";

/// A started RemoteDesktop session. Events are serialized: concurrent callers would
/// interleave presses and releases.
pub struct Input {
    bus: zbus::Connection,
    path: OwnedObjectPath,
    lock: Mutex<()>,
}

impl Input {
    /// Creates and starts a RemoteDesktop session, then warms up the virtual devices.
    pub async fn connect(bus: &zbus::Connection) -> Result<Self> {
        let reply = bus
            .call_method(
                Some("org.gnome.Mutter.RemoteDesktop"),
                "/org/gnome/Mutter/RemoteDesktop",
                Some("org.gnome.Mutter.RemoteDesktop"),
                "CreateSession",
                &(),
            )
            .await
            .context("RemoteDesktop.CreateSession")?;
        let path: OwnedObjectPath = reply.body().deserialize()?;
        let input = Self {
            bus: bus.clone(),
            path,
            lock: Mutex::new(()),
        };
        input.call("Start", &()).await.context("Session.Start")?;
        // Events right after Start can be lost while mutter creates the virtual devices,
        // and the first key event is dropped even after the sleep: warm up both devices.
        tokio::time::sleep(Duration::from_secs(1)).await;
        input
            .call("NotifyPointerMotionRelative", &(1.0f64, 1.0f64))
            .await?;
        input.keysym(keysym::SHIFT_L, true).await?;
        input.keysym(keysym::SHIFT_L, false).await?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(input)
    }

    async fn call<B>(&self, method: &str, body: &B) -> Result<()>
    where
        B: serde::Serialize + zbus::zvariant::DynamicType,
    {
        self.bus
            .call_method(
                Some("org.gnome.Mutter.RemoteDesktop"),
                &self.path,
                Some(SESSION_IFACE),
                method,
                body,
            )
            .await
            .with_context(|| format!("RemoteDesktop.Session.{method}"))?;
        Ok(())
    }

    async fn motion(&self, dx: f64, dy: f64) -> Result<()> {
        self.call("NotifyPointerMotionRelative", &(dx, dy)).await
    }

    async fn button(&self, button: i32, pressed: bool) -> Result<()> {
        self.call("NotifyPointerButton", &(button, pressed)).await
    }

    async fn keysym(&self, keysym: u32, pressed: bool) -> Result<()> {
        self.call("NotifyKeyboardKeysym", &(keysym, pressed)).await
    }

    /// Moves the pointer to (x, y): clamp to the top-left corner, then move relative.
    pub async fn move_to(&self, x: i32, y: i32) -> Result<()> {
        let _guard = self.lock.lock().await;
        self.move_to_locked(x, y).await
    }

    async fn move_to_locked(&self, x: i32, y: i32) -> Result<()> {
        self.motion(CLAMP, CLAMP).await?;
        tokio::time::sleep(Duration::from_millis(20)).await;
        self.motion(f64::from(x), f64::from(y)).await?;
        tokio::time::sleep(Duration::from_millis(50)).await;
        Ok(())
    }

    /// Left click at (x, y) in screen coordinates.
    pub async fn click(&self, x: i32, y: i32) -> Result<()> {
        self.click_button(x, y, BTN_LEFT).await
    }

    /// Right click at (x, y).
    pub async fn right_click(&self, x: i32, y: i32) -> Result<()> {
        self.click_button(x, y, BTN_RIGHT).await
    }

    async fn click_button(&self, x: i32, y: i32, button: i32) -> Result<()> {
        let _guard = self.lock.lock().await;
        self.move_to_locked(x, y).await?;
        self.button(button, true).await?;
        tokio::time::sleep(KEY_DELAY).await;
        self.button(button, false).await?;
        tokio::time::sleep(KEY_DELAY).await;
        Ok(())
    }

    /// Drags with the left button from `from` to `to`, in several intermediate moves so
    /// GTK's drag threshold and drop targets see the motion.
    pub async fn drag(&self, from: (i32, i32), to: (i32, i32)) -> Result<()> {
        const STEPS: i32 = 20;
        let _guard = self.lock.lock().await;
        self.move_to_locked(from.0, from.1).await?;
        self.button(BTN_LEFT, true).await?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        let (dx, dy) = (
            f64::from(to.0 - from.0) / f64::from(STEPS),
            f64::from(to.1 - from.1) / f64::from(STEPS),
        );
        for _ in 0..STEPS {
            self.motion(dx, dy).await?;
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        // Observed: drops land a few pixels away from the target's AT-SPI extents, and a
        // clamp-and-jump before the release did not change that (cause not settled). Callers
        // aim well inside the target.
        tokio::time::sleep(Duration::from_millis(100)).await;
        self.button(BTN_LEFT, false).await?;
        tokio::time::sleep(KEY_DELAY).await;
        Ok(())
    }

    /// Presses the left button at (x, y) and keeps it down: the start of a drag that a test
    /// wants to observe midway (see [`Input::drag_by`] and [`Input::release`]).
    pub async fn press_at(&self, x: i32, y: i32) -> Result<()> {
        let _guard = self.lock.lock().await;
        self.move_to_locked(x, y).await?;
        self.button(BTN_LEFT, true).await?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(())
    }

    /// Moves the pointer by (dx, dy) in `steps` relative moves with short pauses, so drag
    /// thresholds and drop targets see the motion (the button state is left alone).
    pub async fn drag_by(&self, dx: i32, dy: i32, steps: i32) -> Result<()> {
        let _guard = self.lock.lock().await;
        let steps = steps.max(1);
        let (sx, sy) = (
            f64::from(dx) / f64::from(steps),
            f64::from(dy) / f64::from(steps),
        );
        for _ in 0..steps {
            self.motion(sx, sy).await?;
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(())
    }

    /// Releases the left button where the pointer is (the end of a drag).
    pub async fn release(&self) -> Result<()> {
        let _guard = self.lock.lock().await;
        self.button(BTN_LEFT, false).await?;
        tokio::time::sleep(KEY_DELAY).await;
        Ok(())
    }

    /// Scrolls by discrete wheel steps at the current pointer position
    /// (positive = down).
    pub async fn scroll(&self, steps: i32) -> Result<()> {
        let _guard = self.lock.lock().await;
        // Axis 0 is vertical.
        self.call("NotifyPointerAxisDiscrete", &(0u32, steps))
            .await?;
        tokio::time::sleep(KEY_DELAY).await;
        Ok(())
    }

    /// Presses and releases one key.
    pub async fn key(&self, keysym: u32) -> Result<()> {
        self.chord(&[keysym]).await
    }

    /// Presses `keysyms` in order and releases them in reverse, e.g.
    /// `[keysym::CONTROL_L, 'q' as u32]`.
    pub async fn chord(&self, keysyms: &[u32]) -> Result<()> {
        let _guard = self.lock.lock().await;
        for &k in keysyms {
            self.keysym(k, true).await?;
            tokio::time::sleep(KEY_DELAY).await;
        }
        for &k in keysyms.iter().rev() {
            self.keysym(k, false).await?;
            tokio::time::sleep(KEY_DELAY).await;
        }
        Ok(())
    }

    /// Types `text` with real key events.
    ///
    /// ASCII characters are sent as their keysyms. Mutter drops keysyms that are not in
    /// its US keymap (CJK, accented letters), so every other character is entered the way
    /// a user without an input method would: GTK's Unicode hex input, `Ctrl+Shift+U`,
    /// the hex code point, `space`. This needs the app to use GTK's simple input method
    /// context, which the session sets by default (`GTK_IM_MODULE`).
    pub async fn type_text(&self, text: &str) -> Result<()> {
        for c in text.chars() {
            if c.is_ascii() {
                self.key(keysym::from_char(c)).await?;
            } else {
                self.chord(&[keysym::CONTROL_L, keysym::SHIFT_L, u32::from(b'u')])
                    .await?;
                for digit in format!("{:x}", u32::from(c)).chars() {
                    self.key(keysym::from_char(digit)).await?;
                }
                self.key(keysym::SPACE).await?;
            }
        }
        Ok(())
    }
}
