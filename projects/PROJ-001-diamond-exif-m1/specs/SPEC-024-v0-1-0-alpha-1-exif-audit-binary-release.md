---
# A RELEASE SPEC is the spec for cutting a release — tagging, building the
# artifact, publishing it through each channel, and confirming it actually runs
# on a machine that didn't build it. See AGENTS.md "During ship" and
# docs/decisions/DEC-006.
#
# It uses the same task.* schema as a normal spec (so `just validate`,
# `just cost-audit`, and `just status` treat it as first-class), with
# `task.type: release`. The value it adds over a plain spec is the
# ## Release Pre-Flight checklist below: a generic, portable set of
# runtime/operational categories that every release hits, so each project
# stops re-earning the same gotchas one release at a time. This variant
# delegates the cut to an implementer via handoffs/HANDOFF-*.md.

task:
  id: SPEC-024
  type: release                    # epic | story | task | bug | chore | release
  cycle: verify  # frame | design | build | verify | ship
  blocked: false
  priority: high                   # critical | high | medium | low
  complexity: M                    # XS | S | M | L | XL | XXL — the EXPECTED size, set at design
                                   #   (XL/XXL almost certainly means it's a stage, not a spec)
  complexity_actual: null          # stamped at ship: what it ACTUALLY took, same scale.
                                   #   Expected-vs-actual drift is what `just calibration` reads.

project:
  id: PROJ-001
  stage: STAGE-006
repo:
  id: diamond-exif

handoff:
  from_agent: claude-opus-5-5  # from .repo-context tier_map.design (DEC-005)
  to_agent: null                   # filled when HANDOFF is created (any agent — see docs/porting.md)
  created_at: null

references:
  decisions: [DEC-011]
  constraints: []                  # [constraint-id-1, constraint-id-2]
  related_specs: [SPEC-016]

value_link: "The free exif-audit CLI, the open-core funnel the project thesis rests on, becomes downloadable by someone who is not the maintainer."

# Metered cycles (build/verify) require a real tokens_total on a shipped
# release — same rule as any spec. See AGENTS.md §4 and docs/cost-tracking.md.
cost:
  # Optional PREDICTION of the total tokens this spec will take, set at
  # design. Never a gate — its only job is to be compared with the actual
  # below (`just calibration`), so you learn whether you systematically
  # under- or over-estimate. null = didn't predict.
  tokens_estimate: 450000
  sessions:
    - cycle: build
      handoff: HANDOFF-050
      agent: claude-sonnet-5-5
      interface: other
      basis: agent-result
      tokens_total: 281773
      estimated_usd: 1.86
      duration_minutes: 11
      recorded_at: 2026-11-20
      notes: "Cumulative: build 124,602 + verify-round-1 follow-up 157,171 = 281,773 tokens at 6.60 USD/Mtok = 1.86 USD; 387,304 ms + 244,061 ms = 10.5 min; 18 + 13 tool uses. Harness subagent_tokens, the same basis as earlier specs. The builder reported estimates, not used."
    - cycle: verify
      handoff: HANDOFF-051
      agent: claude-opus-5-5
      interface: other
      basis: agent-result
      tokens_total: 133171
      estimated_usd: 0.88
      duration_minutes: 7
      recorded_at: 2026-11-20
      notes: "Round 1, PUNCH LIST (no High, 2 Medium, 8 Low). Orchestrator correction: tokens_total is the harness subagent_tokens (133,171; 29 tool uses; 392,796 ms = 6.5 min) at 6.60 USD/Mtok = 0.88 USD. The verifier self-reported 1,982,859 (transcript, cache reads included), not used."
  totals:
    tokens_total: 414944
    estimated_usd: 2.74
    session_count: 2
---

# SPEC-024: v0.1.0-alpha.1 exif-audit binary release

## Context

