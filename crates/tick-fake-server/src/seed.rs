//! Seed data: an empty account, the "demo" account for screenshots and `run-fake`, a bulk
//! generator, and JSON fixtures.
//!
//! Dates in the demo seed are relative to the server's "now" (`TICK_NOW` when set).
//! Chinese copy uses ASCII punctuation with spaces between Chinese and Latin text or numbers.

use chrono::{Duration, NaiveDate};
use serde_json::{Value, json};

use crate::State;
use crate::order::nth_key;

pub const USER_ID: &str = "2671355";
pub const INBOX_ID: &str = "6Xinbox0000000001";

fn user(st: &State) -> Value {
    let offset = st.now().offset().local_minus_utc();
    let hours = offset / 3600;
    let minutes = (offset.abs() % 3600) / 60;
    json!({
        "id": USER_ID, "email": "sam@example.com", "full_name": "Sam Rivera",
        "inbox_project_id": INBOX_ID, "image_id": null, "lang": "en", "is_premium": true,
        "premium_status": "current_personal_plan", "start_day": 1, "next_week": 1,
        "start_page": "today", "date_format": 0, "time_format": 0, "days_off": [6, 7],
        "daily_goal": 5, "weekly_goal": 25, "karma": 1200, "karma_trend": "up",
        "auto_reminder": 30, "joined_at": "2024-01-15T10:00:00.000000Z",
        "token": "never-cache-this-token",
        "tz_info": {"timezone": if offset == 0 { "UTC".to_owned() } else { format!("Etc/GMT{:+}", -hours) },
                    "gmt_string": format!("{:+03}:{:02}", hours, minutes),
                    "hours": hours, "minutes": minutes, "is_dst": 0},
    })
}

fn settings() -> Value {
    json!({"reminder_push": true, "reminder_desktop": true, "reminder_email": false,
           "completed_sound_desktop": true, "completed_sound_mobile": true,
           "reset_recurring_subtasks": false})
}

fn project(id: &str, name: &str, color: &str, parent: Option<&str>, order: usize) -> Value {
    json!({
        "id": id, "name": name, "description": "", "workspace_id": null, "color": color,
        "icon": null, "parent_id": parent, "child_order": order, "order_key": nth_key(order),
        "is_collapsed": false, "shared": false, "can_assign_tasks": false, "is_deleted": false,
        "is_archived": false, "is_favorite": false, "is_frozen": false, "view_style": "list",
        "role": null, "folder_id": null, "created_at": "2025-01-10T09:00:00Z",
        "updated_at": "2025-01-10T09:00:00Z",
    })
}

/// Reset to an account with only a user and the Inbox.
pub fn empty(st: &mut State) {
    for r in crate::RESOURCES {
        st.map_mut(r).clear();
    }
    st.collaborator_states.clear();
    st.completed.clear();
    let u = user(st);
    st.set_user(u);
    st.set_user_settings(settings());
    let mut inbox = project(INBOX_ID, "Inbox", "charcoal", None, 0);
    inbox["inbox_project"] = json!(true);
    st.put("projects", inbox);
}

fn section(id: &str, project: &str, name: &str, order: usize) -> Value {
    json!({"id": id, "name": name, "description": null, "project_id": project,
           "section_order": order, "order_key": nth_key(order), "is_collapsed": false,
           "user_id": USER_ID, "is_deleted": false, "is_archived": false, "archived_at": null,
           "added_at": "2025-01-10T09:00:00Z", "updated_at": "2025-01-10T09:00:00Z"})
}

/// Builder for one task.
struct T<'a> {
    project: &'a str,
    section: Option<&'a str>,
    parent: Option<&'a str>,
    content: &'a str,
    description: &'a str,
    p: u8,
    due: Option<Value>,
    labels: &'a [&'a str],
    deadline: Option<i64>,
    responsible: Option<&'a str>,
}

