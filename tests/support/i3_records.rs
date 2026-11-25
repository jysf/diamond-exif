//! The records of the PNG and WebP write parity (I3, SPEC-039, DEC-023):
//! format, digest, the ExifTool write runner with its exit-code rule, the
//! recorded policies, and the pinned tables of what is known to differ.
//!
//! Shared by `tests/strip_png_webp_oracle.rs` and `examples/oracle_record.rs`.
//! ExifTool is run, never read or linked.
//!
//! A record is one file per fixture:
//!
//! ```text
//! # command: exiftool -a -u -G1 -s -t -n <file>
//! # writes: exiftool -all= -o <out> <file>; exiftool -gps:all= -o <out> <file>
//! # exiftool-version: 13.55
//! # fixture: <name>
//! ## input
//! <group> TAB <tag> TAB <value>      (ExifTool's reading of the input)
//! ## <policy>                        (each recorded policy that strips)
//! # output: <len> <fnv1a64>          (the bytes the reading was made from)
//! <lines>
//! ## exiftool:all                    (ExifTool's own writer, read back)
//! <lines>                            (or one `# refused: <Error: line>`)
//! ## exiftool:gps:all
//! ```
//!
//! The `File`, `System` and `ExifTool` groups are dropped (host and path
//! dependent). Machine paths never reach a record.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use exif_core::audit::Category;
use exif_core::policy::{StripPolicy, StripSpec};

use crate::i3_shapes::Fixture;
use crate::tools;

pub const ARGS: [&str; 6] = ["-a", "-u", "-G1", "-s", "-t", "-n"];

/// ExifTool's own writes, as `(section name, arguments before the output
/// option)`. `-m` is never passed (DEC-023 rule 4): a `[minor]` error must
/// stay visible.
pub const WRITE_OPS: [(&str, &str); 2] =
    [("exiftool:all", "-all="), ("exiftool:gps:all", "-gps:all=")];

pub fn command_line() -> String {
    format!("exiftool {} <file>", ARGS.join(" "))
}

pub fn writes_line() -> String {
    WRITE_OPS
        .iter()
        .map(|(_, op)| format!("exiftool {op} -o <out> <file>"))
        .collect::<Vec<_>>()
        .join("; ")
}

/// 64-bit FNV-1a, written out (no dependency, DEC-023 rule 1).
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// The `# output:` line of a strip's bytes.
pub fn output_line(bytes: &[u8]) -> String {
    format!("# output: {} {:016x}", bytes.len(), fnv1a64(bytes))
}

fn spec_of(cats: &[Category]) -> StripSpec {
    cats.iter().fold(StripSpec::none(), |s, c| s.include(*c))
}

/// The recorded policies (DEC-023 rule 2): the five presets, then one `Custom`
/// row per category that isolates a kind of chunk. The order is the order of
/// the sections.
pub fn policies() -> Vec<(&'static str, StripPolicy)> {
    let mut v: Vec<(&'static str, StripPolicy)> = StripPolicy::PRESETS
        .iter()
        .map(|p| (p.name(), *p))
        .collect();
    let custom = |cat: Category| StripPolicy::Custom(spec_of(&[cat]));
    v.push(("xmp", custom(Category::Xmp)));
    v.push(("icc", custom(Category::Icc)));
    v.push(("jumbf", custom(Category::Jumbf)));
    v.push(("rights", custom(Category::Rights)));
    v.push(("timestamps", custom(Category::Timestamps)));
    v.push(("free_text", custom(Category::FreeText)));
    v.push(("other", custom(Category::OtherSegment)));
    v
}

// ── lines, sections, records ────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Line {
    pub group: String,
    pub tag: String,
    pub value: String,
}

