---
# Maps to ContextCore epic-level conventions.
# A Stage is a coherent chunk of work within a Project.
# It has a spec backlog and ships as a unit when the backlog is done.

stage:
  id: STAGE-002                     # stable, zero-padded, continuous across the repo
  status: active                    # proposed | active | shipped | cancelled | on_hold
  priority: high                    # critical | high | medium | low
  target_complete: null             # optional: YYYY-MM-DD

project:
  id: PROJ-001                      # parent project
repo:
  id: diamond-exif

created_at: 2026-11-05
shipped_at: null

# What part of the project's value thesis this stage advances.
# If you can't articulate value_contribution, the stage may be
# infrastructure-only — acceptable but flag it.
value_contribution:
  advances: "The first real-world container of the thesis: exif-core reads Exif from JPEG, the format uploads actually arrive in, tolerant per segment and verified against ExifTool."
  delivers:
    - "JPEG segment walker with per-segment ParseIssues and a jpeg_segments fuzz target"
    - "Exif read from APP1 through the existing tolerant TIFF engine; XMP, extended XMP, ICC, IPTC and C2PA regions located (not decoded)"
    - "ExifTool read parity on hand-built JPEGs, container-independence proven, JPEG-shaped red-proofs"
  explicitly_does_not:
    - "Parse PNG or WebP (next stage), HEIC, or decode XMP/ICC/IPTC contents"
    - "Write, strip or rewrite anything (writer is M1c)"
    - "Ship the exif-audit CLI"

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

# STAGE-002: M1b JPEG container read parity

## What This Stage Is

Make `exif-core` read a JPEG. A segment walker finds the APPn segments before the scan, keeps the entropy-coded bytes untouched, and reports a malformed segment as a per-segment `ParseIssue` instead of failing the whole file (DEC-001 carried up one level). The APP1 `Exif\0\0` payload is handed to the existing tolerant TIFF engine, so everything built in STAGE-001 applies to JPEG unchanged; XMP, extended XMP, ICC, IPTC and C2PA segments are located and measured but not decoded. The ExifTool oracle gets JPEG fixtures, committed records and JPEG-shaped red-proofs, including the central claim that the same TIFF blob reads identically as a bare TIFF and inside a JPEG.

## Why Now

STAGE-001 delivered the engine and the oracle; JPEG is where uploads live, so it is the first container that matters to the server-side use case and the prerequisite for the `exif-audit` CLI to scan real files. The CI gates are in place, so every spec here is gated from its first PR. Design §12 places the JPEG parser and I2 read parity in M1a; the project kept STAGE-001 TIFF-only on purpose and this stage is the rest of that milestone.

## Success Criteria

- A `jpeg_segments` fuzz target exists, seeded from hand-built shapes, with a 60 s smoke job in CI (constraint `fuzz-target-per-surface`).
- Exif read from a JPEG equals the same blob read as bare TIFF (container independence), proven by a test and by the oracle.
- Hostile JPEG shapes are typed outcomes, never panics: truncated between APP1 and SOS, APP1 length past EOF or 0/1/2, 0xFF fill bytes, missing SOS, SOS without EOI, a second APP1 with a different byte order, APP1 payload at and over the 65,533-byte cap.
- XMP, extended-XMP (GUID/digest), ICC (APP2 chunks), IPTC (APP13) and C2PA (APP11) regions are located with offsets and lengths; scan data is returned as a borrowed slice that equals the input range byte for byte.
- ExifTool read parity on JPEG fixtures with the comparator red-proofed (replay-and-perturb) and JPEG-shaped mutated-copy red-proofs (control first); any ExifTool divergence reported, never silenced.
- All CI jobs green on every PR; `[dependencies]` still empty unless a DEC says otherwise.

## Spec Backlog

Ordered list of specs composing this stage. Add specs as you identify
them. Update status as specs progress.

Format: `- [status] SPEC-ID (cycle) — one-line summary`

`just frame-stage STAGE-NNN` promotes every `(not yet written)` line below into
a real spec at `cycle: frame` with a stable ID (so `depends_on:` can point at
it) and rewrites the line in place. An optional `[S]`/`[M]`/`[L]` tag on the
line is carried onto the spec as `task.complexity`.

- [x] SPEC-009 (shipped on 2026-11-05) — [M] JPEG segment walker with tolerant issues and the jpeg_segments fuzz target
- [x] SPEC-010 (shipped on 2026-11-05) — [M] Exif and located metadata regions from JPEG
- [ ] SPEC-011 (design) — [L] JPEG read parity against ExifTool

**Count:** 2 shipped / 1 active / 0 pending

## Design Notes

- **Issue channel.** A JPEG-level `ParseIssue` equivalent carries the segment index and byte offset; the embedded TIFF's own issues stay on its `Container`. How the two are presented together is decided in a DEC at the design of the walker spec (DEC-006), before any JPEG code, as DEC-001 was for the TIFF walk.
- **Zero-copy.** Every payload and the scan data borrow the input (`&'a [u8]`, DEC-002). Nothing in this stage decodes pixels.
- **Detect and locate, do not decode.** XMP, extended XMP, ICC, IPTC and C2PA are regions with kind, offset and length. XMP parsing (scanner vs `roxmltree`, open question `xmp-scanner-vs-roxmltree`) and ICC/IPTC decoding belong to later stages.
- **Oracle safety.** ExifTool is run only through `run_tool_with_limit`, over files known to terminate; a new hostile JPEG shape is first probed under the time limit, and a non-terminating shape is recorded as a divergence like `two-hop-cycle`, not run in the loop.
- **Fixtures** are tier A, hand-built from the JPEG markers and the Exif/TIFF layout; no camera file is committed.

## Dependencies

### Depends on
- STAGE-001 (shipped) — the tolerant TIFF engine, the ExifTool oracle and its red-proof apparatus, and the CI gates
- External: ExifTool 13.55 as the oracle (run, never read or linked)

### Enables
- STAGE-003 — PNG and WebP readers on the same container pattern
- The `exif-audit` CLI stage — scanning real JPEGs
- M1c — the JPEG writer builds on the segment model

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
