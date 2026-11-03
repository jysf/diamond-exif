#!/usr/bin/env bash
# scripts/calibration.sh — expected-vs-actual: how good are your estimates?
#
# For every SHIPPED spec, compares what you predicted against what it took:
#   size    task.complexity (expected, set at design)  vs  task.complexity_actual
#   tokens  cost.tokens_estimate (optional)            vs  cost.totals.tokens_total
#
# WARN-ONLY BY DESIGN. This never gates anything and never scolds you for an
# unrecorded prediction — the value is the feedback loop (am I systematically
# under- or over-estimating?), not the individual number.
#
# It also prints the OBSERVED token band per expected size — the honest,
# per-repo version of "map t-shirt sizes to token estimates". The band is
# measured from your own shipped specs rather than guessed, so once it's stable
# the size you assign IS a rough token estimate.
#
# Usage:
#   just calibration                 # active project
#   just calibration --all           # every project
#   just calibration PROJ-002        # a specific project
#   just calibration --json

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

# One row per shipped spec: proj|id|title|expected|actual|verdict|est_tokens|act_tokens
ROWS=()
for proj in "${PROJECTS[@]}"; do
    pdir="${REPO_ROOT}/projects/${proj}"
    [ -d "$pdir" ] || continue
    while IFS= read -r f; do
        [ -f "$f" ] || continue
        case "$f" in *-timeline.md|*/prompts/*) continue ;; esac
        # Shipped = archived under done/, or still in place at cycle: ship.
        case "$f" in
            */done/*) : ;;
            *) [ "$(get_spec_cycle "$f")" = "ship" ] || continue ;;
        esac
        id=$(basename "$f" | sed -E 's/^(SPEC-[0-9]+).*/\1/')
        exp=$(get_spec_complexity "$f")
        act=$(get_spec_complexity_actual "$f")
        er=$(size_rank "${exp:-?}"); ar=$(size_rank "${act:-?}")
        if [ "$er" = "0" ] || [ "$ar" = "0" ]; then verdict="unrecorded"
        elif [ "$ar" -gt "$er" ]; then verdict="under"     # took more than expected
        elif [ "$ar" -lt "$er" ]; then verdict="over"      # expected more than it took
        else verdict="on"
        fi
        est=$(get_tokens_estimate "$f")
        actual_tok=$(sum_cost_tokens_for_spec "$f")
        ROWS+=("${proj}|${id}|$(get_spec_title "$f")|${exp:-?}|${act:-?}|${verdict}|${est:-}|${actual_tok:-0}")
    done < <(find_all_specs "$pdir")
done

TOTAL=${#ROWS[@]}
if [ "$TOTAL" = "0" ] && [ "$JSON_OUT" = "0" ]; then
    echo "${DIM}No shipped specs yet — nothing to calibrate against.${RESET}"
    echo "Calibration needs history: it compares what you predicted at design"
    echo "with what shipping actually cost."
    exit 0
fi

# Tally the size verdicts, and collect actual tokens per EXPECTED size so the
# observed band can be printed. bash 3.2 has no associative arrays — a flat
# "SIZE:tok SIZE:tok" string is enough at these volumes.
under=0; over=0; on=0; unrecorded=0
BAND_DATA=""
tok_under=0; tok_over=0; tok_on=0
for row in ${ROWS[@]+"${ROWS[@]}"}; do
    IFS='|' read -r _p _id _t exp _act verdict est atok <<EOF
$row
EOF
    case "$verdict" in
        under) under=$((under + 1)) ;; over) over=$((over + 1)) ;;
        on) on=$((on + 1)) ;; *) unrecorded=$((unrecorded + 1)) ;;
    esac
    if [ "$(size_rank "$exp")" != "0" ] && [ "${atok:-0}" -gt 0 ]; then
        BAND_DATA="${BAND_DATA}${exp}:${atok} "
    fi
    # Token estimate accuracy: within ±25% counts as on-target.
    if [ -n "$est" ] && [ "$est" -gt 0 ] && [ "${atok:-0}" -gt 0 ]; then
        if   [ $((atok * 100)) -gt $((est * 125)) ]; then tok_under=$((tok_under + 1))
        elif [ $((atok * 100)) -lt $((est * 75))  ]; then tok_over=$((tok_over + 1))
        else tok_on=$((tok_on + 1))
        fi
    fi
done

