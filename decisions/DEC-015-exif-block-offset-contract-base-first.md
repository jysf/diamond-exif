---
insight:
  id: DEC-015
  type: decision
  confidence: 0.7                    # preferred by this repo and by monorail-000; not yet agreed with irradiance/STAGE-017's design
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
status: proposed                     # binding once jysf accepts it with irradiance/STAGE-017's agreement (question exif-block-offset-contract-agreement)
deciders: [claude]

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
2. **The base.** The base is a byte offset from the start of the target TIFF (its header). It must be even (TIFF word alignment), and `base + length` must fit in a `u32` (classic TIFF). Otherwise the result is a typed refusal. Values inside the block are padded to even offsets relative to the block start, so an even base keeps every value aligned.
3. **Pointers outside the block.** The placing writer writes 0x8769 = `base + exif_position` and 0x8825 = `base + gps_position` in its IFD0, in the byte order it asked for. Those two LONG values are the only numbers it computes. Nothing inside the block is touched after this crate returns it.
4. **The self-check runs on the exact returned bytes at that base** (DEC-014 item 9), not on a re-encoding at some other base.
5. **Determinism.** The same source, options and base give the same bytes.

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

irradiance/STAGE-017's design accepts the two-phase shape, or names what it cannot do (for example, it cannot know the base before writing). SPEC-026's test placer uses the API exactly as irradiance would, at an arbitrary even base, and ExifTool reads the result. Revisit if irradiance cannot compute the base before writing, or if a second consumer needs placement-independent bytes.

## References

- Related specs: SPEC-025 (implements), SPEC-026 (test placer)
- Related decisions: DEC-014 (block contents), DEC-013 part 7 (self-check pattern)
- External: monorail-000/DEC-075 (items 5 and 6); monorail-000 `docs/upstream/diamond-exif-requests.md` (request 1, offset contract); irradiance/STAGE-017
- Questions: `exif-block-offset-contract-agreement`
