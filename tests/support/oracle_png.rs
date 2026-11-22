//! Hand-built PNG fixtures for the ExifTool parity oracle (SPEC-031).
//!
//! Tier A: every byte is written here from the PNG specification (W3C, Third
//! Edition): the eight-byte signature, chunks as a big-endian length, a four-byte
//! type, the data and a CRC-32 over type and data; `IHDR`, `IDAT`, `IEND`; `eXIf`
//! (a TIFF, optionally with an `Exif\0\0` prefix); `tEXt`, `zTXt` and `iTXt`
//! (keyword, NUL, then the layout of each); `iCCP` (profile name, NUL,
//! compression method, a zlib stream). The CRC-32 and the zlib stream (stored
//! deflate blocks plus an Adler-32) are written in this file: no compression
//! library, and no code shared with `src/`, with `examples/fuzz_seeds.rs`, with
//! `tests/support/png_shapes.rs` or with the fuzz models. The oracle must not
//! share a bug with the thing it checks. The TIFF blobs come from
//! `oracle_tiff`, which is independent too.
//!
//! Deterministic: [`fixtures`] is a pure function of this file and of
//! `oracle_tiff::fixtures()`. It is the single named list used by the recorder,
//! the live tests and the replay tests. std only.
//!
//! Names are normative (SPEC-031): `png-<tiff fixture name>` for the wrapped
//! TIFFs and the `png-...` shape names below. What ExifTool and `exif-core` read
//! from them is measured, never assumed (`tests/oracle-fixtures/exiftool/png-*.txt`).

#![allow(dead_code)]

use crate::oracle_tiff as t;

