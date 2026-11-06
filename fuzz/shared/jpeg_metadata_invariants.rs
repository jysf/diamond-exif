// The SPEC-010 invariants of `jpeg::read`, shared by the fuzz target
// (`fuzz/fuzz_targets/jpeg_segments.rs`) and the stable-toolchain layout test
// (`tests/fuzz_layout.rs`) with `include!`, so the two cannot drift. This file
// is not a crate root; it is text included at module level, and it defines one
// module, `metadata_invariants`, whose `check` is the harness body.
//
// The extraction is re-derived here, independently of the library (a second,
// deliberately naive implementation of rules 1-7 over `Jpeg::segments()`), and
// compared with `read`'s answer. Every rule that is an iff is asserted in BOTH
// directions: an expected item that is missing and an item that was not
// expected both fail, so a constant-true or constant-false implementation
// fails one of them.

mod metadata_invariants {
    use exif_core::jpeg::{
        self, AnyIssue, Jpeg, Region, SegmentKind, MAX_EXIF_BLOCKS, MAX_JPEG_METADATA_ISSUES,
    };
    use exif_core::tiff::Container;
    use exif_core::Error;

    const SIG_EXIF: usize = 6;
    const SIG_XMP: usize = 29;
    const SIG_EXT: usize = 35;
    const SIG_ICC: usize = 12;
    const SIG_IPTC: usize = 14;
    const SIG_JUMBF: usize = 2;

    /// DEC-007, written out independently of the library: a segment is Exif iff
    /// it is APP1, at least six bytes long, and its first FIVE bytes are
    /// `Exif\0` (the sixth is not checked).
    pub fn exif_by_signature(marker: u8, payload: &[u8]) -> bool {
        marker == 0xE1 && payload.len() >= 6 && payload[..5] == *b"Exif\0"
    }

    /// Whether `inner` lies wholly inside `outer` (empty slices included).
    fn inside(outer: &[u8], inner: &[u8]) -> bool {
        let (o, i) = (outer.as_ptr_range(), inner.as_ptr_range());
        i.start >= o.start && i.end <= o.end
    }

    /// A region, reduced to comparable parts.
    #[derive(Debug, PartialEq)]
    enum Exp<'a> {
        Xmp(usize, &'a [u8]),
        Ext(usize, &'a [u8], u32, u32, &'a [u8]),
        Icc(usize, u8, u8, &'a [u8]),
        Iptc(usize, &'a [u8]),
        Jumbf(usize, u16, u32, &'a [u8]),
    }

    fn reduce<'a>(r: &Region<'a>) -> Exp<'a> {
        match *r {
            Region::Xmp { segment, packet } => Exp::Xmp(segment, packet),
            Region::ExtendedXmp {
                segment,
                guid,
                full_len,
                offset,
                data,
            } => Exp::Ext(segment, guid, full_len, offset, data),
            Region::Icc {
                segment,
                seq,
                count,
                data,
            } => Exp::Icc(segment, seq, count, data),
            Region::Iptc { segment, data } => Exp::Iptc(segment, data),
            Region::Jumbf {
                segment,
                en,
                z,
                data,
            } => Exp::Jumbf(segment, en, z, data),
            _ => panic!("a Region variant the harness does not know"),
        }
    }

    fn be32(b: &[u8]) -> u32 {
        u32::from_be_bytes([b[0], b[1], b[2], b[3]])
    }

    /// The Photoshop resource walk, written out again: the located `0x0404`
    /// data and the one fault that stopped the walk, if any.
    fn expect_iptc(seg_off: usize, payload: &[u8]) -> (Vec<&[u8]>, Option<Error>) {
        let mut found = Vec::new();
        let mut pos = SIG_IPTC;
        while pos < payload.len() {
            let here = seg_off + 4 + pos;
            let rest = &payload[pos..];
            let short = |need: usize| {
                Some(Error::RegionTooShort {
                    what: "8BIM resource",
                    at: here,
                    need,
                    have: rest.len(),
                })
            };
            if rest.len() < 4 || &rest[..4] != b"8BIM" {
                return (
                    found,
                    Some(Error::StrayBytes {
                        at: here,
                        len: rest.len(),
                    }),
                );
            }
            if rest.len() < 6 {
                return (found, short(6));
            }
            let id = u16::from_be_bytes([rest[4], rest[5]]);
            if rest.len() < 7 {
                return (found, short(7));
            }
            let pascal = 1 + usize::from(rest[6]);
            let name = pascal + pascal % 2;
            let size_at = 6 + name;
            if rest.len() < size_at + 4 {
                return (found, short(size_at + 4));
            }
            let size = be32(&rest[size_at..]) as usize;
            let end = size_at + 4 + size;
            if rest.len() < end {
                return (found, short(end));
            }
            if id == 0x0404 {
                found.push(&rest[size_at + 4..end]);
            }
            pos += end + size % 2;
        }
        (found, None)
    }

    type Expected<'a> = (Vec<Exp<'a>>, Vec<Error>, Vec<(usize, u8, u8)>);

