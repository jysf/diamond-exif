//! Fuzz target for the WebP RIFF walker (SPEC-030, DEC-019).
//!
//! The blocking constraint `panic-free-library` stated executably: **for every
//! byte string, `Webp::parse` and `Webp::parse_strict` return, and none
//! panics.** Any answer, including every `Err`, is acceptable. An abort is
//! not.
//!
//! The target also asserts an independent model, in a file included here and
//! shared with `tests/fuzz_layout.rs`: `fuzz/shared/webp_riff_invariants.rs`
//! re-derives the walk from the raw bytes (its own four-character-code table
//! and 64-bit size arithmetic), so every iff (the size mismatch, the pad byte,
//! kinds, `VP8X`, flag mismatches, cleanliness, the caps) is asserted in both
//! directions.

#![no_main]

use libfuzzer_sys::fuzz_target;

include!("../shared/webp_riff_invariants.rs");

fuzz_target!(|data: &[u8]| {
    webp_riff_invariants::check(data);
});
