//! SPEC-029 / DEC-018: PNG metadata regions against the independent builders in
//! `tests/support/png_shapes.rs`. Expectations come from the builders (offsets
//! are computed from chunk lengths), never from the library.

#[path = "support/png_shapes.rs"]
mod png_shapes;

use exif_core::png::{
    read, AnyPngIssue, Png, PngRegion, MAX_PNG_EXIF_BLOCKS, MAX_PNG_REGIONS, MAX_PNG_TRAILER_CHUNKS,
};
use exif_core::tiff::Container;
use exif_core::Error;
use png_shapes::*;

/// IHDR, `parts`, IDAT, IEND. The first part starts at offset 33.
fn wrap(parts: &[Vec<u8>]) -> Vec<u8> {
    let mut all = vec![ihdr()];
    all.extend(parts.iter().cloned());
    all.push(idat());
    all.push(iend());
    png(&all)
}

const FIRST: usize = 33;

fn inside(outer: &[u8], inner: &[u8]) -> bool {
    let o = outer.as_ptr() as usize;
    let i = inner.as_ptr() as usize;
    i >= o && i + inner.len() <= o + outer.len()
}

fn errors(d: &[u8]) -> Vec<Error> {
    read(d)
        .expect("reads")
        .png_issues()
        .iter()
        .map(|i| i.error().clone())
        .collect()
}

#[test]
fn fatal_exactly_when_the_walk_is() {
    for d in [&b""[..], b"\x89PNG", b"not a png at all"] {
        assert_eq!(read(d).err(), Png::parse(d).err());
        assert!(read(d).is_err());
    }
    assert!(read(&minimal()).is_ok());
}

#[test]
fn exif_block_equals_bare_tiff() {
    let payload = exif_with_make();
    let d = wrap(&[exif_chunk(&payload)]);
    let m = read(&d).expect("reads");
    assert!(m.is_clean());
    let [b] = m.exif_blocks() else {
        panic!("one block")
    };
    assert_eq!((b.offset(), b.tiff_at()), (FIRST, FIRST + 8));
    assert_eq!(b.tiff(), &payload[..]);
    assert!(!b.after_idat() && !b.after_iend());
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

    // A fatal error equals the bare error.
    let d = wrap(&[exif_chunk(b"not a tiff")]);
    let m = read(&d).expect("reads");
    let want = Container::parse(b"not a tiff").expect_err("bare fails");
    assert_eq!(m.exif_blocks()[0].container().err(), Some(&want));
    // A fatal TIFF error is data, not an issue.
    assert!(m.png_issues().is_empty());
    assert!(!m.is_clean());
}

#[test]
fn exif_prefix_is_accepted_with_an_issue() {
    let payload = exif_with_make();
    let mut prefixed = b"Exif\0\0".to_vec();
    prefixed.extend_from_slice(&payload);
    let d = wrap(&[exif_chunk(&prefixed)]);
    let m = read(&d).expect("reads");
    assert_eq!(errors(&d), [Error::ImproperExifHeader { at: FIRST }]);
    let b = &m.exif_blocks()[0];
    assert_eq!(b.tiff(), &payload[..]);
    assert_eq!(b.tiff_at(), FIRST + 8 + 6);
    assert_eq!(d[b.tiff_at()..b.tiff_at() + 2], *b"MM");
    assert!(b.container().is_ok());
    assert!(!m.is_clean());
}

#[test]
fn exif_empty_and_garbage_keep_the_block_err() {
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
    let ex = exif_payload();
    let first = exif_chunk(&ex);
    let second_at = FIRST + first.len();
    let d = wrap(&[first, exif_chunk(&exif_with_make())]);
    let m = read(&d).expect("reads");
    assert_eq!(m.exif_blocks().len(), 2);
    assert!(m.exif_blocks().iter().all(|b| b.container().is_ok()));
    assert_eq!(m.exif_blocks()[1].offset(), second_at);
    assert_eq!(errors(&d), [Error::ExtraExif { at: second_at }]);
    // The first block is primary.
    let primary = m.exif().expect("primary");
    assert_eq!(primary.entry_count(), 0);
}

