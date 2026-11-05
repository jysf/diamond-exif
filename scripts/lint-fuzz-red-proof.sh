#!/usr/bin/env bash
# scripts/lint-fuzz-red-proof.sh — proves the FUZZ graph's clippy gate
# (`just lint-fuzz`, the CI job `rust / clippy — fuzz graph`) actually bites
# (constraint `oracle-must-be-shown-red` applied to a gate rather than an
# oracle — SPEC-003/FU-14 widened it to cover gates; SPEC-011). Mechanism per
# DEC-009 (the library's `scripts/lint-red-proof.sh`), here with ONE injected
# fault rather than five, and a fuzz-specific twist explained below.
#
# `fuzz/` is a SECOND cargo graph that DEC-011 keeps outside the library's
# `[dependencies]`, so the library's own clippy — `just lint` / `just lint-ci`
# / the `clippy` CI job, all rooted at the repo's Cargo.toml — never compiles a
# single fuzz target. DEC-027 adds the second invocation
# (`--manifest-path fuzz/Cargo.toml`), exactly as DEC-011 did for licensing.
# A gate that has never gone red cannot be told from one that never ran, so
# this script runs it as a CONTROLLED experiment on a temp-dir copy:
#
#   1. copy the two crates the fuzz graph needs (the library by path + fuzz/)
#      to a temp dir
#   2. CONTROL — run the exact clippy invocation the gate runs, on the
#      UNMUTATED copy. It must exit 0 (the fuzz crate is clippy-clean — SPEC-011
#      Notes: 0 warnings on 0.1.98 and 0.1.99, even under the five panic-free
#      lints, which is why this spec ships NO code fixes).
#   3. inject ONE default-clippy violation into a COPIED fuzz target
#   4. MUTATION — run the same invocation again, and once more WITHOUT the
#      gate's `-D warnings`
#
# The working tree is never mutated, so there is no restore path to get wrong.
#
# ── Why the control run exists (DEC-009, the whole point) ───────────────────
# A red with no control proves only "clippy failed", not "the gate rejected the
# injection". The library's proof learned this the hard way (DEC-009, round 2:
# a mis-landed injection made clippy fail with every expected lint NAME in the
# log and not one lint firing). With the unmutated copy clean under the same
# command, a red below is attributable to the injection by construction.
#
# ── Why the injected fault is a WARN-level clippy lint, and why -D warnings ──
# ── is proven load-bearing (the fuzz-specific twist) ────────────────────────
# The library crate carries a crate-root `#![deny(...)]`; its red-proof injects
# violations of THAT policy and proves it is `deny`-level even without CI's
# blanket `-D warnings`. The fuzz crate is the opposite: `fuzz/Cargo.toml` has
# NO `[lints]` table and the targets carry NO `#![deny]` — so `-D warnings` is
# the ONLY thing with teeth here. The chosen scope (DEC-027) is clippy's default
# lint set promoted to deny by that flag, NOT the five panic-free lints.
#
# So the injected fault is `clippy::unnecessary_cast` — a DEFAULT, WARN-level
# clippy lint (not a panic-free-only one; those are out of scope and the crate
# passes them anyway). That choice lets this proof assert the thing that makes
# the fuzz gate a gate:
#   * WITH `-D warnings` the injected copy is REJECTED (assertions 2–4), and
#   * WITHOUT `-D warnings` the SAME copy exits 0 (assertion 5).
# Drop `-D warnings` from the recipe or the CI job and the gate stops catching
# this entire class — assertion 5 is what pins that the flag is load-bearing,
# the fuzz analogue of the library proof's severity run (DEC-009 assertion 5),
# inverted: there the source `#![deny]` is load-bearing, here the flag is.
#
# ── The injection must be a CLIPPY lint, matched by its help URL ────────────
# Assertion 3 matches clippy's `index.html#<lint>` help-URL fragment, emitted
# ONLY when a lint FIRES — never by a rendered source span, and never for a
# rustc lint (an unused variable / import would fail this gate by exit code but
# emit no clippy help URL, so it could not be name-matched robustly). That is
# why the fault is a clippy lint (`unnecessary_cast`) and not, say, a bare
# `unused_variables`. The bare name is NOT sufficient: it can appear in a
# rendered span. Borrowed straight from `scripts/lint-red-proof.sh`.
#
# ── Clippy pinning (PATCH-004) ──────────────────────────────────────────────
# This script runs BARE `cargo clippy`, exactly as `scripts/lint-red-proof.sh`
# does, and names which clippy proved the result (assertion 0). The fault is a
# long-stable default lint that fires identically on Homebrew's local clippy and
# on CI's floating `@stable`, so the proof is version-agnostic by construction —
# the pinning that matters for the *honest-tree* gate (`just lint-fuzz` carries
# the `~/.cargo/bin/cargo +stable` PATH pin like `just lint-ci`) does not matter
# for proving the gate's mechanism bites. CI runs this job under
# `dtolnay/rust-toolchain@stable` + `components: clippy`, so there the bare
# `cargo clippy` is stable's.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/_lib.sh
source "${SCRIPT_DIR}/_lib.sh"

