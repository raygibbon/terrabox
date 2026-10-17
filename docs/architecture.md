# TerraBox and TerraTUI architecture

## Goals and boundaries

One repository, two independently versioned MIT Rust libraries. TerraBox is a
synchronous terminal engine that applications can use alone. TerraTUI provides
application structure without owning files, documents, syntax or providers.
There is no mandatory async runtime, fixed frame rate, GUI backend or ncurses.

```text
Applications ───────────────> TerraBox
     │                          │
     └──> TerraTUI ──────────────┘
                                └──> native Unix / Windows backends
```

## Modules and rendering

Preserve engine terminal, buffer, cell, colour, event, key, platform and terminfo
modules. Add surface geometry and clipped drawing around the same Buffer/Cell
implementation, plus an independent deadline helper. No second cell model.
The private `platform` directory keeps its name to preserve tests and avoid an
unnecessary backend rewrite.

Widgets draw into a clipped engine Canvas on a Surface. Desktop lays out base
windows, then paints modal windows in stack order, finally compositing the
surface into Terminal. Terminal alone compares against the physical front
buffer and emits changed rows. Rebuilding a logical frame on demand keeps
overlap restoration simple; unchanged frames are not presented. Surfaces are
opaque, not alpha-composited. Wide graphemes are atomic at clipping boundaries;
partial glyphs are skipped and writes never erase a glyph outside a clip.
Portable region scrolling moves logical cells and fills exposed rows. Hardware
terminal scroll commands are deferred until front-buffer synchronisation and
capability fallbacks have dedicated tests.

## Retained state and layout

TerraTUI retains a Desktop of windows with stable WindowId values and boxed
stateful Widget objects. Each widget exposes drawing and event consumption.
Applications may add custom widgets that render styled content using TerraBox
types. A recursive Layout supports horizontal/vertical splits and fixed/flex
lengths. Short terminals truncate fixed lengths and distribute integer remainders
deterministically; zero-sized regions are valid.

Focus is a stable ID, not an array position or document ID. Tab/BackTab cycle
base windows. Modal windows capture all input and restore the previous focus
when closed; nested modal windows form a stack. Draw order is base insertion
order followed by modal stack order. The MVP exposes tiled base windows and
centred modal overlays; arbitrary draggable floating windows are deferred.

Compared with immediate-mode UI, retained state avoids repeatedly rebuilding
selection, focus and menu state, and gives commands persistent targets. Costs
include explicit mutation/redraw requests, stable identity management and boxed
widget dispatch. Logical rendering remains a straightforward redraw on change;
this is not a full cached scene graph or a Ratatui widget API clone.

## Input, commands and application lifecycle

Native events flow through the existing parser/backend to Application, then to
Desktop: modal capture first, menu navigation, local bindings, focused widget,
global bindings and application callback. Local bindings override global ones;
modal input cannot trigger base/global shortcuts accidentally. Menus emit the
same CommandId as key bindings. Bindings are replaceable data, not hardcoded
application operations. The initial menu bar is a single horizontal command
menu (F10, Left/Right, Enter, Escape), not a complete pull-down hierarchy.

Application owns Terminal during `run`, calls initialise/shutdown hooks and
propagates I/O errors. Drop restores terminal state on error/unwind. Context
provides redraw, quit, queued application events, deadlines and child command
handoff through Terminal::run_command. Suspension remains paired with engine
resume; resuming invalidates layout/rendering even if dimensions did not change.
The framework does not intercept process aborts or promise recovery from SIGKILL.

Application-defined events are generic, queued on the UI thread and delivered
to the application callback. This MVP does not provide a cross-thread wake
channel. Background work may request a timer for polling; a future native wake
source can integrate with the existing Unix event descriptors and Windows wait.

## Scheduling and animation

Idle applications block in read_event. Requested one-shot timers establish the
next wake deadline; timeouts are recomputed after input, due timers are removed
before dispatch, and redraw is requested explicitly. No periodic tick exists by
default. A cell animation schedules its next engine Deadline and changes cells.
UI easing/transitions will belong in TerraTUI; do not mix them into Terminal.

## Optional graphics plan

Kitty implementation is deferred and adds no dependency to the MVP. Introduce
an opt-in `kitty-graphics` feature only with protocol and terminal tests. Model
graphics support and animation support independently as unknown/supported/
unsupported. A TERM string alone is insufficient evidence. Probe through a
bounded request/reply exchange that does not consume keyboard input; handle
multiplexers and timeouts explicitly.

The future API owns image/placement IDs, bounded chunked transmission, image
formats and alpha, placement/deletion lifetimes, clipping, resize re-placement
and cleanup on suspend/drop. Animation adds frame upload, durations and control
operations only after separate negotiation. Unsupported/unknown graphics uses
application-selected text/cell placeholders; supported graphics without native
animation uses scheduled static placements only if the application opts in.
No silent graphics output on unsupported terminals. Sixel can be another
optional protocol later; no general plugin system or desktop backend is needed.

## Integration and platform strategy

TDX should first update its dependency to
`terrabox = { path = "../terrabox/crates/terrabox" }` and imports to `terrabox`.
Then replace its Frame with Surface, followed by focus/menu/layout incrementally.
Its document/view and guest terminal models remain intact. Navi8or can replace
its Rust terminal/frame boundary first, preserving providers and keymap policy,
then adopt Desktop and widgets. Neither reference repository is modified here.

Linux PTY tests exercise actual lifecycle and byte output. Windows cross-checks
build native paths but cannot validate console interaction; rerun existing
PowerShell host tests and manual smoke checks on Windows. macOS requires native
build/lifecycle verification; shared Unix code alone is not proof. Keep the
historical Windows JSON report as historical evidence, not renamed output.
