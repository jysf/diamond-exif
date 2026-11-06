//! The mutated-copy apparatus for the red-proofs (SPEC-008), written ONCE.
//!
//! A red-proof of the ExifTool comparison needs a library with a known fault in
//! it. This file stages a throw-away crate (a copy of `src/` plus a generated
//! `probe` binary), optionally lets the caller inject a one-expression fault
//! into the COPY, builds it with the same cargo that is running the tests, runs
//! it on a fixture and returns what the (possibly faulty) library read. The
//! probe renders the readings through the real `tests/support/oracle.rs`
//! (`read_ours`), copied into the crate unmodified, so a mutant is compared
//! through exactly the normalisation the live oracle uses.
//!
//! Provenance: edited port of irradiance `tests/plane_oracle.rs:626-805`
//! (`TempDir`, `copy_dir_recursive`, `inject_*_fault`, `stage_probe_crate`,
//! `build_and_run_probe`; DEC-003, class 3, same author). Edits: the DNG/plane
//! specifics and the MD5 probe are gone, the injector is a closure over the
//! staged `src/` directory, the needle guard is a shared function
//! ([`inject_replace`]), the crate is a minimal generated manifest instead of a
//! copy of the real one, the build is debug, `--offline`, and has its own
//! target directory, and the probe prints oracle readings. See
//! `docs/provenance-ledger.md`.
//!
//! SPEC-011 adds a JPEG mode (a file that starts `FF D8` is read through
//! `exif_core::jpeg::read` and `oracle::read_jpeg_all`: the 20 Exif selectors
//! through the harness's several-Exif model, then the two region presences) and
//! a wall-clock limit on the build and on the run (a faulty library may loop; a
//! timeout is a typed error and a test failure, never a hang).
//!
//! Test support only (`#[path]`-included by `tests/mutation_oracle.rs`, which
//! also includes `oracle.rs`). std only.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Environment variable that overrides the wall-clock limit of a probe BUILD.
pub const BUILD_LIMIT_VAR: &str = "DIAMOND_EXIF_PROBE_BUILD_SECS";
/// Environment variable that overrides the wall-clock limit of a probe RUN.
pub const RUN_LIMIT_VAR: &str = "DIAMOND_EXIF_PROBE_RUN_SECS";
/// Default limit for a probe build (a cold debug build of the dependency-free
/// crate is about 1 s; this is generous for a loaded CI host).
pub const DEFAULT_BUILD_SECS: u64 = 600;
/// Default limit for one probe run on one file.
pub const DEFAULT_RUN_SECS: u64 = 30;

/// The limit in seconds for a raw environment value: a positive integer
/// overrides `default`; anything else (unset, empty, zero, not a number) is the
/// default, so a bad value can never disable the limit. Pure, so the override is
/// testable without touching the process environment.
pub fn parse_limit_secs(raw: Option<&str>, default: u64) -> u64 {
    match raw.map(str::trim).map(str::parse::<u64>) {
        Some(Ok(n)) if n > 0 => n,
        _ => default,
    }
}

fn build_limit() -> Duration {
    Duration::from_secs(parse_limit_secs(
        std::env::var(BUILD_LIMIT_VAR).ok().as_deref(),
        DEFAULT_BUILD_SECS,
    ))
}

fn run_limit() -> Duration {
    Duration::from_secs(parse_limit_secs(
        std::env::var(RUN_LIMIT_VAR).ok().as_deref(),
        DEFAULT_RUN_SECS,
    ))
}

