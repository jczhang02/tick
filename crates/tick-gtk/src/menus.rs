//! Accessible names for `GMenuModel` menus (`DESIGN.md` 4.19).
//!
//! GTK 4.22 exposes `popover.menu` items (`GtkModelButton`) on AT-SPI without a name: the
//! label is drawn but not reported. Every interactive control needs a name (and the E2E
//! tests find menu items by it), so each item gets its text as accessible label when the
//! menu maps.

use gtk::prelude::*;

/// Name the items of `popover` (a `GtkPopoverMenu`) every time it maps.
pub fn name_items(popover: &impl IsA<gtk::Widget>) {
    popover.connect_map(|p| name_now(p.upcast_ref()));
}

/// Name the items of the menu a `GtkMenuButton` opens.
pub fn name_button(button: &gtk::MenuButton) {
    if let Some(p) = button.popover() {
        name_items(&p);
    }
    // The popover is created when the model is set: follow later changes.
    button.connect_notify_local(Some("popover"), |b, _| {
        if let Some(p) = b.popover() {
            name_items(&p);
        }
    });
}

fn name_now(w: &gtk::Widget) {
    // The menu's own scrolled window is a container, not a stop for Tab or AT.
    if let Some(sw) = w.downcast_ref::<gtk::ScrolledWindow>() {
        sw.set_focusable(false);
    }
    if w.type_().name() == "GtkModelButton"
        && let Some(text) = w
            .find_property("text")
            .and_then(|_| w.property::<Option<String>>("text"))
        && !text.is_empty()
    {
        let text = text.replace('_', "");
        // A labelled-by relation (to the item's own, hidden, label) wins over a label in the
        // name computation: drop it.
        w.reset_relation(gtk::AccessibleRelation::LabelledBy);
        w.update_property(&[gtk::accessible::Property::Label(&text)]);
    }
    let mut c = w.first_child();
    while let Some(x) = c {
        name_now(&x);
        c = x.next_sibling();
    }
}

/// libadwaita's alert dialog wraps its text in a focusable, unnamed scrolled window: a Tab
/// stop with nothing to say. Make such containers unfocusable when `widget` maps.
pub fn quiet_scrollers(widget: &impl IsA<gtk::Widget>) {
    fn walk(w: &gtk::Widget) {
        if let Some(sw) = w.downcast_ref::<gtk::ScrolledWindow>() {
            sw.set_focusable(false);
        }
        let mut c = w.first_child();
        while let Some(x) = c {
            walk(&x);
            c = x.next_sibling();
        }
    }
    widget.connect_map(|w| walk(w.upcast_ref()));
}
