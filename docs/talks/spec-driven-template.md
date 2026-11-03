# Talk plan — Spec-driven development with agents

**Thesis:** *The agent has no memory. The repo is the memory. The artifacts you
produce for the agent turn out to be the ones you needed anyway.*

Built as **modules** so you can run it at 30 or 40 minutes without rewriting.
Every number is sourced — see [Fact sheet](#fact-sheet).

---

## Two running orders

| Module | 30-min | 40-min |
|---|---|---|
| **A** — The problem: side projects die at 60% | 3 | 3 |
| **B** — The one idea: the repo is the memory | 3 | 3 |
| **C** — The cycle, and the rule everyone breaks | 4 | 4 |
| **D** — **bragfile: the tool and the loop** | 5 | 5 |
| **E** — bragfile: what the numbers showed | 3 | 4 |
| **F** — **The benefits, layered** *(proj/stages → functionality)* | — | 7 |
| **G** — **The other projects** | — | 5 |
| **H** — What earned its keep, what was tax | 4 | 4 |
| **I** — How the process improves itself (N=3) | 2 | 2 |
| **J** — What to try on Monday | 1 | 1 |
| Q&A | 5 | 2 |
| **Total** | **30** | **40** |

**Cut order if running long:** I → last third of F → G down to two projects.
Never cut D or E — they're the only evidence in the talk.

---

## A — The problem (3 min)

Open with the shared experience, not the solution.

> You start a side project with an agent. Day one is euphoric — more done in an
> evening than in the last month. Day four you come back and the agent has no
> idea what you were doing. Neither do you. Day ten the codebase has three
> competing patterns for the same thing, because nine sessions each made a
> locally reasonable choice.

The actual failure, which is not "the agent isn't smart enough":

- **Every session starts cold.** The agent's context dies with the tab.
- **You are the only continuity** — and you're a side project's part-time
  attention.
- So the repo accumulates *decisions nobody recorded*, and the cost of the next
  change rises until you stop opening it.

Land it: **this is a memory architecture problem, not a model capability
problem.** Better models make each session better. They do nothing about the gap
*between* sessions.

---

## B — The one idea (3 min)

> If the agent's memory dies every session, the repo has to be the memory. Not
> the code — code tells you *what*, never *why*.

```
Repo      the app — persists forever, accumulates the "why"
 └─ Project   a wave of work: "MVP", "v2", "redesign"
     └─ Stage     a coherent chunk (2–5 per project)
         └─ Spec      one implementable task (3–8 per stage)
              └─ Cycle     Frame → Design → Build → Verify → Ship
```

The load-bearing insight: **projects end, the repo doesn't.** Architecture,
constraints and decisions live at repo level so they survive the wave that
produced them. Most tooling models the sprint. Almost none models what the sprint
leaves behind.

All markdown. Zero dependencies — `just` and a folder of files.

---

## C — The cycle, and the rule everyone breaks (4 min)

| Phase | The question it answers |
|---|---|
| **Frame** | Why does this exist? What would prove it's done? |
| **Design** | What's the approach, and what did we decide? |
| **Build** | Make the failing tests pass. |
| **Verify** | Were the acceptance criteria actually met? |
| **Ship** | What did we learn, and what did it cost? |

Then the rule that gets the raised eyebrows:

> **Start a brand-new session for build. And another for verify.**

Pre-empt the objection — *"that's wasteful, I'm throwing away context"*:

- You're not throwing it away. You're **forcing it into the spec**, where the
  next session, and future you, can read it.
- A design session that reviews its own build finds nothing. It already believes
  the code is right — it wrote the plan.
- There's a tell for cheating: the build reflection says *"nothing was unclear."*
  That's the signature of design and build sharing a session.

Be honest with the room: **this discipline is the whole thing.** The commands are
conveniences. Fresh sessions and an independent review are what work.

### "Wait — do I type all this?" (30 sec, but don't skip it)

Someone is already wondering. Answer it before they ask, because the answer is
better than they expect:

> No. **Claude is the UI. The commands are the API.** I work in a conversation —
> *"what's ready?"*, *"start the next spec"*, *"land that spike"* — and the agent
> runs the commands. That's the design, not a shortcut: the conventions file is
> addressed to the agent, not to me, and there's a command whose entire output is
> a prompt.

Then the turn, which is the part worth remembering:

> Which creates one new way to fail, and it's a good one to know about.
> **Agent-graded homework**: the agent proposes the work, implements it, reviews
> it, writes the reflection, and concludes it went well. Every artifact present,
> every gate green, and the whole record is fiction.
>
> So four calls stay mine, and they're the cheap ones: **go/kill** — the agent is
> structurally biased toward *"this is viable, here's a plan"*; **which lane** a
> change belongs in; whether I **accept a rejection**; and whether a reflection is
> **honest**. Everything else it can have.

If you want one line for the slide: *the agent does the work; you keep the
judgment.* And a test the audience can apply to themselves — **when did you last
tell it no?**

---

## D — bragfile: the tool, and the loop it closes (5 min)

*This is the segment that makes the talk yours rather than a methodology
lecture. It's also the most memorable idea in it.*

### What it is (1 min)

A command-line tool for keeping a **brag file** — a running log of what you
actually accomplished, so at review time you're not reconstructing nine months
from memory. Go. Ships to a Homebrew tap. Local-first: a CLI *and* an MCP server.

Name the recursion out loud, then pause:

> A tool for remembering what you did, built with a process for remembering why
> you did it.

### How it evolved (1.5 min)

Three moves, each earned rather than planned:

1. **PROJ-001 — the MVP.** 5 stages, 23 specs, 14 decisions. A CLI that logs an
   accomplishment and gets out of the way.
2. **Then release engineering became the hard part.** Homebrew tap trust, macOS
   Gatekeeper, goreleaser cutting two tags on one commit — none of it logic, all
   of it operational. (Sets up module E.)
3. **Then it grew an MCP server** — and that changed what it was *for*. Once an
   agent can call `brag_add`, the thing logging accomplishments doesn't have to
   be a human at the end of the week.

### The loop closing (2.5 min) — the payoff

Here's the move. The template's **ship** step now calls bragfile *by default*:

```bash
brag add -t "<what shipped>" -k shipped -i "<the impact>"
```

Not "we shipped SPEC-032." The **impact** — who's better off. And the agent
doesn't invent it, which is the part worth dwelling on: the spec already carries
`value_link` (what this contributes) and `cost.totals` (what it took), and the
ship reflection asks, in a fixed slot:

> **"What can a user do now that they couldn't before?"** — one sentence,
> before → after. Write `none` if there's no user-visible outcome — that's a
> real, greppable result, not a blank.

That answer *is* the impact line. The agent transcribes it; it doesn't
reconstruct it from memory. Both halves were already written — the spec's Context
is the before, its Goal is the after.

So the loop is:

```
spec says what value it will deliver
  → ship asks whether it did, in one sentence
    → the agent logs that sentence as impact, with what it cost
      → the brag file is a running, honest record of outcomes
```

Land the two-sentence version, because this is the transferable idea:

> **The agent captures progress and impact as a side effect of finishing the
> work** — at the one moment the answer is actually knowable, by the one party
> who was there. Nobody writes a status report. The status report falls out.

**Honest caveat, and say it:** it was over-built first. v0.6.10 shipped a
`just log-win` wrapper that pre-filled the command; v0.6.11 deleted it. The agent
can just call the tool. *"The template only coaches; the agent runs the tool."*
That's a good instinct to name for the room — when you're integrating agents with
tools, the wrapper is usually the thing to cut.

---

## E — What the numbers showed (3–4 min)

### The numbers (1 min)

One slide:

```
PROJ-001 (MVP)     5 stages · 23 specs · 14 decisions · ~4 weeks
By the 3-project retro:
                   4 projects · 42 specs · 27 decisions · 3 releases
                   40 of 42 specs shipped
                   every stage completed in order
                   zero design→ship drift
                   25 decisions, exactly ONE superseded
                   mean decision confidence 0.823 — and no 1.0s
```

Two deserve a beat: **one supersession in 25** (and it fired on its own
pre-declared trigger — the decision that was wrong announced itself), and **no
confidence of 1.0** anywhere, which is how you know the records aren't theater.

### The miss (1 min) — do not skip

> The brief said roughly two weeks. It took about four. A 2× overrun.

It's in the artifact, in the project's own reflection, where the next estimate
has to look at it.

**That's the deliverable.** Not "the process makes you fast" — it doesn't,
especially at first. It makes your misses **legible** instead of quietly
forgotten. Every side project you've abandoned also overran; you never wrote it
down, so you never got better at predicting.

### The finding that changed the template (1.5 min)

> Across three projects and 121 specs I went looking for the defects that made it
> through design, build, *and* verify and escaped into the real world. I expected
> a mix. It wasn't a mix.

**Every escaped defect was operational or runtime. Not one was logic.**

Read the list — it lands better read than summarized:

- A timezone / day-boundary bug that made the streak counter read zero
- goreleaser cutting two tags on the same commit
- macOS Gatekeeper blocking the signed binary
- Homebrew tap-trust
- **A development binary that migrated the production database**
- A plugin whose manifest validated `--strict` and registered exactly zero
  servers

The diagnosis: design → build → verify is **dense on whether the logic is right
and nearly silent on whether the thing runs.**

The rule it became:

> When a spec claims something about *runtime behavior* — a component registers,
> a hook fires, a binary resolves on PATH, a server answers — the pre-flight must
> run it through the surface that **exercises that behavior**, not the surface
> that **validates its shape**.

The clean paired example if you have the time: one spec ran cobra's real
`GenBashCompletion` and caught a marker mismatch *at design*. Another ran
`plugin validate --strict` — shape only — and shipped a manifest that validated
and registered nothing.

---

## F — The benefits, layered (7 min) — *40-min version only*

*The user-facing "so what." Structure it as a descent: wave → chunk → shipped
capability. The point is that the hierarchy isn't bureaucracy — each layer
answers a question you actually have.*

### The layer cake (2 min)

Walk **one real arc** top to bottom rather than describing the levels abstractly:

| Layer | The question it answers | bragfile's answer |
|---|---|---|
| **Project** | What wave are we on, and why now? | *"MVP: a CLI that makes logging a win take five seconds"* |
| **Stage** | What coherent chunk is next? | *Foundational storage → the CLI surface → release engineering* |
| **Spec** | What's the one task, and what proves it? | *"`brag add` writes an entry with impact"* + its failing tests |
| **Decision** | Why is it this way? | 27 `DEC-*`, honest confidence, one superseded |

Then the punchline about the shape:

> Each layer is a **different question you'd have asked anyway**. The project
> layer is the one you answer at a dinner party. The stage layer is the one you
> answer on Monday morning. The spec layer is the one you answer to an agent.
> The decision layer is the one you answer to yourself in four months.

### What it actually bought (3 min)

Three concrete benefits, each with the evidence attached — no abstractions:

1. **You can stop and restart without cost.** Come back after two weeks, run
   `just status`, and the repo tells you where you were. This is the one people
   feel immediately, and it's why side projects finish.
2. **The record is queryable, not archaeological.** `just specs-by-stage` for
   what shipped and when. `just calibration` for whether your estimates are any
   good. `just dash signals` for what's queued to change. None of this needed a
   database — it's markdown front-matter and shell.
3. **Decisions stop re-litigating themselves.** 27 decision records, one
   supersession — and the supersession fired on a trigger the original decision
   had written down in advance.

### The functionality that fell out (2 min)

Worth naming that the *tooling* came from the work, not from a design phase:

- `just specs-by-stage` — **back-ported out of bragfile** because bragfile needed
  it, then shipped upstream.
- The **patch lane** — invented independently by another project (a 3-line fix
  that ran a full four-cycle), then adopted.
- The **release spec with a runtime pre-flight checklist** — built independently
  by *two* projects before the template had one.
- Build provenance — bragfile wired version+commit into the binary, then built a
  guard that reads it to refuse to let a dev binary touch the production
  database. Became a template decision after the fact.

The general point, which is the most portable thing in this module:

> **Every one of those was invented by a project first and adopted by the
> template second.** None came from thinking about the process. The process's job
> was to *notice* — and to not adopt them too early.

---

## G — The other projects (5 min) — *40-min version only*

*Establishes this isn't an N=1 story, and — more useful — shows the process
behaving differently across genuinely different domains.*

Table on screen, talk to three of the rows:

| Project | What it is | Scale | What it proved |
|---|---|---|---|
| **crustyimg** | Image optimization, Rust | ~43 specs · 9 stages · 73 DECs · public v0.1.0 | Shipped **same day**; invented the patch lane |
| **zany-animal-slots** | A slot-machine game, browser | 37 specs · 6 stages, PROJ-001 shipped | Turned *subjective* quality into CI guards |
| **skillport** | Skill linting + audit | 2 projects, most active | Paid for itself twice before anyone harvested it |
| **standup** | Local standup tool | consumer/product tier | **Started as a vibe-coding session** |
| **rspeed** | Network-speed CLI, Rust | 8 ADRs, multi-OS CI | Green on 3 OSes in ~1m16s |
| **bragfile-report** | Aggregator | — | Produced the cross-project retro itself |

Three worth speaking to:

**crustyimg — the counterexample on speed.** bragfile took ~21 days to first
ship; crustyimg's SPEC-001 was created *and shipped the same day*. Same process,
5× difference. Because the first spec was aimed at a thin end-to-end increment
rather than a foundation. That's the single most actionable lesson in the talk
for anyone starting: **aim spec one at something shippable.**

**zany-animal-slots — the one that surprised me.** A slot machine: juice, motion,
feel. Its own brief flagged the risk that *"juice resists TDD"* — you can't unit
test whether something feels good. It then **refuted its own risk** by inventing
CI guards for subjective quality: contrast ratios, compositor-only keyframes,
touch targets, motion budgets. Not "does it feel good" — the mechanical
preconditions of feeling good, gated in CI.

> The lesson generalizes past games: **when quality feels untestable, you usually
> haven't found the mechanical precondition yet.**

**standup — the honest one.** It didn't use the process at first. It got built.
That was correct — and it's why the template just grew a spike lane (module I).

**And the honest bottom of the table:** two instances have *never been
harvested*. One of them, `uw`, is the only project on the two-agent variant —
which means the multi-agent path is the least-evidenced thing in this whole talk.
Say that plainly. A talk that claims every data point worked is not a talk about
real projects.

---

## H — What earned its keep, what was tax (4 min)

The most useful thing you can hand the room. Be blunt.

**Two things prevented essentially all the errors:**

1. **The independent verify** — a reviewer who wasn't the author.
2. **The decision log** — the written *why*, with an honest confidence number.

Everything else is scaffolding around those two.

**The sharp edge:**

> **Value tracks stakes, not size.**

The full cycle paid on high-stakes work: hardening, releases, an AGPL-licensed
dependency caught before it reached the tree. On low-stakes mechanical changes it
was pure tax. A three-line fix running four phases is ceremony, you will resent
it, and resenting it is how people abandon a process.

So the process grew lanes — **routed by stakes, not size**:

| Lane | Cycle | For |
|---|---|---|
| **Spec** | frame → design → build → verify → ship | Committed work |
| **Patch** | patch → verify → ship | A bounded fix to shipped behavior |
| **Spike** | spike → land | Exploration — before you know the shape |

The patch lane keeps the two things that work and drops the rest. The transferable
form:

> **If your process has one gear, it's wrong for most of your work.**

---

## I — How the process improves itself (2 min)

Every ship asks: *does anything here need to change the rules?* Answers go into
one ledger. Every stage and project close forces a decision on every open item —
accept, reject, or defer **with a trigger**. Nothing silently carries.

The bar for turning an observation into a rule:

> **N=3 same-outcome, or N=2 paired-opposing.**

Three independent times the same failure hits the same way — or two times where
the rule was confirmed once by its presence and once by its absence.

The counterintuitive punchline, for an audience primed on velocity:

> The clearest recommendation out of three projects and 121 specs was:
> **"don't push it to codify sooner."**

**Optional 45-second addition if you ran module G** — the spike lane as a live
example of the *judgment* around the bar:

> The newest lane broke that rule on purpose. Vibe-coding is how standup started,
> and that's exactly one data point — below the bar. It got built anyway, because
> the alternative was leaving the prototype-to-production path with no forcing
> function at all. The decision record says so in as many words, and sets a
> revisit trigger: if spikes actually get *landed*, it earned its place; if they
> get abandoned half-finished, cut it back to prose.
>
> That's the part worth stealing — not the rule, the fact that **breaking it is
> written down with a trigger.**

---

## J — What to try on Monday (1 min)

Don't sell the template. Sell the two cheap habits:

1. **Write the decision down when you make it** — one paragraph, the options you
   rejected, an honest confidence number. Not later. Later never comes.
2. **Review in a session that didn't write the code.** Same model, same machine,
   new context. Nearly free, highest yield in the whole process.

> If those two feel good after a month, the rest of the structure is just what
> they grow into.

Close on the honest measurement standard:

> I don't have a rigorous ROI number. My test is: *am I still using it on the
> next project?* Several projects in, I am. That's a real signal — a process with
> friction and no payoff gets abandoned fast and quietly.

---

## Slides

| # | Slide | For |
|---|---|---|
| 1 | Title — *The agent has no memory. The repo is the memory.* | A |
| 2 | The 60% graveyard — day 1 / day 4 / day 10 | A |
| 3 | The hierarchy | B |
| 4 | The five phases | C |
| 5 | **The rule** — *New session for build. New session for verify.* Big type. | C |
| 5b | *The agent does the work; you keep the judgment.* — the four calls | C |
| 6 | bragfile — what it is (a terminal shot of `brag add`) | D |
| 7 | **The loop** — spec → ship question → impact logged → brag file | D |
| 8 | bragfile numbers, including the 2× miss | E |
| 9 | Escaped defects — the six-item list. Header: *Not one was logic.* | E |
| 10 | The layer cake — project / stage / spec / decision | F |
| 11 | Invented-by-a-project-first — the four features | F |
| 12 | The other projects table | G |
| 13 | Earned vs tax + the three lanes | H |
| 14 | N=3, and *"don't push it to codify sooner"* | I |
| 15 | Monday — the two habits | J |

Slides 10–12 are the 40-minute-only ones. Optional 7b: a real spec file on
screen for ~15 seconds, purely to show it's plain markdown with front-matter.
Don't read it aloud.

---

## Live demo — probably don't

A terminal demo eats 5 minutes and risks 10. If you have time back from Q&A, the
only one worth doing is `just status` on a real project — 45 seconds, one screen,
no typing beyond the command. In the 40-minute version, `brag list` right after
module D is a stronger choice: it shows the *accumulated* impact log, which is
the thing the whole loop exists to produce. Screenshot fallback ready; use it
without apologising if anything is slow.

---

## Q&A prep

**"Isn't this just waterfall with extra steps?"**
Waterfall designs everything up front. Here only *scope and dependencies* are
decided up front; each spec's approach is designed just in time. Specs get stable
IDs early so dependencies can be declared — but designing them all now is work
you'd throw away.

**"How much slower is it?"**
Slower per spec on low-stakes work, honestly. That's what the patch and spike
lanes are for. Across a project it's roughly a wash, and what you gain is that
the project *finishes*.

**"Doesn't a bigger context window solve this?"**
Helps within a session, does nothing across sessions, machines, or six months.
And the record has a second reader: you.

**"What does it cost in tokens?"**
Every spec records its own cost and an audit fails a shipped spec with no real
cost data. Caveat honestly: only delegated sub-agent work is metered reliably, so
recorded numbers are a floor, not a total.

**"Would this work for a team?"**
It's designed for agents that don't share memory, which is structurally the same
problem as people who don't share context. But say what you know: the evidence is
solo side projects, and the two-agent variant is the least-evidenced path here.

**"What about existing codebases?"**
Same shape as graduating a spike: write down the decisions that still constrain
what you build next, frame the first project around what's *next*, and don't
retro-write specs for code that already works.

**"Why build your own instead of using <framework>?"**
Honest answer: it started as a folder of markdown conventions and only became a
template because the conventions kept working. The zero-dependency constraint is
the load-bearing design choice — nothing to install means nothing to abandon.

---

## Fact sheet

Re-check before presenting; some numbers move.

| Claim | Source |
|---|---|
| bragfile: Go CLI, Homebrew tap, PROJ-001 = 5 stages / 23 specs / 14 DECs / ~4 weeks | [`PROJECTS.md`](../../PROJECTS.md) |
| 4 projects / 42 specs / 27 DECs / 3 releases; 40 of 42 shipped; every stage in order; zero design→ship drift; 25 DECs one supersession; mean confidence 0.823, no 1.0s | [`feedback/2026-07-04-…retro.md`](../../feedback/2026-07-04-bragfile-three-project-retro.md) |
| ~2 weeks stated vs ~28 days actual (2× overrun, recorded) | [harvest — time-to-value](../harvests/2026-07-06-three-project-dogfood-harvest.md) |
| Every escaped defect was operational/runtime — the six-item list | [retro](../../feedback/2026-07-04-bragfile-three-project-retro.md) §"one structural gap" |
| Behavioral pre-flight; `GenBashCompletion` vs `plugin validate --strict` pair | same, §A |
| **`brag add -t … -k shipped -i "<impact>"` at ship, on by default; seeded from `value_link` + `cost.totals`; MCP `brag_add`; the wrapper was built then deleted (v0.6.10 → v0.6.11)** | [DEC-010](../decisions/DEC-010-accomplishment-logging-default.md) |
| **Ship Reflection Q5 is the line `impact` is transcribed from** | [`projects/_templates/spec.md`](../../variants/claude-only/projects/_templates/spec.md) |
| crustyimg: ~43 specs / 9 stages / 73 DECs; SPEC-001 created+shipped same day; invented the patch lane | [harvest](../harvests/2026-07-06-three-project-dogfood-harvest.md), [DEC-003](../decisions/DEC-003-patch-lane.md) |
| zany: 37 specs / 6 stages; invented CI guards for subjective quality, refuting its own "juice resists TDD" risk | [harvest #4](../harvests/2026-07-06-three-project-dogfood-harvest.md) |
| skillport surfaced two template fixes (v0.6.22, v0.6.24) before any deliberate harvest; `uw` and `rspeed` never harvested | [`instances.md`](../harvests/instances.md) |
| standup started outside the process | [`instances.md`](../harvests/instances.md), [DEC-012](../decisions/DEC-012-spike-lane.md) |
| `just specs-by-stage` back-ported from bragfile | [`PROJECTS.md`](../../PROJECTS.md) |
| Independent verify + DEC log were the two error-preventers; "value tracks stakes, not size" | [harvest — meta-conclusions](../harvests/2026-07-06-three-project-dogfood-harvest.md) |
| N=3 / N=2 bar; "don't push it to codify sooner" | [`docs/signals.md`](../../variants/claude-only/docs/signals.md) |
| Spike lane shipped at N=1 as a recorded exception, with a revisit trigger | [DEC-012](../decisions/DEC-012-spike-lane.md) |
| 121 specs across three projects | [harvest](../harvests/2026-07-06-three-project-dogfood-harvest.md) |
| Cost undercounted (only sub-agents metered) | [`ROADMAP.md`](../ROADMAP.md) |

**Three things to keep straight if challenged:** some conventions the retro
praises (`premise-audit`, `PEEL-IF-L`) are **bragfile-local, not template
features**. The AGPL catch is from the cross-project harvest, not bragfile
specifically — attribute it as "across the projects." And `rspeed` / `uw` are
*unharvested*, which is not the same as *unsuccessful* — don't claim they failed.
