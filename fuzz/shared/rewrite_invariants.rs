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
// calls into `exif_core::write` except `strip` itself and the public result
// and error types it flattens. It predicts the exact output, or the exact
// refusal, for the policy, and the library's answer is compared with it.
//
// What the harness DOES call in the library, exactly:
//   * `audit(input)` inside `predict`, for the Exif blocks only (their
//     findings, `failed_blocks` and `exif_issues`; an Exif block is the TIFF
//     engine's business, covered by the `audit` target);
//   * `audit(output)` and `audit(input)` in `facts` and in
//     `predicted_output_breaks` (the checks on the library's output and on the
//     independent prediction);
//   * `Jpeg::parse` (the scan region of input and output) in `facts` and in
//     `predicted_output_breaks`;
//   * `jpeg::read(..).jpeg_issues()` inside `predicted_output_breaks` (new
//     issue kinds);
//   * the constants `MAX_JPEG_ISSUES`, `MAX_JPEG_SEGMENTS`, `MAX_MPF_IMAGES`
//     (the walk's and the index's caps), and `Category::ALL`, `StripPolicy`,
//     `StripSpec`.
// The segment walk, the MPF index read, the typing of segments, the targets,
// the overlap and MpfNotIndexed rules and the field arithmetic are the
// harness's own.
//
// Input layout: byte 0 selects the policy (0..=4 the presets in
// `StripPolicy::PRESETS` order, 5 `Custom(none)`, 6 `Custom` from the 16-bit
// little-endian mask in bytes 1 and 2, bit i meaning `Category::ALL[i]`, 7 and
// above `All`), the rest is the JPEG candidate (after bytes 1 and 2 for 6).
//
// Every guard is its own function over `Facts` (the library's answer, flattened)
// and the prediction, so a planted bug in the flattened answer turns exactly its
// guard red (see `tests/fuzz_layout.rs`). Every rule that is an iff is checked in
// BOTH directions. `VerifyFailed` is a harness failure EXCEPT when the
// independent prediction itself breaks an invariant of DEC-013 part 7 (judged
// by `predicted_output_breaks`: a removed category left, kept findings changed,
// the scan changed, a new issue kind, or a prediction that is not idempotent;
// the self-check was then right to refuse), or when the naive arithmetic
// underflows an MPF field (then `MpfFields` is the expected check).

mod rewrite_invariants {
    use std::ops::Range;

