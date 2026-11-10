//! Deterministic writer for the fuzz seeds (SPEC-006 `tiff_ifd`, SPEC-009
//! `jpeg_segments`).
//!
//! `cargo run --example fuzz_seeds` rewrites `fuzz/seeds/tiff_ifd/` and
//! `fuzz/seeds/jpeg_segments/`. Output is a
//! pure function of this file (no clock, no randomness, no environment), so two
//! runs produce byte-identical seeds. The seeds are hand-built, a few hundred
//! bytes each, and are this repository's own work: the shapes follow TIFF 6.0
//! section 2 (header, IFD, entry layout). std only; examples are not part of
//! the library lint set.

use std::fs;
use std::path::PathBuf;

// The full-house inputs and the report fixtures, shared with
// `tests/report_render.rs` so the seeds cannot drift from the tests.
#[path = "../tests/support/report_inputs.rs"]
mod report_inputs;

// SPEC-019: the synthetic real-world shapes (MPO, split APP13, APP10, COM, a
// trailer, a Canon-style note, a 64 KB Exif) become seeds named `shape-<name>`
// in `jpeg_segments` and `audit`, byte-identical to the fixtures.
#[path = "../tests/support/oracle_tiff.rs"]
mod oracle_tiff;
#[path = "../tests/support/real_shapes.rs"]
mod real_shapes;
// SPEC-020: the strip fixtures.
#[path = "../tests/support/strip_shapes.rs"]
mod strip_shapes;

const BYTE: u16 = 1;
const ASCII: u16 = 2;
const SHORT: u16 = 3;
const LONG: u16 = 4;
const RATIONAL: u16 = 5;
const SBYTE: u16 = 6;
const SSHORT: u16 = 8;
const SLONG: u16 = 9;
const SRATIONAL: u16 = 10;
const FLOAT: u16 = 11;
const DOUBLE: u16 = 12;
const SUB_IFDS: u16 = 330;
const EXIF_IFD: u16 = 0x8769;
const GPS_INFO: u16 = 0x8825;
const INTEROP_IFD: u16 = 0xA005;

/// What goes in an entry's 4-byte value field.
enum Val {
    /// Bytes written verbatim into the field (value stored inline).
    Inline([u8; 4]),
    /// Payload appended after the IFD; the field holds its offset.
    Ext(Vec<u8>),
    /// A literal offset, deliberately not backed by any data.
    Raw(u32),
    /// Offsets of other IFDs (by index). Sets the entry count itself.
    Refs(Vec<usize>),
}

struct Entry {
    tag: u16,
    typ: u16,
    count: u32,
    val: Val,
}

/// Where an IFD's chain pointer leads.
enum Next {
    End,
    Ifd(usize),
    Raw(u32),
}

struct Ifd {
    entries: Vec<Entry>,
    next: Next,
}

struct Writer {
    big: bool,
}

impl Writer {
    fn u16(&self, v: u16) -> [u8; 2] {
        if self.big {
            v.to_be_bytes()
        } else {
            v.to_le_bytes()
        }
    }

    fn u32(&self, v: u32) -> [u8; 4] {
        if self.big {
            v.to_be_bytes()
        } else {
            v.to_le_bytes()
        }
    }

    fn header(&self, ifd0: u32) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(if self.big { b"MM" } else { b"II" });
        out.extend_from_slice(&self.u16(42));
        out.extend_from_slice(&self.u32(ifd0));
        out
    }

    /// Lay the IFDs out one after another right behind the header, each
    /// followed by its external payloads. Sizes do not depend on offsets, so
    /// one pass computes every IFD's position and a second writes the bytes.
    fn build(&self, ifds: &[Ifd]) -> Vec<u8> {
        let mut starts = Vec::new();
        let mut pos: usize = 8;
        for ifd in ifds {
            starts.push(pos);
            pos += 2 + 12 * ifd.entries.len() + 4;
            for e in &ifd.entries {
                match &e.val {
                    Val::Ext(bytes) => pos += bytes.len(),
                    Val::Refs(r) if r.len() > 1 => pos += 4 * r.len(),
                    _ => {}
                }
            }
        }
        let off = |i: usize| u32::try_from(starts[i]).expect("seed fits u32");

        let mut out = self.header(off(0));
        for ifd in ifds {
            let mut table = Vec::new();
            let mut ext = Vec::new();
            let ext_base = out.len() + 2 + 12 * ifd.entries.len() + 4;
            table.extend_from_slice(&self.u16(u16::try_from(ifd.entries.len()).expect("few")));
            for e in &ifd.entries {
                let ext_at = |ext: &Vec<u8>| u32::try_from(ext_base + ext.len()).expect("u32");
                table.extend_from_slice(&self.u16(e.tag));
                table.extend_from_slice(&self.u16(e.typ));
                match &e.val {
                    Val::Inline(b) => {
                        table.extend_from_slice(&self.u32(e.count));
                        table.extend_from_slice(b);
                    }
                    Val::Ext(bytes) => {
                        table.extend_from_slice(&self.u32(e.count));
                        table.extend_from_slice(&self.u32(ext_at(&ext)));
                        ext.extend_from_slice(bytes);
                    }
                    Val::Raw(v) => {
                        table.extend_from_slice(&self.u32(e.count));
                        table.extend_from_slice(&self.u32(*v));
                    }
                    Val::Refs(r) => {
                        let n = u32::try_from(r.len()).expect("few");
                        table.extend_from_slice(&self.u32(n));
                        if let [only] = r.as_slice() {
                            table.extend_from_slice(&self.u32(off(*only)));
                        } else {
                            table.extend_from_slice(&self.u32(ext_at(&ext)));
                            for i in r {
                                ext.extend_from_slice(&self.u32(off(*i)));
                            }
                        }
                    }
                }
            }
            let next = match ifd.next {
                Next::End => 0,
                Next::Ifd(i) => off(i),
                Next::Raw(v) => v,
            };
            table.extend_from_slice(&self.u32(next));
            out.extend_from_slice(&table);
            out.extend_from_slice(&ext);
        }
        out
    }

    fn short_inline(&self, v: u16) -> Val {
        let mut b = [0u8; 4];
        b[..2].copy_from_slice(&self.u16(v));
        Val::Inline(b)
    }

    fn long_inline(&self, v: u32) -> Val {
        Val::Inline(self.u32(v))
    }

    fn rational(&self, num: u32, den: u32) -> Vec<u8> {
        let mut b = self.u32(num).to_vec();
        b.extend_from_slice(&self.u32(den));
        b
    }
}

fn entry(tag: u16, typ: u16, count: u32, val: Val) -> Entry {
    Entry {
        tag,
        typ,
        count,
        val,
    }
}

/// An IFD with a handful of well-formed entries: inline LONG/SHORT, external
/// SHORT array, ASCII, RATIONAL and SRATIONAL payloads.
fn valid(w: &Writer) -> Vec<u8> {
    let mut bps = w.u16(8).to_vec();
    bps.extend_from_slice(&w.u16(8));
    bps.extend_from_slice(&w.u16(8));
    let ifd = Ifd {
        entries: vec![
            entry(256, LONG, 1, w.long_inline(4)),
            entry(257, SHORT, 1, w.short_inline(2)),
            entry(258, SHORT, 3, Val::Ext(bps)),
            entry(270, ASCII, 6, Val::Ext(b"hello\0".to_vec())),
            entry(274, SHORT, 1, w.short_inline(6)),
            entry(282, RATIONAL, 1, Val::Ext(w.rational(72, 1))),
            entry(
                37380,
                SRATIONAL,
                1,
                Val::Ext(w.rational(u32::MAX - 2, 3)), // reads as -3/3 signed
            ),
        ],
        next: Next::End,
    };
    w.build(&[ifd])
}

