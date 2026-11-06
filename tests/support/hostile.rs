//! The memory-amplification shape (SPEC-012 measured it, SPEC-017 bounds it),
//! shared by `tests/audit_oracle.rs`, `tests/budget_oracle.rs` and the
//! recorder (`examples/oracle_record.rs`). std only.

#![allow(dead_code)]

/// `MAX_IFDS` (512) minus IFD0: the most SubIFDs one walk can visit.
pub const N_SUB: usize = 511;

/// The committed ExifTool record of the `N_SUB` shape (relative to the crate
/// root). Small: dashes for the selectors and the SubIFD warning in
/// `# stderr:`. The 789,516-byte file itself is never committed.
pub const RECORD: &str = "tests/oracle-fixtures/exiftool/hostile-subifds-overlap.txt";

/// A TIFF whose IFD0 holds one `SubIFDs` entry naming `n_sub` IFDs at offsets
/// two bytes apart. Each of them is "65,535 entries" because the bytes there
/// are all `0xFF`, so the entry tables overlap and the file is only a little
/// over 786 KB however many IFDs it names.
pub fn hostile(n_sub: usize) -> Vec<u8> {
    const ENTRIES: usize = 0xFFFF;
    let array_at = 8 + 2 + 12 + 4;
    let base = array_at + 4 * n_sub;
    let base = base + (base % 2);
    let mut out = b"II\x2A\x00\x08\x00\x00\x00".to_vec();
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&330u16.to_le_bytes());
    out.extend_from_slice(&4u16.to_le_bytes());
    out.extend_from_slice(&(n_sub as u32).to_le_bytes());
    out.extend_from_slice(&(array_at as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    for k in 0..n_sub {
        out.extend_from_slice(&((base + 2 * k) as u32).to_le_bytes());
    }
    while out.len() < base {
        out.push(0);
    }
    let region = 2 * n_sub.saturating_sub(1) + 2 + 12 * ENTRIES + 4;
    out.resize(base + region, 0xFF);
    // The next-IFD words sit in the last `2 * (n_sub - 1) + 4` bytes of the
    // region; zero them so no IFD chains on (a chained attempt would spend one
    // of the 512 IFD visits on a failure).
    let next_words = base + 2 + 12 * ENTRIES;
    for b in out.iter_mut().skip(next_words) {
        *b = 0;
    }
    out
}
