# diamond-exif

Panic-free, pure-Rust, zero-copy EXIF/XMP/IPTC/ICC metadata reader, writer and stripper for server-side workloads (Axum, Actix, Cloudflare Workers), verified differentially against ExifTool.

| Piece | Status | License |
|---|---|---|
| `exif-core` | scaffolded (empty, gated; M1a) | MIT/Apache-2.0 |
| `exif-audit` CLI (read-only scanner) | planned, first deliverable | free |
| `exif-pro` (compliance tier) | scaffold only | paid |

Status: crate scaffold only (SPEC-001); no parsing code yet. See `CLAUDE.md` and `AGENTS.md`. First decision: DEC-001 (tolerant-walk policy).

*This file replaces the template README when you run `just init` — rename or customize as needed.*

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
