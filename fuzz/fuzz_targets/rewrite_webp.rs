//! Fuzz target for the WebP strip writer (SPEC-038, DEC-022).
//!
//! `exif_core::strip(bytes, policy)` on a WebP: the first byte selects the
//! policy (0 to 4 the presets, 5 `Custom(none)`, 6 a custom set from the 16-bit
//! mask in bytes 1 and 2, 7 `All`, 8 to 15 a custom set of one category
//! (`OtherSegment`, `Icc`, `Xmp`, `Gps`, `Jumbf`, `FreeText`, `Timestamps`,
//! `Software`), 16 and above `All`); the rest is the WebP candidate.
//!
//! The invariants live in `fuzz/shared/rewrite_webp_invariants.rs` (included
//! below, and shared with `tests/fuzz_layout.rs`, which shows each guard going
//! red when its bug is planted). The harness holds an independent naive
//! stripper: its own RIFF walk with the walk's caps, its own carrier table
//! (typed from the ExifTool measurement, not from the library), its own Exif
//! mode, RIFF size and `VP8X` flag rules, written from DEC-019 and DEC-022 and
//! never from `src/write/webp.rs`. It predicts the exact output or the exact
//! refusal, and checks:
//!
//! * on `Ok`: the output equals the input less the predicted removed ranges
//!   with the RIFF size and the `VP8X` flag patches applied, outside the `EXIF`
//!   chunk data a partial policy may edit; kept chunks are byte-identical and
//!   in order; the removal and patch records equal the predicted ones; the
//!   output is never longer than the input, equal iff nothing was removed; the
//!   output's audit has no category of the policy; the categories the policy
//!   keeps have the findings they had; no removable chunk is left; stripping
//!   the output changes nothing; stripping twice gives the same answer;
//! * on `Err`: the class and its detail (the offset, the reason) equal the
//!   prediction (`Unstrippable` only from the naive walk, `Uneditable` only
//!   where a partial policy meets a live `EXIF` chunk); `Overlap` and
//!   `VerifyFailed` are always a harness failure.
//!
//! This target CANNOT be run with the default PATH (see `audit.rs`): use
//! `just fuzz-smoke rewrite_webp`. Seeds are written by
//! `cargo run --example fuzz_seeds`.

#![no_main]

use libfuzzer_sys::fuzz_target;

// `rewrite_webp_invariants::check`, shared with `tests/fuzz_layout.rs`.
include!("../shared/rewrite_webp_invariants.rs");

fuzz_target!(|data: &[u8]| {
    rewrite_webp_invariants::check(data);
});
