---
# Maps to ContextCore task.* semantic conventions.
# This variant assumes a separate implementer agent — context for the
# implementer lives in handoffs/HANDOFF-*.md, not in the spec itself.

task:
  id: SPEC-001
  type: story                      # epic | story | task | bug | chore
  cycle: build  # frame | design | build | verify | ship
  blocked: false
  priority: medium                 # critical | high | medium | low
  complexity: S                    # XS | S | M | L | XL | XXL — the EXPECTED size, set at design
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
  decisions: [DEC-001, DEC-002, DEC-003]                    # [DEC-NNN, DEC-MMM]
  constraints: [forbid-unsafe-code, panic-free-library, no-io-in-library, license-policy]                  # [constraint-id-1, constraint-id-2]
  related_specs: []                # [SPEC-NNN]

# Blocking dependencies: specs that must SHIP before this one can start.
# Distinct from references.related_specs (informational). Feeds the ready-set
# (`just ready`) and safe fan-out. Optional; [] = no blockers.
depends_on: []

# Fan-out lease — who/what holds this spec now (`just claim` / `just unclaim`).
# Advisory; null = free. The hard lock for parallel agents is the worktree/branch.
claimed_by: null

# One sentence on what this spec contributes to its stage's
# value_contribution. For plumbing: "infrastructure enabling
# STAGE-001's <capability>". Optional; null is acceptable.
value_link: "infrastructure enabling STAGE-001: a gated, licence-clean crate to port the IFD engine into"

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

# SPEC-001: Scaffold the exif-core crate

## Context

STAGE-001 starts from an empty crate. Nothing can be ported until there is a buildable single `exif-core` crate carrying irradiance's quality bar. Plan Step 0 (docs/DIAMOND-EXIF-EXTRACTION-PLAN.md §3). The gate harness (scripts, red-proofs, CI wiring) is repo setup, not product work: it is handled outside the spec backlog and is a precondition here, not a deliverable.

## Goal

A buildable, green `exif-core` single crate: `Cargo.toml`, `#![forbid(unsafe_code)]` plus the five-lint deny set (copied verbatim from irradiance `src/lib.rs:95-102`), `deny.toml`, `LICENSE-MIT` and `LICENSE-APACHE`, an empty `src/` module skeleton, and the `docs/provenance-ledger.md` header (ExifTool is run, never read). `cargo fmt`, `clippy` and `cargo test` pass on the empty lib.

## Inputs

- **Files to read (irradiance, READ ONLY — do not modify it):** `~/PSeven/experiments/crustimg_redo_plus/irradiance/` `src/lib.rs:95-102` (lint block, copy verbatim), `Cargo.toml` (shape, `include` allow-list), `deny.toml` (allow-list), `LICENSE-MIT`, `docs/provenance-ledger.md` (header and rule).
- **Files to read (this repo):** `AGENTS.md` §5-6, `guidance/toolchain-brief.md`, `decisions/DEC-001..003`, `docs/DIAMOND-EXIF-EXTRACTION-PLAN.md` §2, §6.

## Outputs

