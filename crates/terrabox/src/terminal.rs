use crate::buffer::{Buffer, Slot};
use crate::platform::{Backend, Wake};
use crate::terminfo::Capabilities;
use crate::{Cell, Color, Event, InputMode, InputParser, Style, Surface};
use std::collections::VecDeque;
use std::fmt::Write as _;
use std::io;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Size {
    pub width: u16,
    pub height: u16,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Position {
    pub x: u16,
    pub y: u16,
}

/// Owns the terminal mode and restores it on drop.
/// Only one instance may exist per process, including while suspended.
pub struct Terminal {
    backend: Backend,
    back: Buffer,
    front: Vec<Slot>,
    invalid: bool,
    parser: InputParser,
    escape_since: Option<Instant>,
    sequence_since: Option<Instant>,
    suspended: bool,
    cursor: Option<Position>,
    last_cursor: Position,
    clear_style: Style,
    mouse_enabled: bool,
    caps: Capabilities,
    capabilities: crate::TerminalCapabilities,
    pending_events: VecDeque<Event>,
    #[cfg(feature = "kitty-graphics")]
    graphics: crate::graphics::GraphicsState,
}

impl Terminal {
    pub fn new() -> io::Result<Self> {
        let caps = Capabilities::load()?;
        let backend = Backend::new(caps.clone())?;
        Self::from_backend(backend, caps)
    }
    /// Connect to process stdin and stdout instead of the controlling terminal.
    #[cfg(unix)]
    pub fn from_stdio() -> io::Result<Self> {
        let caps = Capabilities::load()?;
        let backend = Backend::from_stdio(caps.clone())?;
        Self::from_backend(backend, caps)
    }
    #[cfg(unix)]
    pub fn from_fds(input: std::os::fd::OwnedFd, output: std::os::fd::OwnedFd) -> io::Result<Self> {
        let caps = Capabilities::load()?;
        let backend = Backend::from_fds(input, output, caps.clone())?;
        Self::from_backend(backend, caps)
    }
    #[cfg(unix)]
    pub fn from_tty_path(path: impl AsRef<std::path::Path>) -> io::Result<Self> {
        let tty = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)?;
        let input = std::os::fd::OwnedFd::from(tty.try_clone()?);
        let output = std::os::fd::OwnedFd::from(tty);
        Self::from_fds(input, output)
    }
    fn from_backend(backend: Backend, caps: Capabilities) -> io::Result<Self> {
        let mut parser = InputParser::new();
        for (seq, key) in &caps.keys {
            parser
                .add_key_sequence(seq, *key)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        }
        let size = backend.size()?;
        let back = Buffer::new(size);
        let front = back.cells.clone();
        let capabilities = crate::TerminalCapabilities::new(&caps);
        Ok(Self {
            capabilities,
            pending_events: VecDeque::new(),
            #[cfg(feature = "kitty-graphics")]
            graphics: crate::graphics::GraphicsState::default(),
            backend,
            back,
            front,
            invalid: true,
            parser,
            escape_since: None,
            sequence_since: None,
            suspended: false,
            cursor: None,
            last_cursor: Position::default(),
            clear_style: Style::default(),
            mouse_enabled: false,
            caps,
        })
    }
    #[cfg(feature = "kitty-graphics")]
    fn require_graphics(&mut self) -> io::Result<()> {
        if self.suspended {
            return Err(io::Error::other("terminal is suspended"));
        }
        if !self.graphics_available() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Kitty graphics is not confirmed or the transport is a multiplexer",
            ));
        }
        self.parser.set_response_parsing(true);
        Ok(())
    }
    #[cfg(feature = "kitty-graphics")]
    pub fn graphics_available(&self) -> bool {
        self.capabilities.kitty_graphics.support == crate::Support::Supported
            && !self.capabilities.multiplexer
    }
    #[cfg(feature = "kitty-graphics")]
    pub fn upload_image(
        &mut self,
        image: &crate::graphics::RgbaImage,
    ) -> io::Result<crate::graphics::ImageId> {
        self.require_graphics()?;
        self.graphics
            .upload(image, |bytes| self.backend.write(bytes))
    }
    #[cfg(feature = "kitty-graphics")]
    pub fn replace_image(
        &mut self,
        id: crate::graphics::ImageId,
        image: &crate::graphics::RgbaImage,
    ) -> io::Result<()> {
        self.require_graphics()?;
        self.graphics
            .replace(id, image, |bytes| self.backend.write(bytes))
    }
    #[cfg(feature = "kitty-graphics")]
    pub fn place_image(
        &mut self,
        id: crate::graphics::ImageId,
        area: crate::Rect,
    ) -> io::Result<crate::graphics::PlacementId> {
        self.require_graphics()?;
        self.graphics.place(id, area, self.size())
    }
    #[cfg(feature = "kitty-graphics")]
    pub fn move_image(
        &mut self,
        id: crate::graphics::PlacementId,
        area: crate::Rect,
    ) -> io::Result<()> {
        self.require_graphics()?;
        self.graphics.move_placement(id, area, self.size())
    }
    #[cfg(feature = "kitty-graphics")]
    pub fn remove_image_placement(&mut self, id: crate::graphics::PlacementId) -> io::Result<()> {
        self.graphics.remove_placement(id)
    }
    /// Queue deletion of this terminal's image and its placements. Call present.
    #[cfg(feature = "kitty-graphics")]
    pub fn delete_image(&mut self, id: crate::graphics::ImageId) -> io::Result<()> {
        self.graphics.delete(id)
    }
    /// Synchronise retained requests. Missing slots release their resources.
    #[cfg(feature = "kitty-graphics")]
    pub fn sync_graphics(&mut self, requests: &[crate::graphics::ImageRequest]) -> io::Result<()> {
        if !requests.is_empty() {
            self.require_graphics()?;
        }
        self.graphics
            .sync(requests, self.size(), |bytes| self.backend.write(bytes))
    }
    /// Release all owned images immediately. Handles become invalid. This also
    /// happens before suspend, raw output and terminal shutdown.
    #[cfg(feature = "kitty-graphics")]
    pub fn clear_graphics(&mut self) -> io::Result<()> {
        let bytes = self.graphics.cleanup();
        let result = self.backend.write(&bytes);
        self.graphics.clear();
        result
    }
    pub fn capabilities(&self) -> &crate::TerminalCapabilities {
        &self.capabilities
    }
    /// Explicit overrides are application assertions, not detection. Multiplexer
    /// safety restrictions still apply to graphics output.
    pub fn override_capability(&mut self, feature: crate::Capability, support: crate::Support) {
        self.capabilities.override_status(feature, support);
    }
    /// Explicit, cached probes. Silence remains Unknown. Keyboard/paste/resize
    /// events received while probing are replayed in their original order.
    /// Native Windows record input and multiplexers are not queried.
    pub fn probe_capabilities(
        &mut self,
        timeout: Duration,
    ) -> io::Result<&crate::TerminalCapabilities> {
        if timeout > Duration::from_secs(2) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "probe timeout exceeds two seconds",
            ));
        }
        if self.suspended {
            return Err(io::Error::other("terminal is suspended"));
        }
        if self.capabilities.probed || self.capabilities.multiplexer {
            return Ok(&self.capabilities);
        }
        self.capabilities.probed = true;
        #[cfg(unix)]
        {
            self.parser.set_response_parsing(true);
            #[allow(unused_mut)]
            let mut query =
                b"\x1b[?1006$p\x1b[?2004$p\x1b[?u\x1b[c\x1bP+q436f\x1b\\\x1bP+q524742\x1b\\"
                    .to_vec();
            #[cfg(feature = "kitty-graphics")]
            {
                let id = crate::graphics::allocate_id()?;
                self.capabilities.graphics_query = Some(id);
                query.extend_from_slice(
                    format!("\x1b_Ga=q,t=d,f=24,s=1,v=1,i={id};AAAA\x1b\\").as_bytes(),
                );
            }
            if let Err(error) = self.backend.write(&query) {
                self.capabilities.probed = false;
                return Err(error);
            }
            let deadline = Instant::now()
                .checked_add(timeout)
                .ok_or_else(|| io::Error::other("invalid probe deadline"))?;
            let mut events = VecDeque::new();
            let mut event_bytes = 0usize;
            let result: io::Result<()> = (|| {
                while Instant::now() < deadline {
                    match self.wait_event_inner(Some(
                        deadline.saturating_duration_since(Instant::now()),
                    ))? {
                        Some(Event::Response(bytes)) if self.capabilities.observe(&bytes) => {}
                        Some(event) => {
                            event_bytes += match &event {
                                Event::Paste(p) => p.bytes.len(),
                                Event::Response(b) | Event::UnknownSequence(b) => b.len(),
                                _ => 32,
                            };
                            events.push_back(event);
                            // Stop waiting early under input floods; replay buffered events
                            // and leave remaining bytes in the parser/OS queue.
                            if events.len() >= 4096 || event_bytes >= 1024 * 1024 {
                                break;
                            }
                        }
                        None => {}
                    }
                }
                Ok(())
            })();
            self.pending_events.extend(events);
            result?;
        }
        self.capabilities.finish_probe();
        Ok(&self.capabilities)
    }
    /// Wake a blocking `wait_event` from another thread.
    pub fn waker(&self) -> crate::Waker {
        self.backend.waker()
    }

    pub fn size(&self) -> Size {
        self.back.size
    }
    /// Unix descriptors that become readable for input and resize events.
    /// After waiting on either descriptor, call `try_event` to consume events.
    #[cfg(unix)]
    pub fn event_fds(&self) -> (std::os::fd::BorrowedFd<'_>, std::os::fd::BorrowedFd<'_>) {
        self.backend.event_fds()
    }
    pub fn clear(&mut self) {
        self.back.clear_with_style(self.clear_style);
    }
    pub fn set_clear_style(&mut self, style: Style) {
        self.clear_style = style;
    }
    pub fn clear_style(&self) -> Style {
        self.clear_style
    }
    pub fn clear_with_style(&mut self, style: Style) {
        self.back.clear_with_style(style);
    }
    /// Clear the physical screen and force the next `present` to redraw it.
    pub fn clear_screen(&mut self) -> io::Result<()> {
        #[cfg(feature = "kitty-graphics")]
        self.graphics.resize();
        self.invalid = true;
        self.backend.clear_screen()?;
        Ok(())
    }
    /// Send bytes directly to the terminal. The next presentation repaints the screen.
    pub fn send_raw(&mut self, bytes: &[u8]) -> io::Result<()> {
        #[cfg(feature = "kitty-graphics")]
        self.clear_graphics()?;
        self.invalid = true;
        self.backend.write(bytes)?;
        Ok(())
    }
    pub fn put_cell(&mut self, x: u16, y: u16, cell: Cell) {
        self.back.put_cell(x, y, cell);
    }
    /// Composite an opaque surface, skipping partial wide glyphs at the edge.
    pub fn draw_surface(&mut self, surface: &Surface, position: Position) {
        surface.copy_to(&mut self.back, position);
    }
    /// Returns the logical cell, or None for a continuation or out of bounds.
    pub fn cell(&self, x: u16, y: u16) -> Option<&Cell> {
        self.back.cell(x, y)
    }
    /// Change a cell's style, including its continuation when the cell is wide.
    pub fn set_cell_style(&mut self, x: u16, y: u16, style: Style) -> bool {
        self.back.set_cell_style(x, y, style)
    }
    /// Extend a cell with text when the result is one grapheme cluster.
    /// Returns false if the position is invalid or the text would split the cell.
    pub fn extend_cell(&mut self, x: u16, y: u16, suffix: &str) -> bool {
        self.back.extend_cell(x, y, suffix)
    }
    pub fn put_char(&mut self, x: u16, y: u16, ch: char, style: Style) {
        self.back.put_char(x, y, ch, style);
    }
    pub fn put_str(&mut self, x: u16, y: u16, text: &str, style: Style) {
        self.back.put_str(x, y, text, style);
    }
    pub fn invalidate(&mut self) {
        self.invalid = true;
    }
    pub fn show_cursor(&mut self) {
        self.cursor.get_or_insert(self.last_cursor);
    }
    pub fn hide_cursor(&mut self) {
        if let Some(position) = self.cursor {
            self.last_cursor = position;
        }
        self.cursor = None;
    }
    pub fn set_cursor(&mut self, x: u16, y: u16) {
        self.last_cursor = Position { x, y };
        self.cursor = Some(self.last_cursor);
    }
    pub fn cursor(&self) -> Option<Position> {
        self.cursor
    }
    pub fn set_input_mode(&mut self, mode: InputMode) {
        self.parser.set_mode(mode);
    }
    pub fn input_mode(&self) -> InputMode {
        self.parser.mode()
    }
    pub fn set_mouse_enabled(&mut self, enabled: bool) -> io::Result<()> {
        self.backend.set_mouse(enabled)?;
        self.mouse_enabled = enabled;
        Ok(())
    }
    /// Wait for the next event, including resize, without a polling loop.
    pub fn read_event(&mut self) -> io::Result<Event> {
        if self.suspended {
            return Err(io::Error::other("terminal is suspended"));
        }
        loop {
            if let Some(event) = self.wait_event(None)? {
                return Ok(event);
            }
        }
    }
    /// Wait up to `timeout` for an event. Incomplete escape sequences remain
    /// buffered across timeouts; only a lone ESC uses a 30 ms grace period.
    pub fn read_event_timeout(&mut self, timeout: Duration) -> io::Result<Option<Event>> {
        self.wait_event(Some(timeout))
    }
    pub fn try_event(&mut self) -> io::Result<Option<Event>> {
        self.wait_event(Some(Duration::ZERO))
    }
    /// Wait for input, timeout, or a notification. Notifications return `None`.
    pub fn wait_event(&mut self, timeout: Option<Duration>) -> io::Result<Option<Event>> {
        if self.suspended {
            return Ok(None);
        }
        if let Some(event) = self.pending_events.pop_front() {
            return Ok(Some(event));
        }
        self.wait_event_inner(timeout)
    }
    fn wait_event_inner(&mut self, timeout: Option<Duration>) -> io::Result<Option<Event>> {
        if self.suspended {
            return Ok(None);
        }
        let deadline = timeout.and_then(|d| Instant::now().checked_add(d));
        loop {
            if let Some(event) = self.parser.next_event() {
                if let Event::Response(bytes) = &event {
                    self.capabilities.observe(bytes);
                    #[cfg(feature = "kitty-graphics")]
                    self.graphics.reject(bytes);
                }
                self.escape_since = None;
                self.sequence_since = None;
                return Ok(Some(event));
            }
            if self.parser.pending_lone_escape() {
                let since = self.escape_since.get_or_insert_with(Instant::now);
                if since.elapsed() >= Duration::from_millis(30) {
                    self.escape_since = None;
                    return Ok(self.parser.finish_escape());
                }
            } else {
                self.escape_since = None;
            }
            if self.parser.pending_sequence() {
                let since = self.sequence_since.get_or_insert_with(Instant::now);
                if since.elapsed() >= Duration::from_millis(500) {
                    self.sequence_since = None;
                    return Ok(self.parser.finish_incomplete_sequence());
                }
            } else {
                self.sequence_since = None;
            }
            let remaining = deadline.map(|d| d.saturating_duration_since(Instant::now()));
            let wait = match (remaining, self.escape_since) {
                (Some(t), Some(since)) => {
                    Some(t.min(Duration::from_millis(30).saturating_sub(since.elapsed())))
                }
                (None, Some(since)) => {
                    Some(Duration::from_millis(30).saturating_sub(since.elapsed()))
                }
                (other, None) => other,
            };
            let wait = if let Some(since) = self.sequence_since {
                Some(
                    wait.unwrap_or(Duration::from_millis(500))
                        .min(Duration::from_millis(500).saturating_sub(since.elapsed())),
                )
            } else {
                wait
            };
            match self.backend.wait(wait)? {
                #[cfg(unix)]
                Wake::Input => {
                    let mut bytes = [0; 4096];
                    let n = self.backend.read(&mut bytes)?;
                    if n == 0 {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "terminal input closed",
                        ));
                    }
                    self.parser.feed(&bytes[..n]);
                    continue;
                }
                Wake::Resize => {
                    let size = self.backend.size()?;
                    if size != self.back.size {
                        #[cfg(feature = "kitty-graphics")]
                        self.graphics.resize();
                        self.back.resize(size);
                        self.back.clear_with_style(self.clear_style);
                        self.front = self.back.cells.clone();
                        self.invalid = true;
                        return Ok(Some(Event::Resize(size)));
                    }
                }
                #[cfg(windows)]
                Wake::Native => {
                    if let Some(event) = self.backend.take_event() {
                        return Ok(Some(event));
                    }
                }
                Wake::Notification => return Ok(None),
                Wake::Timeout => {}
            }
            if let Some(d) = deadline
                && Instant::now() >= d
            {
                if self.parser.pending_lone_escape()
                    && self
                        .escape_since
                        .is_some_and(|s| s.elapsed() >= Duration::from_millis(30))
                {
                    self.escape_since = None;
                    return Ok(self.parser.finish_escape());
                }
                return Ok(None);
            }
        }
    }
    /// Restore cooked mode and the original screen before running another program.
    pub fn suspend(&mut self) -> io::Result<()> {
        #[cfg(feature = "kitty-graphics")]
        let cleanup = self.clear_graphics();
        let result = self.backend.suspend();
        self.suspended = !self.backend.is_active();
        #[cfg(feature = "kitty-graphics")]
        cleanup?;
        result
    }
    /// Run a child on the real terminal, restoring this terminal even when
    /// spawning/waiting fails. Unix transfers foreground process-group control;
    /// Windows protects the parent from the child's console Ctrl+C events.
    pub fn run_command(
        &mut self,
        command: &mut std::process::Command,
    ) -> io::Result<std::process::ExitStatus> {
        let suspended = self.suspend();
        let result = suspended.and_then(|()| self.backend.run_command(command));
        let restored = self.resume();
        restored?;
        result
    }
    /// Re-enter raw alternate-screen mode and request a full redraw.
    /// Retains cells at the same size; a changed size clears the buffer.
    pub fn resume(&mut self) -> io::Result<()> {
        if !self.suspended {
            return Ok(());
        }
        self.backend.resume()?;
        self.suspended = false;
        let size = self.backend.size()?;
        if size != self.back.size {
            self.back.resize(size);
            self.back.clear_with_style(self.clear_style);
            self.front = self.back.cells.clone();
        }
        self.invalid = true;
        if self.mouse_enabled {
            self.backend.set_mouse(true)?;
        }
        Ok(())
    }
    /// Draw changed rows only. A failed write leaves the front buffer invalid.
    pub fn present(&mut self) -> io::Result<()> {
        if self.suspended {
            return Err(io::Error::other("terminal is suspended"));
        }
        let rows = self.back.dirty_rows(&self.front, self.invalid);
        #[cfg(feature = "kitty-graphics")]
        let mut out = self.graphics.prefix();
        #[cfg(not(feature = "kitty-graphics"))]
        let mut out = Vec::new();
        let width = usize::from(self.back.size.width);
        for y in rows {
            out.extend_from_slice(&self.caps.cursor_position(0, y)?);
            let mut style = None;
            for x in 0..width {
                let slot = &self.back.cells[usize::from(y) * width + x];
                if slot.continuation {
                    continue;
                }
                if style != Some(slot.cell.style) {
                    append_style(&mut out, slot.cell.style, &self.caps);
                    style = Some(slot.cell.style);
                }
                slot.cell.append_to(&mut out);
            }
        }
        out.extend_from_slice(&self.caps.reset);
        #[cfg(feature = "kitty-graphics")]
        out.extend(
            self.graphics
                .suffix(self.invalid, |x, y| self.caps.cursor_position(x, y))?,
        );
        append_cursor(&mut out, self.cursor, &self.caps)?;
        if let Err(error) = self.backend.write(&out) {
            self.invalid = true;
            return Err(error);
        }
        #[cfg(feature = "kitty-graphics")]
        self.graphics.commit();
        self.front.clone_from(&self.back.cells);
        self.invalid = false;
        Ok(())
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        #[cfg(feature = "kitty-graphics")]
        if !self.suspended {
            let _ = self.backend.write(&self.graphics.cleanup());
        }
    }
}

