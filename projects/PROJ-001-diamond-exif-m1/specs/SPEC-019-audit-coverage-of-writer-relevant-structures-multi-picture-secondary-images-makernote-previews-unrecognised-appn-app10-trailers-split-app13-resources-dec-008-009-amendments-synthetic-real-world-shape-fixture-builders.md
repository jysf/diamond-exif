---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-019
  type: story                      # epic | story | task | bug | chore
  cycle: design                    # frame | design | build | verify | ship
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

project:
  id: PROJ-001
  stage: STAGE-004
repo:
  id: diamond-exif

handoff:
  from_agent: claude-opus-5-5  # from .repo-context tier_map.design (DEC-005)
  to_agent: null                   # filled when HANDOFF is created (any agent — see docs/porting.md)
  created_at: null

references:
  decisions: [DEC-006, DEC-008, DEC-009, DEC-010, DEC-012]
  constraints: [panic-free-library, forbid-unsafe-code, no-io-in-library, fuzz-target-per-surface, test-before-implementation, oracle-must-be-shown-red, no-new-top-level-deps-without-decision]
  related_specs: [SPEC-012, SPEC-014, SPEC-015, SPEC-020]                # [SPEC-NNN]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: [SPEC-018]                     # e.g. [SPEC-002]

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-004's <capability>". Optional; null is acceptable.
value_link: "STAGE-004: the audit names everything a strip must handle, so the preview is true before any writer exists"

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
  tokens_estimate: 800000
  sessions: []
  totals:
    tokens_total: 0
    estimated_usd: 0
    session_count: 0
---

# SPEC-019: Audit coverage of writer-relevant structures: Multi-Picture secondary images, MakerNote previews, unrecognised APPn, trailers, split APP13; DEC-008/009 amendments; synthetic real-world-shape fixtures

## Context

SPIKE-001 audited real camera and phone files and found data a strip must deal with that the audit does not report: Multi-Picture Format (MPF) secondary images (4 of 11 JPEGs; the Nikon extras carry their own Exif, the iPhone extras XMP and ICC), proprietary APP10 `AROT` segments (2 of 11), a non-Exif trailer after the last EOI (1 of 11), `COM` comments, FlashPix `FPXR` APP2 segments, and Photoshop resources split across two APP13 segments (0 IPTC regions found where ExifTool lists 2). Until the audit sees these, DEC-009's preview can claim a policy leaves "nothing" while data remains, and the writer stage (SPEC-020 to SPEC-022) would be built and checked against an incomplete picture.

DEC-012 (written in this design, normative; read it first) decides what the JPEG layer now locates (`EmbeddedImage`, `Comment`, `OtherApp`, `Trailer`, and every Photoshop APP13 block as `Iptc`), the benign list (APP0 `JFIF\0`, APP14 `Adobe\0`), two new categories (`EmbeddedImage` and `OtherSegment`, both Medium and both removed only by `Everything`), the `holds` summary for embedded images, the DEC-009 grid rows and may-hold rows, and the report change (`schema_version` 2). This spec implements that and adds **synthetic builders for every real-world shape**, because real files cannot be committed. MakerNote previews are covered by `may_hold(MakerNote)` and are not located (no MakerNote parsing).

## Goal

The JPEG layer locates the four new region kinds and every Photoshop block; the audit turns them into items and findings with the two new categories, reports `holds` for each embedded image; the presets, preview, renderers, CLI, fuzz harnesses and ExifTool oracle are updated to match DEC-012; and committed synthetic fixtures reproduce each shape SPIKE-001 found, including a Canon-style absolute-offset MakerNote and an Exif block at the 64 KB limit, so the writer specs have realistic inputs.

## Inputs

- **Reads:** DEC-012 (normative), DEC-006, DEC-008, DEC-009, DEC-010 and their amendments, SPIKE-001 (`spikes/done/`), `src/jpeg/{mod,metadata}.rs`, `src/audit/{mod,category,preview}.rs`, `src/policy.rs`, `src/report/*`, `src/bin/exif-audit/*` (usage text and tests), `tests/support/{oracle_jpeg,report_inputs,inventory,inventory_corpus,probe,hostile}.rs`, `tests/{jpeg_oracle,inventory_oracle,audit_model,policy_preview,report_render,report_formats,cli,fuzz_layout}.rs`, `tests/golden/*`, `fuzz/shared/{jpeg_metadata_invariants,audit_invariants,render_invariants,cli_invariants}.rs`, `fuzz/fuzz_targets/*`, `fuzz/seeds/**`, `examples/fuzz_seeds.rs`.
- **Does not read:** ExifTool source (run, never read). ExifTool 13.55 is run for the parity checks below.
- Constraints `panic-free-library` (five-lint deny set), `forbid-unsafe-code`, `no-io-in-library`, `fuzz-target-per-surface` (existing targets extended; no new input surface), `test-before-implementation`, `oracle-must-be-shown-red`, `no-new-top-level-deps-without-decision`. No dependencies.

