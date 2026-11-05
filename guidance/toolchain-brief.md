# Toolchain Brief

> Per-repo toolchain facts a cold build sub-agent needs. Keep it short and
> current; prune aggressively. Injected into every build prompt (AGENTS.md §15).
> **Status: crate scaffolded (SPEC-001).**

## Package manager

Single Cargo crate (`exif-core`; `exif-audit` bin later); workspace promotion at M2. No `rust-toolchain.toml` (the clippy-shadowing trap below makes a pin misleading); `rust-version = "1.90"` in `Cargo.toml` is the floor. `Cargo.lock` is committed. A new top-level dependency needs a DEC (`no-new-top-level-deps-without-decision`); license policy via `cargo-deny` (permissive-only).

## Test framework + assertion library

Built-in `cargo test` with plain `assert!`/`assert_eq!`. Single test: `cargo test <name>`. Parity tests shell out to ExifTool (`exiftool -T -n -s3`); if it is absent they must print a loud SKIPPED notice and CI must fail. Every parity test must be shown red before it is trusted.

**Oracle (SPEC-005).** ExifTool 13.55 is at `/opt/homebrew/bin/exiftool` on the reference host. `cargo test --test metadata_oracle` runs live parity plus replay of the committed records in `tests/oracle-fixtures/exiftool/`. With ExifTool absent the live tests print `SKIP ...` and pass; `DIAMOND_EXIF_REQUIRE_ORACLE=1` turns that skip into a failure. `just oracle-record` re-records the files (needs ExifTool; review `git diff`). Never run ExifTool over the `two-hop-cycle` seed: unbounded memory growth (about 9 GB in 5 s), see `guidance/questions.yaml`. To check the skip modes, run the built test binary directly with a scrubbed `PATH` (`env PATH=/nonexistent target/debug/deps/metadata_oracle-<hash> --nocapture`), since cargo itself needs `PATH`.

## Lint / format quirks

`cargo fmt --all -- --check`; clippy with the five-lint deny set (irradiance's set verbatim: unwrap_used, expect_used, indexing_slicing, panic, arithmetic_side_effects). `#![forbid(unsafe_code)]` in every crate; CI re-checks with `-F unsafe_code`. Library code must not unwrap/expect/index unchecked on untrusted input.

## Runtime globals / environments

`exif-core` is no-I/O and takes `Bytes` at the API edge; it must stay WASM-trivial (Workers). Only the `exif-audit` bin touches the filesystem/stdout.

## Installed test/dev utilities (don't re-add)

None yet. ExifTool is an external system tool (never linked, never vendored). `cargo-fuzz` (in `~/.cargo/bin`) runs the fuzz targets under `fuzz/`; `cargo-deny` is also in `~/.cargo/bin`.

## Known gotchas

- Gate scripts come from irradiance's `scripts/` (six scripts + `_lib.sh` fixes exist only there), not from this template's `scripts/`.
- The governing docs (`docs/exif-core-design.md`, `docs/porting-brief-diamond-exif.md`) may not be present yet; do not reconstruct them.
- ifd.rs test portability in the porting brief was classified by name/range, not by execution — verify by running.
- **Host toolchain (measured 2026-11-04, re-verify per host):** bare `cargo`/`rustc` are Homebrew 1.99.1 and reject `+toolchain`. Use `~/.cargo/bin/cargo +1.90.0` for the 1.90 check. Homebrew's `cargo-clippy` shadows rustup's, so put `~/.cargo/bin` first in PATH (or call `~/.cargo/bin/cargo-clippy`) and confirm `clippy --version` reports 0.1.90. rustup's default is nightly; 1.90.0, 1.95.0 and stable are installed.
  - Host mutation (SPEC-001 build): 1.90.0 had no clippy/rustfmt component until `rustup component add --toolchain 1.90.0 clippy rustfmt` was run. A fresh host needs that once.
  - With `~/.cargo/bin` first in PATH the rustup default (nightly) has no clippy/rustfmt, so bare `cargo clippy` fails there; use `cargo +1.90.0 ...` (or `RUSTUP_TOOLCHAIN=1.90.0`). With the default PATH, bare `cargo` is Homebrew's and has its own fmt/clippy (0.1.99).
- **Fuzz graph (SPEC-006, DEC-004):** `fuzz/` is a separate cargo graph, so every graph gate needs two invocations, both required: `just deny` + `just deny-fuzz`, `just lint` + `just lint-fuzz`. `just fuzz-smoke` runs the 60 s `tiff_ifd` target. `cargo fuzz` shells out to a bare `cargo` (Homebrew's stable, which rejects `-Zsanitizer=address`), so it must run as `PATH="$HOME/.cargo/bin:$PATH" ~/.cargo/bin/cargo +nightly fuzz ...`. The corpus directory must exist before `cargo fuzz run` (the recipe `mkdir -p`s it). Seeds are regenerated with `cargo run --example fuzz_seeds` (deterministic).
