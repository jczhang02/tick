//! Preferences (`DESIGN.md` 4.17), the keyboard shortcuts dialog (4.18) and logging out.

use std::rc::Rc;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};

use crate::i18n::gettext;
use crate::portal::{self, PortalError};
use crate::window::TickWindow;

/// `color-scheme` setting values and their `AdwStyleManager` schemes (7.1: "Follow System"
/// is `DEFAULT`, never `PREFER_LIGHT`).
pub fn scheme_of(value: Option<&str>) -> adw::ColorScheme {
    match value {
        Some("light") => adw::ColorScheme::ForceLight,
        Some("dark") => adw::ColorScheme::ForceDark,
        _ => adw::ColorScheme::Default,
    }
}

impl TickWindow {
    /// Ctrl+, and the main menu.
    pub(crate) fn show_preferences(&self) {
        if self.visible_dialog().is_some() {
            return;
        }
        let dialog = adw::PreferencesDialog::builder()
            .title(gettext("Preferences"))
            .search_enabled(false)
            .build();
        let page = adw::PreferencesPage::new();

        // Appearance: three radio rows (4.17).
        let appearance = adw::PreferencesGroup::builder()
            .title(gettext("Appearance"))
            .build();
        let saved = self.setting("color-scheme");
        let mut first: Option<gtk::CheckButton> = None;
        for (value, title) in [
            ("system", gettext("Follow System")),
            ("light", gettext("Light")),
            ("dark", gettext("Dark")),
        ] {
            let check = gtk::CheckButton::builder()
                .valign(gtk::Align::Center)
                .active(saved.as_deref().unwrap_or("system") == value)
                .css_classes(["tick-row-radio"])
                .build();
            check.update_property(&[gtk::accessible::Property::Label(&title)]);
            if let Some(f) = &first {
                check.set_group(Some(f));
            } else {
                first = Some(check.clone());
            }
            let row = adw::ActionRow::builder()
                .title(&title)
                .activatable_widget(&check)
                .build();
            row.add_prefix(&check);
            let weak = self.downgrade();
            check.connect_toggled(move |c| {
                if c.is_active()
                    && let Some(w) = weak.upgrade()
                {
                    w.set_color_scheme(value);
                }
            });
            appearance.add(&row);
        }
        page.add(&appearance);

        // Background: Run at Login.
        let background = adw::PreferencesGroup::builder()
            .title(gettext("Background"))
            .build();
        let run = adw::SwitchRow::builder()
            .title(gettext("Run at Login"))
            .subtitle(gettext(
                "Keep syncing and sending reminders after the window is closed.",
            ))
            .active(self.setting("run-at-login").as_deref() == Some("true"))
            .build();
        let weak = self.downgrade();
        let dialog_weak = dialog.downgrade();
        let handler: Rc<std::cell::Cell<Option<glib::SignalHandlerId>>> = Rc::default();
        let h = handler.clone();
        let id = run.connect_active_notify(move |row| {
            let (Some(w), Some(d)) = (weak.upgrade(), dialog_weak.upgrade()) else {
                return;
            };
            w.request_run_at_login(row, &d, h.clone());
        });
        handler.set(Some(id));
        background.add(&run);
        page.add(&background);

        // Account: Log Out (plain style, 4.17).
        let account = adw::PreferencesGroup::builder()
            .title(gettext("Account"))
            .build();
        let log_out = adw::ButtonRow::builder().title(gettext("Log Out")).build();
        log_out.set_action_name(Some("app.log-out"));
        account.add(&log_out);
        page.add(&account);

        dialog.add(&page);
        dialog.connect_map(|d| {
            let d = d.clone();
            glib::idle_add_local_once(move || name_lists(d.upcast_ref(), None));
        });
        dialog.present(Some(self));
    }

