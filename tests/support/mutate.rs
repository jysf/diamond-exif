//! Byte-mutation helpers for the red-proofs (SPEC-008). Net-new; nothing here
//! is ported.
//!
//! Every function is TOTAL: an out-of-range request leaves the bytes unchanged
//! and says so (`false` / `None`); none can panic, none indexes. A red-proof is
//! only worth having if the tool that corrupts the input cannot itself blow up
//! on the input it is asked to corrupt.
//!
//! [`locate_entry`] is a tiny reader that is independent of `exif-core` on
//! purpose: a test that corrupts "the GPS pointer value" must not find that
//! value with the library under test.
//!
//! Test support only (`#[path]`-included). std only.

#![allow(dead_code)]

/// The byte order a helper reads or writes in. Local to the test support so it
/// does not depend on `exif-core`'s own type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    Little,
    Big,
}

/// The byte order the file's own header claims (`II` / `MM`), if any.
pub fn order_of(bytes: &[u8]) -> Option<Order> {
    match bytes.get(0..2)? {
        b"II" => Some(Order::Little),
        b"MM" => Some(Order::Big),
        _ => None,
    }
}

/// Read a `u16` at `at`, or `None` when it does not fit.
pub fn read_u16_at(bytes: &[u8], at: usize, order: Order) -> Option<u16> {
    let end = at.checked_add(2)?;
    let raw: [u8; 2] = bytes.get(at..end)?.try_into().ok()?;
    Some(match order {
        Order::Little => u16::from_le_bytes(raw),
        Order::Big => u16::from_be_bytes(raw),
    })
}

/// Read a `u32` at `at`, or `None` when it does not fit.
pub fn read_u32_at(bytes: &[u8], at: usize, order: Order) -> Option<u32> {
    let end = at.checked_add(4)?;
    let raw: [u8; 4] = bytes.get(at..end)?.try_into().ok()?;
    Some(match order {
        Order::Little => u32::from_le_bytes(raw),
        Order::Big => u32::from_be_bytes(raw),
    })
}

/// Overwrite two bytes at `at` with `value` in `order`. `false`, bytes
/// unchanged, when `at..at + 2` is not inside `bytes`.
pub fn set_u16_at(bytes: &mut [u8], at: usize, value: u16, order: Order) -> bool {
    let raw = match order {
        Order::Little => value.to_le_bytes(),
        Order::Big => value.to_be_bytes(),
    };
    write_at(bytes, at, &raw)
}

/// Overwrite four bytes at `at` with `value` in `order`. `false`, bytes
/// unchanged, when `at..at + 4` is not inside `bytes`.
pub fn set_u32_at(bytes: &mut [u8], at: usize, value: u32, order: Order) -> bool {
    let raw = match order {
        Order::Little => value.to_le_bytes(),
        Order::Big => value.to_be_bytes(),
    };
    write_at(bytes, at, &raw)
}

fn write_at(bytes: &mut [u8], at: usize, raw: &[u8]) -> bool {
    let Some(end) = at.checked_add(raw.len()) else {
        return false;
    };
    match bytes.get_mut(at..end) {
        Some(window) => {
            window.copy_from_slice(raw);
            true
        }
        None => false,
    }
}

/// Flip bit `bit` (0 = least significant, up to 7) of byte `byte_index`.
/// `false`, bytes unchanged, for `bit > 7` or an index past the end. Flipping
/// twice restores the original.
pub fn flip_bit(bytes: &mut [u8], byte_index: usize, bit: u8) -> bool {
    if bit > 7 {
        return false;
    }
    match bytes.get_mut(byte_index) {
        Some(b) => {
            *b ^= 1 << bit;
            true
        }
        None => false,
    }
}

/// Shorten to `len` bytes. A `len` at or past the end is a no-op.
pub fn truncate_at(bytes: &mut Vec<u8>, len: usize) {
    bytes.truncate(len);
}

