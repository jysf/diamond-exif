//! SPEC-025 (Part A): the Exif block builder against the fixture grid in
//! `tests/support/exif_block_shapes.rs`, the independent model in
//! `exif_block_model.rs` and the test placer in `exif_block_place.rs`. The
//! expectations come from the model and the shapes, never from the library's
//! own self-check.

#[path = "support/exif_block_model.rs"]
mod exif_block_model;
#[path = "support/exif_block_place.rs"]
mod exif_block_place;
#[path = "support/exif_block_shapes.rs"]
mod exif_block_shapes;
#[path = "support/exif_block_sweep_tables.rs"]
mod exif_block_sweep_tables;
#[path = "support/oracle_tiff.rs"]
mod oracle_tiff;
#[path = "support/real_shapes.rs"]
mod real_shapes;
#[path = "support/strip_shapes.rs"]
mod strip_shapes;
#[path = "support/tiff_shapes.rs"]
mod tiff_shapes;

use std::collections::BTreeSet;

use exif_block_model::{model, Ifd as MIfd, Reason as MReason};
use exif_block_place::place;
use exif_block_shapes::{
    grid, Expect, Shape, CONSERVATIVE_TAGS, LENGTH_PARTNERS, MEASURED_OFFSET_TAGS,
    POINTER_OFFSET_TAGS,
};
use exif_core::reader::ByteOrder;
use exif_core::tiff::{Container, IfdKind};
use exif_core::write::{
    exif_block, BlockError, BlockIfd, BlockOptions, ExifBlock, OmitReason, UnbuildableReason,
    DEFAULT_MAX_BLOCK_LEN,
};

// ── the shapes ──────────────────────────────────────────────────────────────

/// The grid plus the two shapes it cannot build: a DNG (its GPS is inside the
/// raw SubIFD, so not in the block) and a JPEG's Exif TIFF blob.
fn all_shapes() -> Vec<Shape> {
    let mut v = grid();
    for be in [false, true] {
        let tag = if be { "be" } else { "le" };
        v.push(Shape {
            name: format!("dng/{tag}"),
            bytes: tiff_shapes::dng(be).bytes,
            expect: Expect::Block,
        });
        v.push(Shape {
            name: format!("jpeg-exif-blob/{tag}"),
            bytes: strip_shapes::full_tiff(be),
            expect: Expect::Block,
        });
    }
    v
}

fn block_shapes() -> Vec<Shape> {
    all_shapes()
        .into_iter()
        .filter(|s| s.expect == Expect::Block)
        .collect()
}

fn shape(name: &str) -> Shape {
    all_shapes()
        .into_iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("no shape {name}"))
}

fn plan(s: &Shape) -> ExifBlock<'_> {
    exif_block(&s.bytes, &BlockOptions::new())
        .unwrap_or_else(|e| panic!("{}: expected a block, got {e}", s.name))
}

/// The refusal key of the shapes file, from the library's error.
fn key(err: &BlockError) -> String {
    let ifd = |i: &BlockIfd| match i {
        BlockIfd::Exif => "exif",
        BlockIfd::Interop => "interop",
        BlockIfd::Gps => "gps",
        _ => "other",
    };
    match err {
        BlockError::Source(_) => "source".into(),
        BlockError::Unbuildable { reason } => match reason {
            UnbuildableReason::WalkHalted => "walk-halted".into(),
            UnbuildableReason::BrokenPointer { tag } => format!("broken-pointer:0x{tag:04x}"),
            UnbuildableReason::DuplicatePointer { tag } => {
                format!("duplicate-pointer:0x{tag:04x}")
            }
            UnbuildableReason::DuplicateTag { ifd: i, tag } => {
                format!("duplicate-tag:{}:0x{tag:04x}", ifd(i))
            }
            UnbuildableReason::UnreadableValue { ifd: i, tag } => {
                format!("unreadable:{}:0x{tag:04x}", ifd(i))
            }
            _ => "other".into(),
        },
        BlockError::OverBudget { .. } => "over-budget".into(),
        BlockError::BadBase { .. } => "bad-base".into(),
        BlockError::BufferLength { .. } => "buffer-length".into(),
        BlockError::VerifyFailed { .. } => "verify-failed".into(),
        _ => "other".into(),
    }
}

