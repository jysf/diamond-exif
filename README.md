# diamond-exif

Panic-free, pure-Rust, zero-copy EXIF/XMP/IPTC/ICC metadata reader, writer and stripper for server-side workloads (Axum, Actix, Cloudflare Workers), verified differentially against ExifTool.

| Piece | Status | License |
|---|---|---|
| `exif-core` | reader and read-only audit model (M1a/M1b); no writer yet | MIT/Apache-2.0 |
| `exif-audit` CLI (read-only scanner) | built (SPEC-016): `cargo build --release --bin exif-audit` | free |
| `exif-pro` (compliance tier) | scaffold only | paid |

Status: strict TIFF/IFD reader with a tolerant walk (a corrupt sub-IFD becomes a reported issue, not a failure) that reaches the Exif, GPS and Interop IFDs and labels each IFD by how it was reached, plus ASCII, signed-integer and float value decoders (strings are borrowed from the input), a panic-free JPEG segment walker (markers, APPn kinds by signature, scan region and trailer, damage reported as per-segment issues) whose `jpeg::read` hands each Exif segment to the same tolerant TIFF engine and locates (does not decode) the XMP, extended XMP, ICC, IPTC and JUMBF regions with a layered issue view, two fuzz targets, and an ExifTool read-parity oracle (live and replayed) over hand-built fixtures, shown able to fail by mutated-copy red-proofs (a fault injected into a throw-away copy of the library turns the comparison red); a pure-library audit model (`exif_core::audit`: container sniff, inventory of every tag and located region, PII findings by category; inventory parity against ExifTool over 178 distinct inputs) a strip-policy preview and three report formats (text, JSON, CSV), and a total entry budget that bounds memory on overlapping IFDs; and the `exif-audit` command line (below), a read-only privacy scan with exit codes a CI job can act on. See `CLAUDE.md` and `AGENTS.md`. First decision: DEC-001 (tolerant-walk policy).

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
      --json          Shorthand for --format json
      --csv           Shorthand for --format csv
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

Incomplete outranks findings, so a CI job never reads "1" as "everything else was fine". An unsupported file (a PNG, say) is never counted as clean: it is reported as `unsupported`, and it fails the run only under `--strict`.

**A CI gate** (fail the build when any file leaks a GPS position or a device serial; the default `--fail-on high`):

```bash
exif-audit --json uploads/ > exif-report.json   # exit 1: findings, 3: some file could not be read
```

**A CSV export** of every inventory entry for a spreadsheet, previewing what stripping all personal data would leave:

```bash
exif-audit --csv --inventory --policy all-pii photos/ > exif-inventory.csv
```

**Privacy default.** The report does not repeat the data it was run to find: `--values` is off by default, and when you turn it on each value excerpt is cut at 64 bytes. File names in the report are escaped, so a hostile name cannot break the JSON or CSV or smuggle control characters into a terminal. A symlink found inside a directory is reported and not followed; a FIFO or device is reported and never opened; hidden files are scanned.

## Workflow

This repo uses a spec-driven multi-agent workflow. Claude handles architecture and review; a separate implementer agent (Kilo Code, Factory Droids, AdaL, Cursor, etc.) handles implementation.

## Hierarchy

```
Repo (this app)
 └─ Project (a wave of work: "MVP", "v2 improvements")
     └─ Stage (a coherent chunk within a project)
         └─ Spec (an individual task)
              └─ Cycle (Frame → Design → Build → Verify → Ship)
```

## Getting started

**First time?** Read `GETTING_STARTED.md` — it walks you through your first project end-to-end.

**Daily work?** Run `just --list` to see available commands.

**Common commands:**
```bash
just status                        # See active project, stage, specs by cycle
just backlog                       # Spec-grained: what's next in the active stage
just roadmap                       # Stage-grained: where this project is going
just new-spec "title" STAGE-001    # Scaffold a new spec
just advance-cycle SPEC-001 verify # Update a spec's cycle
just archive-spec SPEC-001         # Move a shipped spec to done/
just review                        # Print the weekly review prompt
just report daily                  # Generate today's daily report
just report weekly                 # Generate this week's weekly report
just report status                 # Snapshot `just status` to reports/daily/<date>-status.md
```
`report-daily` / `report-weekly` remain as permanent aliases for
`report daily` / `report weekly`.

## Reports

`just report-daily` and `just report-weekly` generate quantitative
snapshots under `reports/daily/` and `reports/weekly/` from spec
front-matter and git log. Daily reports show specs by cycle, value
thesis, cost activity today, and flags. Weekly reports aggregate
ships, cycle times, cost by cycle and interface, and value
advancement. Reports are stand-alone artifacts — re-running
overwrites, so they're always a current snapshot.

## The app itself

Build and test commands are in `AGENTS.md` §6 (wrapped by `app.just`). The crate is scaffolded first; promotion to a workspace is planned for M2.

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
| `src/`, `tests/`, `fuzz/` | single crate for now (`exif-core`; `exif-audit` bin later); `exif-pro` and `site/` at M2 |

## License

`exif-core`: MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`). `exif-pro`: proprietary.
