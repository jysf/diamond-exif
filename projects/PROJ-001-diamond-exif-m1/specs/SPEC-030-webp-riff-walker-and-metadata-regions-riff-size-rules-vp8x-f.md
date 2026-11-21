---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-030
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
  decisions: [DEC-019, DEC-018, DEC-017, DEC-006, DEC-002]
  constraints: [panic-free-library, fuzz-target-per-surface]
  related_specs: []                # [SPEC-NNN]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: []

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
  sessions:
    - cycle: build
      handoff: HANDOFF-058
      agent: claude-sonnet-5-5
      interface: other
      basis: agent-result
      tokens_total: 263463
      estimated_usd: 1.74
      duration_minutes: null
      recorded_at: 2026-11-21
      notes: "No token count is exposed to the subagent; the orchestrator replaces null with the harness figure. About 85 tool calls."
  totals:
    tokens_total: 263463
    estimated_usd: 1.74
    session_count: 1
---

# SPEC-030: WebP RIFF walker and metadata regions: RIFF size rules, `VP8X` flags, EXIF, XMP, ICCP, pad bytes, animation chunks passed over, `webp_riff` fuzz target (DEC for the RIFF walk)

## Context

Third spec of STAGE-008; independent of the PNG specs, so it may build in either order but never at the same time as SPEC-031. WebP is a RIFF container whose `VP8X` header declares which metadata chunks exist and whose chunks carry pad bytes when odd.

## Goal

Walk a WebP tolerantly (`webp::Webp::parse`) and extract its metadata (`webp::read`): Exif from `EXIF` chunks handed to the TIFF engine, XMP and ICC located, `VP8X` info, size and flag disagreements reported as issues, exactly as `decisions/DEC-019-webp-riff-walk-and-metadata-regions.md` specifies; ship the `webp_riff` fuzz target with an independent model and seeds.

## Inputs

- **Decisions:** DEC-019 (the contract; every rule is measured against ExifTool 13.55 on 44 shapes, listed in its Context), plus DEC-017 and DEC-018 (the PNG walk and metadata, whose structure to mirror), DEC-006, DEC-002, DEC-001. Follow DEC-019 exactly; raise any disagreement in the handback instead of choosing.
- **Files to read:** `src/png/mod.rs`, `src/png/issue.rs`, `src/png/metadata.rs` (the pattern: parse, caps, halt rule, `read`, blocks, regions, `AnyPngIssue`), `src/error.rs`, `fuzz/fuzz_targets/png_chunks.rs`, `fuzz/shared/png_chunk_invariants.rs`, `fuzz/shared/png_metadata_invariants.rs` (model pattern), `tests/png_walk.rs`, `tests/png_metadata.rs`, `tests/support/png_shapes.rs`, `tests/fuzz_layout.rs`, `examples/fuzz_seeds.rs`, `src/audit/sniff.rs` (the WebP signature).
- **Wiring points:** grep for `png_chunks` outside `target/`, `corpus/`, `artifacts/`, `projects/` to find every place a fuzz target is listed (ci.yml smoke matrix, `app.just`, `fuzz/Cargo.toml`, README, pins).

## Outputs

