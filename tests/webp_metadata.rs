//! SPEC-030 / DEC-019: WebP metadata extraction against the independent
//! builders in `tests/support/webp_shapes.rs`. Expectations come from the
//! builders (offsets are computed from chunk sizes), never from the library.

#[path = "support/webp_shapes.rs"]
mod webp_shapes;

use exif_core::tiff::Container;
use exif_core::webp::{read, AnyWebpIssue, Webp, WebpMetadata, WebpRegion, MAX_WEBP_EXIF_BLOCKS};
use exif_core::Error;
use webp_shapes::*;

fn inside(outer: &[u8], inner: &[u8]) -> bool {
    let o = outer.as_ptr() as usize;
    let i = inner.as_ptr() as usize;
    i >= o && i + inner.len() <= o + outer.len()
}

fn errors(m: &WebpMetadata<'_>) -> Vec<Error> {
    m.webp_issues().iter().map(|i| i.error().clone()).collect()
}

/// A VP8X with the EXIF flag, then `parts`. The VP8X chunk is 18 bytes, so the
/// first part starts at offset 30.
fn wrap(parts: &[Vec<u8>]) -> Vec<u8> {
    let mut all = vec![vp8x(FLAG_EXIF, 1, 1)];
    all.extend(parts.iter().cloned());
    riff(&all)
}

const FIRST: usize = 30;

#[test]
fn fatal_exactly_when_the_walk_is() {
    for d in [
        &b""[..],
        b"RIFF",
        b"RIFF\0\0\0\0AVI x",
        b"not a webp at all",
    ] {
        assert_eq!(read(d).err(), Webp::parse(d).err());
        assert!(read(d).is_err());
    }
    assert!(read(&minimal()).is_ok());
    assert!(read(&header(4)).is_ok());
}

#[test]
fn exif_block_equals_bare_tiff() {
    let payload = exif_with_make();
    let d = wrap(&[exif_chunk(&payload)]);
    let m = read(&d).expect("reads");
    assert!(m.is_clean(), "{:?}", m.webp_issues());
    let [b] = m.exif_blocks() else {
        panic!("one block")
    };
    assert_eq!((b.offset(), b.tiff_at()), (FIRST, FIRST + 8));
    assert_eq!(b.tiff(), &payload[..]);
    let bare = Container::parse(&payload).expect("bare parses");
    let got = b.container().expect("block parses");
    assert_eq!(got.ifds().len(), bare.ifds().len());
    assert_eq!(got.entry_count(), bare.entry_count());
    assert_eq!(got.issues().len(), bare.issues().len());
    assert_eq!(got.ifd0_offset(), bare.ifd0_offset());
    // tiff_at plus an IFD offset indexes the same byte in the input.
    let at = b.tiff_at() + usize::try_from(got.ifd0_offset()).expect("fits");
    assert_eq!(d[at..at + 2], payload[8..10]);
    assert!(m.exif().is_some());
    assert_eq!(m.webp().chunks().len(), 2);

    // A fatal error equals the bare error, and is data rather than an issue.
    let d = wrap(&[exif_chunk(b"not a tiff")]);
    let m = read(&d).expect("reads");
    let want = Container::parse(b"not a tiff").expect_err("bare fails");
    assert_eq!(m.exif_blocks()[0].container().err(), Some(&want));
    assert!(m.webp_issues().is_empty());
    assert!(!m.is_clean());
}

#[test]
fn exif_prefix_issue() {
    let payload = exif_with_make();
    let d = wrap(&[exif_chunk(&with_exif_prefix(&payload))]);
    let m = read(&d).expect("reads");
    assert_eq!(errors(&m), [Error::ImproperExifHeader { at: FIRST }]);
    let b = &m.exif_blocks()[0];
    assert_eq!(b.tiff(), &payload[..]);
    assert_eq!(b.tiff_at(), FIRST + 8 + 6);
    assert_eq!(d[b.tiff_at()..b.tiff_at() + 2], *b"MM");
    assert!(b.container().is_ok());
    assert!(!m.is_clean());
}

