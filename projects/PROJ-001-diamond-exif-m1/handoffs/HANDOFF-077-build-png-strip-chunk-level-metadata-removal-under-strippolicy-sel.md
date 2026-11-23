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
  id: HANDOFF-077
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-22
  status: completed                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-037

project:
  id: PROJ-001
  stage: STAGE-010
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
  status: completed                     # completed | blocked | rejected
  agent: claude-sonnet-5-5                      # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: null                    # verify handoffs: approved | punch-list | rejected
  basis: agent-result                      # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: 408716               # REAL combined count — what cost-audit reads
  estimated_usd: 2.70              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: null
  branch: claude/spec-037-part-a
  pr: null
  completed_at: 2026-11-23               # YYYY-MM-DD
  notes: null                      # if unusual; double-quote it — an unquoted " #" starts a YAML comment
  synced_at: 2026-11-22
---

# HANDOFF-077: PNG strip, PART A (carrier measurement, reader/audit extension, planner, self-check)

**FIRST STEP, before anything else:** `git checkout -b claude/spec-037-part-a claude/spec-037-design`
(the harness may have started your worktree from main, which lacks the spec and DEC-021).

## Delegation Summary

`claude-opus-5-5` hands `SPEC-037` Part A to `claude-sonnet-5-5`: measure which PNG
chunks ExifTool reads as metadata, extend the reader and audit where it reads
more than the audit lists, build `src/write/png.rs` and the PNG self-check, with
unit tests. Part B (fuzz target, seeds, pins, CI job, README, DEC fill) is a
separate fresh handoff after Part A is reviewed. Do not start Part B.

## Context

Read first: `AGENTS.md`, `guidance/toolchain-brief.md`, **`decisions/DEC-021-*.md`
(the design: follow it, do not re-decide)**, the spec
`projects/PROJ-001-diamond-exif-m1/specs/SPEC-037-*.md` (not the -timeline file),
DEC-013 (all amendments), DEC-009, DEC-018, DEC-020, `src/write/jpeg.rs` (the
model planner), `src/write/exif.rs` (`plan_block`), `src/write/verify.rs`,
`src/png/crc.rs`, `src/audit/mod.rs` (`audit_png`).

Toolchain: `export RUSTUP_TOOLCHAIN=1.90.0 PATH="$HOME/.cargo/bin:$PATH"` (lint on
the pinned 1.90.0); macOS has no `timeout` (`perl -e 'alarm 8; exec @ARGV' --`);
BSD `sed -i` needs `''`; Python for multi-line edits. `indexing_slicing`, the
five-lint deny set, `forbid(unsafe_code)` and no panics apply in `src/`. Never
commit `decisions/INDEX.md` or `fuzz/Cargo.lock`. No CHANGELOG entry. Leave no
background processes running. Compare against `origin/main`.

## Part A, in order

1. **Measure the carriers.** One chunk per file (a minimal valid PNG with one
   extra chunk), ExifTool 13.55 `-a -u -G1 -H -s`, 8 s limit, for every chunk type
   in the spec's Notes list, plus raw-profile spellings, with GPS-bearing content
   where the chunk can carry Exif or XMP (e.g. a valid TIFF with GPS for `zxIf`
   zlib-compressed; an XMP packet with a location for `tXMP`). For each: what ExifTool
   prints, whether the audit lists it, and which category it should have. Report
   the table in your handback BEFORE building anything that depends on it.
2. **Reader and audit extension (only where step 1 requires).** If ExifTool
   decodes a chunk type the audit does not list, locate it as a region (small
   `src/png` change) and categorise it per DEC-021 clause 4 (carrier: removed by
   every preset; non-metadata carrier: `OtherSegment`). Amend DEC-020 with a
   "Clarifications from the SPEC-037 design" section and update the PNG parity
   tests, the inventory and category replays and the fuzz invariant mirrors that
   this moves; report every test or record that moves and why. This is a library
   behaviour change: its own commit first.
3. **`src/write/png.rs`.** The planner per DEC-021 clauses 1 to 5, modelled on
   `src/write/jpeg.rs`: targets by walking the chunks, `exif` handling through
   `plan_block` plus the CRC patch, opaque carriers, refusals. `strip` dispatches
   `Sniffed::Png`. Keep the existing StripError set; add a typed reason only
   where it cannot be expressed (halted walk, chunk past EOF).
4. **Self-check** in `verify.rs` per DEC-021 clause 6, including the raw-profile
   and carrier handling of check (b) (they are removed although their category
   `OtherSegment` is not in the policy: document the exemption in DEC-021 and
   test it).
5. **Tests**: the spec's Failing Tests for `src/write/png.rs` and
   `tests/strip_png.rs`, per preset, per chunk kind and position; I1
   recomputation; refusals; idempotence; CRC of an edited `eXIf`; update the
   SPEC-032 PNG refusal test (WebP still refuses). `ContainerKind::strippable()`
   stays false (a test pins it).
6. **Gates on the pinned 1.90.0:** `just lint`, `just test`, `just forbid-unsafe`,
   `just validate`, `just cost-audit`, `just oracle-record` twice (no diff), and the
   60 s smokes of any fuzz target whose code you changed (`just fuzz-smoke
   png_chunks`, `audit`, `audit_render` if you touched the reader or audit).
   Commit (the library extension first, then the writer) on
   `claude/spec-037-part-a` with trailer
   `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`. No PR. Leave
   tokens null in the handback (the orchestrator records the harness figure).

## Budget

About 100 tool calls or 250k context. Stop and hand back what is done if either
is near (after the step 1 table and the step 2 commit if need be); do not loop.

## Out of Scope

Part B items; WebP; the audit and CLI switch-over; ExifTool write parity; a CLI
strip command.

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

- **Tokens (total):** 408716 (harness subagent_tokens)
- **Estimated USD:** 2.70
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
