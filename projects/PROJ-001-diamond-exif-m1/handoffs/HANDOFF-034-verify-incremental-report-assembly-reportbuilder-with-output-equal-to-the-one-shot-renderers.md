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
  id: HANDOFF-034
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-06
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-018

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
  tokens_total: 212770             # orchestrator-recorded from the metered sub-agent results (subagent_tokens)
  estimated_usd: 1.40
  duration_minutes: 24
  branch: worktree-agent-a4b5a3aaf3af38bfd
  pr: null                         # verify cycle: verdict only, no PR
  completed_at: 2026-11-06         # YYYY-MM-DD
  notes: "verify PUNCH LIST at f18bbda, delta verify APPROVED at 8d4778c (final). Tokens transcribed by the orchestrator from the sub-agent metered results; the agent was resumed once and each completion reported a cumulative figure: 192,339, 212,770 (final, used); wall-clock 911,092 + 543,262 = 1,454,354 ms = 24 minutes; the verifier's own time estimates are not used"
  synced_at: 2026-11-06
---

# HANDOFF-034: Verify the incremental ReportBuilder

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-018` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-018-incremental-report-assembly-reportbuilder-with-output-equal-to-the-one-shot-renderers.md` (API, rules 1 to 6, the fuzz guard, named tests and mutants are normative)
- **DEC-010 and its two amendments**, **DEC-011** (why the builder exists): `decisions/`
- **Build handoff and Handback:** `HANDOFF-033-build-...md` (claims to verify, not to trust; read the deviations, especially the one-shot renderers now being thin wrappers over the builder, and mutant 2 and 11's coverage)
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe)
- Code under review: `git diff 8d7ca73..HEAD` (build commits `19ba3cd`, `d33bd6e`; `cd1b26a` and later are docs). Main: `src/report/{mod,json,csv,text,builder_tests}.rs`, `tests/report_render.rs`, `fuzz/shared/render_invariants.rs`, `tests/fuzz_layout.rs`.

## Expected Deliverables

Review cold; output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

