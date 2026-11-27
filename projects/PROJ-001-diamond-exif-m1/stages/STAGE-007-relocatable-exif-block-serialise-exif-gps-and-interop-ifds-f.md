---
# Maps to ContextCore epic-level conventions.
# A Stage is a coherent chunk of work within a Project.
# It has a spec backlog and ships as a unit when the backlog is done.

stage:
  id: STAGE-007                     # stable, zero-padded, continuous across the repo
  status: active                    # proposed | active | shipped | cancelled | on_hold
  priority: medium                  # critical | high | medium | low
  target_complete: null             # optional: YYYY-MM-DD

project:
  id: PROJ-001                      # parent project
repo:
  id: diamond-exif

created_at: 2026-11-20
shipped_at: null

# What part of the project's value thesis this stage advances.
# If you can't articulate value_contribution, the stage may be
# infrastructure-only — acceptable but flag it.
value_contribution:
  advances: "The writer half of the thesis grows from removing bytes in place to building IFDs: the library can hand another TIFF writer a self-checked copy of a source's Exif, GPS and Interop IFDs that ExifTool reads back unchanged."
  delivers:
    - "A pure, deterministic builder that serialises a source TIFF or DNG's Exif IFD, GPS IFD and Interop IFD, with every out-of-line value, as one block in the byte order the caller asks for (DEC-014)"
    - "A size query and a write at a caller-given base offset with final offsets (DEC-015), checked by reading the block back before any byte is returned"
    - "A record of every entry left out (MakerNote always, plus entries whose bytes cannot be moved or byte-swapped safely)"
    - "ExifTool parity for a placed block against its source in both byte orders, and a fuzz target for the builder"
    - "Optionally, a DEC-009 strip-policy preset applied while building"
  explicitly_does_not:
    - "write IFD0, its structural tags, SubIFDs or image data (the placing writer owns those)"
    - "parse, move or rewrite a MakerNote (DEC-013 C; question makernote-internal-offsets)"
    - "add set operations, a strip CLI, or PNG, WebP or HEIC support (M1d and later)"
    - "publish to crates.io: an early exif-core pre-release follows this stage as its own release spec (monorail-000/DEC-077 item 2; question exif-block-crates-io-release, answered)"

# Orchestration cost — the spend that has no spec to attach to (roadmap:
# orchestration + framing cost attribution). Framing a stage, deciding the spec
# breakdown, and cross-spec steering all happen BEFORE/BETWEEN specs, so today
# they are invisible and recorded cost is systematically under-counted.
#
# THE ORCHESTRATOR FILLS THIS — not the human. At stage close, read your own
# session total (`/cost` in Claude Code; the `usage` object via API) and append
# one entry. Stage grain ONLY: do not try to split orchestration across specs —
# that is a division you cannot observe, so any per-spec number is invented.
# Warn-only, never a gate. A null here is honest; a guess is not. (DEC-013 §5)
orchestration_cost:
  sessions: []                      # - tokens_total: N
                                    #   estimated_usd: N
                                    #   recorded_at: YYYY-MM-DD
                                    #   notes: "framing + spec breakdown"
---

# STAGE-007: Relocatable Exif block: serialise the Exif, GPS and Interop IFDs for another TIFF writer

## What This Stage Is

A new writer operation: given a source TIFF or DNG, build one self-contained block holding its Exif IFD (0x8769), its GPS IFD (0x8825) and the Interop IFD (0xA005) the Exif IFD points to, with every out-of-line value they reference, in the byte order the caller asks for, laid out for a caller-given base offset. Another TIFF writer places the block and writes the IFD0 pointer tags. Before returning, the library reads the block back and checks that it gives the source's entries, less the ones it reports as left out. MakerNote is always left out in this stage. The builder is pure, deterministic, bounded and fail-closed, like `strip` (DEC-013).

Requested by monorail-000 (its `docs/upstream/diamond-exif-requests.md`, requests 1 to 3, and monorail-000/DEC-075): its Linear DNG export builds the block with this crate, and the irradiance DNG writer (irradiance/STAGE-017) places it.

## Why Now

**Re-framed 2026-11-27.** The other half of the contract now exists. irradiance/STAGE-017 shipped on 2026-11-25: its SPEC-081 places an opaque Exif block at the fixed base, writes the IFD0 pointers and never rebases the block (irradiance/DEC-072, accepted 2026-11-21 with the amendment that makes the Exif offset optional). This crate's builder is the missing producer. STAGE-008 (the PNG and WebP readers), STAGE-010 (M1d, the PNG and WebP strip) and STAGE-011 have shipped since the first framing, so nothing else in PROJ-001 is ahead of this stage except STAGE-012 (proposed, low). The consumer, monorail-000's PROJ-002, has still not started, so the date pressure is the consumer's pin (the crates.io pre-release that follows this stage), not a deadline.

