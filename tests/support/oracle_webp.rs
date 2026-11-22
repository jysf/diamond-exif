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

/// A little-endian TIFF that carries a value for EVERY one of the 20 Exif
/// selectors, each different for `v` = 1 and `v` = 2 (H1 of the SPEC-034
/// verify): two such blocks tell which block ExifTool keeps, per selector.
/// All values are exactly representable, so there is no rounding to argue about.
pub fn tiff_full(v: u16) -> Vec<u8> {
    let big = false;
    let word = if v == 1 { "One" } else { "Two" };
    let ifd0 = t::ifd(vec![
        t::short(big, t::T_IMAGE_WIDTH, 100 * v),
        t::short(big, t::T_IMAGE_LENGTH, 50 * v),
        t::ascii(t::T_MAKE, format!("Make{word}\0").as_bytes()),
        t::ascii(t::T_MODEL, format!("Model{word}\0").as_bytes()),
        t::short(big, t::T_ORIENTATION, if v == 1 { 1 } else { 8 }),
        t::rational(big, t::T_X_RESOLUTION, &[(72 * u32::from(v), 1)]),
        t::ascii(t::T_SOFTWARE, format!("Soft{word}\0").as_bytes()),
        t::ascii(
            t::T_DATE_TIME,
            format!("2020:01:0{v} 00:00:00\0").as_bytes(),
        ),
        t::ascii(t::T_ARTIST, format!("Artist{word}\0").as_bytes()),
        t::ascii(t::T_COPYRIGHT, format!("Copy{word}\0").as_bytes()),
        t::ptr(t::T_EXIF_IFD, 1),
        t::ptr(t::T_GPS_IFD, 2),
    ]);
    let exif = t::ifd(vec![
        t::float(big, t::T_EXPOSURE_TIME, 0.125 * f32::from(v)),
        t::rational(big, t::T_F_NUMBER, &[(28 * u32::from(v), 10)]),
        t::sshort(big, t::T_EXPOSURE_PROGRAM, -(v as i16)),
        t::short(big, t::T_ISO, 100 * v),
        t::ascii(
            t::T_DATE_TIME_ORIGINAL,
            format!("2021:02:0{v} 01:02:03\0").as_bytes(),
        ),
        t::srational(big, t::T_EXPOSURE_BIAS, &[(-(v as i32), 4)]),
        t::double(big, t::T_FOCAL_LENGTH, 4.5 * f64::from(v)),
        t::slong(big, t::T_EXIF_IMAGE_WIDTH, -5 * i32::from(v)),
    ]);
    let gps = t::ifd(vec![
        t::ascii(t::T_GPS_LAT_REF, b"N\0"),
        t::rational(
            big,
            t::T_GPS_LAT,
            &[(30 + u32::from(v), 1), (25, 1), (3075, 100)],
        ),
        t::ascii(t::T_GPS_LON_REF, b"E\0"),
        t::rational(
            big,
            t::T_GPS_LON,
            &[(120 + u32::from(v), 1), (5, 1), (4890, 100)],
        ),
    ]);
    t::assemble(big, &[ifd0, exif, gps])
}

