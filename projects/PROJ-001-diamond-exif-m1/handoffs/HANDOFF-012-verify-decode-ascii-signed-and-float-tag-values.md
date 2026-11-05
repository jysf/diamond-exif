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
  id: HANDOFF-012
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-05
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-007

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

# HANDOFF-012: Verify the ASCII, signed and float decoders

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-007` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-007-decode-ascii-signed-and-float-tag-values.md` (semantics and named tests are normative)
- **Build handoff and its Handback:** `HANDOFF-011-build-decode-ascii-signed-and-float-tag-values.md` (claims to verify, not to trust)
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`)
- DEC-002, DEC-003, DEC-004
- Code under review: `git diff 1cf9976..HEAD` on the current branch (build commits `9c5c301`, `9c84808`; handback `8e9b9a0`)

## Expected Deliverables

Review cold; output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

Verify independently:
- Re-run every gate in the spec's acceptance criteria.
- Read `ascii`, `sints`, `floats` and the shared prelude `gated_payload` against the spec semantics: type gate before the buffer, count cap before the payload, ASCII partition invariant, sign extension per width, f32 widening, byte order for every width. Hunt for panic-capable constructs in non-test code, unbounded allocation (u32::MAX counts), `Vec::with_capacity` on an unvalidated count, and any place the three readers can drift. Confirm `uints` and the rational readers behave as before (the prelude refactor must not have changed their order of checks).
- Try adversarial shapes yourself: ASCII count exactly `MAX_TAG_VALUES` and +1; inline ASCII count 1-4 with NULs in every position; DOUBLE with count 1 (always external); FLOAT count 1 (inline); count 0 for each reader; a type-2 entry whose offset is `u32::MAX`; each reader against every field type 1..=13 and unknown types (compare with the spec's accepted sets).
- Redo the six spec plants (a)-(f) and confirm the named tests go red; redo at least two fuzz plants, one against the real fuzz target; try two plants of your own that the builder did not (e.g. `floats` accepting RATIONAL; `Ascii::is_terminated` computed from the wrong slice; `sints` for SBYTE reading two bytes). Revert everything; leave the tree clean.
- Check the fuzz iff-invariants are both-directional and the layout test `exercise` mirrors the target. Note: `SEEDS` in `tests/fuzz_layout.rs` names only 24 of the 29 committed seeds (pre-existing for 21 of 26); judge whether that is worth a punch-list item.
- Confirm the 26 existing seeds are byte-identical and the seed writer is deterministic. Run `just fuzz-smoke` (60 s) and record executions and coverage. Run `just decisions-audit --changed 1cf9976`.
- Judge the builder's deviations (`Ascii::split` empty-tail choice, extra fuzz assertions, re-exports, no extra single-guard plants).
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes, SPEC-005 work, tag interpretation, CI wiring, the 24 h fuzz run

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