# _lib.sh derives REPO_ROOT from the CWD. This script must work from anywhere
# (CI, `just lint-fuzz-red-proof`, and by hand), so resolve the root from the
# script's own location instead.
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

require_initialized

FUZZ_MANIFEST="${REPO_ROOT}/fuzz/Cargo.toml"
[ -f "${REPO_ROOT}/Cargo.toml" ] || die "missing ${REPO_ROOT}/Cargo.toml — the library the fuzz crate path-depends on is not here; nothing to copy"
[ -f "$FUZZ_MANIFEST" ] || die "missing ${FUZZ_MANIFEST} — the fuzz graph this proof exists to test is gone"

# The one fuzz target the fault is injected into, and the lint it must trip,
# hard-coded ON PURPOSE (same reasoning as lint-red-proof.sh's EXPECTED_LINTS):
# deriving either from the mutation would make the proof follow the fault
# instead of pinning it. `tiff_ifd` is this crate's only fuzz target; any target would
# do, because `--all-targets` compiles them all.
INJECT_TARGET_REL="fuzz_targets/tiff_ifd.rs"
INJECT_TARGET="${REPO_ROOT}/fuzz/${INJECT_TARGET_REL}"
EXPECTED_LINT="unnecessary_cast"
# The line every target opens its body with. The injection goes immediately
# after it, so the fault lands as the first statement of live closure code —
# not a standalone `fn`, which `--all-targets` would also flag as `dead_code`
# (a rustc lint with no clippy help URL) and muddy assertion 3.
MACRO_OPEN='fuzz_target!(|data: &[u8]| {'
# `data.len()` is already `usize`, so `as usize` is `clippy::unnecessary_cast`
# — a default, warn-level clippy lint. The binding is underscore-prefixed, so
# it trips no `unused_variables`; it is a plain `let`, so it trips no
# `let_underscore`/`no_effect`. Exactly one diagnostic, by measurement.
INJECT_LINE_TEXT='    let _red_proof_unnecessary_cast = data.len() as usize;'
INJECT_MARKER='_red_proof_unnecessary_cast'

[ -f "$INJECT_TARGET" ] || die "missing ${INJECT_TARGET} — cannot inject the red-proof fault"

# ── Assertion 0: clippy actually ran, AND we can say WHICH clippy ───────────
# PATCH-004. Asserting clippy is *present* is not the same as knowing what it
# is: on the maintainer's machine the default toolchain is nightly (which ships
# no cargo-clippy) and Homebrew's answers bare `cargo clippy` instead. A proof
# that cannot name the compiler that proved the gate is reporting a result it
# has not established — the same defect it exists to catch, one level up.
if ! CLIPPY_VERSION="$(cargo clippy --version 2>&1)"; then
    die "\`cargo clippy --version\` failed — clippy is not available, so this proof can prove NOTHING. Refusing to report green. Output: ${CLIPPY_VERSION}"
fi
if ! CLIPPY_BIN="$(command -v cargo-clippy 2>/dev/null)"; then
    die "\`cargo clippy --version\` answered but \`cargo-clippy\` cannot be resolved on PATH, so this proof cannot name the binary that produced its result. Refusing to report green."
fi
CLIPPY_ID="${CLIPPY_VERSION} (${CLIPPY_BIN})"
info "clippy is present: ${CLIPPY_ID}"

# ── Copy the two crates the fuzz graph needs ────────────────────────────────
# The fuzz package path-depends on the library (`path = ".."`), so clippy must
# compile the library too — copy its Cargo.toml + src/ alongside fuzz/. The
# library's dev-dependencies (toml, ssimulacra2) are NOT part of the fuzz graph
# (dev-deps of a path dependency are never resolved), so they need not come
# along — measured: the fuzz clippy compiles only libfuzzer-sys's deps + the
# library's lib target. No lockfile is copied: the fuzz graph's lock is
# fuzz/Cargo.lock (gitignored, DEC-011), so cargo resolves it fresh here exactly
# as the honest gate does.
WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/exif-core-fuzz-red-proof.XXXXXX")"
cleanup() {
    rm -rf "$WORK_DIR"
}
trap cleanup EXIT

