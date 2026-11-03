#!/usr/bin/env bash
# scripts/frame-stage.sh — batch-promote a stage's un-promoted backlog lines
# into OUTLINE specs, so a planned stage becomes a dispatchable, dependency-aware
# batch.
#
# Every `- [ ] (not yet written) — <summary>` line in the stage's ## Spec Backlog
# becomes a real spec at `cycle: frame` with a STABLE ID, and the backlog line is
# rewritten to `- [ ] SPEC-NNN (frame) — <summary>`. Stable IDs are the whole
# point: a sibling can't declare `depends_on: [SPEC-042]` until SPEC-042 exists.
#
# THE FIDELITY LINE: an outline captures SCOPE and DEPENDENCIES, not APPROACH.
# Design stays just-in-time — a stage framed as ten pre-designed specs is ten
# guesses that go stale. That's why outlines land at `frame`, not `design`.
#
# Usage:
#   just frame-stage STAGE-NNN                # active project
#   just frame-stage STAGE-NNN PROJ-002       # a specific project
#   just frame-stage STAGE-NNN --dry-run      # show the ID assignment, write nothing

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/_lib.sh"

require_initialized

DRY_RUN=0
positional=()
for a in "$@"; do
    case "$a" in
        --dry-run|-n) DRY_RUN=1 ;;
        --*)          usage_error "Unknown flag: $a (use --dry-run)" ;;
        *)            positional+=("$a") ;;
    esac
done
set -- ${positional[@]+"${positional[@]}"}

STAGE_ID="${1:-}"
PROJECT_ID="${2:-}"
[ -n "$STAGE_ID" ] || die "Usage: just frame-stage STAGE-NNN [PROJ-NNN] [--dry-run]"

PROJECT_DIR=$(resolve_project_dir "${PROJECT_ID:-}")
PROJECT_ID=$(basename "$PROJECT_DIR" | awk -F- '{print $1"-"$2}')

STAGE_FILE=$(find "${PROJECT_DIR}/stages" -type f -name "${STAGE_ID}-*.md" 2>/dev/null | head -n1)
[ -n "$STAGE_FILE" ] || die "Stage not found in ${PROJECT_ID}: ${STAGE_ID}"

# Split a backlog bullet into "COMPLEXITY|SUMMARY". Mirrors backlog.sh's
# formatter: strip the checkbox and the `(not yet written) —` marker, then lift
# a bracketed [S]/[M]/[L] tag out of the remaining text.
parse_bullet() {
    local line="$1" summary complexity=""
    summary=$(printf '%s' "$line" \
        | sed -E 's/^[[:space:]]*-[[:space:]]*\[[ x~?]\][[:space:]]*//' \
        | sed -E 's/\(not yet written\)[[:space:]]*—[[:space:]]*//' \
        | sed -E 's/\(not yet written\)[[:space:]]*-[[:space:]]*//')
    # Only a recognized size token is lifted out — a bracketed `[CI]` or `[api]`
    # in a summary stays part of the summary.
    if [[ "$summary" =~ \[(XS|S|M|L|XL|XXL)\] ]]; then
        complexity="${BASH_REMATCH[1]}"
        summary=$(printf '%s' "$summary" | sed -E 's/[[:space:]]*\[(XS|S|M|L|XL|XXL)\][[:space:]]*/ /g')
    fi
    summary=$(printf '%s' "$summary" | sed -E 's/^[[:space:]]+//; s/[[:space:]]+$//')
    printf '%s|%s\n' "$complexity" "$summary"
}

SUMMARIES=(); COMPLEXITIES=(); SLUGS=()
while IFS= read -r line; do
    [ -n "$line" ] || continue
    parsed=$(parse_bullet "$line")
    cx="${parsed%%|*}"; sm="${parsed#*|}"
    [ -n "$sm" ] || die "A '(not yet written)' line in ${STAGE_ID} has no summary — give it one first:
  ${line}"
    SUMMARIES+=("$sm"); COMPLEXITIES+=("$cx"); SLUGS+=("$(slugify "$sm")")
done < <(extract_unpromoted_bullets "$STAGE_FILE")

