---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-015
  type: story                      # epic | story | task | bug | chore
  cycle: frame                     # frame | design | build | verify | ship
  blocked: false
  priority: medium                 # critical | high | medium | low
  complexity: M                    # XS | S | M | L | XL | XXL — the EXPECTED size, set at design
                                   #   (XL/XXL almost certainly means it's a stage, not a spec)
  complexity_actual: null          # stamped at ship: what it ACTUALLY took, same scale.
                                   #   Expected-vs-actual drift is what `just calibration` reads.
  verify_verdict: null             # approved | punch-list | rejected — the OUTCOME of the verify
                                   #   cycle, stamped by `just advance-cycle` when the spec leaves
                                   #   verify (same three verdicts Prompt 4 already returns).
                                   #   Recorded in front-matter, not just prose, so "verify never
                                   #   rejects anything" stops being a hunch and becomes a number.

project:
  id: PROJ-001
  stage: STAGE-003
repo:
  id: diamond-exif

handoff:
  from_agent: claude-opus-5-5  # from .repo-context tier_map.design (DEC-005)
  to_agent: null                   # filled when HANDOFF is created (any agent — see docs/porting.md)
  created_at: null

references:
  decisions: []                    # [DEC-NNN, DEC-MMM]
  constraints: []                  # [constraint-id-1, constraint-id-2]
  related_specs: []                # [SPEC-NNN]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: [SPEC-012, SPEC-014]                     # e.g. [SPEC-002]

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-003's <capability>". Optional; null is acceptable.
value_link: null

# Self-reported AI cost per cycle. Each cycle (design, build, verify,
# ship) appends one entry to sessions[]. Totals are computed at ship.
# Record a REAL tokens_total for metered cycles (build/verify) — the agent
# that runs the cycle writes it from its own interface (/cost, the API
# usage object, or its tool's report). Only un-metered main-loop cycles
# (design/ship) may be null-with-note. `just cost-audit` enforces this on
# shipped specs. See AGENTS.md §4 and docs/cost-tracking.md. interface:
# claude-code | claude-ai | api | ollama | other.
cost:
  # Optional PREDICTION of the total tokens this spec will take, set at
  # design. Never a gate — its only job is to be compared with the actual
  # below (`just calibration`), so you learn whether you systematically
  # under- or over-estimate. null = didn't predict.
  tokens_estimate: null
  sessions: []
  totals:
    tokens_total: 0
    estimated_usd: 0
    session_count: 0
---

# SPEC-015: Text, JSON and CSV renderers and the `audit_render` fuzz target (well-formed on hostile input)

> **OUTLINE — `cycle: frame`.** This spec exists so its ID is stable and
> siblings can declare `depends_on: [SPEC-015]`. Capture **scope** (Context /
> Goal / Non-Goals) and **dependencies** only — the *approach* is designed
> just-in-time when this moves to `design`. Do not pre-design it here.

## Context

Report text comes from hostile input: tag values are arbitrary bytes from files. A hand-written JSON or CSV writer that mishandles a quote, a control character or invalid UTF-8 produces output that breaks the CI job reading it. The renderers are therefore a surface in their own right and get their own fuzz target (constraint `fuzz-target-per-surface`).

## Goal

Pure text, JSON and CSV renderers over `FileAudit` and `Preview` (and a run summary), deterministic with stable field order, plus an `audit_render` fuzz target that feeds arbitrary bytes through audit and render and checks the JSON and CSV are well-formed with an independent checker.

## Carried from SPEC-013 verify (low findings)

- The `audit_harness_guards_are_linear_in_the_input` test counts only where a `tick()` was placed, so a guard can stay linear in the counter yet quadratic in time (the verifier put the old rescan back without a tick and the test passed). Strengthen with a coarse wall-clock bound on a large input, or count the work at the loop itself.
- The Class H rule in `tests/support/inventory.rs` drops every ExifTool region line for a bare TIFF; the companion test checks the file carries one of the four directory-valued ids but not that the region kind matches the id (0x83BB -> Iptc, 0x02BC -> Xmp, 0x8773 -> Icc, 0x8649 -> Photoshop/Iptc). Tighten when a second such input appears.
- The inventory records do not keep ExifTool's exit status (it is 1 on four inputs where both sides are empty); record it if a future divergence could hide there.

## Carried from SPEC-014 verify (findings, none blocking)

From HANDOFF-030; SPEC-015 extends the same `audit` harness, so these belong here:

- **F1 (Medium).** Add the two hand-built full-house inputs of `tests/fuzz_layout.rs` (JPEG and TIFF) as `fuzz/seeds/audit` seeds, so the libFuzzer target, not only the stable layout test, reaches DeviceId and MakerNote (a plant of mutant 9 was not found in 181 s). This moves the audit seed count from 76 to 78: update `audit_model.rs` and the seed hash lists in the same change.
- **F2 (Low).** Cover rule 4's truncated branch end to end with an input `audit()` really truncates (the `over` input of the cap-boundary test: 65,535 entries then Make and BodySerialNumber), so mutants 15, 16 and the harness's truncated `all()` exception are caught outside the hand-built `FileAudit`.
- **F3 (Low).** Assert the harness's `Custom` case is non-trivial (some inputs give a set that is neither empty nor `all()`), so it cannot silently become `Custom(none)`.
- **Wording.** Renderers describe `Kept` for a block as "not targeted by this policy" and `KeptUnread` as "may hold"; they read `reliable()` before the residual numbers (DEC-009 amendment 2026-11-06).

## Non-Goals

- Reading or writing files; argument handling (SPEC-016)
- A JSON dependency (hand-written, escaping fuzzed) unless a DEC says otherwise
- Pretty-printing options beyond a fixed layout

---

## Reflection

*Appended during **ship**. Three questions, short answers.*

1. **What would I do differently next time?**
   — <answer>

2. **Does any template, constraint, or decision need updating?**
   — <answer — if yes but not done this session, record it in
   `/guidance/signals.yaml`: `type: lesson` (with its N-count) for a recurring
   coding pattern, `type: process-debt` for tooling/process friction. A close
   then forces the decision. See `docs/signals.md`.>

3. **Is there a follow-up spec I should write now before I forget?**
   — <answer>

4. **Where was the worst defect caught?** — one word from a fixed vocabulary so
   the defect-escape distribution is greppable across specs:
   `design` | `build` | `verify` | `ship` | `escaped` (reached prod/runtime) |
   `none` (clean first try).
   — <one word>
   *(Runtime/operational defects — the escape-prone class — only exist once the
   artifact meets its real host. `escaped` here is a signal to strengthen the
   §12 behavioral pre-flight for that surface.)*

5. **What can a user do now that they couldn't before?** — one sentence,
   before → after; quote the confirming number if one exists, name the outcome
   if not. Write `none` if this spec has no user-visible outcome — that is a
   real, greppable result, not a blank. This is the line a downstream work-log's
   `impact` field is transcribed from, and both halves are already written above
   (## Context is the before, ## Goal is the after): confirm the prediction,
   don't reconstruct it from memory.
   — <answer | none>
