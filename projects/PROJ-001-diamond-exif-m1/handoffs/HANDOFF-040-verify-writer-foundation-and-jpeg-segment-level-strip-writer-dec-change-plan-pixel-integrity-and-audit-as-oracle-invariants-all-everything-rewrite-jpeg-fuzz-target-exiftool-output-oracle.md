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
  status: completed                # pending | accepted | completed | rejected

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
  status: completed                # completed | blocked | rejected
  tokens_total: 299050             # final subagent_tokens, transcribed by the orchestrator
  estimated_usd: 1.97              # 299050 x 6.60 USD per Mtok = 1.974
  duration_minutes: 19             # 1146426 ms = 19.1 min
  branch: worktree-agent-a3cfa29f74ce0361a
  pr: null
  completed_at: 2026-11-10         # YYYY-MM-DD
  notes: "Verdict APPROVED at 44f0bbc with two Medium follow-ups (audit-side) and Low findings. The sub-agent could not see its own token count; the orchestrator transcribed the final metered figure. Nothing pushed, no PR."
  synced_at: 2026-11-10
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

## Handback

**Verdict: APPROVED at `44f0bbc`** (build `25b2b37` plus the orchestrator's cost record). SPEC-020's acceptance criteria hold as written. The two Medium findings are in the audit/DEC layer, which SPEC-020 may not change ("no change to the audit"), so they are follow-ups, not rework for this spec. They should become questions or specs before the writer ships to users.

### Findings

- **Medium. Variant APP1 signatures that ExifTool reads survive `All` and `Gps` with `Ok`.** `src/jpeg/mod.rs:189-193` (`SIG_EXIF`, `SIG_XMP` exact prefixes), with the effect in `src/write/jpeg.rs:135` (they are `OtherApp`, kept unless `OtherSegment` is in the policy). Scenario: a JPEG whose APP1 payload is `exif\0\0` + TIFF (lowercase), `XYExif\0\0` + TIFF (garbage prefix), or `XMP\0` + packet. The audit reports only `OtherSegment`. `strip(All)` and `strip(Gps)` return `Ok` with 0 bytes removed. ExifTool 13.55 still reads 5 `GPS`, 6 `IFD0`, 4 `ExifIFD` and 3 `IFD1` tags (warning "[minor] Incorrect EXIF segment identifier" or "Unknown garbage at start of EXIF segment"), or `XMP-xmp:CreatorTool` ("Non-standard header for APP1 XMP segment"). `Everything` removes them. `Adobe_Photoshop2.5:`, `Photoshop 3.0 ` (no NUL) and Exif-in-APP0 were checked: ExifTool reads no tag from them. Ran: a synthetic probe (verifier scratch, not committed), piped to `exiftool -a -u -G1 -s -`. DEC-013 part 3 makes the audit the single definition, so the writer is right by its contract. The product promise ("the bytes are gone", checked against ExifTool) fails on these inputs.
- **Medium. A GPS IFD that the tolerant walk cannot read counts as "readable" for the Exif rule.** `src/write/jpeg.rs:206-207`: `unreadable` only uses `failed_blocks` (IFD0/header failures) and the more-than-16-segments test. Scenario: `full_tiff(true)` with the GPS IFD entry count set to `0xFFFF`. The audit then has 0 GPS findings, `exif_issues = 1` and `failed_blocks = 0`. `strip(Gps)` returns `Ok`, unchanged, and the GPS IFD's bytes (latitude and longitude rationals, the `CANARY-GPS-AREA` text) stay in the file. ExifTool also reads 0 GPS tags, so the oracle sees no leak. DEC-013 part 4 says "could not be read", and a per-IFD issue in an IFD of the policy's category is the case DEC-001's tolerant walk exists to express. Fail-closed here would be `NeedsEntryEdit`. Five other GPS corruptions (count+1, type 0, value offset out of range, garbage next pointer, all types 99) still give `NeedsEntryEdit`.
- **Low. `VerifyFailed` is reachable, and the fuzz harness accepts it.** 39 of 200,000 random 1-to-4-byte mutations of the committed seeds, over 12 policies, gave `VerifyFailed`: Scan 11, Walk 7, MpfFields 18, Kept 3. Each one was accepted by `rewrite_invariants::check_input` through `VerifyAllowed`'s "prediction breaks an invariant" branch (`fuzz/shared/rewrite_invariants.rs:965-975`). Example (Scan): fill bytes + a COM + stray `FE` + `FF FF DB`. With the COM removed, the fill joins the stray byte into a new `FF FE FF FF` COM that swallows DQT and the scan (question `writer-stray-bytes`). Every case fails closed. Still, "VerifyFailed is always a harness failure" is untrue beyond the MPF underflow named in question `dec-013-mpf-primary-size-underflow`, and a dedicated refusal for that underflow alone would not make it true.
- **Low. The harness does more than "only `audit(input)` for Exif facts".** `facts` uses `Jpeg::parse` (line 239) and `audit(output)`. `predicted_output_breaks` (lines 977-1015) uses `audit`, `Jpeg::parse` and `jpeg::read`, and it decides whether a library `VerifyFailed` is excused, so a bug shared by the audit and the self-check would be accepted. It also imports the caps `MAX_JPEG_SEGMENTS`, `MAX_JPEG_ISSUES` and `MAX_MPF_IMAGES` from the library. It never calls `exif_core::write` except `strip` (the code under test) and the error/enum types. Judged acceptable: the naive walker, MPF reader, overlap logic and arithmetic are independent, and the output is predicted byte for byte.
- **Low. Mutant gaps (my mutants, all reverted, tree clean).** These survived the SPEC-020 subset (`--lib --test strip --test strip_oracle --test fuzz_layout`):
  - v03b, "primary size patched only when it equalled the primary length": no fixture has a wrong primary size that is still patchable.
  - v05, "self-check (b) ignores region kind": no test has two kinds in one category change.
  - v06, "self-check skipped when the plan is empty": it can only matter together with a planner bug.
  - v10, "MpfFields compares input images to input images": the image-bytes part of `mpf_fields` is never the deciding check.
  - v02, "offset patch uses the image's end": an equivalent mutant, because `Overlap` forbids any removal inside a kept image. Not a gap.
- **Low. Seed coverage by policy-byte family.** Of the 129 seeds, bytes 0, 1, 2 and 5 occur once each (kitchen sink only), byte 3 (`All` preset) never (`All` is seeded as byte 7), byte 4 has 41, byte 6 has 42, byte 7 has 42. The spec's requirement (each fixture and shape under All, Everything and a mask) is met. "Every input class under each family" is not met.
- **Low. Warning normalisation (deviation 8).** It replaces every digit run, so a new warning that differs from an input warning only in a number (for example an image index `MPImage2` vs `MPImage3`) is hidden. No real difference was found to be hidden.
- **Low (process).** In the spec, `cost.sessions[0].notes` says "tokens_total null" while `tokens_total` is 564890.

### Gates (re-run here, all exit 0)

fmt (crate and fuzz graph), `rustfmt --check --edition 2021 fuzz/shared/*.rs`, `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `-F unsafe_code` on lib and bin, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `cargo build --release --locked --bin exif-audit`. `cargo test --no-fail-fast` with `DIAMOND_EXIF_REQUIRE_ORACLE=1 DIAMOND_EXIF_REQUIRE_CLI_FS=1`: rustup stable gives 757 passed, 0 failed, 9 ignored; `+1.90.0` gives the same. Every named test in the spec exists and passed. `just oracle-record` and `cargo run --example fuzz_seeds` leave an empty `git status --porcelain`. `decisions/`, `scripts/`, `Cargo.toml` and `Cargo.lock` are unchanged since 81c9132. `fuzz/Cargo.toml` only gains the bin. No `allow` outside the `#[cfg(test)]` module, no `unsafe`, no unwrap/expect/indexing/panic in `src/write` non-test code. Older seed sets are unchanged.

Fuzz smokes (60 s): `rewrite_jpeg` 65,106 runs (cov 3389), `jpeg_segments` 164,330, `audit` 68,128, `audit_render` 35,896, `cli_args` 2,453,661, all clean.

**Plant:** removal records' `len` shortened by 1 (`src/write/jpeg.rs:83`), a bug the self-check cannot see. The real libFuzzer target, run from an empty corpus plus the committed seeds only, crashed while loading the seeds with `guards [Records] failed`. Reverted.

### Mutants

Builder's mutants, re-applied: 1, 2, 3, 4, 11, 13, 14, 17, 18, 20, 21, 22, 23, 29 and 30 are all caught (20, 21 and 22 by one verify unit test each, matching the Handback). Mine: v01 (patch written at its input offset), v03a, v04 (overlap ignores kept images), v07 (one-category EXIF9 skips the entry edit), v08 (trailer by count), v09 (walk compares input to input) are caught; v02 is equivalent; v03b, v05, v06 and v10 survive (above). v09 is caught only by `a_plan_that_breaks_the_walk_fails_check_walk`.

### Pixel integrity, oracle, corpus

- An independent I1 recompute (delete the listed input ranges, compare every byte outside the listed patch ranges) holds on every fixture-like probe, on 17/17 public JPEGs under All and Everything, and on 13/13 local JPEGs under every preset that returned `Ok`. The scan region is identical in each case.
- **Adversarial probes, all fail closed or `Ok` with I1:** `FF D8 FF D9`; `FF D8` only; Exif + a COM holding `FF D8`; byte-order marks `IM` and swapped `II` (`NeedsEntryEdit` for partial policies, removed whole under All); 100 COMs; a COM right before SOS; a COM between two scans (`Overlap`, question 4); an MPF secondary offset near `u32::MAX`; duplicate entries; an entry into the index; an entry pointing at a removed XMP (`MpfNotIndexed`); a size past the end (`MpfNotIndexed`, deviation 4d); the index as the last segment; gap trailers next to kept images under OtherSegment; an image inside a COM (`Overlap`); a primary size that is wrong but large (`Ok`) or smaller than the removal (`VerifyFailed {MpfFields}`, as specified).
- **ExifTool on public samples:** groups measured on 17 public JPEGs. Under All only ICC, JFIF and Adobe stay; under Everything only JFIF and Adobe. No new warnings. This agrees with the hand-typed table (`tests/support/strip_records.rs:190-258`). The table is built from the test model's removals, not the library's.
- **Local corpus** (counts and short hashes only, nothing written): 21 files, 13 JPEGs, 4 with MPF. All: 13 Ok, 313,995 bytes, 4 patches, scan 13/13, clean 13/13. Everything: 13 Ok, 3,352,855 bytes, 0 patches. Gps: 4 NeedsEntryEdit, 9 Ok unchanged; ExifTool finds GPS in exactly 4 of the 13. GpsAndDevice: 6 NeedsEntryEdit, 7 Ok unchanged. AllPii: 13 NeedsEntryEdit.
- **The four MPF files** (83e006fe, fc448610, a85c304e, 14fe1c56) through ExifTool by pipe: 3 MPImageStart before and after All, 0 after Everything, 0 warnings. Each secondary that ExifTool extracts (PreviewImage or MPImage2/3) is byte-identical (same sha1 prefix) before and after All and starts `FF D8`.

### The builder's questions and deviations

All five pinned behaviours fail closed.
1. `dec-013-exif-resident-xmp-iptc-icc-tags`: safe, though it over-removes a kept-category tag under All. **Decide before SPEC-021**, which edits entries.
2. `dec-013-unlocated-mpf-entries`, refuse: safe. Amend DEC-013 part 5 **before SPEC-021**, because in-segment edits move offsets the same way.
3. `dec-013-mpf-segment-after-a-removed-index`: safe. Fold it into the same amendment.
4. `dec-013-removable-segment-after-the-first-scan`: safe and rare; it can wait.
5. `dec-013-mpf-primary-size-underflow`: safe. Decide it together with `writer-stray-bytes` and the harness's `VerifyAllowed` escape hatch (Low finding 1), not alone.

Deviations 2, 4(a-d) and 5 are each justified by DEC-013 parts 2 and 5 as fail-closed narrowings, not silent relaxations. The refusal order (deviation 5) is new contract and should be written into DEC-013. Deviation 3's 7(b) exemption is a real relaxation of the self-check, but only for findings inside removed Exif segments.
