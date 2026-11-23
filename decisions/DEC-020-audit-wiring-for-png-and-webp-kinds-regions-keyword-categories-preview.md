---
insight:
  id: DEC-020
  type: decision
  confidence: 0.7
  audience:
    - developer
    - agent

agent:
  id: claude-opus-5-5
  session_id: null

project:
  id: PROJ-001
repo:
  id: diamond-exif

created_at: 2026-11-22
supersedes: null
superseded_by: null
status: accepted
deciders: [jysf, claude]
affected_scope:
  - "src/audit/**"
  - "src/report/**"
  - "src/bin/exif-audit/**"
  - "src/write/mod.rs"

tags:
  - audit
  - png
  - webp
  - policy
---

# DEC-020: Audit wiring for PNG and WebP: container kinds, region kinds, text-keyword categories, an honest preview

## Decision

1. **Kinds.** `ContainerKind` gains `Png` and `Webp` (and becomes `#[non_exhaustive]`); `Unsupported(Sniffed)` keeps HEIC, GIF, BigTIFF, empty and unknown. `audit()` dispatches `Sniffed::Png` and `Sniffed::Webp` to readers built on `png::read` and `webp::read`. A parse that returns `Err` is `unreadable(kind)` exactly as for JPEG.
2. **Every Exif block is inventoried, nothing is merged.** Each parsed block of a PNG (`eXIf`) or WebP (`EXIF`) is walked like a JPEG's, tagged with its block index. The audit reports no single value, so ExifTool's per-selector first-or-last rule (SPEC-034, SPEC-035) does not apply to it; a privacy report shows every copy. A WebP block past the library's 16-block cap is not listed; the library's issue for it makes the audit not clean, so the file never reads as safe.
3. **Region kinds** (`RegionKind` is already `#[non_exhaustive]`): existing `Xmp` and `Icc` are reused; new `Text`, `RawProfile`, `Time` (a PNG `tIME` chunk of ANY length: ExifTool 13.55 prints `PNG:ModifyDate` for 0, 6 and 8 bytes too) and the existing `Trailer` (PNG bytes after `IEND`). An item's `segment` is the chunk index for these formats; `Origin::after_scan` means "after the image data" (after the first `IDAT`, or after `IEND`). A length-0 WebP region is inventoried but yields no finding (ExifTool treats it as absent, measured in SPEC-034); a length-0 PNG region (`iCCP` with a name, iTXt XMP, a `Text` with an empty value) DOES yield its finding, because ExifTool prints a tag for each (measured in the SPEC-032 verify). Located is not valid: a located `XMP ` or `ICCP` region may be unreadable to ExifTool, and the reports say "located".
4. **Categories, no new `Category`.** `Xmp`, `Icc` as for JPEG. `RawProfile` and `Trailer` are `OtherSegment` (present, not parsed, may hold Exif). `Time` is `Timestamps`. A `Text` region is categorised by its keyword, taken from the ExifTool names measured in the SPEC-032 design (13.55): `Author` and `Copyright` are `Authorship` (SPEC-033 may re-home `Copyright`; the table is data, one row), `Creation Time` is `Timestamps`, `Software` is `Software`, every other keyword (`Title`, `Description`, `Comment`, `Disclaimer`, `Warning`, `Source`, unknown) is `FreeText`. The keyword match rule (exact, or first-letter-case-insensitive: ExifTool printed `author` as `Author` and `AUTHOR` as its own tag) is measured by the build and pinned.
5. **Raw profiles are unreadable metadata.** A non-empty `RawProfile` region (ImageMagick `Raw profile type ...`) can hold Exif with GPS that ExifTool decodes and we do not (SPEC-032 verify H2). Each one counts as a failed block in `Health` (status `unreadable`, never clean, never `ok`), in addition to its `OtherSegment` finding. Compressed `Xmp` and `Text` regions stay ordinary findings: their category is known from the keyword.
6. **The preview does not lie.** For a PNG or WebP the writer refuses, so `preview()` returns every finding as `Kept`, `Counts::removed` is 0, `Preview::reliable()` is false, and a new `Preview::strippable()` is false; it is true ONLY for JPEG and TIFF, so false for GIF, HEIC, BigTIFF, empty and unknown too. One shared helper on `ContainerKind` is the single definition (the report summary and the fuzz harness use it, not copies). Text, JSON and CSV say so (`strippable` is an additive JSON field). `Disposition` gains no variant.
7. **The writer stays fail-closed.** `write::strip` returns `StripError::Unsupported` carrying the new `ContainerKind::Png` or `Webp`; a test pins it for both, with no byte produced.
8. **`Health` keeps its field names.** `jpeg_issues` counts the container-layer (walk) issues for every format; renaming it would break the report schema (DEC-010). Its documentation says so.