#[test]
fn exif_blocks_are_capped_at_sixteen() {
    let one = exif_chunk(&exif_payload());
    let parts: Vec<Vec<u8>> = (0..MAX_PNG_EXIF_BLOCKS + 2).map(|_| one.clone()).collect();
    let d = wrap(&parts);
    let m = read(&d).expect("reads");
    assert_eq!(m.exif_blocks().len(), MAX_PNG_EXIF_BLOCKS);
    // Every chunk after the first costs an ExtraExif, parsed or not.
    let extras = errors(&d)
        .iter()
        .filter(|e| matches!(e, Error::ExtraExif { .. }))
        .count();
    assert_eq!(extras, MAX_PNG_EXIF_BLOCKS + 1);
    assert!(m.regions().is_empty());
    // The 17th costs only the issue: its offset has no block.
    let seventeenth = FIRST + 16 * one.len();
    assert!(m.exif_blocks().iter().all(|b| b.offset() != seventeenth));
}

#[test]
fn xmp_routes_for_itxt_and_legacy_text() {
    let k = b"XML:com.adobe.xmp";
    let d = wrap(&[
        itxt(k, 0, b"", b"", b"<a/>"),
        itxt(k, 1, b"en", b"tr", &[0x78, 1, 2]),
        text(k, b"<c/>"),
    ]);
    let m = read(&d).expect("reads");
    assert!(m.is_clean());
    let got: Vec<(bool, &[u8])> = m
        .regions()
        .iter()
        .map(|r| match r {
            PngRegion::Xmp {
                compressed, data, ..
            } => (*compressed, *data),
            other => panic!("not Xmp: {other:?}"),
        })
        .collect();
    assert_eq!(
        got,
        [
            (false, &b"<a/>"[..]),
            (true, &[0x78, 1, 2][..]),
            (false, &b"<c/>"[..])
        ]
    );
    // zTXt is an XMP carrier too, always compressed (DEC-018).
    let d = wrap(&[ztxt(k, &[1, 2])]);
    let m = read(&d).expect("reads");
    assert!(m.is_clean());
    assert!(matches!(
        m.regions()[0],
        PngRegion::Xmp {
            compressed: true,
            data: &[1, 2],
            ..
        }
    ));
}

#[test]
fn same_text_under_another_keyword_is_text() {
    let d = wrap(&[
        text(b"Comment", b"<x/>"),
        itxt(b"Description", 0, b"", b"", b"<x/>"),
    ]);
    let m = read(&d).expect("reads");
    for (r, want) in m.regions().iter().zip([&b"Comment"[..], b"Description"]) {
        let PngRegion::Text {
            keyword,
            compressed,
            data,
            ..
        } = r
        else {
            panic!("not Text: {r:?}")
        };
        assert_eq!((*keyword, *compressed, *data), (want, false, &b"<x/>"[..]));
    }
}

#[test]
fn keyword_matching_is_case_sensitive() {
    let d = wrap(&[
        text(b"xml:com.adobe.xmp", b"a"),
        text(b"XML:COM.ADOBE.XMP", b"b"),
        text(b"raw profile type exif", b"c"),
        text(b"Raw Profile Type exif", b"d"),
        text(b"XML:com.adobe.xmp ", b"e"),
    ]);
    let m = read(&d).expect("reads");
    assert_eq!(m.regions().len(), 5);
    assert!(m
        .regions()
        .iter()
        .all(|r| matches!(r, PngRegion::Text { .. })));
}

#[test]
fn raw_profile_types_in_all_three_text_chunks() {
    let d = wrap(&[
        raw_profile("exif", b"\n4578\n"),
        ztxt(b"Raw profile type xmp", &[0x78, 1]),
        itxt(b"Raw profile type foo", 1, b"", b"", &[7]),
        itxt(b"Raw profile type APP1", 0, b"", b"", b"hex"),
        raw_profile("", b"empty name"),
    ]);
    let m = read(&d).expect("reads");
    assert!(m.is_clean());
    let got: Vec<(&[u8], bool, &[u8])> = m
        .regions()
        .iter()
        .map(|r| match r {
            PngRegion::RawProfile {
                profile,
                compressed,
                data,
                ..
            } => (*profile, *compressed, *data),
            other => panic!("not RawProfile: {other:?}"),
        })
        .collect();
    assert_eq!(
        got,
        [
            (&b"exif"[..], false, &b"\n4578\n"[..]),
            (b"xmp", true, &[0x78, 1][..]),
            (b"foo", true, &[7][..]),
            (b"APP1", false, b"hex"),
            (b"", false, b"empty name"),
        ]
    );
}

