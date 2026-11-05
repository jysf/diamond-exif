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
  id: HANDOFF-010
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
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
  tokens_total: 181124             # orchestrator-recorded from the metered sub-agent result (subagent_tokens)
  estimated_usd: 1.20
  duration_minutes: 9
  branch: worktree-agent-aa55bda855552a680
  pr: null
  completed_at: 2026-11-05         # YYYY-MM-DD
  notes: "verdict PUNCH LIST at a68757d (code 06f1702); tokens transcribed by the orchestrator from the sub-agent metered result"
  synced_at: 2026-11-05
---

# HANDOFF-010: Verify the multi-IFD pointer walk

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-004` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-004-multi-ifd-pointer-walk.md` (walk rules and named tests are normative)
- **`decisions/DEC-005-pointer-tag-walk-policy.md`** (normative table), DEC-001 (with clarifications), DEC-002, DEC-004
- **Build handoff and its Handback:** `HANDOFF-009-build-multi-ifd-pointer-walk.md` (claims to verify, not to trust)
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`)
- Code under review: `git diff bad5fcb..HEAD` (build commit `06f1702`)

## Expected Deliverables

The verify contract: review cold, output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

Verify independently:
- Re-run every gate in the spec's acceptance criteria.
- Read the walk against the rules and DEC-005 line by line: permitted-holder table, kinds (Ifd0/Ifd1/Chain/Sub), leaf IFDs' next-word not followed, value rule (type LONG/IFD, count exactly 1, count 0 never reads garbage), the SubIFDs cap checked **before** any payload read, depth-first order, attempt accounting (IFD0 = attempt 1), `from_ifd`/`offset` per issue, duplicate-tag handling, `MAX_ISSUES` bound. Look for any panic-capable construct in non-test code and any unbounded loop or allocation; try a few adversarial shapes yourself (pointer to itself via each tag, huge counts, Interop inside Gps, SubIFD whose chain holds Exif pointers).
- Redo at least the five walk plants (a)-(e) and confirm the named tests go red, then revert; redo at least two fuzz-invariant plants, including one against the real fuzz target, then revert. Leave the tree clean.
- The builder noted `the_issue_list_never_exceeds_max_issues` does not reach `MAX_ISSUES` (Interop is ignored outside Exif). Judge whether the bound is adequately tested; try to construct a worst-case file and report the largest issue count you can reach versus `MAX_ISSUES`.
- Check the flipped/updated SPEC-003 tests still mean what they did; check no test is vacuous and that every iff-invariant in the fuzz target is both-directional. Confirm the 20 original seeds are byte-identical and the seed writer is deterministic.
- Run `just fuzz-smoke` (60 s), record executions and coverage. Run `just decisions-audit --changed bad5fcb`.
- Judge the builder's deviations (tuple argument to `follow`, extra exact-count assert, duplicated `ALL_KINDS` in `tests/fuzz_layout.rs`).
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes, SPEC-007 or SPEC-005 work, CI wiring, the 24 h fuzz run

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

- **Branch / PR:** `worktree-agent-aa55bda855552a680` (fast-forwarded to `a68757d`; this handback is the only commit on top). No push, no PR.
- **Completed at:** 2026-11-05
- **All acceptance criteria met?** All but one: the code works and every gate is green. Walk rule 7 ("the budget halts the whole walk", DEC-001) is not fully met. See finding 1.
- **For `verify`:** ⚠ **PUNCH LIST** at `a68757d` (build commit `06f1702`). The work is close: one medium item and three low items to fix before approval.

**Gates (re-run independently on `a68757d`, all green):** `cargo +1.90.0 fmt --all -- --check`; `just lint` (Homebrew clippy 0.1.99) and the same clippy run under `RUSTUP_TOOLCHAIN=1.90.0` with `~/.cargo/bin` first; `just lint-fuzz`; `rustfmt +1.90.0 --check` on `fuzz/fuzz_targets/tiff_ifd.rs`; `cargo test` on 1.90.0 and stable (53 lib, 4 + 4 integration, 0 doc); `cargo +1.90.0 rustc --lib -- -F unsafe_code`; `just deny` and `just deny-fuzz` (licenses ok); `just validate`; `just decisions-audit` (5 clean); `just cost-audit` (green). `[dependencies]` is still empty. `scripts/`, `Cargo.toml` and `Cargo.lock` are unchanged by the build.

**`just decisions-audit --changed bad5fcb`:** reports "No active decision's affected_scope matches these changes". That result is **wrong**. See finding 2: the gate cannot read the flow-style `affected_scope` lists, so it says this about every change.

**Fuzz smoke (`just fuzz-smoke`, 60 s, clean tree, 25 seeds):** 930,844 executions in 61 s (about 15.3k exec/s). Final `cov: 748 ft: 3263 corp: 454/184Kb`. No crash, exit 0. The builder reported 833,475 executions and cov 748, which is consistent.

**Seeds:** 25 in total. `git diff bad5fcb..HEAD -- fuzz/seeds` shows only the 5 additions, so the 20 original seeds are byte-identical. Running `cargo run --example fuzz_seeds` again gave identical SHA-1s for all 25 files and a clean tree, so the writer is deterministic.

**Walk plants (perl edit of `src/tiff/mod.rs`, `cargo +1.90.0 test --lib`, then `git checkout`). All went red and all are reverted:**

| Plant | Red tests |
|---|---|
| (a) `0xA005` allowed from any holder | `pointer_tags_in_unexpected_places_are_ignored_silently`, `the_issue_list_never_exceeds_max_issues` |
| (b) Exif next-word followed (`Exif => Some(Exif)`) | `a_chain_pointer_inside_a_leaf_ifd_is_not_followed` |
| (c) `SubIFDs` cap `>` to `>=` | `a_subifds_count_at_the_cap_is_accepted` |
| (d) 0x8825 never followed (table tag set to 0xFFFF) | 9 tests, among them `a_corrupt_gps_ifd_leaves_ifd0_and_exif_readable`, `a_gps_pointer_past_eof_with_a_readable_exif_ifd`, `exif_gps_and_interop_are_reached_and_labelled`, `issues_arrive_in_walk_order_across_pointer_kinds` |
| (e) `Ifd0` successor `Chain` | `an_ifd0_chain_is_labelled_ifd1_then_chain` |

These match the builder's table exactly. I also ran these extra plants:

| Plant | Result |
|---|---|
| (f) type check on single pointers removed | red: `a_pointer_with_the_wrong_type_costs_the_branch_not_the_ifd`, `the_issue_list_never_exceeds_max_issues` |
| (g) count check removed | red: `a_pointer_with_the_wrong_count_is_typed` |
| (h) last duplicate entry wins | red: `duplicate_pointer_tags_use_the_first_entry` |
| (i) pointer children not one level deeper | red: 4 tests (depth 8/9, labelled, subifd order) |
| (k) Exif/GPS also allowed from `Gps` | red: `pointer_tags_in_unexpected_places_are_ignored_silently` |
| (l) zero pointer value followed | red: `a_zero_pointer_value_is_skipped_silently` |
| (j) Interop also allowed from `Gps` | **survives the whole `cargo test` suite** (finding 3). The real fuzz target catches it (below). |

**Fuzz-invariant plants. Both went red and both are reverted:**
- Against the **real** `tiff_ifd` target (`cargo +nightly fuzz run`, 60 s budget, scratch corpus and artifacts outside the repo): with plant (j), libFuzzer hit `tiff_ifd.rs:124` (the "Interop parent is Exif" assertion) within the run and reported a deadly signal.
- Against the harness through `tests/fuzz_layout.rs::exercise`: with the parent index planted forward (`holder + 5`), `every_seed_parses_or_errors_without_panicking` went red on "parent index is not earlier".
- A third plant (a `Gps` next-word followed as `Chain`) passed `fuzz_layout`, because no seed has a GPS IFD with a non-zero next-word. That is a gap in seed coverage, not a weak invariant: "top-level iff no parent" would catch it once the shape occurs.

**Adversarial probes (a temporary `tests/zz_verify_probe.rs`, deleted afterwards):**
- An IFD pointing at itself through each tag (Interop and SubIFDs inside Exif, SubIFDs inside GPS, Exif/GPS/next inside Sub) gives `CyclicIfd` with the right `Link::Tag`/`Next`, `from_ifd` and offset. No loops.
- Count `u32::MAX`: `SubIFDs` gives `TagTooLarge` (no decode). Exif `LONG` and GPS `IFD` give `BadPointerCount`. A pointer of type `IFD` (13) with count 1 is followed.
- Interop inside GPS is ignored.
- A Sub chain whose IFDs hold Exif pointers: each Exif IFD gets the right chain member as parent.
- Exif → Sub → Exif → Interop nesting is reachable (depths 2..5). This is permitted by the DEC-005 table (see finding 5).
- **Worst-case issue count: 1,537 = 3·MAX_IFDS + 1, against MAX_ISSUES = 2,561.** Shape: a 513-IFD top-level chain, each IFD with malformed `SubIFDs`, Exif and GPS (3 issues each × 512 recorded), plus `TooManyIfds`. Derivation: no kind may hold all four pointers (the max is 3, for top-level/Sub), so issues ≤ 3R + (MAX_IFDS − R) + 1 ≤ 1,537. The constant is sound but loose, and 1,537 is the actual ceiling. A mixed Exif-heavy shape reached 1,025.

**Code read against DEC-005 and the rules:** I found nothing wrong here.
- The one-row-per-tag table and the fixed walk order (SubIFDs, Exif, GPS, Interop) are correct.
- Leaf next-words are stored but not followed.
- Type is checked before count, and both before the payload, so count 0 never reads anything.
- The `SubIFDs` cap is applied to the declared count before `uints`.
- IFD0 counts as attempt 1. Depth is the holder's depth + 1. Duplicates go through `Ifd::entry` (first wins).
- `from_ifd` is the holder. `offset` is `None` for value faults and `Some` for IFD faults.
- No panic-capable constructs remain in non-test code: the five-lint clippy run is clean, and only `saturating_*`, `checked_*` and `get` are used.
- Allocation is bounded: ≤ 512 offsets per `SubIFDs` and at most `MAX_TAG_VALUES` elsewhere.
- Recursion depth is bounded by `MAX_IFD_DEPTH`, because `follow` checks depth before it returns an index to recurse into.

**Flipped or updated SPEC-003 tests:** `a_hostile_subifds_array_cannot_flood_the_issue_list` now asserts the single `TagTooLarge`, which keeps its intent (no flood). Its old 511+1 derivation moved into `a_subifds_count_at_the_cap_is_accepted` (512 offsets → 511 `CyclicIfd` + `TooManyIfds`), so the SPEC-003 budget arithmetic is still tested. The depth 8/9 tests and the 512/513-IFD chain tests are unchanged. No test I read is vacuous: each was shown red by at least one plant, except where finding 3 notes a gap.

**Fuzz invariants, both directions:** "`Ifd0` iff index 0" and "top-level iff no parent" are both-direction `assert_eq!`s of two booleans. The parent/child kind rules are checked both ways: the child kind implies the allowed parent kinds, and a leaf parent implies the allowed child kinds. `ifd_of_kind` is checked against a linear scan in both directions (Some/Some by pointer identity, None/None, anything else panics).

**Builder's deviations:**
- The tuple argument to `follow`: acceptable. It avoids an `allow`.
- The extra exact-count assert (`3 * MAX_IFDS`): good, it makes the test non-vacuous (plants (a) and (f) were caught by it).
- The duplicated `ALL_KINDS` in `tests/fuzz_layout.rs`: acceptable, since the fuzz crate is a separate graph. There is a drift risk, though. `IfdKind` is `#[non_exhaustive]`, so neither copy fails to compile when a kind is added (low, finding 6).

