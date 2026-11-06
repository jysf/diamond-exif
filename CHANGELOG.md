# Changelog

All notable changes to this app. Newest at top.

Keep an `## [Unreleased]` section on top and add entries under it as you work —
the patch lane files fixes here (`[Unreleased] → Fixed`), and a release spec
(DEC-006) promotes the section to a version heading when it cuts the tag.

## [Unreleased]

### Added

- Inventory parity against ExifTool (SPEC-013, test-only; `src/` unchanged): the audit's inventory (every IFD entry as `(group, tag id)`, every located region) is compared with `exiftool -a -u -G1 -H -s` over 247 inputs (178 distinct): hand-built fixtures, the DEC-008 one-entry TIFFs and the three fuzz-seed directories, live and replay (`tests/inventory_oracle.rs`, records in `tests/oracle-fixtures/exiftool-inventory/`). 146 of the 177 comparable distinct inputs agree exactly; 31 differences are pinned as exact multiset differences with a question each, and anything ExifTool lists that the audit does not leaves the file not clean (asserted). Red-proofs: perturbed inventories and records, byte mutations, and six library faults on a mutated copy. Also closes the SPEC-012 and SPEC-017 verify test gaps (`exif_issues` summed over blocks, region `after_scan`, thumbnail pointer types, and three entry-budget cases) and makes two quadratic fuzz-harness guards linear.
- Total entry budget for the tolerant walk (SPEC-017, DEC-001): `exif_core::tiff::entry_budget(len)` (`len / 12`), `Container::entry_count()` and `Error::TooManyEntries { limit }`. The IFD whose declared count would push the retained entries past the budget is omitted whole, one `TooManyEntries` issue is recorded and the walk ends (like `TooManyIfds`). A real file never reaches it (every entry occupies its own 12 bytes); overlapping tables did: a 789,516-byte TIFF naming 511 overlapping SubIFDs parsed to 33,488,386 entries (about 804 MB) and now to 65,536 (1.5 MB). New `tiff_ifd`/`audit` fuzz invariants (`fuzz/shared/entry_budget_invariants.rs`) and three `tiff_ifd` seeds (32 in all). No dependencies.
- Exif and located metadata regions from JPEG (SPEC-010, DEC-006): `exif_core::jpeg::{read, JpegMetadata, ExifBlock, Region, AnyIssue, MAX_EXIF_BLOCKS, MAX_JPEG_METADATA_ISSUES}`. `read` walks the segments, hands the payload after `Exif\0\0` of each Exif segment (the first is primary; at most 16 are parsed, each further one costs a JPEG-layer `ExtraExif` issue) to the unchanged tolerant TIFF engine, so a JPEG-wrapped Exif reads identically to the bare TIFF (container independence), keeps a fatal TIFF error on its block rather than failing the JPEG, and locates XMP, extended XMP (GUID, full length, offset), ICC chunks (sequence, count; one `IccChunkMismatch` for an inconsistent set), IPTC (the Photoshop `8BIM` resource `0x0404`) and JUMBF (`En`, `Z`) as borrowed regions whose contents are never interpreted. JPEG-layer and per-Exif-block issues stay separate and are viewed together through `AnyIssue`; `is_clean` needs every layer clean. `Error` gains `ExtraExif`, `RegionTooShort`, `IccChunkMismatch`. The `jpeg_segments` fuzz target now also asserts the `read` invariants (shared with `tests/fuzz_layout.rs`, the extraction re-derived independently and compared both ways) and has 13 new seeds (34 in all). No dependencies.
- JPEG segment walker (SPEC-009, DEC-006): `exif_core::jpeg::{Jpeg, Segment, SegmentKind, JpegIssue}` with `parse`, `parse_strict`, `segments`, `issues`, `is_clean`, `scan_region`, `eoi_offset`, `trailer`, `MAX_JPEG_SEGMENTS` (4096) and `MAX_JPEG_ISSUES` (1024). Only a non-JPEG is fatal; stray bytes, bad lengths, truncation, missing SOS/EOI are issues and the walk continues where it safely can. Payloads, scan region and trailer are borrowed from the input; APPn kinds (JFIF, Exif, XMP, extended XMP, ICC, IPTC, JUMBF) are identified by signature, nothing is decoded. `Error` gains `NotJpeg`, `StrayBytes`, `BadSegmentLength`, `MissingSos`, `MissingEoi`, `TooManySegments`, `TooManyIssues`. New `jpeg_segments` fuzz target (iff-invariants in both directions) with 21 seeds, `just fuzz-smoke [TARGET]`, and a CI `fuzz-smoke` matrix over `tiff_ifd` and `jpeg_segments`. No dependencies.
- Rust CI (repo setup, not a spec): `.github/workflows/ci.yml` with fmt and clippy on both cargo graphs, `test` with a pinned, sha256-verified ExifTool 13.55 and `DIAMOND_EXIF_REQUIRE_ORACLE=1`, oracle-record reproducibility, cargo-deny on both graphs, a 60 s `tiff_ifd` fuzz smoke, MSRV 1.90.0, `forbid(unsafe_code)`, the lint-policy and fuzz-lint red-proofs and the no-`#[allow]` check, beside the spec-driven gates and their red-proofs. `scripts/lint-red-proof.sh` and `scripts/lint-fuzz-red-proof.sh` cloned from irradiance; `just lint-red-proof`, `lint-fuzz-red-proof`, `lint-no-allow`.
- Mutated-copy red-proof apparatus (SPEC-008): `tests/support/{mutate,probe}.rs` and `tests/mutation_oracle.rs` with `just mutation-oracle`. Total, panic-free byte-mutation helpers (`set_u16_at`/`set_u32_at`, `flip_bit`, `truncate_at`, `swap_byte_order_mark`, `locate_entry`); one shared apparatus that stages a throw-away copy of `src/`, injects a marked one-expression fault under an exactly-once needle guard, builds it offline in its own target dir and reads the fixtures back through the oracle's own normalisation. Six library-fault red-proofs (SHORT byte order, ASCII last byte, rational swap, GPS pointer not followed, SRATIONAL sign, SSHORT read unsigned), each with a green control first, and three byte-mutation red-proofs (corrupt the GPS pointer, swap the byte-order mark, truncate into the Exif IFD). They compare with the committed ExifTool records, so they need no ExifTool. Closes the SPEC-005 survivor: new `ExifIFD:ExposureProgram` (SSHORT), `ExifImageWidth` (SLONG), `ExposureTime` (FLOAT) and `FocalLength` (DOUBLE) selectors, three new fixtures and records.
- Every ExifTool spawn has a wall-clock limit (SPEC-008): default 5 s, `DIAMOND_EXIF_TOOL_TIMEOUT_SECS` overrides, `ToolError::Timeout`, child killed and reaped.
- ExifTool read-parity oracle (SPEC-005): std-only `tests/support/{tools,oracle,oracle_tiff}.rs`, `tests/metadata_oracle.rs`, `examples/oracle_record.rs` and `just oracle-record`, 38 committed ExifTool records under `tests/oracle-fixtures/exiftool/`. Live parity over nine hand-built fixtures and the 29 seeds, a replay-and-perturb red-proof (one value changed, exactly that field named) and a live mutated-payload demo; loud `SKIP` without ExifTool, a failure under `DIAMOND_EXIF_REQUIRE_ORACLE=1`. One divergence recorded: ExifTool 13.55 does not return on the `two-hop-cycle` seed.
- ASCII, signed and float decoders (SPEC-007): `Container::{ascii, sints, floats}`, `tiff::Ascii` (text split at the first NUL, borrowed from the input; unterminated, empty, embedded-NUL and non-UTF-8 values are data, not errors), the `TYPE_ASCII` / `SBYTE` / `SSHORT` / `SLONG` / `FLOAT` / `DOUBLE` constants, and the `Make`, `Model`, `Artist`, `Copyright`, `DateTime`, `DateTimeOriginal`, `Software`, `ImageDescription` tag constants. `sints` and `floats` return `i32` / `f64` verbatim (NaN and infinities pass through). The `tiff_ifd` fuzz target checks every reader's type gate as an iff and the ASCII partition and zero-copy rules; three new seeds (29 in all).
- Multi-IFD pointer walk (SPEC-004, DEC-005): `tiff::IfdKind` (`Ifd0`, `Ifd1`, `Chain`, `Sub`, `Exif`, `Gps`, `Interop`), `Ifd::kind`, `Container::ifd_of_kind`, `tiff::MAX_ISSUES`, the `EXIF_IFD` / `GPS_INFO` / `INTEROP_IFD` tag constants, and `Error::BadPointerCount`. The `tiff_ifd` fuzz target checks kind and parent invariants (both directions where they are an iff) and has five new seeds (25 in all).
- Tolerant-walk API (SPEC-003, DEC-001): `tiff::{ParseIssue, Link}`, `Container::{issues, is_clean, parse_strict}`; `Error` now derives `Clone, PartialEq, Eq`. The `tiff_ifd` fuzz target checks the strict/tolerant differential, containment and bounds, and has three new seeds (20 in all).
- `tiff_ifd` fuzz target (SPEC-006, DEC-004): separate `fuzz/` crate (`exif-core-fuzz`, `libfuzzer-sys` only there), deterministic seed writer `examples/fuzz_seeds.rs` with 17 committed seeds, `tests/fuzz_layout.rs`, and `just deny-fuzz` / `lint-fuzz` / `fuzz-smoke`. The library is unchanged and still has no dependencies.
- Strict TIFF/IFD skeleton (SPEC-002), ported from irradiance `src/ifd.rs` (copy-then-edit, DNG removed): `exif_core::{Error, Result}`, `reader::ByteOrder`, `tiff::{Container, Ifd, Entry}` with `parse`, `payload` (returns `&'a [u8]` tied to the input), `uints`, `urationals`/`srationals` (verbatim, never divided), the three walk guards, and 23 tests. Any bad IFD is still `Err`; the tolerant walk is SPEC-003. No dependencies.
- `exif-core` crate scaffold (SPEC-001): empty gated lib with `#![forbid(unsafe_code)]` and the five-lint deny set, `deny.toml`, dual MIT/Apache-2.0 licence files, provenance-ledger header, `tests/crate_policy.rs`. No dependencies, no public API.

