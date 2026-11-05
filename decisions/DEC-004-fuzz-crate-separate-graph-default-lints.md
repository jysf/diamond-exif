---
insight:
  id: DEC-004
  type: decision
  confidence: 0.85
  audience:
    - developer
    - agent

agent:
  id: claude-sonnet-5-5
  session_id: null

project:
  id: PROJ-001
repo:
  id: diamond-exif

created_at: 2026-11-05
supersedes: null
superseded_by: null
status: accepted
deciders: [jysf, claude]
affected_scope:
  - "fuzz/**"
  - "deny.toml"
  - "app.just"

tags:
  - fuzzing
  - dependencies
  - lint-policy
---

# DEC-004: Fuzz targets live in a separate `fuzz/` crate, linted at clippy defaults

## Decision

`libfuzzer-sys 0.4` is a dependency of `exif-core-fuzz`, a separate package under `fuzz/` outside the library's cargo graph, so the library's `[dependencies]` stays empty; the fuzz graph is gated by its own `cargo deny` and `cargo clippy` invocations, and its lint scope is clippy's default set under `-D warnings`, not the library's five panic-free lints.

## Context

The constraint `fuzz-target-per-surface` requires a fuzz target from the first parser, which SPEC-002 shipped. `cargo-fuzz` needs the `libfuzzer-sys` macro crate and a nightly toolchain. irradiance made the same call in its DEC-011 (separate fuzz crate) and DEC-027 (fuzz lint scope), after learning that a single `cargo deny` or `cargo clippy` run evaluates only the graph rooted at its manifest, so `fuzz/` silently escaped both gates until each got a second invocation. This DEC carries those lessons over rather than re-deriving them (porting brief §2.4, §3.3).

## Alternatives Considered

- **Option A: `libfuzzer-sys` as a dev-dependency of the library.**
  - Why rejected: puts a C++-building crate (NCSA-licensed vendored runtime) in the library's graph, breaks "no dependencies", and forces nightly onto the root's gates, including the 1.90 MSRV build.
- **Option B: no fuzz crate; property tests only.**
  - Why rejected: does not satisfy `fuzz-target-per-surface` or the 24 h clean-run gate.
- **Option C (chosen): separate `fuzz/` package, path-dependent on the library.**
  - Why selected: the arrow points inward only; the library and its gates are unchanged; consumers never see fuzz dependencies.

## Consequences

- **Positive:** the library keeps zero dependencies; fuzz-only crates cannot reach consumers; the existing `libfuzzer-sys` NCSA exception in `deny.toml` is exactly scoped to this graph.
- **Negative:** every gate that evaluates a cargo graph needs a second invocation: `cargo deny` and `cargo deny --manifest-path fuzz/Cargo.toml`; `cargo clippy` and `cargo clippy --manifest-path fuzz/Cargo.toml --all-targets -- -D warnings` (`just deny-fuzz`, `just lint-fuzz`). Running only the first of each is a green that checked nothing about `fuzz/`.
- **Neutral:** the lint scope for `fuzz/` is clippy defaults under `-D warnings`. The panic-free policy exists to stop attacker-reachable panics in the shipped library; a fuzz harness is dev-only glue and is never shipped, the same category as the `#[cfg(test)]` module exemption. Tightening to the five lints later is a policy change with its own DEC. `cargo fuzz` needs nightly and `~/.cargo/bin` first on PATH (host trap in `guidance/toolchain-brief.md`).
- Every added fuzz dependency is checked by `cargo deny --manifest-path fuzz/Cargo.toml check licenses`; an exception is named in `deny.toml` with its reason.

## Validation

Revisit if a fuzz target needs a dependency whose licence is not permissive, if CI cannot run nightly, or if a harness bug (not a library bug) causes a false crash and the five lints would have prevented it.

## References

- Related specs: SPEC-006, SPEC-002
- Related decisions: DEC-001, DEC-002
- External: irradiance DEC-011, DEC-027 (read-only source); docs/porting-brief-diamond-exif.md §2.4, §3.3
