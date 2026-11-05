#!/usr/bin/env bash
# SPEC-023's red-proof for `just gate-audit` (scripts/gate-audit.sh).
#
# The named test:  gate_audit_catches_the_mute_death_shape_injected_into_a_gate
#
# `constraints.yaml` (oracle-must-be-shown-red, widened to cover gates by
# SPEC-003/FU-14): a gate that has never gone red cannot be told from one that
# never ran — and gate-audit is itself a gate (a check OVER the gates). So this
# proves gate-audit is not decorative: it bites on EACH of the three mute-death
# shapes, by NAME (the shape slug + the offending file), with a clean negative
# control and isolation, and it does NOT fire on prose that merely DESCRIBES the
# shape (AC1's comment-exclusion clause — the exact trap in validate.sh's own
# header, and the defect this class keeps producing: tooling that matches its own
# documentation, `attribute-text-inside-doc-comments`).
#
#   * CONTROL — the honest copied tree PASSES (gate-audit exits 0).
#   * SHAPE 2 (unguarded grep) injected into a COPY of a real gate → caught, named,
#     tagged [unguarded-grep].
#   * SHAPE 1 (swallowed verdict-oracle exit) injected into a COPY of a real gate
#     → caught, named, tagged [swallowed-oracle-exit].
#   * SHAPE 3 (success with no count asserted) as a new gate in the copied tree →
#     caught, tagged [unasserted-count].
#   * COMMENT with the shape's text (no `|| true`) → NOT flagged (comment-exclusion).
#   * ISOLATION — pristine scripts → green again, so every red above is
#     attributable to the injection by construction.
#
# ⚠ AC2's load-bearing clause: it is not enough that gate-audit exits non-zero. A
# scanner that fired on everything (or crashed) would exit non-zero and look
# exactly like a finding while checking nothing. So this asserts gate-audit names
# the INJECTED gate with the RIGHT shape slug, and that the honest tree and a
# shape-shaped COMMENT both stay green.
#
# Everything happens in a temp copy; the working tree is never written to.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

fail() { printf '✗ gate-audit red-proof: %s\n' "$1" >&2; exit 1; }

# gate-audit reads scripts/ only; _lib.sh's require_initialized wants AGENTS.md
# and _lib reads .repo-context.yaml. Copy exactly that (not projects/, decisions/
# or target/) — a gate-audit run needs nothing else.
mkdir -p "$TMP/repo"
cp -R "$ROOT/scripts" "$TMP/repo/scripts"
for f in .repo-context.yaml VERSION justfile app.just AGENTS.md guidance; do
    [ -e "$ROOT/$f" ] && cp -R "$ROOT/$f" "$TMP/repo/$f"
done
cd "$TMP/repo"

# Restore scripts/ to the pristine committed copy between injections, so each red
# is attributable to exactly one injection (DEC-017: rebuild a mutated copy).
reset_scripts() {
    rm -rf "$TMP/repo/scripts"
    cp -R "$ROOT/scripts" "$TMP/repo/scripts"
}

# Run gate-audit on the copied tree; sets $rc and $out. (rc captured per §16
# rule 3 — never read a gate's verdict from an unguarded pipeline.)
run_audit() {
    set +e
    out="$(./scripts/gate-audit.sh 2>&1)"; rc=$?
    set -e
}

# ── 1. CONTROL: the honest tree must PASS, or any red below proves nothing. ──
run_audit
[ "$rc" -eq 0 ] \
    || fail "control failed — gate-audit is already red on the honest tree (out: ${out}). An injected red below would prove nothing."

# A real gate in the subject set to inject into. ready.sh is verdict-bearing and
# clean, so it is a true negative to mutate.
TARGET="scripts/ready.sh"
[ -f "$TARGET" ] || fail "expected gate ${TARGET} not in the copied tree"

# ── 2. SHAPE 2 — an unguarded grep in a command substitution. ────────────────
reset_scripts
printf '\nmute_death_shape_2=$(echo hi | grep hi)   # injected by gate-audit-red-proof\n' >> "$TARGET"
grep -q 'mute_death_shape_2' "$TARGET" || fail "shape-2 injection did not land — this red-proof has caught NOTHING"
run_audit
[ "$rc" -ne 0 ] || fail "gate-audit did NOT go red on an unguarded grep injected into ${TARGET} — the unguarded-grep detector is decorative"
printf '%s\n' "$out" | grep -q 'ready.sh:.*\[unguarded-grep\]' \
    || fail "gate-audit went red but did not name ready.sh with the [unguarded-grep] shape — red for the wrong reason (out: ${out})"

# ── 3. SHAPE 1 — a substitution that swallows a verdict oracle's exit. ────────
reset_scripts
printf '\nwhile read -r l; do :; done < <(front_matter_parse_report "$x")   # injected\n' >> "$TARGET"
grep -q 'front_matter_parse_report' "$TARGET" || fail "shape-1 injection did not land"
run_audit
[ "$rc" -ne 0 ] || fail "gate-audit did NOT go red on a swallowed verdict-oracle exit injected into ${TARGET}"
printf '%s\n' "$out" | grep -q 'ready.sh:.*\[swallowed-oracle-exit\]' \
    || fail "gate-audit went red but did not name ready.sh with the [swallowed-oracle-exit] shape (out: ${out})"

# ── 4. SHAPE 3 — a gate that declares success without asserting any count. ────
# A FILE-level shape, so a new minimal gate in the copied tree is the fixture
# (it also exercises the fail-closed default: a brand-new script IS audited).
reset_scripts
MUTANT="scripts/mutant-success-no-count.sh"
cat > "$MUTANT" <<'MUT'
#!/usr/bin/env bash
set -euo pipefail
echo "pretending to check things"
success "all good"
MUT
grep -q 'success' "$MUTANT" || fail "shape-3 injection did not land"
run_audit
[ "$rc" -ne 0 ] || fail "gate-audit did NOT go red on a gate that declares success with no count assertion"
printf '%s\n' "$out" | grep -q 'mutant-success-no-count.sh:.*\[unasserted-count\]' \
    || fail "gate-audit went red but did not name the mutant with the [unasserted-count] shape (out: ${out})"

# ── 5. COMMENT-EXCLUSION — prose describing the shape must NOT be flagged. ────
# This is the trap gate-audit itself must survive (AC1; validate.sh:~296 carries
# exactly such prose). A comment line holding `grep` with no `|| true`, and one
# naming front_matter_parse_report inside `< <(…)`, must produce ZERO findings.
reset_scripts
{
    printf '\n# describing the shape: x=$(echo hi | grep hi) with no guard, and\n'
    printf '# done < <(front_matter_parse_report "$x") — this is PROSE, not code.\n'
} >> "$TARGET"
run_audit
[ "$rc" -eq 0 ] \
    || fail "gate-audit fired on a COMMENT describing the shape — it matches its own documentation (AC1 comment-exclusion; out: ${out})"

# ── 6. ISOLATION — pristine scripts → green again. ───────────────────────────
reset_scripts
run_audit
[ "$rc" -eq 0 ] \
    || fail "gate-audit still red after restoring pristine scripts — the reds above were not attributable to the injections (out: ${out})"

printf '✓ gate-audit red-proof (gate_audit_catches_the_mute_death_shape_injected_into_a_gate): control clean → an unguarded grep, a swallowed verdict-oracle exit, and a success-without-a-count are each caught BY NAME and shape, a shape-shaped COMMENT is NOT flagged, and the tree is green once the injections are removed.\n'
