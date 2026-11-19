---
# Maps to ContextCore epic-level conventions.
# A Stage is a coherent chunk of work within a Project.
# It has a spec backlog and ships as a unit when the backlog is done.

stage:
  id: STAGE-005                     # stable, zero-padded, continuous across the repo
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
  advances: "The strip presets keep their promise on DNG and multi-page TIFF, so the parity and compliance claims hold on the file kinds the thesis names."
  delivers:
    - "Everything and AllPii remove every DNG identity tag"
    - "Multi-page and tiled-page TIFFs can be stripped under AllPii, All and Everything"
  explicitly_does_not:
    - "add PNG, WebP or HEIC support"
    - "add set operations or edit MakerNotes"

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

# STAGE-005: DEC-008 audit gaps: DNG identity tags and TIFF pages

## What This Stage Is

A one-spec stage that closes the two writer gaps the SPEC-022 verify found and the 2026-11-11 decisions settled: DNG identity tags get DEC-008 categories, and the audit's thumbnail rule becomes container-aware so multi-page and tiled TIFFs strip under the presets that include `Thumbnail`.

## Why Now

They are the only known correctness limits in the shipped writer, they change what the audit calls a thumbnail or an identity tag, and the PNG and WebP stage reuses the same audit and category table. Fixing the table before a third container arrives avoids reworking it twice.

## Success Criteria

- `Everything` and `AllPii` leave no DNG identity tag, with ExifTool agreeing.
- A multi-page and a tiled-page TIFF strip under `AllPii`, `All` and `Everything`, pages byte-identical.
- The two questions are answered with this stage named, the README known-limits bullet is gone, and DEC-008 and DEC-013 D.3 carry the amendments.

## Specs

- SPEC-023: DEC-008 amendment: DNG identity tag categories and a container-aware TIFF page rule (M).
