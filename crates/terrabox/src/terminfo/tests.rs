//! Independently authored protocol and binary-format tests. See the audit for sources.
use super::*;
use crate::{Event, InputParser, KeyEvent, Modifiers};

/// Build a synthetic entry from the six-section format described in term(5).
/// Payloads are chosen by these tests; no installed database is bundled.
fn compiled(magic: u16, slots: &[(usize, &[u8])]) -> Vec<u8> {
    let count = slots.iter().map(|(index, _)| index + 1).max().unwrap_or(0);
    let mut offsets = vec![u16::MAX; count];
    let mut strings = Vec::new();
    for &(index, bytes) in slots {
        offsets[index] = strings.len() as u16;
        strings.extend_from_slice(bytes);
        strings.push(0);
    }
    let name = b"terra-fixture\0";
    let mut out = Vec::new();
    for value in [
        magic,
        name.len() as u16,
        2,
        1,
        count as u16,
        strings.len() as u16,
    ] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.extend_from_slice(name);
    out.extend_from_slice(&[0, 1]);
    if out.len() % 2 != 0 {
        out.push(0);
    }
    if magic == 0x21e {
        out.extend_from_slice(&80i32.to_le_bytes());
    } else {
        out.extend_from_slice(&80i16.to_le_bytes());
    }
    for offset in offsets {
        out.extend_from_slice(&offset.to_le_bytes());
    }
    out.extend(strings);
    out
}

#[test]
fn database_formats_and_cancelled_fields() {
    for magic in [0x11a, 0x21e] {
        let mut bytes = compiled(
            magic,
            &[(5, b"clear fixture"), (10, b"row=%p1%02d,col=%p2%d")],
        );
        let entry = parse_entry(&bytes).unwrap();
        let caps = Capabilities::from_entry(&entry);
        assert_eq!(caps.clear, b"clear fixture");
        assert_eq!(caps.cursor_position(7, 3).unwrap(), b"row=03,col=7");
        assert!(caps.enter_screen.is_empty());
        assert!(caps.keys.is_empty());
        // The first absent offset becomes the standard cancelled sentinel (-2).
        let first_offset = bytes.len() - b"clear fixture\0row=%p1%02d,col=%p2%d\0".len() - 22;
        bytes[first_offset..first_offset + 2].copy_from_slice(&(-2i16).to_le_bytes());
        assert!(parse_entry(&bytes).unwrap()[0].is_none());
    }
}

#[test]
fn malformed_entries_and_cursor_programs_fail_safely() {
    let bytes = compiled(0x11a, &[(10, b"%p1%d")]);
    for end in 0..bytes.len() {
        assert!(parse_entry(&bytes[..end]).is_none());
    }
    let mut invalid = bytes.clone();
    invalid[2..4].copy_from_slice(&(-1i16).to_le_bytes());
    assert!(parse_entry(&invalid).is_none());
    let mut invalid = bytes.clone();
    let table = invalid.len() - b"%p1%d\0".len();
    invalid[table - 2..table].copy_from_slice(&32767i16.to_le_bytes());
    assert!(parse_entry(&invalid).is_none());
    assert!(parse_entry(&[0; 12]).is_none());
    for cup in [b"%p3%d".as_slice(), b"%{broken}", b"%p1%{0}%/%d"] {
        let entry = parse_entry(&compiled(0x11a, &[(5, b"clear"), (10, cup)])).unwrap();
        assert_eq!(
            Capabilities::resolve("xterm", Some(&entry))
                .err()
                .unwrap()
                .kind(),
            io::ErrorKind::InvalidData
        );
    }
}

