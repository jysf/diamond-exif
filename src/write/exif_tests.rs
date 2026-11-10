//! Unit tests of the Exif entry planner (SPEC-021): one hand-built TIFF blob
//! per test, laid out at explicit offsets so every expected byte is written
//! here, not derived from the walk or from the planner.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::arithmetic_side_effects
)]

use super::*;

// ── a tiny blob builder ─────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct E {
    tag: u16,
    ty: u16,
    count: u32,
    val: [u8; 4],
}

struct Blob {
    be: bool,
    bytes: Vec<u8>,
}

impl Blob {
    fn new(be: bool, ifd0: u32) -> Blob {
        let mut b = Blob {
            be,
            bytes: Vec::new(),
        };
        b.data(0, if be { b"MM" } else { b"II" });
        let v = b.w16(42);
        b.data(2, &v);
        let o = b.w32(ifd0);
        b.data(4, &o);
        b
    }
    fn w16(&self, v: u16) -> [u8; 2] {
        if self.be {
            v.to_be_bytes()
        } else {
            v.to_le_bytes()
        }
    }
    fn w32(&self, v: u32) -> [u8; 4] {
        if self.be {
            v.to_be_bytes()
        } else {
            v.to_le_bytes()
        }
    }
    fn data(&mut self, at: usize, bytes: &[u8]) {
        if self.bytes.len() < at + bytes.len() {
            self.bytes.resize(at + bytes.len(), 0);
        }
        self.bytes[at..at + bytes.len()].copy_from_slice(bytes);
    }
    /// Entry with an out-of-line value at `off`.
    fn out(&self, tag: u16, ty: u16, count: u32, off: u32) -> E {
        E {
            tag,
            ty,
            count,
            val: self.w32(off),
        }
    }
    /// A `SHORT` entry, value inline.
    fn short(&self, tag: u16, v: u16) -> E {
        let w = self.w16(v);
        E {
            tag,
            ty: 3,
            count: 1,
            val: [w[0], w[1], 0, 0],
        }
    }
    /// An inline `ASCII` entry (at most 3 characters and the NUL).
    fn inl(&self, tag: u16, text: &[u8]) -> E {
        let mut val = [0u8; 4];
        val[..text.len()].copy_from_slice(text);
        E {
            tag,
            ty: 2,
            count: u32::try_from(text.len()).unwrap(),
            val,
        }
    }
    /// A `LONG` entry whose value is `v` (a pointer or a length).
    fn long(&self, tag: u16, v: u32) -> E {
        E {
            tag,
            ty: 4,
            count: 1,
            val: self.w32(v),
        }
    }
    fn table(&self, es: &[E], next: u32) -> Vec<u8> {
        let mut v = self.w16(u16::try_from(es.len()).unwrap()).to_vec();
        for e in es {
            v.extend_from_slice(&self.w16(e.tag));
            v.extend_from_slice(&self.w16(e.ty));
            v.extend_from_slice(&self.w32(e.count));
            v.extend_from_slice(&e.val);
        }
        v.extend_from_slice(&self.w32(next));
        v
    }
    fn ifd(&mut self, at: usize, es: &[E], next: u32) {
        let t = self.table(es, next);
        self.data(at, &t);
    }
}

fn zeros(n: usize) -> Vec<u8> {
    vec![0; n]
}

/// Run the planner over `blob` with the walk of `walked` (usually the same
/// bytes).
fn run_on(blob: &[u8], walked: &[u8], sel: &Selection) -> Result<BlockPlan, Refusal> {
    let c = Container::parse(walked).unwrap();
    assert!(c.is_clean(), "fixture must walk clean: {:?}", c.issues());
    plan_block(blob, &c, sel)
}

fn run(blob: &[u8], sel: &Selection) -> Result<BlockPlan, Refusal> {
    run_on(blob, blob, sel)
}

fn del(entries: &[(usize, usize)]) -> Selection {
    Selection {
        deleted: entries.to_vec(),
        ..Selection::default()
    }
}

/// Apply a plan the way `EditPlan` would: overwrite in place.
fn apply(blob: &[u8], plan: &BlockPlan) -> Vec<u8> {
    let mut out = blob.to_vec();
    for p in &plan.patches {
        out[p.at..p.at + p.bytes.len()].copy_from_slice(&p.bytes);
    }
    out
}

// ── fixture A: IFD0 with five entries ───────────────────────────────────────

