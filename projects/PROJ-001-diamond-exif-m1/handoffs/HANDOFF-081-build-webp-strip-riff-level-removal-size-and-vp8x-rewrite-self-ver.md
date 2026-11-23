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
  id: HANDOFF-081
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-22
  status: completed                  # pending | accepted | completed | rejected

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
  status: completed                     # completed | blocked | rejected
  agent: claude-sonnet-5-5                      # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: null                    # verify handoffs: approved | punch-list | rejected
  basis: agent-result                      # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: 401638               # REAL combined count — what cost-audit reads
  estimated_usd: 2.65              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: null
  branch: claude/spec-038-part-a
  pr: null
  completed_at: 2026-11-23               # YYYY-MM-DD
  notes: null                      # if unusual; double-quote it — an unquoted " #" starts a YAML comment
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-081: WebP strip, PART A (measurement, reader/audit extension, planner, self-check)

**FIRST STEP, before anything else:** `git checkout -b claude/spec-038-part-a claude/spec-038-design`
(the harness may have started your worktree from main, which lacks the spec and DEC-022).

## Delegation Summary

`claude-opus-5-5` hands `SPEC-038` Part A to `claude-sonnet-5-5`: measure which RIFF
chunks ExifTool reads as metadata in a WebP and how each type folds case, extend the
reader and audit where it reads more than the audit lists, build `src/write/webp.rs`
and the WebP self-check, with unit tests. Part B (fuzz target, seeds, pins, plants,
CI, live ExifTool test, README, DEC fill) is a separate fresh handoff after Part A is
reviewed. Do not start Part B.

## Context

Read first: `AGENTS.md`, `guidance/toolchain-brief.md`, **`decisions/DEC-022-*.md`
(the design: follow it, do not re-decide)**, `decisions/DEC-021-*.md` with its
clarifications (the PNG precedent and its lessons), the spec
`projects/PROJ-001-diamond-exif-m1/specs/SPEC-038-*.md` (not the -timeline file),
DEC-013 (all amendments), DEC-019, DEC-020, `src/write/png.rs` (your model),
`src/write/exif.rs` (`plan_block`), `src/write/verify.rs`, `src/webp/*`,
`src/audit/mod.rs` (`audit_webp`).

Toolchain: `export RUSTUP_TOOLCHAIN=1.90.0 PATH="$HOME/.cargo/bin:$PATH"` (lint on
the pinned 1.90.0; also run `just lint-fuzz`); macOS has no `timeout`
(`perl -e 'alarm 8; exec @ARGV' --`); BSD `sed -i` needs `''`; Python for multi-line
edits. `indexing_slicing`, the five-lint deny set, `forbid(unsafe_code)` and no
panics apply in `src/`. Never commit `decisions/INDEX.md`; restore `fuzz/Cargo.lock`
with git after fuzz or lint-fuzz runs. No CHANGELOG entry. Leave no background
processes running. Compare against `origin/main`. Records must not contain machine
paths.

## Part A, in order

1. **Measure the carriers AND the case folding, report BEFORE building.** One chunk
   per file (a minimal valid WebP, with a VP8X where the chunk needs one, with the
   chunk before the image data, after it, and beyond the declared RIFF size),
   ExifTool 13.55 `-a -u -G1 -H -s`, 8 s limit. Cover: `EXIF` (with and without the
   `Exif\0\0` prefix), `XMP `, `XMP\0`, `ICCP`, `LIST` of every INFO sub-id you can
   think of (IARL, IART, ICMS, ICMT, ICOP, ICRD, ICRP, IDIM, IDPI, IENG, IGNR, IKEY,
   ILGT, IMED, INAM, IPLT, IPRD, ISBJ, ISFT, ISHP, ISRC, ISRF, ITCH) and other LIST
   types, `JUNK` and `PAD ` with text and with zeros, `C2PA` (a real minimal JUMBF
   box with an Exif and XMP assertion if you can build one), `IDIT`, `ID3 `, `id3 `,
   `strd`, `strn`, `cue `, `labl`, `note`, `ltxt`, `bext`, `iXML`, `PrmL`, `Cr8r`,
   `fact`, `smpl`, `inst`, plus anything else you can find that makes ExifTool print a
   tag from a WebP (try every FourCC you know from RIFF, AVI and WAV and report the
   ones that print). Use GPS-bearing TIFF and XMP with a city where the chunk can
   carry Exif or XMP. For EACH type that prints something: all 16 ASCII case
   spellings of its four bytes, and whether ExifTool decodes each. Also measure: Exif
   inside an `ANMF` frame payload (does ExifTool decode it?), and what the audit lists
   today for each. Report the full table in your handback FIRST.
