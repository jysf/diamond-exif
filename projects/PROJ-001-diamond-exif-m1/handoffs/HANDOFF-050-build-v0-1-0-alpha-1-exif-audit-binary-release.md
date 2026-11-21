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
  id: HANDOFF-050
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-20
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-024

project:
  id: PROJ-001
  stage: STAGE-006
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
  basis: agent-result                # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: 124602              # REAL combined count — what cost-audit reads
  estimated_usd: 0.82              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: 6
  branch: claude/alpha-release-exif-audit
  pr: null
  completed_at: 2026-11-20               # YYYY-MM-DD
  notes: "Orchestrator correction: tokens_total is the harness subagent_tokens (124,602; 18 tool uses; 387,304 ms = 6.5 min) at 6.60 USD/Mtok = 0.82 USD, the same basis as earlier specs. The builder reported an estimate of about 130,000 and about 24 tool calls, which is not used."
  synced_at: 2026-11-20
---

# HANDOFF-050: Build SPEC-024 v0.1.0-alpha.1 exif-audit binary release (Phase 1: reversible prep)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-024` to `claude-sonnet-5-5` (implementer) for the **build** cycle. The spec is the contract: **read it whole first**: `projects/PROJ-001-diamond-exif-m1/specs/SPEC-024-v0-1-0-alpha-1-exif-audit-binary-release.md`. This is Phase 1 only (reversible prep). You must NOT create tags, push, run the workflow, open a PR, or publish anything; the repository is PUBLIC and the orchestrator does the dry run and (only on the user's go-ahead) the cut.

## BUDGET (hard)

- At most about **100 tool calls or 250k context**. At the limit: commit what you have, fill the handback (exactly what is done and what remains), stop.
- Read large files with offset/limit. Prefer one write per file over many small edits.
- Record a real `tokens_total` in the handback and say in `basis` where it came from; never invent one (the orchestrator replaces it with the harness `subagent_tokens` figure; do not count cache reads).
- Work in this worktree on branch `claude/alpha-release-exif-audit`, commit on it. Do NOT push, do NOT tag, do NOT open a PR, do NOT touch main. No new dependency.
- Toolchain: `~/.cargo/bin/cargo +1.90.0`, `RUSTUP_TOOLCHAIN=1.90.0` for `just lint`; read `guidance/toolchain-brief.md`. Use absolute paths in shell commands; do not `cd` out of the worktree.

## Context to read first

- The spec (Release Scope, Decisions made here 1 to 9, Outputs, Acceptance Criteria, Failing Tests, Non-Goals, Notes, Pre-Flight).
- `AGENTS.md` sections 8, 11 to 13, 15 only; `guidance/constraints.yaml`; `decisions/DEC-011-exif-audit-cli-contract.md` (the `--version` row and its nearby text); `.github/workflows/ci.yml` (the `exif-audit-release` job: build and smoke commands to reuse; do not modify that job); `Cargo.toml`; `src/bin/exif-audit/main.rs` around line 44; `README.md`; `CHANGELOG.md` (top `[Unreleased]` section); `SECURITY.md`; `tests/fuzz_layout.rs` (how layout tests read repo files with std only); `docs/versioning.md`.

## Work items (in this order; tests first)

1. **Failing tests first.** `version_line` unit tests in `src/bin/exif-audit/main.rs` and `tests/release_layout.rs` (static checks listed in the spec). Run them and see them red before changing anything.
2. **`--version` build ref.** Add `version_line(Option<&str>) -> String` (empty ref counts as none); `main` calls it with `option_env!("EXIF_AUDIT_BUILD_REF")`. Existing CLI tests must pass unchanged.
3. **Cargo.** `version = "0.1.0-alpha.1"`, `publish = false` (with a comment saying why and that removing it is a separate release spec); update `Cargo.lock`'s `exif-core` version entry (let cargo do it, `--locked` must then pass).
4. **`.github/workflows/release.yml`** exactly as designed (decisions 3 to 6 and 8): triggers `push: tags: ['v*']` and `workflow_dispatch` with boolean input `dry_run` (default true); workflow-level `permissions: contents: read`; a first job that checks the tag (or, on dispatch, the Cargo version) against `Cargo.toml` and refuses a dispatch with `dry_run: false` that is not on a tag ref; a build matrix over the three targets that runs `cargo build --release --locked --bin exif-audit --target <t>` with `EXIF_AUDIT_BUILD_REF` set to the short SHA, packs `exif-audit-<version>-<target>.tar.gz` (binary, `LICENSE-MIT`, `LICENSE-APACHE`, `README.md`), unpacks it into an empty directory and runs the four smoke checks of the CI job against it (macOS x86_64 under `arch -x86_64` if Rosetta is present, else say so in the handback), and uploads the archive as a workflow artifact; a final job that needs the builds, downloads the archives, writes `SHA256SUMS`, uploads it, and ONLY when the trigger is a tag push (never on a dry run) runs `gh release create "$TAG" --prerelease --verify-tag --title ... --notes-file ... <assets>` with `contents: write` on that job alone. Release notes come from the `## [0.1.0-alpha.1]` section of `CHANGELOG.md` (extract with awk or sed; fail if it is empty). Only `actions/checkout@v4`, `dtolnay/rust-toolchain` and the built-in `actions/upload-artifact` / `actions/download-artifact` (pin to a major version tag that exists; say which in the handback) are allowed; the test must list exactly the allowed set.
5. **Docs.** DEC-014 (decisions 1 to 6, 8, 9; use `decisions/_template.md`; confidence honest); amend DEC-011's `--version` row (the optional ` (<ref>)` suffix, only when the build ref was injected); README "Install" section (download with `gh release download` or `curl -L`, `shasum -a 256 -c SHA256SUMS`, macOS quarantine note with `xattr -d com.apple.quarantine`, first run, and the plain statements that the CLI never writes files, that the strip writer is library-only, that the library is not published and the binary is unsigned); CHANGELOG `## [0.1.0-alpha.1] - 2026-11-20` with a concise summary of what the CLI user gets (one paragraph plus short bullets; do not write per-spec entries), `[Unreleased]` left empty under its heading.
6. **Red-proofs.** For each static check in `tests/release_layout.rs`, show on a scratch copy (or a temporary edit you revert) that the matching mutation turns it red: add `pull_request` to the triggers, add `contents: write` at workflow level, add a disallowed action, change the Cargo version, drop `--locked`. Say in the handback which you did.
7. **Gates.** `just lint`, `just test`, `just deny`, `just forbid-unsafe`, `just validate`, `just cost-audit`; the CI file `ci.yml` unchanged. Validate the workflow YAML as well as you can offline (read it twice; `python3 -c "import yaml"` only if PyYAML exists). The real test is the dry run that the orchestrator triggers after you hand back, so make the workflow easy to read and its failure messages specific.

