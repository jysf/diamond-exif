//! Exif and located metadata regions from a JPEG (SPEC-010, DEC-006).
//!
//! [`read`] walks the segments ([`Jpeg::parse`]) and then extracts, in segment
//! order and using only the walk's result:
//!
//! * one [`ExifBlock`] per Exif segment (the first is primary; at most
//!   [`MAX_EXIF_BLOCKS`] are parsed), each holding the tolerant TIFF
//!   [`Container`] the unchanged TIFF engine made of the payload after the
//!   six-byte Exif header (`Exif\0` and one unchecked byte, DEC-007), or the
//!   fatal TIFF error;
//! * located, not decoded, [`Region`]s for XMP, extended XMP, ICC chunks, IPTC
//!   (the Photoshop `8BIM` resource `0x0404`) and JUMBF, each with its small
//!   header read and its data borrowed from the input.
//!
//! JPEG-layer problems (the walker's, then the extraction's) and the TIFF-layer
//! problems of each Exif block stay in separate channels (DEC-006); the
//! combined view names the layer ([`AnyIssue`]).
//!
//! # Extraction issues and halts
//!
//! Every extraction issue goes through [`Extract::record`]. When the walk was
//! halted by a cap (`TooManySegments` or `TooManyIssues`) nothing more is
//! recorded: that halting issue is always the last one (DEC-006). The located
//! regions and Exif blocks are still reported; only the issues are suppressed.
//! Extraction issues are bounded by construction (at most one per segment, plus
//! one ICC aggregate), so the JPEG-layer total stays at most
//! [`MAX_JPEG_METADATA_ISSUES`].
//!
//! # ICC chunk consistency
//!
//! Chunks are checked in segment order against the first chunk's `count`. A
//! chunk is inconsistent when its `count` differs from the first's, its `seq`
//! is 0 or above its own `count`, or its `seq` repeats an earlier chunk's. The
//! first inconsistent chunk found is reported (so when the first chunk is
//! simply the odd one out, the second chunk is the one named). If every chunk
//! is individually consistent but their number differs from `count`, the set is
//! incomplete (or too large) and the first chunk, which declared the count, is
//! named. Either way there is exactly one [`Error::IccChunkMismatch`].
//!
//! Provenance (DEC-003, class 1): net-new code written from ITU-T T.81, the
//! Adobe XMP specification part 3, the ICC specification's JPEG embedding, the
//! Photoshop image-resource-block layout and the JPEG XT / JUMBF APP11 header.
//! No ExifTool source is read.

use crate::error::Error;
use crate::reader::{read_u32, ByteOrder};
use crate::tiff::{Container, ParseIssue};

use super::{
    Jpeg, JpegIssue, SegmentKind, EXIF_HEADER_LEN, MAX_JPEG_ISSUES, MAX_JPEG_SEGMENTS,
    SIG_EXTENDED_XMP, SIG_ICC, SIG_IPTC, SIG_JUMBF, SIG_XMP,
};

/// Most Exif segments parsed from one JPEG. Further ones still cost an
/// [`Error::ExtraExif`] issue but get no [`ExifBlock`] (bounded work).
pub const MAX_EXIF_BLOCKS: usize = 16;

/// Most images one MPF index may list, the primary included (DEC-012: at most
/// 16 images, so at most 15 [`Region::EmbeddedImage`] per file). An index entry
/// from this one on costs a single [`Error::TooManyMpfImages`] and is not
/// examined.
pub const MAX_MPF_IMAGES: usize = 16;

/// Most JPEG-layer issues [`JpegMetadata::jpeg_issues`] can hold: the walker's
/// (`MAX_JPEG_ISSUES + 1`) plus the extraction's (at most one per recorded
/// segment, plus one ICC aggregate: `MAX_JPEG_SEGMENTS + 1`) plus the entry
/// issues of the one MPF index that is read (at most one per examined entry,
/// and the too-many issue: `MAX_MPF_IMAGES`).
pub const MAX_JPEG_METADATA_ISSUES: usize =
    (MAX_JPEG_ISSUES + 1) + (MAX_JPEG_SEGMENTS + 1) + MAX_MPF_IMAGES;

/// The MPF segment signature (CIPA DC-007): the four bytes before the TIFF-style
/// MPF header, whose offsets the index entries are relative to.
const SIG_MPF: &[u8] = b"MPF\0";
/// The MP index tag (`MPEntry`): 16 bytes per image.
const TAG_MP_ENTRY: u16 = 0xB002;
/// One MP index entry: attribute (`u32`), size (`u32`), offset (`u32`) and two
/// dependent-image numbers (`u16` each).
const MP_ENTRY_LEN: usize = 16;
/// The APP14 Adobe signature: a colour-transform marker with no free text
/// (benign, DEC-012).
const SIG_ADOBE: &[u8] = b"Adobe\0";

/// The extended-XMP header after the signature: a 32-byte GUID, then the full
/// length and the chunk offset, both big-endian `u32`.
const EXTENDED_XMP_HEADER: usize = 32 + 4 + 4;
/// The ICC chunk header after the signature: sequence number, chunk count.
const ICC_HEADER: usize = 2;
/// The JUMBF header after the `JP` signature: `En` (`u16`) and `Z` (`u32`).
const JUMBF_HEADER: usize = 2 + 4;

/// One parsed Exif segment: where it came from and what the TIFF engine made
/// of it.
///
/// Every offset inside [`ExifBlock::container`] (IFD offsets, issue offsets) is
/// relative to the start of [`ExifBlock::tiff`], not to the JPEG. To get a file
/// offset add `segment.offset() + 10` (see [`AnyIssue::Exif`]).
#[derive(Debug, Clone)]
pub struct ExifBlock<'a> {
    segment: usize,
    tiff_at: usize,
    tiff: &'a [u8],
    container: Result<Container<'a>, Error>,
}

impl<'a> ExifBlock<'a> {
    /// Input offset of the first byte of [`ExifBlock::tiff`]: the segment's
    /// offset plus the marker, the length word and the Exif header (the writer
    /// adds it to every offset the TIFF walk reports, SPEC-021).
    pub fn tiff_at(&self) -> usize {
        self.tiff_at
    }

    /// Index into [`Jpeg::segments`] of the Exif segment.
    pub fn segment(&self) -> usize {
        self.segment
    }

    /// The payload after the six-byte Exif header (`Exif\0` and one byte that
    /// is not checked, DEC-007), borrowed from the input (DEC-002).
    pub fn tiff(&self) -> &'a [u8] {
        self.tiff
    }

    /// `Ok`: the tolerant TIFF parse. `Err`: the fatal TIFF error (a bad header
    /// or IFD0), kept here as data rather than promoted to a JPEG failure.
    pub fn container(&self) -> Result<&Container<'a>, &Error> {
        self.container.as_ref()
    }
}

/// A located, not decoded, non-Exif region. Borrowed; never interpreted.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Region<'a> {
    /// An XMP packet: the payload after the 29-byte signature.
    Xmp {
        /// Index into [`Jpeg::segments`].
        segment: usize,
        /// The packet bytes (may be empty).
        packet: &'a [u8],
    },
    /// One extended-XMP chunk.
    ExtendedXmp {
        /// Index into [`Jpeg::segments`].
        segment: usize,
        /// The 32-byte GUID shared by the chunks of one extended packet.
        guid: &'a [u8],
        /// The full length of the extended packet, big-endian in the file.
        full_len: u32,
        /// This chunk's offset into the extended packet.
        offset: u32,
        /// The chunk's data.
        data: &'a [u8],
    },
    /// One ICC profile chunk.
    Icc {
        /// Index into [`Jpeg::segments`].
        segment: usize,
        /// The chunk's sequence number (1-based).
        seq: u8,
        /// The total number of chunks.
        count: u8,
        /// The chunk's profile bytes.
        data: &'a [u8],
    },
    /// One APP13 `Photoshop 3.0\0` segment (DEC-012: every such segment is a
    /// region, whether or not a `0x0404` resource parses inside it): the
    /// payload after the header, not decoded.
    Iptc {
        /// Index into [`Jpeg::segments`].
        segment: usize,
        /// The segment payload after the header.
        data: &'a [u8],
    },
    /// A JUMBF (JPEG XT) segment. No C2PA detection is claimed (DEC-006).
    Jumbf {
        /// Index into [`Jpeg::segments`].
        segment: usize,
        /// The box instance number `En`.
        en: u16,
        /// The packet sequence number `Z`.
        z: u32,
        /// The bytes after the 6-byte header.
        data: &'a [u8],
    },
    /// One Multi-Picture Format image other than the primary (DEC-012): a
    /// range of the input that starts `FF D8`, found through the MP index.
    EmbeddedImage {
        /// Index into [`Jpeg::segments`] of the MPF segment that lists it.
        segment: usize,
        /// Input offset of the image's first byte.
        offset: usize,
        /// The image's bytes (`size` bytes at `offset`).
        data: &'a [u8],
    },
    /// A `COM` segment: its payload.
    Comment {
        /// Index into [`Jpeg::segments`].
        segment: usize,
        /// The comment bytes.
        data: &'a [u8],
    },
    /// An APPn segment the library does not recognise (DEC-012): not Exif,
    /// XMP, extended XMP, ICC, Photoshop, JUMBF, a parsed MPF, `JFIF\0` or
    /// `Adobe\0`.
    OtherApp {
        /// Index into [`Jpeg::segments`].
        segment: usize,
        /// The marker code (`0xE0..=0xEF`).
        marker: u8,
        /// The segment payload.
        data: &'a [u8],
    },
    /// Bytes after the `EOI` that no embedded image accounts for (DEC-012): one
    /// region per contiguous remainder.
    Trailer {
        /// One past the last segment (`Jpeg::segments().len()`).
        segment: usize,
        /// Input offset of the first byte.
        offset: usize,
        /// The bytes.
        data: &'a [u8],
    },
}

impl<'a> Region<'a> {
    /// Index into [`Jpeg::segments`] of the segment the region came from.
    pub fn segment(&self) -> usize {
        match *self {
            Region::Xmp { segment, .. }
            | Region::ExtendedXmp { segment, .. }
            | Region::Icc { segment, .. }
            | Region::Iptc { segment, .. }
            | Region::Jumbf { segment, .. }
            | Region::EmbeddedImage { segment, .. }
            | Region::Comment { segment, .. }
            | Region::OtherApp { segment, .. }
            | Region::Trailer { segment, .. } => segment,
        }
    }

    /// The region's bytes: the packet or the data.
    pub fn data(&self) -> &'a [u8] {
        match *self {
            Region::Xmp { packet, .. } => packet,
            Region::ExtendedXmp { data, .. }
            | Region::Icc { data, .. }
            | Region::Iptc { data, .. }
            | Region::Jumbf { data, .. }
            | Region::EmbeddedImage { data, .. }
            | Region::Comment { data, .. }
            | Region::OtherApp { data, .. }
            | Region::Trailer { data, .. } => data,
        }
    }
}

/// One 16-byte entry of the MP index: where its `size` and `offset` fields sit
/// in the input, so a writer can patch them in place (DEC-013 part 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MpfEntry {
    index: usize,
    offset: u32,
    size: u32,
    size_field: usize,
    offset_field: usize,
    region: Option<usize>,
}

impl MpfEntry {
    /// The entry's position in the index (entry 0 is the primary image).
    pub fn index(&self) -> usize {
        self.index
    }

    /// The `offset` field as read (relative to [`MpfIndex::header_start`]).
    pub fn offset(&self) -> u32 {
        self.offset
    }

    /// The `size` field as read.
    pub fn size(&self) -> u32 {
        self.size
    }

    /// Input offset of the entry's 4-byte `size` field.
    pub fn size_field(&self) -> usize {
        self.size_field
    }

    /// Input offset of the entry's 4-byte `offset` field.
    pub fn offset_field(&self) -> usize {
        self.offset_field
    }

    /// `Some(i)`: this entry became [`JpegMetadata::regions`]`()[i]`, an
    /// [`Region::EmbeddedImage`]. `None`: the primary (entry 0) or an entry
    /// that was malformed (outside the input, not `FF D8`).
    pub fn region(&self) -> Option<usize> {
        self.region
    }
}

/// The MP index the audit read the embedded images from (DEC-012): the first
/// APP2 `MPF\0` segment that parses. A second MPF segment, or one that does not
/// parse, is never the index. Nothing is copied; the positions are input
/// offsets, checked when they were computed.
#[derive(Debug, Clone)]
pub struct MpfIndex {
    segment: usize,
    header_start: usize,
    big_endian: bool,
    entries: Vec<MpfEntry>,
}

impl MpfIndex {
    /// Index into [`Jpeg::segments`] of the index segment.
    pub fn segment(&self) -> usize {
        self.segment
    }

    /// Input offset of the byte after `MPF\0`; the entries' offsets are
    /// relative to it.
    pub fn header_start(&self) -> usize {
        self.header_start
    }

    /// Whether the index (and so every field in it) is big-endian.
    pub fn big_endian(&self) -> bool {
        self.big_endian
    }

    /// The examined entries in order, at most [`MAX_MPF_IMAGES`] (the ones past
    /// the limit are not examined, as in the audit).
    pub fn entries(&self) -> &[MpfEntry] {
        &self.entries
    }
}

/// An issue with its layer named. Layers are never merged into one flat type
/// (DEC-006).
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum AnyIssue<'a> {
    /// A JPEG-layer issue (the walk's or the extraction's).
    Jpeg(&'a JpegIssue),
    /// A TIFF-layer issue of one Exif block.
    ///
    /// The issue's offsets are relative to the start of the block's TIFF data
    /// ([`ExifBlock::tiff`]), not to the JPEG. The file offset of a TIFF-relative
    /// offset `o` is `segment.offset() + 10 + o`, where `segment` is
    /// `jpeg().segments()[exif_blocks()[block].segment()]` and 10 is the
    /// segment's own `FF` marker byte (1), marker code (1), length (2) and the
    /// six-byte Exif header.
    Exif {
        /// Index into [`JpegMetadata::exif_blocks`].
        block: usize,
        /// The block's own issue; its offsets are TIFF-relative (see above).
        issue: &'a ParseIssue,
    },
}

