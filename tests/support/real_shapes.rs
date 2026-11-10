//! Synthetic builders for the real-world shapes SPIKE-001 found (SPEC-019).
//!
//! Real camera and phone files cannot be committed, so each shape that matters
//! to a writer is rebuilt here from the format specifications: the Multi-Picture
//! Format layout of CIPA DC-007 (an APP2 `MPF\0` segment holding a TIFF-style
//! header, an IFD with `MPFVersion` 0xB000, `NumberOfImages` 0xB001 and
//! `MPEntry` 0xB002 at 16 bytes per image, offsets relative to the byte after
//! `MPF\0`), the Photoshop image-resource layout (a block longer than 32 KB is
//! split across APP13 segments), proprietary APP10 and FlashPix APP2 segments,
//! `COM`, a non-Exif trailer after EOI, a Canon-style MakerNote (absolute
//! offsets from the TIFF header), and an Exif APP1 of exactly the maximum size.
//!
//! Tier A: every byte is written here; the builders share nothing with `src/`,
//! with `examples/fuzz_seeds.rs` or with the library's test builders (the TIFF
//! blobs of ordinary Exif come from `oracle_tiff`, which is independent too).
//! Deterministic: [`fixtures`] is a pure function of this file. std only.
//! Whether ExifTool reads each as intended is measured, not assumed
//! (`tests/oracle-fixtures/exiftool-inventory/real-shapes.txt`).

#![allow(dead_code)]

use crate::oracle_tiff as t;

pub const SOI: [u8; 2] = [0xFF, 0xD8];
pub const EOI: [u8; 2] = [0xFF, 0xD9];
/// The longest Exif APP1 payload: a 16-bit length less its own two bytes.
pub const MAX_APP_PAYLOAD: usize = 65_533;
/// Images in the 17-image index (the primary and sixteen secondaries).
pub const SEVENTEEN: usize = 17;
/// The size of the plain trailer the Leica file in SPIKE-001 carried.
pub const TRAILER_LEN: usize = 313;

/// One length-carrying segment.
pub fn seg(marker: u8, payload: &[u8]) -> Vec<u8> {
    let len = u16::try_from(payload.len() + 2).expect("payload fits a 16-bit length");
    let mut v = vec![0xFF, marker];
    v.extend_from_slice(&len.to_be_bytes());
    v.extend_from_slice(payload);
    v
}

fn sos_header() -> Vec<u8> {
    seg(0xDA, &[1, 1, 0, 0, 0x3F, 0])
}

/// A clean JPEG around `segs` (SOI, the segments, SOS, two scan bytes, EOI).
pub fn jpeg_of(segs: &[Vec<u8>]) -> Vec<u8> {
    let mut d = SOI.to_vec();
    for s in segs {
        d.extend_from_slice(s);
    }
    d.extend_from_slice(&sos_header());
    d.extend_from_slice(&[0x12, 0x34]);
    d.extend_from_slice(&EOI);
    d
}

pub fn exif_app1(tiff: &[u8]) -> Vec<u8> {
    seg(0xE1, &[&b"Exif\0\0"[..], tiff].concat())
}

fn jfif() -> Vec<u8> {
    let mut p = b"JFIF\0".to_vec();
    p.extend_from_slice(&[1, 1, 0, 0, 1, 0, 1, 0, 0]);
    seg(0xE0, &p)
}

/// An Exif TIFF with Make, Model and DateTimeOriginal (the "maker" is a
/// parameter so a Nikon-style and an iPhone-style file differ).
fn camera_tiff(big: bool, make: &[u8], model: &[u8]) -> Vec<u8> {
    let ifd0 = t::ifd(vec![
        t::ascii(t::T_MAKE, &[make, b"\0"].concat()),
        t::ascii(t::T_MODEL, &[model, b"\0"].concat()),
        t::ptr(t::T_EXIF_IFD, 1),
    ]);
    let exif = t::ifd(vec![t::ascii(
        t::T_DATE_TIME_ORIGINAL,
        b"2024:05:06 07:08:08\0",
    )]);
    t::assemble(big, &[ifd0, exif])
}

