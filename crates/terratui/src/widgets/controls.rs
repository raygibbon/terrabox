use crate::{CommandId, EventResult, Layout, Theme, Widget, WindowId};
use terrabox::{Canvas, Event, Key, Modifiers, Rect};

pub(crate) fn tab_direction(event: &Event) -> Option<bool> {
    match event {
        Event::Key(key)
            if key.key == Key::BackTab
                && (key.modifiers == Modifiers::NONE || key.modifiers == Modifiers::SHIFT) =>
        {
            Some(true)
        }
        Event::Key(key)
            if key.key == Key::Tab
                && (key.modifiers == Modifiers::NONE || key.modifiers == Modifiers::SHIFT) =>
        {
            Some(key.modifiers == Modifiers::SHIFT)
        }
        _ => None,
    }
}

/// Cycle traps focus; Bubble hands traversal at an edge to the parent.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum TabBehavior {
    Cycle,
    #[default]
    Bubble,
    Manual,
}

/// A focus scope. Nest groups to build windows containing independent controls.
#[derive(Default)]
pub struct Group {
    pub layout: Layout,
    pub tab: TabBehavior,
    children: Vec<(WindowId, Box<dyn Widget>, bool)>,
    active: Option<WindowId>,
    areas: Vec<(WindowId, Rect)>,
}
impl Group {
    pub fn add(&mut self, widget: impl Widget, focusable: bool) -> WindowId {
        let id = WindowId(self.children.len() as u64 + 1);
        self.children.push((id, Box::new(widget), focusable));
        if self.active.is_none() && focusable {
            self.active = Some(id);
        }
        id
    }
    pub fn focused(&self) -> Option<WindowId> {
        self.active
    }
    pub fn widget_mut<T: Widget>(&mut self, id: WindowId) -> Option<&mut T> {
        let (_, widget, _) = self
            .children
            .iter_mut()
            .find(|(target, _, _)| *target == id)?;
        (widget.as_mut() as &mut dyn std::any::Any).downcast_mut()
    }
    pub fn focus(&mut self, id: WindowId) -> bool {
        if self
            .children
            .iter()
            .any(|(target, _, focusable)| *target == id && *focusable)
        {
            self.active = Some(id);
            true
        } else {
            false
        }
    }
}
impl Widget for Group {
    fn draw(&mut self, canvas: &mut Canvas<'_>, focused: bool, theme: &Theme) {
        let size = canvas.size();
        self.areas = self
            .layout
            .arrange(Rect::new(0, 0, size.width, size.height));
        let visible: Vec<_> = self
            .children
            .iter()
            .filter(|(id, _, focusable)| {
                *focusable
                    && self
                        .areas
                        .iter()
                        .any(|(target, r)| target == id && r.width > 0 && r.height > 0)
            })
            .map(|(id, _, _)| *id)
            .collect();
        if !self.active.is_some_and(|id| visible.contains(&id)) {
            self.active = visible.first().copied();
        }
        for (id, widget, _) in &mut self.children {
            if let Some((_, area)) = self.areas.iter().find(|(target, _)| target == id) {
                widget.draw(
                    &mut canvas.clip(*area),
                    focused && self.active == Some(*id),
                    theme,
                );
            }
        }
    }
    fn event(&mut self, event: &Event) -> EventResult {
        let previous_focus = self.active;
        let mut target = self.active;
        let mut local = event.clone();
        if let Event::Mouse(mouse) = event {
            let Some((id, area)) = self.areas.iter().find(|(_, r)| r.contains(mouse.position))
            else {
                return EventResult::default();
            };
            let (id, area) = (*id, *area);
            target = Some(id);
            if mouse.kind == terrabox::MouseKind::Press {
                self.focus(id);
            }
            let mut mouse = *mouse;
            mouse.position.x -= area.x;
            mouse.position.y -= area.y;
            local = Event::Mouse(mouse);
        }
        let mut result = self
            .children
            .iter_mut()
            .find(|(id, _, _)| Some(*id) == target)
            .map(|(_, w, _)| w.event(&local))
            .unwrap_or_default();
        result.redraw |= previous_focus != self.active;
        if result.consumed {
            return result;
        }
        if let Some(reverse) = tab_direction(event) {
            if self.tab == TabBehavior::Manual {
                return result;
            }
            let order: Vec<_> = self
                .children
                .iter()
                .filter(|(id, _, focusable)| {
                    *focusable
                        && (self.areas.is_empty()
                            || self
                                .areas
                                .iter()
                                .any(|(target, r)| target == id && r.width > 0 && r.height > 0))
                })
                .map(|(id, _, _)| *id)
                .collect();
            if let Some(index) = order.iter().position(|id| Some(*id) == self.active) {
                let edge = if reverse {
                    index == 0
                } else {
                    index + 1 == order.len()
                };
                if edge && self.tab == TabBehavior::Bubble {
                    return result;
                }
                let next = if reverse {
                    (index + order.len() - 1) % order.len()
                } else {
                    (index + 1) % order.len()
                };
                self.active = Some(order[next]);
                return EventResult::handled();
            }
        }
        result
    }
}

