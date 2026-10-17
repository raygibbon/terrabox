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
        (!self.lines.is_empty()).then(|| self.selected.min(self.lines.len() - 1))
    }

    pub fn select(&mut self, index: usize) {
        self.selected = index;
        self.reconcile();
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
        if let Event::Mouse(mouse) = event {
            if mouse.kind == terrabox::MouseKind::Scroll {
                match mouse.button {
                    terrabox::MouseButton::WheelUp => {
                        self.selected = self.selected.saturating_sub(3)
                    }
                    terrabox::MouseButton::WheelDown => {
                        self.selected = self.selected.saturating_add(3)
                    }
                    _ => return EventResult::default(),
                }
                self.reconcile();
                return EventResult::handled();
            }
            if mouse.kind == terrabox::MouseKind::Press
                && mouse.position.y > 0
                && usize::from(mouse.position.y) <= self.rows
            {
                self.selected = self.offset + usize::from(mouse.position.y - 1);
                self.reconcile();
                return EventResult::handled();
            }
        }
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
    menus: Vec<Menu>,
    row: usize,
    dropdown: Option<terrabox::Rect>,
}

impl MenuBar {
    /// Labels use `&` before a mnemonic, for example `&File`.
    pub fn dropdowns(menus: Vec<Menu>) -> Self {
        Self {
            items: menus
                .iter()
                .map(|m| (m.label.replace('&', ""), CommandId(0)))
                .collect(),
            menus,
            ..Self::default()
        }
    }
    pub fn wants_event(&self, event: &Event) -> bool {
        if self.active {
            return true;
        }
        match event {
            Event::Key(k) => {
                (!self.items.is_empty() && k.key == Key::Function(10))
                    || (k.modifiers == Modifiers::ALT
                        && self.menus.iter().any(|m| mnemonic(&m.label, k.key)))
                    || self
                        .menus
                        .iter()
                        .flat_map(|m| &m.entries)
                        .any(|e| e.enabled && e.shortcut == Some(*k))
            }
            Event::Mouse(m) => {
                m.position.y == 0 && m.kind == terrabox::MouseKind::Press && !self.items.is_empty()
            }
            _ => false,
        }
    }
    pub fn draw_dropdown(&mut self, canvas: &mut Canvas<'_>, theme: &Theme) {
        self.dropdown = None;
        if !self.active {
            return;
        }
        let Some(menu) = self.menus.get(self.selected) else {
            return;
        };
        let size = canvas.size();
        let x: u16 = self
            .items
            .iter()
            .take(self.selected)
            .map(|(label, _)| (terrabox::text_width(label) + 2).min(usize::from(u16::MAX)) as u16)
            .fold(0u16, u16::saturating_add);
        let width = menu
            .entries
            .iter()
            .map(|e| {
                terrabox::text_width(&e.label.replace('&', ""))
                    + 4
                    + e.shortcut.map(|k| shortcut_label(k).len() + 1).unwrap_or(0)
            })
            .max()
            .unwrap_or(12)
            .min(usize::from(size.width.saturating_sub(x))) as u16;
        let height = (menu.entries.len().saturating_add(2))
            .min(usize::from(size.height.saturating_sub(1))) as u16;
        let area = terrabox::Rect::new(x, 1, width, height);
        self.dropdown = Some(area);
        let mut view = canvas.clip(area);
        view.fill(theme.dialog);
        view.border(theme.dialog);
        for (i, entry) in menu
            .entries
            .iter()
            .enumerate()
            .take(usize::from(height.saturating_sub(2)))
        {
            let style = if i == self.row && entry.enabled {
                theme.selection
            } else {
                theme.dialog
            };
            let text = if entry.command.is_none() {
                "─".repeat(usize::from(width.saturating_sub(2)))
            } else {
                format!(
                    "{}{}",
                    if entry.enabled { " " } else { "×" },
                    entry.label.replace('&', "")
                )
            };
            view.clip(terrabox::Rect::new(
                1,
                i as u16 + 1,
                width.saturating_sub(2),
                1,
            ))
            .fill(style);
            let hint = entry.shortcut.map(shortcut_label).unwrap_or_default();
            let hint_width =
                terrabox::text_width(&hint).min(usize::from(width.saturating_sub(2))) as u16;
            draw_line(
                &mut view,
                1,
                i as u16 + 1,
                width.saturating_sub(2 + hint_width + u16::from(!hint.is_empty())),
                &text,
                style,
            );
            if !hint.is_empty() {
                draw_line(
                    &mut view,
                    width.saturating_sub(1 + hint_width),
                    i as u16 + 1,
                    hint_width,
                    &hint,
                    style,
                );
            }
        }
    }
    fn activate(&mut self) -> EventResult {
        if self.menus.is_empty() {
            if let Some((_, command)) = self.items.get(self.selected) {
                let command = *command;
                self.close();
                return EventResult::command(command);
            }
        } else if let Some(entry) = self
            .menus
            .get(self.selected)
            .and_then(|m| m.entries.get(self.row))
            && entry.enabled
            && let Some(command) = entry.command
        {
            self.close();
            return EventResult::command(command);
        }
        EventResult::handled()
    }
    fn move_row(&mut self, reverse: bool) {
        if let Some(menu) = self.menus.get(self.selected) {
            let count = menu.entries.len();
            for _ in 0..count {
                self.row = if reverse {
                    (self.row + count - 1) % count
                } else {
                    (self.row + 1) % count
                };
                if menu.entries[self.row].enabled && menu.entries[self.row].command.is_some() {
                    break;
                }
            }
        }
    }
    fn first_row(&mut self) {
        self.row = 0;
        if self
            .menus
            .get(self.selected)
            .and_then(|m| m.entries.first())
            .is_some_and(|e| !e.enabled || e.command.is_none())
        {
            self.move_row(false);
        }
    }
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
        if let Event::Mouse(mouse) = event {
            if mouse.kind != terrabox::MouseKind::Press {
                return EventResult::handled();
            }
            if mouse.position.y == 0 {
                let mut x = 0usize;
                for (i, (label, _)) in self.items.iter().enumerate() {
                    let end = x + terrabox::text_width(label) + 2;
                    if (x..end).contains(&usize::from(mouse.position.x)) {
                        self.selected = i;
                        self.active = true;
                        self.first_row();
                        break;
                    }
                    x = end;
                }
            } else if let Some(rect) = self.dropdown
                && rect.contains(mouse.position)
                && mouse.position.y > rect.y
                && mouse.position.y + 1 < rect.bottom()
            {
                self.row = usize::from(mouse.position.y - rect.y - 1);
                return self.activate();
            } else {
                self.close();
            }
            return EventResult::handled();
        }
        let Event::Key(key) = event else {
            return EventResult::default();
        };
        for entry in self.menus.iter().flat_map(|m| &m.entries) {
            if entry.enabled
                && entry.shortcut == Some(*key)
                && let Some(command) = entry.command
            {
                self.close();
                return EventResult::command(command);
            }
        }
        if key.modifiers == Modifiers::ALT
            && let Some(i) = self.menus.iter().position(|m| mnemonic(&m.label, key.key))
        {
            self.selected = i;
            self.active = true;
            self.first_row();
            return EventResult::handled();
        }
        if key.key == Key::Function(10) {
            self.active = !self.active;
            self.first_row();
        } else if self.active {
            match key.key {
                Key::Escape => self.close(),
                Key::Left | Key::Right if !self.items.is_empty() => {
                    self.selected = if key.key == Key::Left {
                        (self.selected + self.items.len() - 1) % self.items.len()
                    } else {
                        (self.selected + 1) % self.items.len()
                    };
                    self.first_row();
                }
                Key::Up => self.move_row(true),
                Key::Down => self.move_row(false),
                Key::Enter => return self.activate(),
                Key::Char(_) => {
                    if let Some(menu) = self.menus.get(self.selected)
                        && let Some(i) = menu.entries.iter().position(|e| {
                            e.enabled && e.command.is_some() && mnemonic(&e.label, key.key)
                        })
                    {
                        self.row = i;
                        return self.activate();
                    }
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
            EventResult::default()
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

mod controls;
pub use controls::*;

/// A dropdown of application commands. Separators have no command.
pub struct Menu {
    pub label: String,
    pub entries: Vec<MenuEntry>,
}
pub struct MenuEntry {
    pub label: String,
    pub command: Option<CommandId>,
    pub enabled: bool,
    pub shortcut: Option<terrabox::KeyEvent>,
}
impl MenuEntry {
    pub fn command(label: impl Into<String>, command: CommandId) -> Self {
        Self {
            label: label.into(),
            command: Some(command),
            enabled: true,
            shortcut: None,
        }
    }
    pub fn with_shortcut(mut self, key: terrabox::KeyEvent) -> Self {
        self.shortcut = Some(key);
        self
    }
    pub fn separator() -> Self {
        Self {
            label: String::new(),
            command: None,
            enabled: false,
            shortcut: None,
        }
    }
}
fn mnemonic(label: &str, key: Key) -> bool {
    let Key::Char(c) = key else {
        return false;
    };
    label
        .split_once('&')
        .and_then(|(_, tail)| tail.chars().next())
        .is_some_and(|m| m.eq_ignore_ascii_case(&c))
}

fn shortcut_label(key: terrabox::KeyEvent) -> String {
    let mut hint = String::new();
    for (modifier, label) in [
        (Modifiers::CTRL, "Ctrl+"),
        (Modifiers::ALT, "Alt+"),
        (Modifiers::SHIFT, "Shift+"),
    ] {
        if key.modifiers.contains(modifier) {
            hint.push_str(label);
        }
    }
    hint.push_str(&match key.key {
        Key::Char(c) => c.to_string(),
        Key::Function(n) => format!("F{n}"),
        other => format!("{other:?}"),
    });
    hint
}

#[cfg(feature = "kitty-graphics")]
mod image;
#[cfg(feature = "kitty-graphics")]
pub use image::ImageView;
