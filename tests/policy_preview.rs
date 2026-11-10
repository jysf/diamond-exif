//! SPEC-014: the strip-policy preview over full-house fixtures and every
//! committed seed. The expectation grids are typed here by hand from DEC-009;
//! nothing is derived from `StripPolicy::spec`, `may_hold` or `is_block`.

#[path = "support/oracle_jpeg.rs"]
mod oracle_jpeg;
#[path = "support/oracle_tiff.rs"]
mod oracle_tiff;
#[path = "support/real_shapes.rs"]
mod real_shapes;

use exif_core::audit::{audit, preview, Category, Disposition, FileAudit, Preview};
use exif_core::policy::StripPolicy;
use oracle_tiff as t;
use std::path::PathBuf;
use Category::*;
use Disposition::{Kept, KeptUnread, Removed};

/// DEC-009's preset table as amended by DEC-012, typed by hand: `[Gps,
/// GpsAndDevice, AllPii, All, Everything]`, `true` = removed.
const REMOVED: [(Category, [bool; 5]); 15] = [
    (Gps, [true, true, true, true, true]),
    (DeviceId, [false, true, true, true, true]),
    (Authorship, [false, false, true, true, true]),
    (FreeText, [false, false, true, true, true]),
    (Timestamps, [false, false, true, true, true]),
    (CameraModel, [false, false, true, true, true]),
    (Thumbnail, [false, false, true, true, true]),
    (MakerNote, [false, false, true, true, true]),
    (Software, [false, false, false, true, true]),
    (Xmp, [false, false, false, true, true]),
    (Iptc, [false, false, false, true, true]),
    (Icc, [false, false, false, false, true]),
    (Jumbf, [false, false, false, false, true]),
    (EmbeddedImage, [false, false, false, false, true]),
    (OtherSegment, [false, false, false, false, true]),
];

/// The SPEC-014 kept-block table as amended by DEC-012, typed by hand (same
/// column order).
const KEPT_BLOCK: [(Category, [Disposition; 5]); 8] = [
    (
        Thumbnail,
        [KeptUnread, KeptUnread, Removed, Removed, Removed],
    ),
    (
        MakerNote,
        [KeptUnread, KeptUnread, Removed, Removed, Removed],
    ),
    (Xmp, [KeptUnread, KeptUnread, KeptUnread, Removed, Removed]),
    (Iptc, [Kept, Kept, KeptUnread, Removed, Removed]),
    (
        Jumbf,
        [KeptUnread, KeptUnread, KeptUnread, KeptUnread, Removed],
    ),
    (Icc, [Kept, Kept, Kept, Kept, Removed]),
    (
        EmbeddedImage,
        [KeptUnread, KeptUnread, KeptUnread, KeptUnread, Removed],
    ),
    (
        OtherSegment,
        [KeptUnread, KeptUnread, KeptUnread, KeptUnread, Removed],
    ),
];

const PRESET_NAMES: [&str; 5] = ["gps", "gps_and_device", "all_pii", "all", "everything"];

/// What DEC-009 says happens to a finding of `c` under preset column `col`.
fn expected(col: usize, c: Category) -> Disposition {
    let removed = REMOVED
        .iter()
        .find(|(g, _)| *g == c)
        .map(|(_, cells)| cells[col])
        .expect("every category is in the grid");
    if removed {
        return Removed;
    }
    match KEPT_BLOCK.iter().find(|(b, _)| *b == c) {
        Some((_, cells)) => cells[col],
        None => Kept,
    }
}

// ── fixtures ────────────────────────────────────────────────────────────────

