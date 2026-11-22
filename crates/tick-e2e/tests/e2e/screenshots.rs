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
    // Park the pointer in the middle of the header bar: no row shows a hover tint, and it is
    // away from the scrolled window's edge, where GTK shows the overlay scrollbar.
    let (mw, _) = s.monitor_size();
    let input = s.input().await.unwrap();
    input
        .move_to(i32::try_from(mw).unwrap() / 2, 2)
        .await
        .unwrap();
    shoot_here(s, name).await
}

/// Like [`shoot`] but leaves the pointer where it is (hover states).
async fn shoot_here(s: &Session, name: &str) -> PathBuf {
    let (mw, mh) = s.monitor_size();
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
            for n in list.find_all(&Query::role(Role::Label)).await {
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
    b = b.env("TICK_TEST_HOLD", "1");
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

    // The header bar title hand-off (4.2): scrolled past the large title.
    if narrow {
        back_to_sidebar(&s, &window).await;
    }
    app::open_view(&s, &window, "Upcoming").await;
    wait_row(&window, "Upcoming").await;
    let list = app::task_list(&window).await.extents().await.unwrap();
    let input = s.input().await.unwrap();
    let (x, y) = list.center();
    input.move_to(x, y).await.unwrap();
    input.scroll(6).await.unwrap();
    shoot(&s, &name("upcoming-scrolled")).await;
    // Further down: the rest of the fixed week (empty days with their "Add task" rows) and
    // the next month's header (signature move 3).
    let (x, y) = list.center();
    input.move_to(x, y).await.unwrap();
    input.scroll(24).await.unwrap();
    shoot(&s, &name("upcoming-later")).await;
    input.move_to(x, y).await.unwrap();
    input.scroll(14).await.unwrap();
    shoot(&s, &name("upcoming-month")).await;

    // A focused and selected row (keyboard), with a long title that wraps to 3 lines and
    // ends in an ellipsis in the narrow window.
    s.key(u32::from(b'g')).await.unwrap();
    s.key(u32::from(b'i')).await.unwrap();
    wait_row(&window, "Call the dentist to move the appointment").await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    s.key(keysym::DOWN).await.unwrap();
    shoot(&s, &name("selected")).await;

    // The p4 ring (DESIGN 9): hovered, and checked (a recurring task stays visible as
    // completed-pending).
    if !narrow {
        app::open_view(&s, &window, "Home").await;
        let rest = app::task_row(&window, "Replace the hallway light bulb").await;
        let ring = app::inside(&rest, Query::role(Role::CheckBox)).await;
        let (x, y) = ring.extents().await.unwrap().center();
        s.input().await.unwrap().move_to(x, y).await.unwrap();
        shoot_here(&s, &name("p4-hover")).await;
        let water = app::task_row(&window, "Water the plants").await;
        let ring = app::inside(&water, Query::role(Role::CheckBox)).await;
        s.click(&ring).await.unwrap();
        app::inside(
            &app::task_row(&window, "Water the plants").await,
            Query::role(Role::CheckBox).name("Mark as not done"),
        )
        .await;
        shoot(&s, &name("p4-checked")).await;
        // The completion hold (TICK_TEST_HOLD=1): ring filled, title struck through.
        let bulb = app::task_row(&window, "Replace the hallway light bulb").await;
        let ring = app::inside(&bulb, Query::role(Role::CheckBox)).await;
        s.click(&ring).await.unwrap();
        shoot_here(&s, &name("completing")).await;

        // A change the server rejected (4.4 failed change): the row is back with the
        // warning, and the toast explains it.
        server.inject(tick_fake_server::Failure::CommandError {
            command_type: "item_close".into(),
            args_contain: Some("6Xtask00000000007".into()),
            error: Some(serde_json::json!({
                "error_tag": "ITEM_NOT_FOUND", "error_code": 22, "http_code": 400,
                "error": "Item not found", "error_extra": {}
            })),
            count: 1,
        });
        app::open_view(&s, &window, "Inbox").await;
        let desk = app::task_row(&window, "Look into standing desks").await;
        let ring = app::inside(&desk, Query::role(Role::CheckBox)).await;
        s.click(&ring).await.unwrap();
        window
            .wait_for(
                &Query::role(Role::Button).name("Details").showing(),
                Duration::from_secs(20),
            )
            .await
            .unwrap();
        app::task_row(&window, "Look into standing desks").await;
        shoot(&s, &name("failed-change")).await;
        // The sync popover lists it (4.14); Dismiss Errors clears it for the offline shots.
        let sync = window
            .wait_for(
                &Query::role(Role::Button).name("Sync Status").showing(),
                WAIT,
            )
            .await
            .unwrap();
        s.click(&sync).await.unwrap();
        let dismiss = window
            .wait_for(
                &Query::role(Role::Button).name("Dismiss Errors").showing(),
                WAIT,
            )
            .await
            .unwrap();
        shoot(&s, &name("sync-popover-failed")).await;
        s.click(&dismiss).await.unwrap();
        window
            .wait_gone(
                &Query::role(Role::Button).name("Sync Status").showing(),
                WAIT,
            )
            .await
            .unwrap();

        // A tooltip, and the main menu (the floating surface, 2.6).
        let add = window
            .wait_for(&Query::role(Role::Button).name("Add Task").showing(), WAIT)
            .await
            .unwrap();
        let (x, y) = add.extents().await.unwrap().center();
        s.input().await.unwrap().move_to(x, y).await.unwrap();
        tokio::time::sleep(Duration::from_millis(1500)).await;
        shoot_here(&s, &name("tooltip")).await;
        let menu = window
            .wait_for(&Query::role(Role::Button).name("Main Menu").showing(), WAIT)
            .await
            .unwrap();
        s.click(&menu).await.unwrap();
        tokio::time::sleep(Duration::from_millis(500)).await;
        shoot(&s, &name("main-menu")).await;
        s.key(keysym::ESCAPE).await.unwrap();
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
    // Replace the kept draft.
    s.chord(&[keysym::CONTROL_L, u32::from(b'a')])
        .await
        .unwrap();
    s.type_text("Buy oat milk").await.unwrap();
    s.key(keysym::RETURN).await.unwrap();
    tick_e2e::poll(WAIT, "pending count", || async {
        let d = sync.description().await.ok()?;
        d.ends_with("waiting to sync").then_some(())
    })
    .await
    .unwrap();
    shoot(&s, &name("offline")).await;
    // The sync popover (4.14) on the floating surface.
    s.click(&sync).await.unwrap();
    window
        .wait_for(&Query::role(Role::Button).name("Sync Now").showing(), WAIT)
        .await
        .unwrap();
    shoot(&s, &name("sync-popover")).await;
    s.key(keysym::ESCAPE).await.unwrap();
}

/// Empty Today and Inbox (4.13).
async fn empty(scheme: ColorScheme, narrow: bool) {
    let server = app::server("empty").await;
    let s = session(&server, scheme, narrow).await;
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    let size = if narrow { "narrow" } else { "wide" };
    let sc = scheme_name(scheme);
    if narrow {
        app::open_view(&s, &window, "Today").await;
    }
    wait_row(&window, "All done for today").await;
    shoot(&s, &format!("empty-today-{size}-{sc}")).await;
    if narrow {
        back_to_sidebar(&s, &window).await;
    }
    app::open_view(&s, &window, "Inbox").await;
    wait_row(&window, "Inbox is empty").await;
    shoot(&s, &format!("empty-inbox-{size}-{sc}")).await;
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
    if full() {
        empty(ColorScheme::Light, false).await;
        chinese(ColorScheme::Light).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn wide_dark() {
    matrix(ColorScheme::Dark, false).await;
    login(ColorScheme::Dark, false).await;
    if full() {
        empty(ColorScheme::Dark, false).await;
        chinese(ColorScheme::Dark).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn narrow_light() {
    matrix(ColorScheme::Light, true).await;
    login(ColorScheme::Light, true).await;
    if full() {
        empty(ColorScheme::Light, true).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn narrow_dark() {
    matrix(ColorScheme::Dark, true).await;
    login(ColorScheme::Dark, true).await;
    if full() {
        empty(ColorScheme::Dark, true).await;
    }
}

/// Compiles `po/zh_CN.po` into a locale tree for `TICK_LOCALEDIR`, once per test process
/// (the wide sessions run in parallel and must not rewrite a catalog an app is reading).
fn zh_catalog() -> PathBuf {
    static DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(compile_zh_catalog).clone()
}

fn compile_zh_catalog() -> PathBuf {
    let po = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../po/zh_CN.po");
    let dir = std::env::temp_dir().join(format!("tick-e2e-locale-{}", std::process::id()));
    let out = dir.join("zh_CN/LC_MESSAGES");
    std::fs::create_dir_all(&out).unwrap();
    let status = std::process::Command::new("msgfmt")
        .arg("-o")
        .arg(out.join("tick.mo"))
        .arg(&po)
        .status()
        .expect("msgfmt (gettext) is needed for the Chinese screenshots");
    assert!(status.success(), "msgfmt {}", po.display());
    dir
}

/// The Chinese UI (`DESIGN.md` 6.2, 6.3, 9): dates, counts and punctuation.
async fn chinese(scheme: ColorScheme) {
    let sc = scheme_name(scheme);
    let name = |view: &str| format!("{view}-zh-wide-{sc}");
    let locale = zh_catalog();
    let zh = |server: &FakeServer| {
        app::session(server)
            .color_scheme(scheme)
            .locale("zh_CN.UTF-8")
            .env("LANGUAGE", "zh_CN")
            .env("TICK_LOCALEDIR", locale.display().to_string())
    };

    let server = app::server("demo").await;
    let s = zh(&server).start().await.unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    wait_row(&window, "已逾期").await;
    shoot(&s, &name("today")).await;
    app::open_view(&s, &window, "收件箱").await;
    wait_row(&window, "Call the dentist to move the appointment").await;
    shoot(&s, &name("inbox")).await;
    app::open_view(&s, &window, "即将到来").await;
    wait_row(&window, "即将到来").await;
    shoot(&s, &name("upcoming")).await;
    app::open_view(&s, &window, "收件箱").await;
    server.set_offline(true);
    window
        .wait_for(
            &Query::role(Role::Button).name("同步状态").showing(),
            Duration::from_secs(15),
        )
        .await
        .unwrap();
    s.key(u32::from(b'q')).await.unwrap();
    window
        .wait_for(&Query::role(Role::Text).name("任务").showing(), WAIT)
        .await
        .unwrap();
    s.type_text("Call Mom tomorrow 6pm p2 #Personal @phone")
        .await
        .unwrap();
    shoot(&s, &name("quick-add-offline")).await;
    drop(s);

    let server = app::server("empty").await;
    let s = zh(&server).start().await.unwrap();
    let window = s.window().await.unwrap();
    app::wait_first_sync(&server).await;
    wait_row(&window, "今天的任务都完成了").await;
    shoot(&s, &name("empty-today")).await;
}
