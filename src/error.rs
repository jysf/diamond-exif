//! The crate's error type: the TIFF-level failures the strict walker can
//! produce. `#[non_exhaustive]`, so later specs add variants without a breaking
//! change.
//!
//! Provenance: copied from irradiance `src/lib.rs` (the `Error` enum and its
//! `Display`) and edited down to the variants that still have a producer here
//! (DEC-003; see `docs/provenance-ledger.md`).

use core::fmt;

/// `Result` with this crate's [`Error`].
pub type Result<T> = core::result::Result<T, Error>;

/// Errors produced while reading a TIFF container.
///
/// Every fallible path in this crate returns this type rather than panicking
/// (constraint `panic-free-library`). `#[non_exhaustive]` because later specs
/// (tolerant walk, pointer-tag walk, decoders) add variants, and a match
/// outside the crate must not assume today's set is complete.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The input ended before the expected data could be read.
    ///
    /// `at` is the byte offset the read started at; `len` is the number of
    /// bytes that were needed from that offset.
    Truncated {
        /// Byte offset the truncated read started at.
        at: usize,
        /// Number of bytes the read needed but did not have.
        len: usize,
    },

    /// The first two bytes were neither `II` nor `MM`, so this is not a TIFF
    /// container (TIFF 6.0 §2, "Image File Header").
    NotTiff {
        /// The two bytes actually found.
        mark: [u8; 2],
    },

    /// The byte-order mark was valid but the version word was not 42.
    UnsupportedTiffVersion {
        /// The version word actually found.
        version: u16,
    },

    /// An IFD offset was reached twice. The cycle guard: a `SubIFD` chain that
    /// points at itself terminates here instead of recursing forever.
    CyclicIfd {
        /// The offset that was about to be visited a second time.
        offset: u32,
    },

    /// `SubIFD` recursion went deeper than [`crate::tiff::MAX_IFD_DEPTH`].
    IfdDepthExceeded {
        /// The depth limit that was exceeded.
        limit: u32,
    },

    /// The walk reached [`crate::tiff::MAX_IFDS`] directories. A chain long
    /// enough to hit this is hostile even when it is acyclic.
    TooManyIfds {
        /// The IFD-count limit that was reached.
        limit: usize,
    },

    /// A tag was present but carried a field type the requested reader does
    /// not accept (or a type this crate does not know the size of).
    UnexpectedFieldType {
        /// The tag whose type was unexpected.
        tag: u16,
        /// The TIFF field type found (TIFF 6.0 §2, "Types").
        field_type: u16,
    },

    /// `count × sizeof(type)` overflowed, or a value did not fit the type it
    /// was being converted into.
    ValueOverflow {
        /// The tag whose size arithmetic overflowed.
        tag: u16,
    },

    /// A tag declared more values than [`crate::tiff::MAX_TAG_VALUES`]. Bounds
    /// allocation from a hostile `count` independently of the file's length.
    TagTooLarge {
        /// The tag with the outsized count.
        tag: u16,
        /// The count the file declared.
        count: u32,
        /// The limit that was exceeded.
        limit: usize,
    },

    /// A tag the caller required is not present in the IFD.
    MissingTag {
        /// The absent tag.
        tag: u16,
    },

    /// An offset in the file is larger than this platform can address.
    OffsetOutOfRange {
        /// The offset that could not be represented as a `usize`.
        offset: u32,
    },

    /// A pointer tag (Exif, GPS or Interop IFD) must carry exactly one offset.
    BadPointerCount {
        /// The pointer tag.
        tag: u16,
        /// The count the file declared.
        count: u32,
    },

    /// The first two bytes were not `FF D8` (SOI), so this is not a JPEG
    /// (ITU-T T.81 Annex B). Fatal (DEC-006).
    NotJpeg {
        /// The two bytes actually found.
        mark: [u8; 2],
    },

    /// A run of bytes between segments that belongs to no marker, or a
    /// marker that is not allowed outside entropy data (`FF 00`, a stray
    /// `SOI`, `RSTn`, `TEM`). One issue per maximal run.
    StrayBytes {
        /// Offset of the first stray byte.
        at: usize,
        /// Length of the run in bytes.
        len: usize,
    },

    /// A segment declared a length below 2, which cannot even cover its own
    /// length field.
    BadSegmentLength {
        /// The marker code of the segment.
        marker: u8,
        /// The length the file declared.
        length: u16,
    },

    /// The walk ended without finding a start-of-scan marker.
    MissingSos,

    /// The walk ended without finding an end-of-image marker.
    MissingEoi,

    /// The walk reached [`crate::jpeg::MAX_JPEG_SEGMENTS`] segments.
    TooManySegments {
        /// The segment-count limit that was reached.
        limit: usize,
    },

    /// The walk reached [`crate::jpeg::MAX_JPEG_ISSUES`] issues.
    TooManyIssues {
        /// The issue-count limit that was reached.
        limit: usize,
    },

    /// An Exif segment beyond the first in one JPEG. The first is the primary
    /// (DEC-006); each further one costs this JPEG-layer issue.
    ExtraExif {
        /// Input offset of the extra Exif segment's own `FF` marker byte.
        at: usize,
    },

    /// A located region's small header, or a resource inside it, does not fit
    /// in its segment. No partial region is reported.
    RegionTooShort {
        /// Which header or resource was short.
        what: &'static str,
        /// Input offset of the segment's own `FF` marker byte for a segment
        /// header (extended XMP, ICC, JUMBF), or of the resource itself for an
        /// `8BIM` resource inside an IPTC segment.
        at: usize,
        /// Bytes the header or resource needed. For a segment header this is
        /// counted from after the segment's signature (not from `at`); for an
        /// `8BIM` resource it is counted from the resource's start.
        need: usize,
        /// Bytes that were available, counted from the same point as `need`.
        have: usize,
    },

    /// The ICC profile chunks of one JPEG disagree among themselves (count,
    /// sequence number, repeated or missing chunk). One issue per JPEG.
    IccChunkMismatch {
        /// Input offset of the first segment found inconsistent.
        at: usize,
    },

    /// The walk would have retained more than [`crate::tiff::entry_budget`]
    /// entries in total (DEC-001): only overlapping IFD tables can do this,
    /// because in a real file every entry occupies its own 12 bytes. The IFD
    /// that would cross the budget is omitted and the walk ends.
    TooManyEntries {
        /// The total-entry budget that was reached (`data.len() / 12`).
        limit: usize,
    },

    /// An APP2 `MPF\0` segment (Multi-Picture Format, DEC-012) that does not
    /// parse: a bad TIFF header, no readable `0xB002` index, or a second MPF
    /// segment. The segment is reported as an unrecognised APPn instead.
    MpfMalformed {
        /// Input offset of the segment's own `FF` marker byte.
        at: usize,
    },

    /// An MPF index entry whose offset and size do not lie inside the input.
    /// No embedded image is reported for it.
    MpfImageOutsideInput {
        /// Input offset of the MPF segment's own `FF` marker byte.
        at: usize,
        /// The entry's index in the MP index (entry 0 is the primary image).
        index: usize,
    },

    /// An MPF index entry whose bytes do not start with `FF D8`. No embedded
    /// image is reported for it.
    MpfImageNotJpeg {
        /// Input offset of the MPF segment's own `FF` marker byte.
        at: usize,
        /// The entry's index in the MP index (entry 0 is the primary image).
        index: usize,
    },

    /// An MPF index lists more images than the limit (the primary included);
    /// the entries from the limit on are not examined.
    TooManyMpfImages {
        /// Input offset of the MPF segment's own `FF` marker byte.
        at: usize,
        /// The image-count limit that was reached.
        limit: usize,
    },

    /// The first eight bytes were not the PNG signature, so this is not a PNG
    /// (PNG Specification, "PNG file signature"). Fatal (DEC-017).
    NotPng {
        /// The eight bytes actually found.
        mark: [u8; 8],
    },

    /// A chunk's stored CRC-32 differs from the one computed over its type and
    /// data. The chunk stays recorded (DEC-017).
    BadChunkCrc {
        /// The CRC the file stored.
        stored: u32,
        /// The CRC computed over the chunk's type and data.
        computed: u32,
    },

    /// A chunk type with a byte that is not an ASCII letter. The chunk stays
    /// recorded as an unrecognised one.
    BadChunkType {
        /// The four type bytes actually found.
        chunk_type: [u8; 4],
    },

    /// A chunk declared a length above 2^31-1, the PNG maximum. The stream
    /// cannot be resynchronised, so the walk ends there.
    BadChunkLength {
        /// The length the file declared.
        length: u32,
    },

    /// The first chunk is not `IHDR`, or there is no chunk at all.
    MissingIhdr,

    /// An `IHDR` chunk whose length is not 13.
    BadIhdr {
        /// The length the file declared.
        length: u32,
    },

    /// The walk reached the end of the input on a chunk boundary without
    /// finding an `IEND` chunk.
    MissingIend,

    /// The walk reached [`crate::png::MAX_PNG_CHUNKS`] chunks and another
    /// chunk followed.
    TooManyChunks {
        /// The chunk-count limit that was reached.
        limit: usize,
    },

    /// An Exif payload that starts with the `Exif\0\0` header a JPEG APP1
    /// carries, which a PNG `eXIf` chunk must not have (DEC-018). The header is
    /// skipped and the rest is read.
    ImproperExifHeader {
        /// Input offset of the chunk's length field.
        at: usize,
    },

    /// A `tEXt`, `zTXt` or `iTXt` chunk whose keyword or header is malformed:
    /// no NUL within the first 80 bytes, an empty keyword, a truncated header,
    /// or an `iTXt` compression flag other than 0 or 1. No region is located.
    BadTextChunk {
        /// Input offset of the chunk's length field.
        at: usize,
    },

    /// An `iCCP` chunk with an empty or unterminated profile name, or no
    /// compression-method byte. No region is located.
    BadIccpChunk {
        /// Input offset of the chunk's length field.
        at: usize,
    },

    /// More than [`crate::png::MAX_PNG_REGIONS`] metadata regions were located;
    /// extraction stopped there.
    TooManyRegions {
        /// The region-count limit that was reached.
        limit: usize,
    },

    /// The first 12 bytes are not `RIFF`, a size and `WEBP`, so this is not a
    /// WebP (WebP Container Specification). Fatal (DEC-019).
    NotWebp {
        /// The 12 bytes actually found.
        mark: [u8; 12],
    },

    /// The RIFF size field is not the input length minus 8. The walk is not
    /// limited by it (DEC-019); this only reports the disagreement.
    RiffSizeMismatch {
        /// The size the file declared.
        declared: u32,
        /// The input length minus 8, the size an exact file would declare.
        actual: usize,
    },

    /// An odd-sized chunk ends exactly at the end of the input, so its pad
    /// byte is missing.
    MissingPad {
        /// Input offset of the chunk.
        at: usize,
    },

    /// A `VP8X` chunk shorter than the 10 bytes its header needs.
    BadVp8x {
        /// The chunk length the file declared.
        length: u32,
    },

    /// A `VP8X` chunk that is not the first chunk.
    MisplacedVp8x {
        /// Input offset of the chunk.
        at: usize,
    },

    /// A `VP8X` flag disagrees with the chunks present: the flag is set and no
    /// such chunk exists, or the chunk exists and the flag is clear. Flags are
    /// informational (DEC-019); this never stops a read.
    ChunkFlagMismatch {
        /// Which flag: `"ICC"`, `"EXIF"` or `"XMP"`.
        flag: &'static str,
        /// Whether the flag is set.
        declared: bool,
    },

    /// There is no `VP8X` chunk but an `EXIF`, `XMP ` or `ICCP` chunk exists.
    MetadataWithoutVp8x,

    /// A chunk code that is read as metadata although it is not the canonical
    /// four bytes (`XMP\0` for `XMP `).
    NonCanonicalChunkId {
        /// The four code bytes actually found.
        chunk_id: [u8; 4],
        /// Input offset of the chunk.
        at: usize,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Truncated { at, len } => {
                write!(f, "truncated input: needed {len} byte(s) at offset {at}")
            }
            Error::NotTiff { mark } => {
                let (a, b) = (mark.first().copied(), mark.last().copied());
                write!(
                    f,
                    "not a TIFF container: byte-order mark is {:#04x}{:#04x}, expected II or MM",
                    a.unwrap_or(0),
                    b.unwrap_or(0)
                )
            }
            Error::UnsupportedTiffVersion { version } => {
                write!(f, "unsupported TIFF version {version}, expected 42")
            }
            Error::CyclicIfd { offset } => {
                write!(f, "cyclic IFD: offset {offset} was already visited")
            }
            Error::IfdDepthExceeded { limit } => {
                write!(f, "SubIFD recursion exceeded the depth limit of {limit}")
            }
            Error::TooManyIfds { limit } => {
                write!(f, "IFD walk reached the limit of {limit} directories")
            }
            Error::TooManyEntries { limit } => {
                write!(f, "IFD walk reached the limit of {limit} retained entries")
            }
            Error::UnexpectedFieldType { tag, field_type } => {
                write!(f, "tag {tag} has unreadable field type {field_type}")
            }
            Error::ValueOverflow { tag } => {
                write!(f, "tag {tag}: value size arithmetic overflowed")
            }
            Error::TagTooLarge { tag, count, limit } => {
                write!(
                    f,
                    "tag {tag} declares {count} values, over the limit of {limit}"
                )
            }
            Error::MissingTag { tag } => write!(f, "required tag {tag} is absent"),
            Error::OffsetOutOfRange { offset } => {
                write!(f, "offset {offset} is not addressable on this platform")
            }
            Error::BadPointerCount { tag, count } => {
                write!(
                    f,
                    "pointer tag {tag:#06x} carries {count} value(s), expected exactly 1"
                )
            }
            Error::NotJpeg { mark } => {
                let [a, b] = *mark;
                write!(
                    f,
                    "not a JPEG: first bytes are {a:#04x}{b:02x}, expected 0xffd8"
                )
            }
            Error::StrayBytes { at, len } => {
                write!(
                    f,
                    "stray bytes: {len} byte(s) at offset {at} belong to no segment"
                )
            }
            Error::BadSegmentLength { marker, length } => {
                write!(
                    f,
                    "segment marker {marker:#04x} declares length {length}, below the minimum of 2"
                )
            }
            Error::MissingSos => write!(f, "no start-of-scan marker found"),
            Error::MissingEoi => write!(f, "no end-of-image marker found"),
            Error::TooManySegments { limit } => {
                write!(f, "JPEG walk reached the limit of {limit} segments")
            }
            Error::TooManyIssues { limit } => {
                write!(f, "walk reached the limit of {limit} issues")
            }
            Error::ExtraExif { at } => {
                write!(
                    f,
                    "extra Exif segment at offset {at} (the first is primary)"
                )
            }
            Error::RegionTooShort {
                what,
                at,
                need,
                have,
            } => {
                write!(
                    f,
                    "{what} at offset {at} is too short: needs {need} byte(s), has {have}"
                )
            }
            Error::IccChunkMismatch { at } => {
                write!(f, "ICC profile chunks are inconsistent from offset {at}")
            }
            Error::MpfMalformed { at } => {
                write!(f, "Multi-Picture segment at offset {at} does not parse")
            }
            Error::MpfImageOutsideInput { at, index } => {
                write!(
                    f,
                    "Multi-Picture entry {index} (segment at offset {at}) lies outside the input"
                )
            }
            Error::MpfImageNotJpeg { at, index } => {
                write!(
                    f,
                    "Multi-Picture entry {index} (segment at offset {at}) does not start with FF D8"
                )
            }
            Error::TooManyMpfImages { at, limit } => {
                write!(
                    f,
                    "Multi-Picture segment at offset {at} lists more than {limit} images"
                )
            }
            Error::NotPng { mark } => {
                write!(f, "not a PNG: first bytes are")?;
                for b in mark {
                    write!(f, " {b:02x}")?;
                }
                write!(f, ", expected 89 50 4e 47 0d 0a 1a 0a")
            }
            Error::BadChunkCrc { stored, computed } => {
                write!(
                    f,
                    "chunk CRC mismatch: stored {stored:#010x}, computed {computed:#010x}"
                )
            }
            Error::BadChunkType { chunk_type } => {
                write!(
                    f,
                    "chunk type {chunk_type:02x?} has a byte that is not an ASCII letter"
                )
            }
            Error::BadChunkLength { length } => {
                write!(
                    f,
                    "chunk declares length {length}, above the maximum of 2147483647"
                )
            }
            Error::MissingIhdr => write!(f, "the first chunk is not IHDR"),
            Error::BadIhdr { length } => {
                write!(f, "IHDR declares length {length}, expected 13")
            }
            Error::MissingIend => write!(f, "no IEND chunk found"),
            Error::TooManyChunks { limit } => {
                write!(f, "walk reached the limit of {limit} chunks")
            }
            Error::ImproperExifHeader { at } => {
                write!(
                    f,
                    "Exif chunk at offset {at} starts with an Exif header it should not have"
                )
            }
            Error::BadTextChunk { at } => {
                write!(
                    f,
                    "text chunk at offset {at} has a malformed keyword or header"
                )
            }
            Error::BadIccpChunk { at } => {
                write!(
                    f,
                    "iCCP chunk at offset {at} has a malformed profile name or header"
                )
            }
            Error::TooManyRegions { limit } => {
                write!(f, "more than {limit} metadata regions were located")
            }
            Error::NotWebp { mark } => {
                write!(f, "not a WebP: first bytes are")?;
                for b in mark {
                    write!(f, " {b:02x}")?;
                }
                write!(f, ", expected RIFF, a size and WEBP")
            }
            Error::RiffSizeMismatch { declared, actual } => {
                write!(
                    f,
                    "RIFF size field says {declared}, the input has {actual} bytes after it"
                )
            }
            Error::MissingPad { at } => {
                write!(f, "odd-sized chunk at offset {at} has no pad byte")
            }
            Error::BadVp8x { length } => {
                write!(f, "VP8X declares length {length}, expected at least 10")
            }
            Error::MisplacedVp8x { at } => {
                write!(f, "VP8X chunk at offset {at} is not the first chunk")
            }
            Error::ChunkFlagMismatch { flag, declared } => {
                if *declared {
                    write!(f, "VP8X sets the {flag} flag but no such chunk exists")
                } else {
                    write!(f, "a {flag} chunk exists but the VP8X flag is clear")
                }
            }
            Error::MetadataWithoutVp8x => {
                write!(f, "metadata chunks exist but there is no VP8X chunk")
            }
            Error::NonCanonicalChunkId { chunk_id, at } => {
                write!(
                    f,
                    "chunk at offset {at} has code {chunk_id:02x?}, read as metadata though not canonical"
                )
            }
        }
    }
}

impl core::error::Error for Error {}
