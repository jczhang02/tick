//! Fractional-indexing order keys.
//!
//! The docs describe `order_key` as a "fractional-indexing order key" compared
//! lexicographically (example values `a1V`, `a3`). This module implements the common
//! base-62 scheme those examples match: a head character encodes the length of an integer
//! part, followed by an optional fraction without trailing zeros. Keys the server sends that
//! do not follow the scheme are still handled by a generic midpoint fallback.

const DIGITS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
const ZERO: u8 = b'0';

fn digit_index(c: u8) -> Option<usize> {
    DIGITS.iter().position(|&d| d == c)
}

/// Midpoint of two fraction strings, `a < b` (`b` = `None` means "1").
fn midpoint(a: &[u8], b: Option<&[u8]>) -> Option<Vec<u8>> {
    if let Some(b) = b {
        if a >= b {
            return None;
        }
        // Common prefix (treat missing digits of `a` as zero).
        let mut n = 0;
        while n < b.len() && a.get(n).copied().unwrap_or(ZERO) == b[n] {
            n += 1;
        }
        if n > 0 {
            let mut out = b[..n].to_vec();
            let rest_a = if n < a.len() { &a[n..] } else { &[][..] };
            out.extend(midpoint(rest_a, Some(&b[n..]))?);
            return Some(out);
        }
    }
    let digit_a = match a.first() {
        Some(&c) => digit_index(c)?,
        None => 0,
    };
    let digit_b = match b {
        Some(b) => digit_index(*b.first()?)?,
        None => DIGITS.len(),
    };
    if digit_b - digit_a > 1 {
        let mid = (digit_a + digit_b).div_ceil(2);
        return Some(vec![DIGITS[mid]]);
    }
    if let Some(b) = b
        && b.len() > 1
    {
        return Some(vec![b[0]]);
    }
    let mut out = vec![DIGITS[digit_a]];
    let rest = if a.len() > 1 { &a[1..] } else { &[][..] };
    out.extend(midpoint(rest, None)?);
    Some(out)
}

fn integer_length(head: u8) -> Option<usize> {
    match head {
        b'a'..=b'z' => Some((head - b'a') as usize + 2),
        b'A'..=b'Z' => Some((b'Z' - head) as usize + 2),
        _ => None,
    }
}

fn integer_part(key: &[u8]) -> Option<&[u8]> {
    let len = integer_length(*key.first()?)?;
    if len > key.len() {
        return None;
    }
    Some(&key[..len])
}

fn validate(key: &[u8]) -> Option<()> {
    let int = integer_part(key)?;
    if !key.iter().all(|c| digit_index(*c).is_some()) {
        return None;
    }
    let frac = &key[int.len()..];
    if frac.last() == Some(&ZERO) {
        return None;
    }
    Some(())
}

fn increment_integer(x: &[u8]) -> Option<Vec<u8>> {
    let head = x[0];
    let mut digs = x[1..].to_vec();
    let mut carry = true;
    let mut i = digs.len();
    while carry && i > 0 {
        i -= 1;
        let d = digit_index(digs[i])? + 1;
        if d == DIGITS.len() {
            digs[i] = ZERO;
        } else {
            digs[i] = DIGITS[d];
            carry = false;
        }
    }
    if carry {
        if head == b'Z' {
            return Some(vec![b'a', ZERO]);
        }
        if head == b'z' {
            return None;
        }
        let h = head + 1;
        if h > b'a' {
            digs.push(ZERO);
        } else {
            digs.pop();
        }
        let mut out = vec![h];
        out.extend(digs);
        return Some(out);
    }
    let mut out = vec![head];
    out.extend(digs);
    Some(out)
}

fn decrement_integer(x: &[u8]) -> Option<Vec<u8>> {
    let head = x[0];
    let mut digs = x[1..].to_vec();
    let mut borrow = true;
    let mut i = digs.len();
    while borrow && i > 0 {
        i -= 1;
        let d = digit_index(digs[i])?;
        if d == 0 {
            digs[i] = DIGITS[DIGITS.len() - 1];
        } else {
            digs[i] = DIGITS[d - 1];
            borrow = false;
        }
    }
    if borrow {
        if head == b'a' {
            return Some(vec![b'Z', DIGITS[DIGITS.len() - 1]]);
        }
        if head == b'A' {
            return None;
        }
        let h = head - 1;
        if h < b'Z' {
            digs.push(DIGITS[DIGITS.len() - 1]);
        } else {
            digs.pop();
        }
        let mut out = vec![h];
        out.extend(digs);
        return Some(out);
    }
    let mut out = vec![head];
    out.extend(digs);
    Some(out)
}