- **Created:** `src/webp/mod.rs` (`Webp`, `Chunk`, `ChunkKind`, `Vp8x`, constants, framing), `src/webp/issue.rs` (`WebpIssue`), `src/webp/metadata.rs` (`WebpMetadata`, `WebpExifBlock`, `WebpRegion`, `AnyWebpIssue`, `read`), `fuzz/fuzz_targets/webp_riff.rs`, `fuzz/shared/webp_riff_invariants.rs` (independent model of the walk and the extraction), `fuzz/seeds/webp_riff/*` (generator-written, at least 40 seeds), `tests/webp_walk.rs`, `tests/webp_metadata.rs`, `tests/support/webp_shapes.rs`.
- **Modified:** `src/lib.rs` (`pub mod webp;`, docs), `src/error.rs` (variants in DEC-019 with Display; make the existing `TooManyChunks` and `TooManyIssues` Display text walk-neutral), `examples/fuzz_seeds.rs`, `tests/fuzz_layout.rs`, `fuzz/Cargo.toml`, `.github/workflows/ci.yml` (smoke matrix), `app.just` if it lists targets, README (9 fuzz targets; WebP walked and metadata located; `exif-audit` still reports WebP unsupported until SPEC-032), tests with exhaustive `Error` matches (add arms; test code only).
- **API (all `pub`, documented, panic-free):**
  - `Webp::parse(&'a [u8]) -> Result<Webp<'a>, Error>` and `parse_strict`; `chunks()`, `issues()`, `is_clean()`, `declared_size() -> u32`, `vp8x() -> Option<Vp8x>`
  - `Chunk`: `offset()`, `fourcc() -> [u8; 4]`, `data() -> &'a [u8]`, `kind()`, `range()` (framing and pad included)
  - `Vp8x`: `flags() -> u8`, `has_icc()`, `has_alpha()`, `has_exif()`, `has_xmp()`, `is_animated()`, `canvas_width() -> u32`, `canvas_height() -> u32`
  - `webp::read(&'a [u8]) -> Result<WebpMetadata<'a>, Error>`: `webp()`, `exif_blocks()`, `exif()`, `regions()`, `webp_issues()`, `issues()` (layered, as `AnyPngIssue`), `is_clean()`
  - `WebpExifBlock`: `offset()`, `tiff_at()`, `tiff()`, `container()`; `WebpRegion` (`#[non_exhaustive]`): `Xmp { offset, data }`, `Icc { offset, data }`
  - constants `MAX_WEBP_CHUNKS = 65_536`, `MAX_WEBP_ISSUES = 1024`, `MAX_WEBP_EXIF_BLOCKS = 16`
- **No new dependency.**

## Acceptance Criteria

Every DEC-019 rule has a test that fails if the rule is removed, and the probe shapes below are the evidence table (ExifTool result in brackets).

- [x] **Fatal:** under 12 bytes is `Truncated`; wrong magic or a non-`WEBP` form type (a `RIFF....AVI `) is `NotWebp` carrying the 12 bytes; a 12-byte header alone is `Ok`, no chunks, no issue; `parse_strict` agrees.
- [x] **Walk runs to the end of the input, not the declared size:** declared size too small, zero, odd, too large, and trailing bytes after the last chunk all still find the `EXIF` chunk [read in every case], each with exactly one `RiffSizeMismatch { declared, actual }` at offset 4 (and none when the size is exact).
- [x] **Framing:** pad byte after an odd chunk is skipped (the next chunk is found); an odd chunk ending exactly at end of input is `MissingPad`; an unpadded odd chunk in the middle misframes the next chunk exactly as ExifTool does [XMP after it not read]; pad value unchecked.
- [x] **Damage:** chunk size past the end, 1 to 7 bytes left, a partial chunk header (`XMP ` plus 1 byte): one `Truncated` at the chunk, walk ends, the chunk omitted, earlier chunks and Exif kept [Orientation still read].
- [x] **Kinds are case-sensitive:** `exif`, `Exif`, `EXIf`, `xmp `, `iccp`, `XMP-`, `XMPx` are `Other` and yield no region or block [not read]; `XMP\0` is `Xmp` with one `NonCanonicalChunkId` [read, minor warning].
- [x] **`VP8X`:** flags and canvas decoded (`stored + 1`); a `VP8X` under 10 bytes is `BadVp8x` and `vp8x()` is `None` [3-byte VP8X still reads EXIF]; a `VP8X` that is not first is `MisplacedVp8x`.
- [x] **Flags are informational:** `EXIF` read with the flag unset, with no `VP8X`, before `VP8X`, before the image chunk; flag set with no chunk. Disagreements give `ChunkFlagMismatch` per flag (ICC, EXIF, XMP order, after the walk) and `MetadataWithoutVp8x` once when no `VP8X` exists but metadata chunks do; never an error.
- [x] **Exif:** container independence (block container equals bare `Container::parse` of `tiff()`; `tiff_at` indexes the same input byte); `Exif\0\0` prefix accepted with one `ImproperExifHeader`; empty and non-TIFF payloads leave the block `Err`; two `EXIF` chunks: first primary, second `ExtraExif` and parsed [both read]; the 17th parsed block is not created.
- [x] **Regions:** `XMP ` and `ICCP` located byte for byte (`data()` is a subslice of the input, pointer-range test); two `XMP ` chunks both located [both read]; metadata inside `ANMF` payloads is not found, `EXIF` after an `ANMF` is [not read / read].
- [x] **Halts and caps:** `MAX_WEBP_CHUNKS` exactly (65,536 clean, one more is `TooManyChunks`); issue cap exactly (`<= MAX_WEBP_ISSUES + 1`, the cap issue last); after any halt extraction issues are suppressed but regions and blocks already found are kept; at most two issues per chunk; no public issue bound is promised beyond the constants.
- [x] **Panic-free** on every prefix and single-byte flip of each seed and builder shape, including the empty input and an 11-byte header.
- [x] `webp_riff` target builds on nightly; its shared model (walk and extraction derived from raw bytes, every iff both ways, its own four-character-code table) runs under `cargo test` on stable and in the target; at least 40 seeds, names pinned, older seeds (tiff, jpeg, png) byte-identical; CI runs a 60 s smoke.
- [x] `just lint`, `just test`, `just lint-fuzz`, `just validate`, `just forbid-unsafe`, `just cost-audit`, `just fuzz-smoke webp_riff` green; no `#[allow]`; no unsafe.

