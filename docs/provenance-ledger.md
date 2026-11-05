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
| `src/reader.rs` (`ByteOrder`, `read_u16`, `read_u32`, `offset_to_usize`) | irradiance `src/ifd.rs:262-322`, HEAD `b3725d44e7dd` (file last changed `cc67ad6`) | MIT OR Apache-2.0, own code | 3 (derived from class 1: TIFF 6.0 §2) | Copy-then-edit: methods made `pub(crate)`; `i32` kept for `SRATIONAL`. SPEC-007 adds `i16`, `f32` and `f64` reads (net-new, from the TIFF 6.0 type definitions; nothing ported). |
| `src/tiff/mod.rs` (`Container`, `Entry`, `Ifd`, walk, `payload`, limits) | irradiance `src/ifd.rs:58-81, 339-472, 794-1015, 1196-1214`, HEAD `b3725d44e7dd` | MIT OR Apache-2.0, own code | 3 (derived from class 1: TIFF 6.0 §2) | Copy-then-edit: DNG removed; `payload` returns `&'a [u8]` (DEC-002), with `Entry` gaining the value field's input offset so inline payloads borrow from the input too. Walk reworked for DEC-001 (SPEC-003); policy is net-new, not ported. Pointer-tag walk (Exif/GPS/Interop, `IfdKind`, `SubIFDs` cap) net-new, SPEC-004/DEC-005. |
| `src/tiff/types.rs` (`type_size`, `TYPE_*`, `uints`, rational readers) | irradiance `src/ifd.rs:186-215, 1025-1193`, HEAD `b3725d44e7dd` | MIT OR Apache-2.0, own code | 3 (derived from class 1: TIFF 6.0 §2) | Copy-then-edit: DNG per-tag `RATIONAL` gate and integer-division rule deleted; `rationals` split into verbatim `urationals`/`srationals`. SPEC-007 adds `ascii`/`sints`/`floats`, the `Ascii` type and the remaining `TYPE_*` constants: net-new, nothing ported (DEC-003). |
| `src/tiff/tags.rs` (baseline-TIFF tag consts) | irradiance `src/ifd.rs:93-115`, HEAD `b3725d44e7dd` | MIT OR Apache-2.0, own code | 3 (derived from class 1: TIFF 6.0 §8) | Copy-then-edit: DNG/CFA/colour tags dropped; `XResolution` added (a published-spec constant, class 1). |
| `src/tiff/mod.rs` `#[cfg(test)] mod tests` (`Build` byte-builder and the hostile-input tests) | irradiance `src/ifd.rs:1728-2410, 2969-3032`, HEAD `b3725d44e7dd` | MIT OR Apache-2.0, own code | 3 (derived from class 1: TIFF 6.0 §2) | Copy-then-edit: DNG tags and `sensor()` swapped for baseline entries; four ported-with-edit tests and two new tests. |
| `fuzz/fuzz_targets/tiff_ifd.rs`, `examples/fuzz_seeds.rs` and `fuzz/seeds/tiff_ifd/*` | irradiance `fuzz/fuzz_targets/ifd.rs` and its fuzz-seed writer, HEAD `b3725d44e7dd` | MIT OR Apache-2.0, own code | 3 (derived from class 1: TIFF 6.0 §2 for the seed shapes) | Copy-then-edit: DNG/sensor calls dropped, `srationals` and `scalar`/`values` added; seeds rebuilt on a new std-only byte builder and written deterministically. |
| `tests/support/tools.rs` (`ToolError`, `run_tool`, `exiftool_available`, `run_exiftool`, `parse_fields`) | irradiance `tests/support/tools.rs`, HEAD `b3725d44e7dd` | MIT OR Apache-2.0, own code | 3 (derived from class 1: the TIFF 6.0 layout the oracle checks) | Copy-then-edit, ExifTool RUN as an oracle and never read. Dropped: the dnglab wrapper and JSON scraping, the DNG `ToolReading`/`ToolValue` layer. Edited: `parse_fields` yields `Option<String>` per selector (no integer parse), a field-count mismatch or empty output is `Parse`, `exiftool_version`, `run_exiftool_raw`, record read/write helpers and a temp-file helper added. `tests/support/oracle.rs` (normalisation table, comparator, `KNOWN_DIVERGENCES`) and `tests/support/oracle_tiff.rs` (hand-built tier-A fixtures from the TIFF 6.0 / Exif 2.32 layout, own independent byte builder) are net-new, nothing ported. |
| `tests/metadata_oracle.rs` (replay-and-perturb red-proof) | irradiance `tests/metadata_oracle.rs:458-545`, HEAD `b3725d44e7dd` | MIT OR Apache-2.0, own code | 3 (derived from class 1: the TIFF 6.0 layout the oracle checks) | Copy-then-edit: the `oracle_is_clean_on_an_unmodified_reading` / `oracle_names_the_one_field_that_was_perturbed` shape kept, re-pointed at `read_ours`/`diff` and extended to one perturbation per `Norm`, a missing and an extra field, a live mutated-payload demo, the seed sweep and the loud-skip tests (net-new). ExifTool RUN as an oracle, never read. |
| `tests/support/probe.rs` (`TempDir`, `copy_dir_recursive`, `inject_replace`, `stage_probe_crate`, `build_probe`/`run_probe`/`build_and_run_probe`) | irradiance `tests/plane_oracle.rs:626-805` (`TempDir`, `copy_dir_recursive`, `inject_*_fault`, `stage_probe_crate`, `build_and_run_probe`) and its control test (~1044), HEAD at the time of SPEC-008 | MIT OR Apache-2.0, own code | 3 (derived from class 1: the mutated-copy method of irradiance DEC-009; nothing format-specific) | Copy-then-edit (SPEC-008): DNG/plane specifics and the MD5 probe dropped; the injector is a closure over the staged `src/`, the needle guard is one shared exactly-once function that also requires the `RED-PROOF INJECTION` marker, the staged manifest is generated (no dependencies), the probe is a debug `--offline` build in its own target dir run with the `CARGO` that runs the tests, and the probe prints `oracle::read_ours` readings. The three copies irradiance made are lifted into this one. |
| `tests/support/mutate.rs` and `tests/mutation_oracle.rs` (byte-mutation helpers; six library-fault and three byte-mutation red-proofs) | none | MIT OR Apache-2.0, own code | net-new (SPEC-008) | Written from the TIFF 6.0 IFD layout; `locate_entry` is deliberately independent of `exif-core`. The run-time limit added to `run_tool` in `tests/support/tools.rs` (`run_tool_with_limit`, `ToolError::Timeout`) is likewise net-new. |

The JPEG segment walker (`src/jpeg/`, SPEC-009, DEC-006) and its fuzz target `fuzz/fuzz_targets/jpeg_segments.rs` with the 19 seeds in `fuzz/seeds/jpeg_segments/` are net-new, written from ITU-T T.81 Annex B, JFIF and the Adobe XMP specification (provenance class 1); nothing is ported, so there is no ledger row.
