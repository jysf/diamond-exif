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
  id: HANDOFF-029
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-06
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-014

project:
  id: PROJ-001
  stage: STAGE-003
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
  branch: worktree-agent-a89950596c9661fe5
  pr: null
  completed_at: 2026-11-06         # YYYY-MM-DD
  notes: "tokens_total null: a sub-agent cannot read its own meter; orchestrator transcribes the metered figure. No PR (not asked)."                      # one line if unusual (rework, no meter, etc.)
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-029: StripPolicy presets as data and the policy preview; DEC for preset semantics

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-014` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

Read before starting:

- **Spec (API, rules, expectation matrices, named tests and mutants are normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-014-strippolicy-presets-as-data-and-the-policy-preview-dec-for-preset-semantics.md`
- **DEC-009 (normative for every preset cell and the may-hold table; read it, do not edit it):** `decisions/DEC-009-strip-policy-presets-and-the-preview.md`
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe; macOS `sed -i ''`)
- Decisions: DEC-008 (categories, severity), DEC-006, DEC-003 (provenance)
- Code: `src/audit/{mod,category}.rs`, `src/lib.rs`, `tests/support/{audit_tags,oracle_tiff,oracle_jpeg}.rs` (fixture builders), `tests/fuzz_layout.rs`, `fuzz/shared/audit_invariants.rs`, `fuzz/fuzz_targets/audit.rs`
- Prior art for the harness style (guards run alone, both directions, step counter): SPEC-012 and SPEC-013 handoffs (HANDOFF-023/024/027/028).

Constraints: `panic-free-library` (five-lint deny set: `arithmetic_side_effects` forbids `<<` and `+` on integers in the library; use literal per-category constants and iterators), `forbid-unsafe-code`, `no-io-in-library`, `fuzz-target-per-surface`, `test-before-implementation`, `oracle-must-be-shown-red` (applied here to the mutant list), `no-new-top-level-deps-without-decision`.

## Expected Deliverables

- Everything in the spec's Outputs and every Acceptance Criterion; every named test under its name (a rename, fold or missing test is a deviation to report). New files: `src/policy.rs`, `src/audit/preview.rs`, `tests/policy_preview.rs`; small edits: `src/lib.rs` (`pub mod policy;` and root re-exports), `src/audit/mod.rs` (declare and re-export `preview`), `src/audit/category.rs` (`Category::is_block`), `fuzz/shared/audit_invariants.rs` and `tests/fuzz_layout.rs` (the preview guards and plants), `fuzz/fuzz_targets/audit.rs` only if the harness entry point needs it. `audit()` and every existing test expectation stay unchanged.
- Order: `StripSpec` with its unit tests (bits, builder) first, then `Category::is_block`, `StripPolicy` with the hand-typed grid, `may_hold`, `preview` with the kept-block grid, integration fixtures, the fuzz guards and plants, then the 22 mutants (for each: control first, apply by hand, show the named failure with its assertion text, revert, `git diff` empty).
- Type the three literal grids (removed, kept-block, may-hold) **from DEC-009 by hand**, and type the harness's copy a second time; none may be generated from the code under test.
- A doubt about a DEC-009 cell is a question for the architect: write it in the Handback and `guidance/questions.yaml`, do not edit `decisions/`.
- Run locally, because they are CI-only otherwise: `just lint-fuzz-red-proof`, `just lint-red-proof`, `just oracle-record` (empty `git status --porcelain` afterwards), and `just fuzz-smoke audit` (60 s; record executions and coverage).
- Commit on your worktree branch. **Do not push, do not open a PR, do not advance cycles.** First `git merge --ff-only claude/rust-exif-metadata-6351d5` (you need the commit containing HANDOFF-029); if refused, use `git merge` and say so.
- Do not modify irradiance, `scripts/`, `decisions/`. No dependencies. Scratch work goes in a scratchpad directory, not `/tmp`.
- You cannot see your own token count; set `tokens_total: null` with a note, the orchestrator transcribes the metered figure.

## Out of Scope

- Any byte rewriting, `EditPlan`, `sanitize()`, `strip()`; a preview of inventory-only entries; per-tag policies; rendering; the CLI
- Any ExifTool comparison (ExifTool is not run in this spec)
- Pushing, PRs, cycle advancement, SPEC-015/016

## Return Criteria — how to hand back

