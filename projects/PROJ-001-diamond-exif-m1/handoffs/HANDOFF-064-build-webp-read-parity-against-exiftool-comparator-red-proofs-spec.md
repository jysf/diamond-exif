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
  id: HANDOFF-064
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-21
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-034

project:
  id: PROJ-001
  stage: STAGE-008
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
  status: completed                # completed | blocked | rejected
  agent: claude-sonnet-5-5         # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: null                    # verify handoffs: approved | punch-list | rejected
  basis: estimate                  # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: null               # REAL combined count — what cost-audit reads
  estimated_usd: null              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: null
  branch: claude/spec-034-webp-parity
  pr: null
  completed_at: 2026-11-21         # YYYY-MM-DD
  notes: "tokens_total is null: the implementer cannot read its own transcript token counts; the orchestrator replaces it with the harness figure. About 75 tool calls used. Not pushed, no PR."
  # notes (original comment): if unusual; double-quote it — an unquoted " #" starts a YAML comment
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-064: Build SPEC-034 WebP read parity against ExifTool

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-034` to `claude-sonnet-5-5` (implementer) for the **build** cycle. The spec is the contract: read it whole first (`projects/PROJ-001-diamond-exif-m1/specs/SPEC-034-webp-read-parity-against-exiftool-comparator-red-proofs-spec.md`); it contains the ExifTool measurements made at design. This mirrors SPEC-031, which shipped for PNG: the pattern is `tests/png_oracle.rs`, `tests/support/oracle_png.rs`, `tests/png_mutation_oracle.rs`, the PNG parts of `tests/support/oracle.rs`, `examples/oracle_record.rs` (`record_png`), `tests/support/probe.rs`, `tests/support/tools.rs`. Read them before writing anything; do not reinvent. DEC-019 describes the reader under test; its ExifTool claims and the spec's measurements are leads, not evidence: the committed records are the evidence. One thing differs from PNG on purpose: several `EXIF` chunks merge with the later winning (JPEG-style), not first-only.

## FIRST STEP (the harness may start your worktree from main, which lacks the design)

Run `git checkout -b claude/spec-034-webp-parity claude/spec-034-design` (the local branch exists in this repository and holds the spec and this handoff). Confirm `git log --oneline -1` and that this file exists. Do not use `reset --hard`.

## BUDGET (hard)

- At most about **100 tool calls or 250k context**, with three checkpoints: (0) the carried SPEC-030 gaps: tests, seeds, pins, `just lint` and `just test`, COMMIT; (1) fixtures, `oracle_webp.rs`, `read_ours_webp`, recorder, records, replay and live tests, divergences, questions, warning table: gates, COMMIT; (2) the mutation oracle and its red-proofs: gates, COMMIT. If past about 60 percent of budget at checkpoint 1, hand back after it listing checkpoint 2 as the remainder.
- Read large files with offset/limit (`png_oracle.rs` is 1,500 lines and `oracle.rs` 1,200: read the PNG parts and the structure, not all of them). Prefer one write per file. The shape list is long: generate the fixtures from one table in `oracle_webp.rs`, not one function per shape.
- Record `tokens_total: null` with a note if you cannot measure it; the orchestrator replaces it with the harness figure. Never invent one.
- Commit on your branch. Do NOT push, do NOT open a PR, do NOT touch main. No new dependency.
- ExifTool 13.55 is installed; run it only through the repo's `run_tool_with_limit` helpers in tests; for exploratory probing use `perl -e 'alarm 8; exec @ARGV' -- exiftool ...` (macOS has no `timeout`). Never read or link ExifTool's source.
- Toolchain: `RUSTUP_TOOLCHAIN=1.90.0`, `PATH="$HOME/.cargo/bin:$PATH"`; read `guidance/toolchain-brief.md`. BSD `sed -i` rejects multi-expression use: use the edit tool or Python. `indexing_slicing` is denied crate-wide in `src/` (tests may index). `fuzz/Cargo.lock` is rewritten by fuzz builds: restore it with git, do not commit it. `tests/release_layout.rs` pins `[Unreleased]` in CHANGELOG empty: add no CHANGELOG entry. `just decisions-index` writes an untracked `decisions/INDEX.md`: never commit it. When scripting edits on spec files, a glob also matches the `-timeline.md` file: use exact names.

## Work items (in this order)

1. Read the pattern files and DEC-019; list the 20 selectors; decide the WebP region selectors (the spec says: non-empty `Xmp` region against `XMP-xmp:CreatorTool`, non-empty `Icc` region against `ICC-header:ProfileFileSignature`).
2. **Checkpoint 0**, the carried gaps (spec, Outputs). **COMMIT.**
3. **Tests first**: the fixture list and comparator tests in `tests/webp_oracle.rs` with `tests/support/oracle_webp.rs`; see them red for the right reason (no records yet).
4. Probe each hostile shape under the time limit; add the `oracle::` pieces, `record_webp`, the probe, the `.filter` skips; run `just oracle-record`; check idempotence (run twice, no diff).
5. Measure, then model: the several-block rule, the divergences (each a `Divergence` entry plus a `guidance/questions.yaml` entry; a missed class of divergence is the usual defect, so diff EVERY fixture, not only the ones you expect to differ); the warning table. **Gates, COMMIT (checkpoint 1).**
6. The mutated-copy red-proofs (at least seven library faults and two byte mutations; control first; injection marker not leaking). **Gates, COMMIT (checkpoint 2).**
7. Final gates: `just lint`, `just test`, `just mutation-oracle`, `just validate`, `just cost-audit`, `just forbid-unsafe`; `just oracle-record` twice with no diff; `just fuzz-smoke webp_riff` (60 s; fuzz needs nightly: see the toolchain brief) after the seed additions.

## Expected Deliverables

Commits on the branch; every acceptance criterion met or listed as not met with the reason; spec boxes ticked; the handback filled, including every divergence found (with its question id) and every choice made where the spec is silent. If parity exposes a reader bug, fix it with a regression test and say so loudly.

## Out of Scope

PNG parity (done), audit wiring (SPEC-032), decoding or validating XMP or ICC content in the library, any DEC change (raise it instead), JPEG, TIFF or PNG oracle behaviour changes, PR creation.

## Return Criteria

Fill the `handback:` front matter AND the `## Handback` section; set `handoff.status` and `handback.status` to `completed`. If you cannot complete: say exactly what is done and what blocked you, set both to `rejected`/`blocked`, and still report tokens.

