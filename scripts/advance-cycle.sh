#!/usr/bin/env bash
# scripts/advance-cycle.sh — update a spec's task.cycle field.
# Usage: advance-cycle.sh SPEC-NNN <new-cycle>

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/_lib.sh"

require_initialized

SPEC_ID="${1:-}"
NEW_CYCLE="${2:-}"
shift 2 2>/dev/null || true

VERDICT=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        --verdict) VERDICT="${2:-}"; shift 2 ;;
        --verdict=*) VERDICT="${1#--verdict=}"; shift ;;
        '') shift ;;
        *) usage_error "just advance-cycle SPEC-NNN <cycle> [--verdict approved|punch-list|rejected]" ;;
    esac
done

if [ -z "$SPEC_ID" ] || [ -z "$NEW_CYCLE" ]; then
    die "Usage: just advance-cycle SPEC-NNN <frame|design|build|verify|ship>"
fi

VALID_VERDICT=" approved punch-list rejected "
if [ -n "$VERDICT" ]; then
    case "$VALID_VERDICT" in
        *" $VERDICT "*) : ;;
        *) usage_error "--verdict must be one of:${VALID_VERDICT}(got '${VERDICT}')" ;;
    esac
fi

case "$NEW_CYCLE" in
    frame|design|build|verify|ship|patch|spike|land) ;;
    *) die "Invalid cycle: ${NEW_CYCLE}. Must be one of: frame, design, build, verify, ship (specs), patch (the patch lane), or spike|land (the spike lane)." ;;
esac

SPEC_FILE=$(find_spec "$SPEC_ID")
if [ -z "$SPEC_FILE" ]; then
    die "Spec not found: ${SPEC_ID}"
fi

OLD_CYCLE=$(awk '/^---$/{f=!f; next} f && /^[[:space:]]+cycle:/{print $2; exit}' "$SPEC_FILE" 2>/dev/null || echo "unknown")

# Guard: refuse to "advance" a file that has no task.cycle front-matter. Before
# find_spec excluded prompts/, this command could resolve to a cycle-prompt file
# (no front-matter), report success with a blank old-cycle, and leave the real
# spec stuck (verified: zany-animal-slots #7). Fail loudly instead.
if [ -z "$OLD_CYCLE" ] || [ "$OLD_CYCLE" = "unknown" ]; then
    die "No task.cycle front-matter in ${SPEC_FILE} — refusing to advance (resolved a non-spec file?)."
fi

update_frontmatter_scalar "$SPEC_FILE" "task.cycle" "$NEW_CYCLE"

# Stamp the verify verdict whenever a spec LEAVES verify — in either direction.
#
# Stamping only on the way to ship would record approvals and silently drop
# every rejection, which is precisely the number worth having ("verify never
# rejects anything" is a documented failure signature that nothing could
# compute). So the destination carries the default:
#   verify → ship            approved     (you shipped it; verify passed)
#   verify → build/design    punch-list    (it went back; verify found something)
# The one thing the destination cannot tell you is punch-list vs rejected —
# both return to build — so `--verdict rejected` is the manual distinction.
# What was recorded is always printed, so an inferred value can be corrected.
if [ "$OLD_CYCLE" = "verify" ]; then
    recorded="$VERDICT" inferred=0
    if [ -z "$recorded" ]; then
        inferred=1
        case "$NEW_CYCLE" in
            ship)                 recorded="approved" ;;
            build|design|frame)   recorded="punch-list" ;;
            *)                    recorded="" ;;
        esac
    fi
    if [ -n "$recorded" ]; then
        # upsert, not update: specs written before this field existed have no
        # `verify_verdict:` line, and a plain update would no-op while we
        # printed "recorded" below.
        upsert_frontmatter_scalar "$SPEC_FILE" "task.verify_verdict" "$recorded"
        VERDICT_NOTE="$recorded"
        [ "$inferred" = 1 ] && VERDICT_NOTE="${recorded} (inferred from → ${NEW_CYCLE})"
    fi
fi

success "Advanced ${SPEC_ID}: ${OLD_CYCLE} → ${NEW_CYCLE}"
echo "  File: ${SPEC_FILE}"
if [ -n "${VERDICT_NOTE:-}" ]; then
    echo "  Verify verdict recorded: ${VERDICT_NOTE}"
    [ "${inferred:-0}" = 1 ] && \
        echo "  ${DIM}Override with: just advance-cycle ${SPEC_ID} ${NEW_CYCLE} --verdict rejected${RESET}"
fi

# Helpful next-step hints based on the new cycle.
echo ""
case "$NEW_CYCLE" in
    design)
        echo "Next: use Prompt 2b (Spec Design) in FIRST_SESSION_PROMPTS.md"
        ;;
    build)
        echo "Next: use Prompt 3 (Build) in FIRST_SESSION_PROMPTS.md"
        echo "  ${DIM}Claude-only variant reminder: start a NEW session for build.${RESET}"
        ;;
    verify)
        echo "Next: use Prompt 4 (Verify) in FIRST_SESSION_PROMPTS.md"
        echo "  ${DIM}Claude-only variant reminder: start a NEW session for verify.${RESET}"
        ;;
    ship)
        echo "Next: use Prompt 5 (Ship), then run 'just archive-spec ${SPEC_ID}'"
        ;;
    land)
        echo "Next: fill the spike's ## Land section — answer, DECs for load-bearing"
        echo "  choices, the code's fate — then 'just archive-spike ${SPEC_ID}'"
        echo "  ${DIM}spike.outcome must be set: answered | inconclusive | graduated | discarded${RESET}"
        ;;
esac
