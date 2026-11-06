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
  status: pending                  # pending | accepted | completed | rejected

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