pub const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// CRC-32 (IEEE, reflected, polynomial 0xEDB88320), bit by bit.
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in bytes {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// Adler-32 of `bytes` (the zlib trailer).
pub fn adler32(bytes: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &x in bytes {
        a = (a + u32::from(x)) % 65_521;
        b = (b + a) % 65_521;
    }
    (b << 16) | a
}

/// A zlib stream holding `data` in stored (uncompressed) deflate blocks: the
/// header `78 01`, blocks of at most 65 535 bytes (`BFINAL` on the last, `BTYPE`
/// 00, `LEN`, `NLEN`), then the big-endian Adler-32.
pub fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = if data.is_empty() {
        vec![&[][..]]
    } else {
        data.chunks(65_535).collect()
    };
    let last = blocks.len() - 1;
    for (i, block) in blocks.iter().enumerate() {
        out.push(u8::from(i == last));
        let len = u16::try_from(block.len()).expect("block fits 16 bits");
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(block);
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

/// One chunk with a correct CRC.
pub fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut crc_input = kind.to_vec();
    crc_input.extend_from_slice(data);
    let mut v = u32::try_from(data.len())
        .expect("chunk data fits 32 bits")
        .to_be_bytes()
        .to_vec();
    v.extend_from_slice(&crc_input);
    v.extend_from_slice(&crc32(&crc_input).to_be_bytes());
    v
}

/// One chunk whose stored CRC is wrong (the correct one with its low bit flipped).
pub fn chunk_bad_crc(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut v = chunk(kind, data);
    let last = v.len() - 1;
    v[last] ^= 1;
    v
}

/// `IHDR`: a 1 x 1, 8-bit grey-scale image (width, height, depth 8, colour type
/// 0, compression 0, filter 0, interlace 0).
pub fn ihdr() -> Vec<u8> {
    chunk(b"IHDR", &[0, 0, 0, 1, 0, 0, 0, 1, 8, 0, 0, 0, 0])
}

/// `IDAT`: one scanline (filter byte 0, one sample) in a stored zlib stream.
pub fn idat() -> Vec<u8> {
    chunk(b"IDAT", &zlib_stored(&[0, 0x80]))
}

pub fn iend() -> Vec<u8> {
    chunk(b"IEND", &[])
}

/// A PNG: signature, `IHDR`, the chunks `before` the image data, `IDAT`, the
/// chunks `after_idat`, `IEND`, then `trailer` bytes verbatim.
pub fn png(before: &[Vec<u8>], after_idat: &[Vec<u8>], trailer: &[u8]) -> Vec<u8> {
    let mut d = SIGNATURE.to_vec();
    d.extend_from_slice(&ihdr());
    for c in before {
        d.extend_from_slice(c);
    }
    d.extend_from_slice(&idat());
    for c in after_idat {
        d.extend_from_slice(c);
    }
    d.extend_from_slice(&iend());
    d.extend_from_slice(trailer);
    d
}

/// A PNG with `chunks` between `IHDR` and `IDAT`.
fn clean(chunks: &[Vec<u8>]) -> Vec<u8> {
    png(chunks, &[], &[])
}

pub fn exif_chunk(tiff: &[u8]) -> Vec<u8> {
    chunk(b"eXIf", tiff)
}

fn exif_prefixed(tiff: &[u8]) -> Vec<u8> {
    chunk(b"eXIf", &[&b"Exif\0\0"[..], tiff].concat())
}

fn named_tiff(name: &str) -> Vec<u8> {
    t::fixtures()
        .into_iter()
        .find(|(n, _)| *n == name)
        .map(|(_, b)| b)
        .unwrap_or_else(|| panic!("no TIFF fixture named {name}"))
}

fn basic() -> Vec<u8> {
    named_tiff("exif-basic-le")
}

/// A tiny TIFF whose IFD0 holds the given ASCII `(tag, text)` entries.
fn tiff_of(entries: &[(u16, &str)]) -> Vec<u8> {
    let ents: Vec<t::Ent> = entries
        .iter()
        .map(|(tag, text)| t::ascii(*tag, &[text.as_bytes(), b"\0"].concat()))
        .collect();
    t::assemble(false, &[t::ifd(ents)])
}

/// A minimal XMP packet carrying `xmp:CreatorTool` = `DiamondTool`.
pub fn xmp_packet() -> Vec<u8> {
    let mut p = b"<?xpacket begin=\"\xEF\xBB\xBF\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n".to_vec();
    p.extend_from_slice(
        b"<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\
          <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
          <rdf:Description rdf:about=\"\" xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\">\
          <xmp:CreatorTool>DiamondTool</xmp:CreatorTool>\
          </rdf:Description></rdf:RDF></x:xmpmeta>\n\
          <?xpacket end=\"w\"?>",
    );
    p
}

pub const XMP_KEYWORD: &[u8] = b"XML:com.adobe.xmp";

/// `tEXt`: keyword, NUL, text.
pub fn text(keyword: &[u8], body: &[u8]) -> Vec<u8> {
    chunk(b"tEXt", &[keyword, &[0][..], body].concat())
}

/// `zTXt`: keyword, NUL, method 0, a zlib stream of the text.
pub fn ztxt(keyword: &[u8], body: &[u8]) -> Vec<u8> {
    chunk(
        b"zTXt",
        &[keyword, &[0, 0][..], &zlib_stored(body)].concat(),
    )
}

/// `iTXt`: keyword, NUL, compression flag, method, empty language tag, NUL, empty
/// translated keyword, NUL, then the text (a zlib stream when `flag` is 1).
pub fn itxt(keyword: &[u8], flag: u8, body: &[u8]) -> Vec<u8> {
    itxt_full(keyword, flag, b"", b"", body)
}

/// `iTXt` with a language tag and a translated keyword (each NUL-terminated).
pub fn itxt_full(keyword: &[u8], flag: u8, lang: &[u8], translated: &[u8], body: &[u8]) -> Vec<u8> {
    let text = if flag == 1 {
        zlib_stored(body)
    } else {
        body.to_vec()
    };
    let head = [
        keyword,
        &[0, flag, 0][..],
        lang,
        &[0][..],
        translated,
        &[0][..],
    ]
    .concat();
    chunk(b"iTXt", &[&head[..], &text].concat())
}

/// An ICC profile: the 128-byte header (total length, version 2.1.0, class
/// `mntr`, space `RGB `, PCS `XYZ `, `acsp` at 36, the rest zero), a tag count
/// and one `desc` tag.
pub fn icc_profile() -> Vec<u8> {
    let desc = [
        &b"desc"[..],
        &[0, 0, 0, 0][..],
        &5u32.to_be_bytes()[..],
        &b"abcd\0"[..],
    ]
    .concat();
    let at = 128 + 4 + 12;
    let total = at + desc.len();
    let mut p = vec![0u8; 128];
    p[0..4].copy_from_slice(&u32::try_from(total).expect("length").to_be_bytes());
    p[8..12].copy_from_slice(&[0x02, 0x10, 0x00, 0x00]);
    p[12..16].copy_from_slice(b"mntr");
    p[16..20].copy_from_slice(b"RGB ");
    p[20..24].copy_from_slice(b"XYZ ");
    p[36..40].copy_from_slice(b"acsp");
    p.extend_from_slice(&1u32.to_be_bytes());
    p.extend_from_slice(b"desc");
    p.extend_from_slice(&u32::try_from(at).expect("offset").to_be_bytes());
    p.extend_from_slice(&u32::try_from(desc.len()).expect("size").to_be_bytes());
    p.extend_from_slice(&desc);
    p
}

/// `iCCP`: profile name, NUL, method 0, the profile in a zlib stream.
pub fn iccp(name: &[u8], profile: &[u8]) -> Vec<u8> {
    chunk(
        b"iCCP",
        &[name, &[0, 0][..], &zlib_stored(profile)].concat(),
    )
}

/// The text body of an ImageMagick-style raw profile: a newline, the profile name,
/// a newline, the byte count right-aligned in ten columns, a newline, then the
/// bytes as lower-case hex in lines of 72 digits.
pub fn raw_profile_body(name: &str, data: &[u8]) -> Vec<u8> {
    let hex: String = data.iter().map(|b| format!("{b:02x}")).collect();
    let mut s = format!("\n{name}\n{:>10}\n", data.len());
    for line in hex.as_bytes().chunks(72) {
        s.push_str(std::str::from_utf8(line).expect("hex is ASCII"));
        s.push('\n');
    }
    s.into_bytes()
}

fn raw_exif_data() -> Vec<u8> {
    [&b"Exif\0\0"[..], &basic()].concat()
}

const RAW_EXIF_KEYWORD: &[u8] = b"Raw profile type exif";
const RAW_XMP_KEYWORD: &[u8] = b"Raw profile type xmp";

// ── the shape fixtures ──────────────────────────────────────────────────────

fn two_exif(a_then_b: bool) -> Vec<u8> {
    let a = exif_chunk(&tiff_of(&[
        (t::T_MAKE, "AlphaCam"),
        (t::T_MODEL, "OnlyInA"),
    ]));
    let b = exif_chunk(&tiff_of(&[(t::T_MAKE, "BravoCam")]));
    if a_then_b {
        clean(&[a, b])
    } else {
        clean(&[b, a])
    }
}

fn two_exif_disjoint() -> Vec<u8> {
    clean(&[
        exif_chunk(&tiff_of(&[(t::T_MAKE, "AlphaCam")])),
        exif_chunk(&tiff_of(&[(t::T_MODEL, "BravoModel")])),
    ])
}

/// One good `eXIf` and one that is not a TIFF, in either order.
fn good_and_fatal(good_first: bool) -> Vec<u8> {
    let good = exif_chunk(&tiff_of(&[
        (t::T_MAKE, "AlphaCam"),
        (t::T_MODEL, "OnlyInA"),
    ]));
    let fatal = exif_chunk(b"junkjunkjunkjunk");
    if good_first {
        clean(&[good, fatal])
    } else {
        clean(&[fatal, good])
    }
}

/// `eXIf` BEFORE `IHDR` (a chunk-order violation).
fn exif_before_ihdr() -> Vec<u8> {
    let mut d = SIGNATURE.to_vec();
    d.extend_from_slice(&exif_chunk(&basic()));
    d.extend_from_slice(&ihdr());
    d.extend_from_slice(&idat());
    d.extend_from_slice(&iend());
    d
}

/// The shape fixtures, in a fixed order.
fn shapes() -> Vec<(&'static str, Vec<u8>)> {
    let basic = basic();
    let exif = exif_chunk(&basic);
    let xmp = xmp_packet();

    // No IHDR: the signature, an eXIf, IDAT, IEND.
    let mut no_ihdr = SIGNATURE.to_vec();
    no_ihdr.extend_from_slice(&exif);
    no_ihdr.extend_from_slice(&idat());
    no_ihdr.extend_from_slice(&iend());
    // No IEND: ends after IDAT.
    let mut no_iend = SIGNATURE.to_vec();
    no_iend.extend_from_slice(&ihdr());
    no_iend.extend_from_slice(&exif);
    no_iend.extend_from_slice(&idat());
    // The eXIf chunk's declared length runs past the end of the file.
    let mut length_past_eof = SIGNATURE.to_vec();
    length_past_eof.extend_from_slice(&ihdr());
    let mut broken = exif.clone();
    broken[0..4].copy_from_slice(
        &u32::try_from(basic.len() + 1000)
            .expect("len")
            .to_be_bytes(),
    );
    length_past_eof.extend_from_slice(&broken);
    length_past_eof.extend_from_slice(&idat());
    // The eXIf chunk's declared length is 0x80000000.
    let mut length_huge = SIGNATURE.to_vec();
    length_huge.extend_from_slice(&ihdr());
    let mut huge = exif.clone();
    huge[0..4].copy_from_slice(&0x8000_0000u32.to_be_bytes());
    length_huge.extend_from_slice(&huge);
    length_huge.extend_from_slice(&idat());
    length_huge.extend_from_slice(&iend());
    // Truncated between IHDR and IDAT: the file ends after the eXIf chunk.
    let mut truncated = SIGNATURE.to_vec();
    truncated.extend_from_slice(&ihdr());
    truncated.extend_from_slice(&exif);

    let keyword_80 = [b'k'; 80];

    vec![
        // Where the eXIf sits.
        (
            "png-exif-after-idat",
            png(&[], std::slice::from_ref(&exif), &[]),
        ),
        ("png-exif-after-iend", png(&[], &[], &exif)),
        ("png-exif-before-ihdr", exif_before_ihdr()),
        ("png-exif-prefix", clean(&[exif_prefixed(&basic)])),
        ("png-exif-empty", clean(&[exif_chunk(&[])])),
        (
            "png-exif-not-tiff",
            clean(&[exif_chunk(b"junkjunkjunkjunk")]),
        ),
        ("png-exif-bad-crc", clean(&[chunk_bad_crc(b"eXIf", &basic)])),
        // Several Exif chunks.
        ("png-two-exif-same-tag", two_exif(true)),
        ("png-two-exif-same-tag-reversed", two_exif(false)),
        ("png-two-exif-disjoint-tags", two_exif_disjoint()),
        ("png-exif-good-then-fatal", good_and_fatal(true)),
        ("png-exif-fatal-then-good", good_and_fatal(false)),
        // Damage in the chunk stream.
        ("png-no-ihdr", no_ihdr),
        ("png-no-iend", no_iend),
        ("png-length-past-eof", length_past_eof),
        ("png-length-80000000", length_huge),
        ("png-truncated-between-ihdr-and-idat", truncated),
        (
            "png-trailer-bytes",
            png(
                std::slice::from_ref(&exif),
                &[],
                &[0xDE, 0xAD, 0xBE, 0xEF, 1, 2, 3],
            ),
        ),
        // XMP under each text chunk and keyword rule.
        ("png-xmp-itxt", clean(&[itxt(XMP_KEYWORD, 0, &xmp)])),
        (
            "png-xmp-itxt-compressed",
            clean(&[itxt(XMP_KEYWORD, 1, &xmp)]),
        ),
        ("png-xmp-text", clean(&[text(XMP_KEYWORD, &xmp)])),
        ("png-xmp-ztxt", clean(&[ztxt(XMP_KEYWORD, &xmp)])),
        ("png-xmp-other-keyword", clean(&[itxt(b"Comment", 0, &xmp)])),
        (
            "png-xmp-lowercase-keyword",
            clean(&[itxt(b"xml:com.adobe.xmp", 0, &xmp)]),
        ),
        ("png-xmp-itxt-flag-2", clean(&[itxt(XMP_KEYWORD, 2, &xmp)])),
        (
            "png-xmp-after-idat",
            png(&[], &[itxt(XMP_KEYWORD, 0, &xmp)], &[]),
        ),
        // ICC.
        ("png-iccp-valid", clean(&[iccp(b"diamond", &icc_profile())])),
        (
            "png-iccp-bad-header",
            clean(&[iccp(b"diamond", &[0u8; 20])]),
        ),
        // ImageMagick raw profiles, in each text chunk type.
        (
            "png-raw-exif-text",
            clean(&[text(
                RAW_EXIF_KEYWORD,
                &raw_profile_body("exif", &raw_exif_data()),
            )]),
        ),
        (
            "png-raw-exif-ztxt",
            clean(&[ztxt(
                RAW_EXIF_KEYWORD,
                &raw_profile_body("exif", &raw_exif_data()),
            )]),
        ),
        (
            "png-raw-exif-itxt",
            clean(&[itxt(
                RAW_EXIF_KEYWORD,
                1,
                &raw_profile_body("exif", &raw_exif_data()),
            )]),
        ),
        (
            "png-raw-xmp-text",
            clean(&[text(RAW_XMP_KEYWORD, &raw_profile_body("xmp", &xmp))]),
        ),
        (
            "png-raw-xmp-ztxt",
            clean(&[ztxt(RAW_XMP_KEYWORD, &raw_profile_body("xmp", &xmp))]),
        ),
        (
            "png-raw-xmp-itxt",
            clean(&[itxt(RAW_XMP_KEYWORD, 1, &raw_profile_body("xmp", &xmp))]),
        ),
        // Both a real eXIf and a raw-profile Exif (different Make).
        (
            "png-exif-and-raw-exif",
            clean(&[
                exif_chunk(&tiff_of(&[
                    (t::T_MAKE, "AlphaCam"),
                    (t::T_MODEL, "OnlyInA"),
                ])),
                text(
                    RAW_EXIF_KEYWORD,
                    &raw_profile_body(
                        "exif",
                        &[&b"Exif\0\0"[..], &tiff_of(&[(t::T_MAKE, "RawCam")])].concat(),
                    ),
                ),
            ]),
        ),
        // A keyword at the 79-byte limit and one past it.
        (
            "png-keyword-79",
            clean(&[text(&keyword_80[..79], b"value")]),
        ),
        ("png-keyword-80", clean(&[text(&keyword_80, b"value")])),
        // iCCP profile names (SPEC-031 verify M1): 79 bytes, 80 bytes, empty.
        (
            "png-iccp-name-79",
            clean(&[iccp(&[b'N'; 79], &icc_profile())]),
        ),
        (
            "png-iccp-name-80",
            clean(&[iccp(&[b'N'; 80], &icc_profile())]),
        ),
        ("png-iccp-name-empty", clean(&[iccp(b"", &icc_profile())])),
        // iTXt XMP with a language tag and/or a translated keyword (M2).
        (
            "png-xmp-itxt-lang",
            clean(&[itxt_full(XMP_KEYWORD, 0, b"en-US", b"", &xmp)]),
        ),
        (
            "png-xmp-itxt-lang-compressed",
            clean(&[itxt_full(XMP_KEYWORD, 1, b"en-US", b"", &xmp)]),
        ),
        (
            "png-xmp-itxt-translated",
            clean(&[itxt_full(XMP_KEYWORD, 0, b"", b"XMP Daten", &xmp)]),
        ),
        (
            "png-xmp-itxt-lang-translated",
            clean(&[itxt_full(XMP_KEYWORD, 0, b"en-US", b"XMP Daten", &xmp)]),
        ),
        // A text chunk with no NUL: nothing to locate, one BadTextChunk issue.
        ("png-text-no-nul", clean(&[chunk(b"tEXt", b"nonulhere")])),
        // An empty keyword (SPEC-031 verify, round 2): ExifTool shows the value as
        // a tag named `Tag`, in all three chunk types.
        ("png-text-empty-keyword", clean(&[text(b"", b"John Doe")])),
        (
            "png-text-empty-keyword-ztxt",
            clean(&[ztxt(b"", b"secret value")]),
        ),
        (
            "png-text-empty-keyword-itxt",
            clean(&[itxt(b"", 0, b"secret value")]),
        ),
        // An eXIf, an XMP chunk and an iCCP together (a second region matters).
        (
            "png-exif-xmp-iccp",
            clean(&[
                exif_chunk(&basic),
                itxt(XMP_KEYWORD, 0, &xmp),
                iccp(b"diamond", &icc_profile()),
            ]),
        ),
    ]
}

/// One region a fixture is expected to locate, written from what the builders
/// above put in the chunk (not from `src/`): `kind` is `xmp`, `icc`, `raw` or
/// `text`; `name` is the profile name, the raw-profile suffix or the keyword
/// (empty for XMP); `data` is the exact bytes of the text field or of the zlib
/// stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expect {
    pub kind: &'static str,
    pub compressed: bool,
    pub name: Vec<u8>,
    pub data: Vec<u8>,
    /// Where the chunk starts in the file, from the builders' chunk lengths.
    pub offset: usize,
}

