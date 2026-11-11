//! Fixtures for the Exif entry editor (SPEC-021): TIFF blobs built from a
//! declarative list of IFDs, with a **record of the layout** (every IFD table,
//! every entry's position, every out-of-line value, the thumbnail) written by
//! the builder as it lays the bytes out, and an independent model of the edit
//! ([`expected`]) that computes the output blob from that record alone. Nothing
//! here imports `src/`'s walk or planner; the category table is typed again by
//! hand from DEC-008.

#![allow(dead_code)]

use std::ops::Range;

pub const BYTE: u16 = 1;
pub const ASCII: u16 = 2;
pub const SHORT: u16 = 3;
pub const LONG: u16 = 4;
pub const RATIONAL: u16 = 5;
pub const UNDEFINED: u16 = 7;

// ── canaries ────────────────────────────────────────────────────────────────

pub const K_MAKE: &[u8] = b"CANARY-MAKE-0001";
pub const K_MODEL: &[u8] = b"CANARY-MODEL-0002";
pub const K_SOFTWARE: &[u8] = b"CANARY-SOFTWARE-0003";
pub const K_DATETIME: &[u8] = b"2031:02:03 04:05:06";
pub const K_ARTIST: &[u8] = b"CANARY-ARTIST-0004";
pub const K_COPYRIGHT: &[u8] = b"CANARY-COPYRIGHT-0005";
pub const K_DESC: &[u8] = b"CANARY-DESCRIPTION-0006";
pub const K_DTO: &[u8] = b"2032:03:04 05:06:07";
pub const K_USERCOMMENT: &[u8] = b"CANARY-USERCOMMENT-0007";
pub const K_BODYSERIAL: &[u8] = b"CANARY-BODYSERIAL-0008";
pub const K_LENS: &[u8] = b"CANARY-LENSMODEL-0009";
pub const K_NOTE: &[u8] = b"CANARY-MAKERNOTE-0010";
pub const K_GPSAREA: &[u8] = b"CANARY-GPSAREA-0011";
pub const K_THUMB: &[u8] = b"CANARY-THUMBNAIL-0012";
pub const K_XMPTAG: &[u8] = b"CANARY-XMP-IN-EXIF-0013";
pub const K_XRES1: &[u8] = &[0x00, 0x00, 0x01, 0x2C, 0x00, 0x00, 0x00, 0x01];
pub const K_SUBARTIST: &[u8] = b"CANARY-SUBARTIST-0014";
pub const K_CHAINARTIST: &[u8] = b"CANARY-CHAINARTIST-0015";
pub const K_UID: &[u8] = b"CANARY-IMAGEUNIQUEID-0016";
pub const K_LENSSERIAL: &[u8] = b"CANARY-LENSSERIAL-0017";
pub const K_CAMSERIAL: &[u8] = b"CANARY-CAMSERIAL-0018";

/// Latitude and longitude rationals: 24 bytes each, recognisable.
pub const K_LAT: [(u32, u32); 3] = [(0x4C41_5431, 1), (0x4C41_5432, 1), (0x4C41_5433, 1)];
pub const K_LON: [(u32, u32); 3] = [(0x4C4F_4E31, 1), (0x4C4F_4E32, 1), (0x4C4F_4E33, 1)];

// ── specs ───────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub enum Val {
    /// The value bytes in file order; inline when at most 4 bytes.
    Data(Vec<u8>),
    /// The offset of the IFD with this name.
    Ifd(&'static str),
    /// A literal 4-byte field (an offset not backed by anything).
    Raw(u32),
    /// The offset of the thumbnail bytes.
    Thumb,
    /// The offset of the thumbnail bytes plus this many (a second image
    /// appended behind the thumbnail).
    ThumbAt(usize),
    /// The offset of out-of-line bytes shared with another entry: the other
    /// entry's `(ifd name, tag)`.
    SameAs(&'static str, u16),
    /// A literal offset field, written as given.
    At(u32),
}

#[derive(Clone, Debug)]
pub struct Ent {
    pub tag: u16,
    pub ty: u16,
    pub count: u32,
    pub val: Val,
    /// The bytes a search for this entry's content would look for.
    pub canary: Option<Vec<u8>>,
}

#[derive(Clone, Debug)]
pub struct Ifd {
    pub name: &'static str,
    pub ents: Vec<Ent>,
    pub next: Option<&'static str>,
}

#[derive(Clone, Debug, Default)]
pub struct Build {
    pub be: bool,
    pub ifds: Vec<Ifd>,
    pub thumb: Option<Vec<u8>>,
    /// Pad the blob with zero bytes up to this length.
    pub pad_to: Option<usize>,
}

fn u16b(be: bool, v: u16) -> [u8; 2] {
    if be {
        v.to_be_bytes()
    } else {
        v.to_le_bytes()
    }
}

fn u32b(be: bool, v: u32) -> [u8; 4] {
    if be {
        v.to_be_bytes()
    } else {
        v.to_le_bytes()
    }
}

pub fn ascii(tag: u16, text: &[u8]) -> Ent {
    let mut d = text.to_vec();
    d.push(0);
    Ent {
        tag,
        ty: ASCII,
        count: d.len() as u32,
        val: Val::Data(d),
        canary: Some(text.to_vec()),
    }
}

pub fn short(be: bool, tag: u16, v: u16) -> Ent {
    Ent {
        tag,
        ty: SHORT,
        count: 1,
        val: Val::Data(u16b(be, v).to_vec()),
        canary: None,
    }
}

pub fn long(be: bool, tag: u16, v: u32) -> Ent {
    Ent {
        tag,
        ty: LONG,
        count: 1,
        val: Val::Data(u32b(be, v).to_vec()),
        canary: None,
    }
}

pub fn rational(be: bool, tag: u16, vs: &[(u32, u32)]) -> Ent {
    let mut d = Vec::new();
    for (n, m) in vs {
        d.extend_from_slice(&u32b(be, *n));
        d.extend_from_slice(&u32b(be, *m));
    }
    Ent {
        tag,
        ty: RATIONAL,
        count: vs.len() as u32,
        val: Val::Data(d),
        canary: None,
    }
}

/// `e` with its value bytes as the canary.
pub fn canaried(mut e: Ent) -> Ent {
    if let Val::Data(d) = &e.val {
        e.canary = Some(d.clone());
    }
    e
}

pub fn undefined(tag: u16, bytes: &[u8]) -> Ent {
    Ent {
        tag,
        ty: UNDEFINED,
        count: bytes.len() as u32,
        val: Val::Data(bytes.to_vec()),
        canary: Some(bytes.to_vec()),
    }
}

pub fn bytes(tag: u16, vals: &[u8]) -> Ent {
    Ent {
        tag,
        ty: BYTE,
        count: vals.len() as u32,
        val: Val::Data(vals.to_vec()),
        canary: None,
    }
}

pub fn ptr(tag: u16, ifd: &'static str) -> Ent {
    Ent {
        tag,
        ty: LONG,
        count: 1,
        val: Val::Ifd(ifd),
        canary: None,
    }
}

pub fn ifd(name: &'static str, ents: Vec<Ent>, next: Option<&'static str>) -> Ifd {
    Ifd { name, ents, next }
}

// ── the layout record ───────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct EntRec {
    pub tag: u16,
    pub ty: u16,
    pub count: u32,
    /// Blob offset of the entry's 12 bytes.
    pub at: usize,
    /// Blob range of the out-of-line value, if it has one.
    pub value: Option<Range<usize>>,
    pub canary: Option<Vec<u8>>,
    /// The IFD this entry points to (a pointer entry).
    pub points_to: Option<&'static str>,
}

