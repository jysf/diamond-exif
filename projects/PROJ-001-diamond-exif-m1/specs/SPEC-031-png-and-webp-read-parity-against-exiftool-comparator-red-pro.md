---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-031
  type: story                      # epic | story | task | bug | chore
  cycle: verify  # frame | design | build | verify | ship
  blocked: false
  priority: medium                 # critical | high | medium | low
  complexity: L                    # XS | S | M | L | XL | XXL — the EXPECTED size, set at design
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
  decisions: [DEC-017, DEC-018, DEC-006]
  constraints: [oracle-must-be-shown-red]
  related_specs: []                # [SPEC-NNN]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: [SPEC-029, SPEC-030]

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
      handoff: HANDOFF-060
      agent: claude-sonnet-5-5
      interface: other
      basis: agent-result
      tokens_total: 330309
      estimated_usd: 2.18
      duration_minutes: null
      recorded_at: 2026-11-21
      notes: "tokens_total is null: a sub-agent cannot read its own transcript token counts; the orchestrator replaces it with the harness figure. About 75 tool calls."
  totals:
    tokens_total: 330309
    estimated_usd: 2.18
    session_count: 1
---

# SPEC-031: PNG read parity against ExifTool: comparator, records, red-proofs

## Context

Fourth spec of STAGE-008 (split at design: WebP parity is SPEC-034, which reuses this apparatus). SPEC-028 to SPEC-030 built the PNG and WebP readers, with ExifTool probes made at design. This spec turns the PNG half of that evidence into committed, replayable, red-proofed tests, exactly as SPEC-011 did for JPEG (`tests/jpeg_oracle.rs`, `tests/support/oracle_jpeg.rs`, `tests/oracle-fixtures/exiftool/jpeg-*.txt`, the `tests/mutation_oracle.rs` mutants). The JPEG apparatus is about 3,000 lines: mirror its shape, do not reinvent it, and do not refactor it beyond what the PNG half needs.

## Goal

Show ExifTool 13.55 agrees with `png::read` on PNG fixtures and on the committed `png_chunks` seeds: the same 20 Exif value selectors, plus region presence (XMP, ICC), with every measured difference a pinned `KNOWN_DIVERGENCES` entry; the comparator is red-proofed (replay-and-perturb) and the PNG layer has mutated-copy red-proofs (control first); every divergence is reported and tracked in `guidance/questions.yaml`, never silenced.

## Inputs

- **Pattern to mirror (read first):** `tests/jpeg_oracle.rs` (live tests, replay tests, `KNOWN_DIVERGENCES` handling, warning-table pins, perturb red-proof, skip-is-loud test), `tests/support/oracle_jpeg.rs` (independent fixture builders: no code shared with `src/`, `examples/fuzz_seeds.rs` or the fuzz models), `tests/support/oracle.rs` (selectors, `Divergence`, `read_ours_jpeg`, `diff`, `compare_jpeg`, `REGION_SELECTORS`), `tests/support/tools.rs` (`run_tool_with_limit`, record format), `examples/oracle_record.rs` (the recorder), `tests/mutation_oracle.rs` plus `tests/support/probe.rs` and `mutate.rs` (library-fault apparatus), `guidance/questions.yaml` (divergence entries).
- **The readers under test:** `src/png/` (DEC-017, DEC-018). **The evidence already gathered at design** is in DEC-017 and DEC-018 Context and their clarifications (ExifTool behaviour on about 30 PNG shapes); re-measure, never trust it: a record is the evidence, not the DEC.
- **Oracle safety:** ExifTool runs only through `run_tool_with_limit`; every new hostile shape is first probed under the time limit, and a shape on which ExifTool does not terminate is recorded as a divergence (as `two-hop-cycle` was).

## Outputs

