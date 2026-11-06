//! Hand-built JPEG fixtures for the ExifTool parity oracle (SPEC-011).
//!
//! Tier A: every byte is written here from the JPEG marker syntax (ITU-T T.81
//! Annex B), the Exif APP1 layout (`Exif\0\0` then a TIFF), the XMP packet
//! embedding (`http://ns.adobe.com/xap/1.0/\0` then the packet) and the
//! Photoshop `8BIM` resource layout; the IPTC dataset header is `0x1C`, record
//! number, dataset number, two-byte length. The builder is independent of
//! `src/`, of `examples/fuzz_seeds.rs` and of the library's `#[cfg(test)]`
//! builders: an oracle fixture must not share its encoding code with the code it
//! checks. The TIFF blobs come from `oracle_tiff`, which is independent too.
//!
//! Deterministic: [`fixtures`] is a pure function of this file and of
//! `oracle_tiff::fixtures()`. It is the single named list used by the recorder,
//! the live tests and the replay tests. std only.
//!
//! Names are normative (SPEC-011): `jpeg-<tiff fixture name>` for the wrapped
//! TIFFs and the `jpeg-...` shape names below. The shapes' content is described
//! on each builder; what ExifTool and `exif-core` read from them is measured,
//! never assumed (`tests/oracle-fixtures/exiftool/jpeg-*.txt`).

#![allow(dead_code)]

use crate::oracle_tiff as t;

pub const SOI: [u8; 2] = [0xFF, 0xD8];
pub const EOI: [u8; 2] = [0xFF, 0xD9];

/// One length-carrying segment: `FF marker length payload`, the length counting
/// its own two bytes.
pub fn seg(marker: u8, payload: &[u8]) -> Vec<u8> {
    let len = u16::try_from(payload.len() + 2).expect("segment payload fits a 16-bit length");
    let mut v = vec![0xFF, marker];
    v.extend_from_slice(&len.to_be_bytes());
    v.extend_from_slice(payload);
    v
}

/// A start-of-scan header (one component) with no scan data.
pub fn sos_header() -> Vec<u8> {
    seg(0xDA, &[1, 1, 0, 0, 0x3F, 0])
}

/// Two bytes of entropy-coded data (no `FF`, so no marker inside).
pub const SCAN: [u8; 2] = [0x12, 0x34];

/// APP1 `Exif\0\0` + `tiff`.
pub fn exif_app1(tiff: &[u8]) -> Vec<u8> {
    seg(0xE1, &[&b"Exif\0\0"[..], tiff].concat())
}

/// The longest TIFF blob one Exif APP1 can carry (65535 - 2 length bytes - 6
/// signature bytes).
pub const MAX_TIFF_IN_APP1: usize = 65_535 - 2 - 6;

fn jfif() -> Vec<u8> {
    let mut p = b"JFIF\0".to_vec();
    p.extend_from_slice(&[1, 1, 0, 0, 1, 0, 1, 0, 0]);
    seg(0xE0, &p)
}

fn com(text: &[u8]) -> Vec<u8> {
    seg(0xFE, text)
}

fn dqt() -> Vec<u8> {
    let mut p = vec![0];
    p.extend_from_slice(&[16; 64]);
    seg(0xDB, &p)
}

