---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-034
  type: story                      # epic | story | task | bug | chore
  cycle: ship  # frame | design | build | verify | ship
  blocked: false
  priority: medium                 # critical | high | medium | low
  complexity: L                    # XS | S | M | L | XL | XXL — the EXPECTED size, set at design
                                   #   (XL/XXL almost certainly means it's a stage, not a spec)
  complexity_actual: null          # stamped at ship: what it ACTUALLY took, same scale.
                                   #   Expected-vs-actual drift is what `just calibration` reads.
  verify_verdict: punch-list  # approved | punch-list | rejected — the OUTCOME of the verify
                                   #   cycle, stamped by `just advance-cycle` when the spec leaves
                                   #   verify (same three verdicts Prompt 4 already returns).
                                   #   Recorded in front-matter, not just prose, so "verify never
                                   #   rejects anything" stops being a hunch and becomes a number.
  verify_rounds: [punch-list]  # every verify outcome, oldest first, appended by
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
  decisions: [DEC-019, DEC-006, DEC-018]
  constraints: [oracle-must-be-shown-red]
  related_specs: []                # [SPEC-NNN]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: [SPEC-031]

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
  tokens_estimate: 400000
  sessions:
    - cycle: build
      handoff: HANDOFF-064
      agent: claude-sonnet-5-5
      interface: other
      basis: estimate
      tokens_total: 299389
      estimated_usd: 1.98
      duration_minutes: null
      recorded_at: 2026-11-21
      notes: "tokens_total is null: the implementer cannot read its own transcript token counts; the orchestrator replaces it with the harness figure. About 75 tool calls used. Not pushed, no PR."
    - cycle: verify
      handoff: HANDOFF-065
      agent: claude-opus-5-5
      interface: other
      basis: estimate
      tokens_total: 216520
      estimated_usd: 1.43
      duration_minutes: 60
      recorded_at: 2026-11-21
      notes: "tokens_total is null: the verifier cannot read its own transcript token counts; the orchestrator replaces it with the harness figure. About 45 tool calls. Review only; nothing pushed."
    - cycle: build
      handoff: HANDOFF-066
      agent: claude-sonnet-5-5
      interface: other
      basis: estimate
      tokens_total: 179076
      estimated_usd: 1.18
      duration_minutes: null
      recorded_at: 2026-11-22
      notes: "tokens_total null: the subagent session exposes no token count; the orchestrator replaces it with the harness figure. Not measured: duration."
  totals:
    tokens_total: 694985
    estimated_usd: 4.59
    session_count: 3
shipped_at: 2026-11-22
---

# SPEC-034: WebP read parity against ExifTool: comparator, red-proofs, SPEC-030 carried gaps

## Context

Fifth spec of STAGE-008, split from the original parity spec at the SPEC-031 design. SPEC-031 shipped the PNG parity apparatus (independent fixtures, committed records, replay and live tests, pinned divergences, mutated-copy red-proofs); this spec applies the same apparatus to the WebP reader (`src/webp/`, DEC-019). It also closes the test and seed gaps the SPEC-030 verify found.

SPEC-031 needed three fix rounds because its fixture list was hand-picked: every verify round found a located-metadata disagreement in a shape outside the list. So here the fixture list is a **variant grid** (every chunk code, size parity and length class, every header field, every `VP8X` length), measured at design (68 shapes, ExifTool 13.55, results below) and to be re-measured by the builder, never trusted.

## Goal

Show ExifTool 13.55 agrees with `webp::read` on the WebP fixtures and on every committed `webp_riff` seed: the same 20 Exif selectors plus XMP and ICC presence on the hand-built fixtures (the `webp_riff` seeds are compared on the 20 Exif selectors only, as for PNG), with every measured difference a pinned `KNOWN_DIVERGENCES` entry that has a `guidance/questions.yaml` id; the comparator red-proofed (replay-and-perturb) and the WebP layer given mutated-copy red-proofs (control first); the SPEC-030 carried tests and seeds added.

## Measured at design (ExifTool 13.55; `-a -G1 -s -n`, and `-T -n -s3` for values)

