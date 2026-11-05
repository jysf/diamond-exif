//! Hand-built TIFF/EXIF fixtures for the ExifTool parity oracle (SPEC-005).
//!
//! Tier A: every byte is written here from the TIFF 6.0 section 2 layout
//! (header, IFD, 12-byte entries, out-of-line payloads) and the Exif 2.32
//! pointer tags. No camera file is committed, so there is no licence question
//! for these fixtures. The builder is deliberately independent of the
//! `#[cfg(test)]` builder inside `src/` and of `examples/fuzz_seeds.rs`: an
//! oracle fixture must not share its encoding code with the library it checks.
//!
//! Deterministic: `fixtures()` is a pure function of this file. It is the
//! single named list used by the recorder, the live test and the replay test.
//! std only.

#![allow(dead_code)]

const BYTE: u16 = 1;
pub const ASCII: u16 = 2;
pub const SHORT: u16 = 3;
pub const LONG: u16 = 4;
pub const RATIONAL: u16 = 5;
pub const SRATIONAL: u16 = 10;
pub const SSHORT: u16 = 8;
pub const SLONG: u16 = 9;
pub const FLOAT: u16 = 11;
pub const DOUBLE: u16 = 12;

// Tag numbers (TIFF 6.0 section 8, Exif 2.32 sections 4.6.3 to 4.6.6).
pub const T_IMAGE_WIDTH: u16 = 256;
pub const T_IMAGE_LENGTH: u16 = 257;
pub const T_MAKE: u16 = 271;
pub const T_MODEL: u16 = 272;
pub const T_ORIENTATION: u16 = 274;
pub const T_X_RESOLUTION: u16 = 282;
pub const T_SOFTWARE: u16 = 305;
pub const T_DATE_TIME: u16 = 306;
pub const T_ARTIST: u16 = 315;
pub const T_COPYRIGHT: u16 = 33432;
pub const T_EXIF_IFD: u16 = 0x8769;
pub const T_GPS_IFD: u16 = 0x8825;
pub const T_F_NUMBER: u16 = 0x829D;
pub const T_ISO: u16 = 0x8827;
pub const T_DATE_TIME_ORIGINAL: u16 = 0x9003;
pub const T_EXPOSURE_BIAS: u16 = 0x9204;
// Tags read through the signed and float decoders (SPEC-008). The type in the
// file is the point: ExifTool 13.55 was measured to print the value in the
// DECLARED type for these (see the SPEC-008 handback, which lists the 17
// candidate tags tried; 3 of them did not follow the declared type).
pub const T_EXPOSURE_TIME: u16 = 0x829A;
pub const T_EXPOSURE_PROGRAM: u16 = 0x8822;
pub const T_FOCAL_LENGTH: u16 = 0x920A;
pub const T_EXIF_IMAGE_WIDTH: u16 = 0xA002;
pub const T_GPS_LAT_REF: u16 = 1;
pub const T_GPS_LAT: u16 = 2;
pub const T_GPS_LON_REF: u16 = 3;
pub const T_GPS_LON: u16 = 4;

/// What fills an entry's 4-byte value field.
#[derive(Clone)]
enum Val {
    /// Encoded payload, at most 4 bytes: stored inline, zero padded.
    Inline(Vec<u8>),
    /// Encoded payload, more than 4 bytes: appended after the IFD.
    Ext(Vec<u8>),
    /// A literal offset that is deliberately not backed by data.
    Raw(u32),
    /// The offset of another IFD in the same file, by index.
    Ifd(usize),
}

/// One IFD entry, before layout.
#[derive(Clone)]
pub struct Ent {
    tag: u16,
    typ: u16,
    count: u32,
    val: Val,
}

/// An IFD under construction.
#[derive(Clone)]
pub struct IfdSpec {
    ents: Vec<Ent>,
    /// Index of the IFD the next-pointer should lead to.
    next: Option<usize>,
}

fn put(big: bool, v: u32, width: usize) -> Vec<u8> {
    let all = if big {
        v.to_be_bytes()
    } else {
        v.to_le_bytes()
    };
    if big {
        all[4 - width..].to_vec()
    } else {
        all[..width].to_vec()
    }
}

