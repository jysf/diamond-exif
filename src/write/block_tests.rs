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

// ═════════════════════════════════════════════════════════════════════════════
// SPEC-041 (Part A): the target byte order, conversion by declared type, the
// `ByteOrder` omission reason, the CFAPattern and UNICODE rules, and the
// value-level self-check in cross order.
// ═════════════════════════════════════════════════════════════════════════════

const SBYTE: u16 = 6;
const SSHORT: u16 = 8;
const SLONG: u16 = 9;
const SRATIONAL: u16 = 10;
const FLOAT: u16 = 11;
const DOUBLE: u16 = 12;
const TYPE_IFD_13: u16 = 13;

/// The other byte order of a source.
fn other(be: bool) -> ByteOrder {
    if be {
        ByteOrder::Little
    } else {
        ByteOrder::Big
    }
}

fn cross_options(src_be: bool) -> BlockOptions {
    BlockOptions::new().with_byte_order(other(src_be))
}

fn cross_plan(data: &[u8], src_be: bool) -> ExifBlock<'_> {
    exif_block(data, &cross_options(src_be)).unwrap()
}

/// The element size of a declared type, typed again here by hand.
fn size_of_type(ty: u16) -> usize {
    match ty {
        1 | 2 | 6 | 7 => 1,
        3 | 8 => 2,
        4 | 9 | 11 => 4,
        5 | 10 | 12 => 8,
        _ => panic!("no size for type {ty}"),
    }
}

/// The width of the unit a conversion swaps, by declared type.
fn swap_width(ty: u16) -> usize {
    match ty {
        3 | 8 => 2,
        4 | 5 | 9 | 10 | 11 => 4,
        12 => 8,
        _ => 1,
    }
}

/// `logical` is the value's bytes read most significant first; encode them in
/// the given order, one swap unit at a time.
fn encode_logical(be: bool, ty: u16, logical: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for unit in logical.chunks(swap_width(ty)) {
        if be {
            out.extend_from_slice(unit);
        } else {
            out.extend(unit.iter().rev());
        }
    }
    out
}

/// One entry of the `which` IFD (`exif`, `interop` or `gps`) in a minimal
/// source.
fn one_value_source(be: bool, which: &str, entry: E) -> Vec<u8> {
    match which {
        "exif" => build(be, &[vec![ptr(0x8769, 1)], vec![entry]]),
        "interop" => build(
            be,
            &[vec![ptr(0x8769, 1)], vec![ptr(0xA005, 2)], vec![entry]],
        ),
        _ => build(be, &[vec![ptr(0x8825, 1)], vec![entry]]),
    }
}

fn table_position(p: &ExifBlock<'_>, which: &str) -> usize {
    let at = match which {
        "exif" => p.exif_position(),
        "interop" => p.interop_position,
        _ => p.gps_position(),
    };
    usize::try_from(at.unwrap()).unwrap()
}

/// The payload of `tag` in the block's `which` table, `len` bytes, for base 8.
fn block_payload(
    out_be: bool,
    block: &[u8],
    p: &ExifBlock<'_>,
    which: &str,
    tag: u16,
    len: usize,
) -> Vec<u8> {
    let en = entry_at(out_be, block, table_position(p, which), tag);
    if len <= 4 {
        block[en + 8..en + 8 + len].to_vec()
    } else {
        let at = usize::try_from(rd32(out_be, block, en + 8)).unwrap() - 8;
        block[at..at + len].to_vec()
    }
}

/// The block's value of `tag` in `which` for a cross-order plan of a source
/// holding only that entry, or `None` when it was left out.
fn cross_value(src_be: bool, which: &str, tag: u16, ty: u16, value: &[u8]) -> Option<Vec<u8>> {
    let count = u32::try_from(value.len() / size_of_type(ty)).unwrap();
    let data = one_value_source(src_be, which, e(tag, ty, count, value.to_vec()));
    let p = cross_plan(&data, src_be);
    if !p.omissions().is_empty() {
        return None;
    }
    let block = p.write(8).unwrap();
    Some(block_payload(!src_be, &block, &p, which, tag, value.len()))
}

