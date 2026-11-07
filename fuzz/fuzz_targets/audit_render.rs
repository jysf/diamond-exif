//! Fuzz target for the report renderers (SPEC-015, DEC-010).
//!
//! A report is built from attacker-controlled bytes (paths, tag value excerpts,
//! failure messages) and read by a CI job (JSON), a spreadsheet (CSV) and a
//! terminal (text). The blocking property, stated executably: **for every
//! input, the three renderers return, are deterministic, and their output is
//! well-formed.**
//!
//! Input layout: byte 0 selects the options (low 3 bits: no policy, `Gps`,
//! `GpsAndDevice`, `AllPii`, `All`, `Everything`, a `Custom` set derived from
//! byte 1, no policy; bit 3 `inventory`; bit 4 `values`; bits 5 to 6 the
//! failure reason), byte 1 is the label length `L`, the next `L` bytes are the
//! label, and the rest is the file. Two entries are rendered: the audit of the
//! file under that label, and a `Failed` entry with the same label whose
//! detail is the first 64 bytes of the file.
//!
//! The invariants live in `fuzz/shared/render_invariants.rs` (shared with
//! `tests/fuzz_layout.rs`, which shows each one going red when its bug is
//! planted) and run as named guards, each re-derived independently of
//! `exif_core::report` (the strict JSON and CSV parsers and a naive
//! `safe_text` are in `fuzz/shared/render_check.rs`):
//!
//! * `json_wellformed`, `json_shape`, `json_facts`: the strict parser accepts
//!   the output, keys and enumerations are exactly the schema, and every fact
//!   (status, counts, categories, locations, dispositions, previews, labels,
//!   excerpts) equals the re-derivation from the audit.
//! * `csv_wellformed`, `csv_rows`: RFC 4180, the exact header, 19 fields in
//!   every row, the expected rows, no unquoted label or value starting a
//!   formula.
//! * `text_safe`: no forbidden character, deterministic, none of the
//!   forbidden phrases, every label there.
//! * `values_gate`: with `values` off no excerpt appears in any format; with
//!   it on each is at most 64 source bytes and `truncated` iff longer.
//! * `summary_facts`, `render_linear`: the run summary equals the re-derived
//!   counts; output size is linear in items plus findings.
//!
//! This target CANNOT be run with the default PATH. `cargo fuzz` shells out to
//! a bare `cargo build`, which resolves to Homebrew's stable cargo and rejects
//! `-Zsanitizer=address`. Put the rustup shim first on PATH (or run `just
//! fuzz-smoke audit_render`):
//!
//! ```text
//! PATH="$HOME/.cargo/bin:$PATH" ~/.cargo/bin/cargo +nightly fuzz run audit_render \
//!     fuzz/corpus/audit_render fuzz/seeds/audit_render -- -max_total_time=60
//! ```
//!
//! Seeds are written by `cargo run --example fuzz_seeds`.

#![no_main]

use libfuzzer_sys::fuzz_target;

// `audit_invariants::rederive` and friends: the independent audit.
include!("../shared/audit_invariants.rs");
// The strict JSON and CSV parsers and the naive `safe_text`.
include!("../shared/render_check.rs");
// `render_invariants::check`, shared with `tests/fuzz_layout.rs`.
include!("../shared/render_invariants.rs");

fuzz_target!(|data: &[u8]| {
    render_invariants::check(data);
});