fn map_ifd(i: BlockIfd) -> MIfd {
    match i {
        BlockIfd::Exif => MIfd::Exif,
        BlockIfd::Interop => MIfd::Interop,
        BlockIfd::Gps => MIfd::Gps,
        _ => panic!("unknown IFD"),
    }
}

fn map_reason(r: OmitReason) -> MReason {
    match r {
        OmitReason::MakerNote => MReason::MakerNote,
        OmitReason::OffsetCarrying => MReason::OffsetCarrying,
        OmitReason::UnknownType => MReason::UnknownType,
        _ => panic!("unknown reason"),
    }
}

fn omissions_of(p: &ExifBlock<'_>) -> Vec<(MIfd, u16, u16, MReason)> {
    p.omissions()
        .iter()
        .map(|o| {
            (
                map_ifd(o.ifd()),
                o.tag(),
                o.field_type(),
                map_reason(o.reason()),
            )
        })
        .collect()
}

// ── AC3 ─────────────────────────────────────────────────────────────────────

#[test]
fn grid_blocks_match_the_independent_model() {
    for s in block_shapes() {
        let p = plan(&s);
        for base in [8u32, 1000] {
            let m = model(&s.bytes, base);
            assert_eq!(
                p.byte_order(),
                if m.be {
                    ByteOrder::Big
                } else {
                    ByteOrder::Little
                },
                "{}",
                s.name
            );
            assert_eq!(p.len() as usize, m.bytes.len(), "{} len", s.name);
            assert_eq!(p.exif_position(), m.exif_pos, "{} exif position", s.name);
            assert_eq!(p.gps_position(), m.gps_pos, "{} gps position", s.name);
            let model_omissions: Vec<_> = m
                .omissions
                .iter()
                .map(|o| (o.ifd, o.tag, o.field_type, o.reason))
                .collect();
            assert_eq!(omissions_of(&p), model_omissions, "{} omissions", s.name);
            let got = p.write(base).unwrap_or_else(|e| panic!("{}: {e}", s.name));
            assert_eq!(got, m.bytes, "{} bytes at base {base}", s.name);
        }
    }
}

// ── AC7 ─────────────────────────────────────────────────────────────────────

/// `(kind, index)` of the IFD of `kind` whose parent is `parent`.
fn by_parent(c: &Container<'_>, kind: IfdKind, parent: usize) -> Option<usize> {
    c.ifds()
        .iter()
        .position(|i| i.kind() == kind && i.parent() == Some(parent))
}

/// Every entry of an IFD as `(tag, type, count, payload)`.
fn entries_of(c: &Container<'_>, idx: usize) -> Vec<(u16, u16, u32, Vec<u8>)> {
    c.ifds()[idx]
        .entries()
        .iter()
        .map(|e| {
            let payload = c
                .payload(e)
                .map(<[u8]>::to_vec)
                .unwrap_or_else(|_| Vec::new());
            (e.tag(), e.field_type(), e.count(), payload)
        })
        .collect()
}

