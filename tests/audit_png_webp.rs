//! SPEC-032 part A / DEC-020: the audit of a PNG and a WebP, the preview that
//! never claims a removal, and the writer that still refuses both. Fixtures
//! come from the independent builders in `tests/support`; expectations are
//! typed here, never derived from the library.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

#[path = "support/png_shapes.rs"]
mod png_shapes;
#[path = "support/webp_shapes.rs"]
mod webp_shapes;

use exif_core::audit::{
    audit, preview, Category, ContainerKind, FileAudit, ItemKind, RegionKind, Sniffed,
};
use exif_core::{strip, StripError, StripPolicy};

/// A big-endian TIFF whose IFD0 points at a GPS IFD holding one entry
/// (`GPSLatitudeRef`, "N").
fn tiff_with_gps() -> Vec<u8> {
    let mut d = b"MM\x00\x2A\x00\x00\x00\x08".to_vec();
    // IFD0: one entry, GPSInfo pointer (0x8825, LONG, 1) to offset 26.
    d.extend_from_slice(&[0, 1, 0x88, 0x25, 0, 4, 0, 0, 0, 1, 0, 0, 0, 26]);
    d.extend_from_slice(&[0, 0, 0, 0]);
    // GPS IFD at 26: GPSLatitudeRef (0x0001, ASCII, 2) = "N\0".
    d.extend_from_slice(&[0, 1, 0, 1, 0, 2, 0, 0, 0, 2, b'N', 0, 0, 0]);
    d.extend_from_slice(&[0, 0, 0, 0]);
    d
}

fn gps_findings(a: &FileAudit<'_>) -> Vec<Option<usize>> {
    a.findings_in(Category::Gps)
        .map(|f| f.at.origin().block)
        .collect()
}

fn region_items(a: &FileAudit<'_>) -> Vec<(RegionKind, usize, usize)> {
    a.items()
        .iter()
        .filter_map(|i| match i.kind() {
            ItemKind::Region { kind, segment, len } => Some((kind, segment, len)),
            _ => None,
        })
        .collect()
}

fn categories(a: &FileAudit<'_>) -> Vec<Category> {
    a.findings().iter().map(|f| f.category).collect()
}

// ── PNG ─────────────────────────────────────────────────────────────────────

use png_shapes as p;

fn png_with(parts: &[Vec<u8>]) -> Vec<u8> {
    let mut all = vec![p::ihdr()];
    all.extend_from_slice(parts);
    all.push(p::idat());
    all.push(p::iend());
    p::png(&all)
}

#[test]
fn kind_dispatch_and_clean_minimal_files() {
    let min_png = p::minimal();
    let a = audit(&min_png);
    assert_eq!(a.kind(), ContainerKind::Png);
    assert!(a.health().clean && a.findings().is_empty());
    let w = webp_shapes::minimal();
    let a = audit(&w);
    assert_eq!(a.kind(), ContainerKind::Webp);
    assert!(a.health().clean && a.findings().is_empty());
    // Every other sniffed kind is unchanged.
    for (bytes, want) in [
        (b"GIF89a\x01\x00\x01\x00".to_vec(), Sniffed::Gif),
        (b"\x00\x00\x00\x18ftypheic".to_vec(), Sniffed::Heic),
    ] {
        assert_eq!(audit(&bytes).kind(), ContainerKind::Unsupported(want));
    }
}

#[test]
fn png_two_exif_blocks_are_both_inventoried_with_indexes() {
    let first = p::exif_chunk(&p::exif_with_make());
    let second = p::exif_chunk(&tiff_with_gps());
    let bytes = png_with(&[first, second]);
    let a = audit(&bytes);
    assert_eq!(a.kind(), ContainerKind::Png);
    // Block 0 holds Make, block 1 holds the GPS entry (and its pointer).
    let blocks: Vec<Option<usize>> = a.items().iter().map(|i| i.origin().block).collect();
    assert!(blocks.contains(&Some(0)) && blocks.contains(&Some(1)));
    assert_eq!(gps_findings(&a), vec![Some(1)]);
    // The chunk index is recorded: IHDR is chunk 0, the two eXIf are 1 and 2.
    let segs: Vec<Option<usize>> = a
        .findings_in(Category::Gps)
        .map(|f| f.at.origin().segment)
        .collect();
    assert_eq!(segs, vec![Some(2)]);
    // The second eXIf costs the library's ExtraExif issue: not clean.
    assert!(!a.health().clean && a.health().jpeg_issues >= 1);
}

