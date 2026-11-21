//! SPEC-028 / DEC-017: the PNG chunk walker against the independent builders in
//! `tests/support/png_shapes.rs`. Expectations come from the builders (offsets,
//! bitwise CRC), never from the library.

#[path = "support/png_shapes.rs"]
mod png_shapes;

use exif_core::png::{ChunkKind, Png, MAX_PNG_CHUNKS, MAX_PNG_ISSUES};
use exif_core::Error;
use png_shapes::*;

fn errors(p: &Png<'_>) -> Vec<Error> {
    p.issues().iter().map(|i| i.error().clone()).collect()
}

fn offsets(p: &Png<'_>) -> Vec<usize> {
    p.issues().iter().map(|i| i.offset()).collect()
}

fn inside(outer: &[u8], inner: &[u8]) -> bool {
    let o = outer.as_ptr() as usize;
    let i = inner.as_ptr() as usize;
    i >= o && i + inner.len() <= o + outer.len()
}

#[test]
fn not_png_is_fatal() {
    let mut d = SIG.to_vec();
    d[7] = 0x0B;
    d.extend_from_slice(&minimal()[8..]);
    let want = Error::NotPng {
        mark: [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0B],
    };
    assert_eq!(Png::parse(&d).err(), Some(want.clone()));
    assert_eq!(Png::parse_strict(&d).err(), Some(want));
    // A JPEG is not a PNG either.
    assert!(matches!(
        Png::parse(&[0xFF, 0xD8, 0xFF, 0xE0, 0, 0, 0, 0, 0]),
        Err(Error::NotPng { .. })
    ));
}

#[test]
fn short_input_is_truncated() {
    let want = Error::Truncated { at: 0, len: 8 };
    for n in 0..8 {
        let d = &SIG[..n];
        assert_eq!(Png::parse(d).err(), Some(want.clone()), "{n} bytes");
        assert_eq!(Png::parse_strict(d).err(), Some(want.clone()), "{n} bytes");
    }
    assert_eq!(Png::parse(&[]).err(), Some(want));
}

#[test]
fn signature_only_has_missing_ihdr_and_iend() {
    let p = Png::parse(&SIG).expect("signature only");
    assert!(p.chunks().is_empty());
    assert_eq!(errors(&p), vec![Error::MissingIhdr, Error::MissingIend]);
    assert_eq!(offsets(&p), vec![8, 8]);
    assert_eq!(p.iend_offset(), None);
    assert!(p.trailer().is_empty());
    assert!(!p.is_clean());
    assert_eq!(Png::parse_strict(&SIG).err(), Some(Error::MissingIhdr));
}

#[test]
fn minimal_png_is_clean() {
    let d = minimal();
    let p = Png::parse(&d).expect("minimal");
    assert!(p.is_clean(), "{:?}", p.issues());
    assert_eq!(p.chunks().len(), 3);
    let kinds: Vec<ChunkKind> = p.chunks().iter().map(|c| c.kind()).collect();
    assert_eq!(
        kinds,
        vec![ChunkKind::Ihdr, ChunkKind::Idat, ChunkKind::Iend]
    );
    for (c, off) in p.chunks().iter().zip(MINIMAL_OFFSETS) {
        assert!(c.crc_ok());
        assert_eq!(c.offset(), off);
        assert_eq!(c.range(), off..off + 12 + c.data().len());
    }
    assert_eq!(p.chunks()[0].chunk_type(), *b"IHDR");
    assert_eq!(p.chunks()[0].data(), ihdr_data().as_slice());
    assert_eq!(p.chunks()[2].stored_crc(), 0xAE42_6082);
    assert_eq!(p.iend_offset(), Some(51));
    assert!(p.trailer().is_empty());
    assert!(Png::parse_strict(&d).is_ok());
}

