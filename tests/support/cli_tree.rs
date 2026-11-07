//! SPEC-016: helpers for `tests/cli.rs`, `std` only. A unique temporary tree
//! that removes itself (also when a test fails), a runner for the real binary
//! with a hard time limit, a peak-memory runner, and the loud-skip policy.
//! Included with `#[path = ...] mod cli_tree;`.
#![allow(dead_code)]

use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

/// The real binary.
pub const BIN: &str = env!("CARGO_BIN_EXE_exif-audit");

/// Default hard limit for one run of the binary.
pub const LIMIT: Duration = Duration::from_secs(60);

/// A unique temp directory, removed on drop.
pub struct Tree {
    pub root: PathBuf,
}

impl Tree {
    pub fn new(tag: &str) -> Tree {
        static N: AtomicU32 = AtomicU32::new(0);
        let root = std::env::temp_dir().join(format!(
            "exif-audit-cli-{}-{}-{tag}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("create the temp tree");
        Tree { root }
    }

    pub fn path(&self, rel: &str) -> PathBuf {
        self.root.join(rel)
    }

    /// Write `bytes` at `rel`, creating parents.
    pub fn write(&self, rel: &str, bytes: &[u8]) -> PathBuf {
        let p = self.path(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).expect("create parents");
        }
        fs::write(&p, bytes).expect("write a file");
        p
    }

    pub fn mkdir(&self, rel: &str) -> PathBuf {
        let p = self.path(rel);
        fs::create_dir_all(&p).expect("create a directory");
        p
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// What a run gave.
#[derive(Debug)]
pub struct Out {
    /// `None` when the process died of a signal.
    pub code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl Out {
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }

    pub fn err_text(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }
}

/// A run of the binary.
pub struct Cmd {
    args: Vec<OsString>,
    cwd: Option<PathBuf>,
    limit: Duration,
}

impl Cmd {
    pub fn new() -> Cmd {
        Cmd {
            args: Vec::new(),
            cwd: None,
            limit: LIMIT,
        }
    }

    pub fn args<I, S>(mut self, args: I) -> Cmd
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.args
            .extend(args.into_iter().map(|a| a.as_ref().to_os_string()));
        self
    }

    pub fn cwd(mut self, cwd: &Path) -> Cmd {
        self.cwd = Some(cwd.to_path_buf());
        self
    }

    pub fn limit(mut self, limit: Duration) -> Cmd {
        self.limit = limit;
        self
    }

    fn command(&self, program: &OsStr) -> Command {
        let mut c = Command::new(program);
        c.args(&self.args);
        if let Some(cwd) = &self.cwd {
            c.current_dir(cwd);
        }
        c
    }

    /// Run the binary; kill it and fail the test after the limit.
    pub fn run(self) -> Out {
        let c = self.command(OsStr::new(BIN));
        run_command(c, self.limit)
    }

    /// Run the binary under `/usr/bin/time` and return the child's peak
    /// resident memory in bytes, or `None` when that tool is absent (macOS:
    /// `-l`, Linux: `-v`).
    pub fn run_measured(self) -> (Out, Option<u64>) {
        let time = Path::new("/usr/bin/time");
        if !time.is_file() {
            return (self.run(), None);
        }
        let flag = if cfg!(target_os = "macos") { "-l" } else { "-v" };
        let mut c = Command::new(time);
        c.arg(flag).arg(BIN).args(&self.args);
        if let Some(cwd) = &self.cwd {
            c.current_dir(cwd);
        }
        let out = run_command(c, self.limit);
        let peak = peak_from(&out.err_text());
        (out, peak)
    }
}

/// `maximum resident set size` in bytes (macOS prints bytes) or `Maximum
/// resident set size (kbytes)` (Linux).
fn peak_from(stderr: &str) -> Option<u64> {
    for line in stderr.lines() {
        let l = line.trim();
        let lower = l.to_ascii_lowercase();
        if lower.contains("maximum resident set size (kbytes)") {
            let n: u64 = l.rsplit(':').next()?.trim().parse().ok()?;
            return n.checked_mul(1024);
        }
        if lower.ends_with("maximum resident set size") {
            let n: u64 = l.split_whitespace().next()?.parse().ok()?;
            return Some(n);
        }
    }
    None
}

/// Spawn, read both pipes on threads, wait up to `limit`, kill on overrun.
pub fn run_command(mut c: Command, limit: Duration) -> Out {
    c.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = c.spawn().expect("spawn the binary");
    let mut so = child.stdout.take().expect("stdout");
    let mut se = child.stderr.take().expect("stderr");
    let t1 = std::thread::spawn(move || {
        let mut v = Vec::new();
        let _ = so.read_to_end(&mut v);
        v
    });
    let t2 = std::thread::spawn(move || {
        let mut v = Vec::new();
        let _ = se.read_to_end(&mut v);
        v
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait().expect("wait") {
            Some(s) => break s,
            None if start.elapsed() > limit => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("the binary ran longer than {limit:?} and was killed (a hang)");
            }
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    };
    Out {
        code: status.code(),
        stdout: t1.join().expect("stdout thread"),
        stderr: t2.join().expect("stderr thread"),
    }
}

/// A skip is loud; `DIAMOND_EXIF_REQUIRE_CLI_FS=1` turns it into a failure.
pub fn skip(reason: &str) {
    println!("SKIP: {reason}");
    assert!(
        std::env::var("DIAMOND_EXIF_REQUIRE_CLI_FS").as_deref() != Ok("1"),
        "DIAMOND_EXIF_REQUIRE_CLI_FS=1 and a filesystem feature is unavailable: {reason}"
    );
}
