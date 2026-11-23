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