/// The full-house Exif TIFF: one tag of each of the seven tag categories across
/// IFD0, ExifIFD, GPS and IFD1, plus a MakerNote and an IFD1 thumbnail.
fn full_house_tiff() -> Vec<u8> {
    t::assemble(
        false,
        &[
            // 0: IFD0 (chains to IFD1 at index 3)
            t::ifd_next(
                vec![
                    t::ascii(0x010E, b"a description\0"),               // FreeText
                    t::ascii(t::T_MAKE, b"MakeCo\0"),                   // CameraModel
                    t::ascii(t::T_SOFTWARE, b"Editor 1.0\0"),           // Software
                    t::ascii(t::T_DATE_TIME, b"2024:05:06 07:08:09\0"), // Timestamps
                    t::ascii(t::T_ARTIST, b"A. Person\0"),              // Authorship
                    t::ptr(t::T_EXIF_IFD, 1),
                    t::ptr(t::T_GPS_IFD, 2),
                ],
                3,
            ),
            // 1: ExifIFD
            t::ifd(vec![
                t::ascii(t::T_DATE_TIME_ORIGINAL, b"2024:05:06 07:08:09\0"),
                t::ascii(0xA431, b"SERIAL0001\0"), // DeviceId (BodySerialNumber)
                t::ascii(0x927C, b"VendorXY"),     // MakerNote (decided by id)
            ]),
            // 2: GPS
            t::ifd(vec![t::ascii(t::T_GPS_LAT_REF, b"N\0")]),
            // 3: IFD1 with a thumbnail pointer (the range need not be valid)
            t::ifd(vec![t::long(false, 0x0201, 0), t::long(false, 0x0202, 0)]),
        ],
    )
}

fn segment(marker: u8, payload: &[u8]) -> Vec<u8> {
    oracle_jpeg::seg(marker, payload)
}

fn xmp_segment() -> Vec<u8> {
    segment(
        0xE1,
        &[
            &b"http://ns.adobe.com/xap/1.0/\0"[..],
            &oracle_jpeg::xmp_packet(),
        ]
        .concat(),
    )
}

fn iptc_segment() -> Vec<u8> {
    // Photoshop 3.0, one 8BIM resource 0x0404 holding one IIM dataset.
    let data = [0x1C, 0x02, 0x05, 0x00, 0x04, b'T', b'i', b't', b'l'];
    let mut res = b"8BIM".to_vec();
    res.extend_from_slice(&0x0404u16.to_be_bytes());
    res.extend_from_slice(&[0, 0]);
    res.extend_from_slice(&u32::try_from(data.len()).unwrap().to_be_bytes());
    res.extend_from_slice(&data);
    segment(0xED, &[&b"Photoshop 3.0\0"[..], &res].concat())
}

fn icc_segment() -> Vec<u8> {
    segment(0xE2, b"ICC_PROFILE\0\x01\x01profile")
}

fn jumbf_segment() -> Vec<u8> {
    segment(
        0xEB,
        &[
            &[0x4A, 0x50, 0x00, 0x01][..],
            &[0x00, 0x00, 0x00, 0x01][..],
            &b"\0\0\0\x08jumb"[..],
        ]
        .concat(),
    )
}

fn jpeg_of(before_scan: &[Vec<u8>], after_scan: &[Vec<u8>]) -> Vec<u8> {
    let mut d = oracle_jpeg::SOI.to_vec();
    for s in before_scan {
        d.extend_from_slice(s);
    }
    d.extend_from_slice(&oracle_jpeg::sos_header());
    d.extend_from_slice(&oracle_jpeg::SCAN);
    for s in after_scan {
        d.extend_from_slice(s);
    }
    d.extend_from_slice(&oracle_jpeg::EOI);
    d
}

fn full_house_jpeg() -> Vec<u8> {
    jpeg_of(
        &[
            oracle_jpeg::exif_app1(&full_house_tiff()),
            xmp_segment(),
            iptc_segment(),
            icc_segment(),
            jumbf_segment(),
        ],
        &[],
    )
}

fn categories_of(a: &FileAudit<'_>) -> Vec<Category> {
    a.findings().iter().map(|f| f.category).collect()
}

