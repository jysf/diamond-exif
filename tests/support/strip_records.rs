//! The ExifTool side of the strip oracle (SPEC-020): records of
//! `exiftool -a -u -G1 -s -t -n` over the library's stripped outputs, their
//! parsing, and the comparator that checks them against a hand-typed table of
//! which ExifTool groups each kind of kept byte range produces.
//!
//! Shared by `tests/strip_oracle.rs` and `examples/oracle_record.rs`. ExifTool
//! is run, never read or linked. The group names below are facts measured with
//! ExifTool 13.55 on the fixtures of `strip_shapes.rs` (see the SPEC-020
//! handback), not assumed.

#![allow(dead_code)]

use std::collections::BTreeSet;
use std::path::Path;

use exif_core::audit::Category;
use exif_core::policy::{StripPolicy, StripSpec};

use crate::strip_shapes::{model, spec_of, Expect, Shape, K};
use crate::tools;

/// The policies recorded for every fixture (the spec's list).
pub fn policies() -> Vec<(&'static str, StripPolicy)> {
    vec![
        ("all", StripPolicy::All),
        ("everything", StripPolicy::Everything),
        ("xmp", StripPolicy::Custom(spec_of(&[Category::Xmp]))),
        ("icc", StripPolicy::Custom(spec_of(&[Category::Icc]))),
        (
            "embedded",
            StripPolicy::Custom(spec_of(&[Category::EmbeddedImage])),
        ),
        (
            "other",
            StripPolicy::Custom(spec_of(&[Category::OtherSegment])),
        ),
        (
            "iptc+free_text",
            StripPolicy::Custom(spec_of(&[Category::Iptc, Category::FreeText])),
        ),
    ]
}

pub const ARGS: [&str; 6] = ["-a", "-u", "-G1", "-s", "-t", "-n"];

pub fn command_line() -> String {
    format!("exiftool {} <file>", ARGS.join(" "))
}

/// Groups that describe the file or are derived, not carried by a segment.
const FILE_LEVEL: [&str; 4] = ["System", "ExifTool", "File", "Composite"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub group: String,
    pub tag: String,
    pub value: String,
}

#[derive(Clone, Debug)]
pub struct Section {
    pub name: String,
    pub stderr: Vec<String>,
    pub lines: Vec<Line>,
}

#[derive(Clone, Debug)]
pub struct Record {
    pub version: String,
    pub sections: Vec<Section>,
}

impl Record {
    pub fn section(&self, name: &str) -> Option<&Section> {
        self.sections.iter().find(|s| s.name == name)
    }
}

/// Parse ExifTool's `-t` stdout, dropping the `System` group and the version
/// line (they depend on the host and the temporary path).
pub fn parse_stdout(stdout: &str) -> Vec<Line> {
    stdout
        .lines()
        .filter_map(|l| {
            let mut it = l.splitn(3, '\t');
            let (group, tag, value) = (it.next()?, it.next()?, it.next().unwrap_or(""));
            if group == "System" || (group == "ExifTool" && tag == "ExifToolVersion") {
                return None;
            }
            Some(Line {
                group: group.to_string(),
                tag: tag.to_string(),
                value: value.to_string(),
            })
        })
        .collect()
}

/// Run ExifTool on `bytes` (a temporary file, removed afterwards).
pub fn run_exiftool(label: &str, bytes: &[u8]) -> Result<(Vec<Line>, Vec<String>), String> {
    let path = tools::temp_file_with_ext(label, "jpg", bytes).map_err(|e| e.to_string())?;
    let args: Vec<String> = ARGS.iter().map(|s| s.to_string()).collect();
    let run = tools::run_tool("exiftool", &args, &path);
    let _ = std::fs::remove_file(&path);
    let out = run.map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    Ok((
        parse_stdout(&stdout),
        tools::warning_lines(&stderr, Path::new(&path)),
    ))
}

/// The record text of one fixture: the input, then the output of every
/// recorded policy for which the library strips.
pub fn record_text(shape: &Shape, version: &str) -> Result<String, String> {
    let mut out = format!(
        "# command: {}\n# exiftool-version: {version}\n# fixture: {}\n",
        command_line(),
        shape.name
    );
    let mut add = |name: &str, bytes: &[u8]| -> Result<(), String> {
        let (lines, stderr) = run_exiftool(&format!("{}-{name}", shape.name), bytes)?;
        out.push_str(&format!("## {name}\n"));
        for w in stderr {
            out.push_str(&format!("# stderr: {w}\n"));
        }
        for l in lines {
            out.push_str(&format!("{}\t{}\t{}\n", l.group, l.tag, l.value));
        }
        Ok(())
    };
    add("input", &shape.bytes)?;
    for (name, policy) in policies() {
        if let Ok(stripped) = exif_core::strip(&shape.bytes, policy) {
            add(name, stripped.bytes())?;
        }
    }
    Ok(out)
}

