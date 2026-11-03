---
insight:
  id: DEC-006
  type: decision
  confidence: 0.8
status: accepted            # proposed | accepted | superseded
created_at: 2026-06-27
deciders: [jysf, claude]
supersedes: null
superseded_by: null
affected_scope:
  - "variants/*/projects/_templates/release-spec.md"
  - "scripts/new-spec.sh"
  - "scripts/status.sh"
  - "scripts/_lib.sh"
  - "justfile"
  - "variants/*/AGENTS.md"
  - "variants/*/docs/schema-reference.md"
tags: [architecture, process, release, runtime, template]
---

# DEC-006: a release-spec template with a generic runtime/operational pre-flight

> **This is the template's own decision log** (meta). **Status: accepted —
> shipped in v0.5.29.** It is upstream-candidate **B** from the bragfile
> three-project retrospective (`feedback/2026-07-04-bragfile-three-project-retro.md`).
> Complements the design-time **behavioral pre-flight** convention shipped in
> v0.5.24 (AGENTS §12): that is the *general* discipline; this is the
> *release-specific* checklist instance of it.
>
> **What shipped:** `projects/_templates/release-spec.md` (both variants) with
> the six generic pre-flight categories; a `--release` flag on `new-spec` (+ the
> `just new-release-spec` wrapper) that scaffolds it as a `task.type: release`
> spec; `status` recognition (a `[release]` tag in the human view, `task.type`
> in `--json`, via a new `get_spec_type` in `_lib.sh`); an AGENTS "During ship"
> pointer; and `schema-reference.md` documenting the type. **Open questions,
> resolved:** (1) new `release` type over plain `chore` — chose `release` for
> legibility, mirroring how `patch` became first-class (DEC-003). (2) a `--release`
> flag over a separate recipe — chose the flag as the primitive, with a thin
> `new-release-spec` wrapper for ergonomics (just's positional binding makes a
> bare flag awkward on the recipe). (3)/(4) which categories are universal and
> whether to gate — kept all six as a **checklist**, category-level not
> command-level, with a `Delivery shapes` line + per-item `N/A` so a pure web
> service can skip desktop-only OS-trust; **not gated** (`validate` accepts a
> release-spec through the standard spec path but does not enforce checklist
> content) since the checks are host-specific and judgment-laden.

## Context

Across three shipped projects, **every defect that escaped design→build→verify
was operational/runtime**, and the release-phase subclass was especially
consistent: dual-tag-on-the-same-commit, code-signing / Gatekeeper quarantine,
package-manager trust gates, and a dev binary migrating the *production* DB.
Each was **earned in production, then codified after the fact** — and each is
**portable**: any tool that ships an artifact through a release runner and a
package manager will hit the same class.

The template already has the *shape* — "a release cut is its own spec" is an
established precedent — but ships **no release-spec artifact**, so every project
re-discovers the same operational checklist one release at a time. The fix is to
put the checklist in the template so users inherit it.

## Decision (proposed)

Ship **`projects/_templates/release-spec.md`** (both variants): a spec-shaped
template for a release cut whose `## Notes for the Implementer` already carries a
**generic runtime/operational pre-flight checklist**. It reuses the `task.*`
schema (like a normal spec / a patch) so `validate` / `cost-audit` / `status`
treat it as first-class; the architect scaffolds one when cutting a release.

### The generic checklist (categories, not tool commands)

Kept **language- and platform-agnostic** — the categories are universal; the
*instance* fills the tool-specific commands (the same "template ships the slot,
the instance fills the truth" principle as DEC-004's toolchain brief). Each item
is a **behavioral** check (does the released thing actually work on a real host),
not a shape check:

1. **Version / tag integrity** — exactly one release tag points at the release
   commit; the version in the artifact matches the tag; no stale/duplicate tag on
   the same commit.
2. **Artifact trust on a clean host** — a freshly-downloaded artifact is trusted
   by a machine that didn't build it (code-signing / notarization / OS quarantine
   — Gatekeeper, SmartScreen). "Does a stranger's machine run it without a scary
   prompt or a block?"
3. **Distribution-channel trust** — a *new* user can install cleanly through each
   published channel (package-manager tap/registry trust gates, first-install
   auth). "Install it the way a stranger would, on a machine that never saw the
   source."
4. **Data isolation** — a dev/test/CI build can never read or migrate
   **production** data. Confirm the prod data path is unreachable from a non-prod
   binary.
5. **Runtime smoke on a clean host** — the *shipped* artifact runs (not just
   builds) on a fresh install: it starts, answers, and its headline capability
   works end-to-end.
6. **Rollback / uninstall** — there is a known way to withdraw or downgrade the
   release, tested at least once.

### Wiring

- A `just new-spec … --release` flag (or a distinct `just new-release-spec`)
  scaffolds `release-spec.md` instead of `spec.md`; its `task.type` is `chore`
  (or a new `release` type — open question). Everything else (cost, cycles,
  validate) is unchanged.
- AGENTS §15 "During ship" gains one line pointing at the release-spec template
  when a release is the deliverable.

## Alternatives considered

- **A prose checklist in a guidance doc** (not a scaffoldable template) — lower
  friction to add, but it's not *in the flow*; the retro's whole point is that
  users should *inherit* it as the default spec shape, not have to remember a doc.
  Rejected as the primary form (a `guidance/` doc can still back it).
- **Tool-specific checklists** (goreleaser/cargo/Homebrew commands) — higher
  immediate value for one stack, but the template must stay generic; a Rust/Go
  checklist doesn't serve a Node/Python project. Rejected: keep categories
  generic, let the instance fill commands.
- **Fold it into the patch lane** — a release is not a fix to shipped behavior;
  different shape. Rejected.

## Consequences

- **Release gotchas are inherited, not re-earned** — the portable operational
  class is caught before the first public release of each new project.
- **A second scaffoldable artifact type** to maintain (after `patch.md`), and a
  scaffolding flag/recipe. Bounded by reusing the `task.*` schema.
- **The checklist is a living slot** — as more release classes surface across
  projects, the generic categories can grow (via the signals registry, at a
  project close), staying category-level not command-level.

## Open questions

1. **`task.type`** — a new `release` type (first-class in reports/filters) vs
   plain `chore`. Leaning `release` for legibility, mirroring how `patch` became
   first-class (DEC-003).
2. **Scaffolding** — a `--release` flag on `new-spec` vs a separate
   `new-release-spec` recipe. (Prefer a flag; fewer recipes.)
3. **How much is truly universal** — are all six categories portable, or is (2)
   OS-trust really desktop-binary-only (irrelevant to a pure web service)? Maybe
   the template ships the categories and marks which apply to which delivery
   shape (binary / package / service / library).
4. **Relationship to a `just validate` release check** — should any of these be
   *gated* (like cost-audit), or do they stay a checklist? (Prefer checklist —
   most are judgment-laden and host-specific.)
