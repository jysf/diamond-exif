// The SPEC-029 invariants of PNG metadata extraction (DEC-018), shared by the
// fuzz target (`fuzz/fuzz_targets/png_chunks.rs`) and the stable-toolchain
// layout test (`tests/fuzz_layout.rs`) with `include!`. Not a crate root: text
// included at module level, defining one module, `png_metadata_invariants`,
// whose `check` is the harness body.
//
// The extraction is re-derived here from the raw bytes: its own chunk framing,
// its own keyword and header parsing, its own routing and position rules. Only
// the walk's *outcome* is taken from the library (which chunks it recorded and
// which issues it reported, both already pinned by `png_chunk_invariants`);
// whether it halted is derived from those issues, not read from the library.
// The model's answer is compared with `png::read` field by field, so every
// rule that is an iff (a region exists, its variant, its `compressed`, its
// position flags, a block, an issue) is asserted in both directions: a
// constant-true or constant-false implementation fails.

mod png_metadata_invariants {
    use exif_core::png::{
        read, AnyPngIssue, ChunkClass, Png, PngRegion, MAX_PNG_EXIF_BLOCKS, MAX_PNG_REGIONS,
        MAX_PNG_TRAILER_CHUNKS,
    };
    use exif_core::tiff::Container;
    use exif_core::Error;

    /// One region, as plain data: (variant, chunk offset, compressed, name or
    /// keyword range, icc method, data range, after_idat, after_iend). The
    /// variant is 0 Xmp, 1 Icc, 2 RawProfile, 3 Text, 4 Chunk (the
    /// icc-method slot then holds the class: 0 Time, 1 ExifCarrier, 2
    /// XmlCarrier, 3 Jumbf, 4 Other).
    pub(super) type RegionT = (
        u8,
        usize,
        bool,
        (usize, usize),
        u8,
        (usize, usize),
        bool,
        bool,
    );
    /// One Exif block: (chunk offset, tiff_at, tiff range, after_idat,
    /// after_iend).
    pub(super) type BlockT = (usize, usize, (usize, usize), bool, bool);

    /// What an implementation reports, as comparable data.
    #[derive(Clone, Debug, PartialEq)]
    pub(super) struct Observed {
        pub(super) blocks: Vec<BlockT>,
        pub(super) regions: Vec<RegionT>,
        pub(super) issues: Vec<(Error, usize)>,
    }

    struct Framed {
        off: usize,
        ty: [u8; 4],
        data_start: usize,
        data_end: usize,
        end: usize,
    }

    /// Frame the chunk at `p`: a fit is length, type, data and CRC inside the
    /// input with a length of at most 2^31 - 1. Nothing else is checked.
    fn frame(d: &[u8], p: usize) -> Option<Framed> {
        if p.checked_add(12)? > d.len() {
            return None;
        }
        let len = u32::from_be_bytes([d[p], d[p + 1], d[p + 2], d[p + 3]]);
        if len > 0x7FFF_FFFF {
            return None;
        }
        let end = (p as u64) + 12 + (len as u64);
        if end > d.len() as u64 {
            return None;
        }
        Some(Framed {
            off: p,
            ty: [d[p + 4], d[p + 5], d[p + 6], d[p + 7]],
            data_start: p + 8,
            data_end: p + 8 + len as usize,
            end: end as usize,
        })
    }

    fn find_nul(d: &[u8], from: usize, to: usize) -> Option<usize> {
        (from..to).find(|&i| d[i] == 0)
    }

    struct Model {
        issues: Vec<(Error, usize)>,
        halted: bool,
        stopped: bool,
        blocks: Vec<BlockT>,
        regions: Vec<RegionT>,
        exif_seen: usize,
    }

    impl Model {
        fn record(&mut self, e: Error, at: usize) {
            if !self.halted {
                self.issues.push((e, at));
            }
        }

        fn add_region(&mut self, r: RegionT) {
            if self.regions.len() >= MAX_PNG_REGIONS {
                self.record(
                    Error::TooManyRegions {
                        limit: MAX_PNG_REGIONS,
                    },
                    r.1,
                );
                self.stopped = true;
            } else {
                self.regions.push(r);
            }
        }

