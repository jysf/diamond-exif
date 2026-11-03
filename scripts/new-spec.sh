#!/usr/bin/env bash
# scripts/new-spec.sh — scaffold a new spec.
# Usage: new-spec.sh "short title" STAGE-NNN [PROJ-NNN] [--release] [--outline] [--complexity X]
#
# --release scaffolds from release-spec.md (task.type: release) instead of
# spec.md — a release cut with the generic runtime pre-flight checklist
# (DEC-006). `just new-release-spec` is the ergonomic wrapper for it.
#
# --outline scaffolds an OUTLINE spec: the standard spec.md, but parked at
# `cycle: frame` with a banner saying what an outline is for — SCOPE and
# DEPENDENCIES only, approach designed just-in-time at `design`. Its point is a
# STABLE ID a sibling spec's `depends_on:` can point at before anyone writes the
# spec. `just frame-stage` is the batch wrapper that promotes a stage's whole
# `(not yet written)` backlog this way.
#
# --complexity X stamps task.complexity (frame-stage carries the `[S]`/`[M]`/`[L]`
# tag from the backlog line so the estimate isn't lost in promotion).

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/_lib.sh"

require_initialized

# Pull the flags out of the arg list (they may appear anywhere), then treat
# the rest as the positional TITLE / STAGE-NNN / PROJ-NNN.
RELEASE=0
OUTLINE=0
COMPLEXITY=""
positional=()
while [ $# -gt 0 ]; do
    case "$1" in
        --release)    RELEASE=1 ;;
        --outline)    OUTLINE=1 ;;
        --complexity) shift; COMPLEXITY="${1:-}"
                      [ -n "$COMPLEXITY" ] || die "--complexity needs a value (e.g. --complexity M)" ;;
        *)            positional+=("$1") ;;
    esac
    shift
done
if [ "$RELEASE" = "1" ] && [ "$OUTLINE" = "1" ]; then
    die "--release and --outline are mutually exclusive: a release cut is never an outline."
fi
# bash 3.2 + set -u: expanding an empty array is an "unbound variable" error,
# so guard the expansion.
set -- ${positional[@]+"${positional[@]}"}

TITLE="${1:-}"
STAGE_ID="${2:-}"
PROJECT_ID="${3:-}"

if [ -z "$TITLE" ] || [ -z "$STAGE_ID" ]; then
    die "Usage: just new-spec \"title\" STAGE-NNN [PROJ-NNN] [--release|--outline]"
fi

PROJECT_DIR=$(resolve_project_dir "${PROJECT_ID:-}")
PROJECT_ID=$(basename "$PROJECT_DIR" | awk -F- '{print $1"-"$2}')

# Verify stage exists in this project
STAGE_FILE=$(find "${PROJECT_DIR}/stages" -type f -name "${STAGE_ID}-*.md" 2>/dev/null | head -n1)
if [ -z "$STAGE_FILE" ]; then
    die "Stage not found in ${PROJECT_ID}: ${STAGE_ID}"
fi

# SPEC ids are continuous across the whole repo (next_id defaults to a
# repo-wide scan), so specs keep counting up across projects rather than
# restarting at 001. See AGENTS.md (Work Hierarchy).
SPEC_ID=$(next_id SPEC)
SLUG=$(slugify "$TITLE")
SPEC_FILE="${PROJECT_DIR}/specs/${SPEC_ID}-${SLUG}.md"
VARIANT=$(get_variant)

# A hand-created project may not have a specs/ dir yet; create it so
# scaffolding works without a separate new-project step.
mkdir -p "${PROJECT_DIR}/specs"

if [ -f "$SPEC_FILE" ]; then
    die "Spec file already exists: ${SPEC_FILE}"
fi

# Choose template: a release cut uses release-spec.md (task.type: release,
# DEC-006); everything else uses the standard spec.md. Both variants ship both.
if [ "$RELEASE" = "1" ]; then
    TEMPLATE="${REPO_ROOT}/projects/_templates/release-spec.md"
else
    TEMPLATE="${REPO_ROOT}/projects/_templates/spec.md"
fi

if [ ! -f "$TEMPLATE" ]; then
    die "Template not found: ${TEMPLATE}. Did init run correctly?"
fi

# Copy template, substitute placeholders
cp "$TEMPLATE" "$SPEC_FILE"

# Use sed to substitute. Portable across macOS/Linux using a wrapper.
sed_inplace() {
    if [ "$(uname)" = "Darwin" ]; then
        sed -i '' "$@"
    else
        sed -i "$@"
    fi
}

# Escape user-controlled values before substituting them into the
# template (see sed_escape_replacement in _lib.sh). The ID/date values
# are tool-generated or validated, but escaping is harmless and keeps
# the substitutions uniform.
TITLE_ESC=$(sed_escape_replacement "$TITLE")
REPO_ID_ESC=$(sed_escape_replacement "$(get_repo_id)")