## Outputs

### Library

- `src/jpeg`: `Region` (already `#[non_exhaustive]`) gains the variants for an embedded image (segment index of the MPF segment, offset and length in the input, the bytes), a comment, an unrecognised APPn (segment, marker), and a trailer (offset, bytes); `Region::segment()` and `Region::data()` stay total (a trailer's segment is the index one past the last segment). The Photoshop rule of DEC-012 section 1 replaces the `0x0404`-resource rule for the `Iptc` region; the unparsed-resource issues stay. New typed JPEG-layer issues (names are the builder's) for: a malformed MPF segment, an MPF entry outside the input or not starting `FF D8`, more than 16 MPF images. MPF parsing uses the existing tolerant TIFF engine over the MPF payload; every range check uses `checked_*`; nothing panics on any bytes.
- `src/audit`: `RegionKind` gains `EmbeddedImage`, `Comment`, `OtherApp`, `Trailer`; `Category` gains `EmbeddedImage` and `OtherSegment` (names `embedded_image`, `other_segment`; severity Medium; `Category::ALL` is 15 entries in a fixed order: the two new ones last); `is_block` is true for both; `region_category` maps as in DEC-012 section 4; a finding for an `EmbeddedImage` region carries an `embedded` summary (offset, length, `starts_with_ffd8`, `holds`: a `StripSpec` of the categories the inner JPEG audit found, computed with embedded-image location switched off). `audit()` stays total, bounded by the existing caps; the inner audits are bounded by the 16-image limit and run once each.
- `src/policy.rs`: `may_hold` gains the two rows (every category except itself) and `EmbeddedImage` joins `may_hold(MakerNote)`; the five preset columns gain the two rows (removed only by `Everything`); the bit set (`u16`) has 15 of 16 bits used (a test pins the count).
- `src/report/*`: `schema_version` 2 and the DEC-012 section 7 changes (finding key `embedded`, 15-key `by_category`, new enumeration values, text prints `holds`); CSV columns unchanged; `safe_text` untouched. Goldens are regenerated **deliberately** and every changed line is reviewed (report in the Handback).
- `src/bin/exif-audit`: `--help` category/region wording only if it lists them; behaviour and exit codes unchanged (the new findings are Medium; the default `--fail-on high` is unaffected).

### Rules (normative; each maps to a named test)

1. **MPF.** A JPEG with an APP2 `MPF\0` segment whose `0xB002` index lists N images yields N-1 `EmbeddedImage` regions (entry 0 is the primary), each at `mpf_header_start + offset` for `size` bytes, in index order; an entry whose range is outside the input or whose bytes do not start `FF D8` is an issue and no region; the 17th and later entries are an issue; an MPF segment that does not parse is an `OtherApp` region plus an issue. Big- and little-endian MPF headers both work.
2. **Trailer.** The `Trailer` region is `Jpeg::trailer()` minus the located embedded images' ranges; a file whose trailer is exactly its MPF images has no `Trailer` region; a plain 313-byte trailer is one region; a zero-length remainder is no region; a trailer partly covered by an embedded image yields the uncovered remainder (possibly two pieces: one region per contiguous remainder, at most 16).
3. **Comment, OtherApp, benign list.** Each `COM` segment is a `Comment` region (finding `FreeText`); each APPn not Exif, XMP, extended XMP, ICC, Photoshop, JUMBF, parsed MPF, `JFIF\0` or `Adobe\0` is an `OtherApp` region (finding `OtherSegment`); APP0 `JFXX`, APP2 `FPXR`, APP10 `AROT` are `OtherApp`; APP0 `JFIF\0` and APP14 `Adobe\0` produce nothing.
4. **Photoshop.** Two APP13 `Photoshop 3.0\0` segments give two `Iptc` regions; an APP13 segment without that header is `OtherApp`; the SPIKE-001 split file gives 2 `Iptc` regions (and still reports the issues it did).
5. **holds.** The `holds` of an embedded image is exactly the set of finding categories of the ordinary audit of those bytes with embedded-image location off; an inner MPF segment is therefore an `OtherSegment` there and does not recurse; an inner non-JPEG range is not a region at all.
6. **Preview.** The DEC-012 section 5 rows hold for all five presets: `EmbeddedImage` and `OtherSegment` are `Removed` only under `Everything` and `KeptUnread` under the other four; `Comment` findings (FreeText) are `Removed` under `AllPii`, `All`, `Everything` and `Kept` under `Gps`, `GpsAndDevice`; `may_hold(MakerNote)` contains `EmbeddedImage`, so a kept MakerNote stays `KeptUnread` under `Gps` and `GpsAndDevice` as before and is unchanged under the others.
7. **Container independence and ordering.** Items stay in container order with the new regions in segment order, trailer regions last; a bare TIFF has none of them; `Health::clean` is false when the new issues occur.
8. **Totality and bounds.** No input panics; a hostile MPF (cyclic offsets, 2^32 sizes, 65,535 images declared, offsets into the MPF segment itself, overlapping images) yields bounded work and typed issues; the work is linear in the input and the entry budget.
9. **Names are API.** The two category names, the four region names, `schema_version` 2 and the `embedded` object's keys are pinned by tests (the renderers print them).

