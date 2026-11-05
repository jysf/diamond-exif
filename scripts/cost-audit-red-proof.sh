#!/usr/bin/env bash
# PATCH-002's red-proof for the stage orchestration-cost gate.
#
# `constraints.yaml`: a job that exists and has never passed is a deleted job.
# The sibling failure is a gate that exists and has never FAILED — indistinguish-
# able from one that never ran. STAGE-001 shipped with `sessions: []` and nothing
# noticed for fifteen days, so this gate is adopted precisely because its absence
# was invisible. It ships proven red.
#
# Everything happens in a temp copy; the working tree is never written to.
#
# diamond-exif adaptation: irradiance's copy mutated its own real STAGE-001/002
# files. A fresh instance has no such history, so this version SYNTHESISES its
# two subjects (STAGE-900: shipped, honestly filled; STAGE-901: shipped, empty,
# grandfathered via the env override) inside the throwaway copy. The gate and
# every injection below are unchanged.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

fail() { printf '✗ cost-audit red-proof: %s\n' "$1" >&2; exit 1; }

# FU-7 (PATCH-003 verify): `cp -R "$ROOT"` copied `target/` — 105 s locally
# against 3.8 s from a clean clone, and 0.87 s in CI where target/ does not
# exist. The gate reads scripts/, projects/ and nothing else, so copy that.
mkdir -p "$TMP/repo"
for d in scripts projects; do cp -R "$ROOT/$d" "$TMP/repo/$d"; done
# require_initialized() checks for AGENTS.md (_lib.sh:41), and _lib.sh reads the
# active project from .repo-context.yaml. Without these the control fails for a
# reason that has nothing to do with the gate — which is how the first attempt at
# this optimisation looked exactly like a regression.
for f in .repo-context.yaml VERSION justfile app.just AGENTS.md decisions guidance; do
    [ -e "$ROOT/$f" ] && cp -R "$ROOT/$f" "$TMP/repo/$f"
done
cd "$TMP/repo"

# Synthesise the subjects from the instance's OWN stage template, so the shape
# under test is the real shipped one (template comment block included).
TEMPLATE="projects/_templates/stage.md"
[ -f "$TEMPLATE" ] || fail "no $TEMPLATE in the copied tree to synthesise stages from"
STAGES_DIR="$(find projects/PROJ-* -maxdepth 1 -type d -name stages | sort | head -1)"
[ -n "$STAGES_DIR" ] || fail "no projects/PROJ-*/stages dir in the copied tree to inject into"
make_stage() {  # $1 = id, $2 = filled|empty
    python3 - "$TEMPLATE" "$STAGES_DIR/$1-redproof-subject.md" "$1" "$2" <<'INNER0'
import re, sys
src, dst, sid, mode = sys.argv[1:5]
s = open(src).read()
s = re.sub(r'^(\s*id:\s*)STAGE-\w+', lambda m: m.group(1) + sid, s, count=1, flags=re.M)
s, n = re.subn(r'^(\s*status:\s*)\S+', r'\1shipped', s, count=1, flags=re.M)
if n != 1:
    sys.exit("stage status line not found in template")
if mode == "filled":
    m = re.search(r'^orchestration_cost:\n.*?(?=^---$)', s, re.S | re.M)
    if not m:
        sys.exit("orchestration_cost block not found in template")
    s = s[:m.start()] + ("orchestration_cost:\n  sessions:\n    - tokens_total: 1000000\n"
                         "      estimated_usd: 1.00\n      recorded_at: 2026-01-01\n"
                         "      notes: \"red-proof fixture\"\n") + s[m.end():]
open(dst, 'w').write(s)
INNER0
}
SUBJECT="$STAGES_DIR/STAGE-900-redproof-subject.md"
GRANDFATHERED="$STAGES_DIR/STAGE-901-redproof-subject.md"
make_stage STAGE-900 filled
make_stage STAGE-901 empty
# PRISTINE is the reference every restore/diff below uses (irradiance's copy
# used "$PRISTINE", i.e. the real file).
PRISTINE="$TMP/pristine-STAGE-900.md"
cp "$SUBJECT" "$PRISTINE"

