---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-036
  type: story                      # epic | story | task | bug | chore
  cycle: ship  # frame | design | build | verify | ship
  blocked: false
  priority: medium                 # critical | high | medium | low
  complexity: L                    # XS | S | M | L | XL | XXL — the EXPECTED size, set at design
                                   #   (XL/XXL almost certainly means it's a stage, not a spec)
  complexity_actual: null          # stamped at ship: what it ACTUALLY took, same scale.
                                   #   Expected-vs-actual drift is what `just calibration` reads.
  verify_verdict: punch-list  # approved | punch-list | rejected — the OUTCOME of the verify
                                   #   cycle, stamped by `just advance-cycle` when the spec leaves
                                   #   verify (same three verdicts Prompt 4 already returns).
                                   #   Recorded in front-matter, not just prose, so "verify never
                                   #   rejects anything" stops being a hunch and becomes a number.
  verify_rounds: [punch-list]  # every verify outcome, oldest first, appended by
                                   #   `just advance-cycle` each time the spec leaves verify.
                                   #   verify_verdict keeps only the LAST one, so a spec sent
                                   #   back and then approved read as "approved": irradiance's
                                   #   field showed 1 pushback where its handoffs showed 5.

project:
  id: PROJ-001
  stage: STAGE-010
repo:
  id: diamond-exif

handoff:
  from_agent: claude-opus-5-5  # from .repo-context tier_map.design (DEC-005)
  to_agent: null                   # filled when HANDOFF is created (any agent — see docs/porting.md)
  created_at: null

references:
  decisions: [DEC-020]                 # [DEC-NNN, DEC-MMM]
  constraints: []                  # [constraint-id-1, constraint-id-2]
  related_specs: [SPEC-013, SPEC-019, SPEC-031, SPEC-032, SPEC-034]             # [SPEC-NNN]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: [SPEC-032]                     # e.g. [SPEC-002]

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-010's <capability>". Optional; null is acceptable.
value_link: null

# Self-reported AI cost per cycle. Each cycle (design, build, verify,
# ship) appends one entry to sessions[]. Totals are computed at ship.
# Record a REAL tokens_total for metered cycles (build/verify) — the agent
# that runs the cycle writes it from its own interface (/cost, the API
# usage object, or its tool's report). Only un-metered main-loop cycles
# (design/ship) may be null-with-note. `just cost-audit` enforces this on
# shipped specs. See AGENTS.md §4 and docs/cost-tracking.md. interface:
# claude-code | claude-ai | api | ollama | other.
cost:
  # Optional PREDICTION of the total tokens this spec will take, set at
  # design. Never a gate — its only job is to be compared with the actual
  # below (`just calibration`), so you learn whether you systematically
  # under- or over-estimate. null = didn't predict.
  tokens_estimate: 600000
  sessions:
    - cycle: build
      handoff: HANDOFF-073
      agent: claude-sonnet-5-5
      interface: other
      basis: agent-result
      tokens_total: 194639
      estimated_usd: 1.28
      duration_minutes: null
      recorded_at: 2026-11-22
      notes: "transcribed from HANDOFF-073 by handback-sync"
    - cycle: build
      handoff: HANDOFF-074
      agent: claude-sonnet-5-5
      interface: other
      basis: agent-result
      tokens_total: 271968
      estimated_usd: 1.79
      duration_minutes: null
      recorded_at: 2026-11-22
      notes: "Part B done: 56 png-webp divergences pinned (Class P), replay/live/red-proof tests, 5 new questions; tokens_total left null by instruction, the orchestrator measures it."
    - cycle: verify
      handoff: HANDOFF-075
      agent: claude-opus-5-5
      interface: other
      basis: agent-result
      tokens_total: 218377
      estimated_usd: 1.44
      duration_minutes: null
      recorded_at: 2026-11-22
      notes: "tokens_total left null by instruction: the orchestrator records the harness figure. Verify of claude/spec-036-part-b at 1df62be; punch list H1, H2, M1 to M3."
    - cycle: build
      handoff: HANDOFF-076
      agent: claude-sonnet-5-5
      interface: other
      basis: agent-result
      tokens_total: 314205
      estimated_usd: 2.07
      duration_minutes: null
      recorded_at: 2026-11-22
      notes: "transcribed from HANDOFF-076 by handback-sync"
  totals:
    tokens_total: 999189
    estimated_usd: 6.58
    session_count: 4
shipped_at: 2026-11-22
---

# SPEC-036: Tag-level inventory parity against ExifTool for PNG and WebP (carried from STAGE-008)

## Context

Carried from STAGE-008. The inventory oracle (SPEC-013, SPEC-019) compares the audit's items with ExifTool's `-a -u -G1 -H -s` listing as a **multiset of `(group, tag id)`** plus a **set of region kinds**, with every difference either a pinned divergence or a failure. It covers TIFF and JPEG only. PNG and WebP have been audited since SPEC-032, but only the presence of GPS, XMP and ICC findings is checked against ExifTool. The strips in STAGE-010 verify their output by re-auditing it, so the audit's inventory of these two formats must itself be checked, tag by tag, first. SPEC-032 left the PNG and WebP copies and shapes in `fuzz/seeds/audit` out of the existing five corpora for this reason.

