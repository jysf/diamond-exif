//! The audit model (SPEC-012, DEC-008): a pure function from bytes to a
//! [`FileAudit`].
//!
//! [`audit`] sniffs the container ([`sniff`]), reads a TIFF or a JPEG with the
//! existing tolerant readers, and returns
//!
//! * an **inventory** ([`FileAudit::items`]): every IFD entry the reader found
//!   (counts and types only, no value bytes) and every located JPEG region;
//! * **findings** ([`FileAudit::findings`]): the DEC-008 categories, each
//!   pointing at the inventory item it is about;
//! * a **health** summary ([`FileAudit::health`]): was the file readable at
//!   all, and did the readers find problems.
//!
//! # Three things a caller must not conflate
//!
//! "No findings", "clean" and "safe" are different. An unsupported container
//! (PNG, WebP, HEIC, BigTIFF, ...), an empty file and a recognised container
//! whose metadata could not be read all have **no findings**, and none of them
//! is [`Health::clean`]. A caller that wants to say "nothing personal was
//! found" must check [`FileAudit::kind`] and [`FileAudit::health`] as well.
//!
//! # Several Exif blocks
//!
//! Findings come from **every** parsed Exif block of a JPEG, each tagged with
//! its block index ([`Origin::block`]) and whether it lies after the scan
//! ([`Origin::after_scan`]). Nothing is merged or deduplicated: a privacy
//! report must show every copy.
//!
//! # Bounds
//!
//! At most [`MAX_INVENTORY_ITEMS`] items and as many findings are kept. Work is
//! linear in the entries the readers already produced, and every entry is still
//! classified past the cap: a finding for an item that did not fit is kept
//! (with [`Finding::item`] `None`) until the findings cap, and
//! [`Health::inventory_truncated`] is set. The readers themselves keep every
//! IFD's entries, which is bounded by their own limits, not by this module.
//!
//! Provenance (DEC-003, class 1): net-new code over this crate's own readers
//! and the DEC-008 table. No ExifTool source is read.

mod category;
mod preview;
mod sniff;

pub use category::{categorize, region_category, Category, Severity};
pub use preview::{preview, Counts, Disposition, Preview};
pub use sniff::{sniff, Sniffed};

use crate::jpeg::{self, Region};
use crate::policy::StripSpec;
use crate::tiff::{Container, Ifd, IfdKind};

/// The most items (and, separately, the most findings) an audit keeps.
/// Exceeding either sets [`Health::inventory_truncated`].
pub const MAX_INVENTORY_ITEMS: usize = 1 << 16;

/// IFD1 `JPEGInterchangeFormat` (the thumbnail's offset).
const TAG_JPEG_OFFSET: u16 = 0x0201;
/// IFD1 `JPEGInterchangeFormatLength` (the thumbnail's length).
const TAG_JPEG_LENGTH: u16 = 0x0202;
/// `StripOffsets`: an uncompressed IFD1 thumbnail.
const TAG_STRIP_OFFSETS: u16 = 0x0111;
/// TIFF field types `SHORT` and `LONG`.
const TYPE_SHORT: u16 = 3;
const TYPE_LONG: u16 = 4;

/// What container the audit decided the input is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerKind {
    /// Classic TIFF, read with [`Container::parse`].
    Tiff,
    /// JPEG, read with [`jpeg::read`].
    Jpeg,
    /// A container this library does not read (the sniffed kind is kept).
    Unsupported(Sniffed),
}

/// The kind of a located JPEG region (an item's [`ItemKind::Region`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RegionKind {
    /// An XMP packet.
    Xmp,
    /// One extended-XMP chunk.
    ExtendedXmp,
    /// An IPTC-NAA resource.
    Iptc,
    /// One ICC profile chunk.
    Icc,
    /// A JUMBF segment.
    Jumbf,
    /// A Multi-Picture Format image other than the primary (DEC-012).
    EmbeddedImage,
    /// A `COM` segment.
    Comment,
    /// An unrecognised APPn segment.
    OtherApp,
    /// Bytes after the `EOI` that no embedded image accounts for.
    Trailer,
}

/// Where an inventory item came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Origin {
    /// The Exif block (index into [`jpeg::JpegMetadata::exif_blocks`]) for an
    /// entry of a JPEG's Exif; `None` for a bare TIFF and for regions.
    pub block: Option<usize>,
    /// The JPEG segment index the block or region came from; `None` for a bare
    /// TIFF.
    pub segment: Option<usize>,
    /// Whether that segment lies after the first scan.
    pub after_scan: bool,
}

/// What an inventory item is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    /// One IFD entry. Nothing of its value is kept.
    Tag {
        /// How the IFD was reached.
        ifd: IfdKind,
        /// The IFD's index in its container's [`Container::ifds`].
        ifd_index: usize,
        /// The tag number.
        tag: u16,
        /// The TIFF field type as declared (unvalidated).
        field_type: u16,
        /// The value count as declared (unvalidated).
        count: u32,
    },
    /// A located JPEG region.
    Region {
        /// Which region.
        kind: RegionKind,
        /// The JPEG segment index.
        segment: usize,
        /// The region's data length in bytes.
        len: usize,
    },
}

/// One inventory entry: where it came from, what it is (counts and types), and
/// a borrow of the entry's raw payload.
///
/// The payload ([`Item::value`]) is a slice of the audited input: never copied
/// into the audit and never decoded. The audit itself neither reads nor
/// interprets it; the report renderers print a bounded excerpt of it only when
/// asked (DEC-010). Equality compares it. `Debug` shows only its **length**, so
/// a `{:?}` of an audit never prints personal data into a log.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Item<'a> {
    origin: Origin,
    kind: ItemKind,
    value: Option<&'a [u8]>,
}

impl core::fmt::Debug for Item<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Item")
            .field("origin", &self.origin)
            .field("kind", &self.kind)
            .field("value_len", &self.value.map(<[u8]>::len))
            .finish()
    }
}

impl<'a> Item<'a> {
    fn new(origin: Origin, kind: ItemKind) -> Item<'a> {
        Item {
            origin,
            kind,
            value: None,
        }
    }

    fn with_value(mut self, value: Option<&'a [u8]>) -> Item<'a> {
        self.value = value;
        self
    }

    /// Where the item came from.
    pub fn origin(&self) -> Origin {
        self.origin
    }

    /// What the item is.
    pub fn kind(&self) -> ItemKind {
        self.kind
    }

    /// The raw payload of a tag item, exactly as [`Container::payload`]
    /// returns it (a borrowed slice of the audited input, never decoded).
    /// `None` for a region and for a tag whose payload is unreadable (an
    /// unknown field type, an overflowing size, or a range outside the data).
    pub fn value(&self) -> Option<&'a [u8]> {
        self.value
    }
}

/// Where a thumbnail's bytes are, and whether they look like a JPEG.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThumbnailRange {
    /// `(offset, length)` inside the TIFF blob of the Exif block (or the bare
    /// TIFF), present only when `0x0201` and `0x0202` are both there, `SHORT`
    /// or `LONG` with count 1, and the range lies inside the blob.
    pub range: Option<(u32, u32)>,
    /// Whether the bytes at `range` start with `FF D8`. `false` when `range`
    /// is `None`.
    pub starts_with_ffd8: bool,
}

/// What an embedded image (a Multi-Picture Format image other than the
/// primary) holds, looked at once (DEC-012 section 3). Informational: it never
/// changes a disposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmbeddedSummary {
    /// Input offset of the image's first byte.
    pub offset: usize,
    /// The image's length in bytes.
    pub length: usize,
    /// Whether the bytes start with `FF D8` (always true for a located image).
    pub starts_with_ffd8: bool,
    /// The categories of the findings of the ordinary audit of the image's
    /// bytes, run with embedded-image location off (an inner MPF segment is an
    /// `OtherSegment`; nothing recurses).
    pub holds: StripSpec,
}

