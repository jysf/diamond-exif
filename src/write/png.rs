//! The PNG planner (SPEC-037, DEC-021).
//!
//! [`plan`] walks the chunks (the ones before `IEND` and the ones framed after
//! it), decides each by its type and, for text, by its keyword, and turns the
//! policy into an [`EditPlan`]: whole-chunk removals and same-length patches
//! (an edited `eXIf` and its chunk CRC). A kept chunk is never touched, so its
//! CRC stays what it was, a wrong one included.
//!
//! Order of refusals, pinned (the fuzz harness predicts the same order): a walk
//! that cannot be trusted (`Unstrippable`), a trailer past the scan cap
//! (`Unstrippable`), an `eXIf` edit the writer cannot make (`Uneditable`), a
//! removal that cuts kept bytes (`Overlap`).
//!
//! Provenance (DEC-003, class 1): net-new code written from DEC-021 over this
//! crate's own PNG reader and audit. No ExifTool source is read.

use core::ops::Range;
use std::collections::{BTreeMap, HashSet};

use crate::audit::{audit, text_keyword_category, Category};
use crate::error::Error;
use crate::png::{
    self, chunk_class, crc32_parts, frame, ChunkClass, Frame, PngMetadata, PngRegion,
    MAX_PNG_TRAILER_CHUNKS,
};
use crate::policy::StripSpec;
use crate::tiff::Container;

use super::exif::{exif10, mode, plan_block, select, Mode};
use super::jpeg::Planned;
use super::plan::merge_ranges;
use super::{
    EditPlan, EntryRecord, PatchKind, Removal, RemovalKind, StripError, UneditableReason,
    UnstrippableReason,
};

/// One thing the plan removes.
struct Target {
    range: Range<usize>,
    category: Category,
    kind: RemovalKind,
}

/// One chunk, walked or framed after `IEND`: where it is and what type it is.
#[derive(Clone, Copy)]
struct ChunkRef {
    range_start: usize,
    range_end: usize,
    ty: [u8; 4],
}

impl ChunkRef {
    fn range(&self) -> Range<usize> {
        self.range_start..self.range_end
    }
}

/// Whether the policy removes the opaque carriers (DEC-021 clause 4): any tag
/// category, `Xmp` or `Iptc`, which is every preset. `OtherSegment` is the
/// carriers' own category and removes them too.
pub(super) fn removes_carriers(spec: StripSpec) -> bool {
    let reach = [Category::Xmp, Category::Iptc]
        .iter()
        .fold(exif10(), |s, c| s.include(*c));
    spec.intersects(reach) || spec.contains(Category::OtherSegment)
}

/// Where the bytes after `IEND` start: `data.len()` minus the trailer.
pub(super) fn trailer_start(data: &[u8], meta: &PngMetadata<'_>) -> usize {
    data.len().saturating_sub(meta.png().trailer().len())
}

/// What [`trailer_chunks`] found: `(start, end, type)` of each chunk, where the
/// framing stopped, and whether the cap stopped it with a chunk still to come.
pub(super) type TrailerScan = (Vec<(usize, usize, [u8; 4])>, usize, bool);

/// The chunks framed after `IEND` (the reader's own scan: the first chunk that
/// does not fit ends it, at most [`MAX_PNG_TRAILER_CHUNKS`]). Returns them,
/// the offset where the framing stopped, and whether the cap stopped it with
/// another chunk still to come.
pub(super) fn trailer_chunks(data: &[u8], start: usize) -> TrailerScan {
    let mut out = Vec::new();
    let mut at = start;
    for _ in 0..MAX_PNG_TRAILER_CHUNKS {
        let Frame::Chunk(framed) = frame(data, at) else {
            return (out, at, false);
        };
        out.push((at, framed.end, framed.ty));
        at = framed.end;
    }
    let more = matches!(frame(data, at), Frame::Chunk(_));
    (out, at, more)
}

