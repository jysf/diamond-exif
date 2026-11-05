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
