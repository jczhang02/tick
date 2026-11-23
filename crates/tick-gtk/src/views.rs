//! The Search view (`DESIGN.md` 4.15) and the Completed history (4.21): what the window does
//! for them besides building rows (`rows.rs`).

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;
use tick_core::Command;

use crate::i18n::gettext;
use crate::row_object::TickRow;
use crate::rows::{CompletedState, Row, ViewId, completed_default_since};
use crate::window::TickWindow;

/// `completed_at`-style stamp of an RFC 3339 instant (cached rows compare as strings).
fn completed_stamp(rfc3339: &str) -> Option<String> {
    let t = chrono::DateTime::parse_from_rfc3339(rfc3339).ok()?;
    Some(
        t.with_timezone(&chrono::Utc)
            .format("%Y-%m-%dT%H:%M:%S%.6fZ")
            .to_string(),
    )
}

impl TickWindow {
    // ---- search (4.15) ----

    /// `/` and the sidebar row: open Search with its entry focused.
    pub(crate) fn open_search(&self) {
        self.imp().focus_search.set(true);
        if self.current_view() == ViewId::Search {
            self.focus_search_entry();
        } else {
            self.open_view(ViewId::Search, true);
        }
    }

    /// The search entry, if its row is built.
    fn search_entry(&self) -> Option<gtk::SearchEntry> {
        let list = self.imp().task_list.get();
        let mut child = list.first_child();
        while let Some(c) = child {
            let header = c
                .first_child()
                .and_downcast::<adw::Bin>()
                .and_then(|b| b.child())
                .filter(|w| w.widget_name() == "search-header");
            if let Some(h) = header {
                return h.first_child().and_downcast::<gtk::SearchEntry>();
            }
            child = c.next_sibling();
        }
        None
    }

    pub(crate) fn focus_search_entry(&self) {
        self.imp()
            .task_list
            .scroll_to(0, gtk::ListScrollFlags::NONE, None);
        match self.search_entry() {
            Some(e) => {
                self.imp().focus_search.set(false);
                e.grab_focus();
                e.set_position(-1);
            }
            // Built later: it takes focus when it maps.
            None => self.imp().focus_search.set(true),
        }
    }

    pub(crate) fn search_changed(&self, text: &str) {
        {
            let mut ui = self.imp().ui.borrow_mut();
            if ui.search_query == text {
                return;
            }
            ui.search_query = text.to_owned();
        }
        if self.current_view() == ViewId::Search {
            self.refresh_rows();
        }
    }

    /// Enter in the entry: open the first result in the detail pane.
    pub(crate) fn search_activate(&self, text: &str) {
        self.search_changed(text);
        let first = self.imp().rows.get().and_then(|store| {
            (0..store.n_items())
                .filter_map(|i| store.item(i).and_downcast::<TickRow>())
                .find_map(|r| r.task_id())
        });
        if let Some(id) = first {
            self.open_task(&id, false);
        }
    }

    /// Down in the entry: the first result takes keyboard focus.
    pub(crate) fn search_down(&self) {
        self.focus_first_task_now();
    }

    /// Escape in the empty entry: back to the view before Search.
    pub(crate) fn search_escape(&self) {
        let previous = self
            .imp()
            .previous_view
            .borrow()
            .clone()
            .unwrap_or(ViewId::Today);
        self.open_view(previous, true);
        self.focus_first_task_now();
    }

    // ---- completed history (4.21) ----

    /// Opening the Completed view: show the cache, ask the server for the first page.
    pub(crate) fn start_completed(&self) {
        let imp = self.imp();
        let online = !imp.ui.borrow().offline;
        let engine = imp.engine.borrow().clone();
        imp.ui.borrow_mut().completed = CompletedState {
            refreshing: online && engine.is_some(),
            ..CompletedState::default()
        };
        if online && let Some(e) = engine {
            e.load_completed(true);
        }
        self.refresh_rows();
    }