COUNT=${#SUMMARIES[@]}
if [ "$COUNT" = "0" ]; then
    echo "${DIM}No '(not yet written)' backlog lines in ${STAGE_ID} — nothing to frame.${RESET}"
    echo "Add them to the stage's ## Spec Backlog first (that's the planning step"
    echo "this command promotes; it does not invent the backlog for you)."
    exit 0
fi

# Pre-flight the whole batch BEFORE creating anything: a collision discovered
# halfway through would leave specs on disk with an un-rewritten backlog.
i=0
while [ "$i" -lt "$COUNT" ]; do
    j=$((i + 1))
    while [ "$j" -lt "$COUNT" ]; do
        [ "${SLUGS[$i]}" != "${SLUGS[$j]}" ] || \
            die "Two backlog lines slugify the same ('${SLUGS[$i]}') — make the summaries distinct:
  ${SUMMARIES[$i]}
  ${SUMMARIES[$j]}"
        j=$((j + 1))
    done
    existing=$(find "${PROJECT_DIR}/specs" -maxdepth 1 -name "SPEC-*-${SLUGS[$i]}.md" 2>/dev/null | head -n1)
    [ -z "$existing" ] || die "A spec with this slug already exists: ${existing}
Rename the backlog line, or drop it if that spec already covers it."
    i=$((i + 1))
done

# Predict the IDs new-spec will allocate (sequential from the repo-wide next).
NEXT=$(next_id SPEC)
BASE=$((10#$(printf '%s' "$NEXT" | sed -E 's/SPEC-0*([0-9]+)/\1/')))

if [ "$DRY_RUN" = "1" ]; then
    echo "${BOLD}Would frame ${COUNT} outline spec(s) in ${STAGE_ID} (${PROJECT_ID}):${RESET}"
    i=0
    while [ "$i" -lt "$COUNT" ]; do
        printf "  %s  %s%s\n" "$(printf 'SPEC-%03d' $((BASE + i)))" "${SUMMARIES[$i]}" \
            "$([ -n "${COMPLEXITIES[$i]}" ] && printf ' [%s]' "${COMPLEXITIES[$i]}")"
        i=$((i + 1))
    done
    echo ""
    echo "${DIM}Nothing written (--dry-run).${RESET}"
    exit 0
fi

IDS=()
i=0
while [ "$i" -lt "$COUNT" ]; do
    args=("${SUMMARIES[$i]}" "$STAGE_ID" "$PROJECT_ID" --outline)
    [ -z "${COMPLEXITIES[$i]}" ] || args+=(--complexity "${COMPLEXITIES[$i]}")
    "${SCRIPT_DIR}/new-spec.sh" "${args[@]}" >/dev/null
    created=$(find "${PROJECT_DIR}/specs" -maxdepth 1 -name "SPEC-*-${SLUGS[$i]}.md" 2>/dev/null | head -n1)
    [ -n "$created" ] || die "new-spec did not create a spec for: ${SUMMARIES[$i]}"
    IDS+=("$(basename "$created" | sed -E 's/^(SPEC-[0-9]+).*/\1/')")
    success "Framed ${IDS[$i]} — ${SUMMARIES[$i]}"
    i=$((i + 1))
done

# Rewrite the backlog: the Nth "(not yet written)" line takes the Nth new ID,
# then recompute **Count:** the same way archive-spec does (a promoted line is
# now "active", not "pending").
awk -v ids="${IDS[*]}" '
    BEGIN { n = split(ids, id, " ") }
    /^## Spec Backlog/ { inbl = 1; print; next }
    /^## / { if (inbl) inbl = 0 }
    {
        if (inbl && $0 ~ /^-[[:space:]]*\[/) {
            if ($0 ~ /\(not yet written\)/ && k < n) {
                k++
                sub(/\(not yet written\)/, id[k] " (frame)")
            }
            if ($0 ~ /^-[[:space:]]*\[x\]/) shipped++
            else if ($0 ~ /SPEC-[0-9]/) active++
            else pending++
            print; next
        }
        if (inbl && $0 ~ /^\*\*Count:\*\*/) {
            printf "**Count:** %d shipped / %d active / %d pending\n", shipped, active, pending
            next
        }
        print
    }
' "$STAGE_FILE" > "${STAGE_FILE}.tmp" && mv "${STAGE_FILE}.tmp" "$STAGE_FILE"
success "Updated ${STAGE_ID} backlog: ${COUNT} line(s) promoted; **Count:** recomputed."

echo ""
echo "${BOLD}Next — the reason these IDs exist:${RESET}"
echo "  1. Declare the blocking order in each spec's ${BOLD}depends_on:${RESET} (IDs are stable now)."
echo "     ${DIM}${IDS[*]}${RESET}"
echo "  2. Fill each outline's ## Context / ## Goal — ${BOLD}scope only${RESET}. Leave the"
echo "     approach alone; it gets designed at \`design\`, not now."
echo "  3. \`just ready\` then shows the dispatchable set (no unshipped blockers,"
echo "     unclaimed) — that's your safe parallel fan-out."
echo ""
echo "${DIM}At stage close, record how many outlines survived unchanged (Stage-Level"
echo "Reflection) — that number is how you learn whether framing this far ahead pays.${RESET}"
