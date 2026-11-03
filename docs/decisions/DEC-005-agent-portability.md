---
insight:
  id: DEC-005
  type: decision
  confidence: 0.7
status: accepted            # proposed | accepted | superseded
created_at: 2026-06-27
deciders: [jysf, claude]
supersedes: null
superseded_by: null
affected_scope:
  - "variants/*/.repo-context.yaml"
  - "variants/*/projects/_templates/spec.md"
  - "variants/*/projects/_templates/prompts/cost-snippet.md"
  - "variants/*/FIRST_SESSION_PROMPTS.md"
  - "variants/*/docs/cost-tracking.md"
  - "scripts/cost-audit.sh"
tags: [architecture, portability, cost, non-claude, config]
---

# DEC-005: run on non-Claude agents — parameterize the model + cost seams

> **This is the template's own decision log** (meta). **Status: accepted —
> fully implemented.** Phase 1 (v0.5.25): the `spec.agent`/`spec.cost` config in
> `.repo-context.yaml`, a `cost-audit` that honors `metering_source` (`none`
> disables the gate), and `docs/porting.md`. Phase 2 (v0.5.26): `new-spec` /
> `new-patch` stamp `agents.*` (and the plus-agents `handoff.from_agent`) from
> the `tier_map`, and the "new Claude session" AGENTS wording is generalized to
> "new session." It is the dependency named in
> [DEC-004 §5](DEC-004-subagent-execution-mode.md): DEC-004's rule 3 (model
> config) consumes the config this decision defines. Builds on
> [DEC-002](DEC-002-cost-convention.md) (the `cost.*` extension).

## Context

The template is **~70% agent-portable already**, by design, not accident:

- `AGENTS.md` is the emerging **cross-tool standard** (Cursor, Aider, Codex CLI,
  Continue, Gemini CLI all read it); `CLAUDE.md` is just a pointer.
- The `claude-plus-agents` variant's **`handoff.to_agent` is already
  agent-agnostic** — its example values are non-Claude (`kilo-code`,
  `factory-droid`, `adal`).
- `cost.interface` already enumerates `api | ollama | other`, and the whole
  workflow rests on **spec-as-source-of-truth**, which is what lets *any* fresh
  agent pick up a cycle (no "as I said earlier").
- `just` recipes, the front-matter schema, the DEC log, the gates, and the
  independent-verify discipline are all runtime-agnostic.

Both structured dogfood harvests (crustyimg + zany-animal-slots) independently
ran an "if this ran on a non-Claude agent, what breaks?" pass and converged on
the **same four seams** — all concentrated in the **model + cost** layer:

| # | Seam | Where it's hard-coded today |
|---|---|---|
| 1 | **Cost metering source** | `tokens_total` comes from Claude's `subagent_tokens` / `/cost`; `cost-audit` is a **hard gate** with no equivalent source on another platform → the whole "metered subagent" premise collapses. |
| 2 | **Model-id + price** | Specs hard-code `claude-opus-4-8` / `claude-sonnet-4-6` in `agents.*` and `cost.sessions[].agent`; estimates assume a Claude `$/M` rate — both wrong on GPT/Gemini. |
| 3 | **Model-tier prompt wording** | The design=Opus / build=Sonnet split and "fresh **Claude** session" phrasing assume the Claude model family. |
| 4 | **Sub-agent mechanics** | The shared-tree / auto-background hazard is Agent-tool-specific. |

Seam 4 is covered by [DEC-004](DEC-004-subagent-execution-mode.md). This decision
handles seams 1–3: **the deepest coupling is the cost model** — and it's a *hard
gate*, so it's the one that actually blocks a non-Claude run.

## Decision (proposed)

Parameterize the three model/cost seams behind a small, **defaulted** config so
the template runs on another agent by editing config, not by forking. Defaults
reproduce today's Claude behavior exactly (zero change for existing instances).

### 1. An `agent` + `cost` config block in `.repo-context.yaml`

