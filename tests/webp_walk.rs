//! SPEC-030 / DEC-019: the WebP RIFF walker against the independent builders
//! in `tests/support/webp_shapes.rs`. Expectations come from the builders
//! (offsets, sizes), never from the library.

#[path = "support/webp_shapes.rs"]
mod webp_shapes;

use exif_core::webp::{ChunkKind, Webp, MAX_WEBP_CHUNKS, MAX_WEBP_ISSUES};
use exif_core::Error;
use webp_shapes::*;

fn errors(w: &Webp<'_>) -> Vec<Error> {
    w.issues().iter().map(|i| i.error().clone()).collect()
}

fn offsets(w: &Webp<'_>) -> Vec<usize> {
    w.issues().iter().map(|i| i.offset()).collect()
}

fn kinds(w: &Webp<'_>) -> Vec<ChunkKind> {
    w.chunks().iter().map(|c| c.kind()).collect()
}

fn inside(outer: &[u8], inner: &[u8]) -> bool {
    let o = outer.as_ptr() as usize;
    let i = inner.as_ptr() as usize;
    i >= o && i + inner.len() <= o + outer.len()
}

fn exif() -> Vec<u8> {
    chunk(b"EXIF", &tiff_empty())
}

#[test]
fn not_webp_is_fatal_and_header_only_is_ok() {
    // Under 12 bytes: Truncated, for every length including the empty input.
    let full = minimal();
    let want = Error::Truncated { at: 0, len: 12 };
    for n in 0..12 {
        let d = &full[..n];
        assert_eq!(Webp::parse(d).err(), Some(want.clone()), "{n} bytes");
        assert_eq!(Webp::parse_strict(d).err(), Some(want.clone()), "{n} bytes");
    }
    // Wrong magic and a non-WEBP form type carry the 12 bytes.
    let mut bad = minimal();
    bad[0] = b'X';
    let mark: [u8; 12] = bad[..12].try_into().expect("12");
    assert_eq!(
        Webp::parse(&bad).err(),
        Some(Error::NotWebp { mark }),
        "wrong magic"
    );
    assert_eq!(
        Webp::parse_strict(&bad).err(),
        Some(Error::NotWebp { mark })
    );
    let mut avi = b"RIFF".to_vec();
    avi.extend_from_slice(&4u32.to_le_bytes());
    avi.extend_from_slice(b"AVI ");
    avi.extend_from_slice(&vp8l());
    let mark: [u8; 12] = avi[..12].try_into().expect("12");
    assert_eq!(Webp::parse(&avi).err(), Some(Error::NotWebp { mark }));
    assert_eq!(
        Webp::parse_strict(&avi).err(),
        Some(Error::NotWebp { mark })
    );
    // A JPEG is not a WebP.
    assert!(matches!(
        Webp::parse(&[0xFF, 0xD8, 0xFF, 0xE0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        Err(Error::NotWebp { .. })
    ));

    // A 12-byte header alone is Ok: no chunks, no issue.
    let d = header(4);
    let w = Webp::parse(&d).expect("header only");
    assert!(w.chunks().is_empty());
    assert!(w.issues().is_empty());
    assert!(w.is_clean());
    assert!(w.vp8x().is_none());
    assert_eq!(w.declared_size(), 4);
    assert!(Webp::parse_strict(&d).is_ok());
}

#[test]
fn minimal_webp_is_clean() {
    let d = minimal();
    let w = Webp::parse(&d).expect("minimal");
    assert!(w.is_clean(), "{:?}", w.issues());
    assert_eq!(kinds(&w), vec![ChunkKind::Vp8l]);
    let c = &w.chunks()[0];
    assert_eq!(c.offset(), 12);
    assert_eq!(c.fourcc(), *b"VP8L");
    assert_eq!(c.data(), &[0x2F, 0, 0, 0, 0]);
    // Odd data length: the range includes the pad byte.
    assert_eq!(c.range(), 12..12 + 8 + 5 + 1);
    assert_eq!(w.declared_size() as usize, d.len() - 8);
}

#[test]
fn declared_size_never_limits_the_walk() {
    // A VP8X with the EXIF flag keeps the flag rules quiet: the size mismatch
    // is the only thing wrong.
    let body = [vp8x(FLAG_EXIF, 1, 1), exif()];
    let mut trailing = riff(&body);
    trailing.extend_from_slice(&chunk(b"JUNK", &[1, 2]));
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("too small", riff_with_size(4, &body)),
        ("zero", riff_with_size(0, &body)),
        ("odd", riff_with_size(7, &body)),
        ("too large", riff_with_size(0xFFFF_FFF0, &body)),
        ("trailing", trailing),
    ];
    for (name, d) in &cases {
        let w = Webp::parse(d).expect(name);
        assert!(
            kinds(&w).contains(&ChunkKind::Exif),
            "{name}: the EXIF chunk must still be found"
        );
        let declared = w.declared_size();
        assert_eq!(
            errors(&w),
            vec![Error::RiffSizeMismatch {
                declared,
                actual: d.len() - 8,
            }],
            "{name}"
        );
        assert_eq!(offsets(&w), vec![4], "{name}");
    }
    // The trailing chunk is walked too.
    let w = Webp::parse(&cases[4].1).expect("trailing");
    assert_eq!(
        kinds(&w),
        vec![ChunkKind::Vp8x, ChunkKind::Exif, ChunkKind::Other]
    );
}

#[test]
fn riff_size_mismatch_is_one_issue() {
    // Exact: no issue.
    let exact = riff(&[vp8x(FLAG_EXIF, 1, 1), exif()]);
    let w = Webp::parse(&exact).expect("exact");
    assert!(w.is_clean());
    // One more or one fewer byte than declared: exactly one issue each, and
    // it leads the issue list when others follow.
    let d = riff_with_size(100, &[chunk_unpadded(b"JUNK", &[1, 2, 3])]);
    let w = Webp::parse(&d).expect("mismatch plus pad");
    assert_eq!(
        errors(&w),
        vec![
            Error::RiffSizeMismatch {
                declared: 100,
                actual: d.len() - 8,
            },
            Error::MissingPad { at: 12 },
        ]
    );
    assert_eq!(offsets(&w), vec![4, 12]);
    assert_eq!(
        Webp::parse_strict(&d).err(),
        Some(Error::RiffSizeMismatch {
            declared: 100,
            actual: d.len() - 8,
        })
    );
}

#[test]
fn pad_byte_rules() {
    // A pad byte after an odd chunk is skipped: the next chunk is found, and
    // the pad's value is not checked.
    let mut junk = chunk(b"JUNK", &[1]);
    assert_eq!(junk.len(), 10);
    junk[9] = 0xFF;
    let d = riff(&[
        vp8x(FLAG_EXIF | FLAG_XMP, 1, 1),
        junk,
        exif(),
        chunk(b"XMP ", b"<x/>"),
    ]);
    let w = Webp::parse(&d).expect("padded");
    assert!(w.is_clean(), "{:?}", w.issues());
    assert_eq!(
        kinds(&w),
        vec![
            ChunkKind::Vp8x,
            ChunkKind::Other,
            ChunkKind::Exif,
            ChunkKind::Xmp
        ]
    );
    // The VP8X chunk is 18 bytes; the odd JUNK chunk is 8 + 1 + 1 pad.
    assert_eq!(w.chunks()[1].range(), 30..40);
    assert_eq!(w.chunks()[2].offset(), 40);

    // An odd chunk that ends exactly at the end of the input: MissingPad, the
    // chunk stays recorded and its range stops at the end.
    let d = riff(&[chunk_unpadded(b"JUNK", &[1, 2, 3])]);
    let w = Webp::parse(&d).expect("missing pad at end");
    assert_eq!(errors(&w), vec![Error::MissingPad { at: 12 }]);
    assert_eq!(offsets(&w), vec![12]);
    assert_eq!(w.chunks().len(), 1);
    assert_eq!(w.chunks()[0].range(), 12..d.len());

    // An unpadded odd chunk in the middle misframes the next chunk, exactly as
    // ExifTool does: the XMP chunk after it is not found.
    let d = riff(&[chunk_unpadded(b"JUNK", &[1]), chunk(b"XMP ", b"<x/>")]);
    let w = Webp::parse(&d).expect("unpadded in the middle");
    assert!(!kinds(&w).contains(&ChunkKind::Xmp));
    assert_eq!(w.chunks().len(), 1);
    assert!(matches!(
        errors(&w).as_slice(),
        [Error::Truncated { at: 21 | 22, .. }]
    ));
}

#[test]
fn damage_ends_the_walk_and_omits_the_chunk() {
    let good = riff(&[exif()]);
    let at = good.len();
    let mut cases: Vec<(String, Vec<u8>, usize)> = Vec::new();
    // One to seven bytes left: not even a chunk header.
    for n in 1..8 {
        let mut d = good.clone();
        d.extend_from_slice(&vec![b'A'; n]);
        cases.push((format!("{n} bytes left"), d, 8));
    }
    // A partial chunk header: `XMP ` plus one byte.
    let mut d = good.clone();
    d.extend_from_slice(b"XMP \x01");
    cases.push(("partial header".to_string(), d, 8));
    // A chunk size past the end.
    let mut d = good.clone();
    d.extend_from_slice(&raw_chunk(b"XMP ", 1000, b"abc"));
    cases.push(("size past end".to_string(), d, 8 + 1000));
    let mut d = good.clone();
    d.extend_from_slice(&raw_chunk(b"XMP ", u32::MAX, b""));
    cases.push(("size 0xFFFFFFFF".to_string(), d, 8 + u32::MAX as usize));
    // One byte short of the chunk's data.
    let mut d = good.clone();
    d.extend_from_slice(&raw_chunk(b"XMP ", 5, b"1234"));
    cases.push(("one byte short".to_string(), d, 8 + 5));

    for (name, d, len) in cases {
        // The RIFF size is whatever it is: fix it so only the damage shows.
        let mut d = d;
        let exact = (d.len() - 8) as u32;
        d[4..8].copy_from_slice(&exact.to_le_bytes());
        let w = Webp::parse(&d).expect(&name);
        assert_eq!(errors(&w), vec![Error::Truncated { at, len }], "{name}");
        assert_eq!(offsets(&w), vec![at], "{name}");
        // The chunk is omitted entirely; earlier chunks are kept.
        assert_eq!(kinds(&w), vec![ChunkKind::Exif], "{name}");
        assert!(Webp::parse_strict(&d).is_err());
    }
}

#[test]
fn fourcc_kinds_are_case_sensitive() {
    let table: [(&[u8; 4], ChunkKind); 16] = [
        (b"VP8X", ChunkKind::Vp8x),
        (b"VP8 ", ChunkKind::Vp8),
        (b"VP8L", ChunkKind::Vp8l),
        (b"ALPH", ChunkKind::Alph),
        (b"ANIM", ChunkKind::Anim),
        (b"ANMF", ChunkKind::Anmf),
        (b"ICCP", ChunkKind::Iccp),
        (b"EXIF", ChunkKind::Exif),
        (b"XMP ", ChunkKind::Xmp),
        (b"exif", ChunkKind::Other),
        (b"Exif", ChunkKind::Other),
        (b"EXIf", ChunkKind::Other),
        (b"xmp ", ChunkKind::Other),
        (b"iccp", ChunkKind::Other),
        (b"XMP-", ChunkKind::Other),
        (b"XMPx", ChunkKind::Other),
    ];
    for (code, kind) in table {
        // An 11-byte payload keeps VP8X from being short and pad rules out of
        // the way (the pad is built in).
        let d = riff(&[chunk(code, &[0u8; 10])]);
        let w = Webp::parse(&d).expect("kind");
        assert_eq!(kinds(&w), vec![kind], "{code:?}");
        assert_eq!(w.chunks()[0].fourcc(), *code);
        // Only the metadata kinds and the VP8X flag checks may add issues.
        let allowed = |e: &Error| {
            matches!(
                e,
                Error::MetadataWithoutVp8x | Error::ChunkFlagMismatch { .. }
            )
        };
        assert!(errors(&w).iter().all(allowed), "{code:?}: {:?}", w.issues());
    }
    // `XMP\0` is read as XMP with one NonCanonicalChunkId.
    let d = riff(&[chunk(b"XMP\0", b"<x/>")]);
    let w = Webp::parse(&d).expect("xmp nul");
    assert_eq!(kinds(&w), vec![ChunkKind::Xmp]);
    let noncanon = Error::NonCanonicalChunkId {
        chunk_id: *b"XMP\0",
        at: 12,
    };
    assert_eq!(errors(&w)[0], noncanon);
    assert_eq!(
        errors(&w).iter().filter(|e| **e == noncanon).count(),
        1,
        "exactly one"
    );
}

#[test]
fn vp8x_decode_short_and_misplaced() {
    // Flags and canvas decoded (stored + 1).
    let d = riff(&[vp8x(0x3E, 640, 480), vp8l()]);
    let w = Webp::parse(&d).expect("vp8x");
    let v = w.vp8x().expect("vp8x info");
    assert_eq!(v.flags(), 0x3E);
    assert!(v.has_icc() && v.has_alpha() && v.has_exif() && v.has_xmp() && v.is_animated());
    assert_eq!((v.canvas_width(), v.canvas_height()), (640, 480));
    let v = Webp::parse(&riff(&[vp8x(FLAG_ALPHA, 1, 1)]))
        .expect("vp8x")
        .vp8x()
        .expect("info");
    assert!(v.has_alpha());
    assert!(!v.has_icc() && !v.has_exif() && !v.has_xmp() && !v.is_animated());
    assert_eq!((v.canvas_width(), v.canvas_height()), (1, 1));
    // The largest canvas: 2^24 on each side.
    let v = Webp::parse(&riff(&[vp8x(0, 1 << 24, 1 << 24)]))
        .expect("vp8x")
        .vp8x()
        .expect("info");
    assert_eq!((v.canvas_width(), v.canvas_height()), (1 << 24, 1 << 24));

    // Under 10 bytes: BadVp8x, no info; the walk goes on.
    for n in [0usize, 3, 9] {
        let d = riff(&[chunk(b"VP8X", &vec![0u8; n]), exif()]);
        let w = Webp::parse(&d).expect("short vp8x");
        assert!(w.vp8x().is_none(), "{n}");
        assert_eq!(errors(&w), vec![Error::BadVp8x { length: n as u32 }], "{n}");
        assert!(kinds(&w).contains(&ChunkKind::Exif));
    }
    // Exactly 10 bytes is fine, and so is more.
    assert!(Webp::parse(&riff(&[chunk(b"VP8X", &[0u8; 12])]))
        .expect("12")
        .vp8x()
        .is_some());

    // Not first: MisplacedVp8x at the chunk; the info is still the first VP8X.
    let d = riff(&[vp8l(), vp8x(0, 1, 1)]);
    let w = Webp::parse(&d).expect("misplaced");
    let at = 12 + 8 + 5 + 1;
    assert_eq!(errors(&w), vec![Error::MisplacedVp8x { at }]);
    assert_eq!(offsets(&w), vec![at]);
    assert!(w.vp8x().is_some());
}

#[test]
fn chunk_cap_is_exact() {
    let exact = riff(&[empty_junk(MAX_WEBP_CHUNKS)]);
    let w = Webp::parse(&exact).expect("at the cap");
    assert_eq!(w.chunks().len(), MAX_WEBP_CHUNKS);
    assert!(w.is_clean(), "{:?}", w.issues().first());
    assert!(Webp::parse_strict(&exact).is_ok());

    let over = riff(&[empty_junk(MAX_WEBP_CHUNKS + 1)]);
    let w = Webp::parse(&over).expect("over the cap");
    assert_eq!(w.chunks().len(), MAX_WEBP_CHUNKS);
    assert_eq!(
        errors(&w),
        vec![Error::TooManyChunks {
            limit: MAX_WEBP_CHUNKS
        }]
    );
    assert_eq!(offsets(&w), vec![12 + 8 * MAX_WEBP_CHUNKS]);
}

#[test]
fn issue_cap_halts_once() {
    // Every VP8X after the first is one MisplacedVp8x.
    let d = riff(&misplaced_vp8x(1100));
    let w = Webp::parse(&d).expect("many issues");
    assert_eq!(w.issues().len(), MAX_WEBP_ISSUES + 1);
    assert!(
        matches!(
            w.issues().last().map(|i| i.error()),
            Some(Error::TooManyIssues { limit }) if *limit == MAX_WEBP_ISSUES
        ),
        "the cap issue is last"
    );
    let cap_count = errors(&w)
        .iter()
        .filter(|e| matches!(e, Error::TooManyIssues { .. }))
        .count();
    assert_eq!(cap_count, 1);
    // The cap chunk is not recorded: the first VP8X plus 1024 misplaced ones.
    assert_eq!(w.chunks().len(), 1 + MAX_WEBP_ISSUES);
    // Exactly at the cap is not a halt.
    let d = riff(&misplaced_vp8x(MAX_WEBP_ISSUES));
    let w = Webp::parse(&d).expect("at the issue cap");
    assert_eq!(w.issues().len(), MAX_WEBP_ISSUES);
    assert!(w
        .issues()
        .iter()
        .all(|i| matches!(i.error(), Error::MisplacedVp8x { .. })));
}

#[test]
fn anmf_payload_is_not_walked() {
    // A chunk-shaped payload inside ANMF is passed over, not recorded.
    let d = riff(&[chunk(b"ANMF", &exif()), exif()]);
    let w = Webp::parse(&d).expect("anmf");
    assert_eq!(kinds(&w), vec![ChunkKind::Anmf, ChunkKind::Exif]);
    assert_eq!(w.chunks()[0].data(), exif().as_slice());
    // The EXIF after the ANMF is at its top-level offset.
    assert_eq!(w.chunks()[1].offset(), 12 + 8 + exif().len());
}

#[test]
fn payloads_borrow_the_input() {
    let d = riff(&[
        vp8x(FLAG_EXIF | FLAG_XMP, 4, 4),
        vp8l(),
        exif(),
        chunk(b"XMP ", b"<x/>"),
    ]);
    let w = Webp::parse(&d).expect("walk");
    assert_eq!(w.chunks().len(), 4);
    let mut prev_end = 12;
    for c in w.chunks() {
        assert!(inside(&d, c.data()), "{:?}", c.fourcc());
        assert!(c.offset() >= prev_end && c.range().end <= d.len());
        assert_eq!(
            c.data().as_ptr() as usize - d.as_ptr() as usize,
            c.offset() + 8
        );
        prev_end = c.range().end;
    }
    // Even a chunk with no data borrows (an empty slice at the right place).
    let d = riff(&[chunk(b"JUNK", &[])]);
    let w = Webp::parse(&d).expect("empty chunk");
    assert!(inside(&d, w.chunks()[0].data()));
}

#[test]
fn parse_strict_ok_iff_clean() {
    for (name, d) in seeds() {
        match Webp::parse(&d) {
            Ok(w) => {
                let strict = Webp::parse_strict(&d);
                assert_eq!(strict.is_ok(), w.is_clean(), "{name}");
                assert_eq!(w.is_clean(), w.issues().is_empty(), "{name}");
                if let Err(e) = strict {
                    assert_eq!(Some(&e), w.issues().first().map(|i| i.error()), "{name}");
                }
            }
            Err(e) => assert_eq!(Webp::parse_strict(&d).err(), Some(e), "{name}"),
        }
    }
}

#[test]
fn every_prefix_and_byte_flip_is_panic_free() {
    let mut shapes: Vec<(String, Vec<u8>)> = seeds()
        .into_iter()
        .map(|(n, d)| (n.to_string(), d))
        .collect();
    shapes.push(("empty".to_string(), Vec::new()));
    shapes.push(("eleven".to_string(), header(4)[..11].to_vec()));
    for (name, d) in shapes {
        // The big seeds are sampled; the small ones are checked exhaustively.
        let step = (d.len() / 200).max(1);
        let mut i = 0;
        while i <= d.len() {
            let _ = Webp::parse(&d[..i]);
            let _ = Webp::parse_strict(&d[..i]);
            if i < d.len() {
                let mut m = d.clone();
                m[i] ^= 0xFF;
                let _ = Webp::parse(&m);
                let _ = Webp::parse_strict(&m);
            }
            i += step;
        }
        let _ = name;
    }
}
