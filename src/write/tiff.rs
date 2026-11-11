//! The TIFF and DNG planner (SPEC-022, DEC-013 amendment C).
//!
//! A bare classic TIFF (a DNG is one) is stripped IN PLACE: the file keeps its
//! length, nothing moves, and the image data (strips, tiles, raw and preview
//! SubIFD data, the JPEG bytes of a kept page) is never touched. The only edits
//! are the ones the Exif planner makes on a blob: same-length IFD table
//! rewrites and zero-byte scrubs. The blob is the whole file, so a blob offset
//! is an input offset.
//!
//! What is never touched is enforced twice. The planner refuses
//! ([`UneditableReason::SharedBytes`](super::UneditableReason)) when a scrub or a
//! table rewrite would reach bytes that a kept entry points at through the
//! strip, tile, free-block, JPEG-thumbnail or JPEG-table tags, in any IFD of the
//! walk (IFD chain pages and SubIFDs included); and the self-check requires
//! those ranges, computed again over the output, to be byte-identical. The strip
//! and tile offset and byte-count entries are structural, not privacy: no
//! category names them, so no policy deletes them.
//!
//! IFD1 of a TIFF is a thumbnail only when it is one the audit calls so and the
//! planner can locate (a JPEG pair 0x0201/0x0202 and no strips). An IFD1 that
//! holds strips or tiles is a page: the audit still reports it under
//! `Thumbnail`, so a policy that removes `Thumbnail` is refused
//! (`ThumbnailNotLocated`, the strips form), never silently left and never cut.
//!
//! Provenance (DEC-003, class 1): net-new code over this crate's own planner.
//! No ExifTool source is read.

use crate::audit::audit;
use crate::policy::StripSpec;
use crate::tiff::Container;

use super::exif::{mode, plan_block, select, Mode};
use super::jpeg::Planned;
use super::{EntryRecord, StripError, UneditableReason};

/// Plan the strip of the bare TIFF `data` for the categories `spec`.
pub(super) fn plan(data: &[u8], spec: StripSpec) -> Result<Planned, StripError> {
    let refuse =
        |reason: UneditableReason, offset: usize| StripError::Uneditable { reason, offset };
    let container =
        Container::parse(data).map_err(|_| refuse(UneditableReason::BlockNotClean, 0))?;
    if let Some(issue) = container.issues().first() {
        let at = issue
            .offset()
            .and_then(|o| usize::try_from(o).ok())
            .unwrap_or(0);
        return Err(refuse(UneditableReason::BlockNotClean, at));
    }
    let audited = audit(data);
    if audited.health().inventory_truncated {
        return Err(refuse(UneditableReason::InventoryTruncated, 0));
    }
    // A policy with no category an Exif entry can have leaves the file as is.
    // The self-check still runs on it.
    if mode(spec) == Mode::Untouched {
        return Ok(Planned::empty());
    }
    let selection = select(&audited, None, &container, spec)
        .ok_or_else(|| refuse(UneditableReason::BlockNotClean, 0))?;
    let block = plan_block(data, &container, &selection).map_err(|r| StripError::Uneditable {
        reason: r.reason,
        offset: r.at,
    })?;
    let mut planned = Planned::empty();
    for p in block.patches {
        planned.plan.patch(p.at, &p.bytes);
        planned.patches.push((p.at, p.bytes.len(), p.kind));
        if let Some(reason) = p.reason {
            planned.scrubs.push((p.at, p.bytes.len(), reason));
        }
    }
    for d in block.deleted {
        planned.entries.push(EntryRecord {
            // A bare TIFF is block 0 of its own walk.
            block: 0,
            ifd: d.ifd,
            ifd_index: d.ifd_index,
            tag: d.tag,
            field_type: d.field_type,
            count: d.count,
            offset: d.at,
        });
    }
    planned.entries.sort_by_key(|e| e.offset);
    Ok(planned)
}
