use terrabox::{Color, Style};

/// Semantic framework styles; applications supply their own content styles.
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub surface: Style,
    pub border: Style,
    pub focused: Style,
    pub selection: Style,
    pub bar: Style,
    pub dialog: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            surface: Style::new(Color::Indexed(7), Color::Indexed(4)),
            border: Style::new(Color::Indexed(7), Color::Indexed(4)),
            focused: Style::new(Color::Indexed(14), Color::Indexed(4)),
            selection: Style::new(Color::Indexed(0), Color::Indexed(6)),
            bar: Style::new(Color::Indexed(0), Color::Indexed(7)),
            dialog: Style::new(Color::Indexed(0), Color::Indexed(7)),
        }
    }
}
