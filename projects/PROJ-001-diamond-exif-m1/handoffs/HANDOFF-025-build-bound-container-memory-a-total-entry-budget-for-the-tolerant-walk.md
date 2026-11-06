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
  id: HANDOFF-025
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-05
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-017

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
  branch: worktree-agent-aa47050276922284a
  pr: null
  completed_at: 2026-11-05         # YYYY-MM-DD
  notes: "implementer cannot see its own token count; orchestrator transcribes the metered figure. Truncation-first check order is a deviation from the spec text (see Handback)." 
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-025: Bound Container memory (total entry budget)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-017` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

Read before starting:

- **Spec (rule, outputs, named tests normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-017-bound-container-memory-a-total-entry-budget-for-the-tolerant-walk.md`
- **`decisions/DEC-001-tolerant-walk-policy.md`** (read its "Clarifications from the SPEC-017 design"; normative), DEC-005, DEC-002
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe)
- Code: `src/tiff/mod.rs` (`Walk`, `follow`, `read_ifd`, limits, tests), `src/error.rs`, `fuzz/fuzz_targets/tiff_ifd.rs`, `fuzz/shared/audit_invariants.rs`, `fuzz/fuzz_targets/audit.rs`, `tests/audit_oracle.rs` (`hostile(n)` and the amplification measurement), `tests/fuzz_layout.rs`, `examples/fuzz_seeds.rs`, `tests/support/{tools,oracle_tiff}.rs`, `examples/oracle_record.rs`
- Prior art: HANDOFF-023/024 (SPEC-012: how the amplification was measured; shared harness), HANDOFF-009/010 (budget halts: nothing recorded after a halt)

Constraints: `panic-free-library` (five-lint deny set in non-test code; `checked`/`saturating` arithmetic), `forbid-unsafe-code`, `fuzz-target-per-surface`, `test-before-implementation`, `oracle-must-be-shown-red`, `no-new-top-level-deps-without-decision`. ExifTool is run, never read.

## Expected Deliverables

- Everything in the spec's Outputs with every named test and Acceptance Criterion. Tests first: show the hostile-file test red against the current code (it parses to ~33M entries), then implement.
- The budget check uses the IFD's **declared count before any entry is decoded or allocated**. Pin the actual order of checks inside `follow` (attempts, depth, cycle, count-vs-budget) in a test and state it in the Handback.
- **Nothing is recorded after the halt.** Every issue must still go through the existing paths; the budget halt follows the `TooManyIfds` precedent (global, ends the whole walk).
- Existing behaviour must not change: all 120+1 records, the 66+7 seeds (byte-identical) and every existing test must pass unchanged except the two the spec names (the amplification measurement test; the expectation on seed `audit-many-overlapping-ifds`). Any other existing test, fixture or record that changes is a **finding to report**, not to paper over.
- Report before/after memory of the N = 511 hostile file (peak RSS, entry count, time) and the real-photo plausibility numbers (three local JPEGs via ExifTool, counts and Exif blob sizes; commit nothing).
- Run **`just lint-fuzz-red-proof` and `just lint-red-proof` locally**, and `just oracle-record` with a clean `git status` afterwards; update the fuzz-smoke only if you change a harness (but run the 60 s smoke for `tiff_ifd` and `audit` since the invariants changed). Name every deviation.
- Commit on your worktree branch. **Do not push and do not open a PR.** The spec is at `cycle: build`; do not advance cycles. First run `git merge --ff-only claude/rust-exif-metadata-6351d5` (you need the commit containing HANDOFF-025). If the fast-forward is refused, use `git merge` and say so.
- Do not modify irradiance, `scripts/`, `decisions/`, the audit model or the JPEG layer. No dependencies.
- You cannot see your own token count; set `tokens_total: null` with a note, the orchestrator transcribes the metered figure.

## Out of Scope

- Copying ExifTool's 10-SubIFD limit; lowering `MAX_IFDS`; lazy entry decoding; per-tag budgets
- Audit, renderers, CLI changes; SPEC-013/014/015/016
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

