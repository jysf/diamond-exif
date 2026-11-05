//! Crate-policy tests (SPEC-001). std only: they read files relative to the
//! manifest dir, so they check the scaffold's invariants without parsing TOML.

use std::fs;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(rel: &str) -> String {
    fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("cannot read {rel}: {e}"))
}

/// Drops `//` line comments (including `//!` and `///`) so commented-out
/// attributes cannot satisfy a check. Good enough for attribute lines; it does
/// not understand `//` inside string literals, which lib.rs does not contain.
fn strip_line_comments(src: &str) -> String {
    src.lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Splits on anything that cannot be part of a `path::ident` token, so
/// `clippy::panic` never matches `clippy::panic_in_result_fn`.
fn tokens(src: &str) -> Vec<String> {
    src.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == ':'))
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .collect()
}

const FIVE_LINTS: [&str; 5] = [
    "clippy::unwrap_used",
    "clippy::expect_used",
    "clippy::indexing_slicing",
    "clippy::panic",
    "clippy::arithmetic_side_effects",
];

#[test]
fn lib_forbids_unsafe_and_denies_the_five_lints() {
    let lib = strip_line_comments(&read("src/lib.rs"));
    assert!(
        lib.contains("#![forbid(unsafe_code)]"),
        "missing forbid(unsafe_code)"
    );
    let deny_at = lib.find("#![deny(").expect("missing #![deny( block");
    let rest = &lib[deny_at..];
    let block = &rest[..rest.find(")]").expect("unterminated #![deny( block")];
    let deny_tokens = tokens(block);
    for lint in FIVE_LINTS {
        assert!(
            deny_tokens.iter().any(|t| t == lint),
            "deny block lacks {lint}"
        );
    }
    // Nothing may weaken the five lints: any allow/warn/expect attribute that
    // names one of them (inner or outer) fails the policy.
    for (i, _) in lib
        .match_indices("allow(")
        .chain(lib.match_indices("warn("))
        .chain(lib.match_indices("expect("))
    {
        let tail = &lib[i..];
        let attr = &tail[..tail.find(')').unwrap_or(tail.len())];
        let attr_tokens = tokens(attr);
        for lint in FIVE_LINTS {
            assert!(
                !attr_tokens.iter().any(|t| t == lint),
                "{lint} must not be allowed/warned/expected: {attr})"
            );
        }
    }
}

#[test]
fn manifest_is_dual_licensed_with_no_dependencies() {
    let manifest = read("Cargo.toml");
    assert!(
        manifest.contains("license = \"MIT OR Apache-2.0\""),
        "license must be MIT OR Apache-2.0"
    );
    // Any table whose header starts with `[dependencies` (including
    // `[dependencies.foo]`) is the runtime dependency table and must be empty.
    // `[dev-dependencies]`/`[build-dependencies]` are intentionally not covered.
    let mut in_deps = false;
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            assert!(
                !line.starts_with("[dependencies."),
                "dependency sub-table forbidden (DEC-002): {line}"
            );
            in_deps = line.starts_with("[dependencies");
            continue;
        }
        if in_deps && !line.is_empty() && !line.starts_with('#') {
            panic!("[dependencies] must be empty (DEC-002), found: {line}");
        }
    }
}

#[test]
fn both_license_files_exist_and_root_license_is_gone() {
    assert!(root().join("LICENSE-MIT").is_file(), "LICENSE-MIT missing");
    assert!(
        root().join("LICENSE-APACHE").is_file(),
        "LICENSE-APACHE missing"
    );
    assert!(
        !root().join("LICENSE").exists(),
        "root LICENSE must be gone"
    );
}

#[test]
fn ledger_states_exiftool_is_run_never_read() {
    let ledger = read("docs/provenance-ledger.md");
    let rule = ledger
        .lines()
        .find(|l| l.contains("ExifTool") && l.contains("RUN") && l.contains("READ"));
    assert!(
        rule.is_some(),
        "ledger must state ExifTool is RUN, never READ"
    );
}
