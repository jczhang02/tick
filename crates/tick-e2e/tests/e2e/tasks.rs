//! Completing with Undo, quick add (online and offline), the sync indicator and the
//! keyboard shortcuts, asserted on the fake server's state.

use std::time::Duration;

use tick_e2e::{Node, Query, Role, Session, State, keysym};
use tick_fake_server::FakeServer;

use crate::{WAIT, app};

async fn start_in_inbox() -> (FakeServer, Session, Node) {
    let server = app::server_with(app::fixture()).await;
    let s = app::session(&server).start().await.unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    app::open_view(&s, &window, "Inbox").await;
    app::wait_tasks(
        &window,
        &["Call the dentist", "Renew passport", "Read the article"],
    )
    .await;
    (server, s, window)
}

async fn complete(s: &Session, window: &Node, title: &str) {
    let row = app::task_row(window, title).await;
    let ring = app::inside(&row, Query::role(Role::CheckBox).name("Complete task")).await;
    s.click(&ring).await.unwrap();
}

async fn toast_button(window: &Node, name: &str) -> Node {
    window
        .wait_for(&Query::role(Role::Button).name(name).showing(), WAIT)
        .await
        .unwrap()
}

/// "Undo" inside the 5 s window: the close is never sent.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn undo_keeps_the_close_off_the_server() {
    let (server, s, window) = start_in_inbox().await;
    complete(&s, &window, "Read the article").await;
    app::wait_tasks(&window, &["Call the dentist", "Renew passport"]).await;
    let undo = toast_button(&window, "Undo").await;
    s.click(&undo).await.unwrap();
    app::wait_tasks(
        &window,
        &["Call the dentist", "Renew passport", "Read the article"],
    )
    .await;
    // Well past the undo window and several sync intervals.
    tokio::time::sleep(Duration::from_secs(8)).await;
    assert!(app::commands_sent(&server, "item_close").is_empty());
    assert_eq!(server.state().get("items", "i3").unwrap()["checked"], false);
}

/// Without Undo the close goes out after the window.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn complete_reaches_the_server() {
    let (server, s, window) = start_in_inbox().await;
    complete(&s, &window, "Read the article").await;
    app::wait_tasks(&window, &["Call the dentist", "Renew passport"]).await;
    toast_button(&window, "Undo").await;
    let sent = tick_e2e::poll(Duration::from_secs(15), "item_close", || async {
        let c = app::commands_sent(&server, "item_close");
        (!c.is_empty()).then_some(c)
    })
    .await
    .unwrap();
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert_eq!(sent[0]["args"]["id"], "i3");
    assert_eq!(server.state().get("items", "i3").unwrap()["checked"], true);
    // Still gone after the sync.
    tokio::time::sleep(Duration::from_secs(2)).await;
    app::wait_tasks(&window, &["Call the dentist", "Renew passport"]).await;
}

/// Completing a recurring task keeps its row, checked, until the server moves its date.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn recurring_task_shows_completed_pending() {
    let server = app::server_with(app::fixture()).await;
    let s = app::session(&server).start().await.unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    app::open_view(&s, &window, "Home").await;
    app::wait_tasks(&window, &["Water the plants", "Plant bulbs"]).await;
    complete(&s, &window, "Water the plants").await;
    let row = app::task_row(&window, "Water the plants").await;
    app::inside(&row, Query::role(Role::CheckBox).name("Mark as not done")).await;
    assert!(
        row.description().await.unwrap().starts_with("Completing"),
        "{:?}",
        row.description().await
    );
    // After the undo window the server advances the date and the ring empties again.
    tick_e2e::poll(Duration::from_secs(15), "the next date", || async {
        let st = server.state();
        let due = st.get("items", "h1")?["due"]["date"].as_str()?.to_owned();
        (due != "2026-11-23").then_some(())
    })
    .await
    .unwrap();
    let row = app::task_row(&window, "Water the plants").await;
    row.wait_for(
        &Query::role(Role::CheckBox).name("Complete task"),
        Duration::from_secs(10),
    )
    .await
    .unwrap();
}

async fn quick_add(s: &Session, window: &Node, text: &str) {
    s.key(u32::from(b'q')).await.unwrap();
    let entry = window
        .wait_for(&Query::role(Role::Text).name("Task").showing(), WAIT)
        .await
        .unwrap();
    tick_e2e::poll(WAIT, "quick-add entry focused", || async {
        entry.has_state(State::Focused).await.ok().filter(|f| *f)
    })
    .await
    .unwrap();
    s.type_text(text).await.unwrap();
    s.key(keysym::RETURN).await.unwrap();
    window
        .wait_gone(&Query::role(Role::Text).name("Task").showing(), WAIT)
        .await
        .unwrap();
}

