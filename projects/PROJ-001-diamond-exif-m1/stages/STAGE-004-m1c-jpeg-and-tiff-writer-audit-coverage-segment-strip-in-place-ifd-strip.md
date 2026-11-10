---
# Maps to ContextCore epic-level conventions.
# A Stage is a coherent chunk of work within a Project.
# It has a spec backlog and ships as a unit when the backlog is done.

stage:
  id: STAGE-004                     # stable, zero-padded, continuous across the repo
  status: active                    # proposed | active | shipped | cancelled | on_hold
  priority: high                    # critical | high | medium | low
  target_complete: null             # optional: YYYY-MM-DD

project:
  id: PROJ-001                      # parent project
repo:
  id: diamond-exif

created_at: 2026-11-09
shipped_at: null

# What part of the project's value thesis this stage advances.
# If you can't articulate value_contribution, the stage may be
# infrastructure-only — acceptable but flag it.
value_contribution:
  advances: "The write half of the product: a verified, panic-free JPEG and TIFF stripper whose output is checked against its own audit and ExifTool, so 'strip GPS' means the bytes are gone, not hidden."
  delivers:
    - "The audit reports the structures real files carry that a strip must handle (Multi-Picture secondary images, previews inside MakerNotes, unrecognised APPn segments, trailers, Photoshop resources split across APP13 segments), so the policy preview is true"
    - "A library `strip` / `sanitize` for JPEG: whole-segment removal for the broad presets, then in-place IFD entry deletion with scrubbing for the narrow ones, never growing an Exif block and never relocating a MakerNote"
    - "A TIFF and DNG strip in place that never touches strip or tile offsets"
    - "Invariants checked on every fixture and by fuzz: pixel bytes unchanged, the output's own audit shows none of the categories the preview said were removed, and ExifTool agrees with the result"
  explicitly_does_not:
    - "Add or change metadata (`set` Orientation, Artist, Copyright, UserComment, XMP properties): strip only; the set operations are a later stage"
    - "Write PNG, WebP or HEIC (M1d and later), or ship a write command-line tool (the audit CLI stays read-only; a `strip` binary is its own later spec)"
    - "Edit inside an XMP packet or an IPTC record: whole-segment removal only"
    - "Parse MakerNotes (they are removed whole or kept byte for byte)"

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

# STAGE-004: M1c JPEG and TIFF writer: audit coverage, segment strip, in-place IFD strip

## What This Stage Is

Make the verified reader able to remove metadata. `exif-core` gains a pure `strip(bytes, StripPolicy) -> Vec<u8>` (and a one-shot `sanitize`) for JPEG and TIFF/DNG that does exactly what DEC-009's preview says it would do and nothing else: pixel data is never touched, removed entries are scrubbed so their values do not linger in dead space, an Exif block is never grown, and a MakerNote is removed whole or kept byte for byte (never relocated). The stage starts by making the audit see the structures real files carry that a strip must deal with (found by SPIKE-001 on real camera and phone files), then builds the writer in order of risk: whole-segment removal first, in-place IFD entry deletion second, TIFF and DNG third.

## Why Now

STAGE-003 shipped the audit and fixed what a policy means (DEC-009) before any writer existed, which is what lets the writer be checked against a preview. SPIKE-001 then showed on real files what the writer must account for: Multi-Picture secondary images with their own metadata, preview JPEGs inside Nikon/Fuji/Leica MakerNotes, proprietary APP10 segments, Exif blocks at the 64 KB ceiling, and absolute-offset Canon and Sony MakerNotes that break when moved (4 of 7 tested). Those findings decide the design; building the writer first would have rediscovered each as a bug. The strip is also the open-core product's main promise.

## Success Criteria

