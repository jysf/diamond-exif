#!/usr/bin/env bash
# scripts/instances-diff.sh — which template capabilities is each instance missing?
#
# WHY THIS EXISTS, and why it is the whole fix. The obvious diagnosis is that
# instances "freeze" at scaffold time and need an update mechanism. That is
# false: hand-porting scripts between repos works fine and has been done
# repeatedly. What actually goes wrong is subtler and needs no mechanism to fix.
#
# You port what you need TODAY. That is rational every single time — and in
# aggregate it strands exactly one class of tooling: the cross-cutting
# ANALYTICAL readers, whose entire value is being present in every repo at once,
# which is precisely the value that never feels urgent in any single repo.
# `defects-view.sh` reached zero instances for this reason, which is why the
# first attempt at the verify study had nothing to read.
#
# So the gap is not capability, it is VISIBILITY. This turns "which repos are
# behind, and on what?" into one command.
#
# It deliberately lives in the TEMPLATE repo and is not shipped to instances:
# an instance cannot diff itself against a template it has no pointer to, and
# putting the checker in the place that goes stale is the same trap one level
# down. Run it here, point it at checkouts on disk.
#
# Read-only. Never writes to an instance.
#
# Usage:
#   ./scripts/instances-diff.sh ~/path/to/instance [more...]
#   ./scripts/instances-diff.sh --json ~/path/to/instance
#
# Exit status: 0 always (this is a report, not a gate) · 2 usage error.

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/_lib.sh
source "${SCRIPT_DIR}/_lib.sh"

JSON_OUT=$(has_json_flag "$@")
TARGETS=()
for a in "$@"; do
    case "$a" in
        --json) : ;;
        -*)     usage_error "./scripts/instances-diff.sh [--json] <instance-path>..." ;;
        '')     : ;;
        *)      TARGETS+=("$a") ;;
    esac
done
[ "${#TARGETS[@]}" -gt 0 ] || usage_error "./scripts/instances-diff.sh [--json] <instance-path>..."

# The template's own script surface is the reference set.
TEMPLATE_SCRIPTS=()
while IFS= read -r f; do
    b=$(basename "$f")
    # instances-diff itself is template-only by design (see the header).
    [ "$b" = "instances-diff.sh" ] && continue
    TEMPLATE_SCRIPTS+=("$b")
done < <(find "${SCRIPT_DIR}" -maxdepth 1 -type f -name '*.sh' | sort)

TEMPLATE_VERSION=$(tr -d ' \t\n\r' < "${REPO_ROOT}/VERSION" 2>/dev/null || printf '?')

# Scripts whose value is CROSS-REPO — they answer questions only in aggregate,
# so they are the ones selective porting reliably strands. Called out separately
# because "you're missing 20 scripts" is noise; "you're missing the three that
# make a corpus-wide study possible" is a decision.
CROSS_CUTTING=" defects-view.sh calibration.sh decisions-index.sh cost-audit.sh validate.sh lifetime-report.sh "

rows=()
for t in "${TARGETS[@]}"; do
    name=$(basename "$t")
    if [ ! -d "$t" ]; then
        [ "$JSON_OUT" = 1 ] || warn "${name}: no such directory — skipped"
        continue
    fi
    if [ ! -d "${t}/scripts" ]; then
        [ "$JSON_OUT" = 1 ] || warn "${name}: no scripts/ — not a scaffolded instance?"
        continue
    fi

    # Test before reading: `2>/dev/null` on the command does not suppress the
    # shell's own error for a missing redirect source, and three of the live
    # instances predate the VERSION file entirely — which is itself the finding.
    iver="pre-VERSION"
    [ -f "${t}/VERSION" ] && iver=$(tr -d ' \t\n\r' < "${t}/VERSION")
    [ -n "$iver" ] || iver="pre-VERSION"

    have=0; missing=(); missing_cross=()
    for s in "${TEMPLATE_SCRIPTS[@]}"; do
        if [ -f "${t}/scripts/${s}" ]; then
            have=$((have + 1))
        else
            missing+=("$s")
            case "$CROSS_CUTTING" in *" $s "*) missing_cross+=("$s") ;; esac
        fi
    done

    if [ "$JSON_OUT" = 1 ]; then
        mj=(); for m in "${missing[@]:-}"; do [ -n "$m" ] && mj+=("$(json_qs "$m")"); done
        cj=(); for c in "${missing_cross[@]:-}"; do [ -n "$c" ] && cj+=("$(json_qs "$c")"); done
        [ "${#mj[@]}" -gt 0 ] && marr=$(json_arr "${mj[@]}") || marr="[]"
        [ "${#cj[@]}" -gt 0 ] && carr=$(json_arr "${cj[@]}") || carr="[]"
        rows+=("$(json_obj \
            instance "$(json_qs "$name")" \
            path "$(json_qs "$t")" \
            version "$(json_qs "$iver")" \
            scripts_present "$have" \
            scripts_total "${#TEMPLATE_SCRIPTS[@]}" \
            missing "$marr" \
            missing_cross_cutting "$carr")")
        continue
    fi

    printf "${BOLD}%-22s${RESET} %-12s %2d/%d scripts\n" \
        "$name" "$iver" "$have" "${#TEMPLATE_SCRIPTS[@]}"
    if [ "${#missing_cross[@]}" -gt 0 ]; then
        printf "  ${YELLOW}cross-repo tooling missing${RESET} ${DIM}(these only pay off when every repo has them)${RESET}\n"
        for c in "${missing_cross[@]}"; do printf "    %s\n" "$c"; done
    fi
    if [ "${#missing[@]}" -gt 0 ]; then
        printf "  ${DIM}also missing: %s${RESET}\n" "$(printf '%s ' "${missing[@]}" | fold -s -w 92 | sed '2,$s/^/                /')"
    else
        printf "  ${GREEN}up to date${RESET}\n"
    fi
    echo ""
done

if [ "$JSON_OUT" = 1 ]; then
    [ "${#rows[@]}" -gt 0 ] && arr=$(json_arr "${rows[@]}") || arr="[]"
    json_emit instances-diff "$(json_obj \
        template_version "$(json_qs "$TEMPLATE_VERSION")" \
        scripts_total "${#TEMPLATE_SCRIPTS[@]}" \
        instances "$arr")"
    exit 0
fi

printf "${DIM}Template is v%s with %d scripts. Porting is manual and that is fine —\n" \
    "$TEMPLATE_VERSION" "${#TEMPLATE_SCRIPTS[@]}"
printf "the gap this closes is knowing WHICH repo is missing WHAT.${RESET}\n"
