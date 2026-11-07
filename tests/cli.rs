//! SPEC-016: the real `exif-audit` binary, driven as a process (DEC-011).
//!
//! Every test builds its own temporary tree (`std` only) and runs the binary
//! with a relative path from inside it, so labels are what was typed and no
//! absolute path can appear. Expected report bytes come from the library's
//! `render`, over entries built in the test. A filesystem feature the host
//! lacks is a loud `SKIP: <reason>`; `DIAMOND_EXIF_REQUIRE_CLI_FS=1` turns it
//! into a failure (macOS may skip the invalid-UTF-8 file name only).
//!
//! Anything that could hang (a FIFO, a symlink loop) runs under the runner's
//! hard time limit, which kills the child and fails the test.

#[path = "support/cli_tree.rs"]
mod cli_tree;
#[path = "support/report_inputs.rs"]
mod report_inputs;

include!("../fuzz/shared/render_check.rs");

use std::fs;
use std::path::Path;
use std::time::Duration;

use cli_tree::{skip, Cmd, Out, Tree};
use exif_core::audit::audit;
use exif_core::report::{
    render, FailReason, FileEntry, FileResult, Format, RenderOptions,
};
use exif_core::StripPolicy;
use render_check::{naive_safe_text, parse_csv, parse_json, J};

// ── inputs ──────────────────────────────────────────────────────────────────

fn house() -> Vec<u8> {
    report_inputs::full_house_jpeg()
}

fn png() -> Vec<u8> {
    let mut v = b"\x89PNG\r\n\x1a\n".to_vec();
    v.extend_from_slice(&[0; 8]);
    v
}

/// Timestamps only (low).
fn low_only() -> Vec<u8> {
    report_inputs::tiff_with_payloads(&[(0x0132, 2, b"2024:01:01 00:00:00\0")])
}

/// Artist only (medium).
fn medium_only() -> Vec<u8> {
    report_inputs::tiff_with_payloads(&[(0x013B, 2, b"Ann\0")])
}

/// A body serial number only (high).
fn high_only() -> Vec<u8> {
    report_inputs::tiff_with_payloads(&[(0xA431, 2, b"SN1\0")])
}

/// A JPEG whose only segment is an empty application segment: status `issues`.
fn issues_jpeg() -> Vec<u8> {
    vec![0xFF, 0xD8, 0xFF, 0xE1, 0x00, 0x00]
}

/// A JPEG whose Exif segment holds no TIFF: status `unreadable`.
fn unreadable_jpeg() -> Vec<u8> {
    let mut v = vec![0xFF, 0xD8, 0xFF, 0xE1, 0x00, 0x0E];
    v.extend_from_slice(b"Exif\0\0XXXXXX");
    v.extend_from_slice(&[0xFF, 0xD9]);
    v
}

/// A JPEG of exactly `n` bytes (a comment segment pads it).
fn jpeg_of_len(n: usize) -> Vec<u8> {
    let base = report_inputs::jpeg(&[report_inputs::seg(0xFE, &[])]);
    let pad = n.checked_sub(base.len()).expect("n is large enough");
    let v = report_inputs::jpeg(&[report_inputs::seg(0xFE, &vec![b'x'; pad])]);
    assert_eq!(v.len(), n);
    v
}

// ── expected output from the library ────────────────────────────────────────