#[test]
fn png_regions_match_the_category_matrix() {
    let parts = vec![
        p::text(b"Author", b"someone"),
        p::text(b"Title", b"a title"),
        p::text(b"XML:com.adobe.xmp", b"<x:xmpmeta/>"),
        p::iccp(b"sRGB", 0, b"zz"),
        p::raw_profile("exif", b"4578"),
        p::chunk(b"tIME", &[0x07, 0xE4, 1, 2, 3, 4, 5]),
    ];
    let bytes = png_with(&parts);
    let a = audit(&bytes);
    assert_eq!(
        categories(&a),
        vec![
            Category::Authorship,
            Category::FreeText,
            Category::Xmp,
            Category::Icc,
            Category::OtherSegment,
            Category::Timestamps,
        ]
    );
    let kinds: Vec<RegionKind> = region_items(&a).iter().map(|r| r.0).collect();
    assert_eq!(
        kinds,
        vec![
            RegionKind::Text,
            RegionKind::Text,
            RegionKind::Xmp,
            RegionKind::Icc,
            RegionKind::RawProfile,
            RegionKind::Time,
        ]
    );
    // Chunk indexes: IHDR 0, then the six chunks 1..=6.
    let segs: Vec<usize> = region_items(&a).iter().map(|r| r.1).collect();
    assert_eq!(segs, vec![1, 2, 3, 4, 5, 6]);
    assert!(a.health().clean, "all regions located, no issue");
    // Each finding points at its item.
    for f in a.findings() {
        assert_eq!(f.item.map(|i| a.items()[i]), Some(f.at));
    }
}

#[test]
fn png_text_keyword_rule_applies_through_the_audit() {
    let parts = vec![
        p::text(b"author", b"x"),
        p::text(b"AUTHOR", b"x"),
        p::text(b"Copyright", b"x"),
        p::text(b"creation Time", b"x"),
        p::text(b"creation time", b"x"),
        p::text(b"Software", b"x"),
        p::ztxt(b"Software", b"zz"),
        p::itxt(b"Warning", 0, b"", b"", b"x"),
    ];
    let bytes = png_with(&parts);
    let a = audit(&bytes);
    assert_eq!(
        categories(&a),
        vec![
            Category::Authorship,
            Category::FreeText,
            Category::Authorship,
            Category::Timestamps,
            Category::FreeText,
            Category::Software,
            Category::Software,
            Category::FreeText,
        ]
    );
}

#[test]
fn png_empty_region_has_an_item_and_no_finding() {
    let bytes = png_with(&[p::text(b"Title", b""), p::iccp(b"x", 0, b"")]);
    let a = audit(&bytes);
    assert!(region_items(&a)
        .iter()
        .any(|r| r.0 == RegionKind::Text && r.2 == 0));
    assert!(a
        .findings()
        .iter()
        .all(|f| f.category != Category::FreeText));
    assert_eq!(a.items().len(), region_items(&a).len());
}

#[test]
fn png_trailer_and_after_idat_origins() {
    // Before IDAT, after IDAT, and bytes after IEND.
    let mut bytes = p::png(&[
        p::ihdr(),
        p::text(b"Title", b"before"),
        p::idat(),
        p::text(b"Author", b"after"),
        p::iend(),
    ]);
    bytes.extend_from_slice(b"trailing bytes");
    let a = audit(&bytes);
    let scans: Vec<(RegionKind, bool)> = a
        .items()
        .iter()
        .filter_map(|i| match i.kind() {
            ItemKind::Region { kind, .. } => Some((kind, i.origin().after_scan)),
            _ => None,
        })
        .collect();
    assert_eq!(
        scans,
        vec![
            (RegionKind::Text, false),
            (RegionKind::Text, true),
            (RegionKind::Trailer, true),
        ]
    );
    assert_eq!(
        categories(&a),
        vec![
            Category::FreeText,
            Category::Authorship,
            Category::OtherSegment
        ]
    );
}

