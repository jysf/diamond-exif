---
# Maps to ContextCore handoff.* semantic conventions.
#
# ONE handoff per delegated CYCLE. With build and verify running on different
# agents you get TWO handoffs per spec (HANDOFF-N build, HANDOFF-M verify) —
# `handoff.cycle` is what distinguishes them.
#
# The `handback:` block below is the RETURN path and it is not optional: it is
# how cost gets into the spec without the orchestrator hand-counting anything.
# `just handback-sync SPEC-NNN` reads it and appends the cost session for you.
# Rationale + the full contract: docs/decisions/DEC-013-delegated-cost-handback.md

handoff:
  id: HANDOFF-037
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-09
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-019

project:
  id: PROJ-001
  stage: STAGE-004
repo:
  id: diamond-exif

# ── THE HANDBACK ────────────────────────────────────────────────────────────
# Filled in by the EXECUTING AGENT before it reports done. This is a required
# part of completing the handoff, not a courtesy.
#
# `tokens_total` is the one field the cost gate reads. Report the REAL number
# from your own interface:
#   Claude Code   → run `/cost`
#   API           → the `usage` object (input + output, summed)
#   another agent → whatever your harness reports as total tokens
# If your platform genuinely exposes NO token count, set tokens_total: null AND
# write why in `notes` — then set `cost.metering_source: none` in
# .repo-context.yaml so the gate stops asking. Do not invent a number.
handback:
  status: completed                # completed | blocked | rejected
  tokens_total: null               # REAL combined count — what cost-audit reads
  estimated_usd: null              # tokens_total × your rate, or your harness's number
  duration_minutes: null
  branch: worktree-agent-aecfa9c13e2f2cda7
  pr: null
  completed_at: 2026-11-09         # YYYY-MM-DD
  notes: "tokens_total null: the implementer cannot see its own meter; the orchestrator transcribes the metered figure."
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-037: Audit coverage of writer-relevant structures (SPEC-019)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-019` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

Read before starting:

- **Spec (rules 1 to 9, library changes, fixtures, oracle, fuzz and seeds, named tests and the 28 mutants are normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-019-audit-coverage-of-writer-relevant-structures-multi-picture-secondary-images-makernote-previews-unrecognised-appn-app10-trailers-split-app13-resources-dec-008-009-amendments-synthetic-real-world-shape-fixture-builders.md`
- **DEC-012 (normative for the new regions, the benign list, the two categories, the preset rows, `holds`, and schema_version 2; read it, do not edit it):** `decisions/DEC-012-structures-the-audit-names-for-the-writer.md`, with DEC-006, DEC-008, DEC-009, DEC-010 and their amendments.
- **Evidence:** `spikes/done/SPIKE-001-*.md` (what real files contained and how ExifTool saw them).
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe; macOS `sed -i ''`; in zsh never name a shell variable `path`)
- Code: `src/jpeg/{mod,metadata}.rs`, `src/audit/*`, `src/policy.rs`, `src/report/*`, `src/bin/exif-audit/*`, `tests/support/*`, the test files and harness files listed in the spec's Inputs, `tests/golden/*`, `fuzz/seeds/**`, `examples/fuzz_seeds.rs`.
- Prior art and lessons that cost follow-ups before: timing assertions are ratios, never absolute; a guard is only as strong as the seeds that reach it (add a seed for every new class); plants must vary in kind (length, content at the start, middle and end, order), not one dimension; type every expectation grid and name list from the DEC by hand, never from the code under test; record a measured ExifTool relation before writing the rule that depends on it (measure how ExifTool counts the MPF primary first); a skipped test is loud.

Constraints: `panic-free-library` (five-lint deny set: no unchecked `+`, `*`, `<<`; MPF offsets are untrusted `u32` sums: `checked_add`, `get(..)`, never index), `forbid-unsafe-code`, `no-io-in-library`, `fuzz-target-per-surface` (existing targets extended), `test-before-implementation`, `oracle-must-be-shown-red`, `no-new-top-level-deps-without-decision`.

## Expected Deliverables

