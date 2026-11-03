#!/usr/bin/env bash
# scripts/patches-view.sh — the `just dash patches` lens: the patch lane (DEC-003).
# Lists the active project's patches grouped by cycle (patch|verify|ship), with a
# shipped patch flagged when it's missing its metered (patch+verify) cost. --json
# emits the same task.*/cost.* attribute names as status's patches[]. Read-only.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/_lib.sh
source "${SCRIPT_DIR}/_lib.sh"

require_initialized
JSON_OUT=$(has_json_flag "$@")

project=$(get_active_project)
pdir="${REPO_ROOT}/projects/${project}"

ids=(); cycs=(); shipd=(); usds=(); toks=(); miss=()
while IFS= read -r f; do
    [ -n "$f" ] || continue
    case "$f" in *-timeline.md|*/prompts/*) continue ;; esac
    pid=$(basename "$f" | sed -E 's/^(PATCH-[0-9]+).*/\1/')
    cyc=$(get_spec_cycle "$f"); [ -n "$cyc" ] || cyc="?"
    t=$(sum_cost_tokens_for_spec "$f"); u=$(sum_cost_usd_for_spec "$f")
    case "$f" in */patches/done/*) sh=true ;; *) if [ "$cyc" = ship ]; then sh=true; else sh=false; fi ;; esac
    m=""
    if [ "$sh" = true ] && ! is_grandfathered_cost "$(basename "$f" .md)"; then
        m=$(spec_missing_cost_cycles "$f" patch verify)
    fi
    ids+=("$pid"); cycs+=("$cyc"); shipd+=("$sh"); usds+=("$u"); toks+=("$t"); miss+=("$m")
done < <(find_all_patches "$pdir")
total=${#ids[@]}

if [ "$JSON_OUT" = 1 ]; then
    items=()
    i=0
    while [ "$i" -lt "$total" ]; do
        if [ -n "${miss[$i]}" ]; then
            parts=(); for c in ${miss[$i]}; do parts+=("$(json_qs "$c")"); done
            mj=$(json_arr "${parts[@]}")
        else
            mj="[]"
        fi
        items+=("$(json_obj \
            "task.id" "$(json_qs "${ids[$i]}")" \
            "task.cycle" "$(json_qs "${cycs[$i]}")" \
            shipped "${shipd[$i]}" \
            "cost.tokens_total" "${toks[$i]}" \
            "cost.estimated_usd" "${usds[$i]}" \
            missing_cost "$mj")")
        i=$((i + 1))
    done
    [ "${#items[@]}" -gt 0 ] && arr=$(json_arr "${items[@]}") || arr="[]"
    json_emit patches "$(json_obj total "$total" patches "$arr")"
    exit 0
fi

printf "${BOLD}Patches — %s (%d)${RESET}\n" "$project" "$total"
printf "${DIM}columns: id · cost · flag  (⚠ = shipped, missing metered patch+verify cost)${RESET}\n"
if [ "$total" -eq 0 ]; then
    printf "  ${DIM}(none — no patches in this project; scaffold one with just new-patch)${RESET}\n"
    exit 0
fi
for cycle in patch verify ship "?"; do
    shown=0
    i=0
    while [ "$i" -lt "$total" ]; do
        # The "?" pass catches any cycle value outside the known set.
        match=0
        if [ "$cycle" = "?" ]; then
            case "${cycs[$i]}" in patch|verify|ship) : ;; *) match=1 ;; esac
        else
            [ "${cycs[$i]}" = "$cycle" ] && match=1
        fi
        if [ "$match" = 1 ]; then
            [ "$shown" = 0 ] && printf "  ${BOLD}%-8s${RESET}\n" "$cycle"
            flag="  "; [ -n "${miss[$i]}" ] && flag="⚠ "
            printf "    %s%-28s  \$%-7s  %s\n" "$flag" "${ids[$i]}" "${usds[$i]}" "${miss[$i]:+missing cost: ${miss[$i]}}"
            shown=$((shown + 1))
        fi
        i=$((i + 1))
    done
done
