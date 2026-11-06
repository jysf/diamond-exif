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
    // 29 TIFF + 37 JPEG + the audit directory (66 copies + 7 new shapes).
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
    assert_eq!(seen, [29, 37, 73]);
}
