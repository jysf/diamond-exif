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
  id: HANDOFF-069
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-22
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-032

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
# If your platform genuinely exposes NO token count, set tokens_total: 237190 AND
# write why in `notes` — then set `cost.metering_source: none` in
# .repo-context.yaml so the gate stops asking. Do not invent a number.
handback:
  status: completed                # completed | blocked | rejected
  agent: claude-sonnet-5-5         # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: null                    # verify handoffs: approved | punch-list | rejected
  basis: agent-result              # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: null               # REAL combined count — what cost-audit reads
  estimated_usd: 1.57              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: 21
  branch: claude/spec-032-audit-a
  pr: null
  completed_at: 2026-11-22         # YYYY-MM-DD
  notes: "Part A only; cost is the harness subagent_tokens figure recorded by the orchestrator."   # if unusual; double-quote it — an unquoted " #" starts a YAML comment
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-069: Audit wiring: PNG and WebP into exif-audit, PART A (model and writer)

**FIRST STEP, before anything else:** `git checkout -b claude/spec-032-audit-a claude/spec-032-design`
(the harness may have started your worktree from main, which lacks the spec and DEC-020).

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-032` to `claude-sonnet-5-5`
(implementer) for the **build** cycle, **Part A only**: the audit model, the
preview and the writer refusal, with their unit tests. Part B (report, CLI,
fuzz seeds, oracle replay, README) is a separate fresh handoff after Part A is
reviewed. Do not start Part B.

## Context the Receiving Agent Needs

Read first: `AGENTS.md`, `guidance/toolchain-brief.md`,
`decisions/DEC-020-*.md` (the design: follow it), the spec
`projects/PROJ-001-diamond-exif-m1/specs/SPEC-032-*.md` (Goal, Outputs,
Acceptance Criteria, Notes), DEC-008, DEC-009, DEC-018, DEC-019.

Toolchain: `export RUSTUP_TOOLCHAIN=1.90.0 PATH="$HOME/.cargo/bin:$PATH"`.
macOS has no `timeout`: `perl -e 'alarm 8; exec @ARGV' -- exiftool ...`. BSD
`sed -i` needs `''`; use Python for multi-line edits. `indexing_slicing` is
denied in `src/`; no panics, `#![forbid(unsafe_code)]`. Never commit
`decisions/INDEX.md` or `fuzz/Cargo.lock` (restore the latter). No CHANGELOG
entry.

## Part A, in order

1. **Measure the keyword rule.** Build PNGs with `tEXt` chunks for `Author`,
   `author`, `AUTHOR`, `aUTHOR`, `Copyright`, `copyright`, `Creation Time`,
   `creation time`, `Software`, `Title`, `Warning`; run ExifTool
   (`-a -G1 -s -n`) and note which keyword spellings ExifTool maps to the
   standard tag. Pin that rule (exact vs first-letter-case) as the match rule
   in `text_keyword_category`, with a comment citing the measurement. Also
   confirm for `iTXt` and `zTXt` (compressed) keywords. Report the rule.
2. **Kinds and dispatch.** `ContainerKind::{Png, Webp}` and
   `#[non_exhaustive]`; `audit()` dispatch; fix every match the compiler flags
   (`src/report`, `src/write`) minimally (Part B finishes the renderers: keep
   the build green, behaviour for the old kinds unchanged).
3. **`audit_png`, `audit_webp`.** Per DEC-020: all Exif blocks via
   `exif_blocks()` with block index and chunk index; regions as items and
   findings; `RegionKind::{Text, RawProfile, Time}`; PNG `Trailer`; length-0
   region has an item and no finding; `Origin::after_scan` = after the image
   data; health counts the container issues (`png_issues` / `webp_issues`)
   and the Exif issues like `audit_jpeg`; an `Err` parse is `unreadable`.
   `tIME` is not a region in `png::read`: find it from `png().chunks()` (type
   bytes `tIME`, length 7) in the audit.
4. **Categories.** `region_category` rows and the data table
   `text_keyword_category`; unit test row by row.
5. **Preview.** `Preview::strippable()`; PNG and WebP: all findings `Kept`,
   `removed == 0`, `reliable()` false, for every `StripPolicy`; JPEG and TIFF
   output identical (existing tests unchanged and green).
6. **Writer.** `write::strip` refuses `Png` and `Webp` with
   `StripError::Unsupported { kind: ContainerKind::Png|Webp }`; a test for
   every policy, both formats, no bytes.
