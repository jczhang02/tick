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

/// Largest middle part (old x new rows) the keyed diff solves exactly; larger changes (a
/// new view fills an empty list, so this is rare) splice the whole middle.
const MAX_DIFF_CELLS: usize = 4_000_000;

/// Update `store` to show `rows`, keyed by [`Row::key`]: rows whose key stays keep their
/// object (updated in place), so selection, focus and the scroll position survive. Only
/// rows that appear, disappear or move are spliced.
///
/// After trimming the common prefix and suffix, the rest is a longest common subsequence
/// of keys. `prefer` (the selected row's key) weighs more than all other rows together:
/// when a reorder could keep either the moved row or its neighbors in place, the selected
/// row stays and its neighbors move around it (a `ListStore` has no move, so a moved
/// object is removed and inserted again, which drops its widget and keyboard focus).
pub fn update_store(store: &gio::ListStore, rows: Vec<Row>, prefer: Option<&str>) {
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
    let old_mid = &old_keys[prefix..old_keys.len() - suffix];
    let new_mid = &new_keys[prefix..new_keys.len() - suffix];
    let pairs = if old_mid.len() * new_mid.len() <= MAX_DIFF_CELLS {
        keyed_lcs(old_mid, new_mid, prefer)
    } else {
        Vec::new()
    };
    let new_len = rows.len();
    let old_len = old.len();
    let mut rows: Vec<Option<Row>> = rows.into_iter().map(Some).collect();
    // Unchanged ends: update in place.
    for i in 0..prefix {
        if let Some(r) = rows[i].take() {
            old[i].set_row(r);
        }
    }
    for k in 0..suffix {
        if let Some(r) = rows[new_len - 1 - k].take() {
            old[old_len - 1 - k].set_row(r);
        }
    }
    // Matched middle rows: update in place.
    for &(i, j) in &pairs {
        if let Some(r) = rows[prefix + j].take() {
            old[prefix + i].set_row(r);
        }
    }
    // Splice the gaps between matches, from the end so earlier positions stay valid.
    let mut bounds: Vec<(isize, isize)> = vec![(-1, -1)];
    bounds.extend(pairs.iter().map(|&(i, j)| (i as isize, j as isize)));
    bounds.push((old_mid.len() as isize, new_mid.len() as isize));
    for w in bounds.windows(2).rev() {
        let (pi, pj) = w[0];
        let (ci, cj) = w[1];
        let old_start = (pi + 1) as usize;
        let removed = (ci - pi - 1) as usize;
        let added: Vec<TickRow> = ((pj + 1) as usize..cj as usize)
            .filter_map(|j| rows[prefix + j].take().map(TickRow::new))
            .collect();
        if removed > 0 || !added.is_empty() {
            store.splice((prefix + old_start) as u32, removed as u32, &added);
        }
    }
}

/// Matched `(old, new)` index pairs of a longest common subsequence of keys, where a match
/// of `prefer` counts more than every other match together.
fn keyed_lcs(old: &[String], new: &[String], prefer: Option<&str>) -> Vec<(usize, usize)> {
    let (n, m) = (old.len(), new.len());
    if n == 0 || m == 0 {
        return Vec::new();
    }
    let heavy = (n.min(m) as u32) + 1;
    let w = |i: usize| {
        if Some(old[i].as_str()) == prefer {
            heavy
        } else {
            1
        }
    };
    // dp[i][j]: best weight of old[i..] against new[j..].
    let cols = m + 1;
    let mut dp = vec![0u32; (n + 1) * cols];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            let skip = dp[(i + 1) * cols + j].max(dp[i * cols + j + 1]);
            dp[i * cols + j] = if old[i] == new[j] {
                skip.max(w(i) + dp[(i + 1) * cols + j + 1])
            } else {
                skip
            };
        }
    }
    let mut out = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if old[i] == new[j] && dp[i * cols + j] == w(i) + dp[(i + 1) * cols + j + 1] {
            out.push((i, j));
            i += 1;
            j += 1;
        } else if dp[(i + 1) * cols + j] >= dp[i * cols + j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::keyed_lcs;

    fn k(s: &str) -> Vec<String> {
        s.chars().map(|c| c.to_string()).collect()
    }

    #[test]
    fn lcs_keeps_the_preferred_row() {
        // "c" moves to the front: without a preference the others stay.
        assert_eq!(keyed_lcs(&k("abc"), &k("cab"), None), vec![(0, 1), (1, 2)]);
        // With "c" selected, "c" stays and "a", "b" move around it.
        assert_eq!(keyed_lcs(&k("abc"), &k("cab"), Some("c")), vec![(2, 0)]);
        assert_eq!(
            keyed_lcs(&k("abcd"), &k("abxd"), None),
            vec![(0, 0), (1, 1), (3, 3)]
        );
        assert!(keyed_lcs(&k(""), &k("ab"), None).is_empty());
    }

    fn ghosts(keys: &str) -> Vec<crate::rows::Row> {
        keys.chars()
            .map(|c| crate::rows::Row::Ghost {
                key: c.to_string(),
                prefill: String::new(),
            })
            .collect()
    }

    #[test]
    fn update_store_keeps_objects_by_key() {
        use gtk::prelude::*;
        let store = gtk::gio::ListStore::new::<super::TickRow>();
        super::update_store(&store, ghosts("abcde"), None);
        let before: Vec<super::TickRow> = (0..5)
            .map(|i| store.item(i).and_downcast().unwrap())
            .collect();
        super::update_store(&store, ghosts("adbcxe"), Some("d"));
        let keys: Vec<String> = (0..store.n_items())
            .map(|i| {
                store
                    .item(i)
                    .and_downcast::<super::TickRow>()
                    .unwrap()
                    .key()
            })
            .collect();
        assert_eq!(keys, ["a", "d", "b", "c", "x", "e"]);
        let same = |pos: u32, old: usize| {
            store.item(pos).and_downcast::<super::TickRow>().unwrap() == before[old]
        };
        // The preferred row kept its object; so did the untouched ends.
        assert!(same(1, 3) && same(0, 0) && same(5, 4));
    }
}
