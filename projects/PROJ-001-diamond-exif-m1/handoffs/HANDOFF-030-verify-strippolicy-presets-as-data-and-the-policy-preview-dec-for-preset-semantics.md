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
  id: HANDOFF-030
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-06
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-014

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

# HANDOFF-030: Verify StripPolicy presets and the policy preview

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-014` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-014-strippolicy-presets-as-data-and-the-policy-preview-dec-for-preset-semantics.md` (API, rules, expectation matrices, named tests and the 22 mutants are normative)
- **DEC-009:** `decisions/DEC-009-strip-policy-presets-and-the-preview.md` (normative for every preset cell and the may-hold table)
- **Build handoff and Handback:** `HANDOFF-029-build-...md` (claims to verify, not to trust; read the deviations, especially the seed counts, mutant 13's variant, and the added full-house harness inputs)
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe)
- Code under review: `git diff 876ee82..HEAD` (build commits `779b906`, `d1be0b1`, `c2999be`). Main: `src/policy.rs`, `src/audit/preview.rs`, `src/audit/category.rs` (`is_block`), `tests/policy_preview.rs`, `tests/fuzz_layout.rs`, `fuzz/shared/audit_invariants.rs`.

## Expected Deliverables

Review cold; output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

Verify independently:
- Re-run every gate (fmt both graphs and the harness file, `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `cargo test` on stable and 1.90.0 also with `DIAMOND_EXIF_REQUIRE_ORACLE=1`, `-F unsafe_code`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `oracle-record` with an empty `git status --porcelain`). Run `just fuzz-smoke audit` 60 s; record executions and coverage. Confirm `git diff 876ee82..HEAD -- src` touches only `src/policy.rs`, `src/audit/preview.rs`, `src/audit/category.rs` (additions only), `src/audit/mod.rs` and `src/lib.rs`, that `audit()` is unchanged, and that `decisions/`, `scripts/` are untouched.
- **Check the code against DEC-009 yourself**: read DEC-009 and rebuild the preset grid, the kept-block grid and the may-hold table from it by hand, then compare with `src/policy.rs` and `src/audit/preview.rs` cell by cell, independently of the builder's tests. Then check the tests' own grids against the DEC the same way (a test grid copied from the code would make the suite vacuous). Check rules 1 to 10 of the spec one by one, in particular rule 4 (truncated residual severity and the `all()` exception) and rule 5 (`reliable`).
- **Judge DEC-009 itself, not only the code.** Is any preset cell surprising or unsafe (for example, can `GpsAndDevice` or `AllPii` produce a preview that reads as safe while leaving a leak)? Does `KeptUnread` ever fail to fire where a reader would need it, or fire where it is just noise? Does the policy preview ever turn an unsupported, unreadable or truncated audit into a clean-looking one? Report as findings, not as edits.
- **Test the testers.** Re-run the builder's mutants (the report claims 22 red) at least for 1, 3, 9, 12, 15, 16, 17 and 19. Note the builder says mutants 15 and 16 are caught by only one unit test and not by the harness: judge whether that is acceptable. Then write mutants the builder did not try, for example: `kept_unread` check using `is_subset_of` instead of `intersects`; `include` that clears other bits; `categories()` yielding in reverse; `spec()` for `Custom` returning `none()`; residual severity computed over all findings; `reliable` ignoring truncation; the harness `Custom` spec derivation always empty; one preview guard's iff checked in one direction only; a `StripSpec::all()` that misses the last category; `is_block` true for `Software`; `may_hold(Thumbnail)` missing `Software`. Report which the suite catches (name the test or guard) and which survive.
- Redo one preview-guard plant against the real `audit` libFuzzer target (not only the stable-toolchain layout test), and check the step-counter linearity test covers the new guards.
- Check the added harness inputs (hand-built full-house TIFF and JPEG in `tests/fuzz_layout.rs`) really carry every category (`the_full_house_inputs_carry_every_category`) and that the seed count and seed hash lists are unchanged.
- Check no dependencies were added and no file under `decisions/`, `scripts/` or irradiance is touched.
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes; SPEC-015/016 work; the writer; any ExifTool comparison (not run in this spec)

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
