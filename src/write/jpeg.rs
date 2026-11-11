//! The JPEG planner (SPEC-020 rules 2 to 8, DEC-013 parts 3 to 6).
//!
//! [`plan`] reads the file with the same reader the audit uses and turns the
//! policy into an [`EditPlan`]: whole-segment removals, byte-range removals for
//! embedded images and trailers, and same-length patches of the Multi-Picture
//! index. It returns a typed refusal whenever it cannot do exactly that.
//!
//! Order of refusals, pinned (the fuzz harness predicts the same order): a
//! walk that cannot be trusted (`Unstrippable`), an Exif entry edit the writer
//! cannot make (`Uneditable`), an unindexed Multi-Picture segment that would
//! be left wrong (`MpfNotIndexed`), a removal that cuts kept bytes
//! (`Overlap`), then a Multi-Picture field that cannot be computed.

use core::ops::Range;

use crate::audit::{audit, Category};
use crate::error::Error;
use crate::jpeg::{self, JpegMetadata, Region, SegmentKind};
use crate::policy::StripSpec;
use crate::tiff::{Container, Ifd};

use super::exif::{exif9, mode, plan_entries, ExifEdits, Mode};
use super::plan::{merge_ranges, removed_in};
use super::{
    Check, EditPlan, EntryRecord, PatchKind, Removal, RemovalKind, ScrubReason, StripError,
    UnstrippableReason,
};

/// A planned strip: the plan, the removal records (input order), the patched
/// ranges as `(input offset, length, kind)`, the zeroed ranges as `(input
/// offset, length, reason)` and the deleted entries.
pub(super) struct Planned {
    pub plan: EditPlan,
    pub removals: Vec<Removal>,
    pub patches: Vec<(usize, usize, PatchKind)>,
    pub scrubs: Vec<(usize, usize, ScrubReason)>,
    pub entries: Vec<EntryRecord>,
}

impl Planned {
    /// A plan with no edit and no record.
    pub(super) fn empty() -> Planned {
        Planned {
            plan: EditPlan::new(),
            removals: Vec::new(),
            patches: Vec::new(),
            scrubs: Vec::new(),
            entries: Vec::new(),
        }
    }
}

/// One thing the plan removes.
struct Target {
    range: Range<usize>,
    category: Category,
    kind: RemovalKind,
}

/// Plan the strip of the JPEG `data` for the categories `spec`.
pub(super) fn plan(data: &[u8], spec: StripSpec) -> Result<Planned, StripError> {
    let meta = jpeg::read(data).map_err(|_| StripError::NotJpeg)?;
    refuse_untrusted_walk(&meta)?;

    let mut targets: Vec<Target> = Vec::new();
    segment_targets(&meta, spec, &mut targets)?;
    let exif_edits = exif_targets(data, &meta, spec, &mut targets)?;
    region_targets(&meta, spec, &mut targets)?;
    targets.sort_by_key(|t| (t.range.start, t.range.end));

    let removals_merged = merge_ranges(targets.iter().map(|t| t.range.clone()).collect());
    mpf_not_indexed(data, &meta, spec, &targets, &removals_merged)?;
    overlap(data, &meta, &targets, &removals_merged)?;

    let mut plan = EditPlan::new();
    for t in &targets {
        plan.remove(t.range.clone());
    }
    let mut patches = mpf_patches(data, &meta, spec, &removals_merged, &mut plan)?;
    let mut scrubs = Vec::new();
    for (offset, bytes, kind, reason) in &exif_edits.patches {
        plan.patch(*offset, bytes);
        patches.push((*offset, bytes.len(), *kind));
        if let Some(reason) = reason {
            scrubs.push((*offset, bytes.len(), *reason));
        }
    }
    let removals = targets
        .iter()
        .map(|t| Removal {
            category: t.category,
            kind: t.kind,
            offset: t.range.start,
            len: t.range.end.saturating_sub(t.range.start),
        })
        .collect();
    Ok(Planned {
        plan,
        removals,
        patches,
        scrubs,
        entries: exif_edits.entries,
    })
}

/// Rule 2: a walk halted by a cap, or any bad segment length, is not cut.
fn refuse_untrusted_walk(meta: &JpegMetadata<'_>) -> Result<(), StripError> {
    for issue in meta.jpeg_issues() {
        let reason = match issue.error() {
            Error::BadSegmentLength { .. } => UnstrippableReason::BadSegmentLength,
            Error::TooManySegments { .. } => UnstrippableReason::TooManySegments,
            Error::TooManyIssues { .. } => UnstrippableReason::TooManyIssues,
            _ => continue,
        };
        return Err(StripError::Unstrippable {
            reason,
            offset: issue.offset(),
        });
    }
    Ok(())
}

