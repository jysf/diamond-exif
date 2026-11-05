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
  id: HANDOFF-009
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-05
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-004

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
  tokens_total: 185074             # orchestrator-recorded from the metered sub-agent result (subagent_tokens)
  estimated_usd: 1.22
  duration_minutes: 10
  branch: worktree-agent-aa5abcc3ef1eff7d7
  pr: null
  completed_at: 2026-11-05         # YYYY-MM-DD
  notes: "tokens transcribed by the orchestrator from the sub-agent metered result"
  synced_at: 2026-11-05
---

# HANDOFF-009: Multi-IFD pointer walk

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-004` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

Read before starting:

- **Spec (walk rules and named tests are normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-004-multi-ifd-pointer-walk.md`
- **`decisions/DEC-005-pointer-tag-walk-policy.md`** (the pointer table; normative), `DEC-001` (with clarifications), DEC-002, DEC-004
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `~/.cargo/bin/cargo +nightly`; `just lint` runs under the default PATH)
- `src/tiff/mod.rs` (`Walk`, `walk_from`, `walk_sub_ifds`, `follow`, tests, `Build`), `src/tiff/issue.rs`, `src/tiff/tags.rs`, `src/error.rs`, `fuzz/fuzz_targets/tiff_ifd.rs`, `tests/fuzz_layout.rs`, `examples/fuzz_seeds.rs`
- Prior art: HANDOFF-007 (SPEC-003 build) and HANDOFF-008 (verify; finding 1 is carried into this spec)

Constraints: `panic-free-library` (five-lint deny set in non-test code), `forbid-unsafe-code`, `fuzz-target-per-surface`, `test-before-implementation`, `no-new-top-level-deps-without-decision` (no dependencies).

## Expected Deliverables

- Everything in the spec's Outputs with all its Failing Tests present and passing, and every Acceptance Criterion met, including: the five recorded walk plants (each named test goes red, then revert), the fuzz invariants shown red with planted bugs (then removed), `just fuzz-smoke` clean with executions and coverage recorded, and `rustfmt --check` on the fuzz crate.
- **Write the fuzz invariant in both directions where it is an iff**; last time a one-directional check let a constant-`true` plant through.
- Commit on your worktree branch. **Do not push and do not open a PR.** The spec is already at `cycle: build`; do not advance cycles. First run `git merge --ff-only claude/rust-exif-metadata-6351d5` if your worktree was cut from an older commit (you need commit `0d5e381` or later).
- If a derived number in the spec (e.g. the 512-issue count) does not match what the rules produce, stop and report it in the Handback rather than adjusting it.
- Do not modify irradiance, `scripts/`, or anything outside the spec's Outputs. Existing 20 fuzz seeds must remain byte-identical.
- You cannot see your own token count; set `tokens_total: null` with a note, the orchestrator transcribes the metered figure.

## Out of Scope

- Decoders (SPEC-007), `parse_fields` and oracle work (SPEC-005), MakerNote, tag interpretation
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

- **Branch / PR:** `worktree-agent-aa5abcc3ef1eff7d7`, one commit on top of bad5fcb; no push, no PR (per instructions).
- **Completed at:** 2026-11-05
- **All acceptance criteria met?** yes. Walk, kinds, `ifd_of_kind`, GPS tolerance, rule-5 failure modes, the `SubIFDs` cap (100,000-count test plus exactly-512 test), `MAX_ISSUES`, five walk plants red, fuzz plants red, fuzz-smoke clean, gates green (below). The orchestrator re-runs the gates independently.
- **Derived number:** the 512-issue count for a `SubIFDs` array of exactly 512 offsets matched the rules (511 `CyclicIfd` then one `TooManyIfds`). No mismatch.

**Walk plants (each reverted; `cargo +1.90.0 test --lib` red, then green after revert):**

