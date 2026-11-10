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
//! walks and has no new kind of issue; (e) stripping the output again would
//! change nothing.

use core::mem::{discriminant, Discriminant};

use crate::audit::{audit, Category, ItemKind, RegionKind};
use crate::error::Error;
use crate::jpeg::{self, Jpeg, Region};
use crate::policy::StripSpec;

use super::jpeg::plan;
use super::Check;

/// Check `output` against `input` for the categories `spec`.
pub(super) fn verify(input: &[u8], output: &[u8], spec: StripSpec) -> Result<(), Check> {
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

    // (e) Idempotence: the plan for the output is empty.
    match plan(output, spec) {
        Ok(p) if p.plan.is_empty() => Ok(()),
        _ => Err(Check::Idempotent),
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
