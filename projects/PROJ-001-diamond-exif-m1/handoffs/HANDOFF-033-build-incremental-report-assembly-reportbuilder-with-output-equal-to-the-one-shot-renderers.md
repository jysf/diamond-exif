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
  id: HANDOFF-033
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
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
  tokens_total: null               # REAL combined count — what cost-audit reads (not visible to this agent; orchestrator transcribes the metered figure)
  estimated_usd: null              # tokens_total × your rate, or your harness's number
  duration_minutes: 95             # estimate
  branch: worktree-agent-ac688c9a11a7dbe7c
  pr: null                         # not opened, per the brief
  completed_at: 2026-11-06         # YYYY-MM-DD
  notes: "tokens_total null: a subagent cannot see its own token count; orchestrator transcribes the metered figure"                      # one line if unusual (rework, no meter, etc.)
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-033: Incremental report assembly: ReportBuilder with output equal to the one-shot renderers

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-018` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

Read before starting:

- **Spec (API, rules 1 to 6, the fuzz guard, named tests and the 12 mutants are normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-018-incremental-report-assembly-reportbuilder-with-output-equal-to-the-one-shot-renderers.md`
- **DEC-010 and its two amendments (normative for the report content; read it, do not edit it):** `decisions/DEC-010-report-rendering-contract.md`. DEC-011 (`decisions/DEC-011-exif-audit-cli-contract.md`) explains why this spec exists (the binary holds one input file in memory at a time); you implement only the library part.
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe; macOS `sed -i ''`)
- Code: `src/report/{mod,json,csv,text,safe}.rs` (the per-file fragment writers `file_json`, `file_block` and the CSV row writer already exist), `tests/report_render.rs`, `tests/report_formats.rs`, `tests/golden/*`, `fuzz/shared/render_invariants.rs`, `fuzz/fuzz_targets/audit_render.rs`, `tests/fuzz_layout.rs`
- Prior art: HANDOFF-031/032 (SPEC-015 build and verify): the guard style (named guards run alone through `run_guard`, both directions of every iff, plants in `tests/fuzz_layout.rs`, step counter), and the lessons that cost follow-ups there: a timing assertion must be a ratio, never an absolute bound; a guard is only as strong as the seeds that reach it.

Constraints: `panic-free-library` (five-lint deny set: no unchecked `+`, `*`, `<<`; use `saturating_add`, iterators; never index a slice or string), `forbid-unsafe-code`, `no-io-in-library`, `fuzz-target-per-surface` (the existing `audit_render` target is extended; no new target), `test-before-implementation`, `oracle-must-be-shown-red` (applied to the mutant list), `no-new-top-level-deps-without-decision`.

## Expected Deliverables

- Everything in the spec's Outputs and every Acceptance Criterion; every named test under its name (a rename, fold or missing test is a deviation to report). Edits are in `src/report/*` (new `Format`, `ReportBuilder`, `render`; the one-shot `render_text`, `render_json`, `render_csv` and `summarize` rewritten on top of the shared parts so there is one code path), `tests/report_render.rs`, `fuzz/shared/render_invariants.rs` (the `builder_equals_oneshot` guard), `tests/fuzz_layout.rs` (plants, linearity). `audit()`, the preview, the escaping, every golden file and every existing test expectation stay unchanged.
- Order: the equivalence tests first (red against a stub builder), then refactor the renderers into fragment + assembly parts, then make the one-shot functions call the builder parts, then the harness guard and plants, then the 12 mutants (for each: control first, apply by hand, show the named failure with its assertion text, revert, `git diff` empty).
- A doubt about a DEC-010 rule is a question for the architect: write it in the Handback and `guidance/questions.yaml`; do not edit `decisions/`.
- Run locally, because they are CI-only otherwise: `just lint-fuzz-red-proof`, `just lint-red-proof`, `just oracle-record` (empty `git status --porcelain` afterwards), and `just fuzz-smoke audit_render` (60 s; record executions and coverage); also confirm a plant of the new guard is found by the real `audit_render` target.
- Commit on your worktree branch. **Do not push, do not open a PR, do not advance cycles.** First `git merge --ff-only claude/rust-exif-metadata-6351d5` (you need the commit containing HANDOFF-033); if refused, use `git merge` and say so.
- Do not modify irradiance, `scripts/`, `decisions/`. No dependencies. Scratch work goes in a scratchpad directory, not `/tmp`.
- You cannot see your own token count; set `tokens_total: null` with a note, the orchestrator transcribes the metered figure.

