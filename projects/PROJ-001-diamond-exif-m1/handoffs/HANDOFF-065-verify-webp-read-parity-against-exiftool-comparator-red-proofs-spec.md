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
  status: completed                # pending | accepted | completed | rejected

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
  status: completed                # completed | blocked | rejected
  agent: claude-opus-5-5           # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: punch-list              # verify handoffs: approved | punch-list | rejected
  basis: estimate                  # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: null               # REAL combined count — what cost-audit reads
  estimated_usd: null              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: 60
  branch: claude/spec-034-verify
  pr: null
  completed_at: 2026-11-21         # YYYY-MM-DD
  notes: "tokens_total is null: the verifier cannot read its own transcript token counts; the orchestrator replaces it with the harness figure. About 45 tool calls. Review only; nothing pushed."
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

- **Branch / PR:** `claude/spec-034-verify` (local, from `claude/spec-034-webp-parity` at `36462af`, contains `1663d81`); no PR, not pushed.
- **Completed at:** 2026-11-21
- **Verdict:** PUNCH LIST

The evidence is real. ExifTool 13.55 re-run on all 98 fixture records and all 53 seed records gives byte-identical stdout and the same `# stderr:` lines. `just oracle-record` leaves no diff. The builder's red-proofs bite. But the several-`EXIF` model is wrong for 4 of the 20 selectors, and three located-metadata shapes outside the grid disagree without a pinned divergence. All the probes and mutants ran on an rsync copy in scratch. The branch is unmodified apart from this handback.

### Cost self-report

- **Tokens (total):** null (not measurable from inside the session)
- **Source of the number:** none; the orchestrator replaces it with the harness figure.

### Findings

**High**

- **H1. The several-`EXIF` model ("later block wins per tag") is wrong for `IFD0:ImageWidth`, `IFD0:ImageHeight`, `IFD0:Orientation` and `IFD0:XResolution`.** ExifTool 13.55 keeps the FIRST block's value for these four. The other 16 selectors follow the later block. `read_ours_webp` (`tests/support/oracle.rs:1220`, `.rev()` at 1226) applies later-wins to all 20. `several_exif_model_is_measured_and_pinned` (`tests/webp_oracle.rs:742`) measures only Make and Model, so the claim in the doc comment and in the spec's acceptance criterion is a generalisation that was never measured. Failing input: `RIFF`, `VP8X`(0x08), `VP8L`, then `EXIF`(IFD0 ImageWidth=100 SHORT, ImageLength=50, Orientation=1, XResolution=72/1, plus the other tags) and `EXIF`(the same with 200, 100, 8, 144/1). Measured with `exiftool -T -n -s3`: `100 50 MakeTwo ModelTwo 1 72 SoftTwo ...`. The model gives `200 100 MakeTwo ModelTwo 2 144 ...`. With the blocks reversed, ExifTool gives `200 100 MakeOne ... 2 144` and the model gives `100 50 ... 1 72`. Fix: a per-selector priority in the model, a full two-block fixture in both orders, and the test extended to all 20 selectors. `read_ours_jpeg` uses the same `.rev()` rule (`tests/support/oracle.rs:785`) and probably has the same gap; I did not measure JPEG.

**Medium**

- **M1. A 17th or later `EXIF` chunk is read by ExifTool and dropped by us.** The cause is `MAX_WEBP_EXIF_BLOCKS = 16` (`src/webp/metadata.rs:39`, `:242`). It is not a pinned divergence. Failing input: `file(0x08, [EXIF(tiff_a) x16, EXIF(exif-gps-full-le)])`. ExifTool prints `4000 3000 DiamondCam ... 37.4252083333333 122.096916666667`; we print `AlphaCam OnlyInA` and no GPS. The grid has no fixture with more than two distinct blocks (`webp-seed-meta-exif-seventeen` reads nothing on either side). So cap mutants `< 2` and `< 15` are killed only by `exif_blocks_capped_at_sixteen` and the fuzz-model tests, and the ExifTool comparison is blind to them. For SPEC-032 this is a privacy-evasion shape: 16 decoy blocks hide GPS from the audit while a reader shows it.
- **M2. The region comparator is presence-only, so a located-but-wrong ICC region passes.** Scratch mutant m11 (`ChunkKind::Iccp` guarded so only the FIRST `ICCP` is located) SURVIVES the whole WebP suite (`webp_oracle`, `webp_walk`, `webp_metadata`, `fuzz_layout`). Failing input: `webp-icc-garbage-then-valid`. We locate only the garbage chunk, which reads "present", and ExifTool reads `acsp` from the second. `located_region_bytes_equal_what_the_builders_wrote` (`tests/webp_oracle.rs:1272`) covers only `webp-all-three`, `webp-xmp-nul-id` and `webp-exif-prefix`, never a multi-region fixture. Mutant m12 ("only the first non-empty XMP is located") is killed only by the unit test `two_xmp_both_located` (`tests/webp_metadata.rs:186`), not by the comparison, because the two packets in `webp-xmp-two` are identical. ExifTool reads every XMP chunk: with `-a`, two packets give `FirstTool` and `SecondTool`. Fix: region count, kinds and offsets in the comparison or the bytes test for every region fixture, and a two-XMP fixture with different CreatorTool values. This is the same blind spot as the PNG round.
- **M3. An empty null chunk (`\0\0\0\0`, size 0) before `EXIF` makes ExifTool stop, and we still read the EXIF.** Failing input: `webp-exif-basic` with 8 zero bytes inserted at offset 44 (RIFF size fixed up). ExifTool warns "Encountered empty null chunk. Processing aborted" and prints no Make. We read `AlphaCam OnlyInA` with no issue. A null code with size 2, or `JUNK` with size 0, agrees. We over-read here (the safe side for an audit), but the shape is unmeasured and has no divergence entry.

