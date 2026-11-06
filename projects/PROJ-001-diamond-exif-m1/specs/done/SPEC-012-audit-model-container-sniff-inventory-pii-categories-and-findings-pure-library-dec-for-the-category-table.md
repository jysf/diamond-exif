---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-012
  type: story                      # epic | story | task | bug | chore
  cycle: ship  # frame | design | build | verify | ship
  blocked: false
  priority: medium                 # critical | high | medium | low
  complexity: M                    # XS | S | M | L | XL | XXL — the EXPECTED size, set at design
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
  stage: STAGE-003
repo:
  id: diamond-exif

handoff:
  from_agent: claude-opus-5-5  # from .repo-context tier_map.design (DEC-005)
  to_agent: null                   # filled when HANDOFF is created (any agent — see docs/porting.md)
  created_at: null

references:
  decisions: [DEC-002, DEC-005, DEC-006, DEC-007, DEC-008]                    # [DEC-NNN, DEC-MMM]
  constraints: [panic-free-library, forbid-unsafe-code, no-io-in-library, fuzz-target-per-surface, test-before-implementation, oracle-must-be-shown-red, no-new-top-level-deps-without-decision]                  # [constraint-id-1, constraint-id-2]
  related_specs: [SPEC-013, SPEC-014, SPEC-015, SPEC-016]                # [SPEC-NNN]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: []                     # e.g. [SPEC-002]

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-003's <capability>". Optional; null is acceptable.
value_link: "STAGE-003: a settled, tested vocabulary for what is personal in a file, reported without ever calling an unreadable or unsupported file clean"

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
  tokens_estimate: 450000
  sessions:
    - cycle: build
      agent: claude-sonnet-5-5
      interface: other
      tokens_total: 426991
      estimated_usd: 2.82
      duration_minutes: 33
      recorded_at: 2026-11-05
      notes: "tokens transcribed by the orchestrator from the sub-agent metered result (first round)"
    - cycle: build
      agent: claude-sonnet-5-5
      interface: other
      tokens_total: 20575
      estimated_usd: 0.14
      duration_minutes: 3
      recorded_at: 2026-11-05
      notes: "fix round applying the DEC-008 amendment (resumed builder, before verify). The harness reported a cumulative 447,566 for the resumed agent; this is the 20,575 above the 426,991 already recorded for the first round."
    - cycle: verify
      agent: claude-opus-5-5
      interface: other
      tokens_total: 243756
      estimated_usd: 1.61
      duration_minutes: 12
      recorded_at: 2026-11-05
      notes: "verdict APPROVED at 36f0022; tokens transcribed by the orchestrator from the sub-agent metered result"
  totals:
    tokens_total: 691322
    estimated_usd: 4.57
    session_count: 3
shipped_at: 2026-11-05
---

# SPEC-012: Audit model: container sniff, inventory, PII categories and findings (pure library); DEC for the category table

> **OUTLINE — `cycle: frame`.** This spec exists so its ID is stable and
> siblings can declare `depends_on: [SPEC-012]`. Capture **scope** (Context /
> Goal / Non-Goals) and **dependencies** only — the *approach* is designed
> just-in-time when this moves to `design`. Do not pre-design it here.

## Context

STAGE-003's first spec. `exif-core` reads TIFF and JPEG and its parity with ExifTool is shown, but nothing says what is *in* a file in privacy terms. This spec adds the **audit model**: a pure function from bytes to a `FileAudit` holding the sniffed container, an inventory of everything the readers found, and PII findings by category. The categories, their severities and the tag table are DEC-008 (written in this design, normative; read it first). Later specs build on this shape: SPEC-013 checks the inventory against ExifTool, SPEC-014 derives strip-policy presets and a preview from the category table, SPEC-015 renders it, SPEC-016 wraps it in a binary. Design section 5's `StripPolicy` names are the reason the table exists now, before any writer.

