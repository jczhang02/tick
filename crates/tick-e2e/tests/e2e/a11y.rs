//! Accessibility audit of every main view: each focusable or actionable object that is on
//! screen has a non-empty accessible name, and each one is at least 44 x 44 px
//! (`DESIGN.md` 2.7).

use tick_e2e::{Node, Query, Role, Session, State};

use crate::{WAIT, app};

/// Roles that are actionable even when GTK does not report them focusable.
const ACTIONABLE: &[Role] = &[
    Role::Button,
    Role::ToggleButton,
    Role::CheckBox,
    Role::RadioButton,
    Role::Link,
    Role::MenuItem,
    Role::ListItem,
    Role::Entry,
    Role::Text,
    Role::PasswordText,
];

/// Offending objects on screen: (role, name, extents, problem).
async fn audit(root: &Node) -> Vec<String> {
    let mut problems = Vec::new();
    let mut stack = vec![root.clone()];
    let mut seen = 0;
    while let Some(node) = stack.pop() {
        seen += 1;
        assert!(seen < 20_000, "runaway tree");
        if let Ok(children) = node.children().await {
            stack.extend(children);
        }
        let Ok(states) = node.states().await else {
            continue;
        };
        if !states.contains(State::Showing) {
            continue;
        }
        let Ok(role) = node.role().await else {
            continue;
        };
        // `Panel` is GTK's generic role, which GTK never names (it ignores a label set on
        // it). libadwaita's dialog sheet is such a focusable container; the controls in it
        // are checked on their own.
        if role == Role::Panel {
            continue;
        }
        // Links inside rendered text (descriptions) are exemption 1 in DESIGN 1.2.1: GTK 4.22
        // exposes them as unnamed objects with no real extents.
        if role == Role::Link {
            continue;
        }
        let interactive = states.contains(State::Focusable) || ACTIONABLE.contains(&role);
        // Rows that are not focusable are display-only (view, day and month headers).
        if !interactive || (role == Role::ListItem && !states.contains(State::Focusable)) {
            continue;
        }
        let name = node.name().await.unwrap_or_default();
        let rect = node.extents().await.ok();
        if name.trim().is_empty() {
            problems.push(format!("{role:?} without a name at {rect:?}"));
        }
        // Lists and scroll panes are containers, not targets; text fields measure the
        // whole row or entry they sit in.
        // Labels are text: a focusable one is selectable text (comments), read, not
        // clicked. The one label that is a click target, the description, is measured on
        // its own in `editing_surfaces_have_named_44px_targets`.
        let container = matches!(
            role,
            Role::List | Role::ScrollPane | Role::Frame | Role::Label
        );
        if let Some(r) = rect
            && !container
            && (r.width < 44 || r.height < 44)
            && !hit_area_from_parent(&node, &name).await
        {
            problems.push(format!("{role:?} {name:?} is {}x{} px", r.width, r.height));
        }
    }
    problems
}

/// A `GtkMenuButton` carries the 44 px hit area on its outer node (technique 2 in 2.7: the
/// border goes on `menubutton`, its inner toggle stays 34 px); AT-SPI shows the outer node as
/// a same-named parent.
async fn hit_area_from_parent(node: &Node, name: &str) -> bool {
    let Ok(Some(parent)) = node.parent().await else {
        return false;
    };
    let same_name = parent.name().await.is_ok_and(|n| n == name);
    let big = parent
        .extents()
        .await
        .is_ok_and(|r| r.width >= 44 && r.height >= 44);
    same_name && big
}

