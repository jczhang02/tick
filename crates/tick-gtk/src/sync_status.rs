//! The sync status button in the task list header bar (`DESIGN.md` 4.14): hidden when
//! online with nothing pending; otherwise offline, pending or failed state with a popover
//! that lists the changes.

use std::rc::Rc;

use adw::prelude::*;

use crate::i18n::{gettext, ncount};

/// What the indicator shows.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SyncView {
    pub offline: bool,
    /// Descriptions of queued changes ("Complete "Buy milk"").
    pub pending: Vec<String>,
    /// Rejected changes: (description, server message).
    pub failed: Vec<(String, String)>,
}

impl SyncView {
    /// The status line: tooltip, accessible description and popover heading.
    pub fn status_text(&self) -> Option<String> {
        if !self.failed.is_empty() {
            Some(ncount(
                "{n} change could not be saved",
                "{n} changes could not be saved",
                self.failed.len(),
            ))
        } else if !self.pending.is_empty() && self.offline {
            // Say why the changes wait (4.14).
            Some(ncount(
                "Offline. {n} change waiting to sync.",
                "Offline. {n} changes waiting to sync.",
                self.pending.len(),
            ))
        } else if !self.pending.is_empty() {
            Some(ncount(
                "{n} change waiting to sync",
                "{n} changes waiting to sync",
                self.pending.len(),
            ))
        } else if self.offline {
            Some(gettext("Offline. Changes are saved on this computer."))
        } else {
            None
        }
    }
}

pub struct SyncIndicator {
    button: gtk::MenuButton,
    icon: gtk::Image,
    count: gtk::Label,
    heading: gtk::Label,
    detail: gtk::Label,
    list: gtk::Box,
    footer: gtk::Box,
    sync_now: gtk::Button,
    dismiss: gtk::Button,
}

impl SyncIndicator {
    pub fn new(
        button: gtk::MenuButton,
        on_sync_now: impl Fn() + 'static,
        on_dismiss: impl Fn() + 'static,
    ) -> Rc<SyncIndicator> {
        let icon = gtk::Image::builder()
            .icon_name("network-offline-symbolic")
            .accessible_role(gtk::AccessibleRole::Presentation)
            .build();
        let count = gtk::Label::builder()
            .css_classes(["caption", "numeric"])
            .visible(false)
            .build();
        let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        content.append(&icon);
        content.append(&count);
        button.set_child(Some(&content));

        let heading = gtk::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .css_classes(["heading"])
            .build();
        // Failed changes were rolled back: say so once, above the server messages (4.14, 6.4).
        let detail = gtk::Label::builder()
            .label(gettext(
                "Todoist rejected the change. Your task shows the version on the server.",
            ))
            .xalign(0.0)
            .wrap(true)
            // Wrap at the popover width (4.14) instead of widening it to one line.
            .max_width_chars(20)
            .visible(false)
            .css_classes(["caption", "dimmed"])
            .build();
        let list = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let sync_now = gtk::Button::builder()
            .label(gettext("Sync Now"))
            .css_classes(["suggested-action", "tick-hit"])
            .build();
        let dismiss = gtk::Button::builder()
            .label(gettext("Dismiss Errors"))
            .css_classes(["tick-hit"])
            .build();
        let footer = gtk::Box::builder()
            .spacing(12)
            .halign(gtk::Align::End)
            .margin_top(12)
            .build();
        footer.append(&dismiss);
        footer.append(&sync_now);
        let body = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(6)
            .margin_start(6)
            .margin_end(6)
            .margin_top(6)
            .margin_bottom(6)
            .width_request(296)
            .css_classes(["tick-sync-popover"])
            .build();
        body.append(&heading);
        body.append(&detail);
        body.append(&list);
        body.append(&footer);
        let popover = gtk::Popover::builder().child(&body).build();
        button.set_popover(Some(&popover));

        // Weak: the popover owns these buttons (no reference cycle).
        let pop = popover.downgrade();
        sync_now.connect_clicked(move |_| {
            on_sync_now();
            if let Some(p) = pop.upgrade() {
                p.popdown();
            }
        });
        let pop = popover.downgrade();
        dismiss.connect_clicked(move |_| {
            on_dismiss();
            if let Some(p) = pop.upgrade() {
                p.popdown();
            }
        });
        Rc::new(SyncIndicator {
            button,
            icon,
            count,
            heading,
            detail,
            list,
            footer,
            sync_now,
            dismiss,
        })
    }

