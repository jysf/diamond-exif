# How to use this template

A practical, end-to-end walkthrough. The README covers setup and the two
variants; this goes deeper into the daily loop. If you're brand new,
`just init` then open `GETTING_STARTED.md` first — this doc is the
reference you come back to.

For the wider arc — shaping and decomposing an idea *before* you have a
repo, the three on-ramps (greenfield / prototype-first / existing repo),
closing a wave, and how to tell whether the process is earning its keep
— see [PLAYBOOK.md](PLAYBOOK.md). That's the map; this is the terrain.

## The mental model

```
Repo (the app — persists forever)
 └─ Project (a wave of work: "MVP", "v2", "redesign")
     └─ Stage (a coherent chunk: 2–5 per project)
         └─ Spec (one implementable task)
              └─ Cycle (Frame → Design → Build → Verify → Ship)
```

The repo *is* the app and outlives every project. Architecture,
conventions, constraints, and decisions accumulate at repo level
(`docs/`, `guidance/`, `decisions/`) so they survive across waves of
work. Projects are bounded; specs are individual tasks; the cycle is the
five phases each spec moves through.

## Day 0 — initialize

```bash
just init          # pick claude-only or claude-plus-agents; scaffolds the root
```

Then describe the app once in `.repo-context.yaml`, and skim
`AGENTS.md` — it's the single source of truth every agent reads.

## Start a project, a stage, a spec

A **project** is a brief. Copy `projects/_templates/project-brief.md`
into `projects/PROJ-NNN-<slug>/brief.md` and fill in the value thesis
(see `GETTING_STARTED.md` and Prompt 1a in `FIRST_SESSION_PROMPTS.md`).

A **stage** and a **spec** are scaffolded for you, with IDs and
front-matter filled in:

```bash
just new-stage "Foundational infra"            # → STAGE-NNN in the active project
just new-spec  "Logger module" STAGE-001       # → SPEC-NNN under that stage
```

Once a stage's `## Spec Backlog` lists the work as `- [ ] (not yet written) — …`
lines, promote the whole batch in one step:

```bash
just frame-stage STAGE-001 --dry-run           # show the ID assignment, write nothing
just frame-stage STAGE-001                     # → one OUTLINE spec per backlog line
```

Each becomes a spec at `cycle: frame` with a **stable ID**, and its backlog line
is rewritten to `- [ ] SPEC-NNN (frame) — …`. Stable IDs are the point: you can
now declare `depends_on:` between them and let `just ready` compute the
dispatchable set. An outline records **scope and dependencies only** — the
approach is designed just-in-time when it advances to `design`.

## The five-phase cycle

Each spec moves through the cycle; `task.cycle` in its front-matter is
the source of truth, advanced with `just advance-cycle`:

```bash
just advance-cycle SPEC-001 design
just advance-cycle SPEC-001 build     # (allowlisted: frame|design|build|verify|ship)
```

| Phase | What happens | Commands / artifacts |
|---|---|---|
| **Frame** | Why this spec exists, acceptance criteria. | Fill the spec; add open questions to `guidance/questions.yaml`. |
| **Design** | The approach, interfaces, trade-offs. | Record non-trivial choices as `DEC-*` in `decisions/`. |
| **Build** | Implement against the spec. | Create `DEC-*` for build decisions; fill `affected_scope`. |
| **Verify** | Acceptance met? tests pass? no decision drift? | `just decisions-audit --changed` (see below). |
| **Ship** | Reflection + cost totals, then archive. | `just archive-spec SPEC-001`. |

Honest confidence matters: decisions carry an `insight.confidence`
(0.0–1.0) that drives questions at design and flags at verify. See
`AGENTS.md` → Confidence Discipline.

## The patch lane (lightweight fixes)

Not every change deserves the full five-phase cycle. A **patch** is a bounded
fix to *already-shipped* behavior (a bug or UX papercut) that adds no new
feature/command. It runs a collapsed **`patch → verify → ship`** cycle — design
and build fused into one test-first pass — while **keeping the independent
verify** (the discipline that catches real defects) and a `DEC-*` when there's a
real decision. It sheds the separate frame/design cycles and all stage
bookkeeping.

```bash
just new-patch "out-dir should auto-create" PROJ-001   # → patches/PATCH-001-...md
just advance-cycle PATCH-001 verify                    # independent session
just advance-cycle PATCH-001 ship
just archive-patch PATCH-001                            # → patches/done/
```

