//! Managing projects, sections, labels and filters (`DESIGN.md` 4.1, 4.19, 4.23 to 4.25),
//! asserted on the fake server.

use std::time::Duration;

use serde_json::Value;
use tick_e2e::{Node, Query, Role, Session, keysym};
use tick_fake_server::FakeServer;

use crate::detail::{menu_choose, picker_row};
use crate::{WAIT, app};

async fn start() -> (FakeServer, Session, Node) {
    let server = app::server_with(app::fixture()).await;
    let s = app::session(&server).start().await.unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    (server, s, window)
}

async fn button(window: &Node, name: &str) -> Node {
    let app = window.parent().await.unwrap().unwrap();
    app.wait_for(&Query::role(Role::Button).name(name).showing(), WAIT)
        .await
        .unwrap()
}

/// Any showing object named `name` (switch rows, radio swatches).
async fn named(window: &Node, role: Role, name: &str) -> Node {
    let app = window.parent().await.unwrap().unwrap();
    app.wait_for(&Query::role(role).name(name).showing(), WAIT)
        .await
        .unwrap()
}

/// Waits for a server object of `resource` with `field == value` and returns it.
async fn wait_object(server: &FakeServer, resource: &str, field: &str, value: &str) -> Value {
    tick_e2e::poll(
        Duration::from_secs(15),
        &format!("{resource} {field}={value}"),
        || async {
            let st = server.state();
            st.find(resource, field, value).cloned()
        },
    )
    .await
    .unwrap()
}

async fn wait_server(
    server: &FakeServer,
    what: &str,
    f: impl Fn(&tick_fake_server::State) -> bool,
) {
    let r = tick_e2e::poll(Duration::from_secs(15), what, || async {
        f(&server.state()).then_some(())
    })
    .await;
    assert!(r.is_ok(), "{what}");
}

/// The view menu's `index`-th item (4.19).
async fn view_menu(s: &Session, window: &Node, index: usize) {
    s.click(&button(window, "View Menu").await).await.unwrap();
    menu_choose(s, window, index).await;
}

