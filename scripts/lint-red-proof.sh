#!/usr/bin/env bash
# scripts/lint-red-proof.sh — proves the LIBRARY's panic-free lint policy
# actually bites (constraint `oracle-must-be-shown-red` applied to a gate rather
# than an oracle; SPEC-001 acceptance criterion 5). Mechanism per DEC-009,
# which supersedes DEC-007 (which superseded DEC-006).
#
# A lint policy that has never rejected anything is not a policy. This script is
# a MUTATION TEST of the real crate, run as a CONTROLLED experiment:
#
#   1. copy the crate (Cargo.toml, Cargo.lock, src/, any toolchain pin) to a
#      temp dir
#   2. CONTROL — run the exact clippy invocation CI runs, on the UNMUTATED copy.
#      It must exit 0.
#   3. inject five violations into the COPIED src/lib.rs, immediately after the
#      last inner attribute — so src/lib.rs's own `#![deny(...)]` is the only
#      thing that can reject them
#   4. MUTATION — run the same invocation again, and once more WITHOUT CI's
#      blanket `-D warnings`
#
# The working tree is never mutated, so there is no restore path to get wrong.
#
# ── Why the control run exists (DEC-009, the whole point) ───────────────────
# Three rounds of this gate asserted things about the MUTATED run only: that
# clippy ran, that it exited non-zero, that the expected lint names were in its
# log. None of those can distinguish "failed for the reason I intended" from
# "failed for any reason at all" — and that distinction is the entire value of a
# red-proof. Measured consequence (verify round 2): one legal `//` comment in
# lib.rs's prologue moved the injection ABOVE the inner attributes, where
# `pub fn` is a syntax error; clippy exited 101, rustc RENDERED the multi-line
# `#![deny(...)]` span in the diagnostic — putting every expected lint name in
# the log — and the proof printed success with no lint having fired. Combined
# with `deny`→`allow`, that was seven green gates and a panic shipped in the
# library.
#
# The control closes that class by construction: if the unmutated copy is clean
# under the same command, then a failure of the mutated copy is attributable to
# the injection. No amount of log-grepping can do that.
#
# ── The five assertions ─────────────────────────────────────────────────────
#   0. clippy is actually available (a `cargo` that cannot run clippy exits
#      non-zero and an exit-code-only proof reports green having compiled
#      nothing — verify round 1, measured with a stub cargo).
#   1. CONTROL: unmutated copy, exact CI invocation, exit 0.
#   2. MUTATION: same invocation, exit non-zero.
#   3. All FIVE policy lints fired — matched on clippy's `index.html#<lint>`
#      help URL, which is emitted ONLY when the lint actually fires, never by a
#      rendered source span. Bare lint names are NOT sufficient: they are the
#      text of the policy itself and appear whenever rustc renders it.
#   4. The diagnostics are LOCATED inside the injected line range — the names
#      came from the injected code, not from somewhere else in the file.
#   5. SEVERITY: the mutated copy is rejected even WITHOUT `-D warnings`. That
#      is what makes the policy `deny`-level in the library rather than
#      `warn`-level promoted by a CI flag. With `#![warn(...)]` this run exits
#      0 (measured) and the proof fails, as it should — a consumer running a
#      plain `cargo clippy` would otherwise see nothing.
#
# ── Known residual limits, stated rather than papered over ──────────────────
# - The injection point is found by parsing the attribute prologue. It anchors
#   on the LAST inner attribute (not the first non-prologue line), so comments
#   of any kind in the prologue cannot move it. A prologue shape the parser does
#   not understand fails LOUDLY here rather than injecting somewhere useless.
# - Assertion 3 greps a log. It is sound only because assertion 1 (control) and
#   assertion 4 (span location) bracket it. Do not remove either and keep the
#   grep. In particular: never put a literal clippy help URL in src/lib.rs's
#   prose — a rendered span could then reintroduce the round-2 defect.
# - This proves the policy bites on lib.rs. It says nothing about future modules
#   that carry their own `#[allow(...)]`.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/_lib.sh
source "${SCRIPT_DIR}/_lib.sh"

# _lib.sh derives REPO_ROOT from the CWD. This script must work from anywhere
# (it is run by CI, by `just lint-red-proof`, and by hand), so resolve the root
# from the script's own location instead.
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

require_initialized

