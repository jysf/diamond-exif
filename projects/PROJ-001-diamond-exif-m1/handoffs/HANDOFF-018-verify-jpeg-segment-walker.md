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
  id: HANDOFF-018
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-05
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-009

project:
  id: PROJ-001
  stage: STAGE-002
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
  tokens_total: 203715             # orchestrator-recorded from the metered sub-agent result (subagent_tokens)
  estimated_usd: 1.34
  duration_minutes: 14
  branch: worktree-agent-adb34d7a8e7bb9e91
  pr: null
  completed_at: 2026-11-05         # YYYY-MM-DD
  notes: "verdict APPROVED at 662aa5e; tokens transcribed by the orchestrator from the sub-agent metered result"
  synced_at: 2026-11-05
---

# HANDOFF-018: Verify the JPEG segment walker

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-009` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-009-jpeg-segment-walker.md` (walk rules and named tests are normative); **DEC-006** (note: the orchestrator reworded its "suppresses" sentence after the build's fuzz finding; judge that wording against the code)
- **Build handoff and Handback:** `HANDOFF-017-build-jpeg-segment-walker.md` (claims to verify, not to trust)
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`), DEC-001, DEC-002, DEC-004
- Code under review: `git diff dcdd328..HEAD` (build commit `04c5711`)

## Expected Deliverables

Review cold; output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

Verify independently:
- Re-run every gate (fmt on both graphs, `just lint`, `just lint-fuzz`, `just lint-no-allow`, `cargo test` on stable and 1.90.0, `-F unsafe_code`, `just deny`, `just deny-fuzz`, `just validate`, `just decisions-audit`, `just cost-audit`, `just oracle-record` empty diff). Run `just fuzz-smoke jpeg_segments` for 60 s and record executions and coverage; also let `tiff_ifd` run briefly to confirm the `fuzz-smoke [TARGET]` recipe still serves both.
- **Read the walk against the numbered rules line by line**: header, stray runs, fill, `FF 00`, standalone markers, bad length, truncation (segment omitted), SOS entropy scan (what ends it: `FF` followed by not `00`, not `D0..D7`, not `FF`), progressive scans, `after_scan`, end-of-walk `MissingSos`/`MissingEoi` ordering and the halt interactions, caps, `scan_region`, `trailer`. Hunt for: a path that records an issue outside the single `record` function; a non-advancing cursor (find an input where `p` does not advance, or prove none exists); arithmetic that could overflow or index out of range; unbounded allocation; any panic-capable construct in non-test code.
- **Judge the spec/builder gaps**: (1) `BadSegmentLength` then `StrayBytes { at: 4, len: 2 }` for the two length bytes (two issues for one fault; is that acceptable or noise?); (2) a fill-byte run reaching EOF reports `Truncated` at the last `FF`; (3) a lone `FF` at the end of scan data is not an issue; (4) `trailer()` with no EOI returns `data[len..]`; (5) the cap-inside-end-of-walk behaviour (`[.., MissingSos, TooManyIssues]`).
- **Test the testers.** Redo the five boundary mutations and confirm each named test goes red; weaken or change several tests and check they fail; write mutants of your own the builder did not try (for example: SOS scan ends at `FF D0` too, RST not skipped; `FF FF` fill not skipped inside a scan; `Sof` classification includes `0xC4`; `after_scan` set on the SOS itself; EOI offset off by one; `L` read little-endian; segment payload range off by one; stray-run merging broken so two runs become one) and report which the suite catches and which survive. Redo at least three fuzz-invariant plants including one against the real fuzz target; confirm the iff-invariants are both-directional.
- Check seeds: the 29 TIFF seeds byte-identical; the 19 JPEG seeds deterministic (regenerate twice, same hashes) and each seed actually exercises the shape its name claims (inspect a few by hand with `xxd`).
- Check the CI workflow change: the `fuzz-smoke` matrix parses and runs both targets; no other job changed.
- Check the `src/tiff/mod.rs` edit is test-module only (exhaustive error guard) and that `[dependencies]`/`[dev-dependencies]` are empty, `scripts/`, `decisions/` (apart from the orchestrator's DEC-006 rewording), the oracle code and records are untouched, and irradiance is untouched.
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes, SPEC-010/011 work, running ExifTool (not needed here), CI wiring beyond reading the workflow

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

- **Branch / PR:** `worktree-agent-adb34d7a8e7bb9e91` (fast-forwarded to `662aa5e`; only this handback is committed on top; not pushed, no PR)
- **Completed at:** 2026-11-05
- **All acceptance criteria met?** yes. The findings below are test and seed gaps plus nits; none is a code defect.
- **For `verify`:** ✅ **APPROVED at `662aa5e`** (code under review: `git diff dcdd328..662aa5e`, build commit `04c5711`)

**Gates (re-run here; all exit 0).** `cargo fmt --all -- --check` and `cargo fmt --manifest-path fuzz/Cargo.toml --all -- --check`; `just lint` (default PATH, clippy 0.1.99); `just lint-fuzz`; `just lint-no-allow`; `cargo test` on Homebrew stable and `~/.cargo/bin/cargo +1.90.0 test` (lib 105, crate_policy 4, fuzz_layout 7, metadata_oracle 28 + 1 ignored, mutation_oracle 36 + 1 ignored); `cargo +1.90.0 rustc --lib -- -F unsafe_code`; `just deny`, `just deny-fuzz` (licenses ok); `just validate`; `just decisions-audit` (0 structural errors; scope warnings only); `just cost-audit`; `just oracle-record` (ExifTool 13.55, 41 records, `two-hop-cycle` not run, **empty diff**). `[dependencies]` is empty and there is no `[dev-dependencies]` table.

**Fuzz smoke.** `just fuzz-smoke jpeg_segments`: clean, `Done 156650 runs in 61 second(s)`, coverage `cov: 400 ft: 781 corp: 18` at INITED, then `cov: 510 ft: 2243 corp: 297/1237Kb` (about 2,568 exec/s). `just fuzz-smoke tiff_ifd`: clean, `Done 1293296 runs in 61 second(s)`, `cov: 984 ft: 4808 corp: 735`. `just fuzz-smoke bogus` exits 2 with `unknown target 'bogus'`. The builder's 211,248 runs and `cov: 517` differ only by host variance.

**Walk read line by line against the rules.** Header (rule 1), stray runs (2.1, run >= 1 because `data[p] != FF`), lone `FF` (2.2), fill (2.3, re-checked after each advance), `FF 00` (2.4), EOI (2.5), standalone `01`/`D0..D8` (2.6), length (2.7: `p + 4 > len` gives `Truncated{len:4}`; `L < 2` gives `BadSegmentLength` and `p += 2`; past EOF omits the segment; `L == 2` is legal), SOS entropy scan (2.8: ends at `FF` followed by anything except `00`, `D0..D7` or `FF`; `FF FF` advances one), `after_scan` (computed before the first SOS sets `scan_start`, so the first SOS is false), end-of-walk order, caps, `scan_region`, `trailer`: all match the spec. Every issue goes through `Walk::record` (the only two `issues.push` calls are inside it). `record` is a no-op after a halt, and the issue cap is `issues.len() >= MAX_JPEG_ISSUES`. **The cursor always advances.** Proof: 2.1 adds the run (>= 1); the fill loop adds 1 per `FF`; 2.4 and 2.6 add 2; `BadSegmentLength` adds 2; a recorded segment returns `end >= p + 4`; `skip_entropy(end)` returns a value >= `end`; every other branch returns. I found no input that stalls. All arithmetic is `saturating_*`, and every slice goes through `.get(..)` (`lint-no-allow` is green). Allocation is bounded at 4096 segments and 1025 issues.

**Builder's five boundary mutations, redone (each red, then reverted):**

| Mutation | Red |
|---|---|
| (a) segment cap `>=` to `>` | `one_over_the_segment_cap_halts_with_too_many_segments`, `nothing_is_recorded_after_a_halt`, fuzz_layout `every_jpeg_seed_walks_without_panicking` |
| (b) issue cap `>=` to `>` | `a_flood_of_ff00_pairs_is_capped_at_max_issues_plus_one`, `the_issue_cap_boundary_is_exact`, `nothing_is_recorded_after_a_halt`, `every_jpeg_seed_walks_without_panicking` |
| (c) fill arm removed | `fill_bytes_before_a_marker_are_clean` |
| (d) `L < 2` to `L < 3` | `a_length_of_two_is_a_valid_empty_segment`, `app_segments_are_classified_by_marker_and_signature`, `exactly_max_segments_is_clean`, `one_over_the_segment_cap_halts_with_too_many_segments`, `nothing_is_recorded_after_a_halt` |
| (e) truncated segment recorded partially | `a_length_past_eof_omits_the_segment_and_ends_the_walk`, `every_jpeg_seed_walks_without_panicking` |

**My own mutants (lib tests + `tests/fuzz_layout.rs`, 1.90.0):**

| # | Mutant | Result |
|---|---|---|
| M1 | scan ends at `FF D0..D7` (RST not skipped) | killed: `a_minimal_jpeg_walks_clean`, `entropy_data_hides_no_markers_but_ends_at_a_real_one` |
| M2 | `FF FF` not skipped inside a scan | killed: `entropy_data_...`, `a_large_all_ff_scan_is_linear_and_terminates` |
| M3 | `Sof` includes `0xC4` | killed: classification, entropy, progressive, seeds |
| M4 | `after_scan` set on every SOS itself | killed: minimal, progressive, seeds |
| M5 | EOI offset + 1 | killed: 6 unit tests + both harness tests |
| M6 | `L` little-endian | killed: 22 tests |
| M7 | payload range one short | killed: 9 unit tests + both harness tests |
| M8 | stray run merges across `FF 00` (`01 FF 00 02` becomes one run) | **SURVIVED** |
| M9 | TEM dropped from the standalone set | killed: `ff00_and_a_mid_stream_soi_outside_entropy_data_are_stray` |
| M10 | `MissingEoi` before `MissingSos` | killed: 5 tests incl. `soi_only_...`, `the_issue_cap_boundary_is_exact` |
| M11 | `MissingSos` offset 0 | killed: `soi_only_reports_missing_sos_then_missing_eoi` |
| M12 | `Truncated` len = `L` not `2 + L` | killed: `a_length_past_eof_...` |
| M13 | `scan_region` always ends at input end | killed: `a_trailer_after_eoi_...`, seeds |
| M14 | Jumbf marker guard relaxed to any APPn | **SURVIVED** |
| M15 | Jumbf signature guard removed | **SURVIVED** |
| M16 | Iptc marker guard relaxed to any APPn | **SURVIVED** (also against the real target: 300,082 runs / 121 s, not found) |
| M17 | `0xCC` classified `Sof` | **SURVIVED** the plain tests; the **real fuzz target** caught it in ~5 s (see below) |
| M18 | fill run to EOF reports `Truncated` at the first `FF`, not the last | **SURVIVED** (gap 2 is unpinned) |
| M19 | stray run len + 1 | killed: 3 tests |
| M20 | `BadSegmentLength` skips 4, not 2 | killed: `length_zero_and_one_...` (gap 1 is pinned) |
| M21 | `FF D8` inside a scan treated as data | **SURVIVED** |
| M22 | lone `FF` at the end of a scan becomes a marker | killed: `a_large_all_ff_scan_...` (gap 3 is pinned) |
| M23 | issue-cap halt does not set `halted` | killed: flood, boundary, after-a-halt, seeds |
| M24 | walk loop ignores `halted` (segments still recorded after an issue-cap halt) | **SURVIVED** |
| M25 | rule 3 not guarded by `halted` | survived; **equivalent** (`record` is a no-op after a halt) |
| M26 | `scan_start` taken from the last SOS | killed: progressive, seeds |
| M27 | static `&[]` trailer without EOI | killed only by the harness (`jpeg_harness_holds_...`, seeds); gap 4 is pinned |
| M28 | segment cap checked before truncation | **SURVIVED** (ordering corner, no test) |

**Fuzz-invariant plants (harness red in `tests/fuzz_layout.rs`, which mirrors the target; `check`/`exercise_jpeg` and `expected_kind` are byte-identical between the two files):** P1 `after_scan` constant true (red, the builder did constant false); P2 `MissingSos` always and P6 `MissingSos` never (both red: the iff holds both ways); P3 `MissingEoi` never (red); P4 `parse_strict` always `Ok` and P7 `parse_strict` `Err` on clean input (both red); P5 `scan_region` always `None` and P8 `Some` without an SOS (both red); P9 overlapping single guard, `MissingSos` only when EOI is also missing (red, via the `no-sos` seed); P10 Exif marker guard relaxed (red, via the builder's extra APP2 input). **Against the real libFuzzer target:** M17 (`0xCC` as `Sof`) went red at `fuzz_targets/jpeg_segments.rs:132` (`assertion left == right failed: kind`) after about 20,000 executions; reverted.

**Tests of the tests:** changed expectations go red: fill offset 4 to 3 (`fill_bytes_...`), stray len 3 to 4 (`stray_bytes_...`), exact cap fed 4097 segments (`exactly_max_segments_is_clean`), `Truncated` len 258 to 256 (`a_length_past_eof_...`), harness `after_scan` `>` to `>=` and region end `eoi + 2` to `+ 1` (both `every_jpeg_seed_walks_without_panicking`). All 28 spec-named unit tests are present.

**Seeds.** The 29 TIFF seeds are byte-identical to `dcdd328`. Two regenerations (`cargo run --example fuzz_seeds`) gave identical SHA-1s for all 48 files, equal to the committed ones, with a clean tree. By hand (`xxd`): `app1-len-0`/`-1` (APP1 `00 00`/`00 01` then COM), `app1-len-2`, `app1-len-ffff` (65,533 `Z` then SOS/EOI), `app1-past-eof` (`L = 0x0100`, 12 bytes), `fill-bytes` (`FF FF FF FF E0`), `ff00-stray` (`FF 00`, `FF D8`, `FF D3`, `FF 01`), `app-after-scan`, `sos-no-eoi`, `trailer-after-eoi`, `progressive-two-scans` (SOF2, SOS, DHT, SOS, `FF FF D3`), `extended-xmp-pair` (XMP + 2 extended), `many-ff00` (1,100 pairs, over the issue cap), `many-segments` (4,103 COMs, over the segment cap). Each matches its name.

**Scope checks.** The CI diff is one hunk: the `fuzz-smoke` job only (matrix `[tiff_ifd, jpeg_segments]`, `fail-fast: false`); the YAML parses and the other 13 jobs are unchanged. The `src/tiff/mod.rs` edit is at line 1656+, inside `mod tests` (starting at line 798): the guard only. `scripts/`, the oracle code and records, and `tests/oracle-fixtures` are untouched. `decisions/` differs only by the orchestrator's DEC-006 rewording. irradiance is not touched by this diff.

**Judgement of the five builder gaps:**
1. `BadSegmentLength` then `StrayBytes { at: p+2, len: >= 2 }`: rule-faithful and **acceptable for now**. It is deterministic noise: `L < 2` means the length bytes are `00 00`/`00 01`, which are never `FF`, so every `BadSegmentLength` is always followed by a stray run, and one fault spends two issues of the cap. Suggest that a follow-up (or SPEC-011, if ExifTool parity cares) change the rule to `p += 4`. M20 shows the current behaviour is pinned by a test.
2. A fill run reaching EOF reports `Truncated` at the last `FF`: **acceptable**. It is consistent with rule 2.3's "p is the marker's own `FF`". The spec's 2.2/2.3 wording is ambiguous, and no test pins the choice (M18 survives). Pin it with a test and reword 2.2.
3. A lone `FF` at the end of scan data is no issue: **acceptable** and the rules as written (2.8, `p = len`). `MissingEoi` tells the story. Pinned (M22 red).
4. `trailer()` without EOI returns `data[len..]`: **correct, and better** than a static empty slice (the zero-copy invariant holds even when empty). Pinned by the harness (M27).
5. `[.., MissingSos, TooManyIssues]`: **correct** per rules 3 and 4 (the cap fires inside rule 3; nothing follows a halt). Pinned by `the_issue_cap_boundary_is_exact` (M10 and (b) red). The reworded DEC-006 sentence matches the code. It now names only the segment cap as suppressing the end-of-walk pair, and leaves implicit (the spec's rule 3 states it) that an issue-cap halt **during** the walk also suppresses it. That is a wording nit only.

**Findings (none blocking):**
- **F1 (low, test gap):** five behaviour mutants survive both suites: M8 (stray runs merge across `FF 00`), M21 (`FF D8` inside a scan treated as data), M24 (segments recorded after an issue-cap halt; the "nothing after a halt" test covers issues only), M18 (gap 2) and M28 (cap vs truncation order). Suggested tests: `01 FF 00 02` gives three issues; a scan containing `FF D8` ends there; with an issue-cap halt followed by a valid COM, the COM is not in `segments()` (or a harness invariant: no segment offset after the halting issue's offset); `FF D8 FF FF` gives `Truncated { at: 3 }`.
- **F2 (low, seed and harness coverage):** no seed carries the ICC, IPTC or JUMBF signatures, or a signature under the wrong marker. The marker and signature guards for Jumbf (M14, M15) and Iptc (M16) survive the plain-test harness, and the real target did not find M16 in 300k runs over 121 s. Add seeds for `ICC_PROFILE\0` in APP2, `Photoshop 3.0\0` in APP13 and `JP` in APP11, plus each signature under a neighbouring APPn, and an APP11 without `JP`.
- **F3 (low, unit-test gap):** `Other(0xCC)` is untested in `app_segments_are_classified_...` (only `0xC8`); the real fuzzer finds it quickly (M17).
- **F4 (nit):** the module doc of `examples/fuzz_seeds.rs` (lines 1-8) still says it writes only the `tiff_ifd` seeds.
- **F5 (nit):** the `NotJpeg` `Display` uses `mark.first()/last().unwrap_or(0)`; `let [a, b] = *mark;` is direct and lint-clean.
- **F6 (info):** gap 1 as above (two issues per bad length); a design call for a later spec, not a defect.

### Cost self-report

- **Tokens (total):** 203,715 (recorded by the orchestrator from the sub-agent metered result).
- **Estimated USD:** 1.34
- **Duration (minutes):** ~14 (metered wall-clock)
- **Source of the number:** harness report (subagent_tokens)

### Drift and new artifacts

- **New decisions emitted:** none
- **Deviations from spec:** none beyond those the builder named (all judged acceptable above). The builder's handback is accurate: every claim I re-ran held, apart from host-variance differences in fuzz execution counts.
- **Follow-up work identified:**
  - Add the F1 tests and the F2 seeds (small; can ride with SPEC-010 or a patch).
  - Decide gap 1 (`p += 4` after `BadSegmentLength`) when SPEC-011 compares with ExifTool.
  - Reword SPEC-009 rule 2.2/2.3 for a fill run reaching EOF, and DEC-006's suppression sentence so it covers an issue-cap halt during the walk.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Very little. The isolated-worktree shell refuses compound or looping commands, so every mutant, plant and seed check had to run from a script file in the scratchpad (the builder reported the same).

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — "Halting stops the walk" covers segments as well as issues, but the handoff's "nothing is recorded after a halt" was read (and tested) as issues only. Stating "no segment and no issue after a halt" would have produced the missing test (M24).

3. **If you did this task again, what would you do differently?**
   — Run the real fuzz target against each surviving classification mutant first: whether the fuzzer can reach a guard is the real test of the seed set, and it exposed the IPTC/JUMBF gap faster than reading the seed list.