#[derive(Clone, Debug)]
pub struct IfdRec {
    pub name: &'static str,
    /// Blob range of the table: count word through next word.
    pub table: Range<usize>,
    pub ents: Vec<EntRec>,
    /// The name of the IFD the next word leads to.
    pub next: Option<&'static str>,
}

#[derive(Clone, Debug)]
pub struct Layout {
    pub be: bool,
    pub bytes: Vec<u8>,
    pub ifds: Vec<IfdRec>,
    pub thumb: Option<Range<usize>>,
}

impl Layout {
    pub fn ifd(&self, name: &str) -> &IfdRec {
        self.ifds.iter().find(|i| i.name == name).unwrap()
    }
    pub fn has(&self, name: &str) -> bool {
        self.ifds.iter().any(|i| i.name == name)
    }
}

impl Build {
    /// Lay the blob out: header, then for each IFD its table and its
    /// out-of-line values (word aligned), then the thumbnail, then padding.
    pub fn layout(&self) -> Layout {
        let be = self.be;
        // Pass 1: positions.
        let mut tables = Vec::new();
        let mut values: Vec<Vec<Option<Range<usize>>>> = Vec::new();
        let mut pos = 8usize;
        for ifd in &self.ifds {
            let table = pos..pos + 2 + 12 * ifd.ents.len() + 4;
            pos = table.end;
            let mut vs = Vec::new();
            for e in &ifd.ents {
                match &e.val {
                    Val::Data(d) if d.len() > 4 => {
                        let r = pos..pos + d.len();
                        pos += d.len() + (d.len() & 1);
                        vs.push(Some(r));
                    }
                    _ => vs.push(None),
                }
            }
            tables.push(table);
            values.push(vs);
        }
        let thumb = self.thumb.as_ref().map(|t| {
            let r = pos..pos + t.len();
            pos += t.len();
            r
        });
        let total = self.pad_to.map_or(pos, |p| p.max(pos));
        let offset_of = |name: &str| -> usize {
            let i = self.ifds.iter().position(|f| f.name == name).unwrap();
            tables[i].start
        };
        // Pass 2: bytes.
        let mut out = vec![0u8; total];
        out[0..2].copy_from_slice(if be { b"MM" } else { b"II" });
        out[2..4].copy_from_slice(&u16b(be, 42));
        out[4..8].copy_from_slice(&u32b(be, 8));
        let mut recs = Vec::new();
        for (i, ifd) in self.ifds.iter().enumerate() {
            let table = tables[i].clone();
            let mut at = table.start;
            out[at..at + 2].copy_from_slice(&u16b(be, ifd.ents.len() as u16));
            at += 2;
            let mut ents = Vec::new();
            for (n, e) in ifd.ents.iter().enumerate() {
                out[at..at + 2].copy_from_slice(&u16b(be, e.tag));
                out[at + 2..at + 4].copy_from_slice(&u16b(be, e.ty));
                out[at + 4..at + 8].copy_from_slice(&u32b(be, e.count));
                let mut points_to = None;
                let mut value = values[i][n].clone();
                match &e.val {
                    Val::Data(d) => match &values[i][n] {
                        Some(r) => {
                            out[at + 8..at + 12].copy_from_slice(&u32b(be, r.start as u32));
                            out[r.clone()].copy_from_slice(d);
                        }
                        None => out[at + 8..at + 8 + d.len()].copy_from_slice(d),
                    },
                    Val::Ifd(name) => {
                        out[at + 8..at + 12].copy_from_slice(&u32b(be, offset_of(name) as u32));
                        points_to = Some(*name);
                    }
                    Val::Raw(v) | Val::At(v) => {
                        out[at + 8..at + 12].copy_from_slice(&u32b(be, *v));
                    }
                    Val::Thumb => {
                        let t = thumb.clone().unwrap();
                        out[at + 8..at + 12].copy_from_slice(&u32b(be, t.start as u32));
                    }
                    Val::ThumbAt(n) => {
                        let t = thumb.clone().unwrap();
                        out[at + 8..at + 12].copy_from_slice(&u32b(be, (t.start + n) as u32));
                    }
                    Val::SameAs(other_ifd, other_tag) => {
                        let oi = self.ifds.iter().position(|f| f.name == *other_ifd).unwrap();
                        let on = self.ifds[oi]
                            .ents
                            .iter()
                            .position(|x| x.tag == *other_tag)
                            .unwrap();
                        let r = values[oi][on].clone().unwrap();
                        out[at + 8..at + 12].copy_from_slice(&u32b(be, r.start as u32));
                        value = Some(r.start..r.start + len_of(e));
                    }
                }
                ents.push(EntRec {
                    tag: e.tag,
                    ty: e.ty,
                    count: e.count,
                    at,
                    value,
                    canary: e.canary.clone(),
                    points_to,
                });
                at += 12;
            }
            let next = ifd.next.map_or(0, |n| offset_of(n) as u32);
            out[at..at + 4].copy_from_slice(&u32b(be, next));
            recs.push(IfdRec {
                name: ifd.name,
                table,
                ents,
                next: ifd.next,
            });
        }
        if let (Some(t), Some(r)) = (&self.thumb, &thumb) {
            out[r.clone()].copy_from_slice(t);
        }
        Layout {
            be,
            bytes: out,
            ifds: recs,
            thumb,
        }
    }
}