    use exif_core::audit::{audit, Category, FileAudit, ItemKind, RegionKind};
    use exif_core::jpeg::{Jpeg, MAX_JPEG_ISSUES, MAX_JPEG_SEGMENTS, MAX_MPF_IMAGES};
    use exif_core::policy::{StripPolicy, StripSpec};
    use exif_core::write::{Check, RemovalKind, UneditableReason, UnstrippableReason};
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
        /// Every patch (Multi-Picture fields, rewritten IFD tables, zeroed
        /// ranges) as `(output offset, length)`.
        pub patches: Vec<(usize, usize)>,
        /// The zeroed ranges as `(output offset, length)`.
        pub scrubs: Vec<(usize, usize)>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum ErrFact {
        /// Unsupported or NotJpeg (the harness does not sniff).
        NotAJpeg,
        Unstrippable(&'static str, usize),
        /// An Exif entry edit refused: the reason and the offset.
        Uneditable(&'static str, usize),
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
            StripError::Uneditable { reason, offset } => ErrFact::Uneditable(
                match reason {
                    UneditableReason::BlockNotClean => "block-not-clean",
                    UneditableReason::TooManyExifSegments => "too-many-exif-segments",
                    UneditableReason::InventoryTruncated => "inventory-truncated",
                    UneditableReason::AfterScan => "after-scan",
                    UneditableReason::SharedBytes => "shared-bytes",
                    UneditableReason::ThumbnailNotLocated => "thumbnail-not-located",
                    UneditableReason::ChainAfterThumbnail => "chain-after-thumbnail",
                    UneditableReason::PointerNotInParent => "pointer-not-in-parent",
                    UneditableReason::EmptiedIfd => "emptied-ifd",
                    UneditableReason::NestedIfd => "nested-ifd",
                    UneditableReason::PayloadUnreadable => "payload-unreadable",
                    UneditableReason::HiddenPointer => "hidden-pointer",
                    _ => "other",
                },
                *offset,
            ),
            StripError::MpfNotIndexed { offset } => ErrFact::MpfNotIndexed(*offset),
            StripError::Overlap { offset } => ErrFact::Overlap(*offset),
            StripError::VerifyFailed { check } => ErrFact::VerifyFailed(match check {
                Check::Preview => "preview",
                Check::Kept => "kept",
                Check::Scan => "scan",
                Check::Walk => "walk",
                Check::Idempotent => "idempotent",
                Check::MpfFields => "mpf-fields",
                Check::Entries => "entries",
                Check::Scrubbed => "scrubbed",
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
                scrubs: s.scrubs().iter().map(|p| (p.offset, p.len)).collect(),
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

    /// Whether a policy names a category an Exif entry can have: the nine Exif
    /// categories and the three the DEC-008 table files under `Xmp`, `Iptc` and
    /// `Icc`.
    fn exif_may_hold(spec: StripSpec) -> bool {
        EXIF9
            .iter()
            .chain([Category::Xmp, Category::Iptc, Category::Icc].iter())
            .any(|c| spec.contains(*c))
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
        /// The naive Exif reader finds a block that is not clean, or the naive
        /// entry rules refuse (the library must refuse too, as `Uneditable`).
        EntryRefused,
        MpfNotIndexed(usize),
        Overlap(usize),
        /// A primary-size or offset field would go below zero.
        MpfUnderflow,
        Ok {
            bytes: Vec<u8>,
            removals: Vec<RemFact>,
            patches: Vec<(usize, usize)>,
            /// Output ranges of the Exif segments (kept, edited in place) when
            /// the policy edits entries; empty otherwise.
            exif_out: Vec<Range<usize>>,
            /// The naive inventory the output's Exif blocks must have (the
            /// input's minus the deleted entries); `None` when no entry edit.
            inventory: Option<Vec<NKey>>,
            /// Whether the policy edits Exif entries at all.
            edits: bool,
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

    // ── the naive TIFF reader and entry editor (SPEC-021) ──────────────────
    //
    // A second implementation of the tolerant walk's cleanliness rules and of
    // the entry-deletion rules, written here from DEC-005 and DEC-013
    // (amendment B), not from `src/tiff` or `src/write/exif.rs`. It answers:
    // is this Exif blob clean (the walk would record no issue), what would the
    // edit be (patches by blob offset, in the library's segmentation), and
    // which entries does the output keep.

    #[derive(Clone)]
    struct NEnt {
        tag: u16,
        ty: u16,
        count: u32,
        /// Blob offset of the entry's 12 bytes.
        at: usize,
    }

    #[derive(Clone)]
    struct NIfd {
        off: usize,
        role: &'static str,
        parent: Option<usize>,
        ents: Vec<NEnt>,
        next: u32,
        table: Range<usize>,
    }

    struct NTiff {
        big: bool,
        ifds: Vec<NIfd>,
    }

    fn type_size(t: u16) -> Option<u64> {
        match t {
            1 | 2 | 6 | 7 => Some(1),
            3 | 8 => Some(2),
            4 | 9 | 11 | 13 => Some(4),
            5 | 10 | 12 => Some(8),
            _ => None,
        }
    }

    fn nu16(b: &[u8], at: usize, big: bool) -> Option<u16> {
        let s: [u8; 2] = b.get(at..at.checked_add(2)?)?.try_into().ok()?;
        Some(if big {
            u16::from_be_bytes(s)
        } else {
            u16::from_le_bytes(s)
        })
    }

    fn nu32(b: &[u8], at: usize, big: bool) -> Option<u32> {
        let s: [u8; 4] = b.get(at..at.checked_add(4)?)?.try_into().ok()?;
        Some(if big {
            u32::from_be_bytes(s)
        } else {
            u32::from_le_bytes(s)
        })
    }

    fn put16(v: u16, big: bool) -> [u8; 2] {
        if big {
            v.to_be_bytes()
        } else {
            v.to_le_bytes()
        }
    }

    /// The bytes of an entry's value: inline (at most 4) or at its offset;
    /// `None` when the type is unknown or the range is not in the blob.
    fn payload_bytes<'a>(blob: &'a [u8], big: bool, e: &NEnt) -> Option<&'a [u8]> {
        let len = usize::try_from(type_size(e.ty)?.checked_mul(u64::from(e.count))?).ok()?;
        let start = if len <= 4 {
            e.at.checked_add(8)?
        } else {
            usize::try_from(nu32(blob, e.at.checked_add(8)?, big)?).ok()?
        };
        blob.get(start..start.checked_add(len)?)
    }

    /// The blob range of an entry's OUT-OF-LINE value, if it has one.
    fn out_of_line(blob: &[u8], big: bool, e: &NEnt) -> Option<Range<usize>> {
        let len = usize::try_from(type_size(e.ty)?.checked_mul(u64::from(e.count))?).ok()?;
        if len <= 4 {
            return None;
        }
        let start = usize::try_from(nu32(blob, e.at.checked_add(8)?, big)?).ok()?;
        let end = start.checked_add(len)?;
        blob.get(start..end)?;
        Some(start..end)
    }

    /// DEC-005's table: whether an IFD of `role` follows pointer tag `tag`.
    fn n_followed(role: &str, tag: u16) -> bool {
        match tag {
            0x8769 | 0x8825 => matches!(role, "ifd0" | "ifd1" | "chain" | "sub"),
            0xA005 => role == "exif",
            _ => true,
        }
    }

    const N_POINTERS: [(u16, &str); 4] = [
        (0x014A, "sub"),
        (0x8769, "exif"),
        (0x8825, "gps"),
        (0xA005, "interop"),
    ];

    struct TWalk<'a> {
        blob: &'a [u8],
        big: bool,
        ifds: Vec<NIfd>,
        visited: Vec<u32>,
        attempts: usize,
        entries: usize,
        bad: bool,
    }

    impl TWalk<'_> {
        fn read_ifd(&self, off: u32, left: usize) -> Option<(Vec<NEnt>, u32, Range<usize>)> {
            let at = usize::try_from(off).ok()?;
            let count = usize::from(nu16(self.blob, at, self.big)?);
            let first = at.checked_add(2)?;
            let end = first.checked_add(count.checked_mul(12)?)?;
            self.blob.get(first..end)?;
            if count > left {
                return None;
            }
            let next = nu32(self.blob, end, self.big)?;
            let ents = (0..count)
                .map(|i| {
                    let e = first + 12 * i;
                    Some(NEnt {
                        tag: nu16(self.blob, e, self.big)?,
                        ty: nu16(self.blob, e + 2, self.big)?,
                        count: nu32(self.blob, e + 4, self.big)?,
                        at: e,
                    })
                })
                .collect::<Option<Vec<_>>>()?;
            Some((ents, next, at..end + 4))
        }

        /// Record the IFD at `off`; `None` (and `bad`) when it cannot be.
        fn follow(
            &mut self,
            off: u32,
            depth: u32,
            parent: Option<usize>,
            role: &'static str,
        ) -> Option<usize> {
            self.attempts += 1;
            if self.attempts > 512 || depth > 8 || self.visited.contains(&off) {
                self.bad = true;
                return None;
            }
            self.visited.push(off);
            let budget = (self.blob.len() / 12).saturating_sub(self.entries);
            let Some((ents, next, table)) = self.read_ifd(off, budget) else {
                self.bad = true;
                return None;
            };
            self.entries += ents.len();
            self.ifds.push(NIfd {
                off: off as usize,
                role,
                parent,
                ents,
                next,
                table,
            });
            Some(self.ifds.len() - 1)
        }

        /// The offsets a pointer entry carries, or `None` (bad) if the walk
        /// would record an issue for it.
        fn offsets(&self, e: &NEnt, array: bool) -> Option<Vec<u32>> {
            if array {
                if e.count > 512 {
                    return None;
                }
                let size = match e.ty {
                    1 | 7 => 1usize,
                    3 => 2,
                    4 | 13 => 4,
                    _ => return None,
                };
                let bytes = payload_bytes(self.blob, self.big, e)?;
                let value = |c: &[u8]| -> u32 {
                    let mut v = 0u32;
                    if self.big {
                        for b in c {
                            v = (v << 8) | u32::from(*b);
                        }
                    } else {
                        for b in c.iter().rev() {
                            v = (v << 8) | u32::from(*b);
                        }
                    }
                    v
                };
                return Some(bytes.chunks(size).map(value).collect());
            }
            if !matches!(e.ty, 4 | 13) || e.count != 1 {
                return None;
            }
            Some(vec![nu32(self.blob, e.at + 8, self.big)?])
        }

        fn walk_from(&mut self, index: usize) {
            let mut cur = index;
            loop {
                if self.bad {
                    return;
                }
                let (role, next, parent) = {
                    let i = &self.ifds[cur];
                    (i.role, i.next, i.parent)
                };
                let depth = self.depth_of(cur);
                for (tag, child) in N_POINTERS {
                    if self.bad {
                        return;
                    }
                    if !n_followed(role, tag) {
                        continue;
                    }
                    let Some(e) = self.ifds[cur].ents.iter().find(|e| e.tag == tag).cloned() else {
                        continue;
                    };
                    let Some(offs) = self.offsets(&e, tag == 0x014A) else {
                        self.bad = true;
                        return;
                    };
                    for off in offs {
                        if self.bad {
                            return;
                        }
                        if off == 0 {
                            continue;
                        }
                        if let Some(c) = self.follow(off, depth + 1, Some(cur), child) {
                            self.walk_from(c);
                        }
                    }
                }
                let next_role = match role {
                    "ifd0" => "ifd1",
                    "ifd1" | "chain" => "chain",
                    "sub" => "sub",
                    _ => return,
                };
                if self.bad || next == 0 {
                    return;
                }
                match self.follow(next, depth, parent, next_role) {
                    Some(n) => cur = n,
                    None => return,
                }
            }
        }

        fn depth_of(&self, i: usize) -> u32 {
            // The pointer-tag nesting depth: parents up to IFD0.
            let mut d = 0u32;
            let mut at = self.ifds[i].parent;
            while let Some(p) = at {
                d += 1;
                at = self.ifds[p].parent;
            }
            d
        }
    }

    /// Walk an Exif blob; `None` when the library's walk would not be clean.
    fn tiff_read(blob: &[u8]) -> Option<NTiff> {
        let big = match blob.get(..2)? {
            b"II" => false,
            b"MM" => true,
            _ => return None,
        };
        if nu16(blob, 2, big)? != 42 {
            return None;
        }
        let ifd0 = nu32(blob, 4, big)?;
        let mut w = TWalk {
            blob,
            big,
            ifds: Vec::new(),
            visited: Vec::new(),
            attempts: 1,
            entries: 0,
            bad: false,
        };
        if ifd0 == 0 {
            return Some(NTiff { big, ifds: w.ifds });
        }
        w.visited.push(ifd0);
        let (ents, next, table) = w.read_ifd(ifd0, blob.len() / 12)?;
        w.entries = ents.len();
        w.ifds.push(NIfd {
            off: ifd0 as usize,
            role: "ifd0",
            parent: None,
            ents,
            next,
            table,
        });
        w.walk_from(0);
        (!w.bad).then_some(NTiff { big, ifds: w.ifds })
    }

    /// The naive walk of `blob` as `(role, tag, type, count)` per entry in walk
    /// order, or `None` when it is not clean (the layout test compares it with
    /// the library's walk).
    #[allow(dead_code)] // used by tests/fuzz_layout.rs, which includes this file
    pub fn naive_summary(blob: &[u8]) -> Option<Vec<(&'static str, u16, u16, u32)>> {
        let t = tiff_read(blob)?;
        Some(
            t.ifds
                .iter()
                .flat_map(|i| i.ents.iter().map(move |e| (i.role, e.tag, e.ty, e.count)))
                .collect(),
        )
    }

    /// DEC-008's tag table, typed again for the naive editor.
    fn ncat(role: &str, tag: u16, ifd1_jpeg: bool) -> Option<Category> {
        match role {
            "gps" => return Some(Category::Gps),
            "interop" => return None,
            "ifd1" if tag == 0x0201 => return Some(Category::Thumbnail),
            "ifd1" if tag == 0x0111 && !ifd1_jpeg => return Some(Category::Thumbnail),
            _ => {}
        }
        match tag {
            0x010E | 0x9286 | 0x9C9B | 0x9C9C | 0x9C9E | 0x9C9F | 0xA436 => {
                Some(Category::FreeText)
            }
            0x010F | 0x0110 | 0xA433 | 0xA434 => Some(Category::CameraModel),
            0x0131 | 0xA439..=0xA43C => Some(Category::Software),
            0x0132 | 0x9003 | 0x9004 | 0x9010 | 0x9011 | 0x9012 | 0x9290 | 0x9291 | 0x9292 => {
                Some(Category::Timestamps)
            }
            0x013B | 0x8298 | 0xA430 | 0x9C9D | 0xA437 | 0xA438 => Some(Category::Authorship),
            0x927C => Some(Category::MakerNote),
            0xA420 | 0xA431 | 0xA435 | 0xC62F => Some(Category::DeviceId),
            0x02BC => Some(Category::Xmp),
            0x83BB | 0x8649 => Some(Category::Iptc),
            0x8773 => Some(Category::Icc),
            _ => None,
        }
    }

    /// One edit of a blob: patches by blob offset (tables, then merged zero
    /// ranges, sorted), and the entries it deletes as `(ifd index, entry
    /// index)`, whole removed IFDs included.
    struct NEdit {
        patches: Vec<(usize, Vec<u8>)>,
        deleted: Vec<(usize, usize)>,
    }

    fn overlaps(a: &Range<usize>, b: &Range<usize>) -> bool {
        a.start < b.end && b.start < a.end
    }

    /// The edit for `spec`, or `None` where the rules refuse.
    fn n_edit(blob: &[u8], t: &NTiff, spec: StripSpec) -> Option<NEdit> {
        let big = t.big;
        let ifds = &t.ifds;
        // Pointers the walk did not follow.
        for ifd in ifds {
            for (tag, _) in N_POINTERS {
                let n = ifd.ents.iter().filter(|e| e.tag == tag).count();
                if (n_followed(ifd.role, tag) && n > 1) || (!n_followed(ifd.role, tag) && n >= 1) {
                    return None;
                }
            }
        }
        // Tables that share bytes.
        let mut tables: Vec<&Range<usize>> = ifds.iter().map(|i| &i.table).collect();
        tables.sort_by_key(|r| (r.start, r.end));
        if tables.windows(2).any(|w| overlaps(w[0], w[1])) {
            return None;
        }
        let ifd1_jpeg = ifds
            .iter()
            .any(|i| i.role == "ifd1" && i.ents.iter().any(|e| e.tag == 0x0201));
        // Which IFDs go whole.
        let mut removed: Vec<usize> = Vec::new();
        if spec.contains(Category::Gps) {
            removed.extend((0..ifds.len()).filter(|&i| ifds[i].role == "gps"));
        }
        // The thumbnail's bytes, when IFD1 has them as the audit locates them.
        let ifd1 = ifds.iter().position(|i| i.role == "ifd1");
        let thumb: Option<Range<usize>> = ifd1.and_then(|k| {
            let scalar = |tag: u16| -> Option<u32> {
                let e = ifds[k].ents.iter().find(|e| e.tag == tag)?;
                if e.count != 1 || !matches!(e.ty, 3 | 4) {
                    return None;
                }
                if e.ty == 3 {
                    nu16(blob, e.at + 8, big).map(u32::from)
                } else {
                    nu32(blob, e.at + 8, big)
                }
            };
            let (o, l) = (scalar(0x0201)?, scalar(0x0202)?);
            let start = usize::try_from(o).ok()?;
            let end = start.checked_add(usize::try_from(l).ok()?)?;
            blob.get(start..end)?;
            Some(start..end)
        });
        let mut unlink: Option<usize> = None;
        if spec.contains(Category::Thumbnail) {
            if let Some(k) = ifd1 {
                thumb.as_ref()?;
                if ifds[k].next != 0 {
                    return None;
                }
                let z = ifds
                    .iter()
                    .position(|i| i.role == "ifd0" && i.next as usize == ifds[k].off)?;
                unlink = Some(z);
                removed.push(k);
            }
        }
        if removed
            .iter()
            .any(|&r| ifds.iter().any(|i| i.parent == Some(r)))
        {
            return None;
        }
        // Which entries go.
        let mut gone: Vec<(usize, usize)> = Vec::new();
        for (g, ifd) in ifds.iter().enumerate() {
            if removed.contains(&g) {
                continue;
            }
            for (n, e) in ifd.ents.iter().enumerate() {
                if ncat(ifd.role, e.tag, ifd1_jpeg).is_some_and(|c| spec.contains(c)) {
                    gone.push((g, n));
                }
            }
        }
        for &r in removed.iter().filter(|&&r| ifds[r].role == "gps") {
            let p = ifds[r].parent?;
            let n = ifds[p].ents.iter().position(|e| {
                e.tag == 0x8825 && nu32(blob, e.at + 8, big) == Some(ifds[r].off as u32)
            })?;
            gone.push((p, n));
        }
        gone.sort_unstable();
        gone.dedup();
        // Everything deleted, for the sizes of the values and the records.
        let mut all_deleted = gone.clone();
        for &r in &removed {
            all_deleted.extend((0..ifds[r].ents.len()).map(|n| (r, n)));
        }
        all_deleted.sort_unstable();
        if all_deleted
            .iter()
            .any(|&(g, n)| type_size(ifds[g].ents[n].ty).is_none())
        {
            return None;
        }
        // Table rewrites.
        let mut rewrite: Vec<usize> = gone.iter().map(|&(g, _)| g).collect();
        rewrite.extend(unlink);
        rewrite.sort_unstable();
        rewrite.dedup();
        let mut table_patches: Vec<(Range<usize>, Vec<u8>)> = Vec::new();
        for g in rewrite {
            let ifd = &ifds[g];
            let cut = |n: usize| gone.contains(&(g, n));
            let kept: Vec<usize> = (0..ifd.ents.len()).filter(|&n| !cut(n)).collect();
            let mut bytes = put16(u16::try_from(kept.len()).ok()?, big).to_vec();
            for n in &kept {
                let at = ifd.ents[*n].at;
                bytes.extend_from_slice(blob.get(at..at + 12)?);
            }
            let next_at = ifd.table.end - 4;
            if unlink == Some(g) {
                bytes.extend_from_slice(&[0; 4]);
            } else {
                bytes.extend_from_slice(blob.get(next_at..next_at + 4)?);
            }
            bytes.resize(ifd.table.len(), 0);
            table_patches.push((ifd.table.clone(), bytes));
        }
        // Scrubs.
        let mut scrubs: Vec<Range<usize>> = Vec::new();
        for &(g, n) in &all_deleted {
            scrubs.extend(out_of_line(blob, big, &ifds[g].ents[n]));
        }
        for &r in &removed {
            scrubs.push(ifds[r].table.clone());
        }
        if unlink.is_some() {
            scrubs.extend(thumb.clone());
        }
        scrubs.retain(|r| r.start < r.end);
        scrubs.sort_by_key(|r| (r.start, r.end));
        let mut merged: Vec<Range<usize>> = Vec::new();
        for r in scrubs {
            match merged.last_mut() {
                Some(last) if r.start < last.end => last.end = last.end.max(r.end),
                _ => merged.push(r),
            }
        }
        // The bytes that must stay.
        let mut keep: Vec<Range<usize>> = Vec::new();
        keep.push(0..8);
        let mut kept_tables: Vec<Range<usize>> = Vec::new();
        for (g, ifd) in ifds.iter().enumerate() {
            if removed.contains(&g) {
                continue;
            }
            kept_tables.push(ifd.table.clone());
            for (n, e) in ifd.ents.iter().enumerate() {
                if !gone.contains(&(g, n)) {
                    keep.extend(out_of_line(blob, big, e));
                }
            }
        }
        if unlink.is_none() {
            keep.extend(thumb.clone());
        }
        for m in &merged {
            if keep
                .iter()
                .chain(kept_tables.iter())
                .any(|k| overlaps(m, k))
            {
                return None;
            }
        }
        for (t, _) in &table_patches {
            if keep.iter().any(|k| overlaps(t, k)) {
                return None;
            }
        }
        let mut patches: Vec<(usize, Vec<u8>)> = table_patches
            .into_iter()
            .map(|(r, b)| (r.start, b))
            .collect();
        patches.extend(
            merged
                .into_iter()
                .map(|r| (r.start, vec![0u8; r.end - r.start])),
        );
        patches.sort_by_key(|p| p.0);
        Some(NEdit {
            patches,
            deleted: all_deleted,
        })
    }

    /// One tag of a naive inventory: block, IFD role, tag, type, count and the
    /// value bytes.
    pub type NKey = (usize, &'static str, u16, u16, u32, Option<Vec<u8>>);

    /// The inventory of the Exif blocks of `d` (the naive walk of the JPEG, the
    /// naive walk of each blob), minus the entries `drop(block, ifd, entry)`
    /// names; `None` when a block is not clean.
    fn n_inventory(d: &[u8], drop: &dyn Fn(usize, usize, usize) -> bool) -> Option<Vec<NKey>> {
        let w = walk(d);
        let mut keys = Vec::new();
        let mut block = 0usize;
        for s in &w.segs {
            let p = &d[s.off + 4..s.end];
            if !(s.marker == 0xE1 && p.len() >= 6 && p.starts_with(b"Exif\0")) {
                continue;
            }
            let blob = &d[s.off + 10..s.end];
            let t = tiff_read(blob)?;
            for (g, ifd) in t.ifds.iter().enumerate() {
                for (n, e) in ifd.ents.iter().enumerate() {
                    if drop(block, g, n) {
                        continue;
                    }
                    keys.push((
                        block,
                        ifd.role,
                        e.tag,
                        e.ty,
                        e.count,
                        payload_bytes(blob, t.big, e).map(<[u8]>::to_vec),
                    ));
                }
            }
            block += 1;
        }
        Some(keys)
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

        // Exif: removed whole (every Exif category in the policy, the category
        // recorded from the audit's findings), edited entry by entry (the naive
        // reader and editor, SPEC-021) or left alone.
        let exif_idx: Vec<usize> = (0..w.segs.len()).filter(|&i| typed[i] == T::Exif).collect();
        let mut edits = false;
        let mut exif_patches: Vec<(usize, Vec<u8>)> = Vec::new();
        let mut inventory: Option<Vec<NKey>> = None;
        if !exif_idx.is_empty() {
            let a = audit(d);
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
            } else if exif_may_hold(spec) {
                // Partial policy: entries are deleted in place (SPEC-021). The
                // naive reader and editor decide, block by block, with the
                // preconditions of rule 3 first.
                if exif_idx.len() > 16 {
                    return Pred::EntryRefused;
                }
                if exif_idx
                    .iter()
                    .any(|&i| w.scan_start.is_some_and(|s| w.segs[i].off > s))
                {
                    return Pred::EntryRefused;
                }
                edits = true;
                let mut dropped: Vec<(usize, usize, usize)> = Vec::new();
                for (n, &i) in exif_idx.iter().enumerate() {
                    let s = &w.segs[i];
                    let blob = &d[s.off + 10..s.end];
                    let Some(t) = tiff_read(blob) else {
                        return Pred::EntryRefused;
                    };
                    let Some(e) = n_edit(blob, &t, spec) else {
                        return Pred::EntryRefused;
                    };
                    for (at, bytes) in e.patches {
                        exif_patches.push((s.off + 10 + at, bytes));
                    }
                    dropped.extend(e.deleted.iter().map(|&(g, k)| (n, g, k)));
                }
                // The inventory the output must have: the input's, minus these.
                inventory = n_inventory(d, &|b, g, k| dropped.contains(&(b, g, k)));
                if inventory.is_none() {
                    return Pred::EntryRefused;
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
        for (at, bytes) in &exif_patches {
            data[*at..*at + bytes.len()].copy_from_slice(bytes);
        }
        let bytes: Vec<u8> = data
            .iter()
            .zip(&removed)
            .filter(|(_, r)| !**r)
            .map(|(b, _)| *b)
            .collect();
        // Every patch as `(output offset, length)`, in output order.
        let mut records: Vec<(usize, usize)> = patches
            .iter()
            .map(|(at, _)| (*at - removed_before(*at), 4))
            .chain(
                exif_patches
                    .iter()
                    .map(|(at, b)| (*at - removed_before(*at), b.len())),
            )
            .collect();
        records.sort_unstable();
        let exif_out: Vec<Range<usize>> = if edits {
            exif_idx
                .iter()
                .map(|&i| {
                    let s = &w.segs[i];
                    (s.off - removed_before(s.off))..(s.end - removed_before(s.off))
                })
                .collect()
        } else {
            Vec::new()
        };
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
            patches: records,
            exif_out,
            inventory,
            edits,
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
        /// (SPEC-021) The Exif segments of an `Ok` output equal the naive
        /// editor's prediction byte for byte.
        EntryPrediction,
        /// (SPEC-021) `Ok` where the naive reader finds an Exif block that is
        /// not clean (or the naive rules refuse) is a failure; a refusal where
        /// the naive editor accepts is fine (fail closed).
        LibraryOkNaiveUnclean,
        /// (SPEC-021) Every zeroed range the library records reads zero.
        ScrubbedZero,
        /// (SPEC-021) The naive inventory of the output's Exif blocks is the
        /// input's minus the entries the naive rules delete, byte for byte.
        KeptEntriesIntact,
        /// (SPEC-021) No segment removal means the length is the input's.
        LengthUnchanged,
    }

    pub const GUARDS: [Guard; 15] = [
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
        Guard::EntryPrediction,
        Guard::LibraryOkNaiveUnclean,
        Guard::ScrubbedZero,
        Guard::KeptEntriesIntact,
        Guard::LengthUnchanged,
    ];

    /// The output equals the prediction outside the Exif segments the naive
    /// editor rewrote (those are `EntryPrediction`'s).
    fn out_matches_pred(ok: &OkFacts, pred: &Pred) -> bool {
        match pred {
            Pred::Ok {
                bytes, exif_out, ..
            } => {
                bytes.len() == ok.bytes.len()
                    && bytes
                        .iter()
                        .zip(&ok.bytes)
                        .enumerate()
                        .all(|(i, (a, b))| a == b || exif_out.iter().any(|r| r.contains(&i)))
            }
            _ => false,
        }
    }

    /// Whether the guard `g` holds. Each guard stands alone: it reads only its
    /// own part of the facts and the prediction.
    pub fn holds(g: Guard, input: &[u8], spec: StripSpec, f: &Facts, pred: &Pred) -> bool {
        match g {
            Guard::Prediction => match (&f.result, pred) {
                (Ok(ok), Pred::Ok { .. }) => out_matches_pred(ok, pred),
                // An `Ok` the naive Exif rules refuse is `LibraryOkNaiveUnclean`'s.
                (Ok(_), Pred::EntryRefused) => true,
                (Ok(_), _) => false,
                // A refusal is judged by `ErrClass`.
                (Err(_), _) => true,
            },
            Guard::EntryPrediction => match (&f.result, pred) {
                (
                    Ok(ok),
                    Pred::Ok {
                        bytes,
                        exif_out,
                        edits: true,
                        ..
                    },
                ) => exif_out.iter().all(|r| {
                    matches!((ok.bytes.get(r.clone()), bytes.get(r.clone())),
                        (Some(a), Some(b)) if a == b)
                }),
                _ => true,
            },
            Guard::LibraryOkNaiveUnclean => {
                !matches!((&f.result, pred), (Ok(_), Pred::EntryRefused))
            }
            Guard::ScrubbedZero => match &f.result {
                Ok(ok) => ok.scrubs.iter().all(|&(at, len)| {
                    at.checked_add(len)
                        .and_then(|end| ok.bytes.get(at..end))
                        .is_some_and(|s| s.iter().all(|b| *b == 0))
                }),
                Err(_) => true,
            },
            Guard::KeptEntriesIntact => match (&f.result, pred) {
                (
                    Ok(ok),
                    Pred::Ok {
                        inventory: Some(inv),
                        ..
                    },
                ) => n_inventory(&ok.bytes, &|_, _, _| false).as_ref() == Some(inv),
                _ => true,
            },
            Guard::LengthUnchanged => match &f.result {
                Ok(ok) => !ok.removals.is_empty() || ok.bytes.len() == input.len(),
                Err(_) => true,
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
                // The library refuses the Exif edit: always fine when the naive
                // rules refuse too, and fail-closed when they accept; but the
                // refusal comes after the walk (`Unstrippable`).
                (Err(ErrFact::Uneditable(..)), p) => {
                    !matches!(p, Pred::NotAJpeg | Pred::Unstrippable(..))
                }
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
                ..
            } => format!(
                "Ok len {} removals {removals:?} patches {patches:?}",
                bytes.len()
            ),
            other => format!("{other:?}"),
        }
    }
}