enum Exp {
    Bytes(Vec<u8>),
    Fail(FailReason, &'static str),
}

fn expect(format: Format, opts: RenderOptions, entries: &[(&str, Exp)]) -> String {
    let list: Vec<FileEntry<'_>> = entries
        .iter()
        .map(|(label, e)| FileEntry {
            label: label.as_bytes(),
            result: match e {
                Exp::Bytes(b) => FileResult::Audited(audit(b)),
                Exp::Fail(reason, detail) => FileResult::Failed {
                    reason: *reason,
                    detail: detail.as_bytes(),
                },
            },
        })
        .collect();
    render(format, &list, &opts)
}

fn default_opts() -> RenderOptions {
    RenderOptions {
        policy: Some(StripPolicy::AllPii),
        inventory: false,
        values: false,
    }
}

fn exit_of(out: &Out) -> i32 {
    out.code
        .unwrap_or_else(|| panic!("killed by a signal; stderr: {}", out.err_text()))
}

fn json_of(out: &Out) -> J {
    parse_json(&out.stdout).unwrap_or_else(|e| panic!("not strict JSON: {e}\n{}", out.text()))
}

/// `(label, status, failure detail)` of every file in a JSON report.
fn files_of(j: &J) -> Vec<(String, String, Option<String>)> {
    j.get("files")
        .and_then(J::as_arr)
        .expect("files")
        .iter()
        .map(|f| {
            (
                f.get("label").and_then(J::as_str).expect("label").to_string(),
                f.get("status").and_then(J::as_str).expect("status").to_string(),
                f.get("failure")
                    .and_then(|x| x.get("detail"))
                    .and_then(J::as_str)
                    .map(str::to_string),
            )
        })
        .collect()
}

fn run_json(t: &Tree, args: &[&str]) -> (Out, Vec<(String, String, Option<String>)>) {
    let mut a = vec!["--json"];
    a.extend_from_slice(args);
    let out = Cmd::new().args(a).cwd(&t.root).run();
    let files = files_of(&json_of(&out));
    (out, files)
}

// ── defaults, bytes, determinism ────────────────────────────────────────────

#[test]
fn defaults_match_dec_011() {
    let t = Tree::new("defaults");
    t.write("dir/house.jpg", &house());
    t.write("dir/pic.png", &png());
    t.write("dir/note.txt", b"just text");
    let out = Cmd::new().args(["dir"]).cwd(&t.root).run();
    // The full-house file has GPS and a serial: high, so --fail-on high exits 1.
    assert_eq!(exit_of(&out), 1, "stderr: {}", out.err_text());
    assert!(out.stderr.is_empty());
    let entries = [
        ("dir/house.jpg", Exp::Bytes(house())),
        ("dir/note.txt", Exp::Bytes(b"just text".to_vec())),
        ("dir/pic.png", Exp::Bytes(png())),
    ];
    // text format, all_pii preview, no values, no inventory.
    let want = expect(Format::Text, default_opts(), &entries);
    assert_eq!(out.text(), want);
    // The expectation is not insensitive to the options it pins.
    for other in [
        RenderOptions { values: true, ..default_opts() },
        RenderOptions { inventory: true, ..default_opts() },
        RenderOptions { policy: None, ..default_opts() },
        RenderOptions { policy: Some(StripPolicy::Gps), ..default_opts() },
    ] {
        assert_ne!(expect(Format::Text, other, &entries), want, "{other:?}");
    }
    assert_ne!(expect(Format::Json, default_opts(), &entries), want);
}

#[test]
fn report_equals_the_library_render_in_every_format() {
    let t = Tree::new("equals");
    t.write("dir/house.jpg", &house());
    t.write("dir/pic.png", &png());
    t.write("dir/b-low.tif", &low_only());
    t.write("dir/a-issues.jpg", &issues_jpeg());
    let entries = || {
        [
            ("dir/a-issues.jpg", Exp::Bytes(issues_jpeg())),
            ("dir/b-low.tif", Exp::Bytes(low_only())),
            ("dir/house.jpg", Exp::Bytes(house())),
            ("dir/pic.png", Exp::Bytes(png())),
            ("missing", Exp::Fail(FailReason::Unreadable, "not found")),
            ("dir/house.jpg", Exp::Bytes(house())),
        ]
    };
    let formats: [(&[&str], Format); 5] = [
        (&["--format", "text"], Format::Text),
        (&["--format=json"], Format::Json),
        (&["--json"], Format::Json),
        (&["--csv"], Format::Csv),
        (&["--format", "csv"], Format::Csv),
    ];
    let option_sets: [(&[&str], RenderOptions); 4] = [
        (&[], default_opts()),
        (&["--policy", "none"], RenderOptions { policy: None, ..default_opts() }),
        (
            &["--policy=gps", "--inventory"],
            RenderOptions { policy: Some(StripPolicy::Gps), inventory: true, values: false },
        ),
        (
            &["--values", "--inventory", "--policy", "everything"],
            RenderOptions { policy: Some(StripPolicy::Everything), inventory: true, values: true },
        ),
    ];
    for (fargs, format) in formats {
        for (oargs, opts) in &option_sets {
            let mut args: Vec<&str> = fargs.to_vec();
            args.extend_from_slice(oargs);
            args.extend_from_slice(&["dir", "missing", "dir/house.jpg"]);
            let out = Cmd::new().args(&args).cwd(&t.root).run();
            // Findings and a failure: incomplete, exit 3.
            assert_eq!(exit_of(&out), 3, "{args:?}");
            let want = expect(format, *opts, &entries());
            assert_eq!(out.text(), want, "{args:?}");
        }
    }
}

#[test]
fn output_is_deterministic_and_independent_of_cwd() {
    let t = Tree::new("determinism");
    t.write("dir/house.jpg", &house());
    t.write("dir/sub/pic.png", &png());
    t.write("dir/.hidden.tif", &low_only());
    for flag in ["--format=text", "--json", "--csv"] {
        let first = Cmd::new().args([flag, "dir"]).cwd(&t.root).run();
        let second = Cmd::new().args([flag, "dir"]).cwd(&t.root).run();
        assert_eq!(first.stdout, second.stdout, "{flag}");
        assert_eq!(first.code, second.code);
    }
    // From the parent with `dir`; from inside with `.`: labels are as written.
    let from_parent = Cmd::new().args(["dir"]).cwd(&t.root).run();
    let from_inside = Cmd::new().args(["."]).cwd(&t.path("dir")).run();
    let entries = |prefix: &str| {
        [
            (format!("{prefix}/.hidden.tif"), Exp::Bytes(low_only())),
            (format!("{prefix}/house.jpg"), Exp::Bytes(house())),
            (format!("{prefix}/sub/pic.png"), Exp::Bytes(png())),
        ]
    };
    let render_with = |prefix: &str| {
        let owned = entries(prefix);
        let borrowed: Vec<(&str, Exp)> = owned
            .iter()
            .map(|(l, e)| {
                (
                    l.as_str(),
                    match e {
                        Exp::Bytes(b) => Exp::Bytes(b.clone()),
                        Exp::Fail(r, d) => Exp::Fail(*r, d),
                    },
                )
            })
            .collect();
        expect(Format::Text, default_opts(), &borrowed)
    };
    assert_eq!(from_parent.text(), render_with("dir"));
    assert_eq!(from_inside.text(), render_with("."));
    // Not canonicalised: a path with `..` keeps it.
    let dotted = Cmd::new().args(["dir/../dir/sub"]).cwd(&t.root).run();
    assert!(dotted.text().contains("file: dir/../dir/sub/pic.png"), "{}", dotted.text());
    // No absolute path, no temp directory, in any output.
    let root = t.root.display().to_string();
    for out in [&from_parent, &from_inside, &dotted] {
        assert!(!out.text().contains(&root), "an absolute path leaked");
        assert!(!out.text().contains("/private"), "an absolute path leaked");
    }
    // An absolute argument is printed as given (not resolved further).
    let abs = t.path("dir/sub");
    let out = Cmd::new().args([abs.as_os_str()]).cwd(&t.root).run();
    assert!(out.text().contains(&format!("file: {}/pic.png", abs.display())), "{}", out.text());
}

// ── exit codes, usage, help ─────────────────────────────────────────────────

#[test]
fn exit_code_for_each_row_of_the_table() {
    struct Row {
        name: &'static str,
        files: Vec<(&'static str, Vec<u8>)>,
        args: Vec<&'static str>,
        want: i32,
    }
    let row = |name, files, args, want| Row { name, files, args, want };
    let rows = vec![
        // 0: complete and under the threshold.
        row("an empty directory", vec![], vec!["dir"], 0),
        row("a low-only file under fail-on high", vec![("low.tif", low_only())], vec!["dir"], 0),
        row("a png (unsupported is not failure)", vec![("a.png", png())], vec!["dir"], 0),
        row("an issues jpeg without --strict", vec![("i.jpg", issues_jpeg())], vec!["dir"], 0),
        row("findings with --fail-on none", vec![("h.jpg", house())], vec!["--fail-on", "none", "dir"], 0),
        // 1: complete, at or above the threshold.
        row("the full-house jpeg", vec![("h.jpg", house())], vec!["dir"], 1),
        row("a high-only file", vec![("high.tif", high_only())], vec!["dir"], 1),
        row("findings next to unsupported files", vec![("h.jpg", house()), ("a.png", png())], vec!["dir"], 1),
        // 3: incomplete.
        row("an unreadable jpeg", vec![("u.jpg", unreadable_jpeg())], vec!["dir"], 3),
        row("a missing path", vec![], vec!["dir/nope"], 3),
        row("a png under --strict", vec![("a.png", png())], vec!["--strict", "dir"], 3),
        row("an issues jpeg under --strict", vec![("i.jpg", issues_jpeg())], vec!["--strict", "dir"], 3),
        row("a failure beside findings", vec![("h.jpg", house()), ("u.jpg", unreadable_jpeg())], vec!["dir"], 3),
        // 2: usage.
        row("no path", vec![], vec![], 2),
        row("an unknown option", vec![], vec!["--nope", "dir"], 2),
        row("json with csv", vec![], vec!["--json", "--csv", "dir"], 2),
        row("a bad value", vec![], vec!["--fail-on", "critical", "dir"], 2),
    ];
    for r in rows {
        let t = Tree::new("exit");
        t.mkdir("dir");
        for (name, bytes) in &r.files {
            t.write(&format!("dir/{name}"), bytes);
        }
        let out = Cmd::new().args(&r.args).cwd(&t.root).run();
        assert_eq!(exit_of(&out), r.want, "{}: stdout {} stderr {}", r.name, out.text(), out.err_text());
    }
    // 3 again, from a real permission error (not when root can read anything).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let t = Tree::new("exit-perm");
        let p = t.write("dir/locked.jpg", &house());
        fs::set_permissions(&p, fs::Permissions::from_mode(0o000)).expect("chmod");
        if fs::File::open(&p).is_ok() {
            skip("this user can read a mode 000 file (root?): the permission-denied row is not run");
        } else {
            let (out, files) = run_json(&t, &["dir"]);
            assert_eq!(exit_of(&out), 3);
            assert_eq!(
                files,
                vec![(
                    "dir/locked.jpg".into(),
                    "failed".into(),
                    Some("permission denied".into())
                )]
            );
        }
        let _ = fs::set_permissions(&p, fs::Permissions::from_mode(0o644));
    }
}

#[test]
fn precedence_incomplete_over_findings_and_write_failure_over_all() {
    let t = Tree::new("precedence");
    t.write("dir/house.jpg", &house());
    t.write("pad.png", &png());
    // Findings alone: 1. Findings plus a missing path: 3, not 1.
    assert_eq!(exit_of(&Cmd::new().args(["dir"]).cwd(&t.root).run()), 1);
    assert_eq!(exit_of(&Cmd::new().args(["dir", "missing"]).cwd(&t.root).run()), 3);
    assert_eq!(exit_of(&Cmd::new().args(["missing", "dir"]).cwd(&t.root).run()), 3);
    // Usage outranks the rest (nothing is scanned).
    let usage = Cmd::new().args(["dir", "missing", "--bogus"]).cwd(&t.root).run();
    assert_eq!(exit_of(&usage), 2);
    assert!(usage.stdout.is_empty());
    // A report that cannot be written: 4 over findings and over incomplete.
    for args in [&["dir"][..], &["dir", "missing"], &["--fail-on", "none", "dir"]] {
        let code = closed_pipe_run(&t, args);
        assert_eq!(code, Some(4), "{args:?}");
    }
}

/// Run with stdout a pipe whose read end is closed before the child writes:
/// `PADS` copies of `pad.png` (in the tree) make the report far larger than
/// any pipe buffer, so the write must fail. Returns the exit code.
fn closed_pipe_run(t: &Tree, args: &[&str]) -> Option<i32> {
    use std::process::{Command, Stdio};
    const PADS: usize = 2500;
    let mut c = Command::new(cli_tree::BIN);
    c.current_dir(&t.root).args(args);
    c.args(std::iter::repeat("pad.png").take(PADS));
    c.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    let mut child = c.spawn().expect("spawn");
    drop(child.stdout.take());
    let start = std::time::Instant::now();
    loop {
        if let Some(s) = child.try_wait().expect("wait") {
            return s.code();
        }
        if start.elapsed() > Duration::from_secs(60) {
            let _ = child.kill();
            panic!("the binary hung on a closed pipe");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn write_failure_exits_four() {
    let t = Tree::new("write-failure");
    t.write("dir/a.png", &png());
    t.write("pad.png", &png());
    // A closed pipe: the report is far larger than a pipe buffer.
    assert_eq!(closed_pipe_run(&t, &["--csv"]), Some(4));
    assert_eq!(closed_pipe_run(&t, &["--json", "--fail-on", "none"]), Some(4));
    // A full disk, where /dev/full exists.
    if Path::new("/dev/full").exists() {
        use std::process::{Command, Stdio};
        let full = fs::OpenOptions::new().write(true).open("/dev/full").expect("open /dev/full");
        let status = Command::new(cli_tree::BIN)
            .current_dir(&t.root)
            .arg("dir")
            .stdout(Stdio::from(full))
            .stderr(Stdio::null())
            .status()
            .expect("run");
        assert_eq!(status.code(), Some(4), "/dev/full");
    } else {
        println!("NOTE: no /dev/full on this host; the closed-pipe case ran");
    }
}

#[test]
fn usage_error_has_empty_stdout_and_a_message_on_stderr() {
    let t = Tree::new("usage");
    t.mkdir("dir");
    let mut cases: Vec<Vec<std::ffi::OsString>> = [
        &[][..],
        &["--bogus", "dir"],
        &["--json", "--csv", "dir"],
        &["--format", "xml", "dir"],
        &["--max-size", "0", "dir"],
        &["--max-size", "99999999999999999999", "dir"],
        &["-", "dir"],
        &["--strict", "--strict", "dir"],
        &["--fail-on"],
        &["--strict=1", "dir"],
    ]
    .iter()
    .map(|a| a.iter().map(std::ffi::OsString::from).collect())
    .collect();
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        cases.push(vec![std::ffi::OsString::from_vec(b"--bad\xFF\xFE".to_vec())]);
    }
    for args in cases {
        let out = Cmd::new().args(&args).cwd(&t.root).run();
        assert_eq!(exit_of(&out), 2, "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}: stdout {}", out.text());
        let err = out.err_text();
        assert!(err.starts_with("exif-audit: "), "{err}");
        assert!(err.is_ascii() && !err.contains("panicked"), "{err}");
        assert!(err.contains("--help"), "{err}");
    }
}

#[test]
fn help_and_version_exit_zero() {
    let t = Tree::new("help");
    for args in [&["--help"][..], &["-h"], &["--bogus", "--help"], &["--format", "xml", "-h", "p"]] {
        let out = Cmd::new().args(args).cwd(&t.root).run();
        assert_eq!(exit_of(&out), 0, "{args:?}");
        assert!(out.stderr.is_empty(), "{args:?}: {}", out.err_text());
        assert!(out.text().starts_with("Usage: exif-audit"), "{args:?}");
    }
    let version = format!("exif-audit {}\n", env!("CARGO_PKG_VERSION"));
    for args in [&["--version"][..], &["-V"], &["--bogus", "-V"]] {
        let out = Cmd::new().args(args).cwd(&t.root).run();
        assert_eq!(exit_of(&out), 0, "{args:?}");
        assert!(out.stderr.is_empty());
        assert_eq!(out.text(), version, "{args:?}");
    }
}

#[test]
fn usage_text_matches_the_golden_file() {
    let t = Tree::new("golden");
    let golden =
        fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/usage.txt")).expect("golden");
    let out = Cmd::new().args(["--help"]).cwd(&t.root).run();
    assert_eq!(out.stdout, golden, "tests/golden/usage.txt is the --help output byte for byte");
    // Every option's default is in the text, quoted from DEC-011.
    let text = out.text();
    for default in ["[default: text]", "[default: all-pii]", "[default: high]", "[default: 256M]"] {
        assert!(text.contains(default), "{default}");
    }
    assert_eq!(text.matches("[default: off]").count(), 3);
}

#[test]
fn strict_turns_unsupported_and_issues_into_exit_three() {
    for (name, bytes) in [("a.png", png()), ("i.jpg", issues_jpeg()), ("n.txt", b"text".to_vec())] {
        let t = Tree::new("strict");
        t.write(&format!("dir/{name}"), &bytes);
        let plain = Cmd::new().args(["dir"]).cwd(&t.root).run();
        let strict = Cmd::new().args(["--strict", "dir"]).cwd(&t.root).run();
        assert_eq!(exit_of(&plain), 0, "{name} without --strict");
        assert_eq!(exit_of(&strict), 3, "{name} with --strict");
        assert_eq!(plain.stdout, strict.stdout, "--strict changes the exit code, not the report");
    }
    // A clean file stays 0 under --strict.
    let t = Tree::new("strict-clean");
    t.write("dir/low.tif", &low_only());
    assert_eq!(exit_of(&Cmd::new().args(["--strict", "dir"]).cwd(&t.root).run()), 0);
}

#[test]
fn fail_on_thresholds_are_inclusive_and_none_disables() {
    let low = low_only();
    let medium = medium_only();
    let high = high_only();
    // The inputs have the severities the grid assumes.
    use exif_core::audit::Severity;
    assert_eq!(audit(&low).max_severity(), Some(Severity::Low));
    assert_eq!(audit(&medium).max_severity(), Some(Severity::Medium));
    assert_eq!(audit(&high).max_severity(), Some(Severity::High));
    // (file, [(fail-on, expected exit)])
    let grid: [(&str, &Vec<u8>, [(&str, i32); 4]); 3] = [
        ("low", &low, [("none", 0), ("low", 1), ("medium", 0), ("high", 0)]),
        ("medium", &medium, [("none", 0), ("low", 1), ("medium", 1), ("high", 0)]),
        ("high", &high, [("none", 0), ("low", 1), ("medium", 1), ("high", 1)]),
    ];
    for (name, bytes, cells) in grid {
        let t = Tree::new("failon");
        t.write(&format!("dir/{name}.tif"), bytes);
        for (level, want) in cells {
            for form in [vec!["--fail-on", level], vec![]] {
                let mut args = form.clone();
                let eq = format!("--fail-on={level}");
                if form.is_empty() {
                    args = vec![eq.as_str()];
                }
                args.push("dir");
                let out = Cmd::new().args(&args).cwd(&t.root).run();
                assert_eq!(exit_of(&out), want, "{name} file, {args:?}");
            }
        }
    }
    // The default threshold is high.
    let t = Tree::new("failon-default");
    t.write("dir/m.tif", &medium);
    assert_eq!(exit_of(&Cmd::new().args(["dir"]).cwd(&t.root).run()), 0);
    t.write("dir/h.tif", &high);
    assert_eq!(exit_of(&Cmd::new().args(["dir"]).cwd(&t.root).run()), 1);
}

// ── the walk ────────────────────────────────────────────────────────────────

#[cfg(unix)]
#[test]
fn symlink_in_a_walk_is_reported_not_followed() {
    use std::os::unix::fs::symlink;
    let t = Tree::new("symlink");
    t.write("dir/real.jpg", &house());
    t.mkdir("dir/sub");
    t.write("dir/sub/inner.png", &png());
    symlink("real.jpg", t.path("dir/link.jpg")).expect("symlink");
    symlink("sub", t.path("dir/linkdir")).expect("symlink");
    symlink("missing-target", t.path("dir/dangling")).expect("symlink");
    let (out, files) = run_json(&t, &["dir"]);
    assert_eq!(exit_of(&out), 3);
    let want = |l: &str, s: &str, d: Option<&str>| (l.to_string(), s.to_string(), d.map(str::to_string));
    assert_eq!(
        files,
        vec![
            want("dir/dangling", "failed", Some("symlink not followed")),
            want("dir/link.jpg", "failed", Some("symlink not followed")),
            want("dir/linkdir", "failed", Some("symlink not followed")),
            want("dir/real.jpg", "ok", None),
            want("dir/sub/inner.png", "unsupported", None),
        ]
    );
    // The reason class too.
    let j = json_of(&out);
    let f = &j.get("files").and_then(J::as_arr).expect("files")[1];
    let failure = f.get("failure").expect("failure");
    assert_eq!(failure.get("reason").and_then(J::as_str), Some("not_regular_file"));
    // The target was not read twice: one audited house file.
    assert_eq!(files.iter().filter(|f| f.1 != "failed" && f.0.ends_with(".jpg")).count(), 1);
}

#[cfg(unix)]
#[test]
fn named_symlink_is_followed() {
    use std::os::unix::fs::symlink;
    let t = Tree::new("named-symlink");
    t.write("real/house.jpg", &house());
    t.write("real/pic.png", &png());
    symlink("real/house.jpg", t.path("link.jpg")).expect("symlink");
    symlink("real", t.path("linkdir")).expect("symlink");
    let (out, files) = run_json(&t, &["link.jpg"]);
    assert_eq!(exit_of(&out), 1, "{}", out.text());
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].0, "link.jpg");
    assert_ne!(files[0].1, "failed");
    // A named symlink to a directory is walked; its own entries are labelled
    // under the name given.
    let (out, files) = run_json(&t, &["linkdir"]);
    assert_eq!(exit_of(&out), 1);
    let labels: Vec<&str> = files.iter().map(|f| f.0.as_str()).collect();
    assert_eq!(labels, ["linkdir/house.jpg", "linkdir/pic.png"]);
    assert!(files.iter().all(|f| f.1 != "failed"));
}

#[cfg(unix)]
#[test]
fn symlink_loop_does_not_hang() {
    use std::os::unix::fs::symlink;
    let t = Tree::new("loop");
    t.write("dir/a.png", &png());
    symlink(".", t.path("dir/self")).expect("symlink");
    symlink("..", t.path("dir/up")).expect("symlink");
    symlink("p2", t.path("dir/p1")).expect("symlink");
    symlink("p1", t.path("dir/p2")).expect("symlink");
    let out = Cmd::new()
        .args(["--json", "dir"])
        .cwd(&t.root)
        .limit(Duration::from_secs(20))
        .run();
    assert_eq!(exit_of(&out), 3);
    let files = files_of(&json_of(&out));
    for name in ["self", "up", "p1", "p2"] {
        let label = format!("dir/{name}");
        let f = files.iter().find(|f| f.0 == label).unwrap_or_else(|| panic!("{label}"));
        assert_eq!((f.1.as_str(), f.2.as_deref()), ("failed", Some("symlink not followed")), "{label}");
    }
    // A loop named on the command line cannot be followed either: a failed
    // entry with a text from the closed list, never a hang.
    let named = Cmd::new()
        .args(["--json", "dir/p1"])
        .cwd(&t.root)
        .limit(Duration::from_secs(20))
        .run();
    assert_eq!(exit_of(&named), 3);
    let files = files_of(&json_of(&named));
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].1, "failed");
    let closed = ["not found", "permission denied", "read error", "i/o error"];
    assert!(closed.contains(&files[0].2.as_deref().expect("detail")), "{files:?}");
}