fn len_of(e: &Ent) -> usize {
    let size = match e.ty {
        BYTE | ASCII | UNDEFINED => 1,
        SHORT => 2,
        LONG => 4,
        RATIONAL => 8,
        _ => 1,
    };
    size * e.count as usize
}

// ── the independent edit model ─────────────────────────────────────────────

/// The categories, by the names the DEC-008 table uses.
pub type Cat = &'static str;

/// DEC-008's tag table, typed by hand: the category of tag `tag` in an IFD of
/// the given role (`"ifd0"`, `"exif"`, `"gps"`, `"interop"`, `"ifd1"`, other).
pub fn category(role: &str, tag: u16, ifd1_has_jpeg: bool) -> Option<Cat> {
    if role == "gps" {
        return Some("gps");
    }
    if role == "interop" {
        return None;
    }
    if role == "ifd1" && tag == 0x0201 {
        return Some("thumbnail");
    }
    if role == "ifd1" && tag == 0x0111 && !ifd1_has_jpeg {
        return Some("thumbnail");
    }
    match tag {
        0x010E | 0x9286 | 0x9C9B | 0x9C9C | 0x9C9E | 0x9C9F | 0xA436 => Some("free_text"),
        0x010F | 0x0110 | 0xA433 | 0xA434 => Some("camera_model"),
        0x0131 | 0xA439 | 0xA43A | 0xA43B | 0xA43C => Some("software"),
        0x0132 | 0x9003 | 0x9004 | 0x9010 | 0x9011 | 0x9012 | 0x9290 | 0x9291 | 0x9292 => {
            Some("timestamps")
        }
        0x013B | 0x8298 | 0xA430 | 0x9C9D | 0xA437 | 0xA438 => Some("authorship"),
        0x927C => Some("maker_note"),
        0xA420 | 0xA431 | 0xA435 | 0xC62F => Some("device_id"),
        0x02BC => Some("xmp"),
        0x83BB | 0x8649 => Some("iptc"),
        0x8773 => Some("icc"),
        _ => None,
    }
}

/// The role of an IFD name for [`category`].
pub fn role(name: &str) -> &'static str {
    match name {
        "ifd0" => "ifd0",
        "exif" => "exif",
        "gps" => "gps",
        "interop" => "interop",
        "ifd1" => "ifd1",
        _ => "other",
    }
}

/// What an edit removes: `(ifd name, entry index)` pairs, IFDs that go whole,
/// the IFD whose next word is cut, and whether the thumbnail bytes go.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Del {
    pub entries: Vec<(&'static str, usize)>,
    pub whole: Vec<&'static str>,
    pub unlink: Option<&'static str>,
    pub thumb: bool,
}

impl Del {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty() && self.whole.is_empty() && self.unlink.is_none() && !self.thumb
    }
}

/// Rules 2 and 6 to 8 of SPEC-021, applied to a layout and a policy given as
/// category names. Does not check any refusal rule.
pub fn del_for(l: &Layout, spec: &[Cat]) -> Del {
    let has = |c: Cat| spec.contains(&c);
    let mut d = Del::default();
    let ifd1_has_jpeg = l
        .ifds
        .iter()
        .any(|f| role(f.name) == "ifd1" && f.ents.iter().any(|e| e.tag == 0x0201));
    if has("gps") {
        for f in l.ifds.iter().filter(|f| role(f.name) == "gps") {
            d.whole.push(f.name);
            for p in &l.ifds {
                for (n, e) in p.ents.iter().enumerate() {
                    if e.points_to == Some(f.name) {
                        d.entries.push((p.name, n));
                    }
                }
            }
        }
    }
    if has("thumbnail") {
        if let Some(f1) = l.ifds.iter().find(|f| role(f.name) == "ifd1") {
            d.whole.push(f1.name);
            d.unlink = l
                .ifds
                .iter()
                .find(|f| f.next == Some(f1.name))
                .map(|f| f.name);
            d.thumb = true;
        }
    }
    for f in &l.ifds {
        if d.whole.contains(&f.name) {
            continue;
        }
        for (n, e) in f.ents.iter().enumerate() {
            if let Some(c) = category(role(f.name), e.tag, ifd1_has_jpeg) {
                if has(c) && !d.entries.contains(&(f.name, n)) {
                    d.entries.push((f.name, n));
                }
            }
        }
    }
    d.entries.sort();
    d
}

