//! Minimal output policy authored from protocol specifications, not family tables.
//!
//! Sources and compatibility limits: docs/terminal-capability-audit.md.
//! ECMA-48 supplies CUP, ED and SGR; xterm/Microsoft document DEC private modes.
use super::Capabilities;

#[cfg_attr(windows, allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Screen {
    Main,
    SavedAlternate,
    LegacyAlternate,
}

/// This is a compatibility allowlist, not feature detection. Prefer terminfo.
#[cfg(any(not(windows), test))]
pub(super) fn screen_policy(term: &str) -> Option<Screen> {
    let family =
        |name: &str| term == name || term.strip_prefix(name).is_some_and(|s| s.starts_with('-'));
    if term == "linux" {
        return Some(Screen::Main);
    }
    if family("rxvt-unicode") {
        return Some(Screen::SavedAlternate);
    }
    if family("rxvt") || family("Eterm") {
        return Some(Screen::LegacyAlternate);
    }
    if ["xterm", "screen", "tmux", "kitty", "ghostty"]
        .iter()
        .any(|name| family(name))
    {
        return Some(Screen::SavedAlternate);
    }
    None
}

fn private_mode(number: u16, enabled: bool) -> Vec<u8> {
    format!("\x1b[?{number}{}", if enabled { 'h' } else { 'l' }).into_bytes()
}
fn sgr(number: u8) -> Vec<u8> {
    format!("\x1b[{number}m").into_bytes()
}

pub(super) fn output(screen: Screen) -> Capabilities {
    let (enter_screen, exit_screen) = match screen {
        Screen::Main => (Vec::new(), Vec::new()),
        Screen::SavedAlternate => (private_mode(1049, true), private_mode(1049, false)),
        Screen::LegacyAlternate => {
            // DECSC/DECRC preserve cursor position around the older buffer switch.
            let mut enter = b"\x1b7".to_vec();
            enter.extend(private_mode(47, true));
            // ED removes stale alternate-buffer contents before the first draw.
            enter.extend_from_slice(b"\x1b[2J");
            let mut exit = private_mode(47, false);
            exit.extend_from_slice(b"\x1b8");
            (enter, exit)
        }
    };
    let mut style = [1, 2, 4, 3, 5, 7, 8].map(sgr);
    if screen == Screen::Main {
        // Conservative Linux VC subset; do not request emulator-only attributes.
        style[3].clear();
        style[6].clear();
    }
    Capabilities {
        // InputParser already decodes CSI/SS3 function and navigation keys in
        // both normal and application modes, including Linux VC encodings.
        keys: if screen == Screen::Main {
            // Linux kcbt (ESC HT), verified with the installed terminfo entry.
            // It needs registration to distinguish it from Escape + Tab.
            vec![(vec![0x1b, b'\t'], crate::Key::BackTab)]
        } else {
            Vec::new()
        },
        enter_screen,
        exit_screen,
        show_cursor: private_mode(25, true),
        hide_cursor: private_mode(25, false),
        clear: b"\x1b[H\x1b[2J".to_vec(),
        cursor_address: None,
        vt_cursor_fallback: true,
        reset: sgr(0),
        // Do not change keypad modes without a database entry describing them.
        enter_keypad: Vec::new(),
        exit_keypad: Vec::new(),
        style,
    }
}
