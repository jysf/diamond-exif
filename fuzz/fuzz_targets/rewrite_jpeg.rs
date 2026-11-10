//! Fuzz target for the JPEG strip writer (SPEC-020, DEC-013).
//!
//! `exif_core::strip(bytes, policy)` is a new top-level surface (constraint
//! `fuzz-target-per-surface`): for every byte string and every policy it
//! returns, does not panic, and never returns a file that breaks the contract.
//! The first byte selects the policy (0 to 4 the presets, 5 `Custom(none)`,
//! 6 a custom set from the 16-bit mask in bytes 1 and 2, 7 and above `All`); the
//! rest is the JPEG candidate.
//!
//! The invariants live in `fuzz/shared/rewrite_invariants.rs` (included below,
//! and shared with `tests/fuzz_layout.rs`, which shows each guard going red when
//! its bug is planted). The harness holds an independent naive stripper (its own
//! segment walker, benign list, category mapping, MPF reader, overlap and
//! removed-count arithmetic) that predicts the exact output or the exact
//! refusal, and checks:
//!
//! * on `Ok`: the output equals the prediction byte for byte and the removal
//!   and patch records equal the predicted ones; the output is never longer
//!   than the input, equal iff nothing was removed; the scan region is
//!   byte-identical; the output's audit has no category of the policy; the
//!   categories the policy keeps have the findings they had; stripping the
//!   output changes nothing; stripping twice gives the same answer;
//! * on `Err`: the class and its detail (the offset, the categories) equal the
//!   prediction (`NeedsEntryEdit` only when the naive Exif rule says so,
//!   `Overlap` only when an independent overlap exists, `MpfNotIndexed` only
//!   when the naive rule says so, `Unstrippable` only with a halting or
//!   bad-length walk);
//! * `VerifyFailed` is a harness failure, unless the naive prediction itself
//!   breaks an invariant of the self-check or an MPF field would underflow.
//!
//! This target CANNOT be run with the default PATH (see `audit.rs`): use
//! `just fuzz-smoke rewrite_jpeg`. Seeds are written by
//! `cargo run --example fuzz_seeds`.

#![no_main]

use libfuzzer_sys::fuzz_target;

// `rewrite_invariants::check`, shared with `tests/fuzz_layout.rs`.
include!("../shared/rewrite_invariants.rs");

fuzz_target!(|data: &[u8]| {
    rewrite_invariants::check(data);
});