const MAKE: &[u8] = b"MAKE1\0";
const MODEL: &[u8] = b"MODL2\0";
const ARTIST: &[u8] = b"ARTIST\0";

/// IFD0 at 8: Make (out of line at 74), Model (80), Orientation (inline),
/// Software (inline, 3 bytes), Artist (86); table 8..74; next word `next`.
fn base_entries(b: &Blob) -> Vec<E> {
    vec![
        b.out(0x010F, 2, 6, 74),
        b.out(0x0110, 2, 6, 80),
        b.short(0x0112, 6),
        b.inl(0x0131, b"ab\0"),
        b.out(0x013B, 2, 7, 86),
    ]
}

fn base_with(be: bool, es: &dyn Fn(&Blob) -> Vec<E>, next: u32) -> Blob {
    let mut b = Blob::new(be, 8);
    let es = es(&b);
    b.ifd(8, &es, next);
    b.data(74, MAKE);
    b.data(80, MODEL);
    b.data(86, ARTIST);
    b
}

fn base(be: bool) -> Blob {
    base_with(be, &base_entries, 0)
}

/// What the table region must read after deleting the entries at `gone`: the
/// survivors in order, the next word, `12 * k` zero bytes; built from the
/// entry list, not from the planner.
fn expected_table(b: &Blob, all: &[E], gone: &[usize], next: u32) -> Vec<u8> {
    let kept: Vec<E> = all
        .iter()
        .enumerate()
        .filter(|(i, _)| !gone.contains(i))
        .map(|(_, e)| *e)
        .collect();
    let mut t = b.table(&kept, next);
    t.extend_from_slice(&zeros(12 * gone.len()));
    t
}

#[test]
fn delete_a_middle_entry_shifts_and_zeroes_the_tail() {
    for be in [false, true] {
        let b = base(be);
        let plan = run(&b.bytes, &del(&[(0, 1)])).unwrap();
        let mut want = b.bytes.clone();
        want[8..74].copy_from_slice(&expected_table(&b, &base_entries(&b), &[1], 0));
        want[80..86].copy_from_slice(&zeros(6));
        assert_eq!(apply(&b.bytes, &plan), want, "be={be}");
        assert_eq!(plan.deleted.len(), 1);
        assert_eq!(plan.deleted[0].tag, 0x0110);
        assert_eq!(plan.deleted[0].at, 8 + 2 + 12);
        assert_eq!(plan.deleted[0].ifd, IfdKind::Ifd0);
        // One table patch and one scrub.
        let kinds: Vec<PatchKind> = plan.patches.iter().map(|p| p.kind).collect();
        assert_eq!(kinds, vec![PatchKind::IfdTable, PatchKind::Scrub]);
        assert_eq!(plan.patches[1].reason, Some(ScrubReason::EntryValue));
    }
}

#[test]
fn delete_the_first_entry() {
    let b = base(false);
    let plan = run(&b.bytes, &del(&[(0, 0)])).unwrap();
    let mut want = b.bytes.clone();
    want[8..74].copy_from_slice(&expected_table(&b, &base_entries(&b), &[0], 0));
    want[74..80].copy_from_slice(&zeros(6));
    assert_eq!(apply(&b.bytes, &plan), want);
}

#[test]
fn delete_the_last_entry() {
    let b = base(true);
    let plan = run(&b.bytes, &del(&[(0, 4)])).unwrap();
    let mut want = b.bytes.clone();
    want[8..74].copy_from_slice(&expected_table(&b, &base_entries(&b), &[4], 0));
    want[86..93].copy_from_slice(&zeros(7));
    assert_eq!(apply(&b.bytes, &plan), want);
}

#[test]
fn delete_two_non_adjacent_entries() {
    let b = base(false);
    // Make (0) and the inline Software (3).
    let plan = run(&b.bytes, &del(&[(0, 3), (0, 0)])).unwrap();
    let mut want = b.bytes.clone();
    want[8..74].copy_from_slice(&expected_table(&b, &base_entries(&b), &[0, 3], 0));
    want[74..80].copy_from_slice(&zeros(6));
    assert_eq!(apply(&b.bytes, &plan), want);
    // Recorded in input order, whatever the order of the selection.
    let tags: Vec<u16> = plan.deleted.iter().map(|d| d.tag).collect();
    assert_eq!(tags, vec![0x010F, 0x0131]);
}