- **Several `EXIF` chunks MERGE; the winner is PER SELECTOR (corrected at the verify, HANDOFF-065/066).** ExifTool 13.55 keeps the FIRST block's value for `IFD0:ImageWidth`, `ImageHeight`, `Orientation`, `XResolution` and the LAST for the other 16 (measured on two full blocks that differ in every selector, both orders; `oracle::WEBP_FIRST_BLOCK_WINS`). The original wording follows, true for Make and Model only: the later wins (like JPEG, unlike PNG, where only the first counts): `EXIF(Make=Canon)` then `EXIF(Make=Nikon, Model=D800)` reads Make=Nikon, Model=D800; reversed reads Make=Canon, Model=D800. A garbage block before or after a good one changes nothing. The model for `read_ours_webp` is therefore the JPEG one (the last block carrying the tag wins), NOT the PNG one; it must be pinned by a test. The library's `exif()` is "first block is primary" (DEC-019); the model reads all blocks, as the JPEG oracle does.
- **Read:** `EXIF` with the `VP8X` flag unset, with no `VP8X`, before `VP8X`; with a leading `Exif\0\0` (warning "[minor] Improper EXIF header"); after an `ANIM`; after an `ANMF` at top level. `XMP ` and `XMP\0` (warning "[minor] Incorrect XMP tag ID"); two `XMP ` both; an empty `XMP ` then a real one: the real one.
- **Not read:** `EXIF` inside an `ANMF` payload; `Exif`, `exif`; `xmp `, `XMP-`; `iccp`; an empty `EXIF`, `XMP `, `XMP\0`, `ICCP`; a 1-byte `EXIF`; a garbage `EXIF` (no tags).
- **Declared RIFF size never limits the walk:** size 0, 10, odd, 0xFFFFFFF0: `EXIF` still read, no warning. Trailing junk of 12 bytes or of 3 bytes after the last chunk: `EXIF` read AND warning "Error reading RIFF file (corrupted?)" (ours: `Truncated`). A chunk length past the end, 0xFFFFFFFF, 5 or 7 bytes left after a chunk: same warning, earlier chunks kept.
- **Padding:** an odd chunk without its pad byte as the LAST chunk: read, warning "No padding on odd-sized EXIF chunk" (or `XMP`); followed by another chunk: the later chunk is lost (an `EXIF` after an unpadded `XMP `, an `XMP ` after an unpadded `EXIF`). Ours applies the pad rule, so it should lose the same chunk: compare, do not assume.
- **`VP8X` is informational:** lengths 0, 3, 9, 10, 18, two `VP8X`, a short then a full one, a late `VP8X`: `EXIF` read in every case, no warnings.
- **ICC:** a valid profile (128-byte header with `acsp`, one `desc` tag) gives `ICC-header:ProfileFileSignature: acsp` with the `VP8X` flag set or unset; garbage, a 132-byte stub and a header-only profile give no signature and warnings ("Bad length ICC_Profile", "Bad ICC_Profile table"); a valid profile followed by garbage: signature present. A profile whose tag table has 0 entries reads nothing. `ICCP` is therefore like PNG `iCCP`: we LOCATE a region, ExifTool VALIDATES it.
- **XMP content:** a packet carrying `xmp:CreatorTool` reads it; non-XML bytes give warning "[minor] Empty XMP" and no tags; so does an `XMP\0` chunk of 1 to 7 bytes.
- **Headers (the fatal ones, all `Err` for us):** 11 bytes: Error "File format error"; 12 bytes only: no output; `RIFF....WAVE` + `EXIF`: ExifTool reads it AS A WAV file and prints Make; `RIFX....WEBP` + `EXIF`: Make read with warning "Processing TIFF-like data after unknown 20-byte header". These two are expected divergences (we are right: not a WebP), to be recorded, not silenced.

## Inputs

- **Pattern to mirror (read first):** `tests/png_oracle.rs`, `tests/support/oracle_png.rs`, `tests/png_mutation_oracle.rs`, the PNG parts of `tests/support/oracle.rs` (`read_ours_png`, `PNG_REGION_SELECTORS`, `read_png_regions`, `diff_png`, `compare_png`, `KNOWN_DIVERGENCES`, `EXIF_UNREAD`, `png_warning_lines`), `examples/oracle_record.rs` (`record_png`), `tests/support/probe.rs`, `tests/support/tools.rs`. For the several-blocks model: `read_ours_jpeg` in `oracle.rs`. Read `guidance/questions.yaml` for how divergences are written.
- **The reader under test:** `src/webp/` and DEC-019 (with its clarifications). Its ExifTool claims are leads: the committed records are the evidence.
- **Independence:** the fixture builders are written in the new file from the WebP Container Specification and share no code with `src/`, `examples/fuzz_seeds.rs`, `tests/support/webp_shapes.rs` or the fuzz models. The TIFF blobs come from `oracle_tiff` and the ICC profile bytes from `oracle_png::icc_profile` (both are oracle-layer, independent of `src/`); say so in a comment.