#[test]
fn crc_mismatch_is_an_issue_and_the_chunk_is_kept() {
    let d = png(&[
        ihdr(),
        chunk_with_crc(b"IDAT", &[1, 2, 3], 1),
        chunk(b"tEXt", b"k\0v"),
        iend(),
    ]);
    let p = Png::parse(&d).expect("walks");
    let want = crc32(b"IDAT\x01\x02\x03");
    assert_eq!(
        errors(&p),
        vec![Error::BadChunkCrc {
            stored: 1,
            computed: want
        }]
    );
    assert_eq!(offsets(&p), vec![33]);
    assert_eq!(p.chunks().len(), 4);
    assert!(!p.chunks()[1].crc_ok());
    assert_eq!(p.chunks()[1].stored_crc(), 1);
    assert_eq!(p.chunks()[1].data(), &[1, 2, 3]);
    // Later chunks are still walked and still good.
    assert!(p.chunks()[2].crc_ok() && p.chunks()[3].crc_ok());
    assert_eq!(p.iend_offset(), Some(p.chunks()[3].offset()));
    assert_eq!(
        Png::parse_strict(&d).err(),
        Some(Error::BadChunkCrc {
            stored: 1,
            computed: want
        })
    );
}

#[test]
fn crc32_known_vectors() {
    // The builder's bitwise CRC against the published check values...
    assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    assert_eq!(crc32(b"IEND"), 0xAE42_6082);
    // ...and the library's table against them, through the issue it reports.
    let d = png(&[ihdr(), chunk_with_crc(b"1234", b"56789", 0), iend()]);
    let p = Png::parse(&d).expect("walks");
    assert!(errors(&p).contains(&Error::BadChunkCrc {
        stored: 0,
        computed: 0xCBF4_3926
    }));
    // A control: a one-bit-perturbed expected value is not the computed one.
    assert!(!errors(&p).contains(&Error::BadChunkCrc {
        stored: 0,
        computed: 0xCBF4_3927
    }));
}

#[test]
fn length_past_eof_ends_the_walk_without_the_chunk() {
    let at = 33; // IHDR is 25 bytes after the 8-byte signature
    let cases: Vec<(Vec<u8>, Error)> = vec![
        // declared length exceeds the data by exactly one byte
        (
            png(&[ihdr(), raw_chunk(4, b"tEXt", b"abc", 0)]),
            Error::Truncated { at, len: 12 + 4 },
        ),
        // far past EOF but within 2^31-1
        (
            png(&[ihdr(), raw_chunk(100, b"tEXt", b"abc", 0)]),
            Error::Truncated { at, len: 112 },
        ),
        (
            png(&[ihdr(), raw_chunk(0x7FFF_FFFF, b"tEXt", b"abc", 0)]),
            Error::Truncated {
                at,
                len: 12 + 0x7FFF_FFFF,
            },
        ),
        // 11 bytes left: not even a whole empty chunk
        (
            png(&[ihdr(), vec![0, 0, 0, 0, b'I', b'E', b'N', b'D', 0, 0, 0]]),
            Error::Truncated { at, len: 12 },
        ),
        (png(&[ihdr(), vec![0]]), Error::Truncated { at, len: 12 }),
        // above 2^31-1: the stream cannot be resynchronised
        (
            png(&[ihdr(), raw_chunk(0x8000_0000, b"tEXt", b"abc", 0)]),
            Error::BadChunkLength {
                length: 0x8000_0000,
            },
        ),
        (
            png(&[ihdr(), raw_chunk(0xFFFF_FFFF, b"tEXt", b"abc", 0)]),
            Error::BadChunkLength {
                length: 0xFFFF_FFFF,
            },
        ),
    ];
    for (i, (d, want)) in cases.iter().enumerate() {
        let p = Png::parse(d).expect("walks");
        assert_eq!(errors(&p), vec![want.clone()], "case {i}");
        assert_eq!(offsets(&p), vec![at], "case {i}");
        assert_eq!(p.chunks().len(), 1, "case {i}: the chunk is not recorded");
        assert_eq!(p.iend_offset(), None);
        assert!(p.trailer().is_empty());
        assert_eq!(&Png::parse_strict(d).err(), &Some(want.clone()), "case {i}");
    }
}

#[test]
fn missing_ihdr() {
    let d = png(&[chunk(b"tEXt", b"x"), idat(), iend()]);
    let p = Png::parse(&d).expect("walks");
    assert_eq!(errors(&p), vec![Error::MissingIhdr]);
    assert_eq!(offsets(&p), vec![8]);
    assert_eq!(p.chunks().len(), 3);
}

