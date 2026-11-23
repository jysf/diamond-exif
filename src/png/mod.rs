//! A panic-free, zero-copy PNG chunk walker (SPEC-028, DEC-017).
//!
//! A PNG is an eight-byte signature followed by chunks: a 4-byte big-endian
//! length, a 4-byte type, the data, and a CRC-32 over type and data.
//! [`Png::parse`] returns the ordered chunk list, the end-of-image offset and
//! trailer, and a list of per-chunk [`PngIssue`]s. Only "not a PNG at all" is
//! an `Err` (DEC-017); everything else is an issue and the walk continues
//! wherever it safely can.
//!
//! Nothing in the walk reads chunk contents or inflates anything: chunks are
//! identified by their type bytes only. Contents are located by [`read`]
//! (SPEC-029, DEC-018).
//!
//! Provenance (DEC-003, class 1): net-new code written from the PNG
//! Specification (W3C, Third Edition: file signature, chunk layout, CRC).
//! Nothing is ported.
//!
//! Progress argument: every loop iteration either ends the walk or advances
//! the cursor by at least 12 bytes (a whole chunk), so the walk is linear in
//! the input. It is iterative, with no recursion, and every offset is computed
//! with checked arithmetic: a declared length is attacker-controlled up to
//! `u32::MAX` and is compared with the bytes remaining before anything is
//! added to it.

mod crc;
mod issue;
mod metadata;

use core::ops::Range;

pub use issue::PngIssue;
pub use metadata::{
    chunk_class, read, AnyPngIssue, ChunkClass, PngExifBlock, PngMetadata, PngRegion,
    MAX_PNG_EXIF_BLOCKS, MAX_PNG_REGIONS, MAX_PNG_TRAILER_CHUNKS,
};


use crate::error::Error;

/// Most chunks one walk records. The chunk after that halts the walk with a
/// single [`Error::TooManyChunks`]. Far above JPEG's segment cap because a
/// large PNG carries one `IDAT` per 8 to 64 KiB of compressed data.
pub const MAX_PNG_CHUNKS: usize = 65_536;

/// Most ordinary issues one walk records. The next one is replaced by a single
/// terminal [`Error::TooManyIssues`] and the walk halts, so a walk reports at
/// most `MAX_PNG_ISSUES + 1`.
pub const MAX_PNG_ISSUES: usize = 1024;

/// The eight-byte PNG signature.
const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// The framing around a chunk's data: length, type and CRC.
const FRAME: usize = 12;

/// The largest chunk length the PNG specification allows (2^31 - 1).
const MAX_CHUNK_LEN: u32 = 0x7FFF_FFFF;

/// What a chunk is, by its type bytes alone (DEC-017). Contents are not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChunkKind {
    /// `IHDR`, the image header.
    Ihdr,
    /// `PLTE`, the palette.
    Plte,
    /// `IDAT`, compressed image data.
    Idat,
    /// `IEND`, the end of the image; it ends the walk.
    Iend,
    /// `eXIf`, a raw Exif (TIFF) payload.
    Exif,
    /// `iTXt`, international text (carries XMP).
    Itxt,
    /// `tEXt`, Latin-1 text.
    Text,
    /// `zTXt`, compressed text.
    Ztxt,
    /// `iCCP`, an embedded ICC profile.
    Iccp,
    /// Any other chunk, including one with a malformed type.
    Other,
}

pub(crate) fn kind_of(ty: [u8; 4]) -> ChunkKind {
    match &ty {
        b"IHDR" => ChunkKind::Ihdr,
        b"PLTE" => ChunkKind::Plte,
        b"IDAT" => ChunkKind::Idat,
        b"IEND" => ChunkKind::Iend,
        b"eXIf" => ChunkKind::Exif,
        b"iTXt" => ChunkKind::Itxt,
        b"tEXt" => ChunkKind::Text,
        b"zTXt" => ChunkKind::Ztxt,
        b"iCCP" => ChunkKind::Iccp,
        _ => ChunkKind::Other,
    }
}

/// One chunk, borrowing the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk<'a> {
    offset: usize,
    chunk_type: [u8; 4],
    data: &'a [u8],
    stored_crc: u32,
    crc_ok: bool,
    kind: ChunkKind,
    after_idat: bool,
}

impl<'a> Chunk<'a> {
    /// Input offset of the chunk's length field (the start of the chunk).
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// The four type bytes as found.
    pub fn chunk_type(&self) -> [u8; 4] {
        self.chunk_type
    }