fn quick_adds(server: &FakeServer) -> Vec<String> {
    app::requests(server, "/api/v1/tasks/quick")
        .into_iter()
        .filter(|r| r.status == 200)
        .map(|r| r.body["text"].as_str().unwrap_or_default().to_owned())
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn quick_add_online() {
    let (server, s, window) = start_in_inbox().await;
    quick_add(&s, &window, "Buy milk p1").await;
    tick_e2e::poll(WAIT, "/tasks/quick", || async {
        (quick_adds(&server) == ["Buy milk p1"]).then_some(())
    })
    .await
    .unwrap();
    // The server parsed "p1": the row now shows the server's title.
    app::task_row(&window, "Buy milk").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn quick_add_offline_is_sent_once_back_online() {
    let (server, s, window) = start_in_inbox().await;
    server.set_offline(true);
    let sync = window
        .wait_for(
            &Query::role(Role::Button).name("Sync Status").showing(),
            Duration::from_secs(15),
        )
        .await
        .unwrap();
    tick_e2e::poll(WAIT, "offline description", || async {
        let d = sync.description().await.ok()?;
        (d == "Offline. Changes are saved on this computer.").then_some(())
    })
    .await
    .unwrap();

    quick_add(&s, &window, "Offline task").await;
    // The row is there at once, and the indicator counts the queued change.
    app::task_row(&window, "Offline task").await;
    tick_e2e::poll(WAIT, "pending count", || async {
        let d = sync.description().await.ok()?;
        (d == "1 change waiting to sync").then_some(())
    })
    .await
    .unwrap();
    sync.find(&Query::role(Role::Label).name("1"))
        .await
        .expect("the count is visible in the button");
    assert!(quick_adds(&server).is_empty());

    server.set_offline(false);
    tick_e2e::poll(
        Duration::from_secs(20),
        "/tasks/quick after reconnect",
        || async { (!quick_adds(&server).is_empty()).then_some(()) },
    )
    .await
    .unwrap();
    window
        .wait_gone(
            &Query::role(Role::Button).name("Sync Status").showing(),
            Duration::from_secs(15),
        )
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert_eq!(quick_adds(&server), ["Offline task"]);
    app::task_row(&window, "Offline task").await;
}

/// `g i` / `g u` / `g t`, arrow keys, `e` and `q`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn keyboard_shortcuts() {
    let server = app::server_with(app::fixture()).await;
    let s = app::session(&server).start().await.unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    app::wait_tasks(
        &window,
        &["Renew passport", "Plan the deck", "Call the dentist"],
    )
    .await;
    let header = |name: &'static str| {
        let window = window.clone();
        async move {
            // The content page is named after the open view.
            window
                .wait_for(&Query::role(Role::Grouping).name(name).showing(), WAIT)
                .await
                .unwrap();
        }
    };

    s.key(u32::from(b'g')).await.unwrap();
    s.key(u32::from(b'u')).await.unwrap();
    header("Upcoming").await;
    s.key(u32::from(b'g')).await.unwrap();
    s.key(u32::from(b'i')).await.unwrap();
    header("Inbox").await;
    app::wait_tasks(
        &window,
        &["Call the dentist", "Renew passport", "Read the article"],
    )
    .await;

    // "go to" focuses and selects the first task; Down moves to the next one.
    let first = app::task_row(&window, "Call the dentist").await;
    tick_e2e::poll(WAIT, "first task focused", || async {
        first.has_state(State::Focused).await.ok().filter(|f| *f)
    })
    .await
    .unwrap();
    s.key(keysym::DOWN).await.unwrap();
    let second = app::task_row(&window, "Renew passport").await;
    tick_e2e::poll(WAIT, "second task selected", || async {
        second.has_state(State::Selected).await.ok().filter(|f| *f)
    })
    .await
    .unwrap();

    // "e" completes the selected task.
    s.key(u32::from(b'e')).await.unwrap();
    app::wait_tasks(&window, &["Call the dentist", "Read the article"]).await;
    tick_e2e::poll(Duration::from_secs(15), "item_close i2", || async {
        let c = app::commands_sent(&server, "item_close");
        c.iter().any(|c| c["args"]["id"] == "i2").then_some(())
    })
    .await
    .unwrap();

    s.key(u32::from(b'g')).await.unwrap();
    s.key(u32::from(b't')).await.unwrap();
    header("Today").await;

    // "q" opens quick add; Escape closes it.
    s.key(u32::from(b'q')).await.unwrap();
    window
        .wait_for(&Query::role(Role::Text).name("Task").showing(), WAIT)
        .await
        .unwrap();
    s.key(keysym::ESCAPE).await.unwrap();
    window
        .wait_gone(&Query::role(Role::Text).name("Task").showing(), WAIT)
        .await
        .unwrap();
}
