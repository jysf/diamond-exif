---
# Maps to ContextCore epic-level conventions.
# A Stage is a coherent chunk of work within a Project.
# It has a spec backlog and ships as a unit when the backlog is done.

stage:
  id: STAGE-001                     # stable, zero-padded, continuous across the repo
  status: active                    # proposed | active | shipped | cancelled | on_hold
  priority: high                    # critical | high | medium | low
  target_complete: null             # optional: YYYY-MM-DD

project:
  id: PROJ-001                      # parent project
repo:
  id: diamond-exif

created_at: 2026-11-04
shipped_at: null

# What part of the project's value thesis this stage advances.
# If you can't articulate value_contribution, the stage may be
# infrastructure-only — acceptable but flag it.
value_contribution:
  advances: "The reader foundation of the thesis: a tolerant, bounds-checked, ExifTool-verified IFD engine that every later stage (audit CLI, writer) builds on."
  delivers:
    - "Buildable, gated single crate with irradiance's five-lint deny set and hardened gate harness"
    - "Tolerant multi-IFD EXIF reader (IFD0, Exif, GPS, Interop) reporting per-IFD ParseIssues"
    - "ASCII, signed and float tag decoding"
    - "ExifTool-oracle parity tests with red-proofs and a tiff_ifd fuzz target"
  explicitly_does_not:
    - "Parse JPEG/PNG/WebP containers (next stage)"
    - "Write or strip anything (writer is net-new, later stage)"
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

# STAGE-001: M1a foundation — TIFF/IFD engine

## What This Stage Is

Stand up the single `exif-core` crate with irradiance's hardened gate harness, record the three founding decisions, port the strict DNG-stripped TIFF/IFD skeleton with its 17 hostile-input tests, and then extend it: tolerant-walk policy, Exif/GPS/Interop pointer walk, ASCII/signed/float decoders, and the ExifTool oracle with red-proofs and a `tiff_ifd` fuzz target. At ship the crate parses bare TIFF/EXIF IFD data, reports per-IFD issues instead of failing whole, and every parity claim has been shown red.

## Why Now

First stage; everything else (containers, audit CLI, writer) depends on the IFD engine. DEC-001 gates the test port: deciding the tolerant-walk policy first avoids churning ~25% of ported tests twice, so the port is deliberately two-phase (strict skeleton green, then policy change as its own diff).

## Success Criteria

- `cargo fmt`, `clippy` (five-lint deny) and `cargo test` green at every spec boundary; `-F unsafe_code` re-check passes.
- DEC-001, DEC-002, DEC-003 merged before any ported test lands.
- 17 hostile-input tests ported; a corrupt GPS IFD leaves IFD0 readable and yields a `ParseIssue`.
- Every ported file has a `docs/provenance-ledger.md` row; ledger header states ExifTool is run, never read.
- Oracle parity (`-T -n -s3`) red-proofed; `tiff_ifd` fuzz target exists and has run 60 s.

## Scope

### In scope
- Crate scaffold, design docs, provenance ledger
- DEC-001/002/003 and the strict skeleton port (`error.rs`, `reader.rs`, `tiff/{mod,types,tags}.rs`)
- Tolerant walk, multi-IFD pointer walk, wider types
- ExifTool-only `tools.rs` subset, `mutate.rs`, shared `probe.rs`, `tiff_ifd` fuzz

### Explicitly out of scope
- JPEG/PNG/WebP parsing, writer, strip policies, `exif-audit`, XMP/IPTC/ICC, workspace promotion

## Spec Backlog

- [x] SPEC-001 (shipped on 2026-11-05) — [S] Scaffold the `exif-core` crate: five-lint deny, deny.toml, licenses, provenance-ledger header
- [ ] SPEC-002 (frame) — [L] Port the strict TIFF skeleton and the 17 hostile-input tests, green, with ledger rows
- [ ] SPEC-006 (frame) — [M] `tiff_ifd` fuzz target on the strict parser (constraint `fuzz-target-per-surface`: lands before SPEC-003)
- [ ] SPEC-003 (frame) — [M] Apply the tolerant-walk policy (`ParseIssue` channel), flipping the affected tests
- [ ] SPEC-004 (frame) — [L] Multi-IFD pointer walk (Exif/GPS/Interop/SubIFDs) and ASCII/signed/float decoders
- [ ] SPEC-005 (frame) — [L] Oracle discipline: exiftool `tools.rs` subset, `parse_fields`, replay-perturb red-proof, `mutate.rs`, `probe.rs`

**Count:** 1 shipped / 5 active / 0 pending

## Design Notes

- DEC-001 (tolerant-walk), DEC-002 (`&[u8]` core, `Bytes` at the edge) and DEC-003 (provenance class 3) are decision files authored in the design session by the architect, not delegated build specs. They land before the port spec is built.
- Open questions to close inside this stage: ifd.rs test classification was by name/range, not execution (verify by running); CI job-to-script wiring is untraced; ExifTool write-mode exit codes are needed before M1c, not here.
- Gate harness is repo setup, NOT a spec or stage (process, not product). It is a precondition for SPEC-001's green bar and is tracked outside the backlog (template-port session). Reconciliation rule: diff each script against irradiance's before overwriting; irradiance is ahead on the hardened audit/red-proof set (SPEC-011/016/023, PATCH-002), the template (0.6.39) may be ahead elsewhere. Check `gh release list -R jysf/spec-driven-template` (needs sandbox bypass).

## Dependencies

### Depends on
- External: ExifTool installed locally and in CI
- Previous projects: irradiance (read-only source)

### Enables
- The JPEG/container stage, the `exif-audit` CLI stage, the writer stages

## Stage-Level Reflection

*Filled in when status moves to shipped.*

- **Did we deliver the outcome in "What This Stage Is"?** 
- **How many specs did it actually take?** 
- **What changed between starting and shipping?** 
