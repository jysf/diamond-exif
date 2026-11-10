---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-020
  type: story                      # epic | story | task | bug | chore
  cycle: build  # frame | design | build | verify | ship
  blocked: false
  priority: medium                 # critical | high | medium | low
  complexity: L                    # XS | S | M | L | XL | XXL — the EXPECTED size, set at design
                                   #   (XL/XXL almost certainly means it's a stage, not a spec)
  complexity_actual: null          # stamped at ship: what it ACTUALLY took, same scale.
                                   #   Expected-vs-actual drift is what `just calibration` reads.
  verify_verdict: null             # approved | punch-list | rejected — the OUTCOME of the verify
                                   #   cycle, stamped by `just advance-cycle` when the spec leaves
                                   #   verify (same three verdicts Prompt 4 already returns).
                                   #   Recorded in front-matter, not just prose, so "verify never
                                   #   rejects anything" stops being a hunch and becomes a number.

project:
  id: PROJ-001
  stage: STAGE-004
repo:
  id: diamond-exif

handoff:
  from_agent: claude-opus-5-5  # from .repo-context tier_map.design (DEC-005)
  to_agent: null                   # filled when HANDOFF is created (any agent — see docs/porting.md)
  created_at: null

references:
  decisions: [DEC-006, DEC-008, DEC-009, DEC-012, DEC-013]
  constraints: [panic-free-library, forbid-unsafe-code, no-io-in-library, fuzz-target-per-surface, test-before-implementation, oracle-must-be-shown-red, exiftool-never-linked, no-new-top-level-deps-without-decision]
  related_specs: [SPEC-019, SPEC-021, SPEC-022]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: [SPEC-019]                     # e.g. [SPEC-002]

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-004's <capability>". Optional; null is acceptable.
value_link: "STAGE-004: the first verified strip: whole-segment removal with the audit as the oracle, so All and Everything provably remove what the preview says"

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
  tokens_estimate: 1000000
  sessions: []
  totals:
    tokens_total: 0
    estimated_usd: 0
    session_count: 0
---

# SPEC-020: Writer foundation and JPEG segment-level strip: writer DEC, change plan, pixel-integrity and audit-as-oracle invariants, `All`/`Everything`, `rewrite_jpeg` fuzz target, ExifTool output oracle

## Context

SPEC-019 made the audit name everything a strip must handle in a JPEG. This spec is the first writer. DEC-013 (written in this design; **read it first, it is normative**) fixes the contract: an edit plan of whole-range removals and same-length patches only, fail closed (typed refusal, never a best-effort file), segment as the unit of removal, the Exif segment all or nothing, Multi-Picture images and their index removed or kept together with the index patched, no byte both removed and kept, and a self-check of the output against the audit before any byte is returned.

The broad presets (`All`, `Everything`) are exactly what whole-segment removal can do, so they are the proving ground for the invariants: pixel and kept-segment bytes unchanged, the output's audit shows none of the removed categories, ExifTool agrees. The narrow presets (`Gps`, `GpsAndDevice`, `AllPii`) work in this spec only for files with no Exif finding in their categories; otherwise they return `NeedsEntryEdit` (SPEC-021 adds the in-segment edit).

## Goal

`exif_core::write::strip(bytes, policy)` and `sanitize` for JPEG, the shared edit-plan type, the MPF-index accessor in the JPEG layer, the invariants as tests and fuzz guards, a `rewrite_jpeg` fuzz target, an ExifTool output oracle with red-proofs, and a logged re-run on the local real-file corpus.

## Inputs

- **Decisions:** DEC-013 (new, normative), DEC-012 (regions and categories), DEC-009 (presets and dispositions: the preview is the contract the writer is checked against), DEC-008, DEC-006.
- **Files to read first:** `src/jpeg/mod.rs` (`Jpeg`, `Segment`, `SegmentKind`), `src/jpeg/metadata.rs` (`Region`, `read`, the MPF parser, `uncovered`), `src/audit/mod.rs` (`audit`, `Finding`, `Origin`), `src/audit/category.rs` (`region_category`), `src/policy.rs` (`StripSpec`, `StripPolicy`), `src/audit/preview.rs`, `tests/support/real_shapes.rs` (SPEC-019 builders), `fuzz/shared/jpeg_metadata_invariants.rs` (the independent naive MPF reader to reuse), `tests/jpeg_oracle.rs` and `tests/inventory_oracle.rs` (the record/replay/red-proof pattern), `examples/oracle_record.rs`.
- **Evidence:** SPIKE-001 (archived), questions `writer-in-place-delete-and-scrub`, `strip-keeps-orientation`, `writer-stray-bytes`, `writer-mpf-attribute-fields`.

