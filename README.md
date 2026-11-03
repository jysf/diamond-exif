# Spec-Driven Multi-Agent Repo Template

A GitHub template for running spec-driven development on an app (or apps) where the **repo is the app** and **projects are waves of work** against that app. Works whether you use Claude alone or Claude plus a dedicated implementer agent (Kilo Code, Factory Droids, AdaL, etc.).

## Hierarchy

```
Repo (the app — persists forever)
 └─ Project (a wave of work: "MVP", "v2 improvements", "redesign")
     └─ Stage (a coherent chunk within a project)
         └─ Spec (an individual task)
              └─ Cycle (Frame → Design → Build → Verify → Ship)
```

- **Repo** accumulates architecture, conventions, constraints, and decisions across all projects.
- **Projects** are bounded waves of work. You may have one active project, several sequential projects over time, or (rarely) multiple active projects.
- **Stages** are epic-sized chunks within a project. A project typically has 2–5 stages.
- **Specs** are individual implementable tasks. Each belongs to exactly one stage.
- **Cycles** are the 5-phase lifecycle a spec goes through.

## Using this template

**Option A — GitHub template (recommended):**

Click **"Use this template"** at the top of this repo on GitHub. Create your new repo. Clone it. Then inside your new repo:

```bash
just init
```

This asks whether you want the `claude-only` or `claude-plus-agents` variant, moves the right files to the repo root, and removes what you don't need. GitHub's template feature already gives you a clean history, so answer **N** when `init` offers to reset it.

**Option B — Clone directly:**

```bash
git clone https://github.com/YOUR-USERNAME/THIS-TEMPLATE.git my-new-repo
cd my-new-repo
just init      # answer 'y' when it offers a fresh git history
```

On a clone, `just init` asks whether to **start a fresh git history** — answer **y** to discard the template's commits and begin your project on a clean `main`. It records which template version you came from in the initial commit (provenance). You can also do this later with `just fresh-start`.

## After `just init`

You'll have a repo root containing:
- `AGENTS.md` — conventions for all agents working here
- `CLAUDE.md` — pointer to `AGENTS.md` for Claude Code
- `GETTING_STARTED.md` — walkthrough for your first project
- `FIRST_SESSION_PROMPTS.md` — copy-paste prompts for each phase
- `.repo-context.yaml` — describes the app (the repo)
- `justfile` — commands you'll run daily
- `docs/`, `guidance/`, `decisions/` — repo-level (accumulate across all projects)
  - `guidance/recommended-tools.md` — optional, project-level tool escalations (Mermaid is the default for diagrams; Structurizr, LineSpec, etc. when you outgrow it)
- `SECURITY.md` — the trust model for your repo (adapt the reporting section to your team)
- `projects/PROJ-001-example-mvp/` — example project you can learn from or delete

**Next step:** open `GETTING_STARTED.md` and follow it.

## The two variants at a glance

### `claude-plus-agents/` — Claude architects, a separate agent implements

For workflows where:
- Claude writes specs + reviews PRs (architect + reviewer)
- A different tool (Kilo Code, Factory Droids, AdaL, Cursor, etc.) implements

Adds `/projects/*/handoffs/` — explicit handoff documents that carry context between agents that don't share memory.

### `claude-only/` — Claude does everything

For workflows where Claude plays every role — architect, implementer, reviewer. No separate implementer tool.

No `/handoffs/` folder. The context the implementer needs is folded into each spec's `## Implementation Context` section.

**Not sure which?** Start with `claude-only`. Migration to `claude-plus-agents` later is about an hour of mechanical work.

## `just` commands available

Run `just --list` to see everything. The main ones:

