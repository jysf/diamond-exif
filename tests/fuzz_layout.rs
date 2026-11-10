//! Layout and seed checks for the `tiff_ifd` fuzz target (SPEC-006, DEC-004).
//! std only. The target itself needs nightly; these tests run on stable and
//! catch the regressions that do not need a fuzzer: a fuzz crate that leaked
//! into the library graph, a `.gitignore` that swallows the seeds, a seed that
//! went missing, or a seed that makes the parser panic.

use std::fs;
use std::path::PathBuf;

// SPEC-019: the synthetic real-world shapes, to compare the committed `shape-*`
// seeds with the builders byte for byte.
#[path = "support/oracle_tiff.rs"]
mod oracle_tiff;
#[path = "support/real_shapes.rs"]
mod real_shapes;
// SPEC-020: the strip fixtures, for the `rewrite_jpeg` harness plants and seeds.
#[path = "support/strip_shapes.rs"]
mod strip_shapes;

use exif_core::jpeg::{Jpeg, SegmentKind, MAX_JPEG_ISSUES, MAX_JPEG_SEGMENTS};
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

/// JPEG seed names are fixed by `examples/fuzz_seeds.rs`.
const JPEG_SEEDS: [&str; 37] = [
    "minimal",
    "soi-only",
    "fill-bytes",
    "app1-len-0",
    "app1-len-1",
    "app1-len-2",
    "app1-len-ffff",
    "app1-past-eof",
    "exif-and-xmp",
    "extended-xmp-pair",
    "stray-bytes",
    "ff00-stray",
    "no-sos",
    "sos-no-eoi",
    "progressive-two-scans",
    "trailer-after-eoi",
    "app-after-scan",
    "many-segments",
    "many-ff00",
    "icc-iptc-jumbf",
    "signatures-under-wrong-marker",
    // SPEC-010: Exif hand-off and located regions.
    "exif-in-jpeg",
    "two-exif",
    "exif-bad-tiff",
    "exif-after-scan",
    "xmp-and-extended-xmp",
    "extended-xmp-short",
    "icc-chunks-ok",
    "icc-chunks-mismatch",
    "iptc-8bim",
    "iptc-truncated-resource",
    "jumbf-header",
    "jumbf-short",
    "seventeen-exif",
    // SPEC-011: DEC-007 and the SPEC-010 verify gap I1.
    "exif-sixth-byte-nonzero",
    "exif-five-byte-payload",
    "icc-seq-zero",
];

/// The 63 seeds that existed before SPEC-011 (29 `tiff_ifd` + 34
/// `jpeg_segments`): `(target, name, length, FNV-1a 64 of the bytes)`, taken
/// from the committed files before SPEC-011 changed anything. Regenerating the
/// seeds must leave every one of them byte-identical (std has no stable hash
/// API with fixed output, so FNV-1a is computed by hand below).
const PRE_SPEC_011_SEEDS: [(&str, &str, usize, u64); 63] = [
    ("tiff_ifd", "ascii-count-past-eof", 79, 0xf9fb168ecf63c873),
    ("tiff_ifd", "ascii-shapes", 161, 0x40686d2b31db9f20),
    ("tiff_ifd", "chain", 62, 0x5103cafa5ffe6da1),
    ("tiff_ifd", "chain-be", 62, 0x0e7a1e63dbe9517d),
    ("tiff_ifd", "corrupt-subifd", 38, 0x8dad61eefdeaa933),
    ("tiff_ifd", "count-overflow", 86, 0x0ca614f0bde8ce1c),
    ("tiff_ifd", "count-overflow-be", 86, 0x46a5d940e364b10e),
    ("tiff_ifd", "deep-nesting", 356, 0x86bc5a943f3da84d),
    ("tiff_ifd", "depth-limit-exact", 266, 0x2d1ea0dcff4e5a83),
    ("tiff_ifd", "entry-count-past-eof", 13, 0x78590c388acc9606),
    ("tiff_ifd", "exif-gps-interop", 116, 0x662b025c31b0e761),
    ("tiff_ifd", "exif-points-at-ifd0", 38, 0xbd387c33c1a06741),
    ("tiff_ifd", "gps-past-eof-exif-ok", 68, 0xc46df5c8bc378c6b),
    ("tiff_ifd", "ifd0-offset-zero", 8, 0x432d45b4d7c255d5),
    ("tiff_ifd", "leaf-next-word", 74, 0x85063aa66064a89b),
    ("tiff_ifd", "malformed-subifds-tag", 56, 0xb547aa14c20643ba),
    ("tiff_ifd", "next-past-eof", 26, 0x802bbf014b971d79),
    ("tiff_ifd", "payload-past-eof", 50, 0x24228b3502290f30),
    ("tiff_ifd", "payload-past-eof-be", 50, 0xb936734a88d49c7a),
    ("tiff_ifd", "pointer-bad-count", 50, 0x41444d715968b875),
    ("tiff_ifd", "self-cycle", 26, 0x7e9d0243fc17108b),
    ("tiff_ifd", "signed-and-float", 226, 0xc64c538111f83ebb),
    ("tiff_ifd", "subifd", 82, 0x6f3e5be266b40257),
    ("tiff_ifd", "subifd-be", 82, 0xb367930d24976e8b),
    ("tiff_ifd", "subifds-count-over-cap", 38, 0x29ca58ae79ebcb95),
    ("tiff_ifd", "truncated-header", 5, 0x50e0a90335f50b17),
    ("tiff_ifd", "two-hop-cycle", 68, 0xba54c41e6b2198b3),
    ("tiff_ifd", "valid-be", 126, 0xd14e46fc095cffa9),
    ("tiff_ifd", "valid-le", 126, 0x80fe123345fe5215),
    ("jpeg_segments", "app-after-scan", 39, 0x3c0b74c4ea41965e),
    ("jpeg_segments", "app1-len-0", 24, 0xa71f9b333b778368),
    ("jpeg_segments", "app1-len-1", 24, 0xd3cb0a5e7ebf3f1f),
    ("jpeg_segments", "app1-len-2", 18, 0x03f1c23d02e437f0),
    ("jpeg_segments", "app1-len-ffff", 65551, 0xc760bd59985fc432),
    ("jpeg_segments", "app1-past-eof", 16, 0xf7f17c5736b2e083),
    ("jpeg_segments", "exif-after-scan", 148, 0x10ad14693c9e9c6b),
    ("jpeg_segments", "exif-and-xmp", 84, 0xadb9ce26dd6e2b38),
    ("jpeg_segments", "exif-bad-tiff", 32, 0xb11d03c3f7ca9288),
    ("jpeg_segments", "exif-in-jpeg", 148, 0x7c21afba5dea8c89),
    (
        "jpeg_segments",
        "extended-xmp-pair",
        149,
        0x8768c0104678fa1d,
    ),
    (
        "jpeg_segments",
        "extended-xmp-short",
        171,
        0xc2fb6f7d331f2226,
    ),
    ("jpeg_segments", "ff00-stray", 28, 0x329ecc00b7b39d86),
    ("jpeg_segments", "fill-bytes", 35, 0x0c6af3c3636b50db),
    (
        "jpeg_segments",
        "icc-chunks-mismatch",
        77,
        0xcf9d799ac0d0f1e6,
    ),
    ("jpeg_segments", "icc-chunks-ok", 61, 0xd70da14137a10c3a),
    ("jpeg_segments", "icc-iptc-jumbf", 73, 0xbe84bb0a5f3b3ec3),
    ("jpeg_segments", "iptc-8bim", 68, 0xc12b2f98a1b99274),
    (
        "jpeg_segments",
        "iptc-truncated-resource",
        61,
        0xd533a41f8b100bb4,
    ),
    ("jpeg_segments", "jumbf-header", 30, 0xbca67c1eeca4ff73),
    ("jpeg_segments", "jumbf-short", 25, 0xb3cae112b487977c),
    ("jpeg_segments", "many-ff00", 2202, 0x1a0b709fa81c45f2),
    ("jpeg_segments", "many-segments", 16414, 0xc675c75e5ea8ad5e),
    ("jpeg_segments", "minimal", 68, 0x47af3ff740559a4a),
    ("jpeg_segments", "no-sos", 28, 0x1bdded91eb609b96),
    (
        "jpeg_segments",
        "progressive-two-scans",
        55,
        0xd126899024ddea18,
    ),
    ("jpeg_segments", "seventeen-exif", 2173, 0x6e50da0b2ab0c175),
    (
        "jpeg_segments",
        "signatures-under-wrong-marker",
        168,
        0xe0544e0816d47269,
    ),
    ("jpeg_segments", "soi-only", 2, 0x0a99ee07b6f68642),
    ("jpeg_segments", "sos-no-eoi", 19, 0x0e30d944ac162394),
    ("jpeg_segments", "stray-bytes", 34, 0xcaad01d80bb242c1),
    ("jpeg_segments", "trailer-after-eoi", 36, 0x10a1ad72a76b0149),
    ("jpeg_segments", "two-exif", 277, 0x546b67de32768525),
    (
        "jpeg_segments",
        "xmp-and-extended-xmp",
        228,
        0x7cd5037ca5f8982e,
    ),
];

fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