- **Branch / PR:** `worktree-agent-aa47050276922284a` (local commit, not pushed, no PR; fast-forwarded from `claude/rust-exif-metadata-6351d5` first, the ff was accepted)
- **Completed at:** 2026-11-05
- **All acceptance criteria met?** yes, with one measured caveat: the "at least 10x headroom on the sampled photos" criterion holds for camera files (33.9x to 86.6x) but two stripped or phone files with tiny Exif blobs have 1.4x and 2.7x (see Real-photo numbers). That is a property of the rule, not a defect: a real file's disjoint tables can never exceed `len / 12`, so the budget cannot trip on a valid file at any headroom. Flagged for the architect.
- **For `verify`:** n/a (build handoff)

**Order of checks inside `follow` / `read_ifd` (pinned by `the_over_budget_ifd_allocates_no_entries`):** (1) attempts, `TooManyIfds`; (2) depth; (3) cycle; then in `read_ifd`: (4) count word read; (5) table bounds, `Truncated`; (6) declared count against the remaining budget, `TooManyEntries`; (7) only then `Vec::with_capacity` and entry decoding. The check is on the declared `u16` count, so the over-budget IFD allocates nothing.

**Gate results (all run locally, this worktree):**

- `cargo fmt --all --check` and `just lint` (default PATH, clippy 0.1.99): clean. `just lint-fuzz`: clean. `just lint-no-allow`: clean.
- `just lint-red-proof`: control clean (0), injection rejected (101), all five lints fired, still fire without `-D warnings`. `just lint-fuzz-red-proof`: control clean (0), `unnecessary_cast` injection rejected (101) at `fuzz_targets/tiff_ifd.rs:196`, same injection passes (0) without `-D warnings`.
- `DIAMOND_EXIF_REQUIRE_ORACLE=1 cargo test` (stable/Homebrew 1.99) and `~/.cargo/bin/cargo +1.90.0 test`: every target green (lib 180 passed; the audit, budget, fuzz_layout, metadata, jpeg and mutation oracle targets all passed; no skips).
- `just forbid-unsafe` (`-F unsafe_code`), `just deny`, `just deny-fuzz`, `just validate`, `just decisions-audit` (0 structural errors, the same 10 scope warnings as before), `just cost-audit`: green.
- `just oracle-record`: re-ran ExifTool 13.55 over everything; `git status` shows no modified record, only four new untracked records (see Deviations). All 121 pre-existing records byte-identical (hash-pinned by `existing_records_and_seeds_are_byte_identical`).
- 60 s fuzz smoke, clean: `tiff_ifd` 1,857,713 executions, cov 1146, ft 5056, corpus 682 (30,454 exec/s); `audit` 244,901 executions, cov 1710, ft 6799, corpus 779 (4,014 exec/s).

**Red-proofs, each shown red then green:**

- Unit tests: with `entry_budget` stubbed to `usize::MAX` and no check, 9 of the new library tests failed red (budget values, exact/over, ordering, `the_hostile_overlapping_subifds_file_is_bounded` at its budget assertion, the 33M-entry shape being the pre-fix behaviour measured below); then green after the implementation.
- Harness plants as table-driven tests (`entry_budget_plants_are_each_rejected_by_their_own_guard_alone`, 8 plants, each rejected by its named guard run alone, control first): check removed (`budget`), `entry_count()` disagreeing (`budget`), halts on an exact fit i.e. `>=` for `>` (`crossing`), refused IFD recorded anyway (`crossing`), wrong limit (`crossing`), `TooManyEntries` pushed twice (`once-last`), halt skipped (`once-last`), `TooManyIfds` with it (`exclusive`). The existing audit plant "truncation not flagged" still runs, on a generated input (below).
- Real libFuzzer target, library bug planted then reverted: (a) check removed (`if false && ...`), `tiff_ifd` crashed on the seed `entry-budget-over`: `budget: 7 entries retained, budget 6 for 78 bytes`; the same plant crashed the `audit` target identically. (b) `>=` for `>`, `tiff_ifd` crashed: `crossing: TooManyEntries but 3 + 2 fits the budget 5` (seed `entry-budget-exact`). (c) `walk.halted = true` skipped, `tiff_ifd` crashed: `once-last: TooManyEntries recorded 39 times` (seed `overlapping-subifds`). All reverted; the 60 s smokes above ran on the restored code.

**Before / after, N = 511 hostile file (789,516 bytes, release build, `/usr/bin/time -l`):**

