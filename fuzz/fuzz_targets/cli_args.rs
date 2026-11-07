//! Fuzz target for the `exif-audit` argument parser (SPEC-016, DEC-011).
//!
//! The command line is an input surface: a CI script builds it from variables,
//! and a path may be any bytes. The blocking property, stated executably:
//! **for every argument list, `parse` returns, and its outcome is exactly what
//! the DEC-011 grammar says.**
//!
//! The target includes the real parser, `src/bin/exif-audit/args.rs`, through
//! `#[path]` (so it fuzzes the code the binary runs, not a copy).
//!
//! Input layout: the arguments joined with `0x00`. Empty input is no
//! arguments; a lone `0x00` is two empty arguments. (Unix: the bytes become
//! `OsString`s as they are; elsewhere lossy.)
//!
//! The invariants live in `fuzz/shared/cli_invariants.rs` (shared with
//! `tests/fuzz_layout.rs`, which shows each one going red when its bug is
//! planted) and run as named guards, each against a second naive scan of the
//! argument list typed independently of the parser:
//!
//! * `args_total`: the parser returns and gives the same outcome twice.
//! * `args_help_iff`: `Help` iff `-h` or `--help` is an argument before the
//!   first `--`; otherwise `Version` iff `-V` or `--version` is.
//! * `args_run_shape`: `Run` iff the naive scan accepts the arguments, and
//!   then every field equals its derivation (defaults exactly when an option
//!   is absent, `paths` non-empty and in order, values in their allowed sets).
//! * `args_usage_message`: a usage message is non-empty ASCII with no control
//!   character, at most 300 bytes.
//! * `args_format_conflict`: two format selections never give `Run`.
//!
//! This target CANNOT be run with the default PATH. `cargo fuzz` shells out to
//! a bare `cargo build`, which resolves to Homebrew's stable cargo and rejects
//! `-Zsanitizer=address`. Put the rustup shim first on PATH (or run `just
//! fuzz-smoke cli_args`):
//!
//! ```text
//! PATH="$HOME/.cargo/bin:$PATH" ~/.cargo/bin/cargo +nightly fuzz run cli_args \
//!     fuzz/corpus/cli_args fuzz/seeds/cli_args -- -max_total_time=60
//! ```
//!
//! Seeds are written by `cargo run --example fuzz_seeds`.

#![no_main]

use libfuzzer_sys::fuzz_target;

// The real parser (DEC-011 part 1): the module the harness names `cli_args_src`.
#[allow(dead_code)]
#[path = "../../src/bin/exif-audit/args.rs"]
mod cli_args_src;
// `cli_invariants::check`, shared with `tests/fuzz_layout.rs`.
include!("../shared/cli_invariants.rs");

fuzz_target!(|data: &[u8]| {
    cli_invariants::check(data);
});
