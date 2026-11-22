//! Sync engine against an in-process fake server.

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tick_core::api::{ApiError, Client, RESOURCE_TYPES};
use tick_core::engine::{ConnState, EngineError, SyncEngine};
use tick_core::store::{FilterGroupResult, Kind, Store, Task};
use tick_core::{Command, MoveTarget, Now};
use tick_fake_server::{Failure, FakeServer, State, seed};

const NOW: &str = "2026-03-10T09:00:00+08:00";

struct Env {
    server: FakeServer,
    engine: SyncEngine,
    _dir: tempfile::TempDir,
}

fn fixed_clock() -> Arc<dyn Fn() -> Now + Send + Sync> {
    Arc::new(|| Now::fixed(NOW).unwrap())
}

async fn env_with(preset: &str) -> Env {
    let mut state = State::new("secret-token");
    state.now = chrono::DateTime::parse_from_rfc3339(NOW).ok();
    seed::apply_preset(&mut state, preset, 200).unwrap();
    let server = FakeServer::start_with(0, state).await.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path().join("cache.sqlite3")).unwrap();
    store.set_clock(fixed_clock());
    let client = Client::new(server.base_url(), server.token());
    Env {
        server,
        engine: SyncEngine::new(store, client),
        _dir: dir,
    }
}

async fn env() -> Env {
    env_with("demo").await
}

