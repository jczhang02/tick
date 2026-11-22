//! Startup query budget: with 1000 cached tasks, opening the store and running the Today
//! and project view queries must stay well under 50 ms in a release build.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tick_core::store::Store;
use tick_core::{Client, Now, SyncEngine};
use tick_fake_server::{FakeServer, State, seed};

const NOW: &str = "2026-03-10T09:00:00+08:00";

#[tokio::test]
async fn open_today_and_project_view_with_1000_tasks() {
    let mut state = State::new("perf");
    state.now = chrono::DateTime::parse_from_rfc3339(NOW).ok();
    seed::bulk(&mut state, 1000);
    let server = FakeServer::start_with(0, state).await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cache.sqlite3");
    let clock: Arc<dyn Fn() -> Now + Send + Sync> = Arc::new(|| Now::fixed(NOW).unwrap());
    {
        let mut store = Store::open(&path).unwrap();
        store.set_clock(clock.clone());
        let mut engine = SyncEngine::new(store, Client::new(server.base_url(), "perf"));
        engine.sync_once().await.unwrap();
        let n: i64 = engine
            .store()
            .connection()
            .query_row("SELECT COUNT(*) FROM items", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1000);
    }

    let mut best = Duration::MAX;
    for _ in 0..5 {
        let start = Instant::now();
        let mut store = Store::open(&path).unwrap();
        store.set_clock(clock.clone());
        let today = store.today().unwrap();
        let tree = store.projects_tree().unwrap();
        let view = store.project_view(&tree[1].project.id).unwrap().unwrap();
        let elapsed = start.elapsed();
        assert!(!today.today.is_empty() && !today.overdue.is_empty());
        assert!(!view.sections.is_empty());
        best = best.min(elapsed);
    }
    // Release: the real budget. Debug builds (CI runs them on shared runners) get a generous
    // bound that still catches accidental full scans with JSON parsing per row.
    let budget = if cfg!(debug_assertions) {
        Duration::from_millis(500)
    } else {
        Duration::from_millis(50)
    };
    eprintln!("open + today + projects + project view: {best:?} (budget {budget:?})");
    assert!(best < budget, "{best:?} exceeds {budget:?}");
}
