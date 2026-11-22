//! Helpers for tests against the real app and an embedded fake server.

use std::time::Duration;

use serde_json::Value;
use tick_e2e::{Node, Query, Role, Session, SessionBuilder, State};
use tick_fake_server::{DEFAULT_TOKEN, FakeServer, LoggedRequest, seed};

use crate::WAIT;

/// Sunday 22 November 2026, 09:00 UTC: the fixed "now" of the app and the fake server.
pub const NOW: &str = "2026-11-22T09:00:00+00:00";

fn state() -> tick_fake_server::State {
    let mut st = tick_fake_server::State::new(DEFAULT_TOKEN);
    st.now = chrono::DateTime::parse_from_rfc3339(NOW).ok();
    st
}

/// A fake server with a preset (`demo`, `empty`, `bulk:N`).
pub async fn server(preset: &str) -> FakeServer {
    let mut st = state();
    seed::apply_preset(&mut st, preset, 1000).unwrap();
    FakeServer::start_with(0, st).await.unwrap()
}

/// A fake server seeded from a JSON fixture (see `docs/testing.md`).
pub async fn server_with(fixture: Value) -> FakeServer {
    let mut st = state();
    seed::apply_fixture(&mut st, &fixture).unwrap();
    FakeServer::start_with(0, st).await.unwrap()
}

/// A session pointed at `server`, with the fixed clock and a short sync interval.
pub fn session(server: &FakeServer) -> SessionBuilder {
    Session::builder()
        .api(server.base_url(), server.token())
        .now(NOW)
        .env("TICK_SYNC_INTERVAL_MS", "1000")
}

/// The sidebar list ("Views").
pub async fn sidebar(window: &Node) -> Node {
    window
        .wait_for(&Query::role(Role::List).name("Views").showing(), WAIT)
        .await
        .unwrap()
}

/// The task list ("Tasks").
pub async fn task_list(window: &Node) -> Node {
    window
        .wait_for(&Query::role(Role::List).name("Tasks").showing(), WAIT)
        .await
        .unwrap()
}

/// Clicks the sidebar row whose accessible name is `name` or starts with "`name`, ".
pub async fn open_view(s: &Session, window: &Node, name: &str) {
    let list = sidebar(window).await;
    let row = tick_e2e::poll(WAIT, &format!("sidebar row {name:?}"), || async {
        for n in list.find_all(&Query::role(Role::ListItem).showing()).await {
            let got = n.name().await.unwrap_or_default();
            if got == name || got.starts_with(&format!("{name}, ")) {
                return Some(n);
            }
        }
        None
    })
    .await
    .unwrap();
    s.click(&row).await.unwrap();
}

/// Names of the task rows currently in the task list (the rows GTK has built).
pub async fn task_names(window: &Node) -> Vec<String> {
    let list = task_list(window).await;
    let mut out = Vec::new();
    for n in list.find_all(&Query::role(Role::ListItem)).await {
        let Ok(states) = n.states().await else {
            continue;
        };
        // Task rows are focusable and hold a priority ring; headers and ghost rows do not.
        if states.contains(State::Focusable) && n.find(&Query::role(Role::CheckBox)).await.is_some()
        {
            out.push(n.name().await.unwrap_or_default());
        }
    }
    out
}

/// Waits until the task list shows exactly `want` (in order).
pub async fn wait_tasks(window: &Node, want: &[&str]) {
    let result = tick_e2e::poll(WAIT, &format!("tasks {want:?}"), || async {
        let got = task_names(window).await;
        (got == want).then_some(())
    })
    .await;
    if result.is_err() {
        panic!(
            "task list: {:?}, want {want:?}\n{}",
            task_names(window).await,
            task_list(window).await.dump().await
        );
    }
}

/// The task row named `title`.
pub async fn task_row(window: &Node, title: &str) -> Node {
    task_list(window)
        .await
        .wait_for(&Query::role(Role::ListItem).name(title), WAIT)
        .await
        .unwrap()
}

/// The first descendant of `row` matching `query`.
pub async fn inside(row: &Node, query: Query) -> Node {
    row.wait_for(&query, WAIT).await.unwrap()
}

/// Requests to `path` that the fake server answered.
pub fn requests(server: &FakeServer, path: &str) -> Vec<LoggedRequest> {
    server.requests_to(path)
}

