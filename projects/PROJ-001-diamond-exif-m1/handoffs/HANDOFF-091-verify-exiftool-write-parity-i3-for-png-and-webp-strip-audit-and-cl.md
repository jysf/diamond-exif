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
  id: HANDOFF-091
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-26
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

# HANDOFF-091: SPEC-039 verify — ExifTool write parity (I3) for PNG and WebP strip, audit and CLI switch-over

**FIRST STEP:** `git checkout -b claude/spec-039-verify feat/spec-039-png-webp-write-parity-build`. Its tip is
the build plus the commit that adds this handoff. Work read-only on the library and the tests. Commit only to this
handoff file, on `claude/spec-039-verify`.

## Delegation Summary

`claude-opus-5-5` (architect) hands SPEC-039 to an independent `claude-opus-5-5` verifier for the **verify** cycle.
Three `claude-sonnet-5-5` implementers built it from HANDOFF-090, one per part (A: AC1–AC7, B: AC8–AC10, C:
AC11–AC15). The architect amended DEC-023 twice during the build. You did not write this code or these decisions.
Review them cold.

**Read first:**
- `AGENTS.md`: §12 and §15 "During verify".
- `guidance/toolchain-brief.md` and `guidance/constraints.yaml`.
- The SPEC-039 spec: the Context's measured facts, Decisions made here, AC1–AC15, Failing Tests, Notes.
- DEC-023 in full, including its amendments of 2026-11-25 (value-level differences, class D6) and 2026-11-26
  (exception (c) covers its converse).
- DEC-020: clause 6, clarification 4, the amendment of 2026-11-24.
- DEC-021 clause 4. DEC-022 clauses 4 and 7.
- DEC-009, DEC-008, DEC-013.
- `HANDOFF-090-*.md`: the build brief, the three part entries, and the orchestrator notes after each. The notes
  list the deviations the architect accepted.

**The change:** `git diff 100b68c feat/spec-039-png-webp-write-parity-build`, 302 files. Most of them are ExifTool
records. `100b68c` is the design commit, now merged into `main`.

**Toolchain:**
- `export RUSTUP_TOOLCHAIN=1.90.0 PATH="$HOME/.cargo/bin:$PATH"`, then confirm clippy reports 0.1.90. Fuzz needs
  nightly.
- macOS has no `timeout`. Use `perl -e 'alarm 8; exec @ARGV' --`.
- ExifTool 13.55 is at `/opt/homebrew/bin/exiftool`. Run it, never read it. Never pass `-m`. Always give it a fresh
  output path.
- Restore `fuzz/Cargo.lock` after any fuzz or lint-fuzz run.
- Never commit `decisions/INDEX.md`.
- Give each historical tree its own `CARGO_TARGET_DIR`.
- Leave no background processes. Write scratch scripts in a scratch directory, not in the repo.

## Decisions that apply

- **`DEC-023` and its two amendments:** records with output digests (rule 1), recorded policies (2), the judge and
  exceptions (a)/(b)/(c) (3), the differential with key-level D1–D5 and value-level D4/D6 (4), exit codes (5), and
  the sweep residual (6).
- **`DEC-020` and its amendment of 2026-11-24:** the switch-over, the per-file honesty note, three keyword rows,
  and `PLTE` as structure.
- **`DEC-021` / `DEC-022`:** the carrier rule, the whole-block rule, and the `VP8X` flag rule. D6 rests on that
  last one.
- **`DEC-013`:** the self-check that the W1–W4 mutants must escape.
- **`DEC-009` / `DEC-008`:** the presets and categories the judge is typed from.

## Constraints that apply

- **`test-before-implementation` and `oracle-must-be-shown-red`:** AC7, AC9 and AC10 carry recorded red evidence
  in HANDOFF-090.
- **`exiftool-never-linked`, `no-io-in-library`, `panic-free-library`, `forbid-unsafe-code`.**
- **`no-new-top-level-deps-without-decision`:** FNV-1a is hand-written.

## What to establish (evidence for each, not opinion)

1. **The records are honest.**
   - Re-record into a scratch copy and diff it against the committed `exiftool-strip-png/`, `exiftool-strip-webp/`
     and `exiftool-strip/`. Expect byte equality.
   - Confirm that replay recomputes `strip` and fails on a digest mismatch: flip one byte of the writer's output in
     a scratch tree.
   - Confirm the records hold no machine paths.
   - Confirm a refused ExifTool write is a `# refused:` section.
