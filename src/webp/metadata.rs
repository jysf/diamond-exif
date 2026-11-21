//! Exif and located metadata regions from a WebP (SPEC-030, DEC-019).
//!
//! [`read`] walks the chunks ([`Webp::parse`], DEC-019) and then extracts, in
//! chunk order, from the recorded top-level chunks:
//!
//! * one [`WebpExifBlock`] per `EXIF` chunk (the first is primary; at most
//!   [`MAX_WEBP_EXIF_BLOCKS`] are parsed), each holding the tolerant TIFF
//!   [`Container`] the unchanged TIFF engine made of the payload, or the fatal
//!   TIFF error as data (DEC-006). A payload that starts `Exif\0\0` is read
//!   with that header skipped and one [`Error::ImproperExifHeader`] issue;
//! * located, never decoded, [`WebpRegion`]s: `Xmp` (from `XMP ` and `XMP\0`
//!   chunks) and `Icc` (from `ICCP` chunks), their data borrowed from the
//!   input. WebP has no compression inside these chunks.
//!
//! Chunks inside an `ANMF` frame payload are never walked, so metadata there is
//! not found; an `EXIF` chunk after an `ANMF` chunk at top level is.
//!
//! # Extraction issues and halts
//!
//! Every extraction issue goes through [`Extract::record`]. When the walk was
//! halted (DEC-019) nothing more is recorded: the halting issue is always the
//! last one. The located regions and blocks are still reported; only the issues
//! are suppressed (the DEC-018 and DEC-006 rule). Extraction issues are at most
//! two per chunk (`ExtraExif` and `ImproperExifHeader`), over at most
//! [`super::MAX_WEBP_CHUNKS`] chunks, so the WebP-layer total is bounded by
//! constants. Regions are bounded by the chunk count and need no cap of their
//! own.
//!
//! Provenance (DEC-003, class 1): net-new code written from the WebP Container
//! Specification (Google: `EXIF`, `XMP `, `ICCP`). No ExifTool source is read.

use crate::error::Error;
use crate::tiff::{Container, ParseIssue};

use super::{ChunkKind, Webp, WebpIssue};

/// Most `EXIF` chunks parsed from one WebP. Further ones still cost an
/// [`Error::ExtraExif`] issue but get no [`WebpExifBlock`] (bounded work).
pub const MAX_WEBP_EXIF_BLOCKS: usize = 16;

/// The header a JPEG APP1 Exif segment carries and a WebP `EXIF` chunk must
/// not.
const EXIF_PREFIX: &[u8] = b"Exif\0\0";

/// Bytes of chunk framing before the data: code and size.
const CHUNK_HEAD: usize = 8;

/// One `EXIF` chunk's payload, handed to the tolerant TIFF engine.
///
/// Every offset inside [`WebpExifBlock::container`] (IFD offsets, issue
/// offsets) is relative to the start of [`WebpExifBlock::tiff`], not to the
/// WebP. The file offset of a TIFF-relative offset `o` is `tiff_at() + o`.
#[derive(Debug, Clone)]
pub struct WebpExifBlock<'a> {
    offset: usize,
    tiff_at: usize,
    tiff: &'a [u8],
    container: Result<Container<'a>, Error>,
}

impl<'a> WebpExifBlock<'a> {
    /// Input offset of the chunk's four-character code.
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Input offset of the first byte of [`WebpExifBlock::tiff`].
    pub fn tiff_at(&self) -> usize {
        self.tiff_at
    }

    /// The TIFF payload, borrowed from the input (DEC-002): the chunk's data
    /// after an `Exif\0\0` header when there is one.
    pub fn tiff(&self) -> &'a [u8] {
        self.tiff
    }

    /// `Ok`: the tolerant TIFF parse. `Err`: the fatal TIFF error (a bad header
    /// or IFD0, or an empty payload), kept as data rather than promoted to a
    /// WebP failure.
    pub fn container(&self) -> Result<&Container<'a>, &Error> {
        self.container.as_ref()
    }
}

/// A located, not decoded, metadata region. Borrowed; never interpreted.
///
/// `offset` is the input offset of the chunk's four-character code; `data` is
/// the chunk's data without its pad byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum WebpRegion<'a> {
    /// An `XMP ` chunk (or an `XMP\0` one): an XMP packet.
    Xmp {
        /// Input offset of the chunk.
        offset: usize,
        /// The packet bytes.
        data: &'a [u8],
    },
    /// An `ICCP` chunk: an ICC profile.
    Icc {
        /// Input offset of the chunk.
        offset: usize,
        /// The profile bytes.
        data: &'a [u8],
    },
}

impl<'a> WebpRegion<'a> {
    /// Input offset of the region's chunk.
    pub fn offset(&self) -> usize {
        match *self {
            WebpRegion::Xmp { offset, .. } | WebpRegion::Icc { offset, .. } => offset,
        }
    }

    /// The located bytes, borrowed from the input.
    pub fn data(&self) -> &'a [u8] {
        match *self {
            WebpRegion::Xmp { data, .. } | WebpRegion::Icc { data, .. } => data,
        }
    }
}