| | before | after |
|---|---|---|
| entries kept | 33,488,386 | 65,536 (budget 65,793) |
| IFDs recorded | 512 | 2 (IFD0 + first SubIFD) |
| issues | 0 | 1 (`TooManyEntries { limit: 65793 }`) |
| `Container::parse` time | 110.8 ms | 182 microseconds |
| RSS after parse | 792,064 KB | 4,512 KB (3 MB growth) |
| entries x 24 B | 803,721,264 B (1,018x input) | 1,572,864 B (1.99x input) |
| audit (own parse) | 274 ms, items 65,536 (cap), truncated | 575 microseconds, items 65,536, not truncated, not clean |
| process peak RSS (parse + audit) | 1,512,701,952 B (1.5 GB; `peak memory footprint` 1,512,703,800) | 11,091,968 B (11 MB; footprint 10,076,520) |

"Before" is the pre-change `container_amplification_is_measured_and_reported` run in this worktree before any edit; "after" is `container_amplification_is_bounded_and_reported`. The audit item count equals the cap (65,536) exactly now, so `inventory_truncated` is false; see Deviations.

**Real-photo numbers (three-plus local files; nothing committed).** `exif_core::jpeg::read` on the Exif blob via a throwaway example (deleted), plus ExifTool 13.55 `-a -u -G1 -s` counting IFD0/ExifIFD/GPS/InteropIFD/IFD1/SubIFD tags:

| file | Exif blob | IFDs | entries (ours) | budget | headroom | ExifTool IFD tags |
|---|---|---|---|---|---|---|
| Nikon P1100 `DSCN3540.JPG` | 25,244 B | 5 | 62 | 2,103 | 33.9x | 59 |
| `bw/L1020912.JPG` | 46,996 B | 5 | 68 | 3,916 | 57.6x | 55 |
| `fuji_12_2025_misc/DSCF6030.JPG` | 65,460 B | 4 | 63 | 5,455 | 86.6x | 60 |
| `IMG_9242.jpeg` | 2,010 B | 3 | 62 | 167 | 2.7x | 59 |
| `DSC_0770.jpeg` (small-blob) | 866 B | 2 | 51 | 72 | 1.4x | 50 |

All five parse with zero issues. The camera files clear 10x easily; the last two (small Exif blobs, 51-62 entries in 0.9-2 KB) do not, because there entry tables are 70% of the blob. They are still safely under the bound by construction (disjoint 12-byte slots sum to less than the blob), so no false halt is possible, but the "10x" wording in the spec holds only for camera-sized blobs.

### Cost self-report

- **Tokens (total):** null. I cannot see my own token count; the orchestrator transcribes the metered figure.
- **Estimated USD:** null
- **Duration (minutes):** not measured
- **Source of the number:** none available to the implementer

### Drift and new artifacts