#[test]
fn the_existing_63_seeds_are_byte_identical() {
    assert_eq!(PRE_SPEC_011_SEEDS.len(), 63);
    for (target, name, len, hash) in PRE_SPEC_011_SEEDS {
        let path = root().join("fuzz/seeds").join(target).join(name);
        let bytes = fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(bytes.len(), len, "{target}/{name}: length");
        assert_eq!(fnv1a_64(&bytes), hash, "{target}/{name}: contents changed");
    }
    // And nothing but the three SPEC-011 seeds was added (the three SPEC-017
    // `tiff_ifd` seeds make 32; `new_tiff_seeds_have_the_shapes_they_claim`).
    let count = |t: &str| {
        fs::read_dir(root().join("fuzz/seeds").join(t))
            .unwrap_or_else(|e| panic!("{t}: {e}"))
            .count()
    };
    assert_eq!(count("tiff_ifd"), 32);
    // 37 + the 15 `shape-*` seeds SPEC-019 added (none of the 37 changed).
    assert_eq!(count("jpeg_segments"), 37 + SHAPE_SEEDS.len());
}

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
        fuzz.contains("name = \"jpeg_segments\""),
        "fuzz/Cargo.toml must declare the jpeg_segments bin"
    );
    assert!(
        root().join("fuzz/fuzz_targets/jpeg_segments.rs").is_file(),
        "the jpeg_segments target source is missing"
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

#[test]
fn jpeg_seeds_are_committed() {
    let dir = root().join("fuzz/seeds/jpeg_segments");
    for name in JPEG_SEEDS {
        let path = dir.join(name);
        assert!(path.is_file(), "missing seed {}", path.display());
    }
}

#[test]
fn every_jpeg_seed_walks_without_panicking() {
    let dir = root().join("fuzz/seeds/jpeg_segments");
    let mut seen = 0;
    for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
    {
        let path = entry.expect("dir entry").path();
        let data =
            fs::read(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        exercise_jpeg(&data);
        seen += 1;
    }
    assert!(
        seen >= JPEG_SEEDS.len(),
        "expected at least {} seeds, saw {seen}",
        JPEG_SEEDS.len()
    );
}

/// Each SPEC-010 seed has the shape its name promises, so a seed that stopped
/// reaching its code path fails here rather than silently going stale.
#[test]
fn spec_010_seeds_have_the_shapes_they_claim() {
    use exif_core::jpeg::{self, Region};
    let dir = root().join("fuzz/seeds/jpeg_segments");
    let load = |name: &str| fs::read(dir.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
    let issues = |m: &jpeg::JpegMetadata<'_>| -> Vec<Error> {
        m.jpeg_issues().iter().map(|i| i.error().clone()).collect()
    };

    let d = load("exif-in-jpeg");
    let m = jpeg::read(&d).unwrap();
    assert!(m.is_clean() && m.exif().is_some());
    assert_eq!(m.exif().unwrap().ifds().len(), 3);

    let d = load("two-exif");
    let m = jpeg::read(&d).unwrap();
    assert_eq!(m.exif_blocks().len(), 2);
    assert!(matches!(issues(&m)[..], [Error::ExtraExif { .. }]));

    let d = load("exif-bad-tiff");
    let m = jpeg::read(&d).unwrap();
    assert!(m.exif().is_none() && m.exif_blocks()[0].container().is_err());

    let d = load("exif-after-scan");
    let m = jpeg::read(&d).unwrap();
    assert!(m.jpeg().segments()[1].after_scan() && m.exif().is_some());

    let d = load("xmp-and-extended-xmp");
    let m = jpeg::read(&d).unwrap();
    assert!(matches!(m.regions()[0], Region::Xmp { .. }));
    assert_eq!(
        m.regions()
            .iter()
            .filter(|r| matches!(r, Region::ExtendedXmp { .. }))
            .count(),
        2
    );

    let d = load("extended-xmp-short");
    let m = jpeg::read(&d).unwrap();
    assert_eq!(m.regions().len(), 1);
    assert!(matches!(
        issues(&m)[..],
        [Error::RegionTooShort { have: 39, .. }]
    ));

    let d = load("icc-chunks-ok");
    assert!(jpeg::read(&d).unwrap().is_clean());
    let d = load("icc-chunks-mismatch");
    let m = jpeg::read(&d).unwrap();
    // The third chunk has no room for seq/count; the aggregate comes last.
    assert!(matches!(
        issues(&m)[..],
        [
            Error::RegionTooShort {
                what: "ICC chunk header",
                ..
            },
            Error::IccChunkMismatch { .. }
        ]
    ));

    let d = load("iptc-8bim");
    let m = jpeg::read(&d).unwrap();
    assert!(m.is_clean());
    assert_eq!(m.regions().len(), 1);
    // DEC-012: the region is the segment's payload after the header.
    let body = m.regions()[0].data();
    assert!(body.starts_with(b"8BIM"));
    assert!(body.windows(5).any(|w| w == [1u8, 2, 3, 4, 5]));

    let d = load("iptc-truncated-resource");
    let m = jpeg::read(&d).unwrap();
    assert_eq!(m.regions().len(), 1);
    assert!(matches!(
        issues(&m)[..],
        [Error::RegionTooShort {
            what: "8BIM resource",
            ..
        }]
    ));

    let d = load("jumbf-header");
    let m = jpeg::read(&d).unwrap();
    assert!(matches!(
        m.regions()[0],
        Region::Jumbf {
            en: 7,
            z: 0x0102_0304,
            ..
        }
    ));
    let d = load("jumbf-short");
    let m = jpeg::read(&d).unwrap();
    assert!(m.regions().is_empty());
    assert!(matches!(
        issues(&m)[..],
        [Error::RegionTooShort {
            what: "JUMBF header",
            ..
        }]
    ));

    let d = load("seventeen-exif");
    let m = jpeg::read(&d).unwrap();
    assert_eq!(m.exif_blocks().len(), jpeg::MAX_EXIF_BLOCKS);
    assert_eq!(m.jpeg_issues().len(), 16);
}

/// The `jpeg_segments` harness contract on a few extra shapes the seeds do not
/// cover: fatal inputs and an all-`FF` run.
#[test]
fn jpeg_harness_holds_on_fatal_and_degenerate_inputs() {
    for data in [
        &[][..],
        &[0xFF][..],
        &[0x89, 0x50][..],
        &[0xFF, 0xD8, 0xFF][..],
    ] {
        exercise_jpeg(data);
    }
    exercise_jpeg(&[0xFF; 4096]);
    // A signature under the wrong marker is not that kind (the marker guard of
    // the iff), and a plain APP1 without the signature is not Exif (the
    // signature guard).
    exercise_jpeg(&[
        0xFF, 0xD8, 0xFF, 0xE2, 0x00, 0x08, b'E', b'x', b'i', b'f', 0, 0, 0xFF, 0xD9,
    ]);
    exercise_jpeg(&[0xFF, 0xD8, 0xFF, 0xE1, 0x00, 0x04, 1, 2, 0xFF, 0xD9]);
}

/// DEC-007, both directions of the iff, on the harness itself: the shared
/// `exif_by_signature` and the mirrored `expected_kind` agree with the library
/// on every case, and each case is one a six-byte rule would get wrong or one a
/// "first four bytes" rule would get wrong. A harness that only checked the
/// canonical signature would fail the first group; one that accepted any
/// `Exif` prefix would fail the second.
#[test]
fn jpeg_harness_five_byte_iff_both_directions() {
    fn jpeg_with(marker: u8, payload: &[u8]) -> Vec<u8> {
        let l = u16::try_from(payload.len() + 2).expect("short");
        let mut d = vec![0xFF, 0xD8, 0xFF, marker, (l >> 8) as u8, l as u8];
        d.extend_from_slice(payload);
        d.extend_from_slice(&[0xFF, 0xDA, 0, 8, 1, 1, 0, 0, 0x3F, 0, 0xFF, 0xD9]);
        d
    }
    let is_exif: [(&[u8], bool); 12] = [
        (b"Exif\0\0II*\0", true),
        (b"Exif\0\x01II*\0", true),
        (b"Exif\0\xFF", true),
        (b"Exif\0\0", true),
        (b"Exif\0", false),
        (b"Exif", false),
        (b"Exi", false),
        (b"", false),
        (b"Exif\x01\0II*\0", false),
        (b"Exif \0II*\0", false),
        (b"exif\0\0II*\0", false),
        (b"xExif\0\0II*\0", false),
    ];
    for (payload, expect) in is_exif {
        assert_eq!(
            metadata_invariants::exif_by_signature(0xE1, payload),
            expect,
            "{payload:?}"
        );
        let d = jpeg_with(0xE1, payload);
        let jpeg = Jpeg::parse(&d).expect("walks");
        assert_eq!(
            jpeg.segments()[0].kind() == SegmentKind::Exif,
            expect,
            "library kind of {payload:?}"
        );
        assert_eq!(
            expected_kind(0xE1, payload) == SegmentKind::Exif,
            expect,
            "harness kind of {payload:?}"
        );
        exercise_jpeg(&d);
        // The marker half of the iff: the same payload under APP2 is never Exif.
        assert!(!metadata_invariants::exif_by_signature(0xE2, payload));
        exercise_jpeg(&jpeg_with(0xE2, payload));
    }
    // A six-byte rule and a five-byte rule differ exactly on a non-zero sixth
    // byte: the case this test exists to keep.
    let six = |p: &[u8]| p.starts_with(b"Exif\0\0");
    assert!(metadata_invariants::exif_by_signature(0xE1, b"Exif\0\x01II") && !six(b"Exif\0\x01II"));
}

/// The three SPEC-011 seeds have the shapes they claim.
#[test]
fn new_seeds_have_the_shapes_they_claim() {
    use exif_core::jpeg;
    let dir = root().join("fuzz/seeds/jpeg_segments");
    let load = |name: &str| fs::read(dir.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));

    // DEC-007: `Exif\0` + a non-zero sixth byte + a valid TIFF is an Exif block.
    let d = load("exif-sixth-byte-nonzero");
    let m = jpeg::read(&d).unwrap();
    let seg = m.jpeg().segments()[0];
    assert_eq!(seg.kind(), SegmentKind::Exif);
    assert_eq!(&seg.payload()[..6], b"Exif\0\x01");
    assert_eq!(m.exif_blocks().len(), 1);
    assert!(m.is_clean() && m.exif().is_some());

    // DEC-007: a payload of exactly `Exif\0` is not Exif.
    let d = load("exif-five-byte-payload");
    let m = jpeg::read(&d).unwrap();
    let seg = m.jpeg().segments()[0];
    assert_eq!(seg.payload(), b"Exif\0");
    assert_eq!(seg.kind(), SegmentKind::App(1));
    assert!(m.exif_blocks().is_empty() && m.is_clean());
}

/// SPEC-010 verify I1: an ICC chunk with sequence number 0 is one
/// inconsistent-set issue, naming that chunk, and nothing else.
#[test]
fn icc_seq_zero_seed_yields_one_mismatch_issue() {
    use exif_core::jpeg;
    let d = fs::read(root().join("fuzz/seeds/jpeg_segments/icc-seq-zero")).expect("seed");
    let m = jpeg::read(&d).unwrap();
    assert_eq!(m.regions().len(), 1);
    assert!(matches!(
        m.regions()[0],
        jpeg::Region::Icc {
            seq: 0,
            count: 1,
            ..
        }
    ));
    let issues: Vec<(Error, usize)> = m
        .jpeg_issues()
        .iter()
        .map(|i| (i.error().clone(), i.offset()))
        .collect();
    let at = m.jpeg().segments()[0].offset();
    assert_eq!(issues, vec![(Error::IccChunkMismatch { at }, at)]);
}

/// Every TIFF seed, wrapped as the Exif of a JPEG, satisfies the shared
/// invariants. This is where a TIFF-layer issue meets a clean JPEG layer (the
/// layered view and `is_clean`), and where a JPEG-layer issue (no EOI) meets a
/// TIFF-layer one (the issue order).
#[test]
fn every_tiff_seed_wrapped_in_a_jpeg_satisfies_the_harness() {
    let dir = root().join("fuzz/seeds/tiff_ifd");
    let mut wrapped = 0;
    for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
    {
        let path = entry.expect("dir entry").path();
        let tiff = fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let payload = [&b"Exif\0\0"[..], &tiff].concat();
        let Ok(len) = u16::try_from(payload.len() + 2) else {
            continue; // does not fit one segment
        };
        let mut app1 = vec![0xFF, 0xE1, (len >> 8) as u8, len as u8];
        app1.extend_from_slice(&payload);
        let sos = [0xFF, 0xDA, 0, 8, 1, 1, 0, 0, 0x3F, 0];
        // Clean around the Exif, and the same without SOS and EOI.
        let mut clean = vec![0xFF, 0xD8];
        clean.extend_from_slice(&app1);
        clean.extend_from_slice(&sos);
        clean.extend_from_slice(&[0xFF, 0xD9]);
        exercise_jpeg(&clean);
        let mut open = vec![0xFF, 0xD8];
        open.extend_from_slice(&app1);
        exercise_jpeg(&open);
        wrapped += 1;
    }
    assert!(wrapped >= SEEDS.len(), "wrapped only {wrapped} TIFF seeds");
}

/// Shapes for each guard of the SPEC-010 extraction, one guard at a time, so
/// the shared invariants see every single-guard variant a planted bug could
/// leave behind (the seeds alone overlap: several guards would catch the same
/// input). Each is a clean-walking JPEG around the given segments.
#[test]
fn jpeg_harness_holds_on_each_extraction_guard_alone() {
    fn seg(marker: u8, payload: &[u8]) -> Vec<u8> {
        let l = u16::try_from(payload.len() + 2).expect("short");
        let mut v = vec![0xFF, marker, (l >> 8) as u8, l as u8];
        v.extend_from_slice(payload);
        v
    }
    fn jpeg(segs: &[Vec<u8>]) -> Vec<u8> {
        let mut d = vec![0xFF, 0xD8];
        for s in segs {
            d.extend_from_slice(s);
        }
        d.extend_from_slice(&seg(0xDA, &[1, 1, 0, 0, 0x3F, 0]));
        d.extend_from_slice(&[0xFF, 0xD9]);
        d
    }
    let icc = |seq: u8, count: u8| seg(0xE2, &[&b"ICC_PROFILE\0"[..], &[seq, count, 9]].concat());
    // ICC: each fault alone, and the consistent orders.
    for chunks in [
        vec![(1, 2), (2, 2)],
        vec![(2, 2), (1, 2)],
        vec![(1, 2), (2, 3)], // count differs, seqs fine
        vec![(1, 3), (2, 2)], // first is the odd one out
        vec![(0, 2), (1, 2)], // seq 0 only
        vec![(1, 2), (0, 2)],
        vec![(1, 2), (3, 2)], // seq above count only
        vec![(1, 2), (1, 2)], // repeated seq only
        vec![(1, 3), (2, 3)], // count 3, two chunks
        vec![(1, 2), (2, 2), (2, 2)],
        vec![(1, 1)],
        vec![(1, 0)],
    ] {
        let segs: Vec<Vec<u8>> = chunks.iter().map(|&(s, c)| icc(s, c)).collect();
        exercise_jpeg(&jpeg(&segs));
    }
    // Extended XMP either side of the 75-byte boundary (35 + 40).
    let ext = |n: usize| {
        seg(
            0xE1,
            &[&b"http://ns.adobe.com/xmp/extension/\0"[..], &vec![5u8; n]].concat(),
        )
    };
    for n in [0, 39, 40, 41] {
        exercise_jpeg(&jpeg(&[ext(n)]));
    }
    // JUMBF and ICC headers either side of their limits.
    for n in [0, 5, 6, 7] {
        exercise_jpeg(&jpeg(&[seg(0xEB, &[&b"JP"[..], &vec![1u8; n]].concat())]));
    }
    for n in [0, 1, 2, 3] {
        exercise_jpeg(&jpeg(&[seg(
            0xE2,
            &[&b"ICC_PROFILE\0"[..], &vec![1u8; n]].concat(),
        )]));
    }
    // IPTC: name and data parity, a missing last pad, a short size, stray bytes.
    let res = |id: u16, name: &[u8], data: &[u8], pad: bool| {
        let mut v = b"8BIM".to_vec();
        v.extend_from_slice(&id.to_be_bytes());
        v.push(name.len() as u8);
        v.extend_from_slice(name);
        if (1 + name.len()) % 2 == 1 {
            v.push(0);
        }
        v.extend_from_slice(&(data.len() as u32).to_be_bytes());
        v.extend_from_slice(data);
        if pad && data.len() % 2 == 1 {
            v.push(0);
        }
        v
    };
    let ps = |body: &[u8]| seg(0xED, &[&b"Photoshop 3.0\0"[..], body].concat());
    for name in [&b""[..], b"a", b"ab", b"abc"] {
        for data in [&b""[..], b"1", b"12", b"123"] {
            let body = [res(0x0408, name, data, true), res(0x0404, name, data, true)].concat();
            exercise_jpeg(&jpeg(&[ps(&body)]));
            // The last resource without its pad byte.
            exercise_jpeg(&jpeg(&[ps(&res(0x0404, name, data, false))]));
        }
    }
    let good = res(0x0404, b"x", b"abc", true);
    for cut in 1..good.len() {
        exercise_jpeg(&jpeg(&[ps(&[
            res(0x0404, b"", b"z", true),
            good[..cut].to_vec(),
        ]
        .concat())]));
    }
    exercise_jpeg(&jpeg(&[ps(&[good.clone(), b"JUNK".to_vec()].concat())]));
    exercise_jpeg(&jpeg(&[ps(&[good.clone(), b"8B".to_vec()].concat())]));
    // Exif: signature only, junk, a second and a 17th segment.
    let exif = |tiff: &[u8]| seg(0xE1, &[&b"Exif\0\0"[..], tiff].concat());
    exercise_jpeg(&jpeg(&[exif(b"")]));
    exercise_jpeg(&jpeg(&[
        exif(b"NOPE"),
        exif(b"II*\0\x08\0\0\0\0\0\0\0\0\0"),
    ]));
    for n in [1, 2, 16, 17, 18] {
        let segs: Vec<Vec<u8>> = (0..n)
            .map(|_| exif(b"II*\0\x08\0\0\0\0\0\0\0\0\0"))
            .collect();
        exercise_jpeg(&jpeg(&segs));
    }
    // A halt: two Exif segments, then 1100 stray FF00 pairs (issue cap), and
    // 4097 short segments (segment cap). Nothing may be recorded after either.
    let mut d = vec![0xFF, 0xD8];
    d.extend_from_slice(&exif(b"II*\0\x08\0\0\0\0\0\0\0\0\0"));
    d.extend_from_slice(&exif(b"II*\0\x08\0\0\0\0\0\0\0\0\0"));
    for _ in 0..1100 {
        d.extend_from_slice(&[0xFF, 0x00]);
    }
    exercise_jpeg(&d);
    let mut d = vec![0xFF, 0xD8];
    for _ in 0..4097 {
        d.extend_from_slice(&ext(0));
    }
    exercise_jpeg(&d);
    // Not halted, with extraction issues after walker issues.
    let mut d = vec![0xFF, 0xD8, 0x00, 0x01];
    for _ in 0..50 {
        d.extend_from_slice(&ext(3));
    }
    exercise_jpeg(&d);
}

// ── the jpeg_segments harness body, mirrored from the fuzz target ───────────

/// The kind a segment must have, written out independently of the library.
fn expected_kind(marker: u8, payload: &[u8]) -> SegmentKind {
    match marker {
        0xE0 if payload.starts_with(b"JFIF\0") => SegmentKind::Jfif,
        // DEC-007: five signature bytes and at least six bytes of payload.
        0xE1 if payload.len() >= 6 && payload.starts_with(b"Exif\0") => SegmentKind::Exif,
        0xE1 if payload.starts_with(b"http://ns.adobe.com/xap/1.0/\0") => SegmentKind::Xmp,
        0xE1 if payload.starts_with(b"http://ns.adobe.com/xmp/extension/\0") => {
            SegmentKind::ExtendedXmp
        }
        0xE2 if payload.starts_with(b"ICC_PROFILE\0") => SegmentKind::Icc,
        0xED if payload.starts_with(b"Photoshop 3.0\0") => SegmentKind::Iptc,
        0xEB if payload.starts_with(b"JP") => SegmentKind::Jumbf,
        0xE0..=0xEF => SegmentKind::App(marker - 0xE0),
        0xC4 => SegmentKind::Dht,
        0xC8 | 0xCC => SegmentKind::Other(marker),
        0xC0..=0xCF => SegmentKind::Sof(marker),
        0xDB => SegmentKind::Dqt,
        0xDD => SegmentKind::Dri,
        0xFE => SegmentKind::Com,
        0xDA => SegmentKind::Sos,
        _ => SegmentKind::Other(marker),
    }
}

// The SPEC-010 `jpeg::read` invariants, shared with the fuzz target.
include!("../fuzz/shared/jpeg_metadata_invariants.rs");

fn exercise_jpeg(data: &[u8]) {
    metadata_invariants::check(data);
    let strict = Jpeg::parse_strict(data);
    let jpeg = match Jpeg::parse(data) {
        Ok(jpeg) => jpeg,
        Err(error) => {
            // A rejected input is a SUCCESS: both entry points say the same.
            assert_eq!(
                strict.err(),
                Some(error),
                "parse_strict disagrees on a fatal input"
            );
            return;
        }
    };

    // Differential, both directions.
    assert_eq!(
        strict.is_ok(),
        jpeg.is_clean(),
        "parse_strict Ok iff is_clean"
    );
    assert_eq!(jpeg.is_clean(), jpeg.issues().is_empty());
    if let Err(error) = strict {
        let first = jpeg
            .issues()
            .first()
            .expect("parse_strict Err on a jpeg with no issues");
        assert_eq!(&error, first.error(), "strict error is not the first issue");
    }

    let len = data.len();

    // Segments: framing, containment, order, zero-copy.
    let mut prev_end = 2usize;
    for seg in jpeg.segments() {
        let off = seg.offset();
        let payload = seg.payload();
        assert!(off >= prev_end, "segment overlaps the previous one");
        assert_eq!(data.get(off), Some(&0xFF), "segment does not start at FF");
        assert_eq!(data.get(off + 1), Some(&seg.marker()), "marker mismatch");
        let declared = data
            .get(off + 2..off + 4)
            .map(|b| usize::from(u16::from_be_bytes([b[0], b[1]])));
        assert_eq!(declared, Some(payload.len() + 2), "length field mismatch");
        let end = off + 4 + payload.len();
        assert!(end <= len, "segment escapes the input");
        assert_eq!(
            payload.as_ptr(),
            data[off + 4..].as_ptr(),
            "payload is not at offset + 4"
        );
        assert!(lies_inside(data, payload), "payload escapes the input");
        prev_end = end;

        // Kind: each signature kind holds iff marker and signature match.
        let want = expected_kind(seg.marker(), payload);
        assert_eq!(seg.kind(), want, "kind");
        let k = seg.kind();
        let m = seg.marker();
        assert_eq!(
            k == SegmentKind::Exif,
            m == 0xE1 && payload.len() >= 6 && payload.starts_with(b"Exif\0"),
            "Exif iff APP1, at least 6 bytes and the first five are Exif\\0 (DEC-007)"
        );
        assert_eq!(
            k == SegmentKind::Xmp,
            m == 0xE1 && payload.starts_with(b"http://ns.adobe.com/xap/1.0/\0")
        );
        assert_eq!(
            k == SegmentKind::ExtendedXmp,
            m == 0xE1 && payload.starts_with(b"http://ns.adobe.com/xmp/extension/\0")
        );
        assert_eq!(
            k == SegmentKind::Icc,
            m == 0xE2 && payload.starts_with(b"ICC_PROFILE\0")
        );
        assert_eq!(
            k == SegmentKind::Iptc,
            m == 0xED && payload.starts_with(b"Photoshop 3.0\0")
        );
        assert_eq!(
            k == SegmentKind::Jumbf,
            m == 0xEB && payload.starts_with(b"JP")
        );
        assert_eq!(
            k == SegmentKind::Jfif,
            m == 0xE0 && payload.starts_with(b"JFIF\0")
        );
        assert_eq!(k == SegmentKind::Sos, m == 0xDA);
        let _ = (format!("{k}"), format!("{k:?}"));
    }
    assert!(jpeg.segments().len() <= MAX_JPEG_SEGMENTS);

    // Scan region, EOI and trailer.
    let first_sos = jpeg
        .segments()
        .iter()
        .find(|s| s.kind() == SegmentKind::Sos);
    assert_eq!(jpeg.scan_region().is_some(), first_sos.is_some());
    if let (Some(region), Some(sos)) = (jpeg.scan_region(), first_sos) {
        assert_eq!(region.get(..2), Some(&[0xFF, 0xDA][..]), "region start");
        assert_eq!(
            region.as_ptr(),
            data[sos.offset()..].as_ptr(),
            "region is not at the first SOS"
        );
        let end = jpeg.eoi_offset().map_or(len, |e| e + 2);
        assert_eq!(sos.offset() + region.len(), end, "region end");
    }
    match jpeg.eoi_offset() {
        None => assert!(jpeg.trailer().is_empty(), "trailer without EOI"),
        Some(eoi) => {
            assert_eq!(data.get(eoi..eoi + 2), Some(&[0xFF, 0xD9][..]), "EOI bytes");
            assert_eq!(jpeg.trailer(), &data[eoi + 2..], "trailer");
            assert!(
                jpeg.segments().iter().all(|s| s.offset() < eoi),
                "a segment after EOI"
            );
        }
    }
    assert!(lies_inside(data, jpeg.trailer()));

    // after_scan iff the segment is after the first SOS segment.
    for seg in jpeg.segments() {
        let expect = first_sos.is_some_and(|sos| seg.offset() > sos.offset());
        assert_eq!(seg.after_scan(), expect, "after_scan iff after first SOS");
    }

    // Issues: bounds, halts, and the end-of-walk pair.
    let errors: Vec<&Error> = jpeg.issues().iter().map(|i| i.error()).collect();
    assert!(errors.len() <= MAX_JPEG_ISSUES + 1);
    for issue in jpeg.issues() {
        assert!(issue.offset() <= len, "issue offset past the input");
        assert!(!issue.to_string().is_empty());
        let _ = format!("{issue:?}");
    }
    let halt = errors.iter().position(|e| {
        matches!(
            e,
            Error::TooManySegments { .. } | Error::TooManyIssues { .. }
        )
    });
    if let Some(idx) = halt {
        assert_eq!(idx + 1, errors.len(), "issue recorded after a halt");
    }
    assert_eq!(
        errors
            .iter()
            .any(|e| matches!(e, Error::TooManyIssues { .. })),
        errors.len() == MAX_JPEG_ISSUES + 1,
        "TooManyIssues iff the list is one over the cap"
    );
    // The segment cap halts only a full list.
    if errors
        .iter()
        .any(|e| matches!(e, Error::TooManySegments { .. }))
    {
        assert_eq!(
            jpeg.segments().len(),
            MAX_JPEG_SEGMENTS,
            "cap on a short list"
        );
    }
    let has_sos = first_sos.is_some();
    let has_eoi = jpeg.eoi_offset().is_some();
    let missing_sos = errors.iter().any(|e| matches!(e, Error::MissingSos));
    let missing_eoi = errors.iter().any(|e| matches!(e, Error::MissingEoi));
    if let Some(idx) = halt {
        // A halt inside the walk suppresses the end-of-walk pair. The one
        // exception is the issue cap firing INSIDE the end-of-walk step
        // (1023 ordinary issues, then MissingSos is the 1024th and MissingEoi
        // becomes TooManyIssues): the pair is then a contiguous run directly
        // before the halting issue, and each member is still true.
        let tail = errors.get(..idx).unwrap_or(&[]);
        let pair = tail
            .iter()
            .filter(|e| matches!(e, Error::MissingSos | Error::MissingEoi))
            .count();
        if matches!(errors.get(idx), Some(Error::TooManySegments { .. })) {
            assert_eq!(pair, 0, "end-of-walk issue after the segment cap");
        } else {
            assert!(
                tail.iter()
                    .rev()
                    .take(pair)
                    .all(|e| matches!(e, Error::MissingSos | Error::MissingEoi)),
                "end-of-walk issues are not directly before the cap"
            );
            assert!(!missing_sos || !has_sos, "MissingSos but there is an SOS");
            assert!(!missing_eoi || !has_eoi, "MissingEoi but there is an EOI");
        }
    } else {
        assert_eq!(missing_sos, !has_sos, "MissingSos iff no SOS segment");
        assert_eq!(missing_eoi, !has_eoi, "MissingEoi iff no EOI");
    }
}

// ── SPEC-012: the `audit` target ─────────────────────────────────────────────

include!("../fuzz/shared/audit_invariants.rs");

use audit_invariants::{EmbF, Facts, FindingF, ItemF, ThumbF, What, CAP};

/// All 66 seeds that existed before SPEC-012 (29 `tiff_ifd` + 37
/// `jpeg_segments`): `(target, name, length, FNV-1a 64 of the bytes)`, taken
/// from the files committed at HEAD before SPEC-012 changed anything.
/// Regenerating the seeds (which now also writes `fuzz/seeds/audit/`) must
/// leave every one of them byte-identical.
const PRE_SPEC_012_SEEDS: [(&str, &str, usize, u64); 66] = [
    ("tiff_ifd", "ascii-count-past-eof", 79, 0xf9fb168ecf63c873),
    ("tiff_ifd", "ascii-shapes", 161, 0x40686d2b31db9f20),
    ("tiff_ifd", "chain", 62, 0x5103cafa5ffe6da1),
    ("tiff_ifd", "chain-be", 62, 0x0e7a1e63dbe9517d),
    ("tiff_ifd", "corrupt-subifd", 38, 0x8dad61eefdeaa933),
    ("tiff_ifd", "count-overflow", 86, 0x0ca614f0bde8ce1c),
    ("tiff_ifd", "count-overflow-be", 86, 0x46a5d940e364b10e),
    ("tiff_ifd", "deep-nesting", 356, 0x86bc5a943f3da84d),
    ("tiff_ifd", "depth-limit-exact", 266, 0x2d1ea0dcff4e5a83),
    ("tiff_ifd", "entry-count-past-eof", 13, 0x78590c388acc9606),
    ("tiff_ifd", "exif-gps-interop", 116, 0x662b025c31b0e761),
    ("tiff_ifd", "exif-points-at-ifd0", 38, 0xbd387c33c1a06741),
    ("tiff_ifd", "gps-past-eof-exif-ok", 68, 0xc46df5c8bc378c6b),
    ("tiff_ifd", "ifd0-offset-zero", 8, 0x432d45b4d7c255d5),
    ("tiff_ifd", "leaf-next-word", 74, 0x85063aa66064a89b),
    ("tiff_ifd", "malformed-subifds-tag", 56, 0xb547aa14c20643ba),
    ("tiff_ifd", "next-past-eof", 26, 0x802bbf014b971d79),
    ("tiff_ifd", "payload-past-eof", 50, 0x24228b3502290f30),
    ("tiff_ifd", "payload-past-eof-be", 50, 0xb936734a88d49c7a),
    ("tiff_ifd", "pointer-bad-count", 50, 0x41444d715968b875),
    ("tiff_ifd", "self-cycle", 26, 0x7e9d0243fc17108b),
    ("tiff_ifd", "signed-and-float", 226, 0xc64c538111f83ebb),
    ("tiff_ifd", "subifd", 82, 0x6f3e5be266b40257),
    ("tiff_ifd", "subifd-be", 82, 0xb367930d24976e8b),
    ("tiff_ifd", "subifds-count-over-cap", 38, 0x29ca58ae79ebcb95),
    ("tiff_ifd", "truncated-header", 5, 0x50e0a90335f50b17),
    ("tiff_ifd", "two-hop-cycle", 68, 0xba54c41e6b2198b3),
    ("tiff_ifd", "valid-be", 126, 0xd14e46fc095cffa9),
    ("tiff_ifd", "valid-le", 126, 0x80fe123345fe5215),
    ("jpeg_segments", "app-after-scan", 39, 0x3c0b74c4ea41965e),
    ("jpeg_segments", "app1-len-0", 24, 0xa71f9b333b778368),
    ("jpeg_segments", "app1-len-1", 24, 0xd3cb0a5e7ebf3f1f),
    ("jpeg_segments", "app1-len-2", 18, 0x03f1c23d02e437f0),
    ("jpeg_segments", "app1-len-ffff", 65551, 0xc760bd59985fc432),
    ("jpeg_segments", "app1-past-eof", 16, 0xf7f17c5736b2e083),
    ("jpeg_segments", "exif-after-scan", 148, 0x10ad14693c9e9c6b),
    ("jpeg_segments", "exif-and-xmp", 84, 0xadb9ce26dd6e2b38),
    ("jpeg_segments", "exif-bad-tiff", 32, 0xb11d03c3f7ca9288),
    (
        "jpeg_segments",
        "exif-five-byte-payload",
        23,
        0xe5ae7fec035eef93,
    ),
    ("jpeg_segments", "exif-in-jpeg", 148, 0x7c21afba5dea8c89),
    (
        "jpeg_segments",
        "exif-sixth-byte-nonzero",
        148,
        0x929d86c9b12852c6,
    ),
    (
        "jpeg_segments",
        "extended-xmp-pair",
        149,
        0x8768c0104678fa1d,
    ),
    (
        "jpeg_segments",
        "extended-xmp-short",
        171,
        0xc2fb6f7d331f2226,
    ),
    ("jpeg_segments", "ff00-stray", 28, 0x329ecc00b7b39d86),
    ("jpeg_segments", "fill-bytes", 35, 0x0c6af3c3636b50db),
    (
        "jpeg_segments",
        "icc-chunks-mismatch",
        77,
        0xcf9d799ac0d0f1e6,
    ),
    ("jpeg_segments", "icc-chunks-ok", 61, 0xd70da14137a10c3a),
    ("jpeg_segments", "icc-iptc-jumbf", 73, 0xbe84bb0a5f3b3ec3),
    ("jpeg_segments", "icc-seq-zero", 36, 0x49e811923270b17b),
    ("jpeg_segments", "iptc-8bim", 68, 0xc12b2f98a1b99274),
    (
        "jpeg_segments",
        "iptc-truncated-resource",
        61,
        0xd533a41f8b100bb4,
    ),
    ("jpeg_segments", "jumbf-header", 30, 0xbca67c1eeca4ff73),
    ("jpeg_segments", "jumbf-short", 25, 0xb3cae112b487977c),
    ("jpeg_segments", "many-ff00", 2202, 0x1a0b709fa81c45f2),
    ("jpeg_segments", "many-segments", 16414, 0xc675c75e5ea8ad5e),
    ("jpeg_segments", "minimal", 68, 0x47af3ff740559a4a),
    ("jpeg_segments", "no-sos", 28, 0x1bdded91eb609b96),
    (
        "jpeg_segments",
        "progressive-two-scans",
        55,
        0xd126899024ddea18,
    ),
    ("jpeg_segments", "seventeen-exif", 2173, 0x6e50da0b2ab0c175),
    (
        "jpeg_segments",
        "signatures-under-wrong-marker",
        168,
        0xe0544e0816d47269,
    ),
    ("jpeg_segments", "soi-only", 2, 0x0a99ee07b6f68642),
    ("jpeg_segments", "sos-no-eoi", 19, 0x0e30d944ac162394),
    ("jpeg_segments", "stray-bytes", 34, 0xcaad01d80bb242c1),
    ("jpeg_segments", "trailer-after-eoi", 36, 0x10a1ad72a76b0149),
    ("jpeg_segments", "two-exif", 277, 0x546b67de32768525),
    (
        "jpeg_segments",
        "xmp-and-extended-xmp",
        228,
        0x7cd5037ca5f8982e,
    ),
];

#[test]
fn the_existing_66_seeds_are_byte_identical() {
    assert_eq!(PRE_SPEC_012_SEEDS.len(), 66);
    for (target, name, len, hash) in PRE_SPEC_012_SEEDS {
        let path = root().join("fuzz/seeds").join(target).join(name);
        let bytes = fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(bytes.len(), len, "{target}/{name}: length");
        assert_eq!(fnv1a_64(&bytes), hash, "{target}/{name}: contents changed");
    }
    // Nothing was added to or removed from the two older targets, except the
    // three SPEC-017 `tiff_ifd` seeds.
    let count = |t: &str| {
        fs::read_dir(root().join("fuzz/seeds").join(t))
            .unwrap_or_else(|e| panic!("{t}: {e}"))
            .count()
    };
    assert_eq!(count("tiff_ifd"), 32);
    assert_eq!(count("jpeg_segments"), 37 + SHAPE_SEEDS.len());
}

#[test]
fn audit_target_is_declared_and_runs_in_ci() {
    let fuzz = read("fuzz/Cargo.toml");
    assert!(
        fuzz.contains("name = \"audit\"") && fuzz.contains("fuzz_targets/audit.rs"),
        "fuzz/Cargo.toml must declare the audit bin"
    );
    assert!(root().join("fuzz/fuzz_targets/audit.rs").is_file());
    assert!(root().join("fuzz/shared/audit_invariants.rs").is_file());
    let ci = read(".github/workflows/ci.yml");
    assert!(
        ci.contains(
            "target: [tiff_ifd, jpeg_segments, rewrite_jpeg, audit, audit_render, cli_args]"
        ),
        "the fuzz-smoke matrix must run the audit target"
    );
}

fn audit_seed(name: &str) -> Vec<u8> {
    let path = root().join("fuzz/seeds/audit").join(name);
    fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

const AUDIT_NEW_SEEDS: [&str; 9] = [
    "audit-png-magic",
    "audit-bigtiff-magic",
    "audit-empty-marker",
    "audit-exif-gps-and-thumbnail",
    "audit-two-blocks-same-tag",
    "audit-ifd1-strips",
    "audit-many-overlapping-ifds",
    // SPEC-015 (carried SPEC-014 F1): the two full-house inputs.
    "audit-full-house-tiff",
    "audit-full-house-jpeg",
];

#[test]
fn audit_seeds_are_committed_and_have_the_shapes_they_claim() {
    use exif_core::audit::{audit, Category};

    // The 66 older seeds are copied byte for byte under a prefix, and the
    // directory holds exactly those 66 plus the nine new shapes.
    let dir = root().join("fuzz/seeds/audit");
    let mut names: Vec<String> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    let mut want: Vec<String> = SEEDS.iter().map(|n| format!("tiff-{n}")).collect();
    want.extend(JPEG_SEEDS.iter().map(|n| format!("jpeg-{n}")));
    want.extend(AUDIT_NEW_SEEDS.iter().map(|n| n.to_string()));
    // SPEC-017: the three new `tiff_ifd` seeds are copied under the same prefix.
    for n in [
        "entry-budget-exact",
        "entry-budget-over",
        "overlapping-subifds",
    ] {
        want.push(format!("tiff-{n}"));
    }
    // SPEC-019: the shape seeds, written as they are.
    want.extend(SHAPE_SEEDS.iter().map(|n| format!("shape-{n}")));
    want.sort();
    assert_eq!(
        names, want,
        "fuzz/seeds/audit holds exactly the expected seeds"
    );
    for n in SEEDS {
        let old = fs::read(root().join("fuzz/seeds/tiff_ifd").join(n)).expect("old seed");
        assert_eq!(audit_seed(&format!("tiff-{n}")), old, "tiff-{n} is a copy");
    }
    for n in JPEG_SEEDS {
        let old = fs::read(root().join("fuzz/seeds/jpeg_segments").join(n)).expect("old seed");
        assert_eq!(audit_seed(&format!("jpeg-{n}")), old, "jpeg-{n} is a copy");
    }

    // Each new shape reaches what its name promises.
    let kind = |b: &[u8]| format!("{:?}", audit(b).kind());
    assert_eq!(kind(&audit_seed("audit-png-magic")), "Unsupported(Png)");
    assert_eq!(
        kind(&audit_seed("audit-bigtiff-magic")),
        "Unsupported(BigTiff)"
    );
    let empty = audit_seed("audit-empty-marker");
    assert!(empty.is_empty(), "a zero-byte seed (libFuzzer accepts it)");
    assert_eq!(kind(&empty), "Unsupported(Empty)");

    let rich = audit_seed("audit-exif-gps-and-thumbnail");
    let a = audit(&rich);
    assert_eq!(kind(&rich), "Jpeg");
    assert!(a.health().clean, "{:?}", a.health());
    assert_eq!(a.findings_in(Category::Gps).count(), 2);
    assert_eq!(a.findings_in(Category::Thumbnail).count(), 1);
    let thumb = a
        .findings_in(Category::Thumbnail)
        .next()
        .expect("thumbnail");
    let range = thumb.thumbnail.expect("range");
    assert!(range.range.is_some() && range.starts_with_ffd8, "{range:?}");
    assert_eq!(a.findings_in(Category::Timestamps).count(), 1);
    assert_eq!(a.findings_in(Category::CameraModel).count(), 1);

    let two = audit_seed("audit-two-blocks-same-tag");
    let a = audit(&two);
    assert_eq!(a.findings_in(Category::CameraModel).count(), 2);
    let blocks: Vec<Option<usize>> = a
        .findings_in(Category::CameraModel)
        .map(|f| f.at.origin().block)
        .collect();
    assert_eq!(blocks, vec![Some(0), Some(1)]);

    // An IFD1 of strips with no JPEG pointer: a Thumbnail finding on 0x0111,
    // with no range (the shape the real-target plant needed; see the handback).
    let strips = audit_seed("audit-ifd1-strips");
    let a = audit(&strips);
    assert!(a.health().clean, "{:?}", a.health());
    let t: Vec<_> = a.findings_in(Category::Thumbnail).collect();
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].thumbnail.expect("range").range, None);

    // The amplification shape at survivable size: 255 overlapping IFDs of 767
    // entries from a file of about 11 KB. Before SPEC-017 it parsed to 256 IFDs
    // and 195,586 entries and overflowed the inventory cap; the entry budget
    // (`len / 12`) now keeps IFD0 and the first sub-IFD and omits the second,
    // so the file is still the same bytes and the answer is a bounded one.
    let many = audit_seed("audit-many-overlapping-ifds");
    assert!(many.len() < 12 * 1024, "{} bytes", many.len());
    let c = Container::parse(&many).expect("parses");
    assert_eq!(c.ifds().len(), 2);
    let entries: usize = c.ifds().iter().map(|i| i.entries().len()).sum();
    assert_eq!(entries, 1 + 767);
    assert!(entries <= many.len() / 12, "within the entry budget");
    assert!(matches!(
        c.issues().last().map(|i| i.error()),
        Some(Error::TooManyEntries { .. })
    ));
    let a = audit(&many);
    assert_eq!(a.items().len(), 1 + 767);
    assert!(!a.health().inventory_truncated);
    assert!(!a.health().clean, "a TooManyEntries issue is not clean");
    assert_eq!(a.health().exif_issues, 1);
}

