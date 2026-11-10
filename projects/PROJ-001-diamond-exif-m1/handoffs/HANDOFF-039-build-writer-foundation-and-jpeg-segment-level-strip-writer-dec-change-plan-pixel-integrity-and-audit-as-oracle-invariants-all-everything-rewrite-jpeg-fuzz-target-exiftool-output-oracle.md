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
  id: HANDOFF-039
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-10
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-020

project:
  id: PROJ-001
  stage: STAGE-004
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
  status: null                     # completed | blocked | rejected
  tokens_total: null               # REAL combined count — what cost-audit reads
  estimated_usd: null              # tokens_total × your rate, or your harness's number
  duration_minutes: null
  branch: null
  pr: null
  completed_at: null               # YYYY-MM-DD
  notes: null                      # one line if unusual (rework, no meter, etc.)
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-039: Writer foundation and JPEG segment-level strip (SPEC-020)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-020` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

Read before starting:

- **Spec (rules 1 to 11, library changes, fixtures, oracle, fuzz and seeds, the named tests and the 30 mutants are normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-020-writer-foundation-and-jpeg-segment-level-strip-writer-dec-change-plan-pixel-integrity-and-audit-as-oracle-invariants-all-everything-rewrite-jpeg-fuzz-target-exiftool-output-oracle.md`
- **DEC-013 (normative for the whole writer contract: edit plan of removals and same-length patches, fail closed, Exif all or nothing, Multi-Picture as a unit with index patching, overlap, the audit self-check; read it, do not edit it):** `decisions/DEC-013-writer-foundation-fail-closed-removal-and-patch-plan.md`, with DEC-012, DEC-009 (the preview is the contract the writer is checked against), DEC-008, DEC-006.
- **Evidence:** `spikes/done/SPIKE-001-*.md`; questions `strip-keeps-orientation`, `writer-stray-bytes`, `writer-mpf-attribute-fields` in `guidance/questions.yaml`.
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; macOS `sed -i ''`; never measure exit status through a pipe).
- Code: `src/jpeg/{mod,metadata}.rs`, `src/audit/*`, `src/policy.rs`, `src/lib.rs`, `tests/support/real_shapes.rs`, `tests/jpeg_oracle.rs`, `tests/inventory_oracle.rs`, `tests/fuzz_layout.rs`, `fuzz/shared/jpeg_metadata_invariants.rs` (the independent MPF reader to reuse), `examples/{fuzz_seeds,oracle_record}.rs`, `.github/workflows/ci.yml` (fuzz-smoke list).
- Lessons that cost follow-ups before: timing assertions are ratios, never absolute; a guard is only as strong as the seeds that reach it (add a seed for every new class, including the nested-and-overlapping-range and wrong-segment-for-signature shapes); plants must vary in kind (position, length, content) and run in both directions; **every invariant a fuzz guard can find must also have a deterministic unit test** (SPEC-019's verifier found two that only fuzzing caught); `rustfmt --check --edition 2021 fuzz/shared/*.rs` is now part of `just lint`; a mutant's revert must leave `git diff` empty (revert with `git checkout`, never by hand-copying).

Constraints: `panic-free-library` (five-lint deny set: no unchecked `+`, `-`, `*`; every offset and length is untrusted: `checked_*`, `get(..)`, never index), `forbid-unsafe-code`, `no-io-in-library` (the library never reads or writes a file; only the dev example does, and it writes nothing), `fuzz-target-per-surface` (new `rewrite_jpeg`), `test-before-implementation`, `oracle-must-be-shown-red`, `exiftool-never-linked`, `no-new-top-level-deps-without-decision` (no dependencies).

## Expected Deliverables

- Everything in the spec's Outputs and every Acceptance Criterion; every named test under its name (a rename, fold or missing test is a deviation to report). New files include `src/write/{mod,plan,jpeg,verify}.rs`, `tests/support/strip_shapes.rs`, `tests/strip.rs`, `tests/strip_oracle.rs`, `tests/oracle-fixtures/exiftool-strip/*`, `fuzz/fuzz_targets/rewrite_jpeg.rs`, `fuzz/shared/rewrite_invariants.rs`, `fuzz/seeds/rewrite_jpeg/*` (at least 60), `examples/strip_corpus.rs`. Existing seeds, records, goldens and the audit/report/CLI behaviour do not change (only the lists in `tests/fuzz_layout.rs` and the CI fuzz-smoke list gain `rewrite_jpeg`).
- Order: the plan type and its tests; the JPEG accessors; the fixture builders with canaries (and measure ExifTool's group names for the stripped outputs before writing the group table); the planner and integration tests; the Exif rule; MPF patches and overlap; the self-check with its wrong-on-purpose plans; oracle records; the fuzz target and harness (the harness stripper is independent and must not call into `src/write`); the corpus example; the mutants last.
- The local corpus re-run (spec Acceptance Criteria): the folder `/Users/jyashinsky/PSeven/experiments/crustimg_redo_plus/_incoming0` holds the user's real photos. **Read it in place only; copy nothing into the repo; write no file derived from it; print and record only counts and short hashes, never values, content or paths.** ExifTool on a real output only through a pipe (`--emit` to stdout piped to `exiftool -`), one file at a time.
- A doubt about a DEC-013 rule is a question for the architect: write it in the Handback and `guidance/questions.yaml`; do not edit `decisions/`.
- Run locally, because they are CI-only otherwise: `just lint-fuzz-red-proof`, `just lint-red-proof`, `just oracle-record` (empty `git status --porcelain` afterwards), `just fuzz-smoke rewrite_jpeg`, `jpeg_segments`, `audit`, `audit_render`, `cli_args` (60 s each; record runs and cov), `cargo build --release --locked --bin exif-audit`, the whole gate list in the spec's Acceptance Criteria.
- Commit on your worktree branch. **Do not push, do not open a PR, do not advance cycles.** First `git merge --ff-only claude/rust-exif-metadata-6351d5` (you need the commit containing HANDOFF-039); if refused, use `git merge` and say so.
- Do not modify irradiance, `scripts/`, `decisions/`. No dependencies. Scratch work goes in a scratchpad directory, not `/tmp`.
- You cannot see your own token count; set `tokens_total: null` with a note, the orchestrator transcribes the metered figure.

## Out of Scope

- Editing inside the Exif segment; TIFF and DNG writing; keeping Orientation; XMP or IPTC editing; MakerNote parsing; a write CLI; PNG, WebP, HEIC; changing the audit, the report schema, exit codes or the CLI
- Pushing, PRs, cycle advancement, SPEC-021 and SPEC-022

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
