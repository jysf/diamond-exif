# AGENTS.md — Claude + Implementer Variant

Instructions for any AI agent working in this repository. Read this file first, every session.

> This file contains conventions only. For rules/constraints, see `/guidance/constraints.yaml`. For architectural rationale, see `/decisions/`. For waves of work against this app, see `/projects/`.

---

## 1. Repo Overview

- **Repo (the app):** [REPLACE: My App]
- **Purpose:** [REPLACE: one sentence]
- **Primary stakeholders:** [REPLACE]
- **Active project:** [REPLACE: PROJ-001 — MVP]

See `.repo-context.yaml` for structured metadata.

---

## 2. Work Hierarchy

```
REPO (the app — persists across all projects)
 └─ PROJECT (a wave of work: "MVP", "improvements", "v2 redesign")
     └─ STAGE (a coherent chunk within a project)
         └─ SPEC (an individual task)
              └─ HANDOFF (architect → implementer delegation record)
```

Key distinctions:

- The **repo** is the app. It persists. `AGENTS.md`, `/docs/`, `/guidance/`,
  `/decisions/` live at repo level because they accumulate across all
  projects.
- A **project** (`/projects/PROJ-*/`) is a bounded wave of work. Project
  artifacts (brief, stages, specs, handoffs) live inside the project
  folder.
- A **stage** is an epic-sized chunk within a project. A project typically
  has 2–5 stages.
- A **spec** is a single implementable task. It belongs to exactly one
  stage within one project.
- A **handoff** is an architect-to-implementer delegation document.

**Decisions persist at repo level**, even though they're often made
during a specific project. A decision like "we use pino for logging"
was made during PROJ-001 but binds PROJ-002 and PROJ-003 too. This is
intentional.

**Specs do not cross project boundaries.** If a task isn't finished
when a project ships, either finish it first or defer it explicitly into
the next project's brief.

**IDs are globally unique and continuous across the repo.** `STAGE-*` and
`SPEC-*` numbers keep counting up across projects — they do **not** restart at
001 per project. If PROJ-001 ends at `STAGE-006` / `SPEC-037`, PROJ-002 begins
at `STAGE-007` / `SPEC-038`. `just new-stage` / `just new-spec` assign the next
number repo-wide, so an ID unambiguously identifies one artifact anywhere.

---

## 3. Business Value

Value structure exists at project and stage levels; specs link lightly.

**Project `value:` block** states the thesis — a testable claim about
what this wave of work delivers. Beneficiaries, success signals, and
risks to the thesis make it falsifiable, not marketing copy.

**Stage `value_contribution:` block** states what this coherent chunk
of work advances, what capabilities it delivers, and what it
explicitly doesn't try to do. Helps avoid stages that seem valuable
but don't contribute to the project thesis.

**Spec `value_link:`** is a one-sentence reference back to the
stage's value. Infrastructure specs may have
`value_link: "infrastructure enabling X"`. Optional but encouraged —
it surfaces specs that don't trace back to the thesis.

Reports (`just report-daily`, `just report-weekly`) aggregate these
signals: which stages advanced the thesis, which specs most directly
delivered it, and where value traceability broke down.

---

## 4. Cost Tracking Discipline

Every cycle on a spec appends a session entry to the spec's
`cost.sessions` list, with a **real** `tokens_total` for metered cycles —
so reports aggregate actual AI spend, not zeros. Documentation alone is
skippable, and cost tracking silently goes empty (all-null numerics) the
moment a prompt says "leave it null"; the rule below + `just cost-audit`
make it stick. Full reference: `docs/cost-tracking.md`.

- **Schema:** a single combined `tokens_total` per session (most harnesses
  report one number — `/cost` in Claude Code, the `usage` object from an
  API call, `subagent_tokens` in an `Agent` result). Do NOT split
  input/output; there is no reliable split.
- **build / verify cycles** are the metered ones and must NOT be left
  null. The agent that runs the cycle records the real `tokens_total` /
  `duration_minutes` / `estimated_usd` from its own interface — the
  implementer for **build** (Claude Code `/cost`, the API `usage` object,
  or whatever its tool reports), the reviewer for **verify**. Carry the
  build number across in the handoff if the implementer can't write the
  spec directly; whoever ships confirms the numbers are present.
- **design / ship cycles** are main-loop work with no clean per-cycle
  metering — leave numerics `null` with a "main-loop, not separately
  metered" note.