impl<'a> T<'a> {
    fn new(project: &'a str, content: &'a str) -> Self {
        T {
            project,
            section: None,
            parent: None,
            content,
            description: "",
            p: 4,
            due: None,
            labels: &[],
            deadline: None,
            responsible: None,
        }
    }
    fn sec(mut self, s: &'a str) -> Self {
        self.section = Some(s);
        self
    }
    fn parent(mut self, p: &'a str) -> Self {
        self.parent = Some(p);
        self
    }
    fn desc(mut self, d: &'a str) -> Self {
        self.description = d;
        self
    }
    fn p(mut self, p: u8) -> Self {
        self.p = p;
        self
    }
    fn due(mut self, d: Value) -> Self {
        self.due = Some(d);
        self
    }
    fn labels(mut self, l: &'a [&'a str]) -> Self {
        self.labels = l;
        self
    }
    fn deadline(mut self, days: i64) -> Self {
        self.deadline = Some(days);
        self
    }
    fn assign(mut self, uid: &'a str) -> Self {
        self.responsible = Some(uid);
        self
    }
}

fn day(today: NaiveDate, offset: i64) -> String {
    (today + Duration::days(offset))
        .format("%Y-%m-%d")
        .to_string()
}

fn on(today: NaiveDate, offset: i64) -> Value {
    let date = day(today, offset);
    json!({"date": date, "timezone": null, "string": date, "lang": "en", "is_recurring": false})
}

fn at(today: NaiveDate, offset: i64, time: &str) -> Value {
    let date = format!("{}T{}:00", day(today, offset), time);
    json!({"date": date, "timezone": null, "string": format!("{} {}", day(today, offset), time),
           "lang": "en", "is_recurring": false})
}

fn every(today: NaiveDate, offset: i64, string: &str) -> Value {
    json!({"date": day(today, offset), "timezone": null, "string": string, "lang": "en",
           "is_recurring": true})
}

fn insert_task(st: &mut State, id: &str, t: T<'_>, order: usize, today: NaiveDate) {
    let now = st.now_utc_string();
    st.put(
        "items",
        json!({
            "id": id, "user_id": USER_ID, "project_id": t.project, "content": t.content,
            "description": t.description, "priority": 5 - i64::from(t.p), "due": t.due,
            "deadline": t.deadline.map(|d| json!({"date": day(today, d), "lang": "en"})),
            "parent_id": t.parent, "child_order": order, "order_key": nth_key(order),
            "section_id": t.section, "day_order": -1, "is_collapsed": false,
            "labels": t.labels, "added_by_uid": USER_ID, "assigned_by_uid": USER_ID,
            "responsible_uid": t.responsible, "checked": false, "is_deleted": false,
            "added_at": now, "updated_at": now, "completed_at": null, "duration": null,
            "note_count": 0,
        }),
    );
}

