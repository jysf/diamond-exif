//! The writer foundation and the JPEG strip (SPEC-020, DEC-013).
//!
//! [`strip`] is a pure function from input bytes and a [`StripPolicy`] to new
//! bytes or a typed refusal. It never returns a file that still holds a finding
//! of a category the policy removes, and it never returns a file in which
//! anything but the policy's targets changed:
//!
//! * the only edits are whole-range removals and same-length patches
//!   ([`EditPlan`]); the output is never longer than the input;
//! * the unit of removal is the JPEG segment; the Exif segment is all or
//!   nothing in this stage (a policy that needs to edit inside it gets
//!   [`StripError::NeedsEntryEdit`]);
//! * a Multi-Picture index and its images go together, or stay together with
//!   the index patched;
//! * no byte is both removed and kept ([`StripError::Overlap`]);
//! * before any byte is returned the output is audited again and compared with
//!   the input's audit ([`StripError::VerifyFailed`]).
//!
//! The library does no I/O: a caller decides what to do with the bytes. The
//! input slice is never modified.
//!
//! Provenance (DEC-003, class 1): net-new code written from DEC-013 over this
//! crate's own reader and audit. No ExifTool source is read.

mod jpeg;
mod plan;
mod verify;

use core::fmt;

use crate::audit::{sniff, Category, ContainerKind, Sniffed};
use crate::policy::{StripPolicy, StripSpec};

pub use plan::{Edit, EditPlan, PlanError};

/// What kind of bytes a [`Removal`] deleted. The audit's region kinds, plus the
/// two things a strip removes that the audit does not report as a region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RemovalKind {
    /// An Exif APP1 segment (the whole segment, DEC-013 part 4).
    Exif,
    /// The Multi-Picture index segment (APP2 `MPF\0`).
    MpfIndex,
    /// An XMP segment.
    Xmp,
    /// An extended-XMP segment.
    ExtendedXmp,
    /// A Photoshop APP13 segment.
    Iptc,
    /// An ICC profile chunk segment.
    Icc,
    /// A JUMBF segment.
    Jumbf,
    /// A Multi-Picture image other than the primary.
    EmbeddedImage,
    /// A `COM` segment.
    Comment,
    /// An unrecognised APPn segment.
    OtherApp,
    /// Bytes after the `EOI` that no embedded image accounts for.
    Trailer,
}

/// One removed region, in **input** coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Removal {
    /// The category that caused the removal (DEC-008). The Exif segments are
    /// recorded under the first Exif category the audit found in that block
    /// (`Thumbnail` if it found none); the MPF index under `EmbeddedImage`.
    pub category: Category,
    /// What was removed.
    pub kind: RemovalKind,
    /// Input offset of the first removed byte.
    pub offset: usize,
    /// Number of bytes removed.
    pub len: usize,
}

/// One patched field, in **output** coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct PatchRecord {
    /// Output offset of the first patched byte.
    pub offset: usize,
    /// Number of bytes patched.
    pub len: usize,
}

/// The result of a successful strip.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Stripped {
    bytes: Vec<u8>,
    removals: Vec<Removal>,
    patches: Vec<PatchRecord>,
}

impl Stripped {
    /// The new file.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The new file, by value.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// One record per removed region, in input order.
    pub fn removals(&self) -> &[Removal] {
        &self.removals
    }

    /// The patched fields (Multi-Picture offsets and sizes), in output order.
    pub fn patches(&self) -> &[PatchRecord] {
        &self.patches
    }
}

