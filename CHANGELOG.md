# Changelog

All notable changes to this template. One entry per fix; newest at top.

## 2026-08-14 — optional reviewer briefs (v0.6.39)

**New `guidance/agents/`** — a small set of optional review lenses you can point
a session at. Nothing loads them automatically; an empty directory is a fine
steady state.

- `reviewer-correctness.md` — does it do what the spec says
- `reviewer-design.md` — is this the right shape for *this* codebase
- `qa-close.md` — the judgment pass before a stage or project close
- `README.md` — how to wire one, and the rule that makes them work

**The rule, stated up front in the README: a brief is only worth anything in a
session that did not do the work.** Handed to the session that wrote the code it
produces a costume, not a reviewer — the context is already contaminated by
every choice under review, so it agrees with itself in a different voice.
Independence is bought with the **session boundary**; the brief only decides
what the reviewer looks at.

This is why `docs/ROADMAP.md` rejects *role-based personas* while shipping
these: the rejection is aimed at personas as costume inside one context, not at
lenses applied across a boundary. The rejection is amended rather than
contradicted.

**Correctness and design are split on purpose.** "Does it work" and "is it
right" are different questions, and one reviewer asked both will answer the easy
one. Each brief states what it **ignores** as explicitly as what it looks for,
and carries a **narrow licence to reject** — a reviewer that cannot say no is a
formatter.

`qa-close.md` fills the one place the template has a gate with no judgment
behind it: `close-project` refuses on mechanical grounds — reflection filled,
signals dispositioned, thesis scored — but cannot tell whether any of those
answers are *true*. Its bar is deliberately narrow: a project is allowed to end
badly, it is not allowed to end dishonestly.

**Deliberately absent: a stakeholder brief.** Simulating the person you built it
for is the failure mode, not the fix — a proxy stakeholder is still you, and
`value_realized.thesis_held` scored by a simulation of your own customer is
worth less than an honest `too-early`.

Cost note: each extra lens is another full pass. Treat these as a **stakes
tier**, not a default.

Tests assert **reachability**, not presence — the briefs are pointed at from
`AGENTS.md` and the handoff template, because a brief nothing links is a brief
nobody opens.

## 2026-08-13 — know which instance is missing what (v0.6.38)

**New `scripts/instances-diff.sh`** — point it at instance checkouts and it
reports what each is missing.

This started as a different, wrong finding: *"instances freeze at scaffold time
and there is no update path."* False. Hand-porting between repos works and has
been done repeatedly — `lifetime-report` went template → crustyimg → zany.

The scan that corrected it found partial, uneven coverage instead: standup
29/45 scripts, zany 19, crustyimg 17, rspeed 13, bragfile000 10.

**The real finding is the selection bias.** You port what you need *today*.
Rational every single time — and in aggregate it strands exactly one class of
tooling: the cross-cutting analytical readers, whose entire value is being
present in every repo at once, which is precisely the value that never feels
urgent in any one repo. `defects-view.sh` reached **zero** instances for this
reason, and that is why the first Tier-0 verify study had nothing to read. It
is **reserve-then-strand at the distribution layer** — not a field with no
reader, but a reader with no distribution.

It also explains bragfile000's zero cost data: it has no `cost-audit`, no
`validate`, no `test`. Its 72 shipped specs are uncosted **structurally**, not
carelessly.

So the fix needed no mechanism — only visibility. The tool calls the cross-repo
readers out separately, because "you are missing 28 scripts" is noise while
"you are missing the four that make a corpus-wide study possible" is a decision.

## 2026-08-13 — rule 1 generalized past build output (v0.6.37)

DEC-004's rule 1 said *reconcile over self-report*: never advance a cycle on a
sub-agent's word — check git and disk. Correct, and scoped too narrowly.

**The general form, now stated in both variants' `AGENTS.md`: verify any claim
you are about to act on, against the thing it describes.** Build output is only
the most obvious case. A number in a roadmap, a count in a harvest, a "we
already tried that" in a decision record — each is a self-report by a past
session, and each rots silently.

Earned the hard way in one session, N=4 before lunch: an "every escaped defect
was operational" claim with zero recorded escaped defects behind it; a
"never-populated `value_link`" that was populated in 239 specs; an
8-instance/208-DEC corpus baseline inflated by a stale checkout and inherited by
three successive analyses; and a "dead frame cycle" measured by a field that
only reports current state, where deleting the stage would have been the wrong
call for the right-looking reason.

The point is that **none of these needed cleverness to catch.** They needed
someone to run the count. That is what makes the rule worth stating: it is
cheap, mechanical, and requires no judgement — only the habit of looking before
acting.

## 2026-08-13 — the template stops predicting without scoring (v0.6.36)

**New `value_realized:` block on the project brief** — `thesis_held`
(`yes | partly | no | too-early`), `signals_observed`, `evidence`, `notes`.

`value:` was always the **prediction**, written at the moment you knew least.
Nothing anywhere recorded the **outcome**. `close-project` printed the thesis
back and told you to compare it in prose. So across 318 specs and 6 instances,
*"was the idea any good?"* has never been answerable — not because nobody asked,
but because **nobody's answer was ever a field.** A prediction with no recorded
outcome is a wish, not a hypothesis.

`just close-project` now **refuses** to close a project that claims `shipped`
while `thesis_held` is null. The refusal is **conditional on the ending you
claim**, the same shape as the in-flight-specs rule:

- no thesis → never asked
- `closed_reason: abandoned` / `parked` → not asked to score a thesis you
  stopped testing
- claims `shipped` with a thesis → must answer

**`too-early` is a first-class answer.** Some theses need months of use, and
offering that hatch is what stops the field collecting fabricated `yes`es —
which would be worse than leaving it null.

It also warns when `thesis_held` is answered but `evidence` is empty: the
verdict without the observation behind it is the part that rots first.

This is DEC-009's predicted-vs-realized loop, at the only moment it is
knowable. It deliberately does **not** compute a score — the judgement is the
point, and it is the human's.

## 2026-08-13 — ship the reader with the field, and a corrected corpus (v0.6.35)

**New coding convention, in both variants' `AGENTS.md` §11 and in
`CONTRIBUTING.md`: a field ships with its reader, or it doesn't ship.**

This is the template's most reliably recurring defect — **N=5**, well past the
N=3 codification bar, and every instance was found by looking rather than by
anything failing:

1. the stage `orchestration_cost` slot shipped with no reader and stayed empty;
2. the ship-reflection defect-catch question shipped, was answered nowhere, and
   its reader was written months later and reached **no instance**;
3. DEC `status:` was used in this repo's own decisions while `decisions-view`
   computed status from `superseded_by` instead;
4. a provenance namespace shipped with zero readers (harvest signal #7);
5. `value_link` is populated in 239 specs and links to nothing parseable.

The failure is quiet, which is why it repeats: **an unread field is
indistinguishable from a field whose data hasn't arrived.** Nobody fills in what
nothing displays, so it stays empty — and being empty, nobody notices it is also
unread. Unlike most defects this one cannot be fixed retroactively; there is no
back-filling a year of measurements. `CONTRIBUTING.md` adds the corollary that
caught three of the five: **assert on the reader in `scripts/test.sh`, not on
the field's presence.**

**Corpus baseline corrected — it had been inflated for months.**
`bragfile-report` was registered as a separate full-tier instance. It is a
**stale checkout of `bragfile000`**: same remote, same first three projects, 195
commits into the same 296-commit history, verified a strict ancestor with no
divergence. Every figure that counted it double-counted **41 specs / 40 shipped
/ 25 DECs**, including the 2026-07-06 harvest and the 2026-08-12 external
comparison, each inheriting the previous number.

**Actual live corpus: 6 instances (+1 dead) · 318 specs · 306 shipped · 183
DECs**, mean DEC confidence ~0.80. Figures corrected throughout `ROADMAP.md`;
registry fixed in `harvests/instances.md`.

Worth separating: **the practice was never at fault.** The clone never diverged,
so the "a clone forks the memory" hazard still has zero observed instances. The
registry was at fault — and a stale checkout is indistinguishable from an
instance by directory shape alone, which is why three analyses in a row counted
it. Remaining unverified checkouts are now flagged as suspect.

## 2026-08-13 — close gets a command, and it refuses (v0.6.34)

**`just close-project PROJ-NNN`.** Close was never "thin" — Prompt 1e is
seven-plus steps and its signal-disposition walk is one of the more rigorous
rituals here. The gap was an **asymmetry**: close was the only major ritual with
no command behind it. `archive-spike` refuses to land without a `spike.outcome`,
while a project close would happily flip a status with specs in flight, an empty
reflection, and signals silently carried.

Three **hard refusals**, on the spike lane's principle — *refuse when the
missing thing is the exact failure the ritual exists to prevent; warn otherwise*:

1. **Specs still in flight — but only when you claim `shipped`.** In-flight specs
   contradict "shipped" and corrupt every rollup computed over them. Under
   `closed_reason: abandoned` they are *expected*, so the refusal lifts. That
   makes `closed_reason` load-bearing rather than a label.
2. **An empty Project-Level Reflection** — the direct analogue of
   `spike.outcome`. It is the artifact close exists to produce.
3. **Signals still awaiting a project-close disposition.** The v0.5.18 ritual
   said this happens at close and nothing enforced it. `defer-with-trigger` is a
   legal disposition, so the refusal always has an honest escape hatch.

Everything else warns. A null `value.thesis` does **not** refuse — that would
punish exploratory projects, which the value model explicitly allows.

**New `closed_reason:` on the brief** — `shipped | abandoned | superseded |
parked`, open set, warn-only. `status` stays the coarse machine state; this
carries the half that `cancelled` flattens, which is *why* work stopped.

The rollup prints cost, orchestration spend as a share of total, time-to-value
(`created_at` → close), and **decision density paired with supersession rate** —
never density alone, because density alone is noise. The pair reads as: high
density + low supersession = novel domain well recorded; low density + high
supersession = **under-recording**, the dangerous cell precisely because it looks
calm.

**Deliberately not built:** `close-stage`. The evidence is 28 *project* closes;
stage close inherits the shared machinery free once it earns itself. **Also not
built:** a dashboard lens for density — it changes a decision at exactly one
moment, and that moment is close.

**Also in this release:**

- **`just decisions-index --json`** — the typed surface, per DEC-001. It carries
  `affected_scope`, which the human table deliberately omits: width is only a
  problem in a table, and it is the edge a future traceability walk needs.
- **`decisions-audit` warns when a spec references a `proposed` DEC.** The first
  spec→DEC edge anything walks. Building against a proposal is legitimate — the
  failure is the proposal never being accepted while a spec quietly becomes its
  only evidence. Advisory, never a gate.
- **Bug: `affected_scope` leaked its YAML quotes.** Globs are routinely written
  quoted, and the reader emitted `"scripts/**"` *including* the quotes — so any
  `--json` consumer matched nothing. Fixed at the reader, which also corrects
  `just dash decisions --json`.

## 2026-08-12 — three capture seams that can't be back-filled (v0.6.33)

Three fields, one shared property: **none of them can be added retroactively.**
A spec that ships without a verdict, a project that closes without recording
what worked, a stage framed without its spend captured — those are permanent
holes. With several projects starting, the seams have to exist first.

**`task.verify_verdict` — `approved | punch-list | rejected`.** The template's
headline claim is that independent verify is one of the two things that
prevented errors, and `PLAYBOOK.md` lists *"verify never rejects anything"* as a
failure signature — but nothing could compute it, because the ✅/⚠/❌ Prompt 4
returns lived only in prose. The three values **are** Prompt 4's three verdicts,
so nothing new is asked of the verifier.

`just advance-cycle` stamps it whenever a spec **leaves** verify, in either
direction, inferring from the destination (`→ ship` = approved, `→ build` =
punch-list) and printing what it recorded. Stamping only on the way to ship
would have recorded every approval and silently dropped every rejection —
exactly the number worth having. `--verdict rejected` makes the one call the
destination can't. Advisory in `validate`; read by `just dash defects`, which
now pairs *where a defect was caught* with *whether verify ever pushed back*.

**`golden-path` — a fifth signal type.** Every existing type in `signals.yaml`
records a **problem** (`lesson`/`process-debt`/`product`/`risk`). There was no
way to record *"this worked so well another project should copy it wholesale"*,
so every close discarded it. That matters because the template has **no
instance-to-instance transfer mechanism at all** — knowledge flows *up* through
harvests and *down* into new scaffolds, never sideways. Dispositioned at project
close, inheriting the existing ritual unchanged. **Its bar is harder than a
lesson's, not softer:** a wrong paved road is worse than no road, because people
follow it. Capture freely, promote almost nothing.

**`orchestration_cost` finally has a reader.** The slot already existed on the
stage template — and *nothing read it*, which is the reserved-but-unwired
pattern (a field nothing reads is a field nobody fills). `just roadmap` now sums
and surfaces it per stage, in human output and `--json`. Stage grain only: the
roadmap is explicit that splitting orchestration across specs is a division you
cannot observe, so any per-spec number is invented. It stays outside
`cost-audit` — a test asserts the gate's verdict is unchanged by it.

**Bug fixed on the way:** `update_frontmatter_scalar` only ever *replaced*, so
stamping a newly-added field onto an artifact created before that field existed
silently no-opped — while the command printed "recorded". Every live instance
has in-flight specs predating `verify_verdict`, so this would have reported
data it never wrote. New `upsert_frontmatter_scalar` inserts the key when the
block lacks it.

## 2026-08-12 — `just roadmap` gets its declared half, DEC-011 Phase 1 (v0.6.32)

A project's roadmap has two halves, and until now the template only knew one of
them. `just roadmap` **derives** what is shaped or named — framed `STAGE-*.md`
files plus `## Stage Plan` checkboxes. It had no way to express forward intent
that **isn't a stage yet**, so the `standup` portfolio tracker invented its own
`roadmap:` front-matter block and parsed briefs directly. Two roadmaps, one
never blessed, the consumer re-implementing the producer's schema.

**`roadmap:` is now a blessed, optional brief block** — `item`, plus optional
`kind` (`pillar | goal`; `framed`/`planned` stay inferred from files),
`horizon` (`now | next | later`), `resume_when` trigger, and `target` date.
`just roadmap` merges it with the derived half and `--json` emits a new
`declared` bucket, so a tracker consumes **one typed surface** instead of
re-parsing briefs.

**Reconciliation:** a declared `item: STAGE-004` naming a real stage is *not*
listed twice. The derived record wins on status and dates — it comes from the
file, which is ground truth — and the declared `horizon`/`resume_when` ride
along on that row. Only intent with no stage behind it lands in `declared`.
(This resolves DEC-011's open question 2; questions 3, 4 and 5 are resolved in
the DEC as well.)

Buckets are the primary horizon axis on purpose: a dated roadmap you can't hit
is worse than a bucketed one you can. Both vocabularies are **suggested sets,
not enums** — `just validate` warns on an unrecognized `kind`/`horizon` and
never fails, following the `project.activity` precedent. The block is optional
everywhere and its absence is the normal case.

**Phase 2 is standup-side and unstarted.** Until standup actually consumes
`just roadmap --json`, the producer half is validated only by its own tests.

## 2026-08-12 — the decision log gets a lifecycle, an owner, and an index (v0.6.31)

Three changes that turned out to be one chain. The first is the cause of the
other two.

**`decisions-audit` could not see the decisions that govern the template.**
`find_all_decisions()` and `decisions-audit.sh` both hardcoded
`${REPO_ROOT}/decisions`. This repo has no top-level `decisions/` — its own 13
DECs live in `docs/decisions/`, and `test.sh` only ever ran the auditor against
a scaffolded fixture. So the template's own decision records had **never been
linted, once**. Both now resolve `decisions/` and fall back to
`docs/decisions/` (root wins when both exist, so nothing changes for an
instance). Pointing the auditor at them for the first time reported **13
structural errors** immediately.

**Those errors were a schema fork.** All 13 used `date:` where the shipped
schema says `created_at:`, and 11 used `type: architecture`, which is not in the
`insight.type` enum. Both are now fixed in this repo's DECs — and the second one
is a category error worth naming: `architecture` is a **tag, not a type**. Every
one of those 13 decisions *already* carried `architecture` in `tags:`. The enum
describes the kind of insight; the subject belongs in tags. Documented in
`schema-reference.md` so it doesn't recur.

**The fork ran the other way too, and that half is a genuine harvest.** The
template's own DECs carried two fields the shipped `_template.md` did not, and
both are now shipped — optional, with the old behavior as the default:

- **`status:`** — `proposed | accepted | rejected | deprecated | superseded`.
  Omit it and tooling derives `active`/`superseded` from `superseded_by` exactly
  as before, so an existing log reads unchanged and no migration is needed.
  `rejected` is the value that earns the field: with nowhere to record a
  rejection, the same option gets re-litigated every few months. Unrecognized
  values are advisory (the `project.activity` precedent), but a `status` that
  **contradicts** `superseded_by` is a structural error — the two fields would
  be disagreeing about the same fact.
- **`deciders:`** — who made the call. `agent.id` only records which agent was
  in the room, so the shipped schema was *structurally incapable* of separating
  "the human decided" from "the agent decided" — the exact distinction
  `PLAYBOOK.md` relies on when it names the calls that must not be delegated.

**New: `just decisions-index`** regenerates `decisions/INDEX.md` — one row per
DEC (id · title · confidence · status · project · supersedes), with active and
superseded split into separate tables because the first question a cold reader
asks is "what is in force." At the dogfood corpus's scale (208 decisions; 86 in
one repo) answering "what have we already decided about X?" meant opening every
file.

