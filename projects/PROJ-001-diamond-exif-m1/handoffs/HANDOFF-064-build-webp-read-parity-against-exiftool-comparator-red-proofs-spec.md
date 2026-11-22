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
  status: pending                  # pending | accepted | completed | rejected

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
  status: null                     # completed | blocked | rejected
  agent: null                      # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: null                    # verify handoffs: approved | punch-list | rejected
  basis: null                      # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: null               # REAL combined count — what cost-audit reads
  estimated_usd: null              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: null
  branch: null
  pr: null
  completed_at: null               # YYYY-MM-DD
  notes: null                      # if unusual; double-quote it — an unquoted " #" starts a YAML comment
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

- **Branch / PR:**
- **Completed at:**
- **All acceptance criteria met?**

### Cost self-report

- **Tokens (total):**
- **Estimated USD:**
- **Duration (minutes):**
- **Source of the number:**

### Drift and new artifacts

- **New decisions emitted:**
- **Deviations from spec:**
- **Follow-up work identified:**

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   —

2. **Was there a constraint or decision that should have been listed but wasn't?**
   —

3. **Is there a follow-up spec I should write now?**
   —
