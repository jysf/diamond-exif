#!/usr/bin/env bash
# SPEC-016's red-proof for the front-matter PARSE gate in `just validate`.
#
# The named test:  validate_rejects_front_matter_that_no_parser_can_read
#
# `constraints.yaml` (oracle-must-be-shown-red): a gate that has never gone red
# cannot be told from one that never ran. `just validate` reported "valid
# required front-matter" by GREPPING for required keys — it never parsed — so
# front matter that no YAML parser could read was reported valid; one such
# artifact shipped, was archived, and survived three later specs (SPEC-016
# Context). This proves the parse gate SPEC-016 added is not decorative:
#
#   * CONTROL — the honest tree PASSES (also guards that HANDOFF-039 stays fixed).
#   * The fixture (tests/oracle-fixtures/SPEC-900-unparseable-front-matter.md) is
#     grep-satisfiable but parse-broken, in the exact shape `handback-sync`
#     produced. Dropped into the tree it is REJECTED, named, with the parser's
#     OWN message shown — AND the required-KEYS half still passes it, isolating
#     the new coverage to the parse gate (the OLD grep-only gate's verdict, from
#     the SAME bytes of the SAME current script).
#   * ISOLATION — remove the fixture and the gate is green again, so the red is
#     attributable to the fixture by construction.
#
# ⚠ AC2's load-bearing clause: it is not enough that the gate exits non-zero. A
# wrong parser API (e.g. `safe_load(permitted_classes:)` on Ruby 2.6) fails
# UNIFORMLY and looks exactly like a finding while the check never ran. So this
# asserts the gate prints the PARSER'S MESSAGE for the fixture — not just a code.
#
# Everything happens in a temp copy; the working tree is never written to.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

fail() { printf '✗ validate red-proof: %s\n' "$1" >&2; exit 1; }

FIXTURE="$ROOT/tests/oracle-fixtures/SPEC-900-unparseable-front-matter.md"
[ -f "$FIXTURE" ] || fail "fixture not found at $FIXTURE"

# The gate reads scripts/, projects/ and decisions/ (+ the init/context files
# _lib.sh needs). Copy exactly that — not target/ (FU-7: cp -R of the whole repo
# copied a 105 s target/ in cost-audit-red-proof).
mkdir -p "$TMP/repo"
for d in scripts projects decisions; do cp -R "$ROOT/$d" "$TMP/repo/$d"; done
# require_initialized() wants AGENTS.md; _lib.sh reads .repo-context.yaml. Without
# these the control fails for a reason unrelated to the gate.
for f in .repo-context.yaml VERSION justfile app.just AGENTS.md guidance; do
    [ -e "$ROOT/$f" ] && cp -R "$ROOT/$f" "$TMP/repo/$f"
done
cd "$TMP/repo"

# ── 1. CONTROL: the honest tree must PASS, or any red below proves nothing.
./scripts/validate.sh >/dev/null 2>&1 \
    || fail "control failed — validate is already red on the honest tree (is HANDOFF-039 still unparseable?), so the injection below would prove nothing"

# ── 2. INJECT the parse-broken fixture as an active spec (so the OLD required-
#       keys scope — specs + patches + spikes — sees it too, not just the new
#       parse scope). Same bytes as the committed fixture.
SPECS_DIR="$(cd "$TMP/repo" && find projects/*/specs -maxdepth 0 -type d | head -1)"
[ -n "$SPECS_DIR" ] || fail "no specs/ dir in the copied tree to inject into"
INJECTED="$SPECS_DIR/SPEC-900-unparseable-front-matter.md"
cp "$FIXTURE" "$INJECTED"

# ⚠ The clause that has caught false red-proofs across this repo: assert the
# injection actually landed before concluding anything about what was caught.
[ -f "$INJECTED" ] || fail "the fixture was not injected — this red-proof has caught NOTHING"

# ── 3. The gate must now FAIL, through its own die, NAMING the file and showing
#       the PARSER'S OWN MESSAGE (AC2's third clause — not just an exit code).
set +e
out="$(./scripts/validate.sh 2>&1)"; rc=$?
set -e
[ "$rc" -ne 0 ] \
    || fail "gate did NOT go red on front matter no parser can read — the parse gate is decorative"
printf '%s\n' "$out" | grep -q 'SPEC-900-unparseable-front-matter' \
    || fail "gate went red but never NAMED the offending file"
printf '%s\n' "$out" | grep -q 'front matter did not parse' \
    || fail "gate went red but not on the PARSE check — red for the wrong reason"
printf '%s\n' "$out" | grep -q 'did not find expected key' \
    || fail "gate went red without the PARSER'S MESSAGE; an exit code alone cannot tell a real parse failure from a check that never ran (SPEC-016 AC2, DEC-017)"

# ── 3b. THE OLD GREP-ONLY GATE WOULD HAVE PASSED THIS, from the same bytes of
#       the same current script: the required-KEYS half raises no offender for
#       the fixture. (`invalid/missing:` is the key-check's offender line.) This
#       is the coverage the parse gate adds — isolated, not asserted.
printf '%s\n' "$out" | grep 'SPEC-900-unparseable-front-matter' | grep -q 'invalid/missing' \
    && fail "the required-keys check flagged the fixture, so this does not isolate the PARSE gate — pick a fixture that is grep-satisfiable"

# ── 4. ISOLATION: remove the fixture and the gate is green again — the red is
#       attributable to the fixture, nothing else.
rm -f "$INJECTED"
./scripts/validate.sh >/dev/null 2>&1 \
    || fail "gate still red after removing the fixture — the red was not attributable to it"

printf '✓ validate red-proof (validate_rejects_front_matter_that_no_parser_can_read): control clean → a grep-satisfiable, parse-broken artifact is REJECTED by name with the parser'\''s own message, while the required-keys half still passes it → green again once removed.\n'