# ⚠ REPRODUCE THE REAL SHIPPED SHAPE, template comment included. An injection
# that writes a bare `sessions: []` also deletes the commented example — and
# then even a naive `grep -q tokens_total` implementation passes this proof,
# because no text remains to false-match. MEASURED 2026-09-06: the naive version
# passed the first draft of this script. AGENTS.md §16's own warning, verbatim:
# "The obvious test exercises the wrong path."
empty_block() {
    python3 - "$1" <<'INNER'
import sys, re
p = sys.argv[1]
s = open(p).read()
m = re.search(r'^orchestration_cost:\n.*?(?=^---$)', s, re.S | re.M)
if not m:
    sys.exit("orchestration_cost block not found in " + p)
s = s[:m.start()] + (
    "orchestration_cost:\n"
    "  sessions: []                      # - tokens_total: N\n"
    "                                    #   estimated_usd: N\n"
    "                                    #   recorded_at: YYYY-MM-DD\n"
    '                                    #   notes: "framing + spec breakdown"\n'
) + s[m.end():]
open(p, 'w').write(s)
INNER
}

# ── 1. CONTROL: the honest tree must PASS, or a red below proves nothing.
# STAGE-901 is empty on purpose, so it is grandfathered for the whole run;
# step 4 proves that list is the ONLY thing exempting it.
export STAGE_ORCH_COST_GRANDFATHERED="STAGE-901"
./scripts/cost-audit.sh >/dev/null 2>&1 \
    || fail "control failed — cost-audit is already red on the honest tree, so the injection below would prove nothing"

# ── 2. INJECT into a shipped, non-grandfathered stage.
empty_block "$SUBJECT"

# ⚠ The clause that has caught four false red-proofs in three specs: assert the
# mutation CHANGED THE FILE before concluding anything about what was caught.
if diff -q "$SUBJECT" "$PRISTINE" >/dev/null 2>&1; then
    fail "the injection did not change the file — this red-proof has caught NOTHING"
fi
# And assert it reproduced the shape it claims to: the comment must survive, or
# this is the weaker proof the header warns about.
grep -q '# - tokens_total: N' "$SUBJECT" \
    || fail "the injection removed the template comment, so it exercises the wrong path — see this script's header"

# ── 3. The gate must now FAIL, through its OWN die, with a reason.
set +e
out="$(./scripts/cost-audit.sh 2>&1)"; rc=$?
set -e
[ "$rc" -ne 0 ] || fail "gate did NOT go red on a shipped stage with sessions: [] — the gate is decorative"
printf '%s\n' "$out" | grep -q 'missing cost on: orchestration' \
    || fail "gate went red but never named the FIELD; a proof that dies without a message cannot be told from one that never ran"
# FU-2 (PATCH-003): this script's own success line claims the stage is rejected
# BY NAME, and until now it only ever checked the reason — replacing "$name"
# with a literal in cost-audit.sh survived the proof while making the output
# useless. Assert the claim the summary makes.
subject_id="$(basename "$SUBJECT" .md)"
printf '%s\n' "$out" | grep -q -- "$subject_id" \
    || fail "gate went red but never named the STAGE (expected '$subject_id'), while this proof's own summary claims it is rejected by name"
printf '%s\n' "$out" | grep -q 'orchestration_cost.sessions' \
    || fail "the failure message does not say what to do about it"

# ── 3b. SB-2 (PATCH-003): PROSE MUST NOT SATISFY THE GATE.
#      The block stays empty, and the BODY gets a horizontal rule followed by a
#      line that looks exactly like a real entry. Before PATCH-003 this passed:
#      the awk toggled its front-matter flag on every bare `---`, so the third
#      one flipped the body back into "front matter", and `orchestration_cost:`
#      is the last front-matter key in the template and all five stage files —
#      the repo's default shape. Documentation about the field satisfying a
#      check on the field is the exact class this gate exists to prevent.
cp "$PRISTINE" "$SUBJECT"
empty_block "$SUBJECT"
printf '\n\n---\n\nProse about the cost of this stage:\n\n    - tokens_total: 84200000\n' >> "$SUBJECT"
grep -q '^    - tokens_total: 84200000' "$SUBJECT" \
    || fail "the SB-2 injection did not land in the body — this case proves nothing"
