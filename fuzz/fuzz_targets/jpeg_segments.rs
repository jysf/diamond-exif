//! Fuzz target for the JPEG segment walker (SPEC-009, DEC-006).
//!
//! The blocking constraint `panic-free-library` stated executably: **for every
//! byte string, `Jpeg::parse` and `Jpeg::parse_strict` return, and neither
//! panics.** Any answer, including every `Err`, is acceptable. An abort is not.
//!
//! The target also asserts the walker's invariants, so a violated one is a
//! finding. Every rule that is an iff is asserted in BOTH directions, so a
//! constant-true or constant-false implementation fails one of them:
//!
//! * `parse_strict` is `Ok` iff `is_clean()`; `Err` equals the first issue's
//!   error; a fatal `parse` gives an equal `parse_strict` error.
//! * Segments lie inside the input in strictly increasing, non-overlapping
//!   order, with the framing bytes `FF marker length` in front of the payload,
//!   and the payload borrowed from the input (zero-copy).
//! * Each signature kind holds iff its marker and signature match (written out
//!   here, independent of the library's own table). Exif is five bytes
//!   (`Exif\0`) in an APP1 payload of at least six (DEC-007), asserted both ways.
//! * `scan_region()` is `Some` iff some segment is `Sos`; it starts at the
//!   first `SOS` marker and ends at `eoi + 2`, or at the end of the input.
//! * `trailer()` is empty without an `EOI`, else everything after it.
//! * `after_scan()` holds iff the segment is after the first `SOS` segment.
//! * `MissingSos` is present iff there is no `Sos` segment, and `MissingEoi`
//!   iff there is no `EOI`, unless a cap halted the walk (then neither).
//! * Caps: at most `MAX_JPEG_SEGMENTS` segments and `MAX_JPEG_ISSUES + 1`
//!   issues; a cap issue is the last one.
//!
//! From SPEC-010 the target also calls `jpeg::read` on every input and asserts
//! the invariants in `fuzz/shared/jpeg_metadata_invariants.rs` (included, and
//! shared with `tests/fuzz_layout.rs`): the extraction is re-derived there
//! independently and compared both ways.
//!
//! * **Container independence:** each Exif block's container is `Ok` iff the
//!   bare `Container::parse` of its `tiff()` is, with identical IFD structure,
//!   and a fatal error equals the bare error.
//! * **Cleanliness:** `is_clean()` iff no JPEG-layer issue and every block `Ok`
//!   and clean; `issues().count()` is the JPEG count plus the blocks' counts.
//! * **Blocks:** at most `MAX_EXIF_BLOCKS`; `ExtraExif` iff an Exif segment is
//!   not the first (and nothing after a halt); `exif()` is the first `Ok` block.
//! * **Regions:** the located regions equal the independently derived list; an
//!   extended-XMP region exists iff its payload is at least 75 bytes;
//!   `IccChunkMismatch` iff the chunks disagree; nothing is recorded after a
//!   halt; `jpeg_issues().len() <= MAX_JPEG_METADATA_ISSUES`.
//!
//! This target CANNOT be run with the default PATH. `cargo fuzz` shells out to
//! a bare `cargo build`, which resolves to Homebrew's stable cargo and rejects
//! `-Zsanitizer=address`; the outer `+nightly` does not help. Put the rustup
//! shim first on PATH (or run `just fuzz-smoke jpeg_segments`):
//!
//! ```text
//! PATH="$HOME/.cargo/bin:$PATH" ~/.cargo/bin/cargo +nightly fuzz run jpeg_segments \
//!     fuzz/corpus/jpeg_segments fuzz/seeds/jpeg_segments -- -max_total_time=60
//! ```
//!
//! Seeds are written by `cargo run --example fuzz_seeds`.

#![no_main]

use libfuzzer_sys::fuzz_target;

use exif_core::jpeg::{Jpeg, SegmentKind, MAX_JPEG_ISSUES, MAX_JPEG_SEGMENTS};
use exif_core::Error;

/// Whether `inner` lies wholly inside `outer` (empty slices included).
fn lies_inside(outer: &[u8], inner: &[u8]) -> bool {
    let (o, i) = (outer.as_ptr_range(), inner.as_ptr_range());
    i.start >= o.start && i.end <= o.end
}

