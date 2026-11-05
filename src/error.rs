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
                let (a, b) = (mark.first().copied(), mark.last().copied());
                write!(
                    f,
                    "not a JPEG: first bytes are {:#04x}{:02x}, expected 0xffd8",
                    a.unwrap_or(0),
                    b.unwrap_or(0)
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
                write!(f, "JPEG walk reached the limit of {limit} issues")
            }
        }
    }
}

impl core::error::Error for Error {}
