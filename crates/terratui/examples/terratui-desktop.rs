use std::io;
use terratui::terrabox::{Key, KeyEvent, Modifiers, Size};
use terratui::widgets::{
    Button, Dialog, FormDialog, Group, Label, Menu, MenuBar, MenuEntry, Panel, TextInput,
};
use terratui::{
    App, AppEvent, Application, Axis, CommandId, Context, Desktop, Layout, Length, Scope,
};

const QUIT: CommandId = CommandId(1);
const ABOUT: CommandId = CommandId(2);
const CLOSE: CommandId = CommandId(3);
const EDIT: CommandId = CommandId(4);
const VIEW: CommandId = CommandId(5);
const SAVE: CommandId = CommandId(6);

struct Demo {
    profile: Option<(terratui::WindowId, terratui::WindowId)>,
}

impl App for Demo {
    type Message = ();
    fn init(&mut self, context: &mut Context<'_, ()>) -> io::Result<()> {
        context.set_input_mode(terratui::terrabox::InputMode::Alt);
        context.set_mouse_enabled(true)
    }

    fn event(&mut self, event: AppEvent<()>, context: &mut Context<'_, ()>) -> io::Result<()> {
        match event {
            AppEvent::Command(QUIT) => context.quit(),
            AppEvent::Command(ABOUT) => {
                context.desktop.show_modal(Dialog { title: "TerraTUI".into(),
                    message: "A retained terminal application.\nEnter or Escape closes this dialog.\nGlobal commands are captured while modal.".into(),
                    accept: CLOSE }, Size { width: 54, height: 7 });
                context.redraw();
            }
            AppEvent::Command(EDIT) => {
                let mut controls = Group::default();
                let label = controls.add(Label("Workspace name:".into()), false);
                let input = controls.add(TextInput::new("TerraTUI workspace"), true);
                let save = controls.add(Button::new("Save", SAVE), true);
                let cancel = controls.add(Button::new("Cancel", CLOSE), true);
                controls.layout = Layout::Split {
                    axis: Axis::Vertical,
                    children: vec![
                        (Length::Fixed(2), Layout::Window(label)),
                        (Length::Fixed(2), Layout::Window(input)),
                        (Length::Fixed(1), Layout::Window(save)),
                        (Length::Fixed(1), Layout::Window(cancel)),
                    ],
                };
                let id = context.desktop.show_modal(
                    FormDialog {
                        title: " Edit workspace ".into(),
                        controls,
                        accept: SAVE,
                        cancel: CLOSE,
                    },
                    Size {
                        width: 48,
                        height: 9,
                    },
                );
                self.profile = Some((id, input));
                context.redraw();
            }
            AppEvent::Command(SAVE) => {
                if let Some((dialog, input)) = self.profile.take()
                    && let Some(form) = context.desktop.widget_mut::<FormDialog>(dialog)
                    && let Some(input) = form.controls.widget_mut::<TextInput>(input)
                {
                    context.desktop.status.text = format!(
                        " Workspace saved: {} | F10: menu | Ctrl+Q: quit",
                        input.text
                    );
                }
                context.desktop.close_modal();
                context.redraw();
            }
            AppEvent::Command(VIEW) => {
                std::mem::swap(
                    &mut context.desktop.theme.surface,
                    &mut context.desktop.theme.dialog,
                );
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
            "Unicode: e\u{301} 界 λ ░▒▓".into(),
        ],
    ));
    let notes = desktop.add_window(Panel::new(
        " Framework capabilities ",
        vec![
            "Retained windows with stable identities".into(),
            "Nested focus scopes and modal restoration".into(),
            "Dropdown commands · disabled entries · separators".into(),
            "Editable forms: Edit > Workspace name".into(),
            "View > Change theme".into(),
            "Native resize handling and clipped Unicode".into(),
            "Blocking idle loop; no async runtime".into(),
            "Try the widget gallery for background tasks".into(),
        ],
    ));
    desktop.layout = Layout::Split {
        axis: Axis::Horizontal,
        children: vec![
            (Length::Flex(1), Layout::Window(left)),
            (
                Length::Flex(2),
                Layout::Split {
                    axis: Axis::Vertical,
                    children: vec![
                        (Length::Flex(1), Layout::Window(right)),
                        (Length::Flex(1), Layout::Window(notes)),
                    ],
                },
            ),
        ],
    };
    desktop.menu = MenuBar::dropdowns(vec![
        Menu {
            label: "&File".into(),
            entries: vec![
                MenuEntry::command("&About this desktop", ABOUT),
                MenuEntry::separator(),
                MenuEntry::command("&Quit", QUIT)
                    .with_shortcut(KeyEvent::new(Key::Char('Q'), Modifiers::CTRL)),
            ],
        },
        Menu {
            label: "&Edit".into(),
            entries: vec![
                MenuEntry {
                    enabled: false,
                    ..MenuEntry::command("&Copy (demo unavailable)", ABOUT)
                },
                MenuEntry::command("&Workspace name…", EDIT),
            ],
        },
        Menu {
            label: "&View".into(),
            entries: vec![MenuEntry::command("&Change theme", VIEW)],
        },
        Menu {
            label: "&Help".into(),
            entries: vec![MenuEntry::command("&About  F1", ABOUT)],
        },
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
    Application::new(Demo { profile: None }, desktop).run()
}
