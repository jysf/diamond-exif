---
insight:
  id: DEC-010
  type: decision
  confidence: 0.8
status: accepted            # proposed | accepted | superseded
created_at: 2026-07-06
deciders: [jysf, claude]
supersedes: null
superseded_by: null
affected_scope:
  - "variants/*/.repo-context.yaml"
  - "variants/*/guidance/recommended-tools.md"
  - "variants/*/AGENTS.md"
tags: [architecture, value, accomplishments, brag, ship]
---

# DEC-010: accomplishment logging on by default (via `brag`)

> **This is the template's own decision log** (meta). **Status: accepted —
> shipped in v0.6.10, simplified in v0.6.11.** Reverses the earlier "keep it out
> of the template defaults" stance for accomplishment logging.
>
> **v0.6.11 note:** the first cut (v0.6.10) added a `just log-win` wrapper +
> `scripts/log-accomplishment.sh` that pre-filled `brag add`. That was over-built
> — the agent can call `brag` (CLI or MCP) directly. The wrapper, its recipe, and
> the `get_accomplishments_field` helper were removed; **the template now only
> coaches** (config declaration + AGENTS/recommended-tools guidance). The agent
> runs the tool itself.

## Context

The template already told agents to log a shipped win *with impact* at ship, but
framed it as **optional** and "keep it out of the template defaults" — so it kept
getting skipped (including on the template's own work this session). Meanwhile the
default tool, [`brag`](https://github.com/) (a local-first CLI + MCP server for
capturing career accomplishments), is a first-party tool here and one of the
dogfood projects — and it has exactly the right seam: `brag add -i "<impact>"`,
`brag mcp serve`, and per-directory project auto-fill. Impact capture is also the
outward form of the value the template *already* records (`value_link`,
`cost.totals`), so making it default-on is nearly free and closes the loop from
"we tracked value" to "we can report it."

## Decision

**Accomplishment logging is on by default**, tool-configured (default `brag`) —
and it is **coaching, not machinery**: the agent calls the tool directly.

- **Config declaration:** `.repo-context.yaml` → `spec.accomplishments`
  (`enabled: true`, `tool: brag`). Set `enabled: false` to opt out; swap `tool`
  for an equivalent. Declarative context the agent reads — no script consumes it.
- **The agent runs the tool itself:** at ship it calls `brag add -i "<impact>"`
  (CLI) or the `brag_add` tool over `brag mcp serve` (MCP), seeding the entry from
  the spec's **title + `value_link` + `cost.totals`** (framing value-per-dollar).
  No template wrapper — `brag` already has the right seam.
- **Both interfaces documented** in `guidance/recommended-tools.md`: the CLI
  (`brag add`, scripted `--json` mode safe for a sub-agent) and the MCP server.
- **AGENTS "During ship"** (both variants) now says *log the win* (default-on),
  not *optionally log it*.

## Alternatives considered

- **Keep it optional** (status quo) — rejected: it kept being skipped, and the
  impact record is the payoff of the value/cost tracking the template already does.
- **Hard-wire `brag` with no config** — rejected: the template serves other
  users; the config keeps it swappable / opt-out while defaulting to brag.
- **A cost-audit-style gate** — rejected: impact framing is judgment-laden and
  personal; coaching is the right weight (a gate would punish honest "nothing
  brag-worthy" ships).
- **A template wrapper (`just log-win`)** — shipped briefly in v0.6.10, removed in
  v0.6.11: it wrapped a first-party tool the agent can just call. Coaching + a
  config declaration is the right weight; the wrapper was maintenance for no gain.

## Consequences

- Every shipped spec/stage/project gets an impact-framed brag entry, seeded from
  data already in the spec — with **no template machinery** (the agent runs
  `brag` directly, CLI or MCP).
- Nothing to maintain beyond a config declaration + the guidance; a non-brag user
  sets `enabled: false` or swaps `tool`, and is never blocked (no gate).
- Reinforces the value loop: `value_link` → a direct `brag add` → a portable
  entry that can feed retros/reviews/résumés.

## Open questions

1. **Auto-run at `archive-spec`?** Ship could fire the brag entry automatically
   rather than leaving it to the agent. Kept manual for now (an explicit, editable
   impact line beats an auto-generated one) — and it avoids re-introducing a
   wrapper.
2. **MCP-first for delegated agents** — when a build/verify sub-agent ships, is
   `brag mcp` the cleaner path than shelling out to the CLI? Revisit with the
   sub-agent delegation work (DEC-004).
