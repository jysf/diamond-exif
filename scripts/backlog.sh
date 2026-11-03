#!/usr/bin/env bash
# scripts/backlog.sh — spec-grained "what should I pick up next" view.
#
# Surfaces three things `just status` deliberately doesn't:
#   1. In-flight specs (cycle ≠ ship/archived) grouped by cycle, scoped
#      to the active stage by default.
#   2. Stage backlog bullets that haven't been promoted to specs yet —
#      the `(not yet written)` rows in the active stage's `## Spec
#      Backlog`. This is the main new value vs. status.
#   3. Counts (not contents) of un-promoted bullets in upcoming stages.
#
# `--all` expands #1 and #2 to non-active stages too.
#
# Read-only. No writes.

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/_lib.sh"

require_initialized

SHOW_ALL=0
JSON_OUT=0
for arg in "$@"; do
    case "$arg" in
        --all) SHOW_ALL=1 ;;
        --json) JSON_OUT=1 ;;
        -h|--help)
            cat <<EOF
Usage: just backlog [--all] [--json]

  --all    Include in-flight specs and un-promoted bullets across all
           stages, not just the active one. Counts for upcoming stages
           stay rolled up.
  --json   Machine-readable output (DEC-001 §2).
EOF
            exit 0
            ;;
        *) usage_error "Unknown argument: $arg (try --help)" ;;
    esac
done

ACTIVE_PROJECT=$(get_active_project)
ACTIVE_DIR="${REPO_ROOT}/projects/${ACTIVE_PROJECT}"
STAGES_DIR="${ACTIVE_DIR}/stages"
SPECS_DIR="${ACTIVE_DIR}/specs"

ACTIVE_STAGE_FILE=$(get_active_stage_file "$ACTIVE_DIR" || true)
ACTIVE_STAGE_ID=""
ACTIVE_STAGE_NAME=""
if [ -n "$ACTIVE_STAGE_FILE" ]; then
    ACTIVE_STAGE_NAME=$(basename "$ACTIVE_STAGE_FILE" .md)
    ACTIVE_STAGE_ID=$(echo "$ACTIVE_STAGE_NAME" | sed -E 's/^(STAGE-[0-9]+).*/\1/')
fi

# --- Helpers --------------------------------------------------------

# get_spec_complexity lives in _lib.sh — the expected size it reads is the same
# one `just calibration` compares against the actual.

# Read project.stage from a spec.
get_spec_stage_id() {
    awk '
        /^---$/ { fm = !fm; next }
        !fm { exit }
        /^project:/ { in_p = 1; next }
        in_p && /^[a-zA-Z_]/ { in_p = 0 }
        in_p && /^[[:space:]]+stage:/ { print $2; exit }
    ' "$1"
}

# extract_unpromoted_bullets lives in _lib.sh — `just frame-stage` promotes
# exactly the lines this view lists, so both read the backlog the same way.

# Trim a bullet line to "summary [complexity]" form. Strips the
# leading `- [ ] (not yet written) — ` and surfaces a complexity
# tag if present. Best-effort; the convention isn't enforced.
format_unpromoted_bullet() {
    local line="$1"
    local summary
    summary=$(echo "$line" \
        | sed -E 's/^[[:space:]]*-[[:space:]]*\[[ x~?]\][[:space:]]*//' \
        | sed -E 's/\(not yet written\)[[:space:]]*—[[:space:]]*//' \
        | sed -E 's/\(not yet written\)[[:space:]]*-[[:space:]]*//')
    # Best-effort complexity extraction: a bracketed S/M/L token.
    local complexity=""
    if [[ "$summary" =~ \[([SML])\] ]]; then
        complexity="${BASH_REMATCH[1]}"
        summary=$(echo "$summary" | sed -E 's/[[:space:]]*\[[SML]\][[:space:]]*//')
    fi
    summary=$(echo "$summary" | sed -E 's/[[:space:]]+$//')
    if [ -n "$complexity" ]; then
        printf "%-50s [%s]\n" "$summary" "$complexity"
    else
        printf "%s\n" "$summary"
    fi
}

# Count un-promoted bullets in a stage file.
count_unpromoted_bullets() {
    extract_unpromoted_bullets "$1" | wc -l | tr -d ' '
}