fn small_ifd(w: &Writer, next: Next, extra: Vec<Entry>) -> Ifd {
    let mut entries = vec![entry(256, LONG, 1, w.long_inline(1))];
    entries.extend(extra);
    Ifd { entries, next }
}

fn subifd(w: &Writer) -> Vec<u8> {
    let ifds = [
        small_ifd(
            w,
            Next::End,
            vec![entry(SUB_IFDS, LONG, 2, Val::Refs(vec![1, 2]))],
        ),
        small_ifd(w, Next::End, vec![]),
        small_ifd(w, Next::End, vec![]),
    ];
    w.build(&ifds)
}

fn chain(w: &Writer) -> Vec<u8> {
    let ifds = [
        small_ifd(w, Next::Ifd(1), vec![]),
        small_ifd(w, Next::Ifd(2), vec![]),
        small_ifd(w, Next::End, vec![]),
    ];
    w.build(&ifds)
}

/// Header only to byte 5: the offset field is cut short.
fn truncated_header(w: &Writer) -> Vec<u8> {
    let mut b = w.header(8);
    b.truncate(5);
    b
}

fn ifd0_offset_zero(w: &Writer) -> Vec<u8> {
    w.header(0)
}

/// A table that declares 0xFFFF entries and carries three bytes of them.
fn entry_count_past_eof(w: &Writer) -> Vec<u8> {
    let mut b = w.header(8);
    b.extend_from_slice(&w.u16(0xFFFF));
    b.extend_from_slice(&[0, 1, 2]);
    b
}

/// Well-formed table whose external offsets all point past the end.
fn payload_past_eof(w: &Writer) -> Vec<u8> {
    let ifd = Ifd {
        entries: vec![
            entry(256, LONG, 1, w.long_inline(4)),
            entry(700, BYTE, 16, Val::Raw(0xFFFF_FF00)),
            entry(701, LONG, 64, Val::Raw(0x0000_0100)),
        ],
        next: Next::End,
    };
    w.build(&[ifd])
}

/// Counts whose `count * size` is enormous or overflows 32 bits. They sit on
/// non-SubIFD tags so the header still parses and the payload readers get
/// hit.
fn count_overflow(w: &Writer) -> Vec<u8> {
    let ifd = Ifd {
        entries: vec![
            entry(700, RATIONAL, u32::MAX, Val::Raw(16)),
            entry(701, LONG, u32::MAX, Val::Raw(16)),
            entry(702, SHORT, 0x8000_0000, Val::Raw(16)),
            entry(703, SRATIONAL, 0x2000_0001, Val::Raw(u32::MAX)),
            entry(704, BYTE, 0, Val::Raw(0)),
            entry(705, 99, 1, Val::Raw(0)),
        ],
        next: Next::End,
    };
    w.build(&[ifd])
}

/// A valid IFD whose chain pointer leads far past the end.
fn next_past_eof(w: &Writer) -> Vec<u8> {
    w.build(&[small_ifd(w, Next::Raw(0xFFFF_0000), vec![])])
}

/// IFD0's chain pointer leads back to itself.
fn self_cycle(w: &Writer) -> Vec<u8> {
    w.build(&[small_ifd(w, Next::Ifd(0), vec![])])
}

/// IFD0 -> SubIFD 1 -> SubIFD 0.
fn two_hop_cycle(w: &Writer) -> Vec<u8> {
    let ifds = [
        small_ifd(
            w,
            Next::End,
            vec![entry(SUB_IFDS, LONG, 1, Val::Refs(vec![1]))],
        ),
        small_ifd(
            w,
            Next::End,
            vec![entry(SUB_IFDS, LONG, 1, Val::Refs(vec![0]))],
        ),
    ];
    w.build(&ifds)
}

/// Twelve IFDs, each the SubIFD of the last: past the depth limit.
fn deep_nesting(w: &Writer) -> Vec<u8> {
    let n = 12;
    let ifds: Vec<Ifd> = (0..n)
        .map(|i| {
            let extra = if i + 1 < n {
                vec![entry(SUB_IFDS, LONG, 1, Val::Refs(vec![i + 1]))]
            } else {
                vec![]
            };
            small_ifd(w, Next::End, extra)
        })
        .collect();
    w.build(&ifds)
}

/// IFD0 is valid; its `SubIFDs` pointer leads far past the end. The tolerant
/// walk keeps IFD0 and reports one issue.
fn corrupt_subifd(w: &Writer) -> Vec<u8> {
    w.build(&[small_ifd(
        w,
        Next::End,
        vec![entry(SUB_IFDS, LONG, 1, Val::Raw(0xFFFF_FF00))],
    )])
}

/// `SubIFDs` carries a non-LONG type (SRATIONAL), so the pointer itself is
/// unreadable; IFD0's chain still leads to a valid IFD1.
fn malformed_subifds_tag(w: &Writer) -> Vec<u8> {
    let ifds = [
        small_ifd(
            w,
            Next::Ifd(1),
            vec![entry(SUB_IFDS, SRATIONAL, 1, Val::Raw(0))],
        ),
        small_ifd(w, Next::End, vec![]),
    ];
    w.build(&ifds)
}

/// Nine IFDs, each the SubIFD of the last: depths 0..=8, exactly at the depth
/// limit, deepest terminal. Clean, unlike `deep-nesting`.
fn depth_limit_exact(w: &Writer) -> Vec<u8> {
    let n = 9;
    let ifds: Vec<Ifd> = (0..n)
        .map(|i| {
            let extra = if i + 1 < n {
                vec![entry(SUB_IFDS, LONG, 1, Val::Refs(vec![i + 1]))]
            } else {
                vec![]
            };
            small_ifd(w, Next::End, extra)
        })
        .collect();
    w.build(&ifds)
}

/// A valid file with all three pointer IFDs: IFD0 -> Exif -> Interop, and
/// IFD0 -> GPS. Clean under the pointer-tag walk.
fn exif_gps_interop(w: &Writer) -> Vec<u8> {
    let ifds = [
        small_ifd(
            w,
            Next::End,
            vec![
                entry(EXIF_IFD, LONG, 1, Val::Refs(vec![1])),
                entry(GPS_INFO, LONG, 1, Val::Refs(vec![3])),
            ],
        ),
        small_ifd(
            w,
            Next::End,
            vec![entry(INTEROP_IFD, LONG, 1, Val::Refs(vec![2]))],
        ),
        small_ifd(w, Next::End, vec![]),
        small_ifd(w, Next::End, vec![]),
    ];
    w.build(&ifds)
}

/// The GPS pointer leads far past the end; IFD0 and the Exif IFD stay
/// readable and the walk records one issue.
fn gps_past_eof_exif_ok(w: &Writer) -> Vec<u8> {
    let ifds = [
        small_ifd(
            w,
            Next::End,
            vec![
                entry(EXIF_IFD, LONG, 1, Val::Refs(vec![1])),
                entry(GPS_INFO, LONG, 1, Val::Raw(0xFFFF_FF00)),
            ],
        ),
        small_ifd(w, Next::End, vec![]),
    ];
    w.build(&ifds)
}

/// The Exif pointer leads back to IFD0: a cycle through a pointer tag.
fn exif_points_at_ifd0(w: &Writer) -> Vec<u8> {
    w.build(&[small_ifd(
        w,
        Next::End,
        vec![entry(EXIF_IFD, LONG, 1, Val::Refs(vec![0]))],
    )])
}

/// A pointer tag with count 2 (must be exactly 1) and one with count 0.
fn pointer_bad_count(w: &Writer) -> Vec<u8> {
    w.build(&[small_ifd(
        w,
        Next::End,
        vec![
            entry(EXIF_IFD, LONG, 2, Val::Raw(0xFFFF_FF00)),
            entry(GPS_INFO, LONG, 0, Val::Raw(0)),
        ],
    )])
}

