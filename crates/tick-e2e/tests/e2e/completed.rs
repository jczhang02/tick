//! Completed history (`DESIGN.md` 4.21): the last 30 days from the server, "Load More" in
//! pages of 50 and then older windows, the cache shown first (offline here), read-only
//! detail, and reopening a task.

use std::time::Duration;

use serde_json::{Value, json};
use tick_e2e::{Node, Query, Role, Session};
use tick_fake_server::FakeServer;

use crate::detail::pane_title;
use crate::{WAIT, app};

/// `completed_at` `hours` before [`app::NOW`].
fn ago(hours: i64) -> String {
    (chrono::DateTime::parse_from_rfc3339(app::NOW).unwrap() - chrono::Duration::hours(hours))
        .with_timezone(&chrono::Utc)
        .format("%Y-%m-%dT%H:%M:%S%.6fZ")
        .to_string()
}

/// 55 completions in the last 30 days ("Done 00" newest, one every 6 hours from 1 hour
/// ago) and one 40 days ago ("Ancient task").
pub fn fixture() -> Value {
    let mut completed: Vec<Value> = (0..55)
        .map(|i| {
            json!({"id": format!("d{i:02}"), "content": format!("Done {i:02}"),
                   "project_id": "pw", "priority": 1, "checked": true,
                   "description": if i == 3 { "Notes from **Friday**" } else { "" },
                   "completed_at": ago(1 + 6 * i), "labels": [], "child_order": 1})
        })
        .collect();
    completed.push(
        json!({"id": "old1", "content": "Ancient task", "project_id": "pw",
                          "priority": 1, "checked": true, "completed_at": ago(24 * 40),
                          "labels": [], "child_order": 1}),
    );
    json!({
        "projects": [{"id": "pw", "name": "Work", "color": "blue", "child_order": 1}],
        "items": [{"id": "a1", "project_id": "pw", "content": "Still open", "child_order": 1}],
        "completed": completed,
    })
}

async fn start() -> (FakeServer, Session, Node) {
    let server = app::server_with(fixture()).await;
    let s = app::session(&server).start().await.unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    (server, s, window)
}

/// History requests the server answered.
fn history_requests(server: &FakeServer) -> Vec<tick_fake_server::LoggedRequest> {
    app::requests(server, "/api/v1/tasks/completed/by_completion_date")
        .into_iter()
        .filter(|r| r.status == 200)
        .collect()
}

