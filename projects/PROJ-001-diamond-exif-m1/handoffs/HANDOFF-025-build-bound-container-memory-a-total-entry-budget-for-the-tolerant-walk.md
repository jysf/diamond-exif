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
  status: pending                  # pending | accepted | completed | rejected

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
  status: null                     # completed | blocked | rejected
  tokens_total: null               # REAL combined count — what cost-audit reads
  estimated_usd: null              # tokens_total × your rate, or your harness's number
  duration_minutes: null
  branch: null
  pr: null
  completed_at: null               # YYYY-MM-DD
  notes: null                      # one line if unusual (rework, no meter, etc.)
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

- **Branch / PR:** [link]
- **Completed at:** YYYY-MM-DD
- **All acceptance criteria met?** yes/no (if no, explain)
- **For `verify`:** the verdict — ✅ APPROVED (at commit SHA) / ⚠ PUNCH LIST / ❌ REJECTED

### Cost self-report

Mirror what you put in the `handback:` front-matter, and say where the number
came from. **This is the number that lands in the spec** — the orchestrator
transcribes it via `just handback-sync`, it does not estimate it.

- **Tokens (total):** <real number, or null + why>
- **Estimated USD:** <number, or null>
- **Duration (minutes):** <estimate>
- **Source of the number:** `/cost` | API `usage` | harness report | none available

### Drift and new artifacts

- **New decisions emitted:**
  - `DEC-NNN` — <title> (if any)
- **Deviations from spec:**
  - [list]
- **Follow-up work identified:**
  - [any new specs that should be added to the stage's backlog]

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — <answer>

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — <answer>

3. **If you did this task again, what would you do differently?**
   — <answer>
