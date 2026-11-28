//! The independent model of the Exif block (SPEC-025, AC3). It reads the
//! source with its own minimal TIFF reader and predicts the block byte for
//! byte from DEC-014's amendment item 7 (tables Exif, Interop, GPS; then the
//! out-of-line values in that IFD order and ascending tag order, each at an
//! even block-relative offset, zero padded). It imports nothing from
//! `exif_core`. Its omission table is one of the hand-typed mirrors of the
//! decision (AC4): `omission_table_mirrors_agree` compares them.
//!
//! It handles sources whose pointers are well formed. The shapes that refuse
//! carry their expectation in the shape instead.

#![allow(dead_code)]

/// The omission table, typed again here by hand from DEC-014's amendment.
pub const OFFSET_CARRYING: [u16; 23] = [
    // The 11 offset tags ExifTool 13.55 relocates only with their length.
    0x0111, 0x0120, 0x0144, 0x0201, 0x0207, 0x0208, 0x0209, 0x8781, 0xA010, 0xBCC0, 0xBCC2,
    // SubIFDs and the other pointer tags (0xA005 is special-cased below).
    0x014A, 0x8769, 0x8825, 0xA005,
    // Conservative: ExifTool does not follow these from the three IFDs.
    0x0190, 0x888A, 0x8290, 0xFE00, 0xC51B, 0x8568, 0xCD41, 0xC6F5,
];
pub const MAKERNOTE: [u16; 2] = [0x927C, 0xEA1D];

// ── the ByteOrder table (SPEC-041, DEC-014 amendment of 2026-11-28, item 3) ──
//
// Typed again here by hand, as explicit type lists (the library names masks).
// Each row is `(tag, the declared types left out in cross order)`.

/// Types 3, 4, 5, 8, 9, 10, 11, 12.
pub const MULTI_BYTE_TYPES: [u16; 8] = [3, 4, 5, 8, 9, 10, 11, 12];
/// Types 1, 2, 6, 7.
pub const ONE_BYTE_TYPES: [u16; 4] = [1, 2, 6, 7];
/// Types 1 to 12.
pub const EVERY_TYPE: [u16; 12] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];

/// The 35 tags every multi-byte type of which is left out (Exif and Interop).
pub const MULTI_BYTE_TAGS: [u16; 35] = [
    0x0303, 0x4747, 0x828E, 0x8298, 0x9009, 0x9101, 0x9286, 0x9C9B, 0x9C9C, 0x9C9D, 0x9C9E, 0x9C9F,
    0xBC01, 0xC615, 0xC65D, 0xC68B, 0xC68C, 0xC68F, 0xC691, 0xC6D2, 0xC6D3, 0xC6F3, 0xC6F4, 0xC6F6,
    0xC6F8, 0xC6FE, 0xC716, 0xC717, 0xC718, 0xC719, 0xC71C, 0xC71D, 0xC7A7, 0xC7AA, 0xCD48,
];

/// The rows with their own masks (Exif and Interop).
pub const OTHER_ROWS: [(u16, &[u16]); 6] = [
    (0x87AF, &[12]),
    (0x9201, &ONE_BYTE_TYPES),
    (0x9204, &[1, 2, 3, 6, 7, 8, 12]),
    (0xA101, &[4, 5, 9, 10, 11, 12]),
    (0xA302, &[1, 2, 3, 4, 5, 6, 8, 9, 10, 11, 12]),
    (0xA462, &EVERY_TYPE),
];

/// The three tags the Exif specification starts with two SHORTs: every type.
pub const SPEC_TAGS: [u16; 3] = [0x8828, 0xA20C, 0xA40B];

/// The GPS row.
pub const GPS_ROWS: [(u16, &[u16]); 1] = [(0x001D, &[4, 5, 9, 10, 11, 12])];

