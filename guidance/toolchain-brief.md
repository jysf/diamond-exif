# Toolchain Brief

> Per-repo toolchain facts a cold build sub-agent needs. Keep it short and
> current; prune aggressively. Injected into every build prompt (AGENTS.md §15).
> **Status: crate scaffolded (SPEC-001).**

## Package manager

Single Cargo crate (`exif-core`; `exif-audit` bin later); workspace promotion at M2. No `rust-toolchain.toml` (the clippy-shadowing trap below makes a pin misleading); `rust-version = "1.90"` in `Cargo.toml` is the floor. `Cargo.lock` is committed. A new top-level dependency needs a DEC (`no-new-top-level-deps-without-decision`); license policy via `cargo-deny` (permissive-only).

## Test framework + assertion library

Built-in `cargo test` with plain `assert!`/`assert_eq!`. Single test: `cargo test <name>`. Parity tests shell out to ExifTool (`exiftool -T -n -s3`); if it is absent they must print a loud SKIPPED notice and CI must fail. Every parity test must be shown red before it is trusted.

**Oracle (SPEC-005).** ExifTool 13.55 is at `/opt/homebrew/bin/exiftool` on the reference host. `cargo test --test metadata_oracle` runs live parity plus replay of the committed records in `tests/oracle-fixtures/exiftool/`. With ExifTool absent the live tests print `SKIP ...` and pass; `DIAMOND_EXIF_REQUIRE_ORACLE=1` turns that skip into a failure. `just oracle-record` re-records the files (needs ExifTool; review `git diff`). Never run ExifTool over the `two-hop-cycle` seed: unbounded memory growth (about 9 GB in 5 s), see `guidance/questions.yaml`. To check the skip modes, run the built test binary directly with a scrubbed `PATH` (`env PATH=/nonexistent target/debug/deps/metadata_oracle-<hash> --nocapture`), since cargo itself needs `PATH`.

**JPEG oracle (SPEC-011).** `cargo test --test jpeg_oracle` (live + replay; fixtures in `tests/support/oracle_jpeg.rs`, records `tests/oracle-fixtures/exiftool/jpeg-*.txt` and `jpeg-seed-*.txt`, JPEG records carry `# stderr:` lines). ExifTool prints warnings to stderr only when none of the requested tags was found, so those lines characterise inputs whose fields are all `-`. `just oracle-record` must leave the 41 TIFF records byte-identical (CI diffs them). A new JPEG shape is probed through ExifTool alone, under `perl -e 'alarm 8; exec @ARGV' exiftool ...`, before it joins a loop (no `timeout(1)` on this host).

**Inventory parity (SPEC-013).** `cargo test --test inventory_oracle` compares the audit's inventory (a multiset of `(group, tag id)` plus the set of region kinds) with `exiftool -a -u -G1 -H -s` over five corpora (the hand-built fixtures, the DEC-008 one-entry TIFFs, the three fuzz-seed directories: 249 inputs, 180 distinct by content). The model and the pinned divergences (`KNOWN_DIVERGENCES`, each an exact multiset difference) are in `tests/support/inventory.rs`; the corpora in `tests/support/inventory_corpus.rs`; the committed listings in `tests/oracle-fixtures/exiftool-inventory/` (one file per corpus, `## name` sections of `group<TAB>id<TAB>name` lines, file-level groups removed, `two-hop-cycle` recorded as `# outcome: not run`). Live tests skip loudly without ExifTool; replay needs none. An unknown ExifTool group on any input is a parse error until named in `UNMODELLED_GROUPS`. The probe crate has an `--inventory` mode (`probe::run_probe_inventory`) for the mutants i1 to i6 in `tests/mutation_oracle.rs`. `just oracle-record` also re-records these files; CI's `oracle-records` job diffs both record directories.

**Mutation red-proofs (SPEC-008).** `just mutation-oracle` (`cargo test --test mutation_oracle`). The probe crates are built by the cargo that runs the tests: it reads the `CARGO` variable that `cargo test` sets (falling back to a bare `cargo`), in a temp directory with `--offline` and its own `CARGO_TARGET_DIR` (set explicitly, so a stale one in the environment cannot redirect it into the repo's `target/`). So `~/.cargo/bin/cargo +1.90.0 test` probes with 1.90.0 and the default PATH probes with Homebrew's cargo; do not mix them by hand. Measured on the reference host (Apple silicon, warm): the whole file takes about 3 s, because each debug build of the dependency-free crate is about 1 s and the tests run in parallel. Scrub `PATH` to check the ExifTool-absent behaviour by putting the toolchain's own `bin` first, e.g. `env PATH=$HOME/.rustup/toolchains/1.90.0-aarch64-apple-darwin/bin:/usr/bin:/bin <that cargo> test --test mutation_oracle -- --nocapture`. Every ExifTool spawn is killed after 5 s (`DIAMOND_EXIF_TOOL_TIMEOUT_SECS` overrides); that bounds time only, memory approximately.

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
- **Fuzz graph (SPEC-006, DEC-004):** `fuzz/` is a separate cargo graph, so every graph gate needs two invocations, both required: `just deny` + `just deny-fuzz`, `just lint` + `just lint-fuzz`. `just fuzz-smoke [TARGET]` runs the 60 s `tiff_ifd`, `jpeg_segments` or `audit` target (all three in turn without a TARGET; the seeds for each are in `fuzz/seeds/<target>/`: 29 TIFF, 37 JPEG and 73 for `audit` (the 66 older ones copied under a `tiff-`/`jpeg-` prefix plus 7 audit shapes, one of them zero bytes, which libFuzzer does not load as a file), written by `cargo run --example fuzz_seeds`). The `audit` target includes `fuzz/shared/audit_invariants.rs` (like `jpeg_segments` includes `jpeg_metadata_invariants.rs`); `cargo fmt` does not follow `include!`, so format those files with `rustfmt --edition 2021 <file>`. `just oracle-record` also writes `tests/oracle-fixtures/exiftool/audit-tag-names.txt` (the names ExifTool gives the DEC-008 tag ids). `cargo fuzz` shells out to a bare `cargo` (Homebrew's stable, which rejects `-Zsanitizer=address`), so it must run as `PATH="$HOME/.cargo/bin:$PATH" ~/.cargo/bin/cargo +nightly fuzz ...`. The corpus directory must exist before `cargo fuzz run` (the recipe `mkdir -p`s it). Seeds are regenerated with `cargo run --example fuzz_seeds` (deterministic).
