//! Task row shortcuts (`DESIGN.md` 4.4): Enter opens, Escape closes, 1 to 4, T, L, V,
//! Space, Shift+Delete with Undo, and the context menu.

use std::time::Duration;

use tick_e2e::{Node, Query, Role, Session, keysym};
use tick_fake_server::FakeServer;

use crate::detail::{field, menu_choose, pane_title, picker_row, wait_item};
use crate::{WAIT, app};

async fn start() -> (FakeServer, Session, Node) {
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

/// Keyboard focus on the first task row (J from the list).
async fn focus_first_row(s: &Session, window: &Node) -> Node {
    s.key(u32::from(b'g')).await.unwrap();
    s.key(u32::from(b'i')).await.unwrap();
    let row = app::task_row(window, "Call the dentist").await;
    tick_e2e::poll(WAIT, "first row focused", || async {
        row.has_state(tick_e2e::State::Focused)
            .await
            .ok()
            .filter(|f| *f)
    })
    .await
    .unwrap();
    row
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn enter_opens_escape_closes() {
    let (_server, s, window) = start().await;
    focus_first_row(&s, &window).await;
    s.key(u32::from(b'j')).await.unwrap();
    s.key(keysym::RETURN).await.unwrap();
    pane_title(&window, "Renew passport").await;
    s.key(keysym::ESCAPE).await.unwrap();
    window
        .wait_gone(&Query::role(Role::Text).name("Task title").showing(), WAIT)
        .await
        .unwrap();
    // Focus is back on the row.
    let row = app::task_row(&window, "Renew passport").await;
    tick_e2e::poll(WAIT, "row focused again", || async {
        row.has_state(tick_e2e::State::Focused)
            .await
            .ok()
            .filter(|f| *f)
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn priority_date_labels_and_move_keys() {
    let (server, s, window) = start().await;
    focus_first_row(&s, &window).await;
    // 1: priority 1 (API 4).
    s.key(u32::from(b'1')).await.unwrap();
    wait_item(&server, "i1", "p1", |v| v["priority"] == 4).await;
    // T: the date picker, anchored to the row.
    s.key(u32::from(b't')).await.unwrap();
    s.type_text("tomorrow").await.unwrap();
    s.key(keysym::RETURN).await.unwrap();
    wait_item(&server, "i1", "due tomorrow", |v| {
        v["due"]["date"] == "2026-11-23"
    })
    .await;
    // L: the label picker; typing filters, Enter toggles the first row (an exact match).
    s.key(u32::from(b'l')).await.unwrap();
    picker_row(&window, "errand").await;
    s.type_text("errand").await.unwrap();
    s.key(keysym::RETURN).await.unwrap();
    wait_item(&server, "i1", "labels", |v| {
        v["labels"] == serde_json::json!(["phone", "errand"])
    })
    .await;
    s.key(keysym::ESCAPE).await.unwrap();
    // V: move to a project.
    tokio::time::sleep(Duration::from_millis(300)).await;
    s.key(u32::from(b'v')).await.unwrap();
    s.type_text("Home").await.unwrap();
    s.key(keysym::RETURN).await.unwrap();
    wait_item(&server, "i1", "moved", |v| v["project_id"] == "ph").await;
    window
        .wait_for(
            &Query::role(Role::Label).name("Moved to Home").showing(),
            WAIT,
        )
        .await
        .unwrap();
    app::wait_tasks(&window, &["Renew passport", "Read the article"]).await;
    // Undo puts it back.
    s.chord(&[keysym::CONTROL_L, u32::from(b'z')])
        .await
        .unwrap();
    wait_item(&server, "i1", "moved back", |v| v["project_id"] != "ph").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn space_completes_and_shift_delete_undoes() {
    let (server, s, window) = start().await;
    let row = focus_first_row(&s, &window).await;
    let _ = row;
    // Shift+Delete deletes with an Undo toast; Undo keeps it off the server.
    s.chord(&[keysym::SHIFT_L, keysym::DELETE]).await.unwrap();
    app::wait_tasks(&window, &["Renew passport", "Read the article"]).await;
    let undo = window
        .wait_for(&Query::role(Role::Button).name("Undo").showing(), WAIT)
        .await
        .unwrap();
    window
        .wait_for(
            &Query::role(Role::Label).name("Task deleted").showing(),
            WAIT,
        )
        .await
        .unwrap();
    s.click(&undo).await.unwrap();
    app::wait_tasks(
        &window,
        &["Call the dentist", "Renew passport", "Read the article"],
    )
    .await;
    // A toast under the pointer never times out (libadwaita pauses it): move away.
    s.input().await.unwrap().move_to(640, 300).await.unwrap();
    // Deleting again, without Undo, reaches the server after the toast.
    focus_first_row(&s, &window).await;
    s.chord(&[keysym::SHIFT_L, keysym::DELETE]).await.unwrap();
    tick_e2e::poll(Duration::from_secs(20), "item_delete", || async {
        let c = app::commands_sent(&server, "item_delete");
        (!c.is_empty()).then_some(c)
    })
    .await
    .unwrap();
    assert!(server.state().get("items", "i1").is_none());
    // Space completes the focused row.
    focus_first_row_named(&s, &window, "Renew passport").await;
    s.key(keysym::SPACE).await.unwrap();
    app::wait_tasks(&window, &["Read the article"]).await;
    let closes = tick_e2e::poll(Duration::from_secs(20), "item_close", || async {
        let c = app::commands_sent(&server, "item_close");
        (!c.is_empty()).then_some(c)
    })
    .await
    .unwrap();
    assert_eq!(closes[0]["args"]["id"], "i2");
}

async fn focus_first_row_named(s: &Session, window: &Node, title: &str) {
    s.key(u32::from(b'g')).await.unwrap();
    s.key(u32::from(b'i')).await.unwrap();
    let row = app::task_row(window, title).await;
    tick_e2e::poll(WAIT, "row focused", || async {
        row.has_state(tick_e2e::State::Focused)
            .await
            .ok()
            .filter(|f| *f)
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn context_menu_by_keyboard_and_pointer() {
    let (server, s, window) = start().await;
    focus_first_row(&s, &window).await;
    // Shift+F10: Open is the first item.
    s.chord(&[keysym::SHIFT_L, keysym::F10]).await.unwrap();
    menu_choose(&s, &window, 0).await;
    pane_title(&window, "Call the dentist").await;
    // Right-click "Read the article": Priority... (the fourth item) then Priority 3.
    let row = app::task_row(&window, "Read the article").await;
    let r = row.extents().await.unwrap();
    s.input()
        .await
        .unwrap()
        .right_click(r.x + 150, r.y + 20)
        .await
        .unwrap();
    menu_choose(&s, &window, 3).await;
    s.click(&picker_row(&window, "Priority 3").await)
        .await
        .unwrap();
    wait_item(&server, "i3", "p3", |v| v["priority"] == 2).await;
    // The right-clicked row is now the selected one: the pane did not change.
    field(&window, "Priority").await;
}

/// A second Undo toast lets the first change go: two deletes in a row both reach the server
/// within the sync cadence, not after the 60 s Undo hold.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_deletes_in_a_row_both_leave() {
    let (server, s, window) = start().await;
    focus_first_row(&s, &window).await;
    s.chord(&[keysym::SHIFT_L, keysym::DELETE]).await.unwrap();
    app::wait_tasks(&window, &["Renew passport", "Read the article"]).await;
    focus_first_row_named(&s, &window, "Renew passport").await;
    s.chord(&[keysym::SHIFT_L, keysym::DELETE]).await.unwrap();
    app::wait_tasks(&window, &["Read the article"]).await;
    let sent = tick_e2e::poll(Duration::from_secs(20), "both item_delete", || async {
        let c = app::commands_sent(&server, "item_delete");
        (c.len() >= 2).then_some(c)
    })
    .await
    .unwrap();
    let ids: Vec<&str> = sent
        .iter()
        .filter_map(|c| c["args"]["id"].as_str())
        .collect();
    assert!(ids.contains(&"i1") && ids.contains(&"i2"), "{ids:?}");
}