pub struct Label(pub String);
impl Widget for Label {
    fn draw(&mut self, canvas: &mut Canvas<'_>, _: bool, theme: &Theme) {
        canvas.fill(theme.surface);
        canvas.put_str(0, 0, &self.0, theme.surface);
    }
}
pub struct Button {
    pub label: String,
    pub command: CommandId,
    pub enabled: bool,
}
impl Button {
    pub fn new(label: impl Into<String>, command: CommandId) -> Self {
        Self {
            label: label.into(),
            command,
            enabled: true,
        }
    }
}
impl Widget for Button {
    fn draw(&mut self, canvas: &mut Canvas<'_>, focused: bool, theme: &Theme) {
        let style = if focused && self.enabled {
            theme.selection
        } else {
            theme.bar
        };
        canvas.fill(style);
        canvas.put_str(0, 0, &format!("[ {} ]", self.label), style);
    }
    fn event(&mut self, event: &Event) -> EventResult {
        if self.enabled
            && (matches!(event, Event::Key(k) if k.modifiers == Modifiers::NONE && matches!(k.key, Key::Enter | Key::Char(' ')))
                || matches!(event, Event::Mouse(m) if m.kind == terrabox::MouseKind::Press && m.button == terrabox::MouseButton::Left))
        {
            EventResult::command(self.command)
        } else {
            EventResult::default()
        }
    }
}

/// Single-line input. Cursor offsets are UTF-8 boundaries; deletion uses graphemes.
#[derive(Default)]
pub struct TextInput {
    pub text: String,
    cursor: usize,
    pub accept_tab: bool,
}
impl TextInput {
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let cursor = text.len();
        Self {
            text,
            cursor,
            accept_tab: false,
        }
    }
    fn boundaries(&self) -> Vec<usize> {
        use unicode_segmentation::UnicodeSegmentation;
        self.text
            .grapheme_indices(true)
            .map(|(i, _)| i)
            .chain(std::iter::once(self.text.len()))
            .collect()
    }
}
impl Widget for TextInput {
    fn draw(&mut self, canvas: &mut Canvas<'_>, focused: bool, theme: &Theme) {
        self.cursor = self
            .boundaries()
            .into_iter()
            .rfind(|i| *i <= self.cursor)
            .unwrap_or(0);
        let style = if focused { theme.selection } else { theme.bar };
        canvas.fill(style);
        let width = usize::from(canvas.size().width);
        let mut start = self.cursor;
        for boundary in self
            .boundaries()
            .into_iter()
            .rev()
            .filter(|i| *i <= self.cursor)
        {
            if terrabox::text_width(&self.text[boundary..self.cursor]) < width {
                start = boundary;
            } else {
                break;
            }
        }
        canvas.put_str(0, 0, &self.text[start..], style);
        if focused && width > 0 {
            let x = terrabox::text_width(&self.text[start..self.cursor]) as u16;
            canvas.put_str(x, 0, "▏", theme.focused);
        }
    }
    fn event(&mut self, event: &Event) -> EventResult {
        self.cursor = self
            .boundaries()
            .into_iter()
            .rfind(|i| *i <= self.cursor)
            .unwrap_or(0);
        if let Event::Paste(paste) = event {
            let text: String = String::from_utf8_lossy(&paste.bytes)
                .chars()
                .filter(|c| !c.is_control())
                .collect();
            self.text.insert_str(self.cursor, &text);
            self.cursor += text.len();
            return EventResult::handled();
        }
        let Event::Key(key) = event else {
            return EventResult::default();
        };
        if key.modifiers != Modifiers::NONE && key.modifiers != Modifiers::SHIFT {
            return EventResult::default();
        }
        let bounds = self.boundaries();
        let previous = bounds
            .iter()
            .copied()
            .rfind(|i| *i < self.cursor)
            .unwrap_or(0);
        let next = bounds
            .iter()
            .copied()
            .find(|i| *i > self.cursor)
            .unwrap_or(self.text.len());
        match key.key {
            Key::Char(c) if !c.is_control() => {
                self.text.insert(self.cursor, c);
                self.cursor += c.len_utf8();
            }
            Key::Tab if self.accept_tab && key.modifiers == Modifiers::NONE => {
                self.text.insert_str(self.cursor, "    ");
                self.cursor += 4;
            }
            Key::Backspace => {
                self.text.replace_range(previous..self.cursor, "");
                self.cursor = previous;
            }
            Key::Delete => {
                self.text.replace_range(self.cursor..next, "");
            }
            Key::Left => self.cursor = previous,
            Key::Right => self.cursor = next,
            Key::Home => self.cursor = 0,
            Key::End => self.cursor = self.text.len(),
            _ => return EventResult::default(),
        }
        EventResult::handled()
    }
}

