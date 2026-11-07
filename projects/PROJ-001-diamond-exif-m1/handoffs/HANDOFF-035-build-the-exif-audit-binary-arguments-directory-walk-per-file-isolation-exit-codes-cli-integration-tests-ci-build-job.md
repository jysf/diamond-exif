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
  id: HANDOFF-035
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-06
  status: completed                # pending | accepted | completed | rejected

task:
  spec_id: SPEC-016

project:
  id: PROJ-001
  stage: STAGE-003
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
  duration_minutes: null
  branch: worktree-agent-ae6b635c23ea0f0af
  pr: null
  completed_at: 2026-11-06         # YYYY-MM-DD
  notes: "tokens_total and estimated_usd null: the implementer agent cannot see its own token count; the orchestrator transcribes the metered figure. duration_minutes null: not timed."
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-035: The exif-audit binary: arguments, directory walk, per-file isolation, exit codes, CLI integration tests, CI build job

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-016` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

Read before starting:

- **Spec (files, `args.rs` and `exit.rs` APIs, the 14 behaviour rules, the `cli_args` fuzz target and its guards, CI job, README, named tests and the 29 mutants are normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-016-the-exif-audit-binary-arguments-directory-walk-per-file-isolation-exit-codes-cli-integration-tests-ci-build-job.md`
- **DEC-011 (normative for every option and its default, the walk, the closed list of failure texts, and the exit codes; read it, do not edit it):** `decisions/DEC-011-exif-audit-cli-contract.md`. DEC-010 and its two amendments (report content, statuses, "label and detail are caller text, never file bytes"), DEC-009, DEC-008 also apply.
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe; macOS `sed -i ''`)
- Code: `src/report/*` (`ReportBuilder`, `Format`, `RenderOptions`, `FileEntry`, `FileResult`, `FailReason`, `Summary`, `Status`, `render`), `src/audit/*` (`audit`, `sniff`, `Sniffed`, `Severity`), `src/policy.rs`, `tests/support/report_inputs.rs` (full-house inputs), `fuzz/shared/render_check.rs` (strict JSON and CSV checkers: `include!` them in `tests/cli.rs`), `fuzz/fuzz_targets/audit_render.rs` and `fuzz/shared/render_invariants.rs` (the harness pattern to copy for `cli_args`), `tests/fuzz_layout.rs`, `examples/fuzz_seeds.rs`, `.github/workflows/ci.yml`, `app.just` (`dev`), `README.md`, `Cargo.toml`
- Prior art: HANDOFF-029 to 034 (SPEC-014, 015, 018 builds and verifies). Lessons that cost follow-ups there and apply here: a timing assertion must be a ratio, never an absolute bound; a guard is only as strong as the seeds that reach it and its plants must vary in kind (length, content at start/middle/end, order), not one dimension; quote every default; a skipped test is loud.
- **SPEC-018 is on `main`** (the builder you need). The spec also carries two Low items from the SPEC-018 verify (section "Carried from SPEC-018 verify", D1 and D2): do them.

Constraints: `panic-free-library` (the five-lint deny set also applies to the binary crate: put the attributes at the top of `src/bin/exif-audit/main.rs`; no unchecked `+`, `*`, `<<`; no indexing), `forbid-unsafe-code`, `no-io-in-library` (the library gains nothing; all I/O is in `src/bin/exif-audit/`), `fuzz-target-per-surface` (new target `cli_args`), `test-before-implementation`, `oracle-must-be-shown-red` (applied to the 29 mutants), `no-new-top-level-deps-without-decision` (no dependencies).

## Expected Deliverables

