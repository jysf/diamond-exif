//! The I3 fixture set (SPEC-039, DEC-023): what the records of
//! `tests/oracle-fixtures/exiftool-strip-{png,webp}/` are made of.
//!
//! In order, and the names are stable (a new shape goes at the end, a record
//! is keyed by name):
//!
//! 1. the 61 `oracle_png` fixtures,
//! 2. the 113 `oracle_webp` fixtures,
//! 3. the PNG carrier shapes (moved here from `strip_png_live.rs`, which
//!    imports them),
//! 4. one PNG per ExifTool `PNG::TextualData` keyword (`tEXt`, value `Sweep`
//!    plus the keyword),
//! 5. one PNG with a `caBX` JUMBF manifest,
//! 6. one WebP per `webp_carriers` row (the row's own payload, one chunk before
//!    the image data).
//!
//! No import from `src/`. The expectations are typed elsewhere (the judge, the
//! tables), never derived from the library.

#![allow(dead_code)]

use crate::oracle_png;
use crate::oracle_tiff as t;
use crate::oracle_webp;
use crate::webp_carriers;
use crate::webp_shapes;

/// One input of the I3 records.
#[derive(Clone, Debug)]
pub struct Fixture {
    pub name: String,
    /// `png` or `webp`: the container, the record directory suffix and the
    /// extension of the temporary files.
    pub ext: &'static str,
    pub bytes: Vec<u8>,
}

/// How many fixtures the two oracle lists hold (the design's 174).
pub const ORACLE_COUNT: usize = 174;

/// The keywords of ExifTool 13.55's `PNG::TextualData` table (`-listx`, read
/// 2026-11-24), in the table's order. The keyword sweep checks this list
/// against ExifTool's own.
pub const TEXTUAL_DATA_KEYWORDS: [&str; 22] = [
    "Artist",
    "Author",
    "Collection",
    "Comment",
    "Copyright",
    "Creation Time",
    "Description",
    "Disclaimer",
    "Document",
    "Label",
    "Make",
    "Model",
    "Software",
    "Source",
    "TimeStamp",
    "Title",
    "URL",
    "Warning",
    "aesthetic_score",
    "create-date",
    "modify-date",
    "parameters",
];

/// A TIFF with a GPS position, a body serial, an owner name, an artist and a
/// copyright: every tag family the pinned lists name, in the three IFDs.
pub fn rich_tiff() -> Vec<u8> {
    let big = false;
    let ifd0 = t::ifd(vec![
        t::ascii(0x010F, b"MakeCorp\0"),
        t::ascii(0x013B, b"Artist Zed\0"),
        t::ascii(0x8298, b"Copy Zed\0"),
        t::ptr(0x8769, 1),
        t::ptr(0x8825, 2),
    ]);
    let exif = t::ifd(vec![
        t::ascii(0xA430, b"Owner Zed\0"),
        t::ascii(0xA431, b"SN123456\0"),
    ]);
    let gps = t::ifd(vec![
        t::ascii(0x0001, b"N\0"),
        t::rational(big, 0x0002, &[(51, 1), (30, 1), (0, 1)]),
        t::ascii(0x0003, b"E\0"),
        t::rational(big, 0x0004, &[(0, 1), (7, 1), (0, 1)]),
    ]);
    t::assemble(big, &[ifd0, exif, gps])
}

/// `\0`, the TIFF's length (big-endian) and a zlib stream of it: the `zxIf`
/// payload.
pub fn zlib_wrapped(tiff: &[u8]) -> Vec<u8> {
    let mut d = vec![0];
    d.extend_from_slice(&(tiff.len() as u32).to_be_bytes());
    d.extend_from_slice(&oracle_png::zlib_stored(tiff));
    d
}

/// An XMP packet with a GPS position and a creator.
pub fn xmp_with_gps() -> Vec<u8> {
    br#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:exif="http://ns.adobe.com/exif/1.0/" exif:GPSLatitude="51,30.0N" exif:GPSLongitude="0,7.0E"><dc:creator><rdf:Seq><rdf:li>Zed</rdf:li></rdf:Seq></dc:creator></rdf:Description></rdf:RDF></x:xmpmeta>"#.to_vec()
}

