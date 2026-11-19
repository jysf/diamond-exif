---
# Maps to ContextCore epic-level conventions.
# A Stage is a coherent chunk of work within a Project.
# It has a spec backlog and ships as a unit when the backlog is done.

stage:
  id: STAGE-005                     # stable, zero-padded, continuous across the repo
  status: shipped                   # proposed | active | shipped | cancelled | on_hold
  priority: medium                  # critical | high | medium | low
  target_complete: null             # optional: YYYY-MM-DD

project:
  id: PROJ-001                      # parent project
repo:
  id: diamond-exif

created_at: 2026-11-19
shipped_at: 2026-11-19

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
  sessions:
    - tokens_total: 208066
      estimated_usd: 1.37
      recorded_at: 2026-11-19
      notes: "Measured by the orchestrator from this session's transcript (c6de7258) for messages from 2026-11-19T14:47Z (the DEC-008 spec request) to 15:23Z (28 de-duplicated messages): input + output + cache_creation, cache reads excluded, 6.60 USD/Mtok. Covers framing and design of STAGE-005 and SPEC-023, the two handoffs, independent gate re-runs and routing of the build and verify. The ship turns after the read are not included."                      # - tokens_total: N
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

## Stage-Level Reflection

- **Did we deliver the outcome in "What This Stage Is"?** Yes. Both DEC-008 gaps are closed: the four DNG identity tags are categorised and removed by `AllPii` and `Everything` (ExifTool agrees), and multi-page and tiled-page TIFFs strip under the three presets that include `Thumbnail`, page data byte-identical. Differential: 1,075 runs over 215 inputs, 0 violations.
- **How did the spec count compare with the plan?** One spec framed, one shipped, no splits, no additions.
- **What changed from the original plan?** Two edits the design did not list were needed: the `verify_tiff` entries self-check, and the independent fuzz model's IFD1 rule. Verify judged both sound.
- **Cost.** SPEC-023: 353,608 tokens, 2.34 USD (build 173,750, verify 179,858; one verify round, approved with three Lows), against a 600,000 estimate and size M: 41 percent under. Orchestration 208,066 tokens, 1.37 USD. Stage total 561,674 tokens, 3.71 USD. The earlier pattern of M specs running over did not repeat: the spec was small and its design named the code sites.
- **What carries forward?** Two Low test gaps (a page IFD1 holding identity entries, and an audit fuzz seed with the four DNG tags) for a patch; the PNG and WebP stage reuses the same category table and the container-aware rule.
