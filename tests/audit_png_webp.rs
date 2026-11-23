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
    // All regions located, no walk issue: the only reason this file is not
    // clean is the raw profile (DEC-020 clause 5), counted as a failed block.
    assert_eq!(a.health().jpeg_issues + a.health().exif_issues, 0);
    assert_eq!(a.health().failed_blocks, 1);
    assert!(!a.health().clean, "a raw profile is never clean");
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
        p::text(b"Artist", b"x"),
        p::ztxt(b"artist", b"zz"),
        p::text(b"create-date", b"x"),
        p::itxt(b"modify-date", 0, b"", b"", b"x"),
        p::text(b"Create-date", b"x"),
        p::text(b"ARTIST", b"x"),
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
            // Artist, artist: Authorship; create-date, modify-date: Timestamps;
            // Create-date, ARTIST: their own tags in ExifTool, free text here.
            Category::Authorship,
            Category::Authorship,
            Category::Timestamps,
            Category::Timestamps,
            Category::FreeText,
            Category::FreeText,
            Category::Timestamps,
            Category::FreeText,
            Category::Software,
            Category::Software,
            Category::FreeText,
        ]
    );
}

#[test]
fn png_empty_regions_have_an_item_and_their_finding() {
    // ExifTool 13.55 prints a tag for each of these (measured, SPEC-032 fix
    // round): a text chunk with an empty value, PNG:ProfileName and
    // PNG:ICC_Profile (0 bytes), PNG:XMP (0 bytes), PNG:Author empty. So the
    // audit keeps the finding (DEC-020 clause 3); only a WebP region is dropped.
    let bytes = png_with(&[
        p::text(b"Title", b""),
        p::iccp(b"x", 0, b""),
        p::itxt(b"XML:com.adobe.xmp", 0, b"", b"", b""),
        p::text(b"Author", b""),
        p::ztxt(b"Author", &[]),
    ]);
    let a = audit(&bytes);
    let items = region_items(&a);
    assert_eq!(
        items.iter().map(|r| (r.0, r.2)).collect::<Vec<_>>(),
        vec![
            (RegionKind::Text, 0),
            (RegionKind::Icc, 0),
            (RegionKind::Xmp, 0),
            (RegionKind::Text, 0),
            (RegionKind::Text, 0),
        ]
    );
    assert_eq!(
        categories(&a),
        vec![
            Category::FreeText,
            Category::Icc,
            Category::Xmp,
            Category::Authorship,
            Category::Authorship,
        ]
    );
    assert_eq!(a.items().len(), items.len());
    assert!(a.health().clean);
}

