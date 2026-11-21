//! Fixtures for the PNG chunk walker (SPEC-028, DEC-017): one builder set,
//! shared by `tests/png_walk.rs`, `tests/fuzz_layout.rs` and the seed
//! generator (`examples/fuzz_seeds.rs`), so the seeds cannot drift from the
//! tests. No import from `src/`: the CRC here is the bitwise algorithm, so the
//! expected values are independent of the library's table.

#![allow(dead_code)]

/// The eight-byte PNG signature.
pub const SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// CRC-32 (IEEE 802.3, reflected) of `bytes`, bit by bit.
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut c: u32 = 0xFFFF_FFFF;
    for &b in bytes {
        c ^= u32::from(b);
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
    }
    !c
}

/// A chunk with a correct CRC and a correct length.
pub fn chunk(ty: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut crc_input = ty.to_vec();
    crc_input.extend_from_slice(data);
    chunk_with_crc(ty, data, crc32(&crc_input))
}

/// A chunk with a correct length and the given stored CRC.
pub fn chunk_with_crc(ty: &[u8; 4], data: &[u8], crc: u32) -> Vec<u8> {
    raw_chunk(data.len() as u32, ty, data, crc)
}

/// A chunk whose declared length is `declared`, whatever `data` holds.
pub fn raw_chunk(declared: u32, ty: &[u8; 4], data: &[u8], crc: u32) -> Vec<u8> {
    let mut v = declared.to_be_bytes().to_vec();
    v.extend_from_slice(ty);
    v.extend_from_slice(data);
    v.extend_from_slice(&crc.to_be_bytes());
    v
}

/// The signature followed by `parts`, in order.
pub fn png(parts: &[Vec<u8>]) -> Vec<u8> {
    let mut v = SIG.to_vec();
    for p in parts {
        v.extend_from_slice(p);
    }
    v
}

/// A 13-byte IHDR payload: 1x1, 8-bit grayscale.
pub fn ihdr_data() -> Vec<u8> {
    let mut d = Vec::new();
    d.extend_from_slice(&1u32.to_be_bytes());
    d.extend_from_slice(&1u32.to_be_bytes());
    d.extend_from_slice(&[8, 0, 0, 0, 0]);
    d
}

pub fn ihdr() -> Vec<u8> {
    chunk(b"IHDR", &ihdr_data())
}

pub fn idat() -> Vec<u8> {
    chunk(b"IDAT", &[0x78, 0x01, 0x63, 0x60, 0x00, 0x00])
}

pub fn iend() -> Vec<u8> {
    chunk(b"IEND", &[])
}

/// IHDR, IDAT, IEND: the smallest clean PNG (the three chunks are 25, 18 and
/// 12 bytes, so they start at offsets 8, 33 and 51 and the file is 63 bytes).
pub fn minimal() -> Vec<u8> {
    png(&[ihdr(), idat(), iend()])
}

pub const MINIMAL_OFFSETS: [usize; 3] = [8, 33, 51];

/// A small, valid-looking TIFF payload for an `eXIf` chunk.
pub fn exif_payload() -> Vec<u8> {
    let mut d = b"MM\x00\x2A\x00\x00\x00\x08".to_vec();
    d.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
    d
}

/// `n` empty `tEXt` chunks (12 bytes each).
pub fn empty_texts(n: usize) -> Vec<u8> {
    let one = chunk(b"tEXt", &[]);
    let mut v = Vec::with_capacity(one.len().saturating_mul(n));
    for _ in 0..n {
        v.extend_from_slice(&one);
    }
    v
}

/// `n` empty `tEXt` chunks, each with a wrong stored CRC.
pub fn bad_crc_texts(n: usize) -> Vec<u8> {
    let one = chunk_with_crc(b"tEXt", &[], 0xDEAD_BEEF);
    let mut v = Vec::with_capacity(one.len().saturating_mul(n));
    for _ in 0..n {
        v.extend_from_slice(&one);
    }
    v
}

// ── SPEC-029 builders (DEC-018). Compressed bodies may be any bytes: the
// library never inflates, so the builders never compress.

/// An `eXIf` chunk with the given payload.
pub fn exif_chunk(payload: &[u8]) -> Vec<u8> {
    chunk(b"eXIf", payload)
}

/// A TIFF payload (big-endian) with one IFD0 entry: Make (0x010F), ASCII,
/// count 4, inline `abc\0`.
pub fn exif_with_make() -> Vec<u8> {
    let mut d = b"MM\x00\x2A\x00\x00\x00\x08".to_vec();
    d.extend_from_slice(&[0, 1, 0x01, 0x0F, 0, 2, 0, 0, 0, 4]);
    d.extend_from_slice(b"abc\0");
    d.extend_from_slice(&[0, 0, 0, 0]);
    d
}

