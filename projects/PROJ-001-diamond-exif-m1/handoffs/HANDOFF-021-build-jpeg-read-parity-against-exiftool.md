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
  id: HANDOFF-021
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
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

# HANDOFF-021: JPEG read parity against ExifTool

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-011` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

Read before starting:

- **Spec (measured table, outputs, named tests and process rules are normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-011-jpeg-read-parity-against-exiftool.md`
- **`decisions/DEC-007-exif-signature-is-five-bytes.md`** (the one library change), DEC-006, DEC-001, DEC-002, DEC-004
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; `just lint` and `just lint-fuzz-red-proof` under the default PATH; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe)
- Existing oracle apparatus: `tests/support/{tools,oracle,oracle_tiff,probe,mutate}.rs`, `tests/{metadata_oracle,mutation_oracle,fuzz_layout}.rs`, `examples/{oracle_record,fuzz_seeds}.rs`, `fuzz/shared/jpeg_metadata_invariants.rs`, `fuzz/fuzz_targets/jpeg_segments.rs`, `src/jpeg/{mod,metadata}.rs`
- Prior art: HANDOFF-015/016 (SPEC-008 mutated-copy apparatus and what verify caught), HANDOFF-019/020 (SPEC-010)

Constraints: `oracle-must-be-shown-red`, `panic-free-library` (five-lint deny set in non-test code), `forbid-unsafe-code`, `fuzz-target-per-surface`, `test-before-implementation`, `no-new-top-level-deps-without-decision` (no dependencies of any kind). ExifTool is run, never read.

## Expected Deliverables

- Everything in the spec's Outputs with every named test and Acceptance Criterion, including both design-section-8 JPEG red-proofs, mutated-copy faults m1-m7 (control run first, named mismatch each), the five-byte fuzz invariant shown red against the real libFuzzer target, and a clean 60 s `just fuzz-smoke jpeg_segments` (executions and coverage recorded).
- The spec's ExifTool table is a **prediction from the design session**: re-measure every row through the real harness. Where your measurement differs, the measurement wins; list every such row in the Handback.
- Probe each new JPEG shape through ExifTool **alone, under the limit,** before it joins any loop. A shape ExifTool does not return on becomes a `"*"` divergence and is never run through the tool.
- The 41 TIFF records must stay byte-identical (CI's `oracle-records` job diffs them). The existing 63 seeds (29 TIFF + 34 JPEG) must stay byte-identical.
- **Run `just lint-fuzz-red-proof` and `just lint-red-proof` locally** (they are otherwise CI-only; the last PR failed in CI because I did not). Also run `just oracle-record` and confirm an empty diff for the TIFF records.
- Name every deviation (renamed, folded or missing tests; derived numbers that do not match) in the Handback; if a divergence points at a library bug other than DEC-007, stop that test and report it instead of fixing it.
- Commit on your worktree branch. **Do not push and do not open a PR.** The spec is at `cycle: build`; do not advance cycles. First run `git merge --ff-only claude/rust-exif-metadata-6351d5` so your worktree has `c79ed73` or later.
- Do not modify irradiance, `scripts/`, `decisions/` or the ExifTool TIFF records. No dependencies or dev-dependencies.
- You cannot see your own token count; set `tokens_total: null` with a note, the orchestrator transcribes the metered figure.

## Out of Scope

- A merged multi-Exif view in the library; ignoring Exif after the scan; ICC/JUMBF/C2PA parity; XMP/IPTC value comparison; SBYTE selector; writing; the `exif-audit` CLI
- Pushing, PRs, cycle advancement

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
