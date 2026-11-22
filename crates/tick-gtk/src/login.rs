//! The login page (`DESIGN.md` 4.16): paste a token, the window validates it with a sync.

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;
use gtk::glib::subclass::Signal;

use crate::i18n::gettext;

mod imp {
    use std::sync::OnceLock;

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/dev/chengrui/Tick/ui/login.ui")]
    pub struct TickLoginPage {
        #[template_child]
        pub token_row: TemplateChild<adw::PasswordEntryRow>,
        #[template_child]
        pub error_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub login_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub login_button_stack: TemplateChild<gtk::Stack>,
        pub busy: std::cell::Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TickLoginPage {
        const NAME: &'static str = "TickLoginPage";
        type Type = super::TickLoginPage;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for TickLoginPage {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("submit")
                        .param_types([String::static_type()])
                        .build(),
                ]
            })
        }

        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            let weak = obj.downgrade();
            self.token_row.connect_changed(move |_| {
                if let Some(page) = weak.upgrade() {
                    page.update_sensitivity();
                }
            });
            let weak = obj.downgrade();
            self.token_row.connect_entry_activated(move |_| {
                if let Some(page) = weak.upgrade() {
                    page.submit();
                }
            });
            let weak = obj.downgrade();
            self.login_button.connect_clicked(move |_| {
                if let Some(page) = weak.upgrade() {
                    page.submit();
                }
            });
        }
    }

    impl WidgetImpl for TickLoginPage {}
    impl BinImpl for TickLoginPage {}
}

glib::wrapper! {
    pub struct TickLoginPage(ObjectSubclass<imp::TickLoginPage>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

/// Why a token was not accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginError {
    Rejected,
    Unreachable,
}

impl TickLoginPage {
    fn token(&self) -> String {
        self.imp().token_row.text().trim().to_owned()
    }

    fn update_sensitivity(&self) {
        let imp = self.imp();
        imp.login_button
            .set_sensitive(!imp.busy.get() && !self.token().is_empty());
    }

    fn submit(&self) {
        let token = self.token();
        if token.is_empty() || self.imp().busy.get() {
            return;
        }
        self.set_busy(true);
        self.emit_by_name::<()>("submit", &[&token]);
    }

    pub fn connect_submit<F: Fn(&Self, String) + 'static>(&self, f: F) {
        self.connect_closure(
            "submit",
            false,
            glib::closure_local!(move |page: &TickLoginPage, token: String| f(page, token)),
        );
    }

    /// Shows the spinner while the token is checked.
    pub fn set_busy(&self, busy: bool) {
        let imp = self.imp();
        imp.busy.set(busy);
        imp.login_button_stack
            .set_visible_child_name(if busy { "spinner" } else { "label" });
        imp.token_row.set_sensitive(!busy);
        if busy {
            imp.error_label.set_visible(false);
        }
        self.update_sensitivity();
    }

    pub fn show_error(&self, error: LoginError) {
        let imp = self.imp();
        self.set_busy(false);
        let text = match error {
            LoginError::Rejected => gettext("That token was not accepted. Check it and try again."),
            LoginError::Unreachable => {
                gettext("Can't reach Todoist. Check your connection and try again.")
            }
        };
        imp.error_label.set_label(&text);
        imp.error_label.set_visible(true);
        imp.token_row.grab_focus();
    }

    /// Back to the initial state (after a successful login).
    pub fn reset(&self) {
        let imp = self.imp();
        self.set_busy(false);
        imp.token_row.set_text("");
        imp.error_label.set_visible(false);
    }

    pub fn focus_entry(&self) {
        self.imp().token_row.grab_focus();
    }
}
