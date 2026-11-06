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
const JPEG_SEEDS: [&str; 34] = [
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
        0xE1 if payload.starts_with(b"Exif\0\0") => SegmentKind::Exif,
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
            m == 0xE1 && payload.starts_with(b"Exif\0\0")
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
