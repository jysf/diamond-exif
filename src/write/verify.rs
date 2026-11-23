//! The self-check (DEC-013 part 7): the writer audits its own output.
//!
//! [`verify`] is implemented from the audit and the JPEG walk, not from the
//! plan: it takes the input, the candidate output and the policy's categories
//! and nothing else. The first failing check is reported and no bytes leave
//! [`super::strip`].
//!
//! The checks run in this order: (a) the output has no finding in a category
//! the policy removes; the Multi-Picture fields (kept images are the same bytes
//! and the primary's size field still describes the primary); (b) every
//! category the policy keeps has the same findings by kind (kept `Trailer`
//! regions by their total byte count, because removing the images between two
//! pieces joins them); (c) the scan region is byte-identical; (d) the output
//! walks and has no new kind of issue; (g) the Exif tag inventory is the
//! input's minus the deleted entries; (h) every zeroed range reads zero; (e)
//! stripping the output again would change nothing.

use core::mem::{discriminant, Discriminant};
use std::collections::{BTreeMap, BTreeSet};

use crate::audit::{audit, Category, FileAudit, ItemKind, RegionKind};
use crate::error::Error;
use crate::jpeg::{self, Jpeg, Region};
use crate::png;
use crate::policy::StripSpec;
use crate::tiff::{Container, IfdKind};
use crate::webp::{self, ChunkKind};

use super::exif::protected_ranges;
use super::jpeg::plan;
use super::png as png_plan;
use super::tiff;
use super::webp as webp_plan;
use super::Check;

/// Check `output` against `input` for the categories `spec`. `scrubs` are the
/// `(offset, length)` ranges, in output coordinates, that the plan zeroed.
pub(super) fn verify(
    input: &[u8],
    output: &[u8],
    spec: StripSpec,
    scrubs: &[(usize, usize)],
) -> Result<(), Check> {
    let before = audit(input);
    let after = audit(output);

    // (a) Nothing the policy removes is left.
    if after.findings().iter().any(|f| spec.contains(f.category)) {
        return Err(Check::Preview);
    }
    mpf_fields(input, output, spec)?;

    // (b) Everything it keeps is still there. When every Exif category is
    // removed the Exif segments are gone whole, and with them any finding the
    // audit made inside them (an Exif IFD can hold `Icc`-category tags).
    let exif_gone = [
        Category::Gps,
        Category::DeviceId,
        Category::Authorship,
        Category::FreeText,
        Category::Timestamps,
        Category::CameraModel,
        Category::Thumbnail,
        Category::MakerNote,
        Category::Software,
    ]
    .iter()
    .all(|c| spec.contains(*c));
    let (kept_before, trailer_before) = kept_findings(&before, spec, exif_gone);
    let (kept_after, trailer_after) = kept_findings(&after, spec, false);
    if kept_before != kept_after || trailer_before != trailer_after {
        return Err(Check::Kept);
    }

    // (c) The scan region is byte for byte the same.
    let (jin, jout) = match (Jpeg::parse(input), Jpeg::parse(output)) {
        (Ok(a), Ok(b)) => (a, b),
        _ => return Err(Check::Walk),
    };
    if jin.scan_region() != jout.scan_region() {
        return Err(Check::Scan);
    }

    // (d) The output is a walkable JPEG with no new kind of issue.
    let kinds = |data: &[u8]| -> Option<Vec<Discriminant<Error>>> {
        let meta = jpeg::read(data).ok()?;
        Some(
            meta.jpeg_issues()
                .iter()
                .map(|i| discriminant(i.error()))
                .collect(),
        )
    };
    match (kinds(input), kinds(output)) {
        (Some(old), Some(new)) if new.iter().all(|k| old.contains(k)) => {}
        _ => return Err(Check::Walk),
    }

    // (g) The Exif tag inventory is the input's minus the deleted entries. When
    // every Exif category is removed the segments are gone whole (nothing to
    // compare).
    if !exif_gone {
        entries(input, spec, &before, &after, Source::Jpeg)?;
    }

    // (h) Every range the plan zeroed reads zero in the output.
    for &(offset, len) in scrubs {
        let zero = offset
            .checked_add(len)
            .and_then(|end| output.get(offset..end))
            .is_some_and(|bytes| bytes.iter().all(|b| *b == 0));
        if !zero {
            return Err(Check::Scrubbed);
        }
    }

    // (e) Idempotence: the plan for the output is empty.
    match plan(output, spec) {
        Ok(p) if p.plan.is_empty() => Ok(()),
        _ => Err(Check::Idempotent),
    }
}