#[test]
fn bad_ihdr() {
    for (name, mut data, len) in [("short", ihdr_data(), 12u32), ("long", ihdr_data(), 14)] {
        if len < 13 {
            data.pop();
        } else {
            data.push(0);
        }
        let d = png(&[chunk(b"IHDR", &data), idat(), iend()]);
        let p = Png::parse(&d).expect("walks");
        assert_eq!(errors(&p), vec![Error::BadIhdr { length: len }], "{name}");
        assert_eq!(offsets(&p), vec![8], "{name}");
        assert_eq!(p.chunks()[0].kind(), ChunkKind::Ihdr);
        assert_eq!(p.chunks().len(), 3);
    }
}

#[test]
fn missing_iend() {
    let d = png(&[ihdr(), idat()]);
    let p = Png::parse(&d).expect("walks");
    assert_eq!(errors(&p), vec![Error::MissingIend]);
    assert_eq!(offsets(&p), vec![d.len()]);
    assert_eq!(p.iend_offset(), None);
    assert!(p.trailer().is_empty());
    assert_eq!(p.chunks().len(), 2);
}

#[test]
fn bad_chunk_type_keeps_the_chunk_as_other() {
    let d = png(&[ihdr(), chunk(b"te1t", b"x"), idat(), iend()]);
    let p = Png::parse(&d).expect("walks");
    assert_eq!(
        errors(&p),
        vec![Error::BadChunkType {
            chunk_type: *b"te1t"
        }]
    );
    assert_eq!(offsets(&p), vec![33]);
    assert_eq!(p.chunks().len(), 4);
    assert_eq!(p.chunks()[1].kind(), ChunkKind::Other);
    assert_eq!(p.chunks()[1].chunk_type(), *b"te1t");
    // Lower-case ancillary and private types are fine.
    let d = png(&[ihdr(), chunk(b"prVt", b"x"), idat(), iend()]);
    assert!(Png::parse(&d).expect("walks").is_clean());
}

#[test]
fn kinds_are_identified_by_exact_type() {
    let d = png(&[
        ihdr(),
        chunk(b"PLTE", &[0; 3]),
        chunk(b"eXIf", &exif_payload()),
        chunk(b"iTXt", b"k\0\0\0\0\0v"),
        chunk(b"tEXt", b"k\0v"),
        chunk(b"zTXt", b"k\0\0\x78"),
        chunk(b"iCCP", b"p\0\0\x78"),
        chunk(b"exif", b"lower"),
        idat(),
        iend(),
    ]);
    let p = Png::parse(&d).expect("walks");
    assert!(p.is_clean());
    let kinds: Vec<ChunkKind> = p.chunks().iter().map(|c| c.kind()).collect();
    use ChunkKind::*;
    assert_eq!(
        kinds,
        vec![Ihdr, Plte, Exif, Itxt, Text, Ztxt, Iccp, Other, Idat, Iend]
    );
}

#[test]
fn several_issues_at_once_keep_their_order() {
    // Chunk 1: bad type, not IHDR, wrong CRC. Chunk 2: IHDR of 14. Chunk 3:
    // a length above 2^31-1.
    let mut long = ihdr_data();
    long.push(0);
    let d = png(&[
        chunk_with_crc(b"1234", b"x", 7),
        chunk(b"IHDR", &long),
        raw_chunk(0xFFFF_FFFF, b"IDAT", b"", 0),
    ]);
    let p = Png::parse(&d).expect("walks");
    assert_eq!(
        errors(&p),
        vec![
            Error::BadChunkType {
                chunk_type: *b"1234"
            },
            Error::MissingIhdr,
            Error::BadChunkCrc {
                stored: 7,
                computed: crc32(b"1234x")
            },
            Error::BadIhdr { length: 14 },
            Error::BadChunkLength {
                length: 0xFFFF_FFFF
            },
        ]
    );
    assert_eq!(offsets(&p), vec![8, 8, 8, 21, 47]);
    assert_eq!(p.chunks().len(), 2);
}

