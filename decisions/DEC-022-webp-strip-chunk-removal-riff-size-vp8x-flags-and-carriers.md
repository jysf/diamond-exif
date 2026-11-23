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
7. **The `VP8X` flags.** When a `VP8X` is present and the strip removes every `ICCP`, every `EXIF` or every `XMP ` chunk, the matching flag bit (`0x20` ICC, `0x08` Exif, `0x04` XMP) is cleared by a same-length patch, so a stripped file does not claim metadata it lacks; flags of kept metadata, and the alpha and animation bits, are untouched. The bit is cleared in **every** `VP8X` chunk whose data holds 4 bytes or more, wherever it lies (measured, clarification 7): ExifTool prints the flags of each such chunk and reports the last one, so a stripped file must not announce a chunk it no longer has in any of them. A `VP8X` with fewer than 4 data bytes prints no flags and is kept byte for byte; the size rule is unchanged.
8. **Refusals (typed, no bytes):** the walk was halted by a cap or a chunk runs past the end of the input before the bytes after it were audited (`Unstrippable`); an `EXIF` edit that cannot be made exactly (`Uneditable`); a removal that would cut kept bytes (`Overlap`); the RIFF size or flag rule cannot be applied (`Unstrippable`); the self-check failing (`VerifyFailed`). A missing pad byte, a wrong declared size, trailing non-chunk bytes and an unpadded odd chunk are not refusals (the removal of a chunk takes its pad byte if present).
9. **The self-check (DEC-013 part 7) for WebP:** (a) the output's audit has no finding in a removed category and no opaque carrier the policy removes; (b) the multiset of findings in the kept categories equals the input's; (c) every kept chunk is byte-identical and in order, and the image chunks (`VP8 `, `VP8L`, `ALPH`, `ANIM`, `ANMF`, `VP8X` apart from the flag bits) are unchanged; (d) the output walks with no new issue kind; (e) stripping the output again changes nothing; (f) the declared size relation of rule 6 holds and the `VP8X` flags agree with the chunks present; (g) the entry inventory equals the input's minus the deleted entries; (h) zeroed ranges read zero.
10. **Dispatch and honesty.** `strip` accepts `Sniffed::Webp`; `ContainerKind::strippable()` stays false for WebP until SPEC-039 shows ExifTool agrees and flips it.

## The measured table (SPEC-038, ExifTool 13.55, `-a -u -G1 -H -s`)

One chunk per file in a minimal WebP, the chunk before the image data, after it, beyond the declared RIFF size and in a file with no `VP8X`. Every row printed at least one tag from its chunk in every position but after an empty null chunk (ExifTool stops reading there; the strip reads on and removes more, never less). The data is `tests/support/webp_carriers.rs`; the fuzz harness types its own copy and a test compares the two row by row.

