//! The fake server's HTTP surface: control endpoints and documented error shapes.

use serde_json::{Value, json};
use tick_fake_server::FakeServer;

async fn server() -> (FakeServer, reqwest::Client) {
    (FakeServer::start(0).await.unwrap(), reqwest::Client::new())
}

#[tokio::test]
async fn control_endpoints() {
    let (s, http) = server().await;
    let base = s.base_url();
    let r: Value = http
        .post(format!("{base}/__test/seed"))
        .json(&json!({"preset": "demo"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(r["ok"], true);
    let state: Value = http
        .get(format!("{base}/__test/state"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(state["items"].as_array().unwrap().len() > 50);

    // A fixture with documented defaults filled in.
    http.post(format!("{base}/__test/seed"))
        .json(
            &json!({"token": "tok2", "projects": [{"id": "p1", "name": "Only"}],
                      "items": [{"id": "i1", "content": "Hello", "project_id": "p1"}]}),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(s.token(), "tok2");
    let item = s.state().get("items", "i1").unwrap().clone();
    assert_eq!(item["priority"], 1);
    assert_eq!(item["checked"], false);

    // Offline: API connections fail, control endpoints keep working.
    http.post(format!("{base}/__test/fail"))
        .json(&json!({"kind": "offline"}))
        .send()
        .await
        .unwrap();
    let sync = http
        .post(format!("{base}/api/v1/sync"))
        .bearer_auth("tok2")
        .form(&[("sync_token", "*"), ("resource_types", "[\"all\"]")])
        .send()
        .await;
    let failed = match sync {
        Err(_) => true,
        Ok(resp) => resp.text().await.is_err(),
    };
    assert!(failed, "offline requests must fail at the connection level");
    http.delete(format!("{base}/__test/fail"))
        .send()
        .await
        .unwrap();
    let ok = http
        .post(format!("{base}/api/v1/sync"))
        .bearer_auth("tok2")
        .form(&[("sync_token", "*"), ("resource_types", "[\"all\"]")])
        .send()
        .await
        .unwrap();
    assert_eq!(ok.status(), 200);
    let body: Value = ok.json().await.unwrap();
    assert_eq!(body["full_sync"], true);
    assert!(body["sync_token"].as_str().is_some());

    let log: Value = http
        .get(format!("{base}/__test/requests"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(log.as_array().unwrap().len(), 2);
    assert_eq!(log[1]["body"]["resource_types"], json!(["all"]));

    http.post(format!("{base}/__test/reset"))
        .send()
        .await
        .unwrap();
    assert!(s.state().get("items", "i1").is_none());
}

#[tokio::test]
async fn documented_errors() {
    let (s, http) = server().await;
    let base = s.base_url();
    let unauthorized = http
        .post(format!("{base}/api/v1/sync"))
        .bearer_auth("nope")
        .form(&[("sync_token", "*"), ("resource_types", "[\"all\"]")])
        .send()
        .await
        .unwrap();
    assert_eq!(unauthorized.status(), 401);
    assert!(unauthorized.headers().contains_key("www-authenticate"));
    let body: Value = unauthorized.json().await.unwrap();
    assert_eq!(body["error_tag"], "UNAUTHORIZED");
    assert_eq!(body["error_code"], 477);

    s.inject(tick_fake_server::Failure::Status {
        status: 429,
        retry_after: Some(3),
        count: 1,
        path: None,
        body_contains: None,
    });
    let limited = http
        .post(format!("{base}/api/v1/sync"))
        .bearer_auth(s.token())
        .form(&[("sync_token", "*"), ("resource_types", "[\"all\"]")])
        .send()
        .await
        .unwrap();
    assert_eq!(limited.status(), 429);
    assert_eq!(limited.headers()["retry-after"], "3");
    let body: Value = limited.json().await.unwrap();
    assert_eq!(body["error_extra"]["retry_after"], 3);

    let get = |path: &str| {
        http.get(format!("{base}{path}"))
            .bearer_auth(s.token())
            .send()
    };
    assert_eq!(
        get("/api/v1/tasks/filter?query=today,overdue")
            .await
            .unwrap()
            .status(),
        400
    );
    assert_eq!(
        get("/api/v1/tasks/filter?query=created:today")
            .await
            .unwrap()
            .status(),
        400
    );
    let ok: Value = get("/api/v1/tasks/filter?query=today%20%7C%20overdue")
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(ok["results"].is_array());
    assert!(ok["next_cursor"].is_null());
    assert_eq!(
        get("/api/v1/tasks/completed/by_completion_date?until=2026-03-01T00:00:00Z")
            .await
            .unwrap()
            .status(),
        400
    );
    assert_eq!(
        get("/api/v1/tasks/completed/by_completion_date?since=2025-01-01T00:00:00Z&until=2026-03-01T00:00:00Z")
            .await
            .unwrap()
            .status(),
        400
    );
    assert_eq!(
        get("/api/v1/tasks/completed/by_completion_date?since=2026-02-01T00:00:00Z&until=2026-03-01T00:00:00Z&limit=201")
            .await
            .unwrap()
            .status(),
        400
    );
    let empty: Value = get(
        "/api/v1/tasks/completed/by_completion_date?since=2026-02-01T00:00:00Z&until=2026-03-01T00:00:00Z",
    )
    .await
    .unwrap()
    .json()
    .await
    .unwrap();
    assert_eq!(empty["items"], json!([]));
    assert!(
        empty.get("next_cursor").is_none(),
        "omitted on the last page"
    );

    // Commands: unknown types and invalid args fail per command with documented shapes.
    let cmds = json!([
        {"type": "item_add", "uuid": "u1", "temp_id": "t1", "args": {"content": "ok"}},
        {"type": "no_such_command", "uuid": "u2", "args": {}},
        {"type": "item_update", "uuid": "u3", "args": {"id": "missing", "content": "x"}},
        {"type": "note_add", "uuid": "u4", "temp_id": "t2",
         "args": {"item_id": "t1", "content": "temp id from the same request"}}
    ]);
    let r: Value = http
        .post(format!("{base}/api/v1/sync"))
        .bearer_auth(s.token())
        .form(&[
            ("sync_token", "*"),
            ("resource_types", "[]"),
            ("commands", &cmds.to_string()),
        ])
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(r["sync_status"]["u1"], "ok");
    assert_eq!(r["sync_status"]["u2"]["http_code"], 400);
    assert_eq!(r["sync_status"]["u3"]["error_tag"], "ITEM_NOT_FOUND");
    assert_eq!(r["sync_status"]["u4"], "ok");
    assert!(r["temp_id_mapping"]["t1"].is_string());
    assert!(r["temp_id_mapping"]["t2"].is_string());
}
