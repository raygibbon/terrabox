//! Read-only directory browsing; filesystem operations stay in this application.
use std::{fs, io, path::PathBuf};
use terratui::terrabox::{Event, Key, KeyEvent, Modifiers, Size};
use terratui::widgets::{Dialog, Panel};
use terratui::{
    App, AppEvent, Application, Axis, CommandId, Context, Desktop, Layout, Length, Scope, WindowId,
};
const QUIT: CommandId = CommandId(1);
const INFO: CommandId = CommandId(2);
const CLOSE: CommandId = CommandId(3);
struct Browser {
    ids: [WindowId; 2],
    paths: [PathBuf; 2],
    entries: [Vec<PathBuf>; 2],
}
impl Browser {
    fn load(&mut self, index: usize, context: &mut Context<'_, ()>) {
        let result = fs::read_dir(&self.paths[index]).and_then(|items| {
            items
                .map(|item| item.map(|e| e.path()))
                .collect::<io::Result<Vec<_>>>()
        });
        match result {
            Ok(mut paths) => {
                paths.sort_by_key(|p| {
                    (
                        !p.is_dir(),
                        p.file_name().map(|n| n.to_string_lossy().to_lowercase()),
                    )
                });
                if let Some(parent) = self.paths[index].parent() {
                    paths.insert(0, parent.to_path_buf());
                }
                let lines = paths
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        if i == 0 && self.paths[index].parent() == Some(p.as_path()) {
                            "↰ ..".into()
                        } else {
                            format!(
                                "{} {}",
                                if p.is_dir() { "▸" } else { "·" },
                                p.file_name().unwrap_or_default().to_string_lossy()
                            )
                        }
                    })
                    .collect();
                self.entries[index] = paths;
                // Replacing the panel resets selection on directory changes.
                *context
                    .desktop
                    .widget_mut::<Panel>(self.ids[index])
                    .unwrap() = Panel::new(
                    if index == 0 {
                        " Left directory "
                    } else {
                        " Right directory "
                    },
                    lines,
                );
                context.desktop.status.text = " Tab: panel | Enter: open directory | Backspace: parent | F3: info | F10: quit ".into();
            }
            Err(error) => {
                context.desktop.status.text = format!(
                    " Cannot read directory: {} | Backspace: parent | F10: quit",
                    error.kind()
                )
            }
        }
        context.redraw();
    }
}
impl App for Browser {
    type Message = ();
    fn init(&mut self, context: &mut Context<'_, ()>) -> io::Result<()> {
        self.load(0, context);
        self.load(1, context);
        Ok(())
    }
    fn event(&mut self, event: AppEvent<()>, context: &mut Context<'_, ()>) -> io::Result<()> {
        let index = usize::from(context.desktop.focused() == Some(self.ids[1]));
        match event {
            AppEvent::Command(QUIT) => context.quit(),
            AppEvent::Command(CLOSE) => {
                context.desktop.close_modal();
                context.redraw();
            }
            AppEvent::Command(INFO) => {
                if let Some(path) = context
                    .desktop
                    .widget_mut::<Panel>(self.ids[index])
                    .unwrap()
                    .selected()
                    .and_then(|i| self.entries[index].get(i))
                {
                    let message = match fs::metadata(path) {
                        Ok(meta) => format!(
                            "{}\n{} bytes · {}\nRead-only demonstration. Enter closes.",
                            path.file_name().unwrap_or_default().to_string_lossy(),
                            meta.len(),
                            if meta.is_dir() { "directory" } else { "file" }
                        ),
                        Err(e) => format!("Metadata unavailable: {}", e.kind()),
                    };
                    context.desktop.show_modal(
                        Dialog {
                            title: " File information ".into(),
                            message,
                            accept: CLOSE,
                        },
                        Size {
                            width: 62,
                            height: 7,
                        },
                    );
                    context.redraw();
                }
            }
            AppEvent::Terminal(Event::Key(k)) if k.key == Key::Enter => {
                if let Some(path) = context
                    .desktop
                    .widget_mut::<Panel>(self.ids[index])
                    .unwrap()
                    .selected()
                    .and_then(|i| self.entries[index].get(i))
                    .filter(|p| p.is_dir())
                    .cloned()
                {
                    self.paths[index] = path;
                    self.load(index, context);
                }
            }
            AppEvent::Terminal(Event::Key(k)) if k.key == Key::Backspace => {
                if let Some(path) = self.paths[index].parent() {
                    self.paths[index] = path.to_path_buf();
                    self.load(index, context);
                }
            }
            _ => {}
        }
        Ok(())
    }
}
fn main() -> io::Result<()> {
    let mut desktop = Desktop::default();
    let left = desktop.add_window(Panel::new("Left", vec![]));
    let right = desktop.add_window(Panel::new("Right", vec![]));
    desktop.layout = Layout::Split {
        axis: Axis::Horizontal,
        children: vec![
            (Length::Flex(1), Layout::Window(left)),
            (Length::Flex(1), Layout::Window(right)),
        ],
    };
    for (key, command) in [(Key::Function(10), QUIT), (Key::Function(3), INFO)] {
        desktop
            .bindings
            .bind(Scope::Global, KeyEvent::new(key, Modifiers::NONE), command);
    }
    let path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir()?);
    let path = fs::canonicalize(path)?;
    Application::new(
        Browser {
            ids: [left, right],
            paths: [path.clone(), path],
            entries: [vec![], vec![]],
        },
        desktop,
    )
    .run()
}
