//! Each view shows exactly the right rows for the shared fixture (`app::fixture`) at the
//! fixed clock (`app::NOW`, Sunday 22 November 2026).
//!
//! The sessions use a tall virtual monitor so `GtkListView` builds every row of a view.

use tick_e2e::{Node, Query, Role, Session};
use tick_fake_server::FakeServer;

use crate::{WAIT, app};

async fn start() -> (FakeServer, Session, Node) {
    let server = app::server_with(app::fixture()).await;
    let s = app::session(&server)
        .monitor(1280, 2000)
        .start()
        .await
        .unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    (server, s, window)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn today_includes_overdue() {
    let (_server, _s, window) = start().await;
    // Today is the default view.
    app::wait_rows(
        &window,
        &[
            "Today",
            "Overdue",
            "Renew passport",
            "Today",
            "Plan the deck",
            "Call the dentist",
            "Add task",
        ],
    )
    .await;
    // The meta line is the row description: Today shows times only, overdue its date.
    let deck = app::task_row(&window, "Plan the deck").await;
    assert_eq!(
        deck.description().await.unwrap(),
        "Due 14:00, 0 of 1 sub-task, project Work / This week"
    );
    let passport = app::task_row(&window, "Renew passport").await;
    assert_eq!(
        passport.description().await.unwrap(),
        "Due 20 Nov, project Inbox"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn inbox_label_and_local_filter() {
    let (_server, s, window) = start().await;
    app::wait_tasks(
        &window,
        &["Renew passport", "Plan the deck", "Call the dentist"],
    )
    .await;

    app::open_view(&s, &window, "Inbox").await;
    app::wait_rows(
        &window,
        &[
            "Inbox",
            "Call the dentist",
            "Renew passport",
            "Read the article",
            "Add task",
        ],
    )
    .await;

    app::open_view(&s, &window, "phone").await;
    app::wait_rows(&window, &["phone", "Call the dentist"]).await;

    app::open_view(&s, &window, "Urgent").await;
    app::wait_rows(&window, &["Urgent", "Renew passport", "Plan the deck"]).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn upcoming_groups_by_day() {
    let (_server, s, window) = start().await;
    app::wait_tasks(
        &window,
        &["Renew passport", "Plan the deck", "Call the dentist"],
    )
    .await;
    app::open_view(&s, &window, "Upcoming").await;
    app::wait_rows(
        &window,
        &[
            "Upcoming",
            "Overdue",
            "Renew passport",
            "Today",
            "Plan the deck",
            "Call the dentist",
            "Add task",
            "Tomorrow",
            "Water the plants",
            "Add task",
            "Tuesday",
            "Add task",
            "Wednesday",
            "Book the venue",
            "Add task",
            "Thursday",
            "Add task",
            "Friday",
            "Add task",
            "Saturday",
            "Add task",
            "December",
            "Saturday",
            "Plant bulbs",
            "Add task",
        ],
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn project_sections_and_subtasks() {
    let (server, s, window) = start().await;
    app::wait_tasks(
        &window,
        &["Renew passport", "Plan the deck", "Call the dentist"],
    )
    .await;
    app::open_view(&s, &window, "Work").await;
    app::wait_rows(
        &window,
        &[
            "Work",
            "Book the venue",
            "Add task",
            "This week, 2 tasks, expanded",
            "Plan the deck",
            "Collect metrics",
            "Add task",
            "Later, 1 task, expanded",
            "Clean up flags",
            "Add task",
        ],
    )
    .await;

    // A sub-task's ring sits at its parent's title edge (32 px per level).
    let parent = app::task_row(&window, "Plan the deck").await;
    let child = app::task_row(&window, "Collect metrics").await;
    let ring = Query::role(Role::CheckBox);
    let px = app::inside(&parent, ring.clone())
        .await
        .extents()
        .await
        .unwrap();
    let cx = app::inside(&child, ring).await.extents().await.unwrap();
    assert_eq!(cx.x - px.x, 32, "parent ring {px:?}, child ring {cx:?}");

    // Collapsing the parent hides its sub-tasks (and syncs `is_collapsed`).
    let chevron = app::inside(
        &parent,
        Query::role(Role::Button).name("Collapse sub-tasks"),
    )
    .await;
    s.click(&chevron).await.unwrap();
    app::wait_tasks(
        &window,
        &["Book the venue", "Plan the deck", "Clean up flags"],
    )
    .await;
    tick_e2e::poll(WAIT, "is_collapsed on the server", || async {
        let st = server.state();
        (st.get("items", "w1")?["is_collapsed"] == true).then_some(())
    })
    .await
    .unwrap();

    // Collapsing a section keeps its header and removes its tasks.
    let list = app::task_list(&window).await;
    let collapse = list
        .wait_for(&Query::role(Role::Button).name("Collapse This week"), WAIT)
        .await
        .unwrap();
    s.click(&collapse).await.unwrap();
    app::wait_rows(
        &window,
        &[
            "Work",
            "Book the venue",
            "Add task",
            "This week, 2 tasks, collapsed",
            "Later, 1 task, expanded",
            "Clean up flags",
            "Add task",
        ],
    )
    .await;
}

/// A query the local evaluator does not support comes from `GET /api/v1/tasks/filter`
/// and shows where it came from (4.22).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn server_filter_falls_back_to_todoist() {
    let (server, s, window) = start().await;
    app::wait_tasks(
        &window,
        &["Renew passport", "Plan the deck", "Call the dentist"],
    )
    .await;
    app::open_view(&s, &window, "Before Monday").await;
    let list = app::task_list(&window).await;
    list.wait_for(
        &Query::role(Role::Label).name("Results from Todoist, updated 09:00"),
        WAIT,
    )
    .await
    .unwrap();
    let mut got = app::task_names(&window).await;
    got.sort();
    assert_eq!(got, ["Call the dentist", "Plan the deck", "Renew passport"]);
    let asked = app::requests(&server, "/api/v1/tasks/filter");
    assert!(
        asked
            .iter()
            .any(|r| r.query.contains("due+before") || r.query.contains("due%20before")),
        "{asked:?}"
    );
}

/// A change made elsewhere (on the server) shows up in the open view after a sync.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_change_appears() {
    let (server, s, window) = start().await;
    app::open_view(&s, &window, "Inbox").await;
    app::wait_tasks(
        &window,
        &["Call the dentist", "Renew passport", "Read the article"],
    )
    .await;
    server.state().update("items", "i3", |v| {
        v["content"] = serde_json::json!("Read the article twice");
    });
    app::wait_tasks(
        &window,
        &[
            "Call the dentist",
            "Renew passport",
            "Read the article twice",
        ],
    )
    .await;
}

/// An account without tasks shows the empty states (4.13), which pass the audit too.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn empty_states() {
    let server = app::server("empty").await;
    let s = app::session(&server).start().await.unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    app::wait_rows(&window, &["Today", "All done for today"]).await;
    crate::a11y::check(&window, "empty Today").await;
    app::open_view(&s, &window, "Inbox").await;
    app::wait_rows(&window, &["Inbox", "Inbox is empty"]).await;
    let empty = app::task_list(&window)
        .await
        .wait_for(&Query::role(Role::ListItem).name("Inbox is empty"), WAIT)
        .await
        .unwrap();
    assert_eq!(
        empty.description().await.unwrap(),
        "New tasks land here. Press Q to add one."
    );
    crate::a11y::check(&window, "empty Inbox").await;
    app::open_view(&s, &window, "Upcoming").await;
    app::wait_rows(&window, &["Upcoming", "Nothing scheduled"]).await;
}
