---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-029
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
  verify_rounds: []                # every verify outcome, oldest first, appended by
                                   #   `just advance-cycle` each time the spec leaves verify.
                                   #   verify_verdict keeps only the LAST one, so a spec sent
                                   #   back and then approved read as "approved": irradiance's
                                   #   field showed 1 pushback where its handoffs showed 5.

project:
  id: PROJ-001
  stage: STAGE-008
repo:
  id: diamond-exif

handoff:
  from_agent: claude-opus-5-5  # from .repo-context tier_map.design (DEC-005)
  to_agent: null                   # filled when HANDOFF is created (any agent — see docs/porting.md)
  created_at: null

references:
  decisions: [DEC-018, DEC-017, DEC-006, DEC-002]
  constraints: [panic-free-library, fuzz-target-per-surface]
  related_specs: []                # [SPEC-NNN]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: [SPEC-028]

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-008's <capability>". Optional; null is acceptable.
value_link: null

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
  tokens_estimate: null
  sessions: []
  totals:
    tokens_total: 0
    estimated_usd: 0
    session_count: 0
---

# SPEC-029: PNG metadata regions: `eXIf` to the TIFF engine, `iTXt` XMP, `iCCP`, text-chunk Exif located; container independence

## Context

Second spec of STAGE-008. With the PNG chunk walk in place (SPEC-028), the metadata chunks can be located, mirroring SPEC-010 for JPEG.

## Goal

Add `png::read`: Exif from `eXIf` handed to the tolerant TIFF engine, and borrowed, never-decoded regions for XMP, ICC, raw-profile text and other text chunks, including metadata chunks after `IDAT` and after `IEND`, exactly as `decisions/DEC-018-png-metadata-regions-exif-hand-off-and-text-routing.md` specifies; extend the `png_chunks` fuzz target with an independent model of the extraction.

## Inputs

- **Decisions:** DEC-018 (the contract; measured against ExifTool in the design) and DEC-017 (the walk it builds on). Follow both exactly; raise any disagreement in the handback.
- **Files to read:** `src/jpeg/metadata.rs` (the pattern: `read`, `ExifBlock`, `Region`, `AnyIssue`, caps, halt handling), `src/png/mod.rs` and `src/png/issue.rs` (SPEC-028), `src/error.rs`, `fuzz/fuzz_targets/png_chunks.rs`, `fuzz/shared/png_chunk_invariants.rs`, `fuzz/shared/jpeg_metadata_invariants.rs` (shared-model pattern), `tests/fuzz_layout.rs`, `tests/support/png_shapes.rs`, `examples/fuzz_seeds.rs`.
- **Evidence:** the ExifTool probe table is in DEC-018 Context; the 17 probe shapes are described there, build equivalents with `tests/support/png_shapes.rs` (the generator is not committed).

## Outputs

- **Created:** `src/png/metadata.rs` (`PngMetadata`, `PngExifBlock`, `PngRegion`, `AnyPngIssue`, `read`, constants), `fuzz/shared/png_metadata_invariants.rs` (independent model), `tests/png_metadata.rs`; extra seeds under `fuzz/seeds/png_chunks/` (names prefixed `meta-`).
- **Modified:** `src/png/mod.rs` (re-exports; a crate-internal way to frame chunks over the trailer without duplicating the walk logic), `src/error.rs` (`ImproperExifHeader`, `BadTextChunk`, `BadIccpChunk`, `TooManyRegions`, with Display), `src/lib.rs` docs, `fuzz/fuzz_targets/png_chunks.rs` (also calls `read` and asserts the shared model), `tests/support/png_shapes.rs`, `examples/fuzz_seeds.rs`, `tests/fuzz_layout.rs` (png seed count, names, model included), README (PNG metadata located; still not in `exif-audit`), the stage file (one line).
- **API (all `pub`, documented, panic-free):**
  - `png::read(&'a [u8]) -> Result<PngMetadata<'a>, Error>` (fatal exactly when `Png::parse` is)
  - `PngMetadata`: `png() -> &Png`, `exif_blocks() -> &[PngExifBlock]`, `exif() -> Option<&Container>` (first `Ok`), `regions() -> &[PngRegion]`, `png_issues() -> &[PngIssue]`, `issues() -> impl Iterator<Item = AnyPngIssue>`, `is_clean()`
  - `PngExifBlock`: `offset()` (chunk), `tiff_at()`, `tiff()`, `container() -> Result<&Container, &Error>`, `after_idat()`, `after_iend()`
  - `PngRegion` (`#[non_exhaustive]` enum): `Xmp { offset, compressed, data, after_idat, after_iend }`, `Icc { offset, name, method, data, .. }`, `RawProfile { offset, profile, compressed, data, .. }`, `Text { offset, keyword, compressed, data, .. }`, with `offset()`, `data()`, `after_idat()`, `after_iend()`
  - constants `MAX_PNG_EXIF_BLOCKS = 16`, `MAX_PNG_TRAILER_CHUNKS = 4096`, `MAX_PNG_REGIONS = 65_536`
- **No new dependency.**

## Acceptance Criteria