#[cfg(unix)]
fn mkfifo(path: &Path) -> bool {
    std::process::Command::new("mkfifo")
        .arg(path)
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(unix)]
#[test]
fn fifo_is_reported_and_never_opened() {
    let t = Tree::new("fifo");
    t.write("dir/a.png", &png());
    if !mkfifo(&t.path("dir/pipe")) {
        skip("mkfifo is not available");
        return;
    }
    let limit = Duration::from_secs(20);
    // Opening the FIFO for reading would block forever: the limit kills it.
    let out = Cmd::new().args(["--json", "dir"]).cwd(&t.root).limit(limit).run();
    assert_eq!(exit_of(&out), 3);
    let files = files_of(&json_of(&out));
    assert_eq!(
        files,
        vec![
            ("dir/a.png".into(), "unsupported".into(), None),
            ("dir/pipe".into(), "failed".into(), Some("not a regular file".into())),
        ]
    );
    let j = json_of(&out);
    let reason = j.get("files").and_then(J::as_arr).expect("files")[1]
        .get("failure")
        .and_then(|f| f.get("reason"))
        .and_then(J::as_str);
    assert_eq!(reason, Some("not_regular_file"));
    // Named on the command line it is the same.
    let named = Cmd::new().args(["--json", "dir/pipe"]).cwd(&t.root).limit(limit).run();
    assert_eq!(exit_of(&named), 3);
    assert_eq!(
        files_of(&json_of(&named)),
        vec![("dir/pipe".into(), "failed".into(), Some("not a regular file".into()))]
    );
}

