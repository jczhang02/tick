//! gettext setup and helpers. Every user-visible string goes through these functions
//! (textdomain `tick`, catalogs in `po/`).

pub use gettextrs::{gettext, ngettext, pgettext};

/// The gettext domain of Tick (also set as the translation domain of every Blueprint file).
pub const DOMAIN: &str = "tick";

/// Where compiled catalogs are installed (`<localedir>/<lang>/LC_MESSAGES/tick.mo`).
const LOCALEDIR: &str = match option_env!("TICK_LOCALEDIR") {
    Some(dir) => dir,
    None => "/usr/share/locale",
};

/// Selects the user's locale and binds the `tick` domain. Call before any UI is built.
pub fn init() {
    gettextrs::setlocale(gettextrs::LocaleCategory::LcAll, "");
    let dir = std::env::var("TICK_LOCALEDIR").unwrap_or_else(|_| LOCALEDIR.to_owned());
    if let Err(e) = gettextrs::bindtextdomain(DOMAIN, dir) {
        eprintln!("tick: bindtextdomain failed: {e}");
    }
    if let Err(e) = gettextrs::bind_textdomain_codeset(DOMAIN, "UTF-8") {
        eprintln!("tick: bind_textdomain_codeset failed: {e}");
    }
    if let Err(e) = gettextrs::textdomain(DOMAIN) {
        eprintln!("tick: textdomain failed: {e}");
    }
}

/// Replaces `{name}` placeholders in a translated template.
///
/// Translations reorder placeholders freely (`{day} {mon}` becomes `{month} 月 {day} 日`), so
/// positional formatting is not used.
pub fn fill(template: &str, args: &[(&str, &str)]) -> String {
    let mut out = template.to_owned();
    for (key, value) in args {
        out = out.replace(&format!("{{{key}}}"), value);
    }
    out
}

/// `ngettext` with the count filled in as `{n}`.
pub fn ncount(singular: &str, plural: &str, n: usize) -> String {
    let n32 = u32::try_from(n).unwrap_or(u32::MAX);
    fill(&ngettext(singular, plural, n32), &[("n", &n.to_string())])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_replaces_named_placeholders() {
        assert_eq!(
            fill("{month} 月 {day} 日", &[("day", "28"), ("month", "11")]),
            "11 月 28 日"
        );
        assert_eq!(ncount("{n} task", "{n} tasks", 1), "1 task");
        assert_eq!(ncount("{n} task", "{n} tasks", 7), "7 tasks");
    }
}
