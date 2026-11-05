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
  id: HANDOFF-009
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
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

# HANDOFF-009: Multi-IFD pointer walk

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-004` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

Read before starting:

- **Spec (walk rules and named tests are normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-004-multi-ifd-pointer-walk.md`
- **`decisions/DEC-005-pointer-tag-walk-policy.md`** (the pointer table; normative), `DEC-001` (with clarifications), DEC-002, DEC-004
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `~/.cargo/bin/cargo +nightly`; `just lint` runs under the default PATH)
- `src/tiff/mod.rs` (`Walk`, `walk_from`, `walk_sub_ifds`, `follow`, tests, `Build`), `src/tiff/issue.rs`, `src/tiff/tags.rs`, `src/error.rs`, `fuzz/fuzz_targets/tiff_ifd.rs`, `tests/fuzz_layout.rs`, `examples/fuzz_seeds.rs`
- Prior art: HANDOFF-007 (SPEC-003 build) and HANDOFF-008 (verify; finding 1 is carried into this spec)

Constraints: `panic-free-library` (five-lint deny set in non-test code), `forbid-unsafe-code`, `fuzz-target-per-surface`, `test-before-implementation`, `no-new-top-level-deps-without-decision` (no dependencies).

## Expected Deliverables

- Everything in the spec's Outputs with all its Failing Tests present and passing, and every Acceptance Criterion met, including: the five recorded walk plants (each named test goes red, then revert), the fuzz invariants shown red with planted bugs (then removed), `just fuzz-smoke` clean with executions and coverage recorded, and `rustfmt --check` on the fuzz crate.
- **Write the fuzz invariant in both directions where it is an iff**; last time a one-directional check let a constant-`true` plant through.
- Commit on your worktree branch. **Do not push and do not open a PR.** The spec is already at `cycle: build`; do not advance cycles. First run `git merge --ff-only claude/rust-exif-metadata-6351d5` if your worktree was cut from an older commit (you need commit `0d5e381` or later).
- If a derived number in the spec (e.g. the 512-issue count) does not match what the rules produce, stop and report it in the Handback rather than adjusting it.
- Do not modify irradiance, `scripts/`, or anything outside the spec's Outputs. Existing 20 fuzz seeds must remain byte-identical.
- You cannot see your own token count; set `tokens_total: null` with a note, the orchestrator transcribes the metered figure.

## Out of Scope

- Decoders (SPEC-007), `parse_fields` and oracle work (SPEC-005), MakerNote, tag interpretation
- CI wiring, the 24 h fuzz run, pushing, PRs, cycle advancement

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