/// A `tEXt` chunk: keyword, NUL, value.
pub fn text(keyword: &[u8], value: &[u8]) -> Vec<u8> {
    let mut d = keyword.to_vec();
    d.push(0);
    d.extend_from_slice(value);
    chunk(b"tEXt", &d)
}

/// A `zTXt` chunk: keyword, NUL, method 0, then `body` (not inflated).
pub fn ztxt(keyword: &[u8], body: &[u8]) -> Vec<u8> {
    let mut d = keyword.to_vec();
    d.extend_from_slice(&[0, 0]);
    d.extend_from_slice(body);
    chunk(b"zTXt", &d)
}

/// An `iTXt` chunk: keyword, NUL, flag, method 0, language, NUL, translated
/// keyword, NUL, then `text`.
pub fn itxt(keyword: &[u8], flag: u8, lang: &[u8], translated: &[u8], text: &[u8]) -> Vec<u8> {
    let mut d = keyword.to_vec();
    d.extend_from_slice(&[0, flag, 0]);
    d.extend_from_slice(lang);
    d.push(0);
    d.extend_from_slice(translated);
    d.push(0);
    d.extend_from_slice(text);
    chunk(b"iTXt", &d)
}

/// An `iCCP` chunk: name, NUL, method, then `body` (not inflated).
pub fn iccp(name: &[u8], method: u8, body: &[u8]) -> Vec<u8> {
    let mut d = name.to_vec();
    d.extend_from_slice(&[0, method]);
    d.extend_from_slice(body);
    chunk(b"iCCP", &d)
}

/// An ImageMagick-style raw-profile `tEXt` chunk (the text is not hex-decoded).
pub fn raw_profile(name: &str, body: &[u8]) -> Vec<u8> {
    let mut k = b"Raw profile type ".to_vec();
    k.extend_from_slice(name.as_bytes());
    text(&k, body)
}

