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
  id: HANDOFF-014
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
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
  tokens_total: null               # no meter visible to the verifier sub-agent; orchestrator transcribes the metered figure
  estimated_usd: null
  duration_minutes: 35
  branch: worktree-agent-addb5216f35d02366
  pr: null
  completed_at: 2026-11-05
  notes: "verdict PUNCH LIST at 7ce26d2; tokens_total null because the sub-agent has no token meter, orchestrator transcribes the metered figure"
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-014: Verify the ExifTool read-parity oracle

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-005` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-005-oracle-discipline.md`
- **Build handoff and Handback:** `HANDOFF-013-build-oracle-discipline.md` (claims to verify, not to trust)
- `guidance/constraints.yaml` (`oracle-must-be-shown-red`, `exiftool-never-linked`), `docs/porting-brief-diamond-exif.md` §2.2-2.3, `guidance/toolchain-brief.md`, `guidance/questions.yaml` (new entries from the builder)
- Code under review: `git diff 2f8ad6a..HEAD` (build commit `9cba911`, handback `47d8ef9`)
- ExifTool 13.55 is at `/opt/homebrew/bin/exiftool`. **ExifTool's source must never be read.** Irradiance is read-only.

## Expected Deliverables

Review cold; output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

**Safety first:** the builder reports ExifTool 13.55 does not return on `fuzz/seeds/tiff_ifd/two-hop-cycle` (about 8.9 GB resident in 5 s, SIGKILLed by the OS). Never run ExifTool on that file without a hard wall-clock limit (for example `perl -e 'alarm 4; exec @ARGV' exiftool ...`). The harness deliberately does not run it; confirm that.

Verify independently:
- Re-run every gate in the spec; also run the full test suite with ExifTool present (the live tests) and with `PATH` scrubbed (loud SKIP, replay tests still run) and with `DIAMOND_EXIF_REQUIRE_ORACLE=1` and ExifTool hidden (live tests must fail).
- Confirm `parse_fields` never touches an exit status and the single `run_tool` choke point is the only place ExifTool is spawned (grep).
- **Red-proofs are the point.** Redo them yourself: (a) plant a one-field perturbation of our reading per `Norm` variant and confirm exactly one mismatch naming it; (b) plant library-side faults that the oracle must catch on the live fixtures, in a scratch copy of the tree, for example swap the byte order of one multi-byte read, make `ascii` drop the last byte, negate a GPS ref sign in `read_ours`, widen the float tolerance to 1e-2 and see whether any test notices; (c) delete a committed record's value and see the replay fail. For each plant, name which test went red; report any plant that survives. Revert everything.
- Judge the normalisation table and tolerance rules: are text/int compares exact? is `1e-9` relative defensible for 15-digit ExifTool decimals? could a systematic error hide inside the tolerance (e.g. GPS minutes vs seconds scaling error below 1e-9)? Try to construct one.
- Check the spec correction the builder reports (GPS unsigned vs `Composite:` selectors) by measurement; check `KNOWN_DIVERGENCES` entries carry command, version, measured output, which side is right and why from the format spec; confirm the two new questions in `guidance/questions.yaml` are accurate.
- Check the seed sweep rules against what the spec asked; look at the 29 seeds yourself for any where ExifTool and exif-core disagree that the harness missed or excluded (with the alarm guard for any suspect seed). Confirm `just oracle-record` reproduces the committed records exactly (empty `git diff`), except you must not run it unguarded on `two-hop-cycle` (the recorder skips it; confirm).
- Note: the orchestrator's earlier measurement in the spec said ExifTool exits 0 on every seed. That was measured through a pipeline and is **wrong**; judge the builder's correction and the comments in the code for any remaining over-generalised claim ("always", "never") about ExifTool.
- Verify `src/`, `scripts/`, `decisions/`, irradiance and the 29 seeds are untouched; `[dependencies]` and `[dev-dependencies]` empty.
- Judge the builder's deviations (Composite selectors, two-hop-cycle not run live, five extra tests, unit tests inside `tests/support/*`, `ifd1-corrupt` on IFD0 selectors only).
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes; SPEC-008 work (including adding a time/memory limit to `run_tool`, which the builder suggests for SPEC-008); CI wiring; ExifTool write mode

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

