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
  id: HANDOFF-049
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-19
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-023

project:
  id: PROJ-001
  stage: STAGE-005
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

# HANDOFF-049: Verify SPEC-023 (DEC-008 amendment: DNG identity tags, container-aware TIFF page rule)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-023` to a fresh verifier (`claude-opus-5-5`) for the **verify** cycle. You did not write this code. Review it cold, adversarially, and return exactly ONE verdict: APPROVED / PUNCH LIST / REJECTED, with every finding graded High, Medium or Low.

## BUDGET (hard)

About **80 tool calls or 200k context**. Read large files with offset/limit. Work in this worktree (`/Users/jyashinsky/PSeven/experiments/crustimg_redo_plus/diamond-exif/.claude/worktrees/spec-022-tiff-dng-strips-26c250`, branch `claude/dec-008-dng-and-pages`, build commit `5582129`). Do NOT change code, push, or open a PR; write your findings in the handback only. Use `~/.cargo/bin/cargo +1.90.0` and `RUSTUP_TOOLCHAIN=1.90.0` for lint (see `guidance/toolchain-brief.md`). Record a real token count and say how you got it.

## What to read

- Spec: `projects/PROJ-001-diamond-exif-m1/specs/SPEC-023-dec-008-amendment-dng-identity-tag-categories-and-a-containe.md` (Goal, Decisions made here, Acceptance Criteria, Failing Tests, Non-Goals).
- Build handback: HANDOFF-048 (same directory). Diff: `git diff d534384..5582129` (the framing commit to the build commit); start with `src/audit/category.rs`, `src/audit/mod.rs`, `src/write/exif.rs`, `src/write/verify.rs`, `fuzz/shared/audit_invariants.rs`, then the tests.
- DEC-008, DEC-013 (amendments C and D, D.3), DEC-009, DEC-012; `guidance/constraints.yaml`.

## What the builder reports, and where to be suspicious

1. **Unrequested changes.** (a) `src/write/verify.rs`: the `entries` self-check now removes IFD1 only where the audit calls it a thumbnail. Check it cannot let a real thumbnail removal slip through unverified, or let a page's entries be removed unchecked. (b) `fuzz/shared/audit_invariants.rs`: the INDEPENDENT fuzz model was edited to mirror the production rule. A differential model that is changed to match the code it checks is weaker; judge whether it still checks something independent, and whether a deliberate production bug would still be caught.
2. **Changed tests.** The multi-page refusal assertion became an Ok assertion; `expect_refusal` in `every_shape_under_every_policy_keeps_the_image_data_and_the_length` is now always false; two `audit-ifd1-strips` plants in `tests/fuzz_layout.rs` now use a generated JPEG-wrapped copy of the seed. Confirm these changes follow the spec and do not just loosen the tests; confirm the JPEG rule (IFD1 with strips and no 0x0201 is still a `Thumbnail` inside a JPEG Exif block) is really exercised by a test that fails if the container-awareness is dropped.
3. **Oracle red-proofs.** The builder showed the unit, audit and strip tests red first, but the oracle tests only by planting faults in the comparator, not by reverting the `by_id` rows or the old IFD1 rule. Check the replay records are real ExifTool output of the new fixtures (re-record with `just oracle-record` into a scratch copy or compare, if ExifTool is installed) and that reverting the production change turns an oracle or strip test red. Do the mutation yourself on a scratch copy of the tree or by a temporary edit you revert; leave the tree clean.
4. **Behaviour to probe.** (a) A kept page IFD1: its own Make/DateTime/Artist entries are deleted by category, its strips and tiles identical; chain pages after IFD1; a page IFD1 that holds a SubIFDs, Exif or GPS pointer. (b) DNGPrivateData 0xC634 and OriginalRawFileName 0xC68B: out-of-line bytes zeroed, nothing shared with kept data; a 0xC634 whose value overlaps a strip or raw data must refuse, not zero image bytes. (c) An IFD1 with both the JPEG pair and strips is still refused; a JPEG-pair thumbnail IFD1 in a bare TIFF is still removed whole. (d) `AllPii` keeps Software, so the page's Software survives `AllPii`; confirm that matches DEC-009 and the docs say nothing different. (e) `GpsAndDevice` leaves 0xC634, as DEC-008 now says.
5. **Independent differential.** Write a throwaway script or test (do not commit it) that runs `strip` under all five presets over every seed in `fuzz/seeds/rewrite_tiff/` and the shapes in `tests/support/tiff_shapes.rs`, and asserts on every Ok result: same length; every strip, tile and raw-data range of the INPUT walk byte-identical; no finding of a removed category in the output audit; second strip is a no-op. Report counts.
6. **Gates.** Re-run `just lint`, `just test`, `just forbid-unsafe`, `just deny`, `just validate`, and `just fuzz-smoke rewrite_tiff` and report results. Check docs: DEC-008 and DEC-013 D.3 amendments are accurate and short; README no longer promises what is not delivered and no longer lists what is now fixed; `guidance/questions.yaml` entries are consistent.

## Out of scope

PNG, WebP, HEIC, BigTIFF, `set` operations, MakerNote parsing, changing preset membership, the `audit-blob-carrier-parsing` question. Findings there are at most Low notes.

## Return Criteria

Fill the `handback:` front matter (`verdict`, a real `tokens_total`, `basis`, `branch`, `completed_at`) and the `## Handback` section with the verdict, every finding (grade, file:line, failing input or test, suggested fix) and the differential counts. Set `handoff.status` and `handback.status` to `completed`. Make no commits other than the handback file edit; do not push.

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