/// One PII finding: a category, and the inventory item it is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Finding<'a> {
    /// The DEC-008 category.
    pub category: Category,
    /// Index into [`FileAudit::items`], or `None` when the item did not fit in
    /// the inventory (then [`Health::inventory_truncated`] is set).
    pub item: Option<usize>,
    /// What the finding is about, always present (equal to `items()[i]` when
    /// `item` is `Some(i)`).
    pub at: Item<'a>,
    /// For a [`Category::Thumbnail`] finding, where the thumbnail is.
    pub thumbnail: Option<ThumbnailRange>,
    /// For a [`Category::EmbeddedImage`] finding, what the image holds.
    pub embedded: Option<EmbeddedSummary>,
}

/// Was the file readable, and did the readers find problems?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Health {
    /// True iff the container was readable with no JPEG-layer issue, no
    /// TIFF-layer issue, no failed block and no truncation. **Not** "no
    /// personal data": see the module docs.
    pub clean: bool,
    /// JPEG-layer issues (the walker's and the extraction's).
    pub jpeg_issues: usize,
    /// TIFF-layer issues summed over every readable TIFF (every Exif block).
    pub exif_issues: usize,
    /// Exif blocks (or the bare TIFF) whose IFD0 or header was unreadable. A
    /// JPEG the walker rejected outright counts as one failed block.
    pub failed_blocks: usize,
    /// Whether the inventory or the findings hit [`MAX_INVENTORY_ITEMS`].
    pub inventory_truncated: bool,
}

/// The audit of one file. See the module docs.
#[derive(Debug, Clone)]
pub struct FileAudit<'a> {
    kind: ContainerKind,
    items: Vec<Item<'a>>,
    findings: Vec<Finding<'a>>,
    health: Health,
    max_severity: Option<Severity>,
}

impl<'a> FileAudit<'a> {
    /// The container the input was taken for.
    pub fn kind(&self) -> ContainerKind {
        self.kind
    }

    /// The inventory: every IFD entry and region found, at most
    /// [`MAX_INVENTORY_ITEMS`], in container order (a bare TIFF: IFD order,
    /// then entry order; a JPEG: each Exif block in turn, then the regions in
    /// segment order).
    pub fn items(&self) -> &[Item<'a>] {
        &self.items
    }

    /// The findings, in item order, at most [`MAX_INVENTORY_ITEMS`].
    pub fn findings(&self) -> &[Finding<'a>] {
        &self.findings
    }

    /// The health summary.
    pub fn health(&self) -> &Health {
        &self.health
    }

    /// The highest severity over every finding found, including any dropped
    /// by the findings cap. `None` means no findings, which is not the same
    /// as clean (see the module docs).
    pub fn max_severity(&self) -> Option<Severity> {
        self.max_severity
    }

    /// The findings of one category, in item order.
    pub fn findings_in(&self, c: Category) -> impl Iterator<Item = &Finding<'a>> {
        self.findings.iter().filter(move |f| f.category == c)
    }
}

/// Audit `data`. Total: never fails and never panics, whatever the bytes.
pub fn audit(data: &[u8]) -> FileAudit<'_> {
    match sniff(data) {
        Sniffed::Tiff => audit_tiff(data),
        Sniffed::Jpeg => audit_jpeg(data, true),
        other => FileAudit {
            kind: ContainerKind::Unsupported(other),
            items: Vec::new(),
            findings: Vec::new(),
            // Nothing was read, so it can never be reported clean.
            health: Health {
                clean: false,
                jpeg_issues: 0,
                exif_issues: 0,
                failed_blocks: 0,
                inventory_truncated: false,
            },
            max_severity: None,
        },
    }
}

/// An audit whose container was recognised but could not be read at all.
fn unreadable(kind: ContainerKind) -> FileAudit<'static> {
    FileAudit {
        kind,
        items: Vec::new(),
        findings: Vec::new(),
        health: Health {
            clean: false,
            jpeg_issues: 0,
            exif_issues: 0,
            failed_blocks: 1,
            inventory_truncated: false,
        },
        max_severity: None,
    }
}

fn audit_tiff(data: &[u8]) -> FileAudit<'_> {
    let Ok(container) = Container::parse(data) else {
        return unreadable(ContainerKind::Tiff);
    };
    let mut b = Builder::default();
    let origin = Origin {
        block: None,
        segment: None,
        after_scan: false,
    };
    b.add_container(&container, data, origin);
    let exif_issues = container.issues().len();
    b.finish(ContainerKind::Tiff, 0, exif_issues, 0)
}

/// The audit of a JPEG with embedded-image location on (`embedded`: the
/// ordinary audit) or off (the inner audit that computes `holds`).
fn audit_jpeg(data: &[u8], embedded: bool) -> FileAudit<'_> {
    let Ok(meta) = jpeg::read_with(data, embedded) else {
        return unreadable(ContainerKind::Jpeg);
    };
    let segments = meta.jpeg().segments();
    let after_scan = |segment: usize| segments.get(segment).is_some_and(|s| s.after_scan());

    let mut b = Builder::default();
    let mut exif_issues = 0usize;
    let mut failed = 0usize;
    for (index, block) in meta.exif_blocks().iter().enumerate() {
        match block.container() {
            Ok(container) => {
                exif_issues = exif_issues.saturating_add(container.issues().len());
                let origin = Origin {
                    block: Some(index),
                    segment: Some(block.segment()),
                    after_scan: after_scan(block.segment()),
                };
                b.add_container(container, block.tiff(), origin);
            }
            Err(_) => failed = failed.saturating_add(1),
        }
    }
    for region in meta.regions() {
        let (kind, segment) = match *region {
            Region::Xmp { segment, .. } => (RegionKind::Xmp, segment),
            Region::ExtendedXmp { segment, .. } => (RegionKind::ExtendedXmp, segment),
            Region::Icc { segment, .. } => (RegionKind::Icc, segment),
            Region::Iptc { segment, .. } => (RegionKind::Iptc, segment),
            Region::Jumbf { segment, .. } => (RegionKind::Jumbf, segment),
            Region::EmbeddedImage { segment, .. } => (RegionKind::EmbeddedImage, segment),
            Region::Comment { segment, .. } => (RegionKind::Comment, segment),
            Region::OtherApp { segment, .. } => (RegionKind::OtherApp, segment),
            Region::Trailer { segment, .. } => (RegionKind::Trailer, segment),
        };
        // A trailer lies after the scan (its "segment" is one past the last).
        let trailing = kind == RegionKind::Trailer;
        let origin = Origin {
            block: None,
            segment: Some(segment),
            after_scan: trailing || after_scan(segment),
        };
        let item = Item::new(
            origin,
            ItemKind::Region {
                kind,
                segment,
                len: region.data().len(),
            },
        );
        let summary = match *region {
            Region::EmbeddedImage { offset, data, .. } => Some(embedded_summary(offset, data)),
            _ => None,
        };
        b.push_with(item, Some(region_category(kind)), None, summary);
    }
    b.finish(
        ContainerKind::Jpeg,
        meta.jpeg_issues().len(),
        exif_issues,
        failed,
    )
}

/// Accumulates items and findings under the caps.
#[derive(Default)]
struct Builder<'a> {
    items: Vec<Item<'a>>,
    findings: Vec<Finding<'a>>,
    truncated: bool,
    max_severity: Option<Severity>,
}