fn xmp_seg() -> Vec<u8> {
    let packet = b"<?xpacket begin=\"\xEF\xBB\xBF\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
        <x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF \
        xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
        <rdf:Description rdf:about=\"\" xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\">\
        <xmp:CreatorTool>ShapeTool</xmp:CreatorTool></rdf:Description></rdf:RDF>\
        </x:xmpmeta>\n<?xpacket end=\"w\"?>";
    seg(
        0xE1,
        &[&b"http://ns.adobe.com/xap/1.0/\0"[..], packet].concat(),
    )
}

/// One ICC chunk (`seq` 1 of 1) around a tiny 128-byte header and an empty tag
/// table: not a usable profile, but a well-formed header ExifTool names.
fn icc_seg() -> Vec<u8> {
    let mut profile = vec![0u8; 128];
    profile[0..4].copy_from_slice(&132u32.to_be_bytes());
    profile[8..12].copy_from_slice(&[2, 0x10, 0, 0]);
    profile[12..16].copy_from_slice(b"mntr");
    profile[16..20].copy_from_slice(b"RGB ");
    profile[20..24].copy_from_slice(b"XYZ ");
    profile[36..40].copy_from_slice(b"acsp");
    profile.extend_from_slice(&0u32.to_be_bytes());
    seg(
        0xE2,
        &[&b"ICC_PROFILE\0"[..], &[1, 1], &profile[..]].concat(),
    )
}

/// A Nikon-style Multi-Picture extra: a JPEG with its own Exif.
pub fn nikon_extra() -> Vec<u8> {
    jpeg_of(&[exif_app1(&camera_tiff(
        false,
        b"NIKON CORPORATION",
        b"NIKON D-SHAPE",
    ))])
}

/// An iPhone-style Multi-Picture extra: a JPEG with XMP and an ICC profile.
pub fn iphone_extra() -> Vec<u8> {
    jpeg_of(&[xmp_seg(), icc_seg()])
}

fn put16(be: bool, v: u16) -> [u8; 2] {
    if be {
        v.to_be_bytes()
    } else {
        v.to_le_bytes()
    }
}

fn put32(be: bool, v: u32) -> [u8; 4] {
    if be {
        v.to_be_bytes()
    } else {
        v.to_le_bytes()
    }
}

/// An APP2 `MPF\0` segment. `entries` is `(attribute, size, offset)` per image,
/// entry 0 first; `declared` is the byte count the `MPEntry` tag claims (normally
/// `16 * entries.len()`); `with_version` adds `MPFVersion` ("0100").
pub fn mpf_segment(
    be: bool,
    entries: &[(u32, u32, u32)],
    declared: u32,
    with_version: bool,
) -> Vec<u8> {
    let n_entries: u16 = if with_version { 3 } else { 2 };
    let ifd_len = 2 + 12 * usize::from(n_entries) + 4;
    let data_at = u32::try_from(8 + ifd_len).expect("small");
    let mut tiff = if be {
        b"MM\0*".to_vec()
    } else {
        b"II*\0".to_vec()
    };
    tiff.extend_from_slice(&put32(be, 8));
    tiff.extend_from_slice(&put16(be, n_entries));
    if with_version {
        tiff.extend_from_slice(&put16(be, 0xB000));
        tiff.extend_from_slice(&put16(be, 7));
        tiff.extend_from_slice(&put32(be, 4));
        tiff.extend_from_slice(b"0100");
    }
    tiff.extend_from_slice(&put16(be, 0xB001));
    tiff.extend_from_slice(&put16(be, 4));
    tiff.extend_from_slice(&put32(be, 1));
    tiff.extend_from_slice(&put32(be, u32::try_from(entries.len()).expect("small")));
    tiff.extend_from_slice(&put16(be, 0xB002));
    tiff.extend_from_slice(&put16(be, 7));
    tiff.extend_from_slice(&put32(be, declared));
    tiff.extend_from_slice(&put32(be, data_at));
    tiff.extend_from_slice(&put32(be, 0));
    for (attr, size, offset) in entries {
        tiff.extend_from_slice(&put32(be, *attr));
        tiff.extend_from_slice(&put32(be, *size));
        tiff.extend_from_slice(&put32(be, *offset));
        tiff.extend_from_slice(&put16(be, 0));
        tiff.extend_from_slice(&put16(be, 0));
    }
    seg(0xE2, &[&b"MPF\0"[..], &tiff].concat())
}

