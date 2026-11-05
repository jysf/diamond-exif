//! Deterministic writer for the `tiff_ifd` fuzz seeds (SPEC-006).
//!
//! `cargo run --example fuzz_seeds` rewrites `fuzz/seeds/tiff_ifd/`. Output is a
//! pure function of this file (no clock, no randomness, no environment), so two
//! runs produce byte-identical seeds. The seeds are hand-built, a few hundred
//! bytes each, and are this repository's own work: the shapes follow TIFF 6.0
//! section 2 (header, IFD, entry layout). std only; examples are not part of
//! the library lint set.

use std::fs;
use std::path::PathBuf;

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
    ];
    for (name, bytes) in &seeds {
        fs::write(dir.join(name), bytes).expect("write seed");
    }
    println!("wrote {} seeds to {}", seeds.len(), dir.display());
}
