//! `TickRow`: the GObject in the task list's `gio::ListStore`, wrapping one [`Row`].
//!
//! Refreshes update rows in place ([`TickRow::set_row`] bumps `revision`, the bound widget
//! rebuilds) and splice only the part of the model whose keys changed, so selection, focus
//! and scroll position survive a sync.

use std::cell::{Ref, RefCell};

use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use crate::rows::Row;

mod imp {
    use std::cell::Cell;

    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::TickRow)]
    pub struct TickRow {
        pub row: RefCell<Option<Row>>,
        /// Bumped whenever `row` changes.
        #[property(get)]
        pub revision: Cell<u64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TickRow {
        const NAME: &'static str = "TickRow";
        type Type = super::TickRow;
    }

    #[glib::derived_properties]
    impl ObjectImpl for TickRow {}
}

glib::wrapper! {
    pub struct TickRow(ObjectSubclass<imp::TickRow>);
}

impl TickRow {
    pub fn new(row: Row) -> TickRow {
        let obj: TickRow = glib::Object::new();
        obj.imp().row.replace(Some(row));
        obj
    }

    pub fn row(&self) -> Ref<'_, Row> {
        Ref::map(self.imp().row.borrow(), |r| {
            r.as_ref().expect("TickRow without a row")
        })
    }

    pub fn key(&self) -> String {
        self.row().key()
    }

    /// The task id if this is a task row.
    pub fn task_id(&self) -> Option<String> {
        self.row().task().map(|t| t.id.clone())
    }

    /// Rebuild the bound widget without a data change (for example after a breakpoint).
    pub fn touch(&self) {
        let imp = self.imp();
        imp.revision.set(imp.revision.get() + 1);
        self.notify_revision();
    }

    /// Replace the row (same key); bound widgets rebuild on `notify::revision`.
    pub fn set_row(&self, row: Row) {
        if self.imp().row.borrow().as_ref() == Some(&row) {
            return;
        }
        self.imp().row.replace(Some(row));
        let imp = self.imp();
        imp.revision.set(imp.revision.get() + 1);
        self.notify_revision();
    }
}

/// Update `store` to show `rows`: rows with unchanged keys in the common prefix and suffix
/// are updated in place; the middle is spliced.
pub fn update_store(store: &gio::ListStore, rows: Vec<Row>) {
    let old: Vec<TickRow> = (0..store.n_items())
        .filter_map(|i| store.item(i).and_downcast::<TickRow>())
        .collect();
    let old_keys: Vec<String> = old.iter().map(TickRow::key).collect();
    let new_keys: Vec<String> = rows.iter().map(Row::key).collect();
    let mut prefix = 0;
    while prefix < old_keys.len() && prefix < new_keys.len() && old_keys[prefix] == new_keys[prefix]
    {
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < old_keys.len() - prefix
        && suffix < new_keys.len() - prefix
        && old_keys[old_keys.len() - 1 - suffix] == new_keys[new_keys.len() - 1 - suffix]
    {
        suffix += 1;
    }
    let new_len = rows.len();
    let mut rows = rows.into_iter().enumerate();
    let mut middle = Vec::new();
    for (i, row) in rows.by_ref() {
        if i < prefix {
            old[i].set_row(row);
        } else if i < new_len - suffix {
            middle.push(TickRow::new(row));
        } else {
            let j = old.len() - (new_len - i);
            old[j].set_row(row);
        }
    }
    let removed = old.len() - prefix - suffix;
    if removed > 0 || !middle.is_empty() {
        store.splice(prefix as u32, removed as u32, &middle);
    }
}