#[test]
fn delete_every_entry_leaves_count_zero_or_refuses_per_the_measurement() {
    // Rule 9, as measured: ExifTool 13.55 reads an IFD left with no entry
    // (count 0) without a warning, prints no tag for it and still reports the
    // byte order (`emptied_ifd_behaviour_is_pinned` in tests/strip_oracle.rs),
    // and the walk calls it clean; so the planner allows it: count word 0, the
    // next word, zeros.
    let b = base(false);
    let plan = run(&b.bytes, &del(&[(0, 0), (0, 1), (0, 2), (0, 3), (0, 4)])).unwrap();
    let out = apply(&b.bytes, &plan);
    assert_eq!(&out[8..10], &[0, 0]);
    assert!(out[10..74].iter().all(|&x| x == 0));
    assert!(out[74..93].iter().all(|&x| x == 0));
}

#[test]
fn no_target_means_no_patch() {
    let b = base(false);
    let plan = run(&b.bytes, &Selection::default()).unwrap();
    assert!(plan.patches.is_empty());
    assert!(plan.deleted.is_empty());
    // A GPS or thumbnail request on a blob that has neither is also empty.
    let sel = Selection {
        gps: true,
        thumbnail: true,
        ..Selection::default()
    };
    assert!(run(&b.bytes, &sel).unwrap().patches.is_empty());
}

#[test]
fn next_ifd_word_follows_the_surviving_entries() {
    for be in [false, true] {
        let mut b = base_with(be, &base_entries, 93);
        let ifd1 = [b.short(0x0112, 1)];
        b.ifd(93, &ifd1, 0);
        let plan = run(&b.bytes, &del(&[(0, 1)])).unwrap();
        let mut want = b.bytes.clone();
        want[8..74].copy_from_slice(&expected_table(&b, &base_entries(&b), &[1], 93));
        want[80..86].copy_from_slice(&zeros(6));
        assert_eq!(apply(&b.bytes, &plan), want, "be={be}");
        // The next word now sits right after the four survivors.
        assert_eq!(&want[58..62], &b.w32(93));
        assert_eq!(&want[70..74], &[0, 0, 0, 0]);
    }
}

#[test]
fn inline_value_entry_leaves_no_trace_in_the_table() {
    let b = base(false);
    // The inline Orientation.
    let plan = run(&b.bytes, &del(&[(0, 2)])).unwrap();
    assert_eq!(plan.patches.len(), 1);
    assert_eq!(plan.patches[0].kind, PatchKind::IfdTable);
    let out = apply(&b.bytes, &plan);
    // The value 6 appears nowhere in the table any more (the inline byte
    // pattern `06 00` was in the entry's value field).
    let mut want = b.bytes.clone();
    want[8..74].copy_from_slice(&expected_table(&b, &base_entries(&b), &[2], 0));
    assert_eq!(out, want);
    assert!(!out[8..74].windows(4).any(|w| w == [0x12, 0x01, 0x03, 0x00]));
}

#[test]
fn out_of_line_value_is_zeroed() {
    let b = base(true);
    let plan = run(&b.bytes, &del(&[(0, 4)])).unwrap();
    let out = apply(&b.bytes, &plan);
    assert!(out[86..93].iter().all(|&x| x == 0));
    // The neighbours are untouched.
    assert_eq!(&out[74..80], MAKE);
    assert_eq!(&out[80..86], MODEL);
}

#[test]
fn two_deleted_entries_sharing_one_payload_scrub_it_once() {
    let mut b = Blob::new(false, 8);
    let es = vec![
        b.out(0x010F, 2, 6, 74),
        b.out(0x0110, 2, 6, 74),
        b.short(0x0112, 6),
    ];
    b.ifd(8, &es, 0);
    b.data(74, MAKE);
    // Table is 2 + 36 + 4 = 42 bytes: 8..50; the payload sits at 74.
    let plan = run(&b.bytes, &del(&[(0, 0), (0, 1)])).unwrap();
    let scrubs: Vec<_> = plan
        .patches
        .iter()
        .filter(|p| p.kind == PatchKind::Scrub)
        .collect();
    assert_eq!(scrubs.len(), 1);
    assert_eq!((scrubs[0].at, scrubs[0].bytes.len()), (74, 6));
    assert_eq!(plan.deleted.len(), 2);
}

