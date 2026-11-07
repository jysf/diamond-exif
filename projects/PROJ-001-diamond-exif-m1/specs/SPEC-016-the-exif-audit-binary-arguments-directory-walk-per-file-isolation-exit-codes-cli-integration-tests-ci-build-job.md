---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-016
  type: story                      # epic | story | task | bug | chore
  cycle: design                    # frame | design | build | verify | ship
  blocked: false
  priority: medium                 # critical | high | medium | low
  complexity: L                    # XS | S | M | L | XL | XXL — the EXPECTED size, set at design
                                   #   (XL/XXL almost certainly means it's a stage, not a spec)
  complexity_actual: null          # stamped at ship: what it ACTUALLY took, same scale.
                                   #   Expected-vs-actual drift is what `just calibration` reads.
  verify_verdict: null             # approved | punch-list | rejected — the OUTCOME of the verify
                                   #   cycle, stamped by `just advance-cycle` when the spec leaves
                                   #   verify (same three verdicts Prompt 4 already returns).
                                   #   Recorded in front-matter, not just prose, so "verify never
                                   #   rejects anything" stops being a hunch and becomes a number.

project:
  id: PROJ-001
  stage: STAGE-003
repo:
  id: diamond-exif

handoff:
  from_agent: claude-opus-5-5  # from .repo-context tier_map.design (DEC-005)
  to_agent: null                   # filled when HANDOFF is created (any agent — see docs/porting.md)
  created_at: null

references:
  decisions: [DEC-008, DEC-009, DEC-010, DEC-011]
  constraints: [panic-free-library, forbid-unsafe-code, no-io-in-library, fuzz-target-per-surface, test-before-implementation, oracle-must-be-shown-red, no-new-top-level-deps-without-decision]
  related_specs: [SPEC-012, SPEC-014, SPEC-015, SPEC-018]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: [SPEC-015, SPEC-017, SPEC-018]                     # e.g. [SPEC-002]

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-003's <capability>". Optional; null is acceptable.
value_link: "STAGE-003: the first thing a user can run: a read-only privacy scan of files and directories with exit codes a CI job can act on, that never counts an unreadable or unsupported file as clean"

# Self-reported AI cost per cycle. Each cycle (design, build, verify,
# ship) appends one entry to sessions[]. Totals are computed at ship.
# Record a REAL tokens_total for metered cycles (build/verify) — the agent
# that runs the cycle writes it from its own interface (/cost, the API
# usage object, or its tool's report). Only un-metered main-loop cycles
# (design/ship) may be null-with-note. `just cost-audit` enforces this on
# shipped specs. See AGENTS.md §4 and docs/cost-tracking.md. interface:
# claude-code | claude-ai | api | ollama | other.
cost:
  # Optional PREDICTION of the total tokens this spec will take, set at
  # design. Never a gate — its only job is to be compared with the actual
  # below (`just calibration`), so you learn whether you systematically
  # under- or over-estimate. null = didn't predict.
  tokens_estimate: 800000
  sessions: []
  totals:
    tokens_total: 0
    estimated_usd: 0
    session_count: 0
---

# SPEC-016: The `exif-audit` binary: arguments, directory walk, per-file isolation, exit codes, CLI integration tests, CI build job

## Context

Everything before this is library: sniff and audit (SPEC-012), inventory parity (SPEC-013), strip-policy preview (SPEC-014), the three renderers (SPEC-015) and the incremental report builder (SPEC-018). The binary is the product a person runs. It adds `std::fs`, argument parsing and the exit-code contract that CI jobs depend on, and nothing else. DEC-011 (written in this design, normative; read it first) fixes the options **with their defaults**, the walk (sorted, symlinks not followed inside a walk, special files never opened, a size cap, sniff-first reads), the closed list of failure texts, and the exit codes: `0` complete and under the threshold, `1` findings at or above `--fail-on`, `2` usage, `3` incomplete (a file failed or was unreadable, or with `--strict` unsupported or with issues), `4` the report could not be written, with incomplete outranking findings.

Two design choices keep the binary small and provable. First, **the pure parts are separate modules of the binary**: `args.rs` (argument parsing, no I/O) and `exit.rs` (the exit-code function over a `Summary`), so integration tests, unit tests and a new fuzz target reach them without a process. Second, **the binary holds one input file in memory at a time**: each file is read, audited, added to a `ReportBuilder` (SPEC-018), and dropped.