/// Why the writer returned no bytes (DEC-013 part 2).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum StripError {
    /// This writer does not handle that container.
    Unsupported {
        /// What the input was taken for.
        kind: ContainerKind,
    },
    /// Fewer than two bytes, or no `FF D8` (and no other container's magic).
    NotJpeg,
    /// The structure is not trustworthy enough to cut.
    Unstrippable {
        /// Why.
        reason: UnstrippableReason,
        /// The input offset of the first such fault.
        offset: usize,
    },
    /// The policy needs entry-level editing inside an Exif segment, which this
    /// writer cannot do yet.
    NeedsEntryEdit {
        /// The categories that need the edit.
        categories: StripSpec,
    },
    /// A removal would cut bytes that must be kept.
    Overlap {
        /// The first input offset that is both removed and kept.
        offset: usize,
    },
    /// An `MPF\0` segment the audit did not index would be left wrong.
    MpfNotIndexed {
        /// Input offset of that segment.
        offset: usize,
    },
    /// The output failed its own audit: the plan and the self-check disagree.
    VerifyFailed {
        /// The first check that failed.
        check: Check,
    },
}

/// Why a JPEG is not strippable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum UnstrippableReason {
    /// A segment has a length word below 2.
    BadSegmentLength,
    /// The walk stopped at the segment cap.
    TooManySegments,
    /// The walk stopped at the issue cap.
    TooManyIssues,
    /// A segment range overflowed `usize` (not reachable from a real slice).
    RangeOverflow,
}

/// The self-check of DEC-013 part 7.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Check {
    /// (a) The output still has a finding in a category the policy removes.
    Preview,
    /// (b) The output lost or gained a finding in a category the policy keeps.
    Kept,
    /// (c) The scan region is not byte-identical.
    Scan,
    /// (d) The output is not a JPEG that walks, or has a new kind of issue.
    Walk,
    /// (e) Stripping the output again would change it.
    Idempotent,
    /// A Multi-Picture field is stale or cannot be computed.
    MpfFields,
}

impl fmt::Display for StripError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StripError::Unsupported { kind } => {
                write!(f, "the writer does not handle this container ({kind:?})")
            }
            StripError::NotJpeg => write!(f, "not a JPEG"),
            StripError::Unstrippable { reason, offset } => {
                write!(
                    f,
                    "the JPEG cannot be cut safely ({reason:?} at offset {offset})"
                )
            }
            StripError::NeedsEntryEdit { categories } => write!(
                f,
                "the policy needs an edit inside the Exif segment for {} categories",
                categories.len()
            ),
            StripError::Overlap { offset } => {
                write!(f, "a removal would cut kept bytes (offset {offset})")
            }
            StripError::MpfNotIndexed { offset } => write!(
                f,
                "a Multi-Picture segment at offset {offset} is not indexed and would be left wrong"
            ),
            StripError::VerifyFailed { check } => {
                write!(f, "the output failed its own audit ({check:?})")
            }
        }
    }
}

impl std::error::Error for StripError {}

/// Strip `data` according to `policy`. See the module documentation.
pub fn strip(data: &[u8], policy: StripPolicy) -> Result<Stripped, StripError> {
    match sniff(data) {
        Sniffed::Jpeg => {}
        Sniffed::Tiff => {
            return Err(StripError::Unsupported {
                kind: ContainerKind::Tiff,
            })
        }
        Sniffed::Empty | Sniffed::Unknown => return Err(StripError::NotJpeg),
        other => {
            return Err(StripError::Unsupported {
                kind: ContainerKind::Unsupported(other),
            })
        }
    }
    let spec = policy.spec();
    let planned = jpeg::plan(data, spec)?;
    let bytes = planned
        .plan
        .apply(data)
        .map_err(|_| StripError::VerifyFailed {
            check: Check::MpfFields,
        })?;
    verify::verify(data, &bytes, spec).map_err(|check| StripError::VerifyFailed { check })?;
    let patches = planned
        .patches
        .iter()
        .filter_map(|&(offset, len)| {
            let at = planned.plan.output_offset(data.len(), offset)?;
            Some(PatchRecord { offset: at, len })
        })
        .collect();
    Ok(Stripped {
        bytes,
        removals: planned.removals,
        patches,
    })
}

/// [`strip`], returning only the new bytes.
pub fn sanitize(data: &[u8], policy: StripPolicy) -> Result<Vec<u8>, StripError> {
    strip(data, policy).map(Stripped::into_bytes)
}
