//! Reminders and background running (`SPEC.md` Data 5): time-based reminders notify once
//! with "Complete" and "Open", Complete works while the window is hidden, Open shows the
//! task, nothing repeats after a restart, a reminder coming due while the app runs fires
//! from the timer, a closed window keeps syncing, and `--background` starts without one.

use std::time::Duration;

use serde_json::{Value, json};
use tick_e2e::{Backend, Notification, Query, Role, Session, keysym};
use tick_fake_server::FakeServer;

use crate::detail::pane_title;
use crate::{WAIT, app};

/// [`app::NOW`] is 09:00. Due now: "Call the bank" (absolute 08:55) and "Water the
/// garden" (09:20 minus 30 min = 08:50). Not due: one 2 hours late (outside the one-hour
/// grace), one at 09:30, a location reminder and one for another user.
pub fn fixture() -> Value {
    json!({
        "items": [
            {"id": "r1", "content": "Call the bank", "child_order": 1,
             "due": {"date": "2026-11-22T09:30:00", "string": "today 9:30", "is_recurring": false}},
            {"id": "r2", "content": "Water the garden", "child_order": 2,
             "due": {"date": "2026-11-22T09:20:00", "string": "today 9:20", "is_recurring": false}},
            {"id": "r3", "content": "Long missed", "child_order": 3,
             "due": {"date": "2026-11-22T07:00:00", "string": "today 7:00", "is_recurring": false}},
            {"id": "r4", "content": "Later today", "child_order": 4,
             "due": {"date": "2026-11-22T11:00:00", "string": "today 11:00", "is_recurring": false}}
        ],
        "reminders": [
            {"id": "m1", "item_id": "r1", "type": "absolute",
             "due": {"date": "2026-11-22T08:55:00", "string": "8:55", "is_recurring": false}},
            {"id": "m2", "item_id": "r2", "type": "relative", "minute_offset": 30},
            {"id": "m3", "item_id": "r3", "type": "absolute",
             "due": {"date": "2026-11-22T07:00:00", "string": "7:00", "is_recurring": false}},
            {"id": "m4", "item_id": "r4", "type": "absolute",
             "due": {"date": "2026-11-22T09:30:00", "string": "9:30", "is_recurring": false}},
            {"id": "m5", "item_id": "r4", "type": "location", "name": "Office",
             "loc_lat": "41.1", "loc_long": "-8.6", "loc_trigger": "on_enter", "radius": 100},
            {"id": "m6", "item_id": "r4", "type": "absolute", "notify_uid": "someone-else",
             "due": {"date": "2026-11-22T08:58:00", "string": "8:58", "is_recurring": false}}
        ]
    })
}

async fn start(f: Value) -> (FakeServer, Session) {
    let server = app::server_with(f).await;
    let s = app::session(&server).start().await.unwrap();
    s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    (server, s)
}

async fn notification(s: &Session, title: &str) -> Notification {
    s.notifications()
        .wait_for(WAIT, |n| n.title == title)
        .await
        .unwrap_or_else(|_| panic!("no notification {title:?}: {:?}", s.notifications().sent()))
}

fn titles(s: &Session) -> Vec<String> {
    s.notifications()
        .sent()
        .into_iter()
        .map(|n| n.title)
        .collect()
}

