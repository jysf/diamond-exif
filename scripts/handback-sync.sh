#!/usr/bin/env bash
# scripts/handback-sync.sh — transcribe agent-reported cost from handoffs into
# the spec's cost.sessions (claude-plus-agents; DEC-002 / DEC-005).
#
# The problem this solves: with build and verify delegated to external agents,
# the orchestrator has no meter of its own for those cycles. Rather than the
# orchestrator estimating (which corrupts the cost record), the executing agent
# self-reports into its handoff's `handback:` block and this transcribes it.
#
# Idempotent twice over. A handback is stamped `synced_at` once transcribed and
# skipped afterwards; and before appending, the spec's own cost.sessions is
# checked, so a cycle already on the ledger is never appended again — whether an
# earlier sync wrote it (matched by `handoff:` id) or the agent wrote it by hand
# (same cycle + same tokens_total). The stamp alone let one cycle land twice when
# the agent and the script both recorded it (irradiance `cost-field-has-two-owners`).
#
# Usage: handback-sync.sh SPEC-NNN [--json] [--dry-run]
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/_lib.sh
source "${SCRIPT_DIR}/_lib.sh"

require_initialized

SPEC_ID=""
DRY_RUN=0
for a in "$@"; do
    case "$a" in
        --dry-run) DRY_RUN=1 ;;
        --json) : ;;   # handled via has_json_flag
        SPEC-*) SPEC_ID="$a" ;;
        "") : ;;
        *) die "Unknown argument: $a
Usage: just handback-sync SPEC-NNN [--json] [--dry-run]" ;;
    esac
done
JSON_OUT=$(has_json_flag "$@")

[ -n "$SPEC_ID" ] || die "Usage: just handback-sync SPEC-NNN [--json] [--dry-run]"

SPEC_FILE=$(find_spec "$SPEC_ID")
[ -n "$SPEC_FILE" ] || die "Spec not found: ${SPEC_ID}"

PROJECT_DIR=$(dirname "$(dirname "$SPEC_FILE")")
HANDOFF_DIR="${PROJECT_DIR}/handoffs"

sed_inplace() {
    if [ "$(uname)" = "Darwin" ]; then sed -i '' "$@"; else sed -i "$@"; fi
}

