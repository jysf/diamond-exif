//! A test placer (SPEC-025): what a writer that places the block does, as
//! irradiance/SPEC-081 does at base 8 but at any even base. It writes the TIFF
//! header, zeros up to `base`, copies the block unchanged, then writes IFD0
//! after it, holding 0x8769 and 0x8825 as `base + position` (LONG, count 1, in
//! ascending tag order). It imports nothing from `exif_core`; SPEC-026 reuses
//! it.

#![allow(dead_code)]

/// The placed file. `exif_pos` and `gps_pos` are the block-relative positions
/// the builder reports. An empty block places no pointer and no gap: IFD0 is
/// at offset 8 and empty.
pub fn place(
    be: bool,
    base: u32,
    block: &[u8],
    exif_pos: Option<u32>,
    gps_pos: Option<u32>,
) -> Vec<u8> {
    let w16 = |v: u16| if be { v.to_be_bytes() } else { v.to_le_bytes() };
    let w32 = |v: u32| if be { v.to_be_bytes() } else { v.to_le_bytes() };
    let mut out = Vec::new();
    out.extend_from_slice(if be { b"MM\0\x2A" } else { b"II\x2A\0" });
    let ifd0_at = if block.is_empty() {
        8
    } else {
        base + block.len() as u32
    };
    out.extend_from_slice(&w32(ifd0_at));
    if !block.is_empty() {
        out.resize(base as usize, 0);
        out.extend_from_slice(block);
    }
    let mut ptrs = Vec::new();
    if let Some(p) = exif_pos {
        ptrs.push((0x8769u16, base + p));
    }
    if let Some(p) = gps_pos {
        ptrs.push((0x8825u16, base + p));
    }
    out.extend_from_slice(&w16(ptrs.len() as u16));
    for (tag, value) in ptrs {
        out.extend_from_slice(&w16(tag));
        out.extend_from_slice(&w16(4));
        out.extend_from_slice(&w32(1));
        out.extend_from_slice(&w32(value));
    }
    out.extend_from_slice(&w32(0));
    out
}
