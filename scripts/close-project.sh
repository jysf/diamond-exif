#!/usr/bin/env bash
# scripts/close-project.sh — the mechanical half of the project-close ritual.
#
# WHY THIS EXISTS. Close was never "thin" — Prompt 1e is seven-plus steps and its
# signal-disposition walk is one of the more rigorous rituals here. The gap was
# an ASYMMETRY: close was the only major ritual with no command behind it.
#
#   spec ship   → archive-spec    stamps shipped_at, computes cost.totals, audits cost
#   patch ship  → archive-patch
#   spike land  → archive-spike   REFUSES without spike.outcome
#   stage close → (nothing)
#   proj close  → (nothing)   ← this file
#
# So the spike lane refuses to land without an outcome, while a project close
# would happily flip a status with specs in flight, an empty reflection, and
# signals silently carried. This closes the asymmetry; it does NOT rewrite the
# ritual, which stays in FIRST_SESSION_PROMPTS.md Prompt 1e.
#
# THREE HARD REFUSALS, and the principle behind them is the spike lane's:
# refuse when the missing thing is the exact failure the ritual exists to
# prevent; warn otherwise.
#
#   1. Specs still in flight — but ONLY when you claim `shipped`. In-flight
#      specs contradict "shipped" and corrupt every rollup computed over them.
#      Under `abandoned`/`parked` they are expected, so the refusal lifts. That
#      makes closed_reason load-bearing rather than decorative.
#   2. An empty Project-Level Reflection — the direct analogue of spike.outcome.
#      The reflection is the artifact close exists to produce.
#   3. Open signals owned by project-close. The v0.5.18 ritual says these get
#      dispositioned at close and nothing enforced it. `defer-with-trigger` is a
#      legal disposition, so the refusal always has an honest escape hatch.
#
# Everything else warns: a missing closed_reason, an empty value.thesis. Refusing
# on a null thesis would punish exploratory projects, which the value model
# explicitly allows.
#
# Usage:
#   just close-project PROJ-NNN [--dry-run] [--json]
#
# Exit status: 0 closed (or dry-run clean) · 1 refused · 2 usage error.

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/_lib.sh
source "${SCRIPT_DIR}/_lib.sh"

require_initialized

PROJECT_ID=""
DRY_RUN=0
JSON_OUT=$(has_json_flag "$@")
for a in "$@"; do
    case "$a" in
        --dry-run) DRY_RUN=1 ;;
        --json)    : ;;
        PROJ-*)    PROJECT_ID="$a" ;;
        '')        : ;;
        *)         usage_error "just close-project PROJ-NNN [--dry-run] [--json]" ;;
    esac
done
[ -n "$PROJECT_ID" ] || usage_error "just close-project PROJ-NNN [--dry-run] [--json]"

PDIR=$(resolve_project_dir "$PROJECT_ID") || true
[ -n "$PDIR" ] && [ -d "$PDIR" ] || die "No such project: ${PROJECT_ID}"
BRIEF="${PDIR}/brief.md"
[ -f "$BRIEF" ] || die "No brief at ${BRIEF#"${REPO_ROOT}/"}"

# --- Gather -----------------------------------------------------------------