/// For every preset and every finding: the DEC-009 outcome.
fn assert_matches_dec_009(label: &str, a: &FileAudit<'_>) {
    assert!(!a.findings().is_empty(), "{label}: has findings");
    for (col, preset) in StripPolicy::PRESETS.iter().enumerate() {
        let p = preview(a, *preset);
        assert_eq!(p.dispositions().len(), a.findings().len(), "{label}");
        for (i, f) in a.findings().iter().enumerate() {
            assert_eq!(
                p.dispositions()[i],
                expected(col, f.category),
                "{label} / {} / finding {i} ({:?})",
                PRESET_NAMES[col],
                f.category
            );
        }
    }
}

// ── the named tests ─────────────────────────────────────────────────────────

#[test]
fn full_house_jpeg_matches_dec_009_for_every_preset() {
    let bytes = full_house_jpeg();
    let a = audit(&bytes);
    // Premise: the 13 DEC-008 categories are present, and the file is
    // reliable. The two DEC-012 categories are covered by the next test.
    for c in Category::ALL {
        if matches!(c, EmbeddedImage | OtherSegment) {
            continue;
        }
        assert!(categories_of(&a).contains(&c), "full house lacks {c:?}");
    }
    assert!(a.health().clean && !a.health().inventory_truncated);
    assert_matches_dec_009("full-house jpeg", &a);
}

#[test]
fn real_shape_fixtures_match_dec_009_as_amended_for_every_preset() {
    // Together the SPEC-019 fixtures carry both new categories and FreeText
    // (a COM segment); each file's findings follow the hand-typed grids.
    let mut seen: Vec<Category> = Vec::new();
    for (name, bytes) in real_shapes::fixtures() {
        let a = audit(&bytes);
        if a.findings().is_empty() {
            continue;
        }
        seen.extend(categories_of(&a));
        assert_matches_dec_009(name, &a);
    }
    for c in [EmbeddedImage, OtherSegment, FreeText] {
        assert!(seen.contains(&c), "the fixtures lack {c:?}");
    }
}

#[test]
fn full_house_tiff_matches_dec_009_for_every_preset() {
    let bytes = full_house_tiff();
    let a = audit(&bytes);
    // Premise: seven tag categories + MakerNote + Thumbnail, and no regions.
    let present = categories_of(&a);
    for c in [
        Gps,
        DeviceId,
        Authorship,
        FreeText,
        Timestamps,
        CameraModel,
        Software,
        MakerNote,
        Thumbnail,
    ] {
        assert!(present.contains(&c), "tiff lacks {c:?}");
    }
    for c in [Xmp, Iptc, Icc, Jumbf] {
        assert!(!present.contains(&c), "tiff has region category {c:?}");
    }
    assert!(a.health().clean);
    assert_matches_dec_009("full-house tiff", &a);
}

#[test]
fn the_removed_categories_of_each_preview_equal_the_dec_009_column() {
    let bytes = full_house_jpeg();
    let a = audit(&bytes);
    for (col, preset) in StripPolicy::PRESETS.iter().enumerate() {
        let p = preview(&a, *preset);
        let mut removed: Vec<Category> = a
            .findings()
            .iter()
            .zip(p.dispositions())
            .filter(|(_, d)| **d == Removed)
            .map(|(f, _)| f.category)
            .collect();
        removed.sort_by_key(|c| c.name());
        removed.dedup();
        let mut column: Vec<Category> = REMOVED
            .iter()
            // The full house carries no embedded image and no other segment.
            .filter(|(c, cells)| cells[col] && !matches!(c, EmbeddedImage | OtherSegment))
            .map(|(c, _)| *c)
            .collect();
        column.sort_by_key(|c| c.name());
        assert_eq!(removed, column, "{}", PRESET_NAMES[col]);
    }
}

fn kept_unread_cats(a: &FileAudit<'_>, p: &Preview) -> Vec<Category> {
    let mut v: Vec<Category> = a
        .findings()
        .iter()
        .zip(p.dispositions())
        .filter(|(_, d)| **d == KeptUnread)
        .map(|(f, _)| f.category)
        .collect();
    v.sort_by_key(|c| c.name());
    v
}

