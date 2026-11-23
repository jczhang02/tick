# Architecture

## Crates

| Crate | Depends on | Role |
|---|---|---|
| `tick-core` | no GTK | Todoist API v1 types, HTTP client, SQLite store, offline command queue, sync engine, filter parser and evaluator, quick-add token highlighter, search |
| `tick-fake-server` | axum | in-memory Todoist API v1 subset for tests (lib + bin) |
| `tick-gtk` | `tick-core`, gtk4, libadwaita | the `tick` binary: all UI |
| `tick-e2e` | `tick-fake-server`, `atspi`, zbus | E2E harness and tests (no GTK) |

`tick-core` must never depend on GTK, so the UI layer stays replaceable.

## Data flow

1. Startup: open the SQLite cache synchronously on the main thread, build the list models, show the
   window. No network call happens before the first frame.
2. A sync engine runs on a tokio runtime in a background thread. It owns the HTTP client and a
   second SQLite connection (WAL mode, so main-thread reads never block on writes).
3. UI -> engine: user edits become Sync API commands (`uuid`, optional `temp_id`). They are applied
   optimistically to the local store and appended to the persisted queue in the same transaction.
4. Engine -> UI: after applying a sync response, the engine sends a change event (affected object
   kinds and ids) over an `async-channel`; the UI receives it with `glib::spawn_future_local` and
   refreshes the affected models.
5. Sync request: push queued commands (batches of at most 100 commands and about 900 KiB
   form-encoded, below the documented 1 MiB body limit) together with the incremental read
   (`sync_token`). Resolve `temp_id_mapping`: rewrite still-queued commands and remember the
   mapping (`temp_ids` table), so a command enqueued later with the old temp id is rewritten
   too; change events carry the mapping (`Changes::id_map`) so views can switch ids. A `400`
   command error is terminal: drop the command, roll back its optimistic change by re-reading
   server state, and surface the error. `429` and `5xx`: back off and retry; a command still
   failing after 10 answered attempts is dropped the same way. A request refused as a whole
   (`400`/`403`/`413`): if a request without commands passes, bisect the batch and drop the one
   command refused on its own; otherwise keep the queue and back off. Network failure: mark
   offline, keep the queue. Inside a backoff window no request goes out (history and filter
   requests wait for the window to end).
   Completing a task is enqueued with a send delay (`Store::enqueue_deferred`), so "Undo" can
   still discard it; commands behind it wait to keep the order. The UI holds every completion
   its visible "Undo" toast covers (`Store::defer_queued`, at most 60 s, renewed when the toast
   grows) and releases them when the toast goes away. `Store::discard_queued` returns `None`
   for a command that already left; Undo then queues `item_uncomplete`. A close discarded while
   its request was on the wire is answered "ok" with no queue row left: the store queues
   `item_uncomplete` for it (non-recurring tasks).
6. Server wins on conflict: incremental read results overwrite local rows, then still-queued
   commands are re-applied on top.

## Store

- SQLite via `rusqlite` (bundled feature off; system SQLite with FTS5).
- Path: `$XDG_DATA_HOME/tick/cache.sqlite3`. Settings (theme choice, run at login, last view) live
  in a `settings` table so they are available before the first paint. Keys: `color-scheme`
  (`system`, `light` or `dark`), `run-at-login` (`true` or `false`, saved once the Background
  portal granted it), `last-view` (`today`, `inbox`, `upcoming`, `completed`, `project:<id>`,
  `label:<id>`, `filter:<id>`; Search is not restored), `sidebar-collapsed` and
  `collapsed-sections` (comma-separated).
- Log out (`Store::clear_account`): every cached object, the queue, failed changes, the history
  and filter caches, reminder state and the settings except `color-scheme` and `run-at-login`.
  The UI stops the engine first (its event stream ends) and clears after that; the token is
  removed from the keyring on a worker thread (not in test mode).
- Keep raw server JSON per object alongside the indexed columns, so unknown fields survive and
  future features need no migration.