- **`estimated_usd`** = `tokens_total` × your model's published list rate,
  no cache discount — an order-of-magnitude estimate; say so in the note.
- **Interfaces:** set `interface:` to `claude-code` | `claude-ai`
  (estimate by length) | `api` (the `usage` object) | `ollama` | `other`.
  Only genuinely un-metered cycles may be null-with-note.

The cycle-prompt wording lives in
`projects/_templates/prompts/cost-snippet.md` — use it so prompts don't
re-introduce the "null numerics" loophole. **Ship computes `cost.totals`**
(sum of non-null sessions; `tokens_total` uses `0`, never `null`) and runs
`just cost-audit`, which **fails if any shipped spec lacks build/verify
cost** (constraint `cost-captured-per-cycle`; CI job `cost-data`; surfaced
in `just status` and `report-weekly`). Pre-process specs can be
grandfathered via `COST_AUDIT_GRANDFATHERED` in `scripts/_lib.sh` (empty
by default).

Reports aggregate cost by cycle, by interface, by spec, and by stage.

---

## 5. Tech Stack

Replace this section with your actual stack. Be specific about versions.

- **Language:** [REPLACE]
- **Runtime:** [REPLACE]
- **Framework:** [REPLACE]
- **Database:** [REPLACE]
- **Testing:** [REPLACE]
- **Linter / Formatter:** [REPLACE]
- **Hosting:** [REPLACE]
- **CI:** [REPLACE]

---

## 6. Commands (exact)

These are the APP's commands. Wire them into **`app.just`** so they run as
`just build`, `just dev`, `just test`, etc. `app.just` is project-owned and
imported by the template-managed root `justfile` — keep app recipes there (not
in `justfile`) so a template update never clobbers your commands. For
template/workflow commands (`status`, `new-spec`, …) see `justfile`.

```bash
[REPLACE: install command]
[REPLACE: dev command]
[REPLACE: test command]
[REPLACE: test single file command]
[REPLACE: lint command]
[REPLACE: typecheck command]
[REPLACE: build command]
```

---

## 7. Directory Structure

```
/
├── AGENTS.md                          # This file
├── CLAUDE.md                          # Pointer to AGENTS.md
├── README.md                          # Human-facing readme
├── GETTING_STARTED.md                 # First-project walkthrough
├── FIRST_SESSION_PROMPTS.md           # Phase prompts
├── .repo-context.yaml                 # Repo (app) metadata
├── .variant                           # "claude-plus-agents"
├── justfile                           # Template-managed: just status, new-spec, etc. (imports app.just)
├── app.just                           # Project-owned: just build/dev/test/deploy (yours to fill in)
├── scripts/                           # Shell scripts powering justfile
├── docs/                              # Architecture, data model, API contract
├── guidance/                          # Repo-level rules (across all projects)
│   ├── constraints.yaml
│   ├── questions.yaml
│   ├── toolchain-brief.md             # Per-repo toolchain facts for cold build agents (DEC-004)
│   ├── agents/                        # OPTIONAL reviewer briefs (correctness, design, QA-at-close)
│   └── signals.yaml                   # Typed feedback ledger (see docs/signals.md)
├── decisions/                         # Repo-level DEC-* (across all projects)
├── feedback/                          # Raw inbound feedback captures (triaged into signals.yaml)
├── reports/                           # Daily + weekly report outputs
├── projects/                          # Waves of work
│   ├── _templates/                    # Shared templates
│   │   ├── spec.md
│   │   ├── release-spec.md            # Release cut + runtime pre-flight (DEC-006)
│   │   ├── patch.md                   # The patch lane (DEC-003)
│   │   ├── stage.md
│   │   ├── handoff.md
│   │   └── project-brief.md
│   ├── PROJ-001-<slug>/
│   │   ├── brief.md
│   │   ├── stages/
│   │   ├── specs/
│   │   │   └── done/
│   │   └── handoffs/
│   └── PROJ-002-<slug>/
│       └── (same structure)
└── src/                               # [REPLACE]
```

---

## 8. Cycle Model

Every spec moves through five cycles. **Cycles are tags, not gates** — edit any artifact anytime. The word "cycle" names what a spec goes through on its way to shipping.