// ── AC1 ─────────────────────────────────────────────────────────────────────

#[test]
fn options_default_has_no_byte_order() {
    assert_eq!(BlockOptions::default().byte_order(), None);
    assert_eq!(BlockOptions::new().byte_order(), None);
}

#[test]
fn options_with_byte_order_round_trips() {
    for order in [ByteOrder::Little, ByteOrder::Big] {
        let o = BlockOptions::new().with_byte_order(order);
        assert_eq!(o.byte_order(), Some(order));
        // The budget survives, and the option is part of equality.
        assert_eq!(o.max_len(), DEFAULT_MAX_BLOCK_LEN);
        assert_ne!(o, BlockOptions::new());
        assert_eq!(o.clone().with_max_len(5).byte_order(), Some(order));
    }
}

#[test]
fn byte_order_and_source_byte_order_report_both_orders() {
    for src_be in [false, true] {
        let data = sample(src_be);
        let same = plan(&data);
        assert_eq!(same.byte_order(), same.source_byte_order());
        assert_eq!(same.source_byte_order(), other(!src_be));
        let to_source =
            exif_block(&data, &BlockOptions::new().with_byte_order(other(!src_be))).unwrap();
        assert_eq!(to_source.byte_order(), other(!src_be));
        let crossed = cross_plan(&data, src_be);
        assert_eq!(crossed.byte_order(), other(src_be));
        assert_eq!(crossed.source_byte_order(), other(!src_be));
    }
}

// ── AC3 ─────────────────────────────────────────────────────────────────────

#[test]
fn convert_by_type_swaps_each_element() {
    for src_be in [false, true] {
        for ty in 1..=12u16 {
            for count in [0u32, 1, 2, 3, 5] {
                let n = size_of_type(ty) * usize::try_from(count).unwrap();
                let logical: Vec<u8> = (0..n)
                    .map(|i| u8::try_from(i).unwrap().wrapping_mul(37).wrapping_add(1))
                    .collect();
                let src = encode_logical(src_be, ty, &logical);
                let want = encode_logical(!src_be, ty, &logical);
                let data = build(
                    src_be,
                    &[
                        vec![ptr(0x8769, 1)],
                        vec![e(0xF0F0, ty, count, src.clone())],
                    ],
                );
                let p = cross_plan(&data, src_be);
                assert!(p.omissions().is_empty(), "type {ty} count {count}");
                for base in [8u32, 1000] {
                    let block = p.write(base).unwrap();
                    let en = entry_at(!src_be, &block, 0, 0xF0F0);
                    assert_eq!(rd16(!src_be, &block, en + 2), ty);
                    assert_eq!(rd32(!src_be, &block, en + 4), count);
                    let got = if n <= 4 {
                        block[en + 8..en + 8 + n].to_vec()
                    } else {
                        let at = usize::try_from(rd32(!src_be, &block, en + 8) - base).unwrap();
                        block[at..at + n].to_vec()
                    };
                    assert_eq!(
                        got, want,
                        "type {ty} count {count}, source be {src_be}, base {base}"
                    );
                }
            }
        }
    }
}

#[test]
fn cross_order_tables_and_pointer_are_in_the_target_order() {
    for src_be in [false, true] {
        let data = sample(src_be);
        let p = cross_plan(&data, src_be);
        let same = plan(&data);
        // No entry is left out, so the layout is the same function of the
        // kept entries in either order.
        assert!(p.omissions().is_empty());
        assert_eq!(p.len(), same.len());
        assert_eq!(p.exif_position(), same.exif_position());
        assert_eq!(p.gps_position(), same.gps_position());
        let out_be = !src_be;
        for base in [8u32, 1000] {
            let bytes = p.write(base).unwrap();
            assert_eq!(
                table_tags(out_be, &bytes, 0),
                vec![0x829A, 0x8827, 0x9003, 0xA005]
            );
            let at = entry_at(out_be, &bytes, 0, 0xA005);
            assert_eq!(rd16(out_be, &bytes, at + 2), LONG);
            assert_eq!(rd32(out_be, &bytes, at + 4), 1);
            assert_eq!(rd32(out_be, &bytes, at + 8), base + 6 + 12 * 4);
            // The next-word of the Exif table is 0 in either order.
            assert_eq!(rd32(out_be, &bytes, 2 + 12 * 4), 0);
        }
    }
}

