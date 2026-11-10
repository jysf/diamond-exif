//! SPEC-019: the synthetic real-world shapes (`tests/support/real_shapes.rs`)
//! against the library and against ExifTool.
//!
//! The shapes are the structures SPIKE-001 found on real camera and phone files
//! that a strip has to deal with; real files cannot be committed, so each is
//! rebuilt here. This file checks that each builder is deterministic and
//! parses, that the audit reports what DEC-012 says of it, and (live, and
//! replayed from the committed records in `tests/oracle-fixtures/
//! exiftool-inventory/`) that ExifTool reads each as intended. The live tests
//! skip loudly without ExifTool; `DIAMOND_EXIF_REQUIRE_ORACLE=1` turns the skip
//! into a failure.

#[path = "support/audit_tags.rs"]
mod audit_tags;
#[path = "support/inventory.rs"]
mod inventory;
#[path = "support/inventory_corpus.rs"]
mod inventory_corpus;
#[path = "support/oracle_jpeg.rs"]
mod oracle_jpeg;
#[path = "support/oracle_tiff.rs"]
mod oracle_tiff;
#[path = "support/real_shapes.rs"]
mod real_shapes;
#[path = "support/tools.rs"]
mod tools;

use exif_core::audit::{audit, Category, ItemKind, RegionKind};
use exif_core::jpeg::{self, Region};
use exif_core::Error;
use std::collections::BTreeSet;
use std::path::PathBuf;

const REQUIRE_VAR: &str = "DIAMOND_EXIF_REQUIRE_ORACLE";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn live_gate(test: &str) -> bool {
    if tools::exiftool_available() {
        return true;
    }
    if std::env::var(REQUIRE_VAR).as_deref() == Ok("1") {
        panic!("{test}: exiftool is not on PATH and {REQUIRE_VAR}=1 makes that a failure");
    }
    println!("SKIP {test} \u{2014} exiftool not on PATH");
    false
}

fn fixture(name: &str) -> Vec<u8> {
    real_shapes::fixtures()
        .into_iter()
        .find(|(n, _)| *n == name)
        .map(|(_, b)| b)
        .unwrap_or_else(|| panic!("no fixture {name}"))
}

/// The committed ExifTool listing of one fixture.
fn recorded(name: &str) -> inventory::Section {
    let path = root()
        .join(inventory_corpus::RECORD_DIR)
        .join(inventory_corpus::record_file("real-shapes"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e} (run `just oracle-record`)", path.display()));
    let rec = inventory::parse_record(&text).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(rec.corpus, "real-shapes");
    assert_eq!(rec.version, "13.55");
    rec.sections
        .into_iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("no section {name}"))
}

fn live_lines(bytes: &[u8]) -> Vec<inventory::Line> {
    let path = tools::temp_file_with_ext("real-shapes-live", "jpg", bytes).expect("temp file");
    let args: Vec<String> = inventory::COMMAND_ARGS
        .iter()
        .map(|s| s.to_string())
        .collect();
    let run = tools::run_tool("exiftool", &args, &path);
    let _ = std::fs::remove_file(&path);
    let out = run.expect("exiftool runs");
    inventory::parse_stdout(&String::from_utf8_lossy(&out.stdout)).expect("parses")
}

fn groups(lines: &[inventory::Line]) -> BTreeSet<String> {
    lines.iter().map(|l| l.group.clone()).collect()
}

fn ids(lines: &[inventory::Line], group: &str) -> Vec<u32> {
    lines
        .iter()
        .filter(|l| l.group == group)
        .filter_map(|l| l.id)
        .collect()
}

// ── the builders ────────────────────────────────────────────────────────────

