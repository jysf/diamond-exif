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
  id: HANDOFF-017
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
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
  tokens_total: 228956             # orchestrator-recorded from the metered sub-agent result (subagent_tokens)
  estimated_usd: 1.51
  duration_minutes: 14
  branch: worktree-agent-a2f528d5661edeeb8
  pr: null
  completed_at: 2026-11-05         # YYYY-MM-DD
  notes: "tokens transcribed by the orchestrator from the sub-agent metered result"
  synced_at: 2026-11-05
---

# HANDOFF-017: JPEG segment walker

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-009` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

Read before starting:

- **Spec (walk rules and named tests are normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-009-jpeg-segment-walker.md`
- **`decisions/DEC-006-jpeg-segment-walk-and-issue-channel.md`** (policy; normative), DEC-001, DEC-002, DEC-004
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `~/.cargo/bin/cargo +nightly`; `just lint` under the default PATH)
- Mirror the TIFF module: `src/tiff/mod.rs`, `src/tiff/issue.rs`, `src/error.rs`, `src/lib.rs`; and the fuzz layout: `fuzz/Cargo.toml`, `fuzz/fuzz_targets/tiff_ifd.rs`, `tests/fuzz_layout.rs`, `examples/fuzz_seeds.rs`, `app.just`, `.github/workflows/ci.yml`
- Prior art for how a build round is reported and what verify caught: HANDOFF-009/010 (pointer walk: issues recorded after a halt), HANDOFF-011/012, HANDOFF-015/016

Constraints: `panic-free-library` (five-lint deny set in non-test code), `forbid-unsafe-code`, `fuzz-target-per-surface`, `test-before-implementation`, `no-new-top-level-deps-without-decision` (no dependencies of any kind).

## Expected Deliverables