// ── AC4 ─────────────────────────────────────────────────────────────────────

#[test]
fn byte_order_reason_only_in_cross_order() {
    for src_be in [false, true] {
        // 0x8828 OECF is left out under every type, in cross order only.
        let data = build(
            src_be,
            &[
                vec![ptr(0x8769, 1)],
                vec![e(0x8828, UNDEFINED, 4, vec![0, 1, 0, 2])],
            ],
        );
        for options in [
            BlockOptions::new(),
            BlockOptions::new().with_byte_order(other(!src_be)),
        ] {
            let p = exif_block(&data, &options).unwrap();
            assert!(p.omissions().is_empty(), "same order: {:?}", p.omissions());
            assert_eq!(p.byte_order(), p.source_byte_order());
        }
        let p = cross_plan(&data, src_be);
        assert_eq!(p.omissions().len(), 1);
        let o = &p.omissions()[0];
        assert_eq!(
            (o.ifd(), o.tag(), o.field_type(), o.reason()),
            (BlockIfd::Exif, 0x8828, UNDEFINED, OmitReason::ByteOrder)
        );
        // The block still holds the Exif IFD, now empty, and checks clean.
        assert!(p.write(8).is_ok());
    }
}

#[test]
fn byte_order_reason_has_a_display() {
    assert_eq!(
        OmitReason::ByteOrder.to_string(),
        "its contents depend on the byte order"
    );
}

#[test]
fn byte_order_precedence_after_offset_carrying() {
    for src_be in [false, true] {
        // 0x8828 as type 0 is UnknownType; 0x9286 as type 13 is OffsetCarrying;
        // 0x9286 as SHORT is ByteOrder; 0x927C stays MakerNote.
        let data = build(
            src_be,
            &[
                vec![ptr(0x8769, 1)],
                vec![
                    raw(0x8828, 0, 1, [1, 2, 3, 4]),
                    raw(0x927C, UNDEFINED, 1, [1, 0, 0, 0]),
                    e(0x9286, SHORT, 1, w16(src_be, 7).to_vec()),
                    raw(0x9287, TYPE_IFD_13, 1, [0, 0, 0, 0]),
                    raw(0xA20C, 14, 1, [1, 2, 3, 4]),
                ],
            ],
        );
        let data13 = build(
            src_be,
            &[
                vec![ptr(0x8769, 1)],
                vec![raw(0x9286, TYPE_IFD_13, 1, [0, 0, 0, 0])],
            ],
        );
        let p = cross_plan(&data, src_be);
        let got: Vec<_> = p
            .omissions()
            .iter()
            .map(|o| (o.tag(), o.field_type(), o.reason()))
            .collect();
        assert_eq!(
            got,
            vec![
                (0x8828, 0, OmitReason::UnknownType),
                (0x927C, UNDEFINED, OmitReason::MakerNote),
                (0x9286, SHORT, OmitReason::ByteOrder),
                (0x9287, TYPE_IFD_13, OmitReason::OffsetCarrying),
                (0xA20C, 14, OmitReason::UnknownType),
            ]
        );
        let p = cross_plan(&data13, src_be);
        let got: Vec<_> = p.omissions().iter().map(|o| o.reason()).collect();
        assert_eq!(got, vec![OmitReason::OffsetCarrying]);
    }
}

