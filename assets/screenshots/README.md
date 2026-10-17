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
