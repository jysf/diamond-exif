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
  id: HANDOFF-043
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-10
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-021

project:
  id: PROJ-001
  stage: STAGE-004
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

# HANDOFF-043: Re-verify JPEG in-place Exif entry deletion with scrub (SPEC-021, round 2)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-021` to `claude-opus-5-5` (verifier, a fresh cold agent with no memory of round 1) for a second **verify** cycle after a punch list.

## Context the Receiving Agent Needs

- Everything in `HANDOFF-042-verify-*.md` still applies (spec, DEC-013 with its 2026-11-10 amendment, gates, privacy rule for `_incoming0`: read in place, copy nothing, write no derived file, pipes only, counts and 8-hex hashes only, never values or paths; host traps; do not fix code, push, open a PR or advance cycles). Read it, including the round-1 verdict in its Handback.
- **Round 1 (PUNCH LIST at 7697162)** found: H1 (a deleted entry's out-of-line value running past the blob end was left unscrubbed, `Ok` with the canary in the output); H2 (thumbnail removal scrubbed only the first 0x0201/0x0202 range, leaving a second pair or strip data); M1 (the per-block refusal loop was tested only on the first Exif block); Lows L1 to L5. The fixes are `git diff 7697162..HEAD` (commit `1cf8bd1` and its WIP commits; the follow-up for the stray-bytes harness crash is earlier, `56bd4a6`, `9315047`). Review the fixes as a new reader, not as a diff checklist.
- **Do not anchor on round 1.** H1 and H2 were the same class (the planner assumed a range or locator it had not proved). Hunt for siblings by reading `src/write/exif.rs` and `src/tiff/mod.rs` accessors for every place that takes `None`, `first`, `get(..)`, a count, a pointer or a length and acts as if the answer were complete. Specifically try: entries whose type size times count overflows or wraps; counts above the blob; value offsets in the header or inside another IFD's table; two IFDs sharing a value; a thumbnail range inside a kept IFD or a MakerNote; 0x0201 with count != 1 or a non-LONG type; 0x0117 without 0x0111; GPS and thumbnail entries of unknown field types; a SubIFD (0x014A) pointing at the GPS IFD or IFD1; Interop pointing into a removed IFD; IFD1 reached from a SubIFD chain; truncated entries at the blob end; both byte orders for each; and Exif segments that differ (two blocks, one clean, one not; the second block holding the only GPS).
- **Judge the L4 widening.** The harness excuse `walks_as_other_segments` now covers `VerifyFailed` checks `mpf-fields`, `walk` and `entries` (the spec asked for the first two; the builder added `entries` after a 60 s smoke failed on a 4.4 KB input, now seed `crash-entries-joined-stray`, claiming the same stray-bytes class by structure, not verified at byte level). Establish at byte level why the library returns `VerifyFailed{Entries}` there: is it truly the joined-stray class, or a planner bug that `Entries` caught? If a planner bug, that is at least High.
- Orchestrator's own gate run at `1cf8bd1` (all steps exit 0; 2577 tests passed, 0 failed over three runs; five 60 s smokes clean) is evidence, not a substitute: re-run everything in HANDOFF-042's gate list, plus `rewrite_jpeg` for 5 minutes.
- Re-run mutants V04, V05, V08, V11 (round 1 survivors) and the builder's new ones from the HANDOFF-041 Handback addendum; write at least six mutants of your own aimed at the H1/H2 fixes (e.g. the straddle test using `>` instead of `>=`; H2 counting only 0x0201; H2 skipping 0x0117; the mirror in the harness removed; the later-block loop checking only the last block) and name the test or guard that catches each.
- Re-run the local corpus counts (expected: `Gps` 13 Ok, `GpsAndDevice` 13 Ok, `AllPii` 12 Ok + 1 `Uneditable(ChainAfterThumbnail)` on hash `a3b66f84`, `All` 13 Ok, `Everything` 13 Ok; 21 files, 13 JPEGs, 4 with Multi-Picture) and pipe a few outputs to `exiftool -` as in round 1.

## Expected Deliverables

Output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA, with findings by severity and concrete reproducers, mutant results, local-corpus counts, and a verdict on each open builder question in `guidance/questions.yaml`. Fill the HANDOFF-043 handback front matter (`tokens_total: null` with a note; the orchestrator transcribes the metered figure).

## Out of Scope

- Implementing fixes; TIFF/DNG (SPEC-022); `strip-keeps-orientation`; changing DEC-013.

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
