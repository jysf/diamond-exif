---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-025
  type: story                      # epic | story | task | bug | chore
  cycle: build  # frame | design | build | verify | ship
  blocked: false                   # irradiance/DEC-072 accepted as amended (optional Exif offset), checked 2026-11-27
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
  to_agent: claude-sonnet-5-5      # HANDOFF-092 (build, parts A and B)
  created_at: 2026-11-27

references:
  decisions: [DEC-001, DEC-005, DEC-013, DEC-014, DEC-015]
  constraints: [panic-free-library, no-io-in-library, no-new-top-level-deps-without-decision, fuzz-target-per-surface, forbid-unsafe-code, test-before-implementation, exiftool-never-linked]
  related_specs: [SPEC-022, SPEC-026, SPEC-027, SPEC-041]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: []

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-007's <capability>". Optional; null is acceptable.
value_link: "Delivers STAGE-007's block builder: the self-checked Exif, GPS and Interop block at a caller-given base, in the source's byte order, with omissions reported."

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
  tokens_estimate: 1000000          # two builder parts plus a verify and a fix round; STAGE-010's L specs (two parts each) landed at 1.0M-1.4M, so an M is predicted at the low end
  sessions:
    - cycle: design
      agent: claude-opus-5-5
      interface: claude-code
      basis: null
      tokens_total: null
      estimated_usd: null
      duration_minutes: null
      recorded_at: 2026-11-27
      notes: "Main-loop design in the orchestrator session (d1fb0ea8): the ExifTool 13.55 offset-carrying sweep (1,178 sources, writer and reader passes), the unknown-type, count-0 and next-word probes, the layout probe at bases 8 and 1000, the DEC-014 amendment and DEC-015 clarification, the spec, HANDOFF-092. Not metered per cycle; it is counted in STAGE-007's orchestration_cost at stage close."

    - cycle: build
      handoff: HANDOFF-092
      agent: claude-sonnet-5-5
      interface: other
      basis: agent-result
      tokens_total: 564052
      estimated_usd: null
      duration_minutes: 54
      recorded_at: 2026-11-27
      notes: "Parts A and B, one subagent each (claude-sonnet-5-5). tokens_total and duration are the sums of the harness subagent results, filled by the orchestrator: A 274,448 tokens / 68 tool calls / 1,318 s; B 289,604 tokens / 151 tool calls / 1,913 s. The orchestrator's own fix (b0b5130, the sweep without -m) is main-loop work, counted in STAGE-007's orchestration cost."
  totals:
    tokens_total: 564052
    estimated_usd: 0.00
    session_count: 2
---

# SPEC-025: Exif block builder in the source's byte order: DEC-014 and DEC-015 contract, size query and write at a base, MakerNote and offset-carrying entries left out and reported, budget, readback self-check, `exif_block` fuzz target

## Context

First spec of STAGE-007 (PROJ-001). So far the writer only removes bytes in place (DEC-013). This spec adds the first operation that **builds** IFDs: the relocatable Exif block of DEC-014, written for a caller-given base under DEC-015's offset contract. monorail-000 asked for it (its `docs/upstream/diamond-exif-requests.md`, request 1; consumer decision monorail-000/DEC-075). irradiance/STAGE-017 shipped the placing side on 2026-11-25: its SPEC-081 copies a block to file offset 8 and writes the two IFD0 pointers. This builder is the producer it is waiting for.

At the 2026-11-27 re-frame this spec was split by byte order. It builds the block in the **source's** byte order only. SPEC-041 adds conversion into the other order. SPEC-026 adds recorded, red-proofed ExifTool parity. SPEC-027 adds policy presets.

The design measured ExifTool 13.55 before writing these criteria (see *Design measurements* below). It amends DEC-014 (2026-11-27: byte-order staging, the measured omission list, which walk issues refuse, layout) and clarifies DEC-015 (how the self-check reads the block at its base). **DEC-014 was accepted as amended by jysf on 2026-11-27.** irradiance/DEC-072 is accepted with the optional Exif offset, so GPS-only blocks are in scope.

## Goal

`exif_block(source, &options)` returns, for a TIFF-structured source, a plan that reports the block's length, the Exif and GPS IFD positions and every entry left out, or a typed refusal. `plan.write(base)` returns exactly those bytes for that even base, in the source's byte order, after reading them back through the crate's own walk and checking them against the source.