#[test]
fn database_has_priority_and_missing_capabilities_do_not_inherit_fallbacks() {
    let entry = parse_entry(&compiled(
        0x11a,
        &[
            (5, b"database clear"),
            (10, b"pos:%p2%d/%p1%d"),
            (66, b"\x1b[99~"),
        ],
    ))
    .unwrap();
    let caps = Capabilities::resolve("xterm", Some(&entry)).unwrap();
    assert_eq!(caps.clear, b"database clear");
    assert_eq!(caps.cursor_position(6, 4).unwrap(), b"pos:6/4");
    assert!(caps.enter_screen.is_empty());
    assert!(caps.show_cursor.is_empty());
    assert!(caps.reset.is_empty());
    assert_eq!(caps.keys, [(b"\x1b[99~".to_vec(), Key::Function(1))]);
    assert!(Capabilities::resolve("custom-terminal", Some(&entry)).is_ok());
    let incomplete = vec![None; 42];
    assert!(Capabilities::resolve("xterm", Some(&incomplete)).is_err());
}

#[test]
fn missing_invalid_and_unknown_names_are_safe() {
    for term in [
        "",
        ".",
        "..",
        "../xterm",
        "xterm/evil",
        "xterm\0",
        "xterm\n",
        "λ",
    ] {
        assert_eq!(
            Capabilities::resolve(term, None).err().unwrap().kind(),
            io::ErrorKind::InvalidInput
        );
    }
    assert!(Capabilities::resolve(&"x".repeat(129), None).is_err());
    for term in [
        "dumb",
        "not-xterm",
        "xterminal",
        "linux-unrecognised",
        "unknown",
    ] {
        assert_eq!(
            Capabilities::resolve(term, None).err().unwrap().kind(),
            io::ErrorKind::Unsupported
        );
    }
}

#[test]
fn fallback_output_uses_documented_operations_without_family_key_tables() {
    for term in [
        "xterm",
        "xterm-256color",
        "tmux-256color",
        "screen",
        "rxvt-unicode",
        "xterm-kitty",
        "xterm-ghostty",
        "ghostty",
    ] {
        let caps = Capabilities::resolve(term, None).unwrap();
        assert_eq!(caps.enter_screen, b"\x1b[?1049h");
        assert_eq!(caps.exit_screen, b"\x1b[?1049l");
        assert_eq!(caps.show_cursor, b"\x1b[?25h");
        assert_eq!(caps.hide_cursor, b"\x1b[?25l");
        assert_eq!(caps.cursor_position(79, 23).unwrap(), b"\x1b[24;80H");
        assert_eq!(caps.clear, b"\x1b[H\x1b[2J");
        assert_eq!(caps.reset, b"\x1b[0m");
        assert!(caps.keys.is_empty());
        assert!(caps.enter_keypad.is_empty());
    }
    let linux = Capabilities::resolve("linux", None).unwrap();
    assert_eq!(linux.keys, [(vec![0x1b, b'\t'], Key::BackTab)]);
    let mut parser = InputParser::new();
    for (sequence, key) in &linux.keys {
        parser.add_key_sequence(sequence, *key).unwrap();
    }
    parser.feed(&[0x1b, b'\t']);
    assert_eq!(
        parser.next_event(),
        Some(Event::Key(KeyEvent::new(Key::BackTab, Modifiers::NONE)))
    );
    assert!(linux.enter_screen.is_empty());
    assert!(linux.exit_screen.is_empty());
    assert!(linux.style[3].is_empty());
    for term in ["rxvt", "rxvt-256color", "Eterm"] {
        let caps = Capabilities::resolve(term, None).unwrap();
        assert!(caps.enter_screen.windows(6).any(|s| s == b"\x1b[?47h"));
        assert!(caps.exit_screen.ends_with(b"\x1b8"));
    }
    let caps = fallback::output(fallback::Screen::SavedAlternate);
    assert_eq!(
        caps.style,
        [1, 2, 4, 3, 5, 7, 8].map(|n| format!("\x1b[{n}m").into_bytes())
    );
}

fn parse_key(bytes: &[u8], expected: Key, modifiers: Modifiers) {
    let mut parser = InputParser::new();
    parser.feed(bytes);
    assert_eq!(
        parser.next_event(),
        Some(Event::Key(KeyEvent::new(expected, modifiers))),
        "{bytes:?}"
    );
}

