//! The login page (`DESIGN.md` 4.16) without `TICK_TOKEN`: unreachable server, a rejected
//! token, then a good one. Test mode never touches a Secret Service.

use tick_e2e::{Node, Query, Role, Session, State, keysym};

use crate::{WAIT, app};

async fn token_entry(window: &Node) -> Node {
    window
        .wait_for(&Query::role(Role::PasswordText).showing(), WAIT)
        .await
        .unwrap()
}

async fn submit(s: &Session, window: &Node, token: &str) {
    let entry = token_entry(window).await;
    s.click(&entry).await.unwrap();
    tick_e2e::poll(WAIT, "token entry focused", || async {
        entry.has_state(State::Focused).await.ok().filter(|f| *f)
    })
    .await
    .unwrap();
    // Replace whatever is there.
    s.chord(&[keysym::CONTROL_L, u32::from(b'a')])
        .await
        .unwrap();
    s.key(keysym::BACKSPACE).await.unwrap();
    s.type_text(token).await.unwrap();
    s.key(keysym::RETURN).await.unwrap();
}

async fn error_line(window: &Node, text: &str) {
    window
        .wait_for(&Query::role(Role::Label).name(text).showing(), WAIT)
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn login_errors_then_success() {
    let server = app::server_with(app::fixture()).await;
    let s = Session::builder()
        .env("TICK_API_BASE", server.base_url())
        .now(app::NOW)
        .env("TICK_SYNC_INTERVAL_MS", "1000")
        .start()
        .await
        .unwrap();
    let window = s.window().await.unwrap();
    window
        .wait_for(
            &Query::role(Role::Label).name("Welcome to Tick").showing(),
            WAIT,
        )
        .await
        .unwrap();
    // "Log In" stays insensitive while the entry is empty.
    let button = window
        .wait_for(&Query::role(Role::Button).name("Log In").showing(), WAIT)
        .await
        .unwrap();
    assert!(!button.has_state(State::Sensitive).await.unwrap());

    submit(&s, &window, "not-the-token").await;
    error_line(
        &window,
        "That token was not accepted. Check it and try again.",
    )
    .await;

    // The right token, sent with the "Log In" button, while the server is unreachable.
    server.set_offline(true);
    let entry = token_entry(&window).await;
    s.click(&entry).await.unwrap();
    s.chord(&[keysym::CONTROL_L, u32::from(b'a')])
        .await
        .unwrap();
    s.key(keysym::BACKSPACE).await.unwrap();
    s.type_text(&server.token()).await.unwrap();
    s.click(&button).await.unwrap();
    error_line(
        &window,
        "Can't reach Todoist. Check your connection and try again.",
    )
    .await;

    // Once the connection is back the pending login goes through on its own.
    server.set_offline(false);
    app::wait_tasks(
        &window,
        &["Renew passport", "Plan the deck", "Call the dentist"],
    )
    .await;
    window
        .wait_gone(&Query::role(Role::PasswordText).showing(), WAIT)
        .await
        .unwrap();
}
