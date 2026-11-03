---
insight:
  id: DEC-013
  type: decision
  confidence: 0.75
status: accepted            # proposed | accepted | superseded
created_at: 2026-08-10
deciders: [jysf, claude]
supersedes: null
superseded_by: null
affected_scope:
  - "scripts/new-handoff.sh"
  - "scripts/handback-sync.sh"
  - "variants/claude-plus-agents/projects/_templates/handoff.md"
  - "variants/*/projects/_templates/stage.md"
  - "scripts/cost-audit.sh"
tags: [architecture, cost, delegation, handoff, multi-agent, provenance]
---

# DEC-013: delegated-cycle cost — the handback contract

> **This is the template's own decision log** (meta), separate from the
> `decisions/` that ships *inside* each variant. **Accepted 2026-08-10.**

## Context

The template is about to be used the way it always claimed to support but never
actually was: **`claude-plus-agents` with build and verify on two different
non-Claude agents** (the `grebe` project). That surfaced a blocker that had been
latent since the cost convention shipped.

`cost-captured-per-cycle` (a **blocking** constraint, enforced by `just
cost-audit` in the one CI job the template ships) requires a real `tokens_total`
on the **metered** cycles — build and verify. The metering model assumed those
cycles run as Claude **sub-agents**, where the orchestrator reads
`subagent_tokens` straight out of the `Agent` result
([DEC-002](DEC-002-cost-convention.md), [DEC-004](DEC-004-subagent-execution-mode.md)).

**An orchestrator has no meter for an agent it does not host.** So with external
build/verify agents there were only three outcomes, all bad:

1. Every shipped spec fails the cost gate.
2. `cost.metering_source: none` disables the gate — and the cost data is lost
   permanently, for cycles that *do* have a number available.
3. The orchestrator estimates. **This is the worst one**, and it is the reason
   this decision exists: an invented token count is indistinguishable from a
   real one downstream, so it silently corrupts `calibration`, the
   predicted-vs-realized loop ([DEC-009](DEC-009-business-value-metrics.md)),
   and every cost rollup — while *looking* like rigor.

The existing `handoffs/HANDOFF-*.md` artifact had a `## Completion` section, but
it was prose-only, carried no cost, and was hardcoded to a single `build`
delegation — it predates both the tier split and the cost convention.

## Decision

**The agent that ran the cycle reports its own cost. The orchestrator
transcribes; it never estimates.**

### 1. One handoff per delegated *cycle*

`handoff.cycle` ∈ `{build, verify}`. A build/verify split produces **two**
handoffs per spec, and `to_agent` resolves from `.repo-context.yaml` →
`spec.agent.tier_map.<cycle>` ([DEC-005](DEC-005-agent-portability.md)). The
previous single-handoff shape could not express a split it was nominally
designed for. `just new-handoff SPEC-NNN build|verify`.

### 2. A machine-readable `handback:` block

The handoff carries a `handback:` front-matter block the executing agent fills
**before reporting done** — `status`, `tokens_total`, `estimated_usd`,
`duration_minutes`, `branch`, `pr`, `completed_at`, `notes`, plus a
tool-managed `synced_at`. Completing a handoff *means* filling it; the prompt
(`Prompt 3h`) and the artifact both say so.

The instruction to the agent is explicit about the failure mode: report the real
number from your own interface (`/cost`, the API `usage` object, your harness),
or set `null` **and say why** — **never invent one.**

### 3. `just handback-sync SPEC-NNN` transcribes

Reads every handoff for a spec, appends a `cost.sessions` entry per handback,
recomputes `cost.totals`. **Idempotent** via `synced_at`, so a re-run cannot
double-count. **Exits 1 naming any handoff that has not handed back cleanly**,
distinguishing *"hasn't reported"* from *"reported without a token count"* —
because those need different fixes (chase the agent vs. declare the platform
unmetered).

### 4. `null` is honest; a guess is not

A handback may legitimately carry `tokens_total: null` **only** when
`cost.metering_source: none` declares the platform has no meter. Under any other
metering source, a null blocks the sync with an actionable message. This keeps
the escape hatch a **deliberate, repo-level declaration** rather than a per-spec
shrug.

### 5. Stage-level `orchestration_cost` (the same problem, one layer up)

Framing a stage and deciding its spec breakdown happen **before any spec
exists**, so that spend had no artifact to attach to and recorded cost was
structurally under-counted. The stage template now carries an
`orchestration_cost:` slot.

Two constraints, both deliberate:

- **The orchestrator fills it, not the human.** A field that depends on someone
  remembering to jot a number will be empty forever. The instruction lives in
  the artifact.
- **Stage grain only.** Splitting orchestration across the specs it produced is a
  division you cannot observe; any per-spec share would be invented — the exact
  failure §4 rejects. Warn-only, no gate, no view yet: **capture first.**

## Alternatives considered

- **Orchestrator estimates delegated cost** (from duration, diff size, a rate
  card). Rejected — see Context §3. A plausible fabricated number is worse than
  a missing one because nothing downstream can tell them apart.
- **`metering_source: none` for every multi-agent repo.** Simple, and throws away
  real numbers that external agents *can* report. Rejected as the default; kept
  as the honest declaration for genuinely unmetered platforms.
- **A separate `HANDBACK-*.md` artifact.** Symmetric, but doubles the
  bookkeeping and adds a third ID sequence for one round trip. Rejected in favor
  of a section in the handoff — matching the spec's own `## Build Completion`
  precedent.
- **Auto-sync at `archive-spec`.** Convenient, but it would hide a missing
  handback at exactly the moment it stops being fixable. An explicit command
  that fails loudly is the point.
- **Per-spec orchestration attribution.** Rejected as false precision (§5).

## Consequences

- **Positive:** a non-Claude build/verify agent can satisfy the cost gate
  honestly, so a multi-agent repo keeps real cost provenance instead of
  disabling the gate. The un-returned handoff becomes a *detected* state rather
  than a silent gap. Pre-spec orchestration spend finally has somewhere to go.
- **Negative:** cost accuracy now depends on agents self-reporting truthfully —
  a **trust boundary the sub-agent path did not have**. It pairs with
  DEC-004 rule 1 (*reconcile over self-report*) and with `cost-audit`'s
  implausibly-low-cost warning, but neither can catch a plausible lie. Watch for
  a handback-derived analogue of harvest signal #5 (sub-agent metering silently
  undercounting).
- **Evidence level:** **N=1, built ahead of need** — `grebe` is the first real
  user and has not started. This is a deliberate exception to the deliberate-lag
  discipline, taken because the alternative was starting a multi-agent project
  with a cost gate that could only fail or be switched off. **Revisit at the
  first harvest that includes a plus-agents instance:** are handbacks actually
  filled, and do the reported numbers look plausible against duration and diff
  size? If agents routinely hand back `null`, the honest answer is
  `metering_source: none` for that platform, not more tooling.
