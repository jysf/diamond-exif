# Provenance ledger

**Every ported file, algorithm and decoder in `exif-core` gets a row here,
recording where it came from and under what licence.** The row is written
before, or when, the file lands (DEC-003).

The licence a crate *declares* and the provenance its code *carries* are
different things, and `cargo deny` only sees the former. This ledger is what
makes the permissive (MIT OR Apache-2.0) claim defensible rather than merely
asserted.

## The rule

Ranked best to worst. Prefer the highest available:

1. **A published specification** — zero contamination (e.g. TIFF 6.0, the Exif
   specification, the DNG specification).
2. **A published paper** — the algorithm, not someone's code.
3. **Our own prior code** — e.g. irradiance, same author. Code copied from
   irradiance and then edited is class 3, derived from class 1 (DEC-003).
4. **Reading a permissively-licensed implementation** — allowed, must be
   recorded, and the source licence's terms must be honoured.
5. **Reading a copyleft implementation** — **not permitted.** If a behaviour is
   only available this way, it does not ship in the default build; it needs its
   own decision.

"I read it years ago and reimplemented from memory" is class 5, not class 3.
Record it honestly.

### ExifTool

ExifTool (Perl, Artistic/GPL) may be RUN as an oracle but never READ to resolve a format question: running a tool imposes nothing on this crate, while reading copyleft source is a class-5 violation. It is never linked and never vendored.

## The ledger

| Module / algorithm | Source | Source licence | Provenance class | Notes |
|---|---|---|---|---|
| `src/error.rs` (`Error`, `Display`) | irradiance `src/lib.rs` `Error` enum + `Display`, HEAD `b3725d44e7dd` (file last changed `bb4aab1`, unchanged since the brief's `2b6b07a`) | MIT OR Apache-2.0, own code | 3 (derived from class 1: TIFF 6.0 §2) | Copy-then-edit: 11 TIFF-level variants kept; `MalformedRationalValue`, `RationalValueOutOfRange` and the RAW/develop variants dropped (no producer). |
| `src/reader.rs` (`ByteOrder`, `read_u16`, `read_u32`, `offset_to_usize`) | irradiance `src/ifd.rs:262-322`, HEAD `b3725d44e7dd` (file last changed `cc67ad6`) | MIT OR Apache-2.0, own code | 3 (derived from class 1: TIFF 6.0 §2) | Copy-then-edit: methods made `pub(crate)`; `i32` kept for `SRATIONAL`. |
| `src/tiff/mod.rs` (`Container`, `Entry`, `Ifd`, walk, `payload`, limits) | irradiance `src/ifd.rs:58-81, 339-472, 794-1015, 1196-1214`, HEAD `b3725d44e7dd` | MIT OR Apache-2.0, own code | 3 (derived from class 1: TIFF 6.0 §2) | Copy-then-edit: DNG removed; `payload` returns `&'a [u8]` (DEC-002), with `Entry` gaining the value field's input offset so inline payloads borrow from the input too. |
| `src/tiff/types.rs` (`type_size`, `TYPE_*`, `uints`, rational readers) | irradiance `src/ifd.rs:186-215, 1025-1193`, HEAD `b3725d44e7dd` | MIT OR Apache-2.0, own code | 3 (derived from class 1: TIFF 6.0 §2) | Copy-then-edit: DNG per-tag `RATIONAL` gate and integer-division rule deleted; `rationals` split into verbatim `urationals`/`srationals`. |
| `src/tiff/tags.rs` (baseline-TIFF tag consts) | irradiance `src/ifd.rs:93-115`, HEAD `b3725d44e7dd` | MIT OR Apache-2.0, own code | 3 (derived from class 1: TIFF 6.0 §8) | Copy-then-edit: DNG/CFA/colour tags dropped; `XResolution` added (a published-spec constant, class 1). |
| `src/tiff/mod.rs` `#[cfg(test)] mod tests` (`Build` byte-builder and the hostile-input tests) | irradiance `src/ifd.rs:1728-2410, 2969-3032`, HEAD `b3725d44e7dd` | MIT OR Apache-2.0, own code | 3 (derived from class 1: TIFF 6.0 §2) | Copy-then-edit: DNG tags and `sensor()` swapped for baseline entries; four ported-with-edit tests and two new tests. |
| `fuzz/fuzz_targets/tiff_ifd.rs`, `examples/fuzz_seeds.rs` and `fuzz/seeds/tiff_ifd/*` | irradiance `fuzz/fuzz_targets/ifd.rs` and its fuzz-seed writer, HEAD `b3725d44e7dd` | MIT OR Apache-2.0, own code | 3 (derived from class 1: TIFF 6.0 §2 for the seed shapes) | Copy-then-edit: DNG/sensor calls dropped, `srationals` and `scalar`/`values` added; seeds rebuilt on a new std-only byte builder and written deterministically. |
