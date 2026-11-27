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
use std::collections::BTreeSet;

use crate::error::Error;
use crate::reader::ByteOrder;
use crate::tiff::types::{type_size, TYPE_IFD, TYPE_LONG};
use crate::tiff::{Container, Entry, Ifd, IfdKind, Link};

use super::exif::{
    EXIFTOOL_BLOB_CARRIER_TAGS, EXIFTOOL_SUBDIRECTORY_TAGS, OFFSET_TAGS, POINTER_TAGS,
};

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

// ─────────────────────────────────────────────────────────────────────────────
// The omission table (DEC-014 amendment items 2 to 4)
// ─────────────────────────────────────────────────────────────────────────────

/// The Exif IFD's pointer to the Interop IFD: the one pointer the block keeps,
/// rewritten to the Interop IFD's final offset.
const TAG_INTEROP_POINTER: u16 = 0xA005;
/// IFD0's pointers to the Exif and GPS IFDs.
const TAG_EXIF_POINTER: u16 = 0x8769;
const TAG_GPS_POINTER: u16 = 0x8825;

/// 0x927C MakerNote and 0xEA1D OffsetSchema (which only describes a
/// MakerNote's offsets): never moved (DEC-014 item 4).
const MAKERNOTE_TAGS: [u16; 2] = [0x927C, 0xEA1D];

/// The four offset tags the ExifTool 13.55 writer sweep refused with
/// `No size tag for ...` that [`OFFSET_TAGS`] (the strip's list) does not hold
/// in its offset column: 0x8781 and 0xA010 (offsets into the Exif and Interop
/// IFDs) and 0xBCC0 and 0xBCC2 (DNG-style offsets). DEC-014's amendment of
/// 2026-11-27, item 3. They are kept here, not added to `OFFSET_TAGS`, because
/// that would change what the strip protects (question
/// `strip-offset-tags-incomplete`).
const MEASURED_EXTRA_OFFSET_TAGS: [u16; 4] = [0x8781, 0xA010, 0xBCC0, 0xBCC2];

/// The scratch image's tail past the block: IFD0 with two pointer entries is
/// 2 + 24 + 4 = 30 bytes; 34 is the bound DEC-015's clarification states.
const SCRATCH_TAIL: u64 = 34;

/// The TIFF header the scratch image starts with.
const HEADER_LEN: u32 = 8;

/// Whether `tag` carries an offset the builder does not rewrite.
fn is_offset_tag(tag: u16) -> bool {
    OFFSET_TAGS.iter().any(|(offset, _)| *offset == tag)
        || MEASURED_EXTRA_OFFSET_TAGS.contains(&tag)
        || POINTER_TAGS.contains(&tag)
        || EXIFTOOL_SUBDIRECTORY_TAGS.contains(&tag)
        || EXIFTOOL_BLOB_CARRIER_TAGS.contains(&tag)
}

/// Why `entry` of `ifd` is left out, or `None` when it is kept. Order: a type
/// the walk cannot size always wins (its value's length is unknown, so it
/// cannot be moved, DEC-014 amendment item 2); then the MakerNote pair; then
/// the offset carriers.
fn classify(ifd: BlockIfd, entry: &Entry, interop_present: bool) -> Option<OmitReason> {
    let tag = entry.tag();
    if type_size(entry.field_type()).is_none() {
        return Some(OmitReason::UnknownType);
    }
    if MAKERNOTE_TAGS.contains(&tag) {
        return Some(OmitReason::MakerNote);
    }
    if tag == TAG_INTEROP_POINTER && ifd == BlockIfd::Exif {
        // Kept, and rewritten, when there is an Interop IFD to point at. A
        // pointer that leads nowhere (offset 0) would be a dangling number in
        // the block, so it is left out.
        return if interop_present {
            None
        } else {
            Some(OmitReason::OffsetCarrying)
        };
    }
    if is_offset_tag(tag) || entry.field_type() == TYPE_IFD {
        return Some(OmitReason::OffsetCarrying);
    }
    None
}

// ─────────────────────────────────────────────────────────────────────────────
// The plan
// ─────────────────────────────────────────────────────────────────────────────

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

fn unbuildable(reason: UnbuildableReason) -> BlockError {
    BlockError::Unbuildable { reason }
}

