---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-002
  type: story                      # epic | story | task | bug | chore
  cycle: ship  # frame | design | build | verify | ship
  blocked: false
  priority: medium                 # critical | high | medium | low
  complexity: L                    # XS | S | M | L | XL | XXL — the EXPECTED size, set at design
                                   #   (XL/XXL almost certainly means it's a stage, not a spec)
  complexity_actual: L          # stamped at ship: what it ACTUALLY took, same scale.
                                   #   Expected-vs-actual drift is what `just calibration` reads.
  verify_verdict: approved  # approved | punch-list | rejected — the OUTCOME of the verify
                                   #   cycle, stamped by `just advance-cycle` when the spec leaves
                                   #   verify (same three verdicts Prompt 4 already returns).
                                   #   Recorded in front-matter, not just prose, so "verify never
                                   #   rejects anything" stops being a hunch and becomes a number.

project:
  id: PROJ-001
  stage: STAGE-001
repo:
  id: diamond-exif

handoff:
  from_agent: claude-opus-5-5  # from .repo-context tier_map.design (DEC-005)
  to_agent: null                   # filled when HANDOFF is created (any agent — see docs/porting.md)
  created_at: null

references:
  decisions: [DEC-001, DEC-002, DEC-003]                    # [DEC-NNN, DEC-MMM]
  constraints: [forbid-unsafe-code, panic-free-library, no-io-in-library, no-new-top-level-deps-without-decision]                  # [constraint-id-1, constraint-id-2]
  related_specs: []                # [SPEC-NNN]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: [SPEC-001]

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-001's <capability>". Optional; null is acceptable.
value_link: "infrastructure enabling STAGE-001: the bounds-checked, provenance-recorded IFD walker every later spec extends"

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
  tokens_estimate: null
  sessions:
    - cycle: build
      agent: claude-sonnet-5-5
      interface: other
      tokens_total: 193631
      estimated_usd: 1.28
      duration_minutes: 8
      recorded_at: 2026-11-05
      notes: "tokens_total left for the orchestrator (metered from the task result); worktree base was stale and was fast-forwarded to claude/rust-exif-metadata-6351d5"
    - cycle: verify
      agent: claude-opus-5-5
      interface: other
      tokens_total: 152572
      estimated_usd: 1.01
      duration_minutes: 4
      recorded_at: 2026-11-05
      notes: "tokens_total left null for the orchestrator (metered count); verdict APPROVED with advisory test-gap notes"
  totals:
    tokens_total: 346203
    estimated_usd: 2.29
    session_count: 2
shipped_at: 2026-11-05
---

# SPEC-002: Port the strict TIFF skeleton

## Context

The first real code. Plan Step 2 (docs/DIAMOND-EXIF-EXTRACTION-PLAN.md §3) and porting brief §1: port irradiance's DNG-stripped, byte-faithful IFD walk and its hostile-input tests FIRST, with the policy unchanged (fatal on any bad IFD), so that SPEC-003's tolerant-walk diff against it stays legible. "Port" means copy-then-edit (DEC-003). The walker stays over `&[u8]` (DEC-002). Reader code lands only with its reader: anything with no consumer in this spec (u64/f32/f64/i16 readers, encode twins, `Entry` constructors for the writer, `cost_the_field`, `is_structural_tag`) is NOT ported here (AGENTS §11 "ship the reader with the field").

## Goal

`src/error.rs`, `src/reader.rs` and `src/tiff/{mod,types,tags}.rs` ported from irradiance `src/ifd.rs` (DNG removed, `payload` returning `&'a [u8]`), with the 17 hostile-input tests plus four ported-with-edit tests and the independent `Build` byte-builder, all green under the five-lint deny set, with one provenance-ledger row per ported file.

## Inputs

- **Source (irradiance, READ ONLY, never modify; sources unchanged since the brief, HEAD `2b6b07a`):** `~/PSeven/experiments/crustimg_redo_plus/irradiance/src/ifd.rs` and the `Error` enum in `src/lib.rs:144` onward. Port map with line numbers: `docs/porting-brief-diamond-exif.md` §1.1 (item table), §1.2 (coupling points C1-C6), §1.3 (tests), §1.4 (`&[u8]`).
- **This repo:** `decisions/DEC-001..003`, `docs/provenance-ledger.md`, `guidance/toolchain-brief.md`, the SPEC-001 crate.

## Outputs

