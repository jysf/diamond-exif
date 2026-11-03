#!/usr/bin/env bash
# scripts/validate.sh — the schema gate (DEC-001 §1).
#
# Checks that every spec's front-matter carries the required STRUCTURAL fields
# with valid values, so the front-matter stays a reliable contract for reports,
# `--json`, and any downstream consumer (MCP, exporter, UI). Exits non-zero on
# any violation — the CI gate contract (DEC-001 §2, exit 1).
#
# Scope (v1): specs. Cost recording on SHIPPED specs is a separate gate
# (`just cost-audit`); decision records are linted by `just decisions-audit`.
# This validator is the place to grow stage/brief checks over time.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/_lib.sh
source "${SCRIPT_DIR}/_lib.sh"

require_initialized
JSON_OUT=$(has_json_flag "$@")

# fm_scalar FILE TOP SUB → first token of TOP.SUB (2-space-nested scalar),
# empty if absent. Tolerates trailing "# ..." comments (takes field $2).
fm_scalar() {
    awk -v top="$2" -v subk="$3" '
        /^---$/ { f = !f; next }
        !f { exit }
        $0 ~ ("^" top ":") { intop = 1; next }
        intop && /^[^[:space:]]/ { intop = 0 }
        intop && $0 ~ ("^[[:space:]]+" subk ":") { print $2; exit }
    ' "$1"
}

VALID_CYCLE=" frame design build verify ship "
VALID_PATCH_CYCLE=" patch verify ship "
VALID_SPIKE_CYCLE=" spike land "        # the spike lane (DEC-012)
VALID_SPIKE_OUTCOME=" answered inconclusive graduated discarded "
VALID_COMPLEXITY=" XS S M L XL XXL "   # t-shirt scale; S|M|L predate it and stay valid
# The verify cycle's outcome. ADVISORY, never a gate: every spec that shipped
# before this field existed has none, and back-filling one would be fiction.
VALID_VERDICT=" approved punch-list rejected "
# `project.activity` is an OPEN, suggested set — not a hard enum. An
# unrecognized value is advisory (warn-only), never a gate failure, so
# people can extend the vocabulary. `spike` was the documented example
# extension and became official with the spike lane (DEC-012).
SUGGESTED_ACTIVITY=" requirements design build test blocked spike "
# The declared roadmap block (DEC-011). Both vocabularies are SUGGESTED sets on
# the same principle as `activity`: a roadmap you can't express is worse than an
# unrecognized value, so validate advises and never gates. `framed`/`planned`
# are derived from files rather than authored, but accepted here so a brief can
# spell out what the tooling would have inferred.
SUGGESTED_ROADMAP_KIND=" framed planned pillar goal "
SUGGESTED_ROADMAP_HORIZON=" now next later "
# Why a project ended. `status` stays the coarse machine state; this carries the
# interesting half, which `cancelled` flattens. Open set, warn-only.
SUGGESTED_CLOSED_REASON=" shipped abandoned superseded parked "

offenders=0
checked=0
off_names=(); off_problems=(); off_kinds=()
activity_notes=""
depends_notes=""
actual_notes=""
roadmap_notes=""
verdict_notes=""
closed_notes=""

