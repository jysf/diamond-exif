---
insight:
  id: DEC-003
  type: decision
  confidence: 0.90
  audience:
    - developer
    - agent

agent:
  id: claude-sonnet-5-5
  session_id: null

project:
  id: PROJ-001
repo:
  id: diamond-exif

created_at: 2026-11-04
supersedes: null
superseded_by: null
status: accepted
deciders: [jysf, claude]
affected_scope:
  - "docs/provenance-ledger.md"
  - "src/**"
  - "tests/support/**"

tags:
  - provenance
  - licensing
---

# DEC-003: Ported code is provenance class 3, derived from class 1

## Decision

Code copied from irradiance and then edited is recorded as provenance class 3 (our own prior code) derived from class 1 (the published TIFF 6.0 specification), with one `docs/provenance-ledger.md` row per file recorded before or when it lands; ExifTool is run as an oracle and its source is never read.

## Context

Design §2 calls the port "class 1". irradiance's ledger defines class 1 as a published specification and class 3 as our own prior code (porting brief §5). `src/ifd.rs` was itself written from TIFF 6.0 and the Adobe DNG spec, not from any implementation, so the port is class 3 with class 1 upstream. The permissive-licence claim is only defensible if the ledger is honest. Copyleft check done: `src/ifd.rs` imports only `crate::Error`, `[dependencies]` is empty, and `dnglab` (LGPL) appears only as a spawned tool and is dropped.

## Alternatives Considered

- **Option A: label the port class 1.**
  - Why rejected: inaccurate; the code is a copy of prior code, not a fresh reading of the spec.
- **Option B: rewrite clean-room from the spec.**
  - Why rejected: discards tested, fuzzed code for no licence benefit, since the author and licence are the same.
- **Option C (chosen): class 3 (→1), per-file ledger row.**
  - Why selected: truthful, cheap, and auditable.

## Consequences

- **Positive:** the ledger can back the "permissive, no copyleft" claim; new modules (JPEG, PNG, WebP, XMP, IPTC, ICC) are class 1 and must name the exact spec and edition.
- **Negative:** every ported file costs a ledger row; "port" means copy-then-edit, never a link.
- **Neutral:** the ledger header must state that ExifTool (Perl, Artistic/GPL) may be run but never read to resolve a format question, and that running a tool imposes nothing while reading copyleft source is a violation.

## Validation

Revisit if a dependency is added that carries its own provenance, or if code is ever taken from a source that is not the author's own.

## References

- Related specs: SPEC-001, SPEC-002
- External: docs/porting-brief-diamond-exif.md §5; irradiance `docs/provenance-ledger.md`