## Goal

`exif-audit [OPTIONS] [--] PATH...` exactly as DEC-011 specifies: one report on stdout in the chosen format with the chosen preview, deterministic bytes, one exit code from the DEC-011 table, a bad or unsupported file a per-file outcome that never aborts the run, integration tests that drive the real binary, a `cli_args` fuzz target over the argument parser, a CI job that builds and smoke-runs the release binary, README usage, and `just dev` working.

## Inputs

- **Reads:** DEC-011 (normative), DEC-010 (statuses, failure wording rules, amendments 1 and 2), DEC-009, DEC-008; `src/report/*` (`ReportBuilder`, `Format`, `RenderOptions`, `FileEntry`, `FileResult`, `FailReason`, `Summary`, `Status`), `src/audit/{mod,preview}.rs` (`audit`, `sniff`, `Sniffed`, `Severity`), `src/policy.rs` (`StripPolicy`), `tests/support/report_inputs.rs` (full-house inputs), `fuzz/shared/render_check.rs` (strict JSON and CSV checkers, `include!`d by the CLI tests), `fuzz/fuzz_targets/audit_render.rs` (target style), `tests/fuzz_layout.rs`, `.github/workflows/ci.yml`, `app.just` (`dev`), `README.md`, `Cargo.toml`.
- **Does not read:** ExifTool. Not run: the CLI contract is this tool's own.
- Constraints `panic-free-library` (the five-lint deny set applies to the binary crate too: put the attributes at the top of `main.rs`), `forbid-unsafe-code`, `no-io-in-library` (the library gains nothing; all I/O is in `src/bin/exif-audit/`), `fuzz-target-per-surface` (arguments are an input surface: new target `cli_args`), `test-before-implementation`, `oracle-must-be-shown-red` (our own tests: the mutants below), `no-new-top-level-deps-without-decision`. No dependencies.

## Outputs

### Files

`src/bin/exif-audit/{main.rs,args.rs,exit.rs,walk.rs,read.rs}` (a multi-file binary target; `[[bin]] name = "exif-audit" path = "src/bin/exif-audit/main.rs"` in `Cargo.toml`, `include` already covers `src/**/*.rs`). `tests/cli.rs` (drives the real binary through `env!("CARGO_BIN_EXE_exif-audit")`), `tests/support/cli_tree.rs` (a temp-directory helper using only `std`), `tests/golden/usage.txt`, `fuzz/fuzz_targets/cli_args.rs` (+ `[[bin]]` in `fuzz/Cargo.toml`, `fuzz/seeds/cli_args/*`, a `fuzz-smoke` matrix entry, `examples/fuzz_seeds.rs` additions), `.github/workflows/ci.yml` (new job), `README.md` (usage section; the status table and status paragraph updated: the CLI exists), `app.just` (`dev` already runs `cargo run --bin exif-audit -- ARGS`; keep it working).

### `args.rs` (pure)

```rust
pub struct Options {
    pub format: Format,              // default Text
    pub policy: Option<StripPolicy>, // default Some(AllPii); `--policy none` gives None
    pub fail_on: Option<Severity>,   // default Some(High); `--fail-on none` gives None
    pub strict: bool,                // default false
    pub values: bool,                // default false
    pub inventory: bool,             // default false
    pub max_size: u64,               // default 256 * 1024 * 1024
    pub paths: Vec<OsString>,        // at least one, in the order given
}
pub enum Outcome { Run(Options), Help, Version, Usage(UsageError) }
pub fn parse(args: &[OsString]) -> Outcome;     // args exclude the program name; total, never panics
pub struct UsageError { /* message: fixed ASCII text, shown through safe_text when it quotes an argument */ }
impl UsageError { pub fn message(&self) -> String; }
pub const USAGE: &str;                          // the --help text, pinned by tests/golden/usage.txt
```

Grammar and precedence are DEC-011 part 1. `--help`/`-h` anywhere before `--` yields `Help` even if other arguments are invalid; otherwise `--version`/`-V` yields `Version`; otherwise the first usage error. Options take `--name value` or `--name=value`. A repeated option, `--json` with `--csv` (or with `--format`), an unknown option, a bad value, `--max-size 0`, an overflowing size, a lone `-`, and no path are usage errors. Everything after `--` is a path, verbatim, in order.