#[test]
fn grid_blocks_read_back_equal_minus_omissions() {
    for s in block_shapes() {
        let p = plan(&s);
        let src = Container::parse(&s.bytes).unwrap();
        let be = p.byte_order() == ByteOrder::Big;
        for base in [8u32, 1000, 65534] {
            let block = p.write(base).unwrap();
            let placed = place(be, base, &block, p.exif_position(), p.gps_position());
            let c = Container::parse(&placed).unwrap();
            assert!(
                c.is_clean(),
                "{} at {base}: issues {:?}",
                s.name,
                c.issues()
            );
            // Source and placed IFDs, selected by parent.
            let s_exif = by_parent(&src, IfdKind::Exif, 0);
            let s_gps = by_parent(&src, IfdKind::Gps, 0);
            let s_interop = s_exif.and_then(|e| by_parent(&src, IfdKind::Interop, e));
            let p_exif = by_parent(&c, IfdKind::Exif, 0);
            let p_gps = by_parent(&c, IfdKind::Gps, 0);
            let p_interop = p_exif.and_then(|e| by_parent(&c, IfdKind::Interop, e));
            // An IFD is in the block when the source has it, whatever its
            // entries (an empty Exif IFD is kept empty).
            assert_eq!(p_exif.is_some(), s_exif.is_some(), "{} exif", s.name);
            assert_eq!(p_gps.is_some(), s_gps.is_some(), "{} gps", s.name);
            assert_eq!(
                p_interop.is_some(),
                s_interop.is_some(),
                "{} interop",
                s.name
            );
            let expected_ifds = 1
                + usize::from(p_exif.is_some())
                + usize::from(p_gps.is_some())
                + usize::from(p_interop.is_some());
            if p.is_empty() {
                assert_eq!(c.ifds().len(), 1, "{}", s.name);
            } else {
                assert_eq!(c.ifds().len(), expected_ifds, "{} ifd count", s.name);
            }

            for (which, s_idx, p_idx) in [
                (BlockIfd::Exif, s_exif, p_exif),
                (BlockIfd::Interop, s_interop, p_interop),
                (BlockIfd::Gps, s_gps, p_gps),
            ] {
                let (Some(si), Some(pi)) = (s_idx, p_idx) else {
                    continue;
                };
                let omitted: BTreeSet<u16> = p
                    .omissions()
                    .iter()
                    .filter(|o| o.ifd() == which)
                    .map(|o| o.tag())
                    .collect();
                let mut want: Vec<_> = entries_of(&src, si)
                    .into_iter()
                    .filter(|e| !omitted.contains(&e.0))
                    .collect();
                want.sort_by_key(|e| e.0);
                let got = entries_of(&c, pi);
                // The Interop pointer is rewritten: LONG, count 1, the placed
                // Interop IFD's offset.
                let strip_ptr = |v: &mut Vec<(u16, u16, u32, Vec<u8>)>| {
                    v.iter_mut().for_each(|e| {
                        if e.0 == 0xA005 && which == BlockIfd::Exif {
                            *e = (0xA005, 4, 1, Vec::new());
                        }
                    });
                };
                let mut got_cmp = got.clone();
                strip_ptr(&mut got_cmp);
                strip_ptr(&mut want);
                assert_eq!(got_cmp, want, "{} {which} at base {base}", s.name);
                assert_eq!(c.ifds()[pi].next(), 0, "{} next-word", s.name);
                if which == BlockIfd::Exif {
                    if let Some(ptr) = got.iter().find(|e| e.0 == 0xA005) {
                        let ii = p_interop.unwrap();
                        let v = if be {
                            u32::from_be_bytes(ptr.3[..4].try_into().unwrap())
                        } else {
                            u32::from_le_bytes(ptr.3[..4].try_into().unwrap())
                        };
                        assert_eq!(v, c.ifds()[ii].offset(), "{} interop pointer", s.name);
                    }
                }
            }
        }
    }
}

// ── AC4 ─────────────────────────────────────────────────────────────────────

fn offset_tags_under_test() -> Vec<u16> {
    MEASURED_OFFSET_TAGS
        .iter()
        .chain(POINTER_OFFSET_TAGS.iter())
        .chain(CONSERVATIVE_TAGS.iter())
        .copied()
        .collect()
}

#[test]
fn each_offset_tag_is_omitted_in_each_ifd() {
    for be in ["le", "be"] {
        for (which, ifd) in [
            ("exif", BlockIfd::Exif),
            ("interop", BlockIfd::Interop),
            ("gps", BlockIfd::Gps),
        ] {
            for tag in offset_tags_under_test() {
                let s = shape(&format!("offset-0x{tag:04x}-{which}/{be}"));
                let p = plan(&s);
                let got: Vec<_> = p
                    .omissions()
                    .iter()
                    .map(|o| (o.ifd(), o.tag(), o.field_type(), o.reason()))
                    .collect();
                assert_eq!(
                    got,
                    vec![(ifd, tag, 4, OmitReason::OffsetCarrying)],
                    "{}",
                    s.name
                );
            }
        }
        // 0xA005 is omitted wherever it is not the Exif IFD's pointer.
        for (which, ifd) in [("interop", BlockIfd::Interop), ("gps", BlockIfd::Gps)] {
            let s = shape(&format!("offset-0xa005-{which}/{be}"));
            let got: Vec<_> = plan(&s)
                .omissions()
                .iter()
                .map(|o| (o.ifd(), o.tag(), o.reason()))
                .collect();
            assert_eq!(
                got,
                vec![(ifd, 0xA005, OmitReason::OffsetCarrying)],
                "{}",
                s.name
            );
        }
        // A type-13 entry of any other tag; 0xA005 of type 13 is the pointer.
        let s = shape(&format!("type13-entries/{be}"));
        let got: Vec<_> = plan(&s)
            .omissions()
            .iter()
            .map(|o| (o.ifd(), o.tag(), o.field_type(), o.reason()))
            .collect();
        assert_eq!(
            got,
            vec![
                (BlockIfd::Exif, 0x9999, 13, OmitReason::OffsetCarrying),
                (BlockIfd::Interop, 0x9998, 13, OmitReason::OffsetCarrying),
                (BlockIfd::Gps, 0x9997, 13, OmitReason::OffsetCarrying),
            ]
        );
        let s = shape(&format!("a005-type13/{be}"));
        assert!(plan(&s).omissions().is_empty(), "{}", s.name);
        // A pointer that leads nowhere is dropped, not written.
        let s = shape(&format!("a005-zero/{be}"));
        let got: Vec<_> = plan(&s)
            .omissions()
            .iter()
            .map(|o| (o.tag(), o.reason()))
            .collect();
        assert_eq!(
            got,
            vec![(0xA005, OmitReason::OffsetCarrying)],
            "{}",
            s.name
        );
    }
}

