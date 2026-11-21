---
insight:
  id: DEC-017
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

created_at: 2026-11-21
supersedes: null
superseded_by: null
status: accepted
deciders: [jysf, claude]
affected_scope:
  - "src/png/**"
  - "src/error.rs"
  - "fuzz/**"

tags:
  - parser
  - png
  - error-handling
  - policy
---

# DEC-017: PNG chunk walk — what is fatal, and a per-layer issue channel

## Decision

A PNG parse is fatal (`Err`) only when the input is not a PNG: fewer than eight bytes (`Truncated`) or the first eight bytes are not the PNG signature (`NotPng`). Every other problem is a **`PngIssue`** recorded on the returned `Png`, and the walk continues wherever it safely can. This is DEC-006's rule applied to a chunk stream; the issue channel is a third, separate type (not merged with `JpegIssue` or the TIFF `ParseIssue`).

## Context

A PNG is an eight-byte signature followed by chunks: a 4-byte big-endian length, a 4-byte type, the data, and a CRC-32 over type and data. ExifTool 13.55, probed in the SPEC-028 design on hand-built shapes (`-a -G1 -s -n`), is tolerant in the same way: it reads `eXIf` through a bad CRC (no warning), through a missing `IEND` ("Truncated PNG image"), through a missing `IHDR` ("PNG image did not start with IHDR"), and from after `IDAT` or `IEND` (warnings only). It stops on a chunk length past the end of the file ("Corrupted PNG image") and on a length above 2^31-1 ("Invalid PNG chunk size"). The audit and the sanitiser need "the Exif is fine, the file is odd" to be a result, so the same discipline applies.

## Policy detail

- **Fatal:** fewer than 8 bytes, or the signature `89 50 4E 47 0D 0A 1A 0A` absent.
- **Issue, walk continues** (the chunk is recorded unless stated): CRC mismatch (`BadChunkCrc`, with the stored and the computed value; ExifTool ignores CRC, so parity is on data, not on this issue), a chunk type with a byte that is not an ASCII letter (`BadChunkType`), the first chunk not `IHDR` (`MissingIhdr`, one issue), an `IHDR` whose length is not 13 (`BadIhdr`), no `IEND` found (`MissingIend`).
- **Issue, walk ends:** a chunk whose declared length runs past the end of the input, or fewer than 12 bytes left for a chunk (`Truncated`; the chunk is **omitted entirely**, as for JPEG segments and IFDs), a length above 2^31-1 (`BadChunkLength`; the stream cannot be resynchronised), the chunk cap (`TooManyChunks`), the issue cap (`TooManyIssues`). `MissingIend` is recorded only when the walk ended at the end of the input on a chunk boundary: a halting issue suppresses it.
- **`IEND` ends the walk.** Everything after it is the **trailer**, a borrowed slice, not an issue (as JPEG). ExifTool does read metadata chunks that follow `IEND`; whether `exif-core` walks the trailer for metadata is decided in SPEC-029 against the oracle, and the trailer slice is what it will use.
- **Chunk kinds are identification only:** `IHDR`, `PLTE`, `IDAT`, `IEND`, `eXIf`, `iTXt`, `tEXt`, `zTXt`, `iCCP`, `Other`. Contents are not read here (SPEC-029). Each chunk knows whether it comes after the first `IDAT` (`after_idat()`), the analogue of JPEG's `after_scan()`.
- **Resource guards:** `MAX_PNG_CHUNKS = 65_536` recorded chunks and `MAX_PNG_ISSUES = 1024` ordinary issues, each exceeded once by a terminal halting issue. A large legitimate PNG has one `IDAT` per 8 to 64 KiB of compressed data, so the chunk cap is far above JPEG's segment cap; a file past it has its late chunks unread, which is documented, not an error of the walker.
- **CRC-32** is computed per chunk with a built-in table (IEEE 802.3, reflected, polynomial `0xEDB88320`); no dependency. Work is linear in the input.
- **Progress:** every iteration advances the cursor by at least 12 bytes or ends the walk. Iterative, no recursion.

## Alternatives considered

- **Strict (any bad chunk is fatal).** Rejected: contradicts DEC-001; a bad CRC on an unrelated chunk would hide the Exif.
- **Treat a CRC mismatch as fatal for that chunk (drop it).** Rejected: ExifTool reads it, so dropping would make `exif-core` see less than the oracle; the mismatch is an issue and the chunk stays.
- **Coalesce consecutive `IDAT` chunks.** Rejected: hides the real chunk list the writer will need in M1d; a higher cap is simpler.
- **Share one issue type across JPEG and PNG.** Rejected: hides the layer (DEC-006).

## Consequences

- **Positive:** the same tolerant discipline for the third container; hostile PNG shapes are testable data; memory and issue counts bounded by constants.
- **Negative:** callers consult another issue list; CRC computation adds a pass over the data.
- **Neutral:** `Error` gains variants (additive on a `#[non_exhaustive]` enum): `NotPng`, `BadChunkCrc`, `BadChunkType`, `BadChunkLength`, `MissingIhdr`, `BadIhdr`, `MissingIend`, `TooManyChunks`.

## Clarifications from the SPEC-028 build and verify (2026-11-21)

- **Issue order within one chunk:** bad type, then `MissingIhdr`, then `BadIhdr`, then CRC.
- **Issue cap:** the chunk whose own issue would exceed `MAX_PNG_ISSUES` is not recorded, so an issue offset can point at a chunk absent from `chunks()`; once the cap is full a halting issue is replaced by `TooManyIssues`, as in the JPEG walker. The shared `TooManyIssues` message is walk-neutral.
- **Chunk cap:** `TooManyChunks` fires on a further whole, well-formed chunk, so exactly 65,536 chunks is clean; a bad tail after 65,536 chunks gives `Truncated` or `BadChunkLength` instead.
- **`Truncated` length:** 12 when fewer than 12 bytes remain, otherwise the chunk's full size (12 + length).
- **Signature only:** `MissingIhdr` then `MissingIend`, both at offset 8.
- **`BadIhdr`** applies to any `IHDR` chunk; a second well-formed `IHDR` is clean.

## Validation

Revisit if ExifTool reads a class of real files past a point where the walk halts, if the 65,536 cap truncates real files in practice, or if the writer needs per-chunk provenance on issues.

## References

- Related specs: SPEC-028, SPEC-029
- Related decisions: DEC-001, DEC-002, DEC-006
- External: PNG Specification (W3C, Third Edition) chunk layout and CRC; ExifTool 13.55 behaviour measured in the SPEC-028 design