/// The GPS pointer entry of every GPS IFD of `container`, added to `gone`.
fn gps_pointers(
    container: &Container<'_>,
    block: Option<usize>,
    gone: &mut BTreeSet<Place>,
) -> Result<(), Check> {
    for ifd in container.ifds().iter().filter(|i| i.kind() == IfdKind::Gps) {
        let pointer = ifd
            .parent()
            .and_then(|p| {
                let holder = container.ifds().get(p)?;
                let n = holder.entries().iter().position(|e| e.tag() == 0x8825)?;
                Some((p, n))
            })
            .ok_or(Check::Entries)?;
        gone.insert((block, pointer.0, pointer.1));
    }
    Ok(())
}

/// The self-check of a bare TIFF (SPEC-022): the JPEG checks with the scan
/// region replaced by the data ranges. (a) no finding of a removed category;
/// (b) kept categories unchanged; (c) same length, and every strip, tile, JPEG
/// data range of the OUTPUT's walk is byte-identical to the input; (d) the
/// output walks with no new kind of issue; (g) the inventory is the input's
/// minus the deleted entries; (h) zeroed ranges read zero; (e) re-stripping the
/// output changes nothing.
pub(super) fn verify_tiff(
    input: &[u8],
    output: &[u8],
    spec: StripSpec,
    scrubs: &[(usize, usize)],
) -> Result<(), Check> {
    let before = audit(input);
    let after = audit(output);
    if after.findings().iter().any(|f| spec.contains(f.category)) {
        return Err(Check::Preview);
    }
    let (kept_before, _) = kept_findings(&before, spec, false);
    let (kept_after, _) = kept_findings(&after, spec, false);
    if kept_before != kept_after {
        return Err(Check::Kept);
    }
    // (c) Same length and the image data untouched.
    if input.len() != output.len() {
        return Err(Check::Scan);
    }
    // The 8-byte header (byte order, magic, IFD0 offset) is never edited.
    if input.get(..8) != output.get(..8) {
        return Err(Check::Scan);
    }
    let (Ok(cin), Ok(cout)) = (Container::parse(input), Container::parse(output)) else {
        return Err(Check::Walk);
    };
    let protected = protected_ranges(output, &cout).map_err(|_| Check::Walk)?;
    for r in &protected {
        if input.get(r.clone()) != output.get(r.clone()) {
            return Err(Check::Scan);
        }
    }
    // (d) No new kind of issue.
    let kinds = |c: &Container<'_>| -> Vec<Discriminant<Error>> {
        c.issues().iter().map(|i| discriminant(i.error())).collect()
    };
    let old = kinds(&cin);
    if !kinds(&cout).iter().all(|k| old.contains(k)) {
        return Err(Check::Walk);
    }
    entries(input, spec, &before, &after, Source::Tiff)?;
    for &(offset, len) in scrubs {
        let zero = offset
            .checked_add(len)
            .and_then(|end| output.get(offset..end))
            .is_some_and(|bytes| bytes.iter().all(|b| *b == 0));
        if !zero {
            return Err(Check::Scrubbed);
        }
    }
    match tiff::plan(output, spec) {
        Ok(p) if p.plan.is_empty() => Ok(()),
        _ => Err(Check::Idempotent),
    }
}