/// Every `(tag, types)` row of the Exif and the Interop IFD.
pub fn exif_rows() -> Vec<(u16, Vec<u16>)> {
    let mut rows: Vec<(u16, Vec<u16>)> = MULTI_BYTE_TAGS
        .iter()
        .map(|t| (*t, MULTI_BYTE_TYPES.to_vec()))
        .collect();
    rows.extend(OTHER_ROWS.iter().map(|(t, v)| (*t, v.to_vec())));
    rows.extend(SPEC_TAGS.iter().map(|t| (*t, EVERY_TYPE.to_vec())));
    rows
}

/// Every `(tag, types)` row of the GPS IFD.
pub fn gps_rows() -> Vec<(u16, Vec<u16>)> {
    GPS_ROWS.iter().map(|(t, v)| (*t, v.to_vec())).collect()
}

/// Whether `(ifd, tag, ty)` is on the table.
pub fn byte_order_sensitive(ifd: Ifd, tag: u16, ty: u16) -> bool {
    if ifd == Ifd::Gps {
        return GPS_ROWS
            .iter()
            .any(|(t, types)| *t == tag && types.contains(&ty));
    }
    (MULTI_BYTE_TAGS.contains(&tag) && MULTI_BYTE_TYPES.contains(&ty))
        || OTHER_ROWS
            .iter()
            .any(|(t, types)| *t == tag && types.contains(&ty))
        || (SPEC_TAGS.contains(&tag) && EVERY_TYPE.contains(&ty))
}

/// Where the UNICODE rule applies (item 5): 0x9286 as a one-byte type in the
/// Exif or Interop IFD; 0x001B and 0x001C as UNDEFINED in the GPS IFD.
pub fn unicode_place(ifd: Ifd, tag: u16, ty: u16) -> bool {
    match ifd {
        Ifd::Exif | Ifd::Interop => tag == 0x9286 && ONE_BYTE_TYPES.contains(&ty),
        Ifd::Gps => (tag == 0x001B || tag == 0x001C) && ty == 7,
    }
}

