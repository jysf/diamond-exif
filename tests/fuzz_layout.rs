//! Layout and seed checks for the `tiff_ifd` fuzz target (SPEC-006, DEC-004).
//! std only. The target itself needs nightly; these tests run on stable and
//! catch the regressions that do not need a fuzzer: a fuzz crate that leaked
//! into the library graph, a `.gitignore` that swallows the seeds, a seed that
//! went missing, or a seed that makes the parser panic.

use std::fs;
use std::path::PathBuf;

use exif_core::tiff::{Container, Entry, IfdKind, MAX_IFDS, MAX_ISSUES, MAX_TAG_VALUES};
use exif_core::Error;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(rel: &str) -> String {
    fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("cannot read {rel}: {e}"))
}

/// Every `IfdKind`, mirrored from the fuzz target.
const ALL_KINDS: [IfdKind; 7] = [
    IfdKind::Ifd0,
    IfdKind::Ifd1,
    IfdKind::Chain,
    IfdKind::Sub,
    IfdKind::Exif,
    IfdKind::Gps,
    IfdKind::Interop,
];

/// Seed names are fixed by `examples/fuzz_seeds.rs`.
const SEEDS: [&str; 29] = [
    "valid-le",
    "valid-be",
    "subifd",
    "chain",
    "truncated-header",
    "ifd0-offset-zero",
    "entry-count-past-eof",
    "payload-past-eof",
    "count-overflow",
    "self-cycle",
    "two-hop-cycle",
    "deep-nesting",
    "corrupt-subifd",
    "malformed-subifds-tag",
    "depth-limit-exact",
    "exif-gps-interop",
    "gps-past-eof-exif-ok",
    "exif-points-at-ifd0",
    "pointer-bad-count",
    "subifds-count-over-cap",
    "leaf-next-word",
    "ascii-shapes",
    "signed-and-float",
    "ascii-count-past-eof",
    "chain-be",
    "count-overflow-be",
    "next-past-eof",
    "payload-past-eof-be",
    "subifd-be",
];

#[test]
fn fuzz_crate_is_separate_and_licensed() {
    let fuzz = read("fuzz/Cargo.toml");
    assert!(
        fuzz.contains("license = \"MIT OR Apache-2.0\""),
        "fuzz/Cargo.toml needs a license field (a missing one is an ERROR to cargo-deny)"
    );
    assert!(
        fuzz.contains("name = \"tiff_ifd\""),
        "fuzz/Cargo.toml must declare the tiff_ifd bin"
    );
    assert!(
        fuzz.contains("libfuzzer-sys"),
        "fuzz crate must depend on libfuzzer-sys"
    );
    let root_manifest = read("Cargo.toml");
    assert!(
        !root_manifest.contains("libfuzzer-sys"),
        "libfuzzer-sys must stay out of the root manifest (DEC-002, DEC-004)"
    );
}

#[test]
fn fuzz_gitignore_keeps_seeds_and_ignores_corpus() {
    let ignore = read("fuzz/.gitignore");
    let lines: Vec<&str> = ignore.lines().map(str::trim).collect();
    for needed in ["corpus", "artifacts"] {
        assert!(
            lines.iter().any(|l| l.trim_end_matches('/') == needed),
            "fuzz/.gitignore must ignore {needed}"
        );
    }
    assert!(
        !lines.iter().any(|l| l.contains("seeds")),
        "fuzz/.gitignore must not ignore seeds"
    );
}

#[test]
fn seeds_are_committed_and_cover_the_hostile_shapes() {
    let dir = root().join("fuzz/seeds/tiff_ifd");
    for name in SEEDS {
        let path = dir.join(name);
        assert!(path.is_file(), "missing seed {}", path.display());
    }
}

/// Whether `inner` lies wholly inside `outer` (empty slices included).
fn lies_inside(outer: &[u8], inner: &[u8]) -> bool {
    let (o, i) = (outer.as_ptr_range(), inner.as_ptr_range());
    i.start >= o.start && i.end <= o.end
}

/// `r` is the type-gate error for `entry` (and nothing else is).
fn is_type_gate<T>(r: &Result<T, Error>, entry: &Entry) -> bool {
    matches!(
        r,
        Err(Error::UnexpectedFieldType { tag, field_type })
            if *tag == entry.tag() && *field_type == entry.field_type()
    )
}