/// The kind a segment must have, written out independently of the library.
fn expected_kind(marker: u8, payload: &[u8]) -> SegmentKind {
    match marker {
        0xE0 if payload.starts_with(b"JFIF\0") => SegmentKind::Jfif,
        // DEC-007: five signature bytes and at least six bytes of payload.
        0xE1 if payload.len() >= 6 && payload.starts_with(b"Exif\0") => SegmentKind::Exif,
        0xE1 if payload.starts_with(b"http://ns.adobe.com/xap/1.0/\0") => SegmentKind::Xmp,
        0xE1 if payload.starts_with(b"http://ns.adobe.com/xmp/extension/\0") => {
            SegmentKind::ExtendedXmp
        }
        0xE2 if payload.starts_with(b"ICC_PROFILE\0") => SegmentKind::Icc,
        0xED if payload.starts_with(b"Photoshop 3.0\0") => SegmentKind::Iptc,
        0xEB if payload.starts_with(b"JP") => SegmentKind::Jumbf,
        0xE0..=0xEF => SegmentKind::App(marker - 0xE0),
        0xC4 => SegmentKind::Dht,
        0xC8 | 0xCC => SegmentKind::Other(marker),
        0xC0..=0xCF => SegmentKind::Sof(marker),
        0xDB => SegmentKind::Dqt,
        0xDD => SegmentKind::Dri,
        0xFE => SegmentKind::Com,
        0xDA => SegmentKind::Sos,
        _ => SegmentKind::Other(marker),
    }
}