## Design measurements (ExifTool 13.55, 2026-11-27)

The scripts that took these are summarised here. The build turns the sweep into the gated live test in AC9.

- **Offset-carrying sweep.** Every id in ExifTool's `Exif::Main` table (573, from `exiftool -listx -EXIF:All`) was placed in the Exif IFD and, separately, in the Interop IFD. Every id in `GPS::Main` (32) was placed in the GPS IFD. Each was a LONG pointing at a mini-IFD 4 KiB away, in a little-endian TIFF: 1,178 sources.
  - *Writer side* (`exiftool -o out/ -Artist=sweep`): 22 sources were refused with `No size tag for <IFD>:<Name>`. Those are 11 tags in each of the two IFDs: 0x0111, 0x0120, 0x0144, 0x0201, 0x0207, 0x0208, 0x0209, 0x8781, 0xA010, 0xBCC0, 0xBCC2. 0x014A came back with a changed value in both IFDs. All 1,153 other values came back unchanged. One source (Exif IFD 0x013B Artist) lost its Exif IFD, because `-Artist` moved the tag to IFD0. That is not an offset.
    *Corrected at build:* the design ran the writer pass with `-m`, which DEC-023 rules out. Without it, 26 sources are refused: the 22 `No size tag` refusals plus four `[minor]` refusals about the swept value (0xC612 `DNG Version 4096 not yet tested`, 0x927C `Maker notes could not be parsed`, each in the Exif and Interop IFDs). The offset set does not change. The live sweep runs without `-m`.
  - *Reader side* (the pointed-at bytes zeroed, as a moved value would leave them; `exiftool -j -a -G1 -s -n -u`): only 0x014A changed the output, in both IFDs (`SubIFD:Software` disappeared). Nothing changed in the GPS IFD.
- **Unknown types.** An Exif-IFD entry of type 14, 15, 16, 17, 18 or 255 prints `Warning: Bad format (N) for ExifIFD entry 0` and no value. Type 0 prints nothing.
- **Count 0.** ASCII, SHORT and RATIONAL entries with count 0 print the tag with an empty value, so they are kept.
- **Exif IFD next-word.** A source whose Exif IFD chains to a valid IFD prints none of the chained IFD. The walk records the next-word and never follows it (DEC-005), so writing 0 loses nothing.
- **Layout.** The DEC-014 layout (tables Exif, Interop, GPS, then values) was placed at bases 8 and 1000 in a minimal TIFF with IFD0 after the block. It reads exactly as the conventional source does (`-a -G1 -s -n -Warning -ExifIFD:All -GPS:All -InteropIFD:All`), in both byte orders. A GPS-only block reads cleanly, and `-validate` adds no warning that the source did not already have.

## Inputs

- **Decisions.** DEC-014 (with its amendment of 2026-11-27, which is the rule book here), DEC-015 (with its clarification of 2026-11-27), DEC-005, DEC-001, DEC-013 part 7 (the self-check pattern).
- **Library code:**
  - `src/tiff/mod.rs`: `Container::parse`, `ifds`, `Ifd::parent`, `kind`, `entries`, `issues`, `payload`, `entry_budget`.
  - `src/tiff/issue.rs`: `ParseIssue`, `Link`.
  - `src/tiff/types.rs`: `type_size` (today `pub(super)`; widen it to `pub(crate)` and reuse it, do not copy it).
  - `src/reader.rs`: `ByteOrder`.
  - `src/write/exif.rs`: `POINTER_TAGS`, `OFFSET_TAGS`, `EXIFTOOL_SUBDIRECTORY_TAGS` and `EXIFTOOL_BLOB_CARRIER_TAGS`. Share them, do not re-type them.
  - `src/write/mod.rs`: error and record conventions, `#[non_exhaustive]`.
- **Tests and fixtures to follow:**
  - `tests/support/exif_layout.rs` and `tests/support/tiff_shapes.rs` (source builders: `B`, `dng(be)`, `hostile()`).
  - `tests/strip_webp_live.rs` and `tests/strip_webp_sweep_live.rs` (gated live-test pattern).
  - `tests/support/tools.rs` (`run_exiftool_full`, `warning_lines`, `exiftool_available`).
