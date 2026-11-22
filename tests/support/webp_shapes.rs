//! Fixtures for the WebP RIFF walker (SPEC-030, DEC-019): one builder set,
//! shared by `tests/webp_walk.rs`, `tests/webp_metadata.rs`,
//! `tests/fuzz_layout.rs` and the seed generator (`examples/fuzz_seeds.rs`), so
//! the seeds cannot drift from the tests. No import from `src/`: every offset
//! and size here is computed by the builders, never by the library.

#![allow(dead_code)]

/// The 12-byte header of a WebP whose RIFF size field says `size`.
pub fn header(size: u32) -> Vec<u8> {
    let mut v = b"RIFF".to_vec();
    v.extend_from_slice(&size.to_le_bytes());
    v.extend_from_slice(b"WEBP");
    v
}

/// A chunk with its data and, when the size is odd, one zero pad byte.
pub fn chunk(code: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut v = chunk_unpadded(code, data);
    if data.len() % 2 == 1 {
        v.push(0);
    }
    v
}

/// A chunk with its data and never a pad byte, even when the size is odd.
pub fn chunk_unpadded(code: &[u8; 4], data: &[u8]) -> Vec<u8> {
    raw_chunk(code, data.len() as u32, data)
}

/// A chunk whose declared size is `declared`, whatever `data` holds, no pad.
pub fn raw_chunk(code: &[u8; 4], declared: u32, data: &[u8]) -> Vec<u8> {
    let mut v = code.to_vec();
    v.extend_from_slice(&declared.to_le_bytes());
    v.extend_from_slice(data);
    v
}

/// A WebP whose RIFF size field is exact (the body plus the four `WEBP` bytes).
pub fn riff(parts: &[Vec<u8>]) -> Vec<u8> {
    let body: usize = parts.iter().map(Vec::len).sum();
    riff_with_size((body + 4) as u32, parts)
}

/// A WebP whose RIFF size field says `declared`, whatever the body is.
pub fn riff_with_size(declared: u32, parts: &[Vec<u8>]) -> Vec<u8> {
    let mut v = header(declared);
    for p in parts {
        v.extend_from_slice(p);
    }
    v
}

/// A `VP8X` chunk: flags, three reserved bytes, canvas width and height, each
/// stored minus one in 24 bits little-endian.
pub fn vp8x(flags: u8, width: u32, height: u32) -> Vec<u8> {
    let mut d = vec![flags, 0, 0, 0];
    d.extend_from_slice(&(width - 1).to_le_bytes()[..3]);
    d.extend_from_slice(&(height - 1).to_le_bytes()[..3]);
    chunk(b"VP8X", &d)
}

/// `VP8X` flag bits: ICC profile, alpha, Exif, XMP, animation.
pub const FLAG_ICC: u8 = 0x20;
pub const FLAG_ALPHA: u8 = 0x10;
pub const FLAG_EXIF: u8 = 0x08;
pub const FLAG_XMP: u8 = 0x04;
pub const FLAG_ANIM: u8 = 0x02;

/// A minimal `VP8L` chunk (the lossless signature byte and four zero bytes).
pub fn vp8l() -> Vec<u8> {
    chunk(b"VP8L", &[0x2F, 0, 0, 0, 0])
}

/// A small TIFF payload (big-endian, an empty IFD0).
pub fn tiff_empty() -> Vec<u8> {
    let mut d = b"MM\x00\x2A\x00\x00\x00\x08".to_vec();
    d.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
    d
}

/// The smallest clean WebP: a simple lossless file (`VP8L` only).
pub fn minimal() -> Vec<u8> {
    riff(&[vp8l()])
}

/// `n` empty `JUNK` chunks (8 bytes each).
pub fn empty_junk(n: usize) -> Vec<u8> {
    let one = chunk(b"JUNK", &[]);
    let mut v = Vec::with_capacity(one.len().saturating_mul(n));
    for _ in 0..n {
        v.extend_from_slice(&one);
    }
    v
}

/// A `VP8X` header chunk followed by `n` more `VP8X` chunks: every one after
/// the first is misplaced, so each costs one issue.
pub fn misplaced_vp8x(n: usize) -> Vec<Vec<u8>> {
    let mut parts = vec![vp8x(0, 1, 1)];
    for _ in 0..n {
        parts.push(vp8x(0, 1, 1));
    }
    parts
}

