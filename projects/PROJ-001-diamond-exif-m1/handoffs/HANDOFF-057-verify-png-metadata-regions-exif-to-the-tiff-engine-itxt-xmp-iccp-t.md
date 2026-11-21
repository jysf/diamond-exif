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
  id: HANDOFF-057
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-21
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-029

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

# HANDOFF-057: Verify SPEC-029 PNG metadata regions (DEC-018)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-029` to a cold verifier. You did not write this. Review branch `claude/spec-029-png-metadata` (build commit `a8c38c4`, cost commits after it) against the spec (`projects/PROJ-001-diamond-exif-m1/specs/SPEC-029-*.md`, the file without `-timeline`), `decisions/DEC-018-png-metadata-regions-exif-hand-off-and-text-routing.md` and DEC-017. Review only; edit nothing except your own handback in this file. Do not push.

## FIRST STEP

The harness may start your worktree from main. Run `git checkout -b claude/spec-029-verify claude/spec-029-png-metadata` (local branch; do not `reset --hard`) and confirm HEAD contains `a8c38c4`.

## BUDGET (hard)

About 80 tool calls or 200k context. Record `tokens_total: null` with a note if you cannot measure it (the orchestrator replaces it with the harness figure). Toolchain: `RUSTUP_TOOLCHAIN=1.90.0`, `PATH="$HOME/.cargo/bin:$PATH"`; read `guidance/toolchain-brief.md`.

## What to look hard at

1. **Panic-freedom and arithmetic** in `src/png/metadata.rs` and the new `frame` in `src/png/mod.rs`: every offset and length from the file, every slice, every loop's progress, the trailer scan bound.
2. **Did the refactor change the walker?** `frame` is shared by the walk and the trailer scan: confirm SPEC-028 behaviour is identical (all its tests and the old model pass unchanged; look for any subtle change in issue order, offsets or caps).
3. **DEC-018 conformance both directions**: every rule has code and a test that would fail if the rule were removed (keyword rule, 79-byte limit, iTXt header, zTXt, `Raw profile type ` routing, case sensitivity, `Exif\0\0` prefix, `ExtraExif`, caps, trailer scan, after_idat / after_iend, `TooManyRegions`). Judge the builder's choices where the DEC is silent: (a) any walker halt, not only a cap, suppresses all extraction issues; (b) `ImproperExifHeader` only for a parsed block; ExtraExif before ImproperExifHeader on one chunk; (c) `zTXt` with keyword `XML:com.adobe.xmp` is `Text`; (d) `TooManyRegions` stops all extraction including blocks, at the chunk that did not fit; (e) a trailer chunk is `after_idat` if any IDAT precedes it; (f) regions use `&[u8]` names. Say for each whether it is sound or needs a DEC-018 clarification. ExifTool 13.55 is installed: for (c) and any other doubtful case, build a tiny PNG and look at what `exiftool -a -G1 -s -n` does (zTXt with that keyword in particular), and report what you find.
4. **The model's independence**: `fuzz/shared/png_metadata_invariants.rs` derives regions/blocks/issues from raw bytes by its own parsing (it takes only the walker outcome from the library: judge whether that is acceptable); it must be able to fail: mutate the library on a scratch copy (rsync the tree elsewhere; do not mutate the branch) in at least six ways (keyword limit off by one, case-insensitive match, trailer cap off by one, after_iend flag, ExtraExif logic, compressed flag) and confirm tests or the model go red; list survivors and say if each is equivalent.
5. **Pins, seeds, scope**: 31 `meta-` seeds, older 33 byte-identical, `fuzz/Cargo.lock` not committed with a changed version, test-only arms in jpeg/tiff tests change no behaviour, README claims honest (PNG metadata located, not read by `exif-audit`), no CHANGELOG entry (the release pin), no new dependency.
6. Run `just lint`, `just test`, `just lint-fuzz`, `just validate`, `just forbid-unsafe`, `just cost-audit`, and a 20 s fuzz run of `png_chunks` yourself (the recipe hardcodes 60 s; use its cargo command with `-max_total_time=20`).

## Expected Deliverables

Your verdict, exactly one of APPROVED / PUNCH LIST / REJECTED, with findings ranked High, Medium, Low, each with file:line and a concrete failing input. Fill the `handback:` front matter (`verdict`) and the `## Handback` section; set statuses to `completed`; commit only the handback.

## Out of Scope

Fixing anything, DEC edits, audit wiring, ExifTool parity tests (SPEC-031).

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
