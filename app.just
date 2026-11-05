# app.just — project-owned recipes for diamond-exif (single Rust crate; workspace at M2).
# The crate does not exist yet (scaffold step); these recipes fail loudly until it does.
# Gate scripts (five-lint deny, forbid(unsafe) -F re-check, fuzz smoke, oracle parity)
# will be cloned from irradiance's scripts/ and wired here as `just gates`.

# Show the pinned toolchain
install:
    rustup show

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
    cargo clippy --all-targets

# Type check
typecheck:
    cargo check --all-targets