## Outputs

### Library

- **`src/jpeg`**:
  - `Segment::range()` returns `offset..offset + 4 + payload.len()` (marker, length word, payload) with checked arithmetic, `None` if it overflows.
  - `JpegMetadata::mpf()` returns `Option<&MpfIndex>`: the index segment the audit read the embedded images from. `MpfIndex` has `segment()` (index into `Jpeg::segments`), `header_start()` (input offset of the byte after `MPF\0`), `big_endian()`, and `entries()`: for every 16-byte entry, in order, `index`, `offset` (`u32` as read), `size` (`u32` as read), `size_field` and `offset_field` (input offsets of those two `u32` fields), and `region` (`Some(i)` index into `regions()` when the entry became an `EmbeddedImage`, else `None`). Only the first MPF segment is the index (SPEC-019 rule); a second, or an unparsable one, is `None` here. Nothing is copied; all positions are checked.
- **`src/write/` (new module, public):**
  - `plan.rs`: `EditPlan` (`#[non_exhaustive]`), built from `Edit::Remove(Range)` and `Edit::Patch { offset, bytes }`; `EditPlan::validate(input_len)` and `EditPlan::apply(&self, input) -> Result<Vec<u8>, PlanError>` per DEC-013 part 1: ranges inside the input, removals merged (overlapping and adjacent ranges become one), patches outside every removal, output length = input length minus merged removal length, deterministic, checked arithmetic, no allocation larger than the input.
  - `jpeg.rs`: the JPEG planner (rules below).
  - `verify.rs`: the self-check of DEC-013 part 7.
  - `mod.rs`: `pub fn strip(data: &[u8], policy: StripPolicy) -> Result<Stripped, StripError>`; `pub fn sanitize(data: &[u8], policy: StripPolicy) -> Result<Vec<u8>, StripError>`; `Stripped { bytes: Vec<u8>, removals: Vec<Removal>, patches: Vec<PatchRecord> }` (accessors, `#[non_exhaustive]`); `Removal { category, kind: RegionKind, offset, len }` (input coordinates; one entry per removed region, in input order, the MPF index segment and the Exif segments included, with the category that caused them); `PatchRecord { offset, len }` (output coordinates). `StripError` (`#[non_exhaustive]`, `Display`, `std::error::Error`): `Unsupported { kind: ContainerKind }`, `NotJpeg`, `Unstrippable { reason, offset }`, `NeedsEntryEdit { categories: StripSpec }`, `Overlap { offset }`, `MpfNotIndexed { offset }`, `VerifyFailed { check }`. `lib.rs` re-exports `strip`, `sanitize`, `Stripped`, `StripError`, and its docs stop saying "No writer yet".
- **No change** to the audit, the report, the CLI or exit codes. `exif-audit` stays read-only.

### Rules (normative; each maps to a named test)

Let S be `policy.spec()`, and EXIF9 = {Gps, DeviceId, Authorship, FreeText, Timestamps, CameraModel, Thumbnail, MakerNote, Software}.

