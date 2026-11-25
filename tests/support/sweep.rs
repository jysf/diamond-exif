//! The helpers shared by the two completeness sweeps (SPEC-038 for WebP,
//! SPEC-039 for PNG). STUB at the red commit: the parsers return nothing, so
//! the PNG sweep tests are red until the helpers move here from
//! `strip_webp_sweep_live.rs` (SPEC-039 AC8).

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::tools;

const REQUIRE_VAR: &str = "DIAMOND_EXIF_REQUIRE_ORACLE";

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

pub fn listx_ids(_xml: &str) -> BTreeSet<[u8; 4]> {
    BTreeSet::new()
}

pub fn parse_listing(_stdout: &str) -> BTreeMap<String, BTreeSet<String>> {
    BTreeMap::new()
}

pub fn new_tags(_listing: &BTreeSet<String>, _bare: &BTreeSet<String>) -> Vec<String> {
    Vec::new()
}

pub fn unexplained(
    _code: &[u8; 4],
    _tags: &[String],
    _known: &BTreeSet<[u8; 4]>,
    _structure: &[([u8; 4], &[&str])],
) -> Option<String> {
    None
}

/// A scratch directory removed on drop.
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
