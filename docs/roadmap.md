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

## Next: application adoption

- Migrate TDX's dependency/imports in its own repository, then its duplicate Frame.
- Replace Navi8or's Rust terminal boundary independently of its C implementation.
- Add list/tree/input/button/scrollbar widgets driven by real integration needs.
- Add pull-down/popup menus, disabled commands, mnemonics, configurable key chords
  and richer focus traversal inside composite widgets.
- Add explicit cross-thread wake events and background job integration.
- Add floating window geometry, drag/resize and persisted layout adapters.
- Measure rendering allocation/output costs before changed-cell optimisation.
- Native final-source Windows tests and macOS builds/manual lifecycle checks.

## Later, optional and separately validated

- Capability-aware physical scroll regions with portable surface fallback.
- Public verified terminal capability reporting, including colour fallbacks.
- Kitty graphics probes, uploads, alpha, placements, clipping and ID cleanup.
- Separate Kitty native animation negotiation, frames and timing controls.
- UI easing/transitions without an always-running render loop.
- Sixel if actual applications need it.

Do not add document editing, file providers, syntax parsers, guest PTY emulation,
an async runtime requirement or a graphical desktop backend to these libraries.