set +e
out2="$(./scripts/cost-audit.sh 2>&1)"; rc2=$?
set -e
[ "$rc2" -ne 0 ] \
    || fail "SB-2 REGRESSION: prose in the body satisfied the gate — the front-matter scan is leaking into the body again"
printf '%s\n' "$out2" | grep -q 'missing cost on: orchestration' \
    || fail "SB-2 case went red for the wrong reason"

# ── 3c. PATCH-003 round 2: THE FENCE IS NOT `^---$`.
#      Round 1 claimed the body was "unreachable" in four places, off one
#      modelled shape. The reviewer found it was ONE SPACE away. These cases
#      exist so that claim is falsifiable rather than asserted — each is a
#      shape that bypassed the gate at 15c7fe0.
#
#      N1 trailing space on the closing fence   N2 no closing fence at all
#      N3 CRLF line endings                     N5 no front matter
fence_case() {  # $1 = label, $2 = closing fence text ('' for none)
    cp "$PRISTINE" "$SUBJECT"
    empty_block "$SUBJECT"
    if [ -n "$2" ]; then
        # replace the SECOND `---` (the closing fence) with the variant
        python3 - "$SUBJECT" "$2" <<'INNER2'
import sys
p, fence = sys.argv[1], sys.argv[2]
lines = open(p).read().split('\n')
seen = 0
for i, l in enumerate(lines):
    if l.strip() == '---' and l == '---':
        seen += 1
        if seen == 2:
            lines[i] = fence
            break
open(p, 'w').write('\n'.join(lines))
INNER2
    fi
    printf '\n\n    - tokens_total: 84200000\n' >> "$SUBJECT"
    set +e; ./scripts/cost-audit.sh >/dev/null 2>&1; local rc=$?; set -e
    [ "$rc" -ne 0 ] || fail "$1: prose in the body satisfied the gate — the front-matter scan is leaking again"
}

fence_case "N1 (closing fence with a trailing space)" '--- '
fence_case "N2 (no closing fence)" ''

# N3 — CRLF. A real, filled entry must still be FOUND, and the stage must not
# be silently skipped by get_stage_status. Before round 2 both failed, and the
# gate then reported success about a stage it never opened.
cp "$PRISTINE" "$SUBJECT"
python3 -c "
import sys
p=sys.argv[1]; d=open(p,'rb').read().replace(b'\r\n',b'\n').replace(b'\n',b'\r\n')
open(p,'wb').write(d)" "$SUBJECT"
set +e; ./scripts/cost-audit.sh >/dev/null 2>&1; rc3=$?; set -e
[ "$rc3" -eq 0 ] \
    || fail "N3 (CRLF): a stage with a REAL orchestration entry was reported missing under CRLF"
cp "$PRISTINE" "$SUBJECT"

# ── 4. NEGATIVE CONTROL: restore the subject, empty the GRANDFATHERED stage,
#      and confirm the exemption still holds — otherwise STAGE-001 fails today.
cp "$PRISTINE" "$SUBJECT"
# STAGE-901 is already empty. It must trip the gate by default, and be exempt
# only through the explicit override.
# A non-empty blank defeats the `:-` fallback (see _lib.sh), i.e. "no list".
if STAGE_ORCH_COST_GRANDFATHERED=" " ./scripts/cost-audit.sh >/dev/null 2>&1; then
    fail "an EMPTY shipped stage (STAGE-901) did not trip the gate with no grandfather list"
fi
STAGE_ORCH_COST_GRANDFATHERED="STAGE-901" ./scripts/cost-audit.sh >/dev/null 2>&1 \
    || fail "the grandfathered STAGE-901 tripped the gate — the exemption does not work"

printf '✓ cost-audit red-proof: control clean → a shipped stage whose orchestration_cost is the UNFILLED TEMPLATE (comment and all) is REJECTED by name, with a reason → a grandfathered (override-listed) stage is still exempt.\n'
