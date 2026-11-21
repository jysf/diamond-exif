---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-028
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
  decisions: [DEC-017, DEC-006, DEC-001, DEC-002]
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
      handoff: HANDOFF-054
      agent: claude-sonnet-5-5
      interface: other
      basis: agent-result
      tokens_total: 254968
      estimated_usd: 1.68
      duration_minutes: 13
      recorded_at: 2026-11-21
      notes: "No token counter is available inside this subagent run, so tokens_total is null rather than invented; the orchestrator replaces it with the harness subagent_tokens figure. About 60 tool calls."
  totals:
    tokens_total: 254968
    estimated_usd: 1.68
    session_count: 1
---

# SPEC-028: PNG chunk walker: signature, chunk lengths, CRC as an issue, ordering, caps, `png_chunks` fuzz target (DEC for the PNG walk, modelled on DEC-006)

## Context

First of the PNG and WebP reader stage (STAGE-008, PROJ-001). JPEG got a tolerant segment walker with a typed issue channel (DEC-006); PNG is a chunk stream (length, type, data, CRC) after an eight-byte signature and needs the same discipline before any metadata is read from it.

## Goal

Walk a PNG tolerantly: fatal only when it is not a PNG; every other problem (length past EOF, CRC mismatch, missing IHDR or IEND, chunk order, trailing bytes, caps) is a typed per-chunk issue and the walk continues where safe; chunks borrow the input. Ships the `png_chunks` fuzz target and its seeds, and a DEC for the PNG walk.

## Inputs

- **Files to read:** `src/jpeg/mod.rs` and `src/jpeg/issue.rs` (the pattern to mirror: walk loop, caps, `Jpeg`, `JpegIssue`), `src/error.rs` (add variants), `fuzz/fuzz_targets/jpeg_segments.rs` and `fuzz/shared/jpeg_metadata_invariants.rs` (target and shared-invariant pattern), `tests/fuzz_layout.rs` (seed pins), `examples/fuzz_seeds.rs` (seed generator), `tests/support/` (fixture builders), `.github/workflows/ci.yml` line ~187 (smoke matrix).
- **Decision:** `decisions/DEC-017-png-chunk-walk-and-issue-channel.md` is the contract; follow it exactly.
- **Find every wiring point** by grepping for the last-added fuzz target name (`rewrite_tiff`) and the string `jpeg_segments` outside `target/`, `corpus/`, `artifacts/`, `projects/`: README, ci.yml, justfile/app.just, fuzz/Cargo.toml, tests that pin target lists.

## Outputs

