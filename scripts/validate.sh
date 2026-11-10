#!/usr/bin/env bash
# scripts/validate.sh — the schema gate (DEC-001 §1).
#
# Checks that every spec's front-matter carries the required STRUCTURAL fields
# with valid values, so the front-matter stays a reliable contract for reports,
# `--json`, and any downstream consumer (MCP, exporter, UI). Exits non-zero on
# any violation — the CI gate contract (DEC-001 §2, exit 1).
#
# Two scopes, on purpose:
#   * Required-KEYS check — specs, patches, spikes. Schema-specific, so it stays
#     scoped to the lanes whose schema it knows.
#   * Front-matter PARSE check (v0.6.41) — every *.md under projects/ and the
#     decisions dir, handoffs included. It only asks whether the front matter
#     parses at all. The key check never parsed, so a file no parser could read
#     was reported valid (irradiance SPEC-010). Uses ruby when present, else a
#     weaker bash check, and says which (see front_matter_parse_report).
# Cost recording on SHIPPED specs is a separate gate (`just cost-audit`);
# decision records are linted by `just decisions-audit`.
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
parsed=0
off_names=(); off_problems=(); off_kinds=(); off_messages=()
activity_notes=""
depends_notes=""
actual_notes=""
roadmap_notes=""
verdict_notes=""
basis_notes=""
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
        for vr in $(get_spec_verify_rounds "$f"); do
            case "$VALID_VERDICT" in
                *" $vr "*) : ;;
                *) verdict_notes="${verdict_notes}    ${name}: verify_rounds has '${vr}'"$'\n' ;;
            esac
        done

        # Cost session `basis:` — open set, advisory (docs/cost-tracking.md "Basis").
        while IFS= read -r cb; do
            [ -n "$cb" ] || continue
            case "$SUGGESTED_COST_BASIS" in
                *" $cb "*) : ;;
                *) basis_notes="${basis_notes}    ${name}: basis='${cb}'"$'\n' ;;
            esac
        done < <(list_cost_session_bases "$f" | sort -u)

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
            off_names+=("$name"); off_problems+=("$problems"); off_kinds+=("spec"); off_messages+=("")
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
            off_names+=("$name"); off_problems+=("$problems"); off_kinds+=("patch"); off_messages+=("")
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
    # `|| true`: a project folder with no brief.md made awk exit 2 here, which
    # under set -e ended validate with exit 2 and no message.
    pname=$(basename "$pdir")
    cr=$(awk '
        /^---$/ { f = !f; next } !f { next }
        /^closed_reason:/ { v = $2; if (v != "null" && v != "") print v; exit }
    ' "${pdir}/brief.md" 2>/dev/null || true)
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
        off_names+=("$name"); off_problems+=("$problems"); off_kinds+=("spike"); off_messages+=("")
        [ "$JSON_OUT" = 1 ] || printf "  %-52s invalid/missing:%s\n" "$name" "$problems"
        offenders=$((offenders + 1))
    fi
done < <(find_all_spikes)

# ── Front-matter PARSE gate (SPEC-016 AC1) ──────────────────────────────────
# The checks above confirm required KEYS are present (line-oriented awk); they
# never PARSE, so front matter that no YAML parser can read was reported valid.
# Parse every artifact's front matter with a real parser and fail loudly. Scope
# is BROADER than the key checks (which know only the spec/patch/spike schema):
# every *.md under projects/ and the decisions dir — handoffs/ and patches/ live
# under projects/, so this covers them, and it is what surfaces the live
# falsifier HANDOFF-039 that the old specs-only scope let slip. A parse failure
# names the file and prints the parser's own message (DEC-001 §1).
#
# ⚠ A gate that cannot run its parser must not report green (the mute-gate class
# this whole spec exists to close), so a missing parser is a LOUD failure, not a
# skip.
if ! command -v ruby >/dev/null 2>&1; then
    die "validate: the front-matter parse gate needs 'ruby' (ships with macOS and the GitHub runners) and it is not on PATH. Refusing to report green on an unparsed check — install ruby, or move the gate to a parser that is present (SPEC-016 AC1)."
fi
# ── Front-matter PARSE check ────────────────────────────────────────────────
# Runs the parser to a temp file with its exit code captured — NOT in a
# `done < <(…)` loop, which hides a crash from `set -e` and would report green.
# Two more guards against a check that silently didn't run: the parser must
# report having examined every file it was given (SCANNED), and a non-zero
# exit with no FAIL line is a crash, not a verdict.
PARSER=$(fm_parser)
parse_targets=()
while IFS= read -r f; do
    [ -n "$f" ] && parse_targets+=("$f")
done < <(find "${REPO_ROOT}/projects" "$(decisions_dir)" -type f -name '*.md' 2>/dev/null | sort)
if [ "${#parse_targets[@]}" -gt 0 ]; then
    parse_report="$(mktemp)"
    set +e
    front_matter_parse_report "${parse_targets[@]}" > "$parse_report" 2>&1; parse_rc=$?
    set -e
    scanned=0
    parse_fails=0
    while IFS=$'\t' read -r tag a b; do
        case "$tag" in
            SCANNED) scanned="$a" ;;
            PARSED)  parsed="$a" ;;
            FAIL)
                pname=$(basename "$a" .md)
                off_names+=("$pname"); off_problems+=(" front-matter-did-not-parse")
                off_kinds+=("parse"); off_messages+=("$b")
                [ "$JSON_OUT" = 1 ] || printf "  %-52s front matter did not parse: %s\n" "$pname" "$b"
                offenders=$((offenders + 1))
                parse_fails=$((parse_fails + 1))
                ;;
        esac
    done < "$parse_report"
    parse_tail=$(tail -n 3 "$parse_report" | tr '\n' ' ')
    rm -f "$parse_report"
    if [ "$scanned" -ne "${#parse_targets[@]}" ]; then
        die "validate: the ${PARSER} front-matter check examined ${scanned} of ${#parse_targets[@]} file(s) before stopping, so it did not finish. Refusing to report green. Last output: ${parse_tail}"
    fi
    if [ "$parse_rc" -ne 0 ] && [ "$parse_fails" -eq 0 ]; then
        die "validate: the ${PARSER} front-matter check exited ${parse_rc} without naming an unparseable file, so it failed for some other reason. Refusing to report green. Last output: ${parse_tail}"
    fi