#[test]
fn documented_function_navigation_and_modified_key_protocols() {
    // Xterm ctlseqs: PF1–PF4 use SS3; editing/function keys use CSI Ps ~.
    for n in 1..=4 {
        parse_key(&[27, b'O', b'P' + n - 1], Key::Function(n), Modifiers::NONE);
    }
    for (n, parameter) in [
        (5, 15),
        (6, 17),
        (7, 18),
        (8, 19),
        (9, 20),
        (10, 21),
        (11, 23),
        (12, 24),
    ] {
        parse_key(
            format!("\x1b[{parameter}~").as_bytes(),
            Key::Function(n),
            Modifiers::NONE,
        );
    }
    for (suffix, key) in [
        (b'A', Key::Up),
        (b'B', Key::Down),
        (b'C', Key::Right),
        (b'D', Key::Left),
        (b'H', Key::Home),
        (b'F', Key::End),
    ] {
        for prefix in *b"[O" {
            parse_key(&[27, prefix, suffix], key, Modifiers::NONE);
        }
        parse_key(
            format!("\x1b[1;5{}", char::from(suffix)).as_bytes(),
            key,
            Modifiers::CTRL,
        );
    }
    for (parameter, key) in [
        (2, Key::Insert),
        (3, Key::Delete),
        (5, Key::PageUp),
        (6, Key::PageDown),
        (7, Key::Home),
        (8, Key::End),
    ] {
        parse_key(
            format!("\x1b[{parameter}~").as_bytes(),
            key,
            Modifiers::NONE,
        );
    }
    parse_key(b"\x1b[Z", Key::BackTab, Modifiers::SHIFT);
    parse_key(
        b"\x1b[24;6~",
        Key::Function(12),
        Modifiers::CTRL | Modifiers::SHIFT,
    );
    // Linux VC's five legacy keys; rxvt's numeric F1–F4 variants.
    for n in 1..=5 {
        parse_key(
            &[27, b'[', b'[', b'A' + n - 1],
            Key::Function(n),
            Modifiers::NONE,
        );
    }
    for n in 1..=4 {
        parse_key(
            format!("\x1b[{}~", 10 + n).as_bytes(),
            Key::Function(n),
            Modifiers::NONE,
        );
    }
    let mut parser = InputParser::new();
    parser.feed(b"\x1b[999~");
    assert!(matches!(
        parser.next_event(),
        Some(Event::UnknownSequence(_))
    ));
}

#[test]
fn arbitrary_terminfo_bytes_do_not_panic() {
    let mut state = 0x517cc1b727220a95u64;
    for len in 0..512 {
        let mut bytes = vec![0; len];
        for byte in &mut bytes {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            *byte = state as u8;
        }
        let _ = parse_entry(&bytes);
    }
}

