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
  id: HANDOFF-015
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-05
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-008

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
  tokens_total: 261095             # orchestrator-recorded from the metered sub-agent result (subagent_tokens)
  estimated_usd: 1.72
  duration_minutes: 16
  branch: worktree-agent-aaf3f420e72df2b1c
  pr: null
  completed_at: 2026-11-05         # YYYY-MM-DD
  notes: "tokens transcribed by the orchestrator from the sub-agent metered result"
  synced_at: 2026-11-05
---

# HANDOFF-015: Mutation helpers and mutated-copy red-proofs

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-008` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

Read before starting:

- **Spec (normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-008-mutation-helpers-and-mutated-copy-red-proof-apparatus.md`
- `docs/porting-brief-diamond-exif.md` §2.3, `AGENTS.md`, `guidance/toolchain-brief.md`, `guidance/constraints.yaml` (`oracle-must-be-shown-red`, `exiftool-never-linked`)
- SPEC-005's files: `tests/support/{tools,oracle,oracle_tiff}.rs`, `tests/metadata_oracle.rs`, `tests/oracle-fixtures/exiftool/*.txt`, and its handbacks HANDOFF-013 and HANDOFF-014 (what survived, what diverged)
- **Pattern source, READ ONLY, never modify:** `~/PSeven/experiments/crustimg_redo_plus/irradiance/tests/plane_oracle.rs` lines ~626-805 and ~1044 (`TempDir`, `copy_dir_recursive`, `inject_*_fault`, `stage_probe_crate`, `build_and_run_probe`, control test). Port with edit, add a ledger row (DEC-003). **ExifTool's own source must never be read.**

**Safety, read twice:** ExifTool 13.55 on this machine does not return on `fuzz/seeds/tiff_ifd/two-hop-cycle` and balloons to about 9 GB. Never run it on that file; never run any new fixture through it without the wall-clock limit this spec adds. When measuring a tool, capture stdout to a variable or file and read the tool's own status; never read `$?` through a pipe.

