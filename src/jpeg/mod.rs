//! A panic-free, zero-copy JPEG segment walker (SPEC-009, DEC-006).
//!
//! A JPEG is a marker stream: `SOI`, then segments (a `0xFF` byte, a marker
//! code and a 16-bit big-endian length that includes its own two bytes), the
//! metadata APPn segments before `SOS`, then entropy-coded scan data up to
//! `EOI`. [`Jpeg::parse`] returns the ordered segment list, the borrowed scan
//! region, the end-of-image offset and trailer, and a list of per-segment
//! [`JpegIssue`]s. Only "not a JPEG at all" is an `Err` (DEC-006); everything
//! else is an issue and the walk continues wherever it safely can.
//!
//! Nothing here decodes: APPn segments are identified by their signature only.
//!
//! Provenance (DEC-003, class 1): net-new code written from the format
//! specifications, ITU-T T.81 Annex B (marker syntax), JFIF, and the Adobe XMP
//! specification part 3 (JPEG embedding). Nothing is ported.
//!
//! # Progress
//!
//! Every iteration of the walk loop advances the cursor by at least one byte
//! or ends the walk: a stray run is at least one byte long, a fill byte
//! advances by one, every marker branch advances by at least two, and the scan
//! search only moves forward. The loop is iterative; there is no recursion.

mod issue;

use core::fmt;

use crate::error::Error;

pub use issue::JpegIssue;

/// Most segments one walk records. The `MAX_JPEG_SEGMENTS + 1`-th halts the
/// walk with [`Error::TooManySegments`].
pub const MAX_JPEG_SEGMENTS: usize = 4096;

/// Most ordinary issues one walk records. The next one is replaced by a single
/// terminal [`Error::TooManyIssues`], so `issues().len()` is at most
/// `MAX_JPEG_ISSUES + 1`.
pub const MAX_JPEG_ISSUES: usize = 1024;

const SIG_JFIF: &[u8] = b"JFIF\0";
const SIG_EXIF: &[u8] = b"Exif\0\0";
const SIG_XMP: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";
const SIG_EXTENDED_XMP: &[u8] = b"http://ns.adobe.com/xmp/extension/\0";
const SIG_ICC: &[u8] = b"ICC_PROFILE\0";
const SIG_IPTC: &[u8] = b"Photoshop 3.0\0";
const SIG_JUMBF: &[u8] = b"JP";

/// What a segment is, from its marker and (for APPn) its signature.
/// Identification only; the bytes behind a signature are not interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SegmentKind {
    /// APP0 `JFIF\0`.
    Jfif,
    /// APP1 `Exif\0\0`.
    Exif,
    /// APP1 `http://ns.adobe.com/xap/1.0/\0`.
    Xmp,
    /// APP1 `http://ns.adobe.com/xmp/extension/\0`.
    ExtendedXmp,
    /// APP2 `ICC_PROFILE\0`.
    Icc,
    /// APP13 `Photoshop 3.0\0`.
    Iptc,
    /// APP11 starting `JP` (JPEG XT / JUMBF header).
    Jumbf,
    /// Any other APPn, n = 0..=15.
    App(u8),
    /// `0xC0..=0xCF` except `0xC4`, `0xC8` and `0xCC`; carries the marker code.
    Sof(u8),
    /// Define Huffman table (`0xC4`).
    Dht,
    /// Define quantisation table (`0xDB`).
    Dqt,
    /// Define restart interval (`0xDD`).
    Dri,
    /// Comment (`0xFE`).
    Com,
    /// Start of scan (`0xDA`).
    Sos,
    /// Any other marker that carries a length.
    Other(u8),
}

impl fmt::Display for SegmentKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SegmentKind::Jfif => write!(f, "JFIF"),
            SegmentKind::Exif => write!(f, "Exif"),
            SegmentKind::Xmp => write!(f, "XMP"),
            SegmentKind::ExtendedXmp => write!(f, "extended XMP"),
            SegmentKind::Icc => write!(f, "ICC profile"),
            SegmentKind::Iptc => write!(f, "IPTC (Photoshop 3.0)"),
            SegmentKind::Jumbf => write!(f, "JUMBF"),
            SegmentKind::App(n) => write!(f, "APP{n}"),
            SegmentKind::Sof(m) => write!(f, "SOF marker {m:#04x}"),
            SegmentKind::Dht => write!(f, "DHT"),
            SegmentKind::Dqt => write!(f, "DQT"),
            SegmentKind::Dri => write!(f, "DRI"),
            SegmentKind::Com => write!(f, "COM"),
            SegmentKind::Sos => write!(f, "SOS"),
            SegmentKind::Other(m) => write!(f, "marker {m:#04x}"),
        }
    }
}

/// One length-carrying segment, borrowed from the input.
#[derive(Debug, Clone, Copy)]
pub struct Segment<'a> {
    marker: u8,
    offset: usize,
    payload: &'a [u8],
    kind: SegmentKind,
    after_scan: bool,
}

impl<'a> Segment<'a> {
    /// The marker code byte (the byte after `FF`).
    pub fn marker(&self) -> u8 {
        self.marker
    }

    /// Input offset of the segment's own `FF` marker byte.
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// The bytes after the two length bytes, borrowed from the input (DEC-002).
    pub fn payload(&self) -> &'a [u8] {
        self.payload
    }

    /// What the segment is, by marker and signature.
    pub fn kind(&self) -> SegmentKind {
        self.kind
    }

    /// Whether the segment appears after the first `SOS` marker. The first
    /// `SOS` itself is not after the scan.
    pub fn after_scan(&self) -> bool {
        self.after_scan
    }
}

