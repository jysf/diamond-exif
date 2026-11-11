//! The Exif-entry fixtures of SPEC-021 as JPEG files, and the policy grid they
//! run under. Shared by `tests/strip.rs` (byte-exact expectations from the
//! layout record) and `tests/strip_oracle.rs` / `examples/oracle_record.rs`
//! (ExifTool). Tier A: nothing here imports `src/` beyond the public policy
//! types.

#![allow(dead_code)]

use std::ops::Range;

use crate::exif_layout::{self, Cat, Layout};
use crate::real_shapes;
use crate::strip_shapes;
use exif_core::audit::Category;
use exif_core::policy::{StripPolicy, StripSpec};

pub fn scan_bytes() -> Vec<u8> {
    let mut v = real_shapes::seg(0xDA, &[1, 1, 0, 0, 0x3F, 0]);
    v.extend_from_slice(&[0x12, 0x34, 0x56, 0x78, 0x9A]);
    v.extend_from_slice(&real_shapes::EOI);
    v
}

pub fn jfif_bytes() -> Vec<u8> {
    real_shapes::seg(0xE0, b"JFIF\0\x01\x01\0\0\x01\0\x01\0\0")
}

/// A JPEG with a COM (optionally) and one Exif segment per blob.
pub struct Jpg {
    pub bytes: Vec<u8>,
    /// Input offset of each blob (segment offset + 10).
    pub blobs: Vec<usize>,
    pub layouts: Vec<Layout>,
    pub com: Option<Range<usize>>,
    /// Input offset of the first Exif segment.
    pub first_exif: usize,
}

pub fn jpeg_of(layouts: &[Layout], com: bool) -> Jpg {
    let mut v = vec![0xFF, 0xD8];
    v.extend_from_slice(&jfif_bytes());
    let mut com_range = None;
    if com {
        let c = real_shapes::seg(0xFE, strip_shapes::C_COM1);
        com_range = Some(v.len()..v.len() + c.len());
        v.extend_from_slice(&c);
    }
    let first_exif = v.len();
    let mut blobs = Vec::new();
    for l in layouts {
        blobs.push(v.len() + 10);
        v.extend_from_slice(&real_shapes::exif_app1(&l.bytes));
    }
    v.extend_from_slice(&scan_bytes());
    Jpg {
        bytes: v,
        blobs,
        layouts: layouts.to_vec(),
        com: com_range,
        first_exif,
    }
}

pub fn jpeg_with(blobs: &[Vec<u8>]) -> Vec<u8> {
    let mut v = vec![0xFF, 0xD8];
    v.extend_from_slice(&jfif_bytes());
    for b in blobs {
        v.extend_from_slice(&real_shapes::exif_app1(b));
    }
    v.extend_from_slice(&scan_bytes());
    v
}

pub fn category_of(name: Cat) -> Category {
    match name {
        "gps" => Category::Gps,
        "device_id" => Category::DeviceId,
        "authorship" => Category::Authorship,
        "free_text" => Category::FreeText,
        "timestamps" => Category::Timestamps,
        "camera_model" => Category::CameraModel,
        "thumbnail" => Category::Thumbnail,
        "maker_note" => Category::MakerNote,
        "software" => Category::Software,
        "xmp" => Category::Xmp,
        "iptc" => Category::Iptc,
        "icc" => Category::Icc,
        other => panic!("unknown category {other}"),
    }
}

pub const ALL_PII: &[Cat] = &[
    "gps",
    "device_id",
    "authorship",
    "free_text",
    "timestamps",
    "camera_model",
    "thumbnail",
    "maker_note",
];

