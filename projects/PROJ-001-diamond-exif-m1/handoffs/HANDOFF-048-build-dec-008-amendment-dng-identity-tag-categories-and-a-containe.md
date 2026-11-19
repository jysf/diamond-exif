---
# Maps to ContextCore handoff.* semantic conventions.
#
# ONE handoff per delegated CYCLE. With build and verify running on different
# agents you get TWO handoffs per spec (HANDOFF-N build, HANDOFF-M verify) —
# `handoff.cycle` is what distinguishes them.
#
# The `handback:` block below is the RETURN path and it is not optional: it is
# how cost gets into the spec without the orchestrator hand-counting anything.
# `just handback-sync SPEC-NNN` reads it and appends the cost session for you.
# Rationale + the full contract: docs/decisions/DEC-013-delegated-cost-handback.md

handoff:
  id: HANDOFF-048
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-19
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-023

project:
  id: PROJ-001
  stage: STAGE-005
repo:
  id: diamond-exif

# ── THE HANDBACK ────────────────────────────────────────────────────────────
# Filled in by the EXECUTING AGENT before it reports done. This is a required
# part of completing the handoff, not a courtesy.
#
# `tokens_total` is the one field the cost gate reads. Report the REAL number,
# and say in `basis` how you got it:
#   Claude Code   → your session transcript, measured per docs/cost-tracking.md
#                   "Measuring a cycle": count each message.id once, take its
#                   largest output_tokens, include subagents/ (basis: transcript).
#                   Summing every line over-counts ~2×. `/cost` in the desktop
#                   Code tab reports context size, not spend.
#   API           → the `usage` objects, summed (basis: api-usage)
#   another agent → whatever your harness reports as total tokens
# If your platform genuinely exposes NO token count, set tokens_total: null AND
# write why in `notes` — then set `cost.metering_source: none` in
# .repo-context.yaml so the gate stops asking. Do not invent a number.
handback:
  status: completed
  agent: claude-sonnet-5-5   # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: null                    # verify handoffs: approved | punch-list | rejected
  basis: estimate                  # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: 230000              # REAL combined count — what cost-audit reads
  estimated_usd: null              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: 35
  branch: claude/dec-008-dng-and-pages
  pr: null
  completed_at: 2026-11-19
  notes: "tokens_total is an estimate of context use (no transcript measurement possible from inside); orchestrator should replace with harness subagent_tokens. About 60 tool calls."                       # if unusual; double-quote it — an unquoted " #" starts a YAML comment
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-048: Build SPEC-023 DEC-008 amendment (DNG identity tags, container-aware TIFF page rule)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-023` to `claude-sonnet-5-5` (implementer) for the **build** cycle. The spec's Design (Goal, Decisions made here, Outputs, Acceptance Criteria, Failing Tests, Non-Goals, Notes) is the contract: **read the whole spec first**: `projects/PROJ-001-diamond-exif-m1/specs/SPEC-023-dec-008-amendment-dng-identity-tag-categories-and-a-containe.md`.

## BUDGET (hard)

- At most about **100 tool calls or 250k context**. At the limit: commit what you have, fill the handback (state exactly what is done and what remains), stop. Do NOT continue past the limit; the orchestrator will spawn a fresh builder from your handback.
- Generate fixtures and seeds with ONE script or generator function, not many Write calls. Read large files with offset/limit (`src/write/exif.rs`, `src/audit/mod.rs`, `tests/strip_tiff.rs`, `src/write/exif_tests.rs` are large: read only what you need).
- Record a real `tokens_total` in the handback and say in `basis` where it came from; never invent one. (The orchestrator corrects it to the harness `subagent_tokens` figure; do not count cache reads.)
- Work in this worktree on branch `claude/dec-008-dng-and-pages`, commit on it. Do NOT push, do NOT open a PR, do NOT touch main. One agent at a time.
- Toolchain: `~/.cargo/bin/cargo +1.90.0` with `~/.cargo/bin` first in PATH; fuzz needs nightly via the PATH trick (`just fuzz-smoke rewrite_tiff` does it). Read `guidance/toolchain-brief.md`.

## Context to read first

- `AGENTS.md` sections 8, 11 to 13 and 15 (delegated cycles and build rules) only.
- `guidance/toolchain-brief.md`; `guidance/constraints.yaml` (forbid unsafe, five-lint deny set, no panics on untrusted input, oracle-must-be-shown-red, skips are loud, no I/O in the library, ExifTool never linked or read).
- The spec (above), then `decisions/DEC-008-audit-categories-and-severity.md`, `DEC-013-writer-foundation-fail-closed-removal-and-patch-plan.md` (amendments C and D, especially D.3), `DEC-009`, `DEC-012`.
- Code: `src/audit/category.rs` (`categorize`, `by_id`), `src/audit/mod.rs` (`add_container`, `audit_tiff`, the IFD1 `TAG_STRIP_OFFSETS` rule), `src/write/exif.rs` (`select`, `plan_block`, the IFD1 whole-removal block, `THUMB_IFD_TAGS`), `src/write/tiff.rs`, `src/write/verify.rs` (`verify_tiff`), `tests/strip_tiff.rs`, `tests/strip_tiff_oracle.rs`, `tests/support/tiff_shapes.rs`, `examples/fuzz_seeds.rs`.
- The two questions in `guidance/questions.yaml`: `dng-identity-tags-have-no-category`, `tiff-ifd1-strips-is-a-page` (decided 2026-11-11).

## Design summary (full text is in the spec)

1. **Categories.** `by_id`: 0xC614 and 0xC615 -> `CameraModel`; 0xC68B -> `FreeText`; 0xC634 -> `MakerNote`. They apply in any non-GPS, non-Interop IFD. `GpsAndDevice` still leaves 0xC634 (MakerNote is not in that preset); note it in DEC-008.
2. **Page rule.** In a bare TIFF (`audit_tiff`, `Origin.block == None`) an IFD1 is a `Thumbnail` only when it has the JPEG pair 0x0201/0x0202 and no 0x0111, 0x0117, 0x0144 or 0x0145. An IFD1 with strips or tiles is a page: no `Thumbnail` finding. JPEG Exif IFD1 rule unchanged. Keep per-IFD context computed once per IFD (no per-entry scans).
3. **Writer follows the audit.** `select` sets `thumbnail` only when the audit has a `Thumbnail` finding; a kept page IFD1 is never removed whole, its entries still go by their own categories, its strip and tile data stay protected. D.3's strips-form refusal becomes JPEG-only. An IFD1 with both the JPEG pair and strips keeps being refused (`ThumbnailNotLocated`). Do NOT special-case DNG in the writer; if the existing deletion path refuses one of the four tags, report it, do not bypass it.
4. **Decisions and docs.** DEC-008 amendment (the four rows, the GpsAndDevice residual, the container-aware rule) and a DEC-013 D.3 amendment, short, in those files. Mark both questions `answered` naming SPEC-023. Remove the README "Known limits" bullet about multi-page TIFFs and DNG identity tags. Update the `src/write/tiff.rs` module doc.
5. **Tests, written first** (see the spec's Failing Tests; show them red before the change, and say honestly in the handback if you could not). Replace the existing multi-page refusal test and name it in the handback. ExifTool oracle tests with replay records and a red-proof each; extend the records directory `tests/oracle-fixtures/exiftool-strip-tiff/`; live tests skip loudly without ExifTool. Fuzz seeds for the three new shapes; `just fuzz-smoke rewrite_tiff` 60 s clean.

## Expected Deliverables

- Commits on the branch implementing the spec; every Acceptance Criterion met or listed as not met.
- All gates green: `just lint`, `just test`, `just deny`, `just forbid-unsafe`, `just validate`, the fuzz lint/deny recipes in the toolchain brief, `just fuzz-smoke rewrite_tiff`.
- Spec updated (acceptance ticks, build reflection answered honestly).
- Handback filled in the front matter AND the section below, including real `tokens_total`, branch, and the tool-call count.
- Provenance: net-new code is DEC-003 class 1; add ledger rows if the process requires for new files.

## Out of Scope

PNG, WebP, HEIC, BigTIFF, `set` operations, MakerNote parsing or editing, changing `GpsAndDevice` membership, the `audit-blob-carrier-parsing` question, any byte growth or relocation, stage or project bookkeeping, push, PR.

## Return Criteria: how to hand back

Fill the `handback:` front matter (including a real `tokens_total`) AND the `## Handback` section below; set `handoff.status` and `handback.status` to `completed`. If you cannot complete it: fill the section with what is done and what blocked you, set `handoff.status: rejected` and `handback.status: blocked`, still report your tokens, set the spec's `task.blocked: true`, and add a question to `guidance/questions.yaml`. **Split trigger:** if you reach the budget with the audit change and the writer change done but the oracle records or fuzz seeds not, commit, list exactly what remains, and stop.

