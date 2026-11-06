//! Container sniffing: magic bytes only, no parsing (SPEC-012 rule 1).

/// What the first bytes of a file say it is. Nothing past the first twelve
/// bytes is ever read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Sniffed {
    /// `FF D8`.
    Jpeg,
    /// Classic TIFF: `II*\0` or `MM\0*`.
    Tiff,
    /// BigTIFF: `II+\0` or `MM\0+`. Recognised, not read.
    BigTiff,
    /// The eight-byte PNG signature.
    Png,
    /// `RIFF`, four size bytes, `WEBP`.
    Webp,
    /// Bytes 4..8 are `ftyp` (any ISO base media brand: HEIC, HEIF, AVIF, MP4 ...).
    /// The audit does not read it, so it does not tell the brands apart.
    Heic,
    /// `GIF87a` or `GIF89a`.
    Gif,
    /// Anything else.
    Unknown,
    /// Zero bytes.
    Empty,
}

const PNG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// Classify `data` by its magic bytes alone.
///
/// A prefix shorter than a magic never matches that magic: every check is a
/// `starts_with` / exact `get` on the slice, so a short input falls through to
/// [`Sniffed::Unknown`] (or [`Sniffed::Empty`] for zero bytes).
pub fn sniff(data: &[u8]) -> Sniffed {
    // Never look past the first twelve bytes (the longest magic is RIFF....WEBP).
    let head = data.get(..12).unwrap_or(data);
    if head.is_empty() {
        return Sniffed::Empty;
    }
    if head.starts_with(&[0xFF, 0xD8]) {
        return Sniffed::Jpeg;
    }
    if head.starts_with(&[b'I', b'I', 0x2A, 0x00]) || head.starts_with(&[b'M', b'M', 0x00, 0x2A]) {
        return Sniffed::Tiff;
    }
    if head.starts_with(&[b'I', b'I', 0x2B, 0x00]) || head.starts_with(&[b'M', b'M', 0x00, 0x2B]) {
        return Sniffed::BigTiff;
    }
    if head.starts_with(&PNG) {
        return Sniffed::Png;
    }
    if head.starts_with(b"RIFF") && head.get(8..12) == Some(&b"WEBP"[..]) {
        return Sniffed::Webp;
    }
    if head.get(4..8) == Some(&b"ftyp"[..]) {
        return Sniffed::Heic;
    }
    if head.starts_with(b"GIF87a") || head.starts_with(b"GIF89a") {
        return Sniffed::Gif;
    }
    Sniffed::Unknown
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

    /// Every magic, written out again here as data: (name, full magic, expected).
    fn magics() -> Vec<(&'static str, Vec<u8>, Sniffed)> {
        vec![
            ("jpeg", vec![0xFF, 0xD8], Sniffed::Jpeg),
            ("tiff-le", b"II\x2A\x00".to_vec(), Sniffed::Tiff),
            ("tiff-be", b"MM\x00\x2A".to_vec(), Sniffed::Tiff),
            ("bigtiff-le", b"II\x2B\x00".to_vec(), Sniffed::BigTiff),
            ("bigtiff-be", b"MM\x00\x2B".to_vec(), Sniffed::BigTiff),
            (
                "png",
                vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
                Sniffed::Png,
            ),
            ("webp", b"RIFF\x01\x02\x03\x04WEBP".to_vec(), Sniffed::Webp),
            ("heic", b"\x00\x00\x00\x18ftyp".to_vec(), Sniffed::Heic),
            ("gif87", b"GIF87a".to_vec(), Sniffed::Gif),
            ("gif89", b"GIF89a".to_vec(), Sniffed::Gif),
        ]
    }

    #[test]
    fn sniff_recognises_every_magic_and_never_overreads() {
        for (name, magic, want) in magics() {
            assert_eq!(sniff(&magic), want, "{name}: the full magic");
            // Trailing junk after the magic changes nothing.
            let mut long = magic.clone();
            long.extend_from_slice(&[0xAA; 64]);
            assert_eq!(sniff(&long), want, "{name}: magic then junk");
            // Every proper prefix is Unknown (never the magic's kind, and no
            // magic here is a prefix of another), and zero bytes is Empty.
            for cut in 0..magic.len() {
                let got = sniff(&magic[..cut]);
                if cut == 0 {
                    assert_eq!(got, Sniffed::Empty, "{name}: empty prefix");
                } else {
                    assert_eq!(got, Sniffed::Unknown, "{name}: prefix of {cut} bytes");
                }
            }
        }
        assert_eq!(sniff(&[]), Sniffed::Empty);
        assert_eq!(sniff(&[0x00]), Sniffed::Unknown, "one byte");
        assert_eq!(sniff(&[0xFF]), Sniffed::Unknown, "one byte FF");
    }

    #[test]
    fn sniff_distinguishes_tiff_from_bigtiff_and_never_calls_riff_without_webp_webp() {
        assert_eq!(sniff(b"II\x2A\x00"), Sniffed::Tiff);
        assert_eq!(sniff(b"II\x2B\x00"), Sniffed::BigTiff);
        assert_eq!(sniff(b"MM\x00\x2A"), Sniffed::Tiff);
        assert_eq!(sniff(b"MM\x00\x2B"), Sniffed::BigTiff);
        // Byte-order mark and version must agree on endianness.
        assert_eq!(sniff(b"II\x00\x2A"), Sniffed::Unknown);
        assert_eq!(sniff(b"MM\x2A\x00"), Sniffed::Unknown);
        // RIFF without WEBP (a WAV, AVI) is not WebP; so is a truncated one.
        assert_eq!(sniff(b"RIFF\x01\x02\x03\x04WAVE"), Sniffed::Unknown);
        assert_eq!(sniff(b"RIFF\x01\x02\x03\x04AVI "), Sniffed::Unknown);
        assert_eq!(sniff(b"RIFF\x01\x02\x03\x04WEB"), Sniffed::Unknown);
        assert_eq!(sniff(b"RIFF\x01\x02\x03\x04WEBP"), Sniffed::Webp);
        // `ftyp` only counts at bytes 4..8.
        assert_eq!(sniff(b"ftyp\x00\x00\x00\x00"), Sniffed::Unknown);
        assert_eq!(sniff(b"\x00\x00\x00\x18ftyp"), Sniffed::Heic);
        assert_eq!(sniff(b"\x00\x00\x00\x18fty"), Sniffed::Unknown);
        // A JFIF-looking file is JPEG whatever follows.
        assert_eq!(sniff(&[0xFF, 0xD8, 0xFF, 0xE0]), Sniffed::Jpeg);
        assert_eq!(sniff(&[0xFF, 0xD9]), Sniffed::Unknown);
        assert_eq!(sniff(b"GIF88a"), Sniffed::Unknown);
    }
}
