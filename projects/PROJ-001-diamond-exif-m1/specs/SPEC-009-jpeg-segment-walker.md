---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-009
  type: story                      # epic | story | task | bug | chore
  cycle: build  # frame | design | build | verify | ship
  blocked: false
  priority: medium                 # critical | high | medium | low
  complexity: M                    # XS | S | M | L | XL | XXL — the EXPECTED size, set at design
                                   #   (XL/XXL almost certainly means it's a stage, not a spec)
  complexity_actual: null          # stamped at ship: what it ACTUALLY took, same scale.
                                   #   Expected-vs-actual drift is what `just calibration` reads.
  verify_verdict: null             # approved | punch-list | rejected — the OUTCOME of the verify
                                   #   cycle, stamped by `just advance-cycle` when the spec leaves
                                   #   verify (same three verdicts Prompt 4 already returns).
                                   #   Recorded in front-matter, not just prose, so "verify never
                                   #   rejects anything" stops being a hunch and becomes a number.

project:
  id: PROJ-001
  stage: STAGE-002
repo:
  id: diamond-exif

handoff:
  from_agent: claude-opus-5-5  # from .repo-context tier_map.design (DEC-005)
  to_agent: null                   # filled when HANDOFF is created (any agent — see docs/porting.md)
  created_at: null

references:
  decisions: [DEC-001, DEC-002, DEC-004, DEC-006]
  constraints: [panic-free-library, forbid-unsafe-code, fuzz-target-per-surface, test-before-implementation, no-new-top-level-deps-without-decision]
  related_specs: [SPEC-010, SPEC-011]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: []                     # e.g. [SPEC-002]

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-002's <capability>". Optional; null is acceptable.
value_link: "STAGE-002: a JPEG becomes walkable without ever panicking, and a damaged one stays readable and reportable"

# Self-reported AI cost per cycle. Each cycle (design, build, verify,
# ship) appends one entry to sessions[]. Totals are computed at ship.
# Record a REAL tokens_total for metered cycles (build/verify) — the agent
# that runs the cycle writes it from its own interface (/cost, the API
# usage object, or its tool's report). Only un-metered main-loop cycles
# (design/ship) may be null-with-note. `just cost-audit` enforces this on
# shipped specs. See AGENTS.md §4 and docs/cost-tracking.md. interface:
# claude-code | claude-ai | api | ollama | other.
cost:
  # Optional PREDICTION of the total tokens this spec will take, set at
  # design. Never a gate — its only job is to be compared with the actual
  # below (`just calibration`), so you learn whether you systematically
  # under- or over-estimate. null = didn't predict.
  tokens_estimate: 330000
  sessions: []
  totals:
    tokens_total: 0
    estimated_usd: 0
    session_count: 0
---

# SPEC-009: JPEG segment walker with tolerant issues and the jpeg_segments fuzz target

## Context

STAGE-002 starts with the container itself. A JPEG is a marker stream: SOI, then segments (a `0xFF` marker byte, a marker code, a 16-bit big-endian length that **includes its own two bytes**), metadata in APPn segments before SOS, then entropy-coded scan data to EOI. Uploads contain what the format forbids: `0xFF` fill, a missing SOS or EOI, a truncated segment, bad lengths, trailers. DEC-006 (written in this design, normative, read it first) carries DEC-001's rule one level up: a malformed segment is a per-segment issue and the walk continues where it safely can; only "not a JPEG at all" is fatal. Constraint `fuzz-target-per-surface`: the `jpeg_segments` fuzz target, its seeds and its CI smoke job land **with** this parser.

Correction to the stage outline: it listed "APP1 payloads over 65,533 bytes are an issue". A 16-bit length that includes its own bytes caps a payload at 65,533 by construction, so reading has no oversize case; the cap is a writer constraint (`ExifTooLarge`, M1c). This spec tests the boundary instead: a length of `0xFFFF` is a valid 65,533-byte payload.