pub async fn check(window: &Node, what: &str) {
    let problems = audit(window).await;
    assert!(problems.is_empty(), "{what}:\n{}", problems.join("\n"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_view_has_named_44px_targets() {
    let server = app::server_with(app::fixture()).await;
    let s = app::session(&server).start().await.unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    app::wait_tasks(
        &window,
        &["Renew passport", "Plan the deck", "Call the dentist"],
    )
    .await;
    check(&window, "Today").await;
    for (view, first_row) in [
        ("Inbox", "Inbox"),
        ("Upcoming", "Upcoming"),
        ("Work", "Work"),
        ("phone", "phone"),
        ("Urgent", "Urgent"),
        // The source line of server results is a label in the view header.
        ("Before Monday", "Results from Todoist, updated 09:00"),
    ] {
        app::open_view(&s, &window, view).await;
        let role = if first_row.starts_with("Results") {
            Role::Label
        } else {
            Role::ListItem
        };
        app::task_list(&window)
            .await
            .wait_for(&Query::role(role).name(first_row), WAIT)
            .await
            .unwrap();
        check(&window, view).await;
    }

    // The Undo toast (its buttons are targets too).
    app::open_view(&s, &window, "Inbox").await;
    let row = app::task_row(&window, "Read the article").await;
    let ring = app::inside(&row, Query::role(Role::CheckBox)).await;
    s.click(&ring).await.unwrap();
    window
        .wait_for(&Query::role(Role::Button).name("Undo").showing(), WAIT)
        .await
        .unwrap();
    check(&window, "undo toast").await;

    // The sync status button.
    server.set_offline(true);
    window
        .wait_for(
            &Query::role(Role::Button).name("Sync Status").showing(),
            std::time::Duration::from_secs(15),
        )
        .await
        .unwrap();
    check(&window, "sync status").await;
    server.set_offline(false);

    // Quick add dialog.
    s.key(u32::from(b'q')).await.unwrap();
    window
        .wait_for(&Query::role(Role::Text).name("Task").showing(), WAIT)
        .await
        .unwrap();
    s.type_text("Call Mom p2 #Work").await.unwrap();
    check(&window, "quick add").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn login_page_has_named_44px_targets() {
    let server = app::server_with(app::fixture()).await;
    let s = Session::builder()
        .env("TICK_API_BASE", server.base_url())
        .start()
        .await
        .unwrap();
    let window = s.window().await.unwrap();
    window
        .wait_for(&Query::role(Role::PasswordText).showing(), WAIT)
        .await
        .unwrap();
    check(&window, "login").await;
}

/// The narrow layout's root page (sidebar) and list page.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn narrow_pages_have_named_44px_targets() {
    let server = app::server_with(app::fixture()).await;
    let s = app::session(&server).narrow().start().await.unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    app::sidebar(&window).await;
    check(&window, "narrow sidebar").await;
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
    check(&window, "narrow list").await;
}

/// The detail pane, every picker, the menus and the dialogs (`DESIGN.md` 4.8 to 4.10,
/// 4.19, 4.23, 4.25).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn editing_surfaces_have_named_44px_targets() {
    let mut f = app::fixture();
    f["projects"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!(
            {"id": "pt", "name": "Team", "color": "green", "shared": true, "child_order": 3}
        ));
    f["collaborators"] = serde_json::json!([
        {"id": "u2", "full_name": "Ana Silva", "email": "ana@example.com"}
    ]);
    f["collaborator_states"] =
        serde_json::json!([{"project_id": "pt", "user_id": "u2", "state": "active"}]);
    f["items"].as_array_mut().unwrap().extend([
        serde_json::json!({"id": "t1", "project_id": "pt", "content": "Draft the agenda",
                           "description": "See **notes** at https://example.com",
                           "child_order": 1}),
        serde_json::json!({"id": "t2", "project_id": "pt", "parent_id": "t1",
                           "content": "Book a room", "child_order": 1}),
    ]);
    f["notes"] = serde_json::json!([
        {"id": "n1", "item_id": "t1", "content": "Room 4 is free.", "posted_uid": "u2",
         "file_attachment": {"file_name": "plan.pdf", "file_url": "https://example.com/plan.pdf"}}
    ]);
    let server = app::server_with(f).await;
    let s = app::session(&server).start().await.unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    app::open_view(&s, &window, "Team").await;
    crate::detail::open_task(&s, &window, "Draft the agenda").await;
    check(&window, "detail pane").await;
    // The description is a click target (it opens the editor): 44 px tall even as one line.
    // (The list row's description preview has the same text; the pane's is the one at
    // the window's end.)
    let mut best = None;
    for n in window
        .find_all(
            &Query::role(Role::Label)
                .name_contains("See notes")
                .showing(),
        )
        .await
    {
        let r = n.extents().await.unwrap();
        if best.as_ref().is_none_or(|b: &tick_e2e::Rect| r.x > b.x) {
            best = Some(r);
        }
    }
    let d = best.expect("description label");
    assert!(d.height >= 44, "{d:?}");

    for field in ["Date", "Deadline", "Priority", "Labels", "Assignee"] {
        s.click(&crate::detail::field(&window, field).await)
            .await
            .unwrap();
        window
            .wait_for(&Query::role(Role::List).showing(), WAIT)
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        check(&window, &format!("{field} picker")).await;
        s.key(tick_e2e::keysym::ESCAPE).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }
    let crumb = window
        .wait_for(
            &Query::role(Role::Button).name_contains("Move to").showing(),
            WAIT,
        )
        .await
        .unwrap();
    s.click(&crumb).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    check(&window, "move picker").await;
    s.key(tick_e2e::keysym::ESCAPE).await.unwrap();

    // The task context menu and the view menu: items are named (4.19).
    let row = app::task_row(&window, "Draft the agenda").await;
    let r = row.extents().await.unwrap();
    s.input()
        .await
        .unwrap()
        .right_click(r.x + 150, r.y + 20)
        .await
        .unwrap();
    window
        .wait_for(&Query::role(Role::MenuItem).name("Open").showing(), WAIT)
        .await
        .unwrap();
    check(&window, "task menu").await;
    s.key(tick_e2e::keysym::ESCAPE).await.unwrap();
    let view_menu = window
        .wait_for(&Query::role(Role::Button).name("View Menu").showing(), WAIT)
        .await
        .unwrap();
    s.click(&view_menu).await.unwrap();
    window
        .wait_for(
            &Query::role(Role::MenuItem).name("Add Section").showing(),
            WAIT,
        )
        .await
        .unwrap();
    check(&window, "view menu").await;

    // Delete Project...: the confirmation.
    let delete = window
        .wait_for(
            &Query::role(Role::MenuItem)
                .name("Delete Project...")
                .showing(),
            WAIT,
        )
        .await
        .unwrap();
    s.click(&delete).await.unwrap();
    window
        .wait_for(
            &Query::role(Role::Button).name("Delete Project").showing(),
            WAIT,
        )
        .await
        .unwrap();
    check(&window, "delete confirmation").await;
    s.key(tick_e2e::keysym::ESCAPE).await.unwrap();

    // The project dialog with its color grid.
    let add = window
        .wait_for(
            &Query::role(Role::Button).name("Add Project").showing(),
            WAIT,
        )
        .await
        .unwrap();
    s.click(&add).await.unwrap();
    window
        .wait_for(&Query::role(Role::RadioButton).name("Teal").showing(), WAIT)
        .await
        .unwrap();
    check(&window, "project dialog").await;
}
