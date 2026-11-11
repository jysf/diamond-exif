//! Plans built by hand and wrong on purpose: each one is applied with
//! [`EditPlan`] and the result is handed to [`verify`] as if the planner had
//! produced it (SPEC-020).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::arithmetic_side_effects
)]

use super::*;
use crate::write::EditPlan;

/// The SPEC-020 checks, with no zeroed range to check.
fn verify(input: &[u8], output: &[u8], spec: StripSpec) -> Result<(), Check> {
    super::verify(input, output, spec, &[])
}

fn seg(marker: u8, payload: &[u8]) -> Vec<u8> {
    let mut v = vec![0xFF, marker];
    v.extend_from_slice(&u16::try_from(payload.len() + 2).unwrap().to_be_bytes());
    v.extend_from_slice(payload);
    v
}

fn xmp() -> Vec<u8> {
    seg(0xE1, b"http://ns.adobe.com/xap/1.0/\0<x:xmpmeta/>")
}

fn icc() -> Vec<u8> {
    seg(0xE2, b"ICC_PROFILE\0\x01\x01profile bytes")
}

fn com() -> Vec<u8> {
    seg(0xFE, b"a comment")
}

fn jfif() -> Vec<u8> {
    seg(0xE0, b"JFIF\0\x01\x01\0\0\x01\0\x01\0\0")
}

fn jpeg(segs: &[Vec<u8>]) -> Vec<u8> {
    let mut d = vec![0xFF, 0xD8];
    for s in segs {
        d.extend_from_slice(s);
    }
    d.extend_from_slice(&seg(0xDA, &[1, 1, 0, 0, 0x3F, 0]));
    d.extend_from_slice(&[0x12, 0x34, 0x56, 0x78, 0xFF, 0xD9]);
    d
}

fn at(hay: &[u8], needle: &[u8]) -> usize {
    hay.windows(needle.len()).position(|w| w == needle).unwrap()
}

fn apply(input: &[u8], f: impl FnOnce(&mut EditPlan)) -> Vec<u8> {
    let mut plan = EditPlan::new();
    f(&mut plan);
    plan.apply(input).unwrap()
}

fn spec(cats: &[Category]) -> StripSpec {
    cats.iter().fold(StripSpec::none(), |s, c| s.include(*c))
}

#[test]
fn a_plan_that_leaves_a_removed_category_fails_check_preview() {
    let input = jpeg(&[jfif(), xmp(), com()]);
    // An empty plan for a policy that removes XMP.
    let out = apply(&input, |_| {});
    assert_eq!(
        verify(&input, &out, spec(&[Category::Xmp])),
        Err(Check::Preview)
    );
}

#[test]
fn a_plan_that_removes_a_kept_finding_fails_check_kept() {
    let input = jpeg(&[jfif(), xmp(), icc()]);
    let (x, i) = (at(&input, &xmp()), at(&input, &icc()));
    // XMP is the policy's; the ICC chunk goes too, by mistake.
    let out = apply(&input, |p| {
        p.remove(x..x + xmp().len());
        p.remove(i..i + icc().len());
    });
    assert_eq!(
        verify(&input, &out, spec(&[Category::Xmp])),
        Err(Check::Kept)
    );
}

#[test]
fn a_plan_that_cuts_the_scan_fails_check_scan() {
    let input = jpeg(&[jfif(), xmp()]);
    let x = at(&input, &xmp());
    let scan_byte = at(&input, &[0x12, 0x34, 0x56, 0x78]) + 1;
    let out = apply(&input, |p| {
        p.remove(x..x + xmp().len());
        p.remove(scan_byte..scan_byte + 1);
    });
    assert_eq!(
        verify(&input, &out, spec(&[Category::Xmp])),
        Err(Check::Scan)
    );
    // The same length, one byte of the scan changed: a length comparison
    // would miss it.
    let out = apply(&input, |p| {
        p.remove(x..x + xmp().len());
        p.patch(scan_byte, &[0xEE]);
    });
    assert_eq!(
        verify(&input, &out, spec(&[Category::Xmp])),
        Err(Check::Scan)
    );
}

