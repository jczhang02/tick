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
    list: gtk::Box,
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
        body.append(&list);
        body.append(&footer);
        let popover = gtk::Popover::builder().child(&body).build();
        button.set_popover(Some(&popover));

        let pop = popover.clone();
        sync_now.connect_clicked(move |_| {
            on_sync_now();
            pop.popdown();
        });
        let pop = popover.clone();
        dismiss.connect_clicked(move |_| {
            on_dismiss();
            pop.popdown();
        });
        Rc::new(SyncIndicator {
            button,
            icon,
            count,
            heading,
            list,
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
        self.sync_now.set_sensitive(!v.offline);
        self.dismiss.set_visible(!v.failed.is_empty());
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
