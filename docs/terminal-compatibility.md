# Terminal compatibility and manual validation

Support is determined by the actual connection and replies, not an OS or brand.
The following is a validation plan, **not a list of manually tested terminals**.
The [current report](phase3-validation.md) records completed checks.

| Connection | Current library policy / next check |
|---|---|
| Kitty, direct | Probe static support; manually validate RGBA, alpha, replacement, placement and cleanup. Native animation remains separate/deferred. |
| Ghostty, direct | Probe rather than infer from branding; repeat the full static checklist for the installed version. |
| WezTerm, direct | Treat as unknown until the query succeeds; other image protocols or `imgcat` alone do not establish Kitty compatibility. |
| tmux | If TMUX is present, block probes/images. Native handling and passthrough vary by version/configuration and require a future explicit transport. |
| GNU Screen | If STY is present, block probes/images. No passthrough encoder or native graphics negotiation. |
| Windows Terminal / Console Host | Native console records supply input; VT runtime probes are skipped. Unknown graphics gives text fallback. Explicit app override can assert support, but is not validation. |
| xterm | Used here for real capability-report and cell-animation captures. No graphics rendering claim. |
| macOS | Shared Unix path; build/runtime validation on a native host still needed. Test direct Kitty/Ghostty/WezTerm separately. |

The official [Kitty protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/)
specifies static and animation operations separately. Ghostty documents
[Kitty graphics support](https://ghostty.org/docs/features), but this does not
validate our encoder against it. Consult [WezTerm escape-sequence documentation](https://wezterm.org/escape-sequences.html)
for the installed version, and the [tmux FAQ](https://github.com/tmux/tmux/wiki/FAQ)
for transport configuration. No external compatibility claim substitutes for a
runtime reply and a visual test of this implementation.

## Repeatable manual checklist

Record OS, emulator version, font, TERM/COLORTERM, whether remote/multiplexed,
and exact commit/build. Avoid recording private paths or hostnames in captures.

1. Build with `cargo build --workspace --examples --all-features`.
2. Run `terminal-capabilities` directly. Verify statuses/evidence; type keys and
   paste during startup, then resize and quit. Missing replies must stay unknown.
3. Run `kitty-image`. If the query is unknown/rejected, verify explanatory text
   and responsive controls. Never force support just to obtain a screenshot.
4. On confirmed static support, inspect the procedural image, transparency and
   border. Press R/Space repeatedly, D to delete, U to show, and resize larger
   and smaller. Check old pixels disappear and image anchors stay aligned.
5. Press C to toggle scheduled static replacement. This is not native animation.
   Pause it, wait idle, then quit. Verify no image survives in the shell.
6. Run `terratui-image-viewer`. Navigate all three images, Tab between panels,
   resize down to tiny dimensions and restore. F2 hides/shows the preview; check
   removal and reappearance. F1 opens help: images must not cover its text.
   Close help, check restoration, then Ctrl+Q and inspect shell state.
7. Run `terrabox-animation`. Space pauses/resumes, +/- changes speed; resize and
   quit. Paused input should block without periodic redraws.
8. Exercise suspend/resume through a host application's `run_command`; prior
   image handles must become invalid and child/shell output must be clean.
9. Repeat text/fallback checks under tmux and Screen, and native Windows Terminal.
   Record skips rather than treating uninstalled tools as passing tests.
10. Run a small host application that returns an I/O error or unwinds after
    image placement; check image cleanup, cursor, echo and alternate-screen exit.
    SIGKILL/abort are outside RAII guarantees.

Protocol tests and simulated PTY replies validate bytes, ordering and lifecycle;
they do not validate emulator rasterization, scaling, font interaction or flicker.
Native Kitty animation requires an additional independent test matrix before
frame uploads or playback controls can be called supported.