LIB_RS="${REPO_ROOT}/src/lib.rs"
[ -f "${REPO_ROOT}/Cargo.toml" ] || die "missing ${REPO_ROOT}/Cargo.toml — nothing to copy"
[ -f "$LIB_RS" ] || die "missing $LIB_RS — the red-proof has nothing to inject into"

# The policy this proof pins, hard-coded ON PURPOSE. Deriving it from
# src/lib.rs would make the proof follow the policy instead of pinning it:
# deleting a lint would delete its own expectation, which is exactly the
# bypass verify round 2 measured (PL-2 — `panic` and `expect_used` were in
# neither the injection nor the expectations, so both could be dropped from the
# policy with all seven gates green). Changing this list is a deliberate act.
EXPECTED_LINTS="clippy::unwrap_used clippy::expect_used clippy::indexing_slicing clippy::panic clippy::arithmetic_side_effects"

# ── Assertion 0: clippy actually ran, AND we can say WHICH clippy ───────────
# PATCH-004. Asserting that clippy is *present* is not the same as knowing what
# it is. Measured 2026-09-06 on the maintainer's machine: the default toolchain
# is nightly, nightly ships NO cargo-clippy, and yet bare `cargo clippy` answers
# — because Homebrew's /opt/homebrew/bin/cargo-clippy (rust 1.97.1) shadows the
# rustup shim. So this proof was passing while linting with a compiler nobody
# selected, and its success line did not say so. One reviewer's machine failed
# loudly here; this one succeeded silently, and the silent success is worse.
#
# A red-proof that cannot name the compiler that proved the policy is reporting
# a result it has not established — the same defect it exists to catch, one
# level up.
if ! CLIPPY_VERSION="$(cargo clippy --version 2>&1)"; then
    die "\`cargo clippy --version\` failed — clippy is not available, so this proof can prove NOTHING. Refusing to report green. Output: ${CLIPPY_VERSION}"
fi
if ! CLIPPY_BIN="$(command -v cargo-clippy 2>/dev/null)"; then
    die "\`cargo clippy --version\` answered but \`cargo-clippy\` cannot be resolved on PATH, so this proof cannot name the binary that produced its result. Refusing to report green."
fi
CLIPPY_ID="${CLIPPY_VERSION} (${CLIPPY_BIN})"
info "clippy is present: ${CLIPPY_ID}"

