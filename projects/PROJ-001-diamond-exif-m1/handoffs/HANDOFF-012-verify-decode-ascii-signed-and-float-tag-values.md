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
  id: HANDOFF-012
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
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
  tokens_total: 175106             # orchestrator-recorded from the metered sub-agent result (subagent_tokens)
  estimated_usd: 1.16
  duration_minutes: 8
  branch: worktree-agent-acdcbe3df5fa44625
  pr: null
  completed_at: 2026-11-05         # YYYY-MM-DD
  notes: "verdict APPROVED at ffb96a5; tokens transcribed by the orchestrator from the sub-agent metered result"
  synced_at: 2026-11-05
---

# HANDOFF-012: Verify the ASCII, signed and float decoders

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-007` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-007-decode-ascii-signed-and-float-tag-values.md` (semantics and named tests are normative)
- **Build handoff and its Handback:** `HANDOFF-011-build-decode-ascii-signed-and-float-tag-values.md` (claims to verify, not to trust)
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`)
- DEC-002, DEC-003, DEC-004
- Code under review: `git diff 1cf9976..HEAD` on the current branch (build commits `9c5c301`, `9c84808`; handback `8e9b9a0`)

## Expected Deliverables

Review cold; output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

Verify independently:
- Re-run every gate in the spec's acceptance criteria.
- Read `ascii`, `sints`, `floats` and the shared prelude `gated_payload` against the spec semantics: type gate before the buffer, count cap before the payload, ASCII partition invariant, sign extension per width, f32 widening, byte order for every width. Hunt for panic-capable constructs in non-test code, unbounded allocation (u32::MAX counts), `Vec::with_capacity` on an unvalidated count, and any place the three readers can drift. Confirm `uints` and the rational readers behave as before (the prelude refactor must not have changed their order of checks).
- Try adversarial shapes yourself: ASCII count exactly `MAX_TAG_VALUES` and +1; inline ASCII count 1-4 with NULs in every position; DOUBLE with count 1 (always external); FLOAT count 1 (inline); count 0 for each reader; a type-2 entry whose offset is `u32::MAX`; each reader against every field type 1..=13 and unknown types (compare with the spec's accepted sets).
- Redo the six spec plants (a)-(f) and confirm the named tests go red; redo at least two fuzz plants, one against the real fuzz target; try two plants of your own that the builder did not (e.g. `floats` accepting RATIONAL; `Ascii::is_terminated` computed from the wrong slice; `sints` for SBYTE reading two bytes). Revert everything; leave the tree clean.
- Check the fuzz iff-invariants are both-directional and the layout test `exercise` mirrors the target. Note: `SEEDS` in `tests/fuzz_layout.rs` names only 24 of the 29 committed seeds (pre-existing for 21 of 26); judge whether that is worth a punch-list item.
- Confirm the 26 existing seeds are byte-identical and the seed writer is deterministic. Run `just fuzz-smoke` (60 s) and record executions and coverage. Run `just decisions-audit --changed 1cf9976`.
- Judge the builder's deviations (`Ascii::split` empty-tail choice, extra fuzz assertions, re-exports, no extra single-guard plants).
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes, SPEC-005 work, tag interpretation, CI wiring, the 24 h fuzz run

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

- **Branch / PR:** `worktree-agent-acdcbe3df5fa44625` (fast-forwarded to `ffb96a5`, handback committed on top); no push, no PR, no cycle advanced.
- **Completed at:** 2026-11-05
- **All acceptance criteria met?** yes
- **For `verify`:** ✅ **APPROVED at `ffb96a5`** (code under review `1cf9976..ffb96a5`: build commits `9c5c301`, `9c84808`). No blocking or medium findings; five low/nit items below, none required before ship.

**Gates, re-run independently**

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `just lint` (Homebrew clippy 0.1.99) and clippy 0.1.90 `--all-targets -D warnings` | clean (five-lint deny set in non-test code) |
| `just lint-fuzz`; `rustfmt --edition 2021 --check fuzz/fuzz_targets/tiff_ifd.rs` | clean |
| `cargo test`, 1.90.0 and stable 1.99.1 | 73 lib + 4 + 4 + 0, all green on both toolchains |
| `just forbid-unsafe` (`-F unsafe_code`) | clean |
| `just deny`, `just deny-fuzz` | licenses ok, both |
| `just validate`, `just decisions-audit`, `just cost-audit` | green (decisions-audit: 0 structural errors, 4 pre-existing nested-scope warnings) |
| `just decisions-audit --changed 1cf9976` | advisory: DEC-001..005 touched; re-read, change is consistent (DEC-002 borrow-from-input holds, DEC-003 ledger note present, DEC-004 layout unchanged) |
| `[dependencies]` empty; `scripts/` and irradiance untouched; the 26 old seeds byte-identical (diff `7c7bb0b..HEAD -- fuzz/seeds/` shows only the 3 new files) | yes |
| Seed writer deterministic: `cargo run --example fuzz_seeds` twice, `shasum` identical, tree clean (regenerated = committed) | yes |

**Code read** (`ascii`, `sints`, `floats`, `gated_payload`, `ByteOrder::{i16,f32,f64}`): order is type gate (pure entry property), then `usize::try_from` + `MAX_TAG_VALUES` cap, then `payload()`; `Vec::with_capacity(count)` only after the cap (max 1 Mi elements). No indexing/unwrap/arithmetic in non-test code (`split_at_checked`, `get`, `as_chunks`). Sign extension via `i8::from_ne_bytes` / `ByteOrder::i16` / `i32`; FLOAT widened with `f64::from`; byte order respected for every multi-byte width. `uints`, `urationals`, `srationals` and `rational_chunks` are unchanged by the diff (only new constants nearby), so their check order is as before; HANDOFF-011's "existing readers untouched" holds.

**Adversarial probes** (temporary `tests/zz_verify_probe.rs`, deleted; all passed):
- ASCII count exactly `MAX_TAG_VALUES` with a real 1 MiB payload: `Ok`, terminated; `+1`: `TagTooLarge`. SBYTE count = cap: `Ok`, 1,048,576 values.
- Inline ASCII count 0..=4 with NUL in every subset of positions (31 shapes, both orders): text/terminated/rest match a reference split; text pointer is the entry's value field.
- DOUBLE count 1 is external (subnormal read bit-exact, both orders); DOUBLE count 1 with offset `0xFFFFFFFF`: `Truncated`. FLOAT count 1 inline; SSHORT count 2 and SBYTE count 4 inline, both orders.
- Count 0 for ascii/sints/floats: empty values, no panic.
- Type-2 entry with offset `u32::MAX`: `Truncated`; count `u32::MAX`: `TagTooLarge`.
- Every reader (ascii, sints, floats, uints, urationals, srationals) against types 0..=20, 0x7FFF, 0xFFFF with an out-of-bounds payload: `UnexpectedFieldType` exactly outside the spec's accepted sets; accepted new types report `Truncated` (buffer reached only after the gate).

**Spec plants (a)-(f), redone, each reverted** (red tests from `cargo test --lib`):

| Plant | Red |
|---|---|
| (a) `f32`+`f64` Big arms read LE; also each alone (a1 f64 only, a2 f32 only) | `floats_read_verbatim_in_both_orders` (all three variants) |
| (b) SSHORT zero-extended (`u16`) | `sints_sign_extend_every_width_in_both_orders` |
| (c) NUL included in `as_bytes` (split at nul+1, rest kept correct) | 5: `ascii_reads_a_terminated_external_string`, `ascii_reads_inline_values`, `ascii_splits_at_the_first_nul`, `ascii_non_utf8_is_exposed_not_rejected`, `ascii_borrows_from_the_input_not_the_container` |
| (d) `ascii` accepts UNDEFINED | `ascii_refuses_other_types_before_touching_the_buffer` |
| (e) `rest` off by one (`get(2..)`) | `ascii_splits_at_the_first_nul`, `ascii_borrows_from_the_input_not_the_container` |
| (f) `uints` accepts SLONG (gate + decode arm); f1 gate only | `sints_refuse_unsigned_and_unsigned_readers_refuse_signed` (both) |

**Own plants (not run by the builder), each reverted:**

| Plant | Red |
|---|---|
| (g) `floats` accepts RATIONAL | `floats_refuse_other_types` |
| (h) `is_terminated` computed from the wrong slice (`!rest.is_empty()`) | `ascii_reads_a_terminated_external_string`, `ascii_reads_inline_values` |
| (i) SBYTE read as two-byte chunks | `sints_sign_extend_every_width_in_both_orders` |
| (j) `gated_payload` reads the payload before the cap | `ascii_count_over_the_cap_is_too_large`, `sints_count_cap_and_truncation`, `floats_count_cap_and_truncation` |
| (l) `gated_payload` reads the payload before the type gate | 6: the three cap tests plus `ascii_refuses_other_types_before_touching_the_buffer`, `sints_refuse_unsigned_and_unsigned_readers_refuse_signed`, `floats_refuse_other_types` |

**Fuzz plants**, each red in both `tests/fuzz_layout.rs` and the real `tiff_ifd` target (`cargo +nightly fuzz run tiff_ifd <copy of seeds> -- -runs=0`; unplanted baseline 30 runs clean, `cov: 822 ft: 1799`), then reverted:
- F-e `rest` off by one: `ascii partition`
- F-g `floats` accepts RATIONAL (accept side of the gate iff): `floats type gate, type 5`
- F-g2 `floats` rejects DOUBLE (reject side): `floats type gate, type 12`
- F-i SBYTE as two bytes: `sints length`
- F-h `is_terminated` from the wrong slice: `ascii partition`
- F-m unterminated `rest` = static `&[]`: zero-copy assertion (`tiff_ifd.rs:163`, rest escapes the input)

`check_entry` in the fuzz target and in `tests/fuzz_layout.rs` are textually identical (diffed); the rest of `exercise` mirrors the target. Every type-gate rule is an iff in both directions (F-g/F-g2 confirm).

**`just fuzz-smoke` (60 s, fresh corpus from the 29 seeds):** clean, exit 0. **1,011,646 executions in 61 s** (~16.6k exec/s), final `cov: 984 ft: 4801 corp: 743/417Kb`, peak rss 754 MB. Corpus and artifacts removed afterwards (gitignored).

**Findings (all low / nit; none blocking):**
1. **Low: FLOAT NaN payload is not pinned.** `floats_read_verbatim_in_both_orders` checks FLOAT NaN only with `is_nan()`; the spec asks for a `to_bits()` comparison. A plant mapping every FLOAT NaN to `f64::NAN` would pass. Mitigating: Rust does not guarantee NaN-payload preservation through `f32 -> f64`, so "as the file wrote them" is unattainable for FLOAT NaN by widening; the spec sentence should say so. DOUBLE NaN is pinned bit-exact.
2. **Low: decode arms are not total.** `sints` decodes "not SBYTE/SSHORT" as SLONG (`_ =>`) and `floats` decodes "not FLOAT" as DOUBLE (`else`), unlike `uints`, which keeps an explicit error arm so the match stays total. Widening a gate would silently decode at the wrong width. Caught today by unit tests and the fuzz gate iff (plant g), so a drift risk, not a bug.
3. **Nit: `SEEDS` names 24 of 29 seeds** (missing `subifd-be`, `chain-be`, `payload-past-eof-be`, `count-overflow-be`, `next-past-eof`; pre-existing for 5 of 26). All 29 are still exercised by `every_seed_parses_or_errors_without_panicking` (it walks the directory), so only the "a seed went missing" check is weaker. A one-line follow-up, not a punch-list item.
4. **Nit: the fuzz target checks neither cap-before-payload order nor `Ok` iff for `sints`/`floats`** (`ascii` has its `Ok` iff). Plant (j) passes the fuzz target and is caught only by unit tests. The spec does not require these; adding them is cheap.
5. **Nit: provenance ledger `src/reader.rs` row** does not mention that `i16`/`f32`/`f64` are net-new (SPEC-007); the spec only asked for the `types.rs` note, which is present. The `signed-and-float` seed is little-endian only (BE covered by unit tests).

**Builder deviations judged:** `Ascii::split` empty-tail `rest` (`payload.get(len..)`) is the right call: it keeps the zero-copy invariant total (plant F-m shows a static `&[]` fails it). Extra fuzz assertions (text at payload start, rest right after the NUL, `ascii` `Ok` iff) are sound and both-directional. Re-exporting the `TYPE_*` at `exif_core::tiff` matches the spec's "all in `exif_core::tiff`". No extra single-guard plants by the builder: covered here (a1/a2, f1, g/g2, j, l). Tests not committed red first: the named tests were fixed at design and are shown red by plants, so `test-before-implementation` is met in substance; disclosed by the builder.

### Cost self-report

- **Tokens (total):** 175,106 (recorded by the orchestrator from the sub-agent metered result).
- **Estimated USD:** 1.16
- **Duration (minutes):** ~8 (metered wall-clock)
- **Source of the number:** harness report (subagent_tokens)

### Drift and new artifacts

- **New decisions emitted:** none
- **Deviations from spec:** none by the verifier. Tree clean after every plant and probe; only this handback file is committed.
- **Follow-up work identified:**
  - Optional: make `SEEDS` list all 29 committed seeds (finding 3).
  - Optional: add `Ok` iff (type, cap, payload) for `sints`/`floats` to the fuzz `check_entry` (finding 4); make the `sints`/`floats` decode matches total (finding 2).
  - Spec wording: FLOAT NaN payload preservation is not guaranteed by Rust (finding 1).

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Little. "Compare with `to_bits()` (NaN-safe)" for FLOAT NaN conflicts with Rust's unspecified NaN payload on widening; I had to decide whether the builder's `is_nan` check was a gap or the only honest test.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — No. The worktree-isolation guard rejects shell lines that compute `$HOME`; literal `~/.cargo/bin` paths are needed for fuzz/clippy commands (a toolchain-brief line would save a retry).

3. **If you did this task again, what would you do differently?**
   — Script the plants in one file with a "did the plant apply / does it compile" check from the start (one regex plant matched the wrong function and failed to compile before I anchored it).
