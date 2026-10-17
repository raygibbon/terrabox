use std::{
    io,
    thread::{self, JoinHandle},
    time::Duration,
};
use terratui::terrabox::{Color, InputMode, Key, KeyEvent, Modifiers, Size, Style};
use terratui::widgets::{
    Button, FormDialog, Group, Label, List, Menu, MenuBar, MenuEntry, Progress, TextInput, Tree,
    TreeNode,
};
use terratui::{
    App, AppEvent, Application, Axis, CommandId, Context, Desktop, Layout, Length, Scope, Theme,
    WindowId,
};
const QUIT: CommandId = CommandId(1);
const DIALOG: CommandId = CommandId(2);
const CLOSE: CommandId = CommandId(3);
const THEME: CommandId = CommandId(4);
const START: CommandId = CommandId(5);
struct Gallery {
    group: WindowId,
    progress: WindowId,
    worker: Option<JoinHandle<()>>,
    dark: bool,
}
fn split(axis: Axis, children: Vec<(Length, WindowId)>) -> Layout {
    Layout::Split {
        axis,
        children: children
            .into_iter()
            .map(|(l, id)| (l, Layout::Window(id)))
            .collect(),
    }
}
fn dialog() -> FormDialog {
    let mut controls = Group::default();
    let label = controls.add(Label("Your name (Unicode welcome):".into()), false);
    let input = controls.add(TextInput::new("Terminal explorer"), true);
    let ok = controls.add(Button::new("Accept", CLOSE), true);
    let cancel = controls.add(Button::new("Cancel", CLOSE), true);
    controls.layout = split(
        Axis::Vertical,
        vec![
            (Length::Fixed(2), label),
            (Length::Fixed(2), input),
            (Length::Fixed(1), ok),
            (Length::Fixed(1), cancel),
        ],
    );
    FormDialog {
        title: " Profile · Tab / Shift+Tab · Enter / Esc ".into(),
        controls,
        accept: CLOSE,
        cancel: CLOSE,
    }
}
impl App for Gallery {
    type Message = u8;
    fn init(&mut self, context: &mut Context<'_, u8>) -> io::Result<()> {
        context.set_input_mode(InputMode::Alt);
        context.set_mouse_enabled(true)
    }
    fn event(&mut self, event: AppEvent<u8>, context: &mut Context<'_, u8>) -> io::Result<()> {
        match event {
            AppEvent::Command(QUIT) => context.quit(),
            AppEvent::Command(DIALOG) => {
                context.desktop.show_modal(
                    dialog(),
                    Size {
                        width: 58,
                        height: 10,
                    },
                );
                context.redraw();
            }
            AppEvent::Command(CLOSE) => {
                context.desktop.close_modal();
                context.redraw();
            }
            AppEvent::Command(THEME) => {
                self.dark = !self.dark;
                context.desktop.theme = if self.dark {
                    let surface = Style::new(Color::Indexed(7), Color::Indexed(0));
                    Theme {
                        surface,
                        border: surface,
                        ..Theme::default()
                    }
                } else {
                    Theme::default()
                };
                context.redraw();
            }
            AppEvent::Command(START) => {
                if self.worker.as_ref().is_none_or(|w| w.is_finished()) {
                    if let Some(worker) = self.worker.take() {
                        let _ = worker.join();
                    }
                    let sender = context.sender();
                    self.worker = Some(thread::spawn(move || {
                        for value in 0..=100 {
                            if sender.is_closed() || sender.send(value).is_err() {
                                break;
                            }
                            thread::sleep(Duration::from_millis(40));
                        }
                    }));
                }
            }
            AppEvent::User(value) => {
                if let Some(group) = context.desktop.widget_mut::<Group>(self.group)
                    && let Some(progress) = group.widget_mut::<Progress>(self.progress)
                {
                    progress.value = value;
                }
                context.redraw();
            }
            _ => {}
        }
        Ok(())
    }
    fn shutdown(&mut self, _: &mut Context<'_, u8>) -> io::Result<()> {
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        Ok(())
    }
}
fn main() -> io::Result<()> {
    let mut desktop = Desktop::default();
    let mut controls = Group::default();
    let label = controls.add(
        Label(" Widget laboratory\n Tab moves between controls and panels".into()),
        false,
    );
    let input = controls.add(TextInput::new("Edit me: 界 / café"), true);
    let open = controls.add(Button::new("Open dialog · F1", DIALOG), true);
    let start = controls.add(Button::new("Start background work · F5", START), true);
    let theme = controls.add(Button::new("Change theme · F6", THEME), true);
    let progress = controls.add(Progress { value: 0 }, false);
    let guide = controls.add(
        List::new(
            " Keyboard guide ",
            vec![
                "Tab / Shift+Tab: traverse child controls".into(),
                "Arrow / Page keys: navigate lists".into(),
                "Tree: Right expands or enters children".into(),
                "Tree: Left collapses or selects parent".into(),
                "Alt+G: open Gallery menu".into(),
                "Enter: activate the selected button".into(),
                "Dialogs trap focus and restore it on close".into(),
                "Background work keeps input responsive".into(),
            ],
        ),
        true,
    );
    controls.layout = split(
        Axis::Vertical,
        vec![
            (Length::Fixed(3), label),
            (Length::Fixed(2), input),
            (Length::Fixed(2), open),
            (Length::Fixed(2), start),
            (Length::Fixed(2), theme),
            (Length::Fixed(1), progress),
            (Length::Flex(1), guide),
        ],
    );
    let group = desktop.add_window(controls);
    let list = desktop.add_window(List::new(
        " Selectable list ",
        (1..=60)
            .map(|i| format!("Item {i:02} · keyboard navigation"))
            .collect(),
    ));
    let tree = desktop.add_window(Tree::new(
        " Project tree · ← → expand ",
        vec![TreeNode {
            label: "TerraBox".into(),
            expanded: true,
            children: vec![
                TreeNode {
                    label: "Engine".into(),
                    expanded: true,
                    children: vec![TreeNode {
                        label: "Surface / Unicode / Input".into(),
                        expanded: false,
                        children: vec![],
                    }],
                },
                TreeNode {
                    label: "TerraTUI widgets".into(),
                    expanded: false,
                    children: vec![TreeNode {
                        label: "Groups and focus".into(),
                        expanded: false,
                        children: vec![],
                    }],
                },
            ],
        }],
    ));
    desktop.layout = Layout::Split {
        axis: Axis::Horizontal,
        children: vec![
            (
                Length::MinFlex { min: 24, weight: 1 },
                Layout::Window(group),
            ),
            (
                Length::Flex(1),
                split(
                    Axis::Vertical,
                    vec![(Length::Flex(1), list), (Length::Flex(1), tree)],
                ),
            ),
        ],
    };
    desktop.menu = MenuBar::dropdowns(vec![Menu {
        label: "&Gallery".into(),
        entries: vec![
            MenuEntry::command("&Dialog  F1", DIALOG),
            MenuEntry::command("&Background work  F5", START),
            MenuEntry::command("&Theme  F6", THEME),
            MenuEntry::separator(),
            MenuEntry::command("&Quit  Ctrl+Q", QUIT),
        ],
    }]);
    desktop.status.text =
        " Tab: focus | F1: dialog | F5: work | F6: theme | F10: menu | Ctrl+Q: quit ".into();
    for (key, command) in [
        (Key::Function(1), DIALOG),
        (Key::Function(5), START),
        (Key::Function(6), THEME),
    ] {
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
        Gallery {
            group,
            progress,
            worker: None,
            dark: false,
        },
        desktop,
    )
    .run()
}
