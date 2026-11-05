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
  id: HANDOFF-005
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-05
  status: completed                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-006

project:
  id: PROJ-001
  stage: STAGE-001
repo:
  id: diamond-exif

# ── THE HANDBACK ────────────────────────────────────────────────────────────
# Filled in by the EXECUTING AGENT before it reports done. This is a required
# part of completing the handoff, not a courtesy.
#
# `tokens_total` is the one field the cost gate reads. Report the REAL number
# from your own interface:
#   Claude Code   → run `/cost`
#   API           → the `usage` object (input + output, summed)
#   another agent → whatever your harness reports as total tokens
# If your platform genuinely exposes NO token count, set tokens_total: null AND
# write why in `notes` — then set `cost.metering_source: none` in
# .repo-context.yaml so the gate stops asking. Do not invent a number.
handback:
  status: completed                # completed | blocked | rejected
  tokens_total: null               # REAL combined count — what cost-audit reads
  estimated_usd: null              # tokens_total × your rate, or your harness's number
  duration_minutes: null
  branch: worktree-agent-ad7a1619cd9256231
  pr: null
  completed_at: 2026-11-05         # YYYY-MM-DD
  notes: null                      # one line if unusual (rework, no meter, etc.)
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-005: tiff_ifd fuzz target

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-006` to `claude-sonnet-5-5` (implementer) for the **build** cycle: add the separate `fuzz/` crate with a `tiff_ifd` target on the strict parser, committed seeds, the `deny-fuzz` / `lint-fuzz` / `fuzz-smoke` recipes, and a shown-red proof that libFuzzer actually finds a planted panic.

## Context the Receiving Agent Needs

The receiving agent MUST read these before starting work. Keep the
list tight, but don't omit anything necessary.

### Primary

- **Project brief:** `./projects/PROJ-001-diamond-exif-m1/brief.md`
- **Stage:** `./projects/PROJ-001-diamond-exif-m1/stages/STAGE-001-m1a-foundation-tiff-ifd-engine.md`
- **Spec:** `./projects/PROJ-001-diamond-exif-m1/specs/SPEC-006-tiff-ifd-fuzz-target.md`
- **Reviewer brief (OPTIONAL, verify handoffs):** `./guidance/agents/<lens>.md` —
  pick a lens if this review should look at one thing hard. Omit for a
  general review; see `./guidance/agents/README.md`.
- **Toolchain brief:** `./guidance/toolchain-brief.md` — this repo's test
  framework, lint quirks, runtime globals and gotchas. Read it; it exists so a
  cold agent doesn't rediscover them (DEC-004 rule 5).

### Decisions that apply

- `DEC-004` — fuzz crate is a separate graph; two invocations per graph gate (`deny`/`deny-fuzz`, `clippy`/`lint-fuzz`); fuzz lint scope is clippy defaults under `-D warnings`.
- `DEC-002` — library `[dependencies]` must stay empty; `libfuzzer-sys` lives only in `fuzz/`.
- `DEC-001` — context only; the parser is still strict (SPEC-003 changes it, and must update this target then).

### Constraints that apply

Full text in `./guidance/constraints.yaml`: `fuzz-target-per-surface`, `panic-free-library` (library only; do not touch `src/`), `license-policy`, `no-new-top-level-deps-without-decision`.

### Prior related work

- SPEC-002 (merged): the parser and its public API (`Container::parse`, `ifds`, `entries`, `payload`, `uints`, `urationals`, `srationals`, `scalar`, `values`).
- Source to adapt FROM, READ ONLY: `~/PSeven/experiments/crustimg_redo_plus/irradiance/fuzz/` and its `app.just` fuzz recipes.
- Host traps: `guidance/toolchain-brief.md` plus the Notes in the spec (`cargo fuzz` needs `PATH="$HOME/.cargo/bin:$PATH"` and `~/.cargo/bin/cargo +nightly`).

## Expected Deliverables

*(For a `verify` handoff, replace this block with the verify contract below.)*

- Code changes implementing SPEC-006's Acceptance Criteria.
- All failing tests in SPEC-006 now passing.
- Any new tests required to cover edge cases.
- Commits on your branch (no push, no PR); the architect opens the PR after verify.
- PR description referencing: this handoff ID, the spec ID, the stage
  ID, the project ID, all referenced `DEC-*`, and any new `DEC-*`
  created during implementation.

### If `cycle: verify` — the verify contract

You are **not** the agent that implemented this. Review it cold.

- Acceptance criteria met? Tests actually pass? Build reflection answered
  honestly (*"nothing was unclear"* is suspicious)?
- Decision drift — run `just decisions-audit --changed`.
- Constraint violations; non-trivial choices missing a `DEC-*`.
- For any criterion claiming **runtime behavior** (a component registers, a hook
  fires, a binary resolves on PATH, a config takes effect), confirm the
  *behavioral* surface was exercised — not just the shape validated. That is the
  defect class that escapes (AGENTS.md §12).
- Output exactly ONE of: ✅ APPROVED / ⚠ PUNCH LIST / ❌ REJECTED.

## Out of Scope

Explicit list of what this handoff does NOT include. If the receiving agent
thinks any of these need to happen, they should create a new spec in
the stage's backlog, not expand this handoff.

- CI jobs, `lint-fuzz-red-proof` script, 24 h run, other fuzz surfaces, any `src/` change.
- Editing irradiance, adding dependencies other than `libfuzzer-sys` in `fuzz/`.

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

- **Branch / PR:** `worktree-agent-ad7a1619cd9256231` (no push, no PR)
- **Completed at:** 2026-11-05
- **All acceptance criteria met?** yes
- Test first: `tests/fuzz_layout.rs` committed red (4 failed), then green after the fuzz crate and seeds landed.
- `cargo +nightly fuzz build tiff_ifd` OK.
- 60 s smoke (seeds + empty corpus): no crash. Run 1: 14,402,286 execs in 61 s. Run 2 (corpus reused): 16,045,676 execs, final line `DONE cov: 467 ft: 2139 corp: 281/48Kb exec/s: 263043`.
- **Shown red** (scratch copy `/private/tmp/claude-501/redscratch`, repo untouched): planted `self.data[len]` in `Container::payload`, guarded by `field_type == 3 && count > 0x9000_0000`. libFuzzer found it in under 1 s from the seeds. Panic text: `index out of bounds: the len is 126 but the index is 8589278722` at `src/tiff/mod.rs:450:28`, `SUMMARY: libFuzzer: deadly signal`. Artifact: `fuzz/artifacts/tiff_ifd/crash-8cceea1aef4adc274b0e3f9792232a4543e757a3` (first run: `crash-3d57ddf3f20e62a76a94c36aaa0e73d94a73e490`).
- Gates: `just deny`, `just deny-fuzz` (licenses ok), `just lint`, `just lint-fuzz` (stable clippy), fmt, `cargo test` (23 unit + 4 crate_policy + 4 fuzz_layout), `cargo +1.90.0 test` (same counts), `cargo rustc --lib -- -F unsafe_code`, `just validate`, `just decisions-audit` all pass.
- Seeds directory hash identical across two runs of `examples/fuzz_seeds.rs` (`97a4a88b...`). Diff of `src/` and root `Cargo.toml` against 91b99f4 is empty; `cargo tree` shows no dependencies. Irradiance read only.

### Cost self-report

- **Tokens (total):** left for the orchestrator (metered count is in this task's result)
- **Estimated USD:** null
- **Duration (minutes):** ~25
- **Source of the number:** harness report (orchestrator transcribes)

### Drift and new artifacts

- **New decisions emitted:** none
- **Deviations from spec:**
  - Seeds: 17 files, not 12: added `-be` twins of `subifd`, `chain`, `payload-past-eof`, `count-overflow` (second byte order) plus `next-past-eof`. All 12 named shapes present.
  - `fuzz-smoke` does `mkdir -p fuzz/corpus/tiff_ifd` first: libFuzzer exits 1 if the corpus dir is missing (hit on the first run).
  - Seed-parse test in `fuzz_layout.rs` and the target also exercise `ifd.offset/depth/parent/next`/`scalar`/`values` on a fixed probe-tag list.
  - `just lint` must be run with the default (Homebrew) PATH; with `~/.cargo/bin` first, the nightly default lacks rustfmt (existing trap).
- **Follow-up work identified:**
  - SPEC-003 must update the target when the walk becomes tolerant (note in spec). CI jobs and `lint-fuzz-red-proof` remain repo-setup items.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Nothing blocking. The spec did not say the corpus directory must pre-exist; the worktree sandbox also refuses compound shell forms mentioning dotfiles or `$(...)`, which cost a few retries.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — No. The host PATH trap for `just lint` vs `cargo fuzz` pulls in opposite directions and could be stated side by side.

3. **If you did this task again, what would you do differently?**
   — Create the corpus dir and time the shown-red run from the start; otherwise the same order.