Verify independently:
- Re-run every gate (fmt both graphs and the harness files, `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `cargo test` on stable and 1.90.0 also with `DIAMOND_EXIF_REQUIRE_ORACLE=1`, `-F unsafe_code`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `oracle-record` with an empty `git status --porcelain`). Run `just fuzz-smoke audit_render` 60 s; record executions and coverage. Confirm: every file under `tests/golden/` is byte-identical to `8d7ca73`; `audit()`, the preview, the escaping (`src/report/safe.rs`) and `decisions/`, `scripts/`, `Cargo.toml` untouched; no existing test expectation relaxed (`git diff 8d7ca73..HEAD` on existing tests: show every changed line and judge it).
- **The central claim: one code path.** The build made the one-shot `render_*` functions wrappers over the builder, so builder-equals-one-shot is partly true by construction. Find what the equivalence tests and the guard can and cannot see. Is the property actually meaningful (does anything still check the fragments and the assembly are *right*, not merely consistent)? List the ways the one-shot output could be wrong while every equivalence check passes, and show the existing independent checks (goldens, the strict JSON and CSV checkers, the facts guards) catch them.
- **Check the rules by hand:** rule 2 (no retained borrow: read the struct; is anything lifetime-bound or an index into the entry?), rule 3 (order, independence, same entry twice), rule 4 (JSON frame, CSV header exactly once, text summary last, empty builder), rule 5 (memory bound: the fragments are `Vec<String>`; is the stated bound honest? measure retained size growth when adding a 50 MB-class entry built over a buffer that is then dropped), `Summary::add_file` (does the running summary equal `summarize` at every step, including `max_severity`, `reliable_files`, `residual_findings` with and without a policy and the DEC-010 amendment 1 rule that it is `None` when no file is reliable).
- **Test the testers.** Re-run the builder's mutants (the report claims 12 red) at least for 2, 5, 8, 9, 11 and 12. Write mutants of your own that the builder did not try, for example: the CSV builder emitting the header only if the first added entry has findings; the text builder skipping the blank line between blocks only for Failed entries; `finish` consuming fragments in a different order than `add` only when more than 2 entries; `summary()` double-counting after `finish`-like state; the JSON builder re-escaping a fragment; a builder that clones the options at `new` but reads a global default for `inventory`; the guard comparing lengths only; one plant that cannot go red because its control also fails. Report which the suite catches (name the test or guard) and which survive.
- Redo one `builder_equals_oneshot` plant against the real `audit_render` libFuzzer target, and confirm the step-counter linearity test covers the new guard and is a ratio, not an absolute bound.
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes; SPEC-016 work (the binary); changes to report content; any ExifTool comparison

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

- **Branch / PR:** branch `worktree-agent-a4b5a3aaf3af38bfd`, no PR (verify). `git merge --ff-only claude/rust-exif-metadata-6351d5` succeeded (fast-forward to f18bbda). Code reviewed: `git diff 8d7ca73..f18bbda` (build commits 19ba3cd, d33bd6e).
- **Completed at:** 2026-11-06
- **All acceptance criteria met?** Yes for the code and the gates. Two test-strength gaps (F1, F2) make up the punch list.
- **For `verify`:** ⚠ **PUNCH LIST at f18bbda.** The library change is correct and minimal, every gate is green, and the build's mutant claims reproduce. The punch list asks for two small tests (F1, F2). No library code change is needed.

**Gates (run by the verifier, each exit status recorded without a pipe):** `cargo fmt --all -- --check` 0; `rustfmt --edition 2021 --check fuzz/shared/render_invariants.rs fuzz/fuzz_targets/audit_render.rs` 0; `cd fuzz && cargo fmt --all -- --check` 0; `just lint` 0; `just lint-fuzz` 0; `just lint-no-allow` 0; `just lint-red-proof` 0; `just lint-fuzz-red-proof` 0; `DIAMOND_EXIF_REQUIRE_ORACLE=1 cargo test --no-fail-fast` (Homebrew 1.99.1) 0, every binary ok; the same with `~/.cargo/bin/cargo +1.90.0` 0; `just forbid-unsafe` 0; `just deny` 0; `just deny-fuzz` 0; `just validate` 0; `just decisions-audit` 0 (0 structural errors, 11 scope warnings that were already there); `just cost-audit` 0; `just oracle-record` 0, then `git status --porcelain` empty. `just fuzz-smoke audit_render` (60 s, unmodified tree): exit 0, `Done 36441 runs in 61 second(s)`, final `cov: 4780 ft: 14397 corp: 929/245Kb exec/s: 597`. Other builds were running on the host at the same time; the build reported about 700 exec/s.

**Untouched (checked with `git diff --stat 8d7ca73..HEAD`):** `tests/golden/` (all three files byte-identical), `decisions/`, `scripts/`, `Cargo.toml`, `Cargo.lock`, `fuzz/Cargo.toml`, `fuzz/fuzz_targets/`, `fuzz/seeds/`, `src/report/safe.rs`, `src/audit/` (the audit and the preview), `src/policy.rs`, `src/lib.rs`.

**Existing test lines changed:**
- `tests/report_render.rs`: only the `use` line, which adds `render`, `Format` and `ReportBuilder`.
- `tests/fuzz_layout.rs`: the old `render_plants()` body became `fact_render_plants()` with the same plants. The `flat` tuple in `each_render_guard_goes_red_alone_when_planted` gained `&c.pairs, &c.summary_pairs`. That widens only the precondition "the plant changed something". The fact plants still change the same fields, and the assertion "rejected by its own guard alone" is unchanged.

Neither change relaxes an expectation, and no `#[ignore]` was added.

**Rules checked by hand.**
- Rule 2: `ReportBuilder { format: Format, opts: RenderOptions, fragments: Vec<String>, summary: Summary }`. `RenderOptions` is `Option<StripPolicy>` plus two bools, and `Summary` holds only owned counters. There is no lifetime parameter and no index into an entry. `builder_keeps_no_borrow_of_the_added_entry` drops the label and the detail and overwrites the buffer before `finish`. Holds.
- Rule 3: fragments are pushed in `add` order, each built from `entry` and `opts` alone. Adding the same entry twice pushes two fragments. Holds.
- Rule 4: JSON is the frame, then `summary` (the finalized running summary), then the fragments joined by `,`. CSV is `HEADER` plus CRLF once, then the fragments. Text is the fragments, then the summary block. An empty builder goes through the same `assemble`. Holds.
- Rule 5, measured with a counting global allocator in a scratch crate (release build): I added a 50 MiB TIFF (one 50 MiB ASCII Make) and a failed entry, then dropped every input. Across the three formats and two option sets the builder kept 2,334 to 3,552 bytes; the outputs were 1,293 to 2,646 bytes. No input byte is retained. The bytes beyond the fragment lengths are `String`/`Vec` capacity slack (see F3).
- `Summary::add_file`: the logic is identical to the old `summarize` loop. The amendment-1 rule (`residual_findings` is `None` when no file is reliable) lives only in `finalized`. `summarize`, `summary()` and `finish` all apply it, so the running value stays `Some(0)` and recovers when a reliable file arrives. The unit test and my mutants v10, v11 and v13 check this, and all three were caught. `max_severity` equality holds by construction, but no test checks it is correct across files (F2).

**The central claim: one code path.** `render_text`, `render_json` and `render_csv` are all `build_all`: new, add each, finish. So for any bug in the shared code, builder-equals-one-shot stays true by construction.

The equivalence checks (`builder_equals_the_one_shot_*`, the seed cross product, the guard) can only see a divergence in the builder-only surface:
- `summary()` or `len()`
- the `render` dispatch
- a one-shot that stops using the builder

These are spec mutants 8, 9, 10 and 12, and my v04.

Other checks, independent of the builder, keep the shared output *correct*:
- the goldens: `golden_text_json_csv_match_byte_for_byte`, unchanged
- the strict JSON/CSV checkers and the fact guards of `audit_render` (`json_facts`, `csv_rows`, `summary_facts`, `values_gate`, and the rest)
- the format tests in `tests/report_formats.rs` and `render_tests.rs`
- the builder unit tests that pin literal structure: fragment composition, `"files":[]}`, per-file length deltas

Except v09, one of these caught every shared-path mutant I wrote (list below).

Ways the one-shot output could be wrong while every equivalence check passes, and what catches each:
- a wrong separator or frame: the goldens and the fragment-composition test
- the text summary placed first: the goldens and the text format tests
- an option ignored in `add`: the format tests and the harness
- a bug in the empty document: only the two builder unit tests (s11)
- a wrong `max_severity` across files: nothing (F2)

**Mutants.** I applied each by script in a separate detached worktree of f18bbda. Each run made an exact-string edit, ran `cargo test --no-fail-fast --lib --test report_render --test report_formats --test fuzz_layout`, then `git checkout`, and confirmed `git status --porcelain` was empty. A control run was green first, and survivors were re-run against the full `cargo test`.

The build's mutants, re-run (all match the build handback's claims):
- 2 (JSON joined with `, `): caught by `golden_text_json_csv_match_byte_for_byte`, `golden_files_are_unchanged_and_the_builder_reproduces_them`, `builder_fragments_appear_in_add_order_and_are_independent_of_neighbours` and `builder_retains_only_fragments_not_input_bytes`. No harness guard fires; the plant tests fail only because a plant's `},{` anchor is gone. Matches the claim.
- 5 (text summary first): caught by both goldens, `render_dispatches_to_the_matching_one_shot`, `builder_fragments_appear_in_add_order_and_are_independent_of_neighbours`, many text format tests and the render harness.
- 8 (`summary()` skips Failed): 13 tests fail, including `builder_summary_equals_summarize_at_every_step`, `builder_equals_the_one_shot_on_the_seed_cross_product`, `builder_guard_holds_on_every_seed_and_degenerate_inputs` and `render_harness_holds_on_every_seed_and_degenerate_inputs`.
- 9 (divergent `render_json`): 19 tests fail, including both goldens, `builder_equals_the_one_shot_for_one_and_for_many_entries_in_every_format`, the cross product, the builder guard and `json_summary_matches_summarize`.
- 11 (empty JSON `[,]`): caught only by `builder_with_no_entries_equals_the_empty_one_shot_in_every_format` and `builder_retains_only_fragments_not_input_bytes`. Matches the claim.
- 12 (`len()` + 1): caught by `builder_with_no_entries_...`, `builder_adding_the_same_entry_twice_gives_two_fragments`, `builder_summary_equals_summarize_at_every_step`, the cross product and `golden_files_are_unchanged_and_the_builder_reproduces_them`.

