#!/usr/bin/env bash
# scripts/questions-view.sh — the `just dash questions` lens: open questions
# from guidance/questions.yaml (what's blocking work). Lists non-answered
# questions by priority. --json emits ContextCore guidance.* names
# (type=question). Read-only.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/_lib.sh
source "${SCRIPT_DIR}/_lib.sh"

require_initialized
JSON_OUT=$(has_json_flag "$@")

ids=(); pris=(); sts=(); qs=()
while IFS=$'\t' read -r id pri st q; do
    [ -n "$id" ] || continue
    ids+=("$id"); pris+=("${pri:-?}"); sts+=("${st:-?}"); qs+=("$q")
done < <(emit_questions_tsv)
total=${#ids[@]}
open=0
for s in "${sts[@]:-}"; do [ "$s" = open ] && open=$((open + 1)); done

if [ "$JSON_OUT" = 1 ]; then
    items=()
    i=0
    while [ "$i" -lt "$total" ]; do
        items+=("$(json_obj \
            "guidance.id" "$(json_qs "${ids[$i]}")" \
            "guidance.type" "$(json_qs question)" \
            "guidance.priority" "$(json_qs "${pris[$i]}")" \
            status "$(json_qs "${sts[$i]}")" \
            "guidance.content" "$(json_qs "${qs[$i]}")")")
        i=$((i + 1))
    done
    [ "${#items[@]}" -gt 0 ] && arr=$(json_arr "${items[@]}") || arr="[]"
    json_emit questions "$(json_obj open "$open" total "$total" questions "$arr")"
    exit 0
fi

printf "${BOLD}Open questions (%d open / %d total)${RESET}\n" "$open" "$total"
printf "${DIM}columns: priority · status · id · question${RESET}\n"
if [ "$total" -eq 0 ]; then printf "  ${DIM}(none)${RESET}\n"; exit 0; fi
shown=0
for prio in critical high medium low ""; do
    i=0
    while [ "$i" -lt "$total" ]; do
        # "" pass catches any priority value not in the known set.
        match=0
        if [ -n "$prio" ]; then
            [ "${pris[$i]}" = "$prio" ] && match=1
        else
            case "${pris[$i]}" in critical|high|medium|low) : ;; *) match=1 ;; esac
        fi
        if [ "$match" = 1 ] && [ "${sts[$i]}" != answered ]; then
            printf "  [%-8s] %-13s %-26s %s\n" "${pris[$i]}" "${sts[$i]}" "${ids[$i]}" "${qs[$i]}"
            shown=$((shown + 1))
        fi
        i=$((i + 1))
    done
done
if [ "$shown" = 0 ]; then printf "  ${DIM}(all questions answered)${RESET}\n"; fi