## Context

STAGE-008 SPEC-028 to SPEC-031, SPEC-034 built and oracle-checked the two readers; `sniff` already names both formats, and the writer already refuses them, but the audit, the CLI read gate and the renderers still treat both as unsupported. Measured in the design (ExifTool 13.55, `-a -G1 -s`): PNG `tEXt` keywords map to `PNG:Title`, `Author`, `Description`, `Copyright`, `CreationTime`, `Software`, `Disclaimer`, `PNGWarning` (keyword `Warning`), `Source`, `Comment`, unknown keywords to their own names; a `tIME` chunk prints `PNG:ModifyDate`; `pHYs` prints `PNG-pHYs` tags (not personal).

## Alternatives considered

- A new `Category::Text` family or per-keyword categories in `Category`: rejected, DEC-008 is amended by SPEC-033 in parallel and the existing categories already express every row.
- A new `Disposition::NotSupported`: rejected; every consumer of the enum would change, and `Kept` plus `strippable()` is the same fact.
- Renaming `Health::jpeg_issues`: rejected for now (schema stability).
- Merging blocks to one value in the audit: rejected (privacy report shows every copy).

## Consequences

- `exif-audit` reads PNG and WebP: findings, exit codes and `--fail-on` apply to them. HEIC, GIF, BigTIFF stay unsupported.
- PNG `tIME` and the standard text keywords become findings, which is more than the readers' `Text` region location.
- Tag-level inventory parity against ExifTool for PNG and WebP is a separate spec (SPEC-036); this one checks category presence against the committed oracle records.

## Validation

`src/audit` unit tests per shape, a keyword table test, the writer refusal test, the CLI exit-code tests, the category-presence replay against the committed PNG and WebP oracle records with a shown-red perturbation, 60 s smoke of `audit` and `audit_render` with the new seeds.

## References

DEC-001, DEC-006, DEC-008, DEC-009, DEC-010, DEC-017, DEC-018, DEC-019; SPEC-032.

## Clarifications from the SPEC-037 design (2026-11-22)

Measured on ExifTool 13.55 (`-a -u -G1 -H -s`, one chunk per file, GPS-bearing content where the chunk can carry Exif or XMP; the full table is in DEC-021). The audit located only `eXIf`, text, `iCCP`, `tIME` and the raw profiles; ExifTool decodes more. The reader (`PngRegion::Chunk`, `ChunkClass`, `chunk_class`) now locates, by type bytes and in the walk and after `IEND`:

1. **`tIME` after `IEND`.** ExifTool prints `Trailer:ModifyDate` for it. The audit used to look for `tIME` only in the walked chunks; the reader now locates it in both, so a `tIME` after `IEND` is a `Time` region, `Timestamps` (clause 3 holds for every position).
2. **Carriers of Exif or XMP the library does not decode** (`RegionKind::Carrier`, `OtherSegment`): `zxIf`, and the lower-case `zxif` and `exif`, which ExifTool reads as Exif (the data is a TIFF, or `\0`, a 4-byte length and a zlib stream); `tXMP` (raw XMP), `meTa` (XML, also UTF-16) and `seAl` (XML), which ExifTool reads into XMP or XML groups. Like a raw profile (clause 5) a non-empty one counts as a failed block, so the file is never `ok` while one is present.
3. **`caBX`** (a JUMBF box, ExifTool group `JUMBF`) is a `Jumbf` region, `Jumbf` category, as in a JPEG: kept by `AllPii` and `All`, removed by `Everything`.
4. **Opaque binary chunks** that are not metadata for any preset (`RegionKind::OtherChunk`, `OtherSegment`): `dSIG`, `cpIp`, `gIFx`, `gIFg`, `gIFt`, `fRAc`. Chunks ExifTool prints that are image structure or rendering data (`pHYs`, `gAMA`, `sRGB`, `cHRM`, `bKGD`, `tRNS`, `sBIT`, `oFFs`, `sCAL`, `sTER`, `vpAg`, `acTL`, `cICP`, `pCAL`, `sPLT`, `hIST`, `iDOT`, `gdAT`) are not located and are never removed.
5. Type bytes are case-sensitive: `txmp` is nothing (measured), with two exceptions: ExifTool matches `zxIf` and `eXIf` in any ASCII case, so every other spelling of them (`exif`, `ZXIF`, `ExIf`, ...) is an `ExifCarrier` (corrected in the SPEC-037 fix round after a re-measure of every metadata type; see DEC-021).
6. `RegionKind` gains `Carrier` and `OtherChunk` (report names `carrier`, `other_chunk`).

