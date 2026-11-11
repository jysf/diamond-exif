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
use crate::policy::StripSpec;
use crate::tiff::{Container, IfdKind};

use super::exif::protected_ranges;
use super::jpeg::plan;
use super::tiff;
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
        entries(input, spec, &before, &after, false)?;
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
    entries(input, spec, &before, &after, true)?;
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
    tiff: bool,
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
        for (place, key) in items.iter().flatten() {
            if key.1 == IfdKind::Ifd1 {
                gone.insert(*place);
            }
        }
    }
    if spec.contains(Category::Gps) {
        if tiff {
            let container = Container::parse(input).map_err(|_| Check::Entries)?;
            gps_pointers(&container, None, &mut gone)?;
        } else {
            let meta = jpeg::read(input).map_err(|_| Check::Entries)?;
            for (b, block) in meta.exif_blocks().iter().enumerate() {
                let Ok(container) = block.container() else {
                    continue;
                };
                gps_pointers(container, Some(b), &mut gone)?;
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