### `exit.rs` (pure)

```rust
pub fn exit_code(summary: &Summary, opts: &Options, write_failed: bool) -> u8;   // DEC-011 part 2, nothing else
```

`incomplete = summary.failed + summary.unreadable (+ summary.unsupported + summary.issues when strict) > 0`. `findings_over = opts.fail_on is Some(t) and summary.max_severity >= Some(t)`. Order: `write_failed` gives 4, `incomplete` gives 3, `findings_over` gives 1, else 0. (`2`, `Help` and `Version` are decided in `main` from the `Outcome`.)

### `walk.rs` and `read.rs` (the only `std::fs` code)

`walk` turns the `PATH` arguments into a lazy, ordered sequence of candidates `(label: OsString/bytes, kind)` per DEC-011 part 3: file arguments in order; directory arguments expanded recursively, entries sorted by name, depth at most 64; `symlink_metadata` for entries found in a walk, `metadata` (follows) for named paths; symlink found in a walk, special file, missing path and depth overrun are `Failed` candidates with the DEC-011 detail text and `FailReason`. `read` takes a regular-file candidate: open, re-check regular on the handle, read at most 12 bytes, `sniff`; for a non-TIFF, non-JPEG container return the unsupported audit of those bytes without reading more; otherwise read the rest with a `take(max_size + 1)` bound and return `Failed/too_large` if the bound is exceeded; map every `io::Error` to the fixed texts (`not found`, `permission denied`, `read error`, `i/o error`) by `ErrorKind`, never by message. `main` loops: candidate, read, `audit` (when bytes), `FileEntry`, `builder.add`, drop; then `finish`, write once to stdout, `exit_code`.

### Behaviour to pin (all from DEC-011; restated as testable rules)

1. Defaults: with no options on a directory of the full-house JPEG, a PNG and a text file, the output is the `text` report with the `all_pii` preview, `fail-on high`, no values, no inventory; the exit code is 1 (the full-house file has GPS and a serial).
2. Report bytes equal the library's: for the same labels and options the CLI's stdout equals `render(format, entries, opts)` over entries built in the test (golden-style, all three formats).
3. Determinism: running the same command twice gives identical stdout; no timestamp, absolute path, locale or env-dependent text; the output does not depend on the working directory (running from the parent versus from inside the directory with the matching relative path gives labels as written).
4. Order: directory entries sorted by name; arguments in order; a file named twice appears twice.
5. Exit codes: one test per row of the DEC-011 table, plus precedence (incomplete with findings gives 3; write failure gives 4 over everything; usage gives 2 with empty stdout and a message on stderr; `--help` and `--version` give 0 with output on stdout and empty stderr).
6. `--strict`: a PNG-only run is 0 without it and 3 with it; a file with `issues` likewise.
7. `--fail-on` thresholds: a Low-only file (a bare timestamp) is 0 at `high` and `medium`, 1 at `low`, 0 at `none`; boundaries inclusive.
8. Walk: symlink in a walk reported `failed` / `not_regular_file` / `symlink not followed` (unix); a named symlink to a file is followed; a symlink loop in a walked tree does not hang; a FIFO in a walked tree is reported `not a regular file` and the run finishes (guarded by a test-level alarm); depth 65 is `directory depth limit`; a missing path is `failed` / `not found` and the other paths still run.
9. Size: a file of exactly `--max-size` bytes is audited; one byte more is `failed` / `larger than --max-size`; `--max-size 1K` parses; a huge sparse or oversized file is rejected without being read whole (the test measures bytes read through a file one byte over the cap by checking the run finishes under an alarm and reports the right text).
10. Sniff-first: a 2 GB-class non-image is not read whole (test with a large file of zeros created sparse where the filesystem allows, else skipped loudly; the run is fast and reports `unsupported`).
11. Hostile names: files named with a newline, a double quote, a comma, a semicolon, a leading `=`, a leading `@`, a right-to-left override and an invisible character (names every filesystem accepts) give JSON and CSV that the strict checkers (`fuzz/shared/render_check.rs`) accept, with the label equal to `safe_text` of the name; invalid-UTF-8 names are tried on unix and skipped loudly when the filesystem refuses them.
12. No panic: the binary run on every committed seed of all fuzz targets written as files, under every format, never exits 101 or by signal; exit codes are in `{0,1,3}`.
13. Memory shape: a directory of 200 copies of a JPEG padded to about 2 MB (400 MB of input in total) is scanned with the child's peak resident memory under 100 MB, measured through `/usr/bin/time -l` (macOS) or `/usr/bin/time -v` (Linux); where that tool is absent the test prints `SKIP: <reason>` (a failure under `DIAMOND_EXIF_REQUIRE_CLI_FS=1`). The build reports the measured peak. A mutant that retains all file bytes until the end (mutant 27) must exceed the ceiling.
14. Write failure: stdout closed (`> /dev/full` on Linux where it exists, or a closed pipe) gives 4, never 0 or a panic.