## Goal

A panic-free, zero-copy walker over `&[u8]`: `Jpeg::parse` returns the ordered segment list (marker, offset, borrowed payload, kind), the borrowed scan region, the end-of-image offset and trailer, and a list of per-segment `JpegIssue`s, all bounded by constants; plus the `jpeg_segments` fuzz target with deterministic seeds and a CI smoke job. Nothing is decoded; APPn kinds are identified by signature only.

## Inputs

- **Read first:** `decisions/DEC-006-jpeg-segment-walk-and-issue-channel.md` (policy, normative), `decisions/DEC-001`, `DEC-002`, `DEC-004`; `docs/exif-core-design.md` §6 (container facts) and §8; `AGENTS.md`; `guidance/toolchain-brief.md`.
- **Mirror the TIFF module's shape:** `src/tiff/mod.rs` and `src/tiff/issue.rs` (tolerant walk, `parse_strict`, `issues`, `is_clean`, bounded issue list, `Walk` state), `src/error.rs`, the fuzz layout (`fuzz/Cargo.toml`, `fuzz/fuzz_targets/tiff_ifd.rs`, `tests/fuzz_layout.rs`, `examples/fuzz_seeds.rs`).
- **Format source:** ITU-T T.81 Annex B marker syntax, JFIF, the Adobe XMP specification (JPEG embedding). Net-new code: nothing is ported (irradiance has only a lossless-JPEG entropy decoder for DNG, not a segment walker), so no ledger row; cite the standards in module docs (provenance class 1, DEC-003).
- **Host traps:** `~/.cargo/bin/cargo +1.90.0` for library gates; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; `just lint` under the default PATH. No dependencies.

## Outputs

### Public API (new module `exif_core::jpeg`; `pub mod jpeg;` in `lib.rs`)

```rust
pub const MAX_JPEG_SEGMENTS: usize = 4096;
pub const MAX_JPEG_ISSUES: usize = 1024;

/// What a segment is, from its marker and (for APPn) its signature. Identification only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SegmentKind {
    Jfif,          // APP0 "JFIF\0"
    Exif,          // APP1 "Exif\0\0"
    Xmp,           // APP1 "http://ns.adobe.com/xap/1.0/\0"
    ExtendedXmp,   // APP1 "http://ns.adobe.com/xmp/extension/\0"
    Icc,           // APP2 "ICC_PROFILE\0"
    Iptc,          // APP13 "Photoshop 3.0\0"
    Jumbf,         // APP11 starting "JP" (JPEG XT / JUMBF header)
    App(u8),       // any other APPn, n = 0..=15
    Sof(u8),       // C0..=CF except C4, C8, CC; the marker code is carried
    Dht, Dqt, Dri, Com, Sos,
    Other(u8),     // any other marker that carries a length
}

#[derive(Debug, Clone, Copy)]
pub struct Segment<'a> { /* private */ }
impl<'a> Segment<'a> {
    pub fn marker(&self) -> u8;            // the code byte after FF
    pub fn offset(&self) -> usize;         // input offset of the segment's FF marker byte
    pub fn payload(&self) -> &'a [u8];     // bytes after the 2 length bytes, borrowed from the input (DEC-002)
    pub fn kind(&self) -> SegmentKind;
    pub fn after_scan(&self) -> bool;      // true if it appears after the first SOS marker
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct JpegIssue { /* error: Error, offset: usize */ }
impl JpegIssue { pub fn error(&self) -> &Error; pub fn offset(&self) -> usize; }   // + Display

#[derive(Debug, Clone)]
pub struct Jpeg<'a> { /* private */ }
impl<'a> Jpeg<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Jpeg<'a>, Error>;
    pub fn parse_strict(data: &'a [u8]) -> Result<Jpeg<'a>, Error>; // first issue's error, by value
    pub fn segments(&self) -> &[Segment<'a>];
    pub fn issues(&self) -> &[JpegIssue];
    pub fn is_clean(&self) -> bool;
    pub fn scan_region(&self) -> Option<&'a [u8]>;  // first SOS marker .. end of EOI (or end of input)
    pub fn eoi_offset(&self) -> Option<usize>;      // offset of the EOI marker's FF
    pub fn trailer(&self) -> &'a [u8];              // bytes after EOI; empty when no EOI
}
```

