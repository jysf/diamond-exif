# Insight sources — template instances we learn from

The maintained registry of real repos built with (or tracked by) the template.
Each is a **feedback source**: friction and inventions found while shipping them
drive template changes. This list exists so gathering more insight is
*incremental* — scan for what changed since **Last reviewed**, rather than
re-deriving paths and re-reading everything.

**Discipline of this file:** update **Last reviewed** whenever an instance is read
for signals, and point **Insights captured in** at where the digested findings
already live (a harvest doc, `feedback/`, or a `docs/ROADMAP.md` thread) so we
never re-harvest what's already triaged. Repo names/URLs only — machine-local
paths are intentionally kept out (portability + hygiene; see the standup
`standup.toml` precedent).

> This is distinct from [`PROJECTS.md`](../../PROJECTS.md) (the public "built with
> this template" showcase) and from standup's operational tracked-repos list —
> though all three should stay roughly aligned.

## Registry

| Repo | Variant | Tier | Insights captured in | Last reviewed |
|---|---|---|---|---|
| [`bragfile000`](https://github.com/jysf/bragfile000) | claude-only | full | `feedback/2026-04-20-bragfile-project.md`; [harvest 2026-07-06](2026-07-06-three-project-dogfood-harvest.md) | 2026-07-06 |
| `crustyimg` | claude-only | full | [harvest 2026-07-06](2026-07-06-three-project-dogfood-harvest.md) (framework-feedback) | 2026-07-06 |
| [`zany-animal-slots`](https://github.com/jysf/zany-animal-slots) | claude-only | full | `feedback/2026-07-0{3,4}-*`; [harvest 2026-07-06](2026-07-06-three-project-dogfood-harvest.md); lifetime-modes split → v0.6.17 | 2026-07-12 |
| ~~`bragfile-report`~~ | — | **NOT AN INSTANCE** — a stale checkout of `bragfile000` | — | delisted 2026-08-13 |
| `rspeed` | claude-only | full | [`PROJECTS.md`](../../PROJECTS.md) note only | **never harvested** |
| `uw` | **claude-plus-agents** | full | — | **dead — abandoned, nothing to harvest** (was the only plus-agents instance) |
| [`skillport`](https://github.com/jysf/skillport) | claude-only | full | v0.6.22 spec titles + the v0.6.19 Reflection-Q5 defect → v0.6.24 (see `CHANGELOG.md`) | 2026-07-18 |
| [`standup`](https://github.com/jysf/localStandupPlus000) | claude-only | full + **consumer/product** | standup design docs; deep-dive 2026-07-13 (memory); its own `docs/design/` | 2026-07-13 |
| `blog` | — | declared-lite (`.standup.yaml`) | — | — |

**Legend** — Tier: `full` = `.repo-context.yaml` spec-driven; `declared-lite` =
`.standup.yaml`; `non-template` = tracked but not scaffolded.

> A **Discipline** column (explicit vs loose adherence to frame→design→build→
> verify→ship) was carried here for two weeks and **removed 2026-07-28**: the
> owner doesn't recognize the distinction, so no one could ever fill it. That is
> itself a finding — treat every full-tier instance's friction as real template
> friction, rather than discounting it as "they skipped the process".

## To confirm (owner)

- ~~**Canonical vs stale checkouts**~~ — **one was resolved 2026-08-13, and it had
  been inflating every corpus figure.** `bragfile-report` was listed here as a
  separate full-tier "aggregator" instance. It is not an instance: it shares
  `bragfile000`'s remote, carries the same first three projects, and sits 195
  commits into the same history (296 at `bragfile000` HEAD). Verified as a
  **strict ancestor — no divergence**, so the "a clone forks the memory" hazard
  argued in `ROADMAP.md` still has **zero observed instances**; the discipline is
  holding. What failed was the registry, not the practice.
  **Consequence:** every count that treated it as an instance double-counted
  **41 specs / 40 shipped / 25 DECs** — including the 2026-07-06 harvest and the
  2026-08-12 external comparison ("8 live instances · 208 DECs"). Corrected
  figures are in `ROADMAP.md`. Remaining checkouts (crustyimg ×3, zany worktree,
  standup ×3) are still unverified and should be treated as suspect until they
  are.
- **Unharvested blind spots:** `rspeed` has never been read for signals — worth a
  light pass if it is active. `uw` was the other, and is now **dead**: marked
  abandoned 2026-08-12 rather than left reading as an outstanding blind spot.
  Note the registry cannot express *why* it ended — "never harvested" and
  "abandoned, nothing to harvest" are different facts, which is the
  `closed_reason:` gap from [`ROADMAP.md`](../ROADMAP.md) one level up. The
  plus-agents blind spot itself closes with the 1–3 plus-agents projects starting
  2026-08-12.
- **`skillport`** is the newest and most active instance (two projects:
  `PROJ-001-skillport-lint`, `PROJ-002-skillport-audit`). It has already paid for
  itself twice — it surfaced the ID-only spec ledger (→ v0.6.22) and the missing
  Reflection-Q5 answer slot (→ v0.6.24) — but has never had a *deliberate* harvest
  pass; both findings arrived incidentally. It is the natural next harvest, and the
  live validation ground for `frame-stage` (do outlines survive?) and `calibration`
  (do the token bands stabilize?).
