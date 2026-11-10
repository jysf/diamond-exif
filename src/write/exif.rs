//! The Exif entry planner (SPEC-021, DEC-013 amendment B): delete IFD entries
//! in place, scrub their values, never grow, never move.
//!
//! [`plan_block`] works on one TIFF blob and the tolerant walk of it, and knows
//! nothing about JPEG: its offsets are blob offsets (the JPEG layer adds the
//! blob's input offset, and SPEC-022 will call it on a bare TIFF). It produces
//! same-length patches only:
//!
//! * for an IFD that loses entries, one patch over its whole table (count word,
//!   surviving entries in order, the next-IFD word, the freed tail zeroed);
//! * zero-byte patches over what a deletion leaves behind: a deleted entry's
//!   out-of-line value, a removed IFD's table, a removed thumbnail.
//!
//! Every range that would be zeroed or rewritten is checked against the set of
//! bytes that must stay (the TIFF header, kept values, kept IFD tables, a kept
//! thumbnail) and refused as [`UneditableReason::SharedBytes`] on contact.
//!
//! [`plan_entries`] is the JPEG side: it decides, from the audit, what each
//! Exif block loses, and turns the blob patches into input coordinates.
//!
//! Provenance (DEC-003, class 1): net-new code written from DEC-013 over this
//! crate's own TIFF walk. No ExifTool source is read.

use core::ops::Range;
use std::collections::{BTreeMap, BTreeSet};

use crate::audit::{FileAudit, ItemKind};
use crate::jpeg::{JpegMetadata, SegmentKind};
use crate::policy::StripSpec;
use crate::reader::{read_u16, read_u32, ByteOrder};
use crate::tiff::{Container, Entry, Ifd, IfdKind};

use super::plan::merge_ranges;
use super::{EntryRecord, PatchKind, ScrubReason, StripError, UneditableReason};

/// The GPS pointer tag, in IFD0 or a SubIFD.
const TAG_GPS_POINTER: u16 = 0x8825;
/// IFD1 `JPEGInterchangeFormat`: the thumbnail's offset.
const TAG_THUMB_OFFSET: u16 = 0x0201;
/// The tags the walk follows as pointers: `SubIFDs`, Exif, GPS, Interop.
const POINTER_TAGS: [u16; 4] = [0x014A, 0x8769, TAG_GPS_POINTER, 0xA005];
/// Every IFD entry is 12 bytes (TIFF 6.0 section 2).
const ENTRY_LEN: usize = 12;
/// The TIFF header: byte-order mark, 42, offset of IFD0.
const HEADER_LEN: usize = 8;

/// What to remove from one block, decided by the caller from the audit.
#[derive(Debug, Clone, Default)]
pub(super) struct Selection {
    /// Entries to delete, as `(ifd index, entry index)` in the walk. Entries of
    /// an IFD that goes whole are ignored (they go with it).
    pub deleted: Vec<(usize, usize)>,
    /// Remove every GPS IFD (and its pointer entry).
    pub gps: bool,
    /// Remove the IFD1 thumbnail (IFD1, its values, its JPEG bytes).
    pub thumbnail: bool,
    /// Where the audit located the thumbnail bytes `(offset, length)`, when
    /// it could. Scrubbed when `thumbnail`, protected otherwise.
    pub thumbnail_range: Option<(u32, u32)>,
}

/// One deleted entry, in blob coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Deleted {
    pub ifd_index: usize,
    pub ifd: IfdKind,
    pub tag: u16,
    pub field_type: u16,
    pub count: u32,
    /// Blob offset of the entry's 12 bytes.
    pub at: usize,
}

/// One patch, in blob coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BlobPatch {
    pub at: usize,
    pub bytes: Vec<u8>,
    pub kind: PatchKind,
    /// Set for [`PatchKind::Scrub`].
    pub reason: Option<ScrubReason>,
}

/// The plan for one block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct BlockPlan {
    /// Sorted by offset, disjoint.
    pub patches: Vec<BlobPatch>,
    /// In blob (so input) order.
    pub deleted: Vec<Deleted>,
}

/// A refusal: why, and the blob offset of the fault.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Refusal {
    pub reason: UneditableReason,
    pub at: usize,
}

fn put16(order: ByteOrder, v: u16) -> [u8; 2] {
    match order {
        ByteOrder::Little => v.to_le_bytes(),
        ByteOrder::Big => v.to_be_bytes(),
    }
}

