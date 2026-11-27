//! Unit tests of the Exif block builder (SPEC-025). The sources are built here
//! by a small self-contained encoder; the integration tests in
//! `tests/exif_block.rs` carry the fixture grid and the independent model.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::arithmetic_side_effects
)]

use super::*;

const BYTE: u16 = 1;
const ASCII: u16 = 2;
const SHORT: u16 = 3;
const LONG: u16 = 4;
const RATIONAL: u16 = 5;
const UNDEFINED: u16 = 7;

// ── a tiny source encoder ───────────────────────────────────────────────────

#[derive(Clone)]
enum V {
    /// Encoded payload in the file's byte order: inline when at most 4 bytes.
    Bytes(Vec<u8>),
    /// The 4 value-field bytes as given (so unused inline bytes can be garbage).
    Raw([u8; 4]),
    /// A LONG pointer to the table of IFD `n` of the document.
    Ptr(usize),
}

#[derive(Clone)]
struct E {
    tag: u16,
    ty: u16,
    count: u32,
    v: V,
}

fn e(tag: u16, ty: u16, count: u32, bytes: Vec<u8>) -> E {
    E {
        tag,
        ty,
        count,
        v: V::Bytes(bytes),
    }
}

fn raw(tag: u16, ty: u16, count: u32, field: [u8; 4]) -> E {
    E {
        tag,
        ty,
        count,
        v: V::Raw(field),
    }
}

fn ptr(tag: u16, ifd: usize) -> E {
    E {
        tag,
        ty: LONG,
        count: 1,
        v: V::Ptr(ifd),
    }
}

fn w16(be: bool, v: u16) -> [u8; 2] {
    if be {
        v.to_be_bytes()
    } else {
        v.to_le_bytes()
    }
}

fn w32(be: bool, v: u32) -> [u8; 4] {
    if be {
        v.to_be_bytes()
    } else {
        v.to_le_bytes()
    }
}

fn rd16(be: bool, b: &[u8], at: usize) -> u16 {
    let a = [b[at], b[at + 1]];
    if be {
        u16::from_be_bytes(a)
    } else {
        u16::from_le_bytes(a)
    }
}

fn rd32(be: bool, b: &[u8], at: usize) -> u32 {
    let a = [b[at], b[at + 1], b[at + 2], b[at + 3]];
    if be {
        u32::from_be_bytes(a)
    } else {
        u32::from_le_bytes(a)
    }
}

fn rationals(be: bool, vals: &[(u32, u32)]) -> Vec<u8> {
    vals.iter()
        .flat_map(|(n, d)| [w32(be, *n), w32(be, *d)].concat())
        .collect()
}

fn ascii(text: &str) -> (u32, Vec<u8>) {
    let mut bytes = text.as_bytes().to_vec();
    bytes.push(0);
    (u32::try_from(bytes.len()).unwrap(), bytes)
}

/// IFD 0 of `ifds` is IFD0. Tables are laid out from offset 8 in order, then
/// every out-of-line value, each at an even offset.
fn build(be: bool, ifds: &[Vec<E>]) -> Vec<u8> {
    let mut at = Vec::new();
    let mut pos = 8usize;
    for t in ifds {
        at.push(pos);
        pos += 6 + 12 * t.len();
    }
    let mut buf = Vec::new();
    buf.extend_from_slice(if be { b"MM\0\x2A" } else { b"II\x2A\0" });
    buf.extend_from_slice(&w32(be, 8));
    let mut values: Vec<u8> = Vec::new();
    for t in ifds {
        buf.extend_from_slice(&w16(be, u16::try_from(t.len()).unwrap()));
        for en in t {
            buf.extend_from_slice(&w16(be, en.tag));
            buf.extend_from_slice(&w16(be, en.ty));
            buf.extend_from_slice(&w32(be, en.count));
            match &en.v {
                V::Raw(field) => buf.extend_from_slice(field),
                V::Ptr(n) => buf.extend_from_slice(&w32(be, u32::try_from(at[*n]).unwrap())),
                V::Bytes(bytes) if bytes.len() <= 4 => {
                    let mut field = [0u8; 4];
                    field[..bytes.len()].copy_from_slice(bytes);
                    buf.extend_from_slice(&field);
                }
                V::Bytes(bytes) => {
                    if values.len() % 2 == 1 {
                        values.push(0);
                    }
                    let off = pos + values.len();
                    buf.extend_from_slice(&w32(be, u32::try_from(off).unwrap()));
                    values.extend_from_slice(bytes);
                }
            }
        }
        buf.extend_from_slice(&[0; 4]);
    }
    buf.extend_from_slice(&values);
    buf
}