fi

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
        # Parse failures carry the parser's own message; other offenders have none.
        if [ -n "${off_messages[$i]}" ]; then
            items+=("$(json_obj \
                artifact "$(json_qs "${off_names[$i]}")" \
                kind "$(json_qs "${off_kinds[$i]}")" \
                problems "$parr" \
                message "$(json_qs "${off_messages[$i]}")")")
        else
            items+=("$(json_obj \
                artifact "$(json_qs "${off_names[$i]}")" \
                kind "$(json_qs "${off_kinds[$i]}")" \
                problems "$parr")")
        fi
        i=$((i + 1))
    done
    [ "${#items[@]}" -gt 0 ] && arr=$(json_arr "${items[@]}") || arr="[]"
    json_emit validate "$(json_obj \
        checked "$checked" parsed "$parsed" parser "$(json_qs "$PARSER")" offenders "$offenders" ok "$([ "$offenders" -eq 0 ] && echo true || echo false)" \
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
    warn "validate: unrecognized verify verdict (advisory — expected one of${VALID_VERDICT}):"
    printf '%s' "$verdict_notes"
fi

# Advisory: an unrecognized cost-session `basis` (open set). Never changes the exit code.
if [ -n "$basis_notes" ]; then
    warn "validate: unrecognized cost basis (advisory — open set, documented:${SUGGESTED_COST_BASIS}):"
    printf '%s' "$basis_notes"
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

# Advisory: no YAML parser, so the parse check was the weaker bash heuristic.
if [ "$PARSER" = bash ]; then
    warn "validate: front matter was checked with the bash heuristic (no ruby on PATH, or FM_PARSER=bash), which catches unclosed quotes and tab indentation only. Install ruby for a real parse."
fi

if [ "$offenders" -gt 0 ]; then
    echo ""
    die "validate: ${offenders} artifact(s) failed front-matter validation: ${checked} checked for required keys (specs + patches + spikes), ${parsed} parsed (${PARSER}) under projects/ + decisions/. See DEC-001 §1 / docs/schema-reference.md."
fi
success "validate: ${checked} artifact(s) (specs + patches + spikes) have valid required front-matter; ${parsed} artifact(s) under projects/ + decisions/ parse (${PARSER})."