while IFS= read -r pdir; do
    [ -n "$pdir" ] || continue
    while IFS= read -r f; do
        [ -n "$f" ] || continue
        # Skip non-spec SPEC-*.md files that share the glob: cycle-prompt files
        # under specs/prompts/, and timeline artifacts.
        case "$f" in
            */prompts/*) continue ;;
            *-timeline.md) continue ;;
        esac
        name=$(basename "$f" .md)
        problems=""

        [ -n "$(fm_scalar "$f" task id)" ]      || problems="${problems} task.id"
        [ -n "$(fm_scalar "$f" task type)" ]    || problems="${problems} task.type"
        [ -n "$(fm_scalar "$f" project id)" ]   || problems="${problems} project.id"
        [ -n "$(fm_scalar "$f" project stage)" ]|| problems="${problems} project.stage"
        [ -n "$(fm_scalar "$f" repo id)" ]      || problems="${problems} repo.id"

        cyc=$(fm_scalar "$f" task cycle)
        case "$VALID_CYCLE" in *" $cyc "*) : ;; *) problems="${problems} task.cycle(='${cyc:-∅}')" ;; esac

        cx=$(fm_scalar "$f" task complexity)
        case "$VALID_COMPLEXITY" in *" $cx "*) : ;; *) problems="${problems} task.complexity(='${cx:-∅}')" ;; esac

        # `task.complexity_actual` (the size stamped at ship). ADVISORY only:
        # an unrecorded actual is the normal state, and the estimation loop
        # (`just calibration`) is a feedback loop, never a gate.
        cxa=$(get_spec_complexity_actual "$f")
        if [ -n "$cxa" ]; then
            case "$VALID_COMPLEXITY" in
                *" $cxa "*) : ;;
                *) actual_notes="${actual_notes}    ${name}: complexity_actual='${cxa}' (not a t-shirt size)"$'\n' ;;
            esac
        fi

        # `task.verify_verdict`. ADVISORY, same reasoning as complexity_actual:
        # an unrecorded verdict is normal (the spec may not have reached verify),
        # and the verify study is a feedback loop, never a gate.
        vv=$(get_spec_verify_verdict "$f")
        if [ -n "$vv" ]; then
            case "$VALID_VERDICT" in
                *" $vv "*) : ;;
                *) verdict_notes="${verdict_notes}    ${name}: verify_verdict='${vv}'"$'\n' ;;
            esac
        fi

        # `depends_on:` refs. ADVISORY only — a dangling ref is collected here and
        # never counted as an offender, so a spec that names a not-yet-created
        # dependency (the normal case while framing a stage) can't fail the gate.
        while IFS= read -r dep; do
            [ -n "$dep" ] || continue
            if ! find "${pdir}/specs" -maxdepth 2 -type f -name "${dep}-*.md" \
                    ! -name '*-timeline.md' 2>/dev/null | grep -q .; then
                depends_notes="${depends_notes}    ${name}: depends_on '${dep}' (no such spec yet)"$'\n'
            fi
        done < <(get_spec_depends_on "$f")

        checked=$((checked + 1))
        if [ -n "$problems" ]; then
            off_names+=("$name"); off_problems+=("$problems"); off_kinds+=("spec")
            [ "$JSON_OUT" = 1 ] || printf "  %-52s invalid/missing:%s\n" "$name" "$problems"
            offenders=$((offenders + 1))
        fi
    done < <(find_all_specs "$pdir")

    # Patches (the patch lane, DEC-003): same task.* schema as specs, but the
    # cycle enum is patch/verify/ship and there is NO project.stage requirement
    # (a patch attaches to the project, not a stage).
    while IFS= read -r f; do
        [ -n "$f" ] || continue
        case "$f" in
            */prompts/*) continue ;;
            *-timeline.md) continue ;;
        esac
        name=$(basename "$f" .md)
        problems=""

        [ -n "$(fm_scalar "$f" task id)" ]    || problems="${problems} task.id"
        [ -n "$(fm_scalar "$f" task type)" ]  || problems="${problems} task.type"
        [ -n "$(fm_scalar "$f" project id)" ] || problems="${problems} project.id"
        [ -n "$(fm_scalar "$f" repo id)" ]    || problems="${problems} repo.id"

        cyc=$(fm_scalar "$f" task cycle)
        case "$VALID_PATCH_CYCLE" in *" $cyc "*) : ;; *) problems="${problems} task.cycle(='${cyc:-∅}')" ;; esac

        cx=$(fm_scalar "$f" task complexity)
        case "$VALID_COMPLEXITY" in *" $cx "*) : ;; *) problems="${problems} task.complexity(='${cx:-∅}')" ;; esac

        checked=$((checked + 1))
        if [ -n "$problems" ]; then
            off_names+=("$name"); off_problems+=("$problems"); off_kinds+=("patch")
            [ "$JSON_OUT" = 1 ] || printf "  %-52s invalid/missing:%s\n" "$name" "$problems"
            offenders=$((offenders + 1))
        fi
    done < <(find_all_patches "$pdir")

    # Brief `project.activity` (optional, human-facing). OPEN set: an
    # unrecognized value is ADVISORY only — collected here, never counted
    # as an offender, so it can never fail the gate.
    act=$(get_project_activity "$pdir")
    if [ -n "$act" ]; then
        case "$SUGGESTED_ACTIVITY" in
            *" $act "*) : ;;
            *) activity_notes="${activity_notes}    $(basename "$pdir"): activity='${act}'"$'\n' ;;
        esac
    fi

    # `closed_reason` (optional, stamped at close). Open set, advisory only.
    pname=$(basename "$pdir")
    cr=$(awk '
        /^---$/ { f = !f; next } !f { next }
        /^closed_reason:/ { v = $2; if (v != "null" && v != "") print v; exit }
    ' "${pdir}/brief.md" 2>/dev/null)
    if [ -n "$cr" ]; then
        case "$SUGGESTED_CLOSED_REASON" in
            *" $cr "*) : ;;
            *) closed_notes="${closed_notes}    ${pname}: closed_reason='${cr}'"$'\n' ;;
        esac
    fi

    # Declared roadmap block (DEC-011). Optional; advisory-only on both
    # vocabularies, never counted as an offender.
    while IFS='|' read -r r_item r_kind r_hz r_resume r_target; do
        [ -n "$r_item" ] || continue
        if [ "$r_kind" != "-" ]; then
            case "$SUGGESTED_ROADMAP_KIND" in
                *" $r_kind "*) : ;;
                *) roadmap_notes="${roadmap_notes}    ${pname}: roadmap '${r_item}' kind='${r_kind}'"$'\n' ;;
            esac
        fi
        if [ "$r_hz" != "-" ]; then
            case "$SUGGESTED_ROADMAP_HORIZON" in
                *" $r_hz "*) : ;;
                *) roadmap_notes="${roadmap_notes}    ${pname}: roadmap '${r_item}' horizon='${r_hz}'"$'\n' ;;
            esac
        fi
    done <<EOF
$(parse_declared_roadmap "$pdir")
EOF
done < <(find "${REPO_ROOT}/projects" -maxdepth 1 -type d -name 'PROJ-*' 2>/dev/null | sort)

# Spikes (the bounded-exploration lane, DEC-012). REPO-level, not project-scoped
# — a spike may precede any project, so this loop sits outside the project walk.
# Cycle enum is spike|land; `project.stage` is absent and `project.id` optional.
#
# The one real tooth: a spike at `cycle: land` MUST carry a spike.outcome. An
# un-landed spike is precisely the failure this lane exists to prevent
# (undocumented decisions leaking into production), so it fails the gate rather
# than warning. A spike still at `cycle: spike` is mid-exploration — left alone.
while IFS= read -r f; do
    [ -n "$f" ] || continue
    case "$f" in
        */done/*) continue ;;
        */prompts/*) continue ;;
    esac
    name=$(basename "$f" .md)
    problems=""

    [ -n "$(fm_scalar "$f" task id)" ]   || problems="${problems} task.id"
    [ -n "$(fm_scalar "$f" task type)" ] || problems="${problems} task.type"
    [ -n "$(fm_scalar "$f" repo id)" ]   || problems="${problems} repo.id"

    cyc=$(fm_scalar "$f" task cycle)
    case "$VALID_SPIKE_CYCLE" in *" $cyc "*) : ;; *) problems="${problems} task.cycle(='${cyc:-∅}')" ;; esac

    # spike.question and spike.timebox are what make it a spike rather than
    # undirected coding — both required from creation.
    q=$(get_spike_field "$f" question)
    [ -n "$q" ] && [ "$q" != "null" ] || problems="${problems} spike.question"
    tb=$(get_spike_field "$f" timebox)
    [ -n "$tb" ] && [ "$tb" != "null" ] || problems="${problems} spike.timebox"

    md=$(get_spike_field "$f" mode)
    case " question build " in *" $md "*) : ;; *) problems="${problems} spike.mode(='${md:-∅}')" ;; esac

    out=$(get_spike_field "$f" outcome)
    if [ "$cyc" = "land" ]; then
        case "$VALID_SPIKE_OUTCOME" in
            *" $out "*) : ;;
            *) problems="${problems} spike.outcome(='${out:-∅}'; a LANDED spike must record one of${VALID_SPIKE_OUTCOME})" ;;
        esac
    elif [ -n "$out" ] && [ "$out" != "null" ]; then
        # Mid-spike with an outcome already set is fine, but it must be valid.
        case "$VALID_SPIKE_OUTCOME" in
            *" $out "*) : ;;
            *) problems="${problems} spike.outcome(='${out}')" ;;
        esac
    fi

    checked=$((checked + 1))
    if [ -n "$problems" ]; then
        off_names+=("$name"); off_problems+=("$problems"); off_kinds+=("spike")
        [ "$JSON_OUT" = 1 ] || printf "  %-52s invalid/missing:%s\n" "$name" "$problems"
        offenders=$((offenders + 1))
    fi
done < <(find_all_spikes)

# --- Machine-readable output (DEC-001 §2): the gate's findings, not just its
# exit code. An agent that hits a red gate can act on WHICH artifact failed
# WHICH check instead of screen-scraping prose.
if [ "$JSON_OUT" = 1 ]; then
    items=()
    i=0
    while [ "$i" -lt "${#off_names[@]}" ]; do
        # Trim the leading space the accumulator adds, then split on spaces.
        probs="${off_problems[$i]# }"
        parts=(); for tok in $probs; do parts+=("$(json_qs "$tok")"); done
        [ "${#parts[@]}" -gt 0 ] && parr=$(json_arr "${parts[@]}") || parr="[]"
        items+=("$(json_obj \
            artifact "$(json_qs "${off_names[$i]}")" \
            kind "$(json_qs "${off_kinds[$i]}")" \
            problems "$parr")")
        i=$((i + 1))
    done
    [ "${#items[@]}" -gt 0 ] && arr=$(json_arr "${items[@]}") || arr="[]"
    json_emit validate "$(json_obj \
        checked "$checked" offenders "$offenders" ok "$([ "$offenders" -eq 0 ] && echo true || echo false)" \
        violations "$arr")"
    [ "$offenders" -gt 0 ] && exit 1
    exit 0
fi

# Advisory: an unrecognized `project.activity` (open set). Printed after
# the gate result — never changes the exit code.
if [ -n "$activity_notes" ]; then
    warn "validate: unrecognized project.activity (advisory — activity is an open set, not enforced; extend it freely):"
    printf '%s' "$activity_notes"
fi

# Advisory: an unrecognized declared-roadmap `kind`/`horizon` (DEC-011). Both
# are suggested sets, not enums — never changes the exit code.
if [ -n "$roadmap_notes" ]; then
    warn "validate: unrecognized roadmap kind/horizon (advisory — suggested sets, not enforced):"
    printf '%s' "$roadmap_notes"
fi

# Advisory: an unrecognized `closed_reason` (open set). Never changes the exit code.
if [ -n "$closed_notes" ]; then
    warn "validate: unrecognized closed_reason (advisory — open set, expected one of${SUGGESTED_CLOSED_REASON}):"
    printf '%s' "$closed_notes"
fi

# Advisory: an unrecognized `task.verify_verdict`. Never changes the exit code.
if [ -n "$verdict_notes" ]; then
    warn "validate: unrecognized task.verify_verdict (advisory — expected one of${VALID_VERDICT}):"
    printf '%s' "$verdict_notes"
fi

# Advisory: an unrecognized `task.complexity_actual`. Never changes the exit code.
if [ -n "$actual_notes" ]; then
    warn "validate: unrecognized task.complexity_actual (advisory — expected one of${VALID_COMPLEXITY}):"
    printf '%s' "$actual_notes"
fi

# Advisory: a depends_on ref with no matching spec yet. Never changes the exit code
# — naming a dependency before it's written is normal while framing a stage.
if [ -n "$depends_notes" ]; then
    warn "validate: depends_on references a spec that doesn't exist yet (advisory — not enforced):"
    printf '%s' "$depends_notes"
fi

if [ "$offenders" -gt 0 ]; then
    echo ""
    die "validate: ${offenders} artifact(s) with invalid/missing required front-matter (checked ${checked}). See DEC-001 §1 / docs/schema-reference.md."
fi
success "validate: ${checked} artifact(s) (specs + patches + spikes) have valid required front-matter."
