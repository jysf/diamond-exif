// The SPEC-037 invariants of `exif_core::strip` on a PNG (DEC-021), shared by
// the fuzz target (`fuzz/fuzz_targets/rewrite_png.rs`) and the stable-toolchain
// layout test (`tests/fuzz_layout.rs`) with `include!`, so the two cannot
// drift. This file is not a crate root; it is text included at module level,
// and it defines one module, `rewrite_png_invariants`.
//
// The harness holds an INDEPENDENT NAIVE STRIPPER. It has its own chunk walk
// (a second implementation of the DEC-017 framing, caps and issue counting,
// with its own bitwise CRC-32), its own trailer scan, its own typing of chunks
// and text keywords (typed again from DEC-020 and DEC-021, never read from
// `src/write/png.rs`), its own carrier rule (clause 4), its own `eXIf` mode
// rule (clause 3) and its own overlap rule for a trailer that goes whole. From
// those it predicts the exact output (the input less the removed ranges) or the
// exact refusal, and the library's answer is compared with the prediction.
//
// What the harness DOES call in the library, exactly:
//   * `strip` (and nothing else of `exif_core::write`: the removal records are
//     flattened through the public result type);
//   * `audit(input)` and `audit(output)` in `facts`, for the findings'
//     categories and counts (the TIFF engine inside an `eXIf` chunk is the
//     business of the `audit` and `rewrite_jpeg` targets);
//   * `sniff` (to leave non-PNG candidates to the other targets), the caps
//     `MAX_PNG_CHUNKS`, `MAX_PNG_ISSUES`, `MAX_PNG_TRAILER_CHUNKS`, and
//     `Category::ALL`, `StripPolicy`, `StripSpec`.
//
// What it does NOT model: the bytes of an edited `eXIf` chunk (the entry
// editor is covered by `rewrite_jpeg`'s naive editor); there the harness checks
// that the chunk is either the input's or has the same length and a valid CRC,
// and that every patch the library records lies inside such a chunk. A refusal
// `Uneditable` is accepted only when the policy needs an edit and an `eXIf`
// chunk is live.
//
// Input layout: byte 0 selects the policy (0..=4 the presets in
// `StripPolicy::PRESETS` order, 5 `Custom(none)`, 6 `Custom` from the 16-bit
// little-endian mask in bytes 1 and 2, bit i meaning `Category::ALL[i]`, 7 `All`,
// 8 `Custom(OtherSegment)`, 9 `Custom(Icc)`, 10 `Custom(Xmp)`, 11 `Custom(Gps)`,
// 12 `Custom(Jumbf)`, 13 `Custom(FreeText)`, 14 `Custom(Timestamps)`, 15
// `Custom(Software)`, 16 and above `All`), the rest is the PNG candidate (after
// bytes 1 and 2 for 6).
//
// Every guard is its own function over `Facts` (the library's answer,
// flattened) and the prediction, so a planted bug in the flattened answer turns
// exactly its guard red (see `tests/fuzz_layout.rs`). `VerifyFailed` is always
// a harness failure: on a PNG the plan and the self-check must agree.

mod rewrite_png_invariants {
    use std::ops::Range;

    use exif_core::audit::{audit, sniff, Category, FileAudit, Sniffed};
    use exif_core::png::{MAX_PNG_CHUNKS, MAX_PNG_ISSUES, MAX_PNG_TRAILER_CHUNKS};
    use exif_core::policy::{StripPolicy, StripSpec};
    use exif_core::write::{RemovalKind, UneditableReason, UnstrippableReason};
    use exif_core::{strip, StripError};

    const MAX_LEN: u32 = 0x7FFF_FFFF;

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

    // ── the policy byte ────────────────────────────────────────────────────

    fn only(c: Category) -> StripPolicy {
        StripPolicy::Custom(StripSpec::none().include(c))
    }

