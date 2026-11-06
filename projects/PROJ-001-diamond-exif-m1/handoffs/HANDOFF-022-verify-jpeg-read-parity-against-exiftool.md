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
  id: HANDOFF-022
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-05
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-011

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
  tokens_total: 281483             # orchestrator-recorded from the metered sub-agent result (subagent_tokens)
  estimated_usd: 1.86
  duration_minutes: 13
  branch: worktree-agent-a678fade6bb9295e4
  pr: null
  completed_at: 2026-11-05         # YYYY-MM-DD
  notes: "verdict PUNCH LIST at da4e052; tokens transcribed by the orchestrator from the sub-agent metered result"
  synced_at: 2026-11-05
---

# HANDOFF-022: Verify JPEG read parity against ExifTool

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-011` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-011-jpeg-read-parity-against-exiftool.md` (outputs, named tests, process rules normative; its ExifTool table is a prediction the build re-measured); **DEC-007**, DEC-006
- **Build handoff and Handback:** `HANDOFF-021-build-jpeg-read-parity-against-exiftool.md` (claims to verify, not to trust; read its deviations list: wildcard `exif:*` divergence, m4/m5/m3 semantics, seeds compared on Exif selectors only, temp-file race fix)
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure ExifTool exit status through a pipe)
- Code under review: `git diff c79ed73..HEAD` (build commit `2d5cade`); main new files `tests/jpeg_oracle.rs`, `tests/support/oracle_jpeg.rs`, the `tests/support/{oracle,probe,tools}.rs` edits, `src/jpeg/{mod,metadata}.rs` (DEC-007 only)

## Expected Deliverables

Review cold; output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

