//! Fixtures for the strip writer (SPEC-020): deterministic JPEG builders, each
//! with a canary byte string in every metadata carrier, and a record of what
//! every byte range of the file is, written by the builder as it appends (not
//! recovered by parsing). Tier A: nothing here imports `src/`; the TIFF blobs of
//! Exif come from `oracle_tiff` and the Multi-Picture index from `real_shapes`.
//!
//! [`expected_bytes`] is the tiny independent implementation of the output
//! contract: it deletes ranges and applies patches without `EditPlan`.
//! Nothing here is copied from a real file; the local corpus run is counts only.

#![allow(dead_code)]

use std::ops::Range;

use crate::oracle_tiff as t;
use crate::real_shapes as rs;

// ── canaries ────────────────────────────────────────────────────────────────

pub const C_MAKE: &[u8] = b"CANARY-EXIF-MAKE";
pub const C_MODEL: &[u8] = b"CANARY-EXIF-MODEL";
pub const C_ARTIST: &[u8] = b"CANARY-EXIF-ARTIST";
pub const C_SOFTWARE: &[u8] = b"CANARY-EXIF-SOFTWARE";
pub const C_DATETIME: &[u8] = b"2031:02:03 04:05:06";
pub const C_SERIAL: &[u8] = b"CANARY-EXIF-SERIAL";
pub const C_USERCOMMENT: &[u8] = b"CANARY-EXIF-USERCOMMENT";
pub const C_NOTE: &[u8] = b"CANARY-EXIF-MAKERNOTE";
pub const C_GPS: &[u8] = b"CANARY-GPS-AREA";
pub const C_THUMB: &[u8] = b"CANARY-EXIF-THUMBNAIL";
pub const C_XMP: &[u8] = b"CANARY-XMP-PACKET";
pub const C_XMPEXT: &[u8] = b"CANARY-XMP-EXTENDED";
pub const C_IPTC1: &[u8] = b"CANARY-IPTC-ONE";
pub const C_IPTC2: &[u8] = b"CANARY-IPTC-TWO";
pub const C_JUMBF: &[u8] = b"CANARY-JUMBF";
pub const C_COM1: &[u8] = b"CANARY-COMMENT-ONE";
pub const C_COM2: &[u8] = b"CANARY-COMMENT-TWO";
pub const C_APP10: &[u8] = b"CANARY-APP10-AROT";
pub const C_TRAILER: &[u8] = b"CANARY-TRAILER-BYTES";
/// The text of the profile's copyright tag: a kept carrier under `All`.
pub const C_ICC: &[u8] = b"CANARY-ICC-COPYRIGHT";

/// Every canary of a removed carrier (gone after `All`), and the one kept.
pub const REMOVED_CANARIES: &[&[u8]] = &[
    C_MAKE,
    C_MODEL,
    C_ARTIST,
    C_SOFTWARE,
    C_SERIAL,
    C_USERCOMMENT,
    C_NOTE,
    C_GPS,
    C_THUMB,
    C_XMP,
    C_XMPEXT,
    C_IPTC1,
    C_IPTC2,
    C_COM1,
    C_COM2,
];

// ── the record of what each range is ────────────────────────────────────────

/// What a range of a fixture is, as the builder knows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum K {
    Soi,
    Jfif,
    Exif,
    Xmp,
    ExtXmp,
    Icc,
    Iptc,
    Jumbf,
    Com,
    /// An unrecognised APPn (APP10 `AROT`, an unparsed `MPF\0`, ...).
    OtherApp,
    Adobe,
    Dqt,
    Sof,
    Dht,
    Dri,
    /// The Multi-Picture index segment (the one the audit indexes).
    MpfIndex,
    /// From the `SOS` marker through the `EOI` (or the end of the input).
    Scan,
    /// A piece of the trailer no image accounts for.
    Trailer,
    /// A Multi-Picture image other than the primary.
    Image,
    /// Bytes the walk skips between segments (fill, stray).
    Stray,
}

#[derive(Clone, Debug)]
pub struct Part {
    pub kind: K,
    pub range: Range<usize>,
}

/// What the Exif segments of a fixture hold, as the builder wrote them.
#[derive(Clone, Debug, Default)]
pub struct ExifFacts {
    /// Number of Exif segments.
    pub blocks: usize,
    /// Every Exif segment's TIFF parses (IFD0 readable).
    pub readable: bool,
    /// EXIF9 category names present in them (`"gps"`, `"timestamps"`, ...).
    pub cats: Vec<&'static str>,
}

#[derive(Clone, Debug)]
pub struct Shape {
    pub name: &'static str,
    pub bytes: Vec<u8>,
    pub parts: Vec<Part>,
    pub exif: ExifFacts,
    /// A refusal the library must give for every policy, if any.
    pub refused: bool,
    /// Whether the fixture has a Multi-Picture index.
    pub mpf: Option<MpfFacts>,
}

#[derive(Clone, Debug)]
pub struct MpfFacts {
    pub big_endian: bool,
    /// Input offset of the byte after `MPF\0`.
    pub header_start: usize,
    /// `(size_field, offset_field)` input offsets per entry.
    pub fields: Vec<(usize, usize)>,
    /// The index entries as written `(size, offset)`.
    pub entries: Vec<(u32, u32)>,
    /// Input offset of the first byte after the primary's `EOI`.
    pub primary_end: usize,
}

impl Shape {
    pub fn parts_of(&self, kind: K) -> Vec<Range<usize>> {
        self.parts
            .iter()
            .filter(|p| p.kind == kind)
            .map(|p| p.range.clone())
            .collect()
    }
}

/// Appends items and records their ranges.
struct B {
    bytes: Vec<u8>,
    parts: Vec<Part>,
}

impl B {
    fn new() -> B {
        let mut b = B {
            bytes: Vec::new(),
            parts: Vec::new(),
        };
        b.push(K::Soi, &rs::SOI);
        b
    }
    fn push(&mut self, kind: K, bytes: &[u8]) {
        let start = self.bytes.len();
        self.bytes.extend_from_slice(bytes);
        self.parts.push(Part {
            kind,
            range: start..self.bytes.len(),
        });
    }
    fn len(&self) -> usize {
        self.bytes.len()
    }
}

// ── JPEG pieces ─────────────────────────────────────────────────────────────

