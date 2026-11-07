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
  status: pending                  # pending | accepted | completed | rejected

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
  status: null                     # completed | blocked | rejected
  tokens_total: null               # REAL combined count — what cost-audit reads
  estimated_usd: null              # tokens_total × your rate, or your harness's number
  duration_minutes: null
  branch: null
  pr: null
  completed_at: null               # YYYY-MM-DD
  notes: null                      # one line if unusual (rework, no meter, etc.)
  synced_at: null                  # stamped by `just handback-sync` — do not edit
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

- **Branch / PR:** [link]
- **Completed at:** YYYY-MM-DD
- **All acceptance criteria met?** yes/no (if no, explain)
- **For `verify`:** the verdict — ✅ APPROVED (at commit SHA) / ⚠ PUNCH LIST / ❌ REJECTED

### Cost self-report

Mirror what you put in the `handback:` front-matter, and say where the number
came from. **This is the number that lands in the spec** — the orchestrator
transcribes it via `just handback-sync`, it does not estimate it.

- **Tokens (total):** <real number, or null + why>
- **Estimated USD:** <number, or null>
- **Duration (minutes):** <estimate>
- **Source of the number:** `/cost` | API `usage` | harness report | none available

### Drift and new artifacts

- **New decisions emitted:**
  - `DEC-NNN` — <title> (if any)
- **Deviations from spec:**
  - [list]
- **Follow-up work identified:**
  - [any new specs that should be added to the stage's backlog]

### Reflection (3 questions, short answers)

1. **What was unclear in the spec or handoff that slowed you down?**
   — <answer>

2. **Was there a constraint or decision that should have been listed but wasn't?**
   — <answer>

3. **If you did this task again, what would you do differently?**
   — <answer>
