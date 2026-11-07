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
  id: HANDOFF-032
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-06
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-015

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

# HANDOFF-032: Verify the text, JSON and CSV renderers and the audit_render fuzz target

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-015` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-015-text-json-and-csv-renderers-and-the-audit-render-fuzz-target-well-formed-on-hostile-input.md` (API, rules 1 to 9, JSON schema, CSV columns, checkers, fuzz guards, named tests and mutants are normative)
- **DEC-010 and its 2026-11-06 amendment:** `decisions/DEC-010-report-rendering-contract.md` (normative); **DEC-009** with its amendment (dispositions, `reliable`, wording)
- **Build handoff and Handback:** `HANDOFF-031-build-...md` (claims to verify, not to trust; read the seven deviations, the follow-up section, and the builder's doubts)
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe)
- Code under review: `git diff 105a723..HEAD` (build commits through `61817f2`; `dc0362b` and the brief commit touch only docs). Main: `src/report/{mod,safe,text,json,csv,render_tests}.rs`, `src/audit/mod.rs` (`Item::value`), `fuzz/shared/{render_check,render_invariants,audit_invariants}.rs`, `fuzz/fuzz_targets/audit_render.rs`, `tests/{report_render,report_formats,render_check,fuzz_layout}.rs`, `tests/golden/*`, `tests/support/report_inputs.rs`, `examples/fuzz_seeds.rs`, `fuzz/seeds/audit_render/*`, the changes to the inventory oracle suite.

## Expected Deliverables

Review cold; output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

Verify independently:
- Re-run every gate (fmt both graphs and the three harness files, `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `cargo test` on stable and 1.90.0 also with `DIAMOND_EXIF_REQUIRE_ORACLE=1`, `-F unsafe_code`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `oracle-record` with an empty `git status --porcelain`). Run `just fuzz-smoke audit` and `audit_render` 60 s each; record executions and coverage. Confirm `audit()` results are unchanged (`Item::value` additive; `git diff 105a723..HEAD -- src/audit`), `decisions/` and `scripts/` untouched apart from the architect's DEC-010 amendment commit, no dependency added.
- **Judge DEC-010 and the escaping as a security reviewer.** Build or find inputs that the renderers let through unsafely, for each format: a terminal escape sequence, a bidi override, a zero-width or invisible character, a Unicode "tag" character, a very long label, a label that ends a CSV quote early, a JSON-breaking sequence, a formula (`=`, `+`, `-`, `@`, TAB, CR, also after leading spaces or quotes), a NUL, a lone surrogate-looking byte sequence, overlong UTF-8. Say which `safe_text` lets through and whether that matters. Is `safe_text` really injective and total?
- **Check the code against DEC-010 and the spec by hand**: the JSON key order and key set at every level, the CSV header and 19 fields, the five statuses (rebuild the status table from the DEC, then compare), the value-free default (plant your own payload and look in all three outputs, with `values` off and on), the excerpt rule (ASCII up to NUL, 64-byte cap, `truncated`), the preview residuals null unless `reliable`, the summary and the new `residual_findings` rule, the CSV row rules (including `inventory` mode and a file with no findings), the text's required content and forbidden phrases.
- **Test the checkers and the testers.** The independent JSON and CSV checkers are the proof of well-formedness: try to make each accept something invalid (RFC 8259 and RFC 4180 corner cases beyond the builder's table) and reject something valid. Re-run the builder's mutants (the report claims 31 red) at least for 1, 5, 8, 9, 15, 16, 17, 22, 25, 26 to 30 and 31. Write mutants of your own that the builder did not try, for example: JSON writer emitting a raw `\u2028`; a number rendered with a sign or exponent; `container` null for a supported file; CSV guard applied to the `kind` column; the `inventory` flag ignored in text only; excerpt cap applied after escaping in one format only; a status computed from findings instead of health; `summarize` skipping Failed files; a golden file edited to match a bug; a guard that checks one direction of an iff only; the wall-clock linearity bound made vacuous. Report which the suite catches (name the test or guard) and which survive.
- **Judge the deviations.** (2) Adding two `audit` seeds broke the inventory oracle suite and the builder updated counts and lists and **added a pinned Class R divergence** for `seed-audit/audit-full-house-jpeg` plus 28 lines in `tests/oracle-fixtures/exiftool-inventory/seed-audit.txt`: re-run ExifTool yourself on that seed and decide whether the pin is exact and right-sided, not a loosening, and that no existing record or pin changed. (3) the `FailReason` bits; (4) the unreachable `other` IFD name; (5) the wall-clock linearity bound of 55 ms (22 times margin): is it stable, can it flake, is it meaningful; `H_PRESETS` etc. made `pub`. (7) excerpts past the 65,536 item cap.
- Redo one render-guard plant against the real `audit_render` libFuzzer target, and confirm the SPEC-014 F1 plant (mutant 9) is found by the real `audit` target within the smoke run. Check the golden files are what the renderers produce today and that the seeds have the shapes they claim. Check the largest output at the caps (the builder reports 72 MB JSON) and whether any path can produce more.
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes; SPEC-016 work (the binary, I/O, exit codes); value decoding; tag names; any ExifTool comparison of report content

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
