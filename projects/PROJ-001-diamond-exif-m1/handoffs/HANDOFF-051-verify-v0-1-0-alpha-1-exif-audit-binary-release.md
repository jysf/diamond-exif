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
  id: HANDOFF-051
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-20
  status: pending                  # pending | accepted | completed | rejected

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

# HANDOFF-051: Verify SPEC-024 (v0.1.0-alpha.1 exif-audit binary release, Phase 1 prep)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-024` to a fresh verifier (`claude-opus-5-5`) for the **verify** cycle. You did not write this. Review it cold and adversarially, and return exactly ONE verdict (APPROVED / PUNCH LIST / REJECTED) with every finding graded High, Medium or Low. The repository is PUBLIC and the workflow under review can publish a release: treat the workflow as security-sensitive code.

## BUDGET (hard)

About **70 tool calls or 180k context**. Work in this worktree (`/Users/jyashinsky/PSeven/experiments/crustimg_redo_plus/diamond-exif/.claude/worktrees/spec-022-tiff-dng-strips-26c250`, branch `claude/alpha-release-exif-audit`, build commit `b9e391c`; the cost-record commit after it changes only docs). Do NOT change code, push, tag, trigger any workflow, or open a PR; write findings in the handback only. Any mutation you try must be reverted so the tree ends clean. `~/.cargo/bin/cargo +1.90.0`, `RUSTUP_TOOLCHAIN=1.90.0` for lint. Record a real token count and its basis.

## What to read

The spec `projects/PROJ-001-diamond-exif-m1/specs/SPEC-024-v0-1-0-alpha-1-exif-audit-binary-release.md` (Decisions 1 to 9, Acceptance Criteria, Failing Tests, Pre-Flight), the build handback in HANDOFF-050, and `git diff 784576a..b9e391c`. Start with `.github/workflows/release.yml`, then `tests/release_layout.rs`, `src/bin/exif-audit/main.rs`, `Cargo.toml`, DEC-014, the DEC-011 amendment, README Install, CHANGELOG.

## What to probe

1. **Workflow security and logic (most important).** Expression injection (any `${{ }}` inlined into a `run:` script, including `github.ref_name`, `inputs.*`, `needs.*.outputs.*`); the trigger set (a tag push and `workflow_dispatch` only); permissions (read by default; `contents: write` only on `publish`; is it a problem that `publish` holds write on a dry run?); can a dry run, or a dispatch with `dry_run: false` on a branch or tag ref, ever create a release; does a tag that does not equal the Cargo version fail before any build; what happens for a tag like `v0.1.0-alpha.1` pushed on a commit that is not on `main`; `gh release create` flags and assets; whether the release notes step can publish empty notes; the artifact download pattern (`merge-multiple`) when one build leg fails (does `publish` still run? it should not); `fail-fast: false` plus `needs: [check, build]` semantics; the Rosetta fallback that exits 0 (could it hide a broken Intel binary, and is that acceptable for an alpha?); archive contents and names; the `x86_64-unknown-linux-musl` leg needing no extra linker; `cut -c1-7` of the SHA versus `git rev-parse --short` (SHA length collisions are irrelevant, but check the `--version` equality test is robust).
2. **Tests.** `tests/release_layout.rs`: do the static checks check the things the spec lists, and would each go red under the mutations the builder claims? Re-do three of the mutations yourself on the real `release.yml` (restore with `git checkout` and confirm `git status` is clean): add `pull_request` to the triggers, add top-level `contents: write`, add a disallowed action. Are the checks so string-literal that a harmless reformat (quote style, key order) would break them, or so loose that a real regression passes?
3. **Code.** `version_line` and its use: `option_env!`, empty ref, a ref with odd characters (spaces, control bytes, very long) in `EXIF_AUDIT_BUILD_REF`, since it ends up on stdout; the existing CLI tests unchanged and green; the CLI contract (`exit 0`, one line, newline) kept.
4. **Cargo.** `publish = false` and `cargo publish --dry-run` behaviour; `Cargo.lock` consistent with `--locked`; `include` list unaffected; version string valid SemVer.
5. **Docs accuracy.** README Install: every command is correct on macOS and Linux as written (read each one; `gh release download`, `shasum -a 256 -c` with the `grep` form, `xattr -d com.apple.quarantine`), the file names match the workflow's archive names, and the claims (never writes files, library not published, unsigned) are true. CHANGELOG: the `[0.1.0-alpha.1]` section is accurate and non-empty, the awk extraction in the workflow yields exactly that section (run it locally), and the old `[Unreleased]` content was moved, not lost. DEC-014 and the DEC-011 amendment match what was built; DEC-014's confidence is honest.
6. **Known design flaw to confirm or refute.** The spec's pre-flight item 6 proposes exercising rollback on a throw-away tag `v0.0.0-rc.test`. The `check` job rejects any tag that differs from `v<Cargo version>`, so that tag would fail at `check` and never reach `publish`. Confirm, and say what the honest way to exercise rollback is (it may be only to document it and verify the delete commands' syntax).
7. **Gates.** Re-run `just lint`, `just test`, `just deny`, `just forbid-unsafe`, `just validate`, `just cost-audit`; confirm `.github/workflows/ci.yml` is byte-identical to `main`'s.
8. **Not verifiable here, so say so rather than guess:** the workflow on a real runner (the orchestrator will dry-run it from `main` after merge, since GitHub only dispatches workflows present on the default branch), the musl and Intel builds, Rosetta, `gh release create`. List what you could not verify and what in the dry run should be checked.

## Out of scope

crates.io, signing, attestations, Windows, Linux arm64, any audit, parser or writer change. Findings there are at most Low notes.

## Return Criteria

Fill the `handback:` front matter (`verdict`, a real `tokens_total`, `basis`, `branch`, `completed_at`) and the `## Handback` section with the verdict and every finding (grade, file:line, how to reproduce, suggested fix). Set `handoff.status` and `handback.status` to `completed`. Commit only that handoff file; do not push.

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