    pub fn update(&self, v: &SyncView) {
        let Some(text) = v.status_text() else {
            self.button.set_visible(false);
            self.button.popdown();
            return;
        };
        self.button.set_visible(true);
        self.button.set_tooltip_text(Some(&text));
        self.button
            .update_property(&[gtk::accessible::Property::Description(&text)]);
        self.heading.set_label(&text);
        for c in ["dimmed", "tick-sync-error"] {
            self.icon.remove_css_class(c);
        }
        let (icon, n) = if !v.failed.is_empty() {
            self.icon.add_css_class("tick-sync-error");
            ("dialog-warning-symbolic", v.failed.len())
        } else if !v.pending.is_empty() {
            let icon = if v.offline {
                "network-offline-symbolic"
            } else {
                "network-transmit-symbolic"
            };
            (icon, v.pending.len())
        } else {
            self.icon.add_css_class("dimmed");
            ("network-offline-symbolic", 0)
        };
        self.icon.set_icon_name(Some(icon));
        self.count.set_label(&n.to_string());
        self.count.set_visible(n > 0);

        while let Some(c) = self.list.first_child() {
            self.list.remove(&c);
        }
        for (desc, msg) in &v.failed {
            self.list.append(&change_row(desc, Some(msg)));
        }
        for desc in &v.pending {
            self.list.append(&change_row(desc, None));
        }
        self.detail.set_visible(!v.failed.is_empty());
        // One primary action (4.14): Sync Now while changes wait and the network is there;
        // otherwise Dismiss Errors. Offline, Sync Now stays visible but plain and insensitive,
        // so it does not read as an available accent button.
        let sync_primary = !v.offline && (!v.pending.is_empty() || v.failed.is_empty());
        set_class(&self.sync_now, "suggested-action", sync_primary);
        set_class(&self.sync_now, "flat", !v.offline && !sync_primary);
        self.sync_now.set_sensitive(!v.offline);
        self.dismiss.set_visible(!v.failed.is_empty());
        set_class(&self.dismiss, "suggested-action", !sync_primary);
        // The primary action goes last (GNOME HIG).
        if sync_primary {
            self.footer
                .reorder_child_after(&self.sync_now, Some(&self.dismiss));
        } else {
            self.footer
                .reorder_child_after(&self.dismiss, Some(&self.sync_now));
        }
    }

    /// Opens the popover ("Details" on a failed-change toast).
    pub fn popup(&self) {
        if self.button.is_visible() {
            self.button.popup();
        }
    }
}

fn change_row(desc: &str, message: Option<&str>) -> gtk::Widget {
    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .valign(gtk::Align::Center)
        .css_classes(["tick-picker-row"])
        .build();
    let inner = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .vexpand(true)
        .valign(gtk::Align::Center)
        .build();
    inner.append(
        &gtk::Label::builder()
            .label(desc)
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build(),
    );
    if let Some(m) = message {
        inner.append(
            &gtk::Label::builder()
                .label(m)
                .xalign(0.0)
                .wrap(true)
                .css_classes(["caption", "dimmed"])
                .build(),
        );
    }
    row.append(&inner);
    row.upcast()
}

fn set_class(w: &impl IsA<gtk::Widget>, class: &str, on: bool) {
    if on {
        w.add_css_class(class);
    } else {
        w.remove_css_class(class);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_line_says_why_changes_wait() {
        let mut v = SyncView {
            offline: true,
            ..SyncView::default()
        };
        assert_eq!(
            v.status_text().as_deref(),
            Some("Offline. Changes are saved on this computer.")
        );
        v.pending = vec!["Complete \"Buy milk\"".into()];
        assert_eq!(
            v.status_text().as_deref(),
            Some("Offline. 1 change waiting to sync.")
        );
        v.offline = false;
        v.pending.push("Rename project \"Home\"".into());
        assert_eq!(
            v.status_text().as_deref(),
            Some("2 changes waiting to sync")
        );
        v.failed = vec![("Complete \"Buy milk\"".into(), "Item not found".into())];
        assert_eq!(
            v.status_text().as_deref(),
            Some("1 change could not be saved")
        );
    }
}
