//! The corpora of the inventory parity check (SPEC-013): which inputs, under
//! which names, in which committed record. Shared by the recorder
//! (`examples/oracle_record.rs`) and by `tests/inventory_oracle.rs`.
//!
//! Five corpora, in this order (it is also the order in which a byte-identical
//! input takes its canonical name):
//!
//! * `fixtures`: the 12 TIFF and 42 JPEG hand-built oracle fixtures plus the
//!   two TIFF and five JPEG shapes added by SPEC-013;
//! * `audit-tags`: the one-entry TIFFs of `audit_tags` (one per DEC-008 id);
//! * `seed-tiff_ifd`, `seed-jpeg_segments`, `seed-audit`: the committed fuzz
//!   seeds (`seed-audit` copies most of the other two under a prefix).
//!
//! An input's key is `<corpus>/<name>`. Needs `oracle_tiff`, `oracle_jpeg` and
//! `audit_tags` at the including crate's root. std only.

#![allow(dead_code)]

use crate::{
    audit_tags, oracle_jpeg, oracle_png, oracle_tiff, oracle_webp, png_shapes, real_shapes,
    webp_shapes,
};
use std::path::{Path, PathBuf};

/// The corpora, in canonical-name order.
pub const CORPORA: [&str; 5] = [
    "fixtures",
    "audit-tags",
    "seed-tiff_ifd",
    "seed-jpeg_segments",
    "seed-audit",
];

/// Corpora added after SPEC-013. They are NOT in [`CORPORA`], so the 178
/// existing results, their records and their counts cannot move; they have
/// their own records and their own tests (SPEC-019).
///
/// `png-webp` (SPEC-036): every PNG and WebP oracle fixture, shape and committed
/// seed (see [`png_webp_inputs`]), with the extension rule of [`Input::ext`].
pub const NEW_CORPORA: [&str; 2] = ["real-shapes", "png-webp"];

/// The name of the PNG/WebP corpus.
pub const PNG_WEBP: &str = "png-webp";

/// The prefix of the SPEC-019 seeds. They are byte copies of the `real-shapes`
/// fixtures, which carry the record, so the three seed corpora skip them (their
/// records stay byte-identical).
pub const SHAPE_SEED_PREFIX: &str = "shape-";

/// The directory of the committed inventory records, from the repo root.
pub const RECORD_DIR: &str = "tests/oracle-fixtures/exiftool-inventory";

/// The record file of a corpus.
pub fn record_file(corpus: &str) -> String {
    format!("{corpus}.txt")
}

/// The keys of the inputs ExifTool is never run on: it does not return and
/// grows without bound (`exiftool-two-hop-cycle-does-not-return`). Every input
/// with the same bytes is not run too.
pub const NOT_RUN_KEYS: [&str; 1] = ["seed-tiff_ifd/two-hop-cycle"];

/// The `# outcome:` text of a not-run input.
pub const NOT_RUN_OUTCOME: &str = "not run: exiftool 13.55 does not return on this input \
     (unbounded memory growth, about 9 GB in 5 s); see \
     exiftool-two-hop-cycle-does-not-return";

/// One input of a corpus.
#[derive(Clone)]
pub struct Input {
    pub corpus: &'static str,
    pub name: String,
    pub bytes: Vec<u8>,
}

impl Input {
    /// `<corpus>/<name>`.
    pub fn key(&self) -> String {
        format!("{}/{}", self.corpus, self.name)
    }

