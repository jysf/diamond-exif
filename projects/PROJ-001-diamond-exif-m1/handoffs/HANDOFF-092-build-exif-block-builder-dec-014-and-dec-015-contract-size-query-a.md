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
  id: HANDOFF-092
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-27
  status: completed                # pending | accepted | completed | rejected

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
  agent: claude-sonnet-5-5                      # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: null                    # verify handoffs: approved | punch-list | rejected
  basis: agent-result             # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: 564052             # REAL combined count — what cost-audit reads
  estimated_usd: null              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: 54
  branch: feat/spec-025-exif-block-build
  pr: 71
  completed_at: 2026-11-27               # YYYY-MM-DD
  notes: "Parts A and B, one subagent each (claude-sonnet-5-5). tokens_total and duration are the sums of the harness subagent results, filled by the orchestrator: A 274,448 tokens / 68 tool calls / 1,318 s; B 289,604 tokens / 151 tool calls / 1,913 s. The orchestrator's own fix (b0b5130, the sweep without -m) is main-loop work, counted in STAGE-007's orchestration cost."
  synced_at: 2026-11-27
---

# HANDOFF-092: SPEC-025 build: the Exif block builder in the source's byte order

**FIRST STEP, before anything else:** `git checkout -b feat/spec-025-exif-block-build docs/spec-025-design`. That
branch carries the design: the spec, DEC-014's amendment of 2026-11-27, DEC-015's clarification and this handoff. The
harness may have started your worktree from main. (If the design PR has merged by then, branch from `main` instead.)

## Delegation Summary

`claude-opus-5-5` (architect) hands SPEC-025 to `claude-sonnet-5-5` (implementer) for the **build** cycle. There are
**two parts, one agent each**. The prompt that starts you names your part. Run only that part, then stop at its
commit.

| Part | Acceptance criteria | Ends with |
|---|---|---|
| A | AC1 to AC8, AC10: `src/write/block.rs`, the self-check, unit tests, `tests/exif_block.rs`, the shapes, the independent model, the test placer | library committed first; A's tests green; `just lint` and `just lint-no-allow` green |
| B | AC9, AC11, AC12: the two gated live tests, the `exif_block` fuzz target, seeds, pins, CI, README, CHANGELOG, any DEC correction | every gate in AC12 green; `just fuzz-smoke exif_block` 60 s clean |

The design is done. The spec's *Design measurements*, **DEC-014's amendment of 2026-11-27** and **DEC-015's
clarification of 2026-11-27** are normative. Do not re-decide them. If the code or ExifTool contradicts them (a
measured count does not reproduce, or a sweep finds a tag the table lacks), add a question to
`guidance/questions.yaml`, stop, and report it. Do not change the table to match what you got.

**Gate on starting:** cleared. jysf accepted DEC-014 as amended on 2026-11-27 (`status: accepted`).

## Context the Receiving Agent Needs

### Primary

Read in this order:
1. `AGENTS.md` §11, §12, §13 and §15 ("During build").
2. `guidance/toolchain-brief.md` and `guidance/constraints.yaml`.
3. **`projects/PROJ-001-diamond-exif-m1/specs/SPEC-025-*.md`.** This is the source of truth: Context, *Design
   measurements*, the API in Outputs, AC1 to AC12, Failing Tests and Notes.
4. **`decisions/DEC-014-*.md`** in full, especially items 1, 4, 5, 7, 8 and 9 and the **amendment of 2026-11-27**
   (items 1 to 7). That amendment is the rule book for content, omissions, refusals and layout.
5. **`decisions/DEC-015-*.md`**: items 1 to 5 and the **clarification of 2026-11-27** (the scratch image).
6. DEC-005 (pointer tags and walk order: SubIFDs first), DEC-001 (the tolerant walk), and DEC-013 part 7 (the
   self-check pattern).
7. The stage file `stages/STAGE-007-*.md`, Design Notes.

The code and harness files are listed in the spec's Inputs and Outputs.

### Decisions that apply

- `DEC-014` + amendment 2026-11-27: what the block holds, what it leaves out and why, which walk issues refuse, and
  the layout.
- `DEC-015` + clarification 2026-11-27: size query, then write at an even base with final offsets; the self-check runs
  on the exact bytes at that base, in a scratch image that counts against the budget.