impl Expect {
    /// The same region, at `offset` instead of the first chunk position.
    fn at(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }
}

/// The offset of the first chunk after `IHDR`: the signature and the `IHDR` chunk.
fn first_chunk_offset() -> usize {
    SIGNATURE.len() + ihdr().len()
}

fn ex(kind: &'static str, compressed: bool, name: &[u8], data: Vec<u8>) -> Expect {
    Expect {
        kind,
        compressed,
        name: name.to_vec(),
        data,
        offset: first_chunk_offset(),
    }
}

/// The regions each hand-built fixture must locate, in chunk order. A fixture not
/// listed (every wrapped TIFF, the `eXIf` shapes) must locate none.
pub fn expected_regions() -> Vec<(&'static str, Vec<Expect>)> {
    let xmp = xmp_packet();
    let xz = zlib_stored(&xmp);
    let prof = zlib_stored(&icc_profile());
    let raw_exif = raw_profile_body("exif", &raw_exif_data());
    let raw_xmp = raw_profile_body("xmp", &xmp);
    let raw_mixed = raw_profile_body(
        "exif",
        &[&b"Exif\0\0"[..], &tiff_of(&[(t::T_MAKE, "RawCam")])].concat(),
    );
    // The leading `eXIf` chunks' lengths, for the regions that follow them.
    let exif_len = exif_chunk(&basic()).len();
    let mixed_exif_len = exif_chunk(&tiff_of(&[
        (t::T_MAKE, "AlphaCam"),
        (t::T_MODEL, "OnlyInA"),
    ]))
    .len();
    let xmp_r = |c: bool, d: &[u8]| ex("xmp", c, b"", d.to_vec());
    let icc = |n: &[u8]| ex("icc", true, n, prof.clone());
    vec![
        ("png-xmp-itxt", vec![xmp_r(false, &xmp)]),
        ("png-xmp-itxt-compressed", vec![xmp_r(true, &xz)]),
        ("png-xmp-text", vec![xmp_r(false, &xmp)]),
        ("png-xmp-ztxt", vec![xmp_r(true, &xz)]),
        ("png-xmp-itxt-flag-2", vec![xmp_r(true, &xmp)]),
        (
            "png-xmp-after-idat",
            vec![xmp_r(false, &xmp).at(first_chunk_offset() + idat().len())],
        ),
        (
            "png-xmp-other-keyword",
            vec![ex("text", false, b"Comment", xmp.clone())],
        ),
        (
            "png-xmp-lowercase-keyword",
            vec![ex("text", false, b"xml:com.adobe.xmp", xmp.clone())],
        ),
        (
            "png-xmp-itxt-lang",
            vec![ex("text", false, XMP_KEYWORD, xmp.clone())],
        ),
        (
            "png-xmp-itxt-lang-compressed",
            vec![ex("text", true, XMP_KEYWORD, xz.clone())],
        ),
        ("png-xmp-itxt-translated", vec![xmp_r(false, &xmp)]),
        (
            "png-xmp-itxt-lang-translated",
            vec![ex("text", false, XMP_KEYWORD, xmp.clone())],
        ),
        ("png-iccp-valid", vec![icc(b"diamond")]),
        (
            "png-iccp-bad-header",
            vec![ex("icc", true, b"diamond", zlib_stored(&[0u8; 20]))],
        ),
        ("png-iccp-name-79", vec![icc(&[b'N'; 79])]),
        ("png-iccp-name-80", vec![icc(&[b'N'; 80])]),
        ("png-iccp-name-empty", vec![icc(b"")]),
        (
            "png-exif-xmp-iccp",
            vec![
                xmp_r(false, &xmp).at(first_chunk_offset() + exif_len),
                icc(b"diamond")
                    .at(first_chunk_offset() + exif_len + itxt(XMP_KEYWORD, 0, &xmp).len()),
            ],
        ),
        (
            "png-raw-exif-text",
            vec![ex("raw", false, b"exif", raw_exif.clone())],
        ),
        (
            "png-raw-exif-ztxt",
            vec![ex("raw", true, b"exif", zlib_stored(&raw_exif))],
        ),
        (
            "png-raw-exif-itxt",
            vec![ex("raw", true, b"exif", zlib_stored(&raw_exif))],
        ),
        (
            "png-raw-xmp-text",
            vec![ex("raw", false, b"xmp", raw_xmp.clone())],
        ),
        (
            "png-raw-xmp-ztxt",
            vec![ex("raw", true, b"xmp", zlib_stored(&raw_xmp))],
        ),
        (
            "png-raw-xmp-itxt",
            vec![ex("raw", true, b"xmp", zlib_stored(&raw_xmp))],
        ),
        (
            "png-exif-and-raw-exif",
            vec![ex("raw", false, b"exif", raw_mixed).at(first_chunk_offset() + mixed_exif_len)],
        ),
        (
            "png-text-empty-keyword",
            vec![ex("text", false, b"", b"John Doe".to_vec())],
        ),
        (
            "png-text-empty-keyword-ztxt",
            vec![ex("text", true, b"", zlib_stored(b"secret value"))],
        ),
        (
            "png-text-empty-keyword-itxt",
            vec![ex("text", false, b"", b"secret value".to_vec())],
        ),
        (
            "png-keyword-79",
            vec![ex("text", false, &[b'k'; 79], b"value".to_vec())],
        ),
        (
            "png-keyword-80",
            vec![ex("text", false, &[b'k'; 80], b"value".to_vec())],
        ),
    ]
}

