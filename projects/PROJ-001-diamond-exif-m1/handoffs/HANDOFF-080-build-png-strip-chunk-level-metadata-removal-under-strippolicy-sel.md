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
  id: HANDOFF-080
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-22
  status: completed                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-037

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
  status: completed                # completed | blocked | rejected
  agent: claude-sonnet-5-5                      # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: null                    # verify handoffs: approved | punch-list | rejected
  basis: null                      # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: 253930               # REAL combined count — what cost-audit reads
  estimated_usd: 1.68              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: null
  branch: claude/spec-037-fixes
  pr: null
  completed_at: 2026-11-22               # YYYY-MM-DD
  notes: "Token count not exposed to the implementer; tokens_total left null for the orchestrator to measure from the transcript."                      # if unusual; double-quote it — an unquoted " #" starts a YAML comment
  synced_at: 2026-11-22
---

# HANDOFF-080: SPEC-037 verify punch list (fix round)

**FIRST STEP, before anything else:** `git checkout -b claude/spec-037-fixes claude/spec-037-verify`
(the harness may have started your worktree from main).

## Delegation Summary

`claude-opus-5-5` hands the SPEC-037 verify punch list to `claude-sonnet-5-5` for one
fix round. Read first: `AGENTS.md`, `guidance/toolchain-brief.md`, DEC-020 and
DEC-021 (with their SPEC-037 clarifications), `HANDOFF-079-*.md` (the verify
report: evidence, ExifTool output and the grid; scratch scripts under the
scratchpad `v037-grid/` if still present), `src/png/metadata.rs` (`chunk_class`),
`src/write/png.rs`, `tests/strip_png.rs`, the harness
`fuzz/shared/rewrite_png_invariants.rs`.

Toolchain: `export RUSTUP_TOOLCHAIN=1.90.0 PATH="$HOME/.cargo/bin:$PATH"` (lint on
the pinned 1.90.0); fuzz needs nightly; macOS has no `timeout`
(`perl -e 'alarm 8; exec @ARGV' --`); BSD `sed -i` needs `''`; Python for
multi-line edits. Never commit `decisions/INDEX.md`; restore `fuzz/Cargo.lock`
after fuzz runs. No CHANGELOG entry. Leave no background processes. Compare
against `origin/main`. **A record or fixture that stores raw tool output must not
contain machine-specific paths** (SPEC-036 CI lesson: the recorder normalises the
ExifTool install directory via `tools::strip_perl_lib_dir`).

## The punch list, in order. The architect decides these; do not re-decide.

1. **H1 (blocks shipping): a real library gap.** ExifTool 13.55 matches the chunk
   types `eXIf` and `zxIf` case-insensitively (all 32 spellings of each decode, as
   a bare TIFF or `\0` + length + zlib, with a "chunk should be eXIf" warning);
   `chunk_class` knows only `eXIf`, `zxIf`, `zxif`, `exif`. First re-measure to
   confirm the verifier's claim and to check EVERY other PNG metadata chunk type
   (`tXMP`, `meTa`, `seAl`, `caBX`, `tIME`, `iCCP`, `tEXt`, `zTXt`, `iTXt`, `dSIG`, ...)
   once more for case folding (the verifier found none, so a mismatch is a
   finding to report). Case-fold `exif` and `zxif` in `chunk_class` (ASCII,
   four bytes); a mixed-case `EXIF` chunk then falls under the carrier rule
   (removed by every preset, reported `unreadable`). Make this its own commit
   first. Correct DEC-020 clarification 5 and DEC-021's carrier table (chunk
   type bytes are case-sensitive EXCEPT these two), update the committed seed
   `carrier-case-sensitive` (it pins the wrong rule; rename and regenerate with
   `examples/fuzz_seeds.rs`), the harness model, the `tests/strip_png.rs` carrier
   table and every other test or record this moves; report each. Add a test that
   iterates all 32 spellings of both types x 2 encodings under all five presets
   and asserts the output carries none of them.
2. **M1: independence of facts.** Add a gated live test (`tests/strip_png_live.rs`,
   gated like its siblings with `DIAMOND_EXIF_REQUIRE_ORACLE`, replay-free:
   it only makes sense live): for the case-spelling grid, the carrier shapes and a
   dozen representative fixtures from `oracle_png`, strip under all five presets,
   run ExifTool (`-a -u -G1 -H -s`, 8 s limit) on the OUTPUT and fail if it still
   prints any tag the preset removes: for every preset GPS (`GPS` group tags,
   `Composite:GPSPosition`), `SerialNumber`, `OwnerName`, `Author`, `Artist`,
   `Copyright` under the presets that remove Authorship (as the preset table
   says), and any tag from a decoded carrier. State the tag list in the test and
   pin it. This is the cheap precursor of SPEC-039's formal I3; keep it small.
3. **M2: `caBX`.** Decision: keep the JPEG precedent (JUMBF is C2PA provenance, kept
   by `AllPii` and `All`, removed by `Everything`). State the residual honestly:
   README and DEC-021 say a C2PA manifest can carry Exif and creator assertions
   (measured: `stds.exif` GPS and `dc:creator` print from a `caBX`) that survive
   every preset except `Everything`, and that the preview shows `KeptUnread`
   for it. No code change; add a test that a `caBX` with such content is kept by
   `Gps` and removed by `Everything` and is `KeptUnread` in the preview.
4. **L2.** A test that runs ALL 61 `oracle_png::fixtures()` through `strip` under
   the five presets (typed refusals allowed, panics and `VerifyFailed` not) and,
   for the preset-by-preset outputs, passes the self-check; and widen the
   `GpsAndDevice` and `AllPii` seeds beyond three shapes (generated).
5. **L3.** The CLI/report preview reason text "the writer refuses this container"
   is false for PNG: change the wording for PNG to say the strip exists but has
   not been checked against ExifTool yet (`strippable` stays false); update the
   text/JSON/CSV goldens deliberately and say which changed. README audit
   paragraph: name the carrier and other-chunk regions. Orchestrator fixes L4
   (the HANDOFF-077 text) itself.
6. **L1** needs no change (over-removal is the safe direction); add one sentence
   to DEC-021 consequences naming it.
7. **Gates on the pinned 1.90.0:** `just lint`, `just test`, `just mutation-oracle`,
   `just forbid-unsafe`, `just validate`, `just cost-audit`, `just oracle-record`
   twice (no diff; no machine path in any record), `just fuzz-smoke rewrite_png`,
   `png_chunks`, `audit`, `audit_render` (60 s each, nightly). Commit on
   `claude/spec-037-fixes` with trailer
   `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`. No PR. Leave
   tokens null.

## Budget

About 100 tool calls or 250k context. If either is near, stop after the H1
commit and report what is done; do not loop.

## Out of Scope

WebP; the audit/CLI switch-over (`strippable` stays false); the formal ExifTool
write parity (SPEC-039); a CLI strip command.

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

- **Branch / PR:** claude/spec-037-fixes, no PR
- **Completed at:** 2026-11-22
- **All acceptance criteria met?** yes (H1, M1, M2, L1, L2, L3 done; see commits)
- **For `verify`:** the verdict — ✅ APPROVED (at commit SHA) / ⚠ PUNCH LIST / ❌ REJECTED

### Cost self-report

Mirror what you put in the `handback:` front-matter, and say where the number
came from. **This is the number that lands in the spec** — the orchestrator
transcribes it via `just handback-sync`, it does not estimate it.

- **Tokens (total):** 253930 (harness subagent_tokens)
- **Estimated USD:** 1.68
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
