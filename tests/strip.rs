//! SPEC-020: the JPEG strip writer against the independent model in
//! `tests/support/strip_shapes.rs`. Every fixture is run under every policy; the
//! expected output is computed from the builder's own record of what each byte
//! range is, never from the library.

#[path = "support/exif_layout.rs"]
mod exif_layout;
#[path = "support/exif_records.rs"]
mod exif_records;
#[path = "support/oracle_tiff.rs"]
mod oracle_tiff;
#[path = "support/real_shapes.rs"]
mod real_shapes;
#[path = "support/strip_shapes.rs"]
mod strip_shapes;

use std::ops::Range;

use exif_core::audit::{audit, Category, ContainerKind, ItemKind, RegionKind, Sniffed};
use exif_core::policy::{StripPolicy, StripSpec};
use exif_core::write::{
    Check, PatchKind, RemovalKind, ScrubReason, UneditableReason, UnstrippableReason,
};
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
    // The file without Exif: only segments go. (On the kitchen sink the Exif
    // free text goes too, by an entry edit; see the end of this test.)
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
    // The kitchen sink: the IPTC segments and comments go as segments, the
    // Exif UserComment and ImageDescription-style free text as entries.
    let out = strip(&s.bytes, policy("iptc+free_text")).unwrap();
    assert!(!contains(out.bytes(), strip_shapes::C_USERCOMMENT));
    assert!(!contains(out.bytes(), strip_shapes::C_IPTC1));
    assert!(contains(out.bytes(), strip_shapes::C_MAKE));
    assert!(!out.entries().is_empty());
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
fn custom_free_text_deletes_the_exif_free_text_entries_and_the_comments() {
    // Was `NeedsEntryEdit` in SPEC-020 (rewritten for SPEC-021).
    let s = shape("kitchen-sink");
    let out = strip(&s.bytes, policy("free_text")).unwrap();
    assert!(!contains(out.bytes(), strip_shapes::C_USERCOMMENT));
    assert!(!contains(out.bytes(), strip_shapes::C_COM1));
    assert!(!contains(out.bytes(), strip_shapes::C_COM2));
    // Everything else in the Exif is still there.
    assert!(contains(out.bytes(), strip_shapes::C_MAKE));
    assert!(contains(out.bytes(), strip_shapes::C_GPS));
    assert_eq!(
        audit(out.bytes()).findings_in(Category::FreeText).count(),
        0
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
fn gps_preset_edits_the_exif_when_it_has_gps() {
    // Was `NeedsEntryEdit` in SPEC-020 (rewritten for SPEC-021).
    let s = shape("kitchen-sink");
    let out = strip(&s.bytes, StripPolicy::Gps).unwrap();
    assert!(!contains(out.bytes(), strip_shapes::C_GPS));
    assert_eq!(audit(out.bytes()).findings_in(Category::Gps).count(), 0);
    assert!(contains(out.bytes(), strip_shapes::C_MAKE));
    assert_eq!(out.bytes().len(), s.bytes.len());
    let out = strip(&s.bytes, StripPolicy::GpsAndDevice).unwrap();
    assert!(!contains(out.bytes(), strip_shapes::C_SERIAL));
    assert!(contains(out.bytes(), strip_shapes::C_MODEL));
}

#[test]
fn all_pii_edits_the_exif_and_leaves_software() {
    let s = shape("kitchen-sink");
    let out = strip(&s.bytes, StripPolicy::AllPii).unwrap();
    let spec = StripPolicy::AllPii.spec();
    assert!(audit(out.bytes())
        .findings()
        .iter()
        .all(|f| !spec.contains(f.category)));
    // Software is not in AllPii; it stays, with the Exif segment itself.
    assert!(contains(out.bytes(), strip_shapes::C_SOFTWARE));
    assert!(out.removals().iter().all(|r| r.kind != RemovalKind::Exif));
    // A custom set with Xmp and Icc too: nothing of those is in the Exif here.
    let custom = StripPolicy::Custom(spec_of(&[Category::Gps, Category::Xmp, Category::Icc]));
    let out = strip(&s.bytes, custom).unwrap();
    assert!(!contains(out.bytes(), strip_shapes::C_GPS));
    assert!(!contains(out.bytes(), strip_shapes::C_XMP));
}

#[test]
fn an_unreadable_exif_block_is_uneditable_for_partial_policies() {
    let s = shape("unreadable-exif");
    let at = s.parts_of(K::Exif)[0].start;
    for name in ["gps", "xmp", "icc", "free_text", "iptc+free_text"] {
        assert_eq!(
            strip(&s.bytes, policy(name)).unwrap_err(),
            StripError::Uneditable {
                reason: UneditableReason::BlockNotClean,
                offset: at
            },
            "{name}"
        );
    }
    // A policy with no category an Exif entry can have does not care.
    let out = strip(&s.bytes, policy("jumbf")).unwrap();
    assert_eq!(out.bytes(), &s.bytes[..]);
    // Removing every Exif category removes the unreadable block too.
    let out = strip(&s.bytes, StripPolicy::All).unwrap();
    assert!(out.removals().iter().any(|r| r.kind == RemovalKind::Exif));
}

#[test]
fn a_block_with_a_walk_issue_is_uneditable_for_every_partial_policy() {
    // Found by the verifier of SPEC-020: a GPS IFD declaring 0xFFFF entries
    // gives no GPS finding and no failed block, only a TIFF-layer issue, yet
    // the latitude and longitude bytes are in the file. A bad IFD1 pointer and
    // a bad Exif pointer are the same class.
    let s = shape("gps-count-overflow");
    assert!(contains(&s.bytes, strip_shapes::C_GPSNUM));
    assert_eq!(audit(&s.bytes).findings_in(Category::Gps).count(), 0);
    let at = s.parts_of(K::Exif)[0].start;
    for p in [
        StripPolicy::Gps,
        StripPolicy::GpsAndDevice,
        StripPolicy::AllPii,
        policy("xmp"),
    ] {
        assert_eq!(
            strip(&s.bytes, p).unwrap_err(),
            StripError::Uneditable {
                reason: UneditableReason::BlockNotClean,
                offset: at
            },
            "{p:?}"
        );
    }
    for l in [
        exif_layout::bad_ifd1_pointer(),
        exif_layout::bad_exif_pointer(),
    ] {
        let bytes = jpeg_with(&[l.bytes]);
        for p in [StripPolicy::Gps, StripPolicy::AllPii, policy("free_text")] {
            assert!(
                matches!(
                    strip(&bytes, p),
                    Err(StripError::Uneditable {
                        reason: UneditableReason::BlockNotClean,
                        ..
                    })
                ),
                "{p:?}"
            );
        }
    }
    // All removes the block whole.
    let out = strip(&s.bytes, StripPolicy::All).unwrap();
    assert!(!contains(out.bytes(), strip_shapes::C_GPSNUM));
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
fn v03b_the_primary_size_is_patched_even_when_it_was_already_wrong() {
    // The index says the primary is 7 bytes longer than it is. The field is
    // still patchable (7 + the file's length is above the removed bytes), and
    // DEC-013 part 5 says it becomes `size - removed(0, primary_end)`, right or
    // wrong; a writer that patches it only when it was right would leave the
    // stale value.
    let s = strip_shapes::mpo_late_xmp(false);
    let m = s.mpf.as_ref().unwrap();
    let mut bytes = s.bytes.clone();
    let wrong = m.entries[0].0 + 7;
    bytes[m.fields[0].0..m.fields[0].0 + 4].copy_from_slice(&wrong.to_le_bytes());
    let out = strip(&bytes, StripPolicy::All).unwrap();
    let exif_len = s.parts_of(K::Exif)[0].len();
    let late = s.parts_of(K::Xmp)[0].len() + s.parts_of(K::Com)[0].len();
    let at = m.fields[0].0 - exif_len;
    let got = u32::from_le_bytes(out.bytes()[at..at + 4].try_into().unwrap());
    assert_eq!(got, wrong - (exif_len + late) as u32);
    // The secondaries are patched as before (3 patches in all).
    assert_eq!(out.patches().len(), 3);
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
            (
                Expect::Uneditable(o),
                Err(StripError::Uneditable {
                    reason: UneditableReason::BlockNotClean,
                    offset,
                }),
            ) => {
                assert_eq!(o, offset, "{} {name}", s.name)
            }
            // The exact bytes of an entry edit are computed from the layout
            // record in the tests below; here only the library's own
            // invariants (the tests after this one) apply.
            (Expect::EntryEdit, Ok(out)) => {
                assert!(!out.entries().is_empty(), "{} {name}", s.name);
            }
            (Expect::EntryEdit, Err(StripError::Uneditable { .. })) => {}
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
            // Only same-length patches (Exif entry edits) can change anything.
            assert_eq!(out.bytes().len(), s.bytes.len(), "{} {name}", s.name);
            assert!(!out.patches().is_empty() || out.bytes() == &s.bytes[..]);
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

// ═══ SPEC-021: Exif entry editing, against the layout record ═══════════════
//
// Every expected byte below comes from `exif_layout::expected` (the fixture
// builder's record of where each table, entry, value and thumbnail lies) and
// `strip_shapes::expected_bytes`, never from the library or its walk.

use exif_layout::{del_for, deleted_entries, expected, Cat, Layout};
use exif_records::{
    category_of, exif_fixtures, exif_policies, jfif_bytes, jpeg_of, jpeg_with, scan_bytes, Jpg,
    ALL_PII,
};

use exif_records::model_refusal;

/// The expected output of `strip` over `j` for the categories `spec`: the COM
/// removed under `free_text`, every blob edited per the layout record.
fn expected_jpeg(j: &Jpg, spec: &[Cat]) -> Vec<u8> {
    let removals: Vec<Range<usize>> = j
        .com
        .iter()
        .filter(|_| spec.contains(&"free_text"))
        .cloned()
        .collect();
    let patches: Vec<(usize, Vec<u8>)> = j
        .layouts
        .iter()
        .zip(&j.blobs)
        .filter_map(|(l, at)| {
            let d = del_for(l, spec);
            (!d.is_empty()).then(|| (*at, expected(l, &d)))
        })
        .collect();
    expected_bytes(&j.bytes, &removals, &patches)
}

type ExifRun = (&'static str, Jpg, &'static str, &'static [Cat], StripPolicy);

fn exif_runs(com: bool) -> Vec<ExifRun> {
    let mut v = Vec::new();
    for (name, layouts) in exif_fixtures() {
        for (pname, cats, p) in exif_policies() {
            v.push((name, jpeg_of(&layouts, com), pname, cats, p));
        }
    }
    v
}

#[test]
fn the_expected_output_computed_from_the_fixture_layout_equals_the_library_output() {
    let mut oks = 0;
    for (name, j, pname, cats, p) in exif_runs(true) {
        let got = strip(&j.bytes, p);
        if let Some(reason) = model_refusal(name, cats) {
            match got {
                Err(StripError::Uneditable { reason: r, .. }) => {
                    assert_eq!(r, reason, "{name} {pname}")
                }
                other => panic!("{name} {pname}: {other:?}"),
            }
            continue;
        }
        let out = got.unwrap_or_else(|e| panic!("{name} {pname}: {e}"));
        assert_eq!(out.bytes(), &expected_jpeg(&j, cats)[..], "{name} {pname}");
        oks += 1;
    }
    assert!(oks > 100, "{oks}");
}

#[test]
fn output_length_equals_input_length_when_only_exif_entries_go() {
    for (name, j, pname, cats, p) in exif_runs(false) {
        if model_refusal(name, cats).is_some() {
            continue;
        }
        let out = strip(&j.bytes, p).unwrap();
        assert_eq!(out.bytes().len(), j.bytes.len(), "{name} {pname}");
        assert!(out.removals().is_empty(), "{name} {pname}");
    }
}

#[test]
fn gps_removes_the_gps_ifd_and_keeps_everything_else() {
    for be in [false, true] {
        let l = exif_layout::full(be).layout();
        let j = jpeg_of(std::slice::from_ref(&l), false);
        let out = strip(&j.bytes, StripPolicy::Gps).unwrap();
        let o = out.bytes();
        // The GPS values and the pointer are gone.
        assert!(!contains(o, exif_layout::K_GPSAREA), "be={be}");
        let lat: Vec<u8> = l
            .ifd("gps")
            .ents
            .iter()
            .filter_map(|e| e.canary.clone())
            .flatten()
            .collect();
        assert!(!contains(o, &lat[..8.min(lat.len())]), "be={be}");
        let a = audit(o);
        assert_eq!(a.findings_in(Category::Gps).count(), 0);
        assert!(!a
            .items()
            .iter()
            .any(|i| matches!(i.kind(), ItemKind::Tag { tag: 0x8825, .. })));
        // Everything else is as it was: every other canary, the thumbnail,
        // the Orientation entry bytes.
        for f in l.ifds.iter().filter(|f| f.name != "gps") {
            for e in &f.ents {
                if e.tag == 0x8825 {
                    continue;
                }
                if let Some(c) = &e.canary {
                    assert!(contains(o, c), "be={be} {} {:#x}", f.name, e.tag);
                }
            }
        }
        assert!(contains(o, exif_layout::K_THUMB));
        assert_eq!(a.findings_in(Category::Thumbnail).count(), 1);
        assert_eq!(o.len(), j.bytes.len());
        // Six GPS entries and the pointer, one table patch, the values.
        assert_eq!(out.entries().len(), 1 + 6);
    }
}

#[test]
fn gps_and_device_removes_the_four_device_ids_too() {
    let l = exif_layout::full_le().layout();
    let j = jpeg_of(&[l], false);
    let out = strip(&j.bytes, StripPolicy::GpsAndDevice).unwrap();
    for c in [
        exif_layout::K_BODYSERIAL,
        exif_layout::K_UID,
        exif_layout::K_LENSSERIAL,
        exif_layout::K_CAMSERIAL,
        exif_layout::K_GPSAREA,
    ] {
        assert!(!contains(out.bytes(), c), "{}", String::from_utf8_lossy(c));
    }
    for c in [
        exif_layout::K_MAKE,
        exif_layout::K_LENS,
        exif_layout::K_SOFTWARE,
        exif_layout::K_ARTIST,
        exif_layout::K_USERCOMMENT,
        exif_layout::K_NOTE,
        exif_layout::K_THUMB,
    ] {
        assert!(contains(out.bytes(), c), "{}", String::from_utf8_lossy(c));
    }
    assert_eq!(
        audit(out.bytes()).findings_in(Category::DeviceId).count(),
        0
    );
}

#[test]
fn all_pii_matches_the_policy_table_entry_by_entry() {
    for be in [false, true] {
        let l = exif_layout::full(be).layout();
        let j = jpeg_of(std::slice::from_ref(&l), false);
        let out = strip(&j.bytes, StripPolicy::AllPii).unwrap();
        let d = del_for(&l, ALL_PII);
        // The records are exactly the model's deleted entries, in blob order.
        let want: Vec<(u16, usize)> = deleted_entries(&l, &d)
            .iter()
            .map(|(_, tag, at)| (*tag, j.blobs[0] + at))
            .collect();
        let got: Vec<(u16, usize)> = out.entries().iter().map(|e| (e.tag, e.offset)).collect();
        assert_eq!(got, want, "be={be}");
        // The inventory of the output: every surviving entry, nothing else.
        let kept: Vec<(u16, u16)> = {
            let mut v = Vec::new();
            for f in &l.ifds {
                for (i, e) in f.ents.iter().enumerate() {
                    if !d.whole.contains(&f.name) && !d.entries.contains(&(f.name, i)) {
                        v.push((e.tag, e.ty));
                    }
                }
            }
            v.sort();
            v
        };
        let mut have: Vec<(u16, u16)> = audit(out.bytes())
            .items()
            .iter()
            .filter_map(|i| match i.kind() {
                ItemKind::Tag {
                    tag, field_type, ..
                } => Some((tag, field_type)),
                _ => None,
            })
            .collect();
        have.sort();
        assert_eq!(have, kept, "be={be}");
    }
}

#[test]
fn all_pii_keeps_software_and_orientation_and_exif_resident_xmp() {
    let mut b = exif_layout::full_le();
    b.ifds[0].ents.push(exif_layout::Ent {
        tag: 0x02BC,
        ty: exif_layout::BYTE,
        count: exif_layout::K_XMPTAG.len() as u32,
        val: exif_layout::Val::Data(exif_layout::K_XMPTAG.to_vec()),
        canary: Some(exif_layout::K_XMPTAG.to_vec()),
    });
    let l = b.layout();
    let j = jpeg_of(std::slice::from_ref(&l), false);
    let out = strip(&j.bytes, StripPolicy::AllPii).unwrap();
    let o = out.bytes();
    assert!(contains(o, exif_layout::K_SOFTWARE));
    assert!(contains(o, exif_layout::K_XMPTAG));
    let orientation = l.ifd("ifd0").ents.iter().find(|e| e.tag == 0x0112).unwrap();
    assert!(contains(o, &l.bytes[orientation.at..orientation.at + 12]));
    let a = audit(o);
    assert_eq!(a.findings_in(Category::Software).count(), 1);
    assert_eq!(a.findings_in(Category::Xmp).count(), 1);
    assert_eq!(a.findings_in(Category::Gps).count(), 0);
    assert_eq!(a.findings_in(Category::MakerNote).count(), 0);
    assert_eq!(a.findings_in(Category::Thumbnail).count(), 0);
}

/// An XMP segment (with a canary) after the Exif of `l`, for the custom-Xmp
/// test; the XMP tag lives in IFD0 of `l`.
fn jpeg_with_xmp_segment(l: &Layout) -> Vec<u8> {
    let mut v = vec![0xFF, 0xD8];
    v.extend_from_slice(&jfif_bytes());
    v.extend_from_slice(&real_shapes::exif_app1(&l.bytes));
    v.extend_from_slice(&real_shapes::seg(
        0xE1,
        &[
            &b"http://ns.adobe.com/xap/1.0/\0<x:xmpmeta>"[..],
            strip_shapes::C_XMP,
            b"</x:xmpmeta>",
        ]
        .concat(),
    ));
    v.extend_from_slice(&scan_bytes());
    v
}

#[test]
fn custom_xmp_deletes_the_exif_resident_xmp_tag_and_the_segments() {
    let l = exif_layout::xmp_tag_inside().layout();
    let bytes = jpeg_with_xmp_segment(&l);
    let p = policy("xmp");
    let out = strip(&bytes, p).unwrap();
    let o = out.bytes();
    assert!(!contains(o, exif_layout::K_XMPTAG));
    assert!(!contains(o, strip_shapes::C_XMP));
    // All the Exif entries of other categories stay.
    assert!(contains(o, exif_layout::K_MAKE));
    let a = audit(o);
    assert_eq!(a.findings_in(Category::Xmp).count(), 0);
    assert_eq!(a.findings_in(Category::CameraModel).count(), 1);
    assert_eq!(out.entries().len(), 1);
    assert_eq!(out.entries()[0].tag, 0x02BC);
    assert_eq!(out.removals().len(), 1);
    // The Exif edit: model bytes at the Exif blob.
    let d = del_for(&l, &["xmp"]);
    assert_eq!(
        &o[10 + jfif_bytes().len() + 2..][..l.bytes.len()],
        &expected(&l, &d)[..]
    );
}

fn only(cats: &'static [Cat]) -> StripPolicy {
    StripPolicy::Custom(
        cats.iter()
            .fold(StripSpec::none(), |s, c| s.include(category_of(c))),
    )
}

#[test]
fn custom_software_only() {
    let l = exif_layout::full_le().layout();
    let j = jpeg_of(&[l], false);
    let out = strip(&j.bytes, only(&["software"])).unwrap();
    assert!(!contains(out.bytes(), exif_layout::K_SOFTWARE));
    for c in [
        exif_layout::K_MAKE,
        exif_layout::K_ARTIST,
        exif_layout::K_GPSAREA,
        exif_layout::K_NOTE,
        exif_layout::K_THUMB,
    ] {
        assert!(contains(out.bytes(), c));
    }
    assert_eq!(out.entries().len(), 1);
    assert_eq!(out.entries()[0].tag, 0x0131);
}

#[test]
fn custom_timestamps_and_gps() {
    let l = exif_layout::full_be().layout();
    let j = jpeg_of(&[l], false);
    let out = strip(&j.bytes, only(&["timestamps", "gps"])).unwrap();
    for c in [
        exif_layout::K_DATETIME,
        exif_layout::K_DTO,
        exif_layout::K_GPSAREA,
    ] {
        assert!(!contains(out.bytes(), c), "{}", String::from_utf8_lossy(c));
    }
    assert!(contains(out.bytes(), exif_layout::K_MAKE));
    assert_eq!(
        audit(out.bytes()).findings_in(Category::Timestamps).count(),
        0
    );
}

#[test]
fn custom_makernote_only() {
    let l = exif_layout::full_le().layout();
    let j = jpeg_of(std::slice::from_ref(&l), false);
    let out = strip(&j.bytes, only(&["maker_note"])).unwrap();
    assert!(!contains(out.bytes(), exif_layout::K_NOTE));
    assert!(contains(out.bytes(), exif_layout::K_GPSAREA));
    assert!(contains(out.bytes(), exif_layout::K_THUMB));
    assert_eq!(out.entries().len(), 1);
    // The MakerNote's 64 bytes are scrubbed (one scrub record of that length).
    assert!(out
        .scrubs()
        .iter()
        .any(|s| s.len == 64 && s.reason == ScrubReason::EntryValue));
    let note = l.ifd("exif").ents.iter().find(|e| e.tag == 0x927C).unwrap();
    assert_eq!(out.entries()[0].offset, j.blobs[0] + note.at);
}

#[test]
fn custom_thumbnail_only() {
    let l = exif_layout::full_le().layout();
    let j = jpeg_of(std::slice::from_ref(&l), false);
    let out = strip(&j.bytes, only(&["thumbnail"])).unwrap();
    assert!(!contains(out.bytes(), exif_layout::K_THUMB));
    assert!(contains(out.bytes(), exif_layout::K_GPSAREA));
    assert_eq!(
        audit(out.bytes()).findings_in(Category::Thumbnail).count(),
        0
    );
    let t = l.thumb.clone().unwrap();
    assert!(out
        .scrubs()
        .iter()
        .any(|s| s.reason == ScrubReason::Thumbnail && s.len == t.len()));
    // IFD1 is gone whole: its four entries are recorded.
    assert_eq!(out.entries().len(), 4);
}

#[test]
fn exif_64k_block_does_not_grow_and_stays_valid() {
    let l = exif_layout::at_the_ceiling().layout();
    assert_eq!(l.bytes.len(), exif_layout::CEILING_BLOB);
    let j = jpeg_of(&[l], false);
    // The segment's length word is 0xFFFF.
    assert_eq!(&j.bytes[j.first_exif + 2..j.first_exif + 4], &[0xFF, 0xFF]);
    for (name, p) in [("gps", StripPolicy::Gps), ("all_pii", StripPolicy::AllPii)] {
        let out = strip(&j.bytes, p).unwrap();
        assert_eq!(out.bytes().len(), j.bytes.len(), "{name}");
        assert_eq!(
            &out.bytes()[j.first_exif..j.first_exif + 4],
            &j.bytes[j.first_exif..j.first_exif + 4],
            "{name}"
        );
        let a = audit(out.bytes());
        assert!(a.health().clean, "{name}: {:?}", a.health());
    }
}

#[test]
fn no_removed_canary_survives_anywhere_in_the_output() {
    let mut checked = 0;
    for (name, j, pname, cats, p) in exif_runs(true) {
        if model_refusal(name, cats).is_some() {
            continue;
        }
        let out = strip(&j.bytes, p).unwrap();
        for l in &j.layouts {
            let d = del_for(l, cats);
            for f in &l.ifds {
                for (i, e) in f.ents.iter().enumerate() {
                    let removed = d.whole.contains(&f.name) || d.entries.contains(&(f.name, i));
                    if let (true, Some(c)) = (removed, &e.canary) {
                        assert!(
                            !contains(out.bytes(), c),
                            "{name} {pname}: {} {:#x} left",
                            f.name,
                            e.tag
                        );
                        checked += 1;
                    }
                }
            }
            if d.thumb {
                assert!(
                    !contains(out.bytes(), exif_layout::K_THUMB),
                    "{name} {pname}"
                );
            }
        }
        if cats.contains(&"free_text") {
            assert!(
                !contains(out.bytes(), strip_shapes::C_COM1),
                "{name} {pname}"
            );
        }
    }
    assert!(checked > 200, "{checked}");
}

#[test]
fn every_kept_canary_survives() {
    for (name, j, pname, cats, p) in exif_runs(true) {
        if model_refusal(name, cats).is_some() {
            continue;
        }
        let out = strip(&j.bytes, p).unwrap();
        for l in &j.layouts {
            let d = del_for(l, cats);
            for f in &l.ifds {
                for (i, e) in f.ents.iter().enumerate() {
                    let removed = d.whole.contains(&f.name) || d.entries.contains(&(f.name, i));
                    if removed {
                        continue;
                    }
                    if let Some(c) = &e.canary {
                        assert!(
                            contains(out.bytes(), c),
                            "{name} {pname}: {} {:#x}",
                            f.name,
                            e.tag
                        );
                    }
                    // A kept entry's own 12 bytes (inline values included).
                    assert!(
                        contains(out.bytes(), &l.bytes[e.at..e.at + 12]),
                        "{name} {pname}: {} {:#x} entry",
                        f.name,
                        e.tag
                    );
                }
            }
            if !d.thumb && l.thumb.is_some() {
                assert!(
                    contains(out.bytes(), exif_layout::K_THUMB),
                    "{name} {pname}"
                );
            }
        }
        if !cats.contains(&"free_text") {
            assert!(
                contains(out.bytes(), strip_shapes::C_COM1),
                "{name} {pname}"
            );
        }
    }
}

#[test]
fn the_output_equals_the_input_outside_the_recorded_patches_and_removals() {
    for (name, j, pname, _cats, p) in exif_runs(true) {
        let Ok(out) = strip(&j.bytes, p) else {
            continue;
        };
        // Input bytes that survive, in order, with their input offsets.
        let mut gone = vec![false; j.bytes.len()];
        for r in out.removals() {
            for g in &mut gone[r.offset..r.offset + r.len] {
                *g = true;
            }
        }
        let kept: Vec<usize> = (0..j.bytes.len()).filter(|i| !gone[*i]).collect();
        assert_eq!(kept.len(), out.bytes().len(), "{name} {pname}");
        let mut patched = vec![false; out.bytes().len()];
        for pr in out.patches() {
            for x in &mut patched[pr.offset..pr.offset + pr.len] {
                *x = true;
            }
        }
        for (o, i) in kept.iter().enumerate() {
            if !patched[o] {
                assert_eq!(out.bytes()[o], j.bytes[*i], "{name} {pname} byte {o}");
            }
        }
    }
}

#[test]
fn scan_region_and_kept_segments_are_byte_identical() {
    for (name, j, pname, _cats, p) in exif_runs(true) {
        let Ok(out) = strip(&j.bytes, p) else {
            continue;
        };
        let scan = scan_bytes();
        assert!(out.bytes().ends_with(&scan), "{name} {pname}");
        assert!(out.bytes().starts_with(&[0xFF, 0xD8]), "{name} {pname}");
        assert!(contains(out.bytes(), &jfif_bytes()), "{name} {pname}");
        // The Exif segments keep their marker and length words.
        for (k, at) in j.blobs.iter().enumerate() {
            let seg = at - 10;
            let shift = if out.removals().is_empty() {
                0
            } else {
                j.com.clone().map_or(0, |c| c.len())
            };
            let _ = k;
            let o = seg - if out.removals().is_empty() { 0 } else { shift };
            assert_eq!(
                &out.bytes()[o..o + 4],
                &j.bytes[seg..seg + 4],
                "{name} {pname}"
            );
        }
    }
}

#[test]
fn the_output_audit_has_no_finding_in_the_policy_and_the_same_kept_findings() {
    for (name, j, pname, cats, p) in exif_runs(false) {
        let Ok(out) = strip(&j.bytes, p) else {
            continue;
        };
        let spec = p.spec();
        let before = audit(&j.bytes);
        let after = audit(out.bytes());
        assert!(
            after.findings().iter().all(|f| !spec.contains(f.category)),
            "{name} {pname}"
        );
        let key = |a: &exif_core::audit::FileAudit<'_>| {
            let mut v: Vec<(String, Option<Vec<u8>>)> = a
                .findings()
                .iter()
                .filter(|f| !spec.contains(f.category))
                .map(|f| {
                    (
                        format!(
                            "{:?}/{}",
                            f.category,
                            match f.at.kind() {
                                ItemKind::Tag { ifd, tag, .. } => format!("{ifd:?}{tag}"),
                                ItemKind::Region { kind, .. } => format!("{kind:?}"),
                            }
                        ),
                        f.at.value().map(|v| v.to_vec()),
                    )
                })
                .collect();
            v.sort();
            v
        };
        assert_eq!(key(&before), key(&after), "{name} {pname} {cats:?}");
    }
}

#[test]
fn strip_is_idempotent_and_a_wider_policy_composes() {
    for (name, j, pname, _cats, p) in exif_runs(true) {
        let Ok(out) = strip(&j.bytes, p) else {
            continue;
        };
        let twice = strip(out.bytes(), p).unwrap();
        assert_eq!(twice.bytes(), out.bytes(), "{name} {pname}");
        assert!(
            twice.removals().is_empty() && twice.patches().is_empty() && twice.entries().is_empty(),
            "{name} {pname}"
        );
    }
    // A narrow strip followed by a wide one equals the wide one alone.
    for (name, layouts) in exif_fixtures() {
        let j = jpeg_of(&layouts, true);
        if name == "chain-ifd2" {
            continue;
        }
        let narrow = strip(&j.bytes, StripPolicy::Gps).unwrap();
        let wide_after = strip(narrow.bytes(), StripPolicy::AllPii).unwrap();
        let wide = strip(&j.bytes, StripPolicy::AllPii).unwrap();
        assert_eq!(wide_after.bytes(), wide.bytes(), "{name}");
    }
}

#[test]
fn whole_and_none_policies_are_byte_identical_to_spec_020_outputs() {
    // SPEC-020's behaviour for a whole policy is the segments gone; for a none
    // policy the input back. (The old fixtures run the same grid in
    // `output_equals_the_input_minus_the_listed_removals_plus_the_listed_patches`.)
    for (name, layouts) in exif_fixtures() {
        let j = jpeg_of(&layouts, true);
        for p in [StripPolicy::All, StripPolicy::Everything] {
            let out = strip(&j.bytes, p).unwrap();
            let mut gone: Vec<Range<usize>> = j.com.iter().cloned().collect();
            for (k, l) in layouts.iter().enumerate() {
                let at = j.blobs[k] - 10;
                gone.push(at..at + 10 + l.bytes.len());
            }
            assert_eq!(
                out.bytes(),
                &expected_bytes(&j.bytes, &gone, &[])[..],
                "{name} {p:?}"
            );
            assert!(
                out.patches().is_empty() && out.entries().is_empty(),
                "{name}"
            );
        }
        for spec in [
            StripSpec::none(),
            exif_core::StripPolicy::Custom(StripSpec::none()).spec(),
        ] {
            let out = strip(&j.bytes, StripPolicy::Custom(spec)).unwrap();
            assert_eq!(out.bytes(), &j.bytes[..], "{name}");
        }
        // A policy with only non-Exif-capable categories leaves Exif alone too.
        let out = strip(&j.bytes, policy("jumbf")).unwrap();
        assert_eq!(out.bytes(), &j.bytes[..], "{name}");
    }
}

#[test]
fn a_walk_issue_in_an_ifd_with_no_target_still_refuses() {
    let l = exif_layout::walk_issue_in_a_kept_ifd().layout();
    let j = jpeg_of(&[l], false);
    // The issue is in the Exif IFD's Interop pointer; the policy targets only
    // GPS, which is fine. Still a refusal, for every partial policy.
    for (pname, _, p) in exif_policies() {
        assert_eq!(
            strip(&j.bytes, p).unwrap_err(),
            StripError::Uneditable {
                reason: UneditableReason::BlockNotClean,
                offset: j.first_exif
            },
            "{pname}"
        );
    }
}

#[test]
fn exif_after_the_first_scan_is_uneditable() {
    let l = exif_layout::full_le().layout();
    let mut v = vec![0xFF, 0xD8];
    v.extend_from_slice(&jfif_bytes());
    v.extend_from_slice(&real_shapes::seg(0xDA, &[1, 1, 0, 0, 0x3F, 0]));
    v.extend_from_slice(&[0x12, 0x34]);
    let at = v.len();
    v.extend_from_slice(&real_shapes::exif_app1(&l.bytes));
    v.extend_from_slice(&real_shapes::EOI);
    for p in [StripPolicy::Gps, StripPolicy::AllPii, policy("xmp")] {
        assert_eq!(
            strip(&v, p).unwrap_err(),
            StripError::Uneditable {
                reason: UneditableReason::AfterScan,
                offset: at
            }
        );
    }
}

#[test]
fn seventeen_exif_segments_are_uneditable() {
    let l = exif_layout::first_entry_target().layout();
    let blobs: Vec<Vec<u8>> = (0..17).map(|_| l.bytes.clone()).collect();
    let v = jpeg_with(&blobs);
    let seventeenth = 2 + jfif_bytes().len() + 16 * (4 + 6 + l.bytes.len());
    assert_eq!(
        strip(&v, StripPolicy::Gps).unwrap_err(),
        StripError::Uneditable {
            reason: UneditableReason::TooManyExifSegments,
            offset: seventeenth
        }
    );
    // Sixteen is fine.
    assert!(strip(&jpeg_with(&blobs[..16]), StripPolicy::Gps).is_ok());
}

#[test]
fn an_inventory_over_budget_is_uneditable() {
    // 13 blocks of 5,459 entries: 70,967 tag items, past the audit's 65,536.
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
    assert!(big.bytes.len() <= exif_layout::CEILING_BLOB);
    let blobs: Vec<Vec<u8>> = (0..13).map(|_| big.bytes.clone()).collect();
    let v = jpeg_with(&blobs);
    assert!(audit(&v).health().inventory_truncated);
    assert_eq!(
        strip(&v, StripPolicy::Gps).unwrap_err(),
        StripError::Uneditable {
            reason: UneditableReason::InventoryTruncated,
            offset: 2 + jfif_bytes().len()
        }
    );
}

#[test]
fn two_exif_blocks_only_the_targeted_one_changes() {
    let two = exif_layout::two_blocks();
    let (a, b) = (two.0.layout(), two.1.layout());
    let j = jpeg_of(&[a.clone(), b.clone()], false);
    let out = strip(&j.bytes, only(&["camera_model"])).unwrap();
    let o = out.bytes();
    // The first blob is the edit the record predicts; the second is untouched.
    let d = del_for(&a, &["camera_model"]);
    assert_eq!(
        &o[j.blobs[0]..j.blobs[0] + a.bytes.len()],
        &expected(&a, &d)[..]
    );
    assert_eq!(&o[j.blobs[1]..j.blobs[1] + b.bytes.len()], &b.bytes[..]);
    assert_eq!(out.entries().len(), 1);
    assert_eq!(out.entries()[0].block, 0);
    // Targeting both: the records name block 1 too.
    let out = strip(&j.bytes, only(&["camera_model", "software"])).unwrap();
    assert!(out.entries().iter().all(|e| e.block == 0));
}

#[test]
fn chain_ifd_and_sub_ifd_targets_are_deleted() {
    // IFD2 of the chain: the Artist goes, IFD0 and IFD1 are as they were.
    let l = exif_layout::chain_ifd2().layout();
    let j = jpeg_of(std::slice::from_ref(&l), false);
    let out = strip(&j.bytes, only(&["authorship"])).unwrap();
    assert!(!contains(out.bytes(), exif_layout::K_CHAINARTIST));
    assert!(contains(out.bytes(), exif_layout::K_THUMB));
    assert_eq!(out.entries().len(), 1);
    assert_eq!(out.entries()[0].ifd, exif_core::tiff::IfdKind::Chain);
    let d = del_for(&l, &["authorship"]);
    assert_eq!(
        &out.bytes()[j.blobs[0]..][..l.bytes.len()],
        &expected(&l, &d)[..]
    );
    // A SubIFD.
    let l = exif_layout::subifd().layout();
    let j = jpeg_of(std::slice::from_ref(&l), false);
    let out = strip(&j.bytes, only(&["authorship"])).unwrap();
    assert!(!contains(out.bytes(), exif_layout::K_SUBARTIST));
    assert_eq!(out.entries()[0].ifd, exif_core::tiff::IfdKind::Sub);
    let d = del_for(&l, &["authorship"]);
    assert_eq!(
        &out.bytes()[j.blobs[0]..][..l.bytes.len()],
        &expected(&l, &d)[..]
    );
}

#[test]
fn stripped_records_list_every_entry_scrub_and_patch_with_kinds() {
    let l = exif_layout::full_le().layout();
    let j = jpeg_of(std::slice::from_ref(&l), true);
    let out = strip(&j.bytes, StripPolicy::AllPii).unwrap();
    let com = j.com.clone().unwrap().len();
    // Entries: input coordinates, input order, the pointer included.
    let d = del_for(&l, ALL_PII);
    let want: Vec<(u16, usize)> = deleted_entries(&l, &d)
        .iter()
        .map(|(_, tag, at)| (*tag, j.blobs[0] + at))
        .collect();
    let got: Vec<(u16, usize)> = out.entries().iter().map(|e| (e.tag, e.offset)).collect();
    assert_eq!(got, want);
    assert!(out.entries().windows(2).all(|w| w[0].offset < w[1].offset));
    // Patches: output coordinates (the COM before the Exif shifted them),
    // sorted, tagged.
    assert!(out.patches().windows(2).all(|w| w[0].offset < w[1].offset));
    let tables: Vec<_> = out
        .patches()
        .iter()
        .filter(|p| p.kind == PatchKind::IfdTable)
        .collect();
    // IFD0 and the Exif IFD lose entries (IFD1 and GPS go whole, scrubbed).
    assert_eq!(tables.len(), 2);
    assert_eq!(
        tables[0].offset,
        j.blobs[0] + l.ifd("ifd0").table.start - com
    );
    assert_eq!(tables[0].len, l.ifd("ifd0").table.len());
    let scrub_patches: Vec<(usize, usize)> = out
        .patches()
        .iter()
        .filter(|p| p.kind == PatchKind::Scrub)
        .map(|p| (p.offset, p.len))
        .collect();
    let scrubs: Vec<(usize, usize)> = out.scrubs().iter().map(|s| (s.offset, s.len)).collect();
    assert_eq!(scrub_patches, scrubs);
    assert!(out
        .scrubs()
        .iter()
        .any(|s| s.reason == ScrubReason::IfdTable));
    assert!(out
        .scrubs()
        .iter()
        .any(|s| s.reason == ScrubReason::EntryValue));
    assert!(out
        .scrubs()
        .iter()
        .any(|s| s.reason == ScrubReason::Thumbnail));
    // Every scrub range reads zero in the output.
    for s in out.scrubs() {
        assert!(out.bytes()[s.offset..s.offset + s.len]
            .iter()
            .all(|b| *b == 0));
    }
    // The COM goes (FreeText): one removal, in input coordinates.
    assert_eq!(out.removals().len(), 1);
    assert_eq!(out.removals()[0].offset, j.com.unwrap().start);
}

#[test]
fn uneditable_errors_name_reason_and_offset_and_return_no_bytes() {
    let l = exif_layout::value_shared_with_kept_entry().layout();
    let j = jpeg_of(std::slice::from_ref(&l), false);
    let sw = l.ifd("ifd0").ents.iter().find(|e| e.tag == 0x0131).unwrap();
    let at = j.blobs[0] + sw.value.clone().unwrap().start;
    let err = strip(&j.bytes, only(&["camera_model"])).unwrap_err();
    assert_eq!(
        err,
        StripError::Uneditable {
            reason: UneditableReason::SharedBytes,
            offset: at
        }
    );
    let text = err.to_string();
    assert!(
        text.contains("SharedBytes") && text.contains(&at.to_string()),
        "{text}"
    );
    assert_eq!(
        sanitize(&j.bytes, only(&["camera_model"])).unwrap_err(),
        err
    );
}

#[test]
fn the_refusal_order_is_unstrippable_uneditable_mpf_not_indexed_overlap_verify_failed() {
    // Unstrippable beats Uneditable: a bad length word and an unreadable Exif.
    let mut v = vec![0xFF, 0xD8];
    v.extend_from_slice(&jfif_bytes());
    v.extend_from_slice(&[0xFF, 0xFE, 0x00, 0x01]);
    v.extend_from_slice(&real_shapes::exif_app1(b"not a tiff header"));
    v.extend_from_slice(&scan_bytes());
    assert!(matches!(
        strip(&v, StripPolicy::Gps),
        Err(StripError::Unstrippable { .. })
    ));
    // Uneditable beats MpfNotIndexed: an unreadable Exif, then an MPF segment
    // the audit cannot index, then a segment the policy removes.
    let mut v = vec![0xFF, 0xD8];
    v.extend_from_slice(&jfif_bytes());
    v.extend_from_slice(&real_shapes::exif_app1(b"not a tiff header"));
    v.extend_from_slice(&real_shapes::seg(0xE2, b"MPF\0this is not a tiff header"));
    v.extend_from_slice(&real_shapes::seg(0xFE, strip_shapes::C_COM1));
    v.extend_from_slice(&scan_bytes());
    assert!(matches!(
        strip(&v, only(&["free_text"])),
        Err(StripError::Uneditable { .. })
    ));
    // Without the Exif problem the same file is MpfNotIndexed (the next in
    // the order), and `com_over_embedded` is Overlap (the one after).
    let s = shape("unindexed-mpf");
    assert!(matches!(
        strip(&s.bytes, policy("xmp")),
        Err(StripError::MpfNotIndexed { .. })
    ));
    let s = shape("com-over-embedded");
    assert!(matches!(
        strip(&s.bytes, policy("free_text")),
        Err(StripError::Overlap { .. })
    ));
}

#[test]
fn mpf_patches_and_exif_patches_compose_in_one_call() {
    let l = exif_layout::full_le().layout();
    let com = real_shapes::seg(0xFE, strip_shapes::C_COM1);
    let s = strip_shapes::mpo_shape(
        "mpo-with-full-exif",
        false,
        vec![(K::Exif, real_shapes::exif_app1(&l.bytes))],
        vec![(K::Com, com.clone())],
        vec![
            (Vec::new(), real_shapes::nikon_extra()),
            (Vec::new(), real_shapes::iphone_extra()),
        ],
        Vec::new(),
        strip_shapes::ExifFacts::default(),
    );
    let m = s.mpf.clone().unwrap();
    let exif_at = s.parts_of(K::Exif)[0].start + 10;
    let com_range = s.parts_of(K::Com)[0].clone();
    // Gps (Exif entry edit) and FreeText (the COM after the index, plus the
    // Exif free-text entries).
    let cats: &[Cat] = &["gps", "free_text"];
    let out = strip(&s.bytes, only(cats)).unwrap();
    let gone = com.len() as u32;
    let enc = |v: u32| v.to_le_bytes().to_vec();
    let mut patches: Vec<(usize, Vec<u8>)> = vec![
        (m.fields[0].0, enc(m.entries[0].0 - gone)),
        (m.fields[1].1, enc(m.entries[1].1 - gone)),
        (m.fields[2].1, enc(m.entries[2].1 - gone)),
    ];
    patches.push((exif_at, expected(&l, &del_for(&l, cats))));
    assert_eq!(
        out.bytes(),
        &expected_bytes(&s.bytes, &[com_range], &patches)[..]
    );
    // Three MPF fields, plus the Exif patches; the kinds say which is which.
    let kinds = |k: PatchKind| out.patches().iter().filter(|p| p.kind == k).count();
    assert_eq!(kinds(PatchKind::MpfField), 3);
    assert!(kinds(PatchKind::IfdTable) >= 2);
    assert!(kinds(PatchKind::Scrub) >= 3);
    let a = audit(out.bytes());
    assert_eq!(a.findings_in(Category::EmbeddedImage).count(), 2);
    assert_eq!(a.health().jpeg_issues, 0);
}

#[test]
fn a_com_segment_and_an_exif_entry_go_in_the_same_call() {
    let l = exif_layout::full_le().layout();
    let j = jpeg_of(std::slice::from_ref(&l), true);
    let cats: &[Cat] = &["gps", "free_text"];
    let out = strip(&j.bytes, only(cats)).unwrap();
    assert_eq!(out.bytes(), &expected_jpeg(&j, cats)[..]);
    assert!(!contains(out.bytes(), strip_shapes::C_COM1));
    assert!(!contains(out.bytes(), exif_layout::K_GPSAREA));
    assert_eq!(out.removals().len(), 1);
    assert_eq!(out.removals()[0].kind, RemovalKind::Comment);
    // The Exif patches are in output coordinates: the COM is before the Exif.
    let com = j.com.clone().unwrap().len();
    let first_patch = out.patches()[0].offset;
    assert!(first_patch >= j.blobs[0] - com);
    assert_eq!(out.bytes().len(), j.bytes.len() - com);
}

// ── the leak inputs (SPEC-021): each refuses or is correct, never Ok with a
// canary left ──────────────────────────────────────────────────────────────

struct Leak {
    name: &'static str,
    layout: Layout,
    /// The policy (category names) the refusal is pinned for.
    cats: &'static [Cat],
    reason: UneditableReason,
    /// The blob offset of the fault, if the test pins one.
    at: Option<usize>,
}

fn leak_inputs() -> Vec<Leak> {
    let note = |l: &Layout| {
        l.ifd("exif")
            .ents
            .iter()
            .find(|e| e.tag == 0x927C)
            .unwrap()
            .value
            .clone()
            .unwrap()
            .start
    };
    let sw_value = |l: &Layout| {
        l.ifd("ifd0")
            .ents
            .iter()
            .find(|e| e.tag == 0x0131)
            .unwrap()
            .value
            .clone()
            .unwrap()
            .start
    };
    let strips = exif_layout::thumbnail_strips_form().layout();
    let ifd1 = strips.ifd("ifd1").table.start;
    let l1 = exif_layout::value_shared_with_kept_entry().layout();
    let l2 = exif_layout::value_inside_kept_ifd_table().layout();
    let l3 = exif_layout::value_overlaps_tiff_header().layout();
    let l4 = exif_layout::thumbnail_range_overlaps_makernote().layout();
    vec![
        Leak {
            name: "gps_ifd_pointed_to_twice",
            layout: exif_layout::gps_ifd_pointed_to_twice().layout(),
            cats: &["gps"],
            reason: UneditableReason::HiddenPointer,
            at: None,
        },
        Leak {
            name: "gps_pointer_missing_in_parent",
            layout: exif_layout::gps_pointer_hidden_behind_a_zero().layout(),
            cats: &["gps"],
            reason: UneditableReason::HiddenPointer,
            at: None,
        },
        Leak {
            name: "value_shared_with_kept_entry",
            at: Some(sw_value(&l1)),
            layout: l1,
            cats: &["camera_model"],
            reason: UneditableReason::SharedBytes,
        },
        Leak {
            name: "value_inside_kept_ifd_table",
            layout: l2,
            cats: &["camera_model"],
            reason: UneditableReason::SharedBytes,
            at: Some(40),
        },
        Leak {
            name: "value_overlaps_tiff_header",
            layout: l3,
            cats: &["camera_model"],
            reason: UneditableReason::SharedBytes,
            at: Some(4),
        },
        Leak {
            name: "thumbnail_range_overlaps_makernote",
            at: Some(note(&l4)),
            layout: l4,
            cats: &["thumbnail"],
            reason: UneditableReason::SharedBytes,
        },
        Leak {
            name: "thumbnail_strips_form",
            layout: strips,
            cats: &["thumbnail"],
            reason: UneditableReason::ThumbnailNotLocated,
            at: Some(ifd1),
        },
        Leak {
            name: "thumbnail_with_ifd2_after_it",
            layout: exif_layout::thumbnail_with_ifd2_after_it().layout(),
            cats: &["thumbnail"],
            reason: UneditableReason::ChainAfterThumbnail,
            at: None,
        },
        Leak {
            name: "exif_walk_issue_in_a_kept_ifd",
            layout: exif_layout::walk_issue_in_a_kept_ifd().layout(),
            cats: &["gps"],
            reason: UneditableReason::BlockNotClean,
            at: None,
        },
    ]
}

#[test]
fn leak_inputs_refuse_with_the_named_reason_and_offset() {
    for leak in leak_inputs() {
        let j = jpeg_of(std::slice::from_ref(&leak.layout), false);
        let err = strip(&j.bytes, only(leak.cats)).unwrap_err();
        let StripError::Uneditable { reason, offset } = err else {
            panic!("{}: {err:?}", leak.name)
        };
        assert_eq!(reason, leak.reason, "{}", leak.name);
        if let Some(at) = leak.at {
            assert_eq!(offset, j.blobs[0] + at, "{}", leak.name);
        }
    }
}

#[test]
fn no_leak_input_is_ok_with_a_canary_left_under_any_policy() {
    let mut refused = 0;
    let mut ok = 0;
    for leak in leak_inputs() {
        let j = jpeg_of(std::slice::from_ref(&leak.layout), false);
        for (pname, cats, p) in exif_policies() {
            match strip(&j.bytes, p) {
                Err(_) => refused += 1,
                Ok(out) => {
                    ok += 1;
                    // Whatever the policy removes by category is gone.
                    let d = del_for(&leak.layout, cats);
                    for f in &leak.layout.ifds {
                        for (i, e) in f.ents.iter().enumerate() {
                            let removed =
                                d.whole.contains(&f.name) || d.entries.contains(&(f.name, i));
                            if let (true, Some(c)) = (removed, &e.canary) {
                                assert!(
                                    !contains(out.bytes(), c),
                                    "{} {pname}: {} {:#x} left",
                                    leak.name,
                                    f.name,
                                    e.tag
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(refused > 40 && ok > 0, "{refused} {ok}");
    // The existing leak input of SPEC-020.
    let s = shape("gps-count-overflow");
    for (_, _, p) in exif_policies() {
        assert!(strip(&s.bytes, p).is_err());
    }
}