**Completing this handoff means filling in the `handback:` front-matter block
above AND the `## Handback` section below.** An unfilled handback is an
incomplete cycle, and `just handback-sync` will tell the orchestrator so.

On success:
1. Fill the `handback:` front-matter — **including a real `tokens_total`**.
2. Fill the `## Handback` section (the reflection is part of it, not optional).
3. Set `handoff.status` → `completed` and `handback.status` → `completed`.
4. Open a PR (build) or return your verdict (verify).

If you cannot complete the task:
1. Fill the `## Handback` section with what was done and what blocked you.
2. Set `handoff.status` → `rejected`, `handback.status` → `blocked`.
3. **Still report your token usage** — blocked work costs money too.
4. Set the spec's `task.blocked: true` and add a question to
   `/guidance/questions.yaml`.

---

## Handback

*Filled in by the receiving agent. The orchestrator does not reconstruct any of
this — it transcribes it. The reflection questions are part of completion.*

### Execution notes

- **Branch / PR:** branch `worktree-agent-a89950596c9661fe5`, no PR (not asked). `git merge --ff-only claude/rust-exif-metadata-6351d5` succeeded (fast-forward, no merge commit). Build commits: `779b906` (library, unit and integration tests), `d1be0b1` (preview guards and plants in the audit harness), `c2999be` (full-house inputs for the guards), plus the commit that fills this handback.
- **Completed at:** 2026-11-06
- **All acceptance criteria met?** yes, with the deviations listed below. No file under `decisions/`, `scripts/`, `guidance/` or irradiance is touched; no dependency added; `audit()` and every pre-existing test expectation unchanged (the 66 older seeds are byte-identical: `the_existing_66_seeds_are_byte_identical` and `the_existing_63_seeds_are_byte_identical` pass).
- **For `verify`:** n/a (build).

**Files.** New: `src/policy.rs`, `src/audit/preview.rs`, `tests/policy_preview.rs`. Edited: `src/lib.rs` (`pub mod policy;`, root re-exports `StripPolicy`, `StripSpec`), `src/audit/mod.rs` (`mod preview;` and `pub use preview::{preview, Counts, Disposition, Preview}`), `src/audit/category.rs` (`Category::is_block` and its test), `fuzz/shared/audit_invariants.rs` (four preview guards, DEC-009 typed a third/fourth time as data over names, `check` runs them), `tests/fuzz_layout.rs` (the three named tests plus a full-house builder and one premise test). `fuzz/fuzz_targets/audit.rs` needed no change (it calls `audit_invariants::check`).

**Gates run (all green, on this branch at the final build commit):**
- `cargo fmt --all -- --check` (library graph and `--manifest-path fuzz/Cargo.toml`), `rustfmt --edition 2021 --check fuzz/shared/audit_invariants.rs`: clean.
- `just lint`, `just lint-fuzz`, `just lint-no-allow`, `just lint-red-proof`, `just lint-fuzz-red-proof`, `just forbid-unsafe`, `just deny`, `just deny-fuzz`, `just validate`, `just decisions-audit` (0 structural errors, 11 scope warnings, as before), `just cost-audit`: exit 0.
- `DIAMOND_EXIF_REQUIRE_ORACLE=1 ~/.cargo/bin/cargo +1.90.0 test`: 474 passed, 0 failed; `DIAMOND_EXIF_REQUIRE_ORACLE=1 cargo test` (Homebrew stable 1.99): 474 passed, 0 failed.
- `just oracle-record`: exit 0 and `git status --porcelain` empty afterwards.
- `just fuzz-smoke audit` (60 s, nightly): clean exit, **183,720 executions in 61 s (about 3,011 exec/s at the end), cov 1989, ft 7157, corp 729 (4,211 Kb)**. It was run after the preview guards were in `check` and before the full-house test inputs were added; `fuzz/shared/audit_invariants.rs` did not change after that run.

**Named tests.** All present under their spec names. Added beyond the list: `dispositions_come_out_in_finding_order` (preview.rs unit, kills mutant 19 locally), `the_full_house_inputs_carry_every_category` (fuzz_layout premise test), `unsupported_empty_and_unreadable_inputs_preview_to_nothing_and_unreliable` (integration, public-API end to end). `policy_and_disposition_names_are_pinned` lives in `src/policy.rs` as listed and also pins the three `Disposition` names.