`Error` gains seven variants (additive; `#[non_exhaustive]`; each with a lower-case `Display` that names its fields; the exhaustive-match guard in the `every_error_variant...` test must be extended): `NotJpeg { mark: [u8; 2] }`, `StrayBytes { at: usize, len: usize }`, `BadSegmentLength { marker: u8, length: u16 }`, `MissingSos`, `MissingEoi`, `TooManySegments { limit: usize }`, `TooManyIssues { limit: usize }`. Truncation reuses `Error::Truncated { at, len }`.

### Walk rules (normative; DEC-006 has the rationale)

Let `data` be the input and `p` the cursor.

1. **Header.** `data.len() < 2` is `Err(Truncated { at: 0, len: 2 })`; first two bytes not `FF D8` is `Err(NotJpeg { mark })`. `p = 2`.
2. **Loop** until `p >= data.len()` or the walk ends:
   1. If `data[p] != 0xFF`: a maximal run of bytes up to the next `0xFF` (or end of input) is one issue `StrayBytes { at: p, len }`; advance past it.
   2. `data[p] == 0xFF`. If `p + 1 >= len`: issue `Truncated { at: p, len: 2 }`, end.
   3. **Fill:** while `data[p+1] == 0xFF`, advance `p` by one (consecutive `FF` are fill; legal before any marker, no issue). Then `m = data[p+1]`, and `p` is the offset of the marker's own `FF`.
   4. `m == 0x00`: `FF 00` outside entropy data: issue `StrayBytes { at: p, len: 2 }`, `p += 2`, continue.
   5. `m == 0xD9` (EOI): `eoi = p`, end. The trailer is `data[p + 2..]`.
   6. `m` is standalone (`0x01` TEM, `0xD0..=0xD7` RSTn, `0xD8` SOI): carries no segment; outside entropy data it is `StrayBytes { at: p, len: 2 }`, `p += 2`, continue.
   7. Otherwise `m` carries a length. If `p + 4 > len`: issue `Truncated { at: p, len: 4 }`, end. `L` = big-endian u16 at `p + 2`.
      - `L < 2`: issue `BadSegmentLength { marker: m, length: L }` at offset `p`, `p += 2` (rescan; the next bytes fall to rule 2.1), continue.
      - `p + 2 + L > len`: issue `Truncated { at: p, len: 2 + L }` at offset `p`; **the segment is omitted entirely**; end.
      - Else record `Segment { marker: m, offset: p, payload: data[p + 4 .. p + 2 + L], kind, after_scan }` and `p += 2 + L`. `L == 2` is legal (empty payload).
   8. **SOS (`m == 0xDA`)**: after recording its segment (first SOS sets `scan_start = p_of_that_marker`), scan entropy-coded data from the new `p`: look for the next `0xFF` whose following byte is not `0x00`, not `0xD0..=0xD7` and not `0xFF` (a following `0xFF` is fill: advance one and look again). That `FF` is the next marker: set `p` to it and continue the loop (so DHT, further SOS and EOI are walked). If none is found, `p = len` and the walk ends. `after_scan` is true for every segment recorded after the first SOS marker.
