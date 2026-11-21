// The SPEC-028 invariants of the PNG chunk walker (DEC-017), shared by the fuzz
// target (`fuzz/fuzz_targets/png_chunks.rs`) and the stable-toolchain layout
// test (`tests/fuzz_layout.rs`) with `include!`, so the two cannot drift. This
// file is not a crate root; it is text included at module level, and it
// defines one module, `png_chunk_invariants`, whose `check` is the harness
// body.
//
// The walk is re-derived here, independently of the library: a second,
// deliberately naive implementation of the DEC-017 rules over the raw bytes,
// with its own bitwise CRC-32 (not the library's table) and its own kind
// table. The answer is compared with `Png::parse` field by field. Every rule
// that is an iff (a CRC match, a kind, `after_idat`, `is_clean`, the trailer,
// the end-of-walk issues) is asserted by equality with the model, which fails
// both when something expected is missing and when something unexpected is
// present, so a constant-true or constant-false implementation fails.

mod png_chunk_invariants {
    use exif_core::png::{ChunkKind, Png, MAX_PNG_CHUNKS, MAX_PNG_ISSUES};
    use exif_core::Error;

    const SIG: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    const MAX_LEN: u32 = 0x7FFF_FFFF;

    /// Bitwise CRC-32 (reflected, polynomial 0xEDB88320), no table.
    pub(super) fn crc32_bitwise(bytes: &[u8]) -> u32 {
        let mut c: u32 = 0xFFFF_FFFF;
        for &b in bytes {
            c ^= u32::from(b);
            for _ in 0..8 {
                c = if c & 1 == 1 {
                    (c >> 1) ^ 0xEDB8_8320
                } else {
                    c >> 1
                };
            }
        }
        !c
    }

    fn kind_of(ty: &[u8; 4]) -> ChunkKind {
        match ty.as_slice() {
            b"IHDR" => ChunkKind::Ihdr,
            b"PLTE" => ChunkKind::Plte,
            b"IDAT" => ChunkKind::Idat,
            b"IEND" => ChunkKind::Iend,
            b"eXIf" => ChunkKind::Exif,
            b"iTXt" => ChunkKind::Itxt,
            b"tEXt" => ChunkKind::Text,
            b"zTXt" => ChunkKind::Ztxt,
            b"iCCP" => ChunkKind::Iccp,
            _ => ChunkKind::Other,
        }
    }

    struct Chunk {
        offset: usize,
        ty: [u8; 4],
        data_len: usize,
        stored: u32,
        crc_ok: bool,
        kind: ChunkKind,
        after_idat: bool,
    }

    struct Model {
        chunks: Vec<Chunk>,
        issues: Vec<(Error, usize)>,
        halted: bool,
        iend: Option<usize>,
        trailer_start: usize,
    }