impl<'a> Builder<'a> {
    /// Add one item and, if it has one, its finding. Always classifies; only
    /// the *storing* is capped.
    fn push(&mut self, item: Item<'a>, category: Option<Category>, thumb: Option<ThumbnailRange>) {
        self.push_with(item, category, thumb, None);
    }

    /// [`Builder::push`] with, for an embedded image, what it holds.
    fn push_with(
        &mut self,
        item: Item<'a>,
        category: Option<Category>,
        thumb: Option<ThumbnailRange>,
        embedded: Option<EmbeddedSummary>,
    ) {
        let index = if self.items.len() < MAX_INVENTORY_ITEMS {
            let index = self.items.len();
            self.items.push(item);
            Some(index)
        } else {
            self.truncated = true;
            None
        };
        let Some(category) = category else {
            return;
        };
        let severity = category.severity();
        if self.max_severity.is_none_or(|m| m < severity) {
            self.max_severity = Some(severity);
        }
        if self.findings.len() < MAX_INVENTORY_ITEMS {
            self.findings.push(Finding {
                category,
                item: index,
                at: item,
                thumbnail: if category == Category::Thumbnail {
                    thumb
                } else {
                    None
                },
                embedded: if category == Category::EmbeddedImage {
                    embedded
                } else {
                    None
                },
            });
        } else {
            self.truncated = true;
        }
    }

    /// Every entry of every IFD of one parsed TIFF, in container order.
    /// `blob` is the byte string the container was parsed from (its offsets
    /// are relative to it).
    fn add_container(&mut self, container: &Container<'a>, blob: &[u8], origin: Origin) {
        for (ifd_index, ifd) in container.ifds().iter().enumerate() {
            // Per-IFD context, computed once (never per entry: a hostile IFD
            // can hold 65,535 entries, and a per-entry scan would be quadratic).
            let has_jpeg_pointer = ifd.has(TAG_JPEG_OFFSET);
            let thumb = if ifd.kind() == IfdKind::Ifd1 {
                Some(thumbnail_range(container, ifd, blob))
            } else {
                None
            };
            for entry in ifd.entries() {
                let tag = entry.tag();
                let category = match ifd.kind() {
                    IfdKind::Ifd1 if tag == TAG_STRIP_OFFSETS && !has_jpeg_pointer => {
                        Some(Category::Thumbnail)
                    }
                    kind => categorize(kind, tag),
                };
                let item = Item::new(
                    origin,
                    ItemKind::Tag {
                        ifd: ifd.kind(),
                        ifd_index,
                        tag,
                        field_type: entry.field_type(),
                        count: entry.count(),
                    },
                )
                .with_value(container.payload(entry).ok());
                // Only the 0x0201 entry carries a range; a strips thumbnail
                // has none.
                let range = match (category, thumb) {
                    (Some(Category::Thumbnail), Some(t)) if tag == TAG_JPEG_OFFSET => Some(t),
                    (Some(Category::Thumbnail), _) => Some(ThumbnailRange {
                        range: None,
                        starts_with_ffd8: false,
                    }),
                    _ => None,
                };
                self.push(item, category, range);
            }
        }
    }

    fn finish(
        self,
        kind: ContainerKind,
        jpeg_issues: usize,
        exif_issues: usize,
        failed_blocks: usize,
    ) -> FileAudit<'a> {
        let clean = jpeg_issues == 0 && exif_issues == 0 && failed_blocks == 0 && !self.truncated;
        FileAudit {
            kind,
            items: self.items,
            findings: self.findings,
            health: Health {
                clean,
                jpeg_issues,
                exif_issues,
                failed_blocks,
                inventory_truncated: self.truncated,
            },
            max_severity: self.max_severity,
        }
    }
}

/// What the embedded image at input offset `offset` holds: the categories of
/// the ordinary JPEG audit of its bytes with embedded-image location off
/// (DEC-012 section 3). Run once per image; the images are bounded by
/// [`jpeg::MAX_MPF_IMAGES`].
fn embedded_summary(offset: usize, bytes: &[u8]) -> EmbeddedSummary {
    let inner = audit_jpeg(bytes, false);
    let holds = inner
        .findings()
        .iter()
        .fold(StripSpec::none(), |s, f| s.include(f.category));
    EmbeddedSummary {
        offset,
        length: bytes.len(),
        starts_with_ffd8: bytes.starts_with(&[0xFF, 0xD8]),
        holds,
    }
}

