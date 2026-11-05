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
  id: HANDOFF-016
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-05
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-008

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

# HANDOFF-016: Verify the mutation helpers and mutated-copy red-proofs

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-008` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-008-mutation-helpers-and-mutated-copy-red-proof-apparatus.md`
- **Build handoff and Handback:** `HANDOFF-015-build-mutation-helpers-and-mutated-copy-red-proof-apparatus.md` (claims to verify, not to trust)
- SPEC-005 context: `HANDOFF-013`/`HANDOFF-014`; `guidance/constraints.yaml` (`oracle-must-be-shown-red`, `exiftool-never-linked`); `AGENTS.md`; `guidance/toolchain-brief.md`
- Code under review: `git diff 4221d5b..HEAD` (build commit `390d2b8`)
- ExifTool 13.55 at `/opt/homebrew/bin/exiftool`. **Its source must never be read.** Irradiance is read-only.

## Expected Deliverables

Review cold; output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

**Safety first:** ExifTool 13.55 does not return on `fuzz/seeds/tiff_ifd/two-hop-cycle` (about 9 GB). Never run it on that file. This host has no `timeout(1)`; for any ad hoc ExifTool probe use `perl -e 'alarm 4; exec @ARGV' exiftool ...`. Measure tools by capturing stdout to a variable and reading the tool's own status, never `$?` through a pipe.

Verify independently:
- Re-run every gate; also `just mutation-oracle` (record wall-clock) and `just oracle-record` (empty `git diff`, `two-hop-cycle` reported NOT RUN). Run the whole suite with `PATH` scrubbed of ExifTool and `DIAMOND_EXIF_REQUIRE_ORACLE` unset: only the live variants may skip, loudly.
- **The red-proofs are the point; test the testers.** For each of the six library-fault tests, confirm in the code what the needle injects and that the mutation is what the name says; then break the red-proof itself: e.g. (a) weaken a control (make the unmutated copy differ) and confirm the test fails rather than passes; (b) change a mutant's expected selector set and confirm it fails; (c) make a needle match zero or two places and confirm loud failure; (d) inject a mutation that should NOT show (little-endian fixture for the big-endian mutant) and confirm the test asserts the null result; (e) try mutants of your own that the builder did not (for example `floats` DOUBLE read as FLOAT width, `ascii` treating a NUL as not a terminator, wrong IFD kind label, off-by-one in the SubIFDs cap) and report which the suite catches and which survive.
- Judge the deviations: mutant 1 placed at the SHORT use site in `Container::uints` rather than the `ByteOrder::u16` big-endian arm (is the mutation what the spec wanted? is it still the "byte order" fault?); the extra fixtures (`exif-gps-full-le`, `signed-float-le`, `signed-float-be`) and the re-recorded 9 old records (confirm the only change is the added `-` columns); mutation 9's two cuts; `run_tool_with_limit` as the single spawn function; the probe's own cargo/probe run having no wall-clock limit (is that acceptable? could a mutant that loops forever hang the suite? try a mutant that loops).
- Check the new signed/float selectors: re-measure each of the four with ExifTool (command, version) and judge whether the comparison is honest (ExifTool honouring the declared type), including `ExifIFD:ExposureTime` as FLOAT 0.125 and `FocalLength` as DOUBLE 4.5.
- Timeout: confirm `kill -0` style evidence that the child is gone, default 5 s, env override, and that `run_tool_with_limit` is the only place ExifTool is spawned (grep). Confirm `RED-PROOF INJECTION` appears only in test code.
- Check `TempDir` cleanup (no stray temp dirs after the full suite and after a forced panic), that nothing touches the repo `target/` from the probe, and that a stale `CARGO_TARGET_DIR` in the environment cannot redirect the probe builds.
- Verify `src/`, `scripts/`, `decisions/`, irradiance and the 29 seeds are untouched; `[dependencies]` and `[dev-dependencies]` empty.
- Look for over-generalised claims about ExifTool ("always", "never", "every time") in comments and docs added by this spec.
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes; CI wiring; JPEG-level red-proofs; a memory limit

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