// ── AC5 ─────────────────────────────────────────────────────────────────────

/// The design's CFAPattern shapes as `(source value, block value)`: the first
/// two 16-bit words swapped, the rest copied. The same literals serve a
/// little-endian and a big-endian source, because the rule swaps bytes.
fn cfa_cases() -> Vec<(&'static str, Vec<u8>, Vec<u8>)> {
    let mut one_by_256_src = vec![0x00, 0x01, 0x01, 0x00];
    one_by_256_src.extend((0..=255u8).map(|i| i % 3));
    let mut one_by_256_want = vec![0x01, 0x00, 0x00, 0x01];
    one_by_256_want.extend((0..=255u8).map(|i| i % 3));
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
            "wrong-order header",
            vec![2, 0, 2, 0, 1, 2, 2, 1],
            vec![0, 2, 0, 2, 1, 2, 2, 1],
        ),
        ("1x256", one_by_256_src, one_by_256_want),
        (
            "3x3 over 4 bytes",
            vec![0, 3, 0, 3, 1, 2, 0, 1],
            vec![3, 0, 3, 0, 1, 2, 0, 1],
        ),
        ("0x0", vec![0, 0, 0, 0], vec![0, 0, 0, 0]),
        (
            "2x2 with trailing bytes",
            vec![0, 2, 0, 2, 1, 2, 2, 1, 0xAA, 0xBB],
            vec![2, 0, 2, 0, 1, 2, 2, 1, 0xAA, 0xBB],
        ),
        ("header only", vec![0, 2, 0, 3], vec![2, 0, 3, 0]),
        (
            "an asymmetric header and a pixel",
            vec![0x12, 0x34, 0x56, 0x78, 0x9A],
            vec![0x34, 0x12, 0x78, 0x56, 0x9A],
        ),
    ]
}

#[test]
fn cfa_header_swapped_rest_copied() {
    for src_be in [false, true] {
        for which in ["exif", "interop"] {
            for (name, src, want) in cfa_cases() {
                let got = cross_value(src_be, which, 0xA302, UNDEFINED, &src);
                assert_eq!(got.as_ref(), Some(&want), "{name} in {which}, be {src_be}");
            }
        }
    }
}

#[test]
fn cfa_under_four_bytes_copied() {
    for src_be in [false, true] {
        for which in ["exif", "interop"] {
            for value in [vec![], vec![0x12], vec![0x12, 0x34], vec![0x12, 0x34, 0x56]] {
                let got = cross_value(src_be, which, 0xA302, UNDEFINED, &value);
                assert_eq!(got.as_ref(), Some(&value), "{value:?} in {which}");
            }
        }
    }
    // The same order copies every shape, headers included.
    for src_be in [false, true] {
        for (name, src, _) in cfa_cases() {
            let data = one_value_source(
                src_be,
                "exif",
                e(
                    0xA302,
                    UNDEFINED,
                    u32::try_from(src.len()).unwrap(),
                    src.clone(),
                ),
            );
            let p = plan(&data);
            let block = p.write(8).unwrap();
            assert_eq!(
                block_payload(src_be, &block, &p, "exif", 0xA302, src.len()),
                src,
                "{name}"
            );
        }
    }
}

// ── AC6 ─────────────────────────────────────────────────────────────────────

/// A UNICODE comment: the 8-byte code, then the text's bytes.
fn comment(code: &[u8; 8], text: &[u8]) -> Vec<u8> {
    let mut v = code.to_vec();
    v.extend_from_slice(text);
    v
}

/// Where the UNICODE rule applies: `(IFD, tag, types)`.
fn unicode_places() -> Vec<(&'static str, u16, Vec<u16>)> {
    vec![
        ("exif", 0x9286, vec![BYTE, ASCII, SBYTE, UNDEFINED]),
        ("interop", 0x9286, vec![BYTE, ASCII, SBYTE, UNDEFINED]),
        ("gps", 0x001B, vec![UNDEFINED]),
        ("gps", 0x001C, vec![UNDEFINED]),
    ]
}

