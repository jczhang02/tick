//! Reminder notifications (`SPEC.md` Data 5): time-based reminders fire desktop
//! notifications with "Complete" and "Open" buttons, also while the window is hidden.
//!
//! When reminders fire is decided in tick-core (`tick_core::reminders`): each firing shows
//! once, across syncs and restarts, and a missed one is still shown at the next start when
//! it is at most an hour late. The window checks after every relevant change and on a timer
//! armed for the next reminder (at most a minute ahead, so a changed clock is noticed).

use std::time::Duration;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};
use tick_core::{Command, Kind};

use crate::format::{self, DateSettings};
use crate::i18n::gettext;
use crate::window::TickWindow;

/// Longest wait between two checks.
const MAX_WAIT: Duration = Duration::from_secs(60);

fn notification_id(reminder_id: &str) -> String {
    format!("reminder-{reminder_id}")
}

impl TickWindow {
    /// Changes that can add, move or remove a reminder.
    pub(crate) fn reminders_affected(kinds: &[Kind]) -> bool {
        kinds
            .iter()
            .any(|k| matches!(k, Kind::Reminder | Kind::Item | Kind::User))
    }

    /// Send the reminders that are due, then arm the timer for the next one.
    pub(crate) fn check_reminders(&self) {
        let imp = self.imp();
        if let Some(id) = imp.reminder_timer.take() {
            id.remove();
        }
        if imp.logging_out.get() {
            return;
        }
        let Some(app) = self.application() else {
            return;
        };
        let (schedule, enabled, settings) = {
            let store = self.store();
            let now = store.now();
            let schedule = match store.reminder_schedule(&now) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("tick: reading reminders: {e}");
                    return;
                }
            };
            let enabled = store.desktop_reminders_enabled().unwrap_or(true);
            let user = store.user().ok().flatten();
            (schedule, enabled, DateSettings::from_user(user.as_ref()))
        };
        if enabled && !schedule.due.is_empty() {
            for r in &schedule.due {
                self.send_reminder(&app, r, settings);
            }
            if let Err(e) = self.store().mark_reminders_fired(&schedule.due) {
                eprintln!("tick: remembering sent reminders: {e}");
            }
        }
        let now = self.store().now().utc();
        let wait = schedule
            .next
            .and_then(|n| (n - now).to_std().ok())
            .map_or(MAX_WAIT, |d| d.min(MAX_WAIT))
            .max(Duration::from_secs(1));
        let weak = self.downgrade();
        let id = glib::timeout_add_local_once(wait, move || {
            if let Some(w) = weak.upgrade() {
                w.imp().reminder_timer.replace(None);
                w.check_reminders();
            }
        });
        imp.reminder_timer.replace(Some(id));
    }

    fn send_reminder(
        &self,
        app: &gtk::Application,
        r: &tick_core::reminders::DueReminder,
        settings: DateSettings,
    ) {
        let task = &r.task;
        let now = self.store().now();
        let mut lines = Vec::new();
        if let Some(d) = task.due
            && let Some(text) = format::due(d.date, d.time, now.today(), settings, false)
        {
            lines.push(text);
        }
        let project = self
            .store()
            .project(&task.project_id)
            .ok()
            .flatten()
            .map(|p| {
                if p.inbox_project {
                    gettext("Inbox")
                } else {
                    p.name
                }
            });
        if let Some(p) = project {
            lines.push(p);
        }
        let n = gio::Notification::new(&task.content);
        n.set_body(Some(&lines.join("\n")));
        n.set_priority(if r.urgent {
            gio::NotificationPriority::High
        } else {
            gio::NotificationPriority::Normal
        });
        let target = task.id.to_variant();
        n.set_default_action_and_target_value("app.reminder-open", Some(&target));
        n.add_button_with_target_value(
            &gettext("Complete"),
            "app.reminder-complete",
            Some(&target),
        );
        n.add_button_with_target_value(&gettext("Open"), "app.reminder-open", Some(&target));
        let id = notification_id(&r.reminder_id);
        app.send_notification(Some(&id), &n);
        self.imp()
            .sent_reminders
            .borrow_mut()
            .push((id, task.id.clone()));
        if crate::test_mode::enabled() {
            eprintln!("TICK_REMINDER_SENT={}", r.key);
        }
    }

    /// Take back the notifications of `task` (`None`: all of them).
    fn withdraw_for(&self, task: Option<&str>) {
        let Some(app) = self.application() else {
            return;
        };
        self.imp().sent_reminders.borrow_mut().retain(|(id, t)| {
            let hit = task.is_none_or(|x| x == t);
            if hit {
                app.withdraw_notification(id);
            }
            !hit
        });
    }

    /// Log out: no reminder stays on screen or armed.
    pub(crate) fn withdraw_reminders(&self) {
        if let Some(id) = self.imp().reminder_timer.take() {
            id.remove();
        }
        self.withdraw_for(None);
    }

    /// "Complete" on a notification: queue the close at once (no Undo toast: the window may
    /// be hidden) and sync.
    pub(crate) fn complete_from_notification(&self, id: &str) {
        let id = self.live_id(id);
        let open = self
            .store()
            .task(&id)
            .ok()
            .flatten()
            .is_some_and(|t| !t.checked);
        self.withdraw_for(Some(&id));
        if open {
            self.enqueue(vec![Command::item_close(&id)]);
        }
    }

    /// "Open" (or a click on the notification): the window with the task's details.
    pub(crate) fn open_from_notification(&self, id: &str) {
        let id = self.live_id(id);
        self.withdraw_for(Some(&id));
        self.present();
        if self.store().task(&id).ok().flatten().is_some() {
            self.open_task(&id, true);
        }
    }
}

/// `app.reminder-complete(s)` and `app.reminder-open(s)`, the notification buttons.
pub fn install_actions(app: &adw::Application) {
    let window = |app: &adw::Application| {
        app.windows()
            .into_iter()
            .find_map(|w| w.downcast::<TickWindow>().ok())
    };
    let complete = gio::ActionEntry::builder("reminder-complete")
        .parameter_type(Some(glib::VariantTy::STRING))
        .activate(move |app: &adw::Application, _, p| {
            if let (Some(w), Some(id)) = (window(app), p.and_then(|p| p.get::<String>())) {
                w.complete_from_notification(&id);
            }
        })
        .build();
    let open = gio::ActionEntry::builder("reminder-open")
        .parameter_type(Some(glib::VariantTy::STRING))
        .activate(move |app: &adw::Application, _, p| {
            if let (Some(w), Some(id)) = (window(app), p.and_then(|p| p.get::<String>())) {
                w.open_from_notification(&id);
            }
        })
        .build();
    app.add_action_entries([complete, open]);
}
