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
  id: HANDOFF-061
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

# HANDOFF-061: Verify SPEC-031 PNG read parity against ExifTool

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-031` to a cold verifier. You did not write this. Review branch `claude/spec-031-png-parity` (build commits `79712b5` checkpoint 1 and `358322b` checkpoint 2, cost commit after) against the spec (`projects/PROJ-001-diamond-exif-m1/specs/SPEC-031-*.md`, the file without `-timeline`). This is an evidence spec: your job is to decide whether the evidence is real and the red-proofs bite. Review only; edit nothing except your own handback in this file. Do not push.

## FIRST STEP

The harness may start your worktree from main. Run `git checkout -b claude/spec-031-verify claude/spec-031-png-parity` (local branch; do not `reset --hard`) and confirm HEAD contains `358322b`.

## BUDGET (hard)

About 80 tool calls or 200k context. Record `tokens_total: null` with a note if you cannot measure it (the orchestrator replaces it with the harness figure). Toolchain: `RUSTUP_TOOLCHAIN=1.90.0`, `PATH="$HOME/.cargo/bin:$PATH"`; read `guidance/toolchain-brief.md`. ExifTool 13.55 is installed (run it, never read its source). Use a timeout-safe approach for any ad hoc ExifTool run (`perl -e 'alarm 8; exec @ARGV' -- exiftool ...`).

## What to look hard at

1. **Is the evidence real?** Re-run ExifTool yourself on at least 12 committed fixtures (write the fixture bytes out through the harness or a scratch example, run `exiftool -T -n -s3` with the recorded selectors) and compare to the committed `png-*.txt` records byte for byte. Check that the record headers carry the real command, version and stderr. Check `just oracle-record` is idempotent (twice, no diff).
2. **Fixture independence.** `tests/support/oracle_png.rs` must share no encoding code with `src/`, `examples/fuzz_seeds.rs`, `fuzz/shared/*` or `tests/support/png_shapes.rs`. Check imports and look for copied logic. Its CRC-32, Adler-32 and stored-deflate vectors must be checked against published values by a test that can fail.
3. **Comparator honesty.** The builder defined the XMP selector as readable XMP and the ICC selector as `PNG:ProfileName`. Judge whether that hides anything: could a located-but-wrong region pass? The several-`eXIf` finding (ExifTool reads only the first block that is a TIFF, unlike JPEG) is claimed measured: re-measure it on at least five variants including garbage-then-good, good-then-garbage, `Exif\0\0`-prefixed first, and two good blocks with different IFD0 content, and confirm the harness model and `PngMetadata::exif()` agree on every one.
4. **Divergences.** The raw-profile and compressed-XMP entries must each have the measured stdout, the right-side sentence and a `questions.yaml` id; re-measure two of them. One matters for the audit: with `eXIf` plus `Raw profile type exif` both present, ExifTool shows the RAW profile's value (`IFD0:Make = RawCam`) while we read the `eXIf` (`AlphaCam`). Say whether the divergence entry and question text make that privacy-relevant consequence clear (an audit that trusts the `eXIf` can miss what a reader sees). Check that the test requiring each divergence to be PRESENT really fails when a divergence disappears.
5. **Red-proofs.** Run `just mutation-oracle`. For the seven mutants and two byte mutations: confirm each control is green first, each mutation is marked `RED-PROOF INJECTION`, located by a needle that must occur exactly once, and the leak test works. Then add your OWN mutants on a scratch copy (rsync the tree elsewhere; do not mutate the branch): at least five library faults the builder did not use (for example: `iCCP` name length limit, text keyword NUL search, `after_idat` flag, `ExtraExif` logic, region cap, `BadTextChunk` routing), and report which turn the PNG comparison red and which survive (and whether the survivor is equivalent or a gap in fixtures).
6. **Shape coverage (lesson from SPEC-029/030: unmeasured variants cost fix rounds).** Probe ExifTool on PNG variants the fixture list does NOT contain and report agreement with `png::read` on located metadata: for example `eXIf` of exactly 8 bytes, `eXIf` with a big-endian TIFF whose IFD offset is odd, `iTXt` XMP with a non-empty language tag and translated keyword, `iCCP` with name length 79 and 80, a `tEXt` with an empty value, `zTXt` with method byte 1, two IHDR chunks, an `IDAT`-less file with `eXIf`. Disagreements on located metadata are Medium; on warnings only, Low.
7. **Collateral changes.** The builder added a `.filter` skipping `png-*` records in `tests/jpeg_oracle.rs`, `tests/budget_oracle.rs`, `tests/inventory_oracle.rs`: confirm these weaken nothing (they must still cover every non-PNG record). `app.just` change to `just mutation-oracle` is sound. No change under `src/` or `fuzz/` (diff against `a04413d`). README and counts.
8. Run `just lint`, `just test`, `just mutation-oracle`, `just validate`, `just cost-audit`, `just forbid-unsafe` yourself.

## Expected Deliverables

Your verdict, exactly one of APPROVED / PUNCH LIST / REJECTED, with findings ranked High, Medium, Low, each with file:line and a concrete failing input. Fill the `handback:` front matter (`verdict`) and the `## Handback` section; set statuses to `completed`; commit only the handback.

## Out of Scope

Fixing anything, DEC edits, WebP parity (SPEC-034), audit wiring (SPEC-032).

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
