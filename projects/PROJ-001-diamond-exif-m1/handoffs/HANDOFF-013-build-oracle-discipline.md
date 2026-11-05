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
  id: HANDOFF-013
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-05
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-005

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
  tokens_total: 215404             # orchestrator-recorded from the metered sub-agent result (subagent_tokens)
  estimated_usd: 1.42
  duration_minutes: 17
  branch: worktree-agent-ae70592c6152a9b0f
  pr: null
  completed_at: 2026-11-05
  notes: "tokens transcribed by the orchestrator from the sub-agent metered result"
  synced_at: 2026-11-05
---

# HANDOFF-013: ExifTool read-parity oracle

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-005` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

Read before starting:

- **Spec (normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-005-oracle-discipline.md`
- `docs/porting-brief-diamond-exif.md` §2.2 and §2.3, `docs/exif-core-design.md` §8, `AGENTS.md`, `guidance/toolchain-brief.md`, `guidance/constraints.yaml` (`oracle-must-be-shown-red`, `exiftool-never-linked`, `no-new-top-level-deps-without-decision`)
- **Pattern source, READ ONLY, never modify:** `~/PSeven/experiments/crustimg_redo_plus/irradiance/tests/support/tools.rs` and `tests/metadata_oracle.rs` (lines ~458-545). Port with edit and add ledger rows (DEC-003). **ExifTool's own source must never be read**; ExifTool is only run.
- `src/tiff/` public API, `fuzz/seeds/tiff_ifd/*` (inputs only), `examples/fuzz_seeds.rs` (byte-builder style)
- Prior art for how a build round is reported: HANDOFF-011 (SPEC-007 build)

