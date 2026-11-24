---
# Maps to ContextCore epic-level conventions.
# A Stage is a coherent chunk of work within a Project.
# It has a spec backlog and ships as a unit when the backlog is done.

stage:
  id: STAGE-011                     # stable, zero-padded, continuous across the repo
  status: shipped                   # proposed | active | shipped | cancelled | on_hold
  priority: medium                  # critical | high | medium | low
  target_complete: null             # optional: YYYY-MM-DD

project:
  id: PROJ-001                      # parent project
repo:
  id: diamond-exif

created_at: 2026-11-23
shipped_at: 2026-11-24

# What part of the project's value thesis this stage advances.
# If you can't articulate value_contribution, the stage may be
# infrastructure-only — acceptable but flag it.
value_contribution:
  advances: "The privacy guarantee: what the audit reports and Everything removes is everything a reader can recover from the file."
  delivers:
    - "A JFIF APP0 thumbnail is reported by the audit and removed under policies that target thumbnails"
  explicitly_does_not:
    - "Fix the writer's hardening defects from the same review (those are PATCH-001, project-level, no stage)"
    - "Add new containers or metadata kinds beyond what the review found"


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
    - tokens_total: 36753906
      estimated_usd: null
      recorded_at: 2026-11-24
      notes: "Measured by the orchestrator from this session's transcript (203cb371), live at ship, so a floor (docs/cost-tracking.md rule 6): 191 messages, each counted once with its largest output, model claude-opus-5-5, sidechains excluded; input 420, cache creation 936907, cache read 35702610, output 113969 (input + cache creation + cache read + output). The same session also orchestrated PATCH-001 (review triage, both handoffs, the fix round, the port onto main and its ship), which has no stage to carry it; the split between the two is not observable, so the whole total is recorded here. estimated_usd is null: Opus 5.5 list rates are not on record in this repo."
                                    # - tokens_total: N
                                    #   estimated_usd: N
                                    #   recorded_at: YYYY-MM-DD
                                    #   notes: "framing + spec breakdown"
---

# STAGE-011: M1 hardening: external review findings

## What This Stage Is

Closes the gaps an external code review (ChatGPT, against `fd94d4f`,
2026-11-23) found in the shipped M1 audit and strip, where the fix needs a
contract change (a DEC amendment) and so can't go through the patch lane. When
this stage ships, no structure that a reader like ExifTool extracts as
thumbnail or metadata is left unreported by the audit or kept by `Everything`.

## Why Now

The review found that a JFIF APP0 thumbnail gives `clean=true` with zero
findings, and that `strip(Everything)` returns the input unchanged while
ExifTool extracts the thumbnail. That breaks the product's core promise.
DEC-012 itself lists JFIF as benign, so this is a decision gap, not a coding
slip. It has to be amended before any code changes. The review's three
writer-hardening findings were shipped-behavior bugs and shipped separately
as PATCH-001 (PR #59, merged 2026-11-24).

## Success Criteria

- DEC-012 is amended: an APP0 `JFIF\0` segment with a non-zero thumbnail is
  no longer benign, and one with a zero thumbnail still is.
- The audit reports a JFIF thumbnail as a `Thumbnail` region and finding.
- Strip policies that target thumbnails (including `Everything`) remove or
  neutralise it, and the self-check confirms it.
- ExifTool parity: a red proof shows the oracle sees the thumbnail before the
  strip and not after.

## Scope

### In scope
- The JFIF APP0 thumbnail (the inline uncompressed RGB `Xthumbnail ×
  Ythumbnail` payload), in the audit and the JPEG strip.

### Explicitly out of scope
- PATCH-001's writer hardening (offset expansion, MPF cap, quadratic
  deletion).
- JFXX: DEC-012 already classes it as `OtherApp`, so it is already removed.
- Any other APPn reclassification without a measured example (the rule in
  DEC-012 itself).

## Spec Backlog

Format: `- [status] SPEC-ID (cycle) — one-line summary`

- [x] SPEC-040 (shipped on 2026-11-24) — DEC-012 amendment: a JFIF APP0 thumbnail is a Thumbnail region, audited and stripped [M; actual M]

**Count:** 1 shipped / 0 active / 0 pending

## Design Notes

- Related, outside the stage: PATCH-001 (the same review's findings 1, 2
  and 4), shipped in PR #59.

## Dependencies

### Depends on
- STAGE-002 / STAGE-004 (shipped) — the JPEG metadata extraction and the JPEG
  strip being amended
- External: ExifTool (oracle only, never linked)

### Enables
- An honest "Everything" claim for the M1 binary release notes

## Stage-Level Reflection

- **Did we deliver the outcome in "What This Stage Is"?** Yes. A JFIF APP0 thumbnail is a `JfifThumbnail` region
  with a Medium `Thumbnail` finding; `AllPii`, `All` and `Everything` remove the segment, and ExifTool 13.55 agrees on
  every recorded output. The verifier's independent probe (28 inputs, 1,760-input regression sweep) found no leak, no
  over-cut and no change on any input without a JFIF thumbnail.
- **How many specs did it actually take?** One, as planned.
- **How many outlines survived unchanged?** Not framed with `just frame-stage`; n/a.
- **What changed between starting and shipping?** The success criterion "neutralise" was settled as whole-segment
  removal, because ExifTool cannot remove the thumbnail alone and the segment is already DEC-013's unit of removal.
- **Lessons that should update AGENTS.md, templates, or constraints?**
  - None codified. Below the bar: write an AC's terms in the codebase's existing meaning (AC6 `is_clean`).
- **Signals dispositioned at this close?** No `type: lesson` signal in `guidance/signals.yaml` is owned by this
  stage.
- **Should any spec-level reflections be promoted to stage-level lessons?**
  - Not yet. Seed-count pins dominated the build's overrun here and in PATCH-001's port; left for the next close
    that sees it again.
- **Cost.** SPEC-040: 583,555 tokens (build 338,116; verify 245,439; agent-result basis, which is not the transcript
  basis SPEC-033 used, so the two are not comparable). Orchestration: at least 36,753,906 (a floor, shared with
  PATCH-001). USD not priced: no 5.5 list rates are on record.
