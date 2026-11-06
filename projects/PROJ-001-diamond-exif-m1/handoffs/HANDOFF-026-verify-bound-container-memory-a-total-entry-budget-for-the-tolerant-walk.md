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
  id: HANDOFF-026
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-05
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-017

project:
  id: PROJ-001
  stage: STAGE-003
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
  tokens_total: 223965             # orchestrator-recorded from the metered sub-agent result (subagent_tokens), final figure
  estimated_usd: 1.48
  duration_minutes: 35
  branch: worktree-agent-a22dbec9cda81a459
  pr: null
  completed_at: 2026-11-06         # YYYY-MM-DD
  notes: "verdict APPROVED at e44659a; tokens transcribed by the orchestrator from the sub-agent metered result (final figure after the harness marked the first result possibly interim; duration is wall-clock including idle time)"
  synced_at: 2026-11-06
---

# HANDOFF-026: Verify the entry budget

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-017` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-017-bound-container-memory-a-total-entry-budget-for-the-tolerant-walk.md` (rule, outputs, named tests normative); **DEC-001** incl. its SPEC-017 clarification
- **Build handoff and Handback:** `HANDOFF-025-build-bound-container-memory-...md` (claims to verify, not to trust; read deviations 1-5)
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe)
- Code under review: `git diff 9e37c75..HEAD` (build commit `163feab`); main: `src/tiff/mod.rs` (`Walk`, `follow`, `read_ifd`, `entry_budget`, `entry_count`), `src/error.rs`, `fuzz/shared/entry_budget_invariants.rs`, `tests/budget_oracle.rs`, `tests/support/hostile.rs`

## Expected Deliverables

Review cold; output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