/// The design sample: IFD0 (Exif and GPS pointers), an Exif IFD with an
/// out-of-line RATIONAL, an ASCII, a SHORT and an Interop pointer, an Interop
/// IFD and a GPS IFD with a RATIONAL[3]. Entries are not in tag order.
fn sample(be: bool) -> Vec<u8> {
    let (n, dt) = ascii("2031:02:03 04:05:06");
    let ifd0 = vec![ptr(0x8769, 1), ptr(0x8825, 3)];
    let exif = vec![
        e(0x9003, ASCII, n, dt),
        e(0x829A, RATIONAL, 1, rationals(be, &[(1, 250)])),
        ptr(0xA005, 2),
        e(0x8827, SHORT, 1, w16(be, 200).to_vec()),
    ];
    let (n2, r98) = ascii("R98");
    let interop = vec![e(0x0001, ASCII, n2, r98)];
    let gps = vec![
        e(
            0x0002,
            RATIONAL,
            3,
            rationals(be, &[(50, 1), (4, 1), (0, 1)]),
        ),
        e(0x0001, ASCII, 2, b"N\0".to_vec()),
    ];
    build(be, &[ifd0, exif, interop, gps])
}

fn plan(data: &[u8]) -> ExifBlock<'_> {
    exif_block(data, &BlockOptions::new()).unwrap()
}

/// Tags of the table at block-relative `at`, in table order.
fn table_tags(be: bool, block: &[u8], at: usize) -> Vec<u16> {
    let n = usize::from(rd16(be, block, at));
    (0..n).map(|i| rd16(be, block, at + 2 + 12 * i)).collect()
}

/// Offset in `block` of the entry of `tag` in the table at `at`.
fn entry_at(be: bool, block: &[u8], at: usize, tag: u16) -> usize {
    let n = usize::from(rd16(be, block, at));
    let i = (0..n)
        .find(|i| rd16(be, block, at + 2 + 12 * i) == tag)
        .unwrap();
    at + 2 + 12 * i
}

// ── layout ──────────────────────────────────────────────────────────────────

#[test]
fn layout_puts_tables_first_then_values_in_ifd_and_tag_order() {
    for be in [false, true] {
        let data = sample(be);
        let p = plan(&data);
        let bytes = p.write(8).unwrap();
        assert_eq!(
            p.byte_order(),
            if be {
                ByteOrder::Big
            } else {
                ByteOrder::Little
            }
        );
        // Exif (4 entries), Interop (1), GPS (2): tables are 6 + 12n bytes.
        assert_eq!(p.exif_position(), Some(0));
        let interop_at = 6 + 12 * 4;
        let gps_at = interop_at + 6 + 12;
        assert_eq!(p.gps_position(), Some(u32::try_from(gps_at).unwrap()));
        assert_eq!(
            table_tags(be, &bytes, 0),
            vec![0x829A, 0x8827, 0x9003, 0xA005]
        );
        assert_eq!(table_tags(be, &bytes, interop_at), vec![0x0001]);
        assert_eq!(table_tags(be, &bytes, gps_at), vec![0x0001, 0x0002]);
        // Every table ends in a next-word of 0.
        assert_eq!(rd32(be, &bytes, 2 + 12 * 4), 0);
        // Values follow the tables, in IFD then tag order, each even: Exif
        // 0x829A (8 bytes), 0x9003 (20), then GPS 0x0002 (24). Interop "R98"
        // and GPS "N" are inline.
        let values_at = u32::try_from(gps_at + 6 + 24).unwrap();
        let rat = entry_at(be, &bytes, 0, 0x829A);
        assert_eq!(rd32(be, &bytes, rat + 8), 8 + values_at);
        let dt = entry_at(be, &bytes, 0, 0x9003);
        assert_eq!(rd32(be, &bytes, dt + 8), 8 + values_at + 8);
        let lat = entry_at(be, &bytes, gps_at, 0x0002);
        assert_eq!(rd32(be, &bytes, lat + 8), 8 + values_at + 8 + 20);
        assert_eq!(p.len(), values_at + 8 + 20 + 24);
        assert_eq!(usize::try_from(p.len()).unwrap(), bytes.len());
    }
}

