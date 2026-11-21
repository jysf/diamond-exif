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
  id: HANDOFF-053
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-21
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-033

project:
  id: PROJ-001
  stage: STAGE-009
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

# HANDOFF-053: DEC-008 amendment: a `Rights` category for Copyright, kept by `AllPii`

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-033` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

You share no memory with the architect. Everything you need is in the files below, read in this order.

### Primary

- **Spec:** `./projects/PROJ-001-diamond-exif-m1/specs/SPEC-033-dec-008-amendment-a-rights-category-for-copyright-kept-by-al.md`. It holds the code sites with line numbers, the implementation choices, the acceptance
  criteria and the failing tests. It is the contract.
- **The three DEC amendments, all dated 2026-11-21**, at the bottom of each file. They are normative, and the
  spec implements them:
  - `./decisions/DEC-008-audit-categories-and-severity.md`
  - `./decisions/DEC-009-strip-policy-presets-and-the-preview.md`
  - `./decisions/DEC-013-writer-foundation-fail-closed-removal-and-patch-plan.md`
- **Stage:** `./projects/PROJ-001-diamond-exif-m1/stages/STAGE-009-keep-the-copyright-notice-under-allpii.md`
- **Project brief:** `./projects/PROJ-001-diamond-exif-m1/brief.md`
- **Toolchain brief:** `./guidance/toolchain-brief.md`. Read it for the pinned toolchain, the
  `RUSTUP_TOOLCHAIN=1.90.0` lint quirk and the nightly fuzz invocation.

### The change in one paragraph

Exif Copyright (0x8298) moves from category `Authorship` to a new category `Rights`. `Rights` is Medium severity
and a tag category, not a block. It is removed by the `All` and `Everything` presets only, so `AllPii` (the
default privacy preset) now keeps the copyright notice while still deleting Artist. The writer's "every Exif
category" set (EXIF9) gains `Rights` and is renamed EXIF10, so `All` still removes Exif segments whole. Nothing
else moves.

### Decisions that apply

- `DEC-008`: the category table. Copyright moves to `Rights`, and severity is unchanged.
- `DEC-009`: preset grid and may-hold table. `Rights` is removed by `All` and `Everything` only; may-hold rows
  for `Thumbnail`, `Xmp`, `Iptc`, `Jumbf` (and `EmbeddedImage`/`OtherSegment` through their rule).
- `DEC-010`: reports use `Category::name`, so the new name `rights` flows to text, JSON and CSV.
- `DEC-012`: precedent for appending categories after the existing ones.
- `DEC-013`: EXIF9 becomes EXIF10. A kept Copyright that shares bytes with a deleted Artist is `SharedBytes`.

### Constraints that apply

Full text in `./guidance/constraints.yaml`:

- `test-before-implementation`: the spec's Failing Tests are written and seen red first.
- `panic-free-library`, `forbid-unsafe-code`: the five-lint deny set; no `<<` on integers (hence the literal bit
  table).
- `oracle-must-be-shown-red`: the new strip-oracle comparator red-proof.
- `fuzz-target-per-surface`: no new target, but the four category-modelling smokes must stay clean.
- `exiftool-never-linked`: re-record by running it, never reading or linking it.
- `no-new-top-level-deps-without-decision`: no new dependency.
- `one-spec-per-pr`.

### Prior related work

- `SPEC-023` / `HANDOFF-048` / PR #36: the last DEC-008 category amendment. It is the closest precedent.
  Its reflection names the trap this spec guards against: a model in `fuzz/shared/rewrite_invariants.rs` was
  not updated and only CI's `rewrite_jpeg` smoke caught it.
- `SPEC-014`: presets and preview. `SPEC-021`: entry-level deletion and `SharedBytes`.

## Expected Deliverables

- Code, tests, seeds and re-recorded ExifTool records per SPEC-033's Outputs, with every acceptance criterion met.
- The spec's Failing Tests written first and seen red. Say in the handback which ones you saw red, and how.
- Gates, all run and their results quoted in the handback:
  - `just test`
  - `RUSTUP_TOOLCHAIN=1.90.0 just lint`
  - `just lint-fuzz`
  - `just deny`
  - `just mutation-oracle`
  - `just oracle-record` followed by `git diff --stat tests/oracle-fixtures`. Only the `all_pii`, `authorship`
    and new `rights` sections may change.
  - `just fuzz-smoke audit`, `just fuzz-smoke audit_render`, `just fuzz-smoke rewrite_jpeg` and
    `just fuzz-smoke rewrite_tiff`, 60 s each.
- Branch `feat/spec-033-rights-category` from `origin/main`, and one PR against `main`. The PR description
  names PROJ-001, STAGE-009, SPEC-033, HANDOFF-053, DEC-008/009/010/012/013 and the constraints above.
  Its body ends with the line `🤖 Generated with [Claude Code](https://claude.com/claude-code)`.
- Commits are Conventional (`feat(audit): …`), cite SPEC-033 in the body, and end with
  `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>` (or the model that actually ran).
- The DEC amendments are already written. Do not edit them, except to fix a factual error you can show. Report
  any such fix in the handback.

## Out of Scope

- A CLI option to choose categories. Moving any tag other than 0x8298. Changing a severity.
- Widening `StripSpec` beyond `u16`. Editing inside XMP or IPTC. STAGE-007's SPEC-025 to SPEC-027.
- If you find that a preset disposition other than Copyright's changes on a committed seed or fixture, stop and
  report it. It would mean the DEC-009 may-hold analysis is wrong, and that is the architect's call.

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
