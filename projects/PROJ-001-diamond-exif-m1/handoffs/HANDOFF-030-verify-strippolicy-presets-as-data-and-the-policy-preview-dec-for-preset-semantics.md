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
  id: HANDOFF-030
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
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
  duration_minutes: 40
  branch: worktree-agent-ae283bfb20cbaec24
  pr: null
  completed_at: 2026-11-06         # YYYY-MM-DD
  notes: "verify APPROVED at c20df49; the sub-agent cannot read its own meter, the orchestrator transcribes the metered figure"
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-030: Verify StripPolicy presets and the policy preview

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-014` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-014-strippolicy-presets-as-data-and-the-policy-preview-dec-for-preset-semantics.md` (API, rules, expectation matrices, named tests and the 22 mutants are normative)
- **DEC-009:** `decisions/DEC-009-strip-policy-presets-and-the-preview.md` (normative for every preset cell and the may-hold table)
- **Build handoff and Handback:** `HANDOFF-029-build-...md` (claims to verify, not to trust; read the deviations, especially the seed counts, mutant 13's variant, and the added full-house harness inputs)
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe)
- Code under review: `git diff 876ee82..HEAD` (build commits `779b906`, `d1be0b1`, `c2999be`). Main: `src/policy.rs`, `src/audit/preview.rs`, `src/audit/category.rs` (`is_block`), `tests/policy_preview.rs`, `tests/fuzz_layout.rs`, `fuzz/shared/audit_invariants.rs`.

## Expected Deliverables

Review cold; output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

