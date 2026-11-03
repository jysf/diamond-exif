# Signals — one ledger so no feedback rots

Coding lessons in this framework have always had a forcing function: you reflect
at ship ("does anything need updating?") and a stage close forces a
codify / carry / drop decision on each one. Nothing rots, because every close
makes you decide.

**Process and tooling feedback never had that.** It landed in a `feedback/` doc
and then sat — un-adopted recommendations got re-flagged months later (see the
real example seeded into `guidance/signals.yaml`). The Signals registry fixes the
asymmetry: **every** kind of feedback becomes a typed record with an owner and a
forced disposition, not just the coding kind.

It is deliberately lightweight — one YAML file and a ~10-minute walk at each
close. If it ever feels heavier than the friction it removes, it's being used
wrong.

## The registry

`guidance/signals.yaml` is the single ledger (a sibling of `constraints.yaml` and
`questions.yaml`). One file on purpose: it *is* the cross-stage view — every
queued lesson and every un-adopted fix in one place, instead of prose scattered
across stage files. Each record is typed:

| `type` | What it is | Dispositioned at | Terminal states |
|---|---|---|---|
| **lesson** | A coding/process rule earned by recurring evidence, destined for AGENTS.md / a template / a constraint | **stage close** | `codified`, `dropped` |
| **process-debt** | Framework / tooling friction to fix (a `just` chore, a prompt tweak, a template change) | **project close** | `accepted`→`done`, `rejected`, `dropped` |
| **product** | A usage / dogfooding signal about what to build; feeds the **next** project's framing | **project close** | `accepted`, `dropped` |
| **risk** | Something to watch; no automatic action | revisited each close | stays `watch` |
| **golden-path** | An approach that worked well enough that another project should copy it wholesale | **project close** | `codified`, `dropped` |

**`golden-path` is the odd one out, deliberately.** Every other type records a
*problem*. That meant the template could capture "this hurt" in five ways and
"this worked" in none — so every close quietly discarded the more useful half.
It exists because the template has **no instance-to-instance transfer mechanism
at all**: knowledge flows *up* through harvests into the template and *down*
into new scaffolds, but never sideways from one repo to the next. A
`golden-path` is the seam where a thing one project learned can reach another.

**Its bar is harder than a lesson's, not softer.** A wrong paved road is worse
than no road, because people follow it without re-deriving it. So: capture
candidates freely, promote almost none. `bar: 'N=3 same-outcome'` applies, and
"it worked once and felt good" is a preference, not a golden path. When in
doubt, leave it at `watch` for another project.

`status`: `open` (raised, never dispositioned) · `watch` (deliberately parked —
a lesson accumulating toward its bar, or a risk being monitored, or a deferred
fix with a trigger in `notes`) · `accepted` (decided: will do) · `rejected`
(decided: won't, reason in `notes`) · `codified` (a lesson that landed in
AGENTS.md/template) · `done` (accepted work completed) · `dropped` (no longer
relevant).

`open` and `watch` are the **non-terminal** states — the ones a close must walk.
Everything else is settled.

## The codification bar (lessons only — unchanged)

A `type: lesson` only graduates to `codified` once its evidence meets the
**bar**, recorded in the `bar` field:

- **`N=3 same-outcome`** — three independent specs hit the *same* failure the
  *same* way. Three confirmations beat one-off pattern bloat.
- **`N=2 paired-opposing`** — two specs where the rule was confirmed once by its
  presence and once by its absence (the opposite choice caused a problem).

Below the bar, a lesson stays `watch` with the running N in `evidence`. **Do not
codify mid-stage** — let the bar do its job; the stage close is where it lands.
This is the same bar the WATCH convention always used; the registry just makes
the running count visible across stages instead of buried in one stage file.

## The close-disposition ritual (the forcing function)

At each close (Prompt 1d for a stage, Prompt 1e for a project), walk every
`open`/`watch` signal **that close owns** (`disposition_at` matches). Each one
gets exactly one of:

- **accept-and-schedule** → `accepted` (note where/when it lands; for a lesson
  at its bar, `codified` once you've landed the rule).
- **reject-with-reason** → `rejected` (one line in `notes` saying why).
- **defer-with-trigger** → stays `watch`, with the trigger in `notes`
  ("revisit when a 4th spec hits this" / "next time we touch the verify lane").

Then bump `last_touched`. **No silent carry** — a signal you don't touch is the
exact failure this registry exists to prevent. A signal sitting `open` across two
of its own closes is a red flag: decide it.

Owner by type, so the walk is short at each close:

- **Stage close** → only the `lesson` signals (`disposition_at: stage-close`).
- **Project close** → `process-debt`, `product`, `risk`, and `golden-path`
  (`disposition_at: project-close`).

## Raising a signal (the capture half)

Capture is woven into the reflections you already do — give it teeth by writing
the signal down the moment it's raised but *not* acted on:

- **Spec ship** (Reflection Q2, "does any template/constraint/decision need
  updating?"): if yes and you're not doing it this session → add a signal.
  Recurring coding pattern → `type: lesson` (set/raise its `bar` N-count);
  framework/tooling friction → `type: process-debt`.
- **Stage reflection**: lessons ready or accumulating → `type: lesson`.
- **Project reflection / usage observations**: what-to-build-next →
  `type: product`; standing hazards → `type: risk`.
- **Anything that worked unusually well** — a library choice, a test shape, a
  file layout, a review habit — → `type: golden-path`. Ask it explicitly at
  project close, because nothing else prompts for it: *what would I tell the
  next project to just copy?* If the answer is "nothing", that is a fine answer;
  a blank here is honest, while a golden path never written down is lost.

Keep `feedback/` for the raw inbound capture (a dated dump from a user or a
dogfooding session). `signals.yaml` is the **triaged** ledger those get distilled
into — the part with the forcing function. One is the inbox; the other is the
list you're held to.

## Migrating an existing WATCH convention

If this repo already grew its own per-stage "WATCH at N=1 / N=2" notes inside
stage files, fold each into a `type: lesson` entry **without losing anything**:

1. One stage-file WATCH note → one signal. `summary` = the rule; `evidence` =
   the confirming specs **with the N-count verbatim** (e.g.
   `SPEC-026/028/029 — N=3 same-outcome`); `bar` = the bar it's tracking against.
2. `status`: `watch` if still accumulating, `open` if it just hit its bar and is
   awaiting the stage close. An already-codified rule needs no entry (it's in
   AGENTS.md) — only migrate the *pending* ones.
3. `disposition_at: stage-close`; `first_flagged` = when the note started;
   `last_touched` = today.
4. Delete the prose note from the stage file once it's in the registry. The N is
   preserved in `evidence`, so the bar still gates codification exactly as before.

The bar, the cadence, and the codify-at-stage-close rule are all unchanged — only
the storage moves, from scattered prose to one visible file.
