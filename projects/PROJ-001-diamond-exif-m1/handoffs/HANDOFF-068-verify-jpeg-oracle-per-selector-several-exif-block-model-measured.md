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
  id: HANDOFF-068
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
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
  agent: claude-opus-5-5           # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: punch-list              # verify handoffs: approved | punch-list | rejected
  basis: agent-result              # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: 178261            # REAL combined count — what cost-audit reads
  estimated_usd: 1.18              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: 45
  branch: claude/spec-035-verify
  pr: null
  completed_at: 2026-11-22         # YYYY-MM-DD
  notes: "Harness subagent_tokens (178,261) at 6.60 USD per Mtok; the verifier self-reported 5,723,009 counting cache reads, replaced by the orchestrator."
  synced_at: 2026-11-22
---

# HANDOFF-068: JPEG oracle: per-selector several-Exif-block model, measured

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-035` to `claude-opus-5-5` (verifier)
for the **verify** cycle: a cold, adversarial review of the build on
`claude/spec-035-jpeg-per-selector` (HANDOFF-067).

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-035-jpeg-oracle-per-selector-several-exif-block-model-measured.md`
- **Build handoff:** `projects/PROJ-001-diamond-exif-m1/handoffs/HANDOFF-067-build-jpeg-oracle-per-selector-several-exif-block-model-measured.md`
- **Toolchain brief:** `guidance/toolchain-brief.md`
- **Decisions:** DEC-006 (the library keeps "first block is primary"; the
  per-selector rule is harness-only).

## Expected Deliverables (verify contract)

Test-harness change only (no `src/` diff). Check: acceptance criteria and that
the tests exercise them; the first-wins list MEASURED, re-measured by the
verifier on the two fixtures and on a variant grid (three blocks, first-wins tag
in one block only, non-TIFF block between good ones, Exif across the scan,
MPO-style, Make as SHORT); every other comparison unchanged; the red-proofs
real; `guidance/questions.yaml` edits accurate; gates green and
`just oracle-record` idempotent. Verdict: approved | punch-list | rejected.

## Out of Scope

Fixing anything beyond this handoff; any `src/` change; pushing or opening a PR.

## Return Criteria — how to hand back

Fill the `handback:` front-matter and the `## Handback` section; return the
verdict.

---

## Handback

### Execution notes

- **Branch / PR:** `claude/spec-035-verify` (from `claude/spec-035-jpeg-per-selector` at `77d619b`), no PR
- **Completed at:** 2026-11-22
- **All acceptance criteria met?** yes: all six boxes hold, re-run and re-measured. The punch list is
  documentation only: stale "later wins" comments, and one Make-as-SHORT measurement the
  handoff asked for and the build skipped. Nothing in code or records needs to change.
- **For `verify`:** ⚠ PUNCH LIST (at `77d619b`). Three doc-only items, below. The model, the
  records and the tests are correct.

### What was checked, with evidence

1. **No `src/` diff.** `git diff main --stat -- src` is empty. Changed: tests/support (oracle,
   oracle_jpeg, oracle_tiff, oracle_webp), tests/{jpeg,mutation,budget,inventory}_oracle.rs,
   two new records, `exiftool-inventory/fixtures.txt` (+90/-0, only the two new `##` sections),
   questions.yaml, the spec/handoff/stage docs.