/// The kind of a length-carrying marker `m` with payload `payload`.
fn classify(m: u8, payload: &[u8]) -> SegmentKind {
    match m {
        0xE0 if payload.starts_with(SIG_JFIF) => SegmentKind::Jfif,
        0xE1 if payload.starts_with(SIG_EXIF) => SegmentKind::Exif,
        0xE1 if payload.starts_with(SIG_XMP) => SegmentKind::Xmp,
        0xE1 if payload.starts_with(SIG_EXTENDED_XMP) => SegmentKind::ExtendedXmp,
        0xE2 if payload.starts_with(SIG_ICC) => SegmentKind::Icc,
        0xED if payload.starts_with(SIG_IPTC) => SegmentKind::Iptc,
        0xEB if payload.starts_with(SIG_JUMBF) => SegmentKind::Jumbf,
        0xE0..=0xEF => SegmentKind::App(m & 0x0F),
        0xC4 => SegmentKind::Dht,
        0xC8 | 0xCC => SegmentKind::Other(m),
        0xC0..=0xCF => SegmentKind::Sof(m),
        0xDB => SegmentKind::Dqt,
        0xDD => SegmentKind::Dri,
        0xFE => SegmentKind::Com,
        0xDA => SegmentKind::Sos,
        _ => SegmentKind::Other(m),
    }
}

/// The walk's mutable state. Every issue goes through [`Walk::record`], so no
/// code path can bypass the issue cap or record after a halt.
struct Walk<'a> {
    data: &'a [u8],
    segments: Vec<Segment<'a>>,
    issues: Vec<JpegIssue>,
    /// Set by a cap: stops the walk and suppresses the end-of-walk issues.
    halted: bool,
    scan_start: Option<usize>,
    eoi: Option<usize>,
}

impl<'a> Walk<'a> {
    /// The one place an issue is recorded. After a halt nothing is recorded.
    /// When the ordinary-issue count has reached [`MAX_JPEG_ISSUES`] the issue
    /// is replaced by a single terminal `TooManyIssues` and the walk halts.
    fn record(&mut self, error: Error, offset: usize) {
        if self.halted {
            return;
        }
        if self.issues.len() >= MAX_JPEG_ISSUES {
            self.issues.push(JpegIssue::new(
                Error::TooManyIssues {
                    limit: MAX_JPEG_ISSUES,
                },
                offset,
            ));
            self.halted = true;
            return;
        }
        let halts = matches!(error, Error::TooManySegments { .. });
        self.issues.push(JpegIssue::new(error, offset));
        if halts {
            self.halted = true;
        }
    }

    /// Rule 2: the marker loop. Returns when the walk ends for any reason; the
    /// caller then applies the end-of-walk rule (which a halt suppresses).
    fn walk(&mut self) {
        let len = self.data.len();
        let mut p: usize = 2;
        while p < len && !self.halted {
            let Some(&first) = self.data.get(p) else {
                return;
            };
            // 2.1: a maximal run of non-FF bytes is one issue.
            if first != 0xFF {
                let run = self
                    .data
                    .get(p..)
                    .and_then(|rest| rest.iter().position(|&b| b == 0xFF))
                    .unwrap_or_else(|| len.saturating_sub(p));
                self.record(Error::StrayBytes { at: p, len: run }, p);
                p = p.saturating_add(run);
                continue;
            }
            // 2.2 / 2.3: FF fill. `p` ends on the marker's own FF.
            let m = loop {
                match self.data.get(p.saturating_add(1)) {
                    None => {
                        self.record(Error::Truncated { at: p, len: 2 }, p);
                        return;
                    }
                    Some(&0xFF) => p = p.saturating_add(1),
                    Some(&m) => break m,
                }
            };
            match m {
                // 2.4: FF 00 outside entropy data.
                0x00 => {
                    self.record(Error::StrayBytes { at: p, len: 2 }, p);
                    p = p.saturating_add(2);
                }
                // 2.5: EOI ends the walk; the rest is the trailer.
                0xD9 => {
                    self.eoi = Some(p);
                    return;
                }
                // 2.6: standalone markers carry no segment.
                0x01 | 0xD0..=0xD8 => {
                    self.record(Error::StrayBytes { at: p, len: 2 }, p);
                    p = p.saturating_add(2);
                }
                // 2.7: a marker with a length.
                _ => {
                    let Some(next) = self.length_marker(p, m) else {
                        return;
                    };
                    p = next;
                }
            }
        }
    }

    /// Rule 2.7 and 2.8 for a length-carrying marker `m` whose `FF` is at `p`.
    /// Returns the new cursor, or `None` when the walk ends.
    fn length_marker(&mut self, p: usize, m: u8) -> Option<usize> {
        let len = self.data.len();
        let header_end = p.saturating_add(4);
        let declared = match self.data.get(p.saturating_add(2)..header_end) {
            Some(&[hi, lo]) => u16::from_be_bytes([hi, lo]),
            _ => {
                self.record(Error::Truncated { at: p, len: 4 }, p);
                return None;
            }
        };
        if declared < 2 {
            self.record(
                Error::BadSegmentLength {
                    marker: m,
                    length: declared,
                },
                p,
            );
            return Some(p.saturating_add(2));
        }
        let total = usize::from(declared).saturating_add(2);
        let end = p.saturating_add(total);
        let Some(payload) = (end <= len)
            .then(|| self.data.get(header_end..end))
            .flatten()
        else {
            // The segment is omitted entirely, never partially recorded.
            self.record(Error::Truncated { at: p, len: total }, p);
            return None;
        };
        if self.segments.len() >= MAX_JPEG_SEGMENTS {
            self.record(
                Error::TooManySegments {
                    limit: MAX_JPEG_SEGMENTS,
                },
                p,
            );
            return None;
        }
        self.segments.push(Segment {
            marker: m,
            offset: p,
            payload,
            kind: classify(m, payload),
            after_scan: self.scan_start.is_some(),
        });
        if m == 0xDA {
            if self.scan_start.is_none() {
                self.scan_start = Some(p);
            }
            return Some(self.skip_entropy(end));
        }
        Some(end)
    }

    /// Rule 2.8: the offset of the next real marker after scan data starting at
    /// `from`, or the input length when there is none. Stuffed `FF 00` and
    /// restart `FF D0..=D7` are data; `FF FF` is fill.
    fn skip_entropy(&self, from: usize) -> usize {
        let len = self.data.len();
        let mut i = from;
        loop {
            let Some(j) = self
                .data
                .get(i..)
                .and_then(|rest| rest.iter().position(|&b| b == 0xFF))
                .map(|k| i.saturating_add(k))
            else {
                return len;
            };
            match self.data.get(j.saturating_add(1)) {
                // A lone FF at the very end: no marker follows.
                None => return len,
                Some(0x00) | Some(0xD0..=0xD7) => i = j.saturating_add(2),
                Some(0xFF) => i = j.saturating_add(1),
                Some(_) => return j,
            }
        }
    }
}

