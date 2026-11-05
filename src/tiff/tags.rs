//! Baseline-TIFF tag numbers the walker and its tests use (TIFF 6.0 §8).
//!
//! No pointer tags (Exif/GPS/Interop) yet: they land with their consumer in
//! SPEC-004. Copied from irradiance `src/ifd.rs:93-115`, DNG tags removed.

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
/// TIFF/EP: `SubIFDs`, the offsets the walk recurses into.
pub const SUB_IFDS: u16 = 330;
