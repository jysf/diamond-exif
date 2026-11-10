//! SPEC-020: the JPEG strip writer against the independent model in
//! `tests/support/strip_shapes.rs`. Every fixture is run under every policy; the
//! expected output is computed from the builder's own record of what each byte
//! range is, never from the library.

#[path = "support/oracle_tiff.rs"]
mod oracle_tiff;
#[path = "support/real_shapes.rs"]
mod real_shapes;
#[path = "support/strip_shapes.rs"]
mod strip_shapes;

use std::ops::Range;

use exif_core::audit::{audit, Category, ContainerKind, ItemKind, RegionKind, Sniffed};
use exif_core::policy::{StripPolicy, StripSpec};
use exif_core::write::{Check, RemovalKind, UnstrippableReason};
use exif_core::{sanitize, strip, StripError, Stripped};
use strip_shapes::{expected_bytes, model, spec_of, Expect, Shape, EXIF9, K, REMOVED_CANARIES};

// ── policies ────────────────────────────────────────────────────────────────

fn policies() -> Vec<(&'static str, StripPolicy)> {
    vec![
        ("gps", StripPolicy::Gps),
        ("gps_and_device", StripPolicy::GpsAndDevice),
        ("all_pii", StripPolicy::AllPii),
        ("all", StripPolicy::All),
        ("everything", StripPolicy::Everything),
        ("none", StripPolicy::Custom(StripSpec::none())),
        ("xmp", StripPolicy::Custom(spec_of(&[Category::Xmp]))),
        ("icc", StripPolicy::Custom(spec_of(&[Category::Icc]))),
        ("jumbf", StripPolicy::Custom(spec_of(&[Category::Jumbf]))),
        (
            "free_text",
            StripPolicy::Custom(spec_of(&[Category::FreeText])),
        ),
        (
            "embedded",
            StripPolicy::Custom(spec_of(&[Category::EmbeddedImage])),
        ),
        (
            "other",
            StripPolicy::Custom(spec_of(&[Category::OtherSegment])),
        ),
        (
            "iptc+free_text",
            StripPolicy::Custom(spec_of(&[Category::Iptc, Category::FreeText])),
        ),
    ]
}

fn policy(name: &str) -> StripPolicy {
    policies().into_iter().find(|(n, _)| *n == name).unwrap().1
}

fn shape(name: &str) -> Shape {
    strip_shapes::all()
        .into_iter()
        .find(|s| s.name == name)
        .unwrap()
}

