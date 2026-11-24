---
# Maps to ContextCore epic-level conventions.
# A Stage is a coherent chunk of work within a Project.
# It has a spec backlog and ships as a unit when the backlog is done.

stage:
  id: STAGE-011                     # stable, zero-padded, continuous across the repo
  status: proposed                  # proposed | active | shipped | cancelled | on_hold
  priority: medium                  # critical | high | medium | low
  target_complete: null             # optional: YYYY-MM-DD

project:
  id: PROJ-001                      # parent project
repo:
  id: diamond-exif

created_at: 2026-11-23
shipped_at: null

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
  sessions: []                      # - tokens_total: N
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

- [ ] SPEC-040 (frame) — DEC-012 amendment: a JFIF APP0 thumbnail is a Thumbnail region, audited and stripped

**Count:** 0 shipped / 0 active / 1 pending

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

*Filled in when status moves to shipped. Run Prompt 1c (Stage Ship) in
FIRST_SESSION_PROMPTS.md to draft this.*

- **Did we deliver the outcome in "What This Stage Is"?** <yes/no + notes>
- **How many specs did it actually take?** <number vs. plan>
- **How many outlines survived unchanged?** <n of m — only if this stage was
  framed with `just frame-stage`. Counts outlines whose scope held from frame to
  ship. A low number means framing ran ahead of what was knowable; a high one
  means it's paying for itself.>
- **What changed between starting and shipping?** <one sentence>
- **Lessons that should update AGENTS.md, templates, or constraints?**
  - <one-line updates>
- **Signals dispositioned at this close?** (Prompt 1d step 7) Every
  `type: lesson` signal in `/guidance/signals.yaml` owned by this stage close
  was walked — codified (at its bar), left `watch`, or dropped. No silent carry.
  - <note what codified / what's still watch + its N>
- **Should any spec-level reflections be promoted to stage-level lessons?**
  - <one-line items — record below-bar ones as `watch` signals; don't codify yet>
