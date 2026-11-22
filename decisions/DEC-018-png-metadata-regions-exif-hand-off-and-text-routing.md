---
insight:
  id: DEC-018
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
  - "src/png/**"
  - "src/error.rs"
  - "fuzz/**"

tags:
  - parser
  - png
  - metadata
  - policy
---

# DEC-018: PNG metadata regions — Exif hand-off, text routing, and what is located but not decoded

## Decision

`png::read` runs the DEC-017 walk and then extracts, in chunk order, from the recorded chunks **and from the chunks that follow `IEND`**:

1. **Exif** from `eXIf` chunks: the first is primary, each further one costs an `ExtraExif` issue, and at most `MAX_PNG_EXIF_BLOCKS = 16` are parsed. The payload goes to the unchanged tolerant TIFF engine (`Container::parse`); its fatal error is kept as data on the block, as for JPEG (DEC-006). A payload that starts `Exif\0\0` is accepted with that header skipped and one `ImproperExifHeader` issue, because ExifTool does the same.
2. **Located regions**, borrowed and never decoded or inflated: `Xmp`, `Icc`, `RawProfile`, `Text`.
3. **Issues** in the PNG layer's own channel (`PngIssue`, DEC-017), kept apart from each block's TIFF issues; a combined view names the layer, as `AnyIssue` does for JPEG.

## Context (measured in the SPEC-029 design, ExifTool 13.55, `-a -G1 -s -n`, 17 hand-built shapes)

- `eXIf` must start `II` or `MM` (else "Invalid eXIf chunk", nothing read); a leading `Exif\0\0` is read with the warning "Improper Exif00 header". A zero-length `eXIf` is invalid.
- XMP is read from `iTXt` with keyword `XML:com.adobe.xmp`, uncompressed **and** zlib-compressed, and from a legacy `tEXt` with the same keyword. The same text under another keyword is only a free-text `Comment`. Two XMP chunks are both read.
- `iCCP` is a profile name, a compression-method byte and a zlib stream; ExifTool inflates it.
- ImageMagick-style **`Raw profile type exif` / `xmp` in `tEXt` and `zTXt`** are decoded by ExifTool (hex text, zlib for `zTXt`); an `eXIf` chunk and a raw-profile Exif chunk are both read.
- Chunks after `IDAT` and after `IEND` are read, with only minor warnings; the walk must therefore look past `IEND` for metadata.

## Policy detail