    /// Apply and remember a theme choice (`system`, `light`, `dark`) at once.
    pub(crate) fn set_color_scheme(&self, value: &str) {
        adw::StyleManager::default().set_color_scheme(scheme_of(Some(value)));
        self.save_setting("color-scheme", value);
        if crate::test_mode::enabled() {
            eprintln!("TICK_COLOR_SCHEME_SET={value}");
        }
    }

    /// The Run at Login switch moved: ask the Background portal, and put the switch back
    /// (with a toast saying why) when it says no or is missing.
    fn request_run_at_login(
        &self,
        row: &adw::SwitchRow,
        dialog: &adw::PreferencesDialog,
        handler: Rc<std::cell::Cell<Option<glib::SignalHandlerId>>>,
    ) {
        let want = row.is_active();
        let Some(app) = self.application() else {
            return;
        };
        row.set_sensitive(false);
        let weak = self.downgrade();
        let row_weak = row.downgrade();
        let dialog_weak = dialog.downgrade();
        portal::request_background(
            &app,
            want,
            &gettext("Keep syncing and sending reminders after the window is closed."),
            move |result| {
                let Some(w) = weak.upgrade() else { return };
                let row = row_weak.upgrade();
                if let Some(r) = &row {
                    r.set_sensitive(true);
                }
                let failure = match result {
                    Ok(enabled) if enabled == want => {
                        w.save_setting("run-at-login", if want { "true" } else { "false" });
                        return;
                    }
                    Ok(_) | Err(PortalError::Denied) => {
                        gettext("Run at Login was not allowed. Check Background Apps in Settings.")
                    }
                    Err(PortalError::Unavailable(e)) => {
                        eprintln!("tick: background portal: {e}");
                        gettext("Run at Login is not available on this system.")
                    }
                };
                // Put the switch back without asking again.
                if let (Some(r), Some(id)) = (&row, handler.take()) {
                    r.block_signal(&id);
                    r.set_active(!want);
                    r.unblock_signal(&id);
                    handler.set(Some(id));
                }
                if let Some(d) = dialog_weak.upgrade() {
                    d.add_toast(adw::Toast::builder().title(failure).timeout(8).build());
                }
            },
        );
    }

    /// Ctrl+? and the main menu (4.18).
    pub(crate) fn show_shortcuts(&self) {
        if self.visible_dialog().is_some() {
            return;
        }
        crate::shortcuts::dialog().present(Some(self));
    }

