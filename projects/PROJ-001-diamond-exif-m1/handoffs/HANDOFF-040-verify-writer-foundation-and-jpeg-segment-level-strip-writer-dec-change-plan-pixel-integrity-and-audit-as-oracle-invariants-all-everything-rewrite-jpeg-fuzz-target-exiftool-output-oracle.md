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
  id: HANDOFF-040
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-10
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-020

project:
  id: PROJ-001
  stage: STAGE-004
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
  status: null                     # completed | blocked | rejected
  tokens_total: null               # REAL combined count — what cost-audit reads
  estimated_usd: null              # tokens_total × your rate, or your harness's number
  duration_minutes: null
  branch: null
  pr: null
  completed_at: null               # YYYY-MM-DD
  notes: null                      # one line if unusual (rework, no meter, etc.)
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-040: Verify the writer foundation and JPEG segment-level strip (SPEC-020)

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-020` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-020-*.md` (rules 1 to 11, the fixtures, the named tests and the 30 mutants are the contract).
- **DEC-013 (normative), DEC-012 (with amendment), DEC-006, DEC-008, DEC-009, DEC-010 with amendments:** `decisions/`.
- **Build handoff and Handback:** `HANDOFF-039-build-...md` (claims to verify, not to trust; read the eleven deviations and the five raised questions in `guidance/questions.yaml`). The builder extended the `MpfNotIndexed` rule four ways (deviation 4) and pinned a refusal order (deviation 5); judge whether each is justified by DEC-013 part 5 or is a silent change of the contract.
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe; python reading ExifTool output needs `errors="replace"`; macOS `sed -i ''`).
- Code under review: `git diff 81c9132..HEAD` (the build, ending `25b2b37`, plus the orchestrator's cost record). Main: `src/write/{mod,plan,jpeg,verify,verify_tests}.rs`, `src/jpeg/{mod,metadata}.rs` (`Segment::range`, `MpfIndex`), `tests/strip.rs`, `tests/strip_oracle.rs`, `tests/support/{strip_shapes,strip_records}.rs`, `tests/oracle-fixtures/exiftool-strip/`, `fuzz/fuzz_targets/rewrite_jpeg.rs`, `fuzz/shared/rewrite_invariants.rs`, `fuzz/seeds/rewrite_jpeg/`, `examples/strip_corpus.rs`.
- Local photos: `/Users/jyashinsky/PSeven/experiments/crustimg_redo_plus/_incoming0`. **Privacy rule: read in place, copy nothing, write no derived file (no temp files, no saved ExifTool output; pipes only), record only counts and short hashes; never tag values, file contents or paths.** Public samples may be in the orchestrator scratchpad `.../scratchpad/public/{A,B,C}`.
- Orchestrator's own gate run (all 26 steps exit 0; 757 tests on each of three test runs; five 60 s fuzz smokes clean) is evidence, not a substitute: re-run.

## Expected Deliverables

Review cold; output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

Verify independently:
- **Gates:** fmt both graphs and `rustfmt --check --edition 2021 fuzz/shared/*.rs`; `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`; `cargo test --no-fail-fast` on stable and 1.90.0 with `DIAMOND_EXIF_REQUIRE_ORACLE=1 DIAMOND_EXIF_REQUIRE_CLI_FS=1`; `-F unsafe_code` on lib and bin; `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`; `oracle-record` leaves an empty `git status --porcelain`; fuzz smokes for `rewrite_jpeg`, `jpeg_segments`, `audit`, `audit_render`, `cli_args`; release build of `exif-audit`. Confirm `decisions/` and `scripts/` unchanged, no dependency added, no `allow` attribute, no `unsafe`, no `unwrap`/`expect`/indexing/panic/arithmetic in `src/write/`.
- **Read the writer line by line.** `plan.rs`: validation, merge of removals, patch inside a removal, checked arithmetic, `output.len() <= input.len()`. `jpeg.rs`: the planner's rules in the spec order; segment-kind removal (deviation 2); the Exif all-or-nothing rule and `NeedsEntryEdit` (including unreadable blocks, more than 16 Exif segments, Exif-resident Xmp/Iptc/Icc, deviation 3); the MPF unit, the patch arithmetic (`offset - removed(header_start, image_start)`, primary size, byte order, underflow), `MpfNotIndexed` and its four extensions; `Overlap`. `verify.rs`: each of the six checks really compares what DEC-013 part 7 says (kept `Trailer` by total byte count; scan equality byte for byte; the idempotence check re-plans). Try to break it with inputs the builder did not: an Exif block plus a COM that also contains `FF D8`; MPF entries that overlap or point into the index; a primary-size field that is already wrong; offsets near `u32::MAX`; two removals adjacent to a kept image; a removal exactly at the start of the scan; 100 segments; a file that is only `FF D8 FF D9`; an MPF index in the last segment; an Exif segment with the byte-order mark reversed.
- **Fail-closed claim.** For every `Err` path confirm no bytes escape, and that no `Ok` path can return a file with a finding in a removed category. Look for a path where the self-check is skipped, short-circuited, or compares the input to itself.
- **Pixel integrity (I1).** Recompute the expected output independently from the listed removals and patches for the fixtures, the public samples and the local JPEGs, and compare byte for byte; confirm the scan region is identical.
- **Oracle discipline.** ExifTool is run, never linked. Re-measure the group tables with ExifTool on at least three public samples and the synthetic MPOs; check that the expected table in `tests/strip_oracle.rs` states what ExifTool printed, not what the library did; the five red-proofs still go red; digit-run normalisation of warnings (deviation 8) cannot hide a real difference.
- **Test the testers.** Re-run the builder's 30 mutants for at least 1, 2, 4, 11, 13, 14, 17, 18, 20, 21, 22, 23, 29 and 30, including the two the builder had to rewrite (3 and 30). Write mutants the builder did not try, for example: patches applied before removals are merged; the MPF offset patch uses the secondary's end instead of its start; entry 0's size field patched when it did not match the primary; `Overlap` checked only against kept segments, not kept embedded images; self-check (b) comparing categories but not region kinds; the self-check skipped when the plan is empty; `NeedsEntryEdit` skipped for a policy of exactly one EXIF9 category; trailer pieces compared by count instead of total bytes; the harness stripper in `rewrite_invariants.rs` calling library code. Report which the suite catches (name the test or guard) and which survive.
- **Fuzz.** Redo one plant against the real libFuzzer `rewrite_jpeg` target from the committed seeds only. Check that the harness is independent of the library (list every library call it makes and judge each; the builder says only `audit(input)` for Exif-block facts), that `VerifyFailed` is a failure, that every input class has seeds under each of the policy-byte families, and that the older corpora and their recorder counts are untouched.
- **Local real-file run (counts only).** Run `examples/strip_corpus.rs` over `_incoming0` under `All`, `Everything` and `Gps` and reproduce the builder's numbers (21 files, 13 JPEGs, 4 with Multi-Picture; under `All` and `Everything` 13 Ok, scan identical 13/13, audit clean 13/13; `All` removed 313,995 bytes with 4 patches; `Everything` 3,352,855 bytes with 0 patches). Report what `Gps` and `GpsAndDevice` do on real files (how many are `NeedsEntryEdit`). Pipe a few stripped outputs to `exiftool -` (in memory only) and compare groups and `MPImageStart` with the input's.
- **Judge the questions.** For each of the five questions the builder raised, say whether the pinned behaviour is safe (fails closed) and whether it needs a decision before SPEC-021.
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes; entry-level Exif editing (SPEC-021); TIFF (SPEC-022); keeping Orientation (`strip-keeps-orientation`); changing DEC-013 (report doubts as questions).
