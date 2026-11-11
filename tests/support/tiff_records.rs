//! ExifTool records and the comparator for the TIFF and DNG strip (SPEC-022).
//!
//! `record_text` runs `exiftool -a -u -G1 -s -t -n` on a fixture and on the
//! library's output under each recorded policy that strips it. `check` is the
//! comparator: for a policy's output section against the input section, every
//! ExifTool tag of a removed category is gone, every structural tag (strips,
//! tiles, compression, DNG core) has the same values, and every kept tag keeps
//! its value. The table of tag names per category is typed here by hand from
//! the DEC-008 table and ExifTool 13.55's names; nothing is read from ExifTool's
//! source. `#[path]`-included by the test and by `examples/oracle_record.rs`
//! (which also includes `tools.rs` and `tiff_shapes.rs`).

#![allow(dead_code)]

use std::path::Path;

use crate::tiff_shapes::Shape;
use crate::tools;
use exif_core::audit::Category;
use exif_core::policy::{StripPolicy, StripSpec};

pub const ARGS: [&str; 6] = ["-a", "-u", "-G1", "-s", "-t", "-n"];

pub fn command_line() -> String {
    format!("exiftool {} <file>", ARGS.join(" "))
}

/// The recorded policies.
pub fn policies() -> Vec<(&'static str, StripPolicy)> {
    vec![
        ("gps", StripPolicy::Gps),
        ("gps_and_device", StripPolicy::GpsAndDevice),
        ("all_pii", StripPolicy::AllPii),
        ("everything", StripPolicy::Everything),
        (
            "software",
            StripPolicy::Custom(StripSpec::none().include(Category::Software)),
        ),
    ]
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub group: String,
    pub tag: String,
    pub value: String,
}

#[derive(Clone, Debug)]
pub struct Section {
    pub name: String,
    pub lines: Vec<Line>,
}

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

pub fn run_exiftool(label: &str, bytes: &[u8]) -> Result<(Vec<Line>, Vec<String>), String> {
    let path = tools::temp_file_with_ext(label, "tif", bytes).map_err(|e| e.to_string())?;
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

pub fn parse_record(text: &str) -> Vec<Section> {
    let mut out: Vec<Section> = Vec::new();
    for l in text.lines() {
        if let Some(name) = l.strip_prefix("## ") {
            out.push(Section {
                name: name.to_string(),
                lines: Vec::new(),
            });
        } else if l.starts_with('#') {
            continue;
        } else if let Some(s) = out.last_mut() {
            s.lines.extend(parse_stdout(l));
        }
    }
    out
}

/// ExifTool tag names per category (the carriers of the fixtures), by hand.
fn names(c: Category) -> &'static [&'static str] {
    match c {
        Category::FreeText => &["ImageDescription", "UserComment"],
        Category::CameraModel => &["Make", "Model"],
        Category::Software => &["Software"],
        Category::Timestamps => &["ModifyDate", "DateTimeOriginal"],
        Category::Authorship => &["Artist"],
        Category::DeviceId => &["SerialNumber", "BodySerialNumber", "CameraSerialNumber"],
        Category::MakerNote => &["MakerNote"],
        Category::Thumbnail => &["ThumbnailOffset", "ThumbnailLength", "ThumbnailImage"],
        _ => &[],
    }
}

/// Tags whose values the strip must not change, in any group.
const STRUCTURAL: [&str; 11] = [
    "StripOffsets",
    "StripByteCounts",
    "TileOffsets",
    "TileByteCounts",
    "TileWidth",
    "TileLength",
    "Compression",
    "DNGVersion",
    "ColorMatrix1",
    "Orientation",
    "SubIFD",
];

fn tag_set(s: &Section, f: impl Fn(&Line) -> bool) -> Vec<(String, String, String)> {
    let mut v: Vec<_> = s
        .lines
        .iter()
        .filter(|l| f(l))
        .map(|l| (l.group.clone(), l.tag.clone(), l.value.clone()))
        .collect();
    v.sort();
    v
}

/// Compare the output section with the input section for the categories `spec`.
pub fn check(spec: StripSpec, out: &Section, input: &Section) -> Result<(), String> {
    for cat in Category::ALL {
        for l in &out.lines {
            let gone = if cat == Category::Gps {
                (l.group == "GPS" || l.tag.starts_with("GPS"))
            } else {
                names(cat).contains(&l.tag.as_str())
            };
            if gone && spec.contains(cat) {
                return Err(format!("{}:{} survives {cat:?}", l.group, l.tag));
            }
        }
    }
    for tag in STRUCTURAL {
        // A removed IFD1 thumbnail takes its own Compression with it.
        let skip = |l: &Line| spec.contains(Category::Thumbnail) && l.group == "IFD1";
        let (a, b) = (
            tag_set(input, |l| l.tag == tag && !skip(l)),
            tag_set(out, |l| l.tag == tag && !skip(l)),
        );
        if a != b {
            return Err(format!("structural tag {tag} changed: {a:?} -> {b:?}"));
        }
    }
    // A kept category's tags keep their values.
    for cat in Category::ALL {
        if spec.contains(cat) || cat == Category::Gps && spec.contains(cat) {
            continue;
        }
        let pick = |l: &Line| {
            if cat == Category::Gps {
                (l.group == "GPS" || l.tag.starts_with("GPS"))
            } else {
                names(cat).contains(&l.tag.as_str())
            }
        };
        if tag_set(input, pick) != tag_set(out, pick) {
            return Err(format!("kept category {cat:?} changed"));
        }
    }
    Ok(())
}