#[test]
fn every_builder_is_deterministic_and_parses() {
    let a = real_shapes::fixtures();
    let b = real_shapes::fixtures();
    assert_eq!(a, b, "fixtures() is a pure function");
    assert_eq!(a.len(), 15);
    let names: BTreeSet<&str> = a.iter().map(|(n, _)| *n).collect();
    assert_eq!(names.len(), a.len(), "names are unique");
    // Each builder function on its own is deterministic too.
    assert_eq!(real_shapes::mpo_two(false), real_shapes::mpo_two(false));
    assert_eq!(real_shapes::split_app13(), real_shapes::split_app13());
    assert_eq!(
        real_shapes::canon_style_absolute_note(),
        real_shapes::canon_style_absolute_note()
    );
    for (name, bytes) in &a {
        // A JPEG, readable (never the fatal not-a-JPEG error), audited as one.
        let m = jpeg::read(bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(!m.jpeg().segments().is_empty(), "{name}");
        let au = audit(bytes);
        assert!(
            matches!(au.kind(), exif_core::audit::ContainerKind::Jpeg),
            "{name}"
        );
        assert_eq!(au.health().failed_blocks, 0, "{name}");
    }
    // The shapes that are clean files are clean: the damaged ones are named.
    let damaged = [
        "mpo-truncated-index",
        "mpo-offset-out-of-range",
        "mpo-image-not-jpeg",
        "mpo-seventeen",
        "split-app13",
    ];
    for (name, bytes) in &a {
        let clean = audit(bytes).health().clean;
        assert_eq!(clean, !damaged.contains(name), "{name}: clean = {clean}");
    }
}

#[test]
fn the_mpo_fixture_audits_with_holds_matching_its_inner_images() {
    for be in [false, true] {
        let file = real_shapes::mpo_two(be);
        let a = audit(&file);
        assert!(a.health().clean, "be={be}: {:?}", a.health());
        let emb: Vec<_> = a
            .findings_in(Category::EmbeddedImage)
            .map(|f| f.embedded.expect("an embedded summary"))
            .collect();
        assert_eq!(emb.len(), 2, "be={be}");
        let extras = [real_shapes::nikon_extra(), real_shapes::iphone_extra()];
        let mut at = file.len() - extras.iter().map(Vec::len).sum::<usize>();
        for (e, extra) in emb.iter().zip(&extras) {
            assert_eq!((e.offset, e.length), (at, extra.len()), "be={be}");
            assert!(e.starts_with_ffd8);
            // holds is the category set of the ordinary audit of those bytes.
            let inner = audit(extra);
            let want: Vec<Category> = Category::ALL
                .into_iter()
                .filter(|c| inner.findings_in(*c).next().is_some())
                .collect();
            assert_eq!(e.holds.categories().collect::<Vec<_>>(), want, "be={be}");
            at += extra.len();
        }
        // The Nikon-style extra carries its own Exif; the iPhone-style one XMP
        // and an ICC profile (SPIKE-001).
        assert!(emb[0].holds.contains(Category::CameraModel));
        assert!(emb[0].holds.contains(Category::Timestamps));
        assert!(!emb[0].holds.contains(Category::Xmp));
        assert!(emb[1].holds.contains(Category::Xmp));
        assert!(emb[1].holds.contains(Category::Icc));
        assert!(!emb[1].holds.contains(Category::CameraModel));
    }
    // The outer Exif is the primary's; the inner ones are not in the inventory.
    let file = real_shapes::mpo_two(false);
    let a = audit(&file);
    let tags = a
        .items()
        .iter()
        .filter(|i| matches!(i.kind(), ItemKind::Tag { .. }))
        .count();
    let primary_bytes = real_shapes::jpeg_of(&[real_shapes::exif_app1(&primary_tiff(&file))]);
    let primary = audit(&primary_bytes);
    assert_eq!(tags, primary.items().len());
    // An inner MPF index does not recurse: one embedded image, whose holds is
    // an OtherSegment (its own MPF segment, location off).
    let file = real_shapes::mpo_inner_mpf();
    let a = audit(&file);
    let emb: Vec<_> = a
        .findings_in(Category::EmbeddedImage)
        .map(|f| f.embedded.expect("summary"))
        .collect();
    assert_eq!(emb.len(), 1);
    assert_eq!(
        emb[0].holds.categories().collect::<Vec<_>>(),
        [Category::OtherSegment]
    );
}

/// The TIFF of the primary of an MPO: its first Exif block.
fn primary_tiff(file: &[u8]) -> Vec<u8> {
    let m = jpeg::read(file).unwrap();
    m.exif_blocks()[0].tiff().to_vec()
}

#[test]
fn the_split_app13_fixture_gives_two_iptc_regions() {
    let file = real_shapes::split_app13();
    let a = audit(&file);
    let kinds: Vec<RegionKind> = a
        .items()
        .iter()
        .filter_map(|i| match i.kind() {
            ItemKind::Region { kind, .. } => Some(kind),
            ItemKind::Tag { .. } => None,
        })
        .collect();
    assert_eq!(kinds, [RegionKind::Iptc, RegionKind::Iptc]);
    assert_eq!(a.findings_in(Category::Iptc).count(), 2);
    // The issues the walk of the first segment's cut resource always gave.
    let m = jpeg::read(&file).unwrap();
    let e: Vec<&Error> = m.jpeg_issues().iter().map(|i| i.error()).collect();
    assert!(matches!(
        e[..],
        [
            Error::RegionTooShort {
                what: "8BIM resource",
                ..
            },
            Error::StrayBytes { .. }
        ]
    ));
    assert!(!a.health().clean);
    // The two regions together are the whole Photoshop block.
    let data: Vec<&[u8]> = m
        .regions()
        .iter()
        .filter_map(|r| match r {
            Region::Iptc { data, .. } => Some(*data),
            _ => None,
        })
        .collect();
    assert_eq!(data.len(), 2);
    let whole = [data[0], data[1]].concat();
    assert!(whole.starts_with(b"8BIM\x04\x04"));
}

#[test]
fn the_arot_fpxr_com_and_trailer_fixtures_audit_as_other_segment_and_free_text() {
    let cats = |bytes: &[u8]| -> Vec<Category> {
        audit(bytes).findings().iter().map(|f| f.category).collect()
    };
    assert_eq!(cats(&real_shapes::app10_arot()), [Category::OtherSegment]);
    assert_eq!(cats(&real_shapes::app2_fpxr()), [Category::OtherSegment]);
    assert_eq!(cats(&real_shapes::com_segment()), [Category::FreeText]);
    // The trailer: the Exif tags (Make, Model, DateTimeOriginal) and the
    // 313-byte remainder as one OtherSegment finding.
    let file = real_shapes::trailer_313();
    let a = audit(&file);
    let other: Vec<_> = a
        .items()
        .iter()
        .filter_map(|i| match i.kind() {
            ItemKind::Region { kind, len, .. } if kind == RegionKind::Trailer => Some(len),
            _ => None,
        })
        .collect();
    assert_eq!(other, [real_shapes::TRAILER_LEN]);
    assert_eq!(a.findings_in(Category::OtherSegment).count(), 1);
    assert!(a.health().clean);
    // JFIF is benign in all of them: no finding comes from it.
    for bytes in [
        real_shapes::app10_arot(),
        real_shapes::app2_fpxr(),
        real_shapes::com_segment(),
    ] {
        assert_eq!(audit(&bytes).findings().len(), 1);
    }
}

#[test]
fn exif_at_64k_is_the_maximum_segment_and_parses() {
    let file = real_shapes::exif_at_64k();
    let m = jpeg::read(&file).unwrap();
    let exif: Vec<_> = m
        .jpeg()
        .segments()
        .iter()
        .filter(|s| s.kind() == jpeg::SegmentKind::Exif)
        .collect();
    assert_eq!(exif.len(), 1);
    // The payload is the largest a segment can carry: 65,533 bytes, a length
    // field of 0xFFFF.
    assert_eq!(exif[0].payload().len(), 65_533);
    let at = exif[0].offset();
    assert_eq!(&file[at + 2..at + 4], &[0xFF, 0xFF]);
    assert!(m.is_clean(), "{:?}", m.jpeg_issues());
    let c = m.exif().expect("the Exif parses");
    let ifd0 = c.ifd0().unwrap();
    assert!(ifd0.has(0x010E) && ifd0.has(0x010F));
    let entry = ifd0.entry(0x010F).expect("Make");
    let make = c.ascii(entry).unwrap();
    assert_eq!(make.as_bytes(), b"MaxCam");
    let a = audit(&file);
    assert!(a.health().clean);
    assert_eq!(a.findings().len(), 2);
}

// ── ExifTool ────────────────────────────────────────────────────────────────

/// What ExifTool is expected to list for each fixture (measured, 13.55): the
/// groups beyond the file-level ones, and the number of `MPImage<n>` groups.
fn expected_groups(name: &str) -> (Vec<&'static str>, usize) {
    // A Multi-Picture file whose primary carries Exif.
    let with_exif = vec!["IFD0", "ExifIFD", "JFIF", "MPF0"];
    match name {
        "mpo-two-le" | "mpo-two-be" => (with_exif, 3),
        // The index is cut: ExifTool reads the header and no image entry.
        "mpo-truncated-index" => (vec!["JFIF", "MPF0"], 0),
        // ExifTool names every index entry, whatever its range or bytes.
        "mpo-offset-out-of-range" | "mpo-image-not-jpeg" => (vec!["JFIF", "MPF0"], 3),
        "mpo-inner-mpf" => (with_exif, 2),
        "mpo-seventeen" => (vec!["JFIF", "MPF0"], 17),
        "split-app13" => (vec!["IPTC", "JFIF"], 0),
        "app10-arot" => (vec!["AROT", "JFIF"], 0),
        "app2-fpxr" | "com-segment" => (vec!["JFIF"], 0),
        "trailer-313" => (vec!["ExifIFD", "IFD0", "JFIF"], 0),
        "canon-style-note" | "canon-style-note-moved" => (vec!["Canon", "IFD0", "JFIF"], 0),
        "exif-64k" => (vec!["IFD0"], 0),
        other => panic!("no expectation for {other}"),
    }
}

fn check_listing(name: &str, lines: &[inventory::Line]) {
    let (want_groups, want_images) = expected_groups(name);
    let mut got = groups(lines);
    // MPImage<n> groups are counted, not listed.
    let n_images = inventory::mpimage_groups(lines);
    got.retain(|g| !g.starts_with("MPImage"));
    let want: BTreeSet<String> = want_groups.iter().map(|s| s.to_string()).collect();
    // The groups the table names must all be there.
    for g in &want {
        assert!(
            got.contains(g),
            "{name}: ExifTool lists no {g} (got {got:?})"
        );
    }
    assert_eq!(n_images, want_images, "{name}: MPImage groups");
}

#[test]
fn every_builder_is_read_by_exiftool_as_intended() {
    // Replay: the committed listing of each fixture.
    for (name, _) in real_shapes::fixtures() {
        let sec = recorded(name);
        check_listing(name, &sec.lines);
    }
    // The specific facts the fixtures exist for.
    let split = recorded("split-app13");
    assert_eq!(ids(&split.lines, "IPTC"), [0x5a, 0x78, 0x50]);
    let arot = recorded("app10-arot");
    assert_eq!(ids(&arot.lines, "AROT"), [0x06]);
    let c = recorded("canon-style-note");
    assert_eq!(ids(&c.lines, "Canon"), [0x06, 0x07, 0x09]);
    let moved = recorded("canon-style-note-moved");
    assert_ne!(ids(&moved.lines, "Canon"), [0x06, 0x07, 0x09]);
    let big = recorded("exif-64k");
    assert_eq!(ids(&big.lines, "IFD0"), [0x010e, 0x010f]);
    // Live: the same listing from the installed tool.
    if !live_gate("every_builder_is_read_by_exiftool_as_intended") {
        return;
    }
    for (name, bytes) in real_shapes::fixtures() {
        let lines = live_lines(&bytes);
        check_listing(name, &lines);
        assert_eq!(
            lines,
            recorded(name).lines,
            "{name}: live equals the record"
        );
    }
}

/// The values of the three Canon tags from a committed `-T -n -s3` record.
fn recorded_canon(name: &str) -> Vec<Option<String>> {
    let path = root()
        .join(inventory_corpus::RECORD_DIR)
        .join(real_shapes::canon_record_file(name));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e} (run `just oracle-record`)", path.display()));
    let rec = tools::parse_record(&text).expect("a record");
    assert_eq!(rec.version, "13.55");
    assert!(rec
        .command
        .contains("-T -n -s3 -Canon:CanonImageType -Canon:CanonFirmwareVersion -Canon:OwnerName"));
    tools::parse_fields(&real_shapes::CANON_SELECTORS, &rec.stdout).expect("fields")
}

