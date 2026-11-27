// The SPEC-025 invariants of `exif_core::write::exif_block` (DEC-014, DEC-015),
// shared by the fuzz target (`fuzz/fuzz_targets/exif_block.rs`) and the
// stable-toolchain layout test (`tests/fuzz_layout.rs`) with `include!`, so the
// two cannot drift. This file is not a crate root; it is text included at module
// level, and it defines one module, `exif_block_invariants`.
//
// The harness is INDEPENDENT of `src/write/block.rs`: it has its own placer
// (the TIFF header, zeros up to `base`, the block, then IFD0 holding 0x8769 and
// 0x8825 as `base + position`) and its own comparison, and it reads the placed
// bytes back with the crate's WALK (`Container`), not with anything of the
// builder's. What it calls in the library, exactly: `exif_block`,
// `BlockOptions`, `ExifBlock` and its accessors, `Omission` and its accessors,
// the error type, and `Container` / `IfdKind` for the two reads (the source and
// the placed file). A test in `tests/fuzz_layout.rs` asserts that no line of
// code here names the builder's module or its internals.
//
// What it does NOT do: predict the omission REASONS. The omission table is a
// decision (DEC-014's amendment), checked by `tests/exif_block.rs` against
// three hand-typed mirrors; here an omission only has to name an entry that
// exists in the source's selected IFD.
//
// Input layout: bytes 0 and 1 are the base (`u16`, little-endian, made even);
// the rest is the source. Each input is planned under a small budget
// (`SMALL_BUDGET`), and again under the default budget when the small one
// refuses, or when the small one cannot write at that base (the self-check's
// scratch image counts against the budget).
//
// Every guard is its own function over `Facts` (the library's answer,
// flattened), so a planted bug in the flattened answer turns exactly its guard
// red (see `tests/fuzz_layout.rs`).

mod exif_block_invariants {
    use std::collections::BTreeSet;

    use exif_core::reader::ByteOrder;
    use exif_core::tiff::{Container, IfdKind};
    use exif_core::write::{exif_block, BlockError, BlockIfd, BlockOptions, ExifBlock};

    /// The small budget of the first run. Small enough that a few hundred
    /// entries refuse it.
    pub const SMALL_BUDGET: u32 = 2048;

    /// One omission as the harness sees it: IFD (0 Exif, 1 Interop, 2 GPS),
    /// tag, field type.
    pub type Omit = (u8, u16, u16);

    /// The library's answer for one input, flattened.
    #[derive(Clone, Debug)]
    pub struct Facts {
        pub base: u32,
        pub source: Vec<u8>,
        pub be: bool,
        pub len: u32,
        pub exif_pos: Option<u32>,
        pub gps_pos: Option<u32>,
        pub omissions: Vec<Omit>,
        /// `write(base)` of the plan.
        pub block: Vec<u8>,
        /// The same, from a second `exif_block` and `write`.
        pub again: Vec<u8>,
        pub again_omissions: Vec<Omit>,
        pub again_len: u32,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Guard {
        /// `write(base)` has length `len()`.
        Length,
        /// Positions are even, inside the block, and absent only with no IFD.
        Positions,
        /// Two runs give equal bytes, length and omissions.
        Deterministic,
        /// The placed file parses clean, with the IFDs the source has.
        PlacesClean,
        /// The placed entries equal the source's minus the omissions.
        EntriesEqual,
        /// Every omission names an entry that exists in the source.
        OmissionsExist,
    }

    pub const GUARDS: [Guard; 6] = [
        Guard::Length,
        Guard::Positions,
        Guard::Deterministic,
        Guard::PlacesClean,
        Guard::EntriesEqual,
        Guard::OmissionsExist,
    ];

    /// `(base, source)` of an input, or `None` when it is under two bytes.
    pub fn decode(data: &[u8]) -> Option<(u32, &[u8])> {
        let (head, rest) = data.split_first_chunk::<2>()?;
        Some((u32::from(u16::from_le_bytes(*head) & !1), rest))
    }

    fn ifd_code(i: BlockIfd) -> u8 {
        match i {
            BlockIfd::Exif => 0,
            BlockIfd::Interop => 1,
            _ => 2,
        }
    }

    fn omit_list(plan: &ExifBlock<'_>) -> Vec<Omit> {
        plan.omissions()
            .iter()
            .map(|o| (ifd_code(o.ifd()), o.tag(), o.field_type()))
            .collect()
    }

    /// A refusal is typed: one of the three classes `exif_block` has.
    fn assert_typed_refusal(e: &BlockError) {
        match e {
            BlockError::Source(_)
            | BlockError::Unbuildable { .. }
            | BlockError::OverBudget { .. } => {}
            other => panic!("exif_block refused with an unexpected class: {other:?}"),
        }
    }

    /// Plan under the small budget, then the default when the small refuses.
    fn plan_for(source: &[u8]) -> Result<ExifBlock<'_>, BlockError> {
        let small = BlockOptions::new().with_max_len(SMALL_BUDGET);
        match exif_block(source, &small) {
            Err(BlockError::OverBudget { .. }) => exif_block(source, &BlockOptions::new()),
            other => other,
        }
    }