Three design questions this spec settles:

1. **Several Exif segments** (question `several-exif-segments`). The audit reports findings from **every parsed Exif block**, each tagged with its block index and whether it lies after the scan. It does not merge: a privacy report must show every copy, and a stripper must remove every copy. `JpegMetadata::exif()` (first block) is unchanged; the library does not gain a merged view.
2. **Unreadable is not clean.** A file the sniff recognises but whose metadata cannot be read (bad IFD0, a fatal Exif block, a truncated JPEG) is `failed_blocks > 0` / `clean == false`, never an empty successful audit. A container the library does not read (PNG, WebP, HEIC, GIF, BigTIFF, anything else, an empty file) is `Unsupported`, never "no findings".
3. **Thumbnails** (question `audit-thumbnail-recursion`). A thumbnail is a `Thumbnail` finding with its byte range and a `starts_with FF D8` flag recorded. The audit does **not** read inside it in this spec; the question stays open for a later spec with a measured case.

**Not decided here, measured and reported:** `Container` stores every IFD's entries and a hostile TIFF can name up to 512 distinct overlapping IFDs of up to 65,535 entries each inside a small file. The build measures the memory and time of the worst such file against `Container::parse` *today* and reports it (this may be an existing amplification, to be fixed in a TIFF spec by me); this spec bounds the **inventory** it adds on top (below).

## Goal

`audit(&[u8]) -> FileAudit` never fails and never panics: it sniffs the container, reads TIFF or JPEG with the existing tolerant readers, and returns the inventory, the findings (DEC-008) with locations, and a health summary, all bounded by documented constants. A bare TIFF and the same TIFF inside a JPEG give the same inventory and findings (container independence at this layer too).

## Inputs

- **Reads:** `src/tiff/mod.rs` (`Container`, `Ifd`, `Entry`, `IfdKind`), `src/jpeg/{mod,metadata}.rs` (`JpegMetadata`, `ExifBlock`, `Region`, `Segment::after_scan`), `src/tiff/tags.rs`, `src/error.rs`, DEC-008, DEC-005, DEC-006, DEC-007
- **Does not read:** ExifTool source (run, never read)
- Constraints: `panic-free-library` (five-lint deny set), `forbid-unsafe-code`, `no-io-in-library`, `fuzz-target-per-surface`, `test-before-implementation`, `oracle-must-be-shown-red`, `no-new-top-level-deps-without-decision` (no dependencies)

## Outputs

### Public API (new module `exif_core::audit`, files `src/audit/{mod,sniff,category}.rs`)

```rust
pub fn audit(data: &[u8]) -> FileAudit<'_>;          // total: never Err, never panics

pub enum Sniffed { Jpeg, Tiff, BigTiff, Png, Webp, Heic, Gif, Unknown, Empty }
pub fn sniff(data: &[u8]) -> Sniffed;                // magic bytes only, no parsing

pub enum ContainerKind { Tiff, Jpeg, Unsupported(Sniffed) }

#[non_exhaustive] pub enum Category { Gps, DeviceId, Authorship, FreeText, Thumbnail, MakerNote,
                                      Xmp, Iptc, Jumbf, Timestamps, CameraModel, Software, Icc }
pub enum Severity { Low, Medium, High }              // Ord: Low < Medium < High
impl Category { pub fn severity(self) -> Severity; pub fn name(self) -> &'static str; }
pub fn categorize(kind: IfdKind, tag: u16) -> Option<Category>;   // the DEC-008 table; None = inventory only

pub struct Origin { pub block: Option<usize>,     // Exif block index in a JPEG, None for a bare TIFF
                    pub segment: Option<usize>,   // JPEG segment index of that block
                    pub after_scan: bool }
pub struct Item<'a> { /* one inventory entry */ }
impl<'a> Item<'a> {
    pub fn origin(&self) -> Origin;
    pub fn kind(&self) -> ItemKind;               // Tag { ifd: IfdKind, ifd_index: usize, tag: u16, field_type: u16, count: u32 } | Region { kind: RegionKind, segment: usize, len: usize }
}
pub struct Finding { pub category: Category, pub item: usize /* index into items() */ , /* + ThumbnailRange when category == Thumbnail */ }

pub struct Health { pub clean: bool, pub jpeg_issues: usize, pub exif_issues: usize,
                    pub failed_blocks: usize, pub inventory_truncated: bool }

impl FileAudit<'_> {
    pub fn kind(&self) -> ContainerKind;
    pub fn items(&self) -> &[Item<'_>];
    pub fn findings(&self) -> &[Finding];
    pub fn health(&self) -> &Health;
    pub fn max_severity(&self) -> Option<Severity>;
    pub fn findings_in(&self, c: Category) -> impl Iterator<Item = &Finding>;
}
pub const MAX_INVENTORY_ITEMS: usize = 1 << 16;    // 65,536; exceeding it sets inventory_truncated
```