/// The result of [`read`]: the walk, the Exif blocks, the located regions and
/// the layered issue view.
#[derive(Debug, Clone)]
pub struct JpegMetadata<'a> {
    jpeg: Jpeg<'a>,
    exif_blocks: Vec<ExifBlock<'a>>,
    regions: Vec<Region<'a>>,
    jpeg_issues: Vec<JpegIssue>,
    mpf: Option<MpfIndex>,
}

impl<'a> JpegMetadata<'a> {
    /// The MP index the embedded images were read from, if there is one (see
    /// [`MpfIndex`]).
    pub fn mpf(&self) -> Option<&MpfIndex> {
        self.mpf.as_ref()
    }

    /// The segment walk.
    pub fn jpeg(&self) -> &Jpeg<'a> {
        &self.jpeg
    }

    /// One block per parsed Exif segment, in file order, at most
    /// [`MAX_EXIF_BLOCKS`].
    pub fn exif_blocks(&self) -> &[ExifBlock<'a>] {
        &self.exif_blocks
    }

    /// The primary Exif: the first block whose container parsed.
    pub fn exif(&self) -> Option<&Container<'a>> {
        self.exif_blocks.iter().find_map(|b| b.container().ok())
    }

    /// The located regions, in segment order.
    pub fn regions(&self) -> &[Region<'a>] {
        &self.regions
    }

    /// The walker's issues, then the extraction's, in order.
    pub fn jpeg_issues(&self) -> &[JpegIssue] {
        &self.jpeg_issues
    }

    /// Every issue with its layer named: the JPEG layer first, then each Exif
    /// block's own issues, in block order. A fatal TIFF error is not an issue.
    pub fn issues(&self) -> impl Iterator<Item = AnyIssue<'_>> {
        let jpeg = self.jpeg_issues.iter().map(AnyIssue::Jpeg);
        let exif = self.exif_blocks.iter().enumerate().flat_map(|(block, b)| {
            b.container.iter().flat_map(move |c| {
                c.issues()
                    .iter()
                    .map(move |issue| AnyIssue::Exif { block, issue })
            })
        });
        jpeg.chain(exif)
    }

    /// Clean only when the JPEG layer has no issue and every Exif block parsed
    /// and is itself clean.
    pub fn is_clean(&self) -> bool {
        self.jpeg_issues.is_empty()
            && self
                .exif_blocks
                .iter()
                .all(|b| matches!(&b.container, Ok(c) if c.is_clean()))
    }
}

/// Walk `data` as a JPEG and extract its Exif blocks and located regions.
/// `Err` only as for [`Jpeg::parse`] (not a JPEG at all).
pub fn read(data: &[u8]) -> Result<JpegMetadata<'_>, Error> {
    read_with(data, true)
}

/// [`read`] with embedded-image location switched on or off. Off, an MPF
/// segment is an ordinary unrecognised APPn (no index is read, no issue is
/// recorded) and the trailer is not reduced: this is how the audit looks
/// inside an embedded image without recursing (DEC-012 section 3).
pub(crate) fn read_with(data: &[u8], embedded: bool) -> Result<JpegMetadata<'_>, Error> {
    let jpeg = Jpeg::parse(data)?;
    Ok(extract(jpeg, data, embedded))
}

/// What reading one MP index produced: the located images as `(entry index,
/// input offset, bytes)` and the data of [`MpfIndex`].
struct MpfRead<'a> {
    images: Vec<(usize, usize, &'a [u8])>,
    header_start: usize,
    big_endian: bool,
    entries: Vec<MpfEntry>,
}

/// The extraction's mutable state. Every issue goes through
/// [`Extract::record`], so no code path can record after a halt or past the
/// documented bound.
struct Extract {
    /// The walker's issues first, then the extraction's.
    issues: Vec<JpegIssue>,
    /// The walk was halted by a cap: record nothing more (DEC-006).
    halted: bool,
}

impl Extract {
    /// The one place an extraction issue is recorded.
    fn record(&mut self, error: Error, offset: usize) {
        if self.halted || self.issues.len() >= MAX_JPEG_METADATA_ISSUES {
            return;
        }
        self.issues.push(JpegIssue::new(error, offset));
    }

    /// A `RegionTooShort` for a header of `need` bytes of which `have` exist.
    fn short(&mut self, what: &'static str, at: usize, need: usize, have: usize) {
        self.record(
            Error::RegionTooShort {
                what,
                at,
                need,
                have,
            },
            at,
        );
    }

    /// Rule 4, second half: the ICC chunks `(segment offset, seq, count)` in
    /// segment order must agree with one another. Exactly one issue, naming the
    /// first inconsistent chunk (see the module docs for the rule).
    fn check_icc(&mut self, chunks: &[(usize, u8, u8)]) {
        let Some(&(first_at, _, first_count)) = chunks.first() else {
            return;
        };
        let mut seen = [false; 256];
        for (i, &(at, seq, count)) in chunks.iter().enumerate() {
            let repeated = seen.get(usize::from(seq)).copied().unwrap_or(true);
            let differs = i > 0 && count != first_count;
            if differs || seq == 0 || seq > count || repeated {
                self.record(Error::IccChunkMismatch { at }, at);
                return;
            }
            if let Some(slot) = seen.get_mut(usize::from(seq)) {
                *slot = true;
            }
        }
        // Every chunk is individually consistent: the set is complete only if
        // it has as many chunks as the first one declared.
        if chunks.len() != usize::from(first_count) {
            self.record(Error::IccChunkMismatch { at: first_at }, first_at);
        }
    }

    /// Rule 1 (DEC-012): read the MP index of one APP2 `MPF\0` segment at input
    /// offset `at` and return the embedded images it lists as `(input offset,
    /// bytes)`, in index order. `None`: the segment does not parse (a bad
    /// TIFF-style header, no `0xB002` entry, an unreadable or empty index);
    /// the caller reports it as an unrecognised APPn plus an issue.
    ///
    /// Index offsets are relative to the MPF header, the byte after the `MPF\0`
    /// signature. Entry 0 is the primary image and is skipped. An entry whose
    /// range is not inside the input, or whose bytes do not start `FF D8`, is
    /// an issue and no image. Entries from [`MAX_MPF_IMAGES`] on are one issue
    /// and are not examined, so the work is bounded by that constant.
    fn mpf<'a>(&mut self, data: &'a [u8], at: usize, payload: &'a [u8]) -> Option<MpfRead<'a>> {
        let header = payload.get(SIG_MPF.len()..)?;
        // The segment's own marker, code and length (4 bytes), then the signature.
        let base = at.checked_add(4)?.checked_add(SIG_MPF.len())?;
        let container = Container::parse(header).ok()?;
        let entry = container.ifd0()?.entry(TAG_MP_ENTRY)?;
        let index = container.payload(entry).ok()?;
        if index.len() < MP_ENTRY_LEN {
            return None;
        }
        let order = container.byte_order();
        // Where the entries start in the input: the index slice is a sub-slice
        // of `header`, so its address difference is its offset in the header.
        // (An address read is safe Rust; the subtraction is checked.)
        let rel = (index.as_ptr() as usize).checked_sub(header.as_ptr() as usize)?;
        let entries_at = base.checked_add(rel)?;
        let mut found = Vec::new();
        let mut entries = Vec::new();
        for (i, chunk) in index.as_chunks::<MP_ENTRY_LEN>().0.iter().enumerate() {
            if i >= MAX_MPF_IMAGES {
                self.record(
                    Error::TooManyMpfImages {
                        at,
                        limit: MAX_MPF_IMAGES,
                    },
                    at,
                );
                break;
            }
            let (Ok(size), Ok(offset)) = (read_u32(chunk, 4, order), read_u32(chunk, 8, order))
            else {
                continue;
            };
            let field = |k: usize| {
                i.checked_mul(MP_ENTRY_LEN)
                    .and_then(|e| e.checked_add(k))
                    .and_then(|e| entries_at.checked_add(e))
            };
            if let (Some(size_field), Some(offset_field)) = (field(4), field(8)) {
                entries.push(MpfEntry {
                    index: i,
                    offset,
                    size,
                    size_field,
                    offset_field,
                    region: None,
                });
            }
            if i == 0 {
                continue;
            }
            let range = usize::try_from(offset)
                .ok()
                .and_then(|o| base.checked_add(o))
                .and_then(|start| {
                    let len = usize::try_from(size).ok()?;
                    let end = start.checked_add(len)?;
                    Some((start, data.get(start..end)?))
                });
            match range {
                None => self.record(Error::MpfImageOutsideInput { at, index: i }, at),
                Some((_, bytes)) if !bytes.starts_with(&[0xFF, 0xD8]) => {
                    self.record(Error::MpfImageNotJpeg { at, index: i }, at);
                }
                Some((start, bytes)) => found.push((i, start, bytes)),
            }
        }
        Some(MpfRead {
            images: found,
            header_start: base,
            big_endian: order == ByteOrder::Big,
            entries,
        })
    }

    /// Rule 5 (DEC-012): the segment is one `Iptc` region (its payload after
    /// the signature), and its Photoshop image resource blocks are walked for
    /// issues only. `body` starts at input offset `body_at`. Stops at the first
    /// fault: one issue per segment.
    fn iptc<'a>(
        &mut self,
        index: usize,
        body: &'a [u8],
        body_at: usize,
        out: &mut Vec<Region<'a>>,
    ) {
        out.push(Region::Iptc {
            segment: index,
            data: body,
        });
        let mut pos = 0usize;
        while pos < body.len() {
            let rest = body.get(pos..).unwrap_or_default();
            let here = body_at.saturating_add(pos);
            if !rest.starts_with(b"8BIM") {
                self.record(
                    Error::StrayBytes {
                        at: here,
                        len: rest.len(),
                    },
                    here,
                );
                return;
            }
            match parse_8bim(rest) {
                Ok(consumed) => {
                    pos = pos.saturating_add(consumed);
                }
                Err(need) => {
                    self.short("8BIM resource", here, need, rest.len());
                    return;
                }
            }
        }
    }
}

/// Walk one `8BIM` block at the start of `rest`: `8BIM`, a 2-byte id, a Pascal
/// string name padded so that it occupies an even number of bytes, a 4-byte
/// size and the data (padded to even). `Ok(consumed)` is the bytes the block
/// occupies, including the pad byte after odd-size data (which the last block of
/// a segment may omit). `Err(need)` is the number of bytes the block needs from
/// its start when `rest` is too short to hold it. The id and the data are not
/// used: since DEC-012 every Photoshop segment is a region, so the walk only
/// finds faults.
fn parse_8bim(rest: &[u8]) -> Result<usize, usize> {
    const ID_AT: usize = 4;
    const NAME_AT: usize = 6;
    if rest.get(ID_AT..NAME_AT).is_none() {
        return Err(NAME_AT);
    }
    let name_len = rest
        .get(NAME_AT)
        .copied()
        .ok_or(NAME_AT.saturating_add(1))?;
    // The length byte plus the name, rounded up to an even byte count.
    let pascal = usize::from(name_len).saturating_add(1);
    let name_total = pascal.saturating_add(pascal & 1);
    let size_at = NAME_AT.saturating_add(name_total);
    let data_at = size_at.saturating_add(4);
    let size = match rest.get(size_at..data_at) {
        Some(&[a, b, c, d]) => {
            usize::try_from(u32::from_be_bytes([a, b, c, d])).unwrap_or(usize::MAX)
        }
        _ => return Err(data_at),
    };
    let data_end = data_at.saturating_add(size);
    if rest.get(data_at..data_end).is_none() {
        return Err(data_end);
    }
    Ok(data_end.saturating_add(size & 1))
}

/// The parts of `start..end` that none of `covered` (ranges of the input, in
/// any order, possibly overlapping or reaching outside) accounts for, as
/// `(start, end)` pairs in order, none empty. At most `covered.len() + 1`.
fn uncovered(start: usize, end: usize, covered: &mut [(usize, usize)]) -> Vec<(usize, usize)> {
    covered.sort_unstable();
    let mut gaps = Vec::new();
    let mut cursor = start;
    for &(a, b) in covered.iter() {
        let a = a.max(start);
        let b = b.min(end);
        if b <= a {
            continue;
        }
        if a > cursor {
            gaps.push((cursor, a));
        }
        cursor = cursor.max(b);
    }
    if cursor < end {
        gaps.push((cursor, end));
    }
    gaps
}

