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
  id: HANDOFF-002
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-04
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-001

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
  tokens_total: 65662               # REAL combined count — what cost-audit reads
  estimated_usd: 0.43              # tokens_total × 6.60 USD/Mtok (.repo-context.yaml)
  duration_minutes: null
  branch: claude/spec-001-verify-0f40ea
  pr: null
  completed_at: null               # YYYY-MM-DD
  notes: "Recovered by the orchestrator from the session transcript usage records (de-duplicated by message id), not from /cost. tokens_total = input + output + cache_creation; cache_read tokens excluded (1,170,884). Input 24, output 9,177, cache_creation 56,461."
  synced_at: 2026-11-05
---

# HANDOFF-002: <Task Title — same as the spec's title>

## Delegation Summary

One sentence: `<from_agent>` (acting as `<from_role>`) hands `SPEC-001`
to `<to_agent>` (acting as `<to_role>`) for the **`<cycle>`** cycle.

## Context the Receiving Agent Needs

The receiving agent MUST read these before starting work. Keep the
list tight, but don't omit anything necessary.

### Primary

- **Project brief:** `./projects/PROJ-001-<slug>/brief.md`
- **Stage:** `./projects/PROJ-001-<slug>/stages/STAGE-001-<slug>.md`
- **Spec:** `./projects/PROJ-001-<slug>/specs/SPEC-001-<slug>.md`
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

- Code changes implementing SPEC-001's Acceptance Criteria.
- All failing tests in SPEC-001 now passing.
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
above AND the `## Handback

### Execution notes

- **Branch / PR:** `claude/spec-001-verify-0f40ea` (no push, no PR)
- **Completed at:** 2026-11-04
- **All acceptance criteria met?** Yes, every AC verified independently; one weakness in the policy tests (below).
- **For `verify`:** ⚠ PUNCH LIST (reviewed commit 2bee38c)

Evidence: fmt, clippy -D warnings, test (4 pass), `-F unsafe_code`, `cargo deny check licenses` (ok), `just validate`, `just decisions-audit` all pass; 1.90.0 build+test pass; clippy 0.1.90 via `~/.cargo/bin` first in PATH + `+1.90.0` rejects `unwrap()`. Scratch-copy red proofs (Homebrew clippy): unwrap_used, indexing_slicing, arithmetic_side_effects, expect_used, panic each rejected by name; clean lib passes. Lint block byte-identical to irradiance lib.rs:95-102; deny.toml identical except the header and a comment (DEC-011 ref dropped); LICENSE-MIT identical; LICENSE gone; Cargo.lock tracked; `[dependencies]` empty.

Punch list:
1. `lib_forbids_unsafe_and_denies_the_five_lints` is substring-only and passes vacuously. Mutation A: comment out `clippy::panic,` in the deny block (`// clippy::panic,`) leaves all 4 tests green (the real gate, clippy, would still catch only code that panics, not the missing lint). Mutation B: replace the deny block with `#![allow(clippy::unwrap_used)]` plus a commented-out `// #![deny(...)]` leaves all 4 tests green. Fix: strip `//` comments before matching and require the lints inside a non-comment `#![deny(`; ideally also reject `allow(`/`expect(` of those lints. Related: `clippy::panic` would be satisfied by `clippy::panic_in_result_fn`; match whole tokens.
2. Same test file, `manifest_is_dual_licensed_with_no_dependencies`: a `[dependencies.foo]` sub-table header resets `in_deps` to false, so a dependency declared that way passes. Treat any header starting with `[dependencies` as in_deps.
3. `guidance/toolchain-brief.md` host note is incomplete: 1.90.0 had no clippy/rustfmt component until the builder ran `rustup component add --toolchain 1.90.0 clippy rustfmt` (undocumented host mutation); with `~/.cargo/bin` first the nightly default has no clippy so bare `cargo clippy` fails, `+1.90.0` (or RUSTUP_TOOLCHAIN) is required. Add both facts. Also AGENTS.md §6 still says "Stubs until the crate exists" though the crate now exists (minor; `just dev`/`typecheck` for exif-audit remain stubs, say so).

Not a defect: `cargo deny` prints "unmatched license allowance" warnings (expected with zero deps). `decisions-audit --changed` reported "no changed files" because the work is committed; the full audit is clean. No scope creep found; no unrecorded decision-worthy choice (no `rust-toolchain.toml` is stated in Notes; rustup component install is a host tweak, not a DEC).

### Cost self-report

- **Tokens (total):** null
- **Estimated USD:** null
- **Duration (minutes):** ~20
- **Source of the number:** none available (`/cost` cannot be run from this session)

### Drift and new artifacts

- **New decisions emitted:** none
- **Deviations from spec:** none
- **Follow-up work identified:** fix items 1-3 above in a small rework commit on the build branch.

### Reflection (3 questions, short answers)

1. **What was unclear?** — Whether `-- -D warnings` plus `-F` alone is the enforcement and the tests only a tripwire; the spec does not say.
2. **Constraint/decision missing?** — None.
3. **Do differently?** — Run the mutation checks on the tests first; they found the main issue.