/// A minimal PNG around the given chunks (before `IDAT`).
pub fn with(chunks: &[Vec<u8>]) -> Vec<u8> {
    oracle_png::png(chunks, &[], &[])
}

/// The opaque carriers ExifTool decodes: a file that holds only the carrier.
/// The names are those of the live precursor, with a `png-` prefix.
pub fn carrier_shapes() -> Vec<(String, Vec<u8>)> {
    let tiff = rich_tiff();
    let xmp = xmp_with_gps();
    let xml = b"<?xml version=\"1.0\"?><a><b>Zed</b></a>".to_vec();
    let raw = oracle_png::raw_profile_body("exif", &tiff);
    vec![
        (
            "png-carrier-zxIf".to_string(),
            with(&[oracle_png::chunk(b"zxIf", &zlib_wrapped(&tiff))]),
        ),
        (
            "png-carrier-tXMP".to_string(),
            with(&[oracle_png::chunk(b"tXMP", &xmp)]),
        ),
        (
            "png-carrier-meTa".to_string(),
            with(&[oracle_png::chunk(b"meTa", &xml)]),
        ),
        (
            "png-carrier-seAl".to_string(),
            with(&[oracle_png::chunk(b"seAl", &xml)]),
        ),
        (
            "png-carrier-raw-profile-text".to_string(),
            with(&[oracle_png::text(b"Raw profile type exif", &raw)]),
        ),
        (
            "png-carrier-after-iend".to_string(),
            oracle_png::png(&[], &[], &oracle_png::chunk(b"zxIf", &tiff)),
        ),
    ]
}

/// A keyword's file-name part: lower case, anything but a letter or digit is
/// `-`.
fn slug(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

/// One PNG per `TextualData` keyword: a `tEXt` chunk `keyword`, `\0`, `Sweep`
/// and the keyword.
pub fn keyword_shapes() -> Vec<(String, Vec<u8>)> {
    TEXTUAL_DATA_KEYWORDS
        .iter()
        .enumerate()
        .map(|(i, k)| {
            let value = format!("Sweep{k}");
            (
                format!("png-kw-{i:02}-{}", slug(k)),
                with(&[oracle_png::text(k.as_bytes(), value.as_bytes())]),
            )
        })
        .collect()
}

/// A PNG with a valid JUMBF manifest in a `caBX` chunk.
pub fn cabx_shape() -> (String, Vec<u8>) {
    let manifest = webp_carriers::payload(webp_carriers::Payload::Jumbf);
    (
        "png-cabx-jumbf".to_string(),
        with(&[oracle_png::chunk(b"caBX", &manifest)]),
    )
}

/// One WebP per `webp_carriers` row: `VP8X` (no flags), the row's chunk, `VP8L`.
pub fn row_shapes() -> Vec<(String, Vec<u8>)> {
    webp_carriers::ROWS
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let code: String = row
                .code
                .iter()
                .map(|&b| if b == b' ' { '_' } else { b as char })
                .collect();
            let chunk = webp_carriers::row_chunk(&row.code, row.payload);
            let bytes =
                webp_shapes::riff(&[webp_shapes::vp8x(0, 8, 8), chunk, webp_shapes::vp8l()]);
            (
                format!("webp-row-{i:02}-{}", code.to_ascii_lowercase()),
                bytes,
            )
        })
        .collect()
}

/// The whole I3 set, in the order of the module comment.
pub fn fixtures() -> Vec<Fixture> {
    let mut out = Vec::new();
    for (name, bytes) in oracle_png::fixtures() {
        out.push(Fixture {
            name,
            ext: "png",
            bytes,
        });
    }
    for (name, bytes) in oracle_webp::fixtures() {
        out.push(Fixture {
            name,
            ext: "webp",
            bytes,
        });
    }
    let png_extra = carrier_shapes()
        .into_iter()
        .chain(keyword_shapes())
        .chain(std::iter::once(cabx_shape()));
    for (name, bytes) in png_extra {
        out.push(Fixture {
            name,
            ext: "png",
            bytes,
        });
    }
    for (name, bytes) in row_shapes() {
        out.push(Fixture {
            name,
            ext: "webp",
            bytes,
        });
    }
    out
}