/// The partial policies the grid runs, by category names (the presets are
/// used for the three presets, a custom set for the rest).
pub fn exif_policies() -> Vec<(&'static str, &'static [Cat], StripPolicy)> {
    let custom = |cats: &'static [Cat]| {
        StripPolicy::Custom(
            cats.iter()
                .fold(StripSpec::none(), |s, c| s.include(category_of(c))),
        )
    };
    vec![
        ("gps", &["gps"][..], StripPolicy::Gps),
        (
            "gps_and_device",
            &["gps", "device_id"][..],
            StripPolicy::GpsAndDevice,
        ),
        ("all_pii", ALL_PII, StripPolicy::AllPii),
        ("xmp", &["xmp"][..], custom(&["xmp"])),
        ("software", &["software"][..], custom(&["software"])),
        (
            "timestamps+gps",
            &["timestamps", "gps"][..],
            custom(&["timestamps", "gps"]),
        ),
        ("maker_note", &["maker_note"][..], custom(&["maker_note"])),
        ("thumbnail", &["thumbnail"][..], custom(&["thumbnail"])),
        (
            "camera_model",
            &["camera_model"][..],
            custom(&["camera_model"]),
        ),
        ("authorship", &["authorship"][..], custom(&["authorship"])),
        (
            "free_text+device",
            &["free_text", "device_id"][..],
            custom(&["free_text", "device_id"]),
        ),
    ]
}

/// The fixtures, each as a list of Exif blobs.
pub fn exif_fixtures() -> Vec<(&'static str, Vec<Layout>)> {
    let two = exif_layout::two_blocks();
    vec![
        ("full-le", vec![exif_layout::full_le().layout()]),
        ("full-be", vec![exif_layout::full_be().layout()]),
        (
            "first-entry-target",
            vec![exif_layout::first_entry_target().layout()],
        ),
        (
            "last-entry-target",
            vec![exif_layout::last_entry_target().layout()],
        ),
        (
            "every-entry-target",
            vec![exif_layout::every_entry_target().layout()],
        ),
        (
            "no-gps-no-thumb",
            vec![exif_layout::no_gps_no_thumb().layout()],
        ),
        ("subifd", vec![exif_layout::subifd().layout()]),
        ("chain-ifd2", vec![exif_layout::chain_ifd2().layout()]),
        (
            "xmp-tag-inside",
            vec![exif_layout::xmp_tag_inside().layout()],
        ),
        (
            "makernote-canon-style",
            vec![exif_layout::makernote_canon_style().layout()],
        ),
        (
            "at-the-ceiling",
            vec![exif_layout::at_the_ceiling().layout()],
        ),
        ("two-blocks", vec![two.0.layout(), two.1.layout()]),
    ]
}

/// Exif after the first scan: the segment sits behind the entropy data.
pub fn exif_after_the_first_scan() -> Vec<u8> {
    let l = exif_layout::full_le().layout();
    let mut v = vec![0xFF, 0xD8];
    v.extend_from_slice(&jfif_bytes());
    v.extend_from_slice(&real_shapes::seg(0xDA, &[1, 1, 0, 0, 0x3F, 0]));
    v.extend_from_slice(&[0x12, 0x34]);
    v.extend_from_slice(&real_shapes::exif_app1(&l.bytes));
    v.extend_from_slice(&real_shapes::EOI);
    v
}

/// A clean first Exif block, then a second whose GPS IFD declares 0xFFFF
/// entries (a walk issue, no GPS finding) with the coordinates behind it.
pub fn second_block_unclean() -> Vec<u8> {
    let first = exif_layout::first_entry_target().layout();
    jpeg_with(&[first.bytes, unclean_gps_blob()])
}

/// An unclean block first, then a clean one (the order a check of the last
/// block only would pass).
pub fn first_block_unclean() -> Vec<u8> {
    let second = exif_layout::first_entry_target().layout();
    jpeg_with(&[unclean_gps_blob(), second.bytes])
}

/// Three blocks, the middle one unclean.
pub fn middle_block_unclean() -> Vec<u8> {
    let a = exif_layout::first_entry_target().layout();
    let c = exif_layout::last_entry_target().layout();
    jpeg_with(&[a.bytes, unclean_gps_blob(), c.bytes])
}

