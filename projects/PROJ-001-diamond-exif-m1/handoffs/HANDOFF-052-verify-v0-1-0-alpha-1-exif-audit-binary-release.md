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
  id: HANDOFF-052
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

# HANDOFF-052: Verify SPEC-024, round 2 (narrow: the round-1 punch list)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-024` to a fresh verifier (`claude-opus-5-5`) for a **narrow second verify round**. Round 1 (HANDOFF-051, a different verifier) returned PUNCH LIST: 2 Medium, 8 Low. A builder follow-up claims all ten are fixed (commits `69120cf`, `850f915`; the head commit adds only docs after that). Check that each fix is real and did not break anything. Return exactly ONE verdict (APPROVED / PUNCH LIST / REJECTED), every finding graded High, Medium or Low. The repository is PUBLIC and the workflow can publish a release: treat it as security-sensitive.

## BUDGET (hard)

About **45 tool calls or 120k context**. Work in this worktree (`/Users/jyashinsky/PSeven/experiments/crustimg_redo_plus/diamond-exif/.claude/worktrees/spec-022-tiff-dng-strips-26c250`, branch `claude/alpha-release-exif-audit`). Do NOT change code, push, tag, trigger any workflow or open a PR; findings go in the handback only. Any mutation you try must be reverted (`git checkout -- <file>`) and `git status` must end clean. `~/.cargo/bin/cargo +1.90.0`, `RUSTUP_TOOLCHAIN=1.90.0` for lint. Record a real token count and its basis.

## What to read

The round-1 handback in HANDOFF-051 (every finding with reproduction), the follow-up note in HANDOFF-050, `git diff eaaecc2..HEAD` (the follow-up), then `.github/workflows/release.yml`, `tests/release_layout.rs`, `src/bin/exif-audit/main.rs` (`version_line`), the SPEC-024 pre-flight and acceptance criteria, DEC-014, the DEC-011 `--version` row, README Install.

## What to check

1. **M1 (loose tests).** Re-do the five regressions on the real `release.yml` and confirm each now turns `cargo test --test release_layout` red: the release guard widened (`|| always()`); `permissions: write-all` on `build`; `id-token: write` or `actions: write` on `publish`; the tag-equals-version gate disabled (`if false && ...`); `branches: [main]` added under `push:`. Also try two NEW bypasses the builder may not have thought of: moving the `gh release create` into a second step or job without the guard; adding `workflow_run` or `schedule:` triggers. Is the new `if:` guard (`github.event_name == 'push' && startsWith(github.ref, 'refs/tags/v')`) correct on a tag push and absent on dispatch? Are the checks still so literal that a harmless reformat breaks them?
2. **M2.** Pre-flight item 6 is now `[cut]` and no throw-away tag remains in SPEC-024 or DEC-014 (grep for `rc.test` and `throw-away`).
3. **L1 to L8** each fixed as stated in the follow-up note. Specifically: the four allowed actions named consistently in the spec, DEC-014 and the layout test; the Rosetta policy (fails on tag push, warns on dispatch, `file` asserts x86_64) reads correctly in the YAML; `persist-credentials: false` on every checkout; the `check` job's annotated-tag and ancestor-of-`origin/main` steps (do they run only on a tag push, do they have what they need: `fetch-depth: 0`; `GITHUB_SHA` for an annotated tag is the commit, so `--is-ancestor` is right; is `refs/tags/${GITHUB_REF_NAME}` the right ref; do the checks run BEFORE any build and before `publish`?); README wording and the `sha256sum` line; `version_line` accepts only `[A-Za-z0-9._-]`, at most 64 chars, with the unit tests listed; `cargo_problems` red-proof is no longer a tautology.
3b. Simulate the new tag checks in a scratch directory outside the repo (a fresh `git init`, an annotated tag, a lightweight tag, a commit not on main) to confirm the shell logic gives the intended pass or fail for each.
4. **Regressions.** Re-run `just lint`, `just test`, `just deny`, `just forbid-unsafe`, `just validate`, `just cost-audit`; `.github/workflows/ci.yml` byte-identical to `origin/main`; a locked release build prints `exif-audit 0.1.0-alpha.1` and, with `EXIF_AUDIT_BUILD_REF=abc1234`, `exif-audit 0.1.0-alpha.1 (abc1234)`; with `EXIF_AUDIT_BUILD_REF='bad ref'` prints no suffix.
5. **Not verifiable here, so say so:** the workflow on a real runner, `actions/checkout@v4` tag behaviour with `fetch-depth: 0`, musl and Intel builds, Rosetta, `merge-multiple`, `gh release create`. List what the orchestrator's dry run (from `main`, after merge) and the real cut should look at; note that the dry run cannot exercise the tag-only steps.

## Out of scope

crates.io, signing, attestations, Windows, Linux arm64, any audit, parser or writer change. At most Low notes.

## Return Criteria

Fill the `handback:` front matter (`verdict`, a real `tokens_total`, `basis`, `branch`, `completed_at`) and the `## Handback` section with the verdict and every finding (grade, file:line, reproduction, fix). Set `handoff.status` and `handback.status` to `completed`. Commit only that handoff file; do not push.

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