# Is this handoff's cycle already on the spec's cost.sessions ledger?
# Prints `handoff` (an entry names this handoff id), `duplicate` (an entry has
# the same cycle and the same numeric tokens_total), or nothing.
session_already_recorded() {
    local spec="$1" hid="$2" cyc="$3" tok="$4"
    awk -v hid="$hid" -v cyc="$cyc" -v tok="$tok" '
        function check() {
            if (ec == "") return
            if (eh == hid) { print "handoff"; found = 1; exit }
            if (ec == cyc && tok ~ /^[0-9]+$/ && et == tok) { print "duplicate"; found = 1; exit }
        }
        function val(s) { sub(/^[^:]*:[[:space:]]*/, "", s); sub(/[[:space:]]+#.*$/, "", s); gsub(/"/, "", s); return s }
        /^---$/ { if (fm) { check(); exit } fm = 1; next }
        !fm { exit }
        /^cost:/ { inc = 1; next }
        inc && /^[^[:space:]]/ { check(); inc = 0 }
        inc && /^  [a-z_]+:/ && !/^  sessions:/ { check(); ec = ""; ins = 0; next }
        inc && /^  sessions:/ { ins = 1; next }
        ins && /^    - / { check(); ec = ""; et = ""; eh = "" }
        ins && /^    (- |  )cycle:/        { ec = val($0) }
        ins && /^      tokens_total:/      { et = val($0) }
        ins && /^      handoff:/           { eh = val($0) }
    ' "$spec"
}

synced=0; skipped=0; recorded=0; pending=0; superseded=0
sync_ids=(); sync_cycles=(); sync_tokens=(); pending_ids=(); pending_why=()

if [ -d "$HANDOFF_DIR" ]; then
    while IFS= read -r hf; do
        [ -n "$hf" ] || continue
        # Only handoffs belonging to this spec.
        hspec=$(awk '/^---$/{f=!f; next} f && /^[[:space:]]+spec_id:/{print $2; exit}' "$hf")
        [ "$hspec" = "$SPEC_ID" ] || continue

        # A superseded handoff was never dispatched (SPEC-016's L→S rescope
        # retired HANDOFF-038 this way): no cycle ran, so there is no cost to
        # transcribe and it is not "pending". Counting it pending trips the
        # gate's `exit 1` on a non-failure — a gate dying for a non-failure is
        # the mute-death shape this spec closes (SPEC-023). Skip it before the
        # handback fields are read, tallied SEPARATELY from "already synced":
        # claiming a never-dispatched handoff was synced is its own small lie.
        if [ "$(get_handoff_field "$hf" status)" = "superseded" ]; then
            superseded=$((superseded + 1)); continue
        fi

        # `|| true`: under pipefail a zero-match grep inside $() kills the script.
        hid=$(basename "$hf" .md | grep -oE 'HANDOFF-[0-9]+' || true)
        hcycle=$(get_handoff_field "$hf" cycle)
        [ -n "$hcycle" ] || hcycle="build"
        hstatus=$(get_handback_field "$hf" status)
        htok=$(get_handback_field "$hf" tokens_total)
        husd=$(get_handback_field "$hf" estimated_usd)
        hmin=$(get_handback_field "$hf" duration_minutes)
        hnotes=$(get_handback_field "$hf" notes)
        hbasis=$(get_handback_field "$hf" basis)
        # The model that actually ran, when the agent reported it. to_agent is
        # the tier_map PREDICTION, wrong for irradiance's build 6 times in 7.
        hagent=$(get_handback_field "$hf" agent)
        [ -n "$hagent" ] && [ "$hagent" != "null" ] || hagent=$(get_handoff_field "$hf" to_agent)
        hsynced=$(get_handback_field "$hf" synced_at)

        # Already transcribed → skip (idempotence).
        if [ -n "$hsynced" ] && [ "$hsynced" != "null" ]; then
            skipped=$((skipped + 1)); continue
        fi
        # Not handed back yet.
        if [ -z "$hstatus" ] || [ "$hstatus" = "null" ]; then
            pending=$((pending + 1))
            pending_ids+=("$hid"); pending_why+=("handback.status not set — agent has not reported")
            continue
        fi
        # Handed back with no token count. Legitimate only when the platform has
        # no meter — which is what metering_source: none declares.
        if [ -z "$htok" ] || [ "$htok" = "null" ]; then
            if [ "$(get_metering_source)" = none ]; then
                : # allowed — sync the session with null numerics + the note
            else
                pending=$((pending + 1))
                pending_ids+=("$hid")
                pending_why+=("handback.status=${hstatus} but tokens_total is null — ask the agent for its real count, or set cost.metering_source: none")
                continue
            fi
        fi

        # Already on the ledger → stamp the handoff so it isn't re-examined,
        # but never append a second entry for the same cycle.
        already=$(session_already_recorded "$SPEC_FILE" "$hid" "$hcycle" "${htok:-null}")
        if [ -n "$already" ]; then
            recorded=$((recorded + 1))
            [ "$DRY_RUN" = 0 ] && sed_inplace "s|^  synced_at: null.*|  synced_at: $(today)|" "$hf"
            continue
        fi

        sync_ids+=("$hid"); sync_cycles+=("$hcycle"); sync_tokens+=("${htok:-null}")

        if [ "$DRY_RUN" = 0 ]; then
            note="${hnotes:-null}"
            [ "$note" = "null" ] && note="transcribed from ${hid} by handback-sync"
            # Append the session entry to cost.sessions (end of the sessions list,
            # immediately before `  totals:`), preserving 4/6-space indentation.
            # `notes` is free text, so it is always written as a double-quoted
            # YAML scalar with \ " newline and tab escaped: a raw copy of a
            # multi-line or `#`-bearing note left the spec unparseable. It goes
            # through ENVIRON, not -v, because -v would expand its backslashes.
            HB_NOTE="$note" awk -v cyc="$hcycle" -v agent="$hagent" -v basis="${hbasis:-}" \
                -v hid="$hid" -v tok="${htok:-null}" -v usd="${husd:-null}" \
                -v mins="${hmin:-null}" -v day="$(today)" '
                function yq(s,   out, i, c) {
                    out = ""
                    for (i = 1; i <= length(s); i++) {
                        c = substr(s, i, 1)
                        if (c == "\\") out = out "\\\\"
                        else if (c == "\"") out = out "\\\""
                        else if (c == "\n") out = out "\\n"
                        else if (c == "\t") out = out "\\t"
                        else out = out c
                    }
                    return "\"" out "\""
                }
                /^---$/ { fm = !fm; print; next }
                fm && /^cost:/ { in_cost = 1 }
                fm && in_cost && /^  totals:/ && !done {
                    print "    - cycle: " cyc
                    print "      handoff: " hid
                    print "      agent: " agent
                    print "      interface: other"
                    if (basis != "" && basis != "null") print "      basis: " basis
                    print "      tokens_total: " tok
                    print "      estimated_usd: " usd
                    print "      duration_minutes: " mins
                    print "      recorded_at: " day
                    print "      notes: " yq(ENVIRON["HB_NOTE"])
                    done = 1
                }
                { print }
            ' "$SPEC_FILE" > "${SPEC_FILE}.tmp" && mv "${SPEC_FILE}.tmp" "$SPEC_FILE"
            # `sessions: []` becomes a real list once an entry is added.
            sed_inplace 's|^  sessions: \[\]$|  sessions:|' "$SPEC_FILE"
            sed_inplace "s|^  synced_at: null.*|  synced_at: $(today)|" "$hf"
        fi
        synced=$((synced + 1))
    done < <(find "$HANDOFF_DIR" -type f -name 'HANDOFF-*.md' 2>/dev/null | sort)
fi

[ "$DRY_RUN" = 0 ] && [ "$synced" -gt 0 ] && write_cost_totals "$SPEC_FILE"

if [ "$JSON_OUT" = 1 ]; then
    items=()
    i=0
    while [ "$i" -lt "${#sync_ids[@]}" ]; do
        items+=("$(json_obj \
            "handoff.id" "$(json_qs "${sync_ids[$i]}")" \
            "handoff.cycle" "$(json_qs "${sync_cycles[$i]}")" \
            "cost.tokens_total" "$(json_qs "${sync_tokens[$i]}")")")
        i=$((i + 1))
    done
    [ "${#items[@]}" -gt 0 ] && sarr=$(json_arr "${items[@]}") || sarr="[]"
    pitems=()
    i=0
    while [ "$i" -lt "${#pending_ids[@]}" ]; do
        pitems+=("$(json_obj "handoff.id" "$(json_qs "${pending_ids[$i]}")" reason "$(json_qs "${pending_why[$i]}")")")
        i=$((i + 1))
    done
    [ "${#pitems[@]}" -gt 0 ] && parr=$(json_arr "${pitems[@]}") || parr="[]"
    json_emit handback-sync "$(json_obj \
        "task.id" "$(json_qs "$SPEC_ID")" \
        dry_run "$([ "$DRY_RUN" = 1 ] && echo true || echo false)" \
        synced "$synced" already_synced "$skipped" already_recorded "$recorded" superseded "$superseded" pending "$pending" \\
        transcribed "$sarr" awaiting "$parr")"
    [ "$pending" -gt 0 ] && exit 1
    exit 0
fi

prefix=""
[ "$DRY_RUN" = 1 ] && prefix="[dry-run] "
if [ "$synced" -gt 0 ]; then
    i=0
    while [ "$i" -lt "${#sync_ids[@]}" ]; do
        printf "  %s%s → cost.sessions[%s]  tokens=%s\n" \
            "$prefix" "${sync_ids[$i]}" "${sync_cycles[$i]}" "${sync_tokens[$i]}"
        i=$((i + 1))
    done
fi
[ "$skipped" -gt 0 ] && printf "  ${DIM}%d handback(s) already synced — skipped${RESET}\n" "$skipped"
[ "$superseded" -gt 0 ] && printf "  ${DIM}%d handoff(s) superseded (never dispatched) — skipped${RESET}\n" "$superseded"
[ "$recorded" -gt 0 ] && printf "  ${DIM}%d handback(s) already on cost.sessions — not appended again${RESET}\n" "$recorded"

if [ "$pending" -gt 0 ]; then
    echo ""
    warn "${pending} handoff(s) for ${SPEC_ID} have not handed back cleanly:"
    i=0
    while [ "$i" -lt "${#pending_ids[@]}" ]; do
        printf "    %s — %s\n" "${pending_ids[$i]}" "${pending_why[$i]}"
        i=$((i + 1))
    done
    echo ""
    echo "${DIM}The agent that ran the cycle is the only party that knows its token count.${RESET}"
    echo "${DIM}Don't estimate on its behalf — ask it, or declare the platform unmetered.${RESET}"
    exit 1
fi

if [ "$synced" -eq 0 ] && [ "$skipped" -eq 0 ] && [ "$recorded" -eq 0 ] && [ "$superseded" -eq 0 ]; then
    echo "  ${DIM}(no handoffs found for ${SPEC_ID})${RESET}"
else
    success "${prefix}handback-sync: ${synced} transcribed, ${skipped} already synced, ${recorded} already recorded, ${superseded} superseded."
fi
