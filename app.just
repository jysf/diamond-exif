# app.just — project-owned recipes for diamond-exif (single Rust crate; workspace at M2).
# Spec-artifact gates (reconciled from irradiance) are at the bottom; the Rust
# gates (lint red-proof, fuzz smoke, oracle parity) arrive with the crate.

# Show the active toolchain (no rust-toolchain.toml: `rust-version` in Cargo.toml is the floor)
install:
    rustc --version
    cargo --version

# Run the audit CLI
dev *ARGS:
    cargo run --bin exif-audit -- {{ARGS}}

# Build
build:
    cargo build --release

# Run the test suite (oracle tests skip LOUDLY without exiftool)
test:
    cargo test

# Format check + clippy
lint:
    cargo fmt --all -- --check
    cargo clippy --all-targets -- -D warnings

# License policy (permissive-only dependencies)
deny:
    cargo deny check licenses

# Re-check that no unsafe code exists (the `forbid` attribute in lib.rs, enforced from the CLI too)
forbid-unsafe:
    cargo rustc --lib -- -F unsafe_code

# Type check
typecheck:
    cargo check --all-targets

# ── Gate hardening (reconciled from irradiance; SPEC-016/023, PATCH-002) ─────
# These are template-generic: they gate the spec-driven artifacts, not the app.

# Front-matter PARSE gate red-proof: a grep-satisfiable but parse-broken
# artifact must be REJECTED by name with the parser's own message, and the
# honest tree must stay green. Runs in a temp copy (never writes the tree).
validate-red-proof:
    ./scripts/validate-red-proof.sh

# Stage orchestration-cost gate red-proof: a shipped stage left as the unfilled
# template must be rejected by name; prose/CRLF/fence variants must not satisfy it.
cost-audit-red-proof:
    ./scripts/cost-audit-red-proof.sh

# Gate over the gates: flags the "mute death" shapes (swallowed oracle exit,
# unguarded grep in a command substitution, a verdict with no asserted count).
gate-audit *FLAGS:
    ./scripts/gate-audit.sh {{FLAGS}}

# Proves gate-audit itself goes red on each injected shape (and not on comments).
gate-audit-red-proof:
    ./scripts/gate-audit-red-proof.sh

# ── Fuzzing (SPEC-006, DEC-004) ──────────────────────────────────────────────

# The same licence gate over the FUZZ graph (`exif-core-fuzz`).
#
# Not redundant with `just deny`: `fuzz/` is a separate cargo package and
# cargo-deny evaluates one manifest's graph per run, so `just deny` never sees
# libfuzzer-sys or its dependencies. `--manifest-path` is the whole fix. Keep
# both recipes green before claiming the licence gate passes.
deny-fuzz:
    cargo deny --manifest-path fuzz/Cargo.toml check licenses

# clippy over the FUZZ graph, which `just lint` never compiles.
#
# Same reason as `deny-fuzz`: one manifest per invocation. Scope is clippy's
# DEFAULT lints under `-D warnings`, not the library's five panic-free lints
# (DEC-004: the harness is dev-only glue). Pinned to `~/.cargo/bin/cargo +stable`
# with the PATH prefix because Homebrew's clippy shadows rustup's; it prints
# which clippy answered. No nightly needed: only `cargo fuzz` runs need that.
lint-fuzz:
    @printf 'lint-fuzz: using %s (pinned: ~/.cargo/bin/cargo +stable)\n' \
        "$(PATH="$HOME/.cargo/bin:$PATH" ~/.cargo/bin/cargo +stable clippy --version 2>&1)"
    PATH="$HOME/.cargo/bin:$PATH" ~/.cargo/bin/cargo +stable clippy \
        --manifest-path fuzz/Cargo.toml --all-targets -- -D warnings

# 60 s smoke run of a fuzz target from its committed seeds: `just fuzz-smoke
# [TARGET]` (`tiff_ifd` or `jpeg_segments`); with no TARGET every target runs in
# turn, 60 s each.
#
# PATH TRAP: `cargo fuzz` shells out to a bare `cargo`, which resolves to
# Homebrew's stable and rejects `-Zsanitizer=address`; so PATH gets
# `~/.cargo/bin` first AND the outer command is `+nightly`. libFuzzer refuses a
# corpus dir that does not exist, hence the mkdir. Exits non-zero on any crash.
fuzz-smoke TARGET="":
    #!/usr/bin/env bash
    set -euo pipefail
    export PATH="$HOME/.cargo/bin:$PATH"
    targets="{{TARGET}}"
    if [ -z "$targets" ]; then targets="tiff_ifd jpeg_segments"; fi
    for t in $targets; do
        if [ ! -f "fuzz/fuzz_targets/$t.rs" ]; then
            echo "fuzz-smoke: unknown target '$t'" >&2
            exit 2
        fi
        mkdir -p "fuzz/corpus/$t"
        ~/.cargo/bin/cargo +nightly fuzz run "$t" "fuzz/corpus/$t" "fuzz/seeds/$t" -- -max_total_time=60
    done

# ── Oracle (SPEC-005) ────────────────────────────────────────────────────────

# Re-record ExifTool's output for the hand-built fixtures and the tiff_ifd seeds
# into tests/oracle-fixtures/exiftool/. The replay tests read these files with no
# tool installed, so they must be exactly what the installed ExifTool prints:
# re-run this after changing fixtures()/the selector table or upgrading
# ExifTool, and review `git diff`. Needs exiftool on PATH. Never run in CI.
oracle-record:
    cargo run --example oracle_record

# The red-proofs (SPEC-008): faults injected into a throw-away COPY of src/ and
# corruptions of fixture bytes must turn the comparison with ExifTool's committed
# answers red. Each library-fault test stages, builds and runs small crates
# (control first, then the mutant), so it takes longer than `just test`. Needs no
# ExifTool: only the live variant of the GPS-corruption test uses it and it skips
# loudly without it. The probe builds with the cargo that runs the tests (the
# `CARGO` variable), in a temp directory, offline.
mutation-oracle:
    cargo test --test mutation_oracle

# ── Lint-policy gates (cloned from irradiance, SPEC-001/SPEC-011/SPEC-006 there) ──

# The panic-free lint policy actually bites: control clean, then one violation
# per policy lint injected into a COPIED src/lib.rs must be rejected (DEC-009 there).
lint-red-proof:
    ./scripts/lint-red-proof.sh

# Same for the fuzz graph's `-D warnings` gate: a default-clippy violation injected
# into a COPIED fuzz target must fail with the flag and pass without it.
lint-fuzz-red-proof:
    ./scripts/lint-fuzz-red-proof.sh

# The no-#[allow]-escape check for the library: `-F` (forbid) makes any #[allow] of
# a policy lint a compiler error (E0453). Scope is --lib on purpose: test modules
# legitimately allow the five lints. Mirrors the CI job of the same name.
lint-no-allow:
    cargo clippy --lib --quiet -- \
        -F clippy::unwrap_used -F clippy::expect_used -F clippy::indexing_slicing \
        -F clippy::panic -F clippy::arithmetic_side_effects
