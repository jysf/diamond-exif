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
      basis: transcript       # HOW tokens_total was measured — see "Basis" below
      tokens_total: 130653    # ONE combined number (the harness reports one)
      estimated_usd: 0.71     # priced per component (see "Pricing")
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
  measures its own cycle (below) and the real `tokens_total` /
  `duration_minutes` / `estimated_usd` are written into the spec at **ship**.
- **design / ship cycles** are main-loop work with no clean per-cycle metering —
  leave their numerics `null` with a "main-loop, not separately metered" note.
  Orchestrator spend is recorded once per stage, in the stage's
  `orchestration_cost`, never split across specs.

## Measuring a cycle

Two instances (crustyimg, irradiance) measured their recorded figures against
the raw transcripts and found every shortcut wrong, mostly by large factors.
This is the method they converged on. Each rule cites what it fixes.

1. **Read the cycle's own transcript.** Claude Code writes one JSONL file per
   session under `~/.claude/projects/<dir>/<session-id>.jsonl`. A session in a
   git worktree has **its own `<dir>`**, and a subagent's transcript sits under
   `<session-id>/subagents/`. Search all of them: crustyimg concluded 22 records
   were unrecoverable, and 12 of them were in sibling directories.
2. **Count each message once.** A message with several content blocks is written
   as one line per block, and every line repeats the same `usage`. Group lines
   by `message.id`. Summing every line over-counts **1.6–2.25×** (measured on
   five irradiance transcripts).
3. **Within a message, take the largest `output_tokens`.** The lines of one
   message repeat the same `input_tokens`, `cache_creation_input_tokens` and
   `cache_read_input_tokens`, while `output_tokens` grows. Take those three once
   and output at its maximum: the first line's output under-counts about 11×
   (crustyimg).
4. **`tokens_total` = input + cache creation + cache read + output**, summed over
   the deduplicated messages. On a long agentic cycle cache reads are usually
   95%+ of it, so treat it as a spend indicator, not a measure of effort.
5. **Leave out the orchestrator.** The orchestrating session's transcript lives
   in the same directory, runs at the same time and mentions the same spec; it is
   not part of the cycle. Its span encloses the cycle's, and its session id is not
   the one in your scratchpad path. Its spend belongs in `orchestration_cost`.
6. **Re-sum after the session ends.** A figure taken while the session is live is
   a floor: irradiance's re-sums came out 17% and 25% higher. If you record it
   live, say "floor" in `notes`.
7. **`/cost` is not available everywhere.** In the desktop app's Code tab it
   reports context-window occupancy, not spend (irradiance, eight cycles). Use the
   transcript there.

### Basis

`basis:` says which of these numbers a session holds. They are different
quantities, not different readings of one: irradiance measured an `Agent`
result's `subagent_tokens` at 197,940 against 5.2M for the same cycle's
deduplicated transcript. So `just calibration` groups by basis and warns when a
band mixes them, and an entry without one counts as unrecorded.

| `basis` | Where the number came from |
|---|---|
| `transcript` | the cycle's own transcript, measured as above |
| `agent-result` | `subagent_tokens` from an `Agent` result |
| `slash-cost` | `/cost` at the end of an interactive session |
| `api-usage` | the `usage` objects of the API calls, summed |
| `estimate` | not measured (e.g. claude.ai, estimated from length) |

The set is open: another value is reported, never rejected.

### Pricing

Price each component separately, at the list prices of **the model that ran**
(`message.model` in the transcript, also written to `agent`), not the model a
prompt or `tier_map` named; irradiance's tier map predicted the build model
wrong 6 times in 7, and pricing at the wrong model overstated one spec 5×.

    estimated_usd = (input × in_rate) + (output × out_rate)
                  + (cache_read × in_rate × 0.10)
                  + (cache_creation × in_rate × 1.25)   # 5-minute cache writes
                                                        # (× 2.0 for 1-hour writes)

The flat `tokens_total × list rate` rule this file used to give is withdrawn.
It ignored the cache discount on the 95%+ of tokens that are cache reads, and
overstated by 2.6–14.7× across six measured cycles. Entries recorded under it
are not comparable with entries recorded under this one; don't sum across the
change without saying so.

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