# ── Find the injection point in src/lib.rs ──────────────────────────────────
# Returns the line number on which the LAST inner attribute of the prologue
# ends. Injecting at that line + 1 is legal Rust by construction: inner
# attributes must precede every item, so anything after the last one is an item
# position.
#
# Anchoring on the last attribute rather than on "the first line that isn't
# prologue" is what kills the comment class: `//`, `//!` and `///` lines are all
# skipped, and a shape this parser does not understand (a `/* */` header, say)
# terminates the scan and fails loudly below instead of silently relocating the
# injection above the attributes.
find_last_inner_attr_end() {
    local file="$1" line trimmed lineno=0 depth=0 opens closes last=0
    while IFS= read -r line || [ -n "$line" ]; do
        lineno=$((lineno + 1))
        if [ "$depth" -gt 0 ]; then
            # Inside a multi-line `#![...]` — track bracket depth to its close.
            opens="${line//[^\[]/}"
            closes="${line//[^\]]/}"
            depth=$((depth + ${#opens} - ${#closes}))
            if [ "$depth" -le 0 ]; then
                depth=0
                last=$lineno
            fi
            continue
        fi
        trimmed="${line#"${line%%[![:space:]]*}"}"   # strip leading whitespace
        case "$trimmed" in
            '') continue ;;
            '#!['*)
                opens="${line//[^\[]/}"
                closes="${line//[^\]]/}"
                depth=$((${#opens} - ${#closes}))
                if [ "$depth" -le 0 ]; then
                    depth=0
                    last=$lineno
                fi
                continue
                ;;
            # `//`, `//!` and `///` are all legal anywhere before the first item
            # and none of them can carry a lint level. Skip them all.
            '//'*) continue ;;
            *) break ;;   # first real token — the prologue is over
        esac
    done < "$file"
    [ "$last" -gt 0 ] || return 1
    printf '%s\n' "$last"
}

LAST_ATTR_END="$(find_last_inner_attr_end "$LIB_RS")" || die "found no inner attribute (\`#![...]\`) in src/lib.rs's prologue before the first item. Either the \`#![deny(...)]\` policy this proof exists to test is gone, or the prologue has a shape this parser does not understand (e.g. a \`/* */\` header). Fix the parser or restore the policy; do NOT weaken the proof."
INJECT_AT=$((LAST_ATTR_END + 1))

# Belt for the head(1) crash the parser can no longer reach: `head -n 0` is a
# hard error on BSD and a silent empty file on GNU coreutils, so an injection
# point of 1 was a platform-dependent failure with no diagnostic (verify round
# 2, PL-5). Now it is one line and a message.
[ "$INJECT_AT" -ge 2 ] || die "computed an injection point of line ${INJECT_AT}, which is above src/lib.rs's inner attributes. The injection must land strictly AFTER the last \`#![...]\` or the library's policy never applies to it. Refusing to run a proof that cannot prove anything."
info "injection point: src/lib.rs line ${INJECT_AT} (strictly after the last inner attribute, which ends on line ${LAST_ATTR_END})"

# ── Copy the crate ──────────────────────────────────────────────────────────
WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/exif-core-red-proof.XXXXXX")"
cleanup() {
    rm -rf "$WORK_DIR"
}
trap cleanup EXIT

mkdir -p "${WORK_DIR}/src"
cp "${REPO_ROOT}/Cargo.toml" "${WORK_DIR}/Cargo.toml"
if [ -f "${REPO_ROOT}/Cargo.lock" ]; then
    cp "${REPO_ROOT}/Cargo.lock" "${WORK_DIR}/Cargo.lock"
fi
cp -R "${REPO_ROOT}/src/." "${WORK_DIR}/src/"
# Copy a toolchain pin if one ever lands. There is none today; without this the
# copy would silently compile on a different toolchain than the repo, which
# would make the control run answer a question nobody asked.
for pin in rust-toolchain.toml rust-toolchain .cargo; do
    if [ -e "${REPO_ROOT}/${pin}" ]; then
        cp -R "${REPO_ROOT}/${pin}" "${WORK_DIR}/${pin}"
    fi
done

# The one clippy invocation, defined once. `run_clippy <log> [extra args...]`.
run_clippy() {
    local log="$1"
    shift
    # ⚠ `--color never` is LOAD-BEARING, not cosmetic. Assertions 3 and 4 grep
    # this log, and CI's clippy colourises even when redirected to a file:
    # the real bytes are `\e[1m\e[94m--> \e[0msrc/lib.rs:66`, so a reset
    # sequence sits BETWEEN `-->` and the path and `grep -- '--> src/lib\.rs'`
    # matches nothing. That took the proof out on 2026-08-22 (PATCH-001) with
    # NO message — the zero-match grep tripped `set -o pipefail` and killed the
    # script before its own `die` could run. Latent since the job was written;
    # only reachable once everything upstream of assertion 4 was green.
    ( cd "$WORK_DIR" && cargo clippy --color never --all-targets --all-features "$@" ) >"$log" 2>&1
}

# ── Assertion 1: THE CONTROL — the unmutated copy must be clean ─────────────
info "control run: the UNMUTATED copy, exact CI invocation — this MUST pass:"
CONTROL_LOG="${WORK_DIR}/clippy-control.log"
CONTROL_EXIT=0
run_clippy "$CONTROL_LOG" -- -D warnings || CONTROL_EXIT=$?
if [ "$CONTROL_EXIT" -ne 0 ]; then
    cat "$CONTROL_LOG"
    die "the CONTROL run failed (clippy exited ${CONTROL_EXIT}) on the UNMUTATED copy. Something upstream of this proof is broken: the toolchain, the copy step, or the crate itself — run \`cargo clippy --all-targets --all-features -- -D warnings\` at the repo root and fix what it says. Until then NOTHING downstream here means anything: a red from the mutated run would prove nothing about the lint policy, because it could not be attributed to the injection. (DEC-009: this is the assertion the previous three designs did not have.)"
fi
success "control: unmutated copy is clean (clippy exit 0). A red below is now attributable to the injection."

# ── Inject the violations ───────────────────────────────────────────────────
# One violation per policy lint. They are written to COMPILE cleanly, so the
# only thing that can reject them is the lint policy — a syntax error would
# make the mutated run fail for the wrong reason (which the control cannot
# distinguish on its own, and which is how round 2's false green worked).
VIOLATION_FILE="${WORK_DIR}/.violation.rs"
cat > "$VIOLATION_FILE" <<'RUST_EOF'

// ── RED-PROOF INJECTION — scripts/lint-red-proof.sh, DEC-009 ───────────────
// Injected into a COPY of src/lib.rs in a temp dir. Never part of the crate.
// One violation per policy lint — these are exactly the byte-reading mistakes
// the policy exists to stop, on caller-supplied (i.e. attacker-influenced)
// input. All five must be rejected by src/lib.rs's own #![deny(...)].
pub fn red_proof_indexing_and_arithmetic(v: &[u8], n: u8) -> u8 {
    v[0] + n
}

pub fn red_proof_unwrap(v: &[u8]) -> u8 {
    *v.first().unwrap()
}

pub fn red_proof_expect(v: &[u8]) -> u8 {
    *v.first().expect("first byte present")
}

pub fn red_proof_panic(v: &[u8]) -> u8 {
    if v.is_empty() {
        panic!("truncated tag header");
    }
    0
}
// ── END RED-PROOF INJECTION ───────────────────────────────────────────────

RUST_EOF
INJECT_LINES="$(wc -l < "$VIOLATION_FILE" | tr -d '[:space:]')"

INJECTED="${WORK_DIR}/src/lib.rs"
{
    head -n "$((INJECT_AT - 1))" "$LIB_RS"
    cat "$VIOLATION_FILE"
    tail -n "+${INJECT_AT}" "$LIB_RS"
} > "$INJECTED"
rm -f "$VIOLATION_FILE"

INJ_FIRST="$INJECT_AT"
INJ_LAST=$((INJECT_AT + INJECT_LINES - 1))

# ── Structural post-check: the injection really is after the last attribute ──
# Re-parse the MUTATED file. If the injected block had landed above the inner
# attributes, the scan would hit `pub fn` first and find no attribute at all.
POST_ATTR_END="$(find_last_inner_attr_end "$INJECTED")" || die "after injection, src/lib.rs's prologue no longer contains an inner attribute before the first item — the injected block landed ABOVE the \`#![deny(...)]\` policy, where it is not linted. This is the round-2 defect; refusing to continue."
[ "$POST_ATTR_END" = "$LAST_ATTR_END" ] || die "after injection the last inner attribute ends on line ${POST_ATTR_END}, but before injection it ended on line ${LAST_ATTR_END}. The injection perturbed the attribute prologue instead of landing after it."

# ── Assertion 2 + 3 + 4: the mutated copy is rejected, by the RIGHT lints ────
info "mutation run: the same invocation on the injected copy — this MUST fail:"
CLIPPY_LOG="${WORK_DIR}/clippy-mutated.log"
CLIPPY_EXIT=0
run_clippy "$CLIPPY_LOG" -- -D warnings || CLIPPY_EXIT=$?
cat "$CLIPPY_LOG"

if [ "$CLIPPY_EXIT" -eq 0 ]; then
    die "the lint policy did NOT reject the injected violations (clippy exited 0) — and the control run above was clean, so this is the policy's fault and nothing else. src/lib.rs's \`#![deny(...)]\` block is missing, weakened, or not applying. This is exactly the manufactured-confidence failure oracle-must-be-shown-red exists to catch."
fi

# Assertion 3 — every policy lint fired. Matched on clippy's help URL fragment,
# not the bare name: `index.html#unwrap_used` is emitted by the lint machinery
# when the lint FIRES, whereas `clippy::unwrap_used` is the text of the policy
# itself and appears in the log whenever rustc renders that source span. Round 2
# passed all three name checks with zero lints firing for exactly this reason.
assert_lints_fired() {
    local log="$1" label="$2" lint short missing=""
    for lint in $EXPECTED_LINTS; do
        short="${lint#clippy::}"
        grep -qE -- "index\.html#${short}[[:space:]]*\$" "$log" || missing="${missing}${lint} "
    done
    if [ -n "$missing" ]; then
        die "${label}: clippy failed, but the expected lint(s) ${missing}never fired (no \`index.html#<lint>\` help line for them). A non-zero exit is not the proof, and neither is the lint's NAME appearing in the log — rustc prints the policy's own source text. Either the policy no longer denies these lints, or the injection landed somewhere they do not apply."
    fi
}
assert_lints_fired "$CLIPPY_LOG" "mutation run"

# Assertion 4 — the diagnostics are located INSIDE the injected block. This is
# what rules out "the lint names arrived from somewhere else in the file": the
# injected code occupies lines ${INJ_FIRST}..${INJ_LAST} of the mutated copy,
# and each of the four violating functions must be pointed at.
# ⚠ EVERY grep in this pipeline is guarded, and that is not belt-and-braces.
# A zero-match grep exits 1; under `set -o pipefail` + `set -e` that kills this
# script before the `die` below can explain itself, and a proof that dies
# without a message is indistinguishable from a proof that never ran — the
# exact defect class this file exists to prevent (PATCH-001).
#
# Guarding only the FIRST grep does not work, and the first fix did exactly
# that: a zero-match there emits NOTHING, so the second grep zero-matches too
# and aborts the assignment anyway — byte-for-byte the original behaviour.
# Found by PATCH-001's independent verify (SB-1). The orchestrator's own
# suggested test would have PASSED while the defect stayed, because pointing
# INJ_FIRST/INJ_LAST outside the range makes the first grep MATCH and awk do
# the filtering — a different path from the bug. Only a genuine zero-match of
# the LEADING grep exposes it.
IN_RANGE="$( { grep -oE -- '--> src/lib\.rs:[0-9]+' "$CLIPPY_LOG" || true; } \
    | { grep -oE '[0-9]+$' || true; } \
    | awk -v lo="$INJ_FIRST" -v hi="$INJ_LAST" '$1 >= lo && $1 <= hi' \
    | sort -un | wc -l | tr -d '[:space:]')"
# Four violating functions, so four distinct source lines are pointed at
# (`v[0] + n` carries two lints on one line).
if [ "$IN_RANGE" -lt 4 ]; then
    die "clippy failed and named the lints, but only ${IN_RANGE} distinct diagnostic span(s) point inside the injected block (src/lib.rs lines ${INJ_FIRST}-${INJ_LAST}); 4 were expected, one per injected function. The failure is not coming from the injected code."
fi
info "diagnostics located inside the injected block: ${IN_RANGE} distinct lines in src/lib.rs:${INJ_FIRST}-${INJ_LAST}"

# ── Assertion 5: SEVERITY — the policy is `deny`, not CI's `-D warnings` ─────
# Same mutated copy, same command, minus CI's blanket flag. `#![deny(...)]`
# still errors; `#![warn(...)]` does not (measured: exit 0). Without this, the
# policy can be silently downgraded to `warn` and the proof stays green because
# `-D warnings` promotes it back — while a consumer, or anyone running a plain
# `cargo clippy` or `cargo build` locally, sees nothing at all (verify round 2,
# PL-3).
info "severity run: the injected copy WITHOUT CI's -D warnings — the LIBRARY's own deny must still reject it:"
SEVERITY_LOG="${WORK_DIR}/clippy-severity.log"
SEVERITY_EXIT=0
run_clippy "$SEVERITY_LOG" || SEVERITY_EXIT=$?
if [ "$SEVERITY_EXIT" -eq 0 ]; then
    cat "$SEVERITY_LOG"
    die "without CI's \`-D warnings\`, clippy exited 0 on the injected violations. The five lints are present but NOT at \`deny\` level — they are \`warn\`, and the only thing making them bite is a CI flag. src/lib.rs claims they are \`deny\`-level; a consumer running plain \`cargo clippy\` or \`cargo build\` would see nothing. Restore \`#![deny(...)]\`."
fi
assert_lints_fired "$SEVERITY_LOG" "severity run"

# Evidence that it was the LIBRARY's policy that fired, not a header of the
# proof's own making — this is the core DEC-007 retained. Informational.
grep -A1 'the lint level is defined here' "$CLIPPY_LOG" | grep -- '-->' | sort -u | sed 's/^/    /' || true

success "lint policy red-proof: control clean (exit 0) → injection rejected (exit ${CLIPPY_EXIT}) → all five lints fired at the injected code, and still fire without CI's -D warnings (exit ${SEVERITY_EXIT}). src/lib.rs's own #![deny(...)] is what rejected them. PROVED BY: ${CLIPPY_ID}."
