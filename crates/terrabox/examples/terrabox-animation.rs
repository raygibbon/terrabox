use std::{
    io,
    time::{Duration, Instant},
};
use terrabox::{Color, Event, FrameClock, Key, Rect, Style, Surface, Terminal};
fn main() -> io::Result<()> {
    let mut terminal = Terminal::new()?;
    let mut milliseconds = 60u64;
    let mut clock = FrameClock::new(Duration::from_millis(milliseconds), Instant::now()).unwrap();
    let mut frame = 0usize;
    let mut redraw = true;
    let mut elapsed = Duration::ZERO;
    let mut rendering = Duration::ZERO;
    loop {
        if let Some(timing) = clock.poll(Instant::now()) {
            frame += 1;
            elapsed = timing.elapsed;
            redraw = true;
        }
        if redraw {
            let started = Instant::now();
            let size = terminal.size();
            let mut surface = Surface::new(size);
            let style = Style::new(Color::Indexed(7), Color::Indexed(4));
            let mut c = surface.canvas(Rect::new(0, 0, size.width, size.height));
            c.fill(style);
            c.border(Style::new(Color::Indexed(14), Color::Indexed(4)));
            c.put_str(3, 1, "TerraBox / caller-driven cell animation", style);
            c.put_str(
                3,
                3,
                &format!(
                    "Frame {frame} · {} · {} ms target interval",
                    if clock.paused() { "PAUSED" } else { "RUNNING" },
                    milliseconds
                ),
                style,
            );
            c.put_str(
                3,
                4,
                &format!(
                    "Measured frame gap: {:.1} ms | previous render: {} us",
                    elapsed.as_secs_f64() * 1000.0,
                    rendering.as_micros()
                ),
                style,
            );
            let columns = size.width.saturating_sub(8);
            for x in 0..columns {
                let wave = ((f64::from(x) / 8.0 + frame as f64 / 5.0).sin() * 3.0 + 3.0) as u16;
                c.put_char(
                    x + 4,
                    10 + wave,
                    '●',
                    Style::new(Color::Indexed(10 + (x % 5) as u8), Color::Indexed(4)),
                );
                c.put_char(x + 4, 19, '·', style);
            }
            if columns > 0 {
                c.put_char(
                    4 + (frame % usize::from(columns)) as u16,
                    19,
                    '◆',
                    Style::new(Color::Indexed(11), Color::Indexed(4)),
                );
            }
            c.put_str(
                3,
                22,
                "Deadlines, frame measurements and optional pacing; no mandatory tick.",
                style,
            );
            c.put_str(
                3,
                size.height.saturating_sub(2),
                "Space: pause/resume | + / -: speed | Esc: quit",
                style,
            );
            terminal.draw_surface(&surface, Default::default());
            terminal.hide_cursor();
            terminal.present()?;
            rendering = started.elapsed();
            redraw = false;
        }
        let event = terminal.wait_event(clock.deadline().map(|d| d.remaining(Instant::now())))?;
        match event {
            Some(Event::Key(k)) if matches!(k.key, Key::Escape | Key::Char('q')) => break,
            Some(Event::Key(k)) if k.key == Key::Char(' ') => {
                if clock.paused() {
                    clock.resume(Instant::now());
                } else {
                    clock.pause();
                }
                redraw = true;
            }
            Some(Event::Key(k)) if matches!(k.key, Key::Char('+') | Key::Char('-')) => {
                milliseconds = if k.key == Key::Char('+') {
                    milliseconds.saturating_sub(10).max(10)
                } else {
                    (milliseconds + 10).min(500)
                };
                clock.set_interval(Duration::from_millis(milliseconds), Instant::now());
                redraw = true;
            }
            Some(Event::Resize(_)) => redraw = true,
            _ => {}
        }
    }
    Ok(())
}