- **Files created:**
  - `src/error.rs` — `#[non_exhaustive] pub enum Error` with hand-written `Display` and `std::error::Error`, copied from `src/lib.rs` (variants `ifd.rs` uses). Keep only variants that still have a producer after the edits below. Expected 11: `Truncated`, `NotTiff`, `UnsupportedTiffVersion`, `CyclicIfd`, `IfdDepthExceeded`, `TooManyIfds`, `UnexpectedFieldType`, `ValueOverflow`, `TagTooLarge`, `MissingTag`, `OffsetOutOfRange`. `MalformedRationalValue` and `RationalValueOutOfRange` exist only for the DNG integer-division rule and the `i32` narrowing, both removed here; grep to confirm before dropping them and say so in the handback. A `pub type Result<T> = core::result::Result<T, Error>`.
  - `src/reader.rs` — `pub enum ByteOrder { Little, Big }` (CLEAN, `ifd.rs:262-267`) with `pub(crate)` `u16`/`u32` methods (add `i32` only if `rationals`/`SRational` uses it), `pub(crate) fn read_u16`/`read_u32`, `offset_to_usize`. Keep the `.get().try_into()` idiom exactly; it is what `clippy::indexing_slicing` accepts.
  - `src/tiff/mod.rs` — constants `MAX_IFD_DEPTH` 8, `MAX_IFDS` 512, `MAX_TAG_VALUES` 1<<20, `TIFF_VERSION` 42, `ENTRY_SIZE` 12 (re-justify depth 8 for Exif to Interop to MakerNote in a comment); `Entry` + `Entry::parse` (private fields, verbose `.get().ok_or().try_into()` shape kept), `Ifd` + accessors (reword the DNG prose at `:467-469`), `Container<'a>` with `parse`, `byte_order`, `ifd0_offset`, `ifds`, `ifd0`, `walk_chain`, `read_ifd`, the SubIFDs (tag 330) child walk, `payload`, `scalar`, `required_scalar`, `values`. Strict policy unchanged: any bad IFD is `Err`.
  - `src/tiff/types.rs` — `TYPE_*` consts actually used, `type_size` (total over the 12 types, CLEAN), `uints` accepting BYTE/UNDEFINED/SHORT/LONG/IFD only (RATIONAL and everything else is `UnexpectedFieldType`; delete the per-tag RATIONAL gate `:1032` and the integer-division rule `:1078-1107`), and `rationals` returning verbatim `URational { num: u32, den: u32 }` / `SRational { num: i32, den: i32 }` (never divide, zero denominators are representable, `:1135-1138`).
  - `src/tiff/tags.rs` — baseline-TIFF tag consts the walker and tests use (`NEW_SUBFILE_TYPE`...`SUB_IFDS`, `ORIENTATION`); no DNG/CFA/colour tags; no pointer tags yet (SPEC-004).
- **Files modified:**
  - `src/lib.rs` — add `pub mod error; pub mod reader; pub mod tiff;` and re-export `Error`, `Result`; keep the lint block untouched. Crate docs: drop "exports nothing yet".
  - `docs/provenance-ledger.md` — one row per new file (`error.rs`, `reader.rs`, `tiff/mod.rs`, `tiff/types.rs`, `tiff/tags.rs`, and the test builder): Source = irradiance `src/ifd.rs` (and `src/lib.rs` for `Error`) at the HEAD sha you copied from (record `git -C <irradiance> rev-parse HEAD`), Class 3 (upstream class 1: TIFF 6.0 §2), Licence MIT OR Apache-2.0 own code.
  - `CHANGELOG.md` — `[Unreleased] → Added`.
- **Tests:** in-module `#[cfg(test)] mod tests` in `src/tiff/mod.rs`, mirroring irradiance (scoped `#[allow]` of the five lints at the test module only, as `ifd.rs:1717-1727`; the `-F` re-check runs `--lib` so it stays invisible). Port the `Build` byte-builder (`ifd.rs:1735-1860`) as-is; it is deliberately independent of any shared builder.
- **No new public flags, no dependencies.**

## Acceptance Criteria

- [ ] All gates pass: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo rustc --lib -- -F unsafe_code`, `cargo deny check licenses`, `just validate`, `just decisions-audit`; tests and clippy also pass on 1.90.0 (see toolchain-brief for the host traps).
- [ ] No dead code and no `#[allow(dead_code)]` outside the test module: every ported item has a consumer in this spec.
- [ ] The strict policy is unchanged: a bad non-IFD0 IFD still returns `Err` (SPEC-003 flips it).
- [ ] `Container::payload` returns `&'a [u8]` tied to the input, proven by a test where the slice outlives the `Container` and its pointer lies inside the input buffer (DEC-002 validation).
- [ ] `rationals` never divides and holds `u32/u32` and `i32/i32` verbatim, including a zero denominator and a value above `i32::MAX`.
- [ ] No DNG symbol survives: `grep -rniE "dng|cfa|sensor|black_level|active_area|opcode" src/` returns nothing except historical comments that are explicitly about provenance.
- [ ] Every ported file has a ledger row naming the irradiance commit.
- [ ] irradiance is unmodified (`git -C <irradiance> status --porcelain` empty).

