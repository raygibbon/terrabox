# Third-party notices

TerraBox is distributed under its accompanying MIT `LICENSE`. The current crate
does not vendor third-party terminal capability tables or a terminal database.
Its small output fallback implements documented terminal protocols. Test
fixtures are synthetic TerraBox fixtures, not copies of terminal descriptions.
The repository's `docs/terminal-capability-audit.md` records the provenance review
and historical changes; repository history has not been rewritten.

## Dependencies

Current locked dependencies retain their own copyright notices and licences:

| Package | Version | Declared licence |
|---|---|---|
| libc | 0.2.189 | MIT OR Apache-2.0 |
| unicode-segmentation | 1.13.3 | MIT OR Apache-2.0 |
| unicode-width | 0.2.2 | MIT OR Apache-2.0 |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 |
| windows-link | 0.2.1 | MIT OR Apache-2.0 |

Cargo resolves these packages separately. This inventory does not replace the
notices and licence files supplied with each dependency. Preserve those notices
when distributing their code or binaries as required by the selected licence.

Unix terminfo entries are read from the user's installed database, not bundled
with TerraBox. Redistributors who bundle a database must review its licensing
separately. No ncurses linking or runtime utility is required by the library.