    /// "Log Out" (main menu, preferences): confirm first (4.25).
    pub(crate) fn confirm_log_out(&self) {
        let dialog = adw::AlertDialog::new(
            Some(&gettext("Log out of Tick?")),
            Some(&gettext(
                "The local cache and unsynced changes on this computer will be removed.",
            )),
        );
        dialog.add_response("cancel", &gettext("Cancel"));
        dialog.add_response("log-out", &gettext("Log Out"));
        dialog.set_response_appearance("log-out", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        crate::menus::quiet_scrollers(&dialog);
        let weak = self.downgrade();
        dialog.connect_response(None, move |_, r| {
            if r == "log-out"
                && let Some(w) = weak.upgrade()
            {
                w.log_out();
            }
        });
        // From the preferences dialog: the alert goes over it.
        let parent: gtk::Widget = match self.visible_dialog() {
            Some(d) => d.upcast(),
            None => self.clone().upcast(),
        };
        dialog.present(Some(&parent));
    }

    /// Stop syncing, then forget the account: the token in the keyring, the cache and its
    /// queue, reminder state. The login page comes back.
    pub(crate) fn log_out(&self) {
        let imp = self.imp();
        if let Some(d) = self.visible_dialog() {
            d.force_close();
        }
        imp.logging_out.set(true);
        match imp.engine.take() {
            // The engine has its own connection and may be mid-sync. It stops at once (a
            // request in flight is abandoned, never applied), and its event stream ends only
            // when it can no longer write: the cache is cleared then (`engine_stopped`), never
            // before, so a late response cannot refill it and no second engine starts while
            // the old one runs (the login page comes back only then).
            Some(e) => e.shutdown(),
            None => self.finish_log_out(),
        }
    }

    /// The engine's event stream ended (`start_engine`).
    pub(crate) fn engine_stopped(&self) {
        if self.imp().logging_out.get() {
            self.finish_log_out();
        }
    }

    fn finish_log_out(&self) {
        let imp = self.imp();
        if !imp.logging_out.replace(false) {
            return;
        }
        self.close_task(false);
        if let Some(t) = imp.last_toast.take() {
            t.dismiss();
        }
        imp.undo.replace(None);
        imp.undo_action.replace(None);
        if let Err(e) = self.store().clear_account() {
            eprintln!("tick: clearing the cache: {e}");
        }
        self.withdraw_reminders();
        if !crate::test_mode::enabled() {
            // The keyring may block (unlock prompt): never on the main thread.
            let delete = gio::spawn_blocking(tick_core::token::delete_blocking);
            glib::spawn_future_local(async move {
                if let Ok(Err(e)) = delete.await {
                    eprintln!("tick: removing the token from the keyring: {e}");
                }
            });
        }
        {
            let mut ui = imp.ui.borrow_mut();
            *ui = crate::rows::UiState::default();
        }
        imp.status.replace(tick_core::SyncStatus::default());
        imp.known_failed.set(None);
        imp.draft.replace(String::new());
        imp.login_token.replace(None);
        if let Some(sb) = imp.sidebar.get() {
            sb.collapsed.borrow_mut().clear();
        }
        self.open_view(crate::rows::ViewId::Today, false);
        self.refresh();
        self.show_login_page();
        if crate::test_mode::enabled() {
            eprintln!("TICK_LOGGED_OUT=1");
        }
    }
}

/// libadwaita 1.9 leaves each group's boxed list unnamed and the page's scrolled window in
/// the focus order with nothing to say: name the lists after their group, take the scroller
/// out of the focus order.
fn name_lists(w: &gtk::Widget, group: Option<&str>) {
    let title = w
        .downcast_ref::<adw::PreferencesGroup>()
        .map(|g| g.title().to_string());
    let group = title.as_deref().or(group);
    if let Some(list) = w.downcast_ref::<gtk::ListBox>()
        && let Some(g) = group
    {
        list.update_property(&[gtk::accessible::Property::Label(g)]);
    } else if let Some(sw) = w.downcast_ref::<gtk::ScrolledWindow>() {
        sw.set_focusable(false);
    }
    let mut c = w.first_child();
    while let Some(x) = c {
        name_lists(&x, group);
        c = x.next_sibling();
    }
}

/// Application actions of this module: preferences, shortcuts, log out (`main.rs`).
pub fn install_actions(app: &adw::Application) {
    let window = |app: &adw::Application| {
        app.windows()
            .into_iter()
            .find_map(|w| w.downcast::<TickWindow>().ok())
    };
    let prefs = gio::ActionEntry::builder("preferences")
        .activate(move |app: &adw::Application, _, _| {
            if let Some(w) = window(app) {
                w.present();
                w.show_preferences();
            }
        })
        .build();
    let shortcuts = gio::ActionEntry::builder("shortcuts")
        .activate(move |app: &adw::Application, _, _| {
            if let Some(w) = window(app) {
                w.present();
                w.show_shortcuts();
            }
        })
        .build();
    let log_out = gio::ActionEntry::builder("log-out")
        .activate(move |app: &adw::Application, _, _| {
            if let Some(w) = window(app) {
                w.present();
                w.confirm_log_out();
            }
        })
        .build();
    app.add_action_entries([prefs, shortcuts, log_out]);
    app.set_accels_for_action("app.preferences", &["<Control>comma"]);
    app.set_accels_for_action("app.shortcuts", &["<Control>question"]);
    app.set_accels_for_action("window.close", &["<Control>w"]);
}