#[test]
fn depth_limit_is_reported() {
    let t = Tree::new("depth");
    // top/d/d/.../d: 65 directories below the argument; a file in the 64th.
    let mut rel = String::from("top");
    for _ in 0..64 {
        rel.push_str("/d");
    }
    t.write(&format!("{rel}/f.png"), &png());
    let deep = format!("{rel}/d");
    t.mkdir(&deep);
    let (out, files) = run_json(&t, &["top"]);
    assert_eq!(exit_of(&out), 3);
    // In name order inside the 64th directory: the 65th ("d") before "f.png".
    assert_eq!(
        files,
        vec![
            (deep.clone(), "failed".into(), Some("directory depth limit".into())),
            (format!("{rel}/f.png"), "unsupported".into(), None),
        ]
    );
    let j = json_of(&out);
    let reason = j.get("files").and_then(J::as_arr).expect("files")[0]
        .get("failure")
        .and_then(|f| f.get("reason"))
        .and_then(J::as_str);
    assert_eq!(reason, Some("other"));
    // Without the 65th directory nothing is reported.
    fs::remove_dir(t.path(&deep)).expect("rmdir");
    let (out, files) = run_json(&t, &["top"]);
    assert_eq!(exit_of(&out), 0);
    assert_eq!(files.len(), 1);
}

