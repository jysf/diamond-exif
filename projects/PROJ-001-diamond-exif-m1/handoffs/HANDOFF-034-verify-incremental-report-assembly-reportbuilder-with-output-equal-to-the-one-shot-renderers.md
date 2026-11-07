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
  id: HANDOFF-034
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
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

# HANDOFF-034: Verify the incremental ReportBuilder

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-018` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-018-incremental-report-assembly-reportbuilder-with-output-equal-to-the-one-shot-renderers.md` (API, rules 1 to 6, the fuzz guard, named tests and mutants are normative)
- **DEC-010 and its two amendments**, **DEC-011** (why the builder exists): `decisions/`
- **Build handoff and Handback:** `HANDOFF-033-build-...md` (claims to verify, not to trust; read the deviations, especially the one-shot renderers now being thin wrappers over the builder, and mutant 2 and 11's coverage)
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe)
- Code under review: `git diff 8d7ca73..HEAD` (build commits `19ba3cd`, `d33bd6e`; `cd1b26a` and later are docs). Main: `src/report/{mod,json,csv,text,builder_tests}.rs`, `tests/report_render.rs`, `fuzz/shared/render_invariants.rs`, `tests/fuzz_layout.rs`.

## Expected Deliverables

Review cold; output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

Verify independently:
- Re-run every gate (fmt both graphs and the harness files, `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `cargo test` on stable and 1.90.0 also with `DIAMOND_EXIF_REQUIRE_ORACLE=1`, `-F unsafe_code`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `oracle-record` with an empty `git status --porcelain`). Run `just fuzz-smoke audit_render` 60 s; record executions and coverage. Confirm: every file under `tests/golden/` is byte-identical to `8d7ca73`; `audit()`, the preview, the escaping (`src/report/safe.rs`) and `decisions/`, `scripts/`, `Cargo.toml` untouched; no existing test expectation relaxed (`git diff 8d7ca73..HEAD` on existing tests: show every changed line and judge it).
- **The central claim: one code path.** The build made the one-shot `render_*` functions wrappers over the builder, so builder-equals-one-shot is partly true by construction. Find what the equivalence tests and the guard can and cannot see. Is the property actually meaningful (does anything still check the fragments and the assembly are *right*, not merely consistent)? List the ways the one-shot output could be wrong while every equivalence check passes, and show the existing independent checks (goldens, the strict JSON and CSV checkers, the facts guards) catch them.
- **Check the rules by hand:** rule 2 (no retained borrow: read the struct; is anything lifetime-bound or an index into the entry?), rule 3 (order, independence, same entry twice), rule 4 (JSON frame, CSV header exactly once, text summary last, empty builder), rule 5 (memory bound: the fragments are `Vec<String>`; is the stated bound honest? measure retained size growth when adding a 50 MB-class entry built over a buffer that is then dropped), `Summary::add_file` (does the running summary equal `summarize` at every step, including `max_severity`, `reliable_files`, `residual_findings` with and without a policy and the DEC-010 amendment 1 rule that it is `None` when no file is reliable).
- **Test the testers.** Re-run the builder's mutants (the report claims 12 red) at least for 2, 5, 8, 9, 11 and 12. Write mutants of your own that the builder did not try, for example: the CSV builder emitting the header only if the first added entry has findings; the text builder skipping the blank line between blocks only for Failed entries; `finish` consuming fragments in a different order than `add` only when more than 2 entries; `summary()` double-counting after `finish`-like state; the JSON builder re-escaping a fragment; a builder that clones the options at `new` but reads a global default for `inventory`; the guard comparing lengths only; one plant that cannot go red because its control also fails. Report which the suite catches (name the test or guard) and which survive.
- Redo one `builder_equals_oneshot` plant against the real `audit_render` libFuzzer target, and confirm the step-counter linearity test covers the new guard and is a ratio, not an absolute bound.
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes; SPEC-016 work (the binary); changes to report content; any ExifTool comparison

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