/// The committed `meta-` seeds of `png_chunks` (SPEC-029).
pub fn meta_seeds() -> Vec<(&'static str, Vec<u8>)> {
    let ex = exif_payload();
    let mut prefixed = b"Exif\0\0".to_vec();
    prefixed.extend_from_slice(&ex);
    let mut kw80 = vec![b'a'; 80];
    kw80.extend_from_slice(b"\0x");
    let mut itxt_bad_flag = b"Title\0".to_vec();
    itxt_bad_flag.extend_from_slice(&[2, 0, 0, 0, b'x']);
    let wrap = |parts: Vec<Vec<u8>>| {
        let mut all = vec![ihdr()];
        all.extend(parts);
        all.push(idat());
        all.push(iend());
        png(&all)
    };
    vec![
        ("meta-exif-before-idat", wrap(vec![exif_chunk(&ex)])),
        (
            "meta-exif-with-make",
            wrap(vec![exif_chunk(&exif_with_make())]),
        ),
        ("meta-exif-prefix", wrap(vec![exif_chunk(&prefixed)])),
        ("meta-exif-empty", wrap(vec![exif_chunk(&[])])),
        ("meta-exif-garbage", wrap(vec![exif_chunk(b"not a tiff")])),
        (
            "meta-exif-two",
            wrap(vec![exif_chunk(&ex), exif_chunk(&exif_with_make())]),
        ),
        (
            "meta-exif-after-iend",
            png(&[ihdr(), idat(), iend(), exif_chunk(&ex)]),
        ),
        (
            "meta-xmp-itxt",
            wrap(vec![itxt(b"XML:com.adobe.xmp", 0, b"", b"", b"<x/>")]),
        ),
        (
            "meta-xmp-itxt-compressed",
            wrap(vec![itxt(
                b"XML:com.adobe.xmp",
                1,
                b"en",
                b"t",
                &[0x78, 1, 2],
            )]),
        ),
        (
            "meta-xmp-text",
            wrap(vec![text(b"XML:com.adobe.xmp", b"<x/>")]),
        ),
        (
            "meta-xmp-wrong-keyword",
            wrap(vec![text(b"Comment", b"<x/>")]),
        ),
        (
            "meta-xmp-keyword-case",
            wrap(vec![text(b"xml:com.adobe.xmp", b"<x/>")]),
        ),
        (
            "meta-raw-exif-text",
            wrap(vec![raw_profile("exif", b"\n45786966\n")]),
        ),
        (
            "meta-raw-xmp-ztxt",
            wrap(vec![ztxt(b"Raw profile type xmp", &[0x78, 1, 2, 3])]),
        ),
        (
            "meta-raw-unknown-itxt",
            wrap(vec![itxt(b"Raw profile type foo", 1, b"", b"", &[9, 9])]),
        ),
        (
            "meta-text-plain",
            wrap(vec![
                text(b"Author", b"someone"),
                ztxt(b"Title", &[0x78, 1]),
            ]),
        ),
        (
            "meta-itxt-lang",
            wrap(vec![itxt(b"Title", 0, b"fr", b"Titre", b"bonjour")]),
        ),
        ("meta-text-no-nul", wrap(vec![chunk(b"tEXt", b"Comment")])),
        (
            "meta-text-empty-keyword",
            wrap(vec![chunk(b"tEXt", b"\0value")]),
        ),
        ("meta-text-keyword-80", wrap(vec![chunk(b"tEXt", &kw80)])),
        (
            "meta-itxt-bad-flag",
            wrap(vec![chunk(b"iTXt", &itxt_bad_flag)]),
        ),
        (
            "meta-itxt-truncated",
            wrap(vec![chunk(b"iTXt", b"Title\0\0\0en")]),
        ),
        (
            "meta-ztxt-no-method",
            wrap(vec![chunk(b"zTXt", b"Title\0")]),
        ),
        ("meta-iccp", wrap(vec![iccp(b"sRGB", 0, &[0x78, 1, 2])])),
        (
            "meta-iccp-empty-name",
            wrap(vec![chunk(b"iCCP", b"\0\0abc")]),
        ),
        ("meta-iccp-no-method", wrap(vec![chunk(b"iCCP", b"sRGB\0")])),
        (
            "meta-trailer-chunks",
            png(&[
                ihdr(),
                idat(),
                iend(),
                text(b"Comment", b"late"),
                itxt(b"XML:com.adobe.xmp", 0, b"", b"", b"<y/>"),
                iccp(b"p", 0, &[1]),
                exif_chunk(&ex),
            ]),
        ),
        (
            "meta-trailer-garbage",
            png(&[ihdr(), idat(), iend(), vec![0xFF; 40]]),
        ),
        (
            "meta-trailer-truncated",
            png(&[
                ihdr(),
                idat(),
                iend(),
                text(b"Comment", b"ok"),
                raw_chunk(100, b"tEXt", b"Comment\0cut", 0),
            ]),
        ),
        (
            "meta-after-idat",
            png(&[
                ihdr(),
                idat(),
                text(b"Comment", b"mid"),
                exif_chunk(&ex),
                iend(),
            ]),
        ),
        (
            "meta-halted-after-text",
            png(&[
                ihdr(),
                text(b"Comment", b"kept"),
                chunk(b"tEXt", b"bad"),
                raw_chunk(0x8000_0000, b"IDAT", b"", 0),
            ]),
        ),
        (
            "meta-ztxt-xmp",
            wrap(vec![ztxt(b"XML:com.adobe.xmp", &[0x78, 1, 2])]),
        ),
        (
            "meta-raw-lowercase",
            wrap(vec![
                text(b"xml:com.adobe.xmp", b"<x/>"),
                raw_profile_lowercase(),
            ]),
        ),
        ("meta-long-keyword", wrap(vec![long_raw_profile()])),
        (
            "meta-itxt-flag-2",
            wrap(vec![chunk(b"iTXt", &itxt_xmp_flag_2())]),
        ),
    ]
}

/// A `tEXt` chunk whose keyword is `raw profile type exif` in lowercase.
fn raw_profile_lowercase() -> Vec<u8> {
    text(b"raw profile type exif", b"hex")
}

/// A raw-profile `tEXt` chunk whose keyword is 87 bytes long.
fn long_raw_profile() -> Vec<u8> {
    raw_profile(&"x".repeat(70), b"hex")
}

/// An `iTXt` payload for XMP with the compression flag set to 2.
fn itxt_xmp_flag_2() -> Vec<u8> {
    let mut d = b"XML:com.adobe.xmp\0".to_vec();
    d.extend_from_slice(&[2, 0, 0, 0]);
    d.extend_from_slice(&[0x78, 1, 2]);
    d
}

/// The committed `png_chunks` seeds, name and bytes, in a stable order: the
/// walker seeds (SPEC-028), then the `meta-` seeds (SPEC-029).
pub fn seeds() -> Vec<(&'static str, Vec<u8>)> {
    let mut all = walk_seeds();
    all.extend(meta_seeds());
    all
}