## Outputs

- **Created:** `tests/support/oracle_webp.rs` (builders, the named fixture list, `WARNING_FIXTURES`, `WARNINGS_RECORD`), `tests/webp_oracle.rs` (live, replay, red-proof tests), `tests/webp_mutation_oracle.rs`, records `tests/oracle-fixtures/exiftool/webp-<name>.txt` (one per fixture), `webp-seed-<seed>.txt` (one per committed `webp_riff` seed), `webp-warnings.txt`.
- **Modified (additive, keep the diffs small):** `tests/support/oracle.rs` (`read_ours_webp`, WebP region selectors, `read_webp_all`, `diff_webp`, `compare_webp`, new `KNOWN_DIVERGENCES` entries, a `RegionKind` variant if needed), `examples/oracle_record.rs` (`record_webp`), `tests/support/probe.rs` (a probe that reads a file starting `RIFF` through `webp::read`), `guidance/questions.yaml`, `.filter` skips for `webp-*` in `jpeg_oracle`, `budget_oracle`, `inventory_oracle` and `png_oracle` wherever the PNG ones skip `png-*` (look at how SPEC-031 did it), `tests/png_oracle.rs`'s record-set test if it would otherwise count `webp-*` files, `app.just` (`mutation-oracle` runs the third file).
- **Carried SPEC-030 gaps (checkpoint 0, small, first):** (a) a unit test in `tests/webp_walk.rs` or `tests/webp_metadata.rs` plus a `webp_riff` seed for the first-`VP8X` rule: `RIFF` + `VP8X`(flags 0x00) + `VP8X`(flags 0x08) + `VP8L` + `EXIF` gives `vp8x().flags() == 0` and exactly one `ChunkFlagMismatch` for EXIF with the flag false (a mutant making the last `VP8X` win survives today); (b) a `vp8x-nine-bytes` seed (a `BadVp8x` threshold of 9 is otherwise killed only by unit tests); (c) `MAX_WEBP_CHUNKS` and `MAX_WEBP_ISSUES` pinned as literals (65_536 and 1024) in a test; (d) the fuzz model runs on ALL walk seeds, not only the `meta-` ones. New seeds update the pins in `tests/fuzz_layout.rs` and `examples/fuzz_seeds.rs`; the seed count changes, so find the pins by searching, do not guess.
- **No new dependency, no `src/` change** unless parity exposes a reader bug: then fix it with a regression test, and say so in the handback.

## Fixtures (names normative, all `webp-...`)

- **Wrapped TIFFs:** `webp-<tiff fixture name>` for every `oracle_tiff` fixture the JPEG and PNG oracles wrap, in an `EXIF` chunk of a minimal simple WebP (`RIFF`, `VP8X`(EXIF flag), `VP8L`, `EXIF`).
- **The grid (at least these; the measurement above names the expected result):** EXIF: basic, `Exif\0\0` prefix, two with different contents both orders, bad then good, good then bad, empty, 1 byte, garbage, good then prefixed, before `VP8X`, after `ANIM`, inside `ANMF`, `ANMF` then top-level, odd size padded, odd size unpadded last, odd unpadded then `XMP `; chunk-code case variants (`Exif`, `exif`, `EXIf`); XMP: basic, `XMP\0`, empty, `XMP\0` empty, `XMP\0` of 1 to 7 bytes, two, empty then real, garbage, `xmp `, `XMP-`, `XMPx`, odd unpadded last, odd unpadded then `EXIF`; ICC: valid with flag set, valid with flag unset, garbage, 132-byte stub, empty, `iccp`, valid then garbage, garbage then valid; all three together; RIFF size 0, 10, odd, 0xFFFFFFF0, exact; trailing 12 bytes, 3 bytes, 1 byte; chunk length past EOF, 0xFFFFFFFF, 0x7FFFFFFF; partial chunk header of 5 and 7 bytes; `VP8X` lengths 0, 3, 9, 10, 18, two, short then full, late; flags all wrong; `ALPH` before `VP8 `; a simple `VP8 ` file with `EXIF` appended; headers: 11 bytes, 12 bytes only, `RIFX`, `RIFF....WAVE`, `RIFF....AVI `; a `JUNK` chunk first; an animated file with 3 `ANMF` and `EXIF`. Add any shape the measurement suggests. A shape on which ExifTool does not finish within the limit is a recorded divergence (probe each under `run_tool_with_limit` first).
- **Seeds:** every committed `webp_riff` seed (51 today, plus the new ones) compared as `webp-seed-<seed>`.