/// The demo account: projects (one shared, nested sub-projects), sections, about 60 tasks,
/// labels, filters, comments, reminders and a few completed tasks.
pub fn demo(st: &mut State) {
    empty(st);
    let today = st.now().date_naive();
    let maya = "2671362";
    let ken = "2671366";

    for (i, (id, name, color, parent)) in [
        ("6Xwork00000000001", "Work", "blue", None),
        (
            "6Xweb000000000001",
            "Website redesign",
            "teal",
            Some("6Xwork00000000001"),
        ),
        (
            "6Xhire00000000001",
            "Hiring",
            "violet",
            Some("6Xwork00000000001"),
        ),
        ("6Xpers00000000001", "Personal", "green", None),
        (
            "6Xhome00000000001",
            "Home",
            "orange",
            Some("6Xpers00000000001"),
        ),
        ("6Xgroc00000000001", "Groceries", "lime_green", None),
        ("6Xstudy0000000001", "学习计划", "red", None),
    ]
    .into_iter()
    .enumerate()
    {
        let mut p = project(id, name, color, parent, i + 1);
        if id == "6Xwork00000000001" {
            p["shared"] = json!(true);
            p["can_assign_tasks"] = json!(true);
            p["is_favorite"] = json!(true);
        }
        st.put("projects", p);
    }
    let work = "6Xwork00000000001";
    let web = "6Xweb000000000001";
    let hire = "6Xhire00000000001";
    let pers = "6Xpers00000000001";
    let home = "6Xhome00000000001";
    let groc = "6Xgroc00000000001";
    let study = "6Xstudy0000000001";

    for (id, name, email) in [
        (maya, "Maya Lopez", "maya@example.com"),
        (ken, "Ken Ito", "ken@example.com"),
        (USER_ID, "Sam Rivera", "sam@example.com"),
    ] {
        st.put(
            "collaborators",
            json!({"id": id, "email": email, "full_name": name, "timezone": "UTC",
                   "image_id": null}),
        );
        st.add_collaborator_state(work, id, "active");
    }

    for (i, (id, project, name)) in [
        ("6Xsec000000000001", work, "This week"),
        ("6Xsec000000000002", work, "Backlog"),
        ("6Xsec000000000003", web, "Design"),
        ("6Xsec000000000004", web, "Build"),
        ("6Xsec000000000005", groc, "Produce"),
        ("6Xsec000000000006", groc, "Dairy"),
        ("6Xsec000000000007", study, "Rust"),
        ("6Xsec000000000008", study, "日语"),
    ]
    .into_iter()
    .enumerate()
    {
        st.put("sections", section(id, project, name, i + 1));
    }

    for (i, (name, color)) in [
        ("errand", "orange"),
        ("deep-work", "blue"),
        ("waiting", "grey"),
        ("phone", "green"),
        ("home", "lime_green"),
        ("15min", "yellow"),
    ]
    .into_iter()
    .enumerate()
    {
        st.put(
            "labels",
            json!({"id": format!("6Xlabel{:010}", i + 1), "name": name, "color": color,
                   "item_order": i + 1, "order_key": nth_key(i + 1), "is_deleted": false,
                   "is_favorite": name == "deep-work"}),
        );
    }

    for (i, (name, query, color)) in [
        ("Urgent this week", "p1 & next 7 days", "red"),
        ("Waiting on others", "@waiting", "grey"),
        ("Work today", "##Work & (today | overdue)", "blue"),
        ("Assigned to me", "assigned to: me", "teal"),
        ("Home-ish labels", "@home*", "lime_green"),
    ]
    .into_iter()
    .enumerate()
    {
        st.put(
            "filters",
            json!({"id": format!("6Xfilter{:09}", i + 1), "name": name, "query": query,
                   "description": "", "color": color, "item_order": i + 1,
                   "order_key": nth_key(i + 1), "is_deleted": false, "is_favorite": i == 0,
                   "is_frozen": false}),
        );
    }

    let s1 = "6Xsec000000000001";
    let s2 = "6Xsec000000000002";
    let s3 = "6Xsec000000000003";
    let s4 = "6Xsec000000000004";
    let s5 = "6Xsec000000000005";
    let s6 = "6Xsec000000000006";
    let s7 = "6Xsec000000000007";
    let s8 = "6Xsec000000000008";
    let t = today;
    let tasks: Vec<(&str, T<'_>)> = vec![
        // Inbox
        ("6Xtask00000000001", T::new(INBOX_ID, "Call the dentist to move the appointment").p(2).due(on(t, 0)).labels(&["phone"])),
        ("6Xtask00000000002", T::new(INBOX_ID, "Renew passport").desc("Bring **two photos** and the old passport.\n\n- Form DS-82\n- Check fee on the [official site](https://travel.state.gov)").p(1).due(on(t, -2)).deadline(10)),
        ("6Xtask00000000003", T::new(INBOX_ID, "Back up the laptop").due(every(t, 0, "every sunday")).labels(&["home"])),
        ("6Xtask00000000004", T::new(INBOX_ID, "Read the article Ken shared about CRDTs").labels(&["15min"])),
        ("6Xtask00000000005", T::new(INBOX_ID, "Pay the electricity bill").p(2).due(on(t, 1)).labels(&["errand"])),
        ("6Xtask00000000006", T::new(INBOX_ID, "整理 2026 年度预算, 下周一前提交").p(2).due(on(t, 3)).desc("参考去年的表格, 重点看 *交通* 和 *房租* 两项.")),
        ("6Xtask00000000007", T::new(INBOX_ID, "Look into standing desks")),
        // Wraps to three lines and ends in an ellipsis in a narrow window (4.4).
        ("6Xtask00000000008", T::new(INBOX_ID, "Compare the three quotes for replacing the kitchen windows, ask the landlord whether the building insurance covers any of it, and book whichever installer can come before the first frost, then tell the neighbours about the noise").p(3).labels(&["home"])),
        // Work
        ("6Xtask00000000010", T::new(work, "Prepare the quarterly planning deck").sec(s1).p(1).due(at(t, 0, "14:00")).labels(&["deep-work"]).desc("Outline:\n1. Results\n2. Risks\n3. Asks").deadline(2).assign(USER_ID)),
        ("6Xtask00000000011", T::new(work, "Collect metrics from the dashboard").parent("6Xtask00000000010").sec(s1).due(on(t, 0)).assign(USER_ID)),
        ("6Xtask00000000012", T::new(work, "Draft the risks slide").parent("6Xtask00000000010").sec(s1).assign(maya)),
        ("6Xtask00000000013", T::new(work, "Review Maya's pull request").sec(s1).p(2).due(on(t, -1)).labels(&["waiting"]).assign(USER_ID)),
        ("6Xtask00000000014", T::new(work, "Weekly sync with the team").sec(s1).due(every(t, 1, "every wednesday"))),
        ("6Xtask00000000015", T::new(work, "Write the onboarding guide for new engineers").sec(s1).p(3).due(on(t, 4)).labels(&["deep-work"]).assign(ken)),
        ("6Xtask00000000016", T::new(work, "Answer the security questionnaire").sec(s1).p(1).due(on(t, 1)).deadline(3).assign(maya)),
        ("6Xtask00000000017", T::new(work, "Clean up stale feature flags").sec(s2).p(3)),
        ("6Xtask00000000018", T::new(work, "Evaluate a new CI provider").sec(s2).labels(&["deep-work"])),
        ("6Xtask00000000019", T::new(work, "Archive last year's project folders").sec(s2).labels(&["15min"])),
        ("6Xtask00000000020", T::new(work, "Submit expense report").p(2).due(on(t, 0)).labels(&["errand", "15min"])),
        ("6Xtask00000000021", T::new(work, "Book the offsite venue").due(on(t, 6)).labels(&["phone"]).assign(ken)),
        // Website redesign
        ("6Xtask00000000030", T::new(web, "Finalize the color palette").sec(s3).p(2).due(on(t, 0)).desc("Check contrast in **both** themes; target WCAG AA.")),
        ("6Xtask00000000031", T::new(web, "Design the pricing page").sec(s3).due(on(t, 2)).labels(&["deep-work"])),
        ("6Xtask00000000032", T::new(web, "Mobile navigation mockups").sec(s3).due(on(t, 5))),
        ("6Xtask00000000033", T::new(web, "Set up the component library").sec(s4).p(3).due(on(t, 7))),
        ("6Xtask00000000034", T::new(web, "Migrate the blog posts").sec(s4).desc("Use `scripts/migrate.sh`, then spot-check 10 posts.")),
        ("6Xtask00000000035", T::new(web, "Configure redirects").parent("6Xtask00000000034").sec(s4)),
        ("6Xtask00000000036", T::new(web, "Run Lighthouse on the staging site").sec(s4).due(on(t, 8)).labels(&["15min"])),
        // Hiring
        ("6Xtask00000000040", T::new(hire, "Review three backend candidates").p(1).due(on(t, -3)).labels(&["deep-work"])),
        ("6Xtask00000000041", T::new(hire, "Schedule interviews for Friday").p(2).due(on(t, 1)).labels(&["phone"])),
        ("6Xtask00000000042", T::new(hire, "Update the job description").due(on(t, 9))),
        ("6Xtask00000000043", T::new(hire, "Send the take-home exercise").labels(&["waiting"])),
        // Personal
        ("6Xtask00000000050", T::new(pers, "Morning run").due(every(t, 0, "every day")).labels(&["15min"])),
        ("6Xtask00000000051", T::new(pers, "Plan the weekend trip").p(3).due(on(t, 3))),
        ("6Xtask00000000052", T::new(pers, "Pick hotels").parent("6Xtask00000000051").due(on(t, 2))),
        ("6Xtask00000000053", T::new(pers, "Check train times").parent("6Xtask00000000051")),
        ("6Xtask00000000054", T::new(pers, "Buy a birthday present for Mom").p(1).due(on(t, 2)).labels(&["errand"]).deadline(4)),
        ("6Xtask00000000055", T::new(pers, "Call grandma").due(every(t, -1, "every saturday")).labels(&["phone"])),
        ("6Xtask00000000056", T::new(pers, "Read 30 pages").due(every(t, 0, "every day"))),
        ("6Xtask00000000057", T::new(pers, "Sign up for the pottery class").due(on(t, 12))),
        // Home
        ("6Xtask00000000060", T::new(home, "Fix the leaking kitchen tap").p(2).due(on(t, -1)).labels(&["home"])),
        ("6Xtask00000000061", T::new(home, "Water the plants").due(every(t, 0, "every monday")).labels(&["home", "15min"])),
        ("6Xtask00000000062", T::new(home, "Replace the hallway light bulb").labels(&["home", "errand"])),
        ("6Xtask00000000063", T::new(home, "Deep clean the oven").due(on(t, 5)).labels(&["home"])),
        ("6Xtask00000000064", T::new(home, "Sort the recycling").due(every(t, 1, "every wednesday")).labels(&["home"])),
        // Groceries
        ("6Xtask00000000070", T::new(groc, "Bananas").sec(s5).labels(&["errand"])),
        ("6Xtask00000000071", T::new(groc, "Spinach").sec(s5)),
        ("6Xtask00000000072", T::new(groc, "Avocados").sec(s5)),
        ("6Xtask00000000073", T::new(groc, "Oat milk").sec(s6).p(3)),
        ("6Xtask00000000074", T::new(groc, "Greek yogurt").sec(s6)),
        ("6Xtask00000000075", T::new(groc, "Coffee beans").p(2).due(on(t, 0)).labels(&["errand"])),
        ("6Xtask00000000076", T::new(groc, "买 2 斤鸡蛋").sec(s6)),
        // 学习计划
        ("6Xtask00000000080", T::new(study, "复习 Rust 所有权章节").sec(s7).p(2).due(on(t, 0)).labels(&["deep-work"]).desc("重点: **借用检查**, 生命周期标注, `Rc` 和 `Arc` 的区别.")),
        ("6Xtask00000000081", T::new(study, "完成 Rustlings 前 20 题").sec(s7).due(on(t, 2))),
        ("6Xtask00000000082", T::new(study, "写一篇 async Rust 读书笔记").sec(s7).p(3).due(on(t, 6))),
        ("6Xtask00000000083", T::new(study, "背 20 个日语单词").sec(s8).due(every(t, 0, "every day")).labels(&["15min"])),
        ("6Xtask00000000084", T::new(study, "预约 JLPT N3 考试").sec(s8).p(1).due(on(t, -1)).deadline(5)),
        ("6Xtask00000000085", T::new(study, "看一集日剧, 不开字幕").sec(s8).due(on(t, 1))),
        ("6Xtask00000000086", T::new(study, "整理错题本").sec(s8)),
        ("6Xtask00000000087", T::new(study, "下载 Anki 卡组").parent("6Xtask00000000083").sec(s8)),
    ];
    for (i, (id, task)) in tasks.into_iter().enumerate() {
        insert_task(st, id, task, i + 1, today);
    }

    let now = st.now_utc_string();
    for (i, (item, who, content)) in [
        (
            "6Xtask00000000010",
            maya,
            "I added last quarter's numbers to the shared folder.",
        ),
        (
            "6Xtask00000000010",
            USER_ID,
            "Thanks! I'll pull them into slide 3.",
        ),
        (
            "6Xtask00000000013",
            ken,
            "Left two comments on the error handling.",
        ),
        (
            "6Xtask00000000030",
            USER_ID,
            "Primary: #3584e4, accent stays system default.",
        ),
        (
            "6Xtask00000000002",
            USER_ID,
            "Photo booth on 5th street closes at 6 pm.",
        ),
        ("6Xtask00000000080", USER_ID, "第 4 章的例子要自己再敲一遍."),
    ]
    .into_iter()
    .enumerate()
    {
        st.put(
            "notes",
            json!({"id": format!("6Xnote{:011}", i + 1), "posted_uid": who, "item_id": item,
                   "content": content, "file_attachment": null, "uids_to_notify": null,
                   "is_deleted": false, "posted_at": now, "reactions": null}),
        );
    }
    st.put(
        "notes",
        json!({"id": "6Xnote00000000099", "posted_uid": maya, "item_id": "6Xtask00000000030",
               "content": "Palette draft attached.",
               "file_attachment": {"file_name": "palette.pdf", "file_type": "application/pdf",
                                   "file_size": 48213,
                                   "file_url": "https://example.com/files/palette.pdf",
                                   "upload_state": "completed"},
               "uids_to_notify": null, "is_deleted": false, "posted_at": now,
               "reactions": {"👍": [USER_ID]}}),
    );

    st.put(
        "reminders",
        json!({"id": "6Xrem000000000001", "notify_uid": USER_ID, "item_id": "6Xtask00000000010",
               "type": "relative", "due": null, "minute_offset": 30, "is_deleted": false,
               "is_urgent": false}),
    );
    st.put(
        "reminders",
        json!({"id": "6Xrem000000000002", "notify_uid": USER_ID, "item_id": "6Xtask00000000001",
               "type": "absolute",
               "due": {"date": format!("{}T09:30:00", day(today, 0)), "timezone": null,
                       "string": "today at 9:30", "lang": "en", "is_recurring": false},
               "minute_offset": null, "is_deleted": false, "is_urgent": false}),
    );

    // A few completed tasks (in the archive, not in the active data).
    for (i, (project, content, days_ago)) in [
        (work, "Send the meeting notes", 0),
        (pers, "Book the flight", 1),
        (groc, "Rice", 1),
        (web, "Pick the font pairing", 3),
        (study, "读完 The Rust Book 第 3 章", 4),
        (home, "Change the bed sheets", 6),
    ]
    .into_iter()
    .enumerate()
    {
        let id = format!("6Xdone{:011}", i + 1);
        let at = (st.now() - Duration::days(days_ago) - Duration::hours(i as i64 + 1))
            .with_timezone(&chrono::Utc)
            .format("%Y-%m-%dT%H:%M:%S%.6fZ")
            .to_string();
        let mut v = json!({
            "id": id, "user_id": USER_ID, "project_id": project, "content": content,
            "description": "", "priority": 1, "due": null, "deadline": null, "parent_id": null,
            "child_order": 100 + i, "order_key": nth_key(100 + i), "section_id": null,
            "day_order": -1, "is_collapsed": false, "labels": [], "added_by_uid": USER_ID,
            "assigned_by_uid": USER_ID, "responsible_uid": null, "checked": true,
            "is_deleted": false, "added_at": at, "updated_at": at, "completed_at": at,
            "duration": null, "note_count": 0,
        });
        st.put("items", v.clone());
        v["completed_by_uid"] = json!(USER_ID);
        st.add_completed(v);
    }
}

