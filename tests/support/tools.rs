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
//! The exit status is not inspected anywhere in this file; only stdout is
//! trusted. Measured (verifier, ExifTool 13.55, a time limit on each run) with
//! `exiftool -T -n -s3 -IFD0:ImageWidth -IFD0:ImageHeight -IFD0:Orientation
//! <seed>` over the 29 committed `tiff_ifd` seeds: `ifd0-offset-zero` and
//! `truncated-header` exited 1, `two-hop-cycle` did not return (see
//! `KNOWN_DIVERGENCES`), and the other 26 exited 0.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

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
    /// The tool was still running when the wall-clock limit expired and was
    /// killed (SPEC-008). A failure, never a skip: a tool that does not return
    /// is a finding.
    Timeout { tool: &'static str, secs: u64 },
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolError::NotOnPath(tool) => write!(f, "{tool} is not on PATH"),
            ToolError::Exec { tool, message } => write!(f, "{tool}: {message}"),
            ToolError::Parse { tool, message } => {
                write!(f, "{tool}: could not parse its output: {message}")
            }
            ToolError::Timeout { tool, secs } => write!(
                f,
                "{tool}: still running after {secs} s and killed (limit from {TIMEOUT_VAR}, \
                 default {DEFAULT_TIMEOUT_SECS} s); this is a failure, not a skip"
            ),
        }
    }
}

/// Environment variable that overrides the per-spawn wall-clock limit.
pub const TIMEOUT_VAR: &str = "DIAMOND_EXIF_TOOL_TIMEOUT_SECS";

/// The default wall-clock limit for one tool run.
pub const DEFAULT_TIMEOUT_SECS: u64 = 5;

/// The limit in seconds for a raw `DIAMOND_EXIF_TOOL_TIMEOUT_SECS` value: a
/// positive integer overrides the default; anything else (unset, empty, zero,
/// not a number) is the default. Pure, so the override is testable without
/// touching the process environment.
pub fn parse_timeout_secs(raw: Option<&str>) -> u64 {
    match raw.map(str::trim).map(str::parse::<u64>) {
        Some(Ok(n)) if n > 0 => n,
        _ => DEFAULT_TIMEOUT_SECS,
    }
}

/// The limit currently in force (reads [`TIMEOUT_VAR`]).
pub fn timeout_secs() -> u64 {
    parse_timeout_secs(std::env::var(TIMEOUT_VAR).ok().as_deref())
}

/// The single choke point for spawning a tool: `bin args... path`, with the
/// wall-clock limit from [`timeout_secs`] (default 5 s, override
/// `DIAMOND_EXIF_TOOL_TIMEOUT_SECS`). A spawn failure of kind `NotFound`
/// becomes [`ToolError::NotOnPath`], anything else [`ToolError::Exec`]; a run
/// that outlives the limit is killed and becomes [`ToolError::Timeout`].
/// ExifTool is spawned nowhere else.
///
/// The limit bounds TIME, and so bounds memory growth only approximately.
/// Measured (ExifTool 13.55, macOS, `exiftool -T -n -s3 ... two-hop-cycle`,
/// SPEC-005 verify): the process was at about 9 GB resident after 5 s. It is
/// defence in depth; the harness still never runs ExifTool on that seed.
pub fn run_tool(bin: &'static str, args: &[String], path: &Path) -> Result<Output, ToolError> {
    run_tool_with_limit(bin, args, path, Duration::from_secs(timeout_secs()))
}

/// [`run_tool`] with an explicit limit; [`run_tool`] is this with the
/// environment's. Still the one place a tool process is spawned.
pub fn run_tool_with_limit(
    bin: &'static str,
    args: &[String],
    path: &Path,
    limit: Duration,
) -> Result<Output, ToolError> {
    let mut child = Command::new(bin)
        .args(args)
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ToolError::NotOnPath(bin)
            } else {
                ToolError::Exec {
                    tool: bin,
                    message: e.to_string(),
                }
            }
        })?;

    // Drain both pipes on their own threads while this one polls, so a tool
    // that writes more than a pipe buffer cannot deadlock against the poll.
    let out = drain(child.stdout.take());
    let err = drain(child.stderr.take());

    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ToolError::Exec {
                    tool: bin,
                    message: e.to_string(),
                });
            }
        }
        if started.elapsed() >= limit {
            // Kill, then reap: no zombie, no process left behind. The reader
            // threads are not joined here, so a grandchild that kept a pipe
            // open cannot make the timeout itself hang.
            let _ = child.kill();
            let _ = child.wait();
            return Err(ToolError::Timeout {
                tool: bin,
                secs: limit.as_secs().max(1),
            });
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    Ok(Output {
        status,
        stdout: out.join().unwrap_or_default(),
        stderr: err.join().unwrap_or_default(),
    })
}

fn drain<R: std::io::Read + Send + 'static>(pipe: Option<R>) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut buf);
        }
        buf
    })
}