    /// The chunk's data, borrowed from the input.
    pub fn data(&self) -> &'a [u8] {
        self.data
    }

    /// The CRC-32 the file stored for this chunk.
    pub fn stored_crc(&self) -> u32 {
        self.stored_crc
    }

    /// Whether the stored CRC equals the CRC computed over type and data.
    pub fn crc_ok(&self) -> bool {
        self.crc_ok
    }

    /// What the chunk is, by type bytes alone.
    pub fn kind(&self) -> ChunkKind {
        self.kind
    }

    /// Whether the chunk comes after the first `IDAT` chunk (the first `IDAT`
    /// itself does not).
    pub fn after_idat(&self) -> bool {
        self.after_idat
    }

    /// The whole chunk in the input, length, type, data and CRC included.
    pub fn range(&self) -> Range<usize> {
        let end = self
            .offset
            .saturating_add(FRAME)
            .saturating_add(self.data.len());
        self.offset..end
    }
}

/// A walked PNG: the chunks, the issues found, and the trailer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Png<'a> {
    chunks: Vec<Chunk<'a>>,
    issues: Vec<PngIssue>,
    iend_offset: Option<usize>,
    trailer: &'a [u8],
    halted: bool,
}

/// The walk's mutable state. Every issue goes through [`Walk::record`], so no
/// code path can bypass the issue cap or record after a halt.
struct Walk<'a> {
    data: &'a [u8],
    chunks: Vec<Chunk<'a>>,
    issues: Vec<PngIssue>,
    /// Set by a halting issue or a cap: stops the walk and suppresses the
    /// end-of-walk issues.
    halted: bool,
    iend: Option<usize>,
    /// End of the `IEND` chunk, where the trailer starts.
    iend_end: Option<usize>,
}

fn be_u32(data: &[u8], at: usize) -> Option<u32> {
    let end = at.checked_add(4)?;
    let b: [u8; 4] = data.get(at..end)?.try_into().ok()?;
    Some(u32::from_be_bytes(b))
}

/// One chunk framed out of the input: length, type, data and stored CRC.
pub(crate) struct FramedChunk<'a> {
    pub(crate) ty: [u8; 4],
    pub(crate) length: u32,
    pub(crate) payload: &'a [u8],
    pub(crate) stored: u32,
    /// Input offset just past the chunk's CRC.
    pub(crate) end: usize,
}

/// What framing at one offset found.
pub(crate) enum Frame<'a> {
    Chunk(FramedChunk<'a>),
    /// Fewer than [`FRAME`] bytes remain.
    Short,
    /// The declared length is above [`MAX_CHUNK_LEN`].
    BadLength(u32),
    /// The declared length runs past the input; `total` is the whole chunk.
    PastEnd {
        total: usize,
    },
}

/// Frame the chunk that starts at input offset `p` (SPEC-029: the one framing
/// rule, shared by the walk and the trailer scan). Nothing is checked beyond
/// the length: not the CRC and not the type.
pub(crate) fn frame(data: &[u8], p: usize) -> Frame<'_> {
    let remaining = data.len().saturating_sub(p);
    if remaining < FRAME {
        return Frame::Short;
    }
    let (Some(length), Some(ty_start)) = (be_u32(data, p), p.checked_add(4)) else {
        return Frame::Short;
    };
    let Some(ty) = data
        .get(ty_start..)
        .and_then(|r| r.get(..4))
        .and_then(|b| <[u8; 4]>::try_from(b).ok())
    else {
        return Frame::Short;
    };
    if length > MAX_CHUNK_LEN {
        return Frame::BadLength(length);
    }
    // `length` fits in 31 bits, so the conversion only fails on a 16-bit
    // target; treat that as running past the input.
    let data_len = usize::try_from(length).unwrap_or(usize::MAX);
    let total = data_len.saturating_add(FRAME);
    if total > remaining {
        return Frame::PastEnd { total };
    }
    // `total <= remaining`, so none of these can overflow.
    let data_start = ty_start.saturating_add(4);
    let data_end = data_start.saturating_add(data_len);
    let end = data_end.saturating_add(4);
    let (Some(payload), Some(stored)) = (data.get(data_start..data_end), be_u32(data, data_end))
    else {
        return Frame::Short;
    };
    Frame::Chunk(FramedChunk {
        ty,
        length,
        payload,
        stored,
        end,
    })
}

impl<'a> Walk<'a> {
    /// The one place an issue is recorded. After a halt nothing is recorded.
    /// When the ordinary-issue count has reached [`MAX_PNG_ISSUES`] the issue
    /// is replaced by a single terminal `TooManyIssues` and the walk halts.
    fn record(&mut self, error: Error, offset: usize) {
        if self.halted {
            return;
        }
        if self.issues.len() >= MAX_PNG_ISSUES {
            self.issues.push(PngIssue::new(
                Error::TooManyIssues {
                    limit: MAX_PNG_ISSUES,
                },
                offset,
            ));
            self.halted = true;
            return;
        }
        self.issues.push(PngIssue::new(error, offset));
    }

    /// Record a halting issue and stop.
    fn halt(&mut self, error: Error, offset: usize) {
        self.record(error, offset);
        self.halted = true;
    }