    /// The extension of the temporary file ExifTool is run on: `jpg` for
    /// anything that starts `FF D8`, `tif` otherwise. The same rule is used by
    /// the recorder and by the live tests.
    ///
    /// The `png-webp` corpus (SPEC-036) uses `png` for bytes that start with the
    /// PNG signature, `webp` for bytes that start `RIFF`, and otherwise by the
    /// input's name (`webp` if it contains `webp`, else `png`): a corrupted
    /// signature is still a file of the format it was made from.
    pub fn ext(&self) -> &'static str {
        if self.corpus == PNG_WEBP {
            if self.bytes.starts_with(b"\x89PNG") {
                return "png";
            }
            if self.bytes.starts_with(b"RIFF") || self.name.contains("webp") {
                return "webp";
            }
            return "png";
        }
        if self.bytes.starts_with(&[0xFF, 0xD8]) {
            "jpg"
        } else {
            "tif"
        }
    }

    /// Whether the bytes are a JPEG (starts `FF D8`).
    pub fn is_jpeg(&self) -> bool {
        self.bytes.starts_with(&[0xFF, 0xD8])
    }
}

fn read_seeds(root: &Path, corpus: &'static str, dir: &str) -> Vec<Input> {
    let base = root.join("fuzz/seeds").join(dir);
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&base)
        .unwrap_or_else(|e| panic!("read {}: {e}", base.display()))
        .map(|e| e.expect("dir entry").path())
        .collect();
    paths.sort();
    paths
        .into_iter()
        .filter(|p| {
            !p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with(SHAPE_SEED_PREFIX))
        })
        // SPEC-032: the `audit` target also holds PNG and WebP seeds. Tag-level
        // inventory parity for those two formats is SPEC-036, so they stay out
        // of this corpus; `audit-png-magic` (a bare signature, no item) was
        // already in it and stays, so the pinned records do not move.
        .filter(|p| {
            let name = p.file_name().map(|n| n.to_string_lossy().into_owned());
            if name.as_deref() == Some("audit-png-magic") {
                return true;
            }
            // The copies of the walker seeds (some are not even a PNG or a WebP).
            if dir == "audit"
                && name
                    .as_deref()
                    .is_some_and(|n| n.starts_with("png-") || n.starts_with("webp-"))
            {
                return false;
            }
            let Ok(bytes) = std::fs::read(p) else {
                return true;
            };
            let png = bytes.starts_with(b"\x89PNG\r\n\x1a\n");
            let webp = bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP";
            !(png || webp) || dir != "audit"
        })
        .map(|p| Input {
            corpus,
            name: p
                .file_name()
                .expect("file name")
                .to_string_lossy()
                .into_owned(),
            bytes: std::fs::read(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display())),
        })
        .collect()
}

/// Whether the `seed-audit` corpus left this `audit` seed out (SPEC-032): a PNG
/// or WebP, except `audit-png-magic`. The `png-webp` corpus takes exactly these.
fn audit_seed_is_png_or_webp(name: &str, bytes: &[u8]) -> bool {
    if name == "audit-png-magic" {
        return false;
    }
    if name.starts_with("png-") || name.starts_with("webp-") {
        return true;
    }
    let png = bytes.starts_with(b"\x89PNG\r\n\x1a\n");
    let webp = bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP";
    png || webp
}

fn dir_files(root: &Path, dir: &str) -> Vec<(String, Vec<u8>)> {
    let base = root.join("fuzz/seeds").join(dir);
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&base)
        .unwrap_or_else(|e| panic!("read {}: {e}", base.display()))
        .map(|e| e.expect("dir entry").path())
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|p| {
            let name = p
                .file_name()
                .expect("file name")
                .to_string_lossy()
                .into_owned();
            let bytes = std::fs::read(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()));
            (name, bytes)
        })
        .collect()
}

/// The raw-profile keyword of the extras below (ImageMagick's spelling).
const RAW_EXIF_KEYWORD: &[u8] = b"Raw profile type exif";

