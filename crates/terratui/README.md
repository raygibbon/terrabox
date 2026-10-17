# TerraTUI

A small retained application framework built on TerraBox. Provides application
lifecycle, on-demand rendering, timers, commands, focus, nested split layouts,
panels, a command menu bar, modal dialogs and a status bar. No async runtime.

Run the repository demo with `cargo run -p terratui --example desktop`.
F10 opens the command menu, Tab/Shift-Tab switch panels, arrows move selection,
F1 opens a modal, Enter/Escape close it, and Ctrl+Q quits.

Applications own their files, documents and business logic and can implement
the Widget and App traits. See the repository architecture and roadmap for MVP
boundaries and future widgets.
