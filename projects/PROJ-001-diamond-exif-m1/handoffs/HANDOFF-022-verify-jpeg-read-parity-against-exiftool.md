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
  id: HANDOFF-022
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-05
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-011

project:
  id: PROJ-001
  stage: STAGE-002
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

# HANDOFF-022: Verify JPEG read parity against ExifTool

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-011` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-011-jpeg-read-parity-against-exiftool.md` (outputs, named tests, process rules normative; its ExifTool table is a prediction the build re-measured); **DEC-007**, DEC-006
- **Build handoff and Handback:** `HANDOFF-021-build-jpeg-read-parity-against-exiftool.md` (claims to verify, not to trust; read its deviations list: wildcard `exif:*` divergence, m4/m5/m3 semantics, seeds compared on Exif selectors only, temp-file race fix)
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure ExifTool exit status through a pipe)
- Code under review: `git diff c79ed73..HEAD` (build commit `2d5cade`); main new files `tests/jpeg_oracle.rs`, `tests/support/oracle_jpeg.rs`, the `tests/support/{oracle,probe,tools}.rs` edits, `src/jpeg/{mod,metadata}.rs` (DEC-007 only)

## Expected Deliverables

Review cold; output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

Verify independently:
- Re-run every gate (fmt both graphs, `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `cargo test` on stable and 1.90.0, also once with `DIAMOND_EXIF_REQUIRE_ORACLE=1`, `-F unsafe_code`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `oracle-record` with a clean `git status` afterwards). Run `just fuzz-smoke jpeg_segments` for 60 s and record executions and coverage.
- **Re-measure the claims against ExifTool yourself** (the tool is installed; run it, never read its source): the DEC-007 cases (sixth byte non-zero read; fifth byte non-NUL not; 5-byte payload not; APP2 not), multi-Exif (same tag both orders, disjoint tags), Exif after the scan, last-segment-at-EOF, the 17-block case. Does each recorded divergence reproduce, and is each "right side" sentence defensible? Is any difference papered over by the wildcard `exif:*` entry or by seeds being compared on Exif selectors only?
- **Judge the harness model** (`read_ours_jpeg`: union, later block wins): does it match ExifTool for fixtures with a *damaged* later block, a later block that is fatal, and more than 16 blocks? Can it hide a library bug (e.g. mask a wrong primary)? Read `compare_jpeg` and the stale-divergence logic: can a divergence list entry rot or loosen a comparison?
- **Test the testers.** Redo m1-m7; write mutants the builder did not try (e.g. `read_ours_jpeg` picking the first block, skipping fatal blocks wrongly, presence compare inverted for XMP, IPTC id 0x0405 accepted, DEC-007 off by one on the length test `>= 6` vs `> 6`, `tiff` slice at 7, a wrapped-fixture builder that silently drops a TIFF, the probe limit constants, a record writer that drops `# stderr:` lines, the marker-leak scan skipping `tests/support`) and report which the suite catches and which survive. Redo the fuzz plant against the real target. Confirm every named test in the spec exists (list renames/missing).
- Check the records: the 41 TIFF records are byte-identical to `c79ed73`; JPEG records carry command, version, source and `# stderr:`; the re-record reproduces them; the `exif:*` handling in replay equals live. Check the CI change (`oracle-records` now uses `git status --porcelain`): does it fail on a new untracked record, and pass on a clean tree? Confirm no other CI job changed.
- Check the seeds: existing 63 byte-identical; 3 new ones deterministic and claiming their shapes (`xxd`). Check no dependencies, `scripts/`, `decisions/` (apart from DEC-006/DEC-007 as written at design), irradiance untouched.
- Judge the temp-file race fix (`tools.rs`) and the probe `Child::kill` semantics (orphaned `rustc`): acceptable or a finding?
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes, the audit-stage merged Exif view, C2PA, CI wiring beyond reading it

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