| Chunk type | Located as | Category | Removed by | A tag ExifTool 13.55 printed from it |
|---|---|---|---|---|
| `LIST` | Carrier region | OtherSegment | every preset (any tag category, `Xmp`, `Iptc` or `OtherSegment`) | `RIFF:Artist` |
| `RIFF` | Carrier region | OtherSegment | every preset (any tag category, `Xmp`, `Iptc` or `OtherSegment`) | `RIFF:Artist` |
| `IDIT` | Carrier region | OtherSegment | every preset (any tag category, `Xmp`, `Iptc` or `OtherSegment`) | `RIFF:DateTimeOriginal` |
| `JUNK` (only when its data is text) | Carrier region | OtherSegment | every preset (any tag category, `Xmp`, `Iptc` or `OtherSegment`) | `RIFF:TextJunk` |
| `JUNQ` | Carrier region | OtherSegment | every preset (any tag category, `Xmp`, `Iptc` or `OtherSegment`) | `RIFF:OldXMP` |
| `SEAL` | Carrier region | OtherSegment | every preset (any tag category, `Xmp`, `Iptc` or `OtherSegment`) | `XML-dc:Creator` |
| `_PMX` | Carrier region | OtherSegment | every preset (any tag category, `Xmp`, `Iptc` or `OtherSegment`) | `XMP-dc:Creator` |
| `aXML` | Carrier region | OtherSegment | every preset (any tag category, `Xmp`, `Iptc` or `OtherSegment`) | `XML-dc:Creator` |
| `iXML` | Carrier region | OtherSegment | every preset (any tag category, `Xmp`, `Iptc` or `OtherSegment`) | `XML-dc:Creator` |
| `afsp` | Carrier region | OtherSegment | every preset (any tag category, `Xmp`, `Iptc` or `OtherSegment`) | `RIFF:Afsp` |
| `bext` | Carrier region | OtherSegment | every preset (any tag category, `Xmp`, `Iptc` or `OtherSegment`) | `RIFF:Description` |
| `guan` | Carrier region | OtherSegment | every preset (any tag category, `Xmp`, `Iptc` or `OtherSegment`) | `RIFF:Guano` |
| `list` | Carrier region | OtherSegment | every preset (any tag category, `Xmp`, `Iptc` or `OtherSegment`) | `RIFF:ListType` |
| `labl` | Carrier region | OtherSegment | every preset (any tag category, `Xmp`, `Iptc` or `OtherSegment`) | `RIFF:CuePointLabel` |
| `note` | Carrier region | OtherSegment | every preset (any tag category, `Xmp`, `Iptc` or `OtherSegment`) | `RIFF:CuePointNote` |
| `ltxt` | Carrier region | OtherSegment | every preset (any tag category, `Xmp`, `Iptc` or `OtherSegment`) | `RIFF:LabeledText` |
| `olym` | Carrier region | OtherSegment | every preset (any tag category, `Xmp`, `Iptc` or `OtherSegment`) | `Olympus:FileNumber` |
| `ID3 ` | Carrier region | OtherSegment | every preset (any tag category, `Xmp`, `Iptc` or `OtherSegment`) | `ID3v2_3:Artist` |
| `id3 ` | Carrier region | OtherSegment | every preset (any tag category, `Xmp`, `Iptc` or `OtherSegment`) | `ID3v2_3:Artist` |
| `gps0` | Carrier region | OtherSegment | every preset (any tag category, `Xmp`, `Iptc` or `OtherSegment`) | `RIFF:GPSLatitude` |
| `SGLT` | Carrier region | OtherSegment | every preset (any tag category, `Xmp`, `Iptc` or `OtherSegment`) | `RIFF:Accelerometer` |
| `SLLT` | Carrier region | OtherSegment | every preset (any tag category, `Xmp`, `Iptc` or `OtherSegment`) | `RIFF:GPSLatitude` |
| `C2PA` | Jumbf region | Jumbf | `Jumbf` only (`Everything`) | `JSON:Exif_GPSLatitude` |
| `CSET` | OtherChunk region | OtherSegment | `OtherSegment` only (`Everything`) | `RIFF:CodePage` |
| `acid` | OtherChunk region | OtherSegment | `OtherSegment` only (`Everything`) | `RIFF:AcidizerFlags` |
| `ds64` | OtherChunk region | OtherSegment | `OtherSegment` only (`Everything`) | `RIFF:RIFFSize64` |
| `fact` | OtherChunk region | OtherSegment | `OtherSegment` only (`Everything`) | `RIFF:NumberOfSamples` |
| `fmt ` | OtherChunk region | OtherSegment | `OtherSegment` only (`Everything`) | `RIFF:Encoding` |
| `inst` | OtherChunk region | OtherSegment | `OtherSegment` only (`Everything`) | `RIFF:UnshiftedNote` |
| `smpl` | OtherChunk region | OtherSegment | `OtherSegment` only (`Everything`) | `RIFF:Manufacturer` |
| `cue ` | OtherChunk region | OtherSegment | `OtherSegment` only (`Everything`) | `RIFF:CuePoints` |
| `plst` | OtherChunk region | OtherSegment | `OtherSegment` only (`Everything`) | `RIFF:Playlist` |
| `gsen` | OtherChunk region | OtherSegment | `OtherSegment` only (`Everything`) | `RIFF:Accelerometer` |

Chunk types that printed only an `Unknown_<code>` binary dump under `-u`, or nothing, are not metadata, are never located and are never removed: `PAD `, `strd`, `strn`, `PrmL`, `Cr8r`, `wavl`, `slnt`, `DISP`, `avih`, `strh`, `strf`, `vedt`, `ISFT`, `IART`, `INAM`, `ICMT`, `ICOP`, `TEXT`, `COMM`, `umid`, `regn`, `link`, `axml`, `junk`, `pad `, `Junk`, `exif`, `xmp `, `iccp`, `Exif` (`junk` and `Junk` are case spellings of `JUNK`; `exif`, `xmp `, `iccp` and `Exif` of types with a rule).