## Failing Tests

Written first from these descriptions; fixtures come from `tests/support/webp_shapes.rs` (`chunk(code, data)` with correct padding, `chunk_unpadded`, `riff(body)`, `riff_with_size`, `vp8x(flags, w, h)`, a minimal `VP8L`, TIFF blobs). Tests are in two files so the walker can be committed and gated before the metadata work starts.

- **`tests/webp_walk.rs`:** `not_webp_is_fatal_and_header_only_is_ok`, `declared_size_never_limits_the_walk` (table: small, zero, odd, large, trailing), `riff_size_mismatch_is_one_issue`, `pad_byte_rules` (padded, missing at end, unpadded in the middle), `damage_ends_the_walk_and_omits_the_chunk` (table), `fourcc_kinds_are_case_sensitive`, `vp8x_decode_short_and_misplaced`, `chunk_cap_is_exact`, `issue_cap_halts_once`, `anmf_payload_is_not_walked`, `payloads_borrow_the_input`, `parse_strict_ok_iff_clean`, `every_prefix_and_byte_flip_is_panic_free`.
- **`tests/webp_metadata.rs`:** `exif_block_equals_bare_tiff`, `exif_prefix_issue`, `exif_empty_and_garbage_stay_err`, `second_exif_is_extra_but_parsed`, `exif_blocks_capped_at_sixteen`, `xmp_and_iccp_located_byte_for_byte`, `two_xmp_both_located`, `flags_are_informational` (each flag, with and without chunk, with and without VP8X), `flag_mismatch_issue_order`, `halt_suppresses_extraction_issues_but_keeps_regions`, `at_most_two_issues_per_chunk`.
- **`tests/fuzz_layout.rs`** (extend): `webp_riff` seed count and names pinned and equal to the builders byte for byte; older seeds unchanged; model passes on every seed; target listed in ci.yml and `fuzz/Cargo.toml`.
- **Red-proofs (in-file, control first):** the model goes red on mutated copies of its input: a missed chunk, a flipped flag bit, a region offset +1, a dropped `RiffSizeMismatch`; the control passes first.

## Non-Goals

- Writing or stripping WebP (M1d); decoding VP8, VP8L, ALPH or ANMF frames
- Parsing XMP or ICC contents
- Audit categories, rendering and wiring: SPEC-032
- ExifTool parity tests: SPEC-031 (this spec's evidence is the measured table in DEC-019)
- Any change to PNG or JPEG behaviour

## Notes for the Implementer

- **Two checkpoints.** Build the walker first (tests, `src/webp/mod.rs`, issue type, errors, the walk half of the model, seeds, wiring), run its gates and COMMIT. Then the metadata half. If you are past about 60 percent of your budget when the walker is committed, hand back after it with the metadata as a clearly listed remainder: the orchestrator would rather split than receive a rushed second half.
- Mirror the PNG code (module docs with provenance, DEC-003 class 1 from the WebP Container Specification and RIFF; progress argument; checked 64-bit size arithmetic; `get` for every slice; no indexing, no `#[allow]`, no unsafe, no dependency).
- The model in `fuzz/shared/webp_riff_invariants.rs` derives chunks, vp8x, issues, blocks and regions from raw bytes with its own four-character-code table; do not call library parsing from it.
- Seeds: the probe shapes in DEC-019's Context are the seed list; add a seed per acceptance bullet; keep names stable.
- Fixture builders and seed generation share one function set (as for TIFF, JPEG and PNG). Update every pin; grep for the last-added png seed name before and after.
- Walk-neutral wording for the shared `TooManyChunks` and `TooManyIssues` messages; check that no test pins the old text.
- Budget: about 100 tool calls or 250k context.

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
