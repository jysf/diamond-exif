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
  id: HANDOFF-076
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-22
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-036

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

# HANDOFF-076: SPEC-036 verify punch list (fix round)

**FIRST STEP, before anything else:** `git checkout -b claude/spec-036-fixes claude/spec-036-verify`
(the harness may have started your worktree from main).

## Delegation Summary

`claude-opus-5-5` hands the SPEC-036 verify punch list to `claude-sonnet-5-5` for
one fix round. Read first: `AGENTS.md`, `guidance/toolchain-brief.md`, the spec,
`HANDOFF-075-*.md` (the verify report: findings, evidence and the variant grid
with the scratch files under the scratchpad `v036/` directory if still present),
`tests/support/inventory.rs`, `tests/inventory_oracle.rs` (SPEC-036 section),
DEC-018 and DEC-020.

Toolchain: `export RUSTUP_TOOLCHAIN=1.90.0 PATH="$HOME/.cargo/bin:$PATH"` (lint on
the pinned 1.90.0); macOS has no `timeout` (`perl -e 'alarm 8; exec @ARGV' --`);
BSD `sed -i` needs `''`; Python for multi-line edits. Never commit
`decisions/INDEX.md` or `fuzz/Cargo.lock`. No CHANGELOG entry. Leave no
background processes running. Compare against `origin/main`, not the stale local
`main`.

## The punch list, in order. The architect decides these; do not re-decide.

1. **H1: a REAL audit bug (library fix).** ExifTool matches the keyword
   `Raw profile type <name>` case-insensitively and decodes it; `src/png/metadata.rs`
   matches `RAW_PROFILE_PREFIX` exactly, so a `raw profile type exif` chunk with GPS
   is a plain `Text` region and the audit says clean. First MEASURE with ExifTool
   which spellings it decodes: the prefix in lower/upper/mixed case, the profile
   name in lower/upper case (`exif`, `EXIF`, `APP1`, `xmp`, `iptc`, `icc`), in
   `tEXt`, `zTXt` and `iTXt`, with a valid payload and with GPS; pin the measured
   rule (ASCII case-insensitive on the prefix; measure the name). Fix the
   classifier so those chunks are `RawProfile` regions; DEC-020 clause 5 then
   makes the file `unreadable`. Add tests (library unit tests and the
   corpus rows), update `png-shape-meta-raw-lowercase` and the pin
   `png-raw-profile-keyword-case`: the keyword-case divergence disappears, the file
   falls under `png-raw-profile-not-decoded`; rewrite or remove the question
   entry accordingly (its text "not decoded either way" was false). Add a
   "Clarifications from the SPEC-036 verify" section to DEC-018. Make this its own
   commit first (a clean checkpoint) and re-run the PNG parity and oracle tests
   (`tests/png_oracle*`, `tests/audit_png_webp*`, fuzz seeds pins) since a `src/`
   change can move them; report every test or record that moves and why.
2. **M1: non-text PNG chunk tags that print id-less in group `PNG`.** ExifTool
   prints `Gamma` (gAMA), `SRGBRendering` (sRGB), `BackgroundColor` (bKGD),
   `Transparency` (tRNS), `SignificantBits` (sBIT), `ImageOffset` (oFFs),
   `AppleDataOffsets` (iDOT), `PixelCalibration` (pCAL), `PaletteHistogram` (hIST),
   `SuggestedPalette` (sPLT) id-less in `PNG`. Measure ALL standard ancillary and
   critical chunk types (a grid, one chunk each; at least the ones above plus
   cHRM, sCAL, sTER, vpAg, acTL, fcTL, pHYs, cICP, mDCV, cLLI, dSIG, tIME) and add
   the id-less names to `PNG_DROPPED_NAMES` (measured, pinned by the unit test).
   Add one corpus input per such chunk (extras table) so the green tests actually
   exercise them.
3. **H2: cancelling counts.** One net `text_tags` count lets opposite differences
   cancel. Split the id-less count into THREE buckets, compared bucket by bucket:
   (a) normal text and `tIME` lines in group `PNG` (ours: `Text` and `Time`
   regions not after `IEND`); (b) `Trailer` group id-less lines (ours: `Text` and
   `Time` regions after `IEND`, from `png::read` regions' `after_iend()`, the
   harness already uses the library there); (c) the unparseable-chunk names
   `TextualData`, `InternationalText`, `CompressedText` (ours: the number of
   `BadTextChunk` issues from `png::read`; measure that this is the right
   equality, and say so). `Trailer` stops being dropped wholesale: its id-less
   lines go to bucket (b), and its other lines (entries and structural) follow the
   group rules (M3: after `IEND`, ExifTool lists `eXIf` entries and valid XMP
   under their own groups, so correct the `STRUCTURAL_GROUPS` doc comment and the
   cause text of `png-after-iend-in-trailer`; re-derive which rows still differ
   and why, and pin only those with correct causes). Add the three verify files as
   regression inputs: `png-cancel-after-iend-text-plus-badtext`,
   `png-cancel-gama-plus-after-iend-text`, `png-kw-xmp-plus-badtext`
   (generators under the scratchpad `v036/gen/`); each must now be reported (agree
   with ExifTool in every bucket, or a pinned exact difference).
4. **M2: keyword spelled like a dropped name.** A text keyword `XMP`, `Palette`,
   `ProfileName`, `ICC_Profile`, `eXIf`-like, `*_Profile`, `RawProfileTypeX` loses its
   tag to the by-name drop. It cannot be fixed by name alone. Add one corpus input per
   such keyword, pin the exact difference (`ours only: text tags x1` in bucket
   (a)) as ONE new divergence class
   `png-text-keyword-collides-with-chunk-tag-name` with a `questions.yaml` entry
   that says plainly it is a limit of the by-name classification, and a unit test
   that lists the collision names. Do not invent a cleverer rule.
5. **L1.** A test that the Class P rows have canonical, unique keys.
6. **L2.** Harness mutants in `tests/mutation_oracle.rs` as mutated-copy proofs
   (control first, one injection, `RED-PROOF INJECTION` marker, needle exactly
   once, marker must not leak): change the PNG first-OK-block rule to "first
   block", make WebP count only block 0, count `RawProfile` regions as text, remove
   `Palette` from the dropped list. Each fails the named tests; `just
   mutation-oracle` green.
7. **Gates on the pinned 1.90.0:** `just lint`, `just test`, `just mutation-oracle`,
   `just forbid-unsafe`, `just validate`, `just cost-audit`, `just oracle-record`
   twice (second run no diff; the five older inventory records byte-identical),
   and the fuzz smokes for the targets the `src/png` change touches
   (`just fuzz-smoke png_chunks`, `audit`, `audit_render`; 60 s each, nightly).
   Update the README coverage number if the distinct count changes. Commit on
   `claude/spec-036-fixes` with trailer
   `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`. No PR. Leave
   tokens null.

## Budget

About 100 tool calls or 250k context. If either is near, stop after the H1
checkpoint commit and report what is done; do not loop.

## Out of Scope

Strip work; changes to the audit beyond what H1 needs; value comparison; the
five old corpora and their records.

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
