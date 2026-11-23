//! The measured WebP carrier table (SPEC-038, DEC-022 clause 4), as data.
//!
//! Measured on ExifTool 13.55 (`-a -u -G1 -H -s`, one chunk per file in a
//! minimal WebP, the chunk before the image data, after it, beyond the declared
//! RIFF size and without a `VP8X`): every row below printed at least one tag
//! from its chunk, in every position but after an empty null chunk (ExifTool
//! stops there; this library reads on, so it strips more, never less). Every
//! row's code is matched case-sensitively: all 16 ASCII-case spellings of each
//! were measured and none other than the exact code (and `ID3 `/`id3 `, two
//! codes ExifTool knows) decoded. A chunk of any other type prints only an
//! `Unknown_<code>` binary dump under `-u` and is not metadata.
//!
//! No import from `src/`: the expectations here are typed from the
//! measurement, never derived from the library.

#![allow(dead_code)]

use super::webp_shapes::chunk;

/// What a row's payload is.
#[derive(Clone, Copy, Debug)]
pub enum Payload {
    /// Printable ASCII.
    Text,
    /// An XMP packet with a creator and a city.
    Xmp,
    /// 40 zero bytes.
    Zeros,
    /// A `LIST` chunk's data: `INFO`, then an `IART` sub-chunk.
    ListInfo,
    /// A `LIST` chunk's data: `adtl`, then a `labl` sub-chunk.
    ListAdtl,
    /// A minimal JUMBF box (a C2PA manifest with an Exif assertion).
    Jumbf,
    /// An ID3v2.3 tag with an artist.
    Id3,
}

/// One measured row.
#[derive(Clone, Copy, Debug)]
pub struct Row {
    pub code: [u8; 4],
    pub payload: Payload,
    /// The audit region kind's report name.
    pub region: &'static str,
    /// The audit category's report name.
    pub category: &'static str,
    /// A tag ExifTool 13.55 printed from the chunk (group `RIFF` unless the
    /// group is part of the name).
    pub prints: &'static str,
}

const CARRIER: &str = "carrier";
const OTHER: &str = "other_chunk";
const OTHER_SEGMENT: &str = "other_segment";

