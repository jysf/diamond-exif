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
  id: HANDOFF-026
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-05
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-017

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

# HANDOFF-026: Verify the entry budget

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-017` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-017-bound-container-memory-a-total-entry-budget-for-the-tolerant-walk.md` (rule, outputs, named tests normative); **DEC-001** incl. its SPEC-017 clarification
- **Build handoff and Handback:** `HANDOFF-025-build-bound-container-memory-...md` (claims to verify, not to trust; read deviations 1-5)
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe)
- Code under review: `git diff 9e37c75..HEAD` (build commit `163feab`); main: `src/tiff/mod.rs` (`Walk`, `follow`, `read_ifd`, `entry_budget`, `entry_count`), `src/error.rs`, `fuzz/shared/entry_budget_invariants.rs`, `tests/budget_oracle.rs`, `tests/support/hostile.rs`

## Expected Deliverables

Review cold; output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

Verify independently:
- Re-run every gate (fmt both graphs, `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `cargo test` on stable and 1.90.0 also with `DIAMOND_EXIF_REQUIRE_ORACLE=1`, `-F unsafe_code`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `oracle-record` with a clean `git status`). Run `just fuzz-smoke tiff_ifd` and `just fuzz-smoke audit` 60 s each and record executions and coverage.
- **Read the walk change line by line:** where the count is read and compared (before any allocation?), `Walk.entries` accounting (IFD0 counted; omitted IFD not counted), atomic omission, `halted` set, nothing recorded after the halt, the arithmetic (`usize::MAX` input, `checked`/`saturating`, no panic-capable construct), `MAX_ISSUES` still a true bound. Could a legitimate file ever trip the budget? Argue it from the format (disjoint 12-byte slots) and try to find a counterexample: shared tables, a TIFF where the same IFD bytes are legitimately referenced from two distinct offsets, DNG/TIFF-EP layouts, JPEG Exif blobs; use ExifTool-read local photos if useful (read-only, commit nothing).
- **Judge deviation 1** (truncation is checked BEFORE the budget, contrary to the spec's implied order): is that the better behaviour? Construct a corrupt-big-count-in-small-file case and show what each order would do. Judge deviations 2-5 (extra records/seeds counts moved; the plant input; the N=8 expectation change; hostile() moved; CHANGELOG).
- **Judge the headroom wording:** the spec said at least 10x headroom on sampled photos; two sampled files sit at 1.4x and 2.7x (tiny Exif blobs). Is the budget still safe by construction there, and what should the spec/DEC text say?
- **Reproduce the headline measurement** (N = 511 hostile file, 789,516 B): entries kept, issue shape, parse time, RSS before (check out the previous commit in a scratch worktree if needed) and after.
- **Test the testers.** Write mutants the builder did not try: `>` vs `>=` at the boundary (they tried one), budget computed from `data.len()` of the wrong slice, IFD0 not counted, omitted IFD counted anyway, `halted` not set, budget check after allocation (memory only: how would a test see it?), `limit` field wrong in the issue, the check inside `follow` skipped for `Link::Next`, entries counted per IFD kind only, `entry_count` returning IFD count, budget `/ 11` or `/ 13`, saturating_add replaced by wrapping add. Report which the suite catches and which survive. Redo at least three fuzz plants including one against the real target on each of `tiff_ifd` and `audit`; confirm the invariants are two-directional where they claim to be.
- Check the 121 existing records and the 66+7 seeds byte-identical; the 4 new records and 3+3 new seeds deterministic and each claiming its shape (`xxd`); no dependencies; `scripts/`, `decisions/` (apart from the DEC-001 clarification), irradiance, the audit module and the JPEG layer untouched; CI workflow unchanged.
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes, SPEC-013/014/015/016 work, ExifTool's 10-SubIFD policy, CI wiring beyond reading it

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