        fn chunk(&mut self, d: &[u8], c: &Framed, after_idat: bool, after_iend: bool) {
            let off = c.off;
            let (s, e) = (c.data_start, c.data_end);
            match &c.ty {
                b"eXIf" => {
                    if self.exif_seen >= 1 {
                        self.record(Error::ExtraExif { at: off }, off);
                    }
                    if self.exif_seen < MAX_PNG_EXIF_BLOCKS {
                        let improper = e - s >= 6 && &d[s..s + 6] == b"Exif\0\0";
                        let start = if improper { s + 6 } else { s };
                        if improper {
                            self.record(Error::ImproperExifHeader { at: off }, off);
                        }
                        self.blocks
                            .push((off, start, (start, e), after_idat, after_iend));
                    }
                    self.exif_seen += 1;
                }
                b"tEXt" | b"zTXt" | b"iTXt" => {
                    let ty = c.ty;
                    let bad = Error::BadTextChunk { at: off };
                    // The keyword runs to the first NUL however long; only no NUL
                    // leaves nothing to locate. An empty keyword (ExifTool shows
                    // the value as `Tag`) or one of 80 bytes and more is located
                    // with the issue.
                    let Some(k) = find_nul(d, s, e).map(|i| i - s) else {
                        self.record(bad, off);
                        return;
                    };
                    let mut malformed = k == 0 || k > 79;
                    // An iTXt language tag that is not empty (measured, ExifTool
                    // 13.55): the XMP keyword is then plain text, not XMP.
                    let mut tagged = false;
                    let kw = (s, s + k);
                    let mut p = s + k + 1;
                    let (compressed, vstart) = match &ty {
                        b"tEXt" => (false, p),
                        b"zTXt" => {
                            if p >= e {
                                self.record(bad, off);
                                return;
                            }
                            (true, p + 1)
                        }
                        _ => {
                            if p + 2 > e {
                                self.record(bad, off);
                                return;
                            }
                            // Any flag but 0 is compressed; above 1 is malformed.
                            let flag = d[p] != 0;
                            malformed |= d[p] > 1;
                            p += 2;
                            let Some(a) = find_nul(d, p, e) else {
                                self.record(bad, off);
                                return;
                            };
                            let Some(b) = find_nul(d, a + 1, e) else {
                                self.record(bad, off);
                                return;
                            };
                            tagged = a > p;
                            (flag, b + 1)
                        }
                    };
                    if malformed {
                        self.record(bad, off);
                    }
                    let keyword = &d[kw.0..kw.1];
                    let region = if keyword == b"XML:com.adobe.xmp" && !tagged {
                        (
                            0,
                            off,
                            compressed,
                            (0, 0),
                            0,
                            (vstart, e),
                            after_idat,
                            after_iend,
                        )
                    } else if keyword.len() >= 17
                        && (&keyword[..17] == b"Raw profile type "
                            || &keyword[..17] == b"raw profile type ")
                    {
                        (
                            2,
                            off,
                            compressed,
                            (kw.0 + 17, kw.1),
                            0,
                            (vstart, e),
                            after_idat,
                            after_iend,
                        )
                    } else {
                        (
                            3,
                            off,
                            compressed,
                            kw,
                            0,
                            (vstart, e),
                            after_idat,
                            after_iend,
                        )
                    };
                    self.add_region(region);
                }
                b"iCCP" => {
                    let bad = Error::BadIccpChunk { at: off };
                    // The name runs to the first NUL however long (measured,
                    // ExifTool 13.55); no NUL or no method byte leaves nothing to
                    // locate, an empty name or one of 80 bytes and more is
                    // located with the issue.
                    let Some(k) = find_nul(d, s, e).map(|i| i - s) else {
                        self.record(bad, off);
                        return;
                    };
                    let m = s + k + 1;
                    if m >= e {
                        self.record(bad, off);
                        return;
                    }
                    if k == 0 || k > 79 {
                        self.record(bad, off);
                    }
                    self.add_region((
                        1,
                        off,
                        false,
                        (s, s + k),
                        d[m],
                        (m + 1, e),
                        after_idat,
                        after_iend,
                    ));
                }
                other => {
                    // The chunk types ExifTool 13.55 reads as metadata and the
                    // library locates by type (SPEC-037), typed again here.
                    let class = match other {
                        b"tIME" => 0,
                        t if t.eq_ignore_ascii_case(b"zxif") || t.eq_ignore_ascii_case(b"exif") => {
                            1
                        }
                        b"tXMP" | b"meTa" | b"seAl" => 2,
                        b"caBX" => 3,
                        b"dSIG" | b"cpIp" | b"gIFx" | b"gIFg" | b"gIFt" | b"fRAc" => 4,
                        _ => return,
                    };
                    self.add_region((4, off, false, (0, 0), class, (s, e), after_idat, after_iend));
                }
            }
        }
    }

