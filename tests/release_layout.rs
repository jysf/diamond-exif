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

/// The exact `if:` of the step that creates the release: a tag push only. Compared
/// whole-line, so widening it (`|| always()`) is caught.
const RELEASE_GUARD: &str =
    "if: github.event_name == 'push' && startsWith(github.ref, 'refs/tags/v')";
const PUSH_GUARD: &str = "if: github.event_name == 'push'";

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

/// The steps of a job block: each starts at a line indented exactly six spaces
/// with `- `.
fn steps(block: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in block.lines() {
        if line.starts_with("      - ") {
            out.push(String::new());
        }
        if let Some(last) = out.last_mut() {
            last.push_str(line);
            last.push('\n');
        }
    }
    out
}

/// Every problem found in `Cargo.toml` and `Cargo.lock` for the release.
fn cargo_problems(toml: &str, lock: &str) -> Vec<String> {
    let mut p = Vec::new();
    let want = format!("version = \"{VERSION}\"");
    if !toml.contains(&want) {
        p.push(format!("Cargo.toml version is not {VERSION}"));
    }
    if !toml.lines().any(|l| l.trim() == "publish = false") {
        p.push("Cargo.toml must say publish = false".into());
    }
    match lock
        .split("[[package]]")
        .find(|s| s.contains("name = \"exif-core\""))
    {
        Some(pkg) if pkg.contains(&want) => {}
        Some(_) => p.push("Cargo.lock disagrees with Cargo.toml".into()),
        None => p.push("exif-core is not in Cargo.lock".into()),
    }
    p
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
    let push_children: Vec<String> = on
        .iter()
        .skip_while(|l| l.trim_end() != "  push:")
        .skip(1)
        .take_while(|l| l.starts_with("    ") || l.trim().is_empty())
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.trim().to_string())
        .collect();
    if push_children != ["tags: ['v*']"] {
        p.push(format!(
            "`push:` children are {push_children:?}, want only tags: ['v*']"
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
    // No write grant anywhere but `contents: write` in publish; no per-job
    // permissions block at all outside publish (the read-only default stands).
    for (id, block) in &all_jobs {
        let grants: Vec<String> = block
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| l.ends_with(": write") || l.contains("write-all"))
            .collect();
        if id == "publish" {
            if grants != ["contents: write"] {
                p.push(format!(
                    "`publish` write grants are {grants:?}, want only contents: write"
                ));
            }
        } else {
            if !grants.is_empty() {
                p.push(format!("job `{id}` has write grants {grants:?}"));
            }
            if block
                .lines()
                .any(|l| l.trim_start().starts_with("permissions:"))
            {
                p.push(format!("job `{id}` sets its own permissions"));
            }
        }
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

    if let Some((_, c)) = all_jobs.iter().find(|(id, _)| id == "check") {
        if !c.contains("if [ \"${GITHUB_REF_NAME}\" != \"v${version}\" ]; then") {
            p.push("`check` lacks the exact tag-equals-version comparison".into());
        }
        for bypass in ["if false", "|| true", "|| :", "set +e"] {
            if c.contains(bypass) {
                p.push(format!("`check` contains the bypass `{bypass}`"));
            }
        }
        for needle in [
            "fetch-depth: 0",
            "git cat-file -t \"refs/tags/${GITHUB_REF_NAME}\"",
            "is not annotated",
            "git merge-base --is-ancestor \"${GITHUB_SHA}\" origin/main",
            "not an ancestor of origin/main",
        ] {
            if !c.contains(needle) {
                p.push(format!("`check` lacks `{needle}`"));
            }
        }
        let ok = steps(c).iter().any(|st| {
            st.contains("merge-base --is-ancestor")
                && st.contains("cat-file -t")
                && st
                    .lines()
                    .map(str::trim)
                    .filter(|l| l.starts_with("if:"))
                    .collect::<Vec<_>>()
                    == [PUSH_GUARD]
        });
        if !ok {
            p.push(
                "the annotated-and-on-main step must be guarded by exactly the push `if:`".into(),
            );
        }
    }
    // No checkout may keep the token in .git/config.
    for (id, block) in &all_jobs {
        for st in steps(block)
            .iter()
            .filter(|st| st.contains("uses: actions/checkout@v4"))
        {
            if !st.contains("persist-credentials: false") {
                p.push(format!(
                    "a checkout in job `{id}` lacks persist-credentials: false"
                ));
            }
        }
    }
    // Intel smoke: assert the architecture; no Rosetta is fatal on a tag push only.
    if let Some((_, b)) = all_jobs.iter().find(|(id, _)| id == "build") {
        let err = b.find("::error::Rosetta");
        let warn = b.find("::warning::Rosetta");
        let gate = b.find("[ \"${GITHUB_EVENT_NAME}\" = \"push\" ]");
        match (gate, err, warn) {
            (Some(g), Some(e), Some(w)) if g < e && e < w => {}
            _ => p.push(
                "Intel smoke: missing Rosetta must fail a tag push and only warn on a dry run"
                    .into(),
            ),
        }
        if !b.contains("grep -q 'x86_64'") {
            p.push("Intel smoke does not assert that `file` reports x86_64".into());
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
            let create_steps: Vec<String> = steps(b)
                .into_iter()
                .filter(|st| st.contains("gh release create"))
                .collect();
            match create_steps.as_slice() {
                [st] => {
                    let ifs: Vec<&str> = st
                        .lines()
                        .map(str::trim)
                        .filter(|l| l.starts_with("if:"))
                        .collect();
                    if ifs != [RELEASE_GUARD] {
                        p.push(format!("`gh release create` step `if:` is {ifs:?}, want exactly {RELEASE_GUARD}"));
                    }
                }
                _ => p.push("`gh release create` must be in exactly one step".into()),
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
    let p = cargo_problems(&read("Cargo.toml"), &read("Cargo.lock"));
    assert!(p.is_empty(), "Cargo problems:\n{}", p.join("\n"));
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
    let p = mutated(RELEASE_GUARD, "if: always()");
    assert!(
        p.iter().any(|m| m.contains("`gh release create` step")),
        "{p:?}"
    );
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
    let toml = read("Cargo.toml");
    let lock = read("Cargo.lock");
    assert!(cargo_problems(&toml, &lock).is_empty());
    let bad_toml = toml.replacen(
        &format!("version = \"{VERSION}\""),
        "version = \"0.1.0\"",
        1,
    );
    assert!(cargo_problems(&bad_toml, &lock)
        .iter()
        .any(|m| m.contains("Cargo.toml version")));
    let bad_lock = lock.replacen(
        &format!("version = \"{VERSION}\""),
        "version = \"0.0.0\"",
        1,
    );
    assert!(cargo_problems(&toml, &bad_lock)
        .iter()
        .any(|m| m.contains("Cargo.lock")));
    let no_publish = toml.replacen("publish = false", "", 1);
    assert!(cargo_problems(&no_publish, &lock)
        .iter()
        .any(|m| m.contains("publish")));
}

#[test]
fn red_proof_widened_release_guard_is_rejected() {
    let widened = format!("{RELEASE_GUARD} || always()");
    let p = mutated(RELEASE_GUARD, &widened);
    assert!(
        p.iter().any(|m| m.contains("`gh release create` step")),
        "{p:?}"
    );
}

#[test]
fn red_proof_write_all_on_build_is_rejected() {
    let p = mutated(
        "    runs-on: ${{ matrix.os }}",
        "    runs-on: ${{ matrix.os }}\n    permissions: write-all",
    );
    assert!(p.iter().any(|m| m.contains("job `build`")), "{p:?}");
}

#[test]
fn red_proof_extra_write_grants_on_publish_are_rejected() {
    let p = mutated(
        "      contents: write",
        "      contents: write\n      id-token: write",
    );
    assert!(
        p.iter().any(|m| m.contains("`publish` write grants")),
        "{p:?}"
    );
}

#[test]
fn red_proof_disabled_version_gate_is_rejected() {
    let p = mutated(
        "if [ \"${GITHUB_REF_NAME}\" != \"v${version}\" ]; then",
        "if false && [ \"${GITHUB_REF_NAME}\" != \"v${version}\" ]; then",
    );
    assert!(
        p.iter()
            .any(|m| m.contains("tag-equals-version") || m.contains("bypass")),
        "{p:?}"
    );
}

#[test]
fn red_proof_branch_filter_on_push_is_rejected() {
    let p = mutated("    tags: ['v*']", "    tags: ['v*']\n    branches: [main]");
    assert!(p.iter().any(|m| m.contains("`push:` children")), "{p:?}");
}

#[test]
fn red_proof_rosetta_missing_must_fail_a_tag_push() {
    let p = mutated("::error::Rosetta", "::warning::Rosetta");
    assert!(p.iter().any(|m| m.contains("Intel smoke")), "{p:?}");
}

#[test]
fn red_proof_checkout_must_not_persist_credentials() {
    let p = mutated("          persist-credentials: false\n", "");
    assert!(p.iter().any(|m| m.contains("persist-credentials")), "{p:?}");
}

#[test]
fn red_proof_tag_must_be_annotated_and_on_main() {
    let p = mutated(
        "git merge-base --is-ancestor",
        "git merge-base --not-checked",
    );
    assert!(p.iter().any(|m| m.contains("is-ancestor")), "{p:?}");
    let p = mutated("is not annotated", "is fine");
    assert!(p.iter().any(|m| m.contains("is not annotated")), "{p:?}");
}