/// The measured rows. `LIST` appears once here (any list type is a carrier,
/// DEC-022 clause 4).
pub const ROWS: &[Row] = &[
    Row {
        code: *b"LIST",
        payload: Payload::ListInfo,
        region: CARRIER,
        category: OTHER_SEGMENT,
        prints: "RIFF:Artist",
    },
    Row {
        code: *b"IDIT",
        payload: Payload::Text,
        region: CARRIER,
        category: OTHER_SEGMENT,
        prints: "RIFF:DateTimeOriginal",
    },
    Row {
        code: *b"JUNK",
        payload: Payload::Text,
        region: CARRIER,
        category: OTHER_SEGMENT,
        prints: "RIFF:TextJunk",
    },
    Row {
        code: *b"JUNQ",
        payload: Payload::Zeros,
        region: CARRIER,
        category: OTHER_SEGMENT,
        prints: "RIFF:OldXMP",
    },
    Row {
        code: *b"SEAL",
        payload: Payload::Xmp,
        region: CARRIER,
        category: OTHER_SEGMENT,
        prints: "XML-dc:Creator",
    },
    Row {
        code: *b"_PMX",
        payload: Payload::Xmp,
        region: CARRIER,
        category: OTHER_SEGMENT,
        prints: "XMP-dc:Creator",
    },
    Row {
        code: *b"aXML",
        payload: Payload::Xmp,
        region: CARRIER,
        category: OTHER_SEGMENT,
        prints: "XML-dc:Creator",
    },
    Row {
        code: *b"iXML",
        payload: Payload::Xmp,
        region: CARRIER,
        category: OTHER_SEGMENT,
        prints: "XML-dc:Creator",
    },
    Row {
        code: *b"afsp",
        payload: Payload::Text,
        region: CARRIER,
        category: OTHER_SEGMENT,
        prints: "RIFF:Afsp",
    },
    Row {
        code: *b"bext",
        payload: Payload::Text,
        region: CARRIER,
        category: OTHER_SEGMENT,
        prints: "RIFF:Description",
    },
    Row {
        code: *b"guan",
        payload: Payload::Text,
        region: CARRIER,
        category: OTHER_SEGMENT,
        prints: "RIFF:Guano",
    },
    Row {
        code: *b"list",
        payload: Payload::Text,
        region: CARRIER,
        category: OTHER_SEGMENT,
        prints: "RIFF:ListType",
    },
    Row {
        code: *b"labl",
        payload: Payload::Text,
        region: CARRIER,
        category: OTHER_SEGMENT,
        prints: "RIFF:CuePointLabel",
    },
    Row {
        code: *b"note",
        payload: Payload::Text,
        region: CARRIER,
        category: OTHER_SEGMENT,
        prints: "RIFF:CuePointNote",
    },
    Row {
        code: *b"ltxt",
        payload: Payload::Zeros,
        region: CARRIER,
        category: OTHER_SEGMENT,
        prints: "RIFF:LabeledText",
    },
    Row {
        code: *b"olym",
        payload: Payload::Zeros,
        region: CARRIER,
        category: OTHER_SEGMENT,
        prints: "Olympus:FileNumber",
    },
    Row {
        code: *b"ID3 ",
        payload: Payload::Id3,
        region: CARRIER,
        category: OTHER_SEGMENT,
        prints: "ID3v2_3:Artist",
    },
    Row {
        code: *b"id3 ",
        payload: Payload::Id3,
        region: CARRIER,
        category: OTHER_SEGMENT,
        prints: "ID3v2_3:Artist",
    },
    Row {
        code: *b"gps0",
        payload: Payload::Zeros,
        region: CARRIER,
        category: OTHER_SEGMENT,
        prints: "RIFF:GPSLatitude",
    },
    Row {
        code: *b"SGLT",
        payload: Payload::Zeros,
        region: CARRIER,
        category: OTHER_SEGMENT,
        prints: "RIFF:Accelerometer",
    },
    Row {
        code: *b"SLLT",
        payload: Payload::Zeros,
        region: CARRIER,
        category: OTHER_SEGMENT,
        prints: "RIFF:GPSLatitude",
    },
    Row {
        code: *b"C2PA",
        payload: Payload::Jumbf,
        region: "jumbf",
        category: "jumbf",
        prints: "JSON:Exif_GPSLatitude",
    },
    Row {
        code: *b"CSET",
        payload: Payload::Zeros,
        region: OTHER,
        category: OTHER_SEGMENT,
        prints: "RIFF:CodePage",
    },
    Row {
        code: *b"acid",
        payload: Payload::Zeros,
        region: OTHER,
        category: OTHER_SEGMENT,
        prints: "RIFF:AcidizerFlags",
    },
    Row {
        code: *b"ds64",
        payload: Payload::Zeros,
        region: OTHER,
        category: OTHER_SEGMENT,
        prints: "RIFF:RIFFSize64",
    },
    Row {
        code: *b"fact",
        payload: Payload::Zeros,
        region: OTHER,
        category: OTHER_SEGMENT,
        prints: "RIFF:NumberOfSamples",
    },
    Row {
        code: *b"fmt ",
        payload: Payload::Zeros,
        region: OTHER,
        category: OTHER_SEGMENT,
        prints: "RIFF:Encoding",
    },
    Row {
        code: *b"inst",
        payload: Payload::Zeros,
        region: OTHER,
        category: OTHER_SEGMENT,
        prints: "RIFF:UnshiftedNote",
    },
    Row {
        code: *b"smpl",
        payload: Payload::Zeros,
        region: OTHER,
        category: OTHER_SEGMENT,
        prints: "RIFF:Manufacturer",
    },
    Row {
        code: *b"cue ",
        payload: Payload::Zeros,
        region: OTHER,
        category: OTHER_SEGMENT,
        prints: "RIFF:CuePoints",
    },
    Row {
        code: *b"plst",
        payload: Payload::Zeros,
        region: OTHER,
        category: OTHER_SEGMENT,
        prints: "RIFF:Playlist",
    },
    Row {
        code: *b"gsen",
        payload: Payload::Zeros,
        region: OTHER,
        category: OTHER_SEGMENT,
        prints: "RIFF:Accelerometer",
    },
];

/// Chunk types that printed only `Unknown_<code>` (or nothing): never
/// metadata, never located, never removed.
pub const NOT_METADATA: &[[u8; 4]] = &[
    *b"PAD ", *b"strd", *b"strn", *b"PrmL", *b"Cr8r", *b"wavl", *b"slnt", *b"DISP", *b"avih",
    *b"strh", *b"strf", *b"vedt", *b"ISFT", *b"IART", *b"INAM", *b"ICMT", *b"ICOP", *b"TEXT",
    *b"COMM", *b"umid", *b"regn", *b"link", *b"axml", *b"junk", *b"pad ", *b"Junk", *b"exif",
    *b"xmp ", *b"iccp", *b"Exif",
];

/// `LIST` list types that printed tags with the matching sub-chunk (every
/// `LIST` is a carrier, so the type does not matter to the strip).
pub const LIST_TYPES: &[&[u8; 4]] = &[b"INFO", b"INF0", b"adtl", b"exif", b"hdrl"];

