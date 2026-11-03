---
insight:
  id: DEC-002
  type: decision
  confidence: 0.7
status: proposed            # proposed | accepted | superseded
created_at: 2026-06-18
deciders: [jysf, claude]
supersedes: null
superseded_by: null
affected_scope:
  - "scripts/_lib.sh"
  - "scripts/cost-audit.sh"
  - "variants/*/projects/_templates/spec.md"
  - "variants/*/docs/cost-tracking.md"
tags: [architecture, cost, schema, contextcore, otel, upstream]
---

# DEC-002: Formalize `cost.*` as a template extension + propose a cost convention upstream

> Template meta-decision. **Status: proposed** — a fleshed-out proposal for
> review. Builds on [DEC-001](DEC-001-interface-contract.md) (front-matter as
> the public API; `--json` shaped with ContextCore/OTel attribute names).

## Context

Verified 2026-06-18 (sources below): **neither ContextCore nor OTel GenAI
defines a cost/USD convention.** OTel GenAI has token *counts*
(`gen_ai.usage.input_tokens` / `output_tokens`) but no spend; ContextCore has
`business.cost_center` (org attribution) but no per-unit cost. The template's
enforced per-cycle cost capture (`cost.estimated_usd`, `tokens_total`; the
`cost-audit` gate, v5.11) is therefore a working convention **ahead of the
standards it aligns to** — and the strongest evidence the template is useful
beyond its author.

Sources checked: `docs/reference/semantic-conventions.md`,
`docs/reference/agent-semantic-conventions.md`, `semconv/registry/` (only
agent/project/sprint/task), `semconv/spans` (task only), `semconv/events` (task
only), repo code search (`cost_usd`/`estimated_usd`/`spend`/`token_cost` → 0).

Why OTel/CC have avoided USD: **price is provider- and time-variable**, so a raw
`cost_usd` looks unstandardizable. The template already solved this in practice
by treating USD as an *explicit order-of-magnitude estimate* with a stated
basis. That separation — measured fact vs. derived estimate — is the
contribution that makes a cost convention submittable.

## Decision

### 1. `cost.*` is a versioned, first-class template extension

Promote the per-spec `cost` block (today documented inline in `spec.md` /
`docs/cost-tracking.md`) to a **named, versioned convention** in the schema
reference from DEC-001 §1, clearly flagged as a template extension (no
ContextCore/OTel collision). Canonical shape:

**Measured facts (what actually happened):**
| Attribute | Type | Notes / OTel mapping |
|---|---|---|
| `cost.cycle` | enum | frame\|design\|build\|verify\|ship (template SDLC phase — no OTel equiv) |
| `cost.model` | string | → OTel `gen_ai.request.model` |
| `cost.interface` | enum | claude-code\|claude-ai\|api\|ollama\|other (measurement provenance) |
| `cost.tokens_total` | int | combined count (harnesses report one number) → OTel `gen_ai.usage.total_tokens` |
| `cost.tokens_input` / `cost.tokens_output` | int? | optional split → OTel `gen_ai.usage.input_tokens`/`output_tokens` |
| `cost.duration_minutes` | number | → span duration |
| `cost.recorded_at` | date | → span timestamp |

**Derived estimate (clearly labeled, never presented as exact):**
| Attribute | Type | Notes |
|---|---|---|
| `cost.estimated_usd` | double | order-of-magnitude estimate |
| `cost.estimate_basis` | string | **the key field** — e.g. `"list-rate 2026-06; ~80/20 in/out; no cache discount"`. Makes the number interpretable + reproducible |
| `cost.currency` | string | default `USD` |

**Rollup (per spec):** `cost.totals.{tokens_total, estimated_usd, session_count}`.

The `cost-audit` gate (DEC-001 §2 exit codes) enforces a real `tokens_total` on
metered cycles; `--json` emits the above under a `cost.*`-namespaced payload.

### 2. Upstream proposal: a cost/spend convention for ContextCore (and via it, OTel)