/// The chunk types whose chunks a PNG strip may remove or edit: `eXIf`, text,
/// `iCCP` and everything the reader locates by type. Every other chunk (image
/// header, palette, image data, structure, unknown types) must come out
/// byte for byte as it went in.
fn removable_type(ty: &[u8; 4]) -> bool {
    matches!(ty, b"eXIf" | b"tEXt" | b"zTXt" | b"iTXt" | b"iCCP") || png::chunk_class(*ty).is_some()
}

/// Every chunk of a PNG as `(type, bytes, crc ok)`: the walked ones, then the
/// ones framed after `IEND` (when `trailer`), and the bytes after the last of
/// them that frames.
type PngChunks<'a> = (Vec<([u8; 4], &'a [u8], bool)>, &'a [u8]);

fn png_chunks(data: &[u8], trailer: bool) -> Option<PngChunks<'_>> {
    let meta = png::read(data).ok()?;
    let mut out: Vec<([u8; 4], &[u8], bool)> = meta
        .png()
        .chunks()
        .iter()
        .map(|c| Some((c.chunk_type(), data.get(c.range())?, c.crc_ok())))
        .collect::<Option<_>>()?;
    let start = png_plan::trailer_start(data, &meta);
    if !trailer {
        return Some((out, &[]));
    }
    let (framed, stop, _) = png_plan::trailer_chunks(data, start);
    for (a, b, ty) in framed {
        out.push((ty, data.get(a..b)?, true));
    }
    let tail = data.get(stop.max(start)..)?;
    Some((out, tail))
}