/// The blob after the edit `d`, computed from the layout record: tables of
/// IFDs that lose entries are rewritten (count, survivors, next word, zero
/// tail), removed IFDs' tables, every deleted or removed entry's out-of-line
/// value and the thumbnail are zero, a cut next word is zero.
pub fn expected(l: &Layout, d: &Del) -> Vec<u8> {
    let be = l.be;
    let mut out = l.bytes.clone();
    let zero = |out: &mut Vec<u8>, r: Range<usize>| {
        for b in &mut out[r] {
            *b = 0;
        }
    };
    for f in &l.ifds {
        let gone: Vec<usize> = d
            .entries
            .iter()
            .filter(|(n, _)| *n == f.name)
            .map(|(_, i)| *i)
            .collect();
        if d.whole.contains(&f.name) {
            zero(&mut out, f.table.clone());
            for e in &f.ents {
                if let Some(v) = &e.value {
                    zero(&mut out, v.clone());
                }
            }
            continue;
        }
        let cut = d.unlink == Some(f.name);
        if gone.is_empty() && !cut {
            continue;
        }
        let mut table = Vec::new();
        let kept: Vec<&EntRec> = f
            .ents
            .iter()
            .enumerate()
            .filter(|(i, _)| !gone.contains(i))
            .map(|(_, e)| e)
            .collect();
        table.extend_from_slice(&u16b(be, kept.len() as u16));
        for e in &kept {
            table.extend_from_slice(&l.bytes[e.at..e.at + 12]);
        }
        let next_at = f.table.end - 4;
        if cut {
            table.extend_from_slice(&[0, 0, 0, 0]);
        } else {
            table.extend_from_slice(&l.bytes[next_at..next_at + 4]);
        }
        table.extend_from_slice(&vec![0u8; 12 * gone.len()]);
        assert_eq!(table.len(), f.table.len());
        out[f.table.clone()].copy_from_slice(&table);
        for i in gone {
            if let Some(v) = &f.ents[i].value {
                zero(&mut out, v.clone());
            }
        }
    }
    if d.thumb {
        if let Some(t) = &l.thumb {
            zero(&mut out, t.clone());
        }
    }
    out
}

/// Every deleted entry as `(ifd name, tag)`, in blob order, whole IFDs' entries
/// included.
pub fn deleted_entries(l: &Layout, d: &Del) -> Vec<(&'static str, u16, usize)> {
    let mut v = Vec::new();
    for f in &l.ifds {
        for (i, e) in f.ents.iter().enumerate() {
            if d.whole.contains(&f.name) || d.entries.contains(&(f.name, i)) {
                v.push((f.name, e.tag, e.at));
            }
        }
    }
    v.sort_by_key(|x| x.2);
    v
}

// ── fixtures ────────────────────────────────────────────────────────────────

pub fn thumb_jpeg() -> Vec<u8> {
    [&[0xFF, 0xD8][..], K_THUMB, &[0xFF, 0xD9][..]].concat()
}

/// A MakerNote body: 64 bytes, the canary, then a few absolute-looking words.
pub fn note_body(be: bool) -> Vec<u8> {
    let mut v = K_NOTE.to_vec();
    v.resize(48, b'.');
    for i in 0..4u32 {
        v.extend_from_slice(&u32b(be, 0x100 + i));
    }
    assert_eq!(v.len(), 64);
    v
}

/// The full Exif: IFD0, Exif IFD, Interop, GPS and an IFD1 with a thumbnail.
pub fn full(be: bool) -> Build {
    let ifd0 = ifd(
        "ifd0",
        vec![
            ascii(0x010F, K_MAKE),
            ascii(0x0110, K_MODEL),
            ascii(0x0131, K_SOFTWARE),
            ascii(0x0132, K_DATETIME),
            ascii(0x013B, K_ARTIST),
            ascii(0x8298, K_COPYRIGHT),
            ascii(0x010E, K_DESC),
            short(be, 0x0112, 6),
            rational(be, 0x011A, &[(300, 1)]),
            ascii(0xC62F, K_CAMSERIAL),
            ptr(0x8769, "exif"),
            ptr(0x8825, "gps"),
        ],
        Some("ifd1"),
    );
    let exif = ifd(
        "exif",
        vec![
            ascii(0x9003, K_DTO),
            rational(be, 0x829A, &[(1, 250)]),
            undefined(0x9286, &[&b"ASCII\0\0\0"[..], K_USERCOMMENT].concat()),
            ascii(0xA431, K_BODYSERIAL),
            ascii(0xA420, K_UID),
            ascii(0xA435, K_LENSSERIAL),
            ascii(0xA434, K_LENS),
            undefined(0x927C, &note_body(be)),
            short(be, 0xA001, 1),
            ptr(0xA005, "interop"),
        ],
        None,
    );
    let interop = ifd(
        "interop",
        vec![Ent {
            tag: 0x0001,
            ty: ASCII,
            count: 4,
            val: Val::Data(b"R98\0".to_vec()),
            canary: None,
        }],
        None,
    );
    let gps = ifd(
        "gps",
        vec![
            bytes(0x0000, &[2, 3, 0, 0]),
            Ent {
                tag: 0x0001,
                ty: ASCII,
                count: 2,
                val: Val::Data(b"N\0".to_vec()),
                canary: None,
            },
            canaried(rational(be, 0x0002, &K_LAT)),
            Ent {
                tag: 0x0003,
                ty: ASCII,
                count: 2,
                val: Val::Data(b"E\0".to_vec()),
                canary: None,
            },
            canaried(rational(be, 0x0004, &K_LON)),
            undefined(0x001C, &[&b"ASCII\0\0\0"[..], K_GPSAREA].concat()),
        ],
        None,
    );
    let ifd1 = ifd(
        "ifd1",
        vec![
            short(be, 0x0103, 6),
            Ent {
                tag: 0x0201,
                ty: LONG,
                count: 1,
                val: Val::Thumb,
                canary: None,
            },
            long(be, 0x0202, thumb_jpeg().len() as u32),
            rational(be, 0x011A, &[(300, 1)]),
        ],
        None,
    );
    Build {
        be,
        ifds: vec![ifd0, exif, interop, gps, ifd1],
        thumb: Some(thumb_jpeg()),
        pad_to: None,
    }
}