#[test]
fn a_plan_that_breaks_the_walk_fails_check_walk() {
    let input = jpeg(&[jfif(), xmp()]);
    let x = at(&input, &xmp());
    // Turn the JFIF marker `FF E0` into `FF 01` (a standalone marker): the
    // findings and the scan are the same, but the walk now reports stray
    // bytes, a kind of issue the input did not have.
    let out = apply(&input, |p| {
        p.remove(x..x + xmp().len());
        p.patch(3, &[0x01]);
    });
    assert_eq!(
        verify(&input, &out, spec(&[Category::Xmp])),
        Err(Check::Walk)
    );
}

#[test]
fn a_non_idempotent_result_fails_check_idempotent() {
    // An extended-XMP segment whose header is too short is no region (the
    // audit has no finding for it) but it is still an extended-XMP segment
    // that the planner removes: an empty plan passes (a) to (d) and fails (e).
    let short = seg(0xE1, b"http://ns.adobe.com/xmp/extension/\0short");
    let input = jpeg(&[jfif(), short]);
    let out = apply(&input, |_| {});
    assert_eq!(
        verify(&input, &out, spec(&[Category::Xmp])),
        Err(Check::Idempotent)
    );
}

/// An extended-XMP segment with a well-formed header (32-byte GUID, full length,
/// offset) and `data`.
fn ext_xmp(data: &[u8]) -> Vec<u8> {
    let mut p = b"http://ns.adobe.com/xmp/extension/\0".to_vec();
    p.extend_from_slice(&[b'a'; 32]);
    p.extend_from_slice(&u32::try_from(data.len()).unwrap().to_be_bytes());
    p.extend_from_slice(&0u32.to_be_bytes());
    p.extend_from_slice(data);
    seg(0xE1, &p)
}

#[test]
fn v05_self_check_b_compares_the_region_kind() {
    // An XMP and an extended-XMP segment are both category Xmp. The plan turns
    // the extended one into a plain XMP segment (patch the length word and the
    // signature, remove the six bytes by which the signatures differ): the
    // categories and their counts are the same, only the region kinds differ.
    let input = jpeg(&[jfif(), xmp(), ext_xmp(b"<x/>")]);
    let e = at(&input, &ext_xmp(b"<x/>"));
    let old_len = u16::from_be_bytes([input[e + 2], input[e + 3]]);
    let sig = b"http://ns.adobe.com/xap/1.0/\0";
    let out = apply(&input, |p| {
        p.patch(e + 2, &(old_len - 6).to_be_bytes());
        p.patch(e + 4, sig);
        p.remove(e + 4 + sig.len()..e + 4 + 35);
    });
    // Control: the output reads as two Xmp regions and nothing is wrong with it.
    let m = crate::jpeg::read(&out).unwrap();
    assert!(m
        .regions()
        .iter()
        .all(|r| matches!(r, crate::jpeg::Region::Xmp { .. })));
    assert_eq!(
        verify(&input, &out, spec(&[Category::FreeText])),
        Err(Check::Kept)
    );
}

#[test]
fn v10_mpf_fields_compares_the_output_images_with_the_input_images() {
    // Remove the COM after the index and patch ONLY the primary's size: the
    // primary is right, the entry count is the same, but the secondary offset
    // is stale, so the image the index finds is not the input's image.
    let input = mpo();
    let c = at(&input, &com());
    let size_at = 10 + (8 + 2 + 24 + 4) + 4;
    let word = u32::from_le_bytes(input[size_at..size_at + 4].try_into().unwrap());
    let gone = u32::try_from(com().len()).unwrap();
    let out = apply(&input, |p| {
        p.remove(c..c + com().len());
        p.patch(size_at, &(word - gone).to_le_bytes());
    });
    assert_eq!(
        verify(&input, &out, spec(&[Category::FreeText])),
        Err(Check::MpfFields)
    );
}

