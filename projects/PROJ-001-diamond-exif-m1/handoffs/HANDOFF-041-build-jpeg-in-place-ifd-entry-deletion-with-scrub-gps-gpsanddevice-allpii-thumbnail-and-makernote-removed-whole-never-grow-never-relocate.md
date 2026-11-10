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
  id: HANDOFF-041
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-10
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-021

project:
  id: PROJ-001
  stage: STAGE-004
repo:
  id: diamond-exif

# ── THE HANDBACK ────────────────────────────────────────────────────────────
# Filled in by the EXECUTING AGENT before it reports done. This is a required
# part of completing the handoff, not a courtesy.
#
# `tokens_total` is the one field the cost gate reads. Report the REAL number,
# and say in `basis` how you got it:
#   Claude Code   → your session transcript, measured per docs/cost-tracking.md
#                   "Measuring a cycle": count each message.id once, take its
#                   largest output_tokens, include subagents/ (basis: transcript).
#                   Summing every line over-counts ~2×. `/cost` in the desktop
#                   Code tab reports context size, not spend.
#   API           → the `usage` objects, summed (basis: api-usage)
#   another agent → whatever your harness reports as total tokens
# If your platform genuinely exposes NO token count, set tokens_total: null AND
# write why in `notes` — then set `cost.metering_source: none` in
# .repo-context.yaml so the gate stops asking. Do not invent a number.
handback:
  status: completed                # completed | blocked | rejected
  agent: claude-sonnet-5-5         # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: null                    # verify handoffs: approved | punch-list | rejected
  basis: agent-result              # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: 746836             # REAL combined count — what cost-audit reads
  estimated_usd: 4.93              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: 66
  branch: worktree-agent-a4c6983a1eacb858d
  pr: null
  completed_at: 2026-11-10         # YYYY-MM-DD
  notes: "Cost transcribed by the orchestrator from the harness final cumulative subagent_tokens (746,836) at 6.60 USD per Mtok = 4.93 USD; duration 3,942,831 ms = 65.7 min, rounded to 66. Nothing was pushed, no PR, no cycle advanced."
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-041: JPEG in-place Exif entry deletion with scrub (SPEC-021)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-021` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

Read before starting:

- **Spec (rules 1 to 14, library changes, fixtures and leak inputs, oracle, fuzz and seeds, the named tests and the 30 mutants are normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-021-jpeg-in-place-ifd-entry-deletion-with-scrub-gps-gpsanddevice-allpii-thumbnail-and-makernote-removed-whole-never-grow-never-relocate.md`
- **DEC-013 with its 2026-11-10 amendment (normative: part B is the entry-editing contract this spec implements; part A ratifies SPEC-020's refusal order and Multi-Picture refusals; read it, do not edit it):** `decisions/DEC-013-writer-foundation-fail-closed-removal-and-patch-plan.md`, with DEC-012, DEC-009, DEC-008 (the tag table), DEC-005 (IFD kinds), DEC-001 (tolerant walk).
- **The shipped writer you extend:** `src/write/{mod,plan,jpeg,verify,verify_tests}.rs`, `tests/strip.rs`, `tests/strip_oracle.rs`, `tests/support/{strip_shapes,strip_records}.rs`, `fuzz/shared/rewrite_invariants.rs`, `fuzz/fuzz_targets/rewrite_jpeg.rs`, `examples/{strip_corpus,fuzz_seeds,oracle_record}.rs`, and the SPEC-020 Handback in `projects/PROJ-001-diamond-exif-m1/handoffs/HANDOFF-039-*.md` (mutant table, ExifTool 13.55 group facts, deviations).
- **Evidence:** `spikes/done/SPIKE-001-*.md`; questions `writer-in-place-delete-and-scrub`, `strip-keeps-orientation` (stays open, out of scope), `writer-stray-bytes`, `writer-mpf-attribute-fields` in `guidance/questions.yaml`.
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; macOS `sed -i ''`; never measure exit status through a pipe; in zsh never name a variable `path`; do not leave the shell in a subdirectory).
- Code to read: `src/tiff/mod.rs` (`Container`, `Ifd`, `Entry`, `value_at`, how the walk records issues and dedups IFD offsets), `src/audit/{mod,category}.rs`, `src/jpeg/metadata.rs` (`exif_blocks`).
- **Lessons that cost follow-ups before (SPEC-019 and SPEC-020):**
  - "Unreadable means refuse" is written as "the walk reported any issue", never as one counter. SPEC-020's verifier found `strip(Gps)` returning `Ok` with coordinates left in because only `failed_blocks` was tested. For every refusal rule, write the leak input first (the spec lists 14) and ask which counter your code tests.
  - Every invariant a fuzz guard can find must also have a deterministic unit test; a guard is only as strong as the seeds that reach it (seeds for every policy-byte family, not only one); plants vary in kind and run in both directions.
  - Measure ExifTool's behaviour on the fixture **before** writing the declared tag table; an "unchanged" claim is checked against the rule that could move it.
  - Surviving mutants in SPEC-020 were all "a check that is skipped or compares the wrong thing": for each new check write a test whose plan is wrong in exactly that way.
  - `rustfmt --check --edition 2021 fuzz/shared/*.rs` is part of `just lint`; a mutant's revert must leave `git diff` empty (`git checkout`, never by hand).
  - The tolerant walk may differ from the harness's naive reader on edge cases; fail closed in the library, and in the harness only "library Ok and naive unclean" is a failure.

