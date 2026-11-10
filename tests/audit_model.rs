//! SPEC-012: the audit model over the committed fixtures and seeds.
//!
//! * Container independence at this layer: for every `oracle_tiff` fixture,
//!   `audit(bare tiff)` and `audit(the same tiff in a JPEG)` list the same tags
//!   and make the same findings; only the origins differ (block/segment).
//! * `audit` never panics on a committed seed and the answer is consistent.

#[path = "support/oracle_jpeg.rs"]
mod oracle_jpeg;
#[path = "support/oracle_tiff.rs"]
mod oracle_tiff;

use exif_core::audit::{audit, ContainerKind, FileAudit, ItemKind, MAX_INVENTORY_ITEMS};
use exif_core::tiff::IfdKind;
use std::path::PathBuf;

type TagRow = (IfdKind, usize, u16, u16, u32);

fn tag_rows(a: &FileAudit<'_>) -> Vec<TagRow> {
    a.items()
        .iter()
        .map(|i| match i.kind() {
            ItemKind::Tag {
                ifd,
                ifd_index,
                tag,
                field_type,
                count,
            } => (ifd, ifd_index, tag, field_type, count),
            ItemKind::Region { .. } => panic!("a region in a TIFF-only fixture"),
        })
        .collect()
}

/// `(category, position of its item in items(), thumbnail range)` per finding.
fn finding_rows(a: &FileAudit<'_>) -> Vec<(String, Option<usize>)> {
    a.findings()
        .iter()
        .map(|f| (format!("{:?}", f.category), f.item))
        .collect()
}

#[test]
fn container_independence_audit_of_every_oracle_fixture_bare_and_in_jpeg() {
    let wrapped = oracle_jpeg::wrapped();
    let bare = oracle_tiff::fixtures();
    assert_eq!(wrapped.len(), bare.len());
    assert_eq!(bare.len(), 12, "the 12 oracle_tiff fixtures");
    let mut compared = 0;
    for ((name, tiff), (jname, jpeg)) in bare.iter().zip(&wrapped) {
        assert_eq!(jname, &format!("jpeg-{name}"));
        let a = audit(tiff);
        let b = audit(jpeg);
        assert_eq!(a.kind(), ContainerKind::Tiff, "{name}");
        assert_eq!(b.kind(), ContainerKind::Jpeg, "{jname}");
        assert_eq!(tag_rows(&a), tag_rows(&b), "{name}: the same tags");
        assert_eq!(
            finding_rows(&a),
            finding_rows(&b),
            "{name}: the same findings"
        );
        assert_eq!(a.max_severity(), b.max_severity(), "{name}");
        assert_eq!(a.health().exif_issues, b.health().exif_issues, "{name}");
        assert_eq!(a.health().failed_blocks, b.health().failed_blocks, "{name}");
        // The origins differ only by block and segment: bare has none, the
        // wrapped file has block 0 of segment 0, before the scan.
        assert!(a.items().iter().all(|i| {
            let o = i.origin();
            o.block.is_none() && o.segment.is_none() && !o.after_scan
        }));
        assert!(b.items().iter().all(|i| {
            let o = i.origin();
            o.block == Some(0) && o.segment == Some(0) && !o.after_scan
        }));
        // Thumbnails (none in these fixtures) would compare equal as well.
        for (fa, fb) in a.findings().iter().zip(b.findings()) {
            assert_eq!(fa.thumbnail, fb.thumbnail, "{name}");
        }
        // The fixtures are readable.
        assert_eq!(b.health().failed_blocks, 0, "{jname}");
        assert!(!a.items().is_empty(), "{name}");
        compared += 1;
    }
    assert_eq!(compared, 12);
}

fn seed_dir(target: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fuzz/seeds")
        .join(target)
}

#[test]
fn audit_never_panics_on_every_committed_seed() {
    // 32 TIFF + 37 JPEG + the audit directory (69 copies + 9 new shapes), plus the
    // 17 SPEC-019 shape seeds in each of the last two.
    let mut seen = [0usize; 3];
    for (slot, target) in ["tiff_ifd", "jpeg_segments", "audit"].iter().enumerate() {
        let dir = seed_dir(target);
        for entry in std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
            let path = entry.expect("entry").path();
            let data = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let a = audit(&data);
            // The answer is internally consistent.
            assert!(a.items().len() <= MAX_INVENTORY_ITEMS, "{}", path.display());
            for f in a.findings() {
                if let Some(i) = f.item {
                    assert_eq!(a.items().get(i).copied(), Some(f.at), "{}", path.display());
                }
            }
            if matches!(a.kind(), ContainerKind::Unsupported(_)) {
                assert!(
                    !a.health().clean && a.items().is_empty(),
                    "{}",
                    path.display()
                );
            }
            seen[slot] += 1;
        }
    }
    assert_eq!(seen, [32, 37 + 17, 78 + 17]);
}

// ── SPEC-013, carried from the SPEC-012 verify (HANDOFF-024 F2/F3/F4) ───────

use exif_core::audit::Category;

/// A JPEG: SOI, the `before_scan` segments, a start-of-scan header with two
/// scan bytes, the `after_scan` segments, EOI.
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

fn named_tiff(name: &str) -> Vec<u8> {
    oracle_tiff::fixtures()
        .into_iter()
        .find(|(n, _)| *n == name)
        .map(|(_, b)| b)
        .unwrap_or_else(|| panic!("no TIFF fixture named {name}"))
}

