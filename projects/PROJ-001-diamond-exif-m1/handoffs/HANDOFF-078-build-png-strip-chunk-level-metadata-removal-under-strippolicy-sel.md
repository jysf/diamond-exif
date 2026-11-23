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
  id: HANDOFF-078
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-22
  status: pending                  # pending | accepted | completed | rejected

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

# HANDOFF-078: PNG strip, PART B (rewrite_png fuzz target, seeds, pins, CI job, README, DEC fill)

**FIRST STEP, before anything else:** `git checkout -b claude/spec-037-part-b claude/spec-037-design`
(the design branch already contains Part A, commits `d4a2438` and `8d0bd0b`; the
harness may have started your worktree from main, which does not).

## Delegation Summary

`claude-opus-5-5` hands `SPEC-037` Part B to `claude-sonnet-5-5`: everything in the
spec that Part A (HANDOFF-077) did not do. Read HANDOFF-077's Handback and the
Part A summary below first. Do not change `src/write/png.rs` or `src/png`/`src/audit`
behaviour; if a fuzz or test finding shows a real bug there, stop and report it.

Part A built: `src/write/png.rs` (planner), `verify_png`, `RegionKind::{Carrier,
OtherChunk}`, `PngRegion::Chunk` with `ChunkClass`, `tIME` located by the reader,
`UnstrippableReason::ChunkPastEnd`, `PatchKind::ChunkCrc`,
`RemovalKind::{Text, Time, RawProfile, Carrier, OtherChunk}`, `tests/strip_png.rs`
(17 tests). Rules as implemented: text by keyword category; `iCCP` Icc; `tIME`
Timestamps; `caBX` Jumbf; carriers (raw profiles, `zxIf`, `tXMP`, `meTa`, `seAl`)
removed when the policy has any tag category, Xmp, Iptc or `OtherSegment`; `dSIG`
class `OtherSegment` (`Everything` only, or a Custom set with `OtherSegment`);
`eXIf` all/none/edit with CRC patch; trailer whole under `OtherSegment`, else per
chunk, `Overlap` if a whole-trailer removal would cut a kept finding.

## Context

Read: `AGENTS.md`, `guidance/toolchain-brief.md`, `decisions/DEC-021-*.md` and
`DEC-020-*.md` (with the SPEC-037 clarification), the spec, the JPEG model
`fuzz/fuzz_targets/rewrite_jpeg.rs` and `fuzz/shared/rewrite_invariants.rs`,
`fuzz/Cargo.toml`, `tests/fuzz_layout.rs`, `examples/fuzz_seeds.rs`,
`.github/workflows/ci.yml` (the fuzz smoke matrix), `app.just`.

Toolchain: `export RUSTUP_TOOLCHAIN=1.90.0 PATH="$HOME/.cargo/bin:$PATH"` (lint on
the pinned 1.90.0; clippy here differs from CI); fuzz needs nightly; macOS has no
`timeout` (`perl -e 'alarm 8; exec @ARGV' --`); BSD `sed -i` needs `''`; Python
for multi-line edits. Never commit `decisions/INDEX.md`; restore `fuzz/Cargo.lock`
with git after fuzz runs. No CHANGELOG entry. Leave no background processes.
Compare against `origin/main`.

## Part B, in order

1. **`rewrite_png` fuzz target.** `fuzz/fuzz_targets/rewrite_png.rs` and its
   harness `fuzz/shared/rewrite_png_invariants.rs`, modelled on `rewrite_jpeg`: take
   the bytes and a policy index (the five presets plus a few Custom sets, including
   one with only `OtherSegment` and one with only `Icc`); on `Ok`: output length at
   most input length; the output audits with no finding in a removed category;
   kept chunks byte-identical and in order (re-derive the expected output
   INDEPENDENTLY from the chunk walk, not from the library's removals); idempotent;
   the CRC of every chunk the writer edited is valid; on `Err`: no bytes. The
   harness must be independent of `src/write/png.rs` (its own chunk walk and its
   own category rule typed out again, as the other harnesses do). Register the
   target in `fuzz/Cargo.toml`.
2. **Seeds.** `fuzz/seeds/rewrite_png/`: generated by `examples/fuzz_seeds.rs`
   (extend it; no hand-made binary files), from the PNG seed shapes plus one seed
   per rule in DEC-021 (each category, `eXIf` all/none/edit, raw profile, `zxIf`,
   `tXMP`, `meTa`, `caBX`, `dSIG`, trailer, empty chunks, after-`IDAT`,
   after-`IEND`, bad CRC kept, walk halted). Policy byte first, then the PNG, as
   `rewrite_jpeg`'s seeds do.
3. **Pins and plants.** `tests/fuzz_layout.rs`: the seed set, its count, the policy
   byte meaning and plants (a harness that would pass a broken writer must fail on
   a plant: e.g. a writer that leaves a removed text chunk, one that corrupts a
   CRC, one that cuts a kept chunk). Follow the existing pattern for the other
   targets exactly.
4. **CI.** Add `rewrite_png` to the fuzz smoke matrix in `.github/workflows/ci.yml`
   (line 187) and wherever `just fuzz-smoke` lists targets (`app.just`, README
   command list, `tests/` that pin the list). Run `just fuzz-smoke rewrite_png`
   (60 s, nightly) clean, then the other PNG smokes (`png_chunks`, `audit`,
   `audit_render`).
5. **DEC-021 fill.** Put the measured carrier table (Part A's) into DEC-021, mark
   which rows are located and with which category, and record the four judgement
   calls of Part A as clarifications (Custom `OtherSegment` removes carriers;
   trailer whole under `OtherSegment` with `Overlap`; `caBX` is `Jumbf`; carriers are
   removed under `Gps` although plain XMP stays: state it plainly as a consequence).
6. **README.** The writer section: it strips PNG (library only; the audit and CLI
   still report PNG as not strippable until the ExifTool write-parity spec), the
   fuzz target list (ten targets), the commands. Update every test that pins those
   lists.
7. **Gates on the pinned 1.90.0:** `just lint`, `just test`, `just forbid-unsafe`,
   `just validate`, `just cost-audit`, `just oracle-record` twice (no diff),
   `just mutation-oracle`. Commit on `claude/spec-037-part-b` with trailer
   `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`. No PR. Leave tokens
   null in the handback. Tick the remaining acceptance boxes.

## Budget

About 100 tool calls or 250k context. If either is near, stop and report what is
done; do not loop.

## Out of Scope

`src/write/png.rs` and the reader/audit behaviour; WebP; the audit and CLI
switch-over; ExifTool write parity (SPEC-039); a CLI strip command.

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