(Names and exact shapes are the builder's to refine inside these constraints; the observable rules below and the named tests are normative. `Item` holds no value bytes: counts and types only, so the model stays small and nothing hostile is copied out. Renderers that need a value call the existing decoders on the borrowed container, SPEC-015's business.)

### Rules (normative)

1. **Sniff.** `FF D8` is `Jpeg`. `49 49 2A 00` or `4D 4D 00 2A` is `Tiff`; `49 49 2B 00` / `4D 4D 00 2B` is `BigTiff`; the 8-byte PNG signature is `Png`; `RIFF`....`WEBP` is `Webp`; bytes 4..8 `ftyp` is `Heic` (any ISOBMFF brand: the audit does not read it, so it does not distinguish AVIF); `GIF87a`/`GIF89a` is `Gif`; zero bytes is `Empty`; anything else `Unknown`. Sniffing never reads past the first 12 bytes. A `Tiff` sniff but `Container::parse` failing is a parse failure (rule 6), not `Unsupported`.
2. **TIFF.** `Container::parse`; every IFD's entries become `Item::Tag`s in container order (IFD order, then entry order), with `Origin { block: None, .. }`. Pointer tags (`0x8769`, `0x8825`, `0xA005`, `0x014A`) are listed in the inventory but are never findings.
3. **JPEG.** `jpeg::read`; for **each** `ExifBlock` whose container is `Ok`, its entries become items with `Origin { block: Some(i), segment: Some(s), after_scan }`. Regions become `Item::Region`s in segment order. The same tag in several blocks gives several items and several findings (never deduplicated).
4. **Findings.** For every `Tag` item: if `ifd == Gps`, one `Gps` finding per item; otherwise `categorize(kind, tag)` decides (DEC-008 table; `0x0201` only in an `Ifd1` is `Thumbnail`; `0x0111` in an `Ifd1` with no `0x0201` in that IFD is `Thumbnail`). For every `Region` item the DEC-008 region mapping. A `Thumbnail` finding carries the thumbnail's `(offset, length)` when `0x0201` and `0x0202` are present, SHORT/LONG with count 1 and the range lies inside the TIFF blob, plus `starts_with_ffd8`; otherwise it carries `range: None`. Findings are in item order, so they are deterministic.
5. **Bounds.** At most `MAX_INVENTORY_ITEMS` items are kept; further entries set `health.inventory_truncated`, are not listed, **but are still classified**: a finding is never lost to truncation (findings are bounded separately by `MAX_INVENTORY_ITEMS` too, and their count beyond it sets the same flag). Work is linear in the entries the reader already produced. The build must also report the worst-case `Container::parse` memory/time measured as in Context.
6. **Health.** `clean` is true iff the container parsed with no `ParseIssue`/`JpegIssue`, no failed block, no truncation. `failed_blocks` counts Exif blocks with a fatal TIFF error; for a bare TIFF whose IFD0 is unreadable the audit has `kind() == Tiff`, no items, `failed_blocks = 1`, `clean = false`. For `Unsupported` kinds `clean` is false and there are no items; a caller can never read "no findings" as "safe" without also checking `kind()` and `health()`.
7. **No I/O, no value copies, no dependencies.** Items borrow nothing from value bytes; the struct may borrow the input (`'a`) only if the builder finds a need.
8. **Container independence.** For every `oracle_tiff` fixture, `audit(bare_tiff)` and `audit(jpeg_wrapping(tiff))` have the same sequence of `(ifd kind, ifd index, tag, type, count)` and the same `(category, item position)` findings; origins differ only by block/segment.

