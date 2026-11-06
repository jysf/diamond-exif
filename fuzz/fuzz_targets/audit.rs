//! Fuzz target for the audit model (SPEC-012, DEC-008).
//!
//! `audit(bytes)` is a new top-level surface (constraint `fuzz-target-per-surface`):
//! the blocking constraint `panic-free-library` stated executably is that
//! **for every byte string, `audit` returns, and does not panic.** Any answer is
//! acceptable except a violated invariant.
//!
//! The invariants live in `fuzz/shared/audit_invariants.rs` (included below, and
//! shared with `tests/fuzz_layout.rs`, which also shows each of them going red
//! when its bug is planted). The audit is re-derived there independently (the
//! magics, the DEC-008 table, the severities and the thumbnail rule are written
//! again; `sniff` and `categorize` are never called) and compared with the
//! library's answer. Every rule that is an iff is checked in BOTH directions:
//!
//! * `kind()` equals the re-derivation from the first bytes; `Unsupported`
//!   implies no items, no findings, no severity and `!clean`; `clean` implies a
//!   supported container.
//! * `items().len()` and `findings().len()` are at most `MAX_INVENTORY_ITEMS`;
//!   a finding without an item index only when the inventory is truncated and
//!   full.
//! * Every `Finding.item` that is `Some(i)` is a valid index and `items()[i]` is
//!   the finding's item; Region categories only on Region items; `Gps` iff the
//!   item is a tag of a GPS IFD; a thumbnail range accompanies exactly the
//!   `Thumbnail` category.
//! * The inventory and the findings equal the re-derived ones, in order.
//! * A `Gps` finding exists iff a GPS-kind tag item exists.
//! * A `Thumbnail` range lies inside the TIFF blob of its Exif block, and its
//!   FF D8 flag holds iff the bytes there start FF D8.
//! * `clean` iff the container was readable with zero issues, zero failed
//!   blocks and no truncation; the issue and failed-block counts equal the
//!   readers' own.
//! * `max_severity()` is the maximum over every finding found, `None` iff none.
//!
//! This target CANNOT be run with the default PATH. `cargo fuzz` shells out to
//! a bare `cargo build`, which resolves to Homebrew's stable cargo and rejects
//! `-Zsanitizer=address`; the outer `+nightly` does not help. Put the rustup
//! shim first on PATH (or run `just fuzz-smoke audit`):
//!
//! ```text
//! PATH="$HOME/.cargo/bin:$PATH" ~/.cargo/bin/cargo +nightly fuzz run audit \
//!     fuzz/corpus/audit fuzz/seeds/audit -- -max_total_time=60
//! ```
//!
//! Seeds are written by `cargo run --example fuzz_seeds`.

#![no_main]

use libfuzzer_sys::fuzz_target;

// `audit_invariants::check`, shared with `tests/fuzz_layout.rs`.
include!("../shared/audit_invariants.rs");

fuzz_target!(|data: &[u8]| {
    audit_invariants::check(data);
});
