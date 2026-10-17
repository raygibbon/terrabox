use std::{io, sync::Arc, time::Duration};
use terratui::terrabox::{Canvas, Event, Key, KeyEvent, Modifiers, Size, graphics::RgbaImage};
use terratui::widgets::{Dialog, ImageView, List};
use terratui::{
    App, AppEvent, Application, Axis, CommandId, Context, Desktop, EventResult, Layout, Length,
    Scope, Theme, Widget, WindowId,
};
mod support;
const SELECT: CommandId = CommandId(1);
const HELP: CommandId = CommandId(2);
const CLOSE: CommandId = CommandId(3);
const QUIT: CommandId = CommandId(4);
const HIDE: CommandId = CommandId(5);
struct Selection(List);
impl Widget for Selection {
    fn draw(&mut self, c: &mut Canvas<'_>, focused: bool, theme: &Theme) {
        self.0.draw(c, focused, theme);
    }
    fn event(&mut self, event: &Event) -> EventResult {
        let mut result = self.0.event(event);
        if result.consumed || matches!(event,Event::Key(k) if k.key==Key::Enter) {
            result = EventResult::command(SELECT);
        }
        result
    }
}
struct Viewer {
    list: WindowId,
    preview: WindowId,
    images: Vec<Arc<RgbaImage>>,
    hidden: bool,
}
impl App for Viewer {
    type Message = ();
    fn init(&mut self, cx: &mut Context<'_, ()>) -> io::Result<()> {
        cx.probe_capabilities(Duration::from_millis(250))?;
        Ok(())
    }
    fn event(&mut self, event: AppEvent<()>, cx: &mut Context<'_, ()>) -> io::Result<()> {
        match event {
            AppEvent::Command(QUIT) => cx.quit(),
            AppEvent::Command(SELECT) => {
                let index = cx
                    .desktop
                    .widget_mut::<Selection>(self.list)
                    .unwrap()
                    .0
                    .selected()
                    .unwrap_or(0);
                let view = cx.desktop.widget_mut::<ImageView>(self.preview).unwrap();
                view.image = Some(self.images[index].clone());
                view.disabled = false;
                cx.redraw();
            }
            AppEvent::Command(HELP) => {
                cx.desktop.show_modal(Dialog { title:" Image viewer help ".into(),message:"Up/Down: select a procedural image\nTab: focus a panel | F2: hide/show preview\nImage resources are freed while this dialog is open.\nEnter or Escape closes. Ctrl+Q quits.".into(),accept:CLOSE },Size { width:62,height:9 });
                cx.redraw();
            }
            AppEvent::Command(CLOSE) => {
                cx.desktop.close_modal();
                cx.redraw();
            }
            AppEvent::Command(HIDE) => {
                self.hidden = !self.hidden;
                cx.desktop.layout = layout(self.list, self.preview, self.hidden);
                cx.redraw();
            }
            AppEvent::Terminal(Event::Response(bytes))
                if bytes.starts_with(b"\x1b_G") && !bytes.ends_with(b";OK\x1b\\") =>
            {
                cx.desktop
                    .widget_mut::<ImageView>(self.preview)
                    .unwrap()
                    .disabled = true;
                cx.desktop.status.text =
                    " Image rejected by terminal; text fallback active | F1: help | Ctrl+Q: quit "
                        .into();
                cx.redraw();
            }
            _ => {}
        }
        Ok(())
    }
}
fn layout(left: WindowId, right: WindowId, hidden: bool) -> Layout {
    Layout::Split {
        axis: Axis::Horizontal,
        children: vec![
            (Length::Fixed(28), Layout::Window(left)),
            (
                Length::Flex(1),
                if hidden {
                    Layout::Hidden
                } else {
                    Layout::Window(right)
                },
            ),
        ],
    }
}
fn main() -> io::Result<()> {
    let images: Vec<_> = (0..3).map(support::sample).collect();
    let mut desktop = Desktop::default();
    let list = desktop.add_window(Selection(List::new(
        " Procedural images ",
        vec![
            "Lagoon / alpha circle".into(),
            "Sunset / hills".into(),
            "Spectrum / transparency".into(),
        ],
    )));
    let preview = desktop.add_window(ImageView::new(" Image preview ", Some(images[0].clone())));
    desktop.layout = layout(list, preview, false);
    desktop.status.text =
        " Up/Down: select | Tab: focus | F1: help | F2: hide preview | Ctrl+Q: quit ".into();
    for (key, command) in [(Key::Function(1), HELP), (Key::Function(2), HIDE)] {
        desktop
            .bindings
            .bind(Scope::Global, KeyEvent::new(key, Modifiers::NONE), command);
    }
    desktop.bindings.bind(
        Scope::Global,
        KeyEvent::new(Key::Char('Q'), Modifiers::CTRL),
        QUIT,
    );
    Application::new(
        Viewer {
            list,
            preview,
            images,
            hidden: false,
        },
        desktop,
    )
    .run()
}
