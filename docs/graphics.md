# Capabilities, images and frame timing

## Capability evidence

`Terminal::capabilities()` reports `Supported`, `Unsupported` or `Unknown`, each
with evidence. Installed terminfo numeric colour metadata and paired alternate
screen strings supply initial declarations. `COLORTERM` is only an environment
hint. A missing database field or absent reply is not proof of absence. Unicode
rendering depends on font coverage, shaping and terminal width conventions; it
remains unknown unless the application supplies its own assertion.

Call `probe_capabilities(Duration::from_millis(250))` explicitly during startup.
The timeout must be at most two seconds; results are cached for that Terminal.
Unix queries DECRQM mouse/paste modes, enhanced keyboard flags, primary device
attributes and XTGETTCAP `Co`/`RGB`. With `kitty-graphics` it also sends a tiny
non-storing Kitty query with a reserved ID. Static support never confirms native
animation. Sixel detection is informational; there is no Sixel renderer. Enhanced
keyboard detection does not activate that mode or add CSI-u key decoding.

Silence remains unknown (`NoResponse`). Native Windows console record input does
not carry these VT replies, so probes are skipped (`NativeBackend`). Under `TMUX`
or `STY`, all queries are skipped and image transmission is blocked, including
when an application overrides support. This is a conservative transport policy,
not a statement that every multiplexer/version lacks image support. Passthrough
wrapping, native multiplexer image negotiation and remote transport discovery
are not implemented. No terminal brand alone enables images.

Unrelated keyboard, paste, mouse and resize events received during probing are
replayed in order. Probing stops early after 4096 events or approximately 1 MiB
of queued input; remaining input stays in the parser/OS queue. Escape timing and
bounded response parsing are shared with ordinary input. Responses are limited
to 4096 bytes; oversized APC/DCS payloads are discarded through termination or
parser recovery. Applications matching `Event` exhaustively must handle the new
`Event::Response(Vec<u8>)` variant. Independent `InputParser` users opt in using
`set_response_parsing(true)`; parsing defaults remain unchanged.

`override_capability` records `ApplicationOverride`, which runtime replies do
not replace. An override is an application assertion, not detected evidence.
Use it only with knowledge of the actual connection. Detection does not change
existing explicit colour choices or enable modes implicitly.

## Static Kitty graphics

Enable `kitty-graphics` on either crate (TerraTUI forwards it to TerraBox).
Default features are empty. There is no decoder dependency. Supply straight,
unpremultiplied RGBA8 through `graphics::RgbaImage::new(width, height, pixels)`;
exact dimensions, nonzero sizes and checked arithmetic are required. Decoding
PNG/JPEG or loading files belongs to the application.

```rust,no_run
use std::time::Duration;
use terrabox::{Rect, Terminal};
use terrabox::graphics::RgbaImage;

fn main() -> std::io::Result<()> {
    let mut terminal = Terminal::new()?;
    terminal.probe_capabilities(Duration::from_millis(250))?;
    if terminal.graphics_available() && terminal.size().width >= 10
        && terminal.size().height >= 5
    {
        let pixels = RgbaImage::new(1, 1, vec![40, 150, 210, 180])?;
        let image = terminal.upload_image(&pixels)?;
        terminal.place_image(image, Rect::new(0, 0, 10, 5))?;
        terminal.present()?;
        terminal.read_event()?;
        terminal.delete_image(image)?;
        terminal.present()?;
    }
    Ok(())
}
```

Uploads use direct raw RGBA (`f=32`, `t=d`) encoded independently in Base64.
3072-byte raw chunks produce at most 4096 Base64 bytes. No temporary file,
shared memory, external process or shell command is part of transmission.
Uploads are synchronous and can block on terminal output. A successful write
means bytes were delivered to the connection, not that the emulator rendered
them. Error replies remain visible as `Event::Response`; owned images rejected
by the terminal are retired for deletion. Examples then display a text fallback.

## Ownership and presentation

`Terminal` owns uploaded resources. `ImageId` and `PlacementId` are opaque,
process-unique, non-reused handles; another Terminal cannot use them. Dropping a
handle does not delete a resource. Use `delete_image` or `clear_graphics`.
Resource allocation is bounded to 64 images (including pending deletions),
64 MiB of raw pixels and 256 placements. Reusing an image at multiple positions
does not retransmit pixels. Replacing an image keeps its ID and dirties its
placements. Failed/interrupted uploads retain cleanup tracking and attempt a
specific deletion; there is no delete-all command affecting other applications.