/// Run `cmd` (stdin closed, stdout and stderr captured) and wait at most
/// `limit`. The same kill-and-reap shape as `tools::run_tool_with_limit`: both
/// pipes are drained on their own threads (a child that fills a pipe cannot
/// deadlock the poll), a child still running at the limit is killed and then
/// reaped, and the reader threads are not joined on that path (a grandchild that
/// kept a pipe open cannot make the timeout itself hang). A timeout is
/// `Err(.. "killed" ..)`.
fn run_with_limit(mut cmd: Command, what: &str, limit: Duration) -> Result<Output, String> {
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn {what}: {e}"))?;
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
                return Err(format!("wait for {what}: {e}"));
            }
        }
        if started.elapsed() >= limit {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "{what}: still running after {} ms and killed (limits: {BUILD_LIMIT_VAR} \
                 default {DEFAULT_BUILD_SECS} s, {RUN_LIMIT_VAR} default {DEFAULT_RUN_SECS} s); \
                 this is a failure, not a hang",
                limit.as_millis()
            ));
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

/// Every injected replacement carries this, so a leaked injection is
/// greppable. `inject_replace` refuses a replacement without it.
pub const MARKER: &str = "/* RED-PROOF INJECTION */";

/// A directory removed on drop, even if the test panics first (mirrors
/// `scripts/lint-red-proof.sh`'s `trap cleanup EXIT`). Unique per process,
/// per call and per label, under the OS temp directory, never in the repo.
pub struct TempDir(PathBuf);

static NEXT: AtomicU64 = AtomicU64::new(0);

impl TempDir {
    pub fn new(label: &str) -> TempDir {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "diamond-exif-probe-{label}-{}-{n}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir)
            .unwrap_or_else(|e| panic!("create temp dir {}: {e}", dir.display()));
        TempDir(dir)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub fn copy_dir_recursive(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap_or_else(|e| panic!("mkdir {}: {e}", dst.display()));
    for entry in
        std::fs::read_dir(src).unwrap_or_else(|e| panic!("read_dir {}: {e}", src.display()))
    {
        let entry = entry.expect("dir entry");
        let target = dst.join(entry.file_name());
        let file_type = entry.file_type().expect("file_type");
        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target)
                .unwrap_or_else(|e| panic!("copy {}: {e}", target.display()));
        }
    }
}

/// Replace `needle` with `replacement` in the staged file `path`, and PANIC
/// unless the needle occurs exactly once. A moved injection point must fail
/// loudly: a needle that silently matched nothing (or the wrong one of two)
/// would turn the red-proof vacuous. The replacement must carry [`MARKER`].
pub fn inject_replace(path: &Path, needle: &str, replacement: &str) {
    assert!(
        replacement.contains(MARKER),
        "the replacement for `{needle}` must carry the {MARKER} marker"
    );
    let src =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let occurrences = src.matches(needle).count();
    assert_eq!(
        occurrences,
        1,
        "expected exactly one occurrence of `{needle}` in {} (exactly-once needle guard); found \
         {occurrences} -- the injection point moved, update this test",
        path.display()
    );
    std::fs::write(path, src.replacen(needle, replacement, 1))
        .unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The generated probe binary. A file that starts `FF D8` is a JPEG: it is read
/// with `exif_core::jpeg::read` and `oracle::read_jpeg_all` (the Exif selectors,
/// then the two region presences). Anything else is parsed with
/// `exif_core::tiff::Container` and `oracle::read_ours` over `oracle::SELECTORS`.
/// Either way the answer is one tab-separated line, `-` for `None`. A value
/// that would break the line (backslash, tab, newline, CR) or collide with the
/// `-` marker is backslash-escaped; [`unescape`] undoes it. A parse error is
/// reported on stderr with exit status 3 (a faulty library may well fail to
/// parse a file the honest one reads).
const PROBE_MAIN: &str = r#"mod oracle;

use exif_core::tiff::Container;

fn escape(s: &str) -> String {
    if s == "-" {
        return "\\-".to_string();
    }
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            other => out.push(other),
        }
    }
    out
}

