#!/usr/bin/env bash
# scripts/archive-spike.sh — move a landed spike to spikes/done/ (DEC-012).
#
# Unlike archive-spec there is NO stage bookkeeping, and unlike archive-patch the
# spike lives at the REPO root (a spike may precede any project).
#
# Refuses to archive a spike with no `spike.outcome` — an un-landed spike is the
# exact failure this lane exists to prevent (undocumented decisions leaking into
# production), so this is the one place the lane has real teeth.
#
# Usage: archive-spike.sh SPIKE-NNN
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/_lib.sh
source "${SCRIPT_DIR}/_lib.sh"

require_initialized

SPIKE_ID="${1:-}"
if [ -z "$SPIKE_ID" ]; then
    die "Usage: just archive-spike SPIKE-NNN"
fi

SPIKE_FILE=$(find_spec "$SPIKE_ID")
if [ -z "$SPIKE_FILE" ]; then
    die "Spike not found: ${SPIKE_ID}"
fi

OUTCOME=$(get_spike_field "$SPIKE_FILE" outcome)
case "$OUTCOME" in
    answered|inconclusive|graduated|discarded) ;;
    ""|null)
        die "${SPIKE_ID} has no spike.outcome — refusing to archive an UN-LANDED spike.

The land step is the entire point of the lane (DEC-012). Fill the ## Land section:
  - answer the question
  - emit DEC-* for the load-bearing choices this exploration already made
  - decide the code's fate

Then set spike.outcome to one of:
  answered      the question got an answer
  inconclusive  timebox hit, no answer (a real result — say what would make it answerable)
  graduated     the code becomes real work (complete the five-item contract)
  discarded     the code is thrown away (also a win — you bought an answer cheaply)"
        ;;
    *)
        die "${SPIKE_ID} has an unrecognized spike.outcome='${OUTCOME}'.
Must be one of: answered | inconclusive | graduated | discarded."
        ;;
esac

CYCLE=$(get_spec_cycle "$SPIKE_FILE")
if [ "$CYCLE" != "land" ]; then
    warn "Spike cycle is '${CYCLE}', not 'land'. Continue anyway? [y/N]"
    read -r answer
    case "$answer" in
        y|Y) ;;
        *) echo "Aborted. Run: just advance-cycle ${SPIKE_ID} land"; exit 0 ;;
    esac
fi

# Stamp landed_at if the land step left it null.
if [ -z "$(get_spike_field "$SPIKE_FILE" landed_at)" ] \
   || [ "$(get_spike_field "$SPIKE_FILE" landed_at)" = "null" ]; then
    sed_i() {
        if [ "$(uname)" = "Darwin" ]; then sed -i '' "$@"; else sed -i "$@"; fi
    }
    sed_i "s|^  landed_at: null.*|  landed_at: $(today)|" "$SPIKE_FILE"
fi

DONE_DIR="${REPO_ROOT}/spikes/done"
mkdir -p "$DONE_DIR"
TARGET="${DONE_DIR}/$(basename "$SPIKE_FILE")"
mv "$SPIKE_FILE" "$TARGET"
success "Archived: ${SPIKE_FILE} → ${TARGET}  (outcome: ${OUTCOME})"
write_cost_totals "$TARGET"

case "$OUTCOME" in
    graduated)
        echo ""
        echo "${DIM}Graduated. The five-item contract (DEC-012) — confirm each landed:${RESET}"
        echo "  .repo-context.yaml · AGENTS.md · guidance/toolchain-brief.md"
        echo "  retroactive DEC-* (load-bearing only) · a brief framed on what comes NEXT"
        echo "${DIM}Do NOT retro-write specs for code that already works.${RESET}"
        ;;
    discarded)
        echo ""
        echo "${DIM}Discarded — you bought an answer cheaply. That's the lane working.${RESET}"
        ;;
    inconclusive)
        echo ""
        echo "${DIM}Inconclusive is a real result. If you're tempted to extend the timebox a${RESET}"
        echo "${DIM}second time, this isn't a spike — frame it as a project.${RESET}"
        ;;
esac