#[test]
fn png_with_gps_exif_has_a_gps_finding() {
    let bytes = png_with(&[p::exif_chunk(&tiff_with_gps())]);
    let a = audit(&bytes);
    assert_eq!(gps_findings(&a), vec![Some(0)]);
    assert!(a.health().clean);
}

// ── WebP ────────────────────────────────────────────────────────────────────

use webp_shapes as w;

#[test]
fn webp_two_exif_blocks_are_both_inventoried() {
    let bytes = w::riff(&[
        w::vp8x(w::FLAG_EXIF, 1, 1),
        w::vp8l(),
        w::exif_chunk(&w::exif_with_make()),
        w::exif_chunk(&tiff_with_gps()),
    ]);
    let a = audit(&bytes);
    assert_eq!(a.kind(), ContainerKind::Webp);
    let blocks: Vec<Option<usize>> = a.items().iter().map(|i| i.origin().block).collect();
    assert!(blocks.contains(&Some(0)) && blocks.contains(&Some(1)));
    assert_eq!(gps_findings(&a), vec![Some(1)]);
    // Chunks: VP8X 0, VP8L 1, EXIF 2 and 3; both lie after the image data.
    let segs: Vec<Option<usize>> = a
        .findings_in(Category::Gps)
        .map(|f| f.at.origin().segment)
        .collect();
    assert_eq!(segs, vec![Some(3)]);
    assert!(a.items().iter().any(|i| i.origin().after_scan));
}

#[test]
fn webp_seventeen_exif_blocks_with_gps_in_the_last_is_not_clean() {
    // The library parses 16 blocks; the 17th (GPS only there) is not listed,
    // so the audit must not read as clean or reliable (DEC-020 item 2).
    let mut parts = vec![w::vp8x(w::FLAG_EXIF, 1, 1), w::vp8l()];
    for _ in 0..16 {
        parts.push(w::exif_chunk(&w::tiff_empty()));
    }
    parts.push(w::exif_chunk(&tiff_with_gps()));
    let bytes = w::riff(&parts);
    let a = audit(&bytes);
    assert_eq!(a.kind(), ContainerKind::Webp);
    assert!(gps_findings(&a).is_empty(), "the 17th block is not listed");
    assert!(!a.health().clean, "17 blocks: not clean");
    for policy in StripPolicy::PRESETS {
        let pv = preview(&a, policy);
        assert!(!pv.reliable() && !pv.strippable());
    }
}

#[test]
fn webp_regions_and_empty_regions() {
    let bytes = w::riff(&[
        w::vp8x(w::FLAG_ICC | w::FLAG_XMP, 1, 1),
        w::chunk(b"ICCP", b"profile!"),
        w::vp8l(),
        w::chunk(b"XMP ", b"<x:xmpmeta/>"),
    ]);
    let a = audit(&bytes);
    assert_eq!(categories(&a), vec![Category::Icc, Category::Xmp]);
    // The ICCP precedes the image data, the XMP follows it.
    let scans: Vec<bool> = a.items().iter().map(|i| i.origin().after_scan).collect();
    assert_eq!(scans, vec![false, true]);
    let empty = w::riff(&[
        w::vp8x(w::FLAG_XMP, 1, 1),
        w::vp8l(),
        w::chunk(b"XMP ", b""),
    ]);
    let a = audit(&empty);
    assert_eq!(region_items(&a).len(), 1);
    assert!(a.findings().is_empty(), "an empty XMP is not XMP present");
}

// ── preview and writer ──────────────────────────────────────────────────────

fn rich_png() -> Vec<u8> {
    png_with(&[
        p::exif_chunk(&tiff_with_gps()),
        p::text(b"Author", b"x"),
        p::text(b"XML:com.adobe.xmp", b"<x/>"),
    ])
}

