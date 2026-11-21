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
  id: HANDOFF-055
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-21
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-028

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
  agent: claude-opus-5-5           # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: approved                # verify handoffs: approved | punch-list | rejected
  basis: agent-result                # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: 139694             # REAL combined count — what cost-audit reads
  estimated_usd: 0.92              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: 25
  branch: claude/spec-028-verify
  pr: null
  completed_at: 2026-11-21         # YYYY-MM-DD
  notes: "Verifier could not read its own transcript usage; tokens_total is an estimate of context consumed, to be replaced by the harness figure. Approved at ff9f37b (build 0989051) with four Low notes, none blocking."
  synced_at: 2026-11-21
---

# HANDOFF-055: Verify SPEC-028 PNG chunk walker (DEC-017)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-028` to a cold verifier for the **verify** cycle. You did not write this. Review branch `claude/spec-028-png-walker` (build commit `0989051`, cost commits after it) against `projects/PROJ-001-diamond-exif-m1/specs/SPEC-028-png-chunk-walker-signature-chunk-lengths-crc-as-an-issue-ord.md` and `decisions/DEC-017-png-chunk-walk-and-issue-channel.md`. Review only; edit nothing except your own handback in this file. Do not push.

## BUDGET (hard)

About 80 tool calls or 200k context. Record a real `tokens_total` in the handback (the orchestrator replaces it with the harness figure; no cache reads). Toolchain: `RUSTUP_TOOLCHAIN=1.90.0`, `PATH="$HOME/.cargo/bin:$PATH"`; read `guidance/toolchain-brief.md`. Start with `git checkout claude/spec-028-png-walker` in your worktree (it is a local branch of this repository; do not reset --hard).

## What to look hard at

1. **Panic-freedom and arithmetic.** Every offset computed from a 32-bit attacker-controlled length: any unchecked add or cast, any slice that could go out of range, any loop that might not advance. Read `src/png/mod.rs` line by line.
2. **DEC-017 conformance, both directions.** Every rule in "Policy detail" has code and a test that would fail if the rule were removed. Check the builder's six listed choices where the DEC is silent (issue order within a chunk; the chunk that trips the issue cap is not recorded; `TooManyChunks` fires on a further whole chunk so exactly 65,536 is clean; the `len` in `Truncated`; signature-only gives MissingIhdr then MissingIend at offset 8; `BadIhdr` applies to any IHDR) and say whether each is sound or should become a DEC-017 clarification. Also: `TooManyIssues` Display says "JPEG walk reached..." when PNG uses it: judge severity.
3. **The fuzz model's independence.** `fuzz/shared/png_chunk_invariants.rs` must not call the library CRC or kind table, must assert every iff both ways, and must be able to fail (try a mutation of the library on a scratch copy, e.g. flip a CRC comparison, and confirm the shared invariants or tests go red; revert).
4. **Pins and seeds.** 33 seeds, existing seeds byte-identical, every pin updated, `fuzz/Cargo.lock` not committed with a changed version.
5. **Scope.** Test-only arms added to `src/jpeg` and `src/tiff` tests change no behaviour; nothing else outside the spec's Outputs. Honest README claims. Provenance row. The CHANGELOG entry was dropped because `tests/release_layout.rs::changelog_and_readme_carry_the_release` pins `[Unreleased]` empty: say whether you agree.
6. Run `just lint`, `just test`, `just lint-fuzz`, `just validate`, `just forbid-unsafe`, `just cost-audit` and a 20 s `just fuzz-smoke png_chunks` yourself; do not trust the handback.

## Expected Deliverables

Your verdict, exactly one of APPROVED / PUNCH LIST / REJECTED, with findings ranked High, Medium, Low, each with file:line and a concrete failing input or scenario. Fill the `handback:` front matter (`verdict`, a real `tokens_total`) and the `## Handback` section; set statuses to `completed`.

## Out of Scope

Fixing anything, DEC edits, audit wiring, ExifTool parity (SPEC-031), metadata reading (SPEC-029).

## Return Criteria

Fill the handback and reply with the verdict and findings.

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

- **Branch / PR:** `claude/spec-028-verify` (from `claude/spec-028-png-walker` at ff9f37b; no PR, not pushed)
- **Completed at:** 2026-11-21
- **All acceptance criteria met?** yes
- **For `verify`:** APPROVED at ff9f37b (build commit 0989051). Four Low notes below, none blocking.