fn extract<'a>(jpeg: Jpeg<'a>, data: &'a [u8], embedded: bool) -> JpegMetadata<'a> {
    let mut ex = Extract {
        issues: jpeg.issues().to_vec(),
        halted: jpeg.halted(),
    };
    let mut exif_blocks = Vec::new();
    let mut regions = Vec::new();
    let mut icc_chunks: Vec<(usize, u8, u8)> = Vec::new();
    let mut exif_seen = 0usize;
    // Input ranges of the located embedded images (they reduce the trailer).
    let mut covered: Vec<(usize, usize)> = Vec::new();
    // Only the first MPF segment is read as an index; a second one cannot be a
    // second index (the issue count stays bounded).
    let mut mpf_seen = false;
    let mut mpf_index: Option<MpfIndex> = None;

    for (index, segment) in jpeg.segments().iter().enumerate() {
        let payload = segment.payload();
        let at = segment.offset();
        // The bytes after a `sig_len`-byte signature; every kind below was
        // classified by that signature, so `None` is unreachable, but a short
        // payload is handled rather than assumed away.
        let after = |sig_len: usize| payload.get(sig_len..);
        match segment.kind() {
            // Rule 1.
            SegmentKind::Exif => {
                if exif_seen >= 1 {
                    ex.record(Error::ExtraExif { at }, at);
                }
                if exif_seen < MAX_EXIF_BLOCKS {
                    if let Some(tiff) = after(EXIF_HEADER_LEN) {
                        exif_blocks.push(ExifBlock {
                            segment: index,
                            tiff_at: at.saturating_add(4).saturating_add(EXIF_HEADER_LEN),
                            tiff,
                            container: Container::parse(tiff),
                        });
                    }
                }
                exif_seen = exif_seen.saturating_add(1);
            }
            // Rule 2.
            SegmentKind::Xmp => {
                if let Some(packet) = after(SIG_XMP.len()) {
                    regions.push(Region::Xmp {
                        segment: index,
                        packet,
                    });
                }
            }
            // Rule 3.
            SegmentKind::ExtendedXmp => {
                let rest = after(SIG_EXTENDED_XMP.len()).unwrap_or_default();
                let header = (
                    rest.get(..32),
                    rest.get(32..36),
                    rest.get(36..EXTENDED_XMP_HEADER),
                    rest.get(EXTENDED_XMP_HEADER..),
                );
                if let (Some(guid), Some(&[a, b, c, d]), Some(&[e, f, g, h]), Some(data)) = header {
                    regions.push(Region::ExtendedXmp {
                        segment: index,
                        guid,
                        full_len: u32::from_be_bytes([a, b, c, d]),
                        offset: u32::from_be_bytes([e, f, g, h]),
                        data,
                    });
                } else {
                    ex.short("extended XMP header", at, EXTENDED_XMP_HEADER, rest.len());
                }
            }
            // Rule 4.
            SegmentKind::Icc => {
                let rest = after(SIG_ICC.len()).unwrap_or_default();
                if let (Some(&seq), Some(&count), Some(data)) =
                    (rest.first(), rest.get(1), rest.get(ICC_HEADER..))
                {
                    icc_chunks.push((at, seq, count));
                    regions.push(Region::Icc {
                        segment: index,
                        seq,
                        count,
                        data,
                    });
                } else {
                    ex.short("ICC chunk header", at, ICC_HEADER, rest.len());
                }
            }
            // Rule 5.
            SegmentKind::Iptc => {
                let body_at = at.saturating_add(4).saturating_add(SIG_IPTC.len());
                if let Some(body) = after(SIG_IPTC.len()) {
                    ex.iptc(index, body, body_at, &mut regions);
                }
            }
            // Rule 6.
            SegmentKind::Jumbf => {
                let rest = after(SIG_JUMBF.len()).unwrap_or_default();
                if let (Some(&[a, b]), Some(&[c, d, e, f]), Some(data)) = (
                    rest.get(..2),
                    rest.get(2..JUMBF_HEADER),
                    rest.get(JUMBF_HEADER..),
                ) {
                    regions.push(Region::Jumbf {
                        segment: index,
                        en: u16::from_be_bytes([a, b]),
                        z: u32::from_be_bytes([c, d, e, f]),
                        data,
                    });
                } else {
                    ex.short("JUMBF header", at, JUMBF_HEADER, rest.len());
                }
            }
            // Rule 3 (DEC-012): a comment.
            SegmentKind::Com => regions.push(Region::Comment {
                segment: index,
                data: payload,
            }),
            // The benign list: APP0 `JFIF\0` (classified as `Jfif`) and APP14
            // `Adobe\0`. Every other APPn is unrecognised, except a parsed MPF.
            SegmentKind::App(n) => {
                let marker = segment.marker();
                if n == 14 && payload.starts_with(SIG_ADOBE) {
                    continue;
                }
                if n == 2 && embedded && payload.starts_with(SIG_MPF) {
                    // The first MPF segment is read; a later one is malformed.
                    let images = if mpf_seen {
                        None
                    } else {
                        ex.mpf(data, at, payload)
                    };
                    mpf_seen = true;
                    if let Some(read) = images {
                        let mut entries = read.entries;
                        for (entry, offset, bytes) in read.images {
                            covered.push((offset, offset.saturating_add(bytes.len())));
                            if let Some(e) = entries.iter_mut().find(|e| e.index == entry) {
                                e.region = Some(regions.len());
                            }
                            regions.push(Region::EmbeddedImage {
                                segment: index,
                                offset,
                                data: bytes,
                            });
                        }
                        mpf_index = Some(MpfIndex {
                            segment: index,
                            header_start: read.header_start,
                            big_endian: read.big_endian,
                            entries,
                        });
                        continue;
                    }
                    ex.record(Error::MpfMalformed { at }, at);
                }
                regions.push(Region::OtherApp {
                    segment: index,
                    marker,
                    data: payload,
                });
            }
            _ => {}
        }
    }
    ex.check_icc(&icc_chunks);

    // Rule 2 (DEC-012): the trailer minus the embedded images, one region per
    // contiguous remainder. Without an EOI there is no trailer.
    if let Some(eoi) = jpeg.eoi_offset() {
        let start = eoi.saturating_add(2);
        for (a, b) in uncovered(start, data.len(), &mut covered) {
            if let Some(bytes) = data.get(a..b) {
                regions.push(Region::Trailer {
                    segment: jpeg.segments().len(),
                    offset: a,
                    data: bytes,
                });
            }
        }
    }

    JpegMetadata {
        jpeg_issues: ex.issues,
        exif_blocks,
        regions,
        jpeg,
        mpf: mpf_index,
    }
}

#[cfg(test)]
#[allow(
    // Test code is not a parse path exercised on untrusted input (AGENTS.md
    // §11); same sanctioned exception as the other test modules.
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::arithmetic_side_effects
)]
mod tests {
    use super::*;
    use crate::tiff::{IfdKind, Link};

    // ── byte builders, independent of the library ────────────────────────────

    fn soi() -> Vec<u8> {
        vec![0xFF, 0xD8]
    }
    fn eoi() -> Vec<u8> {
        vec![0xFF, 0xD9]
    }
    fn seg(marker: u8, payload: &[u8]) -> Vec<u8> {
        let l = u16::try_from(payload.len() + 2).unwrap();
        let mut v = vec![0xFF, marker];
        v.extend_from_slice(&l.to_be_bytes());
        v.extend_from_slice(payload);
        v
    }
    fn sos() -> Vec<u8> {
        seg(0xDA, &[1, 1, 0, 0, 0x3F, 0])
    }
    fn app1(sig: &[u8], rest: &[u8]) -> Vec<u8> {
        seg(0xE1, &[sig, rest].concat())
    }
    fn exif_seg(tiff: &[u8]) -> Vec<u8> {
        app1(b"Exif\0\0", tiff)
    }
    fn xmp_seg(packet: &[u8]) -> Vec<u8> {
        app1(b"http://ns.adobe.com/xap/1.0/\0", packet)
    }
    fn ext_xmp_seg(rest: &[u8]) -> Vec<u8> {
        app1(b"http://ns.adobe.com/xmp/extension/\0", rest)
    }
    fn icc_seg(rest: &[u8]) -> Vec<u8> {
        seg(0xE2, &[&b"ICC_PROFILE\0"[..], rest].concat())
    }
    fn iptc_seg(rest: &[u8]) -> Vec<u8> {
        seg(0xED, &[&b"Photoshop 3.0\0"[..], rest].concat())
    }
    fn jumbf_seg(rest: &[u8]) -> Vec<u8> {
        seg(0xEB, &[&b"JP"[..], rest].concat())
    }
    /// A clean JPEG around `segs` (before the scan); returns the bytes and the
    /// input offset of each segment.
    fn jpeg_of(segs: &[Vec<u8>]) -> (Vec<u8>, Vec<usize>) {
        let mut d = soi();
        let mut offsets = Vec::new();
        for s in segs {
            offsets.push(d.len());
            d.extend_from_slice(s);
        }
        d.extend_from_slice(&sos());
        d.extend_from_slice(&[0x12, 0x34]);
        d.extend_from_slice(&eoi());
        (d, offsets)
    }
    fn one(seg_bytes: Vec<u8>) -> Vec<u8> {
        jpeg_of(&[seg_bytes]).0
    }

    // A photoshop 8BIM resource, padded by its own rules (not the library's).
    fn res(id: u16, name: &[u8], data: &[u8]) -> Vec<u8> {
        let mut v = b"8BIM".to_vec();
        v.extend_from_slice(&id.to_be_bytes());
        v.push(u8::try_from(name.len()).unwrap());
        v.extend_from_slice(name);
        if (1 + name.len()) % 2 == 1 {
            v.push(0);
        }
        v.extend_from_slice(&u32::try_from(data.len()).unwrap().to_be_bytes());
        v.extend_from_slice(data);
        if data.len() % 2 == 1 {
            v.push(0);
        }
        v
    }

    // ── a literal little-endian TIFF builder ─────────────────────────────────

    enum V {
        Inline([u8; 4]),
        Ext(Vec<u8>),
        /// Offset of another IFD, by index.
        Ptr(usize),
        /// A literal offset backed by nothing.
        Raw(u32),
    }
    struct E {
        tag: u16,
        typ: u16,
        count: u32,
        v: V,
    }
    fn e(tag: u16, typ: u16, count: u32, v: V) -> E {
        E { tag, typ, count, v }
    }
    fn ascii(tag: u16, s: &str) -> E {
        let mut b = s.as_bytes().to_vec();
        b.push(0);
        let count = u32::try_from(b.len()).unwrap();
        if b.len() <= 4 {
            b.resize(4, 0);
            e(tag, 2, count, V::Inline(b.try_into().unwrap()))
        } else {
            e(tag, 2, count, V::Ext(b))
        }
    }
    fn tiff(ifds: &[Vec<E>]) -> Vec<u8> {
        let mut starts = Vec::new();
        let mut pos = 8usize;
        for ifd in ifds {
            starts.push(pos);
            pos += 2 + 12 * ifd.len() + 4;
            for en in ifd {
                if let V::Ext(b) = &en.v {
                    pos += b.len();
                }
            }
        }
        let off = |i: usize| u32::try_from(starts[i]).unwrap();
        let mut out = b"II".to_vec();
        out.extend_from_slice(&42u16.to_le_bytes());
        out.extend_from_slice(&off(0).to_le_bytes());
        for ifd in ifds {
            let ext_base = out.len() + 2 + 12 * ifd.len() + 4;
            let mut ext = Vec::new();
            out.extend_from_slice(&u16::try_from(ifd.len()).unwrap().to_le_bytes());
            for en in ifd {
                out.extend_from_slice(&en.tag.to_le_bytes());
                out.extend_from_slice(&en.typ.to_le_bytes());
                out.extend_from_slice(&en.count.to_le_bytes());
                match &en.v {
                    V::Inline(b) => out.extend_from_slice(b),
                    V::Ext(b) => {
                        let at = u32::try_from(ext_base + ext.len()).unwrap();
                        out.extend_from_slice(&at.to_le_bytes());
                        ext.extend_from_slice(b);
                    }
                    V::Ptr(i) => out.extend_from_slice(&off(*i).to_le_bytes()),
                    V::Raw(o) => out.extend_from_slice(&o.to_le_bytes()),
                }
            }
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(&ext);
        }
        out
    }
    /// Make "DiamondCam", an Exif IFD (ExposureTime 1/250, DateTimeOriginal)
    /// and a GPS IFD (GPSLatitudeRef "N").
    fn good_tiff() -> Vec<u8> {
        let mut rational = 1u32.to_le_bytes().to_vec();
        rational.extend_from_slice(&250u32.to_le_bytes());
        tiff(&[
            vec![
                ascii(0x010F, "DiamondCam"),
                e(0x8769, 4, 1, V::Ptr(1)),
                e(0x8825, 4, 1, V::Ptr(2)),
            ],
            vec![
                e(0x829A, 5, 1, V::Ext(rational)),
                ascii(0x9003, "2024:01:02 03:04:05"),
            ],
            vec![ascii(0x0001, "N")],
        ])
    }
    /// The same, but the GPS pointer leads past the end of the blob.
    fn gps_past_eof_tiff() -> Vec<u8> {
        tiff(&[vec![
            ascii(0x010F, "DiamondCam"),
            e(0x8825, 4, 1, V::Raw(0x00FF_FFFF)),
        ]])
    }

    type Shape = Vec<(u32, IfdKind, u32, Vec<(u16, u16, u32)>)>;
    fn shape(c: &Container<'_>) -> Shape {
        c.ifds()
            .iter()
            .map(|i| {
                (
                    i.offset(),
                    i.kind(),
                    i.depth(),
                    i.entries()
                        .iter()
                        .map(|en| (en.tag(), en.field_type(), en.count()))
                        .collect(),
                )
            })
            .collect()
    }
    fn make(c: &Container<'_>) -> String {
        let en = c.ifd0().unwrap().entry(0x010F).unwrap();
        c.ascii(en).unwrap().to_str().unwrap().to_string()
    }
    fn errs(m: &JpegMetadata<'_>) -> Vec<Error> {
        m.jpeg_issues().iter().map(|i| i.error().clone()).collect()
    }
    fn extra_exif(m: &JpegMetadata<'_>) -> Vec<usize> {
        m.jpeg_issues()
            .iter()
            .filter_map(|i| match i.error() {
                Error::ExtraExif { at } => Some(*at),
                _ => None,
            })
            .collect()
    }

    // ── Exif hand-off ────────────────────────────────────────────────────────

