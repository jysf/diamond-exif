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

# 60 s smoke run of the tiff_ifd target from the committed seeds.
#
# `cargo fuzz` shells out to a bare `cargo`, which resolves to Homebrew's stable
# and rejects `-Zsanitizer=address`; so PATH gets `~/.cargo/bin` first AND the
# outer command is `+nightly`. libFuzzer refuses a corpus dir that does not
# exist, hence the mkdir. Exits non-zero on any crash.
fuzz-smoke:
    mkdir -p fuzz/corpus/tiff_ifd
    PATH="$HOME/.cargo/bin:$PATH" ~/.cargo/bin/cargo +nightly fuzz run tiff_ifd \
        fuzz/corpus/tiff_ifd fuzz/seeds/tiff_ifd -- -max_total_time=60

# ── Oracle (SPEC-005) ────────────────────────────────────────────────────────

# Re-record ExifTool's output for the hand-built fixtures and the tiff_ifd seeds
# into tests/oracle-fixtures/exiftool/. The replay tests read these files with no
# tool installed, so they must be exactly what the installed ExifTool prints:
# re-run this after changing fixtures()/the selector table or upgrading
# ExifTool, and review `git diff`. Needs exiftool on PATH. Never run in CI.
oracle-record:
    cargo run --example oracle_record
