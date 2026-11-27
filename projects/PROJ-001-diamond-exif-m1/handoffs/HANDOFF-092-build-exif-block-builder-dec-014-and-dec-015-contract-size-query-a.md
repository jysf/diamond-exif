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
  id: HANDOFF-092
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-27
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-025

project:
  id: PROJ-001
  stage: STAGE-007
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

# HANDOFF-092: SPEC-025 build: the Exif block builder in the source's byte order

**FIRST STEP, before anything else:** `git checkout -b feat/spec-025-exif-block-build docs/spec-025-design`. That
branch carries the design: the spec, DEC-014's amendment of 2026-11-27, DEC-015's clarification and this handoff. The
harness may have started your worktree from main. (If the design PR has merged by then, branch from `main` instead.)

## Delegation Summary

`claude-opus-5-5` (architect) hands SPEC-025 to `claude-sonnet-5-5` (implementer) for the **build** cycle. There are
**two parts, one agent each**. The prompt that starts you names your part. Run only that part, then stop at its
commit.

| Part | Acceptance criteria | Ends with |
|---|---|---|
| A | AC1 to AC8, AC10: `src/write/block.rs`, the self-check, unit tests, `tests/exif_block.rs`, the shapes, the independent model, the test placer | library committed first; A's tests green; `just lint` and `just lint-no-allow` green |
| B | AC9, AC11, AC12: the two gated live tests, the `exif_block` fuzz target, seeds, pins, CI, README, CHANGELOG, any DEC correction | every gate in AC12 green; `just fuzz-smoke exif_block` 60 s clean |

The design is done. The spec's *Design measurements*, **DEC-014's amendment of 2026-11-27** and **DEC-015's
clarification of 2026-11-27** are normative. Do not re-decide them. If the code or ExifTool contradicts them (a
measured count does not reproduce, or a sweep finds a tag the table lacks), add a question to
`guidance/questions.yaml`, stop, and report it. Do not change the table to match what you got.

**Gate on starting:** cleared. jysf accepted DEC-014 as amended on 2026-11-27 (`status: accepted`).

## Context the Receiving Agent Needs

### Primary

Read in this order:
1. `AGENTS.md` §11, §12, §13 and §15 ("During build").
2. `guidance/toolchain-brief.md` and `guidance/constraints.yaml`.
3. **`projects/PROJ-001-diamond-exif-m1/specs/SPEC-025-*.md`.** This is the source of truth: Context, *Design
   measurements*, the API in Outputs, AC1 to AC12, Failing Tests and Notes.
4. **`decisions/DEC-014-*.md`** in full, especially items 1, 4, 5, 7, 8 and 9 and the **amendment of 2026-11-27**
   (items 1 to 7). That amendment is the rule book for content, omissions, refusals and layout.
5. **`decisions/DEC-015-*.md`**: items 1 to 5 and the **clarification of 2026-11-27** (the scratch image).
6. DEC-005 (pointer tags and walk order: SubIFDs first), DEC-001 (the tolerant walk), and DEC-013 part 7 (the
   self-check pattern).
7. The stage file `stages/STAGE-007-*.md`, Design Notes.

The code and harness files are listed in the spec's Inputs and Outputs.

### Decisions that apply

- `DEC-014` + amendment 2026-11-27: what the block holds, what it leaves out and why, which walk issues refuse, and
  the layout.
- `DEC-015` + clarification 2026-11-27: size query, then write at an even base with final offsets; the self-check runs
  on the exact bytes at that base, in a scratch image that counts against the budget.
- `DEC-005`: the walk follows SubIFDs before IFD0's Exif pointer, so select the Exif and GPS IFDs by parent.
- `DEC-013` part 7: read your own output back before returning it.

### Constraints that apply

- `test-before-implementation`: commit each test red before its code, and say in the handback which commits did.
- `panic-free-library` and `forbid-unsafe-code`: the five-lint deny set (`unwrap`, `expect`, indexing, `panic`,
  `arithmetic_side_effects`). Length arithmetic in `u64` with `checked_*`.
- `no-io-in-library`: the builder takes `&[u8]` and returns bytes.
- `no-new-top-level-deps-without-decision`: none, dev ones included.
- `fuzz-target-per-surface`: the `exif_block` target is part of this spec (Part B).
- `exiftool-never-linked`: ExifTool is run, never read. The sweep classifies tags by ExifTool's behaviour, not its
  source.

### Prior related work

- SPEC-022 / `src/write/exif.rs`: the offset and pointer tag lists to share (`POINTER_TAGS`, `OFFSET_TAGS`,
  `EXIFTOOL_SUBDIRECTORY_TAGS`, `EXIFTOOL_BLOB_CARRIER_TAGS`).
- SPEC-037 / SPEC-038: `tests/strip_webp_live.rs` and `tests/strip_webp_sweep_live.rs` (the gated live-test and sweep
  pattern), and `rewrite_webp` (the fuzz-target pattern, with invariants independent of the writer).
- irradiance/SPEC-081 (shipped): the placing side copies the block to offset 8 and writes 0x8769 and 0x8825 as
  8 + position. Your test placer does the same at any even base.

## Expected Deliverables

- Code and tests implementing SPEC-025's AC1 to AC12 (your part's share).
- Part A commits on `feat/spec-025-exif-block-build`. Part B continues on the same branch and opens the PR against
  `main`. The PR description names HANDOFF-092, SPEC-025, STAGE-007, PROJ-001, DEC-014 and DEC-015, and the question
  ids you added.
- In the handback: the red-then-green evidence (commit SHAs), every measured number the live tests reproduced (the
  sweep's 22 writer refusals and 2 reader differences), and any deviation.

## Out of Scope

- Byte-order conversion and the UNDEFINED rule (SPEC-041). A cross-order request has no API yet; do not add one.
- Recorded, red-proofed ExifTool parity records (SPEC-026). AC9 is a live smoke.
- Policy presets (SPEC-027).
- Changing `OFFSET_TAGS` or anything else the strip does (question `strip-offset-tags-incomplete`).
- Changing the TIFF walk (`src/tiff/mod.rs`) beyond widening `type_size` to `pub(crate)`.

## Return Criteria — how to hand back

**Completing this handoff means filling in the `handback:` front-matter block above AND the `## Handback` section
below.** Leave `tokens_total` null: the delegated run cannot see its own spend, so the orchestrator records the
harness figure. Say so in `notes`.

On success:
1. Fill the `handback:` front-matter, with `tokens_total: null` and a note.
2. Fill the `## Handback` section. The reflection is part of it.
3. Set `handoff.status` → `completed` and `handback.status` → `completed` (Part B; Part A sets neither and reports
   in the PR or branch instead).
4. Part B opens the PR.

If you cannot complete the task:
1. Fill the `## Handback` section with what was done and what blocked you.
2. Set `handoff.status` → `rejected` and `handback.status` → `blocked`.
3. Set the spec's `task.blocked: true`, and add a question to `/guidance/questions.yaml`.

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
