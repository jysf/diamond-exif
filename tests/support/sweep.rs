//! The helpers shared by the two completeness sweeps: the WebP one (SPEC-038,
//! `tests/strip_webp_sweep_live.rs`) and the PNG one (SPEC-039 AC8,
//! `tests/strip_png_sweep_live.rs`). Moved here from the WebP sweep with its
//! verdict unchanged; `unexplained` takes the known set and the pinned
//! structure list as arguments, so each container brings its own.
//!
//! Test support only. `#[path]`-included after `tools.rs`. ExifTool is run,
//! never read or linked.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::tools;

const REQUIRE_VAR: &str = "DIAMOND_EXIF_REQUIRE_ORACLE";

/// `true` when ExifTool is on `PATH`. Without it the test prints `SKIP` and
/// passes, unless `DIAMOND_EXIF_REQUIRE_ORACLE=1` makes that a failure.
pub fn live_gate(test: &str) -> bool {
    if tools::exiftool_available() {
        return true;
    }
    if std::env::var(REQUIRE_VAR).as_deref() == Ok("1") {
        panic!("{test}: exiftool is not on PATH and {REQUIRE_VAR}=1 makes that a failure");
    }
    println!("SKIP {test} \u{2014} exiftool not on PATH");
    false
}

/// The four-character ids of an `exiftool -listx` document (`<tag id='...'`),
/// unescaped, printable ASCII only.
pub fn listx_ids(xml: &str) -> BTreeSet<[u8; 4]> {
    let mut out = BTreeSet::new();
    let mut rest = xml;
    while let Some(at) = rest.find("<tag id='") {
        rest = &rest[at + "<tag id='".len()..];
        let Some(end) = rest.find('\'') else {
            break;
        };
        let raw = &rest[..end];
        let id = raw
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'");
        let b = id.as_bytes();
        if let Ok(code) = <[u8; 4]>::try_from(b) {
            if code.iter().all(|c| (0x20..0x7F).contains(c)) {
                out.insert(code);
            }
        }
    }
    out
}

/// `Group:Name` keys of an `exiftool -a -u -G1 -s -r` listing, by file name
/// (the part after the last `/` of each `======== path` header).
pub fn parse_listing(stdout: &str) -> BTreeMap<String, BTreeSet<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut current: Option<String> = None;
    for line in stdout.lines() {
        if let Some(path) = line.strip_prefix("======== ") {
            let name = path.rsplit('/').next().unwrap_or(path).to_string();
            out.entry(name.clone()).or_default();
            current = Some(name);
            continue;
        }
        let Some(name) = &current else {
            continue;
        };
        let Some(rest) = line.strip_prefix('[') else {
            continue;
        };
        let Some((group, rest)) = rest.split_once(']') else {
            continue;
        };
        let Some((left, _)) = rest.split_once(" : ").or_else(|| rest.split_once(':')) else {
            continue;
        };
        let Some(tag) = left.split_whitespace().last() else {
            continue;
        };
        out.entry(name.clone())
            .or_default()
            .insert(format!("{group}:{tag}"));
    }
    out
}

/// The tags a file prints that the bare file does not, less the structure ones
/// (`Unknown_*` dumps, the `File`, `System` and `ExifTool` groups).
pub fn new_tags(listing: &BTreeSet<String>, bare: &BTreeSet<String>) -> Vec<String> {
    listing
        .difference(bare)
        .filter(|k| {
            let (group, name) = k.split_once(':').unwrap_or(("", k));
            !name.starts_with("Unknown_") && !matches!(group, "File" | "System" | "ExifTool")
        })
        .cloned()
        .collect()
}

/// Why `tags` (printed from one `code`) is not explained, or `None`: the code
/// is in `known` (the strip's own set), or in `structure` with exactly the tag
/// names pinned there. `structure` is each sweep's own pinned list.
pub fn unexplained(
    code: &[u8; 4],
    tags: &[String],
    known: &BTreeSet<[u8; 4]>,
    structure: &[([u8; 4], &[&str])],
) -> Option<String> {
    if tags.is_empty() || known.contains(code) {
        return None;
    }
    let pinned = structure.iter().find(|s| &s.0 == code).map(|s| s.1);
    let extra: Vec<&String> = tags
        .iter()
        .filter(|k| {
            let name = k.split_once(':').map_or(k.as_str(), |(_, n)| n);
            !pinned.is_some_and(|p| p.contains(&name))
        })
        .collect();
    (!extra.is_empty()).then(|| format!("{extra:?}"))
}

/// A scratch directory removed on drop. `label` keeps two sweeps (or two tests)
/// of one process apart.
pub struct Scratch(pub PathBuf);

impl Scratch {
    pub fn new(label: &str) -> Scratch {
        let dir =
            std::env::temp_dir().join(format!("diamond-exif-sweep-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        Scratch(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub fn exiftool_listx() -> String {
    let out = std::process::Command::new("exiftool")
        .arg("-listx")
        .stdin(std::process::Stdio::null())
        .output()
        .expect("exiftool -listx");
    String::from_utf8_lossy(&out.stdout).into_owned()
}
