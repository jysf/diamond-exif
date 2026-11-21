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
  agent: claude-opus-5-5           # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: punch-list              # verify handoffs: approved | punch-list | rejected
  basis: agent-result                # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: 136801              # REAL combined count — what cost-audit reads
  estimated_usd: 0.90              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: 5
  branch: claude/alpha-release-exif-audit
  pr: null
  completed_at: 2026-11-20         # YYYY-MM-DD
  notes: "Round 2, PUNCH LIST (1 High, 4 Low): the annotated-tag check rejects every real annotated tag (actions/checkout@v4 overwrites it). Orchestrator correction: tokens_total is the harness subagent_tokens (136,801; 27 tool uses; 286,999 ms = 4.8 min) at 6.60 USD/Mtok = 0.90 USD. The verifier also used a fetch subagent it reported at 39,291 subagent_tokens, not added here (unclear whether the harness figure already includes it). The verifier self-reported 1,578,357 (transcript, cache reads included), not used."
  synced_at: 2026-11-20
---

# HANDOFF-052: Verify SPEC-024, round 2 (narrow: the round-1 punch list)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-024` to a fresh verifier (`claude-opus-5-5`) for a **narrow second verify round**. Round 1 (HANDOFF-051, a different verifier) returned PUNCH LIST: 2 Medium, 8 Low. A builder follow-up claims all ten are fixed (commits `69120cf`, `850f915`; the head commit adds only docs after that). Check that each fix is real and did not break anything. Return exactly ONE verdict (APPROVED / PUNCH LIST / REJECTED), every finding graded High, Medium or Low. The repository is PUBLIC and the workflow can publish a release: treat it as security-sensitive.

## BUDGET (hard)

About **45 tool calls or 120k context**. Work in this worktree (`/Users/jyashinsky/PSeven/experiments/crustimg_redo_plus/diamond-exif/.claude/worktrees/spec-022-tiff-dng-strips-26c250`, branch `claude/alpha-release-exif-audit`). Do NOT change code, push, tag, trigger any workflow or open a PR; findings go in the handback only. Any mutation you try must be reverted (`git checkout -- <file>`) and `git status` must end clean. `~/.cargo/bin/cargo +1.90.0`, `RUSTUP_TOOLCHAIN=1.90.0` for lint. Record a real token count and its basis.

## What to read

The round-1 handback in HANDOFF-051 (every finding with reproduction), the follow-up note in HANDOFF-050, `git diff eaaecc2..HEAD` (the follow-up), then `.github/workflows/release.yml`, `tests/release_layout.rs`, `src/bin/exif-audit/main.rs` (`version_line`), the SPEC-024 pre-flight and acceptance criteria, DEC-016, the DEC-011 `--version` row, README Install.

## What to check

1. **M1 (loose tests).** Re-do the five regressions on the real `release.yml` and confirm each now turns `cargo test --test release_layout` red: the release guard widened (`|| always()`); `permissions: write-all` on `build`; `id-token: write` or `actions: write` on `publish`; the tag-equals-version gate disabled (`if false && ...`); `branches: [main]` added under `push:`. Also try two NEW bypasses the builder may not have thought of: moving the `gh release create` into a second step or job without the guard; adding `workflow_run` or `schedule:` triggers. Is the new `if:` guard (`github.event_name == 'push' && startsWith(github.ref, 'refs/tags/v')`) correct on a tag push and absent on dispatch? Are the checks still so literal that a harmless reformat breaks them?
2. **M2.** Pre-flight item 6 is now `[cut]` and no throw-away tag remains in SPEC-024 or DEC-016 (grep for `rc.test` and `throw-away`).
3. **L1 to L8** each fixed as stated in the follow-up note. Specifically: the four allowed actions named consistently in the spec, DEC-016 and the layout test; the Rosetta policy (fails on tag push, warns on dispatch, `file` asserts x86_64) reads correctly in the YAML; `persist-credentials: false` on every checkout; the `check` job's annotated-tag and ancestor-of-`origin/main` steps (do they run only on a tag push, do they have what they need: `fetch-depth: 0`; `GITHUB_SHA` for an annotated tag is the commit, so `--is-ancestor` is right; is `refs/tags/${GITHUB_REF_NAME}` the right ref; do the checks run BEFORE any build and before `publish`?); README wording and the `sha256sum` line; `version_line` accepts only `[A-Za-z0-9._-]`, at most 64 chars, with the unit tests listed; `cargo_problems` red-proof is no longer a tautology.
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

