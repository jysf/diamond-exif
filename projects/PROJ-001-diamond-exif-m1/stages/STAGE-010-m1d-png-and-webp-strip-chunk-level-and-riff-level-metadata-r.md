---
# Maps to ContextCore epic-level conventions.
# A Stage is a coherent chunk of work within a Project.
# It has a spec backlog and ships as a unit when the backlog is done.

stage:
  id: STAGE-010                     # stable, zero-padded, continuous across the repo
  status: shipped                   # proposed | active | shipped | cancelled | on_hold
  priority: high                    # critical | high | medium | low
  target_complete: null             # optional: YYYY-MM-DD

project:
  id: PROJ-001                      # parent project
repo:
  id: diamond-exif

created_at: 2026-11-22
shipped_at: 2026-11-27

# What part of the project's value thesis this stage advances.
# If you can't articulate value_contribution, the stage may be
# infrastructure-only — acceptable but flag it.
value_contribution:
  advances: "Closes the gap the audit now shows: exif-audit reads PNG and WebP but says they are not strippable, so the sanitiser still covers only JPEG and TIFF; this stage makes strip work for the two other formats users upload, with the same fail-closed, self-verified discipline."
  delivers:
    - "strip of a PNG and of a WebP under every StripPolicy: whole-chunk and RIFF-level removal, output re-audited, typed refusals, never longer than the input"
    - "rewrite_png and rewrite_webp fuzz targets with a 60 s CI smoke"
    - "ExifTool write parity (I3) for both formats, comparator and writer red-proofed"
    - "exif-audit's preview says Removed for PNG and WebP only where the writer has shown it can"
  explicitly_does_not:
    - "set operations, MakerNote rules, HEIC, or a release (a release is its own spec afterwards)"
    - "edit inside XMP or ICC payloads, re-encode pixels, or handle APNG frames"

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
    - tokens_total: 525194
      estimated_usd: 3.47
      recorded_at: 2026-11-27
      notes: "Measured by the orchestrator from transcript c6de7258 (Sonnet 5.5), main-loop messages from 'Frame next stage' (2026-11-22T21:22Z) to before the next unrelated request (2026-11-25T02:38Z), 137 de-duplicated messages: input 344 + output 154,699 + cache_creation 370,151, cache reads (58,496,065) excluded, 6.60 USD/Mtok (same method as STAGE-008). Covers framing, the SPEC-036 to SPEC-038 designs, DEC-021 and DEC-022, handoffs, routing of builders and verifies, PR and merge steps. Subagent spend is in the specs' own cost sessions, not here."
    - tokens_total: 1376554
      estimated_usd: null
      recorded_at: 2026-11-27
      notes: "Measured by the orchestrator from transcript d1fb0ea8 (Opus 5.5), every main-loop message from 'Design SPEC-039' (2026-11-25T03:47Z) to this close's measurement, 373 de-duplicated messages: input 830 + output 259,508 + cache_creation 1,116,216, cache reads (71,493,585) excluded. Covers the SPEC-039 design (DEC-023, the DEC-020 amendment; the spec's own design session is null), routing of three build parts, five verify and five fix rounds, the CI fix on PR #67, the ship and this close. USD left null: the 6.60 USD/Mtok rate used so far was for Sonnet, and no Opus rate is recorded in the repo. Subagent spend is in SPEC-039's cost sessions, not here."
                                    # - tokens_total: N
                                    #   estimated_usd: N
                                    #   recorded_at: YYYY-MM-DD
                                    #   notes: "framing + spec breakdown"
---

# STAGE-010: M1d PNG and WebP strip: chunk-level and RIFF-level metadata removal, self-verified, ExifTool I3, audit switch-over

## What This Stage Is

Make `strip` work for PNG and WebP the way it works for JPEG and TIFF: remove the metadata the policy names, prove it by re-auditing the output, refuse rather than guess, and show agreement with ExifTool. After this stage `exif-audit` can honestly say a PNG or WebP would be stripped, and the library's writer covers every container read so far.

## Why Now

STAGE-008 shipped the readers and the audit, and the audit's preview states plainly that both formats are "not strippable". The walkers are fuzzed and oracle-checked, the audit lists every chunk that holds metadata, and DEC-013's writer discipline is proven on two containers. The strip is the largest remaining visible gap for a server-side upload sanitiser. Inventory parity (SPEC-036, deferred from STAGE-008) comes first because every strip is verified by re-auditing its output.

