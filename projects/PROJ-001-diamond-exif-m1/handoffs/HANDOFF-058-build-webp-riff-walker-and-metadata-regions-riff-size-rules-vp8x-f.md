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
  id: HANDOFF-058
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-21
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-030

project:
  id: PROJ-001
  stage: STAGE-008
repo:
  id: diamond-exif

# ── THE HANDBACK ────────────────────────────────────────────────────────────
# Filled in by the EXECUTING AGENT before it reports done. This is a required
# part of completing the handoff, not a courtesy.
#
# `tokens_total` is the one field the cost gate reads. Report the REAL number,
# and say in `basis` how you got it:
#   Claude Code   → your session transcript, measured per docs/cost-tracking.md
#                   "Measuring a cycle": count each message.id once, take its
#                   largest output_tokens, include subagents/ (basis: transcript).
#                   Summing every line over-counts ~2×. `/cost` in the desktop
#                   Code tab reports context size, not spend.
#   API           → the `usage` objects, summed (basis: api-usage)
#   another agent → whatever your harness reports as total tokens
# If your platform genuinely exposes NO token count, set tokens_total: null AND
# write why in `notes` — then set `cost.metering_source: none` in
# .repo-context.yaml so the gate stops asking. Do not invent a number.
handback:
  status: null                     # completed | blocked | rejected
  agent: null                      # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: null                    # verify handoffs: approved | punch-list | rejected
  basis: null                      # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: null               # REAL combined count — what cost-audit reads
  estimated_usd: null              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: null
  branch: null
  pr: null
  completed_at: null               # YYYY-MM-DD
  notes: null                      # if unusual; double-quote it — an unquoted " #" starts a YAML comment
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-058: Build SPEC-030 WebP RIFF walker and metadata regions (DEC-019)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-030` to `claude-sonnet-5-5` (implementer) for the **build** cycle. The spec is the contract: read it whole first (`projects/PROJ-001-diamond-exif-m1/specs/SPEC-030-webp-riff-walker-and-metadata-regions-riff-size-rules-vp8x-f.md`), then `decisions/DEC-019-webp-riff-walk-and-metadata-regions.md` (every behaviour rule, measured against ExifTool), and skim DEC-017 and DEC-018 for the PNG pattern you mirror. If they seem to disagree, stop and say so in the handback; do not choose.

## FIRST STEP (the harness may start your worktree from main, which lacks the design)

Run `git checkout -b claude/spec-030-webp claude/spec-030-design` (the local branch exists in this repository and holds the spec, DEC-019 and this handoff). Confirm `git log --oneline -1` and that this file exists. Do not use `reset --hard`.

## BUDGET (hard)

- At most about **100 tool calls or 250k context**. The spec asks for two checkpoints: commit the gated walker first, then the metadata half; if past about 60 percent of budget at the first checkpoint, hand back after it listing the metadata work as the remainder.
- Read large files with offset/limit. Prefer one write per file over many small edits.
- Record `tokens_total: null` with a note if you cannot measure it; the orchestrator replaces it with the harness figure. Never invent one.
- Commit on your branch. Do NOT push, do NOT open a PR, do NOT touch main. No new dependency.
- Toolchain: `RUSTUP_TOOLCHAIN=1.90.0`, `PATH="$HOME/.cargo/bin:$PATH"`; read `guidance/toolchain-brief.md`. BSD `sed -i` rejects multi-expression use: use the edit tool or Python. `indexing_slicing` is denied crate-wide (use `get`; `split_first_mut`-style code in const contexts). `fuzz/Cargo.lock` is rewritten by every fuzz build: restore it, do not commit it. New `Error` variants break exhaustive matches in unit tests: add arms (test code only). `tests/release_layout.rs` pins `[Unreleased]` in CHANGELOG empty: add no CHANGELOG entry. When scripting edits on the spec file, the glob also matches the `-timeline.md` file: use exact names.

## Context to read first

- The spec (Inputs, Outputs, Acceptance Criteria, Failing Tests, Non-Goals, Notes), DEC-019.
- `src/png/*` and the png fuzz files named in the spec's Inputs; `AGENTS.md` sections 8, 11 to 13, 15; `guidance/constraints.yaml`.

## Work items (in this order; tests first)

1. **Failing tests first** (walker): `tests/support/webp_shapes.rs`, `tests/webp_walk.rs`; see them red.
2. **Errors** (DEC-019 variants; walk-neutral shared messages), **`src/webp/mod.rs` + `issue.rs`**, walk half of the model `fuzz/shared/webp_riff_invariants.rs`, target `webp_riff`, seeds, wiring (fuzz/Cargo.toml, ci.yml, app.just, README, pins). **Gates, then COMMIT (checkpoint 1).**
3. **Metadata**: `tests/webp_metadata.rs` first, then `src/webp/metadata.rs`, the extraction half of the model, extra seeds, pins. Gates, commit (checkpoint 2).
4. **Final gates**: `just lint`, `just test`, `just lint-fuzz`, `just validate`, `just forbid-unsafe`, `just cost-audit`, `just fuzz-smoke webp_riff` (60 s).

## Expected Deliverables

Commits on the branch; every acceptance criterion met or listed as not met with the reason; spec acceptance ticks; the handback filled, including every choice you made where DEC-019 is silent.

## Out of Scope

Writer, decoding, XMP or ICC parsing, ExifTool tests (SPEC-031), audit wiring (SPEC-032), any DEC change (raise it instead), any change to PNG or JPEG behaviour, PR creation.

## Return Criteria

Fill the `handback:` front matter AND the `## Handback` section; set `handoff.status` and `handback.status` to `completed`. If you cannot complete: say exactly what is done and what blocked you, set both to `rejected`/`blocked`, and still report tokens.

---

## Handback` section below.** An unfilled handback is an
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
