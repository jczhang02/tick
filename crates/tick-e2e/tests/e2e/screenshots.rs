//! Design-review screenshots through `app.test-screenshot`.
//!
//! With `TICK_SCREENSHOTS=1` (`just screenshots`) every main view, the quick add dialog,
//! the login page and an offline state are captured in light and dark, wide (1280 x 800)
//! and narrow (600 x 800), into `target/screenshots/<view>-<size>-<scheme>.png`, from the
//! demo seed at the fixed clock. Without it (`just e2e`) a smaller set checks that
//! screenshots work, so the suite stays fast.

use std::path::{Path, PathBuf};
use std::time::Duration;

use tick_e2e::{ColorScheme, Node, Query, Role, Session, keysym};
use tick_fake_server::FakeServer;

use crate::{WAIT, app};

fn full() -> bool {
    std::env::var_os("TICK_SCREENSHOTS").is_some()
}

fn scheme_name(c: ColorScheme) -> &'static str {
    match c {
        ColorScheme::Light => "light",
        ColorScheme::Dark => "dark",
    }
}

/// Takes a screenshot of the app's window and returns the PNG path.
async fn shoot(s: &Session, name: &str) -> PathBuf {
    // Park the pointer on the window edge so no row shows a hover tint.
    let (mw, mh) = s.monitor_size();
    let input = s.input().await.unwrap();
    input
        .move_to(
            i32::try_from(mw).unwrap() - 2,
            i32::try_from(mh).unwrap() / 2,
        )
        .await
        .unwrap();
    // Let the last layout and fades settle (animations are off in test mode).
    tokio::time::sleep(Duration::from_millis(300)).await;
    let path = if full() {
        tick_e2e::screenshot_dir()
            .unwrap()
            .join(format!("{name}.png"))
    } else {
        s.root().join(format!("{name}.png"))
    };
    let path = s.screenshot(&path).await.unwrap();
    let (w, h) = png_size(&path);
    assert_eq!((w, h), (mw, mh), "{}", path.display());
    path
}

fn png_size(path: &Path) -> (u32, u32) {
    let png = std::fs::read(path).unwrap();
    assert!(
        png.len() > 24 && png.starts_with(b"\x89PNG\r\n\x1a\n"),
        "{} is not a PNG",
        path.display()
    );
    let be = |i: usize| u32::from_be_bytes(png[i..i + 4].try_into().unwrap());
    (be(16), be(20))
}

/// The views captured in each session: (file name, sidebar row, row that shows it loaded).
const VIEWS: &[(&str, &str, &str)] = &[
    ("inbox", "Inbox", "Call the dentist to move the appointment"),
    ("upcoming", "Upcoming", "Upcoming"),
    ("project", "Work", "This week, 7 tasks, expanded"),
    ("project-cjk", "学习计划", "复习 Rust 所有权章节"),
    ("label", "deep-work", "deep-work"),
    ("filter", "Urgent this week", "Urgent this week"),
    ("filter-server", "Home-ish labels", ""),
];

async fn wait_row(window: &Node, name: &str) {
    let list = app::task_list(window).await;
    if name.is_empty() {
        // Server filter: the source line names where the rows come from.
        tick_e2e::poll(WAIT, "server results", || async {
            for n in list.find_all(&Query::role(Role::ListItem)).await {
                if n.name()
                    .await
                    .is_ok_and(|t| t.starts_with("Results from Todoist"))
                {
                    return Some(());
                }
            }
            None
        })
        .await
        .unwrap();
        return;
    }
    list.wait_for(&Query::role(Role::ListItem).name(name), WAIT)
        .await
        .unwrap();
}

async fn back_to_sidebar(s: &Session, window: &Node) {
    let back = window
        .wait_for(&Query::role(Role::Button).name("Back").showing(), WAIT)
        .await
        .unwrap();
    s.click(&back).await.unwrap();
    app::sidebar(window).await;
}

async fn session(server: &FakeServer, scheme: ColorScheme, narrow: bool) -> Session {
    let mut b = app::session(server).color_scheme(scheme);
    if narrow {
        b = b.narrow();
    }
    b.start().await.unwrap()
}

/// All views of one size and scheme.
async fn matrix(scheme: ColorScheme, narrow: bool) {
    let server = app::server("demo").await;
    let s = session(&server, scheme, narrow).await;
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    let size = if narrow { "narrow" } else { "wide" };
    let sc = scheme_name(scheme);
    let name = |view: &str| format!("{view}-{size}-{sc}");

    if narrow {
        app::sidebar(&window).await;
        shoot(&s, &name("sidebar")).await;
        app::open_view(&s, &window, "Today").await;
    }
    wait_row(&window, "Overdue").await;
    shoot(&s, &name("today")).await;
    if !full() {
        return;
    }

    for (file, view, ready) in VIEWS {
        if narrow {
            back_to_sidebar(&s, &window).await;
        }
        app::open_view(&s, &window, view).await;
        wait_row(&window, ready).await;
        shoot(&s, &name(file)).await;
    }

    // Quick add with highlighted tokens (a bottom sheet on narrow windows).
    s.key(u32::from(b'q')).await.unwrap();
    window
        .wait_for(&Query::role(Role::Text).name("Task").showing(), WAIT)
        .await
        .unwrap();
    s.type_text("Call Mom tomorrow 6pm p2 #Personal @phone")
        .await
        .unwrap();
    shoot(&s, &name("quick-add")).await;
    s.key(keysym::ESCAPE).await.unwrap();
    window
        .wait_gone(&Query::role(Role::Text).name("Task").showing(), WAIT)
        .await
        .unwrap();

    // Offline with one queued change: the sync indicator shows the count.
    server.set_offline(true);
    let sync = window
        .wait_for(
            &Query::role(Role::Button).name("Sync Status").showing(),
            Duration::from_secs(15),
        )
        .await
        .unwrap();
    s.key(u32::from(b'q')).await.unwrap();
    window
        .wait_for(&Query::role(Role::Text).name("Task").showing(), WAIT)
        .await
        .unwrap();
    shoot(&s, &name("quick-add-offline")).await;
    s.type_text("Buy oat milk").await.unwrap();
    s.key(keysym::RETURN).await.unwrap();
    tick_e2e::poll(WAIT, "pending count", || async {
        let d = sync.description().await.ok()?;
        (d == "1 change waiting to sync").then_some(())
    })
    .await
    .unwrap();
    shoot(&s, &name("offline")).await;
}

async fn login(scheme: ColorScheme, narrow: bool) {
    let server = app::server("demo").await;
    let mut b = Session::builder()
        .env("TICK_API_BASE", server.base_url())
        .color_scheme(scheme);
    if narrow {
        b = b.narrow();
    }
    let s = b.start().await.unwrap();
    let window = s.window().await.unwrap();
    window
        .wait_for(&Query::role(Role::PasswordText).showing(), WAIT)
        .await
        .unwrap();
    let size = if narrow { "narrow" } else { "wide" };
    shoot(&s, &format!("login-{size}-{}", scheme_name(scheme))).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn wide_light() {
    matrix(ColorScheme::Light, false).await;
    login(ColorScheme::Light, false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn wide_dark() {
    matrix(ColorScheme::Dark, false).await;
    login(ColorScheme::Dark, false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn narrow_light() {
    matrix(ColorScheme::Light, true).await;
    login(ColorScheme::Light, true).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn narrow_dark() {
    matrix(ColorScheme::Dark, true).await;
    login(ColorScheme::Dark, true).await;
}