    /// The facts for one input, or `None` when the library refused it (typed)
    /// or the input is too short. Panics on an untyped refusal, a refusal
    /// where the library has no right to one, or a failed self-check.
    pub fn facts(data: &[u8]) -> Option<Facts> {
        let (base, source) = decode(data)?;
        let plan = match plan_for(source) {
            Ok(p) => p,
            Err(e) => {
                assert_typed_refusal(&e);
                return None;
            }
        };
        // The default budget is the roomy one: a refusal under the small budget
        // must not be a refusal under it.
        let (plan, block) = match plan.write(base) {
            Ok(b) => (plan, b),
            Err(BlockError::OverBudget { .. }) => {
                let big = exif_block(source, &BlockOptions::new())
                    .expect("a plan the small budget made is a plan under the default");
                match big.write(base) {
                    Ok(b) => (big, b),
                    // The scratch image alone is over 16 MiB: not at 64 KiB.
                    Err(e) => panic!("write({base}) under the default budget: {e:?}"),
                }
            }
            // An odd base cannot happen here; a base inside the TIFF header is
            // refused for a non-empty block (DEC-015, SPEC-025 AC2).
            Err(BlockError::BadBase { base: b }) if b < 8 && !plan.is_empty() => return None,
            Err(e) => panic!("write({base}) failed on a plan that was made: {e:?}"),
        };
        let again_plan = plan_for(source).expect("the same input plans twice");
        let again = again_plan
            .write(base)
            .or_else(|_| {
                exif_block(source, &BlockOptions::new())
                    .expect("plans under the default")
                    .write(base)
            })
            .expect("the same input writes twice");
        Some(Facts {
            base,
            source: source.to_vec(),
            be: plan.byte_order() == ByteOrder::Big,
            len: plan.len(),
            exif_pos: plan.exif_position(),
            gps_pos: plan.gps_position(),
            omissions: omit_list(&plan),
            block,
            again,
            again_omissions: omit_list(&again_plan),
            again_len: again_plan.len(),
        })
    }

    // ── the harness's own placer ────────────────────────────────────────────

    /// The placed file: header, zeros to `base`, the block, IFD0 after it with
    /// the pointers as `base + position` (LONG, count 1, ascending tag order),
    /// next-word 0. An empty block places nothing and IFD0 is at 8.
    pub fn place(f: &Facts) -> Vec<u8> {
        let w16 = |v: u16| {
            if f.be {
                v.to_be_bytes()
            } else {
                v.to_le_bytes()
            }
        };
        let w32 = |v: u32| {
            if f.be {
                v.to_be_bytes()
            } else {
                v.to_le_bytes()
            }
        };
        let mut out = Vec::new();
        out.extend_from_slice(if f.be { b"MM\0\x2A" } else { b"II\x2A\0" });
        let ifd0_at = if f.block.is_empty() {
            8
        } else {
            f.base.saturating_add(u32::try_from(f.block.len()).unwrap_or(0))
        };
        out.extend_from_slice(&w32(ifd0_at));
        if !f.block.is_empty() {
            out.resize(usize::try_from(f.base).unwrap_or(0), 0);
            out.extend_from_slice(&f.block);
        }
        let mut ptrs = Vec::new();
        if let Some(p) = f.exif_pos {
            ptrs.push((0x8769u16, f.base.saturating_add(p)));
        }
        if let Some(p) = f.gps_pos {
            ptrs.push((0x8825u16, f.base.saturating_add(p)));
        }
        out.extend_from_slice(&w16(u16::try_from(ptrs.len()).unwrap_or(0)));
        for (tag, value) in ptrs {
            out.extend_from_slice(&w16(tag));
            out.extend_from_slice(&w16(4));
            out.extend_from_slice(&w32(1));
            out.extend_from_slice(&w32(value));
        }
        out.extend_from_slice(&w32(0));
        out
    }

    // ── reading the IFDs back ───────────────────────────────────────────────

    /// `(tag, type, count, payload)` of every entry of IFD `idx`.
    type Row = (u16, u16, u32, Vec<u8>);

    fn by_parent(c: &Container<'_>, kind: IfdKind, parent: usize) -> Option<usize> {
        c.ifds()
            .iter()
            .position(|i| i.kind() == kind && i.parent() == Some(parent))
    }

