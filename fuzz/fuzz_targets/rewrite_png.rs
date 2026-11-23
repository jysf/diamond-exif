//! Fuzz target for the PNG strip writer (SPEC-037, DEC-021).
//!
//! `exif_core::strip(bytes, policy)` on a PNG: the first byte selects the
//! policy (0 to 4 the presets, 5 `Custom(none)`, 6 a custom set from the 16-bit
//! mask in bytes 1 and 2, 7 `All`, 8 to 15 a custom set of one category
//! (`OtherSegment`, `Icc`, `Xmp`, `Gps`, `Jumbf`, `FreeText`, `Timestamps`,
//! `Software`), 16 and above `All`); the rest is the PNG candidate.
//!
//! The invariants live in `fuzz/shared/rewrite_png_invariants.rs` (included
//! below, and shared with `tests/fuzz_layout.rs`, which shows each guard going
//! red when its bug is planted). The harness holds an independent naive
//! stripper (its own chunk walk with the walk's caps, its own bitwise CRC-32,
//! its own trailer scan, chunk typing, text keyword table, carrier rule,
//! `eXIf` mode rule and overlap rule), written from DEC-020 and DEC-021 and
//! never from `src/write/png.rs`. It predicts the exact output or the exact
//! refusal, and checks:
//!
//! * on `Ok`: the output equals the input less the predicted removed ranges
//!   (kept chunks byte-identical and in order, a kept bad CRC still bad),
//!   outside the `eXIf` chunks a partial policy may edit; the removal records
//!   equal the predicted ones; the output is never longer than the input,
//!   equal iff nothing was removed; an edited `eXIf` chunk keeps its length and
//!   has a valid CRC; the output's audit has no category of the policy; the
//!   categories the policy keeps have the findings they had; no carrier chunk
//!   is left when the policy removes them; stripping the output changes
//!   nothing; stripping twice gives the same answer;
//! * on `Err`: the class and its detail (the offset, the reason) equal the
//!   prediction (`Unstrippable` only from the naive walk, `Overlap` only from
//!   the naive trailer rule, `Uneditable` only where a partial policy meets a
//!   live `eXIf`); `VerifyFailed` is always a harness failure.
//!
//! This target CANNOT be run with the default PATH (see `audit.rs`): use
//! `just fuzz-smoke rewrite_png`. Seeds are written by
//! `cargo run --example fuzz_seeds`.

#![no_main]

use libfuzzer_sys::fuzz_target;

// `rewrite_png_invariants::check`, shared with `tests/fuzz_layout.rs`.
include!("../shared/rewrite_png_invariants.rs");

fuzz_target!(|data: &[u8]| {
    rewrite_png_invariants::check(data);
});
