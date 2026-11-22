//! Quick-add token highlighter.
//!
//! Finds `#project`, `@label`, `/section` and `p1`-`p4` tokens in quick-add text so the UI
//! can highlight them while the user types. Dates are not parsed locally: the server does
//! that (`POST /api/v1/tasks/quick`).
//!
//! Rules (following the Quick Add syntax in `docs/api/pages/tasks.md`):
//! - a token starts at the beginning of the text or after whitespace,
//! - `#`, `@` and `/` names run to the next whitespace; `\ ` escapes a space
//!   (`#My\ Project`); with [`highlight_with_names`] a known multi-word name also matches,
//! - priorities are `p1`..`p4` (any case) or `!!1`..`!!4` as a whole word.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Project,
    Label,
    Section,
    Priority(u8),
}

/// A highlighted range. `start`/`end` are UTF-8 byte offsets (what Pango attributes use).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub kind: TokenKind,
    /// The name without the sigil and with escapes removed.
    pub value: String,
}

impl Span {
    /// Character (not byte) offsets, for widgets that count characters.
    pub fn char_range(&self, text: &str) -> (usize, usize) {
        (
            text[..self.start].chars().count(),
            text[..self.end].chars().count(),
        )
    }
}

/// Highlight without knowing project or label names.
pub fn highlight(text: &str) -> Vec<Span> {
    highlight_with_names(text, &[], &[])
}

/// Highlight, preferring the longest known project or label name (which may contain
/// spaces) right after `#` or `@`.
pub fn highlight_with_names(text: &str, projects: &[&str], labels: &[&str]) -> Vec<Span> {
    let mut spans = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < text.len() {
        let at_word_start = i == 0
            || text[..i]
                .chars()
                .next_back()
                .is_some_and(char::is_whitespace);
        if !at_word_start {
            i += text[i..].chars().next().map_or(1, char::len_utf8);
            continue;
        }
        let c = bytes[i];
        let sigil = match c {
            b'#' => Some(TokenKind::Project),
            b'@' => Some(TokenKind::Label),
            b'/' => Some(TokenKind::Section),
            _ => None,
        };
        if let Some(kind) = sigil {
            let name_start = i + 1;
            let known: &[&str] = match kind {
                TokenKind::Project => projects,
                TokenKind::Label => labels,
                _ => &[],
            };
            let rest = &text[name_start..];
            let best = known
                .iter()
                .filter(|n| !n.is_empty())
                .filter(|n| {
                    rest.len() >= n.len()
                        && rest.is_char_boundary(n.len())
                        && rest[..n.len()].to_lowercase() == n.to_lowercase()
                        && rest[n.len()..]
                            .chars()
                            .next()
                            .is_none_or(char::is_whitespace)
                })
                .max_by_key(|n| n.len());
            let (end, value) = match best {
                Some(n) => (name_start + n.len(), rest[..n.len()].to_owned()),
                None => scan_name(text, name_start),
            };
            if !value.is_empty() {
                spans.push(Span {
                    start: i,
                    end,
                    kind,
                    value,
                });
            }
            i = end.max(i + 1);
            continue;
        }
        // Priority: a whole word.
        let word_end = text[i..]
            .char_indices()
            .find(|(_, ch)| ch.is_whitespace())
            .map_or(text.len(), |(o, _)| i + o);
        let word = &text[i..word_end];
        let p = match word.as_bytes() {
            [b'p' | b'P', d @ b'1'..=b'4'] => Some(d - b'0'),
            [b'!', b'!', d @ b'1'..=b'4'] => Some(d - b'0'),
            _ => None,
        };
        if let Some(p) = p {
            spans.push(Span {
                start: i,
                end: word_end,
                kind: TokenKind::Priority(p),
                value: word.to_owned(),
            });
        }
        i = word_end.max(i + 1);
    }
    spans
}

/// Scan a name until unescaped whitespace; returns (end offset, unescaped name).
fn scan_name(text: &str, start: usize) -> (usize, String) {
    let mut value = String::new();
    let mut chars = text[start..].char_indices().peekable();
    let mut end = start;
    while let Some((o, ch)) = chars.next() {
        if ch == '\\' {
            if let Some(&(o2, next)) = chars.peek() {
                value.push(next);
                chars.next();
                end = start + o2 + next.len_utf8();
                continue;
            }
            end = start + o + 1;
            break;
        }
        if ch.is_whitespace() {
            break;
        }
        value.push(ch);
        end = start + o + ch.len_utf8();
    }
    (end, value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(text: &str) -> Vec<(TokenKind, String)> {
        highlight(text)
            .into_iter()
            .map(|s| (s.kind, text[s.start..s.end].to_owned()))
            .collect()
    }

    #[test]
    fn basic_tokens() {
        assert_eq!(
            kinds("Buy milk tomorrow #Shopping @groceries p1"),
            vec![
                (TokenKind::Project, "#Shopping".into()),
                (TokenKind::Label, "@groceries".into()),
                (TokenKind::Priority(1), "p1".into()),
            ]
        );
        assert_eq!(
            kinds("Submit report #Work /Urgent @important P3"),
            vec![
                (TokenKind::Project, "#Work".into()),
                (TokenKind::Section, "/Urgent".into()),
                (TokenKind::Label, "@important".into()),
                (TokenKind::Priority(3), "P3".into()),
            ]
        );
        assert_eq!(
            kinds("Call !!2"),
            vec![(TokenKind::Priority(2), "!!2".into())]
        );
    }

    #[test]
    fn not_tokens() {
        assert!(kinds("email me@example.com about p5 and p12 or xp1").is_empty());
        assert!(kinds("issue#12 a/b").is_empty());
        assert!(kinds("# @ /").is_empty());
    }

    #[test]
    fn escapes_and_known_names() {
        let s = highlight("Plan #My\\ Project now");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].value, "My Project");
        assert_eq!(
            &"Plan #My\\ Project now"[s[0].start..s[0].end],
            "#My\\ Project"
        );
        let text = "Plan #side project @deep work p2";
        let s = highlight_with_names(text, &["Side Project", "Side"], &["deep work"]);
        assert_eq!(s[0].value, "side project");
        assert_eq!(&text[s[0].start..s[0].end], "#side project");
        assert_eq!(s[1].value, "deep work");
        assert_eq!(s[2].kind, TokenKind::Priority(2));
    }

    #[test]
    fn unicode_offsets() {
        let text = "买牛奶 #购物 @家 p1";
        let s = highlight(text);
        assert_eq!(s.len(), 3);
        assert_eq!(&text[s[0].start..s[0].end], "#购物");
        assert_eq!(s[0].char_range(text), (4, 7));
        assert_eq!(s[2].kind, TokenKind::Priority(1));
    }
}
