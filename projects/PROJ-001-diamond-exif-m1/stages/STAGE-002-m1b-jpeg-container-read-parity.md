---
# Maps to ContextCore epic-level conventions.
# A Stage is a coherent chunk of work within a Project.
# It has a spec backlog and ships as a unit when the backlog is done.

stage:
  id: STAGE-002                     # stable, zero-padded, continuous across the repo
  status: shipped                   # proposed | active | shipped | cancelled | on_hold
  priority: high                    # critical | high | medium | low
  target_complete: null             # optional: YYYY-MM-DD

project:
  id: PROJ-001                      # parent project
repo:
  id: diamond-exif

created_at: 2026-11-05
shipped_at: 2026-11-05

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
  sessions:
    - tokens_total: 535266
      estimated_usd: 3.53
      recorded_at: 2026-11-05
      notes: "the orchestrator conversation after the STAGE-001 close (commit bfbbb7a, 2026-11-05T14:38:44-08:00): CI setup as repo setup, STAGE-002 framing, designs of SPEC-009/010/011 and DEC-006/007, dispatches, independent gate re-runs, fix rounds, ships and PRs 13-15; recovered from transcript 1cd6e68e usage (input + output + cache_creation, de-duplicated by message id, cache_read excluded) at 6.60 USD/Mtok. Read at stage close; the closing turns after the read are not included"
                                    # - tokens_total: N
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
- [x] SPEC-011 (shipped on 2026-11-05) — [L] JPEG read parity against ExifTool

**Count:** 3 shipped / 0 active / 0 pending

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

- **Did we deliver the outcome in "What This Stage Is"?**
  Yes, with one honest narrowing. `exif-core` reads a JPEG however damaged: a segment walker with per-segment typed issues (fatal only for "not a JPEG"), scan data and trailer as borrowed slices, Exif handed to the existing tolerant TIFF engine (container independence proven by a unit test and by the oracle over every wrapped TIFF fixture), and XMP, extended XMP, ICC, IPTC and JUMBF regions located with their small headers read. The oracle now covers JPEG: 79 JPEG records beside the 41 TIFF ones, live and replay parity, every difference written down, both design-section-8 JPEG red-proofs, and mutated-copy faults. Narrowing: C2PA is located only as a JUMBF region; C2PA detection inside it is an open question, and ICC/JUMBF have no ExifTool parity (ExifTool lists nothing for the stubs).

- **How many specs did it actually take?**
  Three against the three framed (SPEC-009 walker, SPEC-010 Exif and regions, SPEC-011 parity). Verify verdicts: approved 2 (SPEC-009, with a follow-up round closing test and seed gaps the verifier found; SPEC-010 with only low findings), punch list 1 resolved by a fix round (SPEC-011: a wildcard divergence that loosened the comparison, three surviving mutants). Metered build and verify cost about 1.71M tokens (about $11.26) across the 3 specs, plus about 0.54M tokens (about $3.53) of orchestration; roughly 2.24M tokens (about $14.79) in total. Per-spec estimates ran under for all three (330k/350k/500k estimated vs 465k/494k/749k actual).

- **How many outlines survived unchanged?**
  3 of 3 in scope; SPEC-010 and SPEC-011 each gained designed content the outline did not predict (DEC-006 clarifications and the walker's four-byte bad-length skip; DEC-007 and the harness model of ExifTool's multi-Exif rule).

- **What changed between starting and shipping?**
  The oracle found a real library gap before any user did (ExifTool reads an Exif block whose sixth signature byte is non-zero; we did not: DEC-007), and the ExifTool measurements replaced three design assumptions (several Exif segments are unioned later-wins, Exif after the scan is ignored, a last segment ending exactly at EOF is dropped), each now a recorded divergence with ours as the safer side.

- **Lessons that should update AGENTS.md, templates, or constraints?**
  - Name every CI-only script in the build brief (PR 14 failed on one); logged as signal `run-ci-only-gates-locally` (watch, N=1).
  - A divergence entry must pin what our side still equals (signal `divergence-entries-must-pin-our-values`, watch, N=1).
  - Treat the design's ExifTool table as a prediction for the build to re-measure: one row (stray-byte warnings) was wrong.
  - CI `oracle-records` now fails on untracked records (done in PR 15).

- **Signals dispositioned at this close?**
  Walked all `type: lesson` signals: `flag-default-explicitness` stays watch; `measure-exit-status-without-a-pipe` stays open at N=1 (SPEC-011's design measured with a file for stdout and the tool's own status, so no recurrence); two new lessons added at N=1 as watch. Nothing codified: none reached its bar.

- **Should any spec-level reflections be promoted to stage-level lessons?**
  - "State cap and halt interactions as a table of cases" (SPEC-009) held again in SPEC-010 (the ICC offset rule was still left open at design; the builder decided and verify approved it); recorded as a below-bar watch note only.
  - Carried forward (not blockers): SBYTE oracle selector (signal), merged multi-Exif view in the library and C2PA-in-JUMBF detection (questions `several-exif-segments`, `c2pa-detection-inside-jumbf`), capturing ExifTool warnings when values were found, upstream `decisions-audit.sh` block-list parsing.