#[test]
fn positions_and_len_are_even_and_base_independent() {
    let data = sample(false);
    let p = plan(&data);
    let len = p.len();
    assert_eq!(len % 2, 0);
    assert_eq!(p.exif_position().unwrap() % 2, 0);
    assert_eq!(p.gps_position().unwrap() % 2, 0);
    for base in [8u32, 1000, 65534] {
        let bytes = p.write(base).unwrap();
        assert_eq!(u32::try_from(bytes.len()).unwrap(), len);
    }
}

#[test]
fn interop_pointer_is_long_count_one_at_base_plus_position() {
    for be in [false, true] {
        let data = sample(be);
        let p = plan(&data);
        for base in [8u32, 1000] {
            let bytes = p.write(base).unwrap();
            let at = entry_at(be, &bytes, 0, 0xA005);
            assert_eq!(rd16(be, &bytes, at + 2), LONG);
            assert_eq!(rd32(be, &bytes, at + 4), 1);
            assert_eq!(rd32(be, &bytes, at + 8), base + 6 + 12 * 4);
        }
    }
}

#[test]
fn inline_unused_bytes_are_zero() {
    let ifd0 = vec![ptr(0x8769, 1)];
    // A SHORT with garbage after it, and a BYTE x2 with garbage after it.
    let exif = vec![
        raw(0x8827, SHORT, 1, [0xC8, 0x00, 0xAA, 0xBB]),
        raw(0x9101, BYTE, 2, [1, 2, 0xCC, 0xDD]),
    ];
    let data = build(false, &[ifd0, exif]);
    let p = plan(&data);
    let bytes = p.write(8).unwrap();
    let a = entry_at(false, &bytes, 0, 0x8827);
    assert_eq!(&bytes[a + 8..a + 12], &[0xC8, 0, 0, 0]);
    let b = entry_at(false, &bytes, 0, 0x9101);
    assert_eq!(&bytes[b + 8..b + 12], &[1, 2, 0, 0]);
}

#[test]
fn count_zero_is_kept() {
    let ifd0 = vec![ptr(0x8769, 1)];
    let exif = vec![
        e(0x010E, ASCII, 0, vec![]),
        e(0x8827, SHORT, 0, vec![]),
        e(0x829A, RATIONAL, 0, vec![]),
    ];
    let data = build(true, &[ifd0, exif]);
    let p = plan(&data);
    assert!(p.omissions().is_empty());
    let bytes = p.write(8).unwrap();
    assert_eq!(table_tags(true, &bytes, 0), vec![0x010E, 0x829A, 0x8827]);
    let a = entry_at(true, &bytes, 0, 0x829A);
    assert_eq!(rd32(true, &bytes, a + 4), 0);
    // Three empty entries need no value bytes: the block is the table alone.
    assert_eq!(p.len(), 6 + 36);
}

#[test]
fn selects_exif_and_gps_by_parent_ifd0() {
    // IFD0 -> SubIFD (holding its own Exif and GPS pointers) and IFD0's Exif
    // and GPS. The walk visits the SubIFD's Exif first; the block must take
    // IFD0's.
    let ifd0 = vec![ptr(0x014A, 1), ptr(0x8769, 3), ptr(0x8825, 4)];
    let sub = vec![ptr(0x8769, 2), ptr(0x8825, 5)];
    let sub_exif = vec![e(0x9003, ASCII, 4, b"sub\0".to_vec())];
    let exif = vec![e(0x9004, ASCII, 4, b"top\0".to_vec())];
    let gps = vec![e(0x0001, ASCII, 2, b"N\0".to_vec())];
    let sub_gps = vec![e(0x0001, ASCII, 2, b"S\0".to_vec())];
    let data = build(false, &[ifd0, sub, sub_exif, exif, gps, sub_gps]);
    let p = plan(&data);
    let bytes = p.write(8).unwrap();
    assert_eq!(table_tags(false, &bytes, 0), vec![0x9004]);
    let gps_at = usize::try_from(p.gps_position().unwrap()).unwrap();
    let g = entry_at(false, &bytes, gps_at, 0x0001);
    assert_eq!(&bytes[g + 8..g + 10], b"N\0");
    assert!(p.omissions().is_empty());
}

