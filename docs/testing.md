# Testing

All tests run with `cargo test`. There is no Python and no pytest.

| Layer | Where | What |
|---|---|---|
| Unit | `#[cfg(test)]` in each crate | parsers, filter evaluator, store queries, queue logic |
| Integration | `crates/tick-core/tests/` | sync engine against an in-process `tick-fake-server` |
| E2E | `crates/tick-e2e/tests/` | the real `tick` binary in headless mutter, driven like a user |
| Live smoke | `crates/tick-e2e/tests/live.rs` | real Todoist API; runs only when `TODOIST_TEST_TOKEN` is set |

## E2E harness: verified facts

These were verified on the dev machine (mutter 50.5, GTK 4.22, libadwaita 1.9) and in a
`docker.io/library/archlinux:latest` container without a GPU. Do not re-litigate them.

1. **Isolated session.** Each E2E session gets its own private D-Bus session bus
   (`dbus-daemon --session --nofork --address=unix:path=$XDG_RUNTIME_DIR/bus`), its own
   `XDG_RUNTIME_DIR` (a fresh 0700 temp dir; without it gvfs and the document portal try to
   mount into the user's real runtime dir), and its own `HOME`, `XDG_CONFIG_HOME`,
   `XDG_DATA_HOME`, `XDG_CACHE_HOME`, `XDG_STATE_HOME`. Tests must never touch the user's real
   keyring or cache.
   - Start every session process from a cleared environment plus an allowlist (`PATH`,
     `XDG_DATA_DIRS`, debug variables). An inherited `DISPLAY`, `WAYLAND_DISPLAY`,
     `GTK_IM_MODULE` or `AT_SPI_BUS_ADDRESS` from the developer's desktop leaks into the test.
   - Set `GIO_USE_VFS=local`, `GIO_USE_VOLUME_MONITOR=unix` and `GVFS_DISABLE_FUSE=1`.
     Otherwise GIO D-Bus-activates gvfsd, gvfsd-fuse mounts `$XDG_RUNTIME_DIR/gvfs`, and the
     dead mount ("Transport endpoint is not connected") survives the session and blocks
     removing the temp dir. The harness also lazily unmounts (`fusermount3 -u -z`) any FUSE
     mount left below the session dir.
   - `GSETTINGS_BACKEND=memory` (no dconf) and `LC_ALL=C.UTF-8` (GTK's own accessible names
     such as "Close" are localized).
   - D-Bus-activated services are not direct children: spawn everything in one process group
     and kill the group.
2. **Compositor.** `mutter --headless --wayland --no-x11 --virtual-monitor 1280x800
   --wayland-display <unique-name>`, run on the private bus. Wait for the socket
   `$XDG_RUNTIME_DIR/<unique-name>` to appear, then for `org.gnome.Mutter.RemoteDesktop` to have
   an owner. Works without a GPU (software rendering).
3. **App.** Launch `tick` with `WAYLAND_DISPLAY=<unique-name>`, `DISPLAY` unset, and
   `TICK_TEST_MODE=1`. In test mode the main window is maximized, so window coordinates equal
   screen coordinates (origin 0,0 on the 1280x800 monitor).
4. **AT-SPI.** `org.a11y.Bus` is D-Bus-activated on the private bus automatically. The app appears
   on the AT-SPI registry with its binary name. Use the Rust `atspi` crate (zbus-based).
   Widget extents with `CoordType::Window` are reliable. GTK4 reports toggle buttons with the
   `pressed` state (not `checked`); the `focused` state is reported only for the focused widget.
   - The harness finds the app by PID (`GetConnectionUnixProcessID` on the a11y bus), so a
     relaunch is never confused with the previous process.
   - A `GtkEntry` has role `Text` (not `Entry`) with the `editable` state.
   - A hidden window (hide-on-close) drops out of the application's children; it does not just
     lose `showing`.
5. **Real input.** Use mutter's `org.gnome.Mutter.RemoteDesktop` on the private bus:
   `CreateSession` -> `Session.Start` -> `NotifyKeyboardKeysym(u keysym, b pressed)`,
   `NotifyPointerMotionRelative(d dx, d dy)`, `NotifyPointerButton(i button, b pressed)`
   (`BTN_LEFT = 0x110`). Absolute motion needs a ScreenCast stream; do not use it.
   - Clicking at (x, y): move relative by (-5000, -5000) to clamp the pointer at the top-left
     corner, then move relative by (x, y), then press and release.
   - The first events after `Start` can be lost while virtual devices are created. Sleep about
     1 s after `Start` and send a warm-up motion and a warm-up key (Shift press + release)
     before real input. Without the warm-up key the first typed character is lost.
   - Add about 50 ms between key press and release, and between keys.
   - Drag: press, several relative moves with short sleeps, release.
   - Verified: typing into an entry, clicking a button and the header bar close button,
     `Ctrl+Q`, Tab + Space.
   - **Non-ASCII text.** Mutter drops keysyms that are not in its (US) keymap and logs
     "No keycode found for keyval": Unicode keysyms (`0x01000000 + code point`) do not type
     CJK, and even `é` (0xe9) is dropped. The harness types non-ASCII characters through GTK's
     Unicode hex input instead: `Ctrl+Shift+U`, the hex code point, `space`. That only works
     with GTK's simple input method context, so the harness launches the app with
     `GTK_IM_MODULE=gtk-im-context-simple` (the default Wayland context ignores
     `Ctrl+Shift+U` under plain mutter). Fallback without key events: AT-SPI `EditableText`
     (`SetTextContents`, `InsertText`) works on GTK4 entries.
6. **Screenshots.** No screenshot D-Bus API exists in plain mutter. In test mode the app exports a
   `GAction` `app.test-screenshot` (parameter: output path string). It renders the window with
   `gtk::WidgetPaintable` + the window's `gsk::Renderer::render_texture` and writes a PNG.
   The harness activates it through `org.gtk.Actions.Activate` on the app's D-Bus object path.
7. **Notifications.** The harness owns `org.gtk.Notifications` and
   `org.freedesktop.Notifications` on the private bus with a recording fake, so reminder tests
   can assert on sent notifications and invoke their actions.
   - GLib 2.88 sends a non-sandboxed app's `GNotification` to `org.gtk.Notifications`
     (`AddNotification`) when that name has an owner; the portal backend is only used inside
     a sandbox. Without an `org.gtk.Notifications` owner it falls back to
     `org.freedesktop.Notifications` (`Notify`).
   - Invoking a button: for the gtk backend, call `org.freedesktop.Application.ActivateAction`
     on the app (what GNOME Shell does); for the freedesktop backend, emit `ActionInvoked`.
     After one `ActionInvoked`, GLib forgets that notification id.
8. **Secrets.** Tests must not use a Secret Service. In test mode the token comes from
   `TICK_TOKEN`; the keyring code path is unit-tested separately or tested with a fake
   `org.freedesktop.secrets` provided by the harness. (On a desktop machine gnome-keyring may
   still be D-Bus-activatable on the private bus, but it only sees the session's temp `HOME`.)
9. **Concurrency.** Each session runs a mutter instance. All E2E tests live in one test binary
   and share a global semaphore of `TICK_E2E_JOBS` slots (default 4).
10. **Noise.** xdg-desktop-portal warnings about RealtimeKit, PipeWire and secrets are expected.
11. **Binary lookup.** Cargo 1.100 places test binaries in `<profile>/build/<pkg>/<hash>/out/`,
    older cargo in `<profile>/deps/`. The harness walks up from the test binary to the first
    directory containing `tick` (override with `TICK_BIN`). `cargo test` does not build other
    packages' binaries: `just e2e` runs `cargo build -p tick-gtk --bins --examples` first.

## Using the harness

```rust
use tick_e2e::{Query, Role, Session, keysym};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn quits_on_ctrl_q() {
    let mut s = Session::builder()
        .api(server_url, "test-token") // TICK_API_BASE + TICK_TOKEN
        .start()
        .await
        .unwrap();
    let window = s.window().await.unwrap();
    let button = window.wait_for(&Query::role(Role::Button).name("Close"), WAIT).await.unwrap();
    s.click(&button).await.unwrap();
    s.chord(&[keysym::CONTROL_L, u32::from(b'q')]).await.unwrap();
    s.wait_app_exit(WAIT).await.unwrap();
}
```

- Tests unwrap instead of returning `Err`: a session dropped during a panic keeps its temp dir
  and prints the paths and log tails (`app.log`, `mutter.log`, `dbus.log`). `TICK_E2E_KEEP=1`
  always keeps it. CI uploads the logs of failed sessions as the `e2e-logs` artifact.
- Builder options: `.narrow()` (600x800), `.monitor(w, h)`, `.color_scheme(..)`, `.now(..)`,
  `.locale(..)`, `.env(k, v)`, `.api(base_url, token)`, `.no_launch()`.
- Harness self-tests that Tick cannot exercise yet (typing into an entry, notifications) run
  against `crates/tick-gtk/examples/e2e_fixture.rs`, a minimal GTK app that is not part of Tick.

## Fake server

`tick-fake-server` is an in-memory Todoist API v1 subset (axum). It is a library (embedded by
tests on a random port) and a binary (for manual runs). The app points at it with
`TICK_API_BASE=http://127.0.0.1:<port>`. Control endpoints under `/__test/` let tests seed
state, read state and the request log, and inject failures (offline, 429, 500, command errors).

The fake server and the client are written from the same docs, so a green E2E suite can still be
wrong about real Todoist. The live smoke tests are the only check against reality. Model the fake
on `docs/api/` (OpenAPI spec and doc pages), not on the client's assumptions.

## Startup budget

The startup test measures from process start to the first frame that shows cached tasks
(1000 tasks). In test mode the app prints `TICK_FIRST_DATA_FRAME_MS=<n>` to stderr.
Budget: `TICK_STARTUP_BUDGET_MS`, default 300 locally (release build). CI sets 1500 because shared
runners render in software.

## Screenshots for design review

`just screenshots` runs the E2E screenshot suite and writes PNGs to `target/screenshots/`:
each key view in light and dark, at 1280x800 (wide) and 600x800 (narrow; the harness starts
mutter with a 600x800 virtual monitor).
