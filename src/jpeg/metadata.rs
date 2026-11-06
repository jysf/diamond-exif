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
use crate::tiff::{Container, ParseIssue};

use super::{
    Jpeg, JpegIssue, SegmentKind, EXIF_HEADER_LEN, MAX_JPEG_ISSUES, MAX_JPEG_SEGMENTS,
    SIG_EXTENDED_XMP, SIG_ICC, SIG_IPTC, SIG_JUMBF, SIG_XMP,
};

/// Most Exif segments parsed from one JPEG. Further ones still cost an
/// [`Error::ExtraExif`] issue but get no [`ExifBlock`] (bounded work).
pub const MAX_EXIF_BLOCKS: usize = 16;

/// Most JPEG-layer issues [`JpegMetadata::jpeg_issues`] can hold: the walker's
/// (`MAX_JPEG_ISSUES + 1`) plus the extraction's (at most one per recorded
/// segment, plus one ICC aggregate: `MAX_JPEG_SEGMENTS + 1`).
pub const MAX_JPEG_METADATA_ISSUES: usize = (MAX_JPEG_ISSUES + 1) + (MAX_JPEG_SEGMENTS + 1);

/// The extended-XMP header after the signature: a 32-byte GUID, then the full
/// length and the chunk offset, both big-endian `u32`.
const EXTENDED_XMP_HEADER: usize = 32 + 4 + 4;
/// The ICC chunk header after the signature: sequence number, chunk count.
const ICC_HEADER: usize = 2;
/// The JUMBF header after the `JP` signature: `En` (`u16`) and `Z` (`u32`).
const JUMBF_HEADER: usize = 2 + 4;
/// The Photoshop image resource id of the IPTC-NAA record block.
const IPTC_RESOURCE_ID: u16 = 0x0404;

/// One parsed Exif segment: where it came from and what the TIFF engine made
/// of it.
///
/// Every offset inside [`ExifBlock::container`] (IFD offsets, issue offsets) is
/// relative to the start of [`ExifBlock::tiff`], not to the JPEG. To get a file
/// offset add `segment.offset() + 10` (see [`AnyIssue::Exif`]).
#[derive(Debug, Clone)]
pub struct ExifBlock<'a> {
    segment: usize,
    tiff: &'a [u8],
    container: Result<Container<'a>, Error>,
}

impl<'a> ExifBlock<'a> {
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
    /// The data of a Photoshop `0x0404` (IPTC-NAA) image resource: the IIM
    /// records, not decoded.
    Iptc {
        /// Index into [`Jpeg::segments`].
        segment: usize,
        /// The resource data (without its padding byte).
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
}

impl<'a> Region<'a> {
    /// Index into [`Jpeg::segments`] of the segment the region came from.
    pub fn segment(&self) -> usize {
        match *self {
            Region::Xmp { segment, .. }
            | Region::ExtendedXmp { segment, .. }
            | Region::Icc { segment, .. }
            | Region::Iptc { segment, .. }
            | Region::Jumbf { segment, .. } => segment,
        }
    }

    /// The region's bytes: the packet or the data.
    pub fn data(&self) -> &'a [u8] {
        match *self {
            Region::Xmp { packet, .. } => packet,
            Region::ExtendedXmp { data, .. }
            | Region::Icc { data, .. }
            | Region::Iptc { data, .. }
            | Region::Jumbf { data, .. } => data,
        }
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
}

impl<'a> JpegMetadata<'a> {
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
    let jpeg = Jpeg::parse(data)?;
    Ok(extract(jpeg))
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

    /// Rule 5: walk the Photoshop image resource blocks of one APP13 segment
    /// and locate every `0x0404` resource. `body` is the segment payload after
    /// the signature; it starts at input offset `body_at`. Stops at the first
    /// fault: one issue per segment.
    fn iptc<'a>(
        &mut self,
        index: usize,
        body: &'a [u8],
        body_at: usize,
        out: &mut Vec<Region<'a>>,
    ) {
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
                Ok(resource) => {
                    if resource.id == IPTC_RESOURCE_ID {
                        out.push(Region::Iptc {
                            segment: index,
                            data: resource.data,
                        });
                    }
                    pos = pos.saturating_add(resource.consumed);
                }
                Err(need) => {
                    self.short("8BIM resource", here, need, rest.len());
                    return;
                }
            }
        }
    }
}

