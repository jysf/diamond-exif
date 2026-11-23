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
  id: HANDOFF-082
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-22
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-038

project:
  id: PROJ-001
  stage: STAGE-010
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
  basis: agent-result                      # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: 349393               # REAL combined count — what cost-audit reads
  estimated_usd: 2.31              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: null
  branch: claude/spec-038-part-b
  pr: null
  completed_at: 2026-11-22
  notes: "tokens_total is null: the implementer harness reports no token count to the agent; the orchestrator fills it from the harness result"
  synced_at: 2026-11-22
---

# HANDOFF-082: WebP strip, PART B (rewrite_webp fuzz target, seeds, pins, plants, CI, live ExifTool test, README, DEC fill)

**FIRST STEP, before anything else:** `git checkout -b claude/spec-038-part-b claude/spec-038-design`
(the design branch already contains Part A, commits `b69948d` and `30825c3`; the
harness may have started your worktree from main, which does not).

## Delegation Summary

`claude-opus-5-5` hands `SPEC-038` Part B to `claude-sonnet-5-5`: everything in the spec
that Part A (HANDOFF-081) did not do. Do not change `src/write/webp.rs` or the
`src/webp`/`src/audit` behaviour; if a fuzz or live finding shows a real bug there, stop
and report it.

Part A built (read HANDOFF-081's handback and the commits): `webp::chunk_class`,
`WebpRegion::Chunk`, carriers as `RegionKind::Carrier`/`OtherChunk`, `C2PA` as `Jumbf`,
`src/write/webp.rs`, `verify_webp`, `PatchKind::{RiffSize, Vp8xFlags}`,
`tests/strip_webp.rs`, `tests/webp_carriers.rs` + `tests/support/webp_carriers.rs` (the
measured table as data), DEC-020 clarification. Measured: no RIFF chunk type folds case;
`LIST` (any), `IDIT`, text `JUNK`, `JUNQ`, `SEAL`/`_PMX`/`aXML`/`iXML`, `afsp`/`bext`/
`guan`/`list`/`labl`/`note`/`ltxt`/`olym`, `ID3 `/`id3 `, `gps0`/`SLLT`/`SGLT` are carriers;
`C2PA` Jumbf; `CSET`/`acid`/`ds64`/`fact`/`fmt `/`inst`/`smpl`/`cue `/`plst`/`gsen`
OtherChunk. Judgement calls kept: every LIST is a carrier; JUNK only by the TextJunk rule.

## Context

Read: `AGENTS.md`, `guidance/toolchain-brief.md`, `decisions/DEC-022-*.md`,
`DEC-021-*.md` with its clarifications, the spec, the PNG models you mirror:
`fuzz/fuzz_targets/rewrite_png.rs`, `fuzz/shared/rewrite_png_invariants.rs`,
`tests/fuzz_layout.rs` (the rewrite_png section), `tests/strip_png_live.rs`,
`examples/fuzz_seeds.rs`, `.github/workflows/ci.yml` (line ~187), `app.just`.

Toolchain: `export RUSTUP_TOOLCHAIN=1.90.0 PATH="$HOME/.cargo/bin:$PATH"` (lint on the
pinned 1.90.0; also `just lint-fuzz`); fuzz needs nightly; macOS has no `timeout`
(`perl -e 'alarm 8; exec @ARGV' --`); BSD `sed -i` needs `''`; Python for multi-line
edits. Never commit `decisions/INDEX.md`; restore `fuzz/Cargo.lock` with git after fuzz or
lint-fuzz runs. No CHANGELOG entry. Leave no background processes. Compare against
`origin/main`. Records must not contain machine paths.

## Part B, in order

1. **`rewrite_webp` fuzz target.** `fuzz/fuzz_targets/rewrite_webp.rs` and
   `fuzz/shared/rewrite_webp_invariants.rs`, modelled on `rewrite_png`: bytes and a
   policy byte (presets 0-4, empty, mask, All, single-category Custom sets incl.
   `OtherSegment` and `Icc`). On `Ok`: length at most the input's; no finding in a
   removed category; the exact predicted output (the input with the predicted removed
   ranges deleted and the RIFF size and `VP8X` flag patches applied, derived
   INDEPENDENTLY from the harness's own RIFF walk, its own size rule and its own flag rule);
   kept chunks byte-identical and in order; idempotent; deterministic; on `Err`: the
   predicted refusal class and offset, no bytes. Independence of CODE (a test asserts no
   line names `write::webp`, `plan_block`, `exif9`, `removes_carriers`) AND of FACTS: the
   harness's carrier table is typed from the measured table in
   `tests/support/webp_carriers.rs`/DEC-022, and a test cross-checks the harness table
   against that data row by row (so a wrong measurement shows as a diff against the
   recorded facts, not as agreement between two copies of the same mistake). Register in
   `fuzz/Cargo.toml`.
2. **Seeds** `fuzz/seeds/rewrite_webp/` generated by `examples/fuzz_seeds.rs` (no
   hand-made binaries): the WebP seed shapes plus one seed per DEC-022 rule (EXIF
   all/none/edit with and without prefix, XMP, `XMP\0`, ICCP, each carrier class, `C2PA`,
   `OtherChunk`, empty chunks, before/after image data, beyond the declared size, after a
   null chunk, 17+ EXIF, size too small/too large, VP8X flags set and unset, animated,
   missing pad, trailing bytes, walk halted, chunk past EOF). Policy byte first, under
   every policy byte for the key shapes (the `GpsAndDevice` and `AllPii` coverage that the
   PNG verify found thin).
3. **Pins and plants** in `tests/fuzz_layout.rs`: seed count and per-rule names, every
   seed x every policy byte x prefix truncations, degenerate inputs (huge chunk counts,
   chunk lengths 0xFFFFFFFF), each refusal class reached by a committed seed, a
   target-declared-and-in-CI test, the independence test, and plants (at least 2 per
   guard, each with a control) for: a writer leaving a removed chunk, not patching the
   RIFF size, patching it wrongly, leaving a stale `VP8X` flag, clearing a flag it must
   keep, cutting a kept chunk, a wrong refusal reason/offset.
4. **CI.** Add `rewrite_webp` to the fuzz smoke matrix (ci.yml ~187), `app.just` (usage
   comment and the default target list), README command list, and every test that pins the
   target list (eleven targets). Run `just fuzz-smoke rewrite_webp` (60 s nightly) clean,
   then `webp_riff`, `audit`, `audit_render`.
5. **Live ExifTool test `tests/strip_webp_live.rs`** (gated `DIAMOND_EXIF_REQUIRE_ORACLE`,
   live only, ExifTool `-a -u -G1 -H -s`, 8 s limit, a few threads), modelled on
   `tests/strip_png_live.rs`: for every carrier row of the measured table (one file per
   row, plus the three main types), the position grid (before, after, beyond the declared
   size) and the case grid for each row's code (all spellings must be untouched or
   removed according to the table), a dozen representative `oracle_webp` fixtures, and the
   17-`EXIF` and size-variant files, strip under all five presets, run ExifTool on the
   OUTPUT and fail if it still prints a tag the preset removes (the pinned tag list per
   preset as in `strip_png_live`: GPS group and `Composite:GPS*`; serial numbers from
   `GpsAndDevice`; `Artist`/`Copyright`/`Author`/`OwnerName`/`CameraOwnerName` and the
   LIST INFO personal tags from `AllPii`; every tag of a decoded carrier under every
   preset). Each carrier file must print a removable tag BEFORE the strip (never
   vacuous). Also check each output still decodes (its pixel chunks are byte-identical)
   and ExifTool prints no `Error` for it. Include a red-check test (the check reports
   violations on an unstripped file).
6. **DEC-022 fill.** Put the measured carrier and case table in DEC-022 (from
   HANDOFF-081's report and `tests/support/webp_carriers.rs`), mark located kind and
   category per row, and record the Part A judgement calls as clarifications (every LIST is
   a carrier; JUNK by the TextJunk rule; `C2PA` residual like PNG `caBX`; `SGLT`/`gsen`
   grouping; size and flag self-check failures reuse `Check::MpfFields`).
7. **README.** The writer strips WebP (library only; the audit and CLI still say not
   strippable until the ExifTool write-parity spec); the eleven fuzz targets; the audit
   paragraph names the WebP carrier regions; the stated residuals (`C2PA`; carriers removed
   under `Gps` while XMP stays; over-removal of every LIST).
8. **Gates on the pinned 1.90.0:** `just lint`, `just lint-fuzz`, `just test`,
   `just mutation-oracle`, `just forbid-unsafe`, `just validate`, `just cost-audit`,
   `just oracle-record` twice (no diff), the live test run with
   `DIAMOND_EXIF_REQUIRE_ORACLE=1`. Commit on `claude/spec-038-part-b` with trailer
   `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`. No PR. Leave tokens null in
   the handback. Tick the remaining acceptance boxes.

## Budget

About 100 tool calls or 250k context. If either is near, stop and report what is done; do
not loop.

## Out of Scope

`src/write/webp.rs` and the reader/audit behaviour; PNG; the audit/CLI switch-over;
ExifTool write parity (SPEC-039); a CLI strip command.

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

- **Branch / PR:** `claude/spec-038-part-b` (from `claude/spec-038-design`), no PR
- **Completed at:** 2026-11-22
- **All acceptance criteria met?** yes (Part B items; see the commit message for the live test findings)
- **For `verify`:** not applicable

### Cost self-report

Mirror what you put in the `handback:` front-matter, and say where the number
came from. **This is the number that lands in the spec** — the orchestrator
transcribes it via `just handback-sync`, it does not estimate it.

- **Tokens (total):** 349393 (harness subagent_tokens)
- **Estimated USD:** 2.31
- **Duration (minutes):** not measured
- **Source of the number:** none available

### Drift and new artifacts

- **New decisions emitted:** none (DEC-022 filled: measured table, eight clarifications)
- **Deviations from spec:**
  - The ID3 fixture in `tests/support/webp_carriers.rs` declared a frame size one byte too large, so ExifTool printed only `File:ID3Size`; corrected (the live test found it).
  - The live test's XMP-with-GPS file is judged with the stated residual (a preset without `Xmp` keeps the XMP packet and its GPS), the other XMP files carry no GPS.
- **Follow-up work identified:** none for the writer; no bug found in `src/write/webp.rs` or the reader.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Nothing blocking. The shell sandbox refuses heredocs and multi-command lines for worktree agents; files were written with the editor tool.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — Whether the live test should treat GPS from a kept XMP packet as a failure (it is the stated residual, so it does not).

3. **If you did this task again, what would you do differently?**
   — Run the live test against a fixture's measured `prints` tag first; that is how the wrong ID3 payload was found only at the end.
