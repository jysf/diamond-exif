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
status: accepted                     # drafted by claude at STAGE-007 framing, 2026-11-20; accepted by jysf 2026-11-27 with the SPEC-025 design amendment; the SPEC-041 design amendment of 2026-11-28 accepted by jysf 2026-11-28 as written
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

1. **What is in the block.** The source's Exif IFD (reached by 0x8769 from IFD0), the Interop IFD (reached by 0xA005 from the Exif IFD), and the GPS IFD (reached by 0x8825 from IFD0), each with every out-of-line value its kept entries reference. Nothing else: no IFD0 entry, no SubIFD, no IFD1, no image data. The placing writer writes the 0x8769 and 0x8825 pointers in its own IFD0, at the positions the result reports (DEC-015). Inside the block, the Exif IFD's 0xA005 pointer is rewritten to the Interop IFD's new position. A source with none of the three IFDs gives an empty result, not an error. **A source with a GPS IFD and no Exif IFD gives a GPS-only block** (binding: irradiance/DEC-072 was accepted with the optional Exif offset on 2026-11-21; see the amendment), and the result reports only the GPS IFD's position. The builder never writes an Exif IFD the source did not have (an empty one would be a tag the source never carried). Each position is optional in the result, and a non-empty result has at least one. monorail-000/DEC-077 item 1 chose this. irradiance/DEC-072's amendment matches it (jysf/irradiance#291): both offsets are optional, at least one comes with a block, and a block with neither is a `CallerError` there. This builder never returns a non-empty block with neither, and the consumer passes no block for an empty result.
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

## Amendment (2026-11-21, from monorail-000/DEC-077)

Item 1 now states the GPS-only case. The other options were the builder writing an empty Exif IFD, or the consumer dropping the block and losing the location. jysf chose the GPS-only block in monorail-000/DEC-077 item 1, with irradiance making its Exif offset optional. irradiance's amendment to irradiance/DEC-072 (jysf/irradiance#291) takes the same option. That DEC is still `proposed`, so the GPS-only case is pending until irradiance confirms it, in irradiance/DEC-072 as amended or a new DEC. SPEC-025's design waits for that confirmation. If irradiance declines, jysf is told, monorail-000 chooses between an empty Exif IFD and dropping the block, and this item is amended to match. Question `exif-block-ifd0-identity-tags` is answered: the consumer passes IFD0's descriptive tags to irradiance as typed fields (irradiance/DEC-071 §3a, monorail-000/DEC-076 and DEC-077), and the block stays Exif, GPS and Interop only.

## Amendment (2026-11-27, SPEC-025 design): the build is staged by byte order; the measured omission list; which walk issues refuse

