# The playbook — idea to shipped wave

The full arc this template supports, including the parts that happen
**before** you have a repo. It's the map; the other docs are the terrain.

| Doc | Job |
|---|---|
| **This doc** | The whole arc, and *when* to do each part |
| [`README.md`](../README.md) | What the template is, setup, variants, command table |
| [`docs/USAGE.md`](USAGE.md) | Reference for the daily loop — commands, cycle, patch lane, views |
| `GETTING_STARTED.md` | Step-by-step first project *(created by `just init`)* |
| `FIRST_SESSION_PROMPTS.md` | The copy-paste prompt for each phase *(created by `just init`)* |
| `AGENTS.md` | The conventions every agent reads *(created by `just init`)* |

---

## The arc

```
Phase 0   Shape the idea            no repo yet — a conversation
          └─ or SPIKE it            don't know the shape? explore, then land
Phase 1   Decompose it              PROJ / STAGE / SPEC / DEC take shape
Phase 2   Scaffold                  just init
Phase 3   First wave                PROJ-001 → first spec shipped
Phase 4   Steady state              the cycle, repeating
Phase 5   Close the wave            stage ship → project ship → next wave
   ↻      Harvest                   what the work taught you, fed back
```

Three lanes run inside this, routed by **stakes**, not size:

| Lane | Cycle | For |
|---|---|---|
| **Spec** | frame → design → build → verify → ship | Committed work |
| **Patch** | patch → verify → ship | A bounded fix to shipped behavior |
| **Spike** | spike → land | Exploration, before you know the shape |

Phases 0 and 1 have no tooling and no files in this repo. They are the
part everyone skips and the part that decides whether the rest works.

---

## How you actually operate this

The phases below list commands, which makes it read like you sit in a terminal
typing `just`. In practice you don't. **Claude is the user interface; `just` is
the API.** You work in a conversation, and the agent drives the commands.

There are four layers, and it's worth knowing which one you're standing on:

```
  YOU              judgment · go/kill · stakes · "is this reflection honest?"
   │  conversation
   ▼
  CLAUDE           reads state · drafts artifacts · implements · reviews cold
   │  just … --json
   ▼
  COMMANDS         status · ready · validate · new-spec · advance-cycle · …
   │  read/write front-matter
   ▼
  ARTIFACTS        brief.md · STAGE-*.md · SPEC-*.md · DEC-*.md · signals.yaml
                   ↑ the memory. Outlives every session above it.
```

Read it bottom-up and the design explains itself: **the artifacts are the only
durable layer.** Everything above them is transient — the session ends, the
command exits, and you forget. So each layer's job is to get truth *down* into
the artifacts and pull context back *up* out of them, cheaply enough that a cold
session can start from nothing.

That's also why the commands are a thin, boring, machine-readable surface rather
than a nice terminal UI: their consumer is the layer above them, and that layer
is a model.

This is the design, not a shortcut around it:

- **`AGENTS.md` is addressed to the agent, not to you.** `CLAUDE.md` exists
  only to point at it. The primary reader of this repo's conventions is a
  model.
- **Every read command takes `--json` with a documented exit-code contract**
  ([DEC-001](decisions/DEC-001-interface-contract.md)), which states outright
  that the front-matter *is* the public API — what "scripts, reports, and any
  future package/MCP/UI consume."
- **`just review` is a command whose output is a prompt.** Its job is to load
  context into a session, not to inform a human.
- **`FIRST_SESSION_PROMPTS.md` is the command palette.** The prompts are how you
  invoke a phase; the `just` recipes are what the phase runs.

### Division of labour

| | Owns |
|---|---|
| **You** | Judgment. Go/kill. The value thesis. Which lane a change belongs in. Whether a verify verdict is accepted. Whether a reflection is honest. Breaking a constraint. |
| **The agent** | Reading state, drafting artifacts, running commands, implementing, reviewing cold, transcribing cost and impact. |
| **The commands** | The cheap deterministic surface between you — so neither side re-derives state from prose. |

### What must not be delegated

The template's own [productization axiom](ROADMAP.md) is the guardrail here:

> **The discipline is the value, not the artifact.** Tooling that exposes the
> *contract* amplifies adoption; tooling that generates the *content or
> judgment* dissolves the discipline it's meant to enforce. Coach, don't wrap.

