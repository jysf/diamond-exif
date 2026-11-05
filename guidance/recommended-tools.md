# Recommended tools (optional)

This template runs with **zero external dependencies** — markdown, a
`justfile`, and pure-bash scripts. Nothing in this file is required.

It's a catalog of external tools worth reaching for *when a project's
needs outgrow the defaults*. Each entry says when to reach for it and
when to skip it. Adopting any of these is a **project-level** choice:
record it as a `DEC-*` in `/decisions/`, add it to the project's setup
docs, and keep it out of the template defaults so a fresh clone stays
dependency-free.

The bias: prefer the in-repo, text-based, LLM-authorable default; only
escalate to a heavier tool when the payoff is real.

---

## Diagrams

### Mermaid — the default (already in use)

<https://mermaid.js.org>

Diagrams in this repo are authored as fenced ```` ```mermaid ```` blocks
directly in markdown — see the example in `/docs/architecture.md`. This
is the default and it's deliberate:

- **Zero dependency.** It's just text in a `.md` file.
- **Renders where the docs live.** GitHub, GitLab, and most markdown
  viewers render Mermaid natively, so diagrams show up on the repo page
  with no build step.
- **Agents can maintain it.** Claude can write and *update* a Mermaid
  block in the same file it's already editing, so diagrams stay current
  through the design/build cycle instead of rotting.

Keep architecture, data-model, and flow diagrams as Mermaid in `/docs/`,
`/decisions/`, and specs. Update them as part of the work, not after.

### Structurizr — optional, for C4 at scale

<https://structurizr.com>

Structurizr models architecture once (the C4 model) and renders multiple
consistent views — context, container, component — that can't drift apart.

**Consider it when:**
- The architecture is large and long-lived, with many diagrams that must
  stay mutually consistent.
- You want enforced C4 rigor across a team.

**Skip it when:**
- A handful of Mermaid diagrams cover it. Most projects never outgrow
  Mermaid.

**No lock-in either way.** Structurizr's CLI exports to Mermaid, so the
clean path is Mermaid-first, escalating to Structurizr only if a project
genuinely needs C4 — and you can still render Mermaid from it. Note it's
a real dependency: a paid SaaS account or the self-hosted "Lite" server.

---

## Testing / Verify phase

The **Verify** cycle is convention-driven by default: a spec ships when
its acceptance criteria are met, tests pass, and there's no decision
drift (see `AGENTS.md` → "During verify"). The tools below help when a
project's verification needs outgrow in-process tests.

### LineSpec — protocol-level integration tests

<https://linespec.dev>

LineSpec intercepts MySQL, PostgreSQL, HTTP, Kafka, gRPC, and Redis at
the protocol level and drives them from a language-agnostic DSL
(`RECEIVE` / `EXPECT` / `VERIFY` / `RESPOND`), so the tests live outside
your application code.

**Consider it when:**
- A spec's acceptance criteria are about *protocol behavior* —
  request/response shapes, DB queries issued, message contracts — not
  just return values.
- You're verifying across a service boundary and mocks keep drifting
  from the real wire format.
- The implementer and the app are in different languages and you want
  one test suite that doesn't care.

**Skip it when:**
- Unit / integration tests in the app's own framework already cover the
  criteria. Don't add an infra dependency you don't need.
- The app has no meaningful DB/HTTP/queue traffic to assert on.

If you adopt it, reference the `.linespec` files from the relevant
spec's acceptance criteria.

---

## Decisions

### Native `just decisions-audit` — the default

Documenting and enforcing architectural decisions is handled in-repo by
`/decisions/` plus:

```bash
just decisions-audit             # structural lint + scope-conflict warnings
just decisions-audit --changed   # which decisions govern your pending changes
```

The optional `affected_scope:` glob list in a decision's front-matter
powers the scope checks (see `/decisions/_template.md`). For commit-time
enforcement, wire `just decisions-audit --changed` into a pre-commit hook.

### LineSpec Provenance Records — optional

LineSpec's *other* half (Provenance Records) is a binary-backed version
of the same idea — YAML decision records with git hooks and semantic
search over decisions via embeddings. It overlaps with what
`/decisions/` + `just decisions-audit` already do natively, so the
template doesn't depend on it. Reach for it only if you specifically want
embedding-based semantic search across a large decision history.

---

## Accomplishment logging (at ship) — on by default via `brag`

When a spec, stage, or project ships, **record the win with impact** — for
retros, performance reviews, and résumés. This is **on by default** (DEC-010):
the tool is [`brag`](https://github.com/) — a local-first CLI (`brag add`) that
also exposes an MCP server (`brag mcp serve`). Declared in `.repo-context.yaml` →
`spec.accomplishments` (`enabled` / `tool`); set `enabled: false` to opt out, or
swap `tool`. **The agent calls the tool directly — there is no wrapper.**

**When** — at the `ship` cycle (AGENTS §15):
- Per shipped spec, or batch a stage's specs into one entry at stage-ship.
- At stage-ship: the user-visible capability the stage delivered.
- At project-ship: whether the project's `value.thesis` held up.

**How** — run `brag add` directly, seeding the entry from the spec's title +
`value_link` + `cost.totals`:

```
brag add \
  -t "<short headline of what shipped>" \
  -p "<project>" -k shipped -T "<comma,tags>" \
  -d "<what + how, 2–4 sentences>" \
  -i "<IMPACT — see below>"
```

- **Project** auto-fills when you're inside a registered brag project (`brag
  project here`); register this repo once with `brag project new`.
- **MCP path** (for an agent that speaks MCP): run `brag mcp serve` and call the
  `brag_add` tool with the same fields — no shell-out needed.
- **Scripted / non-interactive:** `echo '{"title":"…","impact":"…"}' | brag add --json`
  (stdout is just the entry ID) — safe for a build/verify sub-agent to call.

**How to think about impact** (the `-i` field — the one that matters):
Impact is the *outcome*, not the output. "Shipped the logger" is output;
"structured logging cut incident-triage time, unblocking on-call" is impact.
A good impact line answers *who or what is better off, and by how much.*

- Prefer a metric, a quote, or a concrete unblock over an adjective.
- Reuse the value you already wrote — the spec's `value_link`, the stage's
  `value_contribution.delivers`, the project's `value.thesis`. The ship
  Reflection is where this crystallizes; the brag entry is its outward form.
- Pair it with cost: a spec's `cost.totals` lets you frame "delivered &lt;impact&gt;
  for ~$&lt;spend&gt;" — value-per-dollar, which this template already tracks.
- Be honest (AGENTS §17, confidence discipline). A defensible, order-of-
  magnitude claim beats a grand vague one.