1. **Dispatch.** `sniff` the container; JPEG goes on; TIFF, PNG, WebP, HEIC and anything else return `Unsupported { kind }` (SPEC-022 adds TIFF); fewer than two bytes or no `FF D8` is `NotJpeg`.
2. **Refusals (`Unstrippable`).** The walk halted by a cap; any `BadSegmentLength`; `TooManySegments`; `TooManyIssues`. `MissingSos` and `MissingEoi` are **not** refusals. The error names the reason and the input offset.
3. **Segment regions.** In input order: each `Xmp` and `ExtendedXmp` segment is removed iff `Xmp ∈ S`; each `Icc` iff `Icc ∈ S`; each `Iptc` (every Photoshop APP13 segment) iff `Iptc ∈ S`; each `Jumbf` iff `Jumbf ∈ S`; each `Comment` iff `FreeText ∈ S`; each `OtherApp` iff `OtherSegment ∈ S`. The removal is the segment's whole `range()`. `JFIF\0`, `Adobe\0`, `DQT`, `DHT`, `SOF*`, `DRI`, other markers and the scan from `SOS` through `EOI` are never removed.
4. **Exif (DEC-013 part 4).** If `EXIF9 ⊆ S`, every Exif segment (all blocks, including the extra ones) is removed whole. If `S ∩ EXIF9 = ∅`, none. Otherwise, if any Exif block has a finding in `S ∩ EXIF9` **or** is unreadable (`failed_blocks > 0`), return `NeedsEntryEdit { categories: S ∩ EXIF9 }`; if every Exif block is readable and has no finding in `S ∩ EXIF9`, Exif segments are left alone (and the other rules still apply). A file with no Exif segment is never a `NeedsEntryEdit`.
5. **Trailer and embedded images.** Each `Trailer` region is removed iff `OtherSegment ∈ S`. Each `EmbeddedImage` region (its byte range) **and the MPF index segment** are removed iff `EmbeddedImage ∈ S`.
6. **MPF patches (DEC-013 part 5).** If the index exists and `EmbeddedImage ∉ S`: let R be the merged removal set; for each kept secondary entry (`region` is `Some`) add a patch for `offset_field` of `offset - removed(R, header_start, image_start)`, and for entry 0 a patch for `size_field` of `size - removed(R, 0, primary_end)` (`primary_end` = EOI offset + 2); both in the index's byte order, same length (4 bytes), only if the new value differs; a computation that would underflow is `VerifyFailed { check: MpfFields }`. Entries with no region are never patched. An `MPF\0` APP2 segment that is not the index, together with any removal at or after its end offset, is `MpfNotIndexed { offset }`; with no removal after it, nothing is done.
7. **Overlap (DEC-013 part 6).** After merging, a removal range that intersects the scan region, a kept segment, a kept embedded image or a kept trailer piece is `Overlap { offset }` (the first such offset, in input order). Removals that overlap each other are merged (an embedded image removed together with a COM that contains it is fine when both are removed).
8. **Result.** `Stripped.bytes` is `plan.apply(input)`; `removals` lists, in input order, one record per removed region with the category that caused it (the Exif segments under whichever EXIF9 category the audit found first, `Thumbnail` if none: pin in a test; the MPF index under `EmbeddedImage`); `patches` the patch records. An empty plan returns the input bytes unchanged and empty lists.
9. **Self-check (verify.rs).** Before returning, the checks of DEC-013 part 7 (a) to (e) run; the first failing one is `VerifyFailed { check }` (`Preview`, `Kept`, `Scan`, `Walk`, `Idempotent`, `MpfFields`) and **no bytes are returned**. It is implemented from the audit and `Jpeg`, not from the plan.
10. **Invariants (held by tests and fuzz, not only by the self-check).** Output length ≤ input length, equal iff the plan is empty; the scan region and every kept segment's bytes are identical and in order (the output equals the input with the listed removals deleted and the listed patches applied, recomputed independently); `audit(output)` has no finding in any category of S and keeps every other finding (DEC-013 7(b)); the input slice is never modified; `strip` is deterministic and idempotent; no input panics, and the work is linear in the input plus the segment cap.
11. **Values really gone.** After `All` or `Everything` on a fixture whose Exif, GPS, MakerNote, XMP, IPTC, comment and APP10 bytes contain distinctive canary strings, none of those canary byte strings occurs anywhere in the output (a byte search over the whole output); canaries in kept data (ICC under `All`) still occur.

### Fixtures (new `tests/support/strip_shapes.rs`, test code only; reuses `tests/support/real_shapes.rs`)

