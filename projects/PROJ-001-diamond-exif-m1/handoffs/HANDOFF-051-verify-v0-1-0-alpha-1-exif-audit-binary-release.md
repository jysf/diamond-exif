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
  basis: transcript                # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: 1982859            # REAL combined count — what cost-audit reads
  estimated_usd: null              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: 25
  branch: claude/alpha-release-exif-audit
  pr: null
  completed_at: 2026-11-20
  notes: "FLOOR, measured live from subagents/agent-af98cafc183101310.jsonl (20 deduplicated messages: input 39, cache creation 117,067, cache read 1,849,191, output 16,562) before the last ~4 calls; re-sum after the session ends. estimated_usd null: no claude-opus-5-5 rate found in docs/cost-tracking.md; orchestrator to price it. About 25 tool calls."
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

- **Branch / PR:** `claude/alpha-release-exif-audit`, reviewed at build commit `b9e391c` (HEAD `7235de1`; later commits docs only). Not pushed, no PR.
- **Completed at:** 2026-11-20
- **All acceptance criteria met?** No, two gaps. AC 2 is ticked but the workflow also uses `actions/upload-artifact@v4` and `actions/download-artifact@v4` (L1). AC 3 (dry run) cannot be verified until the workflow runs on a runner. The workflow itself is sound: I found no injection, no path that publishes on a dispatch, and the tag gate runs before any build. The punch list is about tests that are too loose to guard those properties, and a flawed spec pre-flight item.
- **For `verify`:** ⚠ PUNCH LIST (0 High, 2 Medium, 8 Low)

#### Findings

**M1 (Medium): the static workflow checks pass several real security regressions.** `tests/release_layout.rs:118-128`, `:186-201`, `:222-229`, `:87-100`. I mutated the real `release.yml`, ran `cargo test --test release_layout release_workflow_triggers`, then ran `git checkout` (the tree was clean after each). These mutations stayed **green**:
(a) the release guard widened to `if: github.event_name == 'push' || always()`. `b.find("if: github.event_name == 'push'")` is a substring match, so this passes, and a `dry_run: false` dispatch on a tag ref would then publish;
(b) `permissions: write-all` added to the `build` job. Only the literal `contents: write` is looked for;
(c) `id-token: write` and `actions: write` added to `publish`;
(d) the tag gate disabled (`if false && [ "${GITHUB_REF_NAME}" != ... ]`). The check only looks for the needles `GITHUB_REF_NAME`, `exit 1` and similar;
(e) `branches: [main]` added under `push:`.
DEC-014:74 says the test "fails when a mutation breaks any designed property", which is not true.
Fix: compare the guard line exactly (`l.trim() == "if: github.event_name == 'push'"`), and require it to be the `if:` of the step that contains `gh release create`. Reject any `write`/`write-all` permission outside `publish`, and allow exactly `contents: write` inside it. Require `push:`'s only child to be `tags:`. Add a red-proof for each of (a) to (e). Optionally harden the guard itself to `github.event_name == 'push' && startsWith(github.ref, 'refs/tags/v')`.
The mutations that should go red did go red: adding `pull_request`, adding top-level `contents: write`, and adding `softprops/action-gh-release@v2`. Brittleness in the other direction: reformatting `tags: ['v*']` to `tags: ["v*"]` turns the check red. That is acceptable as fail-closed, but worth a comment.

**M2 (Medium): the spec's pre-flight item 6 cannot work as written. Confirmed.** `SPEC-024...md:133` and `release.yml:61-66`. A tag `v0.0.0-rc.test` on a commit whose `Cargo.toml` says `0.1.0-alpha.1` fails `check` with "does not match", so `build` and `publish` never run. To make it reach `publish`, you would have to commit a `0.0.0-rc.test` version, which is a public tag and release of a fake version. That is outward-facing and proves little.
Fix (spec amendment): drop the throw-away tag. Rollback is exercised honestly by: (1) the dispatch dry run, which exercises check, build, smoke and SHA256SUMS, everything except `gh release create`; (2) checking the delete syntax, which I did against local `gh` 2.103.3: `gh release delete <tag> --cleanup-tag --yes` takes those flags, and `gh release create` takes `--verify-tag`, `--prerelease`, `--notes-file` and `--title`; (3) running the real delete only if the real cut is bad. Optionally rehearse the whole path in a private scratch fork. Mark item 6 `[cut]`.