/// Closes the window with its close button; the app keeps running.
async fn close_window(s: &mut Session) {
    let window = s.window().await.unwrap();
    let close = window
        .wait_for(&Query::role(Role::Button).name("Close").showing(), WAIT)
        .await
        .unwrap();
    s.click(&close).await.unwrap();
    s.app()
        .await
        .unwrap()
        .wait_gone(&Query::role(Role::Frame), WAIT)
        .await
        .unwrap();
    assert!(s.app_running());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn due_reminders_notify_once_with_both_buttons() {
    let (_server, mut s) = start(fixture()).await;
    let n = notification(&s, "Call the bank").await;
    assert_eq!(n.backend, Backend::Gtk);
    assert_eq!(n.app, "dev.chengrui.Tick");
    let labels: Vec<&str> = n.buttons.iter().map(|b| b.label.as_str()).collect();
    assert_eq!(labels, ["Complete", "Open"]);
    assert_eq!(n.buttons[0].action, "app.reminder-complete");
    assert_eq!(n.buttons[1].action, "app.reminder-open");
    assert!(n.body.contains("Today 09:30"), "{:?}", n.body);
    assert!(n.default_action.is_some());
    notification(&s, "Water the garden").await;
    // Syncs come and go (1 s interval): still one each, and none of the others.
    tokio::time::sleep(Duration::from_secs(3)).await;
    let mut got = titles(&s);
    got.sort();
    assert_eq!(got, ["Call the bank", "Water the garden"]);

    // A restart shows nothing again.
    s.chord(&[keysym::CONTROL_L, u32::from(b'q')])
        .await
        .unwrap();
    s.wait_app_exit(WAIT).await.unwrap();
    s.launch_app().unwrap();
    s.window().await.unwrap();
    s.first_frame_ms(WAIT).await.unwrap();
    tokio::time::sleep(Duration::from_secs(4)).await;
    assert_eq!(titles(&s).len(), 2, "{:?}", titles(&s));
    assert!(!s.app_log().contains("TICK_REMINDER_SENT"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn complete_works_while_the_window_is_hidden() {
    let (server, mut s) = start(fixture()).await;
    let n = notification(&s, "Call the bank").await;
    close_window(&mut s).await;
    s.notifications().invoke(&n, "Complete").await.unwrap();
    tick_e2e::poll(WAIT, "item_close r1", || async {
        app::commands_sent(&server, "item_close")
            .into_iter()
            .find(|c| c["args"]["id"] == "r1")
    })
    .await
    .unwrap();
    crate::detail::wait_item(&server, "r1", "completed", |v| v["checked"] == true).await;
    // Still hidden, still running.
    assert!(
        s.app()
            .await
            .unwrap()
            .find(&Query::role(Role::Frame))
            .await
            .is_none()
    );
    assert!(s.app_running());
    // The notification is taken back.
    tick_e2e::poll(WAIT, "withdrawn", || async {
        s.notifications()
            .sent()
            .into_iter()
            .any(|x| x.title == "Call the bank" && x.removed)
            .then_some(())
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn open_shows_the_window_with_the_task() {
    let (_server, mut s) = start(fixture()).await;
    let n = notification(&s, "Water the garden").await;
    close_window(&mut s).await;
    s.notifications().invoke(&n, "Open").await.unwrap();
    let window = s
        .app()
        .await
        .unwrap()
        .wait_for(&Query::role(Role::Frame).name("Tick"), WAIT)
        .await
        .unwrap();
    pane_title(&window, "Water the garden").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn free_plan_without_reminders_is_quiet() {
    let f = json!({
        "user": {"is_premium": false, "premium_status": "not_premium"},
        "items": [
            {"id": "f1", "content": "Due right now", "child_order": 1,
             "due": {"date": "2026-11-22T09:00:00", "string": "today 9:00", "is_recurring": false}}
        ]
    });
    let (_server, s) = start(f).await;
    let window = s.window().await.unwrap();
    app::wait_tasks(&window, &["Due right now"]).await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert!(s.notifications().sent().is_empty());
    assert!(
        !s.app_log().contains("reading reminders"),
        "{}",
        s.app_log()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn desktop_reminders_off_in_todoist_settings() {
    let mut f = fixture();
    f["user_settings"] = json!({"reminder_desktop": false});
    let (_server, s) = start(f).await;
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert!(s.notifications().sent().is_empty(), "{:?}", titles(&s));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn closed_window_keeps_syncing_and_ctrl_q_quits() {
    let server = app::server_with(app::fixture()).await;
    let mut s = app::session(&server)
        .env("TICK_BACKGROUND_SYNC_MS", "1500")
        .start()
        .await
        .unwrap();
    s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    close_window(&mut s).await;
    let before = app::requests(&server, "/api/v1/sync").len();
    tokio::time::sleep(Duration::from_secs(5)).await;
    let after = app::requests(&server, "/api/v1/sync").len();
    assert!(after >= before + 2, "background syncs: {before} -> {after}");
    assert!(s.app_running());

    // Shown again (a second launch activates the running app), Ctrl+Q quits.
    activate(&s).await;
    s.app()
        .await
        .unwrap()
        .wait_for(&Query::role(Role::Frame).name("Tick"), WAIT)
        .await
        .unwrap();
    s.chord(&[keysym::CONTROL_L, u32::from(b'q')])
        .await
        .unwrap();
    let status = s.wait_app_exit(WAIT).await.unwrap();
    assert!(status.success(), "{status}");
}

/// Asks the running app to activate (what a second `tick` launch does).
async fn activate(s: &Session) {
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
}

/// Whether the app shows a window: a never-shown window may keep the app off the AT-SPI
/// registry altogether, which counts as none.
async fn shows_a_window(s: &Session) -> bool {
    match tokio::time::timeout(Duration::from_secs(3), s.app()).await {
        Ok(Ok(app)) => app.find(&Query::role(Role::Frame)).await.is_some(),
        _ => false,
    }
}

/// `tick --background` (Run at Login): no window, syncing at once, reminders as usual; a
/// second launch shows the window.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn background_start_syncs_and_reminds_without_a_window() {
    let server = app::server_with(fixture()).await;
    let s = app::session(&server)
        .arg("--background")
        .start()
        .await
        .unwrap();
    app::wait_first_sync(&server).await;
    notification(&s, "Call the bank").await;
    notification(&s, "Water the garden").await;
    assert!(!shows_a_window(&s).await, "--background shows no window");
    // No first frame to report either: nothing was painted.
    assert!(!s.app_log().contains("TICK_FIRST_DATA_FRAME_MS"));
    activate(&s).await;
    s.app()
        .await
        .unwrap()
        .wait_for(&Query::role(Role::Frame).name("Tick"), WAIT)
        .await
        .unwrap();
}

/// A reminder that comes due while the app runs fires from the timer, on time and once
/// (`TICK_NOW_ADVANCE` lets the fixed clock run; no sync comes in between).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_reminder_fires_on_time_while_running() {
    let f = json!({
        "items": [
            {"id": "t1", "content": "Take the cake out", "child_order": 1,
             "due": {"date": "2026-11-22T10:00:00", "string": "today 10:00", "is_recurring": false}}
        ],
        "reminders": [
            {"id": "n1", "item_id": "t1", "type": "absolute",
             "due": {"date": "2026-11-22T09:00:15", "string": "9:00:15", "is_recurring": false}}
        ]
    });
    let server = app::server_with(f).await;
    let s = app::session(&server)
        .env("TICK_NOW_ADVANCE", "1")
        .env("TICK_SYNC_INTERVAL_MS", "600000")
        .start()
        .await
        .unwrap();
    // The app is launched by now (`start` may first wait for a free slot); its clock starts
    // later, at its first reading.
    let launched = std::time::Instant::now();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    app::wait_tasks(&window, &["Take the cake out"]).await;
    let synced = launched.elapsed();
    assert!(
        synced < Duration::from_secs(13),
        "the first sync took {synced:?}: too late to tell the timer from the start check"
    );
    assert!(s.notifications().sent().is_empty(), "{:?}", titles(&s));
    let n = s
        .notifications()
        .wait_for(Duration::from_secs(25), |n| n.title == "Take the cake out")
        .await
        .unwrap();
    let at = launched.elapsed();
    assert!(at >= Duration::from_secs(14), "fired early, after {at:?}");
    assert!(at < Duration::from_secs(22), "fired late, after {at:?}");
    assert_eq!(n.buttons.len(), 2);
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert_eq!(titles(&s), ["Take the cake out"]);
    assert_eq!(app::requests(&server, "/api/v1/sync").len(), 1);
}
