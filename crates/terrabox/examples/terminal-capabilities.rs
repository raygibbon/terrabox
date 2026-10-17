use std::{io, time::Duration};
use terrabox::{Capability, Color, Event, Key, Rect, Style, Surface, Terminal};
fn main() -> io::Result<()> {
    let mut terminal = Terminal::new()?;
    terminal.probe_capabilities(Duration::from_millis(250))?;
    loop {
        let size = terminal.size();
        let mut surface = Surface::new(size);
        let style = Style::new(Color::Indexed(7), Color::Indexed(4));
        let mut c = surface.canvas(Rect::new(0, 0, size.width, size.height));
        c.fill(style);
        c.border(style);
        c.put_str(3, 1, "TerraBox / terminal capabilities", style);
        c.put_str(
            3,
            3,
            &format!(
                "TERM={} | graphics compiled: {}",
                std::env::var("TERM").unwrap_or_default(),
                cfg!(feature = "kitty-graphics")
            ),
            style,
        );
        c.put_str(
            3,
            4,
            &format!(
                "Multiplexer detected: {}",
                terminal.capabilities().multiplexer
            ),
            style,
        );
        c.put_str(
            3,
            6,
            "Feature                   Status        Evidence",
            Style::new(Color::Indexed(14), Color::Indexed(4)),
        );
        for (row, (name, feature)) in [
            ("True colour", Capability::TrueColor),
            ("256 colours", Capability::Colors256),
            ("Unicode glyphs", Capability::Unicode),
            ("Alternate screen", Capability::AlternateScreen),
            ("Mouse reporting", Capability::Mouse),
            ("Bracketed paste", Capability::BracketedPaste),
            ("Enhanced keyboard", Capability::EnhancedKeyboard),
            ("Kitty static images", Capability::KittyGraphics),
            ("Kitty native animation", Capability::KittyAnimation),
            ("Sixel", Capability::Sixel),
        ]
        .into_iter()
        .enumerate()
        {
            let status = terminal.capabilities().get(feature);
            c.put_str(
                3,
                8 + row as u16,
                &format!(
                    "{name:26}{:<14}{:?}",
                    format!("{:?}", status.support),
                    status.evidence
                ),
                style,
            );
        }
        c.put_str(
            3,
            20,
            "Silence stays Unknown. TERM branding does not confirm extensions.",
            style,
        );
        c.put_str(
            3,
            21,
            "Environment colour hints are displayed without claiming detection.",
            style,
        );
        c.put_str(
            3,
            23,
            "No native animation claim follows from static image support.",
            style,
        );
        c.put_str(
            3,
            size.height.saturating_sub(2),
            "Esc / Q: quit · resize: reflow · probes cached for this connection",
            style,
        );
        terminal.draw_surface(&surface, Default::default());
        terminal.hide_cursor();
        terminal.present()?;
        match terminal.read_event()? {
            Event::Key(k) if matches!(k.key, Key::Escape | Key::Char('q')) => break,
            _ => {}
        }
    }
    Ok(())
}
