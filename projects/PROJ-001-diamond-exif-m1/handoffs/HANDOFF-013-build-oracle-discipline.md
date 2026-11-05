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
  id: HANDOFF-013
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-05
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-005

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

# HANDOFF-013: ExifTool read-parity oracle

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-005` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

Read before starting:

- **Spec (normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-005-oracle-discipline.md`
- `docs/porting-brief-diamond-exif.md` §2.2 and §2.3, `docs/exif-core-design.md` §8, `AGENTS.md`, `guidance/toolchain-brief.md`, `guidance/constraints.yaml` (`oracle-must-be-shown-red`, `exiftool-never-linked`, `no-new-top-level-deps-without-decision`)
- **Pattern source, READ ONLY, never modify:** `~/PSeven/experiments/crustimg_redo_plus/irradiance/tests/support/tools.rs` and `tests/metadata_oracle.rs` (lines ~458-545). Port with edit and add ledger rows (DEC-003). **ExifTool's own source must never be read**; ExifTool is only run.
- `src/tiff/` public API, `fuzz/seeds/tiff_ifd/*` (inputs only), `examples/fuzz_seeds.rs` (byte-builder style)
- Prior art for how a build round is reported: HANDOFF-011 (SPEC-007 build)

Host: ExifTool 13.55 is at `/opt/homebrew/bin/exiftool`. Host toolchain traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint` under the default PATH.

## Expected Deliverables

- Everything in the spec's Outputs with all its named tests and every Acceptance Criterion met. In particular:
  - the replay control and per-variant perturbation red-proofs, the missing-field and extra-field tests;
  - the **live** demonstration that a mutated payload is caught (applied to a copy inside the test, nothing committed);
  - both skip modes demonstrated and recorded (hidden ExifTool prints `SKIP` and passes; `DIAMOND_EXIF_REQUIRE_ORACLE=1` fails);
  - `just oracle-record` re-run with an empty `git diff` after;
  - **every divergence between ExifTool and `exif-core` reported** in the Handback with the exact command, ExifTool version and measured output, plus entries in `KNOWN_DIVERGENCES` and `guidance/questions.yaml`. Do not silence, loosen a tolerance, or change library code to make one go away. If a divergence shows a library bug, stop and report it.
- Name every deviation (renamed, folded or missing tests) in the Handback.
- Commit on your worktree branch. **Do not push and do not open a PR.** The spec is already at `cycle: build`; do not advance cycles. First run `git merge --ff-only claude/rust-exif-metadata-6351d5` if your worktree was cut from an older commit (you need `b6d1d19` or later).
- Do not modify irradiance, `scripts/`, `decisions/`, the 29 fuzz seeds, or library code under `src/`. No dependencies or dev-dependencies.
- You cannot see your own token count; set `tokens_total: null` with a note, the orchestrator transcribes the metered figure.

## Out of Scope

- SPEC-008 (byte-mutation helpers, mutated-copy red-proof), CI wiring, JPEG/PNG/WebP, ExifTool write mode
- Fixing library bugs a divergence reveals; pushing, PRs, cycle advancement

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
