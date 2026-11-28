---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-041
  type: story                      # epic | story | task | bug | chore
  cycle: design  # frame | design | build | verify | ship
  blocked: false
  priority: medium                 # critical | high | medium | low
  complexity: M                    # XS | S | M | L | XL | XXL — the EXPECTED size, set at design
                                   #   (XL/XXL almost certainly means it's a stage, not a spec)
  complexity_actual: null          # stamped at ship: what it ACTUALLY took, same scale.
                                   #   Expected-vs-actual drift is what `just calibration` reads.
  verify_verdict: null             # approved | punch-list | rejected — the OUTCOME of the verify
                                   #   cycle, stamped by `just advance-cycle` when the spec leaves
                                   #   verify (same three verdicts Prompt 4 already returns).
                                   #   Recorded in front-matter, not just prose, so "verify never
                                   #   rejects anything" stops being a hunch and becomes a number.
  verify_rounds: []                # every verify outcome, oldest first, appended by
                                   #   `just advance-cycle` each time the spec leaves verify.
                                   #   verify_verdict keeps only the LAST one, so a spec sent
                                   #   back and then approved read as "approved": irradiance's
                                   #   field showed 1 pushback where its handoffs showed 5.

project:
  id: PROJ-001
  stage: STAGE-007
repo:
  id: diamond-exif

handoff:
  from_agent: claude-opus-5-5  # from .repo-context tier_map.design (DEC-005)
  to_agent: claude-sonnet-5-5      # HANDOFF-094 (build, parts A and B)
  created_at: 2026-11-28

references:
  decisions: [DEC-001, DEC-013, DEC-014, DEC-015]
  constraints: [panic-free-library, no-io-in-library, fuzz-target-per-surface, forbid-unsafe-code, test-before-implementation, exiftool-never-linked]
  related_specs: [SPEC-025, SPEC-026, SPEC-027]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: [SPEC-025]

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-007's <capability>". Optional; null is acceptable.
value_link: "Delivers STAGE-007's byte-order conversion: a big-endian source becomes the little-endian block the consumer places, value by value, with byte-order-sensitive entries converted by a measured rule or left out and reported."

# Self-reported AI cost per cycle. Each cycle (design, build, verify,
# ship) appends one entry to sessions[]. Totals are computed at ship.
# Record a REAL tokens_total for metered cycles (build/verify) — the agent
# that runs the cycle writes it from its own interface (/cost, the API
# usage object, or its tool's report). Only un-metered main-loop cycles
# (design/ship) may be null-with-note. `just cost-audit` enforces this on
# shipped specs. See AGENTS.md §4 and docs/cost-tracking.md. interface:
# claude-code | claude-ai | api | ollama | other.
cost:
  # Optional PREDICTION of the total tokens this spec will take, set at
  # design. Never a gate — its only job is to be compared with the actual
  # below (`just calibration`), so you learn whether you systematically
  # under- or over-estimate. null = didn't predict.
  tokens_estimate: 900000           # two builder parts plus a verify and a fix round, like SPEC-025 (M, 1.10M actual); the library change is smaller, the generated table fixtures and the sweep are not
  sessions:
    - cycle: design
      agent: claude-opus-5-5
      interface: claude-code
      basis: null
      tokens_total: null
      estimated_usd: null
      duration_minutes: null
      recorded_at: 2026-11-28
      notes: "Main-loop design in the orchestrator session (d1fb0ea8): the real-source byte-order survey, the cross-order sweep (42,336 pairs, rerun after the session ended mid-sweep on 2026-11-27), the CFAPattern and UNICODE probes and the rule check (144 pairs), the DEC-014 amendment of 2026-11-28, the spec, HANDOFF-094. Not metered per cycle; it is counted in STAGE-007's orchestration_cost at stage close."
  totals:
    tokens_total: 0
    estimated_usd: 0
    session_count: 0
---

