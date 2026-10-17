# TerraBox + TerraTUI

A Rust terminal engine and application framework in one Cargo workspace,
evolved from terra-term with its existing history and native backends intact.

| Crate | Responsibility |
|---|---|
| `terrabox` | Terminal lifecycle, input, grapheme cells, changed-row rendering, clipped surfaces and region scrolling |
| `terratui` | Retained applications, commands, focus, nested splits, panels, menu bar, modal dialogs, status and timers |

TerraBox is independently usable. TerraTUI depends on TerraBox and reuses its
cells, styles, input and rendering. Neither requires an async runtime or GUI.
Files, documents, syntax, providers and application operations stay in consumers.

## Run

```sh
cargo run -p terratui --example desktop
cargo run -p terrabox --example hello
```

In the desktop demo: Tab/Shift-Tab switch panels, arrow/page keys navigate,
F10 opens the command menu, F1 opens a modal, Enter/Escape closes it, and Ctrl+Q
quits. Resizing reflows the layout. Idle applications block for input; timers
and redraws run only on request. The menu MVP is a horizontal command menu;
pull-down menus and composite input widgets are follow-up work.

## Use the libraries

For a sibling checkout, depend on `../terrabox/crates/terrabox` or
`../terrabox/crates/terratui`. Imports use `terrabox` and `terratui`.
Each crate has its own version and manifest. Publish TerraBox before TerraTUI,
whose dependency includes both a local path and registry version.

Engine source, tests and examples live under `crates/terrabox/`; framework
source, tests and the desktop demo live under `crates/terratui/`. Test fixtures
and licence files travel with their crate for packaging. `fuzz/` is a separate
workspace referencing the engine and retains its own lockfile.

## Validate

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
cargo check --manifest-path fuzz/Cargo.toml --all-targets
```

Linux has PTY regression tests. Windows has native console support and host
validation scripts; cross-compilation alone does not verify console behaviour.
macOS native validation remains pending. See [validation](docs/validation.md)
for this migration's exact results. Kitty graphics and advanced animations are
planned, not implemented; no graphics support is assumed or probed by default.

## Design and integration

- [Repository audit and ownership matrix](docs/architecture-review.md)
- [Architecture and graphics integration plan](docs/architecture.md)
- [MVP and future roadmap](docs/roadmap.md)
- [Engine guide](docs/ENGINE.md) and [capabilities](docs/FEATURES.md)
- [Framework usage](docs/TERRATUI.md)
- [Windows validation](docs/WINDOWS_VALIDATION.md)

MIT licensed. Adapted termbox2 capability tables retain their
[third-party notice](THIRD_PARTY_NOTICES.md).
