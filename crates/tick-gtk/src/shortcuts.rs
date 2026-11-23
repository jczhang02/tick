//! The keyboard shortcuts dialog (`DESIGN.md` 4.18): every shortcut Tick has, in three
//! sections. Accelerators use `AdwShortcutLabel` syntax: `+` joins a sequence (`g+t`: G,
//! then T), a space separates alternatives.

use adw::prelude::*;
use gtk::glib;

use crate::i18n::gettext;

/// (section title, [(title, accelerator)]).
pub fn entries() -> Vec<(String, Vec<(String, &'static str)>)> {
    vec![
        (
            gettext("General"),
            vec![
                (gettext("Quick add"), "q"),
                (gettext("Search"), "slash"),
                (gettext("Preferences"), "<Control>comma"),
                (gettext("Keyboard shortcuts"), "<Control>question"),
                (gettext("Close window"), "<Control>w"),
                (gettext("Quit"), "<Control>q"),
                (gettext("Undo"), "<Control>z"),
            ],
        ),
        (
            gettext("Navigation"),
            vec![
                (gettext("Inbox"), "g+i"),
                (gettext("Today"), "g+t"),
                (gettext("Upcoming"), "g+u"),
                (gettext("Move focus"), "Up Down k j"),
                (gettext("Collapse or expand sub-tasks"), "Left Right"),
                (gettext("Back"), "<Alt>Left"),
            ],
        ),
        (
            gettext("Tasks"),
            vec![
                (gettext("Open"), "Return"),
                (gettext("Close details"), "Escape"),
                (gettext("Complete"), "e space"),
                (gettext("Set date"), "t"),
                (gettext("Set priority"), "1...4"),
                (gettext("Add labels"), "l"),
                (gettext("Move to project"), "v"),
                (gettext("Reorder"), "<Alt><Shift>Up <Alt><Shift>Down"),
                (
                    gettext("Nest or un-nest"),
                    "<Alt><Shift>Right <Alt><Shift>Left",
                ),
                (gettext("Rename project or section"), "F2"),
                (gettext("Delete"), "<Shift>Delete"),
                (gettext("Context menu"), "<Shift>F10 Menu"),
                (gettext("Add another task (quick add)"), "<Control>Return"),
            ],
        ),
    ]
}

pub fn dialog() -> adw::ShortcutsDialog {
    let dialog = adw::ShortcutsDialog::new();
    for (title, items) in entries() {
        let section = adw::ShortcutsSection::new(Some(&title));
        for (name, accel) in items {
            section.add(adw::ShortcutsItem::new(&name, accel));
        }
        dialog.add(section);
    }
    dialog.connect_map(|d| {
        let d = d.clone();
        glib::idle_add_local_once(move || fix_up(d.upcast_ref()));
    });
    dialog
}

/// libadwaita 1.9 leaves the dialog's rows and lists unnamed and its search entry 34 px
/// tall (2.7): name each row after its title, each list after the dialog, make the entry
/// 44 px, and take the scrolled window out of the focus order (it has nothing to say).
fn fix_up(w: &gtk::Widget) {
    if let Some(row) = w.downcast_ref::<gtk::ListBoxRow>() {
        if let Some(title) = first_label(row.upcast_ref()) {
            row.update_property(&[gtk::accessible::Property::Label(&title)]);
        }
    } else if let Some(list) = w.downcast_ref::<gtk::ListBox>() {
        list.update_property(&[gtk::accessible::Property::Label(&gettext(
            "Keyboard Shortcuts",
        ))]);
    } else if let Some(entry) = w.downcast_ref::<gtk::SearchEntry>() {
        entry.add_css_class("tick-entry");
    } else if let Some(sw) = w.downcast_ref::<gtk::ScrolledWindow>() {
        sw.set_focusable(false);
    }
    let mut c = w.first_child();
    while let Some(x) = c {
        fix_up(&x);
        c = x.next_sibling();
    }
}

/// The text of the first non-empty label under `w`, depth first.
fn first_label(w: &gtk::Widget) -> Option<String> {
    if let Some(l) = w.downcast_ref::<gtk::Label>()
        && !l.label().is_empty()
    {
        return Some(l.label().to_string());
    }
    let mut c = w.first_child();
    while let Some(x) = c {
        if let Some(t) = first_label(&x) {
            return Some(t);
        }
        c = x.next_sibling();
    }
    None
}