/// Right-click a sidebar row and choose the `index`-th item of its menu.
async fn sidebar_menu(s: &Session, window: &Node, row: &str, index: usize) {
    let list = app::sidebar(window).await;
    let node = list
        .wait_for(&Query::role(Role::ListItem).name(row).showing(), WAIT)
        .await
        .unwrap();
    let r = node.extents().await.unwrap();
    s.input()
        .await
        .unwrap()
        .right_click(r.x + 80, r.y + r.height / 2)
        .await
        .unwrap();
    menu_choose(s, window, index).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn project_create_rename_color_favorite_delete() {
    let (server, s, window) = start().await;
    s.click(&button(&window, "Add Project").await)
        .await
        .unwrap();
    named(&window, Role::Dialog, "New Project").await;
    s.type_text("Garden").await.unwrap();
    s.click(&named(&window, Role::RadioButton, "Teal").await)
        .await
        .unwrap();
    named(&window, Role::Label, "Teal").await;
    s.click(&button(&window, "Add").await).await.unwrap();
    let p = wait_object(&server, "projects", "name", "Garden").await;
    assert_eq!(p["color"], "teal");
    let id = p["id"].as_str().unwrap().to_owned();
    // The new project opens.
    app::task_list(&window)
        .await
        .wait_for(&Query::role(Role::ListItem).name("Garden"), WAIT)
        .await
        .unwrap();

    // Rename in place from the view menu.
    view_menu(&s, &window, 0).await;
    named(&window, Role::Text, "Project name").await;
    s.chord(&[keysym::CONTROL_L, u32::from(b'a')])
        .await
        .unwrap();
    s.type_text("Allotment").await.unwrap();
    s.key(keysym::RETURN).await.unwrap();
    wait_server(&server, "renamed", |st| {
        st.get("projects", &id)
            .is_some_and(|p| p["name"] == "Allotment")
    })
    .await;

    // Color: the Edit Project dialog.
    view_menu(&s, &window, 1).await;
    named(&window, Role::Dialog, "Edit Project").await;
    s.click(&named(&window, Role::RadioButton, "Berry Red").await)
        .await
        .unwrap();
    s.click(&button(&window, "Save").await).await.unwrap();
    wait_server(&server, "color", |st| {
        st.get("projects", &id)
            .is_some_and(|p| p["color"] == "berry_red")
    })
    .await;

    // Favorite: it shows under Favorites.
    view_menu(&s, &window, 3).await;
    wait_server(&server, "favorite", |st| {
        st.get("projects", &id)
            .is_some_and(|p| p["is_favorite"] == true)
    })
    .await;
    app::sidebar(&window)
        .await
        .wait_for(
            &Query::role(Role::ListItem).name("Favorites, expanded"),
            WAIT,
        )
        .await
        .unwrap();

    // Delete, after the confirmation; the view falls back.
    view_menu(&s, &window, 5).await;
    named(&window, Role::Label, "Delete \"Allotment\"?").await;
    s.click(&button(&window, "Delete Project").await)
        .await
        .unwrap();
    wait_server(&server, "deleted", |st| st.get("projects", &id).is_none()).await;
    let sent = app::commands_sent(&server, "project_delete");
    assert_eq!(sent.len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn archive_with_undo_and_nesting() {
    let (server, s, window) = start().await;
    app::open_view(&s, &window, "Home").await;
    view_menu(&s, &window, 4).await;
    named(&window, Role::Label, "Archived \"Home\"").await;
    wait_server(&server, "archived", |st| {
        st.get("projects", "ph")
            .is_some_and(|p| p["is_archived"] == true)
    })
    .await;
    let list = app::sidebar(&window).await;
    list.wait_gone(&Query::role(Role::ListItem).name_contains("Home"), WAIT)
        .await
        .unwrap();
    s.click(&button(&window, "Undo").await).await.unwrap();
    wait_server(&server, "unarchived", |st| {
        st.get("projects", "ph")
            .is_some_and(|p| p["is_archived"] == false)
    })
    .await;
    s.input().await.unwrap().move_to(640, 300).await.unwrap();

    // "Move to..." from the sidebar menu: Home under Work.
    sidebar_menu(&s, &window, "Home, 2 tasks", 3).await;
    s.click(&picker_row(&window, "Work").await).await.unwrap();
    wait_server(&server, "nested", |st| {
        st.get("projects", "ph")
            .is_some_and(|p| p["parent_id"] == "pw")
    })
    .await;
    // Keyboard: Alt+Shift+Left moves it back out, after Work.
    let row = app::sidebar(&window)
        .await
        .wait_for(
            &Query::role(Role::ListItem).name("Home, 2 tasks").showing(),
            WAIT,
        )
        .await
        .unwrap();
    s.click(&row).await.unwrap();
    s.chord(&[keysym::ALT_L, keysym::SHIFT_L, keysym::LEFT])
        .await
        .unwrap();
    wait_server(&server, "un-nested", |st| {
        st.get("projects", "ph")
            .is_some_and(|p| p["parent_id"].is_null())
    })
    .await;
    // Alt+Shift+Up: before Work.
    tokio::time::sleep(Duration::from_millis(500)).await;
    s.chord(&[keysym::ALT_L, keysym::SHIFT_L, keysym::UP])
        .await
        .unwrap();
    wait_server(&server, "reordered", |st| {
        let key = |id: &str| {
            st.get("projects", id)
                .and_then(|p| p["order_key"].as_str().map(str::to_owned))
                .unwrap_or_default()
        };
        key("ph") < key("pw")
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sections_create_rename_delete() {
    let (server, s, window) = start().await;
    app::open_view(&s, &window, "Work").await;
    view_menu(&s, &window, 2).await;
    let editor = named(&window, Role::Text, "Section name").await;
    tick_e2e::poll(WAIT, "section editor focused", || async {
        editor
            .has_state(tick_e2e::State::Focused)
            .await
            .ok()
            .filter(|f| *f)
    })
    .await
    .unwrap();
    s.type_text("Next week").await.unwrap();
    s.key(keysym::RETURN).await.unwrap();
    let sec = wait_object(&server, "sections", "name", "Next week").await;
    assert_eq!(sec["project_id"], "pw");
    app::task_list(&window)
        .await
        .wait_for(
            &Query::role(Role::ListItem).name_contains("Next week, 0 tasks"),
            WAIT,
        )
        .await
        .unwrap();

    // Rename "This week" from its menu.
    s.click(&button(&window, "This week section menu").await)
        .await
        .unwrap();
    menu_choose(&s, &window, 0).await;
    named(&window, Role::Text, "Section name").await;
    s.type_text("Current week").await.unwrap();
    s.key(keysym::RETURN).await.unwrap();
    wait_server(&server, "section renamed", |st| {
        st.get("sections", "s1")
            .is_some_and(|v| v["name"] == "Current week")
    })
    .await;

    // Delete "Later" and its task.
    s.click(&button(&window, "Later section menu").await)
        .await
        .unwrap();
    menu_choose(&s, &window, 2).await;
    named(&window, Role::Label, "Delete section \"Later\"?").await;
    s.click(&button(&window, "Delete Section").await)
        .await
        .unwrap();
    wait_server(&server, "section deleted", |st| {
        st.get("sections", "s2").is_none() && st.get("items", "w3").is_none()
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn labels_create_edit_delete() {
    let (server, s, window) = start().await;
    s.click(&button(&window, "Add Label").await).await.unwrap();
    named(&window, Role::Dialog, "New Label").await;
    // A duplicate name keeps Add insensitive.
    s.type_text("Phone").await.unwrap();
    named(
        &window,
        Role::Label,
        "A label with this name already exists.",
    )
    .await;
    let add = button(&window, "Add").await;
    assert!(!add.has_state(tick_e2e::State::Sensitive).await.unwrap());
    s.chord(&[keysym::CONTROL_L, u32::from(b'a')])
        .await
        .unwrap();
    s.type_text("garden").await.unwrap();
    s.key(keysym::RETURN).await.unwrap();
    let l = wait_object(&server, "labels", "name", "garden").await;
    let id = l["id"].as_str().unwrap().to_owned();

    sidebar_menu(&s, &window, "errand, 2 tasks", 0).await;
    named(&window, Role::Dialog, "Edit Label").await;
    s.chord(&[keysym::CONTROL_L, u32::from(b'a')])
        .await
        .unwrap();
    s.type_text("errands").await.unwrap();
    s.click(&button(&window, "Save").await).await.unwrap();
    wait_server(&server, "label renamed on tasks", |st| {
        st.get("labels", "l2")
            .is_some_and(|v| v["name"] == "errands")
            && st
                .get("items", "w3")
                .is_some_and(|v| v["labels"][0] == "errands")
    })
    .await;

    sidebar_menu(&s, &window, "garden", 2).await;
    named(&window, Role::Label, "Delete label \"garden\"?").await;
    s.click(&button(&window, "Delete Label").await)
        .await
        .unwrap();
    wait_server(&server, "label deleted", |st| {
        st.get("labels", &id).is_none()
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn filters_create_edit_delete() {
    let (server, s, window) = start().await;
    s.click(&button(&window, "Add Filter").await).await.unwrap();
    named(&window, Role::Dialog, "New Filter").await;
    s.type_text("Today p1").await.unwrap();
    s.key(keysym::TAB).await.unwrap();
    // Focus moves to the Query row (Tab from the name entry).
    let query = named(&window, Role::Text, "Query").await;
    s.click(&query).await.unwrap();
    s.type_text("today & p1").await.unwrap();
    named(&window, Role::Label, "Runs on this computer.").await;
    s.click(&button(&window, "Add").await).await.unwrap();
    let f = wait_object(&server, "filters", "name", "Today p1").await;
    assert_eq!(f["query"], "today & p1");
    let id = f["id"].as_str().unwrap().to_owned();

    app::open_view(&s, &window, "Today p1").await;
    view_menu(&s, &window, 0).await;
    named(&window, Role::Dialog, "Edit Filter").await;
    let query = named(&window, Role::Text, "Query").await;
    s.click(&query).await.unwrap();
    s.chord(&[keysym::CONTROL_L, u32::from(b'a')])
        .await
        .unwrap();
    s.type_text("due before: tomorrow").await.unwrap();
    named(
        &window,
        Role::Label,
        "Part of this query is checked by Todoist. Results need a connection.",
    )
    .await;
    s.click(&button(&window, "Save").await).await.unwrap();
    wait_server(&server, "filter updated", |st| {
        st.get("filters", &id)
            .is_some_and(|v| v["query"] == "due before: tomorrow")
    })
    .await;

    view_menu(&s, &window, 2).await;
    named(&window, Role::Label, "Delete filter \"Today p1\"?").await;
    s.click(&button(&window, "Delete Filter").await)
        .await
        .unwrap();
    wait_server(&server, "filter deleted", |st| {
        st.get("filters", &id).is_none()
    })
    .await;
}

/// The rejected server filter offers "Edit Filter" (4.22).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rejected_filter_offers_edit() {
    let mut f = app::fixture();
    f["filters"].as_array_mut().unwrap().push(serde_json::json!(
        {"id": "f3", "name": "Broken", "query": "due before: ???", "item_order": 3}
    ));
    let server = app::server_with(f).await;
    server.inject(tick_fake_server::Failure::Status {
        status: 400,
        retry_after: None,
        count: 100,
        path: Some("/api/v1/tasks/filter".into()),
        body_contains: None,
    });
    let s = app::session(&server).start().await.unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    app::open_view(&s, &window, "Broken").await;
    let edit = window
        .wait_for(
            &Query::role(Role::Button).name("Edit Filter").showing(),
            Duration::from_secs(20),
        )
        .await
        .unwrap();
    s.click(&edit).await.unwrap();
    named(&window, Role::Dialog, "Edit Filter").await;
}