7. **Tests** from the spec's Failing Tests (the `src/audit`, `preview` and
   `write` ones), using the existing fixture builders (e.g.
   `tests/support/*_shapes.rs` via the same paths other `src` tests use; if a
   `src` unit test cannot reach `tests/support`, put the shape tests in an
   integration test `tests/audit_png_webp.rs` that imports the builders as the
   oracle tests do). Include: two-block PNG and WebP; 17-block WebP with GPS
   in the 17th is not clean; each region kind; hostile inputs return without a
   panic (truncated, empty, garbage after the signature).
8. **Gates.** `just lint`, `just test`, `just forbid-unsafe`, `just validate`,
   `just cost-audit`. Commit on `claude/spec-032-audit-a` with trailer
   `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`. Do not open a
   PR. Do not tick Part B boxes; tick only the Part A ones, and write in the
   handback which acceptance criteria remain for Part B.

## Budget

About 100 tool calls or 250k context. Stop and hand back if either is near;
do not loop on a failing test, report it.

## Out of Scope

Report, CLI, fuzz seeds, oracle replay and live tests, README (Part B);
inventory parity (SPEC-036, not yet created); new `Category` or `Disposition`
variants; strip support; any `src/png` or `src/webp` change (if you think one
is needed, stop and report why).

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

- **Branch / PR:** `claude/spec-032-audit-a` (no PR opened, by instruction)
- **Completed at:** 2026-11-22
- **All acceptance criteria met?** Part A criteria: yes (see the report). Part B criteria remain: CLI, renderers' "not strippable" wording and JSON `strippable`, fuzz seeds and the independent harness re-derivation, the oracle replay and `png-text-keywords` record, README, `just mutation-oracle`, `fuzz-smoke`, `oracle-record`.
- **For `verify`:** the verdict — ✅ APPROVED (at commit SHA) / ⚠ PUNCH LIST / ❌ REJECTED

### Cost self-report

Mirror what you put in the `handback:` front-matter, and say where the number
came from. **This is the number that lands in the spec** — the orchestrator
transcribes it via `just handback-sync`, it does not estimate it.

- **Tokens (total):** 237190 (harness subagent_tokens)
- **Estimated USD:** 1.57 (6.60 USD per Mtok)
- **Duration (minutes):** 21
- **Source of the number:** harness subagent_tokens, recorded by the orchestrator

### Drift and new artifacts

- **New decisions emitted:**
  - none
- **Deviations from spec:**
  - Keyword rule measured (ExifTool 13.55, `-a -G1 -s -n`, tEXt, zTXt, iTXt, compressed iTXt): ExifTool uppercases the FIRST letter of the keyword and then compares exactly. `author`, `copyright`, `software`, `warning`, `disclaimer`, `creation Time` map to the standard tags; `AUTHOR`, `aUTHOR`, `COPYRIGHT`, `SOFTWARE`, `creation time`, `Creation time`, `CREATION TIME` are their own auto-named tags (`Creation time` prints `CreationTime` but `-d` does not reformat it, so the printed name alone does not prove a standard tag). Pinned in `text_keyword_category`.
  - A bare `RIFF....WEBP` header with no chunks is a clean audit (the library reports no issue); the hostile test uses a header plus a truncated chunk head.
  - Fuzz harness stopgap (outside the handoff's Part A list, needed to keep `just test` green): `fuzz/shared/audit_invariants.rs` `rederive` returns the library's own facts for Png and Webp (circular, commented STOPGAP); Part B replaces it with an independent re-derivation. The `png()` stand-ins in `tests/cli.rs`, `tests/report_formats.rs`, `tests/report_render.rs` (+ three golden files), the two `render-*` png seeds (generator `examples/fuzz_seeds.rs`) and four plant inputs in `tests/fuzz_layout.rs` were PNG signatures used as "an unsupported file"; they are now GIF headers or BigTIFF so they stay unsupported.
  - Part A tests for preview and writer live in `tests/audit_png_webp.rs` (integration) rather than in `src/audit/preview.rs` and `src/write`, as the handoff allows; the keyword and region-kind tables are unit-tested in `src/audit/category.rs`.
- **Follow-up work identified:**
  - none beyond Part B

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Whether Part A had to keep the fuzz harness and the CLI/report tests green: they use PNG signatures as the unsupported stand-in, and the handoff did not say how to treat them.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — The mutation oracle's exactly-once needles in `src/audit/mod.rs` (a line copied into audit_png/audit_webp broke one); worth listing.

3. **If you did this task again, what would you do differently?**
   — Grep for every PNG-signature stand-in in tests and seeds first, and decide the unsupported replacement once.