- **Fuzz pattern to follow:**
  - `fuzz/fuzz_targets/rewrite_webp.rs` and `fuzz/shared/rewrite_webp_invariants.rs`;
  - `examples/fuzz_seeds.rs`;
  - `tests/fuzz_layout.rs` (seed pins and the target-list checks);
  - the `.github/workflows/ci.yml` fuzz-smoke matrix;
  - `app.just`.

## Outputs

- **New `src/write/block.rs`** (with `#[cfg(test)]` unit tests, or `block_tests.rs` beside it as `exif_tests.rs` is). Public API, re-exported from `exif_core::write`:
  ```rust
  pub const DEFAULT_MAX_BLOCK_LEN: u32 = 1 << 24; // 16 MiB

  pub fn exif_block<'a>(data: &'a [u8], options: &BlockOptions) -> Result<ExifBlock<'a>, BlockError>;

  #[non_exhaustive] #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct BlockOptions { /* private */ }
  impl Default for BlockOptions { /* max_len = DEFAULT_MAX_BLOCK_LEN */ }
  impl BlockOptions {
      pub fn new() -> Self;
      pub fn with_max_len(self, max_len: u32) -> Self;
      pub fn max_len(&self) -> u32;
  }

  #[non_exhaustive] #[derive(Debug, Clone)]
  pub struct ExifBlock<'a> { /* private: source, kept entries, layout */ }
  impl<'a> ExifBlock<'a> {
      pub fn len(&self) -> u32;                  // even; 0 when the source has neither IFD
      pub fn is_empty(&self) -> bool;
      pub fn exif_position(&self) -> Option<u32>; // block-relative, even
      pub fn gps_position(&self) -> Option<u32>;  // block-relative, even
      pub fn byte_order(&self) -> ByteOrder;      // the source's (DEC-014 amendment item 1)
      pub fn omissions(&self) -> &[Omission];     // Exif, Interop, GPS order, ascending tag
      pub fn write(&self, base: u32) -> Result<Vec<u8>, BlockError>;
      pub fn write_into(&self, base: u32, out: &mut [u8]) -> Result<(), BlockError>;
  }

  #[non_exhaustive] #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub struct Omission { /* private */ }
  impl Omission {
      pub fn ifd(&self) -> BlockIfd;
      pub fn tag(&self) -> u16;
      pub fn field_type(&self) -> u16;
      pub fn reason(&self) -> OmitReason;
  }

  #[non_exhaustive] pub enum BlockIfd { Exif, Interop, Gps }
  #[non_exhaustive] pub enum OmitReason { MakerNote, OffsetCarrying, UnknownType }
  // ByteOrder (SPEC-041) and Policy (SPEC-027) are added when used.

  #[non_exhaustive]
  pub enum BlockError {
      Source(crate::Error),                              // Container::parse refused: bad header, BigTIFF, unreadable IFD0
      Unbuildable { reason: UnbuildableReason },
      OverBudget { needed: u64, budget: u32 },           // block, or base + block + 34 for the check
      BadBase { base: u32 },                             // odd, or base + len past u32::MAX
      BufferLength { expected: u32, got: usize },        // write_into
      VerifyFailed { check: BlockCheck },
  }

  #[non_exhaustive]
  pub enum UnbuildableReason {
      WalkHalted,                                        // TooManyIfds / TooManyEntries anywhere
      BrokenPointer { tag: u16 },                        // an issue following 0x8769/0x8825 from IFD0, or 0xA005 from the Exif IFD
      DuplicatePointer { tag: u16 },                     // IFD0 holds 0x8769 or 0x8825 twice, or the Exif IFD holds 0xA005 twice
      DuplicateTag { ifd: BlockIfd, tag: u16 },
      UnreadableValue { ifd: BlockIfd, tag: u16 },       // payload() fails for a known type
  }

  #[non_exhaustive]
  pub enum BlockCheck { Parse, Positions, Entries, Omissions, Length }
  ```
  `BlockError`, `UnbuildableReason`, `Omission`, `OmitReason`, `BlockIfd` and `BlockCheck` implement `Display`, as `StripError` does. Names may be refined at build if a clash is found; report any rename in the handback.