/// The walk seeds (SPEC-030): the DEC-019 probe shapes for the RIFF framing.
/// Names are pinned in `tests/fuzz_layout.rs`.
pub fn walk_seeds() -> Vec<(&'static str, Vec<u8>)> {
    let exif = chunk(b"EXIF", &tiff_empty());
    let xmp = chunk(b"XMP ", b"<x/>");
    let mut avi = b"RIFF".to_vec();
    avi.extend_from_slice(&4u32.to_le_bytes());
    avi.extend_from_slice(b"AVI ");
    let mut with_junk = riff(std::slice::from_ref(&exif));
    with_junk.extend_from_slice(&chunk(b"JUNK", &[1, 2]));
    let past_end = {
        let mut d = riff(std::slice::from_ref(&exif));
        d.extend_from_slice(&raw_chunk(b"XMP ", 1000, b"abc"));
        d
    };
    let tail = |n: usize| {
        let mut d = riff(std::slice::from_ref(&exif));
        d.extend_from_slice(&vec![b'A'; n]);
        d
    };
    let vp8x_three = vec![0x08, 0, 0];
    vec![
        ("minimal", minimal()),
        ("header-only", header(4)),
        ("short-11", header(4)[..11].to_vec()),
        ("bad-magic", {
            let mut d = minimal();
            d[0] = b'X';
            d
        }),
        ("form-avi", avi),
        ("size-exact", riff(std::slice::from_ref(&exif))),
        ("size-small", riff_with_size(4, std::slice::from_ref(&exif))),
        ("size-zero", riff_with_size(0, std::slice::from_ref(&exif))),
        ("size-odd", riff_with_size(7, std::slice::from_ref(&exif))),
        (
            "size-large",
            riff_with_size(0xFFFF_FFF0, std::slice::from_ref(&exif)),
        ),
        ("size-trailing", with_junk),
        (
            "pad-present",
            riff(&[chunk(b"JUNK", &[1]), exif.clone(), xmp.clone()]),
        ),
        (
            "pad-missing-end",
            riff(&[chunk_unpadded(b"JUNK", &[1, 2, 3])]),
        ),
        (
            "pad-missing-middle",
            riff(&[chunk_unpadded(b"JUNK", &[1]), xmp.clone()]),
        ),
        ("chunk-past-end", past_end),
        ("tail-1-byte", tail(1)),
        ("tail-7-bytes", tail(7)),
        ("tail-partial-header", {
            let mut d = riff(std::slice::from_ref(&exif));
            d.extend_from_slice(b"XMP \x01");
            d
        }),
        ("kind-exif-lower", riff(&[chunk(b"exif", &tiff_empty())])),
        ("kind-exif-mixed", riff(&[chunk(b"Exif", &tiff_empty())])),
        ("kind-xmp-lower", riff(&[chunk(b"xmp ", b"<x/>")])),
        ("kind-xmp-dash", riff(&[chunk(b"XMP-", b"<x/>")])),
        ("kind-xmp-nul", riff(&[chunk(b"XMP\0", b"<x/>")])),
        ("kind-iccp-lower", riff(&[chunk(b"iccp", b"icc!")])),
        ("vp8x-full", riff(&[vp8x(0x3E, 640, 480), vp8l()])),
        (
            "vp8x-three-bytes",
            riff(&[chunk(b"VP8X", &vp8x_three), exif.clone()]),
        ),
        ("vp8x-misplaced", riff(&[vp8l(), vp8x(0, 1, 1)])),
        // SPEC-034 carried gap (a): the FIRST `VP8X` is the header; a later
        // one (flags 0x08) is misplaced and must not replace it.
        (
            "vp8x-first-wins",
            riff(&[vp8x(0, 1, 1), vp8x(FLAG_EXIF, 1, 1), vp8l(), exif.clone()]),
        ),
        // SPEC-034 carried gap (b): a 9-byte `VP8X` is one byte short of the
        // 10 the header needs (`BadVp8x`), where 10 is fine.
        (
            "vp8x-nine-bytes",
            riff(&[
                chunk(b"VP8X", &[FLAG_EXIF, 0, 0, 0, 0, 0, 0, 0, 0]),
                exif.clone(),
            ]),
        ),
        (
            "flags-clear-with-exif",
            riff(&[vp8x(0, 1, 1), vp8l(), exif.clone()]),
        ),
        (
            "flags-set-no-chunks",
            riff(&[vp8x(FLAG_ICC | FLAG_EXIF | FLAG_XMP, 1, 1), vp8l()]),
        ),
        ("no-vp8x-with-exif", riff(&[vp8l(), exif.clone()])),
        (
            "exif-before-vp8x",
            riff(&[exif.clone(), vp8x(FLAG_EXIF, 1, 1)]),
        ),
        (
            "anmf-payload",
            riff(&[
                vp8x(FLAG_ANIM | FLAG_EXIF, 2, 2),
                chunk(b"ANIM", &[0, 0, 0, 0, 0, 0]),
                chunk(b"ANMF", &exif),
            ]),
        ),
        (
            "exif-after-anmf",
            riff(&[
                vp8x(FLAG_ANIM | FLAG_EXIF, 2, 2),
                chunk(b"ANMF", &[0u8; 16]),
                exif.clone(),
            ]),
        ),
        ("junk-chunks-100", riff(&[empty_junk(100)])),
        ("misplaced-vp8x-1100", riff(&misplaced_vp8x(1100))),
    ]
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

/// An `EXIF` chunk with the given payload.
pub fn exif_chunk(payload: &[u8]) -> Vec<u8> {
    chunk(b"EXIF", payload)
}

/// An `Exif\0\0` header followed by `payload`.
pub fn with_exif_prefix(payload: &[u8]) -> Vec<u8> {
    let mut v = b"Exif\0\0".to_vec();
    v.extend_from_slice(payload);
    v
}

/// The metadata seeds (SPEC-030), all prefixed `meta-`: the DEC-019 metadata
/// rules, extraction half.
pub fn meta_seeds() -> Vec<(&'static str, Vec<u8>)> {
    let make = exif_with_make();
    let all_flags = FLAG_ICC | FLAG_EXIF | FLAG_XMP;
    let seventeen: Vec<Vec<u8>> = std::iter::once(vp8x(FLAG_EXIF, 1, 1))
        .chain((0..17).map(|_| exif_chunk(&tiff_empty())))
        .collect();
    let mut halted = riff(&[
        exif_chunk(&with_exif_prefix(&tiff_empty())),
        exif_chunk(&make),
        chunk(b"XMP ", b"<x/>"),
    ]);
    halted.extend_from_slice(b"EXIF\x10");
    vec![
        (
            "meta-exif",
            riff(&[vp8x(FLAG_EXIF, 1, 1), vp8l(), exif_chunk(&make)]),
        ),
        (
            "meta-exif-prefix",
            riff(&[vp8x(FLAG_EXIF, 1, 1), exif_chunk(&with_exif_prefix(&make))]),
        ),
        (
            "meta-exif-prefix-only",
            riff(&[vp8x(FLAG_EXIF, 1, 1), exif_chunk(b"Exif\0\0")]),
        ),
        (
            "meta-exif-empty",
            riff(&[vp8x(FLAG_EXIF, 1, 1), exif_chunk(&[])]),
        ),
        (
            "meta-exif-garbage",
            riff(&[vp8x(FLAG_EXIF, 1, 1), exif_chunk(b"not a tiff")]),
        ),
        (
            "meta-exif-two",
            riff(&[
                vp8x(FLAG_EXIF, 1, 1),
                exif_chunk(&tiff_empty()),
                exif_chunk(&make),
            ]),
        ),
        ("meta-exif-seventeen", riff(&seventeen)),
        (
            "meta-xmp",
            riff(&[
                vp8x(FLAG_XMP, 1, 1),
                vp8l(),
                chunk(b"XMP ", b"<x:xmpmeta/>"),
            ]),
        ),
        (
            "meta-xmp-odd",
            riff(&[vp8x(FLAG_XMP, 1, 1), chunk(b"XMP ", b"<x/>!")]),
        ),
        (
            "meta-xmp-two",
            riff(&[
                vp8x(FLAG_XMP, 1, 1),
                chunk(b"XMP ", b"<a/>"),
                chunk(b"XMP ", b"<b/>"),
            ]),
        ),
        (
            "meta-xmp-nul-id",
            riff(&[vp8x(FLAG_XMP, 1, 1), chunk(b"XMP\0", b"<x/>")]),
        ),
        (
            "meta-iccp",
            riff(&[vp8x(FLAG_ICC, 1, 1), chunk(b"ICCP", b"icc!"), vp8l()]),
        ),
        (
            "meta-all-three",
            riff(&[
                vp8x(all_flags, 8, 8),
                chunk(b"ICCP", b"icc!"),
                vp8l(),
                exif_chunk(&make),
                chunk(b"XMP ", b"<x/>"),
            ]),
        ),
        (
            "meta-flags-all-wrong",
            riff(&[
                vp8x(FLAG_EXIF, 1, 1),
                chunk(b"ICCP", b"icc!"),
                chunk(b"XMP ", b"<x/>"),
            ]),
        ),
        ("meta-halted-after-exifs", halted),
        (
            "meta-anmf-then-exif",
            riff(&[
                vp8x(FLAG_ANIM | FLAG_EXIF, 2, 2),
                chunk(b"ANMF", &exif_chunk(&make)),
                exif_chunk(&make),
            ]),
        ),
    ]
}

/// Every seed: the walk seeds, then the metadata seeds.
pub fn seeds() -> Vec<(&'static str, Vec<u8>)> {
    let mut all = walk_seeds();
    all.extend(meta_seeds());
    all
}