/// A block whose GPS IFD declares 0xFFFF entries, coordinates behind it.
pub fn unclean_gps_blob() -> Vec<u8> {
    let b = exif_layout::Build {
        be: false,
        ifds: vec![
            exif_layout::ifd(
                "ifd0",
                vec![
                    exif_layout::short(false, 0x0112, 1),
                    exif_layout::ptr(0x8825, "gps"),
                ],
                None,
            ),
            exif_layout::ifd(
                "gps",
                vec![exif_layout::canaried(exif_layout::rational(
                    false,
                    0x0002,
                    &exif_layout::K_LAT,
                ))],
                None,
            ),
        ],
        thumb: None,
        pad_to: None,
    };
    let mut second = b.layout();
    let at = second.ifd("gps").table.start;
    second.bytes[at..at + 2].copy_from_slice(&0xFFFFu16.to_le_bytes());
    second.bytes
}

/// A clean first Exif block before the scan, a second one (clean) after it.
pub fn second_block_after_scan() -> Vec<u8> {
    let l = exif_layout::full_le().layout();
    let first = exif_layout::first_entry_target().layout();
    let mut v = vec![0xFF, 0xD8];
    v.extend_from_slice(&jfif_bytes());
    v.extend_from_slice(&real_shapes::exif_app1(&first.bytes));
    v.extend_from_slice(&real_shapes::seg(0xDA, &[1, 1, 0, 0, 0x3F, 0]));
    v.extend_from_slice(&[0x12, 0x34]);
    v.extend_from_slice(&real_shapes::exif_app1(&l.bytes));
    v.extend_from_slice(&real_shapes::EOI);
    v
}

/// Seventeen small Exif segments (the audit parses sixteen).
pub fn exif_17_segments() -> Vec<u8> {
    let l = exif_layout::first_entry_target().layout();
    let blobs: Vec<Vec<u8>> = (0..17).map(|_| l.bytes.clone()).collect();
    jpeg_with(&blobs)
}

/// Thirteen blocks of 5,459 entries: 70,967 tag items, past the audit's cap of
/// 65,536 (about 850 KB; a deterministic test only, not a seed).
pub fn exif_inventory_over_budget() -> Vec<u8> {
    let ents = (0..5459)
        .map(|i| exif_layout::short(false, 0x9000 + (i % 100) as u16, 1))
        .collect();
    let big = exif_layout::Build {
        be: false,
        ifds: vec![exif_layout::ifd("ifd0", ents, None)],
        thumb: None,
        pad_to: None,
    }
    .layout();
    let blobs: Vec<Vec<u8>> = (0..13).map(|_| big.bytes.clone()).collect();
    jpeg_with(&blobs)
}