Host: ExifTool 13.55 is at `/opt/homebrew/bin/exiftool`. Host toolchain traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint` under the default PATH.

## Expected Deliverables

- Everything in the spec's Outputs with all its named tests and every Acceptance Criterion met. In particular:
  - the replay control and per-variant perturbation red-proofs, the missing-field and extra-field tests;
  - the **live** demonstration that a mutated payload is caught (applied to a copy inside the test, nothing committed);
  - both skip modes demonstrated and recorded (hidden ExifTool prints `SKIP` and passes; `DIAMOND_EXIF_REQUIRE_ORACLE=1` fails);
  - `just oracle-record` re-run with an empty `git diff` after;
  - **every divergence between ExifTool and `exif-core` reported** in the Handback with the exact command, ExifTool version and measured output, plus entries in `KNOWN_DIVERGENCES` and `guidance/questions.yaml`. Do not silence, loosen a tolerance, or change library code to make one go away. If a divergence shows a library bug, stop and report it.
- Name every deviation (renamed, folded or missing tests) in the Handback.
- Commit on your worktree branch. **Do not push and do not open a PR.** The spec is already at `cycle: build`; do not advance cycles. First run `git merge --ff-only claude/rust-exif-metadata-6351d5` if your worktree was cut from an older commit (you need `b6d1d19` or later).
- Do not modify irradiance, `scripts/`, `decisions/`, the 29 fuzz seeds, or library code under `src/`. No dependencies or dev-dependencies.
- You cannot see your own token count; set `tokens_total: null` with a note, the orchestrator transcribes the metered figure.

## Out of Scope

- SPEC-008 (byte-mutation helpers, mutated-copy red-proof), CI wiring, JPEG/PNG/WebP, ExifTool write mode
- Fixing library bugs a divergence reveals; pushing, PRs, cycle advancement

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

- **Branch / PR:** branch `worktree-agent-ae70592c6152a9b0f`, build commit `9cba911` (fast-forwarded from `claude/rust-exif-metadata-6351d5` at `2f8ad6a` first); this handback is a second commit on top. Not pushed, no PR.
- **Completed at:** 2026-11-05
- **All acceptance criteria met?** yes, with one measured divergence reported rather than silenced (below) and the deviations listed below. Per criterion:
  - `parse_fields` pure, never sees an exit status, count mismatch and empty output are `ToolError::Parse`: met (`tools::tests::*`).
  - Live parity on every hand-built fixture: met. Live run: 9 fixtures, 47 ExifTool values compared, 0 mismatches (both byte orders; four GPS ref combinations; ascii-edges, gps-corrupt-exif-ok, ifd1-corrupt).
  - Seed sweep: met with one `KNOWN_DIVERGENCES` entry. Live run: 29 seeds, 28 run live (26 parse Ok; 2 fatal, where ExifTool printed all `-`), 25 ExifTool values compared, 0 mismatches; 1 seed (`two-hop-cycle`) deliberately NOT run (below).
  - Divergences reported: yes, 1 (below), with a `KNOWN_DIVERGENCES` entry and a `guidance/questions.yaml` entry. One more finding that is not a library divergence is recorded as an answered question (GPS selector).
  - Shown red: met. Replay control clean; perturbing one value over every committed record names exactly that selector (Ascii, Uint, Rational, SRational, GpsCoord against real records; Sint and Float have no baseline tag so they are perturbed on synthetic readings, in `oracle::tests::perturbing_one_value_yields_exactly_one_mismatch_per_norm` and `oracle_names_a_perturbed_sint_and_float`). Live demo `live_demo_a_mutated_payload_is_caught`: ours read from the original bytes, `D` of the Make payload changed to `X` in a temp copy, live output `exif-basic-le with Make[0] planted 'X' -> [Mismatch { selector: "IFD0:Make", ours: Some("DiamondCam"), theirs: Some("XiamondCam") }]`. Nothing from the plant is committed.
  - Missing and extra field: met (`diff_reports_a_missing_field`, `diff_reports_an_extra_field`, `comparator_catches_a_missing_and_an_extra_field_on_a_real_record`).
  - Loud skips: met and demonstrated (below).
  - Committed records equal `just oracle-record` output: met. After committing, `just oracle-record` re-ran (38 records) and `git status` / `git diff` were empty (0 diff lines).
  - No ExifTool source read; irradiance read only; `[dependencies]` and `[dev-dependencies]` empty: met.
  - Gates I ran, all green: `just lint` (fmt + clippy -D warnings), `just lint-fuzz`, `cargo test` on stable and `~/.cargo/bin/cargo +1.90.0 test` (22 tests in `metadata_oracle`; other suites unchanged), `cargo rustc --lib -- -F unsafe_code`, `just deny`, `just deny-fuzz`, `just validate`, `just decisions-audit` (0 errors, 4 pre-existing scope warnings), `just cost-audit`. `src/` is unchanged.
- **Skip modes demonstrated (built test binary run directly with `PATH=/nonexistent`):**
  - Hidden, not required: `live_demo_a_mutated_payload_is_caught ... SKIP live_demo_a_mutated_payload_is_caught -- exiftool not on PATH` then `ok`, the same for `live_parity_on_hand_built_fixtures` and `live_parity_on_the_committed_seeds`; replay tests still ran; `test result: ok. 22 passed`.
  - Hidden with `DIAMOND_EXIF_REQUIRE_ORACLE=1`: the three live tests FAILED with `exiftool is not on PATH and DIAMOND_EXIF_REQUIRE_ORACLE=1 makes that a failure`; `test result: FAILED. 0 passed; 3 failed`.
  - `live_skip_is_loud_and_required_mode_fails` automates both modes by re-running the test binary as a child with a scrubbed `PATH`.
- **Divergences found (1):**
  - `two-hop-cycle` (seed `fuzz/seeds/tiff_ifd/two-hop-cycle`: IFD0 -> SubIFDs -> Sub IFD at 38 -> SubIFDs -> offset 8). Command: `exiftool -T -n -s3 -IFD0:ImageWidth -IFD0:ImageHeight -IFD0:Orientation fuzz/seeds/tiff_ifd/two-hop-cycle`, ExifTool 13.55 (Homebrew, macOS). Measured: stdout empty; resident memory about 8.9 GB at 5 s (8.3 GB at 10 s); killed by SIGKILL (exit 137) after 53.4 s; stderr `Deep recursion on subroutine "Image::ExifTool::ProcessDirectory"` and `"Image::ExifTool::Exif::ProcessExif"`. exif-core: `Container::parse` is Ok, one `CyclicIfd { offset: 8 }` issue, `IFD0:ImageWidth = 1`. Right side per TIFF 6.0 section 2: exif-core (an IFD chain ends at a zero offset; a cycle is malformed input a reader must survive). This corrects the spec's measured fact "exit status is 0 on every one": it is 137 here. Not a library bug.
  - Handling: because the tool consumes about 9 GB, `oracle::not_run_divergence` makes the recorder and the live sweep skip it with a loud `NOT RUN` line; the committed record has an `# outcome:` header and the tests assert our side still behaves as the entry states (Ok, issue present, width 1) so the entry cannot go stale. Question `exiftool-two-hop-cycle-does-not-return` (open).
  - Not a divergence but a spec-example correction: `-n GPS:GPSLatitude` is unsigned (the ref is `GPS:GPSLatitudeRef`); `Composite:GPSLatitude` / `Composite:GPSLongitude` are signed. Measured on `exif-gps-sw-be`: `-n -GPS:GPSLatitude` printed `37.4252083333333`, `-n -Composite:GPSLatitude` printed `-37.4252083333333`. The oracle uses the Composite selectors. Question `exiftool-gps-selector-signed-composite` (answered).
  - Other measurements, no disagreement: `ExposureCompensation` for -1/3 prints `-0.3333333333` (10 significant digits), inside the 1e-9 relative tolerance; ExifTool's names differ from the spec names (`ImageHeight` for ImageLength, `ModifyDate` for DateTime, `ISO` for ISOSpeedRatings, `ExposureCompensation` for ExposureBiasValue), noted in the selector table; `ascii-edges` Model with an embedded NUL reads `ab` on both sides and the empty Artist reads as an empty string on both sides.

