---
insight:
  id: DEC-013
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

created_at: 2026-11-10
supersedes: null
superseded_by: null
status: accepted
deciders: [jysf, claude]
affected_scope:
  - "src/write/**"
  - "src/jpeg/**"
  - "tests/strip*.rs"
  - "fuzz/fuzz_targets/rewrite_jpeg.rs"

tags:
  - writer
  - strip
  - jpeg
  - mpf
  - fail-closed
  - invariants
---

# DEC-013: The writer — a fail-closed plan of removals and same-length patches, checked against the audit before any byte is returned

## Decision

`strip` is a **pure function from input bytes and a policy to new bytes or a typed refusal**. It never returns a file that still contains a finding whose category the policy removes, and it never returns a file in which anything but the policy's targets changed. The foundation has seven parts; SPEC-020 implements them for JPEG, SPEC-021 and SPEC-022 reuse them.

**1. An edit plan has two operations only.** `Remove(range)` deletes bytes; `Patch(offset, bytes)` overwrites bytes **in place at the same length**. There is no insert, no grow, no move. The output is the input with the removal ranges deleted and the patches applied; its length is the input length minus the removed bytes, so `output.len() <= input.len()` always. A plan is valid only if its ranges lie inside the input, are sorted and disjoint after merging, and no patch lies inside a removal. Applying a plan is pure, deterministic and uses checked arithmetic. (Net-new insertion, such as writing a minimal Exif block that keeps Orientation, is a later decision: it needs a growth rule and is not a strip.)

**2. Fail closed.** If the library cannot do exactly what the policy says, `strip` returns `Err` and **no bytes**. It never returns a best-effort file with a list of what was left. The refusals are a closed, typed set: the container is not supported by this writer; the structure is not trustworthy enough to cut (a walk halted by a cap, a segment whose length is wrong); the policy needs entry-level editing the writer does not have yet (part 4); a removal would cut bytes that must be kept (part 6); a Multi-Picture index would be left wrong (part 5); the self-check failed (part 7). A caller that ignores the result cannot leak by accident. What the library *chooses not to remove* by policy design (a block that `KeptUnread` says it did not open) is not a refusal: the preview already says so.

**3. The unit of removal is the segment (JPEG).** A JPEG region is removed by deleting its whole segment (marker, length and payload), an embedded image or a trailer by deleting its byte range. The category of each region is the audit's own (DEC-012 section 4); the writer reads the same regions the audit reports, so there is one definition of "what is in the file". Segments that carry no category stay: `JFIF\0`, `Adobe\0`, quantisation and Huffman tables, frame headers, restart intervals, and the scan from `SOS` through `EOI`. Fill bytes and any bytes between segments are never removed.

**4. The Exif segment is all or nothing in this stage.** Every Exif APP1 segment holds, besides uncategorised structural tags, entries of nine categories: `Gps`, `DeviceId`, `Authorship`, `FreeText`, `Timestamps`, `CameraModel`, `Thumbnail`, `MakerNote`, `Software` (call the set EXIF9). Removing a segment whole is the only edit this stage can make there, so:

- If the policy contains **all of EXIF9** (`All`, `Everything`, and any custom set containing them), **every** Exif segment is removed, including the tags that have no category (Orientation, exposure, colour space, pixel dimensions). This is what `exiftool -all=` does for the EXIF group. **Consequence, accepted and documented:** the output no longer carries an Orientation tag, so a viewer that relied on it will show the stored pixel order. A strip that keeps Orientation edits inside the segment (SPEC-021) or writes a new one (later); this stage does not.
- If the policy contains **none of EXIF9**, Exif segments are untouched.
- If it contains **some but not all** of EXIF9 (`Gps`, `GpsAndDevice`, `AllPii`, most custom sets) and the file has an Exif block that either has a finding in one of the policy's EXIF9 categories or could not be read, `strip` returns `NeedsEntryEdit` naming those categories. A file whose Exif blocks are readable and have no finding in the policy's EXIF9 categories is returned unchanged for those categories (there is nothing to remove, and the audit confirms it). `Comment` (COM) findings are `FreeText` too; a policy with `FreeText` and not all of EXIF9 still removes COM segments but also needs the entry edit if an Exif block holds `FreeText`.

**5. Multi-Picture is a unit.** An MPF index segment and the images it lists are removed together or kept together:

- If `EmbeddedImage` is in the policy, every located embedded image **and** the MPF index segment are removed. (An index without images, or images without an index, is a corrupt file.)
- If it is not, the images and the index stay, and the index is **patched** so it still points where it did. Offsets in the index are relative to the MPF header start; sizes are sizes. Let `removed(a, b)` be the number of removed bytes in the input range `[a, b)`. Each kept secondary image's offset field becomes `offset - removed(mpf_header_start, image_start)`; the primary's (entry 0) size field becomes `size - removed(0, primary_end)` where `primary_end` is one past the primary's `EOI`; sizes of secondaries are unchanged because an embedded image is never cut (part 6). Patches are same-length overwrites of the existing `u32` fields in the file's own byte order. Entries that were not located as regions (malformed, outside the input) are left as they are.
- If the file has an `MPF\0` segment that the audit did not index (it is an `OtherApp` region plus an issue, or a second MPF segment) **and** the plan removes bytes that lie between that segment and the end of the file, `strip` returns `MpfNotIndexed`: the offsets cannot be kept right, so nothing is cut. If nothing is removed after it, nothing needs fixing.

