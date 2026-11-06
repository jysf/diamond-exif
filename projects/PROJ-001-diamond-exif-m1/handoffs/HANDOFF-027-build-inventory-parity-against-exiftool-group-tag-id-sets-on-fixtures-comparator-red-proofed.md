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
  id: HANDOFF-027
  cycle: build                 # build | verify — which cycle is delegated
  from_agent: claude-opus-5-5       # the orchestrator (tier_map.design; DEC-005)
  to_agent: claude-sonnet-5-5           # from tier_map.<cycle> — the executing agent
  from_role: architect
  to_role: implementer             # implementer | verifier
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

# HANDOFF-027: Inventory parity against ExifTool: group + tag-id sets on fixtures, comparator red-proofed

## Delegation Summary

`claude-opus-5-5` (architect) hands `SPEC-013` to `claude-sonnet-5-5` (implementer) for the **build** cycle.

## Context the Receiving Agent Needs

Read before starting:

- **Spec (model, divergence classes, outputs, named tests are normative):** `projects/PROJ-001-diamond-exif-m1/specs/SPEC-013-inventory-parity-against-exiftool-group-tag-id-sets-on-fixtures-comparator-red-proofed.md`. Its measured table is a **prediction to re-measure**; where your measurement differs, yours wins and the Handback says so.
- `AGENTS.md`, `guidance/toolchain-brief.md` (host traps: `~/.cargo/bin/cargo +1.90.0` for library gates; `just lint`, `lint-red-proof`, `lint-fuzz`, `lint-fuzz-red-proof` under the default PATH; fuzz needs `PATH="$HOME/.cargo/bin:$PATH"` and `+nightly`; no `timeout(1)`, use `perl -e 'alarm N; exec @ARGV'`; never measure exit status through a pipe; macOS `sed -i ''`)
- Decisions: DEC-001, DEC-005, DEC-006, DEC-007, DEC-008 (read; do not edit)
- Code: `src/audit/{mod,category,sniff}.rs`, `src/tiff/mod.rs`, `tests/support/{tools,oracle,oracle_tiff,oracle_jpeg,audit_tags,probe,mutate,hostile}.rs`, `tests/{audit_oracle,audit_model,budget_oracle,jpeg_oracle,mutation_oracle,fuzz_layout}.rs`, `examples/oracle_record.rs`, `fuzz/shared/audit_invariants.rs`, `.github/workflows/ci.yml`
- Prior art for the carried items: `projects/PROJ-001-diamond-exif-m1/handoffs/HANDOFF-024-*.md` (F1 to F6, mutants M20 to M22) and `HANDOFF-026-*.md` (mutants m04, m07, m08); for the apparatus: HANDOFF-021/022 (SPEC-011)
- The ExifTool command for inventory is `exiftool -a -u -G1 -H -s <file>`; ExifTool is run, never read.

Constraints: `panic-free-library`, `forbid-unsafe-code`, `oracle-must-be-shown-red`, `exiftool-never-linked`, `test-before-implementation`, `no-new-top-level-deps-without-decision`.

## Expected Deliverables

- Everything in the spec's Outputs 1 to 7, every named test, every Acceptance Criterion. **`src/` does not change**; a bug the oracle finds in `src/` is reported in the Handback with the measured output and the affected test stopped (process rule 3).
- Order: the carried tests and their by-hand mutant red-proofs first (M20/M21/M22, m04/m07/m08: control, apply the mutant by hand, show the named failure with its assertion text, revert, `git diff` empty), then the inventory model with its parser tests, then probe each new shape alone, then records, sweep and `KNOWN_DIVERGENCES`, live and replay tests, red-proofs, probe inventory mode with mutants i1 to i6, fuzz harness linearity (F5) and the 60 s `audit` smoke, CI path.
- `KNOWN_DIVERGENCES` entries are **pinned multiset differences**, never "some difference". Add the `guidance/questions.yaml` entries the spec lists.
- The existing 125 records, the seed hash lists and all existing tests stay unchanged except the ones the spec names (the `audit-` prefix exemption in `tests/jpeg_oracle.rs`; seed or record counts only if you add seeds, which you should not). Any other change to an existing test, fixture or record is a **finding to report**.
- Run locally, because they are CI-only otherwise: `just lint-fuzz-red-proof`, `just lint-red-proof`, `just oracle-record` (empty `git status --porcelain` afterwards), and the extended CI path for the new records directory.
- Commit on your worktree branch. **Do not push, do not open a PR, do not advance cycles.** First `git merge --ff-only claude/rust-exif-metadata-6351d5` (you need the commit containing HANDOFF-027); if refused, use `git merge` and say so.
- Do not modify irradiance, `scripts/`, `decisions/`. No dependencies. Scratch work goes in a scratchpad directory, not `/tmp`.
- You cannot see your own token count; set `tokens_total: null` with a note, the orchestrator transcribes the metered figure.

## Out of Scope

- Any `src/` behaviour change; the DEC-008 amendment candidates (stay a question); resolving the depth, SubIFD or budget differences from ExifTool
- Value, XMP/ICC/IPTC/JUMBF content comparison; PNG/WebP/HEIC; the CLI; SPEC-014/015/016
- Pushing, PRs, cycle advancement

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
