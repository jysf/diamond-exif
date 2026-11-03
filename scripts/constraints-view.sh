#!/usr/bin/env bash
# scripts/constraints-view.sh — the `just dash constraints` lens: repo-level
# rules from guidance/constraints.yaml, grouped by severity (blocking first).
# --json emits ContextCore agentGuidance.constraints.* names. Read-only.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/_lib.sh
source "${SCRIPT_DIR}/_lib.sh"

require_initialized
JSON_OUT=$(has_json_flag "$@")

ids=(); sevs=(); paths=(); bys=(); rules=()
while IFS=$'\t' read -r id sev path by rule; do
    [ -n "$id" ] || continue
    ids+=("$id"); sevs+=("${sev:-?}"); paths+=("${path:-}"); bys+=("${by:-?}"); rules+=("$rule")
done < <(emit_constraints_tsv)
total=${#ids[@]}
blocking=0
for s in "${sevs[@]:-}"; do [ "$s" = blocking ] && blocking=$((blocking + 1)); done

if [ "$JSON_OUT" = 1 ]; then
    items=()
    i=0
    while [ "$i" -lt "$total" ]; do
        items+=("$(json_obj \
            "constraint.id" "$(json_qs "${ids[$i]}")" \
            "constraint.severity" "$(json_qs "${sevs[$i]}")" \
            "constraint.paths" "$(json_qs "${paths[$i]}")" \
            "constraint.added_by" "$(json_qs "${bys[$i]}")" \
            "constraint.rule" "$(json_qs "${rules[$i]}")")")
        i=$((i + 1))
    done
    [ "${#items[@]}" -gt 0 ] && arr=$(json_arr "${items[@]}") || arr="[]"
    json_emit constraints "$(json_obj blocking "$blocking" total "$total" constraints "$arr")"
    exit 0
fi

printf "${BOLD}Constraints (%d total · %d blocking)${RESET}\n" "$total" "$blocking"
printf "${DIM}columns: severity · id · paths${RESET}\n"
if [ "$total" -eq 0 ]; then printf "  ${DIM}(none)${RESET}\n"; exit 0; fi
for sev in blocking warning advisory ""; do
    i=0
    while [ "$i" -lt "$total" ]; do
        # "" pass catches any severity value not in the known set.
        match=0
        if [ -n "$sev" ]; then
            [ "${sevs[$i]}" = "$sev" ] && match=1
        else
            case "${sevs[$i]}" in blocking|warning|advisory) : ;; *) match=1 ;; esac
        fi
        if [ "$match" = 1 ]; then
            printf "  [%-8s] %-36s %s\n" "${sevs[$i]}" "${ids[$i]}" "${paths[$i]}"
            printf "             ${DIM}%s${RESET}\n" "${rules[$i]}"
        fi
        i=$((i + 1))
    done
done