/// Rule 3: XMP, extended XMP, ICC, IPTC, JUMBF, comments and unrecognised APPn
/// segments, whole.
fn segment_targets(
    meta: &JpegMetadata<'_>,
    spec: StripSpec,
    out: &mut Vec<Target>,
) -> Result<(), StripError> {
    // The segments the audit reports as unrecognised APPn (not `Adobe`, not
    // the parsed MPF index).
    let other_app: Vec<usize> = meta
        .regions()
        .iter()
        .filter_map(|r| match r {
            Region::OtherApp { segment, .. } => Some(*segment),
            _ => None,
        })
        .collect();
    for (i, seg) in meta.jpeg().segments().iter().enumerate() {
        let (category, kind) = match seg.kind() {
            SegmentKind::Xmp => (Category::Xmp, RemovalKind::Xmp),
            SegmentKind::ExtendedXmp => (Category::Xmp, RemovalKind::ExtendedXmp),
            SegmentKind::Icc => (Category::Icc, RemovalKind::Icc),
            SegmentKind::Iptc => (Category::Iptc, RemovalKind::Iptc),
            SegmentKind::Jumbf => (Category::Jumbf, RemovalKind::Jumbf),
            SegmentKind::Com => (Category::FreeText, RemovalKind::Comment),
            SegmentKind::App(_) if other_app.contains(&i) => {
                (Category::OtherSegment, RemovalKind::OtherApp)
            }
            _ => continue,
        };
        if spec.contains(category) {
            out.push(Target {
                range: segment_range(seg, seg.offset())?,
                category,
                kind,
            });
        }
    }
    Ok(())
}

fn segment_range(seg: &jpeg::Segment<'_>, offset: usize) -> Result<Range<usize>, StripError> {
    seg.range().ok_or(StripError::Unstrippable {
        reason: UnstrippableReason::RangeOverflow,
        offset,
    })
}

/// Rule 4 (DEC-013 part 4) and SPEC-021: the Exif segments are removed whole
/// (every Exif category in the policy), left alone (no category an Exif entry
/// can have), or edited entry by entry (anything between; the edits come back
/// as [`ExifEdits`]).
fn exif_targets(
    data: &[u8],
    meta: &JpegMetadata<'_>,
    spec: StripSpec,
    out: &mut Vec<Target>,
) -> Result<ExifEdits, StripError> {
    let segments = meta.jpeg().segments();
    let exif_segments: Vec<usize> = segments
        .iter()
        .enumerate()
        .filter(|(_, s)| s.kind() == SegmentKind::Exif)
        .map(|(i, _)| i)
        .collect();
    if exif_segments.is_empty() {
        return Ok(ExifEdits::default());
    }
    let all9 = exif9();
    let audited = audit(data);
    // The category a removed block is recorded under: the first EXIF9 finding
    // the audit made in it, `Thumbnail` if it made none.
    let block_category = |segment: usize| -> Category {
        let block = meta
            .exif_blocks()
            .iter()
            .position(|b| b.segment() == segment);
        audited
            .findings()
            .iter()
            .find(|f| block.is_some() && f.at.origin().block == block && all9.contains(f.category))
            .map_or(Category::Thumbnail, |f| f.category)
    };

    match mode(spec) {
        Mode::Whole => {
            for &i in &exif_segments {
                let Some(seg) = segments.get(i) else { continue };
                out.push(Target {
                    range: segment_range(seg, seg.offset())?,
                    category: block_category(i),
                    kind: RemovalKind::Exif,
                });
            }
            Ok(ExifEdits::default())
        }
        Mode::Untouched => Ok(ExifEdits::default()),
        Mode::Partial => plan_entries(meta, spec, &audited),
    }
}