/// An account with `n` active tasks spread over 10 projects with sections, labels, due
/// dates (overdue, today, upcoming, none) and some sub-tasks. Used for the startup test.
pub fn bulk(st: &mut State, n: usize) {
    empty(st);
    let today = st.now().date_naive();
    let mut projects = vec![INBOX_ID.to_owned()];
    for p in 0..10 {
        let id = format!("6Xbulkproj{:07}", p);
        st.put(
            "projects",
            project(&id, &format!("Project {}", p + 1), "blue", None, p + 1),
        );
        for s in 0..3 {
            let sid = format!("6Xbulksec{:04}{:04}", p, s);
            st.put(
                "sections",
                section(&sid, &id, &format!("Section {}", s + 1), s + 1),
            );
        }
        projects.push(id);
    }
    for (i, name) in ["alpha", "beta", "gamma", "delta"].iter().enumerate() {
        st.put(
            "labels",
            json!({"id": format!("6Xbulklabel{:06}", i), "name": name, "color": "grey",
                   "item_order": i + 1, "order_key": nth_key(i + 1), "is_deleted": false,
                   "is_favorite": false}),
        );
    }
    let placement = |j: usize| -> (String, Option<String>) {
        let p_index = j % projects.len();
        let section = if p_index == 0 || j.is_multiple_of(4) {
            None
        } else {
            Some(format!("6Xbulksec{:04}{:04}", p_index - 1, j % 3))
        };
        (projects[p_index].clone(), section)
    };
    for i in 0..n {
        // Every tenth task is a sub-task of the previous one (same project and section).
        let (parent, (project, section)) = if i % 10 == 9 {
            (Some(format!("6Xbulktask{:07}", i - 1)), placement(i - 1))
        } else {
            (None, placement(i))
        };
        let due = match i % 7 {
            0 => Value::Null,
            1 => on(today, -1 - (i % 5) as i64),
            2 | 3 => on(today, 0),
            _ => on(today, (i % 14) as i64),
        };
        let labels: Vec<&str> = ["alpha", "beta", "gamma", "delta"]
            .iter()
            .copied()
            .filter(|_| i % 3 == 0)
            .take(1 + i % 2)
            .collect();
        let now = st.now_utc_string();
        st.put(
            "items",
            json!({
                "id": format!("6Xbulktask{:07}", i), "user_id": USER_ID, "project_id": project,
                "content": format!("Task number {} with a reasonably long title", i + 1),
                "description": if i % 5 == 0 { "Some **notes** for this task." } else { "" },
                "priority": 1 + (i % 4), "due": due, "deadline": null, "parent_id": parent,
                "child_order": i + 1, "order_key": nth_key(i + 1), "section_id": section,
                "day_order": -1, "is_collapsed": false, "labels": labels,
                "added_by_uid": USER_ID, "assigned_by_uid": USER_ID, "responsible_uid": null,
                "checked": false, "is_deleted": false, "added_at": now, "updated_at": now,
                "completed_at": null, "duration": null, "note_count": 0,
            }),
        );
    }
}