## Out of Scope

- Any change to report content, schema, escaping, statuses or goldens; streaming to a writer; bounding report size
- The binary, argument parsing, file I/O (SPEC-016)
- Pushing, PRs, cycle advancement

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

- **Branch / PR:** branch `worktree-agent-ac688c9a11a7dbe7c` (no PR, not pushed, cycle not advanced, per the brief). The `git merge --ff-only claude/rust-exif-metadata-6351d5` succeeded (fast-forward to 8d7ca73).
- **Completed at:** 2026-11-06
- **All acceptance criteria met?** yes. `Format`, `ReportBuilder`, `render` exist; rules 1 to 6 hold; every named test exists under its name; the seed cross product holds; all golden files and existing tests pass unchanged (one existing test helper line widened, see deviations); the guard, 8 plants and the real-target plant check are done; all 12 mutants shown red and reverted.

**Design as built.** `src/report/mod.rs`: `Format` (`name()`), `ReportBuilder` (`new`, `add`, `summary`, `len`, `is_empty`, `finish`; derives Clone and Debug), `render`, a private `build_all(format, files, opts)` (new, add each, finish). `Summary` gained private `new`, `add_file`, `finalized` (the "no reliable file means residual None" rule lives only in `finalized`); `summarize` is `fold(add_file)` then `finalized`. Each of `json.rs`, `text.rs`, `csv.rs` exposes `fragment(entry, opts)` and `assemble(fragments, ...)`; `render_json/text/csv` are one line each over `build_all`, so there is one code path (mutant 9 proves a divergent copy is caught). The builder holds `format`, `opts`, `Vec<String>` of fragments and a running `Summary`; no reference to the entry survives `add`.

**Gate results (all run locally, all green):** `cargo fmt --all -- --check`; `rustfmt --edition 2021 --check` on `fuzz/shared/render_invariants.rs` and `fuzz/fuzz_targets/audit_render.rs`; `just lint`; `just lint-fuzz`; `just lint-no-allow`; `just lint-red-proof`; `just lint-fuzz-red-proof`; `DIAMOND_EXIF_REQUIRE_ORACLE=1 cargo test` on stable (Homebrew 1.99) and on `~/.cargo/bin/cargo +1.90.0` (every test binary ok); `just forbid-unsafe` (`-F unsafe_code`); `just deny`; `just deny-fuzz`; `just validate`; `just decisions-audit` (0 structural errors; the DEC-008/DEC-009 scope warning pre-exists); `just cost-audit`; `just oracle-record` followed by an empty `git status --porcelain`; `just fuzz-smoke audit_render`.

**Fuzz smoke, `audit_render`, 60 s, with the new guard in `GUARDS` (10 guards):** two runs, both clean. Run 1: 43,462 executions in 61 s. Run 2: 42,257 executions in 61 s, final line `DONE cov: 4803 ft: 14400 corp: 923/221Kb exec/s: 692`. About 700 exec/s. I did not take a pre-change baseline, so I cannot state the slowdown from the 12 extra builder renders and 9 extra one-shot renders per case that `build_case` now does; the 60 s smoke is unaffected in outcome. **Plant found by the real target:** with the divergent library mutant 8 (`summary()` skips Failed) applied, `just fuzz-smoke audit_render` exited 1 with `render invariants violated: [("builder_equals_oneshot", "the builder's summary after 2 entries is SummaryF { files: 1, failed: 0, ... }` (`SUMMARY: libFuzzer: deadly signal`); reverted, tree clean.

**Guard `builder_equals_oneshot`.** `Ctx` gained `pairs` (12: forward, reverse, alone-audited, alone-failed, each in text/json/csv, builder output beside one-shot output) and `summary_pairs` (builder `summary()` beside `summarize` after 0, 1, 2 entries). The guard demands all 12 pairs by name and in order, byte equality in each, that the forward pairs equal the case's own `text/json/csv`, the three summary comparisons, and that the final running summary is the case's summary. Plants (8, in `builder_plants()`, folded into `render_plants()` so the generic two-plants-per-guard test also covers it): builder side: drops the last fragment, a fragment rendered without values (other options), summary of the entries before the last, JSON joined with `}, {`; one-shot/bookkeeping side: one-shot gains a byte, summarize side counts one more file, a comparison missing, builder and one-shot agreeing with each other but not with the case's JSON. The accept direction of the iff: an equal change on both sides of five non-forward pairs is accepted. Linear: 114,713 steps at 5,000 thumbnails (about 10,000 items plus findings), 57,399 at half; 47,874 and 23,888 for regions.

