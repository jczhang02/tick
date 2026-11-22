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

    // A notification is done once one of its actions ran (GLib forgets a freedesktop id
    // after ActionInvoked), so send another one for the default action.
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

/// GLib sends through `org.gtk.Notifications` when that name has an owner (a
/// non-sandboxed app; the portal is only used inside a sandbox).
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
