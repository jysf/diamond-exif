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
  id: HANDOFF-089
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
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

# HANDOFF-089: SPEC-040 verify (DEC-012 amendment: a JFIF APP0 thumbnail is a Thumbnail region)

**FIRST STEP:** `git checkout -b claude/spec-040-verify feat/spec-040-jfif-thumbnail`
(its tip is the build `058d243` plus the commit that adds this handoff). Work read-only on the library. Commit only
to this handoff file, on `claude/spec-040-verify`.

## Delegation Summary

`claude-opus-5-5` (architect) hands SPEC-040 to an independent `claude-opus-5-5` verifier for
the **verify** cycle. A `claude-sonnet-5-5` implementer built it from HANDOFF-088. You did not
write this code; review it cold.

Read: `AGENTS.md` (§12, §15 "During verify"), `guidance/toolchain-brief.md` (its seed counts
predate this change), `guidance/constraints.yaml`, the SPEC-040 spec (Decisions made here,
AC1–AC14, Failing Tests), the DEC-012 and DEC-013 amendments dated 2026-11-24, DEC-009,
DEC-010, DEC-020, and `HANDOFF-088-*.md` (the build brief and the orchestrator-transcribed
handback, which lists the implementer's deviations). The change is
`git diff d74f576 feat/spec-040-jfif-thumbnail-build` (112 files; most are seeds, ExifTool
records and seed-count pins).

Toolchain: `export RUSTUP_TOOLCHAIN=1.90.0 PATH="$HOME/.cargo/bin:$PATH"` and confirm clippy
0.1.90. Fuzz needs nightly. macOS has no `timeout` (use `perl -e 'alarm 8; exec @ARGV' --`).
ExifTool 13.55 is on PATH. Restore `fuzz/Cargo.lock` after fuzz or lint-fuzz runs. Never
commit `decisions/INDEX.md`. Give each historical tree its own `CARGO_TARGET_DIR` (a shared
target dir gave a stale result in the last verify). Leave no background processes. Write
scratch scripts in a scratch directory, not the repo.

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

## What to establish (evidence for each, not opinion)

1. **The gap is closed, shown on real behaviour.** Build your own JFIF inputs (your own
   builder, not the implementer's `strip_shapes` or seed builders): a 2×2 thumbnail; a
   short one (declared 3·X·Y > bytes present); one with bytes past 3·X·Y; 0×0, 2×0, 0×3;
   a header cut short; a bare `JFIF\0`; a JFIF after the Exif APP1; version 2.00; two
   JFIF segments; `JFIF\0` in an APP1. For each, compare `main` (`c5c2231`, red) against the
   tip (green) on: the region list, the audit findings (category, severity, `thumbnail`),
   the JSON/CSV/text report, and `strip` under `Gps`, `GpsAndDevice`, `AllPii`, `All`,
   `Everything`, `none` and `Custom({Thumbnail})`. Check each against the DEC-012 amendment's
   rules and against ExifTool 13.55 (`-a -G1 -s` reads; `-all=` / `-JFIF:all=` writes).
2. **The writer cannot leak or over-cut.** Look for inputs where a JFIF thumbnail survives a
   policy that removes `Thumbnail` with `Ok`, or where a JFIF segment without a thumbnail is
   removed. Ideas: the JFIF segment is the first APP0 and the image's only APP0; JFIF in an
   MPF secondary image or after SOS; JFIF combined with a capped MPF index (DEC-013 refusal
   order); X·Y overflow (255×255); a JFIF segment the walk flags but the audit doesn't
   locate. Confirm `src/write/exif.rs` ignores region findings with `block: None` and that
   check (a) in `src/write/verify.rs` would fail closed on any leak.
3. **No regressions on existing files.** Run every input you can enumerate (the builders in
   `tests/support/*_shapes.rs`, `real_shapes::fixtures()`, and every seed under
   `fuzz/seeds/{jpeg_segments,audit,rewrite_jpeg,rewrite_tiff,tiff_ifd}`) through audit and
   `strip` for the presets plus `none` and each single category, on `c5c2231` and the tip.
   List every changed result. A change on an input that has no JFIF thumbnail is a finding.
4. **The test-suite changes are justified.** Check every changed pin (`tests/fuzz_layout.rs`,
   `inventory_oracle.rs`, `jpeg_oracle.rs`, `audit_model.rs`, `policy_preview.rs`,
   `budget_oracle.rs`, `mutation_oracle.rs`): each must equal the count of added JFIF seeds or
   records, with no existing record or seed changed. Confirm `git diff d74f576 --stat --
   tests/oracle-fixtures fuzz/seeds` shows additions only. Confirm the three fuzz models re-derive
   the rule independently and don't just call the library.
5. **The red-first claims.** Reproduce AC10 (audit change only → `strip(AllPii)` is
   `VerifyFailed { Preview }`; use `acb7ef4`), and AC11/AC12 (red on `c5c2231` or with the
   extraction disabled in a scratch tree). Judge whether `fde1f50` really holds the tests red.
6. **The implementer's deviations** (HANDOFF-088 handback): the AC6 `is_clean` reading (the
   architect accepts it: `clean` is parse health; say whether you agree); `RegionTooShort`'s
   need/have; no Thumbnail-only/`Gps`/`Custom` policy in the strip oracle (is the "kept"
   half of AC11 adequately covered?).
7. **Panic-freedom and constraints.** Check new arithmetic (3·X·Y) and slicing; run
   `just decisions-audit --changed`.
8. **Gates on 1.90.0:** `just lint`, `just lint-fuzz`, `just lint-no-allow`, `just test`,
   `just mutation-oracle`, `just forbid-unsafe`, `just deny`, `just validate`, the oracle tests
   with `DIAMOND_EXIF_REQUIRE_ORACLE=1` (strip_oracle, inventory_oracle, jpeg_oracle,
   audit_oracle), and 60 s smokes `jpeg_segments`, `audit`, `audit_render`,
   `rewrite_jpeg`.

Output exactly ONE verdict: ✅ APPROVED (at commit SHA) / ⚠ PUNCH LIST (numbered, each with
severity H/M/L, evidence and a suggested fix) / ❌ REJECTED. Write it into the `## Handback`
section and `handback.verdict` (approved | punch-list | rejected).

## Budget

About 120 tool calls or 300k context. If you get near either, report what you established
and what you did not, and stop.

## Out of Scope

Fixing anything (report it instead); the deferred question `jfif-bytes-past-the-thumbnail`;
correcting AC6's wording or the toolchain brief (the architect does that at ship); CHANGELOG;
push or PR.

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
