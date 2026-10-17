use crate::{CommandId, EventResult, Theme, Widget};
use terrabox::{Canvas, Event, Key, Modifiers};

/// A keyboard-selectable text panel; data and operations remain application-owned.
pub struct Panel {
    pub title: String,
    pub lines: Vec<String>,
    selected: usize,
    offset: usize,
    rows: usize,
}

impl Panel {
    pub fn new(title: impl Into<String>, lines: Vec<String>) -> Self {
        Self {
            title: title.into(),
            lines,
            selected: 0,
            offset: 0,
            rows: 0,
        }
    }

    pub fn selected(&self) -> Option<usize> {
        (!self.lines.is_empty()).then_some(self.selected.min(self.lines.len() - 1))
    }

    fn reconcile(&mut self) {
        self.selected = self.selected.min(self.lines.len().saturating_sub(1));
        self.offset = self.offset.min(self.selected);
        if self.rows > 0 && self.selected >= self.offset + self.rows {
            self.offset = self.selected + 1 - self.rows;
        }
    }
}

impl Widget for Panel {
    fn draw(&mut self, canvas: &mut Canvas<'_>, focused: bool, theme: &Theme) {
        canvas.fill(theme.surface);
        canvas.border(if focused { theme.focused } else { theme.border });
        let size = canvas.size();
        self.rows = usize::from(size.height.saturating_sub(2));
        self.reconcile();
        if size.width < 3 || size.height < 2 {
            return;
        }
        draw_line(
            canvas,
            1,
            0,
            size.width - 2,
            &self.title,
            if focused { theme.focused } else { theme.border },
        );
        for row in 0..self.rows {
            let index = self.offset + row;
            let style = if self.selected() == Some(index) && focused {
                theme.selection
            } else {
                theme.surface
            };
            for x in 1..size.width - 1 {
                canvas.put_char(x, row as u16 + 1, ' ', style);
            }
            if let Some(text) = self.lines.get(index) {
                draw_line(canvas, 1, row as u16 + 1, size.width - 2, text, style);
            }
        }
    }

    fn event(&mut self, event: &Event) -> EventResult {
        let Event::Key(key) = event else {
            return EventResult::default();
        };
        if key.modifiers != Modifiers::NONE {
            return EventResult::default();
        }
        self.reconcile();
        match key.key {
            Key::Up => self.selected = self.selected.saturating_sub(1),
            Key::Down => self.selected = self.selected.saturating_add(1),
            Key::Home => self.selected = 0,
            Key::End => self.selected = self.lines.len().saturating_sub(1),
            Key::PageUp => self.selected = self.selected.saturating_sub(self.rows.max(1)),
            Key::PageDown => self.selected = self.selected.saturating_add(self.rows.max(1)),
            _ => return EventResult::default(),
        }
        self.reconcile();
        EventResult::handled()
    }
}

fn draw_line(
    canvas: &mut Canvas<'_>,
    x: u16,
    y: u16,
    width: u16,
    text: &str,
    style: terrabox::Style,
) {
    canvas
        .clip(terrabox::Rect::new(x, y, width, 1))
        .put_str(0, 0, text, style);
}

/// A single horizontal command menu, activated by F10.
#[derive(Default)]
pub struct MenuBar {
    pub items: Vec<(String, CommandId)>,
    selected: usize,
    active: bool,
}

impl MenuBar {
    pub fn new(items: Vec<(String, CommandId)>) -> Self {
        Self {
            items,
            ..Self::default()
        }
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    pub fn is_active(&self) -> bool {
        self.active
    }
    pub fn close(&mut self) {
        self.active = false;
    }
}

impl Widget for MenuBar {
    fn draw(&mut self, canvas: &mut Canvas<'_>, _focused: bool, theme: &Theme) {
        canvas.fill(theme.bar);
        self.selected = self.selected.min(self.items.len().saturating_sub(1));
        let mut column = 0;
        for (index, (label, _)) in self.items.iter().enumerate() {
            let style = if self.active && index == self.selected {
                theme.selection
            } else {
                theme.bar
            };
            let text = format!(" {label} ");
            canvas.put_str(column, 0, &text, style);
            column = column
                .saturating_add(terrabox::text_width(&text).min(usize::from(u16::MAX)) as u16);
        }
    }

    fn event(&mut self, event: &Event) -> EventResult {
        let Event::Key(key) = event else {
            return EventResult::handled();
        };
        self.selected = self.selected.min(self.items.len().saturating_sub(1));
        if key.key == Key::Function(10) {
            self.active = !self.active;
        } else if self.active {
            match key.key {
                Key::Escape => self.close(),
                Key::Left if !self.items.is_empty() => {
                    self.selected = (self.selected + self.items.len() - 1) % self.items.len()
                }
                Key::Right if !self.items.is_empty() => {
                    self.selected = (self.selected + 1) % self.items.len()
                }
                Key::Enter if !self.items.is_empty() => {
                    self.close();
                    return EventResult::command(self.items[self.selected].1);
                }
                _ => {}
            }
        }
        EventResult::handled()
    }
}

/// A centred message dialog. Enter emits the supplied application command.
pub struct Dialog {
    pub title: String,
    pub message: String,
    pub accept: CommandId,
}

impl Widget for Dialog {
    fn draw(&mut self, canvas: &mut Canvas<'_>, _focused: bool, theme: &Theme) {
        canvas.fill(theme.dialog);
        canvas.border(theme.dialog);
        let size = canvas.size();
        if size.width < 3 || size.height < 3 {
            return;
        }
        draw_line(canvas, 1, 0, size.width - 2, &self.title, theme.dialog);
        for (row, text) in self
            .message
            .lines()
            .take(usize::from(size.height - 2))
            .enumerate()
        {
            draw_line(
                canvas,
                1,
                row as u16 + 1,
                size.width - 2,
                text,
                theme.dialog,
            );
        }
    }

    fn event(&mut self, event: &Event) -> EventResult {
        if matches!(event, Event::Key(key) if key.key == Key::Enter && key.modifiers == Modifiers::NONE)
        {
            EventResult::command(self.accept)
        } else {
            EventResult {
                consumed: true,
                ..EventResult::default()
            }
        }
    }
}

#[derive(Default)]
pub struct StatusBar {
    pub text: String,
}

impl Widget for StatusBar {
    fn draw(&mut self, canvas: &mut Canvas<'_>, _focused: bool, theme: &Theme) {
        canvas.fill(theme.bar);
        canvas.put_str(0, 0, &self.text, theme.bar);
    }
}