// ── the checker ─────────────────────────────────────────────────────────────

fn checked_sample() -> (Vec<u8>, Vec<u8>) {
    let data = sample(false);
    let bytes = plan(&data).write(8).unwrap();
    (data, bytes)
}

#[test]
fn checker_accepts_a_good_block() {
    let (data, bytes) = checked_sample();
    assert_eq!(check_block(&plan(&data), 8, &bytes), Ok(()));
}

#[test]
fn checker_catches_flipped_value_byte() {
    let (data, mut bytes) = checked_sample();
    let at = entry_at(false, &bytes, 0, 0x9003);
    let value = usize::try_from(rd32(false, &bytes, at + 8)).unwrap() - 8;
    bytes[value + 3] ^= 0x01;
    assert_eq!(
        check_block(&plan(&data), 8, &bytes),
        Err(BlockCheck::Entries)
    );
}

#[test]
fn checker_catches_wrong_interop_offset() {
    let (data, mut bytes) = checked_sample();
    let at = entry_at(false, &bytes, 0, 0xA005);
    let v = rd32(false, &bytes, at + 8);
    bytes[at + 8..at + 12].copy_from_slice(&w32(false, v + 2));
    let got = check_block(&plan(&data), 8, &bytes);
    assert!(
        matches!(got, Err(BlockCheck::Positions | BlockCheck::Parse)),
        "{got:?}"
    );
}

#[test]
fn checker_catches_unreported_omission() {
    let ifd0 = vec![ptr(0x8769, 1)];
    let exif = vec![
        e(0x927C, UNDEFINED, 6, b"Nikon\0".to_vec()),
        e(0x8827, SHORT, 1, w16(false, 200).to_vec()),
    ];
    let data = build(false, &[ifd0, exif]);
    let mut p = plan(&data);
    assert_eq!(p.omissions().len(), 1);
    let bytes = p.write(8).unwrap();
    p.omissions.clear();
    assert_eq!(check_block(&p, 8, &bytes), Err(BlockCheck::Omissions));
}

#[test]
fn checker_catches_reported_but_present() {
    let (data, bytes) = checked_sample();
    let mut p = plan(&data);
    p.omissions.push(Omission {
        ifd: BlockIfd::Exif,
        tag: 0x8827,
        field_type: SHORT,
        reason: OmitReason::OffsetCarrying,
    });
    assert_eq!(check_block(&p, 8, &bytes), Err(BlockCheck::Omissions));
}

#[test]
fn checker_catches_truncation() {
    let (data, mut bytes) = checked_sample();
    bytes.truncate(bytes.len() - 2);
    let got = check_block(&plan(&data), 8, &bytes);
    assert!(
        matches!(got, Err(BlockCheck::Length | BlockCheck::Parse)),
        "{got:?}"
    );
}

#[test]
fn fault_hook_makes_write_verify_failed() {
    let data = sample(false);
    let p = plan(&data);
    assert!(p.write(8).is_ok());
    fault::FLIP.with(|f| f.set(Some(0)));
    let got = p.write(8);
    fault::FLIP.with(|f| f.set(None));
    assert!(
        matches!(got, Err(BlockError::VerifyFailed { .. })),
        "{got:?}"
    );
    // The hook is one-shot per set: the next write is clean again.
    assert!(p.write(8).is_ok());
}

/// A source whose Exif IFD holds one inline BYTE (3 unused inline bytes), an
/// out-of-line UNDEFINED of 5 bytes (so one padding byte follows it) and an
/// out-of-line RATIONAL after it (so the padding byte lies inside the block).
fn slack_source(be: bool) -> Vec<u8> {
    let ifd0 = vec![ptr(0x8769, 1)];
    let exif = vec![
        e(0x9101, UNDEFINED, 5, vec![1, 2, 3, 4, 5]),
        e(0x9202, RATIONAL, 1, rationals(be, &[(1, 2)])),
        e(0xA301, BYTE, 1, vec![7]),
    ];
    build(be, &[ifd0, exif])
}