fn check(data: &[u8]) {
    let strict = Jpeg::parse_strict(data);
    let jpeg = match Jpeg::parse(data) {
        Ok(jpeg) => jpeg,
        Err(error) => {
            // A rejected input is a SUCCESS: both entry points say the same.
            assert_eq!(
                strict.err(),
                Some(error),
                "parse_strict disagrees on a fatal input"
            );
            return;
        }
    };

    // Differential, both directions.
    assert_eq!(
        strict.is_ok(),
        jpeg.is_clean(),
        "parse_strict Ok iff is_clean"
    );
    assert_eq!(jpeg.is_clean(), jpeg.issues().is_empty());
    if let Err(error) = strict {
        let first = jpeg
            .issues()
            .first()
            .expect("parse_strict Err on a jpeg with no issues");
        assert_eq!(&error, first.error(), "strict error is not the first issue");
    }

    let len = data.len();

    // Segments: framing, containment, order, zero-copy.
    let mut prev_end = 2usize;
    for seg in jpeg.segments() {
        let off = seg.offset();
        let payload = seg.payload();
        assert!(off >= prev_end, "segment overlaps the previous one");
        assert_eq!(data.get(off), Some(&0xFF), "segment does not start at FF");
        assert_eq!(data.get(off + 1), Some(&seg.marker()), "marker mismatch");
        let declared = data
            .get(off + 2..off + 4)
            .map(|b| usize::from(u16::from_be_bytes([b[0], b[1]])));
        assert_eq!(declared, Some(payload.len() + 2), "length field mismatch");
        let end = off + 4 + payload.len();
        assert!(end <= len, "segment escapes the input");
        assert_eq!(
            payload.as_ptr(),
            data[off + 4..].as_ptr(),
            "payload is not at offset + 4"
        );
        assert!(lies_inside(data, payload), "payload escapes the input");
        prev_end = end;

        // Kind: each signature kind holds iff marker and signature match.
        let want = expected_kind(seg.marker(), payload);
        assert_eq!(seg.kind(), want, "kind");
        let k = seg.kind();
        let m = seg.marker();
        assert_eq!(
            k == SegmentKind::Exif,
            m == 0xE1 && payload.len() >= 6 && payload.starts_with(b"Exif\0"),
            "Exif iff APP1, at least 6 bytes and the first five are Exif\\0 (DEC-007)"
        );
        assert_eq!(
            k == SegmentKind::Xmp,
            m == 0xE1 && payload.starts_with(b"http://ns.adobe.com/xap/1.0/\0")
        );
        assert_eq!(
            k == SegmentKind::ExtendedXmp,
            m == 0xE1 && payload.starts_with(b"http://ns.adobe.com/xmp/extension/\0")
        );
        assert_eq!(
            k == SegmentKind::Icc,
            m == 0xE2 && payload.starts_with(b"ICC_PROFILE\0")
        );
        assert_eq!(
            k == SegmentKind::Iptc,
            m == 0xED && payload.starts_with(b"Photoshop 3.0\0")
        );
        assert_eq!(
            k == SegmentKind::Jumbf,
            m == 0xEB && payload.starts_with(b"JP")
        );
        assert_eq!(
            k == SegmentKind::Jfif,
            m == 0xE0 && payload.starts_with(b"JFIF\0")
        );
        assert_eq!(k == SegmentKind::Sos, m == 0xDA);
        let _ = (format!("{k}"), format!("{k:?}"));
    }
    assert!(jpeg.segments().len() <= MAX_JPEG_SEGMENTS);

    // Scan region, EOI and trailer.
    let first_sos = jpeg
        .segments()
        .iter()
        .find(|s| s.kind() == SegmentKind::Sos);
    assert_eq!(jpeg.scan_region().is_some(), first_sos.is_some());
    if let (Some(region), Some(sos)) = (jpeg.scan_region(), first_sos) {
        assert_eq!(region.get(..2), Some(&[0xFF, 0xDA][..]), "region start");
        assert_eq!(
            region.as_ptr(),
            data[sos.offset()..].as_ptr(),
            "region is not at the first SOS"
        );
        let end = jpeg.eoi_offset().map_or(len, |e| e + 2);
        assert_eq!(sos.offset() + region.len(), end, "region end");
    }
    match jpeg.eoi_offset() {
        None => assert!(jpeg.trailer().is_empty(), "trailer without EOI"),
        Some(eoi) => {
            assert_eq!(data.get(eoi..eoi + 2), Some(&[0xFF, 0xD9][..]), "EOI bytes");
            assert_eq!(jpeg.trailer(), &data[eoi + 2..], "trailer");
            assert!(
                jpeg.segments().iter().all(|s| s.offset() < eoi),
                "a segment after EOI"
            );
        }
    }
    assert!(lies_inside(data, jpeg.trailer()));

    // after_scan iff the segment is after the first SOS segment.
    for seg in jpeg.segments() {
        let expect = first_sos.is_some_and(|sos| seg.offset() > sos.offset());
        assert_eq!(seg.after_scan(), expect, "after_scan iff after first SOS");
    }

    // Issues: bounds, halts, and the end-of-walk pair.
    let errors: Vec<&Error> = jpeg.issues().iter().map(|i| i.error()).collect();
    assert!(errors.len() <= MAX_JPEG_ISSUES + 1);
    for issue in jpeg.issues() {
        assert!(issue.offset() <= len, "issue offset past the input");
        assert!(!issue.to_string().is_empty());
        let _ = format!("{issue:?}");
    }
    let halt = errors.iter().position(|e| {
        matches!(
            e,
            Error::TooManySegments { .. } | Error::TooManyIssues { .. }
        )
    });
    if let Some(idx) = halt {
        assert_eq!(idx + 1, errors.len(), "issue recorded after a halt");
    }
    assert_eq!(
        errors
            .iter()
            .any(|e| matches!(e, Error::TooManyIssues { .. })),
        errors.len() == MAX_JPEG_ISSUES + 1,
        "TooManyIssues iff the list is one over the cap"
    );
    // The segment cap halts only a full list.
    if errors
        .iter()
        .any(|e| matches!(e, Error::TooManySegments { .. }))
    {
        assert_eq!(
            jpeg.segments().len(),
            MAX_JPEG_SEGMENTS,
            "cap on a short list"
        );
    }
    let has_sos = first_sos.is_some();
    let has_eoi = jpeg.eoi_offset().is_some();
    let missing_sos = errors.iter().any(|e| matches!(e, Error::MissingSos));
    let missing_eoi = errors.iter().any(|e| matches!(e, Error::MissingEoi));
    if let Some(idx) = halt {
        // A halt inside the walk suppresses the end-of-walk pair. The one
        // exception is the issue cap firing INSIDE the end-of-walk step
        // (1023 ordinary issues, then MissingSos is the 1024th and MissingEoi
        // becomes TooManyIssues): the pair is then a contiguous run directly
        // before the halting issue, and each member is still true.
        let tail = errors.get(..idx).unwrap_or(&[]);
        let pair = tail
            .iter()
            .filter(|e| matches!(e, Error::MissingSos | Error::MissingEoi))
            .count();
        if matches!(errors.get(idx), Some(Error::TooManySegments { .. })) {
            assert_eq!(pair, 0, "end-of-walk issue after the segment cap");
        } else {
            assert!(
                tail.iter()
                    .rev()
                    .take(pair)
                    .all(|e| matches!(e, Error::MissingSos | Error::MissingEoi)),
                "end-of-walk issues are not directly before the cap"
            );
            assert!(!missing_sos || !has_sos, "MissingSos but there is an SOS");
            assert!(!missing_eoi || !has_eoi, "MissingEoi but there is an EOI");
        }
    } else {
        assert_eq!(missing_sos, !has_sos, "MissingSos iff no SOS segment");
        assert_eq!(missing_eoi, !has_eoi, "MissingEoi iff no EOI");
    }
}

// `metadata_invariants::check`, shared with `tests/fuzz_layout.rs`.
include!("../shared/jpeg_metadata_invariants.rs");

fuzz_target!(|data: &[u8]| {
    check(data);
    metadata_invariants::check(data);
});
