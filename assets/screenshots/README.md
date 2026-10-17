# Actual terminal captures

These PNGs are window captures from running examples, made with xterm and
ImageMagick `import` on Linux/Xwayland. They are not rasterized text, mockups or
AI-generated images. Each is 100×30 terminal cells, 1204×724 pixels, using
DejaVu Sans Mono at 12pt. xterm's blue/cyan palette is set to muted navy/teal;
terminal colours remain user-configurable.

From the repository root:

```sh
cargo build --workspace --examples
python3 scripts/capture-examples.py
```

Requirements: an X11/Xwayland display, `xterm`, `xwininfo`, ImageMagick `import`,
Python 3 and libX11. These are capture tools, not library/runtime dependencies.
The script briefly opens actual terminal windows and sends X key events to its
own VT widgets. It enables synthetic events only on those windows. It exits the
examples through their normal keyboard commands and captures only their client
area, excluding desktop surroundings and title bars.

States captured:

- Desktop: File dropdown open.
- Files: right panel selected, backed by generated temporary sample files.
- Widgets: worker progress active, Start button focused.
- Dialog: editable profile opened during worker progress.
- Renderer: cell animation paused with Space.

Progress and animation positions may vary between runs. Review every PNG for
readability and personal information before committing it. The samples contain
no usernames, personal directory names, home paths or real user file listings.
If a graphical display or tools are unavailable, run the examples manually in
a 100×30 terminal and capture the terminal window with the platform screenshot
utility. Do not replace missing captures with fabricated output.

## Phase 3 captures

The existing five captures are preserved. These additional captures are also
actual xterm windows, with the same font, palette and geometry:

- `terminal-capabilities.png`: statuses and evidence after explicit queries.
- `terrabox-animation.gif`: 60 successive captures of the actual running cell animation, looping with measured capture timing.
- `terrabox-animation.png`: retained still of the animation paused with Space.

```sh
cargo build --workspace --examples --all-features
python3 scripts/capture-examples.py --phase3
```

GIF encoding additionally requires Pillow (`PIL`). The capture script does not
interpolate frames or generate terminal content.

## Graphics captures

No Kitty-capable emulator was available in this environment. Consequently
`kitty-image.png` and `terratui-image-viewer.png` are deliberately absent.
The xterm captures do not claim to demonstrate image rendering.

On a compatible machine, open a direct Kitty/Ghostty/WezTerm session at 100×30
cells using a readable monospace font. Build the examples with `--all-features`.
Run `target/debug/examples/terminal-capabilities` and check a successful runtime
Kitty static query. Then run `target/debug/examples/kitty-image`; capture only
the terminal client area after the procedural image actually renders. Repeat
with `target/debug/examples/terratui-image-viewer` and its selected image visible.
Use the OS screenshot utility (or ImageMagick window capture on X11); save the
client areas under the suggested filenames only after visual verification.
Follow the [manual checklist](../../docs/terminal-compatibility.md) first.
Check alpha, cropping, text readability and absence of personal information;
record emulator/version and dimensions alongside new captures. Do not substitute
an image composited over terminal text or a fallback-only screenshot for a
rendered graphics capture.