### Fixtures (new `tests/support/real_shapes.rs`, test code only)

Builders (documented, deterministic, no ExifTool dependence) for: `mpo(primary, [secondary...])` with big- and little-endian MPF (secondaries with their own Exif, XMP and ICC); a truncated MPF index; MPF offsets out of range; an MPF whose image is not a JPEG; an inner MPF; 17 images; `split_app13()` (two Photoshop segments, the first long enough that a resource crosses the boundary); an APP10 `AROT`-style segment; an APP2 `FPXR` segment; a `COM` segment; a 313-byte trailer; **`canon_style_absolute_note()`**: an Exif block with `Make=Canon` and a MakerNote IFD whose entries use offsets absolute to the TIFF blob, so that ExifTool reads Canon tags from it and moving the note inside the block changes ExifTool's view (the builder proves this with a test, reproducing the SPIKE-001 relocation result synthetically); **`exif_at_64k()`**: an Exif APP1 of exactly 65,533 payload bytes (the maximum). Each builder has a test showing ExifTool reads it as intended (live, replayed from a committed record in the usual way; skipped loudly without the tool).

### Oracle

The SPEC-013 comparator is extended, not loosened: region kinds with an ExifTool group are compared as before; for the new kinds the relation is measured and pinned. The number of `EmbeddedImage` regions equals the number of ExifTool `MPImage<N>` groups on the MPO fixtures (the builder measures whether ExifTool counts the primary and records the offset in the comparator); `Comment`, `OtherApp` and `Trailer` have no reliable ExifTool counterpart, so on the **existing 178 inputs** they are excluded from the multiset on both sides (class S: audit-only region kinds; the exclusion is a named constant, an exact list, with a test that no existing input's pinned entry changed) and on the **new fixtures** they are asserted directly by unit tests. Records: new files under `tests/oracle-fixtures/exiftool-inventory/` for the new fixtures (command, version, `# stderr:`), `just oracle-record` reproduces them with a clean `git status`. Red-proof (control first): drop an expected embedded image, add a phantom one, miscount the primary, and show each named.

### Fuzz and seeds

No new target. The harnesses are extended with the new regions and categories, each re-derived **independently** in the harness (a second naive MPF parser, the benign list, the Photoshop rule, the `holds` computation), `iff` in both directions, guards run alone, plants in both directions, step counter: `fuzz/shared/jpeg_metadata_invariants.rs` (regions), `fuzz/shared/audit_invariants.rs` (items, findings, categories, `embedded`), `fuzz/shared/render_invariants.rs` (15 categories, region names, schema 2, `embedded` object). New seeds (at least 12) in `fuzz/seeds/jpeg_segments` and `fuzz/seeds/audit` for: an MPO with two images, truncated MPF index, MPF offset out of range, inner MPF, 17-image MPF, split APP13, APP10 `AROT`, FPXR, COM, a 313-byte trailer, the Canon-style note, the 64 KB Exif; and in `fuzz/seeds/audit_render` the MPO and the split-APP13 inputs under several option bytes. Seed counts and hash lists in `tests/audit_model.rs`, `tests/fuzz_layout.rs` and `tests/jpeg_oracle.rs` are updated in the same change and **no existing seed or record changes bytes**. A plant of a new-region guard is shown found by the real libFuzzer target.

## Acceptance Criteria

- DEC-012's table is implemented as written: every rule 1 to 9 holds; the hand-typed grids in the tests (removed grid 5 x 15, kept-block grid, may-hold table) equal DEC-012 and DEC-009 as amended, cell by cell.
- The SPIKE-001 shapes are reproduced synthetically and audited as described; the Canon-style note shows the relocation hazard under ExifTool; the 64 KB fixture is exactly the maximum Exif segment.
- The JSON schema is version 2; goldens regenerated and reviewed; `exif-audit` on the new fixtures prints the new regions in text, JSON and CSV that the strict checkers accept; the CLI tests and usage golden are updated only where the DEC requires.
- The oracle: MPImage-count parity on the MPO fixtures, class S exclusion exact, new records reproduce, red-proofs shown; the existing 178 inventory results unchanged.
- Every mutant below is applied by hand, shown to fail at least one named test or guard (the report names which), and reverted; the tree after reverting is byte-identical to the build commit.
- Re-run by hand against the local corpus (`/Users/jyashinsky/PSeven/experiments/crustimg_redo_plus/_incoming0`, tag values never printed, nothing copied): the MPF files report their embedded images with `holds` that match ExifTool's view of the extracted images; the APP10 files report `OtherSegment`; the Leica trailer reports `OtherSegment`; the split-APP13 sample (from the Exiv2 test data used in SPIKE-001, if still available in the scratchpad) reports 2 `Iptc` regions. The numbers go in the Handback; this is a log, not a test.
- Gates: fmt (both graphs and the harness files), `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `cargo test` on stable and 1.90.0 with `DIAMOND_EXIF_REQUIRE_ORACLE=1 DIAMOND_EXIF_REQUIRE_CLI_FS=1`, `-F unsafe_code` on lib and bin, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `oracle-record` with a clean `git status`, 60 s `fuzz-smoke` of `jpeg_segments`, `audit`, `audit_render`. No dependencies; `decisions/` and `scripts/` untouched.

## Failing Tests

Named tests (a rename, fold or missing test is a deviation to report):

**src/jpeg (unit):**
- `mpf_index_yields_one_embedded_image_per_secondary_entry_in_order`, `mpf_big_and_little_endian_headers`, `mpf_entry_outside_the_input_is_an_issue_and_no_region`, `mpf_entry_not_starting_ffd8_is_an_issue_and_no_region`, `mpf_seventeenth_image_is_an_issue`, `unparsable_mpf_segment_is_other_app_plus_an_issue`, `hostile_mpf_is_bounded_and_total` (cyclic, 2^32 sizes, 65,535 declared, offsets into the MPF segment, overlaps)
- `trailer_minus_embedded_images_is_the_remainder`, `a_trailer_that_is_exactly_the_mpf_images_gives_no_trailer_region`, `a_plain_trailer_is_one_region`, `a_partly_covered_trailer_gives_the_uncovered_pieces`
- `com_segment_is_a_comment_region`, `unrecognised_app_segments_are_other_app`, `jfif_and_adobe_app14_are_benign`, `jfxx_fpxr_and_arot_are_other_app`
- `every_photoshop_app13_is_an_iptc_region`, `app13_without_the_photoshop_header_is_other_app`, `split_app13_gives_two_iptc_regions_and_keeps_its_issues`

**src/audit (unit):**
- `new_region_kinds_map_to_the_dec_012_categories`, `category_all_has_fifteen_entries_and_the_two_new_names_are_pinned`, `new_categories_are_medium_and_blocks`, `a_comment_is_a_free_text_finding`
- `embedded_image_finding_carries_range_ffd8_flag_and_holds`, `holds_is_the_inner_audit_with_embedded_location_off`, `an_inner_mpf_does_not_recurse`, `holds_of_an_image_with_no_metadata_is_empty`
- `container_independence_still_holds_for_every_oracle_fixture`, `audit_is_total_on_every_new_shape`

**src/policy.rs and src/audit/preview.rs (unit):**
- `each_preset_spec_equals_its_dec_009_column_as_amended` (hand-typed 5 x 15 grid), `may_hold_equals_the_dec_012_table`, `makernote_may_hold_embedded_image`, `bit_set_uses_fifteen_of_sixteen_bits`
- `embedded_image_and_other_segment_are_removed_only_by_everything`, `embedded_image_and_other_segment_are_kept_unread_under_the_other_four`, `comment_findings_follow_free_text`

**tests/real_shapes.rs (new; integration):**
- `every_builder_is_deterministic_and_parses`, `the_mpo_fixture_audits_with_holds_matching_its_inner_images`, `the_split_app13_fixture_gives_two_iptc_regions`, `the_arot_fpxr_com_and_trailer_fixtures_audit_as_other_segment_and_free_text`
- `canon_style_note_is_read_by_exiftool_and_moving_it_changes_exiftools_view` (live and replay; the relocation hazard reproduced)
- `exif_at_64k_is_the_maximum_segment_and_parses`

**tests/inventory_oracle.rs and tests/jpeg_oracle.rs:**
- `mpimage_group_count_equals_embedded_image_regions_on_the_mpo_fixtures` (live and replay), `class_s_region_kinds_are_excluded_exactly_and_the_existing_pins_are_unchanged`, `new_fixture_records_replay_and_reproduce`, `comparator_is_red_when_an_embedded_image_is_dropped_added_or_the_primary_miscounted` (control first)

**tests/report_render.rs, report_formats.rs, cli.rs, fuzz_layout.rs and the harnesses:**
- `schema_version_is_two_and_the_embedded_key_is_pinned`, `by_category_has_fifteen_keys_in_order`, `new_region_and_category_names_are_pinned_in_every_format`, `golden_files_match_after_deliberate_regeneration`, `text_prints_holds`
- `jpeg_metadata_harness_holds_on_every_seed_and_degenerate_inputs`, `audit_harness_holds_...` (extended), `each_new_region_guard_goes_red_alone_when_planted` (both directions, at least two plants per guard), `new_seeds_are_committed_and_have_the_shapes_they_claim`, `existing_seeds_and_records_are_byte_identical`

## Mutants (each shown red by hand, then reverted)

1. MPF offset taken relative to the file start instead of the MPF header. 2. Entry 0 (the primary) emitted as a region. 3. The size field ignored (region runs to EOF). 4. Big-endian MPF read as little-endian. 5. No FFD8 check. 6. No range check against the input. 7. The 16-image bound off by one. 8. The trailer not reduced by the embedded ranges. 9. COM not mapped to FreeText. 10. APP14 Adobe flagged as OtherApp. 11. JFIF flagged as OtherApp. 12. FPXR treated as benign. 13. An APP13 without the Photoshop header counted as Iptc. 14. Only the first APP13 segment counted. 15. `holds` computed with embedded-image location on (nested recursion). 16. `holds` always empty. 17. `EmbeddedImage` removed by `All`. 18. `OtherSegment` removed by `AllPii`. 19. `may_hold(MakerNote)` loses `EmbeddedImage`. 20. `may_hold(OtherSegment)` omits one tag category. 21. The new categories not blocks (`is_block` false). 22. Category order changed (new entries first). 23. `schema_version` left at 1. 24. `embedded` key placed before `thumbnail`. 25. `by_category` drops `other_segment`. 26. A harness guard compares embedded counts only (one direction). 27. The class S exclusion applied to `Iptc` too. 28. MPF recursion inside an embedded image allowed.

## Non-Goals

- Parsing MakerNotes or locating previews inside them; decoding any embedded image's metadata beyond the category set (`holds`); listing inner items in the outer inventory
- Writing, stripping or changing any byte (SPEC-020 to SPEC-022)
- PNG, WebP, HEIC; a new fuzz target; new CLI options; changing exit codes or the default `--fail-on`

## Notes for the Implementer

- Order of work, test-first: the fixture builders and their ExifTool-reads-it tests (measure how ExifTool counts the MPF primary before writing the rule); the JPEG-layer regions and issues; the audit categories, `holds` and `embedded`; policy grids and may-hold; renderers and goldens (regenerate once, review the diff line by line); oracle comparator and records; harnesses, seeds and plants; CLI tests; then the 28 mutants.
- Type the new grids, the benign list, the MPF layout and the DEC-012 names **from DEC-012 by hand** in the harnesses and tests; none may be imported from the code under test.
- `arithmetic_side_effects` forbids unchecked `+`, `*`, `<<`: MPF offsets are untrusted `u32` sums, use `checked_add` and `get(..)`; never index.
- The MPF segment's own bytes lie before the primary's EOI, while the images lie after it; keep the two straight (offsets are relative to the MPF header start, which is the byte after `MPF\0`).
- Real files are not fixtures: do not copy anything from `_incoming0` or the scratchpad corpus into the repo; the local re-run is a log of counts and category names only, never tag values.
- Run `lint-fuzz-red-proof` locally; use ratio-based timing assertions only; skips are loud (`DIAMOND_EXIF_REQUIRE_ORACLE=1`).
- Report in the Handback: every deviation, each mutant with the test or guard that caught it, how ExifTool counts the MPF primary, the golden diff summary, the local-corpus counts, fuzz smoke numbers, and any doubt about a DEC-012 rule (a doubt is a question for the architect; do not touch `decisions/`).
- Do not push or open a PR; do not advance cycles.

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
