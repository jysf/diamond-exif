//! The writer foundation and the JPEG strip (SPEC-020, DEC-013).
//!
//! [`strip`] is a pure function from input bytes and a [`StripPolicy`] to new
//! bytes or a typed refusal. It never returns a file that still holds a finding
//! of a category the policy removes, and it never returns a file in which
//! anything but the policy's targets changed:
//!
//! * the only edits are whole-range removals and same-length patches
//!   ([`EditPlan`]); the output is never longer than the input;
//! * the unit of removal is the JPEG segment; inside an Exif segment a policy
//!   that names some of its categories deletes the matching IFD entries in
//!   place, never growing or moving anything (SPEC-021, DEC-013 amendment B);
//!   when it cannot do that exactly it returns [`StripError::Uneditable`];
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

mod exif;
mod jpeg;
mod plan;
mod verify;

use core::fmt;

use crate::audit::{sniff, Category, ContainerKind, Sniffed};
use crate::policy::{StripPolicy, StripSpec};
use crate::tiff::IfdKind;

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

/// What a patch is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PatchKind {
    /// A Multi-Picture index offset or size field (DEC-013 part 5).
    MpfField,
    /// An IFD table rewritten after entries were deleted from it (or after its
    /// next-IFD word was cut): count word, surviving entries, next word, and
    /// the freed tail zeroed.
    IfdTable,
    /// Bytes overwritten with zeros: a deleted entry's out-of-line value, a
    /// removed IFD's table, a removed thumbnail.
    Scrub,
}

/// One patched range, in **output** coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct PatchRecord {
    /// Output offset of the first patched byte.
    pub offset: usize,
    /// Number of bytes patched.
    pub len: usize,
    /// What the patch is for.
    pub kind: PatchKind,
}

/// One deleted IFD entry, in **input** coordinates (SPEC-021): a policy target,
/// a GPS pointer entry, or an entry of a removed GPS IFD or IFD1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct EntryRecord {
    /// The Exif block (index into the JPEG's Exif blocks).
    pub block: usize,
    /// How the IFD that held the entry was reached.
    pub ifd: IfdKind,
    /// The IFD's index in that block's walk.
    pub ifd_index: usize,
    /// The tag number.
    pub tag: u16,
    /// The declared TIFF field type.
    pub field_type: u16,
    /// The declared value count.
    pub count: u32,
    /// Input offset of the entry's 12 bytes.
    pub offset: usize,
}

/// Why a range was zeroed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScrubReason {
    /// The out-of-line value of a deleted entry.
    EntryValue,
    /// The table of a removed IFD (GPS or IFD1).
    IfdTable,
    /// The JPEG bytes of a removed thumbnail.
    Thumbnail,
}

/// One zeroed range, in **output** coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ScrubRecord {
    /// Output offset of the first zeroed byte.
    pub offset: usize,
    /// Number of bytes zeroed.
    pub len: usize,
    /// Why.
    pub reason: ScrubReason,
}

