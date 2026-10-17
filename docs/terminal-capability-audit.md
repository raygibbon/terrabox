# Independent terminal capability audit — 17 October 2026

## Scope and starting state

The working tree was clean at task entry, at `a4a890a` (`added screenshots`).
The repository was audited in place. No Git history or remotes were rewritten;
Navi8or and TDX were not modified. The earlier Phase 2 work was already committed.

Inventoried and searched the complete tracked tree for attribution, copied-table
and generator references. Inspected capability-related source/test modules,
fixtures, examples, fuzz harnesses, build/validation scripts, manifests,
lockfiles, licence files, README files and docs. Other modules were assessed
against the available project provenance; this was not a forensic comparison
of every line against all possible upstream projects. Backends are in `crates/terrabox/src/platform/`, not a
`backend/` directory; examples live under each crate, not a root examples folder.
Ignored `target/` and `fuzz/target/` contain local build caches and can include
older compiled material; these are not source or package distribution contents.
Historical commits, including original attribution, remain available unchanged.

## Before-change findings

| Material | Finding and action |
|---|---|
| `terminfo/builtins.rs` | Six 38-element byte-string arrays explicitly attributed to termbox2. Removed in full. |
| `Capabilities::builtin` | Family selection and positional mapping consumed that array organisation. Removed, rather than translating or rearranging it. |
| Root/crate third-party notices | Identified the tables as adapted material and retained upstream MIT notices for Adam Saponara and nsf. Kept until the underlying material was replaced and reviewed. |
| `terminfo/mod.rs` reader and `expand.rs` evaluator | Existing TerraBox Rust code; original project notices describe parser/backend/API code as project-developed. Retained, with bounded loading, stricter malformed-entry checks, and early basic-rendering validation. |
| Binary ABI offsets | Standard terminfo identifiers, cross-checked against installed ncurses `term.h` and a fresh `tic` compilation. Not terminal-specific sequence data. Key lookup is now explicit Rust logic, not the old array mapping. |
| Two `.hex` fixtures | Tiny clear/CUP test entries, with no copied-table generator found. Replaced by newly authored binary builders and a synthetic textual fixture to remove uncertainty about their construction. |
| Table-specific tests | Former family/table assertions removed. Rendering tests no longer assert the old fallback reset prefix. New tests derive expectations from protocols or synthetic capability markers. |
| Input parser and other sequence tests | Generic CSI/SS3, Linux VC, registered keys, paste/mouse and recovery logic. Individual protocol bytes also occur in the old tables, but are independently specified behavior. No copied upstream algorithm or generated table was identified. Retained, with fresh protocol and database coverage. |
| Native backends, renderer, cells, examples, TerraTUI | No additional attributed or recognisably translated termbox2 source found. The notices and available project history identify these implementations as project code. No unrelated framework refactor performed. |
| Docs | Two earlier architecture-review references describe the historical state. Preserved and marked historical. Current README/engine documentation updated. |
| Generated/vendored files | No tracked generator or duplicated capability tables found. Cargo dependencies are separately licensed; none is newly added by this task. |

The baseline notices already distinguished adapted sequence data from the Rust
implementation. The initial commit `e76c792` contains that same distinction.
This is evidence for the technical scope of the audit, not proof of every
contributor's authorship. No blanket statement that the repository has always
been independently developed is made.

## Replacement design and primary references

The replacement was authored from the following behavior/format references,
not by copying implementation code or distributing a terminal database:

- [ECMA-48](https://ecma-international.org/publications-and-standards/standards/ecma-48/): CUP coordinates, ED clearing and SGR attributes.
- [Xterm control sequences](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html): normal/application keyboard encodings, DEC cursor visibility, screen buffer modes and modifiers.
- [ncurses term(5)](https://invisible-island.net/ncurses/man/term.5.html) and [terminfo(5)](https://invisible-island.net/ncurses/man/terminfo.5.html): binary layout, standard capability semantics, lookup conventions and cancellation.
- [ncurses term_variables(3x)](https://invisible-island.net/ncurses/man/term_variables.3x.html): relationship of capability identifiers to the standard ABI.
- [Linux console_codes(4)](https://man7.org/linux/man-pages/man4/console_codes.4.html): Linux cursor visibility and supported VT/SGR operations. Installed `infocmp -1 linux` independently confirmed its Escape+HT `kcbt` encoding.
- [Microsoft console VT documentation](https://learn.microsoft.com/en-us/windows/console/console-virtual-terminal-sequences): Windows VT output, screen buffers and cursor behavior; input remains native console records.
- [RXVT technical reference, upstream documentation mirrored in Debian Sources](https://sources.debian.org/data/main/r/rxvt-unicode/9.30-2/doc/rxvt.7.pod): older mode 47, newer alternate screen modes and cursor save/restore.

The old implementation was inspected during the audit. This is **not** a
clean-room process. Independence here means the new fallback's structure and
operations were authored against specifications and actual requirements,
without retaining or mechanically transforming the old tables. Protocol byte
values necessarily overlap other conforming terminal implementations.

### Unix resolution

1. Require a nonempty `TERM`. Reject traversal, control characters, non-ASCII
   names and names longer than 128 bytes before reading any entry.
2. Search `TERMINFO`, `$HOME/.terminfo`, `TERMINFO_DIRS`, Android's prefix when
   applicable, and standard system directories. Both character and hexadecimal
   subdirectory layouts are supported, including the layout used on macOS.
3. Parse installed compiled entries with 16-bit or 32-bit numeric sections.
   Missing/cancelled fields remain absent: a valid entry is never overlaid with
   guessed family strings. Loads are bounded to 1 MiB, including files that grow
   while being read. Bad offsets/truncations are rejected.
4. Before touching terminal modes, check that a present entry supplies usable
   cursor addressing and clearing. A valid but insufficient entry fails rather
   than silently acquiring xterm operations. Missing/malformed/unreadable entries
   can fall back only for the documented compatibility allowlist below.

Database strings supply `cup`, `clear`, `smcup`/`rmcup`, `cnorm`/`civis`, `sgr0`,
`smkx`/`rmkx`, seven consumed attributes, and F1–F12/navigation key registrations.
Standard CSI/SS3 input remains available through the independent parser.

### Small fallback and Windows

`terminfo/fallback.rs` builds output operations by purpose. There are no
terminal-family sequence arrays, copied datasets, 38-slot layout, title-stack
operations or fallback keypad-mode changes.

| Context without usable database | Fallback policy |
|---|---|
| xterm/screen/tmux names and hyphenated variants; kitty/ghostty | CUP, home + ED, SGR, cursor visibility and mode 1049 alternate buffer |
| rxvt-unicode variants | Same small VT output policy |
| Older rxvt variants and Eterm | Mode 47 buffer switch, explicit clearing, and DEC cursor save/restore |
| Linux VC (`linux`) | Main screen, conservative attributes, and one Escape+HT BackTab registration; no alternate-buffer request |
| Unknown names and `dumb` | Unsupported error unless an installed entry supplies usable basics |
| Native Windows | Documented VT output policy; no Unix database or TERM required; existing native key/mouse/lifecycle code remains |

The allowlist is a compatibility contract, not terminal feature detection.
`TERM=xterm` does not prove RGB, graphics, native animation, extended keyboard
protocols, or every xterm extension. No such feature is newly enabled here.
Mouse collection remains explicit. The existing backend's paste/autowrap and
renderer colour assumptions remain VT-oriented; this work does not claim
arbitrary non-VT hardware support or negotiated extension detection. Database
padding delays, named ncurses extensions and hashed single-file databases are
outside the existing parser subset. Optional graphic protocols remain planned.

Compared with old fallback output, title-stack manipulation, forced cursor
blink changes and unconditional keypad mode changes are deliberately omitted.
Normal and application cursor/function keys are both parsed. The Linux VC
BackTab ambiguity is handled through its one small registration. No previously
supported family has been removed. Real behavior on every named emulator or a
physical Linux VC is not claimed from byte-level tests alone.

## Licensing conclusion and notice changes

The old tables, table-consuming positional mapping, table-specific tests and
opaque fixtures have been removed/replaced. The current source review found no
remaining copied/adapted termbox2 material requiring its notices. The matching
text in the two active third-party notice files was therefore removed **after**
replacement, and those files now record the remaining Cargo dependency licences.
Root and crate MIT licences and all unrelated copyright obligations are intact.

This is a conclusion about identified distributed material, not a claim that
renaming or rewriting identifiers extinguishes obligations. Historical credit
to termbox2, Adam Saponara and nsf remains here and in Git history. The
[MIT licence](https://opensource.org/license/mit) continues to require its
notice for copies/substantial portions of covered material; old revisions
containing the adapted tables still carry that requirement. Git history and
old releases have not been relicensed.

Remaining dependencies are `libc`, `unicode-segmentation`, `unicode-width`,
`windows-sys`, and transitive `windows-link`, all declared MIT OR Apache-2.0 in
the locked metadata. No dependency was added or upgraded. No ncurses library or
database is bundled. A separate distributor bundling dependencies/databases must
preserve their respective notices. The active dependency inventories do not
replace those packages' licence files.

Unresolved legal/provenance limits: this audit cannot independently certify
all historical authorship or provide a binding legal opinion. No additional
specific unresolved copied component was identified. A release counsel review
may still be appropriate before relying on removal of the historical notice.
If later evidence identifies other adapted material, restore its applicable
attribution or replace that material before distribution.

## Validation and distribution inspection

All checks below were executed on the final capability implementation. There
were no dependency additions/upgrades or TerraTUI source changes.

| Check | Result |
|---|---|
| `cargo fmt --all --check` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `cargo test --workspace` | Pass: 82 tests plus 1 doctest |
| `cargo build --workspace` | Pass on Linux |
| `cargo build --workspace --examples` | Pass |
| `cargo clippy --workspace --all-targets --target x86_64-pc-windows-gnu -- -D warnings` | Pass |
| `cargo build --workspace --all-targets --target x86_64-pc-windows-gnu` | Pass: library, example and test binaries |
| `cargo check --manifest-path fuzz/Cargo.toml --all-targets` | Pass |
| `cargo doc --workspace --no-deps` | Pass |
| `python3 scripts/smoke-examples.py` | Pass for all four applications with installed xterm capabilities |
| `TERRABOX_SMOKE_TERM=xterm-terrabox-fallback python3 scripts/smoke-examples.py` | Pass for all four applications using the new fallback |
| `cargo package -p terrabox --offline` in a clean temporary source snapshot | Pass, including Cargo's archive verification |
| `cargo test --offline --manifest-path <packaged-crate>/Cargo.toml --lib` | Pass: all 24 packaged library tests, including the actual `tic` check |
| `cargo package -p terratui --list --allow-dirty --offline` | Inspected unchanged framework package inventory |
| `git diff --check` | Pass |

The clean source snapshot copies only current source files, excludes `.git` and
build caches, and is outside the working repository. Its final engine archive
contains 45 files, including the authored fallback/tests, textual fixture,
licence and dependency notices. It contains no `builtins.rs`, old `.hex`
fixtures, upstream copyright notice text, or case-insensitive termbox string.
Packaged tests compile and execute the included synthetic fixture successfully.
The framework package inventory contains its own source/docs, not an embedded
engine/table copy. No package was published. The working repository remains
uncommitted; a release still requires the project's release checklist.

New checks cover binary format widths/alignment, cancelled/missing/invalid
fields, directory layouts, database priority, bounded reads, missing/empty,
invalid UTF-8 and malformed TERM values, unknown names, unsupported cursor
programs, every consumed standard ABI slot, F1–F12, normal/application arrows,
navigation, modifiers, unknown sequences and fallback lifecycle operations.
The installed databases for xterm, xterm-256color, Linux, Screen, Screen-256color,
tmux-256color, rxvt, rxvt-256color, rxvt-unicode, Eterm and Ghostty were present
and passed loading/key-decoding checks. These validate database interpretation,
not live interaction with each emulator.

Existing PTY regressions and the example scripts validate keyboard input,
rendering, resize, suspend/resume and restoration. All four examples passed
with both database and forced-fallback resolution; the TerraTUI idle samples
were zero CPU ticks over 400 ms in both runs. Screenshot files and interfaces
are unchanged, so regeneration was unnecessary.

Windows results are cross-build results only; native final-source Windows
console execution was unavailable. No macOS Rust target or native host was
available. No claim is made of native macOS verification, real Linux virtual
console interaction, or live testing in every listed terminal emulator.

### Remaining reference classification

A case-insensitive current-tree search excluding `.git` and build caches finds
termbox2/author references only in:

- This audit: legitimate provenance findings and historical credit.
- `docs/architecture-review.md`: two descriptions of the earlier migration
  state, explicitly prefaced as historical with a link to this audit.

No such references remain in active Rust source, tests, examples, manifests,
build scripts, current README files, or active dependency notices. References
were classified rather than deleted solely to make search results empty.
The engine archive scan independently confirms the active package contents.

