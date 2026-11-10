---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-021
  type: story                      # epic | story | task | bug | chore
  cycle: verify  # frame | design | build | verify | ship
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
  decisions: [DEC-005, DEC-008, DEC-009, DEC-012, DEC-013]
  constraints: []                  # [constraint-id-1, constraint-id-2]
  related_specs: [SPEC-019, SPEC-020, SPEC-022]                # [SPEC-NNN]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: [SPEC-020]                     # e.g. [SPEC-002]

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-004's <capability>". Optional; null is acceptable.
value_link: null

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
  tokens_estimate: 1100000
  sessions:
    - cycle: build
      handoff: HANDOFF-041
      agent: claude-sonnet-5-5
      interface: other
      basis: agent-result
      tokens_total: 847188
      estimated_usd: 5.59
      duration_minutes: 122
      recorded_at: 2026-11-10
      notes: "Cost transcribed by the orchestrator from the harness final cumulative subagent_tokens after the second follow-up (847,188; 782,540 after the first, 746,836 at the first build report) at 6.60 USD per Mtok = 5.59 USD. Duration is the sum of the three runs reported by the harness: 3,942,831 + 1,149,869 + 2,246,227 ms = 122.3 min, rounded to 122. Token cumulativity is derived (each resume added tokens in proportion to its tool uses), not confirmed by the harness. Follow-ups: joined-stray-bytes harness fix (56bd4a6, 9315047); verify punch list H1 H2 M1 L1-L5 (1cf8bd1). Nothing was pushed, no PR, no cycle advanced."
    - cycle: verify
      handoff: HANDOFF-042
      agent: claude-opus-5-5
      interface: other
      basis: agent-result
      tokens_total: 315410
      estimated_usd: 2.08
      duration_minutes: 24
      recorded_at: 2026-11-10
      notes: "Cost transcribed by the orchestrator from the harness final subagent_tokens 315,410 at 6.60 USD per Mtok = 2.08 USD; duration 1,434,530 ms = 23.9 min rounded to 24. Verifier cannot see its own token count; the orchestrator transcribes the metered subagent_tokens and duration from the harness. Verdict PUNCH LIST at 7697162: two High leaks on malformed input (straddling out-of-line value not scrubbed; IFD1 thumbnail bytes reachable through a second 0x0201 or through 0x0111 strips left unreferenced), one Medium test gap (rule 3 per-block refusal only tested on the first block), Lows. No code changed, nothing pushed, no cycle advanced."
    - cycle: verify
      handoff: HANDOFF-043
      agent: claude-opus-5-5
      interface: other
      basis: agent-result
      tokens_total: 262089
      estimated_usd: 1.73
      duration_minutes: 20
      recorded_at: 2026-11-10
      notes: "Cost transcribed by the orchestrator from the harness final subagent_tokens 262,089 at 6.60 USD per Mtok = 1.73 USD; duration 1,174,253 ms = 19.6 min rounded to 20. tokens_total, estimated_usd and duration left null on purpose: the verifier cannot see its own token count; the orchestrator transcribes the harness's metered subagent_tokens and duration. Verdict PUNCH LIST at c1c01bd (code 1cf8bd1): one High leak sibling of H2 (IFD1 tile, JPEG-table and free-space offsets left unreferenced and unzeroed when the thumbnail goes), one Medium test gap (rule-3 per-block refusal pinned only when the bad block is last), Lows. No code changed, nothing pushed, no cycle advanced."
  totals:
    tokens_total: 1424687
    estimated_usd: 9.40
    session_count: 3
---

# SPEC-021: JPEG in-place IFD entry deletion with scrub: `Gps`, `GpsAndDevice`, `AllPii` (thumbnail and MakerNote removed whole), never grow, never relocate

## Context

SPEC-020 shipped the writer for whole segments. Its narrow presets (`Gps`, `GpsAndDevice`, `AllPii`) work only for files with nothing to remove inside the Exif segment; otherwise they refuse. On the user's own files that is 4 of 13 JPEGs for `Gps`, 6 of 13 for `GpsAndDevice` and 13 of 13 for `AllPii`: the narrow presets are unusable on real photos until entries can be deleted **inside** the Exif segment. This spec does that.

