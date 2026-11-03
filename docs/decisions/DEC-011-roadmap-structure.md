---
insight:
  id: DEC-011
  type: decision
  confidence: 0.6
status: accepted            # proposed | accepted | rejected | deprecated | superseded
created_at: 2026-07-13
deciders: [jysf, claude]
supersedes: null
superseded_by: null
affected_scope:
  - "variants/*/projects/_templates/project-brief.md"
  - "variants/*/docs/schema-reference.md"
  - "variants/*/AGENTS.md"
  - "scripts/roadmap.sh"
  - "scripts/_lib.sh"
tags: [architecture, roadmap, interface-contract, portfolio, producer-consumer]
---

# DEC-011: roadmap structure

> **This is the template's own decision log** (meta). **Status: accepted —
> Phase 1 shipped v0.6.32 (2026-08-12).** It is grounded on two things that
> already existed and *disagreed*: the template's **derived** roadmap (`just
> roadmap` reads framed `STAGE-*.md` + planned `## Stage Plan` checkboxes,
> shipped v0.6.14) and the **declared** roadmap the `standup` tracker invented
> and read itself (a `roadmap:` brief front-matter block of pillars +
> `resume_when`). Codifying = giving both ends *one* structure.
>
> **What shipped in Phase 1:** the `roadmap:` block is blessed in the brief
> scaffold and `schema-reference.md` (both variants), parsed by
> `parse_declared_roadmap()` in `_lib.sh`, merged into `just roadmap` (human +
> `--json`, new `declared` bucket), and warned-on-but-never-gated by `just
> validate`. **Phase 2 is standup-side and unstarted** — until standup actually
> consumes `just roadmap --json`, the producer half is validated only by its own
> tests, and open question 2's reconciliation rule is a design call, not yet a
> field-tested one.

## Context

The template already produces a roadmap, and its main consumer already wants
one — but they don't share a definition:

- **Derived (template, built):** `scripts/roadmap.sh` renders one row per framed
  `STAGE-*.md` plus a `planned` bucket parsed from the brief's `## Stage Plan`
  checkboxes, with `--json`. This answers *"what work is shaped or named."*
- **Declared (standup, built downstream):** standup parses a `roadmap:` block of
  `{pillar, resume_when}` items from the brief and renders it on a project page.
  This answers *"what forward intent isn't a stage yet."* The template never
  blessed this block; standup reads it directly and only from the brief.

Two problems fall out:

1. **The two halves never meet.** A project's real roadmap is *both* its shaped
   stages and its unframed intent; today each lives in a different reader and
   neither sees the other. standup consumes only `just status --json` — it does
   **not** consume `just roadmap --json` at all — so the template's derived
   roadmap is invisible to the portfolio tracker.
2. **No portfolio view, and no shared contract to build one on.** standup's
   design scopes a cross-repo roadmap; only the per-project page shipped, because
   there is no stable roadmap envelope to aggregate.

Two constraints shape the fix:

- **Producer → consumer.** The template is the producer; standup is the
  aggregating consumer/product. Roadmap structure must be designed so standup
  consumes it through the typed `--json` surface (the DEC-001 Phase 4 direction),
  not by re-parsing briefs.
- **Not every tracked repo is spec-driven.** standup tracks repos across three
  tiers (`.repo-context.yaml` → `.standup.yaml` declared-lite → generic git).
  Any roadmap structure must be **optional and degradable**: declared-lite repos
  may offer a flat roadmap, generic repos none, and that must be fine.

## Decision (proposed)

Codify a single **project-level roadmap structure**, fed by two sources and
exposed through one `--json` contract.

### 1. Altitude — codify the project tier; defer the rest

- **Project roadmap** — lives in the brief. **This is what DEC-011 codifies.**
- **Plan roadmap** (cross-project) — **deferred to the Goals/Plans layer** (a
  future DEC). A Plan's roadmap is the union of its projects' roadmaps; do not
  design it until Plans exist.
- **Repo / portfolio roadmap** — **not a new artifact.** It is the aggregation of
  project roadmaps, which standup already does cross-repo. The template emits
  per-project; the consumer aggregates.

### 2. Two sources, one view — derive + declare

`just roadmap` merges and `--json` emits the union of:

- **Derived** (already built): `framed` (a `STAGE-*.md` exists) and `planned`
  (a `## Stage Plan` checkbox, no file yet).
- **Declared** (new, blessed): an optional `roadmap:` brief block for intent that
  is **not a stage yet** — themes/outcomes with a horizon.

A declared item that references a real stage (`item: STAGE-004`) is reconciled
against the derived set (no double-listing), exactly as `just roadmap` already
de-dupes planned vs framed.

### 3. Item taxonomy — a small closed `kind` set

`framed | planned | pillar | goal`

- `framed`, `planned` — derived; `kind` is inferred, not authored.
- `pillar` — declared, unframed theme coarser than a stage.
- `goal` — declared, an **outcome** rather than work; the seam to
  [DEC-009](DEC-009-business-value-metrics.md) (a roadmap `goal` is the natural
  place a `value_metric` attaches) and to the future Goals/Plans layer.

### 4. Horizon — buckets primary, trigger + date optional

- **`horizon: now | next | later`** — the primary axis. Skimmable, degrade-safe,
  no false precision.
- **`resume_when: "<trigger>"`** — optional; what standup already uses. Honest
  when a date can't be committed.
