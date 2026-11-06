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
  id: HANDOFF-021
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
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
  tokens_total: 415740             # orchestrator-recorded from the metered sub-agent result (subagent_tokens)
  estimated_usd: 2.74
  duration_minutes: 26
  branch: worktree-agent-a62ec9ba8e1c2b43f
  pr: null
  completed_at: 2026-11-05         # YYYY-MM-DD
  notes: "tokens transcribed by the orchestrator from the sub-agent metered result"
  synced_at: 2026-11-05
---

# HANDOFF-021: JPEG read parity against ExifTool

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-011` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

Read before starting:

- **Spec (measured table, outputs, named tests and process rules are normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-011-jpeg-read-parity-against-exiftool.md`
- **`decisions/DEC-007-exif-signature-is-five-bytes.md`** (the one library change), DEC-006, DEC-001, DEC-002, DEC-004
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; `just lint` and `just lint-fuzz-red-proof` under the default PATH; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe)
- Existing oracle apparatus: `tests/support/{tools,oracle,oracle_tiff,probe,mutate}.rs`, `tests/{metadata_oracle,mutation_oracle,fuzz_layout}.rs`, `examples/{oracle_record,fuzz_seeds}.rs`, `fuzz/shared/jpeg_metadata_invariants.rs`, `fuzz/fuzz_targets/jpeg_segments.rs`, `src/jpeg/{mod,metadata}.rs`
- Prior art: HANDOFF-015/016 (SPEC-008 mutated-copy apparatus and what verify caught), HANDOFF-019/020 (SPEC-010)

Constraints: `oracle-must-be-shown-red`, `panic-free-library` (five-lint deny set in non-test code), `forbid-unsafe-code`, `fuzz-target-per-surface`, `test-before-implementation`, `no-new-top-level-deps-without-decision` (no dependencies of any kind). ExifTool is run, never read.

## Expected Deliverables