**6. No byte is both removed and kept.** The plan's removal ranges must not intersect the scan region, a kept segment, a kept embedded image or a kept trailer piece. Overlaps between *removed* ranges are merged. An embedded image can legitimately overlap a segment of the primary in a hostile file (DEC-012: an `FF D8` inside a `COM` payload); removing the COM would cut a kept image, so the answer is `Overlap`, with the offset.

**7. The writer checks its own output against the audit, always.** Before returning, `strip` audits the output and returns `VerifyFailed` (never the bytes) unless all hold: (a) the output's audit has no finding in a category the policy contains; (b) for every category the policy does not contain, the multiset of findings by (category, region or tag location kind) equals the input's (kept `Trailer` regions are compared by their total byte count, because removing the embedded images between two trailer pieces joins them into one region); (c) the output's scan region (`SOS` through `EOI`) equals the input's byte for byte; (d) the output is itself a walkable JPEG with no new issue kinds; (e) `strip(output, policy)` finds nothing to do (idempotence, checked as "the plan is empty"). Costing one more linear pass, this makes the audit the oracle in the library, not only in tests; an implementation bug turns into a typed error that tests and fuzz see, not into a leak.

**8. Pixel integrity (I1) is defined as follows:** the bytes of the scan region and of every kept segment are unchanged and in the same order; the only bytes that differ from the input are the removed ranges (absent) and the listed patches (MPF fields). A test recomputes the expected output independently from the listed removals and patches and compares it byte for byte.

**9. The result is data, not a file operation.** `strip` returns `Stripped { bytes, removals, patches }` (owned): each removal records its category, region kind, offset and length in the **input**; each patch its offset and length in the **output**. `sanitize` returns only the bytes. The library does no I/O and never overwrites; a binary (later) decides what to do with the bytes.

**10. Which inputs are accepted.** JPEG only in SPEC-020 (a TIFF, PNG, WebP or HEIC gets `Unsupported`, naming the container; SPEC-022 adds TIFF). A JPEG is refused as `Unstrippable` when its walk was halted by a cap, when any segment has a bad length, or when the segment cap or issue cap was reached. `MissingSos` and `MissingEoi` are not refusals (a truncated JPEG still has removable segments); bytes the walk skipped between segments are kept byte for byte and are not named by the audit (question `writer-stray-bytes`).

## Context

SPIKE-001 showed on real files what a strip must account for, and DEC-012 made the audit name it. What was open was the contract of the writer. Two properties matter more than any feature: it must never *silently* leave something the preview called removed, and it must never touch pixels. A fail-closed plan of removals and same-length patches has a small enough surface to prove both: no offset arithmetic except one named place (the MPF fields), no growth, no relocation. The audit-as-oracle check in the library (part 7) is what turns the stage's strongest test idea into a runtime guarantee.

The Exif rule (part 4) is the least pleasant honest consequence: a segment-level writer can remove "all of Exif" or none, so the narrow presets wait for SPEC-021 instead of being faked by removing too much (which would break "does exactly what the preview says").

## Alternatives considered

- **Best-effort strip with a report of what was not removed.** Rejected: it fails open for any caller that does not read the report; the product's promise is that the bytes are gone.
- **Remove the whole Exif segment for `Gps` too** (over-removal). Rejected: it contradicts the preview ("`Kept`" for Authorship and the rest) and DEC-009's presets; users who want everything use `All`.
- **Keep Orientation when stripping the whole Exif segment** by writing a minimal Exif block. Rejected for this spec: it is an insertion (growth), so it needs its own rule and tests; SPEC-021's in-place deletion keeps Orientation naturally. Recorded as question `strip-keeps-orientation`.
- **Rewrite the MPF index from scratch** after removals. Rejected: patching the existing `u32` fields keeps the segment byte-identical in length and shape, so it cannot grow or reorder anything.
- **Refuse any file that has an MPF index and a removal after it.** Rejected: iPhone and Nikon files are common, and the patch is a two-field arithmetic rule that is easy to test against ExifTool.
- **Skip the self-check for speed.** Rejected: one linear pass is cheap, and the check is what catches a plan bug as a typed error instead of a leak.
- **Make `strip` mutate in place** (`&mut [u8]` or a file). Rejected: no I/O in the library, and a failed strip must leave the input untouched by construction.

## Consequences

- **Positive:** a failed strip is visible; a returned file is audited; the pixel and kept-segment bytes are provably untouched; the plan type is shared by SPEC-021 and SPEC-022.
- **Negative:** in this stage `Gps`, `GpsAndDevice` and `AllPii` work only for files with no Exif finding in their categories (otherwise `NeedsEntryEdit`), `All` and `Everything` drop Orientation, and a hostile file whose embedded image overlaps a removed segment is refused instead of stripped.
- **Neutral:** `Stripped`, `StripError` and the edit plan are `#[non_exhaustive]`; the library gains a `write` module and one new JPEG-layer accessor (the MPF index with field positions).

## Validation

Revisit when SPEC-021 can edit inside the Exif segment (the all-or-nothing rule then narrows), when a real file shows a refusal that a safe edit could have avoided, when a user needs Orientation preserved under `All`, or when a container other than JPEG needs an operation beyond removal and same-length patch.

## References

- Related specs: SPEC-020 (implements for JPEG), SPEC-021, SPEC-022, SPEC-019 (the regions)
- Related decisions: DEC-006 (regions), DEC-008 (categories), DEC-009 (presets, dispositions), DEC-012 (what the audit names)
- Evidence: SPIKE-001 (Multi-Picture offsets, relocation hazard); questions `writer-in-place-delete-and-scrub`, `strip-keeps-orientation`, `writer-stray-bytes`
