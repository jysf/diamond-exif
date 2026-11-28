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
  id: HANDOFF-094
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-28
  status: accepted                 # pending | accepted | completed | rejected

task:
  spec_id: SPEC-041

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

# HANDOFF-094: SPEC-041 build: Exif block byte-order conversion

**FIRST STEP, before anything else:** `git checkout -b feat/spec-041-exif-block-byte-order docs/spec-041-design`. That
branch carries the design: the spec, DEC-014's amendment of 2026-11-28, the answered question and this handoff. The
harness may have started your worktree from main. (If the design PR has merged by then, branch from `main` instead.)

## Delegation Summary

`claude-opus-5-5` (architect) hands SPEC-041 to `claude-sonnet-5-5` (implementer) for the **build** cycle. There are
**two parts, one agent each**. The prompt that starts you names your part. Run only that part, then stop at its
commit.

| Part | Acceptance criteria | Ends with |
|---|---|---|
| A | AC1 to AC8, AC10: the option, the converter, the `ByteOrder` table, the CFAPattern and UNICODE rules, the value-level self-check, unit tests, `tests/exif_block.rs`, the shapes, the model's cross-order predictor, the mirrors | red tests committed first; library; A's tests and every SPEC-025 test green; `just lint` and `just lint-no-allow` green; pushed |
| B | AC9, AC11, AC12: the cross-order live test, the order sweep, fuzz invariants, seeds, pins, README, CHANGELOG | every gate in AC12 green; `just fuzz-smoke exif_block` 60 s clean; PR opened |

The design is done. The spec's *Design measurements* and **DEC-014's amendment of 2026-11-28** are normative. Do not
re-decide them. If the code or ExifTool contradicts them (a sweep finds a tag the table lacks, a rule shape reads
differently, a number does not reproduce), add a question to `guidance/questions.yaml`, stop, and report it. Do not
change the table to match what you got.

**Gate on starting:** DEC-014's amendment of 2026-11-28 must be accepted by jysf. The orchestrator tells you in the
starting prompt; if it does not say so, stop and ask.

## Context the Receiving Agent Needs

### Primary

Read in this order:
1. `AGENTS.md` §11, §12, §13 and §15 ("During build").
2. `guidance/toolchain-brief.md` and `guidance/constraints.yaml`.
3. **`projects/PROJ-001-diamond-exif-m1/specs/SPEC-041-*.md`.** The source of truth: Context, *Design measurements*,
   the API in Outputs, AC1 to AC12, Failing Tests and Notes.
4. **`decisions/DEC-014-*.md`** in full: items 2, 3, 6 and 9, the **amendment of 2026-11-27** (the existing omission
   list, precedence and layout) and the **amendment of 2026-11-28** (items 1 to 6: this spec's rule book).
5. `decisions/DEC-015-*.md` and its clarification (the scratch image the self-check reads).
6. `projects/PROJ-001-diamond-exif-m1/specs/done/SPEC-025-*.md`: what already exists and how its tests are built.
7. `src/write/block.rs` in full, then `tests/exif_block.rs`, `tests/support/exif_block_shapes.rs`,
   `tests/support/exif_block_model.rs`, `tests/exif_block_live.rs` and `tests/exif_block_sweep_live.rs`.

### Decisions that apply

- `DEC-014` + amendment 2026-11-28: the target option in both directions; conversion by declared type; the `ByteOrder`
  table; the CFAPattern and UNICODE rules; the value-level self-check.
- `DEC-014` + amendment 2026-11-27: everything SPEC-025 built (unchanged in the same order). Precedence is now
  `UnknownType`, `MakerNote`, `OffsetCarrying`, `ByteOrder`.
- `DEC-015`: the base-first contract; every offset is final for the base, in the block's order.
- `DEC-013` part 7: read your own output back. In cross order the comparison is by decoded value.
- `DEC-023`: ExifTool is never run with `-m`.

### Constraints that apply

- `test-before-implementation`: commit each test red before its code, and give the SHAs in the handback.
- `panic-free-library` and `forbid-unsafe-code`: the five-lint deny set. Chunk with `chunks_exact`, not indexing.
- `no-io-in-library`, `no-new-top-level-deps-without-decision` (none, dev ones included).
- `fuzz-target-per-surface`: the cross-order path goes into the existing `exif_block` target (Part B).
- `exiftool-never-linked`: ExifTool is run, never read.

### Hazards measured at design (do not rediscover them)

- **ExifTool hangs on 0x014A** whose value is not a real IFD offset (it follows it as a SubIFD). Never give such a
  source to ExifTool; the sweep excludes every tag the block omits as `OffsetCarrying` or `MakerNote`.
- **SubIFD cycles** (`ok-subifd-loop`, `two-hop-cycle`) make ExifTool recurse without bound; SPEC-025's live test
  already skips those sources. Keep that.
- **Every ExifTool run gets an alarm**: `perl -e 'alarm 8; exec @ARGV' -- exiftool ...` (macOS has no `timeout`).
  Batch a few hundred files per run, with a longer alarm per batch, and re-run a timed-out batch per file.
- **The design's sweep command** was `exiftool -j -n -b -a -G1 -u <dir>` with no `-m`. A value printed as
  `base64:...` is compared by its decoded length only (an undecoded blob). Without that rule ColorMap and the like
  show up as false differences. The judge for AC9(a) is SPEC-025's: `-a -G1 -s -n -H`, no `-b`.
- **Latin UCS-2 hides the UNICODE bug**: ExifTool guesses its order. The UNICODE fixtures must use CJK text
  (`中文`, U+4E2D U+6587), whose swapped reading is also valid text.
- The design generated 42,336 pairs plus 144 rule pairs; at 1,000 files per ExifTool run, a run took under 1 s.

### Prior related work

- SPEC-025 (shipped, PR #71): `src/write/block.rs`, its self-check, grid, model, live tests and fuzz target. Its
  verify rounds caught a design probe that varied only one position (DEC-014's unknown-type note): vary every
  dimension the rule names (IFD, declared type, code, length parity, direction).
- irradiance/SPEC-081 (shipped): the placing side; the block is little-endian at offset 8 for it.

## Expected Deliverables

- Code and tests implementing SPEC-041's AC1 to AC12 (your part's share).
- Part A commits on `feat/spec-041-exif-block-byte-order` and pushes. Part B continues on the same branch and opens
  the PR against `main`. The PR description names HANDOFF-094, SPEC-041, STAGE-007, PROJ-001 and DEC-014, and any
  question ids you added.
- In the handback: the red-then-green SHAs; every number the live tests reproduced (the sweep's pair count, the
  sensitive set's size per IFD, the rule pairs); any new AC9(a) pin with its measured cause; any deviation.

## Out of Scope

- A conversion rule beyond DEC-014 2026-11-28 items 4 and 5, or a change to the table.
- Recorded, red-proofed parity (SPEC-026), policy presets (SPEC-027), MakerNote, IFD0, JPEG containers.
- The strip and `OFFSET_TAGS` (question `strip-offset-tags-incomplete`).
- SPEC-025's layout code and same-order output (AC2 is the net).

## Return Criteria — how to hand back

**Completing this handoff means filling in the `handback:` front-matter block above AND the `## Handback` section
below.** Leave `tokens_total` null: the delegated run cannot see its own spend, so the orchestrator records the
harness figure. Say so in `notes`.

On success:
1. Fill the `handback:` front-matter, with `tokens_total: null` and a note.
2. Fill the `## Handback` section. The reflection is part of it.
3. Set `handoff.status` → `completed` and `handback.status` → `completed` (Part B; Part A sets neither and reports
   in its final message instead).
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