status=$(get_project_status "$PDIR")
closed_reason=$(awk '
    /^---$/ { f = !f; next } !f { next }
    /^closed_reason:/ { v = $2; if (v != "null" && v != "") print v; exit }
' "$BRIEF")
created_at=$(awk '/^---$/{f=!f;next} !f{next} /^created_at:/{print $2; exit}' "$BRIEF")
shipped_at=$(awk '/^---$/{f=!f;next} !f{next} /^shipped_at:/{v=$2; if(v!="null"&&v!="")print v; exit}' "$BRIEF")
thesis=$(awk '
    /^---$/ { f = !f; next } !f { next }
    /^value:/ { inv = 1; next }
    inv && /^[a-zA-Z_]/ { inv = 0 }
    inv && /^[[:space:]]+thesis:/ {
        v = $0; sub(/^[[:space:]]+thesis:[[:space:]]*/, "", v)
        gsub(/^["'\'']|["'\'']$/, "", v)
        if (v != "null" && v != "") print v
        exit
    }
' "$BRIEF")

# The realized half — the answer to the thesis, recorded rather than remembered.
thesis_held=$(awk '
    /^---$/ { f = !f; next } !f { next }
    /^value_realized:/ { inr = 1; next }
    inr && /^[a-zA-Z_]/ { inr = 0 }
    inr && /^[[:space:]]+thesis_held:/ {
        v = $2; if (v != "null" && v != "") print v; exit
    }
' "$BRIEF")
realized_evidence=$(awk '
    /^---$/ { f = !f; next } !f { next }
    /^value_realized:/ { inr = 1; next }
    inr && /^[a-zA-Z_]/ { inr = 0 }
    inr && /^[[:space:]]+evidence:/ {
        v = $0; sub(/^[[:space:]]+evidence:[[:space:]]*/, "", v)
        gsub(/^["'\'']|["'\'']$/, "", v)
        if (v != "null" && v != "") print v
        exit
    }
' "$BRIEF")

# In-flight specs: anything not yet shipped/archived.
in_flight=0; in_flight_ids=""
while IFS= read -r f; do
    [ -n "$f" ] || continue
    case "$f" in */prompts/*|*-timeline.md|*/done/*) continue ;; esac
    case "$(get_spec_cycle "$f")" in
        frame|design|build|verify)
            in_flight=$((in_flight + 1))
            in_flight_ids="${in_flight_ids} $(basename "$f" .md | sed -E 's/^(SPEC-[0-9]+).*/\1/')"
            ;;
    esac
done < <(find_all_specs "$PDIR")

# Is the Project-Level Reflection actually filled in? The template ships the
# section with `<...>` placeholders, so "has the heading" proves nothing —
# check for prose that isn't a placeholder.
reflection_filled=$(awk '
    /^## Project-Level Reflection/ { inr = 1; next }
    inr && /^## / { inr = 0 }
    # Only BULLETS can be answers. A standalone italic line is section
    # commentary ("*To be filled in when this project ships.*") and counting it
    # as content is how a wholly-untouched reflection reads as complete.
    inr && /^[[:space:]]*-[[:space:]]/ {
        line = $0
        sub(/^[[:space:]]*-[[:space:]]*/, "", line)
        sub(/^\*\*[^*]*\*\*[[:space:]]*/, "", line)        # drop the bolded question
        gsub(/<[^>]*>/, "", line)                          # drop <placeholders>
        gsub(/[[:space:]]/, "", line)
        gsub(/[*_.:?—-]/, "", line)
        if (line != "") { found = 1 }
    }
    END { print found ? "yes" : "no" }
' "$BRIEF")

# Signals this close owns, still awaiting disposition.
open_signals=""; open_signal_count=0
while IFS=$'\t' read -r sid sty sst sda sbar ssum; do
    [ -n "$sid" ] || continue
    [ "$sda" = "project-close" ] || continue
    case "$sst" in
        open|watch)
            open_signal_count=$((open_signal_count + 1))
            open_signals="${open_signals}    ${sid} (${sty}, ${sst}) — ${ssum}"$'\n'
            ;;
    esac
done < <(emit_signals_tsv)

# --- Rollup numbers ----------------------------------------------------------
# Close is the ONLY moment the whole wave is knowable, which is why these are
# computed here and not on a dashboard.

spec_count=0; shipped_specs=0; proj_tokens=0; proj_usd="0.00"
while IFS= read -r f; do
    [ -n "$f" ] || continue
    case "$f" in */prompts/*|*-timeline.md) continue ;; esac
    spec_count=$((spec_count + 1))
    [ "$(get_spec_cycle "$f")" = "ship" ] && shipped_specs=$((shipped_specs + 1))
    proj_tokens=$((proj_tokens + $(sum_cost_tokens_for_spec "$f")))
    proj_usd=$(awk -v a="$proj_usd" -v b="$(sum_cost_usd_for_spec "$f")" 'BEGIN{printf "%.2f", a+b}')
done < <(find_all_specs "$PDIR" ; find "${PDIR}/specs/done" -maxdepth 1 -type f -name 'SPEC-*.md' 2>/dev/null)

orch_tokens=0
for s in "${PDIR}"/stages/STAGE-*.md; do
    [ -f "$s" ] || continue
    orch_tokens=$((orch_tokens + $(sum_orchestration_tokens_for_stage "$s")))
done

# Decision density × supersession. NEVER density alone — density alone is noise;
# the pair is the diagnostic. High density + low supersession reads as a novel
# domain well recorded; low density + high supersession is under-recording, and
# that is the dangerous cell precisely because it looks calm.
dec_count=0; dec_superseded=0
while IFS= read -r df; do
    [ -n "$df" ] || continue
    [ "$(get_dec_project_id "$df")" = "$PROJECT_ID" ] || continue
    dec_count=$((dec_count + 1))
    [ -n "$(get_dec_superseded_by "$df")" ] && dec_superseded=$((dec_superseded + 1))
done < <(find_all_decisions)
density="n/a"
[ "$shipped_specs" -gt 0 ] && density=$(awk -v d="$dec_count" -v s="$shipped_specs" 'BEGIN{printf "%.2f", d/s}')

today_str=$(today)
ttv="n/a"
if [ -n "$created_at" ]; then
    d=$(days_between "$created_at" "${shipped_at:-$today_str}")
    [ -n "$d" ] && ttv="$d"
fi

# --- Adjudicate --------------------------------------------------------------

refusals=(); warnings=()

# (1) In-flight specs — conditional on the ending being claimed.
claims_shipped=1
case "$closed_reason" in
    abandoned|superseded|parked) claims_shipped=0 ;;
esac
if [ "$in_flight" -gt 0 ] && [ "$claims_shipped" = 1 ]; then
    refusals+=("${in_flight} spec(s) still in flight:${in_flight_ids}
      A project cannot have shipped its thesis with work outstanding, and every
      rollup below is computed over an incomplete set. Ship or cancel them — or
      set 'closed_reason: abandoned' if that is the honest ending.")
fi

# (2) The reflection is the artifact close exists to produce.
if [ "$reflection_filled" != "yes" ]; then
    refusals+=("Project-Level Reflection is empty (only template placeholders).
      This is the spike-lane rule applied to projects: archive-spike refuses
      without spike.outcome for the same reason. Fill it in ${BRIEF#"${REPO_ROOT}/"}.")
fi

# (3) Undisposed signals owned by this close.
if [ "$open_signal_count" -gt 0 ]; then
    refusals+=("${open_signal_count} signal(s) awaiting a project-close disposition:
${open_signals}      Give each accept / reject-with-reason / defer-with-trigger and bump
      last_touched. 'defer' is a legal answer — silent carry is not.")
fi

# (4) You claimed a thesis AND you claim you shipped it — then say whether it
# held. Same shape as the in-flight rule: the refusal is conditional on the
# ending you claim, so it never blocks an abandoned or exploratory close.
# `too-early` is a legal answer, which keeps the hatch honest.
VALID_HELD=" yes partly no too-early "
if [ -n "$thesis" ] && [ "$claims_shipped" = 1 ] && [ -z "$thesis_held" ]; then
    refusals+=("value_realized.thesis_held is null on a project claiming 'shipped'.
      You wrote a thesis when you knew least; close is the only moment it can be
      scored, and an unanswered prediction is a wish, not a hypothesis. Answer
      yes | partly | no | too-early — 'too-early' is legitimate and honest.")
fi

# Warnings — real signal, never a gate.
[ -z "$closed_reason" ] && warnings+=("closed_reason is null — 'cancelled' flattens abandoned/superseded/parked, which destroys the most interesting signal: why work stopped.")
[ -z "$thesis" ] && warnings+=("value.thesis is null — nothing to compare the outcome against. Fine for an exploratory project; note it in the reflection.")
if [ -n "$thesis_held" ]; then
    case "$VALID_HELD" in
        *" $thesis_held "*) : ;;
        *) warnings+=("value_realized.thesis_held='${thesis_held}' is not one of${VALID_HELD}(advisory).") ;;
    esac
    [ -z "$realized_evidence" ] && warnings+=("thesis_held is answered but value_realized.evidence is null — the verdict without the observation behind it is the part that rots first.")
fi
[ "$dec_count" = 0 ] && [ "$shipped_specs" -gt 3 ] && warnings+=("${shipped_specs} specs shipped and 0 decisions attributed to ${PROJECT_ID} — either genuinely settled work, or decisions were made and lost. Only the supersession rate distinguishes those, and there is nothing to compute it from.")

# --- Emit --------------------------------------------------------------------

if [ "$JSON_OUT" = 1 ]; then
    rj=(); for r in "${refusals[@]:-}"; do [ -n "$r" ] && rj+=("$(json_qs "${r%%$'\n'*}")"); done
    wj=(); for w in "${warnings[@]:-}"; do [ -n "$w" ] && wj+=("$(json_qs "$w")"); done
    [ "${#rj[@]}" -gt 0 ] && rarr=$(json_arr "${rj[@]}") || rarr="[]"
    [ "${#wj[@]}" -gt 0 ] && warr=$(json_arr "${wj[@]}") || warr="[]"
    json_emit close-project "$(json_obj \
        "project.id" "$(json_qs "$PROJECT_ID")" \
        "project.status" "$(json_qs "$status")" \
        closed_reason "$([ -n "$closed_reason" ] && json_qs "$closed_reason" || printf null)" \
        thesis_held "$([ -n "$thesis_held" ] && json_qs "$thesis_held" || printf null)" \
        dry_run "$([ "$DRY_RUN" = 1 ] && printf true || printf false)" \
        ok "$([ "${#refusals[@]}" -eq 0 ] && printf true || printf false)" \
        refusals "$rarr" warnings "$warr" \
        specs_total "$spec_count" specs_shipped "$shipped_specs" specs_in_flight "$in_flight" \
        tokens_total "$proj_tokens" orchestration_tokens "$orch_tokens" \
        estimated_usd "$proj_usd" \
        decisions "$dec_count" decisions_superseded "$dec_superseded" \
        decision_density "$([ "$density" = "n/a" ] && printf null || printf '%s' "$density")" \
        time_to_value_days "$([ "$ttv" = "n/a" ] && printf null || printf '%s' "$ttv")")"
    [ "${#refusals[@]}" -gt 0 ] && exit 1
    exit 0
fi

printf "${BOLD}Closing %s${RESET}  ${DIM}(status: %s%s)${RESET}\n" \
    "$PROJECT_ID" "$status" "${closed_reason:+, reason: $closed_reason}"
echo ""

if [ "${#refusals[@]}" -gt 0 ]; then
    printf "${RED}REFUSED${RESET} — %d blocking issue(s):\n\n" "${#refusals[@]}"
    n=1
    for r in "${refusals[@]}"; do
        printf "  ${BOLD}%d.${RESET} %s\n\n" "$n" "$r"
        n=$((n + 1))
    done
    die "close-project: ${PROJECT_ID} is not ready to close."
fi

for w in "${warnings[@]:-}"; do [ -n "$w" ] && warn "$w"; done
[ "${#warnings[@]}" -gt 0 ] && echo ""

printf "${BOLD}Rollup${RESET}\n"
printf "  specs            %d shipped / %d total\n" "$shipped_specs" "$spec_count"
printf "  cost             %s tokens, ~\$%s\n" "$proj_tokens" "$proj_usd"
if [ "$orch_tokens" -gt 0 ]; then
    pct=$(awk -v o="$orch_tokens" -v t="$proj_tokens" 'BEGIN{ if (o+t>0) printf "%d", o*100/(o+t); else print 0 }')
    printf "  orchestration    %s tokens (%s%% of total — the spend with no spec)\n" "$orch_tokens" "$pct"
else
    printf "  orchestration    ${DIM}none recorded (stage orchestration_cost is empty)${RESET}\n"
fi
printf "  time to value    %s days ${DIM}(created_at → close)${RESET}\n" "$ttv"

# Density × supersession, together. The pair is the diagnostic; either alone
# misleads, which is why they are printed on one line with the reading.
printf "  decisions        %d (%d superseded) · density %s per shipped spec\n" \
    "$dec_count" "$dec_superseded" "$density"
if [ "$dec_count" -gt 0 ] && [ "$density" != "n/a" ]; then
    sup_pct=$(awk -v s="$dec_superseded" -v d="$dec_count" 'BEGIN{printf "%d", s*100/d}')
    hi_d=$(awk -v x="$density" 'BEGIN{print (x+0 >= 0.5) ? 1 : 0}')
    hi_s=$(awk -v x="$sup_pct" 'BEGIN{print (x+0 >= 20) ? 1 : 0}')
    if   [ "$hi_d" = 1 ] && [ "$hi_s" = 0 ]; then reading="novel domain, well recorded"
    elif [ "$hi_d" = 1 ] && [ "$hi_s" = 1 ]; then reading="thrash — the architecture won't settle"
    elif [ "$hi_d" = 0 ] && [ "$hi_s" = 0 ]; then reading="settled, executing"
    else reading="UNDER-RECORDING — decisions being made and lost"; fi
    printf "                   ${DIM}%d%% superseded → %s${RESET}\n" "$sup_pct" "$reading"
fi
echo ""

if [ -n "$thesis" ]; then
    printf "${BOLD}Predicted vs realized${RESET} ${DIM}(close is the only moment this is knowable)${RESET}\n"
    printf "  predicted:       %s\n" "$thesis"
    if [ -n "$thesis_held" ]; then
        printf "  held?            ${BOLD}%s${RESET}\n" "$thesis_held"
        [ -n "$realized_evidence" ] && printf "  evidence:        %s\n" "$realized_evidence"
    else
        printf "  held?            ${DIM}unanswered${RESET}\n"
    fi
    printf "  ${DIM}The template records the verdict; it does not compute it. That\n"
    printf "  judgement is the point, and it is yours.${RESET}\n"
    echo ""
fi

if [ "$DRY_RUN" = 1 ]; then
    printf "${DIM}--dry-run: nothing written. Would stamp shipped_at: %s%s.${RESET}\n" \
        "$today_str" "${closed_reason:+ and keep closed_reason: $closed_reason}"
    exit 0
fi

if [ -z "$shipped_at" ]; then
    upsert_frontmatter_scalar "$BRIEF" "shipped_at" "$today_str"
    success "Stamped shipped_at: ${today_str}"
else
    info "shipped_at already set (${shipped_at}) — left as is."
fi

success "close-project: ${PROJECT_ID} closed."
echo ""
echo "Next:"
echo "  1. Set project.status (shipped | cancelled | on_hold) in ${BRIEF#"${REPO_ROOT}/"}."
echo "     ${DIM}Deliberately NOT automated: which coarse state this is remains a"
echo "     judgement call, and closed_reason above carries the interesting half.${RESET}"
echo "  2. Work Prompt 1e in FIRST_SESSION_PROMPTS.md if you haven't."
