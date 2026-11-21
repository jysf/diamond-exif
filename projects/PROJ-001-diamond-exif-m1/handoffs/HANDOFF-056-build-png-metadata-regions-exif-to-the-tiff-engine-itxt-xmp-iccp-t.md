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
  id: HANDOFF-056
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-21
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-029

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
  basis: agent-result                  # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: 189733              # harness subagent_tokens
  estimated_usd: 1.25              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: 11
  branch: claude/spec-029-png-metadata
  pr: null
  completed_at: 2026-11-21         # YYYY-MM-DD
  notes: "tokens_total left null: the subagent harness exposes no token count to the agent; the orchestrator replaces it with the harness figure. Not pushed, no PR."  # if unusual; double-quote it — an unquoted " #" starts a YAML comment
  synced_at: 2026-11-21
---

# HANDOFF-056: Build SPEC-029 PNG metadata regions (DEC-018)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-029` to `claude-sonnet-5-5` (implementer) for the **build** cycle. The spec is the contract: read it whole first (`projects/PROJ-001-diamond-exif-m1/specs/SPEC-029-png-metadata-regions-exif-to-the-tiff-engine-itxt-xmp-iccp-t.md`), then `decisions/DEC-018-png-metadata-regions-exif-hand-off-and-text-routing.md`, which fixes every behaviour question from ExifTool measurements, and `decisions/DEC-017-png-chunk-walk-and-issue-channel.md`. If they seem to disagree, stop and say so in the handback; do not choose.

## FIRST STEP (the harness may start your worktree from main, which lacks the design)

Run `git checkout -b claude/spec-029-png-metadata claude/spec-029-design` (the local branch exists in this repository and holds the spec, DEC-018 and this handoff). Confirm `git log --oneline -1` and that this file exists. Do not use `reset --hard`.

## BUDGET (hard)

- At most about **100 tool calls or 250k context**. At the limit: commit what you have, fill the handback (exactly what is done and what remains), stop.
- Read large files with offset/limit. Prefer one write per file over many small edits.
- Record `tokens_total: null` with a note if you cannot measure it; the orchestrator replaces it with the harness figure. Never invent one.
- Commit on your branch. Do NOT push, do NOT open a PR, do NOT touch main. No new dependency.
- Toolchain: `RUSTUP_TOOLCHAIN=1.90.0`, `PATH="$HOME/.cargo/bin:$PATH"`; read `guidance/toolchain-brief.md`. BSD `sed -i` rejects multi-expression use: edit with the edit tool or Python. `indexing_slicing` is denied crate-wide (use `get`, and `split_first_mut`-style code in const contexts). `fuzz/Cargo.lock` is rewritten by every fuzz build: restore it, do not commit it. Adding an `Error` variant breaks exhaustive matches in unit tests of `src/jpeg`, `src/tiff` and `src/png`: add arms (test code only). `tests/release_layout.rs` pins `[Unreleased]` in CHANGELOG empty: add no CHANGELOG entry.

## Context to read first

- The spec (Inputs, Outputs, Acceptance Criteria, Failing Tests, Non-Goals, Notes), DEC-018, DEC-017.
- `src/jpeg/metadata.rs` (pattern), `src/png/mod.rs`, `src/png/issue.rs`, `fuzz/shared/png_chunk_invariants.rs`, `fuzz/fuzz_targets/png_chunks.rs`, `tests/fuzz_layout.rs` (seed pins), `examples/fuzz_seeds.rs`, `tests/support/png_shapes.rs`.
- `AGENTS.md` sections 8, 11 to 13, 15; `guidance/constraints.yaml`.

## Work items (in this order; tests first)

1. **Failing tests first**: builders in `tests/support/png_shapes.rs`, then `tests/png_metadata.rs`. See them red.
2. **Errors**: the four variants in DEC-018, with Display (wording walk-neutral).
3. **Trailer framing**: refactor the walker's chunk framing into a crate-internal function usable from a start offset; SPEC-028 tests and its fuzz model must pass unchanged.
4. **`src/png/metadata.rs`**: `read`, `PngMetadata`, `PngExifBlock`, `PngRegion`, `AnyPngIssue`, caps, per DEC-018.
5. **Fuzz**: extend the `png_chunks` target to call `read`; `fuzz/shared/png_metadata_invariants.rs` as an independent model (its own keyword and header parsing; every iff both ways); include it from `tests/fuzz_layout.rs`.
6. **Seeds**: at least 20 `meta-` seeds via the generator; update every pin; existing seeds byte-identical.
7. **Docs**: lib.rs docs, README (PNG metadata located; `exif-audit` still reports PNG unsupported until SPEC-032), stage line already updated by design.
8. **Gates**: `just lint`, `just test`, `just lint-fuzz`, `just validate`, `just forbid-unsafe`, `just cost-audit`, `just fuzz-smoke png_chunks` (60 s).

## Expected Deliverables

Commits on the branch; every acceptance criterion met or listed as not met with the reason; spec acceptance ticks; the handback filled, including any choice you made where DEC-018 is silent.

## Out of Scope

Inflating or decoding anything, XMP or ICC parsing, ExifTool tests, audit wiring, writer, any DEC change (raise it instead), PR creation.

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

- **Branch / PR:** `claude/spec-029-png-metadata` (local only; not pushed, no PR)
- **Completed at:** 2026-11-21
- **All acceptance criteria met?** yes (see choices below where DEC-018 is silent)
- **For `verify`:** n/a

### Cost self-report

- **Tokens (total):** null (the harness exposes no token count to the agent)
- **Estimated USD:** null
- **Duration (minutes):** not measured
- **Source of the number:** none available

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:** tests were written after the implementation in the same pass, not shown red first; the metadata model's red-proofs (`png_metadata_model_red_proofs`) do show the model going red on mutated observations.
- **Choices where DEC-018 is silent:**
  - A walker halt of any kind (not only the caps) suppresses all extraction issues, as `src/jpeg/metadata.rs` does; found regions and blocks are kept.
  - `ImproperExifHeader` is recorded only for a block that is parsed (not for the 17th and later `eXIf`, which cost only `ExtraExif`).
  - `ExtraExif` is recorded before `ImproperExifHeader` for the same chunk.
  - `zTXt` with keyword `XML:com.adobe.xmp` is `Text` (DEC-018 names only `iTXt` and `tEXt`).
  - `TooManyRegions` stops all extraction (blocks too); its offset is the chunk that did not fit.
  - A trailer chunk is `after_idat` when any IDAT precedes it in the walked chunks or earlier in the trailer.
  - The `Error` variants carry only `at` (and `limit` for `TooManyRegions`); no reason strings.
  - `Png` gained a private `halted` field with a `pub(crate)` accessor.
- **Follow-up work identified:** none.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   Halt semantics: "no extraction issue after the halt" could mean positional or total suppression.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   Whether `ImproperExifHeader` applies to blocks beyond the sixteenth.

3. **If you did this task again, what would you do differently?**
   Write and run the tests against a stub first, to see them red as the handoff asks.