/// Every reader on one entry, with the SPEC-004/SPEC-007 invariants. Each rule
/// that is an iff is asserted in both directions, so a constant-true or
/// constant-false reader fails one of them.
fn check_entry(data: &[u8], container: &Container, entry: &Entry) {
    let ty = entry.field_type();
    let _ = entry.byte_len();
    let payload = container.payload(entry);

    let uints = container.uints(entry);
    let urationals = container.urationals(entry);
    let srationals = container.srationals(entry);
    let ascii = container.ascii(entry);
    let sints = container.sints(entry);
    let floats = container.floats(entry);

    // Type gate: UnexpectedFieldType iff the type is outside the reader's set.
    // The sets are written out here, independent of the library's own gates.
    assert_eq!(
        is_type_gate(&uints, entry),
        !matches!(ty, 1 | 3 | 4 | 7 | 13),
        "uints type gate, type {ty}"
    );
    assert_eq!(
        is_type_gate(&urationals, entry),
        ty != 5,
        "urationals type gate, type {ty}"
    );
    assert_eq!(
        is_type_gate(&srationals, entry),
        ty != 10,
        "srationals type gate, type {ty}"
    );
    assert_eq!(
        is_type_gate(&ascii, entry),
        ty != 2,
        "ascii type gate, type {ty}"
    );
    assert_eq!(
        is_type_gate(&sints, entry),
        !matches!(ty, 6 | 8 | 9),
        "sints type gate, type {ty}"
    );
    assert_eq!(
        is_type_gate(&floats, entry),
        !matches!(ty, 11 | 12),
        "floats type gate, type {ty}"
    );

    // Length: an Ok result has exactly `count` values.
    let count = usize::try_from(entry.count()).unwrap_or(usize::MAX);
    if let Ok(v) = &uints {
        assert_eq!(v.len(), count, "uints length");
    }
    if let Ok(v) = &sints {
        assert_eq!(v.len(), count, "sints length");
    }
    if let Ok(v) = &floats {
        assert_eq!(v.len(), count, "floats length");
    }

    // ASCII is Ok iff the type is ASCII, the count is within the cap and the
    // payload resolves.
    assert_eq!(
        ascii.is_ok(),
        ty == 2 && count <= MAX_TAG_VALUES && payload.is_ok(),
        "ascii Ok iff type, cap and payload"
    );

    if let (Ok(a), Ok(p)) = (&ascii, &payload) {
        let text = a.as_bytes();
        let rest = a.rest();
        // Partition: text, the NUL if any, then rest cover the payload.
        assert_eq!(
            text.len() + usize::from(a.is_terminated()) + rest.len(),
            p.len(),
            "ascii partition"
        );
        assert!(!text.contains(&0), "NUL inside ascii text");
        // Terminated iff the payload contains a NUL, and it sits right after
        // the text.
        assert_eq!(a.is_terminated(), p.contains(&0), "terminated iff NUL");
        if a.is_terminated() {
            assert_eq!(p.get(text.len()), Some(&0), "NUL not at end of text");
        } else {
            assert!(rest.is_empty(), "unterminated but rest is not empty");
        }
        // Zero-copy: both parts lie inside the input, and are the payload's
        // own bytes (text first, rest after the NUL).
        assert!(lies_inside(data, text), "ascii text escapes the input");
        assert!(lies_inside(data, rest), "ascii rest escapes the input");
        assert_eq!(text.as_ptr(), p.as_ptr(), "text is not the payload start");
        assert_eq!(
            rest.as_ptr(),
            p.as_ptr()
                .wrapping_add(text.len() + usize::from(a.is_terminated())),
            "rest is not right after the text"
        );
        assert_eq!(p.get(..text.len()), Some(text));
        // to_str is Ok iff the text is UTF-8; the lossy form never panics.
        assert_eq!(
            a.to_str().is_ok(),
            core::str::from_utf8(text).is_ok(),
            "to_str iff UTF-8"
        );
        let lossy = a.to_string_lossy();
        if let Ok(s) = a.to_str() {
            assert_eq!(lossy, s, "lossy differs on valid UTF-8");
        }
    }
}

/// The harness contract from `fuzz/fuzz_targets/tiff_ifd.rs`, run once on a
/// plain input. Any `Err` is fine; a panic or a broken invariant is the failure.
fn exercise(data: &[u8]) {
    let strict = Container::parse_strict(data);
    let container = match Container::parse(data) {
        Ok(container) => container,
        Err(error) => {
            // Fatal input: both entry points must say the same thing.
            assert_eq!(strict.err(), Some(error));
            return;
        }
    };

    // Differential: strict is Ok iff the walk was clean, else the first issue.
    match strict {
        Ok(_) => assert!(container.is_clean(), "strict Ok but issues present"),
        Err(error) => {
            assert!(!container.is_clean(), "strict Err but is_clean");
            let first = container
                .issues()
                .first()
                .expect("strict Err needs an issue");
            assert_eq!(&error, first.error());
        }
    }

    // Containment: every recorded IFD lies fully inside the input.
    for ifd in container.ifds() {
        let end = u64::from(ifd.offset()) + 2 + 12 * ifd.entries().len() as u64 + 4;
        assert!(end <= data.len() as u64, "IFD escapes the input");
    }

    // Bounds.
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

    for ifd in container.ifds() {
        for entry in ifd.entries() {
            check_entry(data, &container, entry);
        }
    }
    if let Some(ifd0) = container.ifd0() {
        for tag in [256, 257, 258, 274, 330, 0xFFFF] {
            let _ = container.scalar(ifd0, tag);
            let _ = container.values(ifd0, tag);
        }
    }
}

#[test]
fn every_seed_parses_or_errors_without_panicking() {
    let dir = root().join("fuzz/seeds/tiff_ifd");
    let mut seen = 0;
    for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
    {
        let path = entry.expect("dir entry").path();
        let data =
            fs::read(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        exercise(&data);
        seen += 1;
    }
    assert!(
        seen >= SEEDS.len(),
        "expected at least {} seeds, saw {seen}",
        SEEDS.len()
    );
}