| Cycle | Purpose | Who |
|---|---|---|
| **frame** | Go/no-go on the spec | Human + Claude (1 min) |
| **design** | Spec + failing tests + handoff | Claude (architect) |
| **build** | Make failing tests pass | Implementer agent |
| **verify** | Review + validation | Claude (reviewer) |
| **ship** | Merge, deploy, reflect, archive | Human + light agent |

Valid transitions:
```
frame → design → build → verify → ship
                   ↑       │
                   └───────┘ (verify sends back on punch list)
```

**`frame` is optional for a single spec — most start at `design`.** By the time a
task reaches `just new-spec` it has usually already passed go/no-go at the
stage/backlog level, so `frame` is redundant (across the dogfood it went unused —
0 of 100+ specs). Use it for one spec only when that spec's very existence is
genuinely in question; otherwise begin at `design`.

**`frame` earns its keep in BATCH, though — that's what `just frame-stage`
is for.** It promotes every `- [ ] (not yet written)` line in a stage's
`## Spec Backlog` into a real spec at `cycle: frame`, so each one has a **stable
ID** a sibling can point `depends_on:` at. That turns a planned stage into a
dependency-aware batch you can fan out (`just ready`) instead of a prose list.

The fidelity line matters: an **outline captures SCOPE and DEPENDENCIES, not
APPROACH**. Design stays just-in-time — a stage framed as ten pre-designed specs
is ten guesses that go stale before you reach spec four. Fill an outline's
`## Context` / `## Goal` / `depends_on:` and leave the rest scaffolded until it
advances to `design`. At stage close, record **how many outlines survived
unchanged** (Stage-Level Reflection) — that's how you learn whether framing this
far ahead pays for your work, rather than assuming it does.

Projects and stages have lighter lifecycles (not full cycles):

- **Project status:** `proposed | active | on_hold | shipped | cancelled` — the
  **coarse, machine-keyed** lifecycle state tooling branches on. Keep it coarse.
- **Project `activity`** (optional): a **human-facing** refinement of the work
  happening *within* an `active` project — `requirements | design | build | test |
  blocked` (a suggested **open** set; extend it, e.g. `spike`). It says *what kind
  of work is going on now* without abusing `status` or making the project look
  stalled. Example: a project gathering requirements before any spec is framed sets
  `status: active` + `activity: requirements`. `validate` warns on an unrecognized
  value but never fails; downstream readers may treat some activities as quiet
  phases (e.g. suppress "cut a release" nudges during `requirements`).
- **Stage status:** `proposed | active | shipped | cancelled | on_hold`

A stage is `active` when its first spec enters design. `shipped` when
its spec backlog is complete AND the stage-level reflection is written.

### The patch lane (lightweight fixes — DEC-003)

A **patch** is a bounded fix to *already-shipped* behavior (a bug or UX papercut)
that adds **no new feature/command** and doesn't warrant a full spec + stage. It
runs a collapsed **`patch → verify → ship`** cycle instead of a spec's five:

- **patch** — design + build fused into one test-first pass (write the failing
  test *and* the fix together).
- **verify** — **kept, and kept independent** (a separate agent from the patch
  author). This is the one discipline the dogfood retrospective proved catches
  real defects; it is non-negotiable.
- **ship** — CHANGELOG `[Unreleased] → Fixed` + `just archive-patch`. **No stage
  bookkeeping** — a patch attaches to the project, not a stage.

**Stays:** the full gate suite, a `DEC-*` when there's a real decision, and
index-verify-before-ship. **Sheds:** the separate frame + design cycles and the
stage backlog/`Count:` bookkeeping. **Guardrail:** if a change adds a
command/flag or needs its own design exploration, it's a **spec, not a patch**.

Mechanics: `just new-patch "title" [PROJ-NNN]` scaffolds
`projects/PROJ-*/patches/PATCH-NNN-<slug>.md` (its own repo-wide `PATCH-*`
sequence). Patches are first-class in `just validate`, `just cost-audit`
(metered on `patch`+`verify`), and `just status`. `just archive-patch PATCH-NNN`
files it under `patches/done/`.

### Delegated cycles: the handoff / handback contract

**One handoff per delegated CYCLE.** With build and verify on different agents
you get two per spec — `handoff.cycle` distinguishes them, and `to_agent` comes
from `.repo-context.yaml` → `spec.agent.tier_map.<cycle>` (DEC-005).

```bash
just new-handoff SPEC-042 build     # → HANDOFF-NNN, to_agent = tier_map.build
just new-handoff SPEC-042 verify    # → HANDOFF-MMM, to_agent = tier_map.verify
```

