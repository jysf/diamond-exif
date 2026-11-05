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
  id: HANDOFF-010
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-05
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-004

project:
  id: PROJ-001
  stage: STAGE-001
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

# HANDOFF-010: Verify the multi-IFD pointer walk

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-004` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-004-multi-ifd-pointer-walk.md` (walk rules and named tests are normative)
- **`decisions/DEC-005-pointer-tag-walk-policy.md`** (normative table), DEC-001 (with clarifications), DEC-002, DEC-004
- **Build handoff and its Handback:** `HANDOFF-009-build-multi-ifd-pointer-walk.md` (claims to verify, not to trust)
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`)
- Code under review: `git diff bad5fcb..HEAD` (build commit `06f1702`)

## Expected Deliverables

The verify contract: review cold, output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

Verify independently:
- Re-run every gate in the spec's acceptance criteria.
- Read the walk against the rules and DEC-005 line by line: permitted-holder table, kinds (Ifd0/Ifd1/Chain/Sub), leaf IFDs' next-word not followed, value rule (type LONG/IFD, count exactly 1, count 0 never reads garbage), the SubIFDs cap checked **before** any payload read, depth-first order, attempt accounting (IFD0 = attempt 1), `from_ifd`/`offset` per issue, duplicate-tag handling, `MAX_ISSUES` bound. Look for any panic-capable construct in non-test code and any unbounded loop or allocation; try a few adversarial shapes yourself (pointer to itself via each tag, huge counts, Interop inside Gps, SubIFD whose chain holds Exif pointers).
- Redo at least the five walk plants (a)-(e) and confirm the named tests go red, then revert; redo at least two fuzz-invariant plants, including one against the real fuzz target, then revert. Leave the tree clean.
- The builder noted `the_issue_list_never_exceeds_max_issues` does not reach `MAX_ISSUES` (Interop is ignored outside Exif). Judge whether the bound is adequately tested; try to construct a worst-case file and report the largest issue count you can reach versus `MAX_ISSUES`.
- Check the flipped/updated SPEC-003 tests still mean what they did; check no test is vacuous and that every iff-invariant in the fuzz target is both-directional. Confirm the 20 original seeds are byte-identical and the seed writer is deterministic.
- Run `just fuzz-smoke` (60 s), record executions and coverage. Run `just decisions-audit --changed bad5fcb`.
- Judge the builder's deviations (tuple argument to `follow`, extra exact-count assert, duplicated `ALL_KINDS` in `tests/fuzz_layout.rs`).
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes, SPEC-007 or SPEC-005 work, CI wiring, the 24 h fuzz run

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
