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
  id: HANDOFF-008
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-05
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-003

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
  tokens_total: 143237             # orchestrator-recorded from the metered sub-agent result (subagent_tokens)
  estimated_usd: 0.95
  duration_minutes: 6
  branch: worktree-agent-a6cd3ffb2817859ef
  pr: null
  completed_at: 2026-11-05         # YYYY-MM-DD
  notes: "verdict APPROVED at 6981332 (code de5217c); tokens transcribed by the orchestrator from the sub-agent metered result"
  synced_at: 2026-11-05
---

# HANDOFF-008: Verify the tolerant-walk policy

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-003` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-003-tolerant-walk.md` (walk rules and named tests are normative)
- **Build handoff and its Handback:** `HANDOFF-007-build-tolerant-walk.md` (claims to verify, not to trust)
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`)
- `decisions/DEC-001-tolerant-walk-policy.md` (with the SPEC-003 clarifications), DEC-002, DEC-004
- Code under review: `git diff 6f4e022..HEAD` on the current branch (build commit `de5217c`)

## Expected Deliverables

The verify contract: review cold, and output exactly ONE of APPROVED / PUNCH LIST / REJECTED, at a commit SHA.

Verify independently, do not rely on the builder's report:
- Re-run every gate in the spec's acceptance criteria yourself.
- Re-do at least the four mutation checks (depth `>=`, depth `+1`, attempts `>=`, attempts `+1`) and confirm each named test goes red, then revert (leave the tree clean).
- Plant `is_clean()` returning constant `true` and confirm the seed test or fuzzer catches it; also try a plant that violates containment or the issue bound if you can make one cheap.
- Read the walk against the Walk rules line by line: attempt counting (IFD0 is attempt 1), per-branch depth stop, global halt, `Link`/`from_ifd`/`offset` per issue, atomic omission, malformed pointer tag keeps the holding IFD, IFD0 fatal paths. Look for any panic-capable construct in non-test code and any unbounded loop or allocation.
- Check that flipped tests really assert BOTH the tolerant outcome and the strict `Err`, that no test is vacuous, and that the fuzz target's differential invariant is both-directional.
- Run `just fuzz-smoke` (60 s) and record executions and coverage.
- Run `just decisions-audit --changed`; check deviations the builder listed (`ParseIssue::into_error`, `Walk` struct, helper renames) for acceptability.
- Do not fix code. Report findings. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes (the orchestrator routes a punch list back to a builder)
- SPEC-004 work, CI wiring, the 24 h fuzz run

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

- **Branch / PR:** worktree branch `worktree-agent-a6cd3ffb2817859ef` (fast-forwarded to `6981332`); handback committed there. Not pushed, no PR, no cycle advanced.
- **Completed at:** 2026-11-05
- **All acceptance criteria met?** yes (each re-run independently; evidence below).
- **For `verify`:** ✅ **APPROVED** at commit `6981332` (code under review: build commit `de5217c`, `git diff 6f4e022..HEAD`).

**Gates (re-run by the verifier, all green):** `cargo +1.90.0 fmt --all -- --check`; `just lint` (clippy 0.1.90, `-D warnings`, five-lint set; needs `RUSTUP_TOOLCHAIN=1.90.0` when `~/.cargo/bin` is first in PATH, see toolchain brief); `just lint-fuzz`; `rustfmt --check` on `fuzz/fuzz_targets/tiff_ifd.rs`; `cargo test` on 1.90.0 and on rustup stable (36 lib + 4 + 4, 0 failed); `cargo +1.90.0 rustc --lib -- -F unsafe_code`; `just deny`, `just deny-fuzz` (licenses ok; only the pre-existing "unmatched license allowance" warning); `just validate`; `just decisions-audit` (4 clean); `just decisions-audit --changed 6f4e022` (no active DEC scope matches); `just cost-audit`. `[dependencies]` still empty; no file under `scripts/` or irradiance touched. `cargo run --example fuzz_seeds` regenerates the 20 seeds with zero diff (deterministic; the 17 pre-existing seeds are byte-identical to `6f4e022`).

**Mutation checks (verifier re-run, each reverted, tree clean):**

| Mutation | Tests that went red |
|---|---|
| (a) `depth > MAX_IFD_DEPTH` -> `>=` | `nesting_past_the_depth_limit_terminates`, `nesting_at_exactly_the_depth_limit_is_clean` |
| (b) `depth > MAX_IFD_DEPTH` -> `> MAX_IFD_DEPTH + 1` | `nesting_past_the_depth_limit_terminates` |
| (c) `attempts > MAX_IFDS` -> `>=` | `a_long_chain_stops_at_the_ifd_limit`, `a_chain_of_exactly_max_ifds_is_clean`, `a_hostile_subifds_array_cannot_flood_the_issue_list` |
| (d) `attempts > MAX_IFDS` -> `> MAX_IFDS + 1` | `a_long_chain_stops_at_the_ifd_limit`, `a_hostile_subifds_array_cannot_flood_the_issue_list` |

Matches the builder's table exactly.

**Plants (each reverted, tree clean):**

| Plant | Caught by |
|---|---|
| P1 `is_clean()` returns `true` | lib `a_self_referential_subifd_terminates`; seed test `every_seed_parses_or_errors_without_panicking` ("strict Err but is_clean"); libFuzzer crash on the differential assert (`tiff_ifd.rs:59`, deadly signal) within the first run |
| P2 containment: a failed IFD recorded as a phantom empty IFD | seed test ("IFD escapes the input"); lib `a_bad_subifd_is_omitted_atomically`, `a_corrupt_ifd1_keeps_ifd0`, `a_corrupt_subifd_leaves_ifd0_readable`, `one_bad_subifd_does_not_hide_its_siblings` |
| P3 issue bound: `halted` never set (budget per-branch) | `a_hostile_subifds_array_cannot_flood_the_issue_list` |
| P4 IFD0 not counted as attempt 1 | `a_long_chain_stops_at_the_ifd_limit`, flood test |
| P5 malformed `SubIFDs` tag also ends the holder's chain | `a_malformed_subifds_tag_costs_the_branch_not_the_ifd` |
| P6 wrong `from_ifd` on a `Link::Next` follow | `a_long_chain_stops_at_the_ifd_limit` |

**`just fuzz-smoke` (60 s, clean tree):** clean, no crash. 1,064,119 executions in 61 s (~17.4k exec/s); `INITED cov: 534 ft: 996` from 20 seeds; final `cov: 643 ft: 2947 corp: 456/221Kb`.

**Code read against the Walk rules:** IFD0 read directly via `read_ifd` with `?` (fatal, atomic, including the next-word) and seeded as attempt 1 / `visited = [ifd0]`; `follow` order is attempts (`saturating_add`) -> global halt -> per-branch depth -> cycle (push) -> `read_ifd` (omit whole on `Err`, chain ends); chain iterative in `walk_from`, recursion only through `walk_sub_ifds` and bounded by the depth check (max 9 frames); `halted` checked before each sub-offset and each chain step; malformed `SubIFDs` -> `Tag(330)`, `offset None`, holder kept, chain continues; `from_ifd` always an index already pushed. Issue bound holds by construction (≤ 512 follow issues for attempts 2..=513, ≤ 512 pointer-tag issues). No `unwrap`/`expect`/indexing/unchecked arithmetic in non-test code (clippy five-lint set green). `parse_strict` agrees with the spec (first issue by value via `into_error`). Fuzz differential is both-directional (strict `Ok` ⇒ clean; strict `Err` ⇒ `!is_clean` and equal to first issue; fatal ⇒ equal `Err`). Every flipped test asserts both the tolerant outcome and the strict `Err`; no vacuous test found.

**Builder deviations — all acceptable:** `ParseIssue::into_error` is `pub(crate)` (spec's own note asks for by-value take); private `Walk` struct and `walk_from`/`walk_sub_ifds`/`follow` replace `walk_chain`/`sub_ifd_offsets_of_last` (module docs updated); `nested`/`chained` test helpers; strengthened `!is_clean` assert. `a_clean_container_reports_no_issues` was not added as a separate test: the spec said to extend the existing SubIFD-order test, which was done (`subifd_offsets_are_walked_in_order_and_recorded_with_their_parent` asserts `is_clean()` and `issues().is_empty()`).

**Findings (none blocking):**

1. **LOW — zero sub-offsets and the `SubIFDs` decode are outside the attempt budget.** Rule 4 skips a `0` sub-offset silently without consuming an attempt, and `uints` materialises up to `MAX_TAG_VALUES` (1 Mi) offsets per holding IFD before the `halted` check. Measured with a temporary probe (deleted): 512 chained IFDs whose `SubIFDs` (count 2^20) all share one 4 MiB block of zeros, a 4.2 MB input, parse clean in 0.34 s release / 5.9 s debug, with 512 transient 4 MiB allocations. Bounded (O(`MAX_IFDS` × `MAX_TAG_VALUES`)), and spec-sanctioned, but rule 6's "every pointer follow consumes the attempt budget" does not cover this cost. The 4 KiB fuzz length cap cannot find it. Suggest SPEC-004 (which adds more pointer tags) caps the offsets read from a pointer tag, or charges zero entries / decode against a work budget.
2. **INFO — `CyclicIfd` also names non-cycles.** `visited` is pushed before `read_ifd`, so a second pointer to an already-read offset (a diamond, not a cycle) or to an offset that already failed reports `CyclicIfd` rather than the real cause. Consistent with Walk rule 3.3 and SPEC-002 semantics; worth wording carefully when the audit CLI renders issues.
3. **INFO — `Error: Clone + PartialEq + Eq` is now public API.** A future variant carrying a float would break `Eq` (builder noted this too). `ParseIssue` being `#[non_exhaustive]` with private fields is redundant but harmless.
4. **INFO — `just lint` host trap.** With `~/.cargo/bin` first in PATH (needed for clippy 0.1.90) `just lint` fails at `cargo fmt` on the nightly default unless `RUSTUP_TOOLCHAIN=1.90.0` is set. Already described in the toolchain brief; not a code defect.

### Cost self-report

- **Tokens (total):** 143,237 (recorded by the orchestrator from the sub-agent metered result).
- **Estimated USD:** 0.95
- **Duration (minutes):** ~6 (metered wall-clock)
- **Source of the number:** harness report (subagent_tokens)

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:** none beyond the builder's listed ones, all accepted above.
- **Follow-up work identified:**
  - SPEC-004: bound the per-pointer-tag offset decode / zero-entry scan (finding 1), and repeat the corrupt-pointer test with the real GPS pointer (existing carry-in).
  - Audit CLI: render `CyclicIfd` as "pointer revisits offset" rather than asserting a cycle (finding 2).

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Little. The one friction was that `cargo test` stops at the first failing binary, so a plant caught by the lib tests hides whether the seed test also catches it; I had to re-run `--test fuzz_layout` alone. The handoff could say "run the seed test on its own for the plant".

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — A work bound, not just a count bound: DEC-001's clarification bounds attempts and issues but not decode work per pointer tag, which is where the remaining amplification lives.

3. **If you did this task again, what would you do differently?**
   — Script the mutations and plants up front (the worktree guard refuses compound git commands, so a script file is the practical form) and add the amplification probe earlier, since it was the only finding that needed measurement.