    /// The whole extraction, re-derived: the expected regions, the expected
    /// extraction issues (when the walk was not halted) and the ICC chunks
    /// `(segment offset, seq, count)`. The ICC aggregate issue is handled by
    /// the caller.
    fn expected<'a>(jpeg: &Jpeg<'a>) -> Expected<'a> {
        let mut regions = Vec::new();
        let mut errors = Vec::new();
        let mut icc = Vec::new();
        let mut exif_seen = 0usize;
        for (index, seg) in jpeg.segments().iter().enumerate() {
            let p = seg.payload();
            let at = seg.offset();
            match seg.kind() {
                SegmentKind::Exif => {
                    if exif_seen > 0 {
                        errors.push(Error::ExtraExif { at });
                    }
                    exif_seen += 1;
                }
                SegmentKind::Xmp => regions.push(Exp::Xmp(index, &p[SIG_XMP..])),
                SegmentKind::ExtendedXmp => {
                    let rest = &p[SIG_EXT..];
                    if rest.len() >= 40 {
                        regions.push(Exp::Ext(
                            index,
                            &rest[..32],
                            be32(&rest[32..]),
                            be32(&rest[36..]),
                            &rest[40..],
                        ));
                    } else {
                        errors.push(Error::RegionTooShort {
                            what: "extended XMP header",
                            at,
                            need: 40,
                            have: rest.len(),
                        });
                    }
                }
                SegmentKind::Icc => {
                    let rest = &p[SIG_ICC..];
                    if rest.len() >= 2 {
                        icc.push((at, rest[0], rest[1]));
                        regions.push(Exp::Icc(index, rest[0], rest[1], &rest[2..]));
                    } else {
                        errors.push(Error::RegionTooShort {
                            what: "ICC chunk header",
                            at,
                            need: 2,
                            have: rest.len(),
                        });
                    }
                }
                SegmentKind::Iptc => {
                    let (found, fault) = expect_iptc(at, p);
                    regions.extend(found.into_iter().map(|d| Exp::Iptc(index, d)));
                    errors.extend(fault);
                }
                SegmentKind::Jumbf => {
                    let rest = &p[SIG_JUMBF..];
                    if rest.len() >= 6 {
                        regions.push(Exp::Jumbf(
                            index,
                            u16::from_be_bytes([rest[0], rest[1]]),
                            be32(&rest[2..]),
                            &rest[6..],
                        ));
                    } else {
                        errors.push(Error::RegionTooShort {
                            what: "JUMBF header",
                            at,
                            need: 6,
                            have: rest.len(),
                        });
                    }
                }
                _ => {}
            }
        }
        (regions, errors, icc)
    }

    /// Whether the ICC chunks are consistent, by a different route than the
    /// library: every count equals the number of chunks and the sequence
    /// numbers, sorted, are exactly `1..=n`.
    fn icc_consistent(chunks: &[(usize, u8, u8)]) -> bool {
        let n = chunks.len();
        if chunks.iter().any(|c| usize::from(c.2) != n) {
            return false;
        }
        let mut seqs: Vec<usize> = chunks.iter().map(|c| usize::from(c.1)).collect();
        seqs.sort_unstable();
        seqs == (1..=n).collect::<Vec<usize>>()
    }

    type Shape = Vec<(u32, String, u32, Option<usize>, Vec<(u16, u16, u32)>)>;

    fn shape(c: &Container<'_>) -> (Shape, usize) {
        let ifds = c
            .ifds()
            .iter()
            .map(|i| {
                (
                    i.offset(),
                    format!("{:?}", i.kind()),
                    i.depth(),
                    i.parent(),
                    i.entries()
                        .iter()
                        .map(|e| (e.tag(), e.field_type(), e.count()))
                        .collect(),
                )
            })
            .collect();
        (ifds, c.issues().len())
    }

    pub fn check(data: &[u8]) {
        // `read` is `Err` exactly when `Jpeg::parse` is, with an equal error.
        let meta = match jpeg::read(data) {
            Ok(meta) => meta,
            Err(error) => {
                assert_eq!(
                    Jpeg::parse(data).err(),
                    Some(error),
                    "read and parse disagree on a fatal input"
                );
                return;
            }
        };
        let walked = Jpeg::parse(data).expect("read Ok but parse Err");
        assert_eq!(meta.jpeg().segments().len(), walked.segments().len());
        assert_eq!(meta.jpeg().issues(), walked.issues());
        let segments = meta.jpeg().segments();

        // The walk was halted by a cap iff its last issue says so.
        let halted = walked.issues().last().is_some_and(|i| {
            matches!(
                i.error(),
                Error::TooManySegments { .. } | Error::TooManyIssues { .. }
            )
        });

        // ── JPEG-layer issues: the walker's, then the extraction's ──────────
        let all = meta.jpeg_issues();
        assert!(
            all.len() <= MAX_JPEG_METADATA_ISSUES,
            "issue total over bound"
        );
        assert!(
            all.starts_with(walked.issues()),
            "walker issues are not the prefix of jpeg_issues"
        );
        for issue in all {
            assert!(issue.offset() <= data.len(), "issue offset past the input");
            assert!(!issue.to_string().is_empty());
        }
        let extraction: Vec<&Error> = all
            .get(walked.issues().len()..)
            .unwrap_or(&[])
            .iter()
            .map(|i| i.error())
            .collect();

        let (want_regions, want_errors, icc) = expected(&walked);

        // Extraction issues, iff: all of the expected ones and no other. After
        // a halt nothing at all is recorded. The ICC aggregate is separate.
        let (got_errors, got_icc): (Vec<&Error>, Vec<&Error>) = extraction
            .iter()
            .partition(|e| !matches!(e, Error::IccChunkMismatch { .. }));
        if halted {
            assert!(extraction.is_empty(), "an issue was recorded after a halt");
        } else {
            let want: Vec<&Error> = want_errors.iter().collect();
            assert_eq!(got_errors, want, "extraction issues");
            // The ICC aggregate: present iff inconsistent, once, and last.
            assert_eq!(
                !got_icc.is_empty(),
                !icc_consistent(&icc),
                "IccChunkMismatch iff the chunks disagree"
            );
            assert!(got_icc.len() <= 1, "more than one IccChunkMismatch");
            if let [Error::IccChunkMismatch { at }] = got_icc[..] {
                assert!(
                    icc.iter().any(|c| c.0 == *at),
                    "IccChunkMismatch names no ICC chunk"
                );
                assert!(
                    matches!(extraction.last(), Some(Error::IccChunkMismatch { .. })),
                    "the ICC aggregate is not last"
                );
            }
        }

        // ── Regions: iff the expected list, in order ────────────────────────
        let got_regions: Vec<Exp<'_>> = meta.regions().iter().map(reduce).collect();
        assert_eq!(got_regions, want_regions, "regions");
        for region in meta.regions() {
            let seg = &segments[region.segment()];
            let kind_ok = matches!(
                (region, seg.kind()),
                (Region::Xmp { .. }, SegmentKind::Xmp)
                    | (Region::ExtendedXmp { .. }, SegmentKind::ExtendedXmp)
                    | (Region::Icc { .. }, SegmentKind::Icc)
                    | (Region::Iptc { .. }, SegmentKind::Iptc)
                    | (Region::Jumbf { .. }, SegmentKind::Jumbf)
            );
            assert!(kind_ok, "region from a segment of the wrong kind");
            assert!(
                inside(seg.payload(), region.data()),
                "region escapes its segment"
            );
            assert!(inside(data, region.data()), "region escapes the input");
        }
        // Spot iffs the design names, restated on their own: an extended-XMP
        // region exists iff its segment's payload is at least 35 + 40 bytes,
        // and a short one costs exactly one RegionTooShort (unless halted).
        for (index, seg) in segments.iter().enumerate() {
            if seg.kind() != SegmentKind::ExtendedXmp {
                continue;
            }
            let has_region = meta
                .regions()
                .iter()
                .any(|r| matches!(r, Region::ExtendedXmp { segment, .. } if *segment == index));
            assert_eq!(
                has_region,
                seg.payload().len() >= 75,
                "extended XMP iff 75 bytes"
            );
            let short = extraction.iter().filter(|e| {
                matches!(e, Error::RegionTooShort { what: "extended XMP header", at, .. } if *at == seg.offset())
            });
            assert_eq!(
                short.count(),
                usize::from(!has_region && !halted),
                "RegionTooShort iff short and not halted"
            );
        }
        let xmp_segments = segments
            .iter()
            .filter(|s| s.kind() == SegmentKind::Xmp)
            .count();
        let xmp_regions = meta
            .regions()
            .iter()
            .filter(|r| matches!(r, Region::Xmp { .. }))
            .count();
        assert_eq!(xmp_regions, xmp_segments, "one Xmp region per Xmp segment");

        // ── Exif blocks ─────────────────────────────────────────────────────
        // The Exif kind is an iff in both directions (DEC-007).
        for seg in segments {
            assert_eq!(
                seg.kind() == SegmentKind::Exif,
                exif_by_signature(seg.marker(), seg.payload()),
                "Exif iff APP1, >= 6 bytes and the first five bytes are Exif\\0"
            );
        }
        let exif_segments: Vec<usize> = segments
            .iter()
            .enumerate()
            .filter(|(_, s)| s.kind() == SegmentKind::Exif)
            .map(|(i, _)| i)
            .collect();
        let blocks = meta.exif_blocks();
        assert!(blocks.len() <= MAX_EXIF_BLOCKS);
        assert_eq!(
            blocks.len(),
            exif_segments.len().min(MAX_EXIF_BLOCKS),
            "block count"
        );
        for (block, &index) in blocks.iter().zip(&exif_segments) {
            assert_eq!(block.segment(), index, "block segment index");
            let payload = segments[index].payload();
            assert_eq!(block.tiff(), &payload[SIG_EXIF..], "tiff slice");
            assert_eq!(
                block.tiff().as_ptr(),
                payload[SIG_EXIF..].as_ptr(),
                "tiff is not borrowed from the segment"
            );
            assert!(inside(data, block.tiff()), "tiff escapes the input");
            // Container independence, both directions.
            match (block.container(), Container::parse(block.tiff())) {
                (Ok(inside_jpeg), Ok(bare)) => {
                    assert_eq!(shape(inside_jpeg), shape(&bare), "container independence")
                }
                (Err(inside_jpeg), Err(bare)) => {
                    assert_eq!(inside_jpeg, &bare, "fatal TIFF error independence")
                }
                (Ok(_), Err(_)) => panic!("Ok inside the JPEG but the bare parse failed"),
                (Err(_), Ok(_)) => panic!("Err inside the JPEG but the bare parse succeeded"),
            }
        }
        // ExtraExif: one per Exif segment after the first, none otherwise
        // (and none after a halt).
        let extras: Vec<usize> = extraction
            .iter()
            .filter_map(|e| match e {
                Error::ExtraExif { at } => Some(*at),
                _ => None,
            })
            .collect();
        let want_extras: Vec<usize> = if halted {
            Vec::new()
        } else {
            exif_segments
                .iter()
                .skip(1)
                .map(|&i| segments[i].offset())
                .collect()
        };
        assert_eq!(extras, want_extras, "ExtraExif issues");
        // `exif()` is the first Ok block's container, and None iff there is none.
        let first_ok = blocks.iter().find_map(|b| b.container().ok());
        match (meta.exif(), first_ok) {
            (Some(a), Some(b)) => assert!(std::ptr::eq(a, b), "exif() is not the first Ok block"),
            (None, None) => {}
            _ => panic!("exif() disagrees with the blocks"),
        }

        // ── The layered view ────────────────────────────────────────────────
        let block_issues: usize = blocks
            .iter()
            .map(|b| b.container().map_or(0, |c| c.issues().len()))
            .sum();
        assert_eq!(
            meta.issues().count(),
            all.len() + block_issues,
            "issues() count"
        );
        let mut jpeg_seen = 0usize;
        let mut last_block = 0usize;
        let mut seen_exif = false;
        for issue in meta.issues() {
            match issue {
                AnyIssue::Jpeg(_) => {
                    assert!(!seen_exif, "a JPEG issue after an Exif issue");
                    jpeg_seen += 1;
                }
                AnyIssue::Exif { block, issue } => {
                    assert!(block < blocks.len(), "AnyIssue::Exif block out of range");
                    assert!(block >= last_block, "Exif issues out of block order");
                    seen_exif = true;
                    last_block = block;
                    assert!(!issue.to_string().is_empty());
                }
                _ => panic!("an AnyIssue variant the harness does not know"),
            }
        }
        assert_eq!(jpeg_seen, all.len());
        // Clean iff no JPEG issue and every block is Ok and clean.
        let want_clean = all.is_empty()
            && blocks
                .iter()
                .all(|b| b.container().is_ok_and(|c| c.is_clean()));
        assert_eq!(meta.is_clean(), want_clean, "is_clean iff every layer is");
    }
}
