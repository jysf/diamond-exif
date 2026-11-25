---
insight:
  id: DEC-023
  type: decision
  confidence: 0.8
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

created_at: 2026-11-24
supersedes: null
superseded_by: null
status: accepted
deciders: [jysf, claude]
affected_scope:
  - "tests/strip_png_webp_oracle.rs"
  - "tests/strip_png_sweep_live.rs"
  - "tests/support/i3_*.rs"
  - "tests/support/sweep.rs"
  - "tests/oracle-fixtures/exiftool-strip-png/**"
  - "tests/oracle-fixtures/exiftool-strip-webp/**"
  - "examples/oracle_record.rs"

tags:
  - oracle
  - write-parity
  - png
  - webp
  - exiftool
---

# DEC-023: PNG and WebP write parity (I3): a category judge, a differential against ExifTool's own writer, and the write-mode exit codes

## Decision

PNG and WebP strip output is trusted (I3) through two layers over committed, replayable ExifTool records. **Layer A, the judge:** every fixture under every recorded policy, ExifTool's reading of our output against its reading of the input, by category, in both directions. **Layer B, the differential** (the design document's I3, §8): for the two presets ExifTool can express, ours and ExifTool's own writer are run on the same input and ExifTool's readings of the two outputs must agree, apart from a pinned table of measured differences. ExifTool's write-mode exit status is a signal and is checked, unlike the read side.

1. **Records.** One record file per fixture under `tests/oracle-fixtures/exiftool-strip-png/` and `exiftool-strip-webp/`, written by `just oracle-record` (`examples/oracle_record.rs`), in the format of the JPEG strip records (`# command`, `# exiftool-version`, `# fixture`, then `## <section>` blocks of `-a -u -G1 -s -t -n` lines with the `File`, `System` and `ExifTool` groups dropped). The sections are `## input`, one `## <policy>` per recorded policy that strips, and for Layer B one `## exiftool:<op>` per ExifTool write (`exiftool:all`, `exiftool:gps:all`). A write that ExifTool refused (exit 1, rule 5) is still a section: it holds a single `# refused: <first Error: line, machine paths stripped>` line and no tag lines, so replay sees ExifTool's outcome as well as its answer. Each `## <policy>` section carries the length and a 64-bit FNV-1a digest of the output it was read from (`# output: <len> <fnv1a64-hex>`). Replay recomputes `strip` today and fails if the bytes no longer match the digest, so a record can never vouch for bytes the writer no longer produces. The live tests re-run ExifTool and must reproduce the committed text byte for byte (same version); CI sets `DIAMOND_EXIF_REQUIRE_ORACLE=1`.
2. **Recorded policies.** The five presets, plus `Custom` rows that isolate a category: `xmp`, `icc`, `jumbf`, `rights`, `timestamps`, `free_text`, `other` (`OtherSegment`). A policy whose strip is refused has no section; the refusal is pinned in the record list (fixture, policy, `StripError` variant), and an unpinned refusal or an unpinned success fails.
3. **Layer A, both directions.** A tag is classified into a category by a table typed in `tests/support/i3_judge.rs` from DEC-008, DEC-020 (with its 2026-11-24 amendment) and DEC-022, never read from the library (the WebP list now in `tests/strip_webp_live.rs` moves there and gains the PNG rows: `PNG:Artist`, `PNG:Author` Authorship; `PNG:Copyright` Rights; `PNG:CreationTime`, `PNG:CreateDate`, `PNG:ModDate`, `PNG:TimeStamp`, `PNG:ModifyDate` and `Trailer:ModifyDate` Timestamps; `PNG:Software` Software; `PNG:Make`, `PNG:Model` CameraModel; every other `PNG` text tag FreeText; `PNG:ICC_Profile`, `PNG:ProfileName` Icc; `JUMBF` and `JSON` groups Jumbf). For each policy: every tag of a removed category is gone, and every input tag of a kept category is still printed with the same value. Three exceptions, each tied to the rule that causes it and checked to apply only where that rule fires:
   - (a) **the carrier rule** (DEC-021 clause 4, DEC-022 clause 4): every tag a carrier chunk prints is gone under every preset. That includes the kept-category tags inside it (measured: `ExifIFD:ISO`, `IFD0:Copyright` and `IFD0:Orientation` from a raw-profile Exif under `Gps`; `XMP-xmp:CreatorTool` from a raw-profile XMP);
   - (b) **the whole-block rule:** a policy that holds every category an Exif entry can have (`All`, `Everything`) removes the whole `eXIf`/`EXIF` chunk, so the uncategorised Exif tags (`Orientation`, `ISO`, `XResolution`, ...) go with it;
   - (c) **the stated residuals:** `Composite:GPS*` derived from XMP survives any preset that keeps `Xmp`, and `JUMBF`/`C2PA` contents survive any preset that keeps `Jumbf`. These count as kept, not as removal failures.
   - No other exception exists. A file that prints no removable tag before the strip is not counted toward any "removed" assertion, so a pass can't be vacuous.
