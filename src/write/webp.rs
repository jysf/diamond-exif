//! The WebP planner (SPEC-038, DEC-022).
//!
//! [`plan`] walks the RIFF chunks (every one the reader records, wherever it
//! lies: before or after the image data, after the declared RIFF size, after an
//! empty null chunk), decides each by its four-character code and the policy,
//! and turns the result into an [`EditPlan`]: whole-chunk removals (the chunk
//! header, its data and its pad byte when it has one) and same-length patches
//! (an edited `EXIF` chunk, the RIFF size, the `VP8X` flag byte). A kept chunk
//! is never touched.
//!
//! Order of refusals, pinned (the fuzz harness will predict the same order): a
//! walk that cannot be trusted (`Unstrippable`), an `EXIF` edit the writer
//! cannot make (`Uneditable`), a removal that cuts kept bytes (`Overlap`), a
//! RIFF size that would underflow (`Unstrippable`).
//!
//! Provenance (DEC-003, class 1): net-new code written from DEC-022 over this
//! crate's own WebP reader and audit. No ExifTool source is read.

use core::ops::Range;

use crate::audit::{audit, Category, ContainerKind};
use crate::error::Error;
use crate::policy::StripSpec;
use crate::tiff::Container;
use crate::webp::{self, chunk_class, ChunkClass, ChunkKind, WebpMetadata};

use super::exif::{exif9, mode, plan_block, select, Mode};
use super::jpeg::Planned;
use super::plan::{merge_ranges, removed_in};
use super::png::removes_carriers;
use super::{
    EditPlan, EntryRecord, PatchKind, Removal, RemovalKind, StripError, UneditableReason,
    UnstrippableReason,
};

/// Bytes of the RIFF header: `RIFF`, the size field, `WEBP`.
const HEADER: usize = 12;

/// Offset of the 4-byte RIFF size field.
const SIZE_AT: usize = 4;

/// Bytes of chunk framing before the data.
const CHUNK_HEAD: usize = 8;

/// Bytes of data a `VP8X` chunk needs for ExifTool 13.55 to print its flags
/// (measured: 4 or more; 3 or fewer print nothing). The reader's `vp8x` wants
/// the full 10 and is not used for the flags (DEC-022 clause 7).
const VP8X_FLAGS_LEN: usize = 4;

/// The `VP8X` flag bits the strip may clear (DEC-022 clause 7): ICC profile,
/// Exif, XMP.
pub(super) const FLAG_ICC: u8 = 0x20;
pub(super) const FLAG_EXIF: u8 = 0x08;
pub(super) const FLAG_XMP: u8 = 0x04;

/// Whether `code` is one of the chunk types a WebP strip may remove or edit:
/// `EXIF`, `XMP `, `XMP\0`, `ICCP` and every carrier the reader locates by
/// type. Every other chunk (image data, `VP8X`, `ANIM`, `ANMF`, unknown types)
/// must come out byte for byte as it went in.
pub(super) fn removable(kind: ChunkKind, code: [u8; 4], data: &[u8]) -> bool {
    matches!(kind, ChunkKind::Exif | ChunkKind::Xmp | ChunkKind::Iccp)
        || (kind == ChunkKind::Other && chunk_class(code, data).is_some())
}

/// One thing the plan removes.
struct Target {
    range: Range<usize>,
    category: Category,
    kind: RemovalKind,
}

/// Plan the strip of the WebP `data` for the categories `spec`.
pub(super) fn plan(data: &[u8], spec: StripSpec) -> Result<Planned, StripError> {
    let meta = webp::read(data).map_err(|_| StripError::Unsupported {
        kind: ContainerKind::Webp,
    })?;
    refuse_untrusted_walk(&meta)?;
    let chunks = meta.webp().chunks();
    let carriers = removes_carriers(spec);

    let mut targets: Vec<Target> = Vec::new();
    for c in chunks {
        let Some((category, kind, remove)) =
            classify(c.kind(), c.fourcc(), c.data(), spec, carriers)
        else {
            continue;
        };
        if remove {
            targets.push(Target {
                range: c.range(),
                category,
                kind,
            });
        }
    }

    let edits = exif_chunks(data, &meta, spec, &mut targets)?;
    targets.sort_by_key(|t| (t.range.start, t.range.end));
    let removals_merged = merge_ranges(targets.iter().map(|t| t.range.clone()).collect());
    overlap(&meta, &targets, &removals_merged)?;

    let mut plan = EditPlan::new();
    for t in &targets {
        plan.remove(t.range.clone());
    }
    let mut patches = Vec::new();
    let mut scrubs = Vec::new();
    for (offset, bytes, kind, reason) in &edits.patches {
        plan.patch(*offset, bytes);
        patches.push((*offset, bytes.len(), *kind));
        if let Some(reason) = reason {
            scrubs.push((*offset, bytes.len(), *reason));
        }
    }

    // Rule 6: the declared RIFF size loses exactly the bytes removed from
    // inside the declared range.
    if let Some(new_size) = riff_size_after(data, &meta, &removals_merged)? {
        plan.patch(SIZE_AT, &new_size.to_le_bytes());
        patches.push((SIZE_AT, 4, PatchKind::RiffSize));
    }
    // Rule 7: a `VP8X` flag whose chunks are all gone is cleared, in every
    // `VP8X` that has a flag byte ExifTool reads.
    for (at, flags) in flags_after(&meta, &targets) {
        plan.patch(at, &[flags]);
        patches.push((at, 1, PatchKind::Vp8xFlags));
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
        entries: edits.entries,
    })
}