#[test]
fn makernote_and_offset_schema_are_omitted() {
    for be in ["le", "be"] {
        let s = shape(&format!("makernote/{be}"));
        let got: Vec<_> = plan(&s)
            .omissions()
            .iter()
            .map(|o| (o.ifd(), o.tag(), o.field_type(), o.reason()))
            .collect();
        assert_eq!(
            got,
            vec![
                (BlockIfd::Exif, 0x927C, 7, OmitReason::MakerNote),
                (BlockIfd::Exif, 0xEA1D, 9, OmitReason::MakerNote),
            ],
            "{}",
            s.name
        );
    }
}

#[test]
fn unknown_types_are_omitted() {
    for be in ["le", "be"] {
        let s = shape(&format!("unknown-types/{be}"));
        let got: Vec<_> = plan(&s)
            .omissions()
            .iter()
            .map(|o| (o.ifd(), o.tag(), o.field_type(), o.reason()))
            .collect();
        let u = OmitReason::UnknownType;
        assert_eq!(
            got,
            vec![
                (BlockIfd::Exif, 0x9100, 0, u),
                (BlockIfd::Exif, 0x9101, 14, u),
                (BlockIfd::Exif, 0x9102, 16, u),
                (BlockIfd::Exif, 0x9103, 255, u),
                // An unknown type wins over the MakerNote tag (item 2 of the
                // amendment: the entry cannot be sized, so it cannot be moved).
                (BlockIfd::Exif, 0x927C, 0, u),
                (BlockIfd::Interop, 0x0010, 14, u),
                (BlockIfd::Gps, 0x0020, 255, u),
            ],
            "{}",
            s.name
        );
    }
}

#[test]
fn length_partners_and_unknown_tags_are_kept() {
    for be in ["le", "be"] {
        let s = shape(&format!("partners-and-unknown-tags/{be}"));
        let p = plan(&s);
        assert!(p.omissions().is_empty(), "{}", s.name);
        let block = p.write(8).unwrap();
        let placed = place(
            p.byte_order() == ByteOrder::Big,
            8,
            &block,
            p.exif_position(),
            p.gps_position(),
        );
        let c = Container::parse(&placed).unwrap();
        for kind in [IfdKind::Exif, IfdKind::Interop, IfdKind::Gps] {
            let idx = c.ifds().iter().position(|i| i.kind() == kind).unwrap();
            let tags: BTreeSet<u16> = c.ifds()[idx].entries().iter().map(|e| e.tag()).collect();
            for t in LENGTH_PARTNERS.iter().chain([0xF0F0, 0x8827].iter()) {
                assert!(tags.contains(t), "{} {kind:?} lost {t:#06x}", s.name);
            }
        }
    }
}

