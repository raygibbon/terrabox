//! Small reader for compiled terminfo's standard string section.
use crate::Key;
mod expand;
mod fallback;
use std::io;
#[cfg(not(windows))]
use std::{
    env, fs,
    path::{Path, PathBuf},
};

// Standard compiled-string offsets, checked against ncurses term.h and term(5).
// These are ABI identifiers, not copied terminal descriptions.
#[cfg(any(not(windows), test))]
fn key_index(key: Key) -> usize {
    match key {
        Key::Function(1) => 66,
        Key::Function(10) => 67,
        Key::Function(n @ 2..=9) => 66 + usize::from(n),
        Key::Function(n @ 11..=12) => 205 + usize::from(n),
        Key::Delete => 59,
        Key::Down => 61,
        Key::Home => 76,
        Key::Insert => 77,
        Key::Left => 79,
        Key::PageDown => 81,
        Key::PageUp => 82,
        Key::Right => 83,
        Key::Up => 87,
        Key::BackTab => 148,
        Key::End => 164,
        _ => unreachable!("only database keys are requested"),
    }
}

#[derive(Clone)]
pub(crate) struct Capabilities {
    pub colors: Option<u32>,
    pub database: bool,
    pub keys: Vec<(Vec<u8>, Key)>,
    pub enter_screen: Vec<u8>,
    pub exit_screen: Vec<u8>,
    pub show_cursor: Vec<u8>,
    pub hide_cursor: Vec<u8>,
    pub clear: Vec<u8>,
    cursor_address: Option<Vec<u8>>,
    vt_cursor_fallback: bool,
    pub reset: Vec<u8>,
    #[cfg_attr(windows, allow(dead_code))]
    pub enter_keypad: Vec<u8>,
    #[cfg_attr(windows, allow(dead_code))]
    pub exit_keypad: Vec<u8>,
    /// bold, dim, underline, italic, blink, reverse, invisible
    pub style: [Vec<u8>; 7],
}
impl Capabilities {
    pub fn load() -> io::Result<Self> {
        #[cfg(windows)]
        {
            // The native Windows backend enables VT output; host TERM databases
            // (including Wine's) do not describe that console connection.
            Ok(fallback::output(fallback::Screen::SavedAlternate))
        }
        #[cfg(not(windows))]
        {
            let term = match env::var("TERM") {
                Ok(term) => term,
                Err(env::VarError::NotPresent) => {
                    return Err(io::Error::new(io::ErrorKind::NotFound, "TERM is not set"));
                }
                Err(env::VarError::NotUnicode(_)) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "TERM is not valid UTF-8",
                    ));
                }
            };
            if term.is_empty() {
                return Err(io::Error::new(io::ErrorKind::NotFound, "TERM is not set"));
            }
            let entry = find_entry(&term);
            let mut caps = Self::resolve(&term, entry.as_deref())?;
            caps.colors = entry.and_then(|entry| entry.colors);
            Ok(caps)
        }
    }
    #[cfg(any(not(windows), test))]
    pub(crate) fn from_entry(db: &[Option<Vec<u8>>]) -> Self {
        let mut result = Self {
            colors: None,
            database: true,
            keys: Vec::new(),
            enter_screen: Vec::new(),
            exit_screen: Vec::new(),
            show_cursor: Vec::new(),
            hide_cursor: Vec::new(),
            clear: Vec::new(),
            cursor_address: db.get(10).and_then(|value| value.clone()),
            vt_cursor_fallback: false,
            reset: Vec::new(),
            enter_keypad: Vec::new(),
            exit_keypad: Vec::new(),
            style: std::array::from_fn(|_| Vec::new()),
        };
        let keys = (1..=12).map(Key::Function).chain([
            Key::Up,
            Key::Right,
            Key::Down,
            Key::Left,
            Key::Home,
            Key::End,
            Key::Insert,
            Key::Delete,
            Key::PageUp,
            Key::PageDown,
            Key::BackTab,
        ]);
        for key in keys {
            let index = key_index(key);
            if let Some(bytes) = db.get(index).and_then(|v| v.as_ref()) {
                result.keys.push((bytes.clone(), key));
            }
        }
        for (field, index) in [
            (&mut result.enter_screen, 28),
            (&mut result.exit_screen, 40),
            (&mut result.show_cursor, 16),
            (&mut result.hide_cursor, 13),
            (&mut result.clear, 5),
            (&mut result.reset, 39),
            (&mut result.enter_keypad, 89),
            (&mut result.exit_keypad, 88),
        ] {
            if let Some(bytes) = db.get(index).and_then(|v| v.as_ref()) {
                *field = bytes.clone();
            }
        }
        for (slot, index) in [27, 30, 36, 311, 26, 34, 32].into_iter().enumerate() {
            if let Some(bytes) = db.get(index).and_then(|v| v.as_ref()) {
                result.style[slot] = bytes.clone();
            }
        }
        result
    }
    #[cfg(any(not(windows), test))]
    fn resolve(term: &str, db: Option<&[Option<Vec<u8>>]>) -> io::Result<Self> {
        if !valid_term(term) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "TERM must be a nonempty terminal name of at most 128 ASCII bytes",
            ));
        }
        if let Some(db) = db {
            let caps = Self::from_entry(db);
            // Fail before raw mode/alternate screen if basic rendering cannot work.
            if caps.cursor_position(0, 0)?.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "cursor positioning capability is empty",
                ));
            }
            if caps.clear.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "screen clearing capability is absent",
                ));
            }
            return Ok(caps);
        }
        Self::fallback(term).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Unsupported,
                format!("no usable terminfo entry or documented fallback for TERM={term}"),
            )
        })
    }
    #[cfg(any(not(windows), test))]
    pub(crate) fn fallback(term: &str) -> Option<Self> {
        fallback::screen_policy(term).map(fallback::output)
    }
    pub fn cursor_position(&self, x: u16, y: u16) -> io::Result<Vec<u8>> {
        if let Some(template) = &self.cursor_address {
            return expand::expand(template, [i32::from(y), i32::from(x)])
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error));
        }
        if self.vt_cursor_fallback {
            return Ok(format!("\x1b[{};{}H", u32::from(y) + 1, u32::from(x) + 1).into_bytes());
        }
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "cursor positioning capability is absent",
        ))
    }
}
#[cfg(any(not(windows), test))]
fn valid_term(term: &str) -> bool {
    !term.is_empty()
        && term != "."
        && term != ".."
        && term.len() <= 128
        && term
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._+-".contains(&b))
}
#[cfg(not(windows))]
fn find_entry(term: &str) -> Option<Entry> {
    if !valid_term(term) {
        return None;
    }
    let mut dirs = Vec::<PathBuf>::new();
    if let Some(path) = env::var_os("TERMINFO") {
        dirs.push(path.into());
    }
    if let Some(home) = env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join(".terminfo"));
    }
    if let Some(paths) = env::var_os("TERMINFO_DIRS") {
        for path in env::split_paths(&paths) {
            if path.as_os_str().is_empty() {
                dirs.push(PathBuf::from("/usr/share/terminfo"));
                dirs.push(PathBuf::from("/usr/lib/terminfo"));
            } else {
                dirs.push(path);
            }
        }
    }
    #[cfg(target_os = "android")]
    if let Some(prefix) = env::var_os("PREFIX") {
        dirs.push(PathBuf::from(prefix).join("share/terminfo"));
    }
    for path in [
        "/usr/local/etc/terminfo",
        "/usr/local/share/terminfo",
        "/usr/local/lib/terminfo",
        "/etc/terminfo",
        "/usr/share/terminfo",
        "/usr/lib/terminfo",
        "/usr/share/lib/terminfo",
        "/lib/terminfo",
    ] {
        dirs.push(path.into());
    }
    find_entry_in(term, &dirs)
}
#[cfg(not(windows))]
fn find_entry_in(term: &str, dirs: &[PathBuf]) -> Option<Entry> {
    if !valid_term(term) {
        return None;
    }
    for dir in dirs {
        let first = term.as_bytes()[0];
        for subdir in [
            (first as char).to_string(),
            format!("{first:x}"),
            format!("{first:02x}"),
        ] {
            let path = dir.join(subdir).join(term);
            if let Ok(bytes) = read_limited(&path)
                && let Some(entry) = parse_entry(&bytes)
            {
                return Some(entry);
            }
        }
    }
    None
}
#[cfg(not(windows))]
fn read_limited(path: &Path) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    const LIMIT: u64 = 1024 * 1024;
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > LIMIT {
        return Err(io::Error::other("terminfo entry too large"));
    }
    Ok(bytes)
}
#[cfg(any(not(windows), test))]
pub(crate) struct Entry {
    strings: Vec<Option<Vec<u8>>>,
    colors: Option<u32>,
}
#[cfg(any(not(windows), test))]
impl std::ops::Deref for Entry {
    type Target = [Option<Vec<u8>>];
    fn deref(&self) -> &Self::Target {
        &self.strings
    }
}
#[cfg(any(not(windows), test))]
pub(crate) fn parse_entry(bytes: &[u8]) -> Option<Entry> {
    fn n(bytes: &[u8], off: usize) -> Option<usize> {
        Some(usize::from(u16::from_le_bytes(
            bytes.get(off..off + 2)?.try_into().ok()?,
        )))
    }
    let magic = n(bytes, 0)?;
    let int_width = match magic {
        0x11a => 2,
        0x21e => 4,
        _ => return None,
    };
    let names = n(bytes, 2)?;
    let bools = n(bytes, 4)?;
    let ints = n(bytes, 6)?;
    let count = n(bytes, 8)?;
    let strings = n(bytes, 10)?;
    if [names, bools, ints, count, strings]
        .iter()
        .any(|n| *n > i16::MAX as usize)
    {
        return None;
    }
    let offsets_at = 12usize
        .checked_add(names)?
        .checked_add(bools)?
        .checked_add((names + bools) & 1)?
        .checked_add(ints.checked_mul(int_width)?)?;
    let table_at = offsets_at.checked_add(count.checked_mul(2)?)?;
    let end = table_at.checked_add(strings)?;
    if end > bytes.len() || count > 4096 {
        return None;
    }
    let mut result = Vec::with_capacity(count);
    for i in 0..count {
        let off = n(bytes, offsets_at + i * 2)?;
        if off == u16::MAX as usize || off == u16::MAX as usize - 1 {
            result.push(None);
            continue;
        }
        let start = table_at.checked_add(off)?;
        if off > i16::MAX as usize || start >= end {
            return None;
        }
        let len = bytes[start..end].iter().position(|b| *b == 0)?;
        result.push(Some(bytes[start..start + len].to_vec()));
    }
    let numbers_at = 12 + names + bools + ((names + bools) & 1);
    let colors = if ints > 13 {
        let offset = numbers_at + 13 * int_width;
        let value = if int_width == 2 {
            i32::from(i16::from_le_bytes(
                bytes.get(offset..offset + 2)?.try_into().ok()?,
            ))
        } else {
            i32::from_le_bytes(bytes.get(offset..offset + 4)?.try_into().ok()?)
        };
        u32::try_from(value).ok()
    } else {
        None
    };
    Some(Entry {
        strings: result,
        colors,
    })
}
#[cfg(test)]
mod tests;