/// Plan the strip of the PNG `data` for the categories `spec`.
pub(super) fn plan(data: &[u8], spec: StripSpec) -> Result<Planned, StripError> {
    let meta = png::read(data).map_err(|_| StripError::Unsupported {
        kind: crate::audit::ContainerKind::Png,
    })?;
    refuse_untrusted_walk(&meta)?;

    // Every chunk, in input order, with its range.
    let mut chunks: Vec<ChunkRef> = meta
        .png()
        .chunks()
        .iter()
        .map(|c| {
            let r = c.range();
            ChunkRef {
                range_start: r.start,
                range_end: r.end,
                ty: c.chunk_type(),
            }
        })
        .collect();
    let tstart = trailer_start(data, &meta);
    let trailer_len = data.len().saturating_sub(tstart);
    let trailer_gone = spec.contains(Category::OtherSegment) && trailer_len > 0;
    if trailer_len > 0 {
        let (framed, _, capped) = trailer_chunks(data, tstart);
        // A trailer past the scan cap holds chunks nobody audited; removing it
        // whole (OtherSegment) needs no audit of them.
        if capped && !trailer_gone {
            return Err(StripError::Unstrippable {
                reason: UnstrippableReason::TooManySegments,
                offset: tstart,
            });
        }
        chunks.extend(framed.into_iter().map(|(start, end, ty)| ChunkRef {
            range_start: start,
            range_end: end,
            ty,
        }));
    }
    // A chunk inside a trailer that goes whole is covered by that removal.
    let live = |c: &ChunkRef| !(trailer_gone && c.range_start >= tstart);

    let by_offset: BTreeMap<usize, &PngRegion<'_>> =
        meta.regions().iter().map(|r| (r.offset(), r)).collect();
    let carriers = removes_carriers(spec);

    // The bytes after `IEND` are one `OtherSegment` finding, so they go whole
    // with that category. A chunk in them that the policy would otherwise keep
    // and that has findings of its own (a located region, or an `eXIf`) would be
    // cut with them: `Overlap`, as a kept embedded image is for a JPEG trailer.
    if trailer_gone {
        for c in chunks.iter().filter(|c| !live(c)) {
            let would_go = if &c.ty == b"eXIf" {
                mode(spec) == Mode::Whole
            } else {
                let region = by_offset.get(&c.range_start).copied();
                match classify(c.ty, region, spec, carriers) {
                    Some((_, _, remove)) => remove || region.is_none(),
                    None => true,
                }
            };
            if !would_go {
                return Err(StripError::Overlap {
                    offset: c.range_start,
                });
            }
        }
    }

    let mut targets: Vec<Target> = Vec::new();
    for c in chunks.iter().filter(|c| live(c)) {
        let region = by_offset.get(&c.range_start).copied();
        let Some((category, kind, remove)) = classify(c.ty, region, spec, carriers) else {
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
    if trailer_gone {
        targets.push(Target {
            range: tstart..data.len(),
            category: Category::OtherSegment,
            kind: RemovalKind::Trailer,
        });
    }

    let edits = exif_chunks(
        data,
        &meta,
        spec,
        &chunks,
        trailer_gone,
        tstart,
        &mut targets,
    )?;
    targets.sort_by_key(|t| (t.range.start, t.range.end));
    let removals_merged = merge_ranges(targets.iter().map(|t| t.range.clone()).collect());
    overlap(&chunks, &targets, &removals_merged, tstart, trailer_gone)?;

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

/// Rule 5: a walk halted by a cap, a chunk past the end of the input, a bad
/// length word, or a region cap: the bytes after it were never audited.
fn refuse_untrusted_walk(meta: &PngMetadata<'_>) -> Result<(), StripError> {
    for issue in meta.png_issues() {
        let reason = match issue.error() {
            Error::BadChunkLength { .. } => UnstrippableReason::BadSegmentLength,
            Error::TooManyChunks { .. } | Error::TooManyRegions { .. } => {
                UnstrippableReason::TooManySegments
            }
            Error::TooManyIssues { .. } => UnstrippableReason::TooManyIssues,
            Error::Truncated { .. } => UnstrippableReason::ChunkPastEnd,
            _ => continue,
        };
        return Err(StripError::Unstrippable {
            reason,
            offset: issue.offset(),
        });
    }
    Ok(())
}

/// Rules 2 and 4 for one chunk that is not an `eXIf`: its category, the kind of
/// removal it would be, and whether the policy removes it. `None` for a chunk
/// the writer has no rule for (image data, structure, unknown types).
fn classify(
    ty: [u8; 4],
    region: Option<&PngRegion<'_>>,
    spec: StripSpec,
    carriers: bool,
) -> Option<(Category, RemovalKind, bool)> {
    let by_category =
        |category: Category, kind: RemovalKind| Some((category, kind, spec.contains(category)));
    // A carrier goes with the category it has in the audit (`OtherSegment`) or
    // with any tag category, `Xmp` or `Iptc` (DEC-021 clause 4).
    let carrier = |kind: RemovalKind| Some((Category::OtherSegment, kind, carriers));
    match region {
        Some(PngRegion::Xmp { .. }) => by_category(Category::Xmp, RemovalKind::Xmp),
        Some(PngRegion::Icc { .. }) => by_category(Category::Icc, RemovalKind::Icc),
        Some(PngRegion::Text { keyword, .. }) => {
            by_category(text_keyword_category(keyword), RemovalKind::Text)
        }
        Some(PngRegion::RawProfile { .. }) => carrier(RemovalKind::RawProfile),
        Some(PngRegion::Chunk { class, .. }) => match class {
            ChunkClass::Time => by_category(Category::Timestamps, RemovalKind::Time),
            ChunkClass::ExifCarrier | ChunkClass::XmlCarrier => carrier(RemovalKind::Carrier),
            ChunkClass::Jumbf => by_category(Category::Jumbf, RemovalKind::Jumbf),
            _ => by_category(Category::OtherSegment, RemovalKind::OtherChunk),
        },
        // A text or `iCCP` chunk the reader could not locate (no NUL, no method
        // byte) has no finding; it is still text or a profile, so the policy's
        // `FreeText` or `Icc` removes it.
        None => match &ty {
            b"tEXt" | b"zTXt" | b"iTXt" => by_category(Category::FreeText, RemovalKind::Text),
            b"iCCP" => by_category(Category::Icc, RemovalKind::Icc),
            _ => {
                // A chunk of a carrier type with no region cannot happen (the
                // reader locates every one); treat it as the class says.
                match chunk_class(ty) {
                    Some(ChunkClass::Time) => by_category(Category::Timestamps, RemovalKind::Time),
                    _ => None,
                }
            }
        },
    }
}

/// What the entry edits of the file's `eXIf` chunks amount to, in input
/// coordinates.
#[derive(Default)]
struct Edits {
    patches: Vec<(usize, Vec<u8>, PatchKind, Option<super::ScrubReason>)>,
    entries: Vec<EntryRecord>,
}

/// Rule 3: every `eXIf` chunk goes whole (every Exif category in the policy),
/// none is touched (no category an Exif entry can have), or its entries are
/// deleted in place and the chunk CRC patched.
fn exif_chunks(
    data: &[u8],
    meta: &PngMetadata<'_>,
    spec: StripSpec,
    chunks: &[ChunkRef],
    trailer_gone: bool,
    tstart: usize,
    out: &mut Vec<Target>,
) -> Result<Edits, StripError> {
    let live = |offset: usize| !(trailer_gone && offset >= tstart);
    let exif_chunks: Vec<&ChunkRef> = chunks
        .iter()
        .filter(|c| &c.ty == b"eXIf" && live(c.range_start))
        .collect();
    if exif_chunks.is_empty() {
        return Ok(Edits::default());
    }
    match mode(spec) {
        Mode::Untouched => Ok(Edits::default()),
        Mode::Whole => {
            let all10 = exif10();
            let audited = audit(data);
            for c in exif_chunks {
                // The category a removed block is recorded under: the first
                // Exif finding the audit made in it, `Thumbnail` if none.
                let block = meta
                    .exif_blocks()
                    .iter()
                    .position(|b| b.offset() == c.range_start);
                let category = audited
                    .findings()
                    .iter()
                    .find(|f| {
                        block.is_some()
                            && f.at.origin().block == block
                            && all10.contains(f.category)
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
            let first = exif_chunks.first().map_or(0, |c| c.range_start);
            let uneditable =
                |reason: UneditableReason, offset: usize| StripError::Uneditable { reason, offset };
            // More `eXIf` chunks than the reader parses: the extra ones were
            // never read, so an edit cannot be shown to be complete.
            let parsed = meta.exif_blocks().len();
            if let Some(c) = chunks.iter().filter(|c| &c.ty == b"eXIf").nth(parsed) {
                if live(c.range_start) {
                    return Err(uneditable(
                        UneditableReason::TooManyExifSegments,
                        c.range_start,
                    ));
                }
            }
            let audited = audit(data);
            if audited.health().inventory_truncated {
                return Err(uneditable(UneditableReason::InventoryTruncated, first));
            }
            // Every parsed block must be a clean tolerant parse, including the
            // ones inside a trailer that goes whole (they are never edited, but
            // an unreadable one is still unreadable).
            for block in meta.exif_blocks() {
                if live(block.offset()) && !block.container().is_ok_and(Container::is_clean) {
                    return Err(uneditable(UneditableReason::BlockNotClean, block.offset()));
                }
            }
            let mut edits = Edits::default();
            for (b, block) in meta.exif_blocks().iter().enumerate() {
                if !live(block.offset()) {
                    continue;
                }
                let Ok(container) = block.container() else {
                    continue;
                };
                let selection = select(&audited, Some(b), container, spec)
                    .ok_or_else(|| uneditable(UneditableReason::BlockNotClean, block.offset()))?;
                let planned = plan_block(block.tiff(), container, &selection)
                    .map_err(|r| uneditable(r.reason, block.tiff_at().saturating_add(r.at)))?;
                if planned.patches.is_empty() {
                    continue;
                }
                let chunk = chunks
                    .iter()
                    .find(|c| c.range_start == block.offset())
                    .ok_or_else(|| uneditable(UneditableReason::BlockNotClean, block.offset()))?;
                // The chunk's type and data, edited, to recompute the CRC from
                // the bytes the output will hold.
                let body_start = chunk.range_start.saturating_add(4);
                let body_end = chunk.range_end.saturating_sub(4);
                let mut body = data
                    .get(body_start..body_end)
                    .ok_or_else(|| uneditable(UneditableReason::BlockNotClean, block.offset()))?
                    .to_vec();
                for p in &planned.patches {
                    let at = block.tiff_at().saturating_add(p.at);
                    let rel = at.checked_sub(body_start);
                    let end = rel.and_then(|r| r.checked_add(p.bytes.len()));
                    let (Some(rel), Some(end)) = (rel, end) else {
                        return Err(uneditable(UneditableReason::BlockNotClean, block.offset()));
                    };
                    let Some(dst) = body.get_mut(rel..end) else {
                        return Err(uneditable(UneditableReason::BlockNotClean, block.offset()));
                    };
                    dst.copy_from_slice(&p.bytes);
                    edits.patches.push((at, p.bytes.clone(), p.kind, p.reason));
                }
                let (ty, edited) = body.split_at(body.len().min(4));
                let crc = crc32_parts(ty, edited).to_be_bytes();
                if data.get(body_end..chunk.range_end) != Some(crc.as_slice()) {
                    edits
                        .patches
                        .push((body_end, crc.to_vec(), PatchKind::ChunkCrc, None));
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

/// Rule 5, `Overlap`: no removed range may intersect the signature, a kept
/// chunk or the kept part of the trailer. Reports the first input offset that
/// is both removed and kept.
fn overlap(
    chunks: &[ChunkRef],
    targets: &[Target],
    removals: &[Range<usize>],
    tstart: usize,
    trailer_gone: bool,
) -> Result<(), StripError> {
    // The signature is never removed.
    let mut kept: Vec<Range<usize>> = Vec::new();
    kept.push(0..8);
    let removed_ranges: HashSet<(usize, usize)> = targets
        .iter()
        .map(|t| (t.range.start, t.range.end))
        .collect();
    for c in chunks {
        if trailer_gone && c.range_start >= tstart {
            continue;
        }
        let r = c.range();
        if !removed_ranges.contains(&(r.start, r.end)) {
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

    fn chunk(ty: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut v = u32::try_from(data.len()).unwrap().to_be_bytes().to_vec();
        v.extend_from_slice(ty);
        v.extend_from_slice(data);
        v.extend_from_slice(&crc32_parts(ty, data).to_be_bytes());
        v
    }

    fn png(parts: &[Vec<u8>]) -> Vec<u8> {
        let mut v = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        let mut ihdr = vec![0, 0, 0, 1, 0, 0, 0, 1];
        ihdr.extend_from_slice(&[8, 0, 0, 0, 0]);
        v.extend_from_slice(&chunk(b"IHDR", &ihdr));
        for p in parts {
            v.extend_from_slice(p);
        }
        v
    }

    #[test]
    fn the_carrier_rule_follows_clause_4() {
        for preset in StripPolicy::PRESETS {
            assert!(removes_carriers(preset.spec()), "{preset:?}");
        }
        let one = |c: Category| StripSpec::none().include(c);
        for c in [
            Category::Gps,
            Category::Software,
            Category::Xmp,
            Category::Iptc,
            Category::Thumbnail,
            Category::OtherSegment,
        ] {
            assert!(removes_carriers(one(c)), "{c:?}");
        }
        for c in [Category::Icc, Category::Jumbf, Category::EmbeddedImage] {
            assert!(!removes_carriers(one(c)), "{c:?}");
        }
        assert!(!removes_carriers(StripSpec::none()));
    }

    #[test]
    fn the_trailer_scan_reports_a_cap() {
        let mut v = Vec::new();
        for _ in 0..MAX_PNG_TRAILER_CHUNKS {
            v.extend_from_slice(&chunk(b"abCd", b""));
        }
        let (framed, stop, more) = trailer_chunks(&v, 0);
        assert_eq!(framed.len(), MAX_PNG_TRAILER_CHUNKS);
        assert_eq!(stop, v.len());
        assert!(!more);
        v.extend_from_slice(&chunk(b"abCd", b""));
        assert!(trailer_chunks(&v, 0).2);
        // The first chunk that does not fit ends the scan quietly.
        let (framed, stop, more) = trailer_chunks(b"not a chunk at all", 0);
        assert!(framed.is_empty() && stop == 0 && !more);
    }

    #[test]
    fn a_text_chunk_the_reader_cannot_locate_is_still_free_text() {
        // No NUL after the keyword: no region, no finding, but it is text.
        let bad = chunk(b"tEXt", b"nonul");
        let input = png(&[
            bad.clone(),
            chunk(b"IDAT", &[0x78, 1, 0x63, 0x60, 0, 0]),
            chunk(b"IEND", &[]),
        ]);
        let spec = StripSpec::none().include(Category::FreeText);
        let planned = plan(&input, spec).unwrap();
        assert_eq!(planned.removals.len(), 1);
        assert_eq!(planned.removals[0].kind, RemovalKind::Text);
        assert_eq!(planned.removals[0].len, bad.len());
        // And it stays when the policy has no `FreeText`.
        let spec = StripSpec::none().include(Category::Gps);
        assert!(plan(&input, spec).unwrap().plan.is_empty());
    }

    #[test]
    fn a_chunk_past_the_end_is_refused_with_its_offset() {
        let first = chunk(b"tEXt", b"Author\0x");
        let mut input = png(std::slice::from_ref(&first));
        input.extend_from_slice(&[0, 0, 3, 0xE8, b't', b'E', b'X', b't', 1, 2]);
        let at = 8 + 25 + first.len();
        let Err(StripError::Unstrippable { reason, offset }) =
            plan(&input, StripPolicy::Gps.spec())
        else {
            panic!("expected a refusal");
        };
        assert_eq!(reason, UnstrippableReason::ChunkPastEnd);
        assert_eq!(offset, at);
    }
}