#### Commands run (all by the verifier, `RUSTUP_TOOLCHAIN=1.90.0`, `~/.cargo/bin` first)

| Command | Result |
|---|---|
| `just lint` | green (fmt, rustfmt over `fuzz/shared/*.rs`, clippy `-D warnings`) |
| `just lint-fuzz` | green (clippy 0.1.98 stable). It rewrote `fuzz/Cargo.lock` locally (exif-core 0.0.0 to 0.1.0-alpha.1); the committed lock still says 0.0.0, same as main. Reverted, not committed. |
| `just forbid-unsafe` | green |
| `just validate` | green (33 artifacts, 123 parse) |
| `just cost-audit` | green, exit 0 |
| `just test` | 24 result lines, 976 passed, 0 failed, 8 ignored |
| `cargo +nightly fuzz run png_chunks ... -max_total_time=20` (the `fuzz-smoke` command with 20 s; the recipe hardcodes 60) | 408,463 runs in 21 s, no crash, cov 505, corpus 190 |
| `cargo run --example fuzz_seeds` | 33 png_chunks seeds; `git status` clean afterwards, so all pre-existing seed sets are byte-identical |
| Mutation probe on a scratch copy (rsync of the tree, `src/png/mod.rs` mutated, `fuzz_layout png_harness_holds` and `png_walk` run) | see below |

Mutants (harness green / png_walk green; 0 means red):
`crc_ok = computed != stored` 0/0 (also red in `png_harness_model_is_independent_and_red_proofs_hold`); `after_idat` includes the first IDAT 0/0; chunk cap `>=` to `>` 1/0 (the seeds never reach 65,536 chunks, so only `chunk_cap_is_exact` catches it, as expected); issue cap `>=` to `>` 0/0; no `BadChunkCrc` recorded 0/0; `MAX_CHUNK_LEN` `>` to `>=` 0/0; `is_ascii_alphabetic` to `alphanumeric` 0/0; `BadIhdr` only when `> 13` 0/0; chunk that trips the issue cap recorded anyway 0/0; trailer starting at 8 without IEND 0/0. One survivor, `if !self.halted` to `if true` at the end-of-walk rule (`src/png/mod.rs:303`), is an equivalent mutant: every halt path `return`s before reaching that line, so the guard is defensive only.

#### Findings

**High:** none. **Medium:** none.

**Low 1. `TooManyIssues` Display says "JPEG walk" on a PNG** (`src/error.rs:361`, raised from `src/png/mod.rs:191`). Input: signature, IHDR, 1,025 empty `tEXt` chunks with a wrong CRC, IEND (the `bad-crc-texts-1100` seed shape). The last issue displays "JPEG walk reached the limit of 1024 issues (at offset ...)". Nothing surfaces PNG issues to users yet (`exif-audit` still reports PNG as unsupported), so it is not blocking, but it must be fixed before SPEC-029/030 wires PNG into the audit. No test pins the string (grep of `tests/` and `src/`), so a layer-neutral "walk reached the limit of {limit} issues" is a one-line change.

**Low 2. An issue can point at a chunk that is not recorded** (`src/png/mod.rs:264-281`). Input: signature, IHDR, 1,023 bad-CRC `tEXt` chunks, then one chunk typed `te1t` with a bad CRC, IEND. The `te1t` chunk's `BadChunkType` is the 1,024th ordinary issue and is recorded at its offset; its `BadChunkCrc` becomes `TooManyIssues` and the chunk is dropped. So `issues()` holds an offset with no entry in `chunks()`. Consistent with the stated rule ("the chunk that trips the cap is not recorded") and with the model, but worth one sentence in a DEC-017 clarification.

