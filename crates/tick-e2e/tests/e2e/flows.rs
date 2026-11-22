//! Flows around the main views: "Add task" prefills, quick add details, the sync popover,
//! the restored last view, falling back when the open view is deleted, keyboard navigation
//! and a revoked token.

use std::time::Duration;

use tick_e2e::{Node, Query, Role, Session, State, keysym};
use tick_fake_server::FakeServer;

use crate::{WAIT, app};

/// A session on a tall monitor, so `GtkListView` builds every row of a view.
async fn start() -> (FakeServer, Session, Node) {
    let server = app::server_with(app::fixture()).await;
    let s = app::session(&server)
        .monitor(1280, 2000)
        .start()
        .await
        .unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    app::wait_tasks(
        &window,
        &["Renew passport", "Plan the deck", "Call the dentist"],
    )
    .await;
    (server, s, window)
}

/// The content page is named after the open view.
async fn wait_view(window: &Node, name: &str) {
    window
        .wait_for(&Query::role(Role::Grouping).name(name).showing(), WAIT)
        .await
        .unwrap();
}

/// The quick-add entry, once it has keyboard focus.
async fn quick_add_entry(window: &Node) -> Node {
    let entry = window
        .wait_for(&Query::role(Role::Text).name("Task").showing(), WAIT)
        .await
        .unwrap();
    tick_e2e::poll(WAIT, "quick-add entry focused", || async {
        entry.has_state(State::Focused).await.ok().filter(|f| *f)
    })
    .await
    .unwrap();
    entry
}

async fn wait_text(entry: &Node, want: &str) {
    let got = tick_e2e::poll(WAIT, &format!("entry text {want:?}"), || async {
        let t = entry.text().await.ok()?;
        (t == want).then_some(t)
    })
    .await;
    if got.is_err() {
        panic!("entry text {:?}, want {want:?}", entry.text().await);
    }
}

async fn close_quick_add(s: &Session, window: &Node) {
    s.key(keysym::ESCAPE).await.unwrap();
    window
        .wait_gone(&Query::role(Role::Text).name("Task").showing(), WAIT)
        .await
        .unwrap();
}

/// Clicks the task-list row at `index` (the order of [`app::row_names`]).
async fn click_row(s: &Session, window: &Node, index: usize) {
    let list = app::task_list(window).await;
    let rows = list.find_all(&Query::role(Role::ListItem)).await;
    s.click(&rows[index]).await.unwrap();
}

fn position(names: &[String], name: &str) -> usize {
    names
        .iter()
        .position(|n| n == name)
        .unwrap_or_else(|| panic!("no row {name:?} in {names:?}"))
}

/// "Add task" rows prefill quick add: `#Project /Section` in a project (4.5), the ISO date
/// of the day in Upcoming (4.7).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_task_rows_prefill_quick_add() {
    let (_server, s, window) = start().await;
    app::open_view(&s, &window, "Work").await;
    app::wait_tasks(
        &window,
        &[
            "Book the venue",
            "Plan the deck",
            "Collect metrics",
            "Clean up flags",
        ],
    )
    .await;
    let names = app::row_names(&window).await;
    let section = position(&names, "This week, 2 tasks, expanded");
    let ghost = section
        + names[section..]
            .iter()
            .position(|n| n == "Add task")
            .unwrap();
    click_row(&s, &window, ghost).await;
    let entry = quick_add_entry(&window).await;
    wait_text(&entry, "#Work /This\\ week ").await;
    close_quick_add(&s, &window).await;

    app::open_view(&s, &window, "Upcoming").await;
    wait_view(&window, "Upcoming").await;
    app::task_list(&window)
        .await
        .wait_for(&Query::role(Role::ListItem).name("Tuesday"), WAIT)
        .await
        .unwrap();
    let names = app::row_names(&window).await;
    let tuesday = position(&names, "Tuesday");
    assert_eq!(names[tuesday + 1], "Add task", "{names:?}");
    click_row(&s, &window, tuesday + 1).await;
    let entry = quick_add_entry(&window).await;
    // An empty day: the date as ISO text, the same in every language.
    wait_text(&entry, "2026-11-24 ").await;
}

