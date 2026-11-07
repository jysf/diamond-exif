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
  id: HANDOFF-036
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-06
  status: completed                  # pending | accepted | completed | rejected

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
  status: completed                     # completed | blocked | rejected
  tokens_total: 297136             # orchestrator-recorded from the metered sub-agent results (subagent_tokens)
  estimated_usd: 1.96
  duration_minutes: 52
  branch: worktree-agent-a8294d7bb91892337
  pr: null
  completed_at: 2026-11-06               # YYYY-MM-DD
  notes: "verify PUNCH LIST at 6935f90, delta verify APPROVED at b224a12 (final). Tokens transcribed by the orchestrator from the sub-agent metered results; the agent was resumed once and the harness reported cumulative figures 258,581, 293,641 and finally 297,136 (final, used; the 293,641 recorded earlier was an interim figure); wall-clock per run 755,929 + 2,352,318 = 3,108,247 ms = 52 minutes; the verifier's own 75-minute estimate is not used"
  synced_at: 2026-11-06
---

# HANDOFF-036: Verify the exif-audit binary

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-016` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-016-the-exif-audit-binary-arguments-directory-walk-per-file-isolation-exit-codes-cli-integration-tests-ci-build-job.md` (files, APIs, the 14 behaviour rules, the `cli_args` fuzz target, CI job, named tests and the 29 mutants are normative)
- **DEC-011 (normative for options, defaults, the walk, failure texts and exit codes), DEC-010 with its amendments, DEC-009:** `decisions/`
- **Build handoff and Handback:** `HANDOFF-035-build-...md` (claims to verify, not to trust; read the seven deviations and the two doubts)
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe)
- Code under review: `git diff 8b2dbf5..HEAD` (build commits through `3bde9e7`; `e0fce8d` and later are docs). Main: `src/bin/exif-audit/{main,args,exit,walk,read}.rs`, `tests/cli.rs`, `tests/support/cli_tree.rs`, `tests/golden/usage.txt`, `fuzz/fuzz_targets/cli_args.rs`, `fuzz/shared/cli_invariants.rs`, `fuzz/seeds/cli_args/*`, `tests/fuzz_layout.rs`, `.github/workflows/ci.yml`, `README.md`, `Cargo.toml`.

## Expected Deliverables

Review cold; output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

