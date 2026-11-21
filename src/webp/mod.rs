//! A panic-free, zero-copy WebP RIFF chunk walker (SPEC-030, DEC-019).
//!
//! A WebP is a 12-byte header (`RIFF`, a little-endian size, `WEBP`) followed
//! by chunks: a four-character code, a 4-byte little-endian size, the data and,
//! when the size is odd, one pad byte. [`Webp::parse`] returns the ordered
//! chunk list, the optional `VP8X` header info and a list of per-chunk
//! [`WebpIssue`]s. Only "not a WebP at all" is an `Err` (DEC-019); everything
//! else is an issue and the walk continues wherever it safely can.
//!
//! The walk runs to the **end of the input**, not to the size the RIFF header
//! declares: a size that is too small, zero, odd or too large is one
//! [`Error::RiffSizeMismatch`] issue and nothing more, because readers such as
//! ExifTool read past it and the audit must see what they see.
//!
//! Nothing in the walk reads chunk contents except the `VP8X` header: chunks
//! are identified by their four-character code only, case-sensitively.
//! Contents are located by [`read`] (DEC-019).
//!
//! Provenance (DEC-003, class 1): net-new code written from the WebP Container
//! Specification (Google) and the RIFF container format (chunk layout, pad
//! byte, `VP8X` flags and canvas fields). Nothing is ported.
//!
//! Progress argument: every loop iteration either ends the walk or advances
//! the cursor by at least 8 bytes (a whole chunk header), so the walk is linear
//! in the input. It is iterative, with no recursion, and every size is compared
//! with the bytes remaining, as a 64-bit-safe saturating value, before any
//! offset is formed from it: a declared size is attacker-controlled up to
//! `u32::MAX`.

mod issue;
// mod metadata;

use core::ops::Range;

pub use issue::WebpIssue;
// pub use metadata::{
//     read, AnyWebpIssue, WebpExifBlock, WebpMetadata, WebpRegion, MAX_WEBP_EXIF_BLOCKS,
// };

use crate::error::Error;

/// Most chunks one walk records. The chunk after that halts the walk with a
/// single [`Error::TooManyChunks`]. An animated WebP has one `ANMF` per frame.
pub const MAX_WEBP_CHUNKS: usize = 65_536;

/// Most ordinary issues one walk records. The next one is replaced by a single
/// terminal [`Error::TooManyIssues`] and the walk halts, so a walk reports at
/// most `MAX_WEBP_ISSUES + 1`.
pub const MAX_WEBP_ISSUES: usize = 1024;

/// Bytes of the RIFF header: `RIFF`, the size field, `WEBP`.
const HEADER: usize = 12;

/// Bytes of chunk framing before the data: code and size.
const CHUNK_HEAD: usize = 8;

/// Bytes a `VP8X` chunk needs for its flags and canvas size.
const VP8X_LEN: usize = 10;

/// What a chunk is, by its four-character code alone (DEC-019). The codes are
/// case-sensitive. Contents are not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChunkKind {
    /// `VP8X`, the extended-format header.
    Vp8x,
    /// `VP8 `, a lossy bitstream.
    Vp8,
    /// `VP8L`, a lossless bitstream.
    Vp8l,
    /// `ALPH`, alpha data for a lossy image.
    Alph,
    /// `ANIM`, animation parameters.
    Anim,
    /// `ANMF`, one animation frame. Its payload is passed over, never walked.
    Anmf,
    /// `ICCP`, an ICC profile.
    Iccp,
    /// `EXIF`, a raw Exif (TIFF) payload.
    Exif,
    /// `XMP `, an XMP packet. `XMP\0` is read as this too, with an issue.
    Xmp,
    /// Any other chunk, including every wrong-case spelling of the above.
    Other,
}