fn audit_degenerate_inputs() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        vec![],
        vec![0],
        vec![0xFF],
        vec![0xFF, 0xD8],
        vec![0xFF, 0xD8, 0xFF],
        b"II*\0".to_vec(),
        b"MM\0*".to_vec(),
        b"II*\0\x08\0\0\0".to_vec(),
        b"II+\0".to_vec(),
        b"RIFF\0\0\0\0WEBP".to_vec(),
        b"RIFF\0\0\0\0WEB".to_vec(),
        b"\0\0\0\x18ftyp".to_vec(),
        b"GIF89a".to_vec(),
        vec![0; 12],
        vec![0xFF; 64],
        vec![0xFF; 70_000],
    ];
    // Every prefix of the rich seed, to hit every truncation point.
    let rich = audit_seed("audit-exif-gps-and-thumbnail");
    for n in 0..rich.len() {
        v.push(rich[..n].to_vec());
    }
    v
}

#[test]
fn audit_harness_holds_on_every_seed_and_degenerate_inputs() {
    let dir = root().join("fuzz/seeds/audit");
    let mut seen = 0usize;
    for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let path = entry.expect("entry").path();
        let data = fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        audit_invariants::check(&data);
        seen += 1;
    }
    assert_eq!(
        seen,
        78 + 17,
        "32 TIFF + 37 JPEG + 9 new + 17 SPEC-019 shapes"
    );
    for data in audit_degenerate_inputs() {
        audit_invariants::check(&data);
    }
}

/// One planted bug: a wrong answer put into the library's flattened answer for
/// `input`, which the named guard, run ALONE, must reject.
/// A plant input built in the test rather than read from a seed: the one
/// honest way left to overflow the inventory cap (65,536 items) is a TIFF of
/// about 786 KB with more than 65,536 real, disjoint entries, which is too big
/// to commit (and no longer reachable with overlapping tables: SPEC-017).
const GENERATED_BIG_INVENTORY: &str = "generated:big-inventory";

/// 65,537 disjoint entries (an unremarkable tag, so no finding) in two IFDs:
/// IFD0 with 65,535 and an IFD1 with 2, in 786,464 bytes (budget 65,538).
fn big_inventory() -> Vec<u8> {
    let first: usize = 65_535;
    let mut out = b"II\x2A\x00\x08\x00\x00\x00".to_vec();
    out.extend_from_slice(&u16::try_from(first).expect("u16").to_le_bytes());
    for _ in 0..first {
        out.extend_from_slice(&0xC000u16.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&[0, 0, 0, 0]);
    }
    let ifd1_at = u32::try_from(out.len() + 4).expect("u32");
    out.extend_from_slice(&ifd1_at.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    for _ in 0..2 {
        out.extend_from_slice(&0xC000u16.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&[0, 0, 0, 0]);
    }
    out.extend_from_slice(&0u32.to_le_bytes());
    out
}

fn plant_input(name: &str) -> Vec<u8> {
    if name == GENERATED_BIG_INVENTORY {
        big_inventory()
    } else {
        audit_seed(name)
    }
}

#[test]
fn the_generated_big_inventory_really_overflows_the_cap() {
    use exif_core::audit::audit;
    let data = big_inventory();
    let c = Container::parse(&data).expect("parses");
    assert!(c.is_clean(), "{:?}", c.issues());
    assert_eq!(c.entry_count(), 65_537);
    assert!(c.entry_count() <= exif_core::tiff::entry_budget(data.len()));
    let a = audit(&data);
    assert_eq!(a.items().len(), CAP);
    assert!(a.health().inventory_truncated);
}

struct Plant {
    name: &'static str,
    input: &'static str,
    guard: &'static str,
    apply: fn(&mut Facts),
}

fn first_tag_finding(f: &Facts) -> usize {
    f.findings
        .iter()
        .position(|x| matches!(x.at.what, What::Tag { .. }) && x.category != "Gps")
        .expect("a non-GPS tag finding")
}

fn first_region_finding(f: &Facts) -> usize {
    f.findings
        .iter()
        .position(|x| matches!(x.at.what, What::Region { .. }))
        .expect("a region finding")
}

fn first_gps_finding(f: &Facts) -> usize {
    f.findings
        .iter()
        .position(|x| x.category == "Gps")
        .expect("a Gps finding")
}

fn thumb_finding(f: &Facts) -> usize {
    f.findings
        .iter()
        .position(|x| x.category == "Thumbnail")
        .expect("a Thumbnail finding")
}

fn dummy_item() -> ItemF {
    ItemF {
        block: None,
        segment: None,
        after_scan: false,
        what: What::Tag {
            ifd: "Ifd0".into(),
            ifd_index: 0,
            tag: 0x010F,
            field_type: 2,
            count: 1,
        },
    }
}

/// The index of the first finding of a category.
fn finding_of(f: &Facts, category: &str) -> usize {
    f.findings
        .iter()
        .position(|x| x.category == category)
        .unwrap_or_else(|| panic!("a {category} finding"))
}

/// SPEC-019 plants: the new region kinds, categories and the `embedded`
/// object, each rejected by its own guard alone, in both directions (a thing
/// expected that is missing, a thing present that was not expected).
fn new_region_plants() -> Vec<Plant> {
    vec![
        // embedded: the object lost / invented (the two directions of iff).
        Plant {
            name: "an EmbeddedImage finding without its embedded object",
            input: "shape-mpo-two-le",
            guard: "embedded",
            apply: |f| {
                let i = finding_of(f, "EmbeddedImage");
                f.findings[i].embedded = None;
            },
        },
        Plant {
            name: "an embedded object on a camera-model finding",
            input: "shape-mpo-two-le",
            guard: "embedded",
            apply: |f| {
                let e = f.findings[finding_of(f, "EmbeddedImage")].embedded.clone();
                let i = finding_of(f, "CameraModel");
                f.findings[i].embedded = e;
            },
        },
        Plant {
            name: "an embedded object on a tag finding of a TIFF",
            input: "tiff-valid-le",
            guard: "embedded",
            apply: |f| {
                let i = first_tag_finding(f);
                f.findings[i].embedded = Some(EmbF {
                    offset: 0,
                    length: 0,
                    ffd8: false,
                    holds: vec![],
                });
            },
        },
        Plant {
            name: "an EmbeddedImage finding dropped but its item kept",
            input: "shape-mpo-two-le",
            guard: "embedded",
            apply: |f| {
                let i = finding_of(f, "EmbeddedImage");
                f.findings.remove(i);
            },
        },
        Plant {
            name: "an embedded range outside the input",
            input: "shape-mpo-two-le",
            guard: "embedded",
            apply: |f| {
                let i = finding_of(f, "EmbeddedImage");
                f.findings[i].embedded.as_mut().unwrap().offset = usize::MAX - 3;
            },
        },
        Plant {
            name: "an embedded range that runs past the end",
            input: "shape-mpo-two-le",
            guard: "embedded",
            apply: |f| {
                let i = finding_of(f, "EmbeddedImage");
                f.findings[i].embedded.as_mut().unwrap().length += 1_000_000;
            },
        },
        Plant {
            name: "the FF D8 flag flipped",
            input: "shape-mpo-two-le",
            guard: "embedded",
            apply: |f| {
                let i = finding_of(f, "EmbeddedImage");
                let e = f.findings[i].embedded.as_mut().unwrap();
                e.ffd8 = !e.ffd8;
            },
        },
        Plant {
            name: "holds out of table order",
            input: "shape-mpo-two-le",
            guard: "embedded",
            apply: |f| {
                let i = finding_of(f, "EmbeddedImage");
                f.findings[i].embedded.as_mut().unwrap().holds.reverse();
            },
        },
        Plant {
            name: "holds listing EmbeddedImage",
            input: "shape-mpo-two-le",
            guard: "embedded",
            apply: |f| {
                let i = finding_of(f, "EmbeddedImage");
                f.findings[i]
                    .embedded
                    .as_mut()
                    .unwrap()
                    .holds
                    .push("EmbeddedImage".into());
            },
        },
        // findings: holds compared with the independent derivation (both ways).
        Plant {
            name: "holds emptied",
            input: "shape-mpo-two-le",
            guard: "findings",
            apply: |f| {
                let i = finding_of(f, "EmbeddedImage");
                f.findings[i].embedded.as_mut().unwrap().holds.clear();
            },
        },
        Plant {
            name: "holds with a category that is not inside",
            input: "shape-mpo-two-le",
            guard: "findings",
            apply: |f| {
                let i = finding_of(f, "EmbeddedImage");
                f.findings[i].embedded.as_mut().unwrap().holds = vec!["Gps".into()];
            },
        },
        Plant {
            name: "an embedded image whose inner MPF is recursed into",
            input: "shape-mpo-inner-mpf",
            guard: "findings",
            apply: |f| {
                // With location on inside, the inner MPF would be an
                // EmbeddedImage (not an OtherSegment) in `holds`.
                let i = finding_of(f, "EmbeddedImage");
                let e = f.findings[i].embedded.as_mut().unwrap();
                e.holds = e
                    .holds
                    .iter()
                    .map(|h| {
                        if h == "OtherSegment" {
                            "Gps".to_string()
                        } else {
                            h.clone()
                        }
                    })
                    .collect();
            },
        },
        // category_shape: the region-kind to category map of DEC-012.
        Plant {
            name: "an EmbeddedImage region categorised Xmp",
            input: "shape-mpo-two-le",
            guard: "category_shape",
            apply: |f| {
                let i = finding_of(f, "EmbeddedImage");
                f.findings[i].category = "Xmp".into();
            },
        },
        Plant {
            name: "a Comment region categorised OtherSegment",
            input: "shape-com-segment",
            guard: "category_shape",
            apply: |f| {
                let i = finding_of(f, "FreeText");
                f.findings[i].category = "OtherSegment".into();
            },
        },
        Plant {
            name: "an OtherApp region categorised FreeText",
            input: "shape-app10-arot",
            guard: "category_shape",
            apply: |f| {
                let i = finding_of(f, "OtherSegment");
                f.findings[i].category = "FreeText".into();
            },
        },
        Plant {
            name: "a Trailer region categorised EmbeddedImage",
            input: "shape-trailer-313",
            guard: "category_shape",
            apply: |f| {
                let i = finding_of(f, "OtherSegment");
                f.findings[i].category = "EmbeddedImage".into();
            },
        },
        Plant {
            name: "OtherSegment on a tag item",
            input: "tiff-valid-le",
            guard: "category_shape",
            apply: |f| {
                let i = first_tag_finding(f);
                f.findings[i].category = "OtherSegment".into();
            },
        },
        // inventory / health: the new regions are items, and issues make it unclean.
        Plant {
            name: "a trailer item lost",
            input: "shape-trailer-313",
            guard: "inventory",
            apply: |f| {
                f.items.pop();
            },
        },
        Plant {
            name: "an invented comment item",
            input: "shape-com-segment",
            guard: "inventory",
            apply: |f| {
                let mut it = f.items[0].clone();
                if let What::Region { kind, .. } = &mut it.what {
                    *kind = "Comment".into();
                }
                f.items.push(it);
            },
        },
        Plant {
            name: "a malformed-MPF file reported clean",
            input: "shape-mpo-truncated-index",
            guard: "health",
            apply: |f| f.health.clean = true,
        },
        Plant {
            name: "an MPF image outside the input forgotten in the issue count",
            input: "shape-mpo-offset-out-of-range",
            guard: "health",
            apply: |f| f.health.jpeg_issues = 0,
        },
    ]
}

fn plants() -> Vec<Plant> {
    let mut all = new_region_plants();
    all.extend(plants_before_spec_019());
    all
}

fn plants_before_spec_019() -> Vec<Plant> {
    vec![
        // kind: both directions (a supported container called another, an
        // unsupported one called a supported one).
        Plant {
            name: "a TIFF called a JPEG",
            input: "tiff-valid-le",
            guard: "kind",
            apply: |f| f.kind = "Jpeg".into(),
        },
        Plant {
            name: "a PNG called a TIFF",
            input: "audit-png-magic",
            guard: "kind",
            apply: |f| f.kind = "Tiff".into(),
        },
        // unsupported => nothing, never clean.
        Plant {
            name: "an unsupported file reported clean",
            input: "audit-png-magic",
            guard: "unsupported",
            apply: |f| f.health.clean = true,
        },
        Plant {
            name: "an unsupported file with an item",
            input: "audit-png-magic",
            guard: "unsupported",
            apply: |f| f.items.push(dummy_item()),
        },
        Plant {
            name: "an unsupported file with a severity",
            input: "audit-bigtiff-magic",
            guard: "unsupported",
            apply: |f| f.max = Some("Low".into()),
        },
        Plant {
            name: "an unsupported file with a finding",
            input: "audit-bigtiff-magic",
            guard: "unsupported",
            apply: |f| {
                f.findings.push(FindingF {
                    category: "Gps".into(),
                    item: None,
                    at: dummy_item(),
                    thumb: None,
                    embedded: None,
                })
            },
        },
        // caps.
        Plant {
            name: "items over the cap",
            input: "audit-exif-gps-and-thumbnail",
            guard: "caps",
            apply: |f| {
                let it = f.items[0].clone();
                f.items = vec![it; CAP + 1];
            },
        },
        Plant {
            name: "findings over the cap",
            input: "audit-exif-gps-and-thumbnail",
            guard: "caps",
            apply: |f| {
                let x = f.findings[0].clone();
                f.findings = vec![x; CAP + 1];
            },
        },
        Plant {
            name: "an unlisted finding with room in the inventory",
            input: "audit-exif-gps-and-thumbnail",
            guard: "caps",
            apply: |f| f.findings[0].item = None,
        },
        // indices.
        Plant {
            name: "a finding index out of range",
            input: "audit-exif-gps-and-thumbnail",
            guard: "indices",
            apply: |f| {
                let n = f.items.len();
                f.findings[0].item = Some(n + 5);
            },
        },
        Plant {
            name: "a finding pointing at another item",
            input: "audit-exif-gps-and-thumbnail",
            guard: "indices",
            apply: |f| {
                let i = f.findings[0].item.expect("listed");
                f.findings[0].item = Some(i + 1);
            },
        },
        Plant {
            name: "an unlisted finding without truncation",
            input: "audit-exif-gps-and-thumbnail",
            guard: "indices",
            apply: |f| f.findings[0].item = None,
        },
        Plant {
            name: "findings out of item order",
            input: "audit-exif-gps-and-thumbnail",
            guard: "indices",
            apply: |f| f.findings.swap(0, 1),
        },
        // category shape, both directions.
        Plant {
            name: "Jumbf on a tag item",
            input: "audit-exif-gps-and-thumbnail",
            guard: "category_shape",
            apply: |f| {
                let i = first_tag_finding(f);
                f.findings[i].category = "Jumbf".into();
            },
        },
        Plant {
            name: "a tag category on a region item",
            input: "jpeg-exif-and-xmp",
            guard: "category_shape",
            apply: |f| {
                let i = first_region_finding(f);
                f.findings[i].category = "Software".into();
            },
        },
        Plant {
            name: "an XMP region called another region category",
            input: "jpeg-exif-and-xmp",
            guard: "category_shape",
            apply: |f| {
                let i = first_region_finding(f);
                f.findings[i].category = "Icc".into();
            },
        },
        Plant {
            name: "Gps on a non-GPS tag",
            input: "audit-exif-gps-and-thumbnail",
            guard: "category_shape",
            apply: |f| {
                let i = first_tag_finding(f);
                f.findings[i].category = "Gps".into();
            },
        },
        Plant {
            name: "a GPS-IFD tag not called Gps",
            input: "audit-exif-gps-and-thumbnail",
            guard: "category_shape",
            apply: |f| {
                let i = first_gps_finding(f);
                f.findings[i].category = "Software".into();
            },
        },
        Plant {
            name: "a thumbnail range on another category",
            input: "audit-exif-gps-and-thumbnail",
            guard: "category_shape",
            apply: |f| {
                let i = first_tag_finding(f);
                f.findings[i].thumb = Some(ThumbF {
                    range: None,
                    ffd8: false,
                });
            },
        },
        Plant {
            name: "a Thumbnail finding without a range",
            input: "audit-exif-gps-and-thumbnail",
            guard: "category_shape",
            apply: |f| {
                let i = thumb_finding(f);
                f.findings[i].thumb = None;
            },
        },
        Plant {
            name: "a region item with no finding",
            input: "jpeg-exif-and-xmp",
            guard: "category_shape",
            apply: |f| {
                let i = first_region_finding(f);
                f.findings.remove(i);
            },
        },
        // inventory and findings equal the re-derivation, both directions.
        Plant {
            name: "a dropped item",
            input: "audit-exif-gps-and-thumbnail",
            guard: "inventory",
            apply: |f| {
                f.items.remove(0);
            },
        },
        Plant {
            name: "an extra item",
            input: "audit-exif-gps-and-thumbnail",
            guard: "inventory",
            apply: |f| f.items.push(dummy_item()),
        },
        Plant {
            name: "an after-scan flag flipped",
            input: "jpeg-exif-after-scan",
            guard: "inventory",
            apply: |f| f.items[0].after_scan = !f.items[0].after_scan,
        },
        Plant {
            name: "a count changed",
            input: "tiff-valid-le",
            guard: "inventory",
            apply: |f| {
                if let What::Tag { count, .. } = &mut f.items[0].what {
                    *count += 1;
                }
            },
        },
        Plant {
            name: "the second block's finding deduplicated away",
            input: "audit-two-blocks-same-tag",
            guard: "findings",
            apply: |f| {
                f.findings.pop();
            },
        },
        Plant {
            name: "an extra finding",
            input: "tiff-valid-le",
            guard: "findings",
            apply: |f| {
                f.findings.push(FindingF {
                    category: "Software".into(),
                    item: Some(0),
                    at: f.items[0].clone(),
                    thumb: None,
                    embedded: None,
                })
            },
        },
        Plant {
            name: "a finding in the wrong category",
            input: "tiff-valid-le",
            guard: "findings",
            apply: |f| f.findings[0].category = "DeviceId".into(),
        },
        // gps iff.
        Plant {
            name: "every Gps finding lost",
            input: "audit-exif-gps-and-thumbnail",
            guard: "gps_iff",
            apply: |f| f.findings.retain(|x| x.category != "Gps"),
        },
        Plant {
            name: "a Gps finding with no GPS tag",
            input: "tiff-valid-le",
            guard: "gps_iff",
            apply: |f| {
                f.findings.push(FindingF {
                    category: "Gps".into(),
                    item: Some(0),
                    at: f.items[0].clone(),
                    thumb: None,
                    embedded: None,
                })
            },
        },
        // thumbnail range.
        Plant {
            name: "a thumbnail range outside the blob",
            input: "audit-exif-gps-and-thumbnail",
            guard: "thumbnail",
            apply: |f| {
                let i = thumb_finding(f);
                f.findings[i].thumb = Some(ThumbF {
                    range: Some((u32::MAX - 1, 8)),
                    ffd8: false,
                });
            },
        },
        Plant {
            name: "an FF D8 flag true over bytes that do not start FF D8",
            input: "audit-exif-gps-and-thumbnail",
            guard: "thumbnail",
            apply: |f| {
                let i = thumb_finding(f);
                let t = f.findings[i].thumb.as_mut().expect("range");
                t.range = Some((0, 4));
                t.ffd8 = true;
            },
        },
        Plant {
            name: "an FF D8 flag false over a JPEG start",
            input: "audit-exif-gps-and-thumbnail",
            guard: "thumbnail",
            apply: |f| {
                let i = thumb_finding(f);
                f.findings[i].thumb.as_mut().expect("range").ffd8 = false;
            },
        },
        Plant {
            name: "an FF D8 flag without a range",
            input: "audit-exif-gps-and-thumbnail",
            guard: "thumbnail",
            apply: |f| {
                let i = thumb_finding(f);
                f.findings[i].thumb = Some(ThumbF {
                    range: None,
                    ffd8: true,
                });
            },
        },
        // health, both directions of `clean`.
        Plant {
            name: "clean constant-false on a clean file",
            input: "audit-exif-gps-and-thumbnail",
            guard: "health",
            apply: |f| f.health.clean = false,
        },
        Plant {
            name: "clean constant-true on an unreadable block",
            input: "jpeg-exif-bad-tiff",
            guard: "health",
            apply: |f| f.health.clean = true,
        },
        Plant {
            name: "a failed block not counted",
            input: "jpeg-exif-bad-tiff",
            guard: "health",
            apply: |f| f.health.failed_blocks = 0,
        },
        Plant {
            name: "truncation not flagged",
            input: GENERATED_BIG_INVENTORY,
            guard: "health",
            apply: |f| f.health.truncated = false,
        },
        Plant {
            name: "an issue not counted",
            input: "jpeg-exif-after-scan",
            guard: "health",
            apply: |f| f.health.jpeg_issues += 1,
        },
        // max severity.
        Plant {
            name: "no severity although findings exist",
            input: "audit-exif-gps-and-thumbnail",
            guard: "max_severity",
            apply: |f| f.max = None,
        },
        Plant {
            name: "a severity below a listed finding",
            input: "audit-exif-gps-and-thumbnail",
            guard: "max_severity",
            apply: |f| f.max = Some("Low".into()),
        },
        Plant {
            name: "a severity above every finding",
            input: "tiff-valid-le",
            guard: "max_severity",
            apply: |f| f.max = Some("High".into()),
        },
        // The IFD1-strips rule, both directions.
        Plant {
            name: "an IFD1 strips entry with no thumbnail finding",
            input: "audit-ifd1-strips",
            guard: "findings",
            apply: |f| f.findings.retain(|x| x.category != "Thumbnail"),
        },
        Plant {
            name: "a thumbnail finding on the wrong IFD1 entry",
            input: "audit-ifd1-strips",
            guard: "findings",
            apply: |f| {
                let i = thumb_finding(f);
                let other = f
                    .items
                    .iter()
                    .position(|x| {
                        *x != f.findings[i].at
                            && matches!(&x.what, What::Tag { ifd, .. } if ifd == "Ifd1")
                    })
                    .expect("another IFD1 entry");
                f.findings[i].at = f.items[other].clone();
                f.findings[i].item = Some(other);
            },
        },
    ]
}

#[test]
fn audit_harness_each_iff_invariant_goes_red_when_planted() {
    use exif_core::audit::audit;
    let plants = plants();
    // Every guard has at least one plant (so none is untested), and the
    // two-directional ones have more than one.
    for (name, _) in audit_invariants::GUARDS {
        assert!(
            plants.iter().any(|p| p.guard == name),
            "no plant for guard {name}"
        );
    }
    for p in &plants {
        let data = plant_input(p.input);
        let got = audit_invariants::facts(&audit(&data));
        let want = audit_invariants::rederive(&data);
        // CONTROL: the unplanted answer satisfies every guard.
        let control = audit_invariants::violations(&data, &got, &want);
        assert!(
            control.is_empty(),
            "{}: control on {}: {control:?}",
            p.name,
            p.input
        );
        // MUTATION: the planted answer is rejected by the named guard ALONE
        // (the other guards do not run), so an overlap between guards cannot
        // hide an untested one.
        let mut bad = got.clone();
        (p.apply)(&mut bad);
        assert_ne!(bad, got, "{}: the plant changed nothing", p.name);
        let alone = audit_invariants::run_guard(p.guard, &data, &bad, &want);
        assert!(
            alone.is_err(),
            "{}: guard {} did not reject the planted bug on {}",
            p.name,
            p.guard,
            p.input
        );
    }
    println!(
        "{} plants, each rejected by its own guard alone",
        plants.len()
    );
}

// ── SPEC-017: the total entry budget ─────────────────────────────────────────

include!("../fuzz/shared/entry_budget_invariants.rs");

fn tiff_seed(name: &str) -> Vec<u8> {
    let path = root().join("fuzz/seeds/tiff_ifd").join(name);
    fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn new_tiff_seeds_have_the_shapes_they_claim() {
    use exif_core::tiff::entry_budget;

    // Exactly on the budget: clean.
    let exact = tiff_seed("entry-budget-exact");
    assert_eq!(exact.len(), 66);
    let c = Container::parse(&exact).expect("parses");
    assert!(c.is_clean(), "{:?}", c.issues());
    assert_eq!(c.entry_count(), 5);
    assert_eq!(c.entry_count(), entry_budget(exact.len()));

    // One over: the second sub-IFD is omitted whole, one issue.
    let over = tiff_seed("entry-budget-over");
    assert_eq!(over.len(), 78);
    let c = Container::parse(&over).expect("parses");
    assert_eq!(entry_budget(over.len()), 6);
    assert_eq!(c.entry_count(), 4);
    assert_eq!(c.ifds().len(), 2);
    assert_eq!(c.issues().len(), 1);
    assert_eq!(c.issues()[0].error(), &Error::TooManyEntries { limit: 6 });

    // The reduced hostile shape: 40 sub-IFDs of 255 entries in 3.3 KB.
    let many = tiff_seed("overlapping-subifds");
    assert!(many.len() < 4 * 1024, "{} bytes", many.len());
    let c = Container::parse(&many).expect("parses");
    let budget = entry_budget(many.len());
    assert_eq!(c.entry_count(), 1 + 255);
    assert!(c.entry_count() <= budget);
    assert!(
        c.entry_count() + 255 > budget,
        "the second sub-IFD must cross"
    );
    assert_eq!(c.ifds().len(), 2);
    assert_eq!(c.issues().len(), 1);
    assert_eq!(
        c.issues()[0].error(),
        &Error::TooManyEntries { limit: budget }
    );
    // Without the budget this file would have kept IFD0 and 40 sub-IFDs.
    let ifd0 = c.ifd0().expect("ifd0");
    assert_eq!(ifd0.entry(330).expect("SubIFDs").count(), 40);
}

#[test]
fn tiff_harness_entry_budget_invariants_hold_on_every_seed() {
    let mut seen = 0usize;
    for target in ["tiff_ifd", "audit"] {
        let dir = root().join("fuzz/seeds").join(target);
        for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
            let path = entry.expect("entry").path();
            let data = fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            entry_budget_invariants::check_input(&data);
            if let Ok(c) = Container::parse(&data) {
                entry_budget_invariants::check_container(&data, &c);
            }
            seen += 1;
        }
    }
    assert_eq!(seen, 32 + 78 + 17);
    // The degenerate inputs too, and the generated big inventory.
    for data in audit_degenerate_inputs() {
        entry_budget_invariants::check_input(&data);
    }
    entry_budget_invariants::check_input(&big_inventory());
}

/// One planted bug, applied to the flattened answer of a seed; the named
/// guard, run ALONE, must reject it.
struct BudgetPlant {
    name: &'static str,
    seed: &'static str,
    guard: &'static str,
    apply: fn(&mut entry_budget_invariants::Shape),
}

fn budget_plants() -> Vec<BudgetPlant> {
    use entry_budget_invariants::Halt;
    vec![
        // The check removed: more entries than the budget are retained.
        BudgetPlant {
            name: "check removed (one entry over the budget retained)",
            seed: "entry-budget-exact",
            guard: "budget",
            apply: |s| {
                s.entry_count = s.budget + 1;
                s.reported_count = s.entry_count;
            },
        },
        BudgetPlant {
            name: "entry_count() disagrees with the IFDs",
            seed: "entry-budget-exact",
            guard: "budget",
            apply: |s| s.reported_count += 1,
        },
        // `>` against `>=`: a halt where the total only EQUALS the budget.
        BudgetPlant {
            name: "halts on an exact fit (>= instead of >)",
            seed: "entry-budget-over",
            guard: "crossing",
            apply: |s| s.declared_at_halt = Some(s.budget - s.entry_count),
        },
        // The refused IFD was recorded anyway: the omission is not atomic.
        BudgetPlant {
            name: "the refused IFD was recorded (omission not atomic)",
            seed: "entry-budget-over",
            guard: "crossing",
            apply: |s| {
                let at = s.issues[0].1.expect("offset");
                s.ifd_offsets.push(at);
            },
        },
        BudgetPlant {
            name: "wrong limit reported",
            seed: "entry-budget-over",
            guard: "crossing",
            apply: |s| s.limits[0] += 1,
        },
        BudgetPlant {
            name: "TooManyEntries pushed twice",
            seed: "entry-budget-over",
            guard: "once-last",
            apply: |s| {
                let again = s.issues[0];
                s.issues.push(again);
                s.limits.push(s.budget);
            },
        },
        BudgetPlant {
            name: "halt skipped (an issue recorded after it)",
            seed: "entry-budget-over",
            guard: "once-last",
            apply: |s| s.issues.push((Halt::Other, None)),
        },
        BudgetPlant {
            name: "TooManyIfds recorded with it",
            seed: "entry-budget-over",
            guard: "exclusive",
            apply: |s| s.issues.insert(0, (Halt::Ifds, None)),
        },
    ]
}

#[test]
fn entry_budget_plants_are_each_rejected_by_their_own_guard_alone() {
    let plants = budget_plants();
    for (name, _) in entry_budget_invariants::GUARDS {
        assert!(
            plants.iter().any(|p| p.guard == name),
            "no plant for guard {name}"
        );
    }
    for p in &plants {
        let data = tiff_seed(p.seed);
        let c = Container::parse(&data).expect("parses");
        let good = entry_budget_invariants::shape(&data, &c);
        // CONTROL: the unplanted answer satisfies every guard.
        let control = entry_budget_invariants::violations(&good);
        assert!(
            control.is_empty(),
            "{}: control on {}: {control:?}",
            p.name,
            p.seed
        );
        let mut bad = good.clone();
        (p.apply)(&mut bad);
        assert_ne!(bad, good, "{}: the plant changed nothing", p.name);
        // MUTATION: rejected by the named guard ALONE.
        assert!(
            entry_budget_invariants::run_guard(p.guard, &bad).is_err(),
            "{}: guard {} did not reject the planted bug on {}",
            p.name,
            p.guard,
            p.seed
        );
    }
    println!(
        "{} entry-budget plants, each rejected by its own guard alone",
        plants.len()
    );
}

// ── SPEC-013 F5: the audit harness's guards are linear in the input ─────────

/// A JPEG segment: `FF marker length payload` (the length counts its own two
/// bytes).
fn lin_seg(marker: u8, payload: &[u8]) -> Vec<u8> {
    let len = u16::try_from(payload.len() + 2).expect("segment fits");
    let mut v = vec![0xFF, marker];
    v.extend_from_slice(&len.to_be_bytes());
    v.extend_from_slice(payload);
    v
}

fn lin_jpeg(segments: &[Vec<u8>]) -> Vec<u8> {
    let mut d = vec![0xFF, 0xD8];
    for s in segments {
        d.extend_from_slice(s);
    }
    d.extend_from_slice(&lin_seg(0xDA, &[1, 1, 0, 0, 0x3F, 0]));
    d.extend_from_slice(&[0x12, 0x34, 0xFF, 0xD9]);
    d
}

/// The TIFF of one Exif block whose IFD1 holds `k` thumbnail-pointer entries:
/// the first `0x0201`/`0x0202` pair gives every `0x0201` entry a valid range,
/// so every one of the `k - 1` thumbnail findings has a range to re-read.
fn thumbnail_tiff(k: usize) -> Vec<u8> {
    let entry = |t: &mut Vec<u8>, tag: u16, v: u32| {
        t.extend_from_slice(&tag.to_le_bytes());
        t.extend_from_slice(&4u16.to_le_bytes());
        t.extend_from_slice(&1u32.to_le_bytes());
        t.extend_from_slice(&v.to_le_bytes());
    };
    let mut t = b"II\x2A\x00\x08\x00\x00\x00".to_vec();
    t.extend_from_slice(&0u16.to_le_bytes()); // IFD0: no entries
    t.extend_from_slice(&14u32.to_le_bytes()); // next: IFD1, right here
    t.extend_from_slice(&u16::try_from(k).expect("k").to_le_bytes());
    entry(&mut t, 0x0201, 0);
    entry(&mut t, 0x0202, 4);
    for _ in 2..k {
        entry(&mut t, 0x0201, 0);
    }
    t.extend_from_slice(&0u32.to_le_bytes());
    t
}

/// One Exif APP1 whose IFD1 holds `k` thumbnail-pointer entries.
fn many_thumbnails(k: usize) -> Vec<u8> {
    lin_jpeg(&[lin_seg(
        0xE1,
        &[&b"Exif\0\0"[..], &thumbnail_tiff(k)].concat(),
    )])
}

/// `n` JUMBF APP11 segments: `n` region items, each with one finding.
fn many_regions(n: usize) -> Vec<u8> {
    let one = lin_seg(
        0xEB,
        &[0x4A, 0x50, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x6A],
    );
    lin_jpeg(&vec![one; n])
}

/// Many findings AND many regions at once: `blocks` Exif blocks of 5,000
/// thumbnail entries (each under the 64 KiB segment limit; 4,999 findings
/// each) and `regions` JUMBF segments. A guard that rescans the findings once
/// per region item is `O(regions x findings)` here.
fn heavy_findings_and_regions(blocks: usize, regions: usize) -> Vec<u8> {
    let exif = lin_seg(0xE1, &[&b"Exif\0\0"[..], &thumbnail_tiff(5_000)].concat());
    let jumbf = lin_seg(
        0xEB,
        &[0x4A, 0x50, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x6A],
    );
    let mut segments = vec![exif; blocks];
    segments.extend(std::iter::repeat_n(jumbf, regions));
    lin_jpeg(&segments)
}

/// The rounds each measurement is repeated in; the minimum over them is kept.
const GUARD_TIME_ROUNDS: usize = 7;

/// Wall time of each audit guard on the small and the large input, measured in
/// interleaved rounds (small, large, small, large, ...) so a burst of load hits
/// both sides alike; the minimum over the rounds is kept for each. Returns
/// `(guard, small, large)`.
fn guard_times_interleaved(
    small: &[u8],
    large: &[u8],
) -> Vec<(&'static str, std::time::Duration, std::time::Duration)> {
    let prep = |d: &[u8]| {
        let a = exif_core::audit::audit(d);
        (audit_invariants::facts(&a), audit_invariants::rederive(d))
    };
    let (sg, sw) = prep(small);
    let (lg, lw) = prep(large);
    assert!(
        lg.findings.len() > 59_000 && sg.findings.len() > 14_000,
        "{} {}",
        lg.findings.len(),
        sg.findings.len()
    );
    let mut out = Vec::new();
    for (name, _) in audit_invariants::GUARDS {
        let mut best_s = std::time::Duration::MAX;
        let mut best_l = std::time::Duration::MAX;
        for _ in 0..GUARD_TIME_ROUNDS {
            let t = std::time::Instant::now();
            let r = audit_invariants::run_guard(name, small, &sg, &sw);
            best_s = best_s.min(t.elapsed());
            assert!(r.is_ok(), "{name}: {r:?}");
            let t = std::time::Instant::now();
            let r = audit_invariants::run_guard(name, large, &lg, &lw);
            best_l = best_l.min(t.elapsed());
            assert!(r.is_ok(), "{name}: {r:?}");
        }
        out.push((name, best_s, best_l));
    }
    out
}

/// A guard that is linear takes about 4 times as long on 4 times the input; a
/// quadratic one about 16 times. The bound sits between, and compares two runs
/// on the same host in the same process, so it does not depend on host speed.
const GUARD_TIME_RATIO_BOUND: u32 = 8;
/// Below this the small run is timer noise and the guard is not in question.
const GUARD_TIME_FLOOR: std::time::Duration = std::time::Duration::from_millis(5);
/// A quadratic guard exceeds the bound on every attempt; load noise has to hit
/// all of them to fail a linear one.
const GUARD_TIME_ATTEMPTS: usize = 3;

/// The work the guards do on `data` (counted, not timed), with the facts and
/// the re-derivation computed before the counter is reset.
fn guard_steps(data: &[u8]) -> (u64, usize, usize) {
    let a = exif_core::audit::audit(data);
    let got = audit_invariants::facts(&a);
    let want = audit_invariants::rederive(data);
    audit_invariants::reset_steps();
    let bad = audit_invariants::violations(data, &got, &want);
    assert!(
        bad.is_empty(),
        "guards fail on the linearity input: {bad:?}"
    );
    (
        audit_invariants::steps(),
        got.items.len(),
        got.findings.len(),
    )
}

/// F5 (HANDOFF-024): `g_category_shape`'s region loop and `g_thumbnail`'s
/// per-finding re-read were O(items x findings) and O(findings x input) on a
/// hostile JPEG. The guards now count their work; it must stay within a fixed
/// multiple of (items + findings + input bytes), however large the input, and
/// double when the input doubles.
#[test]
fn audit_harness_guards_are_linear_in_the_input() {
    let big_thumbs = many_thumbnails(5_000);
    let small_thumbs = many_thumbnails(2_500);
    let big_regions = many_regions(3_000);
    let small_regions = many_regions(1_500);
    // The shapes are what they claim.
    let (_, items, findings) = guard_steps(&big_thumbs);
    assert!(items >= 5_000 && findings >= 4_990, "{items} {findings}");
    let (_, items, findings) = guard_steps(&big_regions);
    assert!(items >= 3_000 && findings >= 3_000, "{items} {findings}");

    for (name, big, small) in [
        ("thumbnails", &big_thumbs, &small_thumbs),
        ("regions", &big_regions, &small_regions),
    ] {
        let (steps, items, findings) = guard_steps(big);
        let bound = 8 * (items + findings) as u64 + 2 * big.len() as u64;
        assert!(
            steps <= bound,
            "{name}: {steps} steps for {items} items, {findings} findings and {} bytes \
             (bound {bound}): a guard is not linear",
            big.len()
        );
        // Doubling the input at most doubles (with slack for rounding) the work.
        let (half, _, _) = guard_steps(small);
        println!("{name}: {half} steps at half size, {steps} at full ({bound} allowed)");
        assert!(
            steps <= 3 * half,
            "{name}: {half} steps at half the size, {steps} at full: not linear"
        );
    }

    // The counter above only sees the places a `tick()` sits. A guard can be
    // linear in the counter and quadratic in time (the SPEC-013 verify's `f5`
    // mutant restores a per-region rescan of the findings with no tick), so
    // each guard is also timed on an input with many findings and many regions
    // and on one a quarter of its size, in the same run: time(N) must stay
    // under 8 x time(N/4).
    let big = heavy_findings_and_regions(12, 4_000);
    let small = heavy_findings_and_regions(3, 1_000);
    let mut last_failures = Vec::new();
    for attempt in 1..=GUARD_TIME_ATTEMPTS {
        let times = guard_times_interleaved(&small, &big);
        let mut failures = Vec::new();
        let mut worst = 0.0f64;
        for (name, small_t, big_t) in &times {
            let ratio = big_t.as_secs_f64() / small_t.as_secs_f64().max(1e-9);
            let reference = (*small_t).max(GUARD_TIME_FLOOR);
            let effective = big_t.as_secs_f64() / reference.as_secs_f64();
            worst = worst.max(effective);
            println!(
                "attempt {attempt} guard {name}: {small_t:?} at N/4, {big_t:?} at N, ratio {ratio:.1} (against the floor {effective:.1})"
            );
            if *big_t >= reference * GUARD_TIME_RATIO_BOUND {
                failures.push(format!(
                    "guard {name} took {big_t:?} at N and {small_t:?} at N/4 (ratio {ratio:.1}, bound {GUARD_TIME_RATIO_BOUND})"
                ));
            }
        }
        println!("attempt {attempt}: worst effective ratio {worst:.1}");
        if failures.is_empty() {
            return;
        }
        last_failures = failures;
    }
    panic!("superlinear in time on all {GUARD_TIME_ATTEMPTS} attempts: {last_failures:?}");
}

// ── SPEC-014: the strip-policy preview guards ────────────────────────────────

use audit_invariants::PreviewInput;

fn preview_input_of(data: &[u8]) -> PreviewInput {
    let a = exif_core::audit::audit(data);
    let f = audit_invariants::facts(&a);
    audit_invariants::preview_input(data, &a, &f)
}

/// One little-endian IFD: `(tag, type, count, value-field)` entries (inline
/// values only), then the next-IFD offset.
fn le_ifd(entries: &[(u16, u16, u32, u32)], next: u32) -> Vec<u8> {
    let mut v = u16::try_from(entries.len())
        .expect("few")
        .to_le_bytes()
        .to_vec();
    for (tag, typ, count, val) in entries {
        v.extend_from_slice(&tag.to_le_bytes());
        v.extend_from_slice(&typ.to_le_bytes());
        v.extend_from_slice(&count.to_le_bytes());
        v.extend_from_slice(&val.to_le_bytes());
    }
    v.extend_from_slice(&next.to_le_bytes());
    v
}

/// A TIFF with a tag of each of the seven tag categories (IFD0, ExifIFD, GPS,
/// IFD1), a MakerNote and an IFD1 thumbnail; every value is inline.
fn full_house_tiff_bytes() -> Vec<u8> {
    const A: u32 = 0x0000_0061; // "a\0" as an inline ASCII value
    let ifd0_len = 2 + 12 * 8 + 4;
    let exif_len = 2 + 12 * 3 + 4;
    let gps_len = 2 + 12 + 4;
    let exif_at = u32::try_from(8 + ifd0_len).expect("small");
    let gps_at = exif_at + u32::try_from(exif_len).expect("small");
    let ifd1_at = gps_at + u32::try_from(gps_len).expect("small");
    let mut t = b"II\x2A\x00\x08\x00\x00\x00".to_vec();
    t.extend(le_ifd(
        &[
            (0x010E, 2, 2, A), // FreeText
            (0x010F, 2, 2, A), // CameraModel
            (0x0131, 2, 2, A), // Software
            (0x0132, 2, 2, A), // Timestamps
            (0x013B, 2, 2, A), // Authorship
            (0x8769, 4, 1, exif_at),
            (0x8825, 4, 1, gps_at),
            (0xA431, 2, 2, A), // DeviceId
        ],
        ifd1_at,
    ));
    t.extend(le_ifd(
        &[(0x9003, 2, 2, A), (0xA431, 2, 2, A), (0x927C, 7, 4, A)],
        0,
    ));
    t.extend(le_ifd(&[(0x0001, 2, 2, A)], 0));
    t.extend(le_ifd(&[(0x0201, 4, 1, 0), (0x0202, 4, 1, 0)], 0));
    t
}

/// The full-house JPEG: that TIFF in an Exif APP1, plus XMP, IPTC, ICC, JUMBF.
fn full_house_jpeg_bytes() -> Vec<u8> {
    let iptc_data = [0x1C, 0x02, 0x05, 0x00, 0x01, b'T'];
    let mut res = b"8BIM".to_vec();
    res.extend_from_slice(&0x0404u16.to_be_bytes());
    res.extend_from_slice(&[0, 0, 0, 0, 0, 6]);
    res.extend_from_slice(&iptc_data);
    lin_jpeg(&[
        lin_seg(0xE1, &[&b"Exif\0\0"[..], &full_house_tiff_bytes()].concat()),
        lin_seg(
            0xE1,
            &[&b"http://ns.adobe.com/xap/1.0/\0"[..], &b"<x/>"[..]].concat(),
        ),
        lin_seg(0xED, &[&b"Photoshop 3.0\0"[..], &res].concat()),
        lin_seg(0xE2, b"ICC_PROFILE\0\x01\x01profile"),
        lin_seg(
            0xEB,
            &[0x4A, 0x50, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x6A],
        ),
    ])
}

#[test]
fn the_full_house_inputs_carry_every_category() {
    use exif_core::audit::{audit, Category};
    let tiff = full_house_tiff_bytes();
    let a = audit(&tiff);
    let have: Vec<Category> = a.findings().iter().map(|f| f.category).collect();
    for c in [
        Category::Gps,
        Category::DeviceId,
        Category::Authorship,
        Category::FreeText,
        Category::Timestamps,
        Category::CameraModel,
        Category::Software,
        Category::MakerNote,
        Category::Thumbnail,
    ] {
        assert!(have.contains(&c), "tiff lacks {c:?}");
    }
    assert!(a.health().clean);
    let jpeg = full_house_jpeg_bytes();
    let a = audit(&jpeg);
    let have: Vec<Category> = a.findings().iter().map(|f| f.category).collect();
    // The 13 DEC-008 categories; the two DEC-012 categories are carried by the
    // shape seeds (`shape_seeds_carry_the_two_dec_012_categories`).
    for c in Category::ALL {
        if matches!(c, Category::EmbeddedImage | Category::OtherSegment) {
            continue;
        }
        assert!(have.contains(&c), "jpeg lacks {c:?}");
    }
    assert!(a.health().clean);
    audit_invariants::check(&tiff);
    audit_invariants::check(&jpeg);
}

#[test]
fn preview_guards_hold_on_every_seed_and_degenerate_inputs() {
    let mut inputs: Vec<(String, Vec<u8>)> = Vec::new();
    for target in ["tiff_ifd", "jpeg_segments", "audit"] {
        let dir = root().join("fuzz/seeds").join(target);
        for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
            let path = entry.expect("entry").path();
            let data = fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            inputs.push((path.display().to_string(), data));
        }
    }
    assert_eq!(inputs.len(), 32 + 37 + 78 + 17 + 17);
    for (i, d) in audit_degenerate_inputs().into_iter().enumerate() {
        inputs.push((format!("degenerate {i}"), d));
    }
    // A truncated audit (the inventory cap is overflowed) exercises rule 4's
    // conservative branch.
    inputs.push((GENERATED_BIG_INVENTORY.to_string(), big_inventory()));
    // The seeds hold no DeviceId, MakerNote or Jumbf-next-to-Exif shape; the
    // full-house inputs put every category in front of the guards.
    inputs.push(("full-house tiff".to_string(), full_house_tiff_bytes()));
    inputs.push(("full-house jpeg".to_string(), full_house_jpeg_bytes()));
    let mut with_findings = 0usize;
    let mut truncated = 0usize;
    for (name, data) in &inputs {
        let pin = preview_input_of(data);
        let bad = audit_invariants::preview_violations(&pin);
        assert!(bad.is_empty(), "{name}: {bad:?}");
        assert_eq!(pin.cases.len(), 6, "five presets and one custom");
        if !pin.findings.is_empty() {
            with_findings += 1;
        }
        if pin.truncated {
            truncated += 1;
        }
    }
    assert!(with_findings > 20, "inputs with findings: {with_findings}");
    assert_eq!(truncated, 1, "exactly the generated big inventory");
}

