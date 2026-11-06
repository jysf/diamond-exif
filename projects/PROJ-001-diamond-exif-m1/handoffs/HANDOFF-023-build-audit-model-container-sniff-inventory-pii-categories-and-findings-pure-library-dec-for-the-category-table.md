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
  id: HANDOFF-023
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-05
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-012

project:
  id: PROJ-001
  stage: STAGE-003
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

# HANDOFF-023: Audit model (sniff, inventory, PII categories and findings)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-012` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

Read before starting:

- **Spec (API shape, rules, named tests are normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-012-audit-model-container-sniff-inventory-pii-categories-and-findings-pure-library-dec-for-the-category-table.md`
- **`decisions/DEC-008-audit-categories-and-severity.md`** (the category table and severities; normative; copy the table into the test as data, do not generate it from the code), DEC-005, DEC-006, DEC-007, DEC-002
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof` and `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe)
- Code to build on: `src/tiff/mod.rs` (`Container`, `Ifd`, `Entry`, `IfdKind`), `src/jpeg/{mod,metadata}.rs`, `src/lib.rs`, `fuzz/fuzz_targets/jpeg_segments.rs`, `fuzz/shared/jpeg_metadata_invariants.rs`, `examples/fuzz_seeds.rs`, `tests/fuzz_layout.rs`, `tests/support/{tools,oracle_tiff,oracle_jpeg}.rs`, `.github/workflows/ci.yml` (the `fuzz-smoke` matrix), `app.just`
- Prior art: HANDOFF-019/020 (SPEC-010: shared fuzz harness, iff-invariants, plants), HANDOFF-021/022 (SPEC-011: oracle records, CI-only scripts)

Constraints: `panic-free-library` (five-lint deny set in non-test code), `forbid-unsafe-code`, `no-io-in-library`, `fuzz-target-per-surface`, `test-before-implementation`, `oracle-must-be-shown-red`, `no-new-top-level-deps-without-decision` (no dependencies of any kind). ExifTool is run, never read.

## Expected Deliverables

- Everything in the spec's Outputs with every named test and Acceptance Criterion, including the live DEC-008 table-name check with its committed record, the Container amplification measurement (report numbers: command, input size, peak memory, time), and the `audit` fuzz target with seeds, invariants (each iff both directions, plus the overlapping-guard variant), plants shown red (at least one against the real libFuzzer target) and a clean 60 s `just fuzz-smoke audit` with executions and coverage.
- **If ExifTool names a table id differently from DEC-008, or does not name it:** do NOT edit `decisions/` or the table. Record the measured command and output in the Handback and `guidance/questions.yaml`; I decide.
- **Run `just lint-fuzz-red-proof` and `just lint-red-proof` locally** (CI-only otherwise; the fuzz one broke PR 14 when a target started `include!`ing a new directory: if you add a shared include dir, confirm the script copies it). Run `just oracle-record` and confirm a clean `git status` afterwards; the existing 120 records and 66 seeds must stay byte-identical.
- CI: add `audit` to the `fuzz-smoke` matrix and nothing else in the workflow. Validate the YAML parses.
- Name every deviation (renamed, folded or missing tests; any derived number that does not match what the rules produce: stop and report rather than adjusting) in the Handback.
- Commit on your worktree branch. **Do not push and do not open a PR.** The spec is at `cycle: build`; do not advance cycles. First run `git merge --ff-only claude/rust-exif-metadata-6351d5` so your worktree has the commit containing HANDOFF-023 (and `290441d` or later).
- Do not modify irradiance, `scripts/`, `decisions/`, or the TIFF/JPEG reader behaviour (a reader bug found is reported, not fixed). No dependencies or dev-dependencies.
- You cannot see your own token count; set `tokens_total: null` with a note, the orchestrator transcribes the metered figure.

## Out of Scope

- Strip policies and previews (SPEC-014), renderers (SPEC-015), the binary (SPEC-016), inventory parity beyond the table-name check (SPEC-013)
- Reading inside thumbnails, MakerNote parsing, XMP/ICC/IPTC/C2PA decoding, PNG/WebP/HEIC
- Fixing a Container memory amplification if found (report it)
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