## Success Criteria

- `strip` of a PNG and of a WebP under all five presets returns a file the audit finds clean of the removed categories, or a typed refusal; no panic on any input; the output is never longer than the input.
- Nothing but the policy's targets changes: kept chunks are byte-identical (PNG CRCs stay valid), the RIFF size and `VP8X` flags are consistent, pad bytes are right.
- Hostile shapes are handled by walking every chunk, not by the library's lists: a 17th WebP `EXIF` block, text after `IEND` or after a null chunk, several `eXIf`, raw-profile chunks that hold GPS, `tIME` of any length, empty regions, a nameless empty `iCCP`.
- `rewrite_png` and `rewrite_webp` fuzz targets exist, seeded, with a 60 s smoke in CI (constraint `fuzz-target-per-surface`).
- ExifTool I3: the stripped file read by ExifTool prints none of what the policy removed and everything else unchanged, comparator red-proofed, writer mutated-copy red-proofed, every divergence reported.
- `ContainerKind::strippable()` flips to true for PNG and WebP only in SPEC-039, with the preview and CLI honest per file; all CI jobs green on every PR; `[dependencies]` still empty.

## Scope

### In scope
- Tag-level inventory parity for PNG and WebP (SPEC-036)
- PNG and WebP writers under `src/write/`, DEC-021 (PNG write) and DEC-022 (WebP write), their fuzz targets
- ExifTool write parity, audit and CLI switch-over, README

### Explicitly out of scope
- `set` operations, MakerNote read/strip rules, HEIC (M1e), APNG and animated-WebP frame editing
- Editing inside XMP or ICC payloads, pixel decoding, re-encoding
- A release; the Relocatable Exif block (STAGE-007, requested by monorail-000, framed separately)

## Spec Backlog

