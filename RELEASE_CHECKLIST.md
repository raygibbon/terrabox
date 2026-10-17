# v0.1.0 release checklist

This checklist records the pre-workspace engine release baseline, not final
approval of the new framework. See `docs/validation.md` for current results.
For a new release, rerun workspace checks and `cargo package --workspace` from
a clean checkout/snapshot. Publish TerraBox before TerraTUI. Native Windows
interactive and macOS validation remain required before claiming those hosts.

Scope: synchronous terminal rendering and input/events with native Unix/Windows backends. No widgets, layout engine, app framework, or async runtime.

Linux automated validation passed on 2026-10-03, including 57 tests and one doctest. Windows GNU cross-check and Clippy also passed; native Windows execution and interactive sign-off remain pending.

## Automated

Run against the final release sources. Packaging and dry-run must use a clean checkout or clean temporary source snapshot; do not bypass dirty-tree checks.

- [x] Format: `cargo fmt --check`
- [x] Check: `cargo check --all-targets`
- [x] Tests: `cargo test --all-targets`
- [x] Clippy: `cargo clippy --all-targets -- -D warnings`
- [x] Docs: `cargo doc --no-deps` and `cargo test --doc`
- [x] Release build: `cargo build --release`
- [x] Package: `cargo package`
- [x] Publish dry-run: `cargo publish --dry-run`

Repeat native checks on Windows; Linux cross-compilation does not run Windows console tests.

## Manual Windows smoke test

Record the host/version and observations. Repeat in classic Console Host where supported. Existing September host results do not complete this final-source sign-off.

- [ ] Launch in Windows Terminal.
- [ ] Verify text rendering.
- [ ] Verify RGB colours visually.
- [ ] Verify Unicode/graphemes, combining characters, and wide-cell clipping.
- [ ] Verify arrows.
- [ ] Verify function keys.
- [ ] Verify Ctrl combinations including physical Ctrl+\, Ctrl+C key input, and Ctrl+Break survival while active.
- [ ] Verify mouse clicks and button release.
- [ ] Verify vertical and horizontal mouse wheel.
- [ ] Resize repeatedly while drawing in the alternate screen.
- [ ] Paste (native Windows paste arrives as key records).
- [ ] Suspend/resume if supported; check mouse settings and same-size buffer redraw.
- [ ] Quit and verify screen, cursor, console modes, and code page are fully restored.

Do not tag until final-source native Windows checks and this manual pass are signed off.
