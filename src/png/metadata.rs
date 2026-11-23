//! Exif and located metadata regions from a PNG (SPEC-029, DEC-018).
//!
//! [`read`] walks the chunks ([`Png::parse`], DEC-017) and then extracts, in
//! chunk order, from the recorded chunks **and from the chunks that follow
//! `IEND`**:
//!
//! * one [`PngExifBlock`] per `eXIf` chunk (the first is primary; at most
//!   [`MAX_PNG_EXIF_BLOCKS`] are parsed), each holding the tolerant TIFF
//!   [`Container`] the unchanged TIFF engine made of the payload, or the fatal
//!   TIFF error as data (DEC-006). A payload that starts `Exif\0\0` is read
//!   with that header skipped and one [`Error::ImproperExifHeader`] issue;
//! * located, never decoded or inflated, [`PngRegion`]s: `Xmp`, `Icc`,
//!   `RawProfile` and `Text`, each with its small header read and its data
//!   borrowed from the input.
//!
//! Nothing is inflated and no hex is decoded (DEC-018): a region that is
//! `compressed`, and every raw profile, is metadata that is located but not
//! readable here.
//!
//! # Trailer chunks
//!
//! After the walk the bytes past `IEND` are scanned with the walker's own
//! chunk framing ([`super::frame`]): length, type and data must fit, the CRC is
//! not checked. The scan stops silently at the first chunk that does not fit and
//! examines at most [`MAX_PNG_TRAILER_CHUNKS`] chunks. A trailer that is not
//! chunks yields nothing and no issue.
//!
//! # Extraction issues and halts
//!
//! Every extraction issue goes through [`Extract::record`]. When the walk was
//! halted (DEC-017) nothing more is recorded: the halting issue is always the
//! last one. The located regions and blocks are still reported; only the issues
//! are suppressed. This is the JPEG rule (DEC-006) applied to every halt, not
//! just the caps. Extraction issues are at most two per chunk (`ExtraExif` and
//! `ImproperExifHeader`), over at most [`super::MAX_PNG_CHUNKS`] walked chunks
//! and [`MAX_PNG_TRAILER_CHUNKS`] trailer chunks, plus one `TooManyRegions`,
//! so the PNG-layer total is bounded by constants.
//!
//! Provenance (DEC-003, class 1): net-new code written from the PNG
//! Specification (W3C, Third Edition: `eXIf`, `tEXt`, `zTXt`, `iTXt`, `iCCP`).
//! No ExifTool source is read.

use crate::error::Error;
use crate::tiff::{Container, ParseIssue};

use super::{frame, kind_of, ChunkKind, Frame, Png, PngIssue};

/// Most `eXIf` chunks parsed from one PNG. Further ones still cost an
/// [`Error::ExtraExif`] issue but get no [`PngExifBlock`] (bounded work).
pub const MAX_PNG_EXIF_BLOCKS: usize = 16;

/// Most chunks the scan of the bytes after `IEND` examines.
pub const MAX_PNG_TRAILER_CHUNKS: usize = 4096;

/// Most regions one PNG reports. The next one costs a single
/// [`Error::TooManyRegions`] issue and extraction stops.
pub const MAX_PNG_REGIONS: usize = 65_536;

/// The longest keyword or profile name the PNG specification allows.
const MAX_KEYWORD: usize = 79;

/// The header a JPEG APP1 Exif segment carries and a PNG `eXIf` chunk must not.
const EXIF_PREFIX: &[u8] = b"Exif\0\0";

/// The text keyword that marks XMP in `iTXt` and `tEXt`.
const XMP_KEYWORD: &[u8] = b"XML:com.adobe.xmp";

/// The text keyword prefixes of an ImageMagick-style raw profile.
///
/// Exactly these two spellings, measured on ExifTool 13.55 (3 text chunk
/// kinds, 5 prefix cases, 14 names): it decodes `Raw profile type exif` and
/// `raw profile type exif` (the all-lower-case form is looked up in lower
/// case), and nothing else. `Raw Profile Type`, `RAW PROFILE TYPE`, a doubled
/// or missing trailing space, and any other mix print an ordinary text tag.
/// The profile name after the prefix does not change which prefixes count.
const RAW_PROFILE_PREFIXES: [&[u8]; 2] = [b"Raw profile type ", b"raw profile type "];

/// Bytes of chunk framing before the data: length and type.
const CHUNK_HEAD: usize = 8;