impl Line {
    pub fn key(&self) -> String {
        format!("{}:{}", self.group, self.tag)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Section {
    pub name: String,
    /// `# output: <len> <fnv1a64>` of a policy section.
    pub output: Option<(usize, u64)>,
    /// `# refused: <line>` of an ExifTool write that refused.
    pub refused: Option<String>,
    pub stderr: Vec<String>,
    pub lines: Vec<Line>,
}

#[derive(Clone, Debug)]
pub struct Record {
    pub version: String,
    pub fixture: String,
    pub sections: Vec<Section>,
}

impl Record {
    pub fn section(&self, name: &str) -> Option<&Section> {
        self.sections.iter().find(|s| s.name == name)
    }

    pub fn names(&self) -> Vec<&str> {
        self.sections.iter().map(|s| s.name.as_str()).collect()
    }
}

/// Groups that describe the file or the host, not what a chunk carries.
const DROPPED_GROUPS: [&str; 3] = ["File", "System", "ExifTool"];

/// Parse ExifTool's `-t` stdout, dropping the host and file groups.
pub fn parse_stdout(stdout: &str) -> Vec<Line> {
    stdout
        .lines()
        .filter_map(|l| {
            let mut it = l.splitn(3, '\t');
            let (group, tag, value) = (it.next()?, it.next()?, it.next().unwrap_or(""));
            if DROPPED_GROUPS.contains(&group) {
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

pub fn parse_record(text: &str) -> Result<Record, String> {
    let mut version = None;
    let mut fixture = None;
    let mut sections: Vec<Section> = Vec::new();
    for l in text.lines() {
        if let Some(v) = l.strip_prefix("# exiftool-version: ") {
            version = Some(v.to_string());
        } else if let Some(v) = l.strip_prefix("# fixture: ") {
            fixture = Some(v.to_string());
        } else if let Some(name) = l.strip_prefix("## ") {
            sections.push(Section {
                name: name.to_string(),
                ..Section::default()
            });
        } else if let Some(w) = l.strip_prefix("# stderr: ") {
            sections
                .last_mut()
                .ok_or("stderr line before any section")?
                .stderr
                .push(w.to_string());
        } else if let Some(o) = l.strip_prefix("# output: ") {
            let (len, digest) = o.split_once(' ').ok_or("bad output line")?;
            let len = len.parse::<usize>().map_err(|e| e.to_string())?;
            let digest = u64::from_str_radix(digest, 16).map_err(|e| e.to_string())?;
            sections
                .last_mut()
                .ok_or("output line before any section")?
                .output = Some((len, digest));
        } else if let Some(r) = l.strip_prefix("# refused: ") {
            sections
                .last_mut()
                .ok_or("refused line before any section")?
                .refused = Some(r.to_string());
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
        fixture: fixture.ok_or("no fixture header")?,
        sections,
    })
}

// ── running ExifTool ────────────────────────────────────────────────────────

/// A path that does not exist yet (an existing output path is exit 1, DEC-023
/// rule 5).
fn fresh_path(label: &str, ext: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "diamond-exif-i3-{}-{n}-{label}.{ext}",
        std::process::id()
    ))
}

/// ExifTool's reading of the file at `path`: the tag lines and the warning
/// lines of stderr (machine paths replaced).
pub fn read_path(path: &Path) -> Result<(Vec<Line>, Vec<String>), String> {
    let args: Vec<String> = ARGS.iter().map(|s| s.to_string()).collect();
    let out = tools::run_tool("exiftool", &args, path).map_err(|e| e.to_string())?;
    Ok((
        parse_stdout(&String::from_utf8_lossy(&out.stdout)),
        tools::warning_lines(&String::from_utf8_lossy(&out.stderr), path),
    ))
}

/// ExifTool's reading of `bytes`, through a temporary file.
pub fn read_bytes(
    label: &str,
    ext: &str,
    bytes: &[u8],
) -> Result<(Vec<Line>, Vec<String>), String> {
    let path = tools::temp_file_with_ext(label, ext, bytes).map_err(|e| e.to_string())?;
    let out = read_path(&path);
    let _ = std::fs::remove_file(&path);
    out
}

/// What a write run came to (DEC-023 rule 5, measured on 13.55: exit 0 exactly
/// when the output file exists).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WriteResult {
    /// Exit 0 and the file exists.
    Written(PathBuf),
    /// Exit 1 and no file: the first `Error:` line (paths replaced).
    Refused(String),
    /// Anything else. A test fails on it; it is never a skip.
    ToolError(String),
}

/// Classify one write run from its exit code, whether the output file exists
/// and stderr. The only place the rule lives.
pub fn classify_write(code: Option<i32>, exists: bool, stderr: &str, out: &Path) -> WriteResult {
    match (code, exists) {
        (Some(0), true) => WriteResult::Written(out.to_path_buf()),
        (Some(1), false) => {
            let first = stderr
                .lines()
                .find(|l| l.starts_with("Error:"))
                .unwrap_or("Error: (no Error line)");
            WriteResult::Refused(first.trim_end().to_string())
        }
        (code, exists) => WriteResult::ToolError(format!(
            "exit {code:?} with output file exists={exists}; stderr: {}",
            stderr.lines().next().unwrap_or("")
        )),
    }
}

/// Run `exiftool <op> -o <fresh path> <input>` on `bytes`. Never `-m`.
pub fn exiftool_write(label: &str, ext: &str, op: &str, bytes: &[u8]) -> WriteResult {
    let input = match tools::temp_file_with_ext(label, ext, bytes) {
        Ok(p) => p,
        Err(e) => return WriteResult::ToolError(e.to_string()),
    };
    let out = fresh_path(label, ext);
    let args = vec![op.to_string(), "-o".to_string(), out.display().to_string()];
    let run = tools::run_tool("exiftool", &args, &input);
    let _ = std::fs::remove_file(&input);
    let run = match run {
        Ok(r) => r,
        Err(e) => return WriteResult::ToolError(e.to_string()),
    };
    // Machine paths out of the error line: the temporary input and output.
    let stderr = String::from_utf8_lossy(&run.stderr)
        .replace(&out.display().to_string(), "<out>")
        .replace(&input.display().to_string(), "<file>");
    let stderr = tools::strip_perl_lib_dir(&stderr);
    classify_write(run.status.code(), out.exists(), &stderr, &out)
}

// ── making a record ─────────────────────────────────────────────────────────

fn push_section(out: &mut String, name: &str, extra: &[String], stderr: &[String], lines: &[Line]) {
    out.push_str(&format!("## {name}\n"));
    for e in extra {
        out.push_str(e);
        out.push('\n');
    }
    for w in stderr {
        out.push_str(&format!("# stderr: {w}\n"));
    }
    for l in lines {
        out.push_str(&format!("{}\t{}\t{}\n", l.group, l.tag, l.value));
    }
}

/// The record text of one fixture: the input, the output of every recorded
/// policy that strips (with its digest), and ExifTool's own two writes.
pub fn record_text(fx: &Fixture, version: &str) -> Result<String, String> {
    let mut out = format!(
        "# command: {}\n# writes: {}\n# exiftool-version: {version}\n# fixture: {}\n",
        command_line(),
        writes_line(),
        fx.name
    );
    let (lines, stderr) = read_bytes(&format!("{}-input", fx.name), fx.ext, &fx.bytes)?;
    push_section(&mut out, "input", &[], &stderr, &lines);
    for (name, policy) in policies() {
        let Ok(stripped) = exif_core::strip(&fx.bytes, policy) else {
            continue;
        };
        let bytes = stripped.bytes();
        let (lines, stderr) = read_bytes(&format!("{}-{name}", fx.name), fx.ext, bytes)?;
        push_section(&mut out, name, &[output_line(bytes)], &stderr, &lines);
    }
    for (name, op) in WRITE_OPS {
        match exiftool_write(&format!("{}-w", fx.name), fx.ext, op, &fx.bytes) {
            WriteResult::Written(path) => {
                let read = read_path(&path);
                let _ = std::fs::remove_file(&path);
                let (lines, stderr) = read?;
                push_section(&mut out, name, &[], &stderr, &lines);
            }
            WriteResult::Refused(line) => {
                push_section(&mut out, name, &[format!("# refused: {line}")], &[], &[]);
            }
            WriteResult::ToolError(e) => {
                return Err(format!("{}: exiftool {op}: {e}", fx.name));
            }
        }
    }
    Ok(out)
}

/// Record every fixture, on `threads` worker threads. Returns `(name, ext,
/// text)` in the order of `fixtures`.
pub fn record_all(
    fixtures: &[Fixture],
    version: &str,
    threads: usize,
) -> Vec<(String, &'static str, String)> {
    let next = AtomicUsize::new(0);
    let results: std::sync::Mutex<Vec<(usize, String)>> = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                let Some(fx) = fixtures.get(i) else {
                    break;
                };
                let text = record_text(fx, version).unwrap_or_else(|e| panic!("{}: {e}", fx.name));
                results.lock().expect("results").push((i, text));
            });
        }
    });
    let mut done = results.into_inner().expect("results");
    done.sort_by_key(|(i, _)| *i);
    done.into_iter()
        .map(|(i, text)| (fixtures[i].name.clone(), fixtures[i].ext, text))
        .collect()
}

/// The directory of a container's records, relative to the repository root.
pub fn record_dir(ext: &str) -> String {
    format!("tests/oracle-fixtures/exiftool-strip-{ext}")
}

/// The name of a refusal for the pinned table: the `StripError` variant, with
/// the reason in parentheses when it has one, and no offset (an offset is a
/// property of the fixture, not of the pin).
pub fn variant_name(e: &exif_core::StripError) -> String {
    let d = format!("{e:?}");
    let name = d.split([' ', '(']).next().unwrap_or("");
    match d.split("reason: ").nth(1) {
        Some(rest) => {
            let reason = rest.split([',', ' ', '}']).next().unwrap_or("");
            format!("{name}({reason})")
        }
        None => name.to_string(),
    }
}

/// One row of [`DIFFERENCES`].
pub struct Diff {
    /// `D1` to `D6` (DEC-023 rule 4).
    pub class: &'static str,
    /// `keys` or `values`.
    pub level: &'static str,
    /// `all` (`Everything` against `-all=`) or `gps` (`Gps` against `-gps:all=`).
    pub pair: &'static str,
    pub ours_only: &'static [&'static str],
    pub theirs_only: &'static [&'static str],
    pub fixtures: &'static [&'static str],
}

/// The design's counts over the 174 oracle fixtures (DEC-023 rule 4, SPEC-039
/// Context 2): classes D1 to D5 at `keys` level, the outcome differences and
/// the pairs both tools refuse.
pub const DESIGN_KEY_COUNTS: [(&str, usize); 5] =
    [("D1", 72), ("D2", 2), ("D3", 7), ("D4", 4), ("D5", 1)];