fn jfif() -> Vec<u8> {
    let mut p = b"JFIF\0".to_vec();
    p.extend_from_slice(&[1, 1, 0, 0, 1, 0, 1, 0, 0]);
    rs::seg(0xE0, &p)
}

fn dqt() -> Vec<u8> {
    rs::seg(0xDB, &[&[0u8][..], &[8u8; 64][..]].concat())
}

fn sof0() -> Vec<u8> {
    rs::seg(0xC0, &[8, 0, 8, 0, 8, 1, 1, 0x11, 0])
}

fn dht() -> Vec<u8> {
    let mut p = vec![0x00u8];
    p.extend_from_slice(&[0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    p.push(0);
    rs::seg(0xC4, &p)
}

fn dri() -> Vec<u8> {
    rs::seg(0xDD, &[0, 4])
}

fn adobe() -> Vec<u8> {
    rs::seg(
        0xEE,
        &[&b"Adobe\0"[..], &[0, 100, 0, 0, 0, 0, 1][..]].concat(),
    )
}

/// `SOS`, entropy bytes (with a stuffed `FF 00` and a restart marker, which are
/// data), `EOI`.
fn scan() -> Vec<u8> {
    let mut v = rs::seg(0xDA, &[1, 1, 0, 0, 0x3F, 0]);
    v.extend_from_slice(&[0x12, 0x34, 0xFF, 0x00, 0x56, 0xFF, 0xD0, 0x78, 0x9A]);
    v.extend_from_slice(&rs::EOI);
    v
}

fn xmp_seg(extra: &[u8]) -> Vec<u8> {
    xmp_seg_with(extra, None)
}

/// The main XMP packet; with a GUID it also names the extended packet.
fn xmp_seg_with(extra: &[u8], ext_guid: Option<&[u8]>) -> Vec<u8> {
    let mut packet = b"<?xpacket begin=\"\xEF\xBB\xBF\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
        <x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF \
        xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
        <rdf:Description rdf:about=\"\" xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\" \
        xmlns:xmpNote=\"http://ns.adobe.com/xmp/note/\">"
        .to_vec();
    if let Some(guid) = ext_guid {
        packet.extend_from_slice(b"<xmpNote:HasExtendedXMP>");
        packet.extend_from_slice(guid);
        packet.extend_from_slice(b"</xmpNote:HasExtendedXMP>");
    }
    packet.extend_from_slice(b"<xmp:CreatorTool>");
    packet.extend_from_slice(extra);
    packet.extend_from_slice(
        b"</xmp:CreatorTool></rdf:Description></rdf:RDF></x:xmpmeta>\n<?xpacket end=\"w\"?>",
    );
    rs::seg(
        0xE1,
        &[&b"http://ns.adobe.com/xap/1.0/\0"[..], &packet].concat(),
    )
}

fn ext_xmp_seg(guid_fill: u8, full: u32, offset: u32, data: &[u8]) -> Vec<u8> {
    let mut p = b"http://ns.adobe.com/xmp/extension/\0".to_vec();
    // The GUID is 32 ASCII hex digits.
    p.extend_from_slice(&[guid_fill; 32]);
    p.extend_from_slice(&full.to_be_bytes());
    p.extend_from_slice(&offset.to_be_bytes());
    p.extend_from_slice(data);
    rs::seg(0xE1, &p)
}

fn icc_chunks() -> (Vec<u8>, Vec<u8>) {
    // Header, a one-entry tag table (copyright), the text tag.
    let mut text = b"text\0\0\0\0".to_vec();
    text.extend_from_slice(C_ICC);
    text.push(0);
    while (text.len() & 3) != 0 {
        text.push(0);
    }
    let size = 128 + 4 + 12 + text.len();
    let mut profile = vec![0u8; 128];
    profile[0..4].copy_from_slice(&u32::try_from(size).unwrap().to_be_bytes());
    profile[8..12].copy_from_slice(&[2, 0x10, 0, 0]);
    profile[12..16].copy_from_slice(b"mntr");
    profile[16..20].copy_from_slice(b"RGB ");
    profile[20..24].copy_from_slice(b"XYZ ");
    profile[36..40].copy_from_slice(b"acsp");
    profile.extend_from_slice(&1u32.to_be_bytes());
    profile.extend_from_slice(b"cprt");
    profile.extend_from_slice(&144u32.to_be_bytes());
    profile.extend_from_slice(&u32::try_from(text.len()).unwrap().to_be_bytes());
    profile.extend_from_slice(&text);
    // Two chunks, split inside the tag table.
    let (a, b) = profile.split_at(136);
    let mk = |seq: u8, d: &[u8]| rs::seg(0xE2, &[&b"ICC_PROFILE\0"[..], &[seq, 2][..], d].concat());
    (mk(1, a), mk(2, b))
}

fn iptc_seg(caption: &[u8]) -> Vec<u8> {
    let mut data = vec![0x1C, 0x02, 0x78];
    data.extend_from_slice(&u16::try_from(caption.len()).unwrap().to_be_bytes());
    data.extend_from_slice(caption);
    let mut res = b"8BIM".to_vec();
    res.extend_from_slice(&0x0404u16.to_be_bytes());
    res.extend_from_slice(&[0, 0]);
    res.extend_from_slice(&u32::try_from(data.len()).unwrap().to_be_bytes());
    res.extend_from_slice(&data);
    if data.len() % 2 == 1 {
        res.push(0);
    }
    rs::seg(0xED, &[&b"Photoshop 3.0\0"[..], &res].concat())
}

fn jumbf_seg() -> Vec<u8> {
    // A superbox holding one description box (a 16-byte type, a toggles byte
    // and a NUL-terminated label).
    let mut label = C_JUMBF.to_vec();
    label.push(0);
    let jumd_len = 8 + 16 + 1 + label.len();
    let jumb_len = 8 + jumd_len;
    let mut p = b"JP".to_vec();
    p.extend_from_slice(&[0, 1]);
    p.extend_from_slice(&1u32.to_be_bytes());
    p.extend_from_slice(&u32::try_from(jumb_len).unwrap().to_be_bytes());
    p.extend_from_slice(b"jumb");
    p.extend_from_slice(&u32::try_from(jumd_len).unwrap().to_be_bytes());
    p.extend_from_slice(b"jumd");
    p.extend_from_slice(&[
        0x63, 0x32, 0x70, 0x61, 0, 0x11, 0, 0x10, 0x80, 0, 0, 0xAA, 0, 0x38, 0x9B, 0x71,
    ]);
    p.push(3);
    p.extend_from_slice(&label);
    rs::seg(0xEB, &p)
}

fn arot_seg() -> Vec<u8> {
    rs::seg(
        0xEA,
        &[&b"AROT\0"[..], &[0, 0, 0, 1, 0, 0, 0, 0][..], C_APP10].concat(),
    )
}

fn com_seg(text: &[u8]) -> Vec<u8> {
    rs::seg(0xFE, text)
}

fn exif_seg(tiff: &[u8]) -> Vec<u8> {
    rs::exif_app1(tiff)
}

// ── Exif blobs ──────────────────────────────────────────────────────────────

fn z(s: &[u8]) -> Vec<u8> {
    [s, b"\0"].concat()
}

/// IFD0 (Make, Model, Orientation, Artist, Software, DateTime), Exif IFD
/// (DateTimeOriginal, BodySerialNumber, UserComment, MakerNote), GPS IFD
/// (latitude, longitude, GPSAreaInformation) and IFD1 with a thumbnail.
pub fn full_tiff(big: bool) -> Vec<u8> {
    let build = |thumb_at: u32| {
        let ifd0 = t::ifd_next(
            vec![
                t::ascii(t::T_MAKE, &z(C_MAKE)),
                t::ascii(t::T_MODEL, &z(C_MODEL)),
                t::short(big, t::T_ORIENTATION, 6),
                t::ascii(t::T_ARTIST, &z(C_ARTIST)),
                t::ascii(t::T_SOFTWARE, &z(C_SOFTWARE)),
                t::ascii(t::T_DATE_TIME, &z(C_DATETIME)),
                t::ptr(t::T_EXIF_IFD, 1),
                t::ptr(t::T_GPS_IFD, 2),
            ],
            3,
        );
        let exif = t::ifd(vec![
            t::ascii(t::T_DATE_TIME_ORIGINAL, &z(C_DATETIME)),
            t::ascii(0xA431, &z(C_SERIAL)),
            // UserComment: UNDEFINED, 8-byte character code then text.
            t::ascii(0x9286, &[&b"ASCII\0\0\0"[..], C_USERCOMMENT].concat()),
            t::ascii(0x927C, C_NOTE),
        ]);
        let gps = t::ifd(vec![
            t::ascii(t::T_GPS_LAT_REF, b"N\0"),
            t::rational(big, t::T_GPS_LAT, &[(50, 1), (4, 1), (0, 1)]),
            t::ascii(t::T_GPS_LON_REF, b"E\0"),
            t::rational(big, t::T_GPS_LON, &[(19, 1), (56, 1), (0, 1)]),
            t::ascii(0x001C, &[&b"ASCII\0\0\0"[..], C_GPS].concat()),
        ]);
        let ifd1 = t::ifd(vec![
            t::ptr_raw(0x0201, thumb_at),
            t::long(big, 0x0202, u32::try_from(thumb().len()).unwrap()),
        ]);
        t::assemble(big, &[ifd0, exif, gps, ifd1])
    };
    let probe = build(0);
    let mut out = build(u32::try_from(probe.len()).unwrap());
    // `ptr_raw` writes in the file's own order via the builder; the thumbnail
    // sits at the end of the blob.
    out.extend_from_slice(&thumb());
    out
}

fn thumb() -> Vec<u8> {
    [&[0xFF, 0xD8][..], C_THUMB, &[0xFF, 0xD9][..]].concat()
}

fn orientation_tiff() -> Vec<u8> {
    t::assemble(false, &[t::ifd(vec![t::short(false, t::T_ORIENTATION, 6)])])
}

fn no_gps_tiff() -> Vec<u8> {
    t::assemble(
        false,
        &[t::ifd(vec![
            t::ascii(t::T_MAKE, &z(C_MAKE)),
            t::ascii(t::T_MODEL, &z(C_MODEL)),
            t::ascii(t::T_SOFTWARE, &z(C_SOFTWARE)),
        ])],
    )
}

const FULL_CATS: &[&str] = &[
    "gps",
    "device_id",
    "authorship",
    "free_text",
    "timestamps",
    "camera_model",
    "thumbnail",
    "maker_note",
    "software",
];

// ── fixtures ────────────────────────────────────────────────────────────────

fn finish(name: &'static str, b: B, exif: ExifFacts) -> Shape {
    Shape {
        name,
        bytes: b.bytes,
        parts: b.parts,
        exif,
        refused: false,
        mpf: None,
    }
}

/// Everything at once. `with_exif` false builds `no_exif`.
fn sink(name: &'static str, with_exif: bool) -> Shape {
    let mut b = B::new();
    b.push(K::Jfif, &jfif());
    let mut facts = ExifFacts {
        readable: true,
        ..ExifFacts::default()
    };
    if with_exif {
        b.push(K::Exif, &exif_seg(&full_tiff(false)));
        facts.blocks = 1;
        facts.cats = FULL_CATS.to_vec();
    }
    // ExifTool reads a JUMBF segment only before the Photoshop one (measured).
    b.push(K::Jumbf, &jumbf_seg());
    let guid = [b'a'; 32];
    b.push(K::Xmp, &xmp_seg_with(C_XMP, Some(&guid)));
    // One extended packet in two chunks, split after the canary.
    let ext = [
        &b"<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF \
        xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
        <rdf:Description rdf:about=\"\" xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\">\
        <xmp:Label>"[..],
        C_XMPEXT,
        &b"</xmp:Label></rdf:Description></rdf:RDF></x:xmpmeta>"[..],
    ]
    .concat();
    let cut = ext.len() - 20;
    let full = u32::try_from(ext.len()).unwrap();
    b.push(K::ExtXmp, &ext_xmp_seg(guid[0], full, 0, &ext[..cut]));
    b.push(
        K::ExtXmp,
        &ext_xmp_seg(guid[0], full, u32::try_from(cut).unwrap(), &ext[cut..]),
    );
    let (i1, i2) = icc_chunks();
    b.push(K::Icc, &i1);
    b.push(K::Icc, &i2);
    b.push(K::Iptc, &iptc_seg(C_IPTC1));
    b.push(K::Iptc, &iptc_seg(C_IPTC2));
    b.push(K::Com, &com_seg(C_COM1));
    b.push(K::Com, &com_seg(C_COM2));
    b.push(K::OtherApp, &arot_seg());
    b.push(K::Adobe, &adobe());
    b.push(K::Dqt, &dqt());
    b.push(K::Sof, &sof0());
    b.push(K::Dht, &dht());
    b.push(K::Dri, &dri());
    b.push(K::Scan, &scan());
    let mut trailer = C_TRAILER.to_vec();
    trailer.extend((0..313 - C_TRAILER.len()).map(|i| 0x41 + (i % 23) as u8));
    b.push(K::Trailer, &trailer);
    finish(name, b, facts)
}

pub fn kitchen_sink() -> Shape {
    sink("kitchen-sink", true)
}

pub fn no_exif() -> Shape {
    sink("no-exif", false)
}

fn small(name: &'static str, items: &[(K, Vec<u8>)], exif: ExifFacts) -> Shape {
    let mut b = B::new();
    for (k, bytes) in items {
        b.push(*k, bytes);
    }
    b.push(K::Scan, &scan());
    finish(name, b, exif)
}

pub fn exif_orientation_only() -> Shape {
    small(
        "exif-orientation-only",
        &[
            (K::Jfif, jfif()),
            (K::Exif, exif_seg(&orientation_tiff())),
            (K::Com, com_seg(C_COM1)),
        ],
        ExifFacts {
            blocks: 1,
            readable: true,
            cats: vec![],
        },
    )
}

pub fn exif_without_gps() -> Shape {
    small(
        "exif-without-gps",
        &[
            (K::Jfif, jfif()),
            (K::Exif, exif_seg(&no_gps_tiff())),
            (K::Xmp, xmp_seg(C_XMP)),
        ],
        ExifFacts {
            blocks: 1,
            readable: true,
            cats: vec!["camera_model", "software"],
        },
    )
}

pub fn two_exif_blocks() -> Shape {
    small(
        "two-exif-blocks",
        &[
            (K::Jfif, jfif()),
            (K::Exif, exif_seg(&no_gps_tiff())),
            (K::Exif, exif_seg(&orientation_tiff())),
        ],
        ExifFacts {
            blocks: 2,
            readable: true,
            cats: vec!["camera_model", "software"],
        },
    )
}

/// Numerators of the GPS rationals of `gps_count_overflow`: four ASCII letters
/// each, so the canary is a plain byte search.
pub const C_GPSNUM: &[u8] = b"CANA";

/// A big-endian Exif whose GPS IFD declares 0xFFFF entries: the tolerant walk
/// reports an issue, finds no GPS entry (so no GPS finding) and the block is
/// still readable (IFD0 parses). The latitude and longitude bytes are in the
/// file. (Found by the verifier of SPEC-020.)
pub fn gps_count_overflow() -> Shape {
    let tiff = t::assemble(
        true,
        &[
            t::ifd(vec![
                t::ascii(t::T_MAKE, &z(C_MAKE)),
                t::ptr(t::T_GPS_IFD, 1),
            ]),
            t::ifd(vec![
                t::ascii(t::T_GPS_LAT_REF, b"N\0"),
                t::rational(
                    true,
                    t::T_GPS_LAT,
                    &[(0x4341_4E41, 1), (0x4341_4E41, 1), (0x4341_4E41, 1)],
                ),
            ]),
        ],
    );
    // IFD0 is at 8: find the GPS pointer and set the count at its target.
    let mut tiff = tiff;
    let n = usize::from(u16::from_be_bytes([tiff[8], tiff[9]]));
    let at = (0..n)
        .map(|i| 10 + 12 * i)
        .find(|&e| u16::from_be_bytes([tiff[e], tiff[e + 1]]) == t::T_GPS_IFD)
        .unwrap();
    let gps = u32::from_be_bytes(tiff[at + 8..at + 12].try_into().unwrap()) as usize;
    tiff[gps..gps + 2].copy_from_slice(&0xFFFFu16.to_be_bytes());
    small(
        "gps-count-overflow",
        &[(K::Jfif, jfif()), (K::Exif, exif_seg(&tiff))],
        ExifFacts {
            blocks: 1,
            readable: false,
            cats: vec![],
        },
    )
}

pub fn unreadable_exif() -> Shape {
    small(
        "unreadable-exif",
        &[
            (K::Jfif, jfif()),
            (K::Exif, rs::seg(0xE1, b"Exif\0\0this is not a tiff header")),
            (K::Xmp, xmp_seg(C_XMP)),
        ],
        ExifFacts {
            blocks: 1,
            readable: false,
            cats: vec![],
        },
    )
}

pub fn comment_only() -> Shape {
    small(
        "comment-only",
        &[(K::Jfif, jfif()), (K::Com, com_seg(C_COM1))],
        ExifFacts::default(),
    )
}

/// Missing `EOI`: the scan runs to the end of the input.
pub fn missing_eoi() -> Shape {
    let mut b = B::new();
    b.push(K::Jfif, &jfif());
    b.push(K::Xmp, &xmp_seg(C_XMP));
    b.push(K::Com, &com_seg(C_COM1));
    let mut s = scan();
    s.truncate(s.len() - 2);
    b.push(K::Scan, &s);
    finish("missing-eoi", b, ExifFacts::default())
}

/// No `SOS` at all: segments, then the end.
pub fn missing_sos() -> Shape {
    let mut b = B::new();
    b.push(K::Jfif, &jfif());
    b.push(K::Xmp, &xmp_seg(C_XMP));
    b.push(K::Com, &com_seg(C_COM1));
    b.push(K::Dqt, &dqt());
    finish("missing-sos", b, ExifFacts::default())
}

/// Runs of `FF` fill before markers, and a byte between segments.
pub fn fill_bytes_between_segments() -> Shape {
    let mut b = B::new();
    b.push(K::Jfif, &jfif());
    b.push(K::Stray, &[0xFF, 0xFF, 0xFF]);
    b.push(K::Xmp, &xmp_seg(C_XMP));
    b.push(K::Stray, &[0xFF]);
    b.push(K::Com, &com_seg(C_COM1));
    b.push(K::Stray, &[0xFF, 0xFF]);
    b.push(K::Dqt, &dqt());
    b.push(K::Scan, &scan());
    finish("fill-bytes", b, ExifFacts::default())
}

/// A segment whose length word is 1.
pub fn bad_length() -> Shape {
    let mut b = B::new();
    b.push(K::Jfif, &jfif());
    b.push(K::Stray, &[0xFF, 0xFE, 0x00, 0x01]);
    b.push(K::Xmp, &xmp_seg(C_XMP));
    b.push(K::Scan, &scan());
    let mut s = finish("bad-length", b, ExifFacts::default());
    s.refused = true;
    s
}

/// More segments than the walk's cap.
pub fn halted_walk() -> Shape {
    let mut b = B::new();
    for _ in 0..4100 {
        b.push(K::Com, &com_seg(b"c"));
    }
    b.push(K::Scan, &scan());
    let mut s = finish("halted-walk", b, ExifFacts::default());
    s.refused = true;
    s
}

/// Where the entries start, relative to the byte after `MPF\0`, in the index
/// `real_shapes::mpf_segment` writes with `MPFVersion`: an 8-byte TIFF header,
/// a 2-byte count, three 12-byte entries and a 4-byte next pointer.
pub const MPF_DATA_AT: usize = 8 + 2 + 3 * 12 + 4;

// ── Multi-Picture ───────────────────────────────────────────────────────────

/// A primary and secondary images behind it. `before` and `after` are the
/// segments around the index (in the primary, before the scan); `images` are
/// `(gap bytes before it, image)`; `tail` follows the last image. The index
/// points each image at its true place.
pub fn mpo_shape(
    name: &'static str,
    be: bool,
    before: Vec<(K, Vec<u8>)>,
    after: Vec<(K, Vec<u8>)>,
    images: Vec<(Vec<u8>, Vec<u8>)>,
    tail: Vec<u8>,
    exif: ExifFacts,
) -> Shape {
    let n = images.len() + 1;
    let layout = |entries: &[(u32, u32, u32)]| {
        let mut b = B::new();
        b.push(K::Jfif, &jfif());
        for (k, bytes) in &before {
            b.push(*k, bytes);
        }
        let mpf_at = b.len();
        b.push(
            K::MpfIndex,
            &rs::mpf_segment(be, entries, u32::try_from(16 * n).unwrap(), true),
        );
        for (k, bytes) in &after {
            b.push(*k, bytes);
        }
        b.push(K::Scan, &scan());
        (b, mpf_at)
    };
    let (probe, mpf_at) = layout(&vec![(0, 0, 0); n]);
    let header_start = mpf_at + 8;
    let primary_end = probe.len();
    let mut entries = vec![(0x2003_0000u32, u32::try_from(primary_end).unwrap(), 0u32)];
    let mut at = primary_end;
    let mut placed = Vec::new();
    for (gap, img) in &images {
        if !gap.is_empty() {
            placed.push((K::Trailer, at..at + gap.len()));
        }
        at += gap.len();
        entries.push((
            0,
            u32::try_from(img.len()).unwrap(),
            u32::try_from(at - header_start).unwrap(),
        ));
        placed.push((K::Image, at..at + img.len()));
        at += img.len();
    }
    if !tail.is_empty() {
        placed.push((K::Trailer, at..at + tail.len()));
    }
    let (mut b, mpf_at2) = layout(&entries);
    assert_eq!(mpf_at, mpf_at2);
    assert_eq!(b.len(), primary_end);
    for ((gap, img), _) in images.iter().zip(0..) {
        b.bytes.extend_from_slice(gap);
        b.bytes.extend_from_slice(img);
    }
    b.bytes.extend_from_slice(&tail);
    b.parts
        .extend(placed.into_iter().map(|(kind, range)| Part { kind, range }));
    let fields = (0..n)
        .map(|i| {
            let e = header_start + MPF_DATA_AT + 16 * i;
            // The size and offset fields follow the 4-byte attribute.
            (e + 4, e + 8)
        })
        .collect();
    let mut s = finish(name, b, exif);
    s.mpf = Some(MpfFacts {
        big_endian: be,
        header_start,
        fields,
        entries: entries.iter().map(|e| (e.1, e.2)).collect(),
        primary_end,
    });
    s
}

fn nikon_exif_facts() -> ExifFacts {
    ExifFacts {
        blocks: 1,
        readable: true,
        cats: vec!["camera_model", "timestamps"],
    }
}

/// The primary has Exif, the MPF index, then an XMP segment and a COM *after*
/// the index; two secondary images, one with its own Exif.
pub fn mpo_late_xmp(be: bool) -> Shape {
    mpo_shape(
        if be {
            "mpo-late-xmp-be"
        } else {
            "mpo-late-xmp-le"
        },
        be,
        vec![(K::Exif, rs::exif_app1(&exif_camera(be)))],
        vec![(K::Xmp, xmp_seg(C_XMP)), (K::Com, com_seg(C_COM1))],
        vec![
            (Vec::new(), rs::nikon_extra()),
            (Vec::new(), rs::iphone_extra()),
        ],
        Vec::new(),
        nikon_exif_facts(),
    )
}

fn exif_camera(big: bool) -> Vec<u8> {
    let ifd0 = t::ifd(vec![
        t::ascii(t::T_MAKE, &z(b"NIKON CORPORATION")),
        t::ascii(t::T_MODEL, &z(b"NIKON D-SHAPE")),
        t::ptr(t::T_EXIF_IFD, 1),
    ]);
    let exif = t::ifd(vec![t::ascii(t::T_DATE_TIME_ORIGINAL, &z(C_DATETIME))]);
    t::assemble(big, &[ifd0, exif])
}

/// Nikon style: gaps between and after the images, an AROT segment, an XMP
/// segment after the index.
pub fn mpo_trailer_gaps() -> Shape {
    mpo_shape(
        "mpo-trailer-gaps",
        false,
        vec![
            (K::Exif, rs::exif_app1(&exif_camera(false))),
            (K::OtherApp, arot_seg()),
        ],
        vec![(K::Xmp, xmp_seg(C_XMP))],
        vec![
            (vec![0x5A; 11], rs::nikon_extra()),
            (vec![0x5B; 7], rs::iphone_extra()),
        ],
        vec![0x5C; 19],
        nikon_exif_facts(),
    )
}

/// An index with no gaps and nothing removable after it: the control for "no
/// patch needed".
pub fn mpo_plain() -> Shape {
    mpo_shape(
        "mpo-plain",
        false,
        vec![(K::Exif, rs::exif_app1(&exif_camera(false)))],
        vec![],
        vec![
            (Vec::new(), rs::nikon_extra()),
            (Vec::new(), rs::iphone_extra()),
        ],
        Vec::new(),
        nikon_exif_facts(),
    )
}

/// An APP2 `MPF\0` segment that does not parse, then a removable segment.
pub fn unindexed_mpf() -> Shape {
    small(
        "unindexed-mpf",
        &[
            (K::Jfif, jfif()),
            (
                K::OtherApp,
                rs::seg(0xE2, b"MPF\0this is not a tiff header"),
            ),
            (K::Xmp, xmp_seg(C_XMP)),
        ],
        ExifFacts::default(),
    )
}

/// An unindexed MPF segment with nothing removable after it.
pub fn unindexed_mpf_last() -> Shape {
    small(
        "unindexed-mpf-last",
        &[
            (K::Jfif, jfif()),
            (K::Xmp, xmp_seg(C_XMP)),
            (
                K::OtherApp,
                rs::seg(0xE2, b"MPF\0this is not a tiff header"),
            ),
        ],
        ExifFacts::default(),
    )
}

/// A parsable index followed by a second `MPF\0` segment (a copy), then a
/// removable segment and a trailer piece.
pub fn second_mpf() -> Shape {
    let mut s = mpo_shape(
        "second-mpf",
        false,
        vec![],
        vec![
            (
                K::OtherApp,
                rs::mpf_segment(false, &[(0x2003_0000, 100, 0), (0, 10, 50)], 32, true),
            ),
            (K::Com, com_seg(C_COM1)),
        ],
        vec![(Vec::new(), rs::iphone_extra())],
        Vec::new(),
        ExifFacts::default(),
    );
    s.name = "second-mpf";
    s
}

/// An index with one entry outside the input (never a region) between two
/// good ones; a removable segment after the index.
pub fn mpf_malformed_entry() -> Shape {
    let mut s = mpo_shape(
        "mpf-malformed-entry",
        false,
        vec![],
        vec![(K::Com, com_seg(C_COM1))],
        vec![
            (Vec::new(), rs::nikon_extra()),
            (Vec::new(), rs::iphone_extra()),
            (Vec::new(), rs::iphone_extra()),
        ],
        Vec::new(),
        ExifFacts::default(),
    );
    // Point entry 2 outside the input; its bytes stay in the file as an
    // unindexed piece of the trailer.
    let m = s.mpf.clone().unwrap();
    let (_, off_field) = m.fields[2];
    let bad = 0x00FF_0000u32;
    let w = if m.big_endian {
        bad.to_be_bytes()
    } else {
        bad.to_le_bytes()
    };
    s.bytes[off_field..off_field + 4].copy_from_slice(&w);
    // The image it used to describe is now a trailer piece.
    for p in &mut s.parts {
        if p.kind == K::Image && p.range.start == m.header_start + m.entries[2].1 as usize {
            p.kind = K::Trailer;
        }
    }
    s.mpf.as_mut().unwrap().entries[2].1 = bad;
    s
}

/// An index entry that points inside the input at bytes that are not a JPEG
/// (so it is no region), ahead of a good image, with a COM after the index.
pub fn mpf_entry_not_jpeg() -> Shape {
    let mut s = mpo_shape(
        "mpf-entry-not-jpeg",
        false,
        vec![],
        vec![(K::Com, com_seg(C_COM1))],
        vec![
            (Vec::new(), vec![0xFF, 0xD9, 1, 2, 3, 4, 5, 6]),
            (Vec::new(), rs::iphone_extra()),
        ],
        Vec::new(),
        ExifFacts::default(),
    );
    // The first secondary's bytes are not an image: a trailer piece.
    let mut first = true;
    for p in &mut s.parts {
        if p.kind == K::Image && first {
            p.kind = K::Trailer;
            first = false;
        }
    }
    s
}

/// Both at once (found by the fuzzer): an index entry that points at bytes that
/// are no image, and a second `MPF\0` segment behind the index, with a COM after
/// both. Pinned order: the index's own entry is reported first.
pub fn second_mpf_and_unlocated_entry() -> Shape {
    let mut s = mpo_shape(
        "second-mpf-unlocated-entry",
        false,
        vec![],
        vec![
            (
                K::OtherApp,
                rs::mpf_segment(false, &[(0x2003_0000, 100, 0), (0, 10, 50)], 32, true),
            ),
            (K::Com, com_seg(C_COM1)),
        ],
        vec![
            (Vec::new(), vec![0xFF, 0xD9, 1, 2, 3, 4, 5, 6]),
            (Vec::new(), rs::iphone_extra()),
        ],
        Vec::new(),
        ExifFacts::default(),
    );
    let mut first = true;
    for p in &mut s.parts {
        if p.kind == K::Image && first {
            p.kind = K::Trailer;
            first = false;
        }
    }
    s
}

/// The index lists an `FF D8` range inside a COM payload of the primary.
pub fn com_over_embedded() -> Shape {
    let inner = rs::iphone_extra();
    let mut payload = b"before-".to_vec();
    let inner_at_in_payload = payload.len();
    payload.extend_from_slice(&inner);
    payload.extend_from_slice(b"-after");
    let com = com_seg(&payload);
    // The index points into the COM: two passes to find the absolute offset.
    let build = |rel: u32| {
        let mut b = B::new();
        b.push(K::Jfif, &jfif());
        let mpf_at = b.len();
        let entries = [
            (0x2003_0000u32, 0u32, 0u32),
            (0, u32::try_from(inner.len()).unwrap(), rel),
        ];
        b.push(K::MpfIndex, &rs::mpf_segment(false, &entries, 32, true));
        let com_at = b.len();
        b.push(K::Com, &com);
        b.push(K::Scan, &scan());
        (b, mpf_at, com_at)
    };
    let (probe, mpf_at, com_at) = build(0);
    let header_start = mpf_at + 8;
    let image_at = com_at + 4 + inner_at_in_payload;
    let (mut b, _, _) = build(u32::try_from(image_at - header_start).unwrap());
    // The primary size is the whole file here.
    let _ = probe;
    let len = b.len();
    // Patch the primary size (entry 0) in place for tidiness.
    let size_at = header_start + MPF_DATA_AT + 4;
    let w = u32::try_from(len).unwrap().to_le_bytes();
    b.bytes[size_at..size_at + 4].copy_from_slice(&w);
    // The image is a part nested inside the COM: record it as an Image (the
    // COM part stays a Com).
    b.parts.push(Part {
        kind: K::Image,
        range: image_at..image_at + inner.len(),
    });
    let mut s = finish("com-over-embedded", b, ExifFacts::default());
    s.mpf = Some(MpfFacts {
        big_endian: false,
        header_start,
        fields: vec![
            (
                header_start + MPF_DATA_AT + 4,
                header_start + MPF_DATA_AT + 8,
            ),
            (
                header_start + MPF_DATA_AT + 16 + 4,
                header_start + MPF_DATA_AT + 16 + 8,
            ),
        ],
        entries: vec![
            (u32::try_from(len).unwrap(), 0),
            (
                u32::try_from(inner.len()).unwrap(),
                u32::try_from(image_at - header_start).unwrap(),
            ),
        ],
        primary_end: len,
    });
    s
}

/// An Adobe APP14 segment (kept under every policy) whose payload also holds an
/// `FF D8` range, which the index lists as a second embedded image: removing
/// that image would cut a kept segment.
pub fn overlapping_images() -> Shape {
    let mut payload = b"Adobe\0".to_vec();
    payload.extend_from_slice(&[0, 100, 0, 0, 0, 0, 1]);
    let ffd8_in_payload = payload.len();
    payload.extend_from_slice(&[0xFF, 0xD8, 1, 2, 3, 4, 5, 6, 7, 8]);
    let adobe_with_image = rs::seg(0xEE, &payload);
    let mut s = mpo_shape(
        "overlapping-images",
        false,
        vec![],
        vec![(K::Adobe, adobe_with_image), (K::Com, com_seg(C_COM1))],
        vec![
            (Vec::new(), rs::nikon_extra()),
            (Vec::new(), rs::iphone_extra()),
        ],
        Vec::new(),
        ExifFacts::default(),
    );
    let m = s.mpf.clone().unwrap();
    let adobe_at = s
        .parts
        .iter()
        .find(|p| p.kind == K::Adobe)
        .unwrap()
        .range
        .start;
    let inner = adobe_at + 4 + ffd8_in_payload;
    let (size2, off2) = (10u32, u32::try_from(inner - m.header_start).unwrap());
    let put = |bytes: &mut Vec<u8>, at: usize, v: u32| {
        bytes[at..at + 4].copy_from_slice(&v.to_le_bytes())
    };
    put(&mut s.bytes, m.fields[2].0, size2);
    put(&mut s.bytes, m.fields[2].1, off2);
    // The second secondary's bytes are now unindexed: a trailer piece.
    for p in &mut s.parts {
        if p.kind == K::Image && p.range.start == m.header_start + m.entries[2].1 as usize {
            p.kind = K::Trailer;
        }
    }
    s.parts.push(Part {
        kind: K::Image,
        range: inner..inner + 10,
    });
    s.mpf.as_mut().unwrap().entries[2] = (size2, off2);
    s
}

/// All fixtures, in a fixed order.
pub fn all() -> Vec<Shape> {
    vec![
        kitchen_sink(),
        no_exif(),
        exif_orientation_only(),
        exif_without_gps(),
        two_exif_blocks(),
        unreadable_exif(),
        gps_count_overflow(),
        comment_only(),
        mpo_late_xmp(false),
        mpo_late_xmp(true),
        mpo_trailer_gaps(),
        mpo_plain(),
        unindexed_mpf(),
        unindexed_mpf_last(),
        second_mpf(),
        mpf_malformed_entry(),
        mpf_entry_not_jpeg(),
        second_mpf_and_unlocated_entry(),
        com_over_embedded(),
        overlapping_images(),
        bad_length(),
        halted_walk(),
        missing_eoi(),
        missing_sos(),
        fill_bytes_between_segments(),
    ]
}

// ── the independent output model ───────────────────────────────────────────

/// The input with `removals` deleted and `patches` (input offset, bytes)
/// written. No `EditPlan`: a naive loop over a boolean mask.
pub fn expected_bytes(
    input: &[u8],
    removals: &[Range<usize>],
    patches: &[(usize, Vec<u8>)],
) -> Vec<u8> {
    let mut data = input.to_vec();
    for (at, bytes) in patches {
        data[*at..*at + bytes.len()].copy_from_slice(bytes);
    }
    let mut gone = vec![false; input.len()];
    for r in removals {
        for g in &mut gone[r.clone()] {
            *g = true;
        }
    }
    data.iter()
        .zip(&gone)
        .filter(|(_, g)| !**g)
        .map(|(b, _)| *b)
        .collect()
}

// ── the independent expectation model (typed from DEC-013, not from src/) ───

use exif_core::audit::Category;
use exif_core::policy::StripSpec;

/// The nine categories an Exif segment can hold, typed by hand.
pub const EXIF9: [Category; 9] = [
    Category::Gps,
    Category::DeviceId,
    Category::Authorship,
    Category::FreeText,
    Category::Timestamps,
    Category::CameraModel,
    Category::Thumbnail,
    Category::MakerNote,
    Category::Software,
];

pub fn cat_name(c: Category) -> &'static str {
    match c {
        Category::Gps => "gps",
        Category::DeviceId => "device_id",
        Category::Authorship => "authorship",
        Category::FreeText => "free_text",
        Category::Timestamps => "timestamps",
        Category::CameraModel => "camera_model",
        Category::Thumbnail => "thumbnail",
        Category::MakerNote => "maker_note",
        Category::Software => "software",
        _ => "other",
    }
}

pub fn spec_of(cats: &[Category]) -> StripSpec {
    cats.iter().fold(StripSpec::none(), |s, c| s.include(*c))
}

/// A removal the model expects: part kind, category, input range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rem {
    pub kind: K,
    pub category: Category,
    pub range: Range<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expect {
    Ok {
        removals: Vec<Rem>,
        /// `(input offset, new bytes)` of the patched fields.
        patches: Vec<(usize, Vec<u8>)>,
    },
    Unstrippable,
    NeedsEntryEdit(StripSpec),
    MpfNotIndexed(usize),
    Overlap(usize),
}

/// The category a part of `kind` is removed under, if any policy removes it.
pub fn part_category(kind: K) -> Option<Category> {
    match kind {
        K::Xmp | K::ExtXmp => Some(Category::Xmp),
        K::Icc => Some(Category::Icc),
        K::Iptc => Some(Category::Iptc),
        K::Jumbf => Some(Category::Jumbf),
        K::Com => Some(Category::FreeText),
        K::OtherApp | K::Trailer => Some(Category::OtherSegment),
        K::Image | K::MpfIndex => Some(Category::EmbeddedImage),
        _ => None,
    }
}

/// The category the audit gives a removed Exif block of `shape`: the first
/// EXIF9 finding of the block (IFD order), `Thumbnail` when it has none.
pub fn exif_block_category(shape: &Shape, block: usize) -> Category {
    match (shape.name, block) {
        ("two-exif-blocks", 1) | ("unreadable-exif", _) | ("exif-orientation-only", _) => {
            Category::Thumbnail
        }
        _ => Category::CameraModel,
    }
}

fn is_unindexed_mpf(shape: &Shape, p: &Part) -> bool {
    p.kind == K::OtherApp
        && shape
            .bytes
            .get(p.range.start + 4..)
            .is_some_and(|b| b.starts_with(b"MPF\0"))
}

pub fn model(shape: &Shape, spec: StripSpec) -> Expect {
    if shape.refused {
        return Expect::Unstrippable;
    }
    let exif9 = spec_of(&EXIF9);
    let mut targets: Vec<(usize, Rem)> = Vec::new();
    let mut exif_block = 0usize;
    for (i, p) in shape.parts.iter().enumerate() {
        if p.kind == K::Exif {
            let block = exif_block;
            exif_block += 1;
            if exif9.is_subset_of(spec) {
                targets.push((
                    i,
                    Rem {
                        kind: K::Exif,
                        category: exif_block_category(shape, block),
                        range: p.range.clone(),
                    },
                ));
            }
            continue;
        }
        if let Some(c) = part_category(p.kind) {
            if spec.contains(c) {
                targets.push((
                    i,
                    Rem {
                        kind: p.kind,
                        category: c,
                        range: p.range.clone(),
                    },
                ));
            }
        }
    }
    // Exif: all, none, or the entry edit.
    if !exif9.is_subset_of(spec) && spec.intersects(exif9) && shape.exif.blocks > 0 {
        let in_spec = |name: &&str| {
            EXIF9
                .iter()
                .any(|c| cat_name(*c) == *name && spec.contains(*c))
        };
        if !shape.exif.readable || shape.exif.cats.iter().any(in_spec) {
            let cats: Vec<Category> = EXIF9
                .iter()
                .copied()
                .filter(|c| spec.contains(*c))
                .collect();
            return Expect::NeedsEntryEdit(spec_of(&cats));
        }
    }
    targets.sort_by_key(|(_, r)| (r.range.start, r.range.end));
    let removed_parts: Vec<usize> = targets.iter().map(|(i, _)| *i).collect();
    let index_part = shape.parts.iter().position(|p| p.kind == K::MpfIndex);
    let index_removed = index_part.is_some_and(|i| removed_parts.contains(&i));
    let n = shape.bytes.len();
    let mut removed = vec![false; n];
    for (_, r) in &targets {
        for b in &mut removed[r.range.clone()] {
            *b = true;
        }
    }
    // An index entry that is no image, pointing inside the input, with a removal
    // between the header and the byte it points at.
    if let (Some(m), Some(ip), false) = (
        &shape.mpf,
        index_part,
        spec.contains(Category::EmbeddedImage),
    ) {
        for &(size, offset) in m.entries.iter().skip(1) {
            let start = m.header_start + offset as usize;
            let located = shape.parts.iter().any(|p| {
                p.kind == K::Image && p.range.start == start && p.range.len() == size as usize
            });
            if !located && start < n && (m.header_start..=start).any(|i| removed[i]) {
                return Expect::MpfNotIndexed(shape.parts[ip].range.start);
            }
        }
    }
    // An MPF segment the index does not describe.
    for (i, p) in shape.parts.iter().enumerate() {
        let second = index_part.is_some_and(|ip| ip < i)
            && shape
                .bytes
                .get(p.range.start + 4..)
                .is_some_and(|b| b.starts_with(b"MPF\0"))
            && p.kind == K::OtherApp;
        if (is_unindexed_mpf(shape, p) || second) && !removed_parts.contains(&i) {
            let later = targets.iter().any(|(_, r)| r.range.start >= p.range.end);
            let after_removed_index = index_removed && index_part.is_some_and(|ip| ip < i);
            if later || after_removed_index {
                return Expect::MpfNotIndexed(p.range.start);
            }
        }
    }
    // Overlap: a removed byte that is also a kept byte.
    let mut kept = vec![false; n];
    for (i, p) in shape.parts.iter().enumerate() {
        let is_kept = !removed_parts.contains(&i) && p.kind != K::Stray;
        if is_kept {
            for b in &mut kept[p.range.clone()] {
                *b = true;
            }
        }
    }
    if let Some(at) = (0..n).find(|&i| removed[i] && kept[i]) {
        return Expect::Overlap(at);
    }
    // Patches.
    let mut patches = Vec::new();
    if let Some(m) = &shape.mpf {
        if !spec.contains(Category::EmbeddedImage) && !targets.is_empty() {
            let removed_in = |a: usize, b: usize| removed[a..b].iter().filter(|x| **x).count();
            let enc = |v: u32| {
                if m.big_endian {
                    v.to_be_bytes().to_vec()
                } else {
                    v.to_le_bytes().to_vec()
                }
            };
            for (i, &(size, offset)) in m.entries.iter().enumerate() {
                let (size_field, offset_field) = m.fields[i];
                if i == 0 {
                    let new = size - removed_in(0, m.primary_end) as u32;
                    if new != size {
                        patches.push((size_field, enc(new)));
                    }
                } else {
                    let start = m.header_start + offset as usize;
                    let located = shape.parts.iter().any(|p| {
                        p.kind == K::Image
                            && p.range.start == start
                            && p.range.len() == size as usize
                    });
                    if located {
                        let new = offset - removed_in(m.header_start, start) as u32;
                        if new != offset {
                            patches.push((offset_field, enc(new)));
                        }
                    }
                }
            }
        }
    }
    Expect::Ok {
        removals: targets.into_iter().map(|(_, r)| r).collect(),
        patches,
    }
}
