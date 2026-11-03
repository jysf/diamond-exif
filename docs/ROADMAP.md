# Roadmap — spec-driven-template's own direction

The template's forward-looking backlog: improvements to **the template itself**
(distinct from ideas for apps *built with* it). Consolidates what was scattered
across `docs/decisions/`, `CHANGELOG.md`, and the dogfood harvests.

**Discipline (non-negotiable):** nothing here is a commitment. Priority is driven
by the **next dogfood harvest**, not by this list — codify a lesson only once it
recurs (N=3 same-outcome / N=2 paired-opposing). "Don't push it to codify sooner."
Full ranked detail + evidence lives in
[`docs/harvests/2026-07-06-three-project-dogfood-harvest.md`](harvests/2026-07-06-three-project-dogfood-harvest.md).

**Signal provenance — a new category (2026-08-12).** Until now every input to
this list came from an instance we own: the template grading its own corpus. The
external-comparison harvest folded in below is the first exception — a read
against *Kalendar Part 1*, an independently-developed spec-driven process. Worth
naming as its own provenance category, because an outside process is the only
source that can surface a hole we have no vocabulary for. Corpus at that survey
was reported as **362 specs (352 shipped) · 208 DECs across 8 live instances**.

> **⚠ Corrected 2026-08-13 — the baseline was inflated.** `bragfile-report` is
> **not an instance**: it is a stale checkout of `bragfile000` — same remote,
> same first three projects, 195 commits into the same 296-commit history,
> verified a **strict ancestor with no divergence**. Every figure that counted it
> double-counted **41 specs / 40 shipped / 25 DECs**, including the 2026-07-06
> harvest and the 2026-08-12 comparison. **Actual live corpus: 6 instances (+1
> dead `uw`) · 318 specs · 306 shipped · 183 DECs**, mean DEC confidence ~0.80.
> Registry fixed in [`instances.md`](harvests/instances.md).
>
> Two things worth separating. **The practice was never at fault** — the clone is
> a strict ancestor, so the "a clone forks the memory" hazard argued below still
> has *zero* observed instances. **The registry was.** And the failure mode is
> the interesting part: a stale checkout is indistinguishable from an instance
> by directory shape alone, so it got counted for months by three separate
> analyses, each inheriting the last one's number. Remaining unverified
> checkouts — crustyimg ×3, a zany worktree, standup ×3 — should be treated as
> suspect until someone checks their remotes the same way.

## Proposed — awaiting a real project to validate

- ~~**Expected-vs-actual estimation loop (raised 2026-07-18).**~~ **✅ Shipped v0.6.26** — the cheap half is built (t-shirt `XS|S|M|L|XL|XXL` expected size + `task.complexity_actual` at ship + optional `cost.tokens_estimate` + `just calibration`, warn-only). The size→token band is **derived from this repo's own shipped specs** rather than a shipped lookup table, so it earns trust as n grows. Still open, and deliberately not built cold: whether the drift view belongs in the project-close rollup, and whether stage-level estimates are worth capturing at all. Original capture:

- **Expected-vs-actual estimation loop (raised 2026-07-18).** Capture the
  prediction alongside the outcome so the template can tell you *how good your
  estimates are* — the feedback loop, not the number, is the value. Two axes:
  - **Size** — expand `task.complexity` (`S | M | L`) to a t-shirt scale
    `XS | S | M | L | XL | XXL`, recorded as the **expected** size at design/frame,
    plus an **actual** size stamped at ship. ("L means split it" shifts: `XL`/`XXL`
    almost certainly means it's a stage, not a spec.)
  - **Tokens** — an optional token **estimate** at design vs the **actual**
    (`cost.totals.tokens_total`, already captured). Ideal refinement: map each
    t-shirt size to a coarse token **band** (calibrated per-repo over time) so the
    size *is* a rough token estimate — answering "can we even estimate tokens?"
    with "yes, via the size you already assign."
  - **The loop** — a `just calibration` view (or a `specs-by-stage` column) showing
    expected-vs-actual drift per spec/stage: are we systematically under- or
    over-estimating? Feeds [DEC-009](decisions/DEC-009-business-value-metrics.md)
    (predicted-vs-realized), the orchestration **cost-gap** thread, and the
    project-close rollup (close is where the whole wave's drift is knowable).
  - **Discipline:** warn-only, never a gate. **Start cheap** — expected size +
    actual size + actual tokens are all near-free (the fields mostly exist); add
    the size→token-band mapping only once there's real data to calibrate against.
    Don't build the calibration view cold.

- **Closing / ending a project (raised 2026-07-18) — ✅ SHIPPED v0.6.34
  (2026-08-13).** `just close-project PROJ-NNN` + `closed_reason:`, with the
  three hard refusals below and `--dry-run`/`--json`. **What was cut, and why:**
  `close-stage` (the evidence is 28 *project* closes; stage close inherits the
  shared machinery free once it earns itself), and the "ready to close"
  informational state (a `status`/`dash` concern — bundling it would have
  blocked this on agreeing a second surface). The density × supersession line
  shipped **inside the rollup, not as a dashboard lens**, as argued below.
  Original capture: this entry carried a "don't build cold" gate. **28 projects
  have now closed**; the gate is spent.

  A sharper framing than "close is thin," which this entry carried and which was
  never quite right — the close *ritual* is genuinely detailed (Prompt 1d is
  eight steps, 1e seven-plus, and its signal-disposition step is more rigorous
  than anything in the external source). The real finding is an **asymmetry:
  close is the only major ritual with no mechanical counterpart.**

  | Ritual | Prompt | Command |
  |---|---|---|
  | Spec ship | ✅ | `archive-spec` — stamps `shipped_at`, computes `cost.totals`, runs `cost-audit` |
  | Patch ship | ✅ | `archive-patch` |
  | Spike land | ✅ | `archive-spike` — *refuses* without `spike.outcome` |
  | Stage close | ✅ 1d | — |
  | Project close | ✅ 1e | — |

  The spike lane refuses to land without an outcome; project close will happily
  flip a status with specs in flight, an empty Project-Level Reflection, and four
  signals silently carried. The honest minimum below still stands — but the
  command has earned itself. Ideas to shape:
  - **Name the end states honestly.** `shipped` (delivered the thesis) is not the
    only ending. A project can be **abandoned** (stopped, thesis unproven),
    **superseded** (its thesis moved into another project), or **parked**
    (`on_hold`, may resume). Today `cancelled` flattens all three, which destroys
    the most interesting signal — *why* work stops. Cheap fix: keep the coarse
    enum, add an optional `closed_reason:` (open set, warn-only — the
    `activity` precedent).
  - **A guided `just close-project`** — the ritual, not just a status flip:
    refuse (or loudly warn) while specs are still in flight; stamp `shipped_at`;
    prompt the Project-Level Reflection; force the open **signals** to a
    disposition (the v0.5.18 close ritual already says dispositions happen at
    project close — nothing enforces it); print the final cost + value rollup.
  - **Predicted-vs-realized at close** — the natural home for
    [DEC-009](decisions/DEC-009-business-value-metrics.md)'s loop: compare the
    brief's `value.thesis` / `success_signals` against what actually shipped, and
    record time-to-value (`created_at` → `shipped_at`). Closing is the *only*
    moment the whole wave is knowable.
  - **A "ready to close" calm state** — a project whose specs are all shipped is
    *informational*, not a nag. (standup currently raises a `NEEDS_CLOSE` warning;
    its own lifecycle proposal wants this downgraded. Producer-side support here
    would settle it for every consumer.)
  - **Where the artifacts go** — closing is the natural trigger for the
    client/delivery handover (#11) and ties to the open "where do spec-driven
    artifacts live / repo declutter" question. A closed project probably wants to
    stop cluttering active views without being deleted.
  - **Decision-density × supersession, in the close rollup (new 2026-08-12).**
    A diagnostic from the scale survey. Density alone is noise; the *shape over a
    project's life, paired with the supersession rate*, is the signal. Three
    instances, three distinct shapes: zany **decays** (0.38 → 0.08), bragfile is
    **flat** (~0.6), crustyimg **re-spikes** (0.96 … 0.78, 0.52, 1.00, 0.88).
    crustyimg's sustained ~0.9 reads as thrash until you pair it with
    supersession — 86 DECs, 3 superseded (**3%**). Not an unsettled architecture:
    a genuinely novel domain, well recorded, where decisions hold. The pair is
    the diagnostic:

    |  | Low supersession | High supersession |
    |---|---|---|
    | **High density** | Novel domain, well recorded — *crustyimg* | Thrash: architecture won't settle |
    | **Low density** | Settled, executing — *zany, late* | **Under-recording** — decisions made and lost |

    Bottom-right is the dangerous cell **because it looks calm**: low density
    reads as maturity, and only the supersession rate distinguishes "we settled"
    from "we stopped writing things down." **Do not build a ninth dashboard lens
    for this** — it changes a decision at exactly one moment, project close, as
    input to "is the architecture settling?" It belongs in the close rollup or
    nowhere. *Caveat: n=3 instances across very different domains — a diagnostic
    to try at a close, not an established metric.* Corpus baseline worth
    recording: 159 decisions across the three largest instances, mean confidence
    **0.82–0.85**, no 1.0s, 2–14% supersession — the strongest quality evidence
    the template has produced.
  - **Don't over-build:** the honest minimum is still *(a)* `closed_reason:` and
    *(b)* a close checklist in `AGENTS.md`. What changed 2026-08-12 is only the
    gate — `just close-project` no longer has to wait for a real close to earn
    itself; 28 of them have happened.

- **Requirements traceability matrix (RTM) — the external steal (raised
  2026-08-12).** The strongest idea in the external comparison, and a genuine
  hole here: *"a requirements traceability matrix that a CI linter regenerates on
  every change and that no human is allowed to hand-edit. Traceability runs from
  requirement to spec to code to test, and the build fails if a thread is
  broken."*

  **We have every edge and nothing that walks the graph.** Specs carry
  `references.decisions`, `references.constraints`, `depends_on`, `value_link`;
  DECs carry `affected_scope`, `supersedes`, `project.id`. `validate` checks
  shape, `decisions-audit` checks the DEC graph in isolation, `cost-audit` checks
  cost. **No check asks whether a thread runs end to end.**

  The argument for promoting it: **three existing items on this list collapse
  into one traversal** — harvest signal #7 (reserve→adoption; DEC-024 shipped a
  provenance namespace with zero readers) is *a decision node with no downstream
  edge*; the never-populated `value_link` failure signature is *a spec with no
  edge up to the thesis*; #14's cross-project `depends_on:` is *a missing edge
  type*. Three symptoms, one missing check.

  Phased like DEC-001:
  - **Phase 1 — `decisions/INDEX.md`** (now-tier item 4 above). The decision
    axis, shippable on its own. Not a detour from the RTM; its first slice.
  - **Phase 2 — spec→DEC and spec→constraint closure**, warning on dangling
    refs. Buildable now against existing data.
  - **Phase 3 — spec→code→test. Do not build cold.** The open question to settle
    first: *what is the unit at the code end* — file, module, symbol, test name?
    And does the linter check that a thread **exists**, or that it is
    **current**? Phase 3 is where every RTM in the wild dies; the answer decides
    whether this is a live guard or a decorative matrix.

- **[DEC-011](decisions/DEC-011-roadmap-structure.md) — roadmap structure
  (drafted 2026-07-13). ✅ Phase 1 shipped v0.6.32 (2026-08-12); accepted.**
  Unifies the two roadmaps that already existed and disagreed: the template's
  **derived** roadmap (`just roadmap` = framed + planned stages) and standup's
  **declared** `roadmap:` brief block (pillars + `resume_when`). One
  project-level structure fed by *derive + declare*, a small `kind` set
  (`framed|planned|pillar|goal`), buckets-first horizon (`now|next|later` +
  optional `resume_when`/`target`), emitted via `just roadmap --json` so
  **standup consumes one typed surface** instead of re-parsing briefs. Optional +
  degradable (not every tracked repo is spec-driven). Sits **below** the
  Goals/Plans layer and **beside** DEC-009 (a roadmap `goal` is where a
  `value_metric` attaches).
  - **Shipped:** the block is blessed in both variants' brief scaffold +
    `schema-reference.md`, parsed by `parse_declared_roadmap()` in `_lib.sh`,
    merged into `just roadmap` (human output + a new `declared` bucket in
    `--json`), and warned-on-but-never-gated by `validate`. Open questions 2–5
    are resolved in the DEC — notably **reconciliation**: a declared
    `item: STAGE-004` is not double-listed; the derived row wins on status and
    dates and carries the declared horizon as an annotation.
  - **Phase 2 is standup-side and unstarted.** The "don't build cold" gate was
    half-satisfied: the schema is real and tested, but *nothing consumes it yet*,
    so the producer/consumer fit is still unvalidated. **Wiring standup is the
    next move**, and it is the only thing that can prove this was the right
    shape.

- **Higher-level Goals + Plans layer (raised 2026-07-12).** A layer *above* the
  current `Repo → Project → Stage → Spec → Cycle` hierarchy, for capturing intent
  at portfolio scale (and the parent of [DEC-011](decisions/DEC-011-roadmap-structure.md)'s
  per-project roadmap — a Plan's roadmap is the union of its projects'):
  - **Plan** — groups **multiple Projects** into one initiative/program (a Project
    is a single wave; a Plan is the arc across several).
  - **Goal** — an outcome that can connect to a Plan and/or a Project, **and can
    also stand alone** as lightweight data capture (not every goal needs a home in
    the tree; some are just recorded, no deep modeling required).

  **Deliberately not designing the data model yet** — capture the intent first, let
  real use shape it. Strong tie to
  [DEC-009](decisions/DEC-009-business-value-metrics.md): a Goal is the natural
  parent of DEC-009's outcome-target / `value_metric` (value/outcome lives on a
  Goal), so the two should co-design. Open (later, not now): is a Plan a new
  artifact dir or just metadata linking existing `PROJ-*`? Do Goals live in a
  ledger (like `guidance/signals.yaml`) or as front-matter with links? How do
  goal↔plan↔project connections express — fields vs a link graph? Earn the design
  via a real Plan spanning ≥2 projects.

- **[DEC-009](decisions/DEC-009-business-value-metrics.md) — measurable value +
  time-to-value.** *(Flagged important 2026-07-12.)* Spine (outcome targets on the
  stage) is likely right, but the **metric-derivation aid** must be validated at a
  real project's *frame* (deriving the target metric live), **and it must handle
  goal-less/exploratory projects** with an honest escape hatch — proxy / checkable
  signal / explicit "exploratory" (open question #5). Do not build cold — the
  3-disciplined-project harvest is its validation path, and the Goals layer above
  is its likely parent.

## Deferred — accepted, phase/track pending

- **[DEC-004](decisions/DEC-004-subagent-execution-mode.md) Phase 3** — mechanical
  per-agent `git worktree` isolation. Rule 2 ("one sub-agent, no interleaved tree
  ops") covers the hazard as convention; open Q1 (worth the bash-3.2 complexity?)
  unresolved. **Direction confirmed 2026-07-12: the guidance is right.** Added axis
  — if built, the isolation helper need not be bash; consider a compiled
  single-binary runtime (**Zig** or similar) that owns the worktree lifecycle more
  safely, weighed against the zero-dep portability promise (see DEC-004 open Q1).
- **[DEC-001](decisions/DEC-001-interface-contract.md) Phase 4** — an **MCP server**
  over the `--json` surface (`status` / `dash` / `validate` / cost as typed tools).
  Unblocked by Phase 1; a small, on-brand project that dogfoods the interface contract.
  - **"Turn the template into an app" (raised 2026-07-12) resolves *into* this.** The
    real question is *which layer* you productize. An app that generates the **content**
    (specs / design prose from a brief) is an anti-pattern — it re-wraps the agent and
    hides the disciplined thinking that *is* the value ("coach, don't wrap"). Productize
    the **read / governance / orchestration surface** instead — which is exactly this MCP
    server. Build it *as a real `PROJ` that dogfoods the template* (not speculative
    feature-work); that project also generates the orchestration-cost data the co-design
    item below needs. One move, two payoffs.
- **[DEC-002](decisions/DEC-002-cost-convention.md)** — contribute the `cost.*`
  convention upstream to ContextCore (which has no cost/USD convention). Proposed;
  ready prompt in the DEC appendix.

## Open harvest backlog — buildable now

Ranked by (evidence × leverage) / cost after the 2026-08-12 external-comparison
harvest. **The first four need no new project.** Items 2→3→4 are one chain and
must land in that order.

1. **Run the verify-the-verify study, Tier 0.** *Cost: one command.* The
   template's headline claim — that independent verify and the DEC log are the
   two things that actually prevented errors (2026-07-06 harvest,
   meta-conclusion #1) — is derived from retros written by *the same process
   that produced the artifacts*. **The process grading its own homework, one
   level up.** It has never been measured, and the instrument already exists:
   [`scripts/defects-view.sh`](../scripts/defects-view.sh) reads ship Reflection
   Q4 over `design | build | verify | ship | escaped`, and its own header says
   `escaped` is the number that matters. We wrote the reader and never ran it as
   a study. Run `just dash defects` across all instances; record verify's catch
   rate and the escaped count against the **306-shipped-spec** denominator
   (corrected from 352 — see the baseline note at the top).
   **Pre-commit the branch before running it:** if verify's catch rate is near
   zero, the honest response is to thin the cycle and reinvest in the
   design-time probe (harvest signal #2, N=17 — the highest-frequency efficiency
   lesson we have). Trading an unproven gate for a measured one is a real
   outcome; without a pre-committed branch this is a reassurance exercise.

   > ### ✅ RUN 2026-08-13 — and the pre-committed branch does NOT fire
   >
   > **364 artifacts across all 7 live instances. Zero parseable Q4 answers.**
   >
   > This is not "verify's catch rate is near zero." It is **no catch rate at
   > all**: the measurement apparatus never worked, so the branch above — which
   > was conditioned on a *measured* near-zero — must not be taken. Thinning the
   > cycle on this result would be acting on absence of data as if it were data,
   > the exact error the pre-commitment existed to prevent.
   >
   > Three findings, and the first is much bigger than the study:
   >
   > 1. **Template improvements do not propagate to existing instances, and
   >    nothing ever told us.** `scripts/defects-view.sh` does not exist in a
   >    single instance — the reader was written here and never reached the
   >    repos holding the data. Worse, in bragfile000, crustyimg and zany the
   >    **Q4 question itself is absent from their spec template**: they were
   >    scaffolded before it existed and there is no update path. Those three
   >    have 81 / 117 / 134 filled-in ship reflections — people *did* the
   >    reflection ritual faithfully for years against a template missing the
   >    field. **Every field shipped today (`verify_verdict`, `golden-path`,
   >    `orchestration_cost`'s reader, `decisions-index`) reaches new scaffolds
   >    only.** This is the template's biggest structural gap and it is not on
   >    this roadmap anywhere.
   > 2. **Where the question does exist, it is answered in prose.** standup
   >    (v0.6.12, the newest instance) carries Q4 in 8 specs: 7 are untouched
   >    template text (`— <one word>`), and the 1 real answer is a paragraph
   >    describing a caught defect rather than a vocabulary word — so it is
   >    invisible to the parser. The fixed vocabulary is what makes the field
   >    greppable, and it is not being used.
   > 3. **A claim in the tooling has no data behind it.** `defects-view.sh`'s own
   >    header asserts *"Across the dogfood, EVERY escaped defect was
   >    operational/runtime rather than spec-logic"* — and the Tier-2 capture
   >    below cites that assertion as the reason to weight seeded defects toward
   >    operational. **The corpus contains zero recorded escaped defects.** The
   >    claim may still be true from memory, but it is not evidence, and it is
   >    currently laundering into the roadmap as if it were. This is precisely
   >    the self-grading pattern the study was built to catch — caught, just not
   >    where anyone was looking.
   >
   > **What this changes.** The headline claim is not merely unmeasured, it is
   > **unmeasurable retrospectively** — no amount of analysis recovers data that
   > was never captured. Only forward instrumentation can answer it, which is
   > what `task.verify_verdict` (v0.6.33) now provides. The incoming projects are
   > scaffolded from v0.6.33 and will carry both Q4 and the verdict field, so
   > **re-run this study after their first stage ships** — that is the earliest
   > honest answer. Doing the capture seams before kickoff turned out to be the
   > difference between a study that can happen and one that cannot.

2. ~~**Teach the auditor to see the template's own decisions.**~~ **✅ Shipped
   v0.6.31.** Pointing it at `docs/decisions/` for the first time reported **13
   structural errors immediately** — the fork below, mechanically confirmed
   rather than argued. Original capture: `find_all_decisions()`
   ([`scripts/_lib.sh:1253`](../scripts/_lib.sh)) and `decisions-audit.sh` both
   hardcode `${REPO_ROOT}/decisions`. **This repo has no top-level `decisions/`** —
   its 13 DECs live in `docs/decisions/`, so the decision records governing the
   template itself receive zero structural checking, and `test.sh` only ever
   exercises the auditor against a scaffolded fixture. That blind spot is *how
   the schema fork below survived, and how `type: architecture` stayed
   out-of-enum on 11 files*. Fix: resolve `decisions/`, fall back to
   `docs/decisions/`. One path change — and it goes first because it is causal.

3. ~~**Harvest the DEC schema fork back into the shipped template.**~~ **✅
   Shipped v0.6.31** — `status:` and `deciders:` are now optional shipped fields
   (absent = the old derived behavior, so no migration), and the template's own
   13 DECs were conformed to `created_at:`. **One correction to the finding:**
   `type: architecture` is not a missing enum value, it is a **category error** —
   all 13 already carried `architecture` in `tags:`, so the domain was leaking
   into the insight-*kind* field. Fixed to `type: decision`; the enum is
   unchanged and `schema-reference.md` now says so. The ID-allocation question
   below is **still open**. Original capture: Verified
   13/13: the template's own DECs carry three fields the shipped
   `variants/*/decisions/_template.md` does not — `status: accepted|proposed`,
   `date:` (vs the shipped `created_at:`), and `deciders: [jysf, claude]` — and
   use `type: architecture` on 11 of 13, which **is not in the shipped enum**
   (`decision|analysis|recommendation|observation`). This is a **harvest, not a
   new convention**: it closes a fork between two copies of a convention already
   in use, so the N=3 bar (which exists to stop codifying *thin* evidence) does
   not apply. The honest alternative is equally fine — write down *why* a
   template repo's DECs need `status`/`deciders` and an app repo's don't.
   Neither has happened. Three consequences, in order of interest:
   - **`deciders:` matters most.** The shipped schema records only the agent, so
     it is *structurally incapable* of distinguishing "the human made this call"
     from "the agent made this call." [`PLAYBOOK.md`](PLAYBOOK.md) names four
     calls that must not be delegated and names the failure mode — agent-graded
     homework, where "every artifact is present, every gate is green, and the
     record is fiction." `deciders:` is the cheapest mechanical trace of exactly
     that.
   - **`status:` is the only ADR-shaped gap in the DEC format** (see the rejected
     list below). Add `rejected` and `deprecated` while we're here: today a
     rejected decision has nowhere to live but prose in this file — the
     instance-registration entry, which literally says *"Recorded here so the
     question doesn't get re-asked,"* is a rejected-decision record in exile.
   - **Nothing reads `status` even in our own log.** There is no
     `get_dec_status()` in `_lib.sh`, and `decisions-view.sh` computes status
     purely from `superseded_by` — so `just dash decisions` renders a `proposed`
     DEC as `active`. Wire the reader and lint the combination
     (`status: superseded` XOR `superseded_by: null`).

   **Related, and a real question:** DEC-002, DEC-009 and DEC-011 hold allocated
   IDs while being roadmap proposals with no binding force — a permanent ID with
   no field saying it isn't real. Either give them `status: proposed`, or don't
   burn an ID until acceptance. **Lean:** proposals live *here* in ROADMAP (which
   already carries the evidence and the N-counts a bare `status: proposed` DEC
   would lose); IDs get allocated on acceptance.

4. ~~**`decisions/INDEX.md` — the wall it predicted has arrived.**~~ **✅ Shipped
   v0.6.31** — `just decisions-index` writes a generated table (active and
   superseded split), `--check` is wired into both variants' CI, and
   `affected_scope` was left out as planned. One design addition the capture
   didn't anticipate: the index is **opt-in by existence** — `--check` passes
   quietly until a repo generates one, which is what makes shipping the CI gate
   to every instance safe. Original capture: *(Promoted from
   #14's "watching" list 2026-07-17; re-evidenced 2026-08-12.)* **183 DECs**
   across the corpus, **crustyimg alone at 86** — this list logged crustyimg at
   73, so it grew 18% while the item sat. A generated table, one row per DEC,
   regenerated from front-matter and **never hand-edited**. Ship it in the
   template as `scripts/decisions-index.sh` + `just decisions-index`, not as a
   per-instance script. Small, because every reader already exists in `_lib.sh`
   (`get_dec_id`, `get_dec_title`, `get_dec_confidence`, `get_dec_superseded_by`,
   `get_dec_affected_scope`); the only new one is `get_dec_status`, **which item
   3 requires anyway — so 3 lands first.** What it buys at this scale: cold-read
   collapse (answering "what has already been decided about X?" opens 86 files in
   crustyimg today — ties `context-coldread-cost`), and supersession chains
   visible at a glance (crustyimg's 3-of-86 reversal rate currently takes a bash
   loop to compute). **Discipline:** regenerate + `git diff --exit-code` in CI,
   so a stale index fails the build — that is the difference between a derived
   artifact and a decorative one.
   - **Scope caution.** The survey also wants `confidence`, `project` and
     `affected_scope` columns. Take the first two; **leave `affected_scope` out**
     — multi-valued path globs make an unreadable table at 86 rows, and "which
     paths are governed, which aren't" is a *different view* (that is RTM Phase 2
     below, not the index).

5. ~~**Capture the verify verdict in front-matter.**~~ **✅ Shipped v0.6.33** —
   `task.verify_verdict` (`approved | punch-list | rejected` — Prompt 4's own
   three verdicts, so nothing new is asked of the verifier), surfaced by
   `just dash defects` as the second axis alongside the catch-stage
   distribution. **One correction to the capture:** stamping "at
   `advance-cycle … ship`" would have recorded every approval and **silently
   dropped every rejection** — precisely the number worth having. It stamps when
   a spec *leaves* verify in **either** direction, inferring from the
   destination, with `--verdict rejected` for the one call the destination
   cannot make.

6. ~~**Lint unattributed decisions.**~~ **✅ Shipped v0.6.31** — `decisions-audit`
   now warns (advisory) on a DEC with no `project.id`, suppressed in a repo with
   no `projects/` to attribute to. It did fall out of items 2–3 nearly free, as
   predicted. The survey's 10 orphans (7 crustyimg, 3 bragfile, 0 zany) had to be
   excluded from every per-project computation — the concrete cost of a broken
   traceability edge, and a miniature of the RTM below.

**Still optional, unchanged:** **#9** — a per-language "known gotchas" appendix
the build prompt links *(2026-08-10: reframed as one half of the **golden paths +
gotchas** candidate below — the gap is not the appendix, it is that the template
has no instance-to-instance transfer mechanism at all)*; **#10** — a
scheduled-advisory CI convention (cron gate for vuln-DB drift).

~~**#8** — `roadmap` surface *planned-but-unframed* stages (parse the brief's
`## Stage Plan` checkboxes).~~ **✅ Shipped v0.6.14** — `just roadmap` renders a
`planned` bucket (human + `--json`) from the brief's Stage Plan, de-duped against
framed `STAGE-*.md` files. Scoped to `roadmap` only (not `backlog`): backlog's
"stage backlog" is un-promoted *specs* inside a framed stage, a different layer
from un-framed *stages*.

> **What changed 2026-08-12:** this section went from one non-speculative
> now-tier build to six, and the top item is a **measurement, not a build**.
> **Items 2, 3, 4 and 6 shipped the same day (v0.6.31)** — the whole decisions
> chain, which turned out to be one causal thread: the auditor couldn't see the
> template's own decision log, so a schema fork grew there unnoticed, so the
> index that fork blocked couldn't be built. Fixing the blind spot surfaced the
> fork within seconds of running it.
>
> **What's left here: item 1 (the verify study) and item 5 (the verify verdict
> field) — and item 1 is now the most valuable thing on this page.** Everything
> shipped today made the *record* better. None of it tested whether the process
> works. Next leverage is still *using* the template on real projects, but the
> case for measuring before building more keeps getting stronger.

## Co-design with the next project(s)

These are shaped by real usage — start them *on* a live project, not in the abstract:

> **1–3 `claude-plus-agents` projects are starting (2026-08-12).** This closes a
> standing variant blind spot: **uw is dead, and it was the only plus-agents
> instance — never harvested.** Every lesson in this file was learned on
> `claude-only`. Decide what to measure *before the first one starts*:
> first-run friction happens once, and it is the one observation these projects
> can produce that no later session can recover.

- **Verify-the-verify, Tiers 2 and 3 (raised 2026-08-12).** Tier 0 (above) is a
  one-command count; these two need a live bed.
  - **Tier 2 — seeded-defect injection.** Pick 8–10 shipped specs, inject one
    plausible defect each into the build output, run a cold verify blind, score
    catch rate per defect class. Cheap, because the work order already exists:
    306 shipped specs with written acceptance criteria and prescribed failing
    tests. This is mutation testing pointed at a process.
    > **Corrected 2026-08-13.** This item said to "draw the defects from our own
    > escape taxonomy… weight toward operational/runtime, since
    > `defects-view.sh` records that every escaped defect was operational rather
    > than spec-logic." **There is no such taxonomy.** The Tier-0 run found zero
    > recorded escaped defects in the entire corpus; that claim lives only in a
    > header comment in `defects-view.sh` and had been cited here as if it were
    > data. Until Tier 0 is re-run against instrumented projects, **invent the
    > defect classes deliberately and say so** — a made-up taxonomy you label as
    > made-up is honest; a remembered one dressed as evidence is not.
  - **Tier 3 — the independence test.** The claim is not that verify works, it
    is that **independence** is what makes it work. [`PLAYBOOK.md`](PLAYBOOK.md)
    states flatly that "build and verify in one session produces a review that
    finds nothing" — *an untested assertion presented as fact in our own
    documentation*. Run verify twice on 3–5 specs (build-session continuation vs
    cold) and diff the findings. plus-agents gives a third arm free, where the
    implementer is genuinely a different tool rather than a different session.
  - Both design constraints transfer from the **build bake-off** below — it is
    the same harness pointed at the other cycle: **the scorer must not be a
    verifier**, and **record the misses, not just the catches.**

- **DEC-009's derivation aid** (above) — the frame-time metric prompt + escape hatch.
- **#4 — contract-tests-as-guards kit** — a starter kit + named constraints
  (contrast-aa, state-not-color-only, compositor-only-keyframes) that turn subjective
  quality (motion/perf/a11y) into CI guards. A creative/visual project is its natural
  first user.
- **#11 — client-handover artifact + user-vs-contributor docs split → a future DEC**
  (the DEC-011 number went to roadmap structure; client-handover is unwritten and
  will take the next free number when it earns one).
  A deliverable handover to the person you built it for (distinct from the internal
  agent↔agent `HANDOFF-*`). Needs a real external delivery to shape it.
- **Orchestration + framing cost attribution (raised 2026-07-12).** The cost model
  meters only where there's a boundary — the **sub-agent** (`build`/`verify` tokens come
  back in the Agent result; see the comment block in `scripts/cost-audit.sh`). Everything
  in the **main loop** — `frame`/`design`/`ship` plus all cross-spec orchestration — is
  nullable today, and *pre-spec* framing (deciding the stage breakdown before any spec
  exists) has no home at all (no `stage.cost`). Net effect: recorded cost is
  systematically **under-counted**, which quietly corrupts DEC-009's predicted-vs-realized
  loop. Two attribution boundaries:
  - **(a) session boundary** — a dedicated framing/orchestration session, one meter read
    at the end → a coarse `overhead` cost line. Portable; degrades to `null` where there's
    no meter (DEC-005). Manual but honest.
  - **(b) sub-task boundary** — push the work into sub-agents (the *same* trick
    `build`/`verify` already use) and define **orchestration = session total −
    Σ(metered sub-agents)** — the top-level residual you can never sub-task to zero.
    More elegant, but Claude-Code-specific; gate it like `metering_source`.

  Lean: coarse `overhead` bucket first, per-sub-task residual later. **Don't build cold —
  this is N=1.** It co-designs naturally *on the MCP-server project above* (orchestrating
  sub-agents to build it produces exactly this framing+orchestration spend).

  > **Update 2026-08-10 — the handback materially unblocked (b).** Approach (b) was
  > stuck because the residual is only computable when every delegated cycle is
  > metered, and non-Claude agents weren't. The **`handback:` block** (v0.6.28) now
  > yields Σ(delegated cost) for *any* agent, so **orchestration = session total −
  > Σ(handbacks)** is computable outside Claude Code for the first time. Two gaps
  > remain, and the second is the interesting one:
  > 1. Reading the orchestrator's own session total (manual `/cost` — fine).
  > 2. ~~**Pre-spec framing still has nowhere to live.**~~ **✅ Partly closed
  >    v0.6.33 — and the diagnosis above was wrong.** The slot was not missing:
  >    `orchestration_cost:` already existed on the stage template. **Nothing
  >    read it** — the only references anywhere were tests asserting the slot was
  >    *present*. That is the reserved-but-unwired pattern (harvest signal #7)
  >    reappearing in the very thread that named it, and it is why the field was
  >    never filled: a field nothing reads is a field nobody fills. `just roadmap`
  >    now sums and surfaces it per stage, human + `--json`, and stays outside
  >    `cost-audit` (a test asserts the gate's verdict is unchanged). Stage grain
  >    only, as argued: **per-spec attribution of orchestration is a split you
  >    cannot observe**, so any number is invented.
  >
  > What remains is **collection, not schema** — nobody has recorded a real
  > orchestrator session total yet. The incoming projects are the bed: read
  > `/cost` at stage close, paste one entry, let real numbers shape the tooling.

  Touches
  [DEC-002](decisions/DEC-002-cost-convention.md) (cost convention),
  [DEC-004](decisions/DEC-004-subagent-execution-mode.md) (delegated-exec cost
  attribution), [DEC-009](decisions/DEC-009-business-value-metrics.md)
  (predicted-vs-realized). Future home: its own DEC once it recurs.

## Candidate conventions — unearned, watching

- **Productization axiom (from the "turn it into an app" discussion, 2026-07-12).**
  When productizing *any* layer, ask *which layer*: **the discipline is the value, not
  the artifact.** Tooling that exposes the **contract** (read / governance / orchestration,
  `--json`, MCP) amplifies adoption; tooling that generates the **content / judgment**
  dissolves the discipline it's meant to enforce. A rule of thumb for every future
  "should we automate this?" call. See DEC-001 Phase 4 above.
- **Repo-level vision + idea parking lot.** The template captures direction well
  *once* an idea is a `PROJ-NNN` brief, but has no home for pre-commitment vision or
  a candidate-idea backlog. A workspace-level `ideas.md` is the **live experiment**;
  fold a convention into the scaffold only if it earns N≥2 across projects.
- **Where do the spec-driven artifacts live? — repo-declutter / artifact storage
  (raised 2026-07-17).** Across many waves, shipped **specs already self-clean** to git
  history (a dogfood survey found bragfile at 6 waves keeps 1 spec file on disk, zany 0),
  but the **DEC log and cold-read corpus grow monotonically** (crustyimg 73 DECs). The
  open, unanswered question: must governance/spec artifacts always live *in* the app repo?
  This collides head-on with the deliberate **"the repo is the app"** axiom
  ([blog](blog/2026-06-02-the-repo-is-the-app.md)) + literal-artifact-as-spec. Two honest
  resolutions, kept strictly separate:
  - **(a) within the axiom — better in-repo cleanup.** Archive completed waves, the
    `decisions/INDEX.md` fix above, sub-template extraction to relieve cold-read cost
    (ties `context-coldread-cost`). This is the template's lane; earn each piece from a
    real project hitting the wall (crustyimg's 73 DECs is one such wall).
  - **(b) departing from the axiom — an external artifact store** that projects into the
    repo on demand. This does **NOT** belong in the template (it breaks the axiom the
    template is built on); it is the natural **second pillar of the helper-app**
    (`~/PSeven/ideas.md` #6 — "own the artifacts so they don't clutter the app repo").
    Do not fold (b) into the template.
- **#14 — scale-tier growth** (informational, from bragfile's scale-recs): AGENTS.md
  cold-read cost at ~40KB, constraint-linting vs honor-system, cross-project
  `depends_on:`. A "what breaks past this scale" list, not current defects. *(The
  `decisions/INDEX.md`-past-~25-DECs item graduated to the buildable backlog above,
  2026-07-17 — crustyimg's 73 DECs made it real.)*

- **Build bake-off — N-version build, and what it really buys (raised 2026-08-10).**
  Run the *same* spec through two or more build agents, verify each independently,
  then pick. The template accidentally already emits the artifact a fair bake-off
  needs: a self-contained work order with testable acceptance criteria and
  prescribed failing tests. Two design constraints, both learned from the existing
  lanes:
  - **Verify must not select.** The moment a verify session picks a winner it is
    invested in that winner and stops being a cold reviewer for it. Verify each
    build independently against the spec; make selection a *separate* step.
  - **Record the losers.** Two builds = two branches, two `## Build Completion`
    blocks, two cost entries, one discard. Throwing the loser away discards the
    actual prize.

  **The prize is not the better build — it's learning which agent is better at
  what.** `spec.agent.tier_map` is currently set once, by judgement, with zero
  evidence behind it; a handful of bake-offs turns it into a *measured* setting
  that compounds across every later spec. It is N× build cost, so it is a **stakes
  tier, not a default** (same shape as the patch lane, pointed the other way).
  Open: where a bake-off is recorded (N `claimed_by` holders on one spec breaks
  the single-holder lease); whether the loser's cost counts toward the spec.
  **Do not build tooling first** — run it by hand on 3–5 high-stakes specs and
  record which agent won and why. grebe (two external agents already configured)
  is the natural first bed.

- **Golden paths + per-language gotchas — the sideways gap (raised 2026-08-10).**
  Two faces of one thing: **gotchas are the anti-pattern list, golden paths the
  pattern list**, and both are per-stack knowledge. What makes this more than
  roadmap #9: `guidance/toolchain-brief.md` is **per-repo and instance-local**,
  but this knowledge wants to be **cross-repo** — the whole point is carrying what
  bragfile learned into grebe. **The template has no instance-to-instance transfer
  mechanism at all.** Everything flows *up* through harvests into the template and
  *down* into new scaffolds; there is no sideways. That absence is the real
  finding.
  - ~~Capture seam that already fits~~ **✅ Shipped v0.6.33 — the seam, not the
    mechanism.** `guidance/signals.yaml` types were all *problems* (`lesson` /
    `process-debt` / `product` / `risk`), with **no type for "this worked so well
    another project should copy it wholesale."** `golden-path` now exists,
    dispositioned at project close, inheriting the ritual unchanged.
    **Be clear about what this does and doesn't do:** it makes the knowledge
    *capturable*, so a close stops discarding it. It does **not** move anything
    sideways between repos — the transfer mechanism is still missing, and that
    remains the real finding. The incoming projects are the first chance to see
    whether captured golden paths are worth transferring at all.
  - **The bar applies harder here, not softer.** A wrong paved road is worse than
    no road, because people follow it. Capture candidates aggressively; promote
    almost nothing. N=3 or it stays a preference.
  - Open: does a golden path live in the template (shipped to every scaffold), in
    a cross-instance store, or only as a harvest note? Ties to the artifact-storage
    question above and to the `instances.md` registry.

- **"One owner per field. No fact is authored in two places." (raised
  2026-08-12) — needs a deliberate call, not a default.** The external source's
  central rule; we do not state it. Three violations surfaced without looking:
  the DEC schema forked between `docs/decisions/` and `variants/*/decisions/`
  (now-tier item 3); two roadmaps that, in DEC-011's own words, "already exist
  and disagree"; and every variant file existing twice (two `AGENTS.md`, two full
  `docs/` trees, two `guidance/` trees).

  **The counter-argument is strong enough to lead with:** all three are
  duplication-by-copy — *one finding wearing three hats*, which makes it N=1, not
  N=3. And the third is not a violation at all: variant duplication is a
  *deliberate* choice (each scaffold must be self-contained after `init` consumes
  `variants/`), so counting it as accidental duplication misreads the design.
  What is left is the DEC schema fork, which is already being fixed on its own
  merits. **Lean: don't codify the axiom.** Note we already apply it correctly
  where it matters — `CLAUDE.md` is a pointer to `AGENTS.md` with a suggested
  symlink — so the instinct exists; it simply doesn't generalize as cleanly as
  the rule implies.

- **Steering-doc split: product / tech / structure (raised 2026-08-12) — below
  bar.** The external source splits steering docs three ways, read first by every
  task. `AGENTS.md` mixes all three — repo purpose (product), stack and
  conventions (tech), work hierarchy and layout (structure) — and signal #14
  already flags its cold-read cost via the standing `context-coldread-cost`
  signal. A build session needs tech + structure and can skip product. **That is
  the shape of the fix when it bites; it has not bitten yet.**

- **Two-tier QA as a named routing principle (raised 2026-08-12) — below bar,
  but the cheapest item here.** The external source separates *mechanical
  linting* (hard-fails: missing sections, broken traceability) from *judgment
  review* ("is this statement actually observable, does this design truly realize
  the spec"). **We have both and have never named the split** — which shows in
  the verify checklist, where "acceptance criteria met? tests pass?" sits beside
  "build reflection answered honestly?" Naming it gives a routing rule for every
  future check: *can a linter decide it → script; needs judgment → prompt.* Costs
  a paragraph; earns itself the first time it stops a judgment call from being
  written as a gate.

- **A completeness check inside verify (raised 2026-08-12) — below bar.** Their
  "completeness checker" is a *role* we should not adopt (see the rejected list),
  but the question is one verify never asks. **"What is missing?" is a different
  question from "is this correct?"** A line in the verify prompt, not an agent.

- **Extend the design-time probe to visual/interaction surfaces (raised
  2026-08-12) — below bar.** Their UX artifact is "explored as runnable
  components and approved before it is built" — which is `AGENTS.md` §12's
  design-time probe, pointed at pixels. Ties to harvest signal #4
  (contract-tests-as-guards, from zany inventing CI guards for motion/contrast/
  perf). Not a new artifact; a one-line extension of an existing rule, when a
  visual project turns up.

- **Instance registration has no trigger (raised 2026-08-10) — parked, owner
  sees no value now.** Nothing registers a new instance in
  [`instances.md`](harvests/instances.md): no script or recipe touches it, and
  every row currently there was added retroactively in one bulk pass. The
  structural reason it will keep happening is that **registration belongs in the
  template repo, but the event that should trigger it — scaffolding a project —
  happens in a different repo**, where nothing prompts you. Cheap fix if it ever
  earns its way in: `just init` / `just fresh-start` already knows the repo name,
  variant and template version at exactly the right moment, so it could print a
  paste-ready registry row (it can't edit the template repo, but it can hand you
  the line).

  **Deliberately not built.** Asked 2026-08-10; the owner's answer was "I don't
  see the value right now." Record that as evidence, not as a deferral — the
  Discipline column in `instances.md` was *removed* on exactly this signal
  (nobody could fill it, so it was noise). A registry maintained in bulk every
  few months may simply be good enough for a handful of instances; the prompt
  only earns itself if a harvest is ever actually blocked by a missing row.

- **Clone-per-agent — asked and answered (2026-08-10): no.** When several coding
  agents work one repo, isolate with **`git worktree`, not clones**. The rule:
  **the code can be isolated; the record must not be.** A clone forks the memory —
  a spec's cost lands in one copy and its `DEC-*` in another, `signals.yaml`
  diverges — which destroys the one thing the template exists to protect. Recorded
  here so the question doesn't get re-asked; it strengthens the case for
  [DEC-004](decisions/DEC-004-subagent-execution-mode.md) Phase 3 rather than
  competing with it. If the real need is *different instructions per agent*, that
  is a `guidance/agents/<name>.md` the handoff links (sibling of the toolchain
  brief), not a repo fork.

## Parked with a note — ContextCore alignment (2026-08-13)

Two threads, deliberately deferred. Recorded so they aren't rediscovered:

- **[`CONTEXTCORE_ALIGNMENT.md`](../variants/claude-only/docs/CONTEXTCORE_ALIGNMENT.md)
  is stale.** It documents the namespaces we borrow as of an early read.
  A fetch on 2026-08-13 found ContextCore's registry now carries **eight** —
  `task` · `sprint` · `project` · `business` · `agent` · `insight` · `guidance` ·
  `handoff`. DEC-002's own verification note (2026-06-18) recorded "only
  agent/project/sprint/task", so **it grew underneath us and nothing checks**.
  Also: their `insight.type` includes **`blocker`**, which our enum lacks, while
  `recommendation`/`observation` may be ours rather than theirs — so the two
  enums have quietly drifted in *both* directions. Re-read and refresh the
  alignment doc before relying on it again.
- **`status:` and `deciders:` are the real upstream candidates**, not
  `architecture`. Neither appears in ContextCore's insight namespace, and
  `deciders` solves a *general* problem for any agent-observability convention:
  you cannot otherwise tell a human decision from an agent one. This is
  [DEC-002](decisions/DEC-002-cost-convention.md)'s sibling and can reuse the
  drafting prompt in its appendix. **Do not propose `architecture`** — we
  concluded it is a category error (see now-tier item 3); proposing it would
  export a mistake upstream.

## Now-tier, added 2026-08-13 — nobody knows which instance has what

> **⚠ Reframed 2026-08-13 (same day), and the first framing was wrong.** This
> item was written as *"instances freeze at scaffold time; there is no update
> path."* **False** — the owner has repeatedly hand-ported scripts between
> repos with Claude Code (`lifetime-report` went template → crustyimg → zany),
> and that works fine. Corrected by a script-by-script scan:
>
> | Instance | Template scripts present | Notably missing |
> |---|---|---|
> | standup | 29 / 45 | `defects-view`, `calibration`, `decisions-index` |
> | zany-animal-slots | 19 / 45 | `defects-view`, `calibration`, `validate`-era views |
> | crustyimg | 17 / 45 | `defects-view`, `dash`, `validate` |
> | rspeed | 13 / 45 | `lifetime-report`, `cost-audit`, `validate` |
> | bragfile000 | 10 / 45 | **every gate** — `validate`, `cost-audit`, `decisions-audit`, `test` |
>
> **The real finding is the selection bias, not the absence of a mechanism.**
> You port what you need *today*. That is rational every single time, and in
> aggregate it systematically strands the **cross-cutting analytical tooling** —
> the readers whose entire value is being present in every repo at once, which
> is precisely the value that never feels urgent in any one repo. `defects-view`
> is in **zero** instances for exactly this reason, and that is why the Tier-0
> study had nothing to read.
>
> This is **reserve-then-strand at the distribution layer**: not a field with no
> reader, but a reader with no distribution. Same pathology, one level up.
>
> It also explains bragfile000's zero cost data. It has no `cost-audit`, no
> `validate`, no `test` — the discipline did not lapse; nothing was ever there
> to enforce it. Its 72 shipped specs are uncosted structurally, not carelessly.
>
> **So the fix is much cheaper than an update mechanism.** Hand-porting already
> works. What is missing is *knowing what is where*: a capability diff — which
> instances lack which scripts — turns an invisible gap into a two-line answer.
> Everything below still stands, but rung one is now clearly the whole job.

## Now-tier, added 2026-08-13 — no update path to existing instances (superseded framing, kept for the record)

**Raised by the Tier-0 run, which could not proceed because of it.** An instance
is scaffolded once and then frozen: `just init` copies a variant to the root and
deletes `variants/`, and **nothing afterwards ever pulls template improvements
in**. Evidence, from a disk scan of all 7 live instances:

| Instance | Template version | Has `defects-view.sh`? | Has Q4 in its spec template? |
|---|---|---|---|
| bragfile000 | no `VERSION` file at all | no | **no** |
| crustyimg | no `VERSION` file at all | no | **no** |
| zany-animal-slots | no `VERSION` file at all | no | **no** |
| standup | v0.6.12 | no | yes |
| *(template today)* | v0.6.33 | yes | yes |

Three instances predate the `VERSION` file entirely. The newest is 21 patch
versions behind. **Every improvement in this file's shipped-log reaches new
scaffolds only** — which quietly halves the value of template work, because the
repos with the most accumulated data are the ones that can never use it.

This is not the "instance-to-instance transfer" gap (that is sideways). This is
the **downward** path failing after the first copy, which is more surprising:
the template's whole delivery model assumes scaffolding is a one-time event and
never says what happens next.

Shape of a fix, cheapest first — **do not build past the first one yet**:
- **Say the version out loud.** `just status` (or `template-version`) warns when
  the instance's `VERSION` is behind the template it came from. Requires knowing
  the upstream version, so probably a recorded remote + a manual check. Cheap,
  honest, and turns an invisible gap into a visible one.
- **A documented manual update recipe** — which files are safe to re-copy
  (`scripts/`, `justfile`) versus which are yours forever (`AGENTS.md`,
  `guidance/`, `projects/`). The split already exists in the `app.just`
  convention; it has just never been written down for updates.
- **`just template-update`** — later, and only if the manual recipe proves it
  earns automation. It has to survive local edits to shipped files, which is the
  hard part and the reason not to start here.

Open, and a real question: **is freezing actually wrong?** A frozen instance is
reproducible and never breaks under you. The cost is only visible when the
template gains a *reader* for data the instance already holds — which is exactly
what happened here. That may be a narrow enough case to solve with the warning
alone.

## Explicitly rejected — recorded so it isn't re-asked

From the 2026-08-12 external comparison. *(This section is itself the evidence
for now-tier item 3's `status: rejected` — a rejected decision currently has
nowhere to live but prose in this file.)*

- **Adopting ADRs alongside DECs.** DEC is a structural *superset*: every Nygard
  element, plus `confidence` (wired to three gates), plus `affected_scope` +
  `--changed` (which solves the actual ADR failure mode — records nobody
  re-reads), plus supersession-graph linting. The only missing element is
  `status`, which item 3 adds. **Two decision logs is strictly worse than one.**
- **Immutable decision records.** The external source's own history is the
  argument against: reversing one platform choice took ADR-005, 008 *and* 009.
  Our supersession model does it in one record and tells you how confident
  anyone was.
- **Role-based agent personas** (product owner / spec author / tech lead / task
  generator / reviewer / completeness checker). **Independence is bought with the
  session boundary, not the persona** — personas sharing a context window do not
  produce a cold reader. `claude-plus-agents` is the honest version of this idea
  and is about to get its first real evidence.
- **A six-artifact pre-code pipeline.** Their day-13 / 41,000-words-before-first-code
  against our same-day-or-next-day first ship across three projects. They name the
  cost themselves — five domains fully specified with zero code, and the decay
  risk stated outright: *"a spec written months before its code can be quietly
  wrong by the time anyone implements it."* [`PLAYBOOK.md`](PLAYBOOK.md) Phase 1
  already forbids this.

## Hygiene — not roadmap items, just do them

- ~~**Mark uw dead in `instances.md`.**~~ **✅ Done 2026-08-12.** It read as an
  unharvested blind spot when it is an abandoned project. Same `closed_reason:`
  gap as the close-project item, one level up: *"never harvested"* and
  *"abandoned, nothing to harvest"* are different facts the registry could not
  express.
- **`git pull` the `maintenance_of_experiments/bragfile000` checkout.** Last
  fetched 2026-08-08, strictly an ancestor of live `origin/main` — stale, not
  diverged. Worth recording explicitly: the "a clone forks the memory" hazard
  argued above is sound but **has not occurred** — two clones, zero forks. That
  is evidence the discipline is holding, not evidence against it.
- **Audit the eight advisory view scripts.** 43 scripts / 28 recipes / 8
  `*-view.sh` for a corpus of 8 instances. The *gating* audits (`validate`,
  `cost-audit`, `decisions-audit`) are well earned; the open question is only the
  advisory views — **which has ever changed a decision?** Keep those; let the
  rest be a `--json` query. The **productization axiom cuts here**: a
  proliferation of read surfaces is a soft form of wrapping.
- **`docs/` is 47,750 words against 25,858 shipped per variant.** Meta-commentary
  about the process is **~1.8× the process we ship**. Not an action — a number
  worth having in front of us before the next doc lands. (Whole repo: 124,044
  words / 117 markdown files.)
- ~~**Re-confirm the dead `frame` cycle at the new scale.**~~ **✅ Checked
  2026-08-13 — and the signal is measuring the wrong thing.** Result: **1 of 359**
  specs currently at `cycle: frame` (signal #12 recorded 0/122). So the raw
  number holds at ~0%.
  **But do not act on it.** `task.cycle` is a **current-state** field, not a
  history: a spec that passed through `frame` on its way to `ship` reads `ship`
  today. "0 specs in frame" therefore means *"nothing is parked in frame right
  now"* — exactly what a healthy, fast-moving cycle looks like — and cannot
  support the conclusion that frame is unused. `just frame-stage` literally
  creates specs at `cycle: frame`, so the cycle is mechanically in use.
  **Absorbing `frame` into `design` on this evidence would be the same error the
  Tier-0 study just caught** one level up: a metric that cannot answer the
  question it is being cited for. To answer it properly you would need cycle
  *transitions*, which nothing records. Park it, or add transition logging first.

---

*Shipped work is in [`CHANGELOG.md`](../CHANGELOG.md); the decision record is in
[`docs/decisions/`](decisions/); projects built with the template are catalogued in
[`PROJECTS.md`](../PROJECTS.md).*