#[test]
fn iend_ends_the_walk_and_the_rest_is_the_trailer() {
    let d = png(&[ihdr(), idat(), iend(), vec![1, 2, 3, 4, 5]]);
    let p = Png::parse(&d).expect("walks");
    assert!(p.is_clean());
    assert_eq!(p.chunks().len(), 3);
    assert_eq!(p.iend_offset(), Some(51));
    assert_eq!(p.trailer(), &[1, 2, 3, 4, 5]);
    // A whole chunk after IEND is trailer bytes, not walked.
    let tail = chunk(b"eXIf", &exif_payload());
    let d = png(&[ihdr(), idat(), iend(), tail.clone()]);
    let p = Png::parse(&d).expect("walks");
    assert!(p.is_clean());
    assert_eq!(p.chunks().len(), 3);
    assert_eq!(p.trailer(), tail.as_slice());
    // IEND with data is still IEND.
    let d = png(&[ihdr(), idat(), chunk(b"IEND", &[1, 2, 3])]);
    let p = Png::parse(&d).expect("walks");
    assert!(p.is_clean());
    assert_eq!(p.iend_offset(), Some(51));
}

#[test]
fn after_idat_flag() {
    let d = png(&[
        ihdr(),
        chunk(b"tEXt", b"a"),
        idat(),
        chunk(b"tEXt", b"b"),
        idat(),
        iend(),
    ]);
    let p = Png::parse(&d).expect("walks");
    let flags: Vec<bool> = p.chunks().iter().map(|c| c.after_idat()).collect();
    assert_eq!(flags, vec![false, false, false, true, true, true]);
    // No IDAT at all: never after.
    let d2 = png(&[ihdr(), chunk(b"tEXt", b"a"), iend()]);
    let p2 = Png::parse(&d2).expect("walks");
    assert!(p2.chunks().iter().all(|c| !c.after_idat()));
}

#[test]
fn payloads_borrow_the_input() {
    let d = png(&[
        ihdr(),
        chunk(b"eXIf", &exif_payload()),
        idat(),
        iend(),
        vec![9, 9, 9],
    ]);
    let p = Png::parse(&d).expect("walks");
    for c in p.chunks() {
        assert!(inside(&d, c.data()), "chunk at {}", c.offset());
        assert_eq!(&d[c.range()][8..8 + c.data().len()], c.data());
    }
    assert!(inside(&d, p.trailer()));
    assert_eq!(
        p.trailer().as_ptr() as usize - d.as_ptr() as usize,
        d.len() - 3
    );
    // Empty trailer is still a subslice (the empty tail of the input).
    let m = minimal();
    let p = Png::parse(&m).expect("walks");
    assert!(inside(&m, p.trailer()));
}

#[test]
fn chunk_cap_is_exact() {
    // IHDR + 65,534 text chunks + IEND = 65,536 chunks: clean.
    let d = png(&[ihdr(), empty_texts(MAX_PNG_CHUNKS - 2), iend()]);
    let p = Png::parse(&d).expect("walks");
    assert!(p.is_clean(), "{:?}", p.issues().first());
    assert_eq!(p.chunks().len(), MAX_PNG_CHUNKS);
    assert!(p.iend_offset().is_some());
    // One more chunk: exactly MAX recorded and one TooManyChunks at the 65,537th.
    let d = png(&[ihdr(), empty_texts(MAX_PNG_CHUNKS - 1), iend()]);
    let p = Png::parse(&d).expect("walks");
    assert_eq!(p.chunks().len(), MAX_PNG_CHUNKS);
    assert_eq!(
        errors(&p),
        vec![Error::TooManyChunks {
            limit: MAX_PNG_CHUNKS
        }]
    );
    assert_eq!(offsets(&p), vec![8 + 25 + 12 * (MAX_PNG_CHUNKS - 1)]);
    assert_eq!(p.iend_offset(), None); // no MissingIend either
                                       // 65,537 empty tEXt chunks, no IEND.
    let d = png(&[ihdr(), empty_texts(MAX_PNG_CHUNKS + 1)]);
    let p = Png::parse(&d).expect("walks");
    assert_eq!(p.chunks().len(), MAX_PNG_CHUNKS);
    assert_eq!(p.issues().len(), 1);
}

