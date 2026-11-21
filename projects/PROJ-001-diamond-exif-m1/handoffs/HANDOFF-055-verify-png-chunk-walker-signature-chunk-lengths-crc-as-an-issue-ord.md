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
  id: HANDOFF-055
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-21
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-028

project:
  id: PROJ-001
  stage: STAGE-008
repo:
  id: diamond-exif

# ── THE HANDBACK ────────────────────────────────────────────────────────────
# Filled in by the EXECUTING AGENT before it reports done. This is a required
# part of completing the handoff, not a courtesy.
#
# `tokens_total` is the one field the cost gate reads. Report the REAL number,
# and say in `basis` how you got it:
#   Claude Code   → your session transcript, measured per docs/cost-tracking.md
#                   "Measuring a cycle": count each message.id once, take its
#                   largest output_tokens, include subagents/ (basis: transcript).
#                   Summing every line over-counts ~2×. `/cost` in the desktop
#                   Code tab reports context size, not spend.
#   API           → the `usage` objects, summed (basis: api-usage)
#   another agent → whatever your harness reports as total tokens
# If your platform genuinely exposes NO token count, set tokens_total: null AND
# write why in `notes` — then set `cost.metering_source: none` in
# .repo-context.yaml so the gate stops asking. Do not invent a number.
handback:
  status: null                     # completed | blocked | rejected
  agent: null                      # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: null                    # verify handoffs: approved | punch-list | rejected
  basis: null                      # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: null               # REAL combined count — what cost-audit reads
  estimated_usd: null              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: null
  branch: null
  pr: null
  completed_at: null               # YYYY-MM-DD
  notes: null                      # if unusual; double-quote it — an unquoted " #" starts a YAML comment
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-055: Verify SPEC-028 PNG chunk walker (DEC-017)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-028` to a cold verifier for the **verify** cycle. You did not write this. Review branch `claude/spec-028-png-walker` (build commit `0989051`, cost commits after it) against `projects/PROJ-001-diamond-exif-m1/specs/SPEC-028-png-chunk-walker-signature-chunk-lengths-crc-as-an-issue-ord.md` and `decisions/DEC-017-png-chunk-walk-and-issue-channel.md`. Review only; edit nothing except your own handback in this file. Do not push.

## BUDGET (hard)

About 80 tool calls or 200k context. Record a real `tokens_total` in the handback (the orchestrator replaces it with the harness figure; no cache reads). Toolchain: `RUSTUP_TOOLCHAIN=1.90.0`, `PATH="$HOME/.cargo/bin:$PATH"`; read `guidance/toolchain-brief.md`. Start with `git checkout claude/spec-028-png-walker` in your worktree (it is a local branch of this repository; do not reset --hard).

## What to look hard at

1. **Panic-freedom and arithmetic.** Every offset computed from a 32-bit attacker-controlled length: any unchecked add or cast, any slice that could go out of range, any loop that might not advance. Read `src/png/mod.rs` line by line.
2. **DEC-017 conformance, both directions.** Every rule in "Policy detail" has code and a test that would fail if the rule were removed. Check the builder's six listed choices where the DEC is silent (issue order within a chunk; the chunk that trips the issue cap is not recorded; `TooManyChunks` fires on a further whole chunk so exactly 65,536 is clean; the `len` in `Truncated`; signature-only gives MissingIhdr then MissingIend at offset 8; `BadIhdr` applies to any IHDR) and say whether each is sound or should become a DEC-017 clarification. Also: `TooManyIssues` Display says "JPEG walk reached..." when PNG uses it: judge severity.
3. **The fuzz model's independence.** `fuzz/shared/png_chunk_invariants.rs` must not call the library CRC or kind table, must assert every iff both ways, and must be able to fail (try a mutation of the library on a scratch copy, e.g. flip a CRC comparison, and confirm the shared invariants or tests go red; revert).
4. **Pins and seeds.** 33 seeds, existing seeds byte-identical, every pin updated, `fuzz/Cargo.lock` not committed with a changed version.
5. **Scope.** Test-only arms added to `src/jpeg` and `src/tiff` tests change no behaviour; nothing else outside the spec's Outputs. Honest README claims. Provenance row. The CHANGELOG entry was dropped because `tests/release_layout.rs::changelog_and_readme_carry_the_release` pins `[Unreleased]` empty: say whether you agree.
6. Run `just lint`, `just test`, `just lint-fuzz`, `just validate`, `just forbid-unsafe`, `just cost-audit` and a 20 s `just fuzz-smoke png_chunks` yourself; do not trust the handback.

## Expected Deliverables

Your verdict, exactly one of APPROVED / PUNCH LIST / REJECTED, with findings ranked High, Medium, Low, each with file:line and a concrete failing input or scenario. Fill the `handback:` front matter (`verdict`, a real `tokens_total`) and the `## Handback` section; set statuses to `completed`.

## Out of Scope

Fixing anything, DEC edits, audit wiring, ExifTool parity (SPEC-031), metadata reading (SPEC-029).

## Return Criteria

Fill the handback and reply with the verdict and findings.

---

## Handback` section below.** An unfilled handback is an
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
