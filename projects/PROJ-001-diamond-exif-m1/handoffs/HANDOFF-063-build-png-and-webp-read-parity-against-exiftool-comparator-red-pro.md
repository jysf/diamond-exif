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
  id: HANDOFF-063
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-21
  status: completed                # pending | accepted | completed | rejected

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
  status: completed
  agent: claude-sonnet-5-5
  verdict: null
  basis: estimate
  tokens_total: 168851              # harness subagent_tokens
  estimated_usd: 1.11
  duration_minutes: null
  branch: claude/spec-031-fixes
  pr: null
  completed_at: 2026-11-21
  notes: "Token count not exposed to this subagent; the orchestrator replaces it with the harness figure. About 55 tool calls."
  synced_at: 2026-11-21
---

# HANDOFF-063: Build SPEC-031 follow-up 2: empty text keyword and verify-2 Lows

## Delegation Summary

`claude-opus-5-5` (architect) hands a NARROW follow-up of `SPEC-031` (`projects/PROJ-001-diamond-exif-m1/specs/SPEC-031-png-and-webp-read-parity-against-exiftool-comparator-red-pro.md`) to a fresh `claude-sonnet-5-5` implementer. The PNG parity build is done and passed two cold verify rounds on all but one finding: round 2 (read the Findings section of `projects/PROJ-001-diamond-exif-m1/handoffs/HANDOFF-062-verify-*.md` first) found ONE Medium and six Lows. Fix them as listed; do nothing else. Do not re-open earlier decisions.

## FIRST STEP

Run `git checkout -b claude/spec-031-fixes claude/spec-031-png-parity` (the local branch exists; tip is the commit that holds this handoff; do not use `reset --hard`). Confirm `git log --oneline -1` and that this file exists.

## BUDGET (hard)

About 50 tool calls or 150k context. Record `tokens_total: null` with a note if you cannot measure it; the orchestrator replaces it with the harness figure. Commit on your branch; do NOT push, do NOT open a PR, do NOT touch main. No new dependency. Toolchain: `RUSTUP_TOOLCHAIN=1.90.0`, `PATH="$HOME/.cargo/bin:$PATH"`; read `guidance/toolchain-brief.md`. ExifTool 13.55 is installed: run it only under `perl -e 'alarm 8; exec @ARGV' -- exiftool ...` or the repo helpers. BSD `sed -i` rejects multi-expression use: use the edit tool or Python. Restore `fuzz/Cargo.lock` after any fuzz build; add no CHANGELOG entry. Do not start background processes you do not wait for.

## Work items

1. **Medium (M1): a text chunk with an EMPTY keyword.** ExifTool shows the value of `tEXt` (`\0John Doe`), `zTXt` (`\0\0` + zlib stream) and `iTXt` (`\0\0\0\0\0secret value`) with an empty keyword (as a tag named `Tag`); we record `BadTextChunk` and locate nothing (`src/png/metadata.rs` around :375, DEC-018 line 56, pinned by `tests/png_metadata.rs` around :299 and `fuzz/shared/png_metadata_invariants.rs` around :139). First re-measure all three plus the compressed and the language-tagged `iTXt` variants. Then make the reader follow the measurement: locate a `Text` region (same routing as any keyword; an empty keyword cannot be XMP or a raw profile) AND record `BadTextChunk`, the same rule DEC-018 already applies to a keyword of 80 bytes or more. No NUL at all and a truncated `iTXt` header stay `BadTextChunk` with no region unless the measurement shows ExifTool reads those too (the verifier saw an `iTXt` that ExifTool exposes as raw binary `InternationalText`: if that is clear-cut, say so in the handback; do not widen the rule beyond what you measure).
2. Update tests (`tests/png_metadata.rs`), the fuzz model (`fuzz/shared/png_metadata_invariants.rs`, from raw bytes, both directions), the located-bytes test, fixtures in `tests/support/oracle_png.rs` (`png-text-empty-keyword`, `-ztxt`, `-itxt`), records via `just oracle-record` (idempotent; only expected records change or are added).
3. **L2:** correct DEC-018 line about the no-NUL `iCCP`: ExifTool prints no name for a true no-NUL `iCCP`, only a 14-byte `ICC_Profile` binary (a no-NUL chunk like `diamondABCDEFG`); the code is right. **You may edit DEC-018 for this sentence and to append a short "Clarifications from the SPEC-031 verify, round 2" line for the empty keyword and for L1 below; nothing else.**
4. **L1 (document only):** chunks we cannot parse (an `iTXt` whose header runs to the end of the chunk; an `iCCP` with no method byte, for which ExifTool prints `PNG:ProfileName`) are still exposed by ExifTool as raw binary or a name. State in DEC-018 that such chunks yield a `BadTextChunk`/`BadIccpChunk` issue and no region, and that consumers (SPEC-032) must treat any such issue as "metadata present that we could not read". Do not change the reader for L1.
5. **L3:** add an ICC-header selector to the PNG record set (for example `ICC-header:ProfileFileSignature`) so that a record pins that ExifTool reads the ICC when the name is empty; regenerate records.
6. **L4:** add seeds to `fuzz/seeds/png_chunks`: an 80-byte `iCCP` name; a language-tagged `iTXt` XMP; an empty-keyword `tEXt`; update the seed pins in `tests/fuzz_layout.rs`. Every existing seed stays byte-identical.
7. **L6:** make the located-bytes test also compare each region's chunk offset against the builder's expected offset (extend `expected_regions()` in `oracle_png.rs`, independent of `src/`).
8. L5 needs no change (warning-only differences): add one sentence to the `png-` warning-table doc listing them.
9. **Gates:** `just lint`, `just test`, `just mutation-oracle`, `just validate`, `just cost-audit`, `just forbid-unsafe`, `just oracle-record` twice with no diff, and `just fuzz-smoke png_chunks` (60 s). Then COMMIT.