**Case.** Measured for all 16 ASCII-case spellings of every row's code: no type folds case. Only the exact code decodes (and `ID3 ` and `id3 `, two codes ExifTool knows); the other spellings print only `Unknown_*` tags and are kept. The live test `tests/strip_webp_live.rs` repeats this against the installed ExifTool for every spelling (415 files) and for every row in four positions, plus a depth-3 `RIFF` chunk and an appended second file.

**`JUNK`.** A `JUNK` chunk is a carrier only when its data is printable ASCII (0x20 to 0x7E) followed by zero or more NULs and holds at least one text byte (ExifTool printed `TextJunk`); empty data, all zeros, a NUL before or inside the text and any control or high byte print nothing and the chunk is kept. `junk`, `Junk` are not `JUNK`.

**Two payload notes for the record.** `LIST` of any list type prints tags for the sub-chunks it knows (`INFO`, `INF0`, `adtl`, `exif`, `hdrl` were measured), so every `LIST` is a carrier (below). The committed `ID3 ` fixture declared a frame one byte too large (13 declared, 12 present), for which ExifTool 13.55 printed only `File:ID3Size`; the live test found it, the fixture was corrected (SPEC-038 Part B) and now prints `ID3v2_3:Artist`. The strip's rule (the chunk is a carrier) never depended on it.

## Clarifications (SPEC-038 build)