/// Hostile shapes the oracle fixtures, shapes and seeds do not hold (SPEC-036
/// Part B, the variant grid): a `tIME` chunk of every length 0 to 12 (the valid
/// one is 7), and a raw-profile EXIF with GPS entries in a `tEXt`, a `zTXt` and
/// an `iTXt` chunk. They belong to this corpus only, so no existing record,
/// fixture list or count of another oracle moves.
pub fn png_webp_extras() -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let time: [u8; 12] = [0x07, 0xE8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    for len in 0..=12usize {
        // Longer than the 12 bytes of the prefix: pad with zeros.
        let mut data = time[..len.min(12)].to_vec();
        data.resize(len, 0);
        out.push((
            format!("png-extra-time-len-{len}"),
            oracle_png::png(&[oracle_png::chunk(b"tIME", &data)], &[], &[]),
        ));
    }
    let gps = oracle_tiff::fixtures()
        .into_iter()
        .find(|(n, _)| *n == "exif-gps-full-le")
        .map(|(_, b)| b)
        .expect("the exif-gps-full-le TIFF fixture");
    let data = [&b"Exif\0\0"[..], &gps].concat();
    let body = oracle_png::raw_profile_body("exif", &data);
    out.push((
        "png-extra-raw-exif-gps-text".to_string(),
        oracle_png::png(&[oracle_png::text(RAW_EXIF_KEYWORD, &body)], &[], &[]),
    ));
    out.push((
        "png-extra-raw-exif-gps-ztxt".to_string(),
        oracle_png::png(&[oracle_png::ztxt(RAW_EXIF_KEYWORD, &body)], &[], &[]),
    ));
    out.push((
        "png-extra-raw-exif-gps-itxt".to_string(),
        oracle_png::png(&[oracle_png::itxt(RAW_EXIF_KEYWORD, 1, &body)], &[], &[]),
    ));
    // SPEC-036 fix round (H1): the keyword spellings ExifTool 13.55 decodes
    // (`Raw profile type` and all-lower-case `raw profile type`, any text
    // chunk) and one it does not (all caps), each with GPS.
    let lower: &[u8] = b"raw profile type exif";
    out.push((
        "png-extra-raw-lower-exif-gps-text".to_string(),
        oracle_png::png(&[oracle_png::text(lower, &body)], &[], &[]),
    ));
    out.push((
        "png-extra-raw-lower-exif-gps-ztxt".to_string(),
        oracle_png::png(&[oracle_png::ztxt(lower, &body)], &[], &[]),
    ));
    out.push((
        "png-extra-raw-lower-exif-gps-itxt".to_string(),
        oracle_png::png(&[oracle_png::itxt(lower, 1, &body)], &[], &[]),
    ));
    let app1_body = oracle_png::raw_profile_body("APP1", &data);
    out.push((
        "png-extra-raw-lower-app1-gps-text".to_string(),
        oracle_png::png(
            &[oracle_png::text(b"raw profile type APP1", &app1_body)],
            &[],
            &[],
        ),
    ));
    out.push((
        "png-extra-raw-allcaps-exif-gps-text".to_string(),
        oracle_png::png(
            &[oracle_png::text(b"RAW PROFILE TYPE EXIF", &body)],
            &[],
            &[],
        ),
    ));
    out
}

/// The `png-webp` inputs (SPEC-036), in this order: the PNG oracle fixtures and
/// the `png-text-keywords` table; the WebP oracle fixtures; the PNG then the
/// WebP shapes; the committed `png_chunks` then `webp_riff` seeds; the PNG and
/// WebP seeds of `audit` that the old corpora leave out. Names: the fixtures
/// keep theirs, the rest are prefixed by their source so no two differ only by
/// directory.
pub fn png_webp_inputs(root: &Path) -> Vec<Input> {
    let mk = |name: String, bytes: Vec<u8>| Input {
        corpus: PNG_WEBP,
        name,
        bytes,
    };
    let mut out = Vec::new();
    for (name, bytes) in oracle_png::fixtures() {
        out.push(mk(name, bytes));
    }
    out.push(mk(
        oracle_png::TEXT_KEYWORDS_NAME.to_string(),
        oracle_png::text_keywords_png(),
    ));
    for (name, bytes) in oracle_webp::fixtures() {
        out.push(mk(name, bytes));
    }
    for (name, bytes) in png_shapes::seeds() {
        out.push(mk(format!("png-shape-{name}"), bytes));
    }
    for (name, bytes) in webp_shapes::seeds() {
        out.push(mk(format!("webp-shape-{name}"), bytes));
    }
    for (name, bytes) in dir_files(root, "png_chunks") {
        out.push(mk(format!("png_chunks-{name}"), bytes));
    }
    for (name, bytes) in dir_files(root, "webp_riff") {
        out.push(mk(format!("webp_riff-{name}"), bytes));
    }
    for (name, bytes) in dir_files(root, "audit") {
        if audit_seed_is_png_or_webp(&name, &bytes) {
            out.push(mk(format!("audit-{name}"), bytes));
        }
    }
    // Last, so the order of every input above is unchanged.
    for (name, bytes) in png_webp_extras() {
        out.push(mk(name, bytes));
    }
    out
}

