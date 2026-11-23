---
insight:
  id: DEC-021
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

created_at: 2026-11-22
supersedes: null
superseded_by: null
status: accepted
deciders: [jysf, claude]
affected_scope:
  - "src/write/**"
  - "src/png/**"
  - "src/audit/**"
  - "fuzz/**"

tags:
  - writer
  - png
  - policy
---

# DEC-021: PNG strip: whole-chunk removal, opaque carriers and the eXIf chunk

## Decision

`write::strip` of a PNG follows DEC-013 unchanged (a pure function; `Remove` and same-length `Patch` only; never longer than the input; fail closed with typed refusals; the output re-audited). The PNG-specific rules:

1. **The unit of removal is the chunk** (length, type, data, CRC), found by the chunk walk, never by the library's lists alone. Every `eXIf`, text, `iCCP` and `tIME` chunk the walk finds is considered, wherever it lies (before `IDAT`, after `IDAT`, after `IEND`). Kept chunks are byte-identical, so their CRCs stay valid, including a kept chunk whose stored CRC was already wrong.
2. **Which chunks go, by the audit's category (DEC-020):** `eXIf` by its entries (rule 3); a text chunk by its keyword (`Author`, `Copyright` Authorship; `Creation Time` Timestamps; `Software` Software; `XML:com.adobe.xmp` Xmp; everything else FreeText), compressed or not; `iCCP` Icc; `tIME` of any length Timestamps; bytes after `IEND` that are not a chunk (the audit's `Trailer`) OtherSegment. A category the policy does not contain is left (so `Gps`, `GpsAndDevice`, `AllPii` keep XMP and ICC, as for JPEG; the preview's `KeptUnread` says so).
3. **`eXIf` is the JPEG Exif rule applied to the chunk's TIFF data (DEC-013 part 4 and amendment B).** A policy containing every Exif-set category removes every `eXIf` chunk whole (Orientation goes with it; documented consequence, as for JPEG). A policy containing none leaves them. A policy containing some deletes the matching entries in place with the existing Exif editor (`src/write/exif.rs`), then **patches the chunk CRC** (the one net-new patch: CRC-32 over the chunk type and the edited data, computed from the planned bytes), or refuses with the existing `Uneditable` reasons.
4. **Opaque carriers.** Some chunks carry metadata the library cannot read, and a kept one would leave what the policy asked to remove: ImageMagick raw profiles (`Raw profile type exif`/`APP1`/`xmp`/`iptc`/`icc`, DEC-018, DEC-020 clause 5) and any other chunk type ExifTool 13.55 decodes into Exif or XMP tags that the audit does not list (the SPEC-037 build measures the grid: at least `zxIf`, `tXMP`, `meTa`; and lists what it finds in this DEC). An opaque carrier is removed whenever the policy contains any tag category, `Xmp` or `Iptc` (that is, by every preset; a `Custom` policy with only `Icc`, `Jumbf` or nothing leaves it; with `OtherSegment` it goes, see the SPEC-037 build clarifications). Carriers that are not metadata for any preset (`dSIG`, `cpIp`, `gIFx`, `fRAc`, ...) are `OtherSegment` and go only with `Everything`. Where the audit does not yet locate such a chunk the build extends the reader and audit (`src/png`, `src/audit`, DEC-020 amended) so the output re-audit sees it, rather than letting the strip leave it silently.
5. **Refusals (typed, no bytes):** the walk was halted by a cap (`MAX_PNG_CHUNKS`, `MAX_PNG_ISSUES`) or a chunk's length runs past the end of the input, so the bytes after it were never audited (`Unstrippable`, with the input offset); an `eXIf` edit that cannot be made exactly (the existing `Uneditable` reasons); a removal that would cut a kept byte (`Overlap`); the self-check failing (`VerifyFailed`). A missing `IEND`, a bad CRC on a removed chunk, an unparseable `eXIf` that the policy removes whole, are not refusals. An unparseable `eXIf` that a partial policy would need to edit is `Uneditable`.
6. **The self-check (DEC-013 part 7) for PNG:** (a) the output's audit has no finding in a removed category and no opaque carrier the policy removes; (b) the multiset of findings in the kept categories equals the input's; (c) `IHDR`, `PLTE`, `IDAT` and every other kept chunk are byte-identical and in order (the I1 pixel-integrity statement for PNG); (d) the output walks with no new issue kind; (e) stripping the output again changes nothing; (g) the entry inventory equals the input's minus the deleted entries; (h) zeroed ranges read zero.
   **Exemption (clause 4 in check (a) and (b)).** A raw profile and a `Carrier` chunk have the audit category `OtherSegment`, which `Gps`, `GpsAndDevice`, `AllPii` and `All` do not contain, yet clause 4 removes them under every preset. So check (a) also fails when the output still has a `RawProfile` or `Carrier` region and the policy removes carriers, and check (b) leaves both kinds out of the kept multiset on both sides in that case (they are not "kept"). A `Custom` policy of `OtherSegment` alone removes them as well, by their own category: "leaves it" in clause 4 applies to `Icc`, `Jumbf` and the empty policy, not to `OtherSegment`, since the output audit would otherwise keep an `OtherSegment` finding the policy removes. The bytes after `IEND` are one `OtherSegment` finding (DEC-020): with that category in the policy they go whole, and a chunk in them that the policy would otherwise keep and that has findings of its own (a located region or an `eXIf`) makes the strip `Overlap` instead (a kept chunk would be cut). Without it the trailer's chunks are decided one by one and the kept bytes are compared by count, less what was removed from them.
7. **Dispatch and honesty.** `strip` accepts `Sniffed::Png`; WebP stays `Unsupported` until SPEC-038. `ContainerKind::strippable()` (DEC-020 clause 6) stays false for PNG until SPEC-039 shows ExifTool agrees (I3) and flips it, so no report claims a removal before then.

## Context

STAGE-008 shipped the reader and the audit for PNG (DEC-017, DEC-018, DEC-020). The SPEC-036 verify showed that ExifTool decodes more PNG chunk types than the audit lists (`zxIf`, `tXMP`, `meTa`, ... printed as their own tags) and that a raw profile is a real Exif carrier, which is why rule 4 exists: a strip that works from the audit's lists alone would leave GPS behind.

## Alternatives considered

- Refuse when a raw profile would remain: rejected, it would make every ImageMagick or GIMP PNG unstrippable by any preset but `Everything`.
- Leave a raw profile under a partial policy (the XMP precedent): rejected, an XMP block is declared kept by the preset's design and shown `KeptUnread`; a raw profile is a positively identified Exif carrier.
- Whole-chunk `eXIf` removal only (no in-place edit): rejected, JPEG already has the entry editor (SPEC-021) and a PNG that keeps Orientation under `Gps` is the common case.

## Consequences

- A PNG strip under any preset also removes raw-profile and other opaque-carrier chunks, including the tags in them that the policy would have kept (documented, like Orientation under `All`).
- The writer is the first place that computes a PNG CRC; `src/png/crc.rs` is reused.
- `rewrite_png` becomes the fuzz target of this writer.

## Validation

Unit tests per rule; the self-check on every fixture; I1 recomputation test; `rewrite_png` 60 s smoke; ExifTool agreement is SPEC-039 (I3).

## Clarifications from the SPEC-037 build (2026-11-22)

**The measured carrier table.** ExifTool 13.55, `-a -u -G1 -s -n`, one chunk per file in a minimal valid PNG (re-measured for this fill; the file contents are a TIFF with Make and a GPS latitude reference for the Exif carriers, an XMP packet with a creator for the XML carriers). "Located" is what `png::read` finds (`PngRegion`), "category" what the audit and this writer give it.

| Chunk type | ExifTool prints | Located as | Audit category | Removed by |
|---|---|---|---|---|
| `eXIf` | `IFD0:Make`, `GPS:GPSLatitudeRef` | Exif block (`eXIf`) | by entry (the nine Exif categories, `Icc` for 0x8773) | clause 3 |
| `zxIf` and `eXIf` in any other ASCII case (`zxif`, `exif`, `EXIF`, `ZxIf`, ... 31 spellings; the exact `eXIf` is the Exif chunk, rule 3) | `IFD0:Make`, `GPS:GPSLatitudeRef` (a bare TIFF, or `\0`, a 4-byte length and a zlib stream; every spelling but the exact one adds a `[minor]` warning "chunk should be eXIf" or "zxIf") | `Chunk` (`ExifCarrier`, `RegionKind::Carrier`) | `OtherSegment` | every preset (clause 4) |
| `tXMP` | `XMP-dc:Creator` | `Chunk` (`XmlCarrier`, `Carrier`) | `OtherSegment` | every preset |
| `meTa`, `seAl` | `XML-dc:Creator` | `Chunk` (`XmlCarrier`, `Carrier`) | `OtherSegment` | every preset |
| `txmp` (lower case), and every other case spelling of any other type (`CABX`, `Time`, `dsig`, `TEXT`, ...) | nothing | not located (type bytes are case-sensitive EXCEPT `zxIf` and `eXIf`, re-measured for every metadata type in the SPEC-037 fix round) | none | never |
| `Raw profile type exif`, `...xmp`, `...iptc`, ... (`tEXt`, `zTXt`, `iTXt`; also `raw profile type ...`) | the profile's tags when its payload decodes (the minimal sample here is malformed and prints only a warning) | `RawProfile` | `OtherSegment` | every preset |
| `caBX` | group `JUMBF` for a valid box (Part A; the minimal box in this sample prints only a warning) | `Chunk` (`Jumbf`) | `Jumbf` | policies with `Jumbf` (`Everything`) |
| `dSIG` | `PNG:DigitalSignature` | `Chunk` (`Other`, `RegionKind::OtherChunk`) | `OtherSegment` | `OtherSegment` only (`Everything`) |
| `cpIp` | `PNG:OLEInfo` | `OtherChunk` | `OtherSegment` | `OtherSegment` only |
| `gIFx`, `gIFg`, `gIFt` | `PNG:GIFApplicationExtension`, `GIFGraphicControlExtension`, `GIFPlainTextExtension` | `OtherChunk` | `OtherSegment` | `OtherSegment` only |
| `fRAc` | `PNG:FractalParameters` | `OtherChunk` | `OtherSegment` | `OtherSegment` only |
| `tIME` (any length, in the walk or after `IEND`) | `PNG:ModifyDate` (`Trailer:ModifyDate` after `IEND`) | `Chunk` (`Time`, `RegionKind::Time`) | `Timestamps` | policies with `Timestamps` |
| `iTXt` XMP (`XML:com.adobe.xmp`, empty language tag) | `XMP-dc:Creator` | `Xmp` | `Xmp` | policies with `Xmp` |

Chunks ExifTool prints that are structure or rendering data (`pHYs`, `gAMA`, `sRGB`, `cHRM`, `bKGD`, `tRNS`, `sBIT`, `oFFs`, `sCAL`, `sTER`, `vpAg`, `acTL`, `cICP`, `pCAL`, `sPLT`, `hIST`, `iDOT`, `gdAT`) are not located and never removed. The table is data for a test (`tests/strip_png.rs` iterates the carrier and opaque types).

**The four judgement calls of Part A, now part of the decision:**

1. **A `Custom` policy with `OtherSegment` removes the carriers** (clause 4 said a `Custom` policy of `OtherSegment` alone leaves them; that was wrong). A raw profile and a `Carrier` chunk have the audit category `OtherSegment`, so a policy that contains it must leave none, or the output audit would keep a finding the policy removes. "Leaves it" applies to `Icc`, `Jumbf`, `EmbeddedImage` and the empty policy.
2. **The bytes after `IEND` go whole under `OtherSegment`, with `Overlap` for a chunk in them the policy would keep.** The audit gives them one `OtherSegment` finding (the `Trailer` region). With that category the writer removes everything after `IEND` as one range; a chunk in it that the policy would otherwise keep and that has findings of its own (a located region, or an `eXIf` chunk the mode does not remove whole) would be cut, so the strip refuses with `Overlap` at that chunk's offset (the JPEG precedent for a kept embedded image). Without `OtherSegment` the trailer's chunks are decided one by one, at most `MAX_PNG_TRAILER_CHUNKS` of them (more is `Unstrippable`, `TooManySegments`, unless the whole trailer goes).
3. **`caBX` is `Jumbf`**, as a JUMBF segment is in a JPEG: kept by `AllPii` and `All`, removed by `Everything`.
4. **Carriers are removed under `Gps` although plain XMP stays.** Consequence, stated plainly: `Gps`, `GpsAndDevice` and `AllPii` keep an `XML:com.adobe.xmp` chunk (the preset's design, shown `KeptUnread`) but remove a raw profile, `zxIf`, `tXMP`, `meTa` and `seAl`, which can hold the very Exif or XMP GPS the preset was asked to remove and which the library cannot read to check. A caller that wants an ImageMagick raw profile kept must build a `Custom` policy without any tag category, `Xmp` or `Iptc` and without `OtherSegment`.

**The `rewrite_png` target (the validation of this decision).** `fuzz/fuzz_targets/rewrite_png.rs` with the harness `fuzz/shared/rewrite_png_invariants.rs`, written from this decision and DEC-020 and never from `src/write/png.rs`: its own chunk walk with the caps, its own bitwise CRC-32, its own trailer scan and its own typing, keyword, carrier, `eXIf` mode and overlap rules predict the exact output or the exact refusal. Eleven guards (prediction, records, length, removed categories, kept findings, edited CRC, no carrier left, idempotence, determinism, refusal class, `VerifyFailed` never), each with at least two plants in `tests/fuzz_layout.rs` that turn it red; 338 committed seeds (one per rule above, the walk refusals, a kept bad CRC, each policy byte). The 60 s smoke found two harness-model gaps and no library bug: a `zTXt` with a method byte and an empty body is still located, and an `Icc`-category entry inside an `eXIf` chunk goes with the chunk under `All`.

## References

DEC-001, DEC-006, DEC-008, DEC-009, DEC-013, DEC-017, DEC-018, DEC-020; SPEC-037.

## Clarifications from the SPEC-037 fix round (2026-11-22)

1. **Chunk-type case (H1).** The verify measured, and the fix round re-measured on ExifTool 13.55 with all 16 ASCII-case spellings of every metadata chunk type (`eXIf`, `zxIf`, `tXMP`, `meTa`, `seAl`, `caBX`, `tIME`, `iCCP`, `tEXt`, `zTXt`, `iTXt`, `dSIG`, `cpIp`, `gIFx`, `gIFg`, `gIFt`, `fRAc`): only `eXIf` and `zxIf` fold. Every spelling of those two decodes, so a mixed-case `EXIF` or `ZxIf` chunk is a carrier like `zxif`: `chunk_class` folds the four bytes, every preset removes it, the audit reports `Carrier`. Every other type stays case-sensitive. (This corrects the earlier "type bytes are case-sensitive" rule in the table above and in DEC-020 clause 5.)
