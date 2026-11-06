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
  id: HANDOFF-019
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-05
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-010

project:
  id: PROJ-001
  stage: STAGE-002
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
  branch: worktree-agent-ab395ff3e0934e17f
  pr: null
  completed_at: 2026-11-05         # YYYY-MM-DD
  notes: "No token meter is visible to the implementer subagent; tokens_total, estimated_usd and duration_minutes are null and the orchestrator transcribes the metered figure from the Agent result. Committed locally, not pushed."
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-019: Exif and located metadata regions from JPEG

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-010` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

Read before starting:

- **Spec (API, extraction rules and named tests are normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-010-exif-and-located-metadata-regions-from-jpeg.md`
- **`decisions/DEC-006-jpeg-segment-walk-and-issue-channel.md`** (incl. "Clarifications from the SPEC-010 design"; normative), DEC-001, DEC-002, DEC-004, DEC-005
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `~/.cargo/bin/cargo +nightly`; `just lint` under the default PATH)
- Existing code to build on: `src/jpeg/{mod,issue}.rs`, `src/tiff/mod.rs` (`Container::parse`), `src/error.rs`, `src/lib.rs`, `fuzz/fuzz_targets/jpeg_segments.rs`, `examples/fuzz_seeds.rs`, `tests/fuzz_layout.rs`, `app.just`
- Prior art for how a build round is reported and what verify caught: HANDOFF-017/018 (SPEC-009), HANDOFF-009/010 (issues recorded after a halt)

Constraints: `panic-free-library` (five-lint deny set in non-test code), `forbid-unsafe-code`, `fuzz-target-per-surface`, `test-before-implementation`, `no-new-top-level-deps-without-decision` (no dependencies of any kind).

## Expected Deliverables

- Everything in the spec's Outputs with all its named tests and every Acceptance Criterion met, including the seven required mutations (a)-(g) each shown red by its named test then reverted, the fuzz invariants (container independence, iff-invariants) each shown red with a planted bug (at least one against the real libFuzzer target), the 13 new seeds, and `just fuzz-smoke jpeg_segments` clean for 60 s with executions and coverage recorded.
- **Write every fuzz iff-invariant in both directions.** For each plant, also try the single-guard variant where two guards overlap.
- **Nothing is recorded after a halt**, and every issue goes through the single `record` function that enforces the cap. Applies to the new `ExtraExif` and region issues too.
- Walker change: a bad segment length now skips four bytes (marker + the two length bytes). Update the one SPEC-009 test that pinned the old double issue, and say so in the Handback.
- Name every deviation (renamed, folded or missing tests) in the Handback; if a derived number in the spec does not match what the rules produce, stop and report it rather than adjusting it. Put any behaviour you pin that differs from this brief or the spec in the Handback too.
- Add the C2PA-in-JUMBF and several-Exif-segments open questions to `guidance/questions.yaml`.
- Commit on your worktree branch. **Do not push and do not open a PR.** The spec is already at `cycle: build`; do not advance cycles. First run `git merge --ff-only claude/rust-exif-metadata-6351d5` so your worktree has `0052025` or later.
- Do not modify irradiance, `scripts/`, the TIFF engine's non-test code, the oracle code, or the ExifTool records. The existing 29 TIFF and 21 JPEG fuzz seeds must remain byte-identical. `decisions/` is not yours to edit. No dependencies or dev-dependencies.
- You cannot see your own token count; set `tokens_total: null` with a note, the orchestrator transcribes the metered figure.

## Out of Scope

- JPEG fixtures and ExifTool parity (SPEC-011), decoding XMP/ICC/IPTC/JUMBF contents, C2PA detection, writing
- Pushing, PRs, cycle advancement

## Return Criteria — how to hand back

