//! Drag and drop with real pointer drags (`DESIGN.md` 4.1, 4.4) and the keyboard
//! equivalents, asserted on the fake server.

use std::time::Duration;

use serde_json::Value;
use tick_e2e::{Node, Query, Role, Session, keysym};
use tick_fake_server::FakeServer;

use crate::detail::wait_item;
use crate::{WAIT, app};

async fn start(view: &str, tasks: &[&str]) -> (FakeServer, Session, Node) {
    let server = app::server_with(app::fixture()).await;
    let s = app::session(&server).start().await.unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    app::open_view(&s, &window, view).await;
    app::wait_tasks(&window, tasks).await;
    (server, s, window)
}

/// The row's title point (left part of the content, away from the ring and the right half).
async fn title_point(window: &Node, title: &str) -> (i32, i32, tick_e2e::Rect) {
    let r = app::task_row(window, title).await.extents().await.unwrap();
    (r.x + 90, r.y + r.height / 2, r)
}

async fn drag(s: &Session, from: (i32, i32), to: (i32, i32)) {
    s.input().await.unwrap().drag(from, to).await.unwrap();
}

fn key(st: &tick_fake_server::State, id: &str) -> String {
    st.get("items", id)
        .and_then(|v| v["order_key"].as_str().map(str::to_owned))
        .unwrap_or_default()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reorder_and_nest_by_drag() {
    let (server, s, window) = start(
        "Inbox",
        &["Call the dentist", "Renew passport", "Read the article"],
    )
    .await;
    // Onto the upper half of the first row: before it.
    let (fx, fy, _) = title_point(&window, "Read the article").await;
    let (_, _, top) = title_point(&window, "Call the dentist").await;
    drag(&s, (fx, fy), (fx, top.y + 6)).await;
    app::wait_tasks(
        &window,
        &["Read the article", "Call the dentist", "Renew passport"],
    )
    .await;
    tick_e2e::poll(Duration::from_secs(15), "order on the server", || async {
        let st = server.state();
        (key(&st, "i3") < key(&st, "i1")).then_some(())
    })
    .await
    .unwrap();
    let updates = app::commands_sent(&server, "item_update");
    assert!(
        updates
            .iter()
            .any(|c| c["args"]["id"] == "i3" && c["args"]["order_key"].is_string()),
        "{updates:?}"
    );

    // Onto the right half of a row: nested under it.
    let (fx, fy, _) = title_point(&window, "Read the article").await;
    let (_, _, target) = title_point(&window, "Renew passport").await;
    drag(
        &s,
        (fx, fy),
        (
            target.x + target.width * 3 / 4,
            target.y + target.height * 3 / 4,
        ),
    )
    .await;
    wait_item(&server, "i3", "nested", |v| v["parent_id"] == "i2").await;
    let moves = app::commands_sent(&server, "item_move");
    assert_eq!(moves[0]["args"]["parent_id"], "i2", "{moves:?}");
    app::wait_tasks(
        &window,
        &["Call the dentist", "Renew passport", "Read the article"],
    )
    .await;
    // Keyboard: Alt+Shift+Left un-nests the selected task.
    let (x, y, _) = title_point(&window, "Read the article").await;
    s.input().await.unwrap().click(x, y).await.unwrap();
    s.chord(&[keysym::ALT_L, keysym::SHIFT_L, keysym::LEFT])
        .await
        .unwrap();
    wait_item(&server, "i3", "un-nested", |v| v["parent_id"].is_null()).await;
    // Alt+Shift+Up: before "Renew passport"; Alt+Shift+Right: under "Call the dentist".
    s.chord(&[keysym::ALT_L, keysym::SHIFT_L, keysym::UP])
        .await
        .unwrap();
    tick_e2e::poll(Duration::from_secs(15), "moved up", || async {
        let st = server.state();
        (key(&st, "i3") < key(&st, "i2") && key(&st, "i3") > key(&st, "i1")).then_some(())
    })
    .await
    .unwrap();
    s.chord(&[keysym::ALT_L, keysym::SHIFT_L, keysym::RIGHT])
        .await
        .unwrap();
    wait_item(&server, "i3", "nested by key", |v| v["parent_id"] == "i1").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn move_between_sections_by_drag() {
    let (server, s, window) = start(
        "Work",
        &[
            "Book the venue",
            "Plan the deck",
            "Collect metrics",
            "Clean up flags",
        ],
    )
    .await;
    let (fx, fy, _) = title_point(&window, "Book the venue").await;
    let header = app::task_list(&window)
        .await
        .wait_for(&Query::role(Role::ListItem).name_contains("Later, "), WAIT)
        .await
        .unwrap();
    let h = header.extents().await.unwrap();
    drag(&s, (fx, fy), (h.x + 120, h.y + h.height / 2)).await;
    wait_item(&server, "w4", "in Later", |v| v["section_id"] == "s2").await;
    let moves = app::commands_sent(&server, "item_move");
    assert_eq!(moves[0]["args"]["section_id"], "s2", "{moves:?}");
    // First in the section: before "Clean up flags".
    let st = server.state();
    assert!(
        key(&st, "w4") < key(&st, "w3"),
        "{} {}",
        key(&st, "w4"),
        key(&st, "w3")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn drop_onto_a_sidebar_project() {
    let (server, s, window) = start(
        "Inbox",
        &["Call the dentist", "Renew passport", "Read the article"],
    )
    .await;
    let (fx, fy, _) = title_point(&window, "Read the article").await;
    let home = app::sidebar(&window)
        .await
        .wait_for(
            &Query::role(Role::ListItem).name("Home, 2 tasks").showing(),
            WAIT,
        )
        .await
        .unwrap();
    let h = home.extents().await.unwrap();
    drag(&s, (fx, fy), (h.x + 80, h.y + h.height / 2)).await;
    wait_item(&server, "i3", "moved to Home", |v| v["project_id"] == "ph").await;
    window
        .wait_for(
            &Query::role(Role::Label).name("Moved to Home").showing(),
            WAIT,
        )
        .await
        .unwrap();
    app::wait_tasks(&window, &["Call the dentist", "Renew passport"]).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reorder_and_nest_projects_by_drag() {
    let server = app::server_with(app::fixture()).await;
    let s = app::session(&server).start().await.unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    let list = app::sidebar(&window).await;
    let row = |name: &'static str| {
        let list = list.clone();
        async move {
            list.wait_for(&Query::role(Role::ListItem).name(name).showing(), WAIT)
                .await
                .unwrap()
                .extents()
                .await
                .unwrap()
        }
    };
    let home = row("Home, 2 tasks").await;
    let work = row("Work, 4 tasks").await;
    // Top third of Work: Home goes before it.
    drag(
        &s,
        (home.x + 80, home.y + home.height / 2),
        // Drops land a few pixels off the AT-SPI extents (docs/testing.md): stay well inside
        // the top third (14 px of 44).
        (work.x + 80, work.y + 10),
    )
    .await;
    let pkey = |st: &tick_fake_server::State, id: &str| -> String {
        st.get("projects", id)
            .and_then(|v: &Value| v["order_key"].as_str().map(str::to_owned))
            .unwrap_or_default()
    };
    tick_e2e::poll(Duration::from_secs(15), "project order", || async {
        let st = server.state();
        (pkey(&st, "ph") < pkey(&st, "pw")).then_some(())
    })
    .await
    .unwrap();
    s.input().await.unwrap().move_to(640, 400).await.unwrap();
    // Middle third of Work: Home becomes its sub-project.
    tokio::time::sleep(Duration::from_millis(500)).await;
    let home = row("Home, 2 tasks").await;
    let work = row("Work, 4 tasks").await;
    drag(
        &s,
        (home.x + 80, home.y + home.height / 2),
        (work.x + 80, work.y + work.height / 2),
    )
    .await;
    tick_e2e::poll(Duration::from_secs(15), "nested project", || async {
        let st = server.state();
        (st.get("projects", "ph")?["parent_id"] == "pw").then_some(())
    })
    .await
    .unwrap();
    // The sidebar shows it under Work (indented by the tree).
    let w = row("Work, 4 tasks").await;
    let h = row("Home, 2 tasks").await;
    assert!(h.y > w.y, "{h:?} {w:?}");
}

/// 4.6: moving the pointer sideways while dragging changes the depth in 32 px steps.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sideways_drag_changes_depth() {
    let (server, s, window) = start(
        "Inbox",
        &["Call the dentist", "Renew passport", "Read the article"],
    )
    .await;
    // On its own row, a step and a half right (drops land a few pixels off, docs/testing.md):
    // one level in, under the task above.
    let (fx, fy, _) = title_point(&window, "Read the article").await;
    let input = s.input().await.unwrap();
    input.press_at(fx, fy).await.unwrap();
    input.drag_by(0, 8, 4).await.unwrap();
    input.drag_by(48, 0, 8).await.unwrap();
    input.release().await.unwrap();
    wait_item(&server, "i3", "one level in", |v| v["parent_id"] == "i2").await;
    let moves = app::commands_sent(&server, "item_move");
    assert_eq!(moves[0]["args"]["parent_id"], "i2", "{moves:?}");

    // A step and a half left: back out to the top level, after "Renew passport".
    tokio::time::sleep(Duration::from_millis(500)).await;
    let (fx, fy, _) = title_point(&window, "Read the article").await;
    input.press_at(fx, fy).await.unwrap();
    input.drag_by(0, 8, 4).await.unwrap();
    input.drag_by(-48, 0, 8).await.unwrap();
    input.release().await.unwrap();
    wait_item(&server, "i3", "back out", |v| v["parent_id"].is_null()).await;
    tick_e2e::poll(Duration::from_secs(15), "after Renew passport", || async {
        let st = server.state();
        (key(&st, "i3") > key(&st, "i2")).then_some(())
    })
    .await
    .unwrap();
}

/// 4.7: dropping a task into another day's group in Upcoming reschedules it, keeping its
/// time.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn drop_on_another_upcoming_day_reschedules() {
    let server = app::server_with(app::fixture()).await;
    let s = app::session(&server)
        .monitor(1280, 1200)
        .start()
        .await
        .unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    app::open_view(&s, &window, "Upcoming").await;
    app::wait_tasks(
        &window,
        &[
            "Renew passport",
            "Plan the deck",
            "Call the dentist",
            "Water the plants",
            "Book the venue",
        ],
    )
    .await;
    // 14:00 today onto Wednesday 25's group: the 25th at 14:00.
    let (fx, fy, _) = title_point(&window, "Plan the deck").await;
    let (tx, ty, _) = title_point(&window, "Book the venue").await;
    drag(&s, (fx, fy), (tx, ty)).await;
    wait_item(&server, "w1", "moved to the 25th", |v| {
        v["due"]["date"] == "2026-11-25T14:00:00"
    })
    .await;
    let updates = app::commands_sent(&server, "item_update");
    assert_eq!(
        updates.last().unwrap()["args"]["due"],
        serde_json::json!({"date": "2026-11-25T14:00:00"}),
        "{updates:?}"
    );
    // A full-day overdue task onto tomorrow's group: tomorrow, still full-day.
    tokio::time::sleep(Duration::from_millis(500)).await;
    let (fx, fy, _) = title_point(&window, "Renew passport").await;
    let (tx, ty, _) = title_point(&window, "Water the plants").await;
    drag(&s, (fx, fy), (tx, ty)).await;
    wait_item(&server, "i2", "moved to tomorrow", |v| {
        v["due"]["date"] == "2026-11-23"
    })
    .await;
}

/// 4.1: hovering a collapsed sidebar project for 600 ms while dragging expands it, so a
/// sub-project can take the drop.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hovering_a_collapsed_project_expands_it() {
    let mut f = app::fixture();
    f["projects"].as_array_mut().unwrap().push(
        serde_json::json!({"id": "pk", "name": "Kitchen", "color": "green",
                                 "parent_id": "ph", "child_order": 1}),
    );
    f["projects"][1]["is_collapsed"] = serde_json::json!(true);
    let server = app::server_with(f).await;
    let s = app::session(&server).start().await.unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    app::open_view(&s, &window, "Inbox").await;
    app::wait_tasks(
        &window,
        &["Call the dentist", "Renew passport", "Read the article"],
    )
    .await;
    let sidebar = app::sidebar(&window).await;
    let home = sidebar
        .wait_for(
            &Query::role(Role::ListItem).name("Home, 2 tasks").showing(),
            WAIT,
        )
        .await
        .unwrap();
    assert!(
        sidebar
            .find(&Query::role(Role::ListItem).name("Kitchen").showing())
            .await
            .is_none(),
        "Home starts collapsed"
    );
    let h = home.extents().await.unwrap();
    let (fx, fy, _) = title_point(&window, "Read the article").await;
    let input = s.input().await.unwrap();
    input.press_at(fx, fy).await.unwrap();
    input
        .drag_by(h.x + 80 - fx, h.y + h.height / 2 - fy, 20)
        .await
        .unwrap();
    // Rest on it: it opens after 600 ms.
    let kitchen = sidebar
        .wait_for(&Query::role(Role::ListItem).name("Kitchen").showing(), WAIT)
        .await
        .unwrap();
    let k = kitchen.extents().await.unwrap();
    input
        .drag_by(0, k.y + k.height / 2 - (h.y + h.height / 2), 6)
        .await
        .unwrap();
    input.release().await.unwrap();
    wait_item(&server, "i3", "moved to Kitchen", |v| {
        v["project_id"] == "pk"
    })
    .await;
}

/// B1 (3.3): a click on the list, empty space included, closes the overlaid detail pane
/// (libadwaita's overlay split view does it; the click does not reach a row under it).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn b1_click_on_the_list_closes_the_detail() {
    let server = app::server_with(app::fixture()).await;
    let s = app::session(&server)
        .monitor(900, 800)
        .start()
        .await
        .unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    app::open_view(&s, &window, "Inbox").await;
    crate::detail::open_task(&s, &window, "Read the article").await;
    // Below the last row, left of the overlaid pane.
    let list = app::task_list(&window).await.extents().await.unwrap();
    s.input()
        .await
        .unwrap()
        .click(list.x + 60, list.y + list.height - 40)
        .await
        .unwrap();
    let title = Query::role(Role::Text).name("Task title").showing();
    window.wait_gone(&title, WAIT).await.unwrap();
    // On a row: closes too, and does not open that row.
    tokio::time::sleep(Duration::from_millis(1000)).await;
    crate::detail::open_task(&s, &window, "Read the article").await;
    tokio::time::sleep(Duration::from_millis(1000)).await;
    let r = app::task_row(&window, "Call the dentist")
        .await
        .extents()
        .await
        .unwrap();
    s.input()
        .await
        .unwrap()
        .click(r.x + 120, r.y + 22)
        .await
        .unwrap();
    window.wait_gone(&title, WAIT).await.unwrap();
    tokio::time::sleep(Duration::from_millis(1000)).await;
    assert!(window.find(&title).await.is_none());
    // The next click opens it.
    crate::detail::open_task(&s, &window, "Call the dentist").await;
}
