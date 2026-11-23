//! The task detail pane (`DESIGN.md` 4.8): every edit reaches the fake server with the
//! right command and arguments, and the list row follows.

use std::time::Duration;

use serde_json::Value;
use tick_e2e::{Node, Query, Role, Session, keysym};
use tick_fake_server::FakeServer;

use crate::{WAIT, app};

/// The fake server, a session in the Inbox of [`app::fixture`] with "Read the article" open.
pub async fn open_in_inbox(title: &str) -> (FakeServer, Session, Node) {
    let server = app::server_with(app::fixture()).await;
    let s = app::session(&server).start().await.unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    app::open_view(&s, &window, "Inbox").await;
    open_task(&s, &window, title).await;
    (server, s, window)
}

/// Click a task row's title and wait for the pane.
pub async fn open_task(s: &Session, window: &Node, title: &str) {
    let row = app::task_row(window, title).await;
    let r = row.extents().await.unwrap();
    // The title, right of the 44 px ring.
    s.input()
        .await
        .unwrap()
        .click(r.x + 120, r.y + 22)
        .await
        .unwrap();
    pane_title(window, title).await;
}

/// The pane's title editor showing `title`.
pub async fn pane_title(window: &Node, title: &str) -> Node {
    let node = tick_e2e::poll(WAIT, "the detail title", || async {
        for name in ["Task title", "任务标题"] {
            if let Some(n) = window
                .find(&Query::role(Role::Text).name(name).showing())
                .await
            {
                return Some(n);
            }
        }
        None
    })
    .await
    .unwrap();
    let got = tick_e2e::poll(WAIT, "detail title", || async {
        node.text().await.ok().filter(|t| t == title)
    })
    .await;
    assert!(
        got.is_ok(),
        "title: {:?}, want {title:?}",
        node.text().await
    );
    node
}

/// The detail field row button whose name starts with `field`.
pub async fn field(window: &Node, field: &str) -> Node {
    window
        .wait_for(
            &Query::role(Role::Button)
                .name_contains(if field.contains(": ") {
                    field.to_owned()
                } else {
                    format!("{field}: ")
                })
                .showing(),
            WAIT,
        )
        .await
        .unwrap()
}