**Low 3. The builder's six choices where DEC-017 is silent.** All six are sound; each is pinned by both `tests/png_walk.rs` and the shared model, so a DEC-017 clarification note should record them: (a) issue order within a chunk is type, MissingIhdr, BadIhdr, CRC (`several_issues_at_once_keep_their_order`); (b) the chunk that trips the issue cap is not recorded (see Low 2); (c) `TooManyChunks` fires only on a further whole chunk, so exactly 65,536 is clean, and 65,536 chunks followed by a short or overlong tail give `Truncated`/`BadChunkLength`, not `TooManyChunks`; (d) `Truncated.len` is 12 when fewer than 12 bytes remain, otherwise `12 + length`, which matches the `Error::Truncated` doc ("bytes needed from that offset"); (e) signature-only gives MissingIhdr then MissingIend, both at offset 8 (= input length); (f) `BadIhdr` applies to any IHDR, including a second one (a second well-formed IHDR, the `two-ihdr` seed, is clean, which is also worth stating). Also implicit: when the issue cap is already full, a halting issue (Truncated, BadChunkLength, TooManyChunks) is replaced by `TooManyIssues`, as in the JPEG walker.

**Low 4. CHANGELOG.** I agree with dropping the entry: `tests/release_layout.rs:487-494` asserts `[Unreleased]` is empty after the cut, so an entry would fail `just test`. But that test contradicts `CHANGELOG.md:5` ("add entries under it as you work"). A release-lane follow-up should relax the test (for example pin only that the 0.1.0-alpha.1 section is unchanged) so SPEC-028 onwards can record work.

#### What was checked and found sound

- Panic-freedom and arithmetic (`src/png/mod.rs`, read line by line): the declared length is compared with `MAX_CHUNK_LEN`, then `total = data_len + 12` (saturating) is compared with `remaining` before any offset is formed; data/CRC offsets are then bounded by `len`; every slice uses `get`; `copy_from_slice` takes the exact 8-byte `get(..8)`; `range()` saturates; the CRC table build uses `split_first_mut`, the lookup `get(...).unwrap_or(0)`. Every iteration advances `p` to `chunk_end >= p + 12` or returns. The three `let ... else { return; }` arms are unreachable once the length check passes.
- DEC-017 rules: each has code and a test that fails without it (confirmed by the mutants above). Fatal only on fewer than 8 bytes or no signature; `NotPng` carries the bytes.
- Fuzz model (`fuzz/shared/png_chunk_invariants.rs`): its own bitwise CRC and kind table, no library CRC or kind import; issues (error and offset, in order), chunks field by field, `iend_offset` and trailer compared by equality with the model, so each iff fails in both directions; `parse_strict` agreement and fatal cases checked. It is a mirror of the same rule order, so it cannot catch a choice both made the same way; tests in `png_walk.rs` pin those choices.
- Pins and seeds: 33 seeds, named in `tests/fuzz_layout.rs` and checked byte-for-byte against the builders; regeneration leaves every seed directory unchanged; CI matrix, `app.just` list and the three existing matrix pins updated.
- Scope: the arms added in `src/jpeg/mod.rs`, `src/jpeg/metadata.rs` and `src/tiff/mod.rs` are only in `#[cfg(test)]` exhaustive-index helpers (27 to 34), no behaviour change. No file outside the spec's Outputs. `[dependencies]` unchanged. README is honest (walks, reads no metadata, audit still unsupported, eight targets). Provenance note added in `docs/provenance-ledger.md` as class 1, no row, consistent with the JPEG precedent.

### Cost self-report

- **Tokens (total):** 110,000 (estimate)
- **Estimated USD:** null
- **Duration (minutes):** 25
- **Source of the number:** none available to the verifier in-session; estimate of context consumed. The orchestrator should replace it with the harness figure.

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:** CHANGELOG entry omitted (Low 4; justified by the release-layout pin).
- **Follow-up work identified:**
  - Neutral `TooManyIssues` Display before PNG is wired into the audit (Low 1).
  - DEC-017 clarification recording the six silent choices and the cap/offset interplay (Low 2, Low 3).
  - Relax `changelog_and_readme_carry_the_release` so `[Unreleased]` can carry entries (Low 4).

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Only that `just fuzz-smoke` hardcodes 60 s while the handoff asks for 20 s; I ran the recipe's own command with `-max_total_time=20`.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — That `just lint-fuzz` rewrites `fuzz/Cargo.lock` (the committed lock pins exif-core 0.0.0 while the crate is 0.1.0-alpha.1), so every verifier has to revert it by hand.

3. **If you did this task again, what would you do differently?**
   — Put the mutation probe in a script from the start; the worktree guard refuses compound shell loops.