/// Attribute word of the primary image (representative image, baseline MP
/// primary JPEG) and of a secondary (undefined type).
const ATTR_PRIMARY: u32 = 0x2003_0000;
const ATTR_SECONDARY: u32 = 0x0000_0000;

/// What `mpo_with` lets a fixture change about the index it writes.
pub struct Tweak {
    /// Replace the true `(size, offset)` of secondary `i` (index `i + 1`).
    pub entries: Vec<(usize, u32, u32)>,
    /// The byte count the `MPEntry` tag claims; `None` is the true one.
    pub declared: Option<u32>,
    /// Cut the index off after this many entries (the segment simply ends).
    pub cut_after: Option<usize>,
}

impl Tweak {
    pub fn none() -> Tweak {
        Tweak {
            entries: Vec::new(),
            declared: None,
            cut_after: None,
        }
    }
}

/// A Multi-Picture file: a primary (SOI, JFIF, `pre`, the MPF segment, SOS, scan,
/// EOI) followed by the `images` back to back, each indexed at its true place
/// unless `tweak` says otherwise. Returns the file and the `(offset, len)` of
/// each secondary.
pub fn mpo_with(
    be: bool,
    pre: &[Vec<u8>],
    images: &[Vec<u8>],
    tweak: &Tweak,
) -> (Vec<u8>, Vec<(usize, usize)>) {
    let build = |entries: &[(u32, u32, u32)], declared: u32| {
        let mut segs = vec![jfif()];
        segs.extend(pre.iter().cloned());
        let mpf_at = 2 + segs.iter().map(Vec::len).sum::<usize>();
        segs.push(mpf_segment(be, entries, declared, true));
        (jpeg_of(&segs), mpf_at)
    };
    let n = images.len() + 1;
    let placeholder = vec![(0, 0, 0); n];
    let declared = tweak
        .declared
        .unwrap_or_else(|| u32::try_from(16 * n).expect("small"));
    let (probe, mpf_at) = build(&placeholder, declared);
    // The MPF header is the byte after `FF E2 len len M P F 00`.
    let header_start = mpf_at + 8;
    let mut entries = vec![(ATTR_PRIMARY, u32::try_from(probe.len()).expect("small"), 0)];
    let mut at = probe.len();
    let mut ranges = Vec::new();
    for (i, img) in images.iter().enumerate() {
        let (mut size, mut offset) = (
            u32::try_from(img.len()).expect("small"),
            u32::try_from(at - header_start).expect("small"),
        );
        if let Some((_, s, o)) = tweak.entries.iter().find(|(k, _, _)| *k == i) {
            (size, offset) = (*s, *o);
        }
        entries.push((ATTR_SECONDARY, size, offset));
        ranges.push((at, img.len()));
        at += img.len();
    }
    if let Some(cut) = tweak.cut_after {
        entries.truncate(cut);
    }
    let (mut file, _) = build(&entries, declared);
    // A cut index makes the segment shorter, which moves nothing the images
    // depend on only when no cut is asked for.
    if tweak.cut_after.is_none() {
        assert_eq!(file.len(), probe.len());
    }
    for img in images {
        file.extend_from_slice(img);
    }
    (file, ranges)
}

/// A well-formed MPO with the true index.
pub fn mpo(be: bool, images: &[Vec<u8>]) -> Vec<u8> {
    mpo_with(
        be,
        &[exif_app1(&camera_tiff(
            be,
            b"NIKON CORPORATION",
            b"NIKON D-SHAPE",
        ))],
        images,
        &Tweak::none(),
    )
    .0
}

/// A Nikon file: the primary has Exif and two extras, one with its own Exif,
/// one with XMP and ICC.
pub fn mpo_two(be: bool) -> Vec<u8> {
    mpo(be, &[nikon_extra(), iphone_extra()])
}

