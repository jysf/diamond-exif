//! Static checks on the first binary release (SPEC-024, DEC-014). std only: the
//! workflow YAML is read as text, like `tests/fuzz_layout.rs` reads repo files.
//! The checks are functions over the file text so the red-proofs at the bottom can
//! mutate a copy and require the matching check to fail (constraint
//! `oracle-must-be-shown-red`, applied to the release gate).

use std::fs;
use std::path::PathBuf;

const VERSION: &str = "0.1.0-alpha.1";
const TARGETS: [&str; 3] = [
    "x86_64-unknown-linux-musl",
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
];
/// The only actions the release workflow may use (DEC-014 decision 3).
const ALLOWED_ACTIONS: [&str; 4] = [
    "actions/checkout@v4",
    "dtolnay/rust-toolchain@stable",
    "actions/upload-artifact@v4",
    "actions/download-artifact@v4",
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(rel: &str) -> String {
    fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("cannot read {rel}: {e}"))
}

/// The text with whole-line YAML comments removed, so prose cannot trip a check.
fn without_comments(text: &str) -> String {
    text.lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The lines indented deeper than `key` under the top-level `key:` line.
fn top_level_block(text: &str, key: &str) -> Vec<String> {
    let head = format!("{key}:");
    let mut out = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        if inside {
            if line.is_empty() || line.starts_with(' ') {
                out.push(line.to_string());
                continue;
            }
            break;
        }
        if line.trim_end() == head {
            inside = true;
        }
    }
    out
}

/// `(job id, block text)` for every job under `jobs:` (ids sit at two spaces).
fn jobs(text: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for line in top_level_block(text, "jobs") {
        let is_id = line.starts_with("  ")
            && !line.starts_with("   ")
            && line.trim_end().ends_with(':')
            && !line.trim().is_empty();
        if is_id {
            out.push((line.trim().trim_end_matches(':').to_string(), String::new()));
        } else if let Some(last) = out.last_mut() {
            last.1.push_str(&line);
            last.1.push('\n');
        }
    }
    out
}