/// The result of walking a JPEG: segments, issues, scan region and trailer.
#[derive(Debug, Clone)]
pub struct Jpeg<'a> {
    segments: Vec<Segment<'a>>,
    issues: Vec<JpegIssue>,
    scan_region: Option<&'a [u8]>,
    eoi_offset: Option<usize>,
    trailer: &'a [u8],
}

impl<'a> Jpeg<'a> {
    /// Walk `data`. `Err` only when it is not a JPEG at all (fewer than two
    /// bytes, or no `FF D8`); every other problem is a [`JpegIssue`] (DEC-006).
    pub fn parse(data: &'a [u8]) -> Result<Jpeg<'a>, Error> {
        let (a, b) = match data.get(..2) {
            Some(&[a, b]) => (a, b),
            _ => return Err(Error::Truncated { at: 0, len: 2 }),
        };
        if [a, b] != [0xFF, 0xD8] {
            return Err(Error::NotJpeg { mark: [a, b] });
        }
        let mut walk = Walk {
            data,
            segments: Vec::new(),
            issues: Vec::new(),
            halted: false,
            scan_start: None,
            eoi: None,
        };
        walk.walk();

        // Rule 3: a halt by a cap suppresses these (`record` is a no-op then).
        let len = data.len();
        if !walk.halted {
            if walk.scan_start.is_none() {
                walk.record(Error::MissingSos, len);
            }
            if walk.eoi.is_none() {
                walk.record(Error::MissingEoi, len);
            }
        }

        let scan_region = walk.scan_start.and_then(|start| {
            let end = walk.eoi.map_or(len, |e| e.saturating_add(2));
            data.get(start..end)
        });
        // Without an EOI the trailer is the empty tail of the input, so even
        // the empty slice is borrowed from `data`.
        let trailer_start = walk.eoi.map_or(len, |e| e.saturating_add(2));
        let trailer = data.get(trailer_start..).unwrap_or(&[]);
        Ok(Jpeg {
            segments: walk.segments,
            issues: walk.issues,
            scan_region,
            eoi_offset: walk.eoi,
            trailer,
        })
    }

    /// Like [`Jpeg::parse`], but the first issue becomes the `Err` (by value).
    pub fn parse_strict(data: &'a [u8]) -> Result<Jpeg<'a>, Error> {
        let jpeg = Jpeg::parse(data)?;
        match jpeg.issues.first() {
            Some(first) => Err(first.error().clone()),
            None => Ok(jpeg),
        }
    }

    /// The recorded segments in input order.
    pub fn segments(&self) -> &[Segment<'a>] {
        &self.segments
    }

    /// The recoverable problems found, in the order found.
    pub fn issues(&self) -> &[JpegIssue] {
        &self.issues
    }

    /// Whether the walk found no issue.
    pub fn is_clean(&self) -> bool {
        self.issues.is_empty()
    }

    /// From the first `SOS` marker to the end of `EOI` (or of the input when
    /// there is no `EOI`); `None` without an `SOS`.
    pub fn scan_region(&self) -> Option<&'a [u8]> {
        self.scan_region
    }

    /// Offset of the `EOI` marker's `FF`.
    pub fn eoi_offset(&self) -> Option<usize> {
        self.eoi_offset
    }

    /// The bytes after `EOI`; empty when there is no `EOI`.
    pub fn trailer(&self) -> &'a [u8] {
        self.trailer
    }
}

#[cfg(test)]
#[allow(
    // Test code is not a parse path exercised on untrusted input (AGENTS.md
    // §11); same sanctioned exception as the TIFF module's tests.
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::arithmetic_side_effects
)]
mod tests {
    use super::*;

    // A small byte builder, independent of the walker.
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
    fn cat(parts: &[&[u8]]) -> Vec<u8> {
        parts.concat()
    }
    fn errors(j: &Jpeg<'_>) -> Vec<Error> {
        j.issues().iter().map(|i| i.error().clone()).collect()
    }
    fn com() -> Vec<u8> {
        seg(0xFE, b"hi")
    }
    fn sos() -> Vec<u8> {
        seg(0xDA, &[1, 1, 0, 0, 0x3F, 0])
    }

    // ── fatal and trivial ────────────────────────────────────────────────────

    #[test]
    fn empty_and_one_byte_inputs_are_truncated() {
        for d in [&[][..], &[0xFF][..]] {
            assert_eq!(
                Jpeg::parse(d).unwrap_err(),
                Error::Truncated { at: 0, len: 2 }
            );
        }
    }

    #[test]
    fn a_non_jpeg_is_rejected_with_its_mark() {
        assert_eq!(
            Jpeg::parse(&[0x89, 0x50]).unwrap_err(),
            Error::NotJpeg { mark: [0x89, 0x50] }
        );
    }

    #[test]
    fn soi_only_reports_missing_sos_then_missing_eoi() {
        let d = soi();
        let j = Jpeg::parse(&d).unwrap();
        assert!(j.segments().is_empty());
        assert_eq!(errors(&j), [Error::MissingSos, Error::MissingEoi]);
        assert!(j.issues().iter().all(|i| i.offset() == 2));
        assert!(j.scan_region().is_none());
        assert_eq!(j.eoi_offset(), None);
        assert!(j.trailer().is_empty());
        assert_eq!(Jpeg::parse_strict(&d).unwrap_err(), Error::MissingSos);
    }

    // ── happy path and kinds ─────────────────────────────────────────────────