/// Quick add: Ctrl+Enter keeps the dialog open for another task; Escape keeps the draft
/// (with the cursor at the end, so typing appends); the unknown-project hint.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn quick_add_another_draft_and_hints() {
    let (server, s, window) = start().await;
    app::open_view(&s, &window, "Inbox").await;
    wait_view(&window, "Inbox").await;

    s.key(u32::from(b'q')).await.unwrap();
    let entry = quick_add_entry(&window).await;
    s.type_text("First one").await.unwrap();
    s.chord(&[keysym::CONTROL_L, keysym::RETURN]).await.unwrap();
    wait_text(&entry, "").await;
    assert!(entry.has_state(State::Showing).await.unwrap());
    s.type_text("Second one").await.unwrap();
    s.key(keysym::RETURN).await.unwrap();
    window
        .wait_gone(&Query::role(Role::Text).name("Task").showing(), WAIT)
        .await
        .unwrap();
    tick_e2e::poll(WAIT, "two quick adds", || async {
        let mut texts: Vec<String> = app::requests(&server, "/api/v1/tasks/quick")
            .into_iter()
            .filter(|r| r.status == 200)
            .map(|r| r.body["text"].as_str().unwrap_or_default().to_owned())
            .collect();
        texts.sort();
        (texts == ["First one", "Second one"]).then_some(())
    })
    .await
    .unwrap();

    // A hint when the project does not exist (the chips are display only, hidden from AT;
    // the quick-add screenshots show them).
    s.key(u32::from(b'q')).await.unwrap();
    let entry = quick_add_entry(&window).await;
    s.type_text("Buy paint p1 #Garage").await.unwrap();
    window
        .wait_for(
            &Query::role(Role::Label)
                .name("No project named \"Garage\". The task goes to the Inbox.")
                .showing(),
            WAIT,
        )
        .await
        .unwrap();

    // Escape keeps the draft; reopening puts the cursor after it, so typing appends.
    close_quick_add(&s, &window).await;
    s.key(u32::from(b'q')).await.unwrap();
    let entry2 = quick_add_entry(&window).await;
    wait_text(&entry2, "Buy paint p1 #Garage").await;
    s.type_text(" now").await.unwrap();
    wait_text(&entry2, "Buy paint p1 #Garage now").await;
    drop(entry);
}

/// "Task added" offers Open, which shows the project the task landed in.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn task_added_toast_opens_the_project() {
    let (server, s, window) = start().await;
    s.key(u32::from(b'q')).await.unwrap();
    quick_add_entry(&window).await;
    s.type_text("Paint the fence #Home").await.unwrap();
    s.key(keysym::RETURN).await.unwrap();
    tick_e2e::poll(Duration::from_secs(15), "/tasks/quick", || async {
        app::requests(&server, "/api/v1/tasks/quick")
            .iter()
            .any(|r| r.status == 200)
            .then_some(())
    })
    .await
    .unwrap();
    let open = window
        .wait_for(&Query::role(Role::Button).name("Open").showing(), WAIT)
        .await
        .unwrap();
    s.click(&open).await.unwrap();
    wait_view(&window, "Home").await;
    app::wait_tasks(
        &window,
        &["Water the plants", "Plant bulbs", "Paint the fence"],
    )
    .await;
}