/// One `eXIf` chunk's payload, handed to the tolerant TIFF engine.
///
/// Every offset inside [`PngExifBlock::container`] (IFD offsets, issue offsets)
/// is relative to the start of [`PngExifBlock::tiff`], not to the PNG. The file
/// offset of a TIFF-relative offset `o` is `tiff_at() + o`.
#[derive(Debug, Clone)]
pub struct PngExifBlock<'a> {
    offset: usize,
    tiff_at: usize,
    tiff: &'a [u8],
    container: Result<Container<'a>, Error>,
    after_idat: bool,
    after_iend: bool,
}

impl<'a> PngExifBlock<'a> {
    /// Input offset of the chunk's length field. Trailer chunks are not in
    /// [`Png::chunks`], so this is an offset, never an index.
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Input offset of the first byte of [`PngExifBlock::tiff`].
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
    /// PNG failure.
    pub fn container(&self) -> Result<&Container<'a>, &Error> {
        self.container.as_ref()
    }

    /// Whether the chunk comes after the first `IDAT`.
    pub fn after_idat(&self) -> bool {
        self.after_idat
    }

    /// Whether the chunk is one of those after `IEND`.
    pub fn after_iend(&self) -> bool {
        self.after_iend
    }
}

/// What a chunk located by type alone is (SPEC-037, DEC-021 clause 4, measured
/// on ExifTool 13.55). These are the chunk types besides `eXIf`, text and `iCCP`
/// that ExifTool reads as metadata or as a personal string; the library locates
/// them and decodes none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChunkClass {
    /// `tIME`, a modification time of any length (ExifTool prints
    /// `PNG:ModifyDate`, also for a `tIME` after `IEND`).
    Time,
    /// A chunk whose data ExifTool reads as a TIFF/Exif block: `zxIf` (a
    /// `\0`, a 4-byte length and a zlib stream, or a bare TIFF) and every
    /// other ASCII case of `zxIf` and of `eXIf` (ExifTool matches those two
    /// types case-insensitively; the exact `eXIf` is the walked Exif chunk).
    ExifCarrier,
    /// A chunk whose data ExifTool reads as XMP or XML tags: `tXMP` (raw XMP),
    /// `meTa` (XML, also UTF-16) and `seAl`.
    XmlCarrier,
    /// `caBX`, a JUMBF (C2PA) box.
    Jumbf,
    /// A chunk ExifTool prints as a binary tag that is not Exif or XMP and not
    /// image structure: `dSIG`, `cpIp` (OLE information), `gIFx`, `gIFg`,
    /// `gIFt`, `fRAc`.
    Other,
}

/// The class of a chunk type, or `None` for every other type (including the
/// image-structure chunks ExifTool also prints, such as `pHYs` or `gAMA`).
/// Case matters for every type but two: `tXMP` is XMP and `txmp` is nothing
/// (measured), but ExifTool matches `zxIf` and `eXIf` case-insensitively (all
/// 32 spellings decode, with a "chunk should be eXIf" or "should be zxIf"
/// warning), so any ASCII case of those two is an [`ChunkClass::ExifCarrier`].
/// The exact `eXIf` is the walked Exif chunk and is never classified here.
pub fn chunk_class(ty: [u8; 4]) -> Option<ChunkClass> {
    let folded = ty.map(|b| b.to_ascii_lowercase());
    if &folded == b"zxif" || &folded == b"exif" {
        return Some(ChunkClass::ExifCarrier);
    }
    match &ty {
        b"tIME" => Some(ChunkClass::Time),
        b"tXMP" | b"meTa" | b"seAl" => Some(ChunkClass::XmlCarrier),
        b"caBX" => Some(ChunkClass::Jumbf),
        b"dSIG" | b"cpIp" | b"gIFx" | b"gIFg" | b"gIFt" | b"fRAc" => Some(ChunkClass::Other),
        _ => None,
    }
}