mkdir -p "${WORK_DIR}/src" "${WORK_DIR}/fuzz/fuzz_targets"
cp "${REPO_ROOT}/Cargo.toml" "${WORK_DIR}/Cargo.toml"
cp -R "${REPO_ROOT}/src/." "${WORK_DIR}/src/"
cp "${FUZZ_MANIFEST}" "${WORK_DIR}/fuzz/Cargo.toml"
cp -R "${REPO_ROOT}/fuzz/fuzz_targets/." "${WORK_DIR}/fuzz/fuzz_targets/"
# Copy a toolchain pin if one ever lands (there is none today). Without this the
# copy would silently compile on a different toolchain than the repo, which
# would make the control run answer a question nobody asked.
for pin in rust-toolchain.toml rust-toolchain .cargo; do
    if [ -e "${REPO_ROOT}/${pin}" ]; then
        cp -R "${REPO_ROOT}/${pin}" "${WORK_DIR}/${pin}"
    fi
done

WORK_MANIFEST="${WORK_DIR}/fuzz/Cargo.toml"
WORK_TARGET="${WORK_DIR}/fuzz/${INJECT_TARGET_REL}"

# The one clippy invocation, defined once. `run_clippy <log> [extra args...]`,
# always `--manifest-path` at the COPIED fuzz manifest.
run_clippy() {
    local log="$1"
    shift
    # ⚠ `--color never` is LOAD-BEARING, not cosmetic. Assertions 3 and 4 grep
    # this log, and CI's clippy colourises even when redirected to a file, which
    # puts a reset sequence between `-->` and the path so a naive grep matches
    # nothing — and under `set -o pipefail` a zero-match grep kills the script
    # before its own `die` can run (lint-red-proof.sh paid for this; PATCH-001).
    cargo clippy --manifest-path "$WORK_MANIFEST" --color never --all-targets "$@" >"$log" 2>&1
}

# ── Assertion 1: THE CONTROL — the unmutated copy must be clean ─────────────
info "control run: the UNMUTATED fuzz copy, exact gate invocation — this MUST pass:"
CONTROL_LOG="${WORK_DIR}/clippy-control.log"
CONTROL_EXIT=0
run_clippy "$CONTROL_LOG" -- -D warnings || CONTROL_EXIT=$?
if [ "$CONTROL_EXIT" -ne 0 ]; then
    cat "$CONTROL_LOG"
    die "the CONTROL run failed (clippy exited ${CONTROL_EXIT}) on the UNMUTATED fuzz copy. Something upstream of this proof is broken: the toolchain, the copy step, or the fuzz crate itself — run \`just lint-fuzz\` and fix what it says. Until then NOTHING downstream here means anything: a red from the mutated run could not be attributed to the injection. (DEC-009: this is the assertion the previous designs did not have.)"
fi
success "control: unmutated fuzz copy is clean (clippy exit 0). A red below is now attributable to the injection."

# ── Inject ONE default-clippy violation into the COPIED target ──────────────
# Find the closure-open line and insert the fault immediately after it, so it
# lands as live code (not a standalone dead fn). If the target's macro shape
# ever changes, grep finds nothing and we die LOUDLY here rather than shipping a
# proof that silently injects nothing and then "passes" its mutation run.
MACRO_LN="$(grep -nF -- "$MACRO_OPEN" "$WORK_TARGET" | head -n1 | cut -d: -f1 || true)"
[ -n "$MACRO_LN" ] || die "could not find the fuzz_target! body opener in ${INJECT_TARGET_REL} (looked for a line containing: ${MACRO_OPEN}). The target's shape changed; fix the injection anchor, do NOT weaken the proof."

INJECTED_TMP="${WORK_TARGET}.injected"
{
    head -n "$MACRO_LN" "$WORK_TARGET"
    printf '%s\n' "$INJECT_LINE_TEXT"
    tail -n "+$((MACRO_LN + 1))" "$WORK_TARGET"
} > "$INJECTED_TMP"
mv "$INJECTED_TMP" "$WORK_TARGET"
INJ_LINE=$((MACRO_LN + 1))

# Structural post-check: the marker is present exactly once, on the line we
# think it is. Guards against an injection that landed in the wrong place (which
# would make the mutation run fail for the wrong reason — the class the control
# cannot distinguish on its own).
MARKER_COUNT="$(grep -cF -- "$INJECT_MARKER" "$WORK_TARGET" || true)"
[ "$MARKER_COUNT" = "1" ] || die "expected exactly one injected marker in the copied ${INJECT_TARGET_REL}, found ${MARKER_COUNT}. The injection did not land as intended."
MARKER_LN="$(grep -nF -- "$INJECT_MARKER" "$WORK_TARGET" | head -n1 | cut -d: -f1)"
[ "$MARKER_LN" = "$INJ_LINE" ] || die "injected marker is on line ${MARKER_LN}, expected ${INJ_LINE} — the insertion perturbed the file."
info "injected \`${EXPECTED_LINT}\` fault into ${INJECT_TARGET_REL} line ${INJ_LINE} (a copy; the working tree is untouched)"