/// Every TIFF fixture, wrapped as `PNG signature, IHDR, eXIf, IDAT, IEND`, named
/// `png-<tiff-name>`.
pub fn wrapped() -> Vec<(String, Vec<u8>)> {
    t::fixtures()
        .into_iter()
        .map(|(name, tiff)| (format!("png-{name}"), clean(&[exif_chunk(&tiff)])))
        .collect()
}

/// The single named list: the wrapped TIFF fixtures first, then the shapes.
/// Order and names are stable; the committed records are keyed by name.
pub fn fixtures() -> Vec<(String, Vec<u8>)> {
    let mut out = wrapped();
    out.extend(shapes().into_iter().map(|(n, b)| (n.to_string(), b)));
    out
}

/// The fixtures whose ExifTool warnings are recorded in `png-warnings.txt` (a
/// second ExifTool run per fixture: `-a -G1 -s -n -Warning -Error`), in the order
/// of the warning table in `tests/png_oracle.rs`, which pins this list. A
/// `-T -s3` run prints warnings to stderr only when none of the requested tags
/// was found, so the per-fixture records cannot carry them all.
pub const WARNING_FIXTURES: &[&str] = &[
    "png-exif-prefix",
    "png-no-iend",
    "png-truncated-between-ihdr-and-idat",
    "png-length-past-eof",
    "png-length-80000000",
    "png-no-ihdr",
    "png-exif-before-ihdr",
    "png-two-exif-same-tag",
    "png-xmp-itxt-flag-2",
    "png-trailer-bytes",
    "png-exif-after-iend",
    "png-exif-after-idat",
    "png-xmp-after-idat",
    "png-exif-empty",
    "png-exif-not-tiff",
    "png-exif-bad-crc",
    "png-keyword-80",
    "png-keyword-79",
    "png-text-no-nul",
    "png-text-empty-keyword",
    "png-iccp-name-80",
    "png-iccp-name-empty",
    "png-exif-basic-le",
    "png-xmp-itxt",
    "png-iccp-valid",
];

/// The record of those warnings: this file name, in the PNG record directory.
pub const WARNINGS_RECORD: &str = "png-warnings.txt";

/// The name of the TIFF fixture a wrapped fixture was built from.
pub fn unwrapped_name(png_name: &str) -> Option<&str> {
    png_name.strip_prefix("png-")
}