/// `usize` to `u64`, saturating (a length that does not fit is over any
/// budget).
fn wide(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

/// Plan the block for `data`, a TIFF-structured source (DEC-014).
///
/// Refuses, with no bytes, when the source does not parse, the walk was halted
/// by a budget, a pointer the block needs cannot be followed or is duplicated,
/// one of the three IFDs holds a tag twice, a kept value cannot be read, or the
/// block exceeds [`BlockOptions::max_len`]. Never allocates a block-sized
/// buffer: the length is computed from the source's entries first.
pub fn exif_block<'a>(data: &'a [u8], options: &BlockOptions) -> Result<ExifBlock<'a>, BlockError> {
    let container = Container::parse(data).map_err(BlockError::Source)?;
    let order = container.byte_order();
    let ifds = container.ifds();

    // A halt can hide an IFD: refuse (amendment item 4).
    if container.issues().iter().any(|i| {
        matches!(
            i.error(),
            Error::TooManyIfds { .. } | Error::TooManyEntries { .. }
        )
    }) {
        return Err(unbuildable(UnbuildableReason::WalkHalted));
    }

    // Select by parent, never by kind alone: the walk visits SubIFDs before
    // IFD0's Exif pointer, so the first Exif IFD can be a SubIFD's (DEC-005).
    let child = |kind: IfdKind, parent: usize| {
        ifds.iter()
            .position(|i| i.kind() == kind && i.parent() == Some(parent))
    };
    let exif_idx = child(IfdKind::Exif, 0);
    let gps_idx = child(IfdKind::Gps, 0);
    let interop_idx = exif_idx.and_then(|e| child(IfdKind::Interop, e));

    // An issue following a pointer the block needs.
    for issue in container.issues() {
        let Link::Tag(tag) = issue.link() else {
            continue;
        };
        let from_ifd0 =
            issue.from_ifd() == 0 && (tag == TAG_EXIF_POINTER || tag == TAG_GPS_POINTER);
        let from_exif = tag == TAG_INTEROP_POINTER && Some(issue.from_ifd()) == exif_idx;
        if from_ifd0 || from_exif {
            return Err(unbuildable(UnbuildableReason::BrokenPointer { tag }));
        }
    }

    // The walk takes the first of a duplicate pointer, so check for one.
    let twice = |ifd: Option<&Ifd>, tag: u16| {
        ifd.is_some_and(|i| i.entries().iter().filter(|e| e.tag() == tag).count() > 1)
    };
    for (holder, tag) in [
        (ifds.first(), TAG_EXIF_POINTER),
        (ifds.first(), TAG_GPS_POINTER),
        (exif_idx.and_then(|i| ifds.get(i)), TAG_INTEROP_POINTER),
    ] {
        if twice(holder, tag) {
            return Err(unbuildable(UnbuildableReason::DuplicatePointer { tag }));
        }
    }

    let mut tables = Vec::new();
    let mut omissions = Vec::new();
    for (which, idx) in [
        (BlockIfd::Exif, exif_idx),
        (BlockIfd::Interop, interop_idx),
        (BlockIfd::Gps, gps_idx),
    ] {
        let Some(ifd) = idx.and_then(|i| ifds.get(i)) else {
            continue;
        };
        tables.push(plan_table(
            &container,
            which,
            ifd,
            interop_idx.is_some(),
            &mut omissions,
        )?);
    }

    // Layout (amendment item 7), in u64, before anything block-sized exists.
    let mut cursor: u64 = 0;
    let mut table_at = Vec::with_capacity(tables.len());
    for table in &tables {
        table_at.push(cursor);
        let n = wide(table.kept.len());
        cursor = cursor.saturating_add(n.saturating_mul(12).saturating_add(6));
    }
    let mut value_at: Vec<Vec<Option<u64>>> = Vec::with_capacity(tables.len());
    for table in &tables {
        let mut offsets = Vec::with_capacity(table.kept.len());
        for kept in &table.kept {
            let len = if kept.interop_pointer {
                0
            } else {
                wide(kept.payload.len())
            };
            if len > 4 {
                offsets.push(Some(cursor));
                // Each value starts at an even offset and is padded to one.
                cursor = cursor.saturating_add(len).saturating_add(len & 1);
            } else {
                offsets.push(None);
            }
        }
        value_at.push(offsets);
    }
    if cursor > u64::from(options.max_len()) {
        return Err(BlockError::OverBudget {
            needed: cursor,
            budget: options.max_len(),
        });
    }
    let over = |needed: u64| BlockError::OverBudget {
        needed,
        budget: options.max_len(),
    };
    let len = u32::try_from(cursor).map_err(|_| over(cursor))?;
    for ((table, at), offsets) in tables.iter_mut().zip(&table_at).zip(&value_at) {
        table.at = u32::try_from(*at).map_err(|_| over(*at))?;
        for (kept, off) in table.kept.iter_mut().zip(offsets) {
            kept.value_at = match off {
                Some(o) => Some(u32::try_from(*o).map_err(|_| over(*o))?),
                None => None,
            };
        }
    }
    let position = |which: BlockIfd| tables.iter().find(|t| t.ifd == which).map(|t| t.at);
    let exif_position = position(BlockIfd::Exif);
    let gps_position = position(BlockIfd::Gps);
    let interop_position = position(BlockIfd::Interop);

    Ok(ExifBlock {
        order,
        omissions,
        len,
        exif_position,
        gps_position,
        interop_position,
        max_len: options.max_len(),
        tables,
    })
}