    #[test]
    fn a_minimal_jpeg_walks_clean() {
        let mut d = soi();
        let mut expect: Vec<(u8, usize, Vec<u8>)> = Vec::new();
        let push = |d: &mut Vec<u8>, expect: &mut Vec<(u8, usize, Vec<u8>)>, m: u8, pl: &[u8]| {
            expect.push((m, d.len(), pl.to_vec()));
            d.extend_from_slice(&seg(m, pl));
        };
        let mut app0 = b"JFIF\0".to_vec();
        app0.extend_from_slice(&[1, 1, 0, 0, 1]);
        push(&mut d, &mut expect, 0xE0, &app0);
        push(&mut d, &mut expect, 0xDB, &[0, 1, 2]);
        push(&mut d, &mut expect, 0xC0, &[8, 0, 1, 0, 1, 0]);
        push(&mut d, &mut expect, 0xC4, &[0, 1, 2, 3]);
        let sos_offset = d.len();
        push(&mut d, &mut expect, 0xDA, &[1, 1, 0, 0, 0x3F, 0]);
        d.extend_from_slice(&[0x12, 0xFF, 0x00, 0x34, 0xFF, 0xD0, 0x56]);
        let eoi_at = d.len();
        d.extend_from_slice(&eoi());

        let j = Jpeg::parse(&d).unwrap();
        assert!(j.is_clean(), "{:?}", j.issues());
        assert_eq!(j.segments().len(), expect.len());
        for (s, (m, off, pl)) in j.segments().iter().zip(&expect) {
            assert_eq!((s.marker(), s.offset(), s.payload()), (*m, *off, &pl[..]));
            assert!(!s.after_scan());
        }
        assert_eq!(j.scan_region().unwrap(), &d[sos_offset..]);
        assert_eq!(j.eoi_offset(), Some(eoi_at));
        assert!(j.trailer().is_empty());
        assert!(Jpeg::parse_strict(&d).is_ok());
    }

    #[test]
    fn app_segments_are_classified_by_marker_and_signature() {
        let cases: Vec<(u8, Vec<u8>, SegmentKind)> = vec![
            (0xE0, b"JFIF\0xx".to_vec(), SegmentKind::Jfif),
            (0xE1, b"Exif\0\0II".to_vec(), SegmentKind::Exif),
            (
                0xE1,
                b"http://ns.adobe.com/xap/1.0/\0<x/>".to_vec(),
                SegmentKind::Xmp,
            ),
            (
                0xE1,
                b"http://ns.adobe.com/xmp/extension/\0guid".to_vec(),
                SegmentKind::ExtendedXmp,
            ),
            (0xE2, b"ICC_PROFILE\0\x01\x01".to_vec(), SegmentKind::Icc),
            (0xED, b"Photoshop 3.0\08BIM".to_vec(), SegmentKind::Iptc),
            (0xEB, b"JP\0\0box".to_vec(), SegmentKind::Jumbf),
            (0xE3, b"whatever".to_vec(), SegmentKind::App(3)),
            (0xE1, b"Nope\0\0".to_vec(), SegmentKind::App(1)),
            (0xE1, b"Exif".to_vec(), SegmentKind::App(1)),
            (0xC2, vec![0; 6], SegmentKind::Sof(0xC2)),
            (0xC4, vec![0; 2], SegmentKind::Dht),
            (0xDB, vec![0; 2], SegmentKind::Dqt),
            (0xDD, vec![0; 2], SegmentKind::Dri),
            (0xFE, b"c".to_vec(), SegmentKind::Com),
            (0xC8, vec![], SegmentKind::Other(0xC8)),
        ];
        let mut d = soi();
        for (m, pl, _) in &cases {
            d.extend_from_slice(&seg(*m, pl));
        }
        let j = Jpeg::parse(&d).unwrap();
        let kinds: Vec<SegmentKind> = j.segments().iter().map(Segment::kind).collect();
        let want: Vec<SegmentKind> = cases.iter().map(|c| c.2).collect();
        assert_eq!(kinds, want);
        // SOS is classified too.
        let mut d2 = soi();
        d2.extend_from_slice(&sos());
        let j2 = Jpeg::parse(&d2).unwrap();
        assert_eq!(j2.segments()[0].kind(), SegmentKind::Sos);
    }

    #[test]
    fn payloads_and_scan_borrow_from_the_input_not_the_jpeg() {
        let d = cat(&[
            &soi(),
            &seg(0xE1, b"Exif\0\0abc"),
            &sos(),
            &[1, 2, 3],
            &eoi(),
            &[9, 9],
        ]);
        let (payload, scan, trailer, off);
        {
            let j = Jpeg::parse(&d).unwrap();
            let s = j.segments()[0];
            payload = s.payload();
            off = s.offset();
            scan = j.scan_region().unwrap();
            trailer = j.trailer();
        } // the Jpeg is dropped; the slices live on
        assert_eq!(payload, b"Exif\0\0abc");
        assert_eq!(payload.as_ptr(), d[off + 4..].as_ptr());
        let range = d.as_ptr_range();
        for s in [payload, scan, trailer] {
            let r = s.as_ptr_range();
            assert!(r.start >= range.start && r.end <= range.end);
        }
        assert_eq!(trailer, &[9, 9]);
    }

    #[test]
    fn a_length_of_ffff_is_a_65533_byte_payload() {
        let payload = vec![0xAB; 65_533];
        let d = cat(&[&soi(), &[0xFF, 0xE1, 0xFF, 0xFF], &payload, &sos(), &eoi()]);
        let j = Jpeg::parse(&d).unwrap();
        assert!(j.is_clean(), "{:?}", j.issues());
        assert_eq!(j.segments()[0].payload().len(), 65_533);
    }

    #[test]
    fn a_length_of_two_is_a_valid_empty_segment() {
        let d = cat(&[&soi(), &seg(0xE1, b""), &sos(), &eoi()]);
        let j = Jpeg::parse(&d).unwrap();
        assert!(j.is_clean());
        assert!(j.segments()[0].payload().is_empty());
        assert_eq!(j.segments()[0].kind(), SegmentKind::App(1));
    }

    // ── hostile segment shapes ───────────────────────────────────────────────

    #[test]
    fn length_zero_and_one_are_bad_lengths_and_the_walk_continues() {
        for l in [0u8, 1] {
            let d = cat(&[&soi(), &[0xFF, 0xE1, 0x00, l], &com(), &sos(), &eoi()]);
            let j = Jpeg::parse(&d).unwrap();
            assert_eq!(
                errors(&j),
                [
                    Error::BadSegmentLength {
                        marker: 0xE1,
                        length: u16::from(l)
                    },
                    // Rescan after the two skipped marker bytes: the length
                    // bytes themselves are a stray run (rule 2.1).
                    Error::StrayBytes { at: 4, len: 2 },
                ],
                "length {l}"
            );
            assert_eq!(j.issues()[0].offset(), 2);
            let kinds: Vec<SegmentKind> = j.segments().iter().map(Segment::kind).collect();
            assert_eq!(kinds, [SegmentKind::Com, SegmentKind::Sos]);
        }
    }

