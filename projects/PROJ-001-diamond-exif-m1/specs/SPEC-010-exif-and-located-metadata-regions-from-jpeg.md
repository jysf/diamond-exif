---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-010
  type: story                      # epic | story | task | bug | chore
  cycle: verify  # frame | design | build | verify | ship
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
  decisions: [DEC-001, DEC-002, DEC-006]
  constraints: [panic-free-library, forbid-unsafe-code, fuzz-target-per-surface, test-before-implementation, no-new-top-level-deps-without-decision]
  related_specs: [SPEC-009, SPEC-011]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: [SPEC-009]                     # e.g. [SPEC-002]

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-002's <capability>". Optional; null is acceptable.
value_link: "STAGE-002: the Exif inside a JPEG is read by the tolerant TIFF engine and the other metadata segments become findable, with each layer's problems kept distinct"

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
  tokens_estimate: 350000
  sessions:
    - cycle: build
      agent: claude-sonnet-5-5
      interface: other
      tokens_total: 285743
      estimated_usd: 1.89
      duration_minutes: 19
      recorded_at: 2026-11-05
      notes: "tokens transcribed by the orchestrator from the sub-agent metered result"
  totals:
    tokens_total: 285743
    estimated_usd: 1.89
    session_count: 1
---

# SPEC-010: Exif and located metadata regions from JPEG

## Context

SPEC-009 walks the segments and classifies APPn by signature. This spec makes that useful: the Exif block reaches the TIFF engine so everything STAGE-001 built applies to JPEG unchanged, and the other metadata segments are **located and measured, not decoded** so the audit and strip stages can find them. Read **DEC-006** first, including its new "Clarifications from the SPEC-010 design" section (normative): the Exif hand-off rule, the first-Exif-is-primary policy with a bounded block count, the located-region list, no C2PA claim yet, and the layered issue view.

Two follow-ups from the SPEC-009 verify are folded in as a small walker change (this is the one place `src/jpeg/` existing behaviour changes): a bad segment length now skips the marker **and** its two length bytes (four bytes), so one fault is one issue (DEC-006 reworded); and rule 2.2's wording about a fill run reaching the end of input is corrected in the module docs (behaviour unchanged: `Truncated` at the last `FF`).

## Goal

A one-call public entry point, `jpeg::read(&[u8]) -> Result<JpegMetadata, Error>`, returning the segment walk, one `ExifBlock` per parsed Exif segment (each holding a tolerant TIFF `Container` or the fatal TIFF error), the located non-Exif regions with their small headers read, and a layered issue view. Container independence is a tested property: the same TIFF blob read bare and inside a JPEG APP1 gives identical IFDs and values.

## Inputs

- **Read first:** `decisions/DEC-006-jpeg-segment-walk-and-issue-channel.md`, `decisions/DEC-001`, `DEC-002`; `src/jpeg/mod.rs` and `src/jpeg/issue.rs` (the walker you are extending), `src/tiff/mod.rs` and `src/tiff/issue.rs` (`Container::parse`, `ParseIssue`), `src/error.rs`, `AGENTS.md`, `guidance/toolchain-brief.md`; the fuzz files `fuzz/fuzz_targets/jpeg_segments.rs`, `tests/fuzz_layout.rs`, `examples/fuzz_seeds.rs`.
- **Format sources (published, class 1):** ITU-T T.81; Adobe XMP specification part 3 (JPEG: standard and extended XMP packets; the extended segment is the namespace string, a 32-byte GUID, a 4-byte full length and a 4-byte offset, then data); the ICC specification's JPEG embedding (`ICC_PROFILE\0`, sequence number, chunk count); the Photoshop image-resource-block layout (`8BIM`, resource id, a Pascal-string name padded to even length, a 4-byte size, data padded to even length; IPTC-NAA is resource `0x0404`); the JPEG XT / JUMBF APP11 header (`JP`, 2-byte `En`, 4-byte `Z`). No ExifTool source is read. Net-new code: no ledger row.
- No dependencies. Host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`.

## Outputs

### Public API (in `exif_core::jpeg`; new file `src/jpeg/metadata.rs`, re-exported)

```rust
pub const MAX_EXIF_BLOCKS: usize = 16;

/// One parsed Exif segment: where it came from and what the TIFF engine made of it.
pub struct ExifBlock<'a> { /* private */ }
impl<'a> ExifBlock<'a> {
    pub fn segment(&self) -> usize;                       // index into Jpeg::segments()
    pub fn tiff(&self) -> &'a [u8];                       // payload after "Exif\0\0", borrowed from the input
    pub fn container(&self) -> Result<&Container<'a>, &Error>;   // Ok = tolerant parse; Err = fatal TIFF error (bad header / IFD0)
}

