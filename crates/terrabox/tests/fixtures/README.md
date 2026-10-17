# Capability test fixture provenance

`terra-capabilities.ti` was authored for TerraBox's independent capability audit.
Its output strings and key values are synthetic markers. `tic` compiles this
file in a temporary directory during an optional integration test, verifying
TerraBox's standard-string ABI indexes against an actual terminfo compiler.
Neither the compiled database nor host terminal descriptions are bundled.

The old two hexadecimal fixtures have been replaced by independently authored
binary builders in `src/terminfo/tests.rs`, following ncurses `term(5)`'s file
format. Tests cover both numeric widths, alignment, cancellation and corruption.
Protocol tests separately use documented CUP, ED, SGR, DEC screen/cursor modes,
and CSI/SS3 keyboard behavior. No fixture is generated from the former fallback
tables or upstream source code.