- **Modified files:**
  - `src/write/mod.rs`: the module and re-exports.
  - `src/write/exif.rs`: make its offset-tag lists `pub(crate)` for sharing; no behaviour change.
  - `src/tiff/types.rs`: `type_size` becomes `pub(crate)`.
  - `src/lib.rs`: module doc, one sentence.
- **Tests:**
  - `tests/exif_block.rs`, with new `tests/support/exif_block_shapes.rs` (the source grid) and `tests/support/exif_block_place.rs` (a test placer: header, the block at `base`, IFD0 after it with the two pointers). SPEC-026 reuses the placer.
  - Gated `tests/exif_block_live.rs` and `tests/exif_block_sweep_live.rs`.
- **Fuzz:**
  - `fuzz/fuzz_targets/exif_block.rs` and `fuzz/shared/exif_block_invariants.rs` (independent of `src/write/block.rs`);
  - `[[bin]]` in `fuzz/Cargo.toml`;
  - seeds in `fuzz/seeds/exif_block/` generated by `examples/fuzz_seeds.rs`;
  - pins in `tests/fuzz_layout.rs`;
  - the twelfth entry in the `ci.yml` fuzz-smoke matrix and in `app.just`, with the README list updated.
- **Docs:**
  - README: a short "Exif block for another TIFF writer" section.
  - CHANGELOG `[Unreleased] → Added`.
  - DEC-014 amendment and DEC-015 clarification, already written at design. Correct them if the build measures otherwise, and say so.
- **No new dependencies.** `[dependencies]` stays empty.

## Acceptance Criteria

Each AC maps to tests named in *Failing Tests*. "Fixture grid" means the shapes in AC10, each in both byte orders.

- [ ] **AC1 API.** The items above exist with those signatures, are `#[non_exhaustive]` where shown, have doc comments, and are re-exported from `exif_core::write`. `exif_block` and `write`/`write_into` never panic and never allocate before the budget check.
- [ ] **AC2 Content (DEC-014 items 1, 4 and 5; amendment items 5 and 6).**
  - The block holds IFD0's Exif IFD, that Exif IFD's Interop IFD, and IFD0's GPS IFD, selected **by parent**. An Exif or GPS IFD under IFD1 or a SubIFD is never taken, even when the walk visits it first.
  - GPS-only gives `exif_position() == None`. Exif-only gives `gps_position() == None`. Neither gives `is_empty()`, `len() == 0`, both positions `None`, and `write(base)` returning an empty `Vec` for any even base.
  - An empty Exif IFD in the source is written as an empty IFD (no entries, next-word 0).
  - Kept entries keep their type, count and payload bytes. Inline unused bytes are zero. Count 0 is kept.
  - 0xA005 is LONG, count 1, holding `base + interop_position`.
  - Positions and `len()` are even, and `len()` does not depend on the base.
- [ ] **AC3 Exact bytes.** An independent model in `tests/support/exif_block_model.rs` predicts the block byte for byte for every fixture at bases 8 and 1000. It is written from the DEC-014 amendment item 7 layout, imports nothing from `src/write`, and has its own copy of the omission table. That copy is one of the table's hand-typed mirrors, listed in AC4.
- [ ] **AC4 Omissions (DEC-014 amendment items 2 and 3).**
  - Each tag in the measured list, in each of the Exif, Interop and GPS IFDs (one fixture per tag and IFD), is left out with `OffsetCarrying`. So are 0x014A, 0x8769 and 0x8825 in any of the three IFDs, 0xA005 in the GPS or Interop IFD, and a type-13 entry of any other tag.
  - 0x927C and 0xEA1D are left out with `MakerNote`.
  - Types 0, 14, 16 and 255 are left out with `UnknownType`.
  - The length partners (0x0117, 0x0121, 0x0145, 0x0202, 0xA011, 0xBCC1, 0xBCC3), a tag ExifTool does not know, and every other entry are kept.
  - `omissions()` is in Exif, Interop, GPS order, ascending by tag, and carries the source's field type.
  - The table's sources and mirrors are exactly:
    - `src/write/exif.rs`: the shared `OFFSET_TAGS`, `POINTER_TAGS`, `EXIFTOOL_SUBDIRECTORY_TAGS` and `EXIFTOOL_BLOB_CARRIER_TAGS`;
    - `src/write/block.rs`: the four measured offset tags `OFFSET_TAGS` lacks, and the MakerNote pair;
    - `tests/support/exif_block_model.rs`;
    - `fuzz/shared/exif_block_invariants.rs`, only if it predicts reasons;
    - the sweep test's expected set.

    A test checks that the mirrors name the same set (signal `hand-typed-mirrors-of-a-decision-table-drift`).