/// Flip bit 0 of block byte `index` through the fault hook, then write.
fn write_with_flip(p: &ExifBlock<'_>, index: usize) -> Result<Vec<u8>, BlockError> {
    fault::FLIP.with(|f| f.set(Some(index)));
    let got = p.write(8);
    fault::FLIP.with(|f| f.set(None));
    got
}

#[test]
fn self_check_catches_a_flipped_inline_slack_byte() {
    for be in [false, true] {
        let data = slack_source(be);
        let p = plan(&data);
        let clean = p.write(8).unwrap();
        // The BYTE entry's value field is [7, 0, 0, 0]: bytes 1 to 3 are slack.
        let field = entry_at(be, &clean, 0, 0xA301) + 8;
        assert_eq!(&clean[field..field + 4], &[7, 0, 0, 0]);
        for slack in 1..4 {
            let got = write_with_flip(&p, field + slack);
            assert!(
                matches!(got, Err(BlockError::VerifyFailed { .. })),
                "slack byte {slack}: {got:?}"
            );
        }
    }
}

#[test]
fn self_check_catches_a_flipped_padding_byte() {
    for be in [false, true] {
        let data = slack_source(be);
        let p = plan(&data);
        let clean = p.write(8).unwrap();
        let field = entry_at(be, &clean, 0, 0x9101) + 8;
        let value = usize::try_from(rd32(be, &clean, field)).unwrap() - 8;
        assert_eq!(&clean[value..value + 5], &[1, 2, 3, 4, 5]);
        let pad = value + 5;
        assert_eq!(clean[pad], 0, "the padding byte is zero");
        assert!(pad < clean.len());
        let got = write_with_flip(&p, pad);
        assert!(
            matches!(got, Err(BlockError::VerifyFailed { .. })),
            "{got:?}"
        );
    }
}

// ── budget and base ─────────────────────────────────────────────────────────

#[test]
fn budget_is_checked_before_allocation() {
    let data = sample(false);
    let len = plan(&data).len();
    // The block alone over the budget: refused by `exif_block`.
    let tiny = BlockOptions::new().with_max_len(len - 2);
    assert!(matches!(
        exif_block(&data, &tiny),
        Err(BlockError::OverBudget { budget, .. }) if budget == len - 2
    ));
    // The block fits, the scratch image (`base + len + 34`) does not.
    let snug = BlockOptions::new().with_max_len(len + 42);
    let p = exif_block(&data, &snug).unwrap();
    assert!(p.write(8).is_ok());
    assert!(matches!(
        p.write(10),
        Err(BlockError::OverBudget { needed, budget })
            if needed == u64::from(10 + len + 34) && budget == len + 42
    ));
    // A huge base is refused before any allocation of that size is attempted.
    let p = plan(&data);
    assert!(matches!(
        p.write(1 << 30),
        Err(BlockError::OverBudget { .. })
    ));
    let mut out = vec![0u8; usize::try_from(len).unwrap()];
    assert!(matches!(
        p.write_into(1 << 30, &mut out),
        Err(BlockError::OverBudget { .. })
    ));
}

#[test]
fn odd_base_and_overflowing_base_are_bad_base() {
    let data = sample(false);
    let p = plan(&data);
    assert_eq!(p.write(9), Err(BlockError::BadBase { base: 9 }));
    assert_eq!(p.write(1001), Err(BlockError::BadBase { base: 1001 }));
    // base + len past u32::MAX.
    assert_eq!(
        p.write(0xFFFF_FFFE),
        Err(BlockError::BadBase { base: 0xFFFF_FFFE })
    );
    // A block cannot sit inside the TIFF header.
    assert_eq!(p.write(4), Err(BlockError::BadBase { base: 4 }));
    // An empty block has nothing to place: any even base gives no bytes.
    let none = build(false, &[vec![e(0x010F, ASCII, 2, b"X\0".to_vec())]]);
    let p = plan(&none);
    assert!(p.is_empty());
    assert_eq!(p.write(0), Ok(Vec::new()));
    assert_eq!(p.write(1000), Ok(Vec::new()));
    assert_eq!(p.write(1001), Err(BlockError::BadBase { base: 1001 }));
}