    fn rows(c: &Container<'_>, idx: usize) -> Vec<Row> {
        c.ifds()
            .get(idx)
            .map(|i| {
                i.entries()
                    .iter()
                    .map(|e| {
                        let payload = c.payload(e).map(<[u8]>::to_vec).unwrap_or_default();
                        (e.tag(), e.field_type(), e.count(), payload)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The three IFDs the block holds, selected by parent (the walk visits
    /// SubIFDs before IFD0's Exif pointer, so the first Exif IFD can be a
    /// SubIFD's): `(Exif, Interop, GPS)` indices.
    fn selected(c: &Container<'_>) -> (Option<usize>, Option<usize>, Option<usize>) {
        let exif = by_parent(c, IfdKind::Exif, 0);
        let interop = exif.and_then(|e| by_parent(c, IfdKind::Interop, e));
        let gps = by_parent(c, IfdKind::Gps, 0);
        (exif, interop, gps)
    }

    fn omitted_in(f: &Facts, code: u8) -> BTreeSet<u16> {
        f.omissions
            .iter()
            .filter(|o| o.0 == code)
            .map(|o| o.1)
            .collect()
    }

    // ── the guards ──────────────────────────────────────────────────────────

    fn length_holds(f: &Facts) -> bool {
        usize::try_from(f.len).ok() == Some(f.block.len())
    }

    fn positions_hold(f: &Facts) -> bool {
        let ok = |p: Option<u32>| p.is_none_or(|p| p % 2 == 0 && p < f.len);
        ok(f.exif_pos)
            && ok(f.gps_pos)
            && f.len % 2 == 0
            && (f.len != 0 || (f.exif_pos.is_none() && f.gps_pos.is_none()))
    }

    fn deterministic_holds(f: &Facts) -> bool {
        f.block == f.again && f.omissions == f.again_omissions && f.len == f.again_len
    }

    fn placed_clean_holds(f: &Facts) -> bool {
        let placed = place(f);
        let (Ok(c), Ok(s)) = (Container::parse(&placed), Container::parse(&f.source)) else {
            return false;
        };
        if !c.is_clean() {
            return false;
        }
        let (se, si, sg) = selected(&s);
        let (pe, pi, pg) = selected(&c);
        // An IFD is in the block when the source has it, whatever its entries.
        let present = se.is_some() == pe.is_some()
            && si.is_some() == pi.is_some()
            && sg.is_some() == pg.is_some();
        let want = 1
            + usize::from(pe.is_some())
            + usize::from(pi.is_some())
            + usize::from(pg.is_some());
        present && c.ifds().len() == want
    }

    fn entries_equal_holds(f: &Facts) -> bool {
        let placed = place(f);
        let (Ok(c), Ok(s)) = (Container::parse(&placed), Container::parse(&f.source)) else {
            return false;
        };
        let (se, si, sg) = selected(&s);
        let (pe, pi, pg) = selected(&c);
        for (code, src, plc) in [(0u8, se, pe), (1, si, pi), (2, sg, pg)] {
            let (Some(src), Some(plc)) = (src, plc) else {
                continue;
            };
            let omitted = omitted_in(f, code);
            let mut want: Vec<Row> = rows(&s, src)
                .into_iter()
                .filter(|r| !omitted.contains(&r.0))
                .collect();
            want.sort_by_key(|r| r.0);
            let mut got = rows(&c, plc);
            // The Interop pointer is rewritten: LONG, count 1, to the placed
            // Interop IFD.
            if code == 0 {
                for r in want.iter_mut().chain(got.iter_mut()) {
                    if r.0 == 0xA005 {
                        *r = (0xA005, 4, 1, Vec::new());
                    }
                }
                if let (Some(row), Some(pi)) = (rows(&c, plc).iter().find(|r| r.0 == 0xA005), pi) {
                    let v: Option<[u8; 4]> = row.3.get(..4).and_then(|b| b.try_into().ok());
                    let value = v.map(|v| {
                        if f.be {
                            u32::from_be_bytes(v)
                        } else {
                            u32::from_le_bytes(v)
                        }
                    });
                    if value != c.ifds().get(pi).map(|i| i.offset()) {
                        return false;
                    }
                }
            }
            if got != want || c.ifds().get(plc).map(|i| i.next()) != Some(0) {
                return false;
            }
        }
        true
    }

    fn omissions_exist_holds(f: &Facts) -> bool {
        let Ok(s) = Container::parse(&f.source) else {
            return false;
        };
        let (se, si, sg) = selected(&s);
        f.omissions.iter().all(|o| {
            let idx = match o.0 {
                0 => se,
                1 => si,
                _ => sg,
            };
            idx.is_some_and(|i| rows(&s, i).iter().any(|r| r.0 == o.1 && r.1 == o.2))
        })
    }

    pub fn holds(g: Guard, f: &Facts) -> bool {
        match g {
            Guard::Length => length_holds(f),
            Guard::Positions => positions_hold(f),
            Guard::Deterministic => deterministic_holds(f),
            Guard::PlacesClean => placed_clean_holds(f),
            Guard::EntriesEqual => entries_equal_holds(f),
            Guard::OmissionsExist => omissions_exist_holds(f),
        }
    }

    /// The guards that fail on `f`.
    pub fn failures(f: &Facts) -> Vec<Guard> {
        GUARDS.iter().copied().filter(|g| !holds(*g, f)).collect()
    }

    /// The fuzz target's body and the layout test's entry point.
    pub fn check(data: &[u8]) {
        let Some(f) = facts(data) else {
            return;
        };
        let failed = failures(&f);
        assert!(
            failed.is_empty(),
            "exif_block invariants failed: {failed:?} (base {})",
            f.base
        );
    }
}