/// An issue with its layer named. Layers are never merged into one flat type
/// (DEC-006).
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum AnyWebpIssue<'a> {
    /// A WebP-layer issue (the walk's or the extraction's).
    Webp(&'a WebpIssue),
    /// A TIFF-layer issue of one Exif block.
    ///
    /// The issue's offsets are relative to the start of the block's TIFF data
    /// ([`WebpExifBlock::tiff`]); add [`WebpExifBlock::tiff_at`] for a file
    /// offset.
    Exif {
        /// Index into [`WebpMetadata::exif_blocks`].
        block: usize,
        /// The block's own issue; its offsets are TIFF-relative (see above).
        issue: &'a ParseIssue,
    },
}

/// The result of [`read`]: the walk, the Exif blocks, the located regions and
/// the layered issue view.
#[derive(Debug, Clone)]
pub struct WebpMetadata<'a> {
    webp: Webp<'a>,
    exif_blocks: Vec<WebpExifBlock<'a>>,
    regions: Vec<WebpRegion<'a>>,
    webp_issues: Vec<WebpIssue>,
}

impl<'a> WebpMetadata<'a> {
    /// The chunk walk.
    pub fn webp(&self) -> &Webp<'a> {
        &self.webp
    }

    /// One block per parsed `EXIF` chunk, in file order, at most
    /// [`MAX_WEBP_EXIF_BLOCKS`].
    pub fn exif_blocks(&self) -> &[WebpExifBlock<'a>] {
        &self.exif_blocks
    }

    /// The primary Exif: the first block whose container parsed.
    pub fn exif(&self) -> Option<&Container<'a>> {
        self.exif_blocks.iter().find_map(|b| b.container().ok())
    }

    /// The located regions, in chunk order.
    pub fn regions(&self) -> &[WebpRegion<'a>] {
        &self.regions
    }

    /// The walker's issues, then the extraction's, in order.
    pub fn webp_issues(&self) -> &[WebpIssue] {
        &self.webp_issues
    }

    /// Every issue with its layer named: the WebP layer first, then each Exif
    /// block's own issues, in block order. A fatal TIFF error is not an issue.
    pub fn issues(&self) -> impl Iterator<Item = AnyWebpIssue<'_>> {
        let webp = self.webp_issues.iter().map(AnyWebpIssue::Webp);
        let exif = self.exif_blocks.iter().enumerate().flat_map(|(block, b)| {
            b.container.iter().flat_map(move |c| {
                c.issues()
                    .iter()
                    .map(move |issue| AnyWebpIssue::Exif { block, issue })
            })
        });
        webp.chain(exif)
    }

    /// Clean only when the WebP layer has no issue and every Exif block parsed
    /// and is itself clean.
    pub fn is_clean(&self) -> bool {
        self.webp_issues.is_empty()
            && self
                .exif_blocks
                .iter()
                .all(|b| matches!(&b.container, Ok(c) if c.is_clean()))
    }
}

/// Walk `data` as a WebP and extract its Exif blocks and located regions.
/// `Err` exactly when [`Webp::parse`] is (not a WebP at all).
pub fn read(data: &[u8]) -> Result<WebpMetadata<'_>, Error> {
    let webp = Webp::parse(data)?;
    Ok(extract(webp))
}

/// The extraction's mutable state. Every issue goes through
/// [`Extract::record`], so no code path can record after a halt.
struct Extract<'a> {
    /// The walker's issues first, then the extraction's.
    issues: Vec<WebpIssue>,
    /// The walk was halted: record no issue (DEC-019).
    halted: bool,
    blocks: Vec<WebpExifBlock<'a>>,
    regions: Vec<WebpRegion<'a>>,
    exif_seen: usize,
}

impl<'a> Extract<'a> {
    /// The one place an extraction issue is recorded.
    fn record(&mut self, error: Error, offset: usize) {
        if self.halted {
            return;
        }
        self.issues.push(WebpIssue::new(error, offset));
    }

    /// Extract from one chunk, by its kind. Other kinds are ignored.
    fn chunk(&mut self, kind: ChunkKind, data: &'a [u8], offset: usize) {
        match kind {
            ChunkKind::Exif => {
                if self.exif_seen >= 1 {
                    self.record(Error::ExtraExif { at: offset }, offset);
                }
                if self.exif_seen < MAX_WEBP_EXIF_BLOCKS {
                    let body_at = offset.saturating_add(CHUNK_HEAD);
                    let (tiff, tiff_at) = match data.strip_prefix(EXIF_PREFIX) {
                        Some(rest) => {
                            self.record(Error::ImproperExifHeader { at: offset }, offset);
                            (rest, body_at.saturating_add(EXIF_PREFIX.len()))
                        }
                        None => (data, body_at),
                    };
                    self.blocks.push(WebpExifBlock {
                        offset,
                        tiff_at,
                        tiff,
                        container: Container::parse(tiff),
                    });
                }
                self.exif_seen = self.exif_seen.saturating_add(1);
            }
            ChunkKind::Xmp => self.regions.push(WebpRegion::Xmp { offset, data }),
            ChunkKind::Iccp => self.regions.push(WebpRegion::Icc { offset, data }),
            _ => {}
        }
    }
}

fn extract(webp: Webp<'_>) -> WebpMetadata<'_> {
    let mut ex = Extract {
        issues: webp.issues().to_vec(),
        halted: webp.halted(),
        blocks: Vec::new(),
        regions: Vec::new(),
        exif_seen: 0,
    };
    for c in webp.chunks() {
        ex.chunk(c.kind(), c.data(), c.offset());
    }
    WebpMetadata {
        exif_blocks: ex.blocks,
        regions: ex.regions,
        webp_issues: ex.issues,
        webp,
    }
}