Constraints: `panic-free-library` (five-lint deny set: no unchecked `+`, `-`, `*`; every offset and length is untrusted: `checked_*`, `get(..)`, never index), `forbid-unsafe-code`, `no-io-in-library`, `fuzz-target-per-surface` (extend `rewrite_jpeg`), `test-before-implementation`, `oracle-must-be-shown-red`, `exiftool-never-linked`, `no-new-top-level-deps-without-decision`.

## Expected Deliverables

- Everything in the spec's Outputs and every Acceptance Criterion; every named test under its name (a rename, fold or missing test is a deviation to report). New: `src/write/exif.rs`, tiff accessors, new fixtures in `strip_shapes.rs`, extended oracle tests and records, extended harness and at least 60 new seeds, extended `strip_corpus`. `NeedsEntryEdit` is removed and replaced by `Uneditable`; the SPEC-020 tests that expected it are rewritten (report each). Under whole and none policies the outputs are byte-identical to SPEC-020's (rule 1).
- Order as in the spec's Notes for the Implementer: tiff accessors; `exif.rs` on one hand-built IFD; keep set and `SharedBytes`; GPS child removal; thumbnail child removal; planner integration; `Entries` and `Scrubbed` checks; **measure ExifTool (including rule 9's emptied IFD) before writing the declared tables**; harness, seeds, CI; the 30 mutants; the local corpus.
- The expected bytes in tests come from the fixture builder's **layout record**, not from the library or the walk.
- The local corpus re-run: `/Users/jyashinsky/PSeven/experiments/crustimg_redo_plus/_incoming0` holds the user's real photos. **Read it in place only; copy nothing into the repo; write no file derived from it (no temp file, no saved ExifTool output); print and record only counts and 8-hex short hashes, never values, content or paths.** ExifTool on a real output only through a pipe (`--emit` to stdout piped to `exiftool -`), one file at a time.
- A doubt about a DEC-013 rule is a question for the architect: write it in the Handback and `guidance/questions.yaml` with the input that raised it; do not edit `decisions/`.
- Run locally, because they are CI-only otherwise: `just lint-fuzz-red-proof`, `just lint-red-proof`, `just oracle-record` (empty `git status --porcelain` afterwards), `just fuzz-smoke rewrite_jpeg`, `jpeg_segments`, `audit`, `audit_render`, `cli_args` (60 s each; record runs and cov), `cargo build --release --locked --bin exif-audit`, `cargo test --no-fail-fast` on stable and 1.90.0 with `DIAMOND_EXIF_REQUIRE_ORACLE=1 DIAMOND_EXIF_REQUIRE_CLI_FS=1`, and the rest of the spec's gate list.
- Commit on your worktree branch. **Do not push, do not open a PR, do not advance cycles.** First `git merge --ff-only claude/rust-exif-metadata-6351d5` (you need the commit containing HANDOFF-041); if refused, use `git merge` and say so.
- Do not modify irradiance, `scripts/`, `decisions/`. No dependencies. Scratch work goes in a scratchpad directory, not `/tmp`.
- You cannot see your own token count; set `tokens_total: null` with a note, the orchestrator transcribes the metered figure.

