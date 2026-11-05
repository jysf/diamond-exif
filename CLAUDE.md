# CLAUDE.md

`AGENTS.md` is the single source of truth for conventions — **read `/AGENTS.md` first, every session.**
Rules/constraints: `/guidance/constraints.yaml`. Rationale: `/decisions/`. Current work: `/projects/<active-project>/brief.md`. State: `just status`.

## What this is

diamond-exif: panic-free, pure-Rust, zero-copy EXIF/XMP/IPTC/ICC reader, writer and stripper, differentially verified against ExifTool. Single crate first (`src/`, `tests/`, `fuzz/` at the root, so irradiance's gates drop in); `exif-core` (MIT/Apache) with the free `exif-audit` CLI as a bin; `exif-pro` and `site/` at M2 when it becomes a workspace (own DEC).

## Governing documents

1. `docs/exif-core-design.md` — architecture, API, oracle strategy, milestones M1a–M2. **§16 (revisions from the porting brief) is read second and overrides earlier sections.**
2. `docs/porting-brief-diamond-exif.md` — irradiance's survey of what ports, with file/line citations.

These may not be in the repo yet. If a task depends on them and they are absent, ask the user for them; never reconstruct their contents.

## Non-negotiables (blocking quality bar)

- `#![forbid(unsafe_code)]` everywhere, plus a CI `-F unsafe_code` re-check; five-lint deny set; no panics on untrusted input.
- Tolerant walk (DEC-001): per-IFD issues, never whole-parse failure. DEC-001 is decided **before** porting any IFD code.
- Oracle-must-be-shown-red for every ExifTool parity test; ExifTool is never linked; `-T -n -s3`; skips are loud.
- A fuzz target per input surface from the first parser (60 s CI smoke; 24 h clean gates the milestone).
- Gate scripts are cloned from irradiance's `scripts/` (not the template — six scripts + `_lib.sh` fixes live only there).
- No I/O in the library, no pixel decoding, no runtime license checks/telemetry. Out-of-scope list: AGENTS.md §5.
- Port-and-extend, not greenfield: IFD engine ports from `irradiance/src/ifd.rs`; ASCII types, Exif/GPS/Interop pointer walk, tolerant-walk policy and the entire writer (EditPlan → apply) are net-new.

## Work order

Scaffold (single crate, gates from irradiance) → DEC-001 tolerant walk / DEC-002 `&[u8]` core, `Bytes` at the edge / DEC-003 provenance class → port the strict TIFF skeleton + 17 hostile-input tests → tolerant-walk policy → multi-IFD pointer walk + ASCII/signed/float decoders → oracle discipline → STAGE-003 `exif-audit` CLI (walk, inventory, PII findings, policy preview, `--json`/`--csv`, CI exit codes). Detailed port map: the extraction plan (to be copied into `docs/`).

## Workflow

Spec-driven, **claude-plus-agents** variant: Claude architects and reviews (frames, designs specs, writes handoffs, verifies PRs); a separate implementer agent builds from the handoff. Cycle: Frame → Design → Build → Verify → Ship; one spec per branch/PR; tests are written in design. Handoff documents under `projects/*/handoffs/` carry context between agents that share no memory, so they must be self-contained. Use `just` recipes (`just --list`); app commands live in `app.just`, never in the template-managed `justfile`. Open questions: `guidance/questions.yaml`. Toolchain facts for cold build agents: `guidance/toolchain-brief.md`.
