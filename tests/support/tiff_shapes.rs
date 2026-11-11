//! Fixtures for the TIFF and DNG in-place strip (SPEC-022): one generator, no
//! import from `src/`. Every fixture is laid out by [`B`], which records what
//! each byte range is as it appends (header, IFD table, a named value, a named
//! image-data range) and the canary text of every metadata carrier with the
//! category the audit table (DEC-008) gives it. Image data is a nonzero
//! pattern, so a stray zeroing or move is visible.
//!
//! Shapes: a stripped TIFF, a tiled TIFF, a multi-page TIFF (IFD1 is a page),
//! a TIFF with a JPEG thumbnail in IFD1, and a DNG-shaped file (SubIFDs: raw
//! tiles and a preview strip, GPS and an artist inside the raw SubIFD). Each in
//! both byte orders. [`hostile`] has the structural attacks.

#![allow(dead_code)]

use std::ops::Range;

use exif_core::audit::Category;

pub const BYTE: u16 = 1;
pub const ASCII: u16 = 2;
pub const SHORT: u16 = 3;
pub const LONG: u16 = 4;
pub const RATIONAL: u16 = 5;
pub const UNDEFINED: u16 = 7;
pub const SRATIONAL: u16 = 10;

pub const C_DESC: &[u8] = b"CANARY-TIFF-DESCRIPTION";
pub const C_MAKE: &[u8] = b"CANARY-TIFF-MAKE";
pub const C_MODEL: &[u8] = b"CANARY-TIFF-MODEL";
pub const C_SOFTWARE: &[u8] = b"CANARY-TIFF-SOFTWARE";
pub const C_SOFTWARE1: &[u8] = b"CANARY-PAGE1-SOFTWARE";
pub const C_DATETIME: &[u8] = b"2031:02:03 04:05:06";
pub const C_DTO: &[u8] = b"2032:07:08 09:10:11";
pub const C_ARTIST: &[u8] = b"CANARY-TIFF-ARTIST";
pub const C_SUB_ARTIST: &[u8] = b"CANARY-SUBIFD-ARTIST";
pub const C_XMP: &[u8] = b"CANARY-TIFF-XMP-PACKET";
pub const C_IPTC: &[u8] = b"CANARY-TIFF-IPTC";
pub const C_PSD: &[u8] = b"CANARY-TIFF-PHOTOSHOP";
pub const C_ICC: &[u8] = b"CANARY-TIFF-ICC-PROFILE";
pub const C_GPS: &[u8] = b"CANARY-TIFF-GPS-AREA";
pub const C_SUB_GPS: &[u8] = b"CANARY-SUBIFD-GPS-AREA";
pub const C_SERIAL: &[u8] = b"CANARY-TIFF-SERIAL";
pub const C_COMMENT: &[u8] = b"CANARY-TIFF-USERCOMMENT";
pub const C_NOTE: &[u8] = b"CANARY-TIFF-MAKERNOTE";
pub const C_DNG_SERIAL: &[u8] = b"CANARY-DNG-SERIAL";
pub const C_THUMB: &[u8] = b"CANARY-TIFF-THUMBNAIL";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum K {
    Header,
    Table,
    /// An out-of-line value (not image data).
    Value,
    /// Strip, tile, raw or preview data, or the JPEG bytes of a kept page.
    Image,
    /// The JPEG bytes of an IFD1 thumbnail (zeroed when the thumbnail goes).
    Thumb,
}

#[derive(Clone, Debug)]
pub struct Part {
    pub kind: K,
    pub range: Range<usize>,
}