impl Env {
    fn store(&self) -> &Store {
        self.engine.store()
    }
    fn enqueue(&mut self, c: Command) {
        self.engine.store_mut().enqueue(c).unwrap();
    }
    fn task(&self, id: &str) -> Option<Task> {
        self.store().task(id).unwrap()
    }
    fn server_item(&self, id: &str) -> Option<Value> {
        self.server.state().get("items", id).cloned()
    }
    fn server_items_with_content(&self, content: &str) -> Vec<Value> {
        self.server
            .state()
            .live("items")
            .filter(|v| v["content"] == content)
            .cloned()
            .collect()
    }
    fn sync_requests_with_commands(&self) -> Vec<Value> {
        self.server
            .requests_to("/api/v1/sync")
            .into_iter()
            .filter(|r| r.body.get("commands").is_some())
            .map(|r| r.body)
            .collect()
    }
    /// All local task ids (to check that no temp id survived).
    fn local_ids(&self) -> Vec<String> {
        let mut stmt = self
            .store()
            .connection()
            .prepare("SELECT id FROM items")
            .unwrap();
        stmt.query_map([], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }
}

fn is_uuid(s: &str) -> bool {
    s.len() == 36 && s.matches('-').count() == 4
}

#[tokio::test]
async fn full_sync_fills_the_cache() {
    let mut e = env().await;
    let report = e.engine.sync_once().await.unwrap();
    assert!(report.full_sync);
    assert_eq!(e.engine.status().state, ConnState::Online);
    let first = &e.server.requests_to("/api/v1/sync")[0];
    assert_eq!(first.body["sync_token"], "*");
    assert_eq!(first.body["resource_types"], json!(RESOURCE_TYPES));

    let store = e.store();
    let tree = store.projects_tree().unwrap();
    assert_eq!(tree[0].project.name, "Inbox");
    let web = tree
        .iter()
        .find(|p| p.project.name == "Website redesign")
        .unwrap();
    assert_eq!(web.depth, 1);
    assert!(web.active_count > 0);
    let today = store.today().unwrap();
    assert!(!today.today.is_empty());
    assert!(!today.overdue.is_empty());
    let user = store.user().unwrap().unwrap();
    assert_eq!(user.full_name, "Sam Rivera");
    // The API token in the user object is never cached.
    let raw: String = store
        .connection()
        .query_row("SELECT value FROM meta WHERE key = 'user'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert!(!raw.contains("never-cache-this-token"));
    assert!(store.sync_token().unwrap().is_some());
    let work = tree.iter().find(|p| p.project.name == "Work").unwrap();
    assert_eq!(store.collaborators(&work.project.id).unwrap().len(), 3);
    assert!(!store.comments("6Xtask00000000010").unwrap().is_empty());
    assert_eq!(store.reminders("6Xtask00000000010").unwrap().len(), 1);
    assert!(store.completed_count("project", &work.project.id).unwrap() >= 1);
    assert!(
        store
            .labels()
            .unwrap()
            .iter()
            .any(|l| l.label.name == "errand")
    );
    assert_eq!(store.filters().unwrap().len(), 5);
}

#[tokio::test]
async fn incremental_sync_returns_only_changes() {
    let mut e = env().await;
    e.engine.sync_once().await.unwrap();
    let token = e.store().sync_token().unwrap().unwrap();
    {
        let mut st = e.server.state();
        let mut v = st.get("items", "6Xtask00000000001").unwrap().clone();
        v["content"] = json!("Call the dentist (moved)");
        st.put("items", v);
        let mut n = st.get("items", "6Xtask00000000002").unwrap().clone();
        n["id"] = json!("6Xnewtask00000001");
        n["content"] = json!("Brand new from another device");
        st.put("items", n);
    }
    // What the server sends for this token: only the two changed items.
    let client = Client::new(e.server.base_url(), e.server.token());
    let raw = client.sync(&token, RESOURCE_TYPES, &[]).await.unwrap();
    assert!(!raw.full_sync);
    assert_eq!(raw.items.as_ref().unwrap().len(), 2);
    assert!(raw.projects.as_ref().unwrap().is_empty());

    let report = e.engine.sync_once().await.unwrap();
    assert!(!report.full_sync);
    assert!(
        report
            .changes
            .ids(Kind::Item)
            .any(|id| id == "6Xnewtask00000001")
    );
    assert_eq!(
        e.task("6Xtask00000000001").unwrap().content,
        "Call the dentist (moved)"
    );
    assert_eq!(
        e.task("6Xnewtask00000001").unwrap().content,
        "Brand new from another device"
    );
    let last = e.server.requests_to("/api/v1/sync").pop().unwrap();
    assert_ne!(last.body["sync_token"], "*");
}

#[tokio::test]
async fn deletions_remove_rows() {
    let mut e = env().await;
    e.engine.sync_once().await.unwrap();
    {
        let mut st = e.server.state();
        st.update("items", "6Xtask00000000004", |v| {
            v["is_deleted"] = json!(true)
        });
        st.update("projects", "6Xgroc00000000001", |v| {
            v["is_deleted"] = json!(true)
        });
        st.update("labels", "6Xlabel0000000006", |v| {
            v["is_deleted"] = json!(true)
        });
    }
    e.engine.sync_once().await.unwrap();
    assert!(e.task("6Xtask00000000004").is_none());
    assert!(e.store().project("6Xgroc00000000001").unwrap().is_none());
    assert!(
        !e.store()
            .labels()
            .unwrap()
            .iter()
            .any(|l| l.label.name == "15min")
    );
    // Completed elsewhere: the task leaves the active views.
    e.server.state().update("items", "6Xtask00000000005", |v| {
        v["checked"] = json!(true);
        v["completed_at"] = json!("2026-03-10T01:00:00Z");
    });
    e.engine.sync_once().await.unwrap();
    let inbox = e.store().inbox().unwrap().unwrap();
    assert!(!TaskNodeIds::of(&inbox.tasks).contains(&"6Xtask00000000005".to_string()));
}

struct TaskNodeIds;
impl TaskNodeIds {
    fn of(nodes: &[tick_core::store::TaskNode]) -> Vec<String> {
        tick_core::store::TaskNode::flatten(nodes)
            .into_iter()
            .map(|(_, t)| t.id.clone())
            .collect()
    }
}

#[tokio::test]
async fn offline_queue_then_reconnect() {
    let mut e = env().await;
    e.engine.sync_once().await.unwrap();
    e.server.set_offline(true);

    let add =
        Command::item_add(json!({"content": "Written offline", "project_id": "6Xwork00000000001"}));
    let temp = add.temp_id.clone().unwrap();
    e.enqueue(add);
    e.enqueue(Command::item_update(
        &temp,
        json!({"description": "with details", "priority": 4}),
    ));
    e.enqueue(Command::item_close("6Xtask00000000001"));

    let err = e.engine.sync_once().await.unwrap_err();
    assert!(
        matches!(err, EngineError::Api(ApiError::Network(_))),
        "{err:?}"
    );
    assert_eq!(e.engine.status().state, ConnState::Offline);
    assert_eq!(e.engine.status().pending, 3);
    // Optimistic state is visible offline.
    let local = e.task(&temp).unwrap();
    assert!(local.pending);
    assert_eq!(local.description, "with details");
    assert!(e.task("6Xtask00000000001").unwrap().checked);
    assert!(e.server_items_with_content("Written offline").is_empty());

    e.server.set_offline(false);
    let report = e.engine.sync_once().await.unwrap();
    assert_eq!(report.acknowledged, 3);
    assert_eq!(e.store().pending_count().unwrap(), 0);
    assert_eq!(e.engine.status().state, ConnState::Online);
    let on_server = e.server_items_with_content("Written offline");
    assert_eq!(on_server.len(), 1);
    assert_eq!(on_server[0]["description"], "with details");
    assert_eq!(on_server[0]["priority"], 4);
    let real = on_server[0]["id"].as_str().unwrap();
    assert!(e.task(&temp).is_none());
    let t = e.task(real).unwrap();
    assert!(!t.pending);
    assert_eq!(t.project_id, "6Xwork00000000001");
    assert!(e.server_item("6Xtask00000000001").unwrap()["checked"] == true);
}

#[tokio::test]
async fn temp_ids_resolve_across_dependent_commands_and_batches() {
    let mut e = env_with("empty").await;
    e.engine.sync_once().await.unwrap();
    e.server.set_offline(true);

    let project = Command::project_add(json!({"name": "Launch"}));
    let p = project.temp_id.clone().unwrap();
    e.enqueue(project);
    let section = Command::section_add(&p, "Prep");
    let s = section.temp_id.clone().unwrap();
    e.enqueue(section);
    let parent = Command::item_add(json!({"content": "Parent", "project_id": p, "section_id": s}));
    let i = parent.temp_id.clone().unwrap();
    e.enqueue(parent);
    e.enqueue(Command::item_add(
        json!({"content": "Child", "parent_id": i}),
    ));
    e.enqueue(Command::note_add(&i, "A comment"));
    e.enqueue(Command::reminder_add(
        json!({"item_id": i, "type": "absolute", "due": {"date": "2026-03-11T09:00:00"}}),
    ));
    // Enough commands for a second /sync batch; they reference ids created in batch one.
    for n in 0..110 {
        e.enqueue(Command::item_add(
            json!({"content": format!("Bulk {n}"), "project_id": p}),
        ));
    }
    let local_view = e.store().project_view(&p).unwrap().unwrap();
    assert_eq!(local_view.sections.len(), 1);
    assert_eq!(local_view.sections[0].tasks[0].children.len(), 1);
    e.enqueue(Command::item_move(&i, MoveTarget::Project(p.clone())));

    e.server.set_offline(false);
    let report = e.engine.sync_once().await.unwrap();
    assert_eq!(e.store().pending_count().unwrap(), 0, "{report:?}");
    assert!(e.store().failed_commands().unwrap().is_empty());
    let batches = e.sync_requests_with_commands();
    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0]["commands"].as_array().unwrap().len(), 100);
    // The second batch was rewritten to real ids before it was sent.
    let second = serde_json::to_string(&batches[1]["commands"]).unwrap();
    assert!(!second.contains(&p), "temp project id leaked into batch 2");