The original framing (2026-11-20) gave these reasons. The first two still hold; the third is done. It is the first code in this crate that **builds** IFDs instead of removing bytes in place, which DEC-013 part 1 left as "a later decision: it needs a growth rule". The M1d `set` operations and question `strip-keeps-orientation` (a minimal Exif block that keeps Orientation) need the same encoder. Building it against a concrete consumer with an acceptance test gives that encoder a sharper first contract than a speculative `set` would. Also, the offset contract had to be agreed with irradiance/STAGE-017 while that stage was in design (done 2026-11-21).

### Where it fits against the M1 milestones

- **STAGE-004 (M1c writer, shipped 2026-11-10)** is strip only: removals and same-length patches, "never grow, never relocate". The block builder is neither, so it is a new stage, not a STAGE-004 follow-up. It reuses STAGE-004's walk, audit categories and self-check pattern.
- **M1d** (PNG and WebP write and strip, StripPolicy presets, MakerNote rules) is about more containers. The builder adds none. M1d's MakerNote read rules could later make a MakerNote movable (request 2), which would be an amendment to DEC-014, not part of this stage.
- **M1e** (HEIC read, IPTC and ICC passthrough, C2PA) is unrelated.
- **M1f** holds the full crates.io publish. The consumer needs a release it can pin exactly, so jysf chose an early `exif-core` pre-release once this stage ships and SPEC-025 and SPEC-026 are verified (monorail-000/DEC-077 item 2). It is a release spec after this stage, not part of it. STAGE-006 (on its own branch) releases the `exif-audit` binary only, and its planned tag v0.1.0-alpha.1 must not be claimed twice.

## Success Criteria

