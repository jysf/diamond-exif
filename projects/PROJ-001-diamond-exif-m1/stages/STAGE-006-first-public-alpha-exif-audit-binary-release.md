---
# Maps to ContextCore epic-level conventions.
# A Stage is a coherent chunk of work within a Project.
# It has a spec backlog and ships as a unit when the backlog is done.

stage:
  id: STAGE-006                     # stable, zero-padded, continuous across the repo
  status: proposed                  # proposed | active | shipped | cancelled | on_hold
  priority: medium                  # critical | high | medium | low
  target_complete: null             # optional: YYYY-MM-DD

project:
  id: PROJ-001                      # parent project
repo:
  id: diamond-exif

created_at: 2026-11-19
shipped_at: null

# What part of the project's value thesis this stage advances.
# If you can't articulate value_contribution, the stage may be
# infrastructure-only — acceptable but flag it.
value_contribution:
  advances: "The free exif-audit CLI, the open-core funnel in the project thesis, becomes something a stranger can download and run."
  delivers:
    - "A public GitHub pre-release (v0.1.0-alpha.1) with exif-audit binaries for Linux x86_64 (static), macOS arm64 and macOS x86_64, checksums and release notes"
    - "A tag-triggered, dry-runnable release workflow with least-privilege permissions"
  explicitly_does_not:
    - "publish exif-core to crates.io"
    - "sign or notarize binaries, or add a package-manager channel"
    - "add Windows or Linux arm64 builds"

# Orchestration cost — the spend that has no spec to attach to (roadmap:
# orchestration + framing cost attribution). Framing a stage, deciding the spec
# breakdown, and cross-spec steering all happen BEFORE/BETWEEN specs, so today
# they are invisible and recorded cost is systematically under-counted.
#
# THE ORCHESTRATOR FILLS THIS — not the human. At stage close, read your own
# session total (`/cost` in Claude Code; the `usage` object via API) and append
# one entry. Stage grain ONLY: do not try to split orchestration across specs —
# that is a division you cannot observe, so any per-spec number is invented.
# Warn-only, never a gate. A null here is honest; a guess is not. (DEC-013 §5)
orchestration_cost:
  sessions: []                      # - tokens_total: N
                                    #   estimated_usd: N
                                    #   recorded_at: YYYY-MM-DD
                                    #   notes: "framing + spec breakdown"
---

# STAGE-006: First public alpha: exif-audit binary release

## What This Stage Is

One release spec that turns the finished `exif-audit` CLI into a public GitHub pre-release: a version bump, a tag-triggered release workflow that builds three binaries and checksums, a `--version` that reports its build ref, an install section, and release notes. The irreversible step (the tag push, on a public repository) happens only on the user's explicit go-ahead.

## Why Now

`exif-audit` is complete and verified, the crates.io names are free, and the thesis depends on the free tool reaching people. Releasing the binary now, with the library unpublished, practises the release process on something fully reversible (delete the release and the tag) before the one-way library publish.

## Success Criteria

- A dry run of the workflow produces three archives and `SHA256SUMS` that pass their smoke checks, including on this Mac.
- After the cut, a stranger can download the asset, verify the checksum and run it; `exif-audit --version` shows the tag's version and commit.
- Rollback was exercised once.

## Specs

- SPEC-024: v0.1.0-alpha.1 exif-audit binary release (M).