/// A located, not decoded, metadata region. Borrowed; never interpreted.
///
/// `offset` is the input offset of the chunk's length field. `data` starts
/// exactly after the chunk's header fields and runs to the end of the chunk's
/// data. Names and keywords are the raw bytes (Latin-1 by the specification),
/// not text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PngRegion<'a> {
    /// XMP: an `iTXt`, `tEXt` or `zTXt` chunk whose keyword is `XML:com.adobe.xmp`.
    Xmp {
        /// Input offset of the chunk.
        offset: usize,
        /// The `iTXt` flag is not 0, or the chunk is `zTXt` (`false` for
        /// `tEXt`). When true `data`
        /// is a zlib stream that is not inflated.
        compressed: bool,
        /// The text field.
        data: &'a [u8],
        /// The chunk is after the first `IDAT`.
        after_idat: bool,
        /// The chunk is after `IEND`.
        after_iend: bool,
    },
    /// An `iCCP` chunk: profile name, compression method, zlib data.
    Icc {
        /// Input offset of the chunk.
        offset: usize,
        /// The profile name: the bytes before the first NUL, of any length
        /// (the specification allows 1 to 79; an empty or longer name is
        /// located with a [`Error::BadIccpChunk`] issue, as ExifTool reads it).
        name: &'a [u8],
        /// The compression-method byte as found (0 is zlib/deflate).
        method: u8,
        /// The compressed profile, not inflated.
        data: &'a [u8],
        /// The chunk is after the first `IDAT`.
        after_idat: bool,
        /// The chunk is after `IEND`.
        after_iend: bool,
    },
    /// An ImageMagick-style `Raw profile type <name>` text chunk (the keyword
    /// is `Raw profile type ` or the all-lower-case `raw profile type `, the
    /// two spellings ExifTool decodes). Its text is
    /// hex, and zlib-compressed first when `compressed`; neither is decoded.
    RawProfile {
        /// Input offset of the chunk.
        offset: usize,
        /// The profile name after the keyword prefix (`exif`, `xmp`, `icc`,
        /// `iptc`, `APP1`, or anything else), possibly empty.
        profile: &'a [u8],
        /// `zTXt`, or `iTXt` with its compression flag set.
        compressed: bool,
        /// The text field.
        data: &'a [u8],
        /// The chunk is after the first `IDAT`.
        after_idat: bool,
        /// The chunk is after `IEND`.
        after_iend: bool,
    },
    /// A chunk located by its type alone (see [`ChunkClass`]): `tIME` and the
    /// carriers of metadata the library cannot read. `data` is the whole chunk
    /// data, possibly empty.
    Chunk {
        /// Input offset of the chunk.
        offset: usize,
        /// The four type bytes.
        chunk_type: [u8; 4],
        /// What the type is.
        class: ChunkClass,
        /// The chunk's data.
        data: &'a [u8],
        /// The chunk is after the first `IDAT`.
        after_idat: bool,
        /// The chunk is after `IEND`.
        after_iend: bool,
    },
    /// Any other `tEXt`, `zTXt` or `iTXt` chunk.
    Text {
        /// Input offset of the chunk.
        offset: usize,
        /// The keyword: 1 to 79 bytes, or any other length with a
        /// [`Error::BadTextChunk`] issue (an empty keyword, or 80 bytes or more).
        keyword: &'a [u8],
        /// `zTXt`, or `iTXt` with its compression flag set.
        compressed: bool,
        /// The text field.
        data: &'a [u8],
        /// The chunk is after the first `IDAT`.
        after_idat: bool,
        /// The chunk is after `IEND`.
        after_iend: bool,
    },
}

impl<'a> PngRegion<'a> {
    /// Input offset of the region's chunk.
    pub fn offset(&self) -> usize {
        match *self {
            PngRegion::Xmp { offset, .. }
            | PngRegion::Icc { offset, .. }
            | PngRegion::RawProfile { offset, .. }
            | PngRegion::Chunk { offset, .. }
            | PngRegion::Text { offset, .. } => offset,
        }
    }

    /// The located bytes, borrowed from the input.
    pub fn data(&self) -> &'a [u8] {
        match *self {
            PngRegion::Xmp { data, .. }
            | PngRegion::Icc { data, .. }
            | PngRegion::RawProfile { data, .. }
            | PngRegion::Chunk { data, .. }
            | PngRegion::Text { data, .. } => data,
        }
    }

    /// Whether the chunk comes after the first `IDAT`.
    pub fn after_idat(&self) -> bool {
        match *self {
            PngRegion::Xmp { after_idat, .. }
            | PngRegion::Icc { after_idat, .. }
            | PngRegion::RawProfile { after_idat, .. }
            | PngRegion::Chunk { after_idat, .. }
            | PngRegion::Text { after_idat, .. } => after_idat,
        }
    }

    /// Whether the chunk is one of those after `IEND`.
    pub fn after_iend(&self) -> bool {
        match *self {
            PngRegion::Xmp { after_iend, .. }
            | PngRegion::Icc { after_iend, .. }
            | PngRegion::RawProfile { after_iend, .. }
            | PngRegion::Chunk { after_iend, .. }
            | PngRegion::Text { after_iend, .. } => after_iend,
        }
    }
}

