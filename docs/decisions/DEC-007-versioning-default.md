---
insight:
  id: DEC-007
  type: decision
  confidence: 0.8
status: accepted            # proposed | accepted | superseded
created_at: 2026-07-05
deciders: [jysf, claude]
supersedes: null
superseded_by: null
affected_scope:
  - ".repo-context.yaml"
  - "variants/*/.repo-context.yaml"
  - "variants/*/docs/versioning.md"
  - "variants/*/projects/_templates/release-spec.md"
  - "variants/*/AGENTS.md"
  - "scripts/_lib.sh"
  - "scripts/next-version.sh"
  - "justfile"
tags: [architecture, release, versioning, config, template]
---

# DEC-007: a simple default versioning scheme (CalVer), overridable per project

> **This is the template's own decision log** (meta). **Status: accepted —
> shipped in v0.6.1.** Follows the release-spec ([DEC-006](DEC-006-release-spec-template.md)),
> which cuts version tags but assumed a bare `vX.Y.Z` with no defined scheme.

## Context

Two gaps surfaced once the release-spec shipped:

1. **Instances had no versioning convention at all.** The release-spec cut
   `vX.Y.Z` tags by example, but nothing defined how an app should version its
   own releases. Every project would re-decide it.
2. **The `VERSION` file is silently overloaded.** It "survives `just init`" so an
   instance can report *which template version it was scaffolded from*
   (provenance). But a project that also uses a bare `VERSION` file for its own
   app version would collide — two meanings, one file.

The instinct to "just use semver by default" is worth resisting: **semver is a
library / public-API convention.** `MAJOR.MINOR.PATCH` earns its keep when
downstream consumers pin against your API and the number tells them whether an
upgrade breaks them. For the majority shape — apps, services, internal tools,
CLIs — there is no such consumer contract, and semver's per-release "is this
major or minor?" judgment becomes ceremony. The desire was explicitly for a
default that is **simple and just works, with semver available when a project
genuinely needs it.**

## Decision

Ship a **configured, overridable versioning scheme** with a zero-judgment
default.

- **Config:** `spec.version.scheme` in `.repo-context.yaml` (the DEC-005 config
  pattern), one of `calver | semver | monotonic`.
- **Default: `calver`** (`vYYYY.MM.PATCH`, e.g. `v2026.07.0`). The date decides
  the version; the patch just increments within the month. **No "major or
  minor?" call, ever** — which is exactly what "simple and just works" wants,
  and it fits the app/service/CLI majority.
- **Opt in to `semver`** for a library or public API whose version must signal
  compatibility to consumers — chosen by **delivery shape** (the release-spec's
  `binary · package · service · library` line is the guide). `monotonic` (`vN`)
  is the minimal-ceremony third option.
- **`just next-version`** (`scripts/next-version.sh`, `get_version_scheme` /
  `get_next_version` in `_lib.sh`) computes the next tag per scheme from git
  tags. semver can't be auto-bumped, so it prints the current latest and defers
  the level to the human.
- **Resolve the `VERSION` overload by documentation, not a rename:** `VERSION`
  is template **provenance**; the *app* version lives in **git tags** (and/or the
  ecosystem file — `package.json` / `Cargo.toml` / `pyproject.toml`). Stated
  explicitly in `.repo-context.yaml`, `docs/versioning.md`, the release-spec, and
  AGENTS. (A `.template-version` rename was considered and rejected as
  heavier-than-needed; the ambiguity only bites projects that use a bare
  `VERSION` for their app, and a documented convention covers it.)

## Alternatives considered

- **Semver by default** — rejected: imports library API-compatibility ceremony
  into non-libraries, and forces a judgment call every release. It's the *opt-in*,
  not the default.
- **No default / leave it to each project** — rejected: that's the status quo
  that made every project re-decide, and left `VERSION` ambiguous.
- **Rename provenance to `.template-version`** — deferred: structurally kills the
  overload but is a breaking change to the provenance mechanism; documentation is
  proportionate for a narrow trap.

## Consequences

- A fresh instance versions cleanly out of the box (calver) with no decision to
  make; a library flips one config line to semver.
- One more config key + a small helper to maintain, both mirrored across variants.
- The `VERSION`-vs-app-version distinction is now explicit everywhere a release is
  discussed.

## Open questions

1. **Do instances actually reach for `just next-version`, or is the convention
   enough on its own?** If the helper goes unused, it can retire to docs-only.
2. **Enforcement** — should a `validate`/release gate ever check that a cut tag
   matches the configured scheme? Kept a convention for now (host-specific,
   judgment-adjacent — same posture as the release pre-flight itself).