4. **Layer B, the pairs:** `Everything` ↔ `exiftool -all= -o <out> <in>`, and `Gps` ↔ `exiftool -gps:all= -o <out> <in>`. No `-m` (a `[minor]` error must stay visible). The comparison is between the two outputs' readings, as a multiset of `(group, tag)` plus values for the tags both keep. The measured difference classes (ExifTool 13.55, the 174 `oracle_png` + `oracle_webp` fixtures, 2026-11-24) are pinned per fixture, each with its class:
   - **D1 — ExifTool drops `VP8X`.** After `-all=`, ExifTool removes the `VP8X` chunk itself, so `RIFF:WebP_Flags`, `RIFF:ImageWidth`, `RIFF:ImageHeight`, `Composite:ImageSize` and `Composite:Megapixels` are absent from its output. We keep `VP8X` with its flag bits cleared (DEC-022 clause 7). 72 of the 73 `Everything` differences (the 73rd is D5).
   - **D2 — ExifTool adds `VP8X`.** Writing a WebP that has `EXIF` and no (or a 0-byte) `VP8X`, ExifTool adds a `VP8X`. We never add a chunk. Fixtures: `webp-exif-no-vp8x`, `webp-vp8x-len-0` under `Gps`.
   - **D3 — carriers.** ExifTool keeps a raw-profile text chunk and rewrites the Exif or XMP inside it. We remove the chunk (exception (a)). Fixtures: `png-raw-exif-{text,ztxt,itxt}`, `png-raw-xmp-{text,ztxt,itxt}`, `png-exif-and-raw-exif` under `Gps`.
   - **D4 — duplicate `EXIF` chunks.** ExifTool's writer rewrites every `EXIF` chunk from the one it reads (the last). That changes kept values: `IFD0:Make AlphaCam` becomes `BravoCam`. We keep each chunk byte for byte apart from the entry edit. Fixtures: `webp-exif-two-same-tag`, `-reversed`, `webp-exif-good-then-prefixed`, `webp-exif-make-short-later` under `Gps`.
   - **D5 — a text chunk without a keyword separator.** ExifTool can't delete it and keeps `PNG:TextualData`. We remove it as FreeText. Fixture: `png-text-no-nul` under `Everything`.
   - **Outcome differences, pinned:** ExifTool refuses (exit 1) where we strip: a bad CRC, no `IEND`, truncation, a null chunk, an odd chunk without its pad byte, trailing bytes, a short `VP8X`, a partial header, and the "IFD0 pointer references previous IFD0" `[minor]` error on two `eXIf` chunks. We refuse where ExifTool writes: Exif blocks the walk cannot trust (`Uneditable { BlockNotClean }`: empty, garbage, fatal-then-good, not TIFF, a 17th block). Both refusing is agreement.
   - An unpinned difference fails. So does a pinned difference that no longer occurs: the pin is stale and must be removed.
5. **Write-mode exit codes (answers question `exiftool-write-exit-codes`).** Measured on ExifTool 13.55, 348 write runs plus probes:
   - `0`: the output file was written. Stdout says `1 image files created` (or `copied` when nothing changed). Stderr may carry `Warning:` lines.
   - `1`: an error, and no output file. Stderr's first `Error:` line gives the cause (format error, truncated chunk, a `[minor]` error without `-m`, an output path that already exists, a missing input).
   - `2`: a `-if` condition failed (not used here).
   - In all 348 runs, `rc == 0` held exactly when the output file existed.
   - The harness asserts that equivalence on every run. A run that breaks it fails as `ToolError`, never a skip. Exit 1 records the `Error:` line, which becomes a pinned outcome difference. `-m` is never passed. The output path is always fresh, because an existing one is exit 1.
   - The read side keeps its rule: only stdout is trusted, and the exit status is not checked.