## Out of Scope

- Keeping Orientation under `All`/`Everything`; TIFF and DNG; XMP or IPTC editing; MakerNote or thumbnail parsing; a write CLI; PNG, WebP, HEIC; changing the audit, the presets, the report schema, exit codes or the CLI
- Pushing, PRs, cycle advancement, SPEC-022

## Handback

**What exists.** `src/write/exif.rs` (the planner, blob-level plus the JPEG glue), `Ifd::{table_range,next_at}`, `Entry::value_at`, `Container::payload_range`, `ExifBlock::tiff_at`; `StripError::Uneditable` replaces `NeedsEntryEdit`; `Stripped::{entries,scrubs}`, `PatchKind`, `ScrubReason`, `EntryRecord`, `ScrubRecord`; checks `Entries` and `Scrubbed` (in that order after `Walk`, before `Idempotent`); `tests/support/exif_layout.rs` (fixture builder with a layout record and the independent edit model), `tests/support/exif_records.rs` (fixtures as JPEGs, policy grid, leak inputs, seed list); 198 new `rewrite_jpeg` seeds (384 in all); the naive TIFF reader and entry editor and five new guards in `fuzz/shared/rewrite_invariants.rs`; ExifTool tag-level oracle with 12 new records `tests/oracle-fixtures/exiftool-strip/exif-*.txt`; `examples/strip_corpus.rs` extended. No dependency, no `decisions/`, no `scripts/`, no `unsafe`, no `allow` outside test modules (one `#[allow(dead_code)]` in the fuzz harness file, for a function the layout test calls).