/// `SubIFDs` declares 100,000 offsets with no payload behind them: rejected by
/// the count cap before any offset is decoded.
fn subifds_count_over_cap(w: &Writer) -> Vec<u8> {
    w.build(&[small_ifd(
        w,
        Next::End,
        vec![entry(SUB_IFDS, LONG, 100_000, Val::Raw(0xFFFF_FF00))],
    )])
}

/// The Exif IFD carries a non-zero next-word leading to a valid IFD. Leaf
/// kinds never chain, so that IFD must not be followed.
fn leaf_next_word(w: &Writer) -> Vec<u8> {
    let ifds = [
        small_ifd(
            w,
            Next::End,
            vec![entry(EXIF_IFD, LONG, 1, Val::Refs(vec![1]))],
        ),
        small_ifd(w, Next::Ifd(2), vec![]),
        small_ifd(w, Next::End, vec![]),
    ];
    w.build(&ifds)
}

/// Up to four bytes stored inline, zero padded.
fn inline_bytes(b: &[u8]) -> Val {
    let mut v = [0u8; 4];
    v[..b.len()].copy_from_slice(b);
    Val::Inline(v)
}

/// ASCII entries in every hostile shape: count 0, unterminated, an embedded
/// NUL with a second string behind it, inline terminated values, and Latin-1
/// bytes that are not UTF-8.
fn ascii_shapes(w: &Writer) -> Vec<u8> {
    w.build(&[Ifd {
        entries: vec![
            entry(270, ASCII, 0, inline_bytes(b"")),
            entry(271, ASCII, 5, Val::Ext(b"Canon".to_vec())),
            entry(272, ASCII, 6, Val::Ext(b"ab\0cd\0".to_vec())),
            entry(305, ASCII, 4, inline_bytes(b"abc\0")),
            entry(306, ASCII, 3, inline_bytes(b"ab\0")),
            entry(315, ASCII, 1, inline_bytes(b"\0")),
            entry(
                33432,
                ASCII,
                8,
                Val::Ext(vec![0x43, 0xE9, 0xFF, 0xC3, 0x28, 0x00, 0x7F, 0x00]),
            ),
            // DateTimeOriginal, and a BYTE entry where ASCII would be
            // expected (the ASCII reader must refuse it).
            entry(
                0x9003,
                ASCII,
                20,
                Val::Ext(b"2024:01:02 03:04:05\0".to_vec()),
            ),
            entry(274, BYTE, 4, inline_bytes(b"abc\0")),
        ],
        next: Next::End,
    }])
}

/// SBYTE/SSHORT/SLONG extremes and FLOAT/DOUBLE specials (NaN with a payload,
/// infinity, subnormal, -0.0), each inline and external.
fn signed_and_float(w: &Writer) -> Vec<u8> {
    let sshort = |vals: &[i16]| -> Vec<u8> { vals.iter().flat_map(|v| w.u16(*v as u16)).collect() };
    let slong = |vals: &[i32]| -> Vec<u8> { vals.iter().flat_map(|v| w.u32(*v as u32)).collect() };
    let float_bits = |vals: &[u32]| -> Vec<u8> { vals.iter().flat_map(|v| w.u32(*v)).collect() };
    let double_bits = |vals: &[u64]| -> Vec<u8> {
        vals.iter()
            .flat_map(|v| {
                let (hi, lo) = ((*v >> 32) as u32, *v as u32);
                if w.big {
                    [w.u32(hi), w.u32(lo)]
                } else {
                    [w.u32(lo), w.u32(hi)]
                }
                .concat()
            })
            .collect()
    };
    w.build(&[Ifd {
        entries: vec![
            entry(1, SBYTE, 3, inline_bytes(&[0xFF, 0x7F, 0x80])),
            entry(2, SBYTE, 6, Val::Ext(vec![0x80, 1, 0xFF, 5, 0xFE, 3])),
            entry(3, SSHORT, 1, inline_bytes(&sshort(&[-32768]))),
            entry(4, SSHORT, 3, Val::Ext(sshort(&[-2, 32767, -32768]))),
            entry(5, SLONG, 1, inline_bytes(&slong(&[i32::MIN]))),
            entry(6, SLONG, 3, Val::Ext(slong(&[i32::MIN, -1, i32::MAX]))),
            entry(7, FLOAT, 1, inline_bytes(&float_bits(&[0x7FC0_1234]))),
            entry(
                8,
                FLOAT,
                5,
                Val::Ext(float_bits(&[
                    0x3FC0_0000,
                    0x8000_0000,
                    0x7F80_0000,
                    0x7FC0_1234,
                    0x0000_0001,
                ])),
            ),
            entry(
                9,
                DOUBLE,
                1,
                Val::Ext(double_bits(&[0x7FF8_0000_DEAD_BEEF])),
            ),
            entry(
                10,
                DOUBLE,
                5,
                Val::Ext(double_bits(&[
                    0x7E37_E43C_8800_759C,
                    0x8000_0000_0000_0000,
                    0xFFF0_0000_0000_0000,
                    0x7FF8_0000_DEAD_BEEF,
                    0x0000_0000_0000_0001,
                ])),
            ),
        ],
        next: Next::End,
    }])
}

/// ASCII counts that run past the end of the file, exceed the value cap, or
/// overflow `count * size`, next to one valid string.
fn ascii_count_past_eof(w: &Writer) -> Vec<u8> {
    w.build(&[Ifd {
        entries: vec![
            entry(271, ASCII, 64, Val::Raw(0xFFFF_FF00)),
            entry(272, ASCII, 1_048_577, Val::Raw(0)),
            entry(305, ASCII, u32::MAX, Val::Raw(0)),
            entry(306, ASCII, 5, Val::Ext(b"ok\0\0\0".to_vec())),
            entry(315, ASCII, 200, Val::Raw(8)),
        ],
        next: Next::End,
    }])
}

// ── JPEG seeds (SPEC-009) ────────────────────────────────────────────────────
// Hand-built from ITU-T T.81 Annex B marker syntax; this repository's own work.

fn jseg(marker: u8, payload: &[u8]) -> Vec<u8> {
    let len = u16::try_from(payload.len() + 2).expect("segment fits 16 bits");
    let mut v = vec![0xFF, marker];
    v.extend_from_slice(&len.to_be_bytes());
    v.extend_from_slice(payload);
    v
}

const JSOI: [u8; 2] = [0xFF, 0xD8];
const JEOI: [u8; 2] = [0xFF, 0xD9];

fn jsos() -> Vec<u8> {
    jseg(0xDA, &[1, 1, 0, 0, 0x3F, 0])
}