6. **The PNG type sweep and its residual.** The sweep takes its candidates from ExifTool, as the WebP one does: every four-character `-listx` id, every case spelling of the writer's codes, and a pinned list of subdirectory types that `-listx` does not print. That pinned list is the residual: a PNG chunk type ExifTool decodes only through a subdirectory, absent from both `-listx` and the pinned list, is not found. The PNG `TextualData` keyword table is also swept. It is the only place `-listx` names text keywords.

## Context

STAGE-010's success criterion for I3 reads: "the stripped file read by ExifTool prints none of what the policy removed and everything else unchanged, comparator red-proofed, writer mutated-copy red-proofed, every divergence reported". The design document's I3 (§8) is stronger: apply the same op with both tools and compare their readings. No test in the repo has run ExifTool in write mode until now (the JPEG and TIFF strip oracles compare against declared tables), and the porting brief and design §16 both flag the write-mode exit codes as unmeasured. The SPEC-037 and SPEC-038 live tests (`strip_png_live.rs`, `strip_webp_live.rs`) are live-only, cover a subset of fixtures and presets, and were explicitly "the cheap precursor" of this record.

Measured in the SPEC-039 design (ExifTool 13.55, 2026-11-24):

- The differential over the 174 fixtures gave 191 identical pairs and 86 differing pairs (73 `Everything`, 13 `Gps`), every one in classes D1 to D5, plus 41 outcome differences and 30 pairs where both refuse.
- No fixture is refused while its audit is clean, under any preset. Every refusal falls on a file whose preview already says "not reliable".
- The PNG type sweep covered 2,622 `-listx` ids and 880 case spellings of 56 PNG types, in 10 payload shapes, 35,020 files in two ExifTool runs. Every type that printed a tag is known to the writer or is a structure chunk listed in DEC-020 clarification 4 (`PLTE` is added to that list).
- The keyword table has `Make`, `Model` and `TimeStamp`, which the audit filed under `FreeText` (DEC-020 amendment of 2026-11-24).

## Alternatives considered

- **The judge alone, as the JPEG oracle does.** Rejected for these formats. A judge typed from the same DECs as the writer can only find mistakes the DECs don't share. The differential asks ExifTool's writer, an independent implementation, and it is the I3 the design document defined.
- **The differential alone.** Rejected. It covers only the two presets ExifTool can express. Its difference table (D1 to D5) needs a second check that our side of each difference is the right one, and the judge is that check.
- **Live-only records, as SPEC-037 and SPEC-038 did.** Rejected. Replay with an output digest binds each record to the bytes the writer makes today. CI's live re-record catches a change in ExifTool's answer. A writer mutant shows red without ExifTool installed.
- **`-m` on ExifTool writes.** Rejected. It turns the `[minor]` refusals into silent success, and those refusals are part of what is compared.

## Consequences

- **Positive:** the switch-over (`ContainerKind::strippable()` true for PNG and WebP, DEC-020 amendment of 2026-11-24) rests on a record that reproduces, ExifTool agrees with it, and a red-proof shows it can fail.
- **Negative:** about 130 new record files, and the pinned difference table must be maintained when ExifTool's writer changes: a new ExifTool version means a re-record and a review of D1 to D5.
- **Neutral:** `strip_png_live.rs` and `strip_webp_live.rs` stay as written (the outline said extend, not rewrite). Their per-category lists move to `tests/support/i3_judge.rs` and they import it.

## Validation

The comparator red-proofs and the mutated-copy writer red-proofs of SPEC-039 (AC9, AC10). Revisit if a new ExifTool version adds a difference class, or if a writer change needs a new exception in rule 3. Any new exception is a DEC amendment, not a test edit.

## References

- Related specs: SPEC-039; SPEC-020, SPEC-021 (the JPEG strip oracle), SPEC-037, SPEC-038
- Related decisions: DEC-008, DEC-009, DEC-013, DEC-020, DEC-021, DEC-022
- `docs/exif-core-design.md` §8 (I3) and §16; `docs/porting-brief-diamond-exif.md` (the write-side warning)
