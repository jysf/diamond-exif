//! SPEC-038 / DEC-022 clause 4: the RIFF chunk types ExifTool 13.55 decodes in
//! a WebP and the audit now locates (`tests/support/webp_carriers.rs` is the
//! measured table, typed from the measurement, never from the library).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

#[path = "support/webp_carriers.rs"]
mod webp_carriers;
#[path = "support/webp_shapes.rs"]
mod webp_shapes;

use exif_core::audit::{audit, Category, ContainerKind, ItemKind, RegionKind};
use exif_core::webp::{chunk_class, read, ChunkClass, WebpRegion};
use webp_carriers::*;
use webp_shapes::*;

fn kind_of(name: &str) -> RegionKind {
    match name {
        "carrier" => RegionKind::Carrier,
        "jumbf" => RegionKind::Jumbf,
        "other_chunk" => RegionKind::OtherChunk,
        other => panic!("unknown region name {other}"),
    }
}

fn category_of(name: &str) -> Category {
    match name {
        "other_segment" => Category::OtherSegment,
        "jumbf" => Category::Jumbf,
        other => panic!("unknown category name {other}"),
    }
}

/// A WebP with `extra` before the image data.
fn with(extra: &[Vec<u8>]) -> Vec<u8> {
    let mut parts = vec![vp8x(0, 1, 1)];
    parts.extend(extra.iter().cloned());
    parts.push(vp8l());
    riff(&parts)
}

fn region_kinds(bytes: &[u8]) -> Vec<RegionKind> {
    audit(bytes)
        .items()
        .iter()
        .filter_map(|i| match i.kind() {
            ItemKind::Region { kind, .. } => Some(kind),
            _ => None,
        })
        .collect()
}

#[test]
fn every_measured_row_is_located_with_its_kind_and_category() {
    for row in ROWS {
        let bytes = with(&[row_chunk(&row.code, row.payload)]);
        let m = read(&bytes).unwrap();
        let located: Vec<_> = m
            .regions()
            .iter()
            .filter(|r| matches!(r, WebpRegion::Chunk { .. }))
            .collect();
        assert_eq!(located.len(), 1, "{:?}", String::from_utf8_lossy(&row.code));
        let a = audit(&bytes);
        assert_eq!(a.kind(), ContainerKind::Webp);
        assert_eq!(
            region_kinds(&bytes),
            vec![kind_of(row.region)],
            "{:?}",
            String::from_utf8_lossy(&row.code)
        );
        assert_eq!(a.findings().len(), 1);
        assert_eq!(
            a.findings()[0].category,
            category_of(row.category),
            "{:?}",
            String::from_utf8_lossy(&row.code)
        );
        assert!(!row.prints.is_empty());
    }
}

#[test]
fn a_chunk_code_is_case_sensitive_except_the_two_id3_codes_in_the_table() {
    for row in ROWS {
        for sp in spellings(row.code) {
            let bytes = with(&[row_chunk(&sp, row.payload)]);
            let known = ROWS.iter().any(|r| r.code == sp);
            let located = read(&bytes)
                .unwrap()
                .regions()
                .iter()
                .any(|r| matches!(r, WebpRegion::Chunk { .. }));
            // `JUNK` text, `ID3 `/`id3 ` and every row spelt exactly: located;
            // every other spelling: not.
            assert_eq!(
                located,
                known,
                "spelling {:?} of {:?}",
                String::from_utf8_lossy(&sp),
                String::from_utf8_lossy(&row.code)
            );
            if !known {
                assert!(region_kinds(&bytes).is_empty());
            }
        }
    }
}

#[test]
fn types_that_print_only_unknown_dumps_are_not_located() {
    for code in NOT_METADATA {
        let bytes = with(&[chunk(code, &payload(Payload::Text))]);
        assert!(
            read(&bytes).unwrap().regions().is_empty(),
            "{:?}",
            String::from_utf8_lossy(code)
        );
        assert!(audit(&bytes).findings().is_empty());
    }
}