#[test]
fn value_shared_with_a_kept_entry_is_shared_bytes() {
    let mut b = Blob::new(false, 8);
    let es = vec![
        b.out(0x010F, 2, 6, 74),
        // Deleted, but its value is the same bytes as the kept Make.
        b.out(0x0110, 2, 6, 74),
    ];
    b.ifd(8, &es, 0);
    b.data(74, MAKE);
    assert_eq!(
        run(&b.bytes, &del(&[(0, 1)])).unwrap_err(),
        Refusal {
            reason: UneditableReason::SharedBytes,
            at: 74
        }
    );
}

#[test]
fn value_inside_a_kept_ifd_table_is_shared_bytes() {
    let mut b = Blob::new(false, 8);
    // The deleted entry's value points into IFD0's own table (offset 20).
    let es = vec![b.out(0x010F, 2, 6, 74), b.out(0x0110, 2, 6, 20)];
    b.ifd(8, &es, 0);
    b.data(74, MAKE);
    assert_eq!(
        run(&b.bytes, &del(&[(0, 1)])).unwrap_err(),
        Refusal {
            reason: UneditableReason::SharedBytes,
            at: 20
        }
    );
}

#[test]
fn value_overlapping_the_tiff_header_is_shared_bytes() {
    let mut b = Blob::new(true, 8);
    let es = vec![b.out(0x010F, 2, 6, 74), b.out(0x0110, 2, 6, 2)];
    b.ifd(8, &es, 0);
    b.data(74, MAKE);
    assert_eq!(
        run(&b.bytes, &del(&[(0, 1)])).unwrap_err(),
        Refusal {
            reason: UneditableReason::SharedBytes,
            at: 2
        }
    );
}

#[test]
fn a_patched_entry_that_is_not_the_walks_entry_is_block_not_clean() {
    let b = base(false);
    let mut other = b.bytes.clone();
    // Change the tag of entry 1 after the walk: the planner must notice.
    other[8 + 2 + 12] ^= 0xFF;
    let err = run_on(&other, &b.bytes, &del(&[(0, 1)])).unwrap_err();
    assert_eq!(err.reason, UneditableReason::BlockNotClean);
    assert_eq!(err.at, 8 + 2 + 12);
    // The count and the type are compared too.
    for off in [2usize, 4] {
        let mut other = b.bytes.clone();
        other[8 + 2 + 12 + off] ^= 0x01;
        let err = run_on(&other, &b.bytes, &del(&[(0, 1)])).unwrap_err();
        assert_eq!(err.reason, UneditableReason::BlockNotClean, "off {off}");
    }
    // A selection that names an entry the walk does not have.
    assert_eq!(
        run(&b.bytes, &del(&[(0, 9)])).unwrap_err().reason,
        UneditableReason::BlockNotClean
    );
    assert_eq!(
        run(&b.bytes, &del(&[(3, 0)])).unwrap_err().reason,
        UneditableReason::BlockNotClean
    );
}

// ── fixture B: IFD0 with a GPS IFD ──────────────────────────────────────────

/// IFD0 at 8: Make (out of line at 66), Orientation, GPS pointer to 72; GPS IFD
/// at 72 (table 72..114): LatRef (inline), Lat (24 bytes at 114), Lon (24 at 138).
fn gps_blob(be: bool, extra_pointer: bool) -> (Blob, Vec<E>) {
    let mut b = Blob::new(be, 8);
    let mut es = vec![
        b.out(0x010F, 2, 6, 66),
        b.short(0x0112, 6),
        b.long(0x8825, 72),
    ];
    if extra_pointer {
        es.push(b.long(0x8825, 72));
    }
    b.ifd(8, &es, 0);
    b.data(66, MAKE);
    let gps = vec![
        b.inl(0x0001, b"N\0"),
        b.out(0x0002, 5, 3, 114),
        b.out(0x0004, 5, 3, 138),
    ];
    b.ifd(72, &gps, 0);
    // The canary rationals: 24 non-zero bytes each.
    b.data(114, &[0xA1; 24]);
    b.data(138, &[0xB2; 24]);
    (b, es)
}

fn gps_sel() -> Selection {
    Selection {
        gps: true,
        ..Selection::default()
    }
}

