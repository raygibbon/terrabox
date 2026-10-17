# Phase 3 validation report

Validated on Linux with Rust 1.97.1 / Cargo 1.97.1. The initial working tree was
clean. Changes remain uncommitted; history, remotes, license/notices, the prior
capability provenance audit and the five existing screenshots were preserved.
No other repository was modified. No new Rust dependency or downloaded test
asset was added. Graphics examples generate their own RGBA data.

## Automated results

| Command / check | Result |
|---|---|
| `cargo fmt --all --check` | Passed |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Passed |
| `cargo test --workspace --all-features` | 104 passed, including one doctest; no failures |
| `cargo build --workspace --all-features` | Passed |
| `cargo build --workspace --examples` | Passed; graphics-only examples correctly require their feature |
| `cargo build --workspace --examples --all-features` | Passed |
| `cargo test --workspace --no-default-features` | 93 passed, including one doctest; no failures |
| `cargo clippy --workspace --all-targets --no-default-features -- -D warnings` | Passed |
| `cargo clippy --workspace --all-targets --all-features --target x86_64-pc-windows-gnu -- -D warnings` | Passed |
| `cargo build --workspace --all-features --target x86_64-pc-windows-gnu` | Passed; cross-build only |
| Fuzz harness workspace all-target check and Clippy with `-D warnings` | Passed; no extended fuzz campaign claimed |
| `python3 scripts/smoke-examples.py` | All eight examples passed input, resize, quit and termios restoration checks |
| `git diff --check` | Passed |

The no-default-feature run exercises ordinary rendering without compiling the
optional graphics API. Protocol tests run without a Kitty terminal. Tests cover
Base64 vectors/chunk boundaries, RGBA validation, IDs, placement bounds, pending
resource limits, movement/deletion order, retries after failed writes,
interrupted upload cleanup, asynchronous rejection, scene reuse/hide/resize,
parser fragmentation and recovery, timing/pause/rate changes, and TerraTUI
layout/fallback/menu/modal behavior.

A Linux PTY protocol responder supplies explicit capability replies and verifies
input replay, cached queries, silence/timeouts, multiplexer blocking, cell-before-
placement order, resize reuse, suspend invalidation and cleanup before screen
restoration. It is a byte-level responder, not an emulator. It does not verify
rasterization, transparency appearance, image scaling or flicker.

Smoke tests deliberately provide no protocol replies. Both graphics examples
must remain text-only and send no pixel uploads. All six measured idle examples
reported zero CPU ticks over 400ms after startup. The continuously animating
examples are not described as idle. An initial capability-example idle check
sampled the 250ms startup probe and failed at two CPU ticks; the measurement now
starts after the probe. The final run passed without relaxing the idle threshold.

Windows cross-target Clippy initially found an unread terminfo metadata field
in the test configuration. A numeric metadata test now validates both integer
widths and absent/cancelled values, and the final Windows check passes.

## Real captures and interactive scope

XTerm 398 on X11/Xwayland was used to run and capture the real capability report
and cell animation in 100×30 windows, DejaVu Sans Mono 12pt. The resulting PNGs
were visually reviewed for layout/readability and private information. The
animation GIF records successive actual running frames, with capture timings;
the paused PNG remains available. The GIF contains 60 distinct frames over
4.81 seconds at 1204×724 pixels (about 305 KiB), loops continuously, and was
checked at representative frames. Capture automation exits through normal keys.
The original five images are unchanged.

The captured xterm session positively reports mouse and bracketed-paste support
and has terminfo alternate-screen evidence; unconfirmed extensions remain
unknown. This is not a claim about every xterm configuration. No captured image
pretends to show Kitty pixels.

No Kitty, Ghostty or WezTerm emulator was available. **Actual Kitty image
rendering has not been visually validated here.** There are no fabricated
`kitty-image.png` or `terratui-image-viewer.png` captures. Compatible-host capture
steps are in [the screenshot guide](../assets/screenshots/README.md#graphics-captures).
Native Windows runtime, macOS, tmux and GNU Screen interactive validation also
remain pending; cross-builds and policy tests do not establish runtime results.
Use the [manual checklist](terminal-compatibility.md) and record exact versions.

## Delivered scope and remaining limitations

Implemented: tri-state capability evidence and explicit cached probes; optional
static Kitty direct RGBA transfer; bounded image/placement ownership; coordinated
cell output, deletion, resize and shutdown; caller-driven FrameClock; optional
TerraTUI ImageView; four functional examples and actual text/cell captures.

Native Kitty animation is deliberately deferred after protocol investigation.
Its capability is independent and remains unknown. The image demo offers
explicit scheduled full-image replacement, not native frame upload/playback.
Static support is insufficient evidence for animation operations in different
emulators. Complete real-emulator static checks before adding that API.

Other limitations: no decoder, Sixel renderer, multiplexer forwarding, CSI-u key
activation/decoding, pixel crop, pixel metrics or aspect-ratio policy. Placements
must fit whole cell rectangles. Menus/modals temporarily release all desktop
images and restore them by upload afterward. Synchronous uploads may block;
broken connections, abort or SIGKILL can defeat best-effort cleanup.

Recommended next step: run the direct Kitty/Ghostty/WezTerm checklist and capture
the two actual image demos, then validate native macOS/Windows lifecycle paths.
Animation negotiation and multiplexer transport should follow as separate,
testable additions. See [API/ownership details](graphics.md) and
[architecture rationale](phase3-architecture.md).
