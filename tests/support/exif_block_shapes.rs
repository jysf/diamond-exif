//! Fixtures for the Exif block builder (SPEC-025, AC10): TIFF sources built
//! from a declarative list of IFDs by an encoder that imports nothing from
//! `src/`. Every shape is produced in both byte orders ([`grid`]) and says
//! what the builder must do with it ([`Expect`]).
//!
//! The grid holds the shapes this file can build. The test adds two more it
//! cannot: `tiff_shapes::dng(be)` (its GPS is inside the raw SubIFD, so it is
//! not in the block) and a JPEG's Exif TIFF blob (`strip_shapes::full_tiff`).
//! Hostile layouts are expressed with [`V`]: a value at an odd source offset
//! ([`V::ValOdd`]), two entries sharing one value ([`V::SameAs`]), a value that
//! lies over an IFD table ([`V::AtTable`]), a pointer past the end
//! ([`V::Abs`]).

#![allow(dead_code)]

pub const BYTE: u16 = 1;
pub const ASCII: u16 = 2;
pub const SHORT: u16 = 3;
pub const LONG: u16 = 4;
pub const RATIONAL: u16 = 5;
pub const UNDEFINED: u16 = 7;
pub const SLONG: u16 = 9;
pub const SRATIONAL: u16 = 10;
pub const IFD: u16 = 13;

/// What the builder must do with a shape.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expect {
    /// A block (possibly empty).
    Block,
    /// A typed refusal. The key names it: `source`, `walk-halted`,
    /// `broken-pointer:<tag>`, `duplicate-pointer:<tag>`,
    /// `duplicate-tag:<ifd>:<tag>`, `unreadable:<ifd>:<tag>`, with the tag as
    /// `0x` and four lower-case hex digits and the IFD as `exif`, `interop` or
    /// `gps`.
    Refuse(String),
}

#[derive(Clone, Debug)]
pub struct Shape {
    pub name: String,
    pub bytes: Vec<u8>,
    pub expect: Expect,
}

/// Where an entry's 4-byte value field gets its content.
#[derive(Clone, Debug)]
pub enum V {
    /// Encoded payload in the file's byte order: inline when at most 4 bytes,
    /// else out of line at the next even offset after the table.
    Val(Vec<u8>),
    /// Out of line at an **odd** offset (the payload must exceed 4 bytes).
    ValOdd(Vec<u8>),
    /// The 4 field bytes exactly as given (unused inline bytes can be garbage).
    Raw([u8; 4]),
    /// A LONG offset to the table of IFD `n` of the document.
    Child(usize),
    /// The same out-of-line offset as entry `.1` of IFD `.0`.
    SameAs(usize, usize),
    /// An offset to the table of IFD `n`, as a value (an overlap).
    AtTable(usize),
    /// A literal offset.
    Abs(u32),
}

#[derive(Clone, Debug)]
pub struct E {
    pub tag: u16,
    pub ty: u16,
    pub count: u32,
    pub v: V,
}

#[derive(Clone, Debug)]
pub enum Next {
    Zero,
    Ifd(usize),
    Abs(u32),
}

#[derive(Clone, Debug)]
pub struct IfdSpec {
    pub ents: Vec<E>,
    pub next: Next,
}

pub fn ifd(ents: Vec<E>) -> IfdSpec {
    IfdSpec {
        ents,
        next: Next::Zero,
    }
}

pub fn ifd_next(ents: Vec<E>, next: Next) -> IfdSpec {
    IfdSpec { ents, next }
}

/// Entry constructors that need no byte order.
pub fn ptr(tag: u16, ifd: usize) -> E {
    E {
        tag,
        ty: LONG,
        count: 1,
        v: V::Child(ifd),
    }
}

pub fn ptr_ty(tag: u16, ty: u16, ifd: usize) -> E {
    E {
        tag,
        ty,
        count: 1,
        v: V::Child(ifd),
    }
}

pub fn abs(tag: u16, ty: u16, count: u32, at: u32) -> E {
    E {
        tag,
        ty,
        count,
        v: V::Abs(at),
    }
}

pub fn raw(tag: u16, ty: u16, count: u32, field: [u8; 4]) -> E {
    E {
        tag,
        ty,
        count,
        v: V::Raw(field),
    }
}

/// Byte-order-aware entry constructors.
#[derive(Clone, Copy)]
pub struct Enc {
    pub be: bool,
}

impl Enc {
    pub fn w16(self, v: u16) -> [u8; 2] {
        if self.be {
            v.to_be_bytes()
        } else {
            v.to_le_bytes()
        }
    }
    pub fn w32(self, v: u32) -> [u8; 4] {
        if self.be {
            v.to_be_bytes()
        } else {
            v.to_le_bytes()
        }
    }
    pub fn ent(self, tag: u16, ty: u16, count: u32, bytes: Vec<u8>) -> E {
        E {
            tag,
            ty,
            count,
            v: V::Val(bytes),
        }
    }
    pub fn short(self, tag: u16, v: u16) -> E {
        self.ent(tag, SHORT, 1, self.w16(v).to_vec())
    }
    pub fn long(self, tag: u16, v: u32) -> E {
        self.ent(tag, LONG, 1, self.w32(v).to_vec())
    }
    pub fn rat(self, tag: u16, vals: &[(u32, u32)]) -> E {
        let bytes = vals
            .iter()
            .flat_map(|(n, d)| [self.w32(*n), self.w32(*d)].concat())
            .collect();
        self.ent(tag, RATIONAL, vals.len() as u32, bytes)
    }
    pub fn srat(self, tag: u16, vals: &[(i32, i32)]) -> E {
        let bytes = vals
            .iter()
            .flat_map(|(n, d)| [self.w32(*n as u32), self.w32(*d as u32)].concat())
            .collect();
        self.ent(tag, SRATIONAL, vals.len() as u32, bytes)
    }
    /// NUL-terminated text.
    pub fn ascii(self, tag: u16, text: &str) -> E {
        let mut bytes = text.as_bytes().to_vec();
        bytes.push(0);
        self.ent(tag, ASCII, bytes.len() as u32, bytes)
    }
    pub fn undef(self, tag: u16, bytes: &[u8]) -> E {
        self.ent(tag, UNDEFINED, bytes.len() as u32, bytes.to_vec())
    }
    pub fn slong(self, tag: u16, v: i32) -> E {
        self.ent(tag, SLONG, 1, self.w32(v as u32).to_vec())
    }
}

