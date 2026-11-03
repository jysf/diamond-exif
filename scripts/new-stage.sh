#!/usr/bin/env bash
# scripts/new-stage.sh — scaffold a new stage.
# Usage: new-stage.sh "short title" [PROJ-NNN]

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/_lib.sh"

require_initialized

TITLE="${1:-}"
PROJECT_ID="${2:-}"

if [ -z "$TITLE" ]; then
    die "Usage: just new-stage \"title\" [PROJ-NNN]"
fi

PROJECT_DIR=$(resolve_project_dir "${PROJECT_ID:-}")
PROJECT_ID=$(basename "$PROJECT_DIR" | awk -F- '{print $1"-"$2}')

# STAGE ids are continuous across the whole repo (next_id defaults to a
# repo-wide scan), so a new project keeps counting up rather than restarting
# at 001. See AGENTS.md (Work Hierarchy).
STAGE_ID=$(next_id STAGE)
SLUG=$(slugify "$TITLE")
STAGE_FILE="${PROJECT_DIR}/stages/${STAGE_ID}-${SLUG}.md"

if [ -f "$STAGE_FILE" ]; then
    die "Stage file already exists: ${STAGE_FILE}"
fi

TEMPLATE="${REPO_ROOT}/projects/_templates/stage.md"
if [ ! -f "$TEMPLATE" ]; then
    die "Template not found: ${TEMPLATE}. Did init run correctly?"
fi

# A hand-created project (copied from project-brief.md) may not have a stages/
# dir yet; create it so scaffolding works without a separate new-project step.
mkdir -p "${PROJECT_DIR}/stages"
cp "$TEMPLATE" "$STAGE_FILE"

sed_inplace() {
    if [ "$(uname)" = "Darwin" ]; then
        sed -i '' "$@"
    else
        sed -i "$@"
    fi
}

# Escape user-controlled values before substituting them into the
# template (see sed_escape_replacement in _lib.sh).
TITLE_ESC=$(sed_escape_replacement "$TITLE")
REPO_ID_ESC=$(sed_escape_replacement "$(get_repo_id)")

sed_inplace "s|STAGE-XXX|${STAGE_ID}|g" "$STAGE_FILE"
sed_inplace "s|PROJ-XXX|${PROJECT_ID}|g" "$STAGE_FILE"
sed_inplace "s|<Short Title — the coherent outcome>|${TITLE_ESC}|g" "$STAGE_FILE"
sed_inplace "s|__TODAY__|$(today)|g" "$STAGE_FILE"
sed_inplace "s|__REPO_ID__|${REPO_ID_ESC}|g" "$STAGE_FILE"

success "Created ${STAGE_FILE}"
echo ""
echo "Next steps:"
echo "  1. Fill in the stage with Claude (use Prompt 1b: STAGE FRAME from FIRST_SESSION_PROMPTS.md)"
echo "  2. When ready, scaffold the first spec:"
echo "       just new-spec \"first task title\" ${STAGE_ID}"