- **Branch / PR:** branch `worktree-agent-addb5216f35d02366` (fast-forwarded to `7ce26d2` from `claude/rust-exif-metadata-6351d5`); this handback is one commit on top. Not pushed, no PR.
- **Completed at:** 2026-11-05
- **All acceptance criteria met?** Functionally yes. One documentation item breaks the spec's `measurement-over-generalised` rule (P1 below).
- **For `verify`:** ⚠ PUNCH LIST at `7ce26d2`. P1 is required; the other findings are recommendations.

**Gates re-run (all green at 7ce26d2):** `cargo fmt --all -- --check`; `just lint` (Homebrew clippy) and `cargo +1.90.0 clippy --all-targets -- -D warnings`; `just lint-fuzz`; `cargo test` on stable (lib 73, crate_policy 4, fuzz_layout 4, metadata_oracle 22) and `~/.cargo/bin/cargo +1.90.0 test` (same counts); `cargo rustc --lib -- -F unsafe_code`; `just deny`, `just deny-fuzz` (licenses ok); `just validate`; `just decisions-audit` (0 errors, 4 pre-existing scope warnings); `just cost-audit`. `git diff 2f8ad6a..HEAD -- src scripts decisions fuzz Cargo.toml Cargo.lock` is empty. `[dependencies]` is empty and there is no `[dev-dependencies]` table. Irradiance `git status` is clean.

**Per criterion:**
- *parse_fields pure / Parse on count mismatch:* met. `parse_fields(&[&str], &str)` sees only text. `Command::new` appears twice in the tree: `tests/support/tools.rs:51` (`run_tool`, the only ExifTool spawn) and `tests/metadata_oracle.rs:519` (re-runs the test binary itself for the skip test). No code reads `.status`.
- *Live parity on fixtures:* met, re-run live: `9 fixtures, 47 ExifTool values compared`, 0 mismatches.
- *Seed sweep:* met under the stated three-selector rule: `29 seeds, 28 run live, 1 not run; 26 parse Ok, 2 fatal; 25 values compared`. My own guarded probe of every seed (`perl -e 'alarm N; exec @ARGV' exiftool -T -n -s3 -IFD0:ImageWidth -IFD0:ImageHeight -IFD0:Orientation`, N=10, N=2 for two-hop-cycle) gave stdout identical to every committed record. two-hop-cycle did not return within 2 s (SIGALRM, exit 142, empty stdout).
- *Divergences reported:* the one entry carries the command, the version, the measured output, the right side and a question. See F3 for a disagreement outside the seed selectors.
- *Shown red:* met (plants below).
- *Missing / extra field:* met (plant C3 turns all three tests red).
- *Loud skips:* redone on the built binary with `PATH=/nonexistent`: three `SKIP <test> — exiftool not on PATH` lines, `22 passed`. With `DIAMOND_EXIF_REQUIRE_ORACLE=1` the three live tests panic with `... DIAMOND_EXIF_REQUIRE_ORACLE=1 makes that a failure`: `FAILED. 19 passed; 3 failed`.
- *Records reproduce:* `just oracle-record` (under a 180 s alarm) printed `NOT RUN on two-hop-cycle` and `wrote 38 records`; `git status` and `git diff` were empty afterwards. The recorder checks `not_run_divergence` before it spawns anything, so two-hop-cycle is never run.
- *No ExifTool source read:* I read none. ExifTool was only run, and every call was time-limited.

**Plants.** All plants ran in a scratch copy of the tree in the scratchpad, with a separate target dir. The only edit to the worktree was one temporary example file, deleted before any commit; `git status` was clean at the end. "red" lists the `metadata_oracle` tests that failed.
- Library:
  - L1, SHORT byte order reversed in `uints`. Red: live_parity_on_hand_built_fixtures, live_parity_on_the_committed_seeds, live_demo, replay_is_clean, oracle_names_the_one_field, comparator_catches_missing_and_extra.
  - L2, LONG byte order reversed. Red: both live parity tests, live_demo, replay, perturb, missing/extra, corrupt_fixtures, gps_coordinate_sign.
  - L3, `ascii` drops the last payload byte. Red: live_parity_on_hand_built_fixtures, replay, perturb. Only the unterminated `ascii-edges` Make catches it; a NUL-terminated value hides this bug.
  - L4, `urationals` numerator and denominator swapped. Red: live fixtures, live_demo, replay, perturb, missing/extra, gps sign.
  - L5, `srationals` numerator through `wrapping_abs`. Red: live fixtures, live_demo, replay, perturb, missing/extra.
  - L7, GPS pointer not followed. Red: live fixtures, replay, perturb, corrupt_fixtures, gps sign.
  - L8, Exif pointer not followed. Red: live fixtures, live_demo, replay, perturb, missing/extra.
  - **Survived:** L6, SSHORT read as unsigned in `sints`. Every oracle test stayed green; only a lib unit test went red. No live or recorded selector is Sint or Float (`signed-and-float` gives `- - -` on the seed selectors), so ExifTool gives no evidence on signed-integer or float decoding (F4).
