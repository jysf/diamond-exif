---
insight:
  id: DEC-004
  type: decision
  confidence: 0.7
status: accepted            # proposed | accepted | superseded
created_at: 2026-06-27
deciders: [jysf, claude]
supersedes: null
superseded_by: null
affected_scope:
  - "variants/*/AGENTS.md"
  - "variants/*/guidance/constraints.yaml"
  - "variants/*/guidance/toolchain-brief.md"
  - "variants/*/FIRST_SESSION_PROMPTS.md"
  - "variants/*/projects/_templates/spec.md"
  - "scripts/_lib.sh"
tags: [architecture, process, methodology, sub-agents, delegation, orchestration]
---

# DEC-004: a documented sub-agent / delegated-execution mode

> **This is the template's own decision log** (meta). **Status: accepted.**
> **Phase 1 shipped in v0.5.27:** rules 1–3 (reconcile-over-self-report + the
> die-mid-cycle recovery, the one-sub-agent shared-tree discipline, and explicit
> per-cycle model config consuming DEC-005's `tier_map`) are documented as a
> "Delegated execution" section in both variants' `AGENTS.md`. **Phase 2 shipped
> in v0.5.28:** rule 4 (the `no-new-top-level-deps-without-decision` constraint
> now sanctions a clearly-trivial DEV-only dep + its DEC in one build pass) and
> rule 5 (`guidance/toolchain-brief.md` — the per-instance toolchain-brief slot,
> referenced from AGENTS.md "During build" / Pointers / the directory diagram);
> the Delegated-execution section now carries all five rules. **Still deferred:**
> Phase 3 (mechanical per-agent `git worktree` isolation) — rule 2's "one
> sub-agent, no interleaved tree ops" covers the hazard as convention, and open
> question #1 (is mechanical worktree management worth the bash-3.2 complexity?)
> is unresolved, so it stays deferred, not dropped. The non-Claude portability
> track (§5) shipped as [DEC-005](DEC-005-agent-portability.md); the patch lane
> ([DEC-003](DEC-003-patch-lane.md)) inherits the reconcile rule.
> On the helper open question (#2): a `_lib.sh` reconcile helper was judged
> low-value — the rule ships the exact `git log`/`git ls-remote` commands, which
> is the mechanical part; a wrapper adds little.

## Context

The template's cycle model assumes **one interactive agent** moving a spec
through design → build → verify. But in practice both shipped dogfood projects
delegated build/verify to **fresh sub-agents** — and, notably, **both did so
under the `claude-only` variant** (local Agent-tool sub-agents in the shared
checkout), not just `claude-plus-agents`. That delegation surfaced a failure
class the template neither documents nor guards. The structured harvests rank it
the **"richest new learning"** (zany) and the **biggest strategic gap** — the
bragfile round-1 feedback flagged "design for multi-agent variants" back on
2026-04-20, and it was never dispositioned until now.

Evidence (from the crustyimg + zany-animal-slots signals harvests):

| Signal | N / pattern | What breaks |
|---|---|---|
| **`trust-git-disk-over-self-report`** | N=6, same-outcome (unanimous) | A build/verify sub-agent returns a **truncated / mid-task self-report** with the gate, tests, or commit still missing; a stale timeline marker claims work that isn't on the branch. Advancing on the self-report ships a lie. (SPEC-004 truncation; SPEC-036 killed mid-run.) |
| **`shared-tree-subagent-corruption`** | N=4 | Agent-tool sub-agents run in the orchestrator's checkout and auto-background; interleaving orchestrator git/tree ops corrupts a branch (a design commit landed on the wrong branch, recovered via cherry-pick + reset). |
| **`build-cycle-cant-author-dec`** | N=5, paired-opposing | A non-interactive build sub-agent **can't pause to author a DEC**, so `no-new-top-level-deps-without-decision` pushed it to a workaround review had to undo (`@types/node` stub, SPEC-002). Counter-example: a pre-authorized DEC (`tone` at design) let build add the dep with no stop-and-ask. |
| **`cold-agent-toolprior-drift`** | N~10 | Every cold sub-agent re-imports model tool-priors that mismatch a lean toolchain (react-hooks disable with no plugin; `@testing-library/user-event` not installed; `scripts/*.mjs` failing `no-undef` on Node globals) — individually trivial, cumulatively ~10 wasted loops. |
| **`set-subagent-model-explicitly`** | N=1, high-cost | A build sub-agent silently defaulted to Opus (~6× the intended Sonnet) because the model wasn't set explicitly. |

The portable through-line: **the spec-as-source-of-truth design already
generalizes** (it's why a fresh sub-agent can pick up at all), and
`trust-git-disk-over-self-report` is **agent-agnostic**. What's missing is the
orchestration discipline around delegation, and the template slots to carry
repo-specific truth a sub-agent lacks.

## Decision (proposed)

Adopt a **documented sub-agent / delegated-execution mode** — a named set of
orchestration rules + template slots, applicable whenever build/verify is
delegated to a fresh sub-agent (both variants). Five mechanisms:

> **This builds on the existing `HANDOFF-*` artifact, it does not replace it.**
> The `claude-plus-agents` variant already has the delegation contract — a
> handoff with `handoff.to_agent` (already values like `kilo-code`,
> `factory-droid`) and `handoff.status: pending → accepted → completed |
> rejected`. This DEC adds the *orchestration discipline* around that contract:
> the rules below are what must hold **before an orchestrator flips
> `handoff.status` to `completed`** (or, in `claude-only`, before it advances the
> spec's `task.cycle`). The handoff is the "what"; this DEC is the "how you trust
> it."

### 1. Reconcile over self-report (the load-bearing rule)

**Never advance a cycle — or mark a handoff `completed` — on a sub-agent's
self-report alone.** After a build/verify sub-agent returns, the orchestrator
reconciles the *claimed* result against actual **`git log` + disk state** (branch
exists, commit present, gate actually ran, the spec's `## Failing Tests` files
exist) before advancing. This generalizes the contract's existing "trust git over
timeline markers" to "trust git/disk over **any** agent self-report." Promote to
AGENTS.md as a first-class orchestration rule (agent-agnostic, highest value). A
small `_lib.sh` helper can assert the mechanical parts (e.g. "commit on the
expected branch, spec files present").

**Includes a named recovery procedure.** Sub-agents die mid-cycle (API overloads,
kills) — observed twice per run in both dogfood projects. When they do, the
orchestrator: (a) reconciles the partial output against disk/git; (b) finishes the
**mechanical remainder** in the main loop (never re-runs the whole cycle); and
(c) attributes cost to the sub-agent's **metered portion** (its `subagent_tokens`),
recording the main-loop finish as a separate null-with-note session. This turns
an ad-hoc save into a documented, cost-honest step — and is the same reconcile
check, applied to a partial rather than a complete result.

### 2. One sub-agent, no interleaved tree ops — with worktree isolation as the fix

**Rule (cheap, now):** launch exactly **one** build/verify sub-agent, then do
**no** git/tree operations (no `new-spec`, `checkout`, commits, and don't design
the next spec) until it reports complete and its branch is merged. **Structural
fix (better):** give each concurrent sub-agent its **own `git worktree`** so the
shared-tree hazard disappears — this is the §16 "one worktree per concurrent
session" habit made mandatory for delegated execution and, ideally, mechanical.

### 3. Explicit per-cycle model configuration

Make the sub-agent's model an **explicit, recorded** choice, not a default. The
design=Opus / build=Sonnet split is a real cost lever, but a silent Opus default
is a ~6× surprise. Record it in the spec's `agents.*` and the cost session's
`agent`. (This is also the seam where non-Claude portability plugs in — §5.)

### 4. A sanctioned trivial-dev-dep + DEC-in-one-pass path

Let a build cycle **add a clearly-trivial dev dependency** (types packages, test
utilities) **and author its DEC in the same pass**, rather than forcing a
stop-and-ask a non-interactive sub-agent can't do (which drives it to a
workaround). Alternatively, **pre-provision** the obvious test-time dev deps in
the scaffold spec. Bound it tightly (dev-only, no runtime deps) so the
`no-new-top-level-deps-without-decision` constraint keeps its teeth for real
choices.

### 5. A per-instance "toolchain brief" slot

The template **can't** encode repo-specific toolchain truth (which ESLint plugins
exist, whether `user-event` is installed, that `scripts/*.mjs` run under Node) —
those are per-repo facts. But it can provide the **slot**: a required, per-instance
**toolchain brief** that build prompts inject, so cold sub-agents stop re-deriving
the same mismatches. The template ships the slot + the discipline; the instance
fills the truth.

## Scope & rollout (proposed)

- **Both variants.** The failure class appeared in `claude-only` running
  sub-agents, so this is not `claude-plus-agents`-only; the shared-tree rule and
  reconcile rule belong in both AGENTS.md files (claude-only §16 Session Hygiene;
  claude-plus-agents §13 coordinator discipline).
- **Additive, phased.** Phase 1: document rules 1–2 in AGENTS.md + the
  reconcile helper (pure discipline + a tiny lib assert). Phase 2: the
  dev-dep+DEC path (rule 4) and the toolchain-brief slot (rule 5). Phase 3
  (optional): mechanical per-agent worktree isolation.

## §5 — Relationship to non-Claude portability

Rules 1, 2, 4, 5 are **agent-agnostic** (and the handoff's `handoff.to_agent`
field is *already* agent-agnostic — `kilo-code`, `factory-droid`). Rule 3 (model
config) is the seam that couples to Claude today: model-ids
(`claude-opus-4-8`/`claude-sonnet-4-6`), the `$/M` rate, and the metering source
(`subagent_tokens` / `/cost`) are all Claude-specific, and `cost-audit` is a hard
gate with **no source** on another platform. Both harvests independently name
this. This is now [**DEC-005**](DEC-005-agent-portability.md) (proposed) — it
parameterizes model-id, the `$/M` rate, and the metering source behind config and
makes the model-tier map pluggable; **rule 3 here consumes that config rather than
re-specifying it.**

## Open questions

1. **Worktree isolation in bash 3.2 — and is bash the right runtime for it?**
   Is mechanical per-agent worktree management worth the complexity, or is the
   "one sub-agent, no tree ops" rule enough in practice? (Rule 2 cheap vs Phase 3
   structural.) **Direction confirmed 2026-07-12** — Phase 3 is the right guidance;
   a second axis raised the same day: if it *is* built, the isolation helper need
   not be bash. A small compiled single-binary runtime (e.g. **Zig**, or another
   portable option) could own the worktree lifecycle more safely than bash 3.2, at
   the cost of a build/distribution step. Sub-open-question: does a non-bash
   component break the "zero-dep bash 3.2" portability promise, or is a
   vendored/optional binary an acceptable exception for this one structural piece?
2. **How much to build vs document** — the reconcile rule is mostly discipline;
   how much does a `_lib.sh` helper actually catch mechanically?
3. **Gate or convention?** Should "reconcile before advancing" ever be a check
   (like `cost-audit`), or does its judgment-laden nature keep it a convention?
4. **Dev-dep boundary** — what exactly counts as "clearly trivial" (dev-only?
   types/test-only?) so rule 4 doesn't erode the deps constraint.
5. **Environment reliability is out of scope but real** — background-dispatched
   sub-agents sometimes can't obtain a Bash permission at all, and overloads are
   frequent. The template can't fix the harness; it can only assume less (the
   reconcile + recovery procedure above is the mitigation). Worth naming so it
   isn't silently assumed away.