/// The inputs of one corpus, in a fixed order.
pub fn inputs(root: &Path, corpus: &str) -> Vec<Input> {
    match corpus {
        "fixtures" => {
            let mut out = Vec::new();
            for (name, bytes) in oracle_tiff::fixtures() {
                out.push(Input {
                    corpus: "fixtures",
                    name: name.to_string(),
                    bytes,
                });
            }
            for (name, bytes) in oracle_tiff::inventory_fixtures() {
                out.push(Input {
                    corpus: "fixtures",
                    name: name.to_string(),
                    bytes,
                });
            }
            for (name, bytes) in oracle_jpeg::fixtures() {
                out.push(Input {
                    corpus: "fixtures",
                    name,
                    bytes,
                });
            }
            for (name, bytes) in oracle_jpeg::inventory_fixtures() {
                out.push(Input {
                    corpus: "fixtures",
                    name,
                    bytes,
                });
            }
            out
        }
        "audit-tags" => audit_tags::rows()
            .iter()
            .map(|row| Input {
                corpus: "audit-tags",
                name: format!("tag-{:04x}", row.id),
                bytes: audit_tags::fixture(row),
            })
            .collect(),
        "seed-tiff_ifd" => read_seeds(root, "seed-tiff_ifd", "tiff_ifd"),
        "seed-jpeg_segments" => read_seeds(root, "seed-jpeg_segments", "jpeg_segments"),
        "seed-audit" => read_seeds(root, "seed-audit", "audit"),
        "real-shapes" => real_shapes::fixtures()
            .into_iter()
            .map(|(name, bytes)| Input {
                corpus: "real-shapes",
                name: name.to_string(),
                bytes,
            })
            .collect(),
        "png-webp" => png_webp_inputs(root),
        other => panic!("unknown corpus {other}"),
    }
}

/// Every input of every corpus, corpus by corpus.
pub fn all_inputs(root: &Path) -> Vec<Input> {
    CORPORA.iter().flat_map(|c| inputs(root, c)).collect()
}

/// One distinct input (by content) and every name it carries.
pub struct Distinct {
    /// The canonical (first) input.
    pub canonical: Input,
    /// Every key with these bytes, the canonical one first.
    pub keys: Vec<String>,
}

/// The distinct inputs of `all`, each once, in order of first appearance.
pub fn distinct(all: &[Input]) -> Vec<Distinct> {
    let mut out: Vec<Distinct> = Vec::new();
    for input in all {
        match out.iter_mut().find(|d| d.canonical.bytes == input.bytes) {
            Some(d) => d.keys.push(input.key()),
            None => out.push(Distinct {
                canonical: input.clone(),
                keys: vec![input.key()],
            }),
        }
    }
    out
}

/// Whether ExifTool must not be run on this input (any of its names is in
/// [`NOT_RUN_KEYS`]).
pub fn is_not_run(d: &Distinct) -> bool {
    d.keys.iter().any(|k| NOT_RUN_KEYS.contains(&k.as_str()))
}

/// For each input of `all`, the canonical key of its distinct group.
pub fn canonical_key_of(all: &[Input], input: &Input) -> String {
    all.iter()
        .find(|i| i.bytes == input.bytes)
        .map(Input::key)
        .unwrap_or_else(|| input.key())
}