2. **The measurement is real (re-measured, ExifTool 13.55, `-T -n -s3`, 20 selectors).** On the
   two committed fixtures, regenerated and re-run, the first block wins ImageWidth,
   ImageHeight, Orientation and XResolution, and the last block wins the other 16, in both
   orders. That matches `JPEG_FIRST_BLOCK_WINS` and the records. Variant grid (built by a
   throwaway test file, now deleted, never committed; ours = `read_ours_jpeg`):

   | # | Shape | ExifTool (first-wins W,H,Or,XRes; Make) | ours vs ExifTool, 20 selectors |
   |---|---|---|---|
   | g01 | full(1), full(2) | 100,50,1,72; MakeTwo | agree |
   | g02 | full(2), full(1) | 200,100,8,144; MakeOne | agree |
   | g03 | three: 1,2,3 | first (1); last (Three) | agree |
   | g04 | three: 3,1,2 | first (3); last (Two) | agree |
   | g05 | three: 2,3,1 | first (2); last (One) | agree (the middle block never wins) |
   | g06 | first-wins tags + Make only in the FIRST block | block 1; MakeOne | agree |
   | g07 | first-wins tags + Make only in the LAST block | block 2; MakeTwo | agree |
   | g08 | three, first lacks the first-wins tags | block 2 (first that HAS it); MakeThree | agree |
   | g09 | three, last lacks the 16 last-wins tags | block 1; MakeTwo (last that has it) | agree |
   | g10 | first-wins tags only in the middle of three | block 2; MakeThree | agree |
   | g11 | good, non-TIFF, good | block 1; MakeTwo | agree |
   | g12 | non-TIFF, good, good | first good block; MakeTwo | agree |
   | g13 | full(1) before SOS, full(2) after the scan | block 1 for all 20 | 16 differ: ours reads the after-scan block (known class `exiftool-ignores-exif-after-scan`, not this model) |
   | g14 | 1, 2 before SOS, 3 after | fw block 1; last-wins block 2 | 16 differ, same known after-scan class |
   | g15 | MPO-style (MPF APP2, 2nd JPEG with full(2) appended) | primary only (block 1) | agree |
   | g16 | full(1), full(2) with Make as SHORT | Make = 14 (the later SHORT) | Make differs: ours MakeOne (L1 limit) |
   | g17 | Make SHORT in the first, full(2) | MakeTwo | agree |
   | g18 | Orientation as ASCII "x" in the first, full(2) | Orientation = x | Orientation differs: ours 8 (L1, first-wins side) |
   | g19 | full(1), Orientation ASCII in the later block | Orientation = 1 | agree |

   g13 and g14 are the recorded after-scan divergence class: ExifTool stops at SOS, and the
   library and the model read past it. g16 and g18 are the L1 type limit: `read_one`
   returns None for a value of the wrong type, so the model takes another block's value.
   Neither is a per-selector error, and no committed fixture has these shapes.
3. **Other comparisons unchanged.** `just oracle-record` run twice: `git status` stayed clean
   both times. No WebP record changed. `tiff_full` was moved, not changed: the old and new
   bodies are identical once the `t::` prefix and whitespace are stripped, apart from one
   trailing comma left by rustfmt. `webp_oracle` passes.
4. **The red-proofs are real.** m3b (`jpeg_mutant_later_block_wins_all_twenty_is_named`) runs
   the control first (`jpeg_control_then_mutant`), then a single `inject_replace` that must
   match exactly once and carries the `RED-PROOF INJECTION` marker. The existing test
   `injection_marker_does_not_leak_into_src_or_tests_support` covers marker leaks. With
   `--nocapture`, the mutant names exactly the four first-wins selectors on both fixtures. The
   replay-and-perturb test `oracle_names_the_one_field_perturbed_in_a_jpeg_record` loops over
   the whole fixture table, so it covers the new fixtures and asserts that the one unexplained
   mismatch names the perturbed selector. Independent mutants on the real `oracle.rs`, each
   reverted afterwards:
   (a) the later block wins all 20: `several_exif_model_is_per_selector_and_pinned` fails at
   "ours against ExifTool", along with four siblings;
   (b) XResolution dropped from `JPEG_FIRST_BLOCK_WINS`: the same five tests fail;
   (c) first-wins reads the first block even when that block lacks the tag or fails to parse:
   the replay and live seed tests fail on `jpeg-seed-two-exif`. So "the first block that has
   the tag" is pinned too, by a seed rather than by the new fixtures.
5. **Gates (RUSTUP_TOOLCHAIN=1.90.0, DIAMOND_EXIF_REQUIRE_ORACLE=1):** `just lint` 0;
   `just test` 0 (31 binaries, 1204 passed, 0 failed); `just mutation-oracle` 0 (59 + 39 + 40);
   `just validate` 0; `just cost-audit` 0; `just forbid-unsafe` 0;
   `just oracle-record` twice with no diff. `just decisions-audit --changed` is advisory
   (DEC-001/003/007), with nothing touched in `src/`.

### Findings

**High:** none.

