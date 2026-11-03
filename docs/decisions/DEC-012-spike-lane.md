---
insight:
  id: DEC-012
  type: decision
  confidence: 0.7
status: accepted            # proposed | accepted | superseded
created_at: 2026-08-09
deciders: [jysf, claude]
supersedes: null
superseded_by: null
affected_scope:
  - "scripts/new-spike.sh"
  - "scripts/archive-spike.sh"
  - "variants/*/projects/_templates/spike.md"
  - "scripts/validate.sh"
  - "scripts/status.sh"
tags: [architecture, process, methodology, spike-lane, exploration, cli]
---

# DEC-012: a "spike" lane for bounded exploration (and vibe-coding sessions)

> **This is the template's own decision log** (meta), separate from the
> `decisions/` that ships *inside* each variant. **Accepted 2026-08-09.**

## Context

The template models **committed** work well — a project is a wave, a stage is a
chunk, a spec is a task, a patch is a fix. Every one of those artifacts assumes
you already know what you're building.

It has no home at all for the phase *before* that: the bounded exploration where
you don't know the shape yet, and the point is to find out. Two real shapes of
this, which the owner named directly:

- **A spike** — a timeboxed investigation answering a specific question. *"Can
  this library do X on our pinned toolchain?"* The code is evidence; it's usually
  thrown away.
- **A vibe-coding session** — exploratory building where the design emerges from
  the doing. The code is the point, and you intend to keep it.

**Evidence this is real and currently unmodelled:**

- **`standup`** (`localStandupPlus000`) started exactly this way — built first,
  process applied afterward — and is now a full-tier instance in the registry.
- The registry's own removed **Discipline** column (2026-07-28) recorded the
  finding that the owner "doesn't recognize the distinction" between explicit and
  loose process adherence — i.e. exploration and disciplined work already
  interleave in practice, invisibly.
- **AGENTS.md §8 already anticipates it**: `project.activity` is documented as an
  open set *"(extend it, e.g. `spike`)"*. The vocabulary was reserved and never
  given an artifact.
- The dogfood meta-conclusion **"value tracks stakes, not size"** — the full
  cycle was "pure tax on low-stakes mechanical" work. An unproven idea is the
  lowest-stakes state there is: running five cycles over a question you haven't
  answered is ceremony over a guess.

The failure mode this addresses is **not** the exploring. Exploring is correct
and fast. The failure is that an exploration **never declares itself finished**,
so undocumented decisions leak into production and nobody can reconstruct why
anything is the way it is. That is precisely the problem the whole template
exists to solve, and it currently has a hole in it exactly where projects start.

## Decision

Add a **spike lane**: a bounded exploration that runs a collapsed
**`spike → land`** cycle.

| Spec cycle | Patch cycle | **Spike cycle** |
|---|---|---|
| frame → design → build → verify → ship | patch → verify → ship | **spike → land** |

- **spike** — explore. **No spec, no failing tests, no `DEC-*` required during.**
  Deliberately unconstrained; the speed *is* the value.
- **land** — **mandatory, and the entire point of the lane.** Write down what you
  learned, emit the `DEC-*` records the exploration already implicitly made, and
  decide what happens to the code.

Two **modes**, sharing one artifact because they share one discipline (a bounded
exploration that must terminate in writing):

| `spike.mode` | Is | Code is | Lands as |
|---|---|---|---|
| **`question`** | A timeboxed investigation | Evidence | `answered` or `inconclusive` |
| **`build`** | A vibe-coding session | The deliverable | `graduated` or `discarded` |

### Required up front — the two fields that make it a spike

- **`spike.question`** — what you're trying to learn, in one sentence. *A spike
  with no question is just coding.* For `mode: build` this is legitimately loose
  (*"is a local standup tool worth having?"*) — loose is fine, absent is not.
- **`spike.timebox`** — `2h`, `1d`, `1 session`. **Exceeded means stop and land
  it as `inconclusive`**, not extend. Extending a timebox twice means it isn't a
  spike; it's a project you haven't framed.

### Why there is no `verify` step (the deliberate divergence from DEC-003)

The patch lane keeps an independent verify because a patch fixes **known**
behavior against a **known** expectation — there is something to verify against.
A spike has neither acceptance criteria nor a spec, so an "independent verify"
would have nothing to check and would degrade into theater. Adding a ceremonial
verify here would discredit the real one in the other two lanes.

The disciplines that replace it are the **timebox** and the **mandatory land
step**. If the spike's code is going to production, it graduates into specs and
gets the real verify *then* — see the graduation contract below.

### The graduation contract (`mode: build` → real work)

When a `build` spike lands as `graduated`, five things get written and one
explicitly does not. This is the conversion ritual, now enforced by the land
step's checklist:

1. **`.repo-context.yaml`** — describe what now exists.
2. **`AGENTS.md`** — real tech stack, commands, conventions. The spike already
   decided these; record what's true.
3. **`guidance/toolchain-brief.md`** — everything the spike learned the hard way.
   Most valuable here: a spike generates exactly this kind of friction.
4. **Retroactive `DEC-*` for load-bearing choices only** — a choice that still
   constrains what you build next, with **honest confidence**. Not archaeology;
   if the rationale is genuinely lost, `confidence: 0.4` with a note is the
   truthful record.
5. **A project brief framed around what comes *next*** — the spike is prior art
   in `Dependencies → Depends on`, never the subject of the project.

**Explicitly not done: retro-writing specs for code that already works.** A spec
directs work that hasn't happened yet; written after the fact it produces a
document nobody reads describing behavior the tests already assert.

### Guardrails

- **A spike may not ship user-facing behavior.** If it's going to users, it's a
  spec (or, for shipped behavior, a patch).
- **Code from an un-landed spike may not be built upon.** Land it first.
- **A spike is not a way to skip the cycle on work you already understand.** If
  you can write acceptance criteria, you don't have a spike — you have a spec.

## Integration

A spike is **repo-level, not project-level** — the decisive difference from a
patch. `standup`'s spike predated its project; forcing `projects/PROJ-NNN/` on it
would have made the lane useless in the exact case that motivated it.

- **`spikes/SPIKE-NNN-<slug>.md`** at the repo root; archived to `spikes/done/`.
- **`project.id` is nullable** — a spike may precede any project, and gets
  back-linked at land if one exists.
- **`just new-spike "question" [--timebox 1d] [--mode build] [PROJ-NNN]`**;
  **`just archive-spike SPIKE-NNN`**. `SPIKE-*` is its own repo-wide continuous
  sequence.
- **`just validate`** validates spike front-matter (cycle enum `spike|land`; no
  stage required; `project.id` optional) and **fails a landed spike with an empty
  `spike.outcome`** — the one mechanical tooth, aimed squarely at the failure
  mode.
- **`just status`** lists open spikes with their timebox.
- **`spike`** joins the suggested `project.activity` set, so a project exploring
  before it frames anything stops drawing an advisory warning.

**Scope (v1):** `just cost-audit` does **not** gate spike cost. A spike is often
pre-project and deliberately cheap, and a cost gate on exploration is exactly the
friction that would make people skip the artifact. Cost fields exist and are
advisory. Revisit if spikes turn out to be expensive.

## Alternatives considered

- **Document it as an on-ramp in prose only, no artifact** (the previous state,
  and the initially-chosen scope). Honest about the N=1 evidence, but it leaves
  the vibe-coding→production path with zero forcing function — which is the whole
  problem. **Rejected on the owner's direction**; the graduation ritual needs a
  checklist that exists somewhere a session will actually read.
- **Two separate lanes, `SPIKE-*` and `VIBE-*`.** They differ in what they
  produce, not in what discipline they need — both are bounded explorations that
  must terminate in writing. Two artifacts would double the vocabulary to buy one
  `mode:` field. **Rejected** in favor of one lane, two modes.
- **Keep the independent verify, mirroring the patch lane.** Nothing to verify
  against; would be theater and would erode the real verify elsewhere.
  **Rejected** — see above.
- **`project.activity: spike` alone, no artifact.** Already available and already
  insufficient: it marks that exploration is happening, captures nothing about
  what was learned, and has no terminal step. **Rejected** — kept as a
  complementary signal, not a substitute.
- **Require a `DEC-*` during the spike.** Defeats the purpose; the whole value is
  moving without stopping to write. **Rejected** — deferred to `land`, where it's
  mandatory.

## Consequences

- **Positive:** the pre-commitment phase becomes a first-class, *terminating*
  artifact instead of an untracked chat log. The graduation ritual gets a home
  that a session reads. Exploration stops being invisible in the record.
- **Negative:** a third cycle vocabulary (`spike`/`land`) and a third id sequence
  (`SPIKE-*`). Real scope-creep risk — a "spike" that is actually unframed
  feature work — mitigated by the timebox rule and the no-user-facing-behavior
  guardrail. **A `land` step that gets skipped reproduces exactly the status quo**;
  the `validate` tooth on `spike.outcome` is the mitigation, and it only bites
  once the spike is marked landed.
- **Evidence level — stated honestly:** this is **N=1** (`standup`), below the
  template's own N=3-same / N=2-paired-opposing codification bar. It is shipped
  on the owner's explicit direction rather than on earned evidence, which is a
  deliberate exception to the deliberate-lag discipline and should be recorded as
  one. **Revisit at the next harvest**: if `SPIKE-*` artifacts are being created
  and *landed*, it earned its place; if spikes get created and abandoned
  un-landed, the lane is failing at its only job and should be cut back to prose.