pub fn apply_preset(st: &mut State, preset: &str, count: usize) -> Result<(), String> {
    match preset {
        "empty" => empty(st),
        "demo" => demo(st),
        "bulk" => bulk(st, count),
        other => {
            if let Some(n) = other.strip_prefix("bulk:") {
                bulk(st, n.parse().map_err(|_| format!("bad bulk count {n:?}"))?);
            } else {
                return Err(format!("unknown preset {other:?}"));
            }
        }
    }
    Ok(())
}

fn defaults(r: &str) -> Value {
    match r {
        "items" => json!({"description": "", "priority": 1, "due": null, "deadline": null,
                          "parent_id": null, "section_id": null, "child_order": 1,
                          "day_order": -1, "is_collapsed": false, "labels": [],
                          "responsible_uid": null, "checked": false, "is_deleted": false,
                          "completed_at": null, "duration": null, "user_id": USER_ID}),
        "projects" => json!({"color": "charcoal", "parent_id": null, "child_order": 1,
                             "is_collapsed": false, "shared": false, "is_deleted": false,
                             "is_archived": false, "is_favorite": false, "view_style": "list"}),
        "sections" => json!({"section_order": 1, "is_collapsed": false, "is_deleted": false,
                             "is_archived": false, "description": null}),
        "labels" | "filters" => json!({"color": "charcoal", "item_order": 1,
                                       "is_deleted": false, "is_favorite": false}),
        "notes" => json!({"is_deleted": false, "file_attachment": null, "posted_uid": USER_ID}),
        "reminders" => json!({"is_deleted": false, "type": "absolute", "notify_uid": USER_ID}),
        _ => json!({}),
    }
}

