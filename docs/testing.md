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
   (spawn `dbus-daemon --session --print-address` or `dbus-run-session`), its own
   `XDG_RUNTIME_DIR` (a fresh 0700 temp dir; without it gvfs and the document portal try to
   mount into the user's real runtime dir), and its own `HOME`, `XDG_CONFIG_HOME`,
   `XDG_DATA_HOME`, `XDG_CACHE_HOME`. Tests must never touch the user's real keyring or cache.
2. **Compositor.** `mutter --headless --wayland --no-x11 --virtual-monitor 1280x800
   --wayland-display <unique-name>`, run on the private bus. Wait for the socket
   `$XDG_RUNTIME_DIR/<unique-name>` to appear. Works without a GPU (software rendering).
3. **App.** Launch `tick` with `WAYLAND_DISPLAY=<unique-name>`, `DISPLAY` unset, and
   `TICK_TEST_MODE=1`. In test mode the main window is maximized, so window coordinates equal
   screen coordinates (origin 0,0 on the 1280x800 monitor).
4. **AT-SPI.** `org.a11y.Bus` is D-Bus-activated on the private bus automatically. The app appears
   on the AT-SPI registry with its binary name. Use the Rust `atspi` crate (zbus-based).
   Widget extents with `CoordType::Window` are reliable. GTK4 reports toggle buttons with the
   `pressed` state (not `checked`); the `focused` state is reported only for the focused widget.
5. **Real input.** Use mutter's `org.gnome.Mutter.RemoteDesktop` on the private bus:
   `CreateSession` -> `Session.Start` -> `NotifyKeyboardKeysym(u keysym, b pressed)`,
   `NotifyPointerMotionRelative(d dx, d dy)`, `NotifyPointerButton(i button, b pressed)`
   (`BTN_LEFT = 0x110`). Absolute motion needs a ScreenCast stream; do not use it.
   - Clicking at (x, y): move relative by (-5000, -5000) to clamp the pointer at the top-left
     corner, then move relative by (x, y), then press and release.
   - The first events after `Start` can be lost while virtual devices are created. Sleep about
     1 s after `Start` and send a warm-up motion before real input.
   - Add about 50 ms between key press and release, and between keys.
   - Drag: press, several relative moves with short sleeps, release.
   - Verified: typing into an entry, clicking a toggle button, Tab + Space.
6. **Screenshots.** No screenshot D-Bus API exists in plain mutter. In test mode the app exports a
   `GAction` `app.test-screenshot` (parameter: output path string). It renders the window with
   `gtk::WidgetPaintable` + the window's `gsk::Renderer::render_texture` and writes a PNG.
   The harness activates it through `org.gtk.Actions.Activate` on the app's D-Bus object path.
7. **Notifications.** The harness owns `org.gtk.Notifications` and
   `org.freedesktop.Notifications` on the private bus with a recording fake, so reminder tests
   can assert on sent notifications and invoke their actions.
8. **Secrets.** There is no Secret Service on the private bus. In test mode the token comes from
   `TICK_TOKEN`; the keyring code path is unit-tested separately or tested with a fake
   `org.freedesktop.secrets` provided by the harness.
9. **Concurrency.** Each session runs a mutter instance. All E2E tests live in one test binary
   and share a global semaphore of `TICK_E2E_JOBS` slots (default 4).
10. **Noise.** xdg-desktop-portal warnings about RealtimeKit, PipeWire and secrets are expected.

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

## Screenshots for design review

`just screenshots` runs the E2E screenshot suite and writes PNGs to `target/screenshots/`:
each key view in light and dark, at 1280x800 (wide) and 600x800 (narrow; the harness starts
mutter with a 600x800 virtual monitor).