- Everything in the spec's Outputs with all its named tests and every Acceptance Criterion met, including: the five recorded boundary mutations (each named test goes red, then revert), at least three new fuzz invariants shown red with planted bugs (one against the real target), `just fuzz-smoke jpeg_segments` clean for 60 s with executions and coverage recorded, and `rustfmt --check` on the fuzz crate.
- **Write every fuzz iff-invariant in both directions.** For each plant, also try the single-guard variant where two guards overlap (HANDOFF-009's lesson).
- **Nothing is recorded after a halt**, including `MissingSos`/`MissingEoi`, and every issue must go through the single `record` function that enforces the cap. Test it.
- Every iteration of the walk must advance the cursor or end the walk; if you find an input that does not, that is a finding to report, not to paper over.
- Name every deviation (renamed, folded or missing tests) in the Handback; if a derived number in the spec does not match what the rules produce, stop and report it rather than adjusting it.
- CI: make the `fuzz-smoke` job a matrix over `tiff_ifd` and `jpeg_segments`; validate the workflow YAML parses; make `just fuzz-smoke [TARGET]` work for both. Do not change any other CI job.
- Commit on your worktree branch. **Do not push and do not open a PR.** The spec is already at `cycle: build`; do not advance cycles. First run `git merge --ff-only claude/rust-exif-metadata-6351d5` if your worktree was cut from an older commit (you need `d0bf41b` or later).
- Do not modify irradiance, `scripts/`, `decisions/`, the TIFF module, the oracle code, or the ExifTool records. The 29 existing TIFF fuzz seeds must remain byte-identical. No dependencies or dev-dependencies.
- You cannot see your own token count; set `tokens_total: null` with a note, the orchestrator transcribes the metered figure.

## Out of Scope

- Handing APP1 to the TIFF engine (SPEC-010), JPEG fixtures and ExifTool parity (SPEC-011), decoding XMP/ICC/IPTC, writing
- Pushing, PRs, cycle advancement

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

- **Branch / PR:** `worktree-agent-a2f528d5661edeeb8` (committed locally; not pushed, no PR)
- **Completed at:** 2026-11-05
- **All acceptance criteria met?** yes, with the deviations below. Orchestrator re-runs of the gates are still due.
- **For `verify`:** n/a

**Boundary mutations (each named test goes red, then reverted; 1.90.0, `cargo test --lib jpeg`):**

| Mutation | Tests red |
|---|---|
| (a) segment cap `>=` to `>` | `one_over_the_segment_cap_halts_with_too_many_segments`, `nothing_is_recorded_after_a_halt` |
| (b) issue cap `>=` to `>` | `a_flood_of_ff00_pairs_is_capped_at_max_issues_plus_one`, `the_issue_cap_boundary_is_exact`, `nothing_is_recorded_after_a_halt` |
| (c) fill-byte arm removed | `fill_bytes_before_a_marker_are_clean` |
| (d) `L < 2` to `L < 3` | `a_length_of_two_is_a_valid_empty_segment`, `app_segments_are_classified_by_marker_and_signature`, `exactly_max_segments_is_clean`, `one_over_the_segment_cap_halts_with_too_many_segments`, `nothing_is_recorded_after_a_halt` |
| (e) truncated segment recorded partially | `a_length_past_eof_omits_the_segment_and_ends_the_walk` |

**Fuzz plants shown red (harness via `cargo test --test fuzz_layout`, which runs the same invariants over the 19 seeds plus extras):**
p1 trailer starts at `eoi + 1` (red: `trailer`); p2 `MissingEoi` only reported when an SOS exists, an overlapping-guard variant (red: `MissingEoi iff no EOI`); p3 Exif kind loses its marker guard (red: `kind`, via the extra APP2-with-Exif-signature input); p4 Exif kind loses its signature guard (red: `kind`); p5 `after_scan` constant false (red: `after_scan iff after first SOS`); p6 scan region ends at `eoi` not `eoi + 2` (red: `region end`). **Against the real libFuzzer target:** p3 with the committed seeds in the corpus went red (`assertion left == right failed: kind`, exit 77) within the 60 s budget.

**Real fuzz finding (first 60 s smoke, 1,416 executions in):** my own harness invariant "after a cap issue neither `MissingSos` nor `MissingEoi` appears" is false at one boundary. With 1023 ordinary issues, `MissingSos` is the 1024th (recorded) and `MissingEoi` would be the 1025th so it becomes `TooManyIssues`: the list ends `[..., MissingSos, TooManyIssues]`. That is the rules as written (the cap fires inside rule 3, not before it), so the library is right and the spec's fuzz invariant text ("a `TooManyIssues` ... then neither ... appears") is slightly over-strong. I relaxed the harness (segment-cap halt: no Missing*; issue-cap halt: Missing* only as a contiguous run directly before the halting issue and each still true) and added the 1023 case to `the_issue_cap_boundary_is_exact`. No library change. The spec's unit test for the 100,000 `FF 00` flood is unaffected.

**60 s smoke (`just fuzz-smoke jpeg_segments`, after the fix):** clean, `Done 211248 runs in 61 second(s)`, final `cov: 517 ft: 2285 corp: 293/957Kb`, about 3,463 exec/s, exit 0. Initial coverage from the 19 seeds: `cov: 400 ft: 781 corp: 18`.

**Progress / non-advancing cursor:** none found. The 30-input `walk_always_makes_progress` table, the 60 s smoke and the all-`FF` 2 MiB test all terminate; the loop is structurally advancing (stray run >= 1 byte, fill +1, every marker branch >= 2, scan search forward only).

**Gates run here:** `cargo fmt --check` (both graphs), `just lint`, `just lint-fuzz`, `just lint-no-allow`, `cargo rustc --lib -- -F unsafe_code`, `just deny`, `just deny-fuzz`, `cargo test` on Homebrew stable and `+1.90.0` (105 lib tests, 7 fuzz_layout tests, rest unchanged), `just validate`, `just decisions-audit` (0 errors), `just cost-audit`, `just oracle-record` (empty diff under `tests/oracle-fixtures/`, `two-hop-cycle` skipped). CI YAML parses (PyYAML) and the `fuzz-smoke` matrix is `target: [tiff_ifd, jpeg_segments]`. The 29 TIFF seeds are byte-identical (shasum before/after the seed writer, 0 differences).

### Cost self-report

- **Tokens (total):** 228,956 (recorded by the orchestrator from the sub-agent metered result).
- **Estimated USD:** 1.51
- **Duration (minutes):** ~14 (metered wall-clock)
- **Source of the number:** harness report (subagent_tokens)

### Drift and new artifacts

- **New decisions emitted:** none
- **Deviations from spec:**
  - `src/tiff/mod.rs` was touched, test module only: the spec requires extending the exhaustive `every_error_variant_this_module_returns_displays_something` guard, which lives in the TIFF test module, and adding seven variants breaks its wildcard-free match at compile time. No non-test TIFF code changed. The same guard is also duplicated in `src/jpeg/mod.rs` (the named test there).
  - Test names: all spec names kept. Added `the_issue_cap_boundary_is_exact`, `nothing_is_recorded_after_a_halt` (the handoff asks for "test it"), `jpeg_seeds_are_committed`, `every_jpeg_seed_walks_without_panicking`, `jpeg_harness_holds_on_fatal_and_degenerate_inputs`.
  - `length_zero_and_one_are_bad_lengths_and_the_walk_continues` also asserts a second issue `StrayBytes { at: 4, len: 2 }`: after `BadSegmentLength` skips the two marker bytes, the two length bytes (`00 00` / `00 01`) are a non-`FF` run and fall to rule 2.1. That is exactly what the rules produce; the spec text lists only the first issue. Same for the seeds.
  - `fill_bytes_before_a_marker_are_clean` uses `FF D8 FF FF E0` shape built by the `seg` helper (its own `FF` is the one before `E0`, offset 4), matching the spec example offset.
  - Rule 2.2/2.3 ordering: a run of fill that reaches end of input (`FF D8 FF FF`) reports `Truncated { at: <offset of the last FF>, len: 2 }`. The spec's rule 2.2 checks `p + 1 >= len` before the fill loop; I apply the same check after each fill advance. For `FF D8 FF` the result is `at: 2` as specified.
  - `trailer()` with no EOI returns `data[len..]` (an empty slice borrowed from the input) rather than a static `&[]`, so the zero-copy "inside the input" check holds for the empty case too (the harness caught the static slice).
  - Lone `FF` as the last byte of scan data is not an issue (rule 2.8: no marker follows, so `p = len`); only `MissingEoi` is reported.
  - Fuzz-invariant relaxation described above (the cap-inside-rule-3 boundary).
- **Follow-up work identified:** reword the spec's fuzz-invariant bullet about `TooManyIssues` and the end-of-walk pair (DEC-006's "suppresses" sentence too: it suppresses only when the halt precedes the end-of-walk step). Consider whether `JpegIssue` for `StrayBytes` after `BadSegmentLength` should be coalesced; SPEC-011's ExifTool parity may care.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — The cap-versus-end-of-walk interaction (does the issue cap suppress rule 3 or fire inside it) and the doubled FF-fill/truncation ordering; the spec's fuzz-invariant text assumed the former. Also the isolated-worktree shell refuses compound commands, so scripted mutation/plant runs had to be file-based.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — That the TIFF test module's exhaustive error guard must change when `Error` grows (a "do not touch the TIFF module" constraint collides with the spec's own instruction). Worth an explicit carve-out.

