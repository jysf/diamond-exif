//! ExifTool as an ORACLE (SPEC-005): it is run as a tool, never linked, and its
//! source is never read (`exiftool-never-linked`). Output is parsed with no
//! dependency: `exiftool -T -n -s3` prints one tab-separated line, one field per
//! requested selector, in the order requested, `-` for a tag it does not report.
//!
//! Provenance: edited port of irradiance `tests/support/tools.rs` (`ToolError`,
//! `run_tool`, `exiftool_available`, `run_exiftool`, `parse_fields`); the DNG
//! typed-reading layer and the dnglab wrapper were dropped. See
//! `docs/provenance-ledger.md` (DEC-003).
//!
//! Test support only: `#[path]`-included by `tests/metadata_oracle.rs` and by
//! `examples/oracle_record.rs`; never compiled into the library. std only.
//!
//! The exit status is never inspected anywhere in this file. Measured with
//! ExifTool 13.55 over the 29 committed `tiff_ifd` seeds (garbage included):
//! exit status 0 every time, so only stdout carries signal.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Why a tool reading could not be produced.
#[derive(Debug)]
pub enum ToolError {
    /// The tool did not resolve on `PATH` (`ErrorKind::NotFound` from the spawn
    /// itself, the OS's own signal rather than a probe).
    NotOnPath(&'static str),
    /// The tool could not be run (a spawn failure that was not `NotFound`).
    Exec { tool: &'static str, message: String },
    /// The tool's stdout did not have the expected shape.
    Parse { tool: &'static str, message: String },
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolError::NotOnPath(tool) => write!(f, "{tool} is not on PATH"),
            ToolError::Exec { tool, message } => write!(f, "{tool}: {message}"),
            ToolError::Parse { tool, message } => {
                write!(f, "{tool}: could not parse its output: {message}")
            }
        }
    }
}

/// The single choke point for spawning a tool: `bin args... path`. A spawn
/// failure of kind `NotFound` becomes [`ToolError::NotOnPath`], anything else
/// [`ToolError::Exec`]. ExifTool is spawned nowhere else.
pub fn run_tool(bin: &'static str, args: &[String], path: &Path) -> Result<Output, ToolError> {
    Command::new(bin)
        .args(args)
        .arg(path)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ToolError::NotOnPath(bin)
            } else {
                ToolError::Exec {
                    tool: bin,
                    message: e.to_string(),
                }
            }
        })
}