#[test]
fn ordinary_text_chunks_are_text() {
    let d = wrap(&[
        text(b"Author", b"someone"),
        ztxt(b"Title", &[1, 2]),
        itxt(b"Title", 1, b"fr", b"Titre", &[3]),
    ]);
    let m = read(&d).expect("reads");
    let got: Vec<(&[u8], bool, &[u8])> = m
        .regions()
        .iter()
        .map(|r| match r {
            PngRegion::Text {
                keyword,
                compressed,
                data,
                ..
            } => (*keyword, *compressed, *data),
            other => panic!("not Text: {other:?}"),
        })
        .collect();
    assert_eq!(
        got,
        [
            (&b"Author"[..], false, &b"someone"[..]),
            (b"Title", true, &[1, 2][..]),
            (b"Title", true, &[3][..]),
        ]
    );
}

#[test]
fn bad_text_chunks_are_issues_without_regions() {
    // Nothing to locate: no NUL, an empty keyword, a truncated header.
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("no NUL", chunk(b"tEXt", b"Comment")),
        ("empty chunk", chunk(b"tEXt", b"")),
        ("empty keyword", chunk(b"tEXt", b"\0v")),
        ("zTXt no method", chunk(b"zTXt", b"Title\0")),
        ("iTXt no flag", chunk(b"iTXt", b"Title\0")),
        ("iTXt no method", chunk(b"iTXt", b"Title\0\0")),
        ("iTXt no language NUL", chunk(b"iTXt", b"Title\0\0\0en")),
        (
            "iTXt no translated NUL",
            chunk(b"iTXt", b"Title\0\0\0en\0tr"),
        ),
    ];
    for (name, c) in cases {
        let d = wrap(&[c]);
        let m = read(&d).expect("reads");
        assert!(m.regions().is_empty(), "{name}: a region was located");
        assert_eq!(
            errors(&d),
            [Error::BadTextChunk { at: FIRST }],
            "{name}: issues"
        );
    }
    // The boundary: a 79-byte keyword is fine.
    let mut kw79 = vec![b'a'; 79];
    kw79.extend_from_slice(b"\0x");
    let d = wrap(&[chunk(b"tEXt", &kw79)]);
    assert!(read(&d).expect("reads").is_clean());
}

#[test]
fn long_keyword_and_odd_itxt_flag_are_issues_with_regions() {
    let mut kw80 = vec![b'a'; 80];
    kw80.extend_from_slice(b"\0x");
    let mut long_raw = b"Raw profile type ".to_vec();
    long_raw.extend_from_slice(&[b'p'; 70]);
    long_raw.extend_from_slice(b"\0hex");
    let d = wrap(&[chunk(b"tEXt", &kw80)]);
    assert_eq!(errors(&d), [Error::BadTextChunk { at: FIRST }]);
    let m = read(&d).expect("reads");
    assert!(matches!(
        m.regions()[0],
        PngRegion::Text { keyword, data: b"x", .. } if keyword == &kw80[..80]
    ));
    // A long keyword is routed by the same rules as a short one.
    let d = wrap(&[chunk(b"tEXt", &long_raw)]);
    assert_eq!(errors(&d), [Error::BadTextChunk { at: FIRST }]);
    let m = read(&d).expect("reads");
    assert!(matches!(
        m.regions()[0],
        PngRegion::RawProfile { profile, compressed: false, .. } if profile == [b'p'; 70]
    ));
    // An iTXt flag other than 0 is compressed, and is also an issue when
    // it is not 1.
    for flag in [2u8, 255] {
        let d = wrap(&[chunk(
            b"iTXt",
            &[&b"XML:com.adobe.xmp\0"[..], &[flag, 0, 0, 0], b"zz"].concat(),
        )]);
        assert_eq!(errors(&d), [Error::BadTextChunk { at: FIRST }]);
        let m = read(&d).expect("reads");
        assert!(matches!(
            m.regions()[0],
            PngRegion::Xmp {
                compressed: true,
                data: b"zz",
                ..
            }
        ));
    }
    let d = wrap(&[chunk(b"iTXt", b"Title\0\x02\0\0\0x")]);
    let m = read(&d).expect("reads");
    assert!(matches!(
        m.regions()[0],
        PngRegion::Text {
            compressed: true,
            data: b"x",
            ..
        }
    ));
}

