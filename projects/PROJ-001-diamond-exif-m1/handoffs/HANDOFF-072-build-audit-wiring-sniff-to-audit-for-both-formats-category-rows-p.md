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
  id: HANDOFF-072
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
  created_at: 2026-11-22
  status: completed

task:
  spec_id: SPEC-032

project:
  id: PROJ-001
  stage: STAGE-008
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
  status: completed
  agent: claude-sonnet-5-5
                                   #   to_agent above is a prediction from tier_map
  verdict: null                    # verify handoffs: approved | punch-list | rejected
  basis: agent-result                      # transcript | agent-result | slash-cost | api-usage | estimate
  tokens_total: 274471               # REAL combined count — what cost-audit reads
  estimated_usd: 1.81              # per component at `agent`'s rates (docs/cost-tracking.md "Pricing")
  duration_minutes: null
  branch: claude/spec-032-fixes
  pr: null
  completed_at: 2026-11-22
  notes: "tokens left null by instruction: the orchestrator records the harness figure"
  synced_at: 2026-11-22
---

# HANDOFF-072: SPEC-032 verify punch list (fix round)

**FIRST STEP, before anything else:** `git checkout -b claude/spec-032-fixes claude/spec-032-verify`
(the harness may have started your worktree from main).

## Delegation Summary

`claude-opus-5-5` hands the SPEC-032 verify punch list to `claude-sonnet-5-5`
for one fix round. Read first: `AGENTS.md`, `guidance/toolchain-brief.md`,
`decisions/DEC-020-*.md` (AMENDED by the orchestrator in this branch: clauses
3, 5 and 6 are the decisions you implement; do not re-decide them),
`projects/PROJ-001-diamond-exif-m1/handoffs/HANDOFF-071-*.md` (the verify
report with the evidence and the variant grid) and the scratch grid files named
in its Handback if still present (`scratchpad/v032/gen.py`, `table.txt`).

Toolchain: `export RUSTUP_TOOLCHAIN=1.90.0 PATH="$HOME/.cargo/bin:$PATH"` (the
pinned 1.90.0; clippy here differs from CI's stable); fuzz needs nightly; macOS
has no `timeout` (`perl -e 'alarm 8; exec @ARGV' --`); BSD `sed -i` needs `''`.
Never commit `decisions/INDEX.md` or `fuzz/Cargo.lock`. No CHANGELOG entry. No
`src/png` or `src/webp` change.

## The punch list, in order

1. **H1 lint.** `fuzz/shared/audit_invariants.rs:967` `nonminimal_bool` on
   1.90.0. Fix it; run `just lint` under 1.90.0 and make sure it is green.
2. **H2 raw profiles (DEC-020 clause 5).** A non-empty `RawProfile` region
   counts as a failed block in `Health` (status `unreadable`, never clean, never
   `ok`), besides its `OtherSegment` finding. Tests: `zTXt` and `tEXt`
   `Raw profile type exif` with GPS, alone and beside a GPS-free `eXIf`: not
   clean, status `unreadable`, exit 3 in the CLI. Record the GPS raw-profile
   cases in the replay's `DIFFERENCES` only if the audit still differs after
   the change (it should now be explained by the unreadable status: say so).
   Update the `png-raw-profile-not-decoded` question text. Make the fuzz
   harness re-derive the same rule independently.
3. **M1 `tIME` any length (DEC-020 clause 3).** First MEASURE lengths 0 to 12
   with ExifTool (`-a -G1 -s -n`, which print `PNG:ModifyDate` and which do
   not); if ExifTool prints nothing for some length, the audit follows
   ExifTool and the rule and the measured set are pinned in a comment. Audit
   and fuzz harness agree; a unit test per length class.
4. **M3 empty PNG regions (DEC-020 clause 3).** A length-0 PNG `iCCP` with a
   name, a length-0 iTXt XMP, and a `Text` with an empty value each yield
   their finding (WebP unchanged: no finding). Measure with ExifTool first and
   pin; if a case prints nothing, follow ExifTool for that case and say so.
5. **M2 strippable (DEC-020 clause 6).** One helper on `ContainerKind`
   (e.g. `strippable()`: true only for `Jpeg` and `Tiff`); `Preview`,
   `Summary` in `src/report/mod.rs` and the fuzz harness (`strippable_kind`,
   also used by `render_invariants.rs`) use that single definition. Update the
   committed golden `tests/golden/report-full-house.json` deliberately (its GIF
   entry becomes `"strippable":false`; rename the entry's file name from
   `screenshot.png` to `screenshot.gif` in the fixture and all three goldens
   if that is all it takes, L4) and state which goldens changed. Test: GIF,
   HEIC, BigTIFF, empty and unknown files report `strippable` false in JSON.
6. **L1 keywords.** Measure `Artist`, `create-date`, `modify-date`
   (`PNG:Artist`, `PNG:CreateDate`, `PNG:ModDate` per the verifier) in tEXt,
   iTXt and zTXt; add rows (`Artist` Authorship; the two dates Timestamps) to
   the data table with the measurement cited, tests row by row, the keyword
   record and replay updated (`just oracle-record`).
7. **L2, L3 docs and test.** README: presence agrees with ExifTool "except the
   recorded differences"; one line that a WebP whose only GPS is in block 17 is
   `issues` (exit 0 without `--strict`) and why. Make the 17-block test assert
   the specific cap issue (not just not-clean), or rename it to what it proves
   (a second block already costs `ExtraExif`); and add a 16-vs-17 pair that
   distinguishes cap handling.
8. **Gates, on 1.90.0:** `just lint`, `just test`, `just mutation-oracle`,
   `just forbid-unsafe`, `just validate`, `just cost-audit`, `just oracle-record`
   twice (second run no diff), `just fuzz-smoke audit` and
   `just fuzz-smoke audit_render` (60 s each). Commit on
   `claude/spec-032-fixes` with trailer
   `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`. No PR. Leave
   `tokens_total` null in the handback (the orchestrator records the harness
   figure) and list every golden and record file that changed.

## Budget

About 100 tool calls or 250k context. If either is near, stop and report what
is done; do not loop. Do not resume after handing back.

## Out of Scope

SPEC-036 (tag-level inventory parity), `src/png`, `src/webp`, new `Category` or
`Disposition`, strip support, a release.

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

- **Tokens (total):** 274471 (harness subagent_tokens)
- **Estimated USD:** 1.81
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


Orchestrator note: the builder left this prose section unfilled; its report (all punch-list items fixed, gates green on 1.90.0, one live-only GPS raw-profile test, one nameless length-0 iCCP case lost) is summarised in the SPEC-032 reflection.