| Plant | Named tests that went red |
|---|---|
| (a) `0xA005` allowed from any holder | `pointer_tags_in_unexpected_places_are_ignored_silently`, `the_issue_list_never_exceeds_max_issues` |
| (b) follow the next-word of `Exif` IFDs | `a_chain_pointer_inside_a_leaf_ifd_is_not_followed` |
| (c) `SubIFDs` cap `>` to `>=` | `a_subifds_count_at_the_cap_is_accepted` |
| (d) never follow 0x8825 | 9 tests, including `a_corrupt_gps_ifd_leaves_ifd0_and_exif_readable`, `a_gps_pointer_past_eof_with_a_readable_exif_ifd`, `exif_gps_and_interop_are_reached_and_labelled` |
| (e) `Ifd1` mislabelled `Chain` | `an_ifd0_chain_is_labelled_ifd1_then_chain` |

**Fuzz invariant plants (shown red through `tests/fuzz_layout.rs::exercise`, the shared harness body; reverted):**

- Interop children labelled `Gps`: red on the "Exif/Gps parent is not Exif/Gps/Interop" invariant.
- Parent index pointing forward (`holder + 1`): red on "parent index is not earlier"; also run against the **real** `tiff_ifd` fuzz target for 20 s: libFuzzer hit the assertion in `tiff_ifd.rs` ("deadly signal"; the crash artifact is under the ignored `fuzz/artifacts`).
- `ifd_of_kind` returning the last match instead of the first: red on the linear-scan agreement check.
- `Ifd0` successor constant (Ifd0 to Ifd0): red on the "Ifd0 kind iff first IFD" assertion, written as `assert_eq!(kind == Ifd0, i == 0)` so it covers both directions (no one-directional check). The "top-level iff no parent" check is likewise an `assert_eq!` of two booleans.

**Gates (all green on this worktree):** `cargo fmt --all -- --check` (via `just lint`), `just lint` (clippy five-lint set, -D warnings), `just lint-fuzz`, `rustfmt +1.90.0 --check` on `fuzz/fuzz_targets/tiff_ifd.rs`, `cargo test` on 1.90.0 and stable (53 lib tests, 4 + 4 integration), `cargo rustc --lib -- -F unsafe_code`, `just deny`, `just deny-fuzz`, `just validate`, `just decisions-audit`. `just cost-audit` not run by me (spec has no sessions yet; orchestrator). `[dependencies]` still empty; `scripts/` and irradiance untouched.

**`just fuzz-smoke` (60 s, `tiff_ifd`, nightly, 25 seeds):** 833,475 executions in 61 s (about 13.7k exec/s), final `cov: 748 ft: 3287 corp: 480/270Kb`, no crash.

**Seeds:** 5 new (`exif-gps-interop`, `gps-past-eof-exif-ok`, `exif-points-at-ifd0`, `pointer-bad-count`, `subifds-count-over-cap`); regenerating produced no change to the existing 20 (only 5 untracked files appeared).

### Cost self-report

- **Tokens (total):** 185,074 (recorded by the orchestrator from the sub-agent metered result).
- **Estimated USD:** 1.22
- **Duration (minutes):** ~10 (metered wall-clock)
- **Source of the number:** harness report (subagent_tokens)

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:**
  - `follow` takes a `(depth, parent, kind)` tuple instead of three separate arguments, to stay under clippy's argument limit without an `allow`.
  - `the_issue_list_never_exceeds_max_issues` also asserts the exact count `3 * MAX_IFDS` (Interop is ignored outside an Exif IFD, so the fourth malformed tag costs nothing; rule 3). Consequently this shape does not reach `MAX_ISSUES`; the bound holds by construction (DEC-005) and is not exercised at its limit.
  - The fuzz layout test mirrors the harness, including a copy of `ALL_KINDS` and the kind checks.
- **Follow-up work identified:**
  - A test that approaches `MAX_ISSUES` itself would need the 512-IFD budget split across kinds (Exif IFDs with bad Interop plus the other tags). Low value, but a verifier may want it.
  - Only the fuzz harness covers random shapes such as a `Sub` IFD chained under an Exif holder.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   Mostly nothing. Rule 2's "Interop is reached from inside the Exif IFD" together with the permitted-holders table took a moment to resolve into one four-row table walked in fixed order for every holder.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   The agent sandbox refuses a Bash call that combines a heredoc with other commands, which cost retries; a toolchain-brief note ("write scripts to the scratchpad, run them as separate calls") would help cold agents.

3. **If you did this task again, what would you do differently?**
   Put the plant scripts in the repo (for example under the spec's directory) so the verifier can re-run all plants with one command; they live only in the scratchpad now.