3. **End of walk** (by reaching the end of input, truncation, or EOI; **not** when halted by a cap): if no SOS was recorded, issue `MissingSos` (offset `len`); if EOI was not seen, issue `MissingEoi` (offset `len`), in that order.
4. **Caps.** Before recording a segment, if `segments.len() == MAX_JPEG_SEGMENTS`: issue `TooManySegments { limit }` and halt. Every issue goes through one `record` function: if the count of ordinary issues has reached `MAX_JPEG_ISSUES`, it records `TooManyIssues { limit }` instead and halts. Halting stops the walk and suppresses rule 3. So `segments().len() <= MAX_JPEG_SEGMENTS` and `issues().len() <= MAX_JPEG_ISSUES + 1` always.
5. **Kinds** per the enum comments; `Sof(m)` for `0xC0..=0xCF` except `0xC4` (`Dht`), `0xC8` and `0xCC` (`Other`); `Dqt = 0xDB`, `Dri = 0xDD`, `Com = 0xFE`, `Sos = 0xDA`; APPn `0xE0..=0xEF` not matching a signature is `App(n)`. An APP payload shorter than its signature is simply `App(n)`.
6. `scan_region()` is `data[scan_start ..= end]` where `end` is `eoi + 2` (EOI found) else `len`; `None` without an SOS. `eoi_offset()` and `trailer()` as stated.
7. All arithmetic is `checked_*`/`saturating_*`; no indexing, no `unwrap` (five lints). Every slice is `.get(..)`.

### Fuzz target and seeds

- `fuzz/fuzz_targets/jpeg_segments.rs` + `[[bin]] jpeg_segments` in `fuzz/Cargo.toml`: for any bytes, `Jpeg::parse`; on `Ok`, assert **both directions** where it is an iff:
  - `parse_strict` is `Ok` **iff** `is_clean()`; when `Err` it equals `issues()[0].error()` (`Error` is `PartialEq`); when `parse` is `Err`, `parse_strict` returns an equal `Err`.
  - Segments are in strictly increasing `offset` order and non-overlapping; each `offset >= 2`, `payload` lies inside the input at exactly `offset + 4 ..`, and `payload.as_ptr()` is inside `data`'s range (zero-copy).
  - `kind == Exif` **iff** `marker == 0xE1` and the payload starts with `Exif\0\0`; same iff for `Xmp`, `ExtendedXmp`, `Icc`, `Iptc`, `Jumbf`, `Jfif` against their marker and signature.
  - `scan_region().is_some()` **iff** some segment has `kind == Sos`; its first byte is `0xFF` followed by `0xDA`; it ends at `eoi_offset + 2` when `eoi_offset` is `Some`, else at the end of the input; `trailer()` is empty when `eoi_offset` is `None` and equals `data[eoi + 2..]` otherwise.
  - `segments().len() <= MAX_JPEG_SEGMENTS`; `issues().len() <= MAX_JPEG_ISSUES + 1`; a `TooManyIssues` or `TooManySegments` issue, if present, is the last issue and then neither `MissingSos` nor `MissingEoi` appears.
  - `after_scan()` is true **iff** the segment's offset is greater than the first SOS segment's offset.
  - Exercise `Display` of every issue and kind (must not panic).
- Seeds (written by extending `examples/fuzz_seeds.rs` into `fuzz/seeds/jpeg_segments/`; deterministic; the 29 existing TIFF seeds stay byte-identical): `minimal`, `soi-only`, `fill-bytes`, `app1-len-0`, `app1-len-1`, `app1-len-2`, `app1-len-ffff`, `app1-past-eof`, `exif-and-xmp`, `extended-xmp-pair`, `stray-bytes`, `ff00-stray`, `no-sos`, `sos-no-eoi`, `progressive-two-scans`, `trailer-after-eoi`, `app-after-scan`, `many-segments`, `many-ff00`.
- `tests/fuzz_layout.rs`: a `[[bin]] jpeg_segments` check, the seed list for the new directory, and `exercise` extended so the plain-test run covers the new invariants (as for `tiff_ifd`).
- `app.just`: `fuzz-smoke` takes an optional target (`just fuzz-smoke [TARGET]`, default runs every target in turn) and the doc comment names the PATH trap; `.github/workflows/ci.yml`: the `fuzz-smoke` job becomes a matrix over `tiff_ifd` and `jpeg_segments` (60 s each).