#[test]
fn exif_empty_and_garbage_stay_err() {
    for payload in [&b""[..], b"x", b"not a tiff", b"Exif\0\0", b"II\x2A"] {
        let d = wrap(&[exif_chunk(payload)]);
        let m = read(&d).expect("reads");
        assert_eq!(m.exif_blocks().len(), 1);
        assert!(m.exif_blocks()[0].container().is_err(), "{payload:?}");
        assert!(m.exif().is_none());
        assert!(!m.is_clean());
    }
}

#[test]
fn second_exif_is_extra_but_parsed() {
    let first = exif_chunk(&tiff_empty());
    let second_at = FIRST + first.len();
    let d = wrap(&[first, exif_chunk(&exif_with_make())]);
    let m = read(&d).expect("reads");
    assert_eq!(m.exif_blocks().len(), 2);
    assert!(m.exif_blocks().iter().all(|b| b.container().is_ok()));
    assert_eq!(m.exif_blocks()[1].offset(), second_at);
    assert_eq!(errors(&m), [Error::ExtraExif { at: second_at }]);
    // The first block is primary.
    assert_eq!(m.exif().expect("primary").entry_count(), 0);
}

#[test]
fn exif_blocks_capped_at_sixteen() {
    let one = exif_chunk(&tiff_empty());
    let parts: Vec<Vec<u8>> = (0..MAX_WEBP_EXIF_BLOCKS + 2).map(|_| one.clone()).collect();
    let d = wrap(&parts);
    let m = read(&d).expect("reads");
    assert_eq!(m.exif_blocks().len(), MAX_WEBP_EXIF_BLOCKS);
    // Every chunk after the first costs an ExtraExif, parsed or not.
    let extras = errors(&m)
        .iter()
        .filter(|e| matches!(e, Error::ExtraExif { .. }))
        .count();
    assert_eq!(extras, MAX_WEBP_EXIF_BLOCKS + 1);
    // Exactly sixteen chunks make sixteen blocks, no more issues than that.
    let d = wrap(&parts[..MAX_WEBP_EXIF_BLOCKS]);
    let m = read(&d).expect("reads");
    assert_eq!(m.exif_blocks().len(), MAX_WEBP_EXIF_BLOCKS);
    assert_eq!(errors(&m).len(), MAX_WEBP_EXIF_BLOCKS - 1);
}

#[test]
fn xmp_and_iccp_located_byte_for_byte() {
    // An odd XMP payload (padded) and an ICC profile, with the image chunk
    // between them.
    let xmp = b"<x:xmpmeta>1</x:xmpmeta>!";
    assert_eq!(xmp.len() % 2, 1);
    let icc = b"profile bytes";
    let d = riff(&[
        vp8x(FLAG_ICC | FLAG_XMP, 4, 4),
        chunk(b"ICCP", icc),
        vp8l(),
        chunk(b"XMP ", xmp),
    ]);
    let m = read(&d).expect("reads");
    assert!(m.is_clean(), "{:?}", m.webp_issues());
    let icc_at = 30;
    let xmp_at = icc_at + 8 + icc.len() + 1 + 8 + 5 + 1;
    let want = [(icc_at, &icc[..], true), (xmp_at, &xmp[..], false)];
    assert_eq!(m.regions().len(), 2);
    for (r, (at, data, is_icc)) in m.regions().iter().zip(want) {
        assert_eq!(r.offset(), at);
        assert_eq!(r.data(), data);
        assert!(inside(&d, r.data()));
        // The data starts right after the chunk's 8 header bytes.
        assert_eq!(r.data().as_ptr() as usize - d.as_ptr() as usize, at + 8);
        match *r {
            WebpRegion::Icc { offset, data: got } => {
                assert!(is_icc);
                assert_eq!((offset, got), (at, data));
            }
            WebpRegion::Xmp { offset, data: got } => {
                assert!(!is_icc);
                assert_eq!((offset, got), (at, data));
            }
            _ => panic!("unknown region variant"),
        }
    }
    // Empty chunks are located too: an empty XMP is a region with no data.
    let d = riff(&[vp8x(FLAG_XMP, 1, 1), chunk(b"XMP ", &[])]);
    let m = read(&d).expect("reads");
    assert_eq!(m.regions().len(), 1);
    assert!(m.regions()[0].data().is_empty());
    assert!(inside(&d, m.regions()[0].data()));
}