#[test]
fn omissions_are_ordered_and_typed() {
    let rank = |i: BlockIfd| match i {
        BlockIfd::Exif => 0,
        BlockIfd::Interop => 1,
        _ => 2,
    };
    for s in block_shapes() {
        let p = plan(&s);
        let src = Container::parse(&s.bytes).unwrap();
        let exif = by_parent(&src, IfdKind::Exif, 0);
        let idx_of = |i: BlockIfd| match i {
            BlockIfd::Exif => exif,
            BlockIfd::Interop => exif.and_then(|e| by_parent(&src, IfdKind::Interop, e)),
            _ => by_parent(&src, IfdKind::Gps, 0),
        };
        let keys: Vec<(i32, u16)> = p
            .omissions()
            .iter()
            .map(|o| (rank(o.ifd()), o.tag()))
            .collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted, "{} order", s.name);
        for o in p.omissions() {
            let ifd =
                idx_of(o.ifd()).unwrap_or_else(|| panic!("{}: omission of an absent IFD", s.name));
            let e = src.ifds()[ifd]
                .entries()
                .iter()
                .find(|e| e.tag() == o.tag())
                .unwrap_or_else(|| panic!("{}: omission names a missing entry", s.name));
            assert_eq!(e.field_type(), o.field_type(), "{} type", s.name);
        }
    }
}

// ── AC2 ─────────────────────────────────────────────────────────────────────

#[test]
fn gps_only_exif_only_and_empty() {
    for be in ["le", "be"] {
        let s = shape(&format!("gps-only/{be}"));
        let p = plan(&s);
        assert_eq!(p.exif_position(), None);
        assert!(p.gps_position().is_some());
        assert!(!p.is_empty());

        let s = shape(&format!("exif-only/{be}"));
        let p = plan(&s);
        assert!(p.exif_position().is_some());
        assert_eq!(p.gps_position(), None);

        // Neither IFD, or only IFDs the block does not take: empty.
        for name in [
            "neither",
            "no-ifd0",
            "exif-only-in-sub",
            "exif-only-under-ifd1",
        ] {
            let s = shape(&format!("{name}/{be}"));
            let p = plan(&s);
            assert!(p.is_empty(), "{}", s.name);
            assert_eq!(p.len(), 0, "{}", s.name);
            assert_eq!(p.exif_position(), None, "{}", s.name);
            assert_eq!(p.gps_position(), None, "{}", s.name);
            assert!(p.omissions().is_empty(), "{}", s.name);
            for base in [0u32, 8, 1000, 65534] {
                assert_eq!(p.write(base), Ok(Vec::new()), "{} at {base}", s.name);
            }
            let mut none: [u8; 0] = [];
            assert_eq!(p.write_into(8, &mut none), Ok(()));
        }

        // An empty Exif IFD is written as an empty IFD: count 0, next-word 0.
        let s = shape(&format!("empty-exif-and-gps/{be}"));
        let p = plan(&s);
        assert_eq!(p.exif_position(), Some(0));
        assert_eq!(p.gps_position(), Some(6));
        let block = p.write(8).unwrap();
        assert_eq!(&block[..6], &[0; 6]);
    }
}

// ── AC5 ─────────────────────────────────────────────────────────────────────

#[test]
fn each_refusal_reason_is_reachable_and_returns_no_bytes() {
    let mut seen = BTreeSet::new();
    for s in all_shapes() {
        let Expect::Refuse(want) = &s.expect else {
            continue;
        };
        let got = exif_block(&s.bytes, &BlockOptions::new());
        match got {
            Err(e) => {
                assert_eq!(&key(&e), want, "{}: {e}", s.name);
                seen.insert(want.split(':').next().unwrap().to_string());
            }
            Ok(p) => panic!("{}: expected a refusal, got a block of {}", s.name, p.len()),
        }
    }
    for kind in [
        "source",
        "walk-halted",
        "broken-pointer",
        "duplicate-pointer",
        "duplicate-tag",
        "unreadable",
    ] {
        assert!(seen.contains(kind), "no fixture reaches {kind}");
    }

    // OverBudget from `exif_block`: 1,000 entries sharing one 2 KiB value.
    let s = shape("shared-2k-x1000/le");
    assert!(exif_block(&s.bytes, &BlockOptions::new()).is_ok());
    let tight = BlockOptions::new().with_max_len(1 << 20);
    match exif_block(&s.bytes, &tight) {
        Err(BlockError::OverBudget { needed, budget }) => {
            assert!(needed > 2_000_000, "needed {needed}");
            assert_eq!(budget, 1 << 20);
        }
        other => panic!("expected OverBudget, got {other:?}"),
    }

    // OverBudget from `write`: the block fits, `base + len + 34` does not.
    let s = shape("sample/le");
    let len = plan(&s).len();
    let p = exif_block(&s.bytes, &BlockOptions::new().with_max_len(len + 40)).unwrap();
    assert!(matches!(p.write(8), Err(BlockError::OverBudget { .. })));
    assert!(matches!(
        p.write(1000),
        Err(BlockError::OverBudget { needed, budget })
            if needed == u64::from(1000 + len + 34) && budget == len + 40
    ));

    // BadBase: odd, and base + len past u32::MAX.
    let p = plan(&s);
    assert_eq!(p.write(7), Err(BlockError::BadBase { base: 7 }));
    assert_eq!(p.write(1001), Err(BlockError::BadBase { base: 1001 }));
    let near = u32::MAX - 1;
    assert_eq!(p.write(near), Err(BlockError::BadBase { base: near }));

    // BufferLength: one byte short and one byte long.
    let n = usize::try_from(len).unwrap();
    for got in [n - 1, n + 1] {
        let mut out = vec![0u8; got];
        assert_eq!(
            p.write_into(8, &mut out),
            Err(BlockError::BufferLength { expected: len, got })
        );
    }
}