### `categorize` is the DEC-008 table

The function is a `match` on `(kind, tag)` with no other source. A test iterates the DEC-008 table (copied into the test as data, deliberately not generated from the code) and asserts each row; another asserts no id outside the table (sampled exhaustively over `0..=65535` for each non-GPS kind) is categorised.

### Live check of the table (oracle)

For each distinct tag id in the table, the test builds a one-entry TIFF (type chosen per tag: ASCII for text tags, UNDEFINED for `UserComment`/`MakerNote`, BYTE for the XP tags, etc.; builder in `tests/support`, independent of `src/`), runs `exiftool -G1 -a -s -u` on it under the time limit and records the **name ExifTool gives the id**. A committed record `tests/oracle-fixtures/exiftool/audit-tag-names.txt` stores the measured names; replay and live tests assert that the name table the test carries equals it. If ExifTool reports a name or group different from DEC-008's tag name, or no name, for any row, the build **does not edit the DEC or the table**: it records the measured output in the Handback and `guidance/questions.yaml` and I decide. (The ExifTool name column is measured data, not copied from memory.)

### Fuzz

New target `fuzz/fuzz_targets/audit.rs` (`fuzz-target-per-surface`: `audit(bytes)` is a new top-level surface), seeded from the 29 TIFF and 37 JPEG seeds plus new ones: `audit-png-magic`, `audit-bigtiff-magic`, `audit-empty-marker` (a zero-byte seed is allowed by libFuzzer; use a 1-byte seed if the tooling needs a non-empty file and say so), `audit-exif-gps-and-thumbnail`, `audit-two-blocks-same-tag`, `audit-many-overlapping-ifds` (the amplification shape, only if the measurement shows it is survivable; otherwise a smaller version plus the finding). Invariants, **each iff written in both directions**: never panics; every `Finding.item` is a valid index into `items()` and the item's kind matches the category (Region categories only on Region items, `Gps` only on `Tag` items with `ifd == Gps`); `items().len() <= MAX_INVENTORY_ITEMS`; `kind()` equals the independent re-derivation from the first bytes; `Unsupported` implies no items and `!clean`; `clean` iff (container readable and zero issues and zero failed blocks and not truncated); a `Gps` finding exists iff a GPS-kind tag item exists; a `Thumbnail` finding's range, when present, lies inside the blob. CI: `fuzz-smoke` matrix gains `audit` (60 s); `just fuzz-smoke audit` works. Shown red with planted bugs, at least one against the real libFuzzer target, and a single-guard variant where two guards overlap (HANDOFF-009's lesson).

## Acceptance Criteria

- `audit` is total and panic-free (five lints in non-test code); `categorize` equals the DEC-008 table row for row; no id outside the table is categorised.
- `sniff` is correct for every magic in rule 1 and for prefixes shorter than the magic (never panics, never misclassifies a short prefix as a longer magic).
- A bare TIFF and the same TIFF in a JPEG audit identically (rule 8) across all `oracle_tiff` fixtures; two Exif blocks give findings from both; an Exif block after the scan is included with `after_scan = true`.
- Unsupported containers, empty input, unreadable IFD0 and fatal Exif blocks are reported (kind/health), never as an empty clean audit.
- A GPS IFD yields one `Gps` finding per GPS tag; the same numeric ids in IFD0 (not GPS) yield none (context sensitivity); a MakerNote yields exactly one `MakerNote` finding and no serial claims; an IFD1 thumbnail yields a `Thumbnail` finding with its range checked against the blob.
- `MAX_INVENTORY_ITEMS` boundary is exact (at the cap: not truncated; one over: truncated, findings still complete); the Container amplification measurement is reported with command, input size, peak memory and time.
- The live table check passes against ExifTool 13.55 for every row, or each exception is a recorded question; the committed record reproduces (`just oracle-record` empty diff).
- Fuzz: `audit` target exists with the seeds above, 60 s smoke clean (executions and coverage recorded), the invariants shown red including one against the real target; CI matrix updated; `just fuzz-smoke audit` works.
- Gates: fmt (both graphs), `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof` (run locally; CI-only otherwise), `cargo test` on stable and 1.90.0 with `DIAMOND_EXIF_REQUIRE_ORACLE=1`, `-F unsafe_code`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `oracle-record` with a clean `git status` afterwards. No dependencies; `scripts/`, `decisions/`, irradiance untouched; existing 66 seeds and 120 records byte-identical.

## Failing Tests

Named tests (a rename, fold or missing test is a deviation to report):

**src/audit (unit):**
- `sniff_recognises_every_magic_and_never_overreads` (each magic; every proper prefix of each magic; empty; 1 byte)
- `sniff_distinguishes_tiff_from_bigtiff_and_never_calls_riff_without_webp_webp`
- `categorize_matches_the_dec_008_table_row_for_row`
- `categorize_assigns_nothing_outside_the_table` (all ids, each non-GPS kind)
- `every_gps_ifd_tag_is_a_gps_finding_whatever_its_id` and `the_same_ids_outside_a_gps_ifd_are_not_gps`
- `category_severity_matches_dec_008` and `severity_orders_low_medium_high`
- `audit_of_empty_and_unsupported_input_is_reported_not_clean` (each `Sniffed` variant)
- `audit_of_tiff_with_unreadable_ifd0_is_a_failed_block_not_an_empty_audit`
- `audit_lists_pointer_tags_in_the_inventory_but_never_as_findings`
- `a_makernote_is_one_finding_and_makes_no_serial_claim`
- `an_ifd1_thumbnail_is_a_finding_with_a_checked_range` (in-blob; range outside the blob gives `range: None`; non-FFD8 start flagged)
- `an_ifd1_with_strips_and_no_jpeg_pointer_is_a_thumbnail_finding`
- `two_exif_blocks_report_findings_from_both_and_never_deduplicate`
- `an_exif_block_after_the_scan_is_included_and_flagged`
- `a_fatal_exif_block_counts_as_failed_and_the_audit_is_not_clean`
- `inventory_cap_boundary_is_exact_and_findings_survive_truncation`
- `audit_is_deterministic` (same input, same output; item and finding order stable)
- `container_independence_audit_of_every_oracle_fixture_bare_and_in_jpeg`
- `audit_never_panics_on_every_committed_seed` (29 TIFF, 37 JPEG, plus the new audit seeds)
- `every_error_variant_displays` guard extended if `Error` gains variants (none expected)

**tests/audit_oracle.rs (live and replay):**
- `table_names_replay_matches_the_committed_record` and `live_table_names_match_exiftool` (skip loudly without the tool; `DIAMOND_EXIF_REQUIRE_ORACLE=1` makes a missing tool a failure)
- `table_name_check_is_red_when_a_name_is_perturbed` (perturb one recorded name, exactly that row is named)
- `container_amplification_is_measured_and_reported` (an `#[ignore]`d measurement test that prints the numbers, plus a bounded in-test version on a smaller hostile file that must finish under a time bound)

**tests/fuzz_layout.rs and the target:**
- `audit_harness_holds_on_every_seed_and_degenerate_inputs`
- `audit_harness_each_iff_invariant_goes_red_when_planted` (the plants as table-driven tests, both directions)
- `audit_seeds_are_committed_and_have_the_shapes_they_claim`
- `the_existing_66_seeds_are_byte_identical` (hash list)

## Non-Goals

- Strip policies or any preview (SPEC-014); rendering (SPEC-015); the binary (SPEC-016); value decoding in the report
- Inventory parity against ExifTool beyond the table-name check (SPEC-013)
- Reading inside the thumbnail, MakerNote parsing, XMP/ICC/IPTC/C2PA decoding, PNG/WebP/HEIC
- Fixing a `Container` memory amplification if the measurement finds one (reported; a TIFF spec will fix it)

## Notes for the Implementer

- Order of work, test-first: `sniff` (table of magics) -> `categorize` and the DEC-008 table test -> `audit` over TIFF -> over JPEG (blocks, regions, flags) -> bounds and health -> the amplification measurement -> the table-name oracle check and record -> the `audit` fuzz target, seeds, harness and plants -> CI matrix entry.
- The library has `std` (it uses `Vec`); keep `audit` allocation proportional to the inventory. Do not copy value bytes into the model.
- The independent re-derivation in the fuzz harness must not call `sniff`/`categorize`; write the magics and the table again in the harness (the shared-file approach from SPEC-010 is fine; `cargo fmt` does not follow `include!`, so format that file by hand).
- Probe any new ExifTool input shape alone under the time limit before it joins a loop. Tag-name fixtures are tiny and valid; a row ExifTool will not name is a finding, not a reason to drop the row.
- Run the two CI-only red-proof scripts locally before reporting done; they broke PR 14.
- Report in the Handback: every deviation, each table row whose ExifTool name differed, the amplification numbers, fuzz executions and coverage, and each red-proof before/after.
- Do not push or open a PR; do not advance cycles; do not edit `decisions/` (propose DEC amendments in the Handback).

---

## Reflection

*Appended during **ship**. Three questions, short answers.*

1. **What would I do differently next time?**
   — Check ids in a DEC against the reference tool at design, not at build: I wrote the Exif 3.0 ids `0xA437`-`0xA439` from memory and was wrong by one place; the build's live name check caught it, which is what it was for, but one ExifTool one-entry probe at design would have caught it first. Also reconcile the spec's invariants with the DEC before dispatch: "region categories only on Region items" contradicted DEC-008 (TIFF tags 700/33723/34377/34675 are container tags); the harness found it on its first seed run.

2. **Does any template, constraint, or decision need updating?**
   — DEC-008 amended twice (the shifted ids; one sentence on pointer tags inside a GPS IFD and the ExifTool-named ids left as inventory only). Verify's 25 own mutants (22 caught, 3 survived) again found only low test gaps, carried to SPEC-013. No template change.

3. **Is there a follow-up spec I should write now before I forget?**
   — Already written: SPEC-017 (bound `Container` memory, found by this build's amplification measurement: a 789 KB TIFF parses cleanly to 33M entries, ~950 MB), placed before the binary. Verify's low findings F1-F6 are carried into SPEC-013's outline.

4. **Where was the worst defect caught?** — one word from a fixed vocabulary so
   the defect-escape distribution is greppable across specs:
   `design` | `build` | `verify` | `ship` | `escaped` (reached prod/runtime) |
   `none` (clean first try).
   — build
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
   — Hand `exif-core` any bytes and get a typed audit: the sniffed container, an inventory of every tag and region, and PII findings by category and severity (GPS and device identity High), with unreadable and unsupported files reported as such and never as clean; before, the library returned parsed structures with no privacy meaning. Confirming numbers: 170 unit tests, 41 ExifTool-checked tag ids, 22 of 25 verify mutants caught.