/// One planted bug in the flattened preview of `input`, which the named guard,
/// run ALONE, must reject.
struct PreviewPlant {
    name: &'static str,
    input: &'static str,
    guard: &'static str,
    apply: fn(&mut PreviewInput),
}

/// The index of the first disposition equal to `what` in case `case`.
fn disp_at(p: &PreviewInput, case: usize, what: &str) -> usize {
    p.cases[case]
        .got
        .dispositions
        .iter()
        .position(|d| d == what)
        .unwrap_or_else(|| panic!("no {what} disposition in case {case}"))
}

fn preview_plants() -> Vec<PreviewPlant> {
    const RICH: &str = "audit-exif-gps-and-thumbnail";
    vec![
        // preview_shape
        PreviewPlant {
            name: "a disposition is missing",
            input: RICH,
            guard: "preview_shape",
            apply: |p| {
                p.cases[0].got.dispositions.pop();
            },
        },
        PreviewPlant {
            name: "a count disagrees with the tally of the dispositions",
            input: RICH,
            guard: "preview_shape",
            apply: |p| p.cases[0].got.kept += 1,
        },
        PreviewPlant {
            name: "residual is not kept plus kept-unread",
            input: RICH,
            guard: "preview_shape",
            apply: |p| p.cases[0].got.residual += 1,
        },
        PreviewPlant {
            name: "a kept disposition becomes removed with the counts untouched",
            input: RICH,
            guard: "preview_shape",
            apply: |p| {
                let i = disp_at(p, 0, "kept");
                p.cases[0].got.dispositions[i] = "removed".into();
            },
        },
        // preview_dispositions: removed and kept_unread, each direction.
        PreviewPlant {
            name: "an expected removed is missing",
            input: RICH,
            guard: "preview_dispositions",
            apply: |p| {
                let i = disp_at(p, 0, "removed");
                p.cases[0].got.dispositions[i] = "kept".into();
            },
        },
        PreviewPlant {
            name: "an unexpected removed appears",
            input: RICH,
            guard: "preview_dispositions",
            apply: |p| {
                let i = disp_at(p, 0, "kept");
                p.cases[0].got.dispositions[i] = "removed".into();
            },
        },
        PreviewPlant {
            name: "an expected kept_unread is missing",
            input: RICH,
            guard: "preview_dispositions",
            apply: |p| {
                let i = disp_at(p, 0, "kept_unread");
                p.cases[0].got.dispositions[i] = "kept".into();
            },
        },
        PreviewPlant {
            name: "an unexpected kept_unread appears",
            input: RICH,
            guard: "preview_dispositions",
            apply: |p| {
                let i = disp_at(p, 0, "kept");
                p.cases[0].got.dispositions[i] = "kept_unread".into();
            },
        },
        PreviewPlant {
            name: "dispositions in reverse order",
            input: RICH,
            guard: "preview_dispositions",
            apply: |p| p.cases[0].got.dispositions.reverse(),
        },
        // preview_nesting
        PreviewPlant {
            name: "removed shrinks along the presets",
            input: RICH,
            guard: "preview_nesting",
            apply: |p| p.cases[0].got.removed += 100,
        },
        PreviewPlant {
            name: "residual grows along the presets",
            input: RICH,
            guard: "preview_nesting",
            apply: |p| p.cases[1].got.residual += 100,
        },
        PreviewPlant {
            name: "everything leaves a residual finding",
            input: RICH,
            guard: "preview_nesting",
            apply: |p| p.cases[4].got.residual = 1,
        },
        PreviewPlant {
            name: "everything leaves a residual severity",
            input: RICH,
            guard: "preview_nesting",
            apply: |p| p.cases[4].got.residual_sev = Some("Low".into()),
        },
        // preview_residual_and_reliable
        PreviewPlant {
            name: "residual severity ignores the kept findings",
            input: RICH,
            guard: "preview_residual_and_reliable",
            apply: |p| {
                assert!(p.cases[0].want.residual_sev.is_some(), "premise");
                p.cases[0].got.residual_sev = None;
            },
        },
        PreviewPlant {
            name: "a residual severity appears where none remains",
            input: RICH,
            guard: "preview_residual_and_reliable",
            apply: |p| {
                assert!(p.cases[4].want.residual_sev.is_none(), "premise");
                p.cases[4].got.residual_sev = Some("High".into());
            },
        },
        PreviewPlant {
            name: "an unsupported file is reported reliable",
            input: "audit-png-magic",
            guard: "preview_residual_and_reliable",
            apply: |p| p.cases[0].got.reliable = true,
        },
        PreviewPlant {
            name: "a clean file is reported unreliable",
            input: "tiff-valid-le",
            guard: "preview_residual_and_reliable",
            apply: |p| p.cases[2].got.reliable = false,
        },
    ]
}