/// The result of a successful strip.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Stripped {
    bytes: Vec<u8>,
    removals: Vec<Removal>,
    patches: Vec<PatchRecord>,
    entries: Vec<EntryRecord>,
    scrubs: Vec<ScrubRecord>,
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

    /// Every patch (Multi-Picture fields, rewritten IFD tables, scrubbed
    /// ranges) with its kind, in output order.
    pub fn patches(&self) -> &[PatchRecord] {
        &self.patches
    }

    /// One record per deleted IFD entry (policy targets, GPS pointer entries,
    /// and every entry of a removed child IFD), in input order.
    pub fn entries(&self) -> &[EntryRecord] {
        &self.entries
    }

    /// The zeroed ranges, in output order (also listed in [`Stripped::patches`]
    /// with kind [`PatchKind::Scrub`]).
    pub fn scrubs(&self) -> &[ScrubRecord] {
        &self.scrubs
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
    /// An Exif entry edit cannot be made exactly (DEC-013 amendment B.4).
    Uneditable {
        /// Why.
        reason: UneditableReason,
        /// The input offset of the fault (a block's segment, or the first
        /// byte in conflict).
        offset: usize,
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

/// Why an Exif entry edit is refused (DEC-013 amendment B.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum UneditableReason {
    /// The block's walk reported an issue, or the block could not be read, or
    /// the bytes at a patch site are not the entry the walk read.
    BlockNotClean,
    /// More Exif segments than the audit parses (16).
    TooManyExifSegments,
    /// The audit's inventory hit its cap.
    InventoryTruncated,
    /// The Exif segment lies after the first scan.
    AfterScan,
    /// A zeroed range touches bytes that must stay (a kept value, a kept IFD
    /// table, the TIFF header, a kept thumbnail), or two IFD tables overlap.
    SharedBytes,
    /// The thumbnail's bytes cannot be located (strips form, missing length).
    ThumbnailNotLocated,
    /// IFD1 has a following IFD, so cutting it would change the chain.
    ChainAfterThumbnail,
    /// A removed child IFD is not pointed to by exactly one entry of its
    /// parent.
    PointerNotInParent,
    /// An IFD would be left with no entry and the writer does not allow it.
    EmptiedIfd,
    /// A removed IFD has IFDs of its own (a pointer tag inside it).
    NestedIfd,
    /// A deleted entry's value cannot be sized (an unknown field type), so it
    /// cannot be zeroed.
    PayloadUnreadable,
    /// A pointer tag (`SubIFDs`, Exif, GPS, Interop) the walk did not follow:
    /// a second entry with the tag, or one in an IFD kind that does not follow
    /// it. What it points to was never audited.
    HiddenPointer,
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
    /// (g) The output's tag inventory is not the input's minus the deleted
    /// entries (SPEC-021).
    Entries,
    /// (h) A zeroed range does not read all zero in the output (SPEC-021).
    Scrubbed,
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
            StripError::Uneditable { reason, offset } => write!(
                f,
                "an Exif entry edit cannot be made exactly ({reason:?} at offset {offset})"
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
    finish(data, spec, planned)
}

/// Apply a plan, run the self-check (always, an empty plan included: the
/// output is then the input, and it must still satisfy the policy) and build
/// the result.
fn finish(data: &[u8], spec: StripSpec, planned: jpeg::Planned) -> Result<Stripped, StripError> {
    let bytes = planned
        .plan
        .apply(data)
        .map_err(|_| StripError::VerifyFailed {
            check: Check::MpfFields,
        })?;
    // Output coordinates of the patched and zeroed ranges.
    let at = |offset: usize| planned.plan.output_offset(data.len(), offset);
    let mut patches: Vec<PatchRecord> = planned
        .patches
        .iter()
        .filter_map(|&(offset, len, kind)| {
            Some(PatchRecord {
                offset: at(offset)?,
                len,
                kind,
            })
        })
        .collect();
    patches.sort_by_key(|p| p.offset);
    let mut scrubs: Vec<ScrubRecord> = planned
        .scrubs
        .iter()
        .filter_map(|&(offset, len, reason)| {
            Some(ScrubRecord {
                offset: at(offset)?,
                len,
                reason,
            })
        })
        .collect();
    scrubs.sort_by_key(|s| s.offset);
    let zeroed: Vec<(usize, usize)> = scrubs.iter().map(|s| (s.offset, s.len)).collect();
    verify::verify(data, &bytes, spec, &zeroed)
        .map_err(|check| StripError::VerifyFailed { check })?;
    Ok(Stripped {
        bytes,
        removals: planned.removals,
        patches,
        entries: planned.entries,
        scrubs,
    })
}

/// [`strip`], returning only the new bytes.
pub fn sanitize(data: &[u8], policy: StripPolicy) -> Result<Vec<u8>, StripError> {
    strip(data, policy).map(Stripped::into_bytes)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::arithmetic_side_effects
)]
mod tests {
    use super::*;

    fn seg(marker: u8, payload: &[u8]) -> Vec<u8> {
        let mut v = vec![0xFF, marker];
        v.extend_from_slice(&u16::try_from(payload.len() + 2).unwrap().to_be_bytes());
        v.extend_from_slice(payload);
        v
    }

    /// v06: the self-check is not skipped for an empty plan. DEC-013 part 7 and
    /// rule 9 say it runs before any byte is returned, and rule 8 only says the
    /// empty plan returns the input; so an empty plan for an input that still
    /// holds a finding of a removed category is `VerifyFailed {Preview}`, and
    /// no bytes.
    #[test]
    fn v06_an_empty_plan_is_still_checked() {
        let mut d = vec![0xFF, 0xD8];
        d.extend_from_slice(&seg(0xE1, b"http://ns.adobe.com/xap/1.0/\0<x/>"));
        d.extend_from_slice(&seg(0xDA, &[1, 1, 0, 0, 0x3F, 0]));
        d.extend_from_slice(&[1, 2, 0xFF, 0xD9]);
        let spec =
            StripPolicy::Custom(StripSpec::none().include(crate::audit::Category::Xmp)).spec();
        // A planner that missed the XMP segment: an empty plan.
        let planned = jpeg::Planned::empty();
        assert_eq!(
            finish(&d, spec, planned).unwrap_err(),
            StripError::VerifyFailed {
                check: Check::Preview
            }
        );
        // The real planner removes it.
        assert!(strip(&d, StripPolicy::Custom(spec)).is_ok());
    }
}
