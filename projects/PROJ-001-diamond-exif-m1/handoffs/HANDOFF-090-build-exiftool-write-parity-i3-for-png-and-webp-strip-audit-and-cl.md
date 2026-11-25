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
  id: HANDOFF-090
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-24
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-039

project:
  id: PROJ-001
  stage: STAGE-010
repo:
  id: diamond-exif

# ── THE HANDBACK ────────────────────────────────────────────────────────────
# Filled in by the EXECUTING AGENT before it reports done. This is a required
# part of completing the handoff, not a courtesy.
#
# `tokens_total` is the one field the cost gate reads. Report the REAL number,
# and say in `basis` how you got it:
#   Claude Code   → your session transcript, measured per docs/cost-tracking.md
#                   "Measuring a cycle": count each message.id once, take its
#                   largest output_tokens, include subagents/ (basis: transcript).
#                   Summing every line over-counts ~2×. `/cost` in the desktop
#                   Code tab reports context size, not spend.
#   API           → the `usage` objects, summed (basis: api-usage)
#   another agent → whatever your harness reports as total tokens
# If your platform genuinely exposes NO token count, set tokens_total: null AND
# write why in `notes` — then set `cost.metering_source: none` in
# .repo-context.yaml so the gate stops asking. Do not invent a number.
handback:
  status: null                     # completed | blocked | rejected
  agent: null                      # the model that ACTUALLY ran (message.model) —
                                   #   to_agent above is a prediction from tier_map
  verdict: null                    # verify handoffs: approved | punch-list | rejected
  basis: null                      # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: null               # REAL combined count — what cost-audit reads
  estimated_usd: null              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: null
  branch: null
  pr: null
  completed_at: null               # YYYY-MM-DD
  notes: null                      # if unusual; double-quote it — an unquoted " #" starts a YAML comment
  synced_at: null                  # stamped by `just handback-sync` — do not edit
---

# HANDOFF-090: SPEC-039 build — ExifTool write parity (I3) for PNG and WebP strip, audit and CLI switch-over

**FIRST STEP, before anything else:** `git checkout -b feat/spec-039-png-webp-write-parity-build feat/spec-039-png-webp-write-parity`.
That branch carries the design: the spec, DEC-023, the DEC-020 amendment and this handoff. The harness may have started
your worktree from main.

## Delegation Summary

`claude-opus-5-5` (architect) hands SPEC-039 to `claude-sonnet-5-5` (implementer) for the **build** cycle. There are
**three parts, one agent each**. This handoff names the part you run in the prompt that starts you. Run only that
part, then stop at its commit.

| Part | Acceptance criteria | Ends with |
|---|---|---|
| A | AC1 to AC7: keyword rows, records, judge, differential, exit codes, comparator red-proofs | records committed, `just oracle-record` twice with no diff, AC1 to AC7 green |
| B | AC8 to AC10: shared sweep, PNG type and keyword sweeps, writer mutated-copy red-proofs W1 to W4 | B's tests green, `just mutation-oracle` green |
| C | AC11 to AC15: switch-over, honesty test, errors, README, JPEG `gps`/`all-pii` records, all gates | every gate in AC15 green |

The design is done. The spec's "Decisions made here", **DEC-023** and **DEC-020's amendment of 2026-11-24** are
normative. Do not re-decide them. If the code contradicts them, or a measured count in the spec doesn't reproduce, add
a question to `guidance/questions.yaml`, stop, and report it. Do not re-pin a table to match what you got.

## Context the Receiving Agent Needs

### Primary

Read in this order:
1. `AGENTS.md` §11, §12, §13, §15 ("During build").
2. `guidance/toolchain-brief.md` and `guidance/constraints.yaml`.
3. **`projects/PROJ-001-diamond-exif-m1/specs/SPEC-039-*.md`.** This is the source of truth: the Context's measured
   facts, AC1 to AC15, the Failing Tests, the Notes.
4. **`decisions/DEC-023-*.md`** in full: rules 1 to 6, the D1 to D5 table, the exit codes.
5. `decisions/DEC-020-*.md`: clause 6, clarification 4, and the amendment of 2026-11-24.
6. DEC-021 clause 4, and DEC-022 clauses 4 and 7 plus its measured table.
7. DEC-009 (the preset columns) and DEC-008 (the categories).
8. The stage file `stages/STAGE-010-*.md`.

The code and harness files are listed in the spec's Inputs and Outputs.

### Decisions that apply

- `DEC-023`: the whole I3 apparatus for this spec.
- `DEC-020` + amendment 2026-11-24: the switch-over (1, 2), the keyword rows (3), and `PLTE` as structure (4).
- `DEC-021` / `DEC-022`: the carrier rule and the whole-block rule the judge's exceptions point at; `VP8X` kept, flags
  cleared (D1).
- `DEC-009` / `DEC-008`: the preset columns the judge is typed from.
- `DEC-013`: the self-check that the writer mutants must escape.