# ── Assertions 2 + 3 + 4: the mutated copy is rejected, by the RIGHT lint ───
info "mutation run: the same invocation on the injected copy — this MUST fail:"
MUT_LOG="${WORK_DIR}/clippy-mutated.log"
MUT_EXIT=0
run_clippy "$MUT_LOG" -- -D warnings || MUT_EXIT=$?
cat "$MUT_LOG"

if [ "$MUT_EXIT" -eq 0 ]; then
    die "the fuzz clippy gate did NOT reject the injected \`${EXPECTED_LINT}\` violation (clippy exited 0) — and the control above was clean, so this is the gate's fault and nothing else. \`just lint-fuzz\`'s \`-D warnings\` is missing or not applying. This is exactly the manufactured-confidence failure oracle-must-be-shown-red exists to catch."
fi

# Assertion 3 — the EXPECTED lint fired, matched on clippy's help-URL fragment,
# not the bare name. `index.html#unnecessary_cast` is emitted by the lint
# machinery when the lint FIRES; `clippy::unnecessary_cast` as a bare string can
# appear in a rendered span. Anchored to the lint name at a URL boundary so a
# longer lint sharing the prefix cannot satisfy it.
assert_lint_fired() {
    local log="$1" label="$2"
    grep -qE -- "index\.html#${EXPECTED_LINT}([[:space:]]|\$)" "$log" \
        || die "${label}: clippy failed, but \`${EXPECTED_LINT}\` never fired (no \`index.html#${EXPECTED_LINT}\` help line). A non-zero exit is not the proof, and neither is the lint's NAME appearing in the log. Either the gate no longer denies this lint, or the injection landed where it does not apply."
}
assert_lint_fired "$MUT_LOG" "mutation run"

# Assertion 4 — the diagnostic is LOCATED on the injected line of the injected
# target. This rules out "the lint name arrived from somewhere else". ⚠ Every
# grep here is guarded: a zero-match grep exits 1, and under pipefail + set -e
# that would kill the script before the `die` can explain itself — a proof that
# dies without a message is indistinguishable from one that never ran
# (PATCH-001, the exact defect class this file guards).
IN_RANGE="$( { grep -oE -- "--> ${INJECT_TARGET_REL}:[0-9]+" "$MUT_LOG" || true; } \
    | { grep -oE '[0-9]+$' || true; } \
    | awk -v ln="$INJ_LINE" '$1 == ln' \
    | sort -un | wc -l | tr -d '[:space:]')"
if [ "$IN_RANGE" -lt 1 ]; then
    die "clippy failed and named \`${EXPECTED_LINT}\`, but no diagnostic span points at the injected line (${INJECT_TARGET_REL}:${INJ_LINE}). The failure is not coming from the injected code."
fi
info "diagnostic located on the injected line: ${INJECT_TARGET_REL}:${INJ_LINE}"

# ── Assertion 5: SEVERITY — the gate's teeth are `-D warnings` ──────────────
# Same mutated copy, same command, minus the gate's `-D warnings`. The fuzz
# crate carries no `#![deny]`, so a default WARN-level lint like
# `unnecessary_cast` exits 0 here — which is the proof that `-D warnings` is the
# ONLY thing promoting it to an error in `just lint-fuzz` / the CI job. Drop the
# flag and the gate stops catching this class; this assertion is what pins it.
info "severity run: the injected copy WITHOUT the gate's -D warnings — a warn-level lint must NOT fail here, proving -D warnings is the gate's teeth:"
SEV_LOG="${WORK_DIR}/clippy-severity.log"
SEV_EXIT=0
run_clippy "$SEV_LOG" || SEV_EXIT=$?
if [ "$SEV_EXIT" -ne 0 ]; then
    cat "$SEV_LOG"
    die "WITHOUT \`-D warnings\`, clippy exited ${SEV_EXIT} on the injected \`${EXPECTED_LINT}\`. That lint is default WARN-level, so a plain \`cargo clippy\` should exit 0 — a non-zero here means the fault was not actually warn-level (wrong injection) or some OTHER deny-level lint fired, and the proof can no longer claim \`-D warnings\` is what gives the fuzz gate its teeth. Re-check the injected fault."
fi
success "fuzz clippy red-proof: control clean (exit 0) → injection rejected WITH -D warnings (exit ${MUT_EXIT}), \`${EXPECTED_LINT}\` fired at ${INJECT_TARGET_REL}:${INJ_LINE} → the SAME injection exits 0 WITHOUT -D warnings, so the flag is the gate's teeth. PROVED BY: ${CLIPPY_ID}."