**Medium**

- **M1: the Make-as-SHORT measurement was skipped, and the doc comment should now give the
  result.** HANDOFF-067 step 2 and the spec's Notes asked for one measurement. The builder
  wrote "was not re-measured" (`tests/support/oracle.rs`, the `read_ours_jpeg` doc). Measured
  here: JPEG has the same L1 limit as WebP. On g16, ExifTool prints `14` (the later block's
  SHORT) and the model prints `MakeOne`. The comment also misses the mirror case. For a
  first-wins selector whose FIRST block has an unreadable type (g18, Orientation as ASCII),
  ExifTool prints the first block's `x` and the model prints the later block's `8`. Fix: in
  the comment, say "measured (SPEC-035 verify)" and describe both directions. No new fixture
  is needed, since the spec allows a statement from one measurement.
- **M2: stale "later wins" statements contradict the new model.** `tests/support/oracle.rs`
  (`read_ours_webp` doc, ~line 1301) says "the JPEG model ([`read_ours_jpeg`]: later wins,
  unmeasured per selector)". This is now false. The `tests/jpeg_oracle.rs` module doc (line
  13) says "union of tags, the later block wins". The spec's Output says the "later block
  wins" claim is dropped. It was dropped from `read_ours_jpeg`'s own doc only. Fix: both
  comments. `tests/png_oracle.rs:816` ("the JPEG rule ... the later wins") could take the same
  edit.

**Low**

- **L1:** `guidance/questions.yaml`. Both edits are accurate. "The FIRST block wins" could
  read as "the first block, period", while g08 and g10 show it is the first block that HAS the
  tag. Two smaller points: "held for Make and Model only, the two tags SPEC-011's fixtures
  varied" leaves out the GPS-ref and seventeen-block fixtures; and two edited lines run past
  the file's wrap width.
- **L2:** the HANDOFF-067 Handback prose disagrees with its front-matter. The prose says
  Estimated USD null and the source is the "session transcript", while the front-matter says
  1.16 and basis agent-result. The orchestrator replaced the number, but the prose was not
  updated.
- **L3:** the name `multi_exif_is_compared_against_the_union_with_the_later_block_winning`
  is now only half true. It is kept as the spec asks, but a doc line pointing at the
  per-selector test would help.
- **L4:** the mutated-copy proof shows the reverted model produces unclean comparisons on the
  two fixtures. It does not run `several_exif_model_is_per_selector_and_pinned` itself. That
  test asserts exactly this cleanliness, and mutant (a) above shows it fails, so the proof
  holds, but only indirectly.

### Cost self-report

- **Tokens (total):** 178,261 (harness figure; orchestrator-corrected; verifier self-report was 5,723,009 incl. cache reads) — was (input 87, output 29,386, cache creation 158,726, cache read
  5,534,810), measured before the final edit and commit calls
- **Estimated USD:** null (docs/cost-tracking.md has no Opus 5.5 rate)
- **Duration (minutes):** 45
- **Source of the number:** session transcript `subagents/agent-ac3a0c14243135951.jsonl`,
  deduplicated by message.id, max output per message (docs/cost-tracking.md "Measuring a
  cycle")

### Drift and new artifacts

- **New decisions emitted:**
  - none
- **Deviations from spec:**
  - The build skipped the one Make-as-SHORT measurement (M1). It is now measured; the model's
    stated consequence was right.
- **Follow-up work identified:**
  - none beyond the punch list. Nothing above needs a spec; L1 is already a documented
    harness limit.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — The HANDOFF-068 body was the unfilled template, so the verify contract came only from
   the dispatch prompt. Also, "Exif in APP1 split across the scan" can only exercise the
   known after-scan divergence: ExifTool stops at SOS. It cannot test the per-selector rule.
2. **Was there a constraint or decision that should have been listed but wasn't?**
   — The question `exiftool-ignores-exif-after-scan`: without it the g13 and g14 differences
   look like model errors.
3. **If you did this task again, what would you do differently?**
   — Write the grid's ExifTool runner as a script file from the start (the worktree guard
   refuses some compound shell lines), and give scratch files names unique to the cycle,
   because the scratchpad is shared with other sessions.
