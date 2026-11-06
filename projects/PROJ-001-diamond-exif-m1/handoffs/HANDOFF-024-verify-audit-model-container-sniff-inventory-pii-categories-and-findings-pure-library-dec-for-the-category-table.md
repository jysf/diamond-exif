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
  id: HANDOFF-024
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-05
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-012

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

# HANDOFF-024: Verify the audit model

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-012` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-012-audit-model-container-sniff-inventory-pii-categories-and-findings-pure-library-dec-for-the-category-table.md` (API, rules, named tests normative); **DEC-008** incl. its Amendment (the Exif 3.0 ids were corrected after the build's live check; judge the amendment against the ExifTool measurement)
- **Build handoff and Handback incl. Fix round:** `HANDOFF-023-build-audit-model-...md` (claims to verify, not to trust; read deviations D1-D12)
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe)
- Code under review: `git diff 290441d..HEAD -- src fuzz tests decisions` (build commit `9ced293`, fix `3d39354`); main files `src/audit/{mod,sniff,category}.rs`, `fuzz/fuzz_targets/audit.rs`, `fuzz/shared/audit_invariants.rs`, `tests/audit_oracle.rs`, `tests/audit_model.rs`, `tests/support/audit_tags.rs`

## Expected Deliverables

Review cold; output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

Verify independently:
- Re-run every gate (fmt both graphs, `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `cargo test` on stable and 1.90.0 also with `DIAMOND_EXIF_REQUIRE_ORACLE=1`, `-F unsafe_code`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `oracle-record` with a clean `git status`). Run `just fuzz-smoke audit` for 60 s and record executions and coverage (the builder saw ~115 exec/s; is that a harness problem or inherent?).
- **Read `src/audit` against the spec rules and DEC-008 line by line**: sniff (every magic, every prefix, never overreads), the table (compare each row to DEC-008 yourself, not to the test), GPS context-sensitivity (same ids in IFD0 vs GPS), Interop (D3), the IFD1 thumbnail rules (0x0201; strips-only; range check against the blob; `starts_with_ffd8`), regions mapping, pointer tags never findings, multi-block and after-scan handling, inventory cap and what happens to findings beyond it (D1: `Finding.item: Option<usize>`), health/clean semantics (D10: `ExtraExif` makes a JPEG not clean; is that right for an audit?), determinism. Hunt for: panic-capable constructs, arithmetic overflow, unbounded allocation (findings vector bound), work that is superlinear, anything that copies value bytes.
- **Judge the builder's deviations**, especially D1, D3, D5 (spec invariant contradicted DEC-008 for TIFF tags 0x02BC/0x83BB/0x8649/0x8773 mapped to Region categories: is the DEC or the invariant right, and is the model sound?), D7 (an existing test was touched), D10, D12.
- **Test the testers.** Write mutants the builder did not try (table row moved to another category; GPS rule keyed on tag range instead of IFD kind; Interop treated as non-GPS table; `0x0201` accepted outside `Ifd1`; thumbnail range check `<` vs `<=`; `after_scan` dropped; second block ignored; findings dropped past the cap; `clean` ignoring jpeg_issues; sniff accepting a 3-byte TIFF prefix; `RIFF` without `WEBP`; BigTIFF classified as Tiff; severity of a category changed; `Category::ALL` missing a variant; the independent re-derivation in the harness copying a library bug). Report which the suite catches and which survive. Redo at least three fuzz plants including one against the real libFuzzer target; confirm each iff-invariant is both-directional.
- **Re-measure the table-name check yourself** with ExifTool (run it, never read its source): pick the rows that matched, the 9 naming-only rows and the 0xA437-0xA43C rows; is the amendment right? Is any row of DEC-008 still doubtful (ids you can check by building a one-entry TIFF)? Also check there is no remaining id where DEC-008's category is arguably wrong.
- **Re-check the amplification measurement** (`container_amplification_is_measured_and_reported`): reproduce the headline numbers (input size, entries, peak RSS) and say whether the reported cause (overlapping IFDs via SubIFDs, next-words zeroed) is right and whether the audit itself is bounded on that input.
- Check seeds and records: the existing 66 seeds and 120 records byte-identical; the 7 new audit seeds deterministic and each claiming its shape (`xxd`); the new record reproduces; CI change is only `audit` in the `fuzz-smoke` matrix; no dependencies; `scripts/`, irradiance untouched; `decisions/` changes only the DEC-008 amendment.
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes, SPEC-013/014/015/016/017 work, the Container memory fix, CI wiring beyond reading it

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