### Other files

`src/lib.rs` (`pub mod jpeg;`), `src/error.rs` (variants + `Display`), `README.md` status line, `CHANGELOG.md` (`Added`), `AGENTS.md` §6 and `guidance/toolchain-brief.md` (new target, new seeds, `just fuzz-smoke [TARGET]`), `docs/provenance-ledger.md` (no row; one sentence in the notes that the JPEG module is net-new from T.81/JFIF).

## Acceptance Criteria

- [ ] Every walk rule behaves as stated, tested by the named tests below; `parse_strict` reproduces the first issue as `Err`.
- [ ] Hostile shapes (length 0/1/2/`0xFFFF`/past EOF, fill, stray runs, `FF 00`, stray SOI, missing SOS/EOI, trailer, many segments, many stray pairs) are typed outcomes, never panics; payloads, scan region and trailer are borrowed from the input.
- [ ] Caps are exact: 4096 segments clean, the 4097th halts with `TooManySegments`; the issue cap yields exactly `MAX_JPEG_ISSUES + 1` issues ending in `TooManyIssues` for a pathological input. **Boundary mutations recorded** (each named test goes red, then revert): (a) segment cap off by one, (b) issue cap off by one, (c) fill-byte handling removed, (d) `L < 2` changed to `L < 3`, (e) a truncated segment recorded partially instead of omitted.
- [ ] Fuzz: `just fuzz-smoke jpeg_segments` 60 s clean with executions and coverage recorded; **at least three of the new invariants shown red** with planted bugs (one against the real target); iff-invariants written in both directions.
- [ ] CI: the matrix `fuzz-smoke` job exists; the whole existing CI stays green; `tests/fuzz_layout.rs` covers the new target and seeds.
- [ ] Library gates green and re-run by the orchestrator: `cargo fmt --check`, `just lint`, `just lint-fuzz`, `just lint-no-allow`, `cargo test` on stable and 1.90.0, `cargo rustc --lib -- -F unsafe_code`, `just deny`, `just deny-fuzz`, `just validate`, `just decisions-audit`, `just cost-audit`, `just oracle-record` leaving an empty diff (never touching `two-hop-cycle`).
- [ ] `[dependencies]` and `[dev-dependencies]` empty; the 29 TIFF seeds unchanged; the TIFF module, oracle code and ExifTool records untouched; irradiance and `scripts/` untouched.

## Failing Tests

Unit tests in `src/jpeg/mod.rs` `mod tests` (with the standard test-module lint allowance) using a small independent byte builder (`soi()`, `seg(marker, payload)`, `eoi()`, raw bytes). Names are normative; offsets below assume the builder lays bytes back to back from offset 0.

**Fatal and trivial**
- `empty_and_one_byte_inputs_are_truncated` — `[]` and `[0xFF]` give `Err(Truncated { at: 0, len: 2 })`.
- `a_non_jpeg_is_rejected_with_its_mark` — `89 50` gives `Err(NotJpeg { mark: [0x89, 0x50] })`.
- `soi_only_reports_missing_sos_then_missing_eoi` — `FF D8`: zero segments, issues `[MissingSos, MissingEoi]` both at offset 2, `scan_region()` `None`, `eoi_offset()` `None`, trailer empty; `parse_strict` is `Err(MissingSos)`.

