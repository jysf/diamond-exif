# diamond-exif

Panic-free, pure-Rust, zero-copy EXIF/XMP/IPTC/ICC metadata reader, writer and stripper for server-side workloads (Axum, Actix, Cloudflare Workers), verified differentially against ExifTool.

| Piece | Status | License |
|---|---|---|
| `exif-core` | reader and read-only audit model (M1a/M1b); no writer yet | MIT/Apache-2.0 |
| `exif-audit` CLI (read-only scanner) | library side done (policy presets, text/JSON/CSV renderers, `ReportBuilder`); the binary is next (SPEC-016) | free |
| `exif-pro` (compliance tier) | scaffold only | paid |

What works today:

- **Reader:** strict TIFF/IFD parsing with a tolerant walk (a corrupt sub-IFD becomes a reported issue, not a failure). It reaches the Exif, GPS and Interop IFDs, labels each IFD by how it was reached, and decodes ASCII (borrowed from the input), signed-integer and float values. A total entry budget bounds memory on overlapping IFDs.
- **JPEG:** a panic-free segment walker (markers, APPn kinds by signature, scan region, trailer; damage becomes per-segment issues). `jpeg::read` hands each Exif segment to the same TIFF engine and locates, without decoding, the XMP, extended XMP, ICC, IPTC and JUMBF regions.
- **Audit model:** `exif_core::audit` does container sniffing, a tag and region inventory, PII findings by category and strip-policy previews. Text, JSON and CSV renderers and an incremental `ReportBuilder` produce the reports.
- **Verification:** four fuzz targets and an ExifTool read-parity oracle (live and replayed), shown able to fail by mutated-copy red-proofs. Inventory parity against ExifTool covers 178 distinct inputs.

See `CLAUDE.md` and `AGENTS.md`. First decision: DEC-001 (tolerant-walk policy).

## Build and test

App commands live in `app.just` (details in `AGENTS.md` §6):

```bash
just build              # cargo build
just test               # cargo test (oracle tests skip LOUDLY without exiftool)
just lint               # fmt check + clippy
just deny               # license policy (permissive-only dependencies)
just forbid-unsafe      # re-check that no unsafe code exists
just fuzz-smoke [TARGET]  # 60 s smoke per fuzz target (tiff_ifd, jpeg_segments, audit, audit_render)
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
| `src/`, `tests/`, `fuzz/` | single crate for now (`exif-core`; `exif-audit` bin later); `exif-pro` and `site/` at M2 |

## License

`exif-core`: MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`). `exif-pro`: proprietary.
