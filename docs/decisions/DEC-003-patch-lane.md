---
insight:
  id: DEC-003
  type: decision
  confidence: 0.8
status: accepted            # proposed | accepted | superseded
created_at: 2026-06-27
deciders: [jysf, claude]
supersedes: null
superseded_by: null
affected_scope:
  - "scripts/new-patch.sh"
  - "scripts/archive-patch.sh"
  - "variants/*/projects/_templates/patch.md"
  - "scripts/validate.sh"
  - "scripts/cost-audit.sh"
  - "scripts/status.sh"
tags: [architecture, process, methodology, patch-lane, cli]
---

# DEC-003: a lightweight "patch" lane for fixes to shipped behavior

> **This is the template's own decision log** (meta), separate from the
> `decisions/` that ships *inside* each variant. **Accepted 2026-06-27.**

## Context

This is the **top recommendation** of the dogfood retrospective, arrived at
independently by two shipped projects and ranked #1 by the structured signals
harvest:

- **crustyimg** (43 specs / 9 stages → v0.1.0): "the full four-cycle was
  disproportionate for trivial changes — SPEC-043 was a 3-line `deny.toml` edit
  that still ran design→build→verify→ship — and the two elements that actually
  *bought* quality were the **DEC log** and the **independent verify**, not the
  ceremony of the four named cycles." It shipped a working proof-of-concept
  (its `DEC-043` + `PATCH-001`).
- **The stakes framing** (both crustyimg and bragfile): the template's value
  tracked the *stakes* of a change, not its size; a stakes-tiered lane that keeps
  only the independent verify + DEC "would shed most of the ceremony without
  losing the quality."

The signals-registry harvest (this template's v0.5.18–0.5.19 work) is what
surfaced and forced a decision on this, rather than letting it re-rot — which is
the mechanism working as intended.

## Decision

Add a **patch lane**: a lightweight track for a **bounded fix to already-shipped
behavior** (a bug or UX papercut) that introduces **no new feature/command** and
doesn't warrant a full spec + stage.

A patch runs a **collapsed 3-step cycle** instead of a spec's 5:

| Spec cycle | Patch cycle |
|---|---|
| frame → design → build → verify → ship | **patch → verify → ship** |

- **patch** — design + build fused into ONE test-first pass.
- **verify** — **kept, and kept INDEPENDENT** (a separate session/agent). The one
  discipline the retrospective validated; non-negotiable.
- **ship** — CHANGELOG `[Unreleased] → Fixed` + archive. **No stage bookkeeping.**

**Stays:** the full gate suite, a `DEC-*` only when there's a real decision, and
index-verify-before-ship. **Sheds:** the separate frame + design cycles, the
stage backlog/`Count:` bookkeeping, and the heavier spec frontmatter.

**Guardrail against scope creep:** if a change adds a command/flag or needs its
own design exploration, it is a **spec, not a patch**.

## Integration (patches are first-class, via `task.type: patch`)

Rather than a parallel artifact invisible to the tooling, a patch reuses the
**same `task.*` schema** as a spec — so it maps to the same ContextCore
attribute names and the existing gates see it. Two differences: `task.cycle` ∈
`{patch, verify, ship}`, and there is **no `project.stage`** (a patch attaches to
the project). Mechanics:

- **`projects/_templates/patch.md`** (both variants) — the artifact.
- **`just new-patch "title" [PROJ-NNN]`** → `projects/PROJ-*/patches/PATCH-NNN-<slug>.md`;
  **`just archive-patch`** → `patches/done/`. `PATCH-*` is its own repo-wide,
  continuous id sequence.
- **`just validate`** validates patch front-matter (cycle enum patch/verify/ship;
  no stage required). **`just cost-audit`** gates a shipped patch's `patch`+`verify`
  cost. **`just status`** lists patches by cycle (human + `--json`).

## Alternatives considered

- **Self-verify + gates-only (no independent verify)** — lighter, but discards
  the single lever the evidence validated. **Rejected.**
- **Just use a normal spec for small fixes** — the status quo; its overhead is
  the exact problem. **Rejected.**
- **A generic "fast lane" for any change** — too broad; a patch is specifically a
  fix to *shipped* behavior with no new surface, which is what makes the
  collapsed cycle safe. **Rejected.**
- **A separate `patch.*` namespace** (the crustyimg PoC) — clean, but leaves
  patches invisible to cost-audit/validate/status. **Rejected** in favor of
  `task.type: patch` so the disciplines apply automatically.

## Consequences

- **Positive:** post-release fixes cost ~2 metered cycles instead of ~4, no stage
  overhead, while keeping independent verify + DECs + gates. Encourages fixing
  papercuts instead of deferring them (the "defer to weekly review" black hole
  the harvest named).
- **Negative:** a second cycle vocabulary (`patch`) and id sequence (`PATCH-*`)
  to track; scope-creep risk (a "patch" that's really a feature) — mitigated by
  the guardrail.
- **Scope (v1):** validate + cost-audit + `status` recognize patches now (and
  `dash now` inherits `status`). **Follow-ups:** a dedicated `dash patches` lens
  and patch lines in `report-daily`/`report-weekly` are deferred (the report
  generators still emit specs-only).
