# Porting to another agent (non-Claude)

This template is designed to run on more than Claude Code. Most of it is already
agent-agnostic — `AGENTS.md` is the emerging cross-tool standard (Cursor, Aider,
Codex CLI, Continue, Gemini CLI all read it), the front-matter schema and `just`
recipes are runtime-neutral, and the whole workflow rests on
**spec-as-source-of-truth**, which is what lets *any* fresh agent pick up a cycle.
The coupling to Claude is concentrated in one place — the **model + cost** layer —
and this doc is how you parameterize it. (Design record: `DEC-005`.)

## 1. Point the tool at `AGENTS.md`

`AGENTS.md` is the single source of truth for conventions. Most agent tools read
it automatically; if yours reads a differently-named file, symlink it
(`ln -s AGENTS.md <name>`, as `CLAUDE.md` already does).

## 2. Set the agent + cost config in `.repo-context.yaml`

Under `spec:` there is an `agent` and a `cost` block. The defaults reproduce the
Claude-Code workflow; edit them once for your agent:

```yaml
spec:
  agent:
    default_model: <your-model-id>     # e.g. gpt-5, gemini-2.5-pro — stamped into new specs
    tier_map:                          # which model runs which cycle
      design: <model>
      build:  <model>
      verify: <model>
  cost:
    metering_source: <see below>
    rate_per_mtok: <USD per 1M tokens for your model>
    currency: USD
```

## 3. Pick a `metering_source` (this is the one hard gate)

`just cost-audit` is a **blocking** gate that requires a real `tokens_total` on
metered cycles. Claude exposes that via `subagent_tokens` / `/cost`; another
platform may not. Set `metering_source` to match yours:

| Value | Meaning | `cost-audit` behavior |
|---|---|---|
| `subagent_tokens` | Claude Code sub-agent token counts (default) | enforced |
| `api_usage` | your API returns per-run token usage | enforced |
| `manual` | a human records the token count each cycle | enforced |
| `none` | the platform exposes no token count | **gate disabled** (cost captured where possible, but not a build-blocker) |

Setting `metering_source: none` is what unblocks a platform with no token meter —
the gate stops blocking on a number that can't exist, instead of failing every
shipped spec.

## What ports cleanly (no change needed)

Spec-as-source-of-truth · `AGENTS.md` as the contract · the front-matter schema +
`--json` · the DEC log + `decisions-audit` · the gates (`validate`, and
`cost-audit` per above) · the independent-verify discipline · and — in the
`claude-plus-agents` variant — `handoff.to_agent`, which is already agent-agnostic
(its examples are non-Claude). The "Implementation Context folded into the spec"
design ports especially well: it removes the separate-handoff object a tool
without cheap fresh contexts might not have.

## What is still Claude-shaped (know before you start)

- **The `claude-only` variant's premise** — "separate cheap sessions, same agent
  per cycle" — assumes a tool with cheap fresh contexts. An agent without that
  will find the per-cycle-session ritual awkward; the spec still carries all the
  context, so the work is unaffected, only the ceremony. (This is the one
  genuinely Claude-shaped assumption; the `claude-plus-agents` variant, with its
  `handoff.to_agent`, is the more agent-neutral one.)
- **Model ids are now stamped from your config**, not hardcoded — `new-spec` /
  `new-patch` fill `agents.*` (and the plus-agents `handoff.from_agent`) from the
  `tier_map`, so a scaffolded spec already carries your models, not Claude's.

## Status

**Fully implemented** (DEC-005, v0.5.25–v0.5.26): the `agent`/`cost` config block,
a `cost-audit` that honors `metering_source`, `agents.*` stamped from the config,
and generalized session wording. Set the config in `.repo-context.yaml` and you're
running on your agent — nothing here is left to edit by hand.