/// An issue with its layer named. Layers are never merged into one flat type
/// (DEC-006).
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum AnyPngIssue<'a> {
    /// A PNG-layer issue (the walk's or the extraction's).
    Png(&'a PngIssue),
    /// A TIFF-layer issue of one Exif block.
    ///
    /// The issue's offsets are relative to the start of the block's TIFF data
    /// ([`PngExifBlock::tiff`]); add [`PngExifBlock::tiff_at`] for a file
    /// offset.
    Exif {
        /// Index into [`PngMetadata::exif_blocks`].
        block: usize,
        /// The block's own issue; its offsets are TIFF-relative (see above).
        issue: &'a ParseIssue,
    },
}

/// The result of [`read`]: the walk, the Exif blocks, the located regions and
/// the layered issue view.
#[derive(Debug, Clone)]
pub struct PngMetadata<'a> {
    png: Png<'a>,
    exif_blocks: Vec<PngExifBlock<'a>>,
    regions: Vec<PngRegion<'a>>,
    png_issues: Vec<PngIssue>,
}

impl<'a> PngMetadata<'a> {
    /// The chunk walk.
    pub fn png(&self) -> &Png<'a> {
        &self.png
    }

    /// One block per parsed `eXIf` chunk, in file order, at most
    /// [`MAX_PNG_EXIF_BLOCKS`].
    pub fn exif_blocks(&self) -> &[PngExifBlock<'a>] {
        &self.exif_blocks
    }

    /// The primary Exif: the first block whose container parsed.
    pub fn exif(&self) -> Option<&Container<'a>> {
        self.exif_blocks.iter().find_map(|b| b.container().ok())
    }

    /// The located regions, in chunk order (walked chunks, then trailer
    /// chunks), at most [`MAX_PNG_REGIONS`].
    pub fn regions(&self) -> &[PngRegion<'a>] {
        &self.regions
    }

    /// The walker's issues, then the extraction's, in order.
    pub fn png_issues(&self) -> &[PngIssue] {
        &self.png_issues
    }

    /// Every issue with its layer named: the PNG layer first, then each Exif
    /// block's own issues, in block order. A fatal TIFF error is not an issue.
    pub fn issues(&self) -> impl Iterator<Item = AnyPngIssue<'_>> {
        let png = self.png_issues.iter().map(AnyPngIssue::Png);
        let exif = self.exif_blocks.iter().enumerate().flat_map(|(block, b)| {
            b.container.iter().flat_map(move |c| {
                c.issues()
                    .iter()
                    .map(move |issue| AnyPngIssue::Exif { block, issue })
            })
        });
        png.chain(exif)
    }

    /// Clean only when the PNG layer has no issue and every Exif block parsed
    /// and is itself clean.
    pub fn is_clean(&self) -> bool {
        self.png_issues.is_empty()
            && self
                .exif_blocks
                .iter()
                .all(|b| matches!(&b.container, Ok(c) if c.is_clean()))
    }
}

/// Walk `data` as a PNG and extract its Exif blocks and located regions.
/// `Err` exactly when [`Png::parse`] is (not a PNG at all).
pub fn read(data: &[u8]) -> Result<PngMetadata<'_>, Error> {
    let png = Png::parse(data)?;
    Ok(extract(png, data))
}

/// The extraction's mutable state. Every issue goes through
/// [`Extract::record`], so no code path can record after a halt.
struct Extract<'a> {
    /// The walker's issues first, then the extraction's.
    issues: Vec<PngIssue>,
    /// The walk was halted: record no issue (DEC-018, DEC-006).
    halted: bool,
    /// The region cap was hit: examine no more chunks.
    stopped: bool,
    blocks: Vec<PngExifBlock<'a>>,
    regions: Vec<PngRegion<'a>>,
    exif_seen: usize,
}

/// Where a chunk is, for the flags its results carry.
#[derive(Clone, Copy)]
struct Place {
    offset: usize,
    after_idat: bool,
    after_iend: bool,
}