/// A located, not decoded, non-Exif region. Borrowed; never interpreted.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Region<'a> {
    Xmp         { segment: usize, packet: &'a [u8] },                      // after the 29-byte signature
    ExtendedXmp { segment: usize, guid: &'a [u8], full_len: u32, offset: u32, data: &'a [u8] },
    Icc         { segment: usize, seq: u8, count: u8, data: &'a [u8] },
    Iptc        { segment: usize, data: &'a [u8] },                        // the 0x0404 resource data
    Jumbf       { segment: usize, en: u16, z: u32, data: &'a [u8] },
}
impl<'a> Region<'a> { pub fn segment(&self) -> usize; pub fn data(&self) -> &'a [u8]; }   // data(): packet / data

/// An issue with its layer named. Layers are never merged into one flat type (DEC-006).
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum AnyIssue<'a> {
    Jpeg(&'a JpegIssue),
    Exif { block: usize, issue: &'a ParseIssue },        // `block` indexes exif_blocks()
}

pub struct JpegMetadata<'a> { /* private */ }
impl<'a> JpegMetadata<'a> {
    pub fn jpeg(&self) -> &Jpeg<'a>;
    pub fn exif_blocks(&self) -> &[ExifBlock<'a>];
    pub fn exif(&self) -> Option<&Container<'a>>;            // the first block whose container is Ok (the primary Exif)
    pub fn regions(&self) -> &[Region<'a>];
    pub fn jpeg_issues(&self) -> &[JpegIssue];               // walker issues, then region/Exif-policy issues, in order
    pub fn issues(&self) -> impl Iterator<Item = AnyIssue<'_>>;   // JPEG-layer first, then each Exif block's own, in block order
    pub fn is_clean(&self) -> bool;                          // jpeg_issues empty AND every block Ok and clean
}
pub fn read(data: &[u8]) -> Result<JpegMetadata<'_>, Error>;   // Jpeg::parse + extraction; Err only as for Jpeg::parse
```

`Error` gains three variants (additive): `ExtraExif { at: usize }` (an Exif segment beyond the first; `at` is its segment offset), `RegionTooShort { what: &'static str, at: usize, need: usize, have: usize }` (a located region's small header or a resource does not fit), `IccChunkMismatch { at: usize }`. Extend the exhaustive-match guard; each gets a lower-case `Display` naming its fields.

### Extraction rules (normative)

Run after the walk, in segment order, using only the `Jpeg` result:

1. **Exif.** For each segment with `kind == Exif`, in file order: the first is primary. The segment with index (in Exif order) 1 or more records `ExtraExif { at: segment.offset() }` as a JPEG-layer issue; only the first `MAX_EXIF_BLOCKS` Exif segments get an `ExifBlock` (later ones still get the `ExtraExif` issue, no block). `tiff = payload[6..]` (never panics: payload is at least the 6 signature bytes by classification); `container = Container::parse(tiff)`. An Exif segment after the scan (`after_scan`) is still read (policy note: ExifTool's behaviour is measured in SPEC-011).
2. **XMP.** `Xmp` for each `kind == Xmp` segment: packet = payload after the 29 signature bytes (may be empty).
3. **Extended XMP.** Payload after the 35-byte signature must hold at least 32 + 4 + 4 bytes; else `RegionTooShort { what: "extended XMP header", at: segment.offset(), need: 40, have }` and no region. Otherwise `guid` = next 32 bytes, `full_len` and `offset` = next two big-endian u32, `data` = the rest.
4. **ICC.** After the 12-byte signature at least 2 bytes (`seq`, `count`) else `RegionTooShort { what: "ICC chunk header", need: 2, .. }`. `data` = the rest. After collecting, if any chunk's `count` differs from the first's, or any `seq` is 0 or greater than `count`, or a `seq` repeats, or the number of chunks differs from `count`: exactly **one** `IccChunkMismatch { at }` where `at` is the offset of the first segment found inconsistent (the second chunk if the first is simply the odd one out; define the rule in code comments and test it).
5. **IPTC.** For each `kind == Iptc` segment, walk the Photoshop image resource blocks after the 14-byte signature: each is `8BIM` (4), id (2, big-endian), name (1 length byte + that many bytes, padded so the Pascal string occupies an even number of bytes), size (4, big-endian), data (`size` bytes, followed by one pad byte if `size` is odd). A resource with id `0x0404` yields `Iptc { data }` (the IIM records, not decoded); several yield several. A resource that does not fit in the segment: `RegionTooShort { what: "8BIM resource", .. }`, stop walking that segment. Anything that is not `8BIM` where a resource should start: `StrayBytes { at, len }` for the remainder, stop. A segment with resources but no `0x0404` yields no region and no issue. Arithmetic is `checked`/`saturating`.
6. **JUMBF.** Payload after the 2-byte `JP` signature must hold 2 + 4 bytes (`En`, `Z`); else `RegionTooShort { what: "JUMBF header", need: 6, .. }`. `data` = the rest. **No C2PA detection** (see DEC-006).
7. **Order and bounds.** Regions are in segment order (a segment yields at most one region, except IPTC which may yield several). Extraction issues are appended to `jpeg_issues()` after the walker's, from one function. They are bounded by construction (at most one per segment, plus one ICC aggregate), so the JPEG-layer total stays at most `(MAX_JPEG_ISSUES + 1) + (MAX_JPEG_SEGMENTS + 1)`; state this as a documented constant `MAX_JPEG_METADATA_ISSUES` and test it. Work is linear in the input; Exif parsing is bounded by `MAX_EXIF_BLOCKS` times the TIFF caps.
8. `read` never panics on any input; every borrowed slice lies inside the input.

### Walker change (SPEC-009 module)

- `BadSegmentLength`: skip four bytes (`p += 4`), not two. Update the module docs, DEC-006 is already reworded, and update the SPEC-009 test that pinned the double issue (`length_zero_and_one_are_bad_lengths_and_the_walk_continues`): the second `StrayBytes { at: 4, len: 2 }` disappears, everything else stays.
- Module docs: correct the rule-2.2 comment so it states that a fill run reaching the end of input reports `Truncated` at the last `FF`.

### Fuzz changes (same change)

Extend `jpeg_segments` to call `jpeg::read` on every input (it already calls `Jpeg::parse`) and assert, both directions where it is an iff:
- **Container independence:** for every `ExifBlock` whose `container()` is `Ok`, `Container::parse(block.tiff())` is also `Ok` and has the same IFD offsets/kinds/depths/entry tags and counts; and for `Err`, the bare parse returns an equal `Err`.
- **Cleanliness:** `is_clean()` **iff** `jpeg_issues()` is empty and every block is `Ok` with `is_clean()`; `issues().count()` equals `jpeg_issues().len()` plus the sum of the blocks' issue counts; every `AnyIssue::Exif.block` is a valid index.
- **Blocks:** `exif_blocks().len() <= MAX_EXIF_BLOCKS`; the number of `Exif`-kind segments minus one equals the number of `ExtraExif` issues (or zero when there are none); `exif()` is the first `Ok` block's container.
- **Regions:** every region's `data()` lies inside its segment's payload (pointer range and bounds); `Region::segment()` indexes a segment of the matching kind; the count of `Xmp` regions equals the count of `Xmp`-kind segments; `ExtendedXmp` region exists **iff** its segment's payload is at least 75 bytes (35 + 40) long, and `RegionTooShort` for it otherwise; IPTC regions only come from `Iptc` segments.
- `jpeg_issues().len() <= MAX_JPEG_METADATA_ISSUES`.

New seeds (extend `examples/fuzz_seeds.rs` into `fuzz/seeds/jpeg_segments/`; the 29 TIFF and 21 JPEG seeds stay byte-identical): `exif-in-jpeg` (valid Exif wrapping a small valid TIFF with Make and a GPS IFD), `two-exif`, `exif-bad-tiff` (APP1 `Exif\0\0` + junk: fatal TIFF error), `exif-after-scan`, `xmp-and-extended-xmp` (two extended chunks with the same GUID), `extended-xmp-short`, `icc-chunks-ok` (2 chunks), `icc-chunks-mismatch`, `iptc-8bim` (a `0x0404` resource plus a path resource), `iptc-truncated-resource`, `jumbf-header`, `jumbf-short`, `seventeen-exif`. Update `tests/fuzz_layout.rs` (seed lists, `exercise` mirrors the new invariants). `src/lib.rs` already exposes `jpeg`; re-export the new names there.

### Other files

`README.md` status, `CHANGELOG.md` (`Added`, `Changed`: bad-length skip), `AGENTS.md` §6 and `guidance/toolchain-brief.md` (the new API and seeds), `docs/provenance-ledger.md` (one sentence, net-new), `guidance/questions.yaml` (open question: C2PA detection inside JUMBF; open question: how does ExifTool treat several Exif segments, to be measured in SPEC-011).

## Acceptance Criteria

- [ ] `jpeg::read` implements rules 1-8 exactly as written; the named tests below pass.
- [ ] **Container independence** holds: a test builds one TIFF blob (with Make, an Exif IFD and a GPS IFD) and shows bare `Container::parse` and the JPEG-wrapped block give identical IFD structure and decoded values (ASCII, rational).
- [ ] Every hostile region shape is typed (short headers, truncated 8BIM resources, mismatched ICC chunks, extra and 17+ Exif segments, Exif after the scan, a fatal TIFF inside a clean JPEG): never a panic, never a partial region.
- [ ] The layered view is correct: a clean JPEG with a damaged GPS IFD inside its Exif is not clean, its only issue is `AnyIssue::Exif` carrying `Link::Tag(0x8825)`, and `jpeg_issues()` is empty.
- [ ] The walker change is minimal and the SPEC-009 suite still passes with only the one pinned test updated.
- [ ] **Shown red** (record test names; then revert): (a) Exif `tiff` slice starts at 5 or 7 instead of 6; (b) `ExtraExif` recorded for the first Exif instead of the second; (c) the block cap `>` to `>=` (or off by one); (d) ICC mismatch check removed; (e) 8BIM name padding wrong (odd/even); (f) `is_clean` ignores block cleanliness; (g) extended-XMP header length 39 instead of 40.
- [ ] Fuzz: `just fuzz-smoke jpeg_segments` 60 s clean (executions and coverage recorded); at least four new invariants shown red with planted bugs (one against the real target); iff-invariants in both directions.
- [ ] Library gates green and re-run by the orchestrator: `cargo fmt --check` (both graphs), `just lint`, `just lint-fuzz`, `just lint-no-allow`, `cargo test` on stable and 1.90.0, `cargo rustc --lib -- -F unsafe_code`, `just deny`, `just deny-fuzz`, `just validate`, `just decisions-audit`, `just cost-audit`, `just oracle-record` empty diff (never touching `two-hop-cycle`).
- [ ] `[dependencies]` and `[dev-dependencies]` empty; TIFF module, oracle code and records untouched (apart from the test-module error guard); the existing 50 seeds byte-identical; irradiance and `scripts/` untouched.

## Failing Tests

Unit tests in `src/jpeg/metadata.rs` `mod tests` (and the one updated walker test in `src/jpeg/mod.rs`), using the SPEC-009 byte-builder style plus a small literal TIFF builder. Names are normative.

**Exif hand-off**
- `exif_in_a_jpeg_reads_like_the_bare_tiff` — one blob (LE; Make "DiamondCam", an Exif IFD with `DateTimeOriginal`, a GPS IFD with `GPSLatitudeRef`) read bare and inside APP1: equal IFD offsets, kinds, depths and entry tags; `ascii` values equal; `exif()` is `Some`; clean at every layer.
- `the_tiff_slice_starts_after_the_six_byte_signature` — `block.tiff()` equals `payload[6..]` and borrows from the input (pointer range; outlives the metadata).
- `a_fatal_tiff_is_kept_on_the_block_not_promoted` — APP1 `Exif\0\0` + `NOPE`: `read` is `Ok`, one block with `container() == Err(NotTiff ..)`, `exif()` is `None`, `jpeg_issues()` empty, `is_clean()` false, `issues()` yields nothing from the Exif layer (a fatal error is not an `AnyIssue`).
- `a_signature_only_exif_is_a_truncated_tiff` — payload exactly `Exif\0\0`: block `Err(Truncated ..)`.
- `a_damaged_gps_ifd_leaves_a_clean_jpeg_layer` — Exif whose GPS pointer is past EOF: `jpeg_issues()` empty; `issues()` yields exactly one `AnyIssue::Exif { block: 0, issue }` with `issue.link() == Link::Tag(0x8825)`; `exif()` still `Some`; `is_clean()` false.
- `a_second_exif_segment_is_an_extra_exif_issue_and_still_a_block` — two Exif segments: two blocks, `exif()` is the first, `jpeg_issues()` holds one `ExtraExif { at: <second segment offset> }` after any walker issues.
- `exif_blocks_are_capped_at_sixteen_but_extras_are_all_reported` — 17 Exif segments: 16 blocks, 16 `ExtraExif` issues (segments 2 through 17), none for the first.
- `the_first_ok_block_is_primary_even_if_the_first_segment_is_fatal` — block 0 `Err`, block 1 `Ok`: `exif()` is block 1's container.
- `an_exif_segment_after_the_scan_is_still_read` — APP1 after SOS: a block, same container as before the scan.

**Located regions**
- `xmp_packet_is_the_payload_after_the_signature` — plus an empty packet case.
- `extended_xmp_header_fields_are_read` — GUID 32 bytes, full length and offset big-endian, `data` the remainder; two chunks sharing a GUID give two regions.
- `a_short_extended_xmp_header_is_region_too_short` — payload of 35 + 39 bytes: `RegionTooShort { what: "extended XMP header", need: 40, have: 39, .. }`, no region; 35 + 40 is accepted with empty data.
- `icc_chunks_report_sequence_and_count` and `consistent_icc_chunks_are_clean` — two chunks 1 of 2 and 2 of 2 in order and also in reverse order: clean.
- `icc_chunk_mismatches_are_one_issue` — differing `count`, `seq` 0, `seq` above `count`, a repeated `seq`, and `count = 3` with two chunks present: each case exactly one `IccChunkMismatch` with the offset the rule defines.
- `an_icc_chunk_without_room_for_seq_and_count_is_region_too_short`.
- `iptc_0404_resource_is_located_with_even_padding` — resources: a path resource (id 0x0408) with an odd-length Pascal name and odd size, then `0x0404` with a 5-byte body (odd, one pad byte): `Iptc.data` is exactly the 5 bytes; a second `0x0404` yields a second region.
- `a_truncated_8bim_resource_is_region_too_short_and_stops` — size field larger than remaining: `RegionTooShort { what: "8BIM resource", .. }`, regions before it are kept.
- `non_8bim_bytes_where_a_resource_should_start_are_stray_bytes`.
- `an_app13_without_a_0404_resource_yields_no_region_and_no_issue`.
- `jumbf_header_fields_are_read` and `a_short_jumbf_header_is_region_too_short` (5 bytes after `JP` fails, 6 passes).

**Bounds and API**
- `the_jpeg_layer_issue_total_never_exceeds_the_documented_constant` — pathological input (many `FF 00` pairs plus thousands of tiny short-header APP1/APP2 segments): `jpeg_issues().len() <= MAX_JPEG_METADATA_ISSUES`.
- `read_agrees_with_parse_on_fatal_input` — empty, one byte, non-JPEG give the same `Err` as `Jpeg::parse`.
- `a_clean_jpeg_with_a_clean_exif_is_clean` and `is_clean_requires_every_layer` (JPEG issue only; Exif issue only; fatal block only: each false).
- `every_error_variant_this_module_returns_displays_something` — extended guard with the three new variants.

**Walker change**
- `length_zero_and_one_are_bad_lengths_and_the_walk_continues` (updated) — now exactly one issue per bad length; the following COM segment is still recorded; add `a_bad_length_skips_the_marker_and_both_length_bytes`.

**Fuzz layout** (`tests/fuzz_layout.rs`): new seed names, `exercise` mirrors every new invariant.

## Non-Goals

- Decoding XMP, ICC, IPTC IIM records or JUMBF boxes; C2PA detection; merging extended XMP chunks or reassembling ICC profiles
- ExifTool parity and JPEG fixtures with committed records (SPEC-011), and what ExifTool does with several Exif segments (measured there)
- Writing, stripping; PNG, WebP; any pixel decoding

## Notes for the Implementer

- Keep the five lints satisfied in non-test code: `get`, `checked_*`/`saturating_*`, no indexing, no `unwrap`. A segment's payload is only guaranteed to be at least as long as its signature for the kind it was classified as; still use `get(6..)` and handle `None`.
- `ExifBlock` stores the `Container` (it borrows the input); expose `container()` as a `Result` of references so the fatal error is data, not an unwrap.
- All extraction issues go through one function that respects the cap; do not push to the issue vector from several places (SPEC-004's and SPEC-009's lesson).
- The IPTC walk is the likeliest place for an off-by-one: write the padding tests first.
- Record in the Handback: the seven mutations, the fuzz plants, the 60 s smoke executions/coverage, any rule you found ambiguous (and how you resolved it) and every deviation from the named tests. If a derived number in the spec disagrees with the rules, stop and report.
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