#[derive(Clone, Debug)]
pub struct Shape {
    pub name: String,
    pub bytes: Vec<u8>,
    pub parts: Vec<Part>,
    /// `(canary, the category the audit table gives its carrier)`.
    pub canaries: Vec<(&'static [u8], Category)>,
    /// Every strip or tile offset array in the file, `(tag, values)`, in
    /// file order of the entries.
    pub offsets: Vec<(u16, Vec<u32>)>,
    /// IFD1 is a page (strips), not a JPEG thumbnail.
    pub ifd1_page: bool,
    pub has_thumb: bool,
}

pub enum V {
    Inline(Vec<u8>),
    Off(u32),
}

pub struct B {
    pub be: bool,
    pub buf: Vec<u8>,
    pub parts: Vec<Part>,
    pub canaries: Vec<(&'static [u8], Category)>,
    pub offsets: Vec<(u16, Vec<u32>)>,
}

impl B {
    pub fn new(be: bool) -> B {
        let mut b = B {
            be,
            buf: Vec::new(),
            parts: Vec::new(),
            canaries: Vec::new(),
            offsets: Vec::new(),
        };
        b.buf
            .extend_from_slice(if be { b"MM\0\x2A" } else { b"II\x2A\0" });
        b.buf.extend_from_slice(&[0; 4]);
        b.parts.push(Part {
            kind: K::Header,
            range: 0..8,
        });
        b
    }
    pub fn w16(&self, v: u16) -> [u8; 2] {
        if self.be {
            v.to_be_bytes()
        } else {
            v.to_le_bytes()
        }
    }
    pub fn w32(&self, v: u32) -> [u8; 4] {
        if self.be {
            v.to_be_bytes()
        } else {
            v.to_le_bytes()
        }
    }
    pub fn at32(&mut self, at: usize, v: u32) {
        let w = self.w32(v);
        self.buf[at..at + 4].copy_from_slice(&w);
    }
    pub fn set_ifd0(&mut self, at: usize) {
        self.at32(4, at as u32);
    }
    /// Append bytes (word aligned) and record them.
    pub fn alloc(&mut self, kind: K, bytes: &[u8]) -> u32 {
        if self.buf.len() % 2 == 1 {
            self.buf.push(0);
        }
        let at = self.buf.len();
        self.buf.extend_from_slice(bytes);
        self.parts.push(Part {
            kind,
            range: at..at + bytes.len(),
        });
        at as u32
    }
    /// Reserve an IFD table of `n` entries (next word 0).
    pub fn ifd(&mut self, n: usize) -> usize {
        let mut t = self.w16(n as u16).to_vec();
        t.extend(std::iter::repeat_n(0u8, n * 12 + 4));
        self.alloc(K::Table, &t) as usize
    }
    pub fn entry(&mut self, ifd: usize, i: usize, tag: u16, typ: u16, count: u32, v: V) {
        let at = ifd + 2 + i * 12;
        let mut e = Vec::new();
        e.extend_from_slice(&self.w16(tag));
        e.extend_from_slice(&self.w16(typ));
        e.extend_from_slice(&self.w32(count));
        match v {
            V::Inline(mut b) => {
                b.resize(4, 0);
                e.extend_from_slice(&b);
            }
            V::Off(o) => e.extend_from_slice(&self.w32(o)),
        }
        self.buf[at..at + 12].copy_from_slice(&e);
    }
    pub fn next(&mut self, ifd: usize, n: usize, next: u32) {
        self.at32(ifd + 2 + n * 12, next);
    }
    /// An entry whose value is appended out of line (or inline when it fits).
    pub fn put(&mut self, ifd: usize, i: usize, tag: u16, typ: u16, count: u32, data: &[u8]) {
        if data.len() <= 4 {
            self.entry(ifd, i, tag, typ, count, V::Inline(data.to_vec()));
        } else {
            let off = self.alloc(K::Value, data);
            self.entry(ifd, i, tag, typ, count, V::Off(off));
        }
    }
    pub fn text(
        &mut self,
        ifd: usize,
        i: usize,
        tag: u16,
        s: &'static [u8],
        cat: Option<Category>,
    ) {
        let mut d = s.to_vec();
        d.push(0);
        self.put(ifd, i, tag, ASCII, d.len() as u32, &d);
        self.canaries.extend(cat.map(|c| (s, c)));
    }
    pub fn blob(&mut self, ifd: usize, i: usize, tag: u16, s: &'static [u8], cat: Category) {
        let mut d = s.to_vec();
        d.extend_from_slice(&[0x11, 0x22, 0x33, 0x44, 0x55]);
        self.put(ifd, i, tag, UNDEFINED, d.len() as u32, &d);
        self.canaries.push((s, cat));
    }
    /// A carrier whose bytes are exactly `d` (a structure ExifTool parses),
    /// holding the canary `s`.
    pub fn raw_blob(
        &mut self,
        ifd: usize,
        i: usize,
        tag: u16,
        d: &[u8],
        s: &'static [u8],
        cat: Category,
    ) {
        self.put(ifd, i, tag, UNDEFINED, d.len() as u32, d);
        self.canaries.push((s, cat));
    }
    pub fn long(&mut self, ifd: usize, i: usize, tag: u16, v: u32) {
        let w = self.w32(v).to_vec();
        self.entry(ifd, i, tag, LONG, 1, V::Inline(w));
    }
    pub fn short(&mut self, ifd: usize, i: usize, tag: u16, v: u16) {
        let w = self.w16(v).to_vec();
        self.entry(ifd, i, tag, SHORT, 1, V::Inline(w));
    }
    pub fn longs(&mut self, ifd: usize, i: usize, tag: u16, vs: &[u32]) {
        let mut d = Vec::new();
        for v in vs {
            d.extend_from_slice(&self.w32(*v));
        }
        self.put(ifd, i, tag, LONG, vs.len() as u32, &d);
    }
    /// A strips or tiles block: data appended as image ranges, offset and
    /// byte-count entries written at `i` and `i + 1` (the tags are the pair).
    pub fn data_entries(
        &mut self,
        ifd: usize,
        i: usize,
        j: usize,
        tags: (u16, u16),
        lens: &[usize],
        seed: u8,
    ) {
        let mut offs = Vec::new();
        for (n, l) in lens.iter().enumerate() {
            let off = self.alloc(K::Image, &pat(seed.wrapping_add(n as u8), *l));
            offs.push(off);
        }
        let counts: Vec<u32> = lens.iter().map(|l| *l as u32).collect();
        self.longs(ifd, i, tags.0, &offs);
        self.longs(ifd, j, tags.1, &counts);
        self.offsets.push((tags.0, offs));
    }
    pub fn gps(&mut self, area: &'static [u8]) -> usize {
        let g = self.ifd(3);
        self.text(g, 0, 1, b"N", None);
        let lat = [
            self.w32(52),
            self.w32(1),
            self.w32(30),
            self.w32(1),
            self.w32(15),
            self.w32(1),
        ]
        .concat();
        self.put(g, 1, 2, RATIONAL, 3, &lat);
        let mut d = area.to_vec();
        d.extend_from_slice(&[9, 9, 9]);
        self.put(g, 2, 0x1C, UNDEFINED, d.len() as u32, &d);
        self.canaries.push((area, Category::Gps));
        g
    }
    pub fn finish(self, name: &str, ifd1_page: bool, has_thumb: bool) -> Shape {
        Shape {
            name: name.to_string(),
            bytes: self.buf,
            parts: self.parts,
            canaries: self.canaries,
            offsets: self.offsets,
            ifd1_page,
            has_thumb,
        }
    }
}

/// Nonzero pattern bytes.
pub fn pat(seed: u8, n: usize) -> Vec<u8> {
    (0..n)
        .map(|i| 1 + ((i * 7 + usize::from(seed)) % 250) as u8)
        .collect()
}

fn name(base: &str, be: bool) -> String {
    format!("{base}-{}", if be { "be" } else { "le" })
}

/// A minimal XMP packet ExifTool reads: `dc:creator` holds the canary.
fn xmp_packet() -> Vec<u8> {
    let mut v = b"<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF \
xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"><rdf:Description rdf:about=\"\" \
xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><dc:creator><rdf:Seq><rdf:li>"
        .to_vec();
    v.extend_from_slice(C_XMP);
    v.extend_from_slice(
        b"</rdf:li></rdf:Seq></dc:creator></rdf:Description></rdf:RDF></x:xmpmeta>",
    );
    v
}

/// One IPTC IIM dataset, record 2 dataset 80 (By-line), holding the canary.
fn iptc_record() -> Vec<u8> {
    let mut v = vec![0x1C, 0x02, 0x50, 0, C_IPTC.len() as u8];
    v.extend_from_slice(C_IPTC);
    v
}

/// A minimal ICC profile: 128-byte header, one `desc` tag (ProfileDescription)
/// holding the canary, and the exact profile size in the header.
fn icc_profile() -> Vec<u8> {
    let mut desc = b"desc\0\0\0\0".to_vec();
    desc.extend_from_slice(&(C_ICC.len() as u32 + 1).to_be_bytes());
    desc.extend_from_slice(C_ICC);
    desc.push(0);
    let total = 128 + 4 + 12 + desc.len();
    let mut v = vec![0u8; 128];
    v[0..4].copy_from_slice(&(total as u32).to_be_bytes());
    v[8..12].copy_from_slice(&0x0210_0000u32.to_be_bytes());
    v[12..16].copy_from_slice(b"mntr");
    v[16..20].copy_from_slice(b"RGB ");
    v[20..24].copy_from_slice(b"XYZ ");
    v[36..40].copy_from_slice(b"acsp");
    v.extend_from_slice(&1u32.to_be_bytes());
    v.extend_from_slice(b"desc");
    v.extend_from_slice(&144u32.to_be_bytes());
    v.extend_from_slice(&(desc.len() as u32).to_be_bytes());
    v.extend_from_slice(&desc);
    v
}

/// IFD0 with every carrier, two strips, an Exif IFD and a GPS IFD.
pub fn stripped(be: bool) -> Shape {
    let mut b = B::new(be);
    let ifd0 = b.ifd(16);
    let exif = b.ifd(4);
    b.set_ifd0(ifd0);
    b.text(ifd0, 0, 0x010E, C_DESC, Some(Category::FreeText));
    b.text(ifd0, 1, 0x010F, C_MAKE, Some(Category::CameraModel));
    b.text(ifd0, 2, 0x0110, C_MODEL, Some(Category::CameraModel));
    b.data_entries(ifd0, 3, 5, (0x0111, 0x0117), &[300, 200], 3);
    b.short(ifd0, 4, 0x0112, 1);
    b.text(ifd0, 6, 0x0131, C_SOFTWARE, Some(Category::Software));
    b.text(ifd0, 7, 0x0132, C_DATETIME, Some(Category::Timestamps));
    b.text(ifd0, 8, 0x013B, C_ARTIST, Some(Category::Authorship));
    b.raw_blob(ifd0, 9, 0x02BC, &xmp_packet(), C_XMP, Category::Xmp);
    b.raw_blob(ifd0, 10, 0x83BB, &iptc_record(), C_IPTC, Category::Iptc);
    b.blob(ifd0, 11, 0x8649, C_PSD, Category::Iptc);
    b.long(ifd0, 12, 0x8769, exif as u32);
    b.raw_blob(ifd0, 13, 0x8773, &icc_profile(), C_ICC, Category::Icc);
    let g = b.gps(C_GPS);
    b.long(ifd0, 14, 0x8825, g as u32);
    b.entry(ifd0, 15, 0xC612, BYTE, 4, V::Inline(vec![1, 4, 0, 0]));
    b.text(exif, 0, 0x9003, C_DTO, Some(Category::Timestamps));
    b.blob(exif, 1, 0x9286, C_COMMENT, Category::FreeText);
    b.blob(exif, 2, 0x927C, C_NOTE, Category::MakerNote);
    b.text(exif, 3, 0xA431, C_SERIAL, Some(Category::DeviceId));
    b.finish(&name("stripped", be), false, false)
}

/// A tiled TIFF: four tiles, a Make canary, a GPS IFD.
pub fn tiled(be: bool) -> Shape {
    let mut b = B::new(be);
    let ifd0 = b.ifd(6);
    b.set_ifd0(ifd0);
    b.text(ifd0, 0, 0x010F, C_MAKE, Some(Category::CameraModel));
    b.short(ifd0, 1, 0x0142, 16);
    b.short(ifd0, 2, 0x0143, 16);
    b.data_entries(ifd0, 3, 4, (0x0144, 0x0145), &[256, 256, 256, 128], 9);
    let g = b.gps(C_GPS);
    b.long(ifd0, 5, 0x8825, g as u32);
    b.finish(&name("tiled", be), false, false)
}

/// Three pages: IFD0, IFD1 (a page: strips, a Software canary) and a chain IFD
/// with an Artist canary.
pub fn multipage(be: bool) -> Shape {
    let mut b = B::new(be);
    let p0 = b.ifd(4);
    let p1 = b.ifd(4);
    let p2 = b.ifd(4);
    b.set_ifd0(p0);
    b.next(p0, 4, p1 as u32);
    b.next(p1, 4, p2 as u32);
    b.text(p0, 0, 0x010F, C_MAKE, Some(Category::CameraModel));
    b.data_entries(p0, 1, 3, (0x0111, 0x0117), &[200], 1);
    b.short(p0, 2, 0x0112, 1);
    b.data_entries(p1, 0, 2, (0x0111, 0x0117), &[150, 150], 5);
    b.short(p1, 1, 0x0112, 1);
    b.text(p1, 3, 0x0131, C_SOFTWARE1, Some(Category::Software));
    b.data_entries(p2, 0, 2, (0x0111, 0x0117), &[100], 7);
    b.short(p2, 1, 0x0112, 1);
    b.text(p2, 3, 0x013B, C_ARTIST, Some(Category::Authorship));
    b.finish(&name("multipage", be), true, false)
}

/// IFD0 (strips, Make, GPS) then IFD1 holding a JPEG thumbnail.
pub fn thumb(be: bool) -> Shape {
    let mut b = B::new(be);
    let p0 = b.ifd(5);
    let p1 = b.ifd(3);
    b.set_ifd0(p0);
    b.next(p0, 5, p1 as u32);
    b.text(p0, 0, 0x010F, C_MAKE, Some(Category::CameraModel));
    b.data_entries(p0, 1, 3, (0x0111, 0x0117), &[256], 2);
    b.short(p0, 2, 0x0112, 1);
    let g = b.gps(C_GPS);
    b.long(p0, 4, 0x8825, g as u32);
    let mut jpeg = vec![0xFF, 0xD8];
    jpeg.extend_from_slice(C_THUMB);
    jpeg.extend_from_slice(&[0xFF, 0xD9]);
    let at = b.alloc(K::Thumb, &jpeg);
    b.short(p1, 0, 0x0103, 6);
    b.long(p1, 1, 0x0201, at);
    b.long(p1, 2, 0x0202, jpeg.len() as u32);
    b.canaries.push((C_THUMB, Category::Thumbnail));
    b.finish(&name("thumb", be), false, true)
}

/// A DNG-shaped file: IFD0 (preview strip, DNG tags, serial, Exif), two
/// SubIFDs (a raw one with tiles, an Artist canary and its own GPS IFD; a
/// preview one with a strip).
pub fn dng(be: bool) -> Shape {
    let mut b = B::new(be);
    let ifd0 = b.ifd(9);
    let raw = b.ifd(7);
    let prev = b.ifd(5);
    let exif = b.ifd(2);
    b.set_ifd0(ifd0);
    b.text(ifd0, 0, 0x010F, C_MAKE, Some(Category::CameraModel));
    b.data_entries(ifd0, 1, 3, (0x0111, 0x0117), &[180], 4);
    b.short(ifd0, 2, 0x0112, 1);
    let mut d = Vec::new();
    for s in [raw as u32, prev as u32] {
        d.extend_from_slice(&b.w32(s));
    }
    b.put(ifd0, 4, 0x014A, LONG, 2, &d);
    b.long(ifd0, 5, 0x8769, exif as u32);
    b.entry(ifd0, 6, 0xC612, BYTE, 4, V::Inline(vec![1, 4, 0, 0]));
    let cm: Vec<u8> = (0..9u32)
        .flat_map(|i| [b.w32(i + 1), b.w32(10)].concat())
        .collect();
    b.put(ifd0, 7, 0xC621, SRATIONAL, 9, &cm);
    b.text(ifd0, 8, 0xC62F, C_DNG_SERIAL, Some(Category::DeviceId));
    b.short(raw, 0, 0x0103, 7);
    b.text(raw, 1, 0x013B, C_SUB_ARTIST, Some(Category::Authorship));
    b.short(raw, 2, 0x0142, 64);
    b.short(raw, 3, 0x0143, 64);
    b.data_entries(raw, 4, 5, (0x0144, 0x0145), &[1500, 1500, 1500], 11);
    let g = b.gps(C_SUB_GPS);
    b.long(raw, 6, 0x8825, g as u32);
    b.short(prev, 0, 0x0103, 7);
    b.data_entries(prev, 1, 4, (0x0111, 0x0117), &[400], 13);
    b.short(prev, 2, 0x0112, 1);
    b.short(prev, 3, 0x0115, 3);
    b.text(exif, 0, 0x9003, C_DTO, Some(Category::Timestamps));
    b.blob(exif, 1, 0x9286, C_COMMENT, Category::FreeText);
    b.finish(&name("dng", be), false, false)
}

pub fn all() -> Vec<Shape> {
    let mut v = Vec::new();
    for be in [false, true] {
        v.extend([stripped(be), tiled(be), multipage(be), thumb(be), dng(be)]);
    }
    v
}

/// IFD0 with strips and a 0xCD41 UNDEFINED entry holding a JUMBF box: in a
/// bare TIFF that tag is the only place JUMBF can be. Not in `all()` (no
/// category the audit sees names it, so the shape has no canary record).
pub fn jumbf(be: bool) -> Shape {
    let mut b = B::new(be);
    let ifd0 = b.ifd(3);
    b.set_ifd0(ifd0);
    b.data_entries(ifd0, 0, 1, (0x0111, 0x0117), &[300, 200], 3);
    // A well-formed box: a big-endian length that counts itself, the type, the
    // payload. (The length is the box's own, whatever the TIFF byte order.)
    let payload = b"jumbPROBE-C2PA-MANIFEST";
    let box_len = u32::try_from(payload.len() + 4).unwrap_or(0);
    let mut jumb = box_len.to_be_bytes().to_vec();
    jumb.extend_from_slice(payload);
    b.put(ifd0, 2, 0xCD41, UNDEFINED, jumb.len() as u32, &jumb);
    b.finish(&name("jumbf", be), false, false)
}

/// The structural attacks, each with the policy that makes it bite.
pub fn hostile() -> Vec<(&'static str, Vec<u8>)> {
    let mut out = Vec::new();
    // A strip whose offset is the GPS latitude value (offset known: the second
    // value of the GPS IFD of `tiled`, found by the part list).
    let t = tiled(false);
    let values: Vec<&Part> = t.parts.iter().filter(|p| p.kind == K::Value).collect();
    let gps_lat = values
        .iter()
        .find(|p| p.range.len() == 24)
        .map(|p| p.range.start)
        .unwrap_or(0);
    let mut over = t.bytes.clone();
    // Tile offsets array is the first LONGx4 value; point its first tile at the
    // GPS latitude, with a 24-byte count.
    let (off_at, cnt_at) = entry_value_at(&t, 0x0144, 0x0145);
    over[off_at..off_at + 4].copy_from_slice(&(gps_lat as u32).to_le_bytes());
    over[cnt_at..cnt_at + 4].copy_from_slice(&24u32.to_le_bytes());
    out.push(("strip-overlaps-gps-value", over));
    // A strip offset into IFD0's own table (rewritten when GPS goes).
    let mut into = t.bytes.clone();
    let ifd0 = 8usize.max(
        t.parts
            .iter()
            .find(|p| p.kind == K::Table)
            .map(|p| p.range.start)
            .unwrap_or(8),
    );
    into[off_at..off_at + 4].copy_from_slice(&((ifd0 + 14) as u32).to_le_bytes());
    into[cnt_at..cnt_at + 4].copy_from_slice(&24u32.to_le_bytes());
    out.push(("strip-offset-into-ifd-table", into));
    // A SubIFD pointer that loops back to IFD0.
    let d = dng(false);
    let mut cyc = d.bytes.clone();
    let sub_at = value_of_tag(&d, 0x014A);
    cyc[sub_at..sub_at + 4].copy_from_slice(&8u32.to_le_bytes());
    out.push(("subifd-cycle", cyc));
    // Truncated mid-file.
    out.push(("truncated", d.bytes[..d.bytes.len() / 2].to_vec()));
    out
}

/// Offsets of the out-of-line arrays of `(offsets tag, counts tag)` in IFD0 of
/// the little-endian shape (the first array, element 0).
fn entry_value_at(s: &Shape, off_tag: u16, cnt_tag: u16) -> (usize, usize) {
    (value_of_tag(s, off_tag), value_of_tag(s, cnt_tag))
}

/// The input offset of the first element of the out-of-line value of the first
/// entry with `tag` in a little-endian shape (IFD0 first).
fn value_of_tag(s: &Shape, tag: u16) -> usize {
    let b = &s.bytes;
    let mut ifd = u32::from_le_bytes([b[4], b[5], b[6], b[7]]) as usize;
    let n = usize::from(u16::from_le_bytes([b[ifd], b[ifd + 1]]));
    for i in 0..n {
        let e = ifd + 2 + i * 12;
        if u16::from_le_bytes([b[e], b[e + 1]]) == tag {
            return u32::from_le_bytes([b[e + 8], b[e + 9], b[e + 10], b[e + 11]]) as usize;
        }
    }
    ifd = 0;
    ifd
}