**Happy path and kinds**
- `a_minimal_jpeg_walks_clean` — SOI, APP0 `JFIF\0`+5 bytes, DQT, SOF0, DHT, SOS header, scan bytes containing `FF 00` and `FF D0`, EOI. Clean; segments in order with exact `marker`/`offset`/`payload` slices; `scan_region()` equals `data[sos_offset..]`; `eoi_offset()` correct; trailer empty; `after_scan()` false for every segment (the SOS itself is not after the scan).
- `app_segments_are_classified_by_marker_and_signature` — one each of Jfif, Exif, Xmp, ExtendedXmp, Icc, Iptc, Jumbf (`JP...` in APP11), a plain `App(3)`, an APP1 with an unknown signature (`App(1)`), an APP1 shorter than its signature (`App(1)`), and `Sof(2)`, `Dht`, `Dqt`, `Dri`, `Com`, `Other(0xC8)`.
- `payloads_and_scan_borrow_from_the_input_not_the_jpeg` — slices taken while a `Jpeg` exists are used after it is dropped; pointer-range check; `payload.as_ptr()` equals `data[offset + 4..].as_ptr()`.
- `a_length_of_ffff_is_a_65533_byte_payload` and `a_length_of_two_is_a_valid_empty_segment` — boundary lengths, both clean.

**Hostile segment shapes**
- `length_zero_and_one_are_bad_lengths_and_the_walk_continues` — an APP1 with `L = 0` then a valid COM: issue `BadSegmentLength { marker: 0xE1, length: 0 }` at the APP1 offset, the COM is still recorded; same for `L = 1`.
- `a_length_past_eof_omits_the_segment_and_ends_the_walk` — APP1 `L = 0x0100` with 10 bytes left: one `Truncated { at: <offset>, len: 258 }`, **no APP1 segment recorded**, then `MissingSos`, `MissingEoi`; strict is `Err(Truncated ..)`.
- `a_marker_with_no_room_for_its_length_is_truncated` — `FF D8 FF E1 00` (length cut): issues `[Truncated { at: 2, len: 4 }, MissingSos, MissingEoi]`.
- `a_lone_trailing_ff_is_truncated` — `FF D8 FF`: issues `[Truncated { at: 2, len: 2 }, MissingSos, MissingEoi]`.
- `fill_bytes_before_a_marker_are_clean` — `FF D8 FF FF FF E0 ...`: clean; the segment's `offset` is the offset of the `FF` immediately before `E0`.
- `stray_bytes_between_segments_are_one_issue_per_run` — two garbage runs of different lengths: two `StrayBytes { at, len }` issues with exact values; segments on both sides still recorded.
- `ff00_and_a_mid_stream_soi_outside_entropy_data_are_stray` — each `StrayBytes { len: 2 }`; a stray `RSTn` and `TEM` likewise.
- `a_trailer_after_eoi_is_returned_not_an_issue` — bytes after EOI: clean; `trailer()` equals them.
- `a_second_eoi_or_data_after_eoi_is_never_walked` — a would-be segment in the trailer is not in `segments()`.

**Scans**
- `progressive_scans_walk_as_a_segment_sequence` — SOS, scan, DHT, SOS, scan, EOI: both SOS recorded, the DHT too, `scan_region()` spans from the first SOS to EOI end, clean; the DHT and second SOS have `after_scan() == true`.
- `entropy_data_hides_no_markers_but_ends_at_a_real_one` — scan containing `FF 00`, `FF D3`, `FF FF D3` (fill then RST) is skipped; `FF C4` ends it.
- `an_app_segment_after_the_scan_is_listed_with_after_scan_set` — APP1 after SOS: recorded, `kind` still `Exif`/etc., `after_scan()` true, no issue.
- `missing_eoi_after_a_scan_extends_the_region_to_the_end` — `scan_region()` ends at `data.len()`; issues `[MissingEoi]` only; `eoi_offset()` `None`.
- `a_jpeg_with_headers_but_no_sos_reports_missing_sos` — issues `[MissingSos]` (EOI present).

