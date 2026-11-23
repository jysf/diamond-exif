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
  id: HANDOFF-085
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-23
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-033

project:
  id: PROJ-001
  stage: STAGE-009
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

# HANDOFF-085: <Task Title — same as the spec's title>

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-033` to a fresh `claude-opus-5-5` (verifier) for the **verify** cycle
of PR #57 (branch `feat/spec-033-rights-category`, built by `claude-sonnet-5-5` under HANDOFF-053). You did not
write the spec or the code. Review it cold.

## Context the Receiving Agent Needs

Read in this order. You share no memory with the architect or the builder.

### Primary

- **Spec (the contract):** `./projects/PROJ-001-diamond-exif-m1/specs/SPEC-033-dec-008-amendment-a-rights-category-for-copyright-kept-by-al.md`.
  Read its "Revision (2026-11-22): PNG and WebP" paragraph: the scope covers JPEG, TIFF, PNG and WebP.
- **The DEC amendments (normative):** the bottom sections of `./decisions/DEC-008-…`, `DEC-009-…`, `DEC-013-…`
  (2026-11-21) and `DEC-020-…`, `DEC-021-…` (2026-11-22). `DEC-022` (WebP strip) applies DEC-013's Exif rule
  unchanged.
- **Build handoff and its handback:** `./projects/PROJ-001-diamond-exif-m1/handoffs/HANDOFF-053-…md`. The handback
  states what the builder claims: its gate results, its deviations and its grep list. Check every claim; do not
  trust any of them.
- **Stage:** `./projects/PROJ-001-diamond-exif-m1/stages/STAGE-009-keep-the-copyright-notice-under-allpii.md`
- **Toolchain brief:** `./guidance/toolchain-brief.md` (the `RUSTUP_TOOLCHAIN=1.90.0` lint quirk, the nightly fuzz
  invocation, ExifTool 13.55).

### Decisions that apply

- `DEC-008`: Copyright 0x8298 is `Rights`, Medium. `Authorship` means "names a person". No severity changes.
- `DEC-009`: `Rights` is removed by `All` and `Everything` only. May-hold rows for Thumbnail, Xmp, Iptc, Jumbf,
  EmbeddedImage and OtherSegment. No other preset disposition changes.
- `DEC-010`: the reports name the category `rights`.
- `DEC-013`: EXIF9 becomes EXIF10. `All` still removes Exif segments whole. Artist and Copyright sharing bytes
  under `AllPii` is `SharedBytes`.
- `DEC-020`, `DEC-021`: the PNG `Copyright` text keyword is `Rights`. The PNG strip keeps it and the `eXIf`
  Copyright under `AllPii`. A `Rights`-only `Custom` set removes carriers.

### Constraints that apply

From `./guidance/constraints.yaml`: `test-before-implementation`, `panic-free-library`, `forbid-unsafe-code`,
`oracle-must-be-shown-red`, `fuzz-target-per-surface`, `exiftool-never-linked`,
`no-new-top-level-deps-without-decision`, `one-spec-per-pr`.

### Prior related work

- `HANDOFF-083` (SPEC-038 verify): the most recent verify in this repo, with the punch-list format and how
  findings were graded H/M/L.
- `SPEC-023` / PR #36: the last category amendment. A fuzz model in `fuzz/shared/rewrite_invariants.rs` was
  missed there, and only CI's smoke caught it.

## Expected Deliverables

Check out `origin/feat/spec-033-rights-category` (PR #57). Do not push code fixes. Commit only your handback
on that branch: this file, with the SPEC-033 timeline's verify line ticked. Then report.

### If `cycle: verify` — the verify contract

You are **not** the agent that implemented this. Review it cold.

1. **Every acceptance criterion**, one by one, with the evidence (a test name and its run, or a command and its
   output). Run `just test`, `RUSTUP_TOOLCHAIN=1.90.0 just lint`, `just lint-fuzz`, `just deny` and
   `just mutation-oracle` yourself.
2. **Completeness grep.** Run `grep -rn "exif9\|EXIF9\|all9\|exif_set\|Authorship\|Copyright\|copyright"` over
   `src`, `fuzz/shared` and `tests`. Every Exif-set copy must hold ten categories, and every Copyright row must say
   `Rights`, unless it is `COPYRIGHT`-style free text or an XMP or IPTC field (both non-goals). Compare the result
   with the builder's list.
3. **The model copies are independent.** The fuzz models and `verify.rs`'s list must be typed by hand, not call
   the library's function.
4. **Record diff discipline.** In `git diff origin/main -- tests/oracle-fixtures`, the strip records may change
   only in the `## all_pii` and `## authorship` sections (a Copyright line appears) and the new `## rights`
   sections. The inventory record may gain only the new seed. Anything else is a finding.
5. **No other disposition moved.** Spot-check the preview grid and `max_severity` against the DEC-009 amendment's
   claim that only Copyright's disposition changes.
6. **Red-proofs (oracle-must-be-shown-red).**
   - Confirm that the committed comparator red-proof ("comparator is red when copyright is removed under
     all_pii") exists and fails when it should.
   - Then do the **library red-proof** in a scratch copy, never committed: put 0x8298 back in the `Authorship`
     arm of `by_id`, and the PNG `Copyright` keyword row back to `Authorship`. Confirm that the replay strip
     oracle test, the `tests/strip.rs` AllPii test, the PNG strip test and the WebP strip test each go red.
   - Record which tests went red.
7. **Live and fuzz.**
   - Run `DIAMOND_EXIF_REQUIRE_ORACLE=1 cargo test --test strip_png_live --test strip_webp_live` and the
     ExifTool strip oracle live.
   - Run `just fuzz-smoke` for `audit`, `audit_render`, `rewrite_jpeg`, `rewrite_tiff`, `rewrite_png` and
     `rewrite_webp` (60 s each). Run them in the background; they are slow.
8. **Docs.** Check the CHANGELOG `[Unreleased]` entry and the two README edits against the spec's Outputs.
9. **The handback** is answered honestly, and its cost basis is stated.
10. Run `just decisions-audit --changed`.

Grade each finding H, M or L, with file:line and a concrete failure scenario. Output exactly ONE of:
✅ APPROVED (at commit SHA) / ⚠ PUNCH LIST / ❌ REJECTED.

## Out of Scope

- Fixing anything. A punch list goes back to a build fix round.
- Re-deciding the DEC amendments. If one is factually wrong, report it as a finding.
- A CLI option to pick categories, XMP `dc:rights` or IPTC, and widening `StripSpec`.

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