**The handback is the return path, and it is mandatory.** The executing agent
fills the `handback:` front-matter block before reporting done — **including a
real `tokens_total`**. This is not a courtesy: build and verify are the *metered*
cycles the cost gate requires, and the orchestrator has no meter for an agent it
doesn't host. The agent that ran the cycle is the only party that knows the
number.

```bash
just handback-sync SPEC-042         # transcribe reported cost → cost.sessions
```

`handback-sync` is idempotent (it stamps `synced_at`) and **exits 1 if any
handoff hasn't handed back cleanly**, naming which one and why. The orchestrator
**never estimates a delegated cycle's cost** — an invented number is worse than
no number, because it looks real in every downstream rollup.

If a platform genuinely exposes no token count, say so once in
`.repo-context.yaml` → `cost.metering_source: none` (which disables the gate,
DEC-005) rather than guessing per-spec. Full contract: DEC-013.

### The spike lane (bounded exploration — DEC-012)

A **spike** is the phase *before* you know the shape: a bounded exploration whose
job is to produce information, not shipped behavior. It runs a collapsed
**`spike → land`** cycle. Two modes, one discipline:

| `spike.mode` | Is | Code is | Lands as |
|---|---|---|---|
| **`question`** | A timeboxed investigation | Evidence | `answered` / `inconclusive` |
| **`build`** | A **vibe-coding session** | The deliverable | `graduated` / `discarded` |

- **spike** — explore. **No spec, no failing tests, no `DEC-*` required.** This is
  the one place in the repo with no conventions to follow; the speed *is* the
  value. `test-before-implementation` does **not** apply during a spike.
- **land** — **mandatory.** Answer the question, emit `DEC-*` for the choices the
  exploration already made, and decide the code's fate.

**There is deliberately no `verify` step.** A patch keeps its independent verify
because it fixes *known* behavior against a *known* expectation. A spike has
neither acceptance criteria nor a spec, so a verify here would have nothing to
check and would degrade into theater — which would erode the real verify in the
other two lanes. The **timebox** and the **mandatory land step** replace it.

**Required from creation:** `spike.question` (one sentence — a spike with no
question is just coding; loose is fine for `mode: build`, absent is not) and
`spike.timebox`. **Hitting the timebox without an answer is `inconclusive`, which
is a real result** — not a reason to extend. Extending twice means it isn't a
spike, it's an unframed project.

**Guardrails:** a spike may not ship user-facing behavior (that's a spec, or a
patch for shipped behavior); its code may not be built upon before it lands; and
it is not a way to skip the cycle on work you already understand — if you can
write acceptance criteria, you have a spec.

**Graduating a `build` spike** (the vibe-coding → real-work conversion) writes
five things: `.repo-context.yaml`, `AGENTS.md`, `guidance/toolchain-brief.md`
(most valuable here — a spike generates exactly this friction), retroactive
`DEC-*` for **load-bearing choices only** with honest confidence, and a project
brief framed around **what comes next** (the spike is prior art in
`Dependencies → Depends on`). **Do NOT retro-write specs for code that already
works** — a spec directs work that hasn't happened yet.

Mechanics: `just new-spike "the question" [TIMEBOX] [MODE] [PROJ-NNN]` scaffolds
`spikes/SPIKE-NNN-<slug>.md` at the **repo root** — not under a project, because
a spike may precede any project (its own repo-wide `SPIKE-*` sequence;
`project.id` is optional and back-linked at land). `just validate` requires a
question, a timebox, and — on a spike at `cycle: land` — a real `spike.outcome`;
`just archive-spike SPIKE-NNN` refuses an un-landed spike and files it under
`spikes/done/`. `just cost-audit` does **not** gate spikes (cost is advisory).
A project exploring before it frames anything can set `project.activity: spike`.

---

## 9. Instruction Timeline

Every spec has a timeline file at
`projects/*/specs/SPEC-NNN-<slug>-timeline.md` listing cycle
instructions in order with status markers.

Status markers:

- `[ ]` not started — no one has picked this up yet
- `[~]` in progress — an executor is currently running this
- `[x]` complete — cycle finished; see the prompt file for what was run
- `[?]` blocked — needs a human decision or external unblock before
  proceeding. Include a one-line reason after the marker.