#[test]
fn each_preview_guard_goes_red_alone_when_planted() {
    let plants = preview_plants();
    for (name, _) in audit_invariants::PREVIEW_GUARDS {
        let n = plants.iter().filter(|p| p.guard == name).count();
        // Every preview guard is an iff or a two-sided bound: a plant for each
        // direction.
        assert!(n >= 2, "guard {name} has {n} plant(s), want at least two");
    }
    for p in &plants {
        let data = plant_input(p.input);
        let good = preview_input_of(&data);
        // CONTROL: the unplanted answer satisfies every preview guard.
        let control = audit_invariants::preview_violations(&good);
        assert!(
            control.is_empty(),
            "{}: control on {}: {control:?}",
            p.name,
            p.input
        );
        // MUTATION: rejected by the named guard ALONE.
        let mut bad = good.clone();
        (p.apply)(&mut bad);
        assert_ne!(
            format!("{bad:?}"),
            format!("{good:?}"),
            "{}: the plant changed nothing",
            p.name
        );
        let alone = audit_invariants::run_preview_guard(p.guard, &bad);
        assert!(
            alone.is_err(),
            "{}: guard {} did not reject the planted bug on {}",
            p.name,
            p.guard,
            p.input
        );
    }
    println!(
        "{} preview plants, each rejected by its own guard alone",
        plants.len()
    );
}

/// The work the preview guards do on `data`, counted not timed.
fn preview_guard_steps(data: &[u8]) -> (u64, usize) {
    let pin = preview_input_of(data);
    audit_invariants::reset_steps();
    let bad = audit_invariants::preview_violations(&pin);
    assert!(
        bad.is_empty(),
        "preview guards fail on the linearity input: {bad:?}"
    );
    (audit_invariants::steps(), pin.findings.len())
}

#[test]
fn preview_guards_are_linear() {
    for (name, big, small) in [
        ("thumbnails", many_thumbnails(5_000), many_thumbnails(2_500)),
        ("regions", many_regions(3_000), many_regions(1_500)),
    ] {
        let (steps, findings) = preview_guard_steps(&big);
        assert!(findings >= 2_900, "{name}: {findings} findings");
        // Six cases, three guards that touch every disposition: about 18 steps
        // per finding; a rescan per finding would be quadratic.
        let bound = 24 * findings as u64 + 64;
        assert!(
            steps <= bound,
            "{name}: {steps} steps for {findings} findings (bound {bound}): not linear"
        );
        let (half, _) = preview_guard_steps(&small);
        println!("{name}: {half} steps at half size, {steps} at full ({bound} allowed)");
        assert!(
            steps <= 3 * half,
            "{name}: {half} steps at half the size, {steps} at full: not linear"
        );
    }
}

// ── SPEC-015: the report renderers ───────────────────────────────────────────

include!("../fuzz/shared/render_check.rs");
include!("../fuzz/shared/render_invariants.rs");

use render_invariants::Ctx;

fn render_seed_path(name: &str) -> PathBuf {
    root().join("fuzz/seeds/audit_render").join(name)
}

fn render_seed(name: &str) -> Vec<u8> {
    let path = render_seed_path(name);
    fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn render_ctx(seed: &str) -> Ctx {
    render_invariants::build(&render_seed(seed))
}

const SEMICOLON: &str = "render-label-semicolon-formula";

const AUDIT_RENDER_SEEDS: [&str; 26] = [
    "render-ascii-payload-300-bytes",
    "render-empty-file",
    "render-full-house-jpeg-all-pii-inventory-values",
    "render-full-house-jpeg-custom-policy",
    "render-full-house-jpeg-everything-inventory",
    "render-full-house-jpeg-no-policy",
    "render-full-house-tiff-gps-values",
    "render-label-255-bytes",
    "render-label-control-characters",
    "render-label-cr-lf",
    "render-label-empty-png",
    "render-label-formula-at",
    "render-label-formula-equals",
    "render-label-invalid-utf8",
    "render-label-invisible-characters",
    "render-label-quotes-comma",
    "render-label-right-to-left-override",
    "render-label-semicolon-formula",
    "render-png-signature-values",
    "render-shape-mpo-all-pii-inventory-values",
    "render-shape-mpo-custom-failure",
    "render-shape-mpo-everything-inventory",
    "render-shape-mpo-gps-values",
    "render-shape-split-app13-all-inventory-values",
    "render-shape-split-app13-gps",
    "render-shape-trailer-no-policy-inventory",
];

const HOUSE_ALL_PII: &str = "render-full-house-jpeg-all-pii-inventory-values";
const HOUSE_EVERYTHING: &str = "render-full-house-jpeg-everything-inventory";
const LONG_ASCII: &str = "render-ascii-payload-300-bytes";
const PNG_VALUES: &str = "render-png-signature-values";

#[test]
fn audit_render_target_is_declared_and_runs_in_ci() {
    let fuzz = read("fuzz/Cargo.toml");
    assert!(
        fuzz.contains("name = \"audit_render\"") && fuzz.contains("fuzz_targets/audit_render.rs"),
        "fuzz/Cargo.toml must declare the audit_render bin"
    );
    for f in [
        "fuzz/fuzz_targets/audit_render.rs",
        "fuzz/shared/render_invariants.rs",
        "fuzz/shared/render_check.rs",
    ] {
        assert!(root().join(f).is_file(), "{f}");
    }
    let ci = read(".github/workflows/ci.yml");
    assert!(
        ci.contains("audit_render, cli_args]"),
        "the fuzz-smoke matrix must run the audit_render target"
    );
    let app = read("app.just");
    assert!(
        app.contains("audit audit_render cli_args\"") && app.contains("`audit_render`"),
        "`just fuzz-smoke` must know audit_render"
    );
}

#[test]
fn audit_render_seeds_are_committed_and_have_the_shapes_they_claim() {
    use exif_core::audit::audit;
    let dir = root().join("fuzz/seeds/audit_render");
    let mut names: Vec<String> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, AUDIT_RENDER_SEEDS, "the audit_render seed set");
    assert!(names.len() >= 14);

    let by = |n: &str| render_ctx(&format!("render-{n}"));
    // The two full-house inputs under several option bytes.
    let a = render_ctx(HOUSE_ALL_PII);
    assert_eq!(a.file, full_house_jpeg_bytes());
    assert_eq!(
        (a.policy, a.inventory, a.values),
        (Some(2), true, true),
        "all_pii, inventory, values"
    );
    let b = by("full-house-tiff-gps-values");
    assert_eq!(b.file, full_house_tiff_bytes());
    assert_eq!((b.policy, b.inventory, b.values), (Some(0), false, true));
    let e = render_ctx(HOUSE_EVERYTHING);
    assert_eq!(
        (e.policy, e.inventory, e.values, e.reason),
        (Some(4), true, false, 1)
    );
    let c = by("full-house-jpeg-custom-policy");
    assert_eq!(c.policy, Some(5));
    assert!(
        !c.removed.is_empty() && c.removed.len() < 13,
        "the seed's custom set is non-trivial: {:?}",
        c.removed
    );
    assert_eq!(by("full-house-jpeg-no-policy").policy, None);
    // Hostile labels.
    let label = |n: &str| by(n).label;
    assert!(label("label-control-characters").iter().any(|b| *b < 0x20));
    let q = label("label-quotes-comma");
    assert!(q.contains(&b'"') && q.contains(&b','));
    let crlf = label("label-cr-lf");
    assert!(crlf.windows(2).any(|w| w == b"\r\n"));
    assert!(std::str::from_utf8(&label("label-invalid-utf8")).is_err());
    // Invisible characters in a valid-UTF-8 label: the harness's own
    // `safe_text` must see them (zero-width space, BOM, soft hyphen, a tag
    // character).
    let invisible = label("label-invisible-characters");
    let text = String::from_utf8(invisible).expect("valid UTF-8");
    for c in ['\u{200B}', '\u{FEFF}', '\u{AD}', '\u{E0041}', 'a', 'b'] {
        assert!(text.contains(c), "{c:?}");
    }
    assert!(by("label-invisible-characters").values);
    assert!(String::from_utf8_lossy(&label("label-right-to-left-override")).contains('\u{202E}'));
    assert_eq!(label("label-formula-equals"), b"=cmd|' /C calc'!A0");
    assert_eq!(label("label-formula-at"), b"@SUM(1)");
    assert_eq!(label("label-semicolon-formula"), b"a;=1+cmd|' /C calc'!A0");
    assert_eq!(label("label-255-bytes").len(), 255);
    assert!(label("label-empty-png").is_empty());
    // A 300-byte ASCII payload: the excerpt is cut and flagged.
    let long = render_ctx(LONG_ASCII);
    assert!(long.values && long.payloads.iter().flatten().any(|p| p.len() == 65));
    assert!(long.json.contains("\"truncated\":true"));
    // A PNG signature, and an empty file.
    let png = render_ctx(PNG_VALUES);
    assert_eq!(format!("{:?}", audit(&png.file).kind()), "Unsupported(Png)");
    let empty = by("empty-file");
    assert!(empty.file.is_empty());
    assert_eq!(
        format!("{:?}", audit(&empty.file).kind()),
        "Unsupported(Empty)"
    );
    // The hostile labels reach the outputs escaped, never raw.
    for n in AUDIT_RENDER_SEEDS {
        let c = render_ctx(n);
        assert!(
            c.text.chars().all(|ch| !ch.is_control() || ch == '\n'),
            "{n}"
        );
    }
}

#[test]
fn the_two_full_house_inputs_are_audit_seeds() {
    assert_eq!(
        audit_seed("audit-full-house-tiff"),
        full_house_tiff_bytes(),
        "the TIFF seed is the full-house TIFF of the layout tests"
    );
    assert_eq!(audit_seed("audit-full-house-jpeg"), full_house_jpeg_bytes());
    // And the real audit target's guards hold on them.
    audit_invariants::check(&audit_seed("audit-full-house-tiff"));
    audit_invariants::check(&audit_seed("audit-full-house-jpeg"));
    // They carry every category, so the preview guards discriminate on them.
    let jpeg = audit_seed("audit-full-house-jpeg");
    let a = exif_core::audit::audit(&jpeg);
    for c in exif_core::audit::Category::ALL {
        if matches!(
            c,
            exif_core::audit::Category::EmbeddedImage | exif_core::audit::Category::OtherSegment
        ) {
            continue;
        }
        assert!(a.findings_in(c).count() > 0, "{c:?}");
    }
}

/// Every committed seed of the four targets, plus the degenerate inputs and
/// the full-house inputs, as raw `audit_render` inputs (the first bytes pick
/// the options, so each is some case).
fn render_harness_inputs() -> Vec<(String, Vec<u8>)> {
    let mut inputs: Vec<(String, Vec<u8>)> = Vec::new();
    for target in ["tiff_ifd", "jpeg_segments", "audit", "audit_render"] {
        let dir = root().join("fuzz/seeds").join(target);
        for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
            let path = entry.expect("entry").path();
            let data = fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            inputs.push((path.display().to_string(), data));
        }
    }
    for (i, d) in audit_degenerate_inputs().into_iter().enumerate() {
        inputs.push((format!("degenerate {i}"), d));
    }
    inputs.push(("full-house tiff".into(), full_house_tiff_bytes()));
    inputs.push(("full-house jpeg".into(), full_house_jpeg_bytes()));
    inputs
}

/// A little-endian TIFF with one ASCII Make holding `payload`.
fn make_tiff(payload: &[u8]) -> Vec<u8> {
    let mut p = payload.to_vec();
    p.push(0);
    let mut t = b"II\x2A\x00\x08\x00\x00\x00".to_vec();
    t.extend_from_slice(&1u16.to_le_bytes());
    t.extend_from_slice(&0x010Fu16.to_le_bytes());
    t.extend_from_slice(&2u16.to_le_bytes());
    t.extend_from_slice(&u32::try_from(p.len()).expect("small").to_le_bytes());
    t.extend_from_slice(&26u32.to_le_bytes());
    t.extend_from_slice(&0u32.to_le_bytes());
    t.extend_from_slice(&p);
    t
}

#[test]
fn render_harness_holds_on_every_seed_and_degenerate_inputs() {
    let inputs = render_harness_inputs();
    for (name, data) in &inputs {
        let bad = render_invariants::violations(&render_invariants::build(data));
        assert!(bad.is_empty(), "{name}: {bad:?}");
    }
    // Every option byte over a rich input and a hostile one: all 256 option
    // bytes (policies, inventory, values, reasons).
    let rich = full_house_jpeg_bytes();
    for opt in 0..=255u8 {
        for (label_len, extra) in [
            (0u8, &rich[..]),
            (4, &rich[..]),
            (3, &b"\xFF\xFE\x00ab"[..]),
        ] {
            let mut input = vec![opt, label_len];
            input.extend_from_slice(&b"=a,\"\r\x1b"[..usize::from(label_len).min(7)]);
            input.extend_from_slice(extra);
            render_invariants::check(&input);
        }
    }
    // Hostile text is ignored by the phrase check: a label, a failure detail
    // and an excerpt that say "safe", "clean file" and "no personal data" are
    // data, not the report's own words.
    let t = make_tiff(b"safe no personal data clean file");
    let c = render_invariants::build_case(
        &t,
        b"safe clean file no personal data",
        b"safe",
        render_invariants::CaseOpts {
            policy: Some(2),
            custom_byte: 0,
            inventory: true,
            values: true,
            reason: 0,
        },
    );
    assert!(c.text.contains("safe clean file no personal data"));
    assert!(render_invariants::violations(&c).is_empty());
    // Found by the fuzzer: an empty label and a failure detail that ends in
    // "file: " put the label line text inside another line.
    let c = render_invariants::build_case(
        b"",
        b"",
        b"x file: ",
        render_invariants::CaseOpts {
            policy: None,
            custom_byte: 0,
            inventory: false,
            values: false,
            reason: 0,
        },
    );
    assert!(c.text.contains("file: \n"));
    assert!(render_invariants::violations(&c).is_empty());
    // Hostile bytes in every position at once.
    render_invariants::check(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00]);
    render_invariants::check(&[]);
    render_invariants::check(&[0x1B]);
    println!("{} inputs", inputs.len());
}

/// One planted bug: a wrong output (or summary) put into the case built from
/// `input`, which the named guard, run ALONE, must reject.
struct RenderPlant {
    name: &'static str,
    input: &'static str,
    guard: &'static str,
    apply: fn(&mut Ctx),
}

fn rep(s: &mut String, from: &str, to: &str) {
    assert!(
        s.contains(from),
        "the plant's anchor {from:?} is not in the output"
    );
    *s = s.replacen(from, to, 1);
}

/// Add one to the digit run right after `marker`.
fn bump_after(s: &mut String, marker: &str) {
    let at = s.find(marker).unwrap_or_else(|| panic!("no {marker:?}")) + marker.len();
    let digits: String = s[at..].chars().take_while(char::is_ascii_digit).collect();
    let n: u64 = digits.parse().expect("a number");
    s.replace_range(at..at + digits.len(), &(n + 1).to_string());
}

fn csv_rows_of(s: &str) -> Vec<String> {
    s.split_inclusive("\r\n").map(str::to_string).collect()
}

/// Every plant the generic render-guard test runs: the fact guards' plants and
/// the builder guard's.
fn render_plants() -> Vec<RenderPlant> {
    let mut all = fact_render_plants();
    all.extend(builder_plants().into_iter().map(|p| RenderPlant {
        name: p.name,
        input: p.input,
        guard: "builder_equals_oneshot",
        apply: p.apply,
    }));
    all
}

fn fact_render_plants() -> Vec<RenderPlant> {
    vec![
        // json_wellformed
        RenderPlant {
            name: "a trailing comma in the files array",
            input: HOUSE_ALL_PII,
            guard: "json_wellformed",
            apply: |c| rep(&mut c.json, "]}\n", ",]}\n"),
        },
        RenderPlant {
            name: "the document is cut in half",
            input: HOUSE_ALL_PII,
            guard: "json_wellformed",
            apply: |c| {
                let n = c.json.len() / 2;
                c.json.truncate(n);
            },
        },
        RenderPlant {
            name: "a raw control character inside a string",
            input: HOUSE_ALL_PII,
            guard: "json_wellformed",
            apply: |c| rep(&mut c.json, "\"label\":\"", "\"label\":\"\u{1}"),
        },
        RenderPlant {
            name: "a duplicate key",
            input: HOUSE_ALL_PII,
            guard: "json_wellformed",
            apply: |c| {
                rep(
                    &mut c.json,
                    "\"schema_version\":2,",
                    "\"schema_version\":2,\"schema_version\":2,",
                )
            },
        },
        RenderPlant {
            name: "a lone surrogate escape",
            input: HOUSE_ALL_PII,
            guard: "json_wellformed",
            apply: |c| rep(&mut c.json, "\"label\":\"", "\"label\":\"\\ud800"),
        },
        // json_shape
        RenderPlant {
            name: "a key is renamed",
            input: HOUSE_ALL_PII,
            guard: "json_shape",
            apply: |c| rep(&mut c.json, "\"status\":", "\"state\":"),
        },
        RenderPlant {
            name: "two keys swap places",
            input: HOUSE_ALL_PII,
            guard: "json_shape",
            apply: |c| {
                rep(
                    &mut c.json,
                    "\"inventory\":true,\"values\":true}",
                    "\"values\":true,\"inventory\":true}",
                )
            },
        },
        RenderPlant {
            name: "an enumeration value outside its set",
            input: HOUSE_ALL_PII,
            guard: "json_shape",
            apply: |c| rep(&mut c.json, "\"status\":\"ok\"", "\"status\":\"fine\""),
        },
        RenderPlant {
            name: "a negative count",
            input: HOUSE_ALL_PII,
            guard: "json_shape",
            apply: |c| {
                rep(
                    &mut c.json,
                    "\"summary\":{\"files\":2",
                    "\"summary\":{\"files\":-2",
                )
            },
        },
        RenderPlant {
            name: "an extra key",
            input: HOUSE_ALL_PII,
            guard: "json_shape",
            apply: |c| {
                rep(
                    &mut c.json,
                    "\"schema_version\":2,",
                    "\"schema_version\":2,\"extra\":0,",
                )
            },
        },
        RenderPlant {
            name: "a severity outside its set",
            input: HOUSE_ALL_PII,
            guard: "json_shape",
            apply: |c| {
                rep(
                    &mut c.json,
                    "\"severity\":\"medium\"",
                    "\"severity\":\"urgent\"",
                )
            },
        },
        RenderPlant {
            name: "a tag number not in 0xNNNN form",
            input: HOUSE_ALL_PII,
            guard: "json_shape",
            apply: |c| rep(&mut c.json, "\"tag\":\"0x010e\"", "\"tag\":\"0x10e\""),
        },
        // json_facts: both directions of the status iff
        RenderPlant {
            name: "a clean file reported with issues",
            input: HOUSE_ALL_PII,
            guard: "json_facts",
            apply: |c| rep(&mut c.json, "\"status\":\"ok\"", "\"status\":\"issues\""),
        },
        RenderPlant {
            name: "an unsupported file reported ok",
            input: PNG_VALUES,
            guard: "json_facts",
            apply: |c| {
                rep(
                    &mut c.json,
                    "\"status\":\"unsupported\"",
                    "\"status\":\"ok\"",
                )
            },
        },
        RenderPlant {
            name: "a finding in the wrong category",
            input: HOUSE_ALL_PII,
            guard: "json_facts",
            apply: |c| {
                rep(
                    &mut c.json,
                    "\"category\":\"free_text\"",
                    "\"category\":\"gps\"",
                )
            },
        },
        RenderPlant {
            name: "a finding at the wrong tag",
            input: HOUSE_ALL_PII,
            guard: "json_facts",
            apply: |c| rep(&mut c.json, "\"tag\":\"0x010e\"", "\"tag\":\"0x010f\""),
        },
        RenderPlant {
            name: "a removed finding reported kept",
            input: HOUSE_ALL_PII,
            guard: "json_facts",
            apply: |c| {
                rep(
                    &mut c.json,
                    "\"disposition\":\"removed\"",
                    "\"disposition\":\"kept\"",
                )
            },
        },
        RenderPlant {
            name: "a kept_unread finding reported kept",
            input: HOUSE_ALL_PII,
            guard: "json_facts",
            apply: |c| {
                rep(
                    &mut c.json,
                    "\"disposition\":\"kept_unread\"",
                    "\"disposition\":\"kept\"",
                )
            },
        },
        RenderPlant {
            name: "the label is changed",
            input: HOUSE_ALL_PII,
            guard: "json_facts",
            apply: |c| {
                rep(
                    &mut c.json,
                    "photos/full-house.jpg",
                    "photos/full-house.jpG",
                )
            },
        },
        RenderPlant {
            name: "residual findings are off by one on a reliable preview",
            input: HOUSE_ALL_PII,
            guard: "json_facts",
            apply: |c| bump_after(&mut c.json, "\"reliable\":true,\"residual_findings\":"),
        },
        RenderPlant {
            name: "residual findings printed for an unreliable preview",
            input: PNG_VALUES,
            guard: "json_facts",
            apply: |c| {
                rep(
                    &mut c.json,
                    "\"reliable\":false,\"residual_findings\":null",
                    "\"reliable\":false,\"residual_findings\":0",
                )
            },
        },
        RenderPlant {
            name: "an excerpt text is wrong",
            input: HOUSE_ALL_PII,
            guard: "json_facts",
            apply: |c| {
                rep(
                    &mut c.json,
                    "\"value\":{\"text\":\"a\"",
                    "\"value\":{\"text\":\"b\"",
                )
            },
        },
        RenderPlant {
            name: "an excerpt is missing with values on",
            input: HOUSE_ALL_PII,
            guard: "json_facts",
            apply: |c| {
                rep(
                    &mut c.json,
                    "\"value\":{\"text\":\"a\",\"truncated\":false}",
                    "\"value\":null",
                )
            },
        },
        RenderPlant {
            name: "an inventory item names a category it has no finding for",
            input: HOUSE_ALL_PII,
            guard: "json_facts",
            apply: |c| {
                let at = c.json.find("\"inventory\":[").expect("inventory");
                let tail = c.json.split_off(at);
                c.json
                    .push_str(&tail.replacen("\"category\":null", "\"category\":\"gps\"", 1));
            },
        },
        RenderPlant {
            name: "the inventory item count is off by one",
            input: HOUSE_ALL_PII,
            guard: "json_facts",
            apply: |c| bump_after(&mut c.json, "\"inventory_items\":"),
        },
        RenderPlant {
            name: "the failure reason is changed",
            input: HOUSE_EVERYTHING,
            guard: "json_facts",
            apply: |c| {
                rep(
                    &mut c.json,
                    "\"reason\":\"too_large\"",
                    "\"reason\":\"other\"",
                )
            },
        },
        // csv_wellformed
        RenderPlant {
            name: "the final CRLF is missing",
            input: HOUSE_ALL_PII,
            guard: "csv_wellformed",
            apply: |c| {
                c.csv.pop();
                c.csv.pop();
            },
        },
        RenderPlant {
            name: "a ragged row",
            input: HOUSE_ALL_PII,
            guard: "csv_wellformed",
            apply: |c| rep(&mut c.csv, ",removed,a,false\r\n", ",removed,a\r\n"),
        },
        RenderPlant {
            name: "a bare LF line end",
            input: HOUSE_ALL_PII,
            guard: "csv_wellformed",
            apply: |c| rep(&mut c.csv, "\r\nphotos", "\nphotos"),
        },
        RenderPlant {
            name: "a bare quote in an unquoted field",
            input: HOUSE_ALL_PII,
            guard: "csv_wellformed",
            apply: |c| rep(&mut c.csv, "\r\nphotos/", "\r\npho\"tos/"),
        },
        RenderPlant {
            name: "an unterminated quoted field",
            input: HOUSE_ALL_PII,
            guard: "csv_wellformed",
            apply: |c| rep(&mut c.csv, "\r\nphotos/", "\r\n\"photos/"),
        },
        // csv_rows
        RenderPlant {
            name: "the header is changed",
            input: HOUSE_ALL_PII,
            guard: "csv_rows",
            apply: |c| rep(&mut c.csv, "value_truncated", "value_trunc"),
        },
        RenderPlant {
            name: "a row is missing",
            input: HOUSE_ALL_PII,
            guard: "csv_rows",
            apply: |c| {
                let mut rows = csv_rows_of(&c.csv);
                rows.remove(2);
                c.csv = rows.concat();
            },
        },
        RenderPlant {
            name: "the failed file has no row",
            input: HOUSE_ALL_PII,
            guard: "csv_rows",
            apply: |c| {
                let mut rows = csv_rows_of(&c.csv);
                rows.pop();
                c.csv = rows.concat();
            },
        },
        RenderPlant {
            name: "an inventory item without a finding is dropped",
            input: HOUSE_ALL_PII,
            guard: "csv_rows",
            apply: |c| {
                let mut rows = csv_rows_of(&c.csv);
                let at = rows
                    .iter()
                    .position(|r| r.contains(",0x8769,"))
                    .expect("pointer");
                rows.remove(at);
                c.csv = rows.concat();
            },
        },
        RenderPlant {
            name: "a label that starts a formula is not guarded",
            input: HOUSE_ALL_PII,
            guard: "csv_rows",
            apply: |c| {
                rep(
                    &mut c.csv,
                    "\r\nphotos/full-house.jpg,",
                    "\r\n=photos/full-house.jpg,",
                )
            },
        },
        RenderPlant {
            name: "a label is guarded that needs no guard",
            input: HOUSE_ALL_PII,
            guard: "csv_rows",
            apply: |c| {
                rep(
                    &mut c.csv,
                    "\r\nphotos/full-house.jpg,",
                    "\r\n'photos/full-house.jpg,",
                )
            },
        },
        RenderPlant {
            name: "a finding row names the wrong category",
            input: HOUSE_ALL_PII,
            guard: "csv_rows",
            apply: |c| rep(&mut c.csv, ",free_text,", ",gps,"),
        },
        RenderPlant {
            name: "a field with a semicolon written unquoted",
            input: SEMICOLON,
            guard: "csv_rows",
            apply: |c| {
                rep(
                    &mut c.csv,
                    "\"a;=1+cmd|' /C calc'!A0\"",
                    "a;=1+cmd|' /C calc'!A0",
                )
            },
        },
        RenderPlant {
            name: "a plain field written quoted",
            input: SEMICOLON,
            guard: "csv_rows",
            apply: |c| rep(&mut c.csv, ",tiff,", ",\"tiff\","),
        },
        // text_safe
        RenderPlant {
            name: "a zero-width space",
            input: HOUSE_ALL_PII,
            guard: "text_safe",
            apply: |c| c.text.insert(3, '\u{200B}'),
        },
        RenderPlant {
            name: "a tag character",
            input: HOUSE_ALL_PII,
            guard: "text_safe",
            apply: |c| c.text.insert(3, '\u{E0041}'),
        },
        RenderPlant {
            name: "a soft hyphen and a BOM",
            input: HOUSE_ALL_PII,
            guard: "text_safe",
            apply: |c| {
                c.text.insert(3, '\u{AD}');
                c.text.insert(3, '\u{FEFF}');
            },
        },
        RenderPlant {
            name: "a terminal escape character",
            input: HOUSE_ALL_PII,
            guard: "text_safe",
            apply: |c| c.text.insert(0, '\u{1b}'),
        },
        RenderPlant {
            name: "a right-to-left override",
            input: HOUSE_ALL_PII,
            guard: "text_safe",
            apply: |c| c.text.insert(3, '\u{202E}'),
        },
        RenderPlant {
            name: "a C1 control",
            input: HOUSE_ALL_PII,
            guard: "text_safe",
            apply: |c| c.text.insert(3, '\u{85}'),
        },
        RenderPlant {
            name: "a DEL",
            input: HOUSE_ALL_PII,
            guard: "text_safe",
            apply: |c| c.text.insert(3, '\u{7f}'),
        },
        RenderPlant {
            name: "the word safe",
            input: HOUSE_ALL_PII,
            guard: "text_safe",
            apply: |c| c.text.push_str("this report is safe\n"),
        },
        RenderPlant {
            name: "the phrase clean file",
            input: HOUSE_ALL_PII,
            guard: "text_safe",
            apply: |c| c.text.push_str("a clean file\n"),
        },
        RenderPlant {
            name: "the phrase no personal data",
            input: PNG_VALUES,
            guard: "text_safe",
            apply: |c| c.text.push_str("  No Personal Data\n"),
        },
        RenderPlant {
            name: "the text is not deterministic",
            input: HOUSE_ALL_PII,
            guard: "text_safe",
            apply: |c| c.text_again.push('x'),
        },
        RenderPlant {
            name: "the label line is wrong",
            input: HOUSE_ALL_PII,
            guard: "text_safe",
            apply: |c| {
                rep(
                    &mut c.text,
                    "file: photos/full-house.jpg",
                    "file: photos/other.jpg",
                )
            },
        },
        RenderPlant {
            name: "no final newline",
            input: HOUSE_ALL_PII,
            guard: "text_safe",
            apply: |c| {
                c.text.pop();
            },
        },
        // values_gate: values off
        RenderPlant {
            name: "a JSON excerpt with values off",
            input: HOUSE_EVERYTHING,
            guard: "values_gate",
            apply: |c| {
                rep(
                    &mut c.json,
                    "\"value\":null",
                    "\"value\":{\"text\":\"a\",\"truncated\":false}",
                )
            },
        },
        RenderPlant {
            name: "a CSV excerpt with values off",
            input: HOUSE_EVERYTHING,
            guard: "values_gate",
            apply: |c| rep(&mut c.csv, ",removed,,\r\n", ",removed,a,false\r\n"),
        },
        RenderPlant {
            name: "a text excerpt with values off",
            input: HOUSE_EVERYTHING,
            guard: "values_gate",
            apply: |c| {
                rep(
                    &mut c.text,
                    " block=0 segment=0",
                    " value=\"a\" block=0 segment=0",
                )
            },
        },
        // values_gate: values on
        RenderPlant {
            name: "an excerpt flagged truncated that is not",
            input: HOUSE_ALL_PII,
            guard: "values_gate",
            apply: |c| rep(&mut c.json, "\"truncated\":false", "\"truncated\":true"),
        },
        RenderPlant {
            name: "a cut excerpt not flagged truncated",
            input: LONG_ASCII,
            guard: "values_gate",
            apply: |c| rep(&mut c.json, "\"truncated\":true", "\"truncated\":false"),
        },
        RenderPlant {
            name: "an excerpt longer than 64 source bytes",
            input: LONG_ASCII,
            guard: "values_gate",
            apply: |c| {
                let long = "x".repeat(300);
                rep(&mut c.json, "\"text\":\"", &format!("\"text\":\"{long}"))
            },
        },
        RenderPlant {
            name: "a text excerpt is missing with values on",
            input: HOUSE_ALL_PII,
            guard: "values_gate",
            apply: |c| rep(&mut c.text, " value=\"a\"", ""),
        },
        RenderPlant {
            name: "the CSV truncated flag disagrees",
            input: HOUSE_ALL_PII,
            guard: "values_gate",
            apply: |c| rep(&mut c.csv, ",a,false\r\n", ",a,true\r\n"),
        },
        // summary_facts
        RenderPlant {
            name: "ok is counted twice",
            input: HOUSE_ALL_PII,
            guard: "summary_facts",
            apply: |c| c.summary.ok += 1,
        },
        RenderPlant {
            name: "findings are over-counted",
            input: HOUSE_ALL_PII,
            guard: "summary_facts",
            apply: |c| c.summary.findings += 1,
        },
        RenderPlant {
            name: "a category count is off by one",
            input: HOUSE_ALL_PII,
            guard: "summary_facts",
            apply: |c| c.summary.by_category[0].1 += 1,
        },
        RenderPlant {
            name: "reliable and unreliable swapped",
            input: HOUSE_ALL_PII,
            guard: "summary_facts",
            apply: |c| {
                c.summary.reliable = 0;
                c.summary.unreliable = 1;
            },
        },
        RenderPlant {
            name: "residual findings missing with a policy",
            input: HOUSE_ALL_PII,
            guard: "summary_facts",
            apply: |c| c.summary.residual = None,
        },
        RenderPlant {
            name: "an unreliable file's residuals are counted",
            input: PNG_VALUES,
            guard: "summary_facts",
            apply: |c| c.summary.residual = Some(3),
        },
        RenderPlant {
            name: "a zero residual over no reliable file (summary)",
            input: PNG_VALUES,
            guard: "summary_facts",
            apply: |c| c.summary.residual = Some(0),
        },
        RenderPlant {
            name: "a zero residual over no reliable file (JSON summary)",
            input: PNG_VALUES,
            guard: "summary_facts",
            apply: |c| {
                rep(
                    &mut c.json,
                    "\"residual_findings\":null,\"reliable_files\"",
                    "\"residual_findings\":0,\"reliable_files\"",
                )
            },
        },
        RenderPlant {
            name: "the maximum severity is wrong",
            input: HOUSE_ALL_PII,
            guard: "summary_facts",
            apply: |c| c.summary.max_severity = Some("Low".into()),
        },
        RenderPlant {
            name: "the JSON summary disagrees",
            input: HOUSE_ALL_PII,
            guard: "summary_facts",
            apply: |c| {
                rep(
                    &mut c.json,
                    "\"summary\":{\"files\":2",
                    "\"summary\":{\"files\":3",
                )
            },
        },
        RenderPlant {
            name: "unsupported is counted as zero",
            input: PNG_VALUES,
            guard: "summary_facts",
            apply: |c| {
                rep(
                    &mut c.json,
                    "\"unsupported\":1,\"unreadable\":0",
                    "\"unsupported\":0,\"unreadable\":0",
                )
            },
        },
        // render_linear
        RenderPlant {
            name: "a JSON document far larger than its items",
            input: HOUSE_ALL_PII,
            guard: "render_linear",
            apply: |c| c.json.push_str(&" ".repeat(10_000_000)),
        },
        RenderPlant {
            name: "a text report far larger than its items",
            input: HOUSE_ALL_PII,
            guard: "render_linear",
            apply: |c| c.text.push_str(&"x".repeat(10_000_000)),
        },
        RenderPlant {
            name: "a CSV with far more rows than items",
            input: HOUSE_ALL_PII,
            guard: "render_linear",
            apply: |c| c.csv.push_str(&"a\r\n".repeat(10_000)),
        },
    ]
}