- [ ] **AC5 Refusals with no bytes (DEC-014 amendment item 4).** Each `UnbuildableReason`, plus `Source`, `OverBudget`, `BadBase` and `BufferLength`, has a fixture or call that produces it:
  - `Source`: BigTIFF, a bad header, an unreadable IFD0.
  - `WalkHalted`: a 600-IFD SubIFD chain that halts before IFD0's Exif pointer is walked.
  - `BrokenPointer`: an Exif-to-IFD0 loop; Exif and GPS pointers to the same offset; 0xA005 past the end.
  - `DuplicatePointer`: two 0x8769 in IFD0.
  - `DuplicateTag`: two 0x9003 in the Exif IFD.
  - `UnreadableValue`: a kept RATIONAL whose value lies past the end.
  - `OverBudget`, two ways: from `exif_block` when the block alone exceeds `max_len`, using 1,000 entries sharing one 2 KiB value; and from `write` when `base + len + 34` exceeds it.
  - `BadBase`: an odd base; `base + len > u32::MAX`.
  - `BufferLength`: an output slice one byte short and one byte long.

  These must **not** refuse, and must give the full block: a broken IFD1 next-word; a SubIFD loop; a broken SubIFD pointer inside the Exif IFD (its entry is an omission).
- [ ] **AC6 Self-check.**
  - `write` runs DEC-015 clarification's scratch-image readback on the exact bytes it returns, and on success those bytes are returned unchanged.
  - The checker is a separate function. Unit tests feed it a corrupted block and expect `VerifyFailed` with the named check:
    - one value byte flipped: `Entries`;
    - the Interop offset off by 2: `Positions` or `Parse`;
    - an omitted entry not reported: `Omissions`;
    - a reported omission still present: `Omissions`;
    - a truncated block: `Length` or `Parse`.
  - A test proves the checker is wired in: a `#[cfg(test)]` fault hook flips one byte after layout, and `write` returns `VerifyFailed`.
- [ ] **AC7 Readback.** For every fixture and each base in {8, 1000, 65534}, the test placer's output parses with `Container::parse` with no issue. Its Exif, GPS and Interop entries equal the source's (by tag, type, count and payload bytes), minus exactly `omissions()`. This comparison is written in the test and does not call the library's checker.
- [ ] **AC8 Determinism and budget.**
  - `exif_block` twice, and `write(base)` twice, give equal results.
  - `write` equals `write_into` on a slice of `len()`.
  - The budget is checked before any `Vec` of block or scratch size is created; the test is the `OverBudget` fixtures with a tiny budget.
  - `BlockOptions::default().max_len() == 1 << 24`.
- [ ] **AC9 Live ExifTool**, gated by `DIAMOND_EXIF_REQUIRE_ORACLE`; prints `SKIP` without ExifTool unless required. ExifTool runs on the builder's **output** during build (the SPEC-037 lesson). Nothing here is recorded or red-proofed; that is SPEC-026.
  - (a) `tests/exif_block_live.rs`: for every non-refusing fixture at bases 8 and 1000, ExifTool reads the placed file the same as the source for `-ExifIFD:All -GPS:All -InteropIFD:All` (`-a -G1 -s -n`), minus the tags in `omissions()`, and prints no warning the source did not print.
  - (b) `tests/exif_block_sweep_live.rs`: reruns the design's two sweeps, the ExifTool writer pass and the zeroed-target reader pass, from `exiftool -listx`. It asserts that the set ExifTool treats as offset-carrying in each IFD is a subset of the block's `OffsetCarrying` set, and that the block leaves out nothing else beyond the documented conservative extras (0x0190, 0x888A, 0x8290, 0xFE00, the four blob carriers, the pointer tags). A new ExifTool that adds an offset tag fails this test.