**Caps and pathological**
- `exactly_max_segments_is_clean` — `SOI + 4095 COM(L=2) + SOS + EOI` = 4096 segments, clean.
- `one_over_the_segment_cap_halts_with_too_many_segments` — `SOI + 4096 COM + SOS + EOI`: `segments().len() == 4096`, last issue `TooManySegments { limit: 4096 }`, no `MissingSos`/`MissingEoi`.
- `a_flood_of_ff00_pairs_is_capped_at_max_issues_plus_one` — 100,000 `FF 00` pairs after SOI: `issues().len() == MAX_JPEG_ISSUES + 1`, last is `TooManyIssues { limit: 1024 }`, no `MissingSos`/`MissingEoi`.
- `a_large_all_ff_scan_is_linear_and_terminates` — 2 MiB of `FF` after an SOS header: completes, issues `[MissingEoi]` (no hang; assert only the result).
- `walk_always_makes_progress` — a table of 30 tiny hostile inputs (e.g. `FF D8 FF 01`, `FF D8 FF E0 00 00 FF E0 00 00`) each parse in bounded steps and return.

**API and errors**
- `parse_strict_agrees_with_parse` — fatal inputs equal `Err`; clean inputs `Ok`; dirty inputs `Err(first issue's error)`.
- `every_error_variant_this_module_returns_displays_something` — extend the exhaustive-match guard with the seven new variants; `JpegIssue` and `SegmentKind` `Display`/`Debug` are non-empty.

**Fuzz layout** (`tests/fuzz_layout.rs`): the new target, its seeds (names above) and `exercise`.

## Non-Goals

- Handing APP1 to the TIFF engine, the public Exif-from-JPEG entry point, combined issue view (SPEC-010)
- Decoding XMP, ICC, IPTC or C2PA; locating sub-structures inside them (8BIM resources, ICC chunk indices); any pixel decoding
- ExifTool parity and JPEG fixtures with committed records (SPEC-011); writing or rewriting
- PNG, WebP, HEIC

## Notes for the Implementer

- Mirror `src/tiff/`: a single `Walk`-style private state with one `record` function that enforces the issue cap, so no code path can bypass it. Keep the loop iterative; there is no recursion in a JPEG walk.
- The "make progress" invariant matters: every iteration must advance `p` by at least one byte or end the walk. The fuzz target and `walk_always_makes_progress` exist to catch a violation.
- Fill-byte handling and the standalone-marker set are where off-by-ones live; write the boundary tests first.
- Keep the five lints satisfied in non-test code. `u16` big-endian reads go through the existing `reader` helpers or `from_be_bytes` over `get(..)` arrays.
- No `Container`/TIFF code is touched. Do not add dependencies.
- Record in the Handback: the five boundary mutations, the fuzz plants, the 60 s smoke executions and coverage, and any deviation from the named tests.
- Do not push or open a PR; do not advance cycles; do not modify irradiance or `scripts/`.

---

## Reflection

*Appended during **ship**. Three questions, short answers.*

1. **What would I do differently next time?**
   — <answer>

2. **Does any template, constraint, or decision need updating?**
   — <answer — if yes but not done this session, record it in
   `/guidance/signals.yaml`: `type: lesson` (with its N-count) for a recurring
   coding pattern, `type: process-debt` for tooling/process friction. A close
   then forces the decision. See `docs/signals.md`.>

3. **Is there a follow-up spec I should write now before I forget?**
   — <answer>

4. **Where was the worst defect caught?** — one word from a fixed vocabulary so
   the defect-escape distribution is greppable across specs:
   `design` | `build` | `verify` | `ship` | `escaped` (reached prod/runtime) |
   `none` (clean first try).
   — <one word>
   *(Runtime/operational defects — the escape-prone class — only exist once the
   artifact meets its real host. `escaped` here is a signal to strengthen the
   §12 behavioral pre-flight for that surface.)*

5. **What can a user do now that they couldn't before?** — one sentence,
   before → after; quote the confirming number if one exists, name the outcome
   if not. Write `none` if this spec has no user-visible outcome — that is a
   real, greppable result, not a blank. This is the line a downstream work-log's
   `impact` field is transcribed from, and both halves are already written above
   (## Context is the before, ## Goal is the after): confirm the prediction,
   don't reconstruct it from memory.
   — <answer | none>