fn pack(typ: u16, count: u32, bytes: Vec<u8>) -> (u16, u32, Val) {
    if bytes.len() <= 4 {
        (typ, count, Val::Inline(bytes))
    } else {
        (typ, count, Val::Ext(bytes))
    }
}

fn ent(tag: u16, packed: (u16, u32, Val)) -> Ent {
    Ent {
        tag,
        typ: packed.0,
        count: packed.1,
        val: packed.2,
    }
}

/// ASCII entry; `bytes` is stored verbatim (the caller adds any NUL).
pub fn ascii(tag: u16, bytes: &[u8]) -> Ent {
    ent(tag, pack(ASCII, bytes.len() as u32, bytes.to_vec()))
}

/// `SHORT` entry (tag values are encoded for the byte order later).
pub fn short(big: bool, tag: u16, v: u16) -> Ent {
    ent(tag, pack(SHORT, 1, put(big, u32::from(v), 2)))
}

/// `LONG` entry.
pub fn long(big: bool, tag: u16, v: u32) -> Ent {
    ent(tag, pack(LONG, 1, put(big, v, 4)))
}

/// `SSHORT` entry (one signed 16-bit value).
pub fn sshort(big: bool, tag: u16, v: i16) -> Ent {
    ent(tag, pack(SSHORT, 1, put(big, u32::from(v as u16), 2)))
}

/// `SLONG` entry (one signed 32-bit value).
pub fn slong(big: bool, tag: u16, v: i32) -> Ent {
    ent(tag, pack(SLONG, 1, put(big, v as u32, 4)))
}

/// `FLOAT` entry (one IEEE 754 binary32).
pub fn float(big: bool, tag: u16, v: f32) -> Ent {
    ent(tag, pack(FLOAT, 1, put(big, v.to_bits(), 4)))
}

/// `DOUBLE` entry (one IEEE 754 binary64, stored out of line).
pub fn double(big: bool, tag: u16, v: f64) -> Ent {
    let b = if big {
        v.to_be_bytes().to_vec()
    } else {
        v.to_le_bytes().to_vec()
    };
    ent(tag, pack(DOUBLE, 1, b))
}

/// `RATIONAL` entry, one value per `(numerator, denominator)`.
pub fn rational(big: bool, tag: u16, vs: &[(u32, u32)]) -> Ent {
    let mut b = Vec::new();
    for (n, d) in vs {
        b.extend(put(big, *n, 4));
        b.extend(put(big, *d, 4));
    }
    ent(tag, pack(RATIONAL, vs.len() as u32, b))
}

/// `SRATIONAL` entry, one value per `(numerator, denominator)`.
pub fn srational(big: bool, tag: u16, vs: &[(i32, i32)]) -> Ent {
    let mut b = Vec::new();
    for (n, d) in vs {
        b.extend(put(big, *n as u32, 4));
        b.extend(put(big, *d as u32, 4));
    }
    ent(tag, pack(SRATIONAL, vs.len() as u32, b))
}

/// A pointer entry (`LONG`) to the IFD at index `idx` of the spec list.
pub fn ptr(tag: u16, idx: usize) -> Ent {
    Ent {
        tag,
        typ: LONG,
        count: 1,
        val: Val::Ifd(idx),
    }
}

/// A pointer entry with a literal, unbacked offset.
pub fn ptr_raw(tag: u16, offset: u32) -> Ent {
    Ent {
        tag,
        typ: LONG,
        count: 1,
        val: Val::Raw(offset),
    }
}

pub fn ifd(ents: Vec<Ent>) -> IfdSpec {
    IfdSpec { ents, next: None }
}

pub fn ifd_next(ents: Vec<Ent>, next: usize) -> IfdSpec {
    IfdSpec {
        ents,
        next: Some(next),
    }
}

