use std::io;
use terratui::terrabox::{Key, KeyEvent, Modifiers, Size};
use terratui::widgets::{Dialog, MenuBar, Panel};
use terratui::{
    App, AppEvent, Application, Axis, CommandId, Context, Desktop, Layout, Length, Scope,
};

const QUIT: CommandId = CommandId(1);
const ABOUT: CommandId = CommandId(2);
const CLOSE: CommandId = CommandId(3);

struct Demo;

impl App for Demo {
    type Message = ();

    fn event(&mut self, event: AppEvent<()>, context: &mut Context<'_, ()>) -> io::Result<()> {
        match event {
            AppEvent::Command(QUIT) => context.quit(),
            AppEvent::Command(ABOUT) => {
                context.desktop.show_modal(Dialog { title: "TerraTUI".into(),
                    message: "A retained terminal application.\nEnter or Escape closes this dialog.\nGlobal commands are captured while modal.".into(),
                    accept: CLOSE }, Size { width: 54, height: 7 });
                context.redraw();
            }
            AppEvent::Command(CLOSE) => {
                context.desktop.close_modal();
                context.redraw();
            }
            _ => {}
        }
        Ok(())
    }
}

fn main() -> io::Result<()> {
    let mut desktop = Desktop::default();
    let left = desktop.add_window(Panel::new(
        "Navigation",
        (1..=40).map(|index| format!("Item {index}")).collect(),
    ));
    let right = desktop.add_window(Panel::new(
        "Workspace",
        vec![
            "Tab / Shift-Tab: switch panels".into(),
            "Up / Down / Page keys: select".into(),
            "F10: command menu".into(),
            "F1: modal dialog".into(),
            "Ctrl+Q: quit".into(),
            "Resize the terminal to reflow".into(),
            "Unicode: e\u{301} 界 👩‍💻".into(),
        ],
    ));
    desktop.layout = Layout::Split {
        axis: Axis::Horizontal,
        children: vec![
            (Length::Flex(1), Layout::Window(left)),
            (Length::Flex(2), Layout::Window(right)),
        ],
    };
    desktop.menu = MenuBar::new(vec![
        ("About (F1)".into(), ABOUT),
        ("Quit (Ctrl+Q)".into(), QUIT),
    ]);
    desktop.status.text = " Tab: focus | F10: menu | F1: about | Ctrl+Q: quit ".into();
    desktop.bindings.bind(
        Scope::Global,
        KeyEvent::new(Key::Function(1), Modifiers::NONE),
        ABOUT,
    );
    desktop.bindings.bind(
        Scope::Global,
        KeyEvent::new(Key::Char('Q'), Modifiers::CTRL),
        QUIT,
    );
    Application::new(Demo, desktop).run()
}
