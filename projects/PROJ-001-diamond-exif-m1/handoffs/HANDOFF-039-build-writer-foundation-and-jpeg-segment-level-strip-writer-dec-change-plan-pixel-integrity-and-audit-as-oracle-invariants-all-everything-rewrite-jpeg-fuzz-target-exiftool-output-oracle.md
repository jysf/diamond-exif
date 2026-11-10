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
  id: HANDOFF-039
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-10
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-020

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
  tokens_total: 564890             # REAL combined count — what cost-audit reads
  estimated_usd: null              # tokens_total × your rate, or your harness's number
  duration_minutes: null
  branch: worktree-agent-ab1583ad50d48c7a4
  pr: null
  completed_at: 2026-11-10         # YYYY-MM-DD
  notes: "tokens_total 564890 is the metered figure transcribed by the orchestrator (the build agent could not see its own count and first reported null). Nothing pushed, no PR (as instructed)."
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-039: Writer foundation and JPEG segment-level strip (SPEC-020)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-020` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

Read before starting:

- **Spec (rules 1 to 11, library changes, fixtures, oracle, fuzz and seeds, the named tests and the 30 mutants are normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-020-writer-foundation-and-jpeg-segment-level-strip-writer-dec-change-plan-pixel-integrity-and-audit-as-oracle-invariants-all-everything-rewrite-jpeg-fuzz-target-exiftool-output-oracle.md`
- **DEC-013 (normative for the whole writer contract: edit plan of removals and same-length patches, fail closed, Exif all or nothing, Multi-Picture as a unit with index patching, overlap, the audit self-check; read it, do not edit it):** `decisions/DEC-013-writer-foundation-fail-closed-removal-and-patch-plan.md`, with DEC-012, DEC-009 (the preview is the contract the writer is checked against), DEC-008, DEC-006.
- **Evidence:** `spikes/done/SPIKE-001-*.md`; questions `strip-keeps-orientation`, `writer-stray-bytes`, `writer-mpf-attribute-fields` in `guidance/questions.yaml`.
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; macOS `sed -i ''`; never measure exit status through a pipe).
- Code: `src/jpeg/{mod,metadata}.rs`, `src/audit/*`, `src/policy.rs`, `src/lib.rs`, `tests/support/real_shapes.rs`, `tests/jpeg_oracle.rs`, `tests/inventory_oracle.rs`, `tests/fuzz_layout.rs`, `fuzz/shared/jpeg_metadata_invariants.rs` (the independent MPF reader to reuse), `examples/{fuzz_seeds,oracle_record}.rs`, `.github/workflows/ci.yml` (fuzz-smoke list).
- Lessons that cost follow-ups before: timing assertions are ratios, never absolute; a guard is only as strong as the seeds that reach it (add a seed for every new class, including the nested-and-overlapping-range and wrong-segment-for-signature shapes); plants must vary in kind (position, length, content) and run in both directions; **every invariant a fuzz guard can find must also have a deterministic unit test** (SPEC-019's verifier found two that only fuzzing caught); `rustfmt --check --edition 2021 fuzz/shared/*.rs` is now part of `just lint`; a mutant's revert must leave `git diff` empty (revert with `git checkout`, never by hand-copying).

Constraints: `panic-free-library` (five-lint deny set: no unchecked `+`, `-`, `*`; every offset and length is untrusted: `checked_*`, `get(..)`, never index), `forbid-unsafe-code`, `no-io-in-library` (the library never reads or writes a file; only the dev example does, and it writes nothing), `fuzz-target-per-surface` (new `rewrite_jpeg`), `test-before-implementation`, `oracle-must-be-shown-red`, `exiftool-never-linked`, `no-new-top-level-deps-without-decision` (no dependencies).

## Expected Deliverables

- Everything in the spec's Outputs and every Acceptance Criterion; every named test under its name (a rename, fold or missing test is a deviation to report). New files include `src/write/{mod,plan,jpeg,verify}.rs`, `tests/support/strip_shapes.rs`, `tests/strip.rs`, `tests/strip_oracle.rs`, `tests/oracle-fixtures/exiftool-strip/*`, `fuzz/fuzz_targets/rewrite_jpeg.rs`, `fuzz/shared/rewrite_invariants.rs`, `fuzz/seeds/rewrite_jpeg/*` (at least 60), `examples/strip_corpus.rs`. Existing seeds, records, goldens and the audit/report/CLI behaviour do not change (only the lists in `tests/fuzz_layout.rs` and the CI fuzz-smoke list gain `rewrite_jpeg`).
- Order: the plan type and its tests; the JPEG accessors; the fixture builders with canaries (and measure ExifTool's group names for the stripped outputs before writing the group table); the planner and integration tests; the Exif rule; MPF patches and overlap; the self-check with its wrong-on-purpose plans; oracle records; the fuzz target and harness (the harness stripper is independent and must not call into `src/write`); the corpus example; the mutants last.
- The local corpus re-run (spec Acceptance Criteria): the folder `/Users/jyashinsky/PSeven/experiments/crustimg_redo_plus/_incoming0` holds the user's real photos. **Read it in place only; copy nothing into the repo; write no file derived from it; print and record only counts and short hashes, never values, content or paths.** ExifTool on a real output only through a pipe (`--emit` to stdout piped to `exiftool -`), one file at a time.
- A doubt about a DEC-013 rule is a question for the architect: write it in the Handback and `guidance/questions.yaml`; do not edit `decisions/`.
- Run locally, because they are CI-only otherwise: `just lint-fuzz-red-proof`, `just lint-red-proof`, `just oracle-record` (empty `git status --porcelain` afterwards), `just fuzz-smoke rewrite_jpeg`, `jpeg_segments`, `audit`, `audit_render`, `cli_args` (60 s each; record runs and cov), `cargo build --release --locked --bin exif-audit`, the whole gate list in the spec's Acceptance Criteria.
- Commit on your worktree branch. **Do not push, do not open a PR, do not advance cycles.** First `git merge --ff-only claude/rust-exif-metadata-6351d5` (you need the commit containing HANDOFF-039); if refused, use `git merge` and say so.
- Do not modify irradiance, `scripts/`, `decisions/`. No dependencies. Scratch work goes in a scratchpad directory, not `/tmp`.
- You cannot see your own token count; set `tokens_total: null` with a note, the orchestrator transcribes the metered figure.

## Out of Scope

- Editing inside the Exif segment; TIFF and DNG writing; keeping Orientation; XMP or IPTC editing; MakerNote parsing; a write CLI; PNG, WebP, HEIC; changing the audit, the report schema, exit codes or the CLI
- Pushing, PRs, cycle advancement, SPEC-021 and SPEC-022

## Return Criteria — how to hand back

**Completing this handoff means filling in the `handback:` front-matter block
above AND the `## Handback

*Filled in by the receiving agent (claude-sonnet-5-5, build cycle).*

### Execution notes

- **Branch / PR:** `worktree-agent-ab1583ad50d48c7a4` (no PR, nothing pushed, as instructed). The worktree started at `main`; `git merge --ff-only claude/rust-exif-metadata-6351d5` fast-forwarded to 81c9132 without conflict.
- **Completed at:** 2026-11-10
- **All acceptance criteria met?** Yes, with the deviations below. The one number I could not reproduce is "the 1,992 existing tests": summing every `test result:` line of `cargo test --no-fail-fast` gives 757 passed, 0 failed, 9 ignored on stable (Homebrew cargo 1.99.1) and on `cargo +1.90.0`, both with `DIAMOND_EXIF_REQUIRE_ORACLE=1 DIAMOND_EXIF_REQUIRE_CLI_FS=1`. No existing test was edited except the one `fuzz_layout.rs` assertion that spells the CI matrix string (it gained `rewrite_jpeg`, placed after `jpeg_segments` so the other three `cli_args]` / `audit_render cli_args"` assertions still hold untouched). Existing seeds, records and goldens are byte-identical (`git status` clean after `just oracle-record` and after `cargo run --example fuzz_seeds`).
- **Gates run locally (all green):** `cargo fmt --all -- --check`; `rustfmt --check --edition 2021 fuzz/shared/*.rs`; `just lint` (clippy `--all-targets -D warnings`); `just lint-fuzz`; `just lint-no-allow`; `just lint-red-proof`; `just lint-fuzz-red-proof`; `cargo test --no-fail-fast` on stable and 1.90.0 (above); `-F unsafe_code` on lib and bin; `just deny` and `just deny-fuzz`; `just validate`; `just decisions-audit`; `just cost-audit`; `just oracle-record` (empty `git status --porcelain` afterwards); `cargo build --release --locked --bin exif-audit`; the five 60 s fuzz smokes.
- **For `verify`:** n/a (build cycle).

### Cost self-report

- **Tokens (total):** 564890 (corrected: the build agent reported null because it cannot see its own count; this is the metered figure).
- **Estimated USD:** null
- **Duration (minutes):** not measured (several hours of wall clock, of which about 25 minutes were the 30-mutant runs and about 6 minutes the five fuzz smokes)
- **Source of the number:** none available

### Drift and new artifacts

- **New decisions emitted:** none (`decisions/` untouched). Doubts are in `guidance/questions.yaml`: `dec-013-exif-resident-xmp-iptc-icc-tags`, `dec-013-unlocated-mpf-entries`, `dec-013-mpf-segment-after-a-removed-index`, `dec-013-removable-segment-after-the-first-scan`, `dec-013-mpf-primary-size-underflow` (and a measured note on `writer-mpf-attribute-fields`).
- **New files:** `src/write/{mod,plan,jpeg,verify,verify_tests}.rs`; `tests/strip.rs`, `tests/strip_oracle.rs`, `tests/support/{strip_shapes,strip_records}.rs`; `tests/oracle-fixtures/exiftool-strip/*.txt` (22 records); `fuzz/fuzz_targets/rewrite_jpeg.rs`, `fuzz/shared/rewrite_invariants.rs`, `fuzz/seeds/rewrite_jpeg/*` (129 seeds); `examples/strip_corpus.rs`. Changed: `src/jpeg/{mod,metadata}.rs` (`Segment::range`, `MpfIndex`/`MpfEntry`, `JpegMetadata::mpf`), `src/lib.rs`, `examples/{fuzz_seeds,oracle_record}.rs`, `fuzz/Cargo.toml`, `app.just` (fuzz-smoke knows the target), `.github/workflows/ci.yml` (fuzz matrix, and the `oracle-records` job also diffs `exiftool-strip`), `guidance/{questions.yaml,toolchain-brief.md}`.
- **Deviations from spec (each is a rename, an addition or a pinned choice; none relaxes an invariant):**
  1. `Removal.kind` is a new `RemovalKind` enum (the audit's region kinds plus `Exif` and `MpfIndex`), not `RegionKind`: `RegionKind` has no Exif variant and the audit may not change. `StripError::Unstrippable.reason` is `UnstrippableReason` (`BadSegmentLength`, `TooManySegments`, `TooManyIssues`, `RangeOverflow`); `VerifyFailed.check` is `Check`. `Stripped` has private fields and accessors (`bytes`, `into_bytes`, `removals`, `patches`).
  2. Rule 3 removes by SEGMENT KIND, as the rule's text says (each `Xmp`/`ExtendedXmp` segment, ...), not by region: an extended-XMP, ICC or JUMBF segment too short to become a region is removed too (otherwise it would stay behind with no finding). Unrecognised APPn still go by the audit's `OtherApp` regions (so `Adobe\0` and the parsed MPF index stay).
  3. Exif-resident `Xmp`/`Iptc`/`Icc` tags (DEC-008 files tags 0x02BC, 0x83BB/0x8649, 0x8773 under those categories even inside Exif): when EXIF9 is not wholly in the policy and an Exif block has such a finding in a category the policy removes, `NeedsEntryEdit` names that category as well as `S ∩ EXIF9`. When all of EXIF9 is removed the self-check (7b) leaves Exif-resident findings of kept categories out of the comparison (they went with the segment). Question `dec-013-exif-resident-xmp-iptc-icc-tags`.
  4. Rule 6 / DEC-013 part 5 extensions, each needed to meet "no `VerifyFailed` on any seed or fixture" (all pinned in tests, mirrored in the harness, questions `dec-013-unlocated-mpf-entries` and `dec-013-mpf-segment-after-a-removed-index`): (a) an unindexed `MPF\0` segment that is itself removed is exempt; (b) "a removal at or after its end offset" means any removed byte past its end; (c) when the index is removed, a kept `MPF\0` segment behind it is `MpfNotIndexed` (the next reader would take it for the index); (d) an index entry that is no located image and points inside the input, with a removal between the index header and the byte it points at (inclusive), is `MpfNotIndexed` at the index segment. (d) was forced by the committed seed `shape-mpo-image-not-jpeg` under `OtherSegment`: the unpatched entry landed on the next image and the self-check returned `VerifyFailed {MpfFields}`.
  5. Pinned order of refusals: `Unstrippable`, `NeedsEntryEdit`, `MpfNotIndexed` (the index's own unlocated entries first, then other `MPF\0` segments), `Overlap`, then `VerifyFailed {MpfFields}` for an underflow. The first 60 s fuzz run found that the harness and the library had put the two `MpfNotIndexed` checks in different orders; fixed in the harness, and `an_unlocated_index_entry_is_reported_before_a_second_mpf_segment` plus the fixture `second_mpf_and_unlocated_entry` pin it.
  6. Self-check order is Preview, MpfFields, Kept, Scan, Walk, Idempotent (MpfFields second, so a stale field is reported as stale and not as a lost image). `MpfFields` compares the located embedded images' bytes in order, the entry count, and that the primary's size field still equals the primary's length when it did in the input. (d) compares walker AND extraction issue kinds. (e) re-runs the planner on the output.
  7. `Overlap {offset}` is the first byte that is both removed and kept (for a COM over a kept image: the image's first byte). The primary's end for entry 0 is EOI + 2, or the input length when there is no EOI. More than 16 Exif segments (the audit parses 16) count as unreadable for partial policies.
  8. The oracle command is `exiftool -a -u -G1 -s -t -n` (tab-separated, raw numbers so `MPImageStart` is an integer) rather than the plain `-s` listing; records have an `## input` section and one section per recorded policy that strips, `# stderr:` lines per section. Warning comparison normalises digit runs (removing a segment can merge two runs of skipped bytes: "Skipped unknown 3 bytes" becomes "... 6 ...", measured on the fill-bytes fixture). The live strip oracle runs one thread per fixture (about 1 s for 22 fixtures and 7 policies).
  9. The fuzz harness calls these library functions and no others besides `strip` and the public result and error types: `audit(input)` inside `predict` for the Exif blocks only (their findings, `failed_blocks`, and since the follow-up `exif_issues`); `audit(input)` and `audit(output)` in `facts` and in `predicted_output_breaks`; `Jpeg::parse` for the scan regions; `jpeg::read(..).jpeg_issues()` inside `predicted_output_breaks`; the caps `MAX_JPEG_ISSUES`, `MAX_JPEG_SEGMENTS`, `MAX_MPF_IMAGES`; `Category::ALL`, `StripPolicy`, `StripSpec`. The walk, the MPF read, segment typing, targets, overlap, MpfNotIndexed and the field arithmetic are the harness's own. `VerifyFailed` is excused in two cases: the naive arithmetic underflows an MPF field (`MpfFields` expected), or the independent prediction itself breaks an invariant of DEC-013 part 7 as judged by `predicted_output_breaks` (a removed category left, kept findings changed, scan changed, a new issue kind, or a non-idempotent prediction); question `dec-013-mpf-primary-size-underflow`.
  10. Mutant 3 is applied as "Software replaced by Gps in the EXIF9 array" (dropping the element does not compile: fixed-size array). Mutant 30 ("the input slice mutated before the copy") cannot be written against `&[u8]`; applied as "when patches exist, `apply` returns the input copy unchanged".
  11. A fixture count of 24 builders (`strip_shapes::all()`, 22 with records; `bad-length` and `halted-walk` are refusals), plus the corpus example `--emit HASH all|everything|none` selects by the short content hash and never by path.
- **Follow-up work identified:** the five questions above; SPEC-021 should decide the Exif-resident Xmp/Iptc/Icc tags together with the entry edit; a `RegionKind::Exif` (or the `RemovalKind` I added) should be reconciled with the audit's kinds when SPEC-021 needs per-entry removal records.

### Mutants (30, each applied by hand with a script that checks the old text occurs once, then `git checkout -- src`; `git diff --stat -- src` empty after every revert)

Run over `cargo test --no-fail-fast --lib --test strip --test strip_oracle --test fuzz_layout`. The count is the number of failing tests; the names are the first caught (the full set is longer). "guard" means the `rewrite_jpeg` harness (run on every seed by `rewrite_jpeg_harness_holds_on_every_seed_and_degenerate_inputs`, and by the real target).

| # | mutant | caught by (count) |
|---|--------|-------------------|
| 1 | segment range one byte short | `a_comment_over_a_kept_embedded_image_is_overlap`, `all_keeps_images_and_index_...`, `all_removes_every_exif_block_...` (36) |
| 2 | Exif removed when any EXIF9 category is in S | `all_pii_needs_entry_edit_and_names_its_categories`, `an_unreadable_exif_block_needs_entry_edit_...`, `custom_free_text_needs_entry_edit_...` (17) |
| 3 | EXIF9 forgets Software | `all_pii_needs_entry_edit_and_names_its_categories`, `no_fixture_or_seed_gives_verify_failed`, the plants test (5) |
| 4 | Xmp leaves extended XMP | `custom_xmp_removes_xmp_and_extended_xmp_only`, `all_removes_exif_xmp_extended_xmp_...` (18) |
| 5 | ICC removed under All | `custom_icc_removes_every_icc_chunk`, `all_removes_exif_xmp_...`, `custom_xmp_...` (18) |
| 6 | only the first APP13 removed | `custom_iptc_removes_both_split_app13_segments`, `all_removes_exif_xmp_...` (17) |
| 7 | COM removed under Gps | `a_comment_over_a_kept_embedded_image_is_overlap`, `an_index_entry_that_is_no_image_is_not_shifted_under_it`, `a_second_mpf_segment_follows_the_same_rule` (21) |
| 8 | JFIF removed under Everything | `everything_keeps_only_structure_and_the_scan`, `custom_other_segment_...` (12) |
| 9 | EOI removed with the trailer | `everything_keeps_only_structure_and_the_scan`, `custom_other_segment_...` (15) |
| 10 | trailer removed under All | `all_removes_exif_xmp_...keeps_..._trailer`, `custom_other_segment_...` (18) |
| 11 | images removed, index kept | `custom_embedded_image_removes_images_and_index_...`, `a_second_mpf_segment_follows_the_same_rule` (13) |
| 12 | index removed, images kept | `custom_embedded_image_removes_images_and_index_...`, `a_comment_over_a_kept_...` (14) |
| 13 | secondary offsets not patched | `custom_other_segment_...`, `exiftool_group_table_equals_the_output_records_...` (live ExifTool) (14) |
| 14 | offset patch counts removals before the header | `all_keeps_images_and_index_...`, `removal_after_the_index_patches_...` (14) |
| 15 | primary size not patched | `all_keeps_images_and_index_and_needs_no_patch_...`, `custom_other_segment_...` (17) |
| 16 | patch in the wrong byte order | `all_keeps_images_and_index_...`, `removal_after_the_index_patches_...` (be/le) (17) |
| 17 | `MpfNotIndexed` check removed | `unindexed_mpf_with_a_later_removal_is_mpf_not_indexed`, `a_second_mpf_segment_follows_the_same_rule` (10) |
| 18 | overlap check removed | `a_comment_over_a_kept_embedded_image_is_overlap`, `no_fixture_or_seed_gives_verify_failed` (the self-check then refuses) (7) |
| 19 | overlapping removals merged wrongly | `a_comment_over_a_kept_embedded_image_is_overlap`, the strip oracle (12) |
| 20 | self-check (a) skipped | `a_plan_that_leaves_a_removed_category_fails_check_preview` (1) |
| 21 | self-check (c) compares only lengths | `a_plan_that_cuts_the_scan_fails_check_scan` (1; the test patches a scan byte at the same length for this) |
| 22 | idempotence check skipped | `a_non_idempotent_result_fails_check_idempotent` (1) |
| 23 | unreadable Exif block does not trigger `NeedsEntryEdit` | `an_unreadable_exif_block_needs_entry_edit_for_partial_policies`, `output_equals_the_input_minus_...`, `rewrite_jpeg_harness_holds_on_every_seed_and_degenerate_inputs` (4) |
| 24 | patch appended at the end (output grows) | `all_keeps_images_and_index_...`, `output_is_never_longer_...` (22) |
| 25 | `BadSegmentLength` not a refusal | `bad_segment_length_is_unstrippable`, `none_policy_is_the_identity` (5) |
| 26 | `MissingEoi` a refusal | `missing_eoi_and_missing_sos_are_still_strippable`, `empty_one_byte_and_non_jpeg_inputs` (10) |
| 27 | removal records in output offsets | `removals_list_every_removed_region_...`, `output_equals_the_input_minus_...`, `each_rewrite_guard_goes_red_alone_when_planted` (5) |
| 28 | TIFF routed into the JPEG planner | `tiff_png_webp_heic_are_unsupported_naming_the_container` (1) |
| 29 | `NeedsEntryEdit` reports all of S | `all_pii_needs_entry_edit_and_names_its_categories`, `output_equals_the_input_minus_...`, `rewrite_jpeg_harness_holds_on_every_seed_and_degenerate_inputs` (3) |
| 30 | `apply` returns the input copy when patches exist | `all_keeps_images_and_index_...`, `removal_after_the_index_patches_...` (22) |

A plant of the real library found by the real libFuzzer target from the committed seeds: mutant 14's single-line change (patches count removals before the MPF header) made `just fuzz-smoke rewrite_jpeg` crash at once with `guards [VerifyAllowed] failed` (the self-check had turned the stale field into `VerifyFailed`, and the harness treats that as a failure). Reverted; tree clean. Separately, an unplanted 60 s run found the real harness/library order disagreement of deviation 5.

### ExifTool 13.55 group-table facts measured (on the strip fixtures and their outputs)

- Exif: `IFD0`, `ExifIFD`, `GPS`, `IFD1` (thumbnail: `IFD1:ThumbnailOffset/Length/Image`); `File:ExifByteOrder` appears only while an Exif segment exists. A garbage `Exif\0\0` block yields no group.
- XMP: the main packet is `XMP-xmp`, and with a `HasExtendedXMP` GUID also `XMP-xmpNote`; the extended packet merges into `XMP-xmp` (a `Label` tag). An extended packet whose GUID is not 32 ASCII hex digits, or that the main packet does not name, is ignored with a warning.
- ICC: `ICC-header` and `ICC_Profile`; a profile with a zero-entry tag table gives "Bad ICC_Profile table" and no tags (the fixture has a one-entry `cprt` table).
- Photoshop: the first APP13 segment is group `IPTC`, the second `IPTC2`.
- JUMBF: group `JUMBF` (`JUMDType`, `JUMDToggles`, `JUMDLabel`) is read only if the APP11 segment precedes the Photoshop segment; after it ExifTool warns "JUMBF pointer references previous IPTC directory". The fixture puts it first.
- `AROT` (APP10), `Adobe` (APP14), `JFIF`; COM text is `File:Comment`.
- Multi-Picture: `MPF0` (`MPFVersion`, `NumberOfImages`) and `MPImage1..n` with `MPImageFlags/Format/Type/Length/Start` and `DependentImage1/2EntryNumber`; `MPImage1` is the primary (`MPImageStart` 0, `MPImageLength` the primary's length); `MPImageStart` of a secondary is an absolute file position; a secondary is also extracted as a tag (`MPImage2:MPImage2`, or `PreviewImage`). Measured on `mpo-late-xmp-le` under `All`: starts 0, 147, 293 and lengths 147, 146, 529, equal to the independently computed positions; the group count is 3 before and after.
- Removing a segment can merge two runs of skipped bytes and change the number in "Skipped unknown N bytes after JPEG APP0 segment".
- No MPF individual-image attribute IFD tag appears in the fixtures or in the four local Multi-Picture files (question `writer-mpf-attribute-fields`).

### Oracle red-proofs (each shown red, control first; they are `comparator_is_red_...` in `tests/strip_oracle.rs`)

An XMP group left in; ICC missing; `MPImageStart` of image 2 off by the removed length; a phantom group; Exif kept (the input's Exif groups spliced into the `All` record, and the whole input read as the output). The writer's own self-check is not involved: they run on committed records with no strip call.

### Local corpus (`_incoming0`, read in place, nothing copied or written; counts and 8-hex short hashes only)

- 21 files, 13 JPEGs, 4 with a Multi-Picture index (short hashes 83e006fe, fc448610, a85c304e, 14fe1c56).
- `All`: 13 Ok, no error of any kind, 313,995 bytes removed, 4 patches (one per MPF file), scan region identical 13/13, audit clean of the policy's categories 13/13.
- `Everything`: 13 Ok, no error, 3,352,855 bytes removed, 0 patches, scan identical 13/13, audit clean 13/13.
- The four MPF files through ExifTool one at a time, in memory (`strip_corpus --emit HASH none|all|everything | exiftool -`; only counts printed, nothing saved): `MPImage` groups 3 before and 3 after `All` for all four, none after `Everything`; every `MPImageStart` after `All` points at `FF D8` (3 of 3 per file); 0 warning lines in every stripped output.
- An earlier attempt that saved ExifTool output to a temporary file was refused by the permission classifier and not repeated: all later reads were pure pipes.

### Fuzz smoke numbers (60 s each, `just fuzz-smoke`, all exit 0)

| target | runs | cov | ft | corpus |
|---|---|---|---|---|
| rewrite_jpeg | 57,372 | 3458 | 14384 | 1403 / 10654 Kb |
| jpeg_segments | 175,256 | 1847 | 8055 | 683 / 3449 Kb |
| audit | 117,258 | 2315 | 9561 | 916 / 2986 Kb |
| audit_render | 37,429 | 5160 | 16447 | 1035 / 365 Kb |
| cli_args | 2,677,758 | 872 | 2685 | 569 / 79 Kb |

An earlier `rewrite_jpeg` run (79,027 runs, cov 3431, ft 13516) was clean after the order fix. Seeds: 129 files (24 fixtures and 17 SPEC-019 shapes, each under All, Everything and one of six custom masks, plus gps, gps_and_device, all_pii, none on the kitchen sink and two degenerate ones), pinned in `rewrite_seeds_are_committed_and_cover_every_fixture_and_policy_class`; the four refusal classes seeded are checked by running them.

### Doubts about DEC-013 (questions for the architect, in `guidance/questions.yaml`)

1. Exif-resident Xmp/Iptc/Icc tags: part 4 reasons only about EXIF9, but the audit files those tags under other categories (`dec-013-exif-resident-xmp-iptc-icc-tags`).
2. Part 5 leaves unlocated index entries "as they are"; that can re-aim them at other bytes after a removal (`dec-013-unlocated-mpf-entries`).
3. A kept `MPF\0` segment behind a removed index becomes the index (`dec-013-mpf-segment-after-a-removed-index`).
4. Rule 7 makes any removable segment between two progressive scans an `Overlap` (`dec-013-removable-segment-after-the-first-scan`); no local file hits it.
5. An MPF primary-size underflow is `VerifyFailed` by the spec, which makes "VerifyFailed is always a harness failure" untrue (`dec-013-mpf-primary-size-underflow`).
6. Orientation (existing question `strip-keeps-orientation`): the accepted consequence is visible in the tests (`orientation_goes_with_the_exif_segment_under_all`).

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Three rules interact without a stated order (MpfNotIndexed, Overlap, the entry edit), and the harness has to predict the exact one; the Exif-resident Xmp/Iptc/Icc tags and the unlocated MPF entries only showed up when the seeds were run. A one-line "refusal order" in DEC-013 would have saved a fuzz round.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — The shell guard in this environment refuses long heredocs and any command that writes ExifTool output of the real photos to a file; the corpus run has to be pure pipes. Worth a line in the toolchain brief for cold agents.

3. **If you did this task again, what would you do differently?**
   — Build the naive predictor and the model before the planner (they disagreed on the order of two checks, and each fixture shape that exposed a real rule was found by running the seeds, not by reading); and keep the mutant script from the start so each rule gets its mutant while its test is fresh.


### Verify follow-up (after the verifier approved at 44f0bbc)

Commits on this branch, in order, each separate: the failing test; the fix; seeds and the new record; the four mutant tests; the doc fix. Nothing pushed; `decisions/`, `scripts/` and dependencies untouched.

**1. Fix: an Exif block with ANY walk issue is unreadable for partial policies.**
- New fixture `gps-count-overflow` (`strip_shapes::gps_count_overflow`): a big-endian Exif whose GPS IFD declares 0xFFFF entries, with canary numerators `CANA` in the latitude rationals. Test `a_block_with_a_walk_issue_needs_entry_edit_for_partial_policies` asserts the canary is in the file, `audit` has 0 GPS findings, `Gps`, `GpsAndDevice` and `AllPii` give `NeedsEntryEdit`, `{Xmp}` still strips, and `All` removes the block whole (canary gone).
- RED on 25b2b37 (commit "test(write): a GPS IFD with 0xFFFF entries ..."): `strip(Gps)` returned Ok with the file unchanged (panic message `Gps`). GREEN after the fix.
- Fix (`src/write/jpeg.rs`): `unreadable` is now `failed_blocks > 0 || exif_issues > 0 || more than 16 Exif segments` (`exif_issues` is the audit's sum of TIFF-layer issues over the readable blocks, so it is true when the tolerant walk reported any issue for any block). The harness's naive Exif rule was changed the same way. `All` and `Everything` are unchanged (Exif removed whole; the new test and the existing grid show it).
- Effect on existing outcomes: none. Every existing fixture, every committed seed (`no_fixture_or_seed_gives_verify_failed` runs `jpeg_segments` and `audit` seeds under 13 policies) and the 22 existing ExifTool strip records gave the same result; `just oracle-record` changed only by adding `gps-count-overflow.txt`. (The kitchen-sink Exif and the other fixtures walk without issues.)
- The mutant "fix reverted" (drop the `exif_issues` term) fails 4 tests: the new test, `output_equals_the_input_minus_the_listed_removals_plus_the_listed_patches`, `rewrite_jpeg_harness_holds_on_every_seed_and_degenerate_inputs`, `strip_records_replay_and_reproduce`.
- 60 s smokes after the fix: `rewrite_jpeg` 49,094 runs, cov 3477, ft 14656, exit 0; `audit` 129,069 runs, cov 2343, ft 10233, exit 0.

**2. Tests for the surviving mutants** (each shown red against its mutant, then `git checkout -- src`; `git diff --stat -- src` empty after each revert):

| mutant | test | result |
|---|---|---|
| v03b primary size patched only when it was already right | `v03b_the_primary_size_is_patched_even_when_it_was_already_wrong` (tests/strip.rs; the index says the primary is 7 bytes longer than it is; the field must become `size - removed`) | red under the mutant (1 failing), green otherwise |
| v05 self-check (b) ignores the region kind | `v05_self_check_b_compares_the_region_kind` (verify_tests.rs; a plan that turns an extended-XMP segment into a plain XMP segment by two patches and a removal: same category, same count, other kind; expects `Check::Kept`) | red (1 failing) |
| v06 self-check skipped on an empty plan | `v06_an_empty_plan_is_still_checked` (src/write/mod.rs; `strip` now calls a private `finish(data, spec, planned)` so a planner that wrongly returns an empty plan can be simulated) | red (1 failing) |
| v10 MpfFields compares input images with input images | `v10_mpf_fields_compares_the_output_images_with_the_input_images` (verify_tests.rs; the COM is removed and only the primary size patched, so the secondary offset is stale and the primary is right) | red (1 failing; without the fix `Kept` is reported instead of `MpfFields`) |

- v06, as the spec states it: rule 9 says the checks run before any byte is returned, and rule 8 only says an empty plan returns the input; so an empty plan for an input that still holds a finding of a removed category is `VerifyFailed {Preview}` and no bytes. Through `strip` this is unreachable (the planner removes every such segment), hence the `finish` split.

**3. Seeds.** `fuzz/seeds/rewrite_jpeg` is regenerated by `examples/fuzz_seeds.rs` (186 files, was 129; nothing else changed): byte 3 (the All preset) now has the `allpreset-` family for all 42 inputs (17 shapes and 25 strip fixtures); bytes 0 (`Gps`) and 1 (`GpsAndDevice`) cover `mpo-plain`, `two-exif-blocks`, `exif-without-gps`, `com-over-embedded`, `unindexed-mpf` and the new `gps-count-overflow`, on top of the kitchen sink; bytes 2 and 5 stay kitchen-sink only (not asked). `tests/fuzz_layout.rs` pins 186, requires the `allpreset-` family, policy byte 3 and the narrow-preset names.

**4. Docs.** The harness header in `fuzz/shared/rewrite_invariants.rs` and deviation 9 above now list the library calls exactly and the two excused `VerifyFailed` cases. The token count is corrected to 564890 here and in the front matter.

**Gates after the follow-up (all green):** `cargo fmt --all -- --check`; `rustfmt --check --edition 2021 fuzz/shared/*.rs`; `just lint`; `just lint-fuzz`; `just lint-no-allow`; `just lint-red-proof`; `just lint-fuzz-red-proof`; `cargo test --no-fail-fast` with `DIAMOND_EXIF_REQUIRE_ORACLE=1 DIAMOND_EXIF_REQUIRE_CLI_FS=1`: 762 passed, 0 failed, 9 ignored on stable and on 1.90.0; `-F unsafe_code` on lib and bin; `just deny`, `just deny-fuzz`; `just validate`; `just decisions-audit`; `just cost-audit`; `just oracle-record` leaves `git status --porcelain` empty (23 strip records); release build of `exif-audit`; the two fuzz smokes above. The local-photo run was not repeated (the fix only affects partial policies; `All` and `Everything` are unchanged); no real-photo data was read or written in this follow-up.