# min|median|max|n of the actual tokens recorded for one expected size.
band_for() {
    local size="$1" vals n
    vals=$(printf '%s\n' $BAND_DATA | awk -F: -v s="$size" '$1 == s { print $2 }' | sort -n)
    [ -n "$vals" ] || return 1
    n=$(printf '%s\n' "$vals" | wc -l | tr -d ' ')
    printf '%s|%s|%s|%s' \
        "$(printf '%s\n' "$vals" | head -n1)" \
        "$(printf '%s\n' "$vals" | awk -v n="$n" 'NR == int((n+1)/2) { print; exit }')" \
        "$(printf '%s\n' "$vals" | tail -n1)" \
        "$n"
}

if [ "$JSON_OUT" = "1" ]; then
    specs_json=""
    for row in ${ROWS[@]+"${ROWS[@]}"}; do
        IFS='|' read -r p id t exp act verdict est atok <<EOF
$row
EOF
        [ -z "$specs_json" ] || specs_json="${specs_json},"
        specs_json="${specs_json}$(json_obj \
            project "$(json_qs "$p")" id "$(json_qs "$id")" title "$(json_qs "$t")" \
            expected_size "$(json_qs "$exp")" actual_size "$(json_qs "$act")" \
            size_verdict "$(json_qs "$verdict")" \
            tokens_estimate "${est:-null}" tokens_actual "${atok:-0}")"
    done
    bands_json=""
    for s in XS S M L XL XXL; do
        b=$(band_for "$s") || continue
        IFS='|' read -r bmin bmed bmax bn <<EOF
$b
EOF
        [ -z "$bands_json" ] || bands_json="${bands_json},"
        bands_json="${bands_json}$(json_obj size "$(json_qs "$s")" \
            n "$bn" tokens_min "$bmin" tokens_median "$bmed" tokens_max "$bmax")"
    done
    json_emit calibration "$(json_obj \
        scope "$(json_qs "$SCOPE")" \
        shipped_specs "$TOTAL" \
        size_drift "$(json_obj under "$under" over "$over" on_target "$on" unrecorded "$unrecorded")" \
        token_drift "$(json_obj under "$tok_under" over "$tok_over" on_target "$tok_on")" \
        observed_token_bands "[${bands_json}]" \
        specs "[${specs_json}]")"
    exit 0
fi

echo "${BOLD}Calibration — expected vs actual${RESET} ${DIM}(${TOTAL} shipped spec(s))${RESET}"
echo ""
printf "  ${DIM}%-10s %-5s %-7s %-8s %10s %11s${RESET}\n" \
    "SPEC" "EXP" "ACTUAL" "VERDICT" "EST TOK" "ACTUAL TOK"
for row in ${ROWS[@]+"${ROWS[@]}"}; do
    IFS='|' read -r _p id title exp act verdict est atok <<EOF
$row
EOF
    # Pad the label BEFORE colouring it — printf widths count escape bytes.
    case "$verdict" in
        under) mark="${YELLOW}$(printf '%-8s' bigger)${RESET}" ;;
        over)  mark="${YELLOW}$(printf '%-8s' smaller)${RESET}" ;;
        on)    mark="${GREEN}$(printf '%-8s' on)${RESET}" ;;
        *)     mark="${DIM}$(printf '%-8s' 'not set')${RESET}" ;;
    esac
    printf "  %-10s %-5s %-7s %s %10s %11s  ${DIM}%s${RESET}\n" \
        "$id" "$exp" "$act" "$mark" "${est:--}" "${atok:-0}" "$title"
done

echo ""
echo "${BOLD}Size drift:${RESET} ${under} under-estimated · ${over} over-estimated · ${on} on target"
[ "$unrecorded" = "0" ] || echo "${DIM}${unrecorded} spec(s) have no recorded actual size — stamp task.complexity_actual at ship.${RESET}"
if [ $((tok_under + tok_over + tok_on)) -gt 0 ]; then
    echo "${BOLD}Token drift:${RESET} ${tok_under} under · ${tok_over} over · ${tok_on} within ±25%"
fi

# The observed band is the payoff: your own history turning a size into a
# rough token estimate. Printed only where there's data — never a guessed table.
BANDS_SHOWN=0
for s in XS S M L XL XXL; do
    b=$(band_for "$s") || continue
    if [ "$BANDS_SHOWN" = "0" ]; then
        echo ""
        echo "${BOLD}Observed token band per expected size${RESET} ${DIM}(from this repo's shipped specs)${RESET}"
        BANDS_SHOWN=1
    fi
    IFS='|' read -r bmin bmed bmax bn <<EOF
$b
EOF
    printf "  %-4s %8s – %-8s ${DIM}median %s  (n=%s)${RESET}\n" "$s" "$bmin" "$bmax" "$bmed" "$bn"
done
if [ "$BANDS_SHOWN" = "1" ]; then
    echo "${DIM}Treat a band as a real estimate only once n is big enough to trust —"
    echo "it's your repo's history, not a rule.${RESET}"
fi