/// Lay the IFDs out one after another (table, then its out-of-line payloads,
/// word aligned) behind an 8-byte header, IFD 0 first.
pub fn assemble(big: bool, ifds: &[IfdSpec]) -> Vec<u8> {
    // Pass 1: each IFD's offset.
    let mut offsets = Vec::new();
    let mut at = 8usize;
    for spec in ifds {
        offsets.push(at as u32);
        at += 2 + 12 * spec.ents.len() + 4;
        for e in &spec.ents {
            if let Val::Ext(b) = &e.val {
                at += b.len() + (b.len() & 1);
            }
        }
    }
    // Pass 2: bytes.
    let mut out = Vec::new();
    out.extend_from_slice(if big { b"MM" } else { b"II" });
    out.extend(put(big, 42, 2));
    out.extend(put(big, offsets.first().copied().unwrap_or(0), 4));
    for spec in ifds {
        let mut payloads: Vec<u8> = Vec::new();
        let table_len = 2 + 12 * spec.ents.len() + 4;
        let here = out.len();
        let payload_base = here + table_len;
        out.extend(put(big, spec.ents.len() as u32, 2));
        for e in &spec.ents {
            out.extend(put(big, u32::from(e.tag), 2));
            out.extend(put(big, u32::from(e.typ), 2));
            out.extend(put(big, e.count, 4));
            match &e.val {
                Val::Inline(b) => {
                    let mut field = b.clone();
                    field.resize(4, 0);
                    out.extend(field);
                }
                Val::Ext(b) => {
                    out.extend(put(big, (payload_base + payloads.len()) as u32, 4));
                    payloads.extend(b);
                    if b.len() & 1 == 1 {
                        payloads.push(0);
                    }
                }
                Val::Raw(o) => out.extend(put(big, *o, 4)),
                Val::Ifd(i) => out.extend(put(big, offsets.get(*i).copied().unwrap_or(0), 4)),
            }
        }
        let next = spec.next.and_then(|i| offsets.get(i).copied()).unwrap_or(0);
        out.extend(put(big, next, 4));
        out.extend(payloads);
    }
    out
}

/// The `exif-basic` IFD0 (tags ascending, as TIFF 6.0 requires), pointing at
/// IFD 1 (the Exif IFD).
fn basic_ifd0(big: bool) -> Vec<Ent> {
    vec![
        long(big, T_IMAGE_WIDTH, 4000),
        long(big, T_IMAGE_LENGTH, 3000),
        ascii(T_MAKE, b"DiamondCam\0"),
        ascii(T_MODEL, b"Model One\0"),
        short(big, T_ORIENTATION, 6),
        rational(big, T_X_RESOLUTION, &[(72, 1)]),
        ascii(T_SOFTWARE, b"oracle-fixtures 1.0\0"),
        ascii(T_DATE_TIME, b"2024:05:06 07:08:09\0"),
        ascii(T_ARTIST, b"A. Photographer\0"),
        ascii(T_COPYRIGHT, b"(c) nobody\0"),
        ptr(T_EXIF_IFD, 1),
    ]
}

fn basic_exif(big: bool) -> Vec<Ent> {
    vec![
        rational(big, T_F_NUMBER, &[(28, 10)]),
        short(big, T_ISO, 400),
        ascii(T_DATE_TIME_ORIGINAL, b"2024:05:06 07:08:08\0"),
        srational(big, T_EXPOSURE_BIAS, &[(-1, 3)]),
    ]
}

fn exif_basic(big: bool) -> Vec<u8> {
    assemble(big, &[ifd(basic_ifd0(big)), ifd(basic_exif(big))])
}

/// A GPS fixture: IFD0 (Make + GPS pointer) and a GPS IFD holding the two refs
/// and the two coordinates (degrees, minutes, seconds as three RATIONALs).
fn exif_gps(big: bool, lat_ref: &[u8], lon_ref: &[u8]) -> Vec<u8> {
    let ifd0 = ifd(vec![ascii(T_MAKE, b"GpsCam\0"), ptr(T_GPS_IFD, 1)]);
    let gps = ifd(vec![
        ascii(T_GPS_LAT_REF, lat_ref),
        rational(big, T_GPS_LAT, &[(37, 1), (25, 1), (3075, 100)]),
        ascii(T_GPS_LON_REF, lon_ref),
        rational(big, T_GPS_LON, &[(122, 1), (5, 1), (4890, 100)]),
    ]);
    assemble(big, &[ifd0, gps])
}