## Out of Scope

Anything not listed, WebP, audit wiring, JPEG or TIFF changes, other DEC edits, PR creation.

## Return Criteria

Fill the `handback:` front matter AND the `## Handback` section (what you measured, what changed, which seeds changed result, anything surprising); set `handoff.status` and `handback.status` to `completed`; if blocked, say exactly what and set `rejected`/`blocked`.

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

- **Branch / PR:** `claude/spec-031-fixes` (from `claude/spec-031-png-parity`), no PR.
- **Completed at:** 2026-11-21
- **All acceptance criteria met?** yes. Measured with ExifTool 13.55: an empty keyword in `tEXt` (`\0John Doe`), `zTXt`, `iTXt` (flag 0, flag 1 compressed, and language-tagged `en`/`tr`) is shown as a tag named `Tag` with the value. A `tEXt` with no NUL, an empty `tEXt`, an `iTXt` cut after the flags or after the language tag, and a `zTXt` with no method byte are all exposed as raw binary (`TextualData`, `InternationalText`, `CompressedText`): clear-cut, but I did not widen the rule (L1: documented only). ICC: `ICC-header:ProfileFileSignature` is printed for valid profiles of any name length, an 80-byte name and an empty name included; NOT printed for a bad-header profile (20 zero bytes), nor for a no-NUL or no-method-byte `iCCP`.
- **What changed:** reader (`src/png/metadata.rs`): an empty keyword is located as `Text` and records `BadTextChunk`; docs in `src/error.rs`. Tests, fuzz model (both directions), fixtures `png-text-empty-keyword{,-ztxt,-itxt}`, `Expect.offset` (L6), a located-bytes test for offsets, a third PNG selector `ICC-header:ProfileFileSignature` (L3) with one new pinned divergence (`png-iccp-bad-header`, question `png-iccp-profile-not-validated` added to `guidance/questions.yaml`, which the divergence rules require), the L5 sentence on the warning table, DEC-018 edits (L2, L1, empty keyword). Records regenerated (every PNG record gains one column; three new fixtures and four new seed records).
- **Seeds:** four added (`meta-iccp-name-80`, `meta-itxt-xmp-lang`, `meta-ztxt-empty-keyword`, `meta-itxt-empty-keyword`; the empty-keyword `tEXt` already existed as `meta-text-empty-keyword`); all 68 older seeds are byte-identical. Only `meta-text-empty-keyword` changed parse result (now one `Text` region plus the `BadTextChunk` issue, was the issue only). No other existing seed changed result.
- **Gates:** `just lint`, `just lint-fuzz`, `just test`, `just mutation-oracle`, `just validate`, `just cost-audit`, `just forbid-unsafe` pass; `just oracle-record` run twice, second run no diff; `just fuzz-smoke png_chunks` 60 s clean (about 1.87M runs). `fuzz/Cargo.lock` restored.

### Cost self-report

Mirror what you put in the `handback:` front-matter, and say where the number
came from. **This is the number that lands in the spec** — the orchestrator
transcribes it via `just handback-sync`, it does not estimate it.

- **Tokens (total):** null (not exposed to this subagent)
- **Estimated USD:** null
- **Duration (minutes):** not measured
- **Source of the number:** none available

### Drift and new artifacts

- **New decisions emitted:** none (DEC-018 clarified).
- **Deviations from spec:** a `guidance/questions.yaml` entry was added (not in the handoff) because every pinned divergence must have one; two extra seeds (`zTXt`, `iTXt` empty keyword) instead of a second empty-keyword `tEXt`, which already existed.
- **Follow-up work identified:** none.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — That the ICC-header selector must be pinned as a divergence for the bad-header fixture, and that an empty-keyword `tEXt` seed already existed.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — The rule that each divergence needs a `questions.yaml` entry.

3. **If you did this task again, what would you do differently?**
   — Probe the ICC selector on all `iCCP` fixtures before writing code, as I finally did.
