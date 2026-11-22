# Tick: product specification

Tick is a fast native GNOME client for Todoist. It exists because the official
Todoist app starts slowly. Every requirement below was confirmed by the owner.
When a behavior is not specified here, match the Todoist web app.

## Product

1. Name "Tick", subtitle "A Todoist client", App ID `dev.chengrui.Tick`.
   Never use "Todoist" as the product name (trademark).
2. GTK4 + libadwaita with the native libadwaita header bar.
3. UI quality is the top priority: it must look beautiful and modern.
   `DESIGN.md` is the binding visual specification.

### Layout

- Three panes on a wide window: sidebar | task list | task detail.
- Narrower: the detail pane becomes an overlay. Narrowest: the sidebar collapses.
- Use `AdwNavigationSplitView` and `AdwOverlaySplitView`.

### Views (sidebar)

Inbox, Today, Upcoming (future tasks grouped by day), Projects (with sections and
sub-projects), Labels, Filters, Search, Completed history (read-only).

- Today includes overdue tasks (Todoist behavior).
- Project view is a list only (no board, no calendar). Tasks are grouped by section.
  Sub-tasks are indented and collapsible.

### Task fields (editable in the detail pane)

Title, description (rendered Markdown subset: bold, italic, code, links, lists;
click to edit as plain text), due date incl. recurrence, deadline, priority,
labels, sub-tasks, plain-text comments, assignee (shared projects).
Existing attachments are shown as links that open. No attachment upload, no duration.
The due field offers both a date picker and a free-text input (server parses the text).

### Management

- Projects: create, rename, color, favorite, archive, delete, drag to change nesting.
- Sections: create, rename, delete.
- Labels and filters: create, edit, delete.
- Tasks: drag to reorder within a list, drag onto a sidebar project to move.
- Completing or deleting a task shows a toast with "Undo" for about 5 s.

### Collaboration

Show assignee avatar and name. Allow assigning a task to a collaborator.
No inviting members, no workspace management.

### Quick add

- Online: the text goes to `POST /api/v1/tasks/quick` (server parses dates, `#project`, `@label`, `p1`).
- Offline: store the raw text; submit it to quick add when back online.
- Locally highlight `#project`, `@label`, `p1`-`p4` tokens while typing. Do not parse dates locally.

### Filters

Evaluate the filter query locally so the view opens instantly and works offline.
Supported subset: `&`, `|`, `!`, parentheses, `#project`, `##project` (with sub-projects),
`@label`, `p1`-`p4`, `today`, `overdue`, `no date`, `next N days`, `search:`, `assigned to`.
Unsupported syntax: use the server result of `GET /api/v1/tasks/filter` and cache it.

### Search

Local only, SQLite FTS5 over active task titles and descriptions, results while typing.
Completed tasks and comments are not searched.

### Completed history

On open, fetch the last 30 days from the server, with "Load more" at the bottom.
Cache the results; show the cache first next time, then refresh.

### Keyboard shortcuts

Todoist web single-key shortcuts, for example `q` quick add, `/` search, `g t` go to Today,
`e` complete task. Plus GNOME conventions: `Ctrl+?` shortcuts window, `Ctrl+Q` quit.

### Theme and color

- Accent color: the GNOME system accent. Priority colors (red, orange, blue) are fixed.
- Contrast is checked in both themes (WCAG AA).
- Settings offer "Follow system / Light / Dark"; the choice is remembered.

### Language

English and Simplified Chinese via gettext, following the system language.
Chinese copy uses ASCII punctuation with English spacing (see `DESIGN.md`).

## Data

1. Login: paste a personal API token on first launch; store it in the GNOME keyring (Secret Service).
   No OAuth.
2. Startup renders from the local SQLite cache before any network call.
   Automated test: release build, cache with 1000 tasks, launch to cached tasks visible < 300 ms
   (local machine; CI uses a looser budget, see `docs/testing.md`).
3. Offline editing: changes go into a persisted local queue and are submitted when online.
   On conflict the server wins. The UI shows the pending-change count; failed changes show an error.
4. Sync cadence: every 15 s while the window is visible; immediately on focus and after each change;
   every 5 min in the background.
5. Reminders: closing the window keeps the process running in the background to sync and fire
   desktop notifications with "Complete" and "Open" buttons. Optional "Run at login" switch.
   Time-based reminders only, no location reminders.

## Engineering

1. Crates: `tick-core` (no GTK), `tick-gtk` (binary `tick`), `tick-fake-server`, `tick-e2e`.
2. All tests run with `cargo test`. E2E tests start the real app in headless mutter, drive it
   through AT-SPI and mutter RemoteDesktop input, and assert on the fake server's state.
   Optional real-API smoke tests run only when `TODOIST_TEST_TOKEN` is set (separate test account).
3. Public repository `jczhang02/tick`, GPL-3.0-or-later. CI runs in an Arch Linux container;
   merging requires green CI.
4. Work lands as milestone PRs, squash-merged after CI passes. Conventional Commits.
5. `just install` builds a release binary and installs it, the `.desktop` file and icon into `~/.local`.
