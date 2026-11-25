---
project:
  id: PROJ-001
  status: active
  activity: build
  priority: high
  target_ship: null

repo:
  id: diamond-exif

created_at: 2026-11-04
shipped_at: null
closed_reason: null

value:
  thesis: "A permissively licensed, panic-free, zero-copy EXIF reader/writer/stripper whose parity with ExifTool is shown by red-proofed differential tests and whose parser survives 24 h of fuzzing is a credible foundation for a free audit CLI (open-core funnel) and a paid compliance tier."
  beneficiaries:
    - "Server-side Rust developers (Axum, Actix, Cloudflare Workers) who must sanitize or inspect uploaded images"
    - "Compliance and privacy teams that need report-backed evidence of metadata handling"
    - "The maintainer: a reusable, extracted crate that proves the irradiance method generalizes"
  success_signals:
    - "A corrupt non-IFD0 IFD (e.g. GPS) yields a reportable ParseIssue while IFD0 and the rest still parse"
    - "Every ExifTool parity test has been shown red against a mutated expectation or input"
    - "A fuzz target exists per input surface, runs 60 s in CI, and a 24 h run is clean at the M1 gate"
    - "exif-audit scans a directory and emits inventory, PII findings and a policy preview as text/JSON/CSV with CI exit codes"
    - "Every ported file has a provenance-ledger row and no copyleft code is read or linked"
  risks_to_thesis:
    - "The port is closer to 40% reuse / 60% new than a lift: ASCII types, pointer walks and the whole writer are net-new, so M1 may be underestimated"
    - "The tolerant-walk policy (DEC-001) flips about 25% of the portable tests; deciding late would churn the port twice"
    - "ExifTool write-mode exit codes are unmeasured; I3 write parity could rest on a guess"
    - "Gate scripts cloned from irradiance may drift from the newer template and need per-script reconciliation"

value_realized:
  thesis_held: null
  signals_observed: []
  evidence: null
  notes: null

roadmap: []
---

# PROJ-001: diamond-exif M1 — hardened EXIF core and audit CLI

## What This Project Is

Extract irradiance's TIFF/IFD reader and three-layer oracle tooling into a standalone single crate (`exif-core`), extend it to a tolerant multi-IFD EXIF reader, and build the free read-only `exif-audit` CLI on top of it. The writer/stripper (M1c/M1d), further containers (JPEG, PNG, WebP) and the 24 h fuzz gate complete M1. Governing documents: `docs/exif-core-design.md` (read §16 second) and `docs/porting-brief-diamond-exif.md`; sequencing authority: `docs/DIAMOND-EXIF-EXTRACTION-PLAN.md`.

## Why Now

The design and porting brief are complete and carry named failure modes. The first decision, tolerant-walk (DEC-001), must precede any test port or the work churns twice. `exif-audit` is the open-core funnel, so the reader foundation unblocks the first public deliverable.

## Success Criteria

- Strict TIFF skeleton ported with its 17 hostile-input tests, green, then the tolerant-walk policy applied as its own change.
- Exif, GPS and Interop IFDs walked; ASCII, signed and float types decoded; I1/I2 read parity with ExifTool (`-T -n -s3`) red-proofed.
- `exif-audit` delivers walk, inventory, PII findings, policy preview, `--json`/`--csv` and CI exit codes.
- Quality gates (lint red-proof, gate-audit, cost-audit, validate and their red-proofs) green in CI, set up as repo process outside the spec backlog; five-lint deny set and `forbid(unsafe_code)` re-checked with `-F`.

## Scope

### In scope
- Single crate `exif-core` with an `exif-audit` binary; `src/`, `tests/`, `fuzz/` at the repo root
- TIFF/IFD engine port-and-extend, tolerant walk, oracle discipline, fuzz targets
- JPEG parser, PNG and WebP readers, then the EditPlan → apply writer and strip policies (later stages)
- Provenance ledger row for every ported file

### Explicitly out of scope
- HEIC write, SaaS, runtime license checks or telemetry, a feature-flag split of the core
- Makernote editing, C2PA parsing, PDF/video (the `Container` abstraction must merely not exclude them), any pixel decoding
- Workspace promotion, `exif-pro` and the site (M2, own DEC)