That applies to the agent exactly as it applies to tooling. The failure mode of
a conversational UI is **agent-graded homework**: the agent proposes the spec,
implements it, reviews it, writes the reflection, and concludes it went well.
Every artifact is present, every gate is green, and the record is fiction.

Four calls stay yours, and they're the cheap ones:

1. **Go/kill** at frame — the agent is structurally biased toward "this is
   viable, here's a plan."
2. **The stakes call** — spec, patch, or spike. Getting this wrong is how you
   end up resenting the process.
3. **Accepting a ⚠ or ❌ verdict** — including deciding a punch list is
   pedantic and overriding it, which is legitimate and should be recorded.
4. **Whether a reflection is honest.** *"Nothing to change"* on every spec is
   the tell, and only you know if it's true.

Everything else is safely the agent's.

### Practical consequences

- **The session is the unit of the interface.** "New session per cycle" is a UI
  discipline, not a file-management one — it's the only way to get a reader
  that hasn't already convinced itself.
- **You can drive it entirely by conversation** — *"start the next spec,"*
  *"what's ready?"*, *"land that spike"* — as long as the four calls above stay
  with you.
- **It degrades gracefully to manual.** Everything is markdown and bash, so a
  session that goes sideways is fixed by editing a file. Nothing is locked
  behind the agent.
- **Cost attribution is the known casualty.** Only delegated sub-agent work is
  metered reliably; the conversational main loop where you spend most of your
  orchestration is undercounted. Recorded cost is a floor.

## Pick your on-ramp

Three ways in. They differ only in where you enter the arc and what you
owe in writing when you get there.

| On-ramp | You have | Enter at | Notes |
|---|---|---|---|
| **Greenfield** | An idea | Phase 0 | The default path. Everything below applies in order. |
| **Prototype-first** | An itch to just build it | `just new-spike … build`, then land it | The spike lane (DEC-012). See [Prototype-first](#the-prototype-first-on-ramp) below. |
| **Existing repo** | A mature codebase | Phase 2, then a conversion | Same conversion ritual, more archaeology. Frame PROJ-001 around the *next* wave, never around what's already there. |

---

## Phase 0 — shape the idea

**Where:** a chat session. No repo, no files, no template.
**Output:** one paragraph you'd be willing to defend, or a kill.
**Time:** minutes to a few sessions. Don't rush the kill decision.

Work the idea with an agent until it collapses to four lines:

- **What** — what this wave of work delivers
- **For** — who actually uses it
- **Why now** — what makes this the right thing to build next
- **Success** — one concrete outcome that would mean it worked

Ask for skepticism explicitly, and ask for a go/kill with reasoning.
An idea that can't survive a hostile reading in chat will not survive
four stages of specs. Killing here costs one conversation.

Two things worth pinning down before you move on, because they get
expensive later:

- **The value thesis** — a testable claim, not marketing. *"Users will
  love it"* is not a thesis. *"Reducing month-2 churn by making
  activation faster"* is. This becomes `value.thesis` in the brief.
- **The honest success signal** — if there's no business metric (an
  experiment, a personal tool, a toy), say so and use a proxy: does the
  headline capability work end to end? A stated *"no metric yet —
  success is X works"* beats an invented number.

> **Known gap:** the template has no home for pre-commitment vision or a
> candidate-idea backlog — ideas either become a `PROJ-NNN` brief or
> evaporate. A workspace-level `ideas.md` outside any repo is the current
> workaround, and is tracked as an unearned convention in
> [`ROADMAP.md`](ROADMAP.md).

---

## Phase 1 — decompose

Still a conversation. You're turning the frame into the four artifact
types the template runs on:

| Artifact | Answers | Grain |
|---|---|---|
| **`brief.md`** (PROJ) | What is this wave, and how will we know it worked? | One wave of work |
| **`STAGE-*.md`** | What coherent chunk comes next, and what does it deliver? | 2–5 per project |
| **`SPEC-*.md`** | What is one implementable task, and what proves it's done? | 3–8 per stage |
| **`DEC-*.md`** | Why did we choose this, and how sure are we? | Whenever a real choice gets made |

Sizing rules that keep the decomposition honest:

- A project with **one** stage is a stage. A project with **eight** is
  two projects.
- Target specs at **S** or **M**. An **L** should be split. An **XL** or
  **XXL** is a stage wearing a spec's clothes.
- A stage with fewer than 3 specs is probably part of another stage;
  more than 8 and it wants splitting.