    let st = e.server.state();
    let launch = st.find("projects", "name", "Launch").unwrap().clone();
    let pid = launch["id"].as_str().unwrap();
    let parent = st.find("items", "content", "Parent").unwrap().clone();
    let child = st.find("items", "content", "Child").unwrap().clone();
    assert_eq!(parent["project_id"], pid);
    assert_eq!(child["parent_id"], parent["id"]);
    assert_eq!(
        st.find("notes", "content", "A comment").unwrap()["item_id"],
        parent["id"]
    );
    assert_eq!(st.live("reminders").count(), 1);
    assert_eq!(
        st.live("items").filter(|v| v["project_id"] == pid).count(),
        112
    );
    drop(st);
    assert!(
        e.local_ids().iter().all(|id| !is_uuid(id)),
        "temp ids left in the cache"
    );
    let view = e.store().project_view(pid).unwrap().unwrap();
    assert_eq!(view.sections.len(), 1);
    assert_eq!(
        view.sections[0].tasks.len(),
        0,
        "the parent moved out of the section"
    );
    assert_eq!(view.tasks.len(), 111);
}

#[tokio::test]
async fn retry_with_the_same_uuid_is_idempotent() {
    let mut e = env().await;
    e.engine.sync_once().await.unwrap();
    e.server.inject(Failure::ErrorAfterCommit {
        status: 503,
        count: 1,
    });
    let add = Command::item_add(json!({"content": "Exactly once"}));
    let uuid = add.uuid.clone();
    e.enqueue(add);
    let err = e.engine.sync_once().await.unwrap_err();
    assert!(err.is_transient(), "{err:?}");
    assert_eq!(e.store().pending_count().unwrap(), 1);
    // The server ran the command although the client saw an error.
    assert_eq!(e.server_items_with_content("Exactly once").len(), 1);

    e.engine.sync_once().await.unwrap();
    assert_eq!(e.server_items_with_content("Exactly once").len(), 1);
    assert_eq!(e.store().pending_count().unwrap(), 0);
    let sent = e.sync_requests_with_commands();
    assert_eq!(sent.len(), 2);
    for body in &sent {
        assert_eq!(body["commands"][0]["uuid"], uuid);
    }
    // The replayed temp_id_mapping lets the client replace its temp row.
    let real = e.server_items_with_content("Exactly once")[0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(e.task(&real).is_some());
    assert!(e.local_ids().iter().all(|id| !is_uuid(id)));
}

#[tokio::test]
async fn rejected_command_rolls_back() {
    let mut e = env().await;
    e.engine.sync_once().await.unwrap();
    let before = e.task("6Xtask00000000007").unwrap();

    e.server.inject(Failure::CommandError {
        command_type: "item_update".into(),
        args_contain: None,
        error: None,
        count: 1,
    });
    e.enqueue(Command::item_update(
        "6Xtask00000000007",
        json!({"content": "Rejected edit"}),
    ));
    // A natural 400 from the server: invalid priority.
    e.enqueue(Command::item_update(
        "6Xtask00000000004",
        json!({"priority": 9}),
    ));
    // A rejected add; its dependent comment fails too instead of wedging the queue.
    e.server.inject(Failure::CommandError {
        command_type: "item_add".into(),
        args_contain: Some("doomed".into()),
        error: Some(
            json!({"error_tag": "MAX_ITEMS_LIMIT_REACHED", "error_code": 48,
                           "error": "Maximum number of items reached", "http_code": 403,
                           "error_extra": {}}),
        ),
        count: 1,
    });
    let add = Command::item_add(json!({"content": "doomed"}));
    let doomed = add.temp_id.clone().unwrap();
    e.enqueue(add);
    e.enqueue(Command::note_add(&doomed, "never"));
    e.enqueue(Command::item_update(
        "6Xtask00000000003",
        json!({"content": "Accepted edit"}),
    ));
    assert_eq!(
        e.task("6Xtask00000000007").unwrap().content,
        "Rejected edit"
    );

    let report = e.engine.sync_once().await.unwrap();
    assert_eq!(report.failed.len(), 4, "{:?}", report.failed);
    assert_eq!(report.acknowledged, 1);
    assert_eq!(e.store().pending_count().unwrap(), 0);
    let failed = e.store().failed_commands().unwrap();
    assert_eq!(failed.len(), 4);
    assert!(
        failed
            .iter()
            .any(|f| f.detail.as_ref().and_then(|d| d.error_code) == Some(15))
    );
    // Rolled back to server state, without a full re-sync.
    let after = e.task("6Xtask00000000007").unwrap();
    assert_eq!(after.content, before.content);
    assert!(!after.pending);
    assert_eq!(e.task("6Xtask00000000004").unwrap().priority, 1);
    assert!(e.task(&doomed).is_none());
    assert_eq!(
        e.task("6Xtask00000000003").unwrap().content,
        "Accepted edit"
    );
    assert!(e.engine.status().last_error.is_some());
    assert_eq!(e.engine.status().failed, 4);
    assert!(!report.full_sync);
}

#[tokio::test]
async fn backs_off_on_429_and_5xx() {
    let mut e = env().await;
    e.engine.sync_once().await.unwrap();
    e.server.inject(Failure::Status {
        status: 429,
        retry_after: Some(1),
        count: 1,
        path: None,
    });
    e.enqueue(Command::item_update(
        "6Xtask00000000007",
        json!({"content": "After backoff"}),
    ));
    let start = Instant::now();
    e.engine
        .sync_with_retry(3, Duration::from_millis(10))
        .await
        .unwrap();
    assert!(
        start.elapsed() >= Duration::from_secs(1),
        "retry_after was not honored"
    );
    assert_eq!(
        e.server_item("6Xtask00000000007").unwrap()["content"],
        "After backoff"
    );
    let statuses: Vec<u16> = e
        .server
        .requests_to("/api/v1/sync")
        .iter()
        .map(|r| r.status)
        .collect();
    assert_eq!(&statuses[statuses.len() - 2..], &[429, 200]);

    // A plain 429 surfaces with its retry_after.
    e.server.inject(Failure::Status {
        status: 429,
        retry_after: Some(7),
        count: 1,
        path: None,
    });
    let err = e.engine.sync_once().await.unwrap_err();
    assert_eq!(err.retry_after(), Some(7));

    e.server.inject(Failure::Status {
        status: 500,
        retry_after: None,
        count: 2,
        path: None,
    });
    e.engine
        .sync_with_retry(4, Duration::from_millis(10))
        .await
        .unwrap();

    // Retryable command errors keep the command queued.
    e.server.inject(Failure::CommandError {
        command_type: "item_update".into(),
        args_contain: None,
        error: Some(
            json!({"error_tag": "SERVICE_UNAVAILABLE", "error_code": 503,
                           "error": "Try later", "http_code": 503,
                           "error_extra": {"retry_after": 2}}),
        ),
        count: 1,
    });
    e.enqueue(Command::item_update(
        "6Xtask00000000007",
        json!({"content": "Second"}),
    ));
    let err = e.engine.sync_once().await.unwrap_err();
    assert_eq!(err.retry_after(), Some(2));
    assert_eq!(e.store().pending_count().unwrap(), 1);
    assert_eq!(e.task("6Xtask00000000007").unwrap().content, "Second");
    e.engine.sync_once().await.unwrap();
    assert_eq!(
        e.server_item("6Xtask00000000007").unwrap()["content"],
        "Second"
    );
}

#[tokio::test]
async fn unauthorized_is_not_retried() {
    let e = env().await;
    let store = Store::open_in_memory().unwrap();
    let mut engine = SyncEngine::new(store, Client::new(e.server.base_url(), "wrong"));
    let err = engine
        .sync_with_retry(5, Duration::from_millis(10))
        .await
        .unwrap_err();
    assert!(matches!(err, EngineError::Api(ApiError::Unauthorized)));
    assert_eq!(engine.status().state, ConnState::Unauthorized);
    assert_eq!(e.server.requests_to("/api/v1/sync").len(), 1);
}

#[tokio::test]
async fn server_wins_and_queued_commands_are_reapplied() {
    let mut e = env().await;
    e.engine.sync_once().await.unwrap();
    e.server.set_offline(true);
    e.enqueue(Command::item_update(
        "6Xtask00000000007",
        json!({"content": "Local title"}),
    ));
    e.enqueue(Command::item_update(
        "6Xtask00000000004",
        json!({"content": "Edit of a task deleted elsewhere"}),
    ));
    // Another device changes the same task and deletes another one.
    {
        let mut st = e.server.state();
        st.update("items", "6Xtask00000000007", |v| {
            v["content"] = json!("Server title");
            v["description"] = json!("Server description");
            v["priority"] = json!(3);
        });
        st.update("items", "6Xtask00000000004", |v| {
            v["is_deleted"] = json!(true)
        });
    }
    e.server.set_offline(false);
    // Our update cannot be applied yet (retryable error): the read still lands, and the
    // queued change is re-applied on top of the server's version.
    e.server.inject(Failure::CommandError {
        command_type: "item_update".into(),
        args_contain: Some("Local title".into()),
        error: Some(json!({"error": "busy", "http_code": 503, "error_code": 503})),
        count: 1,
    });
    let _ = e.engine.sync_once().await.unwrap_err();
    let t = e.task("6Xtask00000000007").unwrap();
    assert_eq!(t.content, "Local title");
    assert_eq!(t.description, "Server description");
    assert_eq!(t.priority, 3);
    assert!(t.pending);
    // The task deleted on the server stays deleted; the edit to it is dropped.
    assert!(e.task("6Xtask00000000004").is_none());
    assert_eq!(e.store().failed_commands().unwrap().len(), 1);

    e.engine.sync_once().await.unwrap();
    let t = e.task("6Xtask00000000007").unwrap();
    assert_eq!(t.content, "Local title");
    assert_eq!(t.description, "Server description");
    assert!(!t.pending);
    assert_eq!(
        e.server_item("6Xtask00000000007").unwrap()["content"],
        "Local title"
    );
}

#[tokio::test]
async fn quick_add_online() {
    let mut e = env().await;
    e.engine.sync_once().await.unwrap();
    e.enqueue(Command::quick_add(
        "Buy milk tomorrow #Groceries @errand p1",
    ));
    e.engine.sync_once().await.unwrap();
    let quick = e.server.requests_to("/api/v1/tasks/quick");
    assert_eq!(quick.len(), 1);
    assert_eq!(
        quick[0].body["text"],
        "Buy milk tomorrow #Groceries @errand p1"
    );
    let view = e
        .store()
        .project_view("6Xgroc00000000001")
        .unwrap()
        .unwrap();
    let milk = view
        .tasks
        .iter()
        .find(|n| n.task.content == "Buy milk")
        .unwrap();
    assert_eq!(milk.task.p(), 1);
    assert_eq!(milk.task.labels, vec!["errand"]);
    assert_eq!(milk.task.due.unwrap().date.to_string(), "2026-03-11");
    assert!(!milk.task.pending);
    assert!(e.local_ids().iter().all(|id| !is_uuid(id)));
}

#[tokio::test]
async fn quick_add_offline_then_online() {
    let mut e = env().await;
    e.engine.sync_once().await.unwrap();
    e.server.set_offline(true);
    let q = Command::quick_add("Call plumber today p2");
    let temp = q.temp_id.clone().unwrap();
    e.enqueue(q);
    // A later edit references the placeholder.
    e.enqueue(Command::item_update(
        &temp,
        json!({"description": "about the tap"}),
    ));
    assert!(e.engine.sync_once().await.is_err());
    let placeholder = e.task(&temp).unwrap();
    assert_eq!(placeholder.content, "Call plumber today p2");
    assert!(placeholder.pending);
    let inbox = e.store().inbox().unwrap().unwrap();
    assert!(TaskNodeIds::of(&inbox.tasks).contains(&temp));
    assert!(
        e.server
            .requests_to("/api/v1/tasks/quick")
            .iter()
            .all(|r| r.status == 0)
    );

    e.server.set_offline(false);
    e.engine.sync_once().await.unwrap();
    assert!(e.task(&temp).is_none());
    let created = e.server_items_with_content("Call plumber");
    assert_eq!(created.len(), 1);
    assert_eq!(created[0]["description"], "about the tap");
    assert_eq!(created[0]["priority"], 3);
    let local = e.task(created[0]["id"].as_str().unwrap()).unwrap();
    assert_eq!(local.description, "about the tap");
    assert_eq!(e.store().pending_count().unwrap(), 0);
}

#[tokio::test]
async fn completed_history_pagination() {
    let mut e = env_with("empty").await;
    {
        let mut st = e.server.state();
        let base = chrono::DateTime::parse_from_rfc3339(NOW).unwrap();
        for i in 0..120 {
            let at = (base - chrono::Duration::hours(i * 4))
                .with_timezone(&chrono::Utc)
                .format("%Y-%m-%dT%H:%M:%S%.6fZ")
                .to_string();
            st.add_completed(
                json!({"id": format!("done{i:04}"), "content": format!("Done {i}"),
                                    "project_id": seed::INBOX_ID, "checked": true,
                                    "completed_at": at, "priority": 1}),
            );
        }
        // Older than 30 days: only reached by "Load more".
        st.add_completed(
            json!({"id": "old0001", "content": "Old", "project_id": seed::INBOX_ID,
                                "checked": true, "completed_at": "2026-01-20T10:00:00.000000Z"}),
        );
    }
    e.engine.sync_once().await.unwrap();
    let n = e.engine.fetch_completed_window(30).await.unwrap();
    assert_eq!(n, 120);
    let pages = e
        .server
        .requests_to("/api/v1/tasks/completed/by_completion_date");
    assert_eq!(pages.len(), 3);
    assert!(pages[0].query.contains("since="));
    assert!(pages[1].query.contains("cursor="));
    let history = e.store().completed_history(None, 200).unwrap();
    assert_eq!(history.len(), 120);
    assert_eq!(history[0].item.content, "Done 0");
    let older = e
        .store()
        .completed_history(Some(&history[49].completed_at), 50)
        .unwrap();
    assert_eq!(older[0].item.content, "Done 50");

    // UI paging: 50 per "Load more", then older windows.
    let (c1, more1) = e.engine.fetch_completed(true).await.unwrap();
    assert_eq!((c1, more1), (50, true));
    let (c2, _) = e.engine.fetch_completed(false).await.unwrap();
    let (c3, _) = e.engine.fetch_completed(false).await.unwrap();
    assert_eq!(c2 + c3, 70);
    let (c4, more4) = e.engine.fetch_completed(false).await.unwrap();
    assert_eq!(c4, 1);
    assert!(more4);
    assert!(
        e.store()
            .completed_history(None, 500)
            .unwrap()
            .iter()
            .any(|c| c.item.id == "old0001")
    );
}

#[tokio::test]
async fn unsupported_filter_uses_the_server() {
    let mut e = env().await;
    e.engine.sync_once().await.unwrap();
    let groups = e.store().filter_view("@home*, today").unwrap();
    assert!(matches!(
        groups[0].result,
        FilterGroupResult::Server { cached: None, .. }
    ));
    let FilterGroupResult::Local(today) = &groups[1].result else {
        panic!()
    };
    assert!(!today.is_empty());
    e.engine.fetch_filter("@home*, today").await.unwrap();
    let reqs = e.server.requests_to("/api/v1/tasks/filter");
    assert_eq!(
        reqs.len(),
        1,
        "only the unsupported part goes to the server"
    );
    assert!(reqs[0].query.contains("query=%40home"));
    let groups = e.store().filter_view("@home*, today").unwrap();
    let FilterGroupResult::Server {
        cached: Some(tasks),
        ..
    } = &groups[0].result
    else {
        panic!("{:?}", groups[0])
    };
    assert!(!tasks.is_empty());
    assert!(
        tasks
            .iter()
            .all(|t| t.labels.iter().any(|l| l.starts_with("home")))
    );
}

#[tokio::test]
async fn local_mutations_round_trip() {
    let mut e = env().await;
    e.engine.sync_once().await.unwrap();
    let label = Command::label_add(json!({"name": "urgent-ish", "color": "red"}));
    e.enqueue(label);
    let filter = Command::filter_add(json!({"name": "Mine", "query": "assigned to: me"}));
    e.enqueue(filter);
    e.enqueue(Command::item_update(
        "6Xtask00000000007",
        json!({"labels": ["urgent-ish"], "due": {"string": "tomorrow"}}),
    ));
    e.enqueue(Command::section_update(
        "6Xsec000000000005",
        json!({"name": "Fruit & veg"}),
    ));
    e.enqueue(Command::project_update(
        "6Xhome00000000001",
        json!({"name": "House", "color": "red"}),
    ));
    e.enqueue(Command::project_archive("6Xhire00000000001"));
    e.enqueue(Command::item_delete("6Xtask00000000034"));
    e.enqueue(Command::item_complete("6Xtask00000000051"));
    e.enqueue(Command::label_update(
        "6Xlabel0000000003",
        json!({"name": "blocked"}),
    ));
    // Before the sync the optimistic state already shows the changes.
    assert!(
        e.store()
            .project("6Xhire00000000001")
            .unwrap()
            .unwrap()
            .is_archived
    );
    assert!(
        e.task("6Xtask00000000035").is_none(),
        "sub-tasks are deleted too"
    );
    assert!(e.task("6Xtask00000000052").unwrap().checked);
    assert_eq!(e.task("6Xtask00000000013").unwrap().labels, vec!["blocked"]);
    e.engine.sync_once().await.unwrap();
    assert!(
        e.store().failed_commands().unwrap().is_empty(),
        "{:?}",
        e.store().failed_commands()
    );
    let t = e.task("6Xtask00000000007").unwrap();
    assert_eq!(t.labels, vec!["urgent-ish"]);
    assert_eq!(t.due.unwrap().date.to_string(), "2026-03-11");
    assert_eq!(
        e.store()
            .section("6Xsec000000000005")
            .unwrap()
            .unwrap()
            .name,
        "Fruit & veg"
    );
    assert_eq!(
        e.store()
            .project("6Xhome00000000001")
            .unwrap()
            .unwrap()
            .name,
        "House"
    );
    assert!(
        e.store()
            .filters()
            .unwrap()
            .iter()
            .any(|f| f.name == "Mine")
    );
    assert!(e.task("6Xtask00000000034").is_none());
    assert_eq!(
        e.server_item("6Xtask00000000013").unwrap()["labels"],
        json!(["blocked"])
    );
    assert!(e.server.state().get("items", "6Xtask00000000052").unwrap()["checked"] == true);
    assert!(
        !e.store()
            .projects_tree()
            .unwrap()
            .iter()
            .any(|p| p.project.id == "6Xhire00000000001")
    );
    // Uncomplete brings a task back.
    e.enqueue(Command::item_uncomplete("6Xtask00000000051"));
    e.engine.sync_once().await.unwrap();
    assert!(!e.task("6Xtask00000000051").unwrap().checked);
}

#[tokio::test]
async fn background_engine_emits_events() {
    let e = env().await;
    let dir = tempfile::tempdir().unwrap();
    let config = tick_core::EngineConfig {
        api_base: e.server.base_url(),
        token: e.server.token(),
        db_path: dir.path().join("bg.sqlite3"),
        foreground_interval: Duration::from_millis(200),
        background_interval: Duration::from_secs(60),
        clock: Some(fixed_clock()),
    };
    let (handle, events) = tick_core::Engine::spawn(config).unwrap();
    let mut saw_change = false;
    let mut saw_online = false;
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline && !(saw_change && saw_online) {
        match tokio::time::timeout(Duration::from_secs(5), events.recv()).await {
            Ok(Ok(tick_core::EngineEvent::Changed(c))) => saw_change |= c.full,
            Ok(Ok(tick_core::EngineEvent::Status(s))) => saw_online |= s.state == ConnState::Online,
            Ok(Ok(_)) => {}
            _ => break,
        }
    }
    assert!(saw_change && saw_online);
    // The UI thread's own connection sees the data (WAL, separate connection).
    let ui = Store::open(dir.path().join("bg.sqlite3")).unwrap();
    assert!(!ui.projects_tree().unwrap().is_empty());
    handle.sync_now();
    handle.shutdown();
}