- [ ] **AC10 Fixture grid (hostile shapes, AGENTS.md §15)**, each in both byte orders, in `tests/support/exif_block_shapes.rs`:
  1. The design sample: Exif with RATIONAL, SHORT, ASCII, SRATIONAL, UNDEFINED and UserComment; Interop; GPS with RATIONAL[3] and UNDEFINED.
  2. `tiff_shapes::dng(be)`, whose GPS is inside the raw SubIFD, so it is not in the block.
  3. GPS only; Exif only; neither; an empty Exif IFD plus GPS.
  4. MakerNote plus OffsetSchema.
  5. Every measured offset tag, in the Exif and the Interop IFD.
  6. A type-13 entry; SubIFDs inside the Exif IFD (the walk follows them).
  7. Unknown types 0, 14, 16, 255.
  8. Count-0 ASCII, SHORT and RATIONAL; unterminated ASCII; odd-length UNDEFINED; out-of-line values at odd source offsets; two entries sharing one value; a value overlapping an IFD table.
  9. An Exif IFD whose next-word leads to a valid IFD.
  10. Source entries out of ascending order.
  11. An Exif IFD inside a SubIFD, walked before IFD0's.
  12. 0xA005 of type 13; 0xA005 in the GPS IFD; 0xA005 in IFD0 (not in the block).
  13. A JPEG's Exif TIFF blob as the source (`strip_shapes::full_tiff(false)`, the TIFF inside the `Exif\0\0` APP1).
  14. The refusal shapes of AC5 and the non-refusing issues of AC5.
- [ ] **AC11 Fuzz target `exif_block`** (constraint `fuzz-target-per-surface`).
  - Input: the first 2 bytes choose the base (`u16`, made even); the rest is the source.
  - Each input is run under a small budget, and then under the default when the small one refuses.
  - Invariants, written in `fuzz/shared/exif_block_invariants.rs` without calling `src/write/block.rs` internals:
    - no panic;
    - every refusal is typed;
    - on success, `write(base)` has length `len()`, places and parses clean, and its entries equal the source's minus `omissions()`;
    - every omission names an entry that exists in the source;
    - the result is deterministic.
  - Wiring:
    - seeds are generated from the AC10 grid by `examples/fuzz_seeds.rs`, and their count is pinned in `tests/fuzz_layout.rs`;
    - the CI matrix entry, `app.just` and the README list are updated, and `tests/fuzz_layout.rs`'s target-list strings with them;
    - `just fuzz-smoke exif_block` runs 60 s clean.
- [ ] **AC12 Gates and docs.**
  - All of these pass on the pinned 1.90.0 toolchain:
    - `just lint`, `just lint-no-allow`, `just lint-fuzz`, `just forbid-unsafe`;
    - `just deny`, `just deny-fuzz`;
    - `DIAMOND_EXIF_REQUIRE_ORACLE=1 just test` (no skips);
    - `just mutation-oracle`;
    - `just validate`, `just cost-audit`.
  - README section; CHANGELOG `[Unreleased] → Added`.
  - `fuzz/Cargo.lock` restored if a run changed it.

## Failing Tests

Written at design. The builder makes them pass.

- **`src/write/block.rs` unit tests:**
  - `layout_puts_tables_first_then_values_in_ifd_and_tag_order`;
  - `positions_and_len_are_even_and_base_independent`;
  - `interop_pointer_is_long_count_one_at_base_plus_position`;
  - `inline_unused_bytes_are_zero`;
  - `count_zero_is_kept`;
  - `selects_exif_and_gps_by_parent_ifd0`;
  - `checker_catches_flipped_value_byte`;
  - `checker_catches_wrong_interop_offset`;
  - `checker_catches_unreported_omission`;
  - `checker_catches_reported_but_present`;
  - `checker_catches_truncation`;
  - `fault_hook_makes_write_verify_failed`;
  - `budget_is_checked_before_allocation`;
  - `odd_base_and_overflowing_base_are_bad_base`.