fn cat(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

/// A clean JPEG around `segs`: SOI, the segments, an SOS header and two scan
/// bytes, EOI.
fn clean(segs: &[Vec<u8>]) -> Vec<u8> {
    let mut d = SOI.to_vec();
    for s in segs {
        d.extend_from_slice(s);
    }
    d.extend_from_slice(&sos_header());
    d.extend_from_slice(&SCAN);
    d.extend_from_slice(&EOI);
    d
}

/// A tiny TIFF whose IFD0 holds the given ASCII `(tag, text)` entries (each
/// NUL-terminated here), little-endian.
fn tiff_of(entries: &[(u16, &str)]) -> Vec<u8> {
    let ents: Vec<t::Ent> = entries
        .iter()
        .map(|(tag, text)| t::ascii(*tag, &[text.as_bytes(), b"\0"].concat()))
        .collect();
    t::assemble(false, &[t::ifd(ents)])
}

fn basic() -> Vec<u8> {
    named_tiff("exif-basic-le")
}

fn named_tiff(name: &str) -> Vec<u8> {
    t::fixtures()
        .into_iter()
        .find(|(n, _)| *n == name)
        .map(|(_, b)| b)
        .unwrap_or_else(|| panic!("no TIFF fixture named {name}"))
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

const XMP_SIG: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";
const EXT_XMP_SIG: &[u8] = b"http://ns.adobe.com/xmp/extension/\0";

fn xmp_seg(marker: u8) -> Vec<u8> {
    seg(marker, &[XMP_SIG, &xmp_packet()].concat())
}

/// One extended-XMP chunk: signature, a 32-byte GUID, the full length, this
/// chunk's offset, then the data.
fn ext_xmp_chunk(offset: u32, data: &[u8]) -> Vec<u8> {
    let mut p = EXT_XMP_SIG.to_vec();
    p.extend_from_slice(b"0123456789ABCDEF0123456789ABCDEF");
    p.extend_from_slice(&16u32.to_be_bytes());
    p.extend_from_slice(&offset.to_be_bytes());
    p.extend_from_slice(data);
    seg(0xE1, &p)
}

/// One Photoshop image resource block: `8BIM`, id, a Pascal name padded to an
/// even byte count, a four-byte size, the data padded to even.
fn resource(id: u16, data: &[u8]) -> Vec<u8> {
    let mut v = b"8BIM".to_vec();
    v.extend_from_slice(&id.to_be_bytes());
    v.extend_from_slice(&[0, 0]); // empty Pascal name + its pad byte
    v.extend_from_slice(&u32::try_from(data.len()).expect("short").to_be_bytes());
    v.extend_from_slice(data);
    if data.len() % 2 == 1 {
        v.push(0);
    }
    v
}

/// One IPTC-IIM dataset (record 2): `1C 02 <dataset> <len16> <data>`.
fn iim_dataset(dataset: u8, data: &[u8]) -> Vec<u8> {
    let mut v = vec![0x1C, 0x02, dataset];
    v.extend_from_slice(&u16::try_from(data.len()).expect("short").to_be_bytes());
    v.extend_from_slice(data);
    v
}

/// An APP13 `Photoshop 3.0\0` segment holding the given resource blocks.
fn photoshop_seg(resources: &[Vec<u8>]) -> Vec<u8> {
    seg(
        0xED,
        &[&b"Photoshop 3.0\0"[..], &resources.concat()].concat(),
    )
}

/// Record 2, dataset 5 is ObjectName; dataset 80 is By-line.
fn iptc_data() -> Vec<u8> {
    [
        iim_dataset(5, b"DiamondTitle"),
        iim_dataset(80, b"A. Byline"),
    ]
    .concat()
}

// ── the shape fixtures ──────────────────────────────────────────────────────

/// Two Exif APP1s with the same tag (Make) at different values, plus a tag only
/// the first has (Model). `a_then_b` orders them.
fn two_exif_same_tag(a_then_b: bool) -> Vec<u8> {
    let a = exif_app1(&tiff_of(&[
        (t::T_MAKE, "AlphaCam"),
        (t::T_MODEL, "OnlyInA"),
    ]));
    let b = exif_app1(&tiff_of(&[(t::T_MAKE, "BravoCam")]));
    if a_then_b {
        clean(&[a, b])
    } else {
        clean(&[b, a])
    }
}

/// Two Exif APP1s with disjoint tags (Make in the first, Model in the second).
fn two_exif_disjoint() -> Vec<u8> {
    clean(&[
        exif_app1(&tiff_of(&[(t::T_MAKE, "AlphaCam")])),
        exif_app1(&tiff_of(&[(t::T_MODEL, "BravoModel")])),
    ])
}

/// Seventeen Exif APP1s; the first sixteen say Make = `SameFirst16`, only the
/// seventeenth says `LastOnly17`. `exif-core` parses at most sixteen blocks.
fn seventeen_exif() -> Vec<u8> {
    let mut segs = Vec::new();
    for _ in 0..16 {
        segs.push(exif_app1(&tiff_of(&[(t::T_MAKE, "SameFirst16")])));
    }
    segs.push(exif_app1(&tiff_of(&[(t::T_MAKE, "LastOnly17")])));
    clean(&segs)
}

/// The shape fixtures, in a fixed order.
fn shapes() -> Vec<(&'static str, Vec<u8>)> {
    let basic = basic();
    let exif = exif_app1(&basic);
    let with_sig = |sig: &[u8]| seg(0xE1, &[sig, &basic].concat());

    // SOI, Exif, DQT, and the file ends there: the cut is between APP1 (and a
    // later non-scan segment) and the start of scan.
    let truncated_between = cat(&[&SOI, &exif, &dqt()]);
    // SOI, Exif, SOS header and one scan byte; no EOI.
    let truncated_mid_scan = cat(&[&SOI, &exif, &sos_header(), &SCAN[..1]]);
    // The Exif segment's declared length runs past the end of the file.
    let mut truncated_mid_app1 = cat(&[&SOI, &exif]);
    truncated_mid_app1.truncate(SOI.len() + 4 + 40);

    let mut bad_length = SOI.to_vec();
    bad_length.extend_from_slice(&[0xFF, 0xE1, 0x00, 0x01]);
    bad_length.extend_from_slice(&com(b"after"));
    bad_length.extend_from_slice(&sos_header());
    bad_length.extend_from_slice(&SCAN);
    bad_length.extend_from_slice(&EOI);

    vec![
        // JFIF APP0 before and after the Exif.
        ("jpeg-jfif-then-exif", clean(&[jfif(), exif.clone()])),
        ("jpeg-exif-then-jfif", clean(&[exif.clone(), jfif()])),
        // The Exif signature under APP2 is not Exif.
        ("jpeg-exif-in-app2", clean(&[with_sig_in(0xE2, &basic)])),
        // DEC-007: only the first five signature bytes matter.
        (
            "jpeg-exif-sixth-byte-nonzero",
            clean(&[with_sig(b"Exif\0\x01")]),
        ),
        (
            "jpeg-exif-fifth-byte-nonzero",
            clean(&[with_sig(b"Exif\x01\0")]),
        ),
        (
            "jpeg-exif-five-byte-payload",
            clean(&[seg(0xE1, b"Exif\0")]),
        ),
        // Several Exif segments.
        ("jpeg-two-exif-same-tag", two_exif_same_tag(true)),
        ("jpeg-two-exif-same-tag-reversed", two_exif_same_tag(false)),
        ("jpeg-two-exif-disjoint-tags", two_exif_disjoint()),
        ("jpeg-seventeen-exif-last-differs", seventeen_exif()),
        // Where the Exif sits in the segment stream.
        (
            "jpeg-exif-after-scan",
            cat(&[&SOI, &sos_header(), &SCAN, &exif, &EOI]),
        ),
        ("jpeg-exif-after-eoi", cat(&[&SOI, &EOI, &exif])),
        ("jpeg-exif-last-segment-at-eof", cat(&[&SOI, &exif])),
        (
            "jpeg-exif-then-com-at-eof",
            cat(&[&SOI, &exif, &com(b"tail")]),
        ),
        ("jpeg-exif-eoi-no-sos", cat(&[&SOI, &exif, &EOI])),
        // Truncation.
        ("jpeg-truncated-between-app1-and-sos", truncated_between),
        ("jpeg-truncated-mid-scan", truncated_mid_scan),
        ("jpeg-truncated-mid-app1", truncated_mid_app1),
        // Damage in the segment stream.
        // Three stray bytes between two COM segments (measured, ExifTool 13.55:
        // it warns when the stray bytes sit between segments after a COM, and
        // not when they directly follow the APP1 or lead into the SOS).
        (
            "jpeg-stray-bytes",
            cat(&[
                &SOI,
                &exif,
                &com(b"hi"),
                &[1, 2, 3],
                &com(b"ho"),
                &sos_header(),
                &SCAN,
                &EOI,
            ]),
        ),
        ("jpeg-bad-length", bad_length),
        (
            "jpeg-bad-tiff-in-exif",
            clean(&[exif_app1(b"junkjunkjunkjunk")]),
        ),
        // Located regions (presence only).
        ("jpeg-xmp", clean(&[xmp_seg(0xE1)])),
        ("jpeg-xmp-in-wrong-marker", clean(&[xmp_seg(0xE2)])),
        (
            "jpeg-iptc",
            clean(&[photoshop_seg(&[resource(0x0404, &iptc_data())])]),
        ),
        (
            "jpeg-iptc-without-0404",
            clean(&[photoshop_seg(&[resource(0x0408, &iptc_data())])]),
        ),
        (
            "jpeg-extended-xmp-pair",
            clean(&[
                xmp_seg(0xE1),
                ext_xmp_chunk(0, b"first"),
                ext_xmp_chunk(5, b"second"),
            ]),
        ),
    ]
}

/// `Exif\0\0` + TIFF under an arbitrary marker.
fn with_sig_in(marker: u8, tiff: &[u8]) -> Vec<u8> {
    seg(marker, &[&b"Exif\0\0"[..], tiff].concat())
}

/// Every TIFF fixture, wrapped as `SOI, APP1 Exif\0\0 + tiff, SOS header + two
/// scan bytes, EOI`, named `jpeg-<tiff-name>`.
pub fn wrapped() -> Vec<(String, Vec<u8>)> {
    t::fixtures()
        .into_iter()
        .map(|(name, tiff)| {
            assert!(
                tiff.len() <= MAX_TIFF_IN_APP1,
                "{name}: {} bytes does not fit one Exif APP1",
                tiff.len()
            );
            (format!("jpeg-{name}"), clean(&[exif_app1(&tiff)]))
        })
        .collect()
}

/// The single named list: the wrapped TIFF fixtures first, then the shapes.
/// Order and names are stable; the committed records are keyed by name.
pub fn fixtures() -> Vec<(String, Vec<u8>)> {
    let mut out = wrapped();
    out.extend(shapes().into_iter().map(|(n, b)| (n.to_string(), b)));
    out
}

/// The bytes of the TIFF fixture a wrapped fixture was built from.
pub fn unwrapped_name(jpeg_name: &str) -> Option<&str> {
    jpeg_name.strip_prefix("jpeg-")
}