- Everything in the spec's Outputs and every Acceptance Criterion; every named test under its name (a rename, fold or missing test is a deviation to report). New files include `tests/support/real_shapes.rs`, `tests/real_shapes.rs`, new oracle records under `tests/oracle-fixtures/exiftool-inventory/` and new seeds; edits across `src/jpeg`, `src/audit`, `src/policy.rs`, `src/report`, `src/bin/exif-audit` (wording only where needed), the oracle comparator and the three harness files. No existing seed or record changes bytes; the existing 178 inventory results are unchanged (class S exclusion is exact).
- Order: fixture builders and their ExifTool-reads-it tests first (and measure how ExifTool counts the MPF primary); the JPEG-layer regions and issues; audit categories, `holds`, `embedded`; policy grids and may-hold; renderers and goldens (regenerate once and review every changed line); oracle comparator, records and red-proofs; harnesses, seeds and plants; CLI tests; then the 28 mutants (control first, apply by hand, show the named failure with its assertion text, revert, `git diff` empty).
- The local corpus re-run (spec Acceptance Criteria): the folder `/Users/jyashinsky/PSeven/experiments/crustimg_redo_plus/_incoming0` holds the user's real photos. **Read it in place only; copy nothing into the repo; print and record only counts, file short hashes and category or region names; never tag values, never file contents, never a path of a file inside it in committed text.** Use the release `exif-audit` with `--json --policy none --inventory` (never `--values`). If a public sample set from SPIKE-001 is still present under the session scratchpad `public/` directory, include it; if not, say so.
- A doubt about a DEC-012 rule is a question for the architect: write it in the Handback and `guidance/questions.yaml`; do not edit `decisions/`.
- Run locally, because they are CI-only otherwise: `just lint-fuzz-red-proof`, `just lint-red-proof`, `just oracle-record` (empty `git status --porcelain` afterwards), `just fuzz-smoke jpeg_segments`, `audit`, `audit_render` (60 s each; record executions and coverage), and confirm one new-region guard plant is found by the real libFuzzer target.
- Commit on your worktree branch. **Do not push, do not open a PR, do not advance cycles.** First `git merge --ff-only claude/rust-exif-metadata-6351d5` (you need the commit containing HANDOFF-037); if refused, use `git merge` and say so.
- Do not modify irradiance, `scripts/`, `decisions/`. No dependencies. Scratch work goes in a scratchpad directory, not `/tmp`.
- You cannot see your own token count; set `tokens_total: null` with a note, the orchestrator transcribes the metered figure.

## Out of Scope

- Writing, stripping or changing any byte; MakerNote parsing or locating previews inside MakerNotes; PNG, WebP, HEIC; a new fuzz target; new CLI options or changed exit codes
- Pushing, PRs, cycle advancement, SPEC-020 to SPEC-022

## Return Criteria — how to hand back

