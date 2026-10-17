# TerraBox engine guide

terrabox is a small, low-level, cell-oriented terminal library written entirely in Rust. It manages terminal initialization and restoration, raw input, an alternate screen, buffered cell rendering, Unicode graphemes, keyboard and mouse events, bracketed paste, resize events, colours, and terminal capabilities. It has native Unix and Windows backends.

```rust
use terrabox::{Event, Key, Style, Terminal};

fn main() -> std::io::Result<()> {
    let mut terminal = Terminal::new()?;
    loop {
        terminal.clear();
        terminal.put_str(2, 2, "Hello", Style::default());
        terminal.present()?;
        if matches!(terminal.read_event()?, Event::Key(key) if key.key == Key::Escape) {
            break;
        }
    }
    Ok(())
}
```

`Terminal::new()` opens the controlling terminal (`/dev/tty`) on Unix. It works when process stdout is redirected if a controlling terminal exists. `from_stdio()` uses process stdin and stdout explicitly; `from_fds()` accepts owned terminal descriptors. The terminal restores its previous mode on drop. `suspend()` and `resume()` allow temporary use of the normal screen; resume retains cells when the size is unchanged and clears them after a resize. Only one `Terminal` may exist per process, including while suspended. Restoration runs on normal drop and unwinding panics; aborts and forced termination bypass it. Applications must coordinate other code that changes terminal modes or signal handlers.

Drawing changes the back buffer. `present()` writes changed rows and restores the logical cursor position and visibility. `Cell::grapheme` stores a cluster; `put_str` segments text into clusters. `Color::Indexed(0..=15)` selects standard and bright colours, `Indexed(16..=255)` selects the extended palette, and `Rgb` selects true colour. Bold is a separate style attribute. `set_clear_style()` controls subsequent clears.

`read_event()` blocks, `read_event_timeout()` waits for a duration, and `try_event()` checks immediately. `InputMode::Escape` reports an unrecognized Escape separately; `InputMode::Alt` combines it with the following key. A lone Escape uses a 30 ms ambiguity period. Incomplete escape sequences, including Alt plus truncated UTF-8, expire after 500 ms as `UnknownSequence`. Paste data arrives in bounded chunks with a completion flag. Unix and Windows normalize control-key aliases where their input sources permit it. Raw Unix Ctrl+C is delivered as a key rather than a process signal.

`InputParser::add_key_sequence()` returns `Result<(), SequenceRegistrationError>` for explicit registration validation. Sequences must start with ESC, contain 2–64 bytes, and are matched longest first. Empty, oversized, non-ESC, and lone-ESC registrations return errors without changing existing registrations. Conflicting duplicate mappings produce `UnknownSequence`; repeating the same mapping is harmless. Bracketed-paste opening delimiters take precedence. `feed()` queues caller-supplied bytes, so drain events between bounded feeds; paste payload and unknown-sequence events are bounded to 1 MiB and 64 bytes respectively.

terrabox is synchronous and ends at the terminal abstraction. It is not a widget framework, layout engine, application framework, or async runtime. See [features](FEATURES.md) for platform details and current limits.

On Windows, use a console attached to stdin and stdout (Windows Terminal is recommended). Redirected streams are rejected. The backend owns duplicate console handles and permits one `Terminal` per process, including while suspended. Native C0 characters use the shared normalization: Ctrl+H is Backspace, Ctrl+I is Tab, Ctrl+J/Ctrl+M are Enter, and Ctrl+[ is Escape. Ctrl+\\ is `Char('\\')` with Ctrl. Ctrl+C arrives as a key; Ctrl+Break is consumed as a Windows signal while active, with no key event. Suspend/drop removes that signal handler. `InputMode` affects the byte parser, not native console records. Paste arrives as key records. See the [Windows validation report](WINDOWS_VALIDATION.md) for tested hosts, build commands, and native input details.

Build with `cargo build -p terrabox` using a normal Rust toolchain. No C compiler, terminal development headers, or external terminal utility is required. The crate is MIT licensed; dependencies retain their licences in [third-party notices](../THIRD_PARTY_NOTICES.md). The [capability audit](terminal-capability-audit.md) documents replacement of the historical tables.

### Foreground child lifecycle

`Terminal::run_command(&mut std::process::Command)` suspends terminal modes, runs
a child on the real terminal, waits, resumes and requests a full redraw. Unix
attaches controlling-terminal descriptors and transfers foreground process-group
ownership so Ctrl+C targets the child; Windows uses a temporary parent-only
console interrupt handler. Spawn/wait and restoration errors propagate. TDX's
PTY suite covers normal/nonzero exits, failed spawn, Ctrl+C and resize. A stopped
child (for example Ctrl+Z without resumption) is not a job-management UI.