### Fuzz: `cli_args`

New target `fuzz/fuzz_targets/cli_args.rs` (it includes `src/bin/exif-audit/args.rs` through `#[path]`, so the target runs the real parser): the input is split on `0x00` into arguments, converted with `OsString::from_vec`-style (unix) or lossy, and parsed. Invariants in `fuzz/shared/cli_invariants.rs` (shared with `tests/fuzz_layout.rs`, same pattern as the other harnesses), as named guards run alone, `iff` in both directions, re-derived by a second naive scan of the argument list typed independently:

- `args_total`: returns, never panics.
- `args_help_iff`: `Help` iff an argument `-h` or `--help` appears before the first `--`.
- `args_run_shape`: on `Run`, `paths` is non-empty, equals the non-option tokens (and every token after `--`) in order, and each option value is in its allowed set; defaults hold exactly when the option is absent (re-derived by scanning for the option names).
- `args_usage_message`: on `Usage`, the message is non-empty ASCII with no control character and at most 300 bytes.
- `args_format_conflict`: two format selections never give `Run`.

Plants in both directions (at least two per guard). Seeds in `fuzz/seeds/cli_args/`: at least 16 (defaults, every option, `=` forms, `--`, repeats, conflicts, bad sizes, a lone `-`, hostile bytes, empty). CI `fuzz-smoke` matrix gains `cli_args`; `just fuzz-smoke cli_args` works.

### CI job and README

A new job `rust / exif-audit release build and smoke run` in `.github/workflows/ci.yml` on `ubuntu-latest` and `macos-latest`: `cargo build --release --locked --bin exif-audit`; `./target/release/exif-audit --version`; run the binary on a committed fixture directory (the full-house inputs written by `cargo run --example fuzz_seeds`, or `tests/golden/`) with `--json` and check the exit code is 1 and the output is valid JSON (the strict checker is not available in CI shell; a `python3 -c 'import json,sys; json.load(sys.stdin)'` is acceptable there and is test-only, not a dependency); `--fail-on none` exits 0. Pinned action versions as in the existing jobs. README: a "Using exif-audit" section with the usage text, the exit-code table, two example invocations (a CI gate and a CSV export) and the privacy default; the status table row for `exif-audit` and the status paragraph no longer say "no CLI yet".

## Acceptance Criteria

