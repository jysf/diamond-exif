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
  id: HANDOFF-011
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-05
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-007

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
  tokens_total: 172264             # orchestrator-recorded from the metered sub-agent result (subagent_tokens)
  estimated_usd: 1.14
  duration_minutes: 10
  branch: worktree-agent-a0900d4a4400af55c
  pr: null
  completed_at: 2026-11-05         # YYYY-MM-DD
  notes: "tokens transcribed by the orchestrator from the sub-agent metered result"
  synced_at: 2026-11-05
---

# HANDOFF-011: Decode ASCII, signed and float tag values

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-007` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

Read before starting:

- **Spec (semantics and named tests are normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-007-decode-ascii-signed-and-float-tag-values.md`
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `~/.cargo/bin/cargo +nightly`; `just lint` runs under the default PATH)
- DEC-002 (borrow from the input), DEC-003, DEC-004
- `src/tiff/types.rs` (the pattern to follow: `uints`, `rational_chunks`), `src/reader.rs`, `src/tiff/mod.rs` (`payload`, tests, `Build`), `src/tiff/tags.rs`, `src/error.rs`, `fuzz/fuzz_targets/tiff_ifd.rs`, `tests/fuzz_layout.rs`, `examples/fuzz_seeds.rs`
- Prior art: HANDOFF-009 (SPEC-004 build, including its punch-list follow-up round) and HANDOFF-010 (verify findings worth avoiding repeating)

Constraints: `panic-free-library` (five-lint deny set in non-test code), `forbid-unsafe-code`, `fuzz-target-per-surface`, `test-before-implementation`, `no-new-top-level-deps-without-decision` (no dependencies).

## Expected Deliverables

- Everything in the spec's Outputs with all its Failing Tests present and passing, and every Acceptance Criterion met, including the six recorded plants (each named test goes red, then revert), at least two fuzz invariants shown red with planted bugs (one against the real target), `just fuzz-smoke` clean with executions and coverage recorded, and `rustfmt --check` on the fuzz crate.
- **Write every fuzz iff-invariant in both directions.** For each plant you run, verify that the named test fails; also try each single-guard variant where two guards overlap.
- Name any deviation from the spec in the Handback, including tests you folded together or renamed. Last time a spec-named test went missing unreported.
- Commit on your worktree branch. **Do not push and do not open a PR.** The spec is already at `cycle: build`; do not advance cycles. First run `git merge --ff-only claude/rust-exif-metadata-6351d5` if your worktree was cut from an older commit (you need `9ec2861` or later).
- Do not modify irradiance, `scripts/`, decisions, or anything outside the spec's Outputs. The 26 existing fuzz seeds must remain byte-identical.
- You cannot see your own token count; set `tokens_total: null` with a note, the orchestrator transcribes the metered figure.

## Out of Scope

- Tag interpretation, `parse_fields`, oracle work (SPEC-005); walk changes; rational changes
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

- **Branch / PR:** `worktree-agent-a0900d4a4400af55c`, committed locally; no push, no PR (by instruction).
- **Completed at:** 2026-11-05
- **All acceptance criteria met?** yes (see Deviations for the points to look at). Gates run here: `just lint`, `just lint-fuzz`, `rustfmt --check` on the fuzz target, `cargo test` on 1.90.0 and Homebrew stable (all 4 test binaries green, 73 lib tests), `cargo rustc --lib -- -F unsafe_code`, `just deny`, `just deny-fuzz`, `just validate`, `just decisions-audit`, `just cost-audit`.
- **For `verify`:** n/a

**Six recorded plants (library tests, each reverted):**