/// The kind a code names, and whether the code is a non-canonical spelling
/// that is read anyway (`XMP\0`, which ExifTool reads with a warning).
fn kind_of(code: [u8; 4]) -> (ChunkKind, bool) {
    match &code {
        b"VP8X" => (ChunkKind::Vp8x, false),
        b"VP8 " => (ChunkKind::Vp8, false),
        b"VP8L" => (ChunkKind::Vp8l, false),
        b"ALPH" => (ChunkKind::Alph, false),
        b"ANIM" => (ChunkKind::Anim, false),
        b"ANMF" => (ChunkKind::Anmf, false),
        b"ICCP" => (ChunkKind::Iccp, false),
        b"EXIF" => (ChunkKind::Exif, false),
        b"XMP " => (ChunkKind::Xmp, false),
        b"XMP\0" => (ChunkKind::Xmp, true),
        _ => (ChunkKind::Other, false),
    }
}

/// One chunk, borrowing the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk<'a> {
    offset: usize,
    fourcc: [u8; 4],
    data: &'a [u8],
    kind: ChunkKind,
    end: usize,
}

impl<'a> Chunk<'a> {
    /// Input offset of the chunk's four-character code (the start of the
    /// chunk).
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// The four code bytes as found.
    pub fn fourcc(&self) -> [u8; 4] {
        self.fourcc
    }

    /// The chunk's data, borrowed from the input, without the pad byte.
    pub fn data(&self) -> &'a [u8] {
        self.data
    }

    /// What the chunk is, by its code alone.
    pub fn kind(&self) -> ChunkKind {
        self.kind
    }

    /// The whole chunk in the input: code, size, data and the pad byte when
    /// the chunk has one.
    pub fn range(&self) -> Range<usize> {
        self.offset..self.end
    }
}

/// The header of a `VP8X` chunk: feature flags and the canvas size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vp8x {
    flags: u8,
    width: u32,
    height: u32,
}

impl Vp8x {
    /// Decode the flags and canvas from a `VP8X` payload of at least 10 bytes.
    fn from_data(data: &[u8]) -> Option<Vp8x> {
        let flags = *data.first()?;
        let le24 = |at: usize| -> Option<u32> {
            let b = data.get(at..at.checked_add(3)?)?;
            let &[b0, b1, b2] = b else { return None };
            Some(u32::from_le_bytes([b0, b1, b2, 0]))
        };
        // Each canvas dimension is stored minus one, in 24 bits.
        Some(Vp8x {
            flags,
            width: le24(4)?.saturating_add(1),
            height: le24(7)?.saturating_add(1),
        })
    }

    /// The flag byte as found.
    pub fn flags(&self) -> u8 {
        self.flags
    }

    /// The ICC-profile flag.
    pub fn has_icc(&self) -> bool {
        self.flags & FLAG_ICC != 0
    }

    /// The alpha flag.
    pub fn has_alpha(&self) -> bool {
        self.flags & FLAG_ALPHA != 0
    }

    /// The Exif flag.
    pub fn has_exif(&self) -> bool {
        self.flags & FLAG_EXIF != 0
    }

    /// The XMP flag.
    pub fn has_xmp(&self) -> bool {
        self.flags & FLAG_XMP != 0
    }

    /// The animation flag.
    pub fn is_animated(&self) -> bool {
        self.flags & FLAG_ANIM != 0
    }

    /// The canvas width in pixels (the stored value plus one).
    pub fn canvas_width(&self) -> u32 {
        self.width
    }

    /// The canvas height in pixels (the stored value plus one).
    pub fn canvas_height(&self) -> u32 {
        self.height
    }
}

const FLAG_ICC: u8 = 0x20;
const FLAG_ALPHA: u8 = 0x10;
const FLAG_EXIF: u8 = 0x08;
const FLAG_XMP: u8 = 0x04;
const FLAG_ANIM: u8 = 0x02;

/// A walked WebP: the chunks, the issues found and the `VP8X` header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Webp<'a> {
    chunks: Vec<Chunk<'a>>,
    issues: Vec<WebpIssue>,
    declared_size: u32,
    vp8x: Option<Vp8x>,
}

/// The walk's mutable state. Every issue goes through [`Walk::record`], so no
/// code path can bypass the issue cap or record after a halt.
struct Walk<'a> {
    data: &'a [u8],
    chunks: Vec<Chunk<'a>>,
    issues: Vec<WebpIssue>,
    /// Set by a halting issue or a cap: stops the walk and suppresses the
    /// end-of-walk issues.
    halted: bool,
    /// A `VP8X` chunk has been seen (of any length).
    seen_vp8x: bool,
    /// Offset and header of the first `VP8X` chunk, when it is long enough.
    vp8x: Option<(usize, Vp8x)>,
}

