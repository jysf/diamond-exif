# Cost tracking — how it works and how it's enforced

Every spec records its AI cost per cycle so reports can aggregate spend over time
(AGENTS.md §4). This is the operational reference.

## The schema (per spec front-matter)

```yaml
cost:
  sessions:
    - cycle: build            # frame | design | build | verify | ship
      agent: <model you ran as>
      interface: claude-code  # claude-code | claude-ai | api | ollama | other
      tokens_total: 130653    # ONE combined number (the harness reports one)
      estimated_usd: 0.71     # order-of-magnitude (see rate note)
      duration_minutes: 22
      recorded_at: 2026-06-15
      notes: "…"
  totals:
    tokens_total: 201141      # sum of non-null sessions (use 0, never null)
    estimated_usd: 1.34
    session_count: 4
```

**`tokens_total` is the schema** — a single combined token count, because that is
what the harness surfaces (`subagent_tokens` in an `Agent` result, or `/cost` in
an interactive session). There is no reliable input/output split, so don't try to
record one. (The reporting lib also still sums legacy `tokens_input`/`tokens_output`
if present, for forward-compatibility.)

## Where the numbers come from

- **build / verify cycles** are the metered ones. The agent that runs the cycle
  reads its real token count from its interface — `subagent_tokens` +
  `duration_ms` from an `Agent` result when the cycle ran as a metered subagent,
  `/cost` when run interactively, the `usage` object for an API call, or whatever
  a third-party agent reports — and the real `tokens_total` / `duration_minutes` /
  `estimated_usd` are written into the spec at **ship**.
- **design / ship cycles** are main-loop work with no clean per-cycle metering —
  leave their numerics `null` with a "main-loop, not separately metered" note.
- **`estimated_usd`** = `tokens_total` × your model's published list rate, with no
  cache discount. It is explicitly order-of-magnitude — say so in the entry's note.

## How it's enforced (so it can't silently go empty again)

Cost tracking can be structurally present but empty: a spec ships with all-null
numerics because documentation alone is skippable (build prompts that say "null
numerics" reintroduce the gap). The enforcement makes it mechanical — in layers:

1. **Rule** — AGENTS.md §4 (capture real numbers; null only for main-loop cycles)
   + constraint `cost-captured-per-cycle` in `guidance/constraints.yaml`.
2. **Check (the teeth)** — `just cost-audit` (`scripts/cost-audit.sh`) fails if any
   *shipped* spec lacks a positive `tokens_total` on its build/verify cycles. It
   runs in CI (the `cost-data` job) and is surfaced in `just status` ("Specs
   missing cost data"); `report-weekly` flags the same.
3. **No-regression** — the cost wording lives in
   `projects/_templates/prompts/cost-snippet.md`, so new cycle prompts don't
   re-introduce the "null numerics" line.

### Where the enforcement lives

| Layer | File |
|---|---|
| The rule | `AGENTS.md` §4 |
| The constraint | `guidance/constraints.yaml` → `cost-captured-per-cycle` |
| The gate | `scripts/cost-audit.sh` (recipe `just cost-audit`) |
| Audit helpers | `scripts/_lib.sh` (`is_grandfathered_cost`, `cycle_tokens_total`, `spec_missing_cost_cycles`) |
| Status surfacing | `scripts/status.sh` → "Specs missing cost data" |
| Weekly surfacing | `scripts/report_weekly.sh` → "Shipped without cost data" |
| Per-stage / grand totals | `scripts/specs-by-stage.sh` → cost column + stage subtotal + "Recorded cost" |
| CI | `.github/workflows/ci.yml` → `cost-data` job |
| Prompt wording | `projects/_templates/prompts/cost-snippet.md` |
| Concurrency hygiene | `AGENTS.md` §16 — one git worktree per concurrent session (parallel agents can clobber an uncommitted edit or land a commit on the wrong branch) |

### Grandfathering

`COST_AUDIT_GRANDFATHERED` in `scripts/_lib.sh` lists specs that predate the
cost-capture process and whose real numbers are unrecoverable; the audit skips
them. **It starts empty** — a fresh project has no pre-process history. Populate
it (space-separated `SPEC-NNN` ids, or override via the env var) only if you adopt
the gate after already shipping specs without cost data.

## Reports

`just report-weekly` aggregates cost by spec / cycle / interface, plus total,
avg-per-shipped-spec, top cost drivers, and the "shipped without cost data" flag.
`just status` shows the missing-cost list, and `just specs-by-stage` shows a cost
column with per-stage subtotals and a grand "Recorded cost" total. The data is
only as good as what's recorded — it gets richer as specs ship under the enforced
process.
