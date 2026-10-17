use crate::widgets::{MenuBar, StatusBar};
use crate::{Bindings, CommandId, Focus, Layout, Scope, Theme, WindowId};
use terrabox::{Canvas, Event, Key, Rect, Size, Surface};

/// Event consumption is explicit; redraw is independent of command emission.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EventResult {
    pub consumed: bool,
    pub redraw: bool,
    pub command: Option<CommandId>,
}

impl EventResult {
    pub fn handled() -> Self {
        Self {
            consumed: true,
            redraw: true,
            command: None,
        }
    }
    pub fn command(command: CommandId) -> Self {
        Self {
            command: Some(command),
            ..Self::handled()
        }
    }
}

/// A persistent interaction object drawing only inside its supplied canvas.
pub trait Widget: std::any::Any {
    #[cfg(feature = "kitty-graphics")]
    fn prepare_graphics(&mut self, _available: bool) {}
    #[cfg(feature = "kitty-graphics")]
    fn collect_graphics(&self, _area: Rect, _requests: &mut Vec<terrabox::graphics::ImageRequest>) {
    }
    fn draw(&mut self, canvas: &mut Canvas<'_>, focused: bool, theme: &Theme);
    fn event(&mut self, _event: &Event) -> EventResult {
        EventResult::default()
    }
}

struct Window {
    id: WindowId,
    widget: Box<dyn Widget>,
    modal_size: Option<Size>,
}

/// Retained tiled windows, modal overlays, command menu and status bar.
pub struct Desktop {
    pub layout: Layout,
    pub bindings: Bindings,
    pub menu: MenuBar,
    pub status: StatusBar,
    pub theme: Theme,
    focus: Focus,
    windows: Vec<Window>,
    next_id: u64,
    /// Apply default Tab traversal only after a widget declines the key.
    pub tab_navigation: bool,
    areas: Vec<(WindowId, Rect)>,
}

impl Default for Desktop {
    fn default() -> Self {
        Self {
            layout: Layout::default(),
            bindings: Bindings::default(),
            menu: MenuBar::default(),
            status: StatusBar::default(),
            theme: Theme::default(),
            focus: Focus::default(),
            windows: Vec::new(),
            next_id: 0,
            tab_navigation: true,
            areas: Vec::new(),
        }
    }
}

impl Desktop {
    /// Mutate a retained widget by stable identity and its concrete type.
    pub fn widget_mut<WidgetType: Widget>(&mut self, id: WindowId) -> Option<&mut WidgetType> {
        let window = self.windows.iter_mut().find(|window| window.id == id)?;
        (window.widget.as_mut() as &mut dyn std::any::Any).downcast_mut()
    }

    pub fn add_window(&mut self, widget: impl Widget + 'static) -> WindowId {
        let id = self.insert(widget, None);
        self.focus.set_order(
            self.windows
                .iter()
                .filter(|window| window.modal_size.is_none())
                .map(|window| window.id)
                .collect(),
        );
        id
    }

    fn insert(&mut self, widget: impl Widget + 'static, modal_size: Option<Size>) -> WindowId {
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("window identity exhausted");
        let id = WindowId(self.next_id);
        self.windows.push(Window {
            id,
            widget: Box::new(widget),
            modal_size,
        });
        id
    }

    pub fn focused(&self) -> Option<WindowId> {
        self.focus.active()
    }

    pub fn focus(&mut self, id: WindowId) -> bool {
        self.focus.select(id)
    }

    pub fn show_modal(&mut self, widget: impl Widget + 'static, size: Size) -> WindowId {
        self.menu.close();
        let id = self.insert(widget, Some(size));
        self.focus.capture(id);
        id
    }

    pub fn close_modal(&mut self) -> Option<WindowId> {
        let id = self.focus.restore()?;
        self.windows.retain(|window| window.id != id);
        Some(id)
    }

    pub fn has_modal(&self) -> bool {
        self.focus.modal().is_some()
    }

    pub fn route(&mut self, event: &Event) -> EventResult {
        if let Some(modal) = self.focus.modal() {
            let result = self.deliver(modal, event);
            if result.consumed {
                return result;
            }
            if matches!(event, Event::Key(key) if key.key == Key::Escape) {
                self.close_modal();
                return EventResult::handled();
            }
            if let Event::Key(key) = event
                && let Some(command) = self.bindings.resolve(Scope::Window(modal), *key)
            {
                return EventResult::command(command);
            }
            return EventResult {
                consumed: true,
                ..result
            };
        }
        if self.menu.wants_event(event) {
            return self.menu.event(event);
        }
        let previous_focus = self.focus.active();
        if let Event::Mouse(mouse) = event
            && mouse.kind == terrabox::MouseKind::Press
            && let Some((id, _)) = self
                .areas
                .iter()
                .find(|(_, area)| area.contains(mouse.position))
        {
            self.focus.select(*id);
        }
        let mut result = self
            .focus
            .active()
            .map(|id| self.deliver(id, event))
            .unwrap_or_default();
        result.redraw |= previous_focus != self.focus.active();
        if result.consumed {
            return result;
        }
        if let Event::Key(key) = event {
            if let Some(reverse) = crate::widgets::tab_direction(event)
                && self.tab_navigation
            {
                self.focus.advance(reverse);
                return EventResult::handled();
            }
            if let Some(id) = self.focus.active()
                && let Some(command) = self.bindings.resolve(Scope::Window(id), *key)
            {
                return EventResult::command(command);
            }
            if let Some(command) = self.bindings.resolve(Scope::Global, *key) {
                return EventResult::command(command);
            }
        }
        result
    }

