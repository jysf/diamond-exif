//! The tables of the Exif block's live completeness sweep (SPEC-025, AC9(b)):
//! what ExifTool 13.55 was measured to treat as offset-carrying in the Exif and
//! Interop IFDs (DEC-014's amendment of 2026-11-27, *Design measurements* of
//! the spec), and the extras the block omits on purpose. Typed by hand from the
//! measurement, never read from the library; `omission_table_mirrors_agree` in
//! `tests/exif_block.rs` compares them with the other mirrors of the decision.

#![allow(dead_code)]

/// The writer pass (`exiftool -Artist=sweep -o`) refuses a LONG in the Exif or
/// Interop IFD under these ids with `No size tag for <IFD>:<Name>`: ExifTool
/// relocates the value, and needs the length partner to do so.
pub const WRITER_NO_SIZE_TAG: [u16; 11] = [
    0x0111, 0x0120, 0x0144, 0x0201, 0x0207, 0x0208, 0x0209, 0x8781, 0xA010, 0xBCC0, 0xBCC2,
];

/// The writer pass's `[minor]` refusals without `-m` (ExifTool 13.55): about
/// the swept value 4096, not an offset, so they are not in the offset set.
/// Each happens in the Exif and the Interop IFD.
pub const WRITER_MINOR_REFUSALS: [(u16, &str); 2] = [
    (0xC612, "[minor] DNG Version 4096 not yet tested"),
    (0x927C, "[minor] Maker notes could not be parsed"),
];

/// The one id whose value the writer pass changes (it follows it as a SubIFD
/// and relocates it) and whose target the reader pass sees move.
pub const SUBIFD_TAG: u16 = 0x014A;

/// The ids the block omits although ExifTool was not measured to relocate
/// them, for being offsets or pointers by name (0x0190, 0x888A, 0x8290,
/// 0xFE00), the four blob carriers, and the pointer tags.
pub const CONSERVATIVE_EXTRAS: [u16; 8] = [
    0x0190, 0x888A, 0x8290, 0xFE00, 0xC51B, 0x8568, 0xCD41, 0xC6F5,
];
pub const POINTER_EXTRAS: [u16; 4] = [0x014A, 0x8769, 0x8825, 0xA005];

/// The MakerNote tags, omitted for being a MakerNote and not for an offset.
pub const MAKERNOTE: [u16; 2] = [0x927C, 0xEA1D];

// ── SPEC-041: the ByteOrder table, as the cross-order sweep expects it ──────
//
// DEC-014's amendment of 2026-11-28, item 3. Typed by hand once more, as
// bit masks (bit `n` is declared type `n`), so it is not a copy of the other
// mirrors' type lists. `byte_order_mirrors_agree` compares all of them.

/// Types 3, 4, 5, 8, 9, 10, 11, 12.
pub const MASK_MULTI: u16 = 0b0001_1111_0011_1000;
/// Types 1, 2, 6, 7.
pub const MASK_ONE: u16 = 0b0000_0000_1100_0110;
/// Types 1 to 12.
pub const MASK_ALL: u16 = 0b0001_1111_1111_1110;

/// Exif and Interop IFDs: every multi-byte type.
pub const BYTE_ORDER_MULTI_TAGS: [u16; 35] = [
    0x0303, 0x4747, 0x828E, 0x8298, 0x9009, 0x9101, 0x9286, 0x9C9B, 0x9C9C, 0x9C9D, 0x9C9E, 0x9C9F,
    0xBC01, 0xC615, 0xC65D, 0xC68B, 0xC68C, 0xC68F, 0xC691, 0xC6D2, 0xC6D3, 0xC6F3, 0xC6F4, 0xC6F6,
    0xC6F8, 0xC6FE, 0xC716, 0xC717, 0xC718, 0xC719, 0xC71C, 0xC71D, 0xC7A7, 0xC7AA, 0xCD48,
];

/// Exif and Interop IFDs: the rows with another mask.
pub const BYTE_ORDER_OTHER: [(u16, u16); 9] = [
    (0x87AF, 1 << 12),
    (0x9201, MASK_ONE),
    (0x9204, 0b0001_0001_1100_1110),
    (0xA101, 0b0001_1110_0011_0000),
    (0xA302, MASK_ALL & !(1 << 7)),
    (0xA462, MASK_ALL),
    (0x8828, MASK_ALL),
    (0xA20C, MASK_ALL),
    (0xA40B, MASK_ALL),
];

/// The three the Exif specification starts with two SHORTs: ExifTool prints
/// them as undecoded bytes, so the sweep cannot see them; the table minus the
/// sweep's sensitive set is exactly these (AC9(b), Part B).
pub const NOT_SEEN_BY_THE_SWEEP: [u16; 3] = [0x8828, 0xA20C, 0xA40B];

/// GPS IFD.
pub const BYTE_ORDER_GPS: [(u16, u16); 1] = [(0x001D, 0b0001_1110_0011_0000)];

/// Every `(IFD, tag, type)` of the table: IFD 0 is Exif, 1 Interop, 2 GPS.
pub fn byte_order_triples() -> Vec<(u8, u16, u16)> {
    let mut out = Vec::new();
    for ifd in [0u8, 1] {
        for tag in BYTE_ORDER_MULTI_TAGS {
            for ty in 1..=12u16 {
                if MASK_MULTI & (1 << ty) != 0 {
                    out.push((ifd, tag, ty));
                }
            }
        }
        for (tag, mask) in BYTE_ORDER_OTHER {
            for ty in 1..=12u16 {
                if mask & (1 << ty) != 0 {
                    out.push((ifd, tag, ty));
                }
            }
        }
    }
    for (tag, mask) in BYTE_ORDER_GPS {
        for ty in 1..=12u16 {
            if mask & (1 << ty) != 0 {
                out.push((2, tag, ty));
            }
        }
    }
    out
}
