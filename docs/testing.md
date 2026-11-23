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
   - `Component.GrabFocus` returns `NotSupported` in GTK4: focus a widget with a real click.
     Action names are capitalized (a button has one action, "Click").
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
   - Drag: press, several relative moves with short sleeps, release. Verified with real
     `GtkDragSource`/`GtkDropTarget` (`fixture::drag_and_drop_between_widgets`) and in Tick's
     list and sidebar. `Input::press_at`, `drag_by` and `release` hold a drag open (a
     screenshot of a drag in progress). Observed, cause not settled: drops land a few pixels
     (about 6) away from the AT-SPI extents of the target row, and a clamp-and-jump before
     the release did not change where the drop landed. Aim well inside the target.
   - Menus (`popover.menu`): GTK 4.22 reports their items with an empty name; Tick names them
     from their text when the menu maps (`menus.rs`), so tests can find them by name. Tests
     may also use the keyboard: the first item has focus when a menu opens.
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
   - Observed with GLib 2.88 and a non-sandboxed app (portal not checked): `GNotification`
     goes to `org.gtk.Notifications` (`AddNotification`) when that name has an owner, and to
     `org.freedesktop.Notifications` (`Notify`) when it has none.
   - Invoking a button: for the gtk backend, call `org.freedesktop.Application.ActivateAction`
     on the app (what GNOME Shell does); for the freedesktop backend, emit `ActionInvoked`.
     A second `ActionInvoked` for the same freedesktop id had no effect; send a new
     notification for each invocation.
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
  `.locale(..)`, `.env(k, v)`, `.api(base_url, token)`, `.no_launch()`,
  `.fake_portal(PortalMode::Grant | Deny | NoBackground)`.
- Fake Background portal (opt-in): the harness owns `org.freedesktop.portal.Desktop` on the
  private bus and records `RequestBackground` calls (`Session::portal().calls()`); `Grant`
  answers `Response` 0 with what was asked, `Deny` answers 1, `NoBackground` owns the name
  without the interface (every call fails, like a portal without a Background backend). Opt-in,
  because owning the name also turns away the Settings portal calls GTK and libadwaita make.
- AT-SPI: libadwaita's switch rows report the "switch" role, which the `atspi` crate does not
  know (`Role` shows it as unknown): find them by name and the `checkable` state.
- Theme checks sample screenshot pixels (`png` dev-dependency) rather than a test hook.
- Harness self-tests that Tick cannot exercise yet (typing into an entry, notifications) run
  against `crates/tick-gtk/examples/e2e_fixture.rs`, a minimal GTK app that is not part of Tick.

## Fake server

`tick-fake-server` is an in-memory Todoist API v1 subset (axum). It is a library (embedded by
tests on a random port) and a binary (for manual runs). The app points at it with
`TICK_API_BASE=http://127.0.0.1:<port>`. Control endpoints under `/__test/` let tests seed
state, read state and the request log, and inject failures (offline, 429, 500, command errors).
Like the real API (`docs/api/pages/request-limits.md`), POST bodies over 1 MiB are refused; the fake
answers `413` with the usual error body (the real status is not documented).

The fake server and the client are written from the same docs, so a green E2E suite can still be
wrong about real Todoist. The live smoke tests are the only check against reality. Model the fake
on `docs/api/` (OpenAPI spec and doc pages), not on the client's assumptions.

Control endpoints (JSON bodies):

| Endpoint | Effect |
|---|---|
| `POST /__test/seed` | `{"preset": "demo" \| "empty" \| "bulk", "count": N}` or a fixture `{"token"?, "now"?, "user"?, "projects": [...], "items": [...], ..., "collaborator_states": [...], "completed": [...]}` (missing fields get documented defaults) |
| `GET /__test/state` | the whole account as JSON (including deleted objects and completion records) |
| `GET /__test/requests`, `DELETE /__test/requests` | request log (method, path, query, decoded body, status, `at_ms`: arrival time in ms since the server state was created) |
| `POST /__test/fail` | inject one failure or a list: `{"kind": "offline"}` / `{"kind": "online"}` (API connections are dropped), `{"kind": "status", "status": 429, "retry_after": 2, "count": 1, "path"?, "body_contains"?}` (`body_contains`: only requests whose decoded body contains the text), `{"kind": "error_after_commit", "status": 503}` (commands run, the response is an error), `{"kind": "command_error", "command_type": "item_add", "args_contain"?, "error"?}` |
| `DELETE /__test/fail` | clear injected failures and go online |
| `POST /__test/reset` | empty account (token and clock kept) |
| `POST /__test/now` | `{"now": "<RFC 3339>"}` fixes the server clock (`null` = system clock) |

The binary: `tick-fake-server [--port P] [--seed demo|empty|bulk:N] [--token T] [--launch]`
(default token `test-token`). `--launch` starts `target/debug/tick` (or `$TICK_BIN`) with
`TICK_API_BASE` and `TICK_TOKEN` set. The server honors `TICK_NOW` for its clock, and the
demo seed places due dates relative to it.

## Startup budget

The startup test measures from process start to the first frame that shows cached tasks
(1000 tasks). In test mode the app prints `TICK_FIRST_DATA_FRAME_MS=<n>` to stderr.
Budget: `TICK_STARTUP_BUDGET_MS`, default 300 locally (release build). CI sets 1500 because shared
runners render in software.

