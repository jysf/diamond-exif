---
source: "bragfile cross-project retrospective — 3 projects / 9 stages / 42 specs / 2 releases (claude-only)"
captured_at: 2026-07-04
captured_by: claude
status: open                # open | addressed | deferred
---

# bragfile three-project retro — template feedback capture (triage)

Full source: `github.com/jysf/bragfile000/blob/main/docs/framework-feedback/2026-07-04-three-project-retro-feedback.md`
(PR #64; underlying retro report in the same PR). Second installment after the
2026-04-20 8-spec note. Much larger sample: 40/42 specs shipped, every stage in
order, 25 DECs with exactly one supersession (fired on its own triggers), zero
design→ship drift, mean decision confidence 0.823 with no 1.0s.

## Headline: the discipline is validated at scale — DON'T speed up or codify sooner

The retro's primary message is a **keep**: the deliberate-lag codification
(N=3-same / N=2-paired-opposing bar), confidence discipline, PEEL-IF-L, the
premise-audit family, and literal-artifact-as-spec are "the numbers of a process
operating as designed." Explicit ask: **do not push the template to move faster
or codify sooner.**

> Cross-validates the template's v0.5.18 **Signals registry** — the WATCH →
> codification pipeline + the N=3/N=2 bar the retro calls "the template's best
> feature" is exactly what the signals registry generalized (and preserved).
> NOTE: §9 premise-audit, §12(b) pre-flight, PEEL-IF-L, and the §-numbers are
> **bragfile-local conventions — not shipped by the template.** Verified: zero
> hits for premise-audit/pre-flight/literal-artifact in `variants/`.

## The one structural gap (actionable): runtime/operational coverage

design→build→verify is dense on **spec-logic** correctness and sparse on
**runtime/operational** behavior. Every defect that escaped a cycle across all
three projects was operational/runtime, not logic: a timezone/day-boundary bug
(streak read 0), goreleaser dual-tag, macOS Gatekeeper, Homebrew tap-trust, a dev
binary migrating the *production* DB, and a plugin that registered 0 MCP servers
despite its manifest validating `--strict`.

### A — behavioral pre-flight (a GAP in the template, not a refinement)

> When a spec's literal makes a claim about *runtime behavior* — a component
> registers, a hook fires, a binary resolves on PATH, a server answers — the
> pre-flight must run the literal through the surface that *exercises that
> behavior*, not merely the surface that *validates its shape*.

Canonical opposing pair: SPEC-024 ran cobra's real `GenBashCompletion` (behavioral,
caught a marker mismatch at design); SPEC-041 ran `claude plugin validate --strict`
(shape-only) but not `claude plugin details` (registration) → a manifest that
validated still registered zero servers. Portable to any project emitting a
manifest/config/registration artifact. **The template teaches NO design-time
pre-flight today**, so this is an addition, not a generalization.

### B — a release-spec template with a runtime/operational pre-flight checklist

Every §4 release gotcha (dual-tag-on-same-commit, code-signing/Gatekeeper,
package-manager trust gates, dev/prod data isolation) was earned in production
then codified after the fact, and is largely portable. Ship a release-spec
template whose "Notes for the Implementer" already carries a runtime/operational
pre-flight checklist so users inherit the lessons. (The template has no
release-spec artifact today; keep any checklist GENERIC, not Rust/Go-specific.)

## Smaller portable additions

- **Defect-catch-stage tag on ship reflections** (design | build | verify | ship |
  escaped) so the **defect-escape distribution** — "where do defects actually get
  caught, and what escapes?" — is cheap to compute across projects. Clean, small,
  high-value; only visible in a cross-project view.
- **"Reserved but not wired"**: when a DEC reserves a capability for later, prompt
  for a paired "how will we know it's actually being used?" observability line —
  else reservations become invisible debt (DEC-024 reserved a provenance namespace
  with zero entries populating it yet).
- **Cross-project retrospective as a template recipe/skill** (read-only worktree +
  parallel per-project extraction → synthesis). Meta; larger; defer.

## Nits (verified against the current template)

- **archive-spec false "all specs shipped"** (from the 2026-04-20 note) —
  **ALREADY FIXED**: reworded to "No active specs remain for STAGE-X" + v0.5.19
  makes archive-spec edit the backlog. Do not re-flag.
- **Brief `status:` resolver comment-sensitivity** — the template's resolver uses
  `awk … {print $2}`, which ignores a trailing `# comment`, so it is **not
  reproducible in the template** (likely a bragfile-local resolver). Low priority.

## Disposition (proposed)

- **Ship** (small, exactly what the retro asks for): the **defect-catch-stage tag**
  + the **behavioral pre-flight guidance (A)**.
- **Propose** (bigger, needs a generic-vs-flavored design call): the **release-spec
  template (B)**.
- **Defer:** the reserved-but-not-wired prompt and the cross-project-retro recipe.
- **Protect (no change):** the codification-lag + N=3/N=2 bar + confidence + PEEL +
  premise-audit family — validated at scale.