#[test]
fn canon_style_note_is_read_by_exiftool_and_moving_it_changes_exiftools_view() {
    let intact = recorded_canon("canon-style-note");
    let moved = recorded_canon("canon-style-note-moved");
    // ExifTool reads Canon tags from the absolute-offset note...
    assert_eq!(
        intact,
        [
            Some("Canon EOS DIAMOND SHAPE".to_string()),
            Some("Firmware Version 1.0.0".to_string()),
            Some("Shape Owner".to_string())
        ]
    );
    // ...and when the note is moved inside the block, with its pointer updated
    // but its own offsets left alone, ExifTool's view changes: the relocation
    // hazard of SPIKE-001, reproduced synthetically.
    assert_ne!(moved, intact);
    assert!(
        moved.iter().any(Option::is_none),
        "a tag is lost: {moved:?}"
    );
    // The note's bytes are the same in both files: only its place differs.
    let payload = |bytes: &[u8]| {
        let a = audit(bytes);
        let item = a
            .items()
            .iter()
            .find(|i| matches!(i.kind(), ItemKind::Tag { tag: 0x927C, .. }))
            .expect("a MakerNote item");
        item.value().expect("a payload").to_vec()
    };
    let a = real_shapes::canon_style_absolute_note();
    let b = real_shapes::canon_style_note_moved();
    assert_eq!(payload(&a), payload(&b), "the same note bytes");
    let (at, len, blob) = real_shapes::canon_note_range();
    assert_eq!(len, payload(&a).len());
    assert!(at + len <= blob);
    // Live.
    if !live_gate("canon_style_note_is_read_by_exiftool_and_moving_it_changes_exiftools_view") {
        return;
    }
    for (name, want) in [
        ("canon-style-note", &intact),
        ("canon-style-note-moved", &moved),
    ] {
        let bytes = fixture(name);
        let path = tools::temp_file_with_ext(name, "jpg", &bytes).expect("temp file");
        let run = tools::run_exiftool(&path, &real_shapes::CANON_SELECTORS);
        let _ = std::fs::remove_file(&path);
        assert_eq!(&run.expect("exiftool runs"), want, "{name}");
    }
}