---

## Handback` section below.** An unfilled handback is an
incomplete cycle, and `just handback-sync` will tell the orchestrator so.

On success:
1. Fill the `handback:` front-matter — **including a real `tokens_total`**.
2. Fill the `## Handback` section (the reflection is part of it, not optional).
3. Set `handoff.status` → `completed` and `handback.status` → `completed`.
4. Open a PR (build) or return your verdict (verify).

If you cannot complete the task:
1. Fill the `## Handback` section with what was done and what blocked you.
2. Set `handoff.status` → `rejected`, `handback.status` → `blocked`.
3. **Still report your token usage** — blocked work costs money too.
4. Set the spec's `task.blocked: true` and add a question to
   `/guidance/questions.yaml`.

---

## Handback

*Filled in by the receiving agent. The orchestrator does not reconstruct any of
this — it transcribes it. The reflection questions are part of completion.*

### Execution notes

- **Branch / PR:** `claude/dec-008-dng-and-pages`, committed, not pushed, no PR.
- **Completed at:** 2026-11-19
- **All acceptance criteria met?** yes. The existing deletion path refused none of the four DNG tags (IFD0 and SubIFD, both byte orders); no writer special case for DNG.

Changes: `by_id` rows (0xC614/0xC615 CameraModel, 0xC68B FreeText, 0xC634 MakerNote); audit IFD1 strips rule gated on `origin.block.is_some()`; `select` sets `thumbnail` only with an IFD1 `Thumbnail` finding in that block; `verify.rs` `entries` check removes IFD1 only in blocks where the audit has a IFD1 `Thumbnail` finding (needed: the self-check otherwise failed `Entries` on a kept page); fuzz model `fuzz/shared/audit_invariants.rs` updated (four rows, container-aware rule). DEC-008 and DEC-013 amendments, questions already `answered` (notes now name SPEC-023), README bullet cut, `tiff.rs` doc.