- read_ours:
  - R1, `W` treated as positive. Red: live fixtures, replay, perturb, gps sign.
  - R2, GPS value computed through f32. Red: the same four.
  - R5, rational quotient through f32. Red: live fixtures, live_demo, replay, perturb, missing/extra.
  - **Survived by design (inside the tolerance):** R3, minutes scaled by (1+1e-10); R4, seconds divided by 3600.0001.
- Comparator:
  - C1, REL_TOL 1e-2. Red: diff_applies_the_float_tolerance, oracle_names_the_one_field (longitude 122.1+1 is within 1 %).
  - C2, Ascii always agrees. Red: perturbing_one_value_per_norm, oracle_names_the_one_field, live_demo.
  - C3, None vs Some agrees. Red: diff_reports_a_missing_field, diff_reports_an_extra_field, comparator_catches_missing_and_extra.
  - C4, Uint always agrees. Red: per_norm, oracle_names_the_one_field.
  - C6, ABS_TOL 1e-3, and C7, ABS_TOL 0.5. Red: diff_applies_the_float_tolerance.
  - **Survived:** C5, REL_TOL widened to 5e-7 (500x). Every test stayed green (F5).
  - T1, `-` no longer mapped to None. Red: parse_fields_maps_dash, both live parity tests, live_demo, replay, perturb, missing/extra.
  - T2, field-count check removed. Red: parse_fields_rejects_a_field_count_mismatch.
- Records:
  - Rec1, FNumber value deleted (`-`) in exif-basic-le. Red: replay, perturb, missing/extra.
  - Rec2, GPS longitude sign flipped in exif-gps-nw-le. Red: replay, perturb.
  - Rec3, `4 2 6` changed to `4 - 6` in the seed record valid-le. Red: replay.
  - Rec4, one field dropped in ascii-edges. Red: replay, perturb (Parse error).
  - Rec5, Make text changed in exif-basic-be. Red: replay, perturb.

**Tolerance rules (judgement):** text and integers compare exactly (confirmed in `texts_agree`). A relative tolerance of `1e-9` is defensible. The measured ExposureCompensation prints 10 significant digits (`-0.3333333333`, an error of 1e-10 relative; the worst case at 10 digits is 5e-10), so 1e-9 is about the tightest round number that holds. For the 15-digit GPS composite it is about 10^5 looser than needed, but the window is small: 1e-9 of 37.4 degrees is 3.7e-8 degrees, about 4 mm. Only artificial errors fit inside it (R3, R4). Every realistic error I planted was caught: f32 precision, sign, swapped numerator and denominator, minutes and seconds confused.

**Spec correction checked:** I measured the GPS selectors on `exif-gps-sw-be`. `-GPS:GPSLatitude` printed `37.4252083333333` with `GPS:GPSLatitudeRef` = `S`; `-Composite:GPSLatitude` printed `-37.4252083333333`; `-Composite:GPSLongitude` printed `-122.096916666667`. The builder's correction and the question `exiftool-gps-selector-signed-composite` are accurate.

**Findings**
- **P1 / F1 (medium, required):** `tests/support/tools.rs:14-16` still says ExifTool's exit status was "0 every time" over the 29 seeds. That is false:
  - two-hop-cycle does not return (137 per the builder; SIGALRM in my 2 s probe);
  - `ifd0-offset-zero` and `truncated-header` exit **1** (my guarded probe, ExifTool 13.55, the seed command).

  The spec rule forbids exactly this kind of over-generalised claim. The questions.yaml note on `exiftool-two-hop-cycle-does-not-return` ("not for this seed (137)") is incomplete in the same way. Fix: state the measured exit statuses per seed, or drop the claim, in both places. No code depends on it, because the exit status is never read.