/// Every problem found in a release workflow; empty means as designed.
fn workflow_problems(raw: &str) -> Vec<String> {
    let text = without_comments(raw);
    let mut p: Vec<String> = Vec::new();

    // Triggers: tag pushes and a manual dispatch only; never a pull request.
    if text.contains("pull_request") {
        p.push("a pull_request trigger (or mention) is present".into());
    }
    let on = top_level_block(&text, "on");
    let on_keys: Vec<String> = on
        .iter()
        .filter(|l| l.starts_with("  ") && !l.starts_with("   ") && !l.trim().is_empty())
        .map(|l| l.trim().to_string())
        .collect();
    if on_keys != ["push:", "workflow_dispatch:"] {
        p.push(format!(
            "`on:` keys are {on_keys:?}, want push and workflow_dispatch only"
        ));
    }
    let on_text = on.join("\n");
    if !on_text.contains("tags: ['v*']") {
        p.push("push does not trigger on tags: ['v*']".into());
    }
    if !on_text.contains("dry_run:") || !on_text.contains("default: true") {
        p.push("workflow_dispatch needs a dry_run input defaulting to true".into());
    }

    // Permissions: read-only at the top, write on the publish job alone.
    let perms: Vec<String> = top_level_block(&text, "permissions")
        .iter()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    if perms != ["contents: read"] {
        p.push(format!(
            "workflow-level permissions are {perms:?}, want contents: read only"
        ));
    }
    let all_jobs = jobs(&text);
    for (id, block) in &all_jobs {
        let writes = block.contains("contents: write");
        if writes && id != "publish" {
            p.push(format!(
                "job `{id}` has contents: write; only `publish` may"
            ));
        }
        if !writes && id == "publish" {
            p.push("job `publish` lacks contents: write".into());
        }
    }
    if text.matches("contents: write").count() != 1 {
        p.push("`contents: write` must appear exactly once (the publish job)".into());
    }
    let ids: Vec<&str> = all_jobs.iter().map(|(i, _)| i.as_str()).collect();
    if ids != ["check", "build", "publish"] {
        p.push(format!("jobs are {ids:?}, want check, build, publish"));
    }

    // Actions: exactly the allowed set.
    for line in text.lines() {
        if let Some(rest) = line.trim().trim_start_matches("- ").strip_prefix("uses:") {
            let used = rest.trim();
            if !ALLOWED_ACTIONS.contains(&used) {
                p.push(format!("action `{used}` is not in the allowed set"));
            }
        }
    }

    // Build: locked, release, one target per matrix entry, the designed three.
    for line in text.lines().filter(|l| l.contains("cargo build")) {
        for flag in ["--release", "--locked", "--bin exif-audit", "--target"] {
            if !line.contains(flag) {
                p.push(format!("cargo build lacks {flag}: {}", line.trim()));
            }
        }
    }
    if !text.contains("cargo build") {
        p.push("no cargo build step".into());
    }
    let mut targets: Vec<String> = all_jobs
        .iter()
        .filter(|(id, _)| id == "build")
        .flat_map(|(_, b)| b.lines().map(str::to_string).collect::<Vec<_>>())
        .filter_map(|l| {
            l.trim()
                .trim_start_matches("- ")
                .strip_prefix("target:")
                .map(|t| t.trim().to_string())
        })
        .collect();
    targets.sort();
    let mut want: Vec<String> = TARGETS.iter().map(|t| t.to_string()).collect();
    want.sort();
    if targets != want {
        p.push(format!("build targets are {targets:?}, want {want:?}"));
    }
    if !text.contains("EXIF_AUDIT_BUILD_REF") {
        p.push("the build does not set EXIF_AUDIT_BUILD_REF".into());
    }
    for name in ["LICENSE-MIT", "LICENSE-APACHE", "README.md"] {
        if !text.contains(name) {
            p.push(format!("archive does not include {name}"));
        }
    }

    // Tag gate: a tag must equal the Cargo version, before any build; a dispatch
    // with dry_run false must be on a tag.
    match all_jobs.iter().find(|(id, _)| id == "check") {
        Some((_, c)) => {
            for needle in [
                "Cargo.toml",
                "GITHUB_REF_NAME",
                "refs/tags/",
                "dry_run",
                "exit 1",
            ] {
                if !c.contains(needle) {
                    p.push(format!("`check` job lacks `{needle}`"));
                }
            }
        }
        None => p.push("no `check` job".into()),
    }
    if let Some((_, b)) = all_jobs.iter().find(|(id, _)| id == "build") {
        if !b.contains("needs: check") {
            p.push("`build` does not need `check`".into());
        }
    }

    // Publish: checksums, then a pre-release created only for a tag push.
    match all_jobs.iter().find(|(id, _)| id == "publish") {
        Some((_, b)) => {
            for needle in [
                "needs: [check, build]",
                "SHA256SUMS",
                "--prerelease",
                "--verify-tag",
                "--notes-file",
            ] {
                if !b.contains(needle) {
                    p.push(format!("`publish` job lacks `{needle}`"));
                }
            }
            let guard = b.find("if: github.event_name == 'push'");
            let create = b.find("gh release create");
            match (guard, create) {
                (Some(g), Some(c)) if g < c => {}
                _ => p.push(
                    "`gh release create` is not behind `if: github.event_name == 'push'`".into(),
                ),
            }
            if b.matches("gh release create").count() != 1 {
                p.push("`gh release create` must appear exactly once".into());
            }
        }
        None => p.push("no `publish` job".into()),
    }
    p
}

#[test]
fn cargo_version_is_the_alpha_and_publish_is_false() {
    let cargo = read("Cargo.toml");
    assert!(
        cargo.contains(&format!("version = \"{VERSION}\"")),
        "Cargo.toml version is not {VERSION}"
    );
    assert!(
        cargo.lines().any(|l| l.trim() == "publish = false"),
        "Cargo.toml must say publish = false"
    );
    let lock = read("Cargo.lock");
    let pkg = lock
        .split("[[package]]")
        .find(|s| s.contains("name = \"exif-core\""));
    let pkg = pkg.unwrap_or_else(|| panic!("exif-core is not in Cargo.lock"));
    assert!(
        pkg.contains(&format!("version = \"{VERSION}\"")),
        "Cargo.lock disagrees with Cargo.toml"
    );
}

#[test]
fn release_workflow_triggers_permissions_and_actions_are_as_designed() {
    let problems = workflow_problems(&read(".github/workflows/release.yml"));
    assert!(
        problems.is_empty(),
        "release.yml problems:\n{}",
        problems.join("\n")
    );
}