    pub(crate) fn load_more_completed(&self) {
        let imp = self.imp();
        let engine = imp.engine.borrow().clone();
        let Some(e) = engine else { return };
        {
            let mut ui = imp.ui.borrow_mut();
            if ui.offline || ui.completed.loading_more || ui.completed.refreshing {
                return;
            }
            ui.completed.loading_more = true;
        }
        e.load_completed(false);
        self.refresh_rows();
    }

    /// A page arrived (`EngineEvent::CompletedLoaded`).
    pub(crate) fn completed_loaded(&self, has_more: bool, since: &str) {
        let now = self.store().now();
        let default = completed_default_since(&now);
        {
            let mut ui = self.imp().ui.borrow_mut();
            let st = &mut ui.completed;
            st.refreshing = false;
            st.loading_more = false;
            st.has_more = Some(has_more);
            // The first window is "Last 30 days"; an older one moves the start back.
            if let Some(s) = completed_stamp(since)
                && s < default
                && st.since.as_ref().is_none_or(|old| s < *old)
                && default_is_later(&s, &default)
            {
                st.since = Some(s);
            }
        }
        if self.current_view() == ViewId::Completed {
            self.refresh_rows();
        }
    }

    /// A history request failed (`EngineEvent::Error`).
    pub(crate) fn completed_failed(&self, offline: bool) {
        let more = {
            let mut ui = self.imp().ui.borrow_mut();
            let more = ui.completed.loading_more;
            ui.completed.refreshing = false;
            ui.completed.loading_more = false;
            more
        };
        if self.current_view() != ViewId::Completed {
            return;
        }
        self.refresh_rows();
        if offline {
            return;
        }
        let toast = adw::Toast::builder()
            .title(gettext("Couldn't load completed tasks"))
            .button_label(gettext("Retry"))
            .timeout(8)
            .build();
        let weak = self.downgrade();
        toast.connect_button_clicked(move |_| {
            let Some(w) = weak.upgrade() else { return };
            if more {
                w.load_more_completed();
            } else {
                w.start_completed();
            }
        });
        self.show_toast(&toast);
    }

    /// Whether the open view's rows are read-only (the history).
    pub(crate) fn view_is_read_only(&self) -> bool {
        self.current_view() == ViewId::Completed
    }

    /// Reopen a completed task from the history (detail pane ring, row menu).
    pub(crate) fn reopen_completed(&self, id: &str) {
        // Only in the history cache, or still in the active data as completed (completed
        // on this computer).
        let completed = {
            let store = self.store();
            match store.task(id).ok().flatten() {
                Some(t) => t.checked,
                None => store.completed_task(id).ok().flatten().is_some(),
            }
        };
        if completed {
            self.enqueue(vec![Command::item_uncomplete(id)]);
            let toast = adw::Toast::builder()
                .title(gettext("Task reopened"))
                .timeout(5)
                .build();
            self.show_toast(&toast);
        }
    }

    /// Reopen the selected history row (its context menu).
    pub(crate) fn reopen_selected(&self) {
        let selected = self
            .imp()
            .selection
            .get()
            .and_then(|s| s.selected_item())
            .and_downcast::<TickRow>();
        let Some(row) = selected else { return };
        let id = match &*row.row() {
            Row::Task(t) if t.read_only => t.id.clone(),
            _ => return,
        };
        self.reopen_completed(&id);
    }

    /// After the first paint the engine may start late: a Completed view opened from the
    /// cache asks for its first page then.
    pub(crate) fn completed_engine_started(&self) {
        if self.current_view() == ViewId::Completed {
            let weak = self.downgrade();
            glib::idle_add_local_once(move || {
                if let Some(w) = weak.upgrade() {
                    w.start_completed();
                }
            });
        }
    }
}

/// `since` starts more than a day before the default 30-day window (a "Load More" window,
/// not the first one written with another precision).
fn default_is_later(since: &str, default: &str) -> bool {
    let (Ok(a), Ok(b)) = (
        chrono::DateTime::parse_from_rfc3339(since),
        chrono::DateTime::parse_from_rfc3339(default),
    ) else {
        return false;
    };
    b - a > chrono::Duration::days(1)
}
