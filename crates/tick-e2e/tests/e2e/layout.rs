//! Window forms per breakpoint (`DESIGN.md` 3.3): docked panes on wide and medium windows,
//! the sidebar as a navigation page on narrow ones.

use tick_e2e::{Node, Query, Role};

use crate::{WAIT, app};

async fn showing(window: &Node, role: Role, name: &str) -> Vec<Node> {
    window
        .find_all(&Query::role(role).name(name).showing())
        .await
}

/// 600 x 800 is B2: the sidebar is the root page; choosing a view pushes the list.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn narrow_window_uses_pages() {
    let server = app::server_with(app::fixture()).await;
    let s = app::session(&server).narrow().start().await.unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;

    // Root page: the sidebar fills the window; its header bar offers "Add Task" (B2).
    let sidebar = app::sidebar(&window).await;
    let rect = sidebar.extents().await.unwrap();
    assert!(rect.width >= 560, "sidebar {rect:?}");
    assert!(
        showing(&window, Role::List, "Tasks").await.is_empty(),
        "the task list is a separate page"
    );
    assert_eq!(showing(&window, Role::Button, "Add Task").await.len(), 1);

    app::open_view(&s, &window, "Inbox").await;
    let list = app::task_list(&window).await;
    app::wait_tasks(
        &window,
        &["Call the dentist", "Renew passport", "Read the article"],
    )
    .await;
    let rect = list.extents().await.unwrap();
    assert!(rect.width >= 560, "task list {rect:?}");
    assert!(showing(&window, Role::List, "Views").await.is_empty());

    // The automatic back button returns to the sidebar.
    let back = window
        .wait_for(&Query::role(Role::Button).name("Back").showing(), WAIT)
        .await
        .unwrap();
    s.click(&back).await.unwrap();
    app::sidebar(&window).await;
    window
        .wait_gone(&Query::role(Role::List).name("Tasks").showing(), WAIT)
        .await
        .unwrap();
}

/// 900 x 800 is B1 (detail pane would overlay) and 1280 x 800 is wide: sidebar and list
/// are both docked, and "Add Task" is only in the list's header bar.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn medium_and_wide_windows_dock_the_sidebar() {
    for (w, h) in [(900, 800), (1280, 800)] {
        let server = app::server_with(app::fixture()).await;
        let s = app::session(&server).monitor(w, h).start().await.unwrap();
        let window = s.window().await.unwrap();
        app::wait_first_sync(&server).await;
        let sidebar = app::sidebar(&window).await.extents().await.unwrap();
        let list = app::task_list(&window).await.extents().await.unwrap();
        // DESIGN 3.2: the sidebar takes 22 % within 240..300 px.
        assert!(
            (240..=300).contains(&sidebar.width),
            "{w}x{h}: sidebar {sidebar:?}"
        );
        assert!(
            list.x >= sidebar.x + sidebar.width,
            "{w}x{h}: list {list:?}"
        );
        // The list column is clamped to 720 px.
        assert!(list.width <= 720, "{w}x{h}: list {list:?}");
        assert_eq!(showing(&window, Role::Button, "Add Task").await.len(), 1);
    }
}

/// Sidebar section headers collapse on click and stay collapsed after a relaunch.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sidebar_sections_collapse_and_persist() {
    let server = app::server_with(app::fixture()).await;
    let mut s = app::session(&server).start().await.unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    let sidebar = app::sidebar(&window).await;
    let projects = sidebar
        .wait_for(
            &Query::role(Role::ListItem).name("Projects, expanded"),
            WAIT,
        )
        .await
        .unwrap();
    sidebar
        .wait_for(&Query::role(Role::ListItem).name_contains("Work"), WAIT)
        .await
        .unwrap();
    s.click(&projects).await.unwrap();
    sidebar
        .wait_for(
            &Query::role(Role::ListItem).name("Projects, collapsed"),
            WAIT,
        )
        .await
        .unwrap();
    sidebar
        .wait_gone(&Query::role(Role::ListItem).name_contains("Work"), WAIT)
        .await
        .unwrap();

    s.activate_action("quit", None).await.unwrap();
    s.wait_app_exit(WAIT).await.unwrap();
    s.launch_app().unwrap();
    let window = s.window().await.unwrap();
    let sidebar = app::sidebar(&window).await;
    sidebar
        .wait_for(
            &Query::role(Role::ListItem).name("Projects, collapsed"),
            WAIT,
        )
        .await
        .unwrap();
    assert!(
        sidebar
            .find(&Query::role(Role::ListItem).name_contains("Work"))
            .await
            .is_none()
    );
}

/// An empty account keeps the Projects, Labels and Filters headers (they say where those
/// will appear); only Favorites hides when empty (4.1).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn empty_account_keeps_sidebar_headers() {
    let server = app::server("empty").await;
    let s = app::session(&server).start().await.unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    let sidebar = app::sidebar(&window).await;
    let want = [
        "Inbox",
        "Today",
        "Upcoming",
        "Search",
        "Completed",
        "Projects, expanded",
        "Labels, expanded",
        "Filters, expanded",
    ];
    let names = || async {
        let mut out = Vec::new();
        for n in sidebar.find_all(&Query::role(Role::ListItem)).await {
            out.push(n.name().await.unwrap_or_default());
        }
        out
    };
    let ok = tick_e2e::poll(WAIT, "sidebar rows", || async {
        (names().await == want).then_some(())
    })
    .await;
    assert!(ok.is_ok(), "sidebar rows: {:?}", names().await);
}
