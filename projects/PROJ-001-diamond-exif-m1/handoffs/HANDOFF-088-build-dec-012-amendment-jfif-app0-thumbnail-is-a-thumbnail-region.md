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
  id: HANDOFF-088
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-24
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-040

project:
  id: PROJ-001
  stage: STAGE-011
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

# HANDOFF-088: SPEC-040 build — a JFIF APP0 thumbnail is a Thumbnail region, audited and stripped

**FIRST STEP, before anything else:** `git checkout -b feat/spec-040-jfif-thumbnail-build feat/spec-040-jfif-thumbnail`
(that branch carries the design: the spec, the DEC-012 and DEC-013 amendments and this handoff. The harness may have
started your worktree from main).

## Delegation Summary

`claude-opus-5-5` (architect) hands SPEC-040 to `claude-sonnet-5-5` (implementer) for the **build** cycle. The design
is done: the spec's "Decisions made here" and the DEC-012 amendment of 2026-11-24 are normative. Do not re-decide
them. If the code contradicts them, add a question to `guidance/questions.yaml` and stop.

Read first, in this order:
1. `AGENTS.md` (§11, §12, §13, §15 "During build").
2. `guidance/toolchain-brief.md` and `guidance/constraints.yaml`.
3. **`projects/PROJ-001-diamond-exif-m1/specs/SPEC-040-*.md`.** This is the source of truth: the acceptance
   criteria AC1–AC14, the failing tests, and the notes.
4. `decisions/DEC-012-*.md`, **its amendment of 2026-11-24 in full, including the measured table**.
5. `decisions/DEC-013-*.md`: rule 3 and its amendment of 2026-11-24.
6. DEC-008 (the `Thumbnail` category) and DEC-009 (the `Thumbnail` rows).
7. DEC-010, and DEC-020 part 3 (the precedent for additive region kinds).
8. The stage file `stages/STAGE-011-*.md`.

The code and harness files are listed in the spec's Inputs and Outputs.

Toolchain:
- `export RUSTUP_TOOLCHAIN=1.90.0 PATH="$HOME/.cargo/bin:$PATH"`, then confirm `~/.cargo/bin/cargo-clippy --version`
  says 0.1.90 (Homebrew's clippy shadows it otherwise).
- Fuzz needs nightly.
- macOS has no `timeout`; use `perl -e 'alarm 8; exec @ARGV' --`.
- BSD `sed -i` needs `''`. Use Python for multi-line edits.
- ExifTool 13.55 is at `/opt/homebrew/bin/exiftool`.
- Never commit `decisions/INDEX.md`.
- Restore `fuzz/Cargo.lock` after every fuzz or lint-fuzz run (`git checkout -- fuzz/Cargo.lock`).
- Stage files by path, never `git add -A`.
- Leave no background processes.
- Records must not contain machine paths.

## Decisions that apply

- `DEC-012` + amendment 2026-11-24: the JFIF rule (part 1), the narrowed benign list (2), category and finding shape
  (3), presets unchanged (4), and whole-segment removal (5).
- `DEC-013` rule 3 + amendment 2026-11-24: the segment is the unit. A JFIF segment with a thumbnail goes under
  `Thumbnail`.
- `DEC-008` / `DEC-009`: `Thumbnail` is Medium, a block category, removed by `AllPii`, `All` and `Everything`.
- `DEC-010` / `DEC-020`: the region name `jfif_thumbnail` is additive, and `schema_version` stays 2.
- `DEC-001`: a short thumbnail is an issue plus the region of what exists.

## Constraints that apply

- `test-before-implementation`: each test is committed red before its code. AC10 and AC11 need recorded red
  evidence.
- `oracle-must-be-shown-red`: AC11 and AC12 each show a red.
- `panic-free-library` and `forbid-unsafe-code`: the header reads use `get(..)`, and `3·X·Y` uses
  `checked_mul`/`saturating_mul` (at most 195,075, but stay safe anyway).
- `fuzz-target-per-surface`: no new surface. Update the existing models and add seeds.
- `exiftool-never-linked` and `no-io-in-library`.

## The work, in order

1. **Tests red** (spec "Failing Tests"), committed first. Some won't compile until the new variants exist. Add the
   variants as stubs that do nothing (the region is never produced, `region_category` is wired) so the tests compile
   and fail, then say so in the commit.
2. **JPEG layer and audit** (AC1–AC7, AC13). Then **show AC10 red**: `strip(AllPii)` on the 2×2 file returns
   `VerifyFailed { check: Preview }`. Paste the output into the handback.
3. **Writer** (AC8, AC9). Check, don't assume, that the Exif thumbnail selection in `src/write/exif.rs` ignores a
   region finding with `block: None`.
4. **Oracles** (AC11, AC12). Strip oracle fixtures `jfif-thumbnail`, `jfif-thumbnail-short`, `jfif-two-segments` and
   `jfif-no-thumbnail`, live and replay, with the red on `main` recorded. In the inventory oracle, keep
   `JFIF:ThumbnailTIFF` as the region's counterpart and drop every other `JFIF` line. Run `just oracle-record` twice
   with no diff.
5. **Fuzz models and seeds.**
   - `fuzz/shared/jpeg_metadata_invariants.rs`: re-derive the JFIF rule independently from the DEC table.
   - `fuzz/shared/audit_invariants.rs`: map the region to `Thumbnail`, and keep "thumbnail iff `Thumbnail`" true.
   - `fuzz/shared/rewrite_invariants.rs`: in the predicted removals, a JFIF segment with a thumbnail goes under
     `Thumbnail`. Add the `RemovalKind::Jfif` name.
   - Seeds for `jpeg_segments`, `audit` and `rewrite_jpeg` via `examples/fuzz_seeds.rs`. Update the pins in
     `tests/fuzz_layout.rs`, and name every count you change.
6. **Gates on 1.90.0:**
   - `just lint`, `just lint-fuzz`, `just lint-no-allow`, `just test`, `just mutation-oracle`, `just forbid-unsafe`,
     `just deny`, `just validate`.
   - The oracle tests with `DIAMOND_EXIF_REQUIRE_ORACLE=1`: `strip_oracle`, `inventory_oracle`, `audit_oracle`,
     `jpeg_oracle`, `strip_tiff_oracle`.
   - 60 s smokes: `jpeg_segments`, `audit`, `audit_render`, `rewrite_jpeg`.
   - `git diff origin/main -- fuzz/Cargo.lock` must be empty.
7. **Commit and finish.** Commit on `feat/spec-040-jfif-thumbnail-build` with Conventional Commit messages
   referencing SPEC-040 and the trailer `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`. No push, no PR,
   no CHANGELOG (ship adds it). Mark the timeline's build line `[x]`. Set the spec's `task.cycle` to `verify`. Fill
   the handback below.

## Budget

About 120 tool calls or 300k context. If you get near either, stop at a clean commit boundary (after step 2, 3 or 4)
and report what is done. Do not loop.

## Out of Scope

- Bytes past the declared thumbnail, or hidden after a 0×0 header (question `jfif-bytes-past-the-thumbnail`).
- Rewriting a JFIF segment in place.
- JFXX, and `JFIF\0` in an APP1.
- Any preset or may-hold change, or a `schema_version` bump.
- PNG, WebP and TIFF.
- A CLI change.

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
