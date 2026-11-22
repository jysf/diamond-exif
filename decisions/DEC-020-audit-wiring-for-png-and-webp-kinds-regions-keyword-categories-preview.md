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