Patches are first-class: `just validate`, `just cost-audit` (metered on
`patch`+`verify`), and `just status` all recognize them. **Guardrail:** if the
change adds a command/flag or needs its own design exploration, it's a spec, not
a patch. Rationale + full contract: `docs/decisions/DEC-003-patch-lane.md`.

## The spike lane (bounded exploration)

Before you know the shape, there's nothing to spec. A **spike** is a bounded
exploration that runs a collapsed **`spike → land`** cycle, in one of two
modes: `question` (a timeboxed investigation) or `build` (a **vibe-coding
session** — you're building, and you intend to keep it).

```bash
just new-spike "can bubbletea do split panes" "2h"              # investigation
just new-spike "is a local standup tool worth having" "1d" build # vibe-coding
just advance-cycle SPIKE-001 land
just archive-spike SPIKE-001
```

During the spike there is **no spec, no failing tests, and no `DEC-*`
required** — `test-before-implementation` doesn't apply, and the speed is the
value. There is deliberately **no verify step**: a spike has no acceptance
criteria, so a verify would have nothing to check.

What replaces it is the **timebox** and the **mandatory land step**. Landing
means answering the question, emitting `DEC-*` for the load-bearing choices the
exploration already made, and setting `spike.outcome` to one of `answered` /
`inconclusive` / `graduated` / `discarded`. `just archive-spike` refuses an
un-landed spike and `just validate` fails one — that's the lane's only tooth,
aimed at its only real failure mode.

Spikes live at the **repo root** (`spikes/`), not under a project: a spike
often precedes any project. Rationale + the graduation contract:
`docs/decisions/DEC-012-spike-lane.md`.

## Decisions and guardrails

Architectural decisions live in `decisions/` as `DEC-*` records. Audit
them anytime:

```bash
just decisions-audit             # lint structure + warn on scope conflicts
just decisions-audit --changed   # which decisions govern your pending edits
```

Fill a decision's optional `affected_scope:` (path globs it governs) so
`--changed` can flag it when those paths change — this is the "decision
drift" check in the Verify phase. Repo-wide rules live in
`guidance/constraints.yaml` (each rule has a severity and paths).

## Staying oriented

Four read-only views, each answering a different question:

```bash
just status            # current state: active project/stage, specs by cycle, stale items
just backlog           # spec-grained "what's next" (active stage; --all to widen)
just roadmap           # stage-grained "where is this project going": framed stages
                       #   + planned-but-unframed stages from the brief's Stage Plan
just specs-by-stage    # flat ledger: every spec by stage, ship date + complexity
                       #   defaults to ALL projects; --active or PROJ-NNN to scope
just calibration       # expected vs actual: predicted size/tokens against what
                       #   shipping took, + the token band each size landed in
```

`calibration` is the estimation feedback loop: `task.complexity` (expected,
set at design) against `task.complexity_actual` (stamped at ship), and the
optional `cost.tokens_estimate` against real `cost.totals.tokens_total`. It is
warn-only and always will be — the point is learning whether you systematically
under- or over-estimate, not hitting a number.

And periodic reflection / reporting:

```bash
just review            # prints the Weekly Review prompt with recent activity loaded
just report daily      # writes reports/daily/YYYY-MM-DD.md
just report weekly     # writes reports/weekly/YYYY-WNN.md
just report status     # writes reports/daily/YYYY-MM-DD-status.md (uncurated snapshot)
```
`report-daily` / `report-weekly` remain as permanent aliases for `report daily` /
`report weekly`.

## When you outgrow the defaults

Everything above is zero-dependency. For diagrams, use Mermaid fenced
blocks in markdown (the default — `docs/architecture.md` and
`docs/data-model.md` ship with examples). When a project genuinely needs
more — C4 modeling, protocol-level integration tests — see the optional,
project-level escalations in `guidance/recommended-tools.md`.

## Pointers

- The whole arc, idea to shipped wave: [PLAYBOOK.md](PLAYBOOK.md)
- First-time walkthrough: `GETTING_STARTED.md`
- Copy-paste prompts per phase: `FIRST_SESSION_PROMPTS.md`
- Conventions every agent follows: `AGENTS.md`
- All commands: `just --list`
