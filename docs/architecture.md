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
Portable region scrolling moves logical cells and fills exposed rows. It snapshots
only the affected rectangle rather than cloning the whole surface; a repeatable
release workload and Unicode boundary comparison validate this change. Hardware
terminal scroll commands are deferred until front-buffer synchronisation and
capability fallbacks have dedicated tests.

## Terminal capabilities and provenance

Unix loads installed compiled terminfo strings first, honoring environment and
system search paths and both character/hex directory layouts. The reader
supports standard strings with either numeric width, bounds reads and rejects
malformed offsets. Database entries remain authoritative; cancelled/missing
fields are not filled from an assumed family. Usable cursor addressing and
screen clearing are checked before entering raw mode.

Without a usable entry, a small output builder supplies documented core VT
operations for an explicit compatibility allowlist. Linux uses the main screen;
older rxvt/Eterm use legacy buffer switching; other allowed modern VT names use
saved alternate buffers. Standard function/navigation input is decoded by the
existing parser, with one Linux BackTab registration. Windows uses documented
VT output and native input independently of Unix databases. No ncurses linking,
new crate or vendored capability dataset is needed.

The former adapted family arrays and positional mapping were removed. The
[terminal capability audit](terminal-capability-audit.md) records source
references, retained-code review, fallback limitations, historical credit and
the basis for updating active third-party notices. TERM selects a compatibility
policy, not proof of graphics, truecolour or extended keyboard support.

## Retained state and layout

TerraTUI retains a Desktop of windows with stable WindowId values and boxed
stateful Widget objects. Each widget exposes drawing and event consumption.
Applications may add custom widgets that render styled content using TerraBox
types. A recursive Layout supports horizontal/vertical splits and fixed/flex
lengths, including minimum-size flex allocation and collapsed children. Short
terminals truncate in declaration order and distribute integer remainders
deterministically; zero-sized regions are valid.

Focus is a stable ID within its owner, not an array position or document ID.
Desktop owns a base window order and modal stack; Group owns its own child order
and remembered focus. Groups nest recursively. Focused children receive keys
before parent default navigation. Each group chooses cycle, bubble-at-edge or
manual Tab policy; desktop default traversal is optional. Modal forms trap Tab
and Shift+Tab, and modal closure restores the previous retained window and child
state. Rendering reconciles focus only when visibility invalidates the target.
Mouse hit testing translates coordinates at every ownership boundary. Draw order
is base windows, menu overlays, then modals. Arbitrary draggable floating windows
remain deferred. See [framework routing details](TERRATUI.md).

Compared with immediate-mode UI, retained state avoids repeatedly rebuilding
selection, focus and menu state, and gives commands persistent targets. Costs
include explicit mutation/redraw requests, stable identity management and boxed
widget dispatch. Logical rendering remains a straightforward redraw on change;
this is not a full cached scene graph or a Ratatui widget API clone.

## Input, commands and application lifecycle

Native events flow from parser/backend through Application to Desktop. Modal
capture comes first. Otherwise menu activation/capture precedes ordinary input;
focused widgets receive remaining keys before local bindings, default traversal
and global bindings. A widget consumes keys explicitly, including Tab for editor
indentation. Local commands override global commands. Modal input never falls
through to base/global bindings. MenuBar retains its original horizontal command
API and now also owns dropdowns with disabled entries, separators, shortcuts,
mnemonics and keyboard/mouse dispatch. Nested submenus are deferred until a
consumer needs them; the examples' small menus do not require a hierarchy.

Application owns Terminal during `run`, calls initialise/shutdown hooks and
propagates I/O errors. Drop restores terminal state on error/unwind. Context
provides redraw, quit, queued application events, deadlines and child command
handoff through Terminal::run_command. Suspension remains paired with engine
resume; resuming invalidates layout/rendering even if dimensions did not change.
The framework does not intercept process aborts or promise recovery from SIGKILL.

Application-defined messages use a per-run standard MPSC channel and a cloneable
EventSender. Enqueue precedes a coalescing TerraBox notification. Unix backends
poll a nonblocking socket alongside terminal input and SIGWINCH; Windows waits
on an auto-reset event and console input. The low-level Waker owns its native
resource through Arc, independent of terminal lifetime; it exposes no widgets
or application types. Terminal::wait_event returns None on notification, while
read_event continues waiting. UI state remains exclusively on the application
thread; messages can carry stable IDs to resolve with widget_mut.

The sender closes before shutdown and during unwind. Workers cooperatively
check is_closed; applications own/join task handles. No scheduler or async
runtime was added. Input is checked between queued messages. The channel is
unbounded, so producers must limit or coalesce high-rate updates.

## Scheduling and animation

Idle applications block in wait_event on native input/notification sources. Requested one-shot timers establish the
next wake deadline; timeouts are recomputed after input, due timers are removed
before dispatch, and redraw is requested explicitly. No periodic tick exists by
default. A cell animation schedules its next engine Deadline and changes cells.
UI easing/transitions will belong in TerraTUI; do not mix them into Terminal.

## Optional graphics

The `kitty-graphics` feature adds a terminal-owned resource registry beside the
cell buffers. Explicit bounded probes report tri-state evidence and preserve
unrelated events. Static RGBA uploads, staged deletions/placements and final
cursor restoration share the terminal output path. Retained widget requests
reuse uploaded images; menus/modals suppress the image scene. No decoder,
mandatory tick or second backend is introduced.

See the [Phase 3 design audit](phase3-architecture.md) and
[graphics guide](graphics.md) for lifecycle, bounds and transport policy.
Native Kitty animation, Sixel rendering and multiplexer forwarding are deferred.
FrameClock provides caller-controlled cell pacing without an idle timer.

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
