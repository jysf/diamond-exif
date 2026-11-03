#!/usr/bin/env bash
# scripts/release-notes.sh — assemble OUTWARD-facing release notes from what the
# specs already recorded. Nothing is authored twice: each shipped spec's ship
# Reflection Q5 ("what can a user do now that they couldn't before?") IS the
# note. Specs that answered `none` are infrastructure and are correctly absent —
# that's why `none` is a recorded value rather than a blank.
#
# Three audiences, three artifacts — don't collapse them:
#   release notes  → whoever USES the thing (this script)
#   CHANGELOG      → the record of what changed
#   Reflection     → internal learning (Q1-Q3)
#
# Modes mirror `lifetime-report`: assembled notes by default, or a synthesis
# prompt for an LLM to turn them into a written announcement.
#
# Usage:
#   just release-notes                # active project
#   just release-notes STAGE-003      # one stage
#   just release-notes PROJ-002       # one project
#   just release-notes --all          # every project
#   just release-notes --prompt       # wrap the notes in a synthesis ask
#   just release-notes --json

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/_lib.sh"
require_initialized

SCOPE="active"; TARGET=""; STAGE_FILTER=""; JSON_OUT=0; PROMPT_OUT=0
for arg in "$@"; do
    case "$arg" in
        --all)              SCOPE="all" ;;
        --active|--current) SCOPE="active" ;;
        --prompt)           PROMPT_OUT=1 ;;
        --json)             JSON_OUT=1 ;;
        --*)                usage_error "Unknown flag: $arg (use --all, --prompt, --json, a STAGE-NNN or a PROJ-NNN id)" ;;
        STAGE-*)            STAGE_FILTER="$arg" ;;
        *)                  SCOPE="one"; TARGET="$arg" ;;
    esac
done

PROJECTS=()
if [ "$SCOPE" = "active" ]; then
    PROJECTS+=("$(get_active_project)")
elif [ "$SCOPE" = "one" ]; then
    dir=$(find "${REPO_ROOT}/projects" -maxdepth 1 -type d -name "${TARGET}*" 2>/dev/null | sort | head -n1)
    [ -n "$dir" ] || die "No project matching '${TARGET}' under projects/."
    PROJECTS+=("$(basename "$dir")")
else
    while IFS= read -r d; do [ -n "$d" ] && PROJECTS+=("$(basename "$d")"); done \
        < <(find "${REPO_ROOT}/projects" -maxdepth 1 -type d -name 'PROJ-*' 2>/dev/null | sort)
fi

# Rows: proj|stage|SPEC-ID|title|outcome  — shipped specs that have a real outcome.
ROWS=""
N_SHIPPED=0; N_SILENT=0
for proj in "${PROJECTS[@]}"; do
    pdir="${REPO_ROOT}/projects/${proj}"
    [ -d "$pdir" ] || continue
    while IFS= read -r sf; do
        [ -f "$sf" ] || continue
        case "$sf" in *-timeline.md|*/prompts/*) continue ;; esac
        # shipped = archived under done/ or carrying cycle: ship
        shipped=0
        case "$sf" in */done/*) shipped=1 ;; esac
        [ "$shipped" = 1 ] || [ "$(get_spec_cycle "$sf")" = "ship" ] || continue
        st=$(awk '/^---$/{f=!f;next} !f{next} /^project:/{p=1;next} p&&/^[a-zA-Z_]/{p=0} p&&/^[[:space:]]+stage:/{print $2;exit}' "$sf")
        [ -z "$STAGE_FILTER" ] || [ "$st" = "$STAGE_FILTER" ] || continue
        N_SHIPPED=$((N_SHIPPED + 1))
        out=$(get_spec_outcome "$sf")
        if [ -z "$out" ]; then N_SILENT=$((N_SILENT + 1)); continue; fi
        id=$(basename "$sf" | sed -E 's/^(SPEC-[0-9]+).*/\1/')
        ROWS="${ROWS}${proj}|${st}|${id}|$(get_spec_title "$sf")|${out}"$'\n'
    done < <(find_all_specs "$pdir" | sort)
done
ROWS=$(printf '%s' "$ROWS" | grep -v '^$' || true)
N_NOTES=$(printf '%s\n' "$ROWS" | grep -c '|' || true); N_NOTES=${N_NOTES:-0}

# ---- JSON (DEC-001 §2) ----
if [ "$JSON_OUT" = 1 ]; then
    notes_json=()
    while IFS='|' read -r pr st id ti ou; do
        [ -n "$id" ] || continue
        notes_json+=("$(json_obj project "$(json_qs "$pr")" "project.stage" "$(json_qs "$st")" \
            "task.id" "$(json_qs "$id")" title "$(json_qs "$ti")" outcome "$(json_qs "$ou")")")
    done <<EOF
$ROWS
EOF
    if [ "${#notes_json[@]}" -gt 0 ]; then notes_arr=$(json_arr "${notes_json[@]}"); else notes_arr="[]"; fi
    totals=$(json_obj notes "$N_NOTES" shipped_specs "$N_SHIPPED" no_user_outcome "$N_SILENT" projects "${#PROJECTS[@]}")
    data=$(json_obj scope "$(json_qs "$SCOPE")" stage "$(json_qs "$STAGE_FILTER")" notes "$notes_arr" totals "$totals")
    json_emit release-notes "$data"
    exit 0
fi

# ---- Prompt mode: same notes, wrapped in a synthesis ask ----
if [ "$PROMPT_OUT" = 1 ]; then
    cat <<'EOF'
# ============================================================
# Release Notes — copy everything below this line into Claude
# ============================================================

Turn the assembled outcomes below into written release notes for the people who
USE this software — not for contributors. Rules:
  - Lead with ONE headline: the single thing a user most gains from this release.
  - Group related outcomes; drop the SPEC ids from the prose.
  - Keep each item in before → after terms, with the confirming number if given.
  - Call out breaking changes and anything requiring action on upgrade.
  - Do NOT invent items. Every line below came from a shipped spec; specs with no
    user-visible outcome were already excluded and must not be back-filled.
EOF
    echo ""
fi

# ---- Human render ----
if [ "$PROMPT_OUT" = 0 ]; then
    label="${STAGE_FILTER:-}"
    case "$SCOPE" in
        all)    scope_label="all projects" ;;
        active) scope_label="${PROJECTS[0]}" ;;
        one)    scope_label="${PROJECTS[0]}" ;;
    esac
    [ -z "$label" ] || scope_label="${scope_label} · ${label}"
    printf "${BOLD}Release notes — %s${RESET}\n" "$scope_label"
    printf "${DIM}assembled from each shipped spec's outward outcome (Reflection Q5)${RESET}\n\n"
fi

if [ "$N_NOTES" -eq 0 ]; then
    printf "  ${DIM}(no user-visible outcomes recorded yet — %s shipped spec(s), %s with no user-facing outcome)${RESET}\n" \
        "$N_SHIPPED" "$N_SILENT"
else
    last_stage=""
    printf '%s\n' "$ROWS" | while IFS='|' read -r pr st id ti ou; do
        [ -n "$id" ] || continue
        if [ "$st" != "$last_stage" ]; then
            printf "${BOLD}%s${RESET}\n" "$st"
            last_stage="$st"
        fi
        printf "  - %s  ${DIM}(%s)${RESET}\n" "$ou" "$id"
    done
fi

if [ "$PROMPT_OUT" = 0 ]; then
    printf "\n${DIM}%s note(s) from %s shipped spec(s); %s had no user-visible outcome.${RESET}\n" \
        "$N_NOTES" "$N_SHIPPED" "$N_SILENT"
fi