`affected_scope` is deliberately **not** a column: multi-valued path globs are
unreadable at 86 rows, and "which paths are governed" is a different view.

The index is **opt-in by existence**. `--check` writes nothing and passes
quietly on a repo that has never generated one; once committed, it fails when
the index falls behind. Both variants' CI now run that check — safe to ship,
because a repo that doesn't want an index never sees it fail.

**Also:** `decisions-audit` now warns (advisory) on a decision with no
`project.id` — unattributed decisions are invisible to every per-project view,
and the dogfood survey found 10 of them. Suppressed where attribution isn't
possible at all (a repo with no `projects/`).

## 2026-08-10 — init stops leaking the template's own docs into your app (v0.6.30)

`just init` copied the variant over the root and deleted `variants/`, but never
removed the **template's own** root docs — so every scaffolded instance inherited
a 100 KB+ CHANGELOG of the template's version history, its "projects built with
this template" showcase, its CONTRIBUTING guide (bash 3.2, variant parity), and
stale migration notes. None of it describes the app being built.

That was not just noise. The patch lane and the release spec both tell an
instance to *"add a CHANGELOG entry under `[Unreleased] → Fixed`"* — against a
file that was the template's history and contained **no `[Unreleased]` section
at all**. An app's first patch either appended to the template's changelog or
quietly went unrecorded.

- **`scripts/scaffold-clean.sh`**, run by `init`: removes `PROJECTS.md`,
  `CONTRIBUTING.md`, `KNOWN_LIMITATIONS.md`, `MIGRATION_TO_REPORTS_AND_COSTS.md`,
  and seeds a fresh app `CHANGELOG.md` **with the `## [Unreleased]` section the
  patch lane writes to**. It prints what it removed — no silent deletion.
- **Kept:** `VERSION` (template provenance — an instance reports the version it
  was scaffolded from) and `LICENSE`, which is never auto-deleted because that
  could leave a repo unlicensed; it's flagged for review instead.
- **Fixed in passing:** the VERSION↔CHANGELOG drift guards were reading the
  *scaffolded* changelog rather than the template's. They guard the template's
  own consistency, so they now target it explicitly.

Root entries in a fresh instance: **26 → 22**, and the changelog goes from
~106 KB of someone else's history to 349 bytes of yours.

Credit: surfaced by an external review flagging root-level clutter. The specific
files it named (`.repo-context.yaml`, `.variant`) are staying — they're dotfiles
and the app's own metadata — but checking the claim found this underneath.

## 2026-08-10 — defect-escape reader, plus-agents coverage, orchestration slot (v0.6.29)

Three gaps closed before kicking off a two-agent project, plus the decision
record the handback shipped without ([DEC-013](docs/decisions/DEC-013-delegated-cost-handback.md)).

- **`just dash defects`** — the reader for `defect-catch-stage`. Every shipped
  spec already answered *"where was the worst defect caught?"* and **nothing read
  it**; the retro that asked for the field said its value is "only visible in a
  cross-project view", so it had shipped half-finished (the reserved-but-not-wired
  pattern, harvest signal #7). Renders the distribution with `escaped` highlighted
  — the class that got through design, build *and* verify. `--json`; scans all
  projects by default. The parser deliberately ignores the template's own
  vocabulary prose (backticked) so an unanswered spec is never counted as data.
- **The `claude-plus-agents` variant now has test coverage — it previously had
  NONE.** The suite only ever scaffolded `claude-only`, so handoffs (and the new
  handback) were entirely unverified. A second scratch repo now scaffolds the
  other variant and exercises the delegated-cost path end to end: one handoff per
  cycle, `to_agent` from `tier_map.<cycle>`, `handback-sync` refusing an
  un-returned or token-less handback, transcription into `cost.sessions`,
  idempotence on re-run, and **`cost-audit` passing on a spec costed purely from
  handbacks** — the whole point of the path.
- **`orchestration_cost:` on the stage template.** Framing a stage and deciding
  its spec breakdown happen before any spec exists, so that spend had nowhere to
  live and recorded cost was structurally under-counted. **The orchestrator fills
  it** at stage close from its own session total — the human never hand-counts.
  Stage grain only: splitting orchestration across specs invents a number you
  cannot observe. Warn-only, no gate, no view yet — capture first (roadmap:
  orchestration + framing cost attribution, now N=2-ready).
- **Fixed:** the shipped example stage had drifted from the template it
  demonstrates; a test now pins them together.

## 2026-08-10 — the handback: delegated cycles report their own cost (v0.6.28)

Groundwork for running build and verify on two different non-Claude agents. The
blocker was cost: `cost-captured-per-cycle` requires a real `tokens_total` on
build and verify, and an orchestrator has no meter for an agent it doesn't host.

- **`handback:` block on every handoff.** The executing agent fills it before
  reporting done — status, **a real `tokens_total`**, USD, duration, branch/PR.
  The prompt says explicitly: if your platform has no meter, set null and say why;
  **do not invent a number** — a fabricated cost looks real in every downstream
  rollup.
- **`just handback-sync SPEC-NNN`** transcribes reported cost into the spec's
  `cost.sessions`. Idempotent (stamps `synced_at`), and **exits 1 naming any
  handoff that hasn't handed back cleanly**. The orchestrator never estimates a
  delegated cycle.
- **`just new-handoff SPEC-NNN build|verify`** — ONE handoff per delegated
  *cycle*, so a build/verify split gets one each. `to_agent` comes from
  `tier_map.<cycle>`; a `verify` handoff carries the cold-review contract.
- **`--json` on the gates** (DEC-001 §2): `just validate --json` and
  `just cost-audit --json` now emit structured findings — which artifact failed
  which check — instead of prose an agent has to screen-scrape. Both still exit 1.
- **`just dash spikes`** — the spike lane's lens (the house rule is *add a lens,
  not a new command*); flags a spike landed with no outcome.

## 2026-08-09 — the spike lane: bounded exploration, and vibe-coding sessions (v0.6.27)

The template modelled **committed** work well — a project is a wave, a stage a
chunk, a spec a task, a patch a fix. Every one of those assumes you already know
what you're building. It had no home for the phase *before* that. `just new-spike`
adds one.

- **A third lane: `spike → land`.** Two modes, one discipline.
  `mode: question` is a timeboxed investigation (code is evidence);
  **`mode: build` is a vibe-coding session** (code is the deliverable). During
  the spike there is **no spec, no failing tests, and no `DEC-*` required** —
  `test-before-implementation` explicitly does not apply (AGENTS.md §12), because
  the speed is the value.
- **No verify step, deliberately.** The patch lane keeps its independent verify
  because it fixes *known* behavior against a *known* expectation. A spike has
  neither acceptance criteria nor a spec, so a verify would have nothing to check
  and would degrade into theater — eroding the real verify in the other lanes.
  The **timebox** and the **mandatory land step** replace it.
- **`land` is the whole point.** Answer the question, emit `DEC-*` for the
  load-bearing choices the exploration already made (honest confidence; not
  archaeology), and set `spike.outcome` ∈ `answered | inconclusive | graduated |
  discarded`. Hitting the timebox with no answer is `inconclusive` — a real
  result. Extending a timebox twice means it isn't a spike, it's an unframed
  project.
- **The graduation contract** (vibe-coding → real work) is a five-item checklist
  in the artifact: `.repo-context.yaml`, `AGENTS.md`, `guidance/toolchain-brief.md`,
  retroactive `DEC-*` for load-bearing choices only, and a brief framed on **what
  comes next**. Explicitly *not* a sixth: **do not retro-write specs for code that
  already works.**
- **Repo-level, not project-scoped.** Spikes live at `spikes/SPIKE-NNN-<slug>.md`
  with their own repo-wide sequence; `project.id` is optional and back-linked at
  land. This is the decisive difference from a patch — a spike often *precedes*
  any project, which is the case that motivated the lane.
- **One mechanical tooth.** `just validate` requires `spike.question` +
  `spike.timebox`, and **fails a spike at `cycle: land` with a null
  `spike.outcome`**; `just archive-spike` refuses to file an un-landed spike. A
  spike still at `cycle: spike` passes untouched — mid-exploration is a legitimate
  state. `just cost-audit` does **not** gate spikes (cost advisory).
- **Also:** `just dash spikes` (the lens — the house rule is *add a lens, not a new
  command*; it flags an un-landed spike); `just status` gets a repo-level Spikes
  section + `spikes[]` in `--json`; `advance-cycle` accepts `spike|land`; `spike` becomes an official
  `project.activity` value (AGENTS.md had reserved it as the example extension);
  Prompts **0 / 0b** (Spike, Land the Spike) in both variants;
  `docs/PLAYBOOK.md` + `docs/USAGE.md` cover the lane.
- **Fixed in passing:** `new-spike` uses `__TOKEN__` placeholders rather than
  prose ones. A prose placeholder containing the `sed` delimiter (`2h | 1d`)
  silently fails to substitute — the v5.8 bug class — and the `justfile` passes
  each parameter quoted rather than through a `*ARGS` splat, which would splice
  a `|` into the recipe shell as a pipe. Both are covered by regression tests.

**Evidence level, stated plainly:** this is **N=1** (`standup`), below the
template's own N=3-same / N=2-paired-opposing codification bar, and shipped on
the owner's explicit direction as a deliberate exception to the deliberate-lag
discipline. [DEC-012](docs/decisions/DEC-012-spike-lane.md) records it as one,
with a revisit trigger at the next harvest: if spikes are being *landed*, it
earned its place; if they're created and abandoned un-landed, the lane is failing
at its only job and should be cut back to prose.

## 2026-07-27 — expected vs actual: the estimation feedback loop (v0.6.26)

The template recorded what work *cost* but never what you *predicted*, so it
could never tell you the one useful thing: **are your estimates any good?**
`just calibration` closes that loop. Warn-only, permanently — the value is
learning whether you systematically under- or over-estimate, not hitting a
number.

- **Sizes are a t-shirt scale.** `task.complexity` widens from `S|M|L` to
  `XS|S|M|L|XL|XXL` and is now explicitly the **EXPECTED** size, set at design.
  Existing `S|M|L` values stay valid. `XL`/`XXL` is itself a finding — a spec
  that size is almost certainly a stage; split it.
- **`task.complexity_actual`** — the size stamped at **ship**, the only moment
  it's knowable. Optional; an unrecognized value is advisory in `validate`,
  never a gate. `archive-spec` nudges when one is missing and archives anyway.
- **`cost.tokens_estimate`** — an optional token prediction at design, sitting
  beside the actual `cost.totals.tokens_total` the spec already records. At
  2-space indent so it can't collide with the session/totals readers.
- **`just calibration [--all|--active|PROJ-NNN] [--json]`** — per shipped spec:
  expected vs actual size (verdict `under`/`over`/`on`), estimated vs actual
  tokens (±25% counts as on target), and the drift totals.
- **The observed token band per expected size** — min/median/max actual tokens
  for every `M`, every `L`, and so on, measured from *this repo's* shipped
  specs. This is the honest version of "map t-shirt sizes to token bands": once
  n is big enough to trust, the size you assign *is* a rough token estimate,
  derived rather than guessed. No invented lookup table ships with the template.
- `get_task_scalar` / `get_spec_complexity_actual` / `get_tokens_estimate` /
  `size_rank` in `_lib.sh`; `get_spec_complexity` moved there from `backlog.sh`.
  AGENTS gains a **During design** section (set the expectation) in both
  variants, and ship stamps the actual. Schema reference + USAGE updated.

Ties to [DEC-009](docs/decisions/DEC-009-business-value-metrics.md)'s
predicted-vs-realized loop and to the orchestration cost-gap thread: both need a
prediction on record before a comparison means anything.

## 2026-07-26 — frame a stage into outline specs (v0.6.25)

`just frame-stage STAGE-NNN` batch-promotes every `- [ ] (not yet written)` line
in a stage's `## Spec Backlog` into a real spec at `cycle: frame`, and rewrites
the backlog line in place to `- [ ] SPEC-NNN (frame) — …`. This closes the
fan-out loop opened by v0.6.23: a **planned** stage becomes a **dispatchable,
dependency-aware batch**, because `depends_on:` can only point at a spec that
exists. Stable IDs are the whole product here.

The fidelity line is deliberate: an outline captures **scope and dependencies,
not approach**. Design stays just-in-time — a stage framed as ten pre-designed
specs is ten guesses that go stale before you reach spec four. Outlines
therefore land at `frame`, not `design`, and carry a banner saying so.

