---
insight:
  id: DEC-006
  type: decision
  confidence: 0.80
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

created_at: 2026-11-05
supersedes: null
superseded_by: null
status: accepted
deciders: [jysf, claude]
affected_scope:
  - "src/jpeg/**"
  - "src/error.rs"
  - "fuzz/**"

tags:
  - parser
  - jpeg
  - error-handling
  - policy
---

# DEC-006: JPEG segment walk — what is fatal, and a separate issue channel per layer

## Decision

A JPEG parse is fatal (`Err`) only when the input is not a JPEG at all: fewer than two bytes, or the first two bytes are not `FF D8` (SOI). Every other problem is a **`JpegIssue`** recorded on the returned `Jpeg`, and the walk continues wherever it safely can. JPEG-level issues and the TIFF-level `ParseIssue`s of an embedded Exif block stay in **separate channels, never merged into one flat type**; every issue is attributable to the layer that found it. How a caller sees both at once (a combined iterator, a combined `is_clean`) is decided in SPEC-010; the principle fixed here is that layers do not lose their identity.

## Context

DEC-001 made the TIFF walk tolerant per IFD. A JPEG is a marker stream, and uploads contain truncated segments, `0xFF` fill, missing EOI and trailing garbage far more often than clean files. The upload sanitiser and the audit CLI need "the Exif is fine, the file ends oddly" to be expressible, exactly as "IFD0 fine, GPS corrupt" is. Merging JPEG and TIFF problems into one type would hide which layer failed and force the TIFF issue type (which carries an IFD index and a `Link`) to describe things it cannot.

## Policy detail

- **Fatal:** fewer than 2 bytes (`Truncated`) or no SOI (`NotJpeg`).
- **Issue, walk continues:** a run of stray bytes between segments (`StrayBytes`, one issue per run), `FF 00` outside entropy data, a stray SOI, a segment length below 2 (`BadSegmentLength`; as of SPEC-010 the marker **and** its two length bytes, four in all, are skipped, because the length bytes of a bad length are `00 00` or `00 01` and can never start a marker, so reporting them again as stray bytes was noise: one fault, one issue), a missing SOS or EOI at the end of the walk (`MissingSos`, `MissingEoi`).
- **Issue, walk ends:** a segment whose declared length runs past the end of the input (`Truncated`; the segment is **omitted entirely**, never partially recorded, as for IFDs), the segment cap (`TooManySegments`), the issue cap (`TooManyIssues`).
- **Standalone markers** (no length field): `TEM` (01), `RSTn` (D0-D7) and a mid-stream `SOI` carry no segment; outside entropy data they are stray bytes. `EOI` (D9) ends the walk; anything after it is the **trailer**, returned as a borrowed slice, not an issue (real files carry thumbnails and trailers there).
- **Entropy-coded data** after an SOS header runs to the next marker that is not a stuffed `FF 00`, a restart `FF D0-D7` or `FF FF` fill. The walk resumes at that marker, so progressive files (several scans with DHT between) walk as a sequence of segments. The **scan region** is the borrowed slice from the first SOS marker to the end of EOI (or to the end of the input when EOI is missing).
- **Resource guards:** `MAX_JPEG_SEGMENTS = 4096` recorded segments and `MAX_JPEG_ISSUES = 1024` ordinary issues, each exceeded once by a terminal halting issue, so `segments().len() <= 4096` and `issues().len() <= MAX_JPEG_ISSUES + 1` always. Hitting the segment cap halts the walk and suppresses the end-of-walk `MissingSos`/`MissingEoi` issues (the cap is the story). The issue cap can also fire **during** the end-of-walk step: with 1023 ordinary issues recorded, `MissingSos` is the 1024th and is kept, and `MissingEoi` would be the 1025th, so it becomes `TooManyIssues` (found by the SPEC-009 fuzz run; the rule is the same, the earlier wording over-claimed). An issue-cap halt during the walk likewise suppresses the end-of-walk pair (nothing is recorded after a halt). `TooManyIssues`, if present, is always the last issue.
- **No length ever exceeds what the format allows:** a segment length is 16 bits and includes its own two bytes, so a payload is at most 65,533 bytes. Reading therefore has no "oversized APP1" case; the 65,533 cap is a **writer** constraint (`ExifTooLarge`, M1c).
- **Signatures identify, they do not decode.** `Exif\0\0`, the XMP namespace, the extended-XMP namespace, `ICC_PROFILE\0`, `Photoshop 3.0\0`, `JFIF\0` and the `JP` JUMBF header classify a segment's kind; the bytes behind them are not interpreted here.