    #[test]
    fn a_length_past_eof_omits_the_segment_and_ends_the_walk() {
        let d = cat(&[&soi(), &[0xFF, 0xE1, 0x01, 0x00], &[0; 6]]);
        assert_eq!(d.len(), 12);
        let j = Jpeg::parse(&d).unwrap();
        assert_eq!(
            errors(&j),
            [
                Error::Truncated { at: 2, len: 258 },
                Error::MissingSos,
                Error::MissingEoi
            ]
        );
        assert!(j.segments().is_empty());
        assert_eq!(
            Jpeg::parse_strict(&d).unwrap_err(),
            Error::Truncated { at: 2, len: 258 }
        );
    }

    #[test]
    fn a_marker_with_no_room_for_its_length_is_truncated() {
        let d = [0xFF, 0xD8, 0xFF, 0xE1, 0x00];
        let j = Jpeg::parse(&d).unwrap();
        assert_eq!(
            errors(&j),
            [
                Error::Truncated { at: 2, len: 4 },
                Error::MissingSos,
                Error::MissingEoi
            ]
        );
    }

    #[test]
    fn a_lone_trailing_ff_is_truncated() {
        let d = [0xFF, 0xD8, 0xFF];
        let j = Jpeg::parse(&d).unwrap();
        assert_eq!(
            errors(&j),
            [
                Error::Truncated { at: 2, len: 2 },
                Error::MissingSos,
                Error::MissingEoi
            ]
        );
    }

    #[test]
    fn fill_bytes_before_a_marker_are_clean() {
        let d = cat(&[&soi(), &[0xFF, 0xFF], &seg(0xE0, b"JFIF\0"), &sos(), &eoi()]);
        let j = Jpeg::parse(&d).unwrap();
        assert!(j.is_clean(), "{:?}", j.issues());
        // The FF immediately before E0 is at offset 4 (2 and 3 are fill).
        assert_eq!(j.segments()[0].offset(), 4);
        assert_eq!(j.segments()[0].kind(), SegmentKind::Jfif);
    }

    #[test]
    fn stray_bytes_between_segments_are_one_issue_per_run() {
        let mut d = soi();
        d.extend_from_slice(&com()); // 2..8
        d.extend_from_slice(&[1, 2, 3]); // stray at 8, len 3
        d.extend_from_slice(&com()); // 11..17
        d.extend_from_slice(&[7, 7, 7, 7, 7]); // stray at 17, len 5
        d.extend_from_slice(&sos());
        d.extend_from_slice(&eoi());
        let j = Jpeg::parse(&d).unwrap();
        assert_eq!(
            errors(&j),
            [
                Error::StrayBytes { at: 8, len: 3 },
                Error::StrayBytes { at: 17, len: 5 }
            ]
        );
        assert_eq!(j.issues()[0].offset(), 8);
        assert_eq!(j.segments().len(), 3);
    }

    #[test]
    fn ff00_and_a_mid_stream_soi_outside_entropy_data_are_stray() {
        // FF 00 / SOI / RST3 / TEM each at a known offset, then a COM.
        let d = cat(&[
            &soi(),
            &[0xFF, 0x00],
            &[0xFF, 0xD8],
            &[0xFF, 0xD3],
            &[0xFF, 0x01],
            &com(),
            &sos(),
            &eoi(),
        ]);
        let j = Jpeg::parse(&d).unwrap();
        assert_eq!(
            errors(&j),
            [
                Error::StrayBytes { at: 2, len: 2 },
                Error::StrayBytes { at: 4, len: 2 },
                Error::StrayBytes { at: 6, len: 2 },
                Error::StrayBytes { at: 8, len: 2 },
            ]
        );
        assert_eq!(j.segments().len(), 2);
    }

    #[test]
    fn a_trailer_after_eoi_is_returned_not_an_issue() {
        let d = cat(&[&soi(), &sos(), &[5], &eoi(), b"THUMB"]);
        let j = Jpeg::parse(&d).unwrap();
        assert!(j.is_clean());
        assert_eq!(j.trailer(), b"THUMB");
        // SOS segment (10) + one scan byte + EOI (2).
        assert_eq!(j.scan_region().unwrap().len(), 13);
    }

    #[test]
    fn a_second_eoi_or_data_after_eoi_is_never_walked() {
        let d = cat(&[&soi(), &sos(), &eoi(), &com(), &eoi()]);
        let j = Jpeg::parse(&d).unwrap();
        assert!(j.is_clean());
        assert_eq!(j.segments().len(), 1);
        assert_eq!(j.trailer().len(), com().len() + 2);
    }

    // ── scans ────────────────────────────────────────────────────────────────

    #[test]
    fn progressive_scans_walk_as_a_segment_sequence() {
        let first = soi().len();
        let d = cat(&[
            &soi(),
            &sos(),
            &[1, 2, 3],
            &seg(0xC4, &[0; 3]),
            &sos(),
            &[4, 5],
            &eoi(),
        ]);
        let j = Jpeg::parse(&d).unwrap();
        assert!(j.is_clean(), "{:?}", j.issues());
        let kinds: Vec<SegmentKind> = j.segments().iter().map(Segment::kind).collect();
        assert_eq!(
            kinds,
            [SegmentKind::Sos, SegmentKind::Dht, SegmentKind::Sos]
        );
        let after: Vec<bool> = j.segments().iter().map(Segment::after_scan).collect();
        assert_eq!(after, [false, true, true]);
        assert_eq!(j.scan_region().unwrap(), &d[first..]);
    }

    #[test]
    fn entropy_data_hides_no_markers_but_ends_at_a_real_one() {
        let d = cat(&[
            &soi(),
            &sos(),
            &[0x01, 0xFF, 0x00, 0xFF, 0xD3, 0xFF, 0xFF, 0xD3, 0x02],
            &seg(0xC4, &[0; 2]),
            &eoi(),
        ]);
        let j = Jpeg::parse(&d).unwrap();
        assert!(j.is_clean(), "{:?}", j.issues());
        assert_eq!(j.segments().len(), 2);
        assert_eq!(j.segments()[1].kind(), SegmentKind::Dht);
        assert!(j.segments()[1].after_scan());
    }