/// A `JUNK` payload ExifTool printed as `TextJunk`: printable ASCII (0x20 to
/// 0x7E) then zero or more NULs.
pub const JUNK_TEXT: &[&[u8]] = &[
    b"Jane Doe Springfield",
    b"A",
    b" ",
    b"Jane Doe\0\0\0\0\0\0\0\0\0\0",
];

/// A `JUNK` payload ExifTool printed nothing for: a control byte, a high byte,
/// a NUL before or inside the text, newline, tab, no bytes, zeros.
pub const JUNK_NOT_TEXT: &[&[u8]] = &[
    b"",
    b"\0\0\0\0\0\0\0\0",
    b"\0\0\0\0Jane Doe",
    b"Jane\0Doe",
    b"Jane\nDoe",
    b"Jane\tDoe",
    b"Jane\rDoe",
    b"Jane\x01Doe",
    b"Jane \xe9 Doe",
    b"Jane\x7fDoe",
    b"Jane Doe\xff",
];

/// All distinct ASCII-case spellings of `code`.
pub fn spellings(code: [u8; 4]) -> Vec<[u8; 4]> {
    let mut out: Vec<[u8; 4]> = Vec::new();
    for mask in 0u8..16 {
        let mut b = code;
        for (i, byte) in b.iter_mut().enumerate() {
            *byte = if mask >> i & 1 == 1 {
                byte.to_ascii_uppercase()
            } else {
                byte.to_ascii_lowercase()
            };
        }
        if !out.contains(&b) {
            out.push(b);
        }
    }
    out
}

fn be32(n: usize) -> [u8; 4] {
    u32::try_from(n).unwrap_or(0).to_be_bytes()
}

fn jbox(ty: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut v = be32(body.len() + 8).to_vec();
    v.extend_from_slice(ty);
    v.extend_from_slice(body);
    v
}

/// The bytes of one payload.
pub fn payload(p: Payload) -> Vec<u8> {
    match p {
        Payload::Text => b"Jane Doe Springfield Copyright Acme 2024".to_vec(),
        Payload::Xmp => b"<?xpacket begin=\"\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?><x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"><rdf:Description rdf:about=\"\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:photoshop=\"http://ns.adobe.com/photoshop/1.0/\"><dc:creator><rdf:Seq><rdf:li>Jane Creator</rdf:li></rdf:Seq></dc:creator><photoshop:City>Springfield</photoshop:City></rdf:Description></rdf:RDF></x:xmpmeta><?xpacket end=\"w\"?>".to_vec(),
        Payload::Zeros => vec![0; 40],
        Payload::ListInfo => {
            let mut v = b"INFO".to_vec();
            v.extend_from_slice(&chunk(b"IART", b"Jane Doe\0"));
            v
        }
        Payload::ListAdtl => {
            let mut v = b"adtl".to_vec();
            let mut label = vec![0; 4];
            label.extend_from_slice(b"Jane\0");
            v.extend_from_slice(&chunk(b"labl", &label));
            v
        }
        Payload::Jumbf => {
            let desc = |uuid: &[u8; 16], label: &[u8]| {
                let mut body = uuid.to_vec();
                body.push(3);
                body.extend_from_slice(label);
                body.push(0);
                jbox(b"jumd", &body)
            };
            let json_uuid = *b"json\0\x11\0\x10\x80\0\0\xAA\0\x38\x9B\x71";
            let c2pa_uuid = *b"c2pa\0\x11\0\x10\x80\0\0\xAA\0\x38\x9B\x71";
            let json = br#"{"exif:GPSLatitude":"51,30.0N","dc:creator":["Jane Jumbf Secret"]}"#;
            let assertion = [desc(&json_uuid, b"stds.exif"), jbox(b"json", json)].concat();
            let manifest = [desc(&c2pa_uuid, b"c2pa"), jbox(b"jumb", &assertion)].concat();
            jbox(b"jumb", &manifest)
        }
        Payload::Id3 => {
            // The frame size counts the encoding byte and the 11 text bytes.
            let frame = b"TPE1\0\0\0\x0c\0\0\0Jane Artist";
            let n = frame.len();
            let mut v = b"ID3\x03\0\0".to_vec();
            v.extend_from_slice(&[
                ((n >> 21) & 127) as u8,
                ((n >> 14) & 127) as u8,
                ((n >> 7) & 127) as u8,
                (n & 127) as u8,
            ]);
            v.extend_from_slice(frame);
            v
        }
    }
}

/// The chunk (frame, data, pad) of a row, spelt `code`.
pub fn row_chunk(code: &[u8; 4], p: Payload) -> Vec<u8> {
    chunk(code, &payload(p))
}