STAGE-006, one release spec. `exif-audit` (SPEC-016) is complete, documented, fuzzed and oracle-checked, and is the free open-core funnel the project thesis rests on, but nobody but the maintainer can run it: it needs a Rust toolchain and a clone. This spec cuts the first public **pre-release**: a GitHub release carrying the `exif-audit` binary for three targets, with checksums, built by a tag-triggered workflow. The library (`exif-core`) is deliberately NOT published: the writer API is days old (`StripPolicy`, `StripError` are `non_exhaustive` and may still change), no real-camera corpus or 24 h fuzz run exists yet, and a crates.io publish cannot be undone (a version can be yanked, never removed). The crates.io names `exif-core`, `diamond-exif`, `exif-audit` were free on 2026-11-11.

The repository is **public**, so the release is public. That makes the cut an outward-facing action: Phase 2 (the tag push) happens only on the user's explicit go-ahead in chat, never as part of build, verify or ship.

## Release Scope

- **Version / tag:** `v0.1.0-alpha.1` (scheme `semver`, already set in `.repo-context.yaml`; `just next-version` only suggests, the version is chosen here). `Cargo.toml` and `Cargo.lock` move from `0.0.0` to `0.1.0-alpha.1`; `exif-audit --version` must report it.
- **What's included:** the work of STAGE-001 to STAGE-005 as it reaches a CLI user: reading and auditing JPEG and TIFF (and DNG), the privacy report in text, JSON and CSV, the exit-code contract, `--policy` previews. The strip **writer is library-only**; the CLI does not write files (DEC-011) and the release notes must say so.
- **Delivery shapes:** **binary** only (GitHub release assets). Library, package-manager channel, service: not in this release.
- **Targets (3):** `x86_64-unknown-linux-musl` (static, runs on any Linux), `aarch64-apple-darwin`, `x86_64-apple-darwin`. Windows is out: the binary has `#[cfg(unix)]` code paths (FIFO and device refusal in `src/bin/exif-audit/read.rs`) and has never been built or tested there. Linux arm64 is out for now (needs a cross-linker or an arm runner; no demand yet).

## Decisions made here (the design; DEC-014 records them)

