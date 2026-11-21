//! Fuzz target for the PNG chunk walker (SPEC-028, DEC-017).
//!
//! The blocking constraint `panic-free-library` stated executably: **for every
//! byte string, `Png::parse` and `Png::parse_strict` return, and neither
//! panics.** Any answer, including every `Err`, is acceptable. An abort is not.
//!
//! The target also asserts the walker's invariants, in
//! `fuzz/shared/png_chunk_invariants.rs` (included, and shared with
//! `tests/fuzz_layout.rs`): the walk is re-derived there independently, with a
//! bitwise CRC-32 and its own kind table, and compared field by field, so every
//! iff (CRC match, kind, `after_idat`, cleanliness, the trailer, the
//! end-of-walk issues, the caps) is asserted in both directions.

#![no_main]

use libfuzzer_sys::fuzz_target;

include!("../shared/png_chunk_invariants.rs");

fuzz_target!(|data: &[u8]| {
    png_chunk_invariants::check(data);
});
