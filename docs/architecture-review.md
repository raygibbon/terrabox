# Repository audit — 2026-10-17

## Scope and preservation

Inspected the existing engine, Navi8or (C implementation and experimental Rust
core), and TDX in their actual local repositories. The reference applications
remain read-only. This work evolves the existing Git repository and keeps its
history and MIT/termbox2 attribution.

At entry, 32 tracked files had uncommitted changes from the preceding rename:
the manifests and lockfiles, licence and notices, README and feature/platform
docs, all nine examples, fuzz sources, engine doctest, Windows tests/scripts,
and integration tests. These changes must be carried into the workspace.
The existing remote is `git@github.com:raygibbon/terrabox.git`; this task does
not change it. The destination directory did not exist at audit time.

## Engine baseline

`src/terminal.rs` owns the native backend, logical back buffer and physical
front buffer. It already supplies single-owner RAII restoration, raw input,
alternate screen, cursor and mouse modes, suspend/resume, and child command
handoff. Preserve these paths rather than adding another terminal abstraction.
Unix uses a controlling TTY, owned descriptors and SIGWINCH wake pipe. Windows
uses console records and native console lifecycle handling with VT output.

`buffer.rs` and `cell.rs` already handle grapheme clusters, wide continuations,
cluster extension, style changes and right-edge clipping. `present` compares
rows, emitting changed rows; it is not a changed-cell renderer. Resize clears
the logical screen and invalidates the physical screen. Input parsing includes
bounded paste chunks, registered sequences, modified keys, malformed-sequence
recovery and explicit Escape grace periods. Windows paste arrives as keys.
Compiled terminfo and built-in capabilities are internal; output still assumes
VT colour support. There is no public surface, clip view, region scroll,
frame deadline helper or graphics protocol implementation.

Dependencies: unicode-width, unicode-segmentation, Unix libc, Windows windows-sys.
No async runtime or framework dependency. Preserve the MIT licence and
THIRD_PARTY_NOTICES for adapted termbox2 sequence tables. Reference code is
studied for requirements, not copied into the libraries.

The baseline passes 57 Linux tests plus one doctest, with PTY lifecycle,
redirected-output, parser corpus/property/recovery and Unicode buffer coverage.
Windows GNU checking and Clippy pass, but this does not execute Windows code.
Historical native Windows evidence is retained unchanged; final-source manual
validation remains open. macOS has not been built or tested in this environment.

## Navi8or findings

The root Cargo workspace contains `rust/nav-rs`; the C application is still
present. Rust `ui/layout.rs` reserves menu, dual panes, status and keybar rows,
with a minimum usable size. `ui/render.rs` draws frames, column headers,
selection, path titles, summaries, menus and overlays. `terminal/mod.rs` has
its own cells, frame, platform polling and presentation. `keys.rs` resolves
configurable one/two-key sequences in panel/global/modal contexts. `ui/menu.rs`
retains the selected menu item and skips separators; `ui/mod.rs` captures modal
input and resets pending keys on resize/context changes. `app.rs` owns pane
selection, offsets, sorting, provider navigation and operations. `profile.rs`
and C `theme.c` use semantic colour roles. C `ui/core/window.c` saves/restores
screen regions and draws frames/shadows; these needs justify shared surfaces.

Reusable needs: pane geometry, focus, menu state, modal capture, role styles,
clipping, selection visibility and status/key labels. Providers, credentials,
network/file operations, filtering semantics and viewers stay in Navi8or.
The experimental Rust UI is smaller than the C UI: do not infer that every C
dialog or operation is already implemented in Rust.
In particular, `viewer_bridge.rs` launches the `nav-viewer-c` helper and accepts
only validated local files/file symlinks, preserving resource identity rather
than display names. Shared terminal handoff belongs in the libraries; viewer
selection, provider validation and actual viewing remain application work.

## TDX findings

`Cargo.toml` still depends on `terra-term` at `../terra-term`. This is an
existing consumer incompatibility with the rename; document the new dependency
instead of modifying the application. `app/event_loop.rs` already uses the
engine, handles external command handoff, paste assembly, resize and background
jobs. It sometimes polls at 10/16/50/250 ms; the new framework should wait
indefinitely when idle and use deadlines only for requested work.

`ui/render.rs` builds another application-owned cell frame and transfers it to
the engine, including styles, grapheme widths and cursor. `ui/layout.rs`,
`workspace/geometry.rs` and `workspace/panes.rs` manage splits, hidden views,
zoom, resize reconciliation and cursor visibility. `app/focus.rs` prioritises
dialogs/menus/tree/terminal/editor and explicitly distinguishes view identity
from document identity. `ui/menu.rs`, menu rendering and search/grep dialogs
need generic menu, focus and modal machinery. `ui/window_render.rs` implements
an overlapping window picker, borders and shadows. Syntax and prose rendering
need styled cell spans, not a shared parser/editor model. `terminal/` is an
embedded PTY emulator, not a replacement host terminal backend.
`command/` already separates binding resolution, prefixes, availability and
contextual enablement. `ui/search_dialog.rs` retains grapheme caret, insert and
selection state alongside application-specific search options. Generic input
state and disabled controls are follow-up widget needs; search/replace option
relationships stay in TDX. The language picker paints registry metadata, which
also argues for reusable selection geometry rather than shared language logic.

Reusable needs: stable view IDs, nested splits, clipping/compositing, scoped
commands, modal focus restoration, status bars, timers and custom widgets.
Documents, editing, search, syntax parsing, recovery, macros, project-tree data,
guest PTY emulation and persistence remain application responsibilities.

## Requirements and ownership matrix

| Capability | TerraBox | TerraTUI | Applications |
|---|---|---|---|
| Raw/alternate screen, restore, child handoff | Existing engine | Lifecycle orchestration | Child command policy |
| Key/mouse/paste/resize collection | Existing engine | Routing and consumption | Domain actions and paste assembly |
| Graphemes, styles, dirty rows | Existing engine | Semantic theme roles | Syntax/prose styles |
| Rectangles, clipped drawing, surfaces | Foundation additions | Compose windows using engine | Content rendering |
| Region scroll | Portable surface operation | Viewport policy | File/document scroll positions |
| Frame deadlines | Low-level deadline helper | Timer dispatch, redraw requests | Background job scheduling |
| Dual panes, nested/fixed/flexible splits | — | Layout | Initial layout and persistence |
| Focus, modal capture/restoration, z-order | — | Retained desktop | Document/view relationships |
| Key bindings, commands, menus/status/dialogs | — | MVP | Command meaning and enablement |
| Lists, trees, inputs, richer popups | — | Follow-up widgets | Item models, filesystem semantics |
| Kitty upload/placement/animation | Deferred optional protocol | UI transitions later | Image selection and fallback policy |
| Files, providers, documents, syntax, guest PTYs | — | — | Navi8or / TDX / Typedeck |

## Compatibility and priority

Keep the public engine API additive and its platform module names stable.
Moving the crate changes path dependencies and source-relative fuzz includes;
examples/tests must remain registered after creating a virtual workspace.
Crate names are independently versioned; TerraTUI uses a versioned path
dependency so published packages can resolve TerraBox. First deliver safe
surfaces and a small retained application framework, not every requested widget
or terminal optimisation. Hardware scroll regions, Kitty, richer inputs/tree
widgets and UI animation belong in the roadmap until separately validated.