1. **SemVer pre-release tags** `v0.MINOR.PATCH-alpha.N`. `0.1.0-alpha.1` signals "unstable, the CLI contract (DEC-011) is intended but not frozen".
2. **`publish = false` in `Cargo.toml` for now**, so an accidental `cargo publish` fails. Removing it is a deliberate, separate act (a later release spec: the library publish).
3. **Four actions only**: `actions/checkout@v4`, `dtolnay/rust-toolchain` (third-party, already used in CI), and GitHub's own `actions/upload-artifact` and `actions/download-artifact` (v4; needed to pass archives between jobs and to keep the dry-run artifacts). No other action. The release is created with the `gh` CLI that is preinstalled on runners, authenticated by the workflow's `GITHUB_TOKEN`. Fewer actions means a smaller supply-chain surface for a security-flavoured tool.
4. **Least privilege.** Workflow-level `permissions: contents: read`; only the publish job gets `contents: write`. Triggers: `push` of tags matching `v*` and `workflow_dispatch` only; never `pull_request`, never `pull_request_target`.
5. **Dry run first.** `workflow_dispatch` with input `dry_run` (default `true`) builds all three targets, runs the smoke checks and uploads the archives as workflow artifacts, and **does not create a release**. The release is created only for a tag push; a dispatch with `dry_run: false` is rejected unless the ref is a tag, and even then it only rehearses.
6. **Reproducible, locked builds:** `cargo build --release --locked --bin exif-audit` (the same command CI's `exif-audit-release` job already proves). Archive layout `exif-audit-<version>-<target>.tar.gz` containing `exif-audit`, `LICENSE-MIT`, `LICENSE-APACHE`, `README.md`; one `SHA256SUMS` file over all archives.
7. **Build provenance in `--version`** (release pre-flight item 1). `exif-audit --version` prints `exif-audit <crate version>` today (DEC-011). Amend DEC-011: when the environment variable `EXIF_AUDIT_BUILD_REF` was set at compile time (read with `option_env!`, no build script, no dependency), the line is `exif-audit <crate version> (<ref>)`; otherwise unchanged. The release workflow sets it to the short commit SHA. Existing CLI tests must still pass unchanged; add a pure `version_line(Option<&str>) -> String` so the format is unit-tested without rebuilding.
8. **Pre-release flag.** The release is created with `--prerelease` and `--verify-tag`, notes from the CHANGELOG section, assets attached.
9. **Not in this spec (named so nobody assumes them):** code signing and macOS notarization (the binary is unsigned: Gatekeeper will quarantine a browser download; the README says how to clear it and that `curl`/`gh` downloads are not quarantined), build attestations (`actions/attest-build-provenance`), a Homebrew tap, a `cargo install --git` recommendation, Windows.

## Inputs

- `.github/workflows/ci.yml` (the `exif-audit-release` job is the template for build and smoke steps), `Cargo.toml`, `Cargo.lock`, `src/bin/exif-audit/main.rs` (line 44, the `--version` text), `decisions/DEC-011-exif-audit-cli-contract.md`, `README.md`, `CHANGELOG.md` (its `[Unreleased]` section stops at SPEC-013: write the missing entries as one concise summary, not per spec), `SECURITY.md`, `guidance/toolchain-brief.md`, `docs/versioning.md`, `projects/_templates/release-spec.md` (the pre-flight categories below).
- `just build-info` prints the version and commit ref stamp (this repo has no use for its other fields).

## Outputs

- **New:** `.github/workflows/release.yml`; `decisions/DEC-014-first-binary-release-versioning-and-distribution.md` (decisions 1 to 6, 8, 9 above); `tests/release_layout.rs` (static checks, see Failing Tests).
- **Modified:** `Cargo.toml` (`version`, `publish = false`), `Cargo.lock`, `src/bin/exif-audit/main.rs` (`version_line`), `decisions/DEC-011` (amendment: the `--version` line), `README.md` (an "Install" section: download, verify, unquarantine, run; the line that the CLI never writes files), `CHANGELOG.md` (`## [0.1.0-alpha.1] - <date>` with a concise summary; `[Unreleased]` left empty), the stage file, the project brief, and a `## Release Notes` filled in this spec.
- **Not created in the build phase:** any tag, any GitHub release, any crates.io action.

## Release Notes

**Headline:** `exif-audit`, the free privacy scanner for image metadata, is now a download: point it at a folder of JPEG, TIFF or DNG files and it reports GPS, device serials, authorship, thumbnails and MakerNotes, with exit codes a CI job can act on.

- Read-only: it never writes or changes a file, reads nothing but the paths you name, and keeps no state.
- Text, JSON and CSV reports; `--policy` previews what stripping would leave; `--fail-on` turns findings into a failing build.
- Checked against ExifTool as an oracle and fuzzed (seven fuzz targets in CI); panic-free by construction.
- Binaries: Linux x86_64 (static), macOS Apple silicon and Intel. Unsigned.

**Breaking / action required:** none (first release). Alpha: the CLI contract is intended but not frozen. The library crate is not published yet; the strip writer is library-only and is not part of this release.

## Release cut is two-phase

- **Phase 1 (this spec's build and verify): reversible prep.** Everything under Outputs, merged by a normal CI-gated PR. Then the **dry run**: the orchestrator pushes the branch, runs `gh workflow run release.yml --ref <branch> -f dry_run=true`, downloads the three artifacts and checks them (see pre-flight).
- **Phase 2: the irreversible cut, on the user's explicit go-ahead only.** After Phase 1 merges: `git tag -a v0.1.0-alpha.1 <merge commit> && git push origin v0.1.0-alpha.1`. The spec ships "prep-complete, cut-deferred" until the user says go; the Cut Record is filled afterwards.

## Release Pre-Flight

Timing: `[now]` verified in Phase 1 with evidence; `[cut]` can only be verified after the tag.

- [ ] **1. Version / tag integrity & build provenance** `[now]` for the build, `[cut]` for the tag. The version in `Cargo.toml`, `exif-audit --version` and the archive names all agree; `--version` of a CI-built binary shows the short SHA of the release commit; exactly one tag `v0.1.0-alpha.1` points at the release commit and it is an annotated tag.
      - Evidence: `git tag --points-at HEAD`; `./exif-audit --version` from the dry-run artifact; `test` in `tests/release_layout.rs` that the three agree.
- [ ] **2. Artifact trust on a clean host** `[cut]`. A browser-downloaded macOS binary is quarantined (unsigned); document the `xattr -d com.apple.quarantine` step and confirm that a `curl` or `gh release download` copy runs without a prompt. Linux: runs on a clean container with no toolchain (`docker run --rm -v ... debian:stable-slim` or the equivalent).
- [ ] **3. Distribution-channel trust** `[cut]`. The only channel is the GitHub release. A stranger can `gh release download` or `curl -L` the asset and verify it with `shasum -a 256 -c SHA256SUMS`; the README commands are copy-pasted and work.
- [ ] **4. Data isolation** `N/A`: the tool reads only the paths named and holds no production data (DEC-011); stated, not tested.
- [ ] **5. Runtime smoke on a clean host** `[now]` on the dry-run artifacts (the workflow runs the smoke steps on the freshly built archive, unpacked into an empty directory) and `[cut]` on the published asset: `--version`, a JSON report over the two committed full-house fixtures exits 1 with `files: 2`, `--fail-on none` exits 0, a missing path exits 3, a bad flag exits 2 (the four checks of the existing CI job). Run the downloaded macOS arm64 artifact on this Mac, outside the repo directory.
- [ ] **6. Rollback / uninstall** `[cut]`. `gh release delete v0.1.0-alpha.1 --cleanup-tag --yes` removes the release and the tag; nothing else was published (no crates.io, no tap), so rollback is complete. Honest verification: (1) the dispatch dry run exercises everything except `gh release create`; (2) the delete syntax and the create flags (`--verify-tag`, `--prerelease`, `--notes-file`, `--title`) were checked against the installed `gh`; (3) the real delete is run only if the real cut turns out bad; (4) optionally, rehearse the whole path in a private scratch fork. A throw-away tag cannot work: the `check` job rejects any tag that is not `v` plus the `Cargo.toml` version, so `publish` would never run.

## Acceptance Criteria

- [x] `Cargo.toml` version is `0.1.0-alpha.1`, `publish = false`, `Cargo.lock` agrees; `exif-audit --version` prints `exif-audit 0.1.0-alpha.1`, and `exif-audit 0.1.0-alpha.1 (<ref>)` when built with `EXIF_AUDIT_BUILD_REF=<ref>`; existing CLI tests pass unchanged.
- [x] `release.yml` triggers only on `push: tags: ['v*']` and `workflow_dispatch`; workflow-level permissions are read-only; only the publish job has `contents: write`; it uses no action other than `actions/checkout@v4`, `dtolnay/rust-toolchain` (third-party, already in CI), `actions/upload-artifact@v4` and `actions/download-artifact@v4`; the build uses `--locked`; a dry run never creates a release.
- [ ] (not verified in build: needs the orchestrator dry run) The dry run on the branch produces three archives and `SHA256SUMS`; each archive unpacks to `exif-audit`, both licences and the README; `shasum -a 256 -c` passes; the smoke checks pass on each target; the macOS arm64 binary runs on this Mac.
- [x] A tag whose version does not equal `Cargo.toml`'s fails the workflow before any build.
- [x] `README.md` has an Install section (download, checksum, quarantine note, first run) and says the CLI never writes files and that the library is not published; `CHANGELOG.md` has the `[0.1.0-alpha.1]` section; DEC-014 exists; DEC-011 is amended.
- [x] All existing gates stay green (`just lint`, `test`, `deny`, `forbid-unsafe`, `validate`, `cost-audit`) and the CI `exif-audit-release` job is unchanged and green.

## Failing Tests

Written first, red before the change.

- **`src/bin/exif-audit/main.rs` unit tests:** `version_line(None)` is `exif-audit <CARGO_PKG_VERSION>`; `version_line(Some("abc1234"))` appends ` (abc1234)`; an empty ref is treated as none.
- **`tests/release_layout.rs`** (std only, reads files like `tests/fuzz_layout.rs` does):
  - `"cargo version is the alpha and publish is false"`.
  - `"release workflow triggers, permissions and actions are as designed"` (string and structure checks on `release.yml`: no `pull_request`, read-only default permissions, `contents: write` only on the publish job, only the two allowed actions, `--locked`, `--prerelease`, `--verify-tag`, a `SHA256SUMS` step, a tag-equals-Cargo-version gate).
  - `"changelog and readme carry the release"` (the `[0.1.0-alpha.1]` heading; the Install section's checksum and quarantine lines).
  - `"the three release targets are the designed three"`.
- **Red-proofs:** mutate each static check's subject on a copy (add `pull_request` to the triggers, add `contents: write` at the top, change the version) and show the matching test goes red; say so in the handback.

## Non-Goals

Publishing `exif-core`; code signing, notarization, attestations; Homebrew or any package channel; Windows or Linux arm64 builds; a CLI that writes files; changing any report format or exit code; any strip, audit or parser change; stage 7 (PNG and WebP).

## Notes for the Implementer

- Model the build and smoke steps on the `exif-audit-release` job in `ci.yml`; do not touch that job.
- Validate the workflow YAML by reading it carefully and by `python3 -c 'import yaml'` only if PyYAML is present; there is no `act` here. The real test is the dry run, which the orchestrator triggers after you hand back; make the workflow easy to dry-run (`workflow_dispatch`, input `dry_run`, no release step on a dry run).
- `x86_64-unknown-linux-musl` needs `rustup target add` and, for a pure-Rust binary with no C dependencies, no extra linker; if it does not link on the runner, report it rather than adding a dependency. `x86_64-apple-darwin` is cross-built on the arm64 macOS runner (`rustup target add x86_64-apple-darwin`); its smoke step cannot run natively there unless Rosetta is available: run it under `arch -x86_64` and say in the handback if that is not possible.
- Do not push, do not create tags, do not run the workflow, do not open a PR: the orchestrator does that. Do not add a dependency.
- Budget: one builder, sonnet, about 100 tool calls or 250k context. Cold verify on opus, one round; a second only for High or Medium findings. The pre-flight items marked `[cut]` are verified by the orchestrator at Phase 2 and recorded in the Cut Record; the verifier checks the `[now]` items from the dry-run evidence.

## Cut Record

*Filled after Phase 2 by whoever runs the cut.* Not yet cut.

---

## Reflection

*Appended during **ship**. Four questions, short answers.*

1. **What would I do differently next release?**
   — <answer>

2. **Does any template, constraint, or decision need updating?** (e.g. a new
   pre-flight category to add to this template)
   — <answer — if yes but not done this session, record it in
   `/guidance/signals.yaml`. See `docs/signals.md`.>

3. **Is there a follow-up spec I should write now before I forget?**
   — <answer>

4. **Where was the worst defect caught?** — one word from a fixed vocabulary so
   the defect-escape distribution is greppable across specs:
   `design` | `build` | `verify` | `ship` | `escaped` (reached prod/runtime) |
   `none` (clean first try).
   — <one word>
   *(A release-phase `escaped` defect means a pre-flight category above was
   missing or skipped — grow the checklist.)*