#[test]
fn gps_child_removal_deletes_the_pointer_and_zeroes_the_ifd_and_values() {
    for be in [false, true] {
        let (b, es) = gps_blob(be, false);
        let plan = run(&b.bytes, &gps_sel()).unwrap();
        let mut want = b.bytes.clone();
        // IFD0: the pointer (entry 2) is gone; table 8..50 keeps its length.
        want[8..50].copy_from_slice(&expected_table(&b, &es, &[2], 0));
        // The GPS table and both values are zero.
        want[72..162].copy_from_slice(&zeros(90));
        assert_eq!(apply(&b.bytes, &plan), want, "be={be}");
        // Records: the pointer, then the three GPS entries, in blob order.
        let tags: Vec<(IfdKind, u16)> = plan.deleted.iter().map(|d| (d.ifd, d.tag)).collect();
        assert_eq!(
            tags,
            vec![
                (IfdKind::Ifd0, 0x8825),
                (IfdKind::Gps, 0x0001),
                (IfdKind::Gps, 0x0002),
                (IfdKind::Gps, 0x0004)
            ]
        );
        let reasons: Vec<_> = plan.patches.iter().map(|p| p.reason).collect();
        assert!(reasons.contains(&Some(ScrubReason::IfdTable)));
        assert!(reasons.contains(&Some(ScrubReason::EntryValue)));
    }
}

#[test]
fn gps_ifd_pointed_to_twice_is_refused() {
    // Two pointer entries with the same target: the walk follows the first and
    // sees no issue; deleting one would leave the other.
    let (b, _) = gps_blob(false, true);
    let err = run(&b.bytes, &gps_sel()).unwrap_err();
    assert_eq!(err.reason, UneditableReason::HiddenPointer);
    assert_eq!(err.at, 8 + 2 + 3 * 12, "the second pointer entry");
}

#[test]
fn a_pointer_the_walk_does_not_follow_is_a_hidden_pointer() {
    // A second Exif pointer in IFD0, an Exif pointer inside the Exif IFD, a
    // GPS pointer inside the Exif IFD and an Interop pointer in IFD0: the walk
    // follows none of them.
    let (b, _) = note_blob(false);
    let sel = del(&[(0, 0)]);
    assert!(run(&b.bytes, &sel).is_ok());
    let hide = |ifd_at: usize, n: usize, tag: u16| {
        let mut other = b.bytes.clone();
        let tag_at = ifd_at + 2 + 12 * n;
        other[tag_at..tag_at + 2].copy_from_slice(&tag.to_le_bytes());
        (other, tag_at)
    };
    // IFD0's second entry (the Exif pointer) retagged as the Interop pointer.
    let (other, at) = hide(8, 1, 0xA005);
    let c = Container::parse(&other).unwrap();
    assert!(c.is_clean());
    let err = plan_block(&other, &c, &sel).unwrap_err();
    assert_eq!(err.reason, UneditableReason::HiddenPointer);
    assert_eq!(err.at, at);
    // The Exif IFD's first entry retagged as a GPS pointer (value 0: no IFD).
    let mut other = b.bytes.clone();
    let tag_at = 56 + 2;
    other[tag_at..tag_at + 2].copy_from_slice(&0x8825u16.to_le_bytes());
    other[tag_at + 8..tag_at + 12].copy_from_slice(&[0; 4]);
    let c = Container::parse(&other).unwrap();
    assert!(c.is_clean());
    let err = plan_block(&other, &c, &sel).unwrap_err();
    assert_eq!(err.reason, UneditableReason::HiddenPointer);
    assert_eq!(err.at, tag_at);
}

#[test]
fn gps_pointer_missing_from_the_parent_is_pointer_not_in_parent() {
    let (b, _) = gps_blob(false, false);
    // After the walk, the pointer entry (entry 2 of IFD0) stops naming the GPS
    // IFD.
    let mut other = b.bytes.clone();
    let value_at = 8 + 2 + 2 * 12 + 8;
    other[value_at..value_at + 4].copy_from_slice(&b.w32(60));
    let err = run_on(&other, &b.bytes, &gps_sel()).unwrap_err();
    assert_eq!(err.reason, UneditableReason::PointerNotInParent);
}

// ── fixture C: IFD0 and a thumbnail IFD1 ────────────────────────────────────

const THUMB: &[u8] = &[0xFF, 0xD8, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xFF, 0xD9];