fn jcat(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

/// A valid TIFF for the SPEC-010 seeds: Make "DiamondCam", an Exif IFD with
/// DateTimeOriginal and a GPS IFD with GPSLatitudeRef.
fn make_and_gps(w: &Writer) -> Vec<u8> {
    let ifds = [
        Ifd {
            entries: vec![
                entry(271, ASCII, 11, Val::Ext(b"DiamondCam\0".to_vec())),
                entry(EXIF_IFD, LONG, 1, Val::Refs(vec![1])),
                entry(GPS_INFO, LONG, 1, Val::Refs(vec![2])),
            ],
            next: Next::End,
        },
        Ifd {
            entries: vec![entry(
                36867,
                ASCII,
                20,
                Val::Ext(b"2024:01:02 03:04:05\0".to_vec()),
            )],
            next: Next::End,
        },
        Ifd {
            entries: vec![entry(1, ASCII, 2, inline_bytes(b"N\0"))],
            next: Next::End,
        },
    ];
    w.build(&ifds)
}

/// One Photoshop image resource block, padded by the format's own rules.
fn jres(id: u16, name: &[u8], data: &[u8]) -> Vec<u8> {
    let mut v = b"8BIM".to_vec();
    v.extend_from_slice(&id.to_be_bytes());
    v.push(u8::try_from(name.len()).expect("short name"));
    v.extend_from_slice(name);
    if (1 + name.len()) % 2 == 1 {
        v.push(0);
    }
    v.extend_from_slice(&u32::try_from(data.len()).expect("short").to_be_bytes());
    v.extend_from_slice(data);
    if data.len() % 2 == 1 {
        v.push(0);
    }
    v
}

fn japp(marker: u8, sig: &[u8], rest: &[u8]) -> Vec<u8> {
    jseg(marker, &[sig, rest].concat())
}

fn jpeg_seeds() -> Vec<(&'static str, Vec<u8>)> {
    let mut app0 = b"JFIF\0".to_vec();
    app0.extend_from_slice(&[1, 1, 0, 0, 1, 0, 1, 0, 0]);
    let exif = jseg(0xE1, b"Exif\0\0II*\0\x08\0\0\0");
    let xmp = jseg(0xE1, b"http://ns.adobe.com/xap/1.0/\0<x:xmpmeta/>");
    let ext = |tag: &[u8]| {
        let mut p = b"http://ns.adobe.com/xmp/extension/\0".to_vec();
        p.extend_from_slice(tag);
        jseg(0xE1, &p)
    };
    let com = jseg(0xFE, b"hi");
    let tiff_ok = make_and_gps(&Writer { big: false });
    let exif_seg = |tiff: &[u8]| japp(0xE1, b"Exif\0\0", tiff);
    // One extended-XMP chunk: a 32-byte GUID, the full length, this chunk's
    // offset, then the data.
    let ext_chunk = |offset: u32, data: &[u8]| {
        let mut rest = b"0123456789ABCDEF0123456789ABCDEF".to_vec();
        rest.extend_from_slice(&16u32.to_be_bytes());
        rest.extend_from_slice(&offset.to_be_bytes());
        rest.extend_from_slice(data);
        ext(&rest)
    };
    let mut seventeen_exif = JSOI.to_vec();
    for _ in 0..17 {
        seventeen_exif.extend_from_slice(&exif_seg(&tiff_ok));
    }
    seventeen_exif.extend_from_slice(&jsos());
    seventeen_exif.extend_from_slice(&JEOI);
    let scan: &[u8] = &[0x12, 0xFF, 0x00, 0x34, 0xFF, 0xD0, 0x56];

    let minimal = jcat(&[
        &JSOI,
        &jseg(0xE0, &app0),
        &jseg(0xDB, &[0, 1, 2, 3]),
        &jseg(0xC0, &[8, 0, 1, 0, 1, 1, 1, 0x11, 0]),
        &jseg(0xC4, &[0, 1, 2, 3]),
        &jsos(),
        scan,
        &JEOI,
    ]);

    let mut many_segments = JSOI.to_vec();
    for _ in 0..4100 {
        many_segments.extend_from_slice(&jseg(0xFE, b""));
    }
    many_segments.extend_from_slice(&jsos());
    many_segments.extend_from_slice(&JEOI);

    let mut many_ff00 = JSOI.to_vec();
    for _ in 0..1100 {
        many_ff00.extend_from_slice(&[0xFF, 0x00]);
    }

    let mut ffff = JSOI.to_vec();
    ffff.extend_from_slice(&[0xFF, 0xE1, 0xFF, 0xFF]);
    ffff.extend_from_slice(&vec![0x5A; 65_533]);
    ffff.extend_from_slice(&jsos());
    ffff.extend_from_slice(&JEOI);

    vec![
        ("minimal", minimal),
        ("soi-only", JSOI.to_vec()),
        (
            "fill-bytes",
            jcat(&[
                &JSOI,
                &[0xFF, 0xFF, 0xFF],
                &jseg(0xE0, &app0),
                &jsos(),
                &JEOI,
            ]),
        ),
        (
            "app1-len-0",
            jcat(&[&JSOI, &[0xFF, 0xE1, 0, 0], &com, &jsos(), &JEOI]),
        ),
        (
            "app1-len-1",
            jcat(&[&JSOI, &[0xFF, 0xE1, 0, 1], &com, &jsos(), &JEOI]),
        ),
        (
            "app1-len-2",
            jcat(&[&JSOI, &jseg(0xE1, b""), &jsos(), &JEOI]),
        ),
        ("app1-len-ffff", ffff),
        (
            "app1-past-eof",
            jcat(&[&JSOI, &[0xFF, 0xE1, 0x01, 0x00], &[0; 10]]),
        ),
        (
            "exif-and-xmp",
            jcat(&[&JSOI, &exif, &xmp, &jsos(), scan, &JEOI]),
        ),
        (
            "extended-xmp-pair",
            jcat(&[
                &JSOI,
                &xmp,
                &ext(b"guid-a"),
                &ext(b"guid-b"),
                &jsos(),
                &JEOI,
            ]),
        ),
        (
            "stray-bytes",
            jcat(&[&JSOI, &com, &[1, 2, 3], &com, &[9; 5], &jsos(), &JEOI]),
        ),
        (
            "ff00-stray",
            jcat(&[
                &JSOI,
                &[0xFF, 0x00],
                &[0xFF, 0xD8],
                &[0xFF, 0xD3],
                &[0xFF, 0x01],
                &com,
                &jsos(),
                &JEOI,
            ]),
        ),
        ("no-sos", jcat(&[&JSOI, &jseg(0xE0, &app0), &com, &JEOI])),
        ("sos-no-eoi", jcat(&[&JSOI, &jsos(), scan])),
        (
            "progressive-two-scans",
            jcat(&[
                &JSOI,
                &jseg(0xC2, &[8, 0, 1, 0, 1, 1, 1, 0x11, 0]),
                &jsos(),
                scan,
                &jseg(0xC4, &[0, 1, 2]),
                &jsos(),
                &[0xFF, 0xFF, 0xD3, 7],
                &JEOI,
            ]),
        ),
        (
            "trailer-after-eoi",
            jcat(&[&JSOI, &jsos(), scan, &JEOI, b"TRAILER", &JEOI, &com]),
        ),
        (
            "app-after-scan",
            jcat(&[&JSOI, &jsos(), scan, &exif, &JEOI]),
        ),
        ("many-segments", many_segments),
        ("many-ff00", many_ff00),
        (
            "icc-iptc-jumbf",
            jcat(&[
                &JSOI,
                &jseg(0xE2, b"ICC_PROFILE\0\x01\x01profile"),
                &jseg(0xED, b"Photoshop 3.0\08BIM"),
                &jseg(0xEB, b"JP\0\0jumb"),
                &jsos(),
                &JEOI,
            ]),
        ),
        (
            "signatures-under-wrong-marker",
            jcat(&[
                &JSOI,
                &jseg(0xE2, b"Exif\0\0II"),
                &jseg(0xE1, b"ICC_PROFILE\0\x01\x01"),
                &jseg(0xEC, b"Photoshop 3.0\08BIM"),
                &jseg(0xEA, b"JP\0\0jumb"),
                &jseg(0xEB, b"XX\0\0nojp"),
                &jseg(0xE1, b"JFIF\0xx"),
                &jseg(0xE2, b"http://ns.adobe.com/xap/1.0/\0<x/>"),
                &jseg(0xE0, b"Exif\0\0II"),
                &jseg(0xED, b"Photoshop 2.5\0"),
                &jsos(),
                &JEOI,
            ]),
        ),
        // ── SPEC-010: Exif hand-off and located regions ─────────────────────
        (
            "exif-in-jpeg",
            jcat(&[&JSOI, &exif_seg(&tiff_ok), &jsos(), scan, &JEOI]),
        ),
        (
            "two-exif",
            jcat(&[
                &JSOI,
                &exif_seg(&tiff_ok),
                &exif_seg(&valid(&Writer { big: true })),
                &jsos(),
                &JEOI,
            ]),
        ),
        (
            "exif-bad-tiff",
            jcat(&[&JSOI, &exif_seg(b"junkjunk"), &jsos(), &JEOI]),
        ),
        (
            "exif-after-scan",
            jcat(&[&JSOI, &jsos(), scan, &exif_seg(&tiff_ok), &JEOI]),
        ),
        (
            "xmp-and-extended-xmp",
            jcat(&[
                &JSOI,
                &xmp,
                &ext_chunk(5, b"first"),
                &ext_chunk(10, b"second"),
                &jsos(),
                &JEOI,
            ]),
        ),
        (
            "extended-xmp-short",
            jcat(&[&JSOI, &ext(&[7u8; 39]), &ext(&[7u8; 40]), &jsos(), &JEOI]),
        ),
        (
            "icc-chunks-ok",
            jcat(&[
                &JSOI,
                &japp(0xE2, b"ICC_PROFILE\0", b"\x01\x02first"),
                &japp(0xE2, b"ICC_PROFILE\0", b"\x02\x02second"),
                &jsos(),
                &JEOI,
            ]),
        ),
        (
            "icc-chunks-mismatch",
            jcat(&[
                &JSOI,
                &japp(0xE2, b"ICC_PROFILE\0", b"\x01\x03first"),
                &japp(0xE2, b"ICC_PROFILE\0", b"\x01\x02again"),
                &japp(0xE2, b"ICC_PROFILE\0", b"\x00"),
                &jsos(),
                &JEOI,
            ]),
        ),
        (
            "iptc-8bim",
            jcat(&[
                &JSOI,
                &japp(
                    0xED,
                    b"Photoshop 3.0\0",
                    &[
                        jres(0x0408, b"ab", &[9, 9, 9]),
                        jres(0x0404, b"", &[1, 2, 3, 4, 5]),
                    ]
                    .concat(),
                ),
                &jsos(),
                &JEOI,
            ]),
        ),
        (
            "iptc-truncated-resource",
            jcat(&[
                &JSOI,
                &japp(
                    0xED,
                    b"Photoshop 3.0\0",
                    &[
                        jres(0x0404, b"", &[1, 2]),
                        b"8BIM\x04\x04\0\0\0\0\x01\0abc".to_vec(),
                    ]
                    .concat(),
                ),
                &jsos(),
                &JEOI,
            ]),
        ),
        (
            "jumbf-header",
            jcat(&[
                &JSOI,
                &japp(0xEB, b"JP", b"\0\x07\x01\x02\x03\x04jumb"),
                &jsos(),
                &JEOI,
            ]),
        ),
        (
            "jumbf-short",
            jcat(&[
                &JSOI,
                &japp(0xEB, b"JP", b"\0\x07\x01\x02\x03"),
                &jsos(),
                &JEOI,
            ]),
        ),
        ("seventeen-exif", seventeen_exif),
        // ── SPEC-011 ────────────────────────────────────────────────────────
        // DEC-007: `Exif\0` and a NON-zero sixth byte, then a valid TIFF. An
        // Exif block (ExifTool 13.55 reads it too).
        (
            "exif-sixth-byte-nonzero",
            jcat(&[
                &JSOI,
                &japp(0xE1, b"Exif\0\x01", &tiff_ok),
                &jsos(),
                scan,
                &JEOI,
            ]),
        ),
        // DEC-007: an APP1 payload of exactly the five signature bytes is not
        // Exif (it has no sixth byte, so no TIFF data position).
        (
            "exif-five-byte-payload",
            jcat(&[&JSOI, &jseg(0xE1, b"Exif\0"), &jsos(), &JEOI]),
        ),
        // SPEC-010 verify I1: a single ICC chunk whose sequence number is 0
        // (sequence numbers are 1-based): one IccChunkMismatch.
        (
            "icc-seq-zero",
            jcat(&[
                &JSOI,
                &japp(0xE2, b"ICC_PROFILE\0", b"\x00\x01only"),
                &jsos(),
                &JEOI,
            ]),
        ),
    ]
}

// ── audit seeds (SPEC-012) ───────────────────────────────────────────────────

/// A TIFF with Make, an Exif IFD, a GPS IFD and an IFD1 thumbnail: the thumbnail
/// pointer and length are real (`FF D8 FF D9` at the end of the blob).
fn tiff_gps_and_thumbnail(w: &Writer) -> Vec<u8> {
    let thumb = [0xFF, 0xD8, 0xFF, 0xD9];
    let ifds = |thumb_at: u32| {
        vec![
            Ifd {
                entries: vec![
                    entry(271, ASCII, 11, Val::Ext(b"DiamondCam\0".to_vec())),
                    entry(EXIF_IFD, LONG, 1, Val::Refs(vec![1])),
                    entry(GPS_INFO, LONG, 1, Val::Refs(vec![2])),
                ],
                next: Next::Ifd(3),
            },
            Ifd {
                entries: vec![entry(
                    36867,
                    ASCII,
                    20,
                    Val::Ext(b"2024:01:02 03:04:05\0".to_vec()),
                )],
                next: Next::End,
            },
            Ifd {
                entries: vec![
                    entry(1, ASCII, 2, inline_bytes(b"N\0")),
                    entry(3, ASCII, 2, inline_bytes(b"E\0")),
                ],
                next: Next::End,
            },
            Ifd {
                entries: vec![
                    entry(0x0201, LONG, 1, Val::Raw(thumb_at)),
                    entry(0x0202, LONG, 1, w.long_inline(4)),
                ],
                next: Next::End,
            },
        ]
    };
    // IFD1 is last and has no external payload, so the thumbnail goes right
    // after the built bytes, and the length does not depend on the pointer.
    let len = u32::try_from(w.build(&ifds(0)).len()).expect("small");
    let mut out = w.build(&ifds(len));
    out.extend_from_slice(&thumb);
    out
}

/// A TIFF whose IFD1 is an uncompressed thumbnail: strip offsets and byte counts
/// and no JPEG pointer (DEC-008: a `Thumbnail` finding on the `0x0111` entry).
fn tiff_ifd1_strips(w: &Writer) -> Vec<u8> {
    w.build(&[
        Ifd {
            entries: vec![entry(271, ASCII, 11, Val::Ext(b"DiamondCam\0".to_vec()))],
            next: Next::Ifd(1),
        },
        Ifd {
            entries: vec![
                entry(0x0111, LONG, 1, w.long_inline(8)),
                entry(0x0117, LONG, 1, w.long_inline(4)),
            ],
            next: Next::End,
        },
    ])
}

/// A classic TIFF whose IFD0 names `n` overlapping SubIFDs two bytes apart, each
/// declaring 767 entries over the same repeating bytes: the memory-amplification
/// shape of `tests/audit_oracle.rs` at 1/85 of the entries per IFD (about
/// 10.8 KB instead of 790 KB), small enough to survive the fuzzer.
fn many_overlapping_ifds(n: usize) -> Vec<u8> {
    const COUNT: usize = 0x02FF;
    let array_at = 8 + 2 + 12 + 4;
    let base = array_at + 4 * n;
    let base = base + base % 2;
    let mut out = b"II\x2A\x00\x08\x00\x00\x00".to_vec();
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&SUB_IFDS.to_le_bytes());
    out.extend_from_slice(&LONG.to_le_bytes());
    out.extend_from_slice(&u32::try_from(n).expect("few").to_le_bytes());
    out.extend_from_slice(&u32::try_from(array_at).expect("small").to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    for k in 0..n {
        out.extend_from_slice(&u32::try_from(base + 2 * k).expect("small").to_le_bytes());
    }
    out.resize(base, 0);
    // The repeating two bytes `FF 02` put the entry count 0x02FF at every even
    // offset of the region.
    let region = 2 * (n - 1) + 2 + 12 * COUNT + 4;
    for i in 0..region {
        out.push(if i % 2 == 0 { 0xFF } else { 0x02 });
    }
    // Zero the next-IFD words so no IFD chains on.
    let next_words = base + 2 + 12 * COUNT;
    for b in out.iter_mut().skip(next_words) {
        *b = 0;
    }
    out
}

/// SPEC-017: IFD0 (one `SubIFDs` entry) naming two sub-IFDs two bytes apart,
/// each declaring `c` entries over the same `0xFF` bytes. The file is
/// `42 + 12 c` bytes, so the entry budget (`len / 12`) is `c + 3` and the walk
/// wants `1 + 2 c`: `c = 2` lands exactly on it (clean), `c = 3` is one over.
fn budget_pair(c: u16) -> Vec<u8> {
    let (array_at, base) = (26usize, 34usize);
    let mut out = b"II\x2A\x00\x08\x00\x00\x00".to_vec();
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&SUB_IFDS.to_le_bytes());
    out.extend_from_slice(&LONG.to_le_bytes());
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&u32::try_from(array_at).expect("small").to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    for k in 0..2usize {
        out.extend_from_slice(&u32::try_from(base + 2 * k).expect("small").to_le_bytes());
    }
    out.resize(base + 2 + 2 + 12 * usize::from(c) + 4, 0xFF);
    // Zero both next-words (the tail after the first table's entries).
    let next_words = base + 2 + 12 * usize::from(c);
    for b in out.iter_mut().skip(next_words) {
        *b = 0;
    }
    for k in 0..2usize {
        let at = base + 2 * k;
        out[at..at + 2].copy_from_slice(&c.to_le_bytes());
    }
    out
}