- `DEC-005`: the walk follows SubIFDs before IFD0's Exif pointer, so select the Exif and GPS IFDs by parent.
- `DEC-013` part 7: read your own output back before returning it.

### Constraints that apply

- `test-before-implementation`: commit each test red before its code, and say in the handback which commits did.
- `panic-free-library` and `forbid-unsafe-code`: the five-lint deny set (`unwrap`, `expect`, indexing, `panic`,
  `arithmetic_side_effects`). Length arithmetic in `u64` with `checked_*`.
- `no-io-in-library`: the builder takes `&[u8]` and returns bytes.
- `no-new-top-level-deps-without-decision`: none, dev ones included.
- `fuzz-target-per-surface`: the `exif_block` target is part of this spec (Part B).
- `exiftool-never-linked`: ExifTool is run, never read. The sweep classifies tags by ExifTool's behaviour, not its
  source.

### Prior related work

- SPEC-022 / `src/write/exif.rs`: the offset and pointer tag lists to share (`POINTER_TAGS`, `OFFSET_TAGS`,
  `EXIFTOOL_SUBDIRECTORY_TAGS`, `EXIFTOOL_BLOB_CARRIER_TAGS`).
- SPEC-037 / SPEC-038: `tests/strip_webp_live.rs` and `tests/strip_webp_sweep_live.rs` (the gated live-test and sweep
  pattern), and `rewrite_webp` (the fuzz-target pattern, with invariants independent of the writer).
- irradiance/SPEC-081 (shipped): the placing side copies the block to offset 8 and writes 0x8769 and 0x8825 as
  8 + position. Your test placer does the same at any even base.

## Expected Deliverables

- Code and tests implementing SPEC-025's AC1 to AC12 (your part's share).
- Part A commits on `feat/spec-025-exif-block-build`. Part B continues on the same branch and opens the PR against
  `main`. The PR description names HANDOFF-092, SPEC-025, STAGE-007, PROJ-001, DEC-014 and DEC-015, and the question
  ids you added.
