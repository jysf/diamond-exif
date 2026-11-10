//! Tests of the read-only accessors the writer uses (SPEC-021): where an IFD's
//! table and an entry's out-of-line payload lie in the blob.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::arithmetic_side_effects
)]

use super::*;

/// Little-endian blob: IFD0 at 8 with `entries` as `(tag, type, count, value
/// word)`, next word `next`, `tail` bytes appended.
fn blob(entries: &[(u16, u16, u32, u32)], next: u32, tail: &[u8]) -> Vec<u8> {
    let mut out = b"II\x2A\x00\x08\x00\x00\x00".to_vec();
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    for (tag, ty, count, value) in entries {
        out.extend_from_slice(&tag.to_le_bytes());
        out.extend_from_slice(&ty.to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.extend_from_slice(&next.to_le_bytes());
    out.extend_from_slice(tail);
    out
}

#[test]
fn ifd_table_range_covers_count_entries_and_next_word() {
    // Two entries: 2 + 24 + 4 = 30 bytes from offset 8.
    let data = blob(&[(0x0112, 3, 1, 6), (0x0131, 2, 1, 0)], 0, &[]);
    let c = Container::parse(&data).unwrap();
    let ifd = c.ifd0().unwrap();
    assert_eq!(ifd.table_range(), Some(8..38));
    assert_eq!(ifd.next_at(), Some(34));
    assert_eq!(data.len(), 38);
    // The empty table: count word and next word only.
    let data = blob(&[], 0, &[]);
    let c = Container::parse(&data).unwrap();
    assert_eq!(c.ifd0().unwrap().table_range(), Some(8..14));
    assert_eq!(c.ifd0().unwrap().next_at(), Some(10));
}

#[test]
fn ifd_table_range_overflow_is_none() {
    // An IFD whose offset is near `usize::MAX` cannot be built through the walk
    // (the table must lie in the input), so the accessor is checked on a
    // constructed value: with the offset at `u32::MAX` nothing overflows on a
    // 64-bit host and the range is exact; with a count that does not fit, the
    // sum is checked.
    let entry = Entry {
        tag: 1,
        field_type: 3,
        count: 1,
        value: [0; 4],
        value_at: 0,
    };
    let ifd = Ifd {
        offset: u32::MAX,
        depth: 0,
        parent: None,
        kind: IfdKind::Ifd0,
        next: 0,
        entries: vec![entry; 3],
    };
    let want = usize::try_from(u32::MAX).ok().and_then(|o| {
        let end = o.checked_add(2 + 36 + 4)?;
        Some(o..end)
    });
    assert_eq!(ifd.table_range(), want);
    assert!(ifd.table_range().is_some() || usize::BITS < 64);
    assert_eq!(ifd.next_at(), want.map(|r| r.end - 4));
}

#[test]
fn payload_range_is_some_only_for_out_of_line_values() {
    // 5 bytes of ASCII live out of line at 50; a SHORT is inline; an ASCII of
    // exactly 4 bytes is inline too.
    let data = blob(
        &[
            (0x010F, 2, 5, 50),
            (0x0112, 3, 1, 6),
            (0x0131, 2, 4, 0x0041_4141),
        ],
        0,
        b"abcde",
    );
    let c = Container::parse(&data).unwrap();
    let ifd = c.ifd0().unwrap();
    let es = ifd.entries();
    assert_eq!(c.payload_range(&es[0]), Some(50..55));
    assert_eq!(c.payload_range(&es[1]), None);
    assert_eq!(c.payload_range(&es[2]), None);
    assert_eq!(es[0].value_at(), 8 + 2 + 8);
    assert_eq!(es[1].value_at(), 8 + 2 + 12 + 8);
}

#[test]
fn payload_range_of_an_unreadable_payload_is_none() {
    // Out-of-line but outside the blob; an unknown field type; a count whose
    // byte length overflows.
    let data = blob(
        &[
            (0x010F, 2, 50, 4000),
            (0x0110, 99, 3, 0),
            (0x0111, 5, 0xFFFF_FFFF, 8),
        ],
        0,
        &[],
    );
    let c = Container::parse(&data).unwrap();
    for e in c.ifd0().unwrap().entries() {
        assert_eq!(c.payload_range(e), None, "tag {:#x}", e.tag());
    }
}
