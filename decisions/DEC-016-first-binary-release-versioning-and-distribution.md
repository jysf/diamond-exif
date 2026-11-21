---
insight:
  id: DEC-016
  type: decision
  confidence: 0.75
  audience:
    - developer
    - operator
    - agent

agent:
  id: claude-opus-5-5
  session_id: null

project:
  id: PROJ-001
repo:
  id: diamond-exif

created_at: 2026-11-20
supersedes: null
superseded_by: null
status: accepted
deciders: [jysf, claude]
affected_scope:
  - ".github/workflows/release.yml"
  - "Cargo.toml"
  - "tests/release_layout.rs"

tags:
  - release
  - versioning
  - distribution
  - supply-chain
---

# DEC-016: First binary release: SemVer pre-release tags, GitHub-only distribution, a locked and least-privilege workflow

## Decision

The first public release is `v0.1.0-alpha.1`: a GitHub pre-release carrying the `exif-audit` binary for three targets and a `SHA256SUMS` file, built by a tag-triggered, dry-runnable, least-privilege workflow that uses four actions only (checkout, dtolnay/rust-toolchain, upload-artifact, download-artifact); the library is not published (`publish = false`).

## Context

`exif-audit` (SPEC-016, DEC-011) is complete, fuzzed and oracle-checked but runnable only from a clone with a Rust toolchain. The repository is public, so a release is an outward-facing, partly irreversible act. The writer API is days old (`StripPolicy` and `StripError` are `non_exhaustive`), there is no real-camera corpus and no 24 h fuzz run, and a crates.io publish can be yanked but never removed. SPEC-024 designs the cut; this record keeps the reasons.

## Decisions

1. **SemVer pre-release tags** `v0.MINOR.PATCH-alpha.N`. `0.1.0-alpha.1` signals "unstable; the CLI contract is intended but not frozen".
2. **`publish = false`** in `Cargo.toml`, so an accidental `cargo publish` fails. Removing it is a deliberate, separate release spec (the library publish).
3. **Four actions only:** `actions/checkout@v4` and `dtolnay/rust-toolchain` (the latter is third-party; both already in `ci.yml`), plus GitHub's own `actions/upload-artifact@v4` and `actions/download-artifact@v4`, which are needed to pass archives between jobs. The release is created with the `gh` CLI that runners ship with, authenticated by `GITHUB_TOKEN`. A smaller supply-chain surface suits a security-flavoured tool. `tests/release_layout.rs` lists the allowed set exactly. Every checkout sets `persist-credentials: false`.
4. **Least privilege.** Workflow-level `permissions: contents: read`; only the `publish` job has `contents: write`. Triggers are `push` of tags `v*` and `workflow_dispatch`, never a pull request.
5. **Dry run first.** `workflow_dispatch` takes `dry_run` (default true): it checks, builds the three targets, runs the smoke checks on each unpacked archive, writes and verifies `SHA256SUMS` and uploads everything as workflow artifacts. The release step runs only when the trigger is a tag push, so a dry run (or any dispatch) never creates a release. A dispatch with `dry_run: false` is rejected before any build unless the ref is a tag, and even on a tag it only rehearses: the release comes from the tag push itself.
6. **Locked, reproducible builds.** `cargo build --release --locked --bin exif-audit --target <t>`, the command CI already proves. Archive `exif-audit-<version>-<target>.tar.gz` holds `exif-audit`, `LICENSE-MIT`, `LICENSE-APACHE`, `README.md`; one `SHA256SUMS` covers all archives. A first job fails the run before any build when the tag is not `v` plus the `Cargo.toml` version, and on a tag push also when the tag is not annotated (read from the GitHub API with `gh api repos/<repo>/git/ref/tags/<tag>`, object type `tag`; a lightweight tag reports `commit`) or its commit is not an ancestor of `origin/main`. The API is used because `actions/checkout@v4` (issue #290) replaces an annotated tag with a lightweight one when it refetches, so `git cat-file` on the checked-out ref would reject every real annotated tag. A GitHub tag ruleset restricting who may create `v*` tags is recommended; it is a repository setting the maintainer applies, not something the workflow enforces.
7. **Build provenance in `--version`** is recorded in DEC-011's amendment: `exif-audit <version> (<ref>)` when `EXIF_AUDIT_BUILD_REF` was set at compile time and is clean (non-empty, at most 64 characters, only ASCII alphanumerics, `.`, `-`, `_`; anything else counts as unset) (`option_env!`, no build script, no dependency).
8. **Pre-release flag.** `gh release create --prerelease --verify-tag`, notes from the `CHANGELOG.md` section for the version (the job fails if it is empty), assets attached.
9. **Not in this release, named so nobody assumes them:** code signing and macOS notarization (the binary is unsigned; the README documents the quarantine step), build attestations, a Homebrew tap, a `cargo install --git` recommendation, Windows (the binary has `#[cfg(unix)]` paths never built or tested there) and Linux arm64.

## Alternatives Considered

- **Publish the library to crates.io now.** Rejected: irreversible, and the writer API is not settled.
- **A release action (for example `softprops/action-gh-release`).** Rejected: an extra third-party dependency with `contents: write` for a one-line `gh` command.
- **Create the release on any dispatch.** Rejected: no way to rehearse without publishing on a public repository.
- **Windows and Linux arm64 targets.** Rejected for now: untested code paths and no demand.

## Consequences

- **Positive:** a stranger can download, verify and run the CLI; the cut is rehearsable; rollback is complete (`gh release delete <tag> --cleanup-tag --yes`; nothing else was published).
- **Negative:** unsigned macOS binaries need a documented manual step; there is no provenance attestation beyond the checksums; the `x86_64-apple-darwin` smoke run needs Rosetta on the runner.
- **Neutral:** `Cargo.toml` and `Cargo.lock` move from `0.0.0` to `0.1.0-alpha.1`.

## Validation

The dry run produces three archives that unpack, verify and pass the four smoke checks; `tests/release_layout.rs` has a permanent mutation test for each designed property (triggers, permissions, the exact release guard, the version gate, allowed actions, `--locked`, credentials, Rosetta policy, tag checks). Revisit when signing or a library publish is on the table, or if the musl or Intel macOS build fails on the runner.

## References

- Related specs: SPEC-024, SPEC-016
- Related decisions: DEC-011 (the `--version` line), DEC-006 (release specs are two-phase)