- A block built from a DNG is placed by a small test-only TIFF writer and reads back through this crate's reader and through ExifTool (`-T -n -s3`) with the same Exif, GPS and Interop values as the source, MakerNote and reported omissions excepted. The parity test has been shown red.
- Covered: a source with an Interop IFD, a source with no GPS IFD, a source with no Exif IFD, and every combination of source and target byte order.
- The same source and options give the same bytes. A block whose size or entry count exceeds the budget is refused before allocation.
- The builder's fuzz target finds no panic in 60 s in CI, and every block it returns reads back clean.
- The offset contract (DEC-015) is accepted by jysf with irradiance/STAGE-017's agreement before SPEC design starts. Done 2026-11-21: irradiance/DEC-072 agrees (base fixed at 8, little-endian).
- A big-endian source converts to the little-endian block the consumer requires (monorail-000's requests document, request 1: "a big-endian source is converted value by value"), with the byte-order-sensitive UNDEFINED tags settled by question `exif-block-byte-order-sensitive-undefined`.

## Scope

### In scope
- The Exif, GPS and Interop IFDs of a classic TIFF or DNG (the bare-TIFF container the writer already accepts). The JPEG Exif blob is the same TIFF structure, so accepting a JPEG source is a design-time choice for the first spec, not a separate spec.
- Byte-order conversion, a size query, a write at a base offset, a records list of omissions, a budget, a self-check.
- ExifTool parity of a placed block, a fuzz target.
- Strip-policy presets applied while building (request 3), as the last and separable spec.

### Explicitly out of scope
- IFD0 and its tags, SubIFDs, IFD1, image data. IFD0's descriptive tags (Make, Model, Artist, Copyright, DateTime, Software, the DNG identity tags) are passed to irradiance by the consumer as typed fields (irradiance/DEC-071 §3a, monorail-000/DEC-077; question `exif-block-ifd0-identity-tags`, answered).
- MakerNote relocation; set operations; containers other than TIFF/DNG (and possibly JPEG); BigTIFF.
- The crates.io pre-release itself (a release spec after this stage).

## Spec Backlog

- [x] SPEC-025 (shipped on 2026-11-27) — [M, was L; split 2026-11-27] Exif block builder in the source's byte order: DEC-014 and DEC-015 contract, size query and write at a base, MakerNote and unsafe entries left out and reported, budget, readback self-check, `exif_block` fuzz target
- [ ] SPEC-041 (frame) — [M, split out of SPEC-025 on 2026-11-27] Byte-order conversion: a big-endian source to the little-endian block the consumer needs, value by value; the UNDEFINED rule (question `exif-block-byte-order-sensitive-undefined`); self-check and fuzz cover the cross-order path
- [ ] SPEC-026 (frame) — [M] ExifTool parity of a placed Exif block: test-only TIFF placer, DNG and TIFF fixtures, both byte orders, Interop and no-GPS cases, red-proofed
- [ ] SPEC-027 (frame) — [S] Strip-policy presets while building the Exif block (DEC-009), with the self-check extended to the policy

**Count:** 1 shipped / 3 active / 0 pending

## Design Notes

- The block is a new writer operation beside `strip`, not an edit plan: DEC-013's "no insert, grow or move" stays true for `strip`. DEC-014 records the block's own contract.
- The builder reads entries through the existing tolerant walk and the audit's categories. It does not add a second definition of which tag is what.
- **SPEC-025 split at the 2026-11-27 re-frame (jysf approved).** As framed it was L, and every L spec in STAGE-010 took two builder parts and a fix round. The seam is byte order: SPEC-025 builds in the source's own byte order (layout, size query, write at a base, omissions, budget, readback self-check, `exif_block` fuzz target) [M]; SPEC-041 converts into the other order, including the UNDEFINED rule [M]. Both are needed before the release, since the consumer requires little-endian from any source. Order: SPEC-025, then SPEC-041, then SPEC-026 (its parity covers all four byte-order pairs, so it depends on both), then SPEC-027 (depends on SPEC-025 only, deferrable).
- **Lessons from STAGE-010 that apply here.** Design measures a variant grid of hostile sources against ExifTool, not only clean DNGs (AGENTS.md §15): a pointer loop between IFDs, an Interop pointer from the GPS IFD, two Exif IFD pointers, offsets past the end, odd counts, an out-of-line value overlapping an IFD, an empty Exif IFD with a GPS IFD. ExifTool runs on the builder's output during build, not only at verify (SPEC-037). Any table the design amends (the omission list, the UNDEFINED rule) lists every hand-typed mirror in src, tests and fuzz (signal `hand-typed-mirrors-of-a-decision-table-drift`).
- **SPEC-027 reads DEC-009 as amended by SPEC-033:** `AllPii` keeps the `Rights` category (Copyright) and removes the rest.
- **DEC-014 accepted** by jysf on 2026-11-27, with the SPEC-025 design amendment (its GPS-only clause binding since irradiance/DEC-072's acceptance on 2026-11-21).
- The consumer's requests are referred to as monorail-000/DEC-075 and its requests document. They are never `depends_on` entries: depends_on names specs in this repo only.

## Dependencies

### Depends on
- STAGE-001 (the tolerant walk, the entry budget), STAGE-004 and STAGE-005 (the TIFF writer path, the audit categories, the self-check pattern).
- External: the offset contract is agreed with irradiance/STAGE-017 (DEC-015 accepted 2026-11-21; irradiance/DEC-072 accepted there the same day). A GPS-only block is allowed (monorail-000/DEC-077 item 1; DEC-014 amendment), and irradiance/DEC-072's amendment (jysf/irradiance#291) makes both offsets optional with at least one required. Gate: SPEC-025's design does not start until irradiance confirms the optional Exif offset (irradiance/DEC-072 accepted as amended, or a new DEC). **Cleared** (checked 2026-11-27): irradiance/DEC-072 is `accepted` (jysf, 2026-11-21) and carries "Amendment 2026-11-21: the Exif IFD offset is optional".
- External, shipped: irradiance/STAGE-017 (2026-11-25), whose SPEC-081 places the block and reads the base from irradiance's public constant. Consumer context: monorail-000/DEC-075, DEC-076 and DEC-077.

### Enables
- monorail-000's Linear DNG export (its PROJ-002), with irradiance/STAGE-017.
- M1d `set` operations and a strip that keeps Orientation (question `strip-keeps-orientation`), which can reuse the IFD encoder.

## Stage-Level Reflection

*Filled in when status moves to shipped. Run Prompt 1c (Stage Ship) in
FIRST_SESSION_PROMPTS.md to draft this.*

- **Did we deliver the outcome in "What This Stage Is"?** <yes/no + notes>
- **How many specs did it actually take?** <number vs. plan>
- **How many outlines survived unchanged?** <n of m>
- **What changed between starting and shipping?** <one sentence>
- **Lessons that should update AGENTS.md, templates, or constraints?**
  - <one-line updates>
- **Signals dispositioned at this close?**
  - <note>
- **Should any spec-level reflections be promoted to stage-level lessons?**
  - <one-line items>