/// IFD0 at 8 (Orientation, Make at 50; table 8..38), next -> IFD1 at 56 (table
/// 56..110: JPEGInterchangeFormat, its length, Compression, XResolution at
/// 110..118), the thumbnail at 118..128.
fn thumb_blob(be: bool, ifd1_next: u32) -> (Blob, Vec<E>) {
    let mut b = Blob::new(be, 8);
    let es = vec![b.short(0x0112, 6), b.out(0x010F, 2, 6, 50)];
    b.ifd(8, &es, 56);
    b.data(50, MAKE);
    let ifd1 = vec![
        b.long(0x0201, 118),
        b.long(0x0202, u32::try_from(THUMB.len()).unwrap()),
        b.short(0x0103, 6),
        b.out(0x011A, 5, 1, 110),
    ];
    b.ifd(56, &ifd1, ifd1_next);
    b.data(110, &[0x71; 8]);
    b.data(118, THUMB);
    (b, es)
}

fn thumb_sel() -> Selection {
    Selection {
        thumbnail: true,
        thumbnail_range: Some((118, 10)),
        ..Selection::default()
    }
}

#[test]
fn thumbnail_removal_sets_the_ifd0_next_word_to_zero_and_zeroes_ifd1_and_the_jpeg() {
    for be in [false, true] {
        let (b, es) = thumb_blob(be, 0);
        let plan = run(&b.bytes, &thumb_sel()).unwrap();
        let mut want = b.bytes.clone();
        // IFD0 loses no entry, only its next word.
        want[8..38].copy_from_slice(&expected_table(&b, &es, &[], 0));
        // IFD1's table, its XResolution value and the JPEG are zero.
        want[56..110].copy_from_slice(&zeros(54));
        want[110..118].copy_from_slice(&zeros(8));
        want[118..128].copy_from_slice(&zeros(10));
        assert_eq!(apply(&b.bytes, &plan), want, "be={be}");
        // The four IFD1 entries are recorded; IFD0's are not.
        assert_eq!(plan.deleted.len(), 4);
        assert!(plan.deleted.iter().all(|d| d.ifd == IfdKind::Ifd1));
        assert!(plan
            .patches
            .iter()
            .any(|p| p.reason == Some(ScrubReason::Thumbnail) && p.at == 118));
    }
}

#[test]
fn thumbnail_with_a_following_ifd_is_chain_after_thumbnail() {
    let (mut b, _) = thumb_blob(false, 140);
    let ifd2 = [b.short(0x0112, 1)];
    b.ifd(140, &ifd2, 0);
    let err = run(&b.bytes, &thumb_sel()).unwrap_err();
    assert_eq!(err.reason, UneditableReason::ChainAfterThumbnail);
}

#[test]
fn strips_form_thumbnail_is_thumbnail_not_located() {
    let (b, _) = thumb_blob(false, 0);
    let sel = Selection {
        thumbnail: true,
        thumbnail_range: None,
        ..Selection::default()
    };
    let err = run(&b.bytes, &sel).unwrap_err();
    assert_eq!(err.reason, UneditableReason::ThumbnailNotLocated);
}

#[test]
fn thumbnail_range_overlapping_a_kept_value_is_shared_bytes() {
    let (b, _) = thumb_blob(false, 0);
    // The thumbnail range claims bytes that include IFD0's Make value (50..56).
    let sel = Selection {
        thumbnail: true,
        thumbnail_range: Some((48, 20)),
        ..Selection::default()
    };
    let err = run(&b.bytes, &sel).unwrap_err();
    assert_eq!(err.reason, UneditableReason::SharedBytes);
    assert_eq!(err.at, 50);
}

#[test]
fn an_ifd_nested_in_a_removed_ifd_is_refused() {
    // A SubIFDs entry inside the GPS IFD: the walk follows it (any IFD may
    // hold SubIFDs), so removing the GPS IFD would orphan it.
    let (mut b, _) = gps_blob(false, false);
    let gps = vec![
        b.inl(0x0001, b"N\0"),
        b.out(0x0002, 5, 3, 114),
        b.long(0x014A, 170),
    ];
    b.ifd(72, &gps, 0);
    let sub = [b.short(0x0112, 1)];
    b.ifd(170, &sub, 0);
    let err = run(&b.bytes, &gps_sel()).unwrap_err();
    assert_eq!(err.reason, UneditableReason::NestedIfd);
}

#[test]
fn a_deleted_entry_of_an_unknown_type_is_payload_unreadable() {
    let mut b = Blob::new(false, 8);
    let es = vec![
        b.out(0x010F, 2, 6, 74),
        E {
            tag: 0x0110,
            ty: 99,
            count: 3,
            val: b.w32(80),
        },
    ];
    b.ifd(8, &es, 0);
    b.data(74, MAKE);
    b.data(80, MODEL);
    let err = run(&b.bytes, &del(&[(0, 1)])).unwrap_err();
    assert_eq!(err.reason, UneditableReason::PayloadUnreadable);
}

