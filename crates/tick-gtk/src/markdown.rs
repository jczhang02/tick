//! The Markdown subset of task descriptions (`SPEC.md`, `DESIGN.md` 4.8) as Pango markup:
//! bold, italic, inline code, links (and bare URLs), bullet and numbered lists. Headings
//! render as bold lines; anything else is plain text.
//!
//! Pango markup has no CSS classes (7.4 pitfall 21): code spans name the monospace family
//! directly, and links are `<a href>` so GTK draws them with its `link` node (accent color,
//! underline) and opens them on click.

/// Escape text for Pango markup.
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Render `md` as Pango markup. `mono` is the monospace font family for code spans.
pub fn to_pango(md: &str, mono: &str) -> String {
    let mut lines = Vec::new();
    for raw in md.lines() {
        let indent = raw.len() - raw.trim_start().len();
        let line = raw.trim_start();
        let pad = " ".repeat(indent.min(8));
        if let Some(rest) = ["- ", "* ", "+ "].iter().find_map(|p| line.strip_prefix(p)) {
            lines.push(format!("{pad}  •  {}", inline(rest, mono)));
        } else if let Some((num, rest)) = line.split_once(". ")
            && !num.is_empty()
            && num.len() <= 3
            && num.chars().all(|c| c.is_ascii_digit())
        {
            lines.push(format!("{pad}  {num}.  {}", inline(rest, mono)));
        } else if line.starts_with('#') {
            let text = line.trim_start_matches('#').trim();
            lines.push(format!("<b>{}</b>", inline(text, mono)));
        } else if let Some(rest) = line.strip_prefix("> ") {
            lines.push(format!("<i>{}</i>", inline(rest, mono)));
        } else {
            lines.push(format!("{pad}{}", inline(line, mono)));
        }
    }
    lines.join("\n")
}

fn find_from(chars: &[char], from: usize, pat: &[char]) -> Option<usize> {
    if pat.is_empty() {
        return None;
    }
    (from..chars.len().saturating_sub(pat.len() - 1)).find(|&i| chars[i..i + pat.len()] == *pat)
}

fn is_word(c: Option<&char>) -> bool {
    c.is_some_and(|c| c.is_alphanumeric())
}

/// Inline spans of one line.
fn inline(text: &str, mono: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    let mut plain = String::new();
    let flush = |plain: &mut String, out: &mut String| {
        out.push_str(&escape(plain));
        plain.clear();
    };
    while i < chars.len() {
        let c = chars[i];
        // Inline code: no markup inside.
        if c == '`'
            && let Some(end) = find_from(&chars, i + 1, &['`'])
        {
            flush(&mut plain, &mut out);
            let code: String = chars[i + 1..end].iter().collect();
            out.push_str(&format!(
                "<span font_family=\"{}\">{}</span>",
                escape(mono),
                escape(&code)
            ));
            i = end + 1;
            continue;
        }
        // [text](url)
        if c == '['
            && let Some(close) = find_from(&chars, i + 1, &[']', '('])
            && let Some(end) = find_from(&chars, close + 2, &[')'])
        {
            let label: String = chars[i + 1..close].iter().collect();
            let url: String = chars[close + 2..end].iter().collect();
            if !url.is_empty() && !url.contains(' ') {
                flush(&mut plain, &mut out);
                out.push_str(&format!(
                    "<a href=\"{}\">{}</a>",
                    escape(&url),
                    inline(&label, mono)
                ));
                i = end + 1;
                continue;
            }
        }
        // Bare URLs.
        if (c == 'h')
            && (chars[i..].starts_with(&['h', 't', 't', 'p', ':', '/', '/'])
                || chars[i..].starts_with(&['h', 't', 't', 'p', 's', ':', '/', '/']))
            && !is_word(i.checked_sub(1).and_then(|p| chars.get(p)))
        {
            let mut end = i;
            while end < chars.len() && !chars[end].is_whitespace() {
                end += 1;
            }
            // Trailing punctuation is not part of the URL.
            while end > i && matches!(chars[end - 1], '.' | ',' | ')' | ';' | ':' | '!' | '?') {
                end -= 1;
            }
            let url: String = chars[i..end].iter().collect();
            flush(&mut plain, &mut out);
            out.push_str(&format!(
                "<a href=\"{}\">{}</a>",
                escape(&url),
                escape(&url)
            ));
            i = end;
            continue;
        }
        // **bold** / __bold__
        if (c == '*' || c == '_') && chars.get(i + 1) == Some(&c) {
            let pat = [c, c];
            if let Some(end) = find_from(&chars, i + 2, &pat)
                && end > i + 2
            {
                flush(&mut plain, &mut out);
                let inner: String = chars[i + 2..end].iter().collect();
                out.push_str(&format!("<b>{}</b>", inline(&inner, mono)));
                i = end + 2;
                continue;
            }
        }
        // *italic* / _italic_ (an underscore inside a word is not emphasis: snake_case).
        if (c == '*' || (c == '_' && !is_word(i.checked_sub(1).and_then(|p| chars.get(p)))))
            && chars.get(i + 1).is_some_and(|n| !n.is_whitespace())
            && let Some(end) = find_from(&chars, i + 1, &[c])
            && end > i + 1
            && !chars[end - 1].is_whitespace()
            && (c != '_' || !is_word(chars.get(end + 1)))
        {
            flush(&mut plain, &mut out);
            let inner: String = chars[i + 1..end].iter().collect();
            out.push_str(&format!("<i>{}</i>", inline(&inner, mono)));
            i = end + 1;
            continue;
        }
        plain.push(c);
        i += 1;
    }
    flush(&mut plain, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> String {
        to_pango(s, "Adwaita Mono")
    }

    #[test]
    fn inline_spans() {
        assert_eq!(p("a **b** c"), "a <b>b</b> c");
        assert_eq!(p("a *b* _c_"), "a <i>b</i> <i>c</i>");
        assert_eq!(
            p("run `cargo <test>`"),
            "run <span font_family=\"Adwaita Mono\">cargo &lt;test&gt;</span>"
        );
        assert_eq!(
            p("see [the docs](https://x.org/a?b=1&c=2)"),
            "see <a href=\"https://x.org/a?b=1&amp;c=2\">the docs</a>"
        );
        assert_eq!(
            p("at https://example.com/x."),
            "at <a href=\"https://example.com/x\">https://example.com/x</a>."
        );
        assert_eq!(p("**bold _and italic_**"), "<b>bold <i>and italic</i></b>");
    }

    #[test]
    fn plain_text_is_escaped_and_kept() {
        assert_eq!(p("a < b & c > d"), "a &lt; b &amp; c &gt; d");
        assert_eq!(p("snake_case_name"), "snake_case_name");
        assert_eq!(p("2 * 3 * 4"), "2 * 3 * 4");
        assert_eq!(p("unclosed **bold"), "unclosed **bold");
        assert_eq!(p("中文 **粗体**"), "中文 <b>粗体</b>");
    }

    #[test]
    fn blocks() {
        assert_eq!(
            p("Intro\n- one\n* two\n1. first\n12. twelfth\n# Head"),
            "Intro\n  •  one\n  •  two\n  1.  first\n  12.  twelfth\n<b>Head</b>"
        );
        assert_eq!(p("  - nested"), "    •  nested");
        assert_eq!(p("a\n\nb"), "a\n\nb");
    }
}