## Expected Deliverables

Commits on the branch; every acceptance criterion met or listed as not met; the spec updated (acceptance ticks; do NOT fill the Cut Record, the Release Notes are already written, adjust only if the implementation forced a change and say so); the handback filled in the front matter AND the section below (real `tokens_total`, branch, tool-call count, which red-proofs were shown, anything that could not be verified offline).

## Out of Scope

Tags, pushes, running or creating releases, crates.io, signing, attestations, Homebrew, Windows, Linux arm64, any change to the audit, parser, writer, report formats or exit codes, `ci.yml`, stage 7. If something forces a deviation, report it in the handback instead of improvising.

## Return Criteria

Fill the `handback:` front matter (including a real `tokens_total`) AND the `## Handback` section; set `handoff.status` and `handback.status` to `completed`. If you cannot complete: fill the section with what is done and what blocked you, set `handoff.status: rejected`, `handback.status: blocked`, still report tokens, and add a question to `guidance/questions.yaml`. **Split trigger:** if you reach the budget with the code, Cargo and tests done but the workflow or docs not, commit, list exactly what remains and stop.

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

- **Branch / PR:** `claude/alpha-release-exif-audit`, committed, not pushed, no PR (per handoff).
- **Completed at:** 2026-11-20
- **All acceptance criteria met?** All except the dry-run criterion, which needs the orchestrator (workflow cannot run offline). Tool calls used: about 24.

### Cost self-report

- **Tokens (total):** about 130000 (estimate; no transcript access)
- **Estimated USD:** null
- **Duration (minutes):** 20
- **Source of the number:** estimate; replace with the harness subagent_tokens figure

### Drift and new artifacts

- **New decisions emitted:** `DEC-014` (first binary release). `DEC-011` amended (`--version` row).
- **Deviations from spec:**
  - `publish` also needs `check`'s output, so it has `needs: [check, build]`.
  - A dispatch with `dry_run: false` on a tag ref passes the check but only rehearses; the release step is gated on `github.event_name == 'push'` alone (handoff wording), recorded in DEC-014 decision 5.
  - Allowed actions are checkout@v4, dtolnay/rust-toolchain@stable, upload-artifact@v4, download-artifact@v4 (v4 exists; `merge-multiple` and `pattern` need v4).
  - CHANGELOG: the large existing `[Unreleased]` entries were kept under a new `## Development history before 0.1.0-alpha.1` heading (so the release-notes extraction sees only the summary); `[Unreleased]` is empty.
  - Stage file and project brief were NOT updated (not needed for the gates; left for the orchestrator).
  - `tests/release_layout.rs` also holds permanent in-file red-proof tests that mutate the workflow text.
- **Follow-up work identified:** none beyond the dry run and cut.

### Red-proofs shown

Real-file mutations of `release.yml` (each reverted, `cmp` confirmed): add `pull_request`, top-level `contents: write`, a disallowed action (`evil/act@v1`), drop `--locked` all turned `release_workflow_triggers_permissions_and_actions_are_as_designed` red. Changing the Cargo version turned `cargo_version_is_the_alpha_and_publish_is_false` red. Tests-first: before implementing, `release_layout` had 11 of 12 red (no workflow, old Cargo version) and the `version_line` unit tests did not compile. Permanent in-file `red_proof_*` tests cover pull_request, workflow-level write, disallowed action, `--locked`, `--prerelease`, unguarded release, wrong target.

### Could not verify offline

The workflow on a runner (only parsed with PyYAML and checked by the static tests); musl link and x86_64-apple-darwin cross-build; Rosetta (the step runs under `arch -x86_64`, else warns loudly and skips the run); `actions/*@v4` behaviour; `gh release create`; the awk notes extraction was run locally and works. `shasum -a 256 -c` and `--ignore-missing` portability avoided in the README (uses `grep | shasum -c -`). Gates: `just lint`, `test`, `deny` (licenses ok, one unmatched-allowance warning), `forbid-unsafe`, `validate`, `cost-audit` all green; `ci.yml` unchanged.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Whether a dispatch with dry_run false on a tag should publish; the spec and handoff differ slightly. I followed the handoff (push only).

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — The `publish` job needing `check` for the version output; and what to do with the old `[Unreleased]` entries.

3. **If you did this task again, what would you do differently?**
   — Restore files by copy, not by sed (I broke the Cargo version once and fixed it).
