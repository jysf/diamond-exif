#!/usr/bin/env bash
# scripts/new-handoff.sh — scaffold a delegation handoff (claude-plus-agents).
#
# ONE handoff per delegated CYCLE. With build and verify on different agents you
# get two per spec; `handoff.cycle` distinguishes them and picks `to_agent` from
# `.repo-context.yaml` → spec.agent.tier_map.<cycle> (DEC-005).
#
# The scaffolded file carries a `handback:` block the executing agent fills in —
# including a real tokens_total — which `just handback-sync` then transcribes
# into the spec's cost.sessions. That is how cost gets recorded without the
# orchestrator counting anything by hand.
#
# Usage: new-handoff.sh SPEC-NNN <build|verify>
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/_lib.sh
source "${SCRIPT_DIR}/_lib.sh"

require_initialized

SPEC_ID="${1:-}"
CYCLE="${2:-}"

if [ -z "$SPEC_ID" ] || [ -z "$CYCLE" ]; then
    die "Usage: just new-handoff SPEC-NNN <build|verify>"
fi

case "$CYCLE" in
    build)  TO_ROLE="implementer" ;;
    verify) TO_ROLE="verifier" ;;
    *) die "Handoff cycle must be 'build' or 'verify' (design/frame/ship stay with the orchestrator), got: ${CYCLE}" ;;
esac

TEMPLATE="${REPO_ROOT}/projects/_templates/handoff.md"
if [ ! -f "$TEMPLATE" ]; then
    die "No handoff template at ${TEMPLATE}.

Handoffs are a claude-plus-agents concept — in claude-only the context the
implementer needs lives in the spec's ## Implementation Context section."
fi

SPEC_FILE=$(find_spec "$SPEC_ID")
if [ -z "$SPEC_FILE" ]; then
    die "Spec not found: ${SPEC_ID}"
fi

PROJECT_DIR=$(dirname "$(dirname "$SPEC_FILE")")
PROJECT_ID=$(basename "$PROJECT_DIR" | awk -F- '{print $1"-"$2}')
STAGE_ID=$(awk '/^---$/{f=!f; next} f && /^[[:space:]]+stage:/{print $2; exit}' "$SPEC_FILE")
[ -n "$STAGE_ID" ] || STAGE_ID="STAGE-XXX"

HANDOFF_ID=$(next_id HANDOFF)
SLUG=$(basename "$SPEC_FILE" .md | sed -E "s/^${SPEC_ID}-//")
HANDOFF_DIR="${PROJECT_DIR}/handoffs"
mkdir -p "$HANDOFF_DIR"
HANDOFF_FILE="${HANDOFF_DIR}/${HANDOFF_ID}-${CYCLE}-${SLUG}.md"

if [ -f "$HANDOFF_FILE" ]; then
    die "Handoff already exists: ${HANDOFF_FILE}"
fi

cp "$TEMPLATE" "$HANDOFF_FILE"

sed_inplace() {
    if [ "$(uname)" = "Darwin" ]; then sed -i '' "$@"; else sed -i "$@"; fi
}

REPO_ID_ESC=$(sed_escape_replacement "$(get_repo_id)")

sed_inplace "s|HANDOFF-XXX|${HANDOFF_ID}|g" "$HANDOFF_FILE"
sed_inplace "s|SPEC-XXX|${SPEC_ID}|g" "$HANDOFF_FILE"
sed_inplace "s|PROJ-XXX|${PROJECT_ID}|g" "$HANDOFF_FILE"
sed_inplace "s|STAGE-XXX|${STAGE_ID}|g" "$HANDOFF_FILE"
sed_inplace "s|__CYCLE__|${CYCLE}|g" "$HANDOFF_FILE"
sed_inplace "s|__TO_ROLE__|${TO_ROLE}|g" "$HANDOFF_FILE"
sed_inplace "s|__FROM_AGENT__|$(get_tier_model design)|g" "$HANDOFF_FILE"
sed_inplace "s|__TO_AGENT__|$(get_tier_model "$CYCLE")|g" "$HANDOFF_FILE"
sed_inplace "s|__TODAY__|$(today)|g" "$HANDOFF_FILE"
sed_inplace "s|__REPO_ID__|${REPO_ID_ESC}|g" "$HANDOFF_FILE"

success "Created ${HANDOFF_FILE}"
echo ""
echo "Delegating ${SPEC_ID} ${CYCLE} → $(get_tier_model "$CYCLE") (as ${TO_ROLE})"
echo ""
echo "Next:"
echo "  1. Fill in the context sections (decisions, constraints, out of scope)."
echo "  2. Hand the file to the agent. It MUST fill the handback: block —"
echo "     including a real tokens_total — before reporting done."
echo "  3. When it returns:  just handback-sync ${SPEC_ID}"
echo "     ${DIM}(transcribes the reported cost into the spec — no hand-counting)${RESET}"
