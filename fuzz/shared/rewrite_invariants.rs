// The SPEC-020 invariants of `exif_core::strip` (DEC-013), shared by the fuzz
// target (`fuzz/fuzz_targets/rewrite_jpeg.rs`) and the stable-toolchain layout
// test (`tests/fuzz_layout.rs`) with `include!`, so the two cannot drift. This
// file is not a crate root; it is text included at module level, and it
// defines one module, `rewrite_invariants`.
//
// The harness holds an INDEPENDENT NAIVE STRIPPER: its own segment walker (a
// second implementation of the walk's marker loop and caps), its own benign
// list and category mapping typed from DEC-012/013, the second MPF reader of
// the SPEC-019 harness (extended with the field positions), and its own
// overlap and removed-byte arithmetic (boolean masks, no ranges). It never
// calls into `exif_core::write`. It predicts the exact output, or the exact
// refusal, for the policy, and the library's answer is compared with it. The
// one thing it borrows from the library is the audit's account of what the Exif
// blocks hold (an Exif block is the TIFF engine's business, covered by the
// `audit` target): `audit(input)` supplies the findings and the failed-block
// count of Exif blocks, nothing else.
//
// Input layout: byte 0 selects the policy (0..=4 the presets in
// `StripPolicy::PRESETS` order, 5 `Custom(none)`, 6 `Custom` from the 16-bit
// little-endian mask in bytes 1 and 2, bit i meaning `Category::ALL[i]`, 7 and
// above `All`), the rest is the JPEG candidate (after bytes 1 and 2 for 6).
//
// Every guard is its own function over `Facts` (the library's answer, flattened)
// and the prediction, so a planted bug in the flattened answer turns exactly its
// guard red (see `tests/fuzz_layout.rs`). Every rule that is an iff is checked in
// BOTH directions. `VerifyFailed` is a harness failure unless the naive
// prediction itself breaks an invariant (then the self-check was right to
// refuse) or the naive arithmetic underflows an MPF field (`MpfFields`).

mod rewrite_invariants {
    use std::ops::Range;

    use exif_core::audit::{audit, Category, FileAudit, ItemKind, RegionKind};
    use exif_core::jpeg::{Jpeg, MAX_JPEG_ISSUES, MAX_JPEG_SEGMENTS, MAX_MPF_IMAGES};
    use exif_core::policy::{StripPolicy, StripSpec};
    use exif_core::write::{Check, RemovalKind, UnstrippableReason};
    use exif_core::{strip, StripError};

    // ── the policy byte ────────────────────────────────────────────────────

    /// Decode the leading bytes into a policy and the JPEG candidate.
    pub fn decode(data: &[u8]) -> (StripPolicy, &[u8]) {
        let Some(&b0) = data.first() else {
            return (StripPolicy::All, data);
        };
        match b0 {
            0..=4 => (StripPolicy::PRESETS[usize::from(b0)], &data[1..]),
            5 => (StripPolicy::Custom(StripSpec::none()), &data[1..]),
            6 => {
                let mask = u16::from_le_bytes([
                    data.get(1).copied().unwrap_or(0),
                    data.get(2).copied().unwrap_or(0),
                ]);
                let mut spec = StripSpec::none();
                for (i, c) in Category::ALL.iter().enumerate() {
                    if mask & (1u16 << i) != 0 {
                        spec = spec.include(*c);
                    }
                }
                (StripPolicy::Custom(spec), data.get(3..).unwrap_or(&[]))
            }
            _ => (StripPolicy::All, &data[1..]),
        }
    }