**L1 (Low): spec AC 2 is ticked but not met as worded, and DEC-014 contradicts itself.** `SPEC-024...md:138` ("uses no action other than checkout and rust-toolchain") and `DEC-014...md:41` ("uses no third-party action", yet `dtolnay/rust-toolchain` is third-party). The builder disclosed the deviation and DEC-014:51 justifies it. Decision 5 (upload the dry-run artifacts) actually needs upload-artifact, so the spec was internally inconsistent.
Fix: amend AC 2 and DEC-014:41 to name the four allowed actions.

**L2 (Low): the Rosetta fallback can ship an Intel binary that never ran.** `release.yml:132-141`. With no Rosetta the step prints a warning, runs `file`, and exits 0 on a tag push too. Nothing asserts what `file` prints, and the `--version` equality check is skipped.
Fix: assert that `file` prints `x86_64`, and fail when `GITHUB_EVENT_NAME = push` and Rosetta is missing. A warning is enough on a dry run. Acceptable for an alpha as is, but the dry-run log must be read for this warning.

**L3 (Low): `publish` holds `contents: write` on a dry run, and its checkout keeps the token.** `release.yml:171-174`. This does not escalate anything: only users with write access can dispatch, and a dispatch runs the workflow file from the ref it names, so that ref could set its own permissions anyway. But `actions/checkout@v4` stores the write token in `.git/config` for later steps.
Fix: add `persist-credentials: false` to the publish checkout. Optionally split the `gh release create` step into a fourth job, `if: github.event_name == 'push'`, so that only it holds write and dry runs never do.

**L4 (Low): nothing checks that a tag is on `main` or that it is annotated.** `release.yml:61-71`. Any `v0.1.0-alpha.1` tag on any commit whose `Cargo.toml` matches publishes. Pre-flight item 1 asks for an annotated tag, which is also unchecked.
Fix: add a tag ruleset on GitHub restricting `v*` creation. Optionally, in `check` (with `fetch-depth: 0`), run `git merge-base --is-ancestor "$GITHUB_SHA" origin/main` and `test "$(git cat-file -t "$GITHUB_REF")" = tag`.

**L5 (Low): README Install says the release exists before it does.** `README.md:27` says "is on the GitHub releases page". Phase 1 merges to the public `main` and the cut is deferred until the user says go, so the commands fail until then.
Fix: "will be on ... once tagged", or land the README in Phase 2.

**L6 (Low): Linux checksum portability.** `README.md:38`. `shasum` is a Perl script that is missing on some minimal distributions (Alpine, Fedora minimal); `sha256sum -c -` takes the same input. I ran the `grep | shasum -a 256 -c -` form in a scratch directory: it passes and matches exactly one line for each of the three targets.
Fix: add "on Linux without `shasum`: `| sha256sum -c -`". Also note that `xattr -d` errors harmlessly when the attribute is absent.

**L7 (Low): `EXIF_AUDIT_BUILD_REF` is printed verbatim.** `src/bin/exif-audit/main.rs:64-69`. I built with the ref `"a b\nc\e[31m"`: `--version` printed the space, the newline and the ESC byte raw (`od -c`), so the "one line" contract breaks. The value is set at build time and the workflow sets it to 7 hex characters, so this is not untrusted input.
Fix (optional): accept the ref only when it is ASCII alphanumeric and at most 40 characters, otherwise treat it as none, plus a unit test.

**L8 (Low): `red_proof_cargo_version_change_is_rejected` is a tautology.** `tests/release_layout.rs:395-404`. It never calls the Cargo check's predicate; it asserts `replacen` did its job.
Fix: factor the check into `fn cargo_problems(toml, lock) -> Vec<String>` and red-proof that function. The builder's handback says they checked the real-file mutation by hand.

#### Checked and fine