## Clarifications from the SPEC-038 design (2026-11-22)

Measured on ExifTool 13.55 (`-a -u -G1 -H -s`, a minimal WebP with one chunk, the chunk before the image data, after it, beyond the declared RIFF size, and with no `VP8X`). The audit located only `EXIF`, `XMP ` (and `XMP\0`) and `ICCP` in a WebP; ExifTool decodes more. The reader (`WebpRegion::Chunk`, `webp::ChunkClass`, `webp::chunk_class`) now locates, by the code bytes and in every position, and the audit reports:

1. **Carriers** (`RegionKind::Carrier`, category `OtherSegment`; every preset removes them, DEC-022 clause 4): every `LIST` chunk of any list type (`INFO`, `INF0`, `adtl`, `exif` and `hdrl` printed tags; the rest is the safe direction); `IDIT`, `afsp`, `bext`, `guan`, `list`, `labl`, `note`, `ltxt`, `olym`; a `JUNK` chunk whose data is printable ASCII (0x20 to 0x7E) followed by zero or more NULs (measured exactly: a control byte, a high byte, a NUL before or inside the text, an empty chunk and an all-zero chunk print nothing, and are not located); `_PMX`, `SEAL`, `aXML`, `iXML`, `JUNQ` (XMP or XML); `ID3 ` and `id3 ` (ID3v2); `gps0`, `SGLT`, `SLLT` (GPS); and a chunk coded exactly `RIFF` (corrected in the SPEC-038 fix round: ExifTool walks into it as a container, whatever its form type, size or depth, and prints what it finds; DEC-022 clarification 9). A non-empty XML carrier (`_PMX`, `SEAL`, `aXML`, `iXML`, `JUNQ`) or `RIFF` chunk counts as a failed block like a PNG `tXMP` (clause 5), so the file is never `ok`.
2. **`C2PA`** is a `Jumbf` region, category `Jumbf`: kept by `AllPii` and `All`, removed by `Everything` (the same stated residual as a PNG `caBX`).
3. **Structure chunks** that print binary tags but are not metadata for any preset (`CSET`, `acid`, `ds64`, `fact`, `fmt `, `inst`, `smpl`, `cue `, `plst`, `gsen`) are `OtherChunk`, category `OtherSegment` (removed by `Everything` only).
4. **Case.** All 16 ASCII-case spellings of every type above were measured: none folds. `ID3 ` and `id3 ` are two codes ExifTool knows; no other spelling of any type decodes. Chunks of any other type print only an `Unknown_<code>` binary dump under `-u` (`PAD `, `strd`, `strn`, `PrmL`, `Cr8r`, a lower-case `exif`, `xmp `, `iccp`, ...) and are not located.
5. **Position.** Every row printed in every position (before and after the image data, beyond the declared RIFF size, with and without a `VP8X`) except after an empty null chunk, where ExifTool stops reading; this library keeps reading, so the audit and the strip see more than ExifTool there, never less. A chunk inside an `ANMF` frame is never decoded by ExifTool and never walked here.
6. A length-0 carrier is located and has no finding (clause 3: a length-0 WebP region yields none); the strip still removes it.
7. **What moved.** Nothing in the committed oracle records or the existing parity tests (no recorded fixture holds one of these chunks); the audit fuzz mirror (`fuzz/shared/audit_invariants.rs`) and the WebP reader's model (`fuzz/shared/webp_riff_invariants.rs`, which panicked on an unknown region variant on purpose) model the new variant from the table above; `tests/webp_carriers.rs` iterates the table (`tests/support/webp_carriers.rs`).

## Amendment (2026-11-22, SPEC-033): the `Copyright` keyword is `Rights`

Clause 4's `Copyright` row moves to `Rights` (DEC-008 amendment, 2026-11-21): a PNG text chunk keyed `Copyright` or `copyright` (the measured first-letter rule, unchanged) is a `Rights` finding, severity Medium as before. `Artist` and `Author` stay `Authorship`. `COPYRIGHT` and other spellings ExifTool prints as their own tag stay `FreeText`. No other row changes and no category is added for PNG or WebP. A WebP has no keyword table: ExifTool's `RIFF:Copyright` comes from a `LIST/INFO` chunk, which is an opaque carrier (DEC-022 clause 4) and is unaffected.