- **`--outline` on `new-spec.sh`** (the primitive; `frame-stage` is the batch
  wrapper, mirroring DEC-006's `--release` + `just new-release-spec`). Parks the
  spec at `cycle: frame` and inserts the scope-not-approach banner under the H1.
  Mutually exclusive with `--release` — a release cut is never an outline.
- **`--complexity X` on `new-spec.sh`** — an optional `[S]`/`[M]`/`[L]` tag on
  the backlog line is carried onto `task.complexity` instead of being dropped in
  promotion. (Recognizes the wider `XS|S|M|L|XL|XXL` set; a bracketed `[CI]` in a
  summary is left alone.)
- **`just frame-stage STAGE-NNN [PROJ-NNN] [--dry-run]`** — pre-flights the whole
  batch (slug collisions, existing specs) *before* creating anything, so a
  failure can't leave specs on disk with an un-rewritten backlog; `--dry-run`
  shows the ID assignment and writes nothing; re-running is a no-op, not a
  duplicate batch; `**Count:**` is recomputed exactly as `archive-spec` does.
- **`extract_unpromoted_bullets` moved to `_lib.sh`** and hardened to require a
  checkbox bullet — the section's own prose names `(not yet written)`, and the
  selftest caught it being promoted into a spec. `just backlog` (the view) and
  `just frame-stage` (the promotion) now read the backlog through one parser, so
  what you see listed is exactly what gets promoted.
- **`AGENTS.md` (both variants)** — `frame` was documented as optional and
  unused (0 of 100+ specs). It now has a real job *in batch*; the cycle-model
  section states the fidelity line and the stage-close self-audit.
- **Stage template (both variants)** — the Spec Backlog section documents the
  ritual, and Stage-Level Reflection asks **"how many outlines survived
  unchanged?"** so framing-ahead earns its keep on evidence instead of faith.

## 2026-07-18 — release notes, assembled not re-authored (v0.6.24)

Release notes are now part of the process at every altitude. The governing
principle is **capture once, assemble many**: the outward outcome is already
recorded per spec (Reflection Q5, v0.6.19), so notes are *derived* rather than
written a third time — writing them again would guarantee drift.

Three audiences, three artifacts, deliberately not collapsed: **release notes**
(whoever USES the thing) · **CHANGELOG** (the record of what changed) ·
**Reflection Q1–Q3** (internal learning).

### Fixed

- **Reflection Q5 had no answer slot.** Q1–Q4 each end with a `— <answer>` line;
  Q5 (added v0.6.19) did not — so despite promising "a real, greppable result,"
  there was nowhere structured to record the outcome and it could not be
  assembled. Both variants now carry `— <answer | none>`. This was the
  prerequisite for everything below.

### Added

- **`just release-notes`** (`scripts/release-notes.sh`) — assembles outward-facing
  notes from each shipped spec's Q5, grouped by stage. Scope: active project, a
  `STAGE-NNN`, a `PROJ-NNN`, or `--all`. `--prompt` wraps the same notes in a
  synthesis ask (the `lifetime-report` data/prompt split); `--json` for consumers.
- **`get_spec_outcome`** in `_lib.sh` — reads Q5, returning empty for both the
  unfilled placeholder and an explicit `none`, so infrastructure specs stay out of
  notes. The count of no-outcome specs is still **reported**, so silence is
  visible rather than lost.
- **`## Release Notes`** in `release-spec.md` (both variants) — the one *authored*
  home, at the release altitude, where a headline, grouping, and breaking-change
  callouts get added. Seeded by `just release-notes`, not written from scratch.

### Tests

- `scripts/test.sh` +8 checks: the Q5 slot exists; the reader reads a real outcome
  and treats `none` as none; notes assemble, exclude `none` specs, and report the
  silent count; `--prompt` emits the ask; the release-spec section ships.

## 2026-07-18 — `depends_on` + the ready set + spec claims (v0.6.23)

The fan-out data foundation. "Can these two specs run in parallel?" was a
judgment call every agent re-made and could get wrong. Now it's **computed**: with
dependencies declared, the parallel batch is just the ready set. Steps 1–2 of the
fan-out ladder, plus the atomic-claim primitive (the idea worth stealing from
[beads](https://github.com/gastownhall/beads), which converged on the same
`dep` / `ready` / `--claim` triple independently).

### Added

- **`depends_on:`** on specs (both variants) — *blocking* dependencies: specs that
  must SHIP before this one. Distinct from `references.related_specs`
  (informational). `get_spec_depends_on` in `_lib.sh` reads both inline
  (`[SPEC-002, SPEC-003]`) and block-list YAML.
- **`claimed_by:`** on specs (both variants) + **`just claim SPEC-NNN <who>`** /
  **`just unclaim SPEC-NNN`** — the fan-out lease, so two agents don't grab the
  same spec. Claiming a spec someone else holds **fails** (`--force` steals it).
  Advisory by design: the hard lock for parallel work is the worktree/branch an
  agent creates; this is the declarative marker that projects back into the spec.
- **`just ready`** (`scripts/ready.sh`) — the READY SET: in-flight specs whose
  dependencies have all shipped and that nobody holds. Also lists what each
  blocked spec waits on. `--all` / `PROJ-NNN` scopes, plus `--json` (state,
  `blocked_by[]`, `claimed_by`, totals) so an orchestrator can dispatch from it.
- **`set_fm_scalar`** + **`spec_is_shipped`** helpers in `_lib.sh`. `set_fm_scalar`
  replaces a front-matter key in place (inserting before the closing `---` only if
  absent), so claiming never reorders fields and diffs stay minimal.

### Changed

- **`just validate`** reports a `depends_on` ref with no matching spec as
  **advisory** — never a gate failure. Naming a dependency before it's written is
  normal while framing a stage (same open-set posture as `project.activity`).
- **`docs/schema-reference.md`** (both variants) documents both new fields.

### Tests

- `scripts/test.sh` +9 checks: the inline-list reader; a dependency blocks;
  shipping it unblocks; claim writes / excludes from ready / refuses a second
  claimant; unclaim clears; a dangling ref stays advisory and is surfaced.

## 2026-07-18 — `specs-by-stage` shows each spec's title (v0.6.22)

`just specs-by-stage` rendered specs as bare IDs — `SPEC-002  shipped  2026-07-18
M  $1.15 174k` — with no way to tell what a spec *was* without opening it. The
inconsistency was visible in the same output: stages render their full slug
(`STAGE-001-core-substrate`) while specs were truncated to the ID. Reported from
the skillport-lint dogfood.

### Added

- **`get_spec_title`** in `scripts/_lib.sh` — reads the human title from a spec's
  first `# SPEC-NNN: <title>` heading. Separator-tolerant (`:` / em-dash /
  hyphen) using the same one-`sub`-per-separator idiom as `parse_stage_plan`
  (never a bracket class — BSD-awk multibyte offsets). Falls back to the filename
  slug (`SPEC-001-my-thing.md` → `my thing`) so a missing or odd heading still
  renders something useful.

### Changed

- **`just specs-by-stage`** renders the title as the **last** column, untruncated
  — the ID / date / complexity / cost columns stay aligned for scanning, and long
  titles are never cut (the truncation complaint already logged against
  `backlog`). Legend updated to `… · cost (usd · tokens) · title`.
- **`specs-by-stage --json`** gains a `title` field per spec. Additive, so it's a
  backward-compatible contract change — downstream consumers (standup) get spec
  titles for free.

### Tests

- `scripts/test.sh` asserts the legend documents the column, the human render
  carries a real title, `--json` emits the field *and* its value, and
  `get_spec_title` degrades to the filename slug when the heading is absent.

## 2026-07-18 — `just init` offers a fresh git history + provenance (v0.6.21)

A repo cloned from the template used to carry the template's entire commit
history into every new project, and the reset was a manual `rm -rf .git && git
init` you had to remember (README Option B). Now `just init` handles it and
records where you came from.

### Added

- **`scripts/fresh-history.sh`** + **`just fresh-start`** — guarded, one-time git
  history reset for a NEW project. Captures provenance (template version + source
  commit + origin) and resolves the committer identity *before* anything
  destructive (so it can't wipe history then fail to commit), then `rm -rf .git`,
  re-inits on `main`, and makes an initial commit whose message records
  `Scaffolded from spec-driven-template <version> (<sha>)`.

### Changed

- **`just init`** now asks *"Start with a fresh git history? [y/N]"* after
  scaffolding (default **No**, EOF-safe so piped/non-interactive init is
  unaffected). On yes it runs the reset with provenance; on no it points at
  `just fresh-start` for later. GitHub's "Use this template" already gives a clean
  history, so that path answers No.
- **README** "Using this template" updated: Option B drops the manual `rm -rf .git`
  step — `just init` offers it.

### Tests

- `scripts/test.sh` runs `fresh-history.sh` against an isolated throwaway repo and
  asserts history collapses to a single commit on `main` with template-version +
  origin provenance stamped in the message. (Init's default-No path is already
  covered by the piped-init checks.)

## 2026-07-17 — spec Outputs: require each flag's default (v0.6.20)

Small completeness fix to the spec template. Specs routinely named a flag's
*accepted values* but not its *default*, so build had to guess — an avoidable
ambiguity. Both variants' `spec.md` `## Outputs` now carry a **New flags /
options** bullet requiring each flag's accepted values **and its default**.

### Changed

- **`spec.md` (both variants)** — added a **New flags / options** line to
  `## Outputs`: "each flag's accepted values **and its default** — an unstated
  default makes build guess." (claude-plus-agents says "the implementer guess",
  matching its voice.)

## 2026-07-17 — outward outcome capture: spec Q5 + release Cut record (v0.6.19)

Adds the framework's missing *outward-facing* outcome sensor — the before→after a
piece of work created for a user — at the two altitudes where work completes: the
spec (a capability) and the release (a shipped version). Every existing feedback
channel (`signals.yaml`, the ship reflection questions) points inward at the
process; none captured the user-visible result, so it kept getting reinvented
ad-hoc and dying. One outward sensor per altitude, each with a forcing `none`
escape modeled on the greppable Q4 defect-escape question.

### Added

- **Spec reflection Q5 — "What can a user do now that they couldn't before?"**
  (`## Reflection (Ship)`, both variants' `spec.md`). One sentence, before →
  after; `none` is a real, greppable result, not a blank. It's a *confirmation*
  task — `## Context` is the before and `## Goal` is the after, both already
  written — and the line a downstream work-log's `impact` field is transcribed
  from.
- **Release Cut record** — the release's outward outcome, filled **after Phase 2**
  by whoever runs the cut (the reflection is written in Phase 1, before the
  publish exists, so this needs its own post-publish slot). Placed per each
  variant's structure: a final bullet in `## Build Completion` (claude-only), a
  new `## Cut Record` section before `## Reflection` (claude-plus-agents). The
  release reflection questions are unchanged.

Establishes a convention for where a scaffolded repo puts its own commands. The
root `justfile` is **template-managed** — it ships with the scaffold and is meant
to be updated when you pull template improvements. Project-specific recipes
(`build`, `dev`, `test`, `deploy`, …) now live in a separate **`app.just`** that
the root justfile imports, so a template update never conflicts with your app
commands. Completes the split AGENTS.md §6 already gestured at ("these are the
APP's commands; for template/workflow commands see `justfile`").

### Added

- **`app.just`** at the repo root — a project-owned recipe file with REPLACE
  stubs (`install`/`dev`/`build`/`test`/`lint`/`typecheck`) mirroring the
  AGENTS.md §6 command block. Fill it in; each stub prints a reminder and exits
  non-zero until you do. Runs as normal `just` recipes (`just build`, …).

### Changed

- **Root `justfile`** carries `import? 'app.just'` (optional import — a fresh
  clone works before the file exists) plus a header banner stating it is
  template-managed and app recipes belong in `app.just`. Template recipes win on
  a name clash (the importing file overrides the import).
- **AGENTS.md §6 + §7** (both variants) point at `app.just`: §6 says to wire the
  app commands there so they're runnable, §7 lists it in the directory tree.

### Tests

- `scripts/test.sh` asserts `app.just` ships and survives `init`, the root
  justfile imports it, and `just --list` surfaces both a template recipe
  (`status`) and an app recipe (`build`) — proving the import resolves without
  clobbering template commands.

## 2026-07-12 — lifetime report: `data` / `save` modes (v0.6.17)

Splits `lifetime-report` into three commands, harvested from the zany-animal-slots
dogfood instance (which refined the template's own `lifetime-report.sh`). The old
script interleaved the raw whole-repo history with the LLM synthesis wrapper in one
output; the refactor separates the two axes — the deterministic *data* and the
narrative *ask* — and adds a timestamped save.

### Added

- **`just lifetime-data`** — the whole-repo Lifetime Data Report (projects, stages,
  specs, decisions, releases, git span) as a **self-contained, LLM-free** document,
  led by a deterministic "Lifetime at a glance" count block. Read it directly; no
  Claude session needed.
- **`just lifetime-save`** — writes the data report to
  `reports/lifetime/YYYY-MM-DD-HHMMSS.md`, timestamped to the second so repeated
  runs never overwrite.

### Changed

- **`scripts/lifetime-report.sh`** takes a mode: `data` (default) or `prompt`. Both
  share one `emit_data`. **`just lifetime-report` is unchanged** — it now calls
  `prompt` mode and prints the same synthesis prompt as before.
- **Latent robustness fix** (carried back from zany): the git-span `first`/`last`
  assignments now guard the `git log | head` pipe with `|| true`, so a long history
  that SIGPIPEs `git` can't abort the script (under `set -euo pipefail`) before the
  status / specs-by-stage aggregates.

### Tests

- `scripts/test.sh` asserts `lifetime-data` prints the self-contained header + the
  at-a-glance block and carries **no** LLM synthesis wrapper, and that
  `lifetime-save` writes a data report whose filename is timestamped to the second.

## 2026-07-12 — `on_hold` added to project status (v0.6.16)

Adds `on_hold` to the coarse project `status` enum:
`proposed | active | on_hold | shipped | cancelled`. Stage status already had
it; this brings project status into line so a wave can be explicitly paused
without being mislabeled `proposed` or `cancelled`. Additive — `get_project_status`
already documented `on_hold`, and `get_active_project` already treats any
non-`active` status (now including `on_hold`) as not-the-active-wave, so no logic
changed: this is enum + docs alignment.

### Changed

- **Brief scaffold** (`projects/_templates/project-brief.md`, both variants),
  **`docs/schema-reference.md`**, and **`AGENTS.md`** (both variants) list
  `on_hold` in the project status enum.

### Tests

- `scripts/test.sh` asserts an `on_hold` project is skipped by the active-project
  resolver in favor of a `status: active` wave.

## 2026-07-12 — optional `project.activity` field (v0.6.15)

Blesses `project.activity` as first-class template vocabulary. `project.status`
was overloaded — the coarse machine-keyed lifecycle state *and* where people
tried to say what kind of work was happening now. Those are two axes; this
splits them. `status` stays coarse (`proposed | active | shipped | cancelled`);
`activity` is a new **optional, human-facing** refinement of the work within an
`active` project. Additive and backward-compatible: every existing brief with no
`activity` stays valid. (The standup portfolio tracker already parses this field
and treats `activity: requirements` as a deliberately-quiet phase.)

### Added

- **`project.activity`** in the brief scaffold (`projects/_templates/project-brief.md`,
  both variants) — optional, defaults to `null`, with the suggested open
  vocabulary `requirements | design | build | test | blocked`.
- **`get_project_activity`** in `scripts/_lib.sh` — reads `project.activity`,
  empty for `null`/missing.

### Changed

- **`just validate`** accepts `activity` and treats the vocabulary as an **open
  set**: an unrecognized value is **advisory (warn-only), never a gate failure**,
  so the set can be extended freely (e.g. `spike`).
- **Docs** — `AGENTS.md` and `docs/schema-reference.md` (both variants) document
  the `status`-vs-`activity` split, the suggested vocabulary, and the example.

### Tests

- `scripts/test.sh` asserts a recognized `activity` passes cleanly, an
  unrecognized value still exits 0 (warn-only), and `get_project_activity` reads
  the value / returns empty for `null`.

## 2026-07-12 — `roadmap` surfaces planned-but-unframed stages (v0.6.14)

Closes harvest backlog #8. `just roadmap` rendered one row per `STAGE-*.md`
file — so a project's *planned* stages (the `## Stage Plan` rows in the brief
that haven't been framed into a stage file yet) were invisible. The roadmap now
shows the whole forward arc: framed stages from their files, then the
planned-but-unframed rows parsed from the brief.

### Added

- **`parse_stage_plan` in `scripts/_lib.sh`** — parses a brief's `## Stage Plan`
  checkbox list into `STAGEID|CHECKED|TITLE` rows (STAGEID is `-` for a
  `(not yet defined)` row), stopping at the next `## ` heading.
- **`roadmap` planned bucket** — `scripts/roadmap.sh` renders planned-but-unframed
  stages (a `## Stage Plan` row with no matching `STAGE-*.md`) after the
  file-driven rows, in both human and `--json` (a new `planned` array). A row
  whose `STAGE-NNN` already has a file is dropped, so nothing is double-listed.

### Changed

- **`roadmap` no longer early-exits when `stages/` is absent** — a brand-new
  project with a Stage Plan but no framed stages yet still shows its planned arc.

### Tests

- `scripts/test.sh` asserts planned stages appear in human + `--json` output and
  that a framed stage is never re-listed as planned.

## 2026-07-10 — `lifetime-report`: whole-repo history horizon (v0.6.13)

Adds the missing time horizon: `just status` is *now*, `just review` is the
*recent slice* (last 7 days, active project), and `just lifetime-report` is the
*whole-repo arc* — every project, stage, spec, decision, and release since the
first commit, pre-loaded into a synthesis prompt an LLM turns into a narrative.
Ported from the bragfile experiment where it was validated and dated.

### Added

- **`scripts/lifetime-report.sh` + `just lifetime-report`** — assembles the
  lifetime aggregates (release timeline, per-project created→shipped dates,
  specs-by-stage, decision log, git span) and prints a copy-into-Claude prompt.
  Depends only on template scripts (`_lib.sh`, `status.sh`, `specs-by-stage.sh`,
  `today()`), so every scaffolded repo inherits it.
- **Dated report** — sets `Generated: <today>` and instructs the report to head
  with it, explicitly distinguished from the git-span / project-history dates
  (report date ≠ history covered). This dated behavior is the reason for the port.
- **`scripts/test.sh`** — coverage asserting the prompt prints, the
  `Generated: <today>` line is present and equals today, and all path bullets
  are repo-relative (same pipe posture as `review`).

## 2026-07-06 — Docs freshening: README + GETTING_STARTED + blog outlines (v0.6.12)

Catches the user-facing docs up to the session's work (v0.5.28→v0.6.11) and
preserves the harvest's blog-worthy lessons. Docs-only.

### Changed

- **README command table** now lists the session's additions: the patch lane
  (`new-patch`), `new-release-spec`, `next-version`, `build-info`, the new `dash`
  lenses (constraints/handoffs/signals/patches), and the `shipped_at` stamp.
- **GETTING_STARTED** (both variants) gains an **"As your project grows"** section
  pointing at the newer surface a first project will meet — `toolchain-brief.md`
  (fill it early), value-with-a-metric (with an honest escape hatch for goal-less
  projects), the patch lane, release/versioning/provenance, and brag-at-ship.
  The core loop is unchanged; a fuller rewrite should be driven by real usage.

### Added

- **`docs/blog/2026-07-06-dogfood-lessons-outline.md`** — lightweight outlines for
  the 3–5 strongest, evidence-backed lessons from the harvest (defects escape at
  runtime; trust git over sub-agent self-report; value tracks stakes not size;
  measure/probe at design; the N=3 codification bar), preserved for write-up.
- **[DEC-009](docs/decisions/DEC-009-business-value-metrics.md)** open question #5:
  goal-less/exploratory projects need an escape hatch (proxy / checkable signal /
  explicit "exploratory") so the metric convention doesn't reward fabrication.

## 2026-07-06 — DEC-010 simplified: coach brag directly, drop the wrapper (v0.6.11)

The v0.6.10 cut added a `just log-win` wrapper + `scripts/log-accomplishment.sh`
that pre-filled `brag add`. That was over-built: `brag` is a first-party tool the
agent can call directly (CLI or MCP). The template should **coach**, not wrap.

### Removed

- **`scripts/log-accomplishment.sh`**, the **`just log-win`** recipe, and the
  `get_accomplishments_field` helper in `_lib.sh`. Also dropped the now-vestigial
  `interface: cli | mcp` config field.

### Changed

- **The agent calls `brag` directly at ship** — AGENTS "During ship" and
  `guidance/recommended-tools.md` (both variants) coach `brag add -i "<impact>"`
  (CLI) or the `brag_add` tool over `brag mcp serve` (MCP), seeded from the spec's
  `value_link` + `cost.totals`. `spec.accomplishments` stays as a declarative
  config (`enabled` / `tool`) the agent reads — no script consumes it. DEC-010
  updated to record the simplification.

## 2026-07-06 — DEC-010: accomplishment logging on by default (via brag) (v0.6.10)

Reverses the earlier "keep it out of the template defaults" stance: recording a
shipped win **with impact** is now on by default, so it stops being skipped. The
default tool is `brag` (a first-party local-first CLI + MCP), which has the right
seam (`brag add -i "<impact>"`). Impact capture is the outward form of the value
the template already records (`value_link`, `cost.totals`).

### Added

- **`spec.accomplishments` config** in `.repo-context.yaml` (both variants):
  `enabled: true` · `tool: brag` · `interface: cli | mcp`. Opt out with
  `enabled: false`; swap `tool` for an equivalent. Keeps the core tool-agnostic
  while defaulting to the first-party tool.
- **`just log-win SPEC-NNN`** (`scripts/log-accomplishment.sh`;
  `get_accomplishments_field` in `_lib.sh`) — pre-fills the entry from the spec's
  title + `value_link` + `cost.totals` (framing value-per-dollar), runs `brag
  add` if present, else prints the ready command. Degrades cleanly when brag is
  absent or logging is disabled.
- **`docs/decisions/DEC-010`** records the decision.

### Changed

- **`guidance/recommended-tools.md`** (both variants): the accomplishment section
  is now "on by default via `brag`," documents the CLI, the scripted `--json`
  mode (safe for a sub-agent), the MCP path (`brag mcp serve` → `brag_add`), and
  `brag project here` auto-fill.
- **AGENTS "During ship"** (both variants): *log the win* (default-on via `just
  log-win`) instead of *optionally log it*.

## 2026-07-06 — Harvest quick wins: frame optional + agents-fields clarity (v0.6.9)

Two small doc clarifications from the harvest. Docs-only.

### Changed

- **`frame` is documented optional** (harvest #12) — AGENTS.md §8 Cycle Model
  (both variants) notes most specs start at `design`; `frame` went unused across
  the dogfood (0 of 100+ specs). Use it only when a spec's existence is genuinely
  in question. Not removed (non-breaking), just clarified.
- **`agents.architect` ≠ `agents.implementer` is the intended design/build tier
  split, not contamination** (harvest #13) — the claude-only `spec.md` agents
  block now says so, so a verify session stops misreading the differing tier_map
  models (DEC-005) as evidence the design session leaked into build.

## 2026-07-06 — cost-audit flags implausibly-low metered cost (v0.6.8)

Harvest signal #5. A session-limited sub-agent can return an implausibly small
`subagent_tokens` (662 for a full verify); that passes the non-null cost gate and
silently deflates cost totals — a real cost-integrity hole (and a hazard for any
future value/ROI number derived from cost).

### Added

- **`just cost-audit` now emits an advisory** when a shipped spec/patch records a
  metered-cycle `tokens_total` that is positive but below `COST_IMPLAUSIBLE_FLOOR`
  (default 1000, override via env). It **does not fail the gate** — a low number
  might be legitimate — but surfaces it so truncated metering isn't trusted
  silently. New `spec_implausible_cost_cycles` in `_lib.sh` (reuses
  `cycle_tokens_total`).

## 2026-07-06 — Release pre-flight: two-phase cut + evidence timing (v0.6.7)

Harvest signal #6 (fresh, from bragfile's v0.3.1 cut). A release session
structurally *can't finish in one pass* — the irreversible tag/publish is
human/coordinator-gated — so several pre-flight items can't be verified in-session.
The release-spec (DEC-006) now models that honestly. Docs-only, both variants.

### Changed

- **`projects/_templates/release-spec.md`** gains a **"Release cut is two-phase"**
  section: Phase 1 = reversible prep (CHANGELOG, `just next-version` bump, backlog
  tick) landed as a CI-gated PR; Phase 2 = the irreversible tag + publish, gated,
  after Phase 1 merges. The spec ships **"prep-complete, cut-deferred."**
- Each pre-flight item now carries a verification **timing** — `[now]` (verified
  this session, evidence attached) vs `[cut]` (only checkable at Phase 2 — a
  clean-host install, a channel check, notarization propagation — so it's deferred
  and verified by whoever runs the cut, recorded honestly rather than ticked).

## 2026-07-06 — Codify the design-time probe / measure-before-build (v0.6.6)

Harvest signal #2 — the **highest-frequency single lesson** across the dogfood
(N=17/43 in one project, convergent with another). Codifies it as a first-class
design-cycle convention. Docs-only.

### Added

- **AGENTS.md §12** (both variants) gains a **"Design-time probe /
  measure-before-build"** convention, sibling to behavioral pre-flight: when a
  spec depends on a load-bearing external's *actual* behavior (real API
  signature, tool resolution on the pinned toolchain, version floor, a config
  field the engine reads) or tunes toward a measurable target, **probe/measure
  the real thing against the pinned tree during design** and record the verified
  calls / baseline in `## Implementation Context` (or the DEC). Build then
  collapses to a near bit-for-bit *transcription* instead of a discovery loop.
  Includes the complementary verify move (**adversarial mutation** — revert and
  confirm the guard fails; proves test teeth + surfaces dead/no-op config).
- The **SPEC-design prompt** (`FIRST_SESSION_PROMPTS.md` Prompt 2b, both
  variants) reinforces it at the point of use — probe/measure *before* writing
  the failing tests.

## 2026-07-06 — Multi-wave correctness: status-aware resolver + shipped_at (v0.6.5)

Two fixes from the 2026-07-06 three-project dogfood harvest
([docs/harvests/](docs/harvests/2026-07-06-three-project-dogfood-harvest.md)),
both prerequisites for trustworthy multi-wave and value/time reporting.

### Fixed

- **`get_active_project` was status-blind (harvest #1, high-impact).** It picked
  the lowest-numbered non-example project and ignored `status`, so the moment a
  second wave existed, every default-scoped command (`just status`, `cost-audit`,
  `backlog`, reports) silently targeted the *shipped* earlier project and never
  inspected the active wave — `cost-audit` would run green on the wrong project.
  The resolver now prefers a project marked `status: active` (lowest-numbered
  among several; falls back to lowest-numbered, then the example). New
  `get_project_status` helper (field-2 parse → tolerates a trailing comment).

### Added

- **`archive-spec` now stamps a top-level `shipped_at: DATE` (harvest #3).** Ship
  dates previously lived only in git tags / timeline / cost blocks, so per-spec
  cycle-time and time-to-value weren't computable from the spec. Now they are —
  the plumbing DEC-009 (time-to-value) builds on.

### Docs

- **`docs/harvests/2026-07-06-three-project-dogfood-harvest.md`** — the durable
  triaged record (field-validation of the recent cycle, the ranked still-open
  backlog, and the business-value / time-to-value findings).
- **[DEC-009](docs/decisions/DEC-009-business-value-metrics.md) (proposed)** — a
  thin measurable-value layer (one headline `value_metric` per stage, a
  metric-*derivation* aid to be validated on the next project's frame, and
  time-to-value computed from the new `shipped_at` + `created_at`). Design-first;
  awaits review + a real project to ground the derivation step.

## 2026-07-06 — DEC-008: build provenance — trace a build to its commit (v0.6.4)

Every project can now stamp its builds so a user (or an external report reader)
knows exactly what they're running — traceable back to the source commit. Pairs
with the versioning scheme (DEC-007): DEC-007 = "what version," DEC-008 = "which
exact source." Additive. See
[DEC-008](docs/decisions/DEC-008-build-provenance.md).

### Added

- **`just build-info`** (`scripts/build-info.sh`; `build_ref` / `build_commit` /
  `build_commit_short` / `build_dirty` in `_lib.sh`) — emits a `git describe`-style
  ref (nearest tag + commits-since + short SHA, `-dirty` if the tree is dirty)
  plus the full commit, dirty flag, and build timestamp. `--json` supported.
  Degrades to `unknown` outside a git repo.
- **A "Build provenance" section in `docs/versioning.md`** (both variants) — the
  rule (always inject the stamp into the artifact at build time so `<app>
  --version` reports it) and a per-delivery-shape injection table (ldflags /
  generated build-info file / OCI label / `BUILD_INFO` sidecar).

### Changed

- **The release-spec's tag-integrity pre-flight** (both variants) now requires
  the shipped artifact to report a build provenance matching the release commit —
  checked at every release, not assumed.

## 2026-07-06 — report --json: machine-readable report envelopes (v0.6.3)

Extends the DEC-001 §2 `--json` contract to the reports, so they can feed
tooling / an external surface (not just the human prose file). Additive.

### Added

- **`just report daily --json`** and **`just report weekly [DATE] --json`** (and
  the `report-daily` / `report-weekly` aliases) emit a lean quantitative envelope
  to **stdout** and skip the prose file. Daily carries `{project, date, progress
  (shipped/scaffolded/active/pct), cost (tokens_total/estimated_usd), thesis}`;
  weekly carries `{week, start, end, project, shipped_this_week (count+specs),
  cost_this_week (tokens/usd/avg)}`.

### Changed

- The `report` dispatcher and the two alias recipes now forward flags
  (`*REST`), and `report_weekly.sh` separates `--json` from its optional `DATE`
  arg — so `report weekly 2026-07-01 --json` and `report weekly --json` both work.

## 2026-07-05 — Quick wins: two dash lenses + stricter changelog guard (v0.6.2)

Small additive ergonomics — two new read-lenses and a tightened release guard.

### Added

- **`just dash constraints`** — the repo-level rules from
  `guidance/constraints.yaml`, grouped by severity (blocking first) with paths +
  rule text. `--json` emits `constraint.*` names. (`emit_constraints_tsv` /
  `count_blocking_constraints` in `_lib.sh`.)
- **`just dash handoffs`** — delegation handoffs (`HANDOFF-*.md`) grouped by
  status (`pending | accepted | completed | rejected`), showing spec + from→to.
  Meaningful in the `claude-plus-agents` variant; an empty view in `claude-only`.
  `--json` emits `handoff.*` names. Both lenses are surfaced in `just dash help`.

### Changed

- **Stricter CHANGELOG drift-guard** (`scripts/test.sh`): beyond "VERSION appears
  somewhere," the newest `## … (vX.Y.Z)` header must now *be* the current
  `VERSION` (catches a bump whose new entry was never added on top), and every
  versioned header must be unique (no duplicate release sections).

## 2026-07-05 — DEC-007: default versioning scheme (CalVer), overridable (v0.6.1)

Gives scaffolded apps a versioning convention that "just works," with semver as
a per-project opt-in — the follow-on to the release-spec ([DEC-006](docs/decisions/DEC-006-release-spec-template.md)),
which cut version tags but defined no scheme. Additive. See
[DEC-007](docs/decisions/DEC-007-versioning-default.md).

### Added

- **`spec.version.scheme`** in `.repo-context.yaml` (both variants) —
  `calver | semver | monotonic`, **default `calver`** (`vYYYY.MM.PATCH`, e.g.
  `v2026.07.0`). CalVer needs no "major or minor?" judgment at release time,
  which is why it's the default; it fits the app/service/CLI majority. semver is
  the opt-in for a library/public API (chosen by delivery shape); monotonic
  (`vN`) is the minimal-ceremony option.
- **`just next-version`** (`scripts/next-version.sh`; `get_version_scheme` /
  `get_next_version` in `_lib.sh`) — suggests the next app tag per scheme from
  git tags (degrades to the scheme's first version with no tags / no git yet).
  semver prints the current latest and defers the bump level to you. `--json`
  supported.
- **`docs/versioning.md`** (both variants) — the scheme, when to pick semver
  (by delivery shape), and the app-version-vs-`VERSION`-file distinction.

### Changed

- **The `VERSION`-file overload is resolved by documentation:** `VERSION` is the
  **template provenance** (which template version this repo was scaffolded from);
  the *app* version lives in **git tags** / the ecosystem file. Stated in
  `.repo-context.yaml`, `docs/versioning.md`, the release-spec `Release Scope`,
  and AGENTS "During ship" + Pointers (both variants).
- The release-spec's `Version / tag` line now points at `just next-version` and
  the configured scheme instead of a bare `vX.Y.Z`.

## 2026-07-05 — DEC-001 Phase 3: report/review command consolidation (v0.6.0)

Ships Phase 3 of [DEC-001](docs/decisions/DEC-001-interface-contract.md) — the
deliberately-deferred **breaking** part of the command-surface rationalization,
hence the minor bump to 0.6.0. The two genuinely-confusable read/prompt commands
are consolidated into a `report` / `review` namespace. The daily-driver bare
names are kept as permanent aliases (muscle memory wins over tidiness).

### ⚠ Breaking

- **`just weekly-review` → `just review`.** The weekly-review prompt command is
  renamed. `weekly-review` no longer exists.
- **`just daily-status-report` → `just report status`.** The uncurated status
  snapshot moves under the `report` namespace. `daily-status-report` no longer
  exists.

### Added

- **`just report {daily | weekly [DATE] | status}`** — one report namespace.
  `daily` / `weekly` wrap the existing curated report scripts; `status` is the
  uncurated `just status` snapshot (formerly `daily-status-report`). An unknown
  subcommand prints usage and exits 2.
- **`just review`** — the weekly-review prompt (formerly `weekly-review`).

### Unchanged (permanent aliases)

- **`just report-daily`** and **`just report-weekly`** keep their bare names as
  permanent aliases for `report daily` / `report weekly` (DEC-001 §3). No
  deprecation — they are load-bearing muscle memory.

### Docs / tests

- README command tables (root + both variants), `docs/USAGE.md`,
  `GETTING_STARTED.md`, `FIRST_SESSION_PROMPTS.md`, and AGENTS.md session-hygiene
  updated to the new names. `docs/decisions/DEC-001` marks Phase 3 shipped.
- `scripts/test.sh` swings to the new names and asserts both that the aliases
  still work **and** that the removed names now fail (proving the break).

## 2026-07-05 — DEC-006: release-spec template + runtime pre-flight (v0.5.29)

Accepts and builds [DEC-006](docs/decisions/DEC-006-release-spec-template.md)
(upstream-candidate B from the bragfile three-project retrospective). Across the
dogfood projects, **every defect that escaped design→build→verify was
operational/runtime**, and the release-phase subclass was especially consistent
(dual-tag-on-the-same-commit, Gatekeeper quarantine, package-manager trust
gates, a dev binary migrating the prod DB). Each was earned in production then
codified after the fact — and each is portable. This puts the checklist in the
template so every project inherits it instead of re-earning it. Additive.

### Added

- **`projects/_templates/release-spec.md`** (both variants) — a spec-shaped
  template for a release cut whose `## Release Pre-Flight` carries the six
  generic, portable categories: (1) version/tag integrity, (2) artifact trust on
  a clean host, (3) distribution-channel trust, (4) data isolation, (5) runtime
  smoke on a clean host, (6) rollback/uninstall. Kept **category-level, not
  command-level** — the instance fills the tool-specific command per its stack
  (same "template ships the slot, instance fills the truth" principle as the
  toolchain brief). A `Delivery shapes` line + per-item `N/A` lets a pure web
  service skip desktop-only OS-trust. The plus-agents copy uses the `handoff:`
  block; the claude-only copy uses `agents:`.
- **`just new-release-spec "vX.Y.Z" STAGE-NNN`** — ergonomic wrapper over the new
  **`--release`** flag on `new-spec` (the flag is the primitive; either works).
  It scaffolds a `task.type: release` spec.
- **`get_spec_type`** in `_lib.sh` — reads `task.type`, scoped to the `task:`
  block.

### Changed

- **`status`** now recognizes releases: the human "Specs by cycle" view tags a
  release spec `[release]`, and `--json` exposes `task.type` on every spec.
- **AGENTS.md "During ship"** (both variants) gains a pointer: a release is its
  own spec — scaffold it with `new-release-spec` and run the pre-flight before
  publishing.
- **`docs/schema-reference.md`** documents the `release` `task.type` and the
  release-spec subtype (reuses the spec schema; `validate`/`cost-audit`/`status`
  treat it first-class).
- **[DEC-006](docs/decisions/DEC-006-release-spec-template.md)** flipped to
  **accepted** (confidence 0.7→0.8); its four open questions resolved in the doc
  (release type over chore; flag-plus-wrapper; checklist not gate; categories
  generic with per-shape N/A).

## 2026-07-05 — DEC-004 Phase 2: dev-dep sanction + toolchain-brief slot (v0.5.28)

Completes Phase 2 of [DEC-004](docs/decisions/DEC-004-subagent-execution-mode.md)
— rules 4 and 5, both additive. Phase 1 (v0.5.27) documented the reconcile /
one-sub-agent / explicit-model rules; the two remaining failure classes from the
dogfood harvests were a non-interactive build sub-agent that **can't stop to
author a DEC** (so a hard deps constraint drove it to a `@types/node`-stub
workaround) and a **cold sub-agent re-deriving the same toolchain mismatches**
every run (~10 wasted loops). This ships the two slots that close them.

### Added

- **`guidance/toolchain-brief.md`** (both variants) — a REPLACE stub of the
  per-repo toolchain facts a cold build sub-agent needs: package manager, test
  framework + assertion lib, lint/format quirks, runtime globals, installed dev
  utilities (don't re-add), and known gotchas. The template ships the slot; the
  instance fills the truth. Referenced from AGENTS.md §15 "During build" (read it
  before coding; inject it into a delegated sub-agent's prompt), the §18/§17
  Pointers, and the §7 directory-structure diagram. (DEC-004 rule 5.)

### Changed

- **`no-new-top-level-deps-without-decision` constraint** (both variants'
  `guidance/constraints.yaml`) now scopes the hard gate to **runtime** deps and
  carves out an explicit exception: a build cycle MAY add a clearly-trivial
  **DEV-only** dependency (types packages, test utilities — never a runtime dep)
  **and author its DEC in the same pass**, with no stop-and-ask. Keeps the
  constraint's teeth for real choices while unblocking a non-interactive build
  sub-agent. (DEC-004 rule 4.)
- **The "Delegated execution (sub-agents)" AGENTS.md section** (both variants)
  grows from three rules to five — rules 4 (dev-dep sanction) and 5 (inject the
  toolchain brief) join the Phase 1 trio.
- **[DEC-004](docs/decisions/DEC-004-subagent-execution-mode.md)** status note
  updated: Phase 2 done; Phase 3 (mechanical per-agent worktree isolation)
  stays deferred.

## 2026-06-27 — DEC-004 Phase 1: delegated-execution (sub-agent) rules (v0.5.27)

Implements Phase 1 of [DEC-004](docs/decisions/DEC-004-subagent-execution-mode.md)
(now **accepted**). Both shipped dogfood projects delegated build/verify to fresh
sub-agents — notably under `claude-only` running the Agent tool, not just
`claude-plus-agents` — and surfaced a failure class the template never documented:
truncated self-reports that claim "done" with the commit missing, shared-tree
corruption, and silent model defaults. This ships the orchestration discipline.

### Added

- **A "Delegated execution (sub-agents)" section** in both variants' `AGENTS.md`
  (claude-only §16, claude-plus-agents §13):
  1. **Reconcile over self-report** — never advance a cycle (or flip
     `handoff.status: completed`) on a sub-agent's word; verify against `git log`
     / `git ls-remote` + disk first (the exact commands are in the rule). Trust
     git/disk over any self-report or timeline marker. Includes the **die-mid-cycle
     recovery** procedure (reconcile the partial output → finish the mechanical
     remainder in the main loop → attribute cost to the metered `subagent_tokens`).
  2. **One sub-agent at a time; no interleaved tree ops** until it reports and its
     branch is merged (the shared-tree hazard; worktree isolation is the fix).
  3. **Set the sub-agent's model explicitly** from `spec.agent.tier_map` — no
     silent Opus default (a ~6× surprise). Consumes DEC-005's config.

### Notes

- +2 test checks (now 187): the delegated-execution section + the reconcile rule
  survive init. Documentation only — no script/command change.
- **Pending** (DEC-004): rule 4 (sanctioned trivial-dev-dep + DEC path), rule 5
  (per-instance toolchain-brief slot), and Phase 3 (mechanical worktree isolation).
  A `_lib.sh` reconcile helper was judged low-value — the rule already ships the
  mechanical `git` commands.

## 2026-06-27 — DEC-005 Phase 2: config-driven models + generalized wording (v0.5.26)

Finishes [DEC-005](docs/decisions/DEC-005-agent-portability.md) (now **fully
implemented**). Phase 1 made cost portable; this removes the last hard-coded
Claude model ids and the Claude-specific session wording.

### Changed

- **`new-spec` / `new-patch` stamp `agents.*` from the `tier_map`** (new
  `get_default_model` + `get_tier_model` in `_lib.sh`): a scaffolded spec's
  `architect` = `tier_map.design`, `implementer` = `tier_map.build` (and the
  `claude-plus-agents` `handoff.from_agent` = `tier_map.design`); a patch's
  `implementer`/`verifier` = `tier_map.build`/`verify`. Model ids are no longer
  hard-coded in the templates — a non-Claude instance's specs carry *its* models.
  With the default tier map this also makes `architect` ≠ `implementer`
  (opus/sonnet), fixing the "architect == implementer looks like design→build
  contamination" misread a downstream verifier hit.
- **Generalized "Claude session" → "session"** in `AGENTS.md`, `GETTING_STARTED.md`,
  and `README.md` (both variants). The `claude-only` fresh-session-per-cycle
  premise is now stated once as an explicit *variant assumption* (read "session"
  as "session/agent" on another tool), not sprinkled as a universal.

### Notes

- +4 test checks (now 185): tier-map stamping on specs (design≠build) and
  patches, no leftover model placeholders, and no `Claude session` left in
  `AGENTS.md`. Defaults reproduce Claude behavior, so existing instances are
  unaffected; `docs/porting.md` updated (no by-hand edits left for a porter).

## 2026-06-27 — DEC-005 Phase 1: run on non-Claude agents (config + graceful cost-audit) (v0.5.25)

Implements Phase 1 of [DEC-005](docs/decisions/DEC-005-agent-portability.md)
(now **accepted**). The template is ~70% agent-portable already (`AGENTS.md` is
the cross-tool standard; `handoff.to_agent` is agent-agnostic); the coupling is
concentrated in the model + cost layer, and the **cost gate is the one hard
blocker** on a platform with no token meter. This parameterizes it.

### Added

- **`spec.agent` + `spec.cost` config in `.repo-context.yaml`** (both variants):
  `agent.default_model`, `agent.tier_map` (design/build/verify), and
  `cost.{metering_source, rate_per_mtok, currency}`. Defaults reproduce the
  Claude-Code workflow, so **existing instances change nothing**.
- **`docs/porting.md`** (both variants) — how to run the template on a non-Claude
  agent (point the tool at `AGENTS.md`, set the config, pick a `metering_source`),
  what ports cleanly, and what's still Claude-shaped.

### Changed

- **`just cost-audit` honors `spec.cost.metering_source`** (new
  `get_metering_source` in `_lib.sh`). `subagent_tokens` / `api_usage` / `manual`
  keep the gate enforced; **`none` disables it** — so a platform that exposes no
  token count no longer fails every shipped spec on an impossible number. This is
  the change that unblocks a non-Claude run.

### Notes

- +5 test checks (now 181): the config + porting doc survive init, cost-audit
  still enforces by default, and `metering_source: none` disables the gate.
- **Phase 2 (pending, DEC-005):** `new-spec`/`new-patch` stamp `agents.*` from
  `default_model`/`tier_map`, and the "fresh Claude session" prompt wording is
  generalized. Until then, edit `agents.*` / wording by hand where it matters.

## 2026-06-27 — Runtime coverage: behavioral pre-flight + defect-catch-stage (v0.5.24)

From the bragfile three-project retrospective (40/42 shipped, one supersession,
zero design→ship drift — the discipline is validated at scale; the retro's
explicit ask is *don't* speed up or codify sooner). Its one structural finding:
design→build→verify is dense on spec-logic and sparse on **runtime/operational**
behavior — every defect that escaped a cycle across three projects was
operational/runtime. These are the two small, portable additions it asked for.

### Added

- **Behavioral pre-flight convention** (AGENTS.md §12, both variants): when a
  spec's literal/artifact claims *runtime behavior* — a component registers, a
  hook fires, a binary resolves on PATH, a server answers, a config takes effect
  — exercise it through the surface that **runs** it before design is done, not
  merely the surface that **validates its shape** (a manifest can pass
  `validate --strict` and still register nothing). The verify checklist gains a
  matching check. The template taught *no* design-time pre-flight before this.
- **Defect-catch-stage tag** on the ship reflection (`spec.md`) and patch
  completion (`patch.md`), both variants: one word from a fixed vocabulary
  (`design | build | verify | ship | escaped | none`) so the **defect-escape
  distribution** — "where do defects actually get caught, and what escapes?" — is
  greppable across specs (it only shows up in a cross-project view).

### Notes

- +3 test checks (now 176): the pre-flight convention + both defect-catch tags
  survive init.
- Guidance/template text only — no script or command change. The retro's
  bigger second ask (a release-spec template with an operational checklist) is
  drafted as **DEC-006** (proposed) rather than built, to keep the checklist
  generic. Validation items the retro says to protect (codification lag, the
  N=3/N=2 bar, confidence, PEEL, premise-audit family) are unchanged by design.

## 2026-06-27 — Harvest backlog: cost rollup, audit hygiene, severity map (v0.5.23)

The smaller-but-worthwhile items from the crustyimg + zany-animal-slots harvest.

### Fixed

- **`archive-spec` / `archive-patch` now recompute `cost.totals`** from the
  recorded `cost.sessions` (new `write_cost_totals` in `_lib.sh`) — the
  non-judgment-laden half of the ship-bookkeeping debt (the other half, the
  backlog `Count:`, landed in v0.5.19). The rollup can no longer go stale.
- **`decisions-audit` stops flagging intentional scope nesting as a conflict.**
  A broad decision that deliberately contains a narrower one (e.g.
  `src/engine/**` over `src/engine/rng.ts`) is now reported as **info**
  ("nested scope — hierarchy, not a conflict"), not a warning — killing the
  standing noise (zany saw 19+). Two decisions at the **same** scope still warn.
- **`decisions-audit` guards against a false-confidence `affected_scope`.** A
  bare-name entry with no path separator or wildcard (e.g. `_headers`, which
  never matched the real `public/_headers`) now warns — silently governing
  nothing is worse than noise.

### Added

- **A canonical severity mapping** in both variants' `constraints.yaml` header
  and `docs/schema-reference.md`: `critical`/`high` → `blocking`, `medium` →
  `warning`, `low` → `advisory` — so a plan's critical/high/medium/low rating
  maps cleanly onto the enforcement enum (resolves the vocab mismatch zany hit).

### Notes

- +4 test checks (now 173): the `cost.totals` recompute, nesting-as-info vs
  same-scope-warning, the bare-name `affected_scope` guard, and the severity
  mapping surviving init.
- Also in this release group: **DEC-004** (sub-agent / delegated-execution mode)
  and **DEC-005** (non-Claude agent portability) — both **proposed** design
  records, no functional change (docs/decisions/).

## 2026-06-27 — Patch lane visibility: `dash patches` lens + reports (v0.5.22)

Completes the patch lane's v1 follow-up (the surfaces DEC-003 / v0.5.21 deferred).

### Added

- **`just dash patches`** (`scripts/patches-view.sh`) — a new `dash` lens: the
  active project's patches grouped by cycle (`patch|verify|ship`), flagging a
  shipped patch that's missing its metered (`patch`+`verify`) cost. Human +
  `--json` (same `task.*`/`cost.*` attribute names as `status`'s `patches[]`).
  A lens, not a new top-level recipe (DEC-001 §4).
- **Patches in the reports.** `report-daily` grows a `## Patches` section
  (in-flight vs shipped counts, WIP patch cost, shipped patches missing metered
  cost) and `report-weekly` a `## Patches` section (total/shipped/in-flight +
  recorded patch cost). Both emit only when the project has patches, so a
  patch-free project's reports are unchanged.

### Notes

- +6 test checks (now 169): the `dash patches` lens (human + `--json` envelope,
  valid JSON, and `task.*` payload) and the daily/weekly `## Patches` sections.
- Removed the `KNOWN_LIMITATIONS.md` note that flagged reports + a patches lens
  as pending — both now ship. The patch lane is complete across `validate`,
  `cost-audit`, `status`, `dash` (a lens + `dash now`), and the reports.

## 2026-06-27 — The patch lane: lightweight fixes to shipped behavior (v0.5.21)

The #1 recommendation of the dogfood retrospective, validated by two shipped
projects (crustyimg, which shipped a proof-of-concept `DEC-043` + `PATCH-001`;
bragfile). The full five-cycle is disproportionate for a trivial fix — the two
things that *bought* quality were the DEC log and the **independent verify**, not
the four named cycles. The patch lane keeps exactly those and drops the rest.
Design record: `docs/decisions/DEC-003-patch-lane.md`.

### Added

- **A patch artifact + collapsed cycle.** A **patch** is a bounded fix to
  already-shipped behavior (a bug or UX papercut) that adds no new
  feature/command. It runs **`patch → verify → ship`** (design+build fused into
  one test-first pass; the **independent verify is kept**). `projects/_templates/patch.md`
  (both variants).
- **`just new-patch "title" [PROJ-NNN]`** (`scripts/new-patch.sh`) → scaffolds
  `projects/PROJ-*/patches/PATCH-NNN-<slug>.md` with its own repo-wide,
  continuous `PATCH-*` id sequence. **`just archive-patch PATCH-NNN`**
  (`scripts/archive-patch.sh`) → `patches/done/`, **no stage bookkeeping**.
- **Patches are first-class in the tooling** (via `task.type: patch`): a patch
  reuses the spec `task.*` schema (so it maps to the same ContextCore attribute
  names), with `task.cycle` ∈ `{patch,verify,ship}` and **no `project.stage`**.
  `just validate` validates patch front-matter; `just cost-audit` gates a shipped
  patch's `patch`+`verify` cost; `just status` lists patches by cycle (human +
  `--json` `patches[]`). `just advance-cycle` accepts the `patch` cycle.
- **Docs:** a "Patch lane" section in both variants' `AGENTS.md` (Cycle Model)
  and in `docs/USAGE.md`; the patch artifact + gates in `docs/schema-reference.md`.

### Notes

- +13 test checks (now 163): scaffold shape (type/cycle/no-stage/PATCH-001),
  `advance-cycle` on the patch cycle, `validate` accept + reject-bad-cycle,
  `cost-audit` gating a shipped patch, `status` human + `--json` patches, and
  `archive-patch` + double-archive refusal.
- Additive; existing spec flow, output, and files are unchanged. **Guardrail:**
  a change that adds a command/flag or needs its own design exploration is a
  spec, not a patch.
- v1 scope: `validate`/`cost-audit`/`status` (and `dash now`, which inherits
  `status`). Deferred to a follow-up: patch lines in `report-daily`/`report-weekly`
  and a dedicated `dash patches` lens (`KNOWN_LIMITATIONS.md`).

## 2026-06-27 — Repo-wide continuous STAGE/SPEC numbering (v0.5.20)

From dogfood feedback (a second project restarted numbering at `001` instead of
continuing). `next_id` already defaulted to a repo-wide scan; the two call-sites
were narrowing it per-project.

### Changed

- **`new-stage` / `new-spec` assign IDs continuously across the whole repo.**
  Dropped the per-project search dir so both use `next_id`'s repo-wide default:
  with PROJ-001 at `STAGE-006` / `SPEC-037`, a stage in PROJ-002 is `STAGE-007`
  and its first spec is `SPEC-038` — IDs are globally unique and no longer
  restart per project. Existing files are untouched. (`scripts/new-stage.sh`,
  `scripts/new-spec.sh`.)
- **Documented the convention** in both variants' `AGENTS.md` (Work Hierarchy)
  and `GETTING_STARTED.md` (project-ship step), and in the `stage.md` template
  comment (`# stable, zero-padded, continuous across the repo`).

### Fixed

- **`new-stage` / `new-spec` now `mkdir -p` their target dir.** A hand-created
  project (copied from `project-brief.md`, so only `brief.md` exists) no longer
  fails with `cp: … No such file or directory` when it has no `stages/`/`specs/`
  dir yet.
- **Reconciled `KNOWN_LIMITATIONS.md`** with the v0.5.19 `archive-spec` change
  (it now performs the mechanical backlog flip + `**Count:**` recompute; only the
  judgment-laden list curation stays manual), and expanded the
  `get_active_project` note (it's status-blind — a shipped project can stay
  "active"; filed as a candidate improvement, not made because it changes the
  selection every command sees).

### Notes

- +1 test check (now 150): a stage in a fresh second project continues the
  repo-wide count instead of restarting at `001` (also exercises the `mkdir -p`).
- Backward-compatible: no files move; only newly-assigned IDs change. If a
  per-project scheme is ever wanted it can become a documented
  `.repo-context.yaml` toggle, but continuous is the default and the only mode.

## 2026-06-27 — P1 dogfood fixes: silent-failure bugs + ship bookkeeping (v0.5.19)

The first fixes harvested through the new Signals registry, from two shipped
projects (crustyimg — 43 specs/9 stages; zany-animal-slots — non-CRUD frontend).
All four were verified still-live against the current template before fixing.

### Fixed

- **`find_spec` now excludes `specs/prompts/`.** A cycle-prompt file
  (`prompts/SPEC-NNN-<cycle>.md`) shares the `SPEC-NNN-*` prefix, so
  `advance-cycle` / `archive-spec` could resolve to it, "succeed" against a file
  with no front-matter, and silently leave the real spec stuck (zany #7). Also
  **`advance-cycle` now hard-errors** when the resolved file has no `task.cycle`
  front-matter instead of no-op'ing with a blank old-cycle. (`scripts/_lib.sh`,
  `scripts/advance-cycle.sh`.)
- **`archive-spec` performs the stage-backlog edit it advertised.** It now flips
  the spec's `- [ ] SPEC-NNN` line to `- [x] … (shipped on DATE)` and recomputes
  the `**Count:**` line, scoped to the `## Spec Backlog` section — the manual
  step every ship used to require, and the single biggest source of ship
  bookkeeping error across three projects (zany #9, crustyimg, bragfile). Falls
  back to a hint if the spec isn't listed. (`scripts/archive-spec.sh`.)
- **Cost-schema drift fixed in the prompts.** The inline cost snippets in
  `FIRST_SESSION_PROMPTS.md` (design/build/ship, both variants) recorded
  `tokens_input`/`tokens_output`, but the `cost-audit` gate and
  `cycle_tokens_total` read a single **`tokens_total`** — so following the
  prompts verbatim guaranteed a `cost-audit` failure (zany #8). Snippets now
  record `tokens_total`, matching `spec.md`, `cost-snippet.md`, AGENTS §4, and
  the gate. (The reporting lib still sums legacy input/output for old specs.)
- **Deterministic project resolution.** `new-stage` / `new-spec` resolved the
  project via `find -name "PROJ-NNN-*" | head -n1`, which silently picked the
  wrong directory when the example and a real project shared a number (zany #1).
  New shared `resolve_project_dir` helper: empty → active project; an exact dir
  name wins; an ambiguous `PROJ-NNN` glob is a **hard error** naming the matches.
  (`scripts/_lib.sh`, `scripts/new-stage.sh`, `scripts/new-spec.sh`.)

### Notes

- +6 test checks (now 149): the cost-snippet drift guard, `find_spec` prompts/
  exclusion (advance-cycle hits the real spec past a planted look-alike),
  `archive-spec` backlog flip + `**Count:**` recompute, and the ambiguous-project
  hard error.
- Raw feedback captured in `feedback/2026-07-03-crustyimg-proj-001.md` and
  `feedback/2026-07-03-zany-animal-slots-proj-001.md`. Deferred, higher-altitude
  threads from the same harvest (a stakes-based lightweight lane; a documented
  sub-agent / delegated-execution mode; contract-tests-as-guards) each warrant
  their own DEC — not folded into this bug-fix batch.
- Already-fixed items were NOT re-flagged (e.g. the `just test` → `template-selftest`
  collision, fixed in v0.5.16).

## 2026-06-19 — Signals registry + close-disposition ritual (v0.5.18)

Closes a structural asymmetry: **coding** lessons had a forcing function
(reflect at ship → codify at a close), but **process/tooling** feedback had only
capture (it landed in `feedback/` and rotted — un-adopted recommendations got
re-flagged months later). Now *every* kind of feedback gets the same teeth.

### Added

- **`guidance/signals.yaml`** (both variants) — one typed feedback ledger, a
  sibling of `constraints.yaml` / `questions.yaml`. Every signal is a record:
  `id, type (lesson|process-debt|product|risk), summary, evidence, bar, status,
  disposition_at, first_flagged, last_touched, raised_by, notes`. It **subsumes**
  the per-stage WATCH convention rather than duplicating it: `type: lesson` is
  dispositioned at a **stage** close and keeps the **N=3 same-outcome / N=2
  paired-opposing** codification bar intact (the running N lives in `evidence` +
  `bar`) — which also fixes the "WATCH items lack cross-stage visibility" gap, by
  putting every queued lesson in one file instead of scattered stage-file prose.
  `process-debt` / `product` / `risk` are dispositioned at a **project** close
  (the previously-missing ritual). Ships with 5 illustrative seeds drawn from
  real dogfooding (incl. the re-flagged-but-un-adopted `lightweight-verify-lane`).
- **The close-disposition ritual** in `FIRST_SESSION_PROMPTS.md` Prompts 1d
  (stage) and 1e (project), both variants: walk every open/watch signal the
  close owns; each gets accept-and-schedule / reject-with-reason /
  defer-with-trigger; **no silent carry**. Capture nudges added to the spec ship
  Reflection Q2, the stage-level reflection, and AGENTS.md "During ship".
- **`just dash signals`** — a new `dash` **lens** (DEC-001 §4: a lens, never a
  new top-level recipe), human + `--json`. Lists signals awaiting disposition
  first. `--json` emits a template-native `signal.*` payload (no ContextCore
  namespace spans all four types — documented like `cost.*`). The open count
  also surfaces in `just dash`'s governance **flags**.
- **`docs/signals.md`** (both variants) — authoring guide: the type table, the
  codification bar, the ritual, capture, and a **migration note** for folding an
  existing per-stage WATCH convention into `type: lesson` entries without losing
  the N-count. Documented in `docs/schema-reference.md`; pointer from AGENTS.md.

### Notes

- Additive (patch): new file + new lens + prompt steps; no existing command,
  output, or artifact changes. Instances are independent forks, so this reaches
  them via new `just init` (or manual cherry-pick) — no automated migration, and
  the seeded file is zero-risk to drop in.
- +9 test checks (now 143): the artifact + guide survive init, the lens (human + `--json`
  with a lesson's bar preserved), the dashboard signals flag, and both close
  prompts wiring the ritual. Enforcement is the ritual, not a CI gate (kept
  lightweight by design).
- Helpers in `scripts/_lib.sh` (`emit_signals_tsv`, `count_open_signals`) feed
  the lens and the flag, mirroring the questions/decisions helpers.

## 2026-06-19 — `dash` governance lenses: decisions + questions (v0.5.17)

Makes two artifacts you create quickly viewable. Per DEC-001 §4 these are
`dash` **lenses, not new top-level commands** — adding a slice is a lens, never
a new `just` recipe (the anti-sprawl rule `dash` exists to enforce).

### Added

- **`just dash decisions`** — browse every `DEC-*` with its confidence,
  active/superseded status, `affected_scope`, and title (⚠ marks confidence
  < 0.7). Complements `just decisions-audit`, which lints rather than lists.
- **`just dash questions`** — open questions from `guidance/questions.yaml`
  (what's blocking), grouped by priority. Skips the `notes:` block.
- **Governance flags in the default `just dash`** — a flags line surfacing the
  open-question count and the number of active decisions at confidence < 0.7,
  so the things that should nag you appear where you already look.
- Both lenses and the default dashboard support `--json`; payloads use
  ContextCore attribute names (`insight.*` for decisions, `guidance.*` with
  `type=question`).
- Shared parsers in `scripts/_lib.sh` (`find_all_decisions`, `get_dec_*`,
  `emit_questions_tsv`, `count_open_questions`, `count_low_confidence_decisions`)
  feed both the lenses and the flag counts.

### Notes

- +6 test checks (now 134): both lenses (human + `--json` with the right command
  names + attribute keys) and the default-dashboard flags (human + `--json`).
- New views are reachable only via `just dash <lens>` — no `just decisions` /
  `just questions` recipe, by design.

## 2026-06-19 — Rename the maintainer self-test recipe → `just template-selftest` (v0.5.16)

### Changed

- **`just test` → `just template-selftest`** (`justfile`) — the recipe that runs
  the template's own maintainer self-test (`scripts/test.sh`) no longer squats on
  `test`, the name an app's real suite (npm/cargo/etc.) expects to own. `test` is
  now free; nothing shadows it. `scripts/test.sh` is unchanged, and CI's
  `cost-data` job is untouched. Updated the dev-loop reference in `CONTRIBUTING.md`
  (dated `docs/sessions/*` logs are left as historical record).

Patch, not minor: the renamed recipe is maintainer-only (it fails early in a
generated instance by design), so no instance/user workflow breaks.

## 2026-06-19 — Accomplishment-logging guidance at ship (v0.5.15)

Tells agents to record the win when something ships, and — more importantly —
how to frame **impact** (outcome, not output). Guidance only; no new dependency.

### Added

- **`guidance/recommended-tools.md` → "Accomplishment logging (at ship)"** (both
  variants) — recommends an accomplishment log (e.g. `brag`) as an optional,
  never-required external tool, with the `brag add` invocation and *when* to log
  (per shipped spec / stage-ship / project-ship). The bulk is **how to think
  about impact**: outcome over output; prefer a metric/quote/unblock; reuse the
  value you already wrote (`value_link`, `value_contribution.delivers`,
  `value.thesis`); pair it with `cost.totals` for a value-per-dollar story; stay
  honest (§17 confidence discipline).
- **AGENTS.md §15 "During ship"** (both variants) — an optional step pointing to
  that guidance, framing the impact from the ship Reflection + `value_link`.

### Notes

- +2 test checks (now 128): the accomplishment-logging guidance survives
  `just init` into an instance (both `recommended-tools.md` and `AGENTS.md`).

## 2026-06-19 — Versioning system + `just template-version` (v0.5.14)

The template now has a real, machine-readable version. (`TEMPLATE_README.md`
previously claimed it had none, while the CHANGELOG tagged `v5.x` — this
reconciles that.) Adopted **honest pre-1.0 semver**: the old `v5.x` tags were a
sequence counter, never semver majors, so the canonical version is restated as
`0.5.14` (a leading `0.` signals the interface is still moving). Prior CHANGELOG
tags stay as historical labels.

### Added

- **`VERSION`** (top-level, semver) — the single source of truth. It survives
  `just init`, so a generated instance reports the template version it was
  scaffolded from.
- **`just template-version`** (`scripts/template-version.sh`) — prints
  `spec-driven-template <version>`; works pre-init (the template) and post-init
  (an instance). `--json` for machine-readable output (Phase 1b envelope).
- **Bump policy** (CONTRIBUTING.md): while pre-1.0 (`0.y.z`), the minor `y`
  bumps on a breaking change (e.g. the DEC-001 Phase 3 command consolidation)
  and the patch `z` on an additive feature or fix; `1.0.0` marks the first
  stable release. Bumping `VERSION` is now a step in the dev loop, and a test
  drift-guards `VERSION` against the newest CHANGELOG entry.

### Notes

- +5 test checks (now 126): semver shape, human + `--json` output, and the
  VERSION↔CHANGELOG drift guard.

## 2026-06-18 — Interface contract Phase 1b: `--json` output + exit-code contract (v5.13)

Completes DEC-001 Phase 1: machine-readable output on the read/dashboard views,
so the front-matter contract is consumable by an MCP server / ContextCore
exporter / UI without scraping. Additive and non-breaking — default human
output is unchanged.

### Added

- **`--json` on `status`, `specs-by-stage`, `roadmap`, `backlog`, and `dash`**
  (and every `dash` lens — `dash now --json` etc. delegate to the underlying
  view). One stable envelope: `{schema_version, command, generated_at, data}`;
  the `data` payload uses ContextCore/OTel attribute names (`task.id`,
  `task.cycle`, `project.stage`, `cost.tokens_total`, `cost.estimated_usd`, …).
  `just dash --json` stitches the status + roadmap reports plus a cost rollup.
- **A JSON toolkit in `scripts/_lib.sh`** — `json_escape` (awk-based; correct
  per the JSON spec for all control chars and UTF-8-safe), `json_qs`,
  `json_obj`, `json_arr`, `json_emit`, `has_json_flag`. Pure bash 3.2, no
  `jq`/`yq`. Resolves the DEC-001 open question on JSON-in-bash.
- **Exit-code contract** (`usage_error`, exit 2): `0` success · `1` gate failure
  (`cost-audit`/`validate`/`decisions-audit`) · `2` usage error. Documented in
  `docs/schema-reference.md`.

### Notes

- +12 test checks (now 121): each `--json` endpoint emits the envelope and
  parses as valid JSON (via `python3` when available), `dash now --json`
  delegates to `status`, and a usage error exits `2`.
- `report-daily` / `report-weekly` keep emitting markdown (a portable artifact
  already); a `--json` variant for the report generators is deferred.
- Building this caught two real bugs: bash-version-fragile param-expansion
  escaping (now awk-based), and a `grep -c … || echo 0` that double-counted on
  empty input (now an awk filter + `wc -l`).

## 2026-06-18 — Interface contract Phase 1a: `just dash` + `just validate` + schema reference (v5.12)

Implements the non-`--json` part of `docs/decisions/DEC-001-interface-contract.md`
Phase 1 (accepted 2026-06-18). Additive and non-breaking — every existing
command and its output is unchanged. (`--json` output is the next increment.)

### Added

- **`just dash`** (`scripts/dash.sh`) — one read command, many lenses, the
  antidote to view sprawl: `dash now` / `next` / `future` / `ledger` dispatch to
  `status` / `backlog` / `roadmap` / `specs-by-stage` (which keep working as
  permanent aliases), and bare `just dash` stitches a single overview (now +
  future + recorded cost + flags). A new slice is a lens here, not a new script.
- **`just validate`** (`scripts/validate.sh`) — the schema gate: fails if any
  spec's front-matter is missing a required structural field (`task.id/type/
  cycle/complexity`, `project.id/stage`, `repo.id`) or has an invalid enum.
  Exits non-zero, CI-suitable. Skips `specs/prompts/` and `*-timeline.md`.
  Cost-on-shipped stays with `cost-audit`; `DEC-*` linting stays with
  `decisions-audit`.
- **`docs/schema-reference.md`** (both variants) — the canonical front-matter
  contract for every artifact (repo-context, brief, stage, spec, decision,
  constraints, handoff), what each gate enforces, and the ContextCore/OTel
  alignment. The front-matter is the repo's public API.

### Notes

- +11 test checks in `scripts/test.sh` (now 109) covering every `dash` lens,
  the stitched dashboard, lens flag-passthrough, and the `validate` gate
  (clean pass, enum failure, prompt-file exclusion).
- Discovered while building `validate`: `find_all_specs` also returns
  `specs/prompts/SPEC-*.md` and `*-timeline.md`; the validator skips both
  explicitly (other commands skip them incidentally via downstream filters).

## 2026-06-17 — Cost-capture gate + license-policy guidance (v5.11)

Ported from a downstream instance of the template (a Rust project built on
it): cost tracking was structurally present but silently went empty (specs
shipped with all-null numerics, and the report lib summed
`tokens_input`/`tokens_output` while specs record a single `tokens_total`).
This makes cost capture real and enforced — a discipline documentation
couldn't keep, made mechanical with a `just` check + a CI job. Plus generic
guidance for the sibling license-policy gate.

### Added

- **`just cost-audit`** (`scripts/cost-audit.sh`, CI job `cost-data`) — fails
  if any *shipped* spec lacks a positive `tokens_total` on its build/verify
  cycles. design/ship (main-loop) cycles may stay null. Surfaced in
  `just status` ("Specs missing cost data") and `just report-weekly`.
- **`.github/workflows/ci.yml`** in both variants — a language-agnostic
  `cost-data` job that runs the gate on every push/PR. Lands at the repo root
  via `just init`; the template repo itself isn't initialized, so it doesn't
  run there. App build/test/lint jobs are left for the project to add.
- **`docs/cost-tracking.md`** — the operational reference: schema, where the
  numbers come from, the enforcement layers, and the (initially empty)
  grandfather list.
- **`docs/license-policy.md`** — optional, per-language license-gate guidance
  with a Rust cargo-deny worked example. The tool is per-ecosystem
  (cargo-deny / pip-licenses / license-checker / go-licenses); the template
  core ships no license tool or `deny.toml`.
- **`projects/_templates/prompts/cost-snippet.md`** — cycle-prompt wording
  that records real `tokens_total` instead of the old "null numerics" line.
- **Constraints** `cost-captured-per-cycle` (warning, enforced) and
  `license-policy` (advisory, opt-in) in both variants' `constraints.yaml`.

### Changed

- **`scripts/_lib.sh`** — `sum_cost_tokens_for_spec` now reads `tokens_total`
  (still sums legacy `tokens_input`/`tokens_output` for forward-compat). New
  audit helpers `is_grandfathered_cost`, `cycle_tokens_total`,
  `spec_missing_cost_cycles`; `COST_AUDIT_GRANDFATHERED` defaults to empty.
- **`scripts/status.sh`** — new "Specs missing cost data" section.
- **`scripts/report_weekly.sh`** — the shipped-without-cost flag now checks
  for null numerics on build/verify, not just whether any session entry exists.
- **`scripts/specs-by-stage.sh`** — cost column per spec, a per-stage subtotal,
  and a grand "Recorded cost" total.
- **`AGENTS.md` §4** (both variants) — rewritten: `tokens_total` schema, no
  null loophole, the capture mechanism, and the enforcement layers.
- **`AGENTS.md` §16 / §13** — the one-worktree-per-concurrent-session habit
  (claude-only Session Hygiene; claude-plus-agents Git Conventions, where two
  agents run concurrently).
- **`projects/_templates/spec.md`** (both variants) — corrected `cost:` block
  comment (record real numbers; null only for un-metered cycles).
- **`scripts/test.sh`** — coverage for the gate (teeth, status surfacing,
  grandfathering, recovery, and the specs-by-stage cost column).

## 2026-06-03 — More blog drafts + SECURITY.md ships downstream (v5.10)

Documentation only — no script or behavior changes.

### Added

- **Two more blog drafts** (`docs/blog/`): "One agent or two" (the
  `claude-only` / `claude-plus-agents` split and session hygiene vs.
  handoffs) and "Two numbers traditional dev hides" (the v5.2 value
  thesis + per-spec AI cost tracking). The blog ideas list is now
  cleared — five drafts total.
- **`SECURITY.md` in both variants**, so a repo created with `just init`
  carries the trust model (local tooling + agents read files and run
  commands; treat externally-sourced content as untrusted; secret
  hygiene). User-facing, with a reporting section to adapt. The template
  root keeps its own maintainer-facing `SECURITY.md`. README's
  post-init file list now mentions it.

## 2026-06-02 — Project docs: usage, security, contributing, blog (v5.9)

Documentation only — no script or behavior changes.

### Added

- **`SECURITY.md`** — threat model (local tooling + the agentic trust
  model where agents read repo files and run `just`), what's been
  hardened, accepted low-severity items, and how to report a vuln.
- **`CONTRIBUTING.md`** — the non-negotiable design principles
  (zero-dependency, bash 3.2, portable shell, escape user input, variant
  parity) and the `just test` dev loop.
- **`PROJECTS.md`** — real projects built with the template (bragfile,
  rspeed) and a note that the template's own history lives in the
  CHANGELOG/git, not in self-specs.
- **`docs/USAGE.md`** — a deeper end-to-end walkthrough than the README:
  the full project → stage → spec → cycle loop with exact commands, the
  four read-only views, and decisions/guardrails.
- **`docs/blog/`** — an index plus three drafted posts (the repo-is-the-
  app philosophy, dogfooding bragfile, zero-dependency tooling) sourced
  from the project history; marked draft for editing into voice.
- README gains a Documentation section linking all of the above.

### Fixed

- **sed-injection hardening in `new-spec` / `new-stage`.** User-supplied
  titles (and the repo id) were substituted into templates via
  `sed "s|<Short Title>|${TITLE}|"` with no escaping. A title containing
  the `|` delimiter could close the s-command early, and a trailing `e`
  would reach GNU sed's execute flag — i.e. command injection. Not
  reachable as shipped (the placeholder sits on a `#`-prefixed markdown
  line, and BSD/macOS sed lacks the `s///e` flag), but it's one template
  edit away on GNU sed and already corrupts files on titles containing
  `|`, `&`, or `\`. Added `sed_escape_replacement` to `_lib.sh` (pure
  bash, escapes `\`, `|`, `&`) and routed the user-controlled
  substitutions through it. Hostile titles now render verbatim instead
  of executing or breaking. (+3 regression checks; 91 total.)

  This was found in a security audit of the template's bash scripts,
  justfile, and CI. No other code findings: `advance-cycle` allowlists
  cycle values, `archive-spec` is awk-only with validated IDs, there are
  no GitHub Actions workflows, and `.gitignore` excludes secrets.

## 2026-06-02 — Recommended-tools catalog + Mermaid convention (v5.7)

Consolidates optional, project-level tool guidance into one catalog and
makes Mermaid the blessed default for diagrams. No new dependencies.

### Added

- **`guidance/recommended-tools.md`** (both variants) — a single catalog
  of optional, project-level tool escalations, organized by concern
  (Diagrams, Testing/Verify, Decisions). States the template's stance
  once — zero-dependency defaults, with "reach for it when / skip it
  when" framing — so adopting any of them is a deliberate `DEC-*`, not a
  default. Covers Mermaid (default), Structurizr (optional C4),
  LineSpec (optional protocol tests), and native `decisions-audit` vs.
  LineSpec provenance.
- **Mermaid as the blessed diagram default.** AGENTS.md (both variants)
  Coding Conventions now states diagrams are Mermaid fenced blocks in
  markdown, updated in the same change as the work. Added a starter
  Mermaid ER diagram to `docs/data-model.md` to match the existing one
  in `docs/architecture.md`.

### Changed

- **Folded `guidance/verify-tooling.md` into `recommended-tools.md`**
  (single source of truth) and repointed the AGENTS.md verify-check and
  Pointers references in both variants. README now mentions the catalog.

## 2026-06-02 — Specs-by-stage ledger (v5.6)

Back-ported and modernized from a downstream project (bragfile000)
built on an earlier template. Fills the one gap the `status` /
`backlog` / `roadmap` trio left: a flat, every-spec ledger.

### Added

- **`just specs-by-stage`** — lists every spec grouped by stage, with
  status, ship date, and complexity. Defaults to **all projects** (a
  historical ledger — unlike `roadmap`/`backlog`, which scope to the
  active project). `--active` (or `--current`) scopes to the active
  project; a `PROJ-NNN` id (or full dir name) scopes to one project.
  Reads authoritative front-matter — `project.stage`, `task.cycle`,
  `task.complexity`, and the `ship` cost session's `recorded_at` for
  archived specs — rather than scraping backlog prose, so it stays
  accurate on the current data model. Also tallies un-promoted
  "(not yet written)" backlog bullets per stage. Read-only.
  (`scripts/specs-by-stage.sh`, `_lib.sh`-based; +5 test.sh checks)

  The original downstream version scraped inline `(shipped on …)` /
  `(S)` annotations out of stage backlogs; this rewrite derives the
  same facts from the fields the template already maintains. Other
  downstream scripts (`claude-code-post-session.sh`,
  `build`/`install`/`uninstall`/`test-docs`) were reviewed and left
  out as project-specific.

## 2026-06-02 — Decision auditing (v5.5)

A native, zero-dependency take on LineSpec-style provenance auditing,
plus a note on where heavier external verify tooling fits. The
template stays dependency-free; the new command is pure bash + awk +
git and works on bash 3.2 (macOS default).

### Added

- **`just decisions-audit`** — audits `decisions/*.md`. Default mode
  lints structure (filename ↔ `insight.id` match, no duplicate IDs,
  required `created_at`/`insight.type`, and bidirectional
  `supersedes`/`superseded_by` integrity — no dangling or one-sided
  links) and warns on scope overlap between active decisions. Exits 1
  on structural errors, so it drops into CI or a pre-commit hook.
  (`scripts/decisions-audit.sh`)
- **`just decisions-audit --changed [BASE]`** — flags which active
  decisions govern the files you're about to commit (working tree +
  staged + untracked, or `BASE...HEAD` with a ref). Advisory (exit 0):
  "re-read DEC-007 before changing this." This is the LineSpec
  `provenance audit` idea done natively.
- **Optional `affected_scope:` field** in the decision front-matter
  (both variants' `decisions/_template.md` and example `DEC-001`) — a
  glob list (`**` spans dirs, `*` within a segment) that powers the
  scope checks. Decisions without it are still linted; they're just
  skipped by the scope passes.
- **`guidance/verify-tooling.md`** (both variants) — documents
  LineSpec for protocol-level integration tests in the Verify phase as
  an *optional, project-level* choice, and explains why the decision
  side is native rather than a dependency.
- AGENTS.md (both variants): the Verify checklist's "decision drift"
  check now points at `just decisions-audit --changed`; Pointers gains
  the verify-tooling note. The build-cycle "Create `DEC-*`" step now
  instructs the agent to fill `affected_scope` for file-bound
  decisions, so the field gets populated at creation time rather than
  rotting empty.

## 2026-04-25 — Backlog and roadmap views (v5.4)

Two read-only views over existing data, answering different
questions at different grains. Together with `just status`, they
form a small "what's the state of work?" trio.

### Added

- **`just backlog`** — spec-grained "what's next" view. Surfaces
  three things `just status` deliberately doesn't: in-flight specs
  (cycle ≠ archived) in the active stage, un-promoted "(not yet
  written)" bullets in the active stage's `## Spec Backlog`, and
  counts of un-promoted bullets in upcoming stages. `--all`
  widens scope across stages. Read-only — no front-matter writes.
  Optional complexity tag (`[S]/[M]/[L]`) parsed if present in a
  backlog line; omitted otherwise. (`scripts/backlog.sh`)

- **`just roadmap`** — stage-grained "where is this project going"
  view. One row per stage with status (shipped / cancelled /
  active / upcoming), date range from existing front-matter
  (`created_at` → `shipped_at` for shipped/active, `target:
  target_complete` for upcoming), and spec counts for active and
  upcoming stages. Active stage row is bolded. (`scripts/roadmap.sh`)

- **`_lib.sh` helpers** for stage front-matter parsing:
  `get_active_stage_file` (lifted from inline use in
  `report_daily.sh`), `get_stage_status`, `get_stage_target`,
  `get_stage_created_at`, `get_stage_shipped_at`. Pure bash + awk;
  null-safe.

- **Both READMEs** mention the two new commands in the
  common-commands block.

- **7 new test assertions** (73 → 80 total): backlog header
  prints, surfaces un-promoted bullets, lists in-flight specs,
  `--all` exits cleanly; roadmap header prints, renders active
  stage with bucket, shows correct spec counts.

### Changed

- **`scripts/report_daily.sh`** uses the shared
  `get_active_stage_file` helper instead of an inline copy. No
  behavior change.

- **Both variants' Prompt 1d (Stage Ship)** gain one numbered step
  instructing the architect to flip `stage.status` to `shipped`
  and set `shipped_at` when wrapping up a stage. This keeps the
  new roadmap accurate without auto-modifying frontmatter from
  `archive-spec.sh` (which `KNOWN_LIMITATIONS.md` explicitly
  documents as deliberate).

### Design notes preserved

- No "accepted" state between bullet and spec. Running `just
  new-spec` is the acceptance.
- Backlog and roadmap stay separate views — one is spec-grained,
  the other stage-grained. Don't merge them.
- No existing front-matter renamed. Roadmap reads what's already
  there (`created_at`, `shipped_at`, `target_complete`).

## 2026-04-25 — Daily status snapshot command (v5.3.1)

Small follow-up. Mirrors a `just daily-status-report` command from a
downstream project (bragfile000) — a thin wrapper that captures
`just status` output to a dated markdown file, distinct from v5.2's
heavier `report-daily`.

### Added

- **`just daily-status-report`** — writes
  `reports/daily/YYYY-MM-DD-status.md` with the current `status.sh`
  output. Lighter than `report-daily`: no curation, no front-matter
  scraping, no git log. Co-located with `report-daily` under
  `reports/daily/`; the `-status.md` suffix distinguishes the two
  artifacts when both run on the same day.
- README mention in both variants' command list.
- Two new test assertions (71 → 73 total): file written at expected
  path, header carries today's date.

## 2026-04-22 — Instruction timeline convention (v5.3)

A small convention, not a mechanism. Every spec gets a peer
markdown timeline file tracking cycle instructions with status
markers. The architect writes cycle prompts to files instead of
leaving them in chat. Executors (build agent, verify reviewer,
shipper) read the prompt file and update the timeline as they go.
No dispatch commands, no MCP servers, no file watchers — just
markdown and discipline.

### Added

- **Timeline file per spec.** Lives at
  `projects/*/specs/SPEC-NNN-<slug>-timeline.md`. Four status
  markers: `[ ]` not started, `[~]` in progress, `[x]` complete,
  `[?]` blocked (with a one-line reason — needs human or external
  unblock; NOT a "I don't know what to do" dumping ground).
  Scaffolded alongside the spec by `just new-spec`, from
  `projects/_templates/timeline.md` (new, in both variants).

- **Per-project prompts directory** at
  `projects/*/specs/prompts/`. Architect writes the next cycle's
  prompt here (`SPEC-NNN-build.md`, `SPEC-NNN-ship.md`); executors
  read from here. Created lazily by `new-spec`.

- **AGENTS.md §9 Instruction Timeline** in both variants.
  Documents all four markers with the onboarding's discipline
  wording. Downstream sections renumbered; cross-references in
  both variants updated.

- **Example artifacts for SPEC-001** in both variants:
  `SPEC-001-example-project-logger-timeline.md` with `[x] design`
  completed and `[ ]` placeholders for build/verify/ship;
  `prompts/SPEC-001-design.md` (retrospective of the design
  prompt) and `prompts/SPEC-001-build.md` (forward-looking build
  prompt). Makes the convention concrete for anyone cloning fresh.

- **14 new test assertions** (57 → 71 total). Covers: timeline
  scaffold at the expected path, legend documents all four
  markers, `prompts/` directory exists, AGENTS.md section present,
  AGENTS.md documents all four markers, archive-spec co-moves the
  timeline into done/.

### Changed

- **`scripts/new-spec.sh`** scaffolds the timeline file + an empty
  `prompts/` directory in addition to the spec.

- **`scripts/archive-spec.sh`** co-archives the spec's timeline
  file into `done/`, keeping history paired.

- **`scripts/_lib.sh`:** `find_spec` now excludes `*-timeline.md`
  (the timeline filename shares the `SPEC-NNN-*` prefix with the
  spec, so the naive glob matched both). New helper
  `find_spec_timeline` locates the paired timeline by ID.

- **Both variants' `FIRST_SESSION_PROMPTS.md`** gain timeline
  instructions across four prompts:
  - 2b (Design): write `prompts/SPEC-NNN-build.md`; replace the
    timeline placeholder with `[x] design` + `[ ]` for later cycles.
  - 3 (Build): mark `[~]` before coding; mark `[x]` with PR/cost/
    date when done; `[?]` only for real blockers needing judgment.
  - 4 (Verify): mark `[~]` before reading; on APPROVED, write
    `prompts/SPEC-NNN-ship.md` and mark verify `[x]` with the SHA.
  - 5 (Ship): mark `[~]` at start; `[x]` with merge date and cost
    before archive.

- **Both variants' `GETTING_STARTED.md`** gain a short paragraph
  + example timeline block in Step 6 (First Spec) explaining the
  convention and reinforcing that the timeline is a dumb markdown
  file with no enforcement.

## 2026-04-21 — Reports, cost tracking, business value (v5.2)

Three bundled features that are tightly coupled: reports need value
structure to tell a project's story; reports need cost data to cover
AI spend; value and cost both live in spec/stage/project front-matter,
so touching those files once for both is cheaper than separate sessions.

Nothing breaks for existing projects — old specs without `cost:` or
`value_link:` advance through cycles and archive as before. Reports
degrade gracefully on pre-v5.2 data. See
`MIGRATION_TO_REPORTS_AND_COSTS.md` for details and optional
backfill blocks.

### Added

- **Business value structure** at project and stage level.
  - `value:` block in project-brief front-matter: `thesis`,
    `beneficiaries`, `success_signals`, `risks_to_thesis`. Testable
    claim, not marketing.
  - `value_contribution:` block in stage front-matter: `advances`,
    `delivers`, `explicitly_does_not`. What this stage advances and
    what it's explicitly not trying to do.
  - `value_link:` scalar on specs. Optional one-sentence reference
    back to the parent stage's value. `null` is acceptable.
  - Applied to both variants' `_templates/` directories in lockstep.
  - `variants/*/projects/_templates/project-brief.md`,
    `variants/*/projects/_templates/stage.md`,
    `variants/*/projects/_templates/spec.md`.

- **Self-reported AI cost** on every spec.
  - `cost.sessions[]` accumulates one entry per cycle (design, build,
    verify, ship). Each entry: `cycle`, `agent`, `interface`,
    `tokens_input`, `tokens_output`, `estimated_usd`,
    `duration_minutes`, `recorded_at`, `notes`.
  - `cost.totals` (`tokens_total`, `estimated_usd`, `session_count`)
    computed at ship.
  - `interface` is a free string. Known values: `claude-code`,
    `claude-ai`, `api`, `ollama`, `other`. Open for future agents.
  - Null numeric fields are fine — reports skip nulls in sums, count
    them in `session_count`.

- **Daily and weekly reports.** Two new commands:
  - `just report-daily` → `reports/daily/YYYY-MM-DD.md`. Sections:
    snapshot (specs by cycle with IDs, project progress), value
    (project thesis, stage advances, value_link population), activity
    today (files touched), cost activity (sessions, WIP cost, specs
    missing cost data), flags (stalled specs, stale decisions), 24h
    git activity.
  - `just report-weekly [YYYY-MM-DD]` →
    `reports/weekly/YYYY-WNN.md`. ISO week; optional date arg for
    back-dated weeks. Sections: summary, value advancement,
    shipped-this-week table, cost breakdown by cycle/interface/top-3,
    decision activity, reflection notes from shipped specs, flags.
  - Both: idempotent (re-run overwrites), graceful on pre-v5.2
    content, deterministic, no daemons.
  - Scripts: `scripts/report_daily.sh`, `scripts/report_weekly.sh`.

- **`scripts/_lib.sh` helpers** for value/cost parsing and portable
  dates. Pure bash + awk + `date`; no `yq` dependency.
  `find_all_specs`, `get_spec_cycle`, `sum_cost_tokens_for_spec`,
  `sum_cost_usd_for_spec`, `sessions_recorded_on`,
  `count_cost_sessions`, `extract_value_link`, `get_project_thesis`,
  `get_stage_value_contribution`, `days_ago`, `iso_week_number`,
  `iso_week_bounds`, `spec_mtime_date`.
  Cost parsers disambiguate `estimated_usd` in sessions vs totals
  by 6-space vs 4-space indent.

- **`reports/`** directory with `daily/` and `weekly/` subdirs;
  sample outputs from the example project committed so users see
  what reports look like before running them.

- **`feedback/`** as a known home for downstream user feedback.
  Rename of the bragfile NOTES file into the dated-slug convention;
  new `_template.md` with front-matter (source, captured_at,
  captured_by, status); `archive/` subdir for addressed/deferred
  items.

- **Prompt updates** across 6 prompts in both variants.
  - 1b Project Brief, 1c Stage Frame, 2b Spec Design: populate the
    new value/cost fields during design.
  - 1d Stage Ship, 1e Project Ship: cross-check shipped
    `value_link`/`value_contribution` against the parent thesis.
  - Prompt 3 Build, 4 Verify, 5 Ship: append cost sessions; compute
    totals at ship; Verify flags specs missing cost data without
    blocking.
  - Prompt 6 Weekly Review: report `value_link` population rate and
    aggregate costs.

- **AGENTS.md sections.** Both variants gained `## 3. Business Value`
  and `## 4. Cost Tracking Discipline` between the Work Hierarchy
  and Tech Stack sections. Cycle-Specific Rules updated to include
  cost-session appends. Downstream sections renumbered;
  cross-references to section 14 updated. `feedback/` and
  `reports/` added to the Directory Structure diagram and Pointers
  list.

- **`MIGRATION_TO_REPORTS_AND_COSTS.md`** at repo root — short,
  leads with "nothing breaks," includes optional backfill blocks.

- **27 new test assertions** in `scripts/test.sh` (30 → 57 total).
  Covers: v5.2 shape in scaffolded specs/stages, AGENTS.md new
  sections, `just report-daily` + `report-weekly` file writing and
  content, idempotency (re-run overwrites), and graceful handling
  of pre-v5.2 data (the critical backwards-compat guarantee).

### Changed

- **`justfile`** gains `report-daily` and `report-weekly` commands.
- **Both variants' README.md** gain a Reports section and the two
  new commands in their common-commands block. Claude-only's
  section-13 cross-reference updated to section 15 to track the
  AGENTS.md renumbering.

## 2026-04-20 — Hardening pass

First polish of the scripts after v5 delivery. Focus was bug-fix only,
exercised on macOS (the original build was tested on Ubuntu). No new
features, no variant dedup, no prompt changes.

### Fixed (follow-on, same-day — reported by downstream user building bragfile)

- **`archive-spec` stage-shipped message no longer falsely claims
  completion.** Archiving the last active spec under a stage used to
  print "All specs for STAGE-X are shipped", which was a false
  positive whenever the stage's Spec Backlog still listed unwritten
  specs. Reworded to "No active specs remain for STAGE-X" — an
  observation, not a completion claim. Stage completion judgment
  stays with the user (and the Stage Ship prompt).
  (`scripts/archive-spec.sh`)

- **Scaffolded specs and stages now pick up the real repo ID.**
  Every template hardcoded `id: my-app` in its `repo:` block;
  `.repo-context.yaml` had a "REPLACE" comment but nothing read
  that file. Even after the user updated `.repo-context.yaml`,
  every new spec/stage still stamped `my-app`. Same fix pattern as
  `__TODAY__`: templates use `__REPO_ID__`, `new-spec`/`new-stage`
  substitute the value parsed from `.repo-context.yaml`
  (`metadata.repo.id`), with `my-app` as the fallback so behavior
  never regresses on a pristine clone.
  (`scripts/_lib.sh`, `scripts/new-spec.sh`, `scripts/new-stage.sh`,
   `variants/*/projects/_templates/*.md`,
   `variants/*/decisions/_template.md`)

### Fixed

- **`just init` no longer silently half-initializes on re-run.** The
  recipe chained steps with `\ ;`, so a failed `cp` would still print
  `✓ Done`, write `.variant`, and leave a broken repo. Now chained with
  `&&` and guarded by a second check that aborts if `variants/` is
  missing. The "already initialized" hint now tells the truth: init is
  one-shot, restore from git or re-clone to start over.
  (`justfile`)

- **`advance-cycle` preserves the cycle-legend inline comment.**
  `update_frontmatter_scalar` used to wipe everything after `:`,
  stripping `# frame | design | build | verify | ship` on first use.
  The updater now preserves any trailing `# …` comment.
  (`scripts/_lib.sh`)

- **`archive-spec` refuses to re-archive a shipped spec.** Running
  `archive-spec SPEC-NNN` twice used to produce `specs/done/done/…`
  because `find_spec` happily returned already-archived files. Now
  `find_spec` excludes `*/done/*`, so both `archive-spec` and
  `advance-cycle` fail loudly with `Spec not found` on archived specs.
  (`scripts/_lib.sh`)

- **`weekly-review` emits repo-relative paths consistently.** Files
  discovered via `find` printed with absolute paths while hand-listed
  files were relative. The prompt promises "paths relative to repo
  root"; this change makes the output match.
  (`scripts/weekly-review.sh`)

- **`YYYY-MM-DD` → today's-date substitution no longer touches comment
  lines.** Templates used the same token for real placeholder values
  and format-documentation comments like `# optional: YYYY-MM-DD`.
  After substitution the comment read like a real target date. Real
  placeholders are now `__TODAY__`; format comments stay as
  `YYYY-MM-DD`.
  (`variants/*/projects/_templates/spec.md`,
   `variants/*/projects/_templates/stage.md`,
   `scripts/new-spec.sh`, `scripts/new-stage.sh`)

- **Removed dangling `just new-project` references.** The die() message
  in `_lib.sh` and the example `brief.md` in both variants pointed
  users at a command that doesn't exist. Replaced with accurate
  instructions (copy `projects/_templates/project-brief.md` into
  `projects/PROJ-NNN-<slug>/brief.md`).
  (`scripts/_lib.sh`, `variants/*/projects/PROJ-001-example-mvp/brief.md`)

### Added

- **`just test` / `scripts/test.sh`** — end-to-end happy-path test that
  spins up a temp copy, runs init + full cycle + archive + weekly-review,
  and asserts the invariants the fixes above depend on. No new deps.
  Intended for template maintainers.