#[test]
fn gps_preview_of_the_full_house_reports_the_unread_blocks() {
    let bytes = full_house_jpeg();
    let a = audit(&bytes);
    let p = preview(&a, StripPolicy::Gps);
    let mut want = vec![Thumbnail, MakerNote, Xmp, Jumbf];
    want.sort_by_key(|c| c.name());
    assert_eq!(kept_unread_cats(&a, &p), want);
    for (f, d) in a.findings().iter().zip(p.dispositions()) {
        if matches!(f.category, Iptc | Icc) {
            assert_eq!(*d, Kept, "{:?}", f.category);
        }
    }
    assert!(p.counts().kept_unread >= 4);
    assert!(p.residual_findings() > 0);
}

#[test]
fn all_preview_reports_only_jumbf_unread() {
    let bytes = full_house_jpeg();
    let a = audit(&bytes);
    let p = preview(&a, StripPolicy::All);
    assert_eq!(kept_unread_cats(&a, &p), vec![Jumbf]);
    assert_eq!(p.counts().kept_unread, 1);
    // The one other residual finding is the ICC profile, kept.
    assert_eq!(p.counts().kept, 1);
    assert_eq!(p.residual_findings(), 2);
}

#[test]
fn everything_leaves_no_residual_findings_on_any_reliable_fixture() {
    let mut inputs: Vec<(String, Vec<u8>)> = vec![
        ("full-house-jpeg".to_string(), full_house_jpeg()),
        ("full-house-tiff".to_string(), full_house_tiff()),
    ];
    for (name, bytes) in oracle_tiff::fixtures() {
        inputs.push((name.to_string(), bytes));
    }
    for (name, bytes) in oracle_jpeg::fixtures() {
        inputs.push((name, bytes));
    }
    let mut reliable_seen = 0usize;
    for (name, bytes) in &inputs {
        let a = audit(bytes);
        let p = preview(&a, StripPolicy::Everything);
        // Holds for every audit, reliable or not: Everything removes every
        // category, so nothing the audit found remains.
        assert_eq!(p.residual_findings(), 0, "{name}");
        assert_eq!(p.residual_severity(), None, "{name}");
        assert_eq!(p.counts().removed, a.findings().len(), "{name}");
        if p.reliable() {
            reliable_seen += 1;
        }
    }
    assert!(
        reliable_seen >= 20,
        "enough reliable fixtures: {reliable_seen}"
    );
}

#[test]
fn two_exif_blocks_give_the_same_disposition_per_copy() {
    let tiff = full_house_tiff();
    let bytes = jpeg_of(
        &[oracle_jpeg::exif_app1(&tiff), oracle_jpeg::exif_app1(&tiff)],
        &[],
    );
    let one = full_house_tiff();
    let single = audit(&one);
    let a = audit(&bytes);
    assert_eq!(a.findings().len(), 2 * single.findings().len());
    for (col, preset) in StripPolicy::PRESETS.iter().enumerate() {
        let p = preview(&a, *preset);
        for c in Category::ALL {
            let ds: Vec<Disposition> = a
                .findings()
                .iter()
                .zip(p.dispositions())
                .filter(|(f, _)| f.category == c)
                .map(|(_, d)| *d)
                .collect();
            if ds.is_empty() {
                continue;
            }
            assert_eq!(ds.len() % 2, 0, "{c:?} appears once per copy");
            assert!(
                ds.iter().all(|d| *d == expected(col, c)),
                "{} / {c:?}: every copy gets the same disposition",
                PRESET_NAMES[col]
            );
        }
    }
}