### Changed

- JPEG walker (SPEC-010): a segment length below 2 now skips the marker and both length bytes (four bytes), so one fault is one `BadSegmentLength` issue instead of that plus a `StrayBytes` for the length bytes (DEC-006 reworded). The module docs state that a fill run reaching the end of the input reports `Truncated` at the last `FF` (behaviour unchanged).

- The walk now follows the Exif (0x8769), GPS (0x8825) and, from inside the Exif IFD, Interop (0xA005) pointers as well as `SubIFDs` (SPEC-004, DEC-005); a corrupt GPS IFD is one reported issue and leaves IFD0 and the Exif IFD readable. The next-word of Exif/GPS/Interop IFDs is recorded but not followed. A `SubIFDs` entry declaring more than 512 offsets is now rejected (`TagTooLarge`) before any offset is decoded, so the issue count is bounded by `MAX_ISSUES`.
- `Container::parse` is now tolerant (SPEC-003, DEC-001): only a bad header or an unreadable IFD0 is `Err`; any other bad IFD (IFD1 and later, `SubIFDs`) is omitted and reported as a `ParseIssue`, so one corrupt sub-IFD no longer discards a readable IFD0. `Container::parse_strict` keeps the old all-or-nothing behaviour. Exact-boundary tests added for the depth and IFD-count limits.