/// SPEC-017: a reduced hostile shape that crosses the entry budget. IFD0 names
/// `n` sub-IFDs two bytes apart, each declaring 255 entries over the same
/// repeating bytes: about 3.3 KB for `n = 40`, budget 276, so IFD0 (1) and the
/// first sub-IFD (255) fit and the second is omitted with `TooManyEntries`.
fn overlapping_subifds(n: usize) -> Vec<u8> {
    const COUNT: usize = 0x00FF;
    let array_at = 8 + 2 + 12 + 4;
    let base = array_at + 4 * n;
    let base = base + base % 2;
    let mut out = b"II\x2A\x00\x08\x00\x00\x00".to_vec();
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&SUB_IFDS.to_le_bytes());
    out.extend_from_slice(&LONG.to_le_bytes());
    out.extend_from_slice(&u32::try_from(n).expect("few").to_le_bytes());
    out.extend_from_slice(&u32::try_from(array_at).expect("small").to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    for k in 0..n {
        out.extend_from_slice(&u32::try_from(base + 2 * k).expect("small").to_le_bytes());
    }
    out.resize(base, 0);
    // The repeating two bytes `FF 00` put the entry count 0x00FF at every even
    // offset of the region.
    let region = 2 * (n - 1) + 2 + 12 * COUNT + 4;
    for i in 0..region {
        out.push(if i % 2 == 0 { 0xFF } else { 0x00 });
    }
    // Zero the next-IFD words so no IFD chains on.
    let next_words = base + 2 + 12 * COUNT;
    for b in out.iter_mut().skip(next_words) {
        *b = 0;
    }
    out
}