#[test]
fn issue_cap_is_exact() {
    // Exactly MAX ordinary issues: all kept, no cap issue.
    let d = png(&[ihdr(), bad_crc_texts(MAX_PNG_ISSUES), iend()]);
    let p = Png::parse(&d).expect("walks");
    assert_eq!(p.issues().len(), MAX_PNG_ISSUES);
    assert!(p
        .issues()
        .iter()
        .all(|i| matches!(i.error(), Error::BadChunkCrc { .. })));
    assert!(p.iend_offset().is_some());
    // One more: MAX issues plus a single terminal TooManyIssues, walk halted.
    let d = png(&[ihdr(), bad_crc_texts(MAX_PNG_ISSUES + 1), iend()]);
    let p = Png::parse(&d).expect("walks");
    assert_eq!(p.issues().len(), MAX_PNG_ISSUES + 1);
    assert_eq!(
        p.issues().last().map(|i| i.error().clone()),
        Some(Error::TooManyIssues {
            limit: MAX_PNG_ISSUES
        })
    );
    assert_eq!(p.chunks().len(), 1 + MAX_PNG_ISSUES);
    assert_eq!(p.iend_offset(), None);
    // Far past the cap: still one terminal issue.
    let d = png(&[ihdr(), bad_crc_texts(MAX_PNG_ISSUES + 500), iend()]);
    let p = Png::parse(&d).expect("walks");
    assert_eq!(p.issues().len(), MAX_PNG_ISSUES + 1);
}

#[test]
fn parse_strict_ok_iff_clean() {
    for (name, d) in seeds() {
        match Png::parse(&d) {
            Err(e) => assert_eq!(Png::parse_strict(&d).err(), Some(e), "{name}"),
            Ok(p) => match Png::parse_strict(&d) {
                Ok(_) => assert!(p.is_clean(), "{name}"),
                Err(e) => {
                    assert!(!p.is_clean(), "{name}");
                    assert_eq!(Some(&e), p.issues().first().map(|i| i.error()), "{name}");
                }
            },
        }
    }
}

#[test]
fn every_prefix_and_every_byte_flip_of_each_shape_is_panic_free() {
    assert!(Png::parse(&[]).is_err());
    for (name, d) in seeds() {
        // The two big issue-cap seeds are sampled, not exhaustive.
        let step = (d.len() / 300).max(1);
        let mut n = 0;
        while n <= d.len() {
            let _ = Png::parse(&d[..n]);
            n += step;
        }
        let mut i = 0;
        while i < d.len() {
            for flip in [0xFFu8, 0x01, 0x80] {
                let mut m = d.clone();
                m[i] ^= flip;
                let _ = Png::parse(&m);
            }
            i += step;
        }
        let _ = name;
    }
}

#[test]
fn every_seed_is_what_its_name_says() {
    let all: std::collections::BTreeMap<_, _> = seeds().into_iter().collect();
    assert!(all.len() >= 24, "{} seeds", all.len());
    let clean = |n: &str| Png::parse(&all[n]).map(|p| p.is_clean()).unwrap_or(false);
    assert!(clean("minimal"));
    assert!(clean("private-chunk-type"));
    assert!(clean("trailer-after-iend"));
    assert!(clean("chunk-after-iend"));
    assert!(clean("exif-chunk") && clean("exif-after-idat"));
    assert!(clean("text-chunks") && clean("iccp-and-plte"));
    assert!(clean("many-idat") && clean("iend-with-data") && clean("empty-texts-100"));
    assert!(Png::parse(&all["bad-signature"]).is_err());
    assert!(Png::parse(&all["truncated-signature"]).is_err());
    for n in [
        "signature-only",
        "iend-only",
        "ihdr-only",
        "no-iend",
        "no-ihdr",
        "ihdr-len-12",
        "ihdr-len-14",
    ] {
        let p = Png::parse(&all[n]).expect("walks");
        assert!(!p.is_clean(), "{n}");
    }
    // Every seed that is a PNG at all has its chunk offsets strictly rising.
    for (n, d) in &all {
        if let Ok(p) = Png::parse(d) {
            let mut last = 0;
            for c in p.chunks() {
                assert!(c.offset() >= last.max(8), "{n}");
                last = c.range().end;
            }
        }
    }
}