fn put32(order: ByteOrder, v: u32) -> [u8; 4] {
    match order {
        ByteOrder::Little => v.to_le_bytes(),
        ByteOrder::Big => v.to_be_bytes(),
    }
}

/// Blob offset of an entry's 12 bytes, checked to be the entry the walk read:
/// the tag, type and count in the blob equal the entry's (rule 4).
fn entry_start(blob: &[u8], order: ByteOrder, e: &Entry) -> Option<usize> {
    let start = e.value_at().checked_sub(8)?;
    let tag = read_u16(blob, start, order).ok()?;
    let ty = read_u16(blob, start.checked_add(2)?, order).ok()?;
    let count = read_u32(blob, start.checked_add(4)?, order).ok()?;
    (tag == e.tag() && ty == e.field_type() && count == e.count()).then_some(start)
}

/// Whether the walk follows pointer tag `tag` when an IFD of kind `kind` holds
/// it (the DEC-005 table, typed again here).
fn pointer_followed_from(kind: IfdKind, tag: u16) -> bool {
    match tag {
        0x8769 | TAG_GPS_POINTER => matches!(
            kind,
            IfdKind::Ifd0 | IfdKind::Ifd1 | IfdKind::Chain | IfdKind::Sub
        ),
        0xA005 => kind == IfdKind::Exif,
        _ => true,
    }
}

/// Where an entry's 12 bytes start according to the walk (for a fault report).
fn start_of(e: &Entry) -> usize {
    e.value_at().saturating_sub(8)
}

/// The first offset at which `r` touches one of the sorted, disjoint `set`.
fn first_touch(set: &[Range<usize>], r: &Range<usize>) -> Option<usize> {
    let i = set.partition_point(|k| k.end <= r.start);
    let k = set.get(i)?;
    (k.start < r.end).then(|| k.start.max(r.start))
}

fn bad(at: usize) -> Refusal {
    Refusal {
        reason: UneditableReason::BlockNotClean,
        at,
    }
}

fn refuse(reason: UneditableReason, at: usize) -> Refusal {
    Refusal { reason, at }
}

/// The IFD1 removal, once its preconditions are checked (rule 7).
struct ThumbCut {
    ifd1: usize,
    ifd0: usize,
    range: Range<usize>,
}

/// The blob range `(offset, length)` names, if it lies inside the blob.
fn blob_range(blob: &[u8], range: (u32, u32)) -> Option<Range<usize>> {
    let start = usize::try_from(range.0).ok()?;
    let end = start.checked_add(usize::try_from(range.1).ok()?)?;
    blob.get(start..end)?;
    Some(start..end)
}

