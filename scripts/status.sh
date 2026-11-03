#!/usr/bin/env bash
# scripts/status.sh — print repo state report.

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/_lib.sh"

require_initialized

VARIANT=$(get_variant)
ACTIVE_PROJECT=$(get_active_project)
ACTIVE_PROJECT_DIR="${REPO_ROOT}/projects/${ACTIVE_PROJECT}"

# --- JSON output (DEC-001 §2) ------------------------------------------------
if [ "$(has_json_flag "$@")" = 1 ]; then
    specs_json=(); missing_specs=()
    while IFS= read -r f; do
        [ -n "$f" ] || continue
        case "$f" in *-timeline.md|*/prompts/*) continue ;; esac
        sid=$(basename "$f" | sed -E 's/^(SPEC-[0-9]+).*/\1/')
        name=$(basename "$f" .md)
        cyc=$(get_spec_cycle "$f"); [ -n "$cyc" ] || cyc="?"
        typ=$(get_spec_type "$f"); [ -n "$typ" ] || typ="story"
        u=$(sum_cost_usd_for_spec "$f"); t=$(sum_cost_tokens_for_spec "$f")
        case "$f" in
            */specs/done/*) shipped=true ;;
            *) if [ "$cyc" = ship ]; then shipped=true; else shipped=false; fi ;;
        esac
        mc_json="[]"
        if [ "$shipped" = true ] && ! is_grandfathered_cost "$name"; then
            m=$(spec_missing_cost_cycles "$f")
            if [ -n "$m" ]; then
                parts=(); for c in $m; do parts+=("$(json_qs "$c")"); done
                mc_json=$(json_arr "${parts[@]}")
                missing_specs+=("$(json_qs "$sid")")
            fi
        fi
        specs_json+=("$(json_obj \
            "task.id" "$(json_qs "$sid")" \
            "task.type" "$(json_qs "$typ")" \
            "task.cycle" "$(json_qs "$cyc")" \
            shipped "$shipped" \
            "cost.tokens_total" "$t" \
            "cost.estimated_usd" "$u" \
            missing_cost "$mc_json")")
    done < <(find_all_specs "$ACTIVE_PROJECT_DIR")
    [ "${#specs_json[@]}" -gt 0 ] && specs_arr=$(json_arr "${specs_json[@]}") || specs_arr="[]"
    [ "${#missing_specs[@]}" -gt 0 ] && missing_arr=$(json_arr "${missing_specs[@]}") || missing_arr="[]"
    # `|| true`: a bare/new active project may have no specs/ dir yet — find then
    # exits non-zero and `pipefail` would abort. wc still prints 0, so counts are
    # correct; we just don't want the non-zero to kill the report.
    total_specs=$(find "${ACTIVE_PROJECT_DIR}/specs" -name "SPEC-*.md" 2>/dev/null | awk '!/-timeline\.md/ && !/\/prompts\//' | wc -l | tr -d ' ' || true)
    shipped_specs=$(find "${ACTIVE_PROJECT_DIR}/specs/done" -name "SPEC-*.md" 2>/dev/null | awk '!/-timeline\.md/' | wc -l | tr -d ' ' || true)
    total_decisions=$(find "${REPO_ROOT}/decisions" -name "DEC-*.md" 2>/dev/null | wc -l | tr -d ' ' || true)
    # Patches (the patch lane, DEC-003) — same task.* attribute names as specs.
    patches_json=()
    while IFS= read -r f; do
        [ -n "$f" ] || continue
        case "$f" in *-timeline.md|*/prompts/*) continue ;; esac
        pid=$(basename "$f" | sed -E 's/^(PATCH-[0-9]+).*/\1/')
        pname=$(basename "$f" .md)
        cyc=$(get_spec_cycle "$f"); [ -n "$cyc" ] || cyc="?"
        u=$(sum_cost_usd_for_spec "$f"); t=$(sum_cost_tokens_for_spec "$f")
        case "$f" in
            */patches/done/*) pshipped=true ;;
            *) if [ "$cyc" = ship ]; then pshipped=true; else pshipped=false; fi ;;
        esac
        pmc_json="[]"
        if [ "$pshipped" = true ] && ! is_grandfathered_cost "$pname"; then
            m=$(spec_missing_cost_cycles "$f" patch verify)
            if [ -n "$m" ]; then
                parts=(); for c in $m; do parts+=("$(json_qs "$c")"); done
                pmc_json=$(json_arr "${parts[@]}")
            fi
        fi
        patches_json+=("$(json_obj \
            "task.id" "$(json_qs "$pid")" \
            "task.cycle" "$(json_qs "$cyc")" \
            shipped "$pshipped" \
            "cost.tokens_total" "$t" \
            "cost.estimated_usd" "$u" \
            missing_cost "$pmc_json")")
    done < <(find_all_patches "$ACTIVE_PROJECT_DIR")
    [ "${#patches_json[@]}" -gt 0 ] && patches_arr=$(json_arr "${patches_json[@]}") || patches_arr="[]"

    # Spikes (the bounded-exploration lane, DEC-012) — REPO-level, so this is not
    # scoped to the active project. Only un-landed spikes are interesting here.
    spikes_json=()
    while IFS= read -r f; do
        [ -n "$f" ] || continue
        case "$f" in */done/*|*/prompts/*) continue ;; esac
        sid=$(basename "$f" | sed -E 's/^(SPIKE-[0-9]+).*/\1/')
        cyc=$(get_spec_cycle "$f"); [ -n "$cyc" ] || cyc="?"
        sout=$(get_spike_field "$f" outcome); [ -n "$sout" ] || sout="null"
        spikes_json+=("$(json_obj \
            "task.id" "$(json_qs "$sid")" \
            "task.cycle" "$(json_qs "$cyc")" \
            "spike.mode" "$(json_qs "$(get_spike_field "$f" mode)")" \
            "spike.timebox" "$(json_qs "$(get_spike_field "$f" timebox)")" \
            "spike.outcome" "$(json_qs "$sout")")")
    done < <(find_all_spikes)
    [ "${#spikes_json[@]}" -gt 0 ] && spikes_arr=$(json_arr "${spikes_json[@]}") || spikes_arr="[]"

    summary=$(json_obj total_specs "${total_specs:-0}" shipped "${shipped_specs:-0}" decisions "${total_decisions:-0}")
    data=$(json_obj \
        variant "$(json_qs "$VARIANT")" \
        active_project "$(json_qs "$ACTIVE_PROJECT")" \
        specs "$specs_arr" \
        patches "$patches_arr" \
        spikes "$spikes_arr" \
        missing_cost_specs "$missing_arr" \
        summary "$summary")
    json_emit status "$data"
    exit 0