- [x] **Container independence:** each Exif block's container is `Ok` iff the bare `Container::parse` of its `tiff()` is, with identical IFD structure; a fatal error equals the bare error. `tiff_at()` plus an IFD offset indexes the same byte in the input.
- [x] `eXIf` rules (DEC-018): `Exif\0\0` prefix accepted with one `ImproperExifHeader`; empty and non-TIFF payloads leave the block `Err` with no panic; the first block is primary; a second is `ExtraExif` and still parsed; at most 16 parsed, the 17th costs only the issue.
- [x] Text routing: `iTXt` and `tEXt` `XML:com.adobe.xmp` give `Xmp` (compressed flag from the iTXt flag, `false` for tEXt); the same text under keyword `Comment` is `Text`; `Raw profile type exif` and `... xmp` in `tEXt`, `zTXt` and `iTXt` give `RawProfile` with the profile name and the right `compressed`; ordinary `tEXt`, `zTXt`, `iTXt` give `Text`. Keyword matching is case-sensitive.
- [x] `BadTextChunk`: no NUL in the first 80 bytes, empty keyword, keyword over 79 bytes, truncated `iTXt` header, `iTXt` flag other than 0 or 1; each yields the issue and no region.
- [x] `iCCP`: name, method and data located (`Icc`); empty name, no NUL, or a missing method byte is `BadIccpChunk`.
- [x] Nothing is inflated or hex-decoded; every region's `data()` is a subslice of the input (pointer-range test), and `Xmp`/`Text`/`RawProfile` data starts exactly after the chunk's header fields.
- [x] Positions: `after_idat` is true exactly for metadata chunks after the first `IDAT`; chunks after `IEND` are found through the trailer scan with `after_iend` true; a trailer that is not chunks yields nothing and no issue; the trailer scan stops at the first chunk that does not fit and never examines more than `MAX_PNG_TRAILER_CHUNKS`.
- [x] Bounds: more than `MAX_PNG_REGIONS` located chunks end with exactly one `TooManyRegions`; walker halts (DEC-017) suppress nothing that was already found and add no extraction issue after the halt.
- [x] `is_clean()` iff no PNG-layer issue and every block `Ok` and clean; `issues()` count equals PNG issues plus each block's TIFF issues.
- [x] Panic-free on every prefix and single-byte flip of each metadata seed; the committed `meta-` seeds (at least 20, named in the test) parse; pre-existing seeds byte-identical; the shared model passes under `cargo test` on stable and in `png_chunks` on nightly; CI smoke still 60 s.
- [x] `just lint`, `just test`, `just lint-fuzz`, `just validate`, `just forbid-unsafe`, `just cost-audit`, `just fuzz-smoke png_chunks` green; no `#[allow]`; no unsafe.

## Failing Tests

Written first from these descriptions; fixtures come from new builders in `tests/support/png_shapes.rs` (`exif_chunk`, `itxt`, `text`, `ztxt`, `iccp`, `raw_profile`; compressed bodies may be any bytes since nothing is inflated).

- **`tests/png_metadata.rs`**
  - `exif_block_equals_bare_tiff` / `exif_prefix_is_accepted_with_an_issue` / `exif_empty_and_garbage_keep_the_block_err` / `second_exif_is_extra_but_parsed` / `exif_blocks_are_capped_at_sixteen`.
  - `xmp_routes_for_itxt_and_legacy_text` / `same_text_under_another_keyword_is_text` / `keyword_matching_is_case_sensitive`.
  - `raw_profile_types_in_all_three_text_chunks` (exif, xmp, and an unknown name).
  - `bad_text_chunks_are_issues_without_regions` (table: each case from the acceptance list).
  - `iccp_located_and_bad_headers_are_issues`.
  - `regions_borrow_the_input_and_start_after_the_header`.
  - `after_idat_and_after_iend_flags`; `trailer_that_is_not_chunks_is_silent`; `trailer_scan_is_bounded`.
  - `region_cap_halts_once`; `walker_halt_suppresses_extraction_issues_but_keeps_found_regions`.
  - `is_clean_and_issue_counts`.
  - `every_prefix_and_byte_flip_of_each_metadata_shape_is_panic_free`.
- **`tests/fuzz_layout.rs`** (extend): `meta-` seed names pinned and equal to the builders byte for byte; pre-existing seeds unchanged; the metadata model (`include!`) passes on every png seed.
- **Red-proofs (in-file, control first):** the model must go red on mutated copies of its input: a flipped `compressed`, a region offset shifted by one, a missed trailer chunk; the control case passes first.

## Non-Goals

- Inflating zlib or decoding hex raw profiles (DEC-018: located only)
- Parsing XMP or ICC contents, or reading text chunk values as strings
- Audit categories, rendering and the "could not read" findings: SPEC-032
- ExifTool parity tests: SPEC-031
- Any change to the DEC-017 walk's behaviour on the recorded chunks

## Notes for the Implementer

- Mirror `src/jpeg/metadata.rs`: module docs with provenance (DEC-003 class 1, net-new from the PNG specification), the halt rule, bounds by construction.
- The trailer scan must reuse the chunk-framing logic of the walker (refactor to a crate-internal function taking a start offset); do not copy-paste it, and do not change what the walker records or reports (all SPEC-028 tests and the png fuzz model must pass unchanged).
- Offsets and lengths are attacker-controlled: checked arithmetic, `get` for every slice, iterative loops only.
- The model in `fuzz/shared/png_metadata_invariants.rs` must derive regions, blocks and issues independently from raw bytes (its own keyword and header parsing), not from the library, and compare in both directions.
- Seed work: add `meta-` seeds through the existing generator; update every pin; grep for the last-added seed name before and after.
- Budget: about 100 tool calls or 250k context; if over half way with the extraction unfinished, hand back with what is done.

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