### Mutants (each: control green first, applied by hand through a script that edits, runs `cargo test --no-fail-fast`, and `git checkout`s; `git diff --stat` and `git status --porcelain` empty after each)

Because the one-shot functions are now the builder, a mutant in shared code changes both sides, so the equivalence tests and the `builder_equals_oneshot` guard cannot see it; the independent anchors are the golden files, the fragment-composition test, the other format/fact tests and the other harness guards. Each line names the test or guard that fired (the first independent one first).

1. Builder drops the last fragment in `finish`: `golden_text_json_csv_match_byte_for_byte` and `golden_files_are_unchanged_and_the_builder_reproduces_them`, `builder_fragments_appear_in_add_order_and_are_independent_of_neighbours` (`assertion failed: !fa.is_empty() && !fb.is_empty() && fa != fb`), `builder_adding_the_same_entry_twice_gives_two_fragments`, `builder_retains_only_fragments_not_input_bytes`, plus 25 tests in all (guards `json_facts` "1 files, want 2", `csv_rows` "18 rows, want 19", `text_safe`).
2. JSON fragments joined with `, `: the two golden tests, `builder_fragments_appear_in_add_order_and_are_independent_of_neighbours`, `builder_retains_only_fragments_not_input_bytes`. NOT caught by any harness guard (`, ` is valid JSON to the strict parser), and the plant "JSON joined with `}, {`" only shows the guard rejects a builder that does it while the one-shot does not.
3. JSON summary computed before the last `add`: `builder_json_assembly_puts_the_running_summary_before_the_files`, `json_summary_matches_summarize`, both golden tests, guard `summary_facts` (the JSON summary differs from the re-derivation), among 10.
4. CSV header per fragment: both golden tests, 15 tests including `csv_uses_crlf`, `csv_has_a_row_for_every_file_without_findings`, `csv_inventory_mode_rows`; guard `csv_rows` via `render_harness_holds_on_every_seed_and_degenerate_inputs`.
5. Text summary block first: first run, only the two golden tests and `render_dispatches_to_the_matching_one_shot` fired; `builder_fragments_appear_in_add_order_and_are_independent_of_neighbours` PASSED because it derived each file block by splitting the output at `summary\n` (a summary-first text makes both blocks empty). I fixed that test (it now asserts each block starts with `file: ` and the two differ; commit d33bd6e) and re-ran the mutant: it now fails with `"" ""`, plus both goldens and `render_dispatches_to_the_matching_one_shot`.
6. `add` uses default options: 20 tests, `builder_keeps_no_borrow_of_the_added_entry` (the secret value missing), both goldens, `audit_render_seeds_are_committed_and_have_the_shapes_they_claim`, guards `values_gate`/`json_facts` via the harness tests.
7. `add` appends in reverse order: both goldens, `builder_fragments_appear_in_add_order_and_are_independent_of_neighbours`, `json_has_the_pinned_key_order_at_every_level`, guard `json_facts` via `render_harness_holds_on_every_seed_and_degenerate_inputs`, among 11.
8. `summary()` skips Failed: the only mutant that diverges builder from one-shot, so the new guard fires: `builder_summary_equals_summarize_at_every_step`, `builder_keeps_no_borrow_of_the_added_entry`, `builder_json_assembly_puts_the_running_summary_before_the_files`, `builder_equals_the_one_shot_on_the_seed_cross_product` (`Summary` inequality), and guard `builder_equals_oneshot` ("the builder's summary after 2 entries is ..."), also found by the real `audit_render` target.
9. `render_json` a divergent copy with one key swapped (`reliable_files` to `reliable_file`): both goldens, `builder_equals_the_one_shot_for_one_and_for_many_entries_in_every_format`, `builder_equals_the_one_shot_on_the_seed_cross_product`, guard `builder_equals_oneshot` (`builder_guard_holds_on_every_seed_and_degenerate_inputs`), `json_has_the_pinned_key_order_at_every_level`, `json_summary_matches_summarize`; 18 in all.
10. `Format::Csv` dispatches to `render_text`: `render_dispatches_to_the_matching_one_shot`, `builder_equals_the_one_shot_on_the_seed_cross_product`, `golden_files_are_unchanged_and_the_builder_reproduces_them` (`csv: render() differs`).
11. Empty JSON builder emits `"files":[,]`: `builder_with_no_entries_equals_the_empty_one_shot_in_every_format`, `builder_retains_only_fragments_not_input_bytes`. Nothing in the harness, because the harness never renders zero files; the golden and cross-product (which use `render`/one-shot as reference) cannot see a shared-path empty-document bug either. Only the two builder unit tests (which pin `"files":[]}` and the length deltas) catch it.
12. `len()` off by one: `builder_with_no_entries_equals_the_empty_one_shot_in_every_format`, `builder_adding_the_same_entry_twice_gives_two_fragments`, `builder_summary_equals_summarize_at_every_step`, `builder_equals_the_one_shot_on_the_seed_cross_product`, `golden_files_are_unchanged_and_the_builder_reproduces_them`.

