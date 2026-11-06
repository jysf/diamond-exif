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
  id: HANDOFF-028
  cycle: verify                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-opus-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: verifier             # implementer | verifier
  created_at: 2026-11-06
  status: pending                  # pending | accepted | completed | rejected

task:
  spec_id: SPEC-013

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

# HANDOFF-028: Verify inventory parity against ExifTool

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-013` to `claude-opus-5-5` (verifier, a fresh cold agent) for the **verify** cycle.

## Context the Receiving Agent Needs

- **Spec:** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-013-inventory-parity-against-exiftool-group-tag-id-sets-on-fixtures-comparator-red-proofed.md` (model, classes, outputs, named tests normative)
- **Build handoff and Handback:** `HANDOFF-027-build-...md` (claims to verify, not to trust; read the deviations, especially the new Class R and the ICC `desc` size)
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe)
- Code under review: `git diff 97f11c6..HEAD` (build commits `dd03f0a`, `add150e`, `72f2a23`). Main: `tests/support/inventory.rs`, `tests/support/inventory_corpus.rs`, `tests/inventory_oracle.rs`, `tests/support/probe.rs`, `tests/mutation_oracle.rs`, `tests/audit_model.rs`, `tests/budget_oracle.rs`, `fuzz/shared/audit_invariants.rs`, `examples/oracle_record.rs`, `tests/oracle-fixtures/exiftool-inventory/*`

## Expected Deliverables

Review cold; output exactly ONE of APPROVED / PUNCH LIST / REJECTED at a commit SHA.

Verify independently:
- Re-run every gate (fmt both graphs, `just lint`, `lint-fuzz`, `lint-no-allow`, `lint-red-proof`, `lint-fuzz-red-proof`, `cargo test` on stable and 1.90.0 also with `DIAMOND_EXIF_REQUIRE_ORACLE=1`, `-F unsafe_code`, `deny`, `deny-fuzz`, `validate`, `decisions-audit`, `cost-audit`, `oracle-record` with an empty `git status --porcelain` over both record directories). Run `just fuzz-smoke audit`, `tiff_ifd`, `jpeg_segments` 60 s each; record executions and coverage. Confirm `git diff 97f11c6..HEAD -- src` is empty.
- **Re-measure independently** (do not reuse the builder's sweep): run `exiftool -a -u -G1 -H -s` yourself over the three seed directories and the fixtures (write a scratch dump of `audit()` in the scratchpad) and confirm the identical/divergent split, each pinned divergence's exact diff, and the claim that `two-hop-cycle` is the only non-return. Check that Class R's 13 inputs really are inputs where ExifTool prints no group for a located region, and judge whether Class R is a legitimate recorded divergence or a loosening (a pinned region-kind difference per input, or a rule?).
- **Is any pinned divergence a loosening?** For each `KNOWN_DIVERGENCES` entry check it pins the exact multiset difference, that changing one count or id in the table turns the test red, that a wrong-direction entry is rejected, and that the `ours_hides => !clean` invariant is not satisfiable vacuously. Look for a way an unexplained difference could pass: the `UNMODELLED_GROUPS` list, `DIRECTORY_VALUED` exclusions, dedup-by-bytes, the Class H TIFF region-line rule, `-` id dropping, the `SubIFD*` normalisation (is `SubIFD` numbering really pinned on `tiff-subifds-three`?), the Chain numbering.
- **Test the testers.** Write your own mutants the builder did not try, against `src/audit/mod.rs` and `src/tiff/mod.rs`, for the inventory path: wrong `after_scan` on items, `ifd_index` off by one, `field_type`/`count` swapped, Sub IFDs labelled Chain, IFD1 labelled Chain, Exif entries of block 1 attributed to block 0, region `segment` wrong, region kind Icc/Jumbf swapped, the inventory cap off by one, findings `item` index off by one, a pointer tag missing from the inventory. Report which the new suite catches (name the test) and which survive. Also re-run the builder's mutants i1 to i6 and carried M20/M21/M22/m04/m07/m08 yourself. Redo one fuzz plant of the F5 harness change against the real `audit` target.
- **Judge the carried tests** (F1 to F5, m04/m07/m08): do they pin the behaviour or only the mutant? The F5 step counters: could a guard be linear in the counter yet still quadratic in time?
- Check the 125 existing records and all seeds are byte-identical, the five new record files reproduce, and each record has command, version and `# stderr:` (or none) as specified; CI path covers the new directory; no dependencies; `scripts/`, `decisions/`, irradiance untouched.
- Do not fix code. Report findings with severity. Do not push or open a PR; do not advance cycles.

## Out of Scope

- Implementing fixes, SPEC-014/015/016 work, the DEC-008 amendment candidates, CI wiring beyond reading it

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