/// A little-endian MPF segment with `entries` as `(size, offset)`, entry 0
/// first.
fn mpf(entries: &[(u32, u32)]) -> Vec<u8> {
    let mut t = b"II*\0".to_vec();
    t.extend_from_slice(&8u32.to_le_bytes());
    t.extend_from_slice(&2u16.to_le_bytes());
    t.extend_from_slice(&0xB001u16.to_le_bytes());
    t.extend_from_slice(&4u16.to_le_bytes());
    t.extend_from_slice(&1u32.to_le_bytes());
    t.extend_from_slice(&u32::try_from(entries.len()).unwrap().to_le_bytes());
    t.extend_from_slice(&0xB002u16.to_le_bytes());
    t.extend_from_slice(&7u16.to_le_bytes());
    t.extend_from_slice(&u32::try_from(16 * entries.len()).unwrap().to_le_bytes());
    t.extend_from_slice(&(8u32 + 2 + 24 + 4).to_le_bytes());
    t.extend_from_slice(&0u32.to_le_bytes());
    for (i, (size, offset)) in entries.iter().enumerate() {
        let attr: u32 = if i == 0 { 0x0003_0000 } else { 0 };
        t.extend_from_slice(&attr.to_le_bytes());
        t.extend_from_slice(&size.to_le_bytes());
        t.extend_from_slice(&offset.to_le_bytes());
        t.extend_from_slice(&[0, 0, 0, 0]);
    }
    seg(0xE2, &[&b"MPF\0"[..], &t].concat())
}

/// A primary (SOI, MPF, COM, scan, EOI) and one secondary image behind it, the
/// index pointing at it truthfully.
fn mpo() -> Vec<u8> {
    let image = jpeg(&[com()]);
    let build = |entries: &[(u32, u32)]| jpeg(&[mpf(entries), com()]);
    let probe = build(&[(0, 0), (0, 0)]);
    // The MPF header starts after `FF D8` and `FF E2 len len MPF\0`.
    let header = 2 + 8;
    let entries = [
        (u32::try_from(probe.len()).unwrap(), 0u32),
        (
            u32::try_from(image.len()).unwrap(),
            u32::try_from(probe.len() - header).unwrap(),
        ),
    ];
    let mut d = build(&entries);
    d.extend_from_slice(&image);
    d
}

#[test]
fn a_stale_mpf_field_fails_check_mpf_fields() {
    let input = mpo();
    let c = at(&input, &com());
    // Remove the COM after the index (the policy's FreeText) but do not patch
    // the index: the secondary offset now points past its image.
    let out = apply(&input, |p| p.remove(c..c + com().len()));
    assert_eq!(
        verify(&input, &out, spec(&[Category::FreeText])),
        Err(Check::MpfFields)
    );
}

#[test]
fn a_clean_plan_passes_every_check() {
    let input = jpeg(&[jfif(), xmp(), com(), icc()]);
    let (x, c) = (at(&input, &xmp()), at(&input, &com()));
    let out = apply(&input, |p| {
        p.remove(x..x + xmp().len());
        p.remove(c..c + com().len());
    });
    let s = spec(&[Category::Xmp, Category::FreeText]);
    assert_eq!(verify(&input, &out, s), Ok(()));
    // The same with the Multi-Picture index patched by hand.
    let input = mpo();
    let c = at(&input, &com());
    let out = apply(&input, |p| {
        p.remove(c..c + com().len());
        // Entry 0's size sits 4 bytes into the entries; the secondary's offset
        // 8 bytes into entry 1.
        let size_at = 10 + (8 + 2 + 24 + 4) + 4;
        let off_at = size_at + 16 + 4;
        let word = |at: usize| u32::from_le_bytes(input[at..at + 4].try_into().unwrap());
        let gone = u32::try_from(com().len()).unwrap();
        p.patch(size_at, &(word(size_at) - gone).to_le_bytes());
        p.patch(off_at, &(word(off_at) - gone).to_le_bytes());
    });
    assert_eq!(verify(&input, &out, spec(&[Category::FreeText])), Ok(()));
}

