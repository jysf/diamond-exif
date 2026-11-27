//! The relocatable Exif block (SPEC-025, DEC-014, DEC-015).
//!
//! [`exif_block`] reads a TIFF-structured source and plans a block holding the
//! source's Exif IFD, that IFD's Interop IFD and the GPS IFD, rebuilt in the
//! source's byte order. The plan reports the block's length (which does not
//! depend on where the block will sit), the two IFD positions relative to the
//! block, and every entry it left out. [`ExifBlock::write`] then produces
//! exactly those bytes for a caller-given even base, with final offsets, after
//! reading them back through the crate's own walk (DEC-015).
//!
//! Provenance (DEC-003, class 1): net-new code written from DEC-014 and
//! DEC-015 over this crate's own reader. No ExifTool source is read.

use core::fmt;

use crate::error::Error;
use crate::reader::ByteOrder;

/// The default budget for a block, and for the scratch image its self-check
/// reads back (DEC-015 clarification): 16 MiB.
pub const DEFAULT_MAX_BLOCK_LEN: u32 = 1 << 24;

/// Options for [`exif_block`].
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockOptions {
    max_len: u32,
}

impl Default for BlockOptions {
    fn default() -> Self {
        BlockOptions {
            max_len: DEFAULT_MAX_BLOCK_LEN,
        }
    }
}

impl BlockOptions {
    /// The default options: a budget of [`DEFAULT_MAX_BLOCK_LEN`].
    pub fn new() -> Self {
        BlockOptions::default()
    }

    /// Set the budget in bytes. It bounds the block, and `base + len + 34` for
    /// the self-check's scratch image (DEC-015 clarification).
    #[must_use]
    pub fn with_max_len(mut self, max_len: u32) -> Self {
        self.max_len = max_len;
        self
    }

    /// The budget in bytes.
    pub fn max_len(&self) -> u32 {
        self.max_len
    }
}

/// Which IFD of the block an entry belongs to.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockIfd {
    /// The Exif IFD (0x8769 from IFD0).
    Exif,
    /// The Interoperability IFD (0xA005 from the Exif IFD).
    Interop,
    /// The GPS IFD (0x8825 from IFD0).
    Gps,
}

/// Why an entry was left out of the block (DEC-014 amendment, items 2 to 4).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OmitReason {
    /// 0x927C MakerNote, or 0xEA1D OffsetSchema (which only describes one).
    MakerNote,
    /// The entry carries an offset the builder does not rewrite.
    OffsetCarrying,
    /// A field type the walk cannot size.
    UnknownType,
}

/// One entry the block leaves out.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Omission {
    ifd: BlockIfd,
    tag: u16,
    field_type: u16,
    reason: OmitReason,
}

impl Omission {
    /// The IFD the entry was in.
    pub fn ifd(&self) -> BlockIfd {
        self.ifd
    }

    /// The tag number.
    pub fn tag(&self) -> u16 {
        self.tag
    }

    /// The field type the source declared.
    pub fn field_type(&self) -> u16 {
        self.field_type
    }

    /// Why it was left out.
    pub fn reason(&self) -> OmitReason {
        self.reason
    }
}

/// Why the source cannot be built into a block (DEC-014 amendment item 4).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnbuildableReason {
    /// The walk was halted by a budget (`TooManyIfds` or `TooManyEntries`): a
    /// halt can hide an IFD.
    WalkHalted,
    /// An issue was raised following 0x8769 or 0x8825 from IFD0, or 0xA005
    /// from the Exif IFD.
    BrokenPointer {
        /// The pointer tag.
        tag: u16,
    },
    /// IFD0 holds 0x8769 or 0x8825 twice, or the Exif IFD holds 0xA005 twice.
    DuplicatePointer {
        /// The pointer tag.
        tag: u16,
    },
    /// One of the three IFDs holds a tag twice.
    DuplicateTag {
        /// The IFD.
        ifd: BlockIfd,
        /// The tag.
        tag: u16,
    },
    /// A kept entry's value cannot be read from the source.
    UnreadableValue {
        /// The IFD.
        ifd: BlockIfd,
        /// The tag.
        tag: u16,
    },
}

/// The self-check that failed (DEC-015 clarification).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockCheck {
    /// The scratch image did not parse clean.
    Parse,
    /// An IFD is not where the plan says.
    Positions,
    /// An entry differs from the plan's.
    Entries,
    /// The omissions do not account for exactly what the block lacks.
    Omissions,
    /// The block's length, or a value's place, is not the plan's.
    Length,
}

/// Why no block was returned.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockError {
    /// The source did not parse: a bad header, BigTIFF, an unreadable IFD0.
    Source(Error),
    /// The source parses but cannot be built into a block.
    Unbuildable {
        /// Why.
        reason: UnbuildableReason,
    },
    /// The block, or `base + block + 34` for the self-check, exceeds the
    /// budget.
    OverBudget {
        /// The bytes needed.
        needed: u64,
        /// The budget.
        budget: u32,
    },
    /// The base is odd, lies inside the TIFF header, or `base + len` does not
    /// fit a `u32`.
    BadBase {
        /// The base given.
        base: u32,
    },
    /// `write_into` was given a slice of the wrong length.
    BufferLength {
        /// The block's length.
        expected: u32,
        /// The slice's length.
        got: usize,
    },
    /// The bytes failed their own readback: the plan and the check disagree.
    VerifyFailed {
        /// The check that failed.
        check: BlockCheck,
    },
}

impl fmt::Display for BlockIfd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            BlockIfd::Exif => "Exif IFD",
            BlockIfd::Interop => "Interop IFD",
            BlockIfd::Gps => "GPS IFD",
        })
    }
}