/// The fields of a text chunk, after its header.
struct TextParts<'a> {
    keyword: &'a [u8],
    compressed: bool,
    value: &'a [u8],
    /// The chunk is malformed but still locatable (DEC-018): an empty keyword, a
    /// keyword of 80 bytes or more, or an `iTXt` flag other than 0 or 1.
    bad: bool,
    /// An `iTXt` whose language tag is not empty. Measured (ExifTool 13.55,
    /// both compression flags, tags `en-US`, `x`, `EN`): such a chunk with the
    /// keyword `XML:com.adobe.xmp` is NOT XMP, only a plain text tag; the
    /// translated keyword does not matter (an empty tag and a translated
    /// keyword is XMP). So it is located as [`PngRegion::Text`].
    tagged: bool,
}

/// The header of a `tEXt`, `zTXt` or `iTXt` chunk. `None` when there is nothing
/// to locate: no NUL or a truncated header. An empty keyword, a keyword of 80
/// bytes or more, or an `iTXt` flag above 1, is located with `bad` set.
fn text_parts(kind: ChunkKind, data: &[u8]) -> Option<TextParts<'_>> {
    let end = data.iter().position(|&b| b == 0)?;
    // An empty keyword is read by ExifTool 13.55 as a tag named `Tag` (all three
    // chunk types, compressed or not), so it is located and flagged like a
    // keyword of 80 bytes or more.
    let mut bad = end == 0 || end > MAX_KEYWORD;
    let keyword = data.get(..end)?;
    let rest = data.get(end.checked_add(1)?..)?;
    match kind {
        ChunkKind::Text => Some(TextParts {
            keyword,
            compressed: false,
            value: rest,
            bad,
            tagged: false,
        }),
        // A method byte follows the keyword; the text is always compressed.
        ChunkKind::Ztxt => Some(TextParts {
            keyword,
            compressed: true,
            value: rest.get(1..)?,
            bad,
            tagged: false,
        }),
        ChunkKind::Itxt => {
            let (&flag, rest) = rest.split_first()?;
            // Any flag other than 0 is read as compressed, as ExifTool does.
            let compressed = flag != 0;
            bad |= flag > 1;
            let (_method, rest) = rest.split_first()?;
            // Language tag, then translated keyword, each NUL-terminated.
            let lang_end = rest.iter().position(|&b| b == 0)?;
            let rest = rest.get(lang_end.checked_add(1)?..)?;
            let translated_end = rest.iter().position(|&b| b == 0)?;
            let value = rest.get(translated_end.checked_add(1)?..)?;
            Some(TextParts {
                keyword,
                compressed,
                value,
                bad,
                tagged: lang_end > 0,
            })
        }
        _ => None,
    }
}

impl<'a> Extract<'a> {
    /// The one place an extraction issue is recorded.
    fn record(&mut self, error: Error, offset: usize) {
        if self.halted {
            return;
        }
        self.issues.push(PngIssue::new(error, offset));
    }

    /// Add a region, or stop extraction when the cap is reached.
    fn push_region(&mut self, region: PngRegion<'a>, offset: usize) {
        if self.regions.len() >= MAX_PNG_REGIONS {
            self.record(
                Error::TooManyRegions {
                    limit: MAX_PNG_REGIONS,
                },
                offset,
            );
            self.stopped = true;
            return;
        }
        self.regions.push(region);
    }