fn main() {
    let path = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("usage: probe <file>");
            std::process::exit(2);
        }
    };
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("read {path}: {e}");
            std::process::exit(2);
        }
    };
    let ours = if data.starts_with(&[0xFF, 0xD8]) {
        match exif_core::jpeg::read(&data) {
            Ok(meta) => oracle::read_jpeg_all(&meta),
            Err(e) => {
                eprintln!("PARSE-ERROR {e:?}");
                std::process::exit(3);
            }
        }
    } else {
        match Container::parse(&data) {
            Ok(c) => oracle::read_ours(&c, oracle::SELECTORS),
            Err(e) => {
                eprintln!("PARSE-ERROR {e:?}");
                std::process::exit(3);
            }
        }
    };
    let line: Vec<String> = ours
        .iter()
        .map(|o| match o {
            Some(v) => escape(v),
            None => "-".to_string(),
        })
        .collect();
    println!("{}", line.join("\t"));
}
"#;

/// Undo the probe's escaping of one field.
fn unescape(field: &str) -> Option<String> {
    if field == "-" {
        return None;
    }
    let mut out = String::new();
    let mut chars = field.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('\\') => out.push('\\'),
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('-') => out.push('-'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    Some(out)
}

/// Stage a throw-away crate in `dest`: a minimal manifest for package
/// `exif-core` (no dependencies, same `[lib]`), a copy of the real `src/`,
/// `tests/support/oracle.rs` copied next to the generated probe binary, and then
/// `inject` called with the STAGED `src/` directory (never the real one). The
/// repo tree is only read.
pub fn stage_probe_crate(dest: &Path, inject: Option<&dyn Fn(&Path)>) {
    let root = repo_root();
    let probe_dir = dest.join("src/bin/probe");
    std::fs::create_dir_all(&probe_dir).expect("mkdir src/bin/probe");

    // No `[dependencies]` entries, an explicit `[[bin]]` (not autobins) and an
    // explicit `[lib]`, so a change in the real manifest cannot leak in.
    std::fs::write(
        dest.join("Cargo.toml"),
        "[package]\nname = \"exif-core\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n\
         [lib]\nname = \"exif_core\"\npath = \"src/lib.rs\"\n\n\
         [[bin]]\nname = \"probe\"\npath = \"src/bin/probe/main.rs\"\n\n\
         [dependencies]\n",
    )
    .expect("write staged Cargo.toml");

    let staged_src = dest.join("src");
    copy_dir_recursive(&root.join("src"), &staged_src);
    std::fs::copy(
        root.join("tests/support/oracle.rs"),
        probe_dir.join("oracle.rs"),
    )
    .expect("copy oracle.rs");
    std::fs::write(probe_dir.join("main.rs"), PROBE_MAIN).expect("write probe main.rs");

    if let Some(inject) = inject {
        inject(&staged_src);
    }
}

/// The cargo that is running these tests (`CARGO`, set by `cargo test`),
/// falling back to a bare `cargo`. With rustup, a bare `cargo` on `PATH` can be
/// a different toolchain (Homebrew's) from the one the library gates run with.
fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string())
}

/// Build the staged crate's probe in `dir` (debug, `--offline`, its own target
/// directory under `dir`) and return the binary's path. A stale
/// `CARGO_TARGET_DIR` in the environment cannot redirect the build into the
/// repo's `target/`, because the variable is set explicitly on the child.
pub fn build_probe(dir: &Path) -> Result<PathBuf, String> {
    build_probe_with_limit(dir, build_limit())
}

/// [`build_probe`] with an explicit wall-clock limit; [`build_probe`] is this
/// with the environment's ([`BUILD_LIMIT_VAR`]).
pub fn build_probe_with_limit(dir: &Path, limit: Duration) -> Result<PathBuf, String> {
    let target = dir.join("target");
    let mut cmd = Command::new(cargo());
    cmd.args(["build", "--offline", "--quiet", "--bin", "probe"])
        .current_dir(dir)
        .env("CARGO_TARGET_DIR", &target)
        .env_remove("CARGO_BUILD_TARGET_DIR");
    let build = run_with_limit(cmd, &format!("`{} build` of the probe", cargo()), limit)?;
    if !build.status.success() {
        return Err(format!(
            "cargo build failed ({}) in {}:\n{}",
            build.status,
            dir.display(),
            String::from_utf8_lossy(&build.stderr)
        ));
    }
    Ok(target.join("debug/probe"))
}