Verify independently:
- Re-run every gate (fmt both graphs and the harness files, `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `cargo test` on stable and 1.90.0 with `DIAMOND_EXIF_REQUIRE_ORACLE=1 DIAMOND_EXIF_REQUIRE_CLI_FS=1`, `-F unsafe_code` on the lib and the bin, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `oracle-record` with an empty `git status --porcelain`), `just fuzz-smoke cli_args` and `audit_render` 60 s each, `cargo build --release --locked --bin exif-audit`. Confirm: nothing under `src/` outside `src/bin/exif-audit/` changed except test additions in `src/report/builder_tests.rs`; `decisions/`, `scripts/`, existing goldens and seeds untouched; no dependency added (`Cargo.lock` diff); the three edited string literals in `tests/fuzz_layout.rs` are the only existing-expectation edits (judge them).
- **Use the binary as a hostile user would.** Build the release binary and try, by hand, everything DEC-011 promises and some it does not mention: arguments (every option and `=` form, repeats, `--`, `-`, empty-string argument, a path starting with `-`, a 10,000-argument command line, a very long path); the exit-code table cell by cell including precedence; `--help`/`--version` with bad other arguments; files: truncated TIFF, zero-byte file, a JPEG with a thousand APP1 segments, a TIFF with the SPEC-017 amplification shape; directories: symlink loop, symlink to a file inside a walk, symlink to a directory named on the command line, a FIFO (with an alarm), a socket, a directory you cannot read (chmod 000), a file you cannot read, a deleted-while-scanning file if you can race one, depth 64/65, a thousand files, names with newline/quote/comma/semicolon/leading `=`/bidi override/invisible characters/invalid UTF-8 (where the filesystem allows); `> /dev/null`, a closed pipe (`| head -c 1`), `>&-`. Compare every output with the library's render for the same entries. Look for panics (exit 101), hangs, partial output on failure, nondeterminism (run twice, diff), output that depends on the working directory, and any place a hostile name reaches stdout unescaped.
- **Walk and read policy against DEC-011 part 3:** read `walk.rs` and `read.rs` line by line. TOCTOU: the regular-file re-check on the open handle; the size rule (the build checks `metadata().len()` before reading and keeps a bounded read: can a file that grows while read exceed the cap in memory? what happens at exactly `--max-size`, and at one byte over, for a TIFF/JPEG and for a non-image?); sniff before the size cap; the 12-byte sniff on a file shorter than 12 bytes; the depth limit; sorted order (`OsString` order: is it deterministic across the two CI OSes? does the CI test depend on a platform-specific order?); the closed list of failure texts and the `read error` / `i/o error` rule the builder chose.
- **Judge the two doubts and the smaller choices.** (1) A closed stdout descriptor (`>&-`) gives exit 0 or 1, not 4: is that acceptable or a defect? Is there a safe-Rust way to detect it? (2) `read error` vs `i/o error` mapping; upper-case-only size suffixes; `--help`/`--version` exit 0 even if their output cannot be written; `-h`/`-V` found anywhere before `--` (so `--format --help` is Help): argue each against DEC-011 and good CLI practice.
- **Review the CI job** (`.github/workflows/ci.yml`) line by line: YAML validity, action pins as in the existing jobs, the matrix, the commands, the `python3 -c` JSON check, `DIAMOND_EXIF_REQUIRE_CLI_FS=1` in the Linux test job: which Linux-only branches (`/dev/full`, invalid-UTF-8 names, `time -v`, `mkfifo`) have never run on this macOS host, and could any fail on `ubuntu-latest`? Reason it through and, if Docker or another Linux environment is available, run them there.
- **Test the testers.** Re-run the builder's mutants (the report claims 29 red) at least for 2, 5, 11, 12, 13, 17, 18, 21, 22, 23, 27, 28 and 29. Write mutants the builder did not try, for example: `exit_code` returning 3 only when `failed > 0` (ignoring `unreadable`); `--strict` counting `issues` but not `unsupported`; the threshold comparing against `reliable` files only; the sniff-first read reading 11 bytes; a walk that sorts case-insensitively; labels lowercased; depth counted from the wrong base; a FIFO check done by file name; the `cli_args` guard `args_run_shape` checking path count only; a plant that cannot go red because its control also fails; the size cap read from an environment variable; `--values` honoured only for JSON. Report which the suite catches (name the test or guard) and which survive.
- Redo one `cli_args` guard plant against the real libFuzzer target, and one `audit_render` builder plant. Check the usage golden against `--help` and that every default in DEC-011 appears in it. Check the README's usage text and exit-code table equal the binary's behaviour.
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes; stdin input; `--follow-symlinks`; parallel scanning; changing the report content; any ExifTool comparison

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

- **Branch / PR:** branch `worktree-agent-a8294d7bb91892337`; no PR, not pushed. `git merge --ff-only claude/rust-exif-metadata-6351d5` was refused (this worktree started at `main` `d5a3c14`, which has two commits the feature branch lacks: the `specs-by-stage` fix, PR #24); a plain `git merge` made the clean merge commit `b5a4041`. The only file it brings in that the build did not touch is `scripts/specs-by-stage.sh` (from `main`). The code under review is the build at `3bde9e7`; the verdict is given at `6935f90` (the tip of `claude/rust-exif-metadata-6351d5`).
- **Completed at:** 2026-11-06
- **All acceptance criteria met?** Every gate is green and every named test exists. Two problems remain: a FIFO race (F1) breaks DEC-011's rule that a FIFO is "never opened", and the walk's special-file handling is under-tested (F2).
- **For `verify`:** ⚠ **PUNCH LIST** at `6935f90`. One Medium finding (F1), plus Low findings and nits. No High findings. Nothing else blocks the work.

#### Gates (run by the verifier, macOS host)

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check`; fuzz graph `--check`; `rustfmt --edition 2021 --check` on all six `fuzz/shared/*.rs` | clean |
| `just lint` (clippy `--all-targets` 0.1.99, covers `bin "exif-audit"` and its tests) | clean |
| `just lint-fuzz`, `just lint-no-allow` | clean |
| `just lint-red-proof`, `just lint-fuzz-red-proof` | both print the green "proved" line |
| `cargo test` on Homebrew stable 1.99.1 and on `~/.cargo/bin/cargo +1.90.0` with `DIAMOND_EXIF_REQUIRE_ORACLE=1 DIAMOND_EXIF_REQUIRE_CLI_FS=1` | all binaries ok on both (unit bin 19, cli 23, fuzz_layout 52, lib 228, …; 0 failed) |
| `cargo rustc --lib -- -F unsafe_code`, `cargo rustc --bin exif-audit -- -F unsafe_code` | clean; no `unsafe` in `src/bin/` or the target |
| `just deny`, `just deny-fuzz` | licenses ok |
| `just validate`, `just decisions-audit` (0 structural errors), `just cost-audit` | pass |
| `just oracle-record`, then `git status --porcelain` | empty |
| `just fuzz-smoke cli_args` | rc 0, 2,376,192 runs in 61 s, `DONE cov: 861 ft: 2627 corp: 502/48Kb` |
| `just fuzz-smoke audit_render` | rc 0, 38,711 runs in 61 s, `DONE cov: 4780 ft: 13984 corp: 950/233Kb` |
| `cargo build --release --locked --bin exif-audit`, `just dev --version` | ok, `exif-audit 0.0.0` |
| cli `--nocapture` | measured peak 7 MB for 200 x 2,015,350 B (ceiling 100 MB); 1 MB for two 2 GiB sparse non-images; 196 seeds over 9 invocations; skips on this host: the invalid-UTF-8 name (allowed on macOS) and `/dev/full` (NOTE) |

Scope checks: outside `src/bin/exif-audit/`, the only change under `src/` is test additions in `src/report/builder_tests.rs`. `decisions/`, `scripts/`, `Cargo.lock`, the existing goldens and the existing seeds are unchanged in `8b2dbf5..6935f90`. No dependency was added. The three string literals edited in `tests/fuzz_layout.rs` (the CI matrix string, `audit_render, cli_args]`, the `fuzz-smoke` list) are **acceptable**. Each one adds `cli_args` to a string that pins the complete target list, so the assertion still means "every target runs in CI".

#### Findings

- **F1 (Medium): a FIFO swapped in during the race window is opened, and the run hangs.** In `read.rs`, `read_file` calls `File::open` and only then re-checks that the handle is a regular file. Opening a FIFO for reading blocks until a writer appears, so the re-check never runs. In `walk.rs` the checks happen earlier (`symlink_metadata` in `found`, `metadata` in `named`), which leaves a window between the check and the open. I reproduced it with the release binary: one process kept renaming a fresh FIFO and a fresh regular file over `w/x`, while `perl -e 'alarm 2' exif-audit w/x` ran in a loop. **3 hangs in 127 runs.** This is the same DoS that DEC-011 part 3 rules out ("never opened (opening a FIFO can block forever)"), and the README says "never opened". Anyone who can write into a scanned tree can hang a CI job until its timeout. A symlink can be swapped in through the same window and is then followed, which lets `--values` excerpt metadata from a file outside the tree (lower impact). Fix in safe Rust: open with `std::os::unix::fs::OpenOptionsExt::custom_flags(O_NONBLOCK | O_NOFOLLOW)` for entries found in a walk (`O_NONBLOCK` only for named paths). With no `libc` dependency, the constants have to be typed per target under `cfg`. Keep the handle re-check, then read normally (O_NONBLOCK does not affect reading a regular file). Verifier mutant v13, which removes the handle re-check, survives the suite. The fix should come with a unit test that the re-check rejects a non-regular handle; the race itself cannot be tested deterministically.
- **F2 (Low): the walk's special-file test can be fooled by a check on the file name.** Verifier mutant v08 treats a non-regular entry as special only when it is named `pipe` and **survives**: `fifo_is_reported_and_never_opened` uses a single FIFO named `pipe`, and no test puts a socket in a walk. By hand, a socket in a walk, a named socket, `/dev/null` and `/dev/zero` are all reported `not a regular file`. Add a socket and a second FIFO with a different name.
- **F3 (Low, a DEC wording issue, not a code defect): judging doubt 1 (`>&-`).** The build's explanation of the cause is wrong, but its conclusion is right. The Rust runtime's startup code reopens a closed fd 0/1/2 on `/dev/null`. A probe compiled in the scratchpad printed identical `(dev, ino, rdev)` for `/dev/fd/1` and `/dev/null` under `>&-`, and `try_clone_to_owned` of stdout succeeded. So `>&-` is exactly `> /dev/null`: no safe or unsafe check can tell them apart, and exit 0/1 is the only possible behaviour. Close `cli-closed-stdout-descriptor` with a DEC-011 amendment saying so. Related: a small report piped into `| head -c 1` exits 1, not 4, because the write fit in the pipe buffer before the reader closed. A report larger than the pipe buffer exits 4 (measured). The DEC's "including a closed pipe" should say "a pipe closed before the report is written".
- **F4 (Low): two survivors that are hard to kill and do not block.** v14: a directory listing that fails part-way is silently ignored, and no test covers it (it needs fault injection). v11: the size cap is read from an environment variable instead of the option; no test varies the environment. Both are recorded here; no action needed now.
- **F5 (nit): documentation and help text.** (a) README line 152 still says "`exif-audit` bin later". (b) HANDOFF-035's "Return Criteria" paragraph is mangled: line 95 is cut off mid-sentence into `## Handback`. (c) `--help` does not state a default for `--json`/`--csv`. DEC-011 part 1 says every option's default is stated, but these are shorthands whose default follows from `--format [default: text]`. `usage_text_names_every_default` deliberately counts only three `[default: off]`. Either change the DEC wording or add the default to the help text.

#### Judging the other doubts and choices

- **`read error` vs `i/o error`: accept the build's rule.** `not found` and `permission denied` are chosen by error kind. A failing `read` on an open file is `read error`; any other failure (metadata, open, `read_dir`, `symlink_metadata`, ENAMETOOLONG) is `i/o error`. A listing that fails part-way is a `read error` on the directory. The rule is deterministic and never quotes the OS. Write it into DEC-011 part 4 and close `cli-io-error-texts`.
- **Upper-case-only suffixes** (`1k` is a usage error), leading zeros accepted, and `0K` rejected: all consistent with DEC-011's literal "`K`, `M`, `G`". The message is clear (`bad value for --max-size: 1k`). Accepting lower case is optional and could be a later question.
- **`--help`/`--version` exit 0 even when their output cannot be written:** GNU tools exit non-zero here. DEC-011 says only "exit 0". It does not matter for CI. Accept, or note it in the DEC.
- **`-h`/`-V` detected in any argument before `--`** (so `--format --help` is Help, while `--help=x` and `-hV` are unknown options): this is what SPEC-016 specifies, and it is friendlier than reporting a bad value. Accept.
- **Sniff before the size cap** (deviation 4): needed for behaviour rule 10, and it resolves a real tension in DEC-011 part 3 in the safe direction. Rejecting a TIFF/JPEG by its length before reading keeps the bounded `take(max+1)` read as the fallback. A file that grows while being read is still bounded at `max+1` bytes in memory. A file of exactly `max` bytes is read; one byte more is `too_large` (unit and CLI tests, and b21 is caught). Accept, and amend DEC-011.

#### Using the binary as a hostile user (release binary, all compared against JSON/CSV parsed by `python3`)

Every option in both its `=` and two-argument form gave the expected result. So did repeats, conflicts, `--`, `-- -` and `-- --` (each treated as a path, `failed`/`not found`), a lone `-` (2), an empty-string argument (`failed`/`not found`, exit 3), `-dash.jpg` without `--` (2) and with `--` or `./` (scanned), bidi-override, ESC and non-UTF-8 option bytes (escaped in a fixed ASCII message), a 5,000-byte option (cut at 64 chars + `...`), 10,000 arguments (exit 1, 37 MB of valid JSON, 10,000 files), a 300-byte name and a 4,200-byte path (`i/o error`, exit 3). The exit-code table held cell by cell, including precedence (2 > 4 > 3 > 1 > 0). `--help` output equals `tests/golden/usage.txt` byte for byte, and so does the README usage block. The README exit-code table matches the binary's behaviour.

The library handled every hostile file: a zero-byte file (`unsupported`/`empty`, 0), a truncated TIFF (`unreadable`, 3), a 3-byte `II*` file (`unsupported`), an 11-byte JPEG (`issues`), a JPEG with 1,000 APP1 segments (`unreadable`, 3, 2 MB RSS), and the SPEC-017 amplification shape with 511 SubIFDs (771 KB, `issues`, 0.01 s, 11 MB RSS). Directories also behaved: a symlink loop, a dangling symlink, a symlink to a file in a walk (each `symlink not followed`); a named symlink to a directory (followed, labelled as named); a named symlink to a FIFO (`not a regular file`); a FIFO and a socket in a walk; a chmod-000 directory and a chmod-000 file (`permission denied`); depth 64 entered and 65 reported; 1,000 files (sorted, 12 MB RSS, 0.04 s); names with a newline, quote, `=`, a bidi override, a zero-width space and ESC (escaped in text, JSON and CSV; CSV parses with 19 columns per row). `> /dev/null` exits 1. Two runs were byte-identical; the output did not change with the working directory (modulo the label as typed) or with `LANG/LC_ALL/TZ`. No run exited 101 and none hung, except the F1 race.

#### CI job review

The YAML is valid. The actions are pinned exactly like the existing jobs (`actions/checkout@v4`, `dtolnay/rust-toolchain@stable`). The matrix is `[ubuntu-latest, macos-latest]` with `fail-fast: false`. The `&& code=0 || code=$?` pattern is safe under the runner's `bash -e`. The `python3 -c` JSON check is test-only. `DIAMOND_EXIF_REQUIRE_CLI_FS: "1"` is on the Linux `test` job only. No Docker daemon was running on this host, so the Linux-only branches were **not run**; they were reasoned through:
- `/dev/full` gives ENOSPC on write, which `write_all` reports, so exit 4. Rust's stdout swallows only EBADF.
- Invalid-UTF-8 names: ext4 accepts them, and the expectation `\xff\xfe` matches the lower-case escape seen on this host.
- `mkfifo`, symlinks, sparse 2 GiB files on ext4 `/tmp`, and chmod 000 all work as the non-root `runner` user.
- Pipe buffers are 64 KiB, so the 2,500-entry reports overflow them and the closed-pipe case still gives 4.
- `/usr/bin/time -v` prints `Maximum resident set size (kbytes)`, which `peak_from` parses.

The one residual risk is that `/usr/bin/time` might not be installed on `ubuntu-latest`. I believe the runner image ships the `time` package but did not verify it. If it is absent, the peak-memory test fails loudly under `REQUIRE_CLI_FS=1`, which is the right failure mode.

#### Mutants (verifier's driver: exact-once text replacement, suites `cargo test --bin exif-audit`, `--test cli`, `--test fuzz_layout cli_args`, file restored after each; `git status --porcelain` empty at the end)

Builder's mutants re-run, all **caught**:
- **b02** (default fail-on medium): unit `defaults_are_exactly_the_dec_011_table`, `every_option_and_its_equals_form`; cli `fail_on_thresholds_are_inclusive_and_none_disables`; layout harness and plants.
- **b05** (repeated `--strict` accepted): unit `repeated_option_is_a_usage_error`; cli `usage_error_has_empty_stdout_and_a_message_on_stderr`; layout harness.
- **b11** (1 and 3 swapped): four exit unit tests, 15 cli tests.
- **b12** (precedence swapped): unit `incomplete_outranks_findings` and the table; cli `exit_code_for_each_row_of_the_table`, `precedence_…`.
- **b13** (threshold `>`): unit `threshold_is_inclusive` and the table; cli `fail_on_thresholds_…`, `defaults_match_dec_011`.
- **b17** (symlinks followed): cli `symlink_in_a_walk_is_reported_not_followed`, `symlink_loop_does_not_hang`.
- **b18** (non-regular treated as a file): cli `fifo_is_reported_and_never_opened`.
- **b21** (cap off by one): unit `sniff_first_and_size_cap_are_exact`; cli `size_cap_is_exact`.
- **b22** (no sniff first): unit `sniff_first_…`; cli `non_image_is_not_read_whole`.
- **b23** (OS message): unit `failure_texts_are_the_closed_list_…`, walk order test; cli `missing_path_…`, `report_equals_…`.
- **b27** (bytes retained): cli `peak_memory_does_not_grow_with_the_number_of_files` only.
- **b28** (stdout per file): 14 cli tests.
- **b29** (JSON trailing newline): cli `report_equals_the_library_render_in_every_format` only.

Verifier's own mutants:
- Caught:
  - **v01** (incomplete ignores `unreadable`): exit table, cli row table.
  - **v02** (`--strict` counts issues but not unsupported): exit table, cli `strict_turns_…`.
  - **v03** (threshold only when every preview is reliable): cli `defaults_match_dec_011`, row table.
  - **v04** (11-byte sniff): unit `sniff_first_and_size_cap_are_exact` only, through its head-length assertion. No test has a WebP whose magic needs byte 12.
  - **v05** (case-insensitive sort): unit walk order test.
  - **v06** (labels lowercased): cli `output_is_deterministic_and_independent_of_cwd`.
  - **v07** (depth from 1): unit depth test, cli `depth_limit_is_reported`.
  - **v09** (`args_run_shape` compares path count only): layout `each_cli_args_guard_goes_red_alone_when_planted`.
  - **v12** (`--values` only for JSON): cli `report_equals_…`.
  - **v15** (version string): cli `help_and_version_exit_zero`.
  - **v16** (usage message on stdout): cli `usage_error_…`, `precedence_…`.
- **Survived:**
  - **v08**: FIFO check by name (F2).
  - **v11**: cap from an environment variable (F4).
  - **v13**: no handle re-check (F1).
  - **v14**: part-failed listing silent (F4).
- A plant whose control also fails, which could never show its guard red: the harness asserts the control is clean for every plant (`each_cli_args_guard_goes_red_alone_when_planted`), so such a plant would fail the test.

Plants against the real libFuzzer targets, each from a fresh corpus plus the committed seeds, then reverted:
- `cli_args`, `parse_size` accepting a lower-case `k`: stopped at exec #26 in `cli_invariants::check`. It was found from the committed seed `cli-size-lowercase-suffix`.
- `audit_render`, `ReportBuilder::summary()` under-counting `files` by one when more than one file was added: stopped at exec #25 in `render_invariants.rs` `check`.

D1 and D2 (carried from SPEC-018) are present: the end and middle same-length plants, and the Medium file in the max-severity test.

### Cost self-report

- **Tokens (total):** 297,136 (orchestrator-transcribed; final cumulative figure after one resume)
- **Estimated USD:** 1.96 (297,136 tokens at 6.60 USD per Mtok)
- **Duration (minutes):** 52 (harness wall-clock, first review 13 minutes plus delta verify 39; the verifier's own estimate was about 75)
- **Source of the number:** harness report (subagent_tokens)

### Drift and new artifacts

- **New decisions emitted:** none. Proposed DEC-011 amendments for the architect: `>&-` is `> /dev/null` (F3); "closed pipe" means closed before the write; the `read error`/`i/o error` rule; sniff comes before the size cap; the `--json`/`--csv` default wording.
- **Deviations from spec:** none by the verifier. The build's deviations 1 to 8 were judged: all accepted (see above). Deviation 7 (Linux paths not run) is still open here, reasoned through but not executed.
- **Follow-up work identified:** a punch-list fix for F1 (`O_NONBLOCK`/`O_NOFOLLOW` open plus a handle-recheck unit test) and F2 (socket and a second FIFO name in the walk test); F5(a) README row. Optional: a WebP case so the 12-byte sniff is pinned by a CLI test.

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — The brief asked for a fast-forward merge, but the worktree started at `main`, which had moved past the feature branch, so a plain merge was needed. Nothing else.

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — DEC-011 says "repeated on the open handle" but not "opened without blocking". The open itself is the hazard for a FIFO, so the DEC should name `O_NONBLOCK` (and `O_NOFOLLOW` for walked entries).

3. **If you did this task again, what would you do differently?**
   — Start the TOCTOU race probe first: it is the one finding a suite cannot surface, and it decided the verdict.

### Delta verify (follow-up 1: `b00299b` fix, `b224a12` handback)

**Verdict: ✅ APPROVED at `b224a12`.** The merge into this branch was a fast-forward from `8f89dd5`. The review covers `git diff 8f89dd5..HEAD`.

- **F1: closed.**
  - `read_file` now opens through `open_nonblocking`: `OpenOptions` with `read(true)` and `OpenOptionsExt::custom_flags(O_NONBLOCK)` under `cfg(unix)`. The `Ok(m) if m.is_file()` re-check on the handle comes before any read. There is no `unsafe` and no `libc`; `-F unsafe_code` on the bin is clean.
  - The constants are correct:
    - Linux x86, x86_64, arm, aarch64 and riscv64 all use the generic `0o4000`. MIPS (`0x80`), SPARC (`0x4000`) and Alpha (`0x4`) are the ones that differ, and they are correctly left out.
    - macOS, iOS, FreeBSD, OpenBSD, NetBSD and DragonFly all use `0x4`.
    - Every other target, Android included (`target_os = "android"`), gets `0`. That is safe: the code behaves exactly as before.
  - **Race, re-run with the original reproducer (`race.sh`):**
    - Control, the old binary: **3 hangs in 132 runs**.
    - New release binary: **0 hangs in 1,500 runs**.
    - A second harness (`race2.sh`) alternates the named path with a walk: FIFO swap 0 hangs in 1,000 runs (all exit 3); directory swap 0 hangs and 0 wrongly audited in 600.
  - Symlink swap in a walk (no `O_NOFOLLOW`): **18 of 1,500 runs audited the outside target** through a symlink swapped in after `symlink_metadata`. This is the window the amendment documents.
  - **The new unit test `read::open_of_a_fifo_returns_promptly_with_not_a_regular_file` is deterministic.** It opens a FIFO directly, without the walk. If the open blocks, the thread stays blocked for the rest of the test run, and the test fails at 10 s. Mutants:

    | Mutant | Result |
    |---|---|
    | v13 (no handle re-check) | red (unit) |
    | d01 (`custom_flags(0)`) | red (unit) |
    | d02 (macOS constant typed as `0o4000`) | red (unit) |
    | d03 (re-check moved after the first 12-byte read: the FIFO reads EOF and is reported `unsupported`) | red (unit) |
    | d04 (a non-blocking probe open, then a second blocking `File::open` that is read) | red (unit) |

    All five are red only through this unit test.
  - **The probabilistic `fifo_swap_race_never_hangs` (300 runs) is a smoke test, not a guard.**
    - It did **not** go red on d01 (no `O_NONBLOCK`) in this run. Its in-process swapper rarely hits the window.
    - It cannot flake red on correct code: a red needs a real hang.
    - If it does hang, `Cmd::run` kills the child at 10 s and panics before `stop` is set. The swapper thread then spins until the test process exits. That does not hang CI, but it costs CPU.
    - It adds about 2 s. **Low, non-blocking:** keep it labelled as a smoke test. The unit test is what holds the fix.
- **F2: closed.** `fifo_is_reported_and_never_opened` now also has `dir/queue.jpg` (a FIFO) and `dir/sock` (a Unix socket, bound in `/tmp` and renamed into the tree), and v08 is red there. One nit: the rename from `/tmp` into `temp_dir()` would fail with EXDEV if `TMPDIR` were on another filesystem. It is fine on both CI runners.
- **F5: closed.**
  - README row fixed.
  - `--help` now shows `[default: off]` on `--json`/`--csv`. `usage_text_names_every_default` counts 5 and checks that both lines end with it.
  - `--help` output and the README usage block both equal the updated `tests/golden/usage.txt` byte for byte.
  - HANDOFF-035's Return Criteria paragraph is restored.
- **v04: killed.** `sniff_first_reads_exactly_twelve_bytes` (cli: `RIFF…WEBP` at 32 and 12 bytes, plus an `ftyp` HEIC) is red together with the unit test.
- **Scope:** under `src/`, the diff touches only `src/bin/exif-audit/{args,read}.rs`. The only golden that changed is `usage.txt`. `decisions/`, `scripts/`, `Cargo.lock` and `fuzz/` are unchanged.
- **Gates, all green:**
  - fmt on both graphs and the six harness files; `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-fuzz-red-proof`.
  - `cargo test` (stable, `DIAMOND_EXIF_REQUIRE_ORACLE=1 DIAMOND_EXIF_REQUIRE_CLI_FS=1`): bin unit 20, cli 25, all ok.
  - `just fuzz-smoke cli_args`: rc 0, 2,233,868 runs in 61 s, cov 878.
  - Release build.
  - The CI job's commands by hand: `--version`; 2-file JSON exit 1 and valid; `--fail-on none` 0; missing path 3; `--nope` 2.
  - `just validate`; `git status --porcelain` empty.
- **The DEC-011 amendment (item 3, not yet committed; read from the architect's checkout):** the **conclusion is acceptable, but the stated reason is not sound.**
  - "Its value differs per architecture" is equally true of `O_NONBLOCK`, which the fix typed per target anyway. `O_NOFOLLOW` could be typed the same way: `0o400000` on Linux x86/x86_64/riscv64, `0o100000` on Linux arm/aarch64, `0x100` on macOS and the BSDs.
  - A portable std-only alternative also exists: compare the handle's `(dev, ino)` from `MetadataExt` with the walk's `symlink_metadata`, and report a mismatch.
  - The honest reason to accept the window: winning it requires running code concurrently on the scanning host. A party who can do that can usually read the file directly. A symlink that is already in the tree is always reported, and `--values` excerpts are cut at 64 bytes.
  - I recommend rewording the amendment that way and adding a question for the `(dev, ino)` check. **Low, non-blocking.**