/// Lay the document out: header, then each IFD's table followed by its
/// out-of-line values, in order. IFD 0 is IFD0.
pub fn build(be: bool, ifds: &[IfdSpec]) -> Vec<u8> {
    let enc = Enc { be };
    let n = ifds.len();
    let mut at = vec![0usize; n];
    let mut voff: Vec<Vec<usize>> = Vec::new();
    let mut pos = 8usize;
    for (i, spec) in ifds.iter().enumerate() {
        if pos % 2 == 1 {
            pos += 1;
        }
        at[i] = pos;
        pos += 2 + 12 * spec.ents.len() + 4;
        let mut offs = vec![0usize; spec.ents.len()];
        for (j, ent) in spec.ents.iter().enumerate() {
            match &ent.v {
                V::Val(bytes) if bytes.len() > 4 => {
                    if pos % 2 == 1 {
                        pos += 1;
                    }
                    offs[j] = pos;
                    pos += bytes.len();
                }
                V::ValOdd(bytes) => {
                    if pos & 1 == 0 {
                        pos += 1;
                    }
                    offs[j] = pos;
                    pos += bytes.len();
                }
                _ => {}
            }
        }
        voff.push(offs);
    }
    let mut buf = vec![0u8; pos];
    buf[0..4].copy_from_slice(if be { b"MM\0\x2A" } else { b"II\x2A\0" });
    if n > 0 {
        buf[4..8].copy_from_slice(&enc.w32(at[0] as u32));
    }
    for (i, spec) in ifds.iter().enumerate() {
        let t = at[i];
        buf[t..t + 2].copy_from_slice(&enc.w16(spec.ents.len() as u16));
        for (j, ent) in spec.ents.iter().enumerate() {
            let e0 = t + 2 + 12 * j;
            buf[e0..e0 + 2].copy_from_slice(&enc.w16(ent.tag));
            buf[e0 + 2..e0 + 4].copy_from_slice(&enc.w16(ent.ty));
            buf[e0 + 4..e0 + 8].copy_from_slice(&enc.w32(ent.count));
            let field: [u8; 4] = match &ent.v {
                V::Raw(f) => *f,
                V::Child(k) | V::AtTable(k) => enc.w32(at[*k] as u32),
                V::Abs(a) => enc.w32(*a),
                V::SameAs(a, b) => enc.w32(voff[*a][*b] as u32),
                V::Val(bytes) if bytes.len() <= 4 => {
                    let mut f = [0u8; 4];
                    f[..bytes.len()].copy_from_slice(bytes);
                    f
                }
                V::Val(bytes) | V::ValOdd(bytes) => {
                    let o = voff[i][j];
                    buf[o..o + bytes.len()].copy_from_slice(bytes);
                    enc.w32(o as u32)
                }
            };
            buf[e0 + 8..e0 + 12].copy_from_slice(&field);
        }
        let next = match &spec.next {
            Next::Zero => 0,
            Next::Ifd(k) => at[*k] as u32,
            Next::Abs(a) => *a,
        };
        let nx = t + 2 + 12 * spec.ents.len();
        buf[nx..nx + 4].copy_from_slice(&enc.w32(next));
    }
    buf
}

fn tag_key(tag: u16) -> String {
    format!("0x{tag:04x}")
}

fn shape(name: &str, be: bool, ifds: &[IfdSpec], expect: Expect) -> Shape {
    Shape {
        name: format!("{name}/{}", if be { "be" } else { "le" }),
        bytes: build(be, ifds),
        expect,
    }
}

fn refuse(key: &str) -> Expect {
    Expect::Refuse(key.to_string())
}

/// The empty IFD used as the target of a pointer tag.
fn empty() -> IfdSpec {
    ifd(vec![])
}

/// The offset-carrying tags of DEC-014's amendment, measured against ExifTool
/// 13.55, and the conservative extras. Typed again here by hand (a mirror of
/// the decision table, checked by `omission_table_mirrors_agree`).
pub const MEASURED_OFFSET_TAGS: [u16; 11] = [
    0x0111, 0x0120, 0x0144, 0x0201, 0x0207, 0x0208, 0x0209, 0x8781, 0xA010, 0xBCC0, 0xBCC2,
];
pub const POINTER_OFFSET_TAGS: [u16; 3] = [0x014A, 0x8769, 0x8825];
pub const CONSERVATIVE_TAGS: [u16; 8] = [
    0x0190, 0x888A, 0x8290, 0xFE00, 0xC51B, 0x8568, 0xCD41, 0xC6F5,
];
pub const LENGTH_PARTNERS: [u16; 7] = [0x0117, 0x0121, 0x0145, 0x0202, 0xA011, 0xBCC1, 0xBCC3];

/// IFD0 with `ents` in one of the three block IFDs: `which` is `exif`,
/// `interop` or `gps`. Returns the document's IFDs (the last is an empty IFD a
/// pointer entry can lead to).
fn with_one(which: &str, ent: E) -> Vec<IfdSpec> {
    // 0 IFD0, 1 Exif, 2 Interop, 3 GPS, 4 empty target.
    let mut ifd0 = vec![];
    let mut exif = vec![];
    let mut interop = vec![];
    let mut gps = vec![];
    match which {
        "exif" => {
            ifd0.push(ptr(0x8769, 1));
            exif.push(ent);
        }
        "interop" => {
            ifd0.push(ptr(0x8769, 1));
            exif.push(ptr(0xA005, 2));
            interop.push(ent);
        }
        _ => {
            ifd0.push(ptr(0x8825, 3));
            gps.push(ent);
        }
    }
    vec![
        ifd(ifd0),
        ifd(exif),
        ifd(interop),
        ifd(gps),
        empty(), // 4: the pointer target
    ]
}

/// A LONG entry whose value is the table of IFD 4 (the empty target).
fn pointing(tag: u16) -> E {
    ptr(tag, 4)
}