/// Plan the edits of one block. See the module documentation.
pub(super) fn plan_block(
    blob: &[u8],
    c: &Container<'_>,
    sel: &Selection,
) -> Result<BlockPlan, Refusal> {
    let order = c.byte_order();
    let ifds = c.ifds();

    // ── pointers the walk did not follow ────────────────────────────────────
    // The walk follows the first entry with a pointer tag, and only from the
    // IFD kinds DEC-005 allows. Any other pointer entry leads to bytes the
    // audit never saw (a second GPS pointer behind a zero, an Exif pointer
    // inside the Exif IFD), where an entry of the policy's categories could be
    // left behind.
    for ifd in ifds {
        for tag in POINTER_TAGS {
            let holders: Vec<&Entry> = ifd.entries().iter().filter(|e| e.tag() == tag).collect();
            let followed = pointer_followed_from(ifd.kind(), tag);
            let hidden = if followed {
                holders.get(1)
            } else {
                holders.first()
            };
            if let Some(e) = hidden {
                return Err(refuse(UneditableReason::HiddenPointer, start_of(e)));
            }
        }
    }

    // ── IFD tables that share bytes ─────────────────────────────────────────
    // Two IFDs whose tables overlap cannot both be read or rewritten as the
    // walk saw them; checked whether or not anything is edited.
    let mut all_tables: Vec<Range<usize>> = Vec::new();
    for ifd in ifds {
        all_tables.push(ifd.table_range().ok_or_else(|| bad(0))?);
    }
    all_tables.sort_by_key(|t| (t.start, t.end));
    for pair in all_tables.windows(2) {
        if let [a, b] = pair {
            if b.start < a.end {
                return Err(refuse(UneditableReason::SharedBytes, b.start));
            }
        }
    }

    // ── which IFDs go whole (rules 2, 6, 7) ────────────────────────────────
    let mut removed: Vec<usize> = Vec::new();
    if sel.gps {
        for (i, ifd) in ifds.iter().enumerate() {
            if ifd.kind() == IfdKind::Gps {
                removed.push(i);
            }
        }
    }
    let mut thumb: Option<ThumbCut> = None;
    if sel.thumbnail {
        if let Some(ifd1) = ifds.iter().position(|i| i.kind() == IfdKind::Ifd1) {
            let ifd = ifds.get(ifd1).ok_or_else(|| bad(0))?;
            let at = usize::try_from(ifd.offset()).unwrap_or(0);
            let range = sel
                .thumbnail_range
                .and_then(|r| blob_range(blob, r))
                .ok_or_else(|| refuse(UneditableReason::ThumbnailNotLocated, at))?;
            if ifd.next() != 0 {
                let next_at = ifd.next_at().ok_or_else(|| bad(at))?;
                return Err(refuse(UneditableReason::ChainAfterThumbnail, next_at));
            }
            let ifd0 = ifds
                .iter()
                .position(|i| i.kind() == IfdKind::Ifd0 && i.next() == ifd.offset())
                .ok_or_else(|| bad(at))?;
            removed.push(ifd1);
            thumb = Some(ThumbCut { ifd1, ifd0, range });
        }
    }
    for &r in &removed {
        if let Some(child) = ifds.iter().find(|i| i.parent() == Some(r)) {
            return Err(refuse(
                UneditableReason::NestedIfd,
                usize::try_from(child.offset()).unwrap_or(0),
            ));
        }
    }

    // ── which entries go (rules 2 and 6) ───────────────────────────────────
    let mut gone: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for &(g, e) in &sel.deleted {
        let entry = ifds
            .get(g)
            .and_then(|i| i.entries().get(e))
            .ok_or_else(|| bad(0))?;
        if !removed.contains(&g) {
            entry_start(blob, order, entry).ok_or_else(|| bad(start_of(entry)))?;
            gone.entry(g).or_default().insert(e);
        }
    }
    let thumb_ifd = thumb.as_ref().map(|t| t.ifd1);
    for &r in removed.iter().filter(|&&r| Some(r) != thumb_ifd) {
        let ifd = ifds.get(r).ok_or_else(|| bad(0))?;
        let at = usize::try_from(ifd.offset()).unwrap_or(0);
        let (p, holder) = ifd
            .parent()
            .and_then(|p| ifds.get(p).map(|i| (p, i)))
            .ok_or_else(|| refuse(UneditableReason::PointerNotInParent, at))?;
        // The entry that names this IFD (a second pointer entry was refused
        // above as a hidden pointer).
        let first = holder
            .entries()
            .iter()
            .position(|e| {
                e.tag() == TAG_GPS_POINTER
                    && read_u32(blob, e.value_at(), order).ok() == Some(ifd.offset())
            })
            .ok_or_else(|| refuse(UneditableReason::PointerNotInParent, at))?;
        let entry = holder.entries().get(first).ok_or_else(|| bad(at))?;
        entry_start(blob, order, entry).ok_or_else(|| bad(start_of(entry)))?;
        gone.entry(p).or_default().insert(first);
    }

    // ── the records ─────────────────────────────────────────────────────────
    let mut deleted: Vec<Deleted> = Vec::new();
    let all_gone = gone
        .iter()
        .flat_map(|(&g, set)| set.iter().map(move |&e| (g, Some(e))))
        .chain(removed.iter().map(|&r| (r, None)));
    for (g, only) in all_gone {
        let ifd = ifds.get(g).ok_or_else(|| bad(0))?;
        for (n, entry) in ifd.entries().iter().enumerate() {
            if only.is_some_and(|e| e != n) {
                continue;
            }
            let at = entry_start(blob, order, entry).ok_or_else(|| bad(start_of(entry)))?;
            deleted.push(Deleted {
                ifd_index: g,
                ifd: ifd.kind(),
                tag: entry.tag(),
                field_type: entry.field_type(),
                count: entry.count(),
                at,
            });
        }
    }
    deleted.sort_by_key(|d| d.at);

    if deleted.is_empty() && thumb.is_none() {
        return Ok(BlockPlan::default());
    }

    // ── table rewrites (rule 5) ─────────────────────────────────────────────
    let mut rewrite: Vec<usize> = gone.keys().copied().collect();
    if let Some(t) = &thumb {
        if !rewrite.contains(&t.ifd0) {
            rewrite.push(t.ifd0);
        }
    }
    rewrite.sort_unstable();
    let mut tables: Vec<(Range<usize>, Vec<u8>)> = Vec::new();
    for &g in &rewrite {
        let ifd = ifds.get(g).ok_or_else(|| bad(0))?;
        let table = ifd.table_range().ok_or_else(|| bad(0))?;
        let new_next = thumb.as_ref().filter(|t| t.ifd0 == g).map(|_| 0u32);
        let bytes = rewrite_table(blob, order, ifd, &table, gone.get(&g), new_next)
            .ok_or_else(|| bad(table.start))?;
        tables.push((table, bytes));
    }

    // ── scrubs (rule 8) ─────────────────────────────────────────────────────
    let mut scrubs: Vec<(Range<usize>, ScrubReason)> = Vec::new();
    let scrub_entries = gone
        .iter()
        .flat_map(|(&g, set)| set.iter().map(move |&e| (g, Some(e))))
        .chain(removed.iter().map(|&r| (r, None)));
    for (g, only) in scrub_entries {
        let ifd = ifds.get(g).ok_or_else(|| bad(0))?;
        for (n, entry) in ifd.entries().iter().enumerate() {
            if only.is_some_and(|e| e != n) {
                continue;
            }
            match c.payload_range(entry) {
                Some(r) => scrubs.push((r, ScrubReason::EntryValue)),
                // A value that cannot be sized cannot be zeroed.
                None if entry.byte_len().is_none() => {
                    return Err(refuse(
                        UneditableReason::PayloadUnreadable,
                        entry.value_at(),
                    ));
                }
                None => {}
            }
        }
    }
    for &r in &removed {
        let table = ifds
            .get(r)
            .and_then(Ifd::table_range)
            .ok_or_else(|| bad(0))?;
        scrubs.push((table, ScrubReason::IfdTable));
    }
    if let Some(t) = &thumb {
        scrubs.push((t.range.clone(), ScrubReason::Thumbnail));
    }
    scrubs.retain(|(r, _)| r.start < r.end);
    scrubs.sort_by_key(|(r, _)| (r.start, r.end));
    let mut merged: Vec<(Range<usize>, ScrubReason)> = Vec::new();
    for (r, why) in scrubs {
        match merged.last_mut() {
            Some((last, _)) if r.start < last.end => last.end = last.end.max(r.end),
            _ => merged.push((r, why)),
        }
    }

    // ── the bytes that must stay, and the conflicts (rule 8) ───────────────
    let mut keep: Vec<Range<usize>> = Vec::new();
    keep.push(0..HEADER_LEN);
    let mut kept_tables: Vec<Range<usize>> = Vec::new();
    for (i, ifd) in ifds.iter().enumerate() {
        let table = ifd.table_range().ok_or_else(|| bad(0))?;
        if removed.contains(&i) {
            continue;
        }
        kept_tables.push(table);
        for (n, entry) in ifd.entries().iter().enumerate() {
            let is_gone = gone.get(&i).is_some_and(|s| s.contains(&n));
            if !is_gone {
                keep.extend(c.payload_range(entry));
            }
        }
    }
    if thumb.is_none() {
        keep.extend(sel.thumbnail_range.and_then(|r| blob_range(blob, r)));
    }
    let keep = merge_ranges(keep);
    let must_stay = merge_ranges(keep.iter().chain(kept_tables.iter()).cloned().collect());
    let mut conflict: Option<usize> = None;
    let mut note = |at: Option<usize>| {
        if let Some(at) = at {
            conflict = Some(conflict.map_or(at, |c| c.min(at)));
        }
    };
    for (r, _) in &merged {
        note(first_touch(&must_stay, r));
    }
    for (t, _) in &tables {
        note(first_touch(&keep, t));
    }
    if let Some(at) = conflict {
        return Err(refuse(UneditableReason::SharedBytes, at));
    }

    // ── the patches ─────────────────────────────────────────────────────────
    let mut patches: Vec<BlobPatch> = Vec::new();
    for (range, bytes) in tables {
        patches.push(BlobPatch {
            at: range.start,
            bytes,
            kind: PatchKind::IfdTable,
            reason: None,
        });
    }
    for (range, why) in merged {
        patches.push(BlobPatch {
            at: range.start,
            bytes: vec![0; range.end.saturating_sub(range.start)],
            kind: PatchKind::Scrub,
            reason: Some(why),
        });
    }
    patches.sort_by_key(|p| p.at);
    Ok(BlockPlan { patches, deleted })
}