// ── SPEC-018: the builder guard (`builder_equals_oneshot`) ───────────────────

/// Which side of a comparison a plant breaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    /// The builder is wrong, the one-shot is right.
    Builder,
    /// The builder is right, the one-shot (or the bookkeeping) is wrong.
    OneShot,
}

struct BuilderPlant {
    name: &'static str,
    input: &'static str,
    side: Side,
    apply: fn(&mut Ctx),
}

/// Index of a pair by name (`scenario/format`).
fn pair_at(c: &Ctx, name: &str) -> usize {
    c.pairs
        .iter()
        .position(|p| p.name == name)
        .unwrap_or_else(|| panic!("no comparison {name}"))
}

/// Change one letter's case: the same length, different content.
fn same_length_corrupt(s: &mut String) {
    let before = s.len();
    let at = s
        .char_indices()
        .find(|(_, ch)| ch.is_ascii_alphabetic())
        .map(|(i, _)| i)
        .expect("a letter to flip");
    let flipped = if s[at..].starts_with(|c: char| c.is_ascii_lowercase()) {
        s[at..at + 1].to_ascii_uppercase()
    } else {
        s[at..at + 1].to_ascii_lowercase()
    };
    s.replace_range(at..at + 1, &flipped);
    assert_eq!(s.len(), before);
}

/// Where `same_length_corrupt_at` flips a letter.
#[derive(Clone, Copy)]
enum Where {
    /// The last ASCII letter of the output.
    End,
    /// The ASCII letter nearest the middle byte.
    Middle,
}

/// Change one letter's case at `at`, keeping the length (SPEC-018 verify D1: a
/// guard that compares the length plus the first 16 bytes must not pass).
fn same_length_corrupt_at(s: &mut String, at: Where) {
    let before = s.len();
    let letters: Vec<usize> = s
        .char_indices()
        .filter(|(_, ch)| ch.is_ascii_alphabetic())
        .map(|(i, _)| i)
        .collect();
    let i = match at {
        Where::End => *letters.last().expect("a letter"),
        Where::Middle => *letters
            .iter()
            .min_by_key(|i| i.abs_diff(before / 2))
            .expect("a letter"),
    };
    assert!(i >= 16, "the flipped letter is past the first 16 bytes");
    let flipped = if s[i..].starts_with(|c: char| c.is_ascii_lowercase()) {
        s[i..i + 1].to_ascii_uppercase()
    } else {
        s[i..i + 1].to_ascii_lowercase()
    };
    s.replace_range(i..=i, &flipped);
    assert_eq!(s.len(), before);
}

fn builder_plants() -> Vec<BuilderPlant> {
    vec![
        BuilderPlant {
            name: "same-length corruption at the END of the builder output of reverse/json",
            input: HOUSE_ALL_PII,
            side: Side::Builder,
            apply: |c| {
                let i = pair_at(c, "reverse/json");
                same_length_corrupt_at(&mut c.pairs[i].builder, Where::End);
            },
        },
        BuilderPlant {
            name: "same-length corruption at the END of the builder output of reverse/text",
            input: HOUSE_ALL_PII,
            side: Side::Builder,
            apply: |c| {
                let i = pair_at(c, "reverse/text");
                same_length_corrupt_at(&mut c.pairs[i].builder, Where::End);
            },
        },
        BuilderPlant {
            name: "same-length corruption in the MIDDLE of the builder output of reverse/csv",
            input: HOUSE_ALL_PII,
            side: Side::Builder,
            apply: |c| {
                let i = pair_at(c, "reverse/csv");
                same_length_corrupt_at(&mut c.pairs[i].builder, Where::Middle);
            },
        },
        BuilderPlant {
            name:
                "same-length corruption in the MIDDLE of the builder output of alone-audited/json",
            input: HOUSE_ALL_PII,
            side: Side::Builder,
            apply: |c| {
                let i = pair_at(c, "alone-audited/json");
                same_length_corrupt_at(&mut c.pairs[i].builder, Where::Middle);
            },
        },
        BuilderPlant {
            name: "same-length corruption at the END of the one-shot output of alone-failed/csv",
            input: HOUSE_ALL_PII,
            side: Side::OneShot,
            apply: |c| {
                let i = pair_at(c, "alone-failed/csv");
                same_length_corrupt_at(&mut c.pairs[i].oneshot, Where::End);
            },
        },
        BuilderPlant {
            name: "same-length corruption of the builder output of forward/text",
            input: HOUSE_ALL_PII,
            side: Side::Builder,
            apply: |c| {
                let i = pair_at(c, "forward/text");
                same_length_corrupt(&mut c.pairs[i].builder);
            },
        },
        BuilderPlant {
            name: "same-length corruption of the builder output of forward/json",
            input: HOUSE_ALL_PII,
            side: Side::Builder,
            apply: |c| {
                let i = pair_at(c, "forward/json");
                same_length_corrupt(&mut c.pairs[i].builder);
            },
        },
        BuilderPlant {
            name: "same-length corruption of the builder output of forward/csv",
            input: HOUSE_ALL_PII,
            side: Side::Builder,
            apply: |c| {
                let i = pair_at(c, "forward/csv");
                same_length_corrupt(&mut c.pairs[i].builder);
            },
        },
        BuilderPlant {
            name: "same-length corruption of the builder output of reverse/text",
            input: HOUSE_ALL_PII,
            side: Side::Builder,
            apply: |c| {
                let i = pair_at(c, "reverse/text");
                same_length_corrupt(&mut c.pairs[i].builder);
            },
        },
        BuilderPlant {
            name: "same-length corruption of the builder output of reverse/json",
            input: HOUSE_ALL_PII,
            side: Side::Builder,
            apply: |c| {
                let i = pair_at(c, "reverse/json");
                same_length_corrupt(&mut c.pairs[i].builder);
            },
        },
        BuilderPlant {
            name: "same-length corruption of the builder output of reverse/csv",
            input: HOUSE_ALL_PII,
            side: Side::Builder,
            apply: |c| {
                let i = pair_at(c, "reverse/csv");
                same_length_corrupt(&mut c.pairs[i].builder);
            },
        },
        BuilderPlant {
            name: "same-length corruption of the builder output of alone-audited/text",
            input: HOUSE_ALL_PII,
            side: Side::Builder,
            apply: |c| {
                let i = pair_at(c, "alone-audited/text");
                same_length_corrupt(&mut c.pairs[i].builder);
            },
        },
        BuilderPlant {
            name: "same-length corruption of the builder output of alone-audited/json",
            input: HOUSE_ALL_PII,
            side: Side::Builder,
            apply: |c| {
                let i = pair_at(c, "alone-audited/json");
                same_length_corrupt(&mut c.pairs[i].builder);
            },
        },
        BuilderPlant {
            name: "same-length corruption of the builder output of alone-audited/csv",
            input: HOUSE_ALL_PII,
            side: Side::Builder,
            apply: |c| {
                let i = pair_at(c, "alone-audited/csv");
                same_length_corrupt(&mut c.pairs[i].builder);
            },
        },
        BuilderPlant {
            name: "same-length corruption of the builder output of alone-failed/text",
            input: HOUSE_ALL_PII,
            side: Side::Builder,
            apply: |c| {
                let i = pair_at(c, "alone-failed/text");
                same_length_corrupt(&mut c.pairs[i].builder);
            },
        },
        BuilderPlant {
            name: "same-length corruption of the builder output of alone-failed/json",
            input: HOUSE_ALL_PII,
            side: Side::Builder,
            apply: |c| {
                let i = pair_at(c, "alone-failed/json");
                same_length_corrupt(&mut c.pairs[i].builder);
            },
        },
        BuilderPlant {
            name: "same-length corruption of the builder output of alone-failed/csv",
            input: HOUSE_ALL_PII,
            side: Side::Builder,
            apply: |c| {
                let i = pair_at(c, "alone-failed/csv");
                same_length_corrupt(&mut c.pairs[i].builder);
            },
        },
        BuilderPlant {
            name: "same-length corruption of the one-shot output of forward/json",
            input: HOUSE_ALL_PII,
            side: Side::OneShot,
            apply: |c| {
                let i = pair_at(c, "forward/json");
                same_length_corrupt(&mut c.pairs[i].oneshot);
            },
        },
        BuilderPlant {
            name: "same-length corruption of the one-shot output of reverse/text",
            input: HOUSE_ALL_PII,
            side: Side::OneShot,
            apply: |c| {
                let i = pair_at(c, "reverse/text");
                same_length_corrupt(&mut c.pairs[i].oneshot);
            },
        },
        BuilderPlant {
            name: "same-length corruption of the one-shot output of reverse/csv",
            input: HOUSE_ALL_PII,
            side: Side::OneShot,
            apply: |c| {
                let i = pair_at(c, "reverse/csv");
                same_length_corrupt(&mut c.pairs[i].oneshot);
            },
        },
        BuilderPlant {
            name: "same-length corruption of the one-shot output of alone-audited/json",
            input: HOUSE_ALL_PII,
            side: Side::OneShot,
            apply: |c| {
                let i = pair_at(c, "alone-audited/json");
                same_length_corrupt(&mut c.pairs[i].oneshot);
            },
        },
        BuilderPlant {
            name: "same-length corruption of the one-shot output of alone-failed/csv",
            input: HOUSE_ALL_PII,
            side: Side::OneShot,
            apply: |c| {
                let i = pair_at(c, "alone-failed/csv");
                same_length_corrupt(&mut c.pairs[i].oneshot);
            },
        },
        // The builder is wrong.
        BuilderPlant {
            name: "the builder drops the last fragment (the CSV loses its last row)",
            input: HOUSE_ALL_PII,
            side: Side::Builder,
            apply: |c| {
                let i = pair_at(c, "forward/csv");
                let b = &mut c.pairs[i].builder;
                let body = &b[..b.len() - 2];
                let cut = body.rfind("\r\n").expect("a row boundary") + 2;
                b.truncate(cut);
            },
        },
        BuilderPlant {
            name: "a fragment is rendered with the options of a different builder (no values)",
            input: HOUSE_ALL_PII,
            side: Side::Builder,
            apply: |c| {
                let i = pair_at(c, "alone-audited/text");
                rep(&mut c.pairs[i].builder, " value=\"", " v=\"");
            },
        },
        BuilderPlant {
            name: "the builder's summary is the one of the entries before the last",
            input: HOUSE_ALL_PII,
            side: Side::Builder,
            apply: |c| {
                let before_last = c.summary_pairs[1].0.clone();
                c.summary_pairs[2].0 = before_last;
            },
        },
        BuilderPlant {
            name: "the builder joins JSON fragments with a space after the comma",
            input: HOUSE_ALL_PII,
            side: Side::Builder,
            apply: |c| {
                let i = pair_at(c, "reverse/json");
                rep(&mut c.pairs[i].builder, "},{\"label\":", "}, {\"label\":");
            },
        },
        // The builder is right and the other side is wrong: an iff.
        BuilderPlant {
            name: "the one-shot output gains a byte",
            input: HOUSE_ALL_PII,
            side: Side::OneShot,
            apply: |c| {
                let i = pair_at(c, "reverse/csv");
                c.pairs[i].oneshot.push('x');
            },
        },
        BuilderPlant {
            name: "the summarize side counts one more file",
            input: HOUSE_ALL_PII,
            side: Side::OneShot,
            apply: |c| c.summary_pairs[1].1.files += 1,
        },
        BuilderPlant {
            name: "a comparison is missing",
            input: HOUSE_ALL_PII,
            side: Side::OneShot,
            apply: |c| {
                c.pairs.pop();
            },
        },
        BuilderPlant {
            name: "builder and one-shot agree with each other but not with the case's JSON",
            input: HOUSE_ALL_PII,
            side: Side::OneShot,
            apply: |c| {
                let i = pair_at(c, "forward/json");
                c.pairs[i].builder.push(' ');
                c.pairs[i].oneshot.push(' ');
            },
        },
    ]
}

#[test]
fn builder_guard_holds_on_every_seed_and_degenerate_inputs() {
    let inputs = render_harness_inputs();
    for (name, data) in &inputs {
        let c = render_invariants::build(data);
        assert_eq!(c.pairs.len(), render_invariants::BUILDER_PAIRS, "{name}");
        render_invariants::run_guard("builder_equals_oneshot", &c)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
    }
    // All 256 option bytes over a rich input and a hostile one.
    let rich = full_house_jpeg_bytes();
    for opt in 0..=255u8 {
        for (label_len, extra) in [
            (0u8, &rich[..]),
            (3, &b"\xFF\xFE\x00ab"[..]),
            (4, &rich[..]),
        ] {
            let mut input = vec![opt, label_len];
            input.extend_from_slice(&b"=a,\"\r\x1b"[..usize::from(label_len).min(7)]);
            input.extend_from_slice(extra);
            let c = render_invariants::build(&input);
            render_invariants::run_guard("builder_equals_oneshot", &c)
                .unwrap_or_else(|e| panic!("option byte {opt}: {e}"));
        }
    }
    // Degenerate: nothing at all, one byte, an empty label and file.
    for input in [&[][..], &[0x1B], &[0xFF, 0xFF, 0xFF], &[0x18, 0x00]] {
        let c = render_invariants::build(input);
        render_invariants::run_guard("builder_equals_oneshot", &c).expect("degenerate input");
    }
    // The seeds' policies cover the Custom case.
    for seed in AUDIT_RENDER_SEEDS {
        let c = render_ctx(seed);
        render_invariants::run_guard("builder_equals_oneshot", &c)
            .unwrap_or_else(|e| panic!("{seed}: {e}"));
    }
    println!("{} inputs", inputs.len());
}

#[test]
fn builder_guard_goes_red_alone_when_planted() {
    let plants = builder_plants();
    for side in [Side::Builder, Side::OneShot] {
        let n = plants.iter().filter(|p| p.side == side).count();
        assert!(n >= 2, "{side:?}: {n} plant(s), want at least two");
    }
    for p in &plants {
        let good = render_ctx(p.input);
        // CONTROL: the unplanted case is green, for this guard alone and for
        // all of them.
        assert_eq!(
            render_invariants::run_guard("builder_equals_oneshot", &good),
            Ok(()),
            "{}: control",
            p.name
        );
        let mut bad = good.clone();
        (p.apply)(&mut bad);
        let flat = |c: &Ctx| format!("{:?}", (&c.pairs, &c.summary_pairs));
        assert!(
            flat(&bad) != flat(&good),
            "{}: the plant changed nothing",
            p.name
        );
        let alone = render_invariants::run_guard("builder_equals_oneshot", &bad);
        assert!(
            alone.is_err(),
            "{}: the builder guard did not reject the planted bug ({:?})",
            p.name,
            p.side
        );
    }
    // The other direction of the iff: a change that keeps builder and
    // one-shot EQUAL is accepted (the guard compares them, it does not pin
    // the text), on every scenario that is not the case's own rendering.
    let good = render_ctx(HOUSE_ALL_PII);
    for name in [
        "reverse/text",
        "reverse/json",
        "reverse/csv",
        "alone-audited/csv",
        "alone-failed/json",
    ] {
        let mut same = good.clone();
        let i = pair_at(&same, name);
        same.pairs[i].builder.push_str("<changed>");
        same.pairs[i].oneshot.push_str("<changed>");
        assert_eq!(
            render_invariants::run_guard("builder_equals_oneshot", &same),
            Ok(()),
            "{name}: equal sides are accepted"
        );
        // And a same-length change applied identically to both sides.
        let mut same = good.clone();
        same_length_corrupt(&mut same.pairs[i].builder);
        same_length_corrupt(&mut same.pairs[i].oneshot);
        assert_eq!(
            render_invariants::run_guard("builder_equals_oneshot", &same),
            Ok(()),
            "{name}: equal sides are accepted"
        );
    }
    println!("{} builder plants, each rejected alone", plants.len());
}

#[test]
fn builder_guard_is_linear() {
    let steps_of = |data: &[u8]| -> (u64, usize) {
        let mut input = vec![3 | 8 | 16, 4];
        input.extend_from_slice(b"f.jp");
        input.extend_from_slice(data);
        let c = render_invariants::build(&input);
        render_invariants::reset_steps();
        render_invariants::run_guard("builder_equals_oneshot", &c).expect("guard holds");
        (
            render_invariants::steps(),
            c.want.items.len() + c.want.findings.len(),
        )
    };
    for (name, big, small) in [
        ("thumbnails", many_thumbnails(5_000), many_thumbnails(2_500)),
        ("regions", many_regions(3_000), many_regions(1_500)),
    ] {
        let (steps, n) = steps_of(&big);
        assert!(n >= 5_900, "{name}: {n} items and findings");
        let (half, n_half) = steps_of(&small);
        assert!(n_half * 2 <= n + 8, "{name}: the small input is half size");
        // Steps are items and findings plus output bytes / 256 (about eight
        // per item at the largest): a rescan per finding would be quadratic.
        let bound = 40 * n as u64 + 256;
        println!("{name}: {half} steps at half size, {steps} at full ({bound} allowed)");
        assert!(
            steps <= bound,
            "{name}: {steps} steps for {n} items and findings (bound {bound}): not linear"
        );
        assert!(
            steps <= 3 * half,
            "{name}: {half} steps at half the size, {steps} at full: not linear"
        );
    }
}