fn le_u32(data: &[u8], at: usize) -> Option<u32> {
    let end = at.checked_add(4)?;
    let b: [u8; 4] = data.get(at..end)?.try_into().ok()?;
    Some(u32::from_le_bytes(b))
}

impl<'a> Walk<'a> {
    /// The one place an issue is recorded. After a halt nothing is recorded.
    /// When the ordinary-issue count has reached [`MAX_WEBP_ISSUES`] the issue
    /// is replaced by a single terminal `TooManyIssues` and the walk halts.
    fn record(&mut self, error: Error, offset: usize) {
        if self.halted {
            return;
        }
        if self.issues.len() >= MAX_WEBP_ISSUES {
            self.issues.push(WebpIssue::new(
                Error::TooManyIssues {
                    limit: MAX_WEBP_ISSUES,
                },
                offset,
            ));
            self.halted = true;
            return;
        }
        self.issues.push(WebpIssue::new(error, offset));
    }

    /// Record a halting issue and stop.
    fn halt(&mut self, error: Error, offset: usize) {
        self.record(error, offset);
        self.halted = true;
    }

    /// The chunk loop. Returns when the walk ends for any reason.
    fn walk(&mut self) {
        let len = self.data.len();
        let mut p: usize = HEADER;
        while p < len && !self.halted {
            let remaining = len.saturating_sub(p);
            if remaining < CHUNK_HEAD {
                self.halt(
                    Error::Truncated {
                        at: p,
                        len: CHUNK_HEAD,
                    },
                    p,
                );
                return;
            }
            let (Some(size), Some(code)) = (
                le_u32(self.data, p.saturating_add(4)),
                self.data
                    .get(p..p.saturating_add(4))
                    .and_then(|b| <[u8; 4]>::try_from(b).ok()),
            ) else {
                self.halt(
                    Error::Truncated {
                        at: p,
                        len: CHUNK_HEAD,
                    },
                    p,
                );
                return;
            };
            // `size` is at most u32::MAX; the sum cannot overflow a 64-bit
            // usize and saturates on a narrower one.
            let size_len = usize::try_from(size).unwrap_or(usize::MAX);
            let total = size_len.saturating_add(CHUNK_HEAD);
            if total > remaining {
                self.halt(Error::Truncated { at: p, len: total }, p);
                return;
            }
            // `total <= remaining`, so none of these can overflow.
            let data_start = p.saturating_add(CHUNK_HEAD);
            let data_end = data_start.saturating_add(size_len);
            let Some(payload) = self.data.get(data_start..data_end) else {
                self.halt(Error::Truncated { at: p, len: total }, p);
                return;
            };
            if self.chunks.len() >= MAX_WEBP_CHUNKS {
                self.halt(
                    Error::TooManyChunks {
                        limit: MAX_WEBP_CHUNKS,
                    },
                    p,
                );
                return;
            }

            let (kind, non_canonical) = kind_of(code);
            if non_canonical {
                self.record(
                    Error::NonCanonicalChunkId {
                        chunk_id: code,
                        at: p,
                    },
                    p,
                );
            }
            if kind == ChunkKind::Vp8x {
                if size_len < VP8X_LEN {
                    self.record(Error::BadVp8x { length: size }, p);
                }
                if !self.chunks.is_empty() {
                    self.record(Error::MisplacedVp8x { at: p }, p);
                }
                if !self.seen_vp8x {
                    self.seen_vp8x = true;
                    self.vp8x = Vp8x::from_data(payload).map(|v| (p, v));
                }
            }
            // An odd size has a pad byte, unless the input ends first.
            let odd = size & 1 == 1;
            let end = if odd && data_end < len {
                data_end.saturating_add(1)
            } else {
                data_end
            };
            if odd && data_end >= len {
                self.record(Error::MissingPad { at: p }, p);
            }
            if self.halted {
                // The issue cap fired on this chunk: it is not recorded.
                return;
            }

            self.chunks.push(Chunk {
                offset: p,
                fourcc: code,
                data: payload,
                kind,
                end,
            });
            // Every chunk is at least 8 bytes, so this always advances.
            p = end;
        }
    }