#[test]
fn iccp_located_and_bad_headers_are_issues() {
    let d = wrap(&[iccp(b"sRGB", 0, &[0x78, 1, 2]), iccp(b"p", 7, b"")]);
    let m = read(&d).expect("reads");
    assert!(m.is_clean());
    let got: Vec<(&[u8], u8, &[u8])> = m
        .regions()
        .iter()
        .map(|r| match r {
            PngRegion::Icc {
                name, method, data, ..
            } => (*name, *method, *data),
            other => panic!("not Icc: {other:?}"),
        })
        .collect();
    assert_eq!(got, [(&b"sRGB"[..], 0, &[0x78, 1, 2][..]), (b"p", 7, b"")]);

    let kw80 = [b'a'; 80];
    for c in [
        chunk(b"iCCP", b""),
        chunk(b"iCCP", b"\0\0abc"),
        chunk(b"iCCP", b"sRGB"),
        chunk(b"iCCP", b"sRGB\0"),
        chunk(b"iCCP", &kw80),
    ] {
        let d = wrap(&[c]);
        let m = read(&d).expect("reads");
        assert!(m.regions().is_empty());
        assert_eq!(errors(&d), [Error::BadIccpChunk { at: FIRST }]);
    }
}

#[test]
fn regions_borrow_the_input_and_start_after_the_header() {
    let t = text(b"Author", b"someone");
    let z = ztxt(b"Title", &[1, 2, 3]);
    let i = itxt(b"Title", 0, b"fr", b"Titre", b"salut");
    let c = iccp(b"sRGB", 0, &[9, 8]);
    let (zt, it, ct) = (FIRST + t.len(), FIRST + t.len() + z.len(), 0);
    let ct = ct + it + i.len();
    let d = wrap(&[t, z, i, c]);
    let m = read(&d).expect("reads");
    let start = |r: &PngRegion<'_>| r.data().as_ptr() as usize - d.as_ptr() as usize;
    for r in m.regions() {
        assert!(inside(&d, r.data()), "{r:?} not borrowed");
    }
    let rs = m.regions();
    assert_eq!(rs.len(), 4);
    // 8 bytes of length and type, then each chunk's own header fields.
    assert_eq!(start(&rs[0]), FIRST + 8 + b"Author\0".len());
    assert_eq!(start(&rs[1]), zt + 8 + b"Title\0\0".len());
    assert_eq!(start(&rs[2]), it + 8 + b"Title\0\0\0fr\0Titre\0".len());
    assert_eq!(start(&rs[3]), ct + 8 + b"sRGB\0\0".len());
    assert_eq!(
        rs.iter().map(PngRegion::offset).collect::<Vec<_>>(),
        [FIRST, zt, it, ct]
    );
    assert_eq!(rs[2].data(), b"salut");
}