/// Plan one IFD of the block: refuse a duplicate tag or an unreadable kept
/// value, classify every entry, keep the rest in ascending tag order.
fn plan_table<'a>(
    container: &Container<'a>,
    which: BlockIfd,
    ifd: &Ifd,
    interop_present: bool,
    omissions: &mut Vec<Omission>,
) -> Result<Table<'a>, BlockError> {
    let mut tags: Vec<u16> = ifd.entries().iter().map(Entry::tag).collect();
    tags.sort_unstable();
    for pair in tags.windows(2) {
        if let [a, b] = pair {
            if a == b {
                return Err(unbuildable(UnbuildableReason::DuplicateTag {
                    ifd: which,
                    tag: *a,
                }));
            }
        }
    }

    let mut sorted: Vec<&Entry> = ifd.entries().iter().collect();
    sorted.sort_by_key(|e| e.tag());
    let mut kept = Vec::with_capacity(sorted.len());
    let mut source = Vec::with_capacity(sorted.len());
    for entry in sorted {
        source.push((entry.tag(), entry.field_type()));
        if let Some(reason) = classify(which, entry, interop_present) {
            omissions.push(Omission {
                ifd: which,
                tag: entry.tag(),
                field_type: entry.field_type(),
                reason,
            });
            continue;
        }
        if entry.tag() == TAG_INTEROP_POINTER && which == BlockIfd::Exif {
            kept.push(Kept {
                tag: entry.tag(),
                field_type: TYPE_LONG,
                count: 1,
                payload: &[],
                interop_pointer: true,
                value_at: None,
            });
            continue;
        }
        let payload = container.payload(entry).map_err(|_| {
            unbuildable(UnbuildableReason::UnreadableValue {
                ifd: which,
                tag: entry.tag(),
            })
        })?;
        kept.push(Kept {
            tag: entry.tag(),
            field_type: entry.field_type(),
            count: entry.count(),
            payload,
            interop_pointer: false,
            value_at: None,
        });
    }
    Ok(Table {
        ifd: which,
        at: 0,
        kept,
        source,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Writing
// ─────────────────────────────────────────────────────────────────────────────

fn enc16(order: ByteOrder, v: u16) -> [u8; 2] {
    match order {
        ByteOrder::Little => v.to_le_bytes(),
        ByteOrder::Big => v.to_be_bytes(),
    }
}

fn enc32(order: ByteOrder, v: u32) -> [u8; 4] {
    match order {
        ByteOrder::Little => v.to_le_bytes(),
        ByteOrder::Big => v.to_be_bytes(),
    }
}

/// Copy `bytes` to `out[at..]`; `None` if it does not fit.
fn put(out: &mut [u8], at: usize, bytes: &[u8]) -> Option<()> {
    let end = at.checked_add(bytes.len())?;
    out.get_mut(at..end)?.copy_from_slice(bytes);
    Some(())
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

    /// The block's bytes for `base`: the table and value bytes with every
    /// offset final for a block starting `base` bytes from the TIFF header.
    ///
    /// `base` must be even, at least 8 for a non-empty block, with
    /// `base + len` in a `u32`; `base + len + 34` (the self-check's scratch
    /// image) must fit the budget. An empty block returns no bytes for any even
    /// base. The returned bytes were parsed back and compared with the source
    /// (DEC-015 clarification) before being returned.
    pub fn write(&self, base: u32) -> Result<Vec<u8>, BlockError> {
        self.check_base(base)?;
        if self.is_empty() {
            return Ok(Vec::new());
        }
        self.check_budget(base)?;
        // The budget was checked first: this is the first block-sized buffer.
        let mut out = vec![0u8; usize::try_from(self.len).unwrap_or(0)];
        self.fill_and_verify(base, &mut out)?;
        Ok(out)
    }

    /// Like [`ExifBlock::write`], into `out`, which must be exactly
    /// [`ExifBlock::len`] bytes. Every byte of `out` is overwritten.
    pub fn write_into(&self, base: u32, out: &mut [u8]) -> Result<(), BlockError> {
        self.check_base(base)?;
        if usize::try_from(self.len).ok() != Some(out.len()) {
            return Err(BlockError::BufferLength {
                expected: self.len,
                got: out.len(),
            });
        }
        if self.is_empty() {
            return Ok(());
        }
        self.check_budget(base)?;
        self.fill_and_verify(base, out)
    }

    /// DEC-015 item 2: even, and `base + len` in a `u32`. A non-empty block also
    /// cannot start inside the 8-byte TIFF header.
    fn check_base(&self, base: u32) -> Result<(), BlockError> {
        if base & 1 == 1 {
            return Err(BlockError::BadBase { base });
        }
        if self.is_empty() {
            return Ok(());
        }
        let end = u64::from(base).saturating_add(u64::from(self.len));
        if base < HEADER_LEN || end > u64::from(u32::MAX) {
            return Err(BlockError::BadBase { base });
        }
        Ok(())
    }

    /// DEC-015 clarification: the self-check's scratch image,
    /// `base + len + 34` bytes, counts against the budget.
    fn check_budget(&self, base: u32) -> Result<(), BlockError> {
        let needed = u64::from(base)
            .saturating_add(u64::from(self.len))
            .saturating_add(SCRATCH_TAIL);
        if needed > u64::from(self.max_len) {
            return Err(BlockError::OverBudget {
                needed,
                budget: self.max_len,
            });
        }
        Ok(())
    }

    fn fill_and_verify(&self, base: u32, out: &mut [u8]) -> Result<(), BlockError> {
        self.encode(base, out).ok_or(BlockError::VerifyFailed {
            check: BlockCheck::Length,
        })?;
        #[cfg(test)]
        fault::apply(out);
        check_block(self, base, out).map_err(|check| BlockError::VerifyFailed { check })
    }

    /// Write every table and value into `out` (exactly `len` bytes) for `base`.
    /// `None` only if the plan's own layout does not fit `out`.
    fn encode(&self, base: u32, out: &mut [u8]) -> Option<()> {
        out.fill(0);
        let order = self.order;
        for table in &self.tables {
            let at = usize::try_from(table.at).ok()?;
            put(
                out,
                at,
                &enc16(order, u16::try_from(table.kept.len()).ok()?),
            )?;
            let mut entry_at = at.checked_add(2)?;
            for kept in &table.kept {
                put(out, entry_at, &enc16(order, kept.tag))?;
                put(
                    out,
                    entry_at.checked_add(2)?,
                    &enc16(order, kept.field_type),
                )?;
                put(out, entry_at.checked_add(4)?, &enc32(order, kept.count))?;
                let field_at = entry_at.checked_add(8)?;
                if kept.interop_pointer {
                    let target = base.checked_add(self.interop_position?)?;
                    put(out, field_at, &enc32(order, target))?;
                } else if let Some(value_at) = kept.value_at {
                    put(out, usize::try_from(value_at).ok()?, kept.payload)?;
                    put(out, field_at, &enc32(order, base.checked_add(value_at)?))?;
                } else {
                    // Inline: the payload, then zeros (`out` was zeroed).
                    put(out, field_at, kept.payload)?;
                }
                entry_at = entry_at.checked_add(12)?;
            }
            // The table's next-word stays 0 (amendment item 7).
        }
        Some(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The self-check (DEC-013 part 7, DEC-015 clarification)
// ─────────────────────────────────────────────────────────────────────────────

/// Index of the IFD of `kind` whose parent is `parent`.
fn child_of(c: &Container<'_>, kind: IfdKind, parent: usize) -> Option<usize> {
    c.ifds()
        .iter()
        .position(|i| i.kind() == kind && i.parent() == Some(parent))
}

/// The self-check: build the scratch image (an 8-byte header, zeros up to
/// `base`, `block`, then an IFD0 holding 0x8769 and 0x8825 as the placing
/// writer writes them), parse it with [`Container::parse`] and compare it with
/// `plan`. Separate from the writer so a test can feed it a corrupted block.
fn check_block(plan: &ExifBlock<'_>, base: u32, block: &[u8]) -> Result<(), BlockCheck> {
    if usize::try_from(plan.len).ok() != Some(block.len()) {
        return Err(BlockCheck::Length);
    }
    let order = plan.order;
    let base_at = usize::try_from(base).map_err(|_| BlockCheck::Length)?;
    let ifd0_at = base_at.checked_add(block.len()).ok_or(BlockCheck::Length)?;
    let ifd0_word = u32::try_from(ifd0_at).map_err(|_| BlockCheck::Length)?;

    let mut pointers: Vec<(u16, u32)> = Vec::with_capacity(2);
    for (tag, position) in [
        (TAG_EXIF_POINTER, plan.exif_position),
        (TAG_GPS_POINTER, plan.gps_position),
    ] {
        if let Some(position) = position {
            let target = base.checked_add(position).ok_or(BlockCheck::Length)?;
            pointers.push((tag, target));
        }
    }

    let mut image = Vec::with_capacity(ifd0_at.saturating_add(34));
    image.extend_from_slice(match order {
        ByteOrder::Little => b"II\x2A\0",
        ByteOrder::Big => b"MM\0\x2A",
    });
    image.extend_from_slice(&enc32(order, ifd0_word));
    image.resize(base_at, 0);
    image.extend_from_slice(block);
    image.extend_from_slice(&enc16(
        order,
        u16::try_from(pointers.len()).map_err(|_| BlockCheck::Length)?,
    ));
    for (tag, target) in &pointers {
        image.extend_from_slice(&enc16(order, *tag));
        image.extend_from_slice(&enc16(order, TYPE_LONG));
        image.extend_from_slice(&enc32(order, 1));
        image.extend_from_slice(&enc32(order, *target));
    }
    image.extend_from_slice(&enc32(order, 0));

    // Parse: clean, and exactly IFD0 plus the block's IFDs.
    let parsed = Container::parse(&image).map_err(|_| BlockCheck::Parse)?;
    if !parsed.is_clean() {
        return Err(BlockCheck::Parse);
    }
    if parsed.ifds().len() != plan.tables.len().saturating_add(1) {
        return Err(BlockCheck::Positions);
    }

    // Positions: each planned IFD is found, by parent, at its planned offset.
    let exif_idx = child_of(&parsed, IfdKind::Exif, 0);
    let index_of = |which: BlockIfd| match which {
        BlockIfd::Exif => exif_idx,
        BlockIfd::Interop => exif_idx.and_then(|e| child_of(&parsed, IfdKind::Interop, e)),
        BlockIfd::Gps => child_of(&parsed, IfdKind::Gps, 0),
    };
    let mut found = Vec::with_capacity(plan.tables.len());
    for table in &plan.tables {
        let idx = index_of(table.ifd).ok_or(BlockCheck::Positions)?;
        let ifd = parsed.ifds().get(idx).ok_or(BlockCheck::Positions)?;
        let want = base.checked_add(table.at).ok_or(BlockCheck::Positions)?;
        if ifd.offset() != want {
            return Err(BlockCheck::Positions);
        }
        found.push(ifd);
    }

    // Entries: same tag, type, count and payload bytes, next-word 0.
    for (table, ifd) in plan.tables.iter().zip(&found) {
        if ifd.next() != 0 || ifd.entries().len() != table.kept.len() {
            return Err(BlockCheck::Entries);
        }
        for (kept, entry) in table.kept.iter().zip(ifd.entries()) {
            if entry.tag() != kept.tag
                || entry.field_type() != kept.field_type
                || entry.count() != kept.count
            {
                return Err(BlockCheck::Entries);
            }
            let got = parsed.payload(entry).map_err(|_| BlockCheck::Entries)?;
            if kept.interop_pointer {
                let target = base
                    .checked_add(plan.interop_position.ok_or(BlockCheck::Entries)?)
                    .ok_or(BlockCheck::Entries)?;
                if got != enc32(order, target) {
                    return Err(BlockCheck::Entries);
                }
            } else if got != kept.payload {
                return Err(BlockCheck::Entries);
            }
        }
        // Slack: the bytes no entry's value covers must be zero. A mismatch is
        // reported as `Entries`, the entry's own bytes being wrong.
        for (i, kept) in table.kept.iter().enumerate() {
            let len = kept.payload.len();
            let range: Option<(usize, usize)> = match kept.value_at {
                // Out of line: the pad byte after an odd-length value.
                Some(value_at) if len % 2 == 1 => {
                    let at = usize::try_from(value_at)
                        .ok()
                        .and_then(|v| v.checked_add(base_at))
                        .and_then(|v| v.checked_add(len))
                        .ok_or(BlockCheck::Entries)?;
                    Some((at, at.saturating_add(1)))
                }
                Some(_) => None,
                // Inline: the value field's bytes past the payload.
                None if !kept.interop_pointer && len < 4 => {
                    let field = usize::try_from(ifd.offset())
                        .ok()
                        .and_then(|o| o.checked_add(i.checked_mul(12)?))
                        .and_then(|o| o.checked_add(10))
                        .ok_or(BlockCheck::Entries)?;
                    Some((field.saturating_add(len), field.saturating_add(4)))
                }
                None => None,
            };
            if let Some((start, end)) = range {
                let bytes = image.get(start..end).ok_or(BlockCheck::Entries)?;
                if bytes.iter().any(|b| *b != 0) {
                    return Err(BlockCheck::Entries);
                }
            }
        }
    }

    // Omissions: what the source held is exactly what the block holds plus
    // what was reported, with nothing in both.
    let mut accounted = 0usize;
    for (table, ifd) in plan.tables.iter().zip(&found) {
        let in_block: BTreeSet<u16> = ifd.entries().iter().map(Entry::tag).collect();
        let omitted: Vec<&Omission> = plan
            .omissions
            .iter()
            .filter(|o| o.ifd == table.ifd)
            .collect();
        accounted = accounted.saturating_add(omitted.len());
        let reported: BTreeSet<u16> = omitted.iter().map(|o| o.tag).collect();
        let source: BTreeSet<u16> = table.source.iter().map(|(tag, _)| *tag).collect();
        let union: BTreeSet<u16> = in_block.union(&reported).copied().collect();
        if in_block.intersection(&reported).next().is_some()
            || reported.len() != omitted.len()
            || union != source
            || omitted
                .iter()
                .any(|o| !table.source.contains(&(o.tag, o.field_type)))
        {
            return Err(BlockCheck::Omissions);
        }
    }
    if accounted != plan.omissions.len() {
        return Err(BlockCheck::Omissions);
    }

    // Length: every out-of-line value lies inside the block.
    let block_end = ifd0_at;
    for ifd in &found {
        for entry in ifd.entries() {
            if let Some(range) = parsed.payload_range(entry) {
                if range.start < base_at || range.end > block_end {
                    return Err(BlockCheck::Length);
                }
            }
        }
    }
    Ok(())
}

/// The test-only fault hook (SPEC-025 AC6): flips one byte of a block after its
/// layout and before its self-check, to prove the check is wired into `write`.
#[cfg(test)]
mod fault {
    use std::cell::Cell;

    std::thread_local! {
        /// Index of the byte to flip in `write` and `write_into` while set.
        pub(super) static FLIP: Cell<Option<usize>> = const { Cell::new(None) };
    }

    pub(super) fn apply(out: &mut [u8]) {
        if let Some(i) = FLIP.with(Cell::get) {
            if let Some(b) = out.get_mut(i) {
                *b ^= 1;
            }
        }
    }
}

#[cfg(test)]
#[path = "block_tests.rs"]
mod tests;
