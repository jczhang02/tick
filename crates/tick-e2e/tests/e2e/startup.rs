//! Startup budget (`SPEC.md`, data 2): with 1000 cached tasks, the first frame that shows
//! cached tasks comes within `TICK_STARTUP_BUDGET_MS`. The default is 300 ms for a release
//! binary (the SPEC test) and 500 ms for a debug binary, so a local `just e2e` runs the same
//! path without failing on an unoptimized build; CI sets 1500.
//!
//! One launch fills the cache; then the app is relaunched three times with the server
//! offline and the median is compared with the budget (single launches in headless mutter
//! vary by about 25 ms).

use std::time::Duration;

use crate::{WAIT, app};

fn budget() -> u64 {
    if let Some(ms) = std::env::var("TICK_STARTUP_BUDGET_MS")
        .ok()
        .and_then(|v| v.parse().ok())
    {
        return ms;
    }
    let release = tick_e2e::tick_bin()
        .ok()
        .and_then(|b| b.parent().map(|d| d.ends_with("release")))
        .unwrap_or(false);
    if release { 300 } else { 500 }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cached_tasks_within_budget() {
    let server = app::server("bulk:1000").await;
    let mut s = app::session(&server).start().await.unwrap();
    let window = s.window().await.unwrap();
    // First launch: the empty cache fills from the server.
    app::wait_first_sync(&server).await;
    tick_e2e::poll(
        Duration::from_secs(30),
        "tasks after the first sync",
        || async { (!app::task_names(&window).await.is_empty()).then_some(()) },
    )
    .await
    .unwrap();

    // Relaunches: the first data frame comes from the cache, before any network call.
    server.set_offline(true);
    let mut runs = Vec::new();
    for _ in 0..3 {
        s.activate_action("quit", None).await.unwrap();
        s.wait_app_exit(WAIT).await.unwrap();
        s.launch_app().unwrap();
        let ms = s.first_frame_ms(Duration::from_secs(30)).await.unwrap();
        let window = s.window().await.unwrap();
        assert!(
            !app::task_names(&window).await.is_empty(),
            "no cached tasks on screen"
        );
        runs.push(ms);
    }
    let mut sorted = runs.clone();
    sorted.sort_unstable();
    let median = sorted[1];
    let budget = budget();
    eprintln!(
        "startup: TICK_FIRST_DATA_FRAME_MS runs {runs:?}, median {median} (budget {budget} ms)"
    );
    assert!(
        median <= budget,
        "first data frame after {median} ms (runs {runs:?}), budget {budget} ms"
    );
}
