//! CRC-32 as PNG uses it (IEEE 802.3, reflected, polynomial `0xEDB88320`),
//! over a chunk's type and data (DEC-017). Table-driven, no dependency.

/// The 256-entry lookup table, built at compile time.
const TABLE: [u32; 256] = build_table();

const fn build_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    // Walk the array with `split_first_mut` rather than indexing it: the crate
    // denies `indexing_slicing`, and this keeps the build free of any index.
    let mut rest: &mut [u32] = &mut table;
    let mut n: u32 = 0;
    while let Some((slot, tail)) = rest.split_first_mut() {
        let mut c = n;
        let mut k: u32 = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k = k.wrapping_add(1);
        }
        *slot = c;
        rest = tail;
        n = n.wrapping_add(1);
    }
    table
}

/// Fold `data` into a running (pre-inverted) CRC state.
fn update(mut state: u32, data: &[u8]) -> u32 {
    for &b in data {
        let idx = usize::from((state as u8) ^ b);
        let entry = TABLE.get(idx).copied().unwrap_or(0);
        state = entry ^ (state >> 8);
    }
    state
}

/// CRC-32 of `first` followed by `second`, as if they were one slice.
pub(crate) fn crc32_parts(first: &[u8], second: &[u8]) -> u32 {
    !update(update(u32::MAX, first), second)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_value_of_123456789() {
        assert_eq!(crc32_parts(b"123456789", b""), 0xCBF4_3926);
        assert_eq!(crc32_parts(b"1234", b"56789"), 0xCBF4_3926);
    }

    #[test]
    fn iend_chunk_crc() {
        assert_eq!(crc32_parts(b"IEND", b""), 0xAE42_6082);
    }

    #[test]
    fn empty_is_zero_and_table_spot_checks() {
        assert_eq!(crc32_parts(b"", b""), 0);
        assert_eq!(TABLE.first().copied(), Some(0));
        assert_eq!(TABLE.get(1).copied(), Some(0x7707_3096));
        assert_eq!(TABLE.get(255).copied(), Some(0x2D02_EF8D));
    }
}