fn kind_of(k: K) -> RemovalKind {
    match k {
        K::Exif => RemovalKind::Exif,
        K::Xmp => RemovalKind::Xmp,
        K::ExtXmp => RemovalKind::ExtendedXmp,
        K::Icc => RemovalKind::Icc,
        K::Iptc => RemovalKind::Iptc,
        K::Jumbf => RemovalKind::Jumbf,
        K::Com => RemovalKind::Comment,
        K::OtherApp => RemovalKind::OtherApp,
        K::Trailer => RemovalKind::Trailer,
        K::Image => RemovalKind::EmbeddedImage,
        K::MpfIndex => RemovalKind::MpfIndex,
        other => panic!("not a removable part: {other:?}"),
    }
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

/// The model's expectation for `shape` under the policy named `name`.
fn expect(s: &Shape, name: &str) -> Expect {
    model(s, policy(name).spec())
}

fn removal_ranges(e: &Expect) -> Vec<Range<usize>> {
    match e {
        Expect::Ok { removals, .. } => removals.iter().map(|r| r.range.clone()).collect(),
        _ => panic!("not an Ok expectation: {e:?}"),
    }
}

// ── dispatch and refusals ───────────────────────────────────────────────────

#[test]
fn tiff_png_webp_heic_are_unsupported_naming_the_container() {
    let tiff = oracle_tiff::assemble(
        false,
        &[oracle_tiff::ifd(vec![oracle_tiff::short(false, 274, 1)])],
    );
    assert_eq!(
        strip(&tiff, StripPolicy::All).unwrap_err(),
        StripError::Unsupported {
            kind: ContainerKind::Tiff
        }
    );
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    png.extend_from_slice(&[0; 16]);
    let mut webp = b"RIFF\x10\0\0\0WEBPVP8 ".to_vec();
    webp.extend_from_slice(&[0; 16]);
    let mut heic = vec![0, 0, 0, 0x18];
    heic.extend_from_slice(b"ftypheic");
    heic.extend_from_slice(&[0; 16]);
    for (bytes, want) in [
        (png, Sniffed::Png),
        (webp, Sniffed::Webp),
        (heic, Sniffed::Heic),
    ] {
        assert_eq!(
            strip(&bytes, StripPolicy::Everything).unwrap_err(),
            StripError::Unsupported {
                kind: ContainerKind::Unsupported(want)
            }
        );
        assert_eq!(
            sanitize(&bytes, StripPolicy::Everything).unwrap_err(),
            StripError::Unsupported {
                kind: ContainerKind::Unsupported(want)
            }
        );
    }
}

#[test]
fn empty_one_byte_and_non_jpeg_inputs() {
    for bytes in [
        &b""[..],
        &[0xFF][..],
        &[0xD8, 0xFF][..],
        b"hello world, not an image",
    ] {
        assert_eq!(
            strip(bytes, StripPolicy::All).unwrap_err(),
            StripError::NotJpeg
        );
    }
    // The shortest JPEG is fine and unchanged.
    let out = strip(&[0xFF, 0xD8], StripPolicy::Everything).unwrap();
    assert_eq!(out.bytes(), &[0xFF, 0xD8]);
    assert!(out.removals().is_empty() && out.patches().is_empty());
}

#[test]
fn bad_segment_length_is_unstrippable() {
    let s = shape("bad-length");
    let at = s
        .parts
        .iter()
        .find(|p| p.kind == K::Stray)
        .unwrap()
        .range
        .start;
    for (name, p) in policies() {
        assert_eq!(
            strip(&s.bytes, p).unwrap_err(),
            StripError::Unstrippable {
                reason: UnstrippableReason::BadSegmentLength,
                offset: at
            },
            "{name}"
        );
    }
}

#[test]
fn halted_walk_is_unstrippable() {
    let s = shape("halted-walk");
    for (name, p) in policies() {
        match strip(&s.bytes, p) {
            Err(StripError::Unstrippable {
                reason: UnstrippableReason::TooManySegments,
                ..
            }) => {}
            other => panic!("{name}: {other:?}"),
        }
    }
}

#[test]
fn missing_eoi_and_missing_sos_are_still_strippable() {
    for name in ["missing-eoi", "missing-sos"] {
        let s = shape(name);
        let out = strip(&s.bytes, StripPolicy::All).unwrap();
        let want = expected_bytes(&s.bytes, &removal_ranges(&expect(&s, "all")), &[]);
        assert_eq!(out.bytes(), &want[..], "{name}");
        assert!(out.bytes().len() < s.bytes.len(), "{name}");
        assert!(!contains(out.bytes(), strip_shapes::C_XMP), "{name}");
        assert!(!contains(out.bytes(), strip_shapes::C_COM1), "{name}");
    }
}

// ── presets ─────────────────────────────────────────────────────────────────

fn parts_kept(s: &Shape, out: &[u8], kind: K) {
    for r in s.parts_of(kind) {
        assert!(contains(out, &s.bytes[r]), "{kind:?} part should remain");
    }
}

#[test]
fn all_removes_exif_xmp_extended_xmp_iptc_and_comments_and_keeps_icc_jumbf_other_embedded_and_trailer(
) {
    let s = shape("kitchen-sink");
    let out = strip(&s.bytes, StripPolicy::All).unwrap();
    let want_removed: Vec<Range<usize>> = s
        .parts
        .iter()
        .filter(|p| matches!(p.kind, K::Exif | K::Xmp | K::ExtXmp | K::Iptc | K::Com))
        .map(|p| p.range.clone())
        .collect();
    assert_eq!(
        out.bytes(),
        &expected_bytes(&s.bytes, &want_removed, &[])[..]
    );
    for k in [
        K::Icc,
        K::Jumbf,
        K::OtherApp,
        K::Trailer,
        K::Jfif,
        K::Adobe,
        K::Dqt,
        K::Sof,
        K::Dht,
        K::Dri,
        K::Scan,
    ] {
        parts_kept(&s, out.bytes(), k);
    }
    // 1 Exif, 1 XMP, 2 extended XMP, 2 IPTC, 2 COM.
    assert_eq!(out.removals().len(), 8);
    assert!(out.patches().is_empty());
    // Embedded images are kept too, on a Multi-Picture file.
    let m = shape("mpo-plain");
    let out = strip(&m.bytes, StripPolicy::All).unwrap();
    for r in m.parts_of(K::Image) {
        assert!(contains(out.bytes(), &m.bytes[r]));
    }
}

#[test]
fn everything_keeps_only_structure_and_the_scan() {
    let s = shape("kitchen-sink");
    let out = strip(&s.bytes, StripPolicy::Everything).unwrap();
    let want: Vec<u8> = s
        .parts
        .iter()
        .filter(|p| {
            matches!(
                p.kind,
                K::Soi | K::Jfif | K::Adobe | K::Dqt | K::Sof | K::Dht | K::Dri | K::Scan
            )
        })
        .flat_map(|p| s.bytes[p.range.clone()].to_vec())
        .collect();
    assert_eq!(out.bytes(), &want[..]);
    assert!(audit(out.bytes()).findings().is_empty());
}

/// Whether the policy removes every Exif category (so the Exif tags go).
fn exif_gone(p: StripPolicy) -> bool {
    EXIF9.iter().all(|c| p.spec().contains(*c))
}

#[test]
fn the_preview_dispositions_match_what_strip_did_for_every_preset_and_fixture() {
    use exif_core::audit::{preview, Disposition};
    let mut compared = 0;
    for s in strip_shapes::all() {
        for (name, p) in policies().into_iter().take(5) {
            let before = audit(&s.bytes);
            let pv = preview(&before, p);
            let Ok(out) = strip(&s.bytes, p) else {
                continue;
            };
            let after = audit(out.bytes());
            // Everything the preview says is removed is gone; everything it
            // keeps (or says it did not open) is still there.
            for finding in before.findings() {
                let removed = p.spec().contains(finding.category);
                let still = after
                    .findings()
                    .iter()
                    .any(|g| g.category == finding.category);
                if removed {
                    assert!(!still, "{} {name}: {:?} left", s.name, finding.category);
                } else if !(exif_gone(p) && finding.at.origin().block.is_some()) {
                    assert!(still, "{} {name}: {:?} lost", s.name, finding.category);
                }
                compared += 1;
            }
            // The preview agrees with the policy on which findings go.
            for (f, d) in before.findings().iter().zip(pv.dispositions()) {
                let gone = matches!(d, Disposition::Removed);
                assert_eq!(gone, p.spec().contains(f.category), "{} {name}", s.name);
            }
        }
    }
    assert!(compared > 100, "{compared}");
}

#[test]
fn none_policy_is_the_identity() {
    for s in strip_shapes::all() {
        let r = strip(&s.bytes, policy("none"));
        if s.refused {
            assert!(r.is_err(), "{}", s.name);
            continue;
        }
        let out = r.unwrap_or_else(|e| panic!("{}: {e}", s.name));
        assert_eq!(out.bytes(), &s.bytes[..], "{}", s.name);
        assert!(
            out.removals().is_empty() && out.patches().is_empty(),
            "{}",
            s.name
        );
    }
}

// ── custom sets and segment rules ──────────────────────────────────────────

fn removed_kinds(s: &Shape, name: &str) -> Vec<K> {
    match expect(s, name) {
        Expect::Ok { removals, .. } => removals.iter().map(|r| r.kind).collect(),
        other => panic!("{other:?}"),
    }
}

fn check_ok(s: &Shape, name: &str) -> Stripped {
    let out = strip(&s.bytes, policy(name)).unwrap_or_else(|e| panic!("{} {name}: {e}", s.name));
    let e = expect(s, name);
    let Expect::Ok { removals, patches } = &e else {
        panic!("{} {name}: model says {e:?}", s.name)
    };
    let ranges: Vec<Range<usize>> = removals.iter().map(|r| r.range.clone()).collect();
    assert_eq!(
        out.bytes(),
        &expected_bytes(&s.bytes, &ranges, patches)[..],
        "{} {name}",
        s.name
    );
    out
}

#[test]
fn custom_xmp_removes_xmp_and_extended_xmp_only() {
    let s = shape("kitchen-sink");
    check_ok(&s, "xmp");
    let mut kinds = removed_kinds(&s, "xmp");
    kinds.sort_by_key(|k| format!("{k:?}"));
    assert_eq!(kinds, [K::ExtXmp, K::ExtXmp, K::Xmp]);
}

#[test]
fn custom_icc_removes_every_icc_chunk() {
    let s = shape("kitchen-sink");
    let out = check_ok(&s, "icc");
    assert_eq!(removed_kinds(&s, "icc"), [K::Icc, K::Icc]);
    assert!(!contains(out.bytes(), strip_shapes::C_ICC));
    assert!(contains(out.bytes(), strip_shapes::C_XMP));
}

#[test]
fn custom_iptc_removes_both_split_app13_segments() {
    let s = shape("kitchen-sink");
    // `FreeText` is also asked for, so the kitchen sink (whose Exif has free
    // text) would need the entry edit: use the file without Exif.
    let s2 = shape("no-exif");
    let out = check_ok(&s2, "iptc+free_text");
    assert!(!contains(out.bytes(), strip_shapes::C_IPTC1));
    assert!(!contains(out.bytes(), strip_shapes::C_IPTC2));
    assert!(!contains(out.bytes(), strip_shapes::C_COM1));
    assert_eq!(
        out.removals()
            .iter()
            .filter(|r| r.kind == RemovalKind::Iptc)
            .count(),
        2
    );
    assert_eq!(
        out.removals()
            .iter()
            .filter(|r| r.kind == RemovalKind::Comment)
            .count(),
        2
    );
    assert!(matches!(
        strip(&s.bytes, policy("iptc+free_text")),
        Err(StripError::NeedsEntryEdit { .. })
    ));
}

#[test]
fn custom_jumbf_removes_jumbf() {
    let s = shape("kitchen-sink");
    let out = check_ok(&s, "jumbf");
    assert!(!contains(out.bytes(), strip_shapes::C_JUMBF));
    assert_eq!(out.removals().len(), 1);
}

#[test]
fn custom_free_text_removes_comments_when_exif_has_no_free_text() {
    // Orientation-only Exif has no FreeText: only the comment goes.
    let s = shape("exif-orientation-only");
    let out = check_ok(&s, "free_text");
    assert!(!contains(out.bytes(), strip_shapes::C_COM1));
    assert_eq!(out.removals().len(), 1);
    // A file with no Exif at all.
    let s = shape("comment-only");
    let out = check_ok(&s, "free_text");
    assert_eq!(out.removals().len(), 1);
}

#[test]
fn custom_free_text_needs_entry_edit_when_exif_has_free_text() {
    let s = shape("kitchen-sink");
    assert_eq!(
        strip(&s.bytes, policy("free_text")).unwrap_err(),
        StripError::NeedsEntryEdit {
            categories: spec_of(&[Category::FreeText])
        }
    );
}

#[test]
fn custom_other_segment_removes_app_segments_and_trailers_but_not_embedded_images() {
    let s = shape("mpo-trailer-gaps");
    let out = check_ok(&s, "other");
    let kinds = removed_kinds(&s, "other");
    assert!(kinds.contains(&K::OtherApp) && kinds.contains(&K::Trailer));
    assert!(!kinds.contains(&K::Image));
    // The images are still there (patched offsets), the AROT segment is not.
    assert!(!contains(out.bytes(), strip_shapes::C_APP10));
    for r in s.parts_of(K::Image) {
        assert!(contains(out.bytes(), &s.bytes[r]));
    }
    assert!(!out.patches().is_empty());
}

// ── the Exif rule ──────────────────────────────────────────────────────────

#[test]
fn exif_policies_work_on_a_file_without_exif() {
    let s = shape("no-exif");
    for name in ["gps", "gps_and_device", "all_pii", "all", "everything"] {
        let out = check_ok(&s, name);
        // The Exif rule never refuses a file with no Exif block.
        assert!(
            !out.removals().iter().any(|r| r.kind == RemovalKind::Exif),
            "{name}"
        );
    }
}

#[test]
fn gps_preset_is_the_identity_on_exif_without_findings_in_its_categories() {
    for n in ["exif-orientation-only", "exif-without-gps"] {
        let s = shape(n);
        let out = strip(&s.bytes, StripPolicy::Gps).unwrap();
        assert_eq!(out.bytes(), &s.bytes[..], "{n}");
    }
    // GpsAndDevice: exif-without-gps has no DeviceId either.
    let s = shape("exif-without-gps");
    assert_eq!(
        strip(&s.bytes, StripPolicy::GpsAndDevice).unwrap().bytes(),
        &s.bytes[..]
    );
}

#[test]
fn gps_preset_needs_entry_edit_when_exif_has_gps() {
    let s = shape("kitchen-sink");
    assert_eq!(
        strip(&s.bytes, StripPolicy::Gps).unwrap_err(),
        StripError::NeedsEntryEdit {
            categories: spec_of(&[Category::Gps])
        }
    );
    assert_eq!(
        strip(&s.bytes, StripPolicy::GpsAndDevice).unwrap_err(),
        StripError::NeedsEntryEdit {
            categories: spec_of(&[Category::Gps, Category::DeviceId])
        }
    );
}

#[test]
fn all_pii_needs_entry_edit_and_names_its_categories() {
    let s = shape("kitchen-sink");
    // AllPii is EXIF9 minus Software: the categories reported are exactly
    // S intersect EXIF9, not all of S (mutant 29 reports more) and not EXIF9.
    let want = spec_of(&[
        Category::Gps,
        Category::DeviceId,
        Category::Authorship,
        Category::FreeText,
        Category::Timestamps,
        Category::CameraModel,
        Category::Thumbnail,
        Category::MakerNote,
    ]);
    assert_eq!(
        strip(&s.bytes, StripPolicy::AllPii).unwrap_err(),
        StripError::NeedsEntryEdit { categories: want }
    );
    // A custom set with Xmp too: the Xmp category is not an Exif category.
    let custom = StripPolicy::Custom(spec_of(&[Category::Gps, Category::Xmp, Category::Icc]));
    assert_eq!(
        strip(&s.bytes, custom).unwrap_err(),
        StripError::NeedsEntryEdit {
            categories: spec_of(&[Category::Gps])
        }
    );
}

#[test]
fn an_unreadable_exif_block_needs_entry_edit_for_partial_policies() {
    let s = shape("unreadable-exif");
    assert!(matches!(
        strip(&s.bytes, StripPolicy::Gps),
        Err(StripError::NeedsEntryEdit { .. })
    ));
    // A policy with no Exif category does not care.
    let out = strip(&s.bytes, policy("xmp")).unwrap();
    assert!(!contains(out.bytes(), strip_shapes::C_XMP));
    // Removing every Exif category removes the unreadable block too.
    let out = strip(&s.bytes, StripPolicy::All).unwrap();
    assert!(out.removals().iter().any(|r| r.kind == RemovalKind::Exif));
}

#[test]
fn all_removes_every_exif_block_including_the_extra_ones() {
    let s = shape("two-exif-blocks");
    let out = check_ok(&s, "all");
    let exif = out
        .removals()
        .iter()
        .filter(|r| r.kind == RemovalKind::Exif)
        .count();
    assert_eq!(exif, 2);
    assert!(audit(out.bytes()).findings().is_empty());
}

#[test]
fn orientation_goes_with_the_exif_segment_under_all() {
    // DEC-013 part 4: the accepted consequence. No Orientation survives.
    let s = shape("exif-orientation-only");
    let out = strip(&s.bytes, StripPolicy::All).unwrap();
    assert!(out.removals().iter().any(|r| r.kind == RemovalKind::Exif));
    assert!(!contains(out.bytes(), b"Exif\0"));
    // Removal records: the Exif segment is recorded under `Thumbnail` when the
    // audit found nothing in the block (pinned), under the first finding else.
    let r = out
        .removals()
        .iter()
        .find(|r| r.kind == RemovalKind::Exif)
        .unwrap();
    assert_eq!(r.category, Category::Thumbnail);
    let k = shape("kitchen-sink");
    let out = strip(&k.bytes, StripPolicy::All).unwrap();
    let r = out
        .removals()
        .iter()
        .find(|r| r.kind == RemovalKind::Exif)
        .unwrap();
    assert_eq!(r.category, Category::CameraModel);
}

// ── Multi-Picture ──────────────────────────────────────────────────────────

#[test]
fn everything_removes_the_images_and_the_index_together() {
    for n in [
        "mpo-late-xmp-le",
        "mpo-late-xmp-be",
        "mpo-trailer-gaps",
        "mpo-plain",
    ] {
        let s = shape(n);
        let out = check_ok(&s, "everything");
        assert!(out.patches().is_empty(), "{n}");
        let a = audit(out.bytes());
        assert!(a.findings().is_empty(), "{n}");
        assert!(out
            .removals()
            .iter()
            .any(|r| r.kind == RemovalKind::MpfIndex && r.category == Category::EmbeddedImage));
        assert_eq!(
            out.removals()
                .iter()
                .filter(|r| r.kind == RemovalKind::EmbeddedImage)
                .count(),
            2
        );
    }
}

#[test]
fn all_keeps_images_and_index_and_needs_no_patch_when_removals_precede_the_index() {
    let s = shape("mpo-plain");
    let out = check_ok(&s, "all");
    // The Exif goes (it is before the index): index offsets are relative to
    // the index header, so the secondary offsets do not change; only the
    // primary's size field does.
    assert!(out.removals().iter().any(|r| r.kind == RemovalKind::Exif));
    let m = s.mpf.as_ref().unwrap();
    let exif_len = s.parts_of(K::Exif)[0].len();
    assert_eq!(out.patches().len(), 1);
    // The size field of entry 0 sits in the shifted index.
    assert_eq!(out.patches()[0].offset, m.fields[0].0 - exif_len);
    assert_eq!(out.patches()[0].len, 4);
    let a = audit(out.bytes());
    assert_eq!(a.findings_in(Category::EmbeddedImage).count(), 2);
    assert_eq!(a.health().jpeg_issues, 0);
}

#[test]
fn removal_after_the_index_patches_secondary_offsets_and_primary_size() {
    for be in [false, true] {
        let s = strip_shapes::mpo_late_xmp(be);
        let out = check_ok(&s, "all");
        // Entry 0 size, plus both secondary offsets (each lies after the XMP
        // and the COM that were removed after the index).
        assert_eq!(out.patches().len(), 3, "be={be}");
        let a = audit(out.bytes());
        assert_eq!(a.findings_in(Category::EmbeddedImage).count(), 2, "be={be}");
        assert_eq!(a.health().jpeg_issues, 0, "be={be}");
        // The patched fields read back in the file's own byte order.
        let m = s.mpf.as_ref().unwrap();
        let exif_len = s.parts_of(K::Exif)[0].len();
        let late = s.parts_of(K::Xmp)[0].len() + s.parts_of(K::Com)[0].len();
        let word = |b: &[u8], at: usize| {
            let w: [u8; 4] = b[at..at + 4].try_into().unwrap();
            if be {
                u32::from_be_bytes(w)
            } else {
                u32::from_le_bytes(w)
            }
        };
        let size_at = m.fields[0].0 - exif_len;
        assert_eq!(
            word(out.bytes(), size_at) as usize,
            m.primary_end - late - exif_len,
            "be={be}"
        );
        let off_at = m.fields[1].1 - exif_len;
        assert_eq!(
            word(out.bytes(), off_at),
            m.entries[1].1 - late as u32,
            "be={be}"
        );
        // The second image sits right behind the first: same shift.
        let off2_at = m.fields[2].1 - exif_len;
        assert_eq!(
            word(out.bytes(), off2_at),
            m.entries[2].1 - late as u32,
            "be={be}"
        );
    }
}

#[test]
fn removing_trailer_gaps_patches_later_image_offsets() {
    let s = shape("mpo-trailer-gaps");
    let out = check_ok(&s, "other");
    let gaps: Vec<_> = s.parts_of(K::Trailer).iter().map(|r| r.len()).collect();
    assert_eq!(gaps, [11, 7, 19]);
    let m = s.mpf.as_ref().unwrap();
    let arot = s.parts_of(K::OtherApp)[0].len();
    let xmp_free = out.removals().iter().all(|r| r.kind != RemovalKind::Xmp);
    assert!(xmp_free);
    // AROT precedes the index (the offsets do not feel it, entry 0's size
    // does); image 1 sits behind the first gap, image 2 behind two gaps.
    let word = |at: usize| u32::from_le_bytes(out.bytes()[at..at + 4].try_into().unwrap());
    let shifted = |field: usize| field - arot;
    assert_eq!(word(shifted(m.fields[0].0)), m.entries[0].0 - arot as u32);
    assert_eq!(word(shifted(m.fields[1].1)), m.entries[1].1 - 11);
    assert_eq!(word(shifted(m.fields[2].1)), m.entries[2].1 - 11 - 7);
    let a = audit(out.bytes());
    assert_eq!(a.findings_in(Category::EmbeddedImage).count(), 2);
    assert!(a.findings_in(Category::OtherSegment).next().is_none());
    assert_eq!(out.patches().len(), 3);
}

#[test]
fn custom_embedded_image_removes_images_and_index_and_keeps_trailer_pieces() {
    let s = shape("mpo-trailer-gaps");
    let out = check_ok(&s, "embedded");
    let a = audit(out.bytes());
    assert_eq!(a.findings_in(Category::EmbeddedImage).count(), 0);
    // The gap bytes are one joined trailer region of the same total length.
    let total: usize = a
        .findings_in(Category::OtherSegment)
        .map(|f| match f.at.kind() {
            ItemKind::Region {
                kind: RegionKind::Trailer,
                len,
                ..
            } => len,
            _ => 0,
        })
        .sum();
    assert_eq!(total, 11 + 7 + 19);
    assert!(out.patches().is_empty());
}

#[test]
fn unindexed_mpf_with_a_later_removal_is_mpf_not_indexed() {
    let s = shape("unindexed-mpf");
    let at = s.parts_of(K::OtherApp)[0].start;
    // XMP comes after the unindexed MPF segment.
    assert_eq!(
        strip(&s.bytes, policy("xmp")).unwrap_err(),
        StripError::MpfNotIndexed { offset: at }
    );
    assert_eq!(
        strip(&s.bytes, StripPolicy::All).unwrap_err(),
        StripError::MpfNotIndexed { offset: at }
    );
    // Removing the unindexed segment itself leaves nothing wrong.
    assert!(strip(&s.bytes, StripPolicy::Everything).is_ok());
}

#[test]
fn unindexed_mpf_with_nothing_removed_after_it_is_fine() {
    let s = shape("unindexed-mpf-last");
    // The XMP is before the segment: nothing is removed at or after its end.
    let out = strip(&s.bytes, policy("xmp")).unwrap();
    assert!(!contains(out.bytes(), strip_shapes::C_XMP));
    assert!(contains(out.bytes(), b"MPF\0this is not"));
}

#[test]
fn a_second_mpf_segment_follows_the_same_rule() {
    let s = shape("second-mpf");
    let second = s.parts_of(K::OtherApp)[0].start;
    // A COM after the second MPF segment is removed under FreeText.
    assert_eq!(
        strip(&s.bytes, policy("free_text")).unwrap_err(),
        StripError::MpfNotIndexed { offset: second }
    );
    // Removing the index and images (a kept second MPF would become the index).
    assert_eq!(
        strip(&s.bytes, policy("embedded")).unwrap_err(),
        StripError::MpfNotIndexed { offset: second }
    );
    // Everything removes the second segment too.
    assert!(strip(&s.bytes, StripPolicy::Everything).is_ok());
    // No removal at all: fine.
    assert!(strip(&s.bytes, policy("none")).is_ok());
}

#[test]
fn malformed_entries_are_not_patched() {
    let s = shape("mpf-malformed-entry");
    let out = check_ok(&s, "free_text");
    let m = s.mpf.as_ref().unwrap();
    // The COM after the index shrinks entry 0 and the two good entries; the
    // malformed entry (2) keeps its stale value.
    assert_eq!(out.patches().len(), 3);
    let bad = m.fields[2].1;
    let word = |b: &[u8], at: usize| u32::from_le_bytes(b[at..at + 4].try_into().unwrap());
    assert_eq!(word(out.bytes(), bad), 0x00FF_0000);
}

#[test]
fn an_index_entry_that_is_no_image_is_not_shifted_under_it() {
    // Deviation pinned (see the Handback): an entry that points inside the
    // input at bytes that are not an image is never patched, so a removal
    // between the index and the byte it points at would silently re-aim it.
    let s = shape("mpf-entry-not-jpeg");
    let index = s.parts_of(K::MpfIndex)[0].start;
    assert_eq!(
        strip(&s.bytes, policy("free_text")).unwrap_err(),
        StripError::MpfNotIndexed { offset: index }
    );
    // Removing the very bytes it points at is the same refusal.
    assert_eq!(
        strip(&s.bytes, policy("other")).unwrap_err(),
        StripError::MpfNotIndexed { offset: index }
    );
    // With nothing removed in that window the file is returned as it was.
    assert_eq!(
        strip(&s.bytes, policy("none")).unwrap().bytes(),
        &s.bytes[..]
    );
}

#[test]
fn an_unlocated_index_entry_is_reported_before_a_second_mpf_segment() {
    // Found by the fuzzer: both refusals apply; the pinned order reports the
    // index's own entry (at the index segment) first.
    let s = shape("second-mpf-unlocated-entry");
    let index = s.parts_of(K::MpfIndex)[0].start;
    assert_eq!(
        strip(&s.bytes, policy("free_text")).unwrap_err(),
        StripError::MpfNotIndexed { offset: index }
    );
}

#[test]
fn a_comment_over_a_kept_embedded_image_is_overlap() {
    let s = shape("com-over-embedded");
    let image = s.parts_of(K::Image)[0].start;
    // The first byte that is both removed (the COM) and kept (the image) is
    // the image's first byte.
    assert_eq!(
        strip(&s.bytes, policy("free_text")).unwrap_err(),
        StripError::Overlap { offset: image }
    );
    // Removing the image alone (the COM kept) cuts the kept COM.
    assert_eq!(
        strip(&s.bytes, policy("embedded")).unwrap_err(),
        StripError::Overlap { offset: image }
    );
    // Both removed together: the ranges merge and nothing kept is cut.
    let both = StripPolicy::Custom(spec_of(&[Category::FreeText, Category::EmbeddedImage]));
    let out = strip(&s.bytes, both).unwrap();
    assert!(out.bytes().len() < s.bytes.len());
    assert!(!contains(out.bytes(), strip_shapes::C_COM1));
}

#[test]
fn overlapping_images_are_overlap_when_one_is_kept() {
    let s = shape("overlapping-images");
    let inner = s.parts_of(K::Image).iter().map(|r| r.start).min().unwrap();
    // Removing the images cuts the kept Adobe segment that holds one of them.
    assert_eq!(
        strip(&s.bytes, policy("embedded")).unwrap_err(),
        StripError::Overlap { offset: inner }
    );
    // With the images kept, a removal elsewhere is fine.
    assert!(strip(&s.bytes, policy("free_text")).is_ok());
}

// ── invariants over every fixture and policy ────────────────────────────────

type Run = (Shape, &'static str, Result<Stripped, StripError>);

fn all_runs() -> Vec<Run> {
    let mut v = Vec::new();
    for s in strip_shapes::all() {
        for (name, p) in policies() {
            let r = strip(&s.bytes, p);
            v.push((s.clone(), name, r));
        }
    }
    v
}

#[test]
fn output_equals_the_input_minus_the_listed_removals_plus_the_listed_patches() {
    let mut oks = 0;
    for (s, name, r) in all_runs() {
        let e = expect(&s, name);
        match (&e, &r) {
            (Expect::Ok { removals, patches }, Ok(out)) => {
                oks += 1;
                let ranges: Vec<Range<usize>> = removals.iter().map(|r| r.range.clone()).collect();
                assert_eq!(
                    out.bytes(),
                    &expected_bytes(&s.bytes, &ranges, patches)[..],
                    "{} {name}",
                    s.name
                );
                // The records: category, kind, input offset, length.
                assert_eq!(out.removals().len(), removals.len(), "{} {name}", s.name);
                for (got, want) in out.removals().iter().zip(removals) {
                    assert_eq!(got.kind, kind_of(want.kind), "{} {name}", s.name);
                    assert_eq!(got.category, want.category, "{} {name}", s.name);
                    assert_eq!(
                        (got.offset, got.len),
                        (want.range.start, want.range.len()),
                        "{} {name}",
                        s.name
                    );
                }
                // Patch records are in output coordinates.
                assert_eq!(out.patches().len(), patches.len(), "{} {name}", s.name);
                for (got, (at, bytes)) in out.patches().iter().zip(patches) {
                    let mut gone = vec![false; s.bytes.len()];
                    for r in &ranges {
                        for g in &mut gone[r.clone()] {
                            *g = true;
                        }
                    }
                    let before = gone[..*at].iter().filter(|g| **g).count();
                    assert_eq!(got.offset, at - before, "{} {name}", s.name);
                    assert_eq!(got.len, bytes.len(), "{} {name}", s.name);
                }
            }
            (Expect::Unstrippable, Err(StripError::Unstrippable { .. })) => {}
            (Expect::NeedsEntryEdit(c), Err(StripError::NeedsEntryEdit { categories })) => {
                assert_eq!(c, categories, "{} {name}", s.name)
            }
            (Expect::MpfNotIndexed(o), Err(StripError::MpfNotIndexed { offset })) => {
                assert_eq!(o, offset, "{} {name}", s.name)
            }
            (Expect::Overlap(o), Err(StripError::Overlap { offset })) => {
                assert_eq!(o, offset, "{} {name}", s.name)
            }
            (e, r) => panic!(
                "{} {name}: model {e:?}, library {:?}",
                s.name,
                r.as_ref().map(|o| o.removals().len())
            ),
        }
    }
    assert!(oks > 100, "the grid should be mostly Ok ({oks})");
}

#[test]
fn scan_and_kept_segments_are_byte_identical() {
    for (s, name, r) in all_runs() {
        let Ok(out) = r else { continue };
        // Every kept structural part appears, in order.
        let mut from = 0usize;
        for p in &s.parts {
            if matches!(
                p.kind,
                K::Soi | K::Jfif | K::Adobe | K::Dqt | K::Sof | K::Dht | K::Dri | K::Scan
            ) {
                let hay = &out.bytes()[from..];
                let pos = hay
                    .windows(p.range.len())
                    .position(|w| w == &s.bytes[p.range.clone()])
                    .unwrap_or_else(|| {
                        panic!("{} {name}: {:?} missing or out of order", s.name, p.kind)
                    });
                from += pos + p.range.len();
            }
        }
    }
}

#[test]
fn audit_of_the_output_has_no_category_the_policy_removes() {
    for (s, name, r) in all_runs() {
        let Ok(out) = r else { continue };
        let a = audit(out.bytes());
        let spec = policy(name).spec();
        assert!(
            a.findings().iter().all(|f| !spec.contains(f.category)),
            "{} {name}",
            s.name
        );
    }
}

#[test]
fn kept_findings_survive_by_category_and_kind() {
    for (s, name, r) in all_runs() {
        let Ok(out) = r else { continue };
        let spec = policy(name).spec();
        let key = |a: &exif_core::audit::FileAudit<'_>, skip_exif: bool| {
            let mut v: Vec<String> = a
                .findings()
                .iter()
                .filter(|f| !spec.contains(f.category))
                .filter(|f| !(skip_exif && f.at.origin().block.is_some()))
                .filter(|f| {
                    !matches!(
                        f.at.kind(),
                        ItemKind::Region {
                            kind: RegionKind::Trailer,
                            ..
                        }
                    )
                })
                .map(|f| {
                    format!(
                        "{:?}/{}",
                        f.category,
                        match f.at.kind() {
                            ItemKind::Tag { ifd, tag, .. } => format!("{ifd:?}{tag}"),
                            ItemKind::Region { kind, .. } => format!("{kind:?}"),
                        }
                    )
                })
                .collect();
            v.sort();
            v
        };
        let gone = EXIF9.iter().all(|c| spec.contains(*c));
        assert_eq!(
            key(&audit(&s.bytes), gone),
            key(&audit(out.bytes()), false),
            "{} {name}",
            s.name
        );
    }
}

#[test]
fn strip_is_idempotent_and_deterministic() {
    for (s, name, r) in all_runs() {
        let Ok(out) = r else { continue };
        let again = strip(&s.bytes, policy(name)).unwrap();
        assert_eq!(again, out, "{} {name}: deterministic", s.name);
        let twice = strip(out.bytes(), policy(name)).unwrap();
        assert_eq!(twice.bytes(), out.bytes(), "{} {name}: idempotent", s.name);
        assert!(
            twice.removals().is_empty() && twice.patches().is_empty(),
            "{} {name}",
            s.name
        );
    }
}

#[test]
fn output_is_never_longer_and_equal_only_for_an_empty_plan() {
    for (s, name, r) in all_runs() {
        let Ok(out) = r else { continue };
        assert!(out.bytes().len() <= s.bytes.len(), "{} {name}", s.name);
        let removed: usize = out.removals().iter().map(|r| r.len).sum();
        // Removal records may nest (an image inside a COM): the bytes removed
        // are at most their sum.
        if out.removals().is_empty() {
            assert_eq!(out.bytes().len(), s.bytes.len(), "{} {name}", s.name);
            assert!(out.patches().is_empty());
        } else {
            assert!(out.bytes().len() < s.bytes.len(), "{} {name}", s.name);
            assert!(
                s.bytes.len() - out.bytes().len() <= removed,
                "{} {name}",
                s.name
            );
        }
    }
}

#[test]
fn removed_canaries_are_gone_and_kept_canaries_remain() {
    let s = shape("kitchen-sink");
    for name in ["all", "everything"] {
        let out = strip(&s.bytes, policy(name)).unwrap();
        for c in REMOVED_CANARIES {
            assert!(
                !contains(out.bytes(), c),
                "{name}: {} survived",
                String::from_utf8_lossy(c)
            );
        }
    }
    // Kept data: ICC, the JUMBF payload, the AROT canary and the trailer under
    // All; none of them under Everything.
    let kept = [
        strip_shapes::C_ICC,
        strip_shapes::C_JUMBF,
        strip_shapes::C_APP10,
        strip_shapes::C_TRAILER,
    ];
    let all = strip(&s.bytes, StripPolicy::All).unwrap();
    for c in kept {
        assert!(
            contains(all.bytes(), c),
            "All lost {}",
            String::from_utf8_lossy(c)
        );
    }
    let ev = strip(&s.bytes, StripPolicy::Everything).unwrap();
    for c in kept {
        assert!(
            !contains(ev.bytes(), c),
            "Everything kept {}",
            String::from_utf8_lossy(c)
        );
    }
    // The input fixture really did carry every canary.
    for c in REMOVED_CANARIES {
        assert!(
            contains(&s.bytes, c),
            "fixture lacks {}",
            String::from_utf8_lossy(c)
        );
    }
}

#[test]
fn removals_list_every_removed_region_with_its_category_and_input_range() {
    let s = shape("kitchen-sink");
    let out = strip(&s.bytes, StripPolicy::Everything).unwrap();
    let mut seen = Vec::new();
    for r in out.removals() {
        // The record is the input range of a part the builder recorded.
        let part = s
            .parts
            .iter()
            .find(|p| p.range.start == r.offset && p.range.len() == r.len)
            .unwrap_or_else(|| panic!("no part at {}+{}", r.offset, r.len));
        assert_eq!(kind_of(part.kind), r.kind);
        if part.kind != K::Exif {
            assert_eq!(Some(r.category), strip_shapes::part_category(part.kind));
        }
        seen.push(r.offset);
    }
    // Input order.
    assert!(seen.windows(2).all(|w| w[0] < w[1]));
    // Every removable part is listed.
    let removable = s
        .parts
        .iter()
        .filter(|p| strip_shapes::part_category(p.kind).is_some() || p.kind == K::Exif)
        .count();
    assert_eq!(out.removals().len(), removable);
}

#[test]
fn sanitize_returns_the_strip_bytes() {
    for (s, name, r) in all_runs() {
        match (r, sanitize(&s.bytes, policy(name))) {
            (Ok(a), Ok(b)) => assert_eq!(a.bytes(), &b[..], "{} {name}", s.name),
            (Err(a), Err(b)) => assert_eq!(a, b, "{} {name}", s.name),
            other => panic!("{} {name}: {other:?}", s.name),
        }
    }
}

#[test]
fn the_input_is_not_modified() {
    for s in strip_shapes::all() {
        for (name, p) in policies() {
            let copy = s.bytes.clone();
            let _ = strip(&copy, p);
            assert_eq!(copy, s.bytes, "{} {name}", s.name);
        }
    }
}

#[test]
fn no_fixture_or_seed_gives_verify_failed() {
    let verify_failed =
        |r: &Result<Stripped, StripError>| matches!(r, Err(StripError::VerifyFailed { .. }));
    for (s, name, r) in all_runs() {
        assert!(!verify_failed(&r), "{} {name}: {r:?}", s.name);
    }
    // Every committed seed of the JPEG surfaces, under every policy.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fuzz/seeds");
    let mut n = 0;
    for dir in ["jpeg_segments", "audit"] {
        for entry in std::fs::read_dir(root.join(dir)).unwrap() {
            let path = entry.unwrap().path();
            let bytes = std::fs::read(&path).unwrap();
            for (name, p) in policies() {
                let r = strip(&bytes, p);
                assert!(
                    !verify_failed(&r),
                    "{dir}/{} under {name}: {r:?}",
                    path.file_name().unwrap().to_string_lossy()
                );
                n += 1;
            }
        }
    }
    assert!(n > 500, "{n}");
}

#[test]
fn verify_failed_displays_its_check() {
    let e = StripError::VerifyFailed {
        check: Check::Preview,
    };
    assert!(e.to_string().contains("Preview"));
}