/// Waits for an item on the fake server to satisfy `f`.
pub async fn wait_item(server: &FakeServer, id: &str, what: &str, f: impl Fn(&Value) -> bool) {
    let r = tick_e2e::poll(Duration::from_secs(15), what, || async {
        let st = server.state();
        let v = st.get("items", id)?;
        f(v).then_some(())
    })
    .await;
    assert!(r.is_ok(), "{what}: {:?}", server.state().get("items", id));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn title_and_description_edits() {
    let (server, s, window) = open_in_inbox("Read the article").await;
    let title = pane_title(&window, "Read the article").await;
    s.click(&title).await.unwrap();
    s.chord(&[keysym::CONTROL_L, u32::from(b'a')])
        .await
        .unwrap();
    s.type_text("Read the long article").await.unwrap();
    s.key(keysym::RETURN).await.unwrap();
    wait_item(&server, "i3", "renamed", |v| {
        v["content"] == "Read the long article"
    })
    .await;
    let sent = app::commands_sent(&server, "item_update");
    assert!(
        sent.iter()
            .any(|c| c["args"]["id"] == "i3" && c["args"]["content"] == "Read the long article"),
        "{sent:?}"
    );
    app::wait_tasks(
        &window,
        &[
            "Call the dentist",
            "Renew passport",
            "Read the long article",
        ],
    )
    .await;

    // Description: click the placeholder, type, Ctrl+Enter saves.
    let desc = window
        .wait_for(
            &Query::role(Role::Label).name("Add a description").showing(),
            WAIT,
        )
        .await
        .unwrap();
    s.click(&desc).await.unwrap();
    let editor = window
        .wait_for(&Query::role(Role::Text).name("Description").showing(), WAIT)
        .await
        .unwrap();
    tick_e2e::poll(WAIT, "description editor focused", || async {
        editor
            .has_state(tick_e2e::State::Focused)
            .await
            .ok()
            .filter(|f| *f)
    })
    .await
    .unwrap();
    s.type_text("Some **bold** notes").await.unwrap();
    s.chord(&[keysym::CONTROL_L, keysym::RETURN]).await.unwrap();
    wait_item(&server, "i3", "description", |v| {
        v["description"] == "Some **bold** notes"
    })
    .await;
    let row = app::task_row(&window, "Read the long article").await;
    tick_e2e::poll(WAIT, "description preview", || async {
        let n = row
            .find(&Query::role(Role::Label).name("Some bold notes"))
            .await;
        n.map(|_| ())
    })
    .await
    .unwrap();
}

/// A showing picker row (`ListItem` or `CheckBox`) named `name`.
pub async fn picker_row(window: &Node, name: &str) -> Node {
    let app = window.parent().await.unwrap().unwrap();
    let found = tick_e2e::poll(WAIT, &format!("picker row {name:?}"), || async {
        for role in [Role::ListItem, Role::CheckBox] {
            if let Some(n) = app.find(&Query::role(role).name(name).showing()).await {
                return Some(n);
            }
        }
        None
    })
    .await;
    match found {
        Ok(n) => n,
        Err(e) => panic!("{e}\n{}", app.dump().await),
    }
}

/// Waits until no popover row named `name` shows (the picker closed).
async fn picker_gone(window: &Node, name: &str) {
    let app = window.parent().await.unwrap().unwrap();
    app.wait_gone(&Query::role(Role::ListItem).name(name).showing(), WAIT)
        .await
        .unwrap();
}

fn update_sent(server: &FakeServer, id: &str, key: &str) -> Vec<Value> {
    app::commands_sent(server, "item_update")
        .into_iter()
        .filter(|c| c["args"]["id"] == id && c["args"].get(key).is_some())
        .map(|c| c["args"][key].clone())
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn due_by_picker_text_and_clear() {
    let (server, s, window) = open_in_inbox("Read the article").await;
    // Quick option: Tomorrow (date arithmetic, sent as a date).
    s.click(&field(&window, "Date").await).await.unwrap();
    s.click(&picker_row(&window, "Tomorrow").await)
        .await
        .unwrap();
    picker_gone(&window, "Tomorrow").await;
    wait_item(&server, "i3", "due tomorrow", |v| {
        v["due"]["date"] == "2026-11-23"
    })
    .await;
    assert_eq!(
        update_sent(&server, "i3", "due")[0],
        serde_json::json!({"date": "2026-11-23"})
    );
    field(&window, "Date: Tomorrow").await;
    let row = app::task_row(&window, "Read the article").await;
    assert!(
        row.description().await.unwrap().contains("Due Tomorrow"),
        "{:?}",
        row.description().await
    );

    // Free text: the server parses it.
    s.click(&field(&window, "Date").await).await.unwrap();
    let entry = window
        .wait_for(&Query::role(Role::Text).name("Date text").showing(), WAIT)
        .await
        .unwrap();
    tick_e2e::poll(WAIT, "date entry focused", || async {
        entry
            .has_state(tick_e2e::State::Focused)
            .await
            .ok()
            .filter(|f| *f)
    })
    .await
    .unwrap();
    s.type_text("every monday").await.unwrap();
    s.key(keysym::RETURN).await.unwrap();
    wait_item(&server, "i3", "recurring due", |v| {
        v["due"]["is_recurring"] == true && v["due"]["string"] == "every monday"
    })
    .await;
    assert!(
        update_sent(&server, "i3", "due").contains(&serde_json::json!({"string": "every monday"}))
    );
    // The recurrence shows under the date.
    window
        .wait_for(
            &Query::role(Role::Label).name("every monday").showing(),
            WAIT,
        )
        .await
        .unwrap();

    // "No Date" clears.
    s.click(&field(&window, "Date").await).await.unwrap();
    s.click(&picker_row(&window, "No Date").await)
        .await
        .unwrap();
    wait_item(&server, "i3", "no due", |v| v["due"].is_null()).await;
    field(&window, "Date: No date").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deadline_set_and_clear() {
    let (server, s, window) = open_in_inbox("Read the article").await;
    s.click(&field(&window, "Deadline").await).await.unwrap();
    // The deadline picker has no text entry (4.9).
    s.click(&picker_row(&window, "Next Week").await)
        .await
        .unwrap();
    wait_item(&server, "i3", "deadline", |v| {
        v["deadline"]["date"] == "2026-11-23"
    })
    .await;
    assert_eq!(
        update_sent(&server, "i3", "deadline")[0],
        serde_json::json!({"date": "2026-11-23"})
    );
    let row = app::task_row(&window, "Read the article").await;
    tick_e2e::poll(WAIT, "row deadline", || async {
        row.description()
            .await
            .ok()
            .filter(|d| d.contains("deadline Tomorrow"))
    })
    .await
    .unwrap();
    // The clear button at the row end.
    let clear = window
        .wait_for(
            &Query::role(Role::Button).name("Clear Deadline").showing(),
            WAIT,
        )
        .await
        .unwrap();
    s.click(&clear).await.unwrap();
    wait_item(&server, "i3", "no deadline", |v| v["deadline"].is_null()).await;
    assert!(update_sent(&server, "i3", "deadline").contains(&Value::Null));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn priority_and_labels() {
    let (server, s, window) = open_in_inbox("Read the article").await;
    s.click(&field(&window, "Priority").await).await.unwrap();
    s.click(&picker_row(&window, "Priority 1").await)
        .await
        .unwrap();
    // API priority 4 is p1 (7.4 pitfall 10).
    wait_item(&server, "i3", "p1", |v| v["priority"] == 4).await;
    let row = app::task_row(&window, "Read the article").await;
    row.wait_for(&Query::role(Role::CheckBox).name("Complete task"), WAIT)
        .await
        .unwrap();

    // Labels apply live; a missing one is created.
    s.click(&field(&window, "Labels").await).await.unwrap();
    s.click(&picker_row(&window, "phone").await).await.unwrap();
    wait_item(&server, "i3", "label phone", |v| {
        v["labels"] == serde_json::json!(["phone"])
    })
    .await;
    s.type_text("urgent").await.unwrap();
    s.key(keysym::RETURN).await.unwrap();
    wait_item(&server, "i3", "label urgent", |v| {
        v["labels"] == serde_json::json!(["phone", "urgent"])
    })
    .await;
    assert!(
        server.state().find("labels", "name", "urgent").is_some(),
        "label_add"
    );
    // Unchecking removes it; Escape closes.
    s.click(&picker_row(&window, "phone").await).await.unwrap();
    wait_item(&server, "i3", "label removed", |v| {
        v["labels"] == serde_json::json!(["urgent"])
    })
    .await;
    s.key(keysym::ESCAPE).await.unwrap();
    field(&window, "Labels: urgent").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn move_with_the_crumb() {
    let (server, s, window) = open_in_inbox("Read the article").await;
    let crumb = window
        .wait_for(
            &Query::role(Role::Button).name_contains("Move to").showing(),
            WAIT,
        )
        .await
        .unwrap();
    s.click(&crumb).await.unwrap();
    s.click(&picker_row(&window, "This week").await)
        .await
        .unwrap();
    wait_item(&server, "i3", "moved", |v| v["section_id"] == "s1").await;
    let moves = app::commands_sent(&server, "item_move");
    assert_eq!(moves[0]["args"]["section_id"], "s1", "{moves:?}");
    // Left the Inbox; the pane follows it.
    app::wait_tasks(&window, &["Call the dentist", "Renew passport"]).await;
    window
        .wait_for(
            &Query::role(Role::Button)
                .name_contains("Work / This week")
                .showing(),
            WAIT,
        )
        .await
        .unwrap();
    window
        .wait_for(&Query::role(Role::Button).name("Undo").showing(), WAIT)
        .await
        .unwrap();
}

fn shared_fixture() -> Value {
    let mut f = app::fixture();
    f["projects"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!(
            {"id": "pt", "name": "Team", "color": "green", "shared": true, "child_order": 3}
        ));
    f["collaborators"] = serde_json::json!([
        {"id": "u2", "full_name": "Ana Silva", "email": "ana@example.com"},
        {"id": "u3", "full_name": "Ben Ode", "email": "ben@example.com"}
    ]);
    f["collaborator_states"] = serde_json::json!([
        {"project_id": "pt", "user_id": "u2", "state": "active"},
        {"project_id": "pt", "user_id": "u3", "state": "active"}
    ]);
    f["items"].as_array_mut().unwrap().push(serde_json::json!(
        {"id": "t1", "project_id": "pt", "content": "Draft the agenda", "child_order": 1}
    ));
    f
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn assign_and_unassign() {
    let server = app::server_with(shared_fixture()).await;
    let s = app::session(&server).start().await.unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    app::open_view(&s, &window, "Team").await;
    open_task(&s, &window, "Draft the agenda").await;
    s.click(&field(&window, "Assignee").await).await.unwrap();
    s.click(&picker_row(&window, "Ana Silva").await)
        .await
        .unwrap();
    wait_item(&server, "t1", "assigned", |v| v["responsible_uid"] == "u2").await;
    let row = app::task_row(&window, "Draft the agenda").await;
    row.wait_for(
        &Query::role(Role::Image).name("Assigned to Ana Silva"),
        WAIT,
    )
    .await
    .unwrap();
    s.click(&field(&window, "Assignee").await).await.unwrap();
    s.click(&picker_row(&window, "Unassigned").await)
        .await
        .unwrap();
    wait_item(&server, "t1", "unassigned", |v| {
        v["responsible_uid"].is_null()
    })
    .await;
    assert!(update_sent(&server, "t1", "responsible_uid").contains(&Value::Null));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn subtasks_add_complete_and_open() {
    let server = app::server_with(app::fixture()).await;
    let s = app::session(&server).start().await.unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    app::open_view(&s, &window, "Work").await;
    open_task(&s, &window, "Plan the deck").await;
    let add = window
        .wait_for(
            &Query::role(Role::Button).name("Add sub-task").showing(),
            WAIT,
        )
        .await
        .unwrap();
    s.click(&add).await.unwrap();
    let entry = window
        .wait_for(
            &Query::role(Role::Text).name("Sub-task name").showing(),
            WAIT,
        )
        .await
        .unwrap();
    tick_e2e::poll(WAIT, "sub-task entry focused", || async {
        entry
            .has_state(tick_e2e::State::Focused)
            .await
            .ok()
            .filter(|f| *f)
    })
    .await
    .unwrap();
    s.type_text("Book a room").await.unwrap();
    s.key(keysym::RETURN).await.unwrap();
    let added = tick_e2e::poll(
        Duration::from_secs(15),
        "sub-task on the server",
        || async {
            let st = server.state();
            st.find("items", "content", "Book a room").cloned()
        },
    )
    .await
    .unwrap();
    assert_eq!(added["parent_id"], "w1", "{added}");
    // The list shows it under its parent.
    app::wait_tasks(
        &window,
        &[
            "Book the venue",
            "Plan the deck",
            "Collect metrics",
            "Book a room",
            "Clean up flags",
        ],
    )
    .await;

    // Complete a sub-task from the pane.
    let open = window
        .wait_for(
            &Query::role(Role::Button)
                .name("Open Collect metrics")
                .showing(),
            WAIT,
        )
        .await
        .unwrap();
    let r = open.extents().await.unwrap();
    // Its ring sits right before the open button.
    s.input()
        .await
        .unwrap()
        .click(r.x - 22, r.y + 22)
        .await
        .unwrap();
    let closed = tick_e2e::poll(Duration::from_secs(20), "item_close w2", || async {
        let c = app::commands_sent(&server, "item_close");
        (!c.is_empty()).then_some(c)
    })
    .await
    .unwrap();
    assert_eq!(closed[0]["args"]["id"], "w2");

    // Open a sub-task.
    let open = window
        .wait_for(
            &Query::role(Role::Button).name("Open Book a room").showing(),
            WAIT,
        )
        .await
        .unwrap();
    s.click(&open).await.unwrap();
    pane_title(&window, "Book a room").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn comments_add_edit_delete() {
    let (server, s, window) = open_in_inbox("Read the article").await;
    let composer = window
        .wait_for(
            &Query::role(Role::Text).name("Write a comment").showing(),
            WAIT,
        )
        .await
        .unwrap();
    s.click(&composer).await.unwrap();
    s.type_text("First note").await.unwrap();
    s.key(keysym::RETURN).await.unwrap();
    let note = tick_e2e::poll(Duration::from_secs(15), "note_add", || async {
        let st = server.state();
        st.find("notes", "content", "First note").cloned()
    })
    .await
    .unwrap();
    assert_eq!(note["item_id"], "i3");
    let id = note["id"].as_str().unwrap().to_owned();
    window
        .wait_for(&Query::role(Role::Label).name("First note").showing(), WAIT)
        .await
        .unwrap();
    // The row's meta line counts it.
    let row = app::task_row(&window, "Read the article").await;
    tick_e2e::poll(WAIT, "comment count", || async {
        row.description()
            .await
            .ok()
            .filter(|d| d.contains("1 comment"))
    })
    .await
    .unwrap();

    // Edit.
    let actions = window
        .wait_for(
            &Query::role(Role::Button).name("Comment Actions").showing(),
            WAIT,
        )
        .await
        .unwrap();
    s.click(&actions).await.unwrap();
    menu_choose(&s, &window, 0).await;
    let editor = window
        .wait_for(
            &Query::role(Role::Text).name("Edit comment").showing(),
            WAIT,
        )
        .await
        .unwrap();
    tick_e2e::poll(WAIT, "comment editor focused", || async {
        editor
            .has_state(tick_e2e::State::Focused)
            .await
            .ok()
            .filter(|f| *f)
    })
    .await
    .unwrap();
    s.chord(&[keysym::CONTROL_L, u32::from(b'a')])
        .await
        .unwrap();
    s.type_text("Edited note").await.unwrap();
    s.key(keysym::RETURN).await.unwrap();
    tick_e2e::poll(Duration::from_secs(15), "note_update", || async {
        let st = server.state();
        (st.get("notes", &id)?["content"] == "Edited note").then_some(())
    })
    .await
    .unwrap();
    window
        .wait_for(
            &Query::role(Role::Label).name("Edited note").showing(),
            WAIT,
        )
        .await
        .unwrap();

    // Delete, after the confirmation.
    let actions = window
        .wait_for(
            &Query::role(Role::Button).name("Comment Actions").showing(),
            WAIT,
        )
        .await
        .unwrap();
    s.click(&actions).await.unwrap();
    menu_choose(&s, &window, 1).await;
    let confirm = window
        .wait_for(
            &Query::role(Role::Button).name("Delete Comment").showing(),
            WAIT,
        )
        .await
        .unwrap();
    s.click(&confirm).await.unwrap();
    tick_e2e::poll(Duration::from_secs(15), "note_delete", || async {
        let st = server.state();
        (st.get("notes", &id).is_none()).then_some(())
    })
    .await
    .unwrap();
    window
        .wait_gone(
            &Query::role(Role::Label).name("Edited note").showing(),
            WAIT,
        )
        .await
        .unwrap();
}

/// Activate the `index`-th item of the menu that just opened, by keyboard. GTK 4.22 does
/// not name `popover.menu` items on AT-SPI (their label is not exposed), so tests cannot
/// find them by name; the first item has focus when the menu opens.
pub async fn menu_choose(s: &Session, window: &Node, index: usize) {
    let app = window.parent().await.unwrap().unwrap();
    app.wait_for(
        &Query::role(Role::MenuItem)
            .state(tick_e2e::State::Focused)
            .showing(),
        WAIT,
    )
    .await
    .unwrap();
    for _ in 0..index {
        s.key(keysym::DOWN).await.unwrap();
    }
    s.key(keysym::RETURN).await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn attachments_open() {
    let mut f = app::fixture();
    f["notes"] = serde_json::json!([
        {"id": "n1", "item_id": "i3", "content": "See the scan.", "posted_uid": "u9",
         "file_attachment": {"file_name": "scan.pdf", "file_type": "application/pdf",
                             "file_url": "https://example.com/scan.pdf"}}
    ]);
    f["collaborators"] = serde_json::json!([{"id": "u9", "full_name": "Ana Silva"}]);
    let server = app::server_with(f).await;
    let s = app::session(&server).start().await.unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    app::open_view(&s, &window, "Inbox").await;
    open_task(&s, &window, "Read the article").await;
    let button = window
        .wait_for(
            &Query::role(Role::Button)
                .name("Open attachment scan.pdf")
                .showing(),
            WAIT,
        )
        .await
        .unwrap();
    let r = button.extents().await.unwrap();
    assert!(r.height >= 44, "{r:?}");
    s.click(&button).await.unwrap();
    let uri = s.wait_for_log_value("TICK_OPEN_URI", WAIT).await.unwrap();
    assert_eq!(uri, "https://example.com/scan.pdf");
    // Someone else's comment has no actions menu.
    assert!(
        window
            .find(&Query::role(Role::Button).name("Comment Actions").showing())
            .await
            .is_none()
    );
}

/// Edits made offline queue and replay in order when the server is back.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn offline_edits_queue_and_replay() {
    let (server, s, window) = open_in_inbox("Read the article").await;
    server.set_offline(true);
    // Wait for the app to notice.
    window
        .wait_for(
            &Query::role(Role::Button).name("Sync Status").showing(),
            Duration::from_secs(20),
        )
        .await
        .unwrap();
    s.click(&field(&window, "Priority").await).await.unwrap();
    s.click(&picker_row(&window, "Priority 2").await)
        .await
        .unwrap();
    s.click(&field(&window, "Deadline").await).await.unwrap();
    s.click(&picker_row(&window, "Tomorrow").await)
        .await
        .unwrap();
    // Shown at once, from the queue.
    field(&window, "Priority: Priority 2").await;
    field(&window, "Deadline: Tomorrow").await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(server.state().get("items", "i3").unwrap()["priority"], 1);
    server.set_offline(false);
    wait_item(&server, "i3", "replayed", |v| {
        v["priority"] == 3 && v["deadline"]["date"] == "2026-11-23"
    })
    .await;
}

/// A change the server rejects rolls back and says so.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rejected_edit_rolls_back() {
    let (server, s, window) = open_in_inbox("Read the article").await;
    server.inject(tick_fake_server::Failure::CommandError {
        command_type: "item_update".into(),
        args_contain: Some("Never saved".into()),
        error: None,
        count: 1,
    });
    let title = pane_title(&window, "Read the article").await;
    s.click(&title).await.unwrap();
    s.chord(&[keysym::CONTROL_L, u32::from(b'a')])
        .await
        .unwrap();
    s.type_text("Never saved").await.unwrap();
    s.key(keysym::RETURN).await.unwrap();
    // The error toast names the task; the list and the pane show the server's title again.
    window
        .wait_for(
            &Query::role(Role::Label)
                .name_contains("Couldn't save")
                .showing(),
            Duration::from_secs(20),
        )
        .await
        .unwrap();
    app::wait_tasks(
        &window,
        &["Call the dentist", "Renew passport", "Read the article"],
    )
    .await;
    pane_title(&window, "Read the article").await;
    let row = app::task_row(&window, "Read the article").await;
    assert!(
        row.description().await.unwrap().contains("Not saved"),
        "{:?}",
        row.description().await
    );
    assert_eq!(
        server.state().get("items", "i3").unwrap()["content"],
        "Read the article"
    );
}

/// Narrow (B2): Escape in a picker closes only the picker, Escape in the composer closes the
/// pane, and neither pops the list page.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn narrow_escape_closes_picker_then_pane() {
    let server = app::server_with(app::fixture()).await;
    let s = app::session(&server).narrow().start().await.unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    app::open_view(&s, &window, "Inbox").await;
    open_task(&s, &window, "Read the article").await;
    s.click(&field(&window, "Labels").await).await.unwrap();
    picker_row(&window, "phone").await;
    s.key(keysym::ESCAPE).await.unwrap();
    // The pane stays.
    tokio::time::sleep(Duration::from_millis(500)).await;
    pane_title(&window, "Read the article").await;
    let composer = window
        .wait_for(
            &Query::role(Role::Text).name("Write a comment").showing(),
            WAIT,
        )
        .await
        .unwrap();
    s.click(&composer).await.unwrap();
    s.key(keysym::ESCAPE).await.unwrap();
    window
        .wait_gone(&Query::role(Role::Text).name("Task title").showing(), WAIT)
        .await
        .unwrap();
    // Still on the list page.
    app::task_row(&window, "Read the article").await;
}