- **Keyword rule (tEXt, zTXt, iTXt):** the keyword is the bytes up to the first NUL, 1 to 79 bytes. No NUL in the chunk or a truncated `iTXt` header is `BadTextChunk` with no region (nothing to locate). An empty keyword, a keyword of 80 bytes or more, or an `iTXt` compression flag other than 0 or 1, is `BadTextChunk` **and the region is still located** (ExifTool reads all three: a long keyword as a tag named by it, an empty one as a tag named `Tag`, a flag of 2 as a compressed value): a long keyword is routed by the same rules as a short one, a flag other than 0 is treated as compressed. `iTXt` has a compression flag, a method byte, a language tag and a translated keyword (each NUL-terminated). `zTXt` is always compressed (method byte after the keyword). Keyword comparison is byte-exact and case-sensitive (the PNG specification's rule).
- **Routing:** keyword `XML:com.adobe.xmp` in `iTXt`, `tEXt` or `zTXt` gives `Xmp` (`compressed` is true for `zTXt`; ExifTool inflates it and parses XMP, verified in the SPEC-029 verify). A keyword starting `Raw profile type ` gives `RawProfile` carrying the profile name that follows it (for example `exif`, `xmp`, `icc`, `iptc`, `APP1`). Every other keyword gives `Text`. `iCCP` gives `Icc`: name (1 to 79 bytes), method byte, then the compressed data; a bad header is `BadIccpChunk`.
- **Located, not decoded.** Regions carry `compressed: bool` where the format can be compressed. Nothing is inflated and no hex is decoded: decoding a raw profile needs an owned buffer (DEC-002 keeps the core zero-copy) and inflation needs a zlib implementation (the crate has no dependency). Both are recorded as audit limitations, not hidden: SPEC-032 reports a compressed or raw-profile region as metadata the audit could not read. Decoding is a decision for a later stage.
- **Trailer chunks.** After the walk, the trailer is scanned with the same chunk framing (length, type, data, CRC not required), bounded by `MAX_PNG_TRAILER_CHUNKS = 4096`, stopping silently at the first chunk that does not fit. Only metadata chunks yield regions or blocks, flagged `after_iend`. Trailer framing problems are **not** issues (a trailer is often arbitrary bytes). Every region and block also knows `after_idat`.
- **Offsets.** Every region and block reports the input offset of its chunk, never an index (trailer chunks are not in `Png::chunks()`); an Exif block's `tiff_at` is the input offset of the first TIFF byte.
- **Bounds.** At most `MAX_PNG_REGIONS = 65_536` regions; the next costs one `TooManyRegions` issue and extraction stops. Extraction issues are at most one per chunk, so the PNG-layer total is bounded by constants.
- **Cleanliness:** clean iff no PNG-layer issue and every Exif block `Ok` and clean.

## Alternatives considered

- **Inflate zlib (XMP, iCCP, zTXt) in-crate.** Rejected for now: a decompressor is a large, untrusted-input parser of its own (its own fuzz target, bomb limits); out of this stage's scope. Located-and-flagged keeps the audit honest at no cost.
- **Decode raw profiles into owned buffers.** Rejected: breaks borrowed-slice results and the writer's offset model; revisit with M1d, which must strip these chunks anyway.
- **Ignore chunks after `IEND`.** Rejected: ExifTool reads them, so an audit that ignored them would miss metadata a reader sees.
- **Treat any non-first `eXIf` as ignorable.** Rejected: it is an issue and a parsed block (bounded), matching the JPEG rule.

## Consequences

- **Positive:** same shape as JPEG's `read`; every metadata container the audit must report is located with an offset; limits are explicit.
- **Negative:** compressed XMP and raw-profile Exif are visible but not readable until a later decision; the second scan over the trailer is extra code.
- **Neutral:** `Error` gains `ImproperExifHeader`, `BadTextChunk`, `BadIccpChunk`, `TooManyRegions` (additive).

## Clarifications from the SPEC-029 build and verify (2026-11-21)

- **Halt rule:** any walker halt (not only a cap) suppresses all extraction issues, as the last issue is always the halting one; regions and blocks already found are kept.
- **Issues per chunk:** at most two (a further `eXIf` that also starts `Exif\0\0` gives `ExtraExif` then `ImproperExifHeader`); `ImproperExifHeader` is recorded only for a parsed block, so the 17th and later `eXIf` cost only `ExtraExif`.
- **`TooManyRegions`** stops all extraction, Exif blocks included, at the chunk that did not fit.
- **`after_idat`** is true for a trailer chunk when any `IDAT` precedes it, walked or in the trailer.
- **Names and keywords** are `&[u8]` (Latin-1 by the PNG specification), borrowed.
- **Case:** keyword matching is case-sensitive; ExifTool treats `xml:com.adobe.xmp` as plain text and decodes `iTXt` raw profiles (measured).
- **Issue count:** there is no public constant bounding `png_issues()` beyond the sum of the walker's and the extraction's own bounds (at most two per chunk over at most `MAX_PNG_CHUNKS` plus `MAX_PNG_TRAILER_CHUNKS` chunks).

## Validation

Revisit if SPEC-031 shows ExifTool reading a metadata chunk class this routing misses, if real files make raw-profile Exif common enough that the audit's "could not read" finding is noisy, or if the writer needs per-region provenance.

## References

- Related specs: SPEC-029, SPEC-031, SPEC-032
- Related decisions: DEC-017 (walk), DEC-006 and DEC-012 (JPEG analogue), DEC-002 (borrowed slices), DEC-001
- External: PNG Specification Third Edition (`eXIf`, `iTXt`, `tEXt`, `zTXt`, `iCCP`); ExifTool 13.55 behaviour measured in the SPEC-029 design

## Clarifications from the SPEC-031 verify (2026-11-21)

Measured with ExifTool 13.55 (records `png-iccp-name-*` and `png-xmp-itxt-*` in `tests/oracle-fixtures/exiftool/`):

- **`iCCP` name.** ExifTool reads the profile (the `ICC-header` tags) whatever the length of the name: 0, 1, 79, 80, 81 and 200 bytes all give them (it prints no `PNG:ProfileName` for an empty name). So the name is the bytes before the first NUL, of any length; an empty name or one of 80 bytes and more is malformed but located: one `BadIccpChunk` issue and the `Icc` region, the rule already used for a long text keyword. A chunk with no NUL, or with no method byte, still gives `BadIccpChunk` and no region. (ExifTool prints no name for a true no-NUL `iCCP`, only a 14-byte binary `ICC_Profile` for a chunk like `diamondABCDEFG`; for a chunk with a name but no method byte it prints `PNG:ProfileName`; it reads no ICC header from either.)
- **`iTXt` language tag.** With the keyword `XML:com.adobe.xmp`, flag 0 or 1, a non-empty language tag (`en-US`, `x` and `EN` measured) makes the chunk plain text for ExifTool, not XMP, whatever the translated keyword; an empty language tag is XMP even with a translated keyword. So such a chunk is located as a `Text` region, not `Xmp`.

## Clarifications from the SPEC-031 verify, round 2 (2026-11-21)

- **Empty text keyword.** ExifTool 13.55 exposes the value of a `tEXt`, `zTXt` or `iTXt` with an empty keyword (compressed, uncompressed and language-tagged `iTXt` included) as a tag named `Tag`. So such a chunk is located as a `Text` region (an empty keyword is never XMP or a raw profile) and records `BadTextChunk`, the rule already used for a keyword of 80 bytes or more. No NUL at all, and a truncated `iTXt` header, stay `BadTextChunk` with no region.
- **Chunks we cannot parse.** ExifTool still exposes some of them as raw binary (`TextualData`, `InternationalText`, `CompressedText`) or, for an `iCCP` with a name but no method byte, as `PNG:ProfileName`. We yield a `BadTextChunk` or `BadIccpChunk` issue and no region for these. A consumer (SPEC-032) must treat any such issue as "metadata present that we could not read".