    #[test]
    fn exif_in_a_jpeg_reads_like_the_bare_tiff() {
        let blob = good_tiff();
        let bare = Container::parse(&blob).unwrap();
        assert!(bare.is_clean());
        let d = one(exif_seg(&blob));
        let m = read(&d).unwrap();
        let wrapped = m.exif().expect("a primary exif");
        assert_eq!(shape(wrapped), shape(&bare));
        assert_eq!(wrapped.ifds().len(), 3);
        assert_eq!(make(wrapped), "DiamondCam");
        assert_eq!(make(&bare), "DiamondCam");
        // ASCII and rational values agree, entry by entry.
        let exif_ifd = wrapped.ifd_of_kind(IfdKind::Exif).unwrap();
        let exif_bare = bare.ifd_of_kind(IfdKind::Exif).unwrap();
        let dt = |c: &Container<'_>, i: &crate::tiff::Ifd| {
            c.ascii(i.entry(0x9003).unwrap())
                .unwrap()
                .to_str()
                .unwrap()
                .to_string()
        };
        assert_eq!(dt(wrapped, exif_ifd), "2024:01:02 03:04:05");
        assert_eq!(dt(wrapped, exif_ifd), dt(&bare, exif_bare));
        let rat = |c: &Container<'_>, i: &crate::tiff::Ifd| {
            c.urationals(i.entry(0x829A).unwrap()).unwrap()
        };
        assert_eq!(rat(wrapped, exif_ifd), rat(&bare, exif_bare));
        assert_eq!(rat(wrapped, exif_ifd)[0].num, 1);
        assert_eq!(rat(wrapped, exif_ifd)[0].den, 250);
        let gps = wrapped.ifd_of_kind(IfdKind::Gps).unwrap();
        let lat_ref = c_ascii(wrapped, gps, 0x0001);
        assert_eq!(lat_ref, "N");
        // Clean at every layer.
        assert!(m.jpeg().is_clean());
        assert!(m.jpeg_issues().is_empty());
        assert_eq!(m.issues().count(), 0);
        assert!(m.is_clean());
    }
    fn c_ascii(c: &Container<'_>, i: &crate::tiff::Ifd, tag: u16) -> String {
        c.ascii(i.entry(tag).unwrap())
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    }

    /// DEC-007: an Exif APP1 is recognised by its first five bytes `Exif\0`;
    /// the sixth byte is not checked and the TIFF data still starts at payload
    /// offset 6. Measured against ExifTool 13.55 in SPEC-011.
    #[test]
    fn exif_signature_is_five_bytes_sixth_byte_ignored() {
        let blob = good_tiff();
        for sixth in [0x00u8, 0x01, 0x7F, 0xFF] {
            let mut payload = b"Exif\0".to_vec();
            payload.push(sixth);
            payload.extend_from_slice(&blob);
            let d = one(seg(0xE1, &payload));
            let m = read(&d).unwrap();
            assert_eq!(
                m.jpeg().segments()[0].kind(),
                SegmentKind::Exif,
                "sixth byte {sixth:#04x}"
            );
            assert_eq!(m.exif_blocks().len(), 1, "sixth byte {sixth:#04x}");
            assert_eq!(
                m.exif_blocks()[0].tiff(),
                &blob[..],
                "sixth byte {sixth:#04x}"
            );
            assert_eq!(make(m.exif().expect("parsed")), "DiamondCam");
            assert!(m.is_clean(), "sixth byte {sixth:#04x}: {:?}", errs(&m));
        }
        // Exactly six bytes (`Exif\0` plus any sixth byte): an Exif block whose
        // TIFF data is empty, a typed per-block error and no JPEG-layer issue.
        for sixth in [0x00u8, 0x01] {
            let d = one(seg(0xE1, &[b"Exif\0"[..].to_vec(), vec![sixth]].concat()));
            let m = read(&d).unwrap();
            assert_eq!(m.jpeg().segments()[0].kind(), SegmentKind::Exif);
            assert_eq!(m.exif_blocks().len(), 1);
            assert!(m.exif_blocks()[0].tiff().is_empty());
            assert!(m.exif_blocks()[0].container().is_err());
            assert!(errs(&m).is_empty());
        }
        // Not Exif: a five-byte payload, a four-byte payload, a non-NUL fifth
        // byte, and a wrong first byte.
        let not_exif: [&[u8]; 5] = [
            b"Exif\0",
            b"Exif",
            b"Exif\x01\0II*\0",
            b"exif\0\0II*\0",
            b"",
        ];
        for payload in not_exif {
            let d = one(seg(0xE1, payload));
            let m = read(&d).unwrap();
            assert_eq!(
                m.jpeg().segments()[0].kind(),
                SegmentKind::App(1),
                "{payload:?}"
            );
            assert!(m.exif_blocks().is_empty(), "{payload:?}");
            assert!(m.exif().is_none());
        }
        // The signature must be under APP1.
        let d = one(seg(0xE2, &[&b"Exif\0\x01"[..], &blob].concat()));
        let m = read(&d).unwrap();
        assert_eq!(m.jpeg().segments()[0].kind(), SegmentKind::App(2));
        assert!(m.exif_blocks().is_empty());
    }

    #[test]
    fn the_tiff_slice_starts_after_the_six_byte_signature() {
        let blob = good_tiff();
        let d = one(exif_seg(&blob));
        let tiff_slice = {
            let m = read(&d).unwrap();
            let seg0 = m.jpeg().segments()[0];
            let b = &m.exif_blocks()[0];
            assert_eq!(b.segment(), 0);
            assert_eq!(b.tiff(), &seg0.payload()[6..]);
            assert_eq!(b.tiff(), &blob[..]);
            b.tiff()
            // `m` is dropped here: the slice borrows from `d`, not from `m`.
        };
        let r = d.as_ptr_range();
        let t = tiff_slice.as_ptr_range();
        assert!(t.start >= r.start && t.end <= r.end);
        assert_eq!(tiff_slice.as_ptr(), d[2 + 4 + 6..].as_ptr());
    }

    #[test]
    fn a_fatal_tiff_is_kept_on_the_block_not_promoted() {
        let d = one(exif_seg(b"NOPE"));
        let m = read(&d).unwrap();
        assert_eq!(m.exif_blocks().len(), 1);
        assert_eq!(
            m.exif_blocks()[0].container().unwrap_err(),
            &Error::NotTiff { mark: *b"NO" }
        );
        assert!(m.exif().is_none());
        assert!(m.jpeg_issues().is_empty());
        assert!(!m.is_clean());
        assert_eq!(m.issues().count(), 0);
    }

    #[test]
    fn a_signature_only_exif_is_a_truncated_tiff() {
        let d = one(exif_seg(b""));
        let m = read(&d).unwrap();
        assert_eq!(m.exif_blocks().len(), 1);
        assert!(m.exif_blocks()[0].tiff().is_empty());
        assert!(matches!(
            m.exif_blocks()[0].container(),
            Err(Error::Truncated { .. })
        ));
    }

    #[test]
    fn a_damaged_gps_ifd_leaves_a_clean_jpeg_layer() {
        let d = one(exif_seg(&gps_past_eof_tiff()));
        let m = read(&d).unwrap();
        assert!(m.jpeg_issues().is_empty());
        let issues: Vec<AnyIssue<'_>> = m.issues().collect();
        assert_eq!(issues.len(), 1);
        match issues[0] {
            AnyIssue::Exif { block, issue } => {
                assert_eq!(block, 0);
                assert_eq!(issue.link(), Link::Tag(0x8825));
            }
            other => panic!("expected an Exif-layer issue, got {other:?}"),
        }
        assert!(m.exif().is_some());
        assert!(!m.is_clean());
    }

    #[test]
    fn a_second_exif_segment_is_an_extra_exif_issue_and_still_a_block() {
        let blob = good_tiff();
        // A stray byte before the first Exif gives a walker issue to order against.
        let mut d = soi();
        d.push(0x00);
        let first = d.len();
        d.extend_from_slice(&exif_seg(&blob));
        let second = d.len();
        d.extend_from_slice(&exif_seg(&blob));
        d.extend_from_slice(&sos());
        d.extend_from_slice(&eoi());
        let m = read(&d).unwrap();
        assert_eq!(m.exif_blocks().len(), 2);
        assert_eq!(m.exif_blocks()[0].segment(), 0);
        assert_eq!(m.exif_blocks()[1].segment(), 1);
        assert!(std::ptr::eq(
            m.exif().unwrap(),
            m.exif_blocks()[0].container().unwrap()
        ));
        assert_eq!(
            errs(&m),
            [
                Error::StrayBytes { at: 2, len: 1 },
                Error::ExtraExif { at: second }
            ]
        );
        assert_ne!(first, second);
        assert_eq!(
            m.jpeg().issues().len(),
            1,
            "the walker's own list is untouched"
        );
        assert_eq!(m.jpeg_issues()[1].offset(), second);
    }

    #[test]
    fn exif_blocks_are_capped_at_sixteen_but_extras_are_all_reported() {
        let blob = good_tiff();
        let segs: Vec<Vec<u8>> = (0..17).map(|_| exif_seg(&blob)).collect();
        let (d, offsets) = jpeg_of(&segs);
        let m = read(&d).unwrap();
        assert_eq!(m.exif_blocks().len(), MAX_EXIF_BLOCKS);
        assert_eq!(MAX_EXIF_BLOCKS, 16);
        assert_eq!(extra_exif(&m), offsets[1..].to_vec());
        assert_eq!(m.jpeg_issues().len(), 16);
        // Exactly sixteen: the sixteenth segment has a block, the 17th none.
        assert_eq!(m.exif_blocks().last().unwrap().segment(), 15);
        // Sixteen Exif segments: sixteen blocks, fifteen extras.
        let (d16, _) = jpeg_of(&segs[..16]);
        let m16 = read(&d16).unwrap();
        assert_eq!(m16.exif_blocks().len(), 16);
        assert_eq!(extra_exif(&m16).len(), 15);
    }

    #[test]
    fn the_first_ok_block_is_primary_even_if_the_first_segment_is_fatal() {
        let blob = good_tiff();
        let (d, _) = jpeg_of(&[exif_seg(b"NOPE"), exif_seg(&blob)]);
        let m = read(&d).unwrap();
        assert!(m.exif_blocks()[0].container().is_err());
        assert!(std::ptr::eq(
            m.exif().unwrap(),
            m.exif_blocks()[1].container().unwrap()
        ));
    }

    #[test]
    fn an_exif_segment_after_the_scan_is_still_read() {
        let blob = good_tiff();
        let before = one(exif_seg(&blob));
        let mut after = soi();
        after.extend_from_slice(&sos());
        after.extend_from_slice(&[0x12, 0x34]);
        let at = after.len();
        after.extend_from_slice(&exif_seg(&blob));
        after.extend_from_slice(&eoi());
        let m = read(&after).unwrap();
        assert!(m.jpeg().segments()[1].after_scan());
        assert_eq!(m.jpeg().segments()[1].offset(), at);
        assert_eq!(m.exif_blocks().len(), 1);
        assert_eq!(m.exif_blocks()[0].segment(), 1);
        let b = read(&before).unwrap();
        assert_eq!(
            shape(m.exif().unwrap()),
            shape(b.exif().unwrap()),
            "same container before and after the scan"
        );
    }

    // ── located regions ──────────────────────────────────────────────────────

    #[test]
    fn xmp_packet_is_the_payload_after_the_signature() {
        let d = one(xmp_seg(b"<x:xmpmeta/>"));
        let m = read(&d).unwrap();
        assert!(m.is_clean());
        assert_eq!(m.regions().len(), 1);
        match m.regions()[0] {
            Region::Xmp { segment, packet } => {
                assert_eq!(segment, 0);
                assert_eq!(packet, b"<x:xmpmeta/>");
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(m.regions()[0].data(), b"<x:xmpmeta/>");
        assert_eq!(m.regions()[0].segment(), 0);
        // An empty packet is still a region.
        let d = one(xmp_seg(b""));
        let m = read(&d).unwrap();
        assert_eq!(m.regions().len(), 1);
        assert!(m.regions()[0].data().is_empty());
        assert!(m.is_clean());
    }

    #[test]
    fn extended_xmp_header_fields_are_read() {
        let mut guid_a = [0u8; 32];
        for (i, b) in guid_a.iter_mut().enumerate() {
            *b = b'A' + u8::try_from(i % 26).unwrap();
        }
        let chunk = |full: u32, off: u32, data: &[u8]| {
            let mut v = guid_a.to_vec();
            v.extend_from_slice(&full.to_be_bytes());
            v.extend_from_slice(&off.to_be_bytes());
            v.extend_from_slice(data);
            ext_xmp_seg(&v)
        };
        let (d, _) = jpeg_of(&[
            chunk(0x0102_0304, 0, b"first"),
            chunk(0x0102_0304, 5, b"second"),
        ]);
        let m = read(&d).unwrap();
        assert!(m.is_clean());
        assert_eq!(m.regions().len(), 2);
        match m.regions()[0] {
            Region::ExtendedXmp {
                segment,
                guid,
                full_len,
                offset,
                data,
            } => {
                assert_eq!(segment, 0);
                assert_eq!(guid, &guid_a[..]);
                assert_eq!((full_len, offset), (0x0102_0304, 0));
                assert_eq!(data, b"first");
            }
            other => panic!("{other:?}"),
        }
        match m.regions()[1] {
            Region::ExtendedXmp {
                segment,
                guid,
                offset,
                data,
                ..
            } => {
                assert_eq!((segment, offset), (1, 5));
                assert_eq!(guid, &guid_a[..]);
                assert_eq!(data, b"second");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_short_extended_xmp_header_is_region_too_short() {
        let d = one(ext_xmp_seg(&[7u8; 39]));
        let m = read(&d).unwrap();
        assert!(m.regions().is_empty());
        assert_eq!(
            errs(&m),
            [Error::RegionTooShort {
                what: "extended XMP header",
                at: 2,
                need: 40,
                have: 39
            }]
        );
        assert_eq!(m.jpeg_issues()[0].offset(), 2);
        // 35 + 40 is accepted with empty data.
        let d = one(ext_xmp_seg(&[7u8; 40]));
        let m = read(&d).unwrap();
        assert!(m.is_clean());
        assert_eq!(m.regions().len(), 1);
        assert!(m.regions()[0].data().is_empty());
    }

    fn icc(seq: u8, count: u8, data: &[u8]) -> Vec<u8> {
        icc_seg(&[&[seq, count][..], data].concat())
    }

    #[test]
    fn icc_chunks_report_sequence_and_count() {
        let d = one(icc(1, 2, b"abc"));
        let m = read(&d).unwrap();
        match m.regions()[0] {
            Region::Icc {
                segment,
                seq,
                count,
                data,
            } => {
                assert_eq!((segment, seq, count), (0, 1, 2));
                assert_eq!(data, b"abc");
            }
            other => panic!("{other:?}"),
        }
        // One chunk of one is complete; one chunk of two is not.
        assert!(read(&one(icc(1, 1, b"x"))).unwrap().is_clean());
        assert!(!read(&one(icc(1, 2, b"x"))).unwrap().is_clean());
    }

    #[test]
    fn consistent_icc_chunks_are_clean() {
        for order in [[(1u8, b'a'), (2, b'b')], [(2, b'b'), (1, b'a')]] {
            let segs: Vec<Vec<u8>> = order.iter().map(|&(s, b)| icc(s, 2, &[b])).collect();
            let (d, _) = jpeg_of(&segs);
            let m = read(&d).unwrap();
            assert!(m.is_clean(), "{:?}", errs(&m));
            assert_eq!(m.regions().len(), 2);
        }
    }

    #[test]
    fn icc_chunk_mismatches_are_one_issue() {
        // (chunks as (seq, count), which chunk is named: index into the list)
        type Case<'a> = (&'a str, &'a [(u8, u8)], usize);
        let cases: [Case<'_>; 8] = [
            ("differing count", &[(1, 2), (2, 3)], 1),
            ("first chunk is the odd one out", &[(1, 3), (2, 2)], 1),
            ("seq 0 in the first chunk", &[(0, 2), (1, 2)], 0),
            ("seq 0 in a later chunk", &[(1, 2), (0, 2)], 1),
            ("seq above count", &[(1, 2), (3, 2)], 1),
            ("repeated seq", &[(1, 2), (1, 2)], 1),
            ("count 3 with two chunks present", &[(1, 3), (2, 3)], 0),
            ("count 0", &[(1, 0)], 0),
        ];
        for (name, chunks, named) in cases {
            let segs: Vec<Vec<u8>> = chunks.iter().map(|&(s, c)| icc(s, c, b"z")).collect();
            let (d, offsets) = jpeg_of(&segs);
            let m = read(&d).unwrap();
            assert_eq!(
                errs(&m),
                [Error::IccChunkMismatch { at: offsets[named] }],
                "{name}"
            );
            assert_eq!(m.regions().len(), chunks.len(), "{name}: regions are kept");
            assert!(!m.is_clean());
        }
        // More chunks than the count says: the third chunk repeats or exceeds.
        let (d, offsets) = jpeg_of(&[icc(1, 2, b"a"), icc(2, 2, b"b"), icc(2, 2, b"c")]);
        assert_eq!(
            errs(&read(&d).unwrap()),
            [Error::IccChunkMismatch { at: offsets[2] }]
        );
        // Several faults still make exactly one issue.
        let (d, offsets) = jpeg_of(&[icc(0, 9, b"a"), icc(5, 2, b"b"), icc(5, 2, b"c")]);
        assert_eq!(
            errs(&read(&d).unwrap()),
            [Error::IccChunkMismatch { at: offsets[0] }]
        );
    }

    #[test]
    fn an_icc_chunk_without_room_for_seq_and_count_is_region_too_short() {
        for rest in [&[][..], &[1u8][..]] {
            let d = one(icc_seg(rest));
            let m = read(&d).unwrap();
            assert!(m.regions().is_empty());
            assert_eq!(
                errs(&m),
                [Error::RegionTooShort {
                    what: "ICC chunk header",
                    at: 2,
                    need: 2,
                    have: rest.len()
                }]
            );
        }
        // Two bytes are enough (empty data).
        assert_eq!(read(&one(icc_seg(&[1, 1]))).unwrap().regions().len(), 1);
    }

    #[test]
    fn every_photoshop_app13_is_an_iptc_region() {
        let mut body = Vec::new();
        // Pascal name "ab" (3 bytes, padded to 4), odd-size data (3, padded).
        body.extend_from_slice(&res(0x0408, b"ab", &[9, 9, 9]));
        // Empty name (1 byte, padded to 2), 0x0404 with a 5-byte body.
        body.extend_from_slice(&res(0x0404, b"", &[1, 2, 3, 4, 5]));
        // Name "abc" (4 bytes, no pad), even data, then another 0x0404.
        body.extend_from_slice(&res(0x0409, b"abc", &[7, 7]));
        body.extend_from_slice(&res(0x0404, b"xyz", &[8, 8]));
        let d = one(iptc_seg(&body));
        let m = read(&d).unwrap();
        assert!(m.is_clean(), "{:?}", errs(&m));
        // DEC-012: the region is the segment (its payload after the header),
        // whatever resources are inside it.
        let datas: Vec<&[u8]> = m.regions().iter().map(Region::data).collect();
        assert_eq!(datas, [&body[..]]);
        assert!(matches!(m.regions()[0], Region::Iptc { segment: 0, .. }));
        // The last resource may omit its trailing pad byte.
        let mut body = res(0x0404, b"", &[1, 2, 3]);
        body.pop();
        let bytes = one(iptc_seg(&body));
        let m = read(&bytes).unwrap();
        assert!(m.is_clean());
        assert_eq!(m.regions()[0].data(), &body[..]);
        // Every Photoshop segment is its own region, in segment order, even
        // one with no resource at all or with no 0x0404 resource.
        let (d, _) = jpeg_of(&[
            iptc_seg(&res(0x0404, b"", &[1, 2])),
            iptc_seg(b""),
            iptc_seg(&res(0x0408, b"", &[1])),
        ]);
        let m = read(&d).unwrap();
        assert!(m.is_clean(), "{:?}", errs(&m));
        let segs: Vec<usize> = m.regions().iter().map(Region::segment).collect();
        assert_eq!(segs, [0, 1, 2]);
        assert!(m.regions().iter().all(|r| matches!(r, Region::Iptc { .. })));
        assert_eq!(m.regions()[1].data(), b"");
    }

    #[test]
    fn a_truncated_8bim_resource_is_region_too_short_and_stops() {
        let first = res(0x0404, b"", &[1, 2, 3, 4]);
        let mut second = b"8BIM".to_vec();
        second.extend_from_slice(&0x0404u16.to_be_bytes());
        second.extend_from_slice(&[0, 0]); // empty name, padded
        second.extend_from_slice(&100u32.to_be_bytes());
        second.extend_from_slice(&[1, 2, 3]);
        let third = res(0x0404, b"", &[6, 6]); // never reached
        let body = [first.clone(), second.clone(), third].concat();
        let d = one(iptc_seg(&body));
        let m = read(&d).unwrap();
        assert_eq!(m.regions().len(), 1, "the segment is still a region");
        assert_eq!(m.regions()[0].data(), &body[..]);
        // segment at 2, payload at 6, then 14 signature bytes, then `first`.
        let at = 2 + 4 + 14 + first.len();
        let have = second.len() + res(0x0404, b"", &[6, 6]).len();
        assert_eq!(
            errs(&m),
            [Error::RegionTooShort {
                what: "8BIM resource",
                at,
                need: 4 + 2 + 2 + 4 + 100,
                have
            }]
        );
        // A header that stops inside the fixed fields is the same error.
        for cut in [4usize, 5, 6, 7, 8, 11] {
            let body = res(0x0404, b"", &[1, 2])[..cut].to_vec();
            let bytes = one(iptc_seg(&body));
            let m = read(&bytes).unwrap();
            assert_eq!(m.regions().len(), 1, "cut {cut}: the segment is a region");
            assert!(
                matches!(
                    errs(&m)[..],
                    [Error::RegionTooShort { what: "8BIM resource", have, .. }] if have == cut
                ) || matches!(errs(&m)[..], [Error::StrayBytes { len, .. }] if len == cut),
                "cut {cut}: {:?}",
                errs(&m)
            );
        }
    }

    #[test]
    fn non_8bim_bytes_where_a_resource_should_start_are_stray_bytes() {
        let first = res(0x0404, b"", &[1, 2]);
        let body = [first.clone(), b"XYZW-and-more".to_vec()].concat();
        let d = one(iptc_seg(&body));
        let m = read(&d).unwrap();
        assert_eq!(m.regions().len(), 1);
        assert_eq!(
            errs(&m),
            [Error::StrayBytes {
                at: 2 + 4 + 14 + first.len(),
                len: 13
            }]
        );
        // Fewer than four bytes where a resource should start are stray too.
        let body = [first.clone(), b"8BI".to_vec()].concat();
        let bytes = one(iptc_seg(&body));
        let m = read(&bytes).unwrap();
        assert_eq!(
            errs(&m),
            [Error::StrayBytes {
                at: 2 + 4 + 14 + first.len(),
                len: 3
            }]
        );
    }

    #[test]
    fn a_photoshop_segment_without_a_0404_resource_is_a_region_and_no_issue() {
        let body = [res(0x0408, b"", &[1]), res(0x040B, b"url", &[2, 2])].concat();
        let bytes = one(iptc_seg(&body));
        let m = read(&bytes).unwrap();
        assert_eq!(m.regions().len(), 1);
        assert!(m.is_clean());
        // And an empty Photoshop segment.
        let bytes = one(iptc_seg(b""));
        let m = read(&bytes).unwrap();
        assert!(matches!(m.regions(), [Region::Iptc { data: b"", .. }]));
        assert!(m.is_clean());
    }

    #[test]
    fn jumbf_header_fields_are_read() {
        let rest = [0x00, 0x07, 0x01, 0x02, 0x03, 0x04, b'j', b'u', b'm', b'b'];
        let bytes = one(jumbf_seg(&rest));
        let m = read(&bytes).unwrap();
        assert!(m.is_clean());
        match m.regions()[0] {
            Region::Jumbf {
                segment,
                en,
                z,
                data,
            } => {
                assert_eq!((segment, en, z), (0, 7, 0x0102_0304));
                assert_eq!(data, b"jumb");
            }
            other => panic!("{other:?}"),
        }
        // Exactly six bytes after `JP`: empty data.
        let bytes = one(jumbf_seg(&rest[..6]));
        let m = read(&bytes).unwrap();
        assert!(m.regions()[0].data().is_empty());
    }

    #[test]
    fn a_short_jumbf_header_is_region_too_short() {
        let d = one(jumbf_seg(&[0, 7, 1, 2, 3]));
        let m = read(&d).unwrap();
        assert!(m.regions().is_empty());
        assert_eq!(
            errs(&m),
            [Error::RegionTooShort {
                what: "JUMBF header",
                at: 2,
                need: 6,
                have: 5
            }]
        );
        assert_eq!(
            read(&one(jumbf_seg(&[0, 7, 1, 2, 3, 4])))
                .unwrap()
                .regions()
                .len(),
            1
        );
    }

    // ── bounds and API ───────────────────────────────────────────────────────

    #[test]
    fn the_jpeg_layer_issue_total_never_exceeds_the_documented_constant() {
        assert_eq!(MAX_JPEG_METADATA_ISSUES, 1025 + 4097 + 16);
        let tiny = ext_xmp_seg(&[]); // a short extended-XMP header: one issue each
                                     // Not halted: 1000 stray FF00 pairs, then 3000 short segments.
        let mut d = soi();
        for _ in 0..1000 {
            d.extend_from_slice(&[0xFF, 0x00]);
        }
        for _ in 0..3000 {
            d.extend_from_slice(&tiny);
        }
        let m = read(&d).unwrap();
        // 1000 FF00 + MissingSos + MissingEoi from the walker, 3000 extraction.
        assert_eq!(m.jpeg().issues().len(), 1002);
        assert_eq!(m.jpeg_issues().len(), 1002 + 3000);
        assert!(m.jpeg_issues().len() <= MAX_JPEG_METADATA_ISSUES);
        // Halted by the issue cap: nothing is recorded after the halt.
        let mut d = soi();
        for _ in 0..2000 {
            d.extend_from_slice(&[0xFF, 0x00]);
        }
        for _ in 0..3000 {
            d.extend_from_slice(&tiny);
        }
        let m = read(&d).unwrap();
        assert_eq!(m.jpeg_issues().len(), MAX_JPEG_ISSUES + 1);
        assert!(matches!(
            m.jpeg_issues().last().unwrap().error(),
            Error::TooManyIssues { .. }
        ));
        assert!(m.jpeg_issues().len() <= MAX_JPEG_METADATA_ISSUES);
        // The segment cap's halt (a 4097th segment) suppresses the extraction
        // altogether, so this shape is a halted case, not the worst case: the
        // extraction adds nothing and the total is just the walker's.
        let mut d = soi();
        for _ in 0..MAX_JPEG_SEGMENTS {
            d.extend_from_slice(&tiny);
        }
        d.extend_from_slice(&sos()); // the 4097th segment halts the walk
        let m = read(&d).unwrap();
        assert!(matches!(
            m.jpeg_issues().last().unwrap().error(),
            Error::TooManySegments { .. }
        ));
        assert_eq!(m.jpeg_issues().len(), 1);
        assert!(m.jpeg_issues().len() <= MAX_JPEG_METADATA_ISSUES);
        // The real worst case (SPEC-010 verify L1): exactly 4096 minimal
        // segments, no SOS and no halt. The walker adds MissingSos and
        // MissingEoi; the extraction adds one RegionTooShort per segment.
        let mut d = soi();
        for _ in 0..MAX_JPEG_SEGMENTS {
            d.extend_from_slice(&tiny);
        }
        let m = read(&d).unwrap();
        assert_eq!(m.jpeg().segments().len(), MAX_JPEG_SEGMENTS);
        assert_eq!(m.jpeg().issues().len(), 2);
        assert_eq!(m.jpeg_issues().len(), 2 + MAX_JPEG_SEGMENTS);
        assert!(m.jpeg_issues().len() <= MAX_JPEG_METADATA_ISSUES);
        assert!(!m
            .jpeg_issues()
            .iter()
            .any(|i| matches!(i.error(), Error::TooManySegments { .. })));
    }

    #[test]
    fn nothing_is_recorded_after_a_halt() {
        // Halted by the segment cap: 4096 short segments are recorded, the
        // 4097th halts; no RegionTooShort follows the halting issue.
        let tiny = ext_xmp_seg(&[]);
        let mut d = soi();
        for _ in 0..=MAX_JPEG_SEGMENTS {
            d.extend_from_slice(&tiny);
        }
        let m = read(&d).unwrap();
        assert_eq!(m.jpeg().segments().len(), MAX_JPEG_SEGMENTS);
        assert_eq!(
            errs(&m),
            [Error::TooManySegments {
                limit: MAX_JPEG_SEGMENTS
            }]
        );
        assert!(m.regions().is_empty());
        // Halted by the issue cap after two Exif segments: both are blocks, the
        // ExtraExif is suppressed, and TooManyIssues is still the last issue.
        let blob = good_tiff();
        let mut d = soi();
        d.extend_from_slice(&exif_seg(&blob));
        d.extend_from_slice(&exif_seg(&blob));
        for _ in 0..1100 {
            d.extend_from_slice(&[0xFF, 0x00]);
        }
        let m = read(&d).unwrap();
        assert_eq!(m.exif_blocks().len(), 2);
        assert!(extra_exif(&m).is_empty());
        assert_eq!(m.jpeg_issues().len(), MAX_JPEG_ISSUES + 1);
        assert!(matches!(
            m.jpeg_issues().last().unwrap().error(),
            Error::TooManyIssues { .. }
        ));
        assert_eq!(m.jpeg_issues().len(), m.jpeg().issues().len());
    }

    #[test]
    fn read_agrees_with_parse_on_fatal_input() {
        for d in [&[][..], &[0xFF][..], &[0x89, 0x50, 0x4E][..], b"not a jpeg"] {
            assert_eq!(read(d).unwrap_err(), Jpeg::parse(d).unwrap_err());
        }
    }

    #[test]
    fn a_clean_jpeg_with_a_clean_exif_is_clean() {
        let bytes = one(exif_seg(&good_tiff()));
        let m = read(&bytes).unwrap();
        assert!(m.is_clean());
        assert!(m.jpeg().is_clean());
        assert!(m.exif_blocks()[0].container().unwrap().is_clean());
    }

    #[test]
    fn is_clean_requires_every_layer() {
        // JPEG issue only (no EOI).
        let mut d = soi();
        d.extend_from_slice(&exif_seg(&good_tiff()));
        d.extend_from_slice(&sos());
        let m = read(&d).unwrap();
        assert!(!m.jpeg_issues().is_empty());
        assert!(m.exif_blocks()[0].container().unwrap().is_clean());
        assert!(!m.is_clean());
        // Exif issue only.
        let bytes = one(exif_seg(&gps_past_eof_tiff()));
        let m = read(&bytes).unwrap();
        assert!(m.jpeg_issues().is_empty());
        assert!(!m.is_clean());
        // Fatal block only.
        let bytes = one(exif_seg(b"NOPE"));
        let m = read(&bytes).unwrap();
        assert!(m.jpeg_issues().is_empty());
        assert!(!m.is_clean());
        // And no Exif at all is clean.
        assert!(read(&one(seg(0xFE, b"hi"))).unwrap().is_clean());
    }

    #[test]
    fn issues_name_the_layer_jpeg_first_then_blocks_in_order() {
        let mut d = soi();
        d.push(0x00); // a JPEG-layer stray byte
        d.extend_from_slice(&exif_seg(&gps_past_eof_tiff()));
        d.extend_from_slice(&exif_seg(&gps_past_eof_tiff()));
        d.extend_from_slice(&sos());
        d.extend_from_slice(&eoi());
        let m = read(&d).unwrap();
        let layers: Vec<String> = m
            .issues()
            .map(|i| match i {
                AnyIssue::Jpeg(j) => {
                    format!("jpeg:{}", matches!(j.error(), Error::StrayBytes { .. }))
                }
                AnyIssue::Exif { block, .. } => format!("exif:{block}"),
            })
            .collect();
        // Stray byte, ExtraExif, then block 0's issue, then block 1's.
        assert_eq!(layers, ["jpeg:true", "jpeg:false", "exif:0", "exif:1"]);
        assert_eq!(m.issues().count(), 4);
    }

    #[test]
    fn every_error_variant_this_module_returns_displays_something() {
        let errors = [
            Error::ExtraExif { at: 17 },
            Error::RegionTooShort {
                what: "JUMBF header",
                at: 3,
                need: 6,
                have: 5,
            },
            Error::IccChunkMismatch { at: 99 },
        ];
        for e in &errors {
            assert!(!e.to_string().is_empty(), "{e:?} has an empty Display");
        }
        assert!(errors[0].to_string().contains("17"));
        let s = errors[1].to_string();
        assert!(
            s.contains("JUMBF header") && s.contains('3') && s.contains('6') && s.contains('5')
        );
        assert!(errors[2].to_string().contains("99"));
        // No wildcard: a new variant fails to compile here until added above.
        fn index(e: &Error) -> usize {
            match e {
                Error::Truncated { .. } => 0,
                Error::NotTiff { .. } => 1,
                Error::UnsupportedTiffVersion { .. } => 2,
                Error::CyclicIfd { .. } => 3,
                Error::IfdDepthExceeded { .. } => 4,
                Error::TooManyIfds { .. } => 5,
                Error::UnexpectedFieldType { .. } => 6,
                Error::ValueOverflow { .. } => 7,
                Error::TagTooLarge { .. } => 8,
                Error::MissingTag { .. } => 9,
                Error::OffsetOutOfRange { .. } => 10,
                Error::BadPointerCount { .. } => 11,
                Error::NotJpeg { .. } => 12,
                Error::StrayBytes { .. } => 13,
                Error::BadSegmentLength { .. } => 14,
                Error::MissingSos => 15,
                Error::MissingEoi => 16,
                Error::TooManySegments { .. } => 17,
                Error::TooManyIssues { .. } => 18,
                Error::ExtraExif { .. } => 19,
                Error::RegionTooShort { .. } => 20,
                Error::IccChunkMismatch { .. } => 21,
                Error::TooManyEntries { .. } => 22,
                Error::MpfMalformed { .. } => 23,
                Error::MpfImageOutsideInput { .. } => 24,
                Error::MpfImageNotJpeg { .. } => 25,
                Error::TooManyMpfImages { .. } => 26,
            }
        }
        let indices: Vec<usize> = errors.iter().map(index).collect();
        assert_eq!(indices, [19, 20, 21]);
    }

    #[test]
    fn regions_are_in_segment_order_and_borrow_from_the_input() {
        let (d, _) = jpeg_of(&[
            xmp_seg(b"p"),
            icc(1, 1, b"i"),
            jumbf_seg(&[0, 1, 0, 0, 0, 2, b'j']),
            iptc_seg(&res(0x0404, b"", &[4, 4])),
        ]);
        let m = read(&d).unwrap();
        assert!(m.is_clean());
        let order: Vec<usize> = m.regions().iter().map(Region::segment).collect();
        assert_eq!(order, [0, 1, 2, 3]);
        let r = d.as_ptr_range();
        for region in m.regions() {
            let t = region.data().as_ptr_range();
            assert!(t.start >= r.start && t.end <= r.end);
            let payload = m.jpeg().segments()[region.segment()].payload();
            let p = payload.as_ptr_range();
            assert!(t.start >= p.start && t.end <= p.end);
        }
    }

    #[test]
    fn signatures_under_the_wrong_marker_yield_nothing() {
        // `Exif\0\0` under APP2 is a plain APPn: no Exif block, and (DEC-012)
        // an unrecognised-APPn region.
        let d = one(seg(0xE2, b"Exif\0\0II*\0"));
        let m = read(&d).unwrap();
        assert!(m.exif_blocks().is_empty() && m.is_clean());
        assert!(matches!(
            m.regions(),
            [Region::OtherApp {
                segment: 0,
                marker: 0xE2,
                ..
            }]
        ));
    }

    // ── DEC-012: Multi-Picture images, comments, other APPn, trailers ────────
    //
    // Everything below is typed from CIPA DC-007 and DEC-012, not derived from
    // the code under test.

    fn u16w(be: bool, v: u16) -> [u8; 2] {
        if be {
            v.to_be_bytes()
        } else {
            v.to_le_bytes()
        }
    }
    fn u32w(be: bool, v: u32) -> [u8; 4] {
        if be {
            v.to_be_bytes()
        } else {
            v.to_le_bytes()
        }
    }

    /// An APP2 `MPF\0` segment whose index lists `entries` as `(size, offset)`
    /// (entry 0 first), in the byte order `be`. Layout of the MPF header (the
    /// byte after `MPF\0`): TIFF header, one IFD of `NumberOfImages` (0xB001)
    /// and `MPEntry` (0xB002), then the entries themselves.
    fn mpf_seg(be: bool, entries: &[(u32, u32)]) -> Vec<u8> {
        let mut t = if be {
            b"MM\0*".to_vec()
        } else {
            b"II*\0".to_vec()
        };
        t.extend_from_slice(&u32w(be, 8));
        t.extend_from_slice(&u16w(be, 2));
        // NumberOfImages: LONG, count 1, inline.
        t.extend_from_slice(&u16w(be, 0xB001));
        t.extend_from_slice(&u16w(be, 4));
        t.extend_from_slice(&u32w(be, 1));
        t.extend_from_slice(&u32w(be, u32::try_from(entries.len()).unwrap()));
        // MPEntry: UNDEFINED, count 16 per image, out of line after the IFD.
        t.extend_from_slice(&u16w(be, 0xB002));
        t.extend_from_slice(&u16w(be, 7));
        t.extend_from_slice(&u32w(be, u32::try_from(16 * entries.len()).unwrap()));
        t.extend_from_slice(&u32w(be, 8 + 2 + 24 + 4));
        t.extend_from_slice(&u32w(be, 0));
        for (i, (size, offset)) in entries.iter().enumerate() {
            let attr: u32 = if i == 0 { 0x0003_0000 } else { 0x0000_0000 };
            t.extend_from_slice(&u32w(be, attr));
            t.extend_from_slice(&u32w(be, *size));
            t.extend_from_slice(&u32w(be, *offset));
            t.extend_from_slice(&u16w(be, 0));
            t.extend_from_slice(&u16w(be, 0));
        }
        seg(0xE2, &[&b"MPF\0"[..], &t].concat())
    }

    /// A small stand-alone JPEG (SOI, optional segments, SOS, scan, EOI).
    fn small_jpeg(segs: &[Vec<u8>]) -> Vec<u8> {
        jpeg_of(segs).0
    }

    /// An MPO: a primary (SOI, `pre` segments, the MPF segment, SOS, scan,
    /// EOI) followed by `after_primary` junk bytes and then each `images[i]`
    /// back to back. Returns the file and the `(offset, len)` of each image.
    /// The index points each secondary at its true place.
    fn mpo(
        be: bool,
        pre: &[Vec<u8>],
        junk: &[u8],
        images: &[Vec<u8>],
    ) -> (Vec<u8>, Vec<(usize, usize)>) {
        let build = |entries: &[(u32, u32)]| {
            let mut segs: Vec<Vec<u8>> = pre.to_vec();
            segs.push(mpf_seg(be, entries));
            jpeg_of(&segs)
        };
        // The segment size depends only on the entry count.
        let placeholder: Vec<(u32, u32)> = (0..=images.len()).map(|_| (0, 0)).collect();
        let (probe, offs) = build(&placeholder);
        let mpf_at = *offs.last().unwrap();
        let header_start = mpf_at + 8;
        let primary_len = probe.len() + junk.len();
        let mut entries = vec![(u32::try_from(probe.len()).unwrap(), 0u32)];
        let mut at = primary_len;
        let mut ranges = Vec::new();
        for img in images {
            entries.push((
                u32::try_from(img.len()).unwrap(),
                u32::try_from(at - header_start).unwrap(),
            ));
            ranges.push((at, img.len()));
            at += img.len();
        }
        let (mut file, _) = build(&entries);
        assert_eq!(file.len(), probe.len());
        file.extend_from_slice(junk);
        for img in images {
            file.extend_from_slice(img);
        }
        (file, ranges)
    }

    fn embedded_of<'a>(m: &JpegMetadata<'a>) -> Vec<(usize, usize, usize)> {
        m.regions()
            .iter()
            .filter_map(|r| match r {
                Region::EmbeddedImage {
                    segment,
                    offset,
                    data,
                } => Some((*segment, *offset, data.len())),
                _ => None,
            })
            .collect()
    }

    fn trailers_of<'a>(m: &JpegMetadata<'a>) -> Vec<(usize, usize)> {
        m.regions()
            .iter()
            .filter_map(|r| match r {
                Region::Trailer { offset, data, .. } => Some((*offset, data.len())),
                _ => None,
            })
            .collect()
    }

    fn exif_basic() -> Vec<u8> {
        exif_seg(&tiff(&[vec![ascii(0x010F, "DiamondCam")]]))
    }

    #[test]
    fn mpf_index_yields_one_embedded_image_per_secondary_entry_in_order() {
        let a = small_jpeg(&[exif_basic()]);
        let b = small_jpeg(&[xmp_seg(b"<x/>"), icc_seg(&[1, 1, 9, 9])]);
        let c = small_jpeg(&[]);
        let (file, ranges) = mpo(false, &[], &[], &[a.clone(), b.clone(), c.clone()]);
        let m = read(&file).unwrap();
        assert!(m.is_clean(), "{:?}", errs(&m));
        // The MPF segment is segment 0; entry 0 (the primary) is no region.
        let got = embedded_of(&m);
        let want: Vec<(usize, usize, usize)> =
            ranges.iter().map(|(o, l)| (0usize, *o, *l)).collect();
        assert_eq!(got, want);
        assert_eq!(got.len(), 3);
        // The bytes are the images' own, in index order.
        let datas: Vec<&[u8]> = m.regions().iter().map(Region::data).collect();
        assert_eq!(datas, [&a[..], &b[..], &c[..]]);
        // Their ranges account for the whole trailer.
        assert!(trailers_of(&m).is_empty());
        // The inner Exif of the first image is not part of the outer.
        assert!(m.exif_blocks().is_empty());
    }

    #[test]
    fn mpf_big_and_little_endian_headers() {
        for be in [false, true] {
            let a = small_jpeg(&[exif_basic()]);
            let (file, ranges) = mpo(be, &[], &[], &[a.clone(), small_jpeg(&[])]);
            let m = read(&file).unwrap();
            assert!(m.is_clean(), "be={be}: {:?}", errs(&m));
            let got: Vec<(usize, usize)> = embedded_of(&m).iter().map(|e| (e.1, e.2)).collect();
            assert_eq!(got, ranges, "be={be}");
            assert_eq!(m.regions()[0].data(), &a[..], "be={be}");
        }
    }

    #[test]
    fn mpf_entry_outside_the_input_is_an_issue_and_no_region() {
        let good = small_jpeg(&[]);
        let glen = u32::try_from(good.len()).unwrap();
        // The file is the primary (SOI, MPF, SOS, scan, EOI) plus one good
        // image; the MPF header starts at input offset 2 + 8.
        let layout = |entries: &[(u32, u32)]| {
            let (mut d, _) = jpeg_of(&[mpf_seg(false, entries)]);
            let image_at = d.len();
            d.extend_from_slice(&good);
            (d, image_at)
        };
        let (probe, image_at) = layout(&[(0, 0); 5]);
        let rel = |abs: usize| u32::try_from(abs - 10).unwrap();
        let entries = [
            (u32::try_from(image_at).unwrap(), 0u32),
            // Ends exactly one byte past the end of the input.
            (glen, rel(probe.len() + 1 - good.len())),
            // The offset sum overflows.
            (glen, u32::MAX),
            // A size of 2^32 - 1.
            (u32::MAX, rel(image_at)),
            // The control: exactly the image, ending at the end of the input.
            (glen, rel(image_at)),
        ];
        let (file, _) = layout(&entries);
        let m = read(&file).unwrap();
        assert_eq!(
            errs(&m),
            [
                Error::MpfImageOutsideInput { at: 2, index: 1 },
                Error::MpfImageOutsideInput { at: 2, index: 2 },
                Error::MpfImageOutsideInput { at: 2, index: 3 },
            ]
        );
        // Only the control entry is a region.
        assert_eq!(embedded_of(&m), [(0, image_at, good.len())]);
        // The MPF segment itself parsed, so it is not an OtherApp.
        assert!(!m
            .regions()
            .iter()
            .any(|r| matches!(r, Region::OtherApp { .. })));
        assert!(!m.is_clean());
    }

    #[test]
    fn mpf_entry_not_starting_ffd8_is_an_issue_and_no_region() {
        // Image 1 starts FF D9, image 2 is empty, image 3 is one byte (FF),
        // image 4 is a real JPEG: only the last is a region.
        let real = small_jpeg(&[]);
        let notjpeg: Vec<u8> = vec![0xFF, 0xD9, 0, 0];
        let one_byte: Vec<u8> = vec![0xFF];
        let (file, ranges) = mpo(
            false,
            &[],
            &[],
            &[notjpeg.clone(), Vec::new(), one_byte.clone(), real.clone()],
        );
        let m = read(&file).unwrap();
        assert_eq!(
            errs(&m),
            [
                Error::MpfImageNotJpeg { at: 2, index: 1 },
                Error::MpfImageNotJpeg { at: 2, index: 2 },
                Error::MpfImageNotJpeg { at: 2, index: 3 },
            ]
        );
        let got: Vec<(usize, usize)> = embedded_of(&m).iter().map(|e| (e.1, e.2)).collect();
        assert_eq!(got, [ranges[3]]);
        // The bytes of the bad entries stay in the trailer (nothing accounts
        // for them).
        let t: usize = trailers_of(&m).iter().map(|x| x.1).sum();
        assert_eq!(t, notjpeg.len() + one_byte.len());
    }

    #[test]
    fn mpf_seventeenth_image_is_an_issue() {
        // DEC-012: at most 16 images, the primary included. An index of 16
        // is clean (15 regions); the 17th entry costs one issue and is not
        // examined, however many follow.
        let img = small_jpeg(&[]);
        for (total, regions, issues) in [(16usize, 15usize, 0usize), (17, 15, 1), (40, 15, 1)] {
            let imgs: Vec<Vec<u8>> = (1..total).map(|_| img.clone()).collect();
            let (file, _) = mpo(false, &[], &[], &imgs);
            let m = read(&file).unwrap();
            assert_eq!(embedded_of(&m).len(), regions, "total {total}");
            let e = errs(&m);
            assert_eq!(e.len(), issues, "total {total}: {e:?}");
            if issues == 1 {
                assert_eq!(e[0], Error::TooManyMpfImages { at: 2, limit: 16 });
            }
        }
    }

    #[test]
    fn unparsable_mpf_segment_is_other_app_plus_an_issue() {
        // Garbage after the signature, a valid TIFF header with no 0xB002, and
        // an index shorter than one entry.
        let no_index = {
            let mut t = b"II*\0".to_vec();
            t.extend_from_slice(&8u32.to_le_bytes());
            t.extend_from_slice(&1u16.to_le_bytes());
            t.extend_from_slice(&0xB000u16.to_le_bytes());
            t.extend_from_slice(&7u16.to_le_bytes());
            t.extend_from_slice(&4u32.to_le_bytes());
            t.extend_from_slice(b"0100");
            t.extend_from_slice(&0u32.to_le_bytes());
            t
        };
        let short_index = {
            let mut t = b"II*\0".to_vec();
            t.extend_from_slice(&8u32.to_le_bytes());
            t.extend_from_slice(&1u16.to_le_bytes());
            t.extend_from_slice(&0xB002u16.to_le_bytes());
            t.extend_from_slice(&7u16.to_le_bytes());
            t.extend_from_slice(&3u32.to_le_bytes());
            t.extend_from_slice(&[1, 2, 3, 0]);
            t.extend_from_slice(&0u32.to_le_bytes());
            t
        };
        for (label, body) in [
            ("garbage", b"not a tiff header at all".to_vec()),
            ("empty", Vec::new()),
            ("no 0xB002", no_index),
            ("short index", short_index),
        ] {
            let d = one(seg(0xE2, &[&b"MPF\0"[..], &body].concat()));
            let m = read(&d).unwrap();
            assert!(
                matches!(
                    m.regions(),
                    [Region::OtherApp {
                        segment: 0,
                        marker: 0xE2,
                        ..
                    }]
                ),
                "{label}: {:?}",
                m.regions()
            );
            assert_eq!(errs(&m), [Error::MpfMalformed { at: 2 }], "{label}");
        }
        // A truncated index: the entry count says 3 images, the segment holds
        // two entries' worth.
        let full = mpf_seg(false, &[(100, 0), (10, 50), (10, 60)]);
        let cut = &full[..full.len() - 20];
        // Re-length the segment so the walk still accepts it.
        let d = one(seg(0xE2, &cut[4..]));
        let m = read(&d).unwrap();
        assert_eq!(errs(&m), [Error::MpfMalformed { at: 2 }]);
        assert!(matches!(m.regions(), [Region::OtherApp { .. }]));
        // A second MPF segment cannot be a second index.
        let seg1 = mpf_seg(false, &[(10, 0), (10, 5000)]);
        let (d, _) = jpeg_of(&[seg1.clone(), seg1]);
        let m = read(&d).unwrap();
        let e = errs(&m);
        assert!(e.contains(&Error::MpfImageOutsideInput { at: 2, index: 1 }));
        assert!(e.iter().any(|x| matches!(x, Error::MpfMalformed { .. })));
        assert_eq!(
            m.regions()
                .iter()
                .filter(|r| matches!(r, Region::OtherApp { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn hostile_mpf_is_bounded_and_total() {
        let img = small_jpeg(&[]);
        // (a) offsets that point into the MPF segment itself (a 2-byte file
        // start is not reachable, an in-segment offset is): no FF D8 there.
        let (d, _) = jpeg_of(&[mpf_seg(false, &[(100, 0), (50, 4), (50, 8), (50, 12)])]);
        let m = read(&d).unwrap();
        assert!(embedded_of(&m).is_empty());
        assert_eq!(errs(&m).len(), 3);
        // (b) 2^32 - 1 sizes and offsets.
        let (d, _) = jpeg_of(&[mpf_seg(
            true,
            &[
                (u32::MAX, 0),
                (u32::MAX, u32::MAX),
                (u32::MAX, 0),
                (1, u32::MAX),
            ],
        )]);
        let m = read(&d).unwrap();
        assert!(embedded_of(&m).is_empty());
        assert_eq!(errs(&m).len(), 3);
        // (c) 65,535 images declared in NumberOfImages and in the entry count,
        // with only a few entries behind: the index does not parse.
        let mut seg_bytes = mpf_seg(false, &[(1, 0), (1, 0)]);
        // The MPEntry count field: payload offset 4 (sig) + 8 (hdr) + 2 + 12 + 4.
        let count_at = 4 + 4 + 8 + 2 + 12 + 4;
        seg_bytes[count_at..count_at + 4].copy_from_slice(&(65_535u32 * 16).to_le_bytes());
        let d = one(seg_bytes);
        let m = read(&d).unwrap();
        assert_eq!(errs(&m), [Error::MpfMalformed { at: 2 }]);
        // (d) overlapping images and one pointing at another's middle: the
        // overlap is allowed (each is a region), the work is bounded by 15.
        let header_start = 2 + 8;
        let len = u32::try_from(img.len()).unwrap();
        let layout = |at: u32| {
            let entries: Vec<(u32, u32)> = std::iter::once((len, 0))
                .chain((0..15).map(|_| (len, at)))
                .collect();
            jpeg_of(&[mpf_seg(false, &entries)]).0
        };
        let base = layout(0).len();
        let mut d = layout(u32::try_from(base - header_start).unwrap());
        assert_eq!(d.len(), base);
        d.extend_from_slice(&img);
        let m = read(&d).unwrap();
        assert!(m.is_clean(), "{:?}", errs(&m));
        assert_eq!(embedded_of(&m).len(), 15);
        // All fifteen overlap one range: the trailer is empty.
        assert!(trailers_of(&m).is_empty());
        // (e) cyclic: an image that is the MPF-bearing file itself (offset such
        // that start is the file start is unreachable; the primary again at a
        // later offset is just another region). No recursion is possible: the
        // regions are slices, never parsed again here.
        let (twice, _) = mpo(false, &[], &[], &[d.clone()]);
        let m = read(&twice).unwrap();
        assert_eq!(embedded_of(&m).len(), 1);
    }

    #[test]
    fn a_plain_trailer_is_one_region() {
        let mut d = one(com_seg(b"x"));
        let end = d.len();
        d.extend_from_slice(&[7u8; 313]);
        let m = read(&d).unwrap();
        assert_eq!(trailers_of(&m), [(end, 313)]);
        let t = m
            .regions()
            .iter()
            .find(|r| matches!(r, Region::Trailer { .. }))
            .unwrap();
        // The pseudo segment index is one past the last segment.
        assert_eq!(t.segment(), m.jpeg().segments().len());
        assert_eq!(t.data(), m.jpeg().trailer());
    }

    #[test]
    fn a_zero_length_remainder_is_no_region() {
        let d = one(com_seg(b"x"));
        let m = read(&d).unwrap();
        assert!(trailers_of(&m).is_empty());
        // And no EOI at all: no trailer either.
        let mut e = d.clone();
        e.truncate(e.len() - 2);
        e.extend_from_slice(&[9, 9, 9]);
        let m = read(&e).unwrap();
        assert!(trailers_of(&m).is_empty());
    }

    fn com_seg(text: &[u8]) -> Vec<u8> {
        seg(0xFE, text)
    }

    #[test]
    fn trailer_minus_embedded_images_is_the_remainder() {
        // [junk 40][image A][image B][tail 313]: the trailer is junk and tail.
        let a = small_jpeg(&[exif_basic()]);
        let b = small_jpeg(&[]);
        let junk = vec![0x11u8; 40];
        let tail = vec![0x22u8; 313];
        let imgs = vec![a.clone(), b.clone()];
        let (mut file, ranges) = mpo(false, &[], &junk, &imgs);
        let tail_at = file.len();
        file.extend_from_slice(&tail);
        let m = read(&file).unwrap();
        let primary_end = ranges[0].0 - junk.len();
        assert_eq!(trailers_of(&m), [(primary_end, 40), (tail_at, 313)]);
        assert_eq!(embedded_of(&m).len(), 2);
        // Trailer regions come last, after every other region.
        let kinds: Vec<bool> = m
            .regions()
            .iter()
            .map(|r| matches!(r, Region::Trailer { .. }))
            .collect();
        assert_eq!(kinds, [false, false, true, true]);
    }

    #[test]
    fn a_trailer_that_is_exactly_the_mpf_images_gives_no_trailer_region() {
        let (file, _) = mpo(
            true,
            &[],
            &[],
            &[small_jpeg(&[]), small_jpeg(&[com_seg(b"c")])],
        );
        let m = read(&file).unwrap();
        assert!(!m.jpeg().trailer().is_empty());
        assert!(trailers_of(&m).is_empty());
        assert_eq!(embedded_of(&m).len(), 2);
    }

    #[test]
    fn a_partly_covered_trailer_gives_the_uncovered_pieces() {
        // The index points at the middle of the junk: [junk 30][image][junk 20]
        // is a trailer of 30 + len + 20 bytes; the image covers only the
        // middle, so two pieces remain, one each side.
        let img = small_jpeg(&[]);
        let mut tail = vec![0x33u8; 20];
        let (mut file, ranges) = mpo(false, &[], &[0x44u8; 30], std::slice::from_ref(&img));
        let end = file.len();
        file.append(&mut tail);
        let m = read(&file).unwrap();
        let first = ranges[0].0 - 30;
        assert_eq!(trailers_of(&m), [(first, 30), (end, 20)]);
        // An image range inside the primary (an entry pointing at an FF D8
        // that sits in a comment) is a region, but it is clipped away from the
        // trailer rather than subtracting from nothing: the trailer is whole.
        let mpf_len = mpf_seg(false, &[(0, 0), (0, 0)]).len();
        let com_payload_at = 2 + mpf_len + 4;
        let rel = u32::try_from(com_payload_at - 10).unwrap();
        let mut com = vec![0xFF, 0xD8];
        com.extend_from_slice(&[0u8; 18]);
        let (mut d, _) = jpeg_of(&[mpf_seg(false, &[(0, 0), (20, rel)]), com_seg(&com)]);
        let eoi_end = d.len();
        d.extend_from_slice(&[5u8; 9]);
        let m = read(&d).unwrap();
        assert!(m.is_clean(), "{:?}", errs(&m));
        assert_eq!(embedded_of(&m), [(0, com_payload_at, 20)]);
        assert_eq!(trailers_of(&m), [(eoi_end, 9)]);
    }

    #[test]
    fn com_segment_is_a_comment_region() {
        let (d, offs) = jpeg_of(&[com_seg(b"hello"), com_seg(b""), exif_basic()]);
        let m = read(&d).unwrap();
        assert!(m.is_clean());
        let got: Vec<(usize, &[u8])> = m
            .regions()
            .iter()
            .filter_map(|r| match r {
                Region::Comment { segment, data } => Some((*segment, *data)),
                _ => None,
            })
            .collect();
        assert_eq!(got, [(0usize, &b"hello"[..]), (1, &b""[..])]);
        let _ = offs;
        assert_eq!(m.regions().len(), 2);
    }

    #[test]
    fn unrecognised_app_segments_are_other_app() {
        for n in 0u8..=15 {
            let marker = 0xE0 + n;
            // A signature none of the recognised kinds uses.
            let d = one(seg(marker, b"VendorX\0payload"));
            let m = read(&d).unwrap();
            assert!(
                matches!(
                    m.regions(),
                    [Region::OtherApp { segment: 0, marker: mk, data }]
                        if *mk == marker && *data == b"VendorX\0payload"
                ),
                "APP{n}: {:?}",
                m.regions()
            );
            assert!(m.is_clean());
        }
        // A recognised signature under the wrong marker is unrecognised.
        let d = one(seg(0xE4, b"ICC_PROFILE\0\x01\x01data"));
        assert!(matches!(
            read(&d).unwrap().regions(),
            [Region::OtherApp { marker: 0xE4, .. }]
        ));
    }

    #[test]
    fn jfif_and_adobe_app14_are_benign() {
        let jfif = seg(0xE0, b"JFIF\0\x01\x01\0\0\x01\0\x01\0\0");
        let adobe = seg(0xEE, b"Adobe\0\x64\x80\0\0\0\0");
        let (d, _) = jpeg_of(&[jfif, adobe]);
        let m = read(&d).unwrap();
        assert!(m.regions().is_empty(), "{:?}", m.regions());
        assert!(m.is_clean());
        // Exactly the DEC-012 signatures: `JFIF` without its NUL and `Adobe`
        // without its NUL are unrecognised.
        let near = one(seg(0xE0, b"JFIF-not-nul"));
        assert!(matches!(
            read(&near).unwrap().regions(),
            [Region::OtherApp { marker: 0xE0, .. }]
        ));
        let near = one(seg(0xEE, b"Adobe"));
        assert!(matches!(
            read(&near).unwrap().regions(),
            [Region::OtherApp { marker: 0xEE, .. }]
        ));
        // Adobe under another marker is not benign.
        let other = one(seg(0xEC, b"Adobe\0\x64"));
        assert!(matches!(
            read(&other).unwrap().regions(),
            [Region::OtherApp { marker: 0xEC, .. }]
        ));
    }

    #[test]
    fn jfxx_fpxr_and_arot_are_other_app() {
        let jfxx = seg(0xE0, b"JFXX\0\x10thumb");
        let fpxr = seg(0xE2, b"FPXR\0\0\x01\x01flashpix");
        let arot = seg(0xEA, b"AROT\0\0\0\x01");
        let (d, _) = jpeg_of(&[jfxx, fpxr, arot]);
        let m = read(&d).unwrap();
        let markers: Vec<u8> = m
            .regions()
            .iter()
            .filter_map(|r| match r {
                Region::OtherApp { marker, .. } => Some(*marker),
                _ => None,
            })
            .collect();
        assert_eq!(markers, [0xE0, 0xE2, 0xEA]);
        assert_eq!(m.regions().len(), 3);
        assert!(m.is_clean());
    }

    #[test]
    fn app13_without_the_photoshop_header_is_other_app() {
        let d = one(seg(0xED, b"Photoshop 2.5\0 8BIM"));
        let m = read(&d).unwrap();
        assert!(matches!(
            m.regions(),
            [Region::OtherApp {
                marker: 0xED,
                segment: 0,
                ..
            }]
        ));
        // A bare "Photoshop 3.0" (no NUL) is not the header either.
        let d = one(seg(0xED, b"Photoshop 3.0 8BIM"));
        assert!(matches!(
            read(&d).unwrap().regions(),
            [Region::OtherApp { .. }]
        ));
    }

    #[test]
    fn split_app13_gives_two_iptc_regions_and_keeps_its_issues() {
        // One resource whose data crosses the segment boundary: the first
        // segment ends inside the data, the second continues it (no 8BIM).
        let whole = res(0x0404, b"", &[0x1C; 40]);
        let (head, tail) = whole.split_at(20);
        let (d, _) = jpeg_of(&[iptc_seg(head), iptc_seg(tail)]);
        let m = read(&d).unwrap();
        let iptc: Vec<&[u8]> = m
            .regions()
            .iter()
            .filter(|r| matches!(r, Region::Iptc { .. }))
            .map(Region::data)
            .collect();
        assert_eq!(iptc, [head, tail]);
        // The issues are the ones the resource walk always gave: the first
        // segment's resource is short, the second's bytes are stray.
        let e = errs(&m);
        assert_eq!(e.len(), 2, "{e:?}");
        assert!(matches!(
            e[0],
            Error::RegionTooShort {
                what: "8BIM resource",
                ..
            }
        ));
        assert!(matches!(e[1], Error::StrayBytes { .. }));
    }

    #[test]
    fn nested_embedded_ranges_leave_no_phantom_trailer() {
        // Entry 1 spans A+B+C, entry 2 is B alone, inside it: the trailer is
        // exactly accounted for, whatever order the ranges are swept in.
        let a = small_jpeg(&[exif_basic()]);
        let b = small_jpeg(&[xmp_seg(b"<x/>")]);
        let c = small_jpeg(&[com_seg(b"c")]);
        let whole = [a.clone(), b.clone(), c.clone()].concat();
        let layout = |e1: (u32, u32), e2: (u32, u32)| {
            let (mut d, _) = jpeg_of(&[mpf_seg(false, &[(0, 0), e1, e2])]);
            let at = d.len();
            d.extend_from_slice(&whole);
            (d, at)
        };
        let (probe, at) = layout((0, 0), (0, 0));
        let rel = |abs: usize| u32::try_from(abs - 10).unwrap();
        let (file, _) = layout(
            (u32::try_from(whole.len()).unwrap(), rel(at)),
            (u32::try_from(b.len()).unwrap(), rel(at + a.len())),
        );
        assert_eq!(file.len(), probe.len());
        let m = read(&file).unwrap();
        assert!(m.is_clean(), "{:?}", errs(&m));
        assert_eq!(
            embedded_of(&m),
            [(0, at, whole.len()), (0, at + a.len(), b.len())]
        );
        assert!(trailers_of(&m).is_empty(), "{:?}", trailers_of(&m));
        // The same with the nested range listed first.
        let (file, _) = layout(
            (u32::try_from(b.len()).unwrap(), rel(at + a.len())),
            (u32::try_from(whole.len()).unwrap(), rel(at)),
        );
        let m = read(&file).unwrap();
        assert!(trailers_of(&m).is_empty(), "{:?}", trailers_of(&m));
    }

    #[test]
    fn mpf_is_read_only_from_app2() {
        // A well-formed MPF payload under APP3: an unrecognised APPn, no
        // index read, no MPF issue, no embedded image.
        let full = mpf_seg(false, &[(100, 0), (10, 50)]);
        let d = one(seg(0xE3, &full[4..]));
        let m = read(&d).unwrap();
        assert!(
            matches!(
                m.regions(),
                [Region::OtherApp {
                    segment: 0,
                    marker: 0xE3,
                    ..
                }]
            ),
            "{:?}",
            m.regions().len()
        );
        assert!(m.is_clean(), "{:?}", errs(&m));
    }

    #[test]
    fn mpf_index_exposes_field_positions_and_byte_order() {
        for be in [false, true] {
            let a = small_jpeg(&[exif_basic()]);
            let b = small_jpeg(&[]);
            let (file, ranges) = mpo(be, &[], &[0x44u8; 3], &[a.clone(), b.clone()]);
            let m = read(&file).unwrap();
            let idx = m.mpf().expect("an index");
            assert_eq!(idx.big_endian(), be);
            // `FF D8`, then `FF E2 len len M P F 00`: the header starts at 10.
            assert_eq!(idx.header_start(), 10);
            assert_eq!(idx.segment(), 0);
            let seg = &m.jpeg().segments()[idx.segment()];
            assert_eq!(seg.range().unwrap().start, 2);
            assert_eq!(idx.entries().len(), 3);
            for e in idx.entries() {
                // Read the fields back from the input at the reported place.
                let word = |at: usize| {
                    let w: [u8; 4] = file[at..at + 4].try_into().unwrap();
                    if be {
                        u32::from_be_bytes(w)
                    } else {
                        u32::from_le_bytes(w)
                    }
                };
                assert_eq!(word(e.size_field()), e.size(), "entry {}", e.index());
                assert_eq!(word(e.offset_field()), e.offset(), "entry {}", e.index());
                assert_eq!(e.offset_field(), e.size_field() + 4);
            }
            // The index fields agree with the true layout.
            let e1 = idx.entries()[1];
            assert_eq!(e1.size() as usize, ranges[0].1);
            assert_eq!(idx.header_start() + e1.offset() as usize, ranges[0].0);
            // The entries sit 16 bytes apart.
            assert_eq!(
                idx.entries()[2].size_field() - idx.entries()[1].size_field(),
                16
            );
        }
    }

    #[test]
    fn mpf_index_marks_which_entries_became_regions() {
        let good = small_jpeg(&[]);
        let glen = u32::try_from(good.len()).unwrap();
        let (probe, _) = jpeg_of(&[mpf_seg(false, &[(0, 0); 4])]);
        let image_at = probe.len();
        let rel = |abs: usize| u32::try_from(abs - 10).unwrap();
        // Entry 1 is good, entry 2 points outside the input, entry 3 is good.
        let entries = [
            (u32::try_from(image_at).unwrap(), 0u32),
            (glen, rel(image_at)),
            (glen, u32::MAX),
            (glen, rel(image_at)),
        ];
        let (mut file, _) = jpeg_of(&[mpf_seg(false, &entries)]);
        file.extend_from_slice(&good);
        let m = read(&file).unwrap();
        let idx = m.mpf().unwrap();
        let regions: Vec<Option<usize>> = idx.entries().iter().map(|e| e.region()).collect();
        assert_eq!(regions, [None, Some(0), None, Some(1)]);
        for e in idx.entries() {
            if let Some(r) = e.region() {
                assert!(matches!(m.regions()[r], Region::EmbeddedImage { .. }));
            }
        }
    }

    #[test]
    fn second_or_unparsed_mpf_is_not_the_index() {
        // A second MPF segment: only the first is the index.
        let first = mpf_seg(false, &[(100, 0), (10, 50)]);
        let second = mpf_seg(true, &[(100, 0), (10, 60)]);
        let d = one_pair(&first, &second);
        let m = read(&d).unwrap();
        let idx = m.mpf().unwrap();
        assert_eq!(idx.segment(), 0);
        assert!(!idx.big_endian());
        // An unparsable MPF segment first: no index at all, and a parsable
        // second one is still not the index (the issue count stays bounded).
        let junk = seg(0xE2, b"MPF\0not a tiff header");
        let d = one_pair(&junk, &second);
        let m = read(&d).unwrap();
        assert!(m.mpf().is_none());
        // No MPF segment: no index.
        assert!(read(&small_jpeg(&[])).unwrap().mpf().is_none());
    }

    fn one_pair(a: &[u8], b: &[u8]) -> Vec<u8> {
        jpeg_of(&[a.to_vec(), b.to_vec()]).0
    }
}