- The binary behaves as DEC-011 and rules 1 to 14; `parse` and `exit_code` equal the DEC tables cell by cell (hand-typed expectation tables in the tests, not generated from the code).
- Report bytes from the binary equal the library's for the same inputs and options in all three formats.
- `tests/golden/usage.txt` equals `--help` output byte for byte and every option's default appears in it.
- `cli_args` exists with the seeds above; 60 s smoke clean (executions and coverage recorded); one guard plant found by the real target; CI matrix, `just fuzz-smoke cli_args` and the CI release job updated; `lint-fuzz-red-proof` run locally.
- Every mutant below is applied by hand, shown to fail at least one named test or guard (the report names which), and reverted; the tree after reverting is byte-identical to the build commit.
- The library is unchanged except that nothing in `src/` outside `src/bin/exif-audit/` moves; no dependency added; `decisions/` and `scripts/` untouched; all existing tests and goldens pass unchanged.
- Gates: fmt (both graphs and harness files), `just lint` (clippy covers the bin; report it), `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `cargo test` on stable and 1.90.0 with `DIAMOND_EXIF_REQUIRE_ORACLE=1`, `-F unsafe_code`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `fuzz-smoke audit_render` and `cli_args`, `oracle-record` with a clean `git status`, and `cargo build --release --bin exif-audit` with a run of the release binary.

## Failing Tests

Named tests (a rename, fold or missing test is a deviation to report):

**src/bin/exif-audit (unit):**
- `args::defaults_are_exactly_the_dec_011_table`, `args::every_option_and_its_equals_form`, `args::json_and_csv_are_format_shorthands_and_conflict`, `args::repeated_option_is_a_usage_error`, `args::double_dash_makes_everything_a_path`, `args::lone_dash_and_no_path_are_usage_errors`, `args::max_size_suffixes_and_bounds`, `args::help_outranks_version_outranks_usage`, `args::usage_messages_are_fixed_ascii`, `args::usage_text_names_every_default`
- `exit::table_matches_dec_011_cell_by_cell` (hand-typed grid over failed, unreadable, unsupported, issues, severity, threshold, strict, write_failed), `exit::incomplete_outranks_findings`, `exit::write_failure_outranks_everything`, `exit::fail_on_none_never_returns_one`, `exit::threshold_is_inclusive`
- `read::failure_texts_are_the_closed_list_and_never_the_os_message`, `walk::order_is_sorted_by_name_and_arguments_keep_their_order` (on a temp tree)

**tests/cli.rs (the real binary):**
- `defaults_match_dec_011`, `report_equals_the_library_render_in_every_format`, `output_is_deterministic_and_independent_of_cwd`, `exit_code_for_each_row_of_the_table`, `precedence_incomplete_over_findings_and_write_failure_over_all`, `usage_error_has_empty_stdout_and_a_message_on_stderr`, `help_and_version_exit_zero`, `usage_text_matches_the_golden_file`, `strict_turns_unsupported_and_issues_into_exit_three`, `fail_on_thresholds_are_inclusive_and_none_disables`, `symlink_in_a_walk_is_reported_not_followed`, `named_symlink_is_followed`, `symlink_loop_does_not_hang`, `fifo_is_reported_and_never_opened`, `depth_limit_is_reported`, `missing_path_is_failed_and_others_still_run`, `size_cap_is_exact`, `non_image_is_not_read_whole`, `hostile_file_names_give_well_formed_json_and_csv`, `no_seed_makes_the_binary_panic`, `peak_memory_does_not_grow_with_the_number_of_files`, `write_failure_exits_four`

**tests/fuzz_layout.rs and the harness:**
- `cli_args_harness_holds_on_every_seed_and_degenerate_inputs`, `each_cli_args_guard_goes_red_alone_when_planted` (both directions), `cli_args_seeds_are_committed_and_have_the_shapes_they_claim`

## Mutants (each shown red by hand, then reverted)

Args: 1. default `--format` becomes json. 2. default `--fail-on` becomes medium. 3. default `--policy` becomes none. 4. `--json` and `--csv` accepted together. 5. repeated option accepted (last wins). 6. `--` not honoured. 7. lone `-` accepted as a path. 8. `--max-size` default 255 MiB. 9. size suffix `K` read as 1000. 10. `--help` after an invalid option gives a usage error. Exit: 11. codes 1 and 3 swapped. 12. incomplete and findings precedence swapped. 13. threshold exclusive (`>` for `>=`). 14. `--strict` ignored. 15. usage error exits 1. 16. write failure exits 0. Walk and read: 17. symlinks followed in a walk. 18. a FIFO opened. 19. directory entries not sorted. 20. depth limit 65 instead of 64. 21. size cap off by one (cap bytes rejected). 22. unsupported files read whole before sniffing. 23. `detail` uses the OS error message. 24. a missing path aborts the run. 25. hidden files skipped. 26. labels canonicalised to absolute paths. 27. all file bytes retained until the end (builder bypassed). Output: 28. stdout written per file instead of once (the run is partial on a mid-run failure); 29. a trailing newline added to JSON output.

## Carried from SPEC-018 verify (Low, non-blocking)

From HANDOFF-034's delta verify; SPEC-016 extends the same `audit_render` harness and `tests/fuzz_layout.rs`:

- **D1.** Every `builder_equals_oneshot` same-length plant corrupts the first letter, so a guard that compares lengths plus the first 16 bytes still passes. Add a same-length plant that corrupts a letter near the END of the output (and one in the middle), on a non-forward pair; the verifier's mutant d03 (length plus first 16 bytes) must go red.
- **D2.** `builder_summary_max_severity_is_the_maximum_in_both_add_orders` pairs High with Low only. Add a Medium file (for example an Artist) so a swapped Low/Medium ranking (mutant d06) goes red.

## Non-Goals

- Any write or strip; stdin input; `--follow-symlinks`; a config file; colour or a TUI; parallel scanning; a streaming or size-bounded report; `--fail-on-residual`; `exif-pro`, licence gates, telemetry, network. Each open item is a question in `guidance/questions.yaml` (added at this design).
- Any change to the report content, schema or escaping (DEC-010) or to `audit()`.

## Notes for the Implementer

- Order of work, test-first: `args.rs` and its tables, `exit.rs`, the usage text and golden, `read.rs` and `walk.rs` with their temp-tree tests, `main.rs`, `tests/cli.rs` (red against a stub binary first), the `cli_args` target, seeds and harness, the CI job and README, then the 29 mutants.
- SPEC-018 must be shipped first (`ReportBuilder`). If it is not on `main`, stop and report.
- Process-level tests: use `std::process::Command` on `env!("CARGO_BIN_EXE_exif-audit")`; build temp trees with `std` only (a unique directory under `std::env::temp_dir()`, removed at the end, even on failure). FIFOs: `mkfifo` through `Command` on unix (skip loudly if absent); symlinks via `std::os::unix::fs::symlink` under `#[cfg(unix)]`; wrap anything that could hang in a `perl -e 'alarm N'`-style limit or a child-kill timer.
- A skip is loud: print `SKIP: <reason>` and, under `DIAMOND_EXIF_REQUIRE_ORACLE=1`-style strictness, add `DIAMOND_EXIF_REQUIRE_CLI_FS=1` that turns a skipped filesystem feature (FIFO, symlink, sparse file, invalid UTF-8 name) into a failure; the CI Linux job sets it, macOS is allowed to skip the invalid-UTF-8 case only.
- `arithmetic_side_effects`: use `saturating_add`, `checked_*`, iterators; never index a slice or string.
- Keep `main` thin: parse, loop, write, exit. Everything with a decision in it is in `args.rs`, `exit.rs`, `walk.rs` or `read.rs`.
- Report in the Handback: every deviation, each mutant with the test or guard that caught it, the fuzz smoke numbers for `cli_args`, the measured peak memory for rule 13, whether clippy covers the bin target, and any doubt about a DEC-011 rule (a doubt is a question for the architect; do not touch `decisions/`).
- Do not push or open a PR; do not advance cycles.

