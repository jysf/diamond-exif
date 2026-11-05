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
  id: HANDOFF-014
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
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

# HANDOFF-014: Verify the ExifTool read-parity oracle

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-005` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-005-oracle-discipline.md`
- **Build handoff and Handback:** `HANDOFF-013-build-oracle-discipline.md` (claims to verify, not to trust)
- `guidance/constraints.yaml` (`oracle-must-be-shown-red`, `exiftool-never-linked`), `docs/porting-brief-diamond-exif.md` §2.2-2.3, `guidance/toolchain-brief.md`, `guidance/questions.yaml` (new entries from the builder)
- Code under review: `git diff 2f8ad6a..HEAD` (build commit `9cba911`, handback `47d8ef9`)
- ExifTool 13.55 is at `/opt/homebrew/bin/exiftool`. **ExifTool's source must never be read.** Irradiance is read-only.

## Expected Deliverables

Review cold; output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

**Safety first:** the builder reports ExifTool 13.55 does not return on `fuzz/seeds/tiff_ifd/two-hop-cycle` (about 8.9 GB resident in 5 s, SIGKILLed by the OS). Never run ExifTool on that file without a hard wall-clock limit (for example `perl -e 'alarm 4; exec @ARGV' exiftool ...`). The harness deliberately does not run it; confirm that.

Verify independently:
- Re-run every gate in the spec; also run the full test suite with ExifTool present (the live tests) and with `PATH` scrubbed (loud SKIP, replay tests still run) and with `DIAMOND_EXIF_REQUIRE_ORACLE=1` and ExifTool hidden (live tests must fail).
- Confirm `parse_fields` never touches an exit status and the single `run_tool` choke point is the only place ExifTool is spawned (grep).
- **Red-proofs are the point.** Redo them yourself: (a) plant a one-field perturbation of our reading per `Norm` variant and confirm exactly one mismatch naming it; (b) plant library-side faults that the oracle must catch on the live fixtures, in a scratch copy of the tree, for example swap the byte order of one multi-byte read, make `ascii` drop the last byte, negate a GPS ref sign in `read_ours`, widen the float tolerance to 1e-2 and see whether any test notices; (c) delete a committed record's value and see the replay fail. For each plant, name which test went red; report any plant that survives. Revert everything.
- Judge the normalisation table and tolerance rules: are text/int compares exact? is `1e-9` relative defensible for 15-digit ExifTool decimals? could a systematic error hide inside the tolerance (e.g. GPS minutes vs seconds scaling error below 1e-9)? Try to construct one.
- Check the spec correction the builder reports (GPS unsigned vs `Composite:` selectors) by measurement; check `KNOWN_DIVERGENCES` entries carry command, version, measured output, which side is right and why from the format spec; confirm the two new questions in `guidance/questions.yaml` are accurate.
- Check the seed sweep rules against what the spec asked; look at the 29 seeds yourself for any where ExifTool and exif-core disagree that the harness missed or excluded (with the alarm guard for any suspect seed). Confirm `just oracle-record` reproduces the committed records exactly (empty `git diff`), except you must not run it unguarded on `two-hop-cycle` (the recorder skips it; confirm).
- Note: the orchestrator's earlier measurement in the spec said ExifTool exits 0 on every seed. That was measured through a pipeline and is **wrong**; judge the builder's correction and the comments in the code for any remaining over-generalised claim ("always", "never") about ExifTool.
- Verify `src/`, `scripts/`, `decisions/`, irradiance and the 29 seeds are untouched; `[dependencies]` and `[dev-dependencies]` empty.
- Judge the builder's deviations (Composite selectors, two-hop-cycle not run live, five extra tests, unit tests inside `tests/support/*`, `ifd1-corrupt` on IFD0 selectors only).
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes; SPEC-008 work (including adding a time/memory limit to `run_tool`, which the builder suggests for SPEC-008); CI wiring; ExifTool write mode

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
