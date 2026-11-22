//! Harness self-tests against the real `tick` binary.

use std::time::Duration;

use tick_e2e::{Query, Role, Session, keysym};

use crate::WAIT;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn window_appears_on_atspi() {
    let s = Session::start_default().await.unwrap();
    let window = s.window().await.unwrap();
    assert_eq!(window.name().await.unwrap(), "Tick");
    // Test mode maximizes the window: window coordinates are screen coordinates.
    let rect = window.extents().await.unwrap();
    assert_eq!((rect.width, rect.height), (1280, 800), "{rect:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn first_frame_is_reported() {
    let s = Session::start_default().await.unwrap();
    let ms = s.first_frame_ms(WAIT).await.unwrap();
    assert!(ms > 0 && ms < 60_000, "TICK_FIRST_DATA_FRAME_MS={ms}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ctrl_q_quits() {
    let mut s = Session::start_default().await.unwrap();
    s.window().await.unwrap();
    s.chord(&[keysym::CONTROL_L, u32::from(b'q')])
        .await
        .unwrap();
    let status = s.wait_app_exit(WAIT).await.unwrap();
    assert!(status.success(), "{status}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn close_button_hides_window_and_keeps_running() {
    let mut s = Session::start_default().await.unwrap();
    let window = s.window().await.unwrap();
    let close = window
        .wait_for(&Query::role(Role::Button).name("Close"), WAIT)
        .await
        .unwrap();
    s.click(&close).await.unwrap();

    // A hidden GTK4 window drops out of the app's AT-SPI children.
    let app = s.app().await.unwrap();
    app.wait_gone(&Query::role(Role::Frame), WAIT)
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(s.app_running(), "closing the window must not quit the app");

    // Activating the running instance (what a second `tick` launch does) shows it again.
    let platform: std::collections::HashMap<&str, zbus::zvariant::Value<'_>> = Default::default();
    s.bus()
        .call_method(
            Some("dev.chengrui.Tick"),
            "/dev/chengrui/Tick",
            Some("org.freedesktop.Application"),
            "Activate",
            &(platform,),
        )
        .await
        .unwrap();
    let window = app
        .wait_for(&Query::role(Role::Frame).name("Tick"), WAIT)
        .await
        .unwrap();
    assert!(window.has_state(tick_e2e::State::Showing).await.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn narrow_session() {
    let s = Session::builder().narrow().start().await.unwrap();
    assert_eq!(s.monitor_size(), (600, 800));
    let window = s.window().await.unwrap();
    let rect = window.extents().await.unwrap();
    assert_eq!((rect.width, rect.height), (600, 800), "{rect:?}");
}