fn strict_between(a: Option<&[u8]>, b: Option<&[u8]>) -> Option<Vec<u8>> {
    if let Some(a) = a {
        validate(a)?;
    }
    if let Some(b) = b {
        validate(b)?;
    }
    match (a, b) {
        (Some(a), Some(b)) if a >= b => None,
        (None, None) => Some(vec![b'a', ZERO]),
        (None, Some(b)) => {
            let ib = integer_part(b)?;
            let fb = &b[ib.len()..];
            let smallest: Vec<u8> = std::iter::once(b'A')
                .chain(std::iter::repeat_n(ZERO, 26))
                .collect();
            if ib == smallest.as_slice() {
                let mut out = ib.to_vec();
                out.extend(midpoint(&[], Some(fb))?);
                return Some(out);
            }
            if ib.len() < b.len() {
                return Some(ib.to_vec());
            }
            decrement_integer(ib)
        }
        (Some(a), None) => {
            let ia = integer_part(a)?;
            let fa = &a[ia.len()..];
            match increment_integer(ia) {
                Some(i) => Some(i),
                None => {
                    let mut out = ia.to_vec();
                    out.extend(midpoint(fa, None)?);
                    Some(out)
                }
            }
        }
        (Some(a), Some(b)) => {
            let ia = integer_part(a)?;
            let fa = &a[ia.len()..];
            let ib = integer_part(b)?;
            let fb = &b[ib.len()..];
            if ia == ib {
                let mut out = ia.to_vec();
                out.extend(midpoint(fa, Some(fb))?);
                return Some(out);
            }
            let i = increment_integer(ia)?;
            if i.as_slice() < b {
                return Some(i);
            }
            let mut out = ia.to_vec();
            out.extend(midpoint(fa, None)?);
            Some(out)
        }
    }
}

/// Generic fallback for keys outside the scheme: any string strictly between `a` and `b`.
fn loose_between(a: Option<&str>, b: Option<&str>) -> String {
    match (a, b) {
        (None, None) => "a0".into(),
        (Some(a), None) => format!("{a}V"),
        (a, Some(b)) => {
            let a = a.unwrap_or("");
            // Walk the common prefix, then pick a character between the first differing
            // characters, or extend `a`.
            let (ab, bb) = (a.as_bytes(), b.as_bytes());
            let mut i = 0;
            while i < ab.len() && i < bb.len() && ab[i] == bb[i] {
                i += 1;
            }
            let ca = ab.get(i).copied().unwrap_or(0);
            let cb = bb.get(i).copied().unwrap_or(0);
            if cb > ca + 1 && (ca + 1).is_ascii_graphic() {
                let mut out = bb[..i].to_vec();
                out.push(ca + (cb - ca) / 2);
                String::from_utf8(out).unwrap_or_else(|_| format!("{a}V"))
            } else if a.is_empty() {
                // Nothing fits before `b` with printable characters other than a prefix trick.
                let mut out = bb.to_vec();
                out.pop();
                String::from_utf8(out).unwrap_or_default()
            } else {
                format!("{a}V")
            }
        }
    }
}

/// A key that sorts strictly between `a` and `b` (either may be absent for "start"/"end").
pub fn key_between(a: Option<&str>, b: Option<&str>) -> String {
    if let Some(k) = strict_between(a.map(str::as_bytes), b.map(str::as_bytes))
        && let Ok(s) = String::from_utf8(k)
    {
        return s;
    }
    loose_between(a, b)
}

/// `n` increasing keys after `a` (used to number a list from scratch).
pub fn keys_after(a: Option<&str>, n: usize) -> Vec<String> {
    let mut out = Vec::with_capacity(n);
    let mut prev = a.map(str::to_owned);
    for _ in 0..n {
        let k = key_between(prev.as_deref(), None);
        prev = Some(k.clone());
        out.push(k);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_sequence() {
        assert_eq!(key_between(None, None), "a0");
        assert_eq!(key_between(Some("a0"), None), "a1");
        assert_eq!(key_between(Some("a1"), None), "a2");
        assert_eq!(key_between(None, Some("a0")), "Zz");
        assert_eq!(key_between(Some("a0"), Some("a1")), "a0V");
        assert_eq!(key_between(Some("a1V"), Some("a2")), "a1l");
    }

    #[test]
    fn between_is_strict_for_many_inserts() {
        let mut keys = keys_after(None, 5);
        for _ in 0..200 {
            let k = key_between(Some(&keys[1]), Some(&keys[2]));
            assert!(keys[1] < k && k < keys[2], "{} {} {}", keys[1], k, keys[2]);
            keys.insert(2, k);
        }
        for _ in 0..100 {
            let k = key_between(None, Some(&keys[0]));
            assert!(k < keys[0]);
            keys.insert(0, k);
        }
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(sorted, keys);
    }

    #[test]
    fn docs_example_keys_work() {
        let k = key_between(Some("a1V"), Some("a3"));
        assert!("a1V" < k.as_str() && k.as_str() < "a3");
    }

    #[test]
    fn invalid_keys_fall_back() {
        let k = key_between(Some("zzz-weird"), None);
        assert!(k.as_str() > "zzz-weird");
        let k = key_between(Some("m"), Some("q"));
        assert!("m" < k.as_str() && k.as_str() < "q");
    }
}