/// One Photoshop image resource block.
struct Resource<'a> {
    id: u16,
    data: &'a [u8],
    /// Bytes the block occupies, including the pad byte after odd-size data
    /// (which the last block of a segment may omit).
    consumed: usize,
}

/// Parse one `8BIM` block at the start of `rest`: `8BIM`, a 2-byte id, a Pascal
/// string name padded so that it occupies an even number of bytes, a 4-byte
/// size and the data (padded to even). `Err(need)` is the number of bytes the
/// block needs from its start when `rest` is too short to hold it.
fn parse_8bim(rest: &[u8]) -> Result<Resource<'_>, usize> {
    const ID_AT: usize = 4;
    const NAME_AT: usize = 6;
    let id = match rest.get(ID_AT..NAME_AT) {
        Some(&[hi, lo]) => u16::from_be_bytes([hi, lo]),
        _ => return Err(NAME_AT),
    };
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
    let data = rest.get(data_at..data_end).ok_or(data_end)?;
    Ok(Resource {
        id,
        data,
        consumed: data_end.saturating_add(size & 1),
    })
}

fn extract(jpeg: Jpeg<'_>) -> JpegMetadata<'_> {
    let mut ex = Extract {
        issues: jpeg.issues().to_vec(),
        halted: jpeg.halted(),
    };
    let mut exif_blocks = Vec::new();
    let mut regions = Vec::new();
    let mut icc_chunks: Vec<(usize, u8, u8)> = Vec::new();
    let mut exif_seen = 0usize;

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
            _ => {}
        }
    }
    ex.check_icc(&icc_chunks);

    JpegMetadata {
        jpeg_issues: ex.issues,
        exif_blocks,
        regions,
        jpeg,
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
    fn iptc_0404_resource_is_located_with_even_padding() {
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
        let datas: Vec<&[u8]> = m.regions().iter().map(Region::data).collect();
        assert_eq!(datas, [&[1u8, 2, 3, 4, 5][..], &[8u8, 8][..]]);
        assert!(m
            .regions()
            .iter()
            .all(|r| matches!(r, Region::Iptc { segment: 0, .. })));
        // The last resource may omit its trailing pad byte.
        let mut body = res(0x0404, b"", &[1, 2, 3]);
        body.pop();
        let bytes = one(iptc_seg(&body));
        let m = read(&bytes).unwrap();
        assert!(m.is_clean());
        assert_eq!(m.regions()[0].data(), &[1u8, 2, 3][..]);
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
        assert_eq!(m.regions().len(), 1, "the region before it is kept");
        assert_eq!(m.regions()[0].data(), &[1u8, 2, 3, 4][..]);
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
            assert!(m.regions().is_empty(), "cut {cut}");
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
    fn an_app13_without_a_0404_resource_yields_no_region_and_no_issue() {
        let body = [res(0x0408, b"", &[1]), res(0x040B, b"url", &[2, 2])].concat();
        let bytes = one(iptc_seg(&body));
        let m = read(&bytes).unwrap();
        assert!(m.regions().is_empty());
        assert!(m.is_clean());
        // And an empty Photoshop segment.
        let bytes = one(iptc_seg(b""));
        let m = read(&bytes).unwrap();
        assert!(m.regions().is_empty() && m.is_clean());
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
        assert_eq!(MAX_JPEG_METADATA_ISSUES, 1025 + 4097);
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
        // `Exif\0\0` under APP2 and an XMP signature under APP2 are plain APPn.
        let d = one(seg(0xE2, b"Exif\0\0II*\0"));
        let m = read(&d).unwrap();
        assert!(m.exif_blocks().is_empty() && m.regions().is_empty() && m.is_clean());
    }
}