/// Seed from a fixture: `{"token"?, "now"?, "user"?, "user_settings"?, "projects": [...],
/// "items": [...], ..., "collaborator_states": [...], "completed": [...], "preset"?}`.
/// Objects get documented defaults for missing fields. The Inbox from the empty account is
/// kept unless the fixture's user names another inbox.
pub fn apply_fixture(st: &mut State, f: &Value) -> Result<(), String> {
    let Some(obj) = f.as_object() else {
        return Err("fixture must be a JSON object".into());
    };
    if let Some(now) = obj.get("now").and_then(Value::as_str) {
        st.now =
            Some(chrono::DateTime::parse_from_rfc3339(now).map_err(|e| format!("bad now: {e}"))?);
    }
    match obj.get("preset").and_then(Value::as_str) {
        Some(p) => apply_preset(
            st,
            p,
            obj.get("count").and_then(Value::as_u64).unwrap_or(1000) as usize,
        )?,
        None => empty(st),
    }
    if let Some(t) = obj.get("token").and_then(Value::as_str) {
        st.token = t.to_owned();
    }
    if let Some(u) = obj.get("user") {
        let mut base = st.user.clone();
        merge_into(&mut base, u);
        st.set_user(base);
    }
    if let Some(s) = obj.get("user_settings") {
        st.set_user_settings(s.clone());
    }
    for r in crate::RESOURCES {
        let Some(list) = obj.get(*r).and_then(Value::as_array) else {
            continue;
        };
        for v in list {
            let mut full = defaults(r);
            merge_into(&mut full, v);
            if full["id"].as_str().is_none() {
                let id = st.new_id();
                full["id"] = json!(id);
            }
            if *r == "items" && full["project_id"].as_str().is_none() {
                full["project_id"] = json!(st.inbox_id());
            }
            if full.get("order_key").is_none() {
                let o = full["child_order"]
                    .as_u64()
                    .or(full["section_order"].as_u64())
                    .or(full["item_order"].as_u64())
                    .unwrap_or(1) as usize;
                full["order_key"] = json!(nth_key(o));
            }
            st.put(r, full);
        }
    }
    if let Some(list) = obj.get("collaborator_states").and_then(Value::as_array) {
        for v in list {
            st.add_collaborator_state(
                v["project_id"].as_str().unwrap_or(""),
                v["user_id"].as_str().unwrap_or(""),
                v["state"].as_str().unwrap_or("active"),
            );
        }
    }
    if let Some(list) = obj.get("completed").and_then(Value::as_array) {
        for v in list {
            st.add_completed(v.clone());
        }
    }
    Ok(())
}