pub fn full_le() -> Build {
    full(false)
}

pub fn full_be() -> Build {
    full(true)
}

fn only_ifd0(be: bool, ents: Vec<Ent>) -> Build {
    Build {
        be,
        ifds: vec![ifd("ifd0", ents, None)],
        thumb: None,
        pad_to: None,
    }
}

/// The first entry of IFD0 is a target for `CameraModel` and nothing else is.
pub fn first_entry_target() -> Build {
    only_ifd0(
        false,
        vec![
            ascii(0x010F, K_MAKE),
            short(false, 0x0112, 6),
            short(false, 0x0128, 2),
        ],
    )
}

/// The last entry of IFD0 is a target for `Authorship`.
pub fn last_entry_target() -> Build {
    only_ifd0(
        true,
        vec![
            short(true, 0x0112, 6),
            short(true, 0x0128, 2),
            ascii(0x013B, K_ARTIST),
        ],
    )
}

/// Every entry of IFD0 is a target for `AllPii`: the IFD empties.
pub fn every_entry_target() -> Build {
    only_ifd0(
        false,
        vec![
            ascii(0x010F, K_MAKE),
            ascii(0x0110, K_MODEL),
            ascii(0x013B, K_ARTIST),
        ],
    )
}

/// No GPS IFD and no thumbnail.
pub fn no_gps_no_thumb() -> Build {
    Build {
        be: false,
        ifds: vec![
            ifd(
                "ifd0",
                vec![
                    ascii(0x010F, K_MAKE),
                    short(false, 0x0112, 6),
                    ptr(0x8769, "exif"),
                ],
                None,
            ),
            ifd(
                "exif",
                vec![ascii(0x9003, K_DTO), ascii(0xA431, K_BODYSERIAL)],
                None,
            ),
        ],
        thumb: None,
        pad_to: None,
    }
}

/// A SubIFD (tag 0x014A) with a targeted tag.
pub fn subifd() -> Build {
    Build {
        be: false,
        ifds: vec![
            ifd(
                "ifd0",
                vec![
                    ascii(0x010F, K_MAKE),
                    short(false, 0x0112, 6),
                    ptr(0x014A, "sub"),
                ],
                None,
            ),
            ifd(
                "sub",
                vec![ascii(0x013B, K_SUBARTIST), short(false, 0x0100, 8)],
                None,
            ),
        ],
        thumb: None,
        pad_to: None,
    }
}

/// IFD0, IFD1 (thumbnail) and a third IFD in the chain, with a targeted tag in
/// the third.
pub fn chain_ifd2() -> Build {
    let be = false;
    Build {
        be,
        ifds: vec![
            ifd(
                "ifd0",
                vec![ascii(0x010F, K_MAKE), short(be, 0x0112, 6)],
                Some("ifd1"),
            ),
            ifd(
                "ifd1",
                vec![
                    Ent {
                        tag: 0x0201,
                        ty: LONG,
                        count: 1,
                        val: Val::Thumb,
                        canary: None,
                    },
                    long(be, 0x0202, thumb_jpeg().len() as u32),
                ],
                Some("ifd2"),
            ),
            ifd(
                "ifd2",
                vec![ascii(0x013B, K_CHAINARTIST), short(be, 0x0112, 1)],
                None,
            ),
        ],
        thumb: Some(thumb_jpeg()),
        pad_to: None,
    }
}

/// A tag 0x02BC (XMP) inside IFD0.
pub fn xmp_tag_inside() -> Build {
    only_ifd0(
        false,
        vec![
            ascii(0x010F, K_MAKE),
            short(false, 0x0112, 6),
            Ent {
                tag: 0x02BC,
                ty: BYTE,
                count: K_XMPTAG.len() as u32,
                val: Val::Data(K_XMPTAG.to_vec()),
                canary: Some(K_XMPTAG.to_vec()),
            },
        ],
    )
}

