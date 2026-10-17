use std::{
    io,
    time::{Duration, Instant},
};
use terrabox::{Color, Event, FrameClock, Key, Rect, Style, Surface, Terminal};
mod support;
fn main() -> io::Result<()> {
    let mut terminal = Terminal::new()?;
    terminal.probe_capabilities(Duration::from_millis(250))?;
    let mut index = 0;
    let mut image = support::sample(index);
    let mut id = None;
    let mut placement = None;
    let mut visible = true;
    let mut failed = false;
    let mut clock = FrameClock::new(Duration::from_millis(250), Instant::now()).unwrap();
    clock.pause();
    let mut redraw = true;
    loop {
        if clock.poll(Instant::now()).is_some() {
            index = (index + 1) % 3;
            image = support::sample(index);
            if let Some(id) = id {
                terminal.replace_image(id, &image)?;
            }
            redraw = true;
        }
        if redraw {
            let size = terminal.size();
            let style = Style::new(Color::Indexed(7), Color::Indexed(4));
            let mut surface = Surface::new(size);
            let mut c = surface.canvas(Rect::new(0, 0, size.width, size.height));
            c.fill(style);
            c.border(style);
            c.put_str(3, 1, "TerraBox / Kitty RGBA image", style);
            c.put_str(
                3,
                3,
                &format!(
                    "Static graphics: {:?} · native animation: {:?}",
                    terminal.capabilities().kitty_graphics.support,
                    terminal.capabilities().kitty_animation.support
                ),
                style,
            );
            if !terminal.graphics_available() || failed {
                c.put_str(
                    3,
                    6,
                    "Text fallback: a successful Kitty query is required.",
                    style,
                );
                c.put_str(
                    3,
                    8,
                    "No image is claimed to be displayed on this terminal.",
                    style,
                );
                c.put_str(
                    3,
                    10,
                    "Procedural RGBA data is ready; no decoder or download required.",
                    style,
                );
            }
            c.put_str(
                3,
                size.height.saturating_sub(2),
                "Space/R: replace | D: delete | U: show | C: scheduled replacements | Esc: quit",
                style,
            );
            terminal.draw_surface(&surface, Default::default());
            if terminal.graphics_available()
                && !failed
                && visible
                && size.width > 8
                && size.height > 10
            {
                let image_id = if let Some(id) = id {
                    id
                } else {
                    let new_id = terminal.upload_image(&image)?;
                    id = Some(new_id);
                    new_id
                };
                let area = Rect::new(4, 5, size.width - 8, size.height - 9);
                if let Some(p) = placement {
                    terminal.move_image(p, area)?;
                } else {
                    placement = Some(terminal.place_image(image_id, area)?);
                }
            }
            terminal.hide_cursor();
            terminal.present()?;
            redraw = false;
        }
        match terminal.wait_event(clock.deadline().map(|d| d.remaining(Instant::now())))? {
            Some(Event::Key(k)) if matches!(k.key, Key::Escape | Key::Char('q')) => break,
            Some(Event::Key(k)) if matches!(k.key, Key::Char(' ') | Key::Char('r')) => {
                index = (index + 1) % 3;
                image = support::sample(index);
                if let Some(id) = id {
                    terminal.replace_image(id, &image)?;
                }
                redraw = true;
            }
            Some(Event::Key(k)) if k.key == Key::Char('d') => {
                if let Some(old) = id.take() {
                    terminal.delete_image(old)?;
                }
                placement = None;
                visible = false;
                clock.pause();
                redraw = true;
            }
            Some(Event::Key(k)) if k.key == Key::Char('u') => {
                visible = true;
                failed = false;
                redraw = true;
            }
            Some(Event::Key(k)) if k.key == Key::Char('c') => {
                if clock.paused() && terminal.graphics_available() && !failed {
                    clock.resume(Instant::now());
                } else {
                    clock.pause();
                }
                redraw = true;
            }
            Some(Event::Resize(_)) => {
                placement = None;
                redraw = true;
            }
            Some(Event::Response(bytes))
                if bytes.starts_with(b"\x1b_G") && !bytes.ends_with(b";OK\x1b\\") =>
            {
                id = None;
                placement = None;
                failed = true;
                clock.pause();
                redraw = true;
            }
            _ => {}
        }
    }
    terminal.clear_graphics()
}