sed_inplace "s|SPEC-XXX|${SPEC_ID}|g" "$SPEC_FILE"
sed_inplace "s|STAGE-XXX|${STAGE_ID}|g" "$SPEC_FILE"
sed_inplace "s|PROJ-XXX|${PROJECT_ID}|g" "$SPEC_FILE"
sed_inplace "s|<Short Title>|${TITLE_ESC}|g" "$SPEC_FILE"
sed_inplace "s|__TODAY__|$(today)|g" "$SPEC_FILE"
sed_inplace "s|__REPO_ID__|${REPO_ID_ESC}|g" "$SPEC_FILE"
# Stamp the design/build models from .repo-context tier_map (DEC-005). Model
# ids are alphanumeric+hyphen, so no sed-escaping needed.
sed_inplace "s|__ARCHITECT_MODEL__|$(get_tier_model design)|g" "$SPEC_FILE"
sed_inplace "s|__IMPLEMENTER_MODEL__|$(get_tier_model build)|g" "$SPEC_FILE"

# --complexity overrides the template's default task.complexity. Anchored to the
# task-block indent so it can't hit a look-alike line elsewhere. The value is
# free-form on purpose (the scale is a convention, not a gate).
if [ -n "$COMPLEXITY" ]; then
    COMPLEXITY_ESC=$(sed_escape_replacement "$COMPLEXITY")
    sed_inplace "s|^\(  complexity:\)[^#]*|\1 ${COMPLEXITY_ESC}                    |" "$SPEC_FILE"
fi

# An OUTLINE spec is parked at `cycle: frame` and carries a banner stating the
# fidelity line, so nobody mistakes a placeholder for a designed spec.
if [ "$OUTLINE" = "1" ]; then
    sed_inplace "s|^\(  cycle:\) design|\1 frame |" "$SPEC_FILE"
    awk -v id="$SPEC_ID" '
        !done && /^#[[:space:]]*SPEC-[0-9]+/ {
            print; print ""
            print "> **OUTLINE — `cycle: frame`.** This spec exists so its ID is stable and"
            print "> siblings can declare `depends_on: [" id "]`. Capture **scope** (Context /"
            print "> Goal / Non-Goals) and **dependencies** only — the *approach* is designed"
            print "> just-in-time when this moves to `design`. Do not pre-design it here."
            done = 1; next
        }
        { print }
    ' "$SPEC_FILE" > "${SPEC_FILE}.tmp" && mv "${SPEC_FILE}.tmp" "$SPEC_FILE"
fi

# Scaffold the timeline file alongside the spec. Architect appends
# cycle lines as it designs them; executors update status markers.
TIMELINE_TEMPLATE="${REPO_ROOT}/projects/_templates/timeline.md"
TIMELINE_FILE="${PROJECT_DIR}/specs/${SPEC_ID}-${SLUG}-timeline.md"
if [ -f "$TIMELINE_TEMPLATE" ]; then
    cp "$TIMELINE_TEMPLATE" "$TIMELINE_FILE"
    sed_inplace "s|SPEC-XXX|${SPEC_ID}|g" "$TIMELINE_FILE"
else
    # Fallback: inline minimal timeline so new-spec never hard-fails on
    # a freshly-cloned repo whose _templates/ is incomplete.
    cat > "$TIMELINE_FILE" <<EOF
# ${SPEC_ID} timeline

Status markers: \`[ ]\` not started · \`[~]\` in progress · \`[x]\` complete · \`[?]\` blocked.

## Instructions

_(Timeline will be populated as the architect writes each cycle's prompt.)_
EOF
fi

# Ensure prompts/ exists as a sibling to the spec file. The architect
# writes one prompt file per dispatched cycle here.
PROMPTS_DIR="${PROJECT_DIR}/specs/prompts"
mkdir -p "$PROMPTS_DIR"
[ -f "${PROMPTS_DIR}/.gitkeep" ] || touch "${PROMPTS_DIR}/.gitkeep"

success "Created ${SPEC_FILE}"
success "Created ${TIMELINE_FILE}"
if [ "$RELEASE" = "1" ]; then
    echo ""
    echo "Release spec (DEC-006): fill in the ## Release Pre-Flight checklist with"
    echo "the tool-specific command for each generic category before you ship."
fi
if [ "$OUTLINE" = "1" ]; then
    echo ""
    echo "Outline spec at ${BOLD}cycle: frame${RESET} — scope + \`depends_on:\` only."
    echo "Design it (approach, cycles, prompts) when you run:"
    echo "  just advance-cycle ${SPEC_ID} design"
    exit 0
fi
echo ""
echo "Next steps:"
echo "  1. Fill in the spec with Claude (use Prompt 2b: SPEC from FIRST_SESSION_PROMPTS.md)"
echo "  2. Update the stage's backlog in ${STAGE_FILE}"
echo "  3. Architect will write cycle prompts to ${PROMPTS_DIR#$REPO_ROOT/}/"
echo "     and append lines to the timeline as cycles are designed."
echo "  4. When ready for build, run:"
echo "       just advance-cycle ${SPEC_ID} build"
