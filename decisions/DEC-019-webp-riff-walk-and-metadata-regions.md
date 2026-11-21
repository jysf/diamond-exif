---
insight:
  id: DEC-019
  type: decision
  confidence: 0.75
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

created_at: 2026-11-21
supersedes: null
superseded_by: null
status: accepted
deciders: [jysf, claude]
affected_scope:
  - "src/webp/**"
  - "src/error.rs"
  - "fuzz/**"

tags:
  - parser
  - webp
  - metadata
  - policy
---

# DEC-019: WebP RIFF walk and metadata regions — the declared size never limits the walk

## Decision

A WebP parse is fatal (`Err`) only when the input is not a WebP: fewer than 12 bytes (`Truncated`) or the first 12 bytes are not `RIFF`, four size bytes, `WEBP` (`NotWebp`). Everything else is a **`WebpIssue`** (a fourth issue channel, DEC-006 and DEC-017's discipline) and the walk continues where it safely can. `webp::read` then extracts Exif and located regions in chunk order, as DEC-018 does for PNG.

## Context (measured in the SPEC-030 design, ExifTool 13.55, `-a -G1 -s -n`, 44 hand-built shapes)

- **The declared RIFF size does not limit what ExifTool reads.** A size that is too small (cutting before the EXIF chunk), zero, odd, too large, or followed by trailing bytes: the EXIF chunk is read in every case.
- **`VP8X` flags are informational.** `EXIF` is read with the EXIF flag unset, with no `VP8X` at all, before `VP8X`, and before the image chunk; the flag set with no chunk is silent. `XMP ` likewise. A 3-byte `VP8X` is tolerated.
- **Four-character codes are case-sensitive:** `exif`, `Exif`, `EXIf`, `xmp `, `iccp` are not read. `XMP\0` **is** read (warning "Incorrect XMP tag ID"); `XMP-` and `XMPx` are not.
- **Padding is real:** an odd-sized chunk without its pad byte makes the next chunk unreadable (the walk skips one byte too far), so framing must apply the pad rule.
- **Two `EXIF` chunks: both are read**; two `XMP ` chunks: both read. `EXIF` with a leading `Exif\0\0`: read with a minor warning. Empty or non-TIFF `EXIF`: an opaque `UnknownEXIF`, nothing parsed.
- **Not read:** metadata nested inside `ANMF` (the animation frame payload); `EXIF` after an `ANMF` chunk at top level **is** read.
- **Damage:** a chunk length past the end, fewer than 8 bytes left, or a partial chunk header: warning "Error reading RIFF file", earlier chunks still read. An 11-byte header: "File format error". A 12-byte header only: no output, no error.

## Policy detail

- **Walk.** After the 12-byte header, chunks run to the **end of the input**, not to the declared size. A chunk is a 4-byte code, a 4-byte little-endian size, the data, and, when the size is odd, one pad byte. A chunk whose size runs past the end of the input, or fewer than 8 bytes left (but more than 0), is `Truncated` and ends the walk; the chunk is omitted entirely. Exactly 0 bytes left is a clean end. Sizes are compared as 64-bit values before any offset is formed.
- **Issues, walk continues:** `RiffSizeMismatch { declared, actual }` when the declared size is not `input length - 8` (one issue, at offset 4); `MissingPad` when an odd chunk ends exactly at the end of the input (no pad byte); `BadVp8x` for a `VP8X` shorter than 10 bytes; `MisplacedVp8x` for a `VP8X` that is not the first chunk; `ChunkFlagMismatch` (one per flag: ICC, EXIF, XMP, in that order, after the walk) when a `VP8X` flag disagrees with the chunks present, and once `MetadataWithoutVp8x` when there is no `VP8X` but an `EXIF`, `XMP ` or `ICCP` chunk exists. The pad byte's value is not checked.
- **Issues, walk ends:** `Truncated`, `TooManyChunks`, `TooManyIssues` (DEC-006/017 rules: a halting issue is replaced by `TooManyIssues` once the issue cap is full; the cap chunk is not recorded).
- **Chunk kinds (case-sensitive, identification only):** `VP8X`, `VP8 `, `VP8L`, `ALPH`, `ANIM`, `ANMF`, `ICCP`, `EXIF`, `XMP `, `Other`. `XMP\0` is `Xmp` with one `NonCanonicalChunkId` issue (ExifTool reads it). `ANMF` payloads are passed over, never walked.
- **Metadata (`webp::read`).** `EXIF` chunks: first primary, each further one an `ExtraExif` issue and still parsed, at most `MAX_WEBP_EXIF_BLOCKS = 16` parsed; payload to the unchanged TIFF engine; a payload starting `Exif\0\0` is accepted with that header skipped and one `ImproperExifHeader`; empty or non-TIFF payloads leave the block `Err`. `XMP ` gives `Xmp { offset, data }`, `ICCP` gives `Icc { offset, data }`; no decoding of any kind (WebP has no compression inside these chunks). Regions and blocks report the input offset of their chunk, and `tiff_at` the input offset of the first TIFF byte. Extraction issues are suppressed after any walk halt (DEC-018 rule); at most two issues per chunk.
- **`VP8X` info:** `Webp::vp8x()` returns the flags (ICC, alpha, EXIF, XMP, animation) and the 24-bit canvas width and height (each stored minus one) of the first `VP8X` when it has at least 10 bytes.
- **Guards:** `MAX_WEBP_CHUNKS = 65_536` recorded chunks (an animated WebP has one `ANMF` per frame), `MAX_WEBP_ISSUES = 1024`; regions are bounded by the chunk count. Progress: every iteration advances at least 8 bytes or ends the walk; iterative.
- **Why walk past the declared size:** the audit must see what ExifTool sees, and trailing data after a short declared size is a known hiding place. The mismatch is reported, never silenced; the writer (M1d) will decide what to rewrite.

## Alternatives considered

- **Stop at the declared RIFF size.** Rejected: ExifTool reads past it; the audit would miss metadata a reader sees.
- **Trust `VP8X` flags to decide what to look for.** Rejected: the flags are routinely wrong and ExifTool ignores them for reading.
- **Case-insensitive four-character codes.** Rejected: ExifTool does not read them, and the RIFF specification is case-sensitive.
- **Walk into `ANMF` payloads.** Rejected: ExifTool does not read metadata there; frame sub-chunks are not top-level metadata.

## Consequences

- **Positive:** same tolerant shape as the other containers; every shape ExifTool reads is located; size and flag disagreements are reported as data.
- **Negative:** a fourth issue channel; a file whose declared size is smaller than its content is walked past that size, which a strict RIFF reader would not.
- **Neutral:** `Error` gains `NotWebp`, `RiffSizeMismatch`, `MissingPad`, `BadVp8x`, `MisplacedVp8x`, `ChunkFlagMismatch`, `MetadataWithoutVp8x`, `NonCanonicalChunkId` (additive); `TooManyChunks` and `TooManyIssues` are reused with walk-neutral wording.

## Clarifications from the SPEC-030 build and verify (2026-11-21)

- **Halt rule:** a walk halt (`Truncated`, `TooManyChunks`, `TooManyIssues`) suppresses the end-of-walk `ChunkFlagMismatch` and `MetadataWithoutVp8x` issues, since a truncated walk cannot know which chunks followed the damage; the halting issue is always last. Regions and blocks already found are kept; extraction issues are suppressed (DEC-018 rule).
- **`vp8x()`** is the first `VP8X` chunk only: `None` when it is shorter than 10 bytes even if a later one is longer (ExifTool decodes a later one; the metadata it locates is the same). A `VP8X` of any length counts as present for `MetadataWithoutVp8x`; a short one gets no flag comparison. A `VP8X` after the first is `MisplacedVp8x` and never replaces it.
- **Issue offsets:** `RiffSizeMismatch` at 4 and always first; a `ChunkFlagMismatch` at the first `VP8X`; `MetadataWithoutVp8x` at the first metadata chunk.
- **Issues per chunk:** "at most two" applies to extraction issues (`ExtraExif`, `ImproperExifHeader`); the walk can add up to three on one chunk (`NonCanonicalChunkId`, `BadVp8x`, `MisplacedVp8x`, `MissingPad` in that order). All totals are bounded by the constants.
- **Empty regions:** a zero-length `XMP ` or `ICCP` chunk is reported as an empty region (ExifTool shows nothing for an empty `XMP `); consumers must not count an empty region as present metadata.
- **`NonCanonicalChunkId`** is raised for `XMP\0` even at zero length (ExifTool warns only when it reads content); a warning-level difference.
- **`Chunk::range()`** includes the pad byte only when the pad is in the input.
- **`RiffSizeMismatch.actual`** is `input length - 8` as a `usize`.
- **Verified against ExifTool** (14 further shapes): `ICCP` with the flag unset or no `VP8X`, `ALPH` before `VP8 `, a simple `VP8 ` file with `EXIF` appended, `EXIF` right after the header, declared size 2, odd last `EXIF` or `XMP ` without pad (ExifTool warns "No padding"), two `VP8X`, an `ANMF` holding an `EXIF` followed by a top-level `EXIF` (only the top-level one is read): the located metadata matches in every case.

## Validation

Revisit if SPEC-031 shows a class of file where ExifTool stops at the declared size, if real files make `ChunkFlagMismatch` so common it is noise, or if the writer needs the declared end as a first-class value.

## References

- Related specs: SPEC-030, SPEC-031, SPEC-032
- Related decisions: DEC-017, DEC-018, DEC-006, DEC-002, DEC-001
- External: WebP Container Specification (Google); RIFF; ExifTool 13.55 behaviour measured in the SPEC-030 design
