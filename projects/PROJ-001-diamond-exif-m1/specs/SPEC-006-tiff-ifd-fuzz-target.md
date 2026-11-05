---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-006
  type: story                      # epic | story | task | bug | chore
  cycle: build  # frame | design | build | verify | ship
  blocked: false
  priority: medium                 # critical | high | medium | low
  complexity: M                    # XS | S | M | L | XL | XXL — the EXPECTED size, set at design
                                   #   (XL/XXL almost certainly means it's a stage, not a spec)
  complexity_actual: null          # stamped at ship: what it ACTUALLY took, same scale.
                                   #   Expected-vs-actual drift is what `just calibration` reads.
  verify_verdict: null             # approved | punch-list | rejected — the OUTCOME of the verify
                                   #   cycle, stamped by `just advance-cycle` when the spec leaves
                                   #   verify (same three verdicts Prompt 4 already returns).
                                   #   Recorded in front-matter, not just prose, so "verify never
                                   #   rejects anything" stops being a hunch and becomes a number.

project:
  id: PROJ-001
  stage: STAGE-001
repo:
  id: diamond-exif

handoff:
  from_agent: claude-opus-5-5  # from .repo-context tier_map.design (DEC-005)
  to_agent: null                   # filled when HANDOFF is created (any agent — see docs/porting.md)
  created_at: null

references:
  decisions: [DEC-001, DEC-004]                    # [DEC-NNN, DEC-MMM]
  constraints: [fuzz-target-per-surface, panic-free-library, license-policy, no-new-top-level-deps-without-decision]                  # [constraint-id-1, constraint-id-2]
  related_specs: []                # [SPEC-NNN]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: [SPEC-002]

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-001's <capability>". Optional; null is acceptable.
value_link: "infrastructure enabling STAGE-001: executable proof that the parser never panics on hostile bytes"

# Self-reported AI cost per cycle. Each cycle (design, build, verify,
# ship) appends one entry to sessions[]. Totals are computed at ship.
# Record a REAL tokens_total for metered cycles (build/verify) — the agent
# that runs the cycle writes it from its own interface (/cost, the API
# usage object, or its tool's report). Only un-metered main-loop cycles
# (design/ship) may be null-with-note. `just cost-audit` enforces this on
# shipped specs. See AGENTS.md §4 and docs/cost-tracking.md. interface:
# claude-code | claude-ai | api | ollama | other.
cost:
  # Optional PREDICTION of the total tokens this spec will take, set at
  # design. Never a gate — its only job is to be compared with the actual
  # below (`just calibration`), so you learn whether you systematically
  # under- or over-estimate. null = didn't predict.
  tokens_estimate: null
  sessions: []
  totals:
    tokens_total: 0
    estimated_usd: 0
    session_count: 0
---

# SPEC-006: tiff_ifd fuzz target

## Context