- Everything in the spec's Outputs and every Acceptance Criterion; every named test under its name (a rename, fold or missing test is a deviation to report). New files: `src/bin/exif-audit/{main,args,exit,walk,read}.rs`, `tests/cli.rs`, `tests/support/cli_tree.rs`, `tests/golden/usage.txt`, `fuzz/fuzz_targets/cli_args.rs`, `fuzz/shared/cli_invariants.rs`, `fuzz/seeds/cli_args/*`; edits: `Cargo.toml` (`[[bin]]`), `fuzz/Cargo.toml` (`[[bin]]`), `.github/workflows/ci.yml` (new release-build job and a `cli_args` matrix entry), `README.md`, `app.just` only if `fuzz-smoke` or `dev` needs it, `examples/fuzz_seeds.rs`, `tests/fuzz_layout.rs`, and the report library only where the D1/D2 items require tests (no library logic change). Existing seeds, goldens and test expectations stay unchanged.
- Order: `args.rs` with its hand-typed tables, `exit.rs`, the usage text and golden, `read.rs` and `walk.rs` with temp-tree tests, `main.rs`, `tests/cli.rs` (red against a stub binary first), the `cli_args` target, seeds and harness, the CI job and README, the D1 and D2 items, then the 29 mutants (for each: control first, apply by hand, show the named failure with its assertion text, revert, `git diff` empty).
- A doubt about a DEC-011 rule is a question for the architect: write it in the Handback and `guidance/questions.yaml`; do not edit `decisions/`.
- Skips are loud (`SKIP: <reason>`), and `DIAMOND_EXIF_REQUIRE_CLI_FS=1` turns a skipped filesystem feature into a failure (set it in the Linux CI job and in your own local run where the feature exists). Anything that could hang (FIFO, symlink loop) runs under a hard time limit.
- Run locally, because they are CI-only otherwise: `just lint-fuzz-red-proof`, `just lint-red-proof`, `just oracle-record` (empty `git status --porcelain` afterwards), `just fuzz-smoke audit_render` and `just fuzz-smoke cli_args` (60 s each; record executions and coverage), `cargo build --release --locked --bin exif-audit` and a run of that release binary exactly as the CI job will run it; confirm one `cli_args` plant is found by the real libFuzzer target; report the measured peak memory for behaviour rule 13 and whether clippy covers the bin target.
- Commit on your worktree branch. **Do not push, do not open a PR, do not advance cycles.** First `git merge --ff-only claude/rust-exif-metadata-6351d5` (you need the commit containing HANDOFF-035); if refused, use `git merge` and say so.
- Do not modify irradiance, `scripts/`, `decisions/`. No dependencies. Scratch work goes in a scratchpad directory, not `/tmp`.
- You cannot see your own token count; set `tokens_total: null` with a note, the orchestrator transcribes the metered figure.

## Out of Scope

- Any write or strip; stdin input; `--follow-symlinks`; config files; colour; parallel scanning; a streaming or size-bounded report; `--fail-on-residual`; `exif-pro`, licence gates, telemetry, network
- Any change to report content, schema or escaping, or to `audit()`
- Pushing, PRs, cycle advancement

## Return Criteria — how to hand back

**Completing this handoff means filling in the `handback:` front-matter block
above AND the `## Handback

### Execution notes

- **Branch / PR:** branch `worktree-agent-ae6b635c23ea0f0af` (worktree `.claude/worktrees/agent-ae6b635c23ea0f0af`); no PR, not pushed. `git merge --ff-only claude/rust-exif-metadata-6351d5` succeeded (fast-forward to `8b2dbf5`).
- **Completed at:** 2026-11-06
- **All acceptance criteria met?** yes, with the deviations and the one DEC-011 doubt listed below. Local gate results: `cargo fmt --all -- --check` (and `cargo fmt --manifest-path fuzz/Cargo.toml --check`, `rustfmt --edition 2021` on both harness files) clean; `just lint` clean; `just lint-fuzz` clean; `just lint-no-allow` clean; `just lint-red-proof` and `just lint-fuzz-red-proof` both print their green "proved" line; `cargo rustc --lib -- -F unsafe_code` and `cargo rustc --bin exif-audit -- -F unsafe_code` clean; `cargo test` with `DIAMOND_EXIF_REQUIRE_ORACLE=1 DIAMOND_EXIF_REQUIRE_CLI_FS=1` green on Homebrew stable 1.99.1 and on `~/.cargo/bin/cargo +1.90.0` (every test binary ok, none failed); `just deny`, `just deny-fuzz`, `just validate`, `just decisions-audit` (0 structural errors), `just cost-audit` pass; `just oracle-record` leaves `git status --porcelain` empty; `cargo build --release --locked --bin exif-audit` and the CI job's commands run by hand on the release binary: `--version` prints `exif-audit 0.0.0`, the two full-house seeds with `--json` exit 1 and parse with `python3 json.load` (2 files), `--fail-on none` exits 0, a missing path exits 3, `--nope` exits 2.
- **Clippy covers the bin target:** yes. `cargo clippy --all-targets -- -D warnings` (the `just lint` recipe and the CI job) compiles `bin "exif-audit"` and `bin "exif-audit" test`; before I fixed them it reported errors in both. The five-lint deny set is the attribute block at the top of `src/bin/exif-audit/main.rs` (it applies to every module of the binary); the only `#[allow]` in the binary are on `#[cfg(test)]` modules, as in the library. `lint-no-allow` is `--lib` only, as before, so it does not look at the bin.
- **Order of work:** args.rs, exit.rs, usage text and golden, read.rs and walk.rs with temp-tree tests, main.rs, tests/cli.rs, the cli_args target, seeds and harness, CI and README, D1 and D2, then the 29 mutants.