2. **The differential is real and its pins are right.** Re-derive the counts over the 174 oracle fixtures with your
   own script, not the implementers' harness. Compare ExifTool's readings of our output and of ExifTool's own
   `-all=` / `-gps:all=` output, at both key and value level. Expected:
   - 157 identical;
   - key level: D1 72, D2 2, D3 7, D4 4, D5 1;
   - value level: D4 3, D6 31;
   - 41 outcome differences and 30 both-refuse.

   Then sample at least one fixture per class and check the class label against the mechanism DEC-023 states for
   it. For D6 especially: is every pinned `WebP_Flags` difference really ExifTool recomputing flags while we clear
   only removed kinds, or is any of them our writer leaving a flag for a chunk it removed (a DEC-022 clause 7 bug)?
3. **The judge cannot pass vacuously.**
   - Check the exception (a)/(b)/(c) conditions in `tests/support/i3_judge.rs` against DEC-023 rule 3 and both
     amendments.
   - Look for a case where an exception excuses a real leak or a real over-cut.
   - Assess the stated limit of the (c) converse: inputs with GPS in both Exif and XMP. Is it as narrow as the
     amendment claims?
   - Build one such input yourself and see what happens.
4. **The writer mutants and sweeps.**
   - Reproduce W1–W4: control green, `strip` returns `Ok`, replay red, live red. Confirm each mutant lives only in
     the test-side probe and never touches `src/`.
   - Reproduce AC9's two red-proofs.
   - Check that the sweep's 29,130 files match AC9's candidate definition (the orchestrator notes explain why this
     is not the Context's 35,020).
5. **The switch-over is honest (AC11–AC13).**
   - Diff `src/` (37 insertions, 34 deletions). Every change must follow from DEC-020's amendment items 1 and 2 or
     from decision 3's neutral `Display` strings.
   - Check every golden and pin change in `tests/golden/report-png-webp.*`, `audit_png_webp.rs`, `report_render.rs`,
     `cli.rs`, `strip_png.rs`, `strip_webp.rs`, `policy_preview.rs` and `fuzz_layout.rs`. Each must be explained by
     the flip.
   - The `fuzz_layout.rs` plants were re-aimed. Confirm each still goes red on its own.
   - Build PNG and WebP inputs yourself and look for a file whose preview says `reliable` while `strip` refuses it
     (AC12).
   - Read the README's new claims against the evidence.
6. **The JPEG `gps`/`all-pii` records (AC14), and the accepted deviation `entry_edit_groups`** in
   `tests/support/strip_records.rs`.
   - Confirm `git diff 100b68c -- tests/oracle-fixtures/exiftool-strip` contains only additions.
   - Judge whether `entry_edit_groups` is sound. Could it hide a writer that wrongly empties a group? Is it typed
     independently of the library?
   - Check the JFIF thumbnail kept under `gps` and gone under `all-pii`.
7. **Panic-freedom and constraints.** Check the library diff. Run `just decisions-audit --changed`.
8. **Gates on 1.90.0:**
   - `just lint`, `just lint-fuzz`, `just lint-no-allow`, `just forbid-unsafe`;
   - `just deny`, `just deny-fuzz`;
   - `DIAMOND_EXIF_REQUIRE_ORACLE=1 just test`;
   - `just mutation-oracle`;
   - `just validate`;
   - 60 s smokes of `audit`, `audit_render`, `rewrite_png`, `rewrite_webp`.

Output exactly ONE verdict:
- ✅ APPROVED (at commit SHA);
- ⚠ PUNCH LIST: numbered items, each with severity H/M/L, evidence and a suggested fix;
- ❌ REJECTED.

Write it into the `## Handback` section and into `handback.verdict` (approved | punch-list | rejected).

## Budget

About 120 tool calls or 300k context. If you get near either, report what you established and what you did not,
then stop.

## Out of Scope

- Fixing anything. Report it instead.
- Byte equality with ExifTool's output.
- Changing the writer to match ExifTool on D1–D6.
- A CLI strip command, `GpsAndDevice`/`AllPii`/`All` differentials, and HEIC.
- CHANGELOG, push, or PR.

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
