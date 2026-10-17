# TerraTUI

A retained terminal application framework over the independently usable TerraBox
engine. It owns application lifecycle, commands, focus, layouts and widgets;
your application owns its data and business operations. No async runtime.

```rust,no_run
use terratui::{App, AppEvent, Application, Context, Desktop, Layout};
use terratui::terrabox::{Event, Key};
use terratui::widgets::List;

struct Demo;
impl App for Demo {
    type Message = ();
    fn event(&mut self, event: AppEvent<()>, cx: &mut Context<'_, ()>) -> std::io::Result<()> {
        if matches!(event, AppEvent::Terminal(Event::Key(k)) if k.key == Key::Escape) {
            cx.quit();
        }
        Ok(())
    }
}
fn main() -> std::io::Result<()> {
    let mut desktop = Desktop::default();
    let list = desktop.add_window(List::new("Choices", vec!["One".into(), "Two".into()]));
    desktop.layout = Layout::Window(list);
    Application::new(Demo, desktop).run()
}
```

Use `Widget::draw` for low-level drawing through TerraBox's clipped `Canvas`.
`Widget::event` returns explicit consumption, redraw and command information.
`App::event` handles commands, unconsumed input, timers and application messages.
Change retained widgets through `desktop.widget_mut::<T>(id)`, then request redraw.

`Group` owns child widgets and a local layout. Groups nest, retain their focused
child, and offer `TabBehavior::{Bubble, Cycle, Manual}`. Child input is delivered
before default traversal; a `TextInput` can opt into Tab indentation. Desktop's
default window traversal can also be disabled. `FormDialog` traps child focus;
closing it restores the prior window and its retained child selection.

Widgets include labels, buttons, text input, panels/selectable lists, trees,
menus, progress and status. Trees contain application-supplied nodes, never
filesystem operations. Menu labels use `&` mnemonics; Alt interpretation and
mouse collection must be enabled through Context when wanted.

Use `context.sender()` to send typed messages from a worker. Native wakeups
unblock the event loop; messages are delivered on the UI thread. Workers check
`sender.is_closed()` and are joined in `App::shutdown`. The gallery demonstrates
this lifecycle without blocking keyboard input. Timers are explicit one-shot
`Deadline`s, replaceable and cancellable by ID. Animation schedules timers and
updates cells; advanced transitions and Kitty graphics are planned.

Run `cargo run -p terratui --example terratui-widgets` or
`cargo run -p terratui --example terratui-desktop` from the workspace.
See [framework usage](../../docs/TERRATUI.md) for routing and API constraints.