/// A Canon-style MakerNote kept under every policy that does not name it: its
/// own entries hold absolute offsets into the blob.
pub fn makernote_canon_style() -> Build {
    let mut b = full(false);
    // The MakerNote body already holds absolute-looking offsets (`note_body`);
    // here they point at real places in the blob, so a move would show.
    for f in &mut b.ifds {
        for e in &mut f.ents {
            if e.tag == 0x927C {
                let mut v = K_NOTE.to_vec();
                v.resize(48, b'.');
                for i in 0..4u32 {
                    v.extend_from_slice(&(8 + 2 * i).to_le_bytes());
                }
                e.val = Val::Data(v.clone());
                e.canary = Some(v);
            }
        }
    }
    b
}

/// A block at the segment ceiling: the largest Exif segment (length word
/// 0xFFFF) holds this blob, so nothing may grow.
pub const CEILING_BLOB: usize = 0xFFFF - 2 - 6;

pub fn at_the_ceiling() -> Build {
    let mut b = full(false);
    b.pad_to = Some(CEILING_BLOB);
    b
}

/// Two IFD0-only blobs for the two-block fixture: the first holds a target,
/// the second does not.
pub fn two_blocks() -> (Build, Build) {
    (
        only_ifd0(false, vec![ascii(0x010F, K_MAKE), short(false, 0x0112, 6)]),
        only_ifd0(
            false,
            vec![short(false, 0x0112, 8), short(false, 0x0128, 2)],
        ),
    )
}

// ── leak inputs ─────────────────────────────────────────────────────────────

pub const K_THUMB2: &[u8] = b"CANARY-THUMB2-0019";
pub const K_STRIPS: &[u8] = b"CANARY-STRIP-DATA-0020";

/// An Artist (a deleted entry under `Authorship`) whose 40-byte value starts
/// at the Make's value and runs past the end of the blob.
pub fn straddling_artist(be: bool) -> Build {
    let mut b = only_ifd0(be, vec![ascii(0x010F, K_MAKE), short(be, 0x0112, 1)]);
    b.ifds[0].ents.push(Ent {
        tag: 0x013B,
        ty: ASCII,
        count: 40,
        val: Val::SameAs("ifd0", 0x010F),
        canary: Some(K_MAKE.to_vec()),
    });
    b
}

/// A GPS entry (removed with its IFD) whose 60-byte value starts at the
/// latitude and runs past the end of the blob.
pub fn straddling_gps_value(be: bool) -> Build {
    let mut b = full(be);
    b.ifds.retain(|f| f.name != "ifd1" && f.name != "interop");
    b.ifds[0].next = None;
    b.ifds[0].ents.retain(|e| e.tag != 0x8769);
    b.ifds.retain(|f| f.name != "exif");
    b.thumb = None;
    for e in &mut b.ifds[0].ents {
        e.canary = None;
    }
    let gps = b.ifds.iter_mut().find(|f| f.name == "gps").unwrap();
    gps.ents.retain(|e| e.tag != 0x001C);
    gps.ents.push(Ent {
        tag: 0x001C,
        ty: UNDEFINED,
        count: 60,
        val: Val::SameAs("gps", 0x0002),
        canary: Some(K_LAT[0].0.to_be_bytes().to_vec()),
    });
    b
}

/// IFD1 names a second thumbnail with a second 0x0201/0x0202 pair; only the
/// first pair is the audit's thumbnail.
pub fn thumbnail_second_pair(be: bool) -> Build {
    let mut b = full(be);
    let second = [&[0xFF, 0xD8][..], K_THUMB2, &[0xFF, 0xD9][..]].concat();
    let first = thumb_jpeg().len();
    let ifd1 = b.ifds.iter_mut().find(|f| f.name == "ifd1").unwrap();
    ifd1.ents.push(Ent {
        tag: 0x0201,
        ty: LONG,
        count: 1,
        val: Val::ThumbAt(first),
        canary: Some(K_THUMB2.to_vec()),
    });
    ifd1.ents.push(long(be, 0x0202, second.len() as u32));
    b.thumb = Some([thumb_jpeg(), second].concat());
    b
}

/// IFD1 names its JPEG and also strips (0x0111, 0x0117) with image data
/// behind the thumbnail.
pub fn thumbnail_with_strips(be: bool) -> Build {
    let mut b = full(be);
    let first = thumb_jpeg().len();
    let ifd1 = b.ifds.iter_mut().find(|f| f.name == "ifd1").unwrap();
    ifd1.ents.push(Ent {
        tag: 0x0111,
        ty: LONG,
        count: 1,
        val: Val::ThumbAt(first),
        canary: Some(K_STRIPS.to_vec()),
    });
    ifd1.ents.push(long(be, 0x0117, K_STRIPS.len() as u32));
    b.thumb = Some([thumb_jpeg(), K_STRIPS.to_vec()].concat());
    b
}

pub const K_HIDDEN: &[u8] = b"CANARY-HIDDEN-DATA-0021";

/// IFD1 holds a tag that carries an offset to data nothing else references
/// (tiles, JPEG tables, free blocks, or an unknown tag).
pub fn thumbnail_hidden_offset(be: bool, tag: u16) -> Build {
    let mut b = full(be);
    let first = thumb_jpeg().len();
    let ifd1 = b.ifds.iter_mut().find(|f| f.name == "ifd1").unwrap();
    ifd1.ents.push(Ent {
        tag,
        ty: LONG,
        count: 1,
        val: Val::ThumbAt(first),
        canary: Some(K_HIDDEN.to_vec()),
    });
    b.thumb = Some([thumb_jpeg(), K_HIDDEN.to_vec()].concat());
    b
}