/// The new bytes of `ifd`'s table after deleting the entries `cut` (indices)
/// and, when `new_next` is set, replacing the next-IFD word: the count word,
/// the surviving entries copied verbatim in order, the next word, then one
/// zero entry-width for every deleted entry (rule 5).
fn rewrite_table(
    blob: &[u8],
    order: ByteOrder,
    ifd: &Ifd,
    table: &Range<usize>,
    cut: Option<&BTreeSet<usize>>,
    new_next: Option<u32>,
) -> Option<Vec<u8>> {
    let n = ifd.entries().len();
    if read_u16(blob, table.start, order).ok().map(usize::from) != Some(n) {
        return None;
    }
    let entries_at = table.start.checked_add(2)?;
    let mut kept: Vec<&[u8]> = Vec::new();
    for i in 0..n {
        if cut.is_some_and(|s| s.contains(&i)) {
            continue;
        }
        let at = entries_at.checked_add(i.checked_mul(ENTRY_LEN)?)?;
        kept.push(blob.get(at..at.checked_add(ENTRY_LEN)?)?);
    }
    let k = n.checked_sub(kept.len())?;
    let table_len = table.end.checked_sub(table.start)?;
    let mut out = Vec::with_capacity(table_len);
    out.extend_from_slice(&put16(order, u16::try_from(kept.len()).ok()?));
    for e in kept {
        out.extend_from_slice(e);
    }
    let next_at = table.end.checked_sub(4)?;
    match new_next {
        Some(v) => out.extend_from_slice(&put32(order, v)),
        None => out.extend_from_slice(blob.get(next_at..table.end)?),
    }
    out.resize(out.len().checked_add(k.checked_mul(ENTRY_LEN)?)?, 0);
    (out.len() == table_len).then_some(out)
}

