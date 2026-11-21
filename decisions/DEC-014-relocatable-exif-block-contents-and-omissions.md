---
insight:
  id: DEC-014
  type: decision
  confidence: 0.75                   # the contents follow the consumer's request closely; the byte-order and offset-carrying rules are new and get tested in SPEC-025 design
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

created_at: 2026-11-20
supersedes: null
superseded_by: null
status: proposed                     # drafted by claude at STAGE-007 framing, 2026-11-20; awaiting jysf's review
deciders: [claude]

affected_scope: []                   # no code yet; SPEC-025 design sets the module path

tags:
  - writer
  - exif-block
  - relocation
  - makernote
  - fail-closed
---

# DEC-014: The relocatable Exif block: the Exif, GPS and Interop IFDs rebuilt in the caller's byte order, MakerNote and entries that cannot be moved safely left out and reported, read back before it is returned

## Decision

The library gains a second writer operation beside `strip`: building a **relocatable Exif block** from a source. It is a pure function of the source bytes and an options value. It returns the block and a list of what it left out, or a typed refusal and no bytes. The block **builds new IFDs** and is not a DEC-013 edit plan. DEC-013 part 1 ("no insert, grow or move") still governs `strip` and is unchanged.

1. **What is in the block.** The source's Exif IFD (reached by 0x8769 from IFD0), the Interop IFD (reached by 0xA005 from the Exif IFD), and the GPS IFD (reached by 0x8825 from IFD0), each with every out-of-line value its kept entries reference. Nothing else: no IFD0 entry, no SubIFD, no IFD1, no image data. The placing writer writes the 0x8769 and 0x8825 pointers in its own IFD0, at the positions the result reports (DEC-015). Inside the block, the Exif IFD's 0xA005 pointer is rewritten to the Interop IFD's new position. A source with none of the three IFDs gives an empty result, not an error.
2. **Byte order.** The block is written in the byte order the caller asks for, whatever the source's was. Every entry whose type the walk decodes (BYTE, ASCII, SHORT, LONG, RATIONAL, SBYTE, UNDEFINED, SSHORT, SLONG, SRATIONAL, FLOAT, DOUBLE) is converted element by element. An entry with a type the walk does not know is left out when the orders differ (its elements cannot be swapped) and copied byte for byte when they match.
3. **UNDEFINED values with byte-order-dependent contents.** Some UNDEFINED entries hold multi-byte fields in the file's byte order. Known candidates, to be confirmed against the Exif specification and ExifTool in SPEC-025 design: 0x8828 OECF, 0xA20C SpatialFrequencyResponse, 0xA302 CFAPattern, 0xA40B DeviceSettingDescription. When the orders differ, each is either converted by a rule SPEC-025 writes and proves against ExifTool, or left out and reported. It is never copied unchanged into the other order. Question `exif-block-byte-order-sensitive-undefined` tracks which.
4. **MakerNote is always left out in this decision** (0x927C), together with 0xEA1D OffsetSchema, which only describes a MakerNote's offsets. A MakerNote with absolute offsets is corrupt once moved, and this crate does not parse MakerNotes (DEC-012, DEC-013 C, question `makernote-internal-offsets`). Moving one when it is provably safe (no internal offsets, or self-relative ones) is a later amendment that needs M1d's MakerNote read rules.
5. **Entries that carry offsets the builder does not rewrite are left out.** Any pointer or blob-carrier tag in the three IFDs other than 0xA005 (for example 0x0190, 0x8290, 0xFE00, 0x888A and the blob carriers of question `exiftool-subdirectory-pointer-tags`) would point at the wrong bytes after the move, so it is left out and reported. So is any entry of type 13 (IFD), which is an offset by type, except 0xA005 itself. Out-of-line values are copied as values, never as references into the source.
6. **Omissions are data.** Every entry left out is reported with its IFD, its tag and a reason (`MakerNote`, `ByteOrder`, `OffsetCarrying`, `UnknownType`, and `Policy` when SPEC-027 lands). A MakerNote that was dropped is therefore visible, as request 2 asks.
7. **Fail closed on the source.** The builder refuses, with no bytes, when the walk of IFD0, the Exif, GPS or Interop IFD has any issue or was halted by a cap, or when one IFD holds a duplicate tag. The caller can still export with no Exif block. It never returns a partial block that silently lacks entries it could not read.
8. **Deterministic and bounded.** The same source and options give the same bytes: entries are written in ascending tag order, and the layout is a fixed function of the kept entries. The size is computed before anything is allocated and checked against a caller budget (an options field with a default) and the existing entry budget (`tiff::entry_budget`). A block over budget is a typed refusal.
9. **The library reads its own output back.** Before returning, the builder parses the exact bytes it returns, at the base it was given (DEC-015), and checks that the Exif, GPS and Interop entries equal the source's: same tag, same type, same count, same value after byte-order conversion, minus exactly the reported omissions. A mismatch is a typed `VerifyFailed`, never bytes. This is DEC-013 part 7's pattern applied to building.

