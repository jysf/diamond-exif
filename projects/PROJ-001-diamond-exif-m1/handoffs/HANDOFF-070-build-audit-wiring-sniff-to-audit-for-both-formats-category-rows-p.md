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
  id: HANDOFF-070
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-22
  status: pending                  # pending | accepted | completed | rejected

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

# HANDOFF-070: Audit wiring: PNG and WebP into exif-audit, PART B (CLI, renderers, fuzz, oracle replay, README)

**FIRST STEP, before anything else:** `git checkout -b claude/spec-032-audit-b claude/spec-032-design`
(the design branch already contains Part A, commit `3279e13` and its cost
commits; the harness may have started your worktree from main, which does not).

## Delegation Summary

`claude-opus-5-5` hands `SPEC-032` Part B to `claude-sonnet-5-5`: everything in
the spec that Part A (HANDOFF-069) did not do. Read HANDOFF-069's Handback
first: it lists what Part A changed outside its scope and what it left.

## Context the Receiving Agent Needs

Read: `AGENTS.md`, `guidance/toolchain-brief.md`, DEC-020, the spec (Outputs,
Acceptance Criteria; Part A boxes are ticked), HANDOFF-069 Handback,
`tests/audit_png_webp.rs` (Part A tests: reuse its fixture helpers), DEC-010
(report schema), DEC-016 or whichever DEC governs the fuzz harness invariants
(`grep -l audit_invariants decisions/*`).

Toolchain: `export RUSTUP_TOOLCHAIN=1.90.0 PATH="$HOME/.cargo/bin:$PATH"`;
fuzz needs nightly; macOS has no `timeout` (`perl -e 'alarm 8; exec @ARGV' --`);
BSD `sed -i` needs `''`; Python for multi-line edits. No panics in `src/`,
`indexing_slicing` denied there. Never commit `decisions/INDEX.md` or
`fuzz/Cargo.lock`. No CHANGELOG entry. No `src/png` or `src/webp` change.

## Part B, in order

1. **Renderers and report.** Text, JSON, CSV for `ContainerKind::Png|Webp`:
   container names, status (`Unsupported` only for HEIC, GIF, BigTIFF, empty,
   unknown), a "not strippable" statement in the text preview and an additive
   JSON `strippable` boolean (never remove or rename an existing key; update
   the schema doc and golden files deliberately and say which changed), regions
   described as "located" not "valid". New golden or shape tests for a PNG and a
   WebP report in all three formats.
2. **CLI gate and exit codes.** `src/bin/exif-audit/read.rs` already takes
   `Png|Webp` after Part A (verify); CLI tests in `tests/cli.rs`: a PNG with
   GPS in `eXIf` gives the findings exit code, a clean PNG is `ok`, a WebP with
   unreadable metadata is `issues`, HEIC is still `unsupported`; size caps
   unchanged; no new flag.
3. **Fuzz.** Replace the Part A STOPGAP in `fuzz/shared/audit_invariants.rs`
   (`rederive` returns the library's own facts for Png and Webp: circular) with
   an independent re-derivation of the invariants for both formats, from the
   chunk or RIFF walk, as JPEG's is. Add PNG and WebP seeds to the `audit` and
   `audit_render` seed sets (copy from `fuzz/seeds/png_chunks` and `webp_riff`,
   through the existing generator `examples/fuzz_seeds.rs` or table, not by
   hand), restore the two render seeds Part A turned into GIF headers to
   meaningful PNG ones if the harness now allows, update `tests/fuzz_layout.rs`
   pins (and put back the `audit-png-magic` plants if they are still the right
   test). Run `just fuzz-smoke audit` and `just fuzz-smoke audit_render`
   (60 s, nightly) clean.
4. **Keyword record.** A fixture `png-text-keywords` (one `tEXt` chunk per
   keyword: Title, Author, author, AUTHOR, aUTHOR, Description, Copyright,
   copyright, Creation Time, creation time, Software, Disclaimer, Warning,
   Source, Comment, an unknown key) built by the one fixture table in
   `tests/support/oracle_png.rs`, recorded through `just oracle-record`
   (`-a -G1 -s -n`, the PNG group only is fine). A test asserts the audit's
   keyword categories agree with the record by name mapping (Author,
   Copyright to Authorship; CreationTime to Timestamps; Software to Software;
   the rest FreeText). The measured rule from Part A (ExifTool uppercases the
   keyword's first letter then compares exactly) is the pinned model; a keyword
   whose printed name differs from the model is a recorded divergence with a
   `questions.yaml` id, never silent. Replay + perturb red-proof (name the
   perturbed keyword).
5. **Category-presence replay.** New `tests/audit_png_webp_oracle.rs`: for
   every PNG and WebP oracle fixture with a committed record
   (`tests/oracle-fixtures/exiftool/png-*.txt`, `webp-*.txt`), the audit has a
   Gps / Xmp / Icc finding iff ExifTool printed the matching selector
   (`GPS*`, the XMP selector, the ICC selector), except recorded divergences
   (reuse `KNOWN_DIVERGENCES` ids for `png-*` and `webp-*`; a case you cannot
   explain stops you: report it). Replay always; live against ExifTool gated
   like the siblings (`DIAMOND_EXIF_REQUIRE_ORACLE`). A perturbed record makes
   the replay name the fixture (shown red).
6. **README** (the two "still reports ... as unsupported" sentences, the
   "unsupported file (a PNG, say)" sentence in the exit-code section, the
   verification line) and `guidance/questions.yaml` for any new divergence.
7. **Gates.** `just lint`, `just test`, `just mutation-oracle`,
   `just forbid-unsafe`, `just validate`, `just cost-audit`, `just oracle-record`
   twice (second run no diff), both fuzz smokes. Tick the remaining acceptance
   boxes. Commit on `claude/spec-032-audit-b` with trailer
   `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`. No PR. In the
   handback leave tokens null: the orchestrator records the harness figure.

## Budget

About 100 tool calls or 250k context. Stop and hand back what is done if
either is near; do not loop.

## Out of Scope

SPEC-036 (tag-level inventory parity); `src/png`, `src/webp`; new
`Category`/`Disposition`; strip; new CLI flags; a release.

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