**Completing this handoff means filling in the `handback:` front-matter block
above AND the `## Handback

### Execution notes

- **Branch / PR:** `worktree-agent-aecfa9c13e2f2cda7`, no PR (not pushed, per the brief). The brief commit was not a fast-forward of this worktree's branch, so I used `git merge claude/rust-exif-metadata-6351d5` (a merge commit).
- **Completed at:** 2026-11-09
- **All acceptance criteria met?** Yes, with the deviations below. Every named test exists under its name except where listed.

### What was built

- JPEG layer: `Region` gains `EmbeddedImage`, `Comment`, `OtherApp`, `Trailer`; every Photoshop APP13 segment is an `Iptc` region; four typed MPF issues (`MpfMalformed`, `MpfImageOutsideInput`, `MpfImageNotJpeg`, `TooManyMpfImages`); `read_with(data, embedded)` is crate-private (location off for the inner audit).
- Audit: two categories, four region kinds, `Finding.embedded` (`EmbeddedSummary` with `holds: StripSpec`), policy rows and may-hold rows, `schema_version` 2, text prints `holds`.
- Fixtures `tests/support/real_shapes.rs` (15 builders), `tests/real_shapes.rs`, 30 `shape-*` seeds (15 in `jpeg_segments`, 15 in `audit`) plus 7 `audit_render` seeds, new records `real-shapes.txt` and two Canon value records, harness extensions (independent second MPF parser, benign list, Photoshop rule, `holds`, byte-by-byte trailer), 21 new-region plants.

### How ExifTool counts the MPF primary

It counts it: one `MPImage<n>` group per index entry, `MPImage1` being the primary. So `groups == our EmbeddedImage regions + 1` (constant `MPIMAGE_PRIMARY_OFFSET = 1` in the comparator). For damaged files ExifTool still names every entry (offset-out-of-range and not-JPEG: 3 groups, we list 1; 17-entry file: 17 groups, we list 15; truncated index: 0 groups, 0 regions); those pairs are pinned.

### Golden diff summary (regenerated once, every line reviewed)

JSON: `schema_version` 1 to 2; `by_category` gains `embedded_image:0`, `other_segment:0`; every finding gains `"embedded":null` after `thumbnail`; the iptc region `len` 6 to 18. Text and CSV: only the iptc region `len` 6 to 18 (DEC-012: the region is the segment payload after the header, 12 bytes of 8BIM header plus 6 data). The full-house inputs are seeds and did not change.

### Mutants (28, each applied by hand through a script, reverted, `git diff` empty after each)

Control green first. Caught by: 1 `mpf_index_yields_one_embedded_image_per_secondary_entry_in_order`, `mpf_entry_outside_the_input_is_an_issue_and_no_region`, `hostile_mpf_is_bounded_and_total`; 2 `mpf_index_yields_one_embedded_image_per_secondary_entry_in_order` (entry 0 gives `MpfImageNotJpeg`/extra region), `mpf_seventeenth_image_is_an_issue`; 3 `mpf_index_yields_...`, `trailer_minus_embedded_images_is_the_remainder`; 4 `mpf_big_and_little_endian_headers`, harness extraction-issue iff; 5 `mpf_entry_not_starting_ffd8_is_an_issue_and_no_region`, harness; 6 `mpf_entry_outside_the_input_is_an_issue_and_no_region`, jpeg harness; 7 `mpf_seventeenth_image_is_an_issue`; 8 `a_trailer_that_is_exactly_the_mpf_images_gives_no_trailer_region`, `trailer_minus_...`; 9 `new_region_kinds_map_to_the_dec_012_categories`, `a_comment_is_a_free_text_finding`, `comment_findings_follow_free_text`; 10 `jfif_and_adobe_app14_are_benign`; 11 `jfif_and_adobe_app14_are_benign`, `the_arot_fpxr_com_and_trailer_...`; 12 `jfxx_fpxr_and_arot_are_other_app`; 13 `app13_without_the_photoshop_header_is_other_app`; 14 `every_photoshop_app13_is_an_iptc_region`, `split_app13_gives_two_iptc_regions_and_keeps_its_issues`; 15 `an_inner_mpf_does_not_recurse`, guard `findings` (real-shapes plants); 16 `holds_is_the_inner_audit_with_embedded_location_off`, `embedded_image_finding_carries_range_ffd8_flag_and_holds`; 17 and 18 `embedded_image_and_other_segment_are_removed_only_by_everything` / `..._kept_unread_under_the_other_four`, `each_preset_spec_equals_its_dec_009_column_as_amended`; 19 `makernote_may_hold_embedded_image`, `may_hold_equals_the_dec_012_table`; 20 `may_hold_equals_the_dec_012_table`; 21 `new_categories_are_medium_and_blocks`; 22 `category_all_has_fifteen_entries_and_the_two_new_names_are_pinned`, `by_category_has_fifteen_keys_in_order`; 23 `schema_version_is_two_and_the_embedded_key_is_pinned`, `json_has_the_pinned_key_order_at_every_level`; 24 same two; 25 `by_category_has_fifteen_keys_in_order`; 26 `each_new_region_guard_goes_red_alone_when_planted` (plants whose counts stay equal); 27 `class_s_region_kinds_are_excluded_exactly_and_the_existing_pins_are_unchanged`, `replay_every_corpus_is_clean_except_pinned_divergences`; 28 `an_inner_mpf_does_not_recurse`. Mutants 22 and 27 were first written so that they did not compile; they were rewritten to compile and then caught.

### Local corpus (read in place, nothing copied, counts and category names only)

`_incoming0`: 21 entries (13 JPEG, 5 TIFF-family incl. DNG read as TIFF, 3 unsupported). 4 JPEGs have an MPF index with 2 embedded images each (8 `embedded_image` findings); ExifTool shows 3 `MPImage` groups on each (regions + 1). `holds`: three Nikon-style files, both extras `[]` (their inner Exif holds only ExifImageWidth/Height, which ExifTool also shows: no DEC-008 category); one iPhone-style file, `[xmp]` and `[xmp, icc]` (matches ExifTool's groups on the extracted images). 2 files carry APP10 `AROT` and report `other_segment` (ExifTool shows the AROT group on both). The Leica file with the trailer reports 1 `trailer` region (`other_segment`). The two Nikon MPOs also report 2 trailer pieces each. Overall `other_segment` findings: 10 across 6 files. Public sets (scratchpad `public/A,B,C`, still present): the Exiv2 split-APP13 sample gives 2 `iptc` regions, status `issues` (ExifTool lists 2 IPTC datasets); no MPF or AROT there.

### Gates run

`cargo test --no-fail-fast` with `DIAMOND_EXIF_REQUIRE_ORACLE=1 DIAMOND_EXIF_REQUIRE_CLI_FS=1` on stable (1.99) and on `+1.90.0`: green. `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `deny`, `deny-fuzz`, `validate`, `decisions-audit` (0 errors), `cost-audit`, `cargo rustc -F unsafe_code` on lib and bin: green. `just oracle-record`: empty `git status --porcelain` afterwards.