- [x] SPEC-036 (shipped on 2026-11-22) — [L, two builder parts and a fix round] Tag-level inventory parity against ExifTool for PNG and WebP (carried from STAGE-008)
- [x] SPEC-037 (shipped on 2026-11-23) — [L, two builder parts and a fix round] PNG strip: whole-chunk removal under StripPolicy, `eXIf` decision, self-verify, `rewrite_png` fuzz, DEC-021
- [x] SPEC-038 (shipped on 2026-11-23) — [L, two builder parts and a fix round] WebP strip: RIFF-level removal, size and `VP8X` rewrite, self-verify, `rewrite_webp` fuzz, DEC-022
- [x] SPEC-039 (shipped on 2026-11-27) — [L, three builder parts] ExifTool write parity (I3) for both strips (DEC-023: judge, differential against ExifTool's writer, exit codes); audit and CLI switch-over (DEC-020 amendment of 2026-11-24)

**Count:** 4 shipped / 0 active / 0 pending

## Design Notes

- **Same discipline as DEC-013:** pure function, `EditPlan` removals and same-length patches, output re-audited and compared with the input's audit, fail closed with typed refusals. A PNG removal is a whole chunk (kept CRCs untouched); a WebP removal changes the RIFF size and `VP8X` flags.
- **Walk, do not list.** The strip works from the chunk or RIFF walk. The library lists at most 16 WebP `EXIF` blocks and a PNG raw profile is not decoded; a strip that relied on those lists would leave GPS behind. SPEC-032's verify found this class three times.
- **Measure hostile edges in the design**, against ExifTool, as a variant grid (signal `design-must-measure-hostile-edges-not-only-the-happy-keys`).
- **Order.** SPEC-036 first (the re-audit the strips rely on must be checked), then SPEC-037 and SPEC-038 (independent; do not build at the same time), then SPEC-039. Specs are framed as outlines only; each is designed just in time.
- **The preview stays honest until the writer is proven:** `strippable()` flips in SPEC-039, per file where the writer can act.
- **Parallel sessions:** STAGE-007 (SPEC-025 to SPEC-027) and STAGE-009 (SPEC-033) are other sessions' work and touch `src/write` and `src/audit/category.rs`; merge main before each design.

## Dependencies

### Depends on
- STAGE-008 (shipped, PR #51 for its close): the readers, the audit wiring (DEC-020), the PNG and WebP oracle fixtures
- STAGE-004 (shipped): the writer foundation (DEC-013), `EditPlan`, the I3 apparatus
- External: ExifTool 13.55 as the oracle (run, never read or linked)

### Enables
- A later binary release whose `exif-audit` strips PNG and WebP (and a crates.io pre-release)
- M1e: HEIC read, the 24 h fuzz gate

## Stage-Level Reflection

- **Did we deliver the outcome in "What This Stage Is"?** Yes. `strip` works on PNG (SPEC-037, whole-chunk removal, `eXIf` edited in place) and WebP (SPEC-038, RIFF-level removal with size and `VP8X` rewrite) under every preset, re-audits its output and refuses with typed errors. Both have `rewrite_png` and `rewrite_webp` fuzz targets with a 60 s CI smoke. ExifTool write parity (SPEC-039, DEC-023) covers 348 differential pairs: 157 identical, 120 classed D1 to D6 and pinned, 41 outcome differences, 30 where both refuse. `exif-audit` now previews PNG and WebP as strippable. `[dependencies]` is still empty. One success criterion was missed once: PR #67's `rewrite_png` fuzz smoke failed in CI (a fuzz model missed the amended keyword rows) and was fixed before merge, so not every PR was green on its first run.
- **How many specs did it actually take?** Four, as planned (SPEC-036 to SPEC-039). Two were framed M and landed L (SPEC-036 and SPEC-039). Every spec needed at least one verify punch list; SPEC-039 took five verify rounds and five delegated fix rounds; the approval rests on round 5's evidence plus two documentation fixes by the orchestrator, not on a sixth independent round.
- **How many outlines survived unchanged?** Two of four (SPEC-037 and SPEC-038, in scope; both grew to two builder parts and a fix round). SPEC-036 grew from M to L. SPEC-039 grew from M to L and gained the PNG completeness sweep from SPEC-038's fix round.
- **What changed between starting and shipping?** The writers were right early; the cost moved into proving them: every verify found a carrier or a judge case outside the design's measurements (lowercase raw profiles, mixed-case `eXIf`/`zxIf`, a nested `RIFF` chunk, judge exception (a) at group level), so completeness sweeps against ExifTool's own tag list became build-time checks.
- **Lessons that should update AGENTS.md, templates, or constraints?**
  - AGENTS.md §15 (design) now requires a variant grid of hostile shapes measured against the oracle, plus a completeness sweep where the oracle has a tag-id list (signal `design-must-measure-hostile-edges-not-only-the-happy-keys`, codified at N=6).
  - Run the oracle on the writer's OUTPUT during build, not only in verify (SPEC-037); the live tests `strip_png_live.rs` and `strip_webp_sweep_live.rs` are that check now.
  - Read the code before writing an amendment about it (SPEC-039: two amendment drafts and one sentence were wrong because they were written from memory of the judge).
- **Signals dispositioned at this close?**
  - `design-must-measure-hostile-edges-not-only-the-happy-keys`: codified (N=6: SPEC-032, 034, 035, 036, 037, 038), landed in AGENTS.md §15.
  - `hand-typed-mirrors-of-a-decision-table-drift`: watch, N=1 (SPEC-039).
  - `fuzz-guard-needs-seeds-for-every-class`: watch, N=2, no new occurrence (the PR #67 failure was the guard working, not a class without a seed).
  - `equality-guard-plants-must-vary-in-kind` and `check-then-open-needs-a-safe-open`: watch, N=1, no new occurrence.
  - `strip-oracle-jfif-kept-presets-unrecorded`: done (SPEC-039 AC14), closed at the SPEC-039 ship.
- **Should any spec-level reflections be promoted to stage-level lessons?**
  - A model typed from the same measurement as the code cannot catch a wrong measurement (SPEC-037); the oracle sweep is what catches it. Recorded here, not yet a signal, because the codified design rule above covers its fix.
  - A fix round that changes `src` got no second cold verify in SPEC-036 to SPEC-038; SPEC-039 re-verified after every fix round, and all five rounds found something. Worth weighing at project close against the cost (SPEC-039: 2.48M tokens over 12 sessions).
  - Builders that "wait on background work" re-notified in three specs and were stopped each time.
  - Follow-ups carried: a JPEG/TIFF write differential against ExifTool (I3 here covers PNG and WebP only); `Make`, `Model` and `TimeStamp` in the PNG keyword fixture grid; the 24 h fuzz gate (M1e).