#[test]
fn unicode_units_swapped_after_code() {
    // 中文 is U+4E2D U+6587.
    let cases: Vec<(&str, Vec<u8>, Vec<u8>)> = vec![
        (
            "UNICODE\\0",
            comment(b"UNICODE\0", &[0x4E, 0x2D, 0x65, 0x87]),
            comment(b"UNICODE\0", &[0x2D, 0x4E, 0x87, 0x65]),
        ),
        (
            "UNICODE space",
            comment(b"UNICODE ", &[0x4E, 0x2D, 0x65, 0x87]),
            comment(b"UNICODE ", &[0x2D, 0x4E, 0x87, 0x65]),
        ),
        (
            "with a byte-order mark",
            comment(b"UNICODE\0", &[0xFE, 0xFF, 0x4E, 0x2D, 0x65, 0x87]),
            comment(b"UNICODE\0", &[0xFF, 0xFE, 0x2D, 0x4E, 0x87, 0x65]),
        ),
        (
            "code only",
            comment(b"UNICODE\0", &[]),
            comment(b"UNICODE\0", &[]),
        ),
    ];
    for src_be in [false, true] {
        for (which, tag, types) in unicode_places() {
            for ty in &types {
                for (name, src, want) in &cases {
                    let got = cross_value(src_be, which, tag, *ty, src);
                    assert_eq!(
                        got.as_ref(),
                        Some(want),
                        "{name}: {tag:#06x} type {ty} in {which}, be {src_be}"
                    );
                }
            }
        }
    }
}

#[test]
fn unicode_odd_last_byte_copied() {
    let src = comment(b"UNICODE\0", &[0x4E, 0x2D, 0x65]);
    let want = comment(b"UNICODE\0", &[0x2D, 0x4E, 0x65]);
    for src_be in [false, true] {
        for (which, tag, types) in unicode_places() {
            for ty in &types {
                assert_eq!(
                    cross_value(src_be, which, tag, *ty, &src).as_ref(),
                    Some(&want),
                    "{tag:#06x} type {ty} in {which}"
                );
            }
        }
    }
}

#[test]
fn unicode_codes_measured() {
    let text = [0x4E, 0x2D, 0x65, 0x87];
    let copied: Vec<(&str, Vec<u8>)> = vec![
        ("Unicode\\0", comment(b"Unicode\0", &text)),
        ("UNICODEX", comment(b"UNICODEX", &text)),
        (" UNICODE", comment(b" UNICODE", &text)),
        ("UNICODE\\t", comment(b"UNICODE\t", &text)),
        ("UNICOD\\0\\0", comment(b"UNICOD\0\0", &text)),
        ("ASCII", comment(b"ASCII\0\0\0", &text)),
        ("JIS", comment(b"JIS\0\0\0\0\0", &text)),
        ("eight zeros", comment(&[0; 8], &text)),
        ("under 8 bytes", b"UNICOD".to_vec()),
        ("exactly 7 bytes", b"UNICODE".to_vec()),
    ];
    for src_be in [false, true] {
        for (which, tag, types) in unicode_places() {
            for ty in &types {
                for (name, value) in &copied {
                    assert_eq!(
                        cross_value(src_be, which, tag, *ty, value).as_ref(),
                        Some(value),
                        "{name}: {tag:#06x} type {ty} in {which}"
                    );
                }
            }
        }
    }
}