fn walk_seeds() -> Vec<(&'static str, Vec<u8>)> {
    let text = chunk(b"tEXt", b"Comment\0hello");
    let mut ihdr_short = ihdr_data();
    ihdr_short.pop();
    let mut ihdr_long = ihdr_data();
    ihdr_long.push(0);
    let mut bad_sig = SIG.to_vec();
    if let Some(last) = bad_sig.last_mut() {
        *last = 0x0B;
    }
    vec![
        ("minimal", minimal()),
        ("signature-only", SIG.to_vec()),
        ("truncated-signature", SIG[..4].to_vec()),
        ("bad-signature", bad_sig),
        ("ihdr-only", png(&[ihdr()])),
        ("iend-only", png(&[iend()])),
        ("no-iend", png(&[ihdr(), idat()])),
        ("no-ihdr", png(&[text.clone(), idat(), iend()])),
        (
            "ihdr-len-12",
            png(&[chunk(b"IHDR", &ihdr_short), idat(), iend()]),
        ),
        (
            "ihdr-len-14",
            png(&[chunk(b"IHDR", &ihdr_long), idat(), iend()]),
        ),
        ("two-ihdr", png(&[ihdr(), ihdr(), idat(), iend()])),
        (
            "bad-crc-ihdr",
            png(&[chunk_with_crc(b"IHDR", &ihdr_data(), 0), idat(), iend()]),
        ),
        (
            "bad-crc-idat",
            png(&[
                ihdr(),
                chunk_with_crc(b"IDAT", &[1, 2, 3], 1),
                text.clone(),
                iend(),
            ]),
        ),
        (
            "bad-chunk-type",
            png(&[ihdr(), chunk(b"te1t", b"x"), idat(), iend()]),
        ),
        (
            "private-chunk-type",
            png(&[ihdr(), chunk(b"prVt", b"x"), idat(), iend()]),
        ),
        (
            "length-past-eof",
            png(&[ihdr(), raw_chunk(100, b"tEXt", b"abc", 0)]),
        ),
        (
            "length-80000000",
            png(&[ihdr(), raw_chunk(0x8000_0000, b"tEXt", b"abc", 0)]),
        ),
        (
            "length-ffffffff",
            png(&[ihdr(), raw_chunk(0xFFFF_FFFF, b"IDAT", b"abc", 0)]),
        ),
        (
            "length-7fffffff-past-eof",
            png(&[ihdr(), raw_chunk(0x7FFF_FFFF, b"IDAT", b"abc", 0)]),
        ),
        (
            "tail-11-bytes",
            png(&[
                ihdr(),
                idat(),
                vec![0, 0, 0, 0, b'I', b'E', b'N', b'D', 0, 0, 0],
            ]),
        ),
        ("tail-1-byte", png(&[ihdr(), idat(), vec![0]])),
        (
            "trailer-after-iend",
            png(&[ihdr(), idat(), iend(), vec![1, 2, 3, 4, 5]]),
        ),
        (
            "chunk-after-iend",
            png(&[ihdr(), idat(), iend(), chunk(b"eXIf", &exif_payload())]),
        ),
        (
            "exif-chunk",
            png(&[ihdr(), chunk(b"eXIf", &exif_payload()), idat(), iend()]),
        ),
        (
            "exif-after-idat",
            png(&[ihdr(), idat(), chunk(b"eXIf", &exif_payload()), iend()]),
        ),
        (
            "text-chunks",
            png(&[
                ihdr(),
                chunk(b"tEXt", b"Author\0someone"),
                chunk(b"zTXt", b"Title\0\0\x78\x01"),
                chunk(b"iTXt", b"XML:com.adobe.xmp\0\0\0\0\0<x/>"),
                idat(),
                iend(),
            ]),
        ),
        (
            "iccp-and-plte",
            png(&[
                ihdr(),
                chunk(b"iCCP", b"sRGB\0\0\x78\x01"),
                chunk(b"PLTE", &[0, 0, 0, 255, 255, 255]),
                idat(),
                iend(),
            ]),
        ),
        (
            "many-idat",
            png(&[ihdr(), idat(), idat(), idat(), text.clone(), idat(), iend()]),
        ),
        (
            "iend-with-data",
            png(&[ihdr(), idat(), chunk(b"IEND", &[1, 2, 3])]),
        ),
        ("empty-texts-100", png(&[ihdr(), empty_texts(100), iend()])),
        (
            "bad-crc-texts-1100",
            png(&[ihdr(), bad_crc_texts(1100), iend()]),
        ),
        (
            "crc-vector",
            png(&[ihdr(), chunk_with_crc(b"1234", b"56789", 0), iend()]),
        ),
        (
            "everything-wrong",
            png(&[
                chunk_with_crc(b"1234", b"x", 7),
                chunk(b"IHDR", &ihdr_long),
                raw_chunk(0xFFFF_FFFF, b"IDAT", b"", 0),
            ]),
        ),
    ]
}