2. **Reader and audit extension (only where step 1 requires).** Locate each chunk
   type ExifTool decodes that the audit does not list as a region (small `src/webp`
   change, mirroring what SPEC-037 did in `src/png`), with a category per DEC-022
   clause 4 (carrier: removed by every preset; `C2PA` as `Jumbf`; non-metadata
   carriers `OtherSegment`). Apply the measured case-folding rule exactly (only the
   types that fold). Amend DEC-020 with a "Clarifications from the SPEC-038 design"
   section; update the WebP parity tests, the inventory and category replays and the
   fuzz invariant mirrors this moves; report every test or record that moves and why.
   Its own commit first.
3. **`src/webp/…` planner `src/write/webp.rs`** per DEC-022 clauses 1 to 8, modelled
   on `src/write/png.rs`: targets from the chunk walk (incl. after the declared size
   and after a null chunk), the `EXIF` edit through `plan_block` (patch offsets +
   `tiff_at`), carriers, the RIFF size patch (subtraction rule, `PatchKind::RiffSize`),
   the `VP8X` flag patch (`PatchKind::Vp8xFlags`), refusals. `strip` dispatches
   `Sniffed::Webp`. Keep the StripError set; add a typed reason only where it cannot
   be expressed.
4. **Self-check** in `verify.rs` per DEC-022 clause 9, including the carrier
   exemption of check (b) (as for PNG), the size relation and the flag agreement.
5. **Tests**: the spec's Failing Tests for `src/write/webp.rs` and `tests/strip_webp.rs`
   (per preset, per chunk kind and position, I1 recomputation, size cases, flag
   cases, refusals, idempotence, 17+ `EXIF`, beyond-declared, after-null-chunk, the
   case grid x every preset asserting none of the folding spellings survive), a test
   that runs ALL 98 `oracle_webp` fixtures through `strip` under the five presets
   (typed refusals allowed, panics and `VerifyFailed` not), and update the SPEC-032
   WebP refusal test (HEIC and GIF still refuse). `ContainerKind::strippable()` stays
   false (a test pins it).
6. **A small live ExifTool spot check** (not the full Part B test): run ExifTool on
   the OUTPUT of strip for at least one file per carrier row under all five presets
   from a scratch script, and report any tag it still prints that the preset should
   have removed. A leak is a finding to report, not to hide.
7. **Gates on the pinned 1.90.0:** `just lint`, `just lint-fuzz`, `just test`,
   `just forbid-unsafe`, `just validate`, `just cost-audit`, `just oracle-record`
   twice (no diff), and the 60 s smokes of every fuzz target whose code you changed
   (`webp_riff`, `audit`, `audit_render`, `png_chunks`, as applicable). Commit (the
   library extension first, then the writer) on `claude/spec-038-part-a` with trailer
   `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`. No PR. Leave tokens
   null in the handback (the orchestrator records the harness figure).

## Budget

About 100 tool calls or 250k context. Stop and hand back what is done if either is
near (after the step 1 table and the step 2 commit if need be); do not loop.

## Out of Scope

Part B items; the audit/CLI switch-over; ExifTool write parity; a CLI strip command.

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

- **Tokens (total):** 401638 (harness subagent_tokens)
- **Estimated USD:** 2.65
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