/// F2 / M20. `Health::exif_issues` is summed over every readable block. The
/// fixture `ifd1-corrupt` carries TIFF-layer issues; `exif-basic-le` carries
/// none. Mutant M20 (`exif_issues` = the last block's count) reads 0 when the
/// clean block is last.
#[test]
fn health_exif_issues_is_the_sum_over_all_blocks() {
    let bad = named_tiff("ifd1-corrupt");
    let good = named_tiff("exif-basic-le");
    let bad_jpeg = jpeg_of(&[oracle_jpeg::exif_app1(&bad)], &[]);
    let good_jpeg = jpeg_of(&[oracle_jpeg::exif_app1(&good)], &[]);
    let bad_issues = audit(&bad_jpeg).health().exif_issues;
    assert!(bad_issues > 0, "ifd1-corrupt must carry TIFF issues");
    assert_eq!(audit(&good_jpeg).health().exif_issues, 0);

    // The issue-carrying block first and a clean block last (the shape M20
    // hides), the reverse, the bad block twice, and bad/good alternating.
    let cases: [(&str, Vec<&Vec<u8>>, usize); 4] = [
        ("bad then good", vec![&bad, &good], bad_issues),
        ("good then bad", vec![&good, &bad], bad_issues),
        ("bad then bad", vec![&bad, &bad], 2 * bad_issues),
        (
            "bad, good, bad, good",
            vec![&bad, &good, &bad, &good],
            2 * bad_issues,
        ),
    ];
    for (label, blocks, want) in cases {
        let segs: Vec<Vec<u8>> = blocks.iter().map(|b| oracle_jpeg::exif_app1(b)).collect();
        let bytes = jpeg_of(&segs, &[]);
        let a = audit(&bytes);
        assert_eq!(a.health().exif_issues, want, "{label}: summed over blocks");
        assert!(!a.health().clean, "{label}: issues mean not clean");
        assert_eq!(a.health().failed_blocks, 0, "{label}");
    }
}

/// F3 / M21. A metadata region after the scan carries `after_scan` on its
/// inventory item and on its finding; the same region before the scan does
/// not. Mutant M21 (region `after_scan` always false) fails the first half.
#[test]
fn a_region_after_the_scan_has_after_scan_origin_on_item_and_finding() {
    let xmp = oracle_jpeg::seg(
        0xE1,
        &[
            &b"http://ns.adobe.com/xap/1.0/\0"[..],
            &oracle_jpeg::xmp_packet(),
        ]
        .concat(),
    );
    let after_bytes = jpeg_of(&[], std::slice::from_ref(&xmp));
    let before_bytes = jpeg_of(std::slice::from_ref(&xmp), &[]);
    let after = audit(&after_bytes);
    let before = audit(&before_bytes);
    for (label, a, want) in [("after", &after, true), ("before", &before, false)] {
        assert_eq!(a.items().len(), 1, "{label}: one region item");
        assert!(
            matches!(a.items()[0].kind(), ItemKind::Region { .. }),
            "{label}"
        );
        assert_eq!(a.items()[0].origin().after_scan, want, "{label}: item");
        let f: Vec<_> = a.findings_in(Category::Xmp).collect();
        assert_eq!(f.len(), 1, "{label}: one XMP finding");
        assert_eq!(f[0].at.origin().after_scan, want, "{label}: finding");
        assert_eq!(f[0].item, Some(0), "{label}");
    }
}

/// F4 / M22. The thumbnail pointer needs `SHORT` or `LONG`. `BYTE` and
/// `SSHORT` values are decodable by the reader (so only the type gate keeps
/// them out); mutant M22 (gate removed) gives `Some` for them.
#[test]
fn thumbnail_pointer_needs_short_or_long_types() {
    use oracle_tiff as t;
    // IFD0 chained to an IFD1 holding 0x0201 (offset 0) and 0x0202 (length 8)
    // of the given encodings; the range lies inside the blob.
    let build = |offset: t::Ent, length: t::Ent| {
        t::assemble(
            false,
            &[
                t::ifd_next(vec![t::ascii(t::T_MAKE, b"Thumb\0")], 1),
                t::ifd(vec![offset, length]),
            ],
        )
    };
    let range_of = |tiff: &[u8]| {
        let a = audit(tiff);
        let f: Vec<_> = a.findings_in(Category::Thumbnail).collect();
        assert_eq!(f.len(), 1, "exactly the 0x0201 finding");
        let ItemKind::Tag { tag, .. } = f[0].at.kind() else {
            panic!("a tag item");
        };
        assert_eq!(tag, 0x0201);
        f[0].thumbnail
            .expect("a thumbnail finding carries a record")
            .range
    };
    type Case = (&'static str, t::Ent, t::Ent, Option<(u32, u32)>);
    let cases: [Case; 6] = [
        (
            "LONG",
            t::long(false, 0x0201, 0),
            t::long(false, 0x0202, 8),
            Some((0, 8)),
        ),
        (
            "SHORT",
            t::short(false, 0x0201, 0),
            t::short(false, 0x0202, 8),
            Some((0, 8)),
        ),
        ("BYTE", t::byte(0x0201, 0), t::byte(0x0202, 8), None),
        (
            "SSHORT",
            t::sshort(false, 0x0201, 0),
            t::sshort(false, 0x0202, 8),
            None,
        ),
        // One side wrong is enough.
        (
            "LONG offset, BYTE length",
            t::long(false, 0x0201, 0),
            t::byte(0x0202, 8),
            None,
        ),
        (
            "SSHORT offset, LONG length",
            t::sshort(false, 0x0201, 0),
            t::long(false, 0x0202, 8),
            None,
        ),
    ];
    for (label, offset, length, want) in cases {
        assert_eq!(range_of(&build(offset, length)), want, "{label}");
    }
}
