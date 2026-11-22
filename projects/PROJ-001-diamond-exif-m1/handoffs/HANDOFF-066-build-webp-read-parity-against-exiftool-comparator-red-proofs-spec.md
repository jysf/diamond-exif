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
  id: HANDOFF-066
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-21
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-034

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

# HANDOFF-066: Fix SPEC-034 verify punch list (narrow)

## Delegation Summary

`claude-opus-5-5` (architect) hands the punch list of the SPEC-034 verify to a fresh implementer. Spec: `projects/PROJ-001-diamond-exif-m1/specs/SPEC-034-webp-read-parity-against-exiftool-comparator-red-proofs-spec.md` (not the `-timeline` file). The full findings are in the `## Handback` of `projects/PROJ-001-diamond-exif-m1/handoffs/HANDOFF-065-verify-webp-read-parity-against-exiftool-comparator-red-proofs-spec.md`: read that section only, plus the code it points at. You are fixing the ORACLE (tests, fixtures, records, model), not `src/`, except where stated. Existing mirror: how SPEC-031 did the same for PNG (`tests/png_oracle.rs` `located_region_bytes_equal_what_the_builders_wrote`, `oracle_png::expected_regions`).

## FIRST STEP

`git checkout -b claude/spec-034-fixes claude/spec-034-verify` (local branch; do not `reset --hard`); confirm `git log --oneline -1` shows `94c6eaa` or later.

## BUDGET (hard)

About 60 tool calls or 150k context, one checkpoint commit per finding group. Record `tokens_total: null` with a note (the orchestrator replaces it). Commit on your branch, trailer `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`; do not push, no PR, no CHANGELOG entry, no new dependency. Toolchain: `RUSTUP_TOOLCHAIN=1.90.0`, `PATH="$HOME/.cargo/bin:$PATH"`; `guidance/toolchain-brief.md`. ExifTool via `perl -e 'alarm 8; exec @ARGV' -- exiftool ...` or the repo helpers. `fuzz/Cargo.lock` must not be committed; never commit `decisions/INDEX.md`.

## Work (measure with ExifTool BEFORE modelling; the verifier's numbers are leads)

1. **H1 (High): the several-`EXIF` model is per selector.** ExifTool keeps the FIRST block's value for `IFD0:ImageWidth`, `ImageHeight`, `Orientation`, `XResolution` and the LAST for the other 16 (verifier's measurement). Re-measure all 20 selectors with two full blocks that differ in every selector (both orders); put the result in `read_ours_webp` as an explicit, documented per-selector rule (a list of first-wins selectors, derived from the measurement and pinned by a test that reads every one of the 20 selectors in both orders). Do NOT change `read_ours_jpeg` or the JPEG oracle (the same gap probably exists there; report it in your handback with whatever evidence you gather, do not fix it).
2. **M1 (Medium): the `MAX_WEBP_EXIF_BLOCKS = 16` cap.** Add fixtures: 16 `EXIF` blocks (exact cap) and 16 `tiff_a` blocks then one carrying GPS (`exif-gps-full-le`). ExifTool reads the 17th; we drop it. Pin the GPS-selector differences as a `KNOWN_DIVERGENCES` entry with a `guidance/questions.yaml` id (`webp-exif-block-cap`): say plainly that ours is the cap policy, that the audit must report blocks past the cap (decoy blocks can hide GPS), and which side is right (neither: a bounded-work policy). No `src/` change.
3. **M2 (Medium): region comparator checks presence only.** Add an `expected_regions()`-style table in `oracle_webp.rs` (kind, offset, bytes, written independently by the builders) for EVERY fixture that has an `XMP `, `XMP\0` or `ICCP` chunk, and a test that our located regions equal it exactly (count, order, kind, offset, bytes, empty ones included), failing for any fixture missing from the table that has such a chunk. Add fixtures `webp-xmp-two-distinct` (two packets with different `CreatorTool` values), `webp-xmp-three` and `webp-icc-three` so several-region cases are distinguishable. Then, on a SCRATCH COPY (never mutate the branch), prove with two mutants that the new test goes red: "only the first `ICCP` is located" and "only the first `XMP ` is located"; say so in the handback (the mutation oracle file need not change).
4. **M3 (Medium): empty null chunk.** Measure and add `webp-null-chunk-before-exif` (8 zero bytes before `EXIF`, RIFF size fixed) and `webp-null-chunk-tail` (8 zero bytes at the end). ExifTool aborts with "Encountered empty null chunk. Processing aborted"; we read on. Pin as a divergence (`webp-null-chunk-not-a-stop`; we over-read, the safe side for an audit, DEC-019 unchanged) and in the warning table.
5. **Low:** (L1) document in a comment on `read_ours_webp` that a later block whose selector is a type `read_one` cannot read (Make as SHORT) falls back to the earlier block while ExifTool prints the value: add a fixture `webp-exif-make-short-later` only if it produces a mismatch you can pin as a divergence under `webp-exif-nonascii-make`; otherwise just the comment. (L2) add the verifier's warning-only disagreements to the warning table (8 zero bytes tail, stray pad byte after an even chunk, 20-byte header plus chunk head, XMP with UTF-8 BOM, an 8-byte TIFF) after measuring each. (L3) leave seeds on the 20 Exif selectors, but amend the SPEC-034 Goal and the `tests/webp_oracle.rs` header comment to say seeds are compared on the Exif selectors only.
6. Re-record (`just oracle-record`, twice, no diff), then gates: `just lint`, `just test`, `just mutation-oracle`, `just validate`, `just cost-audit`, `just forbid-unsafe`. The `webp_oracle` record-set test must still equal fixtures plus seeds.

## Out of scope

`src/` changes (raise any reader bug instead), JPEG or PNG oracle changes, DEC edits, audit wiring.

## Return Criteria

Fill the `handback:` front matter and `## Handback` (branch, commits, per-finding outcome, new divergences with ids, JPEG-gap evidence, gate results); statuses `completed`. If blocked, say exactly where.

---

## Handback

### Execution notes

- **Branch / PR:**
- **Completed at:**

### Cost self-report

- **Tokens (total):**
- **Source of the number:**

### Findings handled

-

### Reflection

1. **What was unclear?**
   —