### Cost self-report

- **Tokens (total):** null (the implementer agent cannot see its own count; orchestrator transcribes the metered figure)
- **Estimated USD:** null
- **Duration (minutes):** null (not timed)
- **Source of the number:** none available

### Numbers asked for

- **Fuzz smoke, `cli_args`** (`just fuzz-smoke cli_args`, 60 s, exit 0, no crash): the recorded run did 3,001,797 executions in 61 s (`DONE cov: 870 ft: 2695 corp: 491/54Kb`; INITED `cov: 870 ft: 2688 corp: 484`, the corpus directory already held the first run's inputs); the first run did 2,101,260 executions in 61 s. 30 seeds in `fuzz/seeds/cli_args/`.
- **Fuzz smoke, `audit_render`** (`just fuzz-smoke audit_render`, 60 s, exit 0, no crash): 35,942 executions in 61 s, `DONE cov: 4774 ft: 14216 corp: 959/250Kb` (INITED `cov: 3992 ft: 8196 corp: 19/4017b`).
- **One `cli_args` plant found by the real libFuzzer target:** in a copy of `args.rs`, `parse_size` rejected only the literal `"0"` (so `0K` and `00` gave `Run` with `max_size: 0`). From a fresh corpus directory and the committed seeds, libFuzzer stopped on `args_run_shape` ("the naive scan rejects the arguments (bad size) but the parser gave Run(... max_size: 0 ...)") after about 5,400 executions; the plant was reverted and the artifact deleted.
- **Measured peak memory (behaviour rule 13):** the 200 x 2,015,350-byte JPEGs (about 403 MB of input) peak at **5 MB** resident (`/usr/bin/time -l`, macOS; the ceiling is 100 MB). Two 2 GiB sparse non-images with `--max-size 4G`: **1 MB**. Mutant 27 (all bytes retained) measured **412 MB** and failed the test.
- **Skips on this host (macOS, `DIAMOND_EXIF_REQUIRE_CLI_FS=1` set):** `invalid_utf8_file_names_are_escaped` skips loudly (APFS refuses a non-UTF-8 name; allowed on macOS only). `write_failure_exits_four` prints `NOTE: no /dev/full on this host; the closed-pipe case ran`; the `/dev/full` branch is therefore untested here and runs only on the Linux CI. Everything else (FIFO, symlinks, sparse files, chmod 000, `/usr/bin/time -l`) ran.

### Tests (all named tests exist under their names)

Unit (`cargo test --bin exif-audit`, 19 tests): the ten `args::` tests, five `exit::` tests, `read::failure_texts_are_the_closed_list_and_never_the_os_message`, `walk::order_is_sorted_by_name_and_arguments_keep_their_order`, plus two extra (`read::sniff_first_and_size_cap_are_exact`, `walk::depth_limit_enters_64_levels_and_reports_the_65th`). `tests/cli.rs`: the 22 named tests plus `invalid_utf8_file_names_are_escaped` (split out of the hostile-names test so a macOS skip does not hide the rest). `tests/fuzz_layout.rs`: the 3 named tests plus `cli_args_target_is_declared_and_runs_in_ci`. `tests/cli.rs` was run red against a stub `fn main() {}`: 22 of 23 failed (the 23rd is the macOS invalid-UTF-8 skip), then green against the real binary.

### Mutants (each: control green first, applied by hand as a text replacement by a driver kept in the scratchpad, shown red, reverted; `git diff` empty after each)

Control: unit, cli and fuzz_layout `cli_args` suites all green before the first mutant. "unit" is `cargo test --bin exif-audit`, "cli" is `tests/cli.rs`, "layout" is `cargo test --test fuzz_layout cli_args` (the harness sweep and guards).

| # | Mutant | Caught by |
|---|---|---|
| 1 | default format json | unit `args::defaults_are_exactly_the_dec_011_table` ("--format default text left: Json"), cli `defaults_match_dec_011`, layout guard `args_run_shape` |
| 2 | default fail-on medium | unit `defaults_are_exactly_the_dec_011_table`, cli `fail_on_thresholds_are_inclusive_and_none_disables`, guard `args_run_shape` |
| 3 | default policy none | unit `defaults_are_exactly_the_dec_011_table`, cli `defaults_match_dec_011` and `report_equals_the_library_render_in_every_format`, guard `args_run_shape` |
| 4 | json and csv together | unit `json_and_csv_are_format_shorthands_and_conflict`, cli `exit_code_for_each_row_of_the_table` ("json with csv"), guard `args_run_shape` (naive: "two format selections"; the harness sweep reports every guard, and only the first failing seed is printed in the log, so `args_format_conflict` is not separately shown red by this mutant: its own proof is the plants) |
| 5 | repeated option, last wins | unit `repeated_option_is_a_usage_error`, cli `usage_error_has_empty_stdout_and_a_message_on_stderr`, guard `args_run_shape` ("--strict twice") |
| 6 | `--` not honoured | unit `double_dash_makes_everything_a_path`, cli `hostile_file_names_give_well_formed_json_and_csv`, guard `args_run_shape` |
| 7 | lone `-` accepted | unit `lone_dash_and_no_path_are_usage_errors`, cli `usage_error_has_empty_stdout_and_a_message_on_stderr`, guard `args_run_shape` ("lone dash") |
| 8 | max-size default 255 MiB | unit `defaults_are_exactly_the_dec_011_table`, guard `args_run_shape` |
| 9 | suffix K is 1000 | unit `max_size_suffixes_and_bounds` and `every_option_and_its_equals_form`, cli `size_cap_is_exact`, guard `args_run_shape` |
| 10 | `--help` after an invalid option is a usage error | unit `help_outranks_version_outranks_usage`, cli `help_and_version_exit_zero`, guard `args_help_iff` |
| 11 | exit codes 1 and 3 swapped | unit `exit::table_matches_dec_011_cell_by_cell` and three more exit tests, cli `defaults_match_dec_011`, `exit_code_for_each_row_of_the_table` and others |
| 12 | incomplete and findings precedence swapped | unit `exit::incomplete_outranks_findings` and the table, cli `precedence_incomplete_over_findings_and_write_failure_over_all`, `exit_code_for_each_row_of_the_table` ("a failure beside findings") |
| 13 | threshold exclusive | unit `exit::threshold_is_inclusive` and the table (row 10), cli `fail_on_thresholds_are_inclusive_and_none_disables`, `defaults_match_dec_011` |
| 14 | `--strict` ignored | unit `exit::incomplete_outranks_findings` and the table, cli `strict_turns_unsupported_and_issues_into_exit_three`, `exit_code_for_each_row_of_the_table` |
| 15 | usage error exits 1 | cli `exit_code_for_each_row_of_the_table` ("no path"), `precedence_incomplete_over_findings_and_write_failure_over_all`, `usage_error_has_empty_stdout_and_a_message_on_stderr` (no unit test: the code is a constant used by `main`) |
| 16 | write failure exits 0 | unit `exit::write_failure_outranks_everything` and the table (row 27), cli `write_failure_exits_four`, `precedence_incomplete_over_findings_and_write_failure_over_all` |
| 17 | symlinks followed in a walk | cli `symlink_in_a_walk_is_reported_not_followed`, `symlink_loop_does_not_hang` (killed by the 20 s limit) |
| 18 | a FIFO opened | cli `fifo_is_reported_and_never_opened` ("the binary ran longer than 20s and was killed (a hang)") |
| 19 | entries not sorted | unit `walk::order_is_sorted_by_name_and_arguments_keep_their_order`, cli `defaults_match_dec_011`, `report_equals_the_library_render_in_every_format` and others |
| 20 | depth limit 65 | unit `walk::depth_limit_enters_64_levels_and_reports_the_65th`, cli `depth_limit_is_reported` |
| 21 | size cap off by one | unit `read::sniff_first_and_size_cap_are_exact`, cli `size_cap_is_exact` ("--max-size 1024 at.jpg: failed") |
| 22 | unsupported read whole before sniffing | unit `read::sniff_first_and_size_cap_are_exact`, cli `non_image_is_not_read_whole` (a 2 GiB PNG reports `failed` not `unsupported`) |
| 23 | detail uses the OS message | unit `read::failure_texts_are_the_closed_list_and_never_the_os_message` ("No such file or directory (os error 2)" vs "not found"), cli `exit_code_for_each_row_of_the_table` (permission denied), `missing_path_is_failed_and_others_still_run` |
| 24 | a missing path aborts the run | unit `walk::order_is_sorted_by_name_and_arguments_keep_their_order`, cli `missing_path_is_failed_and_others_still_run`, `report_equals_the_library_render_in_every_format` |
| 25 | hidden files skipped | unit `walk::order_is_sorted_by_name_and_arguments_keep_their_order`, cli `output_is_deterministic_and_independent_of_cwd` |
| 26 | labels canonicalised | unit `walk::order_is_sorted_by_name_and_arguments_keep_their_order`, cli `defaults_match_dec_011` and ten more (absolute `/private/var/...` labels) |
| 27 | all bytes retained | cli `peak_memory_does_not_grow_with_the_number_of_files` only ("peak resident memory ... 412 MB", ceiling 100 MB) |
| 28 | stdout written per file | cli `defaults_match_dec_011` and 12 more (report no longer equals the library's) |
| 29 | trailing newline on JSON | cli `report_equals_the_library_render_in_every_format` only |

The SPEC-018 verify items: **D1** adds five builder plants whose corruption is a same-length letter flip at the END (reverse/json, reverse/text; one-shot side of alone-failed/csv) or in the MIDDLE (reverse/csv, alone-audited/json) of a non-forward pair, and a helper `same_length_corrupt_at` that asserts the flipped letter is past byte 16. Verifier mutant **d03** (the guard compares the length plus the first 16 bytes) applied to `g_builder_equals_oneshot`: control green, then `builder_guard_goes_red_alone_when_planted` fails with "same-length corruption at the END of the builder output of reverse/json: the builder guard did not reject the planted bug"; reverted. **D2** extends `builder_summary_max_severity_is_the_maximum_in_both_add_orders` with an Artist (medium) file: Medium-with-Low in both orders gives Medium, and all six orders of High, Medium, Low give High. Mutant **d06** (Low and Medium swapped in `Severity`'s declaration order): control green, then that test fails with `left: Some(Low) right: Some(Medium)`; reverted. The library code is unchanged; only `src/report/builder_tests.rs` and `tests/fuzz_layout.rs` changed for D1 and D2.

### Drift and new artifacts

- **New decisions emitted:** none. Two questions added to `guidance/questions.yaml`: `cli-closed-stdout-descriptor` and `cli-io-error-texts` (see doubts).
- **Deviations from spec:**
  1. **Existing assertions edited (three string literals in `tests/fuzz_layout.rs`).** `audit_target_is_declared_and_runs_in_ci` pinned the exact matrix string `target: [tiff_ifd, jpeg_segments, audit, audit_render]`, and `audit_render_target_is_declared_and_runs_in_ci` pinned `audit_render]` and `audit audit_render"`. Adding `cli_args` to the matrix and to `just fuzz-smoke` (both required by the spec) makes them `... audit_render, cli_args]`, `audit_render, cli_args]` and `audit audit_render cli_args"`. No goldens, seeds or behaviour expectations changed. `app.just`'s `fuzz-smoke` default target list also gained `cli_args`.
  2. **`cli_args` guard set.** The five named guards exist. `args_help_iff` also checks `Version` iff (`-V`/`--version` before `--` and no help), which folds the version rule into the help guard rather than adding a sixth guard. `args_format_conflict` says "two or more format selections before the first `--` must give `Usage`" (the spec's "never `Run`" plus: not `Help`/`Version` either), with its plants all replacing a `Usage` by a `Run`.
  3. **The parser's unit tests also run inside `tests/fuzz_layout.rs`** as `cli_args_src::tests::*` (the layout test includes `args.rs` through `#[path]`, so its `#[cfg(test)]` module compiles there). Harmless duplication.
  4. **Size is checked from the file length first.** For a TIFF or JPEG, `read_file` rejects `len > max_size` from `metadata().len()` without reading (fewer bytes than DEC-011's "at most max-size + 1"), and keeps the bounded `take(max+1)` read as the guard for a file that lies about its length. Sniff comes before the size check, so a non-image of any size is `unsupported`, as behaviour rule 10 requires.
  5. **Tests were not committed red-first for the unit tests** (args/exit/read/walk tests were written together with their modules); `tests/cli.rs` was shown red against a stub binary (22 of 23). The D1/D2 tests were shown red by d03/d06.
  6. **CI job fixtures** are the committed `fuzz/seeds/audit/audit-full-house-{jpeg,tiff}` files rather than the example regenerating them (they are already committed; regenerating would rewrite tracked files). The job also checks exit 3 for a missing path and exit 2 for a usage error. `DIAMOND_EXIF_REQUIRE_CLI_FS: "1"` is set in the existing Linux `test` job.
  7. **Not run locally:** the Linux-only paths (`/dev/full`, an invalid-UTF-8 name on ext4, `/usr/bin/time -v`) and the CI release job on `ubuntu-latest`/`macos-latest`; only the macOS commands were run by hand.
  8. Named-test count: every named test exists under its name; the extras are listed above. `README.md` got a "Using exif-audit" section (usage text copied from the golden, exit-code table, a CI gate and a CSV example, the privacy default) and the status row and paragraph no longer say "no CLI yet".
- **Follow-up work identified:** none beyond the two questions; the mutant driver is not committed (scratchpad only).

### Doubts about DEC-011 (questions for the architect; `decisions/` untouched)

1. **Closed stdout descriptor gives no exit 4.** DEC-011 part 2 and behaviour rule 14 say exit 4 when "the report could not be written to stdout (including a closed pipe)". A closed pipe (EPIPE) and `/dev/full` do give 4 (tested). `exif-audit dir >&-` does not: Rust's std treats EBADF on stdout as success, so the report vanishes and the exit code is 0 or 1 (measured on the release binary). Fixing it needs `unsafe` (forbidden) or a non-portable probe. Question `cli-closed-stdout-descriptor`.
2. **`read error` versus `i/o error` is not defined.** Part 4 lists both texts but not when each applies. I used: `not found` and `permission denied` by kind; any other error after the file is open and a read failed is `read error`; any other error elsewhere (metadata, open, `read_dir`, `symlink_metadata`) is `i/o error`; a directory listing that fails part-way is reported on the directory as `read error`. Question `cli-io-error-texts`.
3. Smaller choices the DEC does not state: the size suffix is upper-case only (`K`, `M`, `G`); leading zeros in a size are accepted (`007`) and `0K` is a usage error; `--help`/`--version` exit 0 even if their output cannot be written; `--help`/`-h` and `--version`/`-V` are looked for in every argument before the first `--`, including what would be an option value (`--format --help` is Help).

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — The DEC leaves `read error` versus `i/o error` undefined, and says nothing about the order of "sniff" against "size cap" (rule 10's 2 GB non-image only works if sniff comes first). Rule 14's "stdout closed" is not testable with `>&-` on Rust std.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — Two existing tests pin the exact CI matrix string and the `fuzz-smoke` recipe string, so adding a fuzz target edits them; the spec says existing expectations stay unchanged. Worth listing.

3. **If you did this task again, what would you do differently?**
   — Commit the unit tests red before the modules, and write the mutant driver first (it also served as the control run) so each module's mutants are checked as the module lands, not at the end.