    fn deliver(&mut self, id: WindowId, event: &Event) -> EventResult {
        let local = if let Event::Mouse(mouse) = event {
            let Some((_, area)) = self.areas.iter().find(|(target, _)| *target == id) else {
                return EventResult::default();
            };
            if !area.contains(mouse.position) {
                return EventResult::default();
            }
            let mut mouse = *mouse;
            mouse.position.x -= area.x;
            mouse.position.y -= area.y;
            Event::Mouse(mouse)
        } else {
            event.clone()
        };
        self.windows
            .iter_mut()
            .find(|window| window.id == id)
            .map(|window| window.widget.event(&local))
            .unwrap_or_default()
    }

    /// Compose text and optional image placements through one engine pipeline.
    pub fn present(&mut self, terminal: &mut terrabox::Terminal) -> std::io::Result<()> {
        #[cfg(feature = "kitty-graphics")]
        {
            let available =
                terminal.graphics_available() && !self.has_modal() && !self.menu.is_active();
            for window in &mut self.windows {
                window.widget.prepare_graphics(available);
            }
        }
        let surface = self.render(terminal.size());
        terminal.draw_surface(&surface, terrabox::Position::default());
        #[cfg(feature = "kitty-graphics")]
        {
            let requests = self.image_requests();
            terminal.sync_graphics(&requests)?;
        }
        terminal.hide_cursor();
        terminal.present()
    }

    /// Resolve image requests from the most recently rendered layout. Modal and
    /// menu overlays suppress the whole image scene rather than pixel clipping.
    #[cfg(feature = "kitty-graphics")]
    pub fn image_requests(&self) -> Vec<terrabox::graphics::ImageRequest> {
        let mut requests = Vec::new();
        if !self.has_modal() && !self.menu.is_active() {
            for window in &self.windows {
                if let Some((_, area)) = self.areas.iter().find(|(id, _)| *id == window.id) {
                    window.widget.collect_graphics(*area, &mut requests);
                }
            }
        }
        requests
    }

    pub fn render(&mut self, size: Size) -> Surface {
        let mut surface = Surface::new(size);
        surface.clear(self.theme.surface);
        let menu_rows = u16::from(!self.menu.is_empty()).min(size.height);
        let status_rows =
            u16::from(!self.status.text.is_empty()).min(size.height.saturating_sub(menu_rows));
        let area = Rect::new(
            0,
            menu_rows,
            size.width,
            size.height.saturating_sub(menu_rows + status_rows),
        );
        let arranged = self.layout.arrange(area);
        self.areas = arranged.clone();
        let visible: Vec<_> = arranged
            .iter()
            .filter(|(_, rect)| rect.width > 0 && rect.height > 0)
            .filter(|(id, _)| {
                self.windows
                    .iter()
                    .any(|window| window.id == *id && window.modal_size.is_none())
            })
            .map(|(id, _)| *id)
            .collect();
        self.focus.set_order(visible);
        for window in self
            .windows
            .iter_mut()
            .filter(|window| window.modal_size.is_none())
        {
            let Some((_, area)) = arranged.iter().find(|(id, _)| *id == window.id) else {
                continue;
            };
            window.widget.draw(
                &mut surface.canvas(*area),
                self.focus.active() == Some(window.id),
                &self.theme,
            );
        }
        if menu_rows > 0 {
            self.menu.draw(
                &mut surface.canvas(Rect::new(0, 0, size.width, 1)),
                false,
                &self.theme,
            );
        }
        if status_rows > 0 {
            self.status.draw(
                &mut surface.canvas(Rect::new(0, size.height - 1, size.width, 1)),
                false,
                &self.theme,
            );
        }
        self.menu.draw_dropdown(
            &mut surface.canvas(Rect::new(0, 0, size.width, size.height)),
            &self.theme,
        );
        for window in self
            .windows
            .iter_mut()
            .filter(|window| window.modal_size.is_some())
        {
            let requested = window.modal_size.unwrap();
            let width = requested.width.min(size.width);
            let height = requested.height.min(size.height);
            let area = Rect::new(
                (size.width - width) / 2,
                (size.height - height) / 2,
                width,
                height,
            );
            self.areas.push((window.id, area));
            window.widget.draw(
                &mut surface.canvas(area),
                self.focus.active() == Some(window.id),
                &self.theme,
            );
        }
        surface
    }
}
