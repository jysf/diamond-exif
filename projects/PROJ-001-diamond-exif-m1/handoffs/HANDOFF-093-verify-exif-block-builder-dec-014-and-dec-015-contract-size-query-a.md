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
  status: completed                # completed | blocked | rejected
  agent: claude-opus-5-5           # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: punch-list              # verify handoffs: approved | punch-list | rejected
  basis: agent-result              # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: 223885             # REAL combined count — what cost-audit reads
  estimated_usd: null              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: 15
  branch: claude/spec-025-verify
  pr: null
  completed_at: 2026-11-27         # YYYY-MM-DD
  notes: "Verified at 7cda5d3; punch list of 1 M and 4 L items. The orchestrator transcribed the harness subagent result: 223,885 tokens, 69 tool calls, 928 s."
  synced_at: 2026-11-27
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

- **Branch / PR:** `claude/spec-025-verify` (this handback only); verifies PR #71 at `7cda5d3`.
- **Completed at:** 2026-11-27
- **All acceptance criteria met?** Yes on behaviour. Every probe agrees with the code and every gate is green. The punch list covers one misdescribed mechanism (a test comment and a DEC-014 fact) and four small doc and test gaps.
- **For `verify`:** ⚠ PUNCH LIST

**Verdict: ⚠ PUNCH LIST** (at `7cda5d3`). There are no H items. Once item 1 is fixed, this approves with or without the L items.

1. **M: the `unknown-types` pin explains the wrong mechanism, and DEC-014 amendment item 2 states an incomplete fact.**
   - What the code says: `tests/exif_block_live.rs` (the `Diff` doc comment) says ExifTool "stops reading an IFD at a first entry of an unknown type". Amendment item 2 says types 14 to 18 and 255 print `Bad format (N) for ExifIFD entry 0` and no value.
   - What I measured: ExifTool 13.55, run without `-m` on scratch TIFFs whose Exif IFD holds ISO, an unknown-type entry and ColorSpace.
     - Unknown type (14 or 255) in entry 1 or entry 2: ExifTool warns `Bad format` for that entry and still prints ISO and ColorSpace.
     - Type 0 in entry 1: no warning, and both are printed.
     - Type 14 in entry 0: only the warning, and no entry of the IFD is printed.
   - So the real rule: ExifTool discards the whole IFD when entry 0 has a bad format, and skips a bad-format entry anywhere else. The pin passes only because every IFD of the shape puts its unknown-type entry first.
   - Consequence: the block shows entries ExifTool never printed for such a source. That is acceptable, because TIFF says to skip unknown fields and the block loses nothing. It should be recorded.
   - Fix:
     - Correct the comment.
     - Add a DEC-014 note: an IFD whose entry 0 has an unknown type is dropped by ExifTool, so the placed block shows more than the source.
     - Add a live shape with an unknown-type entry that is not first. Parity there should be exact, so the pin is not the only unknown-type evidence.
2. **L: the module doc of `tests/exif_block_sweep_live.rs` still describes the writer pass as `exiftool -m -Artist=sweep ...` "(`-m`, because ...)".** `b0b5130` removed `-m` from the call but not from the doc. Fix: rewrite the bullet to say no `-m` and 26 pinned refusals.
3. **L: `b0b5130` put the `WRITER_MINOR_REFUSALS` doc comment between the `///` lines of `SUBIFD_TAG` and its item** (`tests/support/exif_block_sweep_tables.rs`). As a result, the `///` text about "the one id whose value the writer pass changes" now documents `WRITER_MINOR_REFUSALS`, and `SUBIFD_TAG` has no doc. Fix: move those two lines down to just above `SUBIFD_TAG`.
4. **L: the README example uses `plan.exif_position()?` and `plan.gps_position()?`.** `?` on an `Option` does not compile in a function that returns `Result<_, BlockError>`. It also suggests both pointers are always written, but a GPS-only block has no Exif position (DEC-015 item 3). Fix: `if let Some(p) = plan.exif_position() { ... }`, and the same for GPS.
5. **L: two narrow gaps in coverage. Neither is a defect today.**
   - (a) The self-check does not check the slack bytes. I flipped every byte of the design sample's block through the `#[cfg(test)]` fault hook (in a scratch copy, both byte orders). 150 of 154 flips gave `VerifyFailed`: Entries 135, Parse 13, Positions 2. The 4 that passed are exactly the unused inline bytes of the two short inline values (bytes 24, 25, 84 and 85). Padding bytes between values would also pass, by reading the code. The byte-exact model tests cover these bytes; the runtime check does not. Fix: zero-check the slack in `check_block`, or note it in DEC-014 item 9.
   - (b) The library sweep in `omission_table_mirrors_agree` covers every tag as a LONG in the Exif and GPS IFDs, but not the Interop IFD. Fix: add Interop to that loop. I ran that sweep myself and found nothing: 1,769,463 cases, listed under point 2.