/// The self-check of a PNG strip (SPEC-037, DEC-021 clause 6). Like the JPEG
/// one it reads the input, the output and the policy; the byte ranges the plan
/// removed are used only to account for the bytes after `IEND`.
///
/// (a) no finding in a removed category, and none of the opaque carriers the
/// policy removes (a raw profile or carrier chunk has category `OtherSegment`,
/// which most presets do not remove, yet every preset removes it: DEC-021
/// clause 4); (b) the kept categories have the same findings (the kept bytes
/// after `IEND` by count, less what was removed from them); (c) every chunk
/// that is not removable is byte-identical and in order, the removable ones
/// that are left are the input's (an `eXIf` may differ in its bytes, never in
/// its length) in order, no chunk has a worse CRC than it had, and the kept
/// non-chunk tail is the input's; (d) the output walks with no new kind of
/// issue; (g) the Exif entry inventory is the input's minus the deleted
/// entries; (h) zeroed ranges read zero; (e) stripping again changes nothing.
pub(super) fn verify_png(
    input: &[u8],
    output: &[u8],
    spec: StripSpec,
    scrubs: &[(usize, usize)],
    removed: &[core::ops::Range<usize>],
) -> Result<(), Check> {
    let before = audit(input);
    let after = audit(output);
    let carriers = png_plan::removes_carriers(spec);
    let is_carrier =
        |kind: RegionKind| matches!(kind, RegionKind::RawProfile | RegionKind::Carrier);

    // (a) Nothing the policy removes is left.
    if after.findings().iter().any(|f| spec.contains(f.category)) {
        return Err(Check::Preview);
    }
    if carriers
        && after
            .items()
            .iter()
            .any(|i| matches!(i.kind(), ItemKind::Region { kind, .. } if is_carrier(kind)))
    {
        return Err(Check::Preview);
    }

    // (b) Everything it keeps is still there.
    let exif_gone = exif_set().iter().all(|c| spec.contains(*c));
    let (mut kept_before, trailer_before) = kept_findings(&before, spec, exif_gone);
    let (mut kept_after, trailer_after) = kept_findings(&after, spec, false);
    if carriers {
        // Removed although their category stays: leave them out of both sides.
        kept_before.retain(|k| k.1 != "RawProfile" && k.1 != "Carrier");
        kept_after.retain(|k| k.1 != "RawProfile" && k.1 != "Carrier");
    }
    let trailer_removed = match png::read(input) {
        Ok(meta) => {
            let start = png_plan::trailer_start(input, &meta);
            super::plan::removed_in(removed, start..input.len())
        }
        Err(_) => return Err(Check::Walk),
    };
    if kept_before != kept_after || trailer_before.saturating_sub(trailer_removed) != trailer_after
    {
        return Err(Check::Kept);
    }

    // (c) The chunks the strip does not target are untouched.
    // With `OtherSegment` the bytes after `IEND` go whole (check (a) shows none
    // is left), so only the walked chunks are compared.
    let trailer_kept = !spec.contains(Category::OtherSegment);
    let (Some((cin, tail_in)), Some((cout, tail_out))) = (
        png_chunks(input, trailer_kept),
        png_chunks(output, trailer_kept),
    ) else {
        return Err(Check::Walk);
    };
    let fixed = |list: &[([u8; 4], &[u8], bool)]| -> Vec<Vec<u8>> {
        list.iter()
            .filter(|c| !removable_type(&c.0))
            .map(|c| c.1.to_vec())
            .collect()
    };
    if fixed(&cin) != fixed(&cout) {
        return Err(Check::Scan);
    }
    // The removable chunks that are left are a subsequence of the input's.
    let mut at = 0usize;
    for (ty, bytes, _) in cout.iter().filter(|c| removable_type(&c.0)) {
        let found = cin.iter().skip(at).position(|(t, b, _)| {
            removable_type(t)
                && t == ty
                && (b == bytes || (ty == b"eXIf" && b.len() == bytes.len()))
        });
        match found {
            Some(n) => at = at.saturating_add(n).saturating_add(1),
            None => return Err(Check::Scan),
        }
    }
    // No chunk is left with a worse CRC than it had.
    let bad_crc = |list: &[([u8; 4], &[u8], bool)]| list.iter().filter(|c| !c.2).count();
    if bad_crc(&cout) > bad_crc(&cin) {
        return Err(Check::Scan);
    }
    if trailer_kept && tail_in != tail_out {
        return Err(Check::Scan);
    }

    // (d) The output walks, with no new kind of issue.
    let kinds = |data: &[u8]| -> Option<Vec<Discriminant<Error>>> {
        let meta = png::read(data).ok()?;
        Some(
            meta.png_issues()
                .iter()
                .map(|i| discriminant(i.error()))
                .collect(),
        )
    };
    match (kinds(input), kinds(output)) {
        (Some(old), Some(new)) if new.iter().all(|k| old.contains(k)) => {}
        _ => return Err(Check::Walk),
    }

    // (g) The Exif tag inventory is the input's minus the deleted entries.
    if !exif_gone {
        entries(input, spec, &before, &after, Source::Png)?;
    }

    // (h) Every range the plan zeroed reads zero in the output.
    for &(offset, len) in scrubs {
        let zero = offset
            .checked_add(len)
            .and_then(|end| output.get(offset..end))
            .is_some_and(|bytes| bytes.iter().all(|b| *b == 0));
        if !zero {
            return Err(Check::Scrubbed);
        }
    }

    // (e) Idempotence: the plan for the output is empty.
    match png_plan::plan(output, spec) {
        Ok(p) if p.plan.is_empty() => Ok(()),
        _ => Err(Check::Idempotent),
    }
}

/// One chunk of a WebP as compared: its kind, its code, the whole chunk (frame,
/// data and pad byte) and its data alone.
type WebpChunk<'a> = (ChunkKind, [u8; 4], &'a [u8], &'a [u8]);

/// Every chunk of a WebP and the bytes after the last chunk the walk framed.
type WebpChunks<'a> = (Vec<WebpChunk<'a>>, &'a [u8]);

fn webp_chunks(data: &[u8]) -> Option<WebpChunks<'_>> {
    let walk = webp::Webp::parse(data).ok()?;
    let list: Vec<WebpChunk<'_>> = walk
        .chunks()
        .iter()
        .map(|c| Some((c.kind(), c.fourcc(), data.get(c.range())?, c.data())))
        .collect::<Option<_>>()?;
    let end = walk.chunks().last().map_or(WEBP_HEADER, |c| c.range().end);
    Some((list, data.get(end..)?))
}