fi

echo "${BOLD}Repo status${RESET}"
echo ""
echo "  Variant:         ${VARIANT}"
echo "  Active project:  ${ACTIVE_PROJECT}"
echo ""

# --- All projects ---
echo "${BOLD}All projects${RESET}"
for p in "${REPO_ROOT}"/projects/PROJ-*; do
    [ -d "$p" ] || continue
    pname=$(basename "$p")
    brief="${p}/brief.md"
    status="unknown"
    if [ -f "$brief" ]; then
        # Grep for "status:" nested under "project:" in the front-matter
        status=$(awk '
            /^---$/ { f = !f; next }
            f && /^project:/ { inproj = 1; next }
            f && inproj && /^[a-zA-Z_]+:/ { inproj = 0 }
            f && inproj && /^[[:space:]]+status:/ { print $2; exit }
        ' "$brief" 2>/dev/null || echo "unknown")
    fi
    marker=" "
    if [ "$pname" = "$ACTIVE_PROJECT" ]; then marker="${GREEN}*${RESET}"; fi
    printf "  %s %-40s  status: %s\n" "$marker" "$pname" "$status"
done
echo ""

# --- Active project: stages ---
echo "${BOLD}Stages in ${ACTIVE_PROJECT}${RESET}"
stages_dir="${ACTIVE_PROJECT_DIR}/stages"
if [ -d "$stages_dir" ]; then
    for s in "${stages_dir}"/STAGE-*.md; do
        [ -f "$s" ] || continue
        sname=$(basename "$s" .md)
        pstatus=$(awk '/^---$/{f=!f; next} f && /^[[:space:]]+status:/{print $2; exit}' "$s" 2>/dev/null || echo "unknown")
        printf "  %-44s  status: %s\n" "$sname" "$pstatus"
    done
else
    echo "  ${DIM}(no stages dir yet)${RESET}"
fi
echo ""

# --- Active project: specs by cycle ---
echo "${BOLD}Specs in ${ACTIVE_PROJECT} by cycle${RESET}"
specs_dir="${ACTIVE_PROJECT_DIR}/specs"
if [ -d "$specs_dir" ]; then
    for cycle in frame design build verify ship; do
        count=0
        names=""
        for f in "${specs_dir}"/SPEC-*.md; do
            [ -f "$f" ] || continue
            spec_cycle=$(awk '/^---$/{f=!f; next} f && /^[[:space:]]+cycle:/{print $2; exit}' "$f" 2>/dev/null || echo "")
            if [ "$spec_cycle" = "$cycle" ]; then
                count=$((count + 1))
                tag=""; [ "$(get_spec_type "$f")" = "release" ] && tag=" ${DIM}[release]${RESET}"
                names="${names}    - $(basename "$f" .md)${tag}\n"
            fi
        done
        # Also count done/ as ship
        if [ "$cycle" = "ship" ] && [ -d "${specs_dir}/done" ]; then
            for f in "${specs_dir}/done"/SPEC-*.md; do
                [ -f "$f" ] || continue
                count=$((count + 1))
                tag=""; [ "$(get_spec_type "$f")" = "release" ] && tag=" ${DIM}[release]${RESET}"
                names="${names}    - $(basename "$f" .md)${tag} ${DIM}(archived)${RESET}\n"
            done
        fi
        printf "  ${BOLD}%-8s${RESET} (%d)\n" "$cycle" "$count"
        if [ -n "$names" ]; then
            printf "%b" "$names"
        fi
    done
else
    echo "  ${DIM}(no specs yet)${RESET}"
fi
echo ""

# --- Active project: patches (the patch lane, DEC-003) ---
patches_dir="${ACTIVE_PROJECT_DIR}/patches"
patch_total=0
if [ -d "$patches_dir" ]; then
    patch_total=$(find "$patches_dir" -name 'PATCH-*.md' 2>/dev/null | wc -l | tr -d ' ')
fi
if [ "${patch_total:-0}" -gt 0 ]; then
    echo "${BOLD}Patches in ${ACTIVE_PROJECT} by cycle${RESET}"
    for cycle in patch verify ship; do
        count=0; names=""
        for f in "${patches_dir}"/PATCH-*.md; do
            [ -f "$f" ] || continue
            pc=$(awk '/^---$/{f=!f; next} f && /^[[:space:]]+cycle:/{print $2; exit}' "$f" 2>/dev/null || echo "")
            if [ "$pc" = "$cycle" ]; then count=$((count + 1)); names="${names}    - $(basename "$f" .md)\n"; fi
        done
        if [ "$cycle" = "ship" ] && [ -d "${patches_dir}/done" ]; then
            for f in "${patches_dir}/done"/PATCH-*.md; do
                [ -f "$f" ] || continue
                count=$((count + 1)); names="${names}    - $(basename "$f" .md) ${DIM}(archived)${RESET}\n"
            done
        fi
        printf "  ${BOLD}%-8s${RESET} (%d)\n" "$cycle" "$count"
        [ -n "$names" ] && printf "%b" "$names"
    done
    echo ""
fi

# --- Spikes (the bounded-exploration lane, DEC-012) ---
# Repo-level, not project-scoped: a spike may precede any project. Only open
# (un-archived) spikes are shown — a landed spike is settled history.
spike_open=0
if [ -d "${REPO_ROOT}/spikes" ]; then
    spike_open=$(find "${REPO_ROOT}/spikes" -maxdepth 1 -name 'SPIKE-*.md' 2>/dev/null | wc -l | tr -d ' ')
fi
if [ "${spike_open:-0}" -gt 0 ]; then
    echo "${BOLD}Spikes — open exploration (repo-level)${RESET}"
    for f in "${REPO_ROOT}/spikes"/SPIKE-*.md; do
        [ -f "$f" ] || continue
        sc=$(get_spec_cycle "$f"); [ -n "$sc" ] || sc="?"
        smode=$(get_spike_field "$f" mode)
        stb=$(get_spike_field "$f" timebox)
        sout=$(get_spike_field "$f" outcome)
        line="    - $(basename "$f" .md) ${DIM}[${smode:-?} · timebox ${stb:-?} · ${sc}]${RESET}"
        # A spike at `land` with no outcome is the failure this lane prevents.
        if [ "$sc" = "land" ] && { [ -z "$sout" ] || [ "$sout" = "null" ]; }; then
            line="${line} ${YELLOW}← landed with no outcome; validate will fail${RESET}"
        fi
        printf "%b\n" "$line"
    done
    echo "  ${DIM}Land every spike (just archive-spike) — an un-landed spike is undocumented decisions.${RESET}"
    echo ""
fi

# --- Low-confidence decisions ---
echo "${BOLD}Low-confidence decisions (< 0.7)${RESET}"
decisions_dir="${REPO_ROOT}/decisions"
found_any=false
if [ -d "$decisions_dir" ]; then
    for d in "${decisions_dir}"/DEC-*.md; do
        [ -f "$d" ] || continue
        conf=$(awk '/^---$/{f=!f; next} f && /^[[:space:]]+confidence:/{print $2; exit}' "$d" 2>/dev/null || echo "")
        if [ -n "$conf" ]; then
            # Use awk for float comparison (portable)
            low=$(awk -v c="$conf" 'BEGIN { print (c + 0 < 0.7) ? "1" : "0" }')
            if [ "$low" = "1" ]; then
                printf "  %-42s  confidence: %s\n" "$(basename "$d" .md)" "$conf"
                found_any=true
            fi
        fi
    done
fi
if [ "$found_any" = "false" ]; then
    echo "  ${DIM}(none — or no decisions yet)${RESET}"
fi
echo ""

# --- Stale specs (no commits on their branch in 7 days, approximate) ---
echo "${BOLD}Possibly stale specs${RESET}"
echo "  ${DIM}(heuristic: specs in build/verify with file mtime > 7 days)${RESET}"
found_stale=false
if [ -d "$specs_dir" ]; then
    for f in "${specs_dir}"/SPEC-*.md; do
        [ -f "$f" ] || continue
        cycle=$(awk '/^---$/{fm=!fm; next} fm && /^[[:space:]]+cycle:/{print $2; exit}' "$f" 2>/dev/null || echo "")
        if [ "$cycle" = "build" ] || [ "$cycle" = "verify" ]; then
            # Age in days (portable across macOS and Linux).
            if [ "$(uname)" = "Darwin" ]; then
                age_days=$(( ( $(date +%s) - $(stat -f %m "$f") ) / 86400 ))
            else
                age_days=$(( ( $(date +%s) - $(stat -c %Y "$f") ) / 86400 ))
            fi
            if [ "$age_days" -gt 7 ]; then
                printf "  %-40s  cycle: %-8s  age: %d days\n" "$(basename "$f" .md)" "$cycle" "$age_days"
                found_stale=true
            fi
        fi
    done
fi
if [ "$found_stale" = "false" ]; then
    echo "  ${DIM}(none)${RESET}"
fi
echo ""

# --- Specs missing cost data (shipped, non-grandfathered) ---
echo "${BOLD}Specs missing cost data${RESET}"
echo "  ${DIM}(shipped specs whose build/verify cycles lack tokens_total — run just cost-audit)${RESET}"
found_missing_cost=false
for f in $(find_all_specs "$ACTIVE_PROJECT_DIR"); do
    case "$f" in *-timeline.md) continue ;; esac
    name=$(basename "$f" .md)
    shipped=0
    case "$f" in
        */specs/done/*) shipped=1 ;;
        *) if [ "$(get_spec_cycle "$f")" = "ship" ]; then shipped=1; fi ;;
    esac
    [ "$shipped" = "1" ] || continue
    if is_grandfathered_cost "$name"; then continue; fi
    missing=$(spec_missing_cost_cycles "$f")
    if [ -n "$missing" ]; then
        printf "  %-44s  missing: %s\n" "$name" "$missing"
        found_missing_cost=true
    fi
done
if [ "$found_missing_cost" = "false" ]; then
    echo "  ${DIM}(none)${RESET}"
fi
echo ""

# --- Summary counts ---
# `|| true`: tolerate a bare active project with no specs/ dir (see --json path).
total_specs=$(find "${ACTIVE_PROJECT_DIR}/specs" -name "SPEC-*.md" 2>/dev/null | wc -l | tr -d ' ' || true)
shipped_specs=$(find "${ACTIVE_PROJECT_DIR}/specs/done" -name "SPEC-*.md" 2>/dev/null | wc -l | tr -d ' ' || true)
total_decisions=$(find "$decisions_dir" -name "DEC-*.md" 2>/dev/null | wc -l | tr -d ' ' || true)
echo "${BOLD}Summary${RESET}"
echo "  Total specs in ${ACTIVE_PROJECT}:     ${total_specs}"
echo "  Shipped (archived):                   ${shipped_specs}"
echo "  Total decisions (across all projects): ${total_decisions}"