ContextCore runs a `semconv/registry/` + a `docs/otel-submission/` pipeline (it
actively submits conventions to OTel). Propose a cost convention there, framed
around the fact/estimate split so it survives the "pricing is volatile"
objection:

- **Measured:** reuse OTel `gen_ai.usage.*_tokens` + `gen_ai.request.model`
  verbatim (don't reinvent).
- **New:** `gen_ai.usage.cost.estimated_amount` (double) +
  `gen_ai.usage.cost.basis` (string) + `gen_ai.usage.cost.currency` — i.e. cost
  is a *derived, basis-annotated estimate*, not a claimed-exact figure. This is
  the genuinely new, defensible idea.
- Offer the template as the **reference producer** of these attributes (a
  zero-dependency emitter), and `docs/cost-tracking.md` as the rationale writeup.

### 3. What else the template can propose to ContextCore (coalescing)

Beyond cost, the template holds two things ContextCore's *observability* model
lacks — both are *governance/process* contributions:

- **The enforcement-gate pattern.** CC records `agentGuidance` constraints in a
  CRD but doesn't *fail a build* on violation. The template's "discipline made
  mechanical with a `just` check + a CI job" (cost-audit, decisions-audit) is a
  reusable pattern CC could adopt: a CI gate that validates artifacts/spans
  against constraints. Offer it as a pattern doc / reference gate.
- **The template as ContextCore's zero-dependency on-ramp.** CC's entry cost is
  a k8s + Tempo/Loki/Mimir/Grafana stack. The template is markdown + bash, no
  infra. Position it as the lightweight authoring front-end that *graduates*
  into CC: a thin exporter maps the template's DEC-001 `--json` → OTel spans
  (CC's README already promises this path; the exporter makes it real). This is
  its own future decision (depends on DEC-001 Phase 1–2), tracked here so the
  intent isn't lost.

## Relationship to DEC-001

DEC-002 does not block DEC-001. `cost.*` is already part of the DEC-001 schema;
this decision (a) formalizes/versions it and (b) opens the upstream + exporter
tracks. Implementation order: DEC-001 Phase 1 (schema doc + `--json`) lands the
`cost.*` shape; the upstream proposal and the exporter are separate, later work.

## Open questions

1. **Namespace home for cost** — a standalone `cost.*` vs. extending
   `gen_ai.usage.cost.*`. Leaning `gen_ai.usage.cost.*` for the upstream
   proposal (rides existing GenAI conventions), keeping bare `cost.*` as the
   template's local front-matter key.
2. **Input/output split** — harnesses report one combined number; OTel wants a
   split. Propose `total_tokens` as first-class with the split optional.
3. **Cache-discount + tiered pricing** — keep out of the standard; capture in
   `cost.estimate_basis` free-text for now.
4. **Currency / FX** — default USD; defer multi-currency.

## Appendix — prompt to draft the upstream proposal (hand to a session)

```
You are drafting a proposal to add a COST/SPEND semantic convention to ContextCore
(github.com/neil-the-nowledgeable/contextcore), which maintains semconv/registry/
and a docs/otel-submission/ pipeline to OpenTelemetry.

Verified gap: neither ContextCore nor OTel GenAI defines a cost/USD attribute —
only token counts (gen_ai.usage.input_tokens/output_tokens). Confirm this still
holds, then draft a convention with this core idea: cost is a DERIVED, BASIS-
ANNOTATED ESTIMATE, not a claimed-exact figure — which is what makes USD
standardizable despite volatile, provider-specific pricing.

Propose: gen_ai.usage.cost.estimated_amount (double), gen_ai.usage.cost.basis
(string; e.g. "list-rate 2026-06; ~80/20 in/out; no cache discount"),
gen_ai.usage.cost.currency (default USD); reuse gen_ai.request.model and
gen_ai.usage.*_tokens for the measured facts. Include: motivation, attribute
table with types/requirement levels/examples, the fact-vs-estimate rationale,
prior art (the spec-driven-template's enforced cost.* + cost-audit gate as a
reference producer), and how it slots into ContextCore's semconv/registry format.
Output a registry-style YAML stub + a short markdown rationale suitable for a PR.
```