The constraints come from SPIKE-001 and are unchanged: Exif blocks can already sit at the 64 KB ceiling (never grow), MakerNotes break when moved (4 of 7 Canon and Sony tested; never relocate), and removing an entry without zeroing its out-of-line value leaves the data in the file (scrub). DEC-013 (read it first, **including its 2026-11-10 amendment, which is normative for this spec**) already forbids insert, grow and move; the entry deletion below is a same-length patch, so no new edit kind exists.

Measured on the local corpus while designing (counts only): 13 JPEGs, 0 with an Exif walk issue, 4 with GPS, 5 with a thumbnail, 7 with a MakerNote. Every Exif block is clean, so refusing a block with any issue costs nothing on real files and removes a whole class of unsafe guesses.

## Goal

`strip` with `Gps`, `GpsAndDevice`, `AllPii` and any custom set deletes the Exif entries the audit reports in the policy's categories (a GPS IFD and an IFD1 thumbnail whole, a MakerNote whole), scrubs their values with zero bytes, never changes the length of the Exif segment or moves any other byte, and fails closed otherwise. Shown by: pixel integrity (I1), audit-as-oracle in the library, a test that greps the output for every removed canary, ExifTool agreement at tag level, an independent byte-for-byte prediction in tests and in fuzz, and a logged re-run on the local corpus.

## Inputs

- **Decisions:** DEC-013 with its amendment (normative), DEC-012, DEC-009 (presets, `KeptUnread`), DEC-008 (tag table), DEC-005 (IFD kinds), DEC-001 (tolerant walk).
- **Files to read first:** `src/write/{mod,plan,jpeg,verify}.rs` (SPEC-020), `src/tiff/mod.rs` (`Container`, `Ifd`, `Entry`; note `value_at`), `src/audit/{mod,category}.rs` (`Finding`, `ItemKind::Tag`, `Origin`, `categorize`, `ThumbnailRange`), `src/jpeg/metadata.rs` (`exif_blocks`), `tests/support/strip_shapes.rs`, `tests/strip.rs`, `tests/strip_oracle.rs`, `tests/support/strip_records.rs`, `fuzz/shared/rewrite_invariants.rs`, `examples/strip_corpus.rs`, `guidance/toolchain-brief.md`.
- **Evidence:** SPIKE-001; questions `writer-in-place-delete-and-scrub` (this spec answers it), `strip-keeps-orientation` (stays open; see Non-Goals), `writer-stray-bytes`, `writer-mpf-attribute-fields`.
- **Lessons carried from SPEC-019 and SPEC-020 (do not relearn):** every unreadable-means-refuse rule is written as "the walk reported any issue", never as one counter (SPEC-020's verify found a fail-open through `failed_blocks`); every invariant a fuzz guard can find also has a deterministic unit test; measure ExifTool's behaviour on the fixture **before** writing the expected table; an unchanged-claim is checked against the rule that could move it; fuzz-shared files are formatted with `rustfmt --edition 2021`.

## Outputs

### Library

- **`src/tiff` (additions, read-only accessors; no behaviour change):**
  - `Ifd::table_range() -> Range<usize>`: offset of the count word through the end of the next-IFD word (`offset .. offset + 2 + 12 * entries().len() + 4`, checked); `Ifd::next_at() -> usize` (offset of the next-IFD word).
  - `Entry::value_at() -> usize` and `Container::payload_range(&Entry) -> Option<Range<usize>>`: `Some(range)` of an **out-of-line** payload inside the blob, `None` for an inline value (`byte_len <= 4`) or an unreadable one.
  - Offsets are relative to the TIFF blob; the JPEG layer adds the blob's input offset.
- **`src/jpeg`:** `JpegMetadata::exif_blocks()` already exposes each block; add what the planner needs and nothing more: the blob's input offset, the segment index and whether the segment lies after the first scan (the audit's `Origin` already carries two of them; reuse it).
- **`src/write/exif.rs` (new):** the Exif entry planner (rules below). It produces `Edit::Patch` values for `EditPlan` and a list of `EntryRecord` and scrub ranges. It never produces a `Remove`.
- **`src/write/mod.rs`:** `strip` and `sanitize` keep their signatures. `StripError::NeedsEntryEdit` is **removed**; `StripError::Uneditable { reason: UneditableReason, offset: usize }` is added (`#[non_exhaustive]`). `UneditableReason`: `BlockNotClean`, `TooManyExifSegments`, `InventoryTruncated`, `AfterScan`, `SharedBytes`, `ThumbnailNotLocated`, `ChainAfterThumbnail`, `PointerNotInParent`, `EmptiedIfd` (only if the measurement in rule 9 says it is needed). `Stripped` gains `entries()` (`EntryRecord { block, ifd: IfdKind, ifd_index, tag, field_type, count, offset }`, input coordinates, one per deleted entry **and per entry of a removed child IFD**, in input order) and `scrubs()` (`ScrubRecord { offset, len, reason }`, output coordinates); `PatchRecord` gains `kind: PatchKind { MpfField, IfdTable, Scrub }`. All `#[non_exhaustive]`.
- **`src/write/verify.rs`:** two new checks, `Entries` and `Scrubbed` (rule 11), run after the SPEC-020 checks, in that order after `Walk` and before `Idempotent`. `Check` gains the two variants.
- **No change** to the audit, the report, the CLI, exit codes or the presets' contents.