#### The nine points

1. **The bytes are right: established.**
   - I wrote my own builder and predictor (scratch `tests/zz_verify.rs`, outside the repo), working from amendment item 7 and not from `tests/support/exif_block_model.rs`.
   - Eight sources, each in LE and BE:
     - full Exif + Interop + GPS, unordered, with an odd ASCII length;
     - a dangling 0xA005;
     - GPS only;
     - an omission mix (MakerNote, 0x0111, 0x8781, type 13, type 20, count 0, 0xEA1D, 0x8568);
     - selection by parent (a SubIFD's Exif IFD walked first);
     - 0xA005 as type 13 with odd UNDEFINED values and a non-zero Exif next-word;
     - an all-omitted Exif IFD (a 0-entry table);
     - offset tags inside Interop.
   - Each was compared byte for byte with `write` at bases 8, 1000 and 4098, with `write_into` into a 0xAA-filled buffer, and on omissions and positions. All matched. Empty sources gave an empty block.
2. **The omission table is right and complete: established.**
   - My own generator (Python) built sources from `exiftool -listx -EXIF:All` (573 Exif ids, 32 GPS ids). Every writer pass ran without `-m`.
   - LONG in the Exif and Interop IFDs:
     - exactly the 11 `No size tag` ids in each IFD, so 22;
     - 4 `[minor]` refusals (0x927C and 0xC612, in each IFD);
     - 0x014A's value rewritten;
     - in the reader pass, only 0x014A changed.
   - Extra shapes:
     - SHORT gives the same 11 plus 0x014A.
     - LONG[2] gives the same 11. 0xA101 SamsungRawByteOrder came back corrupted by ExifTool's writer; it is an ExifTool quirk on a mistyped value, not an offset (the reader pass was unchanged).
     - Type 13: ExifTool refuses to write any type-13 entry (`IFD format not handled`). The block drops them all anyway.
     - GPS IFD with all 573 Exif ids and all 32 GPS ids, each of the four shapes: nothing.
   - I found no offset tag that the LONG count-1 sweep misses.
   - "Nothing beyond DEC-014's list": my classifier, typed from the amendment, against the library for every tag 0 to 0xFFFF, in three IFDs, with nine types (1,769,463 cases). The only differences were 7 cases where a non-LONG/IFD 0xA005 in the Exif IFD is refused as `BrokenPointer`, which is item 4 as designed.
   - The dangling-0xA005 rule is sound: no Interop IFD means no target to rewrite. The precedence UnknownType > MakerNote > OffsetCarrying is sound. Note that a MakerNote with an unknown type is reported as `UnknownType`, not `MakerNote`.
3. **Refusals are exactly item 4: established.** Every probe below gave the expected result.
   - Should not refuse, and did not:
     - an issue following 0xA005 from a SubIFD's Exif IFD;
     - a duplicate 0x010F in IFD0;
     - a broken IFD1;
     - a broken 0x014A inside the Exif IFD;
     - an omitted MakerNote whose value is out of range.
   - Should refuse, and did:
     - a 600-IFD chain halting after the Exif IFD was recorded → `WalkHalted`;
     - 0xA005 as SHORT in the chosen Exif IFD → `BrokenPointer`;
     - a kept value out of range → `UnreadableValue`;
     - a duplicate GPS tag → `DuplicateTag`;
     - a duplicate 0x8769 → `DuplicatePointer`;
     - 0x8769 out of range → `BrokenPointer`.
   - Judged, not punched:
     - When a SubIFD's Exif pointer and IFD0's share one offset, IFD0's raises `CyclicIfd` and the source is refused. This is literal item 4 and the same class as AC5's "Exif and GPS pointers to the same offset".
   - Allocation: I found none sized from the source before the budget check. The walk is entry-budgeted and the plan vectors are bounded by entries. The first block-sized buffer comes after `check_budget`, and so does the scratch image.
   - `base < 8` as `BadBase` is a sound tightening of DEC-015 item 2, now recorded in the clarification. Bases probed: 0, 6, 7 and 9 are refused; 8 is accepted; `u32::MAX-20` gives OverBudget. Budget 60 with base 8 is accepted, and base 10 is refused (62 > 60).
4. **The self-check cannot pass vacuously: established, with item 5(a).**
   - The check compares with the plan, whose payloads borrow the source, so an encoding fault is caught (the flip results above).
   - A plan that wrongly omits and reports an entry passes the check by design, because it is accounting. That case is caught by the independent model in `tests/exif_block.rs` and by the library sweep in `omission_table_mirrors_agree`. It is not caught by the live test (which subtracts reported omissions) or by the fuzz invariants (which only check that omissions name real entries).
   - The fault hook and its call site are both `#[cfg(test)]`.
5. **The tests are independent and not vacuous: established.**
   - The model, the placer and `fuzz/shared/exif_block_invariants.rs` import nothing from `src/write/block.rs`. The fuzz invariants use only the public API.
   - I ran 13 mutations of `block.rs` in a scratch copy (lib tests, `exif_block`, `fuzz_layout exif_block`). All 13 went red:
     - drop the padding;
     - keep 0x8781/0xA010/0xBCC0/0xBCC2;
     - keep type 13;
     - swap Interop and GPS;
     - select by kind;
     - skip the plan budget;
     - skip the write budget;
     - no zero-fill;
     - MakerNote before UnknownType;
     - no halt refusal;
     - allow base < 8;
     - keep a dangling 0xA005;
     - no duplicate-tag check.
   - `omission_table_mirrors_agree` compares the shapes file, the model, the sweep tables and the library's behaviour, which are all the mirrors AC4 lists. The fuzz harness holds no table.
6. **The live tests are honest: established, with item 1.**
   - Every `source_only` pin is an IFD outside the block by DEC-014 item 1 / amendment item 5 (an Exif IFD under a SubIFD or IFD1, 0xA005 in IFD0, SubIFDs inside Exif, the DNG's GPS in the raw SubIFD). Each must be present in the source's listing, so none hides a parity break.
   - `placed_only` is allowed only when the source printed nothing.
   - The `-m` fix in `b0b5130` is correct: no `"-m"` remains in any live test call, and the 22 + 4 refusals reproduce independently. The doc leftovers are items 2 and 3.
7. **Panic-freedom and fuzz: established.**
   - `block.rs` uses only `get`/`get_mut`, `checked_*`/`saturating_*` and `try_from`. It has no `allow`, and it is under the five-lint deny set (lint-no-allow is green).
   - 60 s smokes:
     - `exif_block`: 292,948 runs;
     - `tiff_ifd`: 1,614,628 runs;
     - `rewrite_tiff`: 156,595 runs;
     - all exit 0, with 0 crashes.
   - The seed count is pinned at 473 (`tests/fuzz_layout.rs`), matching 473 files, and `exif_block` is in the `ci.yml` fuzz matrix.
8. **Docs: established, with items 1 to 4.**
   - The CHANGELOG entry matches the evidence. The README matches except item 4.
   - DEC-015 (base-first, the scratch image, `base + len + 34`, base < 8) matches the code.
   - DEC-014's amendment matches the code, apart from the ExifTool fact in item 1.
   - `just decisions-audit --changed origin/docs/spec-025-design` is advisory, exit 0.
9. **Gates on 1.90.0: all green.**
   - `just lint`, `lint-no-allow`, `lint-fuzz`, `forbid-unsafe`, `deny`, `deny-fuzz`, `validate`, `cost-audit`: each exit 0.
   - `validate`: 42 artifacts valid, 179 parse.
   - `just mutation-oracle`: 43 passed, 1 ignored.
   - `DIAMOND_EXIF_REQUIRE_ORACLE=1 just test`: 1,599 passed, 0 failed, 22 ignored across 44 test binaries. The ignored ones are the same two utility tests repeated in every binary. There were no SKIP lines, and both live tests ran.
   - `cargo fmt --check` passes for the root and `fuzz/`.
   - `fuzz/Cargo.lock` was restored; the tree is clean.

What I did not establish:
- No 24 h fuzz run.
- ExifTool's behaviour on real camera files beyond the repo's shapes.
- Whether 0xC612 would be relocated with a value ExifTool accepts. The reader pass shows it is not read through, and the block keeps it.

### Cost self-report

- **Tokens (total):** null. I ran as a subagent and cannot see my own token count; the orchestrator records the harness figure.
- **Estimated USD:** null
- **Duration (minutes):** about 75
- **Source of the number:** none available to the verifier (harness report, via the orchestrator)

### Drift and new artifacts

- **New decisions emitted:** none. Item 1 asks for a DEC-014 note.
- **Deviations from spec:** none in behaviour.
- **Follow-up work identified:**
  - Item 1's live shape (unknown type not in entry 0).
  - Item 5(b): add Interop to the mirror sweep.
  - Item 5(a): optionally, the slack check.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Very little. The handoff's own phrasing of the `unknown-types` mechanism ("stops reading an IFD at the first unknown-type entry") repeated the test's claim. I only saw it was wrong by probing.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — No. The worktree isolation rejects compound shell commands, so scratch scripts had to be written with the editor and then run.

3. **If you did this task again, what would you do differently?**
   — I would probe ExifTool's behaviour behind every pin first. It was the cheapest check and found the only M item.