/// ExifTool 13.55 prints `PNG:ModifyDate` for a `tIME` chunk of every length 0
/// to 12 (measured, SPEC-032 fix round: 0 bytes gives `0000:00:00 00:00:00`,
/// 6 bytes `2020:01:02 03:04:00`, 8 to 12 bytes the same as 7), so the audit has
/// a `Time` item and a `Timestamps` finding for each length class.
#[test]
fn png_time_chunk_of_any_length_is_a_timestamp() {
    let full = [0x07, 0xE4, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
    for len in 0..=12usize {
        let bytes = png_with(&[p::chunk(b"tIME", &full[..len])]);
        let a = audit(&bytes);
        assert_eq!(
            region_items(&a),
            vec![(RegionKind::Time, 1, len)],
            "tIME of {len} bytes"
        );
        assert_eq!(categories(&a), vec![Category::Timestamps], "len {len}");
        assert!(a.health().clean, "len {len}");
    }
}

#[test]
fn png_raw_profile_is_an_unreadable_block_never_clean() {
    use exif_core::report::{status_of, FileResult, Status};
    // The profile body is not decoded: hex for an Exif with GPS would be here.
    let body: &[u8] = b"\nexif\n        26\n4578696600004d4d002a00000008\n";
    let cases: Vec<(&str, Vec<u8>)> = vec![
        (
            "tEXt alone",
            png_with(&[p::text(b"Raw profile type exif", body)]),
        ),
        (
            "zTXt alone",
            png_with(&[p::ztxt(b"Raw profile type exif", body)]),
        ),
        (
            "tEXt beside a GPS-free eXIf",
            png_with(&[
                p::exif_chunk(&p::exif_with_make()),
                p::text(b"Raw profile type exif", body),
            ]),
        ),
        (
            "zTXt beside a GPS-free eXIf",
            png_with(&[
                p::exif_chunk(&p::exif_with_make()),
                p::ztxt(b"Raw profile type exif", body),
            ]),
        ),
        (
            "iTXt alone",
            png_with(&[p::itxt(b"Raw profile type xmp", 1, b"", b"", body)]),
        ),
    ];
    for (name, bytes) in cases {
        let a = audit(&bytes);
        assert_eq!(a.kind(), ContainerKind::Png, "{name}");
        assert!(!a.health().clean, "{name}: not clean");
        assert_eq!(a.health().failed_blocks, 1, "{name}");
        assert_eq!(
            a.health().jpeg_issues + a.health().exif_issues,
            0,
            "{name}: the raw profile is the only reason"
        );
        // Besides its OtherSegment finding, not instead of it.
        assert!(
            a.findings_in(Category::OtherSegment).next().is_some(),
            "{name}"
        );
        let r = FileResult::Audited(a);
        assert_eq!(status_of(&r), Status::Unreadable, "{name}: status");
    }
    // An EMPTY raw profile holds nothing: located, a finding, not unreadable.
    let empty = png_with(&[p::text(b"Raw profile type exif", b"")]);
    let a = audit(&empty);
    assert!(a.health().clean && a.health().failed_blocks == 0);
    // Two raw profiles are two failed blocks.
    let two = png_with(&[
        p::text(b"Raw profile type exif", body),
        p::ztxt(b"Raw profile type xmp", body),
    ]);
    assert_eq!(audit(&two).health().failed_blocks, 2);
}

/// SPEC-036 verify H1: ExifTool 13.55 decodes the all-lower-case keyword
/// `raw profile type exif` like the usual spelling, so it is a raw profile for
/// the audit too (never clean). Any other case mix is plain text for both.
#[test]
fn a_lower_case_raw_profile_keyword_is_unreadable_and_other_cases_are_text() {
    use exif_core::report::{status_of, FileResult, Status};
    let body: &[u8] = b"\nexif\n        26\n4578696600004d4d002a00000008\n";
    let raw = [
        png_with(&[p::text(b"raw profile type exif", body)]),
        png_with(&[p::ztxt(b"raw profile type exif", body)]),
        png_with(&[p::itxt(b"raw profile type exif", 1, b"", b"", body)]),
        png_with(&[p::text(b"raw profile type APP1", body)]),
    ];
    for (i, bytes) in raw.iter().enumerate() {
        let a = audit(bytes);
        assert!(!a.health().clean, "case {i}");
        assert_eq!(a.health().failed_blocks, 1, "case {i}");
        let r = FileResult::Audited(a);
        assert_eq!(status_of(&r), Status::Unreadable, "case {i}");
    }
    for key in [
        &b"RAW PROFILE TYPE EXIF"[..],
        b"Raw Profile Type exif",
        b"raw Profile type exif",
    ] {
        let bytes = png_with(&[p::text(key, body)]);
        let a = audit(&bytes);
        assert!(a.health().clean, "{key:?}");
        assert_eq!(a.health().failed_blocks, 0, "{key:?}");
    }
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

/// A WebP with `n` EXIF blocks, each holding a Make entry, and GPS in the last.
fn webp_blocks_gps_last(n: usize) -> Vec<u8> {
    let mut parts = vec![w::vp8x(w::FLAG_EXIF, 1, 1), w::vp8l()];
    for _ in 1..n {
        parts.push(w::exif_chunk(&w::exif_with_make()));
    }
    parts.push(w::exif_chunk(&tiff_with_gps()));
    w::riff(&parts)
}

/// The cap is 16 blocks (`MAX_WEBP_EXIF_BLOCKS`). Every EXIF chunk after the
/// first already costs one `ExtraExif` walk issue, so "not clean" cannot tell
/// the cap from a second block; the pair does: 16 blocks list the GPS of the
/// 16th, 17 do not list the GPS of the 17th, the listed blocks are 16 either
/// way, and the issue count is blocks - 1 (15, then 16: the 17th chunk is
/// counted though it gets no block).
#[test]
fn webp_sixteen_blocks_list_the_last_gps_seventeen_do_not() {
    let (b16, b17) = (webp_blocks_gps_last(16), webp_blocks_gps_last(17));
    let sixteen = audit(&b16);
    let seventeen = audit(&b17);
    let listed = |a: &FileAudit<'_>| -> Vec<usize> {
        let mut v: Vec<usize> = a.items().iter().filter_map(|i| i.origin().block).collect();
        v.dedup();
        v
    };
    assert_eq!(listed(&sixteen), (0..16).collect::<Vec<_>>());
    assert_eq!(listed(&seventeen), (0..16).collect::<Vec<_>>());
    assert_eq!(gps_findings(&sixteen), vec![Some(15)], "the 16th is listed");
    assert!(
        gps_findings(&seventeen).is_empty(),
        "the 17th block is not listed"
    );
    assert_eq!(sixteen.health().jpeg_issues, 15, "15 extra EXIF chunks");
    assert_eq!(
        seventeen.health().jpeg_issues,
        16,
        "the 17th chunk is counted by the walk though it gets no block"
    );
    for a in [&sixteen, &seventeen] {
        assert_eq!(a.kind(), ContainerKind::Webp);
        assert!(!a.health().clean);
        assert_eq!(
            a.health().failed_blocks,
            0,
            "no block failed: it is `issues`"
        );
        for policy in StripPolicy::PRESETS {
            let pv = preview(a, policy);
            assert!(!pv.reliable() && !pv.strippable());
        }
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
fn strip_refuses_webp_for_every_policy_and_png_strips_while_not_strippable() {
    for policy in StripPolicy::PRESETS {
        // SPEC-037: a PNG strips now (DEC-021); the audit still says it is not
        // strippable until SPEC-039 shows ExifTool agrees (DEC-020 clause 6).
        let out = strip(&rich_png(), policy)
            .unwrap_or_else(|e| panic!("png under {}: {e:?}", policy.name()));
        assert!(out.bytes().len() <= rich_png().len());
        assert!(!preview(&audit(&rich_png()), policy).strippable());
        match strip(&rich_webp(), policy) {
            Err(StripError::Unsupported { kind }) => assert_eq!(kind, ContainerKind::Webp),
            other => panic!("webp under {}: {other:?}", policy.name()),
        }
        // Even a minimal WebP: no bytes.
        assert!(strip(&webp_shapes::minimal(), policy).is_err());
        assert!(strip(&p::minimal(), policy).is_ok());
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

/// `ContainerKind::strippable` is true for JPEG and TIFF only (DEC-020 clause
/// 6); the preview reads it, so GIF, HEIC, BigTIFF, empty and unknown files
/// say `strippable` false.
#[test]
fn only_jpeg_and_tiff_are_strippable() {
    assert!(ContainerKind::Jpeg.strippable());
    assert!(ContainerKind::Tiff.strippable());
    assert!(!ContainerKind::Png.strippable());
    assert!(!ContainerKind::Webp.strippable());
    let inputs: Vec<(&str, Vec<u8>, bool)> = vec![
        ("tiff", tiff_with_gps(), true),
        ("jpeg", b"\xFF\xD8\xFF\xD9".to_vec(), true),
        ("png", p::minimal(), false),
        ("webp", w::minimal(), false),
        (
            "gif",
            b"GIF89a\x01\x00\x01\x00\x00\x00\x00;".to_vec(),
            false,
        ),
        (
            "heic",
            b"\x00\x00\x00\x18ftypheic\x00\x00\x00\x00".to_vec(),
            false,
        ),
        ("bigtiff", b"II+\x00\x08\x00\x00\x00".to_vec(), false),
        ("empty", Vec::new(), false),
        ("unknown", b"hello, world".to_vec(), false),
    ];
    for (name, bytes, want) in inputs {
        let a = audit(&bytes);
        assert_eq!(a.kind().strippable(), want, "{name}");
        for policy in StripPolicy::PRESETS {
            assert_eq!(preview(&a, policy).strippable(), want, "{name}");
        }
    }
}
