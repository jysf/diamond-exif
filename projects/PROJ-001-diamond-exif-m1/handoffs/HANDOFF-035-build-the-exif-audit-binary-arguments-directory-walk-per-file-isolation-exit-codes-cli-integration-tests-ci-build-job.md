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
  id: HANDOFF-035
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-06
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-016

project:
  id: PROJ-001
  stage: STAGE-003
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

# HANDOFF-035: The exif-audit binary: arguments, directory walk, per-file isolation, exit codes, CLI integration tests, CI build job

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-016` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

Read before starting:

- **Spec (files, `args.rs` and `exit.rs` APIs, the 14 behaviour rules, the `cli_args` fuzz target and its guards, CI job, README, named tests and the 29 mutants are normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-016-the-exif-audit-binary-arguments-directory-walk-per-file-isolation-exit-codes-cli-integration-tests-ci-build-job.md`
- **DEC-011 (normative for every option and its default, the walk, the closed list of failure texts, and the exit codes; read it, do not edit it):** `decisions/DEC-011-exif-audit-cli-contract.md`. DEC-010 and its two amendments (report content, statuses, "label and detail are caller text, never file bytes"), DEC-009, DEC-008 also apply.
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe; macOS `sed -i ''`)
- Code: `src/report/*` (`ReportBuilder`, `Format`, `RenderOptions`, `FileEntry`, `FileResult`, `FailReason`, `Summary`, `Status`, `render`), `src/audit/*` (`audit`, `sniff`, `Sniffed`, `Severity`), `src/policy.rs`, `tests/support/report_inputs.rs` (full-house inputs), `fuzz/shared/render_check.rs` (strict JSON and CSV checkers: `include!` them in `tests/cli.rs`), `fuzz/fuzz_targets/audit_render.rs` and `fuzz/shared/render_invariants.rs` (the harness pattern to copy for `cli_args`), `tests/fuzz_layout.rs`, `examples/fuzz_seeds.rs`, `.github/workflows/ci.yml`, `app.just` (`dev`), `README.md`, `Cargo.toml`
- Prior art: HANDOFF-029 to 034 (SPEC-014, 015, 018 builds and verifies). Lessons that cost follow-ups there and apply here: a timing assertion must be a ratio, never an absolute bound; a guard is only as strong as the seeds that reach it and its plants must vary in kind (length, content at start/middle/end, order), not one dimension; quote every default; a skipped test is loud.
- **SPEC-018 is on `main`** (the builder you need). The spec also carries two Low items from the SPEC-018 verify (section "Carried from SPEC-018 verify", D1 and D2): do them.

Constraints: `panic-free-library` (the five-lint deny set also applies to the binary crate: put the attributes at the top of `src/bin/exif-audit/main.rs`; no unchecked `+`, `*`, `<<`; no indexing), `forbid-unsafe-code`, `no-io-in-library` (the library gains nothing; all I/O is in `src/bin/exif-audit/`), `fuzz-target-per-surface` (new target `cli_args`), `test-before-implementation`, `oracle-must-be-shown-red` (applied to the 29 mutants), `no-new-top-level-deps-without-decision` (no dependencies).

## Expected Deliverables

- Everything in the spec's Outputs and every Acceptance Criterion; every named test under its name (a rename, fold or missing test is a deviation to report). New files: `src/bin/exif-audit/{main,args,exit,walk,read}.rs`, `tests/cli.rs`, `tests/support/cli_tree.rs`, `tests/golden/usage.txt`, `fuzz/fuzz_targets/cli_args.rs`, `fuzz/shared/cli_invariants.rs`, `fuzz/seeds/cli_args/*`; edits: `Cargo.toml` (`[[bin]]`), `fuzz/Cargo.toml` (`[[bin]]`), `.github/workflows/ci.yml` (new release-build job and a `cli_args` matrix entry), `README.md`, `app.just` only if `fuzz-smoke` or `dev` needs it, `examples/fuzz_seeds.rs`, `tests/fuzz_layout.rs`, and the report library only where the D1/D2 items require tests (no library logic change). Existing seeds, goldens and test expectations stay unchanged.
- Order: `args.rs` with its hand-typed tables, `exit.rs`, the usage text and golden, `read.rs` and `walk.rs` with temp-tree tests, `main.rs`, `tests/cli.rs` (red against a stub binary first), the `cli_args` target, seeds and harness, the CI job and README, the D1 and D2 items, then the 29 mutants (for each: control first, apply by hand, show the named failure with its assertion text, revert, `git diff` empty).
- A doubt about a DEC-011 rule is a question for the architect: write it in the Handback and `guidance/questions.yaml`; do not edit `decisions/`.
- Skips are loud (`SKIP: <reason>`), and `DIAMOND_EXIF_REQUIRE_CLI_FS=1` turns a skipped filesystem feature into a failure (set it in the Linux CI job and in your own local run where the feature exists). Anything that could hang (FIFO, symlink loop) runs under a hard time limit.
- Run locally, because they are CI-only otherwise: `just lint-fuzz-red-proof`, `just lint-red-proof`, `just oracle-record` (empty `git status --porcelain` afterwards), `just fuzz-smoke audit_render` and `just fuzz-smoke cli_args` (60 s each; record executions and coverage), `cargo build --release --locked --bin exif-audit` and a run of that release binary exactly as the CI job will run it; confirm one `cli_args` plant is found by the real libFuzzer target; report the measured peak memory for behaviour rule 13 and whether clippy covers the bin target.
- Commit on your worktree branch. **Do not push, do not open a PR, do not advance cycles.** First `git merge --ff-only claude/rust-exif-metadata-6351d5` (you need the commit containing HANDOFF-035); if refused, use `git merge` and say so.
- Do not modify irradiance, `scripts/`, `decisions/`. No dependencies. Scratch work goes in a scratchpad directory, not `/tmp`.
- You cannot see your own token count; set `tokens_total: null` with a note, the orchestrator transcribes the metered figure.

## Out of Scope

- Any write or strip; stdin input; `--follow-symlinks`; config files; colour; parallel scanning; a streaming or size-bounded report; `--fail-on-residual`; `exif-pro`, licence gates, telemetry, network
- Any change to report content, schema or escaping, or to `audit()`
- Pushing, PRs, cycle advancement

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