**Do not design every spec up front.** The decomposition you want at
this point is *scope and dependencies only*. Once a stage lists its work
as backlog lines, promote the whole batch to outline specs in one step:

```bash
just frame-stage STAGE-001 --dry-run    # show ID assignment, write nothing
just frame-stage STAGE-001              # one outline spec per backlog line
```

Each gets a **stable ID** at `cycle: frame` — enough to declare
`depends_on:` between them and let `just ready` compute what's
dispatchable. The *approach* for each is designed just in time, when
that spec advances to `design`. Designing all of them now is work you
will throw away.

**Doing Phase 1 before the repo exists** works — draft the brief and the
first stage as plain text and move them in after `just init`. But the
prompts in `FIRST_SESSION_PROMPTS.md` read `AGENTS.md`, the templates,
and `.repo-context.yaml` to do their job, so after your first project
it's usually better to scaffold first (Phase 2) and run Prompts 1a→1c
inside the repo.

---

## Phase 2 — scaffold

```bash
# Option A — GitHub "Use this template", then clone, then:
just init          # answer N to fresh history (the template gave you a clean one)

# Option B — clone directly
git clone <this-template> my-new-repo && cd my-new-repo
just init          # answer y to fresh history
```

`just init` asks which variant you want:

- **`claude-only`** — Claude plays architect, implementer, and reviewer.
  Start here. Every instance in the registry but one runs this variant.
- **`claude-plus-agents`** — Claude architects and reviews; a separate
  tool implements. Adds `handoffs/`. Migration later is about an hour.

Then, before any specs:

- [ ] `just info` and `just status` — confirm variant + active project
- [ ] Fill `.repo-context.yaml` — what this app *is*
- [ ] Read `AGENTS.md` — it's the source of truth every session reads
- [ ] **Fill `guidance/toolchain-brief.md`** — test framework, lint
      quirks, runtime globals, gotchas. This is the single biggest lever
      on a cold build session, which otherwise re-derives the same
      mismatches every time. Doing it early is nearly free; doing it
      late is paying for it once per spec.
- [ ] `just --list` — see what's available
- [ ] Commit

---

## Phase 3 — the first wave

Follow `GETTING_STARTED.md` step by step; it maps 1:1 onto the prompts
in `FIRST_SESSION_PROMPTS.md`. The short version:

| Step | Prompt | Produces |
|---|---|---|
| Project frame | 1a | The paragraph from Phase 0, sharpened |
| Project brief | 1b | `projects/PROJ-001-<slug>/brief.md` |
| First stage | 1c | `stages/STAGE-001-<slug>.md` + spec backlog |
| Repo design | 2a | `docs/architecture.md`, first `DEC-*`, constraints |
| First spec | 2b | `SPEC-001`, its build prompt, its timeline |
| Build | 3 | **Fresh session.** Implementation + PR |
| Verify | 4 | **Fresh session.** ✅ / ⚠ / ❌ |
| Ship | 5 | Reflection, cost totals, `just archive-spec` |

**Aim the first spec at something shippable.** Across the harvested
projects, time-to-first-ship was consistently same-day or next-day — the
projects that front-loaded a thin end-to-end increment, then widened.
Time-to-*full*-value scaled with scope (roughly 2–4 weeks), and the one
project that stated a target missed it by 2×. Expect that.

Delete `projects/PROJ-001-example-mvp/` and the example `DEC-001` once
you've read them.

---

## Phase 4 — steady state

The loop, once per spec:

```
design → build → verify → ship
   │        │        │        │
   │        └── new session   └── new session
   └── writes the build prompt the next session reads
```

The mechanics are in [`docs/USAGE.md`](USAGE.md). What matters here is
which parts are load-bearing and which are convenience.

**The four non-negotiables** (claude-only):

1. **New session per cycle.** The spec file — not your memory — is the
   handoff. Build and verify in one session produces a review that finds
   nothing.
2. **The spec is the source of truth between sessions.** If the
   `## Implementation Context` section feels redundant to you, it's
   correct: the build session has none of your design context.
3. **Weekly review** (`just review`). Without a second agent pushing
   back, drift compounds silently.
4. **Honest reflections.** *"Nothing to change"* on every spec means
   either perfection or mailing it in, and it isn't perfection.

Across ~121 shipped specs, the two elements that demonstrably prevented
errors were the **independent verify** and the **decision log**. If you
economize, don't economize on those.