- **Branch / PR:** `claude/alpha-release-exif-audit`, reviewed at HEAD `03e49f2` (follow-up code in `69120cf`, `850f915`). Not pushed, no PR.
- **Completed at:** 2026-11-20
- **All acceptance criteria met?** No. The ten round-1 fixes are real, and all five round-1 mutations now go red. But the L4 fix adds a tag check that, on `actions/checkout@v4`, rejects every annotated tag (H1). It fails closed, so it is not a security hole. It does block the real cut, and the dry run cannot catch it. AC 3 (the dry run) is still unverified, as expected.
- **For `verify`:** ⚠ PUNCH LIST (1 High, 0 Medium, 4 Low)

#### Findings

**H1 (High): the annotated-tag check fails every real annotated tag, so the first real cut will fail in `check`.** `.github/workflows/release.yml:80-88` (with the checkout at `:40-44`). When the triggering ref is a tag, `actions/checkout@v4` first fetches `+refs/tags/*:refs/tags/*`. It then calls `testRef`, which compares `git rev-parse refs/tags/<tag>` (not peeled, so it returns the tag object SHA for an annotated tag) with the commit SHA. That comparison always fails for an annotated tag, so checkout refetches `+<commit>:refs/tags/<tag>`. The refetch force-overwrites the annotated tag with a lightweight ref. This is `actions/checkout` issue #290, "Preserve tag annotations", which is still open. Its maintainer says the fix shipped in v6.0.2, not v4. I read the v4 source of `ref-helper.ts` (`getRefSpec`, tags branch `+${commit}:${ref}`; `testRef`, tags branch) and `git-command-manager.ts` (`revParse` runs a plain `git rev-parse <ref>`) through a fetch subagent. I did not read `input-helper.ts`, so the claim that `settings.commit` is the peeled commit is inferred from `GITHUB_SHA`. After the overwrite, `git cat-file -t "refs/tags/${GITHUB_REF_NAME}"` prints `commit`, and the step exits with "is not annotated".
Reproduction (scratch repo outside the tree): clone a repo that has an annotated tag, then `git fetch origin "+<commit>:refs/tags/v0.1.0-alpha.1"`; `git cat-file -t refs/tags/v0.1.0-alpha.1` prints `commit` and the workflow's check logic prints FAIL. A later `git fetch --force origin "+refs/tags/v0.1.0-alpha.1:refs/tags/v0.1.0-alpha.1"` brings back `tag`. The dispatch dry run skips this step (`if: github.event_name == 'push'`), so the first time it runs is on the public tag push. Recovery would mean a fix on `main`, deleting the public tag and tagging again.
Fix (any one of these): (a) before `cat-file`, run `git fetch --force --no-tags origin "+refs/tags/${GITHUB_REF_NAME}:refs/tags/${GITHUB_REF_NAME}"` (the repo is public, so an anonymous fetch works with `persist-credentials: false`); (b) check the remote directly: `git ls-remote --exit-code origin "refs/tags/${GITHUB_REF_NAME}^{}"`, which only an annotated tag has; (c) `gh api "repos/${GITHUB_REPOSITORY}/git/ref/tags/${GITHUB_REF_NAME}" --jq .object.type` must equal `tag` (needs `GH_TOKEN`, read scope). Update the layout-test needle at `tests/release_layout.rs:300-306` to match. (Moving to `actions/checkout@v6` is out of scope here; it would change the allowed set.) The `merge-base --is-ancestor "${GITHUB_SHA}" origin/main` half is fine: fetch-depth 0 brings `+refs/heads/*:refs/remotes/origin/*`, and `--is-ancestor` peels, so it passes for the tag SHA or the commit SHA.

