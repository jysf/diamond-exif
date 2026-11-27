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