- Schema migrations are versioned with `PRAGMA user_version`.
- Write transactions are `BEGIN IMMEDIATE`: under WAL a deferred transaction that reads first and
  writes later fails with `SQLITE_BUSY` (no busy wait) when the other connection committed in
  between. Both connections wait up to 5 s for the write lock (`Store::set_busy_timeout`).
- The API token comes from the keyring on a worker thread, never on the main thread (an unlock
  prompt can block indefinitely).

## Configuration through the environment

| Variable | Effect |
|---|---|
| `TICK_API_BASE` | API base URL, default `https://api.todoist.com`. Tests point it at the fake server. |
| `TICK_TOKEN` | Use this token instead of the keyring (tests, CI). |
| `TICK_TEST_MODE=1` | Maximize the window, export `app.test-screenshot`, print `TICK_FIRST_DATA_FRAME_MS`, disable animations. |
| `TICK_NOW` | Fixed "now" (RFC 3339) for deterministic Today/Upcoming and screenshots. |
| `TICK_SYNC_INTERVAL_MS` | Override the 15 s foreground sync interval (tests use a short one). |
| `TICK_BACKGROUND_SYNC_MS` | Override the 5 min sync interval while the window is hidden (tests). |
| `TICK_COLOR_SCHEME` | `light` or `dark`, overrides the saved theme (screenshots). |
| `TICK_LOCALEDIR` | Where the `tick.mo` catalogs are found at run time (default: the build-time `TICK_LOCALEDIR`, else `/usr/share/locale`). |

## UI structure (tick-gtk)

- `AdwApplication` -> `TickWindow` (`AdwApplicationWindow`).
- Layout: `AdwNavigationSplitView` (sidebar | content), the content holds an `AdwOverlaySplitView`
  (task list | task detail). Breakpoints switch forms as the window narrows.
- Lists use `gtk::ListView` with `gio::ListStore` of GObject wrappers (`TaskObject`, ...), so only
  visible rows are built.
- Layouts are written in Blueprint (`data/ui/*.blp`), compiled by `build.rs`.
- Every interactive control has an accessible name (`accessible-label` or a label relation). E2E
  tests find widgets by role and accessible name, so names are part of the test contract.
- Background running: on window close, hide the window and keep the application held. `Ctrl+Q`
  and the menu "Quit" exit. While the window is hidden the engine syncs every 5 min. "Run at
  login" uses the Background portal (`RequestBackground` with `autostart` and the command line
  `tick --background`, `portal.rs`); `--background` builds the window without showing it and
  starts the token lookup and the engine at once (there is no first paint to wait for). A
  second launch shows the window.

## Reminders

- When a reminder fires is decided in tick-core (`reminders.rs`): `absolute` reminders at their
  `due.date` (UTC with `Z`, else local wall clock), `relative` ones `minute_offset` minutes before
  the task's due time (no due time: never). Location reminders, reminders for another user
  (`notify_uid`) and reminders of completed tasks are skipped.
- Each firing is remembered in `fired_reminders` (schema v3) by reminder id and instant, so it
  notifies once across syncs and restarts, and a reminder whose instant moves (a recurring task's
  next date, an edit) fires again. Keys older than 14 days are pruned.
- Grace window: a reminder whose instant passed while Tick was not running is shown at the next
  start (or first sync) when it is at most 60 minutes late; older ones are dropped silently.
- The window checks at start (from the cache), after every change of reminders, tasks or the
  user, and on a timer armed for the next instant (at most a minute ahead).
- `user_settings.reminder_desktop = false` turns desktop notifications off. Accounts without
  reminders (free plan) simply have none.
- Notifications go through `GApplication::send_notification` (id `reminder-<id>`) with
  "Complete" (`app.reminder-complete`, task id) and "Open" (`app.reminder-open`, also the default
  action). Complete queues `item_close` at once (no Undo toast: the window may be hidden) and
  syncs; Open presents the window with the task in the detail pane. Both withdraw the task's
  notifications.

## Dates and time

Use `chrono` with the local time zone. "Now" comes from one clock function that honors `TICK_NOW`.
Recurring tasks: completing one sends `item_close`; the server computes the next date. Until the
sync returns, the UI shows the task as completed-pending.