/// The GPS IFD holds a tag outside 0x0000..=0x001F that points at data.
pub fn gps_hidden_offset(be: bool) -> Build {
    let mut b = full(be);
    let first = thumb_jpeg().len();
    let gps = b.ifds.iter_mut().find(|f| f.name == "gps").unwrap();
    gps.ents.push(Ent {
        tag: 0xF000,
        ty: LONG,
        count: 1,
        val: Val::ThumbAt(first),
        canary: Some(K_HIDDEN.to_vec()),
    });
    b.thumb = Some([thumb_jpeg(), K_HIDDEN.to_vec()].concat());
    b
}

/// IFD1 with two 0x0201 and one 0x0202.
pub fn thumbnail_two_offsets(be: bool) -> Build {
    let mut b = full(be);
    let ifd1 = b.ifds.iter_mut().find(|f| f.name == "ifd1").unwrap();
    ifd1.ents.push(Ent {
        tag: 0x0201,
        ty: LONG,
        count: 1,
        val: Val::Thumb,
        canary: None,
    });
    b
}

/// A KEPT Make whose 40-byte value starts at 60 and runs past the end of an
/// 80-byte blob, and a deleted Artist whose 10 bytes sit at 60.
pub fn kept_straddling_value(be: bool) -> Build {
    let mut b = only_ifd0(
        be,
        vec![
            short(be, 0x0112, 1),
            Ent {
                tag: 0x010F,
                ty: ASCII,
                count: 40,
                val: Val::At(60),
                canary: None,
            },
            Ent {
                tag: 0x013B,
                ty: ASCII,
                count: 10,
                val: Val::At(60),
                canary: None,
            },
        ],
    );
    b.pad_to = Some(80);
    b
}

/// An entry of field type IFD (13) whose offset leads to data nothing else
/// references, in IFD1 (tag 0x0100), the GPS IFD (tag 0x001F) or IFD0 (tag
/// 0x013B, Authorship). `at`: "ifd1", "gps" or "ifd0".
pub fn typed_offset_entry(be: bool, at: &str) -> Build {
    let mut b = full(be);
    let first = thumb_jpeg().len();
    let (name, tag) = match at {
        "ifd1" => ("ifd1", 0x0100),
        "gps" => ("gps", 0x001F),
        _ => ("ifd0", 0x013B),
    };
    let f = b.ifds.iter_mut().find(|f| f.name == name).unwrap();
    if name == "ifd0" {
        f.ents.retain(|e| e.tag != 0x013B);
    }
    f.ents.push(Ent {
        tag,
        ty: 13,
        count: 1,
        val: Val::ThumbAt(first),
        canary: Some(K_HIDDEN.to_vec()),
    });
    b.thumb = Some([thumb_jpeg(), K_HIDDEN.to_vec()].concat());
    b
}

/// A KEPT strip pair in IFD0 whose target data also holds a deleted Artist's
/// value.
pub fn kept_strip_target_overlapped(be: bool) -> Build {
    let mut b = full(be);
    let first = thumb_jpeg().len();
    b.ifds[0].ents.retain(|e| e.tag != 0x013B);
    b.ifds[0].ents.push(Ent {
        tag: 0x0111,
        ty: LONG,
        count: 1,
        val: Val::ThumbAt(first),
        canary: None,
    });
    b.ifds[0].ents.push(long(be, 0x0117, K_HIDDEN.len() as u32));
    b.ifds[0].ents.push(Ent {
        tag: 0x013B,
        ty: ASCII,
        count: 10,
        val: Val::ThumbAt(first + 2),
        canary: Some(K_HIDDEN.to_vec()),
    });
    b.thumb = Some([thumb_jpeg(), K_HIDDEN.to_vec()].concat());
    b
}

/// IFD1 holds a Software tag (a category `Custom{Thumbnail}` keeps).
pub fn thumbnail_ifd1_with_a_kept_category_tag(be: bool) -> Build {
    let mut b = full(be);
    let f = b.ifds.iter_mut().find(|f| f.name == "ifd1").unwrap();
    f.ents.push(ascii(0x0131, K_SOFTWARE));
    b
}

/// The GPS IFD is pointed to by two entries of IFD0.
pub fn gps_ifd_pointed_to_twice() -> Build {
    let mut b = full(false);
    b.ifds[0].ents.push(ptr(0x8825, "gps"));
    b
}

/// The first GPS pointer is 0 (nothing there) and a second entry with the tag
/// leads to the GPS IFD, which the walk never visits.
pub fn gps_pointer_hidden_behind_a_zero() -> Build {
    let mut b = full(false);
    let at = b.ifds[0].ents.iter().position(|e| e.tag == 0x8825).unwrap();
    b.ifds[0].ents[at].val = Val::Raw(0);
    b.ifds[0].ents.push(ptr(0x8825, "gps"));
    b
}

/// A deleted entry's value is the same bytes as a kept entry's.
pub fn value_shared_with_kept_entry() -> Build {
    let mut b = full(false);
    // Model (camera_model, deleted under CameraModel) shares Software's value
    // (kept): the Software ASCII has the canary; Model's count is the same.
    let sw = b.ifds[0].ents.iter().position(|e| e.tag == 0x0131).unwrap();
    let md = b.ifds[0].ents.iter().position(|e| e.tag == 0x0110).unwrap();
    let count = b.ifds[0].ents[sw].count;
    b.ifds[0].ents[md] = Ent {
        tag: 0x0110,
        ty: ASCII,
        count,
        val: Val::SameAs("ifd0", 0x0131),
        canary: None,
    };
    b
}