pub fn parse_record(text: &str) -> Result<Record, String> {
    let mut version = None;
    let mut sections: Vec<Section> = Vec::new();
    for l in text.lines() {
        if let Some(v) = l.strip_prefix("# exiftool-version: ") {
            version = Some(v.to_string());
        } else if let Some(name) = l.strip_prefix("## ") {
            sections.push(Section {
                name: name.to_string(),
                stderr: Vec::new(),
                lines: Vec::new(),
            });
        } else if let Some(w) = l.strip_prefix("# stderr: ") {
            sections
                .last_mut()
                .ok_or("stderr line before any section")?
                .stderr
                .push(w.to_string());
        } else if l.starts_with('#') || l.is_empty() {
            continue;
        } else {
            let mut it = l.splitn(3, '\t');
            let (g, t, v) = (it.next(), it.next(), it.next().unwrap_or(""));
            let (Some(g), Some(t)) = (g, t) else {
                return Err(format!("bad record line: {l}"));
            };
            sections
                .last_mut()
                .ok_or("line before any section")?
                .lines
                .push(Line {
                    group: g.to_string(),
                    tag: t.to_string(),
                    value: v.to_string(),
                });
        }
    }
    Ok(Record {
        version: version.ok_or("no version header")?,
        sections,
    })
}

// ── the hand-typed group table ──────────────────────────────────────────────

/// The Exif groups ExifTool 13.55 reports for the Exif segment of a fixture
/// (measured): the full block has four IFDs, the camera block of the Multi-
/// Picture files two, the small ones one, a garbage block none.
fn exif_groups(shape: &Shape) -> &'static [&'static str] {
    match shape.name {
        "kitchen-sink" => &["IFD0", "ExifIFD", "GPS", "IFD1"],
        "unreadable-exif" => &[],
        n if n.starts_with("mpo-") => &["IFD0", "ExifIFD"],
        _ => &["IFD0"],
    }
}

/// The ExifTool groups the kept (not removed) bytes of `shape` must produce,
/// and whether `File:Comment` must be present.
pub fn expected_groups(
    shape: &Shape,
    removed: &[std::ops::Range<usize>],
) -> (BTreeSet<String>, bool) {
    let mut groups: BTreeSet<String> = BTreeSet::new();
    let mut comment = false;
    let mut iptc_seen = 0usize;
    for p in &shape.parts {
        let kept = !removed.iter().any(|r| r.start == p.range.start);
        if !kept {
            continue;
        }
        let mut add = |gs: &[&str]| {
            for g in gs {
                groups.insert((*g).to_string());
            }
        };
        match p.kind {
            K::Jfif => add(&["JFIF"]),
            K::Adobe => add(&["Adobe"]),
            K::Exif => add(exif_groups(shape)),
            K::Xmp => {
                // The main packet of a file with extended XMP also names it.
                if shape.parts.iter().any(|q| q.kind == K::ExtXmp) {
                    add(&["XMP-xmp", "XMP-xmpNote"]);
                } else {
                    add(&["XMP-xmp"]);
                }
            }
            K::ExtXmp => add(&["XMP-xmp"]),
            K::Icc => add(&["ICC-header", "ICC_Profile"]),
            K::Iptc => {
                add(&[if iptc_seen == 0 { "IPTC" } else { "IPTC2" }]);
                iptc_seen += 1;
            }
            K::Jumbf => add(&["JUMBF"]),
            K::Com => comment = true,
            K::OtherApp => {
                if shape
                    .bytes
                    .get(p.range.clone())
                    .is_some_and(|b| b.windows(5).any(|w| w == b"AROT\0"))
                {
                    add(&["AROT"]);
                }
            }
            K::MpfIndex => {
                add(&["MPF0"]);
                let n = shape.mpf.as_ref().map_or(0, |m| m.entries.len());
                for i in 1..=n {
                    groups.insert(format!("MPImage{i}"));
                }
            }
            _ => {}
        }
    }
    (groups, comment)
}

fn group_set(lines: &[Line]) -> BTreeSet<String> {
    lines
        .iter()
        .map(|l| l.group.clone())
        .filter(|g| !FILE_LEVEL.contains(&g.as_str()))
        .collect()
}

/// A warning with its digit runs replaced by `N`: removing a segment can merge
/// two runs of skipped bytes ("Skipped unknown 3 bytes" becomes "... 6 ..."), a
/// different count and not a different kind of warning.
pub fn normalise(w: &str) -> String {
    let mut out = String::new();
    let mut in_digits = false;
    for c in w.chars() {
        if c.is_ascii_digit() {
            if !in_digits {
                out.push('N');
            }
            in_digits = true;
        } else {
            in_digits = false;
            out.push(c);
        }
    }
    out
}

