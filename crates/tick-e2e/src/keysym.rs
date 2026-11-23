//! X11 keysyms for [`crate::Input`].

pub const BACKSPACE: u32 = 0xff08;
pub const TAB: u32 = 0xff09;
pub const RETURN: u32 = 0xff0d;
pub const ESCAPE: u32 = 0xff1b;
pub const DELETE: u32 = 0xffff;
pub const HOME: u32 = 0xff50;
pub const LEFT: u32 = 0xff51;
pub const UP: u32 = 0xff52;
pub const RIGHT: u32 = 0xff53;
pub const DOWN: u32 = 0xff54;
pub const PAGE_UP: u32 = 0xff55;
pub const PAGE_DOWN: u32 = 0xff56;
pub const END: u32 = 0xff57;
pub const F1: u32 = 0xffbe;
pub const F2: u32 = 0xffbf;
pub const F10: u32 = 0xffc7;
pub const SHIFT_L: u32 = 0xffe1;
pub const CONTROL_L: u32 = 0xffe3;
pub const ALT_L: u32 = 0xffe9;
pub const SUPER_L: u32 = 0xffeb;
pub const SPACE: u32 = 0x20;

/// Keysym that types `c`.
///
/// Printable Latin-1 maps to its code point (X11 keysyms match there), control keys to
/// their function keysyms, and everything else to a Unicode keysym
/// (`0x0100_0000 + code point`). Mutter drops keysyms outside its US keymap, so only
/// the ASCII results type anything; [`crate::Input::type_text`] enters other characters
/// through GTK's Unicode hex input.
pub fn from_char(c: char) -> u32 {
    match c {
        '\n' | '\r' => RETURN,
        '\t' => TAB,
        '\u{8}' => BACKSPACE,
        '\u{1b}' => ESCAPE,
        ' '..='~' | '\u{a0}'..='\u{ff}' => c as u32,
        _ => 0x0100_0000 + c as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_maps_to_code_point() {
        assert_eq!(from_char('a'), 0x61);
        assert_eq!(from_char('Q'), 0x51);
        assert_eq!(from_char('#'), 0x23);
        assert_eq!(from_char(' '), SPACE);
    }

    #[test]
    fn control_chars_map_to_function_keys() {
        assert_eq!(from_char('\n'), RETURN);
        assert_eq!(from_char('\t'), TAB);
    }

    #[test]
    fn other_chars_map_to_unicode_keysyms() {
        assert_eq!(from_char('é'), 0xe9);
        assert_eq!(from_char('今'), 0x0100_4eca);
        assert_eq!(from_char('€'), 0x0100_20ac);
    }
}