#[test]
fn missing_path_is_failed_and_others_still_run() {
    let t = Tree::new("missing");
    t.write("dir/a.png", &png());
    t.write("b.png", &png());
    let (out, files) = run_json(&t, &["nope", "dir", "also/nope", "b.png"]);
    assert_eq!(exit_of(&out), 3);
    assert_eq!(
        files,
        vec![
            ("nope".into(), "failed".into(), Some("not found".into())),
            ("dir/a.png".into(), "unsupported".into(), None),
            ("also/nope".into(), "failed".into(), Some("not found".into())),
            ("b.png".into(), "unsupported".into(), None),
        ]
    );
    // The empty string is a path too, and missing.
    let (out, files) = run_json(&t, &["", "b.png"]);
    assert_eq!(exit_of(&out), 3);
    assert_eq!(files[0], ("".into(), "failed".into(), Some("not found".into())));
    assert_eq!(files.len(), 2);
}

// ── size ────────────────────────────────────────────────────────────────────

#[test]
fn size_cap_is_exact() {
    let t = Tree::new("size");
    t.write("at.jpg", &jpeg_of_len(1024));
    t.write("over.jpg", &jpeg_of_len(1025));
    // Exactly the cap is audited; one byte more is `failed` / `too_large`.
    for (arg, file, audited) in [
        ("1024", "at.jpg", true),
        ("1023", "at.jpg", false),
        ("1K", "at.jpg", true),
        ("1K", "over.jpg", false),
        ("1025", "over.jpg", true),
        ("2K", "over.jpg", true),
    ] {
        let (out, files) = run_json(&t, &["--max-size", arg, file]);
        if audited {
            assert_eq!(exit_of(&out), 0, "--max-size {arg} {file}: {files:?}");
            assert_eq!(files, vec![(file.into(), "ok".into(), None)], "--max-size {arg} {file}");
        } else {
            assert_eq!(exit_of(&out), 3, "--max-size {arg} {file}");
            assert_eq!(
                files,
                vec![(file.into(), "failed".into(), Some("larger than --max-size".into()))],
                "--max-size {arg} {file}"
            );
            let reason = json_of(&out).get("files").and_then(J::as_arr).expect("files")[0]
                .get("failure")
                .and_then(|f| f.get("reason"))
                .and_then(J::as_str)
                .map(str::to_string);
            assert_eq!(reason.as_deref(), Some("too_large"));
        }
    }
    // The equals form, and the default cap is far above these files.
    let (_, files) = run_json(&t, &["--max-size=1024", "at.jpg"]);
    assert_eq!(files[0].1, "ok");
    let (_, files) = run_json(&t, &["over.jpg"]);
    assert_eq!(files[0].1, "ok");
}