pub struct FormDialog {
    pub title: String,
    pub controls: Group,
    pub accept: CommandId,
    pub cancel: CommandId,
}
impl Widget for FormDialog {
    fn draw(&mut self, canvas: &mut Canvas<'_>, focused: bool, theme: &Theme) {
        canvas.fill(theme.dialog);
        canvas.border(theme.dialog);
        let size = canvas.size();
        if size.width < 3 || size.height < 3 {
            return;
        }
        canvas
            .clip(Rect::new(1, 0, size.width - 2, 1))
            .put_str(0, 0, &self.title, theme.dialog);
        self.controls.draw(
            &mut canvas.clip(Rect::new(1, 1, size.width - 2, size.height - 2)),
            focused,
            theme,
        );
    }
    fn event(&mut self, event: &Event) -> EventResult {
        let local = if let Event::Mouse(m) = event {
            if m.position.x == 0 || m.position.y == 0 {
                return EventResult::default();
            }
            let mut m = *m;
            m.position.x -= 1;
            m.position.y -= 1;
            Event::Mouse(m)
        } else {
            event.clone()
        };
        // A dialog always traps traversal, even when its caller chooses Bubble.
        self.controls.tab = TabBehavior::Cycle;
        let result = self.controls.event(&local);
        if result.consumed {
            return result;
        }
        match event {
            Event::Key(k) if k.modifiers == Modifiers::NONE && k.key == Key::Enter => {
                EventResult::command(self.accept)
            }
            Event::Key(k) if k.modifiers == Modifiers::NONE && k.key == Key::Escape => {
                EventResult::command(self.cancel)
            }
            _ => EventResult::default(),
        }
    }
}

/// Percentage progress; values above 100 are clamped for rendering.
#[derive(Default)]
pub struct Progress {
    pub value: u8,
}
impl Widget for Progress {
    fn draw(&mut self, canvas: &mut Canvas<'_>, _: bool, theme: &Theme) {
        canvas.fill(theme.bar);
        let width = canvas.size().width;
        for x in 0..(u32::from(width) * u32::from(self.value.min(100)) / 100) as u16 {
            canvas.put_char(x, 0, ' ', theme.selection);
        }
        canvas.put_str(0, 0, &format!(" {}%", self.value.min(100)), theme.focused);
    }
}

pub type List = super::Panel;

pub struct TreeNode {
    pub label: String,
    pub children: Vec<TreeNode>,
    pub expanded: bool,
}
pub struct Tree {
    pub nodes: Vec<TreeNode>,
    panel: super::Panel,
    paths: Vec<Vec<usize>>,
}
impl Tree {
    pub fn new(title: impl Into<String>, nodes: Vec<TreeNode>) -> Self {
        Self {
            nodes,
            panel: super::Panel::new(title, Vec::new()),
            paths: Vec::new(),
        }
    }
    /// Index path of the selected visible node, starting at the root list.
    pub fn selected_path(&mut self) -> Option<Vec<usize>> {
        self.rebuild();
        self.panel
            .selected()
            .and_then(|i| self.paths.get(i).cloned())
    }
    fn rebuild(&mut self) {
        fn visit(
            nodes: &[TreeNode],
            prefix: &[usize],
            lines: &mut Vec<String>,
            paths: &mut Vec<Vec<usize>>,
        ) {
            for (i, node) in nodes.iter().enumerate() {
                let mut path = prefix.to_vec();
                path.push(i);
                lines.push(format!(
                    "{}{} {}",
                    "  ".repeat(prefix.len()),
                    if node.children.is_empty() {
                        "·"
                    } else if node.expanded {
                        "▾"
                    } else {
                        "▸"
                    },
                    node.label
                ));
                paths.push(path.clone());
                if node.expanded {
                    visit(&node.children, &path, lines, paths);
                }
            }
        }
        self.panel.lines.clear();
        self.paths.clear();
        visit(&self.nodes, &[], &mut self.panel.lines, &mut self.paths);
    }
}
impl Widget for Tree {
    fn draw(&mut self, canvas: &mut Canvas<'_>, focused: bool, theme: &Theme) {
        self.rebuild();
        self.panel.draw(canvas, focused, theme);
    }
    fn event(&mut self, event: &Event) -> EventResult {
        self.rebuild();
        if let Event::Key(k) = event
            && k.modifiers == Modifiers::NONE
            && matches!(k.key, Key::Left | Key::Right | Key::Enter)
        {
            if let Some(path) = self.panel.selected().and_then(|i| self.paths.get(i)) {
                let mut nodes = &mut self.nodes;
                for (depth, index) in path.iter().enumerate() {
                    let node = &mut nodes[*index];
                    if depth + 1 == path.len() {
                        match k.key {
                            Key::Left if node.expanded && !node.children.is_empty() => {
                                node.expanded = false
                            }
                            Key::Left if path.len() > 1 => {
                                if let Some(parent) = self
                                    .paths
                                    .iter()
                                    .position(|p| p.as_slice() == &path[..path.len() - 1])
                                {
                                    self.panel.selected = parent;
                                }
                            }
                            Key::Right if node.expanded && !node.children.is_empty() => {
                                self.panel.selected += 1
                            }
                            Key::Right => node.expanded = !node.children.is_empty(),
                            Key::Enter if !node.children.is_empty() => {
                                node.expanded = !node.expanded
                            }
                            _ => {}
                        }
                        break;
                    }
                    nodes = &mut node.children;
                }
            }
            return EventResult::handled();
        }
        self.panel.event(event)
    }
}
