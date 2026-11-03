---
insight:
  id: DEC-001
  type: decision
  confidence: 0.8
status: accepted            # proposed | accepted | superseded
created_at: 2026-06-18
deciders: [jysf, claude]
supersedes: null
superseded_by: null
affected_scope:
  - "scripts/**"
  - "justfile"
  - "variants/*/projects/_templates/**"
  - "variants/*/guidance/constraints.yaml"
tags: [architecture, api, schema, tooling, cli]
---

# DEC-001: Interface contract — front-matter schema as the public API + structured command output

> **This is the template's own decision log** (meta), separate from the
> `decisions/` that ships *inside* each variant to instances. It records how
> the template itself evolves. **Accepted 2026-06-18** — implementation
> proceeds in phases (see Rollout); the cost-convention track is spun out to
> [DEC-002](DEC-002-cost-convention.md).
>
> **Rollout status:** Phase 1 (schema doc, `validate`, `--json` + exit codes,
> `dash`) and Phase 2 (shared validator + CI job) shipped. **Phase 3 shipped in
> v0.6.0 (breaking):** `daily-status-report` → `report status` and
> `weekly-review` → `review`, consolidated under the `report`/`review`
> namespace; `report-daily` / `report-weekly` kept their bare names as permanent
> aliases (muscle memory wins). Phase 4 (MCP server over `--json`) remains a
> separate future decision.

## Context

The template is maturing from "a scaffold I use" toward a **methodology +
governance layer** others could adopt. A scan of the field (GitHub Spec Kit,
AWS Kiro, Backlog.md, OpenSpec, BMAD, Tessl, …) shows the tools that gained
traction did so on a **clean machine-readable contract + agent integration
(MCP / JSON)** — not on prettier terminal output. Before any packaging (CLI
install, MCP server, Claude plugin, or UI), the *interfaces* have to be stable
and consumable. Three gaps block that today:

1. **No canonical artifact schema.** Every script re-derives front-matter
   parsing with ad-hoc `awk` in `scripts/_lib.sh`. The front-matter IS the real
   public API — it's what scripts, reports, and any future package/MCP/UI
   consume — but it is neither written down in one place nor validated.
2. **No output contract.** Read commands emit ANSI-colored prose. Any
   programmatic consumer must screen-scrape, and exit-code semantics are
   undocumented.
3. **Organic command surface.** Overlaps have accreted: `report-daily` vs
   `daily-status-report` vs `weekly-review` vs `report-weekly`; four read views
   (`status` / `backlog` / `roadmap` / `specs-by-stage`) with fuzzy boundaries.

Constraints that bound any solution (CONTRIBUTING.md, non-negotiable): **zero
runtime dependencies, bash 3.2, portable shell.** And: the template must stay
usable on live projects throughout this work.

**Framing.** In practice the day-to-day value is a **project-manager /
reporting layer** over the spec-driven process — the most-used commands are the
read/report views (`status`, `specs-by-stage`, `roadmap`, `report-daily`,
`backlog`), which answer *where are things now? / what's coming? / what are we
not working on next?*. The governance gates (cost, decisions, constraints)
exist to make those reports **trustworthy**. This positioning orders the
priorities below: the read/report surface is the product, so structured output
for it comes first — and a recurring failure mode is **view sprawl** (wanting a
slightly different slice and writing a whole new command for it).

## Decision

Adopt three contracts, **additive and backward-compatible first**. Defer
breaking renames behind a major version (v6) with deprecation aliases.

### 1. The artifact front-matter is the public API (canonical + validated)

Declare the front-matter the stable, versioned contract. Add a `schema_version`
to each artifact and a single `just validate` that is the **one** source of
truth for "is this artifact well-formed" — scripts migrate to it incrementally
instead of each re-deriving parsing. Canonical shapes (✅ required · ◦ optional):

**`projects/*/specs/SPEC-*.md`**
```
task:        ✅ {id, type, cycle, blocked, priority, complexity}
project:     ✅ {id, stage}          repo: ✅ {id}
agents:      ◦  {architect, implementer, created_at}
references:  ◦  {decisions[], constraints[], related_specs[]}
value_link:  ◦  string | null
cost:        ✅ {sessions[]: {cycle, agent, interface, tokens_total,
                              estimated_usd, duration_minutes, recorded_at, notes},
                 totals: {tokens_total, estimated_usd, session_count}}
```
**`projects/*/stages/STAGE-*.md`** → `status`, `shipped_at`,
`value_contribution: {advances, delivers[]}`
**`projects/*/brief.md`** → `value.thesis`, `project: {…}`
**`decisions/DEC-*.md`** → `insight: {id, type, confidence}`, `supersedes`,
`superseded_by`, `affected_scope[]`
**`guidance/constraints.yaml`** → `constraints[]: {id, rule, severity, paths[],
added_by, added_at, rationale}`
**`.repo-context.yaml`** → repo identity/metadata

