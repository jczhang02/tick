//! Smoke tests against the real Todoist API. They run only when `TODOIST_TEST_TOKEN` is set
//! (use a separate test account); otherwise each test returns at once.

use tick_core::{Client, Command};

fn token() -> Option<String> {
    std::env::var("TODOIST_TEST_TOKEN")
        .ok()
        .map(|t| t.trim().to_owned())
        .filter(|t| !t.is_empty())
}

/// `DESIGN.md` 4.7: "Add task" rows in Upcoming and Today prefill quick add with an ISO
/// date. Quick add must parse it in any account language.
#[tokio::test]
async fn quick_add_iso_date() {
    let Some(token) = token() else {
        eprintln!("skipped: TODOIST_TEST_TOKEN is not set");
        return;
    };
    let client = Client::new(tick_core::api::DEFAULT_API_BASE, token);
    let item = client
        .quick_add("Smoke 2026-11-26", &tick_core::command::new_uuid())
        .await
        .unwrap();
    let due = item["due"]["date"].as_str().map(str::to_owned);
    if let Some(id) = item["id"].as_str() {
        let _ = client.sync("*", &[], &[Command::item_delete(id)]).await;
    }
    assert_eq!(due.as_deref(), Some("2026-11-26"), "{item}");
}