/// Bytes of the RIFF header: `RIFF`, the size field, `WEBP`.
const WEBP_HEADER: usize = 12;

/// Bytes of a `VP8X` chunk's data that hold its flags and canvas size.
const WEBP_VP8X_DATA: usize = 10;

/// How many chunks of `kind` a walk framed.
fn count_kind(list: &[WebpChunk<'_>], kind: ChunkKind) -> usize {
    list.iter().filter(|c| c.0 == kind).count()
}

/// Whether a WebP chunk is one the strip may remove or edit.
fn webp_removable(c: &WebpChunk<'_>) -> bool {
    webp_plan::removable(c.0, c.1, c.3)
}

/// The self-check of a WebP strip (SPEC-038, DEC-022 clause 9). Like the PNG
/// one it reads the input, the output and the policy; the byte ranges the plan
/// removed are used only to compute what the RIFF size must now say.
///
/// (a) no finding in a removed category, and none of the carrier chunks the
/// policy removes (a `Carrier` region has category `OtherSegment`, which most
/// presets do not remove, yet every preset removes it: DEC-022 clause 4); (b)
/// the kept categories have the same findings; (c) the header words, every
/// chunk that is not removable (the image chunks included) and the bytes after
/// the last chunk are byte-identical and in order, `VP8X` apart from the three
/// flag bits the strip may clear, and the removable chunks that are left are
/// the input's in order (an `EXIF` may differ in its bytes, never in its
/// length); (d) the output walks with no new kind of issue; (f) the declared
/// RIFF size is the input's less the bytes removed from inside the declared
/// range, and the `VP8X` flags are the input's with the bit of each kind of
/// metadata chunk that is now absent (and was present) cleared; (g) the Exif
/// entry inventory is the input's minus the deleted entries; (h) zeroed ranges
/// read zero; (e) stripping again changes nothing.
pub(super) fn verify_webp(
    input: &[u8],
    output: &[u8],
    spec: StripSpec,
    scrubs: &[(usize, usize)],
    removed: &[core::ops::Range<usize>],
) -> Result<(), Check> {
    let before = audit(input);
    let after = audit(output);
    let carriers = png_plan::removes_carriers(spec);

    // (a) Nothing the policy removes is left.
    if after.findings().iter().any(|f| spec.contains(f.category)) {
        return Err(Check::Preview);
    }
    if carriers
        && after.items().iter().any(
            |i| matches!(i.kind(), ItemKind::Region { kind, .. } if kind == RegionKind::Carrier),
        )
    {
        return Err(Check::Preview);
    }

    // (b) Everything it keeps is still there.
    let exif_gone = exif_set().iter().all(|c| spec.contains(*c));
    let (mut kept_before, _) = kept_findings(&before, spec, exif_gone);
    let (mut kept_after, _) = kept_findings(&after, spec, false);
    if carriers {
        // Removed although their category stays: leave them out of both sides.
        kept_before.retain(|k| k.1 != "Carrier");
        kept_after.retain(|k| k.1 != "Carrier");
    }
    if kept_before != kept_after {
        return Err(Check::Kept);
    }

    // (c) The chunks the strip does not target are untouched.
    let (Some((cin, tail_in)), Some((cout, tail_out))) = (webp_chunks(input), webp_chunks(output))
    else {
        return Err(Check::Walk);
    };
    let words = |d: &[u8]| {
        (
            d.get(..4).map(<[u8]>::to_vec),
            d.get(8..WEBP_HEADER).map(<[u8]>::to_vec),
        )
    };
    if words(input) != words(output) {
        return Err(Check::Scan);
    }
    // The `VP8X` the reader reports: the first one, when it has a full header.
    let reported = |list: &[WebpChunk<'_>]| {
        list.iter()
            .position(|c| c.0 == ChunkKind::Vp8x)
            .filter(|&i| list.get(i).is_some_and(|c| c.3.len() >= WEBP_VP8X_DATA))
    };
    let fixed = |list: &[WebpChunk<'_>]| -> Vec<Vec<u8>> {
        let flagged = reported(list);
        list.iter()
            .enumerate()
            .filter(|(_, c)| !webp_removable(c))
            .map(|(i, c)| {
                let mut bytes = c.2.to_vec();
                if Some(i) == flagged {
                    // The flag byte is the first byte of the data, after the
                    // 8-byte frame; the three bits the strip may clear are
                    // compared in check (f).
                    if let Some(b) = bytes.get_mut(8) {
                        *b &= !(webp_plan::FLAG_ICC | webp_plan::FLAG_EXIF | webp_plan::FLAG_XMP);
                    }
                }
                bytes
            })
            .collect()
    };
    if fixed(&cin) != fixed(&cout) || tail_in != tail_out {
        return Err(Check::Scan);
    }
    // The removable chunks that are left are a subsequence of the input's.
    let mut at = 0usize;
    for out_chunk in cout.iter().filter(|c| webp_removable(c)) {
        let found = cin.iter().skip(at).position(|c| {
            webp_removable(c)
                && c.0 == out_chunk.0
                && c.1 == out_chunk.1
                && (c.2 == out_chunk.2
                    || (out_chunk.0 == ChunkKind::Exif && c.2.len() == out_chunk.2.len()))
        });
        match found {
            Some(n) => at = at.saturating_add(n).saturating_add(1),
            None => return Err(Check::Scan),
        }
    }

    // (d) The output walks, with no new kind of issue.
    let kinds = |data: &[u8]| -> Option<Vec<Discriminant<Error>>> {
        let meta = webp::read(data).ok()?;
        Some(
            meta.webp_issues()
                .iter()
                .map(|i| discriminant(i.error()))
                .collect(),
        )
    };
    match (kinds(input), kinds(output)) {
        (Some(old), Some(new)) if new.iter().all(|k| old.contains(k)) => {}
        _ => return Err(Check::Walk),
    }

    // (f) The RIFF size and the `VP8X` flags still say what the file holds.
    let (Ok(win), Ok(wout)) = (webp::Webp::parse(input), webp::Webp::parse(output)) else {
        return Err(Check::Walk);
    };
    let declared = win.declared_size();
    let window_end = usize::try_from(declared)
        .ok()
        .and_then(|d| d.checked_add(8))
        .unwrap_or(usize::MAX)
        .min(input.len());
    let inside = super::plan::removed_in(removed, 8..window_end);
    let expected_size = u32::try_from(inside)
        .ok()
        .and_then(|n| declared.checked_sub(n));
    if expected_size != Some(wout.declared_size()) {
        return Err(Check::MpfFields);
    }
    let expected_flags = win.vp8x().map(|v| {
        let mut flags = v.flags();
        for (bit, kind) in [
            (webp_plan::FLAG_ICC, ChunkKind::Iccp),
            (webp_plan::FLAG_EXIF, ChunkKind::Exif),
            (webp_plan::FLAG_XMP, ChunkKind::Xmp),
        ] {
            if count_kind(&cin, kind) > 0 && count_kind(&cout, kind) == 0 {
                flags &= !bit;
            }
        }
        flags
    });
    if expected_flags != wout.vp8x().map(|v| v.flags()) {
        return Err(Check::MpfFields);
    }

    // (g) The Exif tag inventory is the input's minus the deleted entries.
    if !exif_gone {
        entries(input, spec, &before, &after, Source::Webp)?;
    }

    // (h) Every range the plan zeroed reads zero in the output.
    for &(offset, len) in scrubs {
        let zero = offset
            .checked_add(len)
            .and_then(|end| output.get(offset..end))
            .is_some_and(|bytes| bytes.iter().all(|b| *b == 0));
        if !zero {
            return Err(Check::Scrubbed);
        }
    }

    // (e) Idempotence: the plan for the output is empty.
    match webp_plan::plan(output, spec) {
        Ok(p) if p.plan.is_empty() => Ok(()),
        _ => Err(Check::Idempotent),
    }
}

/// The nine categories an Exif block can hold (DEC-013 part 4), by hand.
fn exif_set() -> [Category; 9] {
    [
        Category::Gps,
        Category::DeviceId,
        Category::Authorship,
        Category::FreeText,
        Category::Timestamps,
        Category::CameraModel,
        Category::Thumbnail,
        Category::MakerNote,
        Category::Software,
    ]
}

/// Where the Exif blocks of a file live, for the one place that needs to read
/// them again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    Jpeg,
    Tiff,
    Png,
    Webp,
}