    fn range_of(base: &[u8], part: &[u8]) -> (usize, usize) {
        let start = part.as_ptr() as usize - base.as_ptr() as usize;
        (start, start + part.len())
    }

    /// The library's answer as plain data.
    fn observe(d: &[u8], m: &exif_core::png::PngMetadata<'_>) -> Observed {
        let blocks = m
            .exif_blocks()
            .iter()
            .map(|b| {
                (
                    b.offset(),
                    b.tiff_at(),
                    range_of(d, b.tiff()),
                    b.after_idat(),
                    b.after_iend(),
                )
            })
            .collect();
        let regions = m
            .regions()
            .iter()
            .map(|r| match *r {
                PngRegion::Xmp {
                    offset,
                    compressed,
                    data,
                    after_idat,
                    after_iend,
                } => (
                    0,
                    offset,
                    compressed,
                    (0, 0),
                    0,
                    range_of(d, data),
                    after_idat,
                    after_iend,
                ),
                PngRegion::Icc {
                    offset,
                    name,
                    method,
                    data,
                    after_idat,
                    after_iend,
                } => (
                    1,
                    offset,
                    false,
                    range_of(d, name),
                    method,
                    range_of(d, data),
                    after_idat,
                    after_iend,
                ),
                PngRegion::RawProfile {
                    offset,
                    profile,
                    compressed,
                    data,
                    after_idat,
                    after_iend,
                } => (
                    2,
                    offset,
                    compressed,
                    range_of(d, profile),
                    0,
                    range_of(d, data),
                    after_idat,
                    after_iend,
                ),
                PngRegion::Chunk {
                    offset,
                    class,
                    data,
                    after_idat,
                    after_iend,
                    ..
                } => (
                    4,
                    offset,
                    false,
                    (0, 0),
                    match class {
                        ChunkClass::Time => 0,
                        ChunkClass::ExifCarrier => 1,
                        ChunkClass::XmlCarrier => 2,
                        ChunkClass::Jumbf => 3,
                        ChunkClass::Other => 4,
                        _ => panic!("a ChunkClass the harness does not know"),
                    },
                    range_of(d, data),
                    after_idat,
                    after_iend,
                ),
                PngRegion::Text {
                    offset,
                    keyword,
                    compressed,
                    data,
                    after_idat,
                    after_iend,
                } => (
                    3,
                    offset,
                    compressed,
                    range_of(d, keyword),
                    0,
                    range_of(d, data),
                    after_idat,
                    after_iend,
                ),
                // `PngRegion` is non-exhaustive: a new variant must be modelled.
                _ => panic!("a PngRegion variant the model does not know"),
            })
            .collect();
        let issues = m
            .png_issues()
            .iter()
            .map(|i| (i.error().clone(), i.offset()))
            .collect();
        Observed {
            blocks,
            regions,
            issues,
        }
    }

    /// The model's answer for `d`, given the walk's outcome.
    fn model(d: &[u8], png: &Png<'_>) -> Observed {
        let walk_issues: Vec<(Error, usize)> = png
            .issues()
            .iter()
            .map(|i| (i.error().clone(), i.offset()))
            .collect();
        // A halt is the walk's last issue, and only these four are halts.
        let halted = matches!(
            walk_issues.last(),
            Some((
                Error::Truncated { .. }
                    | Error::BadChunkLength { .. }
                    | Error::TooManyChunks { .. }
                    | Error::TooManyIssues { .. },
                _
            ))
        );
        let mut m = Model {
            issues: walk_issues,
            halted,
            stopped: false,
            blocks: Vec::new(),
            regions: Vec::new(),
            exif_seen: 0,
        };

        // The recorded chunks are the first `n` chunks of the framing
        // sequence (the walker may stop earlier, never later).
        let n = png.chunks().len();
        let mut p = 8usize;
        let mut seen_idat = false;
        let mut iend_end = None;
        for i in 0..n {
            let c = frame(d, p).expect("a recorded chunk frames");
            assert_eq!(c.off, png.chunks()[i].offset(), "chunk offsets agree");
            assert_eq!(c.ty, png.chunks()[i].chunk_type());
            if !m.stopped {
                m.chunk(d, &c, seen_idat, false);
            }
            seen_idat |= &c.ty == b"IDAT";
            p = c.end;
            if &c.ty == b"IEND" {
                iend_end = Some(c.end);
                break;
            }
        }
        assert_eq!(png.iend_offset().is_some(), iend_end.is_some());

        // The trailer: chunks after IEND, framing only, bounded.
        if let Some(mut at) = iend_end {
            let mut scanned = 0;
            while scanned < MAX_PNG_TRAILER_CHUNKS && !m.stopped {
                let Some(c) = frame(d, at) else { break };
                m.chunk(d, &c, seen_idat, true);
                seen_idat |= &c.ty == b"IDAT";
                at = c.end;
                scanned += 1;
            }
        }
        Observed {
            blocks: m.blocks,
            regions: m.regions,
            issues: m.issues,
        }
    }

