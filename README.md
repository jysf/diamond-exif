# diamond-exif

Panic-free, pure-Rust, zero-copy EXIF/XMP/IPTC/ICC metadata reader, writer and stripper for server-side workloads (Axum, Actix, Cloudflare Workers), verified differentially against ExifTool.

| Piece | Status | License |
|---|---|---|
| `exif-core` | reader, read-only audit model and a strip writer for JPEG, TIFF, DNG and PNG (M1a to M1c; PNG is library-only, see below); WebP is read and audited but not yet stripped; `set` operations not yet | MIT/Apache-2.0 |
| `exif-audit` CLI (read-only scanner) | released as a pre-release binary (`v0.1.0-alpha.1`, see Install); or `cargo build --release --bin exif-audit` | free |
| `exif-pro` (compliance tier) | scaffold only | paid |

What works today:

- **Reader:** strict TIFF/IFD parsing with a tolerant walk (a corrupt sub-IFD becomes a reported issue, not a failure). It reaches the Exif, GPS and Interop IFDs, labels each IFD by how it was reached, and decodes ASCII (borrowed from the input), signed-integer and float values. A total entry budget bounds memory on overlapping IFDs.
- **JPEG:** a panic-free segment walker (markers, APPn kinds by signature, scan region, trailer; damage becomes per-segment issues). `jpeg::read` hands each Exif segment to the same TIFF engine and locates, without decoding, the XMP, extended XMP, ICC, IPTC and JUMBF regions.
- **Audit model:** `exif_core::audit` does container sniffing, a tag and region inventory, PII findings by category and strip-policy previews. Text, JSON and CSV renderers and an incremental `ReportBuilder` produce the reports.
- **Strip writer:** `exif_core::strip` (and `sanitize`, which returns only the bytes) takes input bytes and a `StripPolicy` (`Gps`, `GpsAndDevice`, `AllPii`, `All`, `Everything`, or any set of categories) and returns new bytes or a typed refusal. It fails closed: the output is audited again and compared with the input's audit before it is returned, and it is never longer than the input.
  - **JPEG:** whole segments are removed; inside an Exif segment, matching IFD entries are deleted in place. Nothing grows or moves, and a Multi-Picture index stays consistent.
  - **TIFF and DNG:** the same in-place deletion, with scrubbed values zeroed. Strip and tile offsets, strip and tile data, SubIFDs and JPEG data stay byte-identical. When that cannot be guaranteed the writer refuses with a reason (a hidden pointer, shared bytes, or a thumbnail it cannot locate).
  - **PNG:** the library walks a PNG chunk by chunk (`exif_core::png`, tolerant: a bad CRC or a missing `IEND` is an issue, not a failure), and `exif_core::png::read` locates its metadata (`eXIf` Exif handed to the TIFF engine, XMP, ICC, raw-profile and text chunks, including chunks after `IEND`) without inflating or decoding anything. `exif-audit` audits PNG: every `eXIf` block is inventoried (none is merged: ExifTool reads only the first), and the located XMP, ICC, text, raw-profile and `tIME` chunks and any bytes after `IEND` become findings (text chunks by keyword: `Artist`, `Author` and `Copyright` are authorship, `Creation Time`, `create-date` and `modify-date` timestamps, `Software` software, the rest free text; a `tIME` chunk of any length is a timestamp, as ExifTool prints it for every length). A PNG raw profile (ImageMagick `Raw profile type ...`) can hold Exif with GPS that this library does not decode, so it makes the file `unreadable` (never `ok`, exit 3), besides its `other_segment` finding. The report says "located", never "valid", and the audit and CLI still report a PNG as **not strippable** (its preview keeps every finding and says so, `strippable: false` in JSON) until the ExifTool write-parity check (SPEC-039) shows the library's output agrees with ExifTool's.
  - **PNG strip (library only):** `strip` removes whole chunks (DEC-021): text chunks by keyword category, `iCCP`, `tIME`, `caBX`, and `eXIf` (all of it when the policy has every Exif category, none of it when it has no category an Exif entry can have, otherwise the matching entries are deleted in place and the chunk CRC is recomputed). Opaque carriers the library cannot read (ImageMagick `Raw profile type ...` text chunks and the `tXMP`, `meTa`, `seAl` chunks and `zxIf` or `eXIf` in any case other than the exact `eXIf` that ExifTool decodes) go with any tag category, XMP or IPTC, so with every preset, and with `OtherSegment`; `dSIG`-class chunks and the bytes after `IEND` go with `OtherSegment` (`Everything`). The output is never longer than the input, every kept chunk is byte-identical (a wrong CRC it already had included), and the writer refuses (typed, no bytes) when the chunk walk was halted, a chunk runs past the end of the input, an `eXIf` edit cannot be made exactly, or a removal would cut a kept chunk. Consequence of the carrier rule: a preset that keeps plain XMP (`Gps`) still removes a raw profile, which can hold the very Exif the preset asks to remove.
  - **WebP:** the library walks a WebP RIFF stream chunk by chunk (`exif_core::webp`, tolerant: the declared RIFF size never limits the walk, and size, pad-byte and `VP8X` flag disagreements are issues, not failures), and `exif_core::webp::read` locates its metadata (`EXIF` handed to the TIFF engine, `XMP ` and `ICCP` located, animation frames passed over). `exif-audit` audits WebP the same way (every `EXIF` block inventoried, `XMP ` and `ICCP` located; a file with more `EXIF` blocks than the library keeps is never reported clean: a WebP whose only GPS is in the 17th block reads `issues`, which exits 0 without `--strict`, because the 17th block is not listed and ExifTool prints a GPS the audit cannot; run with `--strict` where that matters) and likewise reports it **not strippable**. HEIC, GIF, BigTIFF, empty and unknown files stay `unsupported`.
  - **Known limits:** compressed and raw-profile PNG metadata is located but not read, and WebP metadata is located, never decoded.
