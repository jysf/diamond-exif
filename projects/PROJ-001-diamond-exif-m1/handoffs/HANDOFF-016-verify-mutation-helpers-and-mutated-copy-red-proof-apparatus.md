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
  id: HANDOFF-016
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
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
  tokens_total: null               # REAL combined count — what cost-audit reads
  estimated_usd: null              # tokens_total × your rate, or your harness's number
  duration_minutes: 30
  branch: worktree-agent-a7515e2c3036deba3
  pr: null
  completed_at: 2026-11-05         # YYYY-MM-DD
  notes: "verify verdict APPROVED at 8acb291 (build 390d2b8); the sub-agent cannot see its own token meter, the orchestrator transcribes the metered figure"
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-016: Verify the mutation helpers and mutated-copy red-proofs

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-008` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-008-mutation-helpers-and-mutated-copy-red-proof-apparatus.md`
- **Build handoff and Handback:** `HANDOFF-015-build-mutation-helpers-and-mutated-copy-red-proof-apparatus.md` (claims to verify, not to trust)
- SPEC-005 context: `HANDOFF-013`/`HANDOFF-014`; `guidance/constraints.yaml` (`oracle-must-be-shown-red`, `exiftool-never-linked`); `AGENTS.md`; `guidance/toolchain-brief.md`
- Code under review: `git diff 4221d5b..HEAD` (build commit `390d2b8`)
- ExifTool 13.55 at `/opt/homebrew/bin/exiftool`. **Its source must never be read.** Irradiance is read-only.

## Expected Deliverables

Review cold; output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

**Safety first:** ExifTool 13.55 does not return on `fuzz/seeds/tiff_ifd/two-hop-cycle` (about 9 GB). Never run it on that file. This host has no `timeout(1)`; for any ad hoc ExifTool probe use `perl -e 'alarm 4; exec @ARGV' exiftool ...`. Measure tools by capturing stdout to a variable and reading the tool's own status, never `$?` through a pipe.

Verify independently:
- Re-run every gate; also `just mutation-oracle` (record wall-clock) and `just oracle-record` (empty `git diff`, `two-hop-cycle` reported NOT RUN). Run the whole suite with `PATH` scrubbed of ExifTool and `DIAMOND_EXIF_REQUIRE_ORACLE` unset: only the live variants may skip, loudly.
- **The red-proofs are the point; test the testers.** For each of the six library-fault tests, confirm in the code what the needle injects and that the mutation is what the name says; then break the red-proof itself: e.g. (a) weaken a control (make the unmutated copy differ) and confirm the test fails rather than passes; (b) change a mutant's expected selector set and confirm it fails; (c) make a needle match zero or two places and confirm loud failure; (d) inject a mutation that should NOT show (little-endian fixture for the big-endian mutant) and confirm the test asserts the null result; (e) try mutants of your own that the builder did not (for example `floats` DOUBLE read as FLOAT width, `ascii` treating a NUL as not a terminator, wrong IFD kind label, off-by-one in the SubIFDs cap) and report which the suite catches and which survive.
- Judge the deviations: mutant 1 placed at the SHORT use site in `Container::uints` rather than the `ByteOrder::u16` big-endian arm (is the mutation what the spec wanted? is it still the "byte order" fault?); the extra fixtures (`exif-gps-full-le`, `signed-float-le`, `signed-float-be`) and the re-recorded 9 old records (confirm the only change is the added `-` columns); mutation 9's two cuts; `run_tool_with_limit` as the single spawn function; the probe's own cargo/probe run having no wall-clock limit (is that acceptable? could a mutant that loops forever hang the suite? try a mutant that loops).
- Check the new signed/float selectors: re-measure each of the four with ExifTool (command, version) and judge whether the comparison is honest (ExifTool honouring the declared type), including `ExifIFD:ExposureTime` as FLOAT 0.125 and `FocalLength` as DOUBLE 4.5.
- Timeout: confirm `kill -0` style evidence that the child is gone, default 5 s, env override, and that `run_tool_with_limit` is the only place ExifTool is spawned (grep). Confirm `RED-PROOF INJECTION` appears only in test code.
- Check `TempDir` cleanup (no stray temp dirs after the full suite and after a forced panic), that nothing touches the repo `target/` from the probe, and that a stale `CARGO_TARGET_DIR` in the environment cannot redirect the probe builds.
- Verify `src/`, `scripts/`, `decisions/`, irradiance and the 29 seeds are untouched; `[dependencies]` and `[dev-dependencies]` empty.
- Look for over-generalised claims about ExifTool ("always", "never", "every time") in comments and docs added by this spec.
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes; CI wiring; JPEG-level red-proofs; a memory limit

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