/// `II` to `MM` and back, touching ONLY bytes 0..2. The rest of the file is
/// left exactly as written, so it now lies about its byte order. `false`,
/// bytes unchanged, when the first two bytes are neither mark.
pub fn swap_byte_order_mark(bytes: &mut [u8]) -> bool {
    let swapped: &[u8; 2] = match bytes.get(0..2) {
        Some(b"II") => b"MM",
        Some(b"MM") => b"II",
        _ => return false,
    };
    write_at(bytes, 0, swapped)
}

/// Where a tag's 12-byte IFD entry sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntryLoc {
    /// Offset of the entry's first byte (its tag field).
    pub entry_at: usize,
    /// Offset of the entry's 4-byte value/offset field (`entry_at + 8`).
    pub value_at: usize,
}

/// Find `tag`'s entry in the IFD at `ifd_offset`: an independent reader (not
/// `exif-core`) so a test can corrupt "the GPS pointer value" without
/// hard-coding offsets. `None` when the header is not TIFF, the IFD or its
/// entry table is not inside `bytes`, or the tag is absent. An entry table cut
/// short by the end of the file is searched as far as it goes.
pub fn locate_entry(bytes: &[u8], ifd_offset: usize, tag: u16) -> Option<EntryLoc> {
    let order = order_of(bytes)?;
    let count = usize::from(read_u16_at(bytes, ifd_offset, order)?);
    let table = ifd_offset.checked_add(2)?;
    for i in 0..count {
        let entry_at = table.checked_add(i.checked_mul(12)?)?;
        let this = read_u16_at(bytes, entry_at, order)?;
        if this == tag {
            let value_at = entry_at.checked_add(8)?;
            // The whole 12-byte entry must be present.
            if entry_at.checked_add(12)? > bytes.len() {
                return None;
            }
            return Some(EntryLoc { entry_at, value_at });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_u32_at_writes_both_byte_orders_and_rejects_out_of_range() {
        let mut b = vec![0u8; 8];
        assert!(set_u32_at(&mut b, 2, 0x0102_0304, Order::Little));
        assert_eq!(b, [0, 0, 4, 3, 2, 1, 0, 0]);
        assert!(set_u32_at(&mut b, 2, 0x0102_0304, Order::Big));
        assert_eq!(b, [0, 0, 1, 2, 3, 4, 0, 0]);
        // Last position that fits.
        assert!(set_u32_at(&mut b, 4, 0xAABB_CCDD, Order::Big));
        assert_eq!(b, [0, 0, 1, 2, 0xAA, 0xBB, 0xCC, 0xDD]);
        // One past: unchanged, reported.
        let before = b.clone();
        assert!(!set_u32_at(&mut b, 5, 1, Order::Big));
        assert!(!set_u32_at(&mut b, 8, 1, Order::Big));
        assert!(!set_u32_at(&mut b, usize::MAX, 1, Order::Little));
        assert_eq!(b, before);
    }

    #[test]
    fn set_u16_at_boundary() {
        let mut b = vec![0u8; 4];
        assert!(set_u16_at(&mut b, 2, 0x0102, Order::Little));
        assert_eq!(b, [0, 0, 2, 1]);
        assert!(set_u16_at(&mut b, 0, 0x0102, Order::Big));
        assert_eq!(b, [1, 2, 2, 1]);
        // index == len and len - 1 (only 1 byte left) both refuse.
        let before = b.clone();
        assert!(!set_u16_at(&mut b, 4, 9, Order::Little));
        assert!(!set_u16_at(&mut b, 3, 9, Order::Little));
        assert!(!set_u16_at(&mut b, usize::MAX, 9, Order::Big));
        assert_eq!(b, before);
        // Empty input.
        let mut empty: Vec<u8> = Vec::new();
        assert!(!set_u16_at(&mut empty, 0, 1, Order::Little));
        assert!(empty.is_empty());
    }

    #[test]
    fn flip_bit_twice_restores_and_rejects_bit_8() {
        let mut b = vec![0b0000_0000u8, 0xFF];
        assert!(flip_bit(&mut b, 0, 0));
        assert_eq!(b[0], 0b0000_0001);
        assert!(flip_bit(&mut b, 0, 7));
        assert_eq!(b[0], 0b1000_0001);
        assert!(flip_bit(&mut b, 0, 7));
        assert!(flip_bit(&mut b, 0, 0));
        assert_eq!(b, [0, 0xFF]);
        let before = b.clone();
        assert!(!flip_bit(&mut b, 0, 8), "bit 8 is out of range");
        assert!(!flip_bit(&mut b, 0, u8::MAX));
        assert!(!flip_bit(&mut b, 2, 0), "index == len is out of range");
        assert!(!flip_bit(&mut b, usize::MAX, 0));
        assert_eq!(b, before);
    }

    #[test]
    fn truncate_at_beyond_the_end_is_a_no_op() {
        let mut b = vec![1u8, 2, 3, 4];
        truncate_at(&mut b, 4);
        assert_eq!(b, [1, 2, 3, 4], "len == length is a no-op");
        truncate_at(&mut b, 99);
        assert_eq!(b, [1, 2, 3, 4], "len past the end is a no-op");
        truncate_at(&mut b, usize::MAX);
        assert_eq!(b, [1, 2, 3, 4]);
        truncate_at(&mut b, 2);
        assert_eq!(b, [1, 2]);
        truncate_at(&mut b, 0);
        assert!(b.is_empty());
    }

    #[test]
    fn swap_byte_order_mark_flips_only_the_mark_and_rejects_non_tiff() {
        let mut b = b"II*\0rest".to_vec();
        assert!(swap_byte_order_mark(&mut b));
        assert_eq!(b, b"MM*\0rest");
        assert!(swap_byte_order_mark(&mut b));
        assert_eq!(b, b"II*\0rest");
        // Not a TIFF mark, too short, mixed marks: unchanged, reported.
        for bad in [&b"JPEG"[..], b"", b"I", b"IM", b"mm", b"\xFF\xD8"] {
            let mut v = bad.to_vec();
            assert!(!swap_byte_order_mark(&mut v), "{bad:?}");
            assert_eq!(v, bad);
        }
    }

    /// A little-endian IFD at 8 with entries for tags 0x0100 and 0x8825, and a
    /// big-endian twin, built by hand here.
    fn tiny(order: Order) -> Vec<u8> {
        let mark: &[u8] = if order == Order::Little { b"II" } else { b"MM" };
        let mut b = mark.to_vec();
        b.extend([0u8; 6]);
        assert!(set_u16_at(&mut b, 2, 42, order));
        assert!(set_u32_at(&mut b, 4, 8, order));
        b.extend([0u8; 2 + 24 + 4]);
        assert!(set_u16_at(&mut b, 8, 2, order));
        assert!(set_u16_at(&mut b, 10, 0x0100, order));
        assert!(set_u16_at(&mut b, 22, 0x8825, order));
        b
    }

    #[test]
    fn locate_entry_finds_a_tag_and_none_for_absent_or_out_of_range() {
        for order in [Order::Little, Order::Big] {
            let b = tiny(order);
            assert_eq!(
                locate_entry(&b, 8, 0x0100),
                Some(EntryLoc {
                    entry_at: 10,
                    value_at: 18
                })
            );
            assert_eq!(
                locate_entry(&b, 8, 0x8825),
                Some(EntryLoc {
                    entry_at: 22,
                    value_at: 30
                })
            );
            assert_eq!(locate_entry(&b, 8, 0x0101), None, "absent tag");
            assert_eq!(locate_entry(&b, b.len(), 0x0100), None, "ifd at the end");
            assert_eq!(locate_entry(&b, usize::MAX, 0x0100), None, "huge offset");
            // Entry table cut inside the second entry: the first is found, the
            // second (not wholly present) is not.
            let cut = &b[..30];
            assert!(locate_entry(cut, 8, 0x0100).is_some());
            assert_eq!(locate_entry(cut, 8, 0x8825), None);
        }
        // Not a TIFF at all.
        assert_eq!(locate_entry(b"JPEG0123456789", 8, 1), None);
        assert_eq!(locate_entry(b"", 0, 1), None);
        // A count that claims far more entries than the file holds terminates.
        let mut b = tiny(Order::Little);
        assert!(set_u16_at(&mut b, 8, u16::MAX, Order::Little));
        assert_eq!(locate_entry(&b, 8, 0x1234), None);
    }
}