/// The leak inputs of SPEC-021 as JPEG files, by the names the spec uses
/// (`gps_count_overflow` is SPEC-020's fixture in `strip_shapes`;
/// `exif_inventory_over_budget` is [`exif_inventory_over_budget`]).
pub fn leak_inputs_jpeg() -> Vec<(&'static str, Vec<u8>)> {
    let one = |l: Layout| jpeg_of(&[l], false).bytes;
    vec![
        (
            "straddling_artist_le",
            one(exif_layout::straddling_artist(false).layout()),
        ),
        (
            "straddling_artist_be",
            one(exif_layout::straddling_artist(true).layout()),
        ),
        (
            "straddling_gps_value_le",
            one(exif_layout::straddling_gps_value(false).layout()),
        ),
        (
            "straddling_gps_value_be",
            one(exif_layout::straddling_gps_value(true).layout()),
        ),
        (
            "thumbnail_second_pair_le",
            one(exif_layout::thumbnail_second_pair(false).layout()),
        ),
        (
            "thumbnail_second_pair_be",
            one(exif_layout::thumbnail_second_pair(true).layout()),
        ),
        (
            "thumbnail_with_strips_le",
            one(exif_layout::thumbnail_with_strips(false).layout()),
        ),
        (
            "thumbnail_with_strips_be",
            one(exif_layout::thumbnail_with_strips(true).layout()),
        ),
        (
            "typed_ifd1_le",
            one(exif_layout::typed_offset_entry(false, "ifd1").layout()),
        ),
        (
            "typed_gps_le",
            one(exif_layout::typed_offset_entry(false, "gps").layout()),
        ),
        (
            "typed_ifd0_le",
            one(exif_layout::typed_offset_entry(false, "ifd0").layout()),
        ),
        (
            "kept_strip_target_le",
            one(exif_layout::kept_strip_target_overlapped(false).layout()),
        ),
        (
            "ifd1_kept_category_le",
            one(exif_layout::thumbnail_ifd1_with_a_kept_category_tag(false).layout()),
        ),
        (
            "typed_ifd1_be",
            one(exif_layout::typed_offset_entry(true, "ifd1").layout()),
        ),
        (
            "typed_gps_be",
            one(exif_layout::typed_offset_entry(true, "gps").layout()),
        ),
        (
            "typed_ifd0_be",
            one(exif_layout::typed_offset_entry(true, "ifd0").layout()),
        ),
        (
            "kept_strip_target_be",
            one(exif_layout::kept_strip_target_overlapped(true).layout()),
        ),
        (
            "ifd1_kept_category_be",
            one(exif_layout::thumbnail_ifd1_with_a_kept_category_tag(true).layout()),
        ),
        (
            "interop_category_tag_le",
            one(exif_layout::interop_with_a_category_tag(false).layout()),
        ),
        (
            "interop_category_tag_be",
            one(exif_layout::interop_with_a_category_tag(true).layout()),
        ),
        (
            "gps_next_word_le",
            one(exif_layout::gps_with_a_next_word(false).layout()),
        ),
        (
            "gps_next_word_be",
            one(exif_layout::gps_with_a_next_word(true).layout()),
        ),
        (
            "subdirectory_0190_gps_le",
            one(exif_layout::exiftool_subdirectory(false, 0x0190, "gps").layout()),
        ),
        (
            "subdirectory_0190_gps_be",
            one(exif_layout::exiftool_subdirectory(true, 0x0190, "gps").layout()),
        ),
        (
            "subdirectory_0190_artist_le",
            one(exif_layout::exiftool_subdirectory(false, 0x0190, "artist").layout()),
        ),
        (
            "subdirectory_0190_artist_be",
            one(exif_layout::exiftool_subdirectory(true, 0x0190, "artist").layout()),
        ),
        (
            "subdirectory_888a_gps_le",
            one(exif_layout::exiftool_subdirectory(false, 0x888A, "gps").layout()),
        ),
        (
            "subdirectory_888a_gps_be",
            one(exif_layout::exiftool_subdirectory(true, 0x888A, "gps").layout()),
        ),
        (
            "subdirectory_888a_artist_le",
            one(exif_layout::exiftool_subdirectory(false, 0x888A, "artist").layout()),
        ),
        (
            "subdirectory_888a_artist_be",
            one(exif_layout::exiftool_subdirectory(true, 0x888A, "artist").layout()),
        ),
        (
            "subdirectory_8290_gps_le",
            one(exif_layout::exiftool_subdirectory(false, 0x8290, "gps").layout()),
        ),
        (
            "subdirectory_8290_gps_be",
            one(exif_layout::exiftool_subdirectory(true, 0x8290, "gps").layout()),
        ),
        (
            "subdirectory_8290_artist_le",
            one(exif_layout::exiftool_subdirectory(false, 0x8290, "artist").layout()),
        ),
        (
            "subdirectory_8290_artist_be",
            one(exif_layout::exiftool_subdirectory(true, 0x8290, "artist").layout()),
        ),
        (
            "subdirectory_fe00_gps_le",
            one(exif_layout::exiftool_subdirectory(false, 0xFE00, "gps").layout()),
        ),
        (
            "subdirectory_fe00_gps_be",
            one(exif_layout::exiftool_subdirectory(true, 0xFE00, "gps").layout()),
        ),
        (
            "subdirectory_fe00_artist_le",
            one(exif_layout::exiftool_subdirectory(false, 0xFE00, "artist").layout()),
        ),
        (
            "subdirectory_fe00_artist_be",
            one(exif_layout::exiftool_subdirectory(true, 0xFE00, "artist").layout()),
        ),
        (
            "blob_carrier_c51b_gps_le",
            one(exif_layout::blob_carrier(false, 0xC51B, "gps").layout()),
        ),
        (
            "blob_carrier_c51b_artist_le",
            one(exif_layout::blob_carrier(false, 0xC51B, "artist").layout()),
        ),
        (
            "blob_carrier_8568_iptc_le",
            one(exif_layout::blob_carrier(false, 0x8568, "iptc").layout()),
        ),
        (
            "blob_carrier_cd41_jumbf_le",
            one(exif_layout::blob_carrier(false, 0xCD41, "jumbf").layout()),
        ),
        (
            "profile_ifd_gps_le",
            one(exif_layout::exiftool_subdirectory(false, 0xC6F5, "gps").layout()),
        ),
        (
            "profile_ifd_artist_le",
            one(exif_layout::exiftool_subdirectory(false, 0xC6F5, "artist").layout()),
        ),
        (
            "h4_tag_in_interop_le",
            one(exif_layout::h4_tag_in_a_leaf_ifd(false, "interop").layout()),
        ),
        (
            "h4_tag_in_gps_le",
            one(exif_layout::h4_tag_in_a_leaf_ifd(false, "gps").layout()),
        ),
        (
            "blob_carrier_c51b_gps_be",
            one(exif_layout::blob_carrier(true, 0xC51B, "gps").layout()),
        ),
        (
            "blob_carrier_c51b_artist_be",
            one(exif_layout::blob_carrier(true, 0xC51B, "artist").layout()),
        ),
        (
            "blob_carrier_8568_iptc_be",
            one(exif_layout::blob_carrier(true, 0x8568, "iptc").layout()),
        ),
        (
            "blob_carrier_cd41_jumbf_be",
            one(exif_layout::blob_carrier(true, 0xCD41, "jumbf").layout()),
        ),
        (
            "profile_ifd_gps_be",
            one(exif_layout::exiftool_subdirectory(true, 0xC6F5, "gps").layout()),
        ),
        (
            "profile_ifd_artist_be",
            one(exif_layout::exiftool_subdirectory(true, 0xC6F5, "artist").layout()),
        ),
        (
            "h4_tag_in_interop_be",
            one(exif_layout::h4_tag_in_a_leaf_ifd(true, "interop").layout()),
        ),
        (
            "h4_tag_in_gps_be",
            one(exif_layout::h4_tag_in_a_leaf_ifd(true, "gps").layout()),
        ),
        ("exif_second_block_unclean", second_block_unclean()),
        ("exif_first_block_unclean", first_block_unclean()),
        ("exif_middle_block_unclean", middle_block_unclean()),
        (
            "thumbnail_tiles_le",
            one(exif_layout::thumbnail_hidden_offset(false, 0x0144).layout()),
        ),
        (
            "thumbnail_tiles_be",
            one(exif_layout::thumbnail_hidden_offset(true, 0x0144).layout()),
        ),
        (
            "thumbnail_jpegqtables_le",
            one(exif_layout::thumbnail_hidden_offset(false, 0x0207).layout()),
        ),
        (
            "thumbnail_jpegqtables_be",
            one(exif_layout::thumbnail_hidden_offset(true, 0x0207).layout()),
        ),
        (
            "thumbnail_free_offsets_le",
            one(exif_layout::thumbnail_hidden_offset(false, 0x0120).layout()),
        ),
        (
            "thumbnail_free_offsets_be",
            one(exif_layout::thumbnail_hidden_offset(true, 0x0120).layout()),
        ),
        (
            "thumbnail_unknown_tag_le",
            one(exif_layout::thumbnail_hidden_offset(false, 0xF123).layout()),
        ),
        (
            "thumbnail_unknown_tag_be",
            one(exif_layout::thumbnail_hidden_offset(true, 0xF123).layout()),
        ),
        (
            "gps_unknown_tag_le",
            one(exif_layout::gps_hidden_offset(false).layout()),
        ),
        (
            "gps_unknown_tag_be",
            one(exif_layout::gps_hidden_offset(true).layout()),
        ),
        (
            "thumbnail_two_offsets_le",
            one(exif_layout::thumbnail_two_offsets(false).layout()),
        ),
        (
            "thumbnail_two_offsets_be",
            one(exif_layout::thumbnail_two_offsets(true).layout()),
        ),
        (
            "kept_straddling_make_le",
            one(exif_layout::kept_straddling_value(false).layout()),
        ),
        (
            "kept_straddling_make_be",
            one(exif_layout::kept_straddling_value(true).layout()),
        ),
        ("exif_second_block_after_scan", second_block_after_scan()),
        (
            "gps_ifd_pointed_to_twice",
            one(exif_layout::gps_ifd_pointed_to_twice().layout()),
        ),
        (
            "gps_pointer_missing_in_parent",
            one(exif_layout::gps_pointer_hidden_behind_a_zero().layout()),
        ),
        (
            "value_shared_with_kept_entry",
            one(exif_layout::value_shared_with_kept_entry().layout()),
        ),
        (
            "value_inside_kept_ifd_table",
            one(exif_layout::value_inside_kept_ifd_table().layout()),
        ),
        (
            "value_overlaps_tiff_header",
            one(exif_layout::value_overlaps_tiff_header().layout()),
        ),
        (
            "thumbnail_range_overlaps_makernote",
            one(exif_layout::thumbnail_range_overlaps_makernote().layout()),
        ),
        (
            "thumbnail_strips_form",
            one(exif_layout::thumbnail_strips_form().layout()),
        ),
        (
            "thumbnail_with_ifd2_after_it",
            one(exif_layout::thumbnail_with_ifd2_after_it().layout()),
        ),
        ("exif_after_scan", exif_after_the_first_scan()),
        ("exif_17_segments", exif_17_segments()),
        (
            "exif_walk_issue_in_a_kept_ifd",
            one(exif_layout::walk_issue_in_a_kept_ifd().layout()),
        ),
        ("ifd0_emptied", one(exif_layout::ifd0_emptied().layout())),
    ]
}