## Stage Plan

- [x] STAGE-001 (shipped 2026-11-05) — M1a foundation: scaffold, gates, DEC-001..005, strict TIFF skeleton port, tolerant walk, multi-IFD walk, decoders, oracle discipline, mutated-copy red-proofs, CI
- [x] STAGE-002 (shipped on 2026-11-05) — M1b JPEG container read parity: segment walker + `jpeg_segments` fuzz, Exif and located regions, ExifTool parity and JPEG red-proofs
- [x] STAGE-008 (shipped on 2026-11-22) — PNG and WebP readers (M1b rest): chunk and RIFF walkers, metadata regions, `png_chunks` and `webp_riff` fuzz targets, ExifTool read parity, audit wiring (DEC-017 to DEC-020); 7 specs, inventory parity (SPEC-036) carried to STAGE-010
- [x] STAGE-003 (shipped on 2026-11-06) — `exif-audit` CLI: audit model, inventory parity, strip-policy preview, renderers + fuzz, the incremental report builder, the binary
- [x] STAGE-004 (shipped on 2026-11-10) — M1c JPEG and TIFF writer: audit coverage of writer-relevant structures, segment-level strip, in-place IFD strip, TIFF/DNG strip (SPIKE-001 is the evidence); 4 specs, 4,921,119 tokens, 32.47 USD
- [x] STAGE-005 (shipped on 2026-11-19) — DEC-008 audit gaps: DNG identity tag categories and a container-aware TIFF page rule; 1 spec, 561,674 tokens, 3.71 USD
- [x] STAGE-006 (shipped on 2026-11-21) — First public alpha: `exif-audit` v0.1.0-alpha.1, a public GitHub pre-release with binaries for Linux x86_64, macOS arm64 and macOS Intel; 1 spec, 1,186,445 tokens, 7.83 USD
- [x] STAGE-009 (shipped on 2026-11-23) — Keep the copyright notice under AllPii: the Rights category (SPEC-033)
- [x] STAGE-011 (shipped on 2026-11-24) — M1 hardening from an external review: the JFIF APP0 thumbnail fix (SPEC-040)
- [ ] STAGE-007 (proposed) — Relocatable Exif block for another TIFF writer (requested by monorail-000; consumer decision monorail-000/DEC-075): Exif, GPS and Interop IFDs rebuilt at a caller-given base, MakerNote left out, ExifTool parity, optional presets; DEC-014, DEC-015; 3 specs framed
- [ ] (not yet defined) — exif-core crates.io pre-release once STAGE-007 ships and SPEC-025 and SPEC-026 are verified, before M1f, for monorail-000 to pin exactly (monorail-000/DEC-077 item 2); a release spec with the DEC-006 pre-flight
- [ ] STAGE-010 (active; 3 of 4 specs shipped) — M1d PNG and WebP strip: tag-level inventory parity (SPEC-036, shipped 2026-11-22), PNG strip (SPEC-037, shipped 2026-11-23), WebP strip (SPEC-038, shipped 2026-11-23), ExifTool write parity and audit switch-over (SPEC-039, in design 2026-11-24: DEC-023); `set` operations and MakerNote rules stay out
- [ ] (not yet defined) — M1e/M1f: HEIC read, IPTC/ICC passthrough, 24 h fuzz gate, publish
- [ ] STAGE-012 (proposed, priority low) — In-browser EXIF viewer and stripper: exif-core compiled to wasm behind a thin bindings crate, a static page that shows the audit and hands back a stripped file, Pages deploy gated on a browser smoke test checked by ExifTool; starts after SPEC-039, and its first spec is the DEC that admits it to M1 or moves it to M2 (the site is out of scope above)

**Count:** 9 shipped / 1 active / 4 pending

## Dependencies

### Depends on
- External: ExifTool installed locally and in CI as an oracle (run, never read or linked)
- Previous projects: irradiance (read-only source at `~/PSeven/experiments/crustimg_redo_plus/irradiance`)

### Enables
- Future projects: M2 workspace promotion, `exif-pro`, static site

## Project-Level Reflection

*Filled in when status moves to shipped.*