/// One tag of the inventory, as compared: its block, the IFD kind, the tag, the
/// declared type and count, and the raw value bytes.
type TagKey<'a> = (Option<usize>, IfdKind, u16, u16, u32, Option<&'a [u8]>);

/// A tag item's place: the block, the IFD's index in the walk, and the entry's
/// ordinal inside that IFD.
type Place = (Option<usize>, usize, usize);

/// Every tag item of an audit with its place, in item order.
fn tag_items<'a>(audited: &FileAudit<'a>) -> Vec<Option<(Place, TagKey<'a>)>> {
    let mut seen: BTreeMap<(Option<usize>, usize), usize> = BTreeMap::new();
    audited
        .items()
        .iter()
        .map(|item| {
            let ItemKind::Tag {
                ifd,
                ifd_index,
                tag,
                field_type,
                count,
            } = item.kind()
            else {
                return None;
            };
            let block = item.origin().block;
            let n = seen.entry((block, ifd_index)).or_insert(0);
            let place = (block, ifd_index, *n);
            *n = n.saturating_add(1);
            Some((place, (block, ifd, tag, field_type, count, item.value())))
        })
        .collect()
}

/// (g) The output's tag inventory equals the input's minus exactly what the
/// policy deletes, in order, each with the same type, count and value bytes.
/// What is deleted is derived here from the input's audit (the findings in the
/// policy's categories, IFD1 when the thumbnail goes, the GPS pointer entry
/// when GPS goes), not from the plan.
fn entries(
    input: &[u8],
    spec: StripSpec,
    before: &FileAudit<'_>,
    after: &FileAudit<'_>,
    source: Source,
) -> Result<(), Check> {
    let items = tag_items(before);
    let mut gone: BTreeSet<Place> = BTreeSet::new();
    for f in before.findings() {
        if !spec.contains(f.category) {
            continue;
        }
        if let Some((place, _)) = f.item.and_then(|i| items.get(i)).and_then(Option::as_ref) {
            gone.insert(*place);
        }
    }
    if spec.contains(Category::Thumbnail) {
        // IFD1 goes only in a block where the audit calls it a thumbnail: a
        // bare TIFF's page IFD1 is kept (SPEC-023).
        let thumb_blocks: BTreeSet<Option<usize>> = before
            .findings()
            .iter()
            .filter(|f| f.category == Category::Thumbnail)
            .filter_map(|f| f.item.and_then(|i| items.get(i)).and_then(Option::as_ref))
            .filter(|(_, key)| key.1 == IfdKind::Ifd1)
            .map(|(place, _)| place.0)
            .collect();
        for (place, key) in items.iter().flatten() {
            if key.1 == IfdKind::Ifd1 && thumb_blocks.contains(&place.0) {
                gone.insert(*place);
            }
        }
    }
    if spec.contains(Category::Gps) {
        match source {
            Source::Tiff => {
                let container = Container::parse(input).map_err(|_| Check::Entries)?;
                gps_pointers(&container, None, &mut gone)?;
            }
            Source::Jpeg => {
                let meta = jpeg::read(input).map_err(|_| Check::Entries)?;
                for (b, block) in meta.exif_blocks().iter().enumerate() {
                    let Ok(container) = block.container() else {
                        continue;
                    };
                    gps_pointers(container, Some(b), &mut gone)?;
                }
            }
            Source::Png => {
                let meta = png::read(input).map_err(|_| Check::Entries)?;
                for (b, block) in meta.exif_blocks().iter().enumerate() {
                    let Ok(container) = block.container() else {
                        continue;
                    };
                    gps_pointers(container, Some(b), &mut gone)?;
                }
            }
            Source::Webp => {
                let meta = webp::read(input).map_err(|_| Check::Entries)?;
                for (b, block) in meta.exif_blocks().iter().enumerate() {
                    let Ok(container) = block.container() else {
                        continue;
                    };
                    gps_pointers(container, Some(b), &mut gone)?;
                }
            }
        }
    }
    let expected: Vec<TagKey<'_>> = items
        .iter()
        .flatten()
        .filter(|(place, _)| !gone.contains(place))
        .map(|(_, key)| *key)
        .collect();
    let actual: Vec<TagKey<'_>> = tag_items(after)
        .into_iter()
        .flatten()
        .map(|(_, key)| key)
        .collect();
    if expected == actual {
        Ok(())
    } else {
        Err(Check::Entries)
    }
}

