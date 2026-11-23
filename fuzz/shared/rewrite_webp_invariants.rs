// The SPEC-038 invariants of `exif_core::strip` on a WebP (DEC-022), shared by
// the fuzz target (`fuzz/fuzz_targets/rewrite_webp.rs`) and the stable-toolchain
// layout test (`tests/fuzz_layout.rs`) with `include!`, so the two cannot
// drift. This file is not a crate root; it is text included at module level,
// and it defines one module, `rewrite_webp_invariants`.
//
// The harness holds an INDEPENDENT NAIVE STRIPPER, independent in code and in
// facts:
//
// * CODE. It has its own RIFF chunk walk (a second implementation of the
//   DEC-019 framing: the pad byte, the quiet stop on fewer than 8 bytes, the
//   chunk cap, the issue cap and what counts as an issue), its own Exif mode
//   rule (DEC-013 amendment B), its own size rule (DEC-022 clause 6) and its
//   own flag rule (clause 7). From those it predicts the exact output or the
//   exact refusal. It never reads `src/write/webp.rs`; a test in
//   `tests/fuzz_layout.rs` asserts that no line of code here names the writer
//   module or its helpers.
// * FACTS. The carrier table below is typed from the MEASUREMENT of ExifTool
//   13.55 recorded in `tests/support/webp_carriers.rs` and DEC-022 clause 4
//   (which chunk types print tags, which are not metadata, that no type folds
//   case, the `JUNK` text rule), not from `src/webp` or `src/write`. A test in
//   `tests/fuzz_layout.rs` compares this table with the measured data row by
//   row, so a wrong measurement shows as a diff against the recorded facts and
//   not as two copies of one mistake agreeing.
//
// What the harness DOES call in the library, exactly:
//   * `strip` (the removal and patch records are flattened through the public
//     result type);
//   * `audit(input)` and `audit(output)` in `facts`, for the findings'
//     categories and counts (the TIFF engine inside an `EXIF` chunk is the
//     business of the `audit` and `rewrite_jpeg` targets);
//   * `sniff` (to leave non-WebP candidates to the other targets), the caps
//     `MAX_WEBP_CHUNKS` and `MAX_WEBP_ISSUES`, and `Category::ALL`,
//     `StripPolicy`, `StripSpec`.
//
// What it does NOT model: the bytes of an edited `EXIF` chunk (the entry editor
// is covered by `rewrite_jpeg`'s naive editor); there the harness checks that
// the chunk keeps its header and length, and that every patch the library
// records lies inside such a chunk, or is the size or the flag patch. A refusal
// `Uneditable` is accepted only where a partial policy meets a live `EXIF`
// chunk (and is predicted exactly when there are more than 16 of them).
//
// Input layout: byte 0 selects the policy (0..=4 the presets in
// `StripPolicy::PRESETS` order, 5 `Custom(none)`, 6 `Custom` from the 16-bit
// little-endian mask in bytes 1 and 2, bit i meaning `Category::ALL[i]`, 7 `All`,
// 8 `Custom(OtherSegment)`, 9 `Custom(Icc)`, 10 `Custom(Xmp)`, 11 `Custom(Gps)`,
// 12 `Custom(Jumbf)`, 13 `Custom(FreeText)`, 14 `Custom(Timestamps)`, 15
// `Custom(Software)`, 16 and above `All`), the rest is the WebP candidate
// (after bytes 1 and 2 for 6).
//
// Every guard is its own function over `Facts` (the library's answer,
// flattened) and the prediction, so a planted bug in the flattened answer turns
// exactly its guard red (see `tests/fuzz_layout.rs`). `VerifyFailed` is always
// a harness failure: on a WebP the plan and the self-check must agree.

mod rewrite_webp_invariants {
    use std::ops::Range;

    use exif_core::audit::{audit, sniff, Category, FileAudit, Sniffed};
    use exif_core::policy::{StripPolicy, StripSpec};
    use exif_core::webp::{MAX_WEBP_CHUNKS, MAX_WEBP_ISSUES};
    use exif_core::write::{PatchKind, RemovalKind, UneditableReason, UnstrippableReason};
    use exif_core::{strip, StripError};

    /// The nine Exif-set categories (DEC-013 amendment B).
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

    /// `VP8X` flag bits the strip may clear: ICC, Exif, XMP.
    const BIT_ICC: u8 = 0x20;
    const BIT_EXIF: u8 = 0x08;
    const BIT_XMP: u8 = 0x04;

