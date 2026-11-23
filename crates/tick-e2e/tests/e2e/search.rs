//! Search (`DESIGN.md` 4.15): `/` and the sidebar row open it with the entry focused,
//! results follow typing (titles and descriptions, Chinese included), the keyboard reaches
//! the results, and a result opens in the detail pane.

use serde_json::json;
use tick_e2e::{Node, Query, Role, Session, keysym};
use tick_fake_server::FakeServer;

use crate::detail::pane_title;
use crate::{WAIT, app};

/// [`app::fixture`] plus tasks with Chinese titles and descriptions.
pub fn fixture() -> serde_json::Value {
    let mut f = app::fixture();
    let items = f["items"].as_array_mut().unwrap();
    items.push(json!({"id": "c1", "content": "整理季度报告", "child_order": 4}));
    items.push(
        json!({"id": "c2", "content": "Draft the budget", "child_order": 5,
                      "description": "先看季度数字, then the deck"}),
    );
    items.push(
        json!({"id": "c3", "content": "Done already", "child_order": 6,
                      "checked": true}),
    );
    f
}

async fn start() -> (FakeServer, Session, Node) {
    let server = app::server_with(fixture()).await;
    let s = app::session(&server).start().await.unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    app::wait_tasks(
        &window,
        &["Renew passport", "Plan the deck", "Call the dentist"],
    )
    .await;
    (server, s, window)
}

/// The search entry, focused.
pub async fn entry(window: &Node) -> Node {
    let e = window
        .wait_for(
            &Query::role(Role::Entry).name("Search tasks").showing(),
            WAIT,
        )
        .await
        .unwrap();
    tick_e2e::poll(WAIT, "search entry focused", || async {
        e.has_state(tick_e2e::State::Focused)
            .await
            .ok()
            .filter(|f| *f)
    })
    .await
    .unwrap();
    e
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn typing_shows_results_and_opens_them() {
    let (_server, s, window) = start().await;
    s.type_text("/").await.unwrap();
    entry(&window).await;
    // Before typing: the empty state, no results.
    app::task_list(&window)
        .await
        .wait_for(&Query::role(Role::ListItem).name("Search your tasks"), WAIT)
        .await
        .unwrap();

    // A title.
    s.type_text("deck").await.unwrap();
    // "Plan the deck" by title, "Draft the budget" by its description.
    app::wait_tasks(&window, &["Plan the deck", "Draft the budget"]).await;
    // The view names the result count in the subtitle.
    window
        .wait_for(&Query::role(Role::Label).name("2 tasks").showing(), WAIT)
        .await
        .unwrap();

    // Two words are both required, in any order.
    s.type_text(" plan").await.unwrap();
    app::wait_tasks(&window, &["Plan the deck"]).await;

    // Escape clears the entry (and keeps it focused).
    s.key(keysym::ESCAPE).await.unwrap();
    app::task_list(&window)
        .await
        .wait_for(&Query::role(Role::ListItem).name("Search your tasks"), WAIT)
        .await
        .unwrap();

    // No results: the empty state says so; completed tasks are not searched.
    s.type_text("already").await.unwrap();
    app::task_list(&window)
        .await
        .wait_for(
            &Query::role(Role::ListItem).name("No results for \"already\""),
            WAIT,
        )
        .await
        .unwrap();
    s.key(keysym::ESCAPE).await.unwrap();

    // Down moves to the results, J to the next one, Enter opens it.
    s.type_text("deck").await.unwrap();
    app::wait_tasks(&window, &["Plan the deck", "Draft the budget"]).await;
    s.key(keysym::DOWN).await.unwrap();
    let row = app::task_row(&window, "Plan the deck").await;
    tick_e2e::poll(WAIT, "first result focused", || async {
        row.has_state(tick_e2e::State::Focused)
            .await
            .ok()
            .filter(|f| *f)
    })
    .await
    .unwrap();
    s.key(u32::from(b'j')).await.unwrap();
    s.key(keysym::RETURN).await.unwrap();
    pane_title(&window, "Draft the budget").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn chinese_queries_match_titles_and_descriptions() {
    let (_server, s, window) = start().await;
    app::open_view(&s, &window, "Search").await;
    entry(&window).await;
    // Two characters (shorter than a trigram): both the title and the description.
    s.type_text("季度").await.unwrap();
    app::wait_tasks(&window, &["整理季度报告", "Draft the budget"]).await;
    // Three characters: the trigram index.
    s.type_text("报").await.unwrap();
    app::wait_tasks(&window, &["整理季度报告"]).await;
    // Enter opens the first result.
    s.key(keysym::RETURN).await.unwrap();
    pane_title(&window, "整理季度报告").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn escape_on_an_empty_entry_goes_back() {
    let (_server, s, window) = start().await;
    app::open_view(&s, &window, "Upcoming").await;
    s.type_text("/").await.unwrap();
    entry(&window).await;
    s.key(keysym::ESCAPE).await.unwrap();
    window
        .wait_gone(
            &Query::role(Role::Entry).name("Search tasks").showing(),
            WAIT,
        )
        .await
        .unwrap();
    // Back in Upcoming, with keyboard focus on its first task.
    let row = app::task_row(&window, "Renew passport").await;
    tick_e2e::poll(WAIT, "first task focused", || async {
        row.has_state(tick_e2e::State::Focused)
            .await
            .ok()
            .filter(|f| *f)
    })
    .await
    .unwrap();
    let sidebar = app::sidebar(&window).await;
    let upcoming = sidebar
        .wait_for(&Query::role(Role::ListItem).name("Upcoming"), WAIT)
        .await
        .unwrap();
    assert!(upcoming.has_state(tick_e2e::State::Selected).await.unwrap());
}
