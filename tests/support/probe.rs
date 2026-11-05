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
//! Test support only (`#[path]`-included by `tests/mutation_oracle.rs`, which
//! also includes `oracle.rs`). std only.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

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

/// The generated probe binary. It parses its argument with
/// `exif_core::tiff::Container` and prints `oracle::read_ours` over
/// `oracle::SELECTORS` as one tab-separated line, `-` for `None`. A value
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
    let container = match Container::parse(&data) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("PARSE-ERROR {e:?}");
            std::process::exit(3);
        }
    };
    let ours = oracle::read_ours(&container, oracle::SELECTORS);
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
    let target = dir.join("target");
    let build = Command::new(cargo())
        .args(["build", "--offline", "--quiet", "--bin", "probe"])
        .current_dir(dir)
        .env("CARGO_TARGET_DIR", &target)
        .env_remove("CARGO_BUILD_TARGET_DIR")
        .output()
        .map_err(|e| format!("spawn `{} build`: {e}", cargo()))?;
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

/// Run a built probe on `file`: one entry per `oracle::SELECTORS` selector.
/// Every failure is a `String` that includes the captured stderr.
pub fn run_probe(bin: &Path, file: &Path) -> Result<Vec<Option<String>>, String> {
    let run = Command::new(bin)
        .arg(file)
        .output()
        .map_err(|e| format!("spawn {}: {e}", bin.display()))?;
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
