#!/usr/bin/env bash
# scripts/new-patch.sh — scaffold a new patch (the lightweight fix lane, DEC-003).
# A patch is a bounded fix to already-shipped behavior; it runs the collapsed
# patch -> verify -> ship cycle and attaches to the PROJECT, not a stage.
# Usage: new-patch.sh "short title" [PROJ-NNN]
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/_lib.sh"

require_initialized

TITLE="${1:-}"
PROJECT_ID="${2:-}"

if [ -z "$TITLE" ]; then
    die "Usage: just new-patch \"title\" [PROJ-NNN]"
fi

PROJECT_DIR=$(resolve_project_dir "${PROJECT_ID:-}")
PROJECT_ID=$(basename "$PROJECT_DIR" | awk -F- '{print $1"-"$2}')

# PATCH ids are their own repo-wide continuous sequence (separate from SPEC).
PATCH_ID=$(next_id PATCH)
SLUG=$(slugify "$TITLE")
PATCH_FILE="${PROJECT_DIR}/patches/${PATCH_ID}-${SLUG}.md"

# A hand-created project may not have a patches/ dir yet.
mkdir -p "${PROJECT_DIR}/patches"

if [ -f "$PATCH_FILE" ]; then
    die "Patch file already exists: ${PATCH_FILE}"
fi

TEMPLATE="${REPO_ROOT}/projects/_templates/patch.md"
if [ ! -f "$TEMPLATE" ]; then
    die "Template not found: ${TEMPLATE}. Did init run correctly?"
fi

cp "$TEMPLATE" "$PATCH_FILE"

sed_inplace() {
    if [ "$(uname)" = "Darwin" ]; then
        sed -i '' "$@"
    else
        sed -i "$@"
    fi
}

# Escape user-controlled values before substitution (see _lib.sh).
TITLE_ESC=$(sed_escape_replacement "$TITLE")
REPO_ID_ESC=$(sed_escape_replacement "$(get_repo_id)")

sed_inplace "s|PATCH-XXX|${PATCH_ID}|g" "$PATCH_FILE"
sed_inplace "s|PROJ-XXX|${PROJECT_ID}|g" "$PATCH_FILE"
sed_inplace "s|<the shipped behavior this fixes>|${TITLE_ESC}|g" "$PATCH_FILE"
sed_inplace "s|__TODAY__|$(today)|g" "$PATCH_FILE"
sed_inplace "s|__REPO_ID__|${REPO_ID_ESC}|g" "$PATCH_FILE"
# Stamp the build/verify models from .repo-context tier_map (DEC-005).
sed_inplace "s|__IMPLEMENTER_MODEL__|$(get_tier_model build)|g" "$PATCH_FILE"
sed_inplace "s|__VERIFIER_MODEL__|$(get_tier_model verify)|g" "$PATCH_FILE"

# Cycle-prompt dir, mirroring specs/prompts/.
mkdir -p "${PROJECT_DIR}/patches/prompts"

success "Created ${PATCH_FILE}"
echo ""
echo "Patch lane (DEC-003): patch -> verify -> ship. The independent verify is KEPT."
echo "Next:"
echo "  1. Fill in Problem / Fix / Failing Tests (design+build fused, test-first)."
echo "  2. Run the gate suite + an INDEPENDENT verify session, then:"
echo "       just advance-cycle ${PATCH_ID} ship"
echo "       just archive-patch ${PATCH_ID}"