**Not everything deserves the full cycle.** A bounded fix to
already-shipped behavior is a **patch** — collapsed `patch → verify →
ship`, keeping the independent verify and a `DEC-*` where there's a real
decision:

```bash
just new-patch "out-dir should auto-create" PROJ-001
```

If the change adds a command or flag, or needs its own design
exploration, it's a spec. The underlying lesson from the harvests:
**value tracks stakes, not size.** The full cycle earned its keep on
high-stakes work — hardening, releases, a license landmine caught before
it reached the tree — and was pure tax on low-stakes mechanical changes.
Route by stakes.

**Staying oriented** — each view answers a different question:

```bash
just status         # where am I right now
just dash           # one read view, many lenses (now/next/future/ledger/…)
just ready          # what's dispatchable, dependencies satisfied
just backlog        # what's next at spec grain
just roadmap        # where is this project going, at stage grain
just calibration    # expected vs actual size and tokens
```

---

## Phase 5 — close the wave

Stage done → **Prompt 1d**. Project done → **Prompt 1e**. Both do the
same three things: check what shipped against what was claimed, write
the reflection, and **disposition the open signals** in
`guidance/signals.yaml` — accept, reject, or defer with a trigger, but
never silently carry. A `process-debt` signal that sits `open` across
two project closes is exactly the rot the registry exists to stop.

Then plan the next wave from Phase 0 again. `STAGE-*` and `SPEC-*`
numbers are repo-wide and continuous — if PROJ-001 ended at `STAGE-006`
/ `SPEC-037`, PROJ-002 starts at `STAGE-007` / `SPEC-038`.

> **Known thin spot:** the template is strong on *starting* a wave and
> comparatively quiet on *ending* one. See the "Closing / ending a
> project" thread in [`ROADMAP.md`](ROADMAP.md).

---

## The prototype-first on-ramp

Sometimes the honest first move is to just build the thing and see if
it's any good. That's legitimate, and the template should not pretend
otherwise — the full cycle on an unproven idea is ceremony over a
question you haven't answered yet.

**Spike freely** — and the template now has a lane for it:

```bash
just new-spike "is a local standup tool worth having" "1d" build
```

`mode: build` is the vibe-coding session; `mode: question` (the default) is a
timeboxed investigation. During the spike there are no specs, no failing
tests, and no decision records — `test-before-implementation` explicitly does
not apply, and there is no verify step, because a spike has no acceptance
criteria to verify against. You are buying information about whether this is
worth doing at all.

Two fields are required from the start, and they're what separate a spike
from undirected coding: **`spike.question`** (loose is fine in `build` mode,
absent is not) and **`spike.timebox`**. Hitting the timebox without an answer
is `inconclusive` — a real result, not a reason to extend. Extending twice
means it isn't a spike; it's an unframed project.

**Stop spiking when any of these is true:**

- You can't reconstruct *why* something is the way it is.
- A second person — or a fresh agent session — needs to touch it.
- The cost of a wrong decision starts to exceed the cost of writing it
  down first.
- You're about to build something substantial *on top of* the spike.

**The conversion ritual — now the `land` step.** Landing a spike is
mandatory: `just archive-spike` refuses a spike with no `spike.outcome`, and
`just validate` fails one. When you graduate, you owe five things — and
notably, not a sixth:

1. **`.repo-context.yaml`** — describe what now exists.
2. **`AGENTS.md`** — real tech stack, real commands, real conventions.
   The spike already decided these; write down what's true.
3. **`guidance/toolchain-brief.md`** — everything the spike taught you
   the hard way. This file is *most* valuable here, because a spike
   generates exactly the kind of undocumented friction it captures.
4. **Retroactive `DEC-*` records — only the load-bearing ones.** A
   choice that still constrains what you can build next gets a decision
   record with honest confidence. Everything else stays undocumented.
   This is not an archaeology exercise; if you can't remember the
   rationale, that's a finding, and `confidence: 0.4` with a note is a
   truthful record.
5. **A PROJ-001 brief framed around what comes *next*.** The spike is
   prior art in the brief's `Dependencies → Depends on`, not the subject
   of the project.

**What you do not do:** retro-write specs for code that already works.
A spec's job is to direct and constrain work that hasn't happened yet.
Writing one for shipped code produces a document nobody reads to
describe behavior the tests already assert.