**L1 (Low): an unguarded step in `publish` can still create a release on a dry run.** `tests/release_layout.rs:387` and `:370-386`. The test looks only for the literal `gh release create`. I added a step to `publish` with no `if:` that runs `gh api -X POST repos/.../releases` with `GH_TOKEN`. The test stayed **green**, and `publish` holds `contents: write` on every dispatch. Fix: also reject `gh api`, `gh release` (any subcommand) and `GH_TOKEN`/`GITHUB_TOKEN` anywhere except the single guarded step. Better, the round-1 L3 option: move the create step into a fourth job with `if: github.event_name == 'push' && startsWith(github.ref, 'refs/tags/v')` that alone holds `contents: write`, so a dry run never holds write.

**L2 (Low): write grants that are quoted or followed by a trailing comment are not seen.** `tests/release_layout.rs:194` (`ends_with(": write")`) and `:39` (only whole-line comments are stripped). On `publish`, both `packages: 'write'` and `id-token: write # oidc` stayed **green**. Fix: strip trailing ` #...` comments and quotes before comparing, or require the `publish` `permissions:` block to be exactly the one line `contents: write`.

**L3 (Low): `continue-on-error` and an early `exit 0` defeat the check job without turning the test red.** `tests/release_layout.rs:294` (bypass list). These stayed **green**: `continue-on-error: true` on the version step; `exit 0` as the first line of its script; `if: always()` at the `publish` job level. The runtime partly backs this up. An empty version output breaks the smoke `want` string, and with no archives the `n = 3` count fails. But `continue-on-error` on the annotated-and-on-main step has no such backstop. Fix: reject `continue-on-error`, `exit 0` inside `check`, and any job-level `if:` on `build`/`publish`. Optional for the alpha.

**L4 (Low): README Install still reads as if the release exists.** `README.md:27`: "Download the archive for your platform from the Releases page (version `0.1.0-alpha.1`...)". This is better than round 1's "is on", but between the merge and the cut the commands fail. Fix: add "(published once `v0.1.0-alpha.1` is tagged)", or accept it if the cut follows the merge closely.

#### Round-1 items re-checked

- **M1:** fixed. Real-file mutations, each reverted with `git checkout`: (a) `|| always()` on the guard **red**; (b) `permissions: write-all` on `build` **red**; (c) `id-token: write` **red**, `actions: write` **red**; (d) `if false &&` on the gate **red**; (e) `branches: [main]` under `push:` **red**. New bypasses: a second job `release2` with `gh release create` and `contents: write` **red**; `schedule:` **red**; `workflow_run:` **red**; the green ones are L1 to L3 above. The guard `github.event_name == 'push' && startsWith(github.ref, 'refs/tags/v')` is right: true on a `v*` tag push (the only push this workflow accepts), false on any dispatch. Brittleness: `tags: ["v*"]` and `event_name=='push'` with no spaces turn red. That fails closed, which is acceptable.
- **M2:** fixed. Pre-flight item 6 is `[cut]` and now explains why a throw-away tag cannot work. `rc.test` appears nowhere in the spec or DEC-016; "throw-away" appears only in that explanation.
- **L1:** fixed. Spec Decision 3, AC 2, DEC-016:41 and :51, and the test's `ALLOWED_ACTIONS` all name the same four actions.
- **L2:** fixed. `file` is always asserted to report x86_64 (`release.yml:156-157`). Missing Rosetta gives `::error::` and exit 1 when `GITHUB_EVENT_NAME = push`, and `::warning::` and exit 0 otherwise (`:159-167`).
- **L3:** fixed. `persist-credentials: false` is on all three checkouts (`:44`, `:113`, `:203`). The optional fourth-job split was not done (see L1).
- **L4:** the annotated-tag and on-main checks were added: `fetch-depth: 0`, the step runs only on a push, and it is in `check`, before `build` (`needs: check`) and `publish`. `refs/tags/${GITHUB_REF_NAME}` is the right ref, and `GITHUB_SHA` with `--is-ancestor` is correct. But the annotated half is broken by checkout (H1). Scratch simulation (a fresh `git init` origin plus a clone, logic copied from the step): annotated tag on main PASS; lightweight tag FAIL "not annotated"; annotated tag on a side branch FAIL "not on main"; `GITHUB_SHA` set to the tag object PASS. After checkout's `+<commit>:refs/tags/<tag>` refetch, the annotated tag on main FAILs.
- **L5/L6:** the `sha256sum -c -` line and the note that `xattr` errors harmlessly are present. L5 is only partly fixed (L4 above).
- **L7:** fixed. `build_ref_is_clean` requires non-empty, at most 64 bytes, and only `[A-Za-z0-9._-]` (`src/bin/exif-audit/main.rs:62-71`). The unit tests cover space, newline, ESC, tab, `é`, `/`, `()`, 65 characters, a 64-character ref and `v1.2-rc_3`. Using byte length is fine because only ASCII passes.
- **L8:** fixed. The red-proof calls `cargo_problems` on a mutated version, lock and `publish`.