/// `/sync` requests that carried a command of type `kind`.
pub fn commands_sent(server: &FakeServer, kind: &str) -> Vec<Value> {
    requests(server, "/api/v1/sync")
        .into_iter()
        .filter(|r| r.status == 200)
        .filter_map(|r| r.body.get("commands").cloned())
        .flat_map(|c| c.as_array().cloned().unwrap_or_default())
        .filter(|c| c["type"] == kind)
        .collect()
}

/// Waits for the app's first sync to finish (a request answered with 200).
pub async fn wait_first_sync(server: &FakeServer) {
    tick_e2e::poll(Duration::from_secs(20), "first sync", || async {
        requests(server, "/api/v1/sync")
            .iter()
            .any(|r| r.status == 200)
            .then_some(())
    })
    .await
    .unwrap();
}

/// Accessible names of every row the task list has built, in order (headers, tasks,
/// "Add task" rows).
pub async fn row_names(window: &Node) -> Vec<String> {
    let list = task_list(window).await;
    let mut out = Vec::new();
    for n in list.find_all(&Query::role(Role::ListItem)).await {
        out.push(n.name().await.unwrap_or_default());
    }
    out
}

/// Waits until the task list rows are exactly `want`.
pub async fn wait_rows(window: &Node, want: &[&str]) {
    let result = tick_e2e::poll(WAIT, &format!("rows {want:?}"), || async {
        (row_names(window).await == want).then_some(())
    })
    .await;
    if result.is_err() {
        panic!("rows: {:?}\nwant: {want:?}", row_names(window).await);
    }
}

/// The fixture shared by the view tests: two projects with sections and a sub-task,
/// labels, a local and a server-only filter, dates around [`NOW`].
pub fn fixture() -> Value {
    serde_json::json!({
        "projects": [
            {"id": "pw", "name": "Work", "color": "blue", "child_order": 1},
            {"id": "ph", "name": "Home", "color": "orange", "child_order": 2}
        ],
        "sections": [
            {"id": "s1", "project_id": "pw", "name": "This week", "section_order": 1},
            {"id": "s2", "project_id": "pw", "name": "Later", "section_order": 2}
        ],
        "labels": [
            {"id": "l1", "name": "phone", "item_order": 1},
            {"id": "l2", "name": "errand", "item_order": 2}
        ],
        "filters": [
            {"id": "f1", "name": "Urgent", "query": "p1", "item_order": 1},
            {"id": "f2", "name": "Before Monday", "query": "due before: 2026-11-23", "item_order": 2}
        ],
        "items": [
            {"id": "i1", "content": "Call the dentist", "priority": 3, "labels": ["phone"], "child_order": 1,
             "due": {"date": "2026-11-22", "string": "today", "is_recurring": false}},
            {"id": "i2", "content": "Renew passport", "priority": 4, "child_order": 2,
             "due": {"date": "2026-11-20", "string": "20 Nov", "is_recurring": false}},
            {"id": "i3", "content": "Read the article", "child_order": 3},
            {"id": "w1", "project_id": "pw", "section_id": "s1", "content": "Plan the deck",
             "priority": 4, "child_order": 1,
             "due": {"date": "2026-11-22T14:00:00", "string": "today 14:00", "is_recurring": false}},
            {"id": "w2", "project_id": "pw", "section_id": "s1", "parent_id": "w1",
             "content": "Collect metrics", "child_order": 1},
            {"id": "w3", "project_id": "pw", "section_id": "s2", "content": "Clean up flags",
             "labels": ["errand"], "child_order": 1},
            {"id": "w4", "project_id": "pw", "content": "Book the venue", "child_order": 1,
             "due": {"date": "2026-11-25", "string": "25 Nov", "is_recurring": false}},
            {"id": "h1", "project_id": "ph", "content": "Water the plants", "child_order": 1,
             "due": {"date": "2026-11-23", "string": "every monday", "is_recurring": true}},
            {"id": "h2", "project_id": "ph", "content": "Plant bulbs", "labels": ["errand"],
             "child_order": 2,
             "due": {"date": "2026-12-05", "string": "5 Dec", "is_recurring": false}}
        ]
    })
}