- `crates/tick-e2e/tests/e2e/startup.rs` seeds `bulk:1000`, launches once to fill the cache, then
  relaunches three times with the fake server offline and compares the median with the budget
  (single launches in headless mutter vary by about 25 ms).
- Without `TICK_STARTUP_BUDGET_MS` the default follows the binary: 300 ms for a release binary
  (`TICK_BIN=$PWD/target/release/tick`), 500 ms for a debug binary, so `just e2e` runs the same
  path without failing on an unoptimized build.
- Most of the time is GTK in headless mutter with software rendering: the first launch, whose
  view has no cached tasks yet, already reports about 250 ms.

## Screenshots for design review

`just screenshots` runs the E2E screenshot suite and writes PNGs to `target/screenshots/`:
each key view in light and dark, at 1280x800 (wide) and 600x800 (narrow; the harness starts
mutter with a 600x800 virtual monitor).
Files are named `<view>-<wide|narrow>-<light|dark>.png`, from the demo seed at the fixed clock
(`2026-11-22T09:00:00+00:00`): `today`, `inbox` (with a title long enough to wrap to three
lines and end in an ellipsis on narrow windows), `upcoming`, `upcoming-scrolled` (the header bar
title hand-off), `upcoming-later` (the rest of the fixed week), `upcoming-month` (a month
header), `project` (shared project with sections and sub-tasks), `project-cjk`, `label`,
`filter` (local), `filter-server` (server results), `selected` (a keyboard-focused, selected
row), `quick-add`, `quick-add-offline` (with a kept draft), `offline` (one queued change),
`sync-popover` (offline), `login`, `empty-today`, `empty-inbox`, and `sidebar` (narrow only).
Without `TICK_SCREENSHOTS=1` (`just e2e`) only the Today view is captured.

Editing (milestone 3), both schemes: `detail` (Markdown description, fields, sub-tasks),
`detail-scrolled` (comments), `date-picker`, `label-picker`, `project-dialog`, `dragging` (a
drag in progress: dimmed source row, drag icon, insertion line), `section-editor` and
`delete-project`, wide and narrow; wide only: `deadline-picker`, `priority-picker`,
`assignee-picker`, `move-picker`, `description-edit` (the raw-text editor, entered by a click
at the end of the description's first line), `task-menu`, `view-menu`, `title-rename` (F2 on
a scrolled list scrolls the title back into view), `label-dialog-duplicate`, `filter-dialog`,
`sidebar-menu`; a 1024 x 600 window:
`date-picker-short-<scheme>` (the calendar collapsed behind "Pick a Date") and
`date-picker-short-open-<scheme>` (after "Pick a Date": the popover grows to the window and
scrolls); in Chinese:
`detail`, `date-picker`, `delete-project` and `project-dialog` (`<view>-zh-wide-<scheme>`).
The drag icon is a surface of its own: in test mode the app remembers the `GtkDragIcon` and
the pointer position, and `app.test-screenshot` draws the icon there, its 20 px margin (room
for the floating shadow) included. Headless mutter shows no real drag surface, so the shot
checks Tick's icon widget, not the compositor's rendering of it.

Finding the pane's widgets: the list row previews a task's description (first line, Markdown
stripped), so a label query by description text can match the list row first. Query text that
only the pane shows (a later line), or pick the match furthest right.

The accessibility audit (`a11y.rs`) skips only links inside a label (exemption 1 in DESIGN
1.2.1) and only selectable labels for the 44 px rule (GTK 4.22 reports a selectable label with
`has-popup`, its Copy menu, not with a selectable state). Every other link and focusable label
is checked for a name and a 44 x 44 target.

Wide only, both schemes: `p4-hover` and `p4-checked` (a recurring p4 task in its
completed-pending state), `completing` (the 800 ms hold, kept with `TICK_TEST_HOLD=1`),
`failed-change` (a change the server rejected) and `sync-popover-failed`, `tooltip`, and
`main-menu`. The Chinese UI is captured as `<view>-zh-wide-<scheme>.png` (`today`, `inbox`,
`upcoming`, `quick-add-offline`, `empty-today`): the suite compiles `po/zh_CN.po` with
`msgfmt` and runs the app in `zh_CN.UTF-8` (the locale must be generated; CI runs
`locale-gen`).

- Popovers, menus and tooltips are surfaces of their own. `app.test-screenshot` draws every
  mapped popup native on top of the window at its compositor position (`gdk::Popup`
  position plus the surface transform).
- The pointer is parked in the middle of the header bar before each shot, away from rows
  (no hover tint) and from the scrolled window's edge (where GTK shows the scrollbar).
- The session's fontconfig maps `monospace` to Adwaita Mono, as on a GNOME desktop
  (the session has no GNOME settings, so libadwaita falls back to fontconfig's `monospace`).

Milestone 4 (both schemes, wide and narrow): `search-empty`, `search` (results), `search-none`,
`completed` (the demo history: day groups and "Load More"), `preferences`, `shortcuts`; wide
only: `completed-detail` (read-only pane) and `log-out` (the confirmation over preferences); in
Chinese (`<view>-zh-wide-<scheme>`): `search-empty`, `search`, `completed`, `preferences`,
`shortcuts`.

Not yet captured: a non-blue accent (the session's accent cannot be set yet).