## Alternatives considered

- **Strict (any bad segment is fatal).** Rejected: contradicts DEC-001's use case; a truncated file would lose its Exif.
- **One merged `Issue` type for both layers.** Rejected: hides the layer and bends the TIFF issue shape.
- **Partial segment recording on truncation.** Rejected: IFD-level atomicity is the established rule; a half segment has no trustworthy extent.
- **Cap by input length instead of constants.** Rejected: a 1 MB file of `FF 00` pairs would still yield half a million issues.

## Clarifications from the SPEC-010 design

- **Exif hand-off.** The payload after the 6-byte `Exif\0\0` signature is parsed with `Container::parse`; its tolerant walk and issues are the TIFF layer's, unchanged (container independence: the same blob gives the same IFDs as a bare TIFF). A fatal TIFF error (bad header or IFD0) does not become a JPEG fatal: that Exif block simply has no `Container`, and the error is kept on the block.
- **Several Exif segments.** The first Exif segment is the primary. Each further one costs a JPEG-layer issue `ExtraExif` and at most `MAX_EXIF_BLOCKS = 16` Exif segments are parsed in total (bounded work). ExifTool's behaviour with several is measured in SPEC-011, not assumed.
- **Located, not decoded.** XMP, extended XMP, ICC chunks, IPTC (the Photoshop `8BIM` resource `0x0404`) and JUMBF are reported as borrowed regions with their small headers read (extended-XMP GUID/length/offset, ICC sequence/count, JUMBF `En`/`Z`); contents are never interpreted. C2PA *detection* inside JUMBF is deliberately not claimed here (it needs the box-level content type, a format fact to be measured when the audit stage needs it).
- **Exif signature.** Refined by DEC-007: the Exif test is five bytes (`Exif\0`), measured against ExifTool in the SPEC-011 design; the `Exif\0\0` wording above names the canonical form.
- **Layered view.** The combined view is an iterator of an enum that names the layer (`Jpeg(&JpegIssue)` or `Exif { block, issue: &ParseIssue }`); issues are never converted into one flat type. The metadata object is clean only when the JPEG layer and every parsed Exif block are clean and no Exif block failed fatally.

## Consequences

- **Positive:** the same tolerant discipline at both layers; hostile JPEG shapes are testable data; bounded memory and issue counts by constants; progressive files and trailers need no special case.
- **Negative:** callers must consult two issue lists (mitigated by SPEC-010's combined view); coalescing stray runs means the exact count of garbage bytes is in the issue, not one issue each.
- **Neutral:** `Error` gains seven variants (additive on a `#[non_exhaustive]` enum): `NotJpeg`, `StrayBytes`, `BadSegmentLength`, `MissingSos`, `MissingEoi`, `TooManySegments`, `TooManyIssues`.

## Validation

Revisit if the writer needs per-segment provenance on issues, if fuzzing finds a shape where "skip two bytes and rescan" loops without progress, or if ExifTool disagrees on where scan data ends for a class of real files.

## References

- Related specs: SPEC-009, SPEC-010, SPEC-011
- Related decisions: DEC-001 (tolerant walk), DEC-002 (borrowed slices), DEC-005 (pointer policy)
- External: ITU-T T.81 (JPEG) Annex B marker syntax; JFIF; Adobe XMP specification part 3 (JPEG embedding); docs/exif-core-design.md §6
