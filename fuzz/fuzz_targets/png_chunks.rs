//! Fuzz target for the PNG chunk walker and metadata extraction (SPEC-028,
//! SPEC-029; DEC-017, DEC-018).
//!
//! The blocking constraint `panic-free-library` stated executably: **for every
//! byte string, `Png::parse`, `Png::parse_strict` and `png::read` return, and
//! none panics.** Any answer, including every `Err`, is acceptable. An abort is
//! not.
//!
//! The target also asserts two independent models, each in a file included here
//! and shared with `tests/fuzz_layout.rs`:
//!
//! * `fuzz/shared/png_chunk_invariants.rs` re-derives the walk with a bitwise
//!   CRC-32 and its own kind table, so every iff (CRC match, kind,
//!   `after_idat`, cleanliness, the trailer, the end-of-walk issues, the caps)
//!   is asserted in both directions;
//! * `fuzz/shared/png_metadata_invariants.rs` re-derives the extraction from
//!   the raw bytes (its own keyword and header parsing, routing, trailer scan
//!   and position flags) and compares blocks, regions and issues both ways.

#![no_main]

use libfuzzer_sys::fuzz_target;

include!("../shared/png_chunk_invariants.rs");
include!("../shared/png_metadata_invariants.rs");

fuzz_target!(|data: &[u8]| {
    png_chunk_invariants::check(data);
    png_metadata_invariants::check(data);
});
