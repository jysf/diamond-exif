---
# Maps to ContextCore epic-level conventions.
# A Stage is a coherent chunk of work within a Project.
# It has a spec backlog and ships as a unit when the backlog is done.

stage:
  id: STAGE-006                     # stable, zero-padded, continuous across the repo
  status: shipped                   # proposed | active | shipped | cancelled | on_hold
  priority: medium                  # critical | high | medium | low
  target_complete: null             # optional: YYYY-MM-DD

project:
  id: PROJ-001                      # parent project
repo:
  id: diamond-exif

created_at: 2026-11-19
shipped_at: 2026-11-21

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
  sessions:
    - tokens_total: 465293
      estimated_usd: 3.07
      recorded_at: 2026-11-21
      notes: "Measured by the orchestrator from this session's transcript (c6de7258), messages from the request to frame the alpha release (2026-11-19T17:02Z) to the post-cut verification (2026-11-21T06:24Z), 62 de-duplicated messages: input + output + cache_creation, cache reads excluded, 6.60 USD/Mtok. Covers framing and design, three handoffs, independent gate and workflow reviews, routing of two builder follow-ups and two verifies, the merge conflict and DEC renumbering, the dry run and the cut. The ship turns after the read are not included."                      # - tokens_total: N
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

## Stage-Level Reflection

- **Did we deliver the outcome in "What This Stage Is"?** Yes. `v0.1.0-alpha.1` is a public GitHub pre-release with `exif-audit` for Linux x86_64 (static), macOS arm64 and macOS Intel, `SHA256SUMS` and release notes, built by a tag-triggered, dry-runnable, least-privilege workflow. A curl download passes its checksum and runs.
- **How did the spec count compare with the plan?** One release spec framed, one shipped.
- **What changed from the original plan?** The dry run could not be dispatched before merge (GitHub only dispatches workflows on the default branch), so it ran after PR #38 merged. The throw-away-tag rollback rehearsal was dropped: the workflow's own tag-equals-version gate makes it impossible. The decision number moved from DEC-014 to DEC-016 at merge.
- **Cost.** SPEC-024: 721,152 tokens, 4.76 USD (build 451,180 with two follow-ups; verify 133,171 and 136,801) against a 450,000 estimate: 60 percent over. Orchestration 465,293 tokens, 3.07 USD. Stage total 1,186,445 tokens, 7.83 USD. The overrun is two verify rounds, one High and a builder resumed twice.
- **What carries forward?** A tag ruleset for `v*` (maintainer's setting), Linux clean-host and real-Intel checks of the published binaries, then the library publish as its own release spec after the 24 h fuzz gate and a real-camera corpus. STAGE-007 (relocatable Exif block) is next in the plan from another session.