/// The policy bytes and masks the entry seeds run under: `Gps` (0),
/// `GpsAndDevice` (1), `AllPii` (2) and a custom mask (6) for `{Xmp}`,
/// `{Software}`, `{Timestamps, Gps}`, `{MakerNote}` and `{Thumbnail}`, by
/// `Category::ALL` bit order (Gps 0, DeviceId 1, Authorship 2, FreeText 3,
/// Thumbnail 4, MakerNote 5, Xmp 6, Iptc 7, Jumbf 8, Timestamps 9, CameraModel
/// 10, Software 11, Icc 12).
pub const SEED_HEADS: [(&str, &[u8]); 8] = [
    ("gps", &[0]),
    ("gps_and_device", &[1]),
    ("all_pii", &[2]),
    ("mask0040", &[6, 0x40, 0x00]),
    ("mask0800", &[6, 0x00, 0x08]),
    ("mask0201", &[6, 0x01, 0x02]),
    ("mask0020", &[6, 0x20, 0x00]),
    ("mask0010", &[6, 0x10, 0x00]),
];

/// The seeds of the entry editor: every fixture and leak input under every
/// head of [`SEED_HEADS`] (the ceiling fixture, 64 KB, under two only), and the
/// leak inputs under `All` (byte 3) too. Names are `<head>-<kind>-<name>`.
pub fn entry_seeds() -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let mut add = |kind: &str, name: &str, bytes: &[u8], heads: &[(&str, &[u8])]| {
        for (label, head) in heads {
            let mut v = head.to_vec();
            v.extend_from_slice(bytes);
            out.push((format!("{label}-{kind}-{name}"), v));
        }
    };
    for (name, layouts) in exif_fixtures() {
        let j = jpeg_of(&layouts, false);
        if name == "at-the-ceiling" {
            add("exif", name, &j.bytes, &SEED_HEADS[..3][..1]);
            add("exif", name, &j.bytes, &SEED_HEADS[2..3]);
        } else {
            add("exif", name, &j.bytes, &SEED_HEADS);
        }
    }
    for (name, bytes) in leak_inputs_jpeg() {
        add("leak", name, &bytes, &SEED_HEADS);
        add("leak", name, &bytes, &[("allpreset", &[3])]);
    }
    out.extend(stray_seeds());
    out
}

