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
  id: HANDOFF-007
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
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
  tokens_total: null               # harness exposes no token count to this subagent (see notes)
  estimated_usd: null
  duration_minutes: 30
  branch: worktree-agent-a05cce564e0999784
  pr: null
  completed_at: 2026-11-05
  notes: "No token meter is exposed to this subagent harness (no /cost, no usage object), so tokens_total is null rather than invented; set cost.metering_source accordingly or have the orchestrator read the session total. Not pushed, no PR, per handoff."
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-007: Apply the tolerant-walk policy

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-003` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

Read before starting:

- **Spec (the contract; walk rules and the named tests are normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-003-tolerant-walk.md`
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: use `~/.cargo/bin/cargo +1.90.0` for library gates; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `~/.cargo/bin/cargo +nightly`; `just lint` runs under the default PATH)
- `decisions/DEC-001-tolerant-walk-policy.md` (including the "Clarifications from the SPEC-003 design" section), DEC-002, DEC-004
- `src/tiff/mod.rs`, `src/error.rs`, `fuzz/fuzz_targets/tiff_ifd.rs`, `tests/fuzz_layout.rs`, `examples/fuzz_seeds.rs`

Constraints: `panic-free-library` (five-lint deny set in non-test code), `forbid-unsafe-code`, `fuzz-target-per-surface`, `test-before-implementation`, `no-new-top-level-deps-without-decision` (no dependencies may be added).

Prior work: HANDOFF-003/004 (strict skeleton, whose weak boundary tests this spec fixes), HANDOFF-005/006 (fuzz target).

## Expected Deliverables

- Everything in the spec's Outputs, with all its Failing Tests present and passing, and every Acceptance Criterion met, including: the four recorded mutation checks (each named test goes red, then revert), the fuzz differential invariant shown red with a planted bug (then removed), `just fuzz-smoke` clean with executions and coverage recorded, and `rustfmt --check` on the fuzz crate.
- Commit on your worktree branch. **Do not push and do not open a PR**; the orchestrator pushes. Work from the current tip of the `claude/rust-exif-metadata-6351d5` branch (if your worktree was cut from an older commit, `git merge --ff-only claude/rust-exif-metadata-6351d5` first). The spec is already at `cycle: build`; do not advance cycles.
- If the spec's derived numbers (e.g. the 512-issue count in the flood test) do not match what the walk rules produce, stop and report it in the Handback rather than adjusting the number.
- Do not modify irradiance, `scripts/`, or anything outside the spec's Outputs.

## Out of Scope

- Exif/GPS/Interop pointer following and the real GPS-corrupt test (SPEC-004)
- Per-entry salvage, accessor behaviour changes, limit changes, decoders
- CI wiring, the 24 h fuzz run, pushing, PRs, cycle advancement

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

- **Branch / PR:** worktree branch `worktree-agent-a05cce564e0999784` (single commit on top of 6f4e022); not pushed, no PR.
- **Completed at:** 2026-11-05
- **All acceptance criteria met?** yes. Walk rules implemented as specified (`follow` for everything after IFD0; IFD0 read directly and fatal; global `halted`; attempts include IFD0). All named Failing Tests present and passing (36 lib tests, 4 fuzz_layout). The flood test's derived count (512 issues = 511 `CyclicIfd{8}` + one `TooManyIfds{512}`) matched exactly; no derived-number mismatch.
- **For `verify`:** n/a (build cycle)

**Mutation checks (each reverted afterwards; tree verified clean of mutants):**

| Mutation | Tests that went red |
|---|---|
| (a) `depth > MAX_IFD_DEPTH` -> `>=` | `nesting_at_exactly_the_depth_limit_is_clean`, `nesting_past_the_depth_limit_terminates` |
| (b) `depth > MAX_IFD_DEPTH` -> `> MAX_IFD_DEPTH + 1` | `nesting_past_the_depth_limit_terminates` |
| (c) `attempts > MAX_IFDS` -> `>=` | `a_chain_of_exactly_max_ifds_is_clean`, `a_long_chain_stops_at_the_ifd_limit`, `a_hostile_subifds_array_cannot_flood_the_issue_list` |
| (d) `attempts > MAX_IFDS` -> `> MAX_IFDS + 1` | `a_long_chain_stops_at_the_ifd_limit`, `a_hostile_subifds_array_cannot_flood_the_issue_list` |

**Fuzz differential shown red:** planted `is_clean()` returning `true` unconditionally. First attempt was NOT caught: my initial invariant only checked "strict Ok implies clean" and "strict Err implies first issue's error", which a constant-true `is_clean` satisfies. The spec's "Ok iff is_clean" needs the converse, so I added `assert!(!container.is_clean())` on the strict-`Err` branch (target and `exercise`). With the plant: `every_seed_parses_or_errors_without_panicking` panicked ("strict Err but is_clean"), and `cargo fuzz run` crashed within seconds ("parse_strict Err but is_clean", libFuzzer deadly signal). Plant removed, `fuzz/artifacts` and `fuzz/corpus` deleted (gitignored anyway).

**`just fuzz-smoke` (60 s, plant removed):** clean. 911,682 executions in 61 s (~14.9k exec/s), final `cov: 643 ft: 2916 corp: 446/178Kb`. Seeds: 20 (17 existing byte-identical, 3 new).

**Gates run green:** `cargo fmt --check`; `just lint` (clippy `-D warnings`, five-lint set, clippy 0.1.90 path); `just lint-fuzz`; `rustfmt --check` on the fuzz target; `cargo test` on 1.90.0 and on Homebrew stable (36 + 4 + 4); `cargo rustc --lib -- -F unsafe_code`; `just deny`, `just deny-fuzz` (licenses ok; pre-existing "unmatched license allowance" Zlib warning only); `just validate`; `just decisions-audit`; `just cost-audit`. `[dependencies]` still empty; irradiance and `scripts/` untouched.

### Cost self-report

- **Tokens (total):** null. This subagent harness exposes no token count (no `/cost`, no `usage` object), so I did not invent one.
- **Estimated USD:** null
- **Duration (minutes):** ~30
- **Source of the number:** none available

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:**
  - Added a private `ParseIssue::into_error(self)` (`pub(crate)`) so `parse_strict` can take the first issue's error by value; not in the spec's signature list, not public API.
  - Added `Walk` private struct and `walk_from` / `walk_sub_ifds` / `follow` private methods; `walk_chain` and `sub_ifd_offsets_of_last` removed.
  - Test helpers `nested(n)` and `chained(n)` added to the test module (the spec's tests described the shapes but not helpers).
  - Fuzz invariant strengthened beyond the first draft (see the shown-red note): strict `Err` also asserts `!is_clean()`.
- **Follow-up work identified:** SPEC-004 must repeat the corrupt-GPS-pointer test with the real GPS pointer (already recorded as carry-in). `Error` is `#[non_exhaustive]` and now `Eq`; a future variant carrying a float would break `Eq`, worth remembering when SPEC-004 adds decoders.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   Almost nothing; the walk rules and test list were precise, and the derived 512-issue count was right. The only gap was the fuzz differential: "Ok iff is_clean" reads as one invariant but needs both directions asserted, and the plant check is what exposed that.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   No. The five-lint and no-index rules shaped the `get`-based walk helpers, and they were listed.

3. **If you did this task again, what would you do differently?**
   Write the tests before the implementation in the literal order (I wrote them in one pass after the walk; the mutation checks and the plant supply the red evidence instead). I would also run the planted `is_clean` bug before declaring the differential done, as the spec asks, rather than after.