#[test]
fn unicode_rule_does_not_reach_other_places() {
    let value = comment(b"UNICODE\0", &[0x4E, 0x2D, 0x65, 0x87]);
    for src_be in [false, true] {
        // 0x001B as ASCII (type 2) in the GPS IFD: the rule is type 7 only.
        assert_eq!(
            cross_value(src_be, "gps", 0x001B, ASCII, &value).as_ref(),
            Some(&value)
        );
        // GPSAreaInformation's sibling 0x001D is not text of this kind: a
        // date stamp as type 7 is copied (it is not in the table for type 7).
        assert_eq!(
            cross_value(src_be, "gps", 0x001D, UNDEFINED, &value).as_ref(),
            Some(&value)
        );
        // 0x9286 as a multi-byte type is left out, never swapped.
        let data = one_value_source(
            src_be,
            "exif",
            e(0x9286, SHORT, 1, w16(src_be, 0x4E2D).to_vec()),
        );
        let p = cross_plan(&data, src_be);
        let got: Vec<_> = p.omissions().iter().map(|o| o.reason()).collect();
        assert_eq!(got, vec![OmitReason::ByteOrder]);
        // The same text in the same order is copied.
        let data = one_value_source(src_be, "exif", e(0x9286, UNDEFINED, 12, value.clone()));
        let p = plan(&data);
        let block = p.write(8).unwrap();
        assert_eq!(block_payload(src_be, &block, &p, "exif", 0x9286, 12), value);
    }
}

// ── AC7 ─────────────────────────────────────────────────────────────────────

/// A little-endian source holding one entry of the Exif IFD, built into a
/// big-endian block. Returns the plan's good block and where the entry sits.
fn cross_block_with(entry: E) -> (Vec<u8>, Vec<u8>, usize) {
    let data = build(false, &[vec![ptr(0x8769, 1)], vec![entry]]);
    let p = cross_plan(&data, false);
    let bytes = p.write(8).unwrap();
    assert_eq!(check_block(&p, 8, &bytes), Ok(()));
    (data, bytes, 0)
}

/// The block-relative offset of an out-of-line value of the first entry.
fn value_at(block: &[u8]) -> usize {
    let en = entry_at(true, block, 0, entry_tag_of(block));
    usize::try_from(rd32(true, block, en + 8)).unwrap() - 8
}

fn entry_tag_of(block: &[u8]) -> u16 {
    rd16(true, block, 2)
}

#[test]
fn check_block_cross_order_rejects_each_corruption() {
    // 1. A SHORT left in the source's order.
    let (data, mut bytes, _) = cross_block_with(e(0x8827, SHORT, 1, w16(false, 0x0102).to_vec()));
    assert_eq!(&bytes[10..12], &[0x01, 0x02]);
    bytes[10..12].copy_from_slice(&[0x02, 0x01]);
    assert_eq!(
        check_block(&cross_plan(&data, false), 8, &bytes),
        Err(BlockCheck::Entries),
        "a SHORT in the source's order"
    );

    // 2. One byte of a converted LONG flipped.
    let (data, mut bytes, _) =
        cross_block_with(e(0xA002, LONG, 1, w32(false, 0x0102_0304).to_vec()));
    assert_eq!(&bytes[10..14], &[1, 2, 3, 4]);
    bytes[11] ^= 0x10;
    assert_eq!(
        check_block(&cross_plan(&data, false), 8, &bytes),
        Err(BlockCheck::Entries),
        "a flipped LONG byte"
    );

    // 3. A RATIONAL with its halves swapped.
    let (data, mut bytes, _) =
        cross_block_with(e(0x829A, RATIONAL, 1, rationals(false, &[(1, 250)])));
    let at = value_at(&bytes);
    assert_eq!(&bytes[at..at + 8], &[0, 0, 0, 1, 0, 0, 0, 250]);
    let (a, b) = (bytes[at..at + 4].to_vec(), bytes[at + 4..at + 8].to_vec());
    bytes[at..at + 4].copy_from_slice(&b);
    bytes[at + 4..at + 8].copy_from_slice(&a);
    assert_eq!(
        check_block(&cross_plan(&data, false), 8, &bytes),
        Err(BlockCheck::Entries),
        "RATIONAL halves swapped"
    );

    // 4. A CFAPattern header left unswapped (a 2 x 3 header, little-endian in
    // the source: 02 00 03 00).
    let (data, mut bytes, _) =
        cross_block_with(e(0xA302, UNDEFINED, 10, vec![2, 0, 3, 0, 0, 1, 2, 1, 2, 0]));
    let at = value_at(&bytes);
    assert_eq!(&bytes[at..at + 4], &[0, 2, 0, 3]);
    bytes[at..at + 4].copy_from_slice(&[2, 0, 3, 0]);
    assert_eq!(
        check_block(&cross_plan(&data, false), 8, &bytes),
        Err(BlockCheck::Entries),
        "CFAPattern header unswapped"
    );

    // 5. UNICODE units left unswapped.
    let text = comment(b"UNICODE\0", &[0x2D, 0x4E, 0x87, 0x65]);
    let (data, mut bytes, _) = cross_block_with(e(0x9286, UNDEFINED, 12, text));
    let at = value_at(&bytes);
    assert_eq!(&bytes[at + 8..at + 12], &[0x4E, 0x2D, 0x65, 0x87]);
    bytes[at + 8..at + 12].copy_from_slice(&[0x2D, 0x4E, 0x87, 0x65]);
    assert_eq!(
        check_block(&cross_plan(&data, false), 8, &bytes),
        Err(BlockCheck::Entries),
        "UNICODE units unswapped"
    );

    // 6. An ASCII-coded UserComment swapped.
    let text = comment(b"ASCII\0\0\0", b"abcd");
    let (data, mut bytes, _) = cross_block_with(e(0x9286, UNDEFINED, 12, text));
    let at = value_at(&bytes);
    assert_eq!(&bytes[at + 8..at + 12], b"abcd");
    bytes[at + 8..at + 12].copy_from_slice(b"badc");
    assert_eq!(
        check_block(&cross_plan(&data, false), 8, &bytes),
        Err(BlockCheck::Entries),
        "an ASCII-coded comment swapped"
    );
}