Measured at design (ExifTool 13.55, `-a -u -G1 -H -s`):
- A PNG prints IHDR tags in group `PNG` with hex ids (`0x0000` ImageWidth to `0x000c` Interlace); text and `tIME` tags in group `PNG` with id `-` (no id); other structural chunks in groups `PNG-<chunk>` (e.g. `PNG-pHYs`, ids `0x0000`...).
- A two-`eXIf` PNG lists only the FIRST block's entries even with `-a`; a two-`EXIF` WebP lists BOTH blocks' entries (duplicates kept). WebP structure lines are group `RIFF` (`WebP_Flags`, `ImageWidth`, `AlphaIsUsed`, ...).
- Exif entries inside `eXIf` and `EXIF` appear in the usual `IFD0`, `ExifIFD`, `GPS`, ... groups.

## Goal

A third inventory corpus, `png-webp`, run through the existing comparator extended for these two formats: Exif entries as a multiset of `(group, id)`, XMP and ICC as presence, the PNG text and `tIME` tags as a count, structural groups dropped by name, everything else a pinned divergence with a measured cause and a `guidance/questions.yaml` id. Replay always, live against ExifTool gated like its siblings, comparator red-proofed (replay-and-perturb and mutated-copy).

## Inputs

- `tests/support/inventory.rs` (comparator; `#[path]`-included by the probe crate, so it must stay free of other support modules), `inventory_corpus.rs`, `tests/inventory_oracle.rs` (the SPEC-019 `real-shapes` section is the template), `tests/mutation_oracle.rs`, `examples/oracle_record.rs`
- Fixture sources: `tests/support/oracle_png.rs` (fixtures, `TEXT_KEYWORDS`), `oracle_webp.rs`, `png_shapes.rs`, `webp_shapes.rs`, the committed seeds `fuzz/seeds/{png_chunks,webp_riff,audit}`
- `src/audit/mod.rs` (`audit_png`, `audit_webp`, `Origin`, `RegionKind::{Text, RawProfile, Time, ...}`), DEC-020, DEC-018, DEC-019
- ExifTool 13.55 (run, never read), the existing `KNOWN_DIVERGENCES` and questions for png/webp

## Outputs