#### Gates (re-run here, `RUSTUP_TOOLCHAIN=1.90.0`)

All six exited 0: `just lint`, `just test` (943 passed, 0 failed, 10 ignored across 23 test binaries; 934 in round 1, so 9 new tests), `just deny` (licenses ok, with the known not-encountered allowance warnings), `just forbid-unsafe`, `just validate` and `just cost-audit`.

Other checks: `.github/workflows/ci.yml` is byte-identical to `origin/main` (freshly fetched). A locked release build into a scratch target dir printed `exif-audit 0.1.0-alpha.1`. With `EXIF_AUDIT_BUILD_REF=abc1234` it printed `exif-audit 0.1.0-alpha.1 (abc1234)`, and with `'bad ref'` it printed `exif-audit 0.1.0-alpha.1` with no suffix (checked with `od -c`). `git status` is clean.

#### Could not verify here

- The workflow on a real runner; the musl link; the Intel cross-build and Rosetta on `macos-latest`; `merge-multiple`; `gh release create`; mawk running the notes awk.
- H1 rests on reading the v4 source, not on a run. Only a real tag push exercises it, and the dry run **cannot** (the step is push-only). Fix H1 before tagging rather than relying on the cut to test it.
- In the dry run (from `main`, after the merge), check: `check` logs "not a tag ref ... dry run only" and the tag step shows **skipped**; each of the three build legs logs `smoke checks passed`, with no "Rosetta is not available" warning, and the Intel `file` line says x86_64; `publish` prints a 3-line SHA256SUMS and `OK` lines, and "create the pre-release" is **skipped**; the downloaded arm64 archive's `--version` shows the dispatched commit's 7-character SHA.
- On the real cut, watch the "tag must be annotated and on main" step first.

### Cost self-report

- **Tokens (total):** 1,578,357. This is a floor, measured before the last ~3 calls. My transcript: 15 deduplicated messages, input 31, cache creation 122,282, cache read 1,370,671, output 13,380 (1,506,364). The fetch subagent: input 6, cache creation 36,085, cache read 31,767, output 4,135 (71,993). Without cache reads the total is 175,919. The harness reported the fetch subagent as 39,291 subagent_tokens.
- **Estimated USD:** null. `docs/cost-tracking.md` has no claude-opus-5-5 rate; price it from the components, or replace the figure with the harness subagent_tokens as in round 1.
- **Duration (minutes):** about 20.
- **Source of the number:** the session transcripts `subagents/agent-a9d884c8605f4b3d1.jsonl` and `agent-a5760a7d940552588.jsonl`, measured per docs/cost-tracking.md "Measuring a cycle" (each message.id counted once at its largest output_tokens).

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:** none by the verifier. No code was changed. Every mutation was written to `release.yml`, tested and restored, with a final `git checkout`. `git status` was clean after each. Probe builds went to a scratch `--target-dir`. `git fetch origin main` updated only the remote-tracking ref.
- **Follow-up work identified:**
  - Before tagging: fix H1 and its test needle.
  - Optional for the alpha: L1 (the fourth-job split also removes write from dry runs), L2, L3, L4; the GitHub tag ruleset on `v*` (a repository setting).

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Nothing much. The handoff's question "do they have what they need: fetch-depth: 0" pointed straight at H1. The answer depends on checkout internals that a static test cannot see.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — Steps that only run on a tag push cannot be dry-run, so they should be held to a higher standard (read the action's source, or check the remote with `ls-remote`/the API rather than trusting the local checkout's refs).

3. **If you did this task again, what would you do differently?**
   — Write `${VAR}` everywhere in zsh: `$ON:refs` was parsed as the `:r` modifier and wasted one scratch run.
