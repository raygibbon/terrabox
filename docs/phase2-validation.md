# Phase 2 validation — 17 October 2026

Changes were made in the existing repository. The pre-existing untracked
`Archive.zip` was left untouched. No history, remotes, licensing files or
reference repositories were changed; no commit or push was made.
The earlier [migration report](validation.md) remains historical evidence.

## Implemented

Nested child focus groups, widget-first key routing, configurable traversal,
modal forms with focus trapping/restoration, thread-safe application messages,
native event-loop wakeups, cooperative worker shutdown, dropdown menus,
buttons/labels/Unicode input, selectable lists, expandable trees, progress,
minimum-size and collapsed layouts, and four functional examples.

TerraBox remains independent. Its only renderer optimization snapshots the
scroll rectangle rather than cloning the entire surface. Changed-row output,
Unicode cell rules, clipping and terminal restoration are preserved. The
examples use actual filesystem reads only; business logic stays in the browser.

## Commands executed

| Check | Result |
|---|---|
| `cargo fmt --all --check` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `cargo test --workspace` | Pass: 77 tests and 1 doctest |
| `cargo build --workspace` | Pass on Linux |
| `cargo build --workspace --examples` | Pass on Linux |
| `cargo clippy --workspace --all-targets --target x86_64-pc-windows-gnu -- -D warnings` | Pass |
| `cargo build --workspace --all-targets --target x86_64-pc-windows-gnu` | Pass: libraries, examples and test binaries |
| `cargo check --manifest-path fuzz/Cargo.toml --all-targets` | Pass |
| `cargo build -p terrabox --no-default-features` | Pass independently |
| `cargo doc --workspace --no-deps` | Pass |
| `python3 scripts/smoke-examples.py` | Pass in Unix PTYs for all four examples |
| `python3 scripts/capture-examples.py` | Pass: actual xterm captures and keyboard exits |
| `cargo run --release -p terrabox --example scroll-bench` | Executed before and after optimization |
| `git diff --check` | Pass |

Windows results are cross-build checks, not native console verification. No
macOS Rust target is installed, and no macOS build or runtime check is claimed.
No changes were needed in Navi8or or TDX; they were read-only references.

## Behavioral evidence

- Regression tests cover existing engine input, parser, Unicode, surface and
  terminal lifecycle behavior. An empty-list draw underflow found during new
  focus tests was fixed.
- New controls tests exercise nested bubbling, reverse traversal, input consuming
  Tab, retained focus across repeated redraws, modal traps/restoration,
  accept/cancel dispatch, grapheme deletion, dropdown mnemonics, disabled items,
  shortcuts, minimum/collapsed layouts, tree parent/child navigation and mouse
  hit testing that does not activate a focused button when a label is clicked.
- The serialized framework PTY test covers worker wakeup without keyboard input,
  sender closure, worker join, notification bursts, keyboard priority over
  pending notifications, errors, timers, shutdown and terminal restoration.
- A surface regression compares 18,900 scroll combinations with a full-source
  snapshot reference, including partial wide glyphs at both region edges,
  combining clusters, emoji and extreme positive/negative scroll distances.
- The PTY example script exercises panel traversal, menu/form interaction,
  editable Unicode, list/tree layouts, directory navigation and file metadata,
  background progress, resize to 60×18, normal exit and exact termios restoration.
- Linux `/proc` CPU samples measured **0 ticks over 400 ms** for each idle
  TerraTUI example. This is a short smoke measurement, not a long-duration CPU
  benchmark. The renderer deliberately animates until paused.

## Scroll measurements

Release profile, one run per implementation on this Linux host. Each workload
uses a 200×60 surface, performs 2,000 alternating upward/downward one-row scrolls
and passes the surface through `black_box`. Timing excludes setup/compilation.

| Region | Previous full-buffer snapshot | Region-only snapshot | Reduction |
|---|---:|---:|---:|
| 20×10 | 532.10 ms | 154.22 ms | approximately 71% |
| 200×60 | 11.582 s | 9.089 s | approximately 22% |

These coarse host-specific measurements justify the simpler allocation change;
they are not statistical guarantees. Rerun `scroll-bench` to compare future
changes. Empty rectangles now return without allocating. Full-frame rebuilding,
cell cloning inside the region and existing changed-row output remain; no
cache, hardware scrolling or changed-cell renderer was introduced.

## Screenshots

Five 1204×724 PNGs were captured from actual 100×30 xterm windows using
ImageMagick `import`, inspected for readability and personal information:

- `assets/screenshots/terrabox-demo.png`
- `assets/screenshots/terratui-desktop.png`
- `assets/screenshots/terratui-files.png`
- `assets/screenshots/terratui-widgets.png`
- `assets/screenshots/terratui-dialog.png`

The files capture uses a temporary fixture containing generated sample files.
No personal directory names, usernames or sensitive paths appear. Reproducible
instructions are in [the screenshot guide](../assets/screenshots/README.md).
No mock or generated UI imagery was used.

## Remaining limitations and next milestone

- Native Windows console and macOS verification are pending.
- Menus provide one dropdown level. Nested submenus and scrolling oversized menus
  are deferred; the example command sets fit a single level.
- Text input is single-line with grapheme movement/deletion and printable paste;
  text selection, mouse cursor positioning, IME integration and masking are absent.
- Minimum dimensions truncate by declaration order when space is insufficient;
  applications choose their own alternate layout. There are no scrollbars or
  draggable floating windows.
- Worker cancellation is cooperative and message queues are unbounded. Senders
  should limit/coalesce updates; application shutdown owns joining workers.
- Kitty graphics, image placements and advanced animation remain planned.

Recommended next milestone: adopt these APIs in a small real consumer, then
validate final source on native Windows and macOS. Let that integration drive
scrollbars, richer tree activation and large-menu behavior before adding more
framework abstractions.
