---
insight:
  id: DEC-012
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

created_at: 2026-11-09
supersedes: null
superseded_by: null
status: accepted
deciders: [jysf, claude]
affected_scope:
  - "src/jpeg/**"
  - "src/audit/**"
  - "src/policy.rs"
  - "src/report/**"
  - "tests/real_shapes*.rs"

tags:
  - audit
  - jpeg
  - mpf
  - segments
  - policy
  - writer
---

# DEC-012: The structures the audit names so a strip can be true — Multi-Picture images, comments, unrecognised segments, trailers, Photoshop blocks

## Decision

SPIKE-001 (real camera and phone files) found data a strip must deal with that the audit did not report. A preview that says a policy leaves "nothing" while a Multi-Picture secondary image, a proprietary APP10 segment or a trailer remains is false. This DEC extends what the JPEG layer locates and what the audit names, in the same style as DEC-006 (locate, never decode), DEC-008 (categories) and DEC-009 (presets). It amends all three; their text stands except where stated here.

**1. What the JPEG layer now locates** (regions, in addition to Xmp, ExtendedXmp, Iptc, Icc, Jumbf):

| Region | What it is | How it is found |
|---|---|---|
| `EmbeddedImage` | one Multi-Picture Format (MPF) image other than the primary | an APP2 segment starting `MPF\0` holds a TIFF-style header and an IFD whose tag `0xB002` (MPEntry) lists, 16 bytes per image, an attribute word, a size, and an offset **relative to the start of the MPF header** (the byte after `MPF\0`); entry 0 is the primary and is not a region; each other entry whose `offset` and `size` lie inside the input and whose bytes start `FF D8` is one region (offset, length). |
| `Comment` | a `COM` segment | marker `0xFE` |
| `OtherApp` | an APPn segment (n = 0 to 15) the library does not recognise | any `SegmentKind::App(n)` that is not on the benign list below and is not a parsed MPF segment |
| `Trailer` | bytes after the first EOI that no `EmbeddedImage` accounts for | `Jpeg::trailer()` minus the byte ranges of the located embedded images; one region for the remainder if it is not empty |

**Benign list (no region, no finding):** APP0 `JFIF\0`, and APP14 `Adobe\0` (a colour-transform marker with no free text). Every other APPn that is not Exif, XMP, extended XMP, ICC, Photoshop 3.0, JUMBF or MPF is `OtherApp`: this includes APP0 `JFXX` (it can carry a thumbnail), APP2 `FPXR` (FlashPix), APP10 `AROT` and any vendor segment. An MPF segment that cannot be parsed (bad header, no `0xB002`) is `OtherApp` and the parse failure is a typed JPEG-layer issue; a malformed entry (range outside the input, not `FF D8`) is a typed issue and no region.

**Photoshop blocks.** Every APP13 segment with the `Photoshop 3.0\0` header is an `Iptc` region, whether or not a `0x0404` resource parses inside it (a Photoshop block longer than 32 KB is split across APP13 segments, and each segment is a block). Its data is the segment payload after the header. This replaces DEC-006's rule that the region is the `0x0404` resource data; the earlier rule's unparsed-resource issues stay as issues.

**2. Bounds.** At most 16 `EmbeddedImage` regions (further entries are a typed issue and are not regions); the MPF IFD is read with the existing tolerant TIFF engine (its entry budget applies); no recursion into an embedded image's own MPF; nothing is copied.

**3. Embedded images are looked into once.** For each `EmbeddedImage` the audit runs the ordinary JPEG audit on the embedded bytes with embedded-image location switched off, and reports the **set of categories found inside** (`holds`, informational). It is not a disposition input (below). The inner items are not added to the outer inventory.

**4. Categories** (DEC-008 amendment): two are added, `EmbeddedImage` (**Medium**: an image stored inside the file that can carry its own metadata) and `OtherSegment` (**Medium**: data the library does not understand: an unrecognised APPn segment or trailing bytes). `Comment` is not new: a `COM` region is a `FreeText` finding (text typed by the author or the software). `Category::ALL` has 15 entries; the 16-bit set in `StripSpec` has one spare bit. Region mapping: `EmbeddedImage` to `EmbeddedImage`, `Comment` to `FreeText`, `OtherApp` and `Trailer` to `OtherSegment`.

**5. Presets and may-hold** (DEC-009 amendment). Both new categories are **block** categories (`is_block` is true) and are removed **only by `Everything`**; every other preset keeps them and the preview reports them `KeptUnread`:

| Category | `Gps` | `GpsAndDevice` | `AllPii` | `All` | `Everything` |
|---|---|---|---|---|---|
| `EmbeddedImage` | | | | | removed |
| `OtherSegment` | | | | | removed |

`may_hold(EmbeddedImage)` and `may_hold(OtherSegment)` are every category except themselves (an embedded image can hold anything the primary can; an unknown segment can hold anything). Consequences: under `Gps`, `GpsAndDevice`, `AllPii` and `All`, a Nikon file's Multi-Picture images, an iPhone file's `AROT` segment and a Leica trailer each show as `kept_unread`. The `Comment` rows ride on `FreeText`: removed by `AllPii`, `All`, `Everything`. DEC-009's rule that a block is `KeptUnread` when it may hold something the policy removes is unchanged. `holds` is shown to the reader but never changes a disposition, because a block inside the embedded image can itself be unread.

Defaults for the writer follow from this table, not a separate choice: **unknown segments, trailers and embedded images are kept by every preset except `Everything`, and the preview says so.**