```yaml
spec:
  agent:
    default_model: claude-opus-4-7        # stamped into new specs' agents.* / cost sessions
    tier_map:                             # the design=Opus/build=Sonnet idea, made pluggable
      design: claude-opus-4-7
      build:  claude-sonnet-4-6
      verify: claude-opus-4-7
  cost:
    metering_source: subagent_tokens      # subagent_tokens | api_usage | manual | none
    rate_per_mtok: 6.60                    # USD per 1M tokens, for the estimate (DEC-002 basis)
    currency: USD
```

`new-spec` / `new-patch` stamp `agents.*` from `default_model`/`tier_map` instead
of the hard-coded `claude-opus-4-7`; the cost snippets reference `rate_per_mtok`
instead of a literal rate. A non-Claude instance sets these once.

### 2. `cost-audit` degrades gracefully when there's no meter

The hard gate assumes a metering source exists. Make it **honor
`metering_source`**: with a real source (`subagent_tokens` / `api_usage`) the gate
is unchanged (a shipped spec/patch needs a real `tokens_total`); with
`metering_source: manual` it stays a gate but accepts a hand-entered basis-annotated
number; with `metering_source: none` (a platform that exposes no token count) the
gate **downgrades to a warning** — cost stays *captured where possible* but is no
longer a build-blocker with no source. This keeps the discipline where it's
enforceable and stops it being a false blocker where it isn't.

### 3. Generalize the Claude-specific prompt wording

- "fresh **Claude** session" → "fresh **session/agent**" across
  `FIRST_SESSION_PROMPTS.md` (both variants). The `claude-only` variant's premise
  ("separate cheap sessions, same agent") is the one genuinely Claude-shaped
  thing — name it as a variant assumption, not a universal.
- The design/build/verify model choices reference the **`tier_map`** by role, not
  literal Claude model names.
- Ship a short **`docs/porting.md`**: point tool X at `AGENTS.md`, set the
  `agent`/`cost` config, pick a `metering_source`, and what stays manual. (This is
  the "porting to another agent" doc the roadmap called for.)

## What already ports (do NOT touch)

Spec-as-source-of-truth; `AGENTS.md` as the contract; `handoff.to_agent`; the
front-matter schema + `--json`; the DEC log; the gates; the independent-verify
discipline; the `cost.interface` enum. The "Implementation Context folded into the
spec" design ports especially cleanly — it removes the separate-handoff object a
cheap-fresh-context agent might not have.

## Consequences

- **The template becomes runnable on a non-Claude agent by configuration**, not a
  fork — the credible answer to "is this useful beyond one author + one vendor?"
  alongside the DEC-002 ContextCore upstream.
- **The cost gate stops being a false blocker** on platforms without a token meter,
  while staying a real gate where a meter exists.
- **Cost estimates stay honest and reproducible** (DEC-002): the `$/M` rate becomes
  an explicit, per-instance config value with a basis, not a buried literal.

## Open questions

1. **Config home** — `.repo-context.yaml` (proposed, it's already the repo-identity
   file) vs a dedicated `agents.yaml`. Leaning `.repo-context.yaml` to avoid a new
   file.
2. **`metering_source: none` → warn vs advisory-constraint** — does the
   `cost-captured-per-cycle` constraint flip to `advisory`, or does `cost-audit`
   just warn? (Prefer: constraint severity follows `metering_source`.)
3. **Per-model rate table** — one `rate_per_mtok`, or a per-model map (Opus vs
   Sonnet vs GPT differ)? Start with one; the `cost.estimate_basis` free-text
   already carries the nuance (DEC-002).
4. **How far to generalize wording vs. add a variant** — is a third
   "generic-agent" variant ever warranted, or is config + wording enough? (Prefer
   config; a variant is a maintenance multiplier.)
5. **Sequencing** — this is additive and defaulted, so it can land before or after
   DEC-004; DEC-004 rule 3 simply reads `tier_map` once it exists.