1. **Every `LIST` is a carrier.** The strip does not parse list types: a `LIST` may hold personal `INFO` tags (`Artist`, `Copyright`, `Comment`, `DateCreated`) or `adtl` labels, and a list type the audit does not know today may be decoded by a later ExifTool. Over-removal of a `LIST` that holds nothing personal is a stated residual (README), the same stance as PNG raw profiles.
2. **`JUNK` is a carrier by the text rule only** (above), because the common `JUNK` is zero padding and must be kept for a WebP written by tools that pad; a text `JUNK` is where ExifTool finds a string.
3. **`C2PA` is a `Jumbf` region** with the same stated residual as a PNG `caBX` (DEC-021 clarifications): `AllPii` and `All` keep it, and a manifest can carry personal assertions (a measured `exif:GPSLatitude` assertion printed `JSON:Exif_GPSLatitude`); only `Everything` removes it.
4. **Grouping.** `SGLT`, `SLLT` and `gps0` are carriers (GPS-sensor chunks; `gps0` and `SLLT` print `GPSLatitude`). `gsen` prints the same `Accelerometer` tag as `SGLT` but holds no position, so it is an opaque `OtherChunk` (`Everything` only); `SGLT` stays with the GPS family. `CSET`, `acid`, `ds64`, `fact`, `fmt `, `inst`, `smpl`, `cue `, `plst` print technical tags of an audio chunk and are `OtherChunk`.
5. **Self-check failures reuse `Check::MpfFields`.** The size relation (clause 9 f) and the flag agreement are the "recorded fields agree with the bytes" check class of DEC-013 part 7; no new `Check` variant was added. A plan whose `apply` fails reports the same.
6. **A shorter tail is not a chunk.** Fewer than 8 bytes after the last chunk halt the walk without an issue the writer refuses; the bytes are kept as they are. A chunk whose declared size runs past the end of the input is refused (`ChunkPastEnd`), at the chunk's offset.
7. **Every `VP8X` ExifTool reads is patched (SPEC-038 fix round, verify L1; replaces the first build's "only the one the reader reports").** Measured on ExifTool 13.55 (`-a -n -RIFF:WebP_Flags`): it prints `WebP_Flags` for every `VP8X` chunk with 4 or more data bytes (4, 5, 8, 9, 10 and 12 measured; 0, 1, 2 and 3 print nothing), before or after the image data, byte 0 of the data being the flags; without `-a` the last one is reported (two chunks, flags `0x20` then `0x08`, reported 8). So the strip clears the ICC, Exif and XMP bits, for each kind whose chunks all went, in every `VP8X` with at least 4 data bytes. The reader still reports only the first `VP8X` of 10 bytes or more (its own issue rules are unchanged). A `VP8X` of 3 bytes or fewer is kept byte for byte.
8. **Overlap cannot occur** on a WebP (chunks never overlap and the header is never a chunk); the fuzz harness treats an `Overlap` answer as a failure.

9. **A chunk coded `RIFF` is an opaque carrier (SPEC-038 verify H1, fix round).** ExifTool 13.55 walks into a top-level chunk coded exactly `RIFF` as a container and prints every tag it finds inside (`EXIF`, `XMP `, `ICCP`, `C2PA`, `LIST`, `IDIT`, `JUNK`, `ID3 ` measured), which the first build never listed. Measured on 13.55 (`-a -u -G1 -s`): the code is matched case-sensitively and exactly (`RF64`, `RIFX`, `riff`, `Riff`, `rIFF` and `RIFf` print `Unknown_*` only and are kept); the form type after the chunk header does not matter (`WEBP`, `AVI `, `WAVE` and an unknown `XXXX` were all walked into); the walk goes in at any depth (1, 2, 3, 6 and 12 measured), before or after the image data, and a second whole `RIFF....WEBP` file appended after the first is just a `RIFF` chunk beyond the declared size to this walker; the chunk's own size is not trusted by ExifTool (a size too short, too long or a bare 4 (the form type only) still printed what lay inside); the first four bytes of the data are always read as the form type (a `LIST` placed directly inside, with no form type, lost its own code and printed nothing), and an empty `RIFF` chunk holds nothing to print. So a chunk coded `RIFF` is located by code alone as a carrier (`ChunkClass::Riff`, `RegionKind::Carrier`, `OtherSegment`), removed whole by every preset under the carrier rule (clause 4), whatever its form type, size or depth. A non-empty one counts as a failed block (the nested bytes are unread, as for a PNG `tXMP`), so the file reads `unreadable` until it is removed. A `RIFF` chunk whose declared size is too short to hold what follows leaves the walker inside it with a bogus next chunk: when that runs past the end the strip refuses (`ChunkPastEnd`, clause 8), fail closed.

10. **Refused containers and refusals worth stating (SPEC-038 fix round, verify L4 and L2).** A RIFF file whose form type is not `WEBP` (`WAVE`, `AVI `, `RIFX`, a lower-case `webp`) is not read as a WebP and `strip` refuses it as `NotJpeg` (an old error name, kept for the unsupported-container answer). An odd-sized chunk without its pad byte followed by more chunks makes the walk read the next chunk one byte off, so the strip refuses it as `ChunkPastEnd` when the bytes do not frame (an unpadded odd chunk at the end of the input is not a refusal).
11. **Completeness is checked against ExifTool, not against this table (SPEC-038 fix round, verify L3).** `tests/strip_webp_sweep_live.rs` (gated like the other live tests) tries every four-character tag id of `exiftool -listx` (2640 distinct), every code of this table, the codes with a rule of their own, the not-metadata list, the structure codes and the RIFF, AVI, WAV and WebP container codes (2700 candidates), each as one chunk in a bare WebP in eight payload shapes (text, a TIFF with an Artist, an XMP packet, an ID3 header, a JUMBF manifest, zeros, a nested RIFF file, a `LIST/INFO` body), runs ExifTool once over all 21600 files and FAILS for any code that prints a tag the bare file does not (other than `Unknown_*`, the `File`, `System` and `ExifTool` groups) unless the code is in this table, is one of `EXIF`, `XMP `, `XMP\0`, `ICCP`, or is in the pinned structure list with exactly the tag names pinned there (`VP8X`: `ImageHeight`, `WebP_Flags`, `ImageSize`, `Megapixels`; `ALPH`: `AlphaCompression`, `AlphaFiltering`, `AlphaPreprocessing`; `ANIM`: `AnimationLoopCount`, `BackgroundColor`; `ANMF`: `Duration`; `VP8 `, `VP8L`: none). Run on ExifTool 13.55 it found no code beyond this table: 40 candidate codes printed a tag, all of them rows here or the codes with a rule or the structure list. `-listx` does not list the chunk types ExifTool decodes as sub-directories (`bext`, `gps0`, `SGLT`, `SLLT`, `olym`, `ID3 `, `smpl`, `acid`, `CSET`, `ds64`, `gsen`, ...), which is why the table's own codes and the container codes are added; a type that is neither listed by `-listx` nor in this table is outside what the sweep can see (the residual of the method).

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