fn append_cursor(
    out: &mut Vec<u8>,
    cursor: Option<Position>,
    caps: &Capabilities,
) -> io::Result<()> {
    if let Some(pos) = cursor {
        out.extend_from_slice(&caps.cursor_position(pos.x, pos.y)?);
        out.extend_from_slice(&caps.show_cursor);
    } else {
        out.extend_from_slice(&caps.hide_cursor);
    }
    Ok(())
}
fn append_style(out: &mut Vec<u8>, style: Style, caps: &Capabilities) {
    out.extend_from_slice(&caps.reset);
    let mut color = String::new();
    append_color(&mut color, style.foreground, true);
    append_color(&mut color, style.background, false);
    if !color.is_empty() {
        out.extend_from_slice(b"\x1b[");
        out.extend_from_slice(&color.as_bytes()[1..]);
        out.push(b'm');
    }
    for (active, index) in [
        (style.bold, 0),
        (style.dim, 1),
        (style.underline, 2),
        (style.italic, 3),
        (style.blink, 4),
        (style.reverse, 5),
        (style.invisible, 6),
    ] {
        if active {
            out.extend_from_slice(&caps.style[index]);
        }
    }
    if style.strikeout {
        out.extend_from_slice(b"\x1b[9m");
    }
    if style.double_underline {
        out.extend_from_slice(b"\x1b[21m");
    }
    if style.overline {
        out.extend_from_slice(b"\x1b[53m");
    }
}
fn append_color(out: &mut String, color: Color, foreground: bool) {
    match color {
        Color::Default => {}
        Color::Indexed(i) => {
            if i < 16 {
                let base = if foreground { 30 } else { 40 };
                let code = if i < 8 {
                    base + u32::from(i)
                } else {
                    base + 60 + u32::from(i - 8)
                };
                write!(out, ";{code}").expect("String write");
            } else {
                write!(out, ";{};5;{i}", if foreground { 38 } else { 48 }).expect("String write");
            }
        }
        Color::Rgb(r, g, b) => {
            write!(out, ";{};2;{r};{g};{b}", if foreground { 38 } else { 48 })
                .expect("String write");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn colour_and_attribute_sequences() {
        let mut out = Vec::new();
        let caps = Capabilities::fallback("xterm").unwrap();
        let style = Style {
            foreground: Color::Indexed(9),
            background: Color::Rgb(1, 2, 3),
            italic: true,
            strikeout: true,
            ..Style::default()
        };
        append_style(&mut out, style, &caps);
        assert_eq!(out, b"\x1b[0m\x1b[91;48;2;1;2;3m\x1b[3m\x1b[9m");
        out.clear();
        append_style(
            &mut out,
            Style::new(Color::Indexed(5), Color::Default),
            &caps,
        );
        assert_eq!(out, b"\x1b[0m\x1b[35m");
        out.clear();
        append_style(
            &mut out,
            Style::new(Color::Indexed(234), Color::Default),
            &caps,
        );
        assert_eq!(out, b"\x1b[0m\x1b[38;5;234m");
    }
    #[test]
    fn cursor_sequence_follows_rendering() {
        let caps = Capabilities::fallback("xterm").unwrap();
        let mut out = b"\x1b[3;1Hdrawn".to_vec();
        append_cursor(&mut out, Some(Position { x: 10, y: 5 }), &caps).unwrap();
        assert!(out.ends_with(&[b"\x1b[6;11H".as_slice(), caps.show_cursor.as_slice()].concat()));
        let mut hidden = Vec::new();
        append_cursor(&mut hidden, None, &caps).unwrap();
        assert_eq!(hidden, caps.hide_cursor);
    }
}