#[test]
fn every_list_type_is_a_carrier() {
    for ty in LIST_TYPES {
        let mut data = ty.to_vec();
        data.extend_from_slice(&chunk(b"IART", b"Jane\0"));
        let bytes = with(&[chunk(b"LIST", &data)]);
        assert_eq!(region_kinds(&bytes), vec![RegionKind::Carrier]);
    }
    // Even a list type nothing was measured to decode (the safe direction).
    let bytes = with(&[chunk(b"LIST", b"movi")]);
    assert_eq!(region_kinds(&bytes), vec![RegionKind::Carrier]);
    assert_eq!(
        chunk_class(*b"LIST", b""),
        Some(ChunkClass::List),
        "an empty LIST is a carrier too"
    );
}

#[test]
fn junk_is_a_carrier_exactly_when_exiftool_prints_text_junk() {
    for data in JUNK_TEXT {
        let bytes = with(&[chunk(b"JUNK", data)]);
        assert_eq!(region_kinds(&bytes), vec![RegionKind::Carrier], "{data:?}");
    }
    for data in JUNK_NOT_TEXT {
        let bytes = with(&[chunk(b"JUNK", data)]);
        assert!(region_kinds(&bytes).is_empty(), "{data:?}");
    }
}

#[test]
fn a_carrier_is_found_wherever_it_lies() {
    let c = row_chunk(b"IDIT", Payload::Text);
    let image = vp8l();
    let head = vp8x(0, 1, 1);
    let declared_short = (4 + head.len() + image.len()) as u32;
    let null = vec![0u8; 8];
    for (name, bytes) in [
        ("before", riff(&[head.clone(), c.clone(), image.clone()])),
        ("after", riff(&[head.clone(), image.clone(), c.clone()])),
        (
            "beyond the declared size",
            riff_with_size(declared_short, &[head.clone(), image.clone(), c.clone()]),
        ),
        // ExifTool stops at an empty null chunk; this library reads on.
        (
            "after a null chunk",
            riff(&[head.clone(), image.clone(), null, c.clone()]),
        ),
        ("without a VP8X", riff(&[image.clone(), c.clone()])),
    ] {
        assert_eq!(region_kinds(&bytes), vec![RegionKind::Carrier], "{name}");
        assert_eq!(audit(&bytes).findings().len(), 1, "{name}");
    }
    // The carrier before the image data is not "after the scan"; after is.
    let before = riff(&[head.clone(), c.clone(), image.clone()]);
    let a = audit(&before);
    assert!(!a.items()[0].origin().after_scan);
    let after = riff(&[head, image, c]);
    let a = audit(&after);
    assert!(a.items()[0].origin().after_scan);
}

#[test]
fn an_empty_carrier_is_located_but_has_no_finding() {
    let bytes = with(&[chunk(b"IDIT", b"")]);
    assert_eq!(region_kinds(&bytes), vec![RegionKind::Carrier]);
    assert!(audit(&bytes).findings().is_empty());
}

#[test]
fn chunks_inside_an_anmf_frame_are_never_walked() {
    let mut frame = vec![0u8; 16];
    frame.extend_from_slice(&row_chunk(b"IDIT", Payload::Text));
    let bytes = riff(&[
        vp8x(0x02, 1, 1),
        chunk(b"ANIM", &[0; 6]),
        chunk(b"ANMF", &frame),
    ]);
    assert!(region_kinds(&bytes).is_empty());
}

#[test]
fn an_xml_carrier_is_an_unreadable_block_and_a_text_one_is_not() {
    for (code, payload_kind, failed) in [
        (*b"aXML", Payload::Xmp, 1),
        (*b"JUNQ", Payload::Zeros, 1),
        (*b"IDIT", Payload::Text, 0),
        (*b"LIST", Payload::ListInfo, 0),
        (*b"C2PA", Payload::Jumbf, 0),
    ] {
        let bytes = with(&[row_chunk(&code, payload_kind)]);
        let a = audit(&bytes);
        assert_eq!(a.health().failed_blocks, failed, "{code:?}");
        assert_eq!(a.health().clean, failed == 0, "{code:?}");
    }
}