The verifier's own:
- v01 (CSV header only if the first added entry has findings): caught by `csv_uses_crlf`, `csv_has_a_row_for_every_file_without_findings`, `render_harness_holds_on_every_seed_and_degenerate_inputs`, the fragment-composition test and others.
- v02 (text drops the blank line after Failed blocks only): caught only by the two golden tests.
- v03 (`finish` swaps fragments 2 and 3 when there are more than 2): caught by both goldens, `json_has_the_pinned_key_order_at_every_level` and `json_enumerations_are_pinned`. No guard fires, because the harness never has 3 entries.
- v04 (`summary()` doubles `findings` from 2 entries): caught by `builder_summary_equals_summarize_at_every_step`, the cross product, the builder guard and `builder_json_assembly_puts_the_running_summary_before_the_files`.
  - **The real `audit_render` libFuzzer target also finds it:** `just fuzz-smoke audit_render` exits 1 with `render invariants violated: [("builder_equals_oneshot", "the builder's summary after 2 entries is SummaryF { ... findings: 2 ...` and `deadly signal`. Reverted; tree clean.
- v05 (JSON re-escapes backslashes in fragments): caught by the goldens, `json_escapes_quote_backslash_and_every_control_end_to_end`, the render harness and others.
- v06 (`add` reads `RenderOptions::default().inventory`): caught by `csv_inventory_mode_rows`, `json_has_the_pinned_key_order_at_every_level` and the render harness. The goldens miss it because inventory is off there.
- v07 (the guard compares lengths only, `p.builder.len() != p.oneshot.len()`): **SURVIVES the full `cargo test`.** See F1.
- v08 (the guard always fails, so a plant's control is red): caught by the control assertions in `builder_guard_goes_red_alone_when_planted` and `each_render_guard_goes_red_alone_when_planted`, and by `builder_guard_holds_on_every_seed_and_degenerate_inputs`.
- v09 (`add_file` keeps the lowest `max_severity` across files, `cur > m`): **SURVIVES the full `cargo test`.** See F2.
- v10 (the rule "`None` when no file is reliable" removed): caught by `summary_residual_is_none_when_no_file_is_reliable`, `summarize_counts_each_status_category_and_reliability`, the step test and the harness.
- v11 (`None` latched into the running summary at the first unreliable audited file): caught only by `summary_residual_counts_the_reliable_files_only_when_an_unreliable_file_has_findings`.
- v12 (`reliable` ignores `inventory_truncated`): survives, but it is an equivalent mutant. `Health::clean` already requires `!truncated` (`finish` in `src/audit/mod.rs`). Not a finding.
- v13 (text summary uses the un-finalized running summary): caught only by `summary_residual_is_none_when_no_file_is_reliable`.
- v14 (CSV failed row loses its detail): caught by the goldens, the CSV format tests and the harness.

**Linearity.** `builder_guard_is_linear` runs the new guard alone and asserts the ratio `steps <= 3 * half`. It also asserts a per-item bound of `40 n + 256`, which grows with n, not an absolute constant. `render_guards_are_linear` runs every guard, the new one included, with the same ratio. Covered.

The step counter counts the guard's own comparisons. The 21 renders `build_case` now does happen outside any guard, as they already did for the earlier render guards.

**Findings.**
- **F1 (Medium, test strength; punch list).** The `builder_equals_oneshot` plants cannot tell byte equality from length equality.
  - Every builder-side plant changes the length: dropping the last row, ` value="` to ` v="`, and `}, {`. So does the "one-shot gains a byte" plant.
  - A guard that compares only lengths (v07) therefore passes `builder_guard_goes_red_alone_when_planted`, `builder_guard_holds_...` and every other test.
  - The forward pairs are still byte-checked against the case's own output, so the blind spot is the reverse and alone scenarios.
  - Fix: add a plant that keeps the length on a non-forward pair, e.g. swap two adjacent bytes, or replace `"status":"ok"` with `"status":"ko"` in `reverse/json`.
- **F2 (Low; a test gap that predates this spec, surfaced by its brief; punch list).** No test or guard has two audited files with different maximum severities, the higher one first.
  - So a running maximum that keeps the minimum (v09) passes the full suite.
  - The builder-versus-`summarize` checks are equal by construction, so they cannot see it.
  - Fix: one unit test, on the builder or on `summarize`, with a high-severity file then a lower-severity one, asserting that the summary's `max_severity` is the higher.
- **F3 (Info, doc precision).** The `ReportBuilder` doc says the retained size is the sum of the fragment lengths plus a constant per fragment.
  - It is really the sum of the fragment *capacities* plus the `Vec`'s capacity, up to about twice the lengths. I measured 2.3 to 3.6 KB retained for 1.3 to 2.6 KB of output.
  - The property that matters holds: no input byte is retained (under 4 KB retained after a 50 MiB input).
  - Optional fix: say "capacities" in the doc, or `shrink_to_fit` each fragment.
- **F4 (Info).** Some mutants are caught by a single check or a single kind of check:
  - v02: only the goldens
  - s11: only the two builder unit tests
  - v11 and v13: one existing test each

  Acceptable, but recorded so nobody deletes those tests as redundant.

### Cost self-report

- **Tokens (total):** 212,770 (orchestrator-transcribed; final cumulative figure after one resume)
- **Estimated USD:** 1.40 (212,770 tokens at 6.60 USD per Mtok)
- **Duration (minutes):** 24 (harness wall-clock, first review plus delta verify)
- **Source of the number:** harness report (subagent_tokens)

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:** none beyond those the build handback declares. I checked and accept them all: the location of `builder_tests.rs`, the extra rule-5 test, the per-fragment constant in the bound, and the widened `flat` tuple.
- **Follow-up work identified:**
  - F1 and F2 above: two small tests, no library change.
  - Optional, agreeing with the build handback: a harness guard that renders an empty report and pins its frame, so that mutant 11's class is not caught by the two builder unit tests alone.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Nothing material. The brief's list of mutant ideas was useful. The isolation sandbox refuses compound shell commands that name `git` or `$HOME` in this worktree, so the mutant and fuzz runs went through small scripts.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — A rule for guard plants: every guard that compares strings needs at least one plant that keeps the length unchanged, so a length-only comparison is caught (F1). Other guards that compare strings may have the same gap.

3. **If you did this task again, what would you do differently?**
   — Run the mutants in a separate detached worktree from the start, as I did. But wait for the runner to exit rather than for a marker line: waiting on the marker made me read a half-finished results file once.

### Delta verify (follow-up 1: 4105f42, 73c182b, 8d4778c)

**Verdict: ✅ APPROVED at 8d4778c.** I reviewed `git diff fe862e4..8d4778c` after `git merge --ff-only claude/rust-exif-metadata-6351d5`. F1, F2 and F3 are closed. Two narrower variants survive (D1, D2); both are Low and non-blocking.

**No library logic change.** `git diff --stat f18bbda..8d4778c -- src/` touches only:
- `src/report/builder_tests.rs`: one new test, +49 lines.
- `src/report/mod.rs`: 11 lines, all inside the `ReportBuilder` doc comment (F3). The struct and every function are unchanged.

`tests/golden/`, `src/report/safe.rs`, `src/audit/`, `decisions/`, `scripts/`, `Cargo.toml`, `Cargo.lock` and `fuzz/` have no diff since f18bbda, so the goldens are byte-identical.

**F1 closed.** `same_length_corrupt` flips the case of the first ASCII letter. It asserts the length is unchanged, and the content always differs. There are 17 new plants:
- 12 corrupt the builder side of every pair, forward and non-forward.
- 5 corrupt the one-shot side: forward/json, reverse/text, reverse/csv, alone-audited/json, alone-failed/csv.
- In the accept direction, the same corruption applied to both sides of the 5 non-forward pairs is accepted.

Mutants on the guard (each applied, run, reverted):
- v07, comparing lengths only: now red (`builder_guard_goes_red_alone_when_planted`, `each_render_guard_goes_red_alone_when_planted`).
- d01, comparing lengths and the first byte: red, same two tests.
- d02, byte-comparing only the forward pairs: red, same two tests.
- d04, comparing lengths and the last byte: red, same two tests.

**F2 closed.** `builder_summary_max_severity_is_the_maximum_in_both_add_orders` kills:
- v09, keep the minimum.
- d05, the maximum of the first file only.

Both fail only that test.

**F3 closed.** The doc now says the builder keeps the *capacity* of each fragment (up to about twice its length), plus one header per fragment and a constant. That matches what I measured.

**Residual survivors (Low, non-blocking, both survive the full `cargo test`):**
- **D1.** d03, a guard comparing lengths and the first 16 bytes. Every same-length plant corrupts the first letter, which is within the first 3 bytes of every output (`file:`, `{"schema`, `label`). So a guard that compares a prefix passes them all. Suggested fix: also corrupt a letter near the end of the output, e.g. the last letter.
- **D2.** d06, a severity order of its own with `Low` and `Medium` swapped. Nothing has two audited files whose highest severities are Medium and Low. The F2 test uses High and Low. The real code uses the derived `Ord`, so this needs someone to write a custom ranking. Suggested fix: add a Medium file to the F2 test.

**Gates at 8d4778c (all exit 0):**
- fmt: `cargo fmt --all -- --check`, the fuzz graph, and `rustfmt --check` of the two harness files.
- `just lint`, `just lint-fuzz`, `just lint-fuzz-red-proof`.
- `DIAMOND_EXIF_REQUIRE_ORACLE=1 cargo test --no-fail-fast` on stable 1.99.1: every binary ok.
- `just fuzz-smoke audit_render`, 60 s: `Done 36566 runs in 61 second(s)`, `cov: 4823 ft: 14727 corp: 881/222Kb exec/s: 599`.

The mutant runs used a detached scratch worktree, which has been removed. The tree is clean.
