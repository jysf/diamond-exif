// The SPEC-030 invariants of the WebP RIFF walker (DEC-019), shared by the fuzz
// target (`fuzz/fuzz_targets/webp_riff.rs`) and the stable-toolchain layout
// test (`tests/fuzz_layout.rs`) with `include!`, so the two cannot drift. This
// file is not a crate root; it is text included at module level, and it
// defines one module, `webp_riff_invariants`, whose `check` is the harness
// body.
//
// The walk is re-derived here from the raw bytes, independently of the
// library: a second, deliberately naive implementation of the DEC-019 rules
// with its own four-character-code table and 64-bit size arithmetic. Its answer
// is compared with `Webp::parse` field by field. Every rule that is an iff
// (the size mismatch, the pad byte, a chunk's kind, the `VP8X` fields, each
// flag mismatch, `MetadataWithoutVp8x`, `is_clean`) is asserted by equality
// with the model, which fails both when something expected is missing and when
// something unexpected is present, so a constant-true or constant-false
// implementation fails.

mod webp_riff_invariants {
    use exif_core::webp::{ChunkKind, Webp, MAX_WEBP_CHUNKS, MAX_WEBP_ISSUES};
    use exif_core::Error;

    /// One chunk, as plain data: (offset, four-character code, kind, data
    /// range, end of the chunk including its pad byte).
    pub(super) type ChunkT = (usize, [u8; 4], ChunkKind, (usize, usize), usize);

    /// What an implementation reports, as comparable data.
    #[derive(Clone, Debug, PartialEq)]
    pub(super) struct Observed {
        pub(super) chunks: Vec<ChunkT>,
        pub(super) issues: Vec<(Error, usize)>,
        /// (flags, canvas width, canvas height) of the first `VP8X`, when it
        /// has 10 bytes.
        pub(super) vp8x: Option<(u8, u32, u32)>,
    }

    /// The model's own code table. The second field is true for a spelling
    /// that is read although it is not canonical.
    fn kind_of(code: &[u8; 4]) -> (ChunkKind, bool) {
        match code.as_slice() {
            b"VP8X" => (ChunkKind::Vp8x, false),
            b"VP8 " => (ChunkKind::Vp8, false),
            b"VP8L" => (ChunkKind::Vp8l, false),
            b"ALPH" => (ChunkKind::Alph, false),
            b"ANIM" => (ChunkKind::Anim, false),
            b"ANMF" => (ChunkKind::Anmf, false),
            b"ICCP" => (ChunkKind::Iccp, false),
            b"EXIF" => (ChunkKind::Exif, false),
            b"XMP " => (ChunkKind::Xmp, false),
            b"XMP\0" => (ChunkKind::Xmp, true),
            _ => (ChunkKind::Other, false),
        }
    }

    pub(super) struct Walked {
        pub(super) chunks: Vec<ChunkT>,
        pub(super) issues: Vec<(Error, usize)>,
        pub(super) vp8x: Option<(u8, u32, u32)>,
        /// The walk stopped on a framing error or a cap. Derived here, not
        /// read from the library.
        pub(super) halted: bool,
    }

    struct Model {
        issues: Vec<(Error, usize)>,
        halted: bool,
    }

    impl Model {
        fn record(&mut self, e: Error, at: usize) {
            if self.halted {
                return;
            }
            if self.issues.len() >= MAX_WEBP_ISSUES {
                self.issues.push((
                    Error::TooManyIssues {
                        limit: MAX_WEBP_ISSUES,
                    },
                    at,
                ));
                self.halted = true;
            } else {
                self.issues.push((e, at));
            }
        }

        fn halt(&mut self, e: Error, at: usize) {
            self.record(e, at);
            self.halted = true;
        }
    }

