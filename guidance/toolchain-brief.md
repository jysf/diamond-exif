# Toolchain Brief

> Per-repo toolchain facts a cold build sub-agent needs. Keep it short and
> current; prune aggressively. Injected into every build prompt (AGENTS.md §15).
> **Status: crate scaffolded (SPEC-001).**

## Package manager

Single Cargo crate (`exif-core`; `exif-audit` bin later); workspace promotion at M2. No `rust-toolchain.toml` (the clippy-shadowing trap below makes a pin misleading); `rust-version = "1.90"` in `Cargo.toml` is the floor. `Cargo.lock` is committed. A new top-level dependency needs a DEC (`no-new-top-level-deps-without-decision`); license policy via `cargo-deny` (permissive-only).

## Test framework + assertion library

Built-in `cargo test` with plain `assert!`/`assert_eq!`. Single test: `cargo test <name>`. Parity tests shell out to ExifTool (`exiftool -T -n -s3`); if it is absent they must print a loud SKIPPED notice and CI must fail. Every parity test must be shown red before it is trusted.

## Lint / format quirks

`cargo fmt --all -- --check`; clippy with the five-lint deny set (irradiance's set verbatim: unwrap_used, expect_used, indexing_slicing, panic, arithmetic_side_effects). `#![forbid(unsafe_code)]` in every crate; CI re-checks with `-F unsafe_code`. Library code must not unwrap/expect/index unchecked on untrusted input.

## Runtime globals / environments

`exif-core` is no-I/O and takes `Bytes` at the API edge; it must stay WASM-trivial (Workers). Only the `exif-audit` bin touches the filesystem/stdout.

## Installed test/dev utilities (don't re-add)

None yet. ExifTool is an external system tool (never linked, never vendored). `cargo-fuzz` for fuzz targets under `fuzz/`.

## Known gotchas

- Gate scripts come from irradiance's `scripts/` (six scripts + `_lib.sh` fixes exist only there), not from this template's `scripts/`.
- The governing docs (`docs/exif-core-design.md`, `docs/porting-brief-diamond-exif.md`) may not be present yet; do not reconstruct them.
- ifd.rs test portability in the porting brief was classified by name/range, not by execution — verify by running.
- **Host toolchain (measured 2026-11-04, re-verify per host):** bare `cargo`/`rustc` are Homebrew 1.99.1 and reject `+toolchain`. Use `~/.cargo/bin/cargo +1.90.0` for the 1.90 check. Homebrew's `cargo-clippy` shadows rustup's, so put `~/.cargo/bin` first in PATH (or call `~/.cargo/bin/cargo-clippy`) and confirm `clippy --version` reports 0.1.90. rustup's default is nightly; 1.90.0, 1.95.0 and stable are installed.