- `strip` removes exactly the categories each preset names (DEC-009) from JPEG and TIFF/DNG, with `All`/`Everything` by whole-segment removal and `Gps`/`GpsAndDevice`/`AllPii` by in-place entry deletion plus scrubbing, and reports (not silently ignores) anything it cannot remove.
- Invariant I1 (pixel integrity): the compressed image bytes of the output equal the input's, checked on every fixture, by fuzz, and on the local real-file corpus.
- Invariant "audit is the oracle": for every preset, `audit(strip(x, p))` contains no finding whose category the preview marked `Removed`, and the preview's `Kept`/`KeptUnread` findings are all still present.
- ExifTool agrees: for the hand-built fixtures, the stripped file's ExifTool view (group and id) equals what remains per the policy, and `exiftool -all=`-style output is compared where an equivalent exists; any difference is a pinned divergence.
- A writer fuzz target (`rewrite_jpeg`, and a TIFF one) with the invariants as guards; 60 s CI smoke; the 24 h gate stays a milestone item.
- No Exif block grows; no MakerNote moves; no panic on any input; removed entry values are zeroed (a test greps the output for the removed bytes).
- Re-running the local real-file corpus (SPIKE-001's folder) with every preset: every output is pixel-identical, re-audits clean of the removed categories, and ExifTool reads it without new warnings; the result is logged, nothing from the corpus is committed.

## Scope

### In scope
- Audit coverage of Multi-Picture images, MakerNote previews, unrecognised APPn segments, trailers and split APP13 resources, with the DEC-008/DEC-009 amendments and synthetic fixture builders for those shapes (real files cannot be committed)
- The writer design DEC (change plan, I1, scrub, no growth, no relocation, MPF as a unit, defaults for unknown segments and trailers)
- JPEG segment-level strip, JPEG in-place IFD strip, TIFF/DNG in-place strip
- `rewrite_jpeg` and TIFF-writer fuzz targets, ExifTool output oracle and red-proofs

### Explicitly out of scope
- Any `set` operation; XMP property or IPTC record editing; MakerNote parsing
- PNG, WebP, HEIC readers or writers; a write CLI; in-place file overwrite (the library returns bytes)

## Spec Backlog

Ordered list of specs composing this stage. Add specs as you identify
them. Update status as specs progress.

Format: `- [status] SPEC-ID (cycle) — one-line summary`

- [x] SPEC-019 (shipped on 2026-11-10) — [L] Audit coverage of writer-relevant structures: Multi-Picture secondary images, MakerNote previews, unrecognised APPn (APP10), trailers, split APP13 resources; DEC-008/009 amendments; synthetic real-world-shape fixture builders
- [x] SPEC-020 (shipped on 2026-11-10) — [L] Writer foundation and JPEG segment-level strip: writer DEC, change plan, pixel-integrity and audit-as-oracle invariants, `All`/`Everything`, `rewrite_jpeg` fuzz target, ExifTool output oracle
- [ ] SPEC-021 (frame) — [L] JPEG in-place IFD entry deletion with scrub: `Gps`, `GpsAndDevice`, `AllPii` (thumbnail and MakerNote removed whole), never grow, never relocate
- [ ] SPEC-022 (frame) — [M] TIFF and DNG in-place strip: never touch strip or tile offsets, SubIFDs, TIFF writer fuzz target

**Count:** 1 shipped / 3 active / 0 pending

## Design Notes

- **Order by risk.** Whole-segment removal needs no offset arithmetic; in-place entry deletion does; TIFF/DNG adds raw-data offsets that must never move. Each step is shippable and verified before the next.
- **The audit is the oracle.** DEC-009 made presets data; the strongest check on the writer is the repo's own audit run on its output, with ExifTool as the independent second check.
- **Three choices settled at the first design, each with a recommendation:** unknown segments and trailers are kept by default and removed only under `Everything` (the preview names them `Kept`/`KeptUnread` so nothing is silent); the library returns new bytes and never overwrites; the writer is library-only in this stage.
- **Evidence, not assumption.** SPIKE-001 (archived) is the source for the structures and for the relocation hazard; the local real-file corpus is re-run at stage close (a log, not committed).
- **Real files are not fixtures.** Every shape from the spike gets a synthetic builder in `tests/support`, because the corpus cannot be committed.

## Dependencies

### Depends on
- STAGE-003 (shipped) — the audit model, DEC-008/009, the preview, the oracle apparatus and the report renderers
- SPIKE-001 (landed) — the real-file evidence
- External: ExifTool 13.55 as the oracle (run, never read or linked)

### Enables
- M1d: PNG and WebP write/strip, the `set` operations, MakerNote rules
- A `strip` binary and the paid tier's batch features

## Stage-Level Reflection

*Filled in when status moves to shipped. Run Prompt 1c (Stage Ship) in
FIRST_SESSION_PROMPTS.md to draft this.*

- **Did we deliver the outcome in "What This Stage Is"?** <yes/no + notes>
- **How many specs did it actually take?** <number vs. plan>
- **How many outlines survived unchanged?** <n of m — only if this stage was
  framed with `just frame-stage`. Counts outlines whose scope held from frame to
  ship. A low number means framing ran ahead of what was knowable; a high one
  means it's paying for itself.>
- **What changed between starting and shipping?** <one sentence>
- **Lessons that should update AGENTS.md, templates, or constraints?**
  - <one-line updates>
- **Signals dispositioned at this close?** (Prompt 1d step 7) Every
  `type: lesson` signal in `/guidance/signals.yaml` owned by this stage close
  was walked — codified (at its bar), left `watch`, or dropped. No silent carry.
  - <note what codified / what's still watch + its N>
- **Should any spec-level reflections be promoted to stage-level lessons?**
  - <one-line items — record below-bar ones as `watch` signals; don't codify yet>