#[cfg(not(windows))]
#[test]
fn directory_lookup_supports_both_layouts_and_rejects_corrupt_and_large_entries() {
    let root = std::env::temp_dir().join(format!(
        "terrabox-capability-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(root.join("78")).unwrap();
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(root.clone());
    let bytes = compiled(0x21e, &[(5, b"clear"), (10, b"%p1%d;%p2%d")]);
    fs::write(root.join("78/x-terra"), &bytes).unwrap();
    let dirs = [root.clone()];
    assert!(find_entry_in("x-terra", &dirs).is_some());
    fs::create_dir_all(root.join("x")).unwrap();
    fs::write(root.join("x/x-terra"), b"corrupt").unwrap();
    assert!(find_entry_in("x-terra", &dirs).is_some());
    fs::remove_file(root.join("78/x-terra")).unwrap();
    assert!(find_entry_in("x-terra", &dirs).is_none());
    fs::write(root.join("x/x-terra"), &bytes).unwrap();
    assert!(find_entry_in("x-terra", &dirs).is_some());
    let file = fs::File::create(root.join("x/x-huge")).unwrap();
    file.set_len(1024 * 1024 + 1).unwrap();
    assert!(find_entry_in("x-huge", &dirs).is_none());
    assert!(find_entry_in("../x-terra", &dirs).is_none());
}

#[cfg(not(windows))]
#[test]
fn actual_tic_compiler_confirms_all_consumed_standard_slots() {
    use std::process::Command;
    let root = std::env::temp_dir().join(format!(
        "terrabox-tic-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(root.clone());
    let source = root.join("synthetic.ti");
    fs::write(
        &source,
        include_bytes!("../../tests/fixtures/terra-capabilities.ti"),
    )
    .unwrap();
    match Command::new("tic")
        .arg("-x")
        .arg("-o")
        .arg(&root)
        .arg(&source)
        .output()
    {
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            eprintln!("tic unavailable: optional compiler integration check skipped");
            return;
        }
        Err(e) => panic!("tic: {e}"),
        Ok(result) => assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        ),
    }
    let entry = find_entry_in("terra-audit-fixture", &[root]).unwrap();
    let caps = Capabilities::resolve("terra-audit-fixture", Some(&entry)).unwrap();
    assert_eq!(caps.clear, b"ERASE");
    assert_eq!(caps.cursor_position(8, 2).unwrap(), b"ROW2COL8");
    assert_eq!(caps.enter_screen, b"ALT-ON");
    assert_eq!(caps.exit_screen, b"ALT-OFF");
    assert_eq!(caps.hide_cursor, b"HIDDEN");
    assert_eq!(caps.show_cursor, b"VISIBLE");
    assert_eq!(caps.reset, b"RESET");
    assert_eq!(caps.enter_keypad, b"KEYS-ON");
    assert_eq!(caps.exit_keypad, b"KEYS-OFF");
    assert_eq!(
        caps.style,
        [
            "BOLD",
            "DIM",
            "UNDERLINE",
            "ITALIC",
            "BLINK",
            "REVERSE",
            "INVISIBLE"
        ]
        .map(|s| s.as_bytes().to_vec())
    );
    assert_eq!(caps.keys.len(), 23);
    for n in 1..=12 {
        assert!(caps.keys.contains(&(
            format!("\x1b[{}~", 900 + u16::from(n)).into_bytes(),
            Key::Function(n)
        )));
    }
    for (n, key) in [
        (921, Key::Up),
        (922, Key::Down),
        (923, Key::Left),
        (924, Key::Right),
        (925, Key::Home),
        (926, Key::End),
        (927, Key::Insert),
        (928, Key::Delete),
        (929, Key::PageUp),
        (930, Key::PageDown),
        (931, Key::BackTab),
    ] {
        assert!(
            caps.keys
                .contains(&(format!("\x1b[{n}~").into_bytes(), key))
        );
    }
    let mut parser = InputParser::new();
    for (sequence, key) in &caps.keys {
        parser.add_key_sequence(sequence, *key).unwrap();
    }
    parser.feed(b"\x1b[912~");
    assert_eq!(
        parser.next_event(),
        Some(Event::Key(KeyEvent::new(
            Key::Function(12),
            Modifiers::NONE
        )))
    );
}

#[cfg(not(windows))]
#[test]
fn installed_family_databases_decode_their_registered_keys() {
    for term in [
        "xterm",
        "xterm-256color",
        "linux",
        "screen",
        "screen-256color",
        "tmux-256color",
        "rxvt",
        "rxvt-256color",
        "rxvt-unicode",
        "Eterm",
        "ghostty",
    ] {
        let Some(entry) = find_entry(term) else {
            eprintln!("{term}: no installed entry, database comparison skipped");
            continue;
        };
        let caps =
            Capabilities::resolve(term, Some(&entry)).unwrap_or_else(|e| panic!("{term}: {e}"));
        let mut parser = InputParser::new();
        for (sequence, key) in &caps.keys {
            parser.add_key_sequence(sequence, *key).unwrap();
        }
        for (sequence, key) in &caps.keys {
            parser.feed(sequence);
            assert_eq!(
                parser.next_event(),
                Some(Event::Key(KeyEvent::new(*key, Modifiers::NONE))),
                "{term}: {sequence:?}"
            );
        }
        assert!(!caps.cursor_position(79, 23).unwrap().is_empty());
    }
}
