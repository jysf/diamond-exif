#!/usr/bin/env bash
# scripts/dash.sh — one read command, many lenses (DEC-001 §4).
#
# The antidote to view sprawl: the now/next/future/ledger lenses ARE the
# existing read views (which keep working as their own commands — permanent
# aliases), and `just dash` with no argument stitches a single overview.
#
# The rule this enforces: when you want a slightly different slice, add a LENS
# here — never a new script.
#
#   just dash            stitched dashboard: now + future + cost + governance flags
#   just dash now        where are things now?            (= just status)
#   just dash next       what are we NOT working on next? (= just backlog)
#   just dash future     what's coming?                   (= just roadmap)
#   just dash ledger     every spec, all history          (= just specs-by-stage)
#   just dash decisions  browse DEC-* (confidence, superseded, scope)
#   just dash questions  open questions (what's blocking)
#   just dash signals    the typed feedback ledger (what's queued / un-adopted)
#   just dash patches    the patch lane by cycle (DEC-003)
#   just dash spikes     the spike lane by cycle (DEC-012) — repo-level
#   just dash defects    where defects are caught (ship Q4) — the escape distribution
#   just dash constraints repo-level rules by severity (guidance/constraints.yaml)
#   just dash handoffs   delegation handoffs by status (plus-agents)
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/_lib.sh"

require_initialized

# Lenses pass any remaining flags (e.g. --json) through to the underlying view.
lens="${1:-}"
case "$lens" in
    now)       shift; exec "${SCRIPT_DIR}/status.sh" "$@" ;;
    next)      shift; exec "${SCRIPT_DIR}/backlog.sh" "$@" ;;
    future)    shift; exec "${SCRIPT_DIR}/roadmap.sh" "$@" ;;
    ledger)    shift; exec "${SCRIPT_DIR}/specs-by-stage.sh" "$@" ;;
    decisions)   shift; exec "${SCRIPT_DIR}/decisions-view.sh" "$@" ;;
    questions)   shift; exec "${SCRIPT_DIR}/questions-view.sh" "$@" ;;
    signals)     shift; exec "${SCRIPT_DIR}/signals-view.sh" "$@" ;;
    patches)     shift; exec "${SCRIPT_DIR}/patches-view.sh" "$@" ;;
    spikes)      shift; exec "${SCRIPT_DIR}/spikes-view.sh" "$@" ;;
    defects)     shift; exec "${SCRIPT_DIR}/defects-view.sh" "$@" ;;
    constraints) shift; exec "${SCRIPT_DIR}/constraints-view.sh" "$@" ;;
    handoffs)    shift; exec "${SCRIPT_DIR}/handoffs-view.sh" "$@" ;;
    help|-h|--help)
        cat <<'EOF'
just dash [lens] [--json]
  (no lens)   stitched dashboard: now + future + recorded cost + governance flags
  now         where are things now?             (= just status)
  next        what are we NOT working on next?  (= just backlog)
  future      what's coming?                    (= just roadmap)
  ledger      every spec, all history           (= just specs-by-stage)
  decisions   browse DEC-* (confidence, active/superseded, scope)
  questions   open questions from guidance/questions.yaml (what's blocking)
  signals     the typed feedback ledger (guidance/signals.yaml) — what's queued / un-adopted
  patches     the patch lane by cycle (patch|verify|ship), DEC-003
  spikes      the spike lane by cycle (spike|land), DEC-012 — repo-level, flags un-landed
  defects     defect-escape distribution from ship Reflection Q4 (all projects; --active to scope)
  constraints repo-level rules from guidance/constraints.yaml, by severity
  handoffs    delegation handoffs (HANDOFF-*.md) by status (plus-agents)
  --json      machine-readable output (works on the dashboard and every lens)
EOF
        exit 0 ;;
    ""|--json) : ;;  # no lens → stitched dashboard (human or, with --json, JSON)
    *)      die "Unknown lens: '$lens' (use: now | next | future | ledger | decisions | questions | signals | patches | spikes | defects | constraints | handoffs | help, or no arg for the dashboard)" ;;
esac

project=$(get_active_project)

# --- Default dashboard: JSON (stitches the status + roadmap reports + cost) ---
if [ "$(has_json_flag "$@")" = 1 ]; then
    now_json=$("${SCRIPT_DIR}/status.sh" --json)
    future_json=$("${SCRIPT_DIR}/roadmap.sh" --json)
    pdir="${REPO_ROOT}/projects/${project}"
    tot_usd="0.00"; tot_tok=0
    while IFS= read -r f; do
        [ -n "$f" ] || continue
        case "$f" in *-timeline.md|*/prompts/*) continue ;; esac
        u=$(sum_cost_usd_for_spec "$f"); t=$(sum_cost_tokens_for_spec "$f")
        tot_usd=$(awk -v a="$tot_usd" -v b="$u" 'BEGIN{printf "%.2f", a+b}')
        tot_tok=$((tot_tok + t))
    done < <(find_all_specs "$pdir")
    cost=$(json_obj "cost.tokens_total" "$tot_tok" "cost.estimated_usd" "$tot_usd")
    flags=$(json_obj open_questions "$(count_open_questions)" low_confidence_decisions "$(count_low_confidence_decisions)" open_signals "$(count_open_signals)")
    data=$(json_obj now "$now_json" future "$future_json" recorded_cost "$cost" flags "$flags")
    json_emit dash "$data"
    exit 0
fi

# --- Default: the stitched dashboard (human) -------------------------------
printf "${BOLD}=== Dashboard — %s ===${RESET}\n\n" "$project"

printf "${BOLD}▸ Now${RESET} ${DIM}(just dash now)${RESET}\n"
"${SCRIPT_DIR}/status.sh"
echo

printf "${BOLD}▸ Future${RESET} ${DIM}(just dash future)${RESET}\n"
"${SCRIPT_DIR}/roadmap.sh"
echo

# Recorded cost across the active project (reuses the cost lib — same numbers
# as `just specs-by-stage`'s grand total).
pdir="${REPO_ROOT}/projects/${project}"
tot_usd="0.00"; tot_tok=0
while IFS= read -r f; do
    [ -n "$f" ] || continue
    case "$f" in *-timeline.md) continue ;; esac
    u=$(sum_cost_usd_for_spec "$f"); t=$(sum_cost_tokens_for_spec "$f")
    tot_usd=$(awk -v a="$tot_usd" -v b="$u" 'BEGIN{printf "%.2f", a+b}')
    tot_tok=$((tot_tok + t))
done < <(find_all_specs "$pdir")
printf "${BOLD}▸ Recorded cost${RESET}  \$%s · %s tokens  ${DIM}(just dash ledger for the full ledger)${RESET}\n" "$tot_usd" "$tot_tok"
echo

# Governance flags — things that should nag you, surfaced where you look.
oq=$(count_open_questions)
lcd=$(count_low_confidence_decisions)
os=$(count_open_signals)
if [ "$oq" -gt 0 ] || [ "$lcd" -gt 0 ] || [ "$os" -gt 0 ]; then
    printf "${BOLD}▸ Flags${RESET}  ${YELLOW}⚠${RESET} %s open question(s) · %s decision(s) at confidence <0.7 · %s signal(s) awaiting disposition  ${DIM}(just dash questions | decisions | signals)${RESET}\n" "$oq" "$lcd" "$os"
else
    printf "${BOLD}▸ Flags${RESET}  ${DIM}none — no open questions, no low-confidence decisions, no signals awaiting disposition${RESET}\n"
fi
