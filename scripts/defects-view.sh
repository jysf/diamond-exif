#!/usr/bin/env bash
# scripts/defects-view.sh — the `just dash defects` lens: the defect-escape
# distribution (ship Reflection Q4 / a patch's Defect-catch-stage).
#
# Why this exists: every shipped spec already answers "where was the worst defect
# caught?" from a fixed vocabulary, and until now NOTHING read it. The retro that
# asked for the field said its value is "only visible in a cross-project view" —
# so the field shipped half-finished (the reserved-but-not-wired pattern, harvest
# signal #7). This is the reader.
#
# The number that matters is `escaped`: a defect that made it through design,
# build AND verify into the real world. The §12 behavioral pre-flight exists to
# catch the operational/runtime kind; a rising escaped count means it isn't
# being run on the surfaces that need it. (An earlier version of this comment
# said every escaped defect in the dogfood was operational. The corpus has no
# recorded escaped defects, so that was not a finding.)
#
# Scans ALL projects by default (the distribution is only meaningful in
# aggregate); pass --active or a PROJ-NNN to scope. Read-only.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/_lib.sh
source "${SCRIPT_DIR}/_lib.sh"

require_initialized
JSON_OUT=$(has_json_flag "$@")

SCOPE_DIRS=""
for a in "$@"; do
    case "$a" in
        --active) SCOPE_DIRS="${REPO_ROOT}/projects/$(get_active_project)" ;;
        PROJ-*)   SCOPE_DIRS=$(resolve_project_dir "$a") ;;
    esac
done
if [ -z "$SCOPE_DIRS" ]; then
    SCOPE_DIRS=$(find "${REPO_ROOT}/projects" -maxdepth 1 -type d -name 'PROJ-*' 2>/dev/null | sort)
fi

