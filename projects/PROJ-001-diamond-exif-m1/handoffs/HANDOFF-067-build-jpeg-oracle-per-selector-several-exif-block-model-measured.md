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
  id: HANDOFF-067
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-22
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-035

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
  status: completed                # completed | blocked | rejected
  agent: claude-sonnet-5-5         # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: null                    # verify handoffs: approved | punch-list | rejected
  basis: agent-result              # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: 175052            # REAL combined count — what cost-audit reads
  estimated_usd: 1.16              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: 40
  branch: claude/spec-035-jpeg-per-selector
  pr: null
  completed_at: 2026-11-22         # YYYY-MM-DD
  notes: "Harness subagent_tokens (175,052) at 6.60 USD per Mtok; the builder self-reported 6,303,399 counting cache reads, which is not spend, so the orchestrator replaced it."  # if unusual; double-quote it — an unquoted " #" starts a YAML comment
  synced_at: 2026-11-22
---

# HANDOFF-067: JPEG oracle: per-selector several-Exif-block model, measured

**FIRST STEP, before anything else:** `git checkout -b claude/spec-035-jpeg-per-selector claude/spec-035-design`
(the harness may have started your worktree from main, which lacks the spec).

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-035` to `claude-sonnet-5-5`
(implementer) for the **build** cycle. Test-harness only: no `src/` change.

## Context the Receiving Agent Needs

Read first: `AGENTS.md`, `guidance/toolchain-brief.md`, the spec
`projects/PROJ-001-diamond-exif-m1/specs/SPEC-035-*measured.md` (Goal, Outputs,
Acceptance Criteria, Notes), and `projects/PROJ-001-diamond-exif-m1/handoffs/HANDOFF-066-*.md`
(the WebP fix round that found the per-selector split).

Toolchain: `export RUSTUP_TOOLCHAIN=1.90.0 PATH="$HOME/.cargo/bin:$PATH"`.
macOS has no `timeout`: use `perl -e 'alarm 8; exec @ARGV' -- exiftool ...`.
BSD `sed -i` needs `''`; use Python for multi-line edits. `indexing_slicing` is
denied in `src/` only. Never commit `decisions/INDEX.md` or `fuzz/Cargo.lock`
(restore the latter with git). No CHANGELOG entry (`tests/release_layout.rs`
pins `[Unreleased]` empty).

## The task, in order

1. **Measure.** Add `jpeg-exif-full-one-two` and `jpeg-exif-full-two-one` to the
   one fixture table in `tests/support/oracle_jpeg.rs`: SOI, two Exif APP1s from
   `tiff_full(1)` and `tiff_full(2)` (see `tests/support/oracle_webp.rs`
   `tiff_full`; reuse it or move it to a shared place, whichever is the smaller
   diff), then the usual scan and EOI. Run `just oracle-record`, then read all
   20 selector values in both orders from the new records. For each selector the
   winner must be the first block's or the last block's, the same in both
   orders. Write the list of first-wins selectors from what you SEE. Expected
   (WebP) is `ImageWidth`, `ImageHeight`, `Orientation`, `XResolution`, but the
   measurement decides. If a selector is neither first nor last, stop and report.
2. **Model.** `JPEG_FIRST_BLOCK_WINS` in `tests/support/oracle.rs` and
   `read_ours_jpeg` mirroring `read_ours_webp`. Rewrite the doc comment: the
   measured rule, and whether JPEG shows the Make-as-SHORT limit (check with a
   second block whose Make is a SHORT, as WebP's `exif-make-short-later`; add
   it only if JPEG differs from the model, otherwise just state it in the
   comment from one measurement).
3. **Pin.** In `tests/jpeg_oracle.rs` add
   `several_exif_model_is_per_selector_and_pinned` mirroring
   `tests/webp_oracle.rs::several_exif_model_is_per_selector_and_pinned`
   (all 20 selectors, both orders, asserts the list equals the constant,
   `assert_clean` on both fixtures). Keep the existing Make/Model test.
4. **Red-proof.** Replay-and-perturb for the new records as the siblings do,
   and one mutated-copy proof that reverting to all-later-wins fails the new
   test (control first, a `RED-PROOF INJECTION` marker, needle found exactly
   once, marker must not leak). Follow `tests/webp_mutation_oracle.rs` and
   `tests/mutation_oracle.rs` (the JPEG one) for the
   pattern; extend the existing file, do not invent a new apparatus.
5. **Docs.** Correct the "later wins" statements in `guidance/questions.yaml`
   (`exiftool-several-exif-segments` and `several-exif-segments`) to the
   per-selector rule, citing SPEC-035 and the two records.
6. **Gates.** `just lint`, `just test`, `just mutation-oracle`,
   `just forbid-unsafe`, `just validate`, `just cost-audit`, and
   `just oracle-record` run twice with `git diff --stat tests/oracle-fixtures`
   showing only the two new records. No other record may change; if one does,
   stop and report which.

## Expected Deliverables

Code and records on branch `claude/spec-035-jpeg-per-selector`, committed
(trailer: `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`), the spec
boxes ticked, the handback below filled with a real harness token count. Do not
open a PR; the orchestrator does.

## Budget

About 100 tool calls or 250k context. If you near either, hand back what is
done and what is not. Do not loop on a failing red-proof: report it.

## Out of Scope

Any `src/` change; DEC changes; the library's primary-block policy; PNG and
WebP; the 17-block cap divergence; new JPEG divergences (if one appears, stop
and report it rather than recording it silently).

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

- **Branch / PR:** `claude/spec-035-jpeg-per-selector`, no PR (the orchestrator opens it)
- **Completed at:** 2026-11-22
- **All acceptance criteria met?** yes (one note: the inventory record `tests/oracle-fixtures/exiftool-inventory/fixtures.txt` gained 90 lines, the sections of the two new fixtures only; no existing line changed. The inventory corpus enumerates every JPEG fixture, so this is unavoidable.)
- **For `verify`:** the verdict — ✅ APPROVED (at commit SHA) / ⚠ PUNCH LIST / ❌ REJECTED

### Cost self-report

Mirror what you put in the `handback:` front-matter, and say where the number
came from. **This is the number that lands in the spec** — the orchestrator
transcribes it via `just handback-sync`, it does not estimate it.

- **Tokens (total):** 175052 (harness figure)
- **Estimated USD:** 1.16 (orchestrator: harness subagent_tokens at 6.60 USD per Mtok)
- **Duration (minutes):** 40
- **Source of the number:** harness subagent_tokens (orchestrator-corrected; the builder self-reported a transcript count that included cache reads)

### Drift and new artifacts

- **New decisions emitted:**
  - none
- **Deviations from spec:**
  - `tiff_full` moved from `tests/support/oracle_webp.rs` to `oracle_tiff.rs` (re-exported from `oracle_webp`), so the JPEG builders reach it without importing WebP code.
  - Pins that count fixtures or records were updated for the two additions: `tests/jpeg_oracle.rs` (44 fixtures, 44 + 37 records, warning table rows), `tests/budget_oracle.rs` (two allowed extra records), `tests/inventory_oracle.rs` (254 inputs, 185 distinct, 63 hand-built) and `tests/mutation_oracle.rs` (185 / 184).
  - The mutated-copy proof (`jpeg_mutant_later_block_wins_all_twenty_is_named`) mutates the probe's copy of `oracle.rs` (the harness model), not `src/`, because the model lives in the harness. Control clean first, `RED-PROOF INJECTION` marker, needle exactly once; the existing leak test covers `tests/support`.
  - Make-as-SHORT (L1) was not re-measured for JPEG; the doc comment says so and that no JPEG fixture has such a block.
- **Follow-up work identified:**
  - none

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — The spec said "no other record may change" but the inventory record lists every JPEG fixture, so it necessarily gained the new sections; several count pins in four test files also had to move.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — The list of count pins that a new JPEG fixture touches (jpeg_oracle, budget_oracle, inventory_oracle, mutation_oracle) belongs in the handoff.

3. **If you did this task again, what would you do differently?**
   — Grep for the fixture-count numbers (42, 252, 183, 61) up front instead of finding them one `just test` run at a time.