pub fn warnings(lines: &[Line], stderr: &[String]) -> BTreeSet<String> {
    lines
        .iter()
        .filter(|l| l.tag == "Warning")
        .map(|l| normalise(&l.value))
        .chain(stderr.iter().map(|w| normalise(w)))
        .collect()
}

/// The removals the model expects for `shape` under `spec` (empty if the
/// library refuses; callers only ask for policies that strip).
pub fn removed_ranges(shape: &Shape, spec: StripSpec) -> Option<Vec<std::ops::Range<usize>>> {
    match model(shape, spec) {
        Expect::Ok { removals, .. } => Some(removals.into_iter().map(|r| r.range).collect()),
        _ => None,
    }
}

/// Compare one output section with the table: the groups equal, `File:Comment`
/// present exactly when a COM is kept, and no warning the input did not have.
pub fn check_groups(
    shape: &Shape,
    spec: StripSpec,
    section: &Section,
    input: &Section,
) -> Result<(), String> {
    let removed = removed_ranges(shape, spec).ok_or("the library refuses this policy")?;
    let (want, comment) = expected_groups(shape, &removed);
    let got = group_set(&section.lines);
    if got != want {
        let extra: Vec<_> = got.difference(&want).collect();
        let missing: Vec<_> = want.difference(&got).collect();
        return Err(format!(
            "groups differ: extra {extra:?}, missing {missing:?}"
        ));
    }
    let has_comment = section
        .lines
        .iter()
        .any(|l| l.group == "File" && l.tag == "Comment");
    if has_comment != comment {
        return Err(format!(
            "File:Comment present={has_comment}, want {comment}"
        ));
    }
    let new: Vec<_> = warnings(&section.lines, &section.stderr)
        .difference(&warnings(&input.lines, &input.stderr))
        .cloned()
        .collect();
    if !new.is_empty() {
        return Err(format!("new warnings: {new:?}"));
    }
    Ok(())
}

fn value(section: &Section, group: &str, tag: &str) -> Option<i64> {
    section
        .lines
        .iter()
        .find(|l| l.group == group && l.tag == tag)
        .and_then(|l| l.value.parse().ok())
}

/// The MPF fields ExifTool reads from an output of `All` on a Multi-Picture
/// fixture equal the independently computed ones: image `i`'s start is its
/// position in the output, its length is its length, the primary is the whole
/// primary, and the image count is unchanged.
pub fn check_mpf_fields(
    shape: &Shape,
    spec: StripSpec,
    section: &Section,
    input: &Section,
) -> Result<(), String> {
    let m = shape.mpf.as_ref().ok_or("no index")?;
    let removed = removed_ranges(shape, spec).ok_or("the library refuses this policy")?;
    let mut gone = vec![false; shape.bytes.len() + 1];
    for r in &removed {
        for g in &mut gone[r.clone()] {
            *g = true;
        }
    }
    let new_pos = |p: usize| gone[..p].iter().filter(|g| !**g).count() as i64;
    let n_in = value(input, "MPF0", "NumberOfImages");
    let n_out = value(section, "MPF0", "NumberOfImages");
    if n_in != n_out {
        return Err(format!("NumberOfImages {n_in:?} became {n_out:?}"));
    }
    let groups_in = group_set(&input.lines)
        .iter()
        .filter(|g| g.starts_with("MPImage"))
        .count();
    let groups_out = group_set(&section.lines)
        .iter()
        .filter(|g| g.starts_with("MPImage"))
        .count();
    if groups_in != groups_out {
        return Err(format!("MPImage groups {groups_in} became {groups_out}"));
    }
    // Primary.
    let primary_len = new_pos(m.primary_end);
    if value(section, "MPImage1", "MPImageStart") != Some(0) {
        return Err("primary MPImageStart is not 0".into());
    }
    if value(section, "MPImage1", "MPImageLength") != Some(primary_len) {
        return Err(format!(
            "primary MPImageLength {:?}, want {primary_len}",
            value(section, "MPImage1", "MPImageLength")
        ));
    }
    // Secondaries, in index order.
    let images: Vec<_> = shape
        .parts
        .iter()
        .filter(|p| p.kind == K::Image)
        .map(|p| p.range.clone())
        .collect();
    for (i, r) in images.iter().enumerate() {
        let g = format!("MPImage{}", i + 2);
        let (start, len) = (new_pos(r.start), r.len() as i64);
        if value(section, &g, "MPImageStart") != Some(start) {
            return Err(format!(
                "{g} MPImageStart {:?}, want {start}",
                value(section, &g, "MPImageStart")
            ));
        }
        if value(section, &g, "MPImageLength") != Some(len) {
            return Err(format!("{g} MPImageLength wrong"));
        }
    }
    // The bytes at the new positions start FF D8 (checked by the caller on
    // the output, which has them).
    Ok(())
}