**Findings:**

1. **MEDIUM: the walk records issues after the budget halts it.** In `walk_from`, after the recursive SubIFDs branch sets `walk.halted`, the `for spec in &POINTERS` loop keeps calling `walk_pointer` for the holder's remaining tags. `walk_pointer` checks `halted` only inside its offset loop, after `pointer_offsets` has already validated or decoded the entry and pushed any value-fault issue. Every ancestor on the recursion stack does the same.

   Reproduced with IFD0 {SubIFDs → a 600-IFD Sub chain, Exif as `SHORT`, GPS `LONG` count 2}. The issues come out as `[TooManyIfds, UnexpectedFieldType{0x8769}, BadPointerCount{0x8825}]`.

   This contradicts walk rule 7 and DEC-001 ("`halted` ends the whole walk … halting is what bounds … the number of issues"). It also breaks the SPEC-003 property that `TooManyIfds` is the last issue. The harm is bounded: it stays inside 3 issues per recorded IFD, the extra work is bounded by recursion depth, and `parse_strict` is unaffected. Before SPEC-004 the case could not arise, because SubIFDs was the only pointer tag.

   Fix: return early from `walk_pointer` (or from the spec loop) when `walk.halted` is set. Add a test that asserts `TooManyIfds` is last in this shape, and optionally a fuzz invariant ("no issue follows `TooManyIfds`").