// ── the Exif entry checks (SPEC-021) ────────────────────────────────────────

/// A little-endian TIFF blob: IFD0 at 8 with Make (out of line at 66),
/// Orientation (inline), Software (out of line at 72) and Artist (at 79);
/// table 8..62.
fn exif_blob() -> Vec<u8> {
    let mut b = b"II*\0\x08\0\0\0".to_vec();
    b.extend_from_slice(&4u16.to_le_bytes());
    let mut ent = |tag: u16, ty: u16, count: u32, val: [u8; 4]| {
        b.extend_from_slice(&tag.to_le_bytes());
        b.extend_from_slice(&ty.to_le_bytes());
        b.extend_from_slice(&count.to_le_bytes());
        b.extend_from_slice(&val);
    };
    ent(0x010F, 2, 6, 66u32.to_le_bytes());
    ent(0x0112, 3, 1, [6, 0, 0, 0]);
    ent(0x0131, 2, 7, 72u32.to_le_bytes());
    ent(0x013B, 2, 7, 79u32.to_le_bytes());
    b.extend_from_slice(&[0; 4]);
    b.resize(66, 0);
    b.extend_from_slice(b"MAKE1\0SOFTW1\0ARTIS1\0");
    b
}

/// The JPEG holding `exif_blob`, and the input offset of the blob.
fn exif_input() -> (Vec<u8>, usize) {
    let input = jpeg(&[
        jfif(),
        seg(0xE1, &[&b"Exif\0\0"[..], &exif_blob()].concat()),
    ]);
    let blob_at = at(&input, b"II*\0");
    (input, blob_at)
}

/// The output of a plan that deletes IFD0 entries `gone` (and zeroes the
/// values at `scrub`, blob offsets), with `tweak` applied to the new table.
fn exif_output(
    input: &[u8],
    blob_at: usize,
    gone: &[usize],
    scrub: Option<std::ops::Range<usize>>,
    tweak: impl Fn(&mut Vec<u8>),
) -> Vec<u8> {
    let entry = |i: usize| input[blob_at + 10 + 12 * i..blob_at + 22 + 12 * i].to_vec();
    let mut table = Vec::new();
    let kept: Vec<usize> = (0..4).filter(|i| !gone.contains(i)).collect();
    table.extend_from_slice(&u16::try_from(kept.len()).unwrap().to_le_bytes());
    for i in &kept {
        table.extend_from_slice(&entry(*i));
    }
    table.extend_from_slice(&[0; 4]);
    table.resize(54, 0);
    tweak(&mut table);
    apply(input, |p| {
        p.patch(blob_at + 8, &table);
        if let Some(r) = scrub {
            p.patch(blob_at + r.start, &vec![0; r.len()]);
        }
    })
}

fn make_spec() -> StripSpec {
    spec(&[Category::CameraModel])
}

#[test]
fn a_clean_exif_plan_passes_every_check_including_the_two_new_ones() {
    let (input, at) = exif_input();
    let out = exif_output(&input, at, &[0], Some(66..72), |_| {});
    assert_eq!(
        super::verify(&input, &out, make_spec(), &[(at + 66, 6)]),
        Ok(())
    );
}

#[test]
fn a_plan_that_corrupts_a_kept_entry_value_fails_check_entries() {
    let (input, at) = exif_input();
    // Orientation is the first survivor: its inline value sits at 2 + 8.
    let out = exif_output(&input, at, &[0], Some(66..72), |t| t[10] = 3);
    assert_eq!(
        super::verify(&input, &out, make_spec(), &[(at + 66, 6)]),
        Err(Check::Entries)
    );
}