// ── fixture D: an Exif IFD holding a MakerNote ──────────────────────────────

/// Canon style: a MakerNote whose own entries hold absolute offsets into the
/// blob (here, 4 of them pointing at 200).
fn note_bytes(be: bool) -> Vec<u8> {
    let b = Blob::new(be, 8);
    let mut v = Vec::new();
    for i in 0..10u32 {
        v.extend_from_slice(&b.w32(200 + i));
    }
    v
}

/// IFD0 at 8 (Make, Exif pointer; table 8..38), Make at 50..56, Exif IFD at 56
/// (DateTimeOriginal 20 bytes at 98, MakerNote 40 bytes at 118, ColorSpace
/// inline; table 56..98).
fn note_blob(be: bool) -> (Blob, Vec<E>) {
    let mut b = Blob::new(be, 8);
    let es = vec![b.out(0x010F, 2, 6, 50), b.long(0x8769, 56)];
    b.ifd(8, &es, 0);
    b.data(50, MAKE);
    let exif = vec![
        b.out(0x9003, 2, 20, 98),
        b.out(0x927C, 7, 40, 118),
        b.short(0xA001, 1),
    ];
    b.ifd(56, &exif, 0);
    b.data(98, b"2031:02:03 04:05:06\0");
    b.data(118, &note_bytes(be));
    (b, exif)
}

#[test]
fn makernote_is_deleted_whole_and_its_value_zeroed() {
    for be in [false, true] {
        let (b, exif) = note_blob(be);
        // Exif IFD is walk index 1; the MakerNote is its entry 1.
        let plan = run(&b.bytes, &del(&[(1, 1)])).unwrap();
        let mut want = b.bytes.clone();
        want[56..98].copy_from_slice(&expected_table(&b, &exif, &[1], 0));
        want[118..158].copy_from_slice(&zeros(40));
        assert_eq!(apply(&b.bytes, &plan), want, "be={be}");
        assert_eq!(plan.deleted[0].ifd, IfdKind::Exif);
        assert_eq!(plan.deleted[0].tag, 0x927C);
    }
}

#[test]
fn makernote_kept_is_byte_identical_including_absolute_offsets() {
    for be in [false, true] {
        let (b, _) = note_blob(be);
        // A policy that names something else: delete the Make entry.
        let plan = run(&b.bytes, &del(&[(0, 0)])).unwrap();
        let out = apply(&b.bytes, &plan);
        assert_eq!(&out[118..158], &note_bytes(be)[..], "be={be}");
        assert_eq!(&out[98..118], b"2031:02:03 04:05:06\0");
        // And the Exif IFD table did not move.
        assert_eq!(&out[56..98], &b.bytes[56..98]);
    }
}

// ── HANDOFF-042 punch list ──────────────────────────────────────────────────

/// H1: an out-of-line value that runs past the end of the blob has no
/// `payload_range`, but its in-blob part is still data; it must not be taken
/// for an inline value that needs no scrub.
#[test]
fn h1_a_deleted_value_that_straddles_the_blob_end_is_payload_unreadable() {
    for be in [false, true] {
        let mut b = Blob::new(be, 8);
        let es = vec![b.short(0x0112, 1), b.out(0x013B, 2, 40, 38)];
        b.ifd(8, &es, 0);
        b.data(38, b"CANARY-ARTIST-PREFIX");
        let err = run(&b.bytes, &del(&[(0, 1)])).unwrap_err();
        assert_eq!(err.reason, UneditableReason::PayloadUnreadable, "be={be}");
        // A value that fits is fine (control).
        let mut ok = Blob::new(be, 8);
        let es = vec![ok.short(0x0112, 1), ok.out(0x013B, 2, 20, 38)];
        ok.ifd(8, &es, 0);
        ok.data(38, b"CANARY-ARTIST-PREFIX");
        assert!(run(&ok.bytes, &del(&[(0, 1)])).is_ok(), "be={be}");
    }
}