Field names map to **ContextCore / OTel** semantic conventions where they
overlap (`task.*`, `project.*`, `business.*`, `insight.*`, `guidance.*`,
`agent.*`) — that alignment is the contract's anchor; the full crosswalk
(verified against ContextCore's `docs/reference/` + `semconv/registry/`) is in
§5. Note `cost.*` is **not** a ContextCore or OTel convention — it is a template
extension (see §5). Changing the schema after acceptance requires a decision +
a migration note (precedent: `MIGRATION_TO_REPORTS_AND_COSTS.md`).

### 2. Command output is a contract: `--json` + defined exit codes

Add an opt-in `--json` flag to the read/report commands. **The
project-manager views get it first** — `status`, `specs-by-stage`, `roadmap`,
`report-daily`, `report-weekly`, `backlog` (then `info`) — since that's the
data you'd pipe into a standup, a dashboard, or another tool. Default human
output is unchanged. One stable envelope:
```json
{ "schema_version": 1, "command": "status", "generated_at": "…",
  "data": { /* command-specific, mirrors the documented schema */ } }
```
Exit-code semantics, documented and stable:
- `0` — success (read commands always; gates when clean)
- `1` — **gate failure** (`cost-audit`, `decisions-audit`, future `validate`):
  a real violation. This is the CI contract.
- `2` — usage error (bad flag/argument).

This is the single change that unlocks both an MCP server (thin wrapper over
`--json`) and any future UI — neither has to scrape colored text.

**Shape the `data` payload with ContextCore/OTel attribute names** (`task.*`,
`project.*`, `business.*`, `insight.*`, `guidance.*`, `agent.*` /
`gen_ai.agent.*`, `handoff.*`) so `--json` is a drop-in feeder into
ContextCore's span model — a thin exporter maps it to OTel spans, making
"graduate to the ContextCore stack" mechanical rather than aspirational (§5).
Cost is the one exception: no standard attribute exists, so emit it under the
template's own `cost.*` (and token *counts* as OTel `gen_ai.usage.*` if/when an
input/output split is ever available).

### 3. Command-surface rationalization (phased; breaking parts deferred to v6)