/// A deleted entry's value lies inside a kept IFD's table (the Exif IFD's).
pub fn value_inside_kept_ifd_table() -> Build {
    let mut b = full(false);
    let md = b.ifds[0].ents.iter().position(|e| e.tag == 0x0110).unwrap();
    // Offset 8 + IFD0's table + 14 lands inside IFD0's own entries.
    b.ifds[0].ents[md] = Ent {
        tag: 0x0110,
        ty: ASCII,
        count: 8,
        val: Val::At(40),
        canary: None,
    };
    b
}

/// A deleted entry's value overlaps the TIFF header.
pub fn value_overlaps_tiff_header() -> Build {
    let mut b = full(false);
    let md = b.ifds[0].ents.iter().position(|e| e.tag == 0x0110).unwrap();
    b.ifds[0].ents[md] = Ent {
        tag: 0x0110,
        ty: ASCII,
        count: 8,
        val: Val::At(4),
        canary: None,
    };
    b
}

/// The thumbnail range covers the MakerNote value.
pub fn thumbnail_range_overlaps_makernote() -> Build {
    let mut b = full(false);
    // Point IFD1's JPEGInterchangeFormat at the MakerNote's first byte.
    let l = b.layout();
    let note = l
        .ifd("exif")
        .ents
        .iter()
        .find(|e| e.tag == 0x927C)
        .unwrap()
        .value
        .clone()
        .unwrap();
    let i1 = b.ifds.iter().position(|f| f.name == "ifd1").unwrap();
    for e in &mut b.ifds[i1].ents {
        if e.tag == 0x0201 {
            e.val = Val::At(note.start as u32);
        }
        if e.tag == 0x0202 {
            e.val = Val::Data(u32b(false, 40).to_vec());
        }
    }
    b
}

/// A strips-form IFD1: StripOffsets and StripByteCounts, no JPEG pointer.
pub fn thumbnail_strips_form() -> Build {
    let mut b = full(false);
    let i1 = b.ifds.iter().position(|f| f.name == "ifd1").unwrap();
    b.ifds[i1].ents = vec![
        short(false, 0x0103, 1),
        long(false, 0x0111, 0),
        long(false, 0x0117, 12),
    ];
    b
}

/// IFD1 is followed by a third IFD (the `chain_ifd2` shape).
pub fn thumbnail_with_ifd2_after_it() -> Build {
    chain_ifd2()
}

/// An issue in an IFD that has no target: the Interop pointer leads outside
/// the blob.
pub fn walk_issue_in_a_kept_ifd() -> Build {
    let mut b = full(false);
    for f in &mut b.ifds {
        for e in &mut f.ents {
            if e.tag == 0xA005 {
                e.val = Val::At(0x00FF_0000);
            }
        }
    }
    b.ifds.retain(|f| f.name != "interop");
    b
}

/// IFD0's next word leads outside the blob.
pub fn bad_ifd1_pointer() -> Layout {
    let mut l = full(false).layout();
    let at = l.ifd("ifd0").table.end - 4;
    l.bytes[at..at + 4].copy_from_slice(&0x00FF_0000u32.to_le_bytes());
    l
}

/// IFD0's Exif pointer leads outside the blob.
pub fn bad_exif_pointer() -> Layout {
    let mut l = full(false).layout();
    let e = l
        .ifd("ifd0")
        .ents
        .iter()
        .find(|e| e.tag == 0x8769)
        .unwrap()
        .at;
    l.bytes[e + 8..e + 12].copy_from_slice(&0x00FF_0000u32.to_le_bytes());
    l
}

/// Every entry of IFD0 is a target for the policy (`every_entry_target`).
pub fn ifd0_emptied() -> Build {
    every_entry_target()
}

// ── HANDOFF-045 (round 4) ───────────────────────────────────────────────────

/// IFD0 holds `tag` (an IFD pointer tag ExifTool opens and the walk does not),
/// leading to an IFD that holds a pointer to a GPS IFD with canaried values
/// (`behind == "gps"`) or an Artist (`behind == "artist"`).
pub fn exiftool_subdirectory(be: bool, tag: u16, behind: &str) -> Build {
    let ifd0 = ifd("ifd0", vec![short(be, 0x0112, 1), ptr(tag, "hidden")], None);
    let mut ifds = vec![ifd0];
    if behind == "gps" {
        ifds.push(ifd("hidden", vec![ptr(0x8825, "gps")], None));
        ifds.push(ifd(
            "gps",
            vec![
                Ent {
                    tag: 0x0001,
                    ty: ASCII,
                    count: 2,
                    val: Val::Data(b"N\0".to_vec()),
                    canary: None,
                },
                canaried(rational(be, 0x0002, &K_LAT)),
            ],
            None,
        ));
    } else {
        ifds.push(ifd("hidden", vec![ascii(0x013B, K_SUBARTIST)], None));
    }
    Build {
        be,
        ifds,
        thumb: None,
        pad_to: None,
    }
}

/// The Interop IFD holds an Artist (a category tag the audit does not report
/// there, DEC-008).
pub fn interop_with_a_category_tag(be: bool) -> Build {
    let mut b = full(be);
    let f = b.ifds.iter_mut().find(|f| f.name == "interop").unwrap();
    f.ents.push(ascii(0x013B, K_SUBARTIST));
    b
}

/// The GPS IFD's next-IFD word leads to another IFD with a canaried value.
pub fn gps_with_a_next_word(be: bool) -> Build {
    let mut b = full(be);
    let f = b.ifds.iter_mut().find(|f| f.name == "gps").unwrap();
    f.next = Some("tail");
    b.ifds.push(ifd(
        "tail",
        vec![canaried(rational(be, 0x0002, &K_LON))],
        None,
    ));
    b
}
