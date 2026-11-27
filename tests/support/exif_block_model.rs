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

fn reason(ifd: Ifd, e: &Ent, interop_present: bool) -> Option<Reason> {
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

/// Predict the block for `src` at `base`.
pub fn model(src: &[u8], base: u32) -> Model {
    let be = &src[0..2] == b"MM";
    let empty = Model {
        be,
        bytes: Vec::new(),
        exif_pos: None,
        gps_pos: None,
        omissions: Vec::new(),
    };
    let ifd0_at = rd32(src, be, 4).unwrap_or(0) as usize;
    if ifd0_at == 0 {
        return empty;
    }
    let Some(ifd0) = table(src, be, ifd0_at) else {
        return empty;
    };
    let exif = pointed(src, be, &ifd0, 0x8769);
    let gps = pointed(src, be, &ifd0, 0x8825);
    let interop = exif.as_ref().and_then(|t| pointed(src, be, t, 0xA005));

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
            match reason(which, e, interop.is_some()) {
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
                None => kept.push(Kept {
                    ifd: which,
                    tag: e.tag,
                    ty: e.ty,
                    count: e.count,
                    payload: payload(src, be, e).expect("model: readable payload"),
                    interop_ptr: false,
                }),
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
