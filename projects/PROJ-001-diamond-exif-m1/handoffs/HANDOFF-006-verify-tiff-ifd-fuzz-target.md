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
  id: HANDOFF-006
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-05
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-006

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
  status: completed                # completed | blocked | rejected
  tokens_total: null               # REAL combined count — what cost-audit reads
  estimated_usd: null              # tokens_total × your rate, or your harness's number
  duration_minutes: 25
  branch: worktree-agent-a885bebb6f4b2390b
  pr: null
  completed_at: 2026-11-05         # YYYY-MM-DD
  notes: "tokens left null for the orchestrator to transcribe from the metered count"                      # one line if unusual (rework, no meter, etc.)
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-006: <Task Title — same as the spec's title>

## Delegation Summary

One sentence: `<from_agent>` (acting as `<from_role>`) hands `SPEC-006`
to `<to_agent>` (acting as `<to_role>`) for the **`<cycle>`** cycle.

## Context the Receiving Agent Needs

The receiving agent MUST read these before starting work. Keep the
list tight, but don't omit anything necessary.

### Primary

- **Project brief:** `./projects/PROJ-001-<slug>/brief.md`
- **Stage:** `./projects/PROJ-001-<slug>/stages/STAGE-001-<slug>.md`
- **Spec:** `./projects/PROJ-001-<slug>/specs/SPEC-006-<slug>.md`
- **Reviewer brief (OPTIONAL, verify handoffs):** `./guidance/agents/<lens>.md` —
  pick a lens if this review should look at one thing hard. Omit for a
  general review; see `./guidance/agents/README.md`.
- **Toolchain brief:** `./guidance/toolchain-brief.md` — this repo's test
  framework, lint quirks, runtime globals and gotchas. Read it; it exists so a
  cold agent doesn't rediscover them (DEC-004 rule 5).

### Decisions that apply

- `DEC-NNN` — <one-line summary of why this matters here>
- `DEC-MMM` — <one-line summary>

### Constraints that apply

Check `./guidance/constraints.yaml` for full text. These constraints
apply to the paths touched by this task:

- `constraint-id-1` — <one-line summary>
- `constraint-id-2` — <one-line summary>

### Prior related work

- `HANDOFF-YYY` — <one-line summary, if relevant>
- `PR #NNN` — <link, if relevant>

## Expected Deliverables

*(For a `verify` handoff, replace this block with the verify contract below.)*

- Code changes implementing SPEC-006's Acceptance Criteria.
- All failing tests in SPEC-006 now passing.
- Any new tests required to cover edge cases.
- A PR against `main` from branch `feat/spec-XXX-<slug>`.
- PR description referencing: this handoff ID, the spec ID, the stage
  ID, the project ID, all referenced `DEC-*`, and any new `DEC-*`
  created during implementation.

### If `cycle: verify` — the verify contract

You are **not** the agent that implemented this. Review it cold.

- Acceptance criteria met? Tests actually pass? Build reflection answered
  honestly (*"nothing was unclear"* is suspicious)?
- Decision drift — run `just decisions-audit --changed`.
- Constraint violations; non-trivial choices missing a `DEC-*`.
- For any criterion claiming **runtime behavior** (a component registers, a hook
  fires, a binary resolves on PATH, a config takes effect), confirm the
  *behavioral* surface was exercised — not just the shape validated. That is the
  defect class that escapes (AGENTS.md §12).
- Output exactly ONE of: ✅ APPROVED / ⚠ PUNCH LIST / ❌ REJECTED.

## Out of Scope

Explicit list of what this handoff does NOT include. If the receiving agent
thinks any of these need to happen, they should create a new spec in
the stage's backlog, not expand this handoff.

- ...

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

- **Branch / PR:** `worktree-agent-a885bebb6f4b2390b`, no PR
- **Completed at:** 2026-11-05
- **All acceptance criteria met?** yes
- **For `verify`:** the verdict — ✅ APPROVED (builder tip 303e1c6)
- Gates run myself, all green: fmt, clippy -D warnings, cargo test (23+4+4), `-F unsafe_code`, 1.90.0 tests, `just deny`, `deny-fuzz`, `lint`, `lint-fuzz`, `validate`, `decisions-audit`, rustfmt --check on fuzz target and `examples/fuzz_seeds.rs`.
- Smoke (`just fuzz-smoke`): 12,015,015 runs in 61 s no crash; rerun with warm corpus: 16,122,492 runs, `DONE cov: 467 ft: 2155 corp: 283/47Kb`. `git ls-files fuzz` has no corpus/artifacts.
- Shown red (my own plant, scratch copy outside the repo): `if entry.count == 0x7FFF_FFFF { None.unwrap() }` in `Container::uints`; found within the 60 s run from the seeds, `called Option::unwrap() on a None value` at types.rs:87, artifact `crash-d1bb0df774c7e544d7c022452a97b2ed05bfd5ae`.
- Reverse check: unconditional panics planted in `Entry::count`, `Ifd::has`, `Container::byte_order`, `Container::ifd0_offset`, `Container::required_scalar` (none called by the target): 14.7M runs, no crash. These are the target's unreached public entry points (trivial getters/wrappers; `tag`/`field_type` likewise).
- Seeds inspected by parsing each: every name matches its shape (valid-le/be 7 entries; subifd 3 IFDs at depth 1; chain 3 IFDs; count-overflow typed Truncated; self/two-hop cycle CyclicIfd; deep-nesting IfdDepthExceeded; truncated-header, entry-count-past-eof, next-past-eof Truncated; ifd0-offset-zero OK with 0 IFDs). Writer deterministic (identical hash twice, equal to committed), writes only `fuzz/seeds/tiff_ifd/`.
- fuzz_layout goes red when: a seed is removed, `libfuzzer-sys` added to root manifest, `corpus` dropped from `.gitignore`, `seeds` added to `.gitignore`. It stays green if a seed file is emptied (content is not checked): nit.
- Scope: `git diff 91b99f4 -- src Cargo.toml` empty; deny.toml comment-only; no CI/scripts change; irradiance `git status --porcelain` empty.

### Cost self-report

Mirror what you put in the `handback:` front-matter, and say where the number
came from. **This is the number that lands in the spec** — the orchestrator
transcribes it via `just handback-sync`, it does not estimate it.

- **Tokens (total):** null, left for the orchestrator (metered count)
- **Estimated USD:** null
- **Duration (minutes):** ~25
- **Source of the number:** harness report (orchestrator transcribes)

### Drift and new artifacts

- **New decisions emitted:** none
- **Deviations from spec:** none new (builder's 17 seeds and `mkdir -p` corpus accepted)
- **Follow-up work identified:**
  - Optional: have `seeds_are_committed...` assert seeds are non-empty. SPEC-003 must extend the target with its readers.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — The sandbox rejects compound shell forms in this worktree; planting panics needed the lib's `deny(panic)` relaxed in a scratch copy.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — No.

3. **If you did this task again, what would you do differently?**
   — Check the layout test against content mutations earlier.
