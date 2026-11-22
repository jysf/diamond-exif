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
  agent: claude-sonnet-5-5         # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: null                    # verify handoffs: approved | punch-list | rejected
  basis: estimate
  tokens_total: 179076              # harness subagent_tokens
  estimated_usd: 1.18
  duration_minutes: null
  branch: claude/spec-034-fixes
  pr: null
  completed_at: 2026-11-22         # YYYY-MM-DD
  notes: "tokens_total null: the subagent session exposes no token count; the orchestrator replaces it with the harness figure. Not measured: duration."
  synced_at: 2026-11-22
---

# HANDOFF-066: Fix SPEC-034 verify punch list (narrow)

## Delegation Summary

`claude-opus-5-5` (architect) hands the punch list of the SPEC-034 verify to a fresh implementer. Spec: `projects/PROJ-001-diamond-exif-m1/specs/SPEC-034-webp-read-parity-against-exiftool-comparator-red-proofs-spec.md` (not the `-timeline` file). The full findings are in the `## Handback

### Execution notes

- **Branch / PR:** `claude/spec-034-fixes` (local, from `claude/spec-034-verify` at `5be377d`); no PR, not pushed. Commits: the fix commit (tests, records, questions, model), a spec-text commit, and this handback commit.
- **Completed at:** 2026-11-22

### Cost self-report

- **Tokens (total):** null
- **Source of the number:** none; the subagent session exposes no token count. The orchestrator replaces it.

### Findings handled

- **H1 (fixed).** Measured on `webp-exif-full-one-two` and `webp-exif-full-two-one` (two full blocks, every one of the 20 selectors different, both orders; builder `oracle_webp::tiff_full(v)`). ExifTool keeps the FIRST block for `IFD0:ImageWidth`, `IFD0:ImageHeight`, `IFD0:Orientation`, `IFD0:XResolution` and the LAST for the other 16, exactly as the verifier measured. `read_ours_webp` now has an explicit list `oracle::WEBP_FIRST_BLOCK_WINS` (those four). `several_exif_model_is_per_selector_and_pinned` reads all 20 selectors in both orders against the record, derives the first-wins set from the records and asserts it equals the list. SPEC-034 text amended (Goal: seeds on Exif selectors only; several-block rule is per selector; test name). `read_ours_jpeg` and the JPEG oracle untouched.
- **M1 (fixed, divergence pinned).** `webp-exif-blocks-16` (16 distinct Makes, exactly the cap): both read `Block15`, no divergence. `webp-exif-16-then-gps`: 16 `tiff_a` blocks then a 17th that holds ONLY GPS (a deviation from the handoff, which named `exif-gps-full-le`: that full TIFF differs on 16 selectors and the comparator has no wildcard, so each would need its own entry; the GPS-only block isolates the two GPS selectors). ExifTool prints GPSLatitude 37.4252083333333, GPSLongitude 122.096916666667; we read none. Pinned as two `KNOWN_DIVERGENCES` entries, id `webp-exif-block-cap` (questions.yaml, with the privacy consequence for SPEC-032 and "neither side is right, bounded-work policy"). `exif_block_cap_is_measured_against_exiftool` asserts both sides and the 16-block count.
- **M2 (fixed).** `oracle_webp::expected_regions()` (code, offset, bytes; offsets are running sums of builder lengths) plus `has_region_chunk_code()`; `located_regions_equal_the_independent_table_for_every_region_fixture` compares count, order, kind, offset, bytes (empty ones included) for every fixture and fails for a fixture that has an `XMP `, `XMP\0` or `ICCP` code and no table row (43 regions across 44 rows). New fixtures `webp-xmp-two-distinct`, `webp-xmp-three`, `webp-icc-three`. ExifTool reads the LAST XMP (`SecondTool`, `ThirdTool`) and the ICC of the last `ICCP` shape. Mutation proof on a scratch rsync copy (branch not touched): mutant A ("only the first `ICCP` is located", guard in `src/webp/metadata.rs`) FAILS the new test at `webp-icc-valid-then-garbage`; mutant B ("only the first `XMP ` is located") FAILS it at `webp-xmp-two`. The old test `located_region_bytes_equal_what_the_builders_wrote` is kept.
- **M3 (fixed, divergence pinned).** `webp-null-chunk-before-exif`: ExifTool warns "Encountered empty null chunk. Processing aborted" and prints no Make/Model; we read AlphaCam/OnlyInA. Pinned as two entries (Make, Model), id `webp-null-chunk-not-a-stop` (we over-read, safe side, DEC-019 unchanged), and in the warning table. `webp-null-chunk-tail` (8 zero bytes after the EXIF, RIFF size exact): both read the EXIF, ExifTool still warns; table row only.
- **L1 (fixed as divergence).** `webp-exif-make-short-later` (EXIF with Make AlphaCam, then EXIF with Make as SHORT 7): ExifTool prints `7`, the model reads `AlphaCam`. It does produce a mismatch, so it is pinned as `webp-exif-nonascii-make` (an id that did not exist in questions.yaml; I created it). Documented on `read_ours_webp`.
- **L2 (done).** Warning table rows added (measured): `webp-null-chunk-before-exif` and `-tail` (null chunk warning; ours no issue), `webp-stray-pad-then-xmp` (ExifTool silent; ours `Truncated`), `webp-header-20-exif-size-0` (ExifTool silent; ours `MetadataWithoutVp8x`, block fatal; the file here has an exact RIFF size so there is no `RiffSizeMismatch`, unlike the verifier's probe), `webp-xmp-utf8-bom` ("Superfluous BOM at start of XMP"), `webp-xmp-utf16le` ("XMP is double UTF-encoded"), `webp-exif-tiff-8-bytes` ("Bad IFD0 directory"; ours fatal block). Both XMP packets are read by ExifTool and located by us (the regions table covers them).
- **L3 (done).** Seeds are compared on the Exif selectors only: stated in the `tests/webp_oracle.rs` header and the SPEC-034 Goal.

### New divergences

- `webp-exif-block-cap` (webp-exif-16-then-gps, Composite:GPSLatitude and Composite:GPSLongitude).
- `webp-null-chunk-not-a-stop` (webp-null-chunk-before-exif, IFD0:Make and IFD0:Model).
- `webp-exif-nonascii-make` (webp-exif-make-short-later, IFD0:Make).

### JPEG gap (reported, not fixed)

Same gap, measured: a JPEG with two APP1 `Exif` segments built from the same two full TIFFs (SOI, APP1(full1), APP1(full2), EOI; both orders; ExifTool 13.55, the repo's 20-selector command). Order one-two: `100 50 MakeTwo ModelTwo 1 72 SoftTwo ...` (first block for width, height, orientation, xres; later for the other 16). Order two-one: `200 100 MakeOne ModelOne 8 144 SoftOne ...` (width, height, orientation, xres from the FIRST block, which is full2; Make, Model and the other 12 from the LAST block, full1). So `read_ours_jpeg`'s `.rev()` rule (later wins on all 20) is wrong for the same four selectors. The scratch files are `jg/jpeg-one-two.jpg` and `jg/jpeg-two-one.jpg` in the session scratchpad (not committed).

### Gate results

`just lint` exit 0 (after rustfmt and a clippy `next_back` fix); `just test` exit 0 (1201 passed, 0 failed, 14 ignored); `just mutation-oracle`, `just validate`, `just cost-audit`, `just forbid-unsafe` exit 0. `just oracle-record` run twice after the last fixture change: no diff on the second run. The `webp_oracle` record-set test (fixtures plus seeds) passes with 15 new fixtures (113 fixture records, 53 seed records). No `src/` change, no CHANGELOG entry, `fuzz/Cargo.lock` and `decisions/INDEX.md` not committed.

### Reflection

1. **What was unclear?**
   — The handoff said to use `exif-gps-full-le` as the 17th block, which cannot be pinned with exact-selector entries (16 selectors differ). I used a GPS-only 17th block and said so. `webp-exif-nonascii-make` had no questions.yaml entry; created.
