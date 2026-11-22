//! The Latin-first CJK font stack (`DESIGN.md` 2.3). Colors live in `style.css`, which
//! libadwaita loads from the resource base path; this provider only sets font families.

use std::cell::RefCell;

thread_local! {
    static PROVIDER: RefCell<Option<gtk::CssProvider>> = const { RefCell::new(None) };
}

/// The family part of a Pango font description string ("Adwaita Sans 11" -> "Adwaita Sans").
fn family_of(font_name: &str) -> String {
    let desc = gtk::pango::FontDescription::from_string(font_name);
    desc.family()
        .map(|f| f.to_string())
        .filter(|f| !f.is_empty())
        .unwrap_or_else(|| "Adwaita Sans".to_owned())
}

fn css_for(family: &str) -> String {
    let family = family.replace('"', "");
    format!(
        "window, popover, tooltip, dialog {{ font-family: \"{family}\", \"Noto Sans CJK SC\", \
         \"Source Han Sans SC\", \"Noto Sans SC\", sans-serif; }}"
    )
}

fn apply(settings: &gtk::Settings) {
    let family = family_of(&settings.gtk_font_name().unwrap_or_default());
    PROVIDER.with(|p| {
        let Some(display) = gtk::gdk::Display::default() else {
            return;
        };
        if let Some(old) = p.borrow_mut().take() {
            gtk::style_context_remove_provider_for_display(&display, &old);
        }
        let provider = gtk::CssProvider::new();
        provider.load_from_string(&css_for(&family));
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        p.replace(Some(provider));
    });
}

/// Install the font provider and rebuild it when the UI font changes (7.4 pitfall 13).
pub fn install_font_provider() {
    let Some(settings) = gtk::Settings::default() else {
        return;
    };
    apply(&settings);
    settings.connect_gtk_font_name_notify(apply);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_puts_the_ui_font_first() {
        assert_eq!(
            css_for("Inter"),
            "window, popover, tooltip, dialog { font-family: \"Inter\", \"Noto Sans CJK SC\", \
             \"Source Han Sans SC\", \"Noto Sans SC\", sans-serif; }"
        );
    }
}