/// A sparse file of `len` bytes starting with `head`, or `None` (a loud skip)
/// when the filesystem does not make it sparse.
fn sparse_file(t: &Tree, rel: &str, head: &[u8], len: u64) -> Option<std::path::PathBuf> {
    use std::io::Write;
    let p = t.path(rel);
    fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
    let mut f = fs::File::create(&p).expect("create");
    f.write_all(head).expect("head");
    if f.set_len(len).is_err() {
        skip("this filesystem cannot extend a file to 2 GiB");
        return None;
    }
    drop(f);
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let allocated = fs::metadata(&p).expect("metadata").blocks().saturating_mul(512);
        if allocated > 64 * 1024 * 1024 {
            skip("this filesystem does not keep a 2 GiB file sparse");
            let _ = fs::remove_file(&p);
            return None;
        }
    }
    Some(p)
}

#[test]
fn non_image_is_not_read_whole() {
    let t = Tree::new("sparse");
    let gib2: u64 = 2 * 1024 * 1024 * 1024;
    let Some(_) = sparse_file(&t, "dir/big.png", &png(), gib2) else { return };
    let Some(_) = sparse_file(&t, "dir/big.bin", &[0; 16], gib2) else { return };
    let limit = Duration::from_secs(60);
    // Default cap (256 MiB): a 2 GiB non-image is `unsupported`, not too large.
    let out = Cmd::new().args(["--json", "dir"]).cwd(&t.root).limit(limit).run();
    assert_eq!(exit_of(&out), 0);
    assert_eq!(
        files_of(&json_of(&out)),
        vec![
            ("dir/big.bin".into(), "unsupported".into(), None),
            ("dir/big.png".into(), "unsupported".into(), None),
        ]
    );
    // With the cap raised above the file, still not read: the child's peak
    // memory stays far below 2 GiB.
    let (out, peak) = Cmd::new()
        .args(["--json", "--max-size", "4G", "dir"])
        .cwd(&t.root)
        .limit(limit)
        .run_measured();
    assert_eq!(exit_of(&out), 0);
    match peak {
        Some(p) => {
            println!("peak resident memory over two 2 GiB non-images: {} MB", p / 1_000_000);
            assert!(p < 100_000_000, "peak {p} bytes: the file was read");
        }
        None => skip("/usr/bin/time is not available to measure the peak memory"),
    }
    // A 2 GiB TIFF header is rejected from its length, not read: `too_large`.
    let tiff_head = b"II*\x00\x08\x00\x00\x00";
    let Some(_) = sparse_file(&t, "tiff/big.tif", tiff_head, gib2) else { return };
    let out = Cmd::new().args(["--json", "tiff"]).cwd(&t.root).limit(limit).run();
    assert_eq!(exit_of(&out), 3);
    assert_eq!(
        files_of(&json_of(&out)),
        vec![("tiff/big.tif".into(), "failed".into(), Some("larger than --max-size".into()))]
    );
}

