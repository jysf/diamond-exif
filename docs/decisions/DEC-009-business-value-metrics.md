---
insight:
  id: DEC-009
  type: decision
  confidence: 0.6
status: proposed            # proposed | accepted | superseded
created_at: 2026-07-06
deciders: [jysf, claude]
supersedes: null
superseded_by: null
affected_scope:
  - "variants/*/projects/_templates/stage.md"
  - "variants/*/projects/_templates/project-brief.md"
  - "variants/*/FIRST_SESSION_PROMPTS.md"
  - "variants/*/AGENTS.md"
  - "scripts/dash.sh"
  - "scripts/_lib.sh"
tags: [architecture, value, metrics, time-to-value, measurement]
---

# DEC-009: measurable business value + time-to-value

> **This is the template's own decision log** (meta). **Status: proposed** — a
> draft for review, grounded on the 2026-07-06 three-project dogfood harvest
> ([docs/harvests/](../harvests/2026-07-06-three-project-dogfood-harvest.md)).
> **Do not build the derivation aid cold** — its real test is the *frame* moment
> of the next new project (derive a real target metric live; if it's awkward,
> that's the signal). The plumbing (`shipped_at` stamp) shipped in v0.6.5.

## Context

The goal: capture metrics through the process so we can judge whether we're
building the right things — **measurably, even if qualitatively.** The harvest
makes the shape unusually clear:

1. **Value capture is already strong and structured — protect it.** Every project
   carries a `thesis` / `beneficiaries` / `success_signals` / `risks_to_thesis`
   block, credited with catching scope a feature list would hide.
2. **The gap is *quantification*, not capture.** Success signals are
   narrative/binary ("all five states reachable"). But *both* projects' second
   waves **independently started adding real numbers** (zany: target RTP 94% /
   hit-freq 40% + a metrics simulator; crustyimg: byte-reduction at SSIMULACRA2 ≥
   target). They are organically reaching for exactly the measurable target we
   want — so this DEC should **formalize what they're already doing**, not invent
   a metrics system.
3. **Time-to-value is real and was computable only after the fact.** First ship is
   consistently same-day/next-day (the scaffold front-loads a shippable
   increment); full value scales with scope (15–28 days). bragfile recorded a **2×
   miss vs a stated "~2 weeks" target** — the predicted-vs-realized loop happening
   ad hoc. Worth formalizing.
4. **"Value tracks stakes, not size."** The template earned its keep on high-stakes
   specs; the metric model should not tax low-stakes work.

## Decision (proposed)

Add a **thin measurable-value layer** on top of the existing narrative value
block. Three parts, all additive; none is a gate.

### 1. One headline target metric per stage (quantified *or* checkable)

A `value_metric` block on the **stage** (the unit that delivers value; a spec
stays cost-only — too granular). Qualitative is allowed: the `target` may be a
checkable statement, not just a number.

```yaml
value_metric:
  metric:   "median format-routing accuracy on the fixture corpus"  # what
  baseline: "n/a (new capability)"                                   # from where
  target:   ">= 95% correct photo→lossy / graphic→lossless"          # to where (num OR checkable)
  method:   "run `optimize` over 200 labeled fixtures, count routes" # how measured
  realized: null   # filled at ship — the predicted-vs-realized readout
```

### 2. A metric-derivation aid (the "find the metric" step)

The hard part is *finding* the metric, not recording it. Add a **frame/design
prompt** that derives a candidate target metric from the thesis: *"What is the
one number — or one checkable outcome — that would prove this thesis true? What's
today's baseline?"* The harvest shows this is derivable (crustyimg's
success_signals → "≥X% byte reduction at SSIMULACRA2"; zany → "median
spins-per-session"). **Validate this aid on the next new project's frame**, not
retroactively.

### 3. Time-to-value + predicted-vs-realized, computed from data

`shipped_at` is now stamped at archive (v0.6.5), so — with the stage/spec
`created_at` — **time-to-value is computable with zero new judgment**: earliest
`created_at` → first `shipped_at` (time-to-first-value) and stage/project spans
(time-to-full-value). A proposed **`just dash value`** lens surfaces, per stage:
the `value_metric` target vs `realized`, and the computed TTV + cycle-times. The
predicted-vs-realized delta (target vs realized; and a stage's stated time target
vs actual) is the "are we doing the right things?" signal.

## Alternatives considered

- **RICE-style value/effort scoring** — good for prioritization but doesn't
  answer "did we move the needle?"; and the projects are reaching for *outcomes*,
  not scores. Could layer later.
- **Adoption/usage instrumentation** — the right long-run answer for external
  products, but it needs a live product + analytics; premature as the default.
- **Spec-level value fields** — rejected: too granular; "value tracks stakes"
  says most specs are plumbing. Stage is the value unit.
- **A value gate (like cost-audit)** — deferred: value targets are
  judgment-laden and sometimes qualitative; keep the checklist/convention posture
  of DEC-006/007/008.

## Consequences

- Value stays narrative-first (protected) but gains one measurable headline per
  stage + a computed TTV — the projects' own trajectory, made standard.
- One derivation prompt + one `value_metric` block + one `dash value` lens to
  build and maintain.
- **Dependency / caveat:** any *cost*-derived value metric inherits harvest signal
  #5 (sub-agent metering can silently undercount) — needs the implausibility flag
  before cost feeds a value/ROI number.

## Open questions

1. **Scope:** stage-only, or also a project-level rollup metric in the brief?
   (Leaning stage-spine + brief rollup.)
2. **Qualitative targets in `--json`:** how to represent a checkable-but-not-numeric
   target so the `dash value` lens / reports stay machine-readable.
3. **Derivation-aid placement:** the FRAME prompt (before design) vs the stage
   scaffold. Decide against the next real project.
4. **Does `realized` want a tiny vocabulary** (`hit | missed | partial | unmeasured`)
   next to the free-text, so the predicted-vs-realized distribution is greppable
   cross-project (mirroring the defect-catch-stage tag)?
5. **Goal-less / exploratory projects (user signal, 2026-07-06).** Naming a target
   metric is genuinely hard — and can feel artificial — when there's **no explicit
   business goal** (a personal tool, an experiment, a dogfood). Forcing a number
   there produces theatre. The derivation aid must offer an **escape hatch**: a
   *proxy* (does the headline capability work end-to-end; time-to-first-ship /
   time-to-value), a *checkable success signal* (qualitative but falsifiable), or
   an explicit **exploratory** mark with a learning goal ("no business metric yet —
   success is X works, and I learn Y"). The metric should scale to the project's
   actual goal *type*, and "honest none" must be a first-class answer — otherwise
   the convention rewards fabrication. This is the crux to validate on the next
   real project's frame.