| Current | Disposition |
|---|---|
| `init`, `status`, `new-spec`, `new-stage`, `advance-cycle`, `archive-spec` | **Keep** (core verbs) |
| `cost-audit`, `decisions-audit` | **Keep**; formalize the exit-code gate contract above |
| `status` / `backlog` / `roadmap` / `specs-by-stage` | **Unify** as lenses of one command (see #4); the four keep working as **permanent aliases**, and scope flags `--active \| --all \| PROJ-NNN` become universal |
| `report-daily`, `report-weekly` | **Daily-drivers — keep them.** If they ever move under a `report {daily\|weekly}` namespace, the bare names stay as **permanent aliases** (no deprecation). Muscle memory wins over tidiness. |
| `daily-status-report`, `weekly-review` | **Consolidate** these two (the genuinely confusable ones) into the `report`/`review` namespace; v6, aliased |
| `dash` | **New** — one read command, many lenses (see #4) |
| `validate` | **New** — the schema gate from #1 |

### 4. One read command, many lenses (`just dash`) — the antidote to view sprawl

The read views aren't really separate tools; they're **lenses on one model**,
split by a time/scope axis. Today, wanting a slightly different slice means
writing a *new script* — that's the sprawl. Replace it with **one command,
`just dash [lens]`**, over a single data-collection pass:

| Lens | Question it answers | Today's command |
|---|---|---|
| **now** | Where are things now? | `status` |
| **next** | What are we *not* working on next? (deferred / un-promoted) | `backlog` |
| **future** | What's coming? | `roadmap` |
| **ledger** | Everything, all history | `specs-by-stage` |

- `just dash` (no arg) → a **stitched dashboard**: now + future + recorded cost +
  flags (the single "where's everything at" view you've wanted). **Decided.**
- `just dash now\|next\|future\|ledger` → the individual lenses above.
- A new slice becomes a **lens or flag, never a new script.** That is the rule
  that stops the sprawl.

This is **additive**: `dash` is a brand-new command (Phase 1), and
`status`/`roadmap`/`backlog`/`specs-by-stage` keep working as permanent thin
aliases into it. It also leans directly on #1 and #2 — `dash` collects the
documented model **once** and renders any lens as human text or `--json`, which
is exactly why the schema + structured-output work comes first.

### 5. ContextCore / OTel alignment — the crosswalk and the `cost.*` extension

Verified 2026-06-18 against ContextCore's `docs/reference/semantic-conventions.md`,
`docs/reference/agent-semantic-conventions.md`, and `semconv/registry/`.
ContextCore (v2.0+) emits OTel **GenAI** attributes (`gen_ai.*`) alongside its
own custom governance namespaces. The template's front-matter maps as:

| Template front-matter | ContextCore / OTel | Notes |
|---|---|---|
| `task.id`, `task.type`, `task.priority` | `task.id`, `task.type`, `task.priority` | aligned; `task.type: chore` is template-only (CC adds `subtask`/`spike`/`incident`) |
| `task.cycle` (frame→ship) | `task.status` (backlog…done) | **no 1:1** — `cycle` is the template's SDLC-phase specialization of status; the exporter ships a cycle→status crosswalk |
| `task.complexity` (S/M/L) | — | template extension |
| `project.id`, `project.stage` | `project.id`, `project.epic` / `sprint.*` | `stage` ≈ epic/sprint |
| value layer (`value.thesis`, `value_link`, `value_contribution`) | `business.value`, `business.criticality`, `business.owner`, `business.cost_center` | candidate: align the value layer to `business.*` |
| `DEC-*`: `insight.{id,type,confidence}`, `supersedes`, `affected_scope` | `insight.{id,type,confidence,summary,audience,supersedes,evidence[]}` | strong; use `insight.type: decision`; adopt `insight.audience` (`agent\|human\|both`); `affected_scope` ≈ `insight.evidence[]{type: file/commit/pr/adr}` |
| `constraints.yaml` + `questions.yaml` | `guidance.*` with `guidance.type = constraint \| question` | CC unifies both under one namespace via the `type` discriminator (there is **no** `guidance.constraint_id`/`question_id`); template `severity` ≠ CC `guidance.priority` enum |
| `agents.{architect,implementer,created_at}` | `agent.{id,session_id,type}` ↔ `gen_ai.agent.*` | map roles → `agent.type` (`code_assistant`/`orchestrator`/`specialist`); `session_id` could tie a cost session to an agent |
| `handoffs/HANDOFF-*` (plus-agents) | `handoff.*` ↔ `gen_ai.tool.*` | adopt `handoff.status` enum (`pending`…`completed`) |
| **`cost.{tokens_total,estimated_usd,…}`** | **none** | nearest is OTel `gen_ai.usage.{input,output}_tokens` — token **count**, no USD; `business.cost_center` is org attribution, not spend |

**The `cost.*` finding (strategic).** Cost is confirmed **absent** from both
ContextCore and OTel GenAI — they track token *counts*, never USD. The
template's enforced `cost.estimated_usd` is therefore a genuine **extension
ahead of the standard it aligns to.** Two consequences:

- Keep `cost.*` as a first-class, documented **template extension namespace** —
  it won't collide with ContextCore.
- **Upstream candidate:** ContextCore maintains `semconv/registry/` and a
  `docs/otel-submission/` pipeline (it actively submits conventions to OTel), so
  proposing a `cost`/`spend` convention there is a concrete contribution path —
  arguably the most credible answer to "is this useful beyond myself?" (it fills
  a gap the ecosystem has). Track as its own decision, not this one.

## Backward compatibility & in-repo evolution

We evolve **in this repo, not a fork.** Phase 1 (schema doc + `just validate` +
`--json`) is purely additive: existing commands, output, and files are
unchanged, so the template stays usable on live projects. Breaking changes
(command renames/merges) land only in **v6** with deprecation aliases kept for
at least one minor cycle. A fork would only make sense for a *different product*
(web/SaaS) or a non-bash runtime rewrite — not for this maturation.

## Consequences

- **Packaging becomes mechanical.** An MCP server / Homebrew CLI / Claude plugin
  is a thin layer over `--json` + the schema. MCP is the natural next step.
- **One validator, one truth.** Scripts stop re-deriving parsing (incremental
  migration to the `just validate` helpers).
- **The schema is now a commitment.** Changes need a decision + migration note —
  a cost, but it's the price of being usable beyond one author.

## Open questions (need a spike before "accepted")

1. **JSON emission in bash 3.2 without `yq`/`jq`** — hand-rolled escaping vs a
   tiny vendored `awk` JSON emitter. Affects feasibility of #2.
2. **Schema versioning** — per-artifact `schema_version` vs one repo-wide version.
3. *(resolved)* Unified command named **`dash`**; the no-arg `just dash`
   stitches now + future + recorded cost + flags. Old views stay as aliases.

## Rollout sketch (once accepted)

- **Phase 1 (non-breaking):** write the schema reference doc; add `just validate`;
  add `--json` + documented exit codes to the project-manager views; introduce
  the unified `just dash` command (existing views become permanent aliases into
  it). Ship as a minor (v5.x).
- **Phase 2 (non-breaking):** migrate scripts to the shared validator; add a
  `validate` CI job alongside `cost-data`.
- **Phase 3 (v0.6.0, breaking, aliased) — SHIPPED:** consolidated the
  `daily-status-report` / `weekly-review` pair into the `report`/`review`
  namespace (`report status` / `review`). `report-daily` / `report-weekly` keep
  their bare names as permanent aliases.
- **Phase 4 (packaging):** MCP server over `--json` (separate decision).
