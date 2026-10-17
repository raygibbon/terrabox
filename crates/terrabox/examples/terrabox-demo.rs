use std::{
    io,
    time::{Duration, Instant},
};
use terrabox::{Color, Event, Key, Position, Rect, Style, Surface, Terminal};
fn main() -> io::Result<()> {
    let mut terminal = Terminal::new()?;
    let started = Instant::now();
    let mut paused = false;
    let mut frame = 0u16;
    let mut last_input = String::from("Waiting for a key…");
    loop {
        let size = terminal.size();
        let mut surface = Surface::new(size);
        let style = Style::new(Color::Indexed(15), Color::Indexed(0));
        let mut canvas = surface.canvas(Rect::new(0, 0, size.width, size.height));
        canvas.fill(style);
        canvas.border(Style::new(Color::Indexed(14), Color::Indexed(0)));
        canvas.put_str(3, 1, "TerraBox / direct renderer", style);
        canvas.put_str(3, 3, "Unicode  界  e\u{301}  λ  ░▒▓█  ┌─┐", style);
        for i in 0..16 {
            let x = 3 + i * 5;
            canvas
                .clip(Rect::new(x, 5, 4, 2))
                .fill(Style::new(Color::Indexed(0), Color::Indexed(i as u8)));
        }
        canvas.put_str(3, 9, "Cell animation · no framework · native input", style);
        let width = size.width.saturating_sub(8).max(1);
        for x in 3..size.width.saturating_sub(4) {
            canvas.put_char(x, 12, '·', Style::new(Color::Indexed(8), Color::Indexed(0)));
        }
        canvas.put_char(
            3 + frame % width,
            12.min(size.height.saturating_sub(1)),
            '◆',
            Style::new(Color::Indexed(11), Color::Indexed(0)),
        );
        canvas.put_str(
            3,
            size.height.saturating_sub(3),
            &format!(
                "{}×{} | Space: pause | Esc: quit | {:.1}s",
                size.width,
                size.height,
                started.elapsed().as_secs_f32()
            ),
            style,
        );
        let mut notes = canvas.clip(Rect::new(3, 16, size.width.saturating_sub(6), 8));
        notes.border(Style::new(Color::Indexed(6), Color::Indexed(0)));
        notes.put_str(2, 1, "Low-level terminal foundations", style);
        notes.put_str(
            2,
            3,
            "Clipped canvases · grapheme cells · changed-row presentation",
            style,
        );
        notes.put_str(
            2,
            4,
            "Borders and colours use TerraBox drawing primitives",
            style,
        );
        notes.put_str(2, 6, &format!("Input: {last_input}"), style);
        terminal.draw_surface(&surface, Position::default());
        terminal.hide_cursor();
        terminal.present()?;
        let event = terminal.wait_event(if paused {
            None
        } else {
            Some(Duration::from_millis(60))
        })?;
        if let Some(Event::Key(k)) = &event {
            last_input = format!("{:?}", k.key);
        }
        match event {
            Some(Event::Key(k)) if matches!(k.key, Key::Escape | Key::Char('q')) => break,
            Some(Event::Key(k)) if k.key == Key::Char(' ') => paused = !paused,
            None if !paused => frame = frame.wrapping_add(1),
            _ => {}
        }
    }
    Ok(())
}