/// The thumbnail range of an IFD1: from its first `0x0201` and `0x0202`, both
/// `SHORT`/`LONG` with count 1, with the range inside `blob`.
fn thumbnail_range(container: &Container<'_>, ifd: &Ifd, blob: &[u8]) -> ThumbnailRange {
    let none = ThumbnailRange {
        range: None,
        starts_with_ffd8: false,
    };
    let scalar = |tag: u16| -> Option<u32> {
        let entry = ifd.entry(tag)?;
        if entry.count() != 1 || !matches!(entry.field_type(), TYPE_SHORT | TYPE_LONG) {
            return None;
        }
        container.scalar(ifd, tag).ok().flatten()
    };
    let (Some(offset), Some(length)) = (scalar(TAG_JPEG_OFFSET), scalar(TAG_JPEG_LENGTH)) else {
        return none;
    };
    let Some(start) = usize::try_from(offset).ok() else {
        return none;
    };
    let Some(end) = usize::try_from(length)
        .ok()
        .and_then(|l| start.checked_add(l))
    else {
        return none;
    };
    let Some(bytes) = blob.get(start..end) else {
        return none;
    };
    ThumbnailRange {
        range: Some((offset, length)),
        starts_with_ffd8: bytes.starts_with(&[0xFF, 0xD8]),
    }
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

    /// `super::audit` over a leaked copy, so a test can audit a temporary
    /// (`audit(&build(..))`). A test-only leak of a few hundred kilobytes.
    fn audit(bytes: &[u8]) -> FileAudit<'static> {
        let leaked: &'static [u8] = Box::leak(bytes.to_vec().into_boxed_slice());
        super::audit(leaked)
    }

    // ── a minimal little-endian TIFF builder, independent of src/tiff's ──────

    enum V {
        /// Up to 4 bytes, stored inline (zero padded).
        In(Vec<u8>),
        /// Appended after the IFD; the field holds its offset.
        Out(Vec<u8>),
        /// The offset of IFD `n` of the file.
        Ifd(usize),
        /// A literal field.
        Raw(u32),
    }

    struct E {
        tag: u16,
        typ: u16,
        count: u32,
        v: V,
    }

    fn ascii(tag: u16, s: &[u8]) -> E {
        let mut b = s.to_vec();
        b.push(0);
        let count = b.len() as u32;
        let v = if b.len() <= 4 { V::In(b) } else { V::Out(b) };
        E {
            tag,
            typ: 2,
            count,
            v,
        }
    }

    fn long(tag: u16, n: u32) -> E {
        E {
            tag,
            typ: 4,
            count: 1,
            v: V::In(n.to_le_bytes().to_vec()),
        }
    }

    fn short(tag: u16, n: u16) -> E {
        E {
            tag,
            typ: 3,
            count: 1,
            v: V::In(n.to_le_bytes().to_vec()),
        }
    }

    fn undefined(tag: u16, bytes: &[u8]) -> E {
        let count = bytes.len() as u32;
        let v = if bytes.len() <= 4 {
            V::In(bytes.to_vec())
        } else {
            V::Out(bytes.to_vec())
        };
        E {
            tag,
            typ: 7,
            count,
            v,
        }
    }

    fn ptr(tag: u16, ifd: usize) -> E {
        E {
            tag,
            typ: 4,
            count: 1,
            v: V::Ifd(ifd),
        }
    }

    fn raw(tag: u16, typ: u16, count: u32, field: u32) -> E {
        E {
            tag,
            typ,
            count,
            v: V::Raw(field),
        }
    }

    /// `(entries, index of the next IFD in the chain)`.
    type Spec = (Vec<E>, Option<usize>);

    /// Header, then every IFD with its out-of-line payloads right behind it.
    /// `tail` is appended last (thumbnail bytes live there).
    fn build(ifds: &[Spec], tail: &[u8]) -> Vec<u8> {
        let mut starts = Vec::new();
        let mut at = 8usize;
        for (entries, _) in ifds {
            starts.push(at);
            at += 2 + 12 * entries.len() + 4;
            for e in entries {
                if let V::Out(b) = &e.v {
                    at += b.len() + (b.len() & 1);
                }
            }
        }
        let mut out = b"II\x2A\x00".to_vec();
        out.extend_from_slice(&(starts.first().copied().unwrap_or(0) as u32).to_le_bytes());
        for (entries, next) in ifds {
            let mut ext: Vec<u8> = Vec::new();
            let base = out.len() + 2 + 12 * entries.len() + 4;
            out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
            for e in entries {
                out.extend_from_slice(&e.tag.to_le_bytes());
                out.extend_from_slice(&e.typ.to_le_bytes());
                out.extend_from_slice(&e.count.to_le_bytes());
                match &e.v {
                    V::In(b) => {
                        let mut f = b.clone();
                        f.resize(4, 0);
                        out.extend_from_slice(&f);
                    }
                    V::Out(b) => {
                        out.extend_from_slice(&((base + ext.len()) as u32).to_le_bytes());
                        ext.extend_from_slice(b);
                        if b.len() & 1 == 1 {
                            ext.push(0);
                        }
                    }
                    V::Ifd(i) => out.extend_from_slice(&(starts[*i] as u32).to_le_bytes()),
                    V::Raw(f) => out.extend_from_slice(&f.to_le_bytes()),
                }
            }
            let next = next.map_or(0, |i| starts[i] as u32);
            out.extend_from_slice(&next.to_le_bytes());
            out.extend_from_slice(&ext);
        }
        out.extend_from_slice(tail);
        out
    }

    fn seg(marker: u8, payload: &[u8]) -> Vec<u8> {
        let mut v = vec![0xFF, marker];
        v.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
        v.extend_from_slice(payload);
        v
    }

    fn exif_seg(tiff: &[u8]) -> Vec<u8> {
        seg(0xE1, &[&b"Exif\0\0"[..], tiff].concat())
    }

    fn sos() -> Vec<u8> {
        seg(0xDA, &[1, 1, 0, 0, 0x3F, 0])
    }

    /// A clean JPEG: SOI, the given segments, SOS, two scan bytes, EOI.
    fn jpeg(parts: &[Vec<u8>]) -> Vec<u8> {
        let mut v = vec![0xFF, 0xD8];
        for p in parts {
            v.extend_from_slice(p);
        }
        v.extend_from_slice(&sos());
        v.extend_from_slice(&[0x12, 0x34, 0xFF, 0xD9]);
        v
    }

    fn tags(a: &FileAudit<'_>) -> Vec<(IfdKind, usize, u16, u16, u32)> {
        a.items()
            .iter()
            .filter_map(|i| match i.kind() {
                ItemKind::Tag {
                    ifd,
                    ifd_index,
                    tag,
                    field_type,
                    count,
                } => Some((ifd, ifd_index, tag, field_type, count)),
                ItemKind::Region { .. } => None,
            })
            .collect()
    }

    fn cats(a: &FileAudit<'_>) -> Vec<Category> {
        a.findings().iter().map(|f| f.category).collect()
    }

    fn tag_of(f: &Finding<'_>) -> (IfdKind, u16) {
        match f.at.kind() {
            ItemKind::Tag { ifd, tag, .. } => (ifd, tag),
            ItemKind::Region { .. } => panic!("a region"),
        }
    }

    // ── unsupported, empty, unreadable ───────────────────────────────────────

    #[test]
    fn audit_of_empty_and_unsupported_input_is_reported_not_clean() {
        let cases: Vec<(Sniffed, Vec<u8>)> = vec![
            (Sniffed::Empty, vec![]),
            (Sniffed::BigTiff, b"II\x2B\x00\x08\x00\x08\x00".to_vec()),
            (Sniffed::BigTiff, b"MM\x00\x2B\x00\x08\x00\x00".to_vec()),
            (
                Sniffed::Png,
                vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0, 0],
            ),
            (Sniffed::Webp, b"RIFF\x04\x00\x00\x00WEBPVP8 ".to_vec()),
            (Sniffed::Heic, b"\x00\x00\x00\x18ftypheic".to_vec()),
            (Sniffed::Gif, b"GIF89a\x01\x00\x01\x00".to_vec()),
            (Sniffed::Unknown, b"this is not an image".to_vec()),
            (Sniffed::Unknown, vec![0xFF]),
        ];
        for (want, bytes) in cases {
            let a = audit(&bytes);
            assert_eq!(a.kind(), ContainerKind::Unsupported(want), "{bytes:?}");
            assert!(a.items().is_empty());
            assert!(a.findings().is_empty());
            assert_eq!(a.max_severity(), None);
            assert!(!a.health().clean, "unsupported is never clean: {want:?}");
            assert!(!a.health().inventory_truncated);
        }
    }

    #[test]
    fn audit_of_tiff_with_unreadable_ifd0_is_a_failed_block_not_an_empty_audit() {
        // IFD0 offset points past the end.
        let bytes = b"II\x2A\x00\x40\x00\x00\x00".to_vec();
        let a = audit(&bytes);
        assert_eq!(a.kind(), ContainerKind::Tiff);
        assert!(a.items().is_empty() && a.findings().is_empty());
        assert_eq!(a.health().failed_blocks, 1);
        assert!(!a.health().clean);
        // Header cut short: the same, and never Unsupported.
        for bytes in [&b"II\x2A\x00"[..], b"II\x2A\x00\x08\x00"] {
            let a = audit(bytes);
            assert_eq!(a.kind(), ContainerKind::Tiff);
            assert_eq!(a.health().failed_blocks, 1);
            assert!(!a.health().clean);
        }
        // A readable TIFF with IFD0 offset 0 is a clean, empty audit: nothing
        // is wrong with the file, it just holds no IFD.
        let ok = audit(b"II\x2A\x00\x00\x00\x00\x00");
        assert_eq!(ok.kind(), ContainerKind::Tiff);
        assert!(ok.health().clean && ok.items().is_empty());
        assert_eq!(ok.health().failed_blocks, 0);
    }

    // ── tags and categories ─────────────────────────────────────────────────

    #[test]
    fn audit_lists_pointer_tags_in_the_inventory_but_never_as_findings() {
        let bytes = build(
            &[
                (
                    vec![
                        ascii(0x010F, b"Cam"),
                        ptr(0x8769, 1),
                        ptr(0x8825, 2),
                        raw(0x014A, 4, 0, 0),
                    ],
                    None,
                ),
                (vec![ptr(0xA005, 3), short(0x8827, 100)], None),
                (vec![ascii(1, b"N")], None),
                (vec![ascii(1, b"R98")], None),
            ],
            &[],
        );
        let a = audit(&bytes);
        let t = tags(&a);
        for pointer in [0x8769u16, 0x8825, 0xA005, 0x014A] {
            assert!(t.iter().any(|r| r.2 == pointer), "{pointer:#x} is listed");
        }
        // Findings: Make (CameraModel) and the one GPS tag. The Interop tag and
        // the ISO are inventory only.
        assert_eq!(cats(&a), vec![Category::CameraModel, Category::Gps]);
        assert!(a.health().clean, "{:?}", a.health());
        assert_eq!(a.max_severity(), Some(Severity::High));
    }

    #[test]
    fn a_gps_ifd_yields_one_gps_finding_per_tag_and_ifd0_ids_none() {
        // Tags 1, 2, 3 and 0x9999 in the GPS IFD; the same numeric ids in IFD0.
        let bytes = build(
            &[
                (
                    vec![
                        short(1, 0),
                        short(2, 0),
                        short(3, 0),
                        short(0x9999, 0),
                        ptr(0x8825, 1),
                    ],
                    None,
                ),
                (
                    vec![
                        ascii(1, b"N"),
                        ascii(2, b"x"),
                        ascii(3, b"E"),
                        ascii(0x9999, b"x"),
                    ],
                    None,
                ),
            ],
            &[],
        );
        let a = audit(&bytes);
        assert_eq!(a.findings_in(Category::Gps).count(), 4);
        assert_eq!(a.findings().len(), 4);
        for f in a.findings() {
            assert_eq!(tag_of(f).0, IfdKind::Gps);
        }
    }

    #[test]
    fn a_makernote_is_one_finding_and_makes_no_serial_claim() {
        // A MakerNote that looks like it holds a serial number: still exactly
        // one MakerNote finding and no DeviceId.
        let note = b"Canon\0SerialNumber=1234567890\0";
        let bytes = build(
            &[
                (vec![ptr(0x8769, 1)], None),
                (vec![undefined(0x927C, note), short(0x8827, 100)], None),
            ],
            &[],
        );
        let a = audit(&bytes);
        assert_eq!(cats(&a), vec![Category::MakerNote]);
        assert_eq!(a.findings_in(Category::DeviceId).count(), 0);
        assert_eq!(a.max_severity(), Some(Severity::Medium));
    }

    fn thumb_file(offset: u32, length: u32, thumb: &[u8]) -> Vec<u8> {
        // IFD0 chains to IFD1 holding the JPEG pointer and length; `thumb` is
        // the file tail (the declared range is whatever the test says).
        build(
            &[
                (vec![ascii(0x010F, b"Cam")], Some(1)),
                (vec![long(0x0201, offset), long(0x0202, length)], None),
            ],
            thumb,
        )
    }

    fn thumb_of(a: &FileAudit<'_>) -> ThumbnailRange {
        assert_eq!(a.findings_in(Category::Thumbnail).count(), 1);
        a.findings_in(Category::Thumbnail)
            .next()
            .unwrap()
            .thumbnail
            .expect("a thumbnail finding carries a ThumbnailRange")
    }

    #[test]
    fn an_ifd1_thumbnail_is_a_finding_with_a_checked_range() {
        let jpeg_thumb = [0xFF, 0xD8, 0xFF, 0xD9];
        // The tail starts after both IFDs (the offset does not depend on the
        // pointer values, so a probe build finds it).
        let probe = thumb_file(0, 0, &jpeg_thumb);
        let tail_at = (probe.len() - jpeg_thumb.len()) as u32;

        // In the blob, starts with FF D8.
        let a = audit(&thumb_file(tail_at, 4, &jpeg_thumb));
        assert_eq!(
            thumb_of(&a),
            ThumbnailRange {
                range: Some((tail_at, 4)),
                starts_with_ffd8: true
            }
        );
        // The finding is on the 0x0201 entry, not on 0x0202, and indexes it.
        let f = a.findings_in(Category::Thumbnail).next().unwrap();
        assert_eq!(tag_of(f), (IfdKind::Ifd1, 0x0201));
        assert_eq!(f.item.map(|i| a.items()[i]), Some(f.at));

        // In the blob, not a JPEG: flagged.
        let a = audit(&thumb_file(tail_at, 4, b"\x00\x01\x02\x03"));
        assert_eq!(
            thumb_of(&a),
            ThumbnailRange {
                range: Some((tail_at, 4)),
                starts_with_ffd8: false
            }
        );

        // One byte past the end of the blob: no range.
        let past = audit(&thumb_file(tail_at, 5, &jpeg_thumb));
        assert_eq!(
            thumb_of(&past),
            ThumbnailRange {
                range: None,
                starts_with_ffd8: false
            }
        );
        // Offset past the blob; offset + length overflowing; u32::MAX.
        for (o, l) in [
            (0x00FF_FFFFu32, 4u32),
            (tail_at, 0xFFFF_FFFF),
            (u32::MAX, 2),
        ] {
            let a = audit(&thumb_file(o, l, &jpeg_thumb));
            assert_eq!(thumb_of(&a).range, None, "({o:#x}, {l:#x})");
        }
        // A zero length at the very end of the blob is inside, and cannot
        // start with FF D8.
        let end = probe.len() as u32;
        let a = audit(&thumb_file(end, 0, &jpeg_thumb));
        assert_eq!(thumb_of(&a).range, Some((end, 0)));
        assert!(!thumb_of(&a).starts_with_ffd8);
        // One byte of a JPEG is not a JPEG start.
        let a = audit(&thumb_file(tail_at, 1, &jpeg_thumb));
        assert!(!thumb_of(&a).starts_with_ffd8);
    }

    #[test]
    fn an_ifd1_thumbnail_with_unusable_pointer_types_has_no_range() {
        let no_range = ThumbnailRange {
            range: None,
            starts_with_ffd8: false,
        };
        // 0x0201 declared RATIONAL.
        let a = audit(&build(
            &[
                (vec![ascii(0x010F, b"Cam")], Some(1)),
                (vec![raw(0x0201, 5, 1, 0), long(0x0202, 4)], None),
            ],
            &[0xFF, 0xD8, 0xFF, 0xD9],
        ));
        assert_eq!(thumb_of(&a), no_range);
        // 0x0201 without 0x0202.
        let a = audit(&build(
            &[
                (vec![ascii(0x010F, b"Cam")], Some(1)),
                (vec![long(0x0201, 0)], None),
            ],
            &[],
        ));
        assert_eq!(thumb_of(&a), no_range);
        // 0x0201 in IFD0 is not a thumbnail finding at all.
        let a = audit(&build(
            &[(vec![long(0x0201, 8), long(0x0202, 4)], None)],
            &[],
        ));
        assert_eq!(a.findings().len(), 0);
    }

    #[test]
    fn an_ifd1_with_strips_and_no_jpeg_pointer_is_a_thumbnail_finding() {
        let strips = build(
            &[
                (vec![ascii(0x010F, b"Cam")], Some(1)),
                (vec![long(0x0111, 8), long(0x0117, 4)], None),
            ],
            &[],
        );
        let a = audit(&strips);
        let f: Vec<_> = a.findings_in(Category::Thumbnail).collect();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].thumbnail.unwrap().range, None);
        assert_eq!(tag_of(f[0]), (IfdKind::Ifd1, 0x0111));
        // With a JPEG pointer present the strips entry adds nothing.
        let both = build(
            &[
                (vec![ascii(0x010F, b"Cam")], Some(1)),
                (
                    vec![long(0x0111, 8), long(0x0201, 8), long(0x0202, 4)],
                    None,
                ),
            ],
            &[],
        );
        let a = audit(&both);
        assert_eq!(a.findings_in(Category::Thumbnail).count(), 1);
        assert_eq!(
            tag_of(a.findings_in(Category::Thumbnail).next().unwrap()).1,
            0x0201
        );
        // Strips in IFD0 are the main image, never a thumbnail.
        let main = build(&[(vec![long(0x0111, 8)], None)], &[]);
        assert_eq!(audit(&main).findings().len(), 0);
    }

    // ── JPEG ────────────────────────────────────────────────────────────────

    #[test]
    fn two_exif_blocks_report_findings_from_both_and_never_deduplicate() {
        let t1 = build(&[(vec![ascii(0x010F, b"One")], None)], &[]);
        let t2 = build(&[(vec![ascii(0x010F, b"Two")], None)], &[]);
        let a = audit(&jpeg(&[exif_seg(&t1), exif_seg(&t2)]));
        assert_eq!(a.kind(), ContainerKind::Jpeg);
        let blocks: Vec<Option<usize>> = a.items().iter().map(|i| i.origin().block).collect();
        assert_eq!(blocks, vec![Some(0), Some(1)]);
        assert_eq!(a.findings_in(Category::CameraModel).count(), 2);
        let segs: Vec<Option<usize>> = a.items().iter().map(|i| i.origin().segment).collect();
        assert_eq!(segs, vec![Some(0), Some(1)]);
        assert!(a.items().iter().all(|i| !i.origin().after_scan));
        // The second Exif segment is an ExtraExif issue: not clean, but both
        // blocks are audited.
        assert!(!a.health().clean);
        assert_eq!(a.health().jpeg_issues, 1);
        assert_eq!(a.health().failed_blocks, 0);
    }

    #[test]
    fn an_exif_block_after_the_scan_is_included_and_flagged() {
        let t = build(&[(vec![ascii(0x010F, b"Late")], None)], &[]);
        let mut v = vec![0xFF, 0xD8];
        v.extend_from_slice(&sos());
        v.extend_from_slice(&[0x12, 0x34]);
        v.extend_from_slice(&exif_seg(&t));
        v.extend_from_slice(&[0xFF, 0xD9]);
        let a = audit(&v);
        assert_eq!(a.items().len(), 1);
        assert!(a.items()[0].origin().after_scan);
        assert_eq!(a.items()[0].origin().block, Some(0));
        assert_eq!(cats(&a), vec![Category::CameraModel]);
        // And one before the scan is not flagged.
        let before = audit(&jpeg(&[exif_seg(&t)]));
        assert!(!before.items()[0].origin().after_scan);
    }

    #[test]
    fn a_fatal_exif_block_counts_as_failed_and_the_audit_is_not_clean() {
        let good = build(&[(vec![ascii(0x010F, b"Good")], None)], &[]);
        let bad = b"II\x2A\x00\xFF\xFF\xFF\x7F".to_vec();
        let a = audit(&jpeg(&[exif_seg(&bad), exif_seg(&good)]));
        assert_eq!(a.health().failed_blocks, 1);
        assert!(!a.health().clean);
        // The good second block is still audited, with block index 1.
        assert_eq!(a.items().len(), 1);
        assert_eq!(a.items()[0].origin().block, Some(1));
        assert_eq!(cats(&a), vec![Category::CameraModel]);
        // Only a bad block: no items, one failed block, not clean.
        let only = audit(&jpeg(&[exif_seg(&bad)]));
        assert!(only.items().is_empty());
        assert_eq!(only.health().failed_blocks, 1);
        assert!(!only.health().clean);
    }

    #[test]
    fn regions_are_items_and_findings_in_segment_order() {
        let xmp = seg(0xE1, b"http://ns.adobe.com/xap/1.0/\0<x:xmpmeta/>");
        let mut icc_payload = b"ICC_PROFILE\0".to_vec();
        icc_payload.extend_from_slice(&[1, 1, 9, 9, 9]);
        let icc = seg(0xE2, &icc_payload);
        let mut jumbf_payload = b"JP".to_vec();
        jumbf_payload.extend_from_slice(&[0, 1, 0, 0, 0, 1, 7, 7]);
        let jumbf = seg(0xEB, &jumbf_payload);
        let a = audit(&jpeg(&[xmp, icc, jumbf]));
        assert!(a.health().clean, "{:?}", a.health());
        let kinds: Vec<RegionKind> = a
            .items()
            .iter()
            .filter_map(|i| match i.kind() {
                ItemKind::Region { kind, .. } => Some(kind),
                ItemKind::Tag { .. } => None,
            })
            .collect();
        assert_eq!(
            kinds,
            vec![RegionKind::Xmp, RegionKind::Icc, RegionKind::Jumbf]
        );
        assert_eq!(
            cats(&a),
            vec![Category::Xmp, Category::Icc, Category::Jumbf]
        );
        assert!(a.items().iter().all(|i| i.origin().block.is_none()));
        assert_eq!(a.items()[0].origin().segment, Some(0));
        match a.items()[1].kind() {
            ItemKind::Region { len, segment, .. } => assert_eq!((len, segment), (3, 1)),
            ItemKind::Tag { .. } => panic!("tag"),
        }
        assert_eq!(a.max_severity(), Some(Severity::Medium));
    }

    #[test]
    fn a_clean_jpeg_without_metadata_is_clean_with_no_findings() {
        let a = audit(&jpeg(&[]));
        assert_eq!(a.kind(), ContainerKind::Jpeg);
        assert!(a.health().clean);
        assert!(a.items().is_empty() && a.findings().is_empty());
        assert_eq!(a.max_severity(), None);
        // A truncated JPEG (no SOS, no EOI) is readable but not clean.
        let a = audit(&[0xFF, 0xD8]);
        assert_eq!(a.kind(), ContainerKind::Jpeg);
        assert!(!a.health().clean);
        assert_eq!(a.health().jpeg_issues, 2);
    }

    // ── bounds ───────────────────────────────────────────────────────────────

    /// IFD0 with `n0` SHORT entries with ids that carry no category, chained
    /// to a second IFD holding `second`.
    fn many(n0: usize, second: Vec<E>) -> Vec<u8> {
        let first: Vec<E> = (0..n0)
            .map(|i| short(0xF000 + (i % 0x0FFF) as u16, 0))
            .collect();
        build(&[(first, Some(1)), (second, None)], &[])
    }

    #[test]
    fn inventory_cap_boundary_is_exact_and_findings_survive_truncation() {
        // Exactly at the cap: not truncated. IFD0 holds 65,535 entries, the
        // chained IFD holds the one more that makes 65,536 (and a finding).
        let at_cap = audit(&many(MAX_INVENTORY_ITEMS - 1, vec![ascii(0x010F, b"Cam")]));
        assert_eq!(at_cap.items().len(), MAX_INVENTORY_ITEMS);
        assert!(!at_cap.health().inventory_truncated);
        assert!(at_cap.health().clean, "{:?}", at_cap.health());
        assert_eq!(cats(&at_cap), vec![Category::CameraModel]);
        assert_eq!(at_cap.findings()[0].item, Some(MAX_INVENTORY_ITEMS - 1));

        // One over: the extra entries are not listed, the flag is set, and
        // their findings are kept without an item index.
        let over = audit(&many(
            MAX_INVENTORY_ITEMS - 1,
            vec![
                short(0xF001, 0),
                ascii(0x010F, b"Cam"),
                ascii(0xA431, b"SN"),
            ],
        ));
        assert_eq!(over.items().len(), MAX_INVENTORY_ITEMS);
        assert!(over.health().inventory_truncated);
        assert!(!over.health().clean);
        assert_eq!(cats(&over), vec![Category::CameraModel, Category::DeviceId]);
        assert_eq!(over.findings()[0].item, None, "Make did not fit");
        assert_eq!(over.findings()[1].item, None);
        assert_eq!(over.max_severity(), Some(Severity::High));
        assert_eq!(tag_of(&over.findings()[1]).1, 0xA431);
        // Where a finding has an index, it names its item.
        for f in at_cap.findings() {
            assert_eq!(f.item.map(|i| at_cap.items()[i]), Some(f.at));
        }
    }

    #[test]
    fn the_findings_cap_is_separate_and_sets_the_same_flag() {
        // 65,535 Make entries in IFD0 plus 2 in the chained IFD: 65,537
        // findings; 65,536 are kept and the flag is set.
        let first: Vec<E> = (0..MAX_INVENTORY_ITEMS - 1)
            .map(|_| short(0x010F, 0))
            .collect();
        let bytes = build(
            &[
                (first, Some(1)),
                (vec![short(0x010F, 0), short(0x010F, 0)], None),
            ],
            &[],
        );
        let a = audit(&bytes);
        assert_eq!(a.findings().len(), MAX_INVENTORY_ITEMS);
        assert!(a.health().inventory_truncated);
        assert_eq!(a.items().len(), MAX_INVENTORY_ITEMS);
    }

    #[test]
    fn item_value_is_the_raw_payload_and_none_for_regions_and_unreadable_payloads() {
        let bytes = build(
            &[(
                vec![
                    ascii(0x010F, b"Camera"),       // out of line: "Camera\0"
                    short(0x0112, 0x1234),          // inline, 2 bytes
                    raw(0x9999, 99, 1, 0),          // unknown field type
                    raw(0x9998, 4, 0x4000_0000, 8), // size overflows the data
                    raw(0x9997, 2, 8, 0xFFFF_FF00), // offset outside the data
                    ascii(0x0131, b""),             // one NUL byte, inline
                ],
                None,
            )],
            &[],
        );
        let a = audit(&bytes);
        let values: Vec<Option<&[u8]>> = a.items().iter().map(|i| i.value()).collect();
        assert_eq!(values[0], Some(&b"Camera\0"[..]));
        assert_eq!(values[1], Some(&0x1234u16.to_le_bytes()[..]));
        assert_eq!(values[2], None, "unknown type");
        assert_eq!(values[3], None, "overflowing size");
        assert_eq!(values[4], None, "offset outside the data");
        assert_eq!(values[5], Some(&b"\0"[..]));
        // A finding's item carries the same slice as the inventory's.
        assert_eq!(a.findings()[0].at.value(), Some(&b"Camera\0"[..]));
        // A region has none.
        let xmp = seg(0xE1, b"http://ns.adobe.com/xap/1.0/\0<x:xmpmeta/>");
        let j = audit(&jpeg(&[xmp]));
        assert_eq!(j.items().len(), 1);
        assert_eq!(j.items()[0].value(), None);
    }

    #[test]
    fn debug_of_an_item_and_an_audit_never_prints_the_payload() {
        let bytes = build(
            &[(
                vec![
                    ascii(0x010F, b"SECRET-SERIAL-0042"),
                    ascii(0xA431, b"SECRET-SERIAL-0042"),
                ],
                None,
            )],
            &[],
        );
        let a = audit(&bytes);
        assert_eq!(a.items()[0].value(), Some(&b"SECRET-SERIAL-0042\0"[..]));
        for dump in [
            format!("{:?}", a.items()[0]),
            format!("{:#?}", a.items()),
            format!("{:?}", a.findings()),
            format!("{a:?}"),
        ] {
            assert!(!dump.contains("SECRET"), "{dump}");
            // Nor as a byte list: 'S' is 83, 'E' 69.
            assert!(!dump.contains("83, 69, 67"), "{dump}");
        }
        let one = format!("{:?}", a.items()[0]);
        assert!(one.contains("value_len: Some(19)"), "{one}");
        assert!(one.contains("origin") && one.contains("kind"), "{one}");
    }

    #[test]
    fn audit_is_deterministic() {
        let t = build(
            &[
                (
                    vec![ascii(0x010F, b"Cam"), ptr(0x8825, 1), ptr(0x8769, 2)],
                    Some(3),
                ),
                (vec![ascii(1, b"N")], None),
                (vec![ascii(0x9003, b"2024:01:01 00:00:00")], None),
                (vec![long(0x0201, 0), long(0x0202, 0)], None),
            ],
            &[],
        );
        let file = jpeg(&[exif_seg(&t), exif_seg(&t)]);
        let a = audit(&file);
        let b = audit(&file);
        assert_eq!(a.items(), b.items());
        assert_eq!(a.findings(), b.findings());
        assert_eq!(a.health(), b.health());
        assert_eq!(a.kind(), b.kind());
        assert_eq!(a.max_severity(), b.max_severity());
    }

    // ── DEC-012: embedded images, comments, other segments, trailers ─────────

    /// A little-endian MPF APP2 segment listing `entries` as `(size, offset)`.
    fn mpf_seg_le(entries: &[(u32, u32)]) -> Vec<u8> {
        let mut t = b"II*\0".to_vec();
        t.extend_from_slice(&8u32.to_le_bytes());
        t.extend_from_slice(&2u16.to_le_bytes());
        t.extend_from_slice(&0xB001u16.to_le_bytes());
        t.extend_from_slice(&4u16.to_le_bytes());
        t.extend_from_slice(&1u32.to_le_bytes());
        t.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        t.extend_from_slice(&0xB002u16.to_le_bytes());
        t.extend_from_slice(&7u16.to_le_bytes());
        t.extend_from_slice(&((16 * entries.len()) as u32).to_le_bytes());
        t.extend_from_slice(&(8u32 + 2 + 24 + 4).to_le_bytes());
        t.extend_from_slice(&0u32.to_le_bytes());
        for (size, offset) in entries {
            t.extend_from_slice(&0u32.to_le_bytes());
            t.extend_from_slice(&size.to_le_bytes());
            t.extend_from_slice(&offset.to_le_bytes());
            t.extend_from_slice(&[0, 0, 0, 0]);
        }
        seg(0xE2, &[&b"MPF\0"[..], &t].concat())
    }

    /// A primary JPEG (with `pre` segments before its MPF index) followed by
    /// the images back to back, each indexed at its true place.
    fn mpo(pre: &[Vec<u8>], images: &[Vec<u8>]) -> Vec<u8> {
        let with = |entries: &[(u32, u32)]| {
            let mut segs = pre.to_vec();
            segs.push(mpf_seg_le(entries));
            jpeg(&segs)
        };
        let probe = with(&vec![(0, 0); images.len() + 1]);
        let mpf_at = 2 + pre.iter().map(Vec::len).sum::<usize>();
        let header_start = mpf_at + 8;
        let mut entries = vec![(probe.len() as u32, 0u32)];
        let mut at = probe.len();
        for img in images {
            entries.push((img.len() as u32, (at - header_start) as u32));
            at += img.len();
        }
        let mut file = with(&entries);
        for img in images {
            file.extend_from_slice(img);
        }
        file
    }

    fn embedded_findings<'a>(a: &'a FileAudit<'a>) -> Vec<&'a EmbeddedSummary> {
        a.findings_in(Category::EmbeddedImage)
            .map(|f| f.embedded.as_ref().expect("an embedded summary"))
            .collect()
    }

    fn holds_of(e: &EmbeddedSummary) -> Vec<Category> {
        e.holds.categories().collect()
    }

    #[test]
    fn embedded_image_finding_carries_range_ffd8_flag_and_holds() {
        let make = build(&[(vec![ascii(0x010F, b"Inner")], None)], &[]);
        let xmp = seg(0xE1, b"http://ns.adobe.com/xap/1.0/\0<x:xmpmeta/>");
        let inner = jpeg(&[exif_seg(&make), xmp]);
        let plain = jpeg(&[]);
        let file = mpo(&[], &[inner.clone(), plain.clone()]);
        let a = audit(&file);
        assert!(a.health().clean, "{:?}", a.health());
        let embedded = embedded_findings(&a);
        assert_eq!(embedded.len(), 2);
        let first_at = file.len() - plain.len() - inner.len();
        assert_eq!(
            (embedded[0].offset, embedded[0].length),
            (first_at, inner.len())
        );
        assert_eq!(
            (embedded[1].offset, embedded[1].length),
            (first_at + inner.len(), plain.len())
        );
        assert!(embedded.iter().all(|e| e.starts_with_ffd8));
        assert_eq!(
            holds_of(embedded[0]),
            vec![Category::Xmp, Category::CameraModel]
        );
        assert!(embedded[1].holds.is_empty());
        // The item is a region of kind EmbeddedImage whose length is the
        // image's; the inner items are not in the outer inventory.
        let kinds: Vec<(RegionKind, usize)> = a
            .items()
            .iter()
            .filter_map(|i| match i.kind() {
                ItemKind::Region { kind, len, .. } => Some((kind, len)),
                ItemKind::Tag { .. } => None,
            })
            .collect();
        assert_eq!(
            kinds,
            vec![
                (RegionKind::EmbeddedImage, inner.len()),
                (RegionKind::EmbeddedImage, plain.len())
            ]
        );
        assert_eq!(a.items().len(), 2);
        assert_eq!(a.max_severity(), Some(Severity::Medium));
        // Only embedded-image findings carry the summary.
        assert!(a
            .findings()
            .iter()
            .all(|f| f.embedded.is_some() == (f.category == Category::EmbeddedImage)));
    }

    #[test]
    fn holds_is_the_inner_audit_with_embedded_location_off() {
        // The inner image has a comment, an APP10 segment, a GPS-bearing Exif
        // and a 5-byte trailer: FreeText, OtherSegment (twice over) and Gps.
        let gps = build(
            &[(vec![ptr(0x8825, 1)], None), (vec![ascii(1, b"N")], None)],
            &[],
        );
        let mut inner = jpeg(&[
            exif_seg(&gps),
            seg(0xFE, b"note"),
            seg(0xEA, b"AROT\0\0"),
            seg(0xE0, b"JFIF\0\x01\x01\0\0\x01\0\x01\0\0"),
        ]);
        inner.extend_from_slice(&[1, 2, 3, 4, 5]);
        let a = audit(&mpo(&[], &[inner]));
        let e = embedded_findings(&a);
        assert_eq!(e.len(), 1);
        // In Category::ALL order.
        assert_eq!(
            holds_of(e[0]),
            vec![Category::Gps, Category::FreeText, Category::OtherSegment]
        );
        // `holds` is exactly the categories the ordinary audit of those bytes
        // finds (it has no embedded image of its own here).
        let own = audit(&{
            let mut v = jpeg(&[
                exif_seg(&gps),
                seg(0xFE, b"note"),
                seg(0xEA, b"AROT\0\0"),
                seg(0xE0, b"JFIF\0\x01\x01\0\0\x01\0\x01\0\0"),
            ]);
            v.extend_from_slice(&[1, 2, 3, 4, 5]);
            v
        });
        let want: Vec<Category> = Category::ALL
            .into_iter()
            .filter(|c| own.findings_in(*c).next().is_some())
            .collect();
        assert_eq!(holds_of(e[0]), want);
    }

    #[test]
    fn an_inner_mpf_does_not_recurse() {
        // The inner image carries its own MPF index (pointing at a GPS image
        // past its end): with location off that segment is an OtherSegment and
        // the GPS image is neither located nor looked into.
        let gps = build(
            &[(vec![ptr(0x8825, 1)], None), (vec![ascii(1, b"N")], None)],
            &[],
        );
        let deep = jpeg(&[exif_seg(&gps)]);
        let inner = mpo(&[], std::slice::from_ref(&deep));
        // The inner file really does hold a GPS image of its own.
        assert_eq!(embedded_findings(&audit(&inner)).len(), 1);
        let outer = mpo(&[], &[inner]);
        let a = audit(&outer);
        let e = embedded_findings(&a);
        assert_eq!(e.len(), 1, "one level only");
        assert_eq!(holds_of(e[0]), vec![Category::OtherSegment]);
        assert!(!e[0].holds.contains(Category::EmbeddedImage));
        assert!(!e[0].holds.contains(Category::Gps));
        assert!(a.findings_in(Category::Gps).next().is_none());
    }

    #[test]
    fn holds_of_an_image_with_no_metadata_is_empty() {
        let a = audit(&mpo(&[], &[jpeg(&[])]));
        let e = embedded_findings(&a);
        assert_eq!(e.len(), 1);
        assert!(e[0].holds.is_empty());
        assert_eq!(holds_of(e[0]), Vec::<Category>::new());
    }

    #[test]
    fn a_comment_is_a_free_text_finding() {
        let a = audit(&jpeg(&[seg(0xFE, b"typed by a person")]));
        assert!(a.health().clean);
        assert_eq!(cats(&a), vec![Category::FreeText]);
        assert_eq!(a.findings()[0].category.severity(), Severity::Medium);
        match a.items()[0].kind() {
            ItemKind::Region { kind, len, segment } => {
                assert_eq!(kind, RegionKind::Comment);
                assert_eq!((len, segment), (17, 0));
            }
            ItemKind::Tag { .. } => panic!("a tag"),
        }
        assert!(a.findings()[0].embedded.is_none());
    }

    #[test]
    fn other_app_and_trailer_are_other_segment_findings_in_container_order() {
        let mut v = jpeg(&[seg(0xEA, b"AROT\0"), seg(0xFE, b"c")]);
        let eoi_end = v.len();
        v.extend_from_slice(&[9u8; 313]);
        let a = audit(&v);
        assert_eq!(
            cats(&a),
            vec![
                Category::OtherSegment,
                Category::FreeText,
                Category::OtherSegment
            ]
        );
        let kinds: Vec<RegionKind> = a
            .items()
            .iter()
            .filter_map(|i| match i.kind() {
                ItemKind::Region { kind, .. } => Some(kind),
                ItemKind::Tag { .. } => None,
            })
            .collect();
        assert_eq!(
            kinds,
            vec![
                RegionKind::OtherApp,
                RegionKind::Comment,
                RegionKind::Trailer
            ]
        );
        // The trailer lies after the scan; its pseudo segment is one past the
        // last real one.
        let last = a.items().last().unwrap();
        assert!(last.origin().after_scan);
        assert_eq!(last.origin().segment, Some(3));
        assert_eq!(
            match last.kind() {
                ItemKind::Region { len, .. } => len,
                ItemKind::Tag { .. } => 0,
            },
            313
        );
        let _ = eoi_end;
    }

    #[test]
    fn a_bare_tiff_has_none_of_the_new_regions() {
        let t = build(&[(vec![ascii(0x010F, b"Cam")], None)], &[]);
        let a = audit(&t);
        assert!(a
            .items()
            .iter()
            .all(|i| matches!(i.kind(), ItemKind::Tag { .. })));
        for c in [Category::EmbeddedImage, Category::OtherSegment] {
            assert!(a.findings_in(c).next().is_none());
        }
    }
}
