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
  id: HANDOFF-015
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
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

# HANDOFF-015: Mutation helpers and mutated-copy red-proofs

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-008` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

Read before starting:

- **Spec (normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-008-mutation-helpers-and-mutated-copy-red-proof-apparatus.md`
- `docs/porting-brief-diamond-exif.md` §2.3, `AGENTS.md`, `guidance/toolchain-brief.md`, `guidance/constraints.yaml` (`oracle-must-be-shown-red`, `exiftool-never-linked`)
- SPEC-005's files: `tests/support/{tools,oracle,oracle_tiff}.rs`, `tests/metadata_oracle.rs`, `tests/oracle-fixtures/exiftool/*.txt`, and its handbacks HANDOFF-013 and HANDOFF-014 (what survived, what diverged)
- **Pattern source, READ ONLY, never modify:** `~/PSeven/experiments/crustimg_redo_plus/irradiance/tests/plane_oracle.rs` lines ~626-805 and ~1044 (`TempDir`, `copy_dir_recursive`, `inject_*_fault`, `stage_probe_crate`, `build_and_run_probe`, control test). Port with edit, add a ledger row (DEC-003). **ExifTool's own source must never be read.**

**Safety, read twice:** ExifTool 13.55 on this machine does not return on `fuzz/seeds/tiff_ifd/two-hop-cycle` and balloons to about 9 GB. Never run it on that file; never run any new fixture through it without the wall-clock limit this spec adds. When measuring a tool, capture stdout to a variable or file and read the tool's own status; never read `$?` through a pipe.

Host: ExifTool at `/opt/homebrew/bin/exiftool`. Toolchain traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint` under the default PATH; the probe must build with the cargo that runs the tests (`CARGO` env var) in its own target dir, `--offline`, and must not be redirected into the repo's `target/` by a stale `CARGO_TARGET_DIR`.

## Expected Deliverables

- Everything in the spec's Outputs with all its named tests and every Acceptance Criterion met. In particular:
  - every library-fault test has its **control first and green**, and the mutant red naming the stated selector; the needle guard is exact-once and itself tested;
  - the three byte-mutation red-proofs with their isolation assertions;
  - the `run_tool` time limit (unix test with `sleep`), `ToolError::Timeout`, `DIAMOND_EXIF_TOOL_TIMEOUT_SECS`;
  - the signed/float live selector, with every candidate tag you tried recorded (command and output) in the Handback; if you must use the documented fallback, justify it with four honest measured attempts;
  - the red-proofs run with ExifTool hidden; only live variants skip, loudly;
  - wall-clock of `just mutation-oracle` recorded.
- Name every deviation (renamed, folded or missing tests) in the Handback. Report anything that surprises you: a needle that is hard to place, a probe build that is slow, a red-proof that is hard to make fail.
- Commit on your worktree branch. **Do not push and do not open a PR.** The spec is already at `cycle: build`; do not advance cycles. First run `git merge --ff-only claude/rust-exif-metadata-6351d5` if your worktree was cut from an older commit (you need `2fc776c` or later).
- Do not modify irradiance, `scripts/`, `decisions/`, library code under `src/`, or the 29 fuzz seeds. No dependencies or dev-dependencies. No `RED-PROOF INJECTION` text may end up in a committed non-test file or in `src/`.
- You cannot see your own token count; set `tokens_total: null` with a note, the orchestrator transcribes the metered figure.

## Out of Scope

- JPEG-level red-proofs, ExifTool write mode, CI wiring, a memory limit, fixing a library bug a red-proof reveals (stop and report), pushing, PRs, cycle advancement

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
