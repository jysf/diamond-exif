---
# A PATCH is a lightweight fix to ALREADY-SHIPPED behavior (a bug or UX
# papercut) that adds NO new feature/command and doesn't warrant a full
# spec + stage. See AGENTS.md "Patch lane" and docs/decisions/DEC-003.
#
# Collapsed cycle: patch -> verify -> ship (design+build fused into one
# test-first pass; the INDEPENDENT verify is KEPT). It uses the same task.*
# schema as a spec, so `just validate`, `just cost-audit`, and `just status`
# treat a patch as first-class.

task:
  id: PATCH-XXX
  type: patch                      # epic | story | task | bug | chore | patch
  cycle: patch                     # patch | verify | ship  (collapsed from a spec's 5)
  blocked: false
  priority: medium
  complexity: S                    # S | M  (an L fix is probably a spec, not a patch)

project:
  id: PROJ-XXX
  # No `stage:` — a patch attaches to the PROJECT, not a stage.
repo:
  id: __REPO_ID__

agents:
  implementer: __IMPLEMENTER_MODEL__  # the patch pass (tier_map.build; DEC-005)
  verifier: __VERIFIER_MODEL__        # independent verify — KEPT (tier_map.verify; a separate session/agent)
  created_at: __TODAY__

references:
  decisions: []                    # add a DEC only when there's a real decision

# Cost: patch + verify are the metered cycles — `just cost-audit` requires a
# real tokens_total on both for a shipped patch. ship is main-loop (null-with-note).
cost:
  sessions: []
  totals:
    tokens_total: 0
    estimated_usd: 0
    session_count: 0
---

# PATCH-XXX: <the shipped behavior this fixes>

## Problem

The already-shipped behavior being fixed — what's wrong, who hit it, and the
evidence (a reproduction or a report). A patch fixes EXISTING behavior; if
there is no shipped behavior to fix, it's a spec.

## Fix

The bounded change. Keep it to the fix — no new command/flag, no new feature
surface. **Guardrail:** if the change adds a command/flag or needs its own
design exploration, stop — it's a spec, not a patch.

## Failing Tests

Write these FIRST, in the patch pass (test-first still holds):

- **`path/to/test`** — `"name"` — asserts the bug is fixed / the papercut is gone.

## Verification (independent — KEPT)

Run in a SEPARATE session/agent from the patch pass. This is the one discipline
the framework retrospective proved catches real defects; it is non-negotiable
for a patch.

- Run the project's full gate suite (tests, lint/format, and any security/
  dependency gates the repo defines).
- Confirm the failing tests now pass and no existing test regressed.
- Output: ✅ APPROVED / ⚠ PUNCH LIST / ❌ REJECTED.

## Patch Completion

*Filled at the end of the patch pass, before verify.*

- **Branch / PR:**
- **Fix summary:** <one or two lines>
- **New decision emitted:** `DEC-NNN` (only if a real decision was made)
- **Reflection (1 line):** what would make this class of fix faster next time?
- **Defect-catch-stage:** where the bug this patch fixes was caught —
  `design` | `build` | `verify` | `ship` | `escaped` (reached prod/runtime) —
  one word, for the cross-project defect-escape distribution. (A patch usually
  fixes an `escaped` defect; that's the signal a behavioral pre-flight was missed.)

## Ship

- Add a CHANGELOG entry under `[Unreleased] → Fixed`.
- Append cost sessions (patch + verify metered; ship null-with-note), then
  compute `cost.totals`.
- `just advance-cycle PATCH-NNN ship`, then `just archive-patch PATCH-NNN`.
- **No stage bookkeeping** — a patch attaches to the project, not a stage.
