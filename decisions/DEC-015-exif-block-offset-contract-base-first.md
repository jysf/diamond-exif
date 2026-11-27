---
insight:
  id: DEC-015
  type: decision
  confidence: 0.85                   # agreed by irradiance/DEC-072 and monorail-000/DEC-076; nothing built on either side yet
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
status: accepted                     # drafted by claude 2026-11-20 (#37); accepted by jysf 2026-11-21 after irradiance/DEC-072 agreed
deciders: [jysf, claude]

affected_scope: []

tags:
  - writer
  - exif-block
  - api
  - cross-repo
---

# DEC-015: The Exif block's offset contract is base-first: a size query, then a write at a caller-given base with final offsets, so the placing writer never edits the block's bytes

## Decision

The placing writer asks for the block's size, chooses where the block will start in its file (the **base**), and asks this crate to write the block for that base. Every offset inside the block is then final, and the placing writer copies the bytes unchanged.

1. **Two phases.** Building the block from a source and options gives a value that reports the block's **length**, which does not depend on the base, and the **positions relative to the block start** of the Exif IFD and the GPS IFD. Writing it takes a base and either fills a caller's `&mut [u8]` of exactly that length or returns a `Vec<u8>`. This is the caller-owned output shape monorail-000/DEC-075 item 6 asks of irradiance.
2. **The base.** The base is a byte offset from the start of the target TIFF (its header). It must be even (TIFF word alignment), and `base + length` must fit in a `u32` (classic TIFF). Otherwise the result is a typed refusal. Values inside the block are padded to even offsets relative to the block start, so an even base keeps every value aligned. The block's length is itself even, so whatever follows it stays word-aligned.
3. **Pointers outside the block.** The placing writer writes 0x8769 = `base + exif_position` and 0x8825 = `base + gps_position` in its IFD0, in the byte order it asked for, writing each pointer only when the result reports that IFD's position (DEC-014 item 1: a block may hold the Exif IFD, the GPS IFD or both). Those LONG values are the only numbers it computes. Nothing inside the block is touched after this crate returns it.
4. **The self-check runs on the exact returned bytes at that base** (DEC-014 item 9), not on a re-encoding at some other base.
5. **Determinism.** The same source, options and base give the same bytes.

## Agreement

- **irradiance: agreed in its record**, irradiance/DEC-072 (drafted in jysf/irradiance#284, amended in jysf/irradiance#291; still `proposed` there, awaiting jysf's acceptance). Its writer fixes the base at file offset 8, directly after the TIFF header, and writes little-endian. It exposes the base as a public constant and a function, and mono-core passes that value here. It copies the block verbatim, writes 0x8769 and 0x8825 as 8 + the reported positions, and checks only bounds and alignment. Changing the base needs an irradiance DEC and a breaking release. A fixed base is a special case of item 2, so nothing in this decision changes.
- **monorail-000: agreed**, monorail-000/DEC-076 item 1 (2026-11-20).
- **The seam, decided by the consumer:** irradiance/DEC-072 first required an Exif IFD whenever a block is given, while DEC-014 item 1 allows a GPS-only block. jysf chose the GPS-only block in monorail-000/DEC-077 item 1, and irradiance/DEC-072's amendment (jysf/irradiance#291) makes both offsets optional, at least one required, and a block with neither a `CallerError`. Item 3 states it on this side: a pointer is written only for an IFD the result reports.

## Context

Request 1 from monorail-000 leaves the offset contract to be agreed with irradiance/STAGE-017's design: (a) block-relative offsets plus a list of where they sit, which the placing writer rebases, or (b) base-first, where the caller gives the base and this crate writes final offsets. monorail-000 leans to (b) "if irradiance can report the base before it writes". monorail-000/DEC-075 item 5 phrases irradiance's job as "rebases the block's offsets", written before this choice, and says irradiance "does not interpret the block's contents".

The length does not depend on the base: every offset field in a classic TIFF is a fixed-width `u32`, and alignment is relative to an even base. So the placing writer can lay out its file (header, IFD0, image data, the block) from the length alone and know the base before any block byte exists. That removes the only cost (b) was thought to have.

## Alternatives Considered

- **Option A: block-relative offsets plus a fixup list.**
  - What it is: the block's offsets count from its first byte, and the result lists the positions of every offset field. The placing writer adds its base to each, in the target byte order.
  - Why rejected: a second party then rewrites bytes inside the block **after** this crate's self-check, so the bytes ExifTool reads are not the bytes this crate verified. The placing writer must handle byte order and `u32` overflow on fields it does not understand. The list's format becomes cross-repo API. That is more contract and more tests on both sides, and an export bug would land in a repo that does not own EXIF.
- **Option C: offer both.** Rejected: it doubles the API and the tests, the same reasoning monorail-000/DEC-075 O4 used against a writer that both copies and takes caller bytes. The internal list of offset positions exists anyway (it is how the block is laid out), so exposing it later is a non-breaking addition if a consumer ever needs placement-independent bytes.
- **Option B (chosen): base-first, final offsets.**
  - Why selected: the bytes this crate checks are the bytes that ship, the placing writer computes two pointer values and nothing else, and the output is fully determined by (source, options, base).

## Consequences

- **Positive:** EXIF has one owner in the strict sense: no other crate edits its bytes. The placing writer's EXIF code is a size query, a copy and two pointer tags.
- **Negative:** the placing writer must fix its layout before asking for the bytes. A writer that streams and only learns positions as it goes must reserve the length first, which it can, because the length is known in advance. A block cannot be cached across placements without rewriting.
- **Neutral:** BigTIFF bases (over 4 GiB) are refused, as BigTIFF is everywhere else in the writer.

## Validation

irradiance/STAGE-017's design accepted the shape (irradiance/DEC-072), with the base fixed at 8. irradiance/SPEC-081 places a block through its writer and checks it with ExifTool, and its red-proof shifts the base by 2. SPEC-026's test placer uses the API exactly as irradiance would, at an arbitrary even base, and ExifTool reads the result. Revisit if irradiance cannot compute the base before writing, or if a second consumer needs placement-independent bytes.

## Clarification (2026-11-27, SPEC-025 design): how the self-check reads the block at its base

Item 4 requires the check to run on the exact returned bytes at the caller's base. The crate's walk reads a whole TIFF from offset 0, so the self-check builds a scratch image: an 8-byte header, a gap of zeros up to `base`, the block, then an IFD0 holding only 0x8769 and 0x8825 as item 3 would write them. It walks that image with `Container::parse` and compares. Changing the walk to read at an offset was rejected: the walk is the crate's core and the one reader every other check trusts.

So the check needs `base + length + 34` bytes. That total counts against the same budget as the block (`max_len`, default 16 MiB): a base so large that the image would exceed it is a typed refusal before anything is allocated, and a caller with a large base raises the budget. For irradiance's base of 8 the cost is the block's own size. This adds no condition to item 2: the base is still any even value with `base + length` in a `u32`, within the budget.

## References

- Related specs: SPEC-025 (implements), SPEC-026 (test placer)
- Related decisions: DEC-014 (block contents), DEC-013 part 7 (self-check pattern)
- External: monorail-000/DEC-075 (items 5 and 6); monorail-000/DEC-076 (item 1); monorail-000 `docs/upstream/diamond-exif-requests.md` (request 1, offset contract); irradiance/STAGE-017; irradiance/DEC-072 (jysf/irradiance#284)
- Questions: `exif-block-offset-contract-agreement`
