---
insight:
  id: DEC-022
  type: decision
  confidence: 0.65
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

created_at: 2026-11-23
supersedes: null
superseded_by: null
status: accepted
deciders: [jysf, claude]
affected_scope:
  - "src/write/**"
  - "src/webp/**"
  - "src/audit/**"
  - "fuzz/**"

tags:
  - writer
  - webp
  - policy
---

# DEC-022: WebP strip: chunk removal, the RIFF size, the VP8X flags and carriers

## Decision

`write::strip` of a WebP follows DEC-013 and DEC-021 unchanged (a pure function; `Remove` and same-length `Patch` only; never longer than the input; fail closed with typed refusals; the output re-audited). The WebP-specific rules:

1. **The unit of removal is the top-level RIFF chunk** (four-character code, size, data and its pad byte), found by the chunk walk, never by the library's lists alone. Every `EXIF`, `XMP `/`XMP\0` and `ICCP` chunk the walk finds is considered wherever it lies, including after the declared RIFF size (ExifTool reads chunks beyond the declared size, measured), after an empty null chunk (ExifTool stops there, the walker reads on: strip more, never less), and a 17th and later `EXIF` chunk (the reader lists 16, ExifTool reads all). Kept chunks are byte-identical. Chunks inside an `ANMF` frame are never metadata and are never walked.
2. **Which chunks go, by category:** `EXIF` by its entries (rule 3); `XMP `/`XMP\0` Xmp; `ICCP` Icc; a length-0 chunk of those types too (a harmless removal). A category the policy does not contain is left (so `Gps`, `GpsAndDevice`, `AllPii` keep XMP and ICC, as for JPEG and PNG).
3. **`EXIF` is the Exif rule of DEC-013 part 4 and amendment B applied to the chunk's TIFF data** (a chunk with an `Exif\0\0` prefix is edited at its TIFF start): a policy containing every Exif-set category removes every `EXIF` chunk whole; one containing none leaves them; one containing some deletes the matching entries in place with the existing Exif editor (`src/write/exif.rs`) or refuses with the existing `Uneditable` reasons (more than 16 `EXIF` chunks, an unparseable block). A WebP `EXIF` chunk has no CRC, so there is no checksum patch.
4. **Opaque carriers.** ExifTool 13.55 decodes more RIFF chunk types in a WebP than the audit lists (measured at the SPEC-038 design: `LIST` of type `INFO` printing `Artist`, `Copyright`, `Software`, `Comment`, `DateCreated`; a `JUNK` chunk holding text printing `TextJunk`; `C2PA` decoded as JUMBF; the SPEC-038 build measures the full grid). A chunk that carries metadata the audit cannot read is an opaque carrier and is removed by every preset (whenever the policy contains any tag category, `Xmp`, `Iptc` or `OtherSegment`; DEC-021 clause 4 and its SPEC-037 clarifications apply). A `C2PA` chunk is a `Jumbf` region (kept by `AllPii` and `All`, removed by `Everything`, with the same stated residual as a PNG `caBX`). Chunks that are not metadata for any preset are `OtherSegment` (`Everything` only). The build lists every row in this DEC and extends the reader and audit (`src/webp`, `src/audit`, DEC-020 amended) so the output re-audit sees what the strip removes.
5. **Case.** FourCC matching is measured per type, not assumed: SPEC-030 and SPEC-034 measured `EXIF`/`XMP `/`ICCP` case-sensitive, and the SPEC-037 verify found `eXIf` and `zxIf` fold case in PNG. The build measures all 16 spellings of every type in the table before writing any rule, and pins the result.
6. **The RIFF size.** After a removal the 4-byte little-endian size at offset 4 is patched (same length) by one rule: the new declared size is the declared size minus the bytes removed from inside the declared range (the bytes from offset 8 to 8 plus the declared size). A file whose declared size equals its length minus 8 stays consistent; a file whose declared size lied keeps its lie by the same amount; bytes removed beyond the declared end change nothing. An arithmetic underflow refuses (`Unstrippable`).
7. **The `VP8X` flags.** When a `VP8X` is present and the strip removes every `ICCP`, every `EXIF` or every `XMP ` chunk, the matching flag bit (`0x20` ICC, `0x08` Exif, `0x04` XMP) is cleared by a same-length patch, so a stripped file does not claim metadata it lacks; flags of kept metadata, and the alpha and animation bits, are untouched. A `VP8X` that is not the first chunk or is shorter than 10 bytes is patched only if it is the one the reader reports (`Webp::vp8x`).
8. **Refusals (typed, no bytes):** the walk was halted by a cap or a chunk runs past the end of the input before the bytes after it were audited (`Unstrippable`); an `EXIF` edit that cannot be made exactly (`Uneditable`); a removal that would cut kept bytes (`Overlap`); the RIFF size or flag rule cannot be applied (`Unstrippable`); the self-check failing (`VerifyFailed`). A missing pad byte, a wrong declared size, trailing non-chunk bytes and an unpadded odd chunk are not refusals (the removal of a chunk takes its pad byte if present).
9. **The self-check (DEC-013 part 7) for WebP:** (a) the output's audit has no finding in a removed category and no opaque carrier the policy removes; (b) the multiset of findings in the kept categories equals the input's; (c) every kept chunk is byte-identical and in order, and the image chunks (`VP8 `, `VP8L`, `ALPH`, `ANIM`, `ANMF`, `VP8X` apart from the flag bits) are unchanged; (d) the output walks with no new issue kind; (e) stripping the output again changes nothing; (f) the declared size relation of rule 6 holds and the `VP8X` flags agree with the chunks present; (g) the entry inventory equals the input's minus the deleted entries; (h) zeroed ranges read zero.
10. **Dispatch and honesty.** `strip` accepts `Sniffed::Webp`; `ContainerKind::strippable()` stays false for WebP until SPEC-039 shows ExifTool agrees and flips it.

## Context

PNG strip (DEC-021) showed that a strip built from the audit's lists and a model typed from the same measurements leaves what the tool decodes and the audit does not list, and that chunk-type case folding must be measured per type. WebP adds two things PNG does not have: a size field and flag bits that must stay consistent after a removal.

## Alternatives considered

- Recompute the RIFF size as the output length minus 8: rejected, it would silently repair a file whose declared size was wrong, which is more than the policy asked; the subtraction rule keeps relationships.
- Leave the `VP8X` flags alone: rejected, a stripped file should not claim an `EXIF` chunk it no longer has, and some decoders trust the flags.
- Refuse files with a `LIST/INFO` chunk: rejected, it would make common WebPs unstrippable.

## Consequences

- A WebP strip under any preset also removes `LIST/INFO` and the other opaque carriers (documented, like raw profiles in PNG).
- The audit gains WebP carrier regions (`RegionKind::Carrier`/`OtherChunk` already exist from SPEC-037).
- `rewrite_webp` becomes the fuzz target of this writer.

## Validation

Unit tests per rule; the self-check on every fixture; I1 recomputation; `rewrite_webp` 60 s smoke; a gated live ExifTool check of the outputs (SPEC-038), formal parity in SPEC-039.

## References

DEC-001, DEC-006, DEC-009, DEC-013, DEC-019, DEC-020, DEC-021; SPEC-038.