#[test]
fn issues_outside_the_block_do_not_refuse() {
    for name in [
        "ok-broken-ifd1-next",
        "ok-subifd-loop",
        "ok-broken-subifd-in-exif",
    ] {
        for be in ["le", "be"] {
            let s = shape(&format!("{name}/{be}"));
            // The fixture really does have an issue.
            let c = Container::parse(&s.bytes).unwrap();
            assert!(!c.issues().is_empty(), "{} has no issue", s.name);
            let p = plan(&s);
            assert!(!p.is_empty(), "{}", s.name);
            let m = model(&s.bytes, 8);
            assert_eq!(p.write(8).unwrap(), m.bytes, "{}", s.name);
        }
    }
    // The broken SubIFD pointer inside the Exif IFD is an omission.
    let s = shape("ok-broken-subifd-in-exif/le");
    let got: Vec<_> = plan(&s)
        .omissions()
        .iter()
        .map(|o| (o.tag(), o.reason()))
        .collect();
    assert_eq!(got, vec![(0x014A, OmitReason::OffsetCarrying)]);
}

// ── AC8 ─────────────────────────────────────────────────────────────────────

#[test]
fn deterministic_and_write_into_matches_write() {
    for s in block_shapes() {
        let a = plan(&s);
        let b = plan(&s);
        assert_eq!(a.len(), b.len(), "{}", s.name);
        assert_eq!(a.omissions(), b.omissions(), "{}", s.name);
        for base in [8u32, 1000] {
            let w1 = a.write(base).unwrap();
            let w2 = b.write(base).unwrap();
            assert_eq!(w1, w2, "{} at {base}", s.name);
            // Fill with a pattern: `write_into` must overwrite every byte.
            let mut out = vec![0xA5u8; usize::try_from(a.len()).unwrap()];
            a.write_into(base, &mut out).unwrap();
            assert_eq!(out, w1, "{} write_into at {base}", s.name);
        }
    }
}

#[test]
fn default_budget_is_16_mib() {
    assert_eq!(DEFAULT_MAX_BLOCK_LEN, 1 << 24);
    assert_eq!(BlockOptions::default().max_len(), 1 << 24);
    assert_eq!(BlockOptions::new(), BlockOptions::default());
    assert_eq!(BlockOptions::new().with_max_len(5).max_len(), 5);
}

// ── AC4: the mirrors of the omission table ──────────────────────────────────

/// A source whose single block IFD holds every tag `0..=0xFFFE` as an inline
/// LONG 0 (a pointer of 0 leads nowhere). Which of them the builder omits is
/// the library's omission table, read off its behaviour.
fn sweep_source(be: bool, exif: bool) -> Vec<u8> {
    let w16 = |v: u16| if be { v.to_be_bytes() } else { v.to_le_bytes() };
    let w32 = |v: u32| if be { v.to_be_bytes() } else { v.to_le_bytes() };
    let mut out = Vec::new();
    out.extend_from_slice(if be { b"MM\0\x2A" } else { b"II\x2A\0" });
    out.extend_from_slice(&w32(8));
    // IFD0 at 8: one pointer, 18 bytes; the swept IFD at 26.
    out.extend_from_slice(&w16(1));
    out.extend_from_slice(&w16(if exif { 0x8769 } else { 0x8825 }));
    out.extend_from_slice(&w16(4));
    out.extend_from_slice(&w32(1));
    out.extend_from_slice(&w32(26));
    out.extend_from_slice(&w32(0));
    out.extend_from_slice(&w16(0xFFFF));
    for tag in 0..=0xFFFEu16 {
        out.extend_from_slice(&w16(tag));
        out.extend_from_slice(&w16(4));
        out.extend_from_slice(&w32(1));
        out.extend_from_slice(&w32(0));
    }
    out.extend_from_slice(&w32(0));
    out
}