/// The seeds of the joined-stray-bytes class (found by the 60 s smoke).
pub fn stray_seeds() -> Vec<(String, Vec<u8>)> {
    let mut built = vec![6, 0x40, 0x00];
    built.extend_from_slice(&joined_stray_bytes_input());
    vec![
        ("mask0040-joined-stray-bytes".to_string(), built),
        (
            "crash-mpf-fields-joined-stray".to_string(),
            CRASH_MPF_FIELDS.to_vec(),
        ),
        (
            "crash-entries-joined-stray".to_string(),
            CRASH_ENTRIES.to_vec(),
        ),
        (
            "crash-mpf-entry-table-overlap".to_string(),
            CRASH_MPF_TABLE.to_vec(),
        ),
        (
            "crash-kept-offset-array-nothing-to-edit".to_string(),
            CRASH_NOTHING_TO_EDIT.to_vec(),
        ),
    ]
}

/// The crash the 60 s smoke found (policy byte 6, mask 0x1140, then a 1,032
/// byte JPEG), verbatim; the committed seed is its only copy (the generator reads it, never rewrites it).
pub const CRASH_MPF_FIELDS: &[u8] =
    include_bytes!("../../fuzz/seeds/rewrite_jpeg/crash-mpf-fields-joined-stray");

/// The second crash of the same class (policy mask 0x6008, a 4.4 KB JPEG): the
/// removals change how many Exif blocks the output walk finds, so the
/// self-check refuses with `Entries`. Verbatim; its only copy is the seed.
pub const CRASH_ENTRIES: &[u8] =
    include_bytes!("../../fuzz/seeds/rewrite_jpeg/crash-entries-joined-stray");