#[test]
fn two_xmp_both_located() {
    let d = riff(&[
        vp8x(FLAG_XMP, 1, 1),
        chunk(b"XMP ", b"<a/>"),
        chunk(b"XMP ", b"<b/>"),
    ]);
    let m = read(&d).expect("reads");
    assert!(m.is_clean(), "{:?}", m.webp_issues());
    let got: Vec<&[u8]> = m.regions().iter().map(|r| r.data()).collect();
    assert_eq!(got, [&b"<a/>"[..], b"<b/>"]);
    assert_eq!(
        m.regions().iter().map(|r| r.offset()).collect::<Vec<_>>(),
        [30, 42]
    );
    // `XMP\0` is located like XMP, with its one walk issue.
    let d = riff(&[vp8x(FLAG_XMP, 1, 1), chunk(b"XMP\0", b"<x/>")]);
    let m = read(&d).expect("reads");
    assert_eq!(m.regions().len(), 1);
    assert_eq!(
        errors(&m),
        [Error::NonCanonicalChunkId {
            chunk_id: *b"XMP\0",
            at: 30
        }]
    );
}

#[test]
fn anmf_payload_metadata_is_not_found() {
    let make = exif_with_make();
    // Metadata inside the frame payload is not found; the top-level EXIF after
    // the frame is.
    let d = riff(&[
        vp8x(FLAG_ANIM | FLAG_EXIF, 2, 2),
        chunk(
            b"ANMF",
            &[exif_chunk(&make), chunk(b"XMP ", b"<x/>")].concat(),
        ),
    ]);
    let m = read(&d).expect("reads");
    assert!(m.exif_blocks().is_empty());
    assert!(m.regions().is_empty());
    let d = riff(&[
        vp8x(FLAG_ANIM | FLAG_EXIF, 2, 2),
        chunk(b"ANMF", &exif_chunk(&make)),
        exif_chunk(&make),
    ]);
    let m = read(&d).expect("reads");
    assert_eq!(m.exif_blocks().len(), 1);
    assert_eq!(
        m.exif_blocks()[0].offset(),
        30 + 8 + exif_chunk(&make).len()
    );
}