#[test]
fn omission_table_mirrors_agree() {
    // Mirror 1: the shapes file's lists. Mirror 2: the model's table. Mirror 3, below,
    // is the live sweep's tables. The library's table is read off its behaviour
    // at the end.
    let shapes_set: BTreeSet<u16> = offset_tags_under_test()
        .into_iter()
        .chain([0xA005])
        .collect();
    let model_set: BTreeSet<u16> = exif_block_model::OFFSET_CARRYING.iter().copied().collect();
    assert_eq!(shapes_set, model_set, "shapes file vs model");
    let makernote: BTreeSet<u16> = exif_block_model::MAKERNOTE.iter().copied().collect();
    assert_eq!(makernote, BTreeSet::from([0x927C, 0xEA1D]));

    // Mirror 3: the live sweep's tables (`exif_block_sweep_live.rs` asserts
    // them against ExifTool; here they are compared with the other mirrors).
    // The fuzz invariants (`fuzz/shared/exif_block_invariants.rs`) predict no
    // omission reasons, so they hold no table to compare.
    let writer: BTreeSet<u16> = exif_block_sweep_tables::WRITER_NO_SIZE_TAG
        .iter()
        .copied()
        .collect();
    assert_eq!(
        writer,
        MEASURED_OFFSET_TAGS
            .iter()
            .copied()
            .collect::<BTreeSet<u16>>(),
        "sweep tables vs the measured offset tags"
    );
    let conservative: BTreeSet<u16> = exif_block_sweep_tables::CONSERVATIVE_EXTRAS
        .iter()
        .copied()
        .collect();
    assert_eq!(
        conservative,
        CONSERVATIVE_TAGS.iter().copied().collect::<BTreeSet<u16>>(),
        "sweep tables vs the conservative tags"
    );
    let pointers: BTreeSet<u16> = exif_block_sweep_tables::POINTER_EXTRAS
        .iter()
        .copied()
        .collect();
    assert_eq!(
        pointers,
        POINTER_OFFSET_TAGS
            .iter()
            .copied()
            .chain([0xA005])
            .collect::<BTreeSet<u16>>(),
        "sweep tables vs the pointer tags"
    );
    assert!(pointers.contains(&exif_block_sweep_tables::SUBIFD_TAG));
    assert_eq!(
        exif_block_sweep_tables::MAKERNOTE
            .iter()
            .copied()
            .collect::<BTreeSet<u16>>(),
        makernote,
        "sweep tables vs the model's MakerNote tags"
    );
    let sweep_union: BTreeSet<u16> = writer
        .iter()
        .chain(conservative.iter())
        .chain(pointers.iter())
        .copied()
        .collect();
    assert_eq!(sweep_union, model_set, "sweep tables vs the model");

    // The library: sweep every tag through the GPS IFD and the Exif IFD.
    for be in [false, true] {
        for exif in [false, true] {
            let data = sweep_source(be, exif);
            let s = Shape {
                name: "sweep".into(),
                bytes: data,
                expect: Expect::Block,
            };
            let p = plan(&s);
            let mut offset: BTreeSet<u16> = BTreeSet::new();
            let mut maker: BTreeSet<u16> = BTreeSet::new();
            for o in p.omissions() {
                match o.reason() {
                    OmitReason::OffsetCarrying => offset.insert(o.tag()),
                    OmitReason::MakerNote => maker.insert(o.tag()),
                    other => panic!("unexpected {other:?}"),
                };
            }
            assert_eq!(
                offset, model_set,
                "library vs model (exif: {exif}, be: {be})"
            );
            assert_eq!(maker, makernote, "library MakerNote set");
        }
    }
}