## Context

monorail-000 asks for this block (its `docs/upstream/diamond-exif-requests.md`, requests 1 and 2; its decision monorail-000/DEC-075): its Linear DNG export copies the source's EXIF, the irradiance DNG writer (irradiance/STAGE-017) writes IFD0 and image data, and EXIF must have one owner. The requests set the content (Exif, GPS, Interop with all out-of-line values; no IFD0, no SubIFDs), the byte order rule, the MakerNote posture (drop unless it moves safely; always dropping is fine for v1; report the drop), and the constraints (I/O-free, panic-free, no new dependencies, deterministic, budgeted). This crate could only remove bytes in place until now. DEC-013 part 1 deferred any insertion to "a later decision" with its own growth rule, and this is that decision for one bounded case.

Items 3 and 5 are not in the request. They follow from it: a relocated value that embeds an offset or a byte order is corrupt in the same way a relocated MakerNote is.

## Alternatives Considered

- **Copy the source's IFD bytes and patch offsets in place.** Rejected: it keeps the source's byte order (request: the caller's), keeps unknown internal offsets alive, and makes "what moved" depend on the source's layout. Rebuilding from decoded entries is what makes the output deterministic and checkable.
- **Keep the MakerNote when it looks self-contained.** Rejected for now: "looks" is a vendor parser this crate does not have. The request accepts always dropping it.
- **Return a best-effort block from a dirty walk, with issues attached.** Rejected for the same reason as DEC-013 part 2: a caller that ignores the issues ships EXIF the library knows is incomplete.
- **Include IFD0's non-structural tags (Make, Model, Artist, Copyright, DateTime, Software, the DNG identity tags).** Not decided here. The request leaves IFD0 out. Carrying them would need IFD0 entries in a block, or a second list for the placing writer to merge. Question `exif-block-ifd0-identity-tags` returns it to the consumer.

## Consequences

- **Positive:** the crate gets an IFD encoder with a self-check, which M1d `set` operations and question `strip-keeps-orientation` can reuse. A consumer gets EXIF it can trust without interpreting it.
- **Negative:** the MakerNote is lost on every block, and a cross-order block may lose a few UNDEFINED entries until item 3 has converters. A dirty source gives no block at all.
- **Neutral:** the result types are `#[non_exhaustive]`, so a later movable-MakerNote rule or a JPEG source does not break callers.

## Validation

SPEC-026's placed-block parity against ExifTool, in all four source and target byte order pairs, is the evidence. Revisit when a real source shows an omission a consumer needs (most likely the MakerNote or a cross-order CFAPattern), when M1d gives MakerNote read rules, or when the consumer answers `exif-block-ifd0-identity-tags`.

## References

- Related specs: SPEC-025, SPEC-026, SPEC-027 (STAGE-007)
- Related decisions: DEC-001 (tolerant walk), DEC-005 (pointer tags), DEC-008 (categories), DEC-009 (presets), DEC-012, DEC-013 (strip contract, part 7 self-check), DEC-015 (offset contract)
- External: monorail-000/DEC-075; monorail-000 `docs/upstream/diamond-exif-requests.md`; irradiance/STAGE-017
- Questions: `exif-block-ifd0-identity-tags`, `exif-block-byte-order-sensitive-undefined`, `makernote-internal-offsets`, `exiftool-subdirectory-pointer-tags`