#[test]
fn h1_an_entry_of_a_removed_ifd_that_straddles_the_blob_end_is_payload_unreadable() {
    for be in [false, true] {
        let mut b = Blob::new(be, 8);
        let es = vec![b.short(0x0112, 1), b.long(0x8825, 38)];
        b.ifd(8, &es, 0);
        let gps = vec![b.out(0x001C, 7, 60, 56)];
        b.ifd(38, &gps, 0);
        b.data(56, b"CANARY-GPS-AREA-PREFIX");
        let err = run(&b.bytes, &gps_sel()).unwrap_err();
        assert_eq!(err.reason, UneditableReason::PayloadUnreadable, "be={be}");
    }
}

/// H2: more than one thumbnail locator leaves image bytes unreferenced.
#[test]
fn h2_a_thumbnail_ifd1_with_other_locators_is_thumbnail_not_located() {
    type Ents = Vec<(u16, u16, u32, u32)>;
    let cases: [(&str, Ents); 4] = [
        (
            "second pair",
            vec![
                (0x0201, 4, 1, 80),
                (0x0202, 4, 1, 16),
                (0x0201, 4, 1, 96),
                (0x0202, 4, 1, 20),
            ],
        ),
        (
            "strip offsets",
            vec![(0x0111, 4, 1, 96), (0x0201, 4, 1, 80), (0x0202, 4, 1, 16)],
        ),
        (
            "strip byte counts",
            vec![(0x0117, 4, 1, 20), (0x0201, 4, 1, 80), (0x0202, 4, 1, 16)],
        ),
        (
            "second length",
            vec![(0x0201, 4, 1, 80), (0x0202, 4, 1, 16), (0x0202, 4, 1, 20)],
        ),
    ];
    for be in [false, true] {
        for (what, ents) in &cases {
            let mut b = Blob::new(be, 8);
            let ifd0 = vec![b.short(0x0112, 1)];
            b.ifd(8, &ifd0, 26);
            let es: Vec<E> = ents
                .iter()
                .map(|&(tag, ty, count, val)| E {
                    tag,
                    ty,
                    count,
                    val: b.w32(val),
                })
                .collect();
            b.ifd(26, &es, 0);
            b.data(80, &[0xFF, 0xD8, b'T', b'H', b'U', b'M', b'B', 0xFF, 0xD9]);
            b.data(96, b"CANARY-SECOND-THUMB!");
            let sel = Selection {
                thumbnail: true,
                thumbnail_range: Some((80, 9)),
                ..Selection::default()
            };
            let err = run(&b.bytes, &sel).unwrap_err();
            assert_eq!(
                err.reason,
                UneditableReason::ThumbnailNotLocated,
                "{what} be={be}"
            );
        }
    }
}

/// L2: a GPS IFD with no entry (count 0) behind a present pointer: the pointer
/// goes and the six bytes of the empty table are zeroed.
#[test]
fn l2_an_empty_gps_ifd_is_removed_with_its_pointer() {
    for be in [false, true] {
        let mut b = Blob::new(be, 8);
        let es = vec![b.short(0x0112, 1), b.long(0x8825, 38)];
        b.ifd(8, &es, 0);
        b.ifd(38, &[], 0);
        let plan = run(&b.bytes, &gps_sel()).unwrap();
        let out = apply(&b.bytes, &plan);
        assert_eq!(&out[38..44], &[0; 6], "be={be}");
        assert_eq!(plan.deleted.len(), 1);
        assert_eq!(plan.deleted[0].tag, 0x8825);
        let c = Container::parse(&out).unwrap();
        assert!(c.is_clean() && c.ifds().len() == 1);
    }
}

/// L3: the overlap check runs although only one of the two IFDs is edited.
#[test]
fn l3_overlapping_ifd_tables_are_shared_bytes_even_when_only_one_is_edited() {
    let mut b = Blob::new(false, 8);
    // IFD0's table is 8..38; the SubIFD sits at 34, over its next-IFD word.
    let es = vec![b.out(0x010F, 2, 6, 60), b.long(0x014A, 34)];
    b.ifd(8, &es, 0);
    b.data(60, MAKE);
    let c = Container::parse(&b.bytes).unwrap();
    assert!(c.is_clean(), "{:?}", c.issues());
    assert_eq!(c.ifds().len(), 2);
    // Only IFD0 loses an entry (the Make).
    let err = run(&b.bytes, &del(&[(0, 0)])).unwrap_err();
    assert_eq!(
        err,
        Refusal {
            reason: UneditableReason::SharedBytes,
            at: 34
        }
    );
    // And with nothing to edit at all.
    assert_eq!(
        run(&b.bytes, &Selection::default()).unwrap_err().reason,
        UneditableReason::SharedBytes
    );
}
