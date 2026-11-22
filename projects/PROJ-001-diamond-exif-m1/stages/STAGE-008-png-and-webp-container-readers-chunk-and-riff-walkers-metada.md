---
# Maps to ContextCore epic-level conventions.
# A Stage is a coherent chunk of work within a Project.
# It has a spec backlog and ships as a unit when the backlog is done.

stage:
  id: STAGE-008                     # stable, zero-padded, continuous across the repo
  status: proposed                  # proposed | active | shipped | cancelled | on_hold
  priority: high                    # critical | high | medium | low
  target_complete: null             # optional: YYYY-MM-DD

project:
  id: PROJ-001                      # parent project
repo:
  id: diamond-exif

created_at: 2026-11-21
shipped_at: null

# What part of the project's value thesis this stage advances.
# If you can't articulate value_contribution, the stage may be
# infrastructure-only — acceptable but flag it.
value_contribution:
  advances: "Panic-free, differentially verified reads of the two other formats users upload (PNG, WebP), so the audit CLI and the later sanitiser cover every container in the project thesis, not only JPEG and TIFF."
  delivers:
    - "A PNG chunk walker and a WebP RIFF walker, each tolerant (typed per-chunk issues, fatal only for not-the-format) with a fuzz target and a 60 s CI smoke"
    - "Located metadata regions: PNG eXIf, iTXt XMP and iCCP; WebP EXIF, XMP and ICCP with the VP8X flags; the embedded Exif handed to the existing tolerant TIFF engine"
    - "ExifTool read parity on PNG and WebP fixtures, comparator red-proofed"
    - "exif-audit reads PNG and WebP: inventory and PII findings; the strip preview marks them not-yet-strippable"
  explicitly_does_not:
    - "write or strip PNG or WebP (M1d); the writer refuses both, fail-closed, until then"
    - "decode pixels, inflate zlib streams beyond what a located region needs, or parse XMP or ICC contents"
    - "read APNG animation frames, or HEIC (M1e)"

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

# STAGE-008: PNG and WebP container readers: chunk and RIFF walkers, metadata regions, ExifTool read parity, audit wiring

## What This Stage Is

Make `exif-core` and `exif-audit` read a PNG and a WebP the way they already read a JPEG. A chunk walker and a RIFF walker find the metadata chunks, report a malformed chunk as a typed per-chunk issue instead of failing the file (DEC-001 and DEC-006 carry over), borrow every payload from the input, and hand an embedded Exif blob to the existing tolerant TIFF engine. ExifTool agrees on fixtures, with the comparator red-proofed, and `exif-audit` stops calling these two formats unsupported.

## Why Now

STAGE-002 set the container pattern and STAGE-003 to STAGE-006 built the audit CLI and shipped it, so the cost of a new container is now small and well understood: the category table, sniffing, report rendering and policy preview all exist. The alpha binary reports PNG and WebP as unsupported, which is the largest visible gap for a server-side upload checker. Reading is deliberately separate from writing: M1d adds the writers on top of walkers that are already fuzzed and oracle-checked, the order that worked for JPEG.

## Success Criteria

- A `png_chunks` and a `webp_riff` fuzz target exist, seeded from hand-built shapes, each with a 60 s smoke job in CI (constraint `fuzz-target-per-surface`).
- Exif read from a PNG `eXIf` chunk or a WebP `EXIF` chunk equals the same blob read as bare TIFF (container independence), by test and by the oracle.
- Hostile shapes are typed outcomes, never panics: a bad signature, a chunk length past EOF, a CRC mismatch (reported, not fatal), a missing IEND or IHDR, `eXIf` after IDAT, several `eXIf`, RIFF size smaller or larger than the file, an odd chunk without its pad byte, `VP8X` flags that disagree with the chunks present, trailing bytes, and chunk and issue caps.
- XMP and ICC regions are located with offsets and lengths and borrowed byte for byte; compressed PNG text (`iTXt` with the compression flag) is located and reported as compressed, not inflated.
- ExifTool read parity on PNG and WebP fixtures with the comparator red-proofed (replay-and-perturb) and format-shaped mutated-copy red-proofs (control first); every divergence reported, never silenced.
- `exif-audit` audits PNG and WebP: inventory, PII findings, text, JSON and CSV output; the policy preview marks them not strippable and never claims a removal; the writer refuses them fail-closed.
- All CI jobs green on every PR; `[dependencies]` still empty unless a DEC says otherwise.

## Scope