/// The whole grid, each shape in both byte orders.
pub fn grid() -> Vec<Shape> {
    let mut out = Vec::new();
    for be in [false, true] {
        let c = Enc { be };

        // 1. The design sample.
        out.push(shape(
            "sample",
            be,
            &[
                ifd(vec![
                    c.ascii(0x010F, "CANARY-MAKE"),
                    ptr(0x8769, 1),
                    ptr(0x8825, 3),
                ]),
                ifd(vec![
                    c.rat(0x829A, &[(1, 250)]),
                    c.short(0x8827, 200),
                    c.ascii(0x9003, "2031:02:03 04:05:06"),
                    c.srat(0x9204, &[(-1, 3)]),
                    c.undef(0x9286, b"ASCII\0\0\0CANARY-COMMENT"),
                    ptr(0xA005, 2),
                ]),
                ifd(vec![c.ascii(0x0001, "R98"), c.undef(0x0002, b"0100")]),
                ifd(vec![
                    c.ascii(0x0001, "N"),
                    c.rat(0x0002, &[(50, 1), (4, 1), (0, 1)]),
                    c.undef(0x001B, b"ASCII\0\0\0GPS-FIX"),
                ]),
            ],
            Expect::Block,
        ));

        // 3. GPS only, Exif only, neither, an empty Exif IFD plus GPS.
        out.push(shape(
            "gps-only",
            be,
            &[
                ifd(vec![c.ascii(0x010F, "X"), ptr(0x8825, 1)]),
                ifd(vec![
                    c.ascii(0x0001, "S"),
                    c.rat(0x0002, &[(1, 1), (2, 1), (3, 1)]),
                ]),
            ],
            Expect::Block,
        ));
        out.push(shape(
            "exif-only",
            be,
            &[
                ifd(vec![ptr(0x8769, 1)]),
                ifd(vec![c.short(0x8827, 100), c.rat(0x829D, &[(28, 10)])]),
            ],
            Expect::Block,
        ));
        out.push(shape(
            "neither",
            be,
            &[ifd(vec![c.ascii(0x010F, "ONLY-IFD0")])],
            Expect::Block,
        ));
        out.push(shape(
            "empty-exif-and-gps",
            be,
            &[
                ifd(vec![ptr(0x8769, 1), ptr(0x8825, 2)]),
                empty(),
                ifd(vec![c.ascii(0x0001, "N")]),
            ],
            Expect::Block,
        ));
        // IFD0 offset 0 in the header: no IFD at all.
        out.push(Shape {
            name: format!("no-ifd0/{}", if be { "be" } else { "le" }),
            bytes: if be {
                b"MM\0\x2A\0\0\0\0".to_vec()
            } else {
                b"II\x2A\0\0\0\0\0".to_vec()
            },
            expect: Expect::Block,
        });
        // An Exif IFD that only a SubIFD points to: IFD0 has none.
        out.push(shape(
            "exif-only-in-sub",
            be,
            &[
                ifd(vec![ptr(0x014A, 1)]),
                ifd(vec![ptr(0x8769, 2)]),
                ifd(vec![c.short(0x8827, 1)]),
            ],
            Expect::Block,
        ));
        // An Exif IFD that only IFD1 points to.
        out.push(shape(
            "exif-only-under-ifd1",
            be,
            &[
                ifd_next(vec![c.ascii(0x010F, "A")], Next::Ifd(1)),
                ifd(vec![ptr(0x8769, 2)]),
                ifd(vec![c.short(0x8827, 1)]),
            ],
            Expect::Block,
        ));
        // IFD0's Exif and GPS, plus an Exif IFD under IFD1.
        out.push(shape(
            "ifd0-and-ifd1-exif",
            be,
            &[
                ifd_next(vec![ptr(0x8769, 2), ptr(0x8825, 4)], Next::Ifd(1)),
                ifd(vec![ptr(0x8769, 3)]),
                ifd(vec![c.short(0x8827, 7)]),
                ifd(vec![c.short(0x8827, 99)]),
                ifd(vec![c.ascii(0x0001, "N")]),
            ],
            Expect::Block,
        ));

        // 4. MakerNote plus OffsetSchema.
        out.push(shape(
            "makernote",
            be,
            &[
                ifd(vec![ptr(0x8769, 1)]),
                ifd(vec![
                    c.ascii(0x9003, "2031:02:03 04:05:06"),
                    c.undef(0x927C, b"Nikon\0\x02\x10\0\0MM\0\x2A\0\0\0\x08 more bytes"),
                    c.slong(0xEA1D, 12),
                ]),
            ],
            Expect::Block,
        ));

        // 5. Every offset-carrying tag, in the Exif, Interop and GPS IFDs.
        let all_offset: Vec<u16> = MEASURED_OFFSET_TAGS
            .iter()
            .chain(POINTER_OFFSET_TAGS.iter())
            .chain(CONSERVATIVE_TAGS.iter())
            .copied()
            .collect();
        for which in ["exif", "interop", "gps"] {
            for tag in &all_offset {
                out.push(shape(
                    &format!("offset-{}-{}", tag_key(*tag), which),
                    be,
                    &with_one(which, pointing(*tag)),
                    Expect::Block,
                ));
            }
        }
        // 0xA005 where it is not the Exif IFD's pointer.
        for which in ["interop", "gps"] {
            out.push(shape(
                &format!("offset-0xa005-{which}"),
                be,
                &with_one(which, pointing(0xA005)),
                Expect::Block,
            ));
        }
        // 12. 0xA005 of type 13, 0xA005 in IFD0 (not in the block).
        out.push(shape(
            "a005-type13",
            be,
            &[
                ifd(vec![ptr(0x8769, 1)]),
                ifd(vec![c.short(0x8827, 1), ptr_ty(0xA005, IFD, 2)]),
                ifd(vec![c.ascii(0x0001, "R98")]),
            ],
            Expect::Block,
        ));
        out.push(shape(
            "a005-in-ifd0",
            be,
            &[
                ifd(vec![ptr(0xA005, 2), ptr(0x8769, 1)]),
                ifd(vec![c.short(0x8827, 1)]),
                ifd(vec![c.ascii(0x0001, "R98")]),
            ],
            Expect::Block,
        ));
        // An Exif pointer to 0xA005 that leads nowhere (offset 0).
        out.push(shape(
            "a005-zero",
            be,
            &[
                ifd(vec![ptr(0x8769, 1)]),
                ifd(vec![c.short(0x8827, 1), abs(0xA005, LONG, 1, 0)]),
            ],
            Expect::Block,
        ));
        // Length partners, a tag ExifTool does not know, and plain tags: kept.
        let partners = |_which: &str| -> Vec<E> {
            let mut v: Vec<E> = LENGTH_PARTNERS.iter().map(|t| c.long(*t, 1234)).collect();
            v.push(c.short(0xF0F0, 7));
            v.push(c.short(0x8827, 400));
            v
        };
        out.push(shape(
            "partners-and-unknown-tags",
            be,
            &[
                ifd(vec![ptr(0x8769, 1), ptr(0x8825, 3)]),
                ifd({
                    let mut v = partners("exif");
                    v.push(ptr(0xA005, 2));
                    v
                }),
                ifd(partners("interop")),
                ifd(partners("gps")),
            ],
            Expect::Block,
        ));

        // 6. A type-13 entry of another tag; SubIFDs inside the Exif IFD.
        out.push(shape(
            "type13-entries",
            be,
            &[
                ifd(vec![ptr(0x8769, 1), ptr(0x8825, 3)]),
                ifd(vec![
                    ptr_ty(0x9999, IFD, 4),
                    c.short(0x8827, 1),
                    ptr(0xA005, 2),
                ]),
                ifd(vec![ptr_ty(0x9998, IFD, 4)]),
                ifd(vec![ptr_ty(0x9997, IFD, 4), c.ascii(0x0001, "N")]),
                empty(),
            ],
            Expect::Block,
        ));
        out.push(shape(
            "subifds-inside-exif",
            be,
            &[
                ifd(vec![ptr(0x8769, 1)]),
                ifd(vec![c.short(0x8827, 1), ptr(0x014A, 2)]),
                ifd(vec![c.ascii(0x010F, "SUB"), ptr(0x8769, 3)]),
                ifd(vec![c.short(0x8827, 2)]),
            ],
            Expect::Block,
        ));

        // 7. Unknown types 0, 14, 16 and 255 (and a MakerNote of an unknown
        // type: the type wins, DEC-014 amendment item 2).
        out.push(shape(
            "unknown-types",
            be,
            &[
                ifd(vec![ptr(0x8769, 1), ptr(0x8825, 3)]),
                ifd(vec![
                    raw(0x9100, 0, 1, [1, 2, 3, 4]),
                    raw(0x9101, 14, 1, [1, 2, 3, 4]),
                    raw(0x9102, 16, 2, [1, 2, 3, 4]),
                    raw(0x9103, 255, 1, [1, 2, 3, 4]),
                    raw(0x927C, 0, 1, [9, 9, 9, 9]),
                    c.short(0x8827, 1),
                    ptr(0xA005, 2),
                ]),
                ifd(vec![
                    raw(0x0010, 14, 1, [0, 0, 0, 0]),
                    c.ascii(0x0001, "R98"),
                ]),
                ifd(vec![
                    raw(0x0020, 255, 4, [0, 0, 0, 0]),
                    c.ascii(0x0001, "N"),
                ]),
            ],
            Expect::Block,
        ));

        // 7b. An unknown type in an entry that is NOT first (entry 1 in the
        // Exif and Interop IFDs, type 14; entry 2 in GPS, type 255).
        // ExifTool 13.55 warns about that entry and prints the others, so the
        // parity with the placed block is exact (DEC-014 amendment item 2).
        out.push(shape(
            "unknown-type-not-first",
            be,
            &[
                ifd(vec![ptr(0x8769, 1), ptr(0x8825, 3)]),
                ifd(vec![
                    c.short(0x8827, 1),
                    raw(0x9101, 14, 1, [1, 2, 3, 4]),
                    c.short(0xA001, 1),
                    ptr(0xA005, 2),
                ]),
                ifd(vec![
                    c.ascii(0x0001, "R98"),
                    raw(0x0010, 14, 1, [0, 0, 0, 0]),
                ]),
                ifd(vec![
                    c.ascii(0x0001, "N"),
                    c.ascii(0x0003, "E"),
                    raw(0x0020, 255, 4, [0, 0, 0, 0]),
                ]),
            ],
            Expect::Block,
        ));

        // 8. Count 0, unterminated ASCII, odd-length UNDEFINED, odd source
        // offsets, a shared value, a value that overlaps a table.
        out.push(shape(
            "count-zero",
            be,
            &[
                ifd(vec![ptr(0x8769, 1)]),
                ifd(vec![
                    c.ent(0x010E, ASCII, 0, vec![]),
                    c.ent(0x8827, SHORT, 0, vec![]),
                    c.ent(0x829A, RATIONAL, 0, vec![]),
                ]),
            ],
            Expect::Block,
        ));
        out.push(shape(
            "ascii-unterminated",
            be,
            &[
                ifd(vec![ptr(0x8769, 1)]),
                ifd(vec![
                    c.ent(0x9003, ASCII, 19, b"2031:02:03 04:05:06".to_vec()),
                    c.ent(0x9290, ASCII, 3, b"123".to_vec()),
                ]),
            ],
            Expect::Block,
        ));
        out.push(shape(
            "undefined-odd-lengths",
            be,
            &[
                ifd(vec![ptr(0x8769, 1)]),
                ifd(vec![
                    c.undef(0x9000, b"0230"),
                    c.undef(0x9101, b"abc"),
                    c.undef(0xA300, b"\x03"),
                    c.undef(0xA302, b"1234567"),
                    c.undef(0x9286, b"ASCII\0\0\0odd"),
                ]),
            ],
            Expect::Block,
        ));
        out.push(shape(
            "odd-source-offsets",
            be,
            &[
                ifd(vec![ptr(0x8769, 1), ptr(0x8825, 2)]),
                ifd(vec![
                    E {
                        tag: 0x9003,
                        ty: ASCII,
                        count: 20,
                        v: V::ValOdd(b"2031:02:03 04:05:06\0".to_vec()),
                    },
                    E {
                        tag: 0x829A,
                        ty: RATIONAL,
                        count: 1,
                        v: V::ValOdd(c.rat(0, &[(1, 125)]).bytes()),
                    },
                ]),
                ifd(vec![E {
                    tag: 0x0002,
                    ty: RATIONAL,
                    count: 3,
                    v: V::ValOdd(c.rat(0, &[(1, 1), (2, 1), (3, 1)]).bytes()),
                }]),
            ],
            Expect::Block,
        ));
        out.push(shape(
            "shared-value",
            be,
            &[
                ifd(vec![ptr(0x8769, 1)]),
                ifd(vec![
                    c.ascii(0x9003, "2031:02:03 04:05:06"),
                    E {
                        tag: 0x9004,
                        ty: ASCII,
                        count: 20,
                        v: V::SameAs(1, 0),
                    },
                ]),
            ],
            Expect::Block,
        ));
        out.push(shape(
            "value-over-table",
            be,
            &[
                ifd(vec![ptr(0x8769, 1), ptr(0x8825, 2)]),
                ifd(vec![E {
                    tag: 0x829A,
                    ty: RATIONAL,
                    count: 1,
                    v: V::AtTable(2),
                }]),
                ifd(vec![c.ascii(0x0001, "N"), c.short(0x0005, 0)]),
            ],
            Expect::Block,
        ));
        // 1,000 entries sharing one 2 KiB value (the over-budget fixture: a
        // block of about 2 MiB under a smaller budget).
        out.push(shape(
            "shared-2k-x1000",
            be,
            &[ifd(vec![ptr(0x8769, 1)]), {
                let mut ents = vec![E {
                    tag: 0x9000,
                    ty: UNDEFINED,
                    count: 2048,
                    v: V::Val(vec![0x5A; 2048]),
                }];
                for i in 1..1000u16 {
                    ents.push(E {
                        tag: 0x9000 + i,
                        ty: UNDEFINED,
                        count: 2048,
                        v: V::SameAs(1, 0),
                    });
                }
                ifd(ents)
            }],
            Expect::Block,
        ));

        // Unused inline bytes are garbage in the source and zero in the block.
        out.push(shape(
            "inline-garbage",
            be,
            &[
                ifd(vec![ptr(0x8769, 1)]),
                ifd(vec![
                    raw(0x8827, SHORT, 1, [0xC8, 0x00, 0xAA, 0xBB]),
                    raw(0x9101, BYTE, 2, [1, 2, 0xCC, 0xDD]),
                    raw(0x9102, BYTE, 1, [7, 0xEE, 0xEE, 0xEE]),
                ]),
            ],
            Expect::Block,
        ));

        // 9. An Exif IFD whose next-word leads to a valid IFD.
        out.push(shape(
            "exif-next-word-chains",
            be,
            &[
                ifd(vec![ptr(0x8769, 1)]),
                ifd_next(vec![c.short(0x8827, 1)], Next::Ifd(2)),
                ifd(vec![c.ascii(0x9003, "CHAINED-NOT-FOLLOWED")]),
            ],
            Expect::Block,
        ));

        // 10. Source entries out of ascending order.
        out.push(shape(
            "unordered-entries",
            be,
            &[
                ifd(vec![ptr(0x8825, 3), ptr(0x8769, 1)]),
                ifd(vec![
                    ptr(0xA005, 2),
                    c.ascii(0x9003, "2031:02:03 04:05:06"),
                    c.short(0x8827, 5),
                    c.rat(0x829A, &[(1, 60)]),
                ]),
                ifd(vec![c.undef(0x0002, b"0100"), c.ascii(0x0001, "R98")]),
                ifd(vec![
                    c.rat(0x0004, &[(19, 1), (56, 1), (0, 1)]),
                    c.ascii(0x0003, "E"),
                    c.rat(0x0002, &[(50, 1), (4, 1), (0, 1)]),
                    c.ascii(0x0001, "N"),
                ]),
            ],
            Expect::Block,
        ));

        // 11. An Exif IFD inside a SubIFD, walked before IFD0's.
        out.push(shape(
            "exif-in-subifd-walked-first",
            be,
            &[
                ifd(vec![ptr(0x014A, 1), ptr(0x8769, 3), ptr(0x8825, 4)]),
                ifd(vec![ptr(0x8769, 2), ptr(0x8825, 5)]),
                ifd(vec![c.ascii(0x9003, "SUB-EXIF")]),
                ifd(vec![c.ascii(0x9004, "TOP-EXIF")]),
                ifd(vec![c.ascii(0x0001, "N")]),
                ifd(vec![c.ascii(0x0001, "S")]),
            ],
            Expect::Block,
        ));

        // 14. Issues that must not refuse.
        out.push(shape(
            "ok-broken-ifd1-next",
            be,
            &[
                ifd_next(vec![ptr(0x8769, 1)], Next::Abs(0x00F0_0000)),
                ifd(vec![c.short(0x8827, 1)]),
            ],
            Expect::Block,
        ));
        out.push(shape(
            "ok-subifd-loop",
            be,
            &[
                ifd(vec![ptr(0x014A, 1), ptr(0x8769, 2)]),
                ifd(vec![ptr(0x014A, 0)]),
                ifd(vec![c.short(0x8827, 1)]),
            ],
            Expect::Block,
        ));
        out.push(shape(
            "ok-broken-subifd-in-exif",
            be,
            &[
                ifd(vec![ptr(0x8769, 1)]),
                ifd(vec![c.short(0x8827, 1), abs(0x014A, LONG, 1, 0x00F0_0000)]),
            ],
            Expect::Block,
        ));

        // 14. Refusals.
        let mut bigtiff = b"II\x2B\0\x08\0\0\0\x10\0\0\0\0\0\0\0".to_vec();
        if be {
            bigtiff = b"MM\0\x2B\0\x08\0\0\0\0\0\0\0\0\0\x10".to_vec();
        }
        out.push(Shape {
            name: format!("refuse-bigtiff/{}", if be { "be" } else { "le" }),
            bytes: bigtiff,
            expect: refuse("source"),
        });
        out.push(Shape {
            name: format!("refuse-bad-header/{}", if be { "be" } else { "le" }),
            bytes: b"XX\x2A\0\x08\0\0\0\0\0".to_vec(),
            expect: refuse("source"),
        });
        out.push(Shape {
            name: format!("refuse-ifd0-unreadable/{}", if be { "be" } else { "le" }),
            bytes: if be {
                b"MM\0\x2A\0\x01\0\0".to_vec()
            } else {
                b"II\x2A\0\0\0\x01\0".to_vec()
            },
            expect: refuse("source"),
        });
        // 600 SubIFD-chained empty IFDs halt the walk before IFD0's Exif
        // pointer is walked.
        let mut chain = vec![ifd(vec![ptr(0x014A, 1), ptr(0x8769, 601)])];
        for i in 1..=600usize {
            chain.push(ifd_next(
                vec![],
                if i < 600 {
                    Next::Ifd(i + 1)
                } else {
                    Next::Zero
                },
            ));
        }
        chain.push(ifd(vec![c.short(0x8827, 1)]));
        out.push(shape(
            "refuse-walk-halted",
            be,
            &chain,
            refuse("walk-halted"),
        ));
        out.push(shape(
            "refuse-exif-loops-to-ifd0",
            be,
            &[ifd(vec![ptr(0x8769, 0)])],
            refuse("broken-pointer:0x8769"),
        ));
        out.push(shape(
            "refuse-exif-and-gps-same-offset",
            be,
            &[
                ifd(vec![ptr(0x8769, 1), ptr(0x8825, 1)]),
                ifd(vec![c.short(0x8827, 1)]),
            ],
            refuse("broken-pointer:0x8825"),
        ));
        out.push(shape(
            "refuse-a005-past-end",
            be,
            &[
                ifd(vec![ptr(0x8769, 1)]),
                ifd(vec![c.short(0x8827, 1), abs(0xA005, LONG, 1, 0x00F0_0000)]),
            ],
            refuse("broken-pointer:0xa005"),
        ));
        out.push(shape(
            "refuse-exif-pointer-bad-type",
            be,
            &[ifd(vec![c.short(0x8769, 1)])],
            refuse("broken-pointer:0x8769"),
        ));
        out.push(shape(
            "refuse-duplicate-exif-pointer",
            be,
            &[
                ifd(vec![ptr(0x8769, 1), ptr(0x8769, 1)]),
                ifd(vec![c.short(0x8827, 1)]),
            ],
            refuse("duplicate-pointer:0x8769"),
        ));
        out.push(shape(
            "refuse-duplicate-gps-pointer",
            be,
            &[
                ifd(vec![ptr(0x8825, 1), ptr(0x8825, 1)]),
                ifd(vec![c.ascii(0x0001, "N")]),
            ],
            refuse("duplicate-pointer:0x8825"),
        ));
        out.push(shape(
            "refuse-duplicate-a005",
            be,
            &[
                ifd(vec![ptr(0x8769, 1)]),
                ifd(vec![ptr(0xA005, 2), ptr(0xA005, 2)]),
                ifd(vec![c.ascii(0x0001, "R98")]),
            ],
            refuse("duplicate-pointer:0xa005"),
        ));
        out.push(shape(
            "refuse-duplicate-tag-exif",
            be,
            &[
                ifd(vec![ptr(0x8769, 1)]),
                ifd(vec![
                    c.ascii(0x9003, "2031:02:03 04:05:06"),
                    c.ascii(0x9003, "2032:02:03 04:05:06"),
                ]),
            ],
            refuse("duplicate-tag:exif:0x9003"),
        ));
        out.push(shape(
            "refuse-duplicate-tag-gps",
            be,
            &[
                ifd(vec![ptr(0x8825, 1)]),
                ifd(vec![c.ascii(0x0001, "N"), c.ascii(0x0001, "S")]),
            ],
            refuse("duplicate-tag:gps:0x0001"),
        ));
        out.push(shape(
            "refuse-duplicate-tag-interop",
            be,
            &[
                ifd(vec![ptr(0x8769, 1)]),
                ifd(vec![ptr(0xA005, 2)]),
                ifd(vec![c.ascii(0x0001, "R98"), c.ascii(0x0001, "R03")]),
            ],
            refuse("duplicate-tag:interop:0x0001"),
        ));
        out.push(shape(
            "refuse-unreadable-value",
            be,
            &[
                ifd(vec![ptr(0x8769, 1)]),
                ifd(vec![abs(0x829A, RATIONAL, 1, 0x00F0_0000)]),
            ],
            refuse("unreadable:exif:0x829a"),
        ));
        out.push(shape(
            "refuse-unreadable-gps-value",
            be,
            &[
                ifd(vec![ptr(0x8825, 1)]),
                ifd(vec![abs(0x0002, RATIONAL, 3, 0x00F0_0000)]),
            ],
            refuse("unreadable:gps:0x0002"),
        ));
    }
    out
}