/// An XMP packet like [`packet`] with a different `xmp:CreatorTool`.
fn packet_with(tool: &str) -> Vec<u8> {
    let p = packet();
    let from = b"DiamondTool";
    let at = p
        .windows(from.len())
        .position(|w| w == from)
        .expect("the shared packet carries DiamondTool");
    let mut out = p[..at].to_vec();
    out.extend_from_slice(tool.as_bytes());
    out.extend_from_slice(&p[at + from.len()..]);
    out
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
    // SPEC-034 verify punch list. H1: two FULL blocks that differ in every
    // selector, both orders.
    add(
        "exif-full-one-two",
        file(ef, &[ex(&tiff_full(1)), ex(&tiff_full(2))]),
    );
    add(
        "exif-full-two-one",
        file(ef, &[ex(&tiff_full(2)), ex(&tiff_full(1))]),
    );
    // M1: the block cap (16) and one block past it.
    let sixteen: Vec<Vec<u8>> = (0..16)
        .map(|i| ex(&tiff_of(&[(t::T_MAKE, &format!("Block{i:02}"))])))
        .collect();
    add("exif-blocks-16", file(ef, &sixteen));
    let mut decoys: Vec<Vec<u8>> = (0..16).map(|_| ex(&a)).collect();
    // The 17th block carries ONLY GPS (so only the two GPS selectors can differ
    // and each divergence entry is exact): IFD0 holds just the GPS pointer.
    let gps_only = t::assemble(
        false,
        &[
            t::ifd(vec![t::ptr(t::T_GPS_IFD, 1)]),
            t::ifd(vec![
                t::ascii(t::T_GPS_LAT_REF, b"N\0"),
                t::rational(false, t::T_GPS_LAT, &[(37, 1), (25, 1), (3075, 100)]),
                t::ascii(t::T_GPS_LON_REF, b"E\0"),
                t::rational(false, t::T_GPS_LON, &[(122, 1), (5, 1), (4890, 100)]),
            ]),
        ],
    );
    decoys.push(ex(&gps_only));
    add("exif-16-then-gps", file(ef, &decoys));
    // L1: a later block whose Make is a SHORT.
    let make_short = t::assemble(false, &[t::ifd(vec![t::short(false, t::T_MAKE, 7)])]);
    add(
        "exif-make-short-later",
        file(ef, &[ex(&a), ex(&make_short)]),
    );
    // M2: several regions that can be told apart.
    let xmp_x = chunk(b"XMP ", &packet_with("FirstTool"));
    let xmp_y = chunk(b"XMP ", &packet_with("SecondTool"));
    let xmp_z = chunk(b"XMP ", &packet_with("ThirdTool"));
    add(
        "xmp-two-distinct",
        file(FLAG_XMP, &[xmp_x.clone(), xmp_y.clone()]),
    );
    add("xmp-three", file(FLAG_XMP, &[xmp_x, xmp_y, xmp_z]));
    let icc_b = chunk(b"ICCP", &[icc(), b"b".to_vec()].concat());
    let icc_c = chunk(b"ICCP", &[icc(), b"cc".to_vec()].concat());
    add("icc-three", file(FLAG_ICC, &[icc_ok.clone(), icc_b, icc_c]));
    // M3: an empty chunk whose code is four NUL bytes.
    let null_chunk = vec![0u8; 8];
    add(
        "null-chunk-before-exif",
        file(ef, &[null_chunk.clone(), ex(&a)]),
    );
    add("null-chunk-tail", file(ef, &[ex(&a), null_chunk]));
    // L2: warning-only shapes.
    add(
        "stray-pad-then-xmp",
        file(
            ef | FLAG_XMP,
            &[
                {
                    let mut e = chunk(b"EXIF", &tiff_a());
                    e.push(0xAA);
                    e
                },
                x.clone(),
            ],
        ),
    );
    add("header-20-exif-size-0", riff(&[chunk(b"EXIF", &[])]));
    add(
        "xmp-utf8-bom",
        file(
            FLAG_XMP,
            &[chunk(b"XMP ", &[&b"\xEF\xBB\xBF"[..], &packet()].concat())],
        ),
    );
    let utf16: Vec<u8> = [0xFFu8, 0xFE]
        .into_iter()
        .chain(packet().iter().flat_map(|c| [*c, 0]))
        .collect();
    add("xmp-utf16le", file(FLAG_XMP, &[chunk(b"XMP ", &utf16)]));
    add(
        "exif-tiff-8-bytes",
        file(ef, &[ex(&[b'I', b'I', 42, 0, 8, 0, 0, 0])]),
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
    "webp-null-chunk-before-exif",
    "webp-null-chunk-tail",
    "webp-stray-pad-then-xmp",
    "webp-header-20-exif-size-0",
    "webp-xmp-utf8-bom",
    "webp-xmp-utf16le",
    "webp-exif-tiff-8-bytes",
];

/// The record of those warnings: this file name, in the record directory.
pub const WARNINGS_RECORD: &str = "webp-warnings.txt";

/// The name of the TIFF fixture a wrapped fixture was built from.
pub fn unwrapped_name(webp_name: &str) -> Option<&str> {
    webp_name.strip_prefix("webp-")
}

/// One region the library must locate: the chunk code at `offset` and the data
/// that follows its 8-byte head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expect {
    /// `XMP ` or `XMP\0` or `ICCP`.
    pub code: [u8; 4],
    /// Input offset of the chunk.
    pub offset: usize,
    /// The chunk data (the packet or the profile), empty ones included.
    pub bytes: Vec<u8>,
}

/// A part of a hand-laid file: a plain chunk, or a chunk the library must locate.
enum Part {
    /// Not a region (`EXIF`, `ANMF` and so on, or a chunk that is not located).
    Plain(Vec<u8>),
    /// A located chunk: its code, its data, and whether the pad byte is written.
    Region([u8; 4], Vec<u8>, bool),
}

fn xr(data: &[u8]) -> Part {
    Part::Region(*b"XMP ", data.to_vec(), true)
}

fn xr_np(data: &[u8]) -> Part {
    Part::Region(*b"XMP ", data.to_vec(), false)
}

fn xn(data: &[u8]) -> Part {
    Part::Region(*b"XMP\0", data.to_vec(), true)
}

fn ir(data: &[u8]) -> Part {
    Part::Region(*b"ICCP", data.to_vec(), true)
}

/// The regions of a file laid as the 12-byte header, `head` (chunks that are
/// never regions) and then `parts`: each offset is the running sum of the
/// lengths the builders wrote, never read from the library.
fn lay(head: &[Vec<u8>], parts: &[Part]) -> Vec<Expect> {
    let mut at = 12 + head.iter().map(Vec::len).sum::<usize>();
    let mut out = Vec::new();
    for p in parts {
        match p {
            Part::Plain(b) => at += b.len(),
            Part::Region(code, data, pad) => {
                out.push(Expect {
                    code: *code,
                    offset: at,
                    bytes: data.clone(),
                });
                at += if *pad {
                    chunk(code, data).len()
                } else {
                    chunk_np(code, data).len()
                };
            }
        }
    }
    out
}