# Extract the answered defect-catch stage from one artifact.
#
# The vocabulary words also appear in the template's own prompt text, but ALWAYS
# wrapped in backticks (`design` | `build` | …). The answer is written bare. So:
# take the first BARE vocabulary word on an answer line, ignoring any line that
# contains a backtick. That keeps an unanswered template from counting as data.
defect_stage_of() {
    awk '
        # Spec form: the Q4 block, answer on a following "— <word>" line.
        /Where was the worst defect caught/ { inq = 1; n = 0; next }
        # Patch form: the answer follows the Defect-catch-stage bullet.
        /\*\*Defect-catch-stage:\*\*/ { inq = 1; n = 0 }
        inq && /`/ { next }                      # skip the vocabulary lines
        inq {
            # Bound the window: without this, an unanswered Q4 could swallow a
            # LATER question’s one-word answer (Q5 legitimately accepts "none").
            n++
            if (n > 10) exit
            line = $0
            # Strip everything up to the first lowercase letter. Deliberately NOT
            # a character class containing the em-dash: awk here is byte-oriented,
            # and "—" is three UTF-8 bytes, so [—-] shreds it mid-character and
            # leaves stray bytes glued to the answer.
            sub(/^[^a-z]*/, "", line)
            sub(/[[:space:]]*$/, "", line)
            if (line == "design" || line == "build" || line == "verify" ||
                line == "ship" || line == "escaped" || line == "none") {
                print line; exit
            }
            if (line ~ /^#/) exit
        }
    ' "$1"
}

c_design=0; c_build=0; c_verify=0; c_ship=0; c_escaped=0; c_none=0
answered=0; unanswered=0
esc_ids=(); esc_where=()

# The SECOND axis: what verify actually decided (task.verify_verdict). Q4 above
# says where the worst defect was CAUGHT; this says whether verify ever pushed
# back at all. Together they answer the question the template has never been
# able to: is the verify gate doing work, or waving things through?
v_approved=0; v_punch=0; v_rejected=0; v_unrecorded=0

# verify_verdict is the LAST outcome only, so pushback is counted from every
# round instead: task.verify_rounds where recorded, and for specs that predate
# it, repeat verify handoffs (a second verify handoff means the first round sent
# the spec back). Irradiance: the last-verdict field showed 1 pushback, its
# handoffs 5.
pb_specs=0; pb_from_rounds=0; pb_from_handbacks=0; pb_from_handoffs=0
rounds_total=0; r_punch=0; r_rejected=0

# "SPEC-NNN verdict" per verify handoff across the scope (verdict = the
# handback.verdict the verifier wrote, or "-" when the handoff predates it).
HANDOFF_FILES=""
while IFS= read -r pdir; do
    [ -n "$pdir" ] || continue
    HANDOFF_FILES="${HANDOFF_FILES}$(find "${pdir}/handoffs" -type f -name 'HANDOFF-*.md' 2>/dev/null || true)
"
done <<SCOPE
${SCOPE_DIRS}
SCOPE
VERIFY_HANDOFFS=$(printf '%s' "$HANDOFF_FILES" | while IFS= read -r hf; do
        [ -n "$hf" ] || continue
        [ "$(get_handoff_field "$hf" cycle)" = verify ] || continue
        sid=$(awk '/^---$/ { fm = !fm; next } fm && /^[[:space:]]+spec_id:/ { print $2; exit }' "$hf")
        hv=$(get_handback_field "$hf" verdict)
        [ -n "$hv" ] && [ "$hv" != "null" ] || hv="-"
        printf '%s %s\n' "$sid" "$hv"
    done)
# How many verify handoffs a spec had, and how many of them recorded pushback.
verify_handoffs_for() {
    printf '%s\n' "$VERIFY_HANDOFFS" | awk -v id="$1" '$1 == id { n++ } END { print n + 0 }'
}
handback_pushbacks_for() {
    printf '%s\n' "$VERIFY_HANDOFFS" | awk -v id="$1" '$1 == id && ($2 == "punch-list" || $2 == "rejected") { n++ } END { print n + 0 }'
}

scan_verdict() {
    local f="$1" v r id n pushed=0
    v=$(get_spec_verify_verdict "$f")
    case "$v" in
        approved)   v_approved=$((v_approved + 1)) ;;
        punch-list) v_punch=$((v_punch + 1)) ;;
        rejected)   v_rejected=$((v_rejected + 1)) ;;
        *)          v_unrecorded=$((v_unrecorded + 1)) ;;
    esac
    for r in $(get_spec_verify_rounds "$f"); do
        rounds_total=$((rounds_total + 1))
        case "$r" in
            punch-list) r_punch=$((r_punch + 1)); pushed=1 ;;
            rejected)   r_rejected=$((r_rejected + 1)); pushed=1 ;;
        esac
    done
    if [ "$pushed" = 1 ]; then
        pb_from_rounds=$((pb_from_rounds + 1))
    else
        id=$(basename "$f" | sed -E 's/^(SPEC-[0-9]+).*/\1/')
        n=$(verify_handoffs_for "$id")
        if [ "$(handback_pushbacks_for "$id")" -gt 0 ]; then
            pushed=1; pb_from_handbacks=$((pb_from_handbacks + 1))
        elif [ "${n:-0}" -gt 1 ]; then
            pushed=1; pb_from_handoffs=$((pb_from_handoffs + 1))
        elif [ "$v" = punch-list ] || [ "$v" = rejected ]; then
            pushed=1
        fi
    fi
    if [ "$pushed" = 1 ]; then pb_specs=$((pb_specs + 1)); fi
    return 0
}

scan_one() {
    local f="$1" kind="$2"
    case "$f" in *-timeline.md|*/prompts/*) return 0 ;; esac
    local id stage
    id=$(basename "$f" .md)
    [ "$kind" = spec ] && scan_verdict "$f"
    stage=$(defect_stage_of "$f")
    if [ -z "$stage" ]; then
        unanswered=$((unanswered + 1)); return 0
    fi
    answered=$((answered + 1))
    case "$stage" in
        design)  c_design=$((c_design + 1)) ;;
        build)   c_build=$((c_build + 1)) ;;
        verify)  c_verify=$((c_verify + 1)) ;;
        ship)    c_ship=$((c_ship + 1)) ;;
        none)    c_none=$((c_none + 1)) ;;
        escaped)
            c_escaped=$((c_escaped + 1))
            esc_ids+=("$id"); esc_where+=("$kind")
            ;;
    esac
}

while IFS= read -r pdir; do
    [ -n "$pdir" ] || continue
    while IFS= read -r f; do
        [ -n "$f" ] || continue
        scan_one "$f" spec
    done < <(find_all_specs "$pdir")
    while IFS= read -r f; do
        [ -n "$f" ] || continue
        scan_one "$f" patch
    done < <(find_all_patches "$pdir")
done <<EOF
${SCOPE_DIRS}
EOF

total=$((answered + unanswered))

if [ "$JSON_OUT" = 1 ]; then
    items=()
    i=0
    while [ "$i" -lt "${#esc_ids[@]}" ]; do
        items+=("$(json_obj artifact "$(json_qs "${esc_ids[$i]}")" kind "$(json_qs "${esc_where[$i]}")")")
        i=$((i + 1))
    done
    [ "${#items[@]}" -gt 0 ] && earr=$(json_arr "${items[@]}") || earr="[]"
    dist=$(json_obj design "$c_design" build "$c_build" verify "$c_verify" \
                    ship "$c_ship" escaped "$c_escaped" none "$c_none")
    verdicts=$(json_obj approved "$v_approved" "punch-list" "$v_punch" \
                    rejected "$v_rejected" unrecorded "$v_unrecorded")
    pushback=$(json_obj specs_pushed_back "$pb_specs" \
                    from_verify_rounds "$pb_from_rounds" from_handback_verdicts "$pb_from_handbacks" \
                    from_repeat_handoffs "$pb_from_handoffs" \
                    rounds_recorded "$rounds_total" punch_list_rounds "$r_punch" rejected_rounds "$r_rejected")
    json_emit defects "$(json_obj \
        total "$total" answered "$answered" unanswered "$unanswered" \
        distribution "$dist" verify_verdicts "$verdicts" verify_pushback "$pushback" \
        escaped_artifacts "$earr")"
    exit 0
fi

printf "${BOLD}Defect-escape distribution (%d answered / %d artifacts)${RESET}\n" "$answered" "$total"
printf "${DIM}where the worst defect was caught — ship Reflection Q4 / a patch's Defect-catch-stage${RESET}\n"

# No Q4 answers yet: say so, but keep going. The verify verdicts below are a
# separate record, and an early exit here used to hide them.
if [ "$answered" -eq 0 ]; then
    printf "  ${DIM}(no artifact has answered Q4 yet — the distribution needs shipped specs)${RESET}\n"
    [ "$unanswered" -gt 0 ] && printf "  ${DIM}%d artifact(s) scanned with no answer recorded.${RESET}\n" "$unanswered"
fi

if [ "$answered" -gt 0 ]; then
    # A tiny inline bar, scaled to the largest bucket, so the shape reads at a glance.
    max=$c_design
    for n in $c_build $c_verify $c_ship $c_escaped $c_none; do
        [ "$n" -gt "$max" ] && max=$n
    done
    [ "$max" -gt 0 ] || max=1

    row() {
        local label="$1" n="$2" hl="${3:-}"
        local width=$(( n * 24 / max ))
        local bar=""
        local i=0
        while [ "$i" -lt "$width" ]; do bar="${bar}█"; i=$((i + 1)); done
        local pct=$(( n * 100 / answered ))
        if [ -n "$hl" ] && [ "$n" -gt 0 ]; then
            printf "  ${BOLD}%-8s${RESET} %3d (%2d%%)  ${YELLOW}%s${RESET}\n" "$label" "$n" "$pct" "$bar"
        else
            printf "  %-8s %3d (%2d%%)  %s\n" "$label" "$n" "$pct" "$bar"
        fi
    }

    row design  "$c_design"
    row build   "$c_build"
    row verify  "$c_verify"
    row ship    "$c_ship"
    row escaped "$c_escaped" highlight
    row none    "$c_none"
fi

# The second axis. Kept deliberately plain — it is a count, not a verdict on the
# verdicts, and the interpretation belongs to whoever runs the study.
v_recorded=$((v_approved + v_punch + v_rejected))
if [ "$v_recorded" -gt 0 ]; then
    echo ""
    printf "${BOLD}Verify verdicts${RESET} ${DIM}(latest per spec, task.verify_verdict, %d spec(s) recorded)${RESET}\n" "$v_recorded"
    printf "  approved    %3d\n" "$v_approved"
    printf "  punch-list  %3d\n" "$v_punch"
    printf "  rejected    %3d\n" "$v_rejected"
    # Pushback counts EVERY round, not the latest verdict: a spec sent back and
    # then approved is pushback, though its verify_verdict says "approved".
    if [ "$pb_specs" -eq 0 ]; then
        printf "  ${YELLOW}⚠${RESET}  verify has never pushed back on a recorded spec, in any round.\n"
        printf "     ${DIM}PLAYBOOK.md lists this as a failure signature. Either the specs are\n"
        printf "     that clean, or the gate is not doing work — worth knowing which.${RESET}\n"
    else
        printf "  sent back at least once: %d spec(s) — %d%% ${DIM}(any round)${RESET}\n" \
            "$pb_specs" $(( pb_specs * 100 / v_recorded ))
        if [ "$pb_from_handbacks" -gt 0 ] || [ "$pb_from_handoffs" -gt 0 ]; then
            printf "  ${DIM}from: %d verify_rounds · %d handback.verdict · %d repeat verify handoffs" \
                "$pb_from_rounds" "$pb_from_handbacks" "$pb_from_handoffs"
            printf " · %d latest verdict only${RESET}\n" \
                $(( pb_specs - pb_from_rounds - pb_from_handbacks - pb_from_handoffs ))
        fi
    fi
elif [ "$v_unrecorded" -gt 0 ]; then
    echo ""
    printf "${DIM}Verify verdicts: none recorded yet (%d spec(s) predate task.verify_verdict).${RESET}\n" "$v_unrecorded"
fi

if [ "$c_escaped" -gt 0 ]; then
    echo ""
    printf "${YELLOW}⚠${RESET} %d defect(s) ESCAPED — through design, build and verify, into the real world:\n" "$c_escaped"
    i=0
    while [ "$i" -lt "${#esc_ids[@]}" ]; do
        printf "    %s ${DIM}(%s)${RESET}\n" "${esc_ids[$i]}" "${esc_where[$i]}"
        i=$((i + 1))
    done
    echo ""
    printf "${DIM}Across the dogfood every escaped defect was operational/runtime, not logic.${RESET}\n"
    printf "${DIM}Each one is a signal to strengthen the §12 behavioral pre-flight for that surface.${RESET}\n"
fi

if [ "$unanswered" -gt 0 ]; then
    echo ""
    printf "${DIM}%d artifact(s) have no Q4 answer — the distribution is only as good as its coverage.${RESET}\n" "$unanswered"
fi