impl E {
    /// The encoded bytes of a `Val` entry (for `ValOdd` re-wrapping).
    pub fn bytes(&self) -> Vec<u8> {
        match &self.v {
            V::Val(b) | V::ValOdd(b) => b.clone(),
            _ => Vec::new(),
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// SPEC-041 (Part A, AC10): the shapes of cross-order conversion. Each shape is
// built in both source orders, so a loop over [`order_shapes`] covers both
// directions (a big-endian source to little-endian, and the reverse).
// ═════════════════════════════════════════════════════════════════════════════

pub const SBYTE: u16 = 6;
pub const SSHORT: u16 = 8;
pub const FLOAT: u16 = 11;
pub const DOUBLE: u16 = 12;

/// A tag that no table of this crate or of ExifTool's sweep names.
const PLAIN_LOW: u16 = 0xF000;
const PLAIN_HIGH: u16 = 0xF0F0;

/// The element size of a type 1 to 12, typed again by hand.
pub fn elem_size(ty: u16) -> usize {
    match ty {
        1 | 2 | 6 | 7 => 1,
        3 | 8 => 2,
        4 | 9 | 11 => 4,
        5 | 10 | 12 => 8,
        _ => panic!("no element size for type {ty}"),
    }
}

/// The unit a conversion swaps (a RATIONAL swaps each 4-byte half).
fn swap_unit(ty: u16) -> usize {
    match ty {
        3 | 8 => 2,
        4 | 5 | 9 | 10 | 11 => 4,
        12 => 8,
        _ => 1,
    }
}

/// `count` elements of type `ty` with distinct, asymmetric bytes, encoded in
/// the given byte order, one swap unit at a time.
pub fn typed_value(be: bool, ty: u16, count: u32) -> Vec<u8> {
    let n = elem_size(ty) * count as usize;
    let logical: Vec<u8> = (0..n)
        .map(|i| (i as u8).wrapping_mul(37).wrapping_add(1))
        .collect();
    let mut out = Vec::new();
    for unit in logical.chunks(swap_unit(ty)) {
        if be {
            out.extend_from_slice(unit);
        } else {
            out.extend(unit.iter().rev());
        }
    }
    out
}

/// A source whose `which` IFD (`exif`, `interop` or `gps`) holds `entry`
/// between two plain SHORTs, so a kept entry's neighbours show that the rest of
/// the table does not move.
pub fn one_entry_source(be: bool, which: &str, entry: E) -> Vec<u8> {
    let c = Enc { be };
    let ents = vec![
        c.short(PLAIN_LOW, 0x0102),
        entry,
        c.short(PLAIN_HIGH, 0x0304),
    ];
    let doc = match which {
        "exif" => vec![ifd(vec![ptr(0x8769, 1)]), ifd(ents)],
        "interop" => vec![
            ifd(vec![ptr(0x8769, 1)]),
            ifd(vec![ptr(0xA005, 2)]),
            ifd(ents),
        ],
        _ => vec![ifd(vec![ptr(0x8825, 1)]), ifd(ents)],
    };
    build(be, &doc)
}

/// The ByteOrder table's rows for the Exif and Interop IFDs (`which` is
/// `exif` or `interop`) or the GPS IFD, as `(tag, declared types left out)`.
/// Typed again by hand for the shapes (a mirror of the decision's table,
/// compared by `byte_order_mirrors_agree`).
pub fn byte_order_rows(which: &str) -> Vec<(u16, Vec<u16>)> {
    if which == "gps" {
        return vec![(0x001D, vec![4, 5, 9, 10, 11, 12])];
    }
    let multi: Vec<u16> = vec![3, 4, 5, 8, 9, 10, 11, 12];
    let every: Vec<u16> = (1..=12).collect();
    let mut rows: Vec<(u16, Vec<u16>)> = [
        0x0303u16, 0x4747, 0x828E, 0x8298, 0x9009, 0x9101, 0x9286, 0x9C9B, 0x9C9C, 0x9C9D, 0x9C9E,
        0x9C9F, 0xBC01, 0xC615, 0xC65D, 0xC68B, 0xC68C, 0xC68F, 0xC691, 0xC6D2, 0xC6D3, 0xC6F3,
        0xC6F4, 0xC6F6, 0xC6F8, 0xC6FE, 0xC716, 0xC717, 0xC718, 0xC719, 0xC71C, 0xC71D, 0xC7A7,
        0xC7AA, 0xCD48,
    ]
    .into_iter()
    .map(|t| (t, multi.clone()))
    .collect();
    rows.push((0x87AF, vec![12]));
    rows.push((0x9201, vec![1, 2, 6, 7]));
    rows.push((0x9204, vec![1, 2, 3, 6, 7, 8, 12]));
    rows.push((0xA101, vec![4, 5, 9, 10, 11, 12]));
    rows.push((0xA302, vec![1, 2, 3, 4, 5, 6, 8, 9, 10, 11, 12]));
    rows.push((0xA462, every.clone()));
    for tag in [0x8828u16, 0xA20C, 0xA40B] {
        rows.push((tag, every.clone()));
    }
    rows
}

/// The usual (kept) type of a row: the first of a fixed preference list that
/// the row does not leave out; `None` when it leaves out all 12.
pub fn usual_type(masked: &[u16]) -> Option<u16> {
    [7u16, 2, 1, 3, 10, 4, 5, 6, 8, 9, 11, 12]
        .into_iter()
        .find(|t| !masked.contains(t))
}

/// The design's CFAPattern shapes as `(name, source value, block value)`. The
/// rule swaps the first two 16-bit words and copies the rest, so the same
/// literals serve both directions.
pub fn cfa_cases() -> Vec<(&'static str, Vec<u8>, Vec<u8>)> {
    let mut wide_src = vec![0x00, 0x01, 0x01, 0x00];
    let mut wide_want = vec![0x01, 0x00, 0x00, 0x01];
    for i in 0..=255u8 {
        wide_src.push(i % 3);
        wide_want.push(i % 3);
    }
    vec![
        (
            "2x2",
            vec![0, 2, 0, 2, 1, 2, 2, 1],
            vec![2, 0, 2, 0, 1, 2, 2, 1],
        ),
        (
            "3x2",
            vec![0, 3, 0, 2, 0, 1, 2, 1, 2, 0],
            vec![3, 0, 2, 0, 0, 1, 2, 1, 2, 0],
        ),
        (
            "wrong-order-header",
            vec![2, 0, 2, 0, 1, 2, 2, 1],
            vec![0, 2, 0, 2, 1, 2, 2, 1],
        ),
        ("1x256", wide_src, wide_want),
        (
            "3x3-over-4-bytes",
            vec![0, 3, 0, 3, 1, 2, 0, 1],
            vec![3, 0, 3, 0, 1, 2, 0, 1],
        ),
        ("0x0", vec![0, 0, 0, 0], vec![0, 0, 0, 0]),
        (
            "2x2-trailing",
            vec![0, 2, 0, 2, 1, 2, 2, 1, 0xAA, 0xBB],
            vec![2, 0, 2, 0, 1, 2, 2, 1, 0xAA, 0xBB],
        ),
        ("3-bytes", vec![0x12, 0x34, 0x56], vec![0x12, 0x34, 0x56]),
        ("1-byte", vec![0x12], vec![0x12]),
        ("count-0", vec![], vec![]),
    ]
}

fn comment(code: &[u8; 8], text: &[u8]) -> Vec<u8> {
    let mut v = code.to_vec();
    v.extend_from_slice(text);
    v
}

/// The UNICODE-text shapes as `(name, source value, block value)`. 中文 is
/// U+4E2D U+6587; the converted ones swap every 16-bit unit after the code.
pub fn unicode_cases() -> Vec<(&'static str, Vec<u8>, Vec<u8>)> {
    let cjk = [0x4E, 0x2D, 0x65, 0x87];
    let cjk_swapped = [0x2D, 0x4E, 0x87, 0x65];
    let mut cases = vec![
        (
            "unicode-nul",
            comment(b"UNICODE\0", &cjk),
            comment(b"UNICODE\0", &cjk_swapped),
        ),
        (
            "unicode-space",
            comment(b"UNICODE ", &cjk),
            comment(b"UNICODE ", &cjk_swapped),
        ),
        (
            "unicode-bom",
            comment(b"UNICODE\0", &[0xFE, 0xFF, 0x4E, 0x2D, 0x65, 0x87]),
            comment(b"UNICODE\0", &[0xFF, 0xFE, 0x2D, 0x4E, 0x87, 0x65]),
        ),
        (
            "unicode-odd",
            comment(b"UNICODE\0", &[0x4E, 0x2D, 0x65]),
            comment(b"UNICODE\0", &[0x2D, 0x4E, 0x65]),
        ),
        (
            "unicode-code-only",
            comment(b"UNICODE\0", &[]),
            comment(b"UNICODE\0", &[]),
        ),
    ];
    // Copied: every other code, and values under 8 bytes.
    for (name, code) in [
        ("lower-case", b"Unicode\0"),
        ("trailing-x", b"UNICODEX"),
        ("leading-space", b" UNICODE"),
        ("tab", b"UNICODE\t"),
        ("short-code", b"UNICOD\0\0"),
        ("ascii", b"ASCII\0\0\0"),
        ("jis", b"JIS\0\0\0\0\0"),
        ("zeros", &[0u8; 8]),
    ] {
        cases.push((name, comment(code, &cjk), comment(code, &cjk)));
    }
    cases.push(("under-8", b"UNICOD".to_vec(), b"UNICOD".to_vec()));
    cases.push(("exactly-7", b"UNICODE".to_vec(), b"UNICODE".to_vec()));
    cases
}

/// Where the UNICODE rule applies: `(IFD, tag, types)`.
pub fn unicode_places() -> Vec<(&'static str, u16, Vec<u16>)> {
    vec![
        ("exif", 0x9286, vec![BYTE, ASCII, SBYTE, UNDEFINED]),
        ("interop", 0x9286, vec![BYTE, ASCII, SBYTE, UNDEFINED]),
        ("gps", 0x001B, vec![UNDEFINED]),
        ("gps", 0x001C, vec![UNDEFINED]),
    ]
}

/// The shapes of cross-order conversion, each in both source orders.
pub fn order_shapes() -> Vec<Shape> {
    let mut out = Vec::new();
    for be in [false, true] {
        let c = Enc { be };

        // 1. Every declared type 1 to 12, inline (count 1 of the small types)
        // and out of line, in all three IFDs.
        let types_in = |base_tag: u16| -> Vec<E> {
            let mut v = Vec::new();
            for ty in 1..=12u16 {
                let small = elem_size(ty) <= 4;
                if small {
                    v.push(c.ent(base_tag + ty, ty, 1, typed_value(be, ty, 1)));
                }
                let count = if elem_size(ty) == 1 { 7 } else { 3 };
                v.push(c.ent(base_tag + 0x100 + ty, ty, count, typed_value(be, ty, count)));
            }
            v
        };
        out.push(shape(
            "every-type",
            be,
            &[
                ifd(vec![ptr(0x8769, 1), ptr(0x8825, 3)]),
                {
                    let mut v = types_in(0xF100);
                    v.push(ptr(0xA005, 2));
                    ifd(v)
                },
                ifd(types_in(0xF100)),
                ifd(types_in(0xF100)),
            ],
            Expect::Block,
        ));

        // 2. One fixture per ByteOrder row, in its usual type (kept) and in a
        // masked type (left out), in each IFD the row names.
        for which in ["exif", "interop", "gps"] {
            for (tag, masked) in byte_order_rows(which) {
                if let Some(usual) = usual_type(&masked) {
                    out.push(Shape {
                        name: format!("row-kept-{which}-{}/{}", tag_key(tag), tag_be(be)),
                        bytes: one_entry_source(
                            be,
                            which,
                            c.ent(tag, usual, 1, typed_value(be, usual, 1)),
                        ),
                        expect: Expect::Block,
                    });
                }
                let first = masked[0];
                out.push(Shape {
                    name: format!("row-masked-{which}-{}/{}", tag_key(tag), tag_be(be)),
                    bytes: one_entry_source(
                        be,
                        which,
                        c.ent(tag, first, 1, typed_value(be, first, 1)),
                    ),
                    expect: Expect::Block,
                });
            }
        }

        // 3. The CFAPattern shapes, in the Exif and the Interop IFD.
        for which in ["exif", "interop"] {
            for (name, src, _) in cfa_cases() {
                out.push(Shape {
                    name: format!("cfa-{which}-{name}/{}", tag_be(be)),
                    bytes: one_entry_source(be, which, c.undef(0xA302, &src)),
                    expect: Expect::Block,
                });
            }
        }

        // 4. The UNICODE shapes, at every place the rule applies.
        for (which, tag, types) in unicode_places() {
            for ty in types {
                for (name, src, _) in unicode_cases() {
                    out.push(Shape {
                        name: format!(
                            "unicode-{which}-{}-t{ty}-{name}/{}",
                            tag_key(tag),
                            tag_be(be)
                        ),
                        bytes: one_entry_source(
                            be,
                            which,
                            c.ent(tag, ty, src.len() as u32, src.clone()),
                        ),
                        expect: Expect::Block,
                    });
                }
            }
        }

        // 5. OECF, SpatialFrequencyResponse and DeviceSettingDescription with
        // well-formed contents: two SHORTs in the file's order, then data.
        let two_shorts = |a: u16, b: u16, rest: &[u8]| -> Vec<u8> {
            let mut v = c.w16(a).to_vec();
            v.extend_from_slice(&c.w16(b));
            v.extend_from_slice(rest);
            v
        };
        out.push(shape(
            "oecf-sfr-dsd",
            be,
            &[
                ifd(vec![ptr(0x8769, 1)]),
                ifd(vec![
                    c.undef(
                        0x8828,
                        &two_shorts(2, 2, b"A\0B\0\x01\x02\x03\x04\x05\x06\x07\x08"),
                    ),
                    c.undef(0xA20C, &two_shorts(1, 3, b"F\0\x09\x0A\x0B\x0C\x0D\x0E")),
                    c.undef(0xA40B, &two_shorts(2, 1, b"X\0\x01\x00\x02\x00")),
                    c.short(0x8827, 100),
                ]),
            ],
            Expect::Block,
        ));

        // 6. Precedence: UnknownType, then MakerNote, then OffsetCarrying,
        // then ByteOrder.
        out.push(shape(
            "precedence",
            be,
            &[
                ifd(vec![ptr(0x8769, 1)]),
                ifd(vec![
                    raw(0x8828, 0, 1, [1, 2, 3, 4]),
                    c.short(0x9286, 7),
                    ptr_ty(0x9287, IFD, 2),
                    c.undef(0x927C, b"Nikon\0\x02\x10"),
                    raw(0xA20C, 14, 1, [1, 2, 3, 4]),
                    c.ent(0xA40B, UNDEFINED, 4, vec![0, 1, 0, 2]),
                ]),
                empty(),
            ],
            Expect::Block,
        ));
        // 0x9286 as type 13 (an IFD, so offset-carrying).
        out.push(shape(
            "precedence-9286-type13",
            be,
            &[
                ifd(vec![ptr(0x8769, 1)]),
                ifd(vec![ptr_ty(0x9286, IFD, 2)]),
                empty(),
            ],
            Expect::Block,
        ));
    }
    out
}

fn tag_be(be: bool) -> &'static str {
    if be {
        "be"
    } else {
        "le"
    }
}
