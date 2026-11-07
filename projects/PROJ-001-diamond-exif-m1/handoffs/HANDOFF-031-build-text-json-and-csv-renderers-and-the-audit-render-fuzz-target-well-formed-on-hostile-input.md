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
  id: HANDOFF-031
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-06
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-015

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

# HANDOFF-031: Text, JSON and CSV renderers and the audit_render fuzz target

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-015` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

Read before starting:

- **Spec (API, rules 1 to 9, JSON schema, CSV columns, checkers, fuzz guards, named tests and the 30 mutants are normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-015-text-json-and-csv-renderers-and-the-audit-render-fuzz-target-well-formed-on-hostile-input.md`
- **DEC-010 (normative for escaping, the value-free default, the JSON and CSV shapes and file status; read it, do not edit it):** `decisions/DEC-010-report-rendering-contract.md`
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe; macOS `sed -i ''`)
- Decisions: DEC-008, DEC-009 (dispositions, `reliable`, the 2026-11-06 amendment's wording rules), DEC-006
- Code: `src/audit/{mod,preview,category}.rs`, `src/policy.rs`, `src/tiff/mod.rs` (`Container::payload`, `IfdKind`), `fuzz/shared/audit_invariants.rs` and `fuzz/fuzz_targets/audit.rs` (the pattern to copy: guards run alone through `run_guard`, step counter, plants in `tests/fuzz_layout.rs`), `tests/fuzz_layout.rs`, `tests/audit_model.rs` (seed counts 32/37/76 become 32/37/78), `tests/policy_preview.rs`, `examples/fuzz_seeds.rs`, `fuzz/Cargo.toml`, `.github/workflows/ci.yml`, `app.just`
- Prior art: HANDOFF-029/030 (SPEC-014 build and verify: the verify findings F1 to F3 are carried into this spec) and HANDOFF-027/028 (SPEC-013: the linearity weakness).

Constraints: `panic-free-library` (five-lint deny set: `arithmetic_side_effects` forbids unchecked `+`, `*`, `<<`; use `saturating_add`, `checked_*`, iterators; never index a string by byte position), `forbid-unsafe-code`, `no-io-in-library`, `fuzz-target-per-surface`, `test-before-implementation`, `oracle-must-be-shown-red` (applied to the mutant list, including the checker mutants), `no-new-top-level-deps-without-decision`.

## Expected Deliverables

- Everything in the spec's Outputs and every Acceptance Criterion; every named test under its name (a rename, fold or missing test is a deviation to report). New files: `src/report/{mod,safe,text,json,csv}.rs`, `fuzz/shared/render_check.rs`, `fuzz/shared/render_invariants.rs`, `fuzz/fuzz_targets/audit_render.rs`, `tests/report_render.rs`, `tests/render_check.rs`, `tests/golden/report-full-house.{txt,json,csv}`, `fuzz/seeds/audit_render/*`; edits: `src/lib.rs`, `src/audit/mod.rs` (`Item::value`, additive only), `fuzz/Cargo.toml`, `.github/workflows/ci.yml` (matrix entry), `app.just` if `fuzz-smoke` needs the target, `examples/fuzz_seeds.rs` (new seeds), `tests/audit_model.rs` and the seed hash lists (the two full-house `audit` seeds), `tests/fuzz_layout.rs`. `audit()` results and every existing test expectation stay unchanged except the seed counts and hash lists the spec names.
- Order: `safe_text` and its tests; the two checkers and their valid/invalid tables (test the testers before using them); `Item::value`; `status_of` and `summarize`; JSON; CSV; text and the golden files; the render guards and plants; the `audit_render` target, seeds and CI entry; the carried items (SPEC-014 F1 to F3, the SPEC-013 linearity wall-clock bound); then the 30 mutants plus the five carried ones (for each: control first, apply by hand, show the named failure with its assertion text, revert, `git diff` empty).
- Type DEC-010's escape list, the JSON key lists and the CSV header **from the DEC and the spec by hand** in the harness; none may be imported from `src/report`. The checkers must not call anything in `src/report`.
- A doubt about a DEC-010 rule is a question for the architect: write it in the Handback and `guidance/questions.yaml`; do not edit `decisions/`.
- Run locally, because they are CI-only otherwise: `just lint-fuzz-red-proof`, `just lint-red-proof`, `just oracle-record` (empty `git status --porcelain` afterwards), and `just fuzz-smoke audit` and `just fuzz-smoke audit_render` (60 s each; record executions and coverage). Report the wall-clock bound you chose, the measured time of the quadratic rescan mutant, and the largest output size at the caps.
- Commit on your worktree branch. **Do not push, do not open a PR, do not advance cycles.** First `git merge --ff-only claude/rust-exif-metadata-6351d5` (you need the commit containing HANDOFF-031); if refused, use `git merge` and say so.
- Do not modify irradiance, `scripts/`, `decisions/`. No dependencies. Scratch work goes in a scratchpad directory, not `/tmp`.
- You cannot see your own token count; set `tokens_total: null` with a note, the orchestrator transcribes the metered figure.

## Out of Scope

- File I/O, directory walking, argument parsing, exit codes (SPEC-016); value decoding; tag names; any ExifTool comparison
- Any change to `audit()` results, the presets or the preview
- Pushing, PRs, cycle advancement, SPEC-016

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