- No `${{ }}` in any `run:` body. Every value reaches the shell through `env:`.
- The triggers are exactly a tag push and `workflow_dispatch`. The workflow default is `contents: read`.
- `gh release create` runs only for `event_name == 'push'`. On this workflow that is a tag push only, so no dispatch, whatever `dry_run` says, can create a release.
- `dry_run: false` on a branch is rejected in `check`.
- A wrong tag fails `check`, and `build` needs `check`.
- `publish` needs `[check, build]` with `fail-fast: false`. One failed leg skips `publish`, and the count check (`n = 3`) backs that up.
- The `download-artifact` pattern `exif-audit-*` does not match the `SHA256SUMS` artifact.
- The release-notes awk, run locally (BSD awk 20200816), produces exactly the 7 lines of the `[0.1.0-alpha.1]` section and stops at `## Development history`. The non-empty guard works. The runner's mawk is unverified.
- The old `[Unreleased]` content was moved, not lost: no removed lines against `784576a`.
- "Seven fuzz targets" matches `ci.yml`.
- `cut -c1-7` is used both to build the ref and to compute the expected `--version`, so the equality check is self-consistent.
- The smoke step runs under the runner default `bash -e`, and the `&& code=0 || code=$?` form is exempt from `-e`, so the logic is correct.
- `version_line`: with no ref and with an empty ref it prints `exif-audit 0.1.0-alpha.1`; a local plain build confirms this.
- No CLI test file changed.
- `cargo publish --dry-run` fails with "cannot be published" (from `publish = false`).
- The `cargo package --list` include list is unchanged in kind (src, licences, README, Cargo files).
- A release build with `--locked` passes, so `Cargo.lock` is consistent.
- `0.1.0-alpha.1` is valid SemVer.
- README commands match the archive names `exif-audit-<version>-<target>.tar.gz`, and `gh release download --pattern` (repeatable) and `--repo` are valid flags.
- `ci.yml` is byte-identical to `origin/main`. The local `main` ref is stale (11a5141), so it shows a 4-line difference, but that difference comes from SPEC-022/023, which are already on `origin/main`.

#### Gates (re-run here, `RUSTUP_TOOLCHAIN=1.90.0`)

`just lint` 0, `just test` 0 (934 passed, 0 failed, 10 ignored across 23 test binaries; `release_layout` 12 of 12), `just deny` 0 (licenses ok; the known "Zlib unmatched allowance" warning), `just forbid-unsafe` 0, `just validate` 0, `just cost-audit` 0.

#### Could not verify here (check in the orchestrator's dry run)

- That the workflow parses and runs on GitHub at all.
- The `x86_64-unknown-linux-musl` link with no musl-tools.
- The `x86_64-apple-darwin` cross-build, and whether `macos-latest` has Rosetta. **Read the log for the "Rosetta is not available" warning; if it appears, the Intel binary was not smoke-run (L2).**
- `actions/*-artifact@v4` with `merge-multiple`.
- mawk running the notes awk (read the `cat notes.md` output in the log).
- `shasum` on ubuntu-latest.
- `gh release create` itself, which only the tag push exercises.

In the dry run, check:
- `check` logs "not a tag ref ... dry run only".
- The three build legs pass, each logging `smoke checks passed for <target>`.
- `publish` logs the 3-line SHA256SUMS and `OK` lines, and the step "create the pre-release" shows **skipped**.
- Download the artifacts and run `shasum -a 256 -c SHA256SUMS`.
- Unpack the arm64 archive outside the repo and run `--version`; it should print `exif-audit 0.1.0-alpha.1 (<7-char sha of the dispatched commit>)`.
- Run `file` on the Intel binary; it should say x86_64.
- `tar -tzf` each archive and confirm the 4 files.

### Cost self-report

- **Tokens (total):** 1,982,859. This is a floor, measured live before the last ~4 calls: 20 deduplicated messages, input 39, cache creation 117,067, cache read 1,849,191, output 16,562.
- **Estimated USD:** null. `docs/cost-tracking.md` lists no claude-opus-5-5 rate; the orchestrator prices it from the components.
- **Duration (minutes):** about 25. About 25 tool calls.
- **Source of the number:** the session transcript `subagents/agent-af98cafc183101310.jsonl`, measured per docs/cost-tracking.md "Measuring a cycle". This basis is not comparable with the build's harness `subagent_tokens` figure.

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:** none by the verifier. No code was changed: every mutation was reverted with `git checkout` and the tree was clean after each. The probe builds went to the scratchpad `CARGO_TARGET_DIR`.
- **Follow-up work identified:**
  - Before the cut: fix M1 (harden the tests), amend the spec for M2 and L1, and fix L5.
  - Optional for the alpha: L2, L3, L4 (the tag ruleset is a GitHub setting the user must apply), L6, L7 and L8.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Very little; the handoff named the likely weak spots. The local `main` ref is stale, so "byte-identical to main" first looked like a failure; the comparison should name `origin/main`.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — Spec Decision 3 (only two actions) contradicts Decision 5 (upload dry-run artifacts), which needs upload-artifact. The spec should also have stated the guard's exact semantics: tag push only, or also a `dry_run: false` dispatch on a tag.

3. **If you did this task again, what would you do differently?**
   — Wrap cargo in a function from the start: zsh does not word-split `$C`, which cost one wasted mutation run.