/// The regions each fixture that has an `XMP `, `XMP\0` or `ICCP` chunk code
/// must locate, in chunk order, empty ones included. A fixture whose chunk is
/// not located (a length past the end, a misaligned chunk) maps to none. Every
/// other fixture must locate none; [`has_region_chunk_code`] finds the fixtures
/// that must be listed.
pub fn expected_regions() -> Vec<(String, Vec<Expect>)> {
    let pk = packet();
    let opk = odd_packet();
    let icc_p = icc();
    let a = tiff_a();
    let head = vec![vp8x(0), vp8l()]; // flags do not change the lengths
    let none: Vec<Part> = Vec::new();
    let mut t_: Vec<(String, Vec<Expect>)> = Vec::new();
    let mut put = |name: &str, parts: &[Part]| t_.push((format!("webp-{name}"), lay(&head, parts)));
    put("xmp-basic", &[xr(&pk)]);
    put("xmp-nul-id", &[xn(&pk)]);
    put("xmp-empty", &[xr(&[])]);
    put("xmp-nul-id-empty", &[xn(&[])]);
    for n in 1..=7usize {
        put(
            &format!("xmp-nul-id-{n}-bytes"),
            &[xn(pk.get(..n).unwrap_or_default())],
        );
    }
    put("xmp-two", &[xr(&pk), xr(&pk)]);
    put("xmp-empty-then-real", &[xr(&[]), xr(&pk)]);
    put("xmp-garbage", &[xr(b"not xml at all, just bytes")]);
    put("xmp-odd-padded", &[xr(&opk)]);
    put("xmp-odd-unpadded-last", &[xr_np(&opk)]);
    put(
        "xmp-odd-unpadded-then-exif",
        &[xr_np(&opk), Part::Plain(chunk(b"EXIF", &a))],
    );
    // The unpadded odd EXIF makes the next header start one byte late: the
    // `XMP ` chunk behind it is not located (ExifTool warns too).
    put("exif-odd-unpadded-then-xmp", &none);
    put(
        "xmp-two-distinct",
        &[
            xr(&packet_with("FirstTool")),
            xr(&packet_with("SecondTool")),
        ],
    );
    put(
        "xmp-three",
        &[
            xr(&packet_with("FirstTool")),
            xr(&packet_with("SecondTool")),
            xr(&packet_with("ThirdTool")),
        ],
    );
    let mut bom = b"\xEF\xBB\xBF".to_vec();
    bom.extend_from_slice(&pk);
    put("xmp-utf8-bom", &[xr(&bom)]);
    let utf16: Vec<u8> = [0xFFu8, 0xFE]
        .into_iter()
        .chain(pk.iter().flat_map(|c| [*c, 0]))
        .collect();
    put("xmp-utf16le", &[xr(&utf16)]);
    put("icc-valid-flag-set", &[ir(&icc_p)]);
    put("icc-valid-flag-unset", &[ir(&icc_p)]);
    put("icc-garbage", &[ir(b"icc! not a profile")]);
    put("icc-stub-132", &[ir(&[0u8; 132])]);
    put("icc-empty", &[ir(&[])]);
    put("icc-valid-then-garbage", &[ir(&icc_p), ir(b"garbage!")]);
    put("icc-garbage-then-valid", &[ir(b"garbage!"), ir(&icc_p)]);
    put(
        "icc-valid-trailing-garbage",
        &[ir(&[icc_p.clone(), b"garbage!".to_vec()].concat())],
    );
    put(
        "icc-header-only",
        &[ir(icc_p.get(..128).unwrap_or_default())],
    );
    put("icc-zero-tags", &[ir(icc_p.get(..132).unwrap_or_default())]);
    put(
        "icc-three",
        &[
            ir(&icc_p),
            ir(&[icc_p.clone(), b"b".to_vec()].concat()),
            ir(&[icc_p.clone(), b"cc".to_vec()].concat()),
        ],
    );
    // The RIFF-size shapes carry no region; these do carry an `XMP ` code whose
    // chunk runs past the end or is cut short: none is located.
    for name in [
        "chunk-length-past-eof",
        "chunk-length-ffffffff",
        "chunk-length-7fffffff",
        "partial-header-5",
        "partial-header-7",
    ] {
        put(name, &none);
    }
    // The stray byte after an even EXIF shifts the next header: not located.
    put("stray-pad-then-xmp", &none);
    // The ICCP chunk comes between VP8X and VP8L here.
    t_.push((
        "webp-all-three".to_string(),
        lay(
            &[vp8x(0)],
            &[
                ir(&icc_p),
                Part::Plain(vp8l()),
                Part::Plain(chunk(b"EXIF", &a)),
                xr(&pk),
            ],
        ),
    ));
    t_
}

/// Whether `bytes` holds an `XMP `, `XMP\0` or `ICCP` chunk code anywhere (the
/// payloads of the shared builders never contain those four bytes by chance).
pub fn has_region_chunk_code(bytes: &[u8]) -> bool {
    bytes
        .windows(4)
        .any(|w| w == b"XMP " || w == b"XMP\0" || w == b"ICCP")
}
