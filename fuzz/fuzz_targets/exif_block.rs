//! Fuzz target for the Exif block builder (SPEC-025, DEC-014, DEC-015).
//!
//! `exif_core::write::exif_block(source, &options)` then `write(base)`: the
//! first two bytes are the base (`u16`, little-endian, made even), the rest is
//! the source. Each input is planned under a small budget, and under the
//! default when the small one refuses.
//!
//! The invariants live in `fuzz/shared/exif_block_invariants.rs` (included
//! below, and shared with `tests/fuzz_layout.rs`, which shows each guard going
//! red when its bug is planted). The harness has its own placer and reads the
//! placed bytes back with the crate's walk, never with the builder's own code:
//!
//! * no panic, and every refusal is typed (`Source`, `Unbuildable`,
//!   `OverBudget`); `write` never fails on a plan that was made, except a base
//!   inside the TIFF header for a non-empty block;
//! * `write(base)` has length `len()`; positions are even and inside the block;
//! * the placed file parses clean with exactly the IFDs the source has, and its
//!   entries equal the source's less `omissions()`;
//! * every omission names an entry that exists in the source;
//! * two runs give equal bytes and omissions.
//!
//! This target CANNOT be run with the default PATH (see `audit.rs`): use
//! `just fuzz-smoke exif_block`. Seeds are written by
//! `cargo run --example fuzz_seeds`.

#![no_main]

use libfuzzer_sys::fuzz_target;

// `exif_block_invariants::check`, shared with `tests/fuzz_layout.rs`.
include!("../shared/exif_block_invariants.rs");

fuzz_target!(|data: &[u8]| {
    exif_block_invariants::check(data);
});