Cycle prompts live at `projects/*/specs/prompts/SPEC-NNN-<cycle>.md`.
The architect writes them; executors (the implementer agent for
build, Claude again for verify) read and run them.

**Discipline for executors:**

- When you start a cycle, mark it `[~]`.
- When you finish, mark it `[x]` with a one-line result (PR number,
  cost, completion date).
- If you hit a real blocker — constraint ambiguous, dependency
  missing, verify surfaced something needing architect judgment —
  mark `[?]` with a one-line reason. Do NOT use `[?]` as a "I don't
  know what to do" dumping ground. Blocked means the next move
  requires someone else; everything else is in-progress or a
  question to resolve in the current session.

This is a convention, not a mechanism. No tooling enforces it; the
discipline lives in the prompt set. Skip it and nothing breaks, but
you lose the history artifact and the next executor has to hunt for
the right prompt.

---

## 10. Cross-Reference Rules

Every spec has these relationships, encoded in front-matter:

- `project.id` → the project it belongs to (e.g., `PROJ-001`)
- `project.stage` → the stage within that project (e.g., `STAGE-002`)
- `references.decisions` → DEC-* it was designed against
- `references.constraints` → constraints that apply
- `handoff.from_agent` / `handoff.to_agent` → roles in the delegation

When a spec references a DEC, the DEC does not reciprocally list the
spec. DECs are stable repo-level records; specs come and go.

---

## 11. Coding Conventions

- **Naming:** [REPLACE]
- **File organization:** [REPLACE]
- **Imports:** [REPLACE]
- **Error handling:** [REPLACE]
- **Logging:** [REPLACE]
- **Comments:** Explain *why*, not *what*.
- **No dead code.** Delete, don't comment out.
- **Ship the reader with the field.** A field, column, flag, event or
  namespace lands **in the same change as the thing that reads it** — or it
  doesn't land. No "we'll wire it up later."

  This is the most reliably recurring defect this template has produced
  (N=5 across the dogfood: a stage cost slot nothing summed, a defect-stage
  question nothing counted, a decision `status` nothing rendered, a
  provenance namespace with zero readers, and a `value_link` that was filled
  in everywhere but never actually linked to anything).

  The failure is quiet, which is why it keeps happening: **an unread field
  looks identical to a field whose data hasn't arrived yet.** Nobody fills in
  a field that nothing displays, so it stays empty; and because it is empty,
  nobody notices it is also unread. By the time you want the data, the window
  to have collected it has closed — and unlike most defects, you cannot fix
  this one retroactively. There is no back-filling a year of measurements.

  The cheap version of the reader counts as a reader: a line in an existing
  view, one number in a rollup, a `--json` key someone can grep. It does not
  have to be a dashboard. It has to *surface*.

  **Delegating makes this worse, not better:** a handoff that adds a field is
  the easiest place to lose the reader, because the implementer ships the
  schema and the orchestrator assumes the surfacing came with it. Name the
  reader in the handoff's acceptance criteria.
- **Diagrams:** author them as Mermaid fenced blocks in markdown
  (`/docs/`, `/decisions/`, specs) so they render on GitHub and you can
  keep them current as part of the work. Update the relevant diagram in
  the same change, not afterward. See `/guidance/recommended-tools.md`.

---

## 12. Testing Conventions

- Every new function gets at least one test.
- Test file naming: [REPLACE]
- Coverage expectations: [REPLACE]
- Must test: happy path, error cases, edge cases from acceptance criteria.
- Need not test: third-party internals, framework behavior.
- **TDD:** Tests live in the spec's `## Failing Tests` section, written
  during **design**, made to pass during **build**.
- **Exception — the spike lane.** `test-before-implementation` does NOT apply
  during a `cycle: spike` exploration (DEC-012): a spike has no acceptance
  criteria to test against, and the speed is the point. It applies again the
  moment anything graduates into a spec.
- **Behavioral pre-flight (design-time).** When a spec's literal/artifact makes a
  claim about *runtime behavior* — a component registers, a hook fires, a binary
  resolves on PATH, a server answers, a config is actually loaded — exercise that
  behavior through the surface that **runs** it before declaring design done, not
  merely the surface that **validates its shape**. A manifest that passes
  `validate --strict` can still register nothing; a completion script that lints
  can still emit the wrong marker; a config that parses can still not take effect.
  Shape-check and behavior-check are *different checks* — neither substitutes for
  the other. The defect class that escapes design→build→verify is disproportionately
  operational/runtime, not spec-logic; this is where to catch it.