/// `exiftool -ver`, trimmed, or the reason it could not be had.
pub fn exiftool_version() -> Result<String, ToolError> {
    let output = run_tool("exiftool", &["-ver".to_string()], Path::new(""))?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Whether ExifTool can be run at all.
pub fn exiftool_available() -> bool {
    exiftool_version().is_ok()
}

/// The argument list for `-T -n -s3 -<selector>...` (the file path is added by
/// [`run_tool`]).
pub fn exiftool_args(selectors: &[&str]) -> Vec<String> {
    let mut args: Vec<String> = vec!["-T".into(), "-n".into(), "-s3".into()];
    args.extend(selectors.iter().map(|s| format!("-{s}")));
    args
}

/// The command line as a human-readable string, with `<file>` for the path.
pub fn exiftool_command_line(selectors: &[&str]) -> String {
    format!("exiftool {} <file>", exiftool_args(selectors).join(" "))
}

/// Run `exiftool -T -n -s3 <selector>... <path>`; one entry per selector, in
/// order, `None` for `-`. Only stdout is trusted, never the exit status.
pub fn run_exiftool(path: &Path, selectors: &[&str]) -> Result<Vec<Option<String>>, ToolError> {
    let stdout = run_exiftool_raw(path, selectors)?;
    parse_fields(selectors, &stdout)
}

/// ExifTool's stdout, verbatim (lossy UTF-8). The recorder stores this.
pub fn run_exiftool_raw(path: &Path, selectors: &[&str]) -> Result<String, ToolError> {
    let output = run_tool("exiftool", &exiftool_args(selectors), path)?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// A committed record: `#` header lines (command, version, source), then
/// ExifTool's stdout verbatim.
pub fn record_text(selectors: &[&str], version: &str, source: &str, stdout: &str) -> String {
    format!(
        "# command: {}\n# exiftool-version: {version}\n# source: {source}\n{stdout}",
        exiftool_command_line(selectors)
    )
}

/// A record for an input ExifTool is deliberately NOT run on (it does not
/// return; see `KNOWN_DIVERGENCES`): same header plus an `# outcome:` line
/// stating what was measured, and no output line.
pub fn record_text_not_run(
    selectors: &[&str],
    version: &str,
    source: &str,
    outcome: &str,
) -> String {
    format!(
        "# command: {}\n# exiftool-version: {version}\n# source: {source}\n# outcome: {outcome}\n",
        exiftool_command_line(selectors)
    )
}

/// A parsed record file.
#[derive(Debug)]
pub struct Record {
    pub command: String,
    pub version: String,
    pub source: String,
    /// Set only for an input ExifTool is not run on: what was measured.
    pub outcome: Option<String>,
    /// ExifTool's stdout as recorded (everything after the `#` header).
    pub stdout: String,
}

/// Parse a record file written by [`record_text`]. `None` if a header line is
/// missing.
pub fn parse_record(text: &str) -> Option<Record> {
    let mut command = None;
    let mut version = None;
    let mut source = None;
    let mut outcome = None;
    let mut rest = text;
    while let Some(after) = rest.strip_prefix("# ") {
        let (line, tail) = after.split_once('\n')?;
        if let Some(v) = line.strip_prefix("command: ") {
            command = Some(v.to_string());
        } else if let Some(v) = line.strip_prefix("exiftool-version: ") {
            version = Some(v.to_string());
        } else if let Some(v) = line.strip_prefix("source: ") {
            source = Some(v.to_string());
        } else if let Some(v) = line.strip_prefix("outcome: ") {
            outcome = Some(v.to_string());
        }
        rest = tail;
    }
    Some(Record {
        command: command?,
        version: version?,
        source: source?,
        outcome,
        stdout: rest.to_string(),
    })
}

/// Parse one `-T -n -s3` output line into one entry per selector. Pure: the
/// same code reads live output and committed records. Splits on tabs, requires
/// exactly one field per selector, maps `-` to `None`. Never sees an exit
/// status.
pub fn parse_fields(selectors: &[&str], stdout: &str) -> Result<Vec<Option<String>>, ToolError> {
    // Only the line terminator is dropped: a trailing tab is an (empty) field.
    let line = stdout.strip_suffix('\n').unwrap_or(stdout);
    let line = line.strip_suffix('\r').unwrap_or(line);
    let raw: Vec<&str> = line.split('\t').collect();
    if raw.len() != selectors.len() || stdout.is_empty() {
        return Err(ToolError::Parse {
            tool: "exiftool",
            message: format!(
                "expected {} tab-separated field(s) for {selectors:?}, got {}: {line:?}",
                selectors.len(),
                if stdout.is_empty() { 0 } else { raw.len() }
            ),
        });
    }
    Ok(raw
        .into_iter()
        .map(|f| if f == "-" { None } else { Some(f.to_string()) })
        .collect())
}

/// Write `bytes` to a per-process temp file named after `name` and return its
/// path (ExifTool reads files, not stdin buffers, here). Callers remove it.
pub fn temp_file_for(name: &str, bytes: &[u8]) -> std::io::Result<PathBuf> {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "diamond-exif-oracle-{}-{name}.tif",
        std::process::id()
    ));
    std::fs::write(&path, bytes)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_fields_maps_dash_to_none_and_keeps_order() {
        let got = parse_fields(&["a", "b", "c"], "x\t-\tz\n").expect("parses");
        assert_eq!(got, vec![Some("x".into()), None, Some("z".into())]);
    }

    #[test]
    fn parse_fields_rejects_a_field_count_mismatch() {
        assert!(matches!(
            parse_fields(&["a", "b", "c"], "x\ty\n"),
            Err(ToolError::Parse { .. })
        ));
        assert!(matches!(
            parse_fields(&["a", "b", "c"], ""),
            Err(ToolError::Parse { .. })
        ));
        assert!(matches!(
            parse_fields(&["a", "b", "c"], "\n"),
            Err(ToolError::Parse { .. })
        ));
    }

    #[test]
    fn run_tool_reports_a_missing_binary_as_not_on_path() {
        let err = run_tool(
            "diamond-exif-no-such-binary-xyz",
            &[],
            Path::new("whatever"),
        )
        .expect_err("no such program");
        assert!(matches!(err, ToolError::NotOnPath(_)), "got {err:?}");
    }
}