| Command | What it does |
|---|---|
| `just init` | One-time: choose variant, scaffold the repo |
| `just dash` | The project dashboard — one read view, many lenses: `dash now`/`next`/`future`/`ledger` (= status/backlog/roadmap/specs-by-stage), plus `dash decisions`/`questions`/`signals`/`patches`/`spikes`/`defects`/`constraints`/`handoffs`; no arg stitches an overview with governance flags. Add `--json` to any of them |
| `just status` | Current state: active project, stage, specs by cycle, stale items |
| `just new-spec "title" STAGE-NNN` | Scaffold a new spec with next available ID |
| `just new-stage "title" PROJ-NNN` | Scaffold a new stage in the active (or named) project |
| `just new-patch "title"` | Scaffold a patch — the lightweight fix lane (collapsed patch→verify→ship, keeps independent verify + DEC) |
| `just new-spike "question" [TIMEBOX] [MODE]` | Scaffold a spike — the bounded-exploration lane (collapsed spike→land, no verify). `MODE=build` is a vibe-coding session. Lives at the repo root; may precede any project |
| `just new-handoff SPEC-NNN build\|verify` | Scaffold a delegation handoff (claude-plus-agents) — one per delegated cycle; `to_agent` from `tier_map.<cycle>` |
| `just handback-sync SPEC-NNN` | Transcribe the agent's self-reported cost from its handoffs into the spec — the orchestrator never estimates a delegated cycle |
| `just archive-spike SPIKE-NNN` | File a **landed** spike under `spikes/done/` — refuses one with no `spike.outcome` |
| `just new-release-spec "vX" STAGE-NNN` | Scaffold a release spec with a generic runtime pre-flight checklist (or `new-spec … --release`) |
| `just advance-cycle SPEC-NNN verify` | Update a spec's `task.cycle` field |
| `just archive-spec SPEC-NNN` | Move a shipped spec to `done/` + update stage backlog + stamp `shipped_at` |
| `just close-project PROJ-NNN` | Close a project: refuses on in-flight specs (when claiming `shipped`), an empty reflection, or undisposed signals; stamps `shipped_at`; prints the cost/value/decision rollup. `--dry-run`, `--json` |
| `just specs-by-stage` | Flat ledger of every spec by stage (all projects); `--active` or `PROJ-NNN` to scope |
| `just decisions-audit` | Lint `DEC-*` files + warn on scope conflicts; `--changed` flags decisions governing pending edits |
| `just decisions-index` | Regenerate `decisions/INDEX.md` (id · title · confidence · status · project · supersedes); `--check` fails if stale, `--json` for the typed surface |
| `just cost-audit` | Fail if any shipped spec is missing real build/verify cost (`tokens_total`); same check the CI `cost-data` job runs |
| `just validate` | Fail if any spec's front-matter is missing required structural fields or has invalid enums (the schema gate; see `docs/schema-reference.md`) |
| `just template-version` | Print the spec-driven template version (the version your repo was scaffolded from); `--json` for machine-readable |
| `just next-version` | Suggest this app's next release version per its scheme (default CalVer `vYYYY.MM.PATCH`); `--json` |
| `just build-info` | Build provenance stamp — a `git describe` ref + commit + dirty flag, to bake into the artifact so a build traces back to source; `--json` |
| `just review` | Load recent activity and print the Weekly Review prompt |
| `just report daily \| weekly [DATE] \| status` | Generate a report: curated daily, weekly-aggregate, or an uncurated `status` snapshot (`report-daily` / `report-weekly` remain as permanent aliases) |

## Documentation

- [docs/PLAYBOOK.md](docs/PLAYBOOK.md) — the full arc, **including the part before you have a repo**: shaping the idea, decomposing it, the three on-ramps (greenfield / prototype-first / existing repo), and how to tell whether the process is working for you.
- [docs/USAGE.md](docs/USAGE.md) — the daily loop in depth: project → stage → spec → cycle, the read-only views, decisions and guardrails.
- [PROJECTS.md](PROJECTS.md) — real projects built with this template.
- [docs/blog/](docs/blog/) — posts on the what, why, and what got built (drafts).
- [SECURITY.md](SECURITY.md) — trust model, secret hygiene, reporting.
- [CONTRIBUTING.md](CONTRIBUTING.md) — design principles and the dev loop, if you want to extend the template.
- `GETTING_STARTED.md` + `FIRST_SESSION_PROMPTS.md` — created by `just init` for your first project.

## What ContextCore concepts this template uses

This template is philosophically aligned with [ContextCore](https://github.com/neil-the-nowledgeable/contextcore) — the same vocabulary (`task.*`, `insight.*`, `guidance.*`, `handoff.*`, `project.*`), the same artifact model, the same forward-compatibility to OTel-based observability. But it requires no infrastructure — everything is markdown files until (and only if) you graduate to the full ContextCore stack.

See `docs/CONTEXTCORE_ALIGNMENT.md` for details (created by `just init`).

## License

Do whatever you want with this template.