/// The index declares three images but the segment ends after the second
/// entry: the MPEntry value runs past the segment.
pub fn mpo_truncated_index() -> Vec<u8> {
    let (f, _) = mpo_with(
        false,
        &[],
        &[nikon_extra(), iphone_extra()],
        &Tweak {
            entries: Vec::new(),
            declared: None,
            cut_after: Some(2),
        },
    );
    f
}

/// Secondary 0's offset points past the end of the file.
pub fn mpo_offset_out_of_range() -> Vec<u8> {
    let imgs = [nikon_extra(), iphone_extra()];
    let (f, _) = mpo_with(
        false,
        &[],
        &imgs,
        &Tweak {
            entries: vec![(0, 5_000, 0x00FF_0000)],
            declared: None,
            cut_after: None,
        },
    );
    f
}

/// Secondary 0 is not a JPEG: the bytes the index points at start `FF D9`.
pub fn mpo_image_not_jpeg() -> Vec<u8> {
    let not_jpeg = vec![0xFF, 0xD9, 1, 2, 3, 4, 5, 6];
    let (f, _) = mpo_with(false, &[], &[not_jpeg, iphone_extra()], &Tweak::none());
    f
}

/// Secondary 0 carries an MPF index of its own, listing a further image.
pub fn mpo_inner_mpf() -> Vec<u8> {
    let deep = nikon_extra();
    let (mid, _) = mpo_with(false, &[], std::slice::from_ref(&deep), &Tweak::none());
    mpo(false, &[mid])
}

/// A 17-image index (the primary and sixteen secondaries): over the limit.
pub fn mpo_seventeen() -> Vec<u8> {
    let imgs: Vec<Vec<u8>> = (1..SEVENTEEN).map(|_| iphone_extra()).collect();
    mpo(false, &imgs)
}

/// A JPEG with a plain trailer of `TRAILER_LEN` bytes after EOI (the Leica
/// shape: not Exif, not a JPEG).
pub fn trailer_313() -> Vec<u8> {
    let mut f = jpeg_of(&[
        jfif(),
        exif_app1(&camera_tiff(false, b"LEICA CAMERA AG", b"LEICA D-SHAPE")),
    ]);
    f.extend((0..TRAILER_LEN).map(|i| 0x41 + (i % 23) as u8));
    f
}

/// A Photoshop block split across two APP13 segments: one `0x0404` resource
/// whose data crosses the boundary (the first segment ends inside the data).
pub fn split_app13() -> Vec<u8> {
    let mut data = Vec::new();
    // IIM records: 1:90 (CodedCharacterSet) then 2:120 caption, 2:80 by-line.
    data.extend_from_slice(&[0x1C, 0x01, 0x5A, 0x00, 0x03, 0x1B, 0x25, 0x47]);
    data.extend_from_slice(&[0x1C, 0x02, 0x78, 0x00, 0x0B]);
    data.extend_from_slice(b"split block");
    data.extend_from_slice(&[0x1C, 0x02, 0x50, 0x00, 0x0A]);
    data.extend_from_slice(b"A. Shaper\0");
    let mut resource = b"8BIM".to_vec();
    resource.extend_from_slice(&0x0404u16.to_be_bytes());
    resource.extend_from_slice(&[0, 0]);
    resource.extend_from_slice(&u32::try_from(data.len()).expect("small").to_be_bytes());
    resource.extend_from_slice(&data);
    if data.len() % 2 == 1 {
        resource.push(0);
    }
    // Split inside the data, so the first segment's resource is short.
    let cut = 12 + 10;
    let (head, tail) = resource.split_at(cut);
    let sig = b"Photoshop 3.0\0";
    jpeg_of(&[
        jfif(),
        seg(0xED, &[&sig[..], head].concat()),
        seg(0xED, &[&sig[..], tail].concat()),
    ])
}

/// A proprietary APP10 `AROT` segment (the iPhone one in SPIKE-001).
pub fn app10_arot() -> Vec<u8> {
    jpeg_of(&[
        jfif(),
        seg(
            0xEA,
            &[&b"AROT\0"[..], &[0, 0, 0, 1, 0, 0, 0, 0][..]].concat(),
        ),
    ])
}

