#!/usr/bin/env bash
# scripts/claim.sh — take or release a spec's fan-out lease (`claimed_by:`).
#
# Advisory by design: it marks "who's on it" so the ready-set (`just ready`)
# skips it and two agents don't grab the same spec. The HARD lock for parallel
# work is the worktree/branch an agent creates; this is the declarative marker
# that lock projects back into the spec. Inspired by beads' atomic `--claim`.
#
# Called by the `just claim` / `just unclaim` wrappers.
# Usage:
#   just claim SPEC-003 alice          # take it (fails if held by someone else)
#   just claim SPEC-003 alice --force  # steal it
#   just unclaim SPEC-003              # release it

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/_lib.sh"
require_initialized

MODE="${1:-}"; SPEC_ID="${2:-}"; WHO="${3:-}"; FORCE="${4:-}"
case "$MODE" in claim|unclaim) ;; *) usage_error "internal: mode must be claim|unclaim" ;; esac
[ -n "$SPEC_ID" ] || die "usage: just ${MODE} ${SPEC_ID:-SPEC-NNN}${MODE:+ ${MODE/unclaim/}}"

case "$SPEC_ID" in
    SPEC-*) ;;
    *) die "Expected a SPEC-NNN id, got '${SPEC_ID}'." ;;
esac

# Resolve the spec file (any project), excluding prompts/ and timeline artifacts.
sf=$(find "${REPO_ROOT}/projects" -type f -name "${SPEC_ID}-*.md" \
        ! -name '*-timeline.md' -not -path '*/prompts/*' 2>/dev/null | sort | head -n1)
[ -n "$sf" ] || die "No spec ${SPEC_ID} found under projects/."

current=$(get_spec_claimed_by "$sf")

if [ "$MODE" = "claim" ]; then
    [ -n "$WHO" ] || die "usage: just claim ${SPEC_ID} <who>"
    if [ -n "$current" ] && [ "$current" != "$WHO" ] && [ "$FORCE" != "--force" ]; then
        die "${SPEC_ID} is already claimed by '${current}'. Use --force to steal it."
    fi
    set_fm_scalar "$sf" claimed_by "$WHO"
    echo "✓ ${SPEC_ID} claimed by ${WHO}"
else
    if [ -z "$current" ]; then
        echo "· ${SPEC_ID} was not claimed — nothing to release."
        exit 0
    fi
    set_fm_scalar "$sf" claimed_by null
    echo "✓ ${SPEC_ID} released (was ${current})"
fi