fn rich_webp() -> Vec<u8> {
    w::riff(&[
        w::vp8x(w::FLAG_EXIF | w::FLAG_XMP, 1, 1),
        w::vp8l(),
        w::exif_chunk(&tiff_with_gps()),
        w::chunk(b"XMP ", b"<x/>"),
    ])
}

#[test]
fn png_and_webp_previews_keep_everything_for_every_policy() {
    for bytes in [rich_png(), rich_webp()] {
        let a = audit(&bytes);
        assert!(!a.findings().is_empty(), "premise");
        for policy in StripPolicy::PRESETS {
            let pv = preview(&a, policy);
            assert_eq!(pv.counts().removed, 0, "{}", policy.name());
            assert_eq!(pv.counts().kept, a.findings().len());
            assert_eq!(pv.counts().kept_unread, 0);
            assert!(pv.dispositions().iter().all(|d| d.name() == "kept"));
            assert!(!pv.reliable() && !pv.strippable());
            assert_eq!(pv.residual_findings(), a.findings().len());
            assert_eq!(pv.residual_severity(), a.max_severity());
        }
    }
}

#[test]
fn jpeg_and_tiff_previews_are_strippable_and_unchanged() {
    // A TIFF with GPS: still removed by the Gps policy, still reliable.
    let mut tiff = tiff_with_gps();
    // Little changes needed: the same bytes are a valid bare TIFF.
    tiff.shrink_to_fit();
    let a = audit(&tiff);
    assert_eq!(a.kind(), ContainerKind::Tiff);
    let pv = preview(&a, StripPolicy::Gps);
    assert!(pv.strippable() && pv.reliable());
    assert!(pv.counts().removed >= 1);
    let jpeg = [0xFF, 0xD8, 0xFF, 0xD9];
    assert!(preview(&audit(&jpeg), StripPolicy::Gps).strippable());
}

#[test]
fn strip_refuses_png_and_webp_for_every_policy() {
    for policy in StripPolicy::PRESETS {
        match strip(&rich_png(), policy) {
            Err(StripError::Unsupported { kind }) => assert_eq!(kind, ContainerKind::Png),
            other => panic!("png under {}: {other:?}", policy.name()),
        }
        match strip(&rich_webp(), policy) {
            Err(StripError::Unsupported { kind }) => assert_eq!(kind, ContainerKind::Webp),
            other => panic!("webp under {}: {other:?}", policy.name()),
        }
        // Even a minimal file and a truncated one: no bytes.
        assert!(strip(&p::minimal(), policy).is_err());
        assert!(strip(&webp_shapes::minimal(), policy).is_err());
    }
}

// ── hostile input ───────────────────────────────────────────────────────────

#[test]
fn hostile_prefixes_never_panic_and_are_never_clean() {
    for full in [rich_png(), rich_webp()] {
        for cut in 0..full.len() {
            let a = audit(&full[..cut]);
            // A strict prefix of a file with an IEND or full RIFF size is
            // missing something: never "clean" for a recognised container.
            if matches!(a.kind(), ContainerKind::Png | ContainerKind::Webp) {
                assert!(!a.health().clean, "prefix {cut} read as clean");
            }
            for policy in StripPolicy::PRESETS {
                let _ = preview(&a, policy);
            }
        }
    }
}

#[test]
fn garbage_after_the_signature_does_not_panic() {
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut webp = b"RIFF\xFF\xFF\xFF\xFFWEBP".to_vec();
    for i in 0u8..=255 {
        png.push(i);
        webp.push(i.wrapping_mul(7));
    }
    for bytes in [png, webp] {
        let a = audit(&bytes);
        assert!(matches!(a.kind(), ContainerKind::Png | ContainerKind::Webp));
        assert!(!a.health().clean);
    }
    // The bare signature and bare header.
    for bytes in [
        &[0x89u8, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A][..],
        b"RIFF\x04\x00\x00\x00WEBPJUNK",
    ] {
        assert!(!audit(bytes).health().clean);
    }
}
