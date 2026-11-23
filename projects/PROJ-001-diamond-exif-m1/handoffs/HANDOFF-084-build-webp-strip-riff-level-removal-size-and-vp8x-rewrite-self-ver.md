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
  id: HANDOFF-084
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-22
  status: pending                  # pending | accepted | completed | rejected

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

# HANDOFF-084: SPEC-038 verify punch list (fix round)

**FIRST STEP, before anything else:** `git checkout -b claude/spec-038-fixes claude/spec-038-verify`
(the harness may have started your worktree from main).

## Delegation Summary

`claude-opus-5-5` hands the SPEC-038 verify punch list to `claude-sonnet-5-5` for one fix
round. Read first: `AGENTS.md`, `guidance/toolchain-brief.md`, DEC-020, DEC-021, DEC-022
(with their clarifications), `HANDOFF-083-*.md` (the verify report: evidence, ExifTool
output and the grid; scratch scripts under the scratchpad `v038-verify/` if still present:
`grid.py`, `nested.py`, `discover.py`, `discover2.py`, `check2.py`), `src/webp/metadata.rs`
(`chunk_class`), `src/write/webp.rs`, `tests/strip_webp.rs`, `tests/strip_webp_live.rs`,
`tests/support/webp_carriers.rs`, `fuzz/shared/rewrite_webp_invariants.rs`.

Toolchain: `export RUSTUP_TOOLCHAIN=1.90.0 PATH="$HOME/.cargo/bin:$PATH"` (lint on the pinned
1.90.0; also `just lint-fuzz`); fuzz needs nightly; macOS has no `timeout`
(`perl -e 'alarm 8; exec @ARGV' --`); BSD `sed -i` needs `''`; Python for multi-line edits.
Never commit `decisions/INDEX.md`; restore `fuzz/Cargo.lock` after fuzz or lint-fuzz runs. No
CHANGELOG entry. Leave no background processes (stop every monitor or sleep before you
report). Compare against `origin/main`. Records must not contain machine paths.

## The punch list, in order. The architect decides these; do not re-decide.

1. **H1 (blocks shipping): a chunk coded `RIFF` inside a WebP.** ExifTool 13.55 walks into
   a top-level chunk coded `RIFF` as a container (any form type, any depth, before or after
   the image, and a second `RIFF....WEBP` appended after the first file, which our walker
   reads as a chunk beyond the declared size) and prints the EXIF, XMP, ICCP, C2PA, LIST,
   IDIT, JUNK, ID3 inside. Our strip keeps it under every preset, `Everything` included, and
   the audit says `ok`. First re-measure to confirm and to nail the rule: which spellings
   (`RIFF` only; verifier says `RF64`, `RIFX`, lower-case `riff` and a `RIFF` chunk with no
   form type are NOT walked), nesting depth, the 16 case spellings, a `RIFF` chunk with a
   too-short or lying size. Then: a chunk coded exactly `RIFF` is an opaque carrier
   (DEC-022 clause 4): located as a `Carrier` region by `chunk_class`, removed by every preset
   (the carrier rule), a non-empty one counts as a failed block (the file reads
   `unreadable`), regardless of its form type. Make this its own commit first. Add it to
   DEC-022's measured table, the harness table, the live rows, the seeds (a nested one, an
   appended-second-RIFF one, a depth-3 one) and the plants; correct DEC-020's SPEC-038
   clarification; report every test, seed or record this moves. Add the verifier's two
   minimal cases as tests.
2. **L3 + the root cause of H1: executable completeness of the carrier table.** Add a gated
   live test (`tests/strip_webp_live.rs` or a sibling) that sweeps every FourCC candidate the
   verifier used (every 4-character tag id from `exiftool -listx` plus the RIFF/AVI/WAV/WebP
   codes, plus the codes in your table, with several payload shapes: text, a TIFF, an XMP
   packet, an ID3 header, a JUMBF box, zeros, a nested RIFF) as one chunk in a bare WebP,
   runs ExifTool on the bare file, and FAILS for any code that prints a tag in a personal
   group or any non-structural tag unless the code is in the measured table or in the pinned
   not-metadata list. Pin the not-metadata list with the structure tags it prints. Keep it
   within a few minutes (batched, threads). This turns "complete against the builders' own
   list" into "complete against ExifTool's own tag-id list". If the sweep finds a code that
   is not in the table, report it and add it (carrier rule) with its row.
3. **M2: the live judge checks too little.** `removed_by` in `tests/strip_webp_live.rs`
   checks only GPS, four serials and five authorship tags. Make the judge the complete
   preset table (DEC-009): per preset, every category's tags removed (Make/Model
   `CameraModel`, `Software`, timestamps, free text, `Thumbnail`, `MakerNote`, XMP under
   `All`/`Everything`, ICC and C2PA under `Everything`) AND the tags a preset keeps still
   printed (so a strip that over-removes silently is a failure, except the carrier rule's
   documented over-removal). State the lists in the test, pinned.
4. **M1: the WebP preview wording.** `src/report/text.rs` `preview_lines` still says "the
   writer refuses this container" for WebP. Say the WebP strip exists but has not been
   checked against ExifTool yet (as for PNG, SPEC-037 L3); `strippable` stays false;
   update the golden(s) deliberately and say which changed; a test pins the WebP wording.
5. **L1: `VP8X` flags with several `VP8X` chunks or a short one.** ExifTool reports the
   LAST `VP8X`'s flags; the writer patched only the first 10-byte one. First measure what
   ExifTool does with: two `VP8X`, an 8-byte `VP8X`, a 4-byte one, and a `VP8X` after the
   image. Then clear the matching bits in EVERY `VP8X` chunk whose data holds the flag byte
   (as ExifTool reads it; the verifier saw ExifTool print flags from an 8-byte one) so a
   stripped file never prints a flag for a chunk it no longer has. Update DEC-022 clause 7,
   the harness flag rule and its plants; the size rule is unchanged.
6. **L4 and L2:** no code change. Add one sentence to DEC-022 stating that RIFF `WAVE`,
   `AVI `, `RIFX` and lower-case `webp` files are refused as `NotJpeg` (an old error name)
   and that an odd chunk without its pad followed by more chunks is refused as
   `ChunkPastEnd`.
7. **Orchestrator note for SPEC-039:** the completeness sweep of item 2 has a PNG twin
   (every PNG chunk type ExifTool decodes); do NOT build it here, but list in your handback
   the three cheapest steps to make it, so SPEC-039 can include it.
8. **Gates on the pinned 1.90.0:** `just lint`, `just lint-fuzz`, `just test`, `just
   mutation-oracle`, `just forbid-unsafe`, `just validate`, `just cost-audit`, `just
   oracle-record` twice (no diff), the live tests with `DIAMOND_EXIF_REQUIRE_ORACLE=1`, and
   the 60 s smokes `rewrite_webp`, `webp_riff`, `audit`, `audit_render`. Commit on
   `claude/spec-038-fixes` with trailer
   `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`. No PR. Leave tokens null.

## Budget

About 100 tool calls or 250k context. If either is near, stop after the H1 commit and report
what is done; do not loop.

## Out of Scope

PNG changes; the audit/CLI switch-over (`strippable` stays false); the formal ExifTool write
parity (SPEC-039); a CLI strip command.

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