/// The `audit` target's seeds: every TIFF and JPEG seed of the other two
/// targets (copied under a `tiff-` / `jpeg-` prefix) and the SPEC-012 shapes.
fn audit_seeds(
    tiff: &[(&str, Vec<u8>)],
    jpeg: &[(&'static str, Vec<u8>)],
) -> Vec<(String, Vec<u8>)> {
    let le = Writer { big: false };
    let mut out: Vec<(String, Vec<u8>)> = Vec::new();
    for (n, b) in tiff {
        out.push((format!("tiff-{n}"), b.clone()));
    }
    for (n, b) in jpeg {
        out.push((format!("jpeg-{n}"), b.clone()));
    }
    let exif = |tiff: &[u8]| japp(0xE1, b"Exif\0\0", tiff);
    let gps_thumb = tiff_gps_and_thumbnail(&le);
    let scan: &[u8] = &[0x12, 0x34];
    let new: Vec<(&str, Vec<u8>)> = vec![
        (
            "audit-png-magic",
            [
                &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A][..],
                b"\0\0\0\rIHDR",
            ]
            .concat(),
        ),
        (
            "audit-bigtiff-magic",
            b"II\x2B\x00\x08\x00\x00\x00\x10\0\0\0\0\0\0\0".to_vec(),
        ),
        // Zero bytes: libFuzzer accepts an empty seed file.
        ("audit-empty-marker", Vec::new()),
        (
            "audit-exif-gps-and-thumbnail",
            jcat(&[&JSOI, &exif(&gps_thumb), &jsos(), scan, &JEOI]),
        ),
        (
            "audit-two-blocks-same-tag",
            jcat(&[
                &JSOI,
                &exif(&make_and_gps(&le)),
                &exif(&make_and_gps(&Writer { big: true })),
                &jsos(),
                scan,
                &JEOI,
            ]),
        ),
        ("audit-ifd1-strips", tiff_ifd1_strips(&le)),
        ("audit-many-overlapping-ifds", many_overlapping_ifds(255)),
        // SPEC-015 (carried SPEC-014 F1): the two inputs that carry every
        // category, so the preview's may-hold relation is in the corpus.
        ("audit-full-house-tiff", report_inputs::full_house_tiff()),
        ("audit-full-house-jpeg", report_inputs::full_house_jpeg()),
    ];
    out.extend(new.into_iter().map(|(n, b)| (n.to_string(), b)));
    out
}

/// One `audit_render` input: the options byte, the label length, the label,
/// then the file (see `fuzz/shared/render_invariants.rs`, `build`).
fn render_seed(opts: u8, label: &[u8], file: &[u8]) -> Vec<u8> {
    let mut v = vec![opts, u8::try_from(label.len()).expect("label fits a byte")];
    v.extend_from_slice(label);
    v.extend_from_slice(file);
    v
}

/// The `audit_render` target's seeds (SPEC-015).
fn render_seeds() -> Vec<(String, Vec<u8>)> {
    // Option bits: low 3 = policy (1 gps, 2 gps_and_device, 3 all_pii, 4 all,
    // 5 everything, 6 custom), 8 inventory, 16 values, 32/64 failure reason.
    const INV: u8 = 8;
    const VAL: u8 = 16;
    let house = report_inputs::full_house_jpeg();
    let house_tiff = report_inputs::full_house_tiff();
    let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0];
    let long_ascii = {
        let payload: Vec<u8> = (0..300u32).map(|i| b'a' + (i % 26) as u8).collect();
        report_inputs::tiff_with_payloads(&[(0x010F, 2, &payload)])
    };
    let mut label255 = Vec::new();
    for i in 0..255u32 {
        label255.push(match i % 5 {
            0 => b'=',
            1 => b',',
            2 => b'"',
            3 => 0xC3,
            _ => (0x20 + i % 90) as u8,
        });
    }
    let secret = report_inputs::secret_tiff();
    // A Make whose value holds a semicolon (a spreadsheet in a semicolon
    // locale splits on it).
    let semi = report_inputs::tiff_with_payloads(&[(0x010F, 2, b"x;=1+cmd\0")]);
    let seeds: Vec<(&str, Vec<u8>)> = vec![
        (
            "full-house-jpeg-all-pii-inventory-values",
            render_seed(3 | INV | VAL, b"photos/full-house.jpg", &house),
        ),
        (
            "full-house-tiff-gps-values",
            render_seed(1 | VAL, b"photos/full-house.tif", &house_tiff),
        ),
        (
            "full-house-jpeg-everything-inventory",
            render_seed(5 | INV | 32, b"a.jpg", &house),
        ),
        (
            "full-house-jpeg-custom-policy",
            render_seed(6 | VAL | 64, &[0x5A], &house),
        ),
        (
            "full-house-jpeg-no-policy",
            render_seed(0, b"plain.jpg", &house),
        ),
        (
            "label-control-characters",
            render_seed(2 | VAL, b"a\x01\x1b[31mred\x07\x00b\x7f", &secret),
        ),
        (
            "label-quotes-comma",
            render_seed(4 | INV, b"a,\"b\",c", &house_tiff),
        ),
        ("label-cr-lf", render_seed(INV, b"x\r\ny\rz\nw", &house)),
        (
            "label-invalid-utf8",
            render_seed(3 | VAL, b"\xFF\xFE\xC3(\xE2\x82", &secret),
        ),
        (
            "label-invisible-characters",
            render_seed(
                3 | VAL,
                "a\u{200B}\u{FEFF}\u{AD}\u{E0041}b".as_bytes(),
                &secret,
            ),
        ),
        (
            "label-right-to-left-override",
            render_seed(VAL, "\u{202E}gpj.exe\u{2028}\u{85}".as_bytes(), &house_tiff),
        ),
        (
            "label-formula-equals",
            render_seed(3 | VAL, b"=cmd|' /C calc'!A0", &secret),
        ),
        (
            "label-formula-at",
            render_seed(INV | VAL, b"@SUM(1)", &secret),
        ),
        (
            "label-255-bytes",
            render_seed(5 | INV | VAL, &label255, &house),
        ),
        (
            "label-semicolon-formula",
            render_seed(3 | VAL, b"a;=1+cmd|' /C calc'!A0", &semi),
        ),
        ("label-empty-png", render_seed(3, b"", &png)),
        (
            "ascii-payload-300-bytes",
            render_seed(3 | INV | VAL, b"long.tif", &long_ascii),
        ),
        ("png-signature-values", render_seed(5 | VAL, b"s.png", &png)),
        ("empty-file", render_seed(1 | INV | VAL, b"empty", b"")),
        // SPEC-019: the Multi-Picture file and the split Photoshop file under
        // several option bytes (policies, inventory, values, failure bits).
        (
            "shape-mpo-all-pii-inventory-values",
            render_seed(
                3 | INV | VAL,
                b"photos/nikon.mpo",
                &real_shapes::mpo_two(false),
            ),
        ),
        (
            "shape-mpo-gps-values",
            render_seed(1 | VAL, b"n.mpo", &real_shapes::mpo_two(true)),
        ),
        (
            "shape-mpo-everything-inventory",
            render_seed(5 | INV, b"e.mpo", &real_shapes::mpo_two(false)),
        ),
        (
            "shape-mpo-custom-failure",
            render_seed(6 | INV | 32, &[0x41], &real_shapes::mpo_two(false)),
        ),
        (
            "shape-split-app13-all-inventory-values",
            render_seed(4 | INV | VAL, b"split.jpg", &real_shapes::split_app13()),
        ),
        (
            "shape-split-app13-gps",
            render_seed(2, b"s.jpg", &real_shapes::split_app13()),
        ),
        (
            "shape-trailer-no-policy-inventory",
            render_seed(INV, b"leica.jpg", &real_shapes::trailer_313()),
        ),
    ];
    seeds
        .into_iter()
        .map(|(n, b)| (format!("render-{n}"), b))
        .collect()
}