    /// The harness body with a hook that may corrupt the library's observed
    /// answer before comparison (the red-proofs use it; `check` does not).
    pub(super) fn compare(data: &[u8], tamper: fn(&mut Observed)) {
        let md = read(data);
        let png = match Png::parse(data) {
            Ok(p) => p,
            Err(e) => {
                assert_eq!(md.err(), Some(e), "read is fatal exactly when the walk is");
                return;
            }
        };
        let md = md.expect("read must not be fatal when the walk is not");
        assert_eq!(md.png().chunks().len(), png.chunks().len());

        let want = model(data, &png);
        let mut got = observe(data, &md);
        tamper(&mut got);
        assert_eq!(got.issues, want.issues, "PNG-layer issues");
        assert_eq!(got.blocks, want.blocks, "Exif blocks");
        assert_eq!(got.regions, want.regions, "regions");
        assert!(got.regions.len() <= MAX_PNG_REGIONS);
        assert!(got.blocks.len() <= MAX_PNG_EXIF_BLOCKS);

        // The walker's issues come first and are unchanged.
        assert_eq!(
            &md.png_issues()[..png.issues().len()],
            png.issues(),
            "walker issues must lead"
        );

        // Container independence: the block's container is the bare parse of
        // its tiff slice, and `tiff_at` indexes the input.
        let mut first_ok: Option<*const Container<'_>> = None;
        let mut tiff_issues = 0usize;
        let mut all_ok_clean = true;
        for (b, &(_, tiff_at, (ts, te), _, _)) in md.exif_blocks().iter().zip(&want.blocks) {
            assert_eq!(b.tiff(), &data[ts..te]);
            assert_eq!(tiff_at, ts);
            match (b.container(), Container::parse(&data[ts..te])) {
                (Ok(c), Ok(bare)) => {
                    assert_eq!(c.ifds().len(), bare.ifds().len());
                    assert_eq!(c.entry_count(), bare.entry_count());
                    assert_eq!(c.issues(), bare.issues());
                    tiff_issues += c.issues().len();
                    all_ok_clean &= c.is_clean();
                    if first_ok.is_none() {
                        first_ok = Some(c as *const _);
                    }
                }
                (Err(e), Err(bare)) => {
                    assert_eq!(e, &bare);
                    all_ok_clean = false;
                }
                _ => panic!("block and bare TIFF parse disagree on Ok-ness"),
            }
        }
        assert_eq!(
            md.exif().map(|c| c as *const _),
            first_ok,
            "exif() is the first Ok block"
        );

        // Cleanliness and the layered issue view, both directions.
        assert_eq!(
            md.is_clean(),
            md.png_issues().is_empty() && all_ok_clean,
            "is_clean iff no PNG issue and every block Ok and clean"
        );
        assert_eq!(md.issues().count(), md.png_issues().len() + tiff_issues);
        let png_layer = md
            .issues()
            .filter(|i| matches!(i, AnyPngIssue::Png(_)))
            .count();
        assert_eq!(png_layer, md.png_issues().len());

        // Borrowing: every region and block slice is inside the input.
        for r in md.regions() {
            let (s, e) = range_of(data, r.data());
            assert!(s <= e && e <= data.len());
        }
    }

    /// The harness body: `read` never panics and equals the independent model.
    pub(super) fn check(data: &[u8]) {
        compare(data, |_| {});
    }
}
