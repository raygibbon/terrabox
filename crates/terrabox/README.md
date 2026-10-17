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
Adapted termbox2 capability data is covered by THIRD_PARTY_NOTICES.md.
