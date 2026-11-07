//! SPEC-015: byte-level inputs for the report tests and the `audit_render`
//! seeds, built without the library's own writers. Included with
//! `#[path = ...] mod report_inputs;` by `tests/report_render.rs` and by
//! `examples/fuzz_seeds.rs`, so the seeds and the tests cannot drift.
#![allow(dead_code)]

/// A JPEG segment: `FF marker length payload` (the length counts its own two
/// bytes).
pub fn seg(marker: u8, payload: &[u8]) -> Vec<u8> {
    let len = u16::try_from(payload.len() + 2).expect("segment fits");
    let mut v = vec![0xFF, marker];
    v.extend_from_slice(&len.to_be_bytes());
    v.extend_from_slice(payload);
    v
}

/// SOI, the segments, a start-of-scan header with two scan bytes, EOI.
pub fn jpeg(segments: &[Vec<u8>]) -> Vec<u8> {
    let mut d = vec![0xFF, 0xD8];
    for s in segments {
        d.extend_from_slice(s);
    }
    d.extend_from_slice(&seg(0xDA, &[1, 1, 0, 0, 0x3F, 0]));
    d.extend_from_slice(&[0x12, 0x34, 0xFF, 0xD9]);
    d
}

pub fn exif_app1(tiff: &[u8]) -> Vec<u8> {
    seg(0xE1, &[&b"Exif\0\0"[..], tiff].concat())
}

/// One little-endian IFD: `(tag, type, count, value-field)` entries (inline
/// values only), then the next-IFD offset.
pub fn le_ifd(entries: &[(u16, u16, u32, u32)], next: u32) -> Vec<u8> {
    let mut v = u16::try_from(entries.len())
        .expect("few")
        .to_le_bytes()
        .to_vec();
    for (tag, typ, count, val) in entries {
        v.extend_from_slice(&tag.to_le_bytes());
        v.extend_from_slice(&typ.to_le_bytes());
        v.extend_from_slice(&count.to_le_bytes());
        v.extend_from_slice(&val.to_le_bytes());
    }
    v.extend_from_slice(&next.to_le_bytes());
    v
}

/// A TIFF with a tag of each of the seven tag categories (IFD0, ExifIFD, GPS,
/// IFD1), a MakerNote and an IFD1 thumbnail; every value is inline.
pub fn full_house_tiff() -> Vec<u8> {
    const A: u32 = 0x0000_0061; // "a\0" as an inline ASCII value
    let ifd0_len = 2 + 12 * 8 + 4;
    let exif_len = 2 + 12 * 3 + 4;
    let gps_len = 2 + 12 + 4;
    let exif_at = u32::try_from(8 + ifd0_len).expect("small");
    let gps_at = exif_at + u32::try_from(exif_len).expect("small");
    let ifd1_at = gps_at + u32::try_from(gps_len).expect("small");
    let mut t = b"II\x2A\x00\x08\x00\x00\x00".to_vec();
    t.extend(le_ifd(
        &[
            (0x010E, 2, 2, A), // FreeText
            (0x010F, 2, 2, A), // CameraModel
            (0x0131, 2, 2, A), // Software
            (0x0132, 2, 2, A), // Timestamps
            (0x013B, 2, 2, A), // Authorship
            (0x8769, 4, 1, exif_at),
            (0x8825, 4, 1, gps_at),
            (0xA431, 2, 2, A), // DeviceId
        ],
        ifd1_at,
    ));
    t.extend(le_ifd(
        &[(0x9003, 2, 2, A), (0xA431, 2, 2, A), (0x927C, 7, 4, A)],
        0,
    ));
    t.extend(le_ifd(&[(0x0001, 2, 2, A)], 0));
    t.extend(le_ifd(&[(0x0201, 4, 1, 0), (0x0202, 4, 1, 0)], 0));
    t
}

/// The full-house JPEG: that TIFF in an Exif APP1, plus XMP, IPTC, ICC, JUMBF.
pub fn full_house_jpeg() -> Vec<u8> {
    let iptc_data = [0x1C, 0x02, 0x05, 0x00, 0x01, b'T'];
    let mut res = b"8BIM".to_vec();
    res.extend_from_slice(&0x0404u16.to_be_bytes());
    res.extend_from_slice(&[0, 0, 0, 0, 0, 6]);
    res.extend_from_slice(&iptc_data);
    jpeg(&[
        exif_app1(&full_house_tiff()),
        seg(
            0xE1,
            &[&b"http://ns.adobe.com/xap/1.0/\0"[..], &b"<x/>"[..]].concat(),
        ),
        seg(0xED, &[&b"Photoshop 3.0\0"[..], &res].concat()),
        seg(0xE2, b"ICC_PROFILE\0\x01\x01profile"),
        seg(
            0xEB,
            &[0x4A, 0x50, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x6A],
        ),
    ])
}

/// A little-endian TIFF whose IFD0 holds `entries` as ASCII tags with
/// out-of-line payloads: `(tag, field type, payload bytes)`. The payloads sit
/// right behind the IFD, in order, without padding.
pub fn tiff_with_payloads(entries: &[(u16, u16, &[u8])]) -> Vec<u8> {
    let n = entries.len();
    let ifd_len = 2 + 12 * n + 4;
    let mut at = u32::try_from(8 + ifd_len).expect("small");
    let mut t = b"II\x2A\x00\x08\x00\x00\x00".to_vec();
    t.extend_from_slice(&u16::try_from(n).expect("few").to_le_bytes());
    let mut tail: Vec<u8> = Vec::new();
    for (tag, typ, payload) in entries {
        t.extend_from_slice(&tag.to_le_bytes());
        t.extend_from_slice(&typ.to_le_bytes());
        t.extend_from_slice(&u32::try_from(payload.len()).expect("small").to_le_bytes());
        t.extend_from_slice(&at.to_le_bytes());
        tail.extend_from_slice(payload);
        at += u32::try_from(payload.len()).expect("small");
    }
    t.extend_from_slice(&0u32.to_le_bytes());
    t.extend_from_slice(&tail);
    t
}

/// The distinctive payload of the value-free test.
pub const SECRET: &[u8] = b"SECRET-SERIAL-0042";

/// A TIFF whose Make (0x010F), Artist (0x013B) and BodySerialNumber (0xA431,
/// in IFD0 for the test's sake: DEC-008 maps the id wherever it sits outside
/// GPS and Interop) all hold [`SECRET`].
pub fn secret_tiff() -> Vec<u8> {
    let mut p = SECRET.to_vec();
    p.push(0);
    tiff_with_payloads(&[(0x010F, 2, &p), (0x013B, 2, &p), (0xA431, 2, &p)])
}
