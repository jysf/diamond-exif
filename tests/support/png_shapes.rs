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

/// The committed `png_chunks` seeds, name and bytes, in a stable order.
pub fn seeds() -> Vec<(&'static str, Vec<u8>)> {
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