/// Convert one payload from the source's order into the other (items 2, 4, 5),
/// with its own swap code.
pub fn convert(ifd: Ifd, tag: u16, ty: u16, p: &[u8]) -> Vec<u8> {
    let unit = match ty {
        3 | 8 => 2,
        4 | 5 | 9 | 10 | 11 => 4,
        12 => 8,
        _ => 1,
    };
    let mut out = p.to_vec();
    if unit > 1 {
        for c in out.chunks_exact_mut(unit) {
            c.reverse();
        }
    }
    if ty == 7 && tag == 0xA302 && ifd != Ifd::Gps && out.len() >= 4 {
        out[0..2].reverse();
        out[2..4].reverse();
    }
    if unicode_place(ifd, tag, ty)
        && p.len() >= 8
        && &p[..7] == b"UNICODE"
        && (p[7] == 0x00 || p[7] == 0x20)
    {
        for c in out[8..].chunks_exact_mut(2) {
            c.reverse();
        }
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ifd {
    Exif,
    Interop,
    Gps,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    MakerNote,
    OffsetCarrying,
    UnknownType,
    ByteOrder,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Omit {
    pub ifd: Ifd,
    pub tag: u16,
    pub field_type: u16,
    pub reason: Reason,
}

#[derive(Clone, Debug)]
pub struct Model {
    /// The block's byte order (the target's when one was asked for).
    pub be: bool,
    pub bytes: Vec<u8>,
    pub exif_pos: Option<u32>,
    pub gps_pos: Option<u32>,
    pub omissions: Vec<Omit>,
}

#[derive(Clone, Debug)]
struct Ent {
    tag: u16,
    ty: u16,
    count: u32,
    field: [u8; 4],
}

fn rd16(d: &[u8], be: bool, at: usize) -> Option<u16> {
    let b: [u8; 2] = d.get(at..at + 2)?.try_into().ok()?;
    Some(if be {
        u16::from_be_bytes(b)
    } else {
        u16::from_le_bytes(b)
    })
}

fn rd32(d: &[u8], be: bool, at: usize) -> Option<u32> {
    let b: [u8; 4] = d.get(at..at + 4)?.try_into().ok()?;
    Some(if be {
        u32::from_be_bytes(b)
    } else {
        u32::from_le_bytes(b)
    })
}

fn size_of(ty: u16) -> Option<usize> {
    match ty {
        1 | 2 | 6 | 7 => Some(1),
        3 | 8 => Some(2),
        4 | 9 | 11 | 13 => Some(4),
        5 | 10 | 12 => Some(8),
        _ => None,
    }
}

fn table(d: &[u8], be: bool, at: usize) -> Option<Vec<Ent>> {
    let n = usize::from(rd16(d, be, at)?);
    let mut out = Vec::new();
    for i in 0..n {
        let e = at + 2 + 12 * i;
        out.push(Ent {
            tag: rd16(d, be, e)?,
            ty: rd16(d, be, e + 2)?,
            count: rd32(d, be, e + 4)?,
            field: d.get(e + 8..e + 12)?.try_into().ok()?,
        });
    }
    Some(out)
}

fn field_u32(be: bool, f: [u8; 4]) -> u32 {
    if be {
        u32::from_be_bytes(f)
    } else {
        u32::from_le_bytes(f)
    }
}

/// The payload of an entry in the source.
fn payload(d: &[u8], be: bool, e: &Ent) -> Option<Vec<u8>> {
    let len = size_of(e.ty)?.checked_mul(e.count as usize)?;
    if len <= 4 {
        Some(e.field[..len].to_vec())
    } else {
        let at = field_u32(be, e.field) as usize;
        Some(d.get(at..at.checked_add(len)?)?.to_vec())
    }
}

/// The IFD a pointer entry of IFD `t` leads to: the first entry of `tag`.
fn pointed(d: &[u8], be: bool, t: &[Ent], tag: u16) -> Option<Vec<Ent>> {
    let e = t.iter().find(|e| e.tag == tag)?;
    if !(e.ty == 4 || e.ty == 13) || e.count != 1 {
        return None;
    }
    let at = field_u32(be, e.field);
    if at == 0 {
        return None;
    }
    table(d, be, at as usize)
}

fn reason(ifd: Ifd, e: &Ent, interop_present: bool, cross: bool) -> Option<Reason> {
    if size_of(e.ty).is_none() {
        return Some(Reason::UnknownType);
    }
    if MAKERNOTE.contains(&e.tag) {
        return Some(Reason::MakerNote);
    }
    if e.tag == 0xA005 && ifd == Ifd::Exif {
        return if interop_present {
            None
        } else {
            Some(Reason::OffsetCarrying)
        };
    }
    if OFFSET_CARRYING.contains(&e.tag) || e.ty == 13 {
        return Some(Reason::OffsetCarrying);
    }
    if cross && byte_order_sensitive(ifd, e.tag, e.ty) {
        return Some(Reason::ByteOrder);
    }
    None
}

struct Kept {
    ifd: Ifd,
    tag: u16,
    ty: u16,
    count: u32,
    payload: Vec<u8>,
    interop_ptr: bool,
}

/// Predict the block for `src` at `base`, in the source's own order.
pub fn model(src: &[u8], base: u32) -> Model {
    model_to(src, base, None)
}

/// Predict the block for `src` at `base` in `target_be` (`None`: the source's
/// order). In the other order each kept payload is converted by declared type
/// and the rules of items 4 and 5, and the `ByteOrder` table applies.
pub fn model_to(src: &[u8], base: u32, target_be: Option<bool>) -> Model {
    let src_be = &src[0..2] == b"MM";
    let be = target_be.unwrap_or(src_be);
    let cross = be != src_be;
    let empty = Model {
        be,
        bytes: Vec::new(),
        exif_pos: None,
        gps_pos: None,
        omissions: Vec::new(),
    };
    let ifd0_at = rd32(src, src_be, 4).unwrap_or(0) as usize;
    if ifd0_at == 0 {
        return empty;
    }
    let Some(ifd0) = table(src, src_be, ifd0_at) else {
        return empty;
    };
    let exif = pointed(src, src_be, &ifd0, 0x8769);
    let gps = pointed(src, src_be, &ifd0, 0x8825);
    let interop = exif.as_ref().and_then(|t| pointed(src, src_be, t, 0xA005));

    let mut omissions = Vec::new();
    // (ifd, kept entries) in Exif, Interop, GPS order.
    let mut tables: Vec<(Ifd, Vec<Kept>)> = Vec::new();
    for (which, t) in [
        (Ifd::Exif, &exif),
        (Ifd::Interop, &interop),
        (Ifd::Gps, &gps),
    ] {
        let Some(t) = t else { continue };
        let mut sorted = t.clone();
        sorted.sort_by_key(|e| e.tag);
        let mut kept = Vec::new();
        for e in &sorted {
            match reason(which, e, interop.is_some(), cross) {
                Some(r) => omissions.push(Omit {
                    ifd: which,
                    tag: e.tag,
                    field_type: e.ty,
                    reason: r,
                }),
                None if e.tag == 0xA005 && which == Ifd::Exif => kept.push(Kept {
                    ifd: which,
                    tag: e.tag,
                    ty: 4,
                    count: 1,
                    payload: Vec::new(),
                    interop_ptr: true,
                }),
                None => {
                    let p = payload(src, src_be, e).expect("model: readable payload");
                    kept.push(Kept {
                        ifd: which,
                        tag: e.tag,
                        ty: e.ty,
                        count: e.count,
                        payload: if cross {
                            convert(which, e.tag, e.ty, &p)
                        } else {
                            p
                        },
                        interop_ptr: false,
                    });
                }
            }
        }
        tables.push((which, kept));
    }
    if tables.is_empty() {
        return empty;
    }

    // Layout: tables first, then values.
    let mut pos = 0u32;
    let mut at = Vec::new();
    for (_, kept) in &tables {
        at.push(pos);
        pos += 6 + 12 * kept.len() as u32;
    }
    let mut voff: Vec<Vec<Option<u32>>> = Vec::new();
    for (_, kept) in &tables {
        let mut offs = Vec::new();
        for k in kept {
            if !k.interop_ptr && k.payload.len() > 4 {
                offs.push(Some(pos));
                pos += k.payload.len() as u32 + (k.payload.len() as u32 % 2);
            } else {
                offs.push(None);
            }
        }
        voff.push(offs);
    }
    let len = pos;
    let pos_of = |which: Ifd| tables.iter().position(|(i, _)| *i == which).map(|n| at[n]);
    let w16 = |v: u16| if be { v.to_be_bytes() } else { v.to_le_bytes() };
    let w32 = |v: u32| if be { v.to_be_bytes() } else { v.to_le_bytes() };

    let mut bytes = vec![0u8; len as usize];
    for (n, (_, kept)) in tables.iter().enumerate() {
        let t = at[n] as usize;
        bytes[t..t + 2].copy_from_slice(&w16(kept.len() as u16));
        for (i, k) in kept.iter().enumerate() {
            let e = t + 2 + 12 * i;
            bytes[e..e + 2].copy_from_slice(&w16(k.tag));
            bytes[e + 2..e + 4].copy_from_slice(&w16(k.ty));
            bytes[e + 4..e + 8].copy_from_slice(&w32(k.count));
            if k.interop_ptr {
                let p = pos_of(Ifd::Interop).expect("model: interop table");
                bytes[e + 8..e + 12].copy_from_slice(&w32(base + p));
            } else if let Some(o) = voff[n][i] {
                bytes[e + 8..e + 12].copy_from_slice(&w32(base + o));
                let o = o as usize;
                bytes[o..o + k.payload.len()].copy_from_slice(&k.payload);
            } else {
                bytes[e + 8..e + 8 + k.payload.len()].copy_from_slice(&k.payload);
            }
        }
        // The next-word stays 0.
    }
    Model {
        be,
        bytes,
        exif_pos: pos_of(Ifd::Exif),
        gps_pos: pos_of(Ifd::Gps),
        omissions,
    }
}