#[test]
fn after_idat_and_after_iend_flags() {
    let ex = exif_payload();
    let d = png(&[
        ihdr(),
        text(b"A", b"before"),
        exif_chunk(&ex),
        idat(),
        text(b"B", b"between"),
        exif_chunk(&ex),
        idat(),
        text(b"C", b"after both"),
        iend(),
        text(b"D", b"trailer"),
        exif_chunk(&ex),
    ]);
    let m = read(&d).expect("reads");
    let flags: Vec<(bool, bool)> = m
        .regions()
        .iter()
        .map(|r| (r.after_idat(), r.after_iend()))
        .collect();
    assert_eq!(
        flags,
        [(false, false), (true, false), (true, false), (true, true)]
    );
    let flags: Vec<(bool, bool)> = m
        .exif_blocks()
        .iter()
        .map(|b| (b.after_idat(), b.after_iend()))
        .collect();
    assert_eq!(
        flags,
        [(false, false), (true, false), (true, true)],
        "block flags"
    );
    // The first IDAT itself is not "after".
    let d = png(&[ihdr(), idat(), iend()]);
    assert!(read(&d).expect("reads").regions().is_empty());
    // Offsets of trailer chunks are input offsets.
    let tail = text(b"D", b"trailer");
    let d = png(&[ihdr(), idat(), iend(), tail]);
    let m = read(&d).expect("reads");
    assert_eq!(m.regions()[0].offset(), 63);
    assert!(inside(&d, m.regions()[0].data()));
    assert!(m.regions()[0].after_iend() && m.regions()[0].after_idat());
    // Without any IDAT the trailer is not "after IDAT".
    let d = png(&[ihdr(), iend(), text(b"D", b"t")]);
    let m = read(&d).expect("reads");
    assert!(m.regions()[0].after_iend() && !m.regions()[0].after_idat());
}

#[test]
fn trailer_that_is_not_chunks_is_silent() {
    for tail in [
        vec![0xFFu8; 40],
        vec![1, 2, 3, 4, 5],
        vec![0, 0, 0, 0, b't', b'E', b'X', b't', 0, 0, 0],
        raw_chunk(0x8000_0000, b"tEXt", b"abc", 0),
        raw_chunk(1000, b"tEXt", b"abc", 0),
    ] {
        let d = png(&[ihdr(), idat(), iend(), tail]);
        let m = read(&d).expect("reads");
        assert!(m.regions().is_empty());
        assert!(m.exif_blocks().is_empty());
        assert!(m.png_issues().is_empty());
        assert!(m.is_clean());
    }
    // A trailer scan stops at the first chunk that does not fit.
    let d = png(&[
        ihdr(),
        idat(),
        iend(),
        text(b"A", b"x"),
        raw_chunk(100, b"tEXt", b"B\0cut", 0),
        text(b"C", b"never seen"),
    ]);
    let m = read(&d).expect("reads");
    assert_eq!(m.regions().len(), 1);
    assert!(m.is_clean());
}

#[test]
fn trailer_scan_is_bounded() {
    let n = MAX_PNG_TRAILER_CHUNKS + 10;
    let d = png(&[ihdr(), idat(), iend(), {
        let mut v = Vec::new();
        for _ in 0..n {
            v.extend_from_slice(&text(b"a", b"x"));
        }
        v
    }]);
    let m = read(&d).expect("reads");
    assert_eq!(m.regions().len(), MAX_PNG_TRAILER_CHUNKS);
    assert!(m.is_clean());
    // Non-metadata chunks count too: they are examined.
    let mut v = Vec::new();
    for _ in 0..MAX_PNG_TRAILER_CHUNKS {
        v.extend_from_slice(&chunk(b"prVt", b""));
    }
    v.extend_from_slice(&text(b"a", b"x"));
    let d = png(&[ihdr(), idat(), iend(), v]);
    assert!(read(&d).expect("reads").regions().is_empty());
}

#[test]
fn region_cap_halts_once() {
    // IHDR + N tEXt + IEND stays under the walker's own chunk cap; the trailer
    // supplies the regions that go past MAX_PNG_REGIONS.
    let n = 65_534usize;
    let one = text(b"a", b"x");
    let mut body = Vec::with_capacity(one.len() * n);
    for _ in 0..n {
        body.extend_from_slice(&one);
    }
    let mut tail = Vec::new();
    for _ in 0..5 {
        tail.extend_from_slice(&one);
    }
    tail.extend_from_slice(&exif_chunk(&exif_payload()));
    let d = png(&[ihdr(), body, iend(), tail]);
    let m = read(&d).expect("reads");
    assert_eq!(m.regions().len(), MAX_PNG_REGIONS);
    assert_eq!(
        errors(&d),
        [Error::TooManyRegions {
            limit: MAX_PNG_REGIONS
        }]
    );
    // Extraction stopped: the Exif chunk behind the cap is not parsed.
    assert!(m.exif_blocks().is_empty());
    // The issue is at the first chunk that did not fit: the third of five.
    let want = 8 + 25 + n * one.len() + 12 + 2 * one.len();
    assert_eq!(m.png_issues()[0].offset(), want);
}