    /// Decode the leading bytes into a policy and the PNG candidate.
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
        /// `(offset, length)` of every patch, in output coordinates.
        pub patches: Vec<(usize, usize)>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum ErrFact {
        /// Unsupported or NotJpeg (the harness only feeds PNGs).
        NotAPng,
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
            RemovalKind::Trailer => "trailer",
            RemovalKind::Text => "text",
            RemovalKind::Time => "time",
            RemovalKind::RawProfile => "raw-profile",
            RemovalKind::Carrier => "carrier",
            RemovalKind::OtherChunk => "other-chunk",
            _ => "other",
        }
    }

    fn flatten_err(e: &StripError) -> ErrFact {
        match e {
            StripError::Unsupported { .. } | StripError::NotJpeg => ErrFact::NotAPng,
            StripError::Unstrippable { reason, offset } => ErrFact::Unstrippable(
                match reason {
                    UnstrippableReason::BadSegmentLength => "bad-length",
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
                patches: s.patches().iter().map(|p| (p.offset, p.len)).collect(),
            }),
            Err(e) => Err(flatten_err(e)),
        }
    }

    fn kept_counts(a: &FileAudit<'_>, spec: StripSpec) -> Option<Vec<(Category, usize)>> {
        if a.health().inventory_truncated {
            return None;
        }
        // When every Exif category goes, every `eXIf` chunk goes with the
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

    // ── the naive chunk walk (DEC-017, typed again) ────────────────────────

    /// Bitwise CRC-32 (reflected, polynomial 0xEDB88320), no table.
    fn crc32(parts: &[&[u8]]) -> u32 {
        let mut c: u32 = 0xFFFF_FFFF;
        for part in parts {
            for &b in *part {
                c ^= u32::from(b);
                for _ in 0..8 {
                    c = if c & 1 == 1 {
                        (c >> 1) ^ 0xEDB8_8320
                    } else {
                        c >> 1
                    };
                }
            }
        }
        !c
    }

    fn be32(d: &[u8], at: usize) -> u32 {
        u32::from_be_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]])
    }

    #[derive(Debug, Clone)]
    struct NChunk {
        start: usize,
        end: usize,
        ty: [u8; 4],
        in_trailer: bool,
    }

    impl NChunk {
        fn data<'a>(&self, d: &'a [u8]) -> &'a [u8] {
            &d[self.start + 8..self.end - 4]
        }
    }

    struct NWalk {
        chunks: Vec<NChunk>,
        /// The first fault the writer refuses: its reason and input offset.
        halt: Option<(&'static str, usize)>,
        /// The start of the bytes after `IEND`, when there is an `IEND`.
        trailer_start: Option<usize>,
    }

    /// The walk of DEC-017, with its caps, counting issues the way the cap
    /// does. Only the halting faults matter to the writer.
    fn walk(d: &[u8]) -> NWalk {
        let len = d.len();
        let mut w = NWalk {
            chunks: Vec::new(),
            halt: None,
            trailer_start: None,
        };
        let mut issues = 0usize;
        // `record`: an ordinary issue, or the cap's terminal one (which halts).
        fn record(issues: &mut usize, halt: &mut Option<(&'static str, usize)>, at: usize) {
            if halt.is_some() {
                return;
            }
            if *issues >= MAX_PNG_ISSUES {
                *halt = Some(("too-many-issues", at));
            } else {
                *issues += 1;
            }
        }
        let mut p = 8usize;
        let mut iend = false;
        while p < len && w.halt.is_none() {
            let rem = len - p;
            if rem < 12 {
                w.halt = Some(("chunk-past-end", p));
                break;
            }
            let length = be32(d, p);
            let ty = [d[p + 4], d[p + 5], d[p + 6], d[p + 7]];
            if length > MAX_LEN {
                w.halt = Some(("bad-length", p));
                break;
            }
            let total = length as u64 + 12;
            if total > rem as u64 {
                w.halt = Some(("chunk-past-end", p));
                break;
            }
            let total = total as usize;
            if w.chunks.len() >= MAX_PNG_CHUNKS {
                w.halt = Some(("too-many-segments", p));
                break;
            }
            let first = w.chunks.is_empty();
            if ty.iter().any(|b| !b.is_ascii_alphabetic()) {
                record(&mut issues, &mut w.halt, p);
            }
            if first && &ty != b"IHDR" {
                record(&mut issues, &mut w.halt, p);
            }
            if &ty == b"IHDR" && length != 13 {
                record(&mut issues, &mut w.halt, p);
            }
            let stored = be32(d, p + total - 4);
            if crc32(&[&ty, &d[p + 8..p + total - 4]]) != stored {
                record(&mut issues, &mut w.halt, p);
            }
            if w.halt.is_some() {
                break;
            }
            w.chunks.push(NChunk {
                start: p,
                end: p + total,
                ty,
                in_trailer: false,
            });
            if &ty == b"IEND" {
                iend = true;
                w.trailer_start = Some(p + total);
                break;
            }
            p += total;
        }
        if w.halt.is_none() && !iend {
            if w.chunks.is_empty() {
                record(&mut issues, &mut w.halt, len);
            }
            record(&mut issues, &mut w.halt, len);
        }
        w
    }

    /// The chunks framed after `IEND` (at most `MAX_PNG_TRAILER_CHUNKS`: the
    /// first thing that does not fit ends the scan), and whether the cap
    /// stopped it with another chunk still to come.
    fn trailer_scan(d: &[u8], start: usize) -> (Vec<NChunk>, bool) {
        let frames = |at: usize| -> Option<(usize, [u8; 4])> {
            let rem = d.len() - at;
            if rem < 12 {
                return None;
            }
            let length = be32(d, at);
            if length > MAX_LEN {
                return None;
            }
            let total = length as usize + 12;
            if total > rem {
                return None;
            }
            Some((at + total, [d[at + 4], d[at + 5], d[at + 6], d[at + 7]]))
        };
        let mut out = Vec::new();
        let mut at = start;
        for _ in 0..MAX_PNG_TRAILER_CHUNKS {
            let Some((end, ty)) = frames(at) else {
                return (out, false);
            };
            out.push(NChunk {
                start: at,
                end,
                ty,
                in_trailer: true,
            });
            at = end;
        }
        (out, frames(at).is_some())
    }

    // ── the naive rules (DEC-020, DEC-021) ─────────────────────────────────

    /// DEC-021 clause 4: a carrier goes with any tag category, `Xmp` or `Iptc`,
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

    /// DEC-021 clause 3.
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

    /// The category of a text keyword: a row of DEC-020's table when it equals
    /// the row's name, or equals it after its first letter is uppercased.
    fn keyword_category(kw: &[u8]) -> Category {
        const ROWS: [(&[u8], Category); 7] = [
            (b"Artist", Category::Authorship),
            (b"Author", Category::Authorship),
            (b"Copyright", Category::Authorship),
            (b"Creation Time", Category::Timestamps),
            (b"create-date", Category::Timestamps),
            (b"modify-date", Category::Timestamps),
            (b"Software", Category::Software),
        ];
        for (name, cat) in ROWS {
            let upper_first =
                !kw.is_empty() && kw[0].to_ascii_uppercase() == name[0] && kw[1..] == name[1..];
            if kw == name || upper_first {
                return cat;
            }
        }
        Category::FreeText
    }

    /// What the naive reader finds in a `tEXt`, `zTXt` or `iTXt` chunk: its
    /// keyword and whether an `iTXt` language tag is set. `None`: nothing to
    /// locate (no NUL, a missing method byte, a truncated `iTXt` header).
    fn text_parts(ty: &[u8; 4], data: &[u8]) -> Option<(Vec<u8>, bool)> {
        let end = data.iter().position(|&b| b == 0)?;
        let keyword = data[..end].to_vec();
        let rest = &data[end + 1..];
        match ty {
            b"tEXt" => Some((keyword, false)),
            b"zTXt" => {
                // A method byte; the body may be empty.
                rest.get(1..)?;
                Some((keyword, false))
            }
            b"iTXt" => {
                // flag, method, language NUL, translated keyword NUL.
                let rest = rest.get(2..)?;
                let lang_end = rest.iter().position(|&b| b == 0)?;
                let rest = &rest[lang_end + 1..];
                rest.iter().position(|&b| b == 0)?;
                Some((keyword, lang_end > 0))
            }
            _ => None,
        }
    }

    /// What the policy does to one chunk that is not an `eXIf`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Fate {
        /// The writer has no rule for it: always kept, and not a finding.
        NoRule,
        /// Removed, with this category and removal kind.
        Remove(Category, &'static str),
        /// A rule applies, the policy keeps it; `located` says whether the
        /// reader found a region for it (a trailer that goes whole would cut it).
        Keep { located: bool },
    }

    fn fate(ty: &[u8; 4], data: &[u8], spec: StripSpec) -> Fate {
        let by_cat = |c: Category, kind: &'static str, located: bool| {
            if spec.contains(c) {
                Fate::Remove(c, kind)
            } else {
                Fate::Keep { located }
            }
        };
        let carrier = |kind: &'static str| {
            if carriers_removed(spec) {
                Fate::Remove(Category::OtherSegment, kind)
            } else {
                Fate::Keep { located: true }
            }
        };
        match ty {
            b"tEXt" | b"zTXt" | b"iTXt" => match text_parts(ty, data) {
                // Nothing located: still text.
                None => by_cat(Category::FreeText, "text", false),
                Some((kw, tagged)) => {
                    if kw == b"XML:com.adobe.xmp" && !tagged {
                        by_cat(Category::Xmp, "xmp", true)
                    } else if kw.starts_with(b"Raw profile type ")
                        || kw.starts_with(b"raw profile type ")
                    {
                        carrier("raw-profile")
                    } else {
                        by_cat(keyword_category(&kw), "text", true)
                    }
                }
            },
            b"iCCP" => {
                let located = data
                    .iter()
                    .position(|&b| b == 0)
                    .is_some_and(|end| data.get(end + 1).is_some());
                by_cat(Category::Icc, "icc", located)
            }
            b"tIME" => by_cat(Category::Timestamps, "time", true),
            b"caBX" => by_cat(Category::Jumbf, "jumbf", true),
            // ExifTool matches the types zxIf and eXIf in any ASCII case (the
            // exact eXIf is not a carrier and never reaches here).
            t if (t.eq_ignore_ascii_case(b"zxif") || t.eq_ignore_ascii_case(b"exif"))
                && t != b"eXIf" =>
            {
                carrier("carrier")
            }
            b"tXMP" | b"meTa" | b"seAl" => carrier("carrier"),
            b"dSIG" | b"cpIp" | b"gIFx" | b"gIFg" | b"gIFt" | b"fRAc" => {
                by_cat(Category::OtherSegment, "other-chunk", true)
            }
            _ => Fate::NoRule,
        }
    }

    // ── the prediction ─────────────────────────────────────────────────────

    /// One removal the naive stripper makes.
    #[derive(Debug, Clone, PartialEq)]
    pub struct NRem {
        pub start: usize,
        pub end: usize,
        pub kind: &'static str,
        /// `None`: any Exif-set category (a removed `eXIf` chunk is recorded
        /// under the first Exif finding it holds).
        pub category: Option<Category>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum Pred {
        Unstrippable(&'static str, usize),
        Overlap(usize),
        Ok {
            /// The input less the removed ranges.
            bytes: Vec<u8>,
            removals: Vec<NRem>,
            /// Output ranges of the `eXIf` chunks a partial policy may edit.
            edited: Vec<Range<usize>>,
            /// A partial policy with a live `eXIf` chunk: `Uneditable` is a
            /// fair answer.
            may_refuse: bool,
        },
    }

    pub fn predict(d: &[u8], spec: StripSpec) -> Pred {
        let w = walk(d);
        if let Some((reason, at)) = w.halt {
            return Pred::Unstrippable(reason, at);
        }
        let mut chunks = w.chunks.clone();
        let trailer_len = w.trailer_start.map_or(0, |t| d.len() - t);
        let tstart = w.trailer_start.unwrap_or(d.len());
        let trailer_gone = spec.contains(Category::OtherSegment) && trailer_len > 0;
        if trailer_len > 0 {
            let (framed, capped) = trailer_scan(d, tstart);
            if capped && !trailer_gone {
                return Pred::Unstrippable("too-many-segments", tstart);
            }
            chunks.extend(framed);
        }
        let mode = exif_mode(spec);
        let live = |c: &NChunk| !(trailer_gone && c.in_trailer);
        // A trailer that goes whole cuts every chunk in it: one the policy would
        // keep and that has findings of its own is a refusal.
        if trailer_gone {
            for c in chunks.iter().filter(|c| c.in_trailer) {
                let would_go = if &c.ty == b"eXIf" {
                    mode == NMode::Whole
                } else {
                    match fate(&c.ty, c.data(d), spec) {
                        Fate::NoRule | Fate::Remove(..) => true,
                        Fate::Keep { located } => !located,
                    }
                };
                if !would_go {
                    return Pred::Overlap(c.start);
                }
            }
        }
        let mut removals: Vec<NRem> = Vec::new();
        let mut exif_live = false;
        for c in chunks.iter().filter(|c| live(c)) {
            if &c.ty == b"eXIf" {
                match mode {
                    NMode::Whole => removals.push(NRem {
                        start: c.start,
                        end: c.end,
                        kind: "exif",
                        category: None,
                    }),
                    NMode::Partial => exif_live = true,
                    NMode::Untouched => {}
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
        if trailer_gone {
            removals.push(NRem {
                start: tstart,
                end: d.len(),
                kind: "trailer",
                category: Some(Category::OtherSegment),
            });
        }
        removals.sort_by_key(|r| (r.start, r.end));
        // The output: the input less the removed ranges; and where each
        // possibly edited `eXIf` chunk lands in it.
        let mut bytes = Vec::with_capacity(d.len());
        let mut at = 0usize;
        for r in &removals {
            bytes.extend_from_slice(&d[at..r.start]);
            at = r.end;
        }
        bytes.extend_from_slice(&d[at..]);
        let mut edited = Vec::new();
        if exif_live {
            for c in chunks.iter().filter(|c| live(c) && &c.ty == b"eXIf") {
                let gone: usize = removals
                    .iter()
                    .filter(|r| r.end <= c.start)
                    .map(|r| r.end - r.start)
                    .sum();
                edited.push(c.start - gone..c.end - gone);
            }
        }
        Pred::Ok {
            bytes,
            removals,
            edited,
            may_refuse: exif_live,
        }
    }

    // ── the guards ─────────────────────────────────────────────────────────

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Guard {
        /// `Ok` output equals the prediction, byte for byte, outside the
        /// `eXIf` chunks a partial policy may edit (kept chunks identical, in
        /// order, a kept bad CRC still bad), in both directions.
        Prediction,
        /// The removal records equal the predicted ones (offset, length, kind,
        /// category), and every patch lies inside a possibly edited chunk.
        Records,
        /// Length at most the input's, equal iff nothing was removed.
        Length,
        /// The output's audit has no category of the policy.
        RemovedCategories,
        /// Every category the policy keeps has the findings it had.
        KeptFindings,
        /// An edited `eXIf` chunk keeps its length and has a valid CRC (or is
        /// the input's, untouched).
        EditedCrc,
        /// No carrier chunk or raw profile is left when the policy removes them.
        NoCarrierLeft,
        /// Stripping the output changes nothing.
        Idempotent,
        /// Stripping twice gives the same answer.
        Deterministic,
        /// The refusal class (and its detail) equals the prediction.
        ErrClass,
        /// `VerifyFailed` never happens on a PNG.
        VerifyAllowed,
    }

    pub const GUARDS: [Guard; 11] = [
        Guard::Prediction,
        Guard::Records,
        Guard::Length,
        Guard::RemovedCategories,
        Guard::KeptFindings,
        Guard::EditedCrc,
        Guard::NoCarrierLeft,
        Guard::Idempotent,
        Guard::Deterministic,
        Guard::ErrClass,
        Guard::VerifyAllowed,
    ];

    fn outside(edited: &[Range<usize>], i: usize) -> bool {
        !edited.iter().any(|r| r.contains(&i))
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
            Guard::Records => match (&f.result, pred) {
                (
                    Ok(ok),
                    Pred::Ok {
                        removals, edited, ..
                    },
                ) => {
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
                        && ok.patches.iter().all(|&(at, len)| {
                            edited.iter().any(|r| at >= r.start && at + len <= r.end)
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
            Guard::EditedCrc => match (&f.result, pred) {
                (Ok(ok), Pred::Ok { edited, .. }) => {
                    // The input's chunks at those places, in order, are the
                    // predicted bytes' (the same ranges).
                    let Pred::Ok { bytes, .. } = pred else {
                        return true;
                    };
                    edited.iter().all(|r| {
                        let (Some(out), Some(was)) =
                            (ok.bytes.get(r.clone()), bytes.get(r.clone()))
                        else {
                            return false;
                        };
                        if out == was {
                            return true;
                        }
                        out.len() >= 12
                            && out[..8] == was[..8]
                            && crc32(&[&out[4..out.len() - 4]])
                                == u32::from_be_bytes([
                                    out[out.len() - 4],
                                    out[out.len() - 3],
                                    out[out.len() - 2],
                                    out[out.len() - 1],
                                ])
                    })
                }
                _ => true,
            },
            Guard::NoCarrierLeft => match &f.result {
                Ok(ok) if carriers_removed(spec) => {
                    let w = walk(&ok.bytes);
                    let mut all = w.chunks.clone();
                    if let Some(t) = w.trailer_start {
                        all.extend(trailer_scan(&ok.bytes, t).0);
                    }
                    all.iter().all(|c| !is_carrier(&c.ty, c.data(&ok.bytes)))
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
                (Err(ErrFact::Overlap(o)), Pred::Overlap(po)) => o == po,
                // The refusal comes after the walk and the trailer rule, and
                // only where an entry edit is needed.
                (Err(ErrFact::Uneditable(..)), Pred::Ok { may_refuse, .. }) => *may_refuse,
                (Err(ErrFact::VerifyFailed(_)), _) => true,
                _ => false,
            },
            Guard::VerifyAllowed => !matches!(&f.result, Err(ErrFact::VerifyFailed(_))),
        }
    }

    /// Whether a chunk is an opaque carrier or a raw profile (clause 4), judged
    /// by its type and keyword alone.
    fn is_carrier(ty: &[u8; 4], data: &[u8]) -> bool {
        match ty {
            t if (t.eq_ignore_ascii_case(b"zxif") || t.eq_ignore_ascii_case(b"exif"))
                && t != b"eXIf" =>
            {
                true
            }
            b"tXMP" | b"meTa" | b"seAl" => true,
            b"tEXt" | b"zTXt" | b"iTXt" => text_parts(ty, data).is_some_and(|(kw, _)| {
                kw.starts_with(b"Raw profile type ") || kw.starts_with(b"raw profile type ")
            }),
            _ => false,
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
        let (policy, png) = decode(data);
        check_input(png, policy);
    }

    pub fn check_input(png: &[u8], policy: StripPolicy) {
        // Anything that is not a PNG has its own target.
        if sniff(png) != Sniffed::Png {
            return;
        }
        let spec = policy.spec();
        let facts = facts(png, policy);
        let pred = predict(png, spec);
        let bad = failures(png, spec, &facts, &pred);
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
            } => format!(
                "Ok len {} removals {removals:?} edited {edited:?} may_refuse {may_refuse}",
                bytes.len()
            ),
            other => format!("{other:?}"),
        }
    }
}