**Gate harness reconciled with irradiance's hardened mute-gate scripts** (instance-local; template VERSION unchanged at 0.6.39).

Brought over from irradiance (SPEC-011/016/023, PATCH-002/003) the audit and
red-proof scripts the upstream template has not absorbed. Template-newer content
(everything else, incl. `scripts/test.sh`'s `guidance/agents/` checks) is kept.

- `_lib.sh` — `front_matter_parse_report` (real YAML parse), `|| true` on the
  active-project grep, `next_id` skips `tests/`, quoted-`notes:` `#` no longer
  truncated, CRLF/quoted-tolerant `get_stage_status`, stage orchestration-cost
  helpers. **Not carried:** irradiance's project-specific grandfather lists
  (`SPEC-004 PATCH-002/3/4`, `STAGE-001`) — both defaults are empty here.
- `validate.sh` — front matter is PARSED (ruby `YAML.parse`) across `projects/` +
  decisions, with a completion sentinel; `cost-audit.sh` — shipped stages must
  record `orchestration_cost`; `decisions-audit.sh`, `handback-sync.sh` —
  unguarded-grep fixes, superseded handoffs are not "pending".
- New: `gate-audit.sh` (+ `-red-proof`), `validate-red-proof.sh`,
  `cost-audit-red-proof.sh` (fixture-synthesising: builds its own STAGE-900/901
  from the stage template instead of irradiance's real STAGE-001/002),
  `tests/oracle-fixtures/SPEC-900-unparseable-front-matter.md`.
- `scripts/test.sh` — only the 19 `|| true` guards `gate-audit` demanded.
- `app.just` — `validate-red-proof`, `cost-audit-red-proof`, `gate-audit`,
  `gate-audit-red-proof` recipes.
- **Deliberately not ported:** `lint-red-proof.sh` / `lint-fuzz-red-proof.sh`
  (SPEC-011). They mutate irradiance's Rust crate (`src/lib.rs` panic-free
  `#![deny]` policy, `fuzz/`) and have nothing to run against here.

### Fixed
