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
  id: HANDOFF-073
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-22
  status: completed                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-036

project:
  id: PROJ-001
  stage: STAGE-010
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
  status: completed                     # completed | blocked | rejected
  agent: claude-sonnet-5-5                      # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: null                    # verify handoffs: approved | punch-list | rejected
  basis: agent-result                      # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: 194639               # REAL combined count — what cost-audit reads
  estimated_usd: 1.28              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: null
  branch: claude/spec-036-part-a
  pr: null
  completed_at: 2026-11-22               # YYYY-MM-DD
  notes: null                      # if unusual; double-quote it — an unquoted " #" starts a YAML comment
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-073: Tag-level inventory parity for PNG and WebP, PART A (comparator, corpus, recorder, records)

**FIRST STEP, before anything else:** `git checkout -b claude/spec-036-part-a claude/spec-036-design`
(the harness may have started your worktree from main, which lacks the spec).

## Delegation Summary

`claude-opus-5-5` hands `SPEC-036` Part A to `claude-sonnet-5-5`: the comparator
classification, the new `png-webp` corpus, the recorder and the committed
record, plus the measured table of what ExifTool prints. Part B (replay, live
and divergence tests, red-proofs, questions, docs) is a separate fresh handoff
after Part A is reviewed. Do not start Part B. Test harness only: no `src/`
change (if you find an audit bug, stop and report it).

## Context

Read: `AGENTS.md`, `guidance/toolchain-brief.md`, the spec
`projects/PROJ-001-diamond-exif-m1/specs/SPEC-036-*.md` (Context, Goal,
Outputs, Notes), `decisions/DEC-020-*.md`, `tests/support/inventory.rs`,
`inventory_corpus.rs`, the SPEC-019 `real-shapes` section of
`tests/inventory_oracle.rs` and of `examples/oracle_record.rs` (your template).

Toolchain: `export RUSTUP_TOOLCHAIN=1.90.0 PATH="$HOME/.cargo/bin:$PATH"`;
macOS has no `timeout`: `perl -e 'alarm 8; exec @ARGV' -- exiftool ...`; BSD
`sed -i` needs `''`; Python for multi-line edits. Never commit
`decisions/INDEX.md` or `fuzz/Cargo.lock`. No CHANGELOG entry. Lint under the
pinned 1.90.0 (clippy here differs from CI).

## Part A, in order

1. **Corpus.** `png-webp` in `NEW_CORPORA` (not in `CORPORA`: the existing five
   records, counts and results must not move). Inputs, in this order, deduped by
   bytes as the existing `distinct` does: every PNG fixture and the
   `png-text-keywords` table from `tests/support/oracle_png.rs`; every WebP
   fixture from `oracle_webp.rs`; the PNG and WebP shapes in `png_shapes.rs` and
   `webp_shapes.rs` (if present); the committed seeds of `fuzz/seeds/png_chunks`
   and `fuzz/seeds/webp_riff`; the PNG/WebP files of `fuzz/seeds/audit` that
   SPEC-032 excluded from the old corpora (read
   `tests/support/inventory_corpus.rs` to see how it excludes them, and keep
   `audit-png-magic` where it is). Input extension by corpus: `png` for PNG,
   `webp` for RIFF/WEBP bytes, and the existing `jpg`/`tif` rule untouched for the
   old corpora. Pin the input counts.
2. **Measure first.** Write the recorder to run ExifTool
   (`-a -u -G1 -H -s`, under the 8 s limit) on every input and, BEFORE changing
   the classifier, produce the table of every distinct `(group, id-or-dash)`
   ExifTool prints across the corpus with how many inputs carry each (put it in
   your handback and commit it as a comment-free text file under the scratchpad
   only, not the repo). Known so far: `PNG` with hex ids 0x0000 to 0x000c (IHDR),
   `PNG` with id `-` (text and `tIME`), `PNG-<chunk>` groups (e.g. `PNG-pHYs`),
   `RIFF` (VP8X/VP8 structure), `IFD0`/`ExifIFD`/`GPS`/`InteropIFD`/`IFD1`, XMP and
   ICC groups. A group not in that list is a finding: report it.
3. **Classifier.** In `tests/support/inventory.rs` (no new `use` of other support
   modules: the file is copied next to the probe crate): `PNG` hex-id lines and
   `PNG-*` and `RIFF` groups are structural and dropped BY NAME via a constant;
   `PNG` id-less lines are counted into a new `text_tags` field of `Inventory`
   (our side: the audit's `Text` and `Time` regions); XMP and ICC groups map to
   region presence for these formats as for JPEG; an unknown group stays a
   `ParseError`. The record format (`probe_text`, `parse_probe_text`, section
   text) gains the count in a backward-compatible way: the five old records
   must parse and replay byte-identically. Per-format block rule for `ours`: PNG
   only the first `eXIf` block's tag items (`Origin::block == Some(0)`); WebP
   every block. Unit tests for the classifier cases.
4. **Recorder.** `examples/oracle_record.rs` records the corpus to
   `tests/oracle-fixtures/exiftool-inventory/png-webp.txt` (idempotent; header
   with the version and command like the others). Run `just oracle-record` twice:
   the second run changes nothing and no OTHER record changes.
5. **Raw differences.** Write (in the handback, not the repo) the list of inputs
   where the audit's inventory differs from ExifTool's under the new classifier,
   each with the exact multiset difference and your reading of the cause, grouped
   by cause. Do NOT pin divergences or write `questions.yaml` entries: Part B.
6. **Gates (1.90.0):** `just lint`, `just test` (the old inventory tests must be
   unchanged and green), `just forbid-unsafe`, `just validate`, `just cost-audit`.
   Commit on `claude/spec-036-part-a` with trailer
   `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`. No PR. Leave
   tokens null in the handback (the orchestrator records the harness figure).

## Budget

About 100 tool calls or 250k context. Stop and hand back what is done if either
is near; do not loop.

## Out of Scope

Part B items; `src/`; value comparison; the five existing corpora.

## Return Criteria — how to hand back

**Completing this handoff means filling in the `handback:` front-matter block
above AND the `## Handback` section below.** An unfilled handback is an
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

- **Tokens (total):** 194639 (harness subagent_tokens)
- **Estimated USD:** 1.28
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
