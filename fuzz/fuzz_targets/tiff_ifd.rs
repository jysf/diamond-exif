//! Fuzz target for the TIFF/IFD container reader (SPEC-006).
//!
//! The blocking constraint `panic-free-library` stated executably: **for every
//! byte string, every entry point below returns, and none of them panics.** Any
//! answer, including every `Err`, is acceptable. An abort is not.
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

use exif_core::tiff::Container;

/// Tags worth probing with `scalar`/`values` on IFD0: width, length, bits per
/// sample, orientation, SubIFDs, and one that is never present.
const PROBE_TAGS: [u16; 6] = [256, 257, 258, 274, 330, 0xFFFF];

fuzz_target!(|data: &[u8]| {
    let Ok(container) = Container::parse(data) else {
        // A rejected input is a SUCCESS: the reader said no with a typed error.
        return;
    };

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
