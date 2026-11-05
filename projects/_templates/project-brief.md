---
# Maps to ContextCore project.* semantic conventions.
# A project is a bounded wave of work against the repo (the app).

project:
  id: PROJ-XXX                      # stable, zero-padded, never reused
  status: proposed                  # proposed | active | on_hold | shipped | cancelled  (coarse; tooling keys on this)
  activity: null                    # optional: requirements | design | build | test | blocked
                                    #   (open set) — what work is happening now within an active project
  priority: medium                  # critical | high | medium | low
  target_ship: null                 # optional: YYYY-MM-DD

repo:
  id: __REPO_ID__                   # must match .repo-context.yaml

created_at: YYYY-MM-DD
shipped_at: null

# OPTIONAL, stamped at close by `just close-project`. `status` is the coarse
# machine state; this says WHY the work stopped, which is the more interesting
# signal and the one `cancelled` flattens:
#   shipped    delivered the thesis
#   abandoned  stopped; thesis unproven
#   superseded the thesis moved into another project
#   parked     may resume (pair with status: on_hold)
# Open set (the `activity` precedent) — extend it; validate warns, never fails.
#
# It is also load-bearing, not decorative: close-project REFUSES to close a
# project with specs still in flight when you claim `shipped`, and allows it
# when you say `abandoned`. In-flight specs contradict "shipped"; they are
# expected in an abandonment.
closed_reason: null

# Business value. Testable claim, not marketing copy.
# "Users will love it" is not a thesis; "reducing month-2 churn by
# making activation faster" is. Leave null only if genuinely unknown.
value:
  thesis: null
  beneficiaries: []                 # 2-4 entries: users, team, function
  success_signals: []               # 3-5 observable outcomes
  risks_to_thesis: []               # 2-4 honest things that could make this wrong

# The OTHER HALF of `value:` above. That block is the prediction, written when
# you knew least. This is what actually happened, written at close by
# `just close-project` — the only moment the whole wave is knowable.
#
# Without it the template predicts and never scores, which is why "was the idea
# any good?" has never been answerable across projects. A prediction with no
# recorded outcome is not a hypothesis; it's a wish.
#
# `too-early` is a real, honest answer — some theses need months of use to
# judge. What is NOT acceptable is leaving it null on a project you claim
# shipped: close-project refuses that combination.
value_realized:
  thesis_held: null                 # yes | partly | no | too-early
  signals_observed: []              # which success_signals above actually showed up
  evidence: null                    # one line — the number if you have one, the observation if not
  notes: null                       # what you'd predict differently next time

# OPTIONAL declared roadmap (DEC-011). Forward intent that is NOT a stage yet —
# themes and outcomes too coarse or too early to frame. `just roadmap` merges
# these with the DERIVED roadmap (framed STAGE-*.md files + the Stage Plan
# below) and emits both via `--json`, so a portfolio tracker reads one surface
# instead of re-parsing this file.
#
# Leave it empty. Most projects never need it: the Stage Plan already says what
# is coming. Reach for it when intent outlives the current wave.
#
#   kind:    pillar | goal        (framed/planned are inferred from files)
#   horizon: now | next | later   — buckets, because a date you can't hit is
#                                   worse than a bucket you can
# An `item:` naming a real STAGE-NNN is reconciled against that stage rather
# than listed twice; its horizon rides along on the derived row.
roadmap: []
# roadmap:
#   - item: "Weekly & monthly rollup reports"
#     kind: pillar
#     horizon: next
#     resume_when: "after the daily report stabilizes"   # optional trigger
#     target: null                                       # optional YYYY-MM-DD
---

# PROJ-XXX: <Short Title — the wave of work>

## What This Project Is

One paragraph. What wave of work is this? If someone asked "what are
you doing this quarter," this paragraph is the answer.

## Why Now

Why this wave, in this order, at this time. If the answer is "it
seemed like a good idea," the project isn't ready.

## Success Criteria

Concrete outcomes that would mean this project succeeded. Not a list
of features — a list of capabilities, metrics, or user-observable
changes.

- ...
- ...
- ...

## Scope

### In scope
- ...
- ...

### Explicitly out of scope
- ...
- ...

## Stage Plan

Ordered list of stages this project will produce. A project typically
has 2–5 stages. Update as work proceeds.

Format: `- [status] STAGE-ID — one-line summary`

- [ ] (not yet defined) — <one-line summary>
- [ ] STAGE-NNN (active) — <summary>
- [x] STAGE-MMM (shipped on YYYY-MM-DD) — <summary>

**Count:** 0 shipped / 0 active / 0 pending

## Dependencies

### Depends on
- External: <third-party API, vendor, approval>
- Previous projects: <PROJ-YYY shipped something this depends on>

### Enables
- Future projects: <what becomes possible after this ships>

## Project-Level Reflection

*Filled in when status moves to shipped.*

- **Did we deliver the outcome in "What This Project Is"?** <yes/no + notes>
- **How many stages did it actually take?** <number, compare to plan>
- **What changed between starting and shipping?** <one or two sentences>
- **Lessons that should update AGENTS.md, templates, or constraints?**
  - <one-line updates>
- **What did we defer to the next project?**
  - <one-line items>