- **F2 (low):** the record header and `KNOWN_DIVERGENCES.right_side` cite "TIFF 6.0 section 2" for SubIFD chains. SubIFDs are defined in the TIFF Supplement / Tech Note 1, not in TIFF 6.0 section 2. The principle still holds: a cycle is malformed input that a reader must survive.
- **F3 (low; for a question, not this spec):** I ran the full selector table over the seeds. It found two disagreements outside the three seed selectors, so outside the spec rule:
  - (a) `ascii-count-past-eof`: ExifTool 13.55 warns `Error reading value for IFD0 entry 0, ID 0x010f Make` and reports **no** IFD0 tags. exif-core reads `IFD0:ModifyDate = "ok"` (tolerant walk per DEC-001). TIFF 6.0 addresses each entry independently, so exif-core is defensible.
  - (b) `ascii-shapes` Copyright, which is invalid UTF-8: ours reads as lossy `C\u{FFFD}...(` and ExifTool's as `C\xe9\xff\xc3(..`. How non-UTF-8 text is rendered is outside the selector table.

  Suggest a questions.yaml entry for (a), and recording it as a divergence when the seed sweep widens.
- **F4 (low):** `Norm::Sint` and `Norm::Float` are red-proofed only on synthetic readings. No live or recorded selector uses them, so plant L6 survives the oracle. This complies with the spec (the selector table did not require them). It is a gap to close in SPEC-008 or later if ExifTool has a selector for an SSHORT, SLONG, FLOAT or DOUBLE tag.
- **F5 (low):** `diff_applies_the_float_tolerance` only checks that REL_TOL is below 1e-6, so REL_TOL could widen 1000x unnoticed (C5 survived). Add a must-fail case near 1e-8.
- **F6 (low):** `read_one` returns `SKIPPED` for a GPS coordinate whose count is not 3, and `diff` then prints "zero denominator, not compared". The spec allows skipping zero denominators only. The coordinate is mislabelled and goes uncompared without a real warning.
- **F7 (info):**
  - `live_skip_is_loud_and_required_mode_fails` checks only that required mode fails, not why: it does not look for the `DIAMOND_EXIF_REQUIRE_ORACLE=1 makes that a failure` text.
  - AGENTS.md shows the SKIP line with `--` where the code prints an em dash.
  - The spec's context line about `ascii-shapes` ("a tag we hold as ASCII") is inaccurate: that seed's Orientation is BYTE.

**Deviations judged:**
- Composite GPS selectors: correct and measured.
- two-hop-cycle not run live: the right call for safety, and `check_not_run_seed` keeps the entry from going stale.
- Five extra tests: all useful.
- Unit tests in `tests/support/*`: fine; they run in the metadata_oracle crate.
- `ifd1-corrupt` compared on IFD0 only: acceptable, since there are no IFD1 selectors. ExifTool also warns `Bad IFD1 directory` and agrees on IFD0.

### Cost self-report

- **Tokens (total):** null. The verifier sub-agent has no token meter; the orchestrator transcribes the metered figure.
- **Estimated USD:** null
- **Duration (minutes):** ~35 (estimate)
- **Source of the number:** none available (the orchestrator transcribes the harness figure)

### Drift and new artifacts

- **New decisions emitted:** none
- **Deviations from spec:** none by the verifier. Plants ran in a scratch copy rather than the worktree, so the tree stayed clean.
- **Follow-up work identified:**
  - The P1 fix (the tools.rs comment and the questions.yaml note on exit statuses).
  - A questions.yaml entry for the `ascii-count-past-eof` IFD0 disagreement (F3a).
  - SPEC-008: a live Sint/Float selector if one exists (F4), a tighter tolerance test (F5), and the time and memory limit on `run_tool` the builder proposed.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Nothing in the handoff; the two-hop-cycle safety warning was its most useful line. The host's worktree guard refused several compound shell commands, so probes had to go through small scripts.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — A rule that verify re-measures every measured ExifTool claim in the code. The build had already found the exit-status claim false for one seed, and the comment still survived.

3. **If you did this task again, what would you do differently?**
   — Run the guarded per-seed probe (exit status plus the full selector table) first. In one pass it found P1 and F3, before any plant work.