Uploads happen immediately. Placement, movement and deletion are staged until
`present`, whose output order is:

1. Remove obsolete placements and free retired images.
2. Present changed cell rows.
3. Position dirty images at their cell anchors.
4. Restore the application's cursor position and visibility.

Graphics state commits only after output succeeds. Images use `C=1` (no implicit
cursor movement) and `z=-1`: text is above images, images are above cell
backgrounds. Placement overlap follows protocol/emulator rules; there is no
application-configurable image z-order. Text can overlay pixels. Cell buffers
remain Unicode cells; they neither own image bytes nor erase image resources.

Rectangles must fit completely inside the terminal. Zero-sized, overflowing or
partially off-screen placements return an error. Images scale to the requested
cell rectangle; there is no pixel metric measurement, aspect-ratio preservation
or partial pixel crop. Callers choose the fit rectangle themselves. Resize and
physical clear invalidate placement handles but retain image data; re-place
without reuploading. Retained scenes recreate their placements automatically.

Before suspend, child commands, raw output and shutdown, TerraBox releases its
images. These transitions invalidate all image handles; resumed applications
must upload again. Drop performs best-effort cleanup before restoring terminal
modes. A broken output connection, abort or SIGKILL cannot guarantee cleanup.
`clear_graphics` invalidates handles even if the cleanup write fails.

For retained rendering, use `ImageSlot`, `ImageRequest` and `sync_graphics`.
Each slot identifies one image/rectangle. Reuse the same `Arc<RgbaImage>` across
frames to avoid retransmission; a new Arc requests replacement. Missing slots
release their resources. Scene synchronization does not take ownership of
images created through the manual upload API.

## TerraTUI

The optional `widgets::ImageView` submits requests through TerraBox's public
API. `Desktop::present` arranges/draws cells, resolves global widget regions,
synchronizes requests and presents the coordinated frame. Application uses this
path automatically. Custom run loops should use `Desktop::present` as well;
`Desktop::image_requests` is available for explicit integration after rendering.
Nested Groups translate and clip regions using their ordinary layout geometry.

ImageView has a border, title and explanatory text fallback. Hiding/destroying
a widget removes its scene slot. Resizing updates its placement. During any
open menu or modal, **all** desktop images are suppressed and released so they
cannot cover the overlay. Closing the overlay uploads the visible images again.
This intentionally favors predictable dialogs over partial pixel clipping or
complex image occlusion. Arbitrary overlapping floating image windows and
pixel-exact cropping are deferred.

## Animation

`FrameClock` is a caller-driven pacing helper. `poll(now)` yields elapsed frame
time and deadline lateness; `deadline()` integrates with `wait_event`. Pause
removes the deadline, resume permits an immediate frame, and nonzero intervals
set a rate limit. Late callers skip missed frames instead of spinning through
catch-up updates. Measure actual rendering time around `present` separately.
There is no thread, periodic idle tick, mandatory FPS or easing system.

Native Kitty frame uploads/playback (`a=f`/`a=a`) are **not implemented**.
The capability is separate and remains unknown. Static query success cannot
validate animation operations across emulators. The `kitty-image` C control
uses explicitly scheduled full-image replacement at 250ms intervals; it does
not claim native animation, frame composition, looping or playback controls.
The next graphics milestone is real-emulator static validation, followed by a
separate animation API and verified capability policy.

## Protocol references and provenance

The encoder and response handling were written for this repository from
published protocol behavior, without copying another graphics implementation.
No new dependencies or downloaded test assets were introduced. Example images
are procedural repository source under the existing MIT license. Existing
notices and the [capability provenance audit](terminal-capability-audit.md) are
preserved; this does not make a broader clean-room claim about historical code.

- [Kitty graphics specification](https://sw.kovidgoyal.net/kitty/graphics-protocol/): queries, chunking, RGBA, placement, deletion and separate animation operations.
- [Xterm control sequences](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html): DECRQM, DA and XTGETTCAP.
- [Kitty keyboard protocol](https://sw.kovidgoyal.net/kitty/keyboard-protocol/): query flags.
- [tmux FAQ](https://github.com/tmux/tmux/wiki/FAQ): passthrough requires explicit transport handling.
