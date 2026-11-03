#!/usr/bin/env bash
# scripts/signals-view.sh — the `just dash signals` lens: the typed feedback
# ledger from guidance/signals.yaml
# (lesson | process-debt | product | risk | golden-path).
# Lists signals awaiting disposition first (status open/watch), then settled
# ones, so the cross-stage "what's queued / un-adopted" view is one glance.
# Complements the close-disposition ritual (FIRST_SESSION_PROMPTS.md 1d/1e),
# which is what actually forces the decisions. --json emits a template-native
# signal.* payload (no ContextCore namespace spans these types). Read-only.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/_lib.sh
source "${SCRIPT_DIR}/_lib.sh"

require_initialized
JSON_OUT=$(has_json_flag "$@")

ids=(); tys=(); sts=(); das=(); bars=(); sms=()
while IFS=$'\t' read -r id ty st da bar sm; do
    [ -n "$id" ] || continue
    ids+=("$id"); tys+=("${ty:-?}"); sts+=("${st:-?}")
    das+=("${da:-?}"); bars+=("${bar:-}"); sms+=("$sm")
done < <(emit_signals_tsv)
total=${#ids[@]}
awaiting=0
for s in "${sts[@]:-}"; do case "$s" in open|watch) awaiting=$((awaiting + 1)) ;; esac; done

is_open() { case "$1" in open|watch) return 0 ;; *) return 1 ;; esac; }

if [ "$JSON_OUT" = 1 ]; then
    items=()
    i=0
    while [ "$i" -lt "$total" ]; do
        case "${bars[$i]}" in ''|null) barj=null ;; *) barj=$(json_qs "${bars[$i]}") ;; esac
        items+=("$(json_obj \
            "signal.id" "$(json_qs "${ids[$i]}")" \
            "signal.type" "$(json_qs "${tys[$i]}")" \
            status "$(json_qs "${sts[$i]}")" \
            disposition_at "$(json_qs "${das[$i]}")" \
            "signal.bar" "$barj" \
            "signal.summary" "$(json_qs "${sms[$i]}")")")
        i=$((i + 1))
    done
    [ "${#items[@]}" -gt 0 ] && arr=$(json_arr "${items[@]}") || arr="[]"
    json_emit signals "$(json_obj awaiting "$awaiting" total "$total" signals "$arr")"
    exit 0
fi

printf "${BOLD}Signals (%d awaiting disposition / %d total)${RESET}\n" "$awaiting" "$total"
printf "${DIM}columns: flag · type · status · owner-close · id · summary  (⚠ = open, never dispositioned)${RESET}\n"
if [ "$total" -eq 0 ]; then printf "  ${DIM}(none — registry empty)${RESET}\n"; exit 0; fi

print_row() {
    local i="$1" mark="  "
    [ "${sts[$i]}" = open ] && mark="⚠ "
    printf "  %s%-12s %-10s %-13s %-34s %s\n" \
        "$mark" "${tys[$i]}" "${sts[$i]}" "${das[$i]}" "${ids[$i]}" "${sms[$i]}"
}

# Awaiting disposition first (the ones a close must walk), then settled.
printf "${BOLD}Awaiting disposition${RESET}\n"
shown=0
i=0
while [ "$i" -lt "$total" ]; do is_open "${sts[$i]}" && { print_row "$i"; shown=$((shown + 1)); }; i=$((i + 1)); done
[ "$shown" = 0 ] && printf "  ${DIM}(none — every signal is settled)${RESET}\n"

settled=$((total - awaiting))
if [ "$settled" -gt 0 ]; then
    printf "\n${BOLD}Settled${RESET} ${DIM}(%d)${RESET}\n" "$settled"
    i=0
    while [ "$i" -lt "$total" ]; do is_open "${sts[$i]}" || print_row "$i"; i=$((i + 1)); done
fi