**Completing this handoff means filling in the `handback:` front-matter block
above AND the `## Handback

### Execution notes

- **Branch / PR:** branch `worktree-agent-ab395ff3e0934e17f`, one local commit on top of `e2bc31e` (fast-forwarded from `claude/rust-exif-metadata-6351d5`, so `0052025` is included). Not pushed, no PR, cycle not advanced.
- **Completed at:** 2026-11-05
- **All acceptance criteria met?** Yes. `jpeg::read` implements rules 1-8; every named test exists and passes; the seven mutations are shown red; five fuzz invariants are shown red with planted bugs, one against the real libFuzzer target; the 60 s smoke is clean. Gate results are below.
- **For `verify`:** not applicable.

**Gates (all green, on the final tree):** `cargo fmt --check` on both graphs (root with stable and with 1.90.0; `fuzz/` with `--manifest-path`; the included `fuzz/shared/*.rs` formatted with `rustfmt --edition 2021`), `just lint`, `just lint-fuzz`, `just lint-no-allow`, `cargo test` on stable (Homebrew 1.99.1) and on `~/.cargo/bin/cargo +1.90.0` (143 unit, 4 crate_policy, 10 fuzz_layout, 28 metadata_oracle, 36 mutation_oracle; 1 ignored each in the two oracle files as before), `cargo rustc --lib -- -F unsafe_code`, `just deny`, `just deny-fuzz`, `just validate`, `just decisions-audit` (0 structural errors; the scope warnings are the existing ones), `just cost-audit`, `just oracle-record` (empty diff under `tests/`; `two-hop-cycle` not run, as before). `[dependencies]` and `[dev-dependencies]` are still empty.

**Required mutations (each applied alone to `src/jpeg/metadata.rs`, `cargo test --lib jpeg::`, then reverted):**

| | Mutation | Red by (primary named test) |
|---|---|---|
| a | `tiff` slice from `SIG_EXIF.len() - 1` / `+ 1` | `the_tiff_slice_starts_after_the_six_byte_signature` (and 9 more Exif tests) |
| b | `ExtraExif` for the first Exif (`>= 0`); and for the third on (`> 1`) | `a_second_exif_segment_is_an_extra_exif_issue_and_still_a_block`, `exif_blocks_are_capped_at_sixteen_but_extras_are_all_reported` |
| c | block cap `<` to `<=`; and `< MAX - 1` | `exif_blocks_are_capped_at_sixteen_but_extras_are_all_reported` |
| d | ICC check removed | `icc_chunk_mismatches_are_one_issue`, `icc_chunks_report_sequence_and_count` |
| e | 8BIM name padding removed / inverted; data pad dropped | `iptc_0404_resource_is_located_with_even_padding` (plus the truncated/stray/no-0404 tests) |
| f | `is_clean` ignores block cleanliness (`all(true)`); ignores only cleanliness (`is_ok`) | `is_clean_requires_every_layer`, `a_damaged_gps_ifd_leaves_a_clean_jpeg_layer` |
| g | extended-XMP header 39 instead of 40 | `a_short_extended_xmp_header_is_region_too_short`, `extended_xmp_header_fields_are_read` |

Extra single-guard mutations, all red by `icc_chunk_mismatches_are_one_issue`: each of the four ICC guards (`differs`, `seq == 0`, `seq > count`, `repeated`) removed alone, the final count check removed, and the final-count `at` moved to the last chunk. `record` ignoring a halt is red by `nothing_is_recorded_after_a_halt`; GUID 31, JUMBF header 5 and IPTC resource id `0x0405` are red too. **One survivor, equivalent by construction:** removing `|| self.issues.len() >= MAX_JPEG_METADATA_ISSUES` from `Extract::record` stays green because the bound is unreachable (at most one issue per segment plus one ICC aggregate); it is a belt-and-braces guard, and `the_jpeg_layer_issue_total_never_exceeds_the_documented_constant` pins the constant itself.

**Fuzz plants** (each applied alone to `src/jpeg/metadata.rs`, run against the shared invariants through `cargo test --test fuzz_layout`, then reverted). Red: `Container::parse` of a tiff truncated by one byte (container independence); `is_clean` ignoring block issues and, separately, ignoring only fatal blocks (cleanliness iff); `ExtraExif` for the first Exif, and only from the third on (ExtraExif iff); block cap off by one; extended-XMP header 39 (the 75-byte iff); each ICC guard removed alone, the final count check removed, and two issues instead of one (IccChunkMismatch iff, once, last); `record` ignoring a halt; IPTC name padding inverted and data pad dropped; XMP packet off by one; JUMBF header 5; Exif tiff slice 5; `issues()` with the layers swapped. Two plants initially survived and each exposed a weak spot that is now fixed: ignoring block issues (no input had a TIFF-layer issue under a clean JPEG layer; fixed by `every_tiff_seed_wrapped_in_a_jpeg_satisfies_the_harness`) and the swapped layer order (my "JPEG issue after an Exif issue" check used `last_block == 0`, which cannot see block 0; fixed with a `seen_exif` flag). **Against the real target:** with the `seq > count` ICC guard removed, `just fuzz-smoke jpeg_segments` crashed within the 60 s with `IccChunkMismatch iff the chunks disagree` (artifact input: `FF D8 ... FF E2 00 16 "ICC_PROFILE\0" 02 01 ...`); the plant was reverted and `fuzz/artifacts` removed.

**60 s smoke, clean (final tree):** `just fuzz-smoke jpeg_segments`, "Done 109751 runs in 61 second(s)", final line `#109751 DONE cov: 1480 ft: 6328 corp: 509/2490Kb exec/s: 1799` (an earlier clean run on the same code: 123214 runs). Exit 0.

**Seeds:** 13 new in `fuzz/seeds/jpeg_segments/` (21 existing + 13 new = 34 JPEG seeds; the 29 TIFF seeds are unchanged). The existing 50 seeds are byte-identical (`git status` shows only the 13 new files as untracked, none modified).

### Cost self-report

- **Tokens (total):** null. The implementer subagent has no token counter (no `/cost`, no `usage` object visible); the orchestrator transcribes the metered figure from the Agent result.
- **Estimated USD:** null
- **Duration (minutes):** null (not measured)
- **Source of the number:** none available

### Drift and new artifacts

- **New decisions emitted:** none (`decisions/` is not mine to edit).
- **Deviations from spec:**
  1. **Shared invariants file instead of two mirrored copies.** The new fuzz invariants live once in `fuzz/shared/jpeg_metadata_invariants.rs` and are `include!`d by both `fuzz/fuzz_targets/jpeg_segments.rs` and `tests/fuzz_layout.rs`, so the stable-toolchain test and the libFuzzer target cannot drift. "`exercise` mirrors the new invariants" is therefore met by sharing, not by a second copy. The file is formatted with `rustfmt --edition 2021` by hand because `cargo fmt` does not follow `include!`.
  2. **The invariants re-derive the whole extraction independently** (regions, expected extraction issues, the ICC consistency test by sorting sequence numbers, the IPTC walk) and compare both ways, which is stronger than the spec's list (it covers it).
  3. **A `pub(crate) Jpeg::halted()` accessor** (one new private field on `Jpeg`) carries the walker's halt to the extraction. Rule 7 does not say what happens to extraction issues after a cap; I applied the handoff's "nothing is recorded after a halt": when the walk halted (`TooManySegments` or `TooManyIssues`), `Extract::record` records nothing, so no `ExtraExif`, `RegionTooShort`, stray-byte or ICC issue follows the halting issue, which stays last. Exif blocks and regions are still reported after a halt; only issues are suppressed. The fuzz invariants state the iffs "unless halted" accordingly (`ExtraExif` count, `RegionTooShort` for extended XMP, the ICC aggregate).
  4. **ICC `at` rule (defined in the module docs and tested):** chunks are checked in segment order against the first chunk's `count`; a chunk is inconsistent if its `count` differs from the first's (not for the first chunk itself), its `seq` is 0 or above its own `count`, or its `seq` repeats; the first inconsistent chunk's segment offset is `at`, so when the first chunk is the odd one out the second chunk is named. If every chunk is individually consistent but the number of chunks differs from `count` (for example count 3 with two chunks), `at` is the **first chunk's** offset (the chunk that declared the count). The spec left this case open; the tests pin it.
  5. **IPTC issue offsets and sizes.** For the `8BIM resource` `RegionTooShort`, `at` is the input offset of the resource (not the segment), `need` is counted from the resource start (6 when the id is cut short, 7 when the name length byte is missing, then the end of the fixed fields, then the end of the data) and `have` is the bytes left from the resource start. `StrayBytes` `at` is likewise the input offset of the first stray byte and `len` the remainder. For the other regions `at` is the segment's offset as specified. The last resource of a segment may omit its trailing pad byte (tolerated, no issue).
  6. **Tests added beyond the named ones:** in `metadata.rs`, `nothing_is_recorded_after_a_halt`, `issues_name_the_layer_jpeg_first_then_blocks_in_order`, `regions_are_in_segment_order_and_borrow_from_the_input`, `signatures_under_the_wrong_marker_yield_nothing`; in `tests/fuzz_layout.rs`, `spec_010_seeds_have_the_shapes_they_claim`, `every_tiff_seed_wrapped_in_a_jpeg_satisfies_the_harness` and `jpeg_harness_holds_on_each_extraction_guard_alone`. No named test is renamed, folded or missing. `a_bad_length_skips_the_marker_and_both_length_bytes` and the updated `length_zero_and_one_are_bad_lengths_and_the_walk_continues` live in `src/jpeg/mod.rs`'s test module (that is where the SPEC-009 test was), and `every_error_variant_this_module_returns_displays_something` exists in both modules (the SPEC-009 one now covers 22 variants; the metadata one covers the three new ones and the display contents).
  7. **`src/tiff/mod.rs`:** only its test-module error guard was extended (three variants), as allowed. The existing exhaustive-match guards in `src/jpeg/mod.rs` tests were extended too (they fail to compile otherwise).
  8. **The seed `icc-chunks-mismatch` has three chunks** (a count of 3, a count of 2 with seq 1, and a third chunk too short for its header), so it yields two issues in the order `RegionTooShort`, then the single `IccChunkMismatch` last; the layout test pins that order.
  9. **No `lib.rs` change:** the new names are re-exported from `exif_core::jpeg` (the module `lib.rs` already exposes); `src/lib.rs` itself is untouched.
- **Derived numbers:** `MAX_JPEG_METADATA_ISSUES = (1024 + 1) + (4096 + 1) = 5122`, as the spec's formula produces; nothing disagreed.
- **Follow-up work identified:**
  - The two open questions are added to `guidance/questions.yaml` (`c2pa-detection-inside-jumbf`, `exiftool-several-exif-segments`). SPEC-011 should measure several-Exif behaviour with two- and 17-Exif fixtures.
  - The ICC "count mismatch with consistent chunks" `at` choice (first chunk) and the IPTC `at` offsets (resource, not segment) are policy picks the verifier may want to confirm.
  - `fuzz/shared/*.rs` is not covered by `cargo fmt --check`; if more shared harness files appear, a `just` check for them would be worth adding.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Rule 7 and the "nothing is recorded after a halt" line do not say how a halted walk and the extraction interact (suppress issues only, or also regions), nor how the extraction learns of the halt; and the ICC `at` for "consistent chunks but wrong number" was left open. Both are decided above and pinned by tests.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — A statement that the shared-harness approach (one file for the target and the stable test) is allowed would have saved a judgement call; DEC-004 only talks about the fuzz graph being separate. Also `cargo fmt` not reaching `include!`d files is a small trap.

3. **If you did this task again, what would you do differently?**
   — Write the independent "expected extraction" in the fuzz harness first and derive the unit tests' expectations from the same reading of the rules; two plants survived the first round only because the harness inputs overlapped, which the per-guard inputs and the wrapped TIFF seeds now cover.