/// Rule 5: trailers and embedded images, and the MPF index with the images.
fn region_targets(
    meta: &JpegMetadata<'_>,
    spec: StripSpec,
    out: &mut Vec<Target>,
) -> Result<(), StripError> {
    for region in meta.regions() {
        match *region {
            Region::Trailer { offset, data, .. } if spec.contains(Category::OtherSegment) => {
                out.push(Target {
                    range: offset..offset.saturating_add(data.len()),
                    category: Category::OtherSegment,
                    kind: RemovalKind::Trailer,
                });
            }
            Region::EmbeddedImage { offset, data, .. }
                if spec.contains(Category::EmbeddedImage) =>
            {
                out.push(Target {
                    range: offset..offset.saturating_add(data.len()),
                    category: Category::EmbeddedImage,
                    kind: RemovalKind::EmbeddedImage,
                });
            }
            _ => {}
        }
    }
    if spec.contains(Category::EmbeddedImage) {
        if let Some(index) = meta.mpf() {
            if let Some(seg) = meta.jpeg().segments().get(index.segment()) {
                out.push(Target {
                    range: segment_range(seg, seg.offset())?,
                    category: Category::EmbeddedImage,
                    kind: RemovalKind::MpfIndex,
                });
            }
        }
    }
    Ok(())
}

/// Segments that look like a Multi-Picture index (APP2 `MPF\0`) but are not the
/// one the audit indexed.
fn unindexed_mpf(meta: &JpegMetadata<'_>) -> Vec<(usize, usize, usize)> {
    let index = meta.mpf().map(|m| m.segment());
    meta.jpeg()
        .segments()
        .iter()
        .enumerate()
        .filter(|(i, s)| {
            s.marker() == 0xE2 && s.payload().starts_with(b"MPF\0") && Some(*i) != index
        })
        .filter_map(|(i, s)| Some((i, s.offset(), s.range()?.end)))
        .collect()
}

/// Rule 6, second half (and its one extension, see below): an `MPF\0` segment
/// the audit did not index, when a removal would make its offsets wrong.
///
/// A segment that is itself removed cannot be left wrong, so it is exempt.
/// Extension: when the real index is removed, the first remaining `MPF\0`
/// segment after it becomes the index the next reader finds, so a kept one
/// after the removed index is refused too.
///
/// Second extension: an index entry the audit could not locate as an image
/// (not `FF D8`, outside the input) is never patched, so it must keep meaning
/// what it meant. If it points inside the input and a removal lies between the
/// index header and the byte it points at (inclusive), the output would read
/// it as something else (a stale offset can land on another image), and the
/// index is reported as `MpfNotIndexed` at the index segment.
fn mpf_not_indexed(
    data: &[u8],
    meta: &JpegMetadata<'_>,
    spec: StripSpec,
    targets: &[Target],
    removals: &[Range<usize>],
) -> Result<(), StripError> {
    if let (Some(index), false) = (meta.mpf(), spec.contains(Category::EmbeddedImage)) {
        for entry in index.entries() {
            if entry.index() == 0 || entry.region().is_some() {
                continue;
            }
            let Some(pointed) = usize::try_from(entry.offset())
                .ok()
                .and_then(|o| index.header_start().checked_add(o))
            else {
                continue;
            };
            let moved = pointed < data.len()
                && removals
                    .iter()
                    .any(|r| r.start <= pointed && r.end > index.header_start());
            if moved {
                let offset = meta
                    .jpeg()
                    .segments()
                    .get(index.segment())
                    .map_or(0, |s| s.offset());
                return Err(StripError::MpfNotIndexed { offset });
            }
        }
    }
    let index_removed = meta.mpf().is_some_and(|m| {
        meta.jpeg()
            .segments()
            .get(m.segment())
            .is_some_and(|s| targets.iter().any(|t| t.range.start == s.offset()))
    });
    for (i, offset, end) in unindexed_mpf(meta) {
        let removed_itself = targets.iter().any(|t| t.range.start == offset);
        if removed_itself {
            continue;
        }
        let later_removal = removals.iter().any(|r| r.end > end);
        let after_removed_index = index_removed && meta.mpf().is_some_and(|m| m.segment() < i);
        if later_removal || after_removed_index {
            return Err(StripError::MpfNotIndexed { offset });
        }
    }
    Ok(())
}