Verify independently:
- Re-run every gate (fmt both graphs, `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `cargo test` on stable and 1.90.0, also once with `DIAMOND_EXIF_REQUIRE_ORACLE=1`, `-F unsafe_code`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `oracle-record` with a clean `git status` afterwards). Run `just fuzz-smoke jpeg_segments` for 60 s and record executions and coverage.
- **Re-measure the claims against ExifTool yourself** (the tool is installed; run it, never read its source): the DEC-007 cases (sixth byte non-zero read; fifth byte non-NUL not; 5-byte payload not; APP2 not), multi-Exif (same tag both orders, disjoint tags), Exif after the scan, last-segment-at-EOF, the 17-block case. Does each recorded divergence reproduce, and is each "right side" sentence defensible? Is any difference papered over by the wildcard `exif:*` entry or by seeds being compared on Exif selectors only?
- **Judge the harness model** (`read_ours_jpeg`: union, later block wins): does it match ExifTool for fixtures with a *damaged* later block, a later block that is fatal, and more than 16 blocks? Can it hide a library bug (e.g. mask a wrong primary)? Read `compare_jpeg` and the stale-divergence logic: can a divergence list entry rot or loosen a comparison?
- **Test the testers.** Redo m1-m7; write mutants the builder did not try (e.g. `read_ours_jpeg` picking the first block, skipping fatal blocks wrongly, presence compare inverted for XMP, IPTC id 0x0405 accepted, DEC-007 off by one on the length test `>= 6` vs `> 6`, `tiff` slice at 7, a wrapped-fixture builder that silently drops a TIFF, the probe limit constants, a record writer that drops `# stderr:` lines, the marker-leak scan skipping `tests/support`) and report which the suite catches and which survive. Redo the fuzz plant against the real target. Confirm every named test in the spec exists (list renames/missing).
- Check the records: the 41 TIFF records are byte-identical to `c79ed73`; JPEG records carry command, version, source and `# stderr:`; the re-record reproduces them; the `exif:*` handling in replay equals live. Check the CI change (`oracle-records` now uses `git status --porcelain`): does it fail on a new untracked record, and pass on a clean tree? Confirm no other CI job changed.
- Check the seeds: existing 63 byte-identical; 3 new ones deterministic and claiming their shapes (`xxd`). Check no dependencies, `scripts/`, `decisions/` (apart from DEC-006/DEC-007 as written at design), irradiance untouched.
- Judge the temp-file race fix (`tools.rs`) and the probe `Child::kill` semantics (orphaned `rustc`): acceptable or a finding?
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes, the audit-stage merged Exif view, C2PA, CI wiring beyond reading it

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

- **Branch / PR:** branch `worktree-agent-a678fade6bb9295e4` (fast-forwarded to `da4e052`); no push, no PR, cycle not advanced. Only this file is committed.
- **Completed at:** 2026-11-05
- **All acceptance criteria met?** Mostly. Every named test exists, every gate is green and every claim I re-measured reproduces. One finding (M1 below) contradicts the spec's rule that a divergence entry is never a loosening.
- **For `verify`:** ⚠ **PUNCH LIST** at `da4e052` (build commit `2d5cade`). One medium item (M1), plus low items L1-L3 that can be fixed in the same pass or deferred.

#### Findings (severity)

- **M1 (medium): the `exif:*` wildcard leaves our reading of the after-scan and at-EOF fixtures unchecked.** `compare_jpeg` explains any Exif mismatch on `jpeg-exif-after-scan` / `jpeg-exif-last-segment-at-eof` / `jpeg-seed-exif-after-scan` where ours is `Some` and ExifTool's is `None`, whatever value ours holds. I set ours to `IFD0:Make = "WRONG-VALUE"` with every other field `None` (13 real fields lost) and `compare_jpeg` came back `clean=true, explained=1` on both fixtures, using a scratch test that I then deleted. The test `exif_after_scan_is_a_recorded_divergence_and_still_present` asserts only `Make.is_some()`. So a library fault limited to Exif after the scan or at EOF (wrong slice, wrong block, lost tags) passes. Fix: on each wildcard fixture also require ours (Exif selectors) to equal the committed record of the same TIFF before the scan (`jpeg-exif-basic-le`, or bare `exif-basic-le`). That is container independence across positions, and the wildcard then only stands in for ExifTool's silence.
- **L1 (low): the harness model includes Exif blocks after the scan, but ExifTool ignores them, and no fixture covers a block before the scan plus one after it.** Measured (13.55, `-T -n -s3`, alone under `alarm 8`, exit 0): SOI, Exif A (Make AlphaCam, Model OnlyInA), SOS, Exif B (Make BravoCam), EOI. ExifTool prints `AlphaCam OnlyInA`. `read_ours_jpeg` gives `BravoCam OnlyInA`. This would be an unexplained mismatch with both sides `Some`, which the wildcard does not cover. Fix: add the fixture and either drop `after_scan()` blocks from the model or record the divergence (question `exiftool-ignores-exif-after-scan` already exists).
- **L2 (low): Composite GPS across blocks is not modelled.** Measured: block A has GPSLatitudeRef N with Lat 37/1 30/1 0/1, block B has only GPSLatitudeRef S. ExifTool prints `Composite:GPSLatitude = -37.5`, `GPS:GPSLatitudeRef = S`, combining A's latitude with B's ref. The model computes the composite per block and gives `37.5`. No fixture has this shape. Record it as a known limit of the model (question or a comment on `read_ours_jpeg`).
- **L3 (low): three test-the-tester survivors** (table below). V3: no multi-Exif fixture has a fatal block, so the model's fatal-block skip is unpinned. I measured `good-then-fatal` → AlphaCam/OnlyInA and `fatal-then-good` → BravoCam, and the model agrees on both, so these are cheap fixtures to add. V5: an IPTC resource id `0x0405` accepted as IPTC survives (the only negative fixture uses `0x0408`). V12: removing the `tests/support` scan from the real-tree check survives, because the test proves its scope only on a temp copy and not on the scan of the real tree.
- **Info:** with two Exif APP1s, `exiftool -Warning` reports `[minor] File contains multi-segment EXIF`. "Union, later wins" fits every shape measured (2, 17 and 20 blocks, fatal or damaged later blocks), but it is a fitted model, not ExifTool's documented rule. The builder's "stderr only when nothing is found" holds for `-T` (here the warning goes to the `Warning` tag, not stderr).
- **Info, acceptable:** the temp-file counter fix in `tools.rs` is correct (per-process counter plus pid). Probe `Child::kill` can orphan a `rustc` when the build times out. That is acceptable for a test apparatus that only fires on a timeout; it is documented in the HANDOFF-021 deviation 13.
- **Info:** `jpeg_truncated_between_app1_and_sos_is_a_typed_outcome` compares the cut file with the uncut file's record. ExifTool itself reads nothing on that cut (the last-segment-at-EOF shape). The test says so, and it is a typed-outcome test, not a parity test.

#### ExifTool re-measured (13.55, independent Python-built fixtures, each run alone under `perl -e 'alarm 8'`, exit status read directly: all 0, none hung)

| Shape | ExifTool | Our model | Agrees with build's claim |
|---|---|---|---|
| `Exif\0\x01` + TIFF / `Exif\0\xFF` + TIFF | reads (DiamondCam) | reads | yes |
| `Exif\x01\0` + TIFF | nothing | nothing | yes |
| 5-byte payload `Exif\0` | nothing | nothing | yes |
| 6-byte payload `Exif\0\0` (new) | nothing; warns `Malformed APP1 EXIF segment` | fatal block, nothing | consistent |
| Exif in APP2 | nothing | nothing | yes |
| two Exif same tag, A→B / B→A | BravoCam+OnlyInA / AlphaCam+OnlyInA | same | yes |
| two Exif disjoint | AlphaCam+BravoModel | same | yes |
| Exif after scan | nothing, no stderr | reads | yes (divergence) |
| last segment at EOF | nothing; `Warning: JPEG format error` | reads | yes (divergence) |
| COM at EOF / EOI no SOS | reads | reads | yes |
| 17 blocks, 17th differs | LastOnly17 | SameFirst16 | yes (divergence) |
| 17 blocks, 17th disjoint tag (new) | Software=Only17th | `-` | same cap class |
| 20 blocks, 20th differs (new) | Twentieth | SameFirst | same cap class |
| good then fatal / fatal then good / good then damaged (new) | AlphaCam / BravoCam / AlphaCam | same | model holds |
| before AND after scan (new) | AlphaCam, OnlyInA | **BravoCam**, OnlyInA | **L1** |
| GPS split across blocks (new) | **-37.5**, S | 37.5 | **L2** |

#### Mutant table (tree mutated in place, `cargo test --no-fail-fast` over lib + jpeg_oracle + fuzz_layout + metadata_oracle, plus mutation_oracle for test-support mutants; each reverted with `git checkout`)

| Mutant | Result | Caught by (examples) |
|---|---|---|
| m1 `after(5)` | caught | 30+ incl. `container_independence_…`, `replay_jpeg_records_…`, lib slice tests |
| m2 six-byte signature restored | caught | `exif_signature_is_five_bytes_per_exiftool`, lib/fuzz_layout iff tests, `new_seeds_…`, live |
| m3 blocks reversed | caught | `multi_exif_…`, lib primary tests, replay, live |
| m4 after-scan Exif skipped | caught | `exif_after_scan_…` (stale), `a_divergence_…`, replay, lib |
| m5 cap 15 | caught | lib `exif_blocks_are_capped_at_sixteen…`, `warning_table_…` (not by the oracle, as the build reported) |
| m5 cap 17 | caught | `seventeenth_…` (stale), replay, live |
| m6 APP1 guard to APPn | caught | `exif_signature_…per_exiftool`, lib, live |
| m7 IPTC id `!=` | caught | `iptc_without_0404_…`, lib IPTC tests |
| V1 `read_ours_jpeg` first block | caught | `multi_exif_…`, replay, m3/m5 probe tests |
| V2 last OK block only (no union) | caught | `multi_exif_…`, replay, live |
| V3 fatal block stops the model's search | **survived** | none (no fixture) |
| V4 XMP presence inverted | caught | `region_presence_…`, replay, many |
| V5 IPTC id 0x0405 also accepted | **survived** | none |
| V6 DEC-007 `>= 6` → `> 6` | caught | lib (`a_signature_only_exif…`, `exif_signature_is_five_bytes_sixth_byte_ignored`), classify test, fuzz_layout |
| V7 `after(7)` | caught | 30+ |
| V8 wrapped builder drops a TIFF | caught | `every_wrapped_…`, `jpeg_fixtures_are_deterministic…`, container tests |
| V9 probe run default 30→31 | caught | `probe::tests::limit_override_…` |
| V10 limit parse accepts 0 | caught | same + `probe_build_and_run_have_a_wall_clock_limit` |
| V11 record writer drops `# stderr:` | caught | `tools::tests::jpeg_records_round_trip_their_stderr_lines` (and a re-record diff) |
| V12 marker scan skips `tests/support` | **survived** | none |
| V13 wildcard covers region selectors | caught | `a_divergence_…`, replay |
| V14 stale check disabled | caught | `a_divergence_…`, m4/m5 probe tests |
| V15 wildcard direction guard removed | caught | `a_divergence_…` |
| V16 `exif()` returns the last OK block | caught | lib, `multi_exif_…`, fuzz_layout |
| V17 `warning_lines` keeps the path | caught | tools unit test, live tests |
| V18 `known_divergence` ignores fixture for `exif:*` | caught | `a_divergence_…`, replay, probe tests |

The builder's m1-m7 probe tests also pass under the gate run (mutation_oracle 51/51).

#### Gates (this worktree, at `da4e052`)

All exit 0: `cargo fmt --all -- --check`; `cargo fmt --manifest-path fuzz/Cargo.toml -- --check`; `rustfmt --check` of `fuzz/shared/jpeg_metadata_invariants.rs`; `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `forbid-unsafe`, `deny`, `deny-fuzz`, `validate`, `decisions-audit` (0 errors, 10 scope warnings), `cost-audit`. `cargo test` was run on Homebrew stable, on `+1.90.0`, and on `+1.90.0` with `DIAMOND_EXIF_REQUIRE_ORACLE=1`; each gave lib 144, crate_policy 4, fuzz_layout 14, jpeg_oracle 37, metadata_oracle 30, mutation_oracle 51, 0 failed. `just oracle-record` reproduced all 116 records byte-for-byte and `git status` stayed clean afterwards. `cargo run --example fuzz_seeds` reproduced all 66 seeds (status clean).

#### Records, seeds, scope

- The 41 TIFF records and the 63 existing seeds are unmodified since `c79ed73`: `git diff --diff-filter=MD` is empty and the fnv hash tests pass. The JPEG records carry command, version, source and `# stderr:`, and the replay header test enforces this. Replay and live apply the `exif:*` entry through the same `compare_jpeg`.
- The 3 new seeds have the shapes they claim (`xxd`): `exif-sixth-byte-nonzero` is `Exif\0\x01` + II TIFF + SOS + EOI; `exif-five-byte-payload` is APP1 length 7, `Exif\0`; `icc-seq-zero` is `ICC_PROFILE\0` seq 0 / count 1. Seed counts are 29 / 37.
- CI: the only change since `cb78246` is the `oracle-records` step (made in `da4e052`, not by the build). I ran its body verbatim locally: clean tree exit 0, a new untracked `jpeg-*.txt` exit 1 (named), clean again after removal exit 0.
- No dependency, `scripts/`, `decisions/`, `justfile`/`app.just` change in the build. `decisions/` changed only at design (DEC-006 one line, DEC-007 added). DEC-007 confidence is 0.85.
- Named tests: all 38 spec names exist (m1-m7 with the `jpeg_mutant_` prefix; the 63-seed hash test is `the_existing_63_seeds_are_byte_identical`). There are 2 extra tests in jpeg_oracle.

#### Fuzz

- `just fuzz-smoke jpeg_segments` (60 s, exit 0): **119,512 executions** ("Done 119512 runs in 61 second(s)"). INITED from 37 seeds at `cov: 1223 ft: 2375 corp: 34/86Kb`, final `cov: 1486 ft: 6392 corp: 518/2525Kb`, about 1.96k exec/s.
- Plant against the real target: a different plant from the builder's, the DEC-007 length test off by one (`>= 6` → `> 6`). Fresh corpus, seeds as input. libFuzzer found a crashing input after about 85k executions: `fuzz_targets/jpeg_segments.rs:151` `assertion left == right failed: kind, left: App(1), right: Exif`, exit 1. I reverted the plant and deleted the crash artifact.

The source tree was left clean (`git status --porcelain` empty before this commit). The temporary scratch test and all mutants were removed or reverted.

### Cost self-report

- **Tokens (total):** null. This sub-agent has no `/cost` or `usage` readout. The orchestrator transcribes the metered figure.
- **Estimated USD:** null
- **Duration (minutes):** about 55 (estimate)
- **Source of the number:** none available to the agent

### Drift and new artifacts

- **New decisions emitted:** none
- **Deviations from spec:** none by the verifier. Build deviations I checked: wildcard `exif:*` (see M1); m4 and m5 caught as stale entries (reproduced, acceptable); seeds compared on Exif selectors only (acceptable: there is no region-bearing seed with a comparable tag); the temp-file race fix (acceptable).
- **Follow-up work identified:**
  - Punch list: M1 (pin the values on the wildcard fixtures); L1 fixture with Exif before and after the scan, plus either a model change or a recorded divergence.
  - Can be deferred: L2 GPS-across-blocks note or question; L3 fixtures and tests (fatal later block, IPTC `0x0405`, assert the real-tree scan covers `tests/support`).
  - Audit stage: ExifTool flags several Exif APP1s as "multi-segment EXIF"; the model is fitted, not derived. Keep it measured.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Very little. The handoff's mutant list was concrete. The one thing left open was whether "redo m1-m7" meant rerunning the probe tests or mutating the real tree. I did both: the probe tests in the gate run and in-place mutants through a driver.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — The spec says a divergence entry is "never a loosening", but it gives no rule for wildcard entries. One sentence would have prevented M1: a wildcard divergence must still pin our own values against an independent expectation.

3. **If you did this task again, what would you do differently?**
   — Build the independent shapes and the "ours vs ExifTool" table first, before reading the harness. The two model gaps (L1, L2) came out of shapes that were not in the fixture list, and starting from measurement finds those faster than reading code.