- **Branch / PR:** branch `worktree-agent-a7515e2c3036deba3` (fast-forwarded to `8acb291`, handback committed on top; not pushed); no PR, by instruction.
- **Completed at:** 2026-11-05
- **All acceptance criteria met?** Yes. The findings below are low or informational; none blocks.
- **For `verify`:** ✅ **APPROVED at `8acb291`** (code under review `git diff 4221d5b..390d2b8`, build commit `390d2b8`; `8acb291` adds only handoff docs).

**Gates, re-run by me (each tool's own status captured to a file, no pipes):** `cargo fmt --all -- --check` 0; `just lint` 0; `just lint-fuzz` 0 (clippy 0.1.98, `+stable`); `cargo test` (Homebrew 1.99.1) 0, 145 passed / 0 failed / 2 ignored; `~/.cargo/bin/cargo +1.90.0 test` 0, same counts; `cargo rustc --lib -- -F unsafe_code` 0; `just deny` 0; `just deny-fuzz` 0; `just validate` 0; `just decisions-audit` 0; `just cost-audit` 0. `just oracle-record` 0: 41 records written, `NOT RUN on two-hop-cycle: exiftool-two-hop-cycle-does-not-return`, `git status --porcelain` empty afterwards.

**`just mutation-oracle` wall-clock:** 3.38 s real (`/usr/bin/time -p`, warm, Apple silicon), 36 passed / 1 ignored, test time 3.06 s. Matches the build claim (about 3 s).

**ExifTool hidden:** whole suite on the 1.90.0 toolchain's own cargo with `PATH=<1.90.0 bin>:/usr/bin:/bin`, `DIAMOND_EXIF_REQUIRE_ORACLE` unset: status 0, 145 passed; exactly five `SKIP` lines, all live variants (`corrupting_the_gps_pointer_is_reported_and_isolated (live variant)`, `live_demo_a_mutated_payload_is_caught`, `live_parity_on_hand_built_fixtures`, `live_parity_on_the_committed_seeds`, `live_signed_and_float_selectors_match_exiftool`). With ExifTool present, the GPS live variant ran (`ExifTool agrees, GPS selectors are -`) and the signed/float live test ran against 13.55.

**Testing the testers** (plants into the tree, each reverted with `git checkout`; tree clean at the end):

| Plant | Expected | Result |
|---|---|---|
| (a) committed record `signed-float-le` ExposureProgram `-2` changed to `-3`, so the control cannot be green | mutant 6 and the all-fixtures control fail at the CONTROL | both FAILED: `sshort-unsigned: the unmutated control is not green on signed-float-le: ExifIFD:ExposureProgram: ours="-2" theirs="-3"` (and `all-fixtures: ...`) |
| (b1) mutant 3 expected set reduced to `[FNumber]` | fail | FAILED: named `["ExifIFD:FNumber", "IFD0:XResolution"]` vs `["ExifIFD:FNumber"]` |
| (b2) mutant 5 expected set changed to `[FNumber]` | fail | FAILED |
| (c1) mutant 2 needle misspelt (`bytez`) | loud | FAILED: `expected exactly one occurrence ... found 0 -- the injection point moved` |
| (c2) mutant 6 needle `out.push(` | loud | FAILED: `... found 9` |
| vacuous mutant: mutant 4 replacement equals the needle (plus marker) | fail (mutant green) | FAILED: diff `[]` vs the two GPS selectors |
| (d1) mutant 1 replaced by `from_be_bytes` (shows on LE, not BE) | fail | FAILED at the BE set (`[]`) |
| (d2) mutant 1 replaced by `.swap_bytes()` (wrong on both orders) | the LE null-result assertion fails | FAILED: `little-endian fixture must stay green: IFD0:Orientation ours="1536" ...` |

**My own mutants, planted into the real `src/` one at a time, whole suite run with `--no-fail-fast`, reverted:**

| Mutant | Caught? | By |
|---|---|---|
| DOUBLE read at FLOAT width (`as_chunks::<4>` + `f32`) | yes | replay/live oracle (`replay_signed_and_float_selectors_carry_values`, `replay_is_clean_on_an_unmodified_reading`, `live_*`), all-fixtures control, lib `floats_read_verbatim_in_both_orders` |
| FLOAT always little-endian | yes | oracle on `signed-float-be`, controls, lib test |
| ASCII: NUL not a terminator (`position(\|b\| *b == 0xFF)`) | yes | oracle, every mutation_oracle control, lib ascii tests |
| SLONG truncated to 16 bits | yes | oracle via `ExifIFD:ExifImageWidth`, lib `sints_sign_extend_*` |
| GPS IFD labelled Interop (wrong IFD kind) | yes | oracle, mutation tests, lib labelling test |
| Exif IFD labelled Sub (wrong IFD kind) | yes | oracle, mutation tests, many lib tests |
| SBYTE read unsigned | yes, but only by lib `sints_sign_extend_every_width_in_both_orders` | no oracle selector reads an SBYTE |
| SubIFDs cap off by one (`n > MAX_IFDS + 1`) | **SURVIVED** the whole suite (status 0) | not oracle-visible (ExifTool has no such cap); a pre-existing library-test gap, no test at count == `MAX_IFDS + 1` |

**Deviations judged.**
- Mutant 1 at the SHORT use site: justified, and now measured rather than reasoned. The literal spec mutant (`ByteOrder::Big => u16::from_le_bytes` in `reader.rs`), staged and run on `exif-basic-be`, makes the probe exit 3 with `PARSE-ERROR UnsupportedTiffVersion { version: 10752 }` (0x2A00), so no selector could be named. The planted fault is still "a SHORT decoded in the wrong byte order"; it does not cover tag/type/count reads, which the header failure already makes loud.
- Extra fixtures and re-recorded records: the word diff of the nine old records shows only the command line growing by four selectors and four added `-` fields; the 29 seed records are unchanged (my `just oracle-record` reproduced all 41 byte for byte). `exif-gps-full-le` is needed for test 7's isolation claim; `signed-float-le/-be` carry the new selectors.
- Mutation 9's two cuts: both asserted; (b) (no issue, inline ISO survives) is the stronger, measured statement. Accepted.
- `run_tool_with_limit` as the single spawn function: grep shows the only tool spawn is `tests/support/tools.rs:105`; `exiftool_version` and `run_exiftool_raw` go through `run_tool`. The other `Command::new` hits are the self-respawn tests and the probe's cargo/probe spawns, as the build claimed.
- Probe with no wall-clock limit: see F1.

**Signed/float selectors re-measured independently** (my own Python generator: IFD0 {Make, ExifIFD pointer} plus a one-entry Exif IFD, both byte orders; `perl -e 'alarm 4; exec @ARGV' exiftool -T -n -s3 -ExifIFD:ExposureTime -ExifIFD:ExposureProgram -ExifIFD:FocalLength -ExifIFD:ExifImageWidth <file>`, ExifTool 13.55, status 0 for every file, LE and BE identical): FLOAT 0.125 in 0x829A prints `0.125` (RATIONAL 1/8 there also `0.125`); SSHORT -2 in 0x8822 prints `-2` while SHORT 65534 (same bits) prints `65534`; DOUBLE 4.5 in 0x920A prints `4.5` (RATIONAL 9/2 also `4.5`); SLONG -5 in 0xA002 prints `-5` while LONG 4294967291 (same bits) prints `4294967291`. Same bits, different declared type, different output: ExifTool 13.55 honours the declared type for these four tags, so the comparison is honest. A FLOAT-width read of the DOUBLE would give 0 (the low word of 4.5 is zero) and was caught.

**Timeout.** `run_tool_times_out_and_kills_the_child` passes (1 s limit, `Timeout { tool: "sh", secs: 1 }`, then `kill -0 <pid>` through `run_tool_with_limit` fails: pid gone). After all runs, `pgrep -fl 'sleep 30'` and `pgrep -fl exiftool` both return status 1 (nothing left). Default 5 s (`DEFAULT_TIMEOUT_SECS`, asserted); the env override is exercised end to end by `timeout_variable_is_honoured_by_run_tool` (child copy with the variable at 1). `/* RED-PROOF INJECTION */` occurs in code only in `tests/support/probe.rs` (plus prose in the spec, the handoffs and the provenance ledger row).

**TempDir / target.** No `diamond-exif*` entries in `$TMPDIR` after the full suite, after the plant runs in which six tests panicked inside `inject_replace` (TempDir dropped on unwind), or at the end. Stale `CARGO_TARGET_DIR=<scratch>/stale-target cargo test --test mutation_oracle`: 36 passed, zero probe binaries in the stale dir, repo `target/` mtime unchanged, no `probe` binary in the repo `target/`.

**Untouched.** `git diff --stat 4221d5b..HEAD -- src scripts decisions fuzz/seeds Cargo.toml Cargo.lock` is empty; 29 seeds; `[dependencies]` and `[dev-dependencies]` empty; irradiance `git status --porcelain` empty. ExifTool was never run on `two-hop-cycle` (the recorder reports it NOT RUN).

**Findings.**
- **F1 (low).** The probe's cargo build and probe run (`tests/support/probe.rs` `build_probe`/`run_probe`) have no wall-clock limit. Measured: a staged mutant that sleeps 25 s per ASCII read held its test for 75.6 s (three ASCII reads), with only cargo's "running for over 60 seconds" note; a mutant that loops would hang `just mutation-oracle` indefinitely. The committed mutants are fixed and finish in about 1 s, and the in-process oracle tests have the same property for a looping library, so it does not block. Suggest running the probe binary through the same poll-and-kill helper.
- **F2 (low, measurement-over-generalised).** The `tests/support/oracle_tiff.rs` comment on the new tag constants says the declared-type behaviour is something "most tags do not expose". The builder's own table contradicts it (Orientation SSHORT -3, ImageWidth SLONG -5, ISO SSHORT -200 and FNumber FLOAT/DOUBLE 2.5 were all honoured; only three of the 17 candidates were not). Reword to the measured count.
- **F3 (info, follow-up).** The SubIFDs cap off-by-one survives the whole suite (a library unit-test gap at count == `MAX_IFDS + 1`, outside SPEC-008's oracle scope). SBYTE-read-unsigned is caught only by a library unit test; no oracle selector reads SBYTE.
- **F4 (info).** `escaping_round_trips` tests a hand-written mirror of the probe's `escape`, not the generated `PROBE_MAIN` code, so the two could drift unseen. `no_injection_marker_outside_the_tests` scans only `src`, `examples`, `fuzz` and `scripts`, not `tests/support/oracle.rs`, which is copied into every probe crate.
- **F5 (info).** A mutant that makes `Container::parse` fail becomes an apparatus `Err` (probe exit 3), so its test panics with "the MUTANT could not be built or run" instead of naming selectors. It is red either way; worth knowing when writing future mutants.

### Cost self-report

- **Tokens (total):** null. The verifier sub-agent cannot see its own token meter; the orchestrator transcribes the metered figure.
- **Estimated USD:** null (same reason)
- **Duration (minutes):** ~30
- **Source of the number:** none available to the sub-agent; harness report via the orchestrator

### Drift and new artifacts

- **New decisions emitted:** none.
- **Deviations from spec:** none by the verifier. The build deviations listed in HANDOFF-015 are judged above and accepted.
- **Follow-up work identified:**
  - A wall-clock limit on the probe build/run (F1).
  - A library unit test for the SubIFDs cap boundary (F3); an SBYTE oracle selector if ExifTool exposes one.
  - Reword the `oracle_tiff.rs` comment (F2); test the generated escape directly (F4).

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Nothing material. "Report which the suite catches" for my own mutants could mean the oracle alone or the whole suite; I planted into the real `src/`, ran the whole suite and named which layer caught each.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — The worktree sandbox refuses compound shell lines (pipelines mixed with `git`, variable-driven commands). Small scratch scripts for gates, plants and measurements are the reliable route, and the handoff could say so up front.

3. **If you did this task again, what would you do differently?**
   — Start with the plant script (named, exact-once replacements and one revert command) and run every tester-test from it; it made each plant reproducible and the revert mechanical.