    fn le32(d: &[u8], at: usize) -> u32 {
        u32::from_le_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]])
    }

    fn le24(d: &[u8], at: usize) -> u32 {
        u32::from_le_bytes([d[at], d[at + 1], d[at + 2], 0])
    }

    /// Walk `d`, which must have a valid 12-byte header.
    pub(super) fn walk(d: &[u8]) -> Walked {
        let len = d.len();
        let mut m = Model {
            issues: Vec::new(),
            halted: false,
        };
        let mut chunks: Vec<ChunkT> = Vec::new();
        let declared = le32(d, 4);
        if declared as u64 != (len as u64) - 8 {
            m.record(
                Error::RiffSizeMismatch {
                    declared,
                    actual: len - 8,
                },
                4,
            );
        }
        let mut vp8x: Option<(usize, u8, u32, u32)> = None;
        let mut seen_vp8x = false;
        let mut p = 12usize;
        while p < len && !m.halted {
            let rem = len - p;
            if rem < 8 {
                m.halt(Error::Truncated { at: p, len: 8 }, p);
                break;
            }
            let code = [d[p], d[p + 1], d[p + 2], d[p + 3]];
            let size = le32(d, p + 4);
            let total = size as u64 + 8;
            if total > rem as u64 {
                m.halt(
                    Error::Truncated {
                        at: p,
                        len: total as usize,
                    },
                    p,
                );
                break;
            }
            if chunks.len() >= MAX_WEBP_CHUNKS {
                m.halt(
                    Error::TooManyChunks {
                        limit: MAX_WEBP_CHUNKS,
                    },
                    p,
                );
                break;
            }
            let (kind, odd_spelling) = kind_of(&code);
            let (ds, de) = (p + 8, p + 8 + size as usize);
            if odd_spelling {
                m.record(
                    Error::NonCanonicalChunkId {
                        chunk_id: code,
                        at: p,
                    },
                    p,
                );
            }
            if kind == ChunkKind::Vp8x {
                if size < 10 {
                    m.record(Error::BadVp8x { length: size }, p);
                }
                if !chunks.is_empty() {
                    m.record(Error::MisplacedVp8x { at: p }, p);
                }
                if !seen_vp8x {
                    seen_vp8x = true;
                    if size >= 10 {
                        vp8x = Some((p, d[ds], le24(d, ds + 4) + 1, le24(d, ds + 7) + 1));
                    }
                }
            }
            // The pad byte exists when the size is odd and the input goes on.
            let end = if size % 2 == 1 && de < len {
                de + 1
            } else {
                de
            };
            if size % 2 == 1 && de == len {
                m.record(Error::MissingPad { at: p }, p);
            }
            if m.halted {
                break;
            }
            chunks.push((p, code, kind, (ds, de), end));
            p = end;
        }

        // End of the walk (suppressed by a halt).
        if !m.halted {
            let present = |k: ChunkKind| chunks.iter().any(|c| c.2 == k);
            match vp8x {
                Some((at, flags, _, _)) => {
                    for (name, bit, k) in [
                        ("ICC", 0x20u8, ChunkKind::Iccp),
                        ("EXIF", 0x08, ChunkKind::Exif),
                        ("XMP", 0x04, ChunkKind::Xmp),
                    ] {
                        let set = flags & bit != 0;
                        if set != present(k) {
                            m.record(
                                Error::ChunkFlagMismatch {
                                    flag: name,
                                    declared: set,
                                },
                                at,
                            );
                        }
                    }
                }
                None if !seen_vp8x => {
                    if let Some(c) = chunks.iter().find(|c| {
                        c.2 == ChunkKind::Exif || c.2 == ChunkKind::Xmp || c.2 == ChunkKind::Iccp
                    }) {
                        m.record(Error::MetadataWithoutVp8x, c.0);
                    }
                }
                None => {}
            }
        }
        Walked {
            chunks,
            issues: m.issues,
            vp8x: vp8x.map(|(_, f, w, h)| (f, w, h)),
            halted: m.halted,
        }
    }

    fn range_of(base: &[u8], part: &[u8]) -> (usize, usize) {
        let start = part.as_ptr() as usize - base.as_ptr() as usize;
        (start, start + part.len())
    }

    /// The library's walk as plain data.
    fn observe(d: &[u8], w: &Webp<'_>) -> Observed {
        let chunks = w
            .chunks()
            .iter()
            .map(|c| {
                (
                    c.offset(),
                    c.fourcc(),
                    c.kind(),
                    range_of(d, c.data()),
                    c.range().end,
                )
            })
            .collect();
        let issues = w
            .issues()
            .iter()
            .map(|i| (i.error().clone(), i.offset()))
            .collect();
        let vp8x = w
            .vp8x()
            .map(|v| (v.flags(), v.canvas_width(), v.canvas_height()));
        Observed {
            chunks,
            issues,
            vp8x,
        }
    }

    /// The harness body with a hook that may corrupt the library's observed
    /// answer before comparison (the red-proofs use it; `check` does not).
    pub(super) fn compare(data: &[u8], tamper: fn(&mut Observed)) {
        let strict = Webp::parse_strict(data);
        let webp = match Webp::parse(data) {
            Ok(w) => w,
            Err(e) => {
                // Fatal exactly when not a WebP; both entry points agree.
                let is_webp = data.len() >= 12 && &data[..4] == b"RIFF" && &data[8..12] == b"WEBP";
                assert!(!is_webp, "fatal on a WebP");
                if data.len() < 12 {
                    assert_eq!(e, Error::Truncated { at: 0, len: 12 });
                } else {
                    let mut mark = [0u8; 12];
                    mark.copy_from_slice(&data[..12]);
                    assert_eq!(e, Error::NotWebp { mark });
                }
                assert_eq!(strict.err(), Some(e), "strict disagrees on a fatal input");
                return;
            }
        };
        assert!(
            &data[..4] == b"RIFF" && &data[8..12] == b"WEBP",
            "a non-WebP was accepted"
        );

        // parse_strict is Ok iff clean; Err is the first issue's error.
        assert_eq!(strict.is_ok(), webp.is_clean(), "strict iff clean");
        assert_eq!(webp.is_clean(), webp.issues().is_empty());
        if let Err(e) = strict {
            assert_eq!(Some(&e), webp.issues().first().map(|i| i.error()));
        }
        assert_eq!(webp.declared_size(), le32(data, 4));

        let want = walk(data);
        let mut got = observe(data, &webp);
        tamper(&mut got);
        assert_eq!(got.issues, want.issues, "issue list differs from the model");
        assert_eq!(got.chunks, want.chunks, "chunks differ from the model");
        assert_eq!(got.vp8x, want.vp8x, "VP8X differs from the model");
        assert!(got.issues.len() <= MAX_WEBP_ISSUES + 1);
        assert!(got.chunks.len() <= MAX_WEBP_CHUNKS);

        // The halting issue, when there is one, is the last, and a halt
        // leaves no end-of-walk issue behind it.
        if want.halted {
            assert!(matches!(
                want.issues.last().map(|i| &i.0),
                Some(
                    Error::Truncated { .. }
                        | Error::TooManyChunks { .. }
                        | Error::TooManyIssues { .. }
                )
            ));
        }

        // Chunks are framed, increasing, non-overlapping and borrowed; the
        // walk ends at the end of the input or at a halt.
        let mut prev_end = 12usize;
        for c in webp.chunks() {
            assert!(c.offset() >= prev_end && c.range().end <= data.len());
            assert_eq!(c.range().start, c.offset());
            assert!(c.data().len() <= c.range().len());
            let (s, e) = range_of(data, c.data());
            assert!(
                s >= c.offset() + 8 && e <= data.len(),
                "payload not borrowed"
            );
            prev_end = c.range().end;
        }
        if !want.halted {
            // No halt: the chunks tile the input from byte 12 to its end.
            assert_eq!(prev_end, data.len().max(12), "walk stopped short");
        }
    }

    /// The harness body: neither entry point panics and the walk equals the
    /// independent model.
    pub(super) fn check(data: &[u8]) {
        compare(data, |_| {});
    }
}