- **Files created:**
  - `Cargo.toml` — package `exif-core`, lib `exif_core`, edition 2021, `rust-version = "1.90"` (inherited from irradiance, where it was measured; re-measured here by building on 1.90.0), `license = "MIT OR Apache-2.0"`, `include` allow-list (`src/**/*.rs`, `/Cargo.toml`, `/README.md`, `/LICENSE-MIT`, `/LICENSE-APACHE`), **empty `[dependencies]`** (DEC-002).
  - `src/lib.rs` — crate docs (one paragraph: what it is, that it is pre-release), `#![forbid(unsafe_code)]` and the five-lint `#![deny(...)]` block copied verbatim from irradiance `src/lib.rs:95-102`. No modules yet.
  - `Cargo.lock` — committed.
  - `deny.toml` — irradiance's allow-list verbatim (MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2/3-Clause, Zlib, 0BSD, Unicode-3.0) and the named `libfuzzer-sys` NCSA exception; header rewritten for this repo (one graph today; `fuzz/` arrives in SPEC-005).
  - `LICENSE-MIT` (copy of irradiance's, copyright 2026 jysf); `LICENSE-APACHE` (the existing root `LICENSE` moved with `git mv`).
  - `docs/provenance-ledger.md` — header and rule only, no file rows yet. The header must state: ExifTool (Perl, Artistic/GPL) may be RUN as an oracle but never READ to resolve a format question; running a tool imposes nothing, reading copyleft source is a violation; classes 1/2/3 defined as in irradiance's ledger; every ported file gets a row before or when it lands (DEC-003).
  - `tests/crate_policy.rs` — the failing tests below.
- **Files modified:**
  - `app.just` — `lint` becomes `cargo clippy --all-targets -- -D warnings`; add `deny` (`cargo deny check licenses`) and `forbid-unsafe` (`cargo rustc --lib -- -F unsafe_code`); `install` stops calling `rustup show` (no `rust-toolchain.toml`, see Notes).
  - `AGENTS.md` §5 and `guidance/toolchain-brief.md` — drop "pin in `rust-toolchain.toml`"; record `rust-version` and the measured host facts in Notes below.
  - `README.md`, `CHANGELOG.md` — status line and an `[Unreleased] → Added` entry.
- **No new flags, endpoints or public API.** The crate exports nothing.

## Acceptance Criteria

- [ ] `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass on the empty lib.
- [ ] `cargo rustc --lib -- -F unsafe_code` passes (the `-F` re-check).
- [ ] `cargo deny check licenses` passes.
- [ ] Builds and tests on 1.90.0 (see Notes for the exact command on this host).
- [ ] The lint block is shown red once: temporarily adding an `unwrap()` (and an unchecked index) to `src/lib.rs` makes clippy fail, and removing it makes it pass. Record the observed failure text in the handback. (This is the behavioral pre-flight, AGENTS §12; the formal `lint-red-proof` gate arrives later as repo setup.)
- [ ] `[dependencies]` is empty and `Cargo.lock` is committed.
- [ ] `LICENSE-MIT` and `LICENSE-APACHE` exist at the root; no root `LICENSE` file remains.
- [ ] `docs/provenance-ledger.md` header contains the "RUN never READ" statement.
- [ ] `just validate` and `just decisions-audit` still pass.

## Failing Tests

Written during design; the build cycle makes them pass. Integration tests live outside the library crate, so the lint block does not apply to them.

- **`tests/crate_policy.rs`** (std only, reads files relative to `CARGO_MANIFEST_DIR`)
  - `"lib_forbids_unsafe_and_denies_the_five_lints"` — asserts `src/lib.rs` contains `#![forbid(unsafe_code)]` and each of `clippy::unwrap_used`, `clippy::expect_used`, `clippy::indexing_slicing`, `clippy::panic`, `clippy::arithmetic_side_effects` inside a `#![deny(`.
  - `"manifest_is_dual_licensed_with_no_dependencies"` — asserts `Cargo.toml` has `license = "MIT OR Apache-2.0"` and that the `[dependencies]` table is empty.
  - `"both_license_files_exist_and_root_license_is_gone"` — `LICENSE-MIT` and `LICENSE-APACHE` exist; `LICENSE` does not.
  - `"ledger_states_exiftool_is_run_never_read"` — `docs/provenance-ledger.md` exists and contains both the words `RUN` and `READ` in the ExifTool rule.

## Non-Goals

- Gate scripts, red-proofs and CI wiring (repo setup, not a spec)
- Porting any IFD code or tests
- DEC authoring (architect work) and `exif-audit`, fuzz targets, workspace layout

## Notes for the Implementer

- **Design-time measurements (this host, 2026-11-04):** the default `cargo`/`rustc` on PATH are Homebrew 1.99.1 (`/opt/homebrew/bin`) and **reject `+toolchain`**. `~/.cargo/bin/cargo +1.90.0` works (cargo 1.90.0), but `cargo-clippy` on PATH is the Homebrew one, so `~/.cargo/bin/cargo +1.90.0 clippy --version` reports clippy 0.1.99 (the shadowing trap). To run clippy genuinely on 1.90.0 put `~/.cargo/bin` first in PATH (or call `~/.cargo/bin/cargo-clippy` explicitly) and confirm `clippy --version` reports 0.1.90. The rustup default toolchain is nightly; `1.90.0`, `1.95.0` and stable are installed. Re-verify per host; do not carry these numbers as facts elsewhere.
- No `rust-toolchain.toml`: irradiance has none and the shadowing trap above makes a pin misleading. `rust-version` in `Cargo.toml` is the floor.
- Crate name is `exif-core`. Do not publish; availability on crates.io is an open question (`guidance/questions.yaml`).
- Do **not** add `thiserror` or `bytes`; both wait for their own DEC (DEC-002).
- Keep irradiance unchanged; copy text, never link.
- The gate scripts (`lint-red-proof`, `lint-fuzz-red-proof`) and CI Rust jobs are repo setup handled outside this spec.
- One commit per logical step on the spec branch; PR description per AGENTS §13.

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