    /// The chunk loop. Returns when the walk ends for any reason; the caller
    /// then applies the end-of-walk rule (which a halt suppresses).
    fn walk(&mut self) {
        let len = self.data.len();
        let mut p: usize = SIGNATURE.len();
        let mut seen_idat = false;
        while p < len && !self.halted {
            let framed = match frame(self.data, p) {
                Frame::Chunk(framed) => framed,
                Frame::Short => {
                    self.halt(Error::Truncated { at: p, len: FRAME }, p);
                    return;
                }
                Frame::BadLength(length) => {
                    self.halt(Error::BadChunkLength { length }, p);
                    return;
                }
                Frame::PastEnd { total } => {
                    self.halt(Error::Truncated { at: p, len: total }, p);
                    return;
                }
            };
            let FramedChunk {
                ty,
                length,
                payload,
                stored,
                end: chunk_end,
            } = framed;
            if self.chunks.len() >= MAX_PNG_CHUNKS {
                self.halt(
                    Error::TooManyChunks {
                        limit: MAX_PNG_CHUNKS,
                    },
                    p,
                );
                return;
            }

            let kind = kind_of(ty);
            if !ty.iter().all(u8::is_ascii_alphabetic) {
                self.record(Error::BadChunkType { chunk_type: ty }, p);
            }
            if self.chunks.is_empty() && kind != ChunkKind::Ihdr {
                self.record(Error::MissingIhdr, p);
            }
            if kind == ChunkKind::Ihdr && length != 13 {
                self.record(Error::BadIhdr { length }, p);
            }
            let computed = crc::crc32_parts(&ty, payload);
            let crc_ok = computed == stored;
            if !crc_ok {
                self.record(Error::BadChunkCrc { stored, computed }, p);
            }
            if self.halted {
                // The issue cap fired on this chunk: it is not recorded.
                return;
            }

            self.chunks.push(Chunk {
                offset: p,
                chunk_type: ty,
                data: payload,
                stored_crc: stored,
                crc_ok,
                kind,
                after_idat: seen_idat,
            });
            if kind == ChunkKind::Idat {
                seen_idat = true;
            }
            if kind == ChunkKind::Iend {
                self.iend = Some(p);
                self.iend_end = Some(chunk_end);
                return;
            }
            p = chunk_end;
        }
        // Reached the end of the input on a chunk boundary.
        if !self.halted {
            if self.chunks.is_empty() {
                self.record(Error::MissingIhdr, len);
            }
            self.record(Error::MissingIend, len);
        }
    }
}

impl<'a> Png<'a> {
    /// Walk `data`. `Err` only when it is not a PNG at all (fewer than eight
    /// bytes, or no PNG signature); every other problem is a [`PngIssue`]
    /// (DEC-017).
    pub fn parse(data: &'a [u8]) -> Result<Png<'a>, Error> {
        let Some(head) = data.get(..SIGNATURE.len()) else {
            return Err(Error::Truncated {
                at: 0,
                len: SIGNATURE.len(),
            });
        };
        if head != SIGNATURE {
            let mut mark = [0u8; 8];
            mark.copy_from_slice(head);
            return Err(Error::NotPng { mark });
        }
        let mut walk = Walk {
            data,
            chunks: Vec::new(),
            issues: Vec::new(),
            halted: false,
            iend: None,
            iend_end: None,
        };
        walk.walk();
        // Without an IEND the trailer is the empty tail of the input, so even
        // the empty slice is borrowed from `data`.
        let trailer_start = walk.iend_end.unwrap_or(data.len());
        let trailer = data.get(trailer_start..).unwrap_or(&[]);
        Ok(Png {
            chunks: walk.chunks,
            issues: walk.issues,
            iend_offset: walk.iend,
            trailer,
            halted: walk.halted,
        })
    }

    /// Like [`Png::parse`], but the first issue becomes the `Err` (by value).
    pub fn parse_strict(data: &'a [u8]) -> Result<Png<'a>, Error> {
        let png = Png::parse(data)?;
        match png.issues.first() {
            Some(first) => Err(first.error().clone()),
            None => Ok(png),
        }
    }

    /// The recorded chunks in input order, at most [`MAX_PNG_CHUNKS`].
    pub fn chunks(&self) -> &[Chunk<'a>] {
        &self.chunks
    }

    /// The recoverable problems found, in the order found.
    pub fn issues(&self) -> &[PngIssue] {
        &self.issues
    }

    /// Whether the walk found no issue.
    pub fn is_clean(&self) -> bool {
        self.issues.is_empty()
    }

    /// Whether the walk was halted (by a framing error or a cap).
    pub(crate) fn halted(&self) -> bool {
        self.halted
    }

    /// Offset of the `IEND` chunk, if the walk reached one.
    pub fn iend_offset(&self) -> Option<usize> {
        self.iend_offset
    }

    /// The bytes after the `IEND` chunk, borrowed. Not walked and not an
    /// issue (DEC-017); empty when there is no `IEND`.
    pub fn trailer(&self) -> &'a [u8] {
        self.trailer
    }
}
