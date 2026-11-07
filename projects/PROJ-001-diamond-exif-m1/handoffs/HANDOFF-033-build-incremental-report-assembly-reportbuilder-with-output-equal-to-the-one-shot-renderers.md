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
  id: HANDOFF-033
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-06
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-018

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

# HANDOFF-033: Incremental report assembly: ReportBuilder with output equal to the one-shot renderers

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-018` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

Read before starting:

- **Spec (API, rules 1 to 6, the fuzz guard, named tests and the 12 mutants are normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-018-incremental-report-assembly-reportbuilder-with-output-equal-to-the-one-shot-renderers.md`
- **DEC-010 and its two amendments (normative for the report content; read it, do not edit it):** `decisions/DEC-010-report-rendering-contract.md`. DEC-011 (`decisions/DEC-011-exif-audit-cli-contract.md`) explains why this spec exists (the binary holds one input file in memory at a time); you implement only the library part.
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe; macOS `sed -i ''`)
- Code: `src/report/{mod,json,csv,text,safe}.rs` (the per-file fragment writers `file_json`, `file_block` and the CSV row writer already exist), `tests/report_render.rs`, `tests/report_formats.rs`, `tests/golden/*`, `fuzz/shared/render_invariants.rs`, `fuzz/fuzz_targets/audit_render.rs`, `tests/fuzz_layout.rs`
- Prior art: HANDOFF-031/032 (SPEC-015 build and verify): the guard style (named guards run alone through `run_guard`, both directions of every iff, plants in `tests/fuzz_layout.rs`, step counter), and the lessons that cost follow-ups there: a timing assertion must be a ratio, never an absolute bound; a guard is only as strong as the seeds that reach it.

Constraints: `panic-free-library` (five-lint deny set: no unchecked `+`, `*`, `<<`; use `saturating_add`, iterators; never index a slice or string), `forbid-unsafe-code`, `no-io-in-library`, `fuzz-target-per-surface` (the existing `audit_render` target is extended; no new target), `test-before-implementation`, `oracle-must-be-shown-red` (applied to the mutant list), `no-new-top-level-deps-without-decision`.

## Expected Deliverables

- Everything in the spec's Outputs and every Acceptance Criterion; every named test under its name (a rename, fold or missing test is a deviation to report). Edits are in `src/report/*` (new `Format`, `ReportBuilder`, `render`; the one-shot `render_text`, `render_json`, `render_csv` and `summarize` rewritten on top of the shared parts so there is one code path), `tests/report_render.rs`, `fuzz/shared/render_invariants.rs` (the `builder_equals_oneshot` guard), `tests/fuzz_layout.rs` (plants, linearity). `audit()`, the preview, the escaping, every golden file and every existing test expectation stay unchanged.
- Order: the equivalence tests first (red against a stub builder), then refactor the renderers into fragment + assembly parts, then make the one-shot functions call the builder parts, then the harness guard and plants, then the 12 mutants (for each: control first, apply by hand, show the named failure with its assertion text, revert, `git diff` empty).
- A doubt about a DEC-010 rule is a question for the architect: write it in the Handback and `guidance/questions.yaml`; do not edit `decisions/`.
- Run locally, because they are CI-only otherwise: `just lint-fuzz-red-proof`, `just lint-red-proof`, `just oracle-record` (empty `git status --porcelain` afterwards), and `just fuzz-smoke audit_render` (60 s; record executions and coverage); also confirm a plant of the new guard is found by the real `audit_render` target.
- Commit on your worktree branch. **Do not push, do not open a PR, do not advance cycles.** First `git merge --ff-only claude/rust-exif-metadata-6351d5` (you need the commit containing HANDOFF-033); if refused, use `git merge` and say so.
- Do not modify irradiance, `scripts/`, `decisions/`. No dependencies. Scratch work goes in a scratchpad directory, not `/tmp`.
- You cannot see your own token count; set `tokens_total: null` with a note, the orchestrator transcribes the metered figure.

## Out of Scope

- Any change to report content, schema, escaping, statuses or goldens; streaming to a writer; bounding report size
- The binary, argument parsing, file I/O (SPEC-016)
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