---

## Handback

*Filled in by the receiving agent.*

### Execution notes

- **Branch / PR:** `claude/spec-034-webp-parity` (from `claude/spec-034-design`); three commits: carried gaps, parity apparatus, red-proofs. No push, no PR.
- **Completed at:** 2026-11-21
- **All acceptance criteria met?** Yes, with the items below. `just fuzz-smoke webp_riff` ran clean (see Deviations 6).

Fixtures: 98 (12 wrapped TIFFs plus 86 shapes from one table in `tests/support/oracle_webp.rs`), 53 seeds, records `webp-*.txt` plus `webp-warnings.txt`. Tests: 39 in `tests/webp_oracle.rs`, 8 library mutants, 2 byte mutations, control and marker guard in `tests/webp_mutation_oracle.rs`. Carried gaps (a) to (d) done in `tests/webp_walk.rs` (`first_vp8x_wins_flags_and_one_flag_mismatch`, `limits_are_pinned_literals`), seeds `vp8x-first-wins` and `vp8x-nine-bytes` (51 to 53; walk seeds 35 to 37; pins updated in `tests/fuzz_layout.rs`) and `webp_model_runs_on_every_walk_seed`.

Divergences (all pinned in `KNOWN_DIVERGENCES`, each with a `guidance/questions.yaml` entry, each required to still exist):
- `webp-iccp-profile-not-validated`: webp-icc-garbage, -stub-132, -header-only, -zero-tags (ICC selector; neither is wrong).
- `webp-xmp-content-not-parsed`: webp-xmp-garbage and webp-xmp-nul-id-1-bytes to -7-bytes (XMP selector; neither is wrong).
- `webp-riff-other-forms-read-by-exiftool`: webp-header-wave, webp-header-rifx, and webp-header-avi (a THIRD one beyond the spec: ExifTool reads `RIFF....AVI ` + `EXIF` too and prints Make; we are right, not a WebP).
No other fixture or seed differs (every fixture and all 53 seeds were diffed; the two "agrees" claims that matter: padding and `VP8X` shapes agree entirely).

Reader bugs: none found, `src/` unchanged.

Gates: `just lint` green; `just test` green (no failure line); `just mutation-oracle` green (58 + 39 + 40 tests); `just validate` green; `just cost-audit` green; `just forbid-unsafe` green; `just oracle-record` run repeatedly with no diff in tracked records (also by `git status` after two runs).

### Cost self-report

- **Tokens (total):** null (not measurable from inside the session)
- **Estimated USD:** null
- **Duration (minutes):** null
- **Source of the number:** estimate; the orchestrator replaces it with the harness figure. About 75 tool calls.

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:**
  1. Spec measured "trailing junk of 12 bytes" as warning "Error reading RIFF file (corrupted?)". The record says ExifTool is SILENT for 12 trailing bytes (`webp-tail-12-bytes`) and warns only for 3 bytes, a length past EOF and a partial header. The warning table pins the measured behaviour.
  2. Spec said the 12-byte header and 11-byte header give "no output"; measured, ExifTool prints an all-dash line (and Error "File format error" for 11 bytes), which the record parser accepts.
  3. Chunk-code case variants are named `webp-exif-kind-capital-e`, `-lower`, `-last-letter-lower` (macOS is case-insensitive: `Exif` and `exif` collided as record file names).
  4. Seeds are recorded and compared on the 20 Exif selectors only (regions are not compared on seeds), as the PNG seeds are; ExifTool runs on a `.webp` copy of each seed (the committed seed files have no extension), and the recorder does the same.
  5. The tests were red for the right reason only partly: I ran the recorder before the first test run (to see the records), so the "no records" red was demonstrated afterwards by removing one record (`cannot read ... webp-exif-basic.txt`), not seen first.
  6. `just fuzz-smoke webp_riff` ran clean: 2,113,087 runs in 61 s, no crash (`fuzz/Cargo.lock` restored with git).
  7. No wildcard divergence (`EXIF_UNREAD`-style) was needed: the three divergence classes are exact per-selector entries.
- **Follow-up work identified:** SPEC-032 (audit wiring) should say "located" for Icc and Xmp regions; the AVI divergence suggests the audit may meet RIFF AVI/WAV inputs that ExifTool reads.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — The spec's measured claims (12-byte tail warning, "no output" for short headers) differed from the records; saying "re-measure, never trust" up front was right.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — Case-insensitive filesystems: fixture names that differ only by case collide as record files.

3. **Is there a follow-up spec I should write now?**
   — Optional: a RIFF-form (WAV/AVI) decision, since ExifTool reads other RIFF forms; otherwise SPEC-032.