/// Rule 8: a walk halted by a cap, or a chunk that runs past the end of the
/// input: the bytes after it were never audited. Fewer than 8 bytes after the
/// last chunk are not a chunk at all and are kept as they are.
fn refuse_untrusted_walk(meta: &WebpMetadata<'_>) -> Result<(), StripError> {
    for issue in meta.webp_issues() {
        let reason = match issue.error() {
            // A real overrun names the chunk's whole size, which is more than
            // the 8 bytes of a frame; a bare tail names exactly the frame.
            Error::Truncated { len, .. } if *len > CHUNK_HEAD => UnstrippableReason::ChunkPastEnd,
            Error::TooManyChunks { .. } => UnstrippableReason::TooManySegments,
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

/// Rules 2 and 4 for one chunk that is not decided by its entries (`EXIF` is
/// [`exif_chunks`]): its category, the kind of removal it would be and whether
/// the policy removes it. `None` for a chunk the writer has no rule for.
fn classify(
    kind: ChunkKind,
    code: [u8; 4],
    data: &[u8],
    spec: StripSpec,
    carriers: bool,
) -> Option<(Category, RemovalKind, bool)> {
    let by_category =
        |category: Category, kind: RemovalKind| Some((category, kind, spec.contains(category)));
    // A carrier goes with the category it has in the audit (`OtherSegment`) or
    // with any tag category, `Xmp` or `Iptc` (DEC-022 clause 4).
    let carrier = |kind: RemovalKind| Some((Category::OtherSegment, kind, carriers));
    match kind {
        ChunkKind::Xmp => by_category(Category::Xmp, RemovalKind::Xmp),
        ChunkKind::Iccp => by_category(Category::Icc, RemovalKind::Icc),
        ChunkKind::Other => match chunk_class(code, data)? {
            ChunkClass::Jumbf => by_category(Category::Jumbf, RemovalKind::Jumbf),
            ChunkClass::Other => by_category(Category::OtherSegment, RemovalKind::OtherChunk),
            _ => carrier(RemovalKind::Carrier),
        },
        _ => None,
    }
}

/// What the entry edits of the file's `EXIF` chunks amount to, in input
/// coordinates.
#[derive(Default)]
struct Edits {
    patches: Vec<(usize, Vec<u8>, PatchKind, Option<super::ScrubReason>)>,
    entries: Vec<EntryRecord>,
}

/// Rule 3: every `EXIF` chunk goes whole (every Exif category in the policy),
/// none is touched (no category an Exif entry can have), or its entries are
/// deleted in place. A WebP `EXIF` chunk has no checksum, so there is nothing
/// to patch beyond the entries.
fn exif_chunks(
    data: &[u8],
    meta: &WebpMetadata<'_>,
    spec: StripSpec,
    out: &mut Vec<Target>,
) -> Result<Edits, StripError> {
    let exif_chunks: Vec<_> = meta
        .webp()
        .chunks()
        .iter()
        .filter(|c| c.kind() == ChunkKind::Exif)
        .collect();
    if exif_chunks.is_empty() {
        return Ok(Edits::default());
    }
    match mode(spec) {
        Mode::Untouched => Ok(Edits::default()),
        Mode::Whole => {
            let all9 = exif9();
            let audited = audit(data);
            for c in exif_chunks {
                // The category a removed block is recorded under: the first
                // Exif finding the audit made in it, `Thumbnail` if none (an
                // unparseable block, or a 17th chunk the reader did not parse).
                let block = meta
                    .exif_blocks()
                    .iter()
                    .position(|b| b.offset() == c.offset());
                let category = audited
                    .findings()
                    .iter()
                    .find(|f| {
                        block.is_some() && f.at.origin().block == block && all9.contains(f.category)
                    })
                    .map_or(Category::Thumbnail, |f| f.category);
                out.push(Target {
                    range: c.range(),
                    category,
                    kind: RemovalKind::Exif,
                });
            }
            Ok(Edits::default())
        }
        Mode::Partial => {
            let first = exif_chunks.first().map_or(0, |c| c.offset());
            let uneditable =
                |reason: UneditableReason, offset: usize| StripError::Uneditable { reason, offset };
            // More `EXIF` chunks than the reader parses: the extra ones were
            // never read, so an edit cannot be shown to be complete.
            let parsed = meta.exif_blocks().len();
            if let Some(c) = exif_chunks.get(parsed) {
                return Err(uneditable(
                    UneditableReason::TooManyExifSegments,
                    c.offset(),
                ));
            }
            let audited = audit(data);
            if audited.health().inventory_truncated {
                return Err(uneditable(UneditableReason::InventoryTruncated, first));
            }
            for block in meta.exif_blocks() {
                if !block.container().is_ok_and(Container::is_clean) {
                    return Err(uneditable(UneditableReason::BlockNotClean, block.offset()));
                }
            }
            let mut edits = Edits::default();
            for (b, block) in meta.exif_blocks().iter().enumerate() {
                let Ok(container) = block.container() else {
                    continue;
                };
                let selection = select(&audited, Some(b), container, spec)
                    .ok_or_else(|| uneditable(UneditableReason::BlockNotClean, block.offset()))?;
                let planned = plan_block(block.tiff(), container, &selection)
                    .map_err(|r| uneditable(r.reason, block.tiff_at().saturating_add(r.at)))?;
                for p in &planned.patches {
                    let at = block.tiff_at().saturating_add(p.at);
                    edits.patches.push((at, p.bytes.clone(), p.kind, p.reason));
                }
                for d in planned.deleted {
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
    }
}

/// Rule 6: the new declared RIFF size, or `None` when nothing inside the
/// declared range was removed. The declared range is the bytes from offset 8
/// to 8 plus the declared size (clipped to the input).
fn riff_size_after(
    data: &[u8],
    meta: &WebpMetadata<'_>,
    removals: &[Range<usize>],
) -> Result<Option<u32>, StripError> {
    let declared = meta.webp().declared_size();
    let window_end = usize::try_from(declared)
        .ok()
        .and_then(|d| d.checked_add(CHUNK_HEAD))
        .unwrap_or(usize::MAX)
        .min(data.len());
    let inside = removed_in(removals, CHUNK_HEAD..window_end);
    if inside == 0 {
        return Ok(None);
    }
    let refuse = || StripError::Unstrippable {
        reason: UnstrippableReason::RangeOverflow,
        offset: SIZE_AT,
    };
    let inside = u32::try_from(inside).map_err(|_| refuse())?;
    declared.checked_sub(inside).map(Some).ok_or_else(refuse)
}

/// Rule 7: the flag byte offset and new value of every `VP8X` chunk whose data
/// holds a flag byte ExifTool reads (4 bytes or more, wherever it lies), when
/// the strip removes every chunk of a kind the flags announce (ICC, Exif, XMP)
/// and the byte still has the bit set. ExifTool prints the flags of each such
/// chunk and reports the last, so all are cleared.
fn flags_after(meta: &WebpMetadata<'_>, targets: &[Target]) -> Vec<(usize, u8)> {
    let gone = |kind: ChunkKind| {
        let mut any = false;
        let all_removed = meta
            .webp()
            .chunks()
            .iter()
            .filter(|c| c.kind() == kind)
            .all(|c| {
                any = true;
                let r = c.range();
                targets.iter().any(|t| t.range == r)
            });
        any && all_removed
    };
    let mut clear = 0u8;
    for (bit, kind) in [
        (FLAG_ICC, ChunkKind::Iccp),
        (FLAG_EXIF, ChunkKind::Exif),
        (FLAG_XMP, ChunkKind::Xmp),
    ] {
        if gone(kind) {
            clear |= bit;
        }
    }
    if clear == 0 {
        return Vec::new();
    }
    meta.webp()
        .chunks()
        .iter()
        .filter(|c| c.kind() == ChunkKind::Vp8x && c.data().len() >= VP8X_FLAGS_LEN)
        .filter_map(|c| {
            let was = *c.data().first()?;
            let now = was & !clear;
            let at = c.offset().checked_add(CHUNK_HEAD)?;
            (now != was).then_some((at, now))
        })
        .collect()
}

/// Rule 8, `Overlap`: no removed range may intersect the header or a kept
/// chunk. Reports the first input offset that is both removed and kept.
fn overlap(
    meta: &WebpMetadata<'_>,
    targets: &[Target],
    removals: &[Range<usize>],
) -> Result<(), StripError> {
    // The 12-byte header is never removed.
    let mut kept: Vec<Range<usize>> = Vec::new();
    kept.push(0..HEADER);
    for c in meta.webp().chunks() {
        let r = c.range();
        if !targets.iter().any(|t| t.range == r) {
            kept.push(r);
        }
    }
    let kept = merge_ranges(kept);
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
    use crate::policy::StripPolicy;

    fn chunk(code: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut v = code.to_vec();
        v.extend_from_slice(&u32::try_from(data.len()).unwrap().to_le_bytes());
        v.extend_from_slice(data);
        if data.len() % 2 == 1 {
            v.push(0);
        }
        v
    }

    fn riff(parts: &[Vec<u8>]) -> Vec<u8> {
        let body: usize = parts.iter().map(Vec::len).sum();
        let mut v = b"RIFF".to_vec();
        v.extend_from_slice(&u32::try_from(body + 4).unwrap().to_le_bytes());
        v.extend_from_slice(b"WEBP");
        for p in parts {
            v.extend_from_slice(p);
        }
        v
    }

    fn vp8x(flags: u8) -> Vec<u8> {
        chunk(b"VP8X", &[flags, 0, 0, 0, 0, 0, 0, 0, 0, 0])
    }

    #[test]
    fn a_short_tail_is_kept_and_a_chunk_past_the_end_is_refused() {
        let mut v = riff(&[vp8x(0), chunk(b"IDIT", b"Jane Doe")]);
        v.extend_from_slice(&[1, 2, 3]);
        let planned = plan(&v, StripPolicy::Gps.spec()).unwrap();
        assert_eq!(planned.removals.len(), 1);
        let mut v = riff(&[vp8x(0)]);
        let at = v.len();
        v.extend_from_slice(b"LIST\xe8\x03\0\0abc");
        let Err(StripError::Unstrippable { reason, offset }) = plan(&v, StripPolicy::Gps.spec())
        else {
            panic!("expected a refusal");
        };
        assert_eq!(reason, UnstrippableReason::ChunkPastEnd);
        assert_eq!(offset, at);
    }

    #[test]
    fn the_size_patch_is_the_declared_size_less_the_bytes_removed_inside() {
        let c = chunk(b"IDIT", b"Jane Doe");
        let v = riff(&[vp8x(0), c, chunk(b"VP8L", &[0x2F, 0, 0, 0, 0])]);
        let planned = plan(&v, StripPolicy::Gps.spec()).unwrap();
        let out = planned.plan.apply(&v).unwrap();
        let size = u32::from_le_bytes(out[4..8].try_into().unwrap());
        assert_eq!(size as usize, out.len() - 8);
        assert!(planned
            .patches
            .iter()
            .any(|p| p.2 == PatchKind::RiffSize && p.0 == 4 && p.1 == 4));
        // Nothing removed: no edit at all.
        let clean = riff(&[vp8x(0), chunk(b"VP8L", &[0x2F, 0, 0, 0, 0])]);
        assert!(plan(&clean, StripPolicy::Everything.spec())
            .unwrap()
            .plan
            .is_empty());
    }

    #[test]
    fn the_flag_patch_needs_the_reported_vp8x_and_a_kind_removed_entirely() {
        let xmp = chunk(b"XMP ", b"<x/>");
        let v = riff(&[vp8x(FLAG_XMP | 0x10), xmp.clone()]);
        let planned = plan(&v, StripPolicy::All.spec()).unwrap();
        let flag = planned
            .patches
            .iter()
            .find(|p| p.2 == PatchKind::Vp8xFlags)
            .unwrap();
        assert_eq!(flag.0, 20);
        let out = planned.plan.apply(&v).unwrap();
        assert_eq!(out[20], 0x10);
        // No `VP8X`: nothing to patch, nothing refused.
        let bare = riff(&[xmp]);
        let planned = plan(&bare, StripPolicy::All.spec()).unwrap();
        assert!(planned.patches.iter().all(|p| p.2 != PatchKind::Vp8xFlags));
    }
}