#[test]
fn peak_memory_does_not_grow_with_the_number_of_files() {
    // 200 copies of a JPEG of about 2 MB: 400 MB of input, one file at a time.
    let mut segments = vec![report_inputs::exif_app1(&report_inputs::full_house_tiff())];
    for _ in 0..31 {
        segments.push(report_inputs::seg(0xFE, &vec![b'x'; 65_000]));
    }
    let big = report_inputs::jpeg(&segments);
    assert!(big.len() > 2_000_000 && big.len() < 2_100_000, "{}", big.len());
    let t = Tree::new("memory");
    t.write("dir/f000.jpg", &big);
    for i in 1..200 {
        fs::copy(t.path("dir/f000.jpg"), t.path(&format!("dir/f{i:03}.jpg"))).expect("copy");
    }
    let (out, peak) = Cmd::new().args(["--json", "dir"]).cwd(&t.root).run_measured();
    assert_eq!(exit_of(&out), 1, "stderr: {}", out.err_text());
    assert_eq!(files_of(&json_of(&out)).len(), 200);
    match peak {
        Some(p) => {
            println!("peak resident memory over 200 x {} bytes: {} MB", big.len(), p / 1_000_000);
            assert!(p < 100_000_000, "peak resident memory {p} bytes is not under 100 MB");
        }
        None => skip("/usr/bin/time is not available to measure the peak memory"),
    }
}

// ── hostile names ───────────────────────────────────────────────────────────

const HOSTILE_NAMES: [&str; 11] = [
    "new\nline.png",
    "dq\".png",
    "co,mma.png",
    "semi;colon.png",
    "=eq.png",
    "@at.png",
    "+plus.png",
    "-minus.png",
    "rlo\u{202E}gnp.png",
    "invisible\u{200B}\u{FEFF}\u{AD}.png",
    "tab\there.png",
];

/// What the CSV carries for a label (DEC-010 part 3): `safe_text`, and a
/// leading apostrophe when the text would start a spreadsheet formula.
fn csv_label(label: &[u8]) -> String {
    let text = naive_safe_text(label);
    if text.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        format!("'{text}")
    } else {
        text
    }
}

fn assert_csv_is_well_formed(out: &Out, labels: &[Vec<u8>]) {
    let rows = parse_csv(&out.stdout).unwrap_or_else(|e| panic!("not strict CSV: {e}"));
    assert_eq!(rows.len(), labels.len() + 1, "header plus one row per file");
    assert!(rows.iter().all(|r| r.len() == 19));
    let mut got: Vec<String> = rows.iter().skip(1).map(|r| r[0].clone()).collect();
    let mut want: Vec<String> = labels.iter().map(|l| csv_label(l)).collect();
    got.sort();
    want.sort();
    assert_eq!(got, want);
    // No label cell starts a formula.
    for label in &got {
        assert!(!label.starts_with(['=', '+', '-', '@', '\t', '\r']), "{label:?}");
    }
}