/// Run a built probe on `file`: one entry per `oracle::SELECTORS` selector for a
/// TIFF, per `oracle::jpeg_names()` selector for a JPEG. Every failure is a
/// `String` that includes the captured stderr. Bounded by [`RUN_LIMIT_VAR`].
pub fn run_probe(bin: &Path, file: &Path) -> Result<Vec<Option<String>>, String> {
    run_probe_with_limit(bin, file, run_limit())
}

/// [`run_probe`] with an explicit wall-clock limit.
pub fn run_probe_with_limit(
    bin: &Path,
    file: &Path,
    limit: Duration,
) -> Result<Vec<Option<String>>, String> {
    let mut cmd = Command::new(bin);
    cmd.arg(file);
    let run = run_with_limit(cmd, &format!("probe {}", bin.display()), limit)?;
    if !run.status.success() {
        return Err(format!(
            "probe failed ({}) on {}:\n{}",
            run.status,
            file.display(),
            String::from_utf8_lossy(&run.stderr)
        ));
    }
    let stdout = String::from_utf8(run.stdout).map_err(|e| format!("probe stdout: {e}"))?;
    let line = stdout.strip_suffix('\n').unwrap_or(&stdout);
    Ok(line.split('\t').map(unescape).collect())
}

/// Build the staged crate in `dir` and run its probe on `file`.
pub fn build_and_run_probe(dir: &Path, file: &Path) -> Result<Vec<Option<String>>, String> {
    let bin = build_probe(dir)?;
    run_probe(&bin, file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    #[test]
    fn limit_override_is_a_positive_integer_or_the_default() {
        assert_eq!(parse_limit_secs(None, 30), 30);
        assert_eq!(parse_limit_secs(Some("2"), 30), 2);
        assert_eq!(parse_limit_secs(Some(" 7\n"), 600), 7);
        for bad in ["", "0", "-3", "abc", "1.5", "99999999999999999999999"] {
            assert_eq!(parse_limit_secs(Some(bad), 30), 30, "{bad:?}");
        }
        assert_eq!((DEFAULT_BUILD_SECS, DEFAULT_RUN_SECS), (600, 30));
    }

    #[test]
    fn tempdir_is_removed_on_drop_and_on_panic() {
        let kept;
        {
            let t = TempDir::new("drop");
            kept = t.path().to_path_buf();
            std::fs::write(kept.join("f"), b"x").expect("write");
            assert!(kept.is_dir());
        }
        assert!(!kept.exists(), "removed on a normal drop");

        let path_in_panic = std::cell::RefCell::new(PathBuf::new());
        let r = catch_unwind(AssertUnwindSafe(|| {
            let t = TempDir::new("panic");
            *path_in_panic.borrow_mut() = t.path().to_path_buf();
            std::fs::write(t.path().join("f"), b"x").expect("write");
            panic!("a test failing while it owns a TempDir");
        }));
        assert!(r.is_err());
        let p = path_in_panic.borrow().clone();
        assert!(!p.as_os_str().is_empty());
        assert!(!p.exists(), "removed while unwinding from a panic");

        // Unique per call, even with the same label.
        let (a, b) = (TempDir::new("same"), TempDir::new("same"));
        assert_ne!(a.path(), b.path());
        assert!(a.path().starts_with(std::env::temp_dir()));
    }

    fn panic_message(r: std::thread::Result<()>) -> String {
        let payload = r.expect_err("expected a panic");
        payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_string()))
            .unwrap_or_default()
    }

    #[test]
    fn inject_replace_asserts_exactly_one_occurrence() {
        let dir = TempDir::new("inject");
        let file = dir.path().join("f.rs");
        let marked = |s: &str| format!("{s} {MARKER}");

        // Exactly one: replaced, once.
        std::fs::write(&file, "let a = 1; let b = 2;").expect("write");
        inject_replace(&file, "a = 1", &marked("a = 9"));
        assert_eq!(
            std::fs::read_to_string(&file).expect("read"),
            format!("let a = 9 {MARKER}; let b = 2;")
        );

        // Absent: fails loudly and leaves the file alone.
        std::fs::write(&file, "let a = 1;").expect("write");
        let msg = panic_message(catch_unwind(AssertUnwindSafe(|| {
            inject_replace(&file, "zzz", &marked("y"));
        })));
        assert!(
            msg.contains("exactly one") && msg.contains("found 0"),
            "{msg}"
        );
        assert_eq!(std::fs::read_to_string(&file).expect("read"), "let a = 1;");

        // Duplicate: fails loudly and leaves the file alone.
        std::fs::write(&file, "x = 1; x = 1;").expect("write");
        let msg = panic_message(catch_unwind(AssertUnwindSafe(|| {
            inject_replace(&file, "x = 1", &marked("x = 2"));
        })));
        assert!(
            msg.contains("exactly one") && msg.contains("found 2"),
            "{msg}"
        );
        assert_eq!(
            std::fs::read_to_string(&file).expect("read"),
            "x = 1; x = 1;"
        );

        // A replacement without the marker is refused before the file is read.
        let msg = panic_message(catch_unwind(AssertUnwindSafe(|| {
            inject_replace(&file, "x = 1; x = 1;", "unmarked");
        })));
        assert!(msg.contains("marker"), "{msg}");
    }

    #[test]
    fn stage_probe_crate_does_not_touch_the_repo_tree() {
        let root = repo_root();
        let watched = [
            "src/lib.rs",
            "src/reader.rs",
            "src/tiff/mod.rs",
            "src/tiff/types.rs",
            "Cargo.toml",
            "tests/support/oracle.rs",
        ];
        let read_all = || -> Vec<Vec<u8>> {
            watched
                .iter()
                .map(|p| std::fs::read(root.join(p)).expect("read watched file"))
                .collect()
        };
        let before = read_all();
        let target_before = root
            .join("target")
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok());

        let dir = TempDir::new("stage");
        assert!(!dir.path().starts_with(&root), "staged outside the repo");
        let inject = |src: &Path| {
            inject_replace(
                &src.join("reader.rs"),
                "ByteOrder::Big => u16::from_be_bytes(bytes),",
                &format!("ByteOrder::Big => u16::from_le_bytes(bytes), {MARKER}"),
            );
        };
        stage_probe_crate(dir.path(), Some(&inject));

        // The staged copy carries the injection; the real tree does not.
        let staged = std::fs::read_to_string(dir.path().join("src/reader.rs")).expect("read");
        assert!(staged.contains(MARKER));
        assert_eq!(read_all(), before, "the repo's files were not modified");
        assert!(!String::from_utf8_lossy(&before[1]).contains(MARKER));
        assert!(dir.path().join("Cargo.toml").is_file());
        assert!(dir.path().join("src/bin/probe/main.rs").is_file());
        assert!(dir.path().join("src/bin/probe/oracle.rs").is_file());
        // Staging builds nothing, so the repo's target/ is untouched too.
        let target_after = root
            .join("target")
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok());
        assert_eq!(target_before, target_after);
        // No dependencies in the staged manifest.
        let manifest = std::fs::read_to_string(dir.path().join("Cargo.toml")).expect("read");
        assert!(
            manifest.trim_end().ends_with("[dependencies]"),
            "{manifest}"
        );
    }

    #[test]
    fn escaping_round_trips() {
        for v in [
            "plain",
            "",
            "a\tb",
            "line\nbreak",
            "back\\slash",
            "-",
            "\\-",
            "x\r",
        ] {
            // Mirror of the probe's escape (kept in sync by this test).
            let escaped = if v == "-" {
                "\\-".to_string()
            } else {
                v.replace('\\', "\\\\")
                    .replace('\t', "\\t")
                    .replace('\n', "\\n")
                    .replace('\r', "\\r")
            };
            assert_eq!(unescape(&escaped).as_deref(), Some(v), "{v:?}");
        }
        assert_eq!(unescape("-"), None);
    }
}