# --- JSON output (DEC-001 §2) ---------------------------------------
if [ "$JSON_OUT" = 1 ]; then
    scope=$([ "$SHOW_ALL" = 1 ] && printf all || printf active)
    inflight_json=(); unpromoted_json=(); upcoming_json=()

    if [ -d "$SPECS_DIR" ]; then
        for f in "${SPECS_DIR}"/SPEC-*.md; do
            [ -f "$f" ] || continue
            cycle=$(get_spec_cycle "$f")
            case "$cycle" in frame|design|build|verify|ship) ;; *) continue ;; esac
            stage_id=$(get_spec_stage_id "$f")
            if [ "$SHOW_ALL" = 1 ] || [ "$stage_id" = "$ACTIVE_STAGE_ID" ]; then
                sid=$(basename "$f" | sed -E 's/^(SPEC-[0-9]+).*/\1/')
                cx=$(get_spec_complexity "$f"); [ -n "$cx" ] || cx="?"
                inflight_json+=("$(json_obj "task.id" "$(json_qs "$sid")" "task.cycle" "$(json_qs "$cycle")" \
                    "task.complexity" "$(json_qs "$cx")" "project.stage" "$(json_qs "$stage_id")")")
            fi
        done
    fi

    # Append unpromoted-bullet objects for one stage to unpromoted_json (global).
    emit_bullets_for() {
        local sf="$1" sidp="$2" line summary complexity
        while IFS= read -r line; do
            [ -n "$line" ] || continue
            summary=$(printf '%s' "$line" | sed -E 's/^[[:space:]]*-[[:space:]]*\[[ x~?]\][[:space:]]*//; s/\(not yet written\)[[:space:]]*[—-][[:space:]]*//')
            complexity=null
            if [[ "$summary" =~ \[([SML])\] ]]; then
                complexity=$(json_qs "${BASH_REMATCH[1]}")
                summary=$(printf '%s' "$summary" | sed -E 's/[[:space:]]*\[[SML]\][[:space:]]*//')
            fi
            summary=$(printf '%s' "$summary" | sed -E 's/[[:space:]]+$//')
            unpromoted_json+=("$(json_obj "project.stage" "$(json_qs "$sidp")" summary "$(json_qs "$summary")" complexity "$complexity")")
        done <<< "$(extract_unpromoted_bullets "$sf")"
    }
    if [ -d "$STAGES_DIR" ]; then
        if [ "$SHOW_ALL" = 1 ]; then
            for s in "${STAGES_DIR}"/STAGE-*.md; do
                [ -f "$s" ] || continue
                emit_bullets_for "$s" "$(basename "$s" .md | sed -E 's/^(STAGE-[0-9]+).*/\1/')"
            done
        elif [ -n "$ACTIVE_STAGE_FILE" ]; then
            emit_bullets_for "$ACTIVE_STAGE_FILE" "$ACTIVE_STAGE_ID"
        fi
    fi

    if [ -d "$STAGES_DIR" ]; then
        for s in "${STAGES_DIR}"/STAGE-*.md; do
            [ -f "$s" ] || continue
            sid=$(basename "$s" .md | sed -E 's/^(STAGE-[0-9]+).*/\1/')
            [ "$sid" = "$ACTIVE_STAGE_ID" ] && continue
            status=$(get_stage_status "$s")
            { [ "$status" = shipped ] || [ "$status" = cancelled ]; } && continue
            cnt=$(count_unpromoted_bullets "$s")
            upcoming_json+=("$(json_obj "project.stage" "$(json_qs "$sid")" backlog_count "$cnt")")
        done
    fi

    mkarr() { if [ "$#" -gt 0 ] && [ -n "${1:-}" ]; then json_arr "$@"; else printf '[]'; fi; }
    data=$(json_obj \
        active_project "$(json_qs "$ACTIVE_PROJECT")" \
        active_stage "$([ -n "$ACTIVE_STAGE_ID" ] && json_qs "$ACTIVE_STAGE_ID" || printf null)" \
        scope "$(json_qs "$scope")" \
        in_flight "$(mkarr "${inflight_json[@]:-}")" \
        unpromoted "$(mkarr "${unpromoted_json[@]:-}")" \
        upcoming "$(mkarr "${upcoming_json[@]:-}")")
    json_emit backlog "$data"
    exit 0
fi

# --- Output ---------------------------------------------------------

echo "${BOLD}Backlog for ${ACTIVE_PROJECT}${RESET}"
if [ -n "$ACTIVE_STAGE_NAME" ]; then
    echo "  ${DIM}Active stage:${RESET} ${ACTIVE_STAGE_NAME}"
else
    echo "  ${DIM}(no active stage in this project yet)${RESET}"
fi
echo ""