- **New decisions emitted:** none (`decisions/` untouched).
- **Deviations from spec:**
  1. **Check order: truncation before budget.** The spec's wording and `the_over_budget_ifd_allocates_no_entries` imply the budget wins over a truncated table. I put the table-bounds check (`Truncated`) before count-vs-budget. Reason: budget-first would change existing behaviour of merely damaged files (a thumbnail IFD with a corrupt big count in a small file used to be one `Truncated` issue and the walk continued; budget-first would turn it into a walk-ending `TooManyEntries`, because `len / 12` is small in a small file), which the Goal forbids ("nothing about any well-formed or merely damaged file changes"). The memory bound is unchanged (the check still precedes the `Vec` allocation and decoding). Pinned by the same test, which asserts both orders explicitly. If the architect wants budget-first, it is a one-line swap in `read_ifd` and a handful of existing-test expectations to re-measure.
  2. **Three new `tiff_ifd` seeds necessarily add records and move counts in existing tests.** The oracle recorder writes one record per `tiff_ifd` seed, so the spec's "one new record" is four: `hostile-subifds-overlap.txt` plus `entry-budget-exact.txt`, `entry-budget-over.txt`, `overlapping-subifds.txt` (all with ExifTool dashes, the SubIFD warning is only on the hostile one; each probed alone under `perl -e 'alarm 8'` first). The audit directory copies each `tiff_ifd` seed under `tiff-`, so audit seeds are 73 plus 3 = 76. Existing tests whose hard-coded counts I therefore updated (not papered over): `tests/fuzz_layout.rs` (`tiff_ifd` seed count 29 to 32 in two tests, audit count 73 to 76, the audit-directory name list gains the three `tiff-` copies), `tests/audit_model.rs` (`[29,37,73]` to `[32,37,76]`), `tests/jpeg_oracle.rs` (TIFF-record count 41 to 45). All 121 old records and all 66+7 old seeds are byte-identical (hash lists in `tests/budget_oracle.rs`). The 29 old `tiff_ifd` seeds and the audit seed `audit-many-overlapping-ifds` are unchanged in bytes.
  3. **Behaviour-changed existing expectations (the two named plus consequences).** (a) `container_amplification_is_bounded_in_test` (N = 8) now expects 2 IFDs, 65,536 entries, one `TooManyEntries`, audit items exactly 65,536 and `inventory_truncated` false, not clean, one exif issue (before: 9 IFDs, 524,281 entries, inventory truncated). The spec-named ignored measurement test was replaced by `container_amplification_is_bounded_and_reported` (not ignored, since it is now cheap). (b) `audit_seeds_are_committed_and_have_the_shapes_they_claim`: the seed `audit-many-overlapping-ifds` now yields 2 IFDs, 768 entries (budget 902), one `TooManyEntries`, not clean, items 768, not truncated (before: 256 IFDs, 195,586 entries, items at the cap, truncated). (c) Consequence the spec did not name: that seed was the only input that made the audit plant "truncation not flagged" work, and overlapping tables can no longer overflow the inventory cap. I rebuilt that plant's input in the test (`generated:big-inventory`, `big_inventory()`: a disjoint 786,464-byte TIFF with 65,537 entries; not committed) and added `the_generated_big_inventory_really_overflows_the_cap`. The `audit` harness invariants themselves (`fuzz/shared/audit_invariants.rs`) are unchanged.
  4. **`hostile(n)` moved** from `tests/audit_oracle.rs` to a new `tests/support/hostile.rs` so the recorder, `audit_oracle.rs` and the new `tests/budget_oracle.rs` share one copy (no behaviour change).
  5. **Naming/placement:** the two ExifTool tests and the hash-list test live in a new `tests/budget_oracle.rs` (the spec allowed "audit_oracle.rs ... or a new tests/budget_oracle.rs"); `read_ifd` now takes the remaining budget as a second parameter (private); `Error::TooManyEntries` is appended at the end of the enum (index 22 in the three exhaustive display-guard tests) rather than next to `TooManyIfds`, to avoid renumbering. A CHANGELOG entry was added. I did not edit `decisions/`, `scripts/`, irradiance, the audit model or the JPEG layer.
  6. **Fuzz invariant shape:** the budget invariants live in one shared file `fuzz/shared/entry_budget_invariants.rs` (included by `tiff_ifd`, `audit` and `tests/fuzz_layout.rs`) instead of inline in each target, so the plants test the real code. "No IFD recorded after the halt" is checked as: `TooManyEntries` is last, the refused IFD's offset is not among the recorded ones, and the declared count read independently from the input bytes really crosses the budget (an off-by-one in either direction fails).
- **Follow-up work identified:**
  - Decide whether the spec's 10x-headroom wording should be dropped or restated (it is a camera-file figure; stripped/phone Exif blobs sit at 1.4x-2.7x, still correct by construction).
  - JPEG segment-walker and `jpeg_segments` target were not touched; a JPEG with many Exif blocks each at its own budget is bounded per block (`MAX_EXIF_BLOCKS` = 16), not in total: 16 blocks x `2 x len` is still linear, noted only for completeness.
  - The 60 s CI smoke matrix in `.github/workflows/ci.yml` already runs `tiff_ifd` and `audit`; no CI file changed.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — The check order. The spec text says the budget error should win over a truncated table, while the Goal says damaged files must not change; those conflict for a corrupt count in a small file. I resolved it toward the Goal and pinned both orders in a test. Also: "one new oracle record" does not account for the recorder writing a record per `tiff_ifd` seed.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — The audit fuzz plant "truncation not flagged" depended on the overlapping seed overflowing the inventory cap, which the budget removes by design. A note that overlapping tables can no longer reach the audit cap (and that the plant needs a generated honest input) belonged in the spec's "tests that move" list. The count assertions on seed directories (29/73) and the TIFF record count (41) are also tests that move with any new `tiff_ifd` seed.

3. **If you did this task again, what would you do differently?**
   — Take the "before" measurement and the real-photo sample first thing (done), and generate the pre-change hash lists for records and seeds before touching the seeds directory (done too, but earlier would have saved a re-read). I would also have built the shared invariant file before the seeds so the seed shapes were validated against the plants as I wrote them.