### Fuzz

`just fuzz-smoke` 60 s each, all exit 0: `jpeg_segments` 166,704 runs, cov 1826, ft 8010, corpus 669; `audit` 54,242 runs, cov 2290, ft 8987, corpus 705; `audit_render` 34,726 runs, cov 5142, ft 16002, corpus 1018. Plant found by the real libFuzzer `audit` target: mutant 16 (holds always empty) in the library made it crash within 30 s from the committed seeds, guard `findings` ("audit invariants violated ... EmbeddedImage ... embedded"); reverted.

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:**
  - Existing results moved: DEC-012's Photoshop rule changes two existing inventory inputs (`fixtures/jpeg-iptc-without-0404`, seed `icc-iptc-jumbf`) and one jpeg_oracle comparison; pinned as new Class R rows (34 rows, was 33) and one `oracle::KNOWN_DIVERGENCES` entry. Test pins changed: row count 33 to 34, class R 14 to 15, `regions_agree_...` case moved, `jpeg_oracle` `iptc_without_0404_...` and `signatures_under_the_wrong_marker...`; mutation test m7 reworked (its needle no longer exists). The 5 old corpora keep 249 inputs / 180 distinct; the new fixtures are a separate corpus `real-shapes` (not in `CORPORA`), and the `shape-*` seeds are skipped by the three seed corpora and by the JPEG recorder/replay so no old record changed bytes.
  - Image limit: implemented as 16 images including the primary (15 regions) because the spec text cannot hold both "16 regions" and "17th entry is an issue" (question `dec-012-mpf-image-limit-reading`). `MAX_JPEG_METADATA_ISSUES` gains `+ MAX_MPF_IMAGES`.
  - Trailer pieces: no cap of 16 (a cap would drop bytes); the natural bound is regions + 1.
  - Only the first MPF segment is read as an index; a second one is `OtherApp` plus `MpfMalformed`.
  - `NumberOfImages` (0xB001) is ignored; the count is the whole 16-byte entries of 0xB002.
  - `embedded` is `Option<EmbeddedSummary>` on `Finding` (public field added); `holds` is a `StripSpec`.
  - The full-house seed inputs are unchanged (they are existing seeds), so they carry the 13 DEC-008 categories only; the two new categories are covered by the shape seeds and `real_shape_fixtures_match_dec_009_as_amended_for_every_preset`. Tests that required "every category" in the full house were narrowed accordingly.
  - Spec test names `container_independence_still_holds_for_every_oracle_fixture` and `audit_is_total_on_every_new_shape`: not added under those names. Container independence is the existing `container_independence_audit_of_every_oracle_fixture_bare_and_in_jpeg` (still green); totality is covered by `every_builder_is_deterministic_and_parses`, the harness runs on every shape seed and all prefixes of four shapes. `audit_harness_holds_...` kept its existing name (extended).
  - `existing_seeds_and_records_are_byte_identical` covers seeds (63 + 66 hash lists); the records are covered by the existing `existing_oracle_records_are_byte_identical_to_the_committed_set`.
  - The ExifTool Canon values test uses two `-T -n -s3` records written next to the inventory records (`real-shapes-<name>-values.txt`).
- **Follow-up work identified:** none beyond the four questions added to `guidance/questions.yaml` (`dec-012-mpf-image-limit-reading`, `dec-012-trailer-piece-bound`, `dec-012-photoshop-rule-moves-two-existing-inventory-results`, `dec-012-mpf-parse-edges`). Observation for the architect: both Nikon MPOs in the local corpus leave 2 trailer pieces after the located images (the index does not tile the file); worth a look when the writer decides what to do with trailers.

### Cost self-report

- **Tokens (total):** null (the implementer cannot see its meter)
- **Estimated USD:** null
- **Duration (minutes):** not measured
- **Source of the number:** none available; the orchestrator transcribes the metered figure.

### Reflection

1. **What was unclear in the spec or handoff that slowed you down?** — The 16 versus 15 image limit, and "existing 178 results unchanged" against the Photoshop rule that necessarily moves two.
2. **Was there a constraint or decision that should have been listed but wasn't?** — The record directory and seed-count pins in older tests (`budget_oracle`, `fuzz_layout`, `audit_model`, `policy_preview`) that any new seed touches.
3. **If you did this task again, what would you do differently?** — Decide the new-seed naming (`shape-` prefix, skipped by old corpora) first, and write the mutants as compiling edits from the start.
