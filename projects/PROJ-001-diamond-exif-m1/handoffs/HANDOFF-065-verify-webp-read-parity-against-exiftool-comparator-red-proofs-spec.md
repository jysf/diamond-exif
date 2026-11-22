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
  id: HANDOFF-065
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
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

# HANDOFF-065: Verify SPEC-034 WebP read parity against ExifTool

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-034` to a cold verifier. You did not write this. Review branch `claude/spec-034-webp-parity` (build commits `dfcd915`, `545a366`, `1663d81`, handback `22dad3c`, cost commit after) against the spec (`projects/PROJ-001-diamond-exif-m1/specs/SPEC-034-webp-read-parity-against-exiftool-comparator-red-proofs-spec.md`, the file without `-timeline`). This is an evidence spec: decide whether the evidence is real and the red-proofs bite. Review only; edit nothing except your own handback in this file. Do not push.

## FIRST STEP

Run `git checkout -b claude/spec-034-verify claude/spec-034-webp-parity` (local branch; do not `reset --hard`) and confirm HEAD contains `1663d81`.

## BUDGET (hard)

About 80 tool calls or 200k context. Record `tokens_total: null` with a note if you cannot measure it (the orchestrator replaces it with the harness figure). Toolchain: `RUSTUP_TOOLCHAIN=1.90.0`, `PATH="$HOME/.cargo/bin:$PATH"`; read `guidance/toolchain-brief.md`. ExifTool 13.55 is installed (run it, never read its source); use `perl -e 'alarm 8; exec @ARGV' -- exiftool ...` for ad hoc runs.

## What to look hard at

1. **Is the evidence real?** Re-run ExifTool yourself on at least 12 committed fixtures (write the bytes out through a scratch example or the harness, run `exiftool -T -n -s3` with the recorded selectors) and compare to the committed `webp-*.txt` records byte for byte; check headers carry the real command, version and stderr; `just oracle-record` idempotent.
2. **Fixture independence.** `tests/support/oracle_webp.rs` shares no encoding code with `src/`, `examples/fuzz_seeds.rs`, `fuzz/shared/*`, `tests/support/webp_shapes.rs`. Imports of `oracle_tiff` and `oracle_png::icc_profile` are allowed (oracle layer); check nothing else.
3. **The several-`EXIF` model** (later block wins, JPEG-style; PNG is first-only). Re-measure on at least five variants beyond the grid: three blocks, a middle garbage block, a prefixed block last, a later block that lacks a tag the earlier has, a later block with the same tag but a different type. Confirm `read_ours_webp` and ExifTool agree. Judge whether the library's `exif()` ("first block is primary") versus this model is a hazard for SPEC-032 (the audit must see values a reader sees) and say so plainly; do not fix it.
4. **Comparator honesty.** The XMP selector is `XMP-xmp:CreatorTool` against a non-empty `Xmp` region and ICC is `ICC-header:ProfileFileSignature` against a non-empty `Icc` region. Could a located-but-wrong region pass? In PNG verify this blind spot (comparing presence only, not which chunk) cost a round: check that a second XMP chunk, an XMP that ExifTool reads from a different chunk than the one we call primary, or region bytes/offsets, are checked (the build has `located_region_bytes_equal_what_the_builders_wrote`: does it cover every region fixture, and can it fail?).
5. **Divergences.** Each of the three has the measured stdout, a right-side sentence and a question id; re-measure two. Confirm the test requiring each divergence to be PRESENT really fails when one disappears. The builder says ExifTool also reads `RIFF....AVI ` (beyond the spec) and is silent on 12 trailing bytes (the design said it warns): confirm both by measurement.
6. **Red-proofs.** Run `just mutation-oracle`. For the eight mutants and two byte mutations: each control green first, marker `RED-PROOF INJECTION`, needle exactly once, leak test works. Then add your OWN mutants on a scratch copy (rsync the tree elsewhere; never mutate the branch): at least five library faults the builder did not use (for example: the `MissingPad` condition, `NonCanonicalChunkId` routing, `MAX_WEBP_EXIF_BLOCKS` cap, a zero-length region filter, the `ANMF` pass-over, the `RiffSizeMismatch` check, treating 8 bytes left as clean) and report which turn the WebP comparison red and which survive (equivalent, or a fixture gap).
7. **Shape coverage (lesson from SPEC-029 to SPEC-031: unmeasured variants cost fix rounds).** Probe ExifTool on WebP variants the fixture list does NOT contain and report agreement with `webp::read` on located metadata: for example `VP8X` between `EXIF` chunks, `EXIF` of exactly 8 bytes, a big-endian TIFF (`MM`), `XMP ` with a UTF-16 packet or a BOM, an `EXIF` of exactly the right size with a stray pad byte, 17 `EXIF` chunks (our cap is 16), `EXIF` inside `ANMF` followed by an odd chunk, `ICCP` followed by `EXIF` with the ICC flag unset, a 12-byte file with a chunk header only. Disagreements on located metadata are Medium; on warnings only, Low.
8. **Collateral changes.** The `.filter` skips for `webp-*` in `jpeg_oracle`, `budget_oracle`, `inventory_oracle` (and PNG tests) must weaken nothing (they still cover every non-WebP, non-PNG record as before); `app.just` is sound; no change under `src/`; the seed additions and `tests/fuzz_layout.rs` pins are right (53 seeds, 37 walk seeds); the three carried-gap tests actually kill the mutants named in the spec (try the last-`VP8X`-wins mutant and the `BadVp8x` threshold-9 mutant on a scratch copy).
9. Run `just lint`, `just test`, `just mutation-oracle`, `just validate`, `just cost-audit`, `just forbid-unsafe` yourself.

## Expected Deliverables

Your verdict, exactly one of APPROVED / PUNCH LIST / REJECTED, with findings ranked High, Medium, Low, each with file:line and a concrete failing input. Fill the `handback:` front matter (`verdict`) and the `## Handback` section; set statuses to `completed`; commit only the handback.

## Out of Scope

Fixing anything, DEC edits, audit wiring (SPEC-032).

---

## Handback

### Execution notes

- **Branch / PR:**
- **Completed at:**
- **Verdict:**

### Cost self-report

- **Tokens (total):**
- **Source of the number:**

### Findings

-

### Reflection

1. **What was unclear in the spec or handoff?**
   —