# 1) In-flight specs --------------------------------------------------
echo "${BOLD}Specs in flight${RESET}"
if [ -d "$SPECS_DIR" ]; then
    in_flight=()
    for f in "${SPECS_DIR}"/SPEC-*.md; do
        [ -f "$f" ] || continue
        cycle=$(get_spec_cycle "$f")
        # ship cycle still counts as "in flight" only until archived;
        # archived specs live in done/ and we already excluded that.
        case "$cycle" in
            frame|design|build|verify|ship)
                stage_id=$(get_spec_stage_id "$f")
                if [ "$SHOW_ALL" = "1" ] || [ "$stage_id" = "$ACTIVE_STAGE_ID" ]; then
                    name=$(basename "$f" .md)
                    complexity=$(get_spec_complexity "$f")
                    in_flight+=("$cycle|$name|${complexity:-?}|$stage_id")
                fi
                ;;
        esac
    done
    if [ "${#in_flight[@]}" -gt 0 ]; then
        # Sort by cycle order, then by name.
        printf "  ${DIM}(%d total)${RESET}\n" "${#in_flight[@]}"
        for cycle in frame design build verify ship; do
            for entry in "${in_flight[@]}"; do
                IFS='|' read -r c n cx s <<< "$entry"
                if [ "$c" = "$cycle" ]; then
                    if [ "$SHOW_ALL" = "1" ]; then
                        printf "    %-32s cycle: %-7s complexity: %-2s ${DIM}(%s)${RESET}\n" "$n" "$c" "$cx" "$s"
                    else
                        printf "    %-32s cycle: %-7s complexity: %s\n" "$n" "$c" "$cx"
                    fi
                fi
            done
        done
    else
        if [ "$SHOW_ALL" = "1" ]; then
            echo "  ${DIM}(none across all stages)${RESET}"
        else
            echo "  ${DIM}(none in active stage; try --all)${RESET}"
        fi
    fi
else
    echo "  ${DIM}(no specs/ directory yet)${RESET}"
fi
echo ""

# 2) Un-promoted bullets in active stage (or all) ---------------------
echo "${BOLD}Stage backlog (not yet specced)${RESET}"
if [ -d "$STAGES_DIR" ]; then
    if [ "$SHOW_ALL" = "1" ]; then
        any=0
        for s in "${STAGES_DIR}"/STAGE-*.md; do
            [ -f "$s" ] || continue
            sid=$(basename "$s" .md | sed -E 's/^(STAGE-[0-9]+).*/\1/')
            bullets=$(extract_unpromoted_bullets "$s")
            if [ -n "$bullets" ]; then
                any=1
                echo "  — ${sid}"
                while IFS= read -r line; do
                    echo "    $(format_unpromoted_bullet "$line")"
                done <<< "$bullets"
            fi
        done
        if [ "$any" = "0" ]; then echo "  ${DIM}(none)${RESET}"; fi
    else
        if [ -n "$ACTIVE_STAGE_FILE" ]; then
            bullets=$(extract_unpromoted_bullets "$ACTIVE_STAGE_FILE")
            if [ -n "$bullets" ]; then
                echo "  ${DIM}— ${ACTIVE_STAGE_ID}${RESET}"
                while IFS= read -r line; do
                    echo "    $(format_unpromoted_bullet "$line")"
                done <<< "$bullets"
            else
                echo "  ${DIM}(no un-promoted bullets in ${ACTIVE_STAGE_ID})${RESET}"
            fi
        else
            echo "  ${DIM}(no active stage)${RESET}"
        fi
    fi
else
    echo "  ${DIM}(no stages/ directory yet)${RESET}"
fi
echo ""

# 3) Upcoming-stage rollup --------------------------------------------
echo "${BOLD}Upcoming stages (counts only)${RESET}"
if [ -d "$STAGES_DIR" ]; then
    upcoming_any=0
    for s in "${STAGES_DIR}"/STAGE-*.md; do
        [ -f "$s" ] || continue
        sid=$(basename "$s" .md | sed -E 's/^(STAGE-[0-9]+).*/\1/')
        # Skip the active stage and shipped stages.
        if [ "$sid" = "$ACTIVE_STAGE_ID" ]; then continue; fi
        status=$(get_stage_status "$s")
        if [ "$status" = "shipped" ] || [ "$status" = "cancelled" ]; then continue; fi
        sname=$(basename "$s" .md)
        count=$(count_unpromoted_bullets "$s")
        printf "  %-44s %d backlog item%s\n" "$sname" "$count" "$([ "$count" = "1" ] && echo "" || echo "s")"
        upcoming_any=1
    done
    if [ "$upcoming_any" = "0" ]; then echo "  ${DIM}(no upcoming stages)${RESET}"; fi
else
    echo "  ${DIM}(no stages/ directory yet)${RESET}"
fi