/// The findings of the categories outside `spec` as sorted keys, and the total
/// byte count of the kept `Trailer` regions. `skip_exif` leaves out findings
/// made inside Exif blocks.
fn kept_findings(
    audited: &crate::audit::FileAudit<'_>,
    spec: StripSpec,
    skip_exif: bool,
) -> (Vec<(Category, String)>, usize) {
    let mut keys = Vec::new();
    let mut trailer = 0usize;
    for f in audited.findings() {
        if spec.contains(f.category) || (skip_exif && f.at.origin().block.is_some()) {
            continue;
        }
        match f.at.kind() {
            ItemKind::Region {
                kind: RegionKind::Trailer,
                len,
                ..
            } => trailer = trailer.saturating_add(len),
            ItemKind::Region { kind, .. } => keys.push((f.category, format!("{kind:?}"))),
            other => {
                // A tag: its IFD kind and id (the value and the count are not
                // compared; Exif is kept whole or not at all).
                let key = match other {
                    ItemKind::Tag { ifd, tag, .. } => format!("{ifd:?}/{tag}"),
                    unknown => format!("{unknown:?}"),
                };
                keys.push((f.category, key));
            }
        }
    }
    keys.sort_by(|a, b| (a.0 as u8, &a.1).cmp(&(b.0 as u8, &b.1)));
    (keys, trailer)
}