Deterministic builders, each with a canary byte string in every metadata carrier so rule 11 is a plain search:
- `kitchen_sink()`: SOI, JFIF, Exif (IFD0: Make, Model, Orientation, Artist, Software, DateTime; Exif IFD: DateTimeOriginal, BodySerialNumber, UserComment, MakerNote; GPS IFD with latitude, longitude; IFD1 with an embedded thumbnail), XMP, extended XMP (two chunks), ICC (two chunks), Photoshop IPTC in two APP13 segments, JUMBF, two COM, APP10 `AROT`, APP14 `Adobe`, DQT, SOF0, DHT, DRI, SOS with entropy bytes, EOI, a 313-byte trailer.
- Variants: `no_exif()`; `exif_orientation_only()` (readable, no finding in EXIF9); `exif_without_gps()` (has Make, Model, Software, no GPS); `two_exif_blocks()`; `unreadable_exif()` (APP1 `Exif\0\0` with garbage TIFF); `comment_only()`; `mpo_late_xmp()` (primary has APP1 Exif, APP2 MPF, then APP1 XMP and a COM *after* the MPF segment, two secondary images, one with its own Exif; big- and little-endian variants); `mpo_trailer_gaps()` (Nikon style: secondaries separated by gaps, AROT); `unindexed_mpf()` (APP2 `MPF\0` that does not parse, plus a later removable segment); `second_mpf()`; `mpf_malformed_entry()`; `com_over_embedded()` (an `FF D8` image range inside a COM payload, listed by the MPF index); `overlapping_images()`; `bad_length()`; `halted_walk()` (more segments than the cap); `missing_eoi()`; `missing_sos()`; `fill_bytes_between_segments()`.
- A tiny independent expected-output function in the test support (`expected_bytes(input, removals, patches)`) that deletes ranges and applies patches without using `EditPlan`, to compare byte for byte.

### Oracle (tests/strip_oracle.rs; ExifTool run, never read or linked)

For every fixture and each of `All`, `Everything`, and the custom sets `{Xmp}`, `{Icc}`, `{EmbeddedImage}`, `{OtherSegment}`, `{Iptc, FreeText}`, ExifTool is run on the **stripped output** as `exiftool -a -u -G1 -s` (the inventory form of SPEC-013) and the result is committed as a record under `tests/oracle-fixtures/exiftool-strip/` (command, version, `# stderr:`), replayed by default and re-recorded by `just oracle-record` with a clean `git status`. Checks:
- **Group presence.** A hand-typed table of which ExifTool groups (`IFD0`, `ExifIFD`, `GPS`, `IFD1`, `XMP-*`, `IPTC`, `ICC_Profile`, `JUMBF`, `MPF`/`MPImage*`, `JFIF`, `Adobe`, `Comment`, `File`) must be present or absent per policy and fixture; the output's records equal the table. (The builder measures any group-name facts first and pins them in the comparator.)
- **No new warnings.** No `Warning` tag appears in any output record that the corresponding input record did not have.
- **MPF.** On `mpo_late_xmp` and `mpo_trailer_gaps` under `All`: the stripped file's `MPImageStart`/`MPImageLength` equal the independently computed new positions and sizes, the bytes at those positions start `FF D8`, ExifTool lists the same number of `MPImage` groups as before, and the primary's reported size equals its actual length. Under `Everything`: no `MPF` or `MPImage` group.
- **Red-proofs (control first):** each shown red and named: a record with an XMP group left in; a missing ICC group; an `MPImageStart` off by the removed length; a phantom group; a stripped file whose Exif segment was kept; the writer's own self-check disabled in a test build is **not** the red-proof (the oracle must fail on its own).
- **Live runs** when `exiftool` is present (`DIAMOND_EXIF_REQUIRE_ORACLE=1` makes absence a failure); skips are loud.

### Fuzz and seeds