/// The value-level classes over the same fixtures (DEC-023 rule 4 as amended
/// 2026-11-25): pairs with equal `(group, tag)` multisets whose values differ.
pub const DESIGN_VALUE_COUNTS: [(&str, usize); 2] = [("D4", 3), ("D6", 31)];
/// Pairs identical at both levels.
pub const DESIGN_IDENTICAL: usize = 157;
pub const DESIGN_OUTCOME_DIFFERENCES: usize = 41;
pub const DESIGN_BOTH_REFUSE: usize = 30;
pub const DESIGN_PAIRS: usize = 348;
pub const DESIGN_REFUSED_PRESET_PAIRS: usize = 113;

// ── the pinned tables (hand-pinned from ExifTool 13.55, SPEC-039 build) ──────

/// Refusals of the library: `(fixture, StripError variant, policies)`. An
/// unpinned refusal fails, and so does a pinned one that now strips (DEC-023
/// rule 2). Of the five presets: 13 fixtures are refused under all five, 16
/// under `gps`, `gps_and_device` and `all_pii`.
pub const REFUSALS: &[(&str, &str, &[&str])] = &[
    ("png-exif-after-iend", "Overlap", &["other"]),
    (
        "png-exif-empty",
        "Uneditable(BlockNotClean)",
        &[
            "gps",
            "gps_and_device",
            "all_pii",
            "xmp",
            "icc",
            "rights",
            "timestamps",
            "free_text",
        ],
    ),
    (
        "png-exif-fatal-then-good",
        "Uneditable(BlockNotClean)",
        &[
            "gps",
            "gps_and_device",
            "all_pii",
            "xmp",
            "icc",
            "rights",
            "timestamps",
            "free_text",
        ],
    ),
    (
        "png-exif-good-then-fatal",
        "Uneditable(BlockNotClean)",
        &[
            "gps",
            "gps_and_device",
            "all_pii",
            "xmp",
            "icc",
            "rights",
            "timestamps",
            "free_text",
        ],
    ),
    (
        "png-exif-not-tiff",
        "Uneditable(BlockNotClean)",
        &[
            "gps",
            "gps_and_device",
            "all_pii",
            "xmp",
            "icc",
            "rights",
            "timestamps",
            "free_text",
        ],
    ),
    (
        "png-gps-corrupt-exif-ok",
        "Uneditable(BlockNotClean)",
        &[
            "gps",
            "gps_and_device",
            "all_pii",
            "xmp",
            "icc",
            "rights",
            "timestamps",
            "free_text",
        ],
    ),
    (
        "png-ifd1-corrupt",
        "Uneditable(BlockNotClean)",
        &[
            "gps",
            "gps_and_device",
            "all_pii",
            "xmp",
            "icc",
            "rights",
            "timestamps",
            "free_text",
        ],
    ),
    (
        "png-length-80000000",
        "Unstrippable(BadSegmentLength)",
        &[
            "gps",
            "gps_and_device",
            "all_pii",
            "all",
            "everything",
            "xmp",
            "icc",
            "jumbf",
            "rights",
            "timestamps",
            "free_text",
            "other",
        ],
    ),
    (
        "png-length-past-eof",
        "Unstrippable(ChunkPastEnd)",
        &[
            "gps",
            "gps_and_device",
            "all_pii",
            "all",
            "everything",
            "xmp",
            "icc",
            "jumbf",
            "rights",
            "timestamps",
            "free_text",
            "other",
        ],
    ),
    (
        "webp-chunk-length-7fffffff",
        "Unstrippable(ChunkPastEnd)",
        &[
            "gps",
            "gps_and_device",
            "all_pii",
            "all",
            "everything",
            "xmp",
            "icc",
            "jumbf",
            "rights",
            "timestamps",
            "free_text",
            "other",
        ],
    ),
    (
        "webp-chunk-length-ffffffff",
        "Unstrippable(ChunkPastEnd)",
        &[
            "gps",
            "gps_and_device",
            "all_pii",
            "all",
            "everything",
            "xmp",
            "icc",
            "jumbf",
            "rights",
            "timestamps",
            "free_text",
            "other",
        ],
    ),
    (
        "webp-chunk-length-past-eof",
        "Unstrippable(ChunkPastEnd)",
        &[
            "gps",
            "gps_and_device",
            "all_pii",
            "all",
            "everything",
            "xmp",
            "icc",
            "jumbf",
            "rights",
            "timestamps",
            "free_text",
            "other",
        ],
    ),
    (
        "webp-exif-16-then-gps",
        "Uneditable(TooManyExifSegments)",
        &[
            "gps",
            "gps_and_device",
            "all_pii",
            "xmp",
            "icc",
            "rights",
            "timestamps",
            "free_text",
        ],
    ),
    (
        "webp-exif-bad-then-good",
        "Uneditable(BlockNotClean)",
        &[
            "gps",
            "gps_and_device",
            "all_pii",
            "xmp",
            "icc",
            "rights",
            "timestamps",
            "free_text",
        ],
    ),
    (
        "webp-exif-empty",
        "Uneditable(BlockNotClean)",
        &[
            "gps",
            "gps_and_device",
            "all_pii",
            "xmp",
            "icc",
            "rights",
            "timestamps",
            "free_text",
        ],
    ),
    (
        "webp-exif-garbage",
        "Uneditable(BlockNotClean)",
        &[
            "gps",
            "gps_and_device",
            "all_pii",
            "xmp",
            "icc",
            "rights",
            "timestamps",
            "free_text",
        ],
    ),
    (
        "webp-exif-good-then-bad",
        "Uneditable(BlockNotClean)",
        &[
            "gps",
            "gps_and_device",
            "all_pii",
            "xmp",
            "icc",
            "rights",
            "timestamps",
            "free_text",
        ],
    ),
    (
        "webp-exif-odd-unpadded-then-xmp",
        "Unstrippable(ChunkPastEnd)",
        &[
            "gps",
            "gps_and_device",
            "all_pii",
            "all",
            "everything",
            "xmp",
            "icc",
            "jumbf",
            "rights",
            "timestamps",
            "free_text",
            "other",
        ],
    ),
    (
        "webp-exif-one-byte",
        "Uneditable(BlockNotClean)",
        &[
            "gps",
            "gps_and_device",
            "all_pii",
            "xmp",
            "icc",
            "rights",
            "timestamps",
            "free_text",
        ],
    ),
    (
        "webp-exif-tiff-8-bytes",
        "Uneditable(BlockNotClean)",
        &[
            "gps",
            "gps_and_device",
            "all_pii",
            "xmp",
            "icc",
            "rights",
            "timestamps",
            "free_text",
        ],
    ),
    (
        "webp-gps-corrupt-exif-ok",
        "Uneditable(BlockNotClean)",
        &[
            "gps",
            "gps_and_device",
            "all_pii",
            "xmp",
            "icc",
            "rights",
            "timestamps",
            "free_text",
        ],
    ),
    (
        "webp-header-11-bytes",
        "NotJpeg",
        &[
            "gps",
            "gps_and_device",
            "all_pii",
            "all",
            "everything",
            "xmp",
            "icc",
            "jumbf",
            "rights",
            "timestamps",
            "free_text",
            "other",
        ],
    ),
    (
        "webp-header-20-exif-size-0",
        "Uneditable(BlockNotClean)",
        &[
            "gps",
            "gps_and_device",
            "all_pii",
            "xmp",
            "icc",
            "rights",
            "timestamps",
            "free_text",
        ],
    ),
    (
        "webp-header-avi",
        "NotJpeg",
        &[
            "gps",
            "gps_and_device",
            "all_pii",
            "all",
            "everything",
            "xmp",
            "icc",
            "jumbf",
            "rights",
            "timestamps",
            "free_text",
            "other",
        ],
    ),
    (
        "webp-header-rifx",
        "NotJpeg",
        &[
            "gps",
            "gps_and_device",
            "all_pii",
            "all",
            "everything",
            "xmp",
            "icc",
            "jumbf",
            "rights",
            "timestamps",
            "free_text",
            "other",
        ],
    ),
    (
        "webp-header-wave",
        "NotJpeg",
        &[
            "gps",
            "gps_and_device",
            "all_pii",
            "all",
            "everything",
            "xmp",
            "icc",
            "jumbf",
            "rights",
            "timestamps",
            "free_text",
            "other",
        ],
    ),
    (
        "webp-ifd1-corrupt",
        "Uneditable(BlockNotClean)",
        &[
            "gps",
            "gps_and_device",
            "all_pii",
            "xmp",
            "icc",
            "rights",
            "timestamps",
            "free_text",
        ],
    ),
    (
        "webp-stray-pad-then-xmp",
        "Unstrippable(ChunkPastEnd)",
        &[
            "gps",
            "gps_and_device",
            "all_pii",
            "all",
            "everything",
            "xmp",
            "icc",
            "jumbf",
            "rights",
            "timestamps",
            "free_text",
            "other",
        ],
    ),
    (
        "webp-tail-12-bytes",
        "Unstrippable(ChunkPastEnd)",
        &[
            "gps",
            "gps_and_device",
            "all_pii",
            "all",
            "everything",
            "xmp",
            "icc",
            "jumbf",
            "rights",
            "timestamps",
            "free_text",
            "other",
        ],
    ),
    (
        "webp-xmp-odd-unpadded-then-exif",
        "Unstrippable(ChunkPastEnd)",
        &[
            "gps",
            "gps_and_device",
            "all_pii",
            "all",
            "everything",
            "xmp",
            "icc",
            "jumbf",
            "rights",
            "timestamps",
            "free_text",
            "other",
        ],
    ),
];

