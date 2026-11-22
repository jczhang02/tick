//! Hooks for the E2E harness (`tick-e2e`), active only with `TICK_TEST_MODE=1`.
//!
//! - `app.test-screenshot(s path)` renders the main window to a PNG file.
//! - The first frame prints `TICK_FIRST_DATA_FRAME_MS=<ms since process start>` to stderr.

use std::path::Path;

use adw::prelude::*;
use gtk::{gio, glib, graphene};

use crate::window::TickWindow;

/// True when the app runs under the E2E harness.
pub fn enabled() -> bool {
    std::env::var("TICK_TEST_MODE").is_ok_and(|v| v == "1")
}

/// Adds the test-only application actions.
pub fn install_actions(app: &adw::Application) {
    let screenshot = gio::ActionEntry::builder("test-screenshot")
        .parameter_type(Some(glib::VariantTy::STRING))
        .activate(|app: &adw::Application, _, param| {
            let Some(path) = param.and_then(|p| p.get::<String>()) else {
                return;
            };
            let Some(window) = app.windows().into_iter().next() else {
                eprintln!("test-screenshot: no window");
                return;
            };
            if let Err(err) = save_screenshot(&window, Path::new(&path)) {
                eprintln!("test-screenshot: {err}");
            }
        })
        .build();
    app.add_action_entries([screenshot]);
}

/// Renders `window` with its own renderer and writes a PNG to `path`.
///
/// The PNG is written to a temporary name first and then renamed, so a reader that
/// waits for `path` to exist never sees a partial file.
fn save_screenshot(window: &gtk::Window, path: &Path) -> Result<(), String> {
    let width = window.width();
    let height = window.height();
    if width <= 0 || height <= 0 {
        return Err("window has no size".into());
    }
    let paintable = gtk::WidgetPaintable::new(Some(window));
    let snapshot = gtk::Snapshot::new();
    paintable.snapshot(&snapshot, f64::from(width), f64::from(height));
    let node = snapshot.to_node().ok_or("empty render node")?;
    let renderer = window.renderer().ok_or("window has no renderer")?;
    let viewport = graphene::Rect::new(0.0, 0.0, width as f32, height as f32);
    let texture = renderer.render_texture(&node, Some(&viewport));
    let tmp = path.with_extension("png.part");
    texture.save_to_png(&tmp).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

/// Prints `TICK_FIRST_DATA_FRAME_MS` after the first frame that shows cached tasks: the
/// first paint after a task row of the open view was built, or the first paint at all when
/// the view (or the login page) has no tasks.
pub fn report_first_frame(window: &TickWindow) {
    window.connect_realize(|window| {
        let Some(clock) = window.frame_clock() else {
            return;
        };
        let handler = std::rc::Rc::new(std::cell::Cell::new(None));
        let handler_in = handler.clone();
        let weak = window.downgrade();
        let id = clock.connect_after_paint(move |clock| {
            if !weak.upgrade().is_some_and(|w| w.data_frame_ready()) {
                return;
            }
            if let Some(ms) = ms_since_process_start() {
                eprintln!("TICK_FIRST_DATA_FRAME_MS={ms}");
            }
            if let Some(id) = handler_in.take() {
                clock.disconnect(id);
            }
        });
        handler.set(Some(id));
    });
}

/// Milliseconds since the kernel started this process.
///
/// The start time comes from `/proc/self/stat` (clock ticks since boot, usually 10 ms
/// resolution) and is compared with `CLOCK_BOOTTIME`, the same clock the kernel uses.
fn ms_since_process_start() -> Option<u64> {
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    // Field 2 (comm) may contain spaces, so split after its closing parenthesis.
    // The rest starts at field 3; starttime is field 22.
    let rest = &stat[stat.rfind(')')? + 1..];
    let start_ticks: u64 = rest.split_whitespace().nth(19)?.parse().ok()?;
    // SAFETY: sysconf and clock_gettime only write to the provided out-pointer.
    let (ticks_per_sec, now) = unsafe {
        let tps = libc::sysconf(libc::_SC_CLK_TCK);
        let mut ts = std::mem::zeroed::<libc::timespec>();
        if libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut ts) != 0 {
            return None;
        }
        (tps, ts)
    };
    let ticks_per_sec = u64::try_from(ticks_per_sec).ok().filter(|t| *t > 0)?;
    let start_ms = start_ticks * 1000 / ticks_per_sec;
    let now_ms =
        u64::try_from(now.tv_sec).ok()? * 1000 + u64::try_from(now.tv_nsec).ok()? / 1_000_000;
    now_ms.checked_sub(start_ms)
}