    // ── the library's answer, flattened ────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub struct RemFact {
        pub category: Category,
        pub kind: &'static str,
        pub offset: usize,
        pub len: usize,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct OkFacts {
        pub bytes: Vec<u8>,
        pub removals: Vec<RemFact>,
        pub patches: Vec<(usize, usize)>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum ErrFact {
        /// Unsupported or NotJpeg (the harness does not sniff).
        NotAJpeg,
        Unstrippable(&'static str, usize),
        NeedsEntryEdit(Vec<Category>),
        MpfNotIndexed(usize),
        Overlap(usize),
        VerifyFailed(&'static str),
    }

    /// What the harness learned about an `Ok` output by reading it.
    #[derive(Debug, Clone, PartialEq)]
    pub struct OutFacts {
        /// Categories of the findings of `audit(output)`.
        pub categories: Vec<Category>,
        /// Kept-finding keys of the input and of the output (see `kept_keys`).
        pub kept_in: (Vec<String>, usize),
        pub kept_out: (Vec<String>, usize),
        /// Whether the scan region of the output equals the input's.
        pub scan_same: bool,
        /// `strip(output, policy)`: bytes equal and no records, or not.
        pub idempotent: bool,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct Facts {
        pub result: Result<OkFacts, ErrFact>,
        /// A second `strip` of the same input gave the same answer.
        pub deterministic: bool,
        pub out: Option<OutFacts>,
    }

    fn kind_name(k: RemovalKind) -> &'static str {
        match k {
            RemovalKind::Exif => "exif",
            RemovalKind::MpfIndex => "mpf-index",
            RemovalKind::Xmp => "xmp",
            RemovalKind::ExtendedXmp => "extended-xmp",
            RemovalKind::Iptc => "iptc",
            RemovalKind::Icc => "icc",
            RemovalKind::Jumbf => "jumbf",
            RemovalKind::EmbeddedImage => "embedded-image",
            RemovalKind::Comment => "comment",
            RemovalKind::OtherApp => "other-app",
            RemovalKind::Trailer => "trailer",
            _ => panic!("a RemovalKind the harness does not know"),
        }
    }

    fn flatten_err(e: &StripError) -> ErrFact {
        match e {
            StripError::Unsupported { .. } | StripError::NotJpeg => ErrFact::NotAJpeg,
            StripError::Unstrippable { reason, offset } => ErrFact::Unstrippable(
                match reason {
                    UnstrippableReason::BadSegmentLength => "bad-length",
                    UnstrippableReason::TooManySegments => "too-many-segments",
                    UnstrippableReason::TooManyIssues => "too-many-issues",
                    _ => "other",
                },
                *offset,
            ),
            StripError::NeedsEntryEdit { categories } => {
                ErrFact::NeedsEntryEdit(categories.categories().collect())
            }
            StripError::MpfNotIndexed { offset } => ErrFact::MpfNotIndexed(*offset),
            StripError::Overlap { offset } => ErrFact::Overlap(*offset),
            StripError::VerifyFailed { check } => ErrFact::VerifyFailed(match check {
                Check::Preview => "preview",
                Check::Kept => "kept",
                Check::Scan => "scan",
                Check::Walk => "walk",
                Check::Idempotent => "idempotent",
                Check::MpfFields => "mpf-fields",
                _ => "other",
            }),
            _ => panic!("a StripError the harness does not know"),
        }
    }

    fn flatten(r: &Result<exif_core::Stripped, StripError>) -> Result<OkFacts, ErrFact> {
        match r {
            Ok(s) => Ok(OkFacts {
                bytes: s.bytes().to_vec(),
                removals: s
                    .removals()
                    .iter()
                    .map(|r| RemFact {
                        category: r.category,
                        kind: kind_name(r.kind),
                        offset: r.offset,
                        len: r.len,
                    })
                    .collect(),
                patches: s.patches().iter().map(|p| (p.offset, p.len)).collect(),
            }),
            Err(e) => Err(flatten_err(e)),
        }
    }

    /// The keys of the findings outside `spec` and the total byte count of the
    /// `Trailer` regions among them. `skip_exif` leaves out tag findings made in
    /// Exif blocks (the Exif segments are gone when every Exif category goes).
    fn kept_keys(a: &FileAudit<'_>, spec: StripSpec, skip_exif: bool) -> (Vec<String>, usize) {
        let mut keys = Vec::new();
        let mut trailer = 0usize;
        for f in a.findings() {
            if spec.contains(f.category) || (skip_exif && f.at.origin().block.is_some()) {
                continue;
            }
            match f.at.kind() {
                ItemKind::Region {
                    kind: RegionKind::Trailer,
                    len,
                    ..
                } => trailer += len,
                ItemKind::Region { kind, .. } => keys.push(format!("{:?}/{kind:?}", f.category)),
                ItemKind::Tag { ifd, tag, .. } => {
                    keys.push(format!("{:?}/{ifd:?}/{tag}", f.category))
                }
            }
        }
        keys.sort();
        (keys, trailer)
    }

    const EXIF9: [Category; 9] = [
        Category::Gps,
        Category::DeviceId,
        Category::Authorship,
        Category::FreeText,
        Category::Timestamps,
        Category::CameraModel,
        Category::Thumbnail,
        Category::MakerNote,
        Category::Software,
    ];

    fn all_exif_removed(spec: StripSpec) -> bool {
        EXIF9.iter().all(|c| spec.contains(*c))
    }

    /// Run the library and read its output.
    pub fn facts(input: &[u8], policy: StripPolicy) -> Facts {
        let spec = policy.spec();
        let first = strip(input, policy);
        let second = strip(input, policy);
        let out = first.as_ref().ok().map(|s| {
            let again = strip(s.bytes(), policy);
            let before = audit(input);
            let after = audit(s.bytes());
            OutFacts {
                categories: after.findings().iter().map(|f| f.category).collect(),
                kept_in: kept_keys(&before, spec, all_exif_removed(spec)),
                kept_out: kept_keys(&after, spec, false),
                scan_same: match (Jpeg::parse(input), Jpeg::parse(s.bytes())) {
                    (Ok(a), Ok(b)) => a.scan_region() == b.scan_region(),
                    _ => false,
                },
                idempotent: matches!(&again, Ok(t)
                    if t.bytes() == s.bytes() && t.removals().is_empty() && t.patches().is_empty()),
            }
        });
        Facts {
            result: flatten(&first),
            deterministic: first == second,
            out,
        }
    }

    // ── the naive stripper ─────────────────────────────────────────────────

    #[derive(Debug, Clone, PartialEq)]
    pub enum Pred {
        NotAJpeg,
        Unstrippable(&'static str, usize),
        NeedsEntryEdit(Vec<Category>),
        MpfNotIndexed(usize),
        Overlap(usize),
        /// A primary-size or offset field would go below zero.
        MpfUnderflow,
        Ok {
            bytes: Vec<u8>,
            removals: Vec<RemFact>,
            patches: Vec<(usize, usize)>,
        },
    }

    struct NSeg {
        off: usize,
        marker: u8,
        end: usize,
    }

    struct NWalk {
        segs: Vec<NSeg>,
        issues: usize,
        halted: bool,
        refusal: Option<(&'static str, usize)>,
        scan_start: Option<usize>,
        eoi: Option<usize>,
    }

    impl NWalk {
        fn record(&mut self, what: &'static str, at: usize) {
            if self.halted {
                return;
            }
            if self.issues >= MAX_JPEG_ISSUES {
                self.issues += 1;
                self.halted = true;
                self.refusal.get_or_insert(("too-many-issues", at));
                return;
            }
            self.issues += 1;
            match what {
                "bad-length" => {
                    self.refusal.get_or_insert(("bad-length", at));
                }
                "too-many-segments" => {
                    self.refusal.get_or_insert(("too-many-segments", at));
                    self.halted = true;
                }
                _ => {}
            }
        }
    }

    /// The marker loop again, typed from ITU-T T.81 and DEC-006 (not copied from
    /// the library): stray runs, fill bytes, `FF 00`, standalone markers, a
    /// length-carrying marker with its length checks, the scan skip.
    fn walk(d: &[u8]) -> NWalk {
        let len = d.len();
        let mut w = NWalk {
            segs: Vec::new(),
            issues: 0,
            halted: false,
            refusal: None,
            scan_start: None,
            eoi: None,
        };
        let mut p = 2usize;
        'outer: while p < len && !w.halted {
            if d[p] != 0xFF {
                let run = d[p..].iter().position(|&b| b == 0xFF).unwrap_or(len - p);
                w.record("stray", p);
                p += run;
                continue;
            }
            // Skip fill: p ends on the marker's own FF.
            let m = loop {
                match d.get(p + 1) {
                    None => {
                        w.record("truncated", p);
                        break 'outer;
                    }
                    Some(&0xFF) => p += 1,
                    Some(&m) => break m,
                }
            };
            match m {
                0x00 => {
                    w.record("stray", p);
                    p += 2;
                }
                0xD9 => {
                    w.eoi = Some(p);
                    break;
                }
                0x01 | 0xD0..=0xD8 => {
                    w.record("stray", p);
                    p += 2;
                }
                _ => {
                    let Some(hdr) = d.get(p + 2..p + 4) else {
                        w.record("truncated", p);
                        break;
                    };
                    let declared = usize::from(u16::from_be_bytes([hdr[0], hdr[1]]));
                    if declared < 2 {
                        w.record("bad-length", p);
                        p += 4;
                        continue;
                    }
                    let end = p + declared + 2;
                    if end > len {
                        w.record("truncated", p);
                        break;
                    }
                    if w.segs.len() >= MAX_JPEG_SEGMENTS {
                        w.record("too-many-segments", p);
                        break;
                    }
                    w.segs.push(NSeg {
                        off: p,
                        marker: m,
                        end,
                    });
                    if m == 0xDA {
                        w.scan_start.get_or_insert(p);
                        // Entropy data: stuffed FF 00, restarts D0..D7 and FF
                        // fill are data; the next other marker ends it.
                        let mut i = end;
                        p = loop {
                            let Some(j) =
                                d.get(i..).and_then(|r| r.iter().position(|&b| b == 0xFF))
                            else {
                                break len;
                            };
                            let j = i + j;
                            match d.get(j + 1) {
                                None => break len,
                                Some(0x00) | Some(0xD0..=0xD7) => i = j + 2,
                                Some(0xFF) => i = j + 1,
                                Some(_) => break j,
                            }
                        };
                    } else {
                        p = end;
                    }
                }
            }
        }
        if !w.halted {
            if w.scan_start.is_none() {
                w.record("missing-sos", len);
            }
            if w.eoi.is_none() {
                w.record("missing-eoi", len);
            }
        }
        w
    }

    /// One MP index, read naively.
    struct NMpf {
        /// Input offset of the byte after `MPF\0`.
        header: usize,
        big: bool,
        /// `(size_field, offset_field, size, offset)` per examined entry.
        entries: Vec<(usize, usize, u32, u32)>,
    }

    /// The second, naive MPF reader of `jpeg_metadata_invariants.rs`, extended
    /// with the positions of the fields. `None` when the segment does not parse.
    fn read_mpf(d: &[u8], seg: &NSeg) -> Option<NMpf> {
        let payload = &d[seg.off + 4..seg.end];
        let base = seg.off + 8;
        let h = payload.get(4..)?;
        let big = match h.get(..2)? {
            b"II" => false,
            b"MM" => true,
            _ => return None,
        };
        let u16_at = |at: usize| -> Option<u16> {
            let b: [u8; 2] = h.get(at..at.checked_add(2)?)?.try_into().ok()?;
            Some(if big {
                u16::from_be_bytes(b)
            } else {
                u16::from_le_bytes(b)
            })
        };
        let u32_at = |at: usize| -> Option<u32> {
            let b: [u8; 4] = h.get(at..at.checked_add(4)?)?.try_into().ok()?;
            Some(if big {
                u32::from_be_bytes(b)
            } else {
                u32::from_le_bytes(b)
            })
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
        h.get(table..end.checked_add(4)?)?;
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
        // The entries are 16 bytes each, so the data is out of line (> 4).
        let data_at = if len <= 4 {
            field
        } else {
            u32_at(field)? as usize
        };
        let bytes = h.get(data_at..data_at.checked_add(len)?)?;
        if bytes.len() < 16 {
            return None;
        }
        let whole = bytes.len() / 16;
        let mut entries = Vec::new();
        for i in 0..whole.min(MAX_MPF_IMAGES) {
            let e = &bytes[16 * i..16 * i + 16];
            let rd = |a: usize| {
                let b = [e[a], e[a + 1], e[a + 2], e[a + 3]];
                if big {
                    u32::from_be_bytes(b)
                } else {
                    u32::from_le_bytes(b)
                }
            };
            let abs = base + data_at + 16 * i;
            entries.push((abs + 4, abs + 8, rd(4), rd(8)));
        }
        Some(NMpf {
            header: base,
            big,
            entries,
        })
    }

    #[derive(Clone)]
    struct NTarget {
        range: Range<usize>,
        category: Category,
        kind: &'static str,
    }

    fn spec_cats(spec: StripSpec) -> Vec<Category> {
        spec.categories().collect()
    }

    /// Predict the output of `strip` for a candidate that starts `FF D8`.
    pub fn predict(d: &[u8], spec: StripSpec) -> Pred {
        if !d.starts_with(&[0xFF, 0xD8]) {
            return Pred::NotAJpeg;
        }
        let w = walk(d);
        if let Some((reason, at)) = w.refusal {
            return Pred::Unstrippable(reason, at);
        }
        let len = d.len();
        let payload = |s: &NSeg| &d[s.off + 4..s.end];
        let starts = |s: &NSeg, sig: &[u8]| payload(s).starts_with(sig);

        // Segment typing, in the library's documented order, typed here.
        #[derive(PartialEq, Clone, Copy)]
        enum T {
            Jfif,
            Exif,
            Xmp,
            Ext,
            Icc,
            Iptc,
            Jumbf,
            App,
            Com,
            Other,
        }
        let typed: Vec<T> = w
            .segs
            .iter()
            .map(|s| match s.marker {
                0xE0 if starts(s, b"JFIF\0") => T::Jfif,
                0xE1 if payload(s).len() >= 6 && starts(s, b"Exif\0") => T::Exif,
                0xE1 if starts(s, b"http://ns.adobe.com/xap/1.0/\0") => T::Xmp,
                0xE1 if starts(s, b"http://ns.adobe.com/xmp/extension/\0") => T::Ext,
                0xE2 if starts(s, b"ICC_PROFILE\0") => T::Icc,
                0xED if starts(s, b"Photoshop 3.0\0") => T::Iptc,
                0xEB if starts(s, b"JP") => T::Jumbf,
                0xE0..=0xEF => T::App,
                0xFE => T::Com,
                _ => T::Other,
            })
            .collect();

        // The MPF index: the first APP2 `MPF\0` segment, if it parses.
        let mut mpf: Option<(usize, NMpf)> = None;
        let mut first_mpf_seen = false;
        for (i, s) in w.segs.iter().enumerate() {
            if typed[i] == T::App && s.marker == 0xE2 && starts(s, b"MPF\0") && !first_mpf_seen {
                first_mpf_seen = true;
                if let Some(m) = read_mpf(d, s) {
                    mpf = Some((i, m));
                }
            }
        }
        // Located embedded images: entries 1.. whose range is inside the input
        // and starts FF D8.
        let mut images: Vec<(usize, Range<usize>)> = Vec::new();
        if let Some((_, m)) = &mpf {
            for (k, &(_, _, size, offset)) in m.entries.iter().enumerate().skip(1) {
                let start = m.header.checked_add(offset as usize);
                let end = start.and_then(|s| s.checked_add(size as usize));
                if let (Some(s), Some(e)) = (start, end) {
                    if e <= len && d[s..e].starts_with(&[0xFF, 0xD8]) {
                        images.push((k, s..e));
                    }
                }
            }
        }
        // Trailer pieces: bytes after the EOI that no image covers.
        let mut pieces: Vec<Range<usize>> = Vec::new();
        if let Some(eoi) = w.eoi {
            let start = eoi + 2;
            let mut covered = vec![false; len];
            for (_, r) in &images {
                for b in &mut covered[r.clone()] {
                    *b = true;
                }
            }
            let mut i = start;
            while i < len {
                if covered[i] {
                    i += 1;
                    continue;
                }
                let s = i;
                while i < len && !covered[i] {
                    i += 1;
                }
                pieces.push(s..i);
            }
        }

        let mut targets: Vec<NTarget> = Vec::new();
        let mut kept_segment = vec![true; w.segs.len()];
        let seg_range = |s: &NSeg| s.off..s.end;
        for (i, s) in w.segs.iter().enumerate() {
            let (cat, kind) = match typed[i] {
                T::Xmp => (Category::Xmp, "xmp"),
                T::Ext => (Category::Xmp, "extended-xmp"),
                T::Icc => (Category::Icc, "icc"),
                T::Iptc => (Category::Iptc, "iptc"),
                T::Jumbf => (Category::Jumbf, "jumbf"),
                T::Com => (Category::FreeText, "comment"),
                T::App => {
                    let adobe = s.marker == 0xEE && starts(s, b"Adobe\0");
                    let index = mpf.as_ref().is_some_and(|(k, _)| *k == i);
                    if adobe || index {
                        continue;
                    }
                    (Category::OtherSegment, "other-app")
                }
                _ => continue,
            };
            if spec.contains(cat) {
                kept_segment[i] = false;
                targets.push(NTarget {
                    range: seg_range(s),
                    category: cat,
                    kind,
                });
            }
        }

        // Exif (DEC-013 part 4), over the audit's account of the blocks.
        let exif_idx: Vec<usize> = (0..w.segs.len()).filter(|&i| typed[i] == T::Exif).collect();
        if !exif_idx.is_empty() {
            let a = audit(d);
            let in_exif = |f: &exif_core::audit::Finding<'_>| f.at.origin().block.is_some();
            if all_exif_removed(spec) {
                for (n, &i) in exif_idx.iter().enumerate() {
                    // The category: the first EXIF9 finding of that block.
                    let cat = a
                        .findings()
                        .iter()
                        .find(|f| f.at.origin().block == Some(n) && EXIF9.contains(&f.category))
                        .map_or(Category::Thumbnail, |f| f.category);
                    kept_segment[i] = false;
                    targets.push(NTarget {
                        range: seg_range(&w.segs[i]),
                        category: cat,
                        kind: "exif",
                    });
                }
            } else {
                let touches_exif9 = EXIF9.iter().any(|c| spec.contains(*c));
                // Any walk issue of any Exif block counts as "could not be read".
                let unreadable = touches_exif9
                    && (a.health().failed_blocks > 0
                        || a.health().exif_issues > 0
                        || exif_idx.len() > 16);
                let found: Vec<Category> = Category::ALL
                    .iter()
                    .copied()
                    .filter(|c| {
                        spec.contains(*c)
                            && a.findings().iter().any(|f| in_exif(f) && f.category == *c)
                    })
                    .collect();
                if unreadable || !found.is_empty() {
                    let mut cats: Vec<Category> = Category::ALL
                        .iter()
                        .copied()
                        .filter(|c| {
                            (EXIF9.contains(c) && spec.contains(*c))
                                || (!EXIF9.contains(c) && found.contains(c))
                        })
                        .collect();
                    cats.sort_by_key(|c| Category::ALL.iter().position(|x| x == c));
                    return Pred::NeedsEntryEdit(cats);
                }
            }
        }

        // Trailers, embedded images, the MPF index.
        let mut kept_image = vec![true; images.len()];
        let mut kept_piece = vec![true; pieces.len()];
        if spec.contains(Category::OtherSegment) {
            for (k, r) in pieces.iter().enumerate() {
                kept_piece[k] = false;
                targets.push(NTarget {
                    range: r.clone(),
                    category: Category::OtherSegment,
                    kind: "trailer",
                });
            }
        }
        let index_removed = spec.contains(Category::EmbeddedImage) && mpf.is_some();
        if spec.contains(Category::EmbeddedImage) {
            for (k, (_, r)) in images.iter().enumerate() {
                kept_image[k] = false;
                targets.push(NTarget {
                    range: r.clone(),
                    category: Category::EmbeddedImage,
                    kind: "embedded-image",
                });
            }
            if let Some((i, _)) = &mpf {
                kept_segment[*i] = false;
                targets.push(NTarget {
                    range: seg_range(&w.segs[*i]),
                    category: Category::EmbeddedImage,
                    kind: "mpf-index",
                });
            }
        }
        targets.sort_by_key(|t| (t.range.start, t.range.end));

        // Byte masks: removed, and kept.
        let mut removed = vec![false; len];
        for t in &targets {
            for b in &mut removed[t.range.clone()] {
                *b = true;
            }
        }
        let removed_before = |upto: usize| removed[..upto].iter().filter(|b| **b).count();
        let removed_in = |a: usize, b: usize| removed[a..b].iter().filter(|x| **x).count();

        // MpfNotIndexed (and its two documented extensions).
        // Pinned order (the library's): the index's own unlocated entries first,
        // then the other `MPF\0` segments.
        if let (Some((ki, m)), false) = (&mpf, spec.contains(Category::EmbeddedImage)) {
            let located: Vec<usize> = images.iter().map(|(k, _)| *k).collect();
            for (k, &(_, _, _, offset)) in m.entries.iter().enumerate().skip(1) {
                if located.contains(&k) {
                    continue;
                }
                let Some(pointed) = m.header.checked_add(offset as usize) else {
                    continue;
                };
                if pointed < len && removed[m.header..=pointed].iter().any(|b| *b) {
                    return Pred::MpfNotIndexed(w.segs[*ki].off);
                }
            }
        }
        for (i, s) in w.segs.iter().enumerate() {
            let mpf_like = typed[i] == T::App && s.marker == 0xE2 && starts(s, b"MPF\0");
            let is_index = mpf.as_ref().is_some_and(|(k, _)| *k == i);
            if !mpf_like || is_index || !kept_segment[i] {
                continue;
            }
            let later = removed[s.end..].iter().any(|b| *b);
            let after_removed_index = index_removed && mpf.as_ref().is_some_and(|(k, _)| *k < i);
            if later || after_removed_index {
                return Pred::MpfNotIndexed(s.off);
            }
        }

        // Overlap.
        let mut kept = vec![false; len];
        for b in &mut kept[..2] {
            *b = true;
        }
        if let Some(sos) = w.segs.iter().find(|s| s.marker == 0xDA) {
            let end = w.eoi.map_or(len, |e| e + 2);
            for b in &mut kept[sos.off..end] {
                *b = true;
            }
        }
        for (i, s) in w.segs.iter().enumerate() {
            if kept_segment[i] {
                for b in &mut kept[s.off..s.end] {
                    *b = true;
                }
            }
        }
        for (k, (_, r)) in images.iter().enumerate() {
            if kept_image[k] {
                for b in &mut kept[r.clone()] {
                    *b = true;
                }
            }
        }
        for (k, r) in pieces.iter().enumerate() {
            if kept_piece[k] {
                for b in &mut kept[r.clone()] {
                    *b = true;
                }
            }
        }
        if let Some(at) = (0..len).find(|&i| removed[i] && kept[i]) {
            return Pred::Overlap(at);
        }

        // MPF patches (images stay, removals happen).
        let mut patches: Vec<(usize, [u8; 4])> = Vec::new();
        if let (Some((_, m)), false) = (&mpf, spec.contains(Category::EmbeddedImage)) {
            if !targets.is_empty() {
                let enc = |v: u32| {
                    if m.big {
                        v.to_be_bytes()
                    } else {
                        v.to_le_bytes()
                    }
                };
                let primary_end = w.eoi.map_or(len, |e| e + 2);
                for (k, &(size_field, offset_field, size, offset)) in m.entries.iter().enumerate() {
                    if k == 0 {
                        let gone = removed_in(0, primary_end);
                        let Some(new) = u32::try_from(gone).ok().and_then(|g| size.checked_sub(g))
                        else {
                            return Pred::MpfUnderflow;
                        };
                        if new != size {
                            patches.push((size_field, enc(new)));
                        }
                    } else if let Some((_, r)) = images.iter().find(|(j, _)| *j == k) {
                        let gone = removed_in(m.header, r.start);
                        let Some(new) =
                            u32::try_from(gone).ok().and_then(|g| offset.checked_sub(g))
                        else {
                            return Pred::MpfUnderflow;
                        };
                        if new != offset {
                            patches.push((offset_field, enc(new)));
                        }
                    }
                }
            }
        }

        // The output: the input with the patches written, then the removed
        // bytes dropped.
        let mut data = d.to_vec();
        for (at, bytes) in &patches {
            data[*at..*at + 4].copy_from_slice(bytes);
        }
        let bytes: Vec<u8> = data
            .iter()
            .zip(&removed)
            .filter(|(_, r)| !**r)
            .map(|(b, _)| *b)
            .collect();
        Pred::Ok {
            bytes,
            removals: targets
                .iter()
                .map(|t| RemFact {
                    category: t.category,
                    kind: t.kind,
                    offset: t.range.start,
                    len: t.range.end - t.range.start,
                })
                .collect(),
            patches: patches
                .iter()
                .map(|(at, _)| (*at - removed_before(*at), 4))
                .collect(),
        }
    }

    // ── the guards ─────────────────────────────────────────────────────────

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Guard {
        /// `Ok` output, removal and patch records equal the prediction.
        Prediction,
        /// Length at most the input's, equal iff nothing was removed.
        Length,
        /// The scan region is byte-identical.
        Scan,
        /// The output's audit has no category of the policy.
        RemovedCategories,
        /// Every category the policy keeps has the findings it had.
        KeptFindings,
        /// Stripping the output changes nothing.
        Idempotent,
        /// Stripping twice gives the same answer.
        Deterministic,
        /// The refusal class (and its detail) equals the prediction.
        ErrClass,
        /// `VerifyFailed` only where the prediction itself is wrong or underflows.
        VerifyAllowed,
        /// The `removals` and `patches` records agree with the bytes.
        Records,
    }

    pub const GUARDS: [Guard; 10] = [
        Guard::Prediction,
        Guard::Length,
        Guard::Scan,
        Guard::RemovedCategories,
        Guard::KeptFindings,
        Guard::Idempotent,
        Guard::Deterministic,
        Guard::ErrClass,
        Guard::VerifyAllowed,
        Guard::Records,
    ];

    fn out_matches_pred(ok: &OkFacts, pred: &Pred) -> bool {
        matches!(pred, Pred::Ok { bytes, .. } if *bytes == ok.bytes)
    }

    /// Whether the guard `g` holds. Each guard stands alone: it reads only its
    /// own part of the facts and the prediction.
    pub fn holds(g: Guard, input: &[u8], spec: StripSpec, f: &Facts, pred: &Pred) -> bool {
        match g {
            Guard::Prediction => match (&f.result, pred) {
                (Ok(ok), Pred::Ok { .. }) => out_matches_pred(ok, pred),
                (Ok(_), _) => false,
                // A refusal is judged by `ErrClass`.
                (Err(_), _) => true,
            },
            Guard::Records => match (&f.result, pred) {
                (
                    Ok(ok),
                    Pred::Ok {
                        removals, patches, ..
                    },
                ) => ok.removals == *removals && ok.patches == *patches,
                (Ok(_), _) => false,
                (Err(_), _) => true,
            },
            Guard::Length => match &f.result {
                Ok(ok) => {
                    ok.bytes.len() <= input.len()
                        && (ok.bytes.len() == input.len()) == ok.removals.is_empty()
                }
                Err(_) => true,
            },
            Guard::Scan => f.out.as_ref().is_none_or(|o| o.scan_same),
            Guard::RemovedCategories => f
                .out
                .as_ref()
                .is_none_or(|o| o.categories.iter().all(|c| !spec.contains(*c))),
            Guard::KeptFindings => f.out.as_ref().is_none_or(|o| o.kept_in == o.kept_out),
            Guard::Idempotent => f.out.as_ref().is_none_or(|o| o.idempotent),
            Guard::Deterministic => f.deterministic,
            Guard::ErrClass => match (&f.result, pred) {
                (Ok(_), _) => true,
                (Err(ErrFact::NotAJpeg), Pred::NotAJpeg) => true,
                (Err(ErrFact::Unstrippable(r, o)), Pred::Unstrippable(pr, po)) => {
                    r == pr && o == po
                }
                (Err(ErrFact::NeedsEntryEdit(c)), Pred::NeedsEntryEdit(pc)) => c == pc,
                (Err(ErrFact::MpfNotIndexed(o)), Pred::MpfNotIndexed(po)) => o == po,
                (Err(ErrFact::Overlap(o)), Pred::Overlap(po)) => o == po,
                (Err(ErrFact::VerifyFailed(_)), _) => true,
                _ => false,
            },
            Guard::VerifyAllowed => match &f.result {
                Err(ErrFact::VerifyFailed(check)) => match pred {
                    Pred::MpfUnderflow => *check == "mpf-fields",
                    Pred::Ok { bytes, .. } => predicted_output_breaks(input, spec, bytes),
                    _ => false,
                },
                _ => true,
            },
        }
    }

    /// Whether the naive prediction `out` itself breaks an invariant of
    /// DEC-013 part 7, judged by the harness's own checks (the library's
    /// self-check was then right to refuse).
    fn predicted_output_breaks(input: &[u8], spec: StripSpec, out: &[u8]) -> bool {
        let after = audit(out);
        if after.findings().iter().any(|f| spec.contains(f.category)) {
            return true;
        }
        let before = audit(input);
        if kept_keys(&before, spec, all_exif_removed(spec)) != kept_keys(&after, spec, false) {
            return true;
        }
        match (Jpeg::parse(input), Jpeg::parse(out)) {
            (Ok(a), Ok(b)) => {
                if a.scan_region() != b.scan_region() {
                    return true;
                }
                // New kinds of walk issue.
                let kinds = |bytes: &[u8]| -> Vec<String> {
                    exif_core::jpeg::read(bytes).map_or(Vec::new(), |m| {
                        m.jpeg_issues()
                            .iter()
                            .map(|i| {
                                format!("{:?}", i.error())
                                    .split(|c: char| !c.is_alphanumeric())
                                    .next()
                                    .unwrap_or("")
                                    .to_string()
                            })
                            .collect()
                    })
                };
                let old = kinds(input);
                if kinds(out).iter().any(|k| !old.contains(k)) {
                    return true;
                }
            }
            _ => return true,
        }
        // Idempotence of the prediction: predicting again gives nothing to do.
        match predict(out, spec) {
            Pred::Ok {
                removals, patches, ..
            } => !removals.is_empty() || !patches.is_empty(),
            _ => true,
        }
    }

    /// The guards that fail for `facts` (empty: the library's answer holds).
    pub fn failures(input: &[u8], spec: StripSpec, f: &Facts, pred: &Pred) -> Vec<Guard> {
        GUARDS
            .iter()
            .copied()
            .filter(|g| !holds(*g, input, spec, f, pred))
            .collect()
    }

    /// The harness body for one candidate (policy byte included).
    pub fn check(data: &[u8]) {
        let (policy, jpeg) = decode(data);
        check_input(jpeg, policy);
    }

    pub fn check_input(jpeg: &[u8], policy: StripPolicy) {
        let spec = policy.spec();
        let facts = facts(jpeg, policy);
        let pred = predict(jpeg, spec);
        let bad = failures(jpeg, spec, &facts, &pred);
        assert!(
            bad.is_empty(),
            "guards {bad:?} failed\npolicy {} {:?}\nfacts {:?}\nprediction {:?}",
            policy.name(),
            spec_cats(spec),
            facts_summary(&facts),
            pred_summary(&pred)
        );
    }

    fn facts_summary(f: &Facts) -> String {
        match &f.result {
            Ok(ok) => format!(
                "Ok len {} removals {:?} patches {:?}",
                ok.bytes.len(),
                ok.removals,
                ok.patches
            ),
            Err(e) => format!("Err {e:?}"),
        }
    }

    fn pred_summary(p: &Pred) -> String {
        match p {
            Pred::Ok {
                bytes,
                removals,
                patches,
            } => format!(
                "Ok len {} removals {removals:?} patches {patches:?}",
                bytes.len()
            ),
            other => format!("{other:?}"),
        }
    }
}