- **Modified `tests/support/inventory.rs`:** group classification for `PNG` (hex id: structural; id `-`: counted text/time tag), `PNG-*` and `RIFF` (structural, dropped by name, listed in a constant like `UNMODELLED_GROUPS`), the XMP and ICC groups for these formats as region presence; a `text_tags` count on `Inventory`; `ours` taking a per-format block rule (PNG: only the first `eXIf` block's tag items, because ExifTool reads only the first; WebP: every listed block) and counting our `Text` and `Time` regions against ExifTool's `PNG` id-less lines; `Trailer`, `OtherApp`, `Comment` stay class S. The classification is MEASURED: Part A lists every `(group, id-or-dash)` ExifTool prints across the corpus before anything is dropped, and the list is pinned.
- **Modified `tests/support/inventory_corpus.rs`:** `png-webp` in `NEW_CORPORA` (its own record, so the 178 existing results and counts cannot move, as SPEC-019 did); extension by corpus (`png` / `webp`, the existing `jpg`/`tif` rule untouched for the five old corpora).
- **Modified `examples/oracle_record.rs`:** records the new corpus to `tests/oracle-fixtures/exiftool-inventory/png-webp.txt` (idempotent).
- **Modified `tests/inventory_oracle.rs`, `tests/mutation_oracle.rs`:** replay, live and red-proof tests for the new corpus; pins for the new counts (the old pins unchanged).
- **Modified `src/audit` only if a real bug is found**: record it and fix in a fix round, never silently.
- **Modified `guidance/questions.yaml`:** an id per new divergence class.

## Acceptance Criteria

- [x] The corpus holds every PNG and WebP oracle fixture, the `png-text-keywords` table, the PNG and WebP shapes and the committed `png_chunks`, `webp_riff` and `audit` PNG/WebP seeds, deduplicated by bytes; the count is pinned.
- [x] Every `(group, id)` ExifTool prints on the corpus is either compared, counted or dropped by a named constant; an unknown group is a parse error, not a silent drop.
- [x] For every input the audit and ExifTool agree on the Exif multiset, XMP and ICC presence and the text/time count, or the difference equals a pinned `KNOWN_DIVERGENCES` entry (exact multiset difference, measured cause, `questions.yaml` id); a divergence that stops holding fails.
- [x] PNG: only the first `eXIf` block is compared and the later blocks are asserted present in the audit (a test), not silently dropped; WebP: every listed block compared, the 17th-block case pinned (`webp-exif-block-cap`).
- [x] Hostile edges measured and covered (design variant grid, in the Failing Tests): raw-profile exif (with and without GPS), `tIME` lengths 0 to 12, empty regions, nameless empty `iCCP`, text after `IEND`, `eXIf` before and after `IDAT` and after `IEND`, XMP in `iTXt`/`tEXt`/`zTXt`/compressed, keyword spellings, WebP VP8X flags wrong, odd chunk without pad, animated, trailing bytes, 16 vs 17 blocks.
- [x] Red-proofs: replay-and-perturb on the new record names the perturbed `(group, id)`; at least three mutated-copy proofs (e.g. a `Text` region counted twice, `Time` items dropped, the PNG first-block rule removed) each fail the new tests; control first, one injection, marker not leaked; `just mutation-oracle` green.
- [x] No change to any existing inventory record, count or test result; `just oracle-record` idempotent; `just lint`, `just test`, `just forbid-unsafe`, `just validate`, `just cost-audit` green on the pinned 1.90.0.

## Failing Tests

- **`tests/inventory_oracle.rs`** (new section, SPEC-036): corpus pins; `png_webp_replay_is_clean_or_pinned`; `png_webp_live_parity` (gated); `png_first_block_rule_and_later_blocks_listed`; `webp_every_block_compared_and_cap_pinned`; `text_and_time_count_agrees`; `unknown_group_is_a_parse_error`; `every_png_webp_divergence_has_a_question_entry`; `a_png_webp_divergence_that_stops_being_one_fails`.
- **Unit tests in `tests/support/inventory.rs`'s consumers:** classifier cases for `PNG` hex, `PNG` dash, `PNG-pHYs`, `RIFF`.
- **`tests/mutation_oracle.rs`:** three mutants as above.

## Non-Goals

- Strip or any writer change; new audit behaviour (a real bug found here is fixed in its own round, recorded).
- Listing tags inside compressed text, raw profiles, XMP or ICC.
- Value comparison (this is the tag-level inventory, not values).
- Changing the five existing corpora, their records or counts.

## Notes for the Implementer

- **Two builder parts so a stop is clean.** Part A: comparator classification, corpus, recorder, the committed record and the measured group list (commit). Part B: replay/live/divergence tests, red-proofs, questions, docs. Part A hands back the measured table of every group and id and every raw difference before any divergence is pinned.
- Follow SPEC-019's `real-shapes` wiring exactly (own record, own tests, counts separate).
- `inventory.rs` is copied next to the probe crate: no new `use` of other support modules.
- Per-format model lives in the harness, deliberately not a library feature (as `read_ours_jpeg`/`read_ours_webp`).
- Known differences to expect, not hide: ExifTool decodes raw profiles into groups the audit does not list (`png-raw-profile-not-decoded`; the audit marks such a file unreadable); a 17th WebP block (`webp-exif-block-cap`); ICC and XMP regions ExifTool validates and the audit locates (`webp-iccp-profile-not-validated`, `webp-xmp-content-not-parsed`); `creation time` keyword spelling (`png-creation-time-keyword-case`); a nameless length-0 `iCCP` (walk issue, no located region).
- ExifTool: `perl -e 'alarm 8; exec @ARGV' -- exiftool ...` (macOS has no `timeout`); never run on an input that may not return without the limit; probe each new hostile shape under the limit first.
- Budget about 100 tool calls or 250k context per agent; expect two builder agents.

---

## Reflection

1. **What would I do differently next time?**
   — Put a corpus of ordinary real-world chunks (gAMA, sRGB, iDOT, bKGD...) and keyword collisions in the DESIGN measurements: the design assumed id-less `PNG` lines were text and `tIME`, the part A measurement corrected that, and the verify still found ten more non-text names and a library bug because the corpus held no such chunks. Also tell builders "case-insensitive" only after measuring: I briefed case-insensitive, the fixer measured two exact spellings and was right.

2. **Does any template, constraint, or decision need updating?**
   — DEC-018 got "Clarifications from the SPEC-036 verify" (raw-profile keyword is exactly two spellings). Signal `design-must-measure-hostile-edges-not-only-the-happy-keys` is now N=4 (this spec's H1 and M1); `subagent-token-self-report-counts-cache-reads` did not recur (the handoffs told agents to leave tokens null). A builder that "waits on background work" again re-notified; I stopped it at part B. The fix round (which changed `src/png`) got no second cold verify; I read the `src` diff and re-ran lint and three test binaries.

3. **Is there a follow-up spec I should write now before I forget?**
   — No. The inventory oracle's by-name classification of id-less `PNG` lines is a documented limit (`png-text-keyword-collides-with-chunk-tag-name`); unknown or reserved chunk names that ExifTool prints as their own tag name cannot be dropped by name.

4. **Where was the worst defect caught?** — verify (a lowercase raw-profile chunk with GPS read as clean: a library bug hidden behind a pin with a wrong cause).

5. **What can a user do now that they couldn't before?** — none directly; the audit's PNG and WebP inventory is now checked tag by tag against ExifTool on 364 distinct inputs (63 pinned differences, each with a measured cause), and a lowercase `raw profile type exif` PNG is no longer read as clean.