#[test]
fn check_block_cross_order_accepts_what_it_wrote() {
    for src_be in [false, true] {
        for data in [sample(src_be), slack_source(src_be)] {
            let p = cross_plan(&data, src_be);
            for base in [8u32, 1000] {
                let bytes = p.write(base).unwrap();
                assert_eq!(check_block(&p, base, &bytes), Ok(()));
            }
        }
    }
}

#[test]
fn fault_hook_cross_order_fails_verify() {
    for src_be in [false, true] {
        let data = sample(src_be);
        let p = cross_plan(&data, src_be);
        assert!(p.write(8).is_ok());
        fault::FLIP.with(|f| f.set(Some(0)));
        let got = p.write(8);
        fault::FLIP.with(|f| f.set(None));
        assert!(
            matches!(got, Err(BlockError::VerifyFailed { .. })),
            "{got:?}"
        );
        // Flip a converted value byte: the value comparison must catch it.
        let clean = p.write(8).unwrap();
        let en = entry_at(!src_be, &clean, 0, 0x8827);
        fault::FLIP.with(|f| f.set(Some(en + 8)));
        let got = p.write(8);
        fault::FLIP.with(|f| f.set(None));
        assert!(
            matches!(
                got,
                Err(BlockError::VerifyFailed {
                    check: BlockCheck::Entries
                })
            ),
            "{got:?}"
        );
        assert!(p.write(8).is_ok());
    }
}

// ── budget in cross order ───────────────────────────────────────────────────

#[test]
fn cross_order_budget_is_checked_before_allocation() {
    let data = sample(true);
    let len = cross_plan(&data, true).len();
    let snug = cross_options(true).with_max_len(len + 42);
    let p = exif_block(&data, &snug).unwrap();
    assert!(p.write(8).is_ok());
    assert!(matches!(p.write(10), Err(BlockError::OverBudget { .. })));
    assert_eq!(p.write(9), Err(BlockError::BadBase { base: 9 }));
    assert!(matches!(
        p.write(1 << 30),
        Err(BlockError::OverBudget { .. })
    ));
}
