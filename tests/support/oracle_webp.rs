//! Hand-built WebP fixtures for the ExifTool parity oracle (SPEC-034).
//!
//! Tier A: every byte is written here from the WebP Container Specification
//! (Google) and the RIFF container format: the 12-byte header (`RIFF`, a
//! little-endian size, `WEBP`), chunks as a four-character code, a 4-byte
//! little-endian size, the data and one pad byte after odd data; `VP8X` (flags
//! and canvas), `VP8L`, `VP8 `, `ALPH`, `ANIM`, `ANMF`, `ICCP`, `EXIF`, `XMP `.
//! No code is shared with `src/`, with `examples/fuzz_seeds.rs`, with
//! `tests/support/webp_shapes.rs` or with the fuzz models: the oracle must not
//! share a bug with the thing it checks. The TIFF blobs come from `oracle_tiff`
//! and the ICC profile bytes from `oracle_png::icc_profile`; both are
//! oracle-layer files, independent of `src/`.
//!
//! The fixture list is a VARIANT GRID (every chunk code, size parity and length
//! class, every header field, every `VP8X` length), generated from one table in
//! [`shapes`]: SPEC-031 needed three fix rounds because its list was
//! hand-picked. [`fixtures`] is a pure function of this file and of
//! `oracle_tiff::fixtures()`; it is the single named list used by the recorder,
//! the live tests and the replay tests. std only.
//!
//! Names are normative (SPEC-034): `webp-<tiff fixture name>` for the wrapped
//! TIFFs and the `webp-...` shape names below. What ExifTool and `exif-core`
//! read from them is measured, never assumed
//! (`tests/oracle-fixtures/exiftool/webp-*.txt`).

#![allow(dead_code)]

use crate::oracle_png;
use crate::oracle_tiff as t;

const FLAG_ICC: u8 = 0x20;
const FLAG_EXIF: u8 = 0x08;
const FLAG_XMP: u8 = 0x04;
const FLAG_ANIM: u8 = 0x02;

/// A chunk: code, little-endian size, data, and one zero pad byte after odd data.
pub fn chunk(code: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut v = chunk_np(code, data);
    if data.len() % 2 == 1 {
        v.push(0);
    }
    v
}

/// A chunk with no pad byte, even after odd data.
pub fn chunk_np(code: &[u8; 4], data: &[u8]) -> Vec<u8> {
    raw(code, data.len() as u32, data)
}

/// A chunk whose size field says `declared`, whatever `data` holds; no pad.
pub fn raw(code: &[u8; 4], declared: u32, data: &[u8]) -> Vec<u8> {
    let mut v = code.to_vec();
    v.extend_from_slice(&declared.to_le_bytes());
    v.extend_from_slice(data);
    v
}

/// A file with a RIFF size field of `declared` and the given parts.
pub fn riff_sized(declared: u32, form: &[u8; 4], parts: &[Vec<u8>]) -> Vec<u8> {
    let mut v = b"RIFF".to_vec();
    v.extend_from_slice(&declared.to_le_bytes());
    v.extend_from_slice(form);
    for p in parts {
        v.extend_from_slice(p);
    }
    v
}

/// A WebP whose RIFF size is exact: the body plus the four `WEBP` bytes.
pub fn riff(parts: &[Vec<u8>]) -> Vec<u8> {
    let body: usize = parts.iter().map(Vec::len).sum();
    riff_sized((body + 4) as u32, b"WEBP", parts)
}

