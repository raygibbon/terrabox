# Workspace migration validation — 2026-10-17

## Preservation and layout

The original repository was evolved in place, preserving its `.git` directory,
HEAD `f812fa3859dfc408a969ee76681490b0735b6978`, history and existing uncommitted
rename work. The final local directory is `/home/rgibbon/Sync/git/terrabox`.
The remote remains `git@github.com:raygibbon/terrabox.git`, as it was at task
entry. No reference application was modified, no commit/push was made and
nothing was published. Historical Windows result paths remain unchanged.

The engine's implementation moved into `crates/terrabox`; its existing
examples, tests and fixtures moved with it so package archives remain complete.
The root is a virtual Cargo workspace with two library members. The separate
fuzz workspace references the new engine path and has a regenerated lockfile.
The engine's platform/lifecycle/parser implementation is preserved; additions
are surfaces, clipped drawing, portable row scrolling and deadlines.

## Checks

| Check | Result |
|---|---|
| `cargo fmt --all --check` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass on Linux |
| `cargo test --workspace` | Pass: 71 tests and 1 doctest |
| `cargo build --workspace` | Pass on Linux |
| `cargo build --workspace --release` | Pass on Linux |
| `cargo build --workspace --examples` | Pass on Linux |
| `cargo doc --workspace --no-deps` | Pass |
| `cargo check --manifest-path fuzz/Cargo.toml --all-targets` | Pass |
| `cargo build -p terrabox --no-default-features` | Pass independently of TerraTUI |
| `cargo build --workspace --all-targets --target x86_64-pc-windows-gnu` | Pass: libraries, examples and test binaries built |
| Windows GNU workspace Clippy with `-D warnings` | Pass |
| `cargo package --workspace` | Both archives assembled and verified in a clean temporary source snapshot |
| `python3 tools/desktop-smoke.py` | Pass in a real Linux PTY |

Checks used the local cached dependencies (`--offline`); no dependency upgrades
were requested. Packaging did not bypass dirty-tree checks or alter the checkout.
Cargo's temporary registry verified TerraTUI against the packaged TerraBox.
Registry name/version availability and actual publication were not tested.

## Behavioural coverage

All 57 existing engine tests continue to pass. Six new engine tests cover
nested clips, grapheme boundaries, clipping across existing wide cells, source
and destination blits, region scrolling/fill, empty/extreme rectangles and
deadline saturation. Seven framework tests cover stable focus, nested modal
restoration, scoped commands, menu dispatch, tiny nested layouts, recomposition,
retained widget updates and Unicode. One real-PTY application test covers
one-shot/cancelled timers, queued messages, init/event errors, shutdown and
terminal restoration on normal return and panic unwinding.

The Linux demo smoke test checks initial output, no unsolicited idle frames,
focus redraw, modal global-shortcut capture, resize redraw, Escape/Enter closure,
menu-command quit and restored terminal attributes. It is automated terminal
behaviour coverage, not physical keyboard/font/colour visual sign-off.

## Limits and integration follow-up

Windows code was cross-built and linted, not executed during this migration.
Run the existing PowerShell console-host tests and interactive checks on native
Windows. macOS was not built or tested here. Existing native Windows reports are
historical and do not renew final-source acceptance.

TDX still references `../terra-term` and the `terra-term` package in its own
manifest. Its owner must update that dependency to `../terrabox/crates/terrabox`
and imports to `terrabox` in a separate change. Navi8or's Rust terminal layer can
be replaced incrementally; neither application has been migrated here.

The MVP has tiled panels and centred modal overlays, a horizontal command menu,
scoped single-key bindings and UI-thread messages. Pull-down menus, editable
inputs, trees, arbitrary floating windows, mouse hit testing, cross-thread wake
events, hardware scrolling and graphics/animation protocols remain roadmap work.
Idle behaviour is demand-driven; CPU usage has not been benchmarked numerically.