Tests: new `src/audit/category.rs` unit test, `tests/audit_model.rs` (page rule, JPEG guard), `tests/strip_tiff.rs` (multipage and tiled page strip, DNG identity, pair+strips still refused), `tests/strip_tiff_oracle.rs` (dng identity after everything, multipage after all_pii, live variant, red-proofs by planted lines). Shown red before the src change: category unit test, `bare_tiff_ifd1_with_strips_is_a_page`, and 5 strip_tiff tests failed; the oracle tests were written with the records and were not run red against old src (the red-proof is planted-line mutation of the comparator, not a reverted by_id).
- Replaced test: the multi-page refusal assertion in `a_page_ifd1_is_never_removed_and_a_jpeg_thumbnail_is` (strip_tiff.rs) and `expect_refusal` (now always false) in `every_shape_under_every_policy_...`. Also changed: lib test `an_ifd1_with_strips_is_a_thumbnail_in_a_jpeg_and_a_page_in_a_bare_tiff` (renamed), `audit_seeds_are_committed...` and the two `audit-ifd1-strips` plants in `tests/fuzz_layout.rs` (seed bytes untouched: pinned by `budget_oracle`; the plants now use a generated JPEG-wrapped copy).
- Records: `dngidentity-{le,be}.txt`, `tiledpage-{le,be}.txt` new; `multipage-*.txt` gain `all_pii`, `all`, `everything` sections; the other 10 records byte-identical. Seeds: 4 shapes x 7 heads x 2 plus soi variants added to `fuzz/seeds/rewrite_tiff` (dngidentity, tiledpage, pagejpeg).
- Gates: `just lint`, `lint-fuzz`, `deny`, `deny-fuzz`, `forbid-unsafe`, `validate`, full `cargo test` all green (lint needs `RUSTUP_TOOLCHAIN=1.90.0`); `just fuzz-smoke rewrite_tiff` 60 s, 537k runs, clean. Not run: `lint-red-proof`, `mutation-oracle` separately (included in cargo test: green).

### Cost self-report

- **Tokens (total):** about 230000 (estimate)
- **Estimated USD:** null
- **Duration (minutes):** 35
- **Source of the number:** none available (estimate; replace with harness figure)

### Drift and new artifacts

- **New decisions emitted:** none (amendments to DEC-008 and DEC-013).
- **Deviations from spec:** verify.rs change and fuzz model change were not listed in Outputs but required. The toolchain brief's seed counts and the `audit-ifd1-strips` description were not updated.
- **Follow-up work identified:** none required; `rewrite_tiff` seed counts in docs (toolchain brief) may want a refresh.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — The spec did not mention the self-check (`verify.rs`) or the independent fuzz model, both of which encode the old thumbnail rule.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — The pinned seed hashes in `budget_oracle`, which forbid editing `audit-ifd1-strips`.

3. **If you did this task again, what would you do differently?**
   — Grep for every encoding of the IFD1 strips rule first (`grep 0x0111`), then write tests.
