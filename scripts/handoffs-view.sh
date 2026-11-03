#!/usr/bin/env bash
# scripts/handoffs-view.sh — the `just dash handoffs` lens: delegation handoffs
# (projects/*/handoffs/HANDOFF-*.md) grouped by status. Meaningful in the
# claude-plus-agents variant; claude-only has none (prints an empty view).
# --json emits handoff.* names. Read-only.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/_lib.sh
source "${SCRIPT_DIR}/_lib.sh"

require_initialized
JSON_OUT=$(has_json_flag "$@")

# read_hf FILE BLOCK KEY — a scalar under a top-level front-matter block
# (e.g. handoff.status, task.spec_id). Tolerates a trailing "# comment".
read_hf() {
    awk -v blk="$2" -v k="$3" '
        /^---$/ { fm = !fm; next }
        !fm { exit }
        $0 ~ ("^" blk ":") { inb = 1; next }
        inb && /^[^[:space:]]/ { inb = 0 }
        inb && $0 ~ ("^[[:space:]]+" k ":") {
            v = $0; sub(/^[^:]*:[[:space:]]*/, "", v);
            sub(/[[:space:]]+#.*$/, "", v); gsub(/^"|"$/, "", v); print v; exit
        }
    ' "$1"
}

ids=(); sts=(); tos=(); froms=(); specs=()
while IFS= read -r f; do
    [ -n "$f" ] || continue
    case "$f" in */prompts/*) continue ;; esac
    ids+=("$(basename "$f" .md | grep -oE 'HANDOFF-[0-9]+')")
    sts+=("$(read_hf "$f" handoff status)")
    tos+=("$(read_hf "$f" handoff to_agent)")
    froms+=("$(read_hf "$f" handoff from_agent)")
    specs+=("$(read_hf "$f" task spec_id)")
done < <(find "${REPO_ROOT}/projects" -type f -name 'HANDOFF-*.md' 2>/dev/null | sort)
total=${#ids[@]}
open=0
for s in "${sts[@]:-}"; do case "$s" in pending|accepted) open=$((open + 1)) ;; esac; done

if [ "$JSON_OUT" = 1 ]; then
    items=()
    i=0
    while [ "$i" -lt "$total" ]; do
        items+=("$(json_obj \
            "handoff.id" "$(json_qs "${ids[$i]}")" \
            "handoff.status" "$(json_qs "${sts[$i]:-?}")" \
            "handoff.from_agent" "$(json_qs "${froms[$i]:-?}")" \
            "handoff.to_agent" "$(json_qs "${tos[$i]:-?}")" \
            "task.spec_id" "$(json_qs "${specs[$i]:-?}")")")
        i=$((i + 1))
    done
    [ "${#items[@]}" -gt 0 ] && arr=$(json_arr "${items[@]}") || arr="[]"
    json_emit handoffs "$(json_obj open "$open" total "$total" handoffs "$arr")"
    exit 0
fi

printf "${BOLD}Handoffs (%d open / %d total)${RESET}\n" "$open" "$total"
printf "${DIM}columns: status · id · spec · from → to${RESET}\n"
if [ "$total" -eq 0 ]; then
    printf "  ${DIM}(none — this variant delegates without handoffs, or none exist yet)${RESET}\n"
    exit 0
fi
for status in pending accepted completed rejected ""; do
    i=0
    while [ "$i" -lt "$total" ]; do
        match=0
        if [ -n "$status" ]; then
            [ "${sts[$i]}" = "$status" ] && match=1
        else
            case "${sts[$i]}" in pending|accepted|completed|rejected) : ;; *) match=1 ;; esac
        fi
        if [ "$match" = 1 ]; then
            printf "  [%-9s] %-14s %-12s %s → %s\n" \
                "${sts[$i]:-?}" "${ids[$i]}" "${specs[$i]:-?}" "${froms[$i]:-?}" "${tos[$i]:-?}"
        fi
        i=$((i + 1))
    done
done