| Plant | Red test(s) |
|---|---|
| (a) `ByteOrder::f32`/`f64` Big arms read little-endian | `floats_read_verbatim_in_both_orders` |
| (b) `SSHORT` zero-extended (`as u16`) | `sints_sign_extend_every_width_in_both_orders` |
| (c) NUL included in `as_bytes` | `ascii_reads_a_terminated_external_string`, `ascii_reads_inline_values`, `ascii_splits_at_the_first_nul`, `ascii_non_utf8_is_exposed_not_rejected`, `ascii_borrows_from_the_input_not_the_container` |
| (d) `ascii` also accepts `UNDEFINED` | `ascii_refuses_other_types_before_touching_the_buffer` |
| (e) `rest` off by one (`tail.get(2..)`) | `ascii_splits_at_the_first_nul`, `ascii_borrows_from_the_input_not_the_container` |
| (f) `uints` accepts `SLONG` (gate and decode arm) | `sints_refuse_unsigned_and_unsigned_readers_refuse_signed` |

**Fuzz plants (five, each red in both `tests/fuzz_layout.rs` and the real `tiff_ifd` target run over the seeds with `cargo +nightly fuzz run ... -runs=0`, then reverted):**

- F1 `rest` off by one: `assertion failed: ascii partition`
- F2 `sints` drops the first `SLONG` value: `sints length`
- F3 `ascii` also accepts `BYTE` (accept side of the gate iff): `ascii type gate, type 1`
- F4 `ascii` rejects every type (reject side, constant-false): `ascii type gate, type 2`
- F5 `terminated` always true for unterminated payloads: `ascii partition`

F3 and F4 together show the type-gate iff is checked in both directions. I did not plant further single-guard variants beyond (f), F3 and F4.

**`just fuzz-smoke` (60 s, clean, no crash):** run 1 from the 29 seeds, 1,385,004 executions in 61 s; run 2 (same corpus dir, so it started from run 1's corpus) 656,448 executions, final `cov: 983 ft: 4914 corp: 627/404Kb`, about 10.8k exec/s. Baseline over the seeds only (`-runs=0`): 30 runs, `cov: 822 ft: 1799`.

### Cost self-report

- **Tokens (total):** 172,264 (recorded by the orchestrator from the sub-agent metered result).
- **Estimated USD:** 1.14
- **Duration (minutes):** ~10 (metered wall-clock)
- **Source of the number:** harness report (subagent_tokens)

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:**
  - No spec-named test is renamed, folded or missing: all 19 listed tests exist under their spec names. Several are larger than the spec's minimum (extra inline/external and BE/LE cases).
  - Tests were written in the same pass as the code and then shown red by the plants, not committed red first (`test-before-implementation` was satisfied by the plants, not by a red commit). Committed in two steps (decoders + unit tests, then fuzz + seeds + docs).
  - `Ascii::split`: for an unterminated value `rest` is the empty tail of the payload (`payload.get(len..)`), not a static `&[]`, so the zero-copy pointer check in the fuzz target holds for empty slices too. The fuzz target additionally asserts that `text` starts at the payload start, that `rest` starts right after the NUL, and that `ascii` is `Ok` iff (type 2 and count within cap and payload resolves); all beyond the spec's list.
  - Shared prelude is `Container::gated_payload` (private, in `types.rs`); the existing `uints` and rational readers are untouched.
  - The new `TYPE_*` constants are re-exported at `exif_core::tiff` as well as `tiff::types`.
  - The existing 26 seeds are byte-identical (git shows no change to them); there are now 29. `SEEDS` in `tests/fuzz_layout.rs` already listed only 21 of the 26 (the `-be` variants and `next-past-eof` were never named); I added the three new names (24 now) and left the rest as found.
- **Follow-up work identified:** optional: make `SEEDS` in `tests/fuzz_layout.rs` cover every committed seed (24 of 29 listed).

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   - Little. The "Ok iff" fuzz rules needed one decision: `payload()` itself returns `UnexpectedFieldType` for unknown types, so the gate iff is only clean because every accepted type is a known one; worth a line in the spec. Also, the zero-copy check on an empty `rest` is impossible with a static `&[]`.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   - No.

3. **If you did this task again, what would you do differently?**
   - Write the tests and the plant scripts first and commit them red, as the constraint literally asks, instead of writing them alongside the code and proving red afterwards.
