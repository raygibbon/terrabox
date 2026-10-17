# Development roadmap

## Essential MVP (this migration)

- Preserve Git history, prior rename changes and existing terminal behaviour.
- Publishable TerraBox/TerraTUI workspace with registered examples and tests.
- Public Rect, opaque Surface and clipped Canvas using existing Unicode cells.
- Portable vertical region scroll and optional frame deadlines.
- Retained application lifecycle, event consumption and custom widget boundary.
- Stable focus IDs, modal capture/restoration, local/global command bindings.
- Nested horizontal/vertical fixed/flex layouts with tiny-size handling.
- Panels, single command menu bar, centred modal dialog and status bar.
- Demonstration of focus, commands, modal closure, resize and clean shutdown.
- Linux regression and framework tests; available Windows cross-build checks.

## Phase 2 delivered

- Nested focus groups; widget-first keyboard handling and configurable traversal.
- Application-scoped background messages with native wakeups and cooperative shutdown.
- Labels, buttons, Unicode input, editable modal forms, lists, trees and progress.
- Dropdown menus, mnemonics, shortcuts, disabled items, separators and mouse handling.
- Minimum-size flex allocations and collapsed layout nodes.
- Region-only scroll snapshots backed by measurements and Unicode comparison tests.
- Four interactive examples, real terminal screenshots and repeatable capture tools.

## Next: application adoption and platform verification

- Adopt the Surface/focus/widget boundaries incrementally in TDX and Navi8or,
  with changes performed separately in those repositories.
- Native final-source Windows console tests and macOS build/lifecycle validation.
- Scrollbars, tree selection/activation commands and richer text input/cursor UX.
- Scrolling large menus and nested submenus when demanded by real commands.
- Consumer-controlled alternate layouts for very small terminals.
- Bounded/coalescing worker update policies if integration exposes queue pressure.
- Floating geometry, drag/resize and persisted layout adapters.
- Measure frame allocations and terminal output before further rendering changes.

## Later, optional and separately validated

- Capability-aware physical scroll regions with portable surface fallback.
- Public verified terminal capability reporting, including colour fallbacks.
- Kitty graphics probes, uploads, alpha, placements, clipping and ID cleanup.
- Separate Kitty native animation negotiation, frames and timing controls.
- UI easing/transitions without an always-running render loop.
- Sixel if actual applications need it.

Do not add document editing, file providers, syntax parsers, guest PTY emulation,
an async runtime requirement or a graphical desktop backend to these libraries.