/// The Multi-Picture fields of the output (checked when the images stay): the
/// kept images are the same bytes in the same order, and when the input's
/// primary size field was right, the output's is right too.
fn mpf_fields(input: &[u8], output: &[u8], spec: StripSpec) -> Result<(), Check> {
    if spec.contains(Category::EmbeddedImage) {
        return Ok(());
    }
    let (Ok(min), Ok(mout)) = (jpeg::read(input), jpeg::read(output)) else {
        return Ok(());
    };
    let Some(ix_in) = min.mpf() else {
        return Ok(());
    };
    let Some(ix_out) = mout.mpf() else {
        return Err(Check::MpfFields);
    };
    let images = |m: &jpeg::JpegMetadata<'_>| -> Vec<Vec<u8>> {
        m.regions()
            .iter()
            .filter_map(|r| match r {
                Region::EmbeddedImage { data, .. } => Some(data.to_vec()),
                _ => None,
            })
            .collect()
    };
    if images(&min) != images(&mout) || ix_in.entries().len() != ix_out.entries().len() {
        return Err(Check::MpfFields);
    }
    let primary = |m: &jpeg::JpegMetadata<'_>, ix: &jpeg::MpfIndex, len: usize| {
        let end = m.jpeg().eoi_offset().map_or(len, |e| e.saturating_add(2));
        ix.entries()
            .first()
            .map(|e| (usize::try_from(e.size()).ok() == Some(end), e.size()))
    };
    if let (Some((true, _)), Some((ok, _))) = (
        primary(&min, ix_in, input.len()),
        primary(&mout, ix_out, output.len()),
    ) {
        if !ok {
            return Err(Check::MpfFields);
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "verify_tests.rs"]
mod tests;
