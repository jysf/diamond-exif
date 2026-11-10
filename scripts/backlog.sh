#!/usr/bin/env bash
# scripts/backlog.sh — spec-grained "what should I pick up next" view.
#
# Surfaces three things `just status` deliberately doesn't:
#   1. In-flight specs (cycle ≠ ship/archived) grouped by cycle, scoped
#      to the active stage(s) by default. A project can run several at once.
#   2. Stage backlog bullets that haven't been promoted to specs yet — open
#      `- [ ]` rows in each active stage's `## Spec Backlog` that do not lead
#      with a spec id (extract_unpromoted_bullets). The main value vs. status.
#   3. Counts (not contents) of un-promoted bullets in upcoming stages.
#   4. Counts for `proposed` projects: the next waves, whose filed work was
#      invisible here before (54 items in crustyimg).
#
# `--all` expands #1 and #2 to non-active stages too, and #4 to per-stage rows.
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

# STAGE-NNN from a stage file path.
stage_id_of() { basename "$1" .md | sed -E 's/^(STAGE-[0-9]+).*/\1/'; }
# True when a STAGE-NNN is one of the active stages.
is_active_stage() { case " ${ACTIVE_STAGE_IDS} " in *" $1 "*) return 0 ;; esac; return 1; }

ACTIVE_PROJECT=$(get_active_project)
ACTIVE_DIR="${REPO_ROOT}/projects/${ACTIVE_PROJECT}"
STAGES_DIR="${ACTIVE_DIR}/stages"
SPECS_DIR="${ACTIVE_DIR}/specs"

# Every active stage (crustyimg ran three at once; showing only the first hid a
# spec mid-build in the second). ACTIVE_STAGE_ID stays the first, for --json's
# `active_stage`; ACTIVE_STAGE_IDS holds them all, space-separated.
ACTIVE_STAGE_FILES=()
while IFS= read -r f; do
    [ -n "$f" ] && ACTIVE_STAGE_FILES+=("$f")
done <<EOF
$(get_active_stage_files "$ACTIVE_DIR" || true)
EOF
ACTIVE_STAGE_IDS=""
ACTIVE_STAGE_NAMES=""
for f in ${ACTIVE_STAGE_FILES[@]+"${ACTIVE_STAGE_FILES[@]}"}; do
    n=$(basename "$f" .md)
    ACTIVE_STAGE_NAMES="${ACTIVE_STAGE_NAMES:+${ACTIVE_STAGE_NAMES}, }${n}"
    ACTIVE_STAGE_IDS="${ACTIVE_STAGE_IDS:+${ACTIVE_STAGE_IDS} }$(stage_id_of "$f")"
done
ACTIVE_STAGE_ID="${ACTIVE_STAGE_IDS%% *}"

# --- Helpers --------------------------------------------------------

# get_spec_complexity lives in _lib.sh — the expected size it reads is the same
# one `just calibration` compares against the actual.

# get_spec_stage (project.stage) and extract_unpromoted_bullets live in _lib.sh.
# frame-stage promotes the marked subset of these bullets and names the rest.

# A bullet's text without the checkbox or a leading "(not yet written…)"
# marker. The dash separator is matched as alternatives, not inside a bracket
# expression: an em dash in [—-] breaks into a bad byte range when the locale
# is unset (crustyimg 6b26006 saw mojibake from exactly that).
strip_bullet_marker() {
    printf '%s' "$1" \
        | sed -E 's/^[[:space:]]*-[[:space:]]*\[[ x~?]\][[:space:]]*//' \
        | sed -E 's/^\(not yet written[^)]*\)[[:space:]]*(—|-)?[[:space:]]*//'
}