- **Design-time probe / measure-before-build (design-time).** When a spec's
  implementation depends on the *actual* behavior of a load-bearing external — a
  library's real API signature, a tool resolving on the **pinned** toolchain, the
  true version floor, a config field the engine actually reads — or when it
  **tunes toward a measurable target**, probe or measure the real thing **against
  the real pinned tree during design**, and record the verified facts (the exact
  calls, the baseline number) in the spec's `## Implementation Context` / the
  handoff (or the governing `DEC-*`). Two recurring moves: (1) **probe the real
  API/tool** — don't trust the model's prior; the pinned version's signature may
  differ (a wrong assumed call is then caught at design, not mid-build); (2)
  **measure the baseline now** so the target and the change are grounded in
  numbers, not guesses. When you do, build collapses to a near bit-for-bit
  *transcription* instead of a discovery loop — the strongest efficiency lesson
  from the dogfood (recurring across projects, highest single-lesson frequency).
  Complementary verify move — **adversarial mutation:** revert the change and
  confirm the guard *fails*; it both proves the test has teeth and surfaces
  dead/no-op config (a field the engine never reads).

---

## 13. Git and PR Conventions

- **Branch:** `feat/spec-NNN-<slug>`, `fix/spec-NNN-<slug>`, `chore/<slug>`
- **One spec per branch, one PR per branch.**
- **Commits:** [REPLACE: e.g., Conventional Commits]
- **PR description must include:**
  - Project: `PROJ-NNN`
  - Stage: `STAGE-NNN`
  - Spec: `SPEC-NNN`
  - Handoff: `HANDOFF-NNN`
  - Decisions referenced: `DEC-NNN, DEC-MMM`
  - Constraints checked: `[list]`
  - New `DEC-*` files created during build

**One git worktree per concurrent session.** This variant routinely has
two agents in flight (architect and implementer). If more than one session
touches this repo at once, each MUST run in its own `git worktree`, not the
shared checkout — two agents writing one working tree corrupt each other
(a parallel build can clobber an uncommitted edit, or a commit can land on
the wrong branch). `git worktree add <path> <branch>`, work there, commit +
push, then `git worktree remove`. Always check `git branch --show-current`
before any commit.

### Delegated execution (sub-agents) — DEC-004

This variant delegates build/verify to a separate implementer/reviewer agent via
`HANDOFF-*`. Five rules keep that delegation honest:

1. **Reconcile over self-report — never flip `handoff.status` to `completed` (or
   advance `task.cycle`) on the sub-agent's word alone.** After it reports, verify
   the *claimed* result against actual **git + disk** state:
   - `git log <base>..HEAD` and `git ls-remote origin <branch>` — are the commits
     actually there (locally *and* pushed)?
   - the spec's `## Failing Tests` files exist on disk, and the gate actually ran?

   Trust git/disk over **any** agent self-report or timeline marker — both lie (a
   truncated report can claim "done" with the commit or push missing; agents have
   reported "pushed" while `origin` was still at the prior SHA). **If the sub-agent
   dies mid-cycle:** reconcile the partial output, finish the *mechanical remainder*
   in the coordinator loop (don't re-run the whole cycle), and attribute cost to
   the sub-agent's metered portion (`subagent_tokens`), recording the coordinator
   finish as a separate null-with-note cost session.

   **The general form: verify any claim you are about to act on, against the
   thing it describes.** Build output is only the most obvious case. A number in
   a roadmap, a count in a harvest, a "we already tried that" in a decision
   record — each is a self-report by a past session, and each rots silently.
   Prefer the source: run the count, read the corpus, check the remote. This is
   *cheap* and it is *mechanical* — it needs no judgement, only the habit of
   looking before acting. In this variant the surface is larger, because a
   delegated agent's handback is a claim too.
2. **One sub-agent at a time; no interleaved tree ops.** Launch exactly one
   build/verify sub-agent, then do **no** git/tree operations in the shared
   checkout — no `new-spec`, `checkout`, or commits, and don't design the next
   spec — until it reports complete and its branch is merged. The structural fix
   is per-agent `git worktree` isolation (the worktree habit above).
3. **Set the sub-agent's model explicitly** from `.repo-context.yaml`
   `spec.agent.tier_map` (design/build/verify) — don't rely on a default (a silent
   Opus default is a ~6× cost surprise). `new-spec`/`new-patch` stamp `agents.*` /
   `handoff.from_agent` from it (DEC-005).
