---
insight:
  id: DEC-007
  type: decision
  confidence: 0.85
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

created_at: 2026-11-05
supersedes: null
superseded_by: null
status: accepted
deciders: [jysf, claude]
affected_scope:
  - "src/jpeg/**"
  - "fuzz/**"
  - "tests/**"

tags:
  - parser
  - jpeg
  - exif
  - pii
  - oracle
---

# DEC-007: An Exif APP1 segment is recognised by five bytes (`Exif\0`), not six

## Decision

A JPEG `APP1` segment is classified as Exif when its payload is at least six bytes long and its first **five** bytes are `45 78 69 66 00` (`Exif\0`). The sixth byte is **not checked**. The TIFF data starts at payload offset 6, as before. A payload shorter than six bytes, a different marker, or any other first five bytes is not Exif. Writers (M1c) still emit the canonical `Exif\0\0`; only the reader is lenient.

## Context

The Exif standard says `Exif\0\0`. SPEC-009 and SPEC-010 classified on all six bytes. SPEC-010's verify noticed that no test pinned the sixth byte and called the 5-byte comparison a surviving mutant; SPEC-011's design then **measured** ExifTool 13.55 on a JPEG whose APP1 begins `Exif\0` followed by a non-zero byte and a valid TIFF header at offset 6: ExifTool reads the tags (`-T -n -s3 -IFD0:Make` prints the value). With the sixth byte altered the same way at the *fifth* position (`Exif` + non-NUL), ExifTool reads nothing. So ExifTool's rule is five bytes.

This library's job includes finding personal data in uploads (GPS, serial numbers, authorship) and, later, removing it. A block that ExifTool, and therefore a large share of downstream tools, will read but this library classifies as "not Exif" is metadata the audit would not report and the stripper would not remove. Strictness here is a privacy bug, not a safety feature. The risk of the lenient rule is a false positive on an APP1 that begins `Exif\0` and is not Exif; the TIFF header check (`Container::parse`) already turns that into a typed per-block error rather than a failure.

## Alternatives considered

- **Keep six bytes, record the divergence.** Rejected: the divergence goes the unsafe way (we miss what the reference tool reads).
- **Accept `Exif` followed by any two bytes.** Rejected: measured, ExifTool does not read it; it would invent Exif blocks nobody else sees.
- **Scan for a TIFF header anywhere in the APP1.** Rejected: unbounded guessing, and ExifTool does not do it.

## Consequences

- **Positive:** reader superset of ExifTool for the Exif signature; one more case the oracle pins.
- **Negative:** `SegmentKind::Exif` now includes a non-canonical shape; callers who want canonical-only must look at payload byte 5 themselves. A rewriter must normalise the signature when it rewrites the block.
- **Neutral:** `Error`, `Region` and the public API are unchanged. SPEC-009's classification test, the `jpeg_segments` fuzz `kind` invariant and the shared harness re-derivation change with it (SPEC-011 lists them).

## Validation

Revisit if ExifTool's rule changes in a later version (the pinned version is in `.github/workflows/ci.yml`; the measurement is recorded in `tests/oracle-fixtures/exiftool/jpeg-exif-sixth-byte-nonzero.txt`), or if a real-world false positive shows up in the audit stage.

## References

- Related specs: SPEC-009, SPEC-010, SPEC-011
- Related decisions: DEC-006 (JPEG walk; signatures identify, they do not decode)
- Measured: ExifTool 13.55, `exiftool -T -n -s3 -IFD0:Make <jpeg>` on `Exif\0` + non-zero byte + valid TIFF (reads), on `Exif` + non-NUL (does not read)
