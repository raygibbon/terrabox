# Framework usage

Run the desktop, files and widget examples through the commands in the root
README. The original `desktop` example is retained for compatibility.

## Widgets and focus

Implement `Widget::draw` and optionally `Widget::event`. A supplied `Canvas`
uses local coordinates and clips writes; `canvas.clip(Rect)` creates a nested
view. `EventResult` separates consumption, redraw and command emission. Commands
go to `App::event`; unconsumed terminal events also reach it. Resize reaches the
application even during modal capture. Applications mutate widgets through
`desktop.widget_mut::<T>(id)` and request redraw when changing visible state.

`Group::add(widget, focusable)` returns a local stable `WindowId` for its layout
and `widget_mut` lookup. IDs are scoped to the owning group: store the group ID
and child ID together. Nest groups for hierarchical focus. Labels/progress
usually are not focusable. Focus is retained across redraws; if a focused region
becomes hidden or zero-sized, the first visible focusable child is selected.

For ordinary input, dispatch visits the focused window and then the focused
child recursively. A consumed event stops propagation. Only a declined Tab or
Shift+Tab reaches traversal. `TextInput::accept_tab` inserts four spaces; custom
editors may implement a different policy. `Group::tab` chooses:

- `Bubble` (default): traverse children; at an edge let the parent handle it.
- `Cycle`: wrap within the group, trapping focus.
- `Manual`: decline automatic traversal and leave navigation to the parent/app.

Desktop wraps visible windows by default; set `tab_navigation = false` for
application-managed panel switching. A single-window group that should wrap
its controls should use `Cycle`. Groups retain the prior child when re-entered.

A modal captures all input and excludes global/base bindings. Its child gets
first refusal, then modal bindings/default Escape handling apply. `FormDialog`
forces child traversal to `Cycle`, emits its accept command on unconsumed Enter
and cancel command on Escape. The application closes the modal in its command
handler; a generic `Dialog` uses Enter for accept and default desktop Escape
closure. Nested modal closure restores the previous window without rebuilding
its child focus. Child bindings are implemented by custom widgets or propagated
commands; framework `Scope::Window` bindings address desktop windows.

Menu capture precedes ordinary widget delivery while open. F10, Alt+mnemonic,
menu shortcuts and menu-bar clicks activate menu handling. Otherwise a widget
receives keys before window bindings, then global bindings. Local bindings
still override global bindings. `MenuBar::new` preserves the original horizontal
command menu; `MenuBar::dropdowns` accepts `Menu`/`MenuEntry` values. Disabled
entries and separators never emit commands. Left/Right changes menus, Up/Down
selects enabled entries, Enter/mnemonic activates, Escape closes. Set
`InputMode::Alt` for Alt mnemonics (some terminals intercept those keys).

Mouse events are translated from terminal to window/group-local coordinates.
Clicks change focus, buttons activate, lists select/scroll and menus hit-test.
Call `context.set_mouse_enabled(true)` to collect mouse events. Paste events
insert printable text in text inputs. Input editing moves/deletes by grapheme;
IME integration, multiline editing, selection and masking are not provided.

## Layout and rendering

`Layout::Split` nests horizontal/vertical children. `Fixed(n)` reserves space;
`Flex(weight)` shares the remainder; `MinFlex { min, weight }` reserves a
minimum before sharing extra space. `Layout::Hidden` removes a panel and its
allocation. When minima/fixed sizes exceed the terminal, children are truncated
in declaration order; later children can become zero-sized and lose focus.
This deterministic fallback never expands outside the parent rectangle.

Frames are rebuilt only on redraw, then TerraBox presents changed rows. There
is no frame rate while idle and no per-widget render cache. Trees use visible
rows and index paths supplied by the application. `Tree::selected_path` exposes
the selection; Left collapses/selects parent, Right expands/selects child and
Enter toggles expansion. Domain actions stay outside the framework.

## Messages, timing and shutdown

`Application::run` owns the terminal. Init and shutdown bracket the loop;
shutdown is attempted after init/event I/O errors, and the first error wins.
Terminal Drop restores state on return/unwind. Shutdown is an ordinary callback,
not a panic hook; workers must tolerate closed senders during unwinding.

`Context::sender()` returns an application-scoped `EventSender<Message>`.
It is transferable between threads when `Message: Send`. A standard unbounded
MPSC queue stores messages; enqueue precedes a coalescing native notification.
Unix waits on an independent nonblocking socket alongside input/SIGWINCH;
Windows waits on an auto-reset event alongside console input. No wake thread,
periodic poll or async runtime is needed. Messages route to this application's
`AppEvent::User` callback. Include a window/group ID in the message and use
`widget_mut` to resolve a still-valid destination; no worker holds UI pointers.

The loop handles at most one queued message before checking input and deadlines,
so a stream of worker messages does not bypass keyboard routing. `Context::post`
queues messages from the UI thread. The queue is unbounded: producers must limit
update rate/coalesce data to avoid memory growth under a sustained flood.

`sender.is_closed()` becomes true before shutdown, or when unwinding leaves the
run. Subsequent sends fail. Long-running workers check it between work chunks;
applications own/join worker handles in shutdown. Work is cooperatively
cancelled; the framework never kills threads. A sender should not be joined
from a worker itself. The widget gallery shows completion updates and quit
while work is active.

Timers are one-shot `Deadline`s keyed by `TimerId`. Set replaces the deadline;
cancel removes it. A timer handler explicitly schedules another frame when
needed. `context.redraw()` and `context.quit()` control rendering and exit.
`run_command` suspends/restores the engine for a child process. Graphics and
advanced animation are separate future capabilities, not implied by timers.

## Current limits

Base windows are tiled with centred modals. Floating drag/resize, scrollbars,
mouse text cursor positioning, nested submenus and scrolling over-height menus
are deferred. Current small-terminal fallback clips controls rather than
providing an automatic alternate form. Stable child IDs are group-local;
there is no global widget registry or task scheduler.