### Rules (normative; each maps to a named test)

Let S be `policy.spec()`, and EXIF9 as in DEC-013. A policy is **whole** when `EXIF9 ⊆ S` (SPEC-020 behaviour, unchanged: every Exif segment removed), **none** when `S` contains no category that can occur in an Exif entry (EXIF9, `Xmp`, `Iptc`, `Icc`) and Exif is untouched, and **partial** otherwise.

1. **Regression.** Under a whole or none policy the output bytes are identical to SPEC-020's for every existing fixture and committed seed. The only SPEC-020 tests that change are those that expected `NeedsEntryEdit`.
2. **What is deleted (partial).** Walk the audit's findings. Every finding with `ItemKind::Tag` whose category is in S, from an Exif block, is a **target** `(block, ifd_index, entry_index, tag)`; the audit is the one definition of what is in the file. Two derived targets are added: if `Gps ∈ S`, every GPS IFD (the IFD of kind `Gps`) is a **child removal**; if `Thumbnail ∈ S`, the IFD of kind `Ifd1` is a child removal. A `Gps`-kind IFD's own entries are all `Gps` findings and are removed with the IFD, not one by one. `Interop` IFDs and every uncategorised entry (Orientation, exposure, colour space, pixel dimensions, ...) are never targets.
3. **Preconditions per Exif block that has any target or could hold one.** Re-parse the block's blob with the tolerant walk. The block must be **clean**: no walk issue of any kind, not a failed block, the inventory not truncated, at most 16 Exif segments in the file (the audit parses 16), and the segment must not lie after the first scan (a patch there would change the scan region; DEC-013 part 6 and I1). Otherwise `Uneditable { BlockNotClean | TooManyExifSegments | InventoryTruncated | AfterScan, offset }` for **every** partial policy, whether or not a finding is visible. (A GPS IFD with entry count `0xFFFF` gives zero GPS findings and one issue: it must be refused, not returned unchanged.)
4. **Locating bytes.** Entry *i* of IFD *g* lies at `blob + g.offset + 2 + 12 * i`; its 4-byte value field at `+ 8`. The planner **verifies** that the bytes it patches are the bytes the walk read (tag, type, count equal the entry's) and refuses with `BlockNotClean` otherwise, rather than trusting the arithmetic.
5. **Deleting entries from one IFD.** For an IFD with *n* entries of which *k > 0* are deleted: one `Patch` over `table_range()`. New bytes: the count word `n - k` in the block's byte order; the *n - k* surviving 12-byte entries, copied verbatim, in their original order; the next-IFD word (the original word, or the replacement from rules 6 and 7), immediately after them; then `12 * k` zero bytes. Out-of-line values and every other IFD stay where they are. If the IFD would become empty (`n == k`), see rule 9.
6. **GPS child removal.** For each GPS IFD: (a) its pointer entry (tag `0x8825`) is deleted from the parent IFD by rule 5 (the pointer entry is located through the walk's `parent()` and the entry whose tag is `0x8825` and whose value equals the GPS IFD's offset; none found is `PointerNotInParent`); (b) the GPS IFD's table region and every out-of-line value of its entries are scrubbed (rule 8). Its entries get `EntryRecord`s with the GPS IFD's index.
7. **Thumbnail child removal.** `Thumbnail ∈ S` and an `Ifd1` exists: it must have `ThumbnailRange.range` (tags `0x0201` and `0x0202` readable, inside the blob) else `ThumbnailNotLocated`; its next-IFD word must be 0 else `ChainAfterThumbnail`. Then IFD0's next-IFD word becomes 0 (in IFD0's table patch; rule 5 applies even when IFD0 loses no entry), and IFD1's table region, its out-of-line values and the thumbnail bytes `range` are scrubbed. A strips-form thumbnail (finding on `0x0111`, no `0x0201`) is `ThumbnailNotLocated`.
8. **Scrub and the keep set.** The scrub of a deleted entry is its out-of-line payload range (`payload_range`); of a child IFD, its table region and its entries' payload ranges, and for the thumbnail the thumbnail range. Each scrub range becomes a `Patch` of zero bytes of the same length. Let **K** be the union of: the 8-byte TIFF header; every kept entry's payload range; every kept IFD's table region (after the patch, so the *shape* of the IFD stays in K even though its entry bytes move); the Interop IFDs; the byte range of every kept thumbnail. A scrub range that intersects K, a table patch that intersects another IFD's table or a kept payload, or two *different* IFDs claiming the same table region, is `Uneditable { SharedBytes, offset }` with the first such offset. Two deleted entries whose payloads are the same range scrub it once. Scrub ranges lie inside the blob (guaranteed by a clean walk; the planner checks again with checked arithmetic).
9. **Emptied IFD.** An IFD that would be left with zero entries: **measure first** (builder) how the tolerant walk and ExifTool treat a count-0 IFD0, Exif IFD and chain IFD, and pin the result in a test and in the Handback. If the walk calls it clean and ExifTool reads the file without a new warning, it is allowed (count word 0, next word, zeros). If either objects, refuse with `Uneditable { EmptiedIfd }` for IFD0 and the chain, and delete the pointer entry plus scrub the IFD for an emptied Exif IFD. Whichever result is measured, the self-check must pass; no `VerifyFailed` on any seed.
10. **Interaction with segment rules (SPEC-020 rules 3, 5, 6, 7 and MPF).** Exif patches lie wholly inside kept Exif segments and never touch a length word, a marker or the scan. They are independent of every removal and of the MPF field patches: patch offsets in `Stripped` are output coordinates (shifted by the removals before them, as SPEC-020 does). Segment removals (COM under `FreeText`, XMP under `Xmp`, ...) still apply in the same call. The refusal order is DEC-013's: `Unstrippable`, `Uneditable`, `MpfNotIndexed`, `Overlap`, then `VerifyFailed`.
11. **Self-check additions (library, always).** (g) **`Entries`:** the inventory of tag items of the output (`audit(output).items()` of kind `Tag`, with value bytes) equals the input's inventory minus the targets and the entries of removed child IFDs, in order; compared by `(block, ifd kind, tag, field_type, count, value bytes)`. (h) **`Scrubbed`:** every recorded scrub range reads all zero in the output. The inventory being truncated is a refusal (rule 3), so (g) is always complete. Check 7(b)'s exemption (Exif-resident findings of kept categories left out of the comparison) applies **only** when the whole Exif segment was removed.
12. **Length and idempotence.** With no segment removal, `output.len() == input.len()`; with removals, DEC-013 part 1's bound. `strip(output, policy)` is an empty plan. Re-running with a wider policy works on the output.
13. **Records.** `entries()` lists targets and child-IFD entries in input order; `scrubs()` and `patches()` list every patch with its kind and output offset; `removals()` is as in SPEC-020. The report layer is untouched.
14. **Honest limit.** A `MakerNote` or thumbnail that stays under a policy that does not name it is kept byte for byte; nothing is read inside it (DEC-009 `KeptUnread`). A kept Canon-style MakerNote with absolute offsets is byte-identical after a `Gps` strip (it did not move).