/// Rule 7: no removed range may intersect the scan region, a kept segment, a
/// kept embedded image or a kept trailer piece. Reports the first input offset
/// that is both removed and kept.
fn overlap(
    data: &[u8],
    meta: &JpegMetadata<'_>,
    targets: &[Target],
    removals: &[Range<usize>],
) -> Result<(), StripError> {
    let removed = |r: &Range<usize>| targets.iter().any(|t| t.range == *r);
    // The SOI marker is never removed.
    let mut kept: Vec<Range<usize>> = vec![Range { start: 0, end: 2 }];
    let segments = meta.jpeg().segments();
    if let Some(sos) = segments.iter().find(|s| s.kind() == SegmentKind::Sos) {
        let end = meta
            .jpeg()
            .eoi_offset()
            .map_or(data.len(), |e| e.saturating_add(2));
        kept.push(sos.offset()..end);
    }
    for seg in segments {
        if let Some(r) = seg.range() {
            if !removed(&r) {
                kept.push(r);
            }
        }
    }
    for region in meta.regions() {
        let r = match *region {
            Region::EmbeddedImage { offset, data, .. } | Region::Trailer { offset, data, .. } => {
                offset..offset.saturating_add(data.len())
            }
            _ => continue,
        };
        if !removed(&r) {
            kept.push(r);
        }
    }
    let kept = merge_ranges(kept);
    // Two sorted, disjoint lists: the first intersection found is the first in
    // input order.
    let (mut a, mut b) = (removals.iter().peekable(), kept.iter().peekable());
    while let (Some(r), Some(k)) = (a.peek(), b.peek()) {
        let start = r.start.max(k.start);
        let end = r.end.min(k.end);
        if start < end {
            return Err(StripError::Overlap { offset: start });
        }
        if r.end <= k.end {
            a.next();
        } else {
            b.next();
        }
    }
    Ok(())
}

/// Rule 6, first half: when the images stay, patch the index so it still points
/// where it did. Returns the patched fields as `(input offset, length, kind)`.
fn mpf_patches(
    data: &[u8],
    meta: &JpegMetadata<'_>,
    spec: StripSpec,
    removals: &[Range<usize>],
    plan: &mut EditPlan,
) -> Result<Vec<(usize, usize, PatchKind)>, StripError> {
    let mut patched = Vec::new();
    let Some(index) = meta.mpf() else {
        return Ok(patched);
    };
    if spec.contains(Category::EmbeddedImage) || removals.is_empty() {
        return Ok(patched);
    }
    let stale = StripError::VerifyFailed {
        check: Check::MpfFields,
    };
    let primary_end = meta
        .jpeg()
        .eoi_offset()
        .map_or(data.len(), |e| e.saturating_add(2));
    // Where a field may NOT be patched: the index's own TIFF header and IFD0
    // table (a field that lands on the MPEntry tag's count would rewrite the
    // index), and every located image (a patch would change a kept image). A
    // hostile index whose entries lie on its own structure is refused here,
    // typed, rather than by the self-check afterwards.
    let seg = meta.jpeg().segments().get(index.segment());
    let seg_at = seg.map_or(0, |s| s.offset());
    let mut forbidden: Vec<Range<usize>> = Vec::new();
    let base = index.header_start();
    forbidden.push(base..base.saturating_add(8));
    if let Some(tiff) = seg.and_then(|s| s.payload().get(4..)) {
        if let Some(table) = Container::parse(tiff)
            .ok()
            .and_then(|c| c.ifd0().and_then(Ifd::table_range))
        {
            forbidden.push(base.saturating_add(table.start)..base.saturating_add(table.end));
        }
    }
    for r in meta.regions() {
        if let Region::EmbeddedImage { offset, data, .. } = *r {
            forbidden.push(offset..offset.saturating_add(data.len()));
        }
    }
    let mut put = |field: usize, old: u32, removed: usize| -> Result<(), StripError> {
        let removed = u32::try_from(removed).map_err(|_| stale.clone())?;
        let new = old.checked_sub(removed).ok_or_else(|| stale.clone())?;
        if new != old {
            let field_end = field.saturating_add(4);
            if forbidden
                .iter()
                .any(|r| field < r.end && r.start < field_end)
            {
                return Err(StripError::MpfNotIndexed { offset: seg_at });
            }
            let bytes = if index.big_endian() {
                new.to_be_bytes()
            } else {
                new.to_le_bytes()
            };
            plan.patch(field, &bytes);
            patched.push((field, bytes.len(), PatchKind::MpfField));
        }
        Ok(())
    };
    for entry in index.entries() {
        if entry.index() == 0 {
            // The primary's size: everything removed inside the primary image.
            put(
                entry.size_field(),
                entry.size(),
                removed_in(removals, 0..primary_end),
            )?;
        } else if let Some(Region::EmbeddedImage { offset, .. }) =
            entry.region().and_then(|r| meta.regions().get(r))
        {
            // A secondary's offset: everything removed between the index
            // header and the image.
            put(
                entry.offset_field(),
                entry.offset(),
                removed_in(removals, index.header_start()..*offset),
            )?;
        }
    }
    Ok(patched)
}
