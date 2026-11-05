//! Fuzz target for the TIFF/IFD container reader (SPEC-006).
//!
//! The blocking constraint `panic-free-library` stated executably: **for every
//! byte string, every entry point below returns, and none of them panics.** Any
//! answer, including every `Err`, is acceptable. An abort is not.
//!
//! Since SPEC-003 (tolerant walk, DEC-001) the target also asserts the walk's
//! invariants, so a violated one is a finding: `parse_strict` agrees with
//! `parse`/`is_clean`/`issues` (differential); every recorded IFD lies inside
//! the input (containment); IFD and issue counts stay within their bounds and
//! every issue points at a recorded IFD (bounds).
//!
//! The walk must stay in sync with the public reader API: a spec that adds a
//! reader must add it here in the same change.
//!
//! This target CANNOT be run with the default PATH. `cargo fuzz` shells out to
//! a bare `cargo build`, which resolves to Homebrew's stable cargo and rejects
//! `-Zsanitizer=address`; the outer `+nightly` does not help. Put the rustup
//! shim first on PATH:
//!
//! ```text
//! PATH="$HOME/.cargo/bin:$PATH" ~/.cargo/bin/cargo +nightly fuzz run tiff_ifd \
//!     fuzz/corpus/tiff_ifd fuzz/seeds/tiff_ifd -- -max_total_time=60
//! ```
//!
//! Seeds are written by `cargo run --example fuzz_seeds`.

#![no_main]

use libfuzzer_sys::fuzz_target;

use exif_core::tiff::{Container, MAX_IFDS};

/// Tags worth probing with `scalar`/`values` on IFD0: width, length, bits per
/// sample, orientation, SubIFDs, and one that is never present.
const PROBE_TAGS: [u16; 6] = [256, 257, 258, 274, 330, 0xFFFF];

fuzz_target!(|data: &[u8]| {
    let strict = Container::parse_strict(data);
    let container = match Container::parse(data) {
        Ok(container) => container,
        Err(error) => {
            // A rejected input is a SUCCESS: the reader said no with a typed
            // error, and the strict entry point must say the same.
            assert_eq!(
                strict.err(),
                Some(error),
                "parse_strict disagrees on a fatal input"
            );
            return;
        }
    };

    // Differential: strict is Ok iff the walk was clean; otherwise it returns
    // the first issue's error.
    match strict {
        Ok(_) => assert!(container.is_clean(), "parse_strict Ok but issues present"),
        Err(error) => {
            assert!(!container.is_clean(), "parse_strict Err but is_clean");
            let first = container
                .issues()
                .first()
                .expect("parse_strict Err on a container with no issues");
            assert_eq!(
                &error,
                first.error(),
                "parse_strict returned a different error"
            );
        }
    }

    // Containment: every recorded IFD lies fully inside the input.
    for ifd in container.ifds() {
        let end = u64::from(ifd.offset()) + 2 + 12 * ifd.entries().len() as u64 + 4;
        assert!(end <= data.len() as u64, "an IFD escapes the input");
    }

    // Bounds, and exercise the issue getters and Display.
    assert!(container.ifds().len() <= MAX_IFDS);
    assert!(container.issues().len() <= 2 * MAX_IFDS + 1);
    for issue in container.issues() {
        assert!(issue.from_ifd() < container.ifds().len());
        let _ = (issue.error(), issue.link(), issue.offset());
        assert!(!issue.to_string().is_empty());
    }

    // `parse` only reads entry HEADERS; payload resolution is where the hostile
    // `count x sizeof(type)` product is used, so it must be reached from here.
    for ifd in container.ifds() {
        let _ = (ifd.offset(), ifd.depth(), ifd.parent(), ifd.next());
        for entry in ifd.entries() {
            let _ = entry.byte_len();
            let _ = container.payload(entry);
            let _ = container.uints(entry);
            let _ = container.urationals(entry);
            let _ = container.srationals(entry);
        }
    }

    if let Some(ifd0) = container.ifd0() {
        for tag in PROBE_TAGS {
            let _ = container.scalar(ifd0, tag);
            let _ = container.values(ifd0, tag);
        }
    }
});