#[test]
fn exif_block_after_the_scan_is_previewed_like_any_other() {
    let tiff = full_house_tiff();
    let bytes = jpeg_of(&[], &[oracle_jpeg::exif_app1(&tiff)]);
    let a = audit(&bytes);
    assert!(!a.findings().is_empty());
    assert!(a.findings().iter().all(|f| f.at.origin().after_scan));
    assert_matches_dec_009("exif after scan", &a);
    let before_bytes = jpeg_of(&[oracle_jpeg::exif_app1(&tiff)], &[]);
    let before = audit(&before_bytes);
    for preset in StripPolicy::PRESETS {
        assert_eq!(
            preview(&a, preset).dispositions(),
            preview(&before, preset).dispositions(),
            "{}",
            preset.name()
        );
    }
}

fn seed_dir(target: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fuzz/seeds")
        .join(target)
}

/// Every committed seed, as `(path, bytes)`, with the per-directory counts.
fn all_seeds() -> (Vec<(String, Vec<u8>)>, [usize; 3]) {
    let mut out = Vec::new();
    let mut seen = [0usize; 3];
    for (slot, target) in ["tiff_ifd", "jpeg_segments", "audit"].iter().enumerate() {
        let dir = seed_dir(target);
        for entry in std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
            let path = entry.expect("entry").path();
            let data = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            out.push((path.display().to_string(), data));
            seen[slot] += 1;
        }
    }
    (out, seen)
}

#[test]
fn nesting_holds_on_every_committed_seed_for_every_preset() {
    let (seeds, seen) = all_seeds();
    // The same counts audit_model.rs asserts (32 TIFF, 37 JPEG, 78 audit),
    // plus the 15 SPEC-019 `shape-*` seeds in each of the last two.
    assert_eq!(seen, [32, 37 + 15, 78 + 15]);
    let mut with_findings = 0usize;
    for (name, data) in &seeds {
        let a = audit(data);
        let previews: Vec<Preview> = StripPolicy::PRESETS
            .iter()
            .map(|p| preview(&a, *p))
            .collect();
        for pair in previews.windows(2) {
            assert!(
                pair[0].counts().removed <= pair[1].counts().removed,
                "{name}: removed non-decreasing"
            );
            assert!(
                pair[0].residual_findings() >= pair[1].residual_findings(),
                "{name}: residual non-increasing"
            );
        }
        let last = previews.last().expect("five presets");
        assert_eq!(last.residual_findings(), 0, "{name}");
        assert_eq!(last.residual_severity(), None, "{name}");
        if !a.findings().is_empty() {
            with_findings += 1;
        }
    }
    assert!(with_findings > 20, "seeds with findings: {with_findings}");
}

#[test]
fn preview_never_panics_on_every_committed_seed() {
    let (seeds, _) = all_seeds();
    for (name, data) in &seeds {
        let a = audit(data);
        let custom = exif_core::StripSpec::none().include(Icc).include(Xmp);
        for policy in StripPolicy::PRESETS
            .into_iter()
            .chain([StripPolicy::Custom(custom)])
        {
            let p = preview(&a, policy);
            assert_eq!(p.dispositions().len(), a.findings().len(), "{name}");
            let c = p.counts();
            assert_eq!(c.removed + c.kept + c.kept_unread, a.findings().len());
            assert_eq!(p.residual_findings(), c.kept + c.kept_unread);
            if matches!(a.kind(), exif_core::audit::ContainerKind::Unsupported(_)) {
                assert!(!p.reliable() && p.dispositions().is_empty(), "{name}");
            }
        }
    }
}

// Unreadable, unsupported and empty inputs are also covered at unit level; one
// end-to-end case per kind through the public API:
#[test]
fn unsupported_empty_and_unreadable_inputs_preview_to_nothing_and_unreliable() {
    let inputs: [(&str, Vec<u8>); 3] = [
        ("png", b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec()),
        ("empty", Vec::new()),
        (
            "tiff with unreadable IFD0",
            b"II*\0\x08\0\0\0\xFF\xFF".to_vec(),
        ),
    ];
    for (label, bytes) in inputs {
        let a = audit(&bytes);
        for preset in StripPolicy::PRESETS {
            let p = preview(&a, preset);
            assert!(p.dispositions().is_empty(), "{label}");
            assert!(!p.reliable(), "{label}");
        }
    }
}