#[test]
fn walker_halt_suppresses_extraction_issues_but_keeps_found_regions() {
    let d = png(&[
        ihdr(),
        text(b"Comment", b"kept"),
        chunk(b"tEXt", b"bad"),
        exif_chunk(b"junk"),
        exif_chunk(b"junk"),
        raw_chunk(0x8000_0000, b"IDAT", b"", 0),
    ]);
    let m = read(&d).expect("reads");
    assert_eq!(m.regions().len(), 1);
    assert_eq!(m.exif_blocks().len(), 2);
    // Exactly the walker's own issue: no BadTextChunk, no ExtraExif.
    assert_eq!(
        errors(&d),
        [Error::BadChunkLength {
            length: 0x8000_0000
        }]
    );
    assert_eq!(m.png_issues().len(), m.png().issues().len());
    // Without a halt the same chunks do report.
    let d = png(&[
        ihdr(),
        text(b"Comment", b"kept"),
        chunk(b"tEXt", b"bad"),
        exif_chunk(b"junk"),
        exif_chunk(b"junk"),
        idat(),
        iend(),
    ]);
    assert_eq!(errors(&d).len(), 2);
}

#[test]
fn is_clean_and_issue_counts() {
    let d = wrap(&[text(b"A", b"b"), exif_chunk(&exif_with_make())]);
    let m = read(&d).expect("reads");
    assert!(m.is_clean());
    assert_eq!(m.issues().count(), 0);

    // A PNG-layer issue alone makes it unclean.
    let no_iend = png(&[ihdr(), idat()]);
    let m = read(&no_iend).expect("reads");
    assert!(!m.is_clean());
    assert_eq!(m.issues().count(), 1);
    assert!(matches!(m.issues().next(), Some(AnyPngIssue::Png(_))));

    // A block that parsed with TIFF issues: unclean, counted per block.
    // IFD0 is empty but links to a next IFD far outside the payload.
    let truncated = b"MM\x00\x2A\x00\x00\x00\x08\x00\x00\x00\x00\x10\x00".to_vec();
    let d = wrap(&[
        text(b"A", b"b"),
        chunk(b"tEXt", b"bad"),
        exif_chunk(&truncated),
        exif_chunk(&exif_payload()),
    ]);
    let m = read(&d).expect("reads");
    let tiff_issues: usize = m
        .exif_blocks()
        .iter()
        .filter_map(|b| b.container().ok())
        .map(|c| c.issues().len())
        .sum();
    assert!(
        tiff_issues > 0,
        "the truncated IFD must cost the TIFF layer an issue"
    );
    assert!(!m.is_clean());
    assert_eq!(m.issues().count(), m.png_issues().len() + tiff_issues);
    let layers = m
        .issues()
        .filter(|i| matches!(i, AnyPngIssue::Exif { block: 0, .. }))
        .count();
    assert_eq!(layers, tiff_issues);

    // A fatal block alone makes it unclean without adding an issue.
    let d = wrap(&[exif_chunk(b"")]);
    let m = read(&d).expect("reads");
    assert!(!m.is_clean());
    assert_eq!(m.issues().count(), 0);
}

#[test]
fn every_prefix_and_byte_flip_of_each_metadata_shape_is_panic_free() {
    let seeds = meta_seeds();
    assert!(seeds.len() >= 20);
    for (name, d) in seeds {
        let m = read(&d).unwrap_or_else(|e| panic!("{name}: {e}"));
        for r in m.regions() {
            assert!(inside(&d, r.data()), "{name}");
        }
        for i in 0..=d.len() {
            let _ = read(&d[..i]);
            if i < d.len() {
                let mut f = d.clone();
                f[i] ^= 0xFF;
                let _ = read(&f);
            }
        }
    }
}
