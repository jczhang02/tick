//! `child_order` and `order_key` bookkeeping.
//!
//! The docs describe `order_key` as a fractional-indexing key compared lexicographically and
//! `child_order` as the (deprecated) integer position among siblings. The fake keeps both
//! consistent: after a change to one, the sibling group is renumbered from it.

use serde_json::{Value, json};

use crate::State;

const DIGITS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// The n-th key of an evenly spaced sequence (`a1`, `a2`, ..., `az`, `b10`, ...), valid in
/// the base-62 fractional-indexing scheme the docs' examples (`a1V`, `a3`) use.
pub fn nth_key(n: usize) -> String {
    let n = n.max(1);
    if n < 62 {
        return format!("a{}", DIGITS[n] as char);
    }
    // Two-digit integers start at "b10" (n = 62).
    let m = n;
    if m < 62 * 62 {
        return format!("b{}{}", DIGITS[m / 62] as char, DIGITS[m % 62] as char);
    }
    let m = m.min(62 * 62 * 62 - 1);
    format!(
        "c{}{}{}",
        DIGITS[m / (62 * 62)] as char,
        DIGITS[(m / 62) % 62] as char,
        DIGITS[m % 62] as char
    )
}

fn same(a: &Value, b: Option<&str>) -> bool {
    a.as_str() == b
}

fn item_siblings(
    st: &State,
    project: &str,
    section: Option<&str>,
    parent: Option<&str>,
) -> Vec<Value> {
    st.live("items")
        .filter(|v| {
            v["project_id"].as_str() == Some(project)
                && same(&v["section_id"], section)
                && same(&v["parent_id"], parent)
                && !v["checked"].as_bool().unwrap_or(false)
        })
        .cloned()
        .collect()
}

fn max_key(list: &[Value]) -> Option<String> {
    list.iter()
        .filter_map(|v| v["order_key"].as_str().map(str::to_owned))
        .max()
}

fn after(key: Option<String>, count: usize) -> String {
    match key {
        // Appending "V" always sorts after `k` and stays a valid fraction.
        Some(k) => {
            let candidate = nth_key(count + 1);
            if candidate > k {
                candidate
            } else {
                format!("{k}V")
            }
        }
        None => nth_key(count + 1),
    }
}

/// Position for a new last child: (child_order, order_key).
pub fn next_item_position(
    st: &State,
    project: &str,
    section: Option<&str>,
    parent: Option<&str>,
) -> (i64, String) {
    let sibs = item_siblings(st, project, section, parent);
    let order = sibs
        .iter()
        .filter_map(|v| v["child_order"].as_i64())
        .max()
        .unwrap_or(0)
        + 1;
    (order, after(max_key(&sibs), sibs.len()))
}

pub fn next_project_position(st: &State, parent: Option<&str>) -> (i64, String) {
    let sibs: Vec<Value> = st
        .live("projects")
        .filter(|v| same(&v["parent_id"], parent))
        .cloned()
        .collect();
    let order = sibs
        .iter()
        .filter_map(|v| v["child_order"].as_i64())
        .max()
        .unwrap_or(0)
        + 1;
    (order, after(max_key(&sibs), sibs.len()))
}

pub fn next_section_position(st: &State, project: &str) -> (i64, String) {
    let sibs: Vec<Value> = st
        .live("sections")
        .filter(|v| v["project_id"].as_str() == Some(project))
        .cloned()
        .collect();
    let order = sibs
        .iter()
        .filter_map(|v| v["section_order"].as_i64())
        .max()
        .unwrap_or(0)
        + 1;
    (order, after(max_key(&sibs), sibs.len()))
}

fn group_of(st: &State, id: &str) -> Option<Vec<Value>> {
    let v = st.get("items", id)?;
    Some(item_siblings(
        st,
        v["project_id"].as_str().unwrap_or(""),
        v["section_id"].as_str(),
        v["parent_id"].as_str(),
    ))
}

/// Docs: if a sibling already uses the requested key, the stored key places the task
/// immediately after that position.
pub fn adjust_item_key(st: &State, id: &str, key: &str) -> String {
    let Some(sibs) = group_of(st, id) else {
        return key.to_owned();
    };
    let mut k = key.to_owned();
    while sibs
        .iter()
        .any(|v| v["id"].as_str() != Some(id) && v["order_key"].as_str() == Some(k.as_str()))
    {
        k.push('V');
    }
    k
}

pub fn renumber_item_child_order_from_keys(st: &mut State, id: &str) {
    let Some(mut sibs) = group_of(st, id) else {
        return;
    };
    sibs.sort_by(|a, b| {
        a["order_key"]
            .as_str()
            .cmp(&b["order_key"].as_str())
            .then(a["child_order"].as_i64().cmp(&b["child_order"].as_i64()))
    });
    for (i, v) in sibs.iter().enumerate() {
        let sid = v["id"].as_str().unwrap_or("").to_owned();
        let want = i as i64 + 1;
        if v["child_order"].as_i64() != Some(want) {
            st.update("items", &sid, |x| x["child_order"] = json!(want));
        }
    }
}

pub fn renumber_item_keys_from_child_order(st: &mut State, id: &str) {
    let Some(mut sibs) = group_of(st, id) else {
        return;
    };
    sibs.sort_by_key(|v| v["child_order"].as_i64());
    for (i, v) in sibs.iter().enumerate() {
        let sid = v["id"].as_str().unwrap_or("").to_owned();
        let key = nth_key(i + 1);
        if v["order_key"].as_str() != Some(key.as_str()) {
            st.update("items", &sid, |x| x["order_key"] = json!(key));
        }
    }
}

fn project_group(st: &State, id: &str) -> Vec<Value> {
    let parent = st
        .get("projects", id)
        .and_then(|v| v["parent_id"].as_str())
        .map(str::to_owned);
    st.live("projects")
        .filter(|v| same(&v["parent_id"], parent.as_deref()))
        .cloned()
        .collect()
}

pub fn renumber_projects(st: &mut State, id: &str) {
    let mut sibs = project_group(st, id);
    sibs.sort_by(|a, b| a["order_key"].as_str().cmp(&b["order_key"].as_str()));
    for (i, v) in sibs.iter().enumerate() {
        let sid = v["id"].as_str().unwrap_or("").to_owned();
        let want = i as i64 + 1;
        if v["child_order"].as_i64() != Some(want) {
            st.update("projects", &sid, |x| x["child_order"] = json!(want));
        }
    }
}

pub fn renumber_project_keys_from_child_order(st: &mut State, id: &str) {
    let mut sibs = project_group(st, id);
    sibs.sort_by_key(|v| v["child_order"].as_i64());
    for (i, v) in sibs.iter().enumerate() {
        let sid = v["id"].as_str().unwrap_or("").to_owned();
        let key = nth_key(i + 1);
        if v["order_key"].as_str() != Some(key.as_str()) {
            st.update("projects", &sid, |x| x["order_key"] = json!(key));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_increase() {
        let keys: Vec<String> = (1..5000).map(nth_key).collect();
        for w in keys.windows(2) {
            assert!(w[0] < w[1], "{} !< {}", w[0], w[1]);
        }
        assert_eq!(nth_key(1), "a1");
        assert_eq!(nth_key(61), "az");
        assert_eq!(nth_key(62), "b10");
    }
}