> **Status: shipped at N=1 — a deliberate exception.** This on-ramp has one
> instance behind it (`standup`), below the template's own N=3 same-outcome /
> N=2 paired-opposing bar. It was built anyway, on the owner's explicit
> direction, because the vibe-coding→production path otherwise has no forcing
> function at all — which is the exact problem the template exists to solve.
> Recorded as an exception in
> [DEC-012](decisions/DEC-012-spike-lane.md), with a revisit trigger: **if
> spikes get created and actually *landed*, it earned its place; if they get
> created and abandoned un-landed, the lane is failing at its only job and
> should be cut back to prose.**

---

## Is it working?

The honest baseline: **you keep using it on the next project.** That's a
real signal, not a cop-out — a process with genuine friction and no
payoff gets abandoned, quietly and fast. Everything below is cheaper
than a metrics program and roughly as informative.

**Cheap checks already built in:**

```bash
just calibration    # are your size/token estimates systematically off?
just cost-audit     # is any shipped spec missing real cost data?
just specs-by-stage # the flat ledger — what actually shipped, and when
just lifetime-data  # whole-repo history, no LLM needed
```

`calibration` is warn-only and always will be. The point is learning
whether you consistently under- or over-estimate, not hitting a number.

**Failure signatures worth watching for** — each one observed in a real
instance:

| Signature | What it means |
|---|---|
| Build reflections all say *"nothing was unclear"* | Build and verify ran in the same session |
| Reflections all say *"nothing to change"* | The reflection is being mailed in |
| Signals sitting `open` across two project closes | The disposition ritual isn't happening |
| Weekly review keeps finding the same drift | `AGENTS.md` or `constraints.yaml` is wrong — fix it at the source, not per-spec |
| `value_link` never populated | The value thesis isn't actually driving spec selection |
| Every spec runs the full cycle, including 3-line fixes | You're paying full ceremony on low-stakes work — use the patch lane |
| Verify never rejects anything | Either the specs are unusually good, or verify isn't independent |
| You can't remember the last time you said "no" to a proposed spec | Agent-graded homework — the go/kill call has drifted to the agent |

**What not to do:** don't turn this into a measurement program. The
template's own discipline is *deliberate lag* — a practice becomes a
rule only after it recurs at **N=3 same-outcome or N=2 paired-opposing**.
The strongest finding from three projects and ~121 specs was an explicit
*"don't push it to codify sooner."* The same restraint applies to your
own conventions.

---

## Feeding it back

The process improves from real usage, not from thinking about it:

- **`guidance/signals.yaml`** — the running ledger in your project repo.
  Anything that should change the template, the rules, or the templates
  themselves goes here when you notice it, and gets dispositioned at
  stage or project close. See `docs/signals.md`.
- **`feedback/`** in this repo — friction captured from an instance,
  triaged.
- **[`docs/harvests/instances.md`](harvests/instances.md)** — the
  registry of repos we learn from, with a *last reviewed* date so
  harvesting is incremental rather than a full re-read.
- **[`docs/harvests/`](harvests/)** — periodic cross-project harvests.
  The [2026-07-06 three-project harvest](harvests/2026-07-06-three-project-dogfood-harvest.md)
  is the model: sources, ranked backlog, what to *protect* from
  improvement.

The reflection loop is the harvesting engine. In the largest instance,
the majority of ~90 retro findings had *already* been folded into
`AGENTS.md` by the ship-time reflection prompt before anyone ran a
retro. The bottleneck isn't noticing — it's seeing what's queued.

---

## Where this is thin (known)

Stated plainly so you don't discover it as a surprise. All tracked in
[`ROADMAP.md`](ROADMAP.md):

- **Pre-commitment vision is only half-housed.** A spike now gives
  *exploration* a home (DEC-012), but there's still nowhere for a
  candidate-idea backlog or repo-level vision — Phase 0's "should we
  even?" still lives in chat logs. Tracked in `ROADMAP.md` as an
  unearned convention.
- **The `frame` cycle is nearly dead** — zero specs sat in `frame`
  across 122. It may get absorbed into `design`.
- **Closing a project is thinner than starting one.**
- **Recorded cost is systematically undercounted** — only sub-agent
  work is metered reliably; orchestration and pre-spec framing have no
  home.
- **`claude-plus-agents` is under-evidenced** — one instance, never
  harvested. The claude-only path is the well-trodden one.
- **The prototype-first on-ramp above is N=1.**