/// (name, parts, EXIF blocks, regions, issues)
type Case = (&'static str, Vec<Vec<u8>>, usize, usize, Vec<Error>);

#[test]
fn flags_are_informational() {
    let make = exif_with_make();
    let xmp = chunk(b"XMP ", b"<x/>");
    let icc = chunk(b"ICCP", b"icc!");
    let exif = exif_chunk(&make);

    // (name, parts, EXIF blocks, regions, issues)
    let cases: Vec<Case> = vec![
        (
            "exif, flag set",
            vec![vp8x(FLAG_EXIF, 1, 1), exif.clone()],
            1,
            0,
            vec![],
        ),
        (
            "exif, flag clear",
            vec![vp8x(0, 1, 1), exif.clone()],
            1,
            0,
            vec![Error::ChunkFlagMismatch {
                flag: "EXIF",
                declared: false,
            }],
        ),
        (
            "exif, no VP8X",
            vec![vp8l(), exif.clone()],
            1,
            0,
            vec![Error::MetadataWithoutVp8x],
        ),
        (
            "exif before VP8X",
            vec![exif.clone(), vp8x(FLAG_EXIF, 1, 1)],
            1,
            0,
            vec![Error::MisplacedVp8x {
                at: 12 + exif.len(),
            }],
        ),
        (
            "exif before the image chunk",
            vec![vp8x(FLAG_EXIF, 1, 1), exif.clone(), vp8l()],
            1,
            0,
            vec![],
        ),
        (
            "exif flag, no chunk",
            vec![vp8x(FLAG_EXIF, 1, 1), vp8l()],
            0,
            0,
            vec![Error::ChunkFlagMismatch {
                flag: "EXIF",
                declared: true,
            }],
        ),
        (
            "xmp, flag set",
            vec![vp8x(FLAG_XMP, 1, 1), xmp.clone()],
            0,
            1,
            vec![],
        ),
        (
            "xmp, flag clear",
            vec![vp8x(0, 1, 1), xmp.clone()],
            0,
            1,
            vec![Error::ChunkFlagMismatch {
                flag: "XMP",
                declared: false,
            }],
        ),
        (
            "xmp, no VP8X",
            vec![vp8l(), xmp.clone()],
            0,
            1,
            vec![Error::MetadataWithoutVp8x],
        ),
        (
            "xmp flag, no chunk",
            vec![vp8x(FLAG_XMP, 1, 1), vp8l()],
            0,
            0,
            vec![Error::ChunkFlagMismatch {
                flag: "XMP",
                declared: true,
            }],
        ),
        (
            "icc, flag set",
            vec![vp8x(FLAG_ICC, 1, 1), icc.clone(), vp8l()],
            0,
            1,
            vec![],
        ),
        (
            "icc, flag clear",
            vec![vp8x(0, 1, 1), icc.clone(), vp8l()],
            0,
            1,
            vec![Error::ChunkFlagMismatch {
                flag: "ICC",
                declared: false,
            }],
        ),
        (
            "icc, no VP8X",
            vec![icc.clone(), vp8l()],
            0,
            1,
            vec![Error::MetadataWithoutVp8x],
        ),
        (
            "icc flag, no chunk",
            vec![vp8x(FLAG_ICC, 1, 1), vp8l()],
            0,
            0,
            vec![Error::ChunkFlagMismatch {
                flag: "ICC",
                declared: true,
            }],
        ),
        // A short VP8X carries no flags to compare, and counts as a VP8X.
        (
            "exif, short VP8X",
            vec![chunk(b"VP8X", &[0x08, 0, 0]), exif.clone()],
            1,
            0,
            vec![Error::BadVp8x { length: 3 }],
        ),
    ];
    for (name, parts, blocks, regions, issues) in cases {
        let d = riff(&parts);
        let m = read(&d).expect(name);
        assert_eq!(m.exif_blocks().len(), blocks, "{name}: blocks");
        assert_eq!(m.regions().len(), regions, "{name}: regions");
        assert_eq!(errors(&m), issues, "{name}: issues");
        assert!(m.exif_blocks().iter().all(|b| b.container().is_ok()));
    }
}

#[test]
fn flag_mismatch_issue_order() {
    // Flags say EXIF only; the chunks are ICCP and XMP: ICC, EXIF, XMP, in
    // that order, each at the VP8X chunk.
    let d = riff(&[
        vp8x(FLAG_EXIF, 1, 1),
        chunk(b"ICCP", b"icc!"),
        chunk(b"XMP ", b"<x/>"),
    ]);
    let m = read(&d).expect("reads");
    assert_eq!(
        errors(&m),
        [
            Error::ChunkFlagMismatch {
                flag: "ICC",
                declared: false
            },
            Error::ChunkFlagMismatch {
                flag: "EXIF",
                declared: true
            },
            Error::ChunkFlagMismatch {
                flag: "XMP",
                declared: false
            },
        ]
    );
    assert!(m.webp_issues().iter().all(|i| i.offset() == 12));
    // The walk's issues come first, then these, then no extraction issue is
    // lost: a second EXIF chunk adds ExtraExif after the flag issues.
    let d = riff(&[
        vp8x(0, 1, 1),
        exif_chunk(&tiff_empty()),
        exif_chunk(&tiff_empty()),
    ]);
    let m = read(&d).expect("reads");
    let e = errors(&m);
    assert!(matches!(
        e[0],
        Error::ChunkFlagMismatch { flag: "EXIF", .. }
    ));
    assert!(matches!(e[1], Error::ExtraExif { .. }));
    assert_eq!(e.len(), 2);
}

#[test]
fn halt_suppresses_extraction_issues_but_keeps_regions() {
    let good = [
        exif_chunk(&with_exif_prefix(&tiff_empty())),
        exif_chunk(&exif_with_make()),
        chunk(b"XMP ", b"<x/>"),
    ];
    // Control: no halt, the extraction issues are all there.
    let d = riff(&good);
    let m = read(&d).expect("reads");
    let e = errors(&m);
    assert!(e.iter().any(|e| matches!(e, Error::ExtraExif { .. })));
    assert!(e
        .iter()
        .any(|e| matches!(e, Error::ImproperExifHeader { .. })));
    assert_eq!((m.exif_blocks().len(), m.regions().len()), (2, 1));

    // A truncated chunk halts the walk: its Truncated is the only issue, but
    // the blocks and the region already found are kept.
    let mut d = riff(&good);
    d.extend_from_slice(b"EXIF\x10");
    // Keep the RIFF size exact so the truncation is the only walk issue.
    let exact = u32::try_from(d.len() - 8).expect("small");
    d[4..8].copy_from_slice(&exact.to_le_bytes());
    let m = read(&d).expect("reads");
    assert_eq!(
        errors(&m),
        [Error::Truncated {
            at: d.len() - 5,
            len: 8
        }]
    );
    assert_eq!((m.exif_blocks().len(), m.regions().len()), (2, 1));
}

#[test]
fn at_most_two_issues_per_chunk() {
    // Every EXIF chunk after the first has an Exif header: each costs
    // ExtraExif and ImproperExifHeader, never more.
    let one = exif_chunk(&with_exif_prefix(&tiff_empty()));
    let parts: Vec<Vec<u8>> = (0..MAX_WEBP_EXIF_BLOCKS + 3).map(|_| one.clone()).collect();
    let d = wrap(&parts);
    let m = read(&d).expect("reads");
    let mut per_chunk = std::collections::BTreeMap::<usize, usize>::new();
    for i in m.webp_issues() {
        *per_chunk.entry(i.offset()).or_default() += 1;
    }
    assert!(per_chunk.values().all(|&n| n <= 2), "{per_chunk:?}");
    assert_eq!(per_chunk.values().filter(|&&n| n == 2).count(), 15);
    // 16 blocks are parsed: the first costs one issue, 15 cost two, and the
    // three after the cap cost one each (ExtraExif only: they are not parsed).
    assert_eq!(m.webp_issues().len(), 1 + 15 * 2 + 3);
}

#[test]
fn layered_issue_view_names_each_layer() {
    // A clean walk with a TIFF whose IFD0 is fine but second block is bad
    // Exif: the TIFF layer's own issues are reported with their block.
    let d = wrap(&[exif_chunk(&exif_with_make()), exif_chunk(b"not a tiff")]);
    let m = read(&d).expect("reads");
    let layers: Vec<bool> = m
        .issues()
        .map(|i| matches!(i, AnyWebpIssue::Webp(_)))
        .collect();
    assert_eq!(layers.iter().filter(|&&w| w).count(), m.webp_issues().len());
    // An empty IFD0 that links to a next IFD far outside the payload yields
    // TIFF-layer issues, attributed to block 0 and equal to the bare parse's.
    let tiff = b"MM\x00\x2A\x00\x00\x00\x08\x00\x00\x00\x00\x10\x00".to_vec();
    let d = wrap(&[exif_chunk(&tiff)]);
    let m = read(&d).expect("reads");
    let bare = Container::parse(&tiff).expect("bare parses").issues().len();
    assert!(bare > 0, "the shape must produce a TIFF issue");
    let tiff_layer = m
        .issues()
        .filter(|i| matches!(i, AnyWebpIssue::Exif { block: 0, .. }))
        .count();
    assert_eq!(tiff_layer, bare);
    assert!(m.webp_issues().is_empty());
    assert!(!m.is_clean());
}