    /// The end-of-walk rules (skipped after a halt): `VP8X` flags against the
    /// chunks present, in the order ICC, EXIF, XMP; and metadata chunks with
    /// no `VP8X` at all.
    fn finish(&mut self) {
        if self.halted {
            return;
        }
        let has = |kind: ChunkKind, chunks: &[Chunk<'_>]| chunks.iter().any(|c| c.kind == kind);
        if let Some((at, v)) = self.vp8x {
            let checks = [
                ("ICC", v.has_icc(), ChunkKind::Iccp),
                ("EXIF", v.has_exif(), ChunkKind::Exif),
                ("XMP", v.has_xmp(), ChunkKind::Xmp),
            ];
            for (flag, declared, kind) in checks {
                if declared != has(kind, &self.chunks) {
                    self.record(Error::ChunkFlagMismatch { flag, declared }, at);
                }
            }
        } else if !self.seen_vp8x {
            let first = self
                .chunks
                .iter()
                .find(|c| matches!(c.kind, ChunkKind::Iccp | ChunkKind::Exif | ChunkKind::Xmp));
            if let Some(c) = first {
                let at = c.offset;
                self.record(Error::MetadataWithoutVp8x, at);
            }
        }
    }
}

impl<'a> Webp<'a> {
    /// Walk `data`. `Err` only when it is not a WebP at all (fewer than 12
    /// bytes, or not `RIFF`, a size and `WEBP`); every other problem is a
    /// [`WebpIssue`] (DEC-019).
    pub fn parse(data: &'a [u8]) -> Result<Webp<'a>, Error> {
        let Some(head) = data.get(..HEADER) else {
            return Err(Error::Truncated { at: 0, len: HEADER });
        };
        let magic = head.get(..4);
        let form = head.get(8..HEADER);
        if magic != Some(b"RIFF".as_slice()) || form != Some(b"WEBP".as_slice()) {
            let mut mark = [0u8; HEADER];
            mark.copy_from_slice(head);
            return Err(Error::NotWebp { mark });
        }
        let declared = le_u32(data, 4).unwrap_or(0);
        let mut walk = Walk {
            data,
            chunks: Vec::new(),
            issues: Vec::new(),
            halted: false,
            seen_vp8x: false,
            vp8x: None,
        };
        // The declared size is compared, never trusted: one issue, first.
        let actual = data.len().saturating_sub(8);
        if usize::try_from(declared).ok() != Some(actual) {
            walk.record(Error::RiffSizeMismatch { declared, actual }, 4);
        }
        walk.walk();
        walk.finish();
        Ok(Webp {
            chunks: walk.chunks,
            issues: walk.issues,
            declared_size: declared,
            vp8x: walk.vp8x.map(|(_, v)| v),
        })
    }

    /// Like [`Webp::parse`], but the first issue becomes the `Err` (by value).
    pub fn parse_strict(data: &'a [u8]) -> Result<Webp<'a>, Error> {
        let webp = Webp::parse(data)?;
        match webp.issues.first() {
            Some(first) => Err(first.error().clone()),
            None => Ok(webp),
        }
    }

    /// The recorded chunks in input order, at most [`MAX_WEBP_CHUNKS`].
    pub fn chunks(&self) -> &[Chunk<'a>] {
        &self.chunks
    }

    /// The recoverable problems found, in the order found.
    pub fn issues(&self) -> &[WebpIssue] {
        &self.issues
    }

    /// Whether the walk found no issue.
    pub fn is_clean(&self) -> bool {
        self.issues.is_empty()
    }

    /// The size the RIFF header declared (not trusted; see the module docs).
    pub fn declared_size(&self) -> u32 {
        self.declared_size
    }

    /// The flags and canvas of the first `VP8X` chunk, when that chunk has the
    /// 10 bytes of a header.
    pub fn vp8x(&self) -> Option<Vp8x> {
        self.vp8x
    }
}