/// One `cli_args` input: the arguments joined with NUL (see
/// `fuzz/shared/cli_invariants.rs`, `split_args`). Zero bytes are not loaded
/// by libFuzzer, so "no arguments" is not a seed; a lone NUL is two empty
/// arguments.
fn cli_seed(args: &[&[u8]]) -> Vec<u8> {
    args.join(&0u8)
}

/// The `cli_args` target's seeds (SPEC-016): the DEC-011 grammar.
fn cli_seeds() -> Vec<(String, Vec<u8>)> {
    let long = {
        let mut v = b"--".to_vec();
        v.extend(std::iter::repeat_n(b'a', 400));
        v
    };
    let all: Vec<(&str, Vec<u8>)> = vec![
        ("cli-defaults", cli_seed(&[b"dir"])),
        ("cli-two-paths", cli_seed(&[b"a", b"b"])),
        (
            "cli-every-option",
            cli_seed(&[
                b"--format",
                b"json",
                b"--policy",
                b"gps",
                b"--fail-on",
                b"low",
                b"--strict",
                b"--values",
                b"--inventory",
                b"--max-size",
                b"512K",
                b"a",
                b"b",
            ]),
        ),
        (
            "cli-equals-forms",
            cli_seed(&[
                b"--format=csv",
                b"--policy=none",
                b"--fail-on=none",
                b"--max-size=1G",
                b"p",
            ]),
        ),
        (
            "cli-json-shorthand",
            cli_seed(&[b"--json", b"--policy=all-pii", b"p"]),
        ),
        (
            "cli-csv-shorthand",
            cli_seed(&[b"p", b"--csv", b"--fail-on", b"medium"]),
        ),
        (
            "cli-double-dash",
            cli_seed(&[b"--strict", b"--", b"--json", b"-", b"-h", b"--"]),
        ),
        (
            "cli-repeat-option",
            cli_seed(&[b"--policy", b"gps", b"--policy", b"all", b"p"]),
        ),
        (
            "cli-repeat-flag",
            cli_seed(&[b"--strict", b"p", b"--strict"]),
        ),
        (
            "cli-json-csv-conflict",
            cli_seed(&[b"--json", b"--csv", b"p"]),
        ),
        (
            "cli-format-json-conflict",
            cli_seed(&[b"--format", b"text", b"--json", b"p"]),
        ),
        ("cli-size-zero", cli_seed(&[b"--max-size", b"0", b"p"])),
        (
            "cli-size-overflow",
            cli_seed(&[b"--max-size", b"18446744073709551616", b"p"]),
        ),
        (
            "cli-size-lowercase-suffix",
            cli_seed(&[b"--max-size", b"1k", b"p"]),
        ),
        (
            "cli-size-largest-fit",
            cli_seed(&[b"--max-size", b"17179869183G", b"p"]),
        ),
        ("cli-lone-dash", cli_seed(&[b"-"])),
        (
            "cli-help-after-invalid",
            cli_seed(&[b"--bogus", b"--format", b"xml", b"--help"]),
        ),
        ("cli-version", cli_seed(&[b"-V", b"p"])),
        ("cli-help-and-version", cli_seed(&[b"--version", b"-h"])),
        (
            "cli-help-after-dash-dash",
            cli_seed(&[b"--", b"--help", b"-V"]),
        ),
        ("cli-no-path", cli_seed(&[b"--strict", b"--values"])),
        ("cli-missing-value", cli_seed(&[b"p", b"--format"])),
        ("cli-flag-with-value", cli_seed(&[b"--strict=1", b"p"])),
        (
            "cli-value-looks-like-option",
            cli_seed(&[b"--policy", b"--json", b"p"]),
        ),
        ("cli-unknown-option", cli_seed(&[b"-x", b"p"])),
        (
            "cli-hostile-bytes",
            cli_seed(&[b"--\xFF\xFE", b"\x1B[31m", b"--format=\xE2\x82", b"\xFF"]),
        ),
        (
            "cli-non-utf8-value",
            cli_seed(&[b"--format", b"\xFF", b"p"]),
        ),
        ("cli-two-empty-arguments", cli_seed(&[b"", b""])),
        ("cli-empty-path-and-option", cli_seed(&[b"", b"--strict"])),
        ("cli-long-unknown-option", cli_seed(&[&long, b"p"])),
    ];
    all.into_iter().map(|(n, b)| (n.to_string(), b)).collect()
}