/// `exiftool -ver`, trimmed, or the reason it could not be had.
pub fn exiftool_version() -> Result<String, ToolError> {
    let output = run_tool("exiftool", &["-ver".to_string()], Path::new(""))?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Whether ExifTool can be run at all.
pub fn exiftool_available() -> bool {
    match exiftool_version() {
        Ok(_) => true,
        // Present but not returning is a failure with a clear message, not a skip.
        Err(e @ ToolError::Timeout { .. }) => panic!("exiftool -ver: {e}"),
        Err(_) => false,
    }
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

    /// A slow tool that records its own pid in the file named by the path
    /// argument (`sh` sees it as `$0`), then `exec`s `sleep` so the pid is the
    /// sleeper's: killing the child leaves nothing behind.
    #[cfg(unix)]
    fn slow_tool_args() -> Vec<String> {
        vec!["-c".into(), "echo $$ > \"$0\"; exec sleep 30".into()]
    }

    #[cfg(unix)]
    #[test]
    fn run_tool_times_out_and_kills_the_child() {
        let pidfile = std::env::temp_dir().join(format!(
            "diamond-exif-timeout-test-{}.pid",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&pidfile);

        let started = Instant::now();
        let err = run_tool_with_limit("sh", &slow_tool_args(), &pidfile, Duration::from_secs(1))
            .expect_err("a 30 s sleep must not finish inside a 1 s limit");
        let elapsed = started.elapsed();

        assert!(
            matches!(
                err,
                ToolError::Timeout {
                    tool: "sh",
                    secs: 1
                }
            ),
            "got {err:?}"
        );
        assert!(
            err.to_string().contains("killed") && err.to_string().contains(TIMEOUT_VAR),
            "{err}"
        );
        assert!(
            elapsed >= Duration::from_secs(1) && elapsed < Duration::from_secs(10),
            "returned after {elapsed:?}, expected about 1 s"
        );

        // No process left behind: the recorded pid no longer exists. `kill -0`
        // sends no signal and fails when the pid is gone; it goes through
        // `run_tool_with_limit` like every other spawn here.
        let pid = std::fs::read_to_string(&pidfile)
            .expect("the child wrote its pid before it was killed")
            .trim()
            .to_string();
        let _ = std::fs::remove_file(&pidfile);
        assert!(
            pid.chars().all(|c| c.is_ascii_digit()) && !pid.is_empty(),
            "{pid:?}"
        );
        let probe = run_tool_with_limit(
            "kill",
            &["-0".to_string()],
            Path::new(&pid),
            Duration::from_secs(5),
        )
        .expect("kill -0 runs");
        assert!(
            !probe.status.success(),
            "pid {pid} is still alive after the timeout"
        );
    }

    #[test]
    fn timeout_variable_overrides_the_default() {
        assert_eq!(parse_timeout_secs(None), DEFAULT_TIMEOUT_SECS);
        assert_eq!(DEFAULT_TIMEOUT_SECS, 5);
        assert_eq!(parse_timeout_secs(Some("1")), 1);
        assert_eq!(parse_timeout_secs(Some(" 30\n")), 30);
        // Unusable values fall back to the default rather than disabling the limit.
        for bad in ["", "0", "-3", "abc", "1.5", "99999999999999999999999"] {
            assert_eq!(
                parse_timeout_secs(Some(bad)),
                DEFAULT_TIMEOUT_SECS,
                "{bad:?}"
            );
        }
    }

    /// The same override, end to end: a child copy of this test binary with the
    /// variable set to 1 sends a 30 s sleep through `run_tool` and must get
    /// `Timeout { secs: 1 }`. The inner test is `#[ignore]` so a normal run
    /// does not execute it (and so no test mutates this process's environment).
    #[cfg(unix)]
    #[test]
    fn timeout_variable_is_honoured_by_run_tool() {
        let exe = std::env::current_exe().expect("current_exe");
        let started = Instant::now();
        let out = std::process::Command::new(exe)
            .args([
                "--ignored",
                "--exact",
                "tools::tests::env_limit_child",
                "--nocapture",
            ])
            .env(TIMEOUT_VAR, "1")
            .output()
            .expect("spawn the test binary");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(out.status.success(), "child failed:\n{stdout}");
        assert!(stdout.contains("TIMEOUT-OK secs=1"), "{stdout}");
        assert!(started.elapsed() < Duration::from_secs(20));
    }

    #[cfg(unix)]
    #[test]
    #[ignore = "run by timeout_variable_is_honoured_by_run_tool with the variable set"]
    fn env_limit_child() {
        let args = vec!["-c".to_string(), "exec sleep 30".to_string()];
        match run_tool("sh", &args, Path::new("unused")) {
            Err(ToolError::Timeout { secs, .. }) => println!("TIMEOUT-OK secs={secs}"),
            other => panic!("expected a timeout, got {other:?}"),
        }
    }

    #[test]
    fn run_tool_returns_a_fast_tools_output_unchanged() {
        let out = run_tool_with_limit(
            "sh",
            &[
                "-c".to_string(),
                "printf 'a\\tb\\n'; printf err >&2".to_string(),
            ],
            Path::new("ignored"),
            Duration::from_secs(10),
        );
        #[cfg(unix)]
        {
            let out = out.expect("sh runs");
            assert_eq!(out.stdout, b"a\tb\n");
            assert_eq!(out.stderr, b"err");
            assert!(out.status.success());
        }
        #[cfg(not(unix))]
        let _ = out;
    }
}
