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
  id: HANDOFF-074
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

# HANDOFF-074: Tag-level inventory parity for PNG and WebP, PART B (decisions, tests, divergences, red-proofs)

**FIRST STEP, before anything else:** `git checkout -b claude/spec-036-part-b claude/spec-036-design`
(the design branch already contains Part A, commit `0b32b9c`, and its cost commit;
the harness may have started your worktree from main, which does not).

## Delegation Summary

`claude-opus-5-5` hands `SPEC-036` Part B to `claude-sonnet-5-5`: refine the
classifier with the decisions below, pin every difference, write the replay, live
and red-proof tests, the questions and the docs. Read HANDOFF-073's Handback
first (Part A's measured table and raw differences are in the Part A report
summarised below). Test harness only: no `src/` change; if a difference turns
out to be a real audit bug, stop and report it, do not fix or pin it silently.

## Context

Read: `AGENTS.md`, `guidance/toolchain-brief.md`, the spec
`projects/PROJ-001-diamond-exif-m1/specs/SPEC-036-*.md`, DEC-020,
`tests/support/inventory.rs` and `inventory_corpus.rs` (Part A's changes), the
SPEC-019 `real-shapes` section of `tests/inventory_oracle.rs`, and
`tests/mutation_oracle.rs`.

Toolchain: `export RUSTUP_TOOLCHAIN=1.90.0 PATH="$HOME/.cargo/bin:$PATH"` (lint on
the pinned 1.90.0); macOS has no `timeout` (`perl -e 'alarm 8; exec @ARGV' --`);
BSD `sed -i` needs `''`; Python for multi-line edits. Never commit
`decisions/INDEX.md` or `fuzz/Cargo.lock`. No CHANGELOG entry. Do not leave
background processes running.

Part A measured: corpus `png-webp`, 555 inputs, 301 distinct, 67 distinct differ
before any pinning. Groups seen: `PNG` (hex ids = IHDR; id `-` = MANY chunk
kinds, not only text and `tIME`), `RIFF` (hex and `-` for `-u` unknown chunks),
`Trailer`, Exif groups, `XMP`/`XMP-xmp`, `ICC-header[N]`/`ICC_Profile[N]`.

## Decisions (the architect decides these; implement, do not re-decide)

1. **`PNG` id-less lines are classified by tag NAME.** The Part A rule (count every
   one into `text_tags`) is wrong: ExifTool also prints id-less `[PNG]` lines for
   `iCCP` (`ProfileName`, `ICC_Profile`), `PLTE` (`Palette`), an unparseable
   `eXIf` (`eXIf`), raw profiles (`EXIF_Profile`, `XMP_Profile`,
   `RawProfileType*`, ...) and XMP in text (`XMP`). These are dropped BY NAME
   (exact names plus the prefix `RawProfileType` and the suffix `_Profile`,
   as constants listed in the file and pinned by a unit test); every other id-less
   `PNG` line is a text or `tIME` tag and is counted into `text_tags`. The list of
   dropped names is MEASURED from the record (Part A's table), not guessed: any
   name you cannot place is a finding to report. Update the Part A unit tests.
2. **PNG first-block rule.** ExifTool reads the first `eXIf` block that is a
   valid TIFF and stops (`png-exif-fatal-then-good` reads the 2nd;
   `png-shape-meta-exif-two` stops at a valid first block with no entries). The
   audit's items cannot tell which block parsed, so the caller (the test and the
   recorder-side check, not `inventory.rs`) computes the first block whose
   `container()` is `Ok` from `exif_core::png::read(bytes)` and passes it to the
   PNG model of `ours` (a new `ours_png(audit, first_ok_block)` or an extra
   parameter; keep `inventory.rs` free of other support modules: using
   `exif_core` is fine). WebP and JPEG unchanged. Test both fixtures above plus a
   file with no parseable block.
3. **After-`IEND` text.** ExifTool puts chunks after `IEND` in group `Trailer`;
   the audit lists them as `Text`/`Time` regions (DEC-020). Keep `Trailer`
   structural and pin the difference as a divergence (`png-after-iend-in-trailer`,
   new `questions.yaml` id with the measured cause); do not change the audit.
4. **Pin every remaining difference** as a `KNOWN_DIVERGENCES` entry: an exact
   multiset difference (tags, regions, and the `text_tags` counts: extend
   `Divergence` with the two text-count fields, `Divergence::pinned()` currently
   sets both to 0), a measured cause and a `guidance/questions.yaml` id (reuse
   existing ids where they fit: `png-raw-profile-not-decoded`,
   `webp-exif-block-cap`, `webp-iccp-profile-not-validated`,
   `webp-xmp-content-not-parsed`, `webp-null-chunk-not-a-stop`,
   `webp-riff-other-forms-read-by-exiftool`, `png-creation-time-keyword-case`,
   `png-xmp-flag-2-located-not-read`; add new ones for new causes: bad text chunk
   not listed, unparseable `eXIf` blob, `iCCP` located with no ExifTool group,
   after-IEND text). A difference you cannot explain stops you: report it.

## Part B, in order

1. Apply decisions 1 and 2; re-run the corpus replay to get the new raw
   difference list; report it (counts by cause) before pinning.
2. Tests in `tests/inventory_oracle.rs`, SPEC-036 section: corpus pins;
   `png_webp_replay_is_clean_or_pinned`; `png_webp_live_parity` (gated like its
   siblings, `DIAMOND_EXIF_REQUIRE_ORACLE`); the PNG first-block and WebP
   every-block rules; the WebP 17th-block cap pin; text/time count agreement;
   unknown group is a parse error; every divergence has a question entry; a
   divergence that stops being one fails; the dropped-name list is pinned.
3. Hostile edges: make sure the corpus covers the spec's variant grid; where a
   shape is missing (e.g. `tIME` lengths 0 to 12, raw-profile exif with GPS,
   `eXIf` before and after `IDAT`, XMP in `iTXt`/`tEXt`/`zTXt`/compressed), add it
   to the fixture tables in `oracle_png.rs` / `oracle_webp.rs` (the one table per
   format), re-record, and pin. Probe each new shape under the 8 s limit.
4. Red-proofs: replay-and-perturb on `png-webp.txt` (names the perturbed
   `(group, id)` or the count); at least three mutated-copy proofs in
   `tests/mutation_oracle.rs` (e.g. a `Text` region counted twice, `Time` items
   dropped, the PNG first-block rule removed): control first, one injection,
   needle exactly once, `RED-PROOF INJECTION` marker, marker must not leak.
   `just mutation-oracle` green.
5. Docs: `guidance/questions.yaml` entries; a README sentence only if it states
   inventory coverage ("inventory parity against ExifTool covers N distinct
   inputs": update N and add that PNG and WebP are covered).
6. Gates on the pinned 1.90.0: `just lint`, `just test`, `just mutation-oracle`,
   `just forbid-unsafe`, `just validate`, `just cost-audit`, `just oracle-record`
   twice (no diff), and the five older inventory records byte-identical.
   Tick the remaining boxes. Commit on `claude/spec-036-part-b` with trailer
   `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`. No PR. Leave
   tokens null in the handback.

## Budget

About 100 tool calls or 250k context. Stop and hand back what is done if either
is near; do not loop.

## Out of Scope

`src/`; value comparison; the five old corpora; the strip specs.

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
