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
        MAX_MPF_IMAGES,
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
        Embedded(usize, usize, &'a [u8]),
        Comment(usize, &'a [u8]),
        OtherApp(usize, u8, &'a [u8]),
        Trailer(usize, usize, &'a [u8]),
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
            Region::EmbeddedImage {
                segment,
                offset,
                data,
            } => Exp::Embedded(segment, offset, data),
            Region::Comment { segment, data } => Exp::Comment(segment, data),
            Region::OtherApp {
                segment,
                marker,
                data,
            } => Exp::OtherApp(segment, marker, data),
            Region::Trailer {
                segment,
                offset,
                data,
            } => Exp::Trailer(segment, offset, data),
            _ => panic!("a Region variant the harness does not know"),
        }
    }

    fn be32(b: &[u8]) -> u32 {
        u32::from_be_bytes([b[0], b[1], b[2], b[3]])
    }

    /// The Photoshop resource walk, written out again: the one fault that
    /// stopped the walk, if any. (DEC-012: the region is the whole segment,
    /// not a resource, so the walk finds faults only.)
    fn expect_iptc(seg_off: usize, payload: &[u8]) -> (Vec<&[u8]>, Option<Error>) {
        let found = Vec::new();
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
            pos += end + size % 2;
        }
        (found, None)
    }

    type Expected<'a> = (Vec<Exp<'a>>, Vec<Error>, Vec<(usize, u8, u8)>);

    /// The whole extraction, re-derived: the expected regions, the expected
    /// extraction issues (when the walk was not halted) and the ICC chunks
    /// `(segment offset, seq, count)`. The ICC aggregate issue is handled by
    /// the caller.
    fn expected<'a>(jpeg: &Jpeg<'a>, data: &'a [u8]) -> Expected<'a> {
        let mut regions = Vec::new();
        let mut errors = Vec::new();
        let mut icc = Vec::new();
        let mut exif_seen = 0usize;
        let mut mpf_seen = false;
        // The ranges the embedded images take, to reduce the trailer.
        let mut covered: Vec<(usize, usize)> = Vec::new();
        for (index, seg) in jpeg.segments().iter().enumerate() {
            let p = seg.payload();
            let at = seg.offset();
            // DEC-012, from the marker and the bytes (not from `SegmentKind`):
            // the unrecognised APPn and the comment.
            if seg.marker() == 0xFE {
                regions.push(Exp::Comment(index, p));
            } else if (0xE0..=0xEF).contains(&seg.marker()) && !recognised(seg.marker(), p) {
                let is_mpf = seg.marker() == 0xE2 && p.starts_with(b"MPF\0");
                let benign = (seg.marker() == 0xE0 && p.starts_with(b"JFIF\0"))
                    || (seg.marker() == 0xEE && p.starts_with(b"Adobe\0"));
                if is_mpf {
                    let images = if mpf_seen { None } else { mpf_images(p) };
                    mpf_seen = true;
                    match images {
                        Some((entries, too_many)) => {
                            for (i, size, off) in entries {
                                let start = (at + 8) as u64 + u64::from(off);
                                let end = start + u64::from(size);
                                if end > data.len() as u64 {
                                    errors.push(Error::MpfImageOutsideInput { at, index: i });
                                } else if !data[start as usize..end as usize]
                                    .starts_with(&[0xFF, 0xD8])
                                {
                                    errors.push(Error::MpfImageNotJpeg { at, index: i });
                                } else {
                                    covered.push((start as usize, end as usize));
                                    regions.push(Exp::Embedded(
                                        index,
                                        start as usize,
                                        &data[start as usize..end as usize],
                                    ));
                                }
                            }
                            if too_many {
                                errors.push(Error::TooManyMpfImages {
                                    at,
                                    limit: MAX_MPF_IMAGES,
                                });
                            }
                        }
                        None => {
                            errors.push(Error::MpfMalformed { at });
                            regions.push(Exp::OtherApp(index, seg.marker(), p));
                        }
                    }
                } else if !benign {
                    regions.push(Exp::OtherApp(index, seg.marker(), p));
                }
            }
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
                    // Every Photoshop segment is a region: its payload after
                    // the header, whether or not a resource parses in it.
                    regions.push(Exp::Iptc(index, &p[SIG_IPTC..]));
                    let (_, fault) = expect_iptc(at, p);
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
        // The trailer minus the embedded images, one region per contiguous
        // remainder, found byte by byte (the library sorts and sweeps).
        if let Some(eoi) = jpeg.eoi_offset() {
            let start = eoi + 2;
            let mut taken = vec![false; data.len().saturating_sub(start)];
            for &(a, b) in &covered {
                for pos in a.max(start)..b.min(data.len()) {
                    taken[pos - start] = true;
                }
            }
            let mut pos = 0usize;
            while pos < taken.len() {
                if taken[pos] {
                    pos += 1;
                    continue;
                }
                let from = pos;
                while pos < taken.len() && !taken[pos] {
                    pos += 1;
                }
                regions.push(Exp::Trailer(
                    jpeg.segments().len(),
                    start + from,
                    &data[start + from..start + pos],
                ));
            }
        }
        (regions, errors, icc)
    }

    /// Whether an APPn segment is one of the kinds the library reads (so it is
    /// not an "unrecognised APPn"): Exif, XMP, extended XMP, ICC, Photoshop,
    /// JUMBF. Typed again from DEC-007 and DEC-012.
    fn recognised(marker: u8, p: &[u8]) -> bool {
        exif_by_signature(marker, p)
            || (marker == 0xE1 && p.starts_with(b"http://ns.adobe.com/xap/1.0/\0"))
            || (marker == 0xE1 && p.starts_with(b"http://ns.adobe.com/xmp/extension/\0"))
            || (marker == 0xE2 && p.starts_with(b"ICC_PROFILE\0"))
            || (marker == 0xED && p.starts_with(b"Photoshop 3.0\0"))
            || (marker == 0xEB && p.starts_with(b"JP"))
    }

    /// A second, deliberately naive MPF index reader (CIPA DC-007, typed from
    /// the layout): the `(index, size, offset)` of entries 1 to 15 of the MP
    /// index and whether the index lists more than the limit; `None` when the
    /// segment does not parse. Written without `Container`.
    fn mpf_images(payload: &[u8]) -> Option<(Vec<(usize, u32, u32)>, bool)> {
        let h = &payload[4..];
        let big = match h.get(..2)? {
            b"II" => false,
            b"MM" => true,
            _ => return None,
        };
        let u16_at = |at: usize| -> Option<u16> {
            let b: [u8; 2] = h.get(at..at.checked_add(2)?)?.try_into().ok()?;
            Some(if big { u16::from_be_bytes(b) } else { u16::from_le_bytes(b) })
        };
        let u32_at = |at: usize| -> Option<u32> {
            let b: [u8; 4] = h.get(at..at.checked_add(4)?)?.try_into().ok()?;
            Some(if big { u32::from_be_bytes(b) } else { u32::from_le_bytes(b) })
        };
        if u16_at(2)? != 42 {
            return None;
        }
        let ifd = u32_at(4)? as usize;
        if ifd == 0 {
            return None;
        }
        let count = usize::from(u16_at(ifd)?);
        let table = ifd.checked_add(2)?;
        let end = table.checked_add(count.checked_mul(12)?)?;
        // The table and the 4-byte next pointer must be there.
        h.get(table..end.checked_add(4)?)?;
        // Every entry of the table must itself be readable (a type is not
        // checked here, only at the one entry that is used).
        let mut found: Option<(u16, usize, usize)> = None;
        for i in 0..count {
            let at = table + 12 * i;
            if u16_at(at)? == 0xB002 {
                found = Some((u16_at(at + 2)?, u32_at(at + 4)? as usize, at + 8));
                break;
            }
        }
        let (typ, n, field) = found?;
        let size = match typ {
            1 | 2 | 6 | 7 => 1usize,
            3 | 8 => 2,
            4 | 9 | 11 | 13 => 4,
            5 | 10 | 12 => 8,
            _ => return None,
        };
        let len = n.checked_mul(size)?;
        let bytes = if len <= 4 {
            h.get(field..field.checked_add(len)?)?
        } else {
            let off = u32_at(field)? as usize;
            h.get(off..off.checked_add(len)?)?
        };
        if bytes.len() < 16 {
            return None;
        }
        let whole = bytes.len() / 16;
        let mut entries = Vec::new();
        for i in 1..whole.min(MAX_MPF_IMAGES) {
            let e = &bytes[16 * i..16 * i + 16];
            let rd = |a: usize| {
                let b = [e[a], e[a + 1], e[a + 2], e[a + 3]];
                if big { u32::from_be_bytes(b) } else { u32::from_le_bytes(b) }
            };
            entries.push((i, rd(4), rd(8)));
        }
        Some((entries, whole > MAX_MPF_IMAGES))
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

        let (want_regions, want_errors, icc) = expected(&walked, data);

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
            // A trailer has no segment: its index is one past the last.
            if matches!(region, Region::Trailer { .. }) {
                assert_eq!(region.segment(), segments.len(), "trailer segment index");
                assert!(inside(data, region.data()), "region escapes the input");
                continue;
            }
            let seg = &segments[region.segment()];
            let kind_ok = match region {
                Region::Xmp { .. } => seg.kind() == SegmentKind::Xmp,
                Region::ExtendedXmp { .. } => seg.kind() == SegmentKind::ExtendedXmp,
                Region::Icc { .. } => seg.kind() == SegmentKind::Icc,
                Region::Iptc { .. } => seg.kind() == SegmentKind::Iptc,
                Region::Jumbf { .. } => seg.kind() == SegmentKind::Jumbf,
                Region::Comment { .. } => seg.kind() == SegmentKind::Com,
                Region::OtherApp { .. } | Region::EmbeddedImage { .. } => {
                    matches!(seg.kind(), SegmentKind::App(_))
                }
                _ => false,
            };
            assert!(kind_ok, "region from a segment of the wrong kind");
            // An embedded image lies elsewhere in the input (that is the point).
            if !matches!(region, Region::EmbeddedImage { .. }) {
                assert!(
                    inside(seg.payload(), region.data()),
                    "region escapes its segment"
                );
            }
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