// ── the JPEG side ───────────────────────────────────────────────────────────

/// How a policy treats the Exif segments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    /// Every Exif category is removed: the segments go whole (SPEC-020).
    Whole,
    /// No category that an Exif entry can have: the segments stay untouched.
    Untouched,
    /// Some of them: entries are deleted in place (this module).
    Partial,
}

/// The nine categories an Exif segment can hold (DEC-013 part 4), typed here by
/// hand from the decision.
pub(super) fn exif9() -> StripSpec {
    use crate::audit::Category as C;
    [
        C::Gps,
        C::DeviceId,
        C::Authorship,
        C::FreeText,
        C::Timestamps,
        C::CameraModel,
        C::Thumbnail,
        C::MakerNote,
        C::Software,
    ]
    .iter()
    .fold(StripSpec::none(), |s, c| s.include(*c))
}

/// The mode of a policy (SPEC-021 rules, DEC-013 amendment B).
pub(super) fn mode(spec: StripSpec) -> Mode {
    use crate::audit::Category as C;
    let all9 = exif9();
    if all9.is_subset_of(spec) {
        return Mode::Whole;
    }
    let can_occur = [C::Xmp, C::Iptc, C::Icc]
        .iter()
        .fold(all9, |s, c| s.include(*c));
    if spec.intersects(can_occur) {
        Mode::Partial
    } else {
        Mode::Untouched
    }
}

/// Everything the entry edits of a file amount to, in **input** coordinates.
#[derive(Debug, Clone, Default)]
pub(super) struct ExifEdits {
    /// `(input offset, bytes, kind, scrub reason)`, in input order.
    pub patches: Vec<(usize, Vec<u8>, PatchKind, Option<ScrubReason>)>,
    /// One per deleted entry, in input order.
    pub entries: Vec<EntryRecord>,
}