### Cost self-report

- **Tokens (total):** 215,404 (recorded by the orchestrator from the sub-agent metered result).
- **Estimated USD:** 1.42
- **Duration (minutes):** ~17 (metered wall-clock)
- **Source of the number:** harness report (subagent_tokens)

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:**
  - GPS selectors are `Composite:GPSLatitude` / `Composite:GPSLongitude`, not `GPS:...` (measured above); the Norm and sign rule are as specified.
  - `two-hop-cycle` is not run live and its record has no output line (divergence above), so the seed sweep runs 28 of 29 live. This is a reported `KNOWN_DIVERGENCES` entry, not a loosened comparison.
  - Tests: no renames or missing normative tests. Added beyond the list: `corrupt_fixtures_are_corrupt_in_the_designed_way`, `oracle_names_a_perturbed_sint_and_float`, `comparator_catches_a_missing_and_an_extra_field_on_a_real_record`, `diff_skips_a_zero_denominator_reading`, `perturbing_one_value_yields_exactly_one_mismatch_per_norm`. The per-`Norm` requirement is one parameterised test (the spec allowed this) plus the real-record sweep. The `parse_fields`, `run_tool`, `diff` and GPS unit tests live in `tests/support/*` as `#[cfg(test)]` modules and run inside the `metadata_oracle` test crate (names `tools::tests::...`, `oracle::tests::...`).
  - `ifd1-corrupt` is compared on IFD0 selectors only (the selector table has no IFD1 entries); `corrupt_fixtures_are_corrupt_in_the_designed_way` asserts the walk really records an issue for it and for `gps-corrupt-exif-ok`.
- **Follow-up work identified:**
  - SPEC-008 (as planned). Add a time and memory limit to `run_tool` when it needs more ExifTool runs: the runaway above shows an unbounded call can take the host down. Not added here, to keep `run_tool` the simple choke point the spec describes.
  - CI: pin ExifTool and set `DIAMOND_EXIF_REQUIRE_ORACLE=1` (existing question `rust-ci-and-fixtures`).
  - Decide whether to report the `two-hop-cycle` runaway upstream (open question).

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — The spec's measured fact "exit status 0 on every seed" is false for `two-hop-cycle` (killed, exit 137, about 9 GB), and nothing warned that running the 29-seed sweep live could exhaust host memory. The GPS example selector was also the unsigned one. Both surfaced only by running the tool.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — A resource limit for tool runs. `exiftool-never-linked` covers licence but not blast radius; the spec could have said "no unbounded tool call: run each seed under a time and memory limit or record it as not run".

3. **If you did this task again, what would you do differently?**
   — Probe each seed with a short time-boxed single run before wiring the full sweep, and sample memory on the first slow one, instead of finding the runaway through a 60 s test run.

### Follow-up round (after the HANDOFF-014 punch list)

- **P1:** the `tests/support/tools.rs` header no longer says ExifTool exits 0 every time; it states the verifier's measurement (ExifTool 13.55, time-limited, `-T -n -s3 -IFD0:ImageWidth -IFD0:ImageHeight -IFD0:Orientation <seed>`: `ifd0-offset-zero` and `truncated-header` exit 1, `two-hop-cycle` does not return, 26 exit 0). `exiftool-two-hop-cycle-does-not-return` now lists the two exit-1 seeds. No other "always/never/every time" claim about ExifTool remains in tests/support, `tests/metadata_oracle.rs`, AGENTS.md or the docs I touched.
- **F2:** the `KNOWN_DIVERGENCES` right-side sentence and the question now cite the TIFF Supplement / Tech Note 1 for SubIFDs. The citation was not part of the record header, so `just oracle-record` produced an empty diff (the recorder skips `two-hop-cycle`; ExifTool was not run on it).
- **F3:** questions `exiftool-ascii-count-past-eof-ifd0-tags` and `exiftool-ascii-shapes-invalid-utf8-text` added, with exact commands and measured output. (`ascii-shapes` Orientation is a BYTE-typed tag, not ASCII; the spec's remark was inaccurate.)
- **F5:** `diff_applies_the_float_tolerance` gained a must-fail case at 1e-8 relative. Plant: `REL_TOL = 5e-7` turned `oracle::tests::diff_applies_the_float_tolerance` red (21 passed, 1 failed); reverted, green again.
- **F6:** a GPS coordinate with a count other than 3 now reads `None` (undecodable); only a real zero denominator reads as skipped.
- **F7:** the required-mode test now also asserts the failure message names `DIAMOND_EXIF_REQUIRE_ORACLE=1`; AGENTS.md's SKIP line matches the code (em dash).
- **F4 (known gap, SPEC-008 candidate):** signed integers and floats are red-proofed only on synthetic values, so an SSHORT-read-as-unsigned plant would survive the oracle tests.