#[test]
fn the_three_release_targets_are_the_designed_three() {
    let wf = read(".github/workflows/release.yml");
    for t in TARGETS {
        assert!(wf.contains(t), "release.yml does not build {t}");
    }
    assert!(!wf.contains("windows"), "Windows is out of scope (DEC-014)");
    assert!(
        !wf.contains("aarch64-unknown-linux"),
        "Linux arm64 is out of scope (DEC-014)"
    );
}

#[test]
fn changelog_and_readme_carry_the_release() {
    let log = read("CHANGELOG.md");
    assert!(
        log.contains(&format!("## [{VERSION}] - ")),
        "CHANGELOG.md has no [{VERSION}] heading"
    );
    let unreleased: Vec<&str> = log
        .lines()
        .skip_while(|l| !l.starts_with("## [Unreleased]"))
        .skip(1)
        .take_while(|l| !l.starts_with("## "))
        .filter(|l| !l.trim().is_empty())
        .collect();
    assert!(
        unreleased.is_empty(),
        "[Unreleased] must be empty after the cut: {unreleased:?}"
    );
    let section: Vec<&str> = log
        .lines()
        .skip_while(|l| !l.starts_with(&format!("## [{VERSION}]")))
        .skip(1)
        .take_while(|l| !l.starts_with("## "))
        .filter(|l| !l.trim().is_empty())
        .collect();
    assert!(
        !section.is_empty(),
        "the [{VERSION}] section is empty (the release notes come from it)"
    );

    let readme = read("README.md");
    assert!(
        readme.contains("## Install"),
        "README.md has no Install section"
    );
    for needle in [
        "shasum -a 256 -c SHA256SUMS",
        "xattr -d com.apple.quarantine",
        "gh release download",
        "never writes",
        "not published",
    ] {
        assert!(readme.contains(needle), "README.md lacks `{needle}`");
    }
}

// ---- Red-proofs: each mutation of the real workflow must fail its check. ----

fn mutated(from: &str, to: &str) -> Vec<String> {
    let wf = read(".github/workflows/release.yml");
    assert!(
        wf.contains(from),
        "red-proof subject `{from}` is not in release.yml"
    );
    workflow_problems(&wf.replacen(from, to, 1))
}

#[test]
fn red_proof_pull_request_trigger_is_rejected() {
    let p = mutated("workflow_dispatch:", "pull_request:\n  workflow_dispatch:");
    assert!(p.iter().any(|m| m.contains("pull_request")), "{p:?}");
}

#[test]
fn red_proof_workflow_level_write_is_rejected() {
    let p = mutated(
        "permissions:\n  contents: read",
        "permissions:\n  contents: write",
    );
    assert!(
        p.iter().any(|m| m.contains("workflow-level permissions")),
        "{p:?}"
    );
}

#[test]
fn red_proof_disallowed_action_is_rejected() {
    let p = mutated("actions/checkout@v4", "softprops/action-gh-release@v2");
    assert!(
        p.iter().any(|m| m.contains("not in the allowed set")),
        "{p:?}"
    );
}

#[test]
fn red_proof_dropping_locked_is_rejected() {
    let p = mutated("--locked", "");
    assert!(p.iter().any(|m| m.contains("--locked")), "{p:?}");
}

#[test]
fn red_proof_dropping_prerelease_is_rejected() {
    let p = mutated("--prerelease", "");
    assert!(p.iter().any(|m| m.contains("--prerelease")), "{p:?}");
}

#[test]
fn red_proof_unguarded_release_is_rejected() {
    let p = mutated("if: github.event_name == 'push'", "if: always()");
    assert!(p.iter().any(|m| m.contains("not behind")), "{p:?}");
}

#[test]
fn red_proof_wrong_target_is_rejected() {
    let p = mutated(
        "target: aarch64-apple-darwin",
        "target: aarch64-unknown-linux-gnu",
    );
    assert!(p.iter().any(|m| m.contains("build targets")), "{p:?}");
}

#[test]
fn red_proof_cargo_version_change_is_rejected() {
    // The Cargo check reads the real file; prove its predicate on a mutated string.
    let cargo = read("Cargo.toml").replacen(
        &format!("version = \"{VERSION}\""),
        "version = \"0.1.0\"",
        1,
    );
    assert!(!cargo.contains(&format!("version = \"{VERSION}\"")));
}