#[test]
fn each_render_guard_goes_red_alone_when_planted() {
    let plants = render_plants();
    // Every guard has at least two plants (an iff or a bound: one per
    // direction where it has both).
    for (name, _) in render_invariants::GUARDS {
        let n = plants.iter().filter(|p| p.guard == name).count();
        assert!(n >= 2, "guard {name} has {n} plant(s), want at least two");
    }
    for p in &plants {
        let good = render_ctx(p.input);
        // CONTROL: the unplanted case satisfies every guard.
        let control = render_invariants::violations(&good);
        assert!(
            control.is_empty(),
            "{}: control on {}: {control:?}",
            p.name,
            p.input
        );
        // MUTATION: rejected by the named guard ALONE.
        let mut bad = good.clone();
        (p.apply)(&mut bad);
        let flat = |c: &Ctx| {
            format!(
                "{:?}",
                (
                    &c.json,
                    &c.csv,
                    &c.text,
                    &c.text_again,
                    &c.summary,
                    &c.pairs,
                    &c.summary_pairs
                )
            )
        };
        assert!(
            flat(&bad) != flat(&good),
            "{}: the plant changed nothing",
            p.name
        );
        let alone = render_invariants::run_guard(p.guard, &bad);
        assert!(
            alone.is_err(),
            "{}: guard {} did not reject the planted bug on {}",
            p.name,
            p.guard,
            p.input
        );
    }
    println!(
        "{} render plants, each rejected by its own guard alone",
        plants.len()
    );
}

/// The steps the render guards take on `data` as a case (counted, not timed).
fn render_guard_steps(data: &[u8]) -> (u64, usize) {
    let mut input = vec![3 | 8 | 16, 4];
    input.extend_from_slice(b"f.jp");
    input.extend_from_slice(data);
    let c = render_invariants::build(&input);
    render_invariants::reset_steps();
    let bad = render_invariants::violations(&c);
    assert!(
        bad.is_empty(),
        "render guards fail on the linearity input: {bad:?}"
    );
    (
        render_invariants::steps(),
        c.want.items.len() + c.want.findings.len(),
    )
}

#[test]
fn render_guards_are_linear() {
    for (name, big, small) in [
        ("thumbnails", many_thumbnails(5_000), many_thumbnails(2_500)),
        ("regions", many_regions(3_000), many_regions(1_500)),
    ] {
        let (steps, n) = render_guard_steps(&big);
        assert!(n >= 5_900, "{name}: {n} items and findings");
        // About a dozen steps per item and per finding across the nine guards;
        // a rescan per finding would be quadratic.
        let bound = 40 * n as u64 + 256;
        assert!(
            steps <= bound,
            "{name}: {steps} steps for {n} items and findings (bound {bound}): not linear"
        );
        let (half, _) = render_guard_steps(&small);
        println!("{name}: {half} steps at half size, {steps} at full ({bound} allowed)");
        assert!(
            steps <= 3 * half,
            "{name}: {half} steps at half the size, {steps} at full: not linear"
        );
    }
}

// ── SPEC-014 F3: the preview harness's Custom case is non-trivial ────────────

#[test]
fn preview_custom_case_is_non_trivial() {
    // Over the inputs of the layout tests, the harness's `Custom` case must
    // sometimes remove a set that is neither empty nor `all()`, and on some
    // audit that set must split the dispositions: otherwise the case could
    // silently become `Custom(none)` and test nothing (the vG mutant).
    let mut nontrivial_sets = 0usize;
    let mut splitting = 0usize;
    for (name, data) in render_harness_inputs() {
        let names = audit_invariants::custom_names(&data);
        if !names.is_empty() && names.len() < 13 {
            nontrivial_sets += 1;
        }
        let pin = preview_input_of(&data);
        let custom = &pin.cases[5];
        assert_eq!(custom.label, "custom", "{name}");
        let d = &custom.want.dispositions;
        if d.iter().any(|x| x == "removed") && d.iter().any(|x| x != "removed") {
            splitting += 1;
        }
    }
    assert!(
        nontrivial_sets >= 50,
        "only {nontrivial_sets} inputs give a non-trivial custom set"
    );
    assert!(
        splitting >= 3,
        "only {splitting} audits are split by their custom set"
    );
}

// ── SPEC-016: the `cli_args` target (argument parser, DEC-011 part 1) ────────

// The real parser, as the fuzz target includes it.
#[allow(dead_code)]
#[path = "../src/bin/exif-audit/args.rs"]
mod cli_args_src;
include!("../fuzz/shared/cli_invariants.rs");

use cli_args_src::Outcome as CliOutcome;
use cli_invariants::Ctx as CliCtx;

fn cli_seed_path(name: &str) -> PathBuf {
    root().join("fuzz/seeds/cli_args").join(name)
}

fn cli_ctx(seed: &str) -> CliCtx {
    let path = cli_seed_path(seed);
    let bytes = fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    cli_invariants::build(&bytes)
}

/// The outcome of parsing these arguments (a way to get a `Usage` or a `Run`
/// value without constructing the parser's private types).
fn cli_outcome_of(args: &[&str]) -> CliOutcome {
    let os: Vec<std::ffi::OsString> = args.iter().map(std::ffi::OsString::from).collect();
    cli_args_src::parse(&os)
}

/// Every committed `cli_args` seed with the class of outcome it claims.
const CLI_ARGS_SEEDS: [(&str, &str); 30] = [
    ("cli-csv-shorthand", "run"),
    ("cli-defaults", "run"),
    ("cli-double-dash", "run"),
    ("cli-empty-path-and-option", "run"),
    ("cli-equals-forms", "run"),
    ("cli-every-option", "run"),
    (
        "cli-flag-with-value",
        "usage: --strict does not take a value",
    ),
    (
        "cli-format-json-conflict",
        "usage: only one of --format, --json and --csv may be given",
    ),
    ("cli-help-after-dash-dash", "run"),
    ("cli-help-after-invalid", "help"),
    ("cli-help-and-version", "help"),
    ("cli-hostile-bytes", "usage: unknown option: --\\xff\\xfe"),
    (
        "cli-json-csv-conflict",
        "usage: only one of --format, --json and --csv may be given",
    ),
    ("cli-json-shorthand", "run"),
    ("cli-lone-dash", "usage: a lone - is not a path"),
    ("cli-long-unknown-option", "usage: unknown option: --aaaa"),
    ("cli-missing-value", "usage: missing value for --format"),
    ("cli-no-path", "usage: no path given"),
    ("cli-non-utf8-value", "usage: bad value for --format: \\xff"),
    ("cli-repeat-flag", "usage: --strict given more than once"),
    ("cli-repeat-option", "usage: --policy given more than once"),
    ("cli-size-largest-fit", "run"),
    (
        "cli-size-lowercase-suffix",
        "usage: bad value for --max-size: 1k",
    ),
    (
        "cli-size-overflow",
        "usage: bad value for --max-size: 18446744073709551616",
    ),
    ("cli-size-zero", "usage: bad value for --max-size: 0"),
    ("cli-two-empty-arguments", "run"),
    ("cli-two-paths", "run"),
    ("cli-unknown-option", "usage: unknown option: -x"),
    (
        "cli-value-looks-like-option",
        "usage: bad value for --policy: --json",
    ),
    ("cli-version", "version"),
];

#[test]
fn cli_args_target_is_declared_and_runs_in_ci() {
    let fuzz = read("fuzz/Cargo.toml");
    assert!(
        fuzz.contains("name = \"cli_args\"") && fuzz.contains("fuzz_targets/cli_args.rs"),
        "fuzz/Cargo.toml must declare the cli_args bin"
    );
    for f in [
        "fuzz/fuzz_targets/cli_args.rs",
        "fuzz/shared/cli_invariants.rs",
    ] {
        assert!(root().join(f).is_file(), "{f}");
    }
    let target = read("fuzz/fuzz_targets/cli_args.rs");
    assert!(
        target.contains("../../src/bin/exif-audit/args.rs"),
        "the target must fuzz the real parser through #[path]"
    );
    assert!(read(".github/workflows/ci.yml").contains("audit_render, cli_args]"));
    assert!(read("app.just").contains("audit audit_render cli_args\""));
}

#[test]
fn cli_args_seeds_are_committed_and_have_the_shapes_they_claim() {
    let dir = root().join("fuzz/seeds/cli_args");
    let mut names: Vec<String> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    let want: Vec<&str> = CLI_ARGS_SEEDS.iter().map(|s| s.0).collect();
    assert_eq!(names, want, "the cli_args seed set");
    assert!(names.len() >= 16);
    for (name, class) in CLI_ARGS_SEEDS {
        let c = cli_ctx(name);
        match (&c.outcome, class) {
            (CliOutcome::Run(_), "run") => {}
            (CliOutcome::Help, "help") => {}
            (CliOutcome::Version, "version") => {}
            (CliOutcome::Usage(e), cl) if cl.starts_with("usage: ") => {
                let m = e.message();
                assert!(
                    m.starts_with(&cl["usage: ".len()..]),
                    "{name}: message {m:?}, want prefix {cl:?}"
                );
            }
            (o, class) => panic!("{name}: outcome {o:?}, claimed {class}"),
        }
        assert!(cli_invariants::violations(&c).is_empty(), "{name}");
    }
    // The Run seeds say what they mean.
    let run = |n: &str| match cli_ctx(n).outcome {
        CliOutcome::Run(o) => o,
        other => panic!("{n}: {other:?}"),
    };
    use exif_core::audit::Severity;
    use exif_core::report::Format;
    use exif_core::StripPolicy;
    let d = run("cli-defaults");
    assert_eq!(
        (
            d.format,
            d.policy,
            d.fail_on,
            d.strict,
            d.values,
            d.inventory,
            d.max_size
        ),
        (
            Format::Text,
            Some(StripPolicy::AllPii),
            Some(Severity::High),
            false,
            false,
            false,
            256 * 1024 * 1024
        )
    );
    let e = run("cli-every-option");
    assert_eq!(
        (
            e.format,
            e.policy,
            e.fail_on,
            e.strict,
            e.values,
            e.inventory,
            e.max_size
        ),
        (
            Format::Json,
            Some(StripPolicy::Gps),
            Some(Severity::Low),
            true,
            true,
            true,
            512 * 1024
        )
    );
    assert_eq!(e.paths.len(), 2);
    let q = run("cli-equals-forms");
    assert_eq!(
        (q.format, q.policy, q.fail_on, q.max_size),
        (Format::Csv, None, None, 1 << 30)
    );
    let dd = run("cli-double-dash");
    assert!(dd.strict);
    let paths: Vec<&[u8]> = dd.paths.iter().map(|p| p.as_encoded_bytes()).collect();
    assert_eq!(paths, [&b"--json"[..], b"-", b"-h", b"--"]);
    assert_eq!(run("cli-help-after-dash-dash").paths.len(), 2);
    assert_eq!(
        run("cli-size-largest-fit").max_size,
        17_179_869_183 * (1 << 30)
    );
    let empties = run("cli-two-empty-arguments");
    assert!(empties.paths.iter().all(|p| p.is_empty()) && empties.paths.len() == 2);
    // The hostile seed really holds non-UTF-8 bytes.
    let h = cli_ctx("cli-hostile-bytes");
    assert!(h.args.iter().any(|a| std::str::from_utf8(a).is_err()));
    assert!(h.args.iter().any(|a| a.contains(&0x1B)));
    // The long argument is long, and its message is cut.
    let l = cli_ctx("cli-long-unknown-option");
    assert!(l.args[0].len() > 300);
    assert!(l.message.expect("usage").len() <= 300);
}

/// Every committed seed of every target read as an argument list, plus
/// degenerate inputs.
fn cli_harness_inputs() -> Vec<(String, Vec<u8>)> {
    let mut inputs: Vec<(String, Vec<u8>)> = Vec::new();
    for target in [
        "tiff_ifd",
        "jpeg_segments",
        "audit",
        "audit_render",
        "cli_args",
    ] {
        let dir = root().join("fuzz/seeds").join(target);
        for e in fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
            let e = e.expect("entry");
            inputs.push((
                format!("{target}/{}", e.file_name().to_string_lossy()),
                fs::read(e.path()).expect("seed"),
            ));
        }
    }
    for (name, data) in [
        ("empty", &b""[..]),
        ("one NUL", b"\0"),
        ("three NULs", b"\0\0\0"),
        ("trailing NUL", b"p\0"),
        ("leading NUL", b"\0p"),
        ("only dash-dash", b"--"),
        ("dash-dash NUL dash-dash", b"--\0--"),
        ("every byte", &(0..=255u8).collect::<Vec<u8>>()),
    ] {
        inputs.push((name.to_string(), data.to_vec()));
    }
    inputs
}

const CLI_VOCAB: [&[u8]; 30] = [
    b"p",
    b"-",
    b"--",
    b"-h",
    b"--help",
    b"-V",
    b"--version",
    b"--format",
    b"--format=json",
    b"--json",
    b"--csv",
    b"json",
    b"text",
    b"--policy",
    b"--policy=none",
    b"gps",
    b"none",
    b"--fail-on",
    b"high",
    b"--strict",
    b"--values",
    b"--inventory",
    b"--max-size",
    b"--max-size=1K",
    b"0",
    b"1G",
    b"\xFF",
    b"-x",
    b"--strict=1",
    b"",
];

fn cli_sweep(seq: &mut Vec<&[u8]>, depth: usize, n: &mut u64) {
    if !seq.is_empty() {
        cli_invariants::check(&seq.join(&0u8));
        *n += 1;
    }
    if depth == 0 {
        return;
    }
    for t in CLI_VOCAB {
        seq.push(t);
        cli_sweep(seq, depth - 1, n);
        seq.pop();
    }
}

#[test]
fn cli_args_harness_holds_on_every_seed_and_degenerate_inputs() {
    let inputs = cli_harness_inputs();
    for (name, data) in &inputs {
        let bad = cli_invariants::violations(&cli_invariants::build(data));
        assert!(bad.is_empty(), "{name}: {bad:?}");
    }
    // An exhaustive sweep: every sequence of up to three tokens from a
    // vocabulary that covers every option, its values, the separators and the
    // hostile shapes (30^3 argument lists), then a deterministic sample of
    // longer ones.
    let mut n = 0u64;
    cli_sweep(&mut Vec::new(), 3, &mut n);
    let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
    for _ in 0..20_000 {
        let len = 4 + (x % 8) as usize;
        let mut toks: Vec<&[u8]> = Vec::new();
        for _ in 0..len {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            toks.push(CLI_VOCAB[((x >> 33) % 30) as usize]);
        }
        cli_invariants::check(&toks.join(&0u8));
        n += 1;
    }
    println!(
        "{} seeds and degenerate inputs, {n} swept argument lists",
        inputs.len()
    );
}

/// One planted bug: a wrong outcome (or message) put into the case built from
/// the seed, which the named guard, run ALONE, must reject.
struct CliPlant {
    name: &'static str,
    input: &'static str,
    guard: &'static str,
    apply: fn(&mut CliCtx),
}

fn run_of(args: &[&str]) -> CliOutcome {
    match cli_outcome_of(args) {
        o @ CliOutcome::Run(_) => o,
        other => panic!("{args:?}: {other:?}"),
    }
}

fn usage_of(args: &[&str]) -> CliOutcome {
    match cli_outcome_of(args) {
        o @ CliOutcome::Usage(_) => o,
        other => panic!("{args:?}: {other:?}"),
    }
}

fn with_run(c: &mut CliCtx, f: impl FnOnce(&mut cli_args_src::Options)) {
    match &mut c.outcome {
        CliOutcome::Run(o) => f(o),
        other => panic!("the plant needs a Run, got {other:?}"),
    }
}

fn cli_plants() -> Vec<CliPlant> {
    vec![
        // args_total
        CliPlant {
            name: "the second parse is Help",
            input: "cli-defaults",
            guard: "args_total",
            apply: |c| c.again = CliOutcome::Help,
        },
        CliPlant {
            name: "the second parse flips --strict",
            input: "cli-every-option",
            guard: "args_total",
            apply: |c| {
                c.again = c.outcome.clone();
                if let CliOutcome::Run(o) = &mut c.again {
                    o.strict = !o.strict;
                }
            },
        },
        CliPlant {
            name: "the second parse is a usage error",
            input: "cli-defaults",
            guard: "args_total",
            apply: |c| c.again = usage_of(&["--bogus"]),
        },
        // args_help_iff, both directions
        CliPlant {
            name: "Help for arguments with no help argument",
            input: "cli-defaults",
            guard: "args_help_iff",
            apply: |c| c.outcome = CliOutcome::Help,
        },
        CliPlant {
            name: "a usage error for a help argument after an invalid option",
            input: "cli-help-after-invalid",
            guard: "args_help_iff",
            apply: |c| c.outcome = usage_of(&["--bogus"]),
        },
        CliPlant {
            name: "Version for -h next to --version",
            input: "cli-help-and-version",
            guard: "args_help_iff",
            apply: |c| c.outcome = CliOutcome::Version,
        },
        CliPlant {
            name: "Help honoured after `--`",
            input: "cli-help-after-dash-dash",
            guard: "args_help_iff",
            apply: |c| c.outcome = CliOutcome::Help,
        },
        CliPlant {
            name: "Run for -V",
            input: "cli-version",
            guard: "args_help_iff",
            apply: |c| c.outcome = run_of(&["p"]),
        },
        CliPlant {
            name: "Help for -V",
            input: "cli-version",
            guard: "args_help_iff",
            apply: |c| c.outcome = CliOutcome::Help,
        },
        // args_run_shape, both directions and each field
        CliPlant {
            name: "a default changed: --fail-on none",
            input: "cli-defaults",
            guard: "args_run_shape",
            apply: |c| with_run(c, |o| o.fail_on = None),
        },
        CliPlant {
            name: "a default changed: --policy none",
            input: "cli-defaults",
            guard: "args_run_shape",
            apply: |c| with_run(c, |o| o.policy = None),
        },
        CliPlant {
            name: "a default changed: --format json",
            input: "cli-defaults",
            guard: "args_run_shape",
            apply: |c| with_run(c, |o| o.format = exif_core::report::Format::Json),
        },
        CliPlant {
            name: "a default changed: --max-size one lower",
            input: "cli-defaults",
            guard: "args_run_shape",
            apply: |c| with_run(c, |o| o.max_size -= 1),
        },
        CliPlant {
            name: "a flag lost: --values",
            input: "cli-every-option",
            guard: "args_run_shape",
            apply: |c| with_run(c, |o| o.values = false),
        },
        CliPlant {
            name: "paths in the wrong order",
            input: "cli-two-paths",
            guard: "args_run_shape",
            apply: |c| with_run(c, |o| o.paths.reverse()),
        },
        CliPlant {
            name: "the last path dropped after `--`",
            input: "cli-double-dash",
            guard: "args_run_shape",
            apply: |c| {
                with_run(c, |o| {
                    o.paths.pop();
                })
            },
        },
        CliPlant {
            name: "no path at all",
            input: "cli-defaults",
            guard: "args_run_shape",
            apply: |c| with_run(c, |o| o.paths.clear()),
        },
        CliPlant {
            name: "Run for a repeated option",
            input: "cli-repeat-option",
            guard: "args_run_shape",
            apply: |c| c.outcome = run_of(&["p"]),
        },
        CliPlant {
            name: "Run for a lone dash",
            input: "cli-lone-dash",
            guard: "args_run_shape",
            apply: |c| c.outcome = run_of(&["p"]),
        },
        CliPlant {
            name: "a usage error for valid arguments",
            input: "cli-every-option",
            guard: "args_run_shape",
            apply: |c| c.outcome = usage_of(&["--bogus"]),
        },
        CliPlant {
            name: "a usage error for arguments after `--`",
            input: "cli-help-after-dash-dash",
            guard: "args_run_shape",
            apply: |c| c.outcome = usage_of(&["--bogus"]),
        },
        // args_usage_message
        CliPlant {
            name: "an empty message",
            input: "cli-size-zero",
            guard: "args_usage_message",
            apply: |c| c.message = Some(String::new()),
        },
        CliPlant {
            name: "a non-ASCII message",
            input: "cli-size-zero",
            guard: "args_usage_message",
            apply: |c| c.message = Some("bad value for --max-size: caf\u{e9}".into()),
        },
        CliPlant {
            name: "a control character in the message",
            input: "cli-unknown-option",
            guard: "args_usage_message",
            apply: |c| c.message = Some("unknown option: a\nb".into()),
        },
        CliPlant {
            name: "a 301-byte message",
            input: "cli-long-unknown-option",
            guard: "args_usage_message",
            apply: |c| c.message = Some("a".repeat(301)),
        },
        CliPlant {
            name: "an ESC in the message",
            input: "cli-hostile-bytes",
            guard: "args_usage_message",
            apply: |c| c.message = Some("unknown option: \u{1b}[31m".into()),
        },
        CliPlant {
            name: "a message on a Run",
            input: "cli-defaults",
            guard: "args_usage_message",
            apply: |c| c.message = Some("no path given".into()),
        },
        // args_format_conflict
        CliPlant {
            name: "--json with --csv gives Run",
            input: "cli-json-csv-conflict",
            guard: "args_format_conflict",
            apply: |c| c.outcome = run_of(&["--json", "p"]),
        },
        CliPlant {
            name: "--format with --json gives Run",
            input: "cli-format-json-conflict",
            guard: "args_format_conflict",
            apply: |c| c.outcome = run_of(&["--format", "text", "p"]),
        },
        CliPlant {
            name: "--json with --csv gives a Run with the last format",
            input: "cli-json-csv-conflict",
            guard: "args_format_conflict",
            apply: |c| c.outcome = run_of(&["--csv", "p"]),
        },
    ]
}

#[test]
fn each_cli_args_guard_goes_red_alone_when_planted() {
    let plants = cli_plants();
    // Every guard has at least two plants (one per direction where it has two).
    for (name, _) in cli_invariants::GUARDS {
        let n = plants.iter().filter(|p| p.guard == *name).count();
        assert!(n >= 2, "guard {name} has {n} plant(s), want at least two");
    }
    for p in &plants {
        let good = cli_ctx(p.input);
        // CONTROL: the unplanted case satisfies every guard.
        let control = cli_invariants::violations(&good);
        assert!(
            control.is_empty(),
            "{}: control on {}: {control:?}",
            p.name,
            p.input
        );
        // MUTATION: rejected by the named guard ALONE.
        let mut bad = good.clone();
        (p.apply)(&mut bad);
        assert!(
            format!("{bad:?}") != format!("{good:?}"),
            "{}: the plant changed nothing",
            p.name
        );
        let alone = cli_invariants::run_guard(p.guard, &bad);
        assert!(
            alone.is_err(),
            "{}: guard {} did not reject the planted bug on {}",
            p.name,
            p.guard,
            p.input
        );
    }
    println!(
        "{} cli plants, each rejected by its own guard alone",
        plants.len()
    );
}

// ── SPEC-019: the real-world shapes (DEC-012) ────────────────────────────────

/// The 17 `shape-*` seed names (without the prefix), typed by hand.
const SHAPE_SEEDS: [&str; 17] = [
    "mpo-two-le",
    "mpo-two-be",
    "mpo-truncated-index",
    "mpo-offset-out-of-range",
    "mpo-image-not-jpeg",
    "mpo-inner-mpf",
    "mpo-seventeen",
    "mpo-nested-ranges",
    "app3-mpf-signature",
    "split-app13",
    "app10-arot",
    "app2-fpxr",
    "com-segment",
    "trailer-313",
    "canon-style-note",
    "canon-style-note-moved",
    "exif-64k",
];

