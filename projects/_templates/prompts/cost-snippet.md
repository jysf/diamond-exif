# Cost-session snippet for cycle prompts (paste into build / verify / ship prompts)

> Replaces the old "append a cost session with **null numerics**" line that let
> cost tracking silently go empty (AGENTS.md §4 / docs/cost-tracking.md). Do NOT
> write `null` for build/verify cycles.

**For a build or verify prompt** (the agent running the cycle records its own
real cost — the implementer for build, the reviewer for verify):

```
Append your cycle's entry to the spec front-matter `cost.sessions` with:
  - cycle: <build|verify>
    agent: <the model/tool you ran as>
    interface: <claude-code | api | ollama | other>
    basis: <transcript | agent-result | slash-cost | api-usage | estimate>
    tokens_total: <REAL number from your interface>   # /cost, the API usage
    estimated_usd: <REAL estimate>                     # object, or your tool's
    duration_minutes: <REAL minutes>                   # cost report
    recorded_at: <YYYY-MM-DD>
    notes: "<one line>"
Do NOT invent or null these. Measure from your own transcript where you have
one (docs/cost-tracking.md "Measuring a cycle": dedupe by message.id, max output
per message; `/cost` in the desktop Code tab reports context size, not spend),
otherwise the `usage` object on an API call or whatever your agent reports, and
say which in `basis`. If you cannot write the spec directly, carry the number across
in the handoff's `## Completion` block so the ship step can record it.
```

**For the ship step's bookkeeping** (whoever ships confirms the numbers landed):

```
Confirm each metered cycle (build, verify) has a real tokens_total and a basis
(`just handback-sync` copies both from the handoff). estimated_usd is priced
per component at the list rates of the model that actually ran, cache reads at
0.10× input and cache writes at 1.25× (5-minute) or 2× (1-hour); see
docs/cost-tracking.md "Pricing". Not tokens_total × one rate: that overstated
2.6–14.7× on cache-heavy cycles.
Leave design/ship (main-loop) numerics null with a
"main-loop, not separately metered" note.
Compute cost.totals: tokens_total = sum of non-null sessions (use 0, not null,
for the placeholder), estimated_usd = sum, session_count = number of cycles.
Then `just cost-audit` must pass before the spec is considered shipped.
```
