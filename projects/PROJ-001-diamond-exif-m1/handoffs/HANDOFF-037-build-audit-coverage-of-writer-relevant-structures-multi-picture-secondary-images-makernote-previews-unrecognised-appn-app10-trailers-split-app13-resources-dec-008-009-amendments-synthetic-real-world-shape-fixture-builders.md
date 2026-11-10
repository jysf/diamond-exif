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
  id: HANDOFF-037
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-09
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-019

project:
  id: PROJ-001
  stage: STAGE-004
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

# HANDOFF-037: Audit coverage of writer-relevant structures (SPEC-019)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-019` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

Read before starting:

- **Spec (rules 1 to 9, library changes, fixtures, oracle, fuzz and seeds, named tests and the 28 mutants are normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-019-audit-coverage-of-writer-relevant-structures-multi-picture-secondary-images-makernote-previews-unrecognised-appn-app10-trailers-split-app13-resources-dec-008-009-amendments-synthetic-real-world-shape-fixture-builders.md`
- **DEC-012 (normative for the new regions, the benign list, the two categories, the preset rows, `holds`, and schema_version 2; read it, do not edit it):** `decisions/DEC-012-structures-the-audit-names-for-the-writer.md`, with DEC-006, DEC-008, DEC-009, DEC-010 and their amendments.
- **Evidence:** `spikes/done/SPIKE-001-*.md` (what real files contained and how ExifTool saw them).
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe; macOS `sed -i ''`; in zsh never name a shell variable `path`)
- Code: `src/jpeg/{mod,metadata}.rs`, `src/audit/*`, `src/policy.rs`, `src/report/*`, `src/bin/exif-audit/*`, `tests/support/*`, the test files and harness files listed in the spec's Inputs, `tests/golden/*`, `fuzz/seeds/**`, `examples/fuzz_seeds.rs`.
- Prior art and lessons that cost follow-ups before: timing assertions are ratios, never absolute; a guard is only as strong as the seeds that reach it (add a seed for every new class); plants must vary in kind (length, content at the start, middle and end, order), not one dimension; type every expectation grid and name list from the DEC by hand, never from the code under test; record a measured ExifTool relation before writing the rule that depends on it (measure how ExifTool counts the MPF primary first); a skipped test is loud.

Constraints: `panic-free-library` (five-lint deny set: no unchecked `+`, `*`, `<<`; MPF offsets are untrusted `u32` sums: `checked_add`, `get(..)`, never index), `forbid-unsafe-code`, `no-io-in-library`, `fuzz-target-per-surface` (existing targets extended), `test-before-implementation`, `oracle-must-be-shown-red`, `no-new-top-level-deps-without-decision`.

## Expected Deliverables

- Everything in the spec's Outputs and every Acceptance Criterion; every named test under its name (a rename, fold or missing test is a deviation to report). New files include `tests/support/real_shapes.rs`, `tests/real_shapes.rs`, new oracle records under `tests/oracle-fixtures/exiftool-inventory/` and new seeds; edits across `src/jpeg`, `src/audit`, `src/policy.rs`, `src/report`, `src/bin/exif-audit` (wording only where needed), the oracle comparator and the three harness files. No existing seed or record changes bytes; the existing 178 inventory results are unchanged (class S exclusion is exact).
- Order: fixture builders and their ExifTool-reads-it tests first (and measure how ExifTool counts the MPF primary); the JPEG-layer regions and issues; audit categories, `holds`, `embedded`; policy grids and may-hold; renderers and goldens (regenerate once and review every changed line); oracle comparator, records and red-proofs; harnesses, seeds and plants; CLI tests; then the 28 mutants (control first, apply by hand, show the named failure with its assertion text, revert, `git diff` empty).
- The local corpus re-run (spec Acceptance Criteria): the folder `/Users/jyashinsky/PSeven/experiments/crustimg_redo_plus/_incoming0` holds the user's real photos. **Read it in place only; copy nothing into the repo; print and record only counts, file short hashes and category or region names; never tag values, never file contents, never a path of a file inside it in committed text.** Use the release `exif-audit` with `--json --policy none --inventory` (never `--values`). If a public sample set from SPIKE-001 is still present under the session scratchpad `public/` directory, include it; if not, say so.
- A doubt about a DEC-012 rule is a question for the architect: write it in the Handback and `guidance/questions.yaml`; do not edit `decisions/`.
- Run locally, because they are CI-only otherwise: `just lint-fuzz-red-proof`, `just lint-red-proof`, `just oracle-record` (empty `git status --porcelain` afterwards), `just fuzz-smoke jpeg_segments`, `audit`, `audit_render` (60 s each; record executions and coverage), and confirm one new-region guard plant is found by the real libFuzzer target.
- Commit on your worktree branch. **Do not push, do not open a PR, do not advance cycles.** First `git merge --ff-only claude/rust-exif-metadata-6351d5` (you need the commit containing HANDOFF-037); if refused, use `git merge` and say so.
- Do not modify irradiance, `scripts/`, `decisions/`. No dependencies. Scratch work goes in a scratchpad directory, not `/tmp`.
- You cannot see your own token count; set `tokens_total: null` with a note, the orchestrator transcribes the metered figure.

## Out of Scope

- Writing, stripping or changing any byte; MakerNote parsing or locating previews inside MakerNotes; PNG, WebP, HEIC; a new fuzz target; new CLI options or changed exit codes
- Pushing, PRs, cycle advancement, SPEC-020 to SPEC-022

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
