#!/usr/bin/env bash
# scripts/new-spike.sh — scaffold a new spike (the bounded-exploration lane, DEC-012).
#
# A spike is the phase BEFORE you know the shape. Two modes, one discipline:
#   question — a timeboxed investigation (code is evidence, usually discarded)
#   build    — a vibe-coding session (code is the deliverable, you intend to keep it)
#
# Collapsed cycle: spike -> land. There is deliberately no verify (nothing to
# verify against); the timebox and the MANDATORY land step replace it.
#
# A spike is REPO-level, not project-level — it may precede any project, which is
# the case that motivated the lane. `project.id` is optional and back-linked at land.
#
# Usage: new-spike.sh "the question" [--timebox 1d] [--mode build] [PROJ-NNN]
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/_lib.sh
source "${SCRIPT_DIR}/_lib.sh"

require_initialized

TITLE=""
TIMEBOX="1 session"
MODE="question"
PROJECT_ID=""

while [ $# -gt 0 ]; do
    case "$1" in
        --timebox)
            [ $# -ge 2 ] || die "--timebox needs a value (e.g. --timebox 2h)"
            TIMEBOX="$2"; shift 2 ;;
        --timebox=*) TIMEBOX="${1#*=}"; shift ;;
        --mode)
            [ $# -ge 2 ] || die "--mode needs a value (question|build)"
            MODE="$2"; shift 2 ;;
        --mode=*) MODE="${1#*=}"; shift ;;
        PROJ-*) PROJECT_ID="$1"; shift ;;
        # The justfile always passes a PROJECT_ID slot; empty means "not given".
        "") shift ;;
        -*) die "Unknown flag: $1
Usage: just new-spike \"the question\" [TIMEBOX] [MODE] [PROJ-NNN]" ;;
        *)
            if [ -z "$TITLE" ]; then TITLE="$1"; else
                die "Unexpected argument: $1
Usage: just new-spike \"the question\" [TIMEBOX] [MODE] [PROJ-NNN]"
            fi
            shift ;;
    esac
done

if [ -z "$TITLE" ]; then
    die "Usage: just new-spike \"the question\" [--timebox 1d] [--mode build] [PROJ-NNN]

A spike with no question is just coding. State what you're trying to learn."
fi

case "$MODE" in
    question|build) : ;;
    *) die "--mode must be 'question' (timeboxed investigation) or 'build' (vibe-coding session), got: ${MODE}" ;;
esac

# Spikes live at the REPO root, not under a project — a spike may precede any
# project (DEC-012). An optional PROJ-NNN is recorded as a back-link.
SPIKES_DIR="${REPO_ROOT}/spikes"
mkdir -p "${SPIKES_DIR}/done"

# Resolve the optional project back-link. Only validated if one was passed.
PROJECT_FIELD="null"
if [ -n "$PROJECT_ID" ]; then
    PROJECT_DIR=$(resolve_project_dir "$PROJECT_ID")
    PROJECT_FIELD=$(basename "$PROJECT_DIR" | awk -F- '{print $1"-"$2}')
fi

# SPIKE ids are their own repo-wide continuous sequence (separate from SPEC/PATCH).
SPIKE_ID=$(next_id SPIKE)
SLUG=$(slugify "$TITLE")
SPIKE_FILE="${SPIKES_DIR}/${SPIKE_ID}-${SLUG}.md"

if [ -f "$SPIKE_FILE" ]; then
    die "Spike file already exists: ${SPIKE_FILE}"
fi

TEMPLATE="${REPO_ROOT}/projects/_templates/spike.md"
if [ ! -f "$TEMPLATE" ]; then
    die "Template not found: ${TEMPLATE}. Did init run correctly?"
fi

cp "$TEMPLATE" "$SPIKE_FILE"

sed_inplace() {
    if [ "$(uname)" = "Darwin" ]; then
        sed -i '' "$@"
    else
        sed -i "$@"
    fi
}

# Escape user-controlled values before substitution (see _lib.sh / CONTRIBUTING).
# All placeholders are __TOKEN__ style rather than literal prose: a prose
# placeholder containing the sed delimiter (e.g. "2h | 1d") silently fails to
# substitute — the same substitution-escaping bug class fixed in v5.8.
TITLE_ESC=$(sed_escape_replacement "$TITLE")
TIMEBOX_ESC=$(sed_escape_replacement "$TIMEBOX")
REPO_ID_ESC=$(sed_escape_replacement "$(get_repo_id)")

sed_inplace "s|SPIKE-XXX|${SPIKE_ID}|g" "$SPIKE_FILE"
sed_inplace "s|__QUESTION__|${TITLE_ESC}|g" "$SPIKE_FILE"
sed_inplace "s|__TIMEBOX__|${TIMEBOX_ESC}|g" "$SPIKE_FILE"
sed_inplace "s|__MODE__|${MODE}|g" "$SPIKE_FILE"
sed_inplace "s|__PROJECT_ID__|${PROJECT_FIELD}|g" "$SPIKE_FILE"
sed_inplace "s|__TODAY__|$(today)|g" "$SPIKE_FILE"
sed_inplace "s|__REPO_ID__|${REPO_ID_ESC}|g" "$SPIKE_FILE"
sed_inplace "s|__IMPLEMENTER_MODEL__|$(get_tier_model build)|g" "$SPIKE_FILE"

success "Created ${SPIKE_FILE}"
echo ""
echo "Spike lane (DEC-012): spike -> land.  mode=${MODE}  timebox=${TIMEBOX}"
echo ""
echo "During the spike: no spec, no failing tests, no DEC required. Speed IS the value."
echo "Hitting the timebox without an answer is 'inconclusive' — a real result. Don't extend twice."
echo ""
echo "Next:"
echo "  1. Explore. Keep notes in ## Log (no conventions there — it's yours)."
echo "  2. LAND it (mandatory — this is the whole point of the lane):"
echo "       fill ## Land, set spike.outcome, emit DECs for load-bearing choices"
echo "       just advance-cycle ${SPIKE_ID} land"
echo "       just archive-spike ${SPIKE_ID}"
