#!/usr/bin/env bash
# scripts/ready.sh — the READY SET: in-flight specs whose blocking dependencies
# have all shipped and that no one currently holds. This is what to pick up next,
# and — because it's computed from declared `depends_on:` rather than guessed —
# it is exactly the set that can be dispatched to sub-agents in parallel.
#
# A spec is:
#   ready    — not shipped, every depends_on spec IS shipped, claimed_by empty
#   blocked  — at least one depends_on spec is not yet shipped
#   claimed  — unblocked but already held (claimed_by set)
#
# Usage:
#   just ready                # active project (default)
#   just ready --all          # every project
#   just ready PROJ-002       # a specific project
#   just ready --json

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/_lib.sh"
require_initialized

SCOPE="active"; TARGET=""; JSON_OUT=0
for arg in "$@"; do
    case "$arg" in
        --all)              SCOPE="all" ;;
        --active|--current) SCOPE="active" ;;
        --json)             JSON_OUT=1 ;;
        --*)                usage_error "Unknown flag: $arg (use --all, --active, --json, or a PROJ-NNN id)" ;;
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

# Emit one classified row per in-flight spec: STATE|proj|id|title|cycle|claimed|blockers
classify_rows() {
    local pdir="$1" sf id cyc claimed dep blockers state
    while IFS= read -r sf; do
        [ -f "$sf" ] || continue
        case "$sf" in *-timeline.md|*/done/*|*/prompts/*) continue ;; esac
        cyc=$(get_spec_cycle "$sf"); [ -n "$cyc" ] || cyc="?"
        [ "$cyc" = "ship" ] && continue
        id=$(basename "$sf" | sed -E 's/^(SPEC-[0-9]+).*/\1/')
        claimed=$(get_spec_claimed_by "$sf")
        blockers=""
        while IFS= read -r dep; do
            [ -n "$dep" ] || continue
            spec_is_shipped "$dep" "$pdir" || blockers="${blockers}${dep} "
        done < <(get_spec_depends_on "$sf")
        if   [ -n "$blockers" ]; then state="blocked"
        elif [ -n "$claimed" ];  then state="claimed"
        else                          state="ready"; fi
        printf '%s|%s|%s|%s|%s|%s|%s\n' \
            "$state" "$(basename "$pdir")" "$id" "$(get_spec_title "$sf")" "$cyc" "$claimed" "${blockers% }"
    done < <(find_all_specs "$pdir" | sort)
}

ROWS=""
for proj in "${PROJECTS[@]}"; do
    pdir="${REPO_ROOT}/projects/${proj}"
    [ -d "$pdir" ] || continue
    ROWS="${ROWS}$(classify_rows "$pdir")"$'\n'
done
ROWS=$(printf '%s' "$ROWS" | grep -v '^$' || true)

n_ready=$(printf '%s\n'   "$ROWS" | grep -c '^ready|'   || true);   n_ready=${n_ready:-0}
n_blocked=$(printf '%s\n' "$ROWS" | grep -c '^blocked|' || true); n_blocked=${n_blocked:-0}
n_claimed=$(printf '%s\n' "$ROWS" | grep -c '^claimed|' || true); n_claimed=${n_claimed:-0}

# ---- JSON (DEC-001 §2) ----
if [ "$JSON_OUT" = 1 ]; then
    specs_json=()
    while IFS='|' read -r st pr id ti cy cl bl; do
        [ -n "$st" ] || continue
        if [ -n "$bl" ]; then
            blk_arr=$(json_arr $(for b in $bl; do json_qs "$b"; done))
        else blk_arr="[]"; fi
        if [ -n "$cl" ]; then cl_json=$(json_qs "$cl"); else cl_json=null; fi
        specs_json+=("$(json_obj \
            state "$(json_qs "$st")" project "$(json_qs "$pr")" "task.id" "$(json_qs "$id")" \
            title "$(json_qs "$ti")" "task.cycle" "$(json_qs "$cy")" \
            claimed_by "$cl_json" blocked_by "$blk_arr")")
    done <<EOF
$ROWS
EOF
    if [ "${#specs_json[@]}" -gt 0 ]; then specs_arr=$(json_arr "${specs_json[@]}"); else specs_arr="[]"; fi
    totals=$(json_obj ready "$n_ready" blocked "$n_blocked" claimed "$n_claimed" projects "${#PROJECTS[@]}")
    data=$(json_obj scope "$(json_qs "$SCOPE")" specs "$specs_arr" totals "$totals")
    json_emit ready "$data"
    exit 0
fi

# ---- Human ----
case "$SCOPE" in
    all)    scope_label="all projects" ;;
    active) scope_label="active project (${PROJECTS[0]})" ;;
    one)    scope_label="${PROJECTS[0]}" ;;
esac
printf "${BOLD}Ready to pick up — %s${RESET}\n" "$scope_label"

if [ "$n_ready" -eq 0 ]; then
    printf "  ${DIM}(nothing ready — %s blocked, %s claimed)${RESET}\n" "$n_blocked" "$n_claimed"
else
    printf '%s\n' "$ROWS" | while IFS='|' read -r st pr id ti cy cl bl; do
        [ "$st" = "ready" ] || continue
        printf "  ${GREEN}%-10s${RESET}  %-8s  %-3s  %s\n" "$id" "$cy" "" "$ti"
    done
fi

# Blocked, with what they wait on — the dependency picture in one glance.
if [ "$n_blocked" -gt 0 ]; then
    printf "\n${DIM}Blocked:${RESET}\n"
    printf '%s\n' "$ROWS" | while IFS='|' read -r st pr id ti cy cl bl; do
        [ "$st" = "blocked" ] || continue
        printf "  ${DIM}%-10s  waits on %s  (%s)${RESET}\n" "$id" "$bl" "$ti"
    done
fi

printf "\n${BOLD}%s ready${RESET} · %s blocked · %s claimed  ${DIM}(%s project(s))${RESET}\n" \
    "$n_ready" "$n_blocked" "$n_claimed" "${#PROJECTS[@]}"