/// Plan the entry edits for a partial policy over every Exif block of the file
/// (rule 3 first for every block, then rules 4 to 8 block by block).
pub(super) fn plan_entries(
    meta: &JpegMetadata<'_>,
    spec: StripSpec,
    audited: &FileAudit<'_>,
) -> Result<ExifEdits, StripError> {
    let segments = meta.jpeg().segments();
    let exif_segments: Vec<usize> = segments
        .iter()
        .enumerate()
        .filter(|(_, s)| s.kind() == SegmentKind::Exif)
        .map(|(i, _)| i)
        .collect();
    let uneditable = |reason: UneditableReason, segment: Option<usize>| StripError::Uneditable {
        reason,
        offset: segment
            .and_then(|i| segments.get(i))
            .map_or(0, |s| s.offset()),
    };
    let first_exif = exif_segments.first().copied();
    if exif_segments.is_empty() {
        return Ok(ExifEdits::default());
    }
    if exif_segments.len() > meta.exif_blocks().len() {
        return Err(uneditable(
            UneditableReason::TooManyExifSegments,
            exif_segments.get(meta.exif_blocks().len()).copied(),
        ));
    }
    if audited.health().inventory_truncated {
        return Err(uneditable(UneditableReason::InventoryTruncated, first_exif));
    }
    for block in meta.exif_blocks() {
        let seg = segments.get(block.segment());
        if seg.is_some_and(|s| s.after_scan()) {
            return Err(uneditable(
                UneditableReason::AfterScan,
                Some(block.segment()),
            ));
        }
        if !block.container().is_ok_and(Container::is_clean) {
            return Err(uneditable(
                UneditableReason::BlockNotClean,
                Some(block.segment()),
            ));
        }
    }

    let mut edits = ExifEdits::default();
    for (b, block) in meta.exif_blocks().iter().enumerate() {
        let Ok(container) = block.container() else {
            continue;
        };
        let selection = select(audited, b, container, spec)
            .ok_or_else(|| uneditable(UneditableReason::BlockNotClean, Some(block.segment())))?;
        let plan = plan_block(block.tiff(), container, &selection).map_err(|r| {
            StripError::Uneditable {
                reason: r.reason,
                offset: block.tiff_at().saturating_add(r.at),
            }
        })?;
        for p in plan.patches {
            edits.patches.push((
                block.tiff_at().saturating_add(p.at),
                p.bytes,
                p.kind,
                p.reason,
            ));
        }
        for d in plan.deleted {
            edits.entries.push(EntryRecord {
                block: b,
                ifd: d.ifd,
                ifd_index: d.ifd_index,
                tag: d.tag,
                field_type: d.field_type,
                count: d.count,
                offset: block.tiff_at().saturating_add(d.at),
            });
        }
    }
    edits.patches.sort_by_key(|p| p.0);
    edits.entries.sort_by_key(|e| e.offset);
    Ok(edits)
}

/// What block `b` loses under `spec`: the audit's findings in the policy's
/// categories (rule 2), mapped to `(ifd, entry)` positions of the walk. `None`
/// when the audit and the walk disagree.
fn select(
    audited: &FileAudit<'_>,
    b: usize,
    container: &Container<'_>,
    spec: StripSpec,
) -> Option<Selection> {
    use crate::audit::Category;
    // A block with no tag item at all (an emptied IFD0) has no findings either.
    let first = audited
        .items()
        .iter()
        .position(|it| it.origin().block == Some(b))
        .unwrap_or(0);
    let mut sel = Selection {
        gps: spec.contains(Category::Gps),
        thumbnail: spec.contains(Category::Thumbnail),
        ..Selection::default()
    };
    for f in audited
        .findings()
        .iter()
        .filter(|f| f.at.origin().block == Some(b))
    {
        let ItemKind::Tag {
            ifd_index,
            tag,
            ifd,
            ..
        } = f.at.kind()
        else {
            continue;
        };
        if ifd == IfdKind::Ifd1 && tag == TAG_THUMB_OFFSET {
            if let Some(range) = f.thumbnail.and_then(|t| t.range) {
                sel.thumbnail_range = Some(range);
            }
        }
        if !spec.contains(f.category) {
            continue;
        }
        let before = container
            .ifds()
            .get(..ifd_index)?
            .iter()
            .try_fold(0usize, |n, i| n.checked_add(i.entries().len()))?;
        let entry = f.item?.checked_sub(first)?.checked_sub(before)?;
        let seen = container.ifds().get(ifd_index)?.entries().get(entry)?;
        if seen.tag() != tag {
            return None;
        }
        sel.deleted.push((ifd_index, entry));
    }
    Some(sel)
}

#[cfg(test)]
#[path = "exif_tests.rs"]
mod tests;