4. **Sanction a trivial dev-dep + its DEC in one build pass.** The implementer
   can't stop-and-ask mid-run, so the `no-new-top-level-deps-without-decision`
   constraint carves out an exception: a build cycle MAY add a clearly-trivial
   **DEV-only** dependency (types packages, test utilities — **never** a runtime
   dep) and author its DEC in the same pass. This keeps the constraint's teeth
   for real (runtime) choices while sparing the `@types/node`-stub workaround.
5. **Inject the toolchain brief into the handoff / implementer prompt.** A cold
   implementer re-imports generic tool-priors and wastes loops rediscovering this
   repo's specifics. Give it `/guidance/toolchain-brief.md` (test framework +
   assertion lib, lint/format quirks, runtime globals, installed dev utilities,
   gotchas) so it doesn't. The template ships the slot; the instance fills the
   truth — keep the brief current or the implementer will trust a stale fact.

---

## 14. Domain Glossary

- **[REPLACE: Term]** — [REPLACE: Definition]

---

## 15. Cycle-Specific Agent Rules

### During **design**

Set the **expected size** in `task.complexity` on the t-shirt scale
`XS | S | M | L | XL | XXL`. This is a *prediction*, and the point of a
prediction is that it later gets checked: ship stamps `task.complexity_actual`,
and `just calibration` shows whether you systematically under- or
over-estimate. `XL`/`XXL` is itself a finding — a spec that size is almost
certainly a stage; split it.

Optionally record `cost.tokens_estimate` (predicted total tokens) too. Once
enough specs have shipped, `just calibration` prints the token band each
expected size *actually* landed in — at which point the size you assign doubles
as a token estimate, measured from this repo rather than guessed. None of this
gates anything; the feedback loop is the whole value.

### During **build** (implementer reads this)

Before writing code:
1. Read the `/projects/PROJ-*/handoffs/HANDOFF-*.md` for your spec.
2. Read the linked `SPEC-*.md`, `STAGE-*.md`, and the project's `brief.md`.
3. Read every `DEC-*` listed in the handoff's references.
4. Read `/guidance/constraints.yaml`; check rules for paths you'll touch.
5. Read `/guidance/toolchain-brief.md` — the per-repo toolchain facts (test
   framework, lint quirks, runtime globals, installed dev utilities, gotchas) a
   cold implementer otherwise re-derives loop by loop. **The coordinator injects
   this brief into the handoff / implementer prompt** (DEC-004 rule 5).
6. If anything is ambiguous, add to `/guidance/questions.yaml` and stop.

When done:
1. Fill in the handoff's `## Completion` section (including reflection).
2. Update `handoff.status` → `completed`; update spec's `task.cycle` → `verify`.
3. Append a build cost session entry to the spec's `cost.sessions`.
4. Create `DEC-*` files for non-trivial implementer decisions. When a
   decision is tied to specific code, fill in its `affected_scope`
   with the path globs it governs (e.g. `src/lib/log.ts`,
   `src/api/**`). This is required for file-bound decisions — it's
   what lets `just decisions-audit --changed` surface the decision
   when those paths change later. Leave `affected_scope: []` only for
   decisions not tied to particular files (e.g. a process choice).
5. Open PR following Section 13.

Shortcut: `just advance-cycle SPEC-NNN verify`.

### During **verify** (reviewer reads this)

