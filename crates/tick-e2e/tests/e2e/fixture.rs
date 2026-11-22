//! Harness self-tests against a minimal GTK fixture app
//! (`crates/tick-gtk/examples/e2e_fixture.rs`), for input and notification paths Tick
//! cannot exercise yet.

use tick_e2e::{Backend, Node, Query, Role, Session, SessionBuilder};

use crate::WAIT;

fn fixture() -> SessionBuilder {
    let bin = tick_e2e::example_bin("e2e_fixture").unwrap();
    Session::builder().app_binary(bin, "dev.chengrui.Tick.E2eFixture")
}

async fn entry(s: &Session) -> Node {
    let window = s.window().await.unwrap();
    window
        .wait_for(&Query::role(Role::Text).name("Probe entry"), WAIT)
        .await
        .unwrap()
}

async fn wait_text(node: &Node, want: &str) {
    let got = tick_e2e::poll(WAIT, want, || async {
        node.text().await.ok().filter(|t| t == want)
    })
    .await;
    assert!(got.is_ok(), "text: {:?}, want {want:?}", node.text().await);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn typing_ascii_and_cjk() {
    let s = fixture().start().await.unwrap();
    let entry = entry(&s).await;
    s.click(&entry).await.unwrap();
    s.type_text("Buy milk #Home p1 !@").await.unwrap();
    wait_text(&entry, "Buy milk #Home p1 !@").await;
    // Non-ASCII goes through GTK's Ctrl+Shift+U hex input (mutter drops these keysyms).
    s.type_text(" 买牛奶 café").await.unwrap();
    wait_text(&entry, "Buy milk #Home p1 !@ 买牛奶 café").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn editable_text_fallback() {
    let s = fixture().start().await.unwrap();
    let entry = entry(&s).await;
    entry.set_text("明天 report").await.unwrap();
    wait_text(&entry, "明天 report").await;
    entry.insert_text(2, "写").await.unwrap();
    wait_text(&entry, "明天写 report").await;
}

async fn notification_round_trip(s: &Session, backend: Backend) {
    let window = s.window().await.unwrap();
    let send = window
        .wait_for(&Query::role(Role::Button).name("Send notification"), WAIT)
        .await
        .unwrap();
    s.click(&send).await.unwrap();
    let n = s.notifications().wait_for(WAIT, |_| true).await.unwrap();
    assert_eq!(n.backend, backend, "{n:?}");
    assert_eq!(
        (n.title.as_str(), n.body.as_str()),
        ("Reminder", "Buy milk")
    );

    s.notifications().invoke(&n, "Complete").await.unwrap();
    window
        .wait_for(&Query::role(Role::Label).name("completed 42"), WAIT)
        .await
        .unwrap();

    // A second ActionInvoked for the same freedesktop id has no effect, so send another
    // notification for the default action.
    s.click(&send).await.unwrap();
    let n = tick_e2e::poll(WAIT, "second notification", || async {
        s.notifications().sent().get(1).cloned()
    })
    .await
    .unwrap();
    s.notifications().activate_default(&n).await.unwrap();
    window
        .wait_for(&Query::role(Role::Label).name("opened"), WAIT)
        .await
        .unwrap();
}

/// GLib sends through `org.gtk.Notifications` when that name has an owner.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn notifications_use_gtk_backend() {
    let s = fixture().start().await.unwrap();
    notification_round_trip(&s, Backend::Gtk).await;
}

/// Without `org.gtk.Notifications`, GLib falls back to `org.freedesktop.Notifications`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn notifications_freedesktop_fallback() {
    let s = fixture()
        .freedesktop_notifications_only()
        .start()
        .await
        .unwrap();
    notification_round_trip(&s, Backend::Freedesktop).await;
}

/// AT-SPI actions, focus state and the remaining input calls against a real widget tree.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn actions_focus_scroll_and_drag() {
    let s = fixture().start().await.unwrap();
    let window = s.window().await.unwrap();
    let entry = entry(&s).await;
    // GTK4 answers Component.GrabFocus with NotSupported: focus with a real click.
    s.click(&entry).await.unwrap();
    tick_e2e::poll(WAIT, "entry focused", || async {
        entry
            .has_state(tick_e2e::State::Focused)
            .await
            .ok()
            .filter(|f| *f)
    })
    .await
    .unwrap();

    let send = window
        .wait_for(&Query::role(Role::Button).name("Send notification"), WAIT)
        .await
        .unwrap();
    send.do_action(Some("click")).await.unwrap();
    s.notifications().wait_for(WAIT, |_| true).await.unwrap();

    // No scrollable or draggable widget here: check that mutter accepts the calls and
    // the app survives them.
    let input = s.input().await.unwrap();
    input.move_to(640, 400).await.unwrap();
    input.scroll(1).await.unwrap();
    input.drag((640, 400), (700, 450)).await.unwrap();
    window
        .wait_for(&Query::role(Role::Label).name("idle"), WAIT)
        .await
        .unwrap();
}

/// Real pointer drags reach GTK's drag-and-drop machinery under headless mutter.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn drag_and_drop_between_widgets() {
    let s = fixture().start().await.unwrap();
    let window = s.window().await.unwrap();
    let source = window
        .wait_for(&Query::role(Role::Label).name("Drag source"), WAIT)
        .await
        .unwrap();
    let target = window
        .wait_for(&Query::role(Role::Label).name("Drop target"), WAIT)
        .await
        .unwrap();
    let from = source.extents().await.unwrap().center();
    let to = target.extents().await.unwrap().center();
    let input = s.input().await.unwrap();
    input.press_at(from.0, from.1).await.unwrap();
    input
        .drag_by(to.0 - from.0, to.1 - from.1, 20)
        .await
        .unwrap();
    window
        .wait_for(&Query::role(Role::Label).name("dragging"), WAIT)
        .await
        .unwrap();
    input.release().await.unwrap();
    window
        .wait_for(&Query::role(Role::Label).name("dropped probe-42"), WAIT)
        .await
        .unwrap();
    // The one-call form.
    input.drag(from, to).await.unwrap();
    window
        .wait_for(&Query::role(Role::Label).name("dropped probe-42"), WAIT)
        .await
        .unwrap();
}