    #[test]
    fn an_app_segment_after_the_scan_is_listed_with_after_scan_set() {
        let d = cat(&[&soi(), &sos(), &[1], &seg(0xE1, b"Exif\0\0x"), &eoi()]);
        let j = Jpeg::parse(&d).unwrap();
        assert!(j.is_clean());
        let s = j.segments()[1];
        assert_eq!(s.kind(), SegmentKind::Exif);
        assert!(s.after_scan());
    }

    #[test]
    fn missing_eoi_after_a_scan_extends_the_region_to_the_end() {
        let d = cat(&[&soi(), &sos(), &[1, 2, 3, 4]]);
        let j = Jpeg::parse(&d).unwrap();
        assert_eq!(errors(&j), [Error::MissingEoi]);
        assert_eq!(j.scan_region().unwrap(), &d[2..]);
        assert_eq!(j.eoi_offset(), None);
        assert!(j.trailer().is_empty());
    }

    #[test]
    fn a_jpeg_with_headers_but_no_sos_reports_missing_sos() {
        let d = cat(&[&soi(), &com(), &eoi()]);
        let j = Jpeg::parse(&d).unwrap();
        assert_eq!(errors(&j), [Error::MissingSos]);
        assert!(j.scan_region().is_none());
        assert_eq!(j.eoi_offset(), Some(8));
    }

    // ── caps and pathological ────────────────────────────────────────────────

    fn many_coms(n: usize) -> Vec<u8> {
        let mut d = soi();
        for _ in 0..n {
            d.extend_from_slice(&seg(0xFE, b""));
        }
        d.extend_from_slice(&sos());
        d.extend_from_slice(&eoi());
        d
    }

    #[test]
    fn exactly_max_segments_is_clean() {
        let d = many_coms(MAX_JPEG_SEGMENTS - 1); // + the SOS = 4096
        let j = Jpeg::parse(&d).unwrap();
        assert!(j.is_clean(), "{:?}", j.issues().first());
        assert_eq!(j.segments().len(), MAX_JPEG_SEGMENTS);
    }

    #[test]
    fn one_over_the_segment_cap_halts_with_too_many_segments() {
        let d = many_coms(MAX_JPEG_SEGMENTS);
        let j = Jpeg::parse(&d).unwrap();
        assert_eq!(j.segments().len(), MAX_JPEG_SEGMENTS);
        assert_eq!(
            errors(&j),
            [Error::TooManySegments {
                limit: MAX_JPEG_SEGMENTS
            }]
        );
    }

    #[test]
    fn a_flood_of_ff00_pairs_is_capped_at_max_issues_plus_one() {
        let mut d = soi();
        for _ in 0..100_000 {
            d.extend_from_slice(&[0xFF, 0x00]);
        }
        let j = Jpeg::parse(&d).unwrap();
        assert_eq!(j.issues().len(), MAX_JPEG_ISSUES + 1);
        assert_eq!(
            j.issues().last().unwrap().error(),
            &Error::TooManyIssues { limit: 1024 }
        );
        assert!(!errors(&j).contains(&Error::MissingSos));
        assert!(!errors(&j).contains(&Error::MissingEoi));
    }

    #[test]
    fn the_issue_cap_boundary_is_exact() {
        // 1024 stray pairs fill the cap; the end-of-walk MissingSos would be
        // the 1025th ordinary issue, so it becomes TooManyIssues and
        // MissingEoi is suppressed.
        let mut d = soi();
        for _ in 0..MAX_JPEG_ISSUES {
            d.extend_from_slice(&[0xFF, 0x00]);
        }
        let j = Jpeg::parse(&d).unwrap();
        assert_eq!(j.issues().len(), MAX_JPEG_ISSUES + 1);
        assert_eq!(
            j.issues().last().unwrap().error(),
            &Error::TooManyIssues {
                limit: MAX_JPEG_ISSUES
            }
        );
        // Two fewer pairs leave room for MissingSos and MissingEoi: exactly
        // MAX_JPEG_ISSUES issues, no cap issue.
        let mut d = soi();
        for _ in 0..(MAX_JPEG_ISSUES - 2) {
            d.extend_from_slice(&[0xFF, 0x00]);
        }
        let j = Jpeg::parse(&d).unwrap();
        assert_eq!(j.issues().len(), MAX_JPEG_ISSUES);
        assert_eq!(j.issues().last().unwrap().error(), &Error::MissingEoi);
        // One pair more: MissingSos is the 1024th ordinary issue and recorded;
        // MissingEoi would be the 1025th, so it becomes TooManyIssues. (Found
        // by the jpeg_segments fuzz target; the cap can fire inside rule 3.)
        let mut d = soi();
        for _ in 0..(MAX_JPEG_ISSUES - 1) {
            d.extend_from_slice(&[0xFF, 0x00]);
        }
        let j = Jpeg::parse(&d).unwrap();
        let tail: Vec<Error> = errors(&j).split_off(MAX_JPEG_ISSUES - 1);
        assert_eq!(
            tail,
            [
                Error::MissingSos,
                Error::TooManyIssues {
                    limit: MAX_JPEG_ISSUES
                }
            ]
        );
    }

    #[test]
    fn nothing_is_recorded_after_a_halt() {
        // A segment-cap halt leaves exactly the halting issue, although a
        // missing SOS and EOI would otherwise follow.
        let mut d = soi();
        for _ in 0..=MAX_JPEG_SEGMENTS {
            d.extend_from_slice(&seg(0xFE, b""));
        }
        let j = Jpeg::parse(&d).unwrap();
        assert_eq!(
            errors(&j),
            [Error::TooManySegments {
                limit: MAX_JPEG_SEGMENTS
            }]
        );
        // The one record function replaces the 1025th issue and then refuses
        // everything after the halt.
        let mut w = Walk {
            data: &[],
            segments: Vec::new(),
            issues: Vec::new(),
            halted: false,
            scan_start: None,
            eoi: None,
        };
        for i in 0..(MAX_JPEG_ISSUES + 50) {
            w.record(Error::MissingSos, i);
        }
        assert_eq!(w.issues.len(), MAX_JPEG_ISSUES + 1);
        assert!(w.halted);
    }