    /// Extract from one chunk, by its kind. Other kinds are ignored.
    fn chunk(&mut self, kind: ChunkKind, ty: [u8; 4], data: &'a [u8], place: Place) {
        let Place {
            offset,
            after_idat,
            after_iend,
        } = place;
        match kind {
            ChunkKind::Exif => {
                if self.exif_seen >= 1 {
                    self.record(Error::ExtraExif { at: offset }, offset);
                }
                if self.exif_seen < MAX_PNG_EXIF_BLOCKS {
                    let body_at = offset.saturating_add(CHUNK_HEAD);
                    let (tiff, tiff_at) = match data.strip_prefix(EXIF_PREFIX) {
                        Some(rest) => {
                            self.record(Error::ImproperExifHeader { at: offset }, offset);
                            (rest, body_at.saturating_add(EXIF_PREFIX.len()))
                        }
                        None => (data, body_at),
                    };
                    self.blocks.push(PngExifBlock {
                        offset,
                        tiff_at,
                        tiff,
                        container: Container::parse(tiff),
                        after_idat,
                        after_iend,
                    });
                }
                self.exif_seen = self.exif_seen.saturating_add(1);
            }
            ChunkKind::Itxt | ChunkKind::Text | ChunkKind::Ztxt => {
                let Some(TextParts {
                    keyword,
                    compressed,
                    value,
                    bad,
                    tagged,
                }) = text_parts(kind, data)
                else {
                    self.record(Error::BadTextChunk { at: offset }, offset);
                    return;
                };
                if bad {
                    self.record(Error::BadTextChunk { at: offset }, offset);
                }
                let region = if keyword == XMP_KEYWORD && !tagged {
                    PngRegion::Xmp {
                        offset,
                        compressed,
                        data: value,
                        after_idat,
                        after_iend,
                    }
                } else if let Some(profile) = RAW_PROFILE_PREFIXES
                    .iter()
                    .find_map(|prefix| keyword.strip_prefix(*prefix))
                {
                    PngRegion::RawProfile {
                        offset,
                        profile,
                        compressed,
                        data: value,
                        after_idat,
                        after_iend,
                    }
                } else {
                    PngRegion::Text {
                        offset,
                        keyword,
                        compressed,
                        data: value,
                        after_idat,
                        after_iend,
                    }
                };
                self.push_region(region, offset);
            }
            ChunkKind::Iccp => {
                // The name is the bytes before the first NUL, of any length
                // (measured, ExifTool 13.55: names of 0, 1, 79, 80, 81 and 200
                // bytes all give the ICC header tags; a chunk with no NUL or no
                // method byte gives none). Empty or 80 bytes and more is
                // malformed but located, like a long text keyword.
                let parsed = data.iter().position(|&b| b == 0).and_then(|end| {
                    let name = data.get(..end)?;
                    let rest = data.get(end.checked_add(1)?..)?;
                    let (&method, body) = rest.split_first()?;
                    Some((name, method, body))
                });
                let Some((name, method, body)) = parsed else {
                    self.record(Error::BadIccpChunk { at: offset }, offset);
                    return;
                };
                if name.is_empty() || name.len() > MAX_KEYWORD {
                    self.record(Error::BadIccpChunk { at: offset }, offset);
                }
                self.push_region(
                    PngRegion::Icc {
                        offset,
                        name,
                        method,
                        data: body,
                        after_idat,
                        after_iend,
                    },
                    offset,
                );
            }
            _ => {
                if let Some(class) = chunk_class(ty) {
                    self.push_region(
                        PngRegion::Chunk {
                            offset,
                            chunk_type: ty,
                            class,
                            data,
                            after_idat,
                            after_iend,
                        },
                        offset,
                    );
                }
            }
        }
    }
}

fn extract<'a>(png: Png<'a>, data: &'a [u8]) -> PngMetadata<'a> {
    let mut ex = Extract {
        issues: png.issues().to_vec(),
        halted: png.halted(),
        stopped: false,
        blocks: Vec::new(),
        regions: Vec::new(),
        exif_seen: 0,
    };
    let mut seen_idat = false;
    for c in png.chunks() {
        if ex.stopped {
            break;
        }
        ex.chunk(
            c.kind(),
            c.chunk_type(),
            c.data(),
            Place {
                offset: c.offset(),
                after_idat: c.after_idat(),
                after_iend: false,
            },
        );
        seen_idat |= c.kind() == ChunkKind::Idat;
    }

    // The trailer starts where the (borrowed) trailer slice starts; with no
    // `IEND` it is empty and the first framing attempt stops the scan.
    let mut at = data.len().saturating_sub(png.trailer().len());
    for _ in 0..MAX_PNG_TRAILER_CHUNKS {
        if ex.stopped {
            break;
        }
        let Frame::Chunk(framed) = frame(data, at) else {
            break;
        };
        let kind = kind_of(framed.ty);
        ex.chunk(
            kind,
            framed.ty,
            framed.payload,
            Place {
                offset: at,
                after_idat: seen_idat,
                after_iend: true,
            },
        );
        seen_idat |= kind == ChunkKind::Idat;
        // A framed chunk is at least 12 bytes, so this always advances.
        at = framed.end;
    }

    PngMetadata {
        png,
        exif_blocks: ex.blocks,
        regions: ex.regions,
        png_issues: ex.issues,
    }
}
