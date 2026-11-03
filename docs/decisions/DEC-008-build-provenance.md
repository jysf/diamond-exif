---
insight:
  id: DEC-008
  type: decision
  confidence: 0.85
status: accepted            # proposed | accepted | superseded
created_at: 2026-07-06
deciders: [jysf, claude]
supersedes: null
superseded_by: null
affected_scope:
  - "scripts/build-info.sh"
  - "scripts/_lib.sh"
  - "justfile"
  - "variants/*/docs/versioning.md"
  - "variants/*/projects/_templates/release-spec.md"
tags: [architecture, release, provenance, reproducibility, build]
---

# DEC-008: build provenance — trace every build back to its source commit

> **This is the template's own decision log** (meta). **Status: accepted —
> shipped in v0.6.4.** Pairs with the versioning scheme
> ([DEC-007](DEC-007-versioning-default.md)): DEC-007 answers "what version is
> this?"; this answers "which exact source produced the thing I'm running?"

## Context

A version number alone doesn't tell a user *what they're actually running*. Two
builds can carry the same version; a build can be cut from a dirty tree; an
external recipient of an artifact (or of a report about it) needs to trace it
back to the exact commit to trust or debug it. As the template starts being used
to build projects **for others**, "know exactly what you're looking at" becomes a
trust requirement, not a nicety. The template had no convention for stamping
build provenance into an artifact.

## Decision

Ship a **build provenance stamp** plus the convention for injecting it.

- **`just build-info`** (`scripts/build-info.sh`; `build_ref` / `build_commit` /
  `build_commit_short` / `build_dirty` in `_lib.sh`) emits a `git describe`-style
  ref — nearest tag + commits-since + short SHA, `-dirty` if the tree has
  uncommitted changes — plus the full commit, dirty flag, and build timestamp.
  `--json` for machine consumers. Degrades to `unknown` outside a git repo.
- **The convention: always inject the stamp into the artifact at build time**, so
  the running thing can report its own provenance (`<app> --version` → ref+SHA).
  The template is language-agnostic, so it ships the *stamp*, not the wiring;
  `docs/versioning.md` documents injection per delivery shape (ldflags / a
  generated build-info file / OCI label / a `BUILD_INFO` sidecar).
- **Wired into the release-spec pre-flight:** the tag-integrity category now
  requires that the shipped artifact reports a provenance matching the release
  commit — so it's checked at every release, not assumed.

## Alternatives considered

- **A required generated file the template writes** — rejected: the template
  can't know the target language/build system, and writing a `version.ts` into a
  Python repo is wrong. The stamp + per-shape injection doc is the portable form.
- **Version-only (no commit)** — rejected: that's what DEC-007 already gives and
  it's insufficient for tracing a specific build (dirty trees, same-version
  rebuilds).
- **A hard gate** (fail a release if provenance is absent) — deferred: kept a
  pre-flight checklist item, consistent with DEC-006's posture (host-specific,
  judgment-adjacent). Can promote to a gate if a release ever ships without it.

## Consequences

- Every project inherits a one-command provenance stamp and a documented way to
  bake it into the artifact; external recipients can trace a build to source.
- Reports and artifacts can carry the same commit ref, tying a deliverable to its
  exact source (the external-reports direction).
- One more small script + four `_lib` helpers to maintain (all git-only, no deps).

## Open questions

1. **Should reports embed the commit ref by default?** (A daily/weekly report that
   goes external should say which commit it reflects.) Cheap to add to the report
   header + `--json` envelope; deferred to the external-reports work so its shape
   is driven by a real need.
2. **Gate vs checklist** — as with DEC-006, revisit if a release ever ships
   without an injected stamp.