/// The crash of HANDOFF-044 (policy All, 3,108 bytes): the MP index's entry
/// array starts inside its own IFD table, so the primary-size field of entry 0
/// is the MPEntry tag's count. Verbatim; its only copy is the seed.
pub const CRASH_MPF_TABLE: &[u8] =
    include_bytes!("../../fuzz/seeds/rewrite_jpeg/crash-mpf-entry-table-overlap");

/// A crash input (policy Gps): an IFD1 whose kept 0x0202 entry has an
/// unreadable type, and no GPS to delete. The library leaves the block alone
/// (empty plan); the harness' naive editor refused because it checked the kept
/// offset array before noticing there was nothing to edit. Verbatim.
pub const CRASH_NOTHING_TO_EDIT: &[u8] =
    include_bytes!("../../fuzz/seeds/rewrite_jpeg/crash-kept-offset-array-nothing-to-edit");

/// The same shape built by hand: fill bytes, an XMP segment, stray bytes that
/// start with a non-marker byte, an MPF index and a segment that runs past the
/// end. The stray bytes are skipped while the XMP segment stands; once it is
/// removed the fill bytes in front of it meet them and `FF FF 41 7F FF` reads
/// as a marker of length 32767, which swallows the index.
pub fn joined_stray_bytes_input() -> Vec<u8> {
    let mut v = vec![0xFF, 0xD8];
    v.extend_from_slice(&real_shapes::seg(0xE0, b"JFIF\0\x01\x01\0\0\x01\0\x01\0\0"));
    v.extend_from_slice(&[0xFF, 0xFF]);
    v.extend_from_slice(&real_shapes::seg(
        0xE1,
        b"http://ns.adobe.com/xap/1.0/\0<x:xmpmeta/>",
    ));
    v.extend_from_slice(&[0x41, 0x7F]);
    v.extend_from_slice(&real_shapes::mpf_segment(
        false,
        &[(0x2003_0000, 100, 0), (0, 10, 50)],
        32,
        true,
    ));
    // The input already has a `Truncated` issue (a segment that runs past the
    // end), so the output's one is not a new kind of issue.
    v.extend_from_slice(&[0xFF, 0xE3, 0xFF, 0xF0, 0x01]);
    v
}

/// The refusal the model expects, if any: IFD1 followed by a third IFD cannot
/// lose its thumbnail.
pub fn model_refusal(name: &str, spec: &[Cat]) -> Option<exif_core::write::UneditableReason> {
    (name == "chain-ifd2" && spec.contains(&"thumbnail"))
        .then_some(exif_core::write::UneditableReason::ChainAfterThumbnail)
}