- Everything in the spec's Outputs with every named test and Acceptance Criterion, including both design-section-8 JPEG red-proofs, mutated-copy faults m1-m7 (control run first, named mismatch each), the five-byte fuzz invariant shown red against the real libFuzzer target, and a clean 60 s `just fuzz-smoke jpeg_segments` (executions and coverage recorded).
- The spec's ExifTool table is a **prediction from the design session**: re-measure every row through the real harness. Where your measurement differs, the measurement wins; list every such row in the Handback.
- Probe each new JPEG shape through ExifTool **alone, under the limit,** before it joins any loop. A shape ExifTool does not return on becomes a `"*"` divergence and is never run through the tool.
- The 41 TIFF records must stay byte-identical (CI's `oracle-records` job diffs them). The existing 63 seeds (29 TIFF + 34 JPEG) must stay byte-identical.
- **Run `just lint-fuzz-red-proof` and `just lint-red-proof` locally** (they are otherwise CI-only; the last PR failed in CI because I did not). Also run `just oracle-record` and confirm an empty diff for the TIFF records.
- Name every deviation (renamed, folded or missing tests; derived numbers that do not match) in the Handback; if a divergence points at a library bug other than DEC-007, stop that test and report it instead of fixing it.
- Commit on your worktree branch. **Do not push and do not open a PR.** The spec is at `cycle: build`; do not advance cycles. First run `git merge --ff-only claude/rust-exif-metadata-6351d5` so your worktree has `c79ed73` or later.
- Do not modify irradiance, `scripts/`, `decisions/` or the ExifTool TIFF records. No dependencies or dev-dependencies.
- You cannot see your own token count; set `tokens_total: null` with a note, the orchestrator transcribes the metered figure.

## Out of Scope

- A merged multi-Exif view in the library; ignoring Exif after the scan; ICC/JUMBF/C2PA parity; XMP/IPTC value comparison; SBYTE selector; writing; the `exif-audit` CLI
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

- **Branch / PR:** branch `worktree-agent-a62ec9ba8e1c2b43f`, one local commit on top of `d800fad` (HANDOFF-021 itself); not pushed, no PR, cycle not advanced (spec stays `cycle: build`).
- **Completed at:** 2026-11-05
- **All acceptance criteria met?** yes. Where the spec's wording could not be met literally the difference is named under Deviations (m4 and m5 are the two that matter).
- **For `verify`:** n/a (build cycle).

### Cost self-report

- **Tokens (total):** null. This sub-agent has no `/cost` or `usage` readout, so it cannot report a number and does not invent one; the orchestrator transcribes the metered figure.
- **Estimated USD:** null
- **Duration (minutes):** null (not measured by the agent)
- **Source of the number:** none available to the agent

### What was built (summary)

- **DEC-007** in `src/jpeg/mod.rs` (`SIG_EXIF` is now the five bytes `Exif\0`, new `EXIF_HEADER_LEN = 6`; Exif iff APP1 and payload >= 6 bytes and first five match) and `src/jpeg/metadata.rs` (`tiff = payload[6..]` via `EXIF_HEADER_LEN`). Tests written first and shown red (2 failures: `exif_signature_is_five_bytes_sixth_byte_ignored`, `app_segments_are_classified_by_marker_and_signature`), then green. L1 (real worst case: exactly 4096 minimal segments, no SOS, no halt: 2 walker issues + 4096 `RegionTooShort` = 4098 <= 5122; the old 4097-segment case relabelled as a halted case that records exactly one issue), L2 (docs on `AnyIssue::Exif`, `ExifBlock`, `Error::RegionTooShort`), L3 (the sixth-byte tests), I1 (`icc-seq-zero` seed + `icc_seq_zero_seed_yields_one_mismatch_issue`) closed.
- **Fixtures** `tests/support/oracle_jpeg.rs`: 12 wrapped TIFFs + the 26 normative shapes = 38 (std only, independent of `src/` and `examples/fuzz_seeds.rs`).
- **Records**: 38 `jpeg-<fixture>.txt` (22 selectors) + 37 `jpeg-seed-<seed>.txt` (20 selectors), all with `# stderr:` lines; 41 TIFF records untouched.
- **Comparator** (`tests/support/oracle.rs`): `read_ours_jpeg` (union, later block wins), `REGION_SELECTORS` presence, `diff_jpeg`, `compare_jpeg` (explained / unexplained / stale), `EXIF_ANY` wildcard divergence, 4 new `KNOWN_DIVERGENCES` entries.
- **Tests**: new `tests/jpeg_oracle.rs` (37 tests, builder's choice stated: separate file), JPEG additions to `tests/mutation_oracle.rs` (51 tests total there), `tests/fuzz_layout.rs` (14).
- **Apparatus**: probe JPEG mode, probe build/run wall-clock limits, marker-leak scan widened to `tests/support`.
- **Fuzz**: target + shared harness + `fuzz_layout.rs` mirror use the five-byte rule; 3 seeds (jpeg_segments 34 -> 37).

### Gate results (all run in this worktree)

- `cargo test` on 1.90.0 (`~/.cargo/bin/cargo +1.90.0 test`) and on the default Homebrew cargo 1.99.1: both green. Counts: lib 144, crate_policy 4, fuzz_layout 14, jpeg_oracle 37, metadata_oracle 30, mutation_oracle 51 (plus 1 ignored child test in each of the files that include `tools.rs`).
- `just lint` (fmt check + clippy `-D warnings`), `just lint-fuzz` (clippy 0.1.98 stable, pinned), `just lint-no-allow`, `just forbid-unsafe` (`-F unsafe_code`), `just deny`, `just deny-fuzz`: green. `cargo fmt --manifest-path fuzz/Cargo.toml -- --check` and `rustfmt --check fuzz/shared/jpeg_metadata_invariants.rs` (the `include!`d file `cargo fmt` does not reach): clean.
- `just lint-fuzz-red-proof`: green (control clean, `unnecessary_cast` injected into a copy rejected with `-D warnings`, passes without it; clippy 0.1.99). `just lint-red-proof`: green (all five lints fire, still fire without `-D warnings`).
- `just oracle-record`: run twice; the 41 TIFF records are byte-identical to HEAD (`git diff` over `tests/oracle-fixtures` is empty; the new `jpeg-*` files are untracked-new), and the second run reproduced all 116 files byte-for-byte (sha256 list compared). The TIFF records are also pinned by hash in `tiff_records_are_byte_identical_to_the_committed_set`.
- Live: `live_jpeg_parity_on_hand_built_fixtures`, `live_jpeg_parity_on_the_jpeg_fuzz_seeds`, `live_container_independence_against_exiftool` all green against ExifTool 13.55 (also re-checks that each committed record equals what the live tool prints today).
- Fuzz smoke, `just fuzz-smoke jpeg_segments` (60 s, exit 0): **155,437 executions** ("Done 155437 runs in 61 second(s)"), final `cov: 1478 ft: 6568 corp: 593/3164Kb`, exec/s about 2.5k; INITED from the 37 seeds at `cov: 1223 ft: 2375 corp: 34/86Kb`. `just fuzz-smoke tiff_ifd` (60 s, exit 0): **1,377,605 executions**, final `cov: 997 ft: 4669 corp: 726/544Kb`, INITED `cov: 833 ft: 1846 corp: 29`.
- `just validate`: 11 artifacts with valid front matter, 50 parseable. `just decisions-audit`: 0 structural errors, 10 scope warnings (the same advisory kind as before, now including `DEC-006 and DEC-007 govern the same scope`; DEC-007 is ours-by-design and does not contradict DEC-006, not touched because `decisions/` is out of bounds). `just cost-audit`: green. `just handback-sync SPEC-011 --dry-run` reports only that `tokens_total` is null (declared in the notes; the orchestrator transcribes the metered figure).

### Red-proofs, each shown red (control first where it applies)

All numbers below are printed by the tests under `--nocapture` and asserted.

- **Fuzz five-byte invariant, against the real libFuzzer target.** Plant (temporary, reverted, tree re-verified green): `classify` in `src/jpeg/mod.rs` restored to `payload.starts_with(b"Exif\0\0")`. `cargo +nightly fuzz run jpeg_segments ... -max_total_time=20` crashed at once on the seed `exif-sixth-byte-nonzero` (`fuzz_targets/jpeg_segments.rs:151`, `assertion left == right failed: kind, left: App(1), right: Exif`; exit status 77). Note it is the `kind` re-derivation (`expected_kind`) that fires first; the iff assertion written right below it (and `exif_by_signature` in the shared harness) is reached only if that one passes. The unplanted target then ran clean for 60 s (above).
- **Live parity red.** Same plant, `cargo test --test jpeg_oracle live_jpeg_parity`: both live tests FAILED naming `jpeg-exif-sixth-byte-nonzero: IFD0:Make: ours=None theirs=Some("DiamondCam")` (and the other 13 populated selectors) and `jpeg-seed-exif-sixth-byte-nonzero: IFD0:Make ...`; restored, both green.
- **Design section 8 red-proof 1 (truncate between APP1 and SOS)**: `jpeg_truncated_between_app1_and_sos_is_a_typed_outcome` on `jpeg-exif-gps-full-le` cut at the SOS (file offset 460): `Ok`, JPEG-layer issues exactly `[MissingSos, MissingEoi]`, 1 Exif block, clean, every tag the same as the uncut file (compared with the committed record: clean); plus every one of the 0..=len prefixes of the file read without a panic. Fixture `jpeg-truncated-between-app1-and-sos` (SOI, APP1, DQT, EOF) is separate.
- **Design section 8 red-proof 2 (corrupt an IFD offset inside the APP1 payload)**: GPS pointer value of `jpeg-exif-gps-full-le` set to `0x00FF0000` at file offset 162 (= the `Exif\0\0` signature position + 6 + `locate_entry`'s TIFF-relative offset, found without the library). Control: unmutated bytes clean against the record. Mutant: exactly `Composite:GPSLatitude: ours=None theirs=Some("37.4252083333333")` and `Composite:GPSLongitude: ours=None theirs=Some("122.096916666667")`; the other 14 populated selectors still match. Live variant: ExifTool on the mutated bytes reports `-` for both and agrees with ours. Layering test: the JPEG layer stays clean (`jpeg_issues()` empty), the damage is exactly one `AnyIssue::Exif { block: 0 }` with link `Tag(0x8825)` and offset `Some(0x00FF0000)` (TIFF-relative), `is_clean()` false.
- **Mutated-copy faults** (each: control on the unmutated staged copy clean first, then the mutant; before = control clean, after = what the mutant produced):
  - **m1** `after(EXIF_HEADER_LEN)` -> `after(5)`: every selector ExifTool reported becomes `ours=None theirs=Some(..)` on `jpeg-exif-basic-le` (14 of 14 populated; e.g. `IFD0:Make: ours=None theirs=Some("DiamondCam")`) and on `jpeg-exif-gps-ne-le` (`IFD0:Make`, `Composite:GPSLatitude`, `Composite:GPSLongitude`).
  - **m2** Exif arm back to `payload.starts_with(b"Exif\0\0")`: `jpeg-exif-sixth-byte-nonzero` names every reported selector, `ours=None`; `jpeg-exif-basic-le` stays clean.
  - **m3** `exif_blocks.push(` -> `exif_blocks.insert(0, `: `jpeg-two-exif-same-tag` names exactly `IFD0:Make: ours=Some("AlphaCam") theirs=Some("BravoCam")`, `-reversed` names exactly `IFD0:Make: ours=Some("BravoCam") theirs=Some("AlphaCam")`; `-disjoint-tags` and `exif-basic-le` clean.
  - **m4** Exif after the scan skipped: not a new mismatch (see Deviations): `jpeg-exif-after-scan` fails as a stale divergence, `jpeg-exif-after-scan exif:*: the divergence (exiftool-ignores-exif-after-scan) is no longer present`; `exif-basic-le` clean.
  - **m5** `MAX_EXIF_BLOCKS` 16 -> 17: `jpeg-seventeen-exif-last-differs IFD0:Make: the divergence (exiftool-reads-17th-exif-block) is no longer present`. 16 -> 15 is also run and does NOT change this fixture (divergence still present, `ours=Some("SameFirst16") theirs=Some("LastOnly17")`); reported, not hidden.
  - **m6** Exif arm `0xE1` -> `0xE0..=0xEF`: `jpeg-exif-in-app2` names the 14 selectors of `exif-basic-le`, all `ours=Some(..) theirs=None` (e.g. `IFD0:Make: ours=Some("DiamondCam") theirs=None`); `exif-basic-le` clean.
  - **m7** `resource.id == IPTC_RESOURCE_ID` -> `!=`: `jpeg-iptc` names exactly `IPTC:ObjectName: ours=None theirs=Some("present")`; `jpeg-iptc-without-0404` names exactly `IPTC:ObjectName: ours=Some("present") theirs=None`.
- **Probe limit**: `probe_build_and_run_have_a_wall_clock_limit`: a probe whose `extract` is `loop {}` builds, is killed after 2 s with a typed error containing "killed" and `DIAMOND_EXIF_PROBE_RUN_SECS`; a build given a 1 ms limit fails the same way (`DIAMOND_EXIF_PROBE_BUILD_SECS`). Defaults 600 s / 30 s; the override parser is a pure function (`parse_limit_secs`) with its own unit test, so no test touches the process environment.
- **Marker scan**: `injection_marker_does_not_leak_into_src_or_tests_support`; its own scope is proved (a temp `tests/support/{oracle,oracle_jpeg,mutate,tools}.rs` carrying the marker is caught 4/4; `probe.rs` alone is exempt and is checked to hold the marker exactly once).
- **Comparator red-proofs**: `oracle_names_the_one_field_perturbed_in_a_jpeg_record` (every populated field of every divergence-free fixture perturbed in turn: 7 Norm variants all seen; regions removed/added), `a_divergence_that_stops_being_one_fails` (flip the expectation: stale; wildcard one-directional).

### Divergences (all measured, ExifTool 13.55, macOS; all in `KNOWN_DIVERGENCES` and `guidance/questions.yaml`)

1. `jpeg-exif-after-scan` and seed `exif-after-scan` (`exif:*`, ours reads, ExifTool none; stdout all `-`, no stderr). Question `exiftool-ignores-exif-after-scan`. Right side: ours.
2. `jpeg-exif-last-segment-at-eof` (`exif:*`; stdout all `-`, stderr `Warning: JPEG format error - <file>`; ours reads and records `MissingSos`, `MissingEoi`). Question `exiftool-drops-exif-when-last-segment-ends-at-eof`. Right side: ours.
3. `jpeg-seventeen-exif-last-differs`, `IFD0:Make`: ExifTool `LastOnly17`, harness model `SameFirst16` (cap 16). Question `exiftool-reads-17th-exif-block`. Neither wrong; ours is the DEC-006 cap.
4. The seed sweep (37 seeds, each run alone under the 5 s limit; none timed out, none became a `"*"` entry; `soi-only` exits 1 with `Error: Unknown file type`) exposed exactly one more: seed `exif-after-scan` (entry above). `app-after-scan` is not one (its Exif is unreadable on both sides).

No library bug other than DEC-007 was found; nothing was stopped for that reason.

### Rows of the spec's ExifTool table whose measurement differed from the prediction

Everything in the table reproduced (two-Exif union and later-wins both orders, after-scan, JFIF before/after, APP2, sixth/fifth byte, 5-byte payload, last-at-EOF warning and no values, COM at EOF, EOI without SOS, after-EOI warning `Missing JPEG SOS`, truncated mid-scan, bad TIFF warning `Malformed APP1 EXIF segment`, bad lengths `JPEG format error`, XMP and IPTC presence) EXCEPT:

- **"stray bytes between segments: warns `Skipped unknown N bytes`"**: not reproduced for the shapes this spec builds. Three stray bytes (and in one shape an `FF 00` pair and a trailer after EOI) placed directly after the APP1, between APP1 and SOS, after the SOS header, between COM segments after an Exif, or leading into the first APP1: stdout has the Exif values and stderr is EMPTY (10 shapes tried, including the committed fixture). The warning (`Warning: [minor] Skipped unknown 3 bytes after JPEG COM segment - <file>`) appears only when nothing requested was found, e.g. seed `stray-bytes`, which has no Exif. So `jpeg-stray-bytes` records no warning. This is the more general finding recorded as question `exiftool-warnings-only-when-no-tag-is-found`: ExifTool's command line printed warnings only on inputs where all requested fields are `-` in every record I have (also `extended-xmp-pair`: warns on the seed, silent on `jpeg-extended-xmp-pair` where CreatorTool is found). The warning table is therefore a characterisation, as the spec says.
- Not in the table but measured: `jpeg-truncated-mid-app1` warns `JPEG format error` and reads nothing; `jpeg-ifd1-corrupt` and `jpeg-gps-corrupt-exif-ok` (wrapped) print no warning though ours records one TIFF-layer issue each.

### Drift and new artifacts

- **New decisions emitted:** none (DEC-007 was written in design).
- **Deviations from spec:**
  1. New test file `tests/jpeg_oracle.rs` for the "metadata_oracle.rs or a new jpeg_oracle.rs" choice. Two extra tests there beyond the list: `every_known_divergence_has_a_question_entry` (each divergence's `question_id` exists in `guidance/questions.yaml`) and `live_jpeg_skip_is_loud_and_required_mode_fails`.
  2. The marker-leak test already existed as `no_injection_marker_outside_the_tests`; renamed to the spec's `injection_marker_does_not_leak_into_src_or_tests_support`. `tests/support/probe.rs` is exempt from the scan because it defines the marker literal (checked to hold it exactly once).
  3. m1-m7 test names carry the prefix `jpeg_mutant_`, e.g. `jpeg_mutant_tiff_slice_starts_at_five_is_named` (the spec gives the tails only).
  4. **m4** ("Exif after the scan skipped"): that fault makes ours AGREE with ExifTool on `jpeg-exif-after-scan`, so there is no new mismatch to name; the named failure is the recorded divergence disappearing (stale entry naming fixture, selector and question). Same mechanism the spec itself prescribes for m5.
  5. **m5** "cap off by one low": a cap of 15 does not change this fixture (its first 16 blocks are identical; measured and asserted), so the caught case is 16 -> 17 (the divergence disappears). Both are run and asserted; the 15 case is caught by the library's own unit tests, not by this fixture.
  6. **m3** is implemented as inserting each block at the front of the block list (so the last file block becomes `exif()`'s primary): the harness model reads `exif_blocks()` independently of `exif()`, so a change confined to `exif()` would be invisible to it.
  7. `KNOWN_DIVERGENCES` grew a wildcard selector `exif:*` (`oracle::EXIF_ANY`) for "ExifTool reads none of the Exif, ours reads it" instead of 20 near-identical entries per fixture; the wildcard covers only mismatches of the shape ours=Some, theirs=None and only Exif value selectors (tested). The seed `exif-after-scan` has its own entry.
  8. The fuzz seeds are compared on the 20 Exif selectors only; the two region presences are fixture-only. Reason, measured: the `iptc-8bim` seed has a `0x0404` resource whose bytes are not an IIM `ObjectName`, so "any IPTC region" vs `IPTC:ObjectName` would be an unexplained mismatch that says nothing about the library. Seed records are named `jpeg-seed-<name>` and carry the 20-selector command; fixture records carry the 22-selector command.
  9. `tools::temp_file_with_ext` (and `temp_file_for`) now put a per-process counter in the temp file name. Two live tests that ran the same fixture name concurrently deleted each other's file (seen once as a spurious failure of `live_container_independence_against_exiftool`); fixed. The TIFF behaviour and records are unchanged.
  10. The "existing 63 seeds byte-identical (hash list)" test is `the_existing_63_seeds_are_byte_identical`; the hash is a hand-written FNV-1a 64 plus length (std has no fixed-output hash), and the cross-check of sha256 values was done in the shell before changing anything.
  11. Question ids: the spec asks for `c2pa-in-jumbf`; the existing open `c2pa-detection-inside-jumbf` (SPEC-010) is that question and was left as is rather than duplicated. The existing `exiftool-several-exif-segments` is marked answered (the measurement) and the spec's `several-exif-segments` is added as the open library-policy question. Also added: `exiftool-ignores-exif-after-scan`, `exiftool-drops-exif-when-last-segment-ends-at-eof`, `exiftool-reads-17th-exif-block`, and the extra `exiftool-warnings-only-when-no-tag-is-found`; `guidance/signals.yaml` gets `oracle-has-no-sbyte-selector`; `guidance/toolchain-brief.md` got the JPEG oracle paragraph and the 37-seed count.
  12. Fuzz red-proof order: the planted bug fires at the `kind` re-derivation, not at the iff assertion below it (above). The iff is additionally exercised in both directions by `jpeg_harness_five_byte_iff_both_directions` and in the shared harness `check` via `exif_by_signature`.
  13. Wall-clock kill is `Child::kill` + `wait` (as `run_tool_with_limit`); std offers no process-group kill, so a `cargo build` timed out at the limit may leave its `rustc` child to finish on its own.
  14. `just lint` includes `cargo fmt --all -- --check`; the fuzz graph's formatting and the `include!`d shared file were checked separately (above), as `just lint-fuzz` is clippy only.
- **Follow-up work identified:**
  - The audit stage must decide `several-exif-segments` (merged view) and what to say about an Exif block after the scan or at EOF.
  - A way to capture ExifTool warnings when values were found (`-Warning` selector? `-a`?) so the `# stderr:` characterisation is not limited to all-`-` inputs.
  - An SBYTE selector (signals.yaml).
  - `lint-fuzz-red-proof` / CI: `git diff --exit-code -- tests/oracle-fixtures/exiftool` in the `oracle-records` job does not see NEW untracked files; it will catch changes to the committed JPEG records once they are committed, but a record that is never committed is invisible. Not changed (CI is outside this spec).

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — Three things. The spec lists divergences as `KNOWN_DIVERGENCES` entries keyed by fixture and selector, but "ExifTool reads none of the Exif" is twenty selectors; I had to invent the wildcard. "m5 off by one low" cannot be caught by the named fixture. And the table's stray-byte row did not say which segment precedes the stray bytes, which turned out to decide whether ExifTool warns at all.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — A statement that ExifTool prints warnings only when no requested tag was found would have changed how the `# stderr:` lines are described (it is now a question). Also worth a rule: live tests that share a temp-file namespace must use unique names (they run in parallel).

3. **If you did this task again, what would you do differently?**
   — Probe the stderr behaviour with and without a readable tag before designing the warning table, and write the records, the comparator and the wildcard divergence in one pass instead of discovering the stale-entry accounting during the mutants.