Check:
1. Acceptance criteria all met and tested?
2. Failing tests from spec now pass?
3. No drift from referenced decisions?
4. No constraint violations?
5. Non-trivial implementer choices have accompanying `DEC-*`?
6. Implementer reflection answered (not mailed in)?
7. `cost.sessions` has entries for prior cycles? Flag if missing
   (don't block).
8. For any acceptance criterion claiming **runtime behavior** (a component
   registers, a hook fires, a binary resolves on PATH, a server answers, a
   config takes effect), was the *behavioral* surface actually exercised — not
   just the shape validated (§12 behavioral pre-flight)? This is the class that
   escapes.

For check 3, run `just decisions-audit --changed` — it flags which
`DEC-*` records govern the files the implementer touched, so you can
confirm the work stayed consistent with them. `just decisions-audit`
(no flag) lints the records themselves. See `/guidance/recommended-tools.md`
for optional, heavier verify tooling (e.g. LineSpec for protocol-level
integration tests).

Append a verify cost session entry before returning the verdict.

Output: ✅ APPROVED (with SHA) / ⚠ PUNCH LIST / ❌ REJECTED.

### During **ship**

Append a `## Reflection` block to the spec with three answers:
1. What would I do differently next time?
2. Does any template, constraint, or decision need updating?
3. Is there a follow-up spec to write now?

Then:
- Update the spec's `task.cycle` → `ship`.
- Append a ship cost session entry, then compute `cost.totals`.
- Stamp `task.complexity_actual` — what it actually took, on the same
  `XS|S|M|L|XL|XXL` scale as the expected `task.complexity`. Ship is the only
  moment that number is knowable.
- Run `just archive-spec SPEC-NNN` (moves to `done/`, updates stage).
- If Q2 surfaces a template/constraint/decision change you're NOT making now,
  record it in `/guidance/signals.yaml` (`type: lesson` with its N-count for a
  recurring coding pattern; `type: process-debt` for tooling friction) so a
  close forces the decision. See `docs/signals.md`; browse `just dash signals`.
- If stage backlog is complete, run the Stage Ship prompt.
- Log the win — **on by default** (DEC-010). Call the configured tool directly
  (default `brag`): `brag add -t "<what shipped>" -k shipped -i "<IMPACT>"` (CLI),
  or the `brag_add` tool over `brag mcp serve` (MCP). Seed it from the spec's
  `value_link` + `cost.totals`, and frame the **impact** (the outcome / who's
  better off), not the output. See `guidance/recommended-tools.md`; opt out via
  `spec.accomplishments.enabled: false`.
- Commit.

**Cutting a release?** A release is its own spec — scaffold it with
`just new-release-spec "<version>" STAGE-NNN` (or `just new-spec … --release`).
It carries a generic runtime **pre-flight checklist** (tag integrity, artifact
trust on a clean host, channel trust, data isolation, runtime smoke, rollback —
DEC-006); fill in the tool-specific command for each before you publish. Every
defect that escaped design→build→verify across the dogfood projects was
operational/runtime, so don't skip it. For the version to cut, run
`just next-version` — it follows this app's `spec.version.scheme` (default
`calver`; DEC-007). That app version lives in git tags; the top-level `VERSION`
file is template provenance, not the app's version (see `docs/versioning.md`).

---

## 16. Confidence Discipline

Decisions in `/decisions/` have an `insight.confidence` field (0.0–1.0).
Honest values matter — they drive these behaviors:

- **Design phase:** if Claude emits a decision at confidence < 0.7, it
  also adds an entry to `/guidance/questions.yaml` flagging it for
  further investigation.
- **Verify phase:** if a spec references any decision at confidence < 0.6,
  that's a yellow flag worth surfacing in the review.
- **Weekly review:** all decisions at confidence < 0.8 are listed with
  a note on whether recent work has strengthened or weakened them.

Use 1.0 only for decisions that are truly locked (tech stack choice
after it's been installed and working, for example). Most decisions
should land between 0.7 and 0.95.

---

## 17. Pointers

- Constraints: `/guidance/constraints.yaml`
- Open questions: `/guidance/questions.yaml`
- Toolchain brief (per-repo facts for cold build agents): `/guidance/toolchain-brief.md` (DEC-004 rule 5)
- Reviewer briefs (OPTIONAL review lenses — correctness, design fit, QA at close):
  `/guidance/agents/` — see its README. Only meaningful in a session that did NOT do the work.
- Signals (typed feedback ledger): `/guidance/signals.yaml` (browse `just dash signals`; ritual + bar in `docs/signals.md`)
- Decisions: `/decisions/` (audit with `just decisions-audit`)
- Recommended (optional) tools: `/guidance/recommended-tools.md`
- Versioning (app scheme + `just next-version`): `/docs/versioning.md` (DEC-007)
- Projects: `/projects/`
- Templates: `/projects/_templates/`
- What we're building (architecture): `/docs/architecture.md`
- Feedback: `/feedback/`
- Reports: `/reports/` (daily, weekly)
- Timelines: `/projects/*/specs/SPEC-NNN-*-timeline.md` (per-spec)
- Cycle prompts: `/projects/*/specs/prompts/`
- Phase prompts: `/FIRST_SESSION_PROMPTS.md`
- First walkthrough: `/GETTING_STARTED.md`
- Daily commands: run `just --list`
