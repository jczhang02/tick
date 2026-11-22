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
        let container = matches!(role, Role::List | Role::ScrollPane | Role::Frame);
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

async fn check(window: &Node, what: &str) {
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
        ("Before Monday", "Results from Todoist, updated 09:00"),
    ] {
        app::open_view(&s, &window, view).await;
        app::task_list(&window)
            .await
            .wait_for(&Query::role(Role::ListItem).name(first_row), WAIT)
            .await
            .unwrap();
        check(&window, view).await;
    }

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