#[test]
fn a_plan_that_deletes_a_kept_entry_fails_check_entries() {
    let (input, at) = exif_input();
    // The Orientation entry (no category) goes with the Make.
    let out = exif_output(&input, at, &[0, 1], Some(66..72), |_| {});
    assert_eq!(
        super::verify(&input, &out, make_spec(), &[(at + 66, 6)]),
        Err(Check::Entries)
    );
}

#[test]
fn a_plan_that_deletes_a_kept_exif_finding_fails_check_kept() {
    // The Artist (Authorship, not the policy's) goes with the Make: the audit
    // sees a kept finding lost. The exemption of check (b) is for a removed
    // Exif segment only, never for an entry edit.
    let (input, at) = exif_input();
    let out = exif_output(&input, at, &[0, 3], Some(66..72), |_| {});
    assert_eq!(
        super::verify(&input, &out, make_spec(), &[(at + 66, 6)]),
        Err(Check::Kept)
    );
}

#[test]
fn l1_scrubbed_checks_every_range_not_only_the_first() {
    let (input, at) = exif_input();
    let out = exif_output(&input, at, &[0], Some(66..72), |_| {});
    // The first range is zero; the second is the (kept) Software value, which
    // is not. Only a check that reads past the first range sees it.
    assert_eq!(
        super::verify(&input, &out, make_spec(), &[(at + 66, 6), (at + 72, 7)]),
        Err(Check::Scrubbed)
    );
    // Control: the first range alone passes.
    assert_eq!(
        super::verify(&input, &out, make_spec(), &[(at + 66, 6)]),
        Ok(())
    );
}

#[test]
fn a_plan_that_leaves_a_deleted_entry_fails_check_preview() {
    let (input, at) = exif_input();
    let out = exif_output(&input, at, &[], None, |_| {});
    assert_eq!(out, input);
    assert_eq!(
        super::verify(&input, &out, make_spec(), &[]),
        Err(Check::Preview)
    );
}

#[test]
fn a_scrub_that_leaves_a_byte_fails_check_scrubbed() {
    let (input, at) = exif_input();
    // The value of the deleted Make is zeroed except its first byte.
    let out = exif_output(&input, at, &[0], Some(67..72), |_| {});
    assert_eq!(
        super::verify(&input, &out, make_spec(), &[(at + 66, 6)]),
        Err(Check::Scrubbed)
    );
    // Without the recorded range nothing notices (the check is the range).
    assert_eq!(super::verify(&input, &out, make_spec(), &[]), Ok(()));
    // A range that is not inside the output is a failure too.
    assert_eq!(
        super::verify(&input, &out, make_spec(), &[(out.len() - 2, 9)]),
        Err(Check::Scrubbed)
    );
}

#[test]
fn entries_check_compares_type_count_and_value_not_only_the_tag() {
    let (input, at) = exif_input();
    // Each change keeps the tag and the walk; the Orientation entry is the
    // first survivor: type at 2 + 2, count at 2 + 4, value at 2 + 8.
    type Tweak = fn(&mut Vec<u8>);
    let tweaks: [(&str, Tweak); 4] = [
        ("type", |t| t[4] = 8),
        ("count", |t| t[6] = 2),
        ("value", |t| t[10] = 7),
        ("out-of-line value", |t| t[2 + 24 + 8] ^= 1),
    ];
    for (what, tweak) in tweaks {
        let out = exif_output(&input, at, &[0], Some(66..72), tweak);
        assert_eq!(
            super::verify(&input, &out, make_spec(), &[(at + 66, 6)]),
            Err(Check::Entries),
            "{what}"
        );
    }
}

