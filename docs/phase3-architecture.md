# Phase 3 architecture

The starting tree was clean. Existing terminfo/provenance work and licensing
are retained. TerraBox owns terminal modes, a bounded input parser, native
blocking waits/wakeups, changed-row output, clipped surfaces and Deadline.
TerraTUI owns retained widgets, layouts, modal focus and explicitly scheduled
redraws/timers. No graphics implementation existed at task entry.

Graphics cannot be modelled as cells: images have independent terminal IDs,
upload lifetimes and persistent placements. The existing renderer also restores
the cursor after presenting rows, while widget drawing knows only local clips.
The smaller extension is a terminal-owned graphics registry and coordinated
pre-cell deletions/post-cell placements, plus widget placement requests resolved
from the same arranged regions. No second renderer or graphics backend is needed.

Runtime capability evidence must distinguish confirmed/advertised support,
explicit rejection and unknown. TERM alone is insufficient. Probes are explicit,
bounded and cached; normal rendering never queries. Unrelated events are replayed
in order. Missing replies remain unknown. Native Windows record input and
multiplexers require conservative policies rather than optimistic forwarding.

Static RGBA transmission and safe full-rectangle placement are the first target.
Native Kitty animation is separately tracked; static support does not prove
frame/playback support. Unsupported/unprobed cases use text. No decoder,
mandatory tick, async runtime or graphics dependency is introduced.
