#!/usr/bin/env bash
# scripts/archive-patch.sh — move a shipped patch to patches/done/ (DEC-003).
# Unlike archive-spec there is NO stage bookkeeping — a patch attaches to the
# project, not a stage. Usage: archive-patch.sh PATCH-NNN
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/_lib.sh"

require_initialized

PATCH_ID="${1:-}"
if [ -z "$PATCH_ID" ]; then
    die "Usage: just archive-patch PATCH-NNN"
fi

# find_spec's generic prefix glob locates PATCH-*.md too (and excludes done/,
# so a second archive fails loudly rather than nesting done/done/).
PATCH_FILE=$(find_spec "$PATCH_ID")
if [ -z "$PATCH_FILE" ]; then
    die "Patch not found: ${PATCH_ID}"
fi

CYCLE=$(get_spec_cycle "$PATCH_FILE")
if [ "$CYCLE" != "ship" ]; then
    warn "Patch cycle is '${CYCLE}', not 'ship'. Continue anyway? [y/N]"
    read -r answer
    case "$answer" in
        y|Y) ;;
        *) echo "Aborted."; exit 0 ;;
    esac
fi

PATCH_DIR=$(dirname "$PATCH_FILE")
DONE_DIR="${PATCH_DIR}/done"
mkdir -p "$DONE_DIR"
TARGET="${DONE_DIR}/$(basename "$PATCH_FILE")"
mv "$PATCH_FILE" "$TARGET"
success "Archived: ${PATCH_FILE} → ${TARGET}"
# Recompute cost.totals from the recorded sessions (same as archive-spec).
write_cost_totals "$TARGET"
echo "${DIM}No stage bookkeeping — a patch attaches to the project, not a stage (DEC-003).${RESET}"
