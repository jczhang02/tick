//! E2E harness: the real `tick` binary in headless mutter, driven through AT-SPI and
//! mutter RemoteDesktop input. See `docs/testing.md` for the facts this relies on.
//!
//! A test starts a [`Session`] (private D-Bus bus, temp dirs, headless mutter, the app),
//! finds widgets with [`Query`] on the AT-SPI tree, and drives them with real input
//! ([`Input`]). A fake Todoist server plugs in through [`SessionBuilder::api`] (base URL +
//! token), which sets `TICK_API_BASE` and `TICK_TOKEN` for the app.
//!
//! Tests must panic on failure (`unwrap`/`expect`), not return `Err`: [`Session`]'s `Drop`
//! keeps the temp dir and prints the logs only while a panic unwinds.

mod a11y;
mod input;
pub mod keysym;
mod notify;
mod session;

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

pub use a11y::{A11y, Node, Query, Rect};
pub use atspi::{Role, State, StateSet};
pub use input::Input;
pub use notify::{Backend, Notification, NotificationButton, Notifications};
pub use session::{ColorScheme, Session, SessionBuilder};

/// Errors are `anyhow` errors: tests unwrap them, and the context chain is the report.
pub type Result<T, E = anyhow::Error> = std::result::Result<T, E>;

/// Global limit on concurrent sessions: `TICK_E2E_JOBS` slots (default 4).
fn job_slots() -> Arc<tokio::sync::Semaphore> {
    static SLOTS: OnceLock<Arc<tokio::sync::Semaphore>> = OnceLock::new();
    SLOTS
        .get_or_init(|| {
            let n = std::env::var("TICK_E2E_JOBS")
                .ok()
                .and_then(|v| v.parse::<usize>().ok())
                .filter(|n| *n > 0)
                .unwrap_or(4);
            Arc::new(tokio::sync::Semaphore::new(n))
        })
        .clone()
}

/// The cargo profile dir (`<target>/debug`) that holds the built `tick` binary.
///
/// Found by walking up from this test binary to the first directory that contains a
/// `tick` file. That covers both cargo layouts (test binaries in `<profile>/deps/` or in
/// `<profile>/build/<pkg>/<hash>/`) and a custom `CARGO_TARGET_DIR`. `TICK_BIN` overrides
/// it with its parent dir.
pub fn target_profile_dir() -> Result<PathBuf> {
    if let Some(bin) = std::env::var_os("TICK_BIN") {
        let bin = PathBuf::from(bin);
        return bin
            .parent()
            .map(PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("TICK_BIN has no parent dir"));
    }
    let exe = std::env::current_exe()?;
    exe.ancestors()
        .skip(1)
        .find(|dir| dir.join("tick").is_file())
        .map(PathBuf::from)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "no `tick` binary in any parent dir of {}; run `cargo build -p tick-gtk` first \
                 (or set TICK_BIN)",
                exe.display()
            )
        })
}

/// Path of the `tick` binary: `TICK_BIN`, else `<profile dir>/tick`.
pub fn tick_bin() -> Result<PathBuf> {
    match std::env::var_os("TICK_BIN") {
        Some(bin) => Ok(PathBuf::from(bin)),
        None => Ok(target_profile_dir()?.join("tick")),
    }
}

/// Path of an example binary of the workspace (for harness self-tests).
pub fn example_bin(name: &str) -> Result<PathBuf> {
    let bin = target_profile_dir()?.join("examples").join(name);
    anyhow::ensure!(
        bin.is_file(),
        "{} not found; run `cargo build -p tick-gtk --examples` first",
        bin.display()
    );
    Ok(bin)
}

/// Directory for design-review screenshots: `<target>/screenshots/`.
pub fn screenshot_dir() -> Result<PathBuf> {
    let profile = target_profile_dir()?;
    let target = profile
        .parent()
        .ok_or_else(|| anyhow::anyhow!("profile dir has no parent"))?;
    let dir = target.join("screenshots");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Polls `f` every 50 ms until it returns `Some`, or fails after `timeout`.
pub async fn poll<T, F, Fut>(timeout: std::time::Duration, what: &str, mut f: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Option<T>>,
{
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if let Some(v) = f().await {
            return Ok(v);
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!("timed out after {timeout:?} waiting for {what}");
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}