## What is compared

The 20 Exif selectors through `read_ours_webp` (the JPEG-style merge model above; a fatal `Err` reads as all `None`), plus: `XMP-xmp:CreatorTool` present when we located a NON-EMPTY `Xmp` region; `ICC-header:ProfileFileSignature` present when we located a NON-EMPTY `Icc` region. Empty regions never count (DEC-019). Fixtures carrying a packet always use a `xmp:CreatorTool` packet and a valid profile where the shape is meant to read.

## Acceptance Criteria

- [x] **Carried gaps (a) to (d)** done with tests killing the named mutants.
- [x] **Replay (no ExifTool):** every committed `webp-*.txt` record compares clean on an unmodified reading except pinned `KNOWN_DIVERGENCES`; headers carry command, ExifTool version and warnings.
- [x] **Live:** same comparison over freshly run ExifTool on the fixtures and the `webp_riff` seeds; missing ExifTool is a loud `SKIP`, a failure under `DIAMOND_EXIF_REQUIRE_ORACLE=1` (both tested).
- [x] **Several-`EXIF` model measured and pinned** by a test from the records (per selector: the first block wins four selectors, the later block the other 16; bad blocks ignored; all 20 read in both orders), documented in `oracle.rs` as different from PNG.
- [x] **Container independence:** the bare TIFF read equals the `EXIF`-wrapped read for every wrapped fixture, live and by record.
- [x] **Divergences pinned, each with a `questions.yaml` id, each required to still exist:** at least `webp-iccp-profile-not-validated` (garbage, stub), `webp-xmp-content-not-parsed` (garbage and 1 to 7 byte `XMP\0`), `webp-riff-other-forms-read-by-exiftool` (`RIFF....WAVE`, `RIFX`), plus whatever else the grid measures. State which side is right.
- [x] **Warning table pinned:** our `WebpIssue` kinds beside ExifTool's warnings for: prefix, truncated tail (trailing bytes, length past EOF, partial header), missing pad, `XMP\0`; and where ExifTool is silent (`RiffSizeMismatch`, `ChunkFlagMismatch`, `MisplacedVp8x`, `BadVp8x`) or louder (ICC).
- [x] **Replay-and-perturb red-proof:** changing one field of one committed record turns the comparison red naming exactly that selector; control first.
- [x] **Mutated-copy red-proofs** (control first, `RED-PROOF INJECTION`, needle exactly once, marker must not leak into `src/` or `tests/support/`): at least seven library faults, each naming the expected selectors and no others: the `EXIF` TIFF offset wrong by one; the first block wins instead of the last (visible on the two-block fixtures); the `Exif\0\0` prefix not skipped; the declared RIFF size limits the walk (visible on `webp-riff-size-0` and `-small`); the pad rule removed; `XMP\0` not accepted; `ICCP` not located; four-character codes compared case-insensitively. Plus two byte mutations of a fixture (a corrupted offset inside the `EXIF` payload; truncation between `VP8L` and `EXIF`), each also run live when ExifTool exists.
- [x] Any reader bug found is fixed here with a regression test; every divergence is in `guidance/questions.yaml`.
- [x] `just lint`, `just test`, `just mutation-oracle`, `just validate`, `just cost-audit`, `just forbid-unsafe` green; `just oracle-record` idempotent (twice, no diff); no `#[allow]` in `src/`; a 60 s `just fuzz-smoke webp_riff` after the seed additions.

## Failing Tests

Written first: the fixture list and comparator tests exist and are seen red for the right reason (no records) before `just oracle-record` produces them.

