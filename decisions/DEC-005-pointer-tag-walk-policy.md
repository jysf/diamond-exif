---
insight:
  id: DEC-005
  type: decision
  confidence: 0.80
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
affected_scope: ["src/tiff/**", "src/error.rs"]

tags:
  - parser
  - ifd-walk
  - policy
---

# DEC-005: Pointer-tag walk — which pointers are followed, from where

## Decision

The walker follows exactly four pointer tags, each only from the IFD kinds listed below. Every IFD is labelled with how it was reached (`IfdKind`). A pointer tag found anywhere else is an ordinary entry and is ignored by the walk, without an issue.

| Pointer tag | Follows from | Child kind | Value rule |
|---|---|---|---|
| `SubIFDs` 330 | any IFD | `Sub` | `LONG`/`IFD`/`BYTE`/`SHORT`/`UNDEFINED` array, count at most `MAX_IFDS`; zero offsets skipped |
| `ExifIFD` 0x8769 | `Ifd0`, `Ifd1`, `Chain`, `Sub` | `Exif` | exactly one `LONG` or `IFD` value |
| `GPSInfo` 0x8825 | `Ifd0`, `Ifd1`, `Chain`, `Sub` | `Gps` | exactly one `LONG` or `IFD` value |
| `InteropIFD` 0xA005 | `Exif` only | `Interop` | exactly one `LONG` or `IFD` value |

- **The next-word of `Exif`, `Gps` and `Interop` IFDs is recorded but not followed.** They are leaf directories by definition; real files carry garbage there often enough that following it would manufacture issues. The chain of `Ifd0` (giving `Ifd1`, then `Chain`) and of `Sub` IFDs is followed as before.
- **Kinds:** the first IFD at the top is `Ifd0`, the next in its chain `Ifd1`, later ones `Chain`; a chain inside a `SubIFDs` branch stays `Sub`.
- **Duplicates:** if an IFD carries the same pointer tag twice, only the first entry is used.
- **Pointer tag with a bad value** (wrong type, count other than one, offset unreadable) records an issue with `Link::Tag(tag)` and `offset: None`; the holding IFD is kept, its other pointers and its chain still walk (DEC-001).
- **Work bound per tag.** A `SubIFDs` entry declaring more than `MAX_IFDS` offsets is rejected (`TagTooLarge`) before any offset is decoded. This closes the gap SPEC-003 verify found (a zero offset cost no attempt and a million offsets were decoded before the budget could halt the walk). The guaranteed issue bound becomes `MAX_ISSUES = 5 * MAX_IFDS + 1` (at most one failed attempt per budgeted visit plus one `TooManyIfds`, plus at most four pointer-tag issues per recorded IFD).
- **Not followed here:** MakerNote (vendor-specific, later), `ExifVersion`-style undefined blobs, and any thumbnail strip data.

## Context

SPEC-002/003 follow only `SubIFDs`. EXIF Make/Model/DateTime live in IFD0 and the Exif IFD, GPS in its own IFD, lens and interop data in Interop. The audit and sanitiser use cases need a stable answer to "where did this IFD come from" and a guarantee that a hostile pointer cannot buy unbounded work. A pointer-tag table is also the seam the writer will use later to find what to rewrite.

## Alternatives Considered

- **Follow every pointer tag from every IFD.** Rejected: allows `Exif -> Exif -> Exif` nesting that no camera writes, and gives the audit CLI IFDs of meaningless kind.
- **Follow the next-word of Exif/GPS/Interop.** Rejected: spec says it is zero; garbage values would produce false cyclic/truncated issues.
- **Charge each offset in a pointer array to the attempt budget instead of capping the count.** Rejected: still decodes up to a million offsets per IFD before the budget bites; a count cap bounds decoding itself.

## Consequences

- **Positive:** the kind of every IFD is explicit; GPS corruption is expressible and testable (corrupt GPS leaves IFD0 and the Exif IFD readable); work and issue counts are bounded by constants.
- **Negative:** a legitimate file with more than 512 `SubIFDs` offsets in one tag loses that branch (reported, not silent). A pointer tag in an unexpected place is invisible to the walk. Both are acceptable for a metadata reader.
- **Neutral:** `Error` gains `BadPointerCount`; `IfdKind` and `MAX_ISSUES` join the public API (both `#[non_exhaustive]` where applicable).

## Validation

Revisit when MakerNote walking lands (it needs a new kind and its own offset base), or if the oracle comparison (SPEC-005) shows ExifTool surfaces an IFD kind this table drops.

## References

- Related specs: SPEC-003, SPEC-004
- Related decisions: DEC-001 (tolerant walk)
- External: TIFF 6.0 §2; Exif 2.32 (CIPA DC-008) IFD and pointer structure; docs/exif-core-design.md §16.3