**Low**

- **L1.** When a later block holds the same tag with a different type, ExifTool shows the later value and the model falls back to the earlier block. Failing input: EXIF(tiff_a) then EXIF(Make as SHORT 7). ExifTool prints `7`; the model prints `AlphaCam`, because `read_one` returns None and `find_map` goes on to the earlier block (`tests/support/oracle.rs:1227`).
- **L2. Warning-only disagreements (not pinned).** 8 zero bytes after the last chunk: ExifTool "Encountered empty null chunk. Processing aborted"; ours only `RiffSizeMismatch`. An even `EXIF` plus a stray byte, then `XMP `: ExifTool silent, ours `Truncated`; both lose the XMP. A 20-byte file `RIFF` + `EXIF` with size 0: ExifTool silent, ours `RiffSizeMismatch,MetadataWithoutVp8x`. XMP with a UTF-8 BOM: "Superfluous BOM at start of XMP". UTF-16LE XMP: "XMP is double UTF-encoded". A TIFF of exactly 8 bytes: "Bad IFD0 directory". Ours has no WebP-layer issue for the last three.
- **L3.** Seeds are compared on the 20 Exif selectors only (`tests/webp_oracle.rs:141-150`, `examples/oracle_record.rs` `record_webp`), not on XMP and ICC presence. The spec's Goal names both. This mirrors PNG; state it or extend it.

**Checked and clean**

- **Evidence.** 98 of 98 fixture records and 53 of 53 seed records are byte-identical to a fresh ExifTool run, including stderr. The headers carry the command and version 13.55. Oracle-record is idempotent.
- **Independence.** `tests/support/oracle_webp.rs` imports only `oracle_png` (`xmp_packet`, `icc_profile`) and `oracle_tiff`.
- **Model probes that agree with ExifTool:**
  - three blocks
  - a garbage block in the middle
  - a prefixed block last, and a prefixed block first
  - a later block lacking a tag the earlier one has
  - LE then BE blocks
  - GPS only in the second block
- **Item-7 shapes that agree on located metadata:**
  - `VP8X` between two `EXIF` chunks
  - an 8-byte `EXIF`
  - an `MM` TIFF
  - a UTF-8 BOM packet and a UTF-16LE packet (both read by ExifTool, both located by us)
  - a stray pad byte
  - `EXIF` inside `ANMF` followed by an odd chunk, and an `ANMF` with an odd inner chunk
  - `ICCP` then `EXIF` with the ICC flag unset
  - a header plus a chunk header of size 0 or 10
  - 16 blocks with the last one differing
  - 8 `A` bytes as the tail
- **The `exif()` hazard for SPEC-032 is real.** `exif()` is the first parsed block. ExifTool shows the later block for 16 of the 20 selectors, and the union of the blocks. On EXIF(tiff_a) then EXIF(gps-full), `exif()` has no GPS and ExifTool prints the GPS. The audit must walk `exif_blocks()` with the per-tag priority from H1 and report unparsed blocks (M1). It must never use `exif()` alone.
- **Divergences.** I re-measured `webp-icc-garbage` (no signature; "Bad length ICC_Profile (length 1768121121)") and `webp-xmp-nul-id-3-bytes` ("Incorrect XMP tag ID", "Empty XMP"). `RIFF....AVI ` is read (FileType AVI, Make WaveCam) and `WAVE` reads as WAV. 12 trailing bytes are silent and 3 trailing bytes warn, so the builder is right and the design text was wrong. `a_divergence_that_stops_being_one_fails` flips each entry to stale.
- **Red-proofs.** `just mutation-oracle` passes: 40 tests in the WebP file, the controls first, needles exactly once, the leak test included. My 14 scratch library mutants (m1 to m13, with b variants; table below) all die except m11 (M2). How each dies:

  | Killed by | Mutants |
  |---|---|
  | The ExifTool replay or live comparison | m5 and m5b (`ANMF` walked from 0 or from 16) |
  | The warning table only | m1 `MissingPad` never, m2 `XMP\0` not non-canonical, m6 `RiffSizeMismatch` never, m7 short tail clean |
  | `empty_regions_never_count_as_present` | m4 and m4b (empty regions dropped), m7b (8 bytes left halts) |
  | `located_region_bytes` | m13 (XMP offset +8) |
  | Unit and fuzz-model tests only, never the comparison | m3 and m3b (EXIF cap 2 and 15), m10 (`ExtraExif` never), m12 (only the first non-empty XMP located) |

- **Carried gaps.** Mutant m8 (last `VP8X` wins) is killed by `first_vp8x_wins_flags_and_one_flag_mismatch`. Mutant m9 (`BadVp8x` threshold 9) is killed by `webp_model_runs_on_every_walk_seed` (the new seed) and `vp8x_decode_short_and_misplaced`.
- **Collateral.** No change under `src/`, `fuzz/fuzz_targets`, `fuzz/shared` or `examples/fuzz_seeds.rs`. The `webp-` filters in `jpeg_oracle`, `budget_oracle` and `inventory_oracle` only exclude new files. `app.just` runs the third file. The pins are 53 seeds and 37 walk seeds.
- **Gates.** `just lint`, `just test` (1197 passed, 0 failed, 14 ignored), `just mutation-oracle`, `just validate`, `just cost-audit` and `just forbid-unsafe` all exit 0.

### Reflection

1. **What was unclear in the spec or handoff?**
   — The spec says the several-block rule is "later wins per tag", from a Make/Model measurement. ExifTool's per-tag priority is invisible to a two-tag fixture, so the next model spec should require a full 20-selector two-block fixture in both orders before it states a rule. Item 7's "17 `EXIF` chunks" needs a distinct last block to mean anything, which the seed lacks.