Verify independently:
- Re-run every gate (fmt both graphs, `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `cargo test` on stable and 1.90.0 also with `DIAMOND_EXIF_REQUIRE_ORACLE=1`, `-F unsafe_code`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `oracle-record` with a clean `git status`). Run `just fuzz-smoke tiff_ifd` and `just fuzz-smoke audit` 60 s each and record executions and coverage.
- **Read the walk change line by line:** where the count is read and compared (before any allocation?), `Walk.entries` accounting (IFD0 counted; omitted IFD not counted), atomic omission, `halted` set, nothing recorded after the halt, the arithmetic (`usize::MAX` input, `checked`/`saturating`, no panic-capable construct), `MAX_ISSUES` still a true bound. Could a legitimate file ever trip the budget? Argue it from the format (disjoint 12-byte slots) and try to find a counterexample: shared tables, a TIFF where the same IFD bytes are legitimately referenced from two distinct offsets, DNG/TIFF-EP layouts, JPEG Exif blobs; use ExifTool-read local photos if useful (read-only, commit nothing).
- **Judge deviation 1** (truncation is checked BEFORE the budget, contrary to the spec's implied order): is that the better behaviour? Construct a corrupt-big-count-in-small-file case and show what each order would do. Judge deviations 2-5 (extra records/seeds counts moved; the plant input; the N=8 expectation change; hostile() moved; CHANGELOG).
- **Judge the headroom wording:** the spec said at least 10x headroom on sampled photos; two sampled files sit at 1.4x and 2.7x (tiny Exif blobs). Is the budget still safe by construction there, and what should the spec/DEC text say?
- **Reproduce the headline measurement** (N = 511 hostile file, 789,516 B): entries kept, issue shape, parse time, RSS before (check out the previous commit in a scratch worktree if needed) and after.
- **Test the testers.** Write mutants the builder did not try: `>` vs `>=` at the boundary (they tried one), budget computed from `data.len()` of the wrong slice, IFD0 not counted, omitted IFD counted anyway, `halted` not set, budget check after allocation (memory only: how would a test see it?), `limit` field wrong in the issue, the check inside `follow` skipped for `Link::Next`, entries counted per IFD kind only, `entry_count` returning IFD count, budget `/ 11` or `/ 13`, saturating_add replaced by wrapping add. Report which the suite catches and which survive. Redo at least three fuzz plants including one against the real target on each of `tiff_ifd` and `audit`; confirm the invariants are two-directional where they claim to be.
- Check the 121 existing records and the 66+7 seeds byte-identical; the 4 new records and 3+3 new seeds deterministic and each claiming its shape (`xxd`); no dependencies; `scripts/`, `decisions/` (apart from the DEC-001 clarification), irradiance, the audit module and the JPEG layer untouched; CI workflow unchanged.
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes, SPEC-013/014/015/016 work, ExifTool's 10-SubIFD policy, CI wiring beyond reading it

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

- **Branch / PR:** `worktree-agent-a22dbec9cda81a459` (fast-forwarded to `e44659a` from `claude/rust-exif-metadata-6351d5`; only this handoff file committed; not pushed, no PR)
- **Completed at:** 2026-11-06
- **All acceptance criteria met?** yes. The spec's "at least 10x headroom on the sampled photos" is not a property of the rule and should be reworded (below); the rule itself is safe by construction.
- **For `verify`:** ✅ **APPROVED at `e44659ad4054d147c0132f28cf8ac685c354fedb`** (code under review: build commit `163feab`). Two LOW test-hardening findings and four INFO notes, none blocking.

**Walk change, read line by line (`src/tiff/mod.rs`).**
- `read_ifd(offset, budget_left)`: count word, then table bounds (`Truncated`), then `usize::from(count) > budget_left` (`TooManyEntries`), then `Vec::with_capacity` and decoding. The check precedes every allocation and decode and uses the declared `u16`.
- `follow`: attempts, depth and cycle checks unchanged; `budget.saturating_sub(walk.entries)` passed in; `walk.entries` grows only on `Ok` (an omitted IFD is not counted); on `TooManyEntries`, `walk.halted = true` before the issue is pushed. `walk_from`/`walk_pointer` check `halted` before every pointer, offset and chain step, so nothing is recorded after the halt.
- IFD0 goes through `read_ifd(ifd0_offset, budget)` and seeds `Walk.entries`. With `>` it cannot trip (an in-bounds table has `12c <= len - 10`); with `>=` it can (mutant m01 turned the pre-existing `ifd0_with_its_next_word_cut_off_is_still_fatal` red).
- Arithmetic: `entry_budget` is `checked_div(..).unwrap_or(0)` (no panic for `usize::MAX`, pinned); `saturating_add`/`saturating_sub` throughout; `entry_count` folds with `saturating_add`. No panic-capable construct added in library code.
- `MAX_ISSUES`: `TooManyEntries` is raised inside one of the `MAX_IFDS` budgeted attempts and halts; `TooManyIfds` is raised on attempt 513 and halts; so at most one halting issue, and `5 * MAX_IFDS + 1` still holds (pinned by `max_issues_bound_still_holds_with_the_entry_budget`: 1,534 issues).

**Can a legitimate file trip the budget?** No. Disjoint tables each take `2 + 12c + 4` bytes after an 8-byte header, so `12 x total <= len - 8 - 6k < len`. The same IFD referenced from two pointers is the same offset and is already `CyclicIfd` (pre-existing), not budget. Shared tables at distinct offsets are overlapping by definition, i.e. not a valid TIFF. A JPEG Exif blob is parsed as its own slice, so the budget is the blob's (correct: its tables live inside it). Synthetic legitimate DNG/TIFF-EP-like layouts (IFD0 + two SubIFDs + Exif + GPS + IFD1, all tables dense with no payload at all, the worst case; throw-away test, deleted):

| layout (IFD0, each SubIFD, Exif, GPS, IFD1 entries) | len | budget | entries | headroom | issues |
|---|---|---|---|---|---|
| 3,1,1,1,1 | 148 | 12 | 8 | 1.50x | none |
| 60,30,40,10,8 | 2,188 | 182 | 178 | 1.02x | none |
| 200,100,300,31,20 | 9,064 | 755 | 751 | 1.005x | none |
| 65,535 (IFD0 only) | 786,472 | 65,539 | 65,535 | 1.0001x | none |

Headroom on a legitimate file can approach 1.0x and still never trip. Local photos could not be re-read: the session's permission classifier denied running ExifTool on files under `~/Pictures` (PII), so the builder's photo table is not independently reproduced; the format argument and the table above cover the claim.

**Deviation 1 (truncation before budget): agree, it is the better order.** Probe: 98-byte TIFF, IFD0 with an Exif pointer to a count word `0xFFFF` (table past the end) and a GPS pointer to a good 1-entry IFD (budget 8). As built: IFD0 + GPS IFD kept, one `Truncated { at: 40, len: 786420 }`. Budget-first (swapped in `read_ifd`, then reverted): only IFD0, one walk-ending `TooManyEntries { limit: 8 }`, GPS IFD lost, and three existing DEC-001 tests go red (`a_corrupt_ifd1_keeps_ifd0`, `a_corrupt_gps_ifd_leaves_ifd0_and_exif_readable`, `an_entry_count_past_eof_is_truncated`). Memory is identical either way (an in-bounds table holds at most `len/12` entries). The spec text should be updated to the built order.

**Deviations 2-5: accepted.** (2) The recorder writes one record per `tiff_ifd` seed, so four records and the moved counts (29 to 32, 73 to 76, 41 to 45) are forced; `git diff --name-status 9e37c75..HEAD` shows only additions under `tests/oracle-fixtures/` and `fuzz/seeds/` (no old record or seed modified, independent of the builder's hash list). New seeds checked with `xxd`: `entry-budget-exact` 66 B (counts 2+2, budget 5), `entry-budget-over` 78 B (3+3, budget 6), `overlapping-subifds` 40 x `0x00FF`; the `audit/tiff-*` copies are byte-identical. (3) The N = 8 and `audit-many-overlapping-ifds` expectation changes are the predicted ones; the generated `big_inventory()` (65,537 disjoint entries, clean) is a legitimate replacement input for the audit plant. (4) `hostile()` moved verbatim (one comment shortened). (5) `budget_oracle.rs` placement allowed; index 22 fine; CHANGELOG accurate. The JPEG-layer diff is only the exhaustive display-guard tests (required); `src/audit/`, `scripts/`, `decisions/`, `.github/`, `Cargo.toml`/`Cargo.lock` untouched; no dependencies.

**Headroom wording.** Replace "at least 10x headroom on the sampled photos" with: "A file whose IFD tables are disjoint can never reach the budget (`12 x entries <= len - 8 - 6 x IFDs`); the margin depends on how much non-table data the file holds, from about 1.0x for a table-only blob to 30-90x for camera Exif blobs, so the budget needs no headroom to be safe." DEC-001's clarification ("a real file never reaches it") needs no change.

**Headline measurement, reproduced (N = 511, 789,516 B; release test binary run directly under `/usr/bin/time -l`).**

| | before (`9e37c75`, scratch worktree) | after (`e44659a`) |
|---|---|---|
| IFDs / entries kept | 512 / 33,488,386 | 2 / 65,536 (budget 65,793) |
| issues | 0 | 1, `TooManyEntries { limit: 65793 }`, no `TooManyIfds` |
| `Container::parse` time | 113.9 ms | 198.8 µs |
| RSS after parse | 792,080 KB | 4,496 KB |
| entries x 24 B | 803,721,264 B (1,018x) | 1,572,864 B (1.99x) |
| audit (own parse) | 151 ms, items 65,536, truncated | 712 µs, items 65,536, not truncated, not clean |
| process max RSS / peak footprint | 826,540,032 B / 826,098,864 B | 11,075,584 B / 10,060,136 B |

The builder reported a 1.5 GB "before" process peak; I measure 826 MB for the same test (parse RSS agrees at 792 MB). Order of magnitude and all "after" figures agree.

**Mutants (mine; each applied to `src/tiff/mod.rs`, full `cargo test --no-fail-fast`, reverted).**

| # | mutant | result | caught by |
|---|---|---|---|
| m01 | `>` to `>=` | caught | 4 lib (incl. pre-existing `ifd0_with_its_next_word_cut_off_is_still_fatal`), 3 fuzz_layout |
| m02 | budget from the wrong slice (`len - offset`) | caught | about 40 tests (false halts on legitimate files) |
| m03 | IFD0 not counted | caught | `ifd0_is_counted_in_the_budget` + 4 lib + 3 fuzz_layout |
| m04 | a Truncated IFD's declared count charged anyway | **survived** `cargo test`; caught by the real `tiff_ifd` target in 1,346 execs (`crossing: TooManyEntries but 1 + 2 fits the budget 47`) | fuzz only |
| m05 | `halted` not set | caught | 2 lib, 2 audit_oracle, 2 budget_oracle, fuzz_layout |
| m06 | `limit: budget_left` | caught | 6 lib + oracles |
| m07 | budget skipped for `Link::Next` | caught, thinly | only `max_issues_bound_still_holds_with_the_entry_budget`; real `audit` target crashed in 3,362 execs (`budget: 1535 entries retained, budget 900`) |
| m08 | only SubIFD entries counted | caught, thinly | only `max_issues_bound_still_holds_with_the_entry_budget`; real `tiff_ifd` target crashed (`budget: 44 entries retained, budget 43 for 527 bytes`) |
| m09 | per-IFD budget (no subtraction) | caught | 7 lib + oracles |
| m10 | `entry_count` returns the IFD count | caught | 4 lib, fuzz_layout, audit_oracle |
| m11 | `/ 11` | caught | 7 lib + fuzz_layout |
| m12 | `/ 13` | caught | 4 lib, 2 audit unit, audit_oracle |
| m13 | `saturating_add` to `wrapping_add` | survived | equivalent (`walk.entries <= budget + 65,535` cannot wrap) |
| m14 | budget check after allocation and decoding | survived | equivalent in output; visible only to a counting global allocator or a memory bound on a file whose crossing IFD is large |
| m15 | halt on every `follow` error | caught | 3 pre-existing walk-order tests |

Fuzz plants on the real libFuzzer targets (120 s budget each, all crashed quickly, all reverted, crash artifacts removed): m04 on `tiff_ifd`, m07 on `audit`, m08 on `tiff_ifd`. Two-directional confirmed: m04 is caught by `crossing` (a halt must be genuine), m07/m08 by `budget` (no halt means within budget).

**Gates (all local, on `e44659a`).** fmt: library 0, fuzz manifest 0, `rustfmt --check` on `fuzz/shared/entry_budget_invariants.rs` 0. `just lint` 0; `lint-fuzz` 0; `lint-no-allow` 0; `lint-red-proof` (control 0, injection 101, all five lints, also without `-D warnings`); `lint-fuzz-red-proof` (control 0, `unnecessary_cast` 101 at `tiff_ifd.rs:196`, 0 without `-D warnings`). `DIAMOND_EXIF_REQUIRE_ORACLE=1 cargo test` on Homebrew stable 1.99 and on `~/.cargo/bin/cargo +1.90.0`: all green, no SKIP (lib 180, audit_oracle 17, budget_oracle 19, fuzz_layout 23, jpeg_oracle 42, metadata_oracle 30, mutation_oracle 51). `forbid-unsafe` 0; `deny`/`deny-fuzz` licenses ok; `validate` ok; `decisions-audit` 0 structural errors (same 10 scope warnings; `--changed` lists DEC-002/003/005 for `src/tiff/mod.rs`, no drift); `cost-audit` ok. `just oracle-record` then `git status`: clean (125 records = 121 + 4). `cargo run --example fuzz_seeds`: clean, deterministic (32/37/76). Oracle shown red: the hostile record's warning edited to "Ignoring 500" turns the replay and live tests red; reverted.

**Fuzz smoke (60 s, `just fuzz-smoke`).** `tiff_ifd`: 1,683,714 executions, cov 1145, ft 5057, corpus 663 (27,601 exec/s), clean. `audit`: 275,097 executions, cov 1723, ft 6974, corpus 818 (4,509 exec/s), clean. (Final rss 557 MB / 739 MB is libFuzzer + ASan over the growing corpus, under the 2 GB limit.)

**Findings.**
1. LOW, test gap: no deterministic test shows that a `Truncated` IFD spends no budget (m04 survives `cargo test`; `the_over_budget_ifd_allocates_no_entries` says "the budget is intact" but does not assert it). Suggested: a truncated big-count sub-IFD followed by a sibling that exactly fits, expecting one `Truncated` and the sibling kept. Fuzz finds it in seconds, so not blocking.
2. LOW, test gap: budget accounting on non-SubIFD links (chain `Link::Next`, Exif/GPS/Interop) is pinned only by one incidental test (m07, m08); every named budget test uses SubIFD shapes. Suggested: one chain-crossing and one Exif-crossing case.
3. INFO: m13 and m14 are equivalent mutants; "before allocation" is guaranteed by code reading and the direct `read_ifd` call, not by an observable test.
4. INFO: spec wording (the 10x headroom criterion; the implied budget-before-truncation order) should be updated to match the build.
5. INFO: "before" process peak differs from the builder's (826 MB vs 1.5 GB); parse RSS agrees.
6. INFO: real-photo re-measurement not possible in this session (permission denied for files under `~/Pictures`).

### Cost self-report

- **Tokens (total):** null. The verifier cannot see its own token meter; the orchestrator transcribes the metered sub-agent figure.
- **Estimated USD:** null
- **Duration (minutes):** about 75 (wall clock, mostly the 15 full-suite mutant runs and 5 fuzz runs)
- **Source of the number:** none available to the verifier (harness `subagent_tokens`, via the orchestrator)

### Drift and new artifacts

- **New decisions emitted:** none
- **Deviations from spec:** none by the verifier. Build deviations 1-5 judged acceptable (deviation 1 preferred over the spec's order).
- **Follow-up work identified:**
  - Two small test additions (findings 1 and 2), for the next spec that touches `src/tiff`.
  - Spec text: reword the headroom criterion and the check order.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Nothing in the content. The cost was tooling: the full suite takes over a minute per mutant (`mutation_oracle` rebuilds probe crates), so 15 mutants took over 20 minutes. The handoff's suggestion to read local photos ran into the permission classifier.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — The interaction between the check order and the "merely damaged files do not change" goal belonged in the spec; the builder had to resolve a contradiction created at design. Also, the isolated shell rejects `/usr/bin/time cargo ...` and loops over files, so measurements have to run the built test binary directly.

3. **If you did this task again, what would you do differently?**
   — Run the mutants against a targeted subset (lib + budget/audit/fuzz_layout tests) first and confirm survivors on the full suite, and start the "before" worktree build at the outset so measurements overlap the mutant runs.
