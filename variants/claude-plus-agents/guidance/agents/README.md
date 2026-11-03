# guidance/agents/ — optional reviewer briefs

**Everything here is optional and unproven.** Nothing loads these files
automatically. They are prompt fragments you point a session at when you want a
review from a *specific angle*, and an empty directory is a perfectly good
steady state.

## The one rule that makes these work — or makes them theatre

**A brief is only worth anything in a session that did not do the work.**

Handing these to the same session that wrote the code produces a costume, not a
reviewer: the context is already contaminated by every choice being reviewed, so
it will agree with itself in a different voice. That is why the template buys
independence with the **session boundary** — a fresh session, or a delegated
agent — and treats the brief as a *lens on top of that boundary*, never a
substitute for it.

If you only remember one thing: **the boundary is what makes the review real.
The brief only decides what it looks at.**

## How to use one

- **Delegating (this variant):** link the brief from the handoff's context list,
  alongside `guidance/toolchain-brief.md`. This is the strongest form available —
  the reviewer is a different *tool*, not merely a different session, so the
  boundary is real rather than conventional.
- **Not delegating:** open a new session, paste the brief, then the spec.
- **Routing:** `.repo-context.yaml` → `spec.agent.tier_map` already sends
  different cycles to different agents; a brief says *what that agent is looking
  for* once it arrives.

## Cost, honestly

Each extra review is another full pass over the work. Two lenses on one spec is
roughly double the verify spend. **Treat these as a stakes tier, not a default**
— reach for a second lens on the specs where being wrong is expensive, not on
every spec. Most specs need one review.

## What's here, and what deliberately isn't

| Brief | Lens | Can reject for |
|---|---|---|
| `reviewer-correctness.md` | does it do what the spec says | unmet criteria, missing tests, unhandled errors |
| `reviewer-design.md` | is this the right shape *here* | duplication, violating a DEC, foreign patterns |
| `qa-close.md` | is the wave actually done | a close that would record fiction |

The first two are deliberately split: **"does it work" and "is it right" are
different questions**, and one reviewer asked to do both will answer the easy
one. That split is the only thing here with real precedent behind it.

**There is deliberately no `stakeholder.md`.** Simulating the person you built
it for is the failure mode, not the fix — a proxy stakeholder is still you, and
`value_realized.thesis_held` scored by a simulation of your own customer is
worth less than an honest `too-early`. The real move is finding one.

## Writing your own

Keep it under ~200 words. State what the reviewer **ignores** as explicitly as
what it looks for — a brief that permits everything changes nothing. And give it
a **narrow licence to reject**; a reviewer that cannot say no is a formatter.