fn merge_into(base: &mut Value, over: &Value) {
    match (base.as_object_mut(), over.as_object()) {
        (Some(b), Some(o)) => {
            for (k, v) in o {
                b.insert(k.clone(), v.clone());
            }
        }
        _ => *base = over.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_has_expected_shape() {
        let mut st = State::new("t");
        st.now = chrono::DateTime::parse_from_rfc3339("2026-03-10T09:00:00+08:00").ok();
        demo(&mut st);
        let active = st.active_items().len();
        assert!((55..=70).contains(&active), "{active} active tasks");
        assert!(st.live("projects").count() >= 6);
        assert_eq!(st.completed.len(), 6);
        let today = "2026-03-10";
        assert!(st.active_items().iter().any(|v| v["due"]["date"] == today));
        assert!(
            st.active_items()
                .iter()
                .any(|v| v["due"]["date"].as_str().is_some_and(|d| d < today))
        );
        assert!(
            st.active_items()
                .iter()
                .any(|v| v["due"]["is_recurring"] == true)
        );
        assert!(
            st.active_items()
                .iter()
                .any(|v| v["content"].as_str().unwrap().contains("Rust"))
        );
    }

    #[test]
    fn bulk_counts() {
        let mut st = State::new("t");
        bulk(&mut st, 1000);
        assert_eq!(st.active_items().len(), 1000);
        // Every parent exists and shares the project.
        for v in st.active_items() {
            if let Some(p) = v["parent_id"].as_str() {
                let parent = st.get("items", p).unwrap();
                assert_eq!(parent["project_id"], v["project_id"]);
                assert_eq!(parent["section_id"], v["section_id"]);
            }
        }
    }
}