# SPEC-041: Exif block byte-order conversion: a target byte order, conversion by declared type, the measured `ByteOrder` omission table, the CFAPattern and UNICODE-text rules

## Context

Second spec of STAGE-007 (PROJ-001), split out of SPEC-025 at the 2026-11-27 re-frame. SPEC-025 (shipped 2026-11-27, PR #71) builds the block in the source's own byte order. This spec adds conversion into the other order. The consumer needs it: its block is always little-endian at irradiance's fixed base (DEC-015, irradiance/DEC-072), and "a big-endian source is converted value by value" (monorail-000's `docs/upstream/diamond-exif-requests.md`, request 1). Today a caller with a big-endian source has to pass no block.

The design measured ExifTool 13.55 before writing these criteria (see *Design measurements*). It amends DEC-014 (2026-11-28: the target option, conversion by declared type, the measured `ByteOrder` table, two conversion rules, the value-level self-check) and answers question `exif-block-byte-order-sensitive-undefined`. **The amendment is proposed; jysf accepts or changes it before the build starts.**

## Goal

`exif_block(source, &BlockOptions::new().with_byte_order(order))` returns a plan whose `write(base)` is in `order`, whatever the source's order. Its values read the same as the source's, through this crate's reader and through ExifTool. Entries whose meaning a conversion cannot keep are left out and reported as `ByteOrder`. Without the option, the output is byte for byte SPEC-025's. The readback self-check and the `exif_block` fuzz target cover the cross-order path.

## Design measurements (ExifTool 13.55, 2026-11-27 and 2026-11-28)

The scripts that took these are summarised here. The build turns the sweep into the gated live test in AC9(b).

- **Real sources cross.** Of 26 raw files on this machine (DNG, NEF, PEF), 3 are big-endian: an iPhone 12 Pro DNG, a second DNG (`M2462362.DNG`) and a Nikon NEF. The UNDEFINED tags real files carry in the three IFDs are 0x927C, 0x9000, 0xA300, 0x9101, 0xA000, 0xA302 CFAPattern (7 files, both orders; the NEF's is `0002 0002 00 01 01 02`, big-endian), 0xA301, 0x9286 UserComment (2 files, ASCII-coded) and GPS 0x001B GPSProcessingMethod (7 files, ASCII-coded).
- **Cross-order sweep.** Every id of ExifTool's `Exif::Main` (573, from `exiftool -listx -EXIF:All`) was placed in the Exif IFD and, separately, in the Interop IFD; every id of `GPS::Main` (32) was placed in the GPS IFD. Each was placed under all 12 declared types. One-byte types (1, 2, 6, 7) got 7 payload shapes: CFA-like dimensions, OECF-like, `UNICODE` + Latin UCS-2, `UNICODE` + CJK UCS-2, bare UTF-16 CJK, DSD-like, and 16 counting bytes. Multi-byte types got one 16-byte payload. Each big-endian source was compared with a little-endian copy whose value was swapped element by element per its declared type: 42,336 pairs, read with `exiftool -j -n -b -a -G1 -u`.
  - An undecoded blob ExifTool prints only under `-b` (for example ColorMap as SHORT) counts as equal when its length is equal, because `-n` without `-b` prints the same `(Binary data N bytes)` for both. SPEC-025's live judge reads without `-b`.
  - 798 pairs differ. The Exif and Interop sets are identical. Folded to tags and declared types, they are DEC-014's amendment item 3 table, plus the rule cases of items 4 and 5.
  - ExifTool hangs on 0x014A SubIFDs whose value is not a real IFD offset. It follows them as IFDs. The block always leaves 0x014A out as `OffsetCarrying`, so those 144 files were dropped; the live sweep must never give 0x014A to ExifTool.
- **CFAPattern (0xA302, UNDEFINED).** A big-endian source was read beside its copy with the bytes unchanged, and beside its copy with the two header SHORTs swapped. Seven header shapes were used: 2 × 2, 3 × 2, a header in the wrong order for its file, 1 × 256 (valid read either way), 3 × 3 over 4 bytes (malformed), 0 × 0, and 2 × 2 with trailing bytes. A 3-byte value was added.
  - With the header swapped, ExifTool reads all 8 the same as the source.
  - With the bytes unchanged, 1 × 256 and 3 × 3 read differently.
  - As BYTE (type 1), ExifTool reads a CFAPattern differently under any treatment, so it is left out.
- **UNICODE text.** UserComment (Exif and Interop), GPSProcessingMethod and GPSAreaInformation:
  - With the 16-bit units after the code swapped, CJK text reads the same.
  - Copied unchanged, it reads as other characters (`中文` → `ⵎ蝥`).
  - ExifTool accepts the code `UNICODE` followed by 0x00 or 0x20. `Unicode\0`, `UNICODEX`, ` UNICODE`, `UNICODE\t` and `UNICOD\0\0` print `Invalid EXIF text encoding` and read the same in either order.
  - ExifTool honours a byte-order mark, ignores an odd last byte, and copes with a code-only value and with a 7-byte value.
  - ASCII, JIS and all-zero codes read the same when copied, and differently when swapped.
  - The rule was then checked against every sweep shape plus those edges, under each type it applies to: 144 pairs, 0 differ.
- **OECF (0x8828), SpatialFrequencyResponse (0xA20C), DeviceSettingDescription (0xA40B).** ExifTool prints them as undecoded bytes, so it reads a plain copy the same and a structured conversion differently. It cannot judge a conversion. By the Exif specification each begins with two SHORTs in the file's order, so a plain copy is wrong for any other reader. They are left out.

## Inputs

- **Decisions.** DEC-014 with its amendments of 2026-11-27 and 2026-11-28 (the second is the rule book here), DEC-015 with its clarification, DEC-013 part 7 (the self-check pattern).
- **Library code:** `src/write/block.rs` (all of it; `classify`, `plan_table`, `encode`, `check_block`, the `fault` hook), `src/tiff/types.rs` (`type_size`), `src/reader.rs` (`ByteOrder`).
- **Tests to extend:** `tests/exif_block.rs`, `tests/support/exif_block_shapes.rs` (the grid; `grid()` already builds each shape in both orders), `tests/support/exif_block_model.rs`, `tests/support/exif_block_place.rs`, `tests/exif_block_live.rs`, `tests/exif_block_sweep_live.rs` (the pattern for the new sweep), `tests/support/sweep.rs`, `tests/support/tools.rs`.
- **Fuzz:** `fuzz/shared/exif_block_invariants.rs`, `fuzz/fuzz_targets/exif_block.rs`, `examples/fuzz_seeds.rs`, `tests/fuzz_layout.rs`.

## Outputs

- **Modified `src/write/block.rs`.** Public API additions, re-exported from `exif_core::write`:
  ```rust
  impl BlockOptions {
      /// The block's byte order. Unset: the source's (SPEC-025's output, unchanged).
      pub fn with_byte_order(self, order: ByteOrder) -> Self;
      /// `None` when unset.
      pub fn byte_order(&self) -> Option<ByteOrder>;
  }
  impl<'a> ExifBlock<'a> {
      pub fn byte_order(&self) -> ByteOrder;        // now: the block's order (the target, or the source's)
      pub fn source_byte_order(&self) -> ByteOrder; // new
  }
  #[non_exhaustive] pub enum OmitReason {
      MakerNote, OffsetCarrying, UnknownType,
      /// The entry's meaning depends on the byte order and no measured conversion keeps it
      /// (DEC-014 amendment of 2026-11-28, item 3). Only when the orders differ.
      ByteOrder,
  }
  ```
  - `Display` for `ByteOrder`: "its contents depend on the byte order".
  - Internals: each kept entry holds its source payload (`&'a [u8]`) and its block payload (`Cow<'a, [u8]>`, borrowed when the orders match). The converter is one function over (declared type, tag, IFD, bytes, orders). The `ByteOrder` table is a `const` of (tag, type mask) rows for the Exif and Interop IFDs and one for the GPS IFD, with named masks for "one-byte", "multi-byte" and "all". `check_block` gains the value comparison of amendment item 6: a decoder that is not the converter.
- **New `tests/exif_block_order_sweep_live.rs`** (AC9(b)) and **extended** `tests/exif_block_live.rs` (AC9(a)).
- **Extended** test support: shapes, model (an independent cross-order predictor and its own typed copy of the table), mirrors test.
- **Fuzz:** bit 0 of the base word picks the other byte order (the base is already made even with `& !1`, so existing seeds keep their base); new invariants; new seeds.
- **Docs:** rustdoc on every new item; the README's Exif block section gains a cross-order example; CHANGELOG `[Unreleased] → Added`.

## Acceptance Criteria

Each AC maps to tests named in *Failing Tests*. "Cross order" means the block's order differs from the source's; "both directions" means a big-endian source to little-endian, and a little-endian source to big-endian.

- [ ] **AC1 API.** The items above exist with those signatures and doc comments, re-exported from `exif_core::write`. `BlockOptions::default().byte_order() == None`. `ExifBlock::byte_order()` is the target when set, else the source's; `source_byte_order()` is always the source's. No panic, no allocation before the budget check, on any path.
- [ ] **AC2 Same order is unchanged.** For every shape of the grid, in both source orders, `exif_block` with default options, and with `with_byte_order(<the source's order>)`, give the same `len`, positions, omissions and `write(base)` bytes as SPEC-025 does today, at bases 8 and 1000. Every existing SPEC-025 test passes unchanged; none is edited to pass. `ByteOrder` never appears in a same-order plan.
- [ ] **AC3 Conversion by declared type (amendment item 2).** In cross order, both directions:
  - SHORT and SSHORT are swapped per 2 bytes; LONG, SLONG and FLOAT per 4; RATIONAL and SRATIONAL per 4-byte half; DOUBLE per 8. BYTE, ASCII, SBYTE and UNDEFINED (outside items 4 and 5) are copied.
  - Inline and out-of-line values are both covered, including count 0, count 1 and odd counts of one-byte types.
  - 0xA005 holds `base + interop_position` in the target order, and every table count, tag, type, count and offset field is in the target order.
  - `len()` and the positions equal the same-order plan's when no entry is left out as `ByteOrder`.
  - The independent model (`tests/support/exif_block_model.rs`) predicts the block byte for byte for every non-refusing shape in both directions, at bases 8 and 1000. It imports nothing from `src/write` and has its own swap code.
- [ ] **AC4 `ByteOrder` omissions (amendment item 3).**
  - For every row of the table, in each IFD it names, the tag is left out as `ByteOrder` under every type in its mask, in cross order. For the Exif and Interop rows that means both of those IFDs. It is kept, converted, under every one of the 12 types outside its mask.
  - The same entries are kept in the same order, under every type.
  - The fixtures are generated: every (row, IFD, type), about 1,070 sources, in both directions. No ExifTool is needed.
  - Precedence: `UnknownType`, then `MakerNote`, then `OffsetCarrying`, then `ByteOrder`. Fixtures: 0x8828 as type 0 is `UnknownType`; 0x9286 as type 13 is `OffsetCarrying`.
  - `omissions()` keeps its order (Exif, Interop, GPS; ascending tag) and carries the source's field type.
  - Mirrors of the table:
    - `src/write/block.rs`;
    - `tests/support/exif_block_model.rs`;
    - the sweep test's expected table (`tests/support/exif_block_sweep_tables.rs` or a sibling);
    - `fuzz/shared/exif_block_invariants.rs`, only if it predicts reasons.

    The existing `omission_table_mirrors_agree` (or a sibling test) checks that they name the same (IFD, tag, type) set (signal `hand-typed-mirrors-of-a-decision-table-drift`).
- [ ] **AC5 CFAPattern (amendment item 4).** 0xA302 as type 7, in the Exif and in the Interop IFD, in cross order and both directions. The block's value is the source's with its first two 16-bit words written in the target order and the rest copied, for each design shape:
  - 2 × 2;
  - 3 × 2;
  - the wrong-order header;
  - 1 × 256;
  - 3 × 3 over 4 bytes;
  - 0 × 0;
  - 2 × 2 plus trailing bytes.

  A value of 3 bytes, of 1 byte, or of count 0 is copied. In the same order, every one of them is copied.
- [ ] **AC6 UNICODE text (amendment item 5).** For 0x9286 as types 1, 2, 6 and 7 in the Exif and the Interop IFD, and 0x001B and 0x001C as type 7 in the GPS IFD, in cross order and both directions:
  - **Converted** (every 16-bit unit after the 8-byte code swapped; an odd last byte copied):
    - codes `UNICODE\0` and `UNICODE `;
    - CJK text, with and without a byte-order mark;
    - an odd length;
    - code only.
  - **Copied:**
    - codes `Unicode\0`, `UNICODEX`, ` UNICODE`, `UNICODE\t`, `UNICOD\0\0`, `ASCII\0\0\0`, `JIS\0\0\0\0\0` and eight zeros;
    - a value under 8 bytes.
  - **Unchanged by the rule:** 0x001B as type 2 is copied, because the rule is type 7 only in GPS, and 0x9286 as a multi-byte type is `ByteOrder`.
- [ ] **AC7 Self-check in cross order (amendment item 6).**
  - `write` reads its own cross-order output back. Each kept entry's value, decoded from the block in the block's order, must equal the source's value decoded in the source's order. The decoder must not call the converter: it reads each element as a number and, for items 4 and 5, the header words and text units as numbers.
  - The slack checks of SPEC-025 apply unchanged.
  - Unit tests feed `check_block` a corrupted cross-order block and expect `VerifyFailed { check: Entries }` for each of these:
    - a SHORT left in the source's order;
    - one byte of a converted LONG flipped;
    - a RATIONAL with its halves swapped;
    - a CFAPattern header left unswapped;
    - UNICODE units left unswapped;
    - an ASCII-coded UserComment swapped.
  - The `#[cfg(test)]` fault hook in a cross-order `write` gives `VerifyFailed`.
- [ ] **AC8 Readback, round trip and determinism.** For every non-refusing shape, both directions, at bases in {8, 1000, 65534}, the following hold.
  - The test placer's file, in the block's order, parses with no issue, and its entries equal the source's by tag, type and count. Their values are equal after conversion, minus exactly `omissions()`. This is checked by the test's own decoder.
  - Rebuilding the placed file in its own order gives the same bytes and no omissions.
  - Rebuilding it back into the source's order, placing that, and building once more into the target order gives the first block again.
  - Two runs give equal bytes, and `write` equals `write_into`.
- [ ] **AC9 Live ExifTool**, gated by `DIAMOND_EXIF_REQUIRE_ORACLE`. Without ExifTool it prints `SKIP` and passes unless required. ExifTool runs on the builder's **output** during build. Nothing is recorded or red-proofed here; that is SPEC-026. ExifTool is never given `-m`.
  - **(a) `tests/exif_block_live.rs`:** every non-refusing shape is also built in cross order, both directions, and placed at bases 8 and 1000.
    - ExifTool reads the placed file as it reads the source for `-ExifIFD:All -GPS:All -InteropIFD:All` (`-a -G1 -s -n -H`), minus `omissions()`, and prints no warning the source did not.
    - SPEC-025's 7 pinned differences apply unchanged.
    - A new pin is allowed only with its measured cause, written in the pin and in the handback. A pin must never cover a value that reads differently because of conversion.
  - **(b) New `tests/exif_block_order_sweep_live.rs`:** reruns the design's cross-order sweep from `exiftool -listx`. Tags the block leaves out as `OffsetCarrying` or `MakerNote` are excluded; 0x014A is never given to ExifTool. Every run carries an 8 s alarm.
    - It computes the sensitive set S: the (IFD, tag, type) triples whose element-swapped copy ExifTool reads differently, judged as in the design (undecoded blobs equal by length).
    - It asserts that S minus the rule cases is a subset of the table, so a new ExifTool that reads one more tag by its own format fails this test.
    - It asserts that the table minus S is exactly 0x8828, 0xA20C and 0xA40B, under every type, in the Exif and Interop IFDs, so no other entry is left out needlessly.
    - For the rule cases, the builder's own cross-order output, placed, reads the same as the source.
- [ ] **AC10 Fixture grid (hostile shapes, AGENTS.md §15)**, added to `tests/support/exif_block_shapes.rs` and run in both directions:
  1. A design sample with every declared type 1 to 12, inline and out of line, in all three IFDs.
  2. One fixture per `ByteOrder` row, in its usual type (kept) and in a masked type (left out).
  3. The CFAPattern shapes of AC5.
  4. The UNICODE shapes of AC6.
  5. OECF, SFR and DSD with well-formed contents.
  6. The precedence fixtures of AC4.
  7. A GPS-only source, an Exif-only source, and an empty Exif IFD.
  8. A JPEG's Exif TIFF blob.
  9. `tiff_shapes::dng(be)`.
  10. The SPEC-025 refusal shapes, which still refuse in cross order with the same reason.

  Every SPEC-025 shape also runs in cross order.
- [ ] **AC11 Fuzz target `exif_block`.**
  - Bit 0 of the 2-byte base word selects the other byte order. The base is still made even.
  - Invariants, in `fuzz/shared/exif_block_invariants.rs`, without calling `src/write/block.rs` internals:
    - the plan reports the order asked for;
    - the placed file is in that order and parses clean;
    - its entries equal the source's, decoded each in its own order, minus `omissions()`, where the harness's own decoder applies items 4 and 5;
    - `ByteOrder` appears only in cross order;
    - the round trip of AC8 holds: same-order rebuild identical; there and back identical.
  - New seeds cover AC10 items 1 to 5 in both directions. Their pins are in `tests/fuzz_layout.rs`, and `tests/fuzz_layout.rs` shows each new guard red with its bug planted.
  - `just fuzz-smoke exif_block` runs 60 s clean.
- [ ] **AC12 Docs and gates.**
  - The README example uses `with_byte_order(ByteOrder::Little)` on a big-endian source and builds in a scratch crate.
  - There is a CHANGELOG entry.
  - Gates pass on toolchain 1.90.0: `just lint`, `lint-no-allow`, `lint-fuzz`, `forbid-unsafe`, `deny`, `deny-fuzz`, `DIAMOND_EXIF_REQUIRE_ORACLE=1 just test` with 0 skips, `mutation-oracle`, `validate`, and `cargo fmt --check` on both crates.

## Failing Tests

Written red first (in their own commit) by the build, from these names. Each asserts what its AC says.

- **`src/write/block_tests.rs`**
  - `options_default_has_no_byte_order`, `options_with_byte_order_round_trips` — AC1.
  - `convert_by_type_swaps_each_element` (all 12 types, inline and out of line) — AC3.
  - `byte_order_reason_only_in_cross_order`, `byte_order_precedence_after_offset_carrying` — AC4.
  - `cfa_header_swapped_rest_copied`, `cfa_under_four_bytes_copied` — AC5.
  - `unicode_units_swapped_after_code`, `unicode_codes_measured`, `unicode_odd_last_byte_copied` — AC6.
  - `check_block_cross_order_rejects_each_corruption` (the six of AC7), `fault_hook_cross_order_fails_verify` — AC7.
- **`tests/exif_block.rs`**
  - `same_order_bytes_unchanged_for_every_shape` — AC2.
  - `cross_order_bytes_match_model_both_directions` — AC3.
  - `byte_order_table_every_row_and_type` (generated fixtures), `byte_order_mirrors_agree` (or the extended `omission_table_mirrors_agree`) — AC4.
  - `cfa_pattern_rule_shapes`, `unicode_text_rule_shapes` — AC5, AC6.
  - `cross_order_readback_and_round_trip`, `cross_order_deterministic` — AC8.
  - `cross_order_refusals_unchanged` — AC10 item 10.
- **`tests/exif_block_live.rs`** — `placed_cross_order_block_reads_as_source_in_exiftool` — AC9(a).
- **`tests/exif_block_order_sweep_live.rs`** — `cross_order_sweep_matches_byte_order_table` — AC9(b).
- **`tests/fuzz_layout.rs`** — the new seed pins, and the new guards shown red — AC11.

## Non-Goals

- No conversion rule beyond items 4 and 5. A tag on the table that a consumer needs in its unusual type is a later amendment with its own measurement.
- No ExifTool parity record and no red-proofs (SPEC-026). No real camera fixtures in the repo (SPEC-026 has DNG and TIFF fixtures). No MakerNote, policy presets (SPEC-027), IFD0 or JPEG container handling.
- No change to the strip or its offset table (question `strip-offset-tags-incomplete` stays open).

## Notes for the Implementer

- **Two parts** (HANDOFF-094). Part A: the library, unit tests, model, shapes, mirrors, AC1 to AC8 and AC10. Part B: AC9 live tests, AC11 fuzz, AC12 docs. Part B starts from Part A's pushed branch.
- **Run ExifTool on outputs during the build** (AC9), not only in verify. macOS has no `timeout`: use `perl -e 'alarm 8; exec @ARGV' -- exiftool ...`. Give 0x014A to ExifTool never (it hangs). Batch files into one ExifTool run per few hundred, with an alarm per batch, and fall back to per-file runs for a batch that times out.
- **The table is data, typed once in the library.** Name the masks; do not spread type lists through `classify`. The other mirrors are typed again by hand on purpose; the mirrors test is what keeps them equal.
- **Keep the same-order path byte-identical.** Borrow the payload when the orders match; AC2 is the regression net. Do not change SPEC-025's layout code.
- **Toolchain:** pinned 1.90.0 (`RUSTUP_TOOLCHAIN=1.90.0`). Run `just lint-fuzz` too. Restore `fuzz/Cargo.lock` after fuzz runs.
- **Cost:** leave `tokens_total` null in the handback; the orchestrator records the harness figure. Budget about 100 tool calls or 250k context per agent.

---


## Reflection

*Appended during **ship**. Three questions, short answers.*

1. **What would I do differently next time?**
   — <answer>

2. **Does any template, constraint, or decision need updating?**
   — <answer — if yes but not done this session, record it in
   `/guidance/signals.yaml`: `type: lesson` (with its N-count) for a recurring
   coding pattern, `type: process-debt` for tooling/process friction. A close
   then forces the decision. See `docs/signals.md`.>

3. **Is there a follow-up spec I should write now before I forget?**
   — <answer>

4. **Where was the worst defect caught?** — one word from a fixed vocabulary so
   the defect-escape distribution is greppable across specs:
   `design` | `build` | `verify` | `ship` | `escaped` (reached prod/runtime) |
   `none` (clean first try).
   — <one word>
   *(Runtime/operational defects — the escape-prone class — only exist once the
   artifact meets its real host. `escaped` here is a signal to strengthen the
   §12 behavioral pre-flight for that surface.)*

5. **What can a user do now that they couldn't before?** — one sentence,
   before → after; quote the confirming number if one exists, name the outcome
   if not. Write `none` if this spec has no user-visible outcome — that is a
   real, greppable result, not a blank. This is the line a downstream work-log's
   `impact` field is transcribed from, and both halves are already written above
   (## Context is the before, ## Goal is the after): confirm the prediction,
   don't reconstruct it from memory.
   — <answer | none>