## Failing Tests

Written in design; the build ports/writes them first, sees them fail to compile or fail, then makes them pass. Irradiance line numbers are for `src/ifd.rs`; swap any DNG tag for an EXIF/baseline tag (e.g. `ORIENTATION`, `IMAGE_WIDTH`) and replace the `sensor(&b)` helper with a small local baseline entry set. In `src/tiff/mod.rs` tests:

- The 17 hostile-input tests (`2190-2341`), semantics unchanged: `header_reads_both_byte_orders`, `empty_input_is_truncated_not_a_panic`, `a_non_tiff_mark_is_rejected`, `a_wrong_version_word_is_rejected`, `an_ifd0_offset_of_zero_yields_no_ifds`, `an_ifd_offset_past_eof_is_truncated`, `an_entry_count_past_eof_is_truncated`, `a_payload_offset_past_eof_is_truncated`, `a_count_that_overflows_the_size_product_is_typed`, `an_inline_value_is_read_from_the_entry_not_the_file`, `an_unknown_field_type_costs_the_entry_not_the_file`, `a_readable_type_this_module_does_not_widen_is_typed` (uses an SRATIONAL entry; `uints` must refuse it), `a_self_referential_subifd_terminates`, `a_two_hop_subifd_cycle_terminates`, `a_chain_that_points_at_itself_terminates`, `nesting_past_the_depth_limit_terminates`, `a_long_chain_stops_at_the_ifd_limit`.
- Ported with edit: `srational_reads_signed_and_fractional_verbatim` (from `2063`, EXIF tag, `SRational`), `rational_reads_verbatim_without_integer_division` (from `2097`, `URational`), `subifd_offsets_are_walked_in_order_and_recorded_with_their_parent` (from `2969`), `every_error_variant_this_module_returns_displays_something` (from `2996`; variant list must equal the final `Error`).
- New: `rational_holds_zero_denominator_and_values_above_i32_max_verbatim`; `payload_borrows_from_the_input_not_the_container` (slice outlives the `Container`, `as_ptr` within the input range).

## Non-Goals

- The tolerant-walk policy and `ParseIssue` (SPEC-003)
- Exif/GPS/Interop pointer walking, ASCII/signed/float decoders (SPEC-004)
- Oracle, ExifTool, `mutate.rs`, `probe.rs` (SPEC-005); fuzz target (SPEC-006)
- The writer: no encode twins, no `Entry` constructors, no raw-value accessor
- `cost_the_field` and a diamond-exif `is_structural` set (they land with their first consumer in SPEC-003/004)
- Any change to irradiance

## Notes for the Implementer

- Copy text, then edit; never link or depend on irradiance. Work from `docs/porting-brief-diamond-exif.md` §1; its line numbers match irradiance HEAD `2b6b07a` (verified unchanged since the brief's `a8b9af8`).
- Order: write/port the tests first, then the code. Keep each ported file buildable and green; commit per file or small group.
- `clippy::indexing_slicing` and `arithmetic_side_effects` are denied in the library: keep irradiance's checked/`get()` shapes verbatim, do not "simplify" `Entry::parse`. Use `checked_*`/`saturating_*` as irradiance does. If a ported line trips a lint, fix it the irradiance way, not with `#[allow]` (forbidden below the crate root outside the test module).
- Do not add `bytes`, `thiserror` or any dependency (DEC-002).
- The 30 non-portable irradiance tests are not ported. Open question `ifd-rs-test-classification` is closed by this work: run the ported set and note in the handback any test that behaved differently from the brief's classification.
- Do not push or open a PR; report the branch.

---

## Reflection

1. **What would I do differently next time?**
   — Specify the inline-value borrow in the spec: the brief called the `&'a [u8]` change "one token", but inline payloads needed a `value_at` offset on `Entry`. Reading the code, not the brief, at design time would have caught it.

2. **Does any template, constraint, or decision need updating?**
   — No DEC change. DEC-002 held (the borrow test is a real guard: tying `payload` to `&self` fails to compile). Open question `ifd-rs-test-classification` is answered. Two guard tests inherited from irradiance are weak (depth limit +1, IFD-count `>=` vs `>`); carried into SPEC-003.

3. **Is there a follow-up spec I should write now before I forget?**
   — SPEC-006 (fuzz target) is already framed and must land before SPEC-003. The two boundary tests ride along with SPEC-003.

4. **Where was the worst defect caught?** — none
   *(Verify found no defects; two surviving mutants were inherited from irradiance's tests and are advisory.)*

5. **What can a user do now that they couldn't before?** — none (infrastructure): the crate now parses TIFF/EXIF IFD structure with bounds-checked, cycle-guarded reads and typed errors, but exports no EXIF-level API yet.