#[test]
fn new_seeds_are_committed_and_have_the_shapes_they_claim() {
    use exif_core::audit::{audit, Category, RegionKind};
    use exif_core::jpeg::{self, Region};
    let fixtures = real_shapes::fixtures();
    assert_eq!(fixtures.len(), SHAPE_SEEDS.len());
    for ((name, bytes), want) in fixtures.iter().zip(SHAPE_SEEDS) {
        assert_eq!(*name, want, "the builders and the seed names agree");
        // Byte for byte the builder's output, in both JPEG-reading targets.
        for target in ["jpeg_segments", "audit"] {
            let path = root()
                .join("fuzz/seeds")
                .join(target)
                .join(format!("shape-{name}"));
            let on_disk = fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            assert_eq!(&on_disk, bytes, "{target}/shape-{name}");
        }
    }
    let seed = |n: &str| {
        fs::read(
            root()
                .join("fuzz/seeds/jpeg_segments")
                .join(format!("shape-{n}")),
        )
        .unwrap()
    };
    let regions = |n: &str| -> Vec<&'static str> {
        let d = seed(n);
        let m = jpeg::read(&d).unwrap();
        m.regions()
            .iter()
            .map(|r| match r {
                Region::EmbeddedImage { .. } => "embedded",
                Region::Comment { .. } => "comment",
                Region::OtherApp { .. } => "other_app",
                Region::Trailer { .. } => "trailer",
                Region::Iptc { .. } => "iptc",
                _ => "other",
            })
            .collect()
    };
    assert_eq!(regions("mpo-two-le"), ["embedded", "embedded"]);
    assert_eq!(regions("mpo-two-be"), ["embedded", "embedded"]);
    // A truncated index: the MPF segment is an unrecognised APPn.
    assert_eq!(regions("mpo-truncated-index"), ["other_app", "trailer"]);
    // One bad range, one good image; the bytes of the bad one stay a trailer.
    assert_eq!(regions("mpo-offset-out-of-range"), ["embedded", "trailer"]);
    assert_eq!(regions("mpo-image-not-jpeg"), ["embedded", "trailer"]);
    // Only the outer image: the inner MPF belongs to it.
    assert_eq!(regions("mpo-inner-mpf"), ["embedded"]);
    // 17 images: 15 regions and one TooManyMpfImages issue.
    let r17 = regions("mpo-seventeen");
    assert_eq!(r17.iter().filter(|r| **r == "embedded").count(), 15);
    assert_eq!(
        r17.last(),
        Some(&"trailer"),
        "the 16th secondary stays a trailer"
    );
    let d = seed("mpo-seventeen");
    let m = jpeg::read(&d).unwrap();
    assert!(m
        .jpeg_issues()
        .iter()
        .any(|i| matches!(i.error(), Error::TooManyMpfImages { .. })));
    // Nested ranges: no phantom trailer. MPF only on APP2: an unrecognised APPn.
    assert_eq!(regions("mpo-nested-ranges"), ["embedded", "embedded"]);
    assert_eq!(regions("app3-mpf-signature"), ["other_app"]);
    let d = seed("app3-mpf-signature");
    assert!(jpeg::read(&d).unwrap().jpeg_issues().is_empty());
    assert_eq!(regions("split-app13"), ["iptc", "iptc"]);
    assert_eq!(regions("app10-arot"), ["other_app"]);
    assert_eq!(regions("app2-fpxr"), ["other_app"]);
    assert_eq!(regions("com-segment"), ["comment"]);
    assert_eq!(regions("trailer-313"), ["trailer"]);
    assert!(regions("canon-style-note").is_empty());
    assert!(regions("exif-64k").is_empty());
    // The Canon-style note and the 64 KB Exif parse as Exif blocks.
    for n in ["canon-style-note", "canon-style-note-moved", "exif-64k"] {
        let d = seed(n);
        let m = jpeg::read(&d).unwrap();
        assert!(m.exif().is_some(), "{n}");
    }
    assert!(seed("exif-64k").len() > 65_000);
    // And the audit sees what the names say.
    let mpo = seed("mpo-two-le");
    let a = audit(&mpo);
    assert_eq!(a.findings_in(Category::EmbeddedImage).count(), 2);
    let trailer = seed("trailer-313");
    let a = audit(&trailer);
    let kinds: Vec<RegionKind> = a
        .items()
        .iter()
        .filter_map(|i| match i.kind() {
            exif_core::audit::ItemKind::Region { kind, .. } => Some(kind),
            exif_core::audit::ItemKind::Tag { .. } => None,
        })
        .collect();
    assert_eq!(kinds, [RegionKind::Trailer]);
}

#[test]
fn shape_seeds_carry_the_two_dec_012_categories() {
    use exif_core::audit::{audit, Category};
    let mut have: Vec<Category> = Vec::new();
    for n in SHAPE_SEEDS {
        let d = fs::read(root().join("fuzz/seeds/audit").join(format!("shape-{n}"))).unwrap();
        have.extend(audit(&d).findings().iter().map(|f| f.category));
    }
    for c in [
        Category::EmbeddedImage,
        Category::OtherSegment,
        Category::FreeText,
    ] {
        assert!(have.contains(&c), "the shape seeds lack {c:?}");
    }
}

#[test]
fn existing_seeds_and_records_are_byte_identical() {
    // The two older hash lists still hold (63 + 66 seeds); the SPEC-019 seeds
    // only added `shape-*` files. The ExifTool records are checked by
    // `existing_oracle_records_are_byte_identical_to_the_committed_set`.
    the_existing_63_seeds_are_byte_identical();
    the_existing_66_seeds_are_byte_identical();
    // A shape seed is never among the pinned names.
    for (target, name, _, _) in PRE_SPEC_011_SEEDS {
        assert!(!name.starts_with("shape-"), "{target}/{name}");
    }
}

#[test]
fn jpeg_metadata_harness_holds_on_every_seed_and_degenerate_inputs() {
    let dir = root().join("fuzz/seeds/jpeg_segments");
    let mut seen = 0usize;
    for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let path = entry.expect("entry").path();
        let data = fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        metadata_invariants::check(&data);
        seen += 1;
    }
    assert_eq!(seen, 37 + SHAPE_SEEDS.len());
    for data in audit_degenerate_inputs() {
        metadata_invariants::check(&data);
    }
    // Every prefix of the MPO and of the split file, to hit every truncation.
    for n in ["mpo-two-le", "mpo-two-be", "split-app13", "mpo-inner-mpf"] {
        let d = fs::read(dir.join(format!("shape-{n}"))).unwrap();
        for k in 0..d.len() {
            metadata_invariants::check(&d[..k]);
        }
    }
}

#[test]
fn each_new_region_guard_goes_red_alone_when_planted() {
    use exif_core::audit::audit;
    let plants = new_region_plants();
    // Each new-region guard has at least two plants (an iff or a two-sided
    // bound is not tested by one).
    for guard in [
        "embedded",
        "category_shape",
        "inventory",
        "health",
        "findings",
    ] {
        let n = plants.iter().filter(|p| p.guard == guard).count();
        assert!(n >= 2, "guard {guard} has {n} new-region plant(s)");
    }
    for p in &plants {
        let data = plant_input(p.input);
        let got = audit_invariants::facts(&audit(&data));
        let want = audit_invariants::rederive(&data);
        // CONTROL: the unplanted answer satisfies every guard.
        let control = audit_invariants::violations(&data, &got, &want);
        assert!(control.is_empty(), "{}: control: {control:?}", p.name);
        let mut bad = got.clone();
        (p.apply)(&mut bad);
        assert_ne!(bad, got, "{}: the plant changed nothing", p.name);
        let alone = audit_invariants::run_guard(p.guard, &data, &bad, &want);
        assert!(
            alone.is_err(),
            "{}: guard {} did not reject the planted bug on {}",
            p.name,
            p.guard,
            p.input
        );
    }
}

// ── SPEC-020: the `rewrite_jpeg` target (the JPEG strip writer, DEC-013) ─────

include!("../fuzz/shared/rewrite_invariants.rs");

fn rewrite_seed_files() -> Vec<(String, Vec<u8>)> {
    let dir = root().join("fuzz/seeds/rewrite_jpeg");
    let mut v: Vec<(String, Vec<u8>)> = fs::read_dir(&dir)
        .expect("read rewrite_jpeg seeds")
        .map(|e| {
            let p = e.unwrap().path();
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                fs::read(&p).unwrap(),
            )
        })
        .collect();
    v.sort();
    v
}

/// Degenerate inputs the policy byte can be put in front of.
fn rewrite_degenerate() -> Vec<Vec<u8>> {
    let seg = |m: u8, p: &[u8]| {
        let mut v = vec![0xFF, m];
        v.extend_from_slice(&((p.len() + 2) as u16).to_be_bytes());
        v.extend_from_slice(p);
        v
    };
    let mut many = vec![0xFF, 0xD8];
    for _ in 0..4100 {
        many.extend_from_slice(&seg(0xFE, b"c"));
    }
    let mut stray = vec![0xFF, 0xD8];
    for _ in 0..1100 {
        stray.extend_from_slice(&[0xFF, 0x00]);
    }
    stray.extend_from_slice(&seg(0xFE, b"after"));
    vec![
        vec![],
        vec![0xFF],
        vec![0xFF, 0xD8],
        vec![0xFF, 0xD8, 0xFF],
        vec![0xFF, 0xD8, 0xFF, 0xFE, 0x00, 0x01],
        vec![0xFF, 0xD8, 0xFF, 0xFE, 0xFF, 0xFF, 1, 2, 3],
        b"II*\0\x08\0\0\0\0\0\0\0\0\0".to_vec(),
        b"\x89PNG\r\n\x1a\n".to_vec(),
        many,
        stray,
        [vec![0xFF, 0xD8], seg(0xE2, b"MPF\0"), vec![0xFF, 0xD9]].concat(),
        [
            vec![0xFF, 0xD8],
            seg(0xE1, b"http://ns.adobe.com/xap/1.0/\0x"),
            seg(0xFE, b"c"),
            vec![0xFF, 0xDA, 0, 2, 1, 2, 3],
        ]
        .concat(),
    ]
}

#[test]
fn rewrite_jpeg_harness_holds_on_every_seed_and_degenerate_inputs() {
    let seeds = rewrite_seed_files();
    assert!(seeds.len() >= 60, "{} seeds", seeds.len());
    for (name, data) in &seeds {
        // The seed as committed, and with every policy byte in front of it.
        rewrite_invariants::check(data);
        let (_, jpeg) = rewrite_invariants::decode(data);
        for head in 0u8..=8 {
            let mut v = vec![head, 0x08, 0x40];
            v.extend_from_slice(jpeg);
            rewrite_invariants::check(&v);
        }
        let _ = name;
    }
    for d in rewrite_degenerate() {
        for head in 0u8..=8 {
            let mut v = vec![head, 0xFF, 0x7F];
            v.extend_from_slice(&d);
            rewrite_invariants::check(&v);
        }
        // Every custom set of the 15 single categories.
        for bit in 0..15u16 {
            let m = (1u16 << bit).to_le_bytes();
            let mut v = vec![6, m[0], m[1]];
            v.extend_from_slice(&d);
            rewrite_invariants::check(&v);
        }
    }
    rewrite_invariants::check(&[]);
    rewrite_invariants::check(&[6]);
}

/// A plant: a wrong answer put into the library's flattened answer for `input`
/// under the policy byte `head`, which the named guard (and only the guards in
/// `also`, if any) must reject.
struct RewritePlant {
    name: &'static str,
    input: &'static str,
    head: &'static [u8],
    guard: rewrite_invariants::Guard,
    apply: fn(&mut rewrite_invariants::Facts),
}

fn rewrite_plant_input(name: &str) -> Vec<u8> {
    match name {
        "kitchen" => strip_shapes::kitchen_sink().bytes,
        "mpo-late" => strip_shapes::mpo_late_xmp(false).bytes,
        "mpo-gaps" => strip_shapes::mpo_trailer_gaps().bytes,
        "no-exif" => strip_shapes::no_exif().bytes,
        "needs-edit" => strip_shapes::kitchen_sink().bytes,
        "overlap" => strip_shapes::com_over_embedded().bytes,
        "unindexed" => strip_shapes::unindexed_mpf().bytes,
        "bad-length" => strip_shapes::bad_length().bytes,
        "empty-plan" => strip_shapes::comment_only().bytes,
        other => panic!("no plant input {other}"),
    }
}

fn ok_mut(f: &mut rewrite_invariants::Facts) -> &mut rewrite_invariants::OkFacts {
    f.result.as_mut().expect("an Ok answer")
}

fn out_mut(f: &mut rewrite_invariants::Facts) -> &mut rewrite_invariants::OutFacts {
    f.out.as_mut().expect("output facts")
}

#[test]
fn each_rewrite_guard_goes_red_alone_when_planted() {
    use rewrite_invariants::{ErrFact, Guard};
    let plants: Vec<RewritePlant> = vec![
        // Prediction: the output bytes, in both directions (a wrong byte, a
        // missing byte).
        RewritePlant {
            name: "a byte of the output is changed",
            input: "kitchen",
            head: &[7],
            guard: Guard::Prediction,
            apply: |f| {
                let b = &mut ok_mut(f).bytes;
                let n = b.len() / 2;
                b[n] ^= 0x01;
            },
        },
        RewritePlant {
            name: "the output is one byte short",
            input: "mpo-late",
            head: &[7],
            guard: Guard::Prediction,
            apply: |f| {
                ok_mut(f).bytes.pop();
            },
        },
        RewritePlant {
            name: "an Ok answer where a refusal is predicted",
            input: "overlap",
            head: &[7],
            guard: Guard::Prediction,
            apply: |f| {
                f.result = Ok(rewrite_invariants::OkFacts {
                    bytes: vec![0xFF, 0xD8],
                    removals: vec![],
                    patches: vec![],
                });
            },
        },
        // Records.
        RewritePlant {
            name: "a removal record has an output offset",
            input: "kitchen",
            head: &[7],
            guard: Guard::Records,
            apply: |f| {
                let r = ok_mut(f).removals.last_mut().expect("a removal");
                r.offset = r.offset.saturating_sub(5);
            },
        },
        RewritePlant {
            name: "a removal record is missing",
            input: "kitchen",
            head: &[7],
            guard: Guard::Records,
            apply: |f| {
                ok_mut(f).removals.remove(0);
            },
        },
        RewritePlant {
            name: "a patch record is missing",
            input: "mpo-late",
            head: &[7],
            guard: Guard::Records,
            apply: |f| {
                ok_mut(f).patches.pop();
            },
        },
        RewritePlant {
            name: "a removal record has the wrong category",
            input: "kitchen",
            head: &[7],
            guard: Guard::Records,
            apply: |f| {
                ok_mut(f).removals[0].category = exif_core::audit::Category::Icc;
            },
        },
        // Length, both directions of the iff.
        RewritePlant {
            name: "the output grows",
            input: "kitchen",
            head: &[7],
            guard: Guard::Length,
            apply: |f| {
                // Longer than the input.
                let input_len = strip_shapes::kitchen_sink().bytes.len();
                ok_mut(f).bytes.resize(input_len + 1, 0);
            },
        },
        RewritePlant {
            name: "equal length with removals recorded",
            input: "kitchen",
            head: &[7],
            guard: Guard::Length,
            apply: |f| {
                let input_len = strip_shapes::kitchen_sink().bytes.len();
                ok_mut(f).bytes.resize(input_len, 0);
            },
        },
        RewritePlant {
            name: "shorter with nothing recorded as removed",
            input: "empty-plan",
            head: &[7],
            guard: Guard::Length,
            apply: |f| {
                // `All` on a comment-only file removes the comment: clear the
                // records but keep the shorter output.
                ok_mut(f).removals.clear();
            },
        },
        // Scan.
        RewritePlant {
            name: "the scan region differs",
            input: "kitchen",
            head: &[7],
            guard: Guard::Scan,
            apply: |f| out_mut(f).scan_same = false,
        },
        RewritePlant {
            name: "the scan region differs on a Multi-Picture file",
            input: "mpo-late",
            head: &[4],
            guard: Guard::Scan,
            apply: |f| out_mut(f).scan_same = false,
        },
        // Removed categories left in the output.
        RewritePlant {
            name: "an Xmp finding is left in",
            input: "kitchen",
            head: &[7],
            guard: Guard::RemovedCategories,
            apply: |f| out_mut(f).categories.push(exif_core::audit::Category::Xmp),
        },
        RewritePlant {
            name: "an Icc finding is left in under Everything",
            input: "no-exif",
            head: &[4],
            guard: Guard::RemovedCategories,
            apply: |f| out_mut(f).categories.push(exif_core::audit::Category::Icc),
        },
        // Kept findings, both directions.
        RewritePlant {
            name: "a kept finding is lost",
            input: "kitchen",
            head: &[7],
            guard: Guard::KeptFindings,
            apply: |f| {
                out_mut(f).kept_out.0.pop();
            },
        },
        RewritePlant {
            name: "a kept finding appears from nowhere",
            input: "kitchen",
            head: &[7],
            guard: Guard::KeptFindings,
            apply: |f| out_mut(f).kept_out.0.push("Icc/Phantom".to_string()),
        },
        RewritePlant {
            name: "the kept trailer bytes change",
            input: "mpo-gaps",
            head: &[7],
            guard: Guard::KeptFindings,
            apply: |f| out_mut(f).kept_out.1 += 1,
        },
        // Idempotence.
        RewritePlant {
            name: "stripping the output again changes it",
            input: "kitchen",
            head: &[4],
            guard: Guard::Idempotent,
            apply: |f| out_mut(f).idempotent = false,
        },
        RewritePlant {
            name: "stripping the output again changes a Multi-Picture file",
            input: "mpo-gaps",
            head: &[6, 0x00, 0x40],
            guard: Guard::Idempotent,
            apply: |f| out_mut(f).idempotent = false,
        },
        // Determinism.
        RewritePlant {
            name: "two calls disagree",
            input: "kitchen",
            head: &[7],
            guard: Guard::Deterministic,
            apply: |f| f.deterministic = false,
        },
        RewritePlant {
            name: "two calls disagree on a refusal",
            input: "overlap",
            head: &[7],
            guard: Guard::Deterministic,
            apply: |f| f.deterministic = false,
        },
        // Refusal classes, in both directions.
        RewritePlant {
            name: "NeedsEntryEdit names too many categories",
            input: "needs-edit",
            head: &[2],
            guard: Guard::ErrClass,
            apply: |f| {
                f.result = Err(ErrFact::NeedsEntryEdit(
                    exif_core::audit::Category::ALL.to_vec(),
                ))
            },
        },
        RewritePlant {
            name: "an Overlap is reported at the wrong offset",
            input: "overlap",
            head: &[6, 0x08, 0x00],
            guard: Guard::ErrClass,
            apply: |f| {
                if let Err(ErrFact::Overlap(o)) = &mut f.result {
                    *o += 1;
                }
            },
        },
        RewritePlant {
            name: "MpfNotIndexed is reported for a file that is fine",
            input: "no-exif",
            head: &[7],
            guard: Guard::ErrClass,
            apply: |f| f.result = Err(ErrFact::MpfNotIndexed(0)),
        },
        RewritePlant {
            name: "a bad length is reported as Overlap",
            input: "bad-length",
            head: &[7],
            guard: Guard::ErrClass,
            apply: |f| f.result = Err(ErrFact::Overlap(4)),
        },
        RewritePlant {
            name: "an unindexed MPF is reported as fine-with-a-different-offset",
            input: "unindexed",
            head: &[7],
            guard: Guard::ErrClass,
            apply: |f| {
                if let Err(ErrFact::MpfNotIndexed(o)) = &mut f.result {
                    *o += 3;
                }
            },
        },
        // VerifyFailed where the prediction is fine.
        RewritePlant {
            name: "VerifyFailed on a clean file",
            input: "kitchen",
            head: &[7],
            guard: Guard::VerifyAllowed,
            apply: |f| f.result = Err(ErrFact::VerifyFailed("preview")),
        },
        RewritePlant {
            name: "VerifyFailed on a refusal",
            input: "needs-edit",
            head: &[2],
            guard: Guard::VerifyAllowed,
            apply: |f| f.result = Err(ErrFact::VerifyFailed("kept")),
        },
    ];

    // Every guard has at least two plants, and the iff guards have plants in
    // both directions (their names say which).
    for g in rewrite_invariants::GUARDS {
        let n = plants.iter().filter(|p| p.guard == g).count();
        assert!(n >= 2, "{g:?} has {n} plants");
    }

    for p in &plants {
        let input = rewrite_plant_input(p.input);
        let mut seed = p.head.to_vec();
        seed.extend_from_slice(&input);
        let (policy, jpeg) = rewrite_invariants::decode(&seed);
        let spec = policy.spec();
        let pred = rewrite_invariants::predict(jpeg, spec);
        let good = rewrite_invariants::facts(jpeg, policy);
        // CONTROL: the unplanted answer satisfies every guard.
        let control = rewrite_invariants::failures(jpeg, spec, &good, &pred);
        assert!(control.is_empty(), "{}: control: {control:?}", p.name);
        let mut bad = good.clone();
        (p.apply)(&mut bad);
        assert_ne!(bad, good, "{}: the plant changed nothing", p.name);
        // The guard rejects the plant, run alone.
        assert!(
            !rewrite_invariants::holds(p.guard, jpeg, spec, &bad, &pred),
            "{}: guard {:?} did not reject the planted bug",
            p.name,
            p.guard
        );
        // And the other guards that read other parts of the facts still hold
        // where the plant did not touch their part: a guard is not red merely
        // because another is.
        let others = rewrite_invariants::failures(jpeg, spec, &bad, &pred);
        assert!(others.contains(&p.guard), "{}: {others:?}", p.name);
    }
}

#[test]
fn rewrite_guards_are_not_constant() {
    // Each guard accepts at least one real answer and rejects at least one
    // planted one: a constant-true or constant-false guard fails one of them.
    use rewrite_invariants::Guard;
    let input = strip_shapes::kitchen_sink().bytes;
    let policy = exif_core::StripPolicy::All;
    let spec = policy.spec();
    let pred = rewrite_invariants::predict(&input, spec);
    let good = rewrite_invariants::facts(&input, policy);
    for g in rewrite_invariants::GUARDS {
        assert!(
            rewrite_invariants::holds(g, &input, spec, &good, &pred),
            "{g:?} rejects the real answer"
        );
    }
    // The naive stripper agrees with the library on the Ok answer.
    match pred {
        rewrite_invariants::Pred::Ok { bytes, .. } => {
            assert_eq!(good.result.as_ref().unwrap().bytes, bytes)
        }
        other => panic!("{other:?}"),
    }
    let _ = Guard::Prediction;
}

#[test]
fn rewrite_seeds_are_committed_and_cover_every_fixture_and_policy_class() {
    let seeds = rewrite_seed_files();
    assert_eq!(seeds.len(), 186, "the rewrite_jpeg seed count is pinned");
    let names: Vec<&str> = seeds.iter().map(|(n, _)| n.as_str()).collect();
    // Every SPEC-019 shape and every strip fixture, under All, Everything and a
    // custom mask.
    for (n, _) in real_shapes::fixtures() {
        for prefix in ["all-", "allpreset-", "everything-", "mask"] {
            assert!(
                names
                    .iter()
                    .any(|s| s.starts_with(prefix) && s.ends_with(&format!("shape-{n}"))),
                "{prefix} shape-{n}"
            );
        }
    }
    for s in strip_shapes::all() {
        for prefix in ["all-", "allpreset-", "everything-", "mask"] {
            assert!(
                names
                    .iter()
                    .any(|x| x.starts_with(prefix) && x.ends_with(&format!("strip-{}", s.name))),
                "{prefix} strip-{}",
                s.name
            );
        }
    }
    // Policy classes: the leading byte covers presets, the empty set, a mask
    // and the default; every mask value used is one of the six pinned ones.
    let heads: std::collections::BTreeSet<u8> = seeds
        .iter()
        .filter_map(|(_, d)| d.first().copied())
        .collect();
    for h in [0u8, 1, 2, 3, 4, 5, 6, 7] {
        assert!(heads.contains(&h), "no seed starts with policy byte {h}");
    }
    // The narrow presets cover the named classes.
    for class in [
        "mpo-plain",
        "two-exif-blocks",
        "exif-without-gps",
        "com-over-embedded",
        "unindexed-mpf",
        "gps-count-overflow",
    ] {
        for prefix in ["gps-strip-", "gps_and_device-strip-"] {
            assert!(
                names.contains(&format!("{prefix}{class}").as_str()),
                "{prefix}{class}"
            );
        }
    }
    // Every shape of the audit's refusal classes is in there: a bad length, a
    // halted walk, an overlap, an unindexed MPF, a needs-entry-edit file.
    let expected_classes = [
        ("strip-bad-length", "Unstrippable"),
        ("strip-halted-walk", "Unstrippable"),
        ("strip-com-over-embedded", "Overlap"),
        ("strip-unindexed-mpf", "MpfNotIndexed"),
        ("strip-kitchen-sink", "NeedsEntryEdit"),
    ];
    let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for (fixture, class) in expected_classes {
        for (n, d) in &seeds {
            if !n.ends_with(fixture) {
                continue;
            }
            let (policy, jpeg) = rewrite_invariants::decode(d);
            let got = match exif_core::strip(jpeg, policy) {
                Err(exif_core::StripError::Unstrippable { .. }) => "Unstrippable",
                Err(exif_core::StripError::Overlap { .. }) => "Overlap",
                Err(exif_core::StripError::MpfNotIndexed { .. }) => "MpfNotIndexed",
                Err(exif_core::StripError::NeedsEntryEdit { .. }) => "NeedsEntryEdit",
                _ => "",
            };
            if got == class {
                seen.insert(class);
            }
        }
    }
    assert_eq!(seen.len(), 4, "refusal classes seeded: {seen:?}");
    // An Ok seed with patches exists.
    assert!(seeds.iter().any(|(_, d)| {
        let (p, j) = rewrite_invariants::decode(d);
        exif_core::strip(j, p).is_ok_and(|s| !s.patches().is_empty())
    }));
}

#[test]
fn ci_fuzz_smoke_list_includes_rewrite_jpeg() {
    let fuzz = read("fuzz/Cargo.toml");
    assert!(
        fuzz.contains("name = \"rewrite_jpeg\"") && fuzz.contains("fuzz_targets/rewrite_jpeg.rs"),
        "fuzz/Cargo.toml must declare the rewrite_jpeg bin"
    );
    for f in [
        "fuzz/fuzz_targets/rewrite_jpeg.rs",
        "fuzz/shared/rewrite_invariants.rs",
    ] {
        assert!(root().join(f).is_file(), "{f}");
    }
    let target = read("fuzz/fuzz_targets/rewrite_jpeg.rs");
    assert!(target.contains("rewrite_invariants.rs"));
    assert!(
        !target.contains("write::"),
        "the harness must not reach into src/write"
    );
    assert!(
        !read("fuzz/shared/rewrite_invariants.rs").contains("exif_core::write::plan")
            && !read("fuzz/shared/rewrite_invariants.rs").contains("EditPlan"),
        "the naive stripper must not use the library's plan"
    );
    let ci = read(".github/workflows/ci.yml");
    assert!(
        ci.contains("rewrite_jpeg"),
        "the fuzz-smoke matrix must run the rewrite_jpeg target"
    );
    assert!(read("app.just").contains("rewrite_jpeg"));
}
