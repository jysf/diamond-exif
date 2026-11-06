//! Layout and seed checks for the `tiff_ifd` fuzz target (SPEC-006, DEC-004).
//! std only. The target itself needs nightly; these tests run on stable and
//! catch the regressions that do not need a fuzzer: a fuzz crate that leaked
//! into the library graph, a `.gitignore` that swallows the seeds, a seed that
//! went missing, or a seed that makes the parser panic.

use std::fs;
use std::path::PathBuf;

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
    assert_eq!(count("jpeg_segments"), 37);
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
    assert_eq!(m.regions()[0].data(), &[1u8, 2, 3, 4, 5][..]);

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

use audit_invariants::{Facts, FindingF, ItemF, ThumbF, What, CAP};

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
    assert_eq!(count("jpeg_segments"), 37);
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
        ci.contains("target: [tiff_ifd, jpeg_segments, audit]"),
        "the fuzz-smoke matrix must run the audit target"
    );
}

fn audit_seed(name: &str) -> Vec<u8> {
    let path = root().join("fuzz/seeds/audit").join(name);
    fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

const AUDIT_NEW_SEEDS: [&str; 7] = [
    "audit-png-magic",
    "audit-bigtiff-magic",
    "audit-empty-marker",
    "audit-exif-gps-and-thumbnail",
    "audit-two-blocks-same-tag",
    "audit-ifd1-strips",
    "audit-many-overlapping-ifds",
];

#[test]
fn audit_seeds_are_committed_and_have_the_shapes_they_claim() {
    use exif_core::audit::{audit, Category};

    // The 66 older seeds are copied byte for byte under a prefix, and the
    // directory holds exactly those 66 plus the seven new shapes.
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
    assert_eq!(seen, 76, "32 TIFF + 37 JPEG + 7 new");
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

fn plants() -> Vec<Plant> {
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
    assert_eq!(seen, 32 + 76);
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