### In scope
- `src/png/` and `src/webp/` walkers, issue types, metadata regions (kind, offset, length, small headers), Exif hand-off
- Fuzz targets `png_chunks` and `webp_riff`, seeds, CI smoke, `fuzz_layout` pins
- ExifTool parity and red-proofs for both formats; hostile-shape fixtures generated by one script
- Audit wiring: sniff to audit, category rows for the new region kinds, preview and report behaviour, README and CHANGELOG lines

### Explicitly out of scope
- PNG and WebP writers and strip (M1d); `rewrite_png` fuzz target
- APNG frame handling, animated WebP frame parsing (ANIM and ANMF chunks are located and passed over, not decoded)
- Pixel decoding, zlib inflation of image data, XMP or ICC content parsing (open question `xmp-scanner-vs-roxmltree` stays open)
- Text-chunk Exif encodings other than `eXIf` (`Raw profile type exif` in `tEXt` and `zTXt`) are located and counted as an audit finding if ExifTool reads them; they are located and flagged, never decoded (DEC-018); decoding is a later decision
- A new release; the next binary release is its own spec after this stage

## Spec Backlog

- [x] SPEC-028 (shipped on 2026-11-21) — [M] PNG chunk walker: signature, chunk lengths, CRC as an issue, ordering, caps, `png_chunks` fuzz target (DEC for the PNG walk, modelled on DEC-006)
- [x] SPEC-029 (shipped on 2026-11-21) — [M] PNG metadata regions: `eXIf` to the TIFF engine, `iTXt` XMP, `iCCP`, text-chunk Exif located; container independence
- [x] SPEC-030 (shipped on 2026-11-21) — [M] WebP RIFF walker and metadata regions: RIFF size rules, `VP8X` flags, EXIF, XMP, ICCP, pad bytes, animation chunks passed over, `webp_riff` fuzz target (DEC for the RIFF walk)
- [x] SPEC-031 (shipped on 2026-11-21) — [L] PNG read parity against ExifTool: fixtures and records, comparator, warning table, replay-and-perturb and mutated-copy red-proofs
- [x] SPEC-034 (shipped on 2026-11-22) — [L] WebP read parity against ExifTool (applies the SPEC-031 apparatus) and the SPEC-030 carried test and seed gaps
- [x] SPEC-035 (shipped on 2026-11-22) — [S] JPEG oracle: per-selector several-Exif-block model, measured (follow-up to SPEC-034 verify H1; test harness only)
- [x] SPEC-032 (shipped on 2026-11-22) — [L, two builder parts] Audit wiring: PNG and WebP into exif-audit (kinds, text-keyword categories, honest preview, writer refusal, renderers, CLI, fuzz seeds, README); DEC-020
- [ ] SPEC-036 (not yet created) — [M] Tag-level inventory parity against ExifTool for PNG and WebP (split out of SPEC-032)

**Count:** 7 shipped / 0 active / 1 pending

## Design Notes

- **Two walkers, one pattern.** Fatal only when the input is not the format (too short, wrong signature); everything else is an issue on the returned value and the walk continues where safe. Issue types are per layer, not merged with the TIFF layer's (DEC-006). Caps are constants, as for JPEG.
- **Why PNG takes two specs and WebP one.** PNG has the larger metadata surface (three chunk families plus text variants), so the walk and the regions are separate reviews. WebP's RIFF walk and its regions are smaller and tightly coupled through `VP8X`, so one spec, with the estimate marked [M] and a split allowed at design if the verifier would otherwise see an oversized diff.
- **Zero-copy.** Every payload borrows the input (DEC-002). Nothing inflates or decodes pixels.
- **Fixtures** are tier A and hand-built by one generator script (as for TIFF and JPEG); no real photograph is committed. ExifTool is run only through `run_tool_with_limit`, and each new hostile shape is probed under the time limit first.
- **Order.** Walkers first, then parity, then audit wiring; the parity spec depends on both readers, so the two walker specs may build in either order but not at the same time as parity.
- **Sniffing is already done.** `src/audit/sniff.rs` returns `Png` and `Webp`; the audit only needs to dispatch on them.

## Dependencies

### Depends on
- STAGE-002 (shipped) — the container pattern, DEC-006, the oracle apparatus
- STAGE-003 and STAGE-005 (shipped) — the audit model, category table (DEC-008) and renderers
- External: ExifTool 13.55 as the oracle (run, never read or linked)

### Enables
- M1d — PNG and WebP writers and strip build on fuzzed, oracle-checked walkers
- A later binary release whose `exif-audit` reads PNG and WebP

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