    #[test]
    fn a_large_all_ff_scan_is_linear_and_terminates() {
        let mut d = cat(&[&soi(), &sos()]);
        d.resize(d.len() + 2 * 1024 * 1024, 0xFF);
        let j = Jpeg::parse(&d).unwrap();
        assert_eq!(errors(&j), [Error::MissingEoi]);
    }

    #[test]
    fn walk_always_makes_progress() {
        let inputs: Vec<Vec<u8>> = vec![
            vec![0xFF, 0xD8, 0xFF, 0x01],
            vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x00, 0xFF, 0xE0, 0x00, 0x00],
            vec![0xFF, 0xD8, 0xFF, 0xFF],
            vec![0xFF, 0xD8, 0xFF, 0xFF, 0xFF, 0xFF],
            vec![0xFF, 0xD8, 0xFF, 0xD8],
            vec![0xFF, 0xD8, 0xFF, 0xD0, 0xFF, 0xD7],
            vec![0xFF, 0xD8, 0xFF, 0x00, 0xFF, 0x00],
            vec![0xFF, 0xD8, 0xFF, 0xE1, 0x00, 0x01, 0xFF, 0xE1, 0x00, 0x01],
            vec![0xFF, 0xD8, 0x00],
            vec![0xFF, 0xD8, 0x00, 0x00, 0x00],
            vec![0xFF, 0xD8, 0xFF, 0xDA, 0x00, 0x02],
            vec![0xFF, 0xD8, 0xFF, 0xDA, 0x00, 0x02, 0xFF],
            vec![0xFF, 0xD8, 0xFF, 0xDA, 0x00, 0x02, 0xFF, 0xFF],
            vec![0xFF, 0xD8, 0xFF, 0xDA, 0x00, 0x02, 0xFF, 0x00],
            vec![0xFF, 0xD8, 0xFF, 0xDA, 0x00, 0x02, 0xFF, 0xDA, 0x00, 0x02],
            vec![0xFF, 0xD8, 0xFF, 0xDA, 0x00, 0x02, 0xFF, 0xD9],
            vec![0xFF, 0xD8, 0xFF, 0xD9],
            vec![0xFF, 0xD8, 0xFF, 0xD9, 0xFF, 0xD9],
            vec![0xFF, 0xD8, 0xFF, 0xE1, 0xFF, 0xFF],
            vec![0xFF, 0xD8, 0xFF, 0xE1, 0x00],
            vec![0xFF, 0xD8, 0xFF, 0xE1],
            vec![0xFF, 0xD8, 0xFF, 0xC8, 0x00, 0x02, 0xFF, 0xC8, 0x00, 0x02],
            vec![0xFF, 0xD8, 0x12, 0xFF],
            vec![0xFF, 0xD8, 0xFF, 0x00, 0x12, 0xFF, 0x00],
            vec![0xFF, 0xD8, 0xFF, 0x01, 0xFF, 0x01, 0xFF, 0x01],
            vec![0xFF, 0xD8, 0xFF, 0xFE, 0x00, 0x03, 0xFF],
            vec![0xFF, 0xD8, 0xFF, 0xFE, 0x00, 0x03, 0xFF, 0xD9],
            vec![0xFF, 0xD8, 0xFF, 0xFF, 0x00],
            vec![0xFF, 0xD8, 0xFF, 0xFF, 0xD9],
            vec![
                0xFF, 0xD8, 0xFF, 0xDA, 0x00, 0x03, 0xFF, 0xFF, 0xD3, 0xFF, 0x00, 0xFF,
            ],
        ];
        assert_eq!(inputs.len(), 30);
        for d in &inputs {
            // Returning at all is the progress proof (a stuck cursor would hang
            // or exhaust the issue cap); every issue consumes at least a byte,
            // apart from the two end-of-walk ones.
            let j = Jpeg::parse(d).unwrap();
            assert!(j.issues().len() <= d.len() + 2, "{d:02x?}");
        }
    }

    // ── follow-up round: pins found by the SPEC-009 verify ──────────────────

    #[test]
    fn stray_runs_do_not_merge_across_an_ff00_pair() {
        // 01 | FF 00 | 02: a run, the FF 00 pair, a run, as three issues.
        let d = cat(&[&soi(), &[1, 0xFF, 0x00, 2], &com(), &sos(), &eoi()]);
        let j = Jpeg::parse(&d).unwrap();
        assert_eq!(
            errors(&j),
            [
                Error::StrayBytes { at: 2, len: 1 },
                Error::StrayBytes { at: 3, len: 2 },
                Error::StrayBytes { at: 5, len: 1 },
            ]
        );
        assert_eq!(j.segments().len(), 2);
    }

    #[test]
    fn an_soi_inside_entropy_data_ends_the_scan_as_a_stray_marker() {
        // Rule 2.8: only FF 00, FF D0..D7 and FF FF are data inside a scan. FF D8
        // is a real marker there, so the scan ends and rule 2.6 reports it as
        // stray; the bytes after it are a separate stray run. (The verify note
        // suggested "FF D8 is data"; the rules and the code say otherwise, and
        // this pins the rules.)
        let d = cat(&[&soi(), &sos(), &[1], &[0xFF, 0xD8], &[2], &eoi()]);
        let j = Jpeg::parse(&d).unwrap();
        assert_eq!(
            errors(&j),
            [
                Error::StrayBytes { at: 13, len: 2 },
                Error::StrayBytes { at: 15, len: 1 }
            ]
        );
        assert_eq!(j.eoi_offset(), Some(16));
        // The scan region still runs from the SOS to the end of EOI.
        assert_eq!(j.scan_region().unwrap(), &d[2..]);
    }

    #[test]
    fn no_segment_is_recorded_after_an_issue_cap_halt() {
        // 1025 stray pairs: the 1025th issue is TooManyIssues and halts. The
        // COM after them must not become a segment.
        let mut d = soi();
        for _ in 0..=MAX_JPEG_ISSUES {
            d.extend_from_slice(&[0xFF, 0x00]);
        }
        d.extend_from_slice(&com());
        d.extend_from_slice(&sos());
        let j = Jpeg::parse(&d).unwrap();
        assert_eq!(j.issues().len(), MAX_JPEG_ISSUES + 1);
        assert!(j.segments().is_empty());
        // And a segment before the flood stays, with none added after it.
        let mut d2 = cat(&[&soi(), &com()]);
        for _ in 0..=MAX_JPEG_ISSUES {
            d2.extend_from_slice(&[0xFF, 0x00]);
        }
        d2.extend_from_slice(&com());
        let j2 = Jpeg::parse(&d2).unwrap();
        assert_eq!(j2.segments().len(), 1);
    }

    #[test]
    fn a_fill_run_reaching_the_end_is_truncated_at_the_last_ff() {
        // Rule 2.3: `p` is the marker's own FF, i.e. the last FF of the fill,
        // so the truncation is reported there (offset 3), not at the first FF.
        let d = [0xFF, 0xD8, 0xFF, 0xFF];
        let j = Jpeg::parse(&d).unwrap();
        assert_eq!(
            errors(&j),
            [
                Error::Truncated { at: 3, len: 2 },
                Error::MissingSos,
                Error::MissingEoi
            ]
        );
        let d = [0xFF, 0xD8, 0xFF, 0xFF, 0xFF, 0xFF];
        assert_eq!(
            errors(&Jpeg::parse(&d).unwrap())[0],
            Error::Truncated { at: 5, len: 2 }
        );
    }

    #[test]
    fn a_truncated_segment_is_decided_before_the_segment_cap() {
        // The code checks length-past-EOF first and the cap only when the
        // segment would actually be recorded. So a 4097th segment that is also
        // truncated reports Truncated (and the walk ends normally, with the
        // end-of-walk issues), not TooManySegments.
        let mut d = soi();
        for _ in 0..MAX_JPEG_SEGMENTS {
            d.extend_from_slice(&seg(0xFE, b""));
        }
        let cut_at = d.len();
        d.extend_from_slice(&[0xFF, 0xE1, 0x01, 0x00, 0]);
        let j = Jpeg::parse(&d).unwrap();
        assert_eq!(j.segments().len(), MAX_JPEG_SEGMENTS);
        assert_eq!(
            errors(&j),
            [
                Error::Truncated {
                    at: cut_at,
                    len: 258
                },
                Error::MissingSos,
                Error::MissingEoi
            ]
        );
    }

    #[test]
    fn sof_dht_and_other_markers_classify_exactly() {
        let want = [
            (0xC0, SegmentKind::Sof(0xC0)),
            (0xC2, SegmentKind::Sof(0xC2)),
            (0xCF, SegmentKind::Sof(0xCF)),
            (0xC4, SegmentKind::Dht),
            (0xC8, SegmentKind::Other(0xC8)),
            (0xCC, SegmentKind::Other(0xCC)),
        ];
        for (m, kind) in want {
            let d = cat(&[&soi(), &seg(m, &[0; 2]), &sos(), &eoi()]);
            let j = Jpeg::parse(&d).unwrap();
            assert_eq!(j.segments()[0].kind(), kind, "marker {m:#04x}");
        }
    }

    // ── API and errors ───────────────────────────────────────────────────────

    #[test]
    fn parse_strict_agrees_with_parse() {
        let fatal: [&[u8]; 3] = [&[], &[0xFF], &[0x00, 0x00]];
        for d in fatal {
            assert_eq!(
                Jpeg::parse_strict(d).unwrap_err(),
                Jpeg::parse(d).unwrap_err()
            );
        }
        let clean = cat(&[&soi(), &sos(), &eoi()]);
        assert!(Jpeg::parse_strict(&clean).is_ok());
        let dirty = cat(&[&soi(), &[1], &sos(), &eoi()]);
        assert_eq!(
            Jpeg::parse_strict(&dirty).unwrap_err(),
            Error::StrayBytes { at: 2, len: 1 }
        );
    }

    #[test]
    fn every_error_variant_this_module_returns_displays_something() {
        let errors = [
            Error::Truncated { at: 1, len: 2 },
            Error::NotTiff { mark: *b"XX" },
            Error::UnsupportedTiffVersion { version: 43 },
            Error::CyclicIfd { offset: 8 },
            Error::IfdDepthExceeded { limit: 8 },
            Error::TooManyIfds { limit: 512 },
            Error::UnexpectedFieldType {
                tag: 1,
                field_type: 250,
            },
            Error::ValueOverflow { tag: 1 },
            Error::TagTooLarge {
                tag: 1,
                count: 2,
                limit: 3,
            },
            Error::MissingTag { tag: 256 },
            Error::OffsetOutOfRange { offset: 9 },
            Error::BadPointerCount {
                tag: 0x8769,
                count: 2,
            },
            Error::NotJpeg { mark: [1, 2] },
            Error::StrayBytes { at: 3, len: 4 },
            Error::BadSegmentLength {
                marker: 0xE1,
                length: 1,
            },
            Error::MissingSos,
            Error::MissingEoi,
            Error::TooManySegments { limit: 4096 },
            Error::TooManyIssues { limit: 1024 },
        ];
        for e in &errors {
            assert!(!e.to_string().is_empty(), "{e:?} has an empty Display");
        }
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
            }
        }
        let indices: Vec<usize> = errors.iter().map(index).collect();
        assert_eq!(indices, (0..19).collect::<Vec<usize>>());

        let kinds = [
            SegmentKind::Jfif,
            SegmentKind::Exif,
            SegmentKind::Xmp,
            SegmentKind::ExtendedXmp,
            SegmentKind::Icc,
            SegmentKind::Iptc,
            SegmentKind::Jumbf,
            SegmentKind::App(3),
            SegmentKind::Sof(0xC2),
            SegmentKind::Dht,
            SegmentKind::Dqt,
            SegmentKind::Dri,
            SegmentKind::Com,
            SegmentKind::Sos,
            SegmentKind::Other(0xC8),
        ];
        for k in kinds {
            assert!(!k.to_string().is_empty() && !format!("{k:?}").is_empty());
        }
        let issue = JpegIssue::new(Error::MissingSos, 7);
        assert!(!issue.to_string().is_empty() && !format!("{issue:?}").is_empty());
    }
}