# Trim a bullet line to "summary [complexity]" form, surfacing a complexity
# tag if present. Best-effort; the convention isn't enforced.
format_unpromoted_bullet() {
    local line="$1"
    local summary
    summary=$(strip_bullet_marker "$line")
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

# count_unpromoted_bullets lives in _lib.sh, shared with roadmap and
# specs-by-stage so the three can no longer disagree.

# Projects whose brief says `status: proposed`, one dir per line, in id order.
proposed_project_dirs() {
    local d
    find "${REPO_ROOT}/projects" -maxdepth 1 -type d -name 'PROJ-*' 2>/dev/null | sort \
        | while IFS= read -r d; do
            [ "$(get_project_status "$d")" = proposed ] && printf '%s\n' "$d"
        done || true
}

# "<stage count>|<backlog count>" for a project's non-shipped stages.
project_backlog_totals() {
    local s st n_st=0 n_b=0
    for s in "$1"/stages/STAGE-*.md; do
        [ -f "$s" ] || continue
        st=$(get_stage_status "$s")
        { [ "$st" = shipped ] || [ "$st" = cancelled ]; } && continue
        n_st=$((n_st + 1))
        n_b=$((n_b + $(count_unpromoted_bullets "$s")))
    done
    printf '%d|%d\n' "$n_st" "$n_b"
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
            stage_id=$(get_spec_stage "$f")
            if [ "$SHOW_ALL" = 1 ] || is_active_stage "$stage_id"; then
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
            summary=$(strip_bullet_marker "$line")
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
        else
            for s in ${ACTIVE_STAGE_FILES[@]+"${ACTIVE_STAGE_FILES[@]}"}; do
                emit_bullets_for "$s" "$(stage_id_of "$s")"
            done
        fi
    fi

    if [ -d "$STAGES_DIR" ]; then
        for s in "${STAGES_DIR}"/STAGE-*.md; do
            [ -f "$s" ] || continue
            sid=$(stage_id_of "$s")
            is_active_stage "$sid" && continue
            status=$(get_stage_status "$s")
            { [ "$status" = shipped ] || [ "$status" = cancelled ]; } && continue
            cnt=$(count_unpromoted_bullets "$s")
            upcoming_json+=("$(json_obj "project.stage" "$(json_qs "$sid")" backlog_count "$cnt")")
        done
    fi

    active_ids_json=()
    for sid in $ACTIVE_STAGE_IDS; do active_ids_json+=("$(json_qs "$sid")"); done
    proposed_json=()
    while IFS= read -r d; do
        [ -n "$d" ] || continue
        t=$(project_backlog_totals "$d")
        proposed_json+=("$(json_obj project "$(json_qs "$(basename "$d")")" \
            stages "${t%%|*}" backlog_count "${t#*|}")")
    done <<EOF
$(proposed_project_dirs)
EOF

    mkarr() { if [ "$#" -gt 0 ] && [ -n "${1:-}" ]; then json_arr "$@"; else printf '[]'; fi; }
    data=$(json_obj \
        active_project "$(json_qs "$ACTIVE_PROJECT")" \
        active_stage "$([ -n "$ACTIVE_STAGE_ID" ] && json_qs "$ACTIVE_STAGE_ID" || printf null)" \
        active_stages "$(mkarr "${active_ids_json[@]:-}")" \
        scope "$(json_qs "$scope")" \
        in_flight "$(mkarr "${inflight_json[@]:-}")" \
        unpromoted "$(mkarr "${unpromoted_json[@]:-}")" \
        upcoming "$(mkarr "${upcoming_json[@]:-}")" \
        proposed_projects "$(mkarr "${proposed_json[@]:-}")")
    json_emit backlog "$data"
    exit 0
fi

# --- Output ---------------------------------------------------------

echo "${BOLD}Backlog for ${ACTIVE_PROJECT}${RESET}"
if [ "${#ACTIVE_STAGE_FILES[@]}" -gt 1 ]; then
    echo "  ${DIM}Active stages:${RESET} ${ACTIVE_STAGE_NAMES}"
elif [ -n "$ACTIVE_STAGE_NAMES" ]; then
    echo "  ${DIM}Active stage:${RESET} ${ACTIVE_STAGE_NAMES}"
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
                stage_id=$(get_spec_stage "$f")
                if [ "$SHOW_ALL" = "1" ] || is_active_stage "$stage_id"; then
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
        if [ "${#ACTIVE_STAGE_FILES[@]}" -gt 0 ]; then
            for s in "${ACTIVE_STAGE_FILES[@]}"; do
                sid=$(stage_id_of "$s")
                bullets=$(extract_unpromoted_bullets "$s")
                if [ -n "$bullets" ]; then
                    echo "  ${DIM}— ${sid}${RESET}"
                    while IFS= read -r line; do
                        echo "    $(format_unpromoted_bullet "$line")"
                    done <<< "$bullets"
                else
                    echo "  ${DIM}(no un-promoted bullets in ${sid})${RESET}"
                fi
            done
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
        # Skip the active stage(s) and shipped stages.
        if is_active_stage "$sid"; then continue; fi
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
echo ""

# 4) Proposed projects ------------------------------------------------
# The next waves. Counts only; --all breaks each into its stages.
echo "${BOLD}Proposed projects (counts only)${RESET}"
proposed_any=0
while IFS= read -r d; do
    [ -n "$d" ] || continue
    proposed_any=1
    t=$(project_backlog_totals "$d")
    printf "  %-44s %d stage%s · %d backlog item%s\n" "$(basename "$d")" \
        "${t%%|*}" "$([ "${t%%|*}" = 1 ] && echo "" || echo "s")" \
        "${t#*|}" "$([ "${t#*|}" = 1 ] && echo "" || echo "s")"
    if [ "$SHOW_ALL" = 1 ]; then
        for s in "$d"/stages/STAGE-*.md; do
            [ -f "$s" ] || continue
            printf "      %-40s %d\n" "$(basename "$s" .md)" "$(count_unpromoted_bullets "$s")"
        done
    fi
done <<EOF
$(proposed_project_dirs)
EOF
if [ "$proposed_any" = 0 ]; then echo "  ${DIM}(none)${RESET}"; fi
