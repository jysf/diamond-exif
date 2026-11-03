#!/usr/bin/env bash
# scripts/cost-audit.sh — the mechanical backstop for the Cost Tracking
# Discipline (AGENTS.md §4). Fails if any SHIPPED spec is missing real
# build/verify cost data (a positive `tokens_total` on those cycles).
#
# Why this exists: documentation alone tells agents to record cost, and
# documentation alone is skippable — cost tracking silently goes empty
# (all-null numerics) the moment a prompt says "leave it null". A check
# makes it stick. Same pattern as wiring a license policy into CI: a
# discipline made mechanical with a `just` check + a CI job.
#
# Scope: only build/verify cycles are required (those run as metered
# subagents whose token count is in the Agent result). design/ship are
# orchestrator main-loop cycles and may legitimately be null.
#
# Grandfathered pre-process specs are skipped — see
# COST_AUDIT_GRANDFATHERED in scripts/_lib.sh (empty in a fresh template
# instance; populate it only if you adopt the gate after shipping specs
# without it).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/_lib.sh
source "${SCRIPT_DIR}/_lib.sh"

require_initialized
JSON_OUT=$(has_json_flag "$@")

# DEC-005: with no token meter on this platform, tokens_total can't exist, so
# the gate would block on an impossible number. Honor the configured metering
# source — `none` disables the gate (cost is still captured where possible).
METERING=$(get_metering_source)
if [ "$METERING" = none ]; then
    if [ "$JSON_OUT" = 1 ]; then
        json_emit cost-audit "$(json_obj gate "$(json_qs "disabled")" metering_source "$(json_qs "none")" offenders 0 ok true violations "[]")"
        exit 0
    fi
    info "cost-audit: metering_source=none — this platform exposes no token count, so the cost gate is disabled (DEC-005). Capture cost where you can; see docs/cost-tracking.md."
    exit 0
fi

project=$(get_active_project)
project_dir="${REPO_ROOT}/projects/${project}"

offenders=0
warnings=()
off_names=(); off_missing=()
while IFS= read -r f; do
    [ -n "$f" ] || continue
    case "$f" in *-timeline.md) continue ;; esac
    name=$(basename "$f" .md)
    # "shipped" = archived under done/, or front-matter cycle == ship.
    shipped=0
    case "$f" in
        */specs/done/*) shipped=1 ;;
        *) if [ "$(get_spec_cycle "$f")" = "ship" ]; then shipped=1; fi ;;
    esac
    [ "$shipped" = "1" ] || continue
    if is_grandfathered_cost "$name"; then continue; fi
    missing=$(spec_missing_cost_cycles "$f")
    if [ -n "$missing" ]; then
        off_names+=("$name"); off_missing+=("$missing")
        [ "$JSON_OUT" = 1 ] || printf "  %-58s missing cost on: %s\n" "$name" "$missing"
        offenders=$((offenders + 1))
    fi
    imp=$(spec_implausible_cost_cycles "$f")
    [ -n "$imp" ] && warnings+=("${name}: ${imp}")
done < <(find_all_specs "$project_dir")

# Patches (DEC-003) are gated the same way, but their metered cycles are
# `patch verify` (a patch has no separate build cycle).
while IFS= read -r f; do
    [ -n "$f" ] || continue
    case "$f" in *-timeline.md) continue ;; esac
    name=$(basename "$f" .md)
    shipped=0
    case "$f" in
        */patches/done/*) shipped=1 ;;
        *) if [ "$(get_spec_cycle "$f")" = "ship" ]; then shipped=1; fi ;;
    esac
    [ "$shipped" = "1" ] || continue
    if is_grandfathered_cost "$name"; then continue; fi
    missing=$(spec_missing_cost_cycles "$f" patch verify)
    if [ -n "$missing" ]; then
        off_names+=("$name"); off_missing+=("$missing")
        [ "$JSON_OUT" = 1 ] || printf "  %-58s missing cost on: %s\n" "$name" "$missing"
        offenders=$((offenders + 1))
    fi
    imp=$(spec_implausible_cost_cycles "$f" patch verify)
    [ -n "$imp" ] && warnings+=("${name}: ${imp}")
done < <(find_all_patches "$project_dir")

# Machine-readable findings (DEC-001 §2) — which artifact is missing which
# metered cycle, so an agent can fix it rather than screen-scrape.
if [ "$JSON_OUT" = 1 ]; then
    items=()
    i=0
    while [ "$i" -lt "${#off_names[@]}" ]; do
        parts=(); for c in ${off_missing[$i]}; do parts+=("$(json_qs "$c")"); done
        [ "${#parts[@]}" -gt 0 ] && marr=$(json_arr "${parts[@]}") || marr="[]"
        items+=("$(json_obj artifact "$(json_qs "${off_names[$i]}")" missing_cost "$marr")")
        i=$((i + 1))
    done
    [ "${#items[@]}" -gt 0 ] && arr=$(json_arr "${items[@]}") || arr="[]"
    witems=()
    for w in ${warnings[@]+"${warnings[@]}"}; do witems+=("$(json_qs "$w")"); done
    [ "${#witems[@]}" -gt 0 ] && warr=$(json_arr "${witems[@]}") || warr="[]"
    json_emit cost-audit "$(json_obj \
        gate "$(json_qs "enforced")" metering_source "$(json_qs "$METERING")" \
        offenders "$offenders" ok "$([ "$offenders" -eq 0 ] && echo true || echo false)" \
        violations "$arr" implausible "$warr")"
    [ "$offenders" -gt 0 ] && exit 1
    exit 0
fi

# Advisory (does NOT fail the gate): implausibly-low metered cost is a strong
# hint that sub-agent metering was truncated (e.g. a session-limit deflated
# subagent_tokens) — the number passes the non-null gate but silently
# undercounts. Surface it so cost/value rollups aren't quietly wrong (#5).
if [ "${#warnings[@]}" -gt 0 ]; then
    echo ""
    warn "cost-audit: implausibly-low metered cost (< ${COST_IMPLAUSIBLE_FLOOR} tokens) — sub-agent metering may have been truncated; verify these numbers:"
    for w in ${warnings[@]+"${warnings[@]}"}; do
        echo "    ${w}"
    done
fi

if [ "$offenders" -gt 0 ]; then
    echo ""
    die "cost-audit: ${offenders} shipped spec(s)/patch(es) missing metered-cycle cost. Record tokens_total per AGENTS.md §4 / docs/cost-tracking.md."
fi
success "cost-audit: all shipped specs and patches have their metered-cycle cost recorded."