/// A FlashPix `FPXR` APP2 segment.
pub fn app2_fpxr() -> Vec<u8> {
    jpeg_of(&[
        jfif(),
        seg(
            0xE2,
            &[&b"FPXR\0"[..], &[0, 1, 0, 0, 0, 0, 0, 0, 0, 0][..]].concat(),
        ),
    ])
}

/// A `COM` segment.
pub fn com_segment() -> Vec<u8> {
    jpeg_of(&[jfif(), seg(0xFE, b"Shaped by the real_shapes builders")])
}

// ── the Canon-style absolute-offset MakerNote ───────────────────────────────

/// The TIFF blob of an Exif block whose MakerNote is an IFD with entries whose
/// offsets are absolute from the TIFF header (the Canon convention), and the
/// byte range of the whole note inside the blob. `shift` bytes of padding are
/// put in front of the note and the MakerNote pointer follows it, but the note's
/// own offsets are **not** adjusted: that is what moving the note inside the
/// block does to a naive writer.
fn canon_tiff(shift: usize) -> (Vec<u8>, usize, usize) {
    const MAKE: &[u8] = b"Canon\0";
    const MODEL: &[u8] = b"Canon EOS DIAMOND\0";
    // Three ASCII values the note points at (Canon:CanonImageType 0x0006,
    // CanonFirmwareVersion 0x0007, OwnerName 0x0009), as 32-byte fields.
    let value = |s: &[u8]| {
        let mut v = s.to_vec();
        v.resize(32, 0);
        v
    };
    let image_type = value(b"Canon EOS DIAMOND SHAPE");
    let firmware = value(b"Firmware Version 1.0.0");
    let owner = value(b"Shape Owner");
    let note_entries = 3usize;
    let note_ifd_len = 2 + 12 * note_entries + 4;
    let note_len = note_ifd_len + 3 * 32;

    // Layout: header (8), IFD0 (3 entries), Make, Model, Exif IFD (1 entry),
    // padding, the note.
    let ifd0_at = 8usize;
    let ifd0_len = 2 + 12 * 3 + 4;
    let make_at = ifd0_at + ifd0_len;
    let model_at = make_at + MAKE.len();
    let exif_at = model_at + MODEL.len() + (MODEL.len() & 1);
    let exif_len = 2 + 12 + 4;
    let note_at = exif_at + exif_len + shift;

    let le16 = |v: usize| u16::try_from(v).expect("small").to_le_bytes();
    let le32 = |v: usize| u32::try_from(v).expect("small").to_le_bytes();
    let entry = |out: &mut Vec<u8>, tag: u16, typ: u16, count: usize, field: [u8; 4]| {
        out.extend_from_slice(&tag.to_le_bytes());
        out.extend_from_slice(&typ.to_le_bytes());
        out.extend_from_slice(&le32(count));
        out.extend_from_slice(&field);
    };

    let mut out = b"II*\0".to_vec();
    out.extend_from_slice(&le32(ifd0_at));
    out.extend_from_slice(&le16(3));
    entry(&mut out, t::T_MAKE, 2, MAKE.len(), le32(make_at));
    entry(&mut out, t::T_MODEL, 2, MODEL.len(), le32(model_at));
    entry(&mut out, t::T_EXIF_IFD, 4, 1, le32(exif_at));
    out.extend_from_slice(&le32(0));
    out.extend_from_slice(MAKE);
    out.extend_from_slice(MODEL);
    if MODEL.len() & 1 == 1 {
        out.push(0);
    }
    assert_eq!(out.len(), exif_at);
    out.extend_from_slice(&le16(1));
    entry(&mut out, 0x927C, 7, note_len, le32(note_at));
    out.extend_from_slice(&le32(0));
    out.extend(std::iter::repeat_n(0u8, shift));
    assert_eq!(out.len(), note_at);
    // The note: offsets absolute from the TIFF header, and computed for the
    // note's UNSHIFTED place, so a shifted note keeps stale offsets.
    let nominal_at = exif_at + exif_len;
    let data_at = nominal_at + note_ifd_len;
    out.extend_from_slice(&le16(note_entries));
    entry(&mut out, 0x0006, 2, 32, le32(data_at));
    entry(&mut out, 0x0007, 2, 32, le32(data_at + 32));
    entry(&mut out, 0x0009, 2, 32, le32(data_at + 64));
    out.extend_from_slice(&le32(0));
    out.extend_from_slice(&image_type);
    out.extend_from_slice(&firmware);
    out.extend_from_slice(&owner);
    (out, note_at, note_len)
}