3. **If you did this task again, what would you do differently?**
   — Write the cap-boundary cases (1022/1023/1024 ordinary issues) as unit tests before the harness, and write the fuzz invariant from the rules rather than from the prose summary; the first 60 s smoke would then have been clean.

### Follow-up round (verify gaps from HANDOFF-018; tests, seeds and docs only, no change to `src/jpeg` behaviour)

| Mutant | Pinned by | Result |
|---|---|---|
| M8 stray run merges across `FF 00` | `stray_runs_do_not_merge_across_an_ff00_pair` | red |
| M21 `FF D8` in a scan treated as data | `an_soi_inside_entropy_data_ends_the_scan_as_a_stray_marker` | red |
| M24 walk ignores `halted` (segments after an issue-cap halt) | `no_segment_is_recorded_after_an_issue_cap_halt` | red |
| M18 fill run to EOF truncated at first `FF` | `a_fill_run_reaching_the_end_is_truncated_at_the_last_ff` | red |
| M28 segment cap checked before truncation | `a_truncated_segment_is_decided_before_the_segment_cap` | red |
| F3 `0xCC` as `Sof`; `0xC4` not `Dht`; `0xCF` not `Sof` | `sof_dht_and_other_markers_classify_exactly` | red (all three) |
| M14 Jumbf marker guard relaxed | seed `signatures-under-wrong-marker` via `every_jpeg_seed_walks_without_panicking` | red (`kind`) |
| M15 Jumbf signature guard removed | same seed (APP11 `XX`) | red |
| M16 Iptc marker guard relaxed | same seed (APP12 `Photoshop 3.0`) | red |
| M16b Iptc signature guard removed | same seed (APP13 `Photoshop 2.5`) | red |
| ICC marker guard relaxed | same seed (APP1 `ICC_PROFILE`) | red |

Notes: M21 contradicts the coordinator's brief ("FF D8 inside entropy data is data"). Rule 2.8 and the code treat `FF D8` as a real marker that ends the scan (then stray, rule 2.6); the test pins the rules and the code, not the brief. M28 pins the actual order: truncation first, cap only for a segment that would be recorded. Two new seeds (`icc-iptc-jumbf`, `signatures-under-wrong-marker`; 21 JPEG seeds); the other 19 JPEG and 29 TIFF seeds are byte-identical. F4 (seed-writer doc) and F5 (`NotJpeg` Display via `let [a, b] = *mark`) done.
