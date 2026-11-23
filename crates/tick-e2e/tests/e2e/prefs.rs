//! Preferences (`DESIGN.md` 4.17), the main menu (3.4), the keyboard shortcuts dialog
//! (4.18) and logging out.

use std::path::Path;
use std::time::Duration;

use tick_e2e::{Node, PortalMode, Query, Role, Session, keysym};
use tick_fake_server::{Failure, FakeServer};

use crate::{WAIT, app};

/// The RGB of pixel (x, y) of a PNG screenshot.
pub fn pixel(path: &Path, x: u32, y: u32) -> [u8; 3] {
    let decoder = png::Decoder::new(std::fs::File::open(path).unwrap());
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    let channels = info.color_type.samples();
    let i = (y as usize * info.line_size) + x as usize * channels;
    [buf[i], buf[i + 1], buf[i + 2]]
}

/// Whether the task list's background (empty space at the bottom right) is dark.
async fn list_is_dark(s: &Session) -> bool {
    let path = s.root().join(format!(
        "theme-{}.png",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    s.screenshot(&path).await.unwrap();
    let [r, g, b] = pixel(&path, 1100, 760);
    let luma = (u32::from(r) + u32::from(g) + u32::from(b)) / 3;
    assert!(
        !(70..=200).contains(&luma),
        "unexpected list background {r},{g},{b}"
    );
    luma < 70
}

async fn wait_dark(s: &Session, dark: bool) {
    let ok = tick_e2e::poll(WAIT, "theme", || async {
        (list_is_dark(s).await == dark).then_some(())
    })
    .await;
    assert!(
        ok.is_ok(),
        "the window should be {}",
        if dark { "dark" } else { "light" }
    );
}

async fn start(
    builder: impl FnOnce(tick_e2e::SessionBuilder) -> tick_e2e::SessionBuilder,
) -> (FakeServer, Session, Node) {
    let server = app::server_with(app::fixture()).await;
    let s = builder(app::session(&server)).start().await.unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    app::wait_tasks(
        &window,
        &["Renew passport", "Plan the deck", "Call the dentist"],
    )
    .await;
    (server, s, window)
}

/// Opens Preferences with Ctrl+, and returns the dialog's root.
async fn open_prefs(s: &Session, window: &Node) -> Node {
    s.chord(&[keysym::CONTROL_L, u32::from(b',')])
        .await
        .unwrap();
    window
        .wait_for(&Query::role(Role::Label).name("Appearance").showing(), WAIT)
        .await
        .unwrap();
    window.clone()
}

async fn radio(window: &Node, name: &str) -> Node {
    window
        .wait_for(&Query::role(Role::RadioButton).name(name).showing(), WAIT)
        .await
        .unwrap()
}

fn saved_setting(s: &Session, key: &str) -> Option<String> {
    let store = tick_core::Store::open(s.data_home().join("tick/cache.sqlite3")).unwrap();
    store.setting(key).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn theme_switches_live_and_persists() {
    let (_server, mut s, window) = start(|b| b).await;
    wait_dark(&s, false).await;
    open_prefs(&s, &window).await;
    let follow = radio(&window, "Follow System").await;
    assert!(follow.has_state(tick_e2e::State::Checked).await.unwrap());
    s.click(&radio(&window, "Dark").await).await.unwrap();
    // Live: the dialog closes over a dark window.
    s.key(keysym::ESCAPE).await.unwrap();
    window
        .wait_gone(&Query::role(Role::Label).name("Appearance").showing(), WAIT)
        .await
        .unwrap();
    wait_dark(&s, true).await;
    assert_eq!(saved_setting(&s, "color-scheme").as_deref(), Some("dark"));

    // Restart: dark from the first frame.
    s.chord(&[keysym::CONTROL_L, u32::from(b'q')])
        .await
        .unwrap();
    s.wait_app_exit(WAIT).await.unwrap();
    s.launch_app().unwrap();
    let window = s.window().await.unwrap();
    s.first_frame_ms(WAIT).await.unwrap();
    assert!(
        list_is_dark(&s).await,
        "the saved theme applies before the first paint"
    );

    // Back to Follow System (light in the test session), live.
    open_prefs(&s, &window).await;
    assert!(
        radio(&window, "Dark")
            .await
            .has_state(tick_e2e::State::Checked)
            .await
            .unwrap()
    );
    s.click(&radio(&window, "Follow System").await)
        .await
        .unwrap();
    s.key(keysym::ESCAPE).await.unwrap();
    wait_dark(&s, false).await;
    assert_eq!(saved_setting(&s, "color-scheme").as_deref(), Some("system"));
}

async fn run_at_login(window: &Node) -> Node {
    match window
        // A switch row: AT-SPI's "switch" role is unknown to the atspi crate.
        .wait_for(
            &Query::default()
                .name("Run at Login")
                .state(tick_e2e::State::Checkable)
                .showing(),
            WAIT,
        )
        .await
    {
        Ok(n) => n,
        Err(e) => panic!("{e}\n{}", window.dump().await),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn run_at_login_asks_the_background_portal() {
    let (_server, mut s, window) = start(|b| b.fake_portal(PortalMode::Grant)).await;
    open_prefs(&s, &window).await;
    let switch = run_at_login(&window).await;
    assert!(!switch.has_state(tick_e2e::State::Checked).await.unwrap());
    s.click(&switch).await.unwrap();
    let calls = s.portal().unwrap().wait_calls(1, WAIT).await.unwrap();
    assert!(calls[0].autostart, "{calls:?}");
    assert_eq!(calls[0].commandline, ["tick", "--background"]);
    assert!(!calls[0].reason.is_empty());
    tick_e2e::poll(WAIT, "setting saved", || async {
        (saved_setting(&s, "run-at-login").as_deref() == Some("true")).then_some(())
    })
    .await
    .unwrap();
    assert!(switch.has_state(tick_e2e::State::Checked).await.unwrap());

    // Remembered across a restart; switching off asks again without autostart.
    s.chord(&[keysym::CONTROL_L, u32::from(b'q')])
        .await
        .unwrap();
    s.wait_app_exit(WAIT).await.unwrap();
    s.launch_app().unwrap();
    let window = s.window().await.unwrap();
    open_prefs(&s, &window).await;
    let switch = run_at_login(&window).await;
    assert!(switch.has_state(tick_e2e::State::Checked).await.unwrap());
    s.click(&switch).await.unwrap();
    let calls = s.portal().unwrap().wait_calls(2, WAIT).await.unwrap();
    assert!(!calls[1].autostart, "{calls:?}");
    tick_e2e::poll(WAIT, "setting saved", || async {
        (saved_setting(&s, "run-at-login").as_deref() == Some("false")).then_some(())
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn run_at_login_without_a_portal_says_so() {
    let (_server, s, window) = start(|b| b.fake_portal(PortalMode::NoBackground)).await;
    open_prefs(&s, &window).await;
    let switch = run_at_login(&window).await;
    s.click(&switch).await.unwrap();
    window
        .wait_for(
            &Query::role(Role::Label)
                .name("Run at Login is not available on this system.")
                .showing(),
            WAIT,
        )
        .await
        .unwrap();
    // The switch goes back off and nothing is saved.
    tick_e2e::poll(WAIT, "switch back off", || async {
        (!switch.has_state(tick_e2e::State::Checked).await.ok()?).then_some(())
    })
    .await
    .unwrap();
    assert_eq!(saved_setting(&s, "run-at-login"), None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn run_at_login_denied_reverts() {
    let (_server, s, window) = start(|b| b.fake_portal(PortalMode::Deny)).await;
    open_prefs(&s, &window).await;
    let switch = run_at_login(&window).await;
    s.click(&switch).await.unwrap();
    s.portal().unwrap().wait_calls(1, WAIT).await.unwrap();
    window
        .wait_for(
            &Query::role(Role::Label)
                .name("Run at Login was not allowed. Check Background Apps in Settings.")
                .showing(),
            WAIT,
        )
        .await
        .unwrap();
    assert!(!switch.has_state(tick_e2e::State::Checked).await.unwrap());
}

/// The main menu's items, in order (3.4).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn main_menu_items() {
    let (_server, s, window) = start(|b| b).await;
    let button = window
        .wait_for(&Query::role(Role::Button).name("Main Menu").showing(), WAIT)
        .await
        .unwrap();
    s.click(&button).await.unwrap();
    let app_node = s.app().await.unwrap();
    let mut names = Vec::new();
    for name in [
        "Preferences",
        "Keyboard Shortcuts",
        "About Tick",
        "Log Out",
        "Quit",
    ] {
        let item = app_node
            .wait_for(&Query::role(Role::MenuItem).name(name).showing(), WAIT)
            .await
            .unwrap();
        names.push(item.extents().await.unwrap().y);
    }
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "menu order");
    // Keyboard Shortcuts opens the dialog.
    let item = app_node
        .wait_for(
            &Query::role(Role::MenuItem)
                .name("Keyboard Shortcuts")
                .showing(),
            WAIT,
        )
        .await
        .unwrap();
    s.click(&item).await.unwrap();
    window
        .wait_for(&Query::role(Role::Label).name("Navigation").showing(), WAIT)
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shortcuts_dialog_lists_the_shortcuts() {
    let (_server, s, window) = start(|b| b).await;
    s.chord(&[keysym::CONTROL_L, u32::from(b'?')])
        .await
        .unwrap();
    for name in ["General", "Navigation", "Tasks"] {
        window
            .wait_for(&Query::role(Role::Label).name(name).showing(), WAIT)
            .await
            .unwrap();
    }
    for name in [
        "Quick add",
        "Search",
        "Preferences",
        "Keyboard shortcuts",
        "Quit",
        "Undo",
        "Inbox",
        "Today",
        "Upcoming",
        "Open",
        "Complete",
        "Set date",
        "Set priority",
        "Move to project",
        "Delete",
    ] {
        window
            .wait_for(&Query::role(Role::Label).name(name), WAIT)
            .await
            .unwrap_or_else(|_| panic!("no shortcut {name:?}"));
    }
    crate::a11y::check(&window, "shortcuts dialog").await;
    s.key(keysym::ESCAPE).await.unwrap();
    window
        .wait_gone(&Query::role(Role::Label).name("Navigation").showing(), WAIT)
        .await
        .unwrap();
}

/// Preferences, "Log Out": waits for the confirmation (4.25) and returns its "Log Out"
/// response.
async fn ask_to_log_out(s: &Session, window: &Node) -> Node {
    open_prefs(s, window).await;
    let button = window
        .wait_for(&Query::role(Role::Button).name("Log Out").showing(), WAIT)
        .await
        .unwrap();
    s.click(&button).await.unwrap();
    window
        .wait_for(
            &Query::role(Role::Label).name("Log out of Tick?").showing(),
            WAIT,
        )
        .await
        .unwrap();
    tick_e2e::poll(WAIT, "Log Out response", || async {
        let all = window
            .find_all(&Query::role(Role::Button).name("Log Out").showing())
            .await;
        // The alert's response is the last one (the preferences row is under it).
        all.into_iter().last()
    })
    .await
    .unwrap()
}

/// Nothing of the account is left on this computer.
fn assert_cache_empty(s: &Session) {
    let store = tick_core::Store::open(s.data_home().join("tick/cache.sqlite3")).unwrap();
    assert!(store.user().unwrap().is_none());
    assert!(store.task("i1").unwrap().is_none());
    assert_eq!(store.sync_token().unwrap(), None);
    assert!(store.pending_commands().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn log_out_returns_to_login_and_clears_the_cache() {
    let (server, s, window) = start(|b| b).await;
    let confirm = ask_to_log_out(&s, &window).await;
    s.click(&confirm).await.unwrap();
    window
        .wait_for(
            &Query::role(Role::Label).name("Welcome to Tick").showing(),
            WAIT,
        )
        .await
        .unwrap();
    s.wait_for_log_value("TICK_LOGGED_OUT", WAIT).await.unwrap();
    assert_cache_empty(&s);
    // And no syncing goes on.
    let n = app::requests(&server, "/api/v1/sync").len();
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert_eq!(app::requests(&server, "/api/v1/sync").len(), n);
}

/// Logging out while a sync hangs on the network (longer than any timer the UI could
/// use): the engine stops at once, and the late response never reaches the cleared cache.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn log_out_during_a_slow_sync_keeps_the_cache_empty() {
    let (server, s, window) = start(|b| b).await;
    let confirm = ask_to_log_out(&s, &window).await;
    server.inject(Failure::Delay {
        ms: 15_000,
        count: 100,
    });
    let n = app::requests(&server, "/api/v1/sync").len();
    tick_e2e::poll(WAIT, "a slow sync", || async {
        (app::requests(&server, "/api/v1/sync").len() > n).then_some(())
    })
    .await
    .unwrap();
    let asked = std::time::Instant::now();
    s.click(&confirm).await.unwrap();
    window
        .wait_for(
            &Query::role(Role::Label).name("Welcome to Tick").showing(),
            WAIT,
        )
        .await
        .unwrap();
    s.wait_for_log_value("TICK_LOGGED_OUT", WAIT).await.unwrap();
    assert!(
        asked.elapsed() < Duration::from_secs(8),
        "log out waited for the slow sync: {:?}",
        asked.elapsed()
    );
    assert_cache_empty(&s);
    // The held response is answered now; nothing writes it into the cache.
    tokio::time::sleep(Duration::from_secs(16)).await;
    assert_cache_empty(&s);
    let store = tick_core::Store::open(s.data_home().join("tick/cache.sqlite3")).unwrap();
    assert!(store.projects_tree().unwrap().is_empty());
    assert!(
        !s.app_log().contains("tick: clearing the cache"),
        "{}",
        s.app_log()
    );
}

/// Ctrl+W closes (hides) the window; the app keeps running.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ctrl_w_closes_the_window() {
    let (_server, mut s, _window) = start(|b| b).await;
    s.chord(&[keysym::CONTROL_L, u32::from(b'w')])
        .await
        .unwrap();
    s.app()
        .await
        .unwrap()
        .wait_gone(&Query::role(Role::Frame), WAIT)
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(s.app_running());
}

/// Left collapses a parent's sub-tasks (or moves to the parent), Right expands them (4.4).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn left_and_right_collapse_and_expand() {
    let (server, s, window) = start(|b| b).await;
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
    // Focus the sub-task, Left: its parent.
    let child = app::task_row(&window, "Collect metrics").await;
    let r = child.extents().await.unwrap();
    s.input()
        .await
        .unwrap()
        .click(r.x + 150, r.y + 22)
        .await
        .unwrap();
    s.key(keysym::ESCAPE).await.unwrap();
    s.key(keysym::LEFT).await.unwrap();
    let parent = app::task_row(&window, "Plan the deck").await;
    tick_e2e::poll(WAIT, "parent focused", || async {
        parent
            .has_state(tick_e2e::State::Focused)
            .await
            .ok()
            .filter(|f| *f)
    })
    .await
    .unwrap();
    // Left again: collapsed; Right: expanded.
    s.key(keysym::LEFT).await.unwrap();
    app::wait_tasks(
        &window,
        &["Book the venue", "Plan the deck", "Clean up flags"],
    )
    .await;
    crate::detail::wait_item(&server, "w1", "collapsed", |v| v["is_collapsed"] == true).await;
    s.key(keysym::RIGHT).await.unwrap();
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
}

/// Narrow: Alt+Left goes back from a view to the sidebar.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn alt_left_goes_back() {
    let server = app::server_with(app::fixture()).await;
    let s = app::session(&server).narrow().start().await.unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    app::sidebar(&window).await;
    app::open_view(&s, &window, "Inbox").await;
    app::wait_tasks(
        &window,
        &["Call the dentist", "Renew passport", "Read the article"],
    )
    .await;
    s.chord(&[keysym::ALT_L, keysym::LEFT]).await.unwrap();
    window
        .wait_gone(&Query::role(Role::List).name("Tasks").showing(), WAIT)
        .await
        .unwrap();
    app::sidebar(&window).await;
}