**Gates (all exit 0, measured directly).** `cargo fmt --check` (library, fuzz graph), `rustfmt --check --edition 2021 fuzz/shared/*.rs`, `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `forbid-unsafe`, `deny`, `deny-fuzz`, `./scripts/validate.sh`, `decisions-audit`, `cost-audit`, `cargo build --release --locked --bin exif-audit`, `just oracle-record` (then `git status --porcelain` empty), `cargo test --no-fail-fast` with `DIAMOND_EXIF_REQUIRE_ORACLE=1 DIAMOND_EXIF_REQUIRE_CLI_FS=1` on stable (Homebrew 1.99.1) and `+1.90.0`: 848 passed, 0 failed, 9 ignored on each. Fuzz smokes, 60 s each: `rewrite_jpeg` 94,089 runs cov 5,585; `jpeg_segments` 185,455 runs cov 1,843; `audit` 57,725 runs cov 2,290; `audit_render` 34,537 runs cov 5,138; `cli_args` 2,252,013 runs cov 872; all exit 0. A longer `rewrite_jpeg` run (270 s) found one crash that is not from this spec (see the questions). One plant repeated against the real libFuzzer target from the committed seeds: mutant 7 (scrub not applied) made `rewrite_jpeg` crash on a seed within 30 s (guards Records and EntryPrediction).

**The 30 mutants** (each applied by exact text replacement, tests run, reverted with `git checkout`; the diff was empty after every revert). Caught by: 1 `delete_a_middle_entry_shifts_and_zeroes_the_tail` (also first, last, every); 2 the same; 3 `delete_a_middle_entry_shifts_and_zeroes_the_tail`, `delete_two_non_adjacent_entries`; 4 `next_ifd_word_follows_the_surviving_entries`; 5 `delete_a_middle_entry_shifts_and_zeroes_the_tail`; 6 the same; 7 the same and `out_of_line_value_is_zeroed`; 8 `gps_child_removal_deletes_the_pointer_and_zeroes_the_ifd_and_values`, `makernote_is_deleted_whole_and_its_value_zeroed`; 9 `gps_child_removal_...`, `thumbnail_removal_sets_the_ifd0_next_word_to_zero_and_zeroes_ifd1_and_the_jpeg`; 10 `payload_range_is_some_only_for_out_of_line_values`; 11 `gps_child_removal_...`; 12 `gps_child_removal_...`; 13 `gps_child_removal_...`; 14 `thumbnail_removal_sets_the_ifd0_next_word_...`; 15 `thumbnail_removal_sets_..._`, `thumbnail_range_overlapping_a_kept_value_is_shared_bytes`, `custom_thumbnail_only`; 16 `thumbnail_removal_sets_...`; 17 `thumbnail_with_a_following_ifd_is_chain_after_thumbnail`; 18 `custom_makernote_only`, `all_pii_matches_the_policy_table_entry_by_entry`; 19 `custom_free_text_deletes_the_exif_free_text_entries_and_the_comments` (and 32 more, through the `Entries` check); 20 the four `*_is_shared_bytes` unit tests; 21 `value_inside_a_kept_ifd_table_is_shared_bytes`; 22 `a_block_with_a_walk_issue_is_uneditable_for_every_partial_policy`, `a_walk_issue_in_an_ifd_with_no_target_still_refuses`; 23 `exif_after_the_first_scan_is_uneditable`; 24 `delete_a_middle_entry_shifts_and_zeroes_the_tail` (be); 25 `a_plan_that_corrupts_a_kept_entry_value_fails_check_entries`, `entries_check_compares_type_count_and_value_not_only_the_tag`; 26 `a_scrub_that_leaves_a_byte_fails_check_scrubbed`; 27 `stripped_records_list_every_entry_scrub_and_patch_with_kinds`, `the_output_equals_the_input_outside_the_recorded_patches_and_removals`; 28 (audit table, `categorize` row `0x0112`) `audit::category::tests::categorize_assigns_nothing_outside_the_table` and the AllPii tests; 29 `a_plan_that_deletes_a_kept_exif_finding_fails_check_kept` (a test added for it: before it, nothing failed); 30 `all_removes_every_exif_block_including_the_extra_ones` and 20 more. Mutant 28 is applied at the category table (Orientation filed under CameraModel), the only place a tag becomes a target, since the writer takes its targets from the audit.

**ExifTool 13.55 measurements (written into the declared tables after reading them).** Group names: `IFD0`, `ExifIFD`, `InteropIFD`, `GPS`, `IFD1`, `IFD2` (a third chain IFD), `SubIFD`. Tag names that differ from the DEC-008 names: tag 0xA431 reads `SerialNumber`, 0x0132 `ModifyDate`; the MakerNote prints no tag (only `ExifTool:Warning [minor] Unrecognized MakerNotes`, in the input and in every output that keeps it); an IFD0 0x02BC of non-XMP bytes prints `Warning [minor] Empty XMP` and no tag; pointers print nothing; a kept IFD1 with both thumbnail pointers adds `IFD1:ThumbnailImage`; Composite `GPSLatitude`, `GPSLongitude`, `GPSPosition` are derived from the GPS group and vanish with it (declared expected losses; `ShutterSpeed` stays). Two Exif segments: ExifTool reads both (`-a`), `File:ExifByteOrder` printed twice. **Rule 9, an IFD left with no entry (count 0):** IFD0 emptied, the Exif IFD emptied (its pointer kept) and IFD1 emptied (chain kept) all read with no warning and no error; the group prints no tag; `File:ExifByteOrder` survives (`II`); the walk calls each output clean and `strip(output)` is an empty plan. So rule 9 allows it (count word 0, next word, zeros); `EmptiedIfd` exists in `UneditableReason` and is never returned (the spec said "only if the measurement says it is needed"). Pinned by `emptied_ifd_behaviour_is_pinned` and `delete_every_entry_leaves_count_zero_or_refuses_per_the_measurement`. IFD1 after unlink: the group disappears entirely, with `ThumbnailImage`.

**Local corpus (`_incoming0`, read in place, counts and 8-hex hashes only, no file written).** 21 files, 13 JPEGs, 4 with a Multi-Picture index. `Gps` 13 Ok, `GpsAndDevice` 13 Ok, `AllPii` 12 Ok and 1 `Uneditable(ChainAfterThumbnail)` (hash `a3b66f84`: IFD0, Exif, Interop, IFD1 with a next word, and a third chain IFD of 11 entries); `All` 13 Ok, `Everything` 13 Ok. For `Gps` and `GpsAndDevice`: bytes removed 0, output length equal to input length 13 of 13, scan region identical 13 of 13, audit of the output clean of the policy's categories 13 of 13, the former out-of-line value of every deleted entry reads zero in the output 13 of 13 (22 and 24 entries deleted; 1,035 and 1,968 bytes patched). `AllPii` 12 Ok: scan identical 12 of 12, audit clean 12 of 12, 154 entries deleted, 100,969 bytes patched, values zero 12 of 12; the 313,995 bytes removed by `All` are SPEC-020's. ExifTool by pipe, one file at a time, in memory (stdin): the `GPS` group is gone from exactly the 4 files that had it, under all three policies (4 of 4, none left, none appeared); `IFD1` (thumbnail) is gone after `AllPii` for 4 of 4 files that had it among the 12 Ok, and unchanged under `Gps` and `GpsAndDevice`; the `IFD0:Orientation` count before and after is equal in all 38 runs; 0 new warning lines; the `MPImage` group count is unchanged for every Multi-Picture file in every run. No file of the corpus was copied or derived; the examples print only counts and short hashes.

**Deviations from the spec (all deliberate; each is reported, none edits `decisions/`).**
1. **Red first.** The accessor tests were run red (compile error) before the accessors; for `exif.rs` and the verify tests the tests were written before the planner but the planner was written in full before the first run, so there was no stub-red. The 30 mutants (each turning at least one named test red) and the real-libFuzzer plant are the evidence of teeth instead.
2. **Three `UneditableReason`s beyond the list:** `HiddenPointer` (a pointer tag the walk did not follow: a second entry with the tag, or one in an IFD kind that does not follow it), `NestedIfd` (a removed GPS IFD or IFD1 that has IFDs of its own), `PayloadUnreadable` (a deleted entry of an unknown field type). `EmptiedIfd` is defined and unused (rule 9). The leak input `gps_pointer_missing_in_parent` cannot be built through the walk (a GPS IFD is found only through its pointer); its buildable form (a first GPS pointer of 0 and a second one to the GPS IFD, which the walk never visits) refuses with `HiddenPointer`; the unit test `gps_pointer_missing_from_the_parent_is_pointer_not_in_parent` mutates the blob after the walk to reach `PointerNotInParent`. `gps_ifd_pointed_to_twice` (two entries, same target) is also `HiddenPointer`.
3. **The pairwise "no two IFD tables overlap" check runs for every partial policy**, not only when something is edited (the harness guard `LibraryOkNaiveUnclean` names overlapping ranges).
4. **`Ifd::table_range()` and `Ifd::next_at()` return `Option`** (checked arithmetic), not `Range`/`usize`. `ExifBlock::tiff_at()` was added to `src/jpeg` (the blob's input offset); segment index and after-scan were already reachable.
5. **Order of the refusals inside the Exif planner (pinned):** `TooManyExifSegments`, `InventoryTruncated`, then for every block `AfterScan` and `BlockNotClean`, then the per-block planning. The offset of `TooManyExifSegments` is the 17th segment; of `InventoryTruncated` the first Exif segment; of a block-level refusal the segment (or, for a fault inside, the first byte in conflict as an input offset).
6. **Check (g) derives the deleted set from the input's audit and walk** (findings in the policy's categories, IFD1 when the thumbnail goes, the first `0x8825` entry of the parent of every GPS IFD), not from the plan, so a plan that deletes more or less fails it; `verify` takes the zeroed ranges as a fourth argument.
7. **`Custom({Xmp})`, `{Icc}`, `{Iptc}` and any set naming only those are partial** (spec rule: they can sit in an Exif IFD), so an unreadable Exif block now refuses them (SPEC-020: Ok). Changed SPEC-020 expectations: `unreadable-exif` and `gps-count-overflow` under `xmp`, `icc`, `free_text`, `iptc+free_text`; `kitchen-sink` under `iptc+free_text` and `free_text` now strips Exif free text. Their ExifTool records in `exiftool-strip/` were re-recorded (3 files changed, `iptc+free_text` section added to `kitchen-sink`).
8. **SPEC-020 tests rewritten (every one that expected `NeedsEntryEdit`):** `custom_iptc_removes_both_split_app13_segments` (tail), `custom_free_text_needs_entry_edit_when_exif_has_free_text` to `custom_free_text_deletes_the_exif_free_text_entries_and_the_comments`, `gps_preset_needs_entry_edit_when_exif_has_gps` to `gps_preset_edits_the_exif_when_it_has_gps`, `all_pii_needs_entry_edit_and_names_its_categories` to `all_pii_edits_the_exif_and_leaves_software`, `an_unreadable_exif_block_needs_entry_edit_for_partial_policies` to `..._is_uneditable_for_partial_policies`, `a_block_with_a_walk_issue_needs_entry_edit_for_partial_policies` to `a_block_with_a_walk_issue_is_uneditable_for_every_partial_policy`, the grid arm in `output_equals_the_input_minus_...`, the length test (patches allowed when nothing is removed), the model (`Expect::NeedsEntryEdit` to `Uneditable(offset)` and `EntryEdit`), and in `tests/fuzz_layout.rs` the `NeedsEntryEdit` plants (now a walk fault reported as Uneditable; the VerifyFailed-on-a-refusal plant moved to the overlap input) and the expected refusal classes. `rewrite_jpeg_harness_holds_on_every_seed_and_degenerate_inputs` was renamed to `rewrite_jpeg_harness_holds_on_every_seed_including_the_entry_seeds` (the spec's name).
9. **No committed seed for `exif_inventory_over_budget`** (13 blocks of 5,459 entries, about 850 KB; a deterministic test only, `an_inventory_over_budget_is_uneditable`). The 64 KB ceiling fixture has seeds under two policies, not eight. Fixture and seed names use hyphens (`every-entry-target`, `chain-ifd2`), the leak seeds the spec's names with underscores.
10. **`exif_walk_issue_in_a_kept_ifd`** is an Interop pointer outside the blob; the fixture list's `exif_chain_ifd2` is `chain-ifd2` and doubles as `thumbnail_with_ifd2_after_it`.
11. **Entry plants:** two per new guard, each guard red on its plants; "alone" is checked as "the guard is among the failing ones" (as SPEC-020's test did), not as the only one, because a changed byte inside the Exif segment fails `Prediction` outside it and `KeptEntriesIntact` inside it.

## Questions raised (all in `guidance/questions.yaml`, status open)

- `allpii-refuses-ifd1-with-a-following-ifd`: 1 of 13 local files refuses `AllPii`; keep the refusal or keep an empty IFD1 in the chain?
- `writer-refusals-beyond-the-spec`: ratify `HiddenPointer`, `NestedIfd`, `PayloadUnreadable` and the always-on table-overlap check.
- `partial-policy-refuses-unreadable-exif-for-xmp-icc-iptc`: confirm that Xmp, Iptc and Icc sets are partial.
- `writer-self-check-fires-on-segments-joined-by-stray-bytes`: a 270 s `rewrite_jpeg` run found an input where `All` returns `VerifyFailed {MpfFields}` while the harness predicts Ok; it reproduces on the SPEC-020 library (6518356). Stray bytes between segments join once the segments between them go. Not fixed here; the 24 h gate would hit it.
