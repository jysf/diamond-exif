---
# Maps to ContextCore epic-level conventions.
# A Stage is a coherent chunk of work within a Project.
# It has a spec backlog and ships as a unit when the backlog is done.

stage:
  id: STAGE-010                     # stable, zero-padded, continuous across the repo
  status: proposed                  # proposed | active | shipped | cancelled | on_hold
  priority: high                    # critical | high | medium | low
  target_complete: null             # optional: YYYY-MM-DD

project:
  id: PROJ-001                      # parent project
repo:
  id: diamond-exif

created_at: 2026-11-22
shipped_at: null

# What part of the project's value thesis this stage advances.
# If you can't articulate value_contribution, the stage may be
# infrastructure-only — acceptable but flag it.
value_contribution:
  advances: "Closes the gap the audit now shows: exif-audit reads PNG and WebP but says they are not strippable, so the sanitiser still covers only JPEG and TIFF; this stage makes strip work for the two other formats users upload, with the same fail-closed, self-verified discipline."
  delivers:
    - "strip of a PNG and of a WebP under every StripPolicy: whole-chunk and RIFF-level removal, output re-audited, typed refusals, never longer than the input"
    - "rewrite_png and rewrite_webp fuzz targets with a 60 s CI smoke"
    - "ExifTool write parity (I3) for both formats, comparator and writer red-proofed"
    - "exif-audit's preview says Removed for PNG and WebP only where the writer has shown it can"
  explicitly_does_not:
    - "set operations, MakerNote rules, HEIC, or a release (a release is its own spec afterwards)"
    - "edit inside XMP or ICC payloads, re-encode pixels, or handle APNG frames"

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

# STAGE-010: M1d PNG and WebP strip: chunk-level and RIFF-level metadata removal, self-verified, ExifTool I3, audit switch-over

## What This Stage Is

Make `strip` work for PNG and WebP the way it works for JPEG and TIFF: remove the metadata the policy names, prove it by re-auditing the output, refuse rather than guess, and show agreement with ExifTool. After this stage `exif-audit` can honestly say a PNG or WebP would be stripped, and the library's writer covers every container read so far.

## Why Now

STAGE-008 shipped the readers and the audit, and the audit's preview states plainly that both formats are "not strippable". The walkers are fuzzed and oracle-checked, the audit lists every chunk that holds metadata, and DEC-013's writer discipline is proven on two containers. The strip is the largest remaining visible gap for a server-side upload sanitiser. Inventory parity (SPEC-036, deferred from STAGE-008) comes first because every strip is verified by re-auditing its output.

## Success Criteria

- `strip` of a PNG and of a WebP under all five presets returns a file the audit finds clean of the removed categories, or a typed refusal; no panic on any input; the output is never longer than the input.
- Nothing but the policy's targets changes: kept chunks are byte-identical (PNG CRCs stay valid), the RIFF size and `VP8X` flags are consistent, pad bytes are right.
- Hostile shapes are handled by walking every chunk, not by the library's lists: a 17th WebP `EXIF` block, text after `IEND` or after a null chunk, several `eXIf`, raw-profile chunks that hold GPS, `tIME` of any length, empty regions, a nameless empty `iCCP`.
- `rewrite_png` and `rewrite_webp` fuzz targets exist, seeded, with a 60 s smoke in CI (constraint `fuzz-target-per-surface`).
- ExifTool I3: the stripped file read by ExifTool prints none of what the policy removed and everything else unchanged, comparator red-proofed, writer mutated-copy red-proofed, every divergence reported.
- `ContainerKind::strippable()` flips to true for PNG and WebP only in SPEC-039, with the preview and CLI honest per file; all CI jobs green on every PR; `[dependencies]` still empty.

## Scope

### In scope
- Tag-level inventory parity for PNG and WebP (SPEC-036)
- PNG and WebP writers under `src/write/`, DEC-021 (PNG write) and DEC-022 (WebP write), their fuzz targets
- ExifTool write parity, audit and CLI switch-over, README

### Explicitly out of scope
- `set` operations, MakerNote read/strip rules, HEIC (M1e), APNG and animated-WebP frame editing
- Editing inside XMP or ICC payloads, pixel decoding, re-encoding
- A release; the Relocatable Exif block (STAGE-007, requested by monorail-000, framed separately)

## Spec Backlog

- [x] SPEC-036 (shipped on 2026-11-22) — [L, two builder parts and a fix round] Tag-level inventory parity against ExifTool for PNG and WebP (carried from STAGE-008)
- [ ] SPEC-037 (design) — [L, two builder parts] PNG strip: whole-chunk removal under StripPolicy, `eXIf` decision, self-verify, `rewrite_png` fuzz, DEC-021
- [ ] SPEC-038 (frame) — [L] WebP strip: RIFF-level removal, size and `VP8X` rewrite, self-verify, `rewrite_webp` fuzz, DEC-022
- [ ] SPEC-039 (frame) — [M] ExifTool write parity (I3) for both strips; audit and CLI switch-over

**Count:** 1 shipped / 0 active / 3 pending

## Design Notes

- **Same discipline as DEC-013:** pure function, `EditPlan` removals and same-length patches, output re-audited and compared with the input's audit, fail closed with typed refusals. A PNG removal is a whole chunk (kept CRCs untouched); a WebP removal changes the RIFF size and `VP8X` flags.
- **Walk, do not list.** The strip works from the chunk or RIFF walk. The library lists at most 16 WebP `EXIF` blocks and a PNG raw profile is not decoded; a strip that relied on those lists would leave GPS behind. SPEC-032's verify found this class three times.
- **Measure hostile edges in the design**, against ExifTool, as a variant grid (signal `design-must-measure-hostile-edges-not-only-the-happy-keys`).
- **Order.** SPEC-036 first (the re-audit the strips rely on must be checked), then SPEC-037 and SPEC-038 (independent; do not build at the same time), then SPEC-039. Specs are framed as outlines only; each is designed just in time.
- **The preview stays honest until the writer is proven:** `strippable()` flips in SPEC-039, per file where the writer can act.
- **Parallel sessions:** STAGE-007 (SPEC-025 to SPEC-027) and STAGE-009 (SPEC-033) are other sessions' work and touch `src/write` and `src/audit/category.rs`; merge main before each design.

## Dependencies

### Depends on
- STAGE-008 (shipped, PR #51 for its close): the readers, the audit wiring (DEC-020), the PNG and WebP oracle fixtures
- STAGE-004 (shipped): the writer foundation (DEC-013), `EditPlan`, the I3 apparatus
- External: ExifTool 13.55 as the oracle (run, never read or linked)

### Enables
- A later binary release whose `exif-audit` strips PNG and WebP (and a crates.io pre-release)
- M1e: HEIC read, the 24 h fuzz gate

## Stage-Level Reflection

*Filled in when status moves to shipped.*

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