    // ── the measured carrier table (facts) ─────────────────────────────────

    /// What a chunk type is to the strip.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Class {
        /// ExifTool prints tags the audit cannot read: removed with any tag
        /// category, `Xmp`, `Iptc` and `OtherSegment`.
        Carrier,
        /// An opaque binary tag, metadata for no preset: removed with
        /// `OtherSegment` only.
        Other,
        /// A JUMBF manifest: removed with `Jumbf`.
        Jumbf,
    }

    /// Typed from the measurement (ExifTool 13.55, `-a -u -G1 -H -s`, one chunk
    /// per file): every type here printed at least one tag from its chunk, and
    /// the type is matched case-sensitively (all 16 spellings of each were
    /// measured; none other than the exact code decoded). `JUNK` is not here:
    /// it is a carrier only when its data is text (see [`junk_is_text`]).
    pub const TABLE: [([u8; 4], Class); 31] = [
        (*b"LIST", Class::Carrier),
        (*b"IDIT", Class::Carrier),
        (*b"JUNQ", Class::Carrier),
        (*b"SEAL", Class::Carrier),
        (*b"_PMX", Class::Carrier),
        (*b"aXML", Class::Carrier),
        (*b"iXML", Class::Carrier),
        (*b"afsp", Class::Carrier),
        (*b"bext", Class::Carrier),
        (*b"guan", Class::Carrier),
        (*b"list", Class::Carrier),
        (*b"labl", Class::Carrier),
        (*b"note", Class::Carrier),
        (*b"ltxt", Class::Carrier),
        (*b"olym", Class::Carrier),
        (*b"ID3 ", Class::Carrier),
        (*b"id3 ", Class::Carrier),
        (*b"gps0", Class::Carrier),
        (*b"SGLT", Class::Carrier),
        (*b"SLLT", Class::Carrier),
        (*b"C2PA", Class::Jumbf),
        (*b"CSET", Class::Other),
        (*b"acid", Class::Other),
        (*b"ds64", Class::Other),
        (*b"fact", Class::Other),
        (*b"fmt ", Class::Other),
        (*b"inst", Class::Other),
        (*b"smpl", Class::Other),
        (*b"cue ", Class::Other),
        (*b"plst", Class::Other),
        (*b"gsen", Class::Other),
    ];

    /// Measured: ExifTool printed `TextJunk` for a `JUNK` chunk whose data is
    /// printable ASCII (0x20 to 0x7E) followed by zero or more NULs, and
    /// nothing for empty data, all zeros, a NUL before or inside the text, or
    /// any control or high byte.
    pub fn junk_is_text(data: &[u8]) -> bool {
        let text_len = data.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
        text_len > 0 && data[..text_len].iter().all(|&b| (0x20..=0x7E).contains(&b))
    }

    /// The class of a chunk by its type (and, for `JUNK`, its data); `None`
    /// for a chunk that is not metadata (and is never removed). The chunk
    /// types the strip decides by entries or category (`EXIF`, `XMP `,
    /// `XMP\0`, `ICCP`) are handled before this is asked.
    pub fn class_of(code: &[u8; 4], data: &[u8]) -> Option<Class> {
        if code == b"JUNK" {
            return junk_is_text(data).then_some(Class::Carrier);
        }
        TABLE.iter().find(|(c, _)| c == code).map(|(_, k)| *k)
    }

    // ── the policy byte ────────────────────────────────────────────────────

    fn only(c: Category) -> StripPolicy {
        StripPolicy::Custom(StripSpec::none().include(c))
    }

    /// Decode the leading bytes into a policy and the WebP candidate.
    pub fn decode(data: &[u8]) -> (StripPolicy, &[u8]) {
        let Some(&b0) = data.first() else {
            return (StripPolicy::All, data);
        };
        let rest = &data[1..];
        match b0 {
            0..=4 => (StripPolicy::PRESETS[usize::from(b0)], rest),
            5 => (StripPolicy::Custom(StripSpec::none()), rest),
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
            7 => (StripPolicy::All, rest),
            8 => (only(Category::OtherSegment), rest),
            9 => (only(Category::Icc), rest),
            10 => (only(Category::Xmp), rest),
            11 => (only(Category::Gps), rest),
            12 => (only(Category::Jumbf), rest),
            13 => (only(Category::FreeText), rest),
            14 => (only(Category::Timestamps), rest),
            15 => (only(Category::Software), rest),
            _ => (StripPolicy::All, rest),
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
        /// In input coordinates, in the order the library lists them.
        pub removals: Vec<RemFact>,
        /// `(offset, length, kind)` of every patch, in output coordinates.
        pub patches: Vec<(usize, usize, &'static str)>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum ErrFact {
        /// Unsupported or NotJpeg (the harness only feeds WebPs).
        NotAWebp,
        Unstrippable(&'static str, usize),
        Uneditable(&'static str, usize),
        Overlap(usize),
        VerifyFailed(&'static str),
    }

    /// What the harness learned about an `Ok` output by auditing it.
    #[derive(Debug, Clone, PartialEq)]
    pub struct OutFacts {
        /// Categories of the findings of `audit(output)`, sorted.
        pub categories: Vec<Category>,
        /// Finding counts per kept category (not in the policy, not
        /// `OtherSegment`) of the input and the output; `None` when an
        /// inventory was truncated.
        pub kept_in: Option<Vec<(Category, usize)>>,
        pub kept_out: Option<Vec<(Category, usize)>>,
        /// `strip(output, policy)` is `Ok`, returns the output and records
        /// nothing.
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
            RemovalKind::Xmp => "xmp",
            RemovalKind::Icc => "icc",
            RemovalKind::Jumbf => "jumbf",
            RemovalKind::Carrier => "carrier",
            RemovalKind::OtherChunk => "other-chunk",
            _ => "other",
        }
    }

    fn patch_name(k: PatchKind) -> &'static str {
        match k {
            PatchKind::RiffSize => "riff-size",
            PatchKind::Vp8xFlags => "vp8x-flags",
            _ => "other",
        }
    }

    fn flatten_err(e: &StripError) -> ErrFact {
        match e {
            StripError::Unsupported { .. } | StripError::NotJpeg => ErrFact::NotAWebp,
            StripError::Unstrippable { reason, offset } => ErrFact::Unstrippable(
                match reason {
                    UnstrippableReason::TooManySegments => "too-many-segments",
                    UnstrippableReason::TooManyIssues => "too-many-issues",
                    UnstrippableReason::ChunkPastEnd => "chunk-past-end",
                    _ => "other",
                },
                *offset,
            ),
            StripError::Uneditable { reason, offset } => ErrFact::Uneditable(
                match reason {
                    UneditableReason::BlockNotClean => "block-not-clean",
                    UneditableReason::TooManyExifSegments => "too-many-exif-segments",
                    UneditableReason::InventoryTruncated => "inventory-truncated",
                    _ => "other",
                },
                *offset,
            ),
            StripError::Overlap { offset } => ErrFact::Overlap(*offset),
            StripError::MpfNotIndexed { .. } => ErrFact::VerifyFailed("mpf-not-indexed"),
            StripError::VerifyFailed { .. } => ErrFact::VerifyFailed("verify"),
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
                patches: s
                    .patches()
                    .iter()
                    .map(|p| (p.offset, p.len, patch_name(p.kind)))
                    .collect(),
            }),
            Err(e) => Err(flatten_err(e)),
        }
    }

    fn kept_counts(a: &FileAudit<'_>, spec: StripSpec) -> Option<Vec<(Category, usize)>> {
        if a.health().inventory_truncated {
            return None;
        }
        // When every Exif category goes, every `EXIF` chunk goes with the
        // entries in it, whatever their category (an `Icc` entry of an Exif IFD
        // included): they are not "kept" findings.
        let skip_exif = exif_mode(spec) == NMode::Whole;
        Some(
            Category::ALL
                .iter()
                .filter(|c| **c != Category::OtherSegment && !spec.contains(**c))
                .map(|c| {
                    let n = a
                        .findings()
                        .iter()
                        .filter(|f| f.category == *c)
                        .filter(|f| !(skip_exif && f.at.origin().block.is_some()))
                        .count();
                    (*c, n)
                })
                .collect(),
        )
    }

    fn out_facts(input: &[u8], policy: StripPolicy, out: &[u8]) -> OutFacts {
        let spec = policy.spec();
        let after = audit(out);
        let categories: Vec<Category> = Category::ALL
            .iter()
            .copied()
            .filter(|c| after.findings().iter().any(|f| f.category == *c))
            .collect();
        let idempotent = match strip(out, policy) {
            Ok(s) => s.bytes() == out && s.removals().is_empty() && s.patches().is_empty(),
            Err(_) => false,
        };
        OutFacts {
            categories,
            kept_in: kept_counts(&audit(input), spec),
            kept_out: kept_counts(&after, spec),
            idempotent,
        }
    }

    pub fn facts(input: &[u8], policy: StripPolicy) -> Facts {
        let first = strip(input, policy);
        let result = flatten(&first);
        let deterministic = flatten(&strip(input, policy)) == result;
        let out = first
            .as_ref()
            .ok()
            .map(|s| out_facts(input, policy, s.bytes()));
        Facts {
            result,
            deterministic,
            out,
        }
    }

    // ── the naive RIFF walk (DEC-019, typed again) ─────────────────────────

    fn le32(d: &[u8], at: usize) -> u32 {
        u32::from_le_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]])
    }

    #[derive(Debug, Clone)]
    struct NChunk {
        start: usize,
        /// End including the pad byte when there is one.
        end: usize,
        data_start: usize,
        data_end: usize,
        ty: [u8; 4],
    }

    impl NChunk {
        fn data<'a>(&self, d: &'a [u8]) -> &'a [u8] {
            &d[self.data_start..self.data_end]
        }
    }

    struct NWalk {
        chunks: Vec<NChunk>,
        /// The first fault the writer refuses: its reason and input offset.
        halt: Option<(&'static str, usize)>,
        /// Index into `chunks` of the first `VP8X` when it has 10 bytes or more
        /// (the one whose flags the file declares).
        vp8x: Option<usize>,
    }

    /// The issue counter and the stop flag of the walk.
    struct Count {
        issues: usize,
        halt: Option<(&'static str, usize)>,
        stopped: bool,
    }

    impl Count {
        /// An ordinary issue, or the cap's terminal one (which halts).
        fn record(&mut self, at: usize) {
            if self.stopped {
                return;
            }
            if self.issues >= MAX_WEBP_ISSUES {
                self.halt = Some(("too-many-issues", at));
                self.stopped = true;
            } else {
                self.issues += 1;
            }
        }

        /// A walk-ending fault: its own reason (`None`: a quiet stop the
        /// writer keeps as it is) unless the issue cap fires on it first.
        fn stop(&mut self, reason: Option<&'static str>, at: usize) {
            if self.stopped {
                return;
            }
            self.record(at);
            if self.halt.is_none() {
                self.halt = reason.map(|r| (r, at));
            }
            self.stopped = true;
        }
    }

    fn is_xmp(ty: &[u8; 4]) -> bool {
        ty == b"XMP " || ty == b"XMP\0"
    }

    /// Walk `d`, which must have a valid 12-byte header.
    fn walk(d: &[u8]) -> NWalk {
        let len = d.len();
        let mut c = Count {
            issues: 0,
            halt: None,
            stopped: false,
        };
        let mut chunks: Vec<NChunk> = Vec::new();
        let mut vp8x: Option<usize> = None;
        let mut seen_vp8x = false;
        // The declared size is compared, never trusted: one issue, first.
        if u64::from(le32(d, 4)) != len as u64 - 8 {
            c.record(4);
        }
        let mut p = 12usize;
        while p < len && !c.stopped {
            let rem = len - p;
            if rem < 8 {
                c.stop(None, p);
                break;
            }
            let size = u64::from(le32(d, p + 4));
            if size + 8 > rem as u64 {
                c.stop(Some("chunk-past-end"), p);
                break;
            }
            if chunks.len() >= MAX_WEBP_CHUNKS {
                c.stop(Some("too-many-segments"), p);
                break;
            }
            let size = size as usize;
            let ty = [d[p], d[p + 1], d[p + 2], d[p + 3]];
            if &ty == b"XMP\0" {
                c.record(p);
            }
            if &ty == b"VP8X" {
                if size < 10 {
                    c.record(p);
                }
                if !chunks.is_empty() {
                    c.record(p);
                }
                if !seen_vp8x {
                    seen_vp8x = true;
                    if size >= 10 {
                        vp8x = Some(chunks.len());
                    }
                }
            }
            let data_end = p + 8 + size;
            let odd = size % 2 == 1;
            let end = if odd && data_end < len {
                data_end + 1
            } else {
                data_end
            };
            if odd && data_end >= len {
                c.record(p);
            }
            if c.stopped {
                break;
            }
            chunks.push(NChunk {
                start: p,
                end,
                data_start: p + 8,
                data_end,
                ty,
            });
            p = end;
        }
        if !c.stopped {
            // The end-of-walk rules count as issues too.
            if let Some(i) = vp8x {
                let at = chunks[i].start;
                let flags = d[chunks[i].data_start];
                let has = |f: &dyn Fn(&NChunk) -> bool| chunks.iter().any(f);
                let present = [
                    has(&|k| &k.ty == b"ICCP"),
                    has(&|k| &k.ty == b"EXIF"),
                    has(&|k| is_xmp(&k.ty)),
                ];
                for (bit, here) in [BIT_ICC, BIT_EXIF, BIT_XMP].into_iter().zip(present) {
                    if (flags & bit != 0) != here {
                        c.record(at);
                    }
                }
            } else if !seen_vp8x {
                if let Some(k) = chunks
                    .iter()
                    .find(|k| &k.ty == b"ICCP" || &k.ty == b"EXIF" || is_xmp(&k.ty))
                {
                    c.record(k.start);
                }
            }
        }
        NWalk {
            chunks,
            halt: c.halt,
            vp8x,
        }
    }

    // ── the naive rules (DEC-013, DEC-022) ─────────────────────────────────

    /// DEC-022 clause 4: a carrier goes with any tag category, `Xmp` or `Iptc`,
    /// and with its own category `OtherSegment`.
    fn carriers_removed(spec: StripSpec) -> bool {
        EXIF9
            .iter()
            .chain([Category::Xmp, Category::Iptc].iter())
            .any(|c| spec.contains(*c))
            || spec.contains(Category::OtherSegment)
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum NMode {
        Whole,
        Untouched,
        Partial,
    }

    /// DEC-022 clause 3 (DEC-013 part 4 and amendment B).
    fn exif_mode(spec: StripSpec) -> NMode {
        if EXIF9.iter().all(|c| spec.contains(*c)) {
            NMode::Whole
        } else if EXIF9
            .iter()
            .chain([Category::Xmp, Category::Iptc, Category::Icc].iter())
            .any(|c| spec.contains(*c))
        {
            NMode::Partial
        } else {
            NMode::Untouched
        }
    }

    /// What the policy does to one chunk that is not an `EXIF`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Fate {
        /// Not metadata: always kept.
        NoRule,
        /// Removed, with this category and removal kind.
        Remove(Category, &'static str),
        /// A rule applies and the policy keeps the chunk.
        Keep,
    }

    fn fate(ty: &[u8; 4], data: &[u8], spec: StripSpec) -> Fate {
        let by_cat = |c: Category, kind: &'static str| {
            if spec.contains(c) {
                Fate::Remove(c, kind)
            } else {
                Fate::Keep
            }
        };
        if is_xmp(ty) {
            return by_cat(Category::Xmp, "xmp");
        }
        if ty == b"ICCP" {
            return by_cat(Category::Icc, "icc");
        }
        match class_of(ty, data) {
            None => Fate::NoRule,
            Some(Class::Jumbf) => by_cat(Category::Jumbf, "jumbf"),
            Some(Class::Other) => by_cat(Category::OtherSegment, "other-chunk"),
            Some(Class::Carrier) => {
                if carriers_removed(spec) {
                    Fate::Remove(Category::OtherSegment, "carrier")
                } else {
                    Fate::Keep
                }
            }
        }
    }

    // ── the prediction ─────────────────────────────────────────────────────

    /// One removal the naive stripper makes.
    #[derive(Debug, Clone, PartialEq)]
    pub struct NRem {
        pub start: usize,
        pub end: usize,
        pub kind: &'static str,
        /// `None`: any Exif-set category (a removed `EXIF` chunk is recorded
        /// under the first Exif finding it holds).
        pub category: Option<Category>,
    }

    /// How much of a kept chunk the library may change.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Lenient {
        /// Nothing: the chunk is byte-identical.
        No,
        /// A partial policy edits the entries of an `EXIF` chunk in place.
        Data,
        /// The `VP8X` flag byte may be cleared.
        FirstByte,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct Kept {
        pub ty: [u8; 4],
        pub data: Vec<u8>,
        pub lenient: Lenient,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum Pred {
        Unstrippable(&'static str, usize),
        /// More than 16 `EXIF` chunks under a partial policy: the reader parsed
        /// 16, an edit cannot be shown complete. Exact reason and offset.
        Uneditable(&'static str, usize),
        Ok {
            /// The input less the removed ranges, with the RIFF size and the
            /// `VP8X` flag patches applied.
            bytes: Vec<u8>,
            removals: Vec<NRem>,
            /// Output ranges of the data of the `EXIF` chunks a partial policy
            /// may edit.
            edited: Vec<Range<usize>>,
            /// A partial policy with a live `EXIF` chunk: `Uneditable` is a
            /// fair answer.
            may_refuse: bool,
            /// The declared RIFF size the output must carry.
            riff_size: u32,
            /// Whether the size is patched (something inside the declared
            /// range was removed).
            size_patched: bool,
            /// The output offset of the `VP8X` flag byte and the value it must
            /// hold, when the file has a `VP8X` of 10 bytes or more first.
            flag: Option<(usize, u8)>,
            /// Whether the flag byte is patched.
            flag_patched: bool,
            /// The chunks that stay, in order.
            kept: Vec<Kept>,
        },
    }

    pub fn predict(d: &[u8], spec: StripSpec) -> Pred {
        let w = walk(d);
        if let Some((reason, at)) = w.halt {
            return Pred::Unstrippable(reason, at);
        }
        let mode = exif_mode(spec);
        let exif_chunks: Vec<&NChunk> = w.chunks.iter().filter(|c| &c.ty == b"EXIF").collect();
        if mode == NMode::Partial {
            // The reader parses 16 `EXIF` chunks and no more.
            if let Some(c) = exif_chunks.get(16) {
                return Pred::Uneditable("too-many-exif-segments", c.start);
            }
        }
        let mut removals: Vec<NRem> = Vec::new();
        for c in &w.chunks {
            if &c.ty == b"EXIF" {
                if mode == NMode::Whole {
                    removals.push(NRem {
                        start: c.start,
                        end: c.end,
                        kind: "exif",
                        category: None,
                    });
                }
            } else if let Fate::Remove(category, kind) = fate(&c.ty, c.data(d), spec) {
                removals.push(NRem {
                    start: c.start,
                    end: c.end,
                    kind,
                    category: Some(category),
                });
            }
        }
        removals.sort_by_key(|r| (r.start, r.end));
        let removed_before = |at: usize| -> usize {
            removals
                .iter()
                .filter(|r| r.end <= at)
                .map(|r| r.end - r.start)
                .sum()
        };
        // DEC-022 clause 6: the declared size less the bytes removed from
        // inside the declared range (offset 8 to 8 plus the declared size).
        let declared = le32(d, 4);
        let window_end = (8u64 + u64::from(declared)).min(d.len() as u64) as usize;
        let inside: usize = removals
            .iter()
            .map(|r| r.end.min(window_end).saturating_sub(r.start.max(8)))
            .sum();
        let riff_size = declared - inside as u32;
        let size_patched = inside > 0;
        // DEC-022 clause 7: a flag whose chunks all go is cleared.
        let mut flag: Option<(usize, u8)> = None;
        let mut flag_patched = false;
        if let Some(i) = w.vp8x {
            let v = &w.chunks[i];
            let was = d[v.data_start];
            let mut now = was;
            for (bit, kind) in [(BIT_ICC, 0), (BIT_EXIF, 1), (BIT_XMP, 2)] {
                let of_kind = |c: &&NChunk| match kind {
                    0 => &c.ty == b"ICCP",
                    1 => &c.ty == b"EXIF",
                    _ => is_xmp(&c.ty),
                };
                let any = w.chunks.iter().filter(of_kind).count() > 0;
                let all_removed = w
                    .chunks
                    .iter()
                    .filter(of_kind)
                    .all(|c| removals.iter().any(|r| r.start == c.start));
                if any && all_removed {
                    now &= !bit;
                }
            }
            flag_patched = now != was;
            flag = Some((v.data_start - removed_before(v.start), now));
        }
        // The output: the input less the removed ranges, patched.
        let mut bytes = Vec::with_capacity(d.len());
        let mut at = 0usize;
        for r in &removals {
            bytes.extend_from_slice(&d[at..r.start]);
            at = r.end;
        }
        bytes.extend_from_slice(&d[at..]);
        if size_patched {
            bytes[4..8].copy_from_slice(&riff_size.to_le_bytes());
        }
        if let Some((fat, value)) = flag {
            bytes[fat] = value;
        }
        let live_exif = mode == NMode::Partial && !exif_chunks.is_empty();
        let mut edited = Vec::new();
        let mut kept = Vec::new();
        for (i, c) in w.chunks.iter().enumerate() {
            if removals.iter().any(|r| r.start == c.start) {
                continue;
            }
            let lenient = if &c.ty == b"EXIF" && mode == NMode::Partial {
                let off = removed_before(c.start);
                edited.push(c.data_start - off..c.data_end - off);
                Lenient::Data
            } else if w.vp8x == Some(i) {
                Lenient::FirstByte
            } else {
                Lenient::No
            };
            kept.push(Kept {
                ty: c.ty,
                data: c.data(d).to_vec(),
                lenient,
            });
        }
        Pred::Ok {
            bytes,
            removals,
            edited,
            may_refuse: live_exif,
            riff_size,
            size_patched,
            flag,
            flag_patched,
            kept,
        }
    }

    // ── the guards ─────────────────────────────────────────────────────────

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Guard {
        /// `Ok` output equals the prediction, byte for byte, outside the
        /// `EXIF` chunk data a partial policy may edit, in both directions.
        Prediction,
        /// The output's chunks are the input's kept chunks, in order, each
        /// byte-identical (an edited `EXIF` chunk keeps its length; the `VP8X`
        /// keeps all but its flag byte).
        KeptChunks,
        /// The removal records equal the predicted ones (offset, length, kind,
        /// category), and the patches are the size patch, the flag patch and
        /// patches inside an edited `EXIF` chunk, each exactly when predicted.
        Records,
        /// Length at most the input's, equal iff nothing was removed.
        Length,
        /// The declared RIFF size is the declared size less the bytes removed
        /// from inside the declared range.
        RiffSize,
        /// The `VP8X` flag byte has the bits of the metadata chunks that
        /// remain, and keeps the others.
        Vp8xFlags,
        /// The output's audit has no category of the policy.
        RemovedCategories,
        /// Every category the policy keeps has the findings it had.
        KeptFindings,
        /// No chunk the policy removes is left in the output.
        NoRemovableLeft,
        /// Stripping the output changes nothing.
        Idempotent,
        /// Stripping twice gives the same answer.
        Deterministic,
        /// The refusal class (and its detail) equals the prediction.
        ErrClass,
        /// `VerifyFailed` never happens on a WebP.
        VerifyAllowed,
    }

    pub const GUARDS: [Guard; 13] = [
        Guard::Prediction,
        Guard::KeptChunks,
        Guard::Records,
        Guard::Length,
        Guard::RiffSize,
        Guard::Vp8xFlags,
        Guard::RemovedCategories,
        Guard::KeptFindings,
        Guard::NoRemovableLeft,
        Guard::Idempotent,
        Guard::Deterministic,
        Guard::ErrClass,
        Guard::VerifyAllowed,
    ];

    fn outside(edited: &[Range<usize>], i: usize) -> bool {
        !edited.iter().any(|r| r.contains(&i))
    }

    /// The output has a valid 12-byte header (it always has: the input did).
    fn has_header(d: &[u8]) -> bool {
        d.len() >= 12 && &d[..4] == b"RIFF" && &d[8..12] == b"WEBP"
    }

    /// Whether the guard `g` holds. Each guard stands alone: it reads only its
    /// own part of the facts and the prediction.
    pub fn holds(g: Guard, input: &[u8], spec: StripSpec, f: &Facts, pred: &Pred) -> bool {
        match g {
            Guard::Prediction => match (&f.result, pred) {
                (Ok(ok), Pred::Ok { bytes, edited, .. }) => {
                    bytes.len() == ok.bytes.len()
                        && bytes
                            .iter()
                            .zip(&ok.bytes)
                            .enumerate()
                            .all(|(i, (a, b))| a == b || !outside(edited, i))
                }
                (Ok(_), _) => false,
                // A refusal is judged by `ErrClass`.
                (Err(_), _) => true,
            },
            Guard::KeptChunks => match (&f.result, pred) {
                (Ok(ok), Pred::Ok { kept, .. }) => {
                    if !has_header(&ok.bytes) {
                        return false;
                    }
                    let w = walk(&ok.bytes);
                    w.chunks.len() == kept.len()
                        && w.chunks.iter().zip(kept).all(|(c, k)| {
                            let data = c.data(&ok.bytes);
                            c.ty == k.ty
                                && data.len() == k.data.len()
                                && match k.lenient {
                                    Lenient::No => data == &k.data[..],
                                    Lenient::Data => true,
                                    Lenient::FirstByte => data[1..] == k.data[1..],
                                }
                        })
                }
                (Ok(_), _) => false,
                (Err(_), _) => true,
            },
            Guard::Records => match (&f.result, pred) {
                (
                    Ok(ok),
                    Pred::Ok {
                        removals,
                        edited,
                        size_patched,
                        flag,
                        flag_patched,
                        ..
                    },
                ) => {
                    let sizes = ok.patches.iter().filter(|p| p.2 == "riff-size").count();
                    let flags = ok.patches.iter().filter(|p| p.2 == "vp8x-flags").count();
                    ok.removals.len() == removals.len()
                        && ok.removals.iter().zip(removals).all(|(a, b)| {
                            a.offset == b.start
                                && a.len == b.end - b.start
                                && a.kind == b.kind
                                && match b.category {
                                    Some(c) => a.category == c,
                                    None => EXIF9.contains(&a.category),
                                }
                        })
                        && sizes == usize::from(*size_patched)
                        && flags == usize::from(*flag_patched)
                        && ok.patches.iter().all(|&(at, len, kind)| match kind {
                            "riff-size" => (at, len) == (4, 4),
                            "vp8x-flags" => len == 1 && flag.is_some_and(|(fat, _)| fat == at),
                            _ => edited.iter().any(|r| at >= r.start && at + len <= r.end),
                        })
                }
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
            Guard::RiffSize => match (&f.result, pred) {
                (Ok(ok), Pred::Ok { riff_size, .. }) => {
                    ok.bytes.len() >= 8
                        && u32::from_le_bytes([ok.bytes[4], ok.bytes[5], ok.bytes[6], ok.bytes[7]])
                            == *riff_size
                }
                (Ok(_), _) => false,
                (Err(_), _) => true,
            },
            Guard::Vp8xFlags => match (&f.result, pred) {
                (Ok(ok), Pred::Ok { flag, .. }) => match flag {
                    None => true,
                    Some((at, value)) => ok.bytes.get(*at) == Some(value),
                },
                (Ok(_), _) => false,
                (Err(_), _) => true,
            },
            Guard::RemovedCategories => f
                .out
                .as_ref()
                .is_none_or(|o| o.categories.iter().all(|c| !spec.contains(*c))),
            Guard::KeptFindings => f
                .out
                .as_ref()
                .is_none_or(|o| match (&o.kept_in, &o.kept_out) {
                    (Some(a), Some(b)) => a == b,
                    _ => true,
                }),
            Guard::NoRemovableLeft => match &f.result {
                Ok(ok) if has_header(&ok.bytes) => {
                    let mode = exif_mode(spec);
                    walk(&ok.bytes).chunks.iter().all(|c| {
                        if &c.ty == b"EXIF" {
                            mode != NMode::Whole
                        } else {
                            !matches!(fate(&c.ty, c.data(&ok.bytes), spec), Fate::Remove(..))
                        }
                    })
                }
                _ => true,
            },
            Guard::Idempotent => f.out.as_ref().is_none_or(|o| o.idempotent),
            Guard::Deterministic => f.deterministic,
            Guard::ErrClass => match (&f.result, pred) {
                (Ok(_), _) => true,
                (Err(ErrFact::Unstrippable(r, o)), Pred::Unstrippable(pr, po)) => {
                    r == pr && o == po
                }
                (Err(ErrFact::Uneditable(r, o)), Pred::Uneditable(pr, po)) => r == pr && o == po,
                // After the walk, and only where an entry edit is needed (any
                // reason of the Exif editor, but not the 17th chunk, which is
                // predicted exactly).
                (Err(ErrFact::Uneditable(r, _)), Pred::Ok { may_refuse, .. }) => {
                    *may_refuse && *r != "too-many-exif-segments"
                }
                // A removal of a whole chunk cannot cut another: never Overlap.
                _ => false,
            },
            Guard::VerifyAllowed => !matches!(&f.result, Err(ErrFact::VerifyFailed(_))),
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
        let (policy, webp) = decode(data);
        check_input(webp, policy);
    }

    pub fn check_input(webp: &[u8], policy: StripPolicy) {
        // Anything that is not a WebP has its own target.
        if sniff(webp) != Sniffed::Webp {
            return;
        }
        let spec = policy.spec();
        let facts = facts(webp, policy);
        let pred = predict(webp, spec);
        let bad = failures(webp, spec, &facts, &pred);
        assert!(
            bad.is_empty(),
            "guards {bad:?} failed\npolicy {}\nfacts {}\nprediction {}",
            policy.name(),
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
                edited,
                may_refuse,
                riff_size,
                flag,
                ..
            } => format!(
                "Ok len {} removals {removals:?} edited {edited:?} may_refuse {may_refuse} \
                 riff_size {riff_size} flag {flag:?}",
                bytes.len()
            ),
            other => format!("{other:?}"),
        }
    }
}