- **`tests/webp_oracle.rs`:** `webp_fixtures_are_deterministic_and_uniquely_named`; `every_wrapped_tiff_fixture_is_present_and_matches_its_tiff_bytes`; `replay_webp_records_are_clean_on_an_unmodified_reading`; `replay_webp_headers_record_command_version_and_stderr`; `the_webp_record_set_is_exactly_the_fixtures_and_the_seeds`; `oracle_names_the_one_field_perturbed_in_a_webp_record`; `container_independence_every_wrapped_fixture_reads_the_same_bare_and_in_webp`; `several_exif_model_is_per_selector_and_pinned`; `region_presence_xmp_and_icc_agree_with_exiftool`; `empty_regions_never_count_as_present`; `chunk_code_case_rules_agree`; `declared_size_never_limits_the_walk_agrees`; `padding_rules_agree`; `vp8x_shapes_agree`; `fatal_headers_are_recorded_divergences_where_exiftool_reads_another_format`; `located_region_bytes_equal_what_the_builders_wrote`; `warning_table_pins_our_issue_kinds_beside_exiftools_warnings`; `every_known_divergence_has_a_question_entry`; `a_divergence_that_stops_being_one_fails`; `live_webp_parity_on_fixtures`; `live_webp_parity_on_the_webp_seeds`; `live_webp_skip_is_loud_and_required_mode_fails`.
- **`tests/webp_mutation_oracle.rs`:** `controls_are_green`, one `mutant_...` per fault, `byte_mutation_...` for the two corruptions, `injection_marker_does_not_leak`.
- **Carried:** `first_vp8x_wins_flags_and_one_flag_mismatch`, `limits_are_pinned_literals`, a test running the fuzz model on every walk seed.

## Non-Goals

- PNG parity (done, SPEC-031); audit wiring (SPEC-032); decoding or validating XMP or ICC content in the library; changes to the JPEG, TIFF or PNG oracle behaviour beyond additive entries.

## Notes for the Implementer

- **Checkpoints.** (0) carried gaps: tests, seeds, pins, gates, COMMIT. (1) fixtures, `oracle_webp.rs`, `read_ours_webp`, recorder, records, replay and live tests, divergences, questions, warning table: gates, COMMIT. (2) the mutation oracle: gates, COMMIT. If past about 60 percent of budget at checkpoint 1, hand back after it.
- Measure before modelling: write the several-block model and the divergence entries from the records, not from this text.
- The ICC profile for the valid fixtures must have a tag table with at least one entry (a 0-entry table reads nothing); reuse `oracle_png::icc_profile`.
- `Make` needs a TIFF that differs between blocks: build two small TIFFs in the builder (Make only, and Make plus Model) for the two-block fixtures.
- Carried SPEC-032 notes (do not act on them here): an empty region is not metadata; `Icc` and `Xmp` regions can be unreadable to ExifTool (validation), so the audit must say "located" not "valid".
- Budget: about 100 tool calls or 250k context.

---

## Reflection

1. **What would I do differently next time?**
   — Measure every selector, not two, before writing a model: the design measured only Make and Model for several EXIF blocks and "later wins" was wrong for four of the 20 selectors (the verify's one High). Also measure each cap and size limit against ExifTool in the grid (the 17th-block case was missing). The variant grid did cut verify to one round (PNG took three); the fix round was a fresh narrow agent (179,076 tokens).

2. **Does any template, constraint, or decision need updating?**
   — No DEC change. The fix round was accepted by the orchestrator without a second cold verify: it changed only tests, fixtures, records and the model (no `src/`), each finding re-measured against ExifTool, gates green. The same per-selector gap probably exists in the JPEG oracle (`read_ours_jpeg`, later-wins for all 20): evidence in HANDOFF-066; it needs its own small spec.

3. **Is there a follow-up spec I should write now before I forget?**
   — Yes: fix the JPEG oracle's several-APP1 model (first block wins ImageWidth, ImageHeight, Orientation, XResolution). SPEC-032 carries: the audit must walk all `exif_blocks()`, never `exif()` alone (`exif()` is the first block only), apply the per-selector priority, and report blocks past the 16-block cap (decoy blocks can hide GPS); located regions can be invalid to ExifTool.

4. **Where was the worst defect caught?** — verify

5. **What can a user do now that they couldn't before?** — none: test evidence only; WebP reads are now checked against ExifTool on 98 fixtures and 53 seeds, with six divergences pinned and ten mutants showing the comparison bites.