/// A JPEG whose Exif has `Make=Canon` and a MakerNote IFD with absolute offsets:
/// ExifTool reads Canon tags from it.
pub fn canon_style_absolute_note() -> Vec<u8> {
    jpeg_of(&[jfif(), exif_app1(&canon_tiff(0).0)])
}

/// The same Exif with the note moved 64 bytes further into the block and the
/// MakerNote pointer updated, but the note's own absolute offsets left alone.
pub fn canon_style_note_moved() -> Vec<u8> {
    jpeg_of(&[jfif(), exif_app1(&canon_tiff(64).0)])
}

/// `(range of the note in the TIFF blob, blob length)` for the unmoved fixture.
pub fn canon_note_range() -> (usize, usize, usize) {
    let (blob, at, len) = canon_tiff(0);
    (at, len, blob.len())
}

// ── an Exif block of the maximum size ───────────────────────────────────────

/// A JPEG whose single Exif APP1 has a payload of exactly 65,533 bytes (the
/// largest a segment can carry): a big out-of-line ImageDescription pads it.
pub fn exif_at_64k() -> Vec<u8> {
    let make = b"MaxCam\0".to_vec();
    // header 8 + IFD0 (2 entries) = 8 + 2 + 24 + 4, then Make, then the text.
    let fixed = 8 + 2 + 2 * 12 + 4 + make.len();
    let tiff_len = MAX_APP_PAYLOAD - 6;
    let text_len = tiff_len - fixed;
    let mut text = vec![b'x'; text_len - 1];
    text.push(0);
    let tiff = t::assemble(
        false,
        &[t::ifd(vec![
            t::ascii(0x010E, &text),
            t::ascii(t::T_MAKE, &make),
        ])],
    );
    // The builder word-aligns each payload; the last one (Make, 7 bytes) is
    // padded to 8, and the extra pad byte is dropped to land on the maximum.
    let mut tiff = tiff;
    if tiff.len() == tiff_len + 1 && tiff.last() == Some(&0) {
        tiff.pop();
    }
    assert_eq!(tiff.len(), tiff_len, "the Exif payload is the maximum");
    let seg_bytes = exif_app1(&tiff);
    assert_eq!(seg_bytes.len(), 4 + MAX_APP_PAYLOAD);
    jpeg_of(&[seg_bytes])
}

/// The Canon tags the relocation test asks ExifTool for (`-T -n -s3`).
pub const CANON_SELECTORS: [&str; 3] = [
    "Canon:CanonImageType",
    "Canon:CanonFirmwareVersion",
    "Canon:OwnerName",
];

/// The two fixtures whose Canon tags are recorded, and the record file of each
/// (under `tests/oracle-fixtures/exiftool-inventory/`).
pub const CANON_RECORDED: [&str; 2] = ["canon-style-note", "canon-style-note-moved"];

/// The record file name of a recorded Canon fixture.
pub fn canon_record_file(name: &str) -> String {
    format!("real-shapes-{name}-values.txt")
}

/// The named fixtures, in a fixed order. Names are the record section names.
pub fn fixtures() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("mpo-two-le", mpo_two(false)),
        ("mpo-two-be", mpo_two(true)),
        ("mpo-truncated-index", mpo_truncated_index()),
        ("mpo-offset-out-of-range", mpo_offset_out_of_range()),
        ("mpo-image-not-jpeg", mpo_image_not_jpeg()),
        ("mpo-inner-mpf", mpo_inner_mpf()),
        ("mpo-seventeen", mpo_seventeen()),
        ("split-app13", split_app13()),
        ("app10-arot", app10_arot()),
        ("app2-fpxr", app2_fpxr()),
        ("com-segment", com_segment()),
        ("trailer-313", trailer_313()),
        ("canon-style-note", canon_style_absolute_note()),
        ("canon-style-note-moved", canon_style_note_moved()),
        ("exif-64k", exif_at_64k()),
    ]
}