- In the handback: the red-then-green evidence (commit SHAs), every measured number the live tests reproduced (the
  sweep's 22 writer refusals and 2 reader differences), and any deviation.

## Out of Scope

- Byte-order conversion and the UNDEFINED rule (SPEC-041). A cross-order request has no API yet; do not add one.
- Recorded, red-proofed ExifTool parity records (SPEC-026). AC9 is a live smoke.
- Policy presets (SPEC-027).
- Changing `OFFSET_TAGS` or anything else the strip does (question `strip-offset-tags-incomplete`).
- Changing the TIFF walk (`src/tiff/mod.rs`) beyond widening `type_size` to `pub(crate)`.

## Return Criteria — how to hand back

**Completing this handoff means filling in the `handback:` front-matter block above AND the `## Handback` section
below.** Leave `tokens_total` null: the delegated run cannot see its own spend, so the orchestrator records the
harness figure. Say so in `notes`.

On success:
1. Fill the `handback:` front-matter, with `tokens_total: null` and a note.
2. Fill the `## Handback` section. The reflection is part of it.
3. Set `handoff.status` → `completed` and `handback.status` → `completed` (Part B; Part A sets neither and reports
   in the PR or branch instead).
4. Part B opens the PR.

If you cannot complete the task:
1. Fill the `## Handback` section with what was done and what blocked you.
2. Set `handoff.status` → `rejected` and `handback.status` → `blocked`.
3. Set the spec's `task.blocked: true`, and add a question to `/guidance/questions.yaml`.

---

## Handback

*Filled in by the receiving agent. The orchestrator does not reconstruct any of
this — it transcribes it. The reflection questions are part of completion.*

### Execution notes

- **Branch / PR:** `feat/spec-025-exif-block-build`, PR #71 (https://github.com/jysf/diamond-exif/pull/71)
- **Completed at:** 2026-11-27
- **All acceptance criteria met?** Yes for Part B (AC9, AC11, AC12). Part A's commits `3b359a3` (red tests and API stub), `c3c1837` (library) and `551bde5` came first. Part B commits: `961202f` (the two live tests, the sweep tables, the mirror comparison), `130e177` (the fuzz target, seeds, pins, CI, lists), `dd3f594` (README, CHANGELOG, DEC-015 note, lint fixes). The Part B tests were written against a library that already existed, so there was no red phase for them; the live tests were red on first run in two ways, both fixed in the tests and neither in the library (see Deviations).
- **Reproduced numbers (ExifTool 13.55):** 1,178 sweep sources (573 Exif ids x Exif and Interop IFDs, 32 GPS ids). Writer pass: 22 refused, all with `No size tag` (11 tags x 2 IFDs); 2 changed (0x014A in the Exif and Interop IFDs); 1 lost its Exif IFD (Exif 0x013B Artist); 1,153 unchanged. Reader pass: 2 listings change (0x014A in both IFDs; `SubIFD:Software` disappears). ExifTool's offset set (24 sources) is inside the block's `OffsetCarrying` set; the only other omissions are MakerNotes and the documented extras.
- **Gates (Rust 1.90.0):** `just lint`, `lint-no-allow`, `lint-fuzz`, `forbid-unsafe`, `deny`, `deny-fuzz` green; `DIAMOND_EXIF_REQUIRE_ORACLE=1 just test` 1,599 passed, 0 failed, 22 ignored (the pre-existing `tools::tests::env_limit_child` copy per test binary; none in the new files), 0 `SKIP` lines; `just mutation-oracle` 152 passed, 0 failed; `just validate` and `just cost-audit` green; `cargo fmt --check` for both crates clean. `just fuzz-smoke exif_block`: 385,235 runs in 61 s, no crash.
- **Seeds:** 473 (234 shapes x bases 8 and 1000, three edge bases for the design sample, an empty and a one-byte input), pinned as `EXIF_BLOCK_SEEDS` in `tests/fuzz_layout.rs`.

### Cost self-report

- **Tokens (total):** null. The delegated run cannot see its own spend; the orchestrator records the harness figure.
- **Estimated USD:** null
- **Duration (minutes):** about 90 (estimate)
- **Source of the number:** none available

### Drift and new artifacts

- **New decisions emitted:** none. A note was added to DEC-015's clarification of 2026-11-27: a non-empty block refuses `base < 8` (`BadBase`).
- **Deviations from spec:**
  - The writer pass of the sweep runs `exiftool -m`. Without it ExifTool also refuses the swept 0xC612 (DNGVersion) and 0x927C (MakerNote) with a `[minor]` error about the value 4096 (4 more refusals, 26 in all), which is about the swept value and not an offset. With `-m` the 22 `No size tag` refusals reproduce exactly. The design's numbers did not say whether it used `-m` or a different target value.
  - AC9(a) does not run ExifTool on one source, `ok-subifd-loop` (IFD0's SubIFD leads to an IFD whose SubIFD leads back to IFD0): ExifTool recurses on it without returning in 60 s ("Deep recursion", the `two-hop-cycle` hazard of `tests/support/tools.rs`). For that shape only the placed files are read, against a pinned expectation.
  - AC9(a) pins, per shape, the lines ExifTool prints for the source that the block cannot hold (an Exif IFD reached through a SubIFD or IFD1, an Interop IFD pointed to from IFD0, the DNG's GPS in its raw SubIFD) and the lines only the placed file prints (ExifTool stops reading an IFD at a first entry of an unknown type, so the `unknown-types` source prints nothing from those three IFDs). Every pin is checked to be present, so a change turns the test red.
  - ExifTool runs once per shape (its files) over a few threads in AC9(a), because one hostile source took a whole-directory run down.
  - The fuzz harness accepts `BadBase` from `write` only for a base below 8 and a non-empty block, and retries under the default budget when the small budget cannot hold the self-check scratch image. It predicts no omission reasons, so it holds no table for `omission_table_mirrors_agree`; that test compares the sweep tables with the shapes file and the model.
- **Follow-up work identified:** none new. The sweep's `-m` and the `ok-subifd-loop` handling are the points SPEC-026 (recorded parity) should know about.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — The design's sweep command: whether `-m` was used decides whether 22 or 26 sources are refused. The ExifTool hang on `ok-subifd-loop` was not mentioned although the shape is in the grid.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — A note that ExifTool must not be run on SubIFD-cycle sources (the repo already knows it for `two-hop-cycle`), and that ExifTool stops an IFD at a first unknown-type entry, which makes "same as the source" false for the `unknown-types` shape.

3. **If you did this task again, what would you do differently?**
   — Run ExifTool one shape at a time from the start, and read its output on the shapes before writing the comparison, instead of discovering the divergent shapes one run at a time.