### Constraints that apply

- `test-before-implementation`: each test is committed red before its code. AC7 and AC10 need recorded red evidence
  in the handback.
- `oracle-must-be-shown-red`: AC7, AC9 and AC10.
- `exiftool-never-linked`: ExifTool is run, never read. Don't read its Perl source to find subdirectory chunk types.
  The pinned list in AC9 is the answer, and the gap is DEC-023 rule 6's stated residual.
- `no-new-top-level-deps-without-decision`: no new dependency, dev ones included. FNV-1a is about five lines; write
  it.
- `panic-free-library` and `forbid-unsafe-code`: library edits are small (three keyword rows, one `matches!`, two
  `Display` strings). Keep the lint set clean.
- `no-io-in-library`, and `fuzz-target-per-surface` (no new surface; update the `audit` keyword mirror).

### Prior related work

- SPEC-020 and SPEC-021: the JPEG strip oracle, which is the record format to follow.
- SPEC-037 and SPEC-038: the PNG and WebP strips, and their live precursors and sweep.
- SPEC-040: the JFIF fixtures that part C's `gps`/`all-pii` sections check.

## Expected Deliverables

Everything in the spec's Outputs, for your part. Per part:
- **A:**
  - `src/audit/category.rs` and the fuzz keyword mirror.
  - `tests/support/i3_{judge,records,shapes}.rs`.
  - `tests/strip_png_webp_oracle.rs`.
  - `examples/oracle_record.rs` extended.
  - The committed records under `tests/oracle-fixtures/exiftool-strip-{png,webp}/`.
  - `strip_png_live.rs` and `strip_webp_live.rs` importing the shared judge and shapes, with the same tests.
- **B:**
  - `tests/support/sweep.rs` and `tests/strip_png_sweep_live.rs`.
  - The probe strip mode in `tests/support/probe.rs`.
  - The W1 to W4 tests.
- **C:**
  - The switch-over in `src/audit` and `src/report/text.rs`, and the error strings in `src/write/mod.rs`.
  - The goldens, and the updated pins in the tests the spec's Inputs list.
  - The README.
  - `strip_records::policies()` gaining `gps` and `all-pii`, and the re-recorded `exiftool-strip/`.

Mark the timeline's build line `[~]` when part A starts. Each part adds a line in the handback below. After part C, set
the spec's `task.cycle` to `verify` and the timeline line to `[x]`.

## The work, in order (inside each part)

1. Write the part's failing tests (spec "Failing Tests"), and commit them red. If a test can't compile without a new
   item, add a stub that does nothing and say so in the commit.
2. Implement until green.
3. Capture the red evidence the part requires (AC7 cases (i) to (ix), AC9's two red-proofs, AC10's four mutants:
   control green, `strip` `Ok`, replay red, live red) and paste the failing lines into the handback.
4. Run the part's gates on 1.90.0:
   - Every part: `just lint`, `just lint-no-allow`, `just test` with `DIAMOND_EXIF_REQUIRE_ORACLE=1`, and
     `just forbid-unsafe`.
   - A adds: `just lint-fuzz` and a 60 s `audit` smoke.
   - B adds: `just mutation-oracle`.
   - C runs all of AC15.
5. Commit on `feat/spec-039-png-webp-write-parity-build`, with Conventional Commit messages referencing SPEC-039 and
   the trailer `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`. No push, no PR, no CHANGELOG.

Toolchain, recapped from the spec's Notes:
- `export RUSTUP_TOOLCHAIN=1.90.0 PATH="$HOME/.cargo/bin:$PATH"`. Check that `~/.cargo/bin/cargo-clippy --version`
  prints 0.1.90.
- macOS has no `timeout`; use `perl -e 'alarm 8; exec @ARGV' --`.
- BSD `sed -i` needs `''`. Use Python with `-I` for multi-line edits.
- ExifTool 13.55 is at `/opt/homebrew/bin/exiftool`.
- Restore `fuzz/Cargo.lock` after every fuzz or lint-fuzz run.
- Stage files by path, never `git add -A`, and never commit `decisions/INDEX.md`.
- Records contain no machine paths.
- Leave no background processes.

## Budget

About 120 tool calls or 300k context **per part**. If you get near either, stop at a clean commit boundary and report
what is done and what is left. Do not loop. The ExifTool runs are the slow step: batch the files into one ExifTool
invocation per directory where the format allows it, as the WebP sweep does.

## Out of Scope

- A CLI strip command, a release, `set` operations, HEIC.
- A differential for `GpsAndDevice`, `AllPii` or `All`.
- Byte equality with ExifTool's output.
- Changing the writer to match ExifTool on D1 to D5.
- Renaming `StripError::NotJpeg`.
- JPEG or TIFF differentials.
- Any change to DEC-009 presets, `Category`, or `schema_version`.
- Editing ExifTool's answers to make a table pass.

If you think one of these must happen, add a question to `guidance/questions.yaml` and stop.

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