/// The sync popover lists a rejected change; Sync Now syncs at once, Dismiss Errors clears
/// the failed state and the indicator.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_popover_sync_now_and_dismiss() {
    let server = app::server_with(app::fixture()).await;
    // A long interval, so only Sync Now explains a request.
    let s = app::session(&server)
        .env("TICK_SYNC_INTERVAL_MS", "600000")
        .start()
        .await
        .unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    app::open_view(&s, &window, "Inbox").await;
    app::wait_tasks(
        &window,
        &["Call the dentist", "Renew passport", "Read the article"],
    )
    .await;
    server.inject(tick_fake_server::Failure::CommandError {
        command_type: "item_close".into(),
        args_contain: None,
        error: Some(serde_json::json!({
            "error_tag": "ITEM_NOT_FOUND", "error_code": 22, "http_code": 400,
            "error": "Item not found", "error_extra": {}
        })),
        count: 1,
    });
    let row = app::task_row(&window, "Read the article").await;
    let ring = app::inside(&row, Query::role(Role::CheckBox).name("Complete task")).await;
    s.click(&ring).await.unwrap();
    let sync = window
        .wait_for(
            &Query::role(Role::Button).name("Sync Status").showing(),
            Duration::from_secs(20),
        )
        .await
        .unwrap();
    tick_e2e::poll(WAIT, "failed state", || async {
        let d = sync.description().await.ok()?;
        (d == "1 change could not be saved").then_some(())
    })
    .await
    .unwrap();

    let before = app::requests(&server, "/api/v1/sync").len();
    s.click(&sync).await.unwrap();
    let sync_now = window
        .wait_for(&Query::role(Role::Button).name("Sync Now").showing(), WAIT)
        .await
        .unwrap();
    window
        .wait_for(
            &Query::role(Role::Label)
                .name("Complete \"Read the article\"")
                .showing(),
            WAIT,
        )
        .await
        .unwrap();
    // The rolled-back change is explained once (4.14, 6.4).
    window
        .wait_for(
            &Query::role(Role::Label)
                .name("Todoist rejected the change. Your task shows the version on the server.")
                .showing(),
            WAIT,
        )
        .await
        .unwrap();
    s.click(&sync_now).await.unwrap();
    tick_e2e::poll(Duration::from_secs(5), "a sync after Sync Now", || async {
        (app::requests(&server, "/api/v1/sync").len() > before).then_some(())
    })
    .await
    .unwrap();
    window
        .wait_gone(&Query::role(Role::Button).name("Sync Now").showing(), WAIT)
        .await
        .unwrap();

    s.click(&sync).await.unwrap();
    let dismiss = window
        .wait_for(
            &Query::role(Role::Button).name("Dismiss Errors").showing(),
            WAIT,
        )
        .await
        .unwrap();
    s.click(&dismiss).await.unwrap();
    window
        .wait_gone(
            &Query::role(Role::Button).name("Sync Status").showing(),
            WAIT,
        )
        .await
        .unwrap();
    let row = app::task_row(&window, "Read the article").await;
    tick_e2e::poll(WAIT, "no failed-change note", || async {
        let d = row.description().await.ok()?;
        (!d.contains("Not saved")).then_some(())
    })
    .await
    .unwrap();
}

/// The open view is saved (`last-view`) and comes back after a relaunch.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn last_view_is_restored() {
    let server = app::server_with(app::fixture()).await;
    let mut s = app::session(&server).start().await.unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    app::open_view(&s, &window, "Work").await;
    wait_view(&window, "Work").await;
    s.activate_action("quit", None).await.unwrap();
    s.wait_app_exit(WAIT).await.unwrap();
    s.launch_app().unwrap();
    let window = s.window().await.unwrap();
    wait_view(&window, "Work").await;
    app::task_row(&window, "Plan the deck").await;
}

/// A project, label or filter deleted elsewhere while it is open: the view falls back to
/// Today.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deleted_view_falls_back_to_today() {
    let (server, s, window) = start().await;
    app::open_view(&s, &window, "Home").await;
    wait_view(&window, "Home").await;
    server.state().update("projects", "ph", |v| {
        v["is_deleted"] = serde_json::json!(true);
    });
    wait_view(&window, "Today").await;
    app::wait_tasks(
        &window,
        &["Renew passport", "Plan the deck", "Call the dentist"],
    )
    .await;

    app::open_view(&s, &window, "phone").await;
    wait_view(&window, "phone").await;
    server.state().update("labels", "l1", |v| {
        v["is_deleted"] = serde_json::json!(true);
    });
    wait_view(&window, "Today").await;

    app::open_view(&s, &window, "Urgent").await;
    wait_view(&window, "Urgent").await;
    server.state().update("filters", "f1", |v| {
        v["is_deleted"] = serde_json::json!(true);
    });
    wait_view(&window, "Today").await;
}