impl fmt::Display for OmitReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            OmitReason::MakerNote => "a MakerNote, which is never moved",
            OmitReason::OffsetCarrying => "it carries an offset the block does not rewrite",
            OmitReason::UnknownType => "its field type cannot be sized",
        })
    }
}

impl fmt::Display for Omission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} tag {:#06x} (type {}) left out: {}",
            self.ifd, self.tag, self.field_type, self.reason
        )
    }
}

impl fmt::Display for UnbuildableReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnbuildableReason::WalkHalted => write!(f, "the walk was halted by a budget"),
            UnbuildableReason::BrokenPointer { tag } => {
                write!(f, "the pointer {tag:#06x} could not be followed")
            }
            UnbuildableReason::DuplicatePointer { tag } => {
                write!(f, "the pointer {tag:#06x} appears twice")
            }
            UnbuildableReason::DuplicateTag { ifd, tag } => {
                write!(f, "the {ifd} holds tag {tag:#06x} twice")
            }
            UnbuildableReason::UnreadableValue { ifd, tag } => {
                write!(f, "the value of tag {tag:#06x} in the {ifd} cannot be read")
            }
        }
    }
}

impl fmt::Display for BlockCheck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            BlockCheck::Parse => "parse",
            BlockCheck::Positions => "positions",
            BlockCheck::Entries => "entries",
            BlockCheck::Omissions => "omissions",
            BlockCheck::Length => "length",
        })
    }
}

impl fmt::Display for BlockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockError::Source(error) => write!(f, "the source is not readable: {error}"),
            BlockError::Unbuildable { reason } => {
                write!(f, "the source cannot be built into a block: {reason}")
            }
            BlockError::OverBudget { needed, budget } => {
                write!(f, "{needed} bytes are needed, over the budget of {budget}")
            }
            BlockError::BadBase { base } => write!(f, "the base {base} is not usable"),
            BlockError::BufferLength { expected, got } => {
                write!(
                    f,
                    "the output slice holds {got} bytes, the block {expected}"
                )
            }
            BlockError::VerifyFailed { check } => {
                write!(f, "the block failed its own readback ({check})")
            }
        }
    }
}

impl std::error::Error for BlockError {}

/// One kept entry. `payload` borrows the source.
#[derive(Debug, Clone)]
struct Kept<'a> {
    tag: u16,
    field_type: u16,
    count: u32,
    payload: &'a [u8],
    /// 0xA005 in the Exif IFD: written as LONG, count 1, holding the Interop
    /// IFD's final offset.
    interop_pointer: bool,
    /// Block-relative offset of an out-of-line value; `None` when inline.
    value_at: Option<u32>,
}

/// One IFD of the block.
#[derive(Debug, Clone)]
struct Table<'a> {
    ifd: BlockIfd,
    /// Block-relative offset of the table.
    at: u32,
    kept: Vec<Kept<'a>>,
    /// `(tag, type)` of every entry the source's IFD held, kept or not: what
    /// the self-check accounts for.
    source: Vec<(u16, u16)>,
}

/// A planned block (DEC-014, DEC-015).
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct ExifBlock<'a> {
    order: ByteOrder,
    tables: Vec<Table<'a>>,
    omissions: Vec<Omission>,
    len: u32,
    exif_position: Option<u32>,
    gps_position: Option<u32>,
    interop_position: Option<u32>,
    max_len: u32,
}

/// Plan the block for `data`, a TIFF-structured source.
pub fn exif_block<'a>(data: &'a [u8], options: &BlockOptions) -> Result<ExifBlock<'a>, BlockError> {
    let _ = (data, options);
    Err(BlockError::Unbuildable {
        reason: UnbuildableReason::WalkHalted,
    })
}

impl<'a> ExifBlock<'a> {
    /// The block's length in bytes: even, independent of the base, 0 when the
    /// source has neither IFD.
    pub fn len(&self) -> u32 {
        self.len
    }

    /// Whether the block is empty (the source has neither IFD).
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Block-relative position of the Exif IFD, even; `None` when the source
    /// has none.
    pub fn exif_position(&self) -> Option<u32> {
        self.exif_position
    }

    /// Block-relative position of the GPS IFD, even; `None` when the source
    /// has none.
    pub fn gps_position(&self) -> Option<u32> {
        self.gps_position
    }

    /// The block's byte order: the source's (DEC-014 amendment item 1).
    pub fn byte_order(&self) -> ByteOrder {
        self.order
    }

    /// Every entry left out, in Exif, Interop, GPS order and ascending tag
    /// order.
    pub fn omissions(&self) -> &[Omission] {
        &self.omissions
    }

    /// The block's bytes for `base`.
    pub fn write(&self, base: u32) -> Result<Vec<u8>, BlockError> {
        Err(BlockError::BadBase { base })
    }

    /// The block's bytes for `base`, into `out`, which must be exactly
    /// [`ExifBlock::len`] long.
    pub fn write_into(&self, base: u32, out: &mut [u8]) -> Result<(), BlockError> {
        let _ = out;
        Err(BlockError::BadBase { base })
    }
}

/// The self-check: parse `block` placed at `base` in a scratch image and
/// compare it with `plan`.
fn check_block(plan: &ExifBlock<'_>, base: u32, block: &[u8]) -> Result<(), BlockCheck> {
    let _ = (plan, base, block);
    Err(BlockCheck::Parse)
}

/// The test-only fault hook (SPEC-025 AC6): flips one byte of a block after its
/// layout and before its self-check, to prove the check is wired into `write`.
#[cfg(test)]
mod fault {
    use std::cell::Cell;

    std::thread_local! {
        /// Index of the byte to flip in the next `write`/`write_into`, once.
        pub(super) static FLIP: Cell<Option<usize>> = const { Cell::new(None) };
    }
}

#[cfg(test)]
#[path = "block_tests.rs"]
mod tests;
