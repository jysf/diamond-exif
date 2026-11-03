#!/usr/bin/env bash
# scripts/spikes-view.sh — the `just dash spikes` lens: the spike lane (DEC-012).
#
# Lists spikes grouped by cycle (spike|land), open first then archived, and flags
# the one state that matters: a spike at `cycle: land` with no `spike.outcome` —
# an un-landed spike, which is the exact failure the lane exists to prevent.
#
# REPO-level, not project-scoped (a spike may precede any project), which is the
# structural difference from the patches lens. Read-only. --json emits the same
# task.*/spike.* attribute names as status's spikes[].
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/_lib.sh
source "${SCRIPT_DIR}/_lib.sh"

require_initialized
JSON_OUT=$(has_json_flag "$@")

ids=(); cycs=(); modes=(); tbs=(); outs=(); archived=(); stale=()
while IFS= read -r f; do
    [ -n "$f" ] || continue
    case "$f" in */prompts/*) continue ;; esac
    sid=$(basename "$f" | sed -E 's/^(SPIKE-[0-9]+).*/\1/')
    cyc=$(get_spec_cycle "$f"); [ -n "$cyc" ] || cyc="?"
    md=$(get_spike_field "$f" mode);    [ -n "$md" ]  || md="?"
    tb=$(get_spike_field "$f" timebox); [ -n "$tb" ]  || tb="?"
    out=$(get_spike_field "$f" outcome)
    [ -n "$out" ] || out="null"
    case "$f" in */done/*) arc=true ;; *) arc=false ;; esac
    # The un-landed flag: at `land` but carrying no real outcome.
    st=false
    if [ "$cyc" = "land" ] && [ "$out" = "null" ]; then st=true; fi
    ids+=("$sid"); cycs+=("$cyc"); modes+=("$md"); tbs+=("$tb")
    outs+=("$out"); archived+=("$arc"); stale+=("$st")
done < <(find_all_spikes)
total=${#ids[@]}

if [ "$JSON_OUT" = 1 ]; then
    items=()
    i=0
    while [ "$i" -lt "$total" ]; do
        items+=("$(json_obj \
            "task.id" "$(json_qs "${ids[$i]}")" \
            "task.cycle" "$(json_qs "${cycs[$i]}")" \
            "spike.mode" "$(json_qs "${modes[$i]}")" \
            "spike.timebox" "$(json_qs "${tbs[$i]}")" \
            "spike.outcome" "$(json_qs "${outs[$i]}")" \
            archived "${archived[$i]}" \
            unlanded "${stale[$i]}")")
        i=$((i + 1))
    done
    [ "${#items[@]}" -gt 0 ] && arr=$(json_arr "${items[@]}") || arr="[]"
    json_emit spikes "$(json_obj total "$total" spikes "$arr")"
    exit 0
fi

printf "${BOLD}Spikes — bounded exploration (%d)${RESET}\n" "$total"
printf "${DIM}columns: id · mode · timebox · outcome  (⚠ = landed with no outcome)${RESET}\n"
if [ "$total" -eq 0 ]; then
    printf "  ${DIM}(none — scaffold one with just new-spike \"the question\" \"1d\" [question|build])${RESET}\n"
    exit 0
fi

for cycle in spike land "?"; do
    shown=0
    i=0
    while [ "$i" -lt "$total" ]; do
        match=0
        if [ "$cycle" = "?" ]; then
            case "${cycs[$i]}" in spike|land) : ;; *) match=1 ;; esac
        else
            [ "${cycs[$i]}" = "$cycle" ] && match=1
        fi
        if [ "$match" = 1 ]; then
            [ "$shown" = 0 ] && printf "  ${BOLD}%-8s${RESET}\n" "$cycle"
            flag="  "; [ "${stale[$i]}" = true ] && flag="⚠ "
            note=""
            [ "${archived[$i]}" = true ] && note="${DIM}(archived)${RESET}"
            [ "${stale[$i]}" = true ] && note="${YELLOW}un-landed — set spike.outcome${RESET}"
            printf "    %s%-16s  %-9s  %-12s  %-12s %b\n" \
                "$flag" "${ids[$i]}" "${modes[$i]}" "${tbs[$i]}" "${outs[$i]}" "$note"
            shown=$((shown + 1))
        fi
        i=$((i + 1))
    done
done