#[test]
fn hostile_file_names_give_well_formed_json_and_csv() {
    let t = Tree::new("hostile");
    for name in HOSTILE_NAMES {
        t.write(&format!("dir/{name}"), &png());
    }
    // In a walk: labels are `dir/` plus the name.
    let labels: Vec<Vec<u8>> = HOSTILE_NAMES.iter().map(|n| format!("dir/{n}").into_bytes()).collect();
    let json = Cmd::new().args(["--json", "dir"]).cwd(&t.root).run();
    assert_eq!(exit_of(&json), 0, "{}", json.err_text());
    let mut got: Vec<String> = files_of(&json_of(&json)).into_iter().map(|f| f.0).collect();
    let mut want: Vec<String> = labels.iter().map(|l| naive_safe_text(l)).collect();
    got.sort();
    want.sort();
    assert_eq!(got, want, "the JSON label is safe_text of the name");
    let csv = Cmd::new().args(["--csv", "dir"]).cwd(&t.root).run();
    assert_eq!(exit_of(&csv), 0);
    assert_csv_is_well_formed(&csv, &labels);
    // And the text report has no raw control or bidi characters.
    let text = Cmd::new().args(["dir"]).cwd(&t.root).run().text();
    assert!(text.chars().all(|c| !c.is_control() || c == '\n'), "a control character leaked");
    assert!(!text.contains('\u{202E}') && !text.contains('\u{200B}') && !text.contains('\u{FEFF}'));
    // Named directly from inside `dir`, the label starts with the hostile
    // character (a formula start): the CSV quotes it. `--` lets a path start
    // with `-`.
    let inside: Vec<&str> = HOSTILE_NAMES.iter().copied().filter(|n| !n.contains('\n')).collect();
    let mut a2 = vec!["--csv", "--"];
    a2.extend(inside.iter().copied());
    let out = Cmd::new().args(&a2).cwd(&t.path("dir")).run();
    assert_eq!(exit_of(&out), 0, "{}", out.err_text());
    let labels: Vec<Vec<u8>> = inside.iter().map(|n| n.as_bytes().to_vec()).collect();
    assert_csv_is_well_formed(&out, &labels);
    let formula_starts = inside.iter().filter(|n| n.starts_with(['=', '@', '+', '-'])).count();
    assert_eq!(formula_starts, 4);
    let json = Cmd::new().args(["--json", "--"]).args(&inside).cwd(&t.path("dir")).run();
    let mut got: Vec<String> = files_of(&json_of(&json)).into_iter().map(|f| f.0).collect();
    let mut want: Vec<String> = labels.iter().map(|l| naive_safe_text(l)).collect();
    got.sort();
    want.sort();
    assert_eq!(got, want);
}

#[cfg(unix)]
#[test]
fn invalid_utf8_file_names_are_escaped() {
    use std::os::unix::ffi::OsStrExt;
    let t = Tree::new("badutf8");
    let name: &[u8] = b"bad\xFF\xFE.png";
    let path = t.path("dir");
    fs::create_dir_all(&path).expect("mkdir");
    let file = path.join(std::ffi::OsStr::from_bytes(name));
    if fs::write(&file, png()).is_err() {
        // macOS filesystems refuse non-UTF-8 names; Linux must accept them.
        println!("SKIP: this filesystem refuses an invalid UTF-8 file name");
        assert!(
            cfg!(target_os = "macos") || std::env::var("DIAMOND_EXIF_REQUIRE_CLI_FS").as_deref() != Ok("1"),
            "DIAMOND_EXIF_REQUIRE_CLI_FS=1 and an invalid UTF-8 name is refused"
        );
        return;
    }
    let out = Cmd::new().args(["--json", "dir"]).cwd(&t.root).run();
    assert_eq!(exit_of(&out), 0);
    let files = files_of(&json_of(&out));
    let mut label = b"dir/".to_vec();
    label.extend_from_slice(name);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].0, naive_safe_text(&label));
    assert!(files[0].0.contains("\\xff\\xfe"), "{}", files[0].0);
    let csv = Cmd::new().args(["--csv", "dir"]).cwd(&t.root).run();
    assert_csv_is_well_formed(&csv, &[label]);
}

// ── no panic ────────────────────────────────────────────────────────────────

#[test]
fn no_seed_makes_the_binary_panic() {
    // Every committed seed of every fuzz target, written as a file.
    let seeds = Path::new(env!("CARGO_MANIFEST_DIR")).join("fuzz/seeds");
    let t = Tree::new("seeds");
    let mut n = 0usize;
    for target in fs::read_dir(&seeds).expect("fuzz/seeds") {
        let target = target.expect("entry");
        for seed in fs::read_dir(target.path()).expect("target dir") {
            let seed = seed.expect("entry");
            let name = format!(
                "{}-{}",
                target.file_name().to_string_lossy(),
                seed.file_name().to_string_lossy()
            );
            fs::copy(seed.path(), t.path(&format!("seeds/{name}"))).or_else(|_| {
                fs::create_dir_all(t.path("seeds")).and_then(|()| fs::copy(seed.path(), t.path(&format!("seeds/{name}"))))
            }).expect("copy a seed");
            n += 1;
        }
    }
    assert!(n >= 150, "only {n} seeds found");
    for format in ["text", "json", "csv"] {
        for extra in [&[][..], &["--strict"], &["--policy", "none"]] {
            let mut args = vec!["--format", format, "--inventory", "--values", "--policy", "everything"];
            if !extra.is_empty() {
                args.retain(|a| *a != "everything" && *a != "--policy");
                args.extend_from_slice(extra);
            }
            args.push("seeds");
            let out = Cmd::new().args(&args).cwd(&t.root).run();
            let code = out.code.unwrap_or_else(|| panic!("killed by a signal: {args:?}"));
            assert!([0, 1, 3].contains(&code), "{args:?} exited {code}; stderr: {}", out.err_text());
            assert_ne!(code, 101);
            assert!(out.stderr.is_empty(), "{args:?}: {}", out.err_text());
            assert!(!out.stdout.is_empty());
        }
    }
    println!("{n} seeds over 9 invocations");
}