/// SPEC-020: the seeds of `rewrite_jpeg`. Byte 0 selects the policy (4
/// `Everything`, 7 `All`, 6 a custom set from the 16-bit little-endian mask in
/// bytes 1 and 2, bit i meaning `Category::ALL[i]`). Every SPEC-019 shape and
/// every strip fixture appears under `All`, `Everything` and one custom mask
/// (rotated through `MASKS`); a few presets and the empty set come on top.
fn rewrite_seeds() -> Vec<(String, Vec<u8>)> {
    // Xmp+Icc, EmbeddedImage, OtherSegment, Iptc+FreeText, Jumbf+Xmp+Icc,
    // EmbeddedImage+OtherSegment+FreeText.
    const MASKS: [u16; 6] = [0x1040, 0x2000, 0x4000, 0x0088, 0x1140, 0x6008];
    let mut inputs: Vec<(String, Vec<u8>)> = real_shapes::fixtures()
        .into_iter()
        .map(|(n, b)| (format!("shape-{n}"), b))
        .collect();
    inputs.extend(
        strip_shapes::all()
            .into_iter()
            .map(|s| (format!("strip-{}", s.name), s.bytes)),
    );
    let mut out = Vec::new();
    for (i, (name, bytes)) in inputs.iter().enumerate() {
        let mask = MASKS[i % MASKS.len()];
        let mut with = |label: String, head: Vec<u8>| {
            let mut v = head;
            v.extend_from_slice(bytes);
            out.push((format!("{label}-{name}"), v));
        };
        with("all".to_string(), vec![7]);
        with("everything".to_string(), vec![4]);
        let m = mask.to_le_bytes();
        with(format!("mask{mask:04x}"), vec![6, m[0], m[1]]);
    }
    let sink = strip_shapes::kitchen_sink().bytes;
    for (label, head) in [
        ("gps", vec![0u8]),
        ("gps_and_device", vec![1]),
        ("all_pii", vec![2]),
        ("none", vec![5]),
    ] {
        let mut v = head;
        v.extend_from_slice(&sink);
        out.push((format!("{label}-strip-kitchen-sink"), v));
    }
    // Degenerate: a policy byte alone, and a mask cut short.
    out.push(("policy-byte-only".to_string(), vec![7]));
    out.push(("mask-cut-short".to_string(), vec![6, 0x40]));
    out
}

fn main() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fuzz/seeds/tiff_ifd");
    fs::create_dir_all(&dir).expect("create seeds dir");

    let le = Writer { big: false };
    let be = Writer { big: true };
    let seeds: Vec<(&str, Vec<u8>)> = vec![
        ("valid-le", valid(&le)),
        ("valid-be", valid(&be)),
        ("subifd", subifd(&le)),
        ("subifd-be", subifd(&be)),
        ("chain", chain(&le)),
        ("chain-be", chain(&be)),
        ("truncated-header", truncated_header(&le)),
        ("ifd0-offset-zero", ifd0_offset_zero(&le)),
        ("entry-count-past-eof", entry_count_past_eof(&le)),
        ("payload-past-eof", payload_past_eof(&le)),
        ("payload-past-eof-be", payload_past_eof(&be)),
        ("count-overflow", count_overflow(&le)),
        ("count-overflow-be", count_overflow(&be)),
        ("next-past-eof", next_past_eof(&le)),
        ("self-cycle", self_cycle(&le)),
        ("two-hop-cycle", two_hop_cycle(&le)),
        ("deep-nesting", deep_nesting(&le)),
        ("corrupt-subifd", corrupt_subifd(&le)),
        ("malformed-subifds-tag", malformed_subifds_tag(&le)),
        ("depth-limit-exact", depth_limit_exact(&le)),
        ("exif-gps-interop", exif_gps_interop(&le)),
        ("gps-past-eof-exif-ok", gps_past_eof_exif_ok(&le)),
        ("exif-points-at-ifd0", exif_points_at_ifd0(&le)),
        ("pointer-bad-count", pointer_bad_count(&le)),
        ("subifds-count-over-cap", subifds_count_over_cap(&le)),
        ("leaf-next-word", leaf_next_word(&le)),
        ("ascii-shapes", ascii_shapes(&le)),
        ("signed-and-float", signed_and_float(&le)),
        ("ascii-count-past-eof", ascii_count_past_eof(&le)),
        ("entry-budget-exact", budget_pair(2)),
        ("entry-budget-over", budget_pair(3)),
        ("overlapping-subifds", overlapping_subifds(40)),
    ];
    for (name, bytes) in &seeds {
        fs::write(dir.join(name), bytes).expect("write seed");
    }
    println!("wrote {} seeds to {}", seeds.len(), dir.display());

    let jdir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fuzz/seeds/jpeg_segments");
    fs::create_dir_all(&jdir).expect("create seeds dir");
    let jseeds = jpeg_seeds();
    for (name, bytes) in &jseeds {
        fs::write(jdir.join(name), bytes).expect("write seed");
    }
    println!("wrote {} seeds to {}", jseeds.len(), jdir.display());

    let adir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fuzz/seeds/audit");
    fs::create_dir_all(&adir).expect("create seeds dir");
    let aseeds = audit_seeds(&seeds, &jseeds);
    for (name, bytes) in &aseeds {
        fs::write(adir.join(name), bytes).expect("write seed");
    }
    println!("wrote {} seeds to {}", aseeds.len(), adir.display());

    let rdir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fuzz/seeds/audit_render");
    fs::create_dir_all(&rdir).expect("create seeds dir");
    let rseeds = render_seeds();
    for (name, bytes) in &rseeds {
        fs::write(rdir.join(name), bytes).expect("write seed");
    }
    println!("wrote {} seeds to {}", rseeds.len(), rdir.display());

    // SPEC-019: the shape seeds, one file each, in both JPEG-reading targets.
    for target in ["jpeg_segments", "audit"] {
        let sdir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fuzz/seeds")
            .join(target);
        for (name, bytes) in real_shapes::fixtures() {
            fs::write(sdir.join(format!("shape-{name}")), bytes).expect("write seed");
        }
    }
    println!(
        "wrote {} shape seeds to jpeg_segments and audit",
        real_shapes::fixtures().len()
    );

    // SPEC-020: the `rewrite_jpeg` seeds, a policy byte then a JPEG.
    let wdir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fuzz/seeds/rewrite_jpeg");
    fs::create_dir_all(&wdir).expect("create seeds dir");
    let wseeds = rewrite_seeds();
    for (name, bytes) in &wseeds {
        fs::write(wdir.join(name), bytes).expect("write seed");
    }
    println!("wrote {} seeds to {}", wseeds.len(), wdir.display());

    let cdir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fuzz/seeds/cli_args");
    fs::create_dir_all(&cdir).expect("create seeds dir");
    let cseeds = cli_seeds();
    for (name, bytes) in &cseeds {
        fs::write(cdir.join(name), bytes).expect("write seed");
    }
    println!("wrote {} seeds to {}", cseeds.len(), cdir.display());
}