- **`tests/exif_block.rs`:**
  - `grid_blocks_match_the_independent_model` (AC3);
  - `grid_blocks_read_back_equal_minus_omissions` (AC7);
  - `each_offset_tag_is_omitted_in_each_ifd`;
  - `makernote_and_offset_schema_are_omitted`;
  - `unknown_types_are_omitted`;
  - `length_partners_and_unknown_tags_are_kept`;
  - `omissions_are_ordered_and_typed` (AC4);
  - `gps_only_exif_only_and_empty` (AC2);
  - `each_refusal_reason_is_reachable_and_returns_no_bytes`;
  - `issues_outside_the_block_do_not_refuse` (AC5);
  - `deterministic_and_write_into_matches_write`;
  - `default_budget_is_16_mib` (AC8);
  - `omission_table_mirrors_agree` (AC4, greps the listed mirrors).
- **`tests/exif_block_live.rs`** (gated): `placed_block_reads_as_source_in_exiftool`.
- **`tests/exif_block_sweep_live.rs`** (gated): `exiftool_offset_tags_are_all_omitted`.
- **`tests/fuzz_layout.rs`:**
  - `exif_block_seeds_are_pinned_and_parse`;
  - the target-list assertions extended to `exif_block`.

## Non-Goals

- **Byte-order conversion.** Converting a big-endian source into a little-endian block, and the byte-order-sensitive UNDEFINED rule (question `exif-block-byte-order-sensitive-undefined`), are SPEC-041.
- **Recorded, red-proofed ExifTool parity.** The fixture matrix with committed records is SPEC-026. AC9 here is a live smoke.
- **Policy presets** are SPEC-027.
- **The surrounding file:**
  - writing IFD0, SubIFDs, IFD1 or image data;
  - a JPEG or PNG container as the source. The caller passes the TIFF bytes; a JPEG's Exif blob works because it is TIFF-structured.
- **Things the builder never does:**
  - moving a MakerNote;
  - BigTIFF;
  - repairing a source;
  - caching a block across bases.
- **Release work:**
  - a CLI command;
  - the crates.io pre-release (a release spec after STAGE-007).

## Notes for the Implementer

- **Two builder parts so a stop is clean.**
  - *Part A* is the library, the unit tests, `tests/exif_block.rs`, the shapes, the model and the placer. Commit the library first.
  - *Part B* is the fuzz target and its seeds, pins and CI entry; the two live tests; and README, CHANGELOG and any DEC correction.
- **Select by parent, never by `ifd_of_kind`.** The walk visits SubIFDs before IFD0's Exif pointer (DEC-005 walk order), so the first Exif IFD in `ifds()` can be a SubIFD's.
- **Share the offset lists; do not re-type them.**
  - `POINTER_TAGS`, `OFFSET_TAGS`, `EXIFTOOL_SUBDIRECTORY_TAGS` and `EXIFTOOL_BLOB_CARRIER_TAGS` in `src/write/exif.rs` become `pub(crate)` and are imported.
  - `OFFSET_TAGS` holds 7 of the 11 measured offset tags (in its offset column). The other four (0x8781, 0xA010, 0xBCC0, 0xBCC2) get one new const in `block.rs`, with a comment citing the DEC-014 amendment.
  - Do **not** add them to `OFFSET_TAGS`. That would change what the strip protects, which is question `strip-offset-tags-incomplete`, not this spec.
- **Compute the length in `u64`** with `checked_*` arithmetic (the lint set denies `arithmetic_side_effects` and indexing). Refuse before allocating.
- **The self-check's scratch image is DEC-015's clarification:**
  - an 8-byte header in the block's byte order;
  - zeros up to `base`;
  - the block;
  - IFD0 with 0x8769 and/or 0x8825 (LONG, count 1, `base + position`) in ascending tag order, then a next-word of 0.

  It is at most `base + len + 34` bytes. Parse it with `Container::parse` and compare against the plan's kept entries.
- **Run ExifTool on outputs during the build** (AC9), not only in verify. The design's layout probe and sweep are reproducible from the measurements above. For timeouts: macOS has no `timeout`; use `perl -e 'alarm 8; exec @ARGV' -- exiftool ...`.
- **Toolchain:** pinned 1.90.0 (`RUSTUP_TOOLCHAIN=1.90.0`) for lint and test. Run `just lint-fuzz` too. Restore `fuzz/Cargo.lock` after fuzz runs.
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
