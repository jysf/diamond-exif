---
# A SPIKE is a BOUNDED EXPLORATION — the phase before you know the shape.
# Two modes, one discipline:
#   mode: question — a timeboxed investigation. Code is evidence, usually thrown away.
#   mode: build    — a vibe-coding session. Code is the deliverable, you intend to keep it.
# See AGENTS.md "Spike lane" and docs/decisions/DEC-012.
#
# Collapsed cycle: spike -> land.
#   spike — explore. NO spec, NO failing tests, NO DEC required. Speed IS the value.
#   land  — MANDATORY. The entire point of the lane: write down what you learned,
#           emit the DECs the exploration already made, decide the code's fate.
#
# There is deliberately NO verify step: a spike has no acceptance criteria, so an
# "independent verify" would have nothing to check (DEC-012). The timebox and the
# mandatory land step are the disciplines that replace it.

task:
  id: SPIKE-XXX
  type: spike                      # epic | story | task | bug | chore | patch | spike
  cycle: spike                     # spike | land  (collapsed from a spec's 5)
  blocked: false
  priority: medium

spike:
  question: __QUESTION__
                                   # REQUIRED. A spike with no question is just coding.
                                   # For mode: build this is legitimately loose
                                   # ("is a local standup tool worth having?").
                                   # Loose is fine. Absent is not.
  mode: __MODE__                   # question | build
  timebox: __TIMEBOX__
                                   # REQUIRED. Exceeded means STOP and land it as
                                   # `inconclusive` — not extend. Extending twice
                                   # means it isn't a spike, it's an unframed project.
  outcome: null                    # set at LAND. Never leave null on a landed spike —
                                   #   answered     — the question got an answer
                                   #   inconclusive — timebox hit, no answer (a real result)
                                   #   graduated    — the code becomes real work (see below)
                                   #   discarded    — the code is thrown away (also a win)
  landed_at: null                  # YYYY-MM-DD, stamped at land

project:
  id: __PROJECT_ID__               # OPTIONAL (null) — a spike may PRECEDE any project
                                   # (that's the point). Back-link at land if one exists.
  # No `stage:` — a spike attaches to the repo, not a stage.
repo:
  id: __REPO_ID__

agents:
  explorer: __IMPLEMENTER_MODEL__  # who ran the spike (tier_map.build; DEC-005)
  created_at: __TODAY__

references:
  decisions: []                    # DECs EMITTED AT LAND (not during — that's the point)

# Cost is ADVISORY for spikes — `just cost-audit` does NOT gate them (DEC-012 v1).
# A spike is often pre-project and deliberately cheap; a cost gate on exploration
# is exactly the friction that would make people skip the artifact.
cost:
  sessions: []
  totals:
    tokens_total: 0
    estimated_usd: 0
    session_count: 0
---

# SPIKE-XXX: __QUESTION__

## Question

What you're trying to learn, and **why it matters what the answer is**. If the
answer wouldn't change what you do next, don't run the spike.

## Timebox

`__TIMEBOX__` — and what you'll do when you hit it.

**The rule:** hitting the timebox without an answer is `inconclusive`, and
`inconclusive` is a *real, useful result* — it says the question is harder than
it looked. Extending once is a judgement call. Extending twice means this is an
unframed project; stop and frame it.

## What's out of bounds

A spike may **not**:

- Ship user-facing behavior (that's a spec — or a patch, for shipped behavior).
- Be built upon before it lands. Land it first.
- Be a way to skip the cycle on work you already understand. If you can write
  acceptance criteria, you have a spec, not a spike.

---

## Log

*Free-form. Written during the spike, for you. No structure required — this is
the one place in the template with no conventions to follow. What you tried,
what surprised you, dead ends worth remembering.*

---

## Land

*The mandatory terminal step. A spike that is never landed is exactly the failure
this lane exists to prevent — undocumented decisions leaking into production.*

### Answer

What did you learn? Answer the `## Question` directly, in a few sentences. If the
outcome is `inconclusive`, say what would make it answerable next time.

### Decisions this exploration already made

The spike made real choices without writing them down — that's allowed *during*,
and settled *here*. For each one that **still constrains what you build next**,
emit a `DEC-*` and list it below with honest confidence.

Not archaeology: only the load-bearing ones. If the rationale is genuinely lost,
`confidence: 0.4` with a note is the truthful record, not a failure.

- `DEC-NNN` — <one line> (confidence: X.X)

### What happens to the code

Pick one and complete its block.

**`discarded`** — the code is thrown away.
- What's worth keeping in writing instead: <one or two lines>
- *(A discarded spike is a win: you bought an answer cheaply.)*

**`graduated`** — the code becomes real work. **The five-item contract (DEC-012):**
- [ ] `.repo-context.yaml` describes what now exists
- [ ] `AGENTS.md` carries the real tech stack, commands, conventions
- [ ] `guidance/toolchain-brief.md` filled in from what this spike learned the
      hard way (most valuable here — a spike generates exactly this friction)
- [ ] Retroactive `DEC-*` for load-bearing choices only, honest confidence
- [ ] A project brief framed around **what comes next** — this spike is prior art
      in `Dependencies → Depends on`, not the subject of the project

  **Do NOT retro-write specs for code that already works.** A spec directs work
  that hasn't happened yet; written after the fact it describes behavior the
  tests already assert, and nobody reads it.

**`answered` / `inconclusive`** — the question resolved (or didn't); no code
graduates.
- What this unblocks or changes: <one or two lines>

### What's next

- [ ] <the concrete next action — frame a project, write a spec, drop the idea>

### Land checklist

- [ ] `spike.outcome` set (never leave it null — `just validate` fails on this)
- [ ] `spike.landed_at` stamped
- [ ] `project.id` back-linked if a project exists now
- [ ] `just advance-cycle SPIKE-NNN land`
- [ ] `just archive-spike SPIKE-NNN`