- New target `fuzz/fuzz_targets/rewrite_jpeg.rs` over `fuzz/shared/rewrite_invariants.rs`. Input: byte 0 selects the policy (0 to 4 the presets in `StripPolicy::PRESETS` order, 5 `Custom(none)`, 6 `Custom` from a 16-bit mask in bytes 1 and 2, 7 and above `All`), the rest is the JPEG candidate.
- The harness holds an **independent naive stripper** (own segment walker for the strip subset, own benign list and category mapping typed from DEC-012/013, the SPEC-019 harness's second MPF reader, its own overlap and removed-count arithmetic) that predicts the exact output (or the exact refusal class) for the policy, and checks: on `Ok`, output equals the prediction byte for byte, output length ≤ input, scan region equal, `audit(output)` has no category of S, kept findings preserved, idempotent, `removals` consistent with the bytes; on `Err`, the class matches the naive prediction (`NeedsEntryEdit` only when the naive Exif rule says so, `Overlap` only when an independent overlap exists, `MpfNotIndexed` only when the naive rule says so, `Unstrippable` only with a halting or bad-length issue); **`VerifyFailed` is always a harness failure** (it means the plan and the self-check disagree). Guards run alone, `iff` in both directions, plants in both directions, step counter, as in SPEC-019.
- Seeds (≥ 60 files) in `fuzz/seeds/rewrite_jpeg/`: each SPEC-019 `shape-*` JPEG seed and the new strip fixtures under `All`, `Everything` and a custom mask; the old corpora and records are **not** changed. `tests/fuzz_layout.rs` and the CI fuzz-smoke list gain `rewrite_jpeg`; seed counts are pinned in the same change. A plant of one guard is shown found by the real libFuzzer target (60 s) from the committed seeds.

### Local corpus (a log, not committed)

`examples/strip_corpus.rs` (a dev example, library calls only) walks a directory in place, strips every JPEG in memory with `All` and `Everything`, runs the self-check, and prints only **counts**: files, `Ok`/each error kind, bytes removed, patches, files with MPF, pixel-identical count. It writes nothing to disk. For ExifTool on a real output the example can emit **one** stripped file to stdout (`--emit PATH POLICY`) to be piped to `exiftool -`; nothing is saved. Counts and short hashes only in the Handback; never values, content or paths.

## Acceptance Criteria

- DEC-013 is implemented as written: rules 1 to 11 hold, each by its named test; `exif-audit` and the report are byte-for-byte unchanged (goldens and the 1,992 existing tests pass without edits other than the fuzz-layout lists).
- On every fixture and policy, `Ok` outputs satisfy the invariants of rule 10 and the canary search of rule 11; every `Err` is the one the table in rules 2 to 7 predicts; there is no `VerifyFailed` on any fixture or seed.
- `All` and `Everything` on the kitchen sink leave exactly the kept structure (and, for `All`, ICC, JUMBF, AROT, trailer, embedded images) and ExifTool's group table agrees; the MPF fixtures' stripped files have patched index fields that ExifTool reads back as expected.
- The ExifTool strip records reproduce with a clean tree; the red-proofs are shown and named.
- Every mutant below is applied by hand, shown to fail at least one named test or guard (the Handback names which), and reverted; the tree after reverting is byte-identical to the build commit.
- Local corpus: every JPEG in `_incoming0` strips under `All` and `Everything` or returns a typed refusal you explain; every `Ok` output has its scan region identical and its audit clean of S; the four MPF files' stripped outputs read in ExifTool with the same `MPImage` count and no new warnings (one file at a time, piped in memory). Counts only in the Handback.
- Gates: fmt (both graphs, `rustfmt --check --edition 2021 fuzz/shared/*.rs`), `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `cargo test --no-fail-fast` on stable and 1.90.0 with `DIAMOND_EXIF_REQUIRE_ORACLE=1 DIAMOND_EXIF_REQUIRE_CLI_FS=1`, `-F unsafe_code` on lib and bin, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `oracle-record` (clean `git status`), 60 s fuzz smokes of `rewrite_jpeg`, `jpeg_segments`, `audit`, `audit_render`, `cli_args`, `cargo build --release --locked --bin exif-audit`.

## Failing Tests

Named tests (a rename, fold or missing test is a deviation to report):

**src/write/plan.rs (unit):** `empty_plan_returns_the_input_unchanged`, `remove_deletes_exactly_the_range`, `overlapping_and_adjacent_removals_merge`, `patch_overwrites_at_the_same_length`, `patch_inside_a_removal_is_rejected`, `range_outside_the_input_is_rejected`, `edit_order_does_not_change_the_output`, `output_is_never_longer_than_the_input` (property over generated plans), `u32_max_offsets_and_lengths_do_not_overflow`.

**src/jpeg (unit):** `segment_range_is_marker_length_and_payload`, `segment_range_overflow_is_none`, `mpf_index_exposes_field_positions_and_byte_order` (big- and little-endian), `mpf_index_marks_which_entries_became_regions`, `second_or_unparsed_mpf_is_not_the_index`.

**src/write/verify.rs (unit; plans built by hand and wrong on purpose):** `a_plan_that_leaves_a_removed_category_fails_check_preview`, `a_plan_that_removes_a_kept_finding_fails_check_kept`, `a_plan_that_cuts_the_scan_fails_check_scan`, `a_plan_that_breaks_the_walk_fails_check_walk`, `a_non_idempotent_result_fails_check_idempotent`, `a_stale_mpf_field_fails_check_mpf_fields`, `a_clean_plan_passes_every_check`.

**tests/strip.rs (integration):**
- Dispatch and refusals: `tiff_png_webp_heic_are_unsupported_naming_the_container`, `empty_one_byte_and_non_jpeg_inputs`, `bad_segment_length_is_unstrippable`, `halted_walk_is_unstrippable`, `missing_eoi_and_missing_sos_are_still_strippable`.
- Presets: `all_removes_exif_xmp_extended_xmp_iptc_and_comments_and_keeps_icc_jumbf_other_embedded_and_trailer`, `everything_keeps_only_structure_and_the_scan`, `the_preview_dispositions_match_what_strip_did_for_every_preset_and_fixture`, `none_policy_is_the_identity`.
- Custom and segment rules: `custom_xmp_removes_xmp_and_extended_xmp_only`, `custom_icc_removes_every_icc_chunk`, `custom_iptc_removes_both_split_app13_segments`, `custom_jumbf_removes_jumbf`, `custom_free_text_removes_comments_when_exif_has_no_free_text`, `custom_free_text_needs_entry_edit_when_exif_has_free_text`, `custom_other_segment_removes_app_segments_and_trailers_but_not_embedded_images`.
- Exif rule: `exif_policies_work_on_a_file_without_exif`, `gps_preset_is_the_identity_on_exif_without_findings_in_its_categories`, `gps_preset_needs_entry_edit_when_exif_has_gps`, `all_pii_needs_entry_edit_and_names_its_categories`, `an_unreadable_exif_block_needs_entry_edit_for_partial_policies`, `all_removes_every_exif_block_including_the_extra_ones`, `orientation_goes_with_the_exif_segment_under_all`.
- Multi-Picture: `everything_removes_the_images_and_the_index_together`, `all_keeps_images_and_index_and_needs_no_patch_when_removals_precede_the_index`, `removal_after_the_index_patches_secondary_offsets_and_primary_size` (big- and little-endian), `removing_trailer_gaps_patches_later_image_offsets`, `custom_embedded_image_removes_images_and_index_and_keeps_trailer_pieces`, `unindexed_mpf_with_a_later_removal_is_mpf_not_indexed`, `unindexed_mpf_with_nothing_removed_after_it_is_fine`, `a_second_mpf_segment_follows_the_same_rule`, `malformed_entries_are_not_patched`, `a_comment_over_a_kept_embedded_image_is_overlap`, `overlapping_images_are_overlap_when_one_is_kept`.
- Invariants: `output_equals_the_input_minus_the_listed_removals_plus_the_listed_patches` (independent recompute, every fixture × policy), `scan_and_kept_segments_are_byte_identical`, `audit_of_the_output_has_no_category_the_policy_removes`, `kept_findings_survive_by_category_and_kind`, `strip_is_idempotent_and_deterministic`, `output_is_never_longer_and_equal_only_for_an_empty_plan`, `removed_canaries_are_gone_and_kept_canaries_remain`, `removals_list_every_removed_region_with_its_category_and_input_range`, `sanitize_returns_the_strip_bytes`, `the_input_is_not_modified`, `no_fixture_or_seed_gives_verify_failed`.

**tests/strip_oracle.rs:** `exiftool_group_table_equals_the_output_records_per_policy_and_fixture` (live and replay), `exiftool_reads_every_output_without_new_warnings`, `mpf_fields_read_back_as_expected_after_all`, `no_mpf_groups_after_everything`, `strip_records_replay_and_reproduce`, `comparator_is_red_when_an_xmp_group_is_left_in`, `..._when_icc_is_missing`, `..._when_an_mp_image_start_is_off_by_the_removed_length`, `..._when_a_phantom_group_is_added`, `..._when_the_exif_segment_was_kept`.

**tests/fuzz_layout.rs and the harness:** `rewrite_jpeg_harness_holds_on_every_seed_and_degenerate_inputs`, `each_rewrite_guard_goes_red_alone_when_planted` (both directions, at least two plants per guard), `rewrite_seeds_are_committed_and_cover_every_fixture_and_policy_class`, `ci_fuzz_smoke_list_includes_rewrite_jpeg`.

## Mutants (each shown red by hand, then reverted)

1. Segment removal range one byte short (leaves the last payload byte). 2. Exif removed when any EXIF9 category is in S (no all-of-EXIF9 test). 3. EXIF9 check forgets `Software` (so `AllPii` removes Exif whole). 4. `Xmp` removes the standard packet but not extended XMP. 5. ICC removed under `All`. 6. Only the first Photoshop APP13 removed. 7. COM removed under `Gps` (FreeText check dropped); or kept under `AllPii`. 8. JFIF removed under `Everything`. 9. The scan region's `EOI` removed with the trailer. 10. Trailer removed under `All`. 11. Embedded images removed but the MPF index kept. 12. MPF index removed but the images kept. 13. MPF secondary offsets not patched. 14. Offset patch subtracts removals that lie before the MPF header start. 15. Primary size field not patched. 16. Patch written in the wrong byte order. 17. `MpfNotIndexed` check removed. 18. Overlap check removed. 19. Merge of overlapping removals counts bytes twice (wrong output length). 20. Self-check (a) skipped. 21. Self-check (c) compares only lengths. 22. Idempotence check skipped. 23. An unreadable Exif block does not trigger `NeedsEntryEdit`. 24. A patch appended at the end (output grows). 25. `BadSegmentLength` not a refusal. 26. `MissingEoi` treated as a refusal. 27. Removal records give output offsets instead of input offsets. 28. TIFF routed into the JPEG planner. 29. `NeedsEntryEdit` reports all of S instead of `S ∩ EXIF9`. 30. The input slice mutated before the copy (e.g. patch applied to the input clone in place and returned as the input).

## Non-Goals

- Editing inside the Exif segment (SPEC-021); TIFF and DNG (SPEC-022); keeping Orientation under `All` (question `strip-keeps-orientation`); XMP or IPTC editing; `set` operations; MakerNote parsing or preview location.
- A write command-line tool or overwriting files (the library returns bytes); PNG, WebP, HEIC; changing the audit, the report schema, exit codes or the CLI.
- Naming or removing bytes the walk skips between segments (question `writer-stray-bytes`); rewriting MPF individual-image attribute IFDs (question `writer-mpf-attribute-fields`).

## Notes for the Implementer

- Order of work, test-first: the plan type and its unit tests; the JPEG accessors (`Segment::range`, `mpf()`); the fixture builders with canaries and their ExifTool-reads-it checks (measure ExifTool's group names for stripped output before writing the group table); the planner for rules 2 to 5 with the integration tests; the Exif rule; the MPF patches and overlap; the self-check last, with its wrong-on-purpose plans; then the oracle records, the fuzz target and harness, the corpus example, the mutants.
- Type the group table, the EXIF9 set, the benign list and the expected positions **by hand** in the tests and harness; none may be imported from the code under test, and the harness stripper must not call into `src/write`.
- `arithmetic_side_effects` forbids unchecked `+`, `-`, `*`: every offset and length is untrusted; use `checked_*`, `get(..)` and `saturating_*` only where a saturated value cannot become a plausible range (prefer an `Err`). Never index.
- The MPF index segment lies before the primary's EOI; its images lie after. Offsets are relative to the byte after `MPF\0`. Keep the three coordinate systems straight: input offsets (removals), output offsets (patch records), MPF-relative offsets (the field values).
- ExifTool's group names for the stripped outputs are facts to measure, not assume. Real files are not fixtures: copy nothing from `_incoming0` or the scratchpad corpus into the repo; the local run is counts only and writes no file.
- Run `lint-fuzz-red-proof` locally; ratio-based timing only; skips are loud.
- Report in the Handback: every deviation, each mutant with the test or guard that caught it, the ExifTool group-table facts measured, the corpus counts, fuzz smoke numbers, a doubt about any DEC-013 rule (a doubt is a question in `guidance/questions.yaml`, not an edit to `decisions/`).
- Do not push or open a PR; do not advance cycles.

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