/// J and K move between tasks like Down and Up.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn j_and_k_move_between_tasks() {
    let (_server, s, window) = start().await;
    s.key(u32::from(b'g')).await.unwrap();
    s.key(u32::from(b'i')).await.unwrap();
    wait_view(&window, "Inbox").await;
    let selected = |title: &'static str| {
        let window = window.clone();
        async move {
            let row = app::task_row(&window, title).await;
            tick_e2e::poll(WAIT, &format!("{title:?} selected"), || async {
                row.has_state(State::Selected).await.ok().filter(|f| *f)
            })
            .await
            .unwrap();
        }
    };
    selected("Call the dentist").await;
    s.key(u32::from(b'j')).await.unwrap();
    selected("Renew passport").await;
    s.key(u32::from(b'j')).await.unwrap();
    selected("Read the article").await;
    s.key(u32::from(b'k')).await.unwrap();
    selected("Renew passport").await;
}

/// A token revoked while the main view is showing leads to the login page.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn revoked_token_shows_the_login_page() {
    let (server, _s, window) = start().await;
    server.state().token = "a-new-token".into();
    window
        .wait_for(
            &Query::role(Role::PasswordText).showing(),
            Duration::from_secs(15),
        )
        .await
        .unwrap();
    window
        .wait_for(
            &Query::role(Role::Label)
                .name("That token was not accepted. Check it and try again.")
                .showing(),
            WAIT,
        )
        .await
        .unwrap();
}

/// A filter the server must evaluate, opened offline without a cached result, and one the
/// server refuses (4.22).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn server_filter_offline_and_rejected() {
    let (server, s, window) = start().await;
    server.set_offline(true);
    window
        .wait_for(
            &Query::role(Role::Button).name("Sync Status").showing(),
            Duration::from_secs(15),
        )
        .await
        .unwrap();
    app::open_view(&s, &window, "Before Monday").await;
    app::wait_rows(
        &window,
        &["Before Monday", "Can't show this filter offline"],
    )
    .await;
    assert!(app::requests(&server, "/api/v1/tasks/filter").is_empty());

    app::open_view(&s, &window, "Inbox").await;
    wait_view(&window, "Inbox").await;
    server.inject(tick_fake_server::Failure::Status {
        status: 400,
        retry_after: None,
        count: 1,
        path: Some("/api/v1/tasks/filter".into()),
        body_contains: None,
    });
    server.set_offline(false);
    window
        .wait_gone(
            &Query::role(Role::Button).name("Sync Status").showing(),
            Duration::from_secs(15),
        )
        .await
        .unwrap();
    app::open_view(&s, &window, "Before Monday").await;
    app::wait_rows(
        &window,
        &["Before Monday", "Todoist couldn't read this filter"],
    )
    .await;
}

/// Offline, the indicator and its popover stay up while the app keeps retrying (each retry
/// used to hide them for a moment, closing the popover); Sync Now waits for a connection.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn offline_popover_stays_open() {
    let (server, s, window) = start().await;
    server.set_offline(true);
    let sync = window
        .wait_for(
            &Query::role(Role::Button).name("Sync Status").showing(),
            Duration::from_secs(15),
        )
        .await
        .unwrap();
    s.click(&sync).await.unwrap();
    let sync_now = window
        .wait_for(&Query::role(Role::Button).name("Sync Now").showing(), WAIT)
        .await
        .unwrap();
    assert!(!sync_now.has_state(State::Sensitive).await.unwrap());
    // Several retries (the sync interval is 1 s).
    tokio::time::sleep(Duration::from_secs(4)).await;
    assert!(sync_now.has_state(State::Showing).await.unwrap());
}