/// Scroll the task list down until `query` shows up.
pub async fn scroll_to(s: &Session, window: &Node, query: &Query) -> Node {
    let list = app::task_list(window).await;
    let (x, y) = list.extents().await.unwrap().center();
    let input = s.input().await.unwrap();
    for _ in 0..60 {
        if let Some(n) = list.find(&query.clone().showing()).await {
            return n;
        }
        input.move_to(x, y).await.unwrap();
        input.scroll(5).await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("{query:?} not found:\n{}", list.dump().await);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn loads_pages_and_older_windows() {
    let (server, s, window) = start().await;
    app::open_view(&s, &window, "Completed").await;
    let list = app::task_list(&window).await;
    list.wait_for(&Query::role(Role::ListItem).name("Done 00"), WAIT)
        .await
        .unwrap();
    window
        .wait_for(
            &Query::role(Role::Label).name("Last 30 days").showing(),
            WAIT,
        )
        .await
        .unwrap();
    // Grouped by day, newest first: today, then yesterday.
    list.wait_for(&Query::role(Role::ListItem).name("Today"), WAIT)
        .await
        .unwrap();
    let first = history_requests(&server);
    assert_eq!(first.len(), 1, "{first:?}");
    assert!(first[0].query.contains("since=2026-10-23T09"), "{first:?}");
    assert!(!first[0].query.contains("cursor="));

    // Page 1 holds 50; "Load More" fetches the other 5 of the window.
    let more = scroll_to(&s, &window, &Query::role(Role::Button).name("Load More")).await;
    list.find(&Query::role(Role::ListItem).name("Done 49"))
        .await
        .expect("page 1 ends with Done 49");
    assert!(
        list.find(&Query::role(Role::ListItem).name("Done 50"))
            .await
            .is_none()
    );
    s.click(&more).await.unwrap();
    scroll_to(&s, &window, &Query::role(Role::ListItem).name("Done 54")).await;
    let pages = history_requests(&server);
    assert_eq!(pages.len(), 2, "{pages:?}");
    assert!(pages[1].query.contains("cursor="), "{pages:?}");

    // The window is done: the next "Load More" goes 30 days further back.
    let more = scroll_to(&s, &window, &Query::role(Role::Button).name("Load More")).await;
    s.click(&more).await.unwrap();
    scroll_to(
        &s,
        &window,
        &Query::role(Role::ListItem).name("Ancient task"),
    )
    .await;
    let pages = history_requests(&server);
    assert_eq!(pages.len(), 3, "{pages:?}");
    assert!(pages[2].query.contains("since=2026-09-23T09"), "{pages:?}");
    assert!(pages[2].query.contains("until=2026-10-23T09"), "{pages:?}");
    // Back at the top, the subtitle names the new start.
    s.input().await.unwrap().scroll(-200).await.unwrap();
    window
        .wait_for(
            &Query::role(Role::Label)
                .name("Since 23 September")
                .showing(),
            WAIT,
        )
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shows_the_cache_first() {
    let (server, mut s, window) = start().await;
    app::open_view(&s, &window, "Completed").await;
    app::task_list(&window)
        .await
        .wait_for(&Query::role(Role::ListItem).name("Done 00"), WAIT)
        .await
        .unwrap();
    s.activate_action("quit", None).await.unwrap();
    s.wait_app_exit(WAIT).await.unwrap();

    // Offline: the history comes from the cache alone (the view is restored at start; its
    // refresh request fails).
    server.set_offline(true);
    let before = history_requests(&server).len();
    s.launch_app().unwrap();
    let window = s.window().await.unwrap();
    let list = app::task_list(&window).await;
    list.wait_for(&Query::role(Role::ListItem).name("Done 00"), WAIT)
        .await
        .unwrap();
    window
        .wait_for(
            &Query::role(Role::Label)
                .name("Offline. Showing saved results.")
                .showing(),
            WAIT,
        )
        .await
        .unwrap();
    let rows = app::row_names(&window).await;
    assert!(rows.iter().any(|r| r == "Done 01"), "{rows:?}");
    assert_eq!(history_requests(&server).len(), before);

    // Back online: the view refreshes on its own and offers "Load More".
    server.set_offline(false);
    tick_e2e::poll(Duration::from_secs(20), "history refresh", || async {
        (history_requests(&server).len() > before).then_some(())
    })
    .await
    .unwrap();
    scroll_to(&s, &window, &Query::role(Role::Button).name("Load More")).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn read_only_detail_and_reopen() {
    let (server, s, window) = start().await;
    app::open_view(&s, &window, "Completed").await;
    let row = app::task_row(&window, "Done 03").await;
    // The ring is the disabled history ring.
    let ring = app::inside(&row, Query::role(Role::CheckBox).name("Completed")).await;
    assert!(!ring.has_state(tick_e2e::State::Enabled).await.unwrap());
    let r = row.extents().await.unwrap();
    s.input()
        .await
        .unwrap()
        .click(r.x + 120, r.y + 22)
        .await
        .unwrap();
    let title = pane_title(&window, "Done 03").await;
    assert!(
        !title.has_state(tick_e2e::State::Editable).await.unwrap(),
        "history titles are read-only"
    );
    // No editors: no comment composer, no "Add sub-task".
    assert!(
        window
            .find(&Query::role(Role::Button).name("Add sub-task").showing())
            .await
            .is_none()
    );
    // The pane's ring reopens it.
    let pane_ring = window
        .wait_for(
            &Query::role(Role::CheckBox)
                .name("Mark as not done")
                .showing(),
            WAIT,
        )
        .await
        .unwrap();
    s.click(&pane_ring).await.unwrap();
    tick_e2e::poll(WAIT, "item_uncomplete", || async {
        app::commands_sent(&server, "item_uncomplete")
            .into_iter()
            .find(|c| c["args"]["id"] == "d03")
    })
    .await
    .unwrap();
    // Gone from the history, open again in the project; the pane closed with its row.
    app::task_list(&window)
        .await
        .wait_gone(&Query::role(Role::ListItem).name("Done 03"), WAIT)
        .await
        .unwrap();
    window
        .wait_gone(
            &Query::role(Role::CheckBox)
                .name("Mark as not done")
                .showing(),
            WAIT,
        )
        .await
        .unwrap();
    crate::detail::wait_item(&server, "d03", "reopened", |v| v["checked"] == false).await;
    app::open_view(&s, &window, "Work").await;
    app::wait_tasks(&window, &["Still open", "Done 03"]).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reopen_from_the_row_menu() {
    let (server, s, window) = start().await;
    app::open_view(&s, &window, "Completed").await;
    let row = app::task_row(&window, "Done 01").await;
    let r = row.extents().await.unwrap();
    s.input()
        .await
        .unwrap()
        .right_click(r.x + 120, r.y + 22)
        .await
        .unwrap();
    let reopen = window
        .wait_for(&Query::role(Role::MenuItem).name("Reopen").showing(), WAIT)
        .await;
    let reopen = match reopen {
        Ok(n) => n,
        Err(_) => s
            .app()
            .await
            .unwrap()
            .wait_for(&Query::role(Role::MenuItem).name("Reopen").showing(), WAIT)
            .await
            .unwrap(),
    };
    s.click(&reopen).await.unwrap();
    tick_e2e::poll(WAIT, "item_uncomplete", || async {
        app::commands_sent(&server, "item_uncomplete")
            .into_iter()
            .find(|c| c["args"]["id"] == "d01")
    })
    .await
    .unwrap();
}

/// A task completed on this computer stays in the active data (`checked`): its history row
/// is still read-only and Reopen still sends `item_uncomplete`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_task_completed_here_is_read_only_in_the_history() {
    let server = app::server_with(fixture()).await;
    let s = app::session(&server)
        .env("TICK_TEST_UNDO_HOLD_MS", "200")
        .start()
        .await
        .unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    app::open_view(&s, &window, "Work").await;
    let row = app::task_row(&window, "Still open").await;
    s.click(&app::inside(&row, Query::role(Role::CheckBox).name("Complete task")).await)
        .await
        .unwrap();
    crate::detail::wait_item(&server, "a1", "completed", |v| v["checked"] == true).await;
    app::open_view(&s, &window, "Completed").await;
    let row = app::task_row(&window, "Still open").await;
    let r = row.extents().await.unwrap();
    s.input()
        .await
        .unwrap()
        .click(r.x + 120, r.y + 22)
        .await
        .unwrap();
    let title = pane_title(&window, "Still open").await;
    assert!(!title.has_state(tick_e2e::State::Editable).await.unwrap());
    s.input()
        .await
        .unwrap()
        .right_click(r.x + 120, r.y + 22)
        .await
        .unwrap();
    let reopen = s
        .app()
        .await
        .unwrap()
        .wait_for(&Query::role(Role::MenuItem).name("Reopen").showing(), WAIT)
        .await
        .unwrap();
    crate::a11y::check(&s.app().await.unwrap(), "history row menu").await;
    s.click(&reopen).await.unwrap();
    tick_e2e::poll(WAIT, "item_uncomplete a1", || async {
        app::commands_sent(&server, "item_uncomplete")
            .into_iter()
            .find(|c| c["args"]["id"] == "a1")
    })
    .await
    .unwrap();
    crate::detail::wait_item(&server, "a1", "reopened", |v| v["checked"] == false).await;
}

/// A history request the server fails: a toast says so and its "Retry" asks again (4.21).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_failed_load_offers_retry() {
    let (server, s, window) = start().await;
    let path = "/api/v1/tasks/completed/by_completion_date";
    server.inject(tick_fake_server::Failure::Status {
        status: 500,
        retry_after: None,
        count: 1,
        path: Some(path.into()),
        body_contains: None,
    });
    app::open_view(&s, &window, "Completed").await;
    window
        .wait_for(
            &Query::role(Role::Label)
                .name("Couldn't load completed tasks")
                .showing(),
            WAIT,
        )
        .await
        .unwrap();
    let failed = app::requests(&server, path);
    assert_eq!(failed.len(), 1, "{failed:?}");
    assert_eq!(failed[0].status, 500);
    let list = app::task_list(&window).await;
    assert!(
        list.find(&Query::role(Role::ListItem).name("Done 00"))
            .await
            .is_none()
    );
    let retry = window
        .wait_for(&Query::role(Role::Button).name("Retry").showing(), WAIT)
        .await
        .unwrap();
    s.click(&retry).await.unwrap();
    // After the backoff of the 500, the request goes out again and the rows come.
    list.wait_for(&Query::role(Role::ListItem).name("Done 00"), WAIT)
        .await
        .unwrap();
    let all = app::requests(&server, path);
    assert_eq!(all.len(), 2, "{all:?}");
    assert_eq!(history_requests(&server).len(), 1);
}
