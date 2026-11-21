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
  id: HANDOFF-059
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
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

# HANDOFF-059: Verify SPEC-030 WebP RIFF walker and metadata regions (DEC-019)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-030` to a cold verifier. You did not write this. Review branch `claude/spec-030-webp` (build commits `009e96e` walker and `20141ef` metadata, cost commit after) against the spec (`projects/PROJ-001-diamond-exif-m1/specs/SPEC-030-*.md`, the file without `-timeline`) and `decisions/DEC-019-webp-riff-walk-and-metadata-regions.md`. Review only; edit nothing except your own handback in this file. Do not push.

## FIRST STEP

The harness may start your worktree from main. Run `git checkout -b claude/spec-030-verify claude/spec-030-webp` (local branch; do not `reset --hard`) and confirm HEAD contains `20141ef`.

## BUDGET (hard)

About 80 tool calls or 200k context. Record `tokens_total: null` with a note if you cannot measure it (the orchestrator replaces it with the harness figure). Toolchain: `RUSTUP_TOOLCHAIN=1.90.0`, `PATH="$HOME/.cargo/bin:$PATH"`; read `guidance/toolchain-brief.md`.

## What to look hard at

1. **Panic-freedom and arithmetic** in `src/webp/mod.rs` and `src/webp/metadata.rs`: every size from the file (a 32-bit LE size on a possibly 32-bit `usize`), every slice, every loop's progress, pad handling at the end of input, `Chunk::range()` and `tiff_at` arithmetic.
2. **DEC-019 conformance, both directions.** Every rule has code and a test that fails if the rule is removed. Judge the builder's choices where the DEC is silent and say for each whether it is sound or needs a DEC-019 clarification: (4) walk halts suppress the end-of-walk `ChunkFlagMismatch` and `MetadataWithoutVp8x` issues; (5) `vp8x()` is the first VP8X only, `None` if it is short even when a later one is long, a VP8X of any length counts as present for `MetadataWithoutVp8x`, no flag comparison with a short VP8X; (3) up to three walk issues on one chunk; (2) issue offsets for the flag and no-VP8X issues; (6) `range()` includes the pad only when present; (9) the public API shape. Also check the `RiffSizeMismatch` arithmetic for inputs shorter than 8 bytes beyond the header, and whether `actual` (a `usize`) is sensible.
3. **Oracle evidence beyond the DEC (important lesson from SPEC-029: the DEC's measurements missed variants and cost a fix round).** ExifTool 13.55 is installed. Build tiny WebPs and look at `exiftool -a -G1 -s -n` for the cases the DEC did NOT measure and report what you find: `ICCP` with the flag unset; `ALPH` before `VP8 `; a lossy `VP8 ` simple file with EXIF appended; `EXIF` immediately after the 12-byte header; an `XMP ` chunk of zero length; chunk code `XMP\0` with zero length; a RIFF header whose form type is `WEBP` but whose declared size is below 4; an `EXIF` chunk whose size is odd and is the last chunk with no pad; two `VP8X` chunks; an `ANMF` that contains an `EXIF` sub-chunk followed by a top-level `EXIF`. For each, say whether our walker and `read` agree with ExifTool on which metadata is located, and what to change if not. Disagreements on located metadata are Medium; on warnings only, Low.
4. **The model's independence and strength**: `fuzz/shared/webp_riff_invariants.rs` derives chunks, vp8x, issues, blocks and regions from raw bytes with its own four-character-code table (no library parsing); it must be able to fail. Mutate the library on a scratch copy (rsync the tree elsewhere; do not mutate the branch) in at least eight ways (pad rule, declared-size limiting the walk, case-insensitive code match, `XMP\0` handling, flag-mismatch order, MissingPad condition, ExtraExif logic, cap off-by-one, vp8x canvas `+1`, halt suppression) and confirm tests or the model go red; list survivors and say whether each is equivalent. Say which mutants only the unit tests (not the model on the committed seeds) kill, as a seed-shape gap.
5. **Pins, seeds, scope**: 51 `webp_riff` seeds (35 walk, 16 `meta-`), older seeds (tiff, jpeg, png) byte-identical (run `cargo run --example fuzz_seeds` and check the tree is clean), `fuzz/Cargo.lock` not committed with a changed version, test-only arms in other modules change no behaviour, README claims honest (WebP walked and metadata located, still unsupported in `exif-audit`), no CHANGELOG entry, no new dependency, PNG and JPEG behaviour unchanged (the shared `TooManyChunks` wording is the only intended change).
6. Run `just lint`, `just test`, `just lint-fuzz`, `just validate`, `just forbid-unsafe`, `just cost-audit`, and a 20 s fuzz run of `webp_riff` yourself (the recipe hardcodes 60 s; use its cargo command with `-max_total_time=20`).

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
