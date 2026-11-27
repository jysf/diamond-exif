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
  id: HANDOFF-093
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-27
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-025

project:
  id: PROJ-001
  stage: STAGE-007
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

# HANDOFF-093: SPEC-025 verify: the Exif block builder in the source's byte order

**FIRST STEP:** `git fetch origin && git checkout -b claude/spec-025-verify origin/feat/spec-025-exif-block-build`. Its
tip is the build (PR #71) plus the commit that adds this handoff. Work read-only on the library and the tests. Commit
only to this handoff file, on `claude/spec-025-verify`, and push that branch.

## Delegation Summary

`claude-opus-5-5` (architect) hands SPEC-025 to an independent `claude-opus-5-5` verifier for the **verify** cycle.
Two `claude-sonnet-5-5` implementers built it from HANDOFF-092: Part A covered AC1 to AC8 and AC10, Part B covered
AC9, AC11 and AC12. The architect then made one fix (`b0b5130`: the writer sweep runs without `-m`). You did not
write this code, these tests or these decisions. Review them cold.

**Read first:**
- `AGENTS.md`: §12, and §15 "During verify".
- `guidance/toolchain-brief.md` and `guidance/constraints.yaml`.
- The SPEC-025 spec: Context, *Design measurements* (including the "Corrected at build" note), the API in Outputs,
  AC1 to AC12, Failing Tests, and Notes.
- DEC-014 in full, especially the **amendment of 2026-11-27**, items 1 to 7, which is the rule book.
- DEC-015 with its **clarification of 2026-11-27** (the scratch image, and `base < 8`).
- DEC-005 (walk order), DEC-001, DEC-013 part 7, and DEC-023's rule on `-m`.
- `HANDOFF-092-*.md`: the build brief and both handback entries. They list the deviations: `base < 8` is a
  `BadBase`; omission precedence is UnknownType, then MakerNote, then OffsetCarrying; a dangling 0xA005 is left out;
  pinned per-shape differences in the live test; the looping source is never given to ExifTool; the fuzz harness's
  budget retry.

**The change:** `git diff origin/docs/spec-025-design origin/feat/spec-025-exif-block-build`. The library is
`src/write/block.rs` plus small visibility changes in `src/write/exif.rs` and `src/tiff/types.rs`. The rest is tests,
the fuzz target, seeds and docs.

**Toolchain:**
- `export RUSTUP_TOOLCHAIN=1.90.0`; fuzz needs nightly.
- macOS has no `timeout`; use `perl -e 'alarm 8; exec @ARGV' --`.
- ExifTool 13.55 is installed. Run it, never read its source. **Never pass `-m`** (DEC-023). Always give it a fresh
  output path.
- Restore `fuzz/Cargo.lock` after any fuzz or lint-fuzz run. Never commit `decisions/INDEX.md`. Never `git stash`
  bare. Leave no background processes. Keep scratch scripts outside the repo.

## Decisions that apply

- **`DEC-014`** and its amendment: what the block holds, what it leaves out and why, which walk issues refuse,
  selection by parent, how entries are copied, and the layout.
- **`DEC-015`** and its clarification: the size query, then a write at an even base; the self-check runs on the exact
  bytes in a scratch image; the budget covers that image.
- **`DEC-005`:** SubIFDs are walked before IFD0's Exif pointer.
- **`DEC-013` part 7:** read your own output back.
- **`DEC-023`:** no `-m` on ExifTool writes.

## Constraints that apply

`panic-free-library`, `forbid-unsafe-code`, `no-io-in-library`, `no-new-top-level-deps-without-decision`,
`fuzz-target-per-surface`, `test-before-implementation` (Part A's red commit is `3b359a3`), `exiftool-never-linked`.

## What to establish (evidence for each, not opinion)

1. **The bytes are right, by your own derivation.** Write your own small script (not the repo's
   `tests/support/exif_block_model.rs`). Build at least six sources by hand, in both byte orders, and predict each
   block byte for byte from DEC-014 amendment item 7: positions, padding, next-words, the 0xA005 value at
   `base + position`, and zeroed inline bytes. Compare against `write(base)` at two bases, through a tiny example
   binary or a scratch test outside the repo. Include a source where a SubIFD's Exif IFD is walked before IFD0's
   (selection by parent).
2. **The omission table is right and complete.**
   - Re-run the offset sweep yourself without `-m`, with your own generator. Confirm the 22 `No size tag`
     refusals, the four `[minor]` value refusals, 0x014A, and the reader-side result.
   - Confirm every tag ExifTool treats as an offset is left out, and that the builder leaves out nothing beyond
     DEC-014's list.
   - Try the swept tags with other types (SHORT, a LONG array, IFD type 13) and in the GPS IFD. Look for an offset
     tag the sweep's LONG-count-1 shape could miss.
   - Judge the builder's added rule (a 0xA005 with no Interop IFD is left out) and its precedence order.
3. **Refusals are exactly DEC-014 amendment item 4.**
   - Build sources that should refuse but don't, and sources that refuse but shouldn't. For example: an issue
     following 0xA005 that is not from the chosen Exif IFD; a halt after the Exif IFD was recorded; a duplicate tag
     in IFD0 other than 0x8769 or 0x8825.
   - Read the code for any allocation sized from the source before the budget check.
   - Judge `base < 8` as `BadBase` against DEC-015 item 2.
4. **The self-check cannot pass vacuously.**
   - The checker compares the output with the plan. What catches a plan that wrongly leaves out a real entry and
     reports it? Is that covered anywhere besides the omission-table tests?
   - Flip bytes in the scratch image: positions, counts, types, values, the Interop pointer. Confirm each gives
     `VerifyFailed` and that the fault hook exists only under `#[cfg(test)]`.
5. **The tests are independent and not vacuous.**
   - Confirm the model, the placer and `fuzz/shared/exif_block_invariants.rs` import nothing from
     `src/write/block.rs`.
   - Mutate the library in a scratch copy (drop the padding, keep an offset tag, swap the IFD order, select by kind
     instead of parent, skip the budget check) and confirm the suite goes red for each.
   - Check that `omission_table_mirrors_agree` really compares every listed mirror.
6. **The live tests (AC9) are honest.**
   - Check each pinned per-shape difference in `tests/exif_block_live.rs` against its mechanism. Is any of them a
     real parity break hidden by a pin?
   - Assess the `unknown-types` case: ExifTool stops reading an IFD at the first unknown-type entry, so the placed
     block shows entries ExifTool never printed for the source. Is that acceptable, or should it be a DEC-014 note?
   - Review the architect's `-m` fix (`b0b5130`).
7. **Panic-freedom and the fuzz target.**
   - Read `src/write/block.rs` for unchecked arithmetic and indexing.
   - Run 60 s smokes of `exif_block`, `tiff_ifd` and `rewrite_tiff` (the `exif.rs` lists only changed visibility).
   - Confirm the seed count pin and the CI matrix entry.
8. **Docs:** the README section and the CHANGELOG entry against the evidence. DEC-014 and DEC-015 against the code.
   Run `just decisions-audit --changed`.
9. **Gates on 1.90.0:**
   - `just lint`, `just lint-no-allow`, `just lint-fuzz`, `just forbid-unsafe`;
   - `just deny`, `just deny-fuzz`;
   - `DIAMOND_EXIF_REQUIRE_ORACLE=1 just test`, with no skips;
   - `just mutation-oracle`;
   - `just validate`, `just cost-audit`;
   - `cargo fmt --check` for both crates.

Output exactly ONE verdict:
- ✅ APPROVED (at commit SHA);
- ⚠ PUNCH LIST: numbered items, each with severity H/M/L, evidence and a suggested fix;
- ❌ REJECTED.

Write it into the `## Handback` section and into `handback.verdict` (approved | punch-list | rejected). Leave
`tokens_total` null; the orchestrator records the harness figure.

## Budget

About 120 tool calls or 300k context. If you get near either, report what you established and what you did not,
then stop. Do not wait on background work at the end.

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
