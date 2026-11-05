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
  id: HANDOFF-001
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
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
  tokens_total: 100445               # REAL combined count — what cost-audit reads
  estimated_usd: 0.66              # tokens_total × 6.60 USD/Mtok (.repo-context.yaml)
  duration_minutes: 15
  branch: claude/spec-001-scaffold-crate-0aa57b
  pr: null
  completed_at: 2026-11-05              # YYYY-MM-DD
  notes: "Recovered by the orchestrator from the session transcript usage records (de-duplicated by message id), not from /cost. tokens_total = input + output + cache_creation; cache_read tokens excluded (2,648,154). Input 49, output 16,957, cache_creation 83,439. Includes the punch-list rework turn."
  synced_at: 2026-11-05
---

# HANDOFF-001: Scaffold the exif-core crate

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-001` to `claude-sonnet-5-5` (implementer) for the **build** cycle: create the empty, gated, licence-clean `exif-core` crate. No parsing code.

## Context the Receiving Agent Needs

The receiving agent MUST read these before starting work. Keep the
list tight, but don't omit anything necessary.

### Primary

- **Project brief:** `./projects/PROJ-001-diamond-exif-m1/brief.md`
- **Stage:** `./projects/PROJ-001-diamond-exif-m1/stages/STAGE-001-m1a-foundation-tiff-ifd-engine.md`
- **Spec:** `./projects/PROJ-001-diamond-exif-m1/specs/SPEC-001-scaffold-crate.md`
- **Reviewer brief (OPTIONAL, verify handoffs):** `./guidance/agents/<lens>.md` —
  pick a lens if this review should look at one thing hard. Omit for a
  general review; see `./guidance/agents/README.md`.
- **Toolchain brief:** `./guidance/toolchain-brief.md` — this repo's test
  framework, lint quirks, runtime globals and gotchas. Read it; it exists so a
  cold agent doesn't rediscover them (DEC-004 rule 5).

### Decisions that apply

- `DEC-002` — `&[u8]` core; keep `[dependencies]` empty (no `bytes`, no `thiserror`).
- `DEC-003` — the ledger header must say ExifTool is RUN, never READ; no file rows yet.
- `DEC-001` — read for context only; no tolerant-walk code in this spec.

### Constraints that apply

Full text in `./guidance/constraints.yaml`:

- `forbid-unsafe-code`, `panic-free-library` — copy irradiance's lint block verbatim.
- `no-io-in-library`, `license-policy` — no I/O, permissive-only dependencies (`cargo deny`).
- `no-new-top-level-deps-without-decision` — adding any dependency needs a DEC.

### Prior related work

- Source to copy FROM (read-only, never modify): `~/PSeven/experiments/crustimg_redo_plus/irradiance` (`src/lib.rs:95-102`, `Cargo.toml`, `deny.toml`, `LICENSE-MIT`, `docs/provenance-ledger.md`).
- Host toolchain traps measured at design time are in the spec's Notes (Homebrew cargo rejects `+toolchain`; Homebrew clippy shadows rustup's). Use `~/.cargo/bin/cargo +1.90.0` for the 1.90 check.

## Expected Deliverables

*(For a `verify` handoff, replace this block with the verify contract below.)*

- Code changes implementing SPEC-001's Acceptance Criteria.
- All failing tests in SPEC-001 now passing.
- Any new tests required to cover edge cases.
- A PR against `main` from branch `feat/spec-001-scaffold-crate`.
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

- Any IFD/TIFF parsing code, `error.rs`, `reader.rs`, test ports (SPEC-002).
- Gate scripts, `lint-red-proof`, CI Rust jobs, fuzz targets (repo setup / later specs).
- Editing irradiance, adding dependencies, publishing, renaming the repo.

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

- **Branch / PR:** `claude/spec-001-scaffold-crate-0aa57b`, commit 67321b2 (not pushed, no PR)
- **Completed at:** 2026-11-05
- **All acceptance criteria met?** yes. fmt, clippy -D warnings, test, `-F unsafe_code`, `cargo deny check licenses`, `just validate`, `just decisions-audit` all pass; tests and clippy also pass on genuine 1.90.0 (clippy 0.1.90). Lint red shown: with an `unwrap()` and `v[0]` added to src/lib.rs, clippy failed with `error: used \`unwrap()\` on an \`Option\` value` (clippy::unwrap_used) and `error: indexing may panic` (clippy::indexing_slicing), `could not compile exif-core (lib)`; removing them restored green. Red run used Homebrew clippy 0.1.99.
- **For `verify`:** the verdict — ✅ APPROVED (at commit SHA) / ⚠ PUNCH LIST / ❌ REJECTED

### Cost self-report

Mirror what you put in the `handback:` front-matter, and say where the number
came from. **This is the number that lands in the spec** — the orchestrator
transcribes it via `just handback-sync`, it does not estimate it.

- **Tokens (total):** 100445 (orchestrator-recovered from the session transcript)
- **Estimated USD:** null
- **Duration (minutes):** ~15
- **Source of the number:** none available (context window read 118,304 tokens at handback; that is not billed usage)

### Drift and new artifacts

- **New decisions emitted:**
  - none
  - Verify punch-list rework (HANDOFF-002): crate_policy tests now strip `//` comments, match whole tokens, reject allow/warn/expect of the five lints and `[dependencies.*]` sub-tables. Mutations shown red: commented `clippy::panic`, `panic_in_result_fn` substitute, commented-out deny block + `#![allow(unwrap_used)]`, `#![allow(clippy::panic)]`, `[dependencies.foo]`. Host-note docs and AGENTS.md §6 wording fixed.
- **Deviations from spec:**
  - `rustup component add --toolchain 1.90.0 clippy rustfmt` was run on the host: rustup had no clippy/rustfmt for 1.90.0, so the spec's "put ~/.cargo/bin first" check could not run otherwise.
  - Also edited AGENTS.md (2 lines) and README per spec; deny.toml header rewritten, DEC-011 reference dropped.
- **Follow-up work identified:**
  - Cost metering: orchestrator must fill tokens_total.
  - With ~/.cargo/bin first in PATH the nightly default has no fmt/clippy; use Homebrew's on default PATH and `+1.90.0` explicitly.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Toolchain-brief claim that ~/.cargo/bin-first gives rustup clippy: 1.90.0 had no clippy component installed.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — None missing.

3. **If you did this task again, what would you do differently?**
   — Check rustup components up front.
