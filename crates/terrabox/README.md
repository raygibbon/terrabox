# TerraBox

A synchronous Rust terminal engine with native Unix and Windows backends,
RAII restoration, keyboard/mouse/paste/resize events, Unicode grapheme cells,
changed-row rendering, clipped surfaces and portable region scrolling.

Use this crate alone or with the optional higher-level `terratui` crate.
There are no widgets, application layouts or async runtime dependencies here.

```no_run
use terrabox::{Event, Key, Style, Terminal};

fn main() -> std::io::Result<()> {
    let mut terminal = Terminal::new()?;
    terminal.put_str(0, 0, "Hello TerraBox", Style::default());
    terminal.present()?;
    while !matches!(terminal.read_event()?, Event::Key(key) if key.key == Key::Escape) {}
    Ok(())
}
```

See the repository documentation for platform limitations and validation scope.
Dependency licences are recorded in THIRD_PARTY_NOTICES.md.

Unix prefers installed terminfo entries for cursor positioning, screen/cursor
modes, attributes and keyboard registrations. Its small specification-based
fallback supplies only core VT operations for recognised compatibility names;
unknown names without usable entries fail before terminal mode changes. Windows
uses native console input and documented VT output without a Unix database.
No ncurses dependency or terminal database is bundled. See the repository
`docs/terminal-capability-audit.md` for fallback details and provenance limits.

For off-screen drawing, create `Surface::new(size)` and obtain a local
`surface.canvas(Rect::new(x, y, width, height))`. Canvases clip nested views and
preserve wide glyphs at boundaries. Composite with `terminal.draw_surface`, then
`terminal.present`. Surface scrolling snapshots only the requested region.

Run `cargo run -p terrabox --example terrabox-demo` from the workspace for
colours, Unicode, drawing, resize and timed cell animation without TerraTUI.
`read_event` blocks without polling. `wait_event` accepts an optional timeout
and returns `None` for timeout or notification; `terminal.waker()` is cloneable
and thread-safe. Store messages separately, then wake after enqueueing them.
The convenience `read_event` keeps waiting through notifications.

This engine does not manage application lifecycle callbacks, widgets or focus.
Cell animation can use `Deadline` and timed waits; Kitty graphics, image
placement and advanced animation are planned and are not implemented.