---

## Reflection

*Appended during **ship**. Three questions, short answers.*

1. **What would I do differently next time?**
   — <answer>

2. **Does any template, constraint, or decision need updating?**
   — <answer — if yes but not done this session, record it in
   `/guidance/signals.yaml`: `type: lesson` (with its N-count) for a recurring
   coding pattern, `type: process-debt` for tooling/process friction. A close
   then forces the decision. See `docs/signals.md`.>

3. **Is there a follow-up spec I should write now before I forget?**
   — <answer>

4. **Where was the worst defect caught?** — one word from a fixed vocabulary so
   the defect-escape distribution is greppable across specs:
   `design` | `build` | `verify` | `ship` | `escaped` (reached prod/runtime) |
   `none` (clean first try).
   — <one word>
   *(Runtime/operational defects — the escape-prone class — only exist once the
   artifact meets its real host. `escaped` here is a signal to strengthen the
   §12 behavioral pre-flight for that surface.)*

5. **What can a user do now that they couldn't before?** — one sentence,
   before → after; quote the confirming number if one exists, name the outcome
   if not. Write `none` if this spec has no user-visible outcome — that is a
   real, greppable result, not a blank. This is the line a downstream work-log's
   `impact` field is transcribed from, and both halves are already written above
   (## Context is the before, ## Goal is the after): confirm the prediction,
   don't reconstruct it from memory.
   — <answer | none>
