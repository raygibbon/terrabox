# Third-party notices

TerraBox and TerraTUI are distributed under the repository's MIT licence.
The current crates do not vendor third-party terminal capability tables or a
terminal capability database. The capability replacement and provenance scope
are documented in [the audit](docs/terminal-capability-audit.md).

## Rust dependencies

The following dependency versions occur in the current lockfile. Their own
copyright notices and licence texts remain applicable to their code:

| Package | Version | Declared licence |
|---|---|---|
| libc | 0.2.189 | MIT OR Apache-2.0 |
| unicode-segmentation | 1.13.3 | MIT OR Apache-2.0 |
| unicode-width | 0.2.2 | MIT OR Apache-2.0 |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 |
| windows-link | 0.2.1 | MIT OR Apache-2.0 |

These dependencies are resolved by Cargo; their source is not copied into this
repository. Preserve their supplied notices when redistributing their code or
binaries, as required by the selected licence. This inventory does not replace
the licence files supplied with each dependency.

System terminfo files are read from the user's installation at runtime; no
ncurses implementation or database is bundled. A distributor separately
bundling a terminal database must review that database's licences.
