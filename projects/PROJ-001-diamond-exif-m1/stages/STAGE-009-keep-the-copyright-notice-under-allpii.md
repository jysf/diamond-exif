---
# Maps to ContextCore epic-level conventions.
# A Stage is a coherent chunk of work within a Project.
# It has a spec backlog and ships as a unit when the backlog is done.

stage:
  id: STAGE-009                     # stable, zero-padded, continuous across the repo
  status: active                    # proposed | active | shipped | cancelled | on_hold
  priority: medium                  # critical | high | medium | low
  target_complete: null             # optional: YYYY-MM-DD

project:
  id: PROJ-001                      # parent project
repo:
  id: diamond-exif

created_at: 2026-11-21
shipped_at: null

# What part of the project's value thesis this stage advances.
# If you can't articulate value_contribution, the stage may be
# infrastructure-only — acceptable but flag it.
value_contribution:
  advances: "The default privacy strip removes what identifies a person without removing the owner's own copyright notice, so a photographer can use the default on their own work."
  delivers:
    - "AllPii (the CLI's default preview policy and the library's privacy preset) keeps the Exif Copyright tag; All and Everything still remove it"
    - "Reports name Copyright under its own category, rights, separate from authorship"
  explicitly_does_not:
    - "add a CLI flag to keep or remove a single category (a Custom set stays library-only)"
    - "edit inside XMP or IPTC packets (their rights fields follow the packet, as today)"
    - "change any category's severity or what Gps, GpsAndDevice or Everything remove"

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

# STAGE-009: Keep the copyright notice under AllPii

## What This Stage Is

A one-spec stage that takes the Exif Copyright tag (0x8298) out of the `Authorship` category and gives it its own category, `Rights`, which `AllPii` keeps and `All` and `Everything` remove. After it, the privacy preset still removes every name the camera or the editor wrote about a person (Artist, Photographer, XPAuthor, CameraOwnerName, ImageEditor), and leaves the rights notice the owner attached.

## Why Now

`AllPii` is the default of `exif-audit --policy` and the obvious preset for the library's `strip`. Removing a copyright notice by default works against the most common user (someone publishing their own photos), and in the US knowingly removing copyright management information is what 17 U.S.C. §1202 is about. The owner decided on 2026-11-20 that a default strip must not remove it. The change is cheapest now: the CLI contract is alpha (0.1.0-alpha.1), the strip writer is library-only, and STAGE-007's SPEC-027 (presets while building the Exif block) has not been designed, so it reads the amended DEC-009 from the start.

## Success Criteria

- `strip(_, AllPii)` leaves 0x8298 byte-identical while removing Artist; ExifTool prints `IFD0:Copyright` on the `all_pii` output of the strip fixtures and no `IFD0:Artist`.
- `All` and `Everything` still leave no Copyright, and `All` still removes the Exif segment whole.
- No audit's `max_severity` changes on any committed fixture or seed; only the category name of the Copyright finding changes.
- DEC-008, DEC-009 and DEC-013 carry the amendment; CHANGELOG `[Unreleased]` says what changed for a 0.1.0-alpha.1 user.

## Scope

### In scope
- The `Rights` category (code, DEC-008 row, DEC-009 preset and may-hold rows), the writer's Exif set going from nine categories to ten, the fuzz models, the ExifTool strip records.

### Explicitly out of scope
- A CLI way to choose a `Custom` set (question to raise if a user asks).
- XMP `dc:rights` and IPTC CopyrightNotice: they are inside packets the writer keeps or drops whole.

## Spec Backlog

- [ ] SPEC-033 (design) — DEC-008 amendment: a `Rights` category for Copyright, kept by `AllPii` [S]

**Count:** 0 shipped / 1 active / 0 pending

## Design Notes

The category is appended after `OtherSegment` in `Category::ALL` (as DEC-012 appended its two), so the bit is `0x8000` and existing report orderings do not move. That fills `StripSpec`'s `u16`; the next category needs a `u32` (said in the spec, not done here).

## Dependencies

### Depends on
- STAGE-005 (shipped): the category table and the writer's category-driven entry deletion.
- STAGE-010 SPEC-038 (WebP strip, PR #55): adds WebP copies of the Exif set; SPEC-033 builds after it ships (revision 2026-11-22). The PNG strip (SPEC-037) is already on main and is in SPEC-033's scope.

### Enables
- STAGE-007 SPEC-027: presets while building the Exif block read the amended DEC-009 (no `depends_on`; SPEC-027 is still an outline).

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
