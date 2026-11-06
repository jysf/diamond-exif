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

use crate::{audit_tags, oracle_jpeg, oracle_tiff};
use std::path::{Path, PathBuf};

/// The corpora, in canonical-name order.
pub const CORPORA: [&str; 5] = [
    "fixtures",
    "audit-tags",
    "seed-tiff_ifd",
    "seed-jpeg_segments",
    "seed-audit",
];

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
    pub fn ext(&self) -> &'static str {
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