**Policy results to expect (a table the tests pin; kitchen-sink Exif):** `Gps`: GPS IFD and pointer gone, every other entry and the thumbnail kept. `GpsAndDevice`: plus ImageUniqueID, BodySerialNumber, LensSerialNumber, CameraSerialNumber. `AllPii`: plus Authorship, FreeText (entries and `COM` segments), Timestamps, CameraModel, the IFD1 thumbnail and the MakerNote; `Software` (tag 0x0131) and Orientation stay; Exif-resident `Xmp`/`Iptc`/`Icc` tags stay. Custom `{Xmp}`: tag `0x02BC` inside Exif is deleted and scrubbed, and XMP segments are removed, all Exif entries of other categories stay (this resolves question `dec-013-exif-resident-xmp-iptc-icc-tags`).

### Fixtures (extend `tests/support/strip_shapes.rs`; test code only)

A builder that records its own layout (every entry's offset, every value's offset, every canary's offset), so tests compute the expected bytes **without calling the library or the walk**:

- `exif_full_le` and `exif_full_be`: IFD0 (Make, Model, Software, DateTime, Artist, Copyright, ImageDescription, Orientation, XResolution, ExifIFD pointer, GPS pointer), Exif IFD (DateTimeOriginal, ExposureTime, UserComment, BodySerialNumber, LensModel, MakerNote 64 bytes out of line, ColorSpace, Interop pointer), Interop IFD, GPS IFD (6 entries; lat and lon rationals out of line), IFD1 with a JPEG thumbnail (a canary inside) and Compression/XResolution. Every string and the MakerNote carry a unique canary; every inline value (Orientation, ColorSpace) too.
- Variants: `exif_first_entry_target`, `exif_last_entry_target`, `exif_every_entry_target` (IFD empties), `exif_64k` (block at the ceiling), `exif_makernote_canon_style` (absolute offsets inside), `exif_subifd` (a `Sub` IFD with a targeted tag), `exif_chain_ifd2` (IFD2 after IFD0 and IFD1), `exif_two_blocks` (two segments, only one targeted), `exif_xmp_tag_inside` (tag 0x02BC in IFD0), `exif_no_gps_no_thumb`.
- **Leak inputs (each must refuse or be correct, never `Ok` with a canary left):** `gps_count_overflow` (SPEC-020's), `gps_ifd_pointed_to_twice`, `value_shared_with_kept_entry`, `value_inside_kept_ifd_table`, `value_overlaps_tiff_header`, `thumbnail_range_overlaps_makernote`, `thumbnail_strips_form`, `thumbnail_with_ifd2_after_it`, `gps_pointer_missing_in_parent`, `exif_after_scan`, `exif_17_segments`, `exif_inventory_over_budget`, `exif_walk_issue_in_a_kept_ifd` (an issue in an IFD with no target still refuses a partial policy), `ifd0_emptied`.

### Oracle (extend `tests/strip_oracle.rs`; ExifTool run, never read or linked)

- Command `exiftool -a -u -G1 -s -t -n` (SPEC-020's), **tag level**: for each fixture and each policy recorded, the **expected tag set per group is declared by hand in the test** (not computed from the library), e.g. after `Gps`: no `GPS` group; `IFD0:Orientation`, `ExifIFD:BodySerialNumber`, `IFD1:ThumbnailImage` present. Compared: tag-name sets per group, **and the values of every kept tag equal the input's ExifTool values** (so a shifted entry that still names the right tag but reads garbage fails). `Composite` tags derived from a removed source (GPSLatitude, GPSPosition, `SubSecDateTimeOriginal`, ...) are declared as expected losses; any other Composite difference fails.
- No new warning lines in any output other than the declared ones (digit-run normalisation as in SPEC-020, but a *new* warning kind fails).
- **Measure before writing expectations** (SPEC-020's lesson): record ExifTool's output for the input fixtures and for the library's outputs, read it, and only then write the declared tables; the Handback lists what was measured (group names for an emptied IFD, for IFD1 after unlink, whether `File:ExifByteOrder` survives).
- Red-proofs (each shown red, control first): a deleted tag still present; a kept tag missing (Orientation); `GPS` group left; a kept value changed; `ThumbnailImage` left; a phantom tag added.
- Records under `tests/oracle-fixtures/exiftool-strip/` extended (new files per fixture and policy), reproduced by `just oracle-record` with an empty `git status`; CI's `oracle-records` job already diffs the directory.

### Fuzz and seeds

- Extend `rewrite_jpeg` (no new target): `fuzz/shared/rewrite_invariants.rs` gains an **independent naive TIFF reader and entry editor** (own code: header, IFD0 chain, Exif/GPS pointers, bounds, the disjointness check; it does not call the library walk) and these guards, each with a plant and a deterministic unit test in `tests/fuzz_layout.rs` or the harness tests:
  - `EntryPrediction`: when the library returns `Ok` and the naive reader finds the touched block clean, the output's Exif segment equals the naive prediction **byte for byte**.
  - `LibraryOkNaiveUnclean`: the library returned `Ok` for a partial policy on a block the naive reader finds unclean (cycle, range out of the blob, overlapping ranges, an IFD referenced twice) is a failure. The other direction (library refuses, naive accepts) is fine: fail closed.
  - `ScrubbedZero`, `KeptEntriesIntact` (naive inventory of kept entries equals the input's minus targets, byte-wise), `LengthUnchanged` (no segment removal implies equal length), `ScanIdentical` (SPEC-020's).
  - `VerifyFailed` stays a failure except the two SPEC-020 excuses.
- Seeds: at least **60 new** under `fuzz/seeds/rewrite_jpeg/`, regenerated by `examples/fuzz_seeds.rs`: every new fixture and leak input under policy bytes 0 (`Gps`), 1 (`GpsAndDevice`), 2 (`AllPii`) and the custom-mask byte with masks for `{Xmp}`, `{Software}`, `{Timestamps, Gps}`, `{MakerNote}`, `{Thumbnail}`; the `All`-preset family for the leak inputs; `tests/fuzz_layout.rs` pins the count and the classes. 60 s smoke for `rewrite_jpeg`, `jpeg_segments`, `audit` stays in CI.

### Local corpus (a log, not committed; `examples/strip_corpus.rs` extended; counts and 8-hex short hashes only, nothing written to disk)

Run under `Gps`, `GpsAndDevice`, `AllPii` on `_incoming0`: counts of Ok / `Uneditable` by reason (expected 13 Ok each: all 13 JPEG Exif blocks are clean), bytes patched, scan region identical 13/13, output audit has no finding of the policy's categories 13/13, output length equal to input length under `Gps` and `GpsAndDevice` 13/13. ExifTool by pipe, one file at a time, in memory: the `GPS` group is gone from exactly the 4 files that had it; `IFD0:Orientation` count before and after is equal; the thumbnail group is gone after `AllPii` for exactly the files that had it; zero new warning lines; every MPF file keeps its 3 `MPImage` groups. A canary-style search is impossible on real files, so the log instead records that every deleted entry's former payload range reads zero in the output (counted).

## Acceptance Criteria

- [ ] Rules 1 to 14 hold, each with a named test below; `NeedsEntryEdit` no longer exists; `Uneditable` carries a reason and an offset.
- [ ] I1 holds: scan region and every kept segment byte-identical; the Exif segment length unchanged; outside the recorded patches and removals the output equals the input (a test recomputes the expected output from the fixture layout, not from the library).
- [ ] Canary test: for every fixture and policy, no deleted entry's canary appears anywhere in the output, and every kept canary does.
- [ ] Every leak input refuses with the named reason (or, where rule 9 says so, strips correctly); none returns `Ok` with a canary left.
- [ ] ExifTool agrees at tag level, with six red-proofs.
- [ ] `rewrite_jpeg` guards hold on every seed; each guard goes red alone when planted (two plants each); one plant is repeated against the real libFuzzer target from committed seeds.
- [ ] 30 mutants (below) each fail at least one named test or guard.
- [ ] Local corpus numbers logged in the Handback; no copy, no derived file of any photo.
- [ ] Gates green: fmt (both graphs and `fuzz/shared`), `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `cargo test --no-fail-fast` on stable and 1.90.0 with `DIAMOND_EXIF_REQUIRE_ORACLE=1 DIAMOND_EXIF_REQUIRE_CLI_FS=1`, `-F unsafe_code`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `oracle-record` clean, five 60 s smokes.
- [ ] No `unsafe`, no `allow` outside test modules, no unwrap, expect, indexing, panic or unchecked arithmetic in `src/write/`; no dependency added; `decisions/` and `scripts/` untouched by the build.

## Failing Tests

Written first, red, in design order. Names are the contract.

**src/tiff (unit):** `ifd_table_range_covers_count_entries_and_next_word`, `ifd_table_range_overflow_is_none`, `payload_range_is_some_only_for_out_of_line_values`, `payload_range_of_an_unreadable_payload_is_none`.

**src/write/exif.rs (unit; one synthetic IFD per test, built by hand, both byte orders where noted):** `delete_a_middle_entry_shifts_and_zeroes_the_tail` (le, be), `delete_the_first_entry`, `delete_the_last_entry`, `delete_two_non_adjacent_entries`, `delete_every_entry_leaves_count_zero_or_refuses_per_the_measurement`, `no_target_means_no_patch`, `next_ifd_word_follows_the_surviving_entries`, `inline_value_entry_leaves_no_trace_in_the_table`, `out_of_line_value_is_zeroed`, `two_deleted_entries_sharing_one_payload_scrub_it_once`, `value_shared_with_a_kept_entry_is_shared_bytes`, `value_inside_a_kept_ifd_table_is_shared_bytes`, `value_overlapping_the_tiff_header_is_shared_bytes`, `a_patched_entry_that_is_not_the_walks_entry_is_block_not_clean`, `gps_child_removal_deletes_the_pointer_and_zeroes_the_ifd_and_values`, `gps_ifd_pointed_to_twice_is_refused`, `gps_pointer_missing_from_the_parent_is_pointer_not_in_parent`, `thumbnail_removal_sets_the_ifd0_next_word_to_zero_and_zeroes_ifd1_and_the_jpeg`, `thumbnail_with_a_following_ifd_is_chain_after_thumbnail`, `strips_form_thumbnail_is_thumbnail_not_located`, `thumbnail_range_overlapping_a_kept_value_is_shared_bytes`, `makernote_is_deleted_whole_and_its_value_zeroed`, `makernote_kept_is_byte_identical_including_absolute_offsets`.

**src/write/verify_tests.rs (unit; plans wrong on purpose):** `a_plan_that_corrupts_a_kept_entry_value_fails_check_entries`, `a_plan_that_deletes_a_kept_entry_fails_check_entries`, `a_plan_that_leaves_a_deleted_entry_fails_check_preview`, `a_scrub_that_leaves_a_byte_fails_check_scrubbed`, `entries_check_compares_type_count_and_value_not_only_the_tag`, `an_empty_plan_on_an_input_with_targets_is_still_refused`, `the_seven_old_checks_still_pass_and_fail_as_before`.

**tests/strip.rs (integration):** `gps_removes_the_gps_ifd_and_keeps_everything_else`, `gps_and_device_removes_the_four_device_ids_too`, `all_pii_matches_the_policy_table_entry_by_entry`, `all_pii_keeps_software_and_orientation_and_exif_resident_xmp`, `custom_xmp_deletes_the_exif_resident_xmp_tag_and_the_segments`, `custom_software_only`, `custom_timestamps_and_gps`, `custom_makernote_only`, `custom_thumbnail_only`, `output_length_equals_input_length_when_only_exif_entries_go`, `exif_64k_block_does_not_grow_and_stays_valid`, `no_removed_canary_survives_anywhere_in_the_output` (every fixture, every policy), `every_kept_canary_survives`, `the_output_equals_the_input_outside_the_recorded_patches_and_removals`, `the_expected_output_computed_from_the_fixture_layout_equals_the_library_output` (independent of the library and the walk), `scan_region_and_kept_segments_are_byte_identical`, `the_output_audit_has_no_finding_in_the_policy_and_the_same_kept_findings`, `strip_is_idempotent_and_a_wider_policy_composes`, `whole_and_none_policies_are_byte_identical_to_spec_020_outputs`, `a_block_with_a_walk_issue_is_uneditable_for_every_partial_policy` (the 0xFFFF GPS count, a bad IFD1 pointer, a bad Exif pointer), `a_walk_issue_in_an_ifd_with_no_target_still_refuses`, `exif_after_the_first_scan_is_uneditable`, `seventeen_exif_segments_are_uneditable`, `an_inventory_over_budget_is_uneditable`, `two_exif_blocks_only_the_targeted_one_changes`, `chain_ifd_and_sub_ifd_targets_are_deleted`, `stripped_records_list_every_entry_scrub_and_patch_with_kinds`, `uneditable_errors_name_reason_and_offset_and_return_no_bytes`, `the_refusal_order_is_unstrippable_uneditable_mpf_not_indexed_overlap_verify_failed`, `mpf_patches_and_exif_patches_compose_in_one_call`, `a_com_segment_and_an_exif_entry_go_in_the_same_call`.

**tests/strip_oracle.rs:** `exiftool_tag_sets_equal_the_declared_tables_per_policy_and_fixture` (live and replay), `exiftool_values_of_kept_tags_equal_the_inputs`, `exiftool_reads_every_output_without_new_warning_kinds`, `emptied_ifd_behaviour_is_pinned` (the rule 9 measurement), `strip_records_replay_and_reproduce`, `comparator_is_red_when_a_deleted_tag_is_left`, `..._when_a_kept_tag_is_missing`, `..._when_the_gps_group_is_left`, `..._when_a_kept_value_changed`, `..._when_the_thumbnail_image_is_left`, `..._when_a_phantom_tag_is_added`.

**tests/fuzz_layout.rs and the harness:** `rewrite_jpeg_harness_holds_on_every_seed_including_the_entry_seeds`, `each_entry_guard_goes_red_alone_when_planted` (both directions, two plants per guard), `the_naive_tiff_reader_agrees_with_the_walk_on_every_clean_fixture`, `entry_seeds_are_committed_and_cover_every_fixture_leak_input_and_policy_family`, `seed_count_is_pinned`.

## Mutants (each shown red by hand, then reverted; the Handback maps each to the test or guard that caught it)

1 count word not decremented. 2 count decremented by one too many. 3 surviving entries copied in reverse. 4 next-IFD word left at the old position. 5 freed tail not zeroed. 6 entry copied without its value field. 7 a deleted entry's out-of-line value not scrubbed. 8 scrub one byte short. 9 scrub range uses the count instead of the byte length (count x size forgotten). 10 inline value treated as out-of-line. 11 GPS pointer entry left in the parent. 12 GPS IFD table not zeroed (values only). 13 GPS IFD removed but not its values. 14 IFD0 next-IFD word left pointing at IFD1 after the thumbnail went. 15 thumbnail bytes not zeroed. 16 IFD1 table not zeroed. 17 thumbnail removal allowed with a chain after it. 18 MakerNote kept when `MakerNote ∈ S`. 19 MakerNote removed under `Gps`. 20 `SharedBytes` check removed. 21 `SharedBytes` compares only kept entries, not kept tables. 22 block-clean precondition reduced to `failed_blocks == 0` (the SPEC-020 defect). 23 `AfterScan` check removed. 24 big-endian block patched with little-endian words. 25 `Entries` self-check compares tags only. 26 `Scrubbed` self-check skipped. 27 Exif patch offsets reported in input coordinates. 28 Orientation treated as a target. 29 the check-7(b) exemption applied for partial policies too. 30 whole policy routed into the entry planner (Exif entries edited instead of the segment removed).

## Non-Goals

- Keeping or re-inserting Orientation under `All` and `Everything` (they still remove every Exif segment whole; question `strip-keeps-orientation` stays open. Entry editing makes it a small policy change: delete the categorised entries and keep the rest. Decide it before SPEC-022, not here).
- TIFF and DNG (SPEC-022: the same planner against a bare TIFF with the extra rule that strip and tile offsets are never touched); XMP property editing; opening or editing inside a MakerNote or a thumbnail; growing, relocating or reordering anything; inserting a new tag; rewriting the Exif segment length; a CLI or report change; the audit's APP1 signature variants (question `audit-app1-signature-variants`); `writer-stray-bytes`; `writer-mpf-attribute-fields`.
- Editing an Exif segment that lies after the first scan, a block with any walk issue, or a strips-form thumbnail (all refuse, by design).

## Notes for the Implementer

- **Order of work, red first at every step:** (1) the `tiff` accessors; (2) `exif.rs` rewriting one hand-built IFD (rules 5 and 8) with the unit tests above, both byte orders; (3) the keep set K and `SharedBytes`; (4) GPS child removal; (5) thumbnail child removal; (6) integrate into the planner, replace `NeedsEntryEdit` by `Uneditable` and rewrite the SPEC-020 tests that expected it; (7) the `Entries` and `Scrubbed` checks; (8) **measure ExifTool** on the new fixtures (and rule 9's emptied IFD) and only then write the declared tag tables and records; (9) the harness (naive reader and guards), seeds, CI list; (10) the 30 mutants; (11) the local corpus, counts only.
- Build the expected bytes from the **fixture's layout record**, never from the walk or the library; that is what makes the tests an oracle.
- Refuse on doubt: every ambiguous input is an `Uneditable`, not a guess. A refusal on a real file would be news (none is expected on the 13 local JPEGs); report any in the Handback.
- Privacy for `_incoming0`: read in place, copy nothing, write no derived file, no temp file, no saved ExifTool output (pipes only); record only counts and 8-hex short hashes. The verifier follows the same rule.
- Do not touch `decisions/`, `scripts/`, dependencies, the audit's categories or the presets. Doubts go to `guidance/questions.yaml`, with the leak input that raised them.
- Cost expectation: L, about the size of SPEC-020 (tokens_estimate 1,100,000 over build, verify and follow-up).

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