#[test]
fn an_empty_plan_on_an_input_with_targets_is_still_refused() {
    let (input, _) = exif_input();
    let s = make_spec();
    assert_eq!(
        crate::write::finish(
            &input,
            s,
            crate::write::jpeg::Planned::empty(),
            crate::write::Format::Jpeg
        )
        .unwrap_err(),
        crate::write::StripError::VerifyFailed {
            check: Check::Preview
        }
    );
    // The real planner deletes the entry.
    assert!(crate::write::strip(&input, crate::StripPolicy::Custom(s)).is_ok());
}

#[test]
fn the_seven_old_checks_still_pass_and_fail_as_before() {
    // One wrong plan per SPEC-020 check, with the check it must trip; the
    // order of the checks is the documented one (the new ones come after Walk
    // and before Idempotent).
    let xmp_in = jpeg(&[jfif(), xmp(), com()]);
    let none = apply(&xmp_in, |_| {});
    assert_eq!(
        verify(&xmp_in, &none, spec(&[Category::Xmp])),
        Err(Check::Preview)
    );
    let two = jpeg(&[jfif(), xmp(), icc()]);
    let (x, i) = (at(&two, &xmp()), at(&two, &icc()));
    let both = apply(&two, |p| {
        p.remove(x..x + xmp().len());
        p.remove(i..i + icc().len());
    });
    assert_eq!(
        verify(&two, &both, spec(&[Category::Xmp])),
        Err(Check::Kept)
    );
    let scan_byte = at(&xmp_in, &[0x12, 0x34, 0x56, 0x78]) + 1;
    let x = at(&xmp_in, &xmp());
    let cut = apply(&xmp_in, |p| {
        p.remove(x..x + xmp().len());
        p.patch(scan_byte, &[0xEE]);
    });
    assert_eq!(
        verify(&xmp_in, &cut, spec(&[Category::Xmp])),
        Err(Check::Scan)
    );
    let broken = apply(&xmp_in, |p| {
        p.remove(x..x + xmp().len());
        p.patch(3, &[0x01]);
    });
    assert_eq!(
        verify(&xmp_in, &broken, spec(&[Category::Xmp])),
        Err(Check::Walk)
    );
    let short = seg(0xE1, b"http://ns.adobe.com/xmp/extension/\0short");
    let short_in = jpeg(&[jfif(), short]);
    let unchanged = apply(&short_in, |_| {});
    assert_eq!(
        verify(&short_in, &unchanged, spec(&[Category::Xmp])),
        Err(Check::Idempotent)
    );
    let stale = mpo();
    let c = at(&stale, &com());
    let moved = apply(&stale, |p| p.remove(c..c + com().len()));
    assert_eq!(
        verify(&stale, &moved, spec(&[Category::FreeText])),
        Err(Check::MpfFields)
    );
    let ok = jpeg(&[jfif(), xmp()]);
    let x = at(&ok, &xmp());
    let clean = apply(&ok, |p| p.remove(x..x + xmp().len()));
    assert_eq!(verify(&ok, &clean, spec(&[Category::Xmp])), Ok(()));
}

/// An empty little-endian TIFF: header, an IFD0 with no entries, no next IFD.
fn empty_tiff() -> Vec<u8> {
    let mut v = b"II\x2A\0\x08\0\0\0".to_vec();
    v.extend_from_slice(&[0; 6]);
    v
}

#[test]
fn verify_tiff_accepts_the_identity_and_rejects_a_changed_header() {
    let input = empty_tiff();
    assert_eq!(verify_tiff(&input, &input, StripSpec::none(), &[]), Ok(()));
    // Each header byte: the check that fails is the header compare (Scan), not
    // whichever later check the changed byte happens to break.
    for at in 0..8 {
        let mut out = input.clone();
        out[at] ^= 0x01;
        assert_eq!(
            verify_tiff(&input, &out, StripSpec::none(), &[]),
            Err(Check::Scan),
            "header byte {at}"
        );
    }
}