/// IFD0 and Exif IFD from `exif-basic` plus the GPS IFD of `exif-gps`: one file
/// with all three, so a corruption of the GPS pointer can be shown to leave
/// both of the others alone.
fn exif_gps_full(big: bool) -> Vec<u8> {
    let mut ifd0 = basic_ifd0(big);
    ifd0.push(ptr(T_GPS_IFD, 2));
    let gps = ifd(vec![
        ascii(T_GPS_LAT_REF, b"N\0"),
        rational(big, T_GPS_LAT, &[(37, 1), (25, 1), (3075, 100)]),
        ascii(T_GPS_LON_REF, b"E\0"),
        rational(big, T_GPS_LON, &[(122, 1), (5, 1), (4890, 100)]),
    ]);
    assemble(big, &[ifd(ifd0), ifd(basic_exif(big)), gps])
}

/// One signed 16-bit, one signed 32-bit, one `FLOAT` and one `DOUBLE`, in the
/// Exif IFD (tags ascending). Values are exactly representable, so the text
/// ExifTool prints and the text `exif-core` renders cannot differ by rounding.
fn signed_float(big: bool) -> Vec<u8> {
    let ifd0 = ifd(vec![ascii(T_MAKE, b"SignedFloat\0"), ptr(T_EXIF_IFD, 1)]);
    let exif = ifd(vec![
        float(big, T_EXPOSURE_TIME, 0.125),
        sshort(big, T_EXPOSURE_PROGRAM, -2),
        double(big, T_FOCAL_LENGTH, 4.5),
        slong(big, T_EXIF_IMAGE_WIDTH, -5),
    ]);
    assemble(big, &[ifd0, exif])
}

/// Make unterminated, Model with an embedded NUL, Artist empty (one NUL).
fn ascii_edges() -> Vec<u8> {
    let ifd0 = ifd(vec![
        ascii(T_MAKE, b"NoTerminator"),
        ascii(T_MODEL, b"ab\0cd\0"),
        ascii(T_ARTIST, b"\0"),
    ]);
    assemble(false, &[ifd0])
}

/// The GPS pointer leads past the end of the file; the Exif IFD is fine.
fn gps_corrupt_exif_ok() -> Vec<u8> {
    let ifd0 = ifd(vec![
        ascii(T_MAKE, b"PartlyBad\0"),
        ptr(T_EXIF_IFD, 1),
        ptr_raw(T_GPS_IFD, 0x00FF_0000),
    ]);
    let exif = ifd(vec![short(false, T_ISO, 200)]);
    assemble(false, &[ifd0, exif])
}

/// IFD0 is fine and chains to an IFD1 whose table is cut short.
fn ifd1_corrupt() -> Vec<u8> {
    let ifd0 = ifd_next(
        vec![
            ascii(T_MAKE, b"ThumbBroken\0"),
            short(false, T_ORIENTATION, 1),
        ],
        1,
    );
    let ifd1 = ifd(vec![
        long(false, T_IMAGE_WIDTH, 160),
        long(false, T_IMAGE_LENGTH, 120),
    ]);
    let mut bytes = assemble(false, &[ifd0, ifd1]);
    // Cut inside IFD1's entry table: 2 (count) + 5 bytes of its first entry.
    let ifd1_at = 8 + 2 + 24 + 4 + (b"ThumbBroken\0".len());
    bytes.truncate(ifd1_at + 7);
    bytes
}

/// Fixtures whose parse is designed to fail (none today): names listed here
/// are allowed to be `Err` in `every_fixture_parses_or_is_a_designed_failure`.
pub const DESIGNED_PARSE_FAILURES: &[&str] = &[];

/// The single named list. Order and names are stable: the committed ExifTool
/// records are keyed by name.
pub fn fixtures() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("exif-basic-le", exif_basic(false)),
        ("exif-basic-be", exif_basic(true)),
        ("exif-gps-ne-le", exif_gps(false, b"N\0", b"E\0")),
        ("exif-gps-nw-le", exif_gps(false, b"N\0", b"W\0")),
        ("exif-gps-se-be", exif_gps(true, b"S\0", b"E\0")),
        ("exif-gps-sw-be", exif_gps(true, b"S\0", b"W\0")),
        ("ascii-edges", ascii_edges()),
        ("gps-corrupt-exif-ok", gps_corrupt_exif_ok()),
        ("ifd1-corrupt", ifd1_corrupt()),
        ("exif-gps-full-le", exif_gps_full(false)),
        ("signed-float-le", signed_float(false)),
        ("signed-float-be", signed_float(true)),
    ]
}