2. **MEDIUM (process, pre-existing, not in SPEC-004's code): `just decisions-audit --changed` is a mute gate.** `get_affected_scope` in `scripts/decisions-audit.sh` parses only block-style YAML lists (`- item`). Every DEC in this repo uses flow style (`affected_scope: ["src/tiff/**", "src/error.rs"]`), so it extracts zero globs. As a result it reports "No active decision's affected_scope matches" for changes to `src/tiff/mod.rs` that DEC-001, DEC-002, DEC-003 and DEC-005 all govern. The build handback's "decisions-audit green" therefore carries no information for `--changed`. I did not touch `scripts/` (out of bounds). It needs a template/irradiance fix, or the DECs need converting to block lists. Recommend a `process-debt` signal.

3. **LOW: unit-test gap for Interop holders.** Plant (j), which lets a `Gps` IFD follow 0xA005, passes all 61 tests. `pointer_tags_in_unexpected_places_are_ignored_silently` places 0xA005 only in IFD0, and no unit test puts it in Gps, Interop or Sub. Only the fuzz invariant catches this. Add 0xA005 inside the GPS IFD (and ideally inside a Sub IFD) to that test.

4. **LOW: a named test is missing.** The spec names `parse_issue_display_for_new_pointer_tags`. It is not present. Its assertion (`Link::Tag(0x8825)` displays `tag 0x8825`) was folded as one line into `parse_issue_display_names_error_and_location`. The assertion exists, but the deviation is not reported in HANDOFF-009. Either add the named test or record the deviation.

5. **LOW / informational: MAX_ISSUES is loose and the bound test does not reach it.** The documented rationale ("at most four pointer-tag issues per recorded IFD") is true but not tight: no kind can hold four, and the real ceiling is 3·MAX_IFDS + 1 = 1,537 against 2,561. `the_issue_list_never_exceeds_max_issues` stops at 1,536 because its chain ends without a next-word. Suggest adding a next-word to the last IFD (giving 1,537 + `TooManyIfds` last, which also covers finding 1). Optionally, tighten the constant or its doc in a later spec; the public value can stay as a guarantee. Separately, Exif → Sub → Exif nesting is reachable via `SubIFDs`. The DEC-005 table permits it, though the DEC's rationale rejects "Exif → Exif → Exif nesting". It is worth one sentence in DEC-005.

6. **LOW / nit:**
   - `ALL_KINDS` is duplicated in two places, and neither copy is compile-time guarded against a new `IfdKind`. An in-crate exhaustive-match test returning the count would guard it.
   - Stale docs: `Ifd::depth` ("`SubIFD` nesting depth; IFD0 and its chain are depth 0") and `MAX_IFD_DEPTH` ("Maximum `SubIFD` nesting depth") predate pointer-tag depth.
   - No seed exercises a non-zero next-word on a leaf IFD.

### Cost self-report

- **Tokens (total):** 181,124 (recorded by the orchestrator from the sub-agent metered result).
- **Estimated USD:** 1.20
- **Duration (minutes):** ~9 (metered wall-clock)
- **Source of the number:** harness report (subagent_tokens)

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:** none by the verifier. Builder deviations are judged above, and finding 4 is an unreported one.
- **Follow-up work identified:**
  - Punch list for SPEC-004: finding 1 (halt early, plus a test), finding 3 (Interop-in-GPS test), finding 4 (named test or recorded deviation). Optionally the next-word in the bound test (finding 5).
  - Process debt: fix `decisions-audit.sh` so it reads flow-style `affected_scope` (finding 2), in the template/irradiance, not here.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Whether rule 7's "halts the whole walk" covers recording pointer-value faults after the halt. I read it, with DEC-001's clarification, as "no more issues after `TooManyIfds`". If the architect disagrees, finding 1 becomes a doc note instead.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — The verifier sandbox refuses compound Bash commands that use variables or mix git with other commands. Plant scripts in the scratchpad, run as one plain `bash <file>`, work. That note belongs in the toolchain brief for cold agents.

3. **If you did this task again, what would you do differently?**
   — I would start with "what can happen after a guard trips" probes (post-halt, post-depth) rather than plants. The plants all confirmed the builder's table. The real defect turned up through a probe.