- **Created:** `tests/support/oracle_png.rs` (independent fixture builders and the named fixture list), `tests/png_oracle.rs` (live, replay, red-proof tests), `tests/oracle-fixtures/exiftool/png-<name>.txt` records (one per fixture, `png-seed-<seed>.txt` per committed `png_chunks` seed), `tests/png_mutation_oracle.rs` (or a clearly separated section of `tests/mutation_oracle.rs`, builder's choice, documented).
- **Modified:** `tests/support/oracle.rs` (`read_ours_png`, PNG region selectors, `compare_png`), `examples/oracle_record.rs` (PNG records; `just oracle-record` keeps its name), `tests/support/probe.rs` (a probe binary that reads a PNG through `png::read`), `guidance/questions.yaml` (one entry per divergence), README or docs only if a count changes.
- **Fixtures (all named `png-...`, independent builders):** (a) every TIFF fixture that the JPEG oracle wraps (`jpeg-<tiff name>`), wrapped in an `eXIf` chunk of a minimal valid PNG; (b) shapes, at least: `eXIf` after IDAT, after IEND; `Exif\0\0` prefix; empty and non-TIFF `eXIf`; two `eXIf` with different IFD0 contents; bad CRC on the `eXIf` chunk; no IHDR; no IEND; length past EOF; length 0x80000000; `iTXt` XMP uncompressed, compressed (a zlib stream built with stored deflate blocks, no dependency), legacy `tEXt` XMP, `zTXt` XMP, XMP under another keyword, lowercase keyword; `iCCP` valid and with a bad header; `Raw profile type exif` and `xmp` in `tEXt`, `zTXt`, `iTXt` (the hex body built by the harness); a 80-byte keyword; `iTXt` flag 2; `eXIf` plus raw-profile Exif together; a trailer of arbitrary bytes; `eXIf` before IHDR. Add any shape the measurements suggest.
- **No new dependency, no change to `src/`** unless parity exposes a reader bug: then fix it in this spec with a regression test and say so in the handback.

## Acceptance Criteria

- [x] **Replay (no ExifTool installed):** for every committed `png-*.txt` record the comparator reports zero mismatches on an unmodified reading, except entries pinned in `KNOWN_DIVERGENCES`; the record headers carry the exact command, the ExifTool version and the stderr warnings.
- [x] **Live (ExifTool installed):** the same comparison over freshly run ExifTool on the fixtures and the `png_chunks` seeds; a missing ExifTool prints a loud `SKIP` and passes unless `DIAMOND_EXIF_REQUIRE_ORACLE=1`, which then fails (test for both).
- [x] **What is compared:** the 20 Exif selectors through `read_ours_png`, whose treatment of several Exif blocks is a MEASURED model of ExifTool's behaviour (two `eXIf`, `eXIf` plus raw-profile Exif), documented in the file and pinned by a test; plus region presence: XMP present, ICC present.
- [x] **Known limits are pinned divergences, not silent:** raw-profile Exif values and compressed XMP content are read by ExifTool and only located by us (DEC-018): each is a `KNOWN_DIVERGENCES` entry with command, version, measured stdout, which side is right and a `questions.yaml` id; a test requires every entry to be PRESENT (a divergence that stops existing fails the build) and every entry to have its question.
- [x] **Warning table:** a test pins our `PngIssue` kinds beside ExifTool's warnings for the shapes that produce both (prefix header, truncated, missing IHDR, trailer data, chunks after IDAT) and states where ExifTool is silent (bad CRC) or louder.
- [x] **Container independence:** the bare TIFF read equals the `eXIf`-wrapped read for every wrapped fixture, live and by record.
- [x] **Replay-and-perturb red-proof:** changing one field of one committed record turns the comparison red naming exactly that selector; control first (unperturbed is green).
- [x] **Mutated-copy red-proofs** (apparatus of `tests/mutation_oracle.rs`, control first, mutation marked `RED-PROOF INJECTION`, located by a needle that must occur exactly once): at least six library faults in `src/png/` each turn the comparison red naming the expected selectors, for example: the `eXIf` TIFF offset is wrong by one; the first block is not primary; the `Exif\0\0` prefix is not skipped; the trailer scan is disabled (`eXIf` after IEND lost); `XML:com.adobe.xmp` routing broken (XMP presence lost); `iCCP` not located; plus two byte mutations of a fixture (a corrupted offset inside the `eXIf` payload; truncation between IHDR and IDAT). The injection marker must not leak into `src/` or `tests/support/` (reuse the leak test).
- [x] **Hostile shapes** are probed under the time limit before being recorded; any non-terminating shape is a recorded divergence.
- [x] Every divergence found is in `guidance/questions.yaml`; if parity exposes a reader bug it is fixed here with a regression test.
- [x] `just lint`, `just test`, `just mutation-oracle`, `just validate`, `just cost-audit`, `just forbid-unsafe` green; `just oracle-record` is idempotent (run twice, no diff); no `#[allow]` in `src/`.

## Failing Tests

Written first. The build is mostly tests, so "failing first" means the comparator tests exist with the fixture list before the records are generated, and are seen red for the right reason (missing records) before `just oracle-record` produces them.

- **`tests/png_oracle.rs`:** `png_fixtures_are_deterministic_and_uniquely_named`; `every_wrapped_tiff_fixture_is_present_and_matches_its_tiff_bytes`; `replay_png_records_are_clean_on_an_unmodified_reading`; `replay_png_headers_record_command_version_and_stderr`; `oracle_names_the_one_field_perturbed_in_a_png_record`; `container_independence_every_wrapped_fixture_reads_the_same_bare_and_in_png`; `several_eXIf_model_is_measured_and_pinned`; `region_presence_xmp_and_icc_agree_with_exiftool`; `xmp_keyword_rules_agree` (itxt, text, ztxt, other keyword, lowercase); `raw_profile_and_compressed_xmp_are_recorded_divergences_and_still_present`; `exif_after_idat_and_after_iend_agree`; `warning_table_pins_our_issue_kinds_beside_exiftools_warnings`; `every_known_divergence_has_a_question_entry`; `a_divergence_that_stops_being_one_fails`; `live_png_parity_on_fixtures`; `live_png_parity_on_the_png_seeds`; `live_png_skip_is_loud_and_required_mode_fails`.
- **`tests/png_mutation_oracle.rs`:** `controls_are_green`, one `mutant_...` test per fault above, `byte_mutation_...` for the two corruptions, `injection_marker_does_not_leak`.

## Non-Goals

- WebP parity (SPEC-034); audit inventory parity (SPEC-032)
- Decoding or inflating anything in the library (DEC-018): divergences for raw profiles and compressed XMP are the expected outcome, not a defect to fix here
- Changing the JPEG or TIFF oracle behaviour; refactoring the shared apparatus beyond adding PNG entries

## Notes for the Implementer

- **Two checkpoints.** (1) fixtures, `oracle_png.rs`, `read_ours_png`, recorder, records, replay and live tests, divergences and questions, warning table: gates, COMMIT. (2) the mutation oracle and red-proofs: gates, COMMIT. If past about 60 percent of budget at checkpoint 1, hand back after it.
- Measure before modelling: the several-Exif rule and the ICC and XMP selectors must come from records, not from DEC text; write the model last, from the records.
- The fixture builders must not share encoding code with `src/`, `examples/fuzz_seeds.rs` or `tests/support/png_shapes.rs` (the oracle must not share a bug with the thing it checks): write them from the PNG specification again. A stored-deflate zlib stream needs only an Adler-32 and a CRC-32 written in the file.
- Names are normative: `png-<tiff name>` for wrapped TIFFs, `png-<shape>` for shapes, `png-seed-<seed>` for seeds.
- The shared oracle files are large: add, do not rewrite; keep the diff to `oracle.rs` and `oracle_record.rs` small and additive.
- Probe each new hostile shape under `run_tool_with_limit` before recording it.
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
