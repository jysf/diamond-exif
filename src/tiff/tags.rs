//! Baseline-TIFF tag numbers the walker and its tests use (TIFF 6.0 §8).
//!
//! The baseline tags are copied from irradiance `src/ifd.rs:93-115`, DNG tags
//! removed. The three Exif pointer tags and the ASCII tags (`Make`, `Model`, ...)
//! are net-new (SPEC-004, DEC-005; SPEC-007).

/// TIFF 6.0: `NewSubfileType`.
pub const NEW_SUBFILE_TYPE: u16 = 254;
/// TIFF 6.0: `ImageWidth`.
pub const IMAGE_WIDTH: u16 = 256;
/// TIFF 6.0: `ImageLength` (height).
pub const IMAGE_LENGTH: u16 = 257;
/// TIFF 6.0: `BitsPerSample`.
pub const BITS_PER_SAMPLE: u16 = 258;
/// TIFF 6.0: `Compression`.
pub const COMPRESSION: u16 = 259;
/// TIFF 6.0: `PhotometricInterpretation`.
pub const PHOTOMETRIC: u16 = 262;
/// TIFF 6.0: `ImageDescription` (`ASCII`).
pub const IMAGE_DESCRIPTION: u16 = 270;
/// TIFF 6.0: `Make` (`ASCII`).
pub const MAKE: u16 = 271;
/// TIFF 6.0: `Model` (`ASCII`).
pub const MODEL: u16 = 272;
/// TIFF 6.0: `StripOffsets`.
pub const STRIP_OFFSETS: u16 = 273;
/// TIFF 6.0: `Orientation` (also an Exif tag).
pub const ORIENTATION: u16 = 274;
/// TIFF 6.0: `SamplesPerPixel`.
pub const SAMPLES_PER_PIXEL: u16 = 277;
/// TIFF 6.0: `RowsPerStrip`.
pub const ROWS_PER_STRIP: u16 = 278;
/// TIFF 6.0: `StripByteCounts`.
pub const STRIP_BYTE_COUNTS: u16 = 279;
/// TIFF 6.0: `XResolution` (`RATIONAL`).
pub const X_RESOLUTION: u16 = 282;
/// TIFF 6.0: `Software` (`ASCII`).
pub const SOFTWARE: u16 = 305;
/// TIFF 6.0: `DateTime` (`ASCII`, `YYYY:MM:DD HH:MM:SS`).
pub const DATE_TIME: u16 = 306;
/// TIFF 6.0: `Artist` (`ASCII`).
pub const ARTIST: u16 = 315;
/// TIFF/EP: `SubIFDs`, the offsets the walk recurses into.
pub const SUB_IFDS: u16 = 330;
/// Exif 2.32 (CIPA DC-008) section 4.6.3: `ExifIFDPointer`, the offset of the
/// Exif IFD.
pub const EXIF_IFD: u16 = 0x8769;
/// Exif 2.32 section 4.6.3: `GPSInfoIFDPointer`, the offset of the GPS IFD.
pub const GPS_INFO: u16 = 0x8825;
/// Exif 2.32 section 4.6.3: `InteroperabilityIFDPointer`, the offset of the
/// Interoperability IFD (found inside the Exif IFD).
pub const INTEROP_IFD: u16 = 0xA005;
/// TIFF 6.0 / Exif 2.32: `Copyright` (`ASCII`).
pub const COPYRIGHT: u16 = 33432;
/// Exif 2.32 section 4.6.5: `DateTimeOriginal` (`ASCII`), in the Exif IFD.
pub const DATE_TIME_ORIGINAL: u16 = 0x9003;