- **Created:** `src/png/mod.rs` (`Png`, `Chunk`, `ChunkKind`, constants), `src/png/issue.rs` (`PngIssue`), `src/png/crc.rs` (CRC-32, const table), `fuzz/fuzz_targets/png_chunks.rs`, `fuzz/shared/png_chunk_invariants.rs` (independent model, including a bitwise CRC-32), `fuzz/seeds/png_chunks/*` (written by the generator), `tests/png_walk.rs`, `tests/support/png_shapes.rs`.
- **Modified:** `src/lib.rs` (`pub mod png;`, crate docs), `src/error.rs` (variants named in DEC-017, with Display), `examples/fuzz_seeds.rs`, `tests/fuzz_layout.rs` (png_chunks seed count and names pinned, shared invariants included), `fuzz/Cargo.toml`, `.github/workflows/ci.yml` (smoke matrix), `justfile`/`app.just` smoke list if it names targets, `README.md` (8 fuzz targets, the PNG line), `CHANGELOG.md` (Unreleased).
- **API (all `pub`, panic-free, documented):**
  - `Png::parse(&'a [u8]) -> Result<Png<'a>, Error>` and `Png::parse_strict` (Ok iff clean; Err equals the first issue's error)
  - `Png::chunks() -> &[Chunk<'a>]`, `issues() -> &[PngIssue]`, `is_clean()`, `iend_offset() -> Option<usize>`, `trailer() -> &'a [u8]`
  - `Chunk::offset()`, `chunk_type() -> [u8; 4]`, `data() -> &'a [u8]`, `stored_crc() -> u32`, `crc_ok() -> bool`, `kind() -> ChunkKind`, `after_idat() -> bool`, `range() -> Range<usize>` (whole chunk, framing included)
  - `PngIssue::error()`, `offset()`; `MAX_PNG_CHUNKS = 65_536`, `MAX_PNG_ISSUES = 1024`
- **No new dependency;** `[dependencies]` stays empty.

## Acceptance Criteria

- [x] Not a PNG: fewer than 8 bytes is `Truncated`; any other first 8 bytes is `NotPng` carrying them; both fatal and equal under `parse_strict`.
- [x] A signature-only input is `Ok` with issues `MissingIhdr` and `MissingIend`; no chunks.
- [x] A valid minimal PNG (IHDR, IDAT, IEND) is clean: three chunks, `crc_ok()` true on each, `iend_offset()` set, empty trailer, `parse_strict` is `Ok`.
- [x] CRC: a wrong stored CRC gives `BadChunkCrc { stored, computed }`, the chunk stays recorded with `crc_ok() == false`, and later chunks are still walked. CRC-32 of `123456789` is `0xCBF43926` and of an `IEND` chunk `0xAE426082`.
- [x] Length past EOF, fewer than 12 bytes left, and a length above 2^31-1 each end the walk with exactly one `Truncated` or `BadChunkLength` at the chunk offset; the chunk is not recorded; `MissingIend` is not added.
- [x] First chunk not `IHDR` gives one `MissingIhdr` (walk continues); an `IHDR` of length other than 13 gives `BadIhdr`; no `IEND` at a clean end of input gives `MissingIend`; non-letter type bytes give `BadChunkType` with the chunk kept as `Other`.
- [x] `IEND` ends the walk; bytes after it are `trailer()`, borrowed, no issue. `after_idat()` is true exactly for chunks after the first `IDAT`.
- [x] Zero-copy: every `data()` and `trailer()` is a subslice of the input (pointer-range test).
- [x] Caps: an input with more than `MAX_PNG_CHUNKS` chunks records exactly that many and ends with one `TooManyChunks`; more than `MAX_PNG_ISSUES` ordinary issues ends with one `TooManyIssues` (total issues `<= MAX_PNG_ISSUES + 1`).
- [x] Panic-free over every prefix and every single-byte corruption of each seed (loop test), and over the empty input.
- [x] `png_chunks` fuzz target builds on nightly, its shared invariants run under `cargo test` on stable, the committed seeds (at least 24, named in the test) all parse without panic, the generator leaves all pre-existing seeds byte-identical, and CI runs a 60 s smoke for it.
- [x] ExifTool agreement on the DEC-017 probe shapes is recorded as the DEC says (design evidence); no new oracle test in this spec (SPEC-031).
- [x] `just lint`, `just test`, `just validate`, `just lint-fuzz` green; `forbid(unsafe_code)` and the five-lint deny set unchanged; no `#[allow]`.

## Failing Tests

Written first by the implementer from these descriptions (design supplies the cases; fixtures come from `tests/support/png_shapes.rs`, one builder: signature, `chunk(type, data)`, `chunk_with_crc`, `minimal()`).

- **`tests/png_walk.rs`**
  - `not_png_is_fatal` / `short_input_is_truncated` / `signature_only_has_missing_ihdr_and_iend`: fatal cases and the signature-only case, strict equals.
  - `minimal_png_is_clean`: three chunks, crc ok, iend offset, empty trailer.
  - `crc_mismatch_is_an_issue_and_the_chunk_is_kept` and `crc32_known_vectors`.
  - `length_past_eof_ends_the_walk_without_the_chunk` (table: past EOF by 1, 11 bytes left, 0x80000000, 0xFFFFFFFF), asserting no `MissingIend`.
  - `missing_ihdr_bad_ihdr_missing_iend_bad_type`: one test per issue, plus a combined shape that has several at once and keeps order.
  - `iend_ends_the_walk_and_the_rest_is_the_trailer`; `after_idat_flag`.
  - `payloads_borrow_the_input`.
  - `chunk_cap_and_issue_cap_halt_once` (built shapes: 65,537 empty `tEXt` chunks; 1,025+ bad-CRC chunks).
  - `every_prefix_and_every_byte_flip_of_each_shape_is_panic_free`.
  - `parse_strict_ok_iff_clean`.
- **`tests/fuzz_layout.rs`** (extend): `png_chunks` seed count and name set pinned; committed seeds equal the builders byte for byte; pre-existing seeds unchanged; the shared invariants (`include!` of `png_chunk_invariants.rs`) pass on every seed; `png_chunks` appears in ci.yml's smoke matrix and fuzz/Cargo.toml.
- **`src/png/crc.rs` unit tests:** table spot-checks and the two vectors above.
- **Red-proofs (in-file, control first):** the model's independent CRC rejects a one-bit-perturbed expected value; a deliberately wrong `crc_ok` (mutated copy of the invariant input) is caught by the shared invariants; removing the cap check in a copied walker config is not available, so the cap tests assert the exact counts both sides of the limit.

## Non-Goals

- Reading chunk contents (eXIf, iTXt, iCCP): SPEC-029
- Inflating any zlib stream or decoding pixels
- ExifTool parity tests: SPEC-031
- Writing or stripping PNG (M1d)
- Walking chunks inside the trailer (SPEC-029 decides, on oracle evidence)

## Notes for the Implementer

- Mirror `src/jpeg/mod.rs` closely: the same module docs on provenance (DEC-003 class 1, net-new from the PNG specification), progress argument, caps, `parse`/`parse_strict`, and the `#[non_exhaustive]` error additions.
- Use checked arithmetic for every offset: `len` is attacker-controlled up to `u32::MAX`; compare it with the remaining bytes, never add before comparing.
- The CRC table must be `const` and built with a `const fn`; no `static mut`, no unsafe.
- The fuzz model in `fuzz/shared/png_chunk_invariants.rs` must NOT call the library's CRC: use the bitwise algorithm so the model is independent.
- Fixtures are generated by one function set shared by tests and `examples/fuzz_seeds.rs` (as for TIFF and JPEG); no committed binary fixture except seeds.
- Run `just fuzz-smoke png_chunks` before handing back (needs the nightly PATH trick in the fuzz target docs).
- Budget: about 100 tool calls or 250k context; if you are over half way with the walker unfinished, hand back with what is done and what is not.

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