/// What ExifTool's own writer does differently from ours (DEC-023 rule 4).
/// One row per `(class, level, pair, ours-only lines, ExifTool-only lines)`, the
/// fixtures it holds for. `level` is `keys` (the `(group, tag)` multisets of the
/// two outputs' readings differ) or `values` (the keys agree and a value differs).
/// Lines are `Group:Tag`, or `Group:Tag=value` at `values`.
pub const DIFFERENCES: &[Diff] = &[
    Diff {
        class: "D3",
        level: "keys",
        pair: "gps",
        ours_only: &[],
        theirs_only: &[
            "Composite:GPSPosition",
            "ExifIFD:OwnerName",
            "ExifIFD:SerialNumber",
            "IFD0:Artist",
            "IFD0:Copyright",
            "IFD0:Make",
            "Trailer:ExifByteOrder",
            "Trailer:GPSLatitude",
            "Trailer:GPSLatitudeRef",
            "Trailer:GPSLongitude",
            "Trailer:GPSLongitudeRef",
        ],
        fixtures: &["png-carrier-after-iend"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "all",
        ours_only: &[],
        theirs_only: &["XML:AB"],
        fixtures: &["png-carrier-meTa"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "gps",
        ours_only: &[],
        theirs_only: &["XML:AB"],
        fixtures: &["png-carrier-meTa"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "gps",
        ours_only: &[],
        theirs_only: &[
            "ExifIFD:OwnerName",
            "ExifIFD:SerialNumber",
            "IFD0:Artist",
            "IFD0:Copyright",
            "IFD0:Make",
        ],
        fixtures: &["png-carrier-raw-profile-text", "png-carrier-zxIf"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "gps",
        ours_only: &[],
        theirs_only: &["SEAL:AB"],
        fixtures: &["png-carrier-seAl"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "gps",
        ours_only: &[],
        theirs_only: &[
            "Composite:GPSLatitudeRef",
            "Composite:GPSLongitudeRef",
            "Composite:GPSPosition",
            "XMP-dc:Creator",
            "XMP-exif:GPSLatitude",
            "XMP-exif:GPSLongitude",
        ],
        fixtures: &["png-carrier-tXMP"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "gps",
        ours_only: &[],
        theirs_only: &["IFD0:Make"],
        fixtures: &["png-exif-and-raw-exif"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "gps",
        ours_only: &[],
        theirs_only: &[
            "Composite:Aperture",
            "ExifIFD:DateTimeOriginal",
            "ExifIFD:ExposureCompensation",
            "ExifIFD:FNumber",
            "ExifIFD:ISO",
            "IFD0:Artist",
            "IFD0:Copyright",
            "IFD0:ImageHeight",
            "IFD0:ImageWidth",
            "IFD0:Make",
            "IFD0:Model",
            "IFD0:ModifyDate",
            "IFD0:Orientation",
            "IFD0:Software",
            "IFD0:XResolution",
        ],
        fixtures: &[
            "png-raw-exif-itxt",
            "png-raw-exif-text",
            "png-raw-exif-ztxt",
        ],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "gps",
        ours_only: &[],
        theirs_only: &["XMP-xmp:CreatorTool"],
        fixtures: &["png-raw-xmp-itxt", "png-raw-xmp-text", "png-raw-xmp-ztxt"],
    },
    Diff {
        class: "D5",
        level: "keys",
        pair: "all",
        ours_only: &[],
        theirs_only: &["PNG:TextualData"],
        fixtures: &["png-text-no-nul"],
    },
    Diff {
        class: "D1",
        level: "keys",
        pair: "all",
        ours_only: &[
            "Composite:ImageSize",
            "Composite:Megapixels",
            "RIFF:ImageHeight",
            "RIFF:ImageWidth",
            "RIFF:WebP_Flags",
        ],
        theirs_only: &[],
        fixtures: &[
            "webp-all-three",
            "webp-ascii-edges",
            "webp-exif-16-then-gps",
            "webp-exif-anmf-then-top-level",
            "webp-exif-bad-then-good",
            "webp-exif-basic-be",
            "webp-exif-basic-le",
            "webp-exif-basic",
            "webp-exif-before-vp8x",
            "webp-exif-blocks-16",
            "webp-exif-flag-unset",
            "webp-exif-full-one-two",
            "webp-exif-full-two-one",
            "webp-exif-garbage",
            "webp-exif-good-then-bad",
            "webp-exif-good-then-prefixed",
            "webp-exif-gps-full-le",
            "webp-exif-gps-ne-le",
            "webp-exif-gps-nw-le",
            "webp-exif-gps-se-be",
            "webp-exif-gps-sw-be",
            "webp-exif-make-short-later",
            "webp-exif-odd-padded",
            "webp-exif-one-byte",
            "webp-exif-prefix",
            "webp-exif-tiff-8-bytes",
            "webp-exif-two-same-tag-reversed",
            "webp-exif-two-same-tag",
            "webp-flags-all-wrong",
            "webp-gps-corrupt-exif-ok",
            "webp-icc-garbage-then-valid",
            "webp-icc-garbage",
            "webp-icc-header-only",
            "webp-icc-stub-132",
            "webp-icc-three",
            "webp-icc-valid-flag-set",
            "webp-icc-valid-flag-unset",
            "webp-icc-valid-then-garbage",
            "webp-icc-valid-trailing-garbage",
            "webp-icc-zero-tags",
            "webp-ifd1-corrupt",
            "webp-junk-first",
            "webp-riff-size-0",
            "webp-riff-size-10",
            "webp-riff-size-exact",
            "webp-riff-size-ffff-fff0",
            "webp-riff-size-odd",
            "webp-riff-size-small",
            "webp-row-05-seal",
            "webp-row-22-c2pa",
            "webp-signed-float-be",
            "webp-signed-float-le",
            "webp-vp8x-late",
            "webp-vp8x-len-10",
            "webp-vp8x-len-18",
            "webp-xmp-basic",
            "webp-xmp-empty-then-real",
            "webp-xmp-garbage",
            "webp-xmp-nul-id-1-bytes",
            "webp-xmp-nul-id-2-bytes",
            "webp-xmp-nul-id-3-bytes",
            "webp-xmp-nul-id-4-bytes",
            "webp-xmp-nul-id-5-bytes",
            "webp-xmp-nul-id-6-bytes",
            "webp-xmp-nul-id-7-bytes",
            "webp-xmp-nul-id",
            "webp-xmp-odd-padded",
            "webp-xmp-plain-gps",
            "webp-xmp-three",
            "webp-xmp-two-distinct",
            "webp-xmp-two",
            "webp-xmp-utf16le",
            "webp-xmp-utf8-bom",
        ],
    },
    Diff {
        class: "D4",
        level: "values",
        pair: "gps",
        ours_only: &[
            "IFD0:Make=Block00",
            "IFD0:Make=Block01",
            "IFD0:Make=Block02",
            "IFD0:Make=Block03",
            "IFD0:Make=Block04",
            "IFD0:Make=Block05",
            "IFD0:Make=Block06",
            "IFD0:Make=Block07",
            "IFD0:Make=Block08",
            "IFD0:Make=Block09",
            "IFD0:Make=Block10",
            "IFD0:Make=Block11",
            "IFD0:Make=Block12",
            "IFD0:Make=Block13",
            "IFD0:Make=Block14",
        ],
        theirs_only: &[
            "IFD0:Make=Block15",
            "IFD0:Make=Block15",
            "IFD0:Make=Block15",
            "IFD0:Make=Block15",
            "IFD0:Make=Block15",
            "IFD0:Make=Block15",
            "IFD0:Make=Block15",
            "IFD0:Make=Block15",
            "IFD0:Make=Block15",
            "IFD0:Make=Block15",
            "IFD0:Make=Block15",
            "IFD0:Make=Block15",
            "IFD0:Make=Block15",
            "IFD0:Make=Block15",
            "IFD0:Make=Block15",
        ],
        fixtures: &["webp-exif-blocks-16"],
    },
    Diff {
        class: "D6",
        level: "values",
        pair: "gps",
        ours_only: &["RIFF:WebP_Flags=0"],
        theirs_only: &["RIFF:WebP_Flags=8"],
        fixtures: &["webp-exif-flag-unset", "webp-vp8x-two"],
    },
    Diff {
        class: "D4",
        level: "values",
        pair: "gps",
        ours_only: &[
            "ExifIFD:DateTimeOriginal=2021:02:01 01:02:03",
            "ExifIFD:ExifImageWidth=-5",
            "ExifIFD:ExposureCompensation=-0.25",
            "ExifIFD:ExposureProgram=-1",
            "ExifIFD:ExposureTime=0.125",
            "ExifIFD:FNumber=2.8",
            "ExifIFD:FocalLength=4.5",
            "ExifIFD:ISO=100",
            "IFD0:Artist=ArtistOne",
            "IFD0:Copyright=CopyOne",
            "IFD0:ImageHeight=50",
            "IFD0:ImageWidth=100",
            "IFD0:Make=MakeOne",
            "IFD0:Model=ModelOne",
            "IFD0:ModifyDate=2020:01:01 00:00:00",
            "IFD0:Orientation=1",
            "IFD0:Software=SoftOne",
            "IFD0:XResolution=72",
        ],
        theirs_only: &[
            "ExifIFD:DateTimeOriginal=2021:02:02 01:02:03",
            "ExifIFD:ExifImageWidth=-10",
            "ExifIFD:ExposureCompensation=-0.5",
            "ExifIFD:ExposureProgram=-2",
            "ExifIFD:ExposureTime=0.25",
            "ExifIFD:FNumber=5.6",
            "ExifIFD:FocalLength=9",
            "ExifIFD:ISO=200",
            "IFD0:Artist=ArtistTwo",
            "IFD0:Copyright=CopyTwo",
            "IFD0:ImageHeight=100",
            "IFD0:ImageWidth=200",
            "IFD0:Make=MakeTwo",
            "IFD0:Model=ModelTwo",
            "IFD0:ModifyDate=2020:01:02 00:00:00",
            "IFD0:Orientation=8",
            "IFD0:Software=SoftTwo",
            "IFD0:XResolution=144",
        ],
        fixtures: &["webp-exif-full-one-two"],
    },
    Diff {
        class: "D4",
        level: "values",
        pair: "gps",
        ours_only: &[
            "ExifIFD:DateTimeOriginal=2021:02:02 01:02:03",
            "ExifIFD:ExifImageWidth=-10",
            "ExifIFD:ExposureCompensation=-0.5",
            "ExifIFD:ExposureProgram=-2",
            "ExifIFD:ExposureTime=0.25",
            "ExifIFD:FNumber=5.6",
            "ExifIFD:FocalLength=9",
            "ExifIFD:ISO=200",
            "IFD0:Artist=ArtistTwo",
            "IFD0:Copyright=CopyTwo",
            "IFD0:ImageHeight=100",
            "IFD0:ImageWidth=200",
            "IFD0:Make=MakeTwo",
            "IFD0:Model=ModelTwo",
            "IFD0:ModifyDate=2020:01:02 00:00:00",
            "IFD0:Orientation=8",
            "IFD0:Software=SoftTwo",
            "IFD0:XResolution=144",
        ],
        theirs_only: &[
            "ExifIFD:DateTimeOriginal=2021:02:01 01:02:03",
            "ExifIFD:ExifImageWidth=-5",
            "ExifIFD:ExposureCompensation=-0.25",
            "ExifIFD:ExposureProgram=-1",
            "ExifIFD:ExposureTime=0.125",
            "ExifIFD:FNumber=2.8",
            "ExifIFD:FocalLength=4.5",
            "ExifIFD:ISO=100",
            "IFD0:Artist=ArtistOne",
            "IFD0:Copyright=CopyOne",
            "IFD0:ImageHeight=50",
            "IFD0:ImageWidth=100",
            "IFD0:Make=MakeOne",
            "IFD0:Model=ModelOne",
            "IFD0:ModifyDate=2020:01:01 00:00:00",
            "IFD0:Orientation=1",
            "IFD0:Software=SoftOne",
            "IFD0:XResolution=72",
        ],
        fixtures: &["webp-exif-full-two-one"],
    },
    Diff {
        class: "D4",
        level: "keys",
        pair: "gps",
        ours_only: &["IFD0:Model"],
        theirs_only: &[],
        fixtures: &[
            "webp-exif-good-then-prefixed",
            "webp-exif-make-short-later",
            "webp-exif-two-same-tag",
        ],
    },
    Diff {
        class: "D6",
        level: "values",
        pair: "all",
        ours_only: &["RIFF:WebP_Flags=10"],
        theirs_only: &["RIFF:WebP_Flags=2"],
        fixtures: &["webp-exif-inside-anmf"],
    },
    Diff {
        class: "D6",
        level: "values",
        pair: "gps",
        ours_only: &["RIFF:WebP_Flags=10"],
        theirs_only: &["RIFF:WebP_Flags=2"],
        fixtures: &["webp-exif-inside-anmf"],
    },
    Diff {
        class: "D6",
        level: "values",
        pair: "all",
        ours_only: &["RIFF:WebP_Flags=8"],
        theirs_only: &["RIFF:WebP_Flags=0"],
        fixtures: &[
            "webp-exif-kind-capital-e",
            "webp-exif-kind-last-letter-lower",
            "webp-exif-kind-lower",
        ],
    },
    Diff {
        class: "D6",
        level: "values",
        pair: "gps",
        ours_only: &["RIFF:WebP_Flags=8"],
        theirs_only: &["RIFF:WebP_Flags=0"],
        fixtures: &[
            "webp-exif-kind-capital-e",
            "webp-exif-kind-last-letter-lower",
            "webp-exif-kind-lower",
        ],
    },
    Diff {
        class: "D2",
        level: "keys",
        pair: "gps",
        ours_only: &[],
        theirs_only: &[
            "Composite:ImageSize",
            "Composite:Megapixels",
            "RIFF:ImageHeight",
            "RIFF:ImageWidth",
            "RIFF:WebP_Flags",
        ],
        fixtures: &["webp-exif-no-vp8x", "webp-vp8x-len-0"],
    },
    Diff {
        class: "D4",
        level: "keys",
        pair: "gps",
        ours_only: &[],
        theirs_only: &["IFD0:Model"],
        fixtures: &["webp-exif-two-same-tag-reversed"],
    },
    Diff {
        class: "D6",
        level: "values",
        pair: "gps",
        ours_only: &["RIFF:WebP_Flags=36"],
        theirs_only: &["RIFF:WebP_Flags=8"],
        fixtures: &["webp-flags-all-wrong"],
    },
    Diff {
        class: "D6",
        level: "values",
        pair: "gps",
        ours_only: &["RIFF:WebP_Flags=32"],
        theirs_only: &["RIFF:WebP_Flags=0"],
        fixtures: &["webp-icc-empty", "webp-icc-kind-lower"],
    },
    Diff {
        class: "D6",
        level: "values",
        pair: "all",
        ours_only: &["RIFF:WebP_Flags=32"],
        theirs_only: &["RIFF:WebP_Flags=0"],
        fixtures: &["webp-icc-kind-lower"],
    },
    Diff {
        class: "D6",
        level: "values",
        pair: "gps",
        ours_only: &["RIFF:WebP_Flags=0"],
        theirs_only: &["RIFF:WebP_Flags=32"],
        fixtures: &["webp-icc-valid-flag-unset"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "all",
        ours_only: &[],
        theirs_only: &["RIFF:Artist"],
        fixtures: &["webp-row-00-list", "webp-row-01-riff"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "gps",
        ours_only: &[],
        theirs_only: &["RIFF:Artist"],
        fixtures: &["webp-row-00-list", "webp-row-01-riff"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "all",
        ours_only: &[],
        theirs_only: &["RIFF:DateTimeOriginal"],
        fixtures: &["webp-row-02-idit"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "gps",
        ours_only: &[],
        theirs_only: &["RIFF:DateTimeOriginal"],
        fixtures: &["webp-row-02-idit"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "all",
        ours_only: &[],
        theirs_only: &["RIFF:TextJunk"],
        fixtures: &["webp-row-03-junk"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "gps",
        ours_only: &[],
        theirs_only: &["RIFF:TextJunk"],
        fixtures: &["webp-row-03-junk"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "all",
        ours_only: &[],
        theirs_only: &["RIFF:OldXMP"],
        fixtures: &["webp-row-04-junq"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "gps",
        ours_only: &[],
        theirs_only: &["RIFF:OldXMP"],
        fixtures: &["webp-row-04-junq"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "gps",
        ours_only: &[],
        theirs_only: &["XML-dc:Creator", "XML-photoshop:City"],
        fixtures: &["webp-row-05-seal", "webp-row-07-axml", "webp-row-08-ixml"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "all",
        ours_only: &[],
        theirs_only: &["XMP-dc:Creator", "XMP-photoshop:City"],
        fixtures: &["webp-row-06-_pmx"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "gps",
        ours_only: &[],
        theirs_only: &["XMP-dc:Creator", "XMP-photoshop:City"],
        fixtures: &["webp-row-06-_pmx"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "all",
        ours_only: &[],
        theirs_only: &["XML-dc:Creator", "XML-photoshop:City"],
        fixtures: &["webp-row-07-axml", "webp-row-08-ixml"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "all",
        ours_only: &[],
        theirs_only: &["RIFF:Afsp"],
        fixtures: &["webp-row-09-afsp"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "gps",
        ours_only: &[],
        theirs_only: &["RIFF:Afsp"],
        fixtures: &["webp-row-09-afsp"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "all",
        ours_only: &[],
        theirs_only: &["RIFF:Description"],
        fixtures: &["webp-row-10-bext"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "gps",
        ours_only: &[],
        theirs_only: &["RIFF:Description"],
        fixtures: &["webp-row-10-bext"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "all",
        ours_only: &[],
        theirs_only: &["RIFF:Guano"],
        fixtures: &["webp-row-11-guan"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "gps",
        ours_only: &[],
        theirs_only: &["RIFF:Guano"],
        fixtures: &["webp-row-11-guan"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "all",
        ours_only: &[],
        theirs_only: &["RIFF:ListType"],
        fixtures: &["webp-row-12-list"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "gps",
        ours_only: &[],
        theirs_only: &["RIFF:ListType"],
        fixtures: &["webp-row-12-list"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "all",
        ours_only: &[],
        theirs_only: &["RIFF:CuePointLabel"],
        fixtures: &["webp-row-13-labl"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "gps",
        ours_only: &[],
        theirs_only: &["RIFF:CuePointLabel"],
        fixtures: &["webp-row-13-labl"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "all",
        ours_only: &[],
        theirs_only: &["RIFF:CuePointNote"],
        fixtures: &["webp-row-14-note"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "gps",
        ours_only: &[],
        theirs_only: &["RIFF:CuePointNote"],
        fixtures: &["webp-row-14-note"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "all",
        ours_only: &[],
        theirs_only: &["RIFF:LabeledText"],
        fixtures: &["webp-row-15-ltxt"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "gps",
        ours_only: &[],
        theirs_only: &["RIFF:LabeledText"],
        fixtures: &["webp-row-15-ltxt"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "all",
        ours_only: &[],
        theirs_only: &["Olympus:FileNumber", "Olympus:Model"],
        fixtures: &["webp-row-16-olym"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "gps",
        ours_only: &[],
        theirs_only: &["Olympus:FileNumber", "Olympus:Model"],
        fixtures: &["webp-row-16-olym"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "all",
        ours_only: &[],
        theirs_only: &["ID3v2_3:Artist"],
        fixtures: &["webp-row-17-id3_", "webp-row-18-id3_"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "gps",
        ours_only: &[],
        theirs_only: &["ID3v2_3:Artist"],
        fixtures: &["webp-row-17-id3_", "webp-row-18-id3_"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "all",
        ours_only: &[],
        theirs_only: &[
            "Composite:GPSPosition",
            "RIFF:GPSAltitude",
            "RIFF:GPSDateTime",
            "RIFF:GPSLatitude",
            "RIFF:GPSLongitude",
            "RIFF:GPSSpeed",
            "RIFF:GPSTrack",
            "RIFF:Warning",
        ],
        fixtures: &["webp-row-19-gps0"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "gps",
        ours_only: &[],
        theirs_only: &[
            "Composite:GPSPosition",
            "RIFF:GPSAltitude",
            "RIFF:GPSDateTime",
            "RIFF:GPSLatitude",
            "RIFF:GPSLongitude",
            "RIFF:GPSSpeed",
            "RIFF:GPSTrack",
            "RIFF:Warning",
        ],
        fixtures: &["webp-row-19-gps0"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "all",
        ours_only: &[],
        theirs_only: &["RIFF:Accelerometer", "RIFF:FrameNumber", "RIFF:Warning"],
        fixtures: &["webp-row-20-sglt"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "gps",
        ours_only: &[],
        theirs_only: &["RIFF:Accelerometer", "RIFF:FrameNumber", "RIFF:Warning"],
        fixtures: &["webp-row-20-sglt"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "all",
        ours_only: &[],
        theirs_only: &[
            "Composite:GPSPosition",
            "RIFF:FrameNumber",
            "RIFF:GPSAltitude",
            "RIFF:GPSDateTime",
            "RIFF:GPSLatitude",
            "RIFF:GPSLongitude",
            "RIFF:GPSSpeed",
            "RIFF:GPSSpeedRef",
            "RIFF:Warning",
        ],
        fixtures: &["webp-row-21-sllt"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "gps",
        ours_only: &[],
        theirs_only: &[
            "Composite:GPSPosition",
            "RIFF:FrameNumber",
            "RIFF:GPSAltitude",
            "RIFF:GPSDateTime",
            "RIFF:GPSLatitude",
            "RIFF:GPSLongitude",
            "RIFF:GPSSpeed",
            "RIFF:GPSSpeedRef",
            "RIFF:Warning",
        ],
        fixtures: &["webp-row-21-sllt"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "all",
        ours_only: &[],
        theirs_only: &[
            "RIFF:CodePage",
            "RIFF:CountryCode",
            "RIFF:Dialect",
            "RIFF:LanguageCode",
        ],
        fixtures: &["webp-row-23-cset"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "all",
        ours_only: &[],
        theirs_only: &[
            "RIFF:AcidizerFlags",
            "RIFF:Beats",
            "RIFF:Meter",
            "RIFF:RootNote",
            "RIFF:Tempo",
        ],
        fixtures: &["webp-row-24-acid"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "all",
        ours_only: &[],
        theirs_only: &[
            "RIFF:DataSize64",
            "RIFF:NumberOfSamples64",
            "RIFF:RIFFSize64",
        ],
        fixtures: &["webp-row-25-ds64"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "all",
        ours_only: &[],
        theirs_only: &["RIFF:NumberOfSamples"],
        fixtures: &["webp-row-26-fact"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "all",
        ours_only: &[],
        theirs_only: &[
            "RIFF:AvgBytesPerSec",
            "RIFF:BitsPerSample",
            "RIFF:Encoding",
            "RIFF:NumChannels",
            "RIFF:SampleRate",
        ],
        fixtures: &["webp-row-27-fmt_"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "all",
        ours_only: &[],
        theirs_only: &[
            "RIFF:FineTune",
            "RIFF:Gain",
            "RIFF:HighNote",
            "RIFF:HighVelocity",
            "RIFF:LowNote",
            "RIFF:LowVelocity",
            "RIFF:UnshiftedNote",
        ],
        fixtures: &["webp-row-28-inst"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "all",
        ours_only: &[],
        theirs_only: &[
            "RIFF:MIDIPitchFraction",
            "RIFF:MIDIUnityNote",
            "RIFF:Manufacturer",
            "RIFF:NumSampleLoops",
            "RIFF:Product",
            "RIFF:SMPTEFormat",
            "RIFF:SMPTEOffset",
            "RIFF:SamplePeriod",
            "RIFF:SamplerData",
            "RIFF:SamplerDataLen",
        ],
        fixtures: &["webp-row-29-smpl"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "all",
        ours_only: &[],
        theirs_only: &["RIFF:CuePoints"],
        fixtures: &["webp-row-30-cue_"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "all",
        ours_only: &[],
        theirs_only: &["RIFF:Playlist"],
        fixtures: &["webp-row-31-plst"],
    },
    Diff {
        class: "D3",
        level: "keys",
        pair: "all",
        ours_only: &[],
        theirs_only: &["RIFF:Accelerometer", "RIFF:Warning"],
        fixtures: &["webp-row-32-gsen"],
    },
    Diff {
        class: "D1",
        level: "keys",
        pair: "all",
        ours_only: &["RIFF:ImageWidth", "RIFF:WebP_Flags"],
        theirs_only: &[],
        fixtures: &["webp-vp8x-len-9"],
    },
    Diff {
        class: "D1",
        level: "keys",
        pair: "all",
        ours_only: &[
            "Composite:ImageSize",
            "Composite:Megapixels",
            "RIFF:ImageHeight",
            "RIFF:ImageHeight",
            "RIFF:ImageWidth",
            "RIFF:ImageWidth",
            "RIFF:WebP_Flags",
            "RIFF:WebP_Flags",
        ],
        theirs_only: &[],
        fixtures: &["webp-vp8x-two"],
    },
    Diff {
        class: "D6",
        level: "values",
        pair: "gps",
        ours_only: &["RIFF:WebP_Flags=4"],
        theirs_only: &["RIFF:WebP_Flags=0"],
        fixtures: &[
            "webp-xmp-empty",
            "webp-xmp-kind-dash",
            "webp-xmp-kind-lower",
            "webp-xmp-kind-x",
            "webp-xmp-nul-id-1-bytes",
            "webp-xmp-nul-id-2-bytes",
            "webp-xmp-nul-id-3-bytes",
            "webp-xmp-nul-id-4-bytes",
            "webp-xmp-nul-id-5-bytes",
            "webp-xmp-nul-id-6-bytes",
            "webp-xmp-nul-id-7-bytes",
            "webp-xmp-nul-id-empty",
            "webp-xmp-nul-id",
        ],
    },
    Diff {
        class: "D6",
        level: "values",
        pair: "all",
        ours_only: &["RIFF:WebP_Flags=4"],
        theirs_only: &["RIFF:WebP_Flags=0"],
        fixtures: &[
            "webp-xmp-kind-dash",
            "webp-xmp-kind-lower",
            "webp-xmp-kind-x",
        ],
    },
];

/// Outcome differences: one tool refuses where the other writes. `(fixture, pair,
/// who, detail)`: ExifTool's first `Error:` line, or our `StripError` variant.
pub const OUTCOMES: &[(&str, &str, &str, &str)] = &[
    (
        "png-exif-bad-crc",
        "all",
        "exiftool-refuses",
        "Error: [minor] Bad CRC for eXIf chunk - <file>",
    ),
    (
        "png-exif-bad-crc",
        "gps",
        "exiftool-refuses",
        "Error: [minor] Bad CRC for eXIf chunk - <file>",
    ),
    (
        "png-exif-empty",
        "gps",
        "we-refuse",
        "Uneditable(BlockNotClean)",
    ),
    (
        "png-exif-fatal-then-good",
        "gps",
        "we-refuse",
        "Uneditable(BlockNotClean)",
    ),
    (
        "png-exif-good-then-fatal",
        "gps",
        "we-refuse",
        "Uneditable(BlockNotClean)",
    ),
    (
        "png-exif-not-tiff",
        "gps",
        "we-refuse",
        "Uneditable(BlockNotClean)",
    ),
    (
        "png-no-iend",
        "all",
        "exiftool-refuses",
        "Error: Error writing output file - <file>",
    ),
    (
        "png-no-iend",
        "gps",
        "exiftool-refuses",
        "Error: Error writing output file - <file>",
    ),
    (
        "png-truncated-between-ihdr-and-idat",
        "all",
        "exiftool-refuses",
        "Error: Error writing output file - <file>",
    ),
    (
        "png-truncated-between-ihdr-and-idat",
        "gps",
        "exiftool-refuses",
        "Error: Error writing output file - <file>",
    ),
    (
        "png-two-exif-disjoint-tags",
        "gps",
        "exiftool-refuses",
        "Error: [minor] IFD0 pointer references previous IFD0 directory - <file>",
    ),
    (
        "png-two-exif-same-tag-reversed",
        "gps",
        "exiftool-refuses",
        "Error: [minor] IFD0 pointer references previous IFD0 directory - <file>",
    ),
    (
        "png-two-exif-same-tag",
        "gps",
        "exiftool-refuses",
        "Error: [minor] IFD0 pointer references previous IFD0 directory - <file>",
    ),
    (
        "webp-exif-16-then-gps",
        "gps",
        "we-refuse",
        "Uneditable(TooManyExifSegments)",
    ),
    (
        "webp-exif-bad-then-good",
        "gps",
        "we-refuse",
        "Uneditable(BlockNotClean)",
    ),
    (
        "webp-exif-empty",
        "gps",
        "we-refuse",
        "Uneditable(BlockNotClean)",
    ),
    (
        "webp-exif-garbage",
        "gps",
        "we-refuse",
        "Uneditable(BlockNotClean)",
    ),
    (
        "webp-exif-good-then-bad",
        "gps",
        "we-refuse",
        "Uneditable(BlockNotClean)",
    ),
    (
        "webp-exif-odd-unpadded-last",
        "all",
        "exiftool-refuses",
        "Error: Truncated 'EXIF' chunk - <file>",
    ),
    (
        "webp-exif-odd-unpadded-last",
        "gps",
        "exiftool-refuses",
        "Error: Truncated 'EXIF' chunk - <file>",
    ),
    (
        "webp-exif-one-byte",
        "gps",
        "we-refuse",
        "Uneditable(BlockNotClean)",
    ),
    (
        "webp-exif-tiff-8-bytes",
        "gps",
        "we-refuse",
        "Uneditable(BlockNotClean)",
    ),
    (
        "webp-header-20-exif-size-0",
        "gps",
        "we-refuse",
        "Uneditable(BlockNotClean)",
    ),
    (
        "webp-null-chunk-before-exif",
        "all",
        "exiftool-refuses",
        "Error: Encountered empty null chunk. Processing aborted - <file>",
    ),
    (
        "webp-null-chunk-before-exif",
        "gps",
        "exiftool-refuses",
        "Error: Encountered empty null chunk. Processing aborted - <file>",
    ),
    (
        "webp-null-chunk-tail",
        "all",
        "exiftool-refuses",
        "Error: Encountered empty null chunk. Processing aborted - <file>",
    ),
    (
        "webp-null-chunk-tail",
        "gps",
        "exiftool-refuses",
        "Error: Encountered empty null chunk. Processing aborted - <file>",
    ),
    (
        "webp-partial-header-5",
        "all",
        "exiftool-refuses",
        "Error: RIFF format error - <file>",
    ),
    (
        "webp-partial-header-5",
        "gps",
        "exiftool-refuses",
        "Error: RIFF format error - <file>",
    ),
    (
        "webp-partial-header-7",
        "all",
        "exiftool-refuses",
        "Error: RIFF format error - <file>",
    ),
    (
        "webp-partial-header-7",
        "gps",
        "exiftool-refuses",
        "Error: RIFF format error - <file>",
    ),
    (
        "webp-tail-1-bytes",
        "all",
        "exiftool-refuses",
        "Error: RIFF format error - <file>",
    ),
    (
        "webp-tail-1-bytes",
        "gps",
        "exiftool-refuses",
        "Error: RIFF format error - <file>",
    ),
    (
        "webp-tail-3-bytes",
        "all",
        "exiftool-refuses",
        "Error: RIFF format error - <file>",
    ),
    (
        "webp-tail-3-bytes",
        "gps",
        "exiftool-refuses",
        "Error: RIFF format error - <file>",
    ),
    (
        "webp-vp8x-len-3",
        "all",
        "exiftool-refuses",
        "Error: Truncated VP8X chunk - <file>",
    ),
    (
        "webp-vp8x-len-3",
        "gps",
        "exiftool-refuses",
        "Error: Truncated VP8X chunk - <file>",
    ),
    (
        "webp-vp8x-short-then-full",
        "all",
        "exiftool-refuses",
        "Error: Truncated VP8X chunk - <file>",
    ),
    (
        "webp-vp8x-short-then-full",
        "gps",
        "exiftool-refuses",
        "Error: Truncated VP8X chunk - <file>",
    ),
    (
        "webp-xmp-odd-unpadded-last",
        "all",
        "exiftool-refuses",
        "Error: Truncated 'XMP ' chunk - <file>",
    ),
    (
        "webp-xmp-odd-unpadded-last",
        "gps",
        "exiftool-refuses",
        "Error: Truncated RIFF chunk - <file>",
    ),
];
