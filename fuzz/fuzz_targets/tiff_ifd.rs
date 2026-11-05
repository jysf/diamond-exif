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
//! every issue points at a recorded IFD (bounds). Since SPEC-004 (DEC-005) it
//! also checks the IFD kinds and parent links (only the first IFD is `Ifd0`;
//! `Interop` hangs off `Exif`; parents precede children; child depth is
//! parent + 1) and that `ifd_of_kind` agrees with a linear scan.
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

use exif_core::tiff::{Container, IfdKind, MAX_IFDS, MAX_ISSUES};

/// Tags worth probing with `scalar`/`values` on IFD0: width, length, bits per
/// sample, orientation, SubIFDs, and one that is never present.
const PROBE_TAGS: [u16; 6] = [256, 257, 258, 274, 330, 0xFFFF];

/// Every `IfdKind`, for the `ifd_of_kind` cross-check. Keep in step with the
/// enum: a new kind must be added here in the same change.
const ALL_KINDS: [IfdKind; 7] = [
    IfdKind::Ifd0,
    IfdKind::Ifd1,
    IfdKind::Chain,
    IfdKind::Sub,
    IfdKind::Exif,
    IfdKind::Gps,
    IfdKind::Interop,
];

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
    assert!(container.issues().len() <= MAX_ISSUES);
    for issue in container.issues() {
        assert!(issue.from_ifd() < container.ifds().len());
        let _ = (issue.error(), issue.link(), issue.offset());
        assert!(!issue.to_string().is_empty());
    }

    // The attempt budget halts the walk, so TooManyIfds is always the last
    // issue (iff-style: present implies last, and a non-last one is a bug).
    let halts = container
        .issues()
        .iter()
        .enumerate()
        .filter(|(_, i)| matches!(i.error(), exif_core::Error::TooManyIfds { .. }))
        .map(|(idx, _)| idx)
        .collect::<Vec<usize>>();
    assert!(halts.len() <= 1, "more than one TooManyIfds");
    if let Some(idx) = halts.first() {
        assert_eq!(
            *idx + 1,
            container.issues().len(),
            "issue recorded after halt"
        );
    }

    // Kinds and parents (DEC-005). Each rule is an iff where one exists, so a
    // constant-true or constant-false labeller fails in one direction.
    for (i, ifd) in container.ifds().iter().enumerate() {
        // Only the first IFD is IFD0, and it is always IFD0.
        assert_eq!(
            ifd.kind() == IfdKind::Ifd0,
            i == 0,
            "Ifd0 kind iff first IFD"
        );
        // Top-level chain kinds have no parent; every other kind has one.
        let top_level = matches!(ifd.kind(), IfdKind::Ifd0 | IfdKind::Ifd1 | IfdKind::Chain);
        assert_eq!(top_level, ifd.parent().is_none(), "top-level iff no parent");
        let Some(parent_index) = ifd.parent() else {
            assert_eq!(ifd.depth(), 0, "a top-level IFD is depth 0");
            continue;
        };
        // The parent was recorded before its child.
        assert!(parent_index < i, "parent index is not earlier");
        let parent = &container.ifds()[parent_index];
        assert_eq!(ifd.depth(), parent.depth() + 1, "child depth is parent + 1");
        match ifd.kind() {
            // Interop only ever hangs off an Exif IFD.
            IfdKind::Interop => assert_eq!(parent.kind(), IfdKind::Exif),
            // Exif and GPS hang off a top-level or Sub IFD, never a leaf kind.
            IfdKind::Exif | IfdKind::Gps => assert!(!matches!(
                parent.kind(),
                IfdKind::Exif | IfdKind::Gps | IfdKind::Interop
            )),
            _ => {}
        }
        // Converse: an Exif parent only has Interop or Sub children, and
        // GPS / Interop parents only Sub children (leaf kinds do not chain).
        match parent.kind() {
            IfdKind::Exif => assert!(matches!(ifd.kind(), IfdKind::Interop | IfdKind::Sub)),
            IfdKind::Gps | IfdKind::Interop => assert_eq!(ifd.kind(), IfdKind::Sub),
            _ => {}
        }
    }
    // `ifd_of_kind` is the first IFD of the kind, and `None` iff there is none.
    for kind in ALL_KINDS {
        let scan = container.ifds().iter().find(|ifd| ifd.kind() == kind);
        match (container.ifd_of_kind(kind), scan) {
            (Some(a), Some(b)) => assert!(core::ptr::eq(a, b), "ifd_of_kind is not the first"),
            (None, None) => {}
            _ => panic!("ifd_of_kind disagrees with a linear scan"),
        }
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