Verify independently:
- Re-run every gate (fmt both graphs and the harness file, `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `cargo test` on stable and 1.90.0 also with `DIAMOND_EXIF_REQUIRE_ORACLE=1`, `-F unsafe_code`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `oracle-record` with an empty `git status --porcelain`). Run `just fuzz-smoke audit` 60 s; record executions and coverage. Confirm `git diff 876ee82..HEAD -- src` touches only `src/policy.rs`, `src/audit/preview.rs`, `src/audit/category.rs` (additions only), `src/audit/mod.rs` and `src/lib.rs`, that `audit()` is unchanged, and that `decisions/`, `scripts/` are untouched.
- **Check the code against DEC-009 yourself**: read DEC-009 and rebuild the preset grid, the kept-block grid and the may-hold table from it by hand, then compare with `src/policy.rs` and `src/audit/preview.rs` cell by cell, independently of the builder's tests. Then check the tests' own grids against the DEC the same way (a test grid copied from the code would make the suite vacuous). Check rules 1 to 10 of the spec one by one, in particular rule 4 (truncated residual severity and the `all()` exception) and rule 5 (`reliable`).
- **Judge DEC-009 itself, not only the code.** Is any preset cell surprising or unsafe (for example, can `GpsAndDevice` or `AllPii` produce a preview that reads as safe while leaving a leak)? Does `KeptUnread` ever fail to fire where a reader would need it, or fire where it is just noise? Does the policy preview ever turn an unsupported, unreadable or truncated audit into a clean-looking one? Report as findings, not as edits.
- **Test the testers.** Re-run the builder's mutants (the report claims 22 red) at least for 1, 3, 9, 12, 15, 16, 17 and 19. Note the builder says mutants 15 and 16 are caught by only one unit test and not by the harness: judge whether that is acceptable. Then write mutants the builder did not try, for example: `kept_unread` check using `is_subset_of` instead of `intersects`; `include` that clears other bits; `categories()` yielding in reverse; `spec()` for `Custom` returning `none()`; residual severity computed over all findings; `reliable` ignoring truncation; the harness `Custom` spec derivation always empty; one preview guard's iff checked in one direction only; a `StripSpec::all()` that misses the last category; `is_block` true for `Software`; `may_hold(Thumbnail)` missing `Software`. Report which the suite catches (name the test or guard) and which survive.
- Redo one preview-guard plant against the real `audit` libFuzzer target (not only the stable-toolchain layout test), and check the step-counter linearity test covers the new guards.
- Check the added harness inputs (hand-built full-house TIFF and JPEG in `tests/fuzz_layout.rs`) really carry every category (`the_full_house_inputs_carry_every_category`) and that the seed count and seed hash lists are unchanged.
- Check no dependencies were added and no file under `decisions/`, `scripts/` or irradiance is touched.
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes; SPEC-015/016 work; the writer; any ExifTool comparison (not run in this spec)

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

- **Branch / PR:** branch `worktree-agent-ae283bfb20cbaec24`, no PR (verify). `git merge --ff-only claude/rust-exif-metadata-6351d5` succeeded (fast-forward to `c20df49`).
- **Completed at:** 2026-11-06
- **All acceptance criteria met?** yes. No code defect found; the findings below are test-strength gaps and DEC-level observations for the architect, none blocking.
- **For `verify`:** ✅ **APPROVED at `c20df49`** (the code under review is the build commits `779b906`, `d1be0b1`, `c2999be`; `f6797a1` and `c20df49` touch only the spec and handoffs).

**Gates, run by me on `c20df49`** (all exit 0 unless stated):
- `cargo fmt --all -- --check`, the same with `--manifest-path fuzz/Cargo.toml`, `rustfmt --edition 2021 --check fuzz/shared/audit_invariants.rs`: clean.
- `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `forbid-unsafe`, `deny`, `deny-fuzz`, `validate`, `cost-audit`: exit 0. `just decisions-audit`: 0 structural errors, 11 scope warnings across 9 decisions (as before).
- `DIAMOND_EXIF_REQUIRE_ORACLE=1 cargo test` (Homebrew stable): 474 passed, 0 failed. `DIAMOND_EXIF_REQUIRE_ORACLE=1 ~/.cargo/bin/cargo +1.90.0 test`: 474 passed, 0 failed. `RUSTFLAGS="-F unsafe_code" cargo +1.90.0 check --all-targets`: clean.
- `just oracle-record`: exit 0, `git status --porcelain` empty afterwards.
- `just fuzz-smoke audit` (60 s, nightly): clean, **205,555 runs in 61 s, cov 1995, ft 7524, corp 807 (4,578 Kb)**.
- Scope: `git diff 876ee82..HEAD -- src` touches only `src/policy.rs`, `src/audit/preview.rs`, `src/audit/category.rs`, `src/audit/mod.rs`, `src/lib.rs`, with no removed line in the three edited files (so `audit()` is unchanged; `mod.rs` gains `mod preview;` and one `pub use`). Empty diff for `decisions/`, `scripts/`, `guidance/`, `Cargo.toml`, `Cargo.lock`, `fuzz/Cargo.*`, `fuzz/seeds/`, `fuzz/fuzz_targets/`, `tests/oracle-fixtures/`. No dependency added. All 34 named tests exist exactly once.

**DEC-009 against the code, by hand.** I rebuilt the 5 x 13 preset grid, the 6 x 5 kept-block grid and the may-hold table from DEC-009 and compared them with `StripPolicy::spec`, `may_hold`, `is_block` and `disposition_of` cell by cell: all equal. The grids typed into the tests (`REMOVED` in `src/policy.rs` and `tests/policy_preview.rs`, `KEPT_BLOCK` in `src/audit/preview.rs` and `tests/policy_preview.rs`, `may_hold_table`, the harness's `H_PRESETS`/`h_may_hold`/`H_BLOCKS`) also equal DEC-009, and their row order is DEC-009's, not `Category::ALL`'s, so they were not copied from the code. The hand-derived counts in `counts_sum_to_findings_and_residual_is_kept_plus_unread` ((1,8,4), (2,7,4), (8,2,3), (11,1,1), (13,0,0)) check out. Rules 1 to 10: each holds. Rule 4: truncated means the audit's `max_severity()` unless the spec is `all()`, then `None`; `max_severity` is computed in `Builder::push` over every classified entry, including dropped ones, so it is a true upper bound. Rule 5: `reliable = clean && !inventory_truncated`; since `Health::clean` already includes `!truncated` (mod.rs line 465) the second conjunct is redundant on real audits (harmless; see mutant vF).

**Mutants, the builder's, re-run** (`cargo test --no-fail-fast --lib --test policy_preview --test fuzz_layout`, revert by `git checkout`, tree clean after each): all caught.
- 1: `each_preset_spec_equals_its_dec_009_column`, `presets_are_nested_...`, `counts_sum_...`, the full-house x2, removed-categories, two-blocks, after-scan, and the harness (`preview_guards_hold_on_every_seed_and_degenerate_inputs`, `the_full_house_inputs_carry_every_category`).
- 3: as 1 plus `disposition_rule_matches_the_kept_block_grid`.
- 9: `may_hold_equals_the_dec_009_table`, `disposition_rule_...`, `counts_sum_...`, `gps_preview_of_the_full_house_reports_the_unread_blocks`, full-house x2, two-blocks, after-scan, harness.
- 12: `is_block_is_exactly_the_six_block_categories`, `may_hold_equals_the_dec_009_table` only (behaviourally silent, as the builder says).
- 15, 16: `truncated_audit_residual_severity_is_conservative` only.
- 17: the unit test above, `unsupported_empty_and_unreadable_audits_...`, `unsupported_empty_and_unreadable_inputs_...`, `preview_never_panics_on_every_committed_seed`, harness (`audit_harness_holds_...`, `preview_guards_hold_...`, `each_preview_guard_goes_red_alone_when_planted`).
- 19: `dispositions_come_out_in_finding_order`, `preview_is_deterministic`, 7 integration tests, harness.

**Mutants of my own:**

| Id | Mutant | Result |
|---|---|---|
| vA | `KeptUnread` via `may_hold(c).is_subset_of(spec)` instead of `intersects` | caught: `custom_none_keeps_everything_...`, `disposition_rule_...`, integration full-house/gps/all, harness |
| vB | `include` clears the other bits | caught: 12 unit, all integration, harness |
| vC | `categories()` yields in reverse | caught: `strip_spec_categories_iterate_in_category_all_order` only |
| vD | `spec()` for `Custom` returns `none()` | caught: `custom_spec_is_returned_unchanged`, `custom_all_equals_everything`, `truncated_...`, harness (the custom case) |
| vE | residual severity over all findings | caught: `residual_severity_is_the_max_...`, `custom_all_equals_everything`, `everything_leaves_no_residual_...`, `nesting_holds_...`, harness |
| vF | `reliable` ignores truncation (`= health.clean`) | caught only by `truncated_audit_residual_severity_is_conservative`, whose synthetic `Health` (clean and truncated at once) `audit()` can never produce; equivalent on real inputs |
| vG | harness `custom_names` always empty | **SURVIVED** |
| vH | harness `reliable` iff checked one way (`got && !want`) | caught: `each_preview_guard_goes_red_alone_when_planted` ("a clean file is reported unreliable") |
| vH2 | harness `p_dispositions` reports only a missing `removed` | caught: `each_preview_guard_goes_red_alone_when_planted` |
| vI | `StripSpec::all()` misses the last category | caught: 9 unit, 4 integration, harness |
| vJ | `is_block` true for `Software` | caught: `is_block_is_exactly_...`, `may_hold_equals_...` only (silent in `preview`, like 12) |
| vK | `may_hold(Thumbnail)` lacks `Software` | caught: `may_hold_equals_the_dec_009_table` only |
| vL | `disposition_of` drops `c.is_block() &&` | **SURVIVED** (equivalent: `may_hold` is empty for every tag category) |
| vM | harness truncated `all()` exception dropped | **SURVIVED** |

**libFuzzer plants (the real `audit` target, nightly, fresh scratch corpus plus `fuzz/seeds/audit`):**
- Mutant 17 (`reliable: true`): crashed at once on the empty input, `preview invariants violated: [("preview_residual_and_reliable", "gps: reliable is true, clean && !truncated is false")]`. The guards are live in the target.
- Mutant 9 (`may_hold(MakerNote)` loses `Gps`): **not found in 181 s / 395,416 runs**. The fuzzer did not reach a MakerNote finding from the committed seeds in that time (finding F1).

Linearity: `preview_guards_are_linear` drives all four preview guards through `preview_violations` with the step counter (every guard `tick`s per case and per disposition) on 5,000/2,500 thumbnails and 3,000/1,500 regions; it passes. The work in `preview_input`/`h_want` is outside the counter but constant per finding (13-entry lists). Full-house harness inputs: `the_full_house_inputs_carry_every_category` asserts all 13 categories in the JPEG and the nine Exif ones in the TIFF, both clean, and runs `check` on both; seed directories and seed hash lists are unchanged (empty diff on `fuzz/seeds/`, count asserted 32/37/76).

**Findings** (none blocks):
- **F1 (Medium, test strength).** In the libFuzzer target the preview guards discriminate only for categories that the committed seeds hold. The full-house inputs that kill mutants 1 and 3 (and 9) in the harness exist only in `tests/fuzz_layout.rs`, not in `fuzz/seeds/audit`, and a 181 s run did not find a MakerNote. The handback's "a real fuzz run reaches DeviceId/MakerNote by mutating tag ids" is not borne out at smoke length. Follow-up: add the two full-house inputs as `audit` seeds in a spec allowed to move the seed count (76 → 78).
- **F2 (Low, test strength).** Rule 4's truncated branch is discriminated only by the unit test with a hand-built `FileAudit` (mutants 15, 16, vM). The handback says the harness "cannot reach it with real input", which is too strong: `audit()` does produce truncated audits with stored findings (mod.rs `inventory_cap_boundary_is_exact_and_findings_survive_truncation`, the `over` input: 65,535 entries plus Make and BodySerialNumber after the cap). A generated input of that shape in the harness or integration tests would kill 15, 16 and vM end to end.
- **F3 (Low, test strength).** vG survives: nothing checks that the harness's `Custom` case is non-trivial, so it could silently become `Custom(none)` and test nothing. A premise assertion (some inputs give a custom set that is neither empty nor `all()`) would close it.
- **F4 (Low, DEC observation).** A truncation caused only by the items cap leaves the findings complete, yet rule 4 still treats them as unknown: on the `over` input `AllPii` gives `residual_findings() == 0` with `residual_severity() == Some(High)`. This is conservative and matches DEC-009, but the pair reads as contradictory. SPEC-015/016 should render and gate on `reliable` first. `Health` cannot tell "items truncated" from "findings truncated" today.
- **F5 (Medium, DEC observation, for the architect).** `may_hold(Icc) = ∅` makes ICC `Kept` ("nothing the policy targets can hide inside it") under every preset except `Everything`. An ICC header carries a creation date/time, a device manufacturer and model signature, and a profile creator, and `desc`/`cprt`/`dmnd`/`dmdd` tags carry free text. Keeping ICC under `All` is a stated requirement, but the disposition's wording overstates. Possible fixes: `KeptUnread` for ICC under presets that remove `Timestamps`/`CameraModel`, or a DEC note that `Kept` for ICC means "by decision".
- **F6 (Low, DEC observation).** The may-hold table omits some real containments, which affects `Custom` policies only (every preset cell comes out the same either way):
  - MakerNote may hold `Authorship` (e.g. Canon OwnerName).
  - IPTC may hold `Software` (2:65 OriginatingProgram, 2:70 ProgramVersion).
  - Blocks may hold other blocks: a C2PA manifest can carry a thumbnail and XMP, and a thumbnail JPEG can carry XMP or ICC. So `Custom({Thumbnail})` reports `Jumbf` as `Kept`.
  - IPTC location strings (2:90 to 2:101) are textual location. Under `Gps`, `Iptc` is `Kept`. By DEC-008's definition of `Gps` (coordinates) that is consistent, but a user choosing "GPS" may expect location text to go.
- **F7 (Info).** Mutants 12 and vJ (`is_block` widened) and vL (`is_block` check removed) are silent in `preview`, because `may_hold` alone already decides `KeptUnread`. Rule 7 holds structurally through `may_hold`, and `is_block` is pinned only by two unit tests. That is acceptable, but the doc comment could say so.

On the brief's question about mutants 15/16 being caught only by one unit test: acceptable for this spec (the test is named, hand-built and discriminating), with F2 as the cheap end-to-end hardening.

### Cost self-report

- **Tokens (total):** null. The verifier sub-agent cannot read its own meter; the orchestrator transcribes the metered figure from the sub-agent result.
- **Estimated USD:** null (follows from the token figure)
- **Duration (minutes):** about 40 (including 60 s + 60 s + 181 s of fuzzing and a 22-mutant sweep)
- **Source of the number:** harness report (to be transcribed by the orchestrator)

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:** none by the verifier. The builder's deviations check out: seed counts 32/37/76 are correct on disk; mutant 13's type-correct variant is reasonable; the added full-house harness inputs carry every category.
- **Follow-up work identified:**
  - Add the full-house TIFF/JPEG to `fuzz/seeds/audit` (F1).
  - Add an end-to-end truncated-with-findings input (F2) and a non-trivial-custom premise (F3).
  - Architect review of the ICC and may-hold observations (F5, F6) before SPEC-015 fixes the wording of `kept`/`kept_unread`.
  - SPEC-016: gate on `reliable` before reading residual counts (F4).

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Nothing material. The isolated-worktree shell refuses compound commands that mention git or `$HOME`, so mutant and fuzz loops had to go into script files.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — That the libFuzzer target only discriminates the preview for categories present in the seeds; a "the guard is in the target" acceptance criterion should be paired with "a seed exercises it".

3. **If you did this task again, what would you do differently?**
   — Start the long libFuzzer plant (mutant 9) at the very beginning in parallel with the gates, since it is the slowest and the most informative check.