Constraint `fuzz-target-per-surface`: each input surface gets a fuzz target from its FIRST parser, not after the policy work. SPEC-002 lands the first parser, so its fuzz target lands next, before SPEC-003 changes the walk. The fuzz graph is a second Cargo graph outside the library (`fuzz/`), which needs `cargo-fuzz`, `libfuzzer-sys` (NCSA exception already in `deny.toml`), a `deny-fuzz` run, and a DEC for the graph layout (irradiance's equivalent is its DEC-011). Ports nearly verbatim from irradiance `fuzz/fuzz_targets/tiff_ifd.rs` (brief §2.4).

## Goal

A `fuzz/` crate with a `tiff_ifd` target that feeds arbitrary bytes to `Container::parse` and exercises the public readers without panicking, plus a 60 s smoke run recorded green, with `cargo deny --manifest-path fuzz/Cargo.toml check licenses` passing.

## Inputs

- **Source to adapt (irradiance, READ ONLY, never modify):** `~/PSeven/experiments/crustimg_redo_plus/irradiance/fuzz/` — `Cargo.toml`, `fuzz_targets/ifd.rs` (the target; note irradiance names it `ifd`, we name ours `tiff_ifd`), `.gitignore`, and the seed approach (`examples/fuzz-seeds.rs` writing valid containers in both byte orders, truncated headers, cycles, absurd offsets, overflowing counts). Recipes: `app.just` (`deny-fuzz`, `lint-fuzz`, the 60 s smoke command). Rationale: irradiance DEC-011/DEC-027, restated in `decisions/DEC-004`.
- **This repo:** `decisions/DEC-004`, `deny.toml` (already carries the `libfuzzer-sys` NCSA exception), `guidance/toolchain-brief.md`, the SPEC-002 public API in `src/tiff/`.

## Outputs

- **Files created:**
  - `fuzz/Cargo.toml` — package `exif-core-fuzz`, `publish = false`, `license = "MIT OR Apache-2.0"` (a missing licence field is an ERROR to cargo-deny), `[package.metadata] cargo-fuzz = true`, `libfuzzer-sys = "0.4"`, `exif-core = { path = ".." }`, one `[[bin]]` `tiff_ifd` (`test = false`, `doc = false`, `bench = false`).
  - `fuzz/Cargo.lock` — committed.
  - `fuzz/fuzz_targets/tiff_ifd.rs` — `#![no_main]`; for any bytes: `Container::parse`; on `Ok`, walk every `ifds()` / `entries()` and call `byte_len`, `payload`, `uints`, `urationals`, `srationals`, plus `scalar`/`values` on `ifd0`. The contract: every entry point returns and none panics; any `Err` is a success. Doc comment states the contract and the exact PATH-first-`~/.cargo/bin` + `+nightly` run command.
  - `examples/fuzz_seeds.rs` — deterministic writer (std only, its own tiny byte builder; examples are not part of the library lint set) that creates `fuzz/seeds/tiff_ifd/*` : valid containers in both byte orders (inline value, external payload, SubIFDs, a chain), truncated header, IFD0 offset zero, entry count past EOF, payload past EOF, `count * size` overflow, self-referencing and two-hop SubIFD cycles, deep nesting. Seeds are committed (own work, a few hundred bytes each).
  - `fuzz/seeds/tiff_ifd/` — the generated seeds, committed. `fuzz/.gitignore` ignoring `target`, `corpus`, `artifacts`, `coverage` (not `seeds`).
- **Files modified:**
  - `app.just` — add `deny-fuzz` (`cargo deny --manifest-path fuzz/Cargo.toml check licenses`), `lint-fuzz` (clippy on the fuzz manifest, `-D warnings`; pinned to `~/.cargo/bin/cargo +stable` with the PATH prefix, printing which clippy answered) and `fuzz-smoke` (the 60 s run below), each with a comment saying why it exists next to its library twin.
  - `AGENTS.md` §6 and `guidance/toolchain-brief.md` — list the three recipes, the nightly + PATH trap, and "two invocations, both required".
  - `deny.toml` — fix the stale "SPEC-005" reference in the `libfuzzer-sys` exception comment to SPEC-006.
  - `docs/provenance-ledger.md` — one row for `fuzz/fuzz_targets/tiff_ifd.rs` and the seeds writer (class 3, irradiance fuzz target/seeds, same author; upstream class 1 TIFF 6.0 for the seed shapes).
  - `CHANGELOG.md` — `[Unreleased] → Added`.
- **Not created here (repo setup, tracked outside the backlog):** CI jobs, and `scripts/lint-fuzz-red-proof.sh`.
- **Library untouched:** `src/` and `[dependencies]` do not change.

## Acceptance Criteria

- [ ] `~/.cargo/bin/cargo +nightly fuzz build tiff_ifd` succeeds (with `PATH="$HOME/.cargo/bin:$PATH"`).
- [ ] The 60 s smoke run — `PATH="$HOME/.cargo/bin:$PATH" ~/.cargo/bin/cargo +nightly fuzz run tiff_ifd fuzz/corpus/tiff_ifd fuzz/seeds/tiff_ifd -- -max_total_time=60` — finishes with no crash, and the handback records the executions count and coverage line.
- [ ] **Shown red:** in a scratch copy, plant a deliberate panic that is reachable only through a hostile shape (e.g. `unwrap()` on `payload` of an entry whose `count` is huge, or an index past a short slice) and show libFuzzer finds and reports it within 60 s from the seeds. Record the crash file name and the panic text. Remove the plant; the tree stays clean.
- [ ] `just deny` and `just deny-fuzz` both pass; `just lint` and `just lint-fuzz` both pass (clippy `-D warnings` on each graph).
- [ ] Library gates unchanged and green: fmt, clippy, `cargo test`, `cargo rustc --lib -- -F unsafe_code`, tests on 1.90.0, `just validate`, `just decisions-audit`. `git diff` shows no change under `src/`.
- [ ] `examples/fuzz_seeds.rs` is deterministic: running it twice produces byte-identical seeds (check with a hash of the directory).
- [ ] The library's `[dependencies]` is still empty; `cargo tree` for the root shows no `libfuzzer-sys`.
- [ ] No irradiance file was modified.

## Failing Tests

The artifact is a harness, so the "tests" are executable checks, written first:

- **`tests/fuzz_layout.rs`** (std only): `"fuzz_crate_is_separate_and_licensed"` — `fuzz/Cargo.toml` exists, declares `license = "MIT OR Apache-2.0"` and a `tiff_ifd` bin, and the ROOT `Cargo.toml` does not mention `libfuzzer-sys`; `"fuzz_gitignore_keeps_seeds_and_ignores_corpus"` — `fuzz/.gitignore` lists `corpus` and `artifacts` but not `seeds`; `"seeds_are_committed_and_cover_the_hostile_shapes"` — `fuzz/seeds/tiff_ifd/` holds at least the named shapes (file names fixed by the seed writer: `valid-le`, `valid-be`, `subifd`, `chain`, `truncated-header`, `ifd0-offset-zero`, `entry-count-past-eof`, `payload-past-eof`, `count-overflow`, `self-cycle`, `two-hop-cycle`, `deep-nesting`).
- **Seeds smoke (in the same file):** `"every_seed_parses_or_errors_without_panicking"` — each seed file is fed to `Container::parse` and, on `Ok`, to the same entry-point walk the target uses; this is the harness contract executed as a plain test, so a regression is caught without nightly.
- The shown-red run above is the behavioural pre-flight (AGENTS §12): shape checks alone are not enough.

## Non-Goals

- CI wiring of the smoke job and the `clippy-fuzz` / `licenses-fuzz` jobs; `lint-fuzz-red-proof` gate script (repo setup)
- The 24 h clean run (M1f gate)
- Fuzz targets for other surfaces, fuzz dictionaries, structure-aware (`arbitrary`) fuzzing
- Any change to library code

## Notes for the Implementer

- Host traps (measured on this machine; re-verify): `cargo fuzz` shells out to a bare `cargo`, which resolves to Homebrew's stable cargo and rejects `-Zsanitizer=address`. Always run with `PATH="$HOME/.cargo/bin:$PATH"` and `~/.cargo/bin/cargo +nightly fuzz ...`. The rustup default here is nightly; `cargo-fuzz` and `cargo-deny` are installed in `~/.cargo/bin`.
- Keep the target's walk in sync with the public API at the time of the spec; SPEC-003/004 will extend it when they add readers (they must update this target in the same change, per "ship the reader with the field").
- The fuzz crate may use `unwrap()` etc. (DEC-004) but the target is written defensively anyway (`let Ok(..) = .. else { return }`).
- Do not add dependencies beyond `libfuzzer-sys` (no `arbitrary` structure-aware work). Do not push or open a PR.

---

## Reflection

*Appended during **ship**. Three questions, short answers.*

1. **What would I do differently next time?**
   — <answer>

2. **Does any template, constraint, or decision need updating?**
   — <answer — if yes but not done this session, record it in
   `/guidance/signals.yaml`: `type: lesson` (with its N-count) for a recurring
   coding pattern, `type: process-debt` for tooling/process friction. A close
   then forces the decision. See `docs/signals.md`.>

3. **Is there a follow-up spec I should write now before I forget?**
   — <answer>

4. **Where was the worst defect caught?** — one word from a fixed vocabulary so
   the defect-escape distribution is greppable across specs:
   `design` | `build` | `verify` | `ship` | `escaped` (reached prod/runtime) |
   `none` (clean first try).
   — <one word>
   *(Runtime/operational defects — the escape-prone class — only exist once the
   artifact meets its real host. `escaped` here is a signal to strengthen the
   §12 behavioral pre-flight for that surface.)*

5. **What can a user do now that they couldn't before?** — one sentence,
   before → after; quote the confirming number if one exists, name the outcome
   if not. Write `none` if this spec has no user-visible outcome — that is a
   real, greppable result, not a blank. This is the line a downstream work-log's
   `impact` field is transcribed from, and both halves are already written above
   (## Context is the before, ## Goal is the after): confirm the prediction,
   don't reconstruct it from memory.
   — <answer | none>