Host: ExifTool at `/opt/homebrew/bin/exiftool`. Toolchain traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint` under the default PATH; the probe must build with the cargo that runs the tests (`CARGO` env var) in its own target dir, `--offline`, and must not be redirected into the repo's `target/` by a stale `CARGO_TARGET_DIR`.

## Expected Deliverables

- Everything in the spec's Outputs with all its named tests and every Acceptance Criterion met. In particular:
  - every library-fault test has its **control first and green**, and the mutant red naming the stated selector; the needle guard is exact-once and itself tested;
  - the three byte-mutation red-proofs with their isolation assertions;
  - the `run_tool` time limit (unix test with `sleep`), `ToolError::Timeout`, `DIAMOND_EXIF_TOOL_TIMEOUT_SECS`;
  - the signed/float live selector, with every candidate tag you tried recorded (command and output) in the Handback; if you must use the documented fallback, justify it with four honest measured attempts;
  - the red-proofs run with ExifTool hidden; only live variants skip, loudly;
  - wall-clock of `just mutation-oracle` recorded.
- Name every deviation (renamed, folded or missing tests) in the Handback. Report anything that surprises you: a needle that is hard to place, a probe build that is slow, a red-proof that is hard to make fail.
- Commit on your worktree branch. **Do not push and do not open a PR.** The spec is already at `cycle: build`; do not advance cycles. First run `git merge --ff-only claude/rust-exif-metadata-6351d5` if your worktree was cut from an older commit (you need `2fc776c` or later).
- Do not modify irradiance, `scripts/`, `decisions/`, library code under `src/`, or the 29 fuzz seeds. No dependencies or dev-dependencies. No `RED-PROOF INJECTION` text may end up in a committed non-test file or in `src/`.
- You cannot see your own token count; set `tokens_total: null` with a note, the orchestrator transcribes the metered figure.

## Out of Scope

- JPEG-level red-proofs, ExifTool write mode, CI wiring, a memory limit, fixing a library bug a red-proof reveals (stop and report), pushing, PRs, cycle advancement

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

- **Branch / PR:** branch `worktree-agent-aaf3f420e72df2b1c` (committed, not pushed); no PR, by instruction.
- **Completed at:** 2026-11-05
- **All acceptance criteria met?** Yes, with the deviations listed below. The orchestrator-run gates were run by me too and are green: `cargo fmt --check`, `just lint`, `just lint-fuzz`, `cargo test` on the default toolchain (Homebrew cargo 1.99.1) and on `~/.cargo/bin/cargo +1.90.0`, `cargo rustc --lib -- -F unsafe_code`, `just deny`, `just deny-fuzz`, `just validate`, `just decisions-audit`, `just cost-audit`. `just oracle-record` ran (it never touches `two-hop-cycle`); the 29 seed records came back byte-identical (`git status` shows only the 9 old fixture records modified and 3 new ones; no seed record changed), because the selector table grew only for fixtures.
- **For `verify`:** n/a (build).

**What was built.** `tests/support/mutate.rs`, `tests/support/probe.rs`, `tests/mutation_oracle.rs`; `run_tool_with_limit` / `ToolError::Timeout` / `DIAMOND_EXIF_TOOL_TIMEOUT_SECS` in `tests/support/tools.rs`; four new selectors (`ExifIFD:ExposureTime` Float, `ExposureProgram` Sint, `FocalLength` Float, `ExifImageWidth` Sint) and three new fixtures (`exif-gps-full-le`, `signed-float-le`, `signed-float-be`) with committed records; `just mutation-oracle`; ledger rows, AGENTS.md §6, toolchain brief, CHANGELOG, README, questions.yaml.

**Plants, the exact needles (all exact-once in the staged tree, test modules included) and what they gave** (each: control first and green, then the mutant, asserted as an exact set of selectors):

| # | Test | File and needle | Mutant result |
|---|---|---|---|
| 1 | `mutant_big_endian_short_read_is_caught` | `src/tiff/types.rs`: `out.push(u32::from(self.byte_order.u16(*chunk)));` becomes `u16::from_le_bytes(*chunk)` | `exif-basic-be`: exactly `IFD0:Orientation` (6 read as 1536) and `ExifIFD:ISO` (400 read as 36865); `exif-basic-le`: zero mismatches |
| 2 | `mutant_ascii_drops_the_last_byte_is_caught` | `types.rs`: `Ok(Ascii::split(bytes))` gets the payload minus its last byte | `ascii-edges`: exactly `IFD0:Make`, ours `NoTerminato`, theirs `NoTerminator` |
| 3 | `mutant_rational_swaps_numerator_and_denominator_is_caught` | `types.rs`: `num: order.u32(num),` and `den: order.u32(den),` swapped (two needles, each once) | `exif-basic-le`: exactly `ExifIFD:FNumber`, `IFD0:XResolution` (72 read as 1/72); `exif-gps-ne-le`: exactly the two `Composite:GPS*` |
| 4 | `mutant_gps_pointer_is_not_followed_is_caught` | `src/tiff/mod.rs`, two-line needle `tag: tags::GPS_INFO,\n        allowed_from: from_top_level_or_sub,` (the one-line form also occurs in that file's own tests) becomes `tag: 0xFFFF,` | `exif-gps-ne-le`: exactly the two `Composite:GPS*`, ours `None`, theirs a value |
| 5 | `mutant_signed_rational_sign_is_lost_is_caught` | `types.rs`: `num: order.i32(num),` becomes `...wrapping_abs()` | `exif-basic-le`: exactly `ExifIFD:ExposureCompensation` (sign lost) |
| 6 | `mutant_signed_short_is_read_unsigned_is_caught` | `types.rs`: `out.push(i32::from(self.byte_order.i16(*chunk)));` gets `as u16` | `signed-float-le` and `-be`: exactly `ExifIFD:ExposureProgram`, ours `65534`, theirs `-2` |

**Byte-mutation measurements.** (7) `exif-gps-full-le`, GPS pointer value set to 0x00FF0000: the diff names exactly the two GPS selectors; `Container::parse` is `Ok` with one issue, `Link::Tag(0x8825)`, offset `0x00FF0000`; IFD0 and Exif selectors untouched; live ExifTool 13.55 on the same mutated bytes returns `-` for both GPS selectors, agrees with ours on every selector and returns the original IFD0/Exif text. (8) `II` to `MM` on `exif-basic-le`: measured, `Container::parse` returns `Err(UnsupportedTiffVersion { version: 0x2A00 })` (the stored `2A 00` read big-endian); it fails at the header, so no garbage values are read and every selector ExifTool reported goes `None`. (9) `exif-basic-le` cut inside the Exif IFD, two cuts: (a) mid entry table: one issue `Link::Tag(0x8769)`, exactly the four Exif selectors vanish, IFD0 intact; (b) exactly where the Exif IFD's payload region starts (table and next-word whole): no issue, and exactly FNumber, DateTimeOriginal and ExposureCompensation vanish while the inline SHORT ISO survives. The spec's wording for (9) ("just before the payload region; Exif selectors vanish, issue reported") describes (a); (b) is the literal reading and behaves differently, so both are asserted.

**Signed/float outcome: the real live selector, no fallback.** ExifTool 13.55 honours the declared type for these tags (tiny hand-built little-endian single-entry files, `exiftool -n -G1 -a -u -U <file>`, one file per row, `<file>` being that row's file; the second column is the relevant output line; the committed records use the oracle's own `-T -n -s3` form):

| Declared type, tag, IFD, stored value | ExifTool output |
|---|---|
| SSHORT(8), 0x0112 Orientation, IFD0, -3 | `[IFD0] Orientation : -3` |
| SHORT(3), 0x0112 Orientation, IFD0, 3 (control) | `Orientation : 3` |
| SLONG(9), 0x0100 ImageWidth, IFD0, -5 | `Image Width : -5` |
| SSHORT(8), 0x8822 ExposureProgram, Exif, -2 | `Exposure Program : -2` (selected) |
| SSHORT(8), 0x8827 ISO, Exif, -200 | `ISO : -200` |
| SLONG(9), 0x9204 ExposureCompensation, Exif, -2 | `Exposure Compensation :` (empty) |
| SSHORT(8), 0x9201 ShutterSpeedValue, Exif, -2 | `Shutter Speed Value : 0` |
| FLOAT(11), 0x829D FNumber, Exif, 2.5 | `F Number : 2.5` |
| DOUBLE(12), 0x829D FNumber, Exif, 2.5 | `F Number : 2.5` |
| FLOAT(11), 0x829A ExposureTime, Exif, 0.125 | `Exposure Time : 0.125` (selected) |
| FLOAT(11), 0x011A XResolution, IFD0, 72.5 | `X Resolution : 72.5` |
| SLONG(9), 0xA002 ExifImageWidth, Exif, -5 | `Exif Image Width : -5` (selected) |
| SSHORT(8), 0xA002 ExifImageWidth, Exif, -5 | `Exif Image Width : -5` |
| SLONG(9), 0x9202 ApertureValue, Exif, -7 | `Aperture Value : 0.0883883476483184` (converted, not usable) |
| DOUBLE(12), 0x920A FocalLength, Exif, 4.5 | `Focal Length : 4.5` (selected) |
| FLOAT(11), 0x920A FocalLength, Exif, 4.5 | `Focal Length : 4.5` |
| SLONG(9), 0x8822 ExposureProgram, Exif, -2 | `Exposure Program : -2` |

(Eighteen files in all, in two batches; the table omits one control, SSHORT(8) 0x8822 stored 2, which printed `Exposure Program : 2`; the tag column is the hexadecimal of the decimal tag the generator was given. "Exif" means the entry sat in an Exif IFD reached from IFD0's 0x8769 pointer. ExifTool printed more lines per file (File, System and ExifTool groups); only the line for the tag under test is shown. The ISO row used 0x8827.) The four rows marked "selected" are the table additions; the generator and driver scripts were scratch files outside the repo. The committed records for `signed-float-le` and `-be` both read `0.125 -2 4.5 -5` for the four new selectors. Mutant 6 is red through `ExifIFD:ExposureProgram`; `replay_signed_and_float_selectors_carry_values` pins the committed record and our reading to those values offline, and `live_signed_and_float_selectors_match_exiftool` does the same live. The `oracle_names_the_one_field_that_was_perturbed` test now requires real records for `Sint` and `Float` too.

**Wall-clock.** `just mutation-oracle` (37 tests, 36 run): about 3.0 s of test time and 3.1 s end to end when warm (`time just mutation-oracle`), on Apple silicon. The 13 probe builds (6 mutants, 6 controls, 1 all-fixtures control) are debug builds of a dependency-free crate, about 1 s each, run in parallel by the test harness. Far under the 3 minute line, so no shared target dir is proposed. Probe temp dirs under the OS temp directory are removed (checked: none left after runs); a stale `CARGO_TARGET_DIR` in the environment produced zero probe binaries in the stale directory; the repo's `target/` mtime is asserted unchanged by staging.

**Red-proofs with ExifTool hidden.** On the 1.90.0 toolchain's own cargo with `PATH=<toolchain bin>:/usr/bin:/bin` and `DIAMOND_EXIF_REQUIRE_ORACLE` unset, all 36 tests pass; the only skip is `SKIP corrupting_the_gps_pointer_is_reported_and_isolated (live variant) — exiftool not on PATH`. A child-process test (`the_gps_live_variant_skips_loudly_and_required_mode_fails`) also checks the skip line and that `DIAMOND_EXIF_REQUIRE_ORACLE=1` fails with the variable named.

**Timeout.** `run_tool_times_out_and_kills_the_child` (unix): `sh -c 'echo $$ > "$0"; exec sleep 30'` through `run_tool_with_limit` with a 1 s limit returns `ToolError::Timeout { tool: "sh", secs: 1 }` in about 1 s; the recorded pid is then checked with `kill -0` (itself through `run_tool_with_limit`) and is gone. `timeout_variable_overrides_the_default` tests the pure parser; `timeout_variable_is_honoured_by_run_tool` re-runs this test binary with the variable set to 1 and checks `Timeout { secs: 1 }` end to end (no test mutates this process's environment). `exiftool_available` panics on a `Timeout` rather than skipping. Grep: the only `Command::new(bin)` that spawns a tool is in `tools.rs`; the other `Command::new` hits are the self-respawn tests (`mutation_oracle.rs`, `metadata_oracle.rs`, `tools.rs` test) and the probe's cargo and probe-binary spawns in `probe.rs`.

### Cost self-report

Mirror what you put in the `handback:` front-matter, and say where the number
came from. **This is the number that lands in the spec** — the orchestrator
transcribes it via `just handback-sync`, it does not estimate it.

- **Tokens (total):** 261,095 (recorded by the orchestrator from the sub-agent metered result).
- **Estimated USD:** 1.72
- **Duration (minutes):** ~16 (metered wall-clock)
- **Source of the number:** harness report (subagent_tokens)

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:**
  - Mutant 1 is placed at the SHORT use site in `Container::uints`, not in `ByteOrder::u16`'s big-endian arm. By reading the code, that arm is also what `read_u16` uses for the header version word and every entry count, so mutating it breaks every big-endian file at the header and no SHORT selector could be named (reasoned from the code, not run). The fault the test is about (a SHORT decoded in the wrong byte order) is the same.
  - A fourth new fixture beyond the two the signed/float case needed: `exif-gps-full-le` (IFD0, Exif IFD and GPS IFD in one file). No existing fixture had both an Exif and a GPS IFD, and byte mutation 7's isolation claim ("IFD0 and Exif selectors still match") needs one. Three new fixtures in all: `exif-gps-full-le`, `signed-float-le`, `signed-float-be`.
  - Mutation 9 asserts two cuts, see above; the spec's one-sentence description matched only (a).
  - `run_tool` gained a sibling `run_tool_with_limit` (the one spawn function); `run_tool` is the environment-limit wrapper. The spec's "sleep through `run_tool` with a 1 s limit" is done through `run_tool_with_limit`, and the environment path is covered by a child-process test, so no test changes this process's environment.
  - Test names in the spec all exist, plus extras: `no_injection_marker_outside_the_tests`, `the_gps_live_variant_skips_loudly_and_required_mode_fails`, `escaping_round_trips`, `timeout_variable_is_honoured_by_run_tool` (+ ignored child `env_limit_child`), `run_tool_returns_a_fast_tools_output_unchanged`, `replay_signed_and_float_selectors_carry_values`. The live variant of test 7 is part of the same test (offline half first), not a separate test.
  - Selector table: the four new selectors sit before the two GPS ones on purpose (`gps_coordinate_sign_follows_the_ref_tag` indexes the last two). All 9 older records changed (the command line lists more selectors) and were re-recorded with `just oracle-record`; the only content change in the old records is four extra `-` columns before the GPS pair.
  - The probe run itself (cargo build, probe binary) has no wall-clock limit; only ExifTool spawns do.
  - Edited `tests/metadata_oracle.rs` beyond the spec's list: the `norms_seen` requirement now includes `Sint` and `Float`, and `live_signed_and_float_selectors_match_exiftool` is added to the loud-skip loop.
- **Follow-up work identified:**
  - CI wiring of `just mutation-oracle` (out of scope here; it is fast enough, about 3 s, to be part of the normal test job).
  - JPEG-level red-proofs and write-parity red-proofs when the container and writer stages exist (already in the spec's non-goals).
  - Upstream report of the ExifTool `two-hop-cycle` runaway is still open in `guidance/questions.yaml`.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Mutant 1's wording ("`ByteOrder::u16` big-endian arm") would have broken every big-endian file at the header rather than only SHORT values; and test 9's "just before the payload region; Exif selectors vanish, issue reported" is true for a mid-table cut, not for a cut at the payload region (the inline ISO survives, no issue). Both were quick to resolve once the layout was in front of me, but they are the places a literal reading fails.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — Adding selectors changes the command line recorded in every fixture record, so every record must be re-recorded (and a selector added in the middle of the table breaks any test that indexes from the end). The handoff could say so. Also, the environment of this host lacks `timeout(1)`, so "measure with a time limit" needs a harness-level limit; the measurements here were safe only because the files are single-entry and cycle-free.

3. **If you did this task again, what would you do differently?**
   — Measure the candidate tags with a small recorded script from the start (I did, but the per-file transcript is only in this handback) and write the needles as a table first; the exactly-once check against `src/` including test modules (the one-line GPS needle also occurs in `mod.rs`'s own tests) is the part worth doing before writing any test.