/// A `VP8X` chunk with `len` data bytes: flags, three reserved bytes, canvas
/// 1x1 (each side stored minus one), then zero fill or truncation to `len`.
pub fn vp8x_len(flags: u8, len: usize) -> Vec<u8> {
    let mut d = vec![flags, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    d.resize(len, 0);
    chunk(b"VP8X", &d)
}

pub fn vp8x(flags: u8) -> Vec<u8> {
    vp8x_len(flags, 10)
}

pub fn vp8l() -> Vec<u8> {
    chunk(b"VP8L", &[0x2F, 0, 0, 0, 0])
}

pub fn vp8() -> Vec<u8> {
    chunk(b"VP8 ", &[0u8; 10])
}

/// A well-formed WebP: `VP8X` with `flags`, `VP8L`, then `parts`.
pub fn file(flags: u8, parts: &[Vec<u8>]) -> Vec<u8> {
    let mut all = vec![vp8x(flags), vp8l()];
    all.extend_from_slice(parts);
    riff(&all)
}

fn named_tiff(name: &str) -> Vec<u8> {
    t::fixtures()
        .into_iter()
        .find(|(n, _)| *n == name)
        .map(|(_, b)| b)
        .unwrap_or_else(|| panic!("no TIFF fixture named {name}"))
}

/// A tiny TIFF whose IFD0 holds the given ASCII `(tag, text)` entries.
pub fn tiff_of(entries: &[(u16, &str)]) -> Vec<u8> {
    let ents: Vec<t::Ent> = entries
        .iter()
        .map(|(tag, text)| t::ascii(*tag, &[text.as_bytes(), b"\0"].concat()))
        .collect();
    t::assemble(false, &[t::ifd(ents)])
}

/// Block A: Make `AlphaCam` and Model `OnlyInA`. Block B: Make `BravoCam`.
pub fn tiff_a() -> Vec<u8> {
    tiff_of(&[(t::T_MAKE, "AlphaCam"), (t::T_MODEL, "OnlyInA")])
}

pub fn tiff_b() -> Vec<u8> {
    tiff_of(&[(t::T_MAKE, "BravoCam")])
}

/// Block A with one trailing byte if needed so its length is odd.
fn tiff_a_odd() -> Vec<u8> {
    let mut a = tiff_a();
    if a.len().is_multiple_of(2) {
        a.push(0);
    }
    a
}

fn exif(tiff: &[u8]) -> Vec<u8> {
    chunk(b"EXIF", tiff)
}

fn prefixed(tiff: &[u8]) -> Vec<u8> {
    [&b"Exif\0\0"[..], tiff].concat()
}

/// A minimal XMP packet carrying `xmp:CreatorTool` (shared with the PNG oracle).
fn packet() -> Vec<u8> {
    oracle_png::xmp_packet()
}

fn icc() -> Vec<u8> {
    oracle_png::icc_profile()
}

/// The `RIFF....WEBP` header for a `body` of the given length, size exact.
fn exact_size(body: &[Vec<u8>]) -> u32 {
    (body.iter().map(Vec::len).sum::<usize>() + 4) as u32
}

/// Every shape, as `(name without the webp- prefix, bytes)`, from one table.
fn shapes() -> Vec<(String, Vec<u8>)> {
    let a = tiff_a();
    let b = tiff_b();
    let ex = |t: &[u8]| exif(t);
    let x = chunk(b"XMP ", &packet());
    let icc_ok = chunk(b"ICCP", &icc());
    let anim = chunk(b"ANIM", &[0u8; 6]);
    let anmf = chunk(b"ANMF", &[0u8; 16]);
    let ef = FLAG_EXIF;
    let mut out: Vec<(String, Vec<u8>)> = Vec::new();
    let mut add = |name: &str, bytes: Vec<u8>| out.push((name.to_string(), bytes));

    // EXIF: the basics, the prefix, several blocks.
    add("exif-basic", file(ef, &[ex(&a)]));
    add("exif-prefix", file(ef, &[exif(&prefixed(&a))]));
    add("exif-two-same-tag", file(ef, &[ex(&a), ex(&b)]));
    add("exif-two-same-tag-reversed", file(ef, &[ex(&b), ex(&a)]));
    add("exif-bad-then-good", file(ef, &[ex(b"not a tiff"), ex(&a)]));
    add("exif-good-then-bad", file(ef, &[ex(&a), ex(b"not a tiff")]));
    add(
        "exif-good-then-prefixed",
        file(ef, &[ex(&a), exif(&prefixed(&b))]),
    );
    add("exif-empty", file(ef, &[ex(&[])]));
    add("exif-one-byte", file(ef, &[ex(&[0x4D])]));
    add("exif-garbage", file(ef, &[ex(b"not a tiff at all")]));
    add("exif-flag-unset", file(0, &[ex(&a)]));
    add("exif-no-vp8x", riff(&[vp8l(), ex(&a)]));
    add("exif-before-vp8x", riff(&[ex(&a), vp8x(ef), vp8l()]));
    add(
        "exif-after-anim",
        riff(&[vp8x(ef | FLAG_ANIM), anim.clone(), ex(&a)]),
    );
    add(
        "exif-inside-anmf",
        riff(&[vp8x(ef | FLAG_ANIM), anim.clone(), chunk(b"ANMF", &ex(&a))]),
    );
    add(
        "exif-anmf-then-top-level",
        riff(&[vp8x(ef | FLAG_ANIM), chunk(b"ANMF", &ex(&b)), ex(&a)]),
    );
    add(
        "animated-three-anmf",
        riff(&[
            vp8x(ef | FLAG_ANIM),
            anim.clone(),
            anmf.clone(),
            anmf.clone(),
            anmf.clone(),
            ex(&a),
        ]),
    );
    add(
        "exif-odd-padded",
        file(ef, &[chunk(b"EXIF", &tiff_a_odd())]),
    );
    add(
        "exif-odd-unpadded-last",
        file(ef, &[chunk_np(b"EXIF", &tiff_a_odd())]),
    );
    add(
        "exif-odd-unpadded-then-xmp",
        file(
            ef | FLAG_XMP,
            &[chunk_np(b"EXIF", &tiff_a_odd()), x.clone()],
        ),
    );
    add(
        "xmp-odd-unpadded-then-exif",
        file(ef | FLAG_XMP, &[chunk_np(b"XMP ", &odd_packet()), ex(&a)]),
    );
    add(
        "xmp-odd-unpadded-last",
        file(FLAG_XMP, &[chunk_np(b"XMP ", &odd_packet())]),
    );
    add(
        "xmp-odd-padded",
        file(FLAG_XMP, &[chunk(b"XMP ", &odd_packet())]),
    );
    // Chunk-code case variants.
    for (n, code) in [
        ("capital-e", b"Exif"),
        ("lower", b"exif"),
        ("last-letter-lower", b"EXIf"),
    ] {
        add(&format!("exif-kind-{n}"), file(ef, &[chunk(code, &a)]));
    }
    // XMP.
    add("xmp-basic", file(FLAG_XMP, std::slice::from_ref(&x)));
    add("xmp-nul-id", file(FLAG_XMP, &[chunk(b"XMP\0", &packet())]));
    add("xmp-empty", file(FLAG_XMP, &[chunk(b"XMP ", &[])]));
    add("xmp-nul-id-empty", file(FLAG_XMP, &[chunk(b"XMP\0", &[])]));
    for n in 1..=7usize {
        let data = packet().get(..n).unwrap_or_default().to_vec();
        add(
            &format!("xmp-nul-id-{n}-bytes"),
            file(FLAG_XMP, &[chunk(b"XMP\0", &data)]),
        );
    }
    add("xmp-two", file(FLAG_XMP, &[x.clone(), x.clone()]));
    add(
        "xmp-empty-then-real",
        file(FLAG_XMP, &[chunk(b"XMP ", &[]), x.clone()]),
    );
    add(
        "xmp-garbage",
        file(FLAG_XMP, &[chunk(b"XMP ", b"not xml at all, just bytes")]),
    );
    add(
        "xmp-kind-lower",
        file(FLAG_XMP, &[chunk(b"xmp ", &packet())]),
    );
    add(
        "xmp-kind-dash",
        file(FLAG_XMP, &[chunk(b"XMP-", &packet())]),
    );
    add("xmp-kind-x", file(FLAG_XMP, &[chunk(b"XMPx", &packet())]));
    // ICC.
    add(
        "icc-valid-flag-set",
        file(FLAG_ICC, std::slice::from_ref(&icc_ok)),
    );
    add(
        "icc-valid-flag-unset",
        file(0, std::slice::from_ref(&icc_ok)),
    );
    add(
        "icc-garbage",
        file(FLAG_ICC, &[chunk(b"ICCP", b"icc! not a profile")]),
    );
    add(
        "icc-stub-132",
        file(FLAG_ICC, &[chunk(b"ICCP", &[0u8; 132])]),
    );
    add("icc-empty", file(FLAG_ICC, &[chunk(b"ICCP", &[])]));
    add("icc-kind-lower", file(FLAG_ICC, &[chunk(b"iccp", &icc())]));
    add(
        "icc-valid-then-garbage",
        file(FLAG_ICC, &[icc_ok.clone(), chunk(b"ICCP", b"garbage!")]),
    );
    add(
        "icc-garbage-then-valid",
        file(FLAG_ICC, &[chunk(b"ICCP", b"garbage!"), icc_ok.clone()]),
    );
    add(
        "icc-valid-trailing-garbage",
        file(
            FLAG_ICC,
            &[chunk(b"ICCP", &[icc(), b"garbage!".to_vec()].concat())],
        ),
    );
    add(
        "icc-header-only",
        file(
            FLAG_ICC,
            &[chunk(b"ICCP", icc().get(..128).unwrap_or_default())],
        ),
    );
    add(
        "icc-zero-tags",
        file(
            FLAG_ICC,
            &[chunk(b"ICCP", icc().get(..132).unwrap_or_default())],
        ),
    );
    add(
        "all-three",
        riff(&[
            vp8x(FLAG_ICC | ef | FLAG_XMP),
            icc_ok.clone(),
            vp8l(),
            ex(&a),
            x.clone(),
        ]),
    );
    // The declared RIFF size never limits the walk.
    let body = vec![vp8x(ef), vp8l(), ex(&a)];
    for (n, size) in [
        ("0", 0u32),
        ("10", 10),
        ("small", 4),
        ("odd", exact_size(&body) - 1),
        ("ffff-fff0", 0xFFFF_FFF0),
    ] {
        add(&format!("riff-size-{n}"), riff_sized(size, b"WEBP", &body));
    }
    add("riff-size-exact", riff(&body));
    // Trailing bytes after the last chunk.
    for n in [12usize, 3, 1] {
        let mut d = riff(&body);
        d.extend(std::iter::repeat_n(b'A', n));
        add(&format!("tail-{n}-bytes"), d);
    }
    // Chunk length past the end, partial chunk headers.
    let mut d = riff(&body);
    d.extend_from_slice(&raw(b"XMP ", 1000, b"abc"));
    add("chunk-length-past-eof", d);
    for (n, len) in [("ffffffff", 0xFFFF_FFFFu32), ("7fffffff", 0x7FFF_FFFF)] {
        let mut d = riff(&body);
        d.extend_from_slice(&raw(b"XMP ", len, b"abc"));
        add(&format!("chunk-length-{n}"), d);
    }
    for n in [5usize, 7] {
        let mut d = riff(&body);
        d.extend_from_slice(b"XMP \x01\x00\x00\x00".get(..n).unwrap_or_default());
        add(&format!("partial-header-{n}"), d);
    }
    // VP8X lengths and arrangements: informational for ExifTool.
    for n in [0usize, 3, 9, 10, 18] {
        add(
            &format!("vp8x-len-{n}"),
            riff(&[vp8x_len(ef, n), vp8l(), ex(&a)]),
        );
    }
    add("vp8x-two", riff(&[vp8x(0), vp8x(ef), vp8l(), ex(&a)]));
    add(
        "vp8x-short-then-full",
        riff(&[vp8x_len(0, 3), vp8x(ef), vp8l(), ex(&a)]),
    );
    add("vp8x-late", riff(&[vp8l(), vp8x(ef), ex(&a)]));
    add(
        "flags-all-wrong",
        riff(&[vp8x(FLAG_ICC | FLAG_XMP), vp8l(), ex(&a)]),
    );
    add(
        "alph-before-vp8",
        riff(&[vp8x(ef), chunk(b"ALPH", &[0u8; 4]), vp8(), ex(&a)]),
    );
    add("simple-vp8-exif", riff(&[vp8(), ex(&a)]));
    add(
        "junk-first",
        riff(&[chunk(b"JUNK", &[1, 2, 3]), vp8x(ef), vp8l(), ex(&a)]),
    );
    // Headers. A file that is not a WebP is an `Err` for us (all `None`).
    let h = riff(&[vp8l()]);
    add("header-11-bytes", h.get(..11).unwrap_or_default().to_vec());
    add("header-12-bytes", h.get(..12).unwrap_or_default().to_vec());
    let rifx_body = vec![ex(&tiff_of(&[(t::T_MAKE, "RifxCam")]))];
    add("header-rifx", {
        let mut d = riff_sized(exact_size(&rifx_body), b"WEBP", &rifx_body);
        d.get_mut(..4).unwrap_or_default().copy_from_slice(b"RIFX");
        d
    });
    let wave_body = vec![ex(&tiff_of(&[(t::T_MAKE, "WaveCam")]))];
    add(
        "header-wave",
        riff_sized(exact_size(&wave_body), b"WAVE", &wave_body),
    );
    add(
        "header-avi",
        riff_sized(exact_size(&wave_body), b"AVI ", &wave_body),
    );
    out
}

/// An odd-length XMP packet (the shared packet plus a space if it is even).
fn odd_packet() -> Vec<u8> {
    let mut p = packet();
    if p.len().is_multiple_of(2) {
        p.push(b' ');
    }
    p
}

/// Every TIFF fixture, wrapped as `RIFF, VP8X(EXIF flag), VP8L, EXIF`, named
/// `webp-<tiff-name>`.
pub fn wrapped() -> Vec<(String, Vec<u8>)> {
    t::fixtures()
        .into_iter()
        .map(|(name, tiff)| (format!("webp-{name}"), file(FLAG_EXIF, &[exif(&tiff)])))
        .collect()
}

/// The single named list: the wrapped TIFF fixtures first, then the shapes.
/// Order and names are stable; the committed records are keyed by name.
pub fn fixtures() -> Vec<(String, Vec<u8>)> {
    let mut out = wrapped();
    out.extend(shapes().into_iter().map(|(n, b)| (format!("webp-{n}"), b)));
    out
}

/// The fixtures whose ExifTool warnings are recorded in `webp-warnings.txt` (a
/// second ExifTool run per fixture: `-a -G1 -s -n -Warning -Error`), in the
/// order of the warning table in `tests/webp_oracle.rs`, which pins this list.
pub const WARNING_FIXTURES: &[&str] = &[
    "webp-exif-prefix",
    "webp-tail-12-bytes",
    "webp-tail-3-bytes",
    "webp-chunk-length-past-eof",
    "webp-partial-header-5",
    "webp-exif-odd-unpadded-last",
    "webp-xmp-odd-unpadded-last",
    "webp-xmp-nul-id",
    "webp-xmp-nul-id-3-bytes",
    "webp-riff-size-0",
    "webp-riff-size-small",
    "webp-flags-all-wrong",
    "webp-vp8x-two",
    "webp-vp8x-len-3",
    "webp-exif-no-vp8x",
    "webp-icc-garbage",
    "webp-icc-stub-132",
    "webp-xmp-garbage",
    "webp-header-11-bytes",
    "webp-header-rifx",
    "webp-exif-basic",
    "webp-xmp-basic",
    "webp-icc-valid-flag-set",
];

/// The record of those warnings: this file name, in the record directory.
pub const WARNINGS_RECORD: &str = "webp-warnings.txt";

/// The name of the TIFF fixture a wrapped fixture was built from.
pub fn unwrapped_name(webp_name: &str) -> Option<&str> {
    webp_name.strip_prefix("webp-")
}
