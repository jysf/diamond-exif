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
  id: HANDOFF-060
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-21
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-031

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

# HANDOFF-060: Build SPEC-031 PNG read parity against ExifTool

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-031` to `claude-sonnet-5-5` (implementer) for the **build** cycle. The spec is the contract: read it whole first (`projects/PROJ-001-diamond-exif-m1/specs/SPEC-031-png-and-webp-read-parity-against-exiftool-comparator-red-pro.md`). This is a test-and-evidence spec that mirrors SPEC-011 (JPEG parity): the pattern is `tests/jpeg_oracle.rs`, `tests/support/oracle_jpeg.rs`, `tests/support/oracle.rs`, `tests/support/tools.rs`, `examples/oracle_record.rs`, `tests/mutation_oracle.rs`, `tests/support/probe.rs`. Read them before writing anything; do not reinvent. DEC-017 and DEC-018 describe the readers under test; their ExifTool claims are leads, not evidence: the committed records are the evidence.

## FIRST STEP (the harness may start your worktree from main, which lacks the design)

Run `git checkout -b claude/spec-031-png-parity claude/spec-031-design` (the local branch exists in this repository and holds the spec and this handoff). Confirm `git log --oneline -1` and that this file exists. Do not use `reset --hard`.

## BUDGET (hard)

- At most about **100 tool calls or 250k context**, with two checkpoints: (1) fixtures, `oracle_png.rs`, recorder, records, replay and live tests, divergences, questions, warning table: gates, COMMIT; (2) the mutation oracle and its red-proofs: gates, COMMIT. If past about 60 percent of budget at checkpoint 1, hand back after it listing checkpoint 2 as the remainder.
- Read large files with offset/limit (the JPEG oracle is 1,540 lines: read its structure and the pieces you mirror, not all of it). Prefer one write per file.
- Record `tokens_total: null` with a note if you cannot measure it; the orchestrator replaces it with the harness figure. Never invent one.
- Commit on your branch. Do NOT push, do NOT open a PR, do NOT touch main. No new dependency.
- ExifTool 13.55 is installed; run it only through the repo's `run_tool_with_limit` helpers in tests; for exploratory probing use a timeout-safe approach (macOS has no `timeout`: use `perl -e 'alarm N; exec @ARGV'` or the Rust helper). Never read or link ExifTool's source.
- Toolchain: `RUSTUP_TOOLCHAIN=1.90.0`, `PATH="$HOME/.cargo/bin:$PATH"`; read `guidance/toolchain-brief.md`. BSD `sed -i` rejects multi-expression use: use the edit tool or Python. `indexing_slicing` is denied crate-wide in `src/` (tests may index). `fuzz/Cargo.lock` is rewritten by fuzz builds: restore it, do not commit it. `tests/release_layout.rs` pins `[Unreleased]` in CHANGELOG empty: add no CHANGELOG entry. When scripting edits on spec files, the glob also matches `-timeline.md`: use exact names.

## Work items (in this order)

1. Read the pattern files; list the 20 selectors and the region selectors; decide the PNG region selectors from measurement.
2. **Tests first**: the fixture list and comparator tests in `tests/png_oracle.rs` with `tests/support/oracle_png.rs`; see them red for the right reason (no records yet).
3. Probe each hostile shape under the time limit; extend `oracle::` (`read_ours_png`, PNG region selectors, `compare_png`), `examples/oracle_record.rs`, `tools.rs` if needed; run `just oracle-record`; check idempotence (run twice, no diff).
4. Measure then model: the several-Exif rule, raw-profile and compressed-XMP divergences with `Divergence` entries and `guidance/questions.yaml` entries; the warning table.
5. **Gates, COMMIT (checkpoint 1).**
6. The mutated-copy red-proofs (at least six library faults and two byte mutations; control first; injection marker not leaking). **Gates, COMMIT (checkpoint 2).**
7. Final gates: `just lint`, `just test`, `just mutation-oracle`, `just validate`, `just cost-audit`, `just forbid-unsafe`; `just oracle-record` twice with no diff.

## Expected Deliverables

Commits on the branch; every acceptance criterion met or listed as not met with the reason; spec ticks; the handback filled, including every divergence found (with its question id) and every choice made where the spec is silent. If parity exposes a reader bug, fix it with a regression test and say so loudly.

## Out of Scope

WebP parity (SPEC-034), audit wiring (SPEC-032), decoding or inflating in the library, any DEC change (raise it instead), JPEG or TIFF oracle behaviour changes, PR creation.

## Return Criteria

Fill the `handback:` front matter AND the `## Handback` section; set `handoff.status` and `handback.status` to `completed`. If you cannot complete: say exactly what is done and what blocked you, set both to `rejected`/`blocked`, and still report tokens.

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

- **Branch / PR:** [link]
- **Completed at:** YYYY-MM-DD
- **All acceptance criteria met?** yes/no (if no, explain)
- **For `verify`:** the verdict — ✅ APPROVED (at commit SHA) / ⚠ PUNCH LIST / ❌ REJECTED

### Cost self-report

Mirror what you put in the `handback:` front-matter, and say where the number
came from. **This is the number that lands in the spec** — the orchestrator
transcribes it via `just handback-sync`, it does not estimate it.

- **Tokens (total):** <real number, or null + why>
- **Estimated USD:** <number, or null>
- **Duration (minutes):** <estimate>
- **Source of the number:** `/cost` | API `usage` | harness report | none available

### Drift and new artifacts

- **New decisions emitted:**
  - `DEC-NNN` — <title> (if any)
- **Deviations from spec:**
  - [list]
- **Follow-up work identified:**
  - [any new specs that should be added to the stage's backlog]

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — <answer>

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — <answer>

3. **If you did this task again, what would you do differently?**
   — <answer>