### Cost self-report

- **Tokens (total):** null. A subagent cannot see its own token count; the orchestrator transcribes the metered figure from the `Agent` result.
- **Estimated USD:** null
- **Duration (minutes):** about 95 (estimate)
- **Source of the number:** none available to this agent (harness meters the subagent)

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:**
  - Fragments are stored as `Vec<String>`, so the retained size is the sum of fragment lengths plus one `String` header per fragment (a constant per file, independent of input bytes) rather than plus a single constant. The doc comment on `ReportBuilder` states exactly this. A single `String` body would meet "plus a constant" literally but makes mutants 1 and 7 artificial.
  - The builder unit tests are in a new file `src/report/builder_tests.rs` (`#[cfg(test)] mod builder_tests;` in `mod.rs`), not in `render_tests.rs`. All nine named unit tests are there under their names, in module `report::builder_tests`.
  - One extra test beyond the named list: `builder_retains_only_fragments_not_input_bytes` (rule 5: a 3 MiB failure detail, buffer dropped; output growth per file is exactly one fragment, JSON plus one comma).
  - The brief's equivalence unit tests are partly tautological after the refactor (the one-shot functions are the builder), as the spec intends ("one code path"). The independent anchors are the golden files, the fragment-composition test (CSV/text/JSON composed from single-file renders), and the harness guards. Mutants 1 to 7 and 11 demonstrate this: only the builder-versus-one-shot guard cannot see them.
  - `tests/fuzz_layout.rs`: the existing `each_render_guard_goes_red_alone_when_planted` compared a `flat` tuple of `json, csv, text, text_again, summary` to prove a plant changed something; I added `pairs` and `summary_pairs` to that tuple (otherwise builder plants read as "changed nothing"). The old `render_plants()` body was renamed `fact_render_plants()` and `render_plants()` now appends the builder plants. No expectation was relaxed.
  - `Ctx` (harness) gained `pairs` and `summary_pairs`; `build_case` now runs 12 builder renders and 9 extra one-shot renders and a builder summary walk per case. `summarize`'s flattening moved to a `flatten` helper in the harness (same fields).
  - The cross-product's sixth policy is `Custom(StripSpec::all().exclude(Category::Icc))`; the seed entries per case are: the seed audit, a failure with the seed's first 64 bytes, the seed audit again, a failure with empty detail and another reason, and the previous seed's audit (so 0, 1, 2 and 5 entries mix Audited and Failed).
  - `Summary::new`, `add_file`, `finalized` are private to `report` (spec said "internal").
- **Follow-up work identified:**
  - The `builder_equals_oneshot` guard only catches builder-versus-one-shot divergence (mutant 8, 9, 10, 12-style). A bug in the shared fragment/assembly code is caught by the other guards and goldens, but an empty-document bug in shared assembly (mutant 11) is caught only by the builder unit tests. If wanted, a harness guard could render an empty report and pin its frame independently.
  - No doubt about any DEC-010 rule; nothing added to `guidance/questions.yaml`.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Rule 5 says "a constant" beyond the fragments, but a list of fragments carries a per-fragment header; and the spec's equivalence tests and mutants 1 to 7, 11 cannot be distinguished by a builder-versus-one-shot comparison once the one-shot is the builder. I resolved both by stating the bound honestly and by naming the independent anchors per mutant.
2. **Was there a constraint or decision that should have been listed but wasn't?**
   — A note that the shell sandbox refuses long heredocs and compound commands naming `git` inside the worktree (I used the Write tool and small scripts instead), and that `render_text` etc. being `pub use`d from submodules means the one-shot functions must stay defined there or the re-export changes.
3. **If you did this task again, what would you do differently?**
   — Write the text-fragment test to compare against literal expected blocks from the start (the first version derived the blocks from the output and let mutant 5 through), and take a pre-change `audit_render` exec/s baseline to quantify the extra renders per case.
