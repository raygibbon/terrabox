# Framework usage

Run `cargo run -p terratui --example desktop` for a complete application.
`crates/terratui/examples/desktop.rs` demonstrates App callbacks, command IDs,
bindings, retained panels, a split layout, menu and modal closure.

## Custom widgets

Implement `Widget::draw` and optionally `Widget::event`. The supplied Canvas
uses local coordinates and clips writes; call `canvas.clip(Rect)` for a nested
view. Draw using TerraBox Cell/Style types and `put_str` for grapheme correctness.
Return EventResult with consumed/redraw flags and an optional command. Commands
go to App::event; unconsumed terminal events also reach App::event. Resize always
reaches the application, including while a modal captures keyboard input.

Keep widgets stateful: selection and viewport offsets persist in the widget.
Use `desktop.widget_mut::<Panel>(id)` to update retained built-in widget state,
then request redraw through Context. A type mismatch or unknown ID returns None.
Keep domain models outside the framework. A custom editor widget can hold an
application-owned document handle without teaching TerraTUI about documents.

## Scheduling and lifecycle

App::init and App::shutdown surround the event loop; shutdown is attempted even
when init or event handling returns an error. The first error propagates. The
owned Terminal restores native state on return and unwind. App::shutdown is an
ordinary callback, not a panic hook. Terminal restoration cannot survive abort.

Mutate `context.desktop`, then call `context.redraw()` when changing visible
state. Input handled by built-in widgets requests redraw automatically. Use
`context.quit()` for orderly exit. Use `context.run_command()` for child process
handoff and restored terminal rendering. `Application::run_on` accepts an owned
Terminal connected through an explicit engine constructor.
Initialise mouse collection and Escape/Alt interpretation through Context's
`set_mouse_enabled` and `set_input_mode`; both reuse the engine implementation.

```rust
use std::time::Duration;
use terratui::{AppEvent, Context, TimerId};
use terratui::terrabox::Deadline;

fn request_frame(context: &mut Context<'_, ()>) {
    if let Some(deadline) = Deadline::after(Duration::from_millis(40)) {
        context.set_timer(TimerId(1), deadline);
    }
}
```

Timers are one-shot, identified by TimerId, replaceable and cancellable. A timer
handler can schedule its next frame; no timer is installed automatically.
Queued `context.post(message)` events are delivered on the UI thread. There is
no cross-thread wake channel yet. Long work should not block event handlers;
an application can poll a worker through explicitly requested timer events.

## MVP boundaries

Base windows are tiled by Layout; centred modals overlay in stack order. Focus
cycles visible base windows; modal focus restores on close. Menu items and
bindings emit application-owned CommandId values. The menu uses F10,
Left/Right, Enter and Escape. Modal shortcuts never fall through to global
bindings. This MVP does not provide an editable input, text cursor policy,
pull-down hierarchy, mouse hit testing or arbitrary floating window geometry.
Mouse/paste events remain available to custom widgets and applications.
