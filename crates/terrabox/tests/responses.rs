use terrabox::{Event, InputMode, InputParser, Key, KeyEvent, Modifiers};
#[test]
fn replies_preserve_keys_and_paste_at_every_fragment_boundary() {
    let reply = b"\x1b_Gi=12;OK\x1b\\";
    for split in 0..=reply.len() {
        let mut p = InputParser::new();
        p.set_response_parsing(true);
        p.feed(b"x");
        p.feed(&reply[..split]);
        assert_eq!(
            p.next_event(),
            Some(Event::Key(KeyEvent::new(Key::Char('x'), Modifiers::NONE)))
        );
        let early = p.next_event();
        p.feed(&reply[split..]);
        p.feed(b"\x1b[A\x1b[200~literal\x1b_Gi=12;OK\x1b\\\x1b[201~");
        assert_eq!(
            early.or_else(|| p.next_event()),
            Some(Event::Response(reply.to_vec()))
        );
        assert!(matches!(p.next_event(),Some(Event::Key(k)) if k.key==Key::Up));
        assert!(
            matches!(p.next_event(),Some(Event::Paste(data)) if data.bytes==b"literal\x1b_Gi=12;OK\x1b\\")
        );
    }
}
#[test]
fn malformed_oversized_and_incomplete_replies_do_not_become_commands() {
    let mut p = InputParser::new();
    p.set_response_parsing(true);
    let mut bytes = b"\x1b_G".to_vec();
    bytes.extend(vec![b'q'; 10000]);
    bytes.extend_from_slice(b"\x1b\\z");
    p.feed(&bytes);
    let mut unknown = 0;
    while let Some(event) = p.next_event() {
        match event {
            Event::UnknownSequence(bytes) => {
                assert!(bytes.len() <= 4096);
                unknown += 1;
            }
            Event::Key(k) => assert_eq!(k.key, Key::Char('z')),
            _ => panic!("unexpected event"),
        }
    }
    assert!(unknown >= 2);
    p.feed(b"\x1b_Gi=1;unfinished\x1b[B");
    assert!(matches!(p.next_event(), Some(Event::UnknownSequence(_))));
    assert!(matches!(p.next_event(),Some(Event::Key(k)) if k.key==Key::Down));
    p.feed(b"\x1b_Gi=1;");
    assert!(p.next_event().is_none());
    assert!(matches!(
        p.finish_incomplete_sequence(),
        Some(Event::UnknownSequence(_))
    ));
    p.feed(b"a");
    assert!(matches!(p.next_event(),Some(Event::Key(k)) if k.key==Key::Char('a')));
}
#[test]
fn unrelated_alt_keys_and_csi_replies_are_distinct() {
    let mut p = InputParser::new();
    p.set_response_parsing(true);
    p.set_mode(InputMode::Alt);
    p.feed(b"\x1b_");
    assert!(p.next_event().is_none());
    assert_eq!(
        p.finish_incomplete_sequence(),
        Some(Event::Key(KeyEvent::new(Key::Char('_'), Modifiers::ALT)))
    );
    p.feed(b"\x1b[?2004;2$y\x1b[?0u\x1b[?62;4c");
    for _ in 0..3 {
        assert!(matches!(p.next_event(), Some(Event::Response(_))));
    }
}

#[test]
fn xtgettcap_responses_survive_fragmentation_without_changing_alt_p() {
    let reply = b"\x1bP1+r436f=323536\x1b\\";
    for split in 0..=reply.len() {
        let mut parser = InputParser::new();
        parser.set_response_parsing(true);
        parser.feed(&reply[..split]);
        let early = parser.next_event();
        parser.feed(&reply[split..]);
        assert_eq!(
            early.or_else(|| parser.next_event()),
            Some(Event::Response(reply.to_vec()))
        );
    }
    let mut parser = InputParser::new();
    parser.set_response_parsing(true);
    parser.set_mode(InputMode::Alt);
    parser.feed(b"\x1bP");
    assert!(parser.next_event().is_none());
    assert_eq!(
        parser.finish_incomplete_sequence(),
        Some(Event::Key(KeyEvent::new(Key::Char('P'), Modifiers::ALT)))
    );
}