    impl Model {
        fn record(&mut self, e: Error, at: usize) {
            if self.halted {
                return;
            }
            if self.issues.len() >= MAX_PNG_ISSUES {
                self.issues.push((
                    Error::TooManyIssues {
                        limit: MAX_PNG_ISSUES,
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

    fn be32(d: &[u8], at: usize) -> u32 {
        u32::from_be_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]])
    }

    fn model(d: &[u8]) -> Model {
        let len = d.len();
        let mut m = Model {
            chunks: Vec::new(),
            issues: Vec::new(),
            halted: false,
            iend: None,
            trailer_start: len,
        };
        let mut p = 8usize;
        let mut seen_idat = false;
        while p < len && !m.halted {
            let rem = len - p;
            if rem < 12 {
                m.halt(Error::Truncated { at: p, len: 12 }, p);
                break;
            }
            let length = be32(d, p);
            let ty = [d[p + 4], d[p + 5], d[p + 6], d[p + 7]];
            if length > MAX_LEN {
                m.halt(Error::BadChunkLength { length }, p);
                break;
            }
            let total = length as u64 + 12;
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
            let total = total as usize;
            if m.chunks.len() >= MAX_PNG_CHUNKS {
                m.halt(
                    Error::TooManyChunks {
                        limit: MAX_PNG_CHUNKS,
                    },
                    p,
                );
                break;
            }
            let data = &d[p + 8..p + 8 + length as usize];
            let stored = be32(d, p + total - 4);
            let mut crc_in = ty.to_vec();
            crc_in.extend_from_slice(data);
            let computed = crc32_bitwise(&crc_in);
            let kind = kind_of(&ty);
            if ty.iter().any(|b| !b.is_ascii_alphabetic()) {
                m.record(Error::BadChunkType { chunk_type: ty }, p);
            }
            if m.chunks.is_empty() && kind != ChunkKind::Ihdr {
                m.record(Error::MissingIhdr, p);
            }
            if kind == ChunkKind::Ihdr && length != 13 {
                m.record(Error::BadIhdr { length }, p);
            }
            if computed != stored {
                m.record(Error::BadChunkCrc { stored, computed }, p);
            }
            if m.halted {
                break;
            }
            m.chunks.push(Chunk {
                offset: p,
                ty,
                data_len: length as usize,
                stored,
                crc_ok: computed == stored,
                kind,
                after_idat: seen_idat,
            });
            if kind == ChunkKind::Idat {
                seen_idat = true;
            }
            if kind == ChunkKind::Iend {
                m.iend = Some(p);
                m.trailer_start = p + total;
                break;
            }
            p += total;
        }
        if !m.halted && m.iend.is_none() {
            if m.chunks.is_empty() {
                m.record(Error::MissingIhdr, len);
            }
            m.record(Error::MissingIend, len);
        }
        m
    }

    fn inside(outer: &[u8], inner: &[u8]) -> bool {
        let o = outer.as_ptr() as usize;
        let i = inner.as_ptr() as usize;
        i >= o && i + inner.len() <= o + outer.len()
    }

    /// The harness body: neither entry point panics, and the walk equals the
    /// independent model.
    pub(super) fn check(data: &[u8]) {
        let strict = Png::parse_strict(data);
        let png = match Png::parse(data) {
            Ok(p) => p,
            Err(e) => {
                // Fatal exactly when not a PNG; both entry points agree.
                assert!(data.len() < 8 || data[..8] != SIG, "fatal on a PNG");
                if data.len() < 8 {
                    assert_eq!(e, Error::Truncated { at: 0, len: 8 });
                } else {
                    let mut mark = [0u8; 8];
                    mark.copy_from_slice(&data[..8]);
                    assert_eq!(e, Error::NotPng { mark });
                }
                assert_eq!(strict.err(), Some(e), "strict disagrees on a fatal input");
                return;
            }
        };
        assert_eq!(data[..8], SIG, "a non-PNG was accepted");

        // parse_strict is Ok iff clean; Err is the first issue's error.
        assert_eq!(strict.is_ok(), png.is_clean(), "strict iff clean");
        assert_eq!(png.is_clean(), png.issues().is_empty());
        if let Err(e) = strict {
            assert_eq!(Some(&e), png.issues().first().map(|i| i.error()));
        }

        let m = model(data);

        // Issues equal the model's, in order, with offsets.
        let got: Vec<(Error, usize)> = png
            .issues()
            .iter()
            .map(|i| (i.error().clone(), i.offset()))
            .collect();
        assert_eq!(got, m.issues, "issue list differs from the model");
        assert!(png.issues().len() <= MAX_PNG_ISSUES + 1);
        assert!(png.chunks().len() <= MAX_PNG_CHUNKS);

        // Chunks equal the model's, field by field.
        assert_eq!(png.chunks().len(), m.chunks.len(), "chunk count");
        let mut prev_end = 8usize;
        for (c, e) in png.chunks().iter().zip(&m.chunks) {
            assert_eq!(c.offset(), e.offset);
            assert_eq!(c.chunk_type(), e.ty);
            assert_eq!(c.data().len(), e.data_len);
            assert_eq!(c.data(), &data[e.offset + 8..e.offset + 8 + e.data_len]);
            assert!(inside(data, c.data()), "payload not borrowed");
            assert_eq!(c.stored_crc(), e.stored);
            assert_eq!(c.crc_ok(), e.crc_ok, "crc_ok");
            assert_eq!(c.kind(), e.kind);
            assert_eq!(c.after_idat(), e.after_idat);
            assert_eq!(c.range(), e.offset..e.offset + 12 + e.data_len);
            // Strictly increasing, non-overlapping, framed in the input.
            assert!(c.offset() >= prev_end && c.range().end <= data.len());
            prev_end = c.range().end;
            // A bad CRC always has its issue (the cap's terminal issue aside).
            if !c.crc_ok() {
                assert!(
                    png.issues().iter().any(|i| i.offset() == c.offset()
                        && matches!(i.error(), Error::BadChunkCrc { .. })),
                    "bad crc without an issue"
                );
            }
        }

        // IEND and the trailer.
        assert_eq!(png.iend_offset(), m.iend);
        assert_eq!(png.trailer(), &data[m.trailer_start..]);
        assert!(inside(data, png.trailer()), "trailer not borrowed");
        if m.iend.is_none() {
            assert!(png.trailer().is_empty());
        }
    }
}