**The 22 mutants.** Method for each: control green first, one edit by hand (perl in-place), run `cargo test --no-fail-fast --lib --test policy_preview --test fuzz_layout`, revert with `git checkout -- <file>`, `git diff --stat` empty. "unit" = `src/` tests, "int" = `tests/policy_preview.rs`, "fuzz" = `tests/fuzz_layout.rs` (harness guard).

| # | Mutant | Caught by |
|---|---|---|
| 1 | `Gps` also contains `DeviceId` | unit `each_preset_spec_equals_its_dec_009_column` ("gps / DeviceId: DEC-009 says removed = false"), `presets_are_nested_and_everything_is_every_category`, `counts_sum_to_findings_and_residual_is_kept_plus_unread`; int `full_house_{jpeg,tiff}_matches_dec_009_for_every_preset`, `the_removed_categories_of_each_preview_equal_the_dec_009_column`, `exif_block_after_the_scan_...`, `two_exif_blocks_...`. fuzz: NOT caught at that point (no seed holds a DeviceId finding); after the full-house inputs were added the guard `preview_dispositions` catches it ("gps: finding 5: unexpected removed (want kept)") |
| 2 | `GpsAndDevice` loses `Gps` | unit `each_preset_spec_equals_its_dec_009_column` ("gps_and_device / Gps: DEC-009 says removed = true"), `presets_are_nested...`, `counts_sum...`; int 6 tests incl. `nesting_holds_on_every_committed_seed_for_every_preset`; fuzz `audit_harness_holds_on_every_seed_and_degenerate_inputs`, `preview_guards_hold_on_every_seed_and_degenerate_inputs` |
| 3 | `AllPii` loses `MakerNote` | unit `each_preset_spec_equals_its_dec_009_column` ("all_pii / MakerNote: DEC-009 says removed = true"), `disposition_rule_matches_the_kept_block_grid`, `counts_sum...`, `presets_are_nested...`; int full-house x2, removed-categories, two-blocks, after-scan; fuzz guard `preview_dispositions` ("all_pii: finding 8: expected removed, got kept_unread") via the full-house inputs |
| 4 | `AllPii` gains `Xmp` | unit `each_preset_spec_equals_its_dec_009_column` ("all_pii / Xmp: DEC-009 says removed = false"), `disposition_rule_matches_the_kept_block_grid`, `counts_sum...`, `presets_are_nested...`; int `full_house_jpeg_...`, `the_removed_categories_...`; fuzz `preview_dispositions` ("all_pii: ... unexpected removed (want kept_unread)") |
| 5 | `AllPii` gains `Software` | unit `each_preset_spec_equals_its_dec_009_column` ("all_pii / Software: DEC-009 says removed = false"), `residual_severity_is_the_max_over_non_removed_findings`, `counts_sum...`, `presets_are_nested...`; int 5 tests; fuzz `preview_dispositions` + `preview_residual_and_reliable` |
| 6 | `All` loses `Software` | unit `each_preset_spec_equals_its_dec_009_column` ("all / Software: DEC-009 says removed = true"), `residual_severity_...`, `truncated_audit_residual_severity_is_conservative`, `counts_sum...`, `presets_are_nested...`; int 6 tests incl. `all_preview_reports_only_jumbf_unread`; fuzz `preview_dispositions` ("all: finding 2: expected removed, got kept") |
| 7 | `All` gains `Icc` | unit `each_preset_spec_equals_its_dec_009_column` ("all / Icc: DEC-009 says removed = false"), `disposition_rule_matches_the_kept_block_grid`, `counts_sum...`, `presets_are_nested...`; int `all_preview_reports_only_jumbf_unread`, `full_house_jpeg_...`, `the_removed_categories_...`; fuzz `preview_dispositions` ("all: finding 13: unexpected removed (want kept)") |
| 8 | `Everything` loses `Jumbf` | unit `custom_all_equals_everything`, `each_preset_spec_equals_its_dec_009_column` ("everything / Jumbf: DEC-009 says removed = true"), `presets_are_nested...` (Everything == all()), `disposition_rule_matches_the_kept_block_grid`, 3 more; int `everything_leaves_no_residual_findings_on_any_reliable_fixture`, `nesting_holds_on_every_committed_seed...`, `full_house_jpeg_...`; fuzz `preview_nesting` ("everything leaves residual 1 / Some(\"Medium\")") |
| 9 | `may_hold(MakerNote)` loses `Gps` | unit `may_hold_equals_the_dec_009_table`, `disposition_rule_matches_the_kept_block_grid`, `counts_sum...`; int `gps_preview_of_the_full_house_reports_the_unread_blocks`, full-house x2, two-blocks, after-scan; fuzz `preview_dispositions` ("gps: finding 8: expected kept_unread, got kept") |
| 10 | `may_hold(Iptc)` gains `Gps` | unit `may_hold_equals_the_dec_009_table`, `disposition_rule_matches_the_kept_block_grid`, `counts_sum...`; int `gps_preview_...`, `full_house_jpeg_...`; fuzz `preview_dispositions` ("gps: finding 12: unexpected kept_unread (want kept)") |
| 11 | `may_hold(Icc)` non-empty (`[Gps]`) | unit `may_hold_equals_the_dec_009_table`, `disposition_rule_matches_the_kept_block_grid`, `dispositions_come_out_in_finding_order`, `counts_sum...`; int `gps_preview_...`, `all_preview_reports_only_jumbf_unread`, `full_house_jpeg_...`; fuzz `preview_dispositions` ("gps: finding 0: unexpected kept_unread (want kept)") |
| 12 | `is_block` true for `Timestamps` | unit `is_block_is_exactly_the_six_block_categories`, `may_hold_equals_the_dec_009_table` (its last loop compares `is_block` with the block rows). Not caught by int or fuzz: a tag category that `may_hold` says holds nothing cannot become `KeptUnread` through `may_hold(c).intersects`, so the mutant is behaviourally silent in `preview`; that is expected |
| 13 | disposition rule checks the wrong operand (written `may_hold(c).contains(c)` instead of `may_hold(c).intersects(spec)`; `s.contains` as worded in the spec does not type-check against a `StripSpec` argument, this is the type-correct variant) | unit `disposition_rule_matches_the_kept_block_grid`, `dispositions_come_out_in_finding_order`, `counts_sum...`; int `gps_preview_...`, `all_preview_...`, full-house x2, two-blocks, after-scan; fuzz `preview_dispositions` ("gps: finding 4: expected kept_unread, got kept") |
| 14 | residual severity ignores `KeptUnread` (`if d == Kept`) | unit `residual_severity_is_the_max_over_non_removed_findings`; fuzz `preview_residual_and_reliable` ("gps: residual severity Some(\"Low\"), want Some(\"Medium\")") |
| 15 | truncated rule dropped | unit `truncated_audit_residual_severity_is_conservative` only. The fuzz harness cannot reach it with real input: the only truncated input available (the generated 65,537-entry inventory) has no findings, so computed and conservative severity agree (both `None`). Stated honestly: the harness's rederivation of the truncated branch is exercised but not discriminating |
| 16 | truncated `all()` exception dropped | unit `truncated_audit_residual_severity_is_conservative` only (same reason as 15) |
| 17 | `reliable` constant true | unit `unsupported_empty_and_unreadable_audits_have_no_dispositions_and_are_not_reliable`, `truncated_audit_residual_severity_is_conservative`; int `unsupported_empty_and_unreadable_inputs_...`, `preview_never_panics_on_every_committed_seed`; fuzz `preview_residual_and_reliable` ("gps: reliable is true, clean && !truncated is false") |
| 18 | `kept_unread` counted as `kept` | unit `counts_sum_to_findings_and_residual_is_kept_plus_unread`; int `gps_preview_...`, `all_preview_...`; fuzz guard `preview_shape` ("gps: counts (2, 3, 0) but the dispositions tally (2, 2, 1)") |
| 19 | dispositions in reverse order | unit `dispositions_come_out_in_finding_order`, `preview_is_deterministic`; int 7 tests incl. full-house and two-blocks; fuzz `preview_dispositions` + `preview_residual_and_reliable` |
| 20 | two categories share one bit (`Icc` takes `Software`'s bit) | unit `strip_spec_has_one_distinct_bit_per_category`, `strip_spec_categories_iterate_in_category_all_order`, `each_preset_spec_equals_its_dec_009_column` ("all / Icc: DEC-009 says removed = false"), `may_hold_equals_the_dec_009_table`, 3 more; int `all_preview_...`, `full_house_jpeg_...`, `the_removed_categories_...`; fuzz `preview_dispositions` |
| 21 | `exclude` is a no-op | unit `strip_spec_none_all_include_exclude_contains_len`, `truncated_audit_residual_severity_is_conservative` (its `all().exclude(Icc)` case) |
| 22 | two policy names swapped (`all` and `all_pii`) | unit `policy_and_disposition_names_are_pinned` |

The harness's guards also have the 17 plants in `preview_plants()` (each guard has at least 2, both directions of every iff: missing/unexpected `removed`, missing/unexpected `kept_unread`, `reliable` flipped each way, residual severity `None` vs `Some` each way, removed shrinking, residual growing, Everything leaving a finding or a severity, a count disagreeing with the tally, residual not kept plus unread, a missing disposition, a reordered list). Each plant has a control (the unplanted answer passes all four guards) and is rejected by its guard run alone.

### Cost self-report

- **Tokens (total):** null. A sub-agent cannot read its own meter; the orchestrator transcribes the metered figure for this agent run.
- **Estimated USD:** null
- **Duration (minutes):** not measured by me (roughly an hour of wall time, a guess, not a reading)
- **Source of the number:** none available to the executing agent

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:**
  1. **Seed counts.** The spec says "29 TIFF, 37 JPEG, the audit seeds". The committed directories hold 32 TIFF, 37 JPEG and 76 audit seeds (`tests/audit_model.rs` already asserts `[32, 37, 76]`; the toolchain brief's "29 TIFF ... 73" is stale). My tests (`nesting_holds_on_every_committed_seed_for_every_preset`, `preview_guards_hold_on_every_seed_and_degenerate_inputs`) assert 32 / 37 / 76.
  2. **An added test and an added non-listed integration test**, listed above; no named test was renamed, folded or dropped.
  3. **`Preview::policy()` stores the `StripPolicy` it was computed with**, a one-field addition to the sketched struct (the signature in the spec is unchanged).
  4. **`Custom` spec in the harness** is chosen from a 16-bit rolling hash of the input bytes (rotate-left 5 then xor each byte, seed 0x5A5A), mapping bit i to the i-th name in the harness's own category list. One custom case per input.
  5. **Mutant 13 wording**: see the table; implemented as `may_hold(c).contains(c)`.
  6. **Mutant 1 and 3 in the fuzz harness**: the committed seeds contain no `DeviceId` and no `MakerNote` finding, so the seeds alone did not catch them. I added two hand-built full-house inputs (a TIFF and a JPEG, built in `tests/fuzz_layout.rs` from raw bytes) to `preview_guards_hold_on_every_seed_and_degenerate_inputs` and `the_full_house_inputs_carry_every_category`; the libFuzzer target itself still starts from the committed seeds (I did not add seeds: the count 76 is asserted elsewhere and "the existing 66 seeds byte-identical" must hold). A real fuzz run reaches DeviceId/MakerNote by mutating tag ids, as the 60 s smoke does for other categories.
  7. **The harness is not discriminating for the truncated branch of rule 4** (mutants 15, 16): only the unit test kills them. See table.
  8. `Disposition`, `Counts` and `Preview` are re-exported from `exif_core::audit` (including `audit::preview` as a function name next to the private module `preview`); the module itself is private, as `category` and `sniff` are.
- **Doubts about a DEC-009 cell:** none that change a cell. Two observations for the architect, neither is a request to edit the DEC: (a) `Thumbnail` is listed in `AllPii` as removed, but a thumbnail finding is also created by an IFD1 with `StripOffsets` and no `0x0201` (SPEC-012); the preview treats it the same, which is what the DEC implies; (b) under `Custom({Software})` an `Xmp` finding is `KeptUnread` and an `Iptc` finding is `Kept`, which follows the may-hold table but may surprise a reader; the CLI wording in SPEC-015 should say "may hold".
- **Follow-up work identified:** none beyond SPEC-015/016. Optional: add a DeviceId/MakerNote seed to `fuzz/seeds/audit` in a later spec that is allowed to move the seed count.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Mutant 13's wording (`s.contains` where `intersects` is meant) does not type-check; I used the closest compiling variant. The stale seed counts (29/73 vs 32/76) cost a minute.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — Nothing blocking. It would help to say that a worktree-isolated agent's shell refuses compound commands (heredocs, `&&` chains containing git); I wrapped the mutant loop in a script file.

3. **If you did this task again, what would you do differently?**
   — Build the full-house fuzz inputs before the first mutant run instead of after seeing mutants 1 and 3 pass the harness, and write the truncated-audit unit tests first, since they are the only killers of mutants 15 and 16.