- **`target: YYYY-MM-DD`** — optional; precise but discouraged as the *only*
  signal (a dated roadmap you can't hit is worse than a bucketed one you can).

Rationale: matches the template's honest-escape-hatch pattern (DEC-005
null-safety, DEC-009's exploratory hatch) and standup's deliberate date-avoidance.

### 5. The `--json` contract (the "feeds standup" deliverable)

`just roadmap --json` emits the unified structure under the standard envelope, so
standup consumes **one typed surface** and drops its bespoke brief parsing. This
is the concrete producer↔consumer bridge and is exactly the typed-tool
consumption DEC-001 Phase 4 formalizes.

### 6. Degradation

Optional everywhere. Declared-lite `.standup.yaml` repos may set a flat
`roadmap:` / `next`; generic repos get nothing. The template **never requires** a
roadmap block; its absence is normal.

### Blessed block (grounded on what standup already reads)

```yaml
roadmap:
  - item: "Weekly & monthly rollup reports"
    kind: pillar            # framed | planned | pillar | goal  (framed/planned inferred)
    horizon: next           # now | next | later
    resume_when: "after the daily report stabilizes"   # optional trigger
    target: 2026-09-01                                  # optional date
  - item: STAGE-004         # a reference; reconciled against derived stages
    horizon: now
```

## Alternatives considered

- **Derived-only (no declared block).** Keep `just roadmap` reading only stages;
  tell people to frame a stage for anything on the roadmap. *Rejected:* forces
  premature framing of intent that isn't ready to be work, and standup already
  proved a declared layer is wanted (it built one).
- **Dates as the primary horizon.** *Rejected:* dates lie in solo/portfolio work;
  standup already avoids them. Buckets + triggers are the honest default.
- **A Plan-level roadmap artifact now.** *Rejected/deferred:* no Plans exist yet;
  the cross-project roadmap is the union of project roadmaps. Fold into the
  Goals/Plans DEC when that layer earns design.
- **Let standup keep parsing briefs itself.** *Rejected:* re-parsing the producer
  duplicates the schema in the consumer and blocks the portfolio view; the whole
  point of DEC-001 is that consumers read `--json`, not files.

## Consequences

- **standup gets one truth.** It consumes `just roadmap --json` and can finally
  build the portfolio roadmap view off a stable envelope.
- **Phase-4-aligned.** Roadmap becomes a typed surface an MCP server exposes for
  free (DEC-001 Phase 4).
- **A new optional front-matter to maintain.** `roadmap:` joins the schema; small,
  and gated by drift-guard + `just validate` like the rest.
- **Producer/consumer coupling is now explicit** — a good thing, but it means
  roadmap-schema changes need a migration note for standup.

## Open questions

1. **`goal` vs DEC-009 `value_metric` — one concept or two?** *(Still open.)* Is
   a roadmap `goal` item the *same* object a `value_metric` attaches to, or a
   lighter pointer to it? Resolve alongside the Goals/Plans layer; they likely
   converge. Phase 1 ships `goal` as a `kind` value with no metric attached, so
   nothing here is foreclosed.
2. **Merge/dedupe rules for declared-references-a-stage.** ✅ **Resolved in
   Phase 1: the derived record wins, the declared horizon rides along.** When
   `item: STAGE-004` names a stage that exists as a file or a Stage Plan row, it
   is *not* emitted as a separate declared item; the stage's own row carries
   `horizon`/`resume_when` as annotations. Rationale: the file is ground truth
   for status and dates, and double-listing is the exact bug the existing
   planned-vs-framed de-dupe avoids one layer down. The declared horizon does
   **not** reorder the derived output — ordering stays file-driven, because a
   consumer that wants horizon ordering can do it from the `--json`.
3. **Horizon vocabulary final?** ✅ **Resolved: `now|next|later`, as a suggested
   set rather than an enum.** No fourth bucket. `validate` warns on anything
   else and never fails, so a project that genuinely needs `someday` can just
   use it — which is a cheaper way to discover a missing bucket than debating
   one now.
4. **Does the template's own `docs/ROADMAP.md` adopt this structure?** ✅
   **Resolved: no — it stays prose.** It is the *meta* roadmap, and it carries
   evidence, N-counts and dissent that a structured block would flatten. The
   codified structure is for projects built *with* the template.
5. **Validate warn-only or hard?** ✅ **Resolved: warn-only**, following the
   `project.activity` precedent.

## Rollout sketch (once accepted)

- ~~**Phase 1 (template, non-breaking)**~~ **✅ Shipped v0.6.32 (2026-08-12).**
  `roadmap:` blessed in `schema-reference.md` + the brief scaffold (both
  variants); `parse_declared_roadmap()` in `_lib.sh`; `just roadmap` merges
  declared + derived in human output and emits a new `declared` bucket in
  `--json`; `just validate` warns-only on unrecognized `kind`/`horizon`; 8 new
  checks in `scripts/test.sh`.
- **Phase 2 (standup, cross-repo):** wire standup's Tier-1 enrichment to consume
  `just roadmap --json`, replacing its bespoke brief `roadmap:` parsing; build the
  portfolio roadmap view on the shared envelope. (standup-side work, read-only on
  the template.)
- **Phase 3 (later):** Plan-level roadmap — folds into the Goals/Plans DEC.

---

*Relationship: DEC-011 sits **below** the future Goals/Plans layer (a Plan
aggregates project roadmaps) and **beside** [DEC-009](DEC-009-business-value-metrics.md)
(a roadmap `goal` is where a value metric attaches). It **consumes**
[DEC-001](DEC-001-interface-contract.md)'s `--json` contract and is a natural
tool in its Phase 4 MCP surface.*