- **Verification:** ten fuzz targets (`tiff_ifd`, `jpeg_segments`, `audit`, `audit_render`, `cli_args`, `rewrite_jpeg`, `rewrite_tiff`, `rewrite_png`, `png_chunks`, `webp_riff`) and an ExifTool oracle for reads, inventory and strip output (live and replayed), shown able to fail by mutated-copy red-proofs. Inventory parity against ExifTool covers 183 distinct JPEG and TIFF inputs and 364 distinct PNG and WebP inputs (Exif entries as a multiset of group and tag id, XMP and ICC as presence, PNG text and time tags as a count, every difference pinned with a measured cause); PNG and WebP are also checked for category presence (a GPS, XMP or ICC finding exactly when ExifTool prints the matching tag, except the recorded differences) and for the tag name ExifTool gives each PNG text keyword.
- **CLI:** `exif-audit`, a read-only privacy scan with exit codes a CI job can act on (below).

See `CLAUDE.md` and `AGENTS.md`. First decision: DEC-001 (tolerant-walk policy).

## Install

`exif-audit` is a single binary with no runtime dependencies. Once a release has been published, download the archive for your platform from the [Releases page](https://github.com/jysf/diamond-exif/releases) (version `0.1.0-alpha.1`; Linux x86_64 (static musl), macOS Apple silicon (`aarch64-apple-darwin`) and macOS Intel (`x86_64-apple-darwin`)). It is an alpha: the CLI contract (DEC-011) is intended but not frozen.

```bash
# Pick your target: x86_64-unknown-linux-musl, aarch64-apple-darwin or x86_64-apple-darwin
TARGET=aarch64-apple-darwin
VERSION=0.1.0-alpha.1
gh release download "v${VERSION}" --repo jysf/diamond-exif --pattern "exif-audit-${VERSION}-${TARGET}.tar.gz" --pattern SHA256SUMS
# or: curl -LO https://github.com/jysf/diamond-exif/releases/download/v${VERSION}/exif-audit-${VERSION}-${TARGET}.tar.gz  (and the same URL ending in SHA256SUMS)

# Verify. SHA256SUMS lists all three archives, so check only the line for the one you downloaded
# (if you downloaded all three, plain `shasum -a 256 -c SHA256SUMS` checks them together):
grep "${TARGET}" SHA256SUMS | shasum -a 256 -c -
# where `shasum` is missing (minimal Linux images): grep "${TARGET}" SHA256SUMS | sha256sum -c -

mkdir exif-audit-bin && tar -xzf "exif-audit-${VERSION}-${TARGET}.tar.gz" -C exif-audit-bin
./exif-audit-bin/exif-audit --version
```

**macOS and the quarantine flag.** The binary is not signed or notarized. A file downloaded with `gh` or `curl` runs without a prompt; one downloaded in a browser is quarantined by Gatekeeper and refused. Clear it once with:

```bash
xattr -d com.apple.quarantine ./exif-audit-bin/exif-audit
```

(`xattr` reports an error if the attribute is absent; that is harmless.)

**First run.** `./exif-audit-bin/exif-audit --help`, then for example `./exif-audit-bin/exif-audit path/to/photos/` (exit 1 means findings, 3 means some input could not be read; see the next section).

What to expect, plainly: the CLI never writes or modifies a file. The strip writer is library-only and is not part of this release. The library (`exif-core`) is not published to crates.io (`publish = false`). The binary is unsigned and there is no Windows or Linux arm64 build yet. To uninstall, delete the folder; nothing else is installed.

## Using exif-audit

`exif-audit` scans files and directories for metadata that can identify a person or a device (GPS, device serial numbers, authorship, free text, thumbnails, MakerNotes, XMP, IPTC, ICC) and prints one report on stdout. It never writes or modifies a file, reads nothing but the paths you name, and keeps no state.

```bash
cargo build --release --bin exif-audit
./target/release/exif-audit photos/
```

```text
Usage: exif-audit [OPTIONS] [--] PATH...

Scan files and directories for metadata that can identify a person or a
device. Prints one report on stdout and exits with a code a CI job can act on.
Reads nothing but the named paths; writes nothing but the report.

Options:
      --format <F>    Report format: text, json or csv [default: text]
      --json          Shorthand for --format json [default: off]
      --csv           Shorthand for --format csv [default: off]
      --policy <P>    Strip policy the report previews: gps, gps-and-device,
                      all-pii, all, everything, or none for no preview
                      [default: all-pii]
      --fail-on <S>   Exit 1 when a file has a finding of at least this
                      severity: none, low, medium or high [default: high]
      --strict        Count unsupported files and files with issues as
                      incomplete (exit 3) [default: off]
      --values        Include a raw value excerpt of each tag [default: off]
      --inventory     List every inventory entry, not only findings
                      [default: off]
      --max-size <N>  Largest file read: bytes with an optional K, M or G
                      suffix (powers of 1024), at least 1 [default: 256M]
  -h, --help          Print this help and exit 0
  -V, --version       Print the version and exit 0

Options take their value as the next argument or as --name=value. Each option
may be given once. A lone - is an error (stdin is not read). After -- every
argument is a path. Directories are walked recursively in name order; a symlink
found in a walk is reported, not followed.

Exit codes:
  0  complete, and no file reaches the --fail-on threshold
  1  complete, and a file has a finding at or above the threshold
  2  usage error
  3  incomplete: a file failed or was unreadable (with --strict, also
     unsupported or with issues)
  4  the report could not be written
```

**Exit codes.** Exactly one is returned, in this order of precedence: `2` usage error, `4` report not written, `3` incomplete, `1` findings at or above `--fail-on`, `0` otherwise.

| Code | Meaning |
|---|---|
| `0` | complete, and no file reaches the `--fail-on` threshold (also: no files, and `--fail-on none`) |
| `1` | complete, and at least one file has a finding of at least `--fail-on` severity |
| `2` | usage error (nothing was scanned; message on stderr, stdout empty) |
| `3` | incomplete: a file failed or was unreadable (with `--strict`, also `unsupported` or `issues`) |
| `4` | the report could not be written to stdout (a closed pipe, a full disk) |

Incomplete outranks findings, so a CI job never reads "1" as "everything else was fine". An unsupported file (a HEIC, say) is never counted as clean: it is reported as `unsupported`, and it fails the run only under `--strict`.

**A CI gate** (fail the build when any file leaks a GPS position or a device serial; the default `--fail-on high`):

```bash
exif-audit --json uploads/ > exif-report.json   # exit 1: findings, 3: some file could not be read
```

**A CSV export** of every inventory entry for a spreadsheet, previewing what stripping all personal data would leave:

```bash
exif-audit --csv --inventory --policy all-pii photos/ > exif-inventory.csv
```

**Privacy default.** The report does not repeat the data it was run to find: `--values` is off by default, and when you turn it on each value excerpt is cut at 64 bytes. File names in the report are escaped, so a hostile name cannot break the JSON or CSV or smuggle control characters into a terminal. A symlink found inside a directory is reported and not followed; a FIFO or device is reported and never opened; hidden files are scanned.

## Build and test

App commands live in `app.just` (details in `AGENTS.md` §6):

```bash
just build              # cargo build
just test               # cargo test (oracle tests skip LOUDLY without exiftool)
just lint               # fmt check + clippy
just deny               # license policy (permissive-only dependencies)
just forbid-unsafe      # re-check that no unsafe code exists
just fuzz-smoke [TARGET]  # 60 s smoke per fuzz target (tiff_ifd, jpeg_segments, rewrite_jpeg, rewrite_tiff, rewrite_png, audit, audit_render, cli_args, png_chunks, webp_riff)
just oracle-record      # re-record ExifTool output for the parity fixtures
just mutation-oracle    # mutated-copy red-proofs against the committed ExifTool records
```

Run `just --list` for the full set, including the lint and gate red-proofs.

## Process

This repo is built with a spec-driven, multi-agent workflow (Claude architects and reviews, a separate implementer agent builds from handoffs). The workflow commands (`just status`, `just new-spec`, ...) come from [spec-driven-template](https://github.com/jysf/spec-driven-template), which this repo was scaffolded from (`VERSION` records the template version); they are not documented here.

- `AGENTS.md`: conventions, constraints and the build/test commands.
- `docs/PLAYBOOK.md` and `docs/USAGE.md`: the workflow and its daily loop.
- `projects/PROJ-001-diamond-exif-m1/brief.md`: the current project; `just status` shows where it stands.

## Where things live

| Path | Purpose |
|---|---|
| `AGENTS.md` | Conventions for agents working in this repo |
| `.repo-context.yaml` | Structured metadata about the app |
| `docs/` | Architecture, data model, API contract |
| `guidance/` | Repo-level rules, open questions, and the signals ledger (`just dash signals`) |
| `decisions/` | Decision log (accumulates across projects) |
| `projects/` | Each project (wave of work) lives here |
| `projects/*/brief.md` | What this project is and why |
| `projects/*/stages/` | Stages within a project |
| `projects/*/specs/` | Specs within a project |
| `projects/*/handoffs/` | Architect → implementer delegation records |
| `src/`, `tests/`, `fuzz/` | single crate for now (`exif-core` with the `exif-audit` bin); `exif-pro` and `site/` at M2 |

## License

`exif-core`: MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`). `exif-pro`: proprietary.