**6. MakerNote previews are not located.** A preview JPEG inside a MakerNote (Nikon, Fuji, Leica) lives in a block the library does not parse (DEC-001 scope, STAGE-003 non-goal). It is covered by `may_hold(MakerNote)`, which gains `EmbeddedImage` (a MakerNote may hold an embedded image). No MakerNote is parsed to find it.

**7. Report changes** (DEC-010 amendment). `schema_version` becomes **2**: the finding object gains the key `embedded` (after `thumbnail`): `null` or `{"offset":n,"length":n,"starts_with_ffd8":bool,"holds":[category names in Category::ALL order]}`; the category enumeration, the `by_category` object (15 keys in `Category::ALL` order) and the `region` enumeration gain `embedded_image`, `other_segment` / `embedded_image`, `comment`, `other_app`, `trailer`. CSV keeps its 19 columns (no column for `holds`); text prints `holds`. Every other DEC-010 rule stands.

## Context

The strip must be provably true, and SPIKE-001 showed on real files that the audit's picture of "what is in this file" stopped at Exif, XMP, ICC, IPTC and JUMBF. Four of eleven JPEGs had Multi-Picture images (two Nikon files' extras carried their own Exif), two had proprietary APP10 segments, one had a trailer, one split Photoshop data across two APP13 segments, and several carried COM text. The writer stage cannot be checked against a preview that does not know these exist. Deciding the names, categories and preset cells here, before the writer, is the same discipline as DEC-008/009.

## Alternatives considered

- **Recurse fully into embedded images and list their items in the outer inventory.** Rejected: it multiplies inventory, breaks container independence checks, and the inner images can nest. A one-level `holds` set tells the reader what is inside without that.
- **Let `holds` decide the disposition** (Kept when the inner image holds nothing the policy targets). Rejected: the inner image's own MakerNote or XMP may be unread, so "holds nothing" is not knowable; `KeptUnread` stays honest.
- **Remove embedded images and unknown segments under `All`.** Rejected: an iPhone's gain map is rendering data and a vendor segment may be required by the vendor's software; `All` means "all Exif, XMP and IPTC" (design section 5). `Everything` is the explicit opt-in.
- **Call COM a new category.** Rejected: DEC-008's `FreeText` is exactly "text the author typed".
- **A segment-by-segment benign list wider than JFIF and Adobe.** Rejected: an unknown segment is unknown; adding an entry to the benign list is a DEC amendment with a measured example.
- **Locate MakerNote previews by parsing MakerNotes.** Rejected for this stage: vendor parsing is out of scope.

## Consequences

- **Positive:** the preview becomes true for real files (nothing the library can see is silently outside it); the writer stage has a complete list of what to remove or keep per preset; the report says what an embedded image holds.
- **Negative:** nearly every camera JPEG now has an `OtherSegment` or `EmbeddedImage` or `Comment` finding under non-`Everything` presets (loud by design); `schema_version` 2 changes every JSON consumer's `by_category` shape; the inner audit costs one extra linear pass over embedded images.
- **Neutral:** the benign list and the 16-image bound are constants; the 16-bit set has one bit left, so a further category needs a wider set (a DEC-009 mechanical change).

## Validation

Revisit when a real file shows a benign segment flagged as `OtherSegment` often enough to be noise (add to the benign list with a measured example), when a MakerNote parser makes preview location possible, or when the set of categories needs a seventeenth entry.

## References

- Related specs: SPEC-019 (implements), SPEC-020 to SPEC-022 (the writer consumes this table)
- Related decisions: DEC-006 (regions; amended: the Photoshop rule), DEC-008 (categories; amended), DEC-009 (presets and may-hold; amended), DEC-010 (report; schema_version 2)
- Evidence: SPIKE-001 (archived), questions `audit-embedded-images-and-unrecognised-segments`, `iptc-photoshop-resources-across-app13-segments`
- External: CIPA DC-007 (Multi-Picture Format), Photoshop file format specification (image resource blocks)

## Amendment (2026-11-09, from the SPEC-019 build)

Rulings on the build's four doubts (questions `dec-012-*`). Only item 1 and item 3 change what the text says.

1. **The 16-image bound counts the index, primary included.** At most 16 images are read from the MPF index (entry 0 is the primary), so at most **15** `EmbeddedImage` regions; entry 17 and later cost one typed issue and are not examined. The "at most 16 regions" in part 2 is read this way. A sixteenth secondary is not lost: its bytes stay in the `Trailer` region, so the strip still sees them.
2. **No cap on trailer pieces.** The remainder after embedded images has at most (regions + 1) pieces, so at most 16. A cap would drop uncovered bytes silently, which a strip must not do.
3. **The Photoshop rule moves two existing inventory results** (`fixtures/jpeg-iptc-without-0404`, seed `icc-iptc-jumbf`): they now list an `Iptc` region that ExifTool does not name. This is the intended effect of part 1 ("every APP13 `Photoshop 3.0` segment is a region"); the two are pinned as class R rows and one `KNOWN_DIVERGENCES` entry. The Iptc region data is the payload after the `Photoshop 3.0\0` header, so the golden region length for the full-house fixture is 18 (12-byte 8BIM header + 6), not 6.
4. **MPF parse edges.** Malformed (`OtherApp` plus a typed issue): bad TIFF header or version, IFD0 absent or unreadable, no `0xB002`, a truncated index, fewer than 16 bytes, and any second MPF segment (only the first is read). `NumberOfImages` (0xB001) is ignored; the count is the whole 16-byte entries of the `0xB002` payload and a trailing partial entry is ignored.
