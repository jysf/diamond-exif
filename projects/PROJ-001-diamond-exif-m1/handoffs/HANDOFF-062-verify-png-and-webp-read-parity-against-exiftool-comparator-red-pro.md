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
  id: HANDOFF-062
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-21
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-031

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

# HANDOFF-062: Verify SPEC-031 PNG read parity, round 2 (fixes for 3 Mediums)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-031` to a cold verifier for a SECOND, focused round. Round 1 (HANDOFF-061, read its handback first) returned PUNCH LIST with 3 Medium and 6 Low findings; the builder fixed them in commit `29df025` (bookkeeping commits after it). You did not write this and you are not the round-1 verifier. Review branch `claude/spec-031-png-parity` against the spec (`projects/PROJ-001-diamond-exif-m1/specs/SPEC-031-*.md`, the file without `-timeline`). Review only; edit nothing except your own handback in this file. Do not push.

## FIRST STEP

The harness may start your worktree from main. Run `git checkout -b claude/spec-031-verify2 claude/spec-031-png-parity` (local branch; do not `reset --hard`) and confirm HEAD contains `29df025`.

## BUDGET (hard)

About 70 tool calls or 180k context. Record `tokens_total: null` with a note if you cannot measure it (the orchestrator replaces it with the harness figure). Toolchain: `RUSTUP_TOOLCHAIN=1.90.0`, `PATH="$HOME/.cargo/bin:$PATH"`; read `guidance/toolchain-brief.md`. ExifTool 13.55 is installed (run it, never read its source); use `perl -e 'alarm 8; exec @ARGV' -- exiftool ...` for ad hoc runs.

## What to look hard at

1. **Were the three Mediums really fixed?** (M1) `iCCP` with names of 0, 1, 79, 80, 81, 200 bytes: re-measure with ExifTool and confirm `png::read` locates exactly what ExifTool reads; also no-NUL and NUL-without-method-byte and method byte 1. (M2) `iTXt` XMP keyword with language tag and translated keyword variants, both flags: re-measure at least eight variants (including an empty language tag with a non-empty translated keyword, a language tag of one byte, uppercase, and a language tag followed immediately by the end of the chunk) and confirm the routing agrees. (M3) the new test that located region bytes equal the builders' bytes: run your OWN mutants on a scratch copy (rsync the tree elsewhere; do not mutate the branch): at least six: XMP data shifted by one, ICC data the whole chunk, translated keyword NUL not skipped, raw-profile data offset wrong, `Text` data off by one, region offset field wrong. Every one must turn some PNG test red; report survivors and whether each is equivalent or a gap.
2. **Unmeasured shapes again.** Round 1 found its Mediums among shapes outside the fixture list; the builder added nine. Probe at least ten MORE shapes not in the list and report agreement with `png::read` on located metadata (disagreement on located metadata is Medium, warnings only Low). Suggestions: `iTXt` with compression flag 1 and method 1; `iTXt` keyword exactly 79 bytes with a language tag; `tEXt` XMP keyword followed by an empty packet; `zTXt` XMP with method byte 1; `eXIf` with `MM` header and zero IFD entries; `eXIf` whose IFD0 offset points inside the header; raw-profile text with the wrong declared length; raw profile of type `icc`, `iptc` and `APP1`; two `iCCP` chunks; `iCCP` after IDAT; a `Raw profile type exif` keyword in upper case.
3. **The fuzz model after the rule change.** `fuzz/shared/png_metadata_invariants.rs` was rewritten for the new iCCP and iTXt rules: it must still derive from raw bytes only, assert both directions, and fail on a planted fault (try three). Run the `png_chunks` fuzz target for 20 s (the recipe hardcodes 60 s; use its cargo command with `-max_total_time=20`). The two seeds whose parse result changed (`meta-iccp-empty-name`, `meta-xmp-itxt-compressed`) keep their bytes: confirm, and confirm every other seed is byte-identical (`cargo run --example fuzz_seeds` leaves the tree clean).
4. **DEC-018 and the divergence text.** Read the appended "Clarifications from the SPEC-031 verify" section: does it state exactly what the code does? Read the `png-exif-and-raw-exif` entry and the `png-raw-profile-not-decoded` question: is the privacy consequence (an audit or strip trusting the `eXIf` can miss values a reader shows, including raw-only tags such as GPS) stated plainly?
5. **Collateral.** Records: `just oracle-record` twice leaves no diff; only `png-warnings.txt` changed among existing records and 9 were added. No change to JPEG, TIFF or WebP behaviour; only `src/png/metadata.rs` changed under `src/` since `937b15f`.
6. Run `just lint`, `just test`, `just mutation-oracle`, `just validate`, `just cost-audit`, `just forbid-unsafe` yourself.

## Expected Deliverables

Your verdict, exactly one of APPROVED / PUNCH LIST / REJECTED, with findings ranked High, Medium, Low, each with file:line and a concrete failing input. Fill the `handback:` front matter (`verdict`) and the `## Handback` section; set statuses to `completed`; commit only the handback.

## Out of Scope

Fixing anything, further DEC edits, WebP parity (SPEC-034), audit wiring (SPEC-032).

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