Measured against ExifTool 13.55 at SPEC-025 design (scripts in the spec's design notes; the build turns them into a gated live test). Nothing here changes what the block contains; it makes items 2, 5 and 7 exact.

1. **Byte order is staged (item 2).** SPEC-025 builds the block in the **source's** byte order only, and the result reports that order. SPEC-041 adds a target byte order as a new options field (the options type is `#[non_exhaustive]` with private fields, so that is not a breaking change), with item 3's rule. Until SPEC-041 ships, a caller that needs little-endian checks the result's byte order and passes no block from a big-endian source.
2. **An entry of a type the walk does not size is always left out (`UnknownType`), in either order (item 2).** "Copied byte for byte when the orders match" cannot be done: without the type's size the value's length and place are unknown. ExifTool 13.55 skips such an entry too: types 14 to 18 and 255 in the Exif IFD each print `Bad format (N) for ExifIFD entry 0` and no value, and type 0 prints nothing. *(Refined at verify round 1, 2026-11-27, ExifTool 13.55 without `-m`: ExifTool drops the whole IFD when entry 0 has a bad format (type 14 in entry 0 prints only `Bad format (14) for ExifIFD entry 0`); type 0 counts as a bad format here too, and in entry 0 it drops the IFD silently, with no warning (verify round 2), and otherwise skips only the bad entry: an unknown type in entry 1 or later prints the warning for that entry and every other entry still prints. So a block from a source whose first entry has an unknown type shows entries ExifTool never printed for the source. That is accepted, because the entries are the source's own. `tests/exif_block_live.rs` pins that difference for `unknown-types` and holds `unknown-type-not-first` to exact parity.)*
3. **The offset-carrying list is measured, not guessed (item 5).** Two sweeps over every tag id in ExifTool's `Exif::Main` table (573 ids) placed in the Exif IFD and in the Interop IFD, and every id in `GPS::Main` (32) placed in the GPS IFD, each as a LONG pointing at a mini-IFD 4 KiB away:
   - *Writer sweep:* ExifTool rewrote each file (`-Artist=sweep`). It refused 22 with `No size tag for ...`, the offset tags it relocates only with their length: 0x0111, 0x0120, 0x0144, 0x0201, 0x0207, 0x0208, 0x0209, 0x8781, 0xA010, 0xBCC0, 0xBCC2 (each in the Exif and the Interop IFD). It rewrote the value of 0x014A SubIFDs. Every other value came back unchanged. *(Corrected at build, 2026-11-27: the design probe passed `-m`, which DEC-023 rules out for ExifTool writes. Without it ExifTool also refuses four sources with a `[minor]` error about the swept value, not an offset: 0xC612 `DNG Version 4096 not yet tested` and 0x927C `Maker notes could not be parsed`, each in the Exif and the Interop IFD. The offset set is unchanged; `tests/exif_block_sweep_live.rs` runs without `-m` and pins all 26 refusals.)*
   - *Reader sweep:* the pointed-at bytes were zeroed, as a moved value would leave them. Only 0x014A changed what ExifTool printed (`SubIFD:Software` disappeared), in the Exif and the Interop IFD. Nothing in the GPS IFD changed.

   So the block leaves out, as `OffsetCarrying`, in any of the three IFDs: those 11 offset tags; 0x014A; the pointer tags 0x8769 and 0x8825, and 0xA005 anywhere but the Exif IFD; any entry of type 13 (IFD) except the Exif IFD's 0xA005; and, as item 5 already says, 0x0190, 0x888A, 0x8290, 0xFE00 and the blob carriers 0xC51B, 0x8568, 0xCD41, 0xC6F5. The measurement shows ExifTool does not follow those last eight from the Exif, GPS or Interop IFD, so leaving them out is conservative, not required; it is kept because the strip planner treats them the same way and none occurs in a real camera's Exif IFD. The length partners (0x0117, 0x0121, 0x0145, 0x0202, 0xA011, 0xBCC1, 0xBCC3) are plain numbers and are kept. 0x927C MakerNote and 0xEA1D OffsetSchema are left out as `MakerNote` (item 4).
4. **Which walk issues refuse (item 7, narrowed).** The builder refuses when: the walk was halted by a budget (`TooManyIfds`, `TooManyEntries`), because a halt can hide an IFD; an issue was raised following 0x8769 or 0x8825 from IFD0, or 0xA005 from the chosen Exif IFD; one of the three IFDs, or IFD0's 0x8769 or 0x8825, appears twice (the walk takes the first of a duplicate pointer, so the builder checks); or a kept entry's value cannot be read (`Container::payload` fails for a known type). An issue anywhere else (IFD1, a SubIFD chain, a SubIFD pointer inside the Exif IFD, whose entry is left out anyway) does not refuse: a DNG with a damaged preview chain still exports its Exif. Item 7's "any issue in the walk of IFD0" read wider than this, and would have refused such a DNG for an IFD the block never touches.
5. **Which Exif and GPS IFDs.** The block takes the Exif IFD and the GPS IFD whose parent is IFD0, and the Interop IFD whose parent is that Exif IFD. The walk visits SubIFDs before IFD0's Exif pointer, so `Container::ifd_of_kind(IfdKind::Exif)` can return a SubIFD's Exif IFD: the builder selects by parent. An Exif or GPS IFD reached from IFD1 or a SubIFD is not part of the block and is not reported as an omission: it was never a candidate.
6. **Kept entries keep their type, count and value bytes.** In the source's order, the payload (`count × size` bytes) is copied as is. An inline value's unused bytes are written as zero. A count of 0 is kept (ExifTool prints such an entry with an empty value). 0xA005 is written as LONG, count 1, holding the Interop IFD's final offset, whatever its type was (LONG or IFD).
7. **Layout (item 8).** The tables come first, Exif, Interop, GPS (each present one), each ending in a next-word of 0; then the out-of-line values in that IFD order and ascending tag order, each starting at an even block-relative offset, padded with 0x00. ExifTool ignores an Exif IFD's next-word (measured: a source whose Exif IFD chains to another IFD prints none of it), so writing 0 loses nothing. Placed at base 8 and at base 1000 in a minimal TIFF with IFD0 after the block, this layout reads in ExifTool exactly as the conventional source does, in both byte orders, and `-validate` adds no warning.

## Amendment (2026-11-28, SPEC-041 design): conversion into the other byte order; the measured byte-order omission table; two conversion rules

**Accepted** by jysf on 2026-11-28 as written, before the build started (proposed at SPEC-041 design by claude). Measured against ExifTool 13.55 (scripts summarised in SPEC-041's design measurements; the build turns them into a gated live test). It completes items 2 and 3 and closes question `exif-block-byte-order-sensitive-undefined`.

1. **A target byte order, both directions (items 2 and 3; the 2026-11-27 amendment item 1).** `BlockOptions` gains a target byte order. Unset, the block is in the source's order, byte for byte what SPEC-025 builds. Set, the block is in that order whatever the source's, and the result reports both orders. Little-endian to big-endian is offered as well as the consumer's big-endian to little-endian: it is the same code, and DEC-014's validation already names all four pairs. Real sources cross: 3 of 26 local raw files are big-endian (an iPhone 12 Pro DNG, a second DNG, a Nikon NEF), and the NEF's CFAPattern is big-endian.
2. **Conversion is by declared type (item 2).** SHORT and SSHORT swap 2 bytes per element; LONG, SLONG and FLOAT 4; RATIONAL and SRATIONAL each 4-byte half; DOUBLE 8. BYTE, ASCII, SBYTE and UNDEFINED are copied. A converted value has the same length, so the layout (2026-11-27 amendment item 7) is the same function of the kept entries in either order. 0xA005 and the next-words are written in the target order.
3. **Entries left out as `ByteOrder` when the orders differ (items 3 and 6).** ExifTool reads some tags with a format of its own, whatever type the file declares, so converting by the declared type changes what it reads. The cross-order sweep (every id of `Exif::Main` in the Exif and in the Interop IFD, every id of `GPS::Main` in the GPS IFD, each under all 12 declared types and up to 7 payload shapes; 42,336 big-endian sources each against its converted little-endian copy) found exactly these, the same in the Exif and the Interop IFD. "Multi-byte" means types 3, 4, 5, 8, 9, 10, 11, 12; "one-byte" means 1, 2, 6, 7.
   - *Exif and Interop IFDs, any multi-byte type* (ExifTool reads them as bytes or text): 0x0303, 0x4747, 0x828E, 0x8298, 0x9009, 0x9101, 0x9286, 0x9C9B, 0x9C9C, 0x9C9D, 0x9C9E, 0x9C9F, 0xBC01, 0xC615, 0xC65D, 0xC68B, 0xC68C, 0xC68F, 0xC691, 0xC6D2, 0xC6D3, 0xC6F3, 0xC6F4, 0xC6F6, 0xC6F8, 0xC6FE, 0xC716, 0xC717, 0xC718, 0xC719, 0xC71C, 0xC71D, 0xC7A7, 0xC7AA, 0xCD48 (35 tags).
   - *Exif and Interop IFDs, other masks:* 0x87AF type 12; 0x9201 any one-byte type; 0x9204 types 1, 2, 3, 6, 7, 8, 12; 0xA101 types 4, 5, 9, 10, 11, 12; 0xA302 every type but 7 (item 4 converts type 7); 0xA462 every type.
   - *Exif and Interop IFDs, every type, from the Exif specification:* 0x8828 OECF, 0xA20C SpatialFrequencyResponse and 0xA40B DeviceSettingDescription begin with two SHORTs in the file's order, and ExifTool prints them as undecoded bytes, so no conversion can be shown against it. They are left out (item 3's second choice), never copied.
   - *GPS IFD:* 0x001D GPSDateStamp types 4, 5, 9, 10, 11, 12.

   In its usual type every one of these tags converts correctly, except 0xA462 (UNDEFINED) and the three from the specification: the table bites on files that declare an unusual type. An entry is classified `UnknownType`, then `MakerNote`, then `OffsetCarrying`, then `ByteOrder`. In the source's own order nothing is left out as `ByteOrder`.
4. **CFAPattern (0xA302, UNDEFINED, Exif or Interop IFD).** When the orders differ, the first two 16-bit words are written in the target order and the rest is copied; a value under 4 bytes is copied. ExifTool reads the result as it reads the source on every shape measured, including a header in the wrong order for its file, a header both orders read as valid (1 × 256), a header that does not match the length, and 0 × 0. A plain copy reads differently on two of those.
5. **UNICODE text (0x9286 UserComment in the Exif or Interop IFD as types 1, 2, 6 or 7; 0x001B GPSProcessingMethod and 0x001C GPSAreaInformation in the GPS IFD as type 7).** When the orders differ and the value starts with the 8-byte code `UNICODE` followed by 0x00 or 0x20 (the two codes ExifTool accepts; `Unicode\0`, `UNICODEX`, ` UNICODE`, `UNICODE\t` and `UNICOD\0\0` are invalid to it), every 16-bit unit after the code is swapped and an odd last byte is copied. Any other code (ASCII, JIS, eight zero bytes, an invalid code) is copied. ExifTool decodes UNICODE text in the file's order: copied unchanged, CJK text reads as other characters (Latin text survives only because ExifTool guesses its order). A byte-order mark is a unit like any other and is swapped with the text; ExifTool honours it either way.
6. **The self-check compares values, not only bytes (item 9).** In the other order the bytes differ from the source's by design, so the readback compares each kept entry's value decoded from the block in the block's order with the source's value decoded in the source's order, using a decoder that does not call the converter. The same-order check is unchanged.

## References

- Related specs: SPEC-025, SPEC-041, SPEC-026, SPEC-027 (STAGE-007)
- Related decisions: DEC-001 (tolerant walk), DEC-005 (pointer tags), DEC-008 (categories), DEC-009 (presets), DEC-012, DEC-013 (strip contract, part 7 self-check), DEC-015 (offset contract)
- External: monorail-000/DEC-075; monorail-000/DEC-077 (items 1, 9, 10); monorail-000 `docs/upstream/diamond-exif-requests.md`; irradiance/STAGE-017
- Questions: `exif-block-ifd0-identity-tags`, `exif-block-byte-order-sensitive-undefined`, `makernote-internal-offsets`, `exiftool-subdirectory-pointer-tags`
