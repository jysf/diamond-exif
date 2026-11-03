#!/usr/bin/env bash
# scripts/archive-spec.sh — move a shipped spec to done/ and update stage backlog.
# Usage: archive-spec.sh SPEC-NNN

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/_lib.sh"

require_initialized

SPEC_ID="${1:-}"

if [ -z "$SPEC_ID" ]; then
    die "Usage: just archive-spec SPEC-NNN"
fi

SPEC_FILE=$(find_spec "$SPEC_ID")
if [ -z "$SPEC_FILE" ]; then
    die "Spec not found: ${SPEC_ID}"
fi

# Check cycle is ship
CYCLE=$(awk '/^---$/{f=!f; next} f && /^[[:space:]]+cycle:/{print $2; exit}' "$SPEC_FILE" 2>/dev/null || echo "")
if [ "$CYCLE" != "ship" ]; then
    warn "Spec cycle is '${CYCLE}', not 'ship'. Continue anyway? [y/N]"
    read -r answer
    if [ "$answer" != "y" ] && [ "$answer" != "Y" ]; then
        echo "Aborted."
        exit 0
    fi
fi

SPEC_DIR=$(dirname "$SPEC_FILE")
DONE_DIR="${SPEC_DIR}/done"
mkdir -p "$DONE_DIR"

SPEC_BASENAME=$(basename "$SPEC_FILE")
TARGET="${DONE_DIR}/${SPEC_BASENAME}"

mv "$SPEC_FILE" "$TARGET"
success "Archived: ${SPEC_FILE} → ${TARGET}"

# Stamp a top-level `shipped_at: DATE` into the front-matter (harvest signal #3).
# Ship dates previously lived only in git tags / timeline / cost blocks, so
# per-spec cycle-time and time-to-value weren't computable from the spec itself.
# Pairs with the `created_at` the scaffold already records. Idempotent: skip if
# one is already present (archive-spec won't re-run on an archived spec anyway).
SHIP_DATE=$(today)
if ! grep -qE '^shipped_at:' "$TARGET"; then
    awk -v d="$SHIP_DATE" '
        /^---$/ { fm++; if (fm == 2) print "shipped_at: " d; print; next }
        { print }
    ' "$TARGET" > "${TARGET}.tmp" && mv "${TARGET}.tmp" "$TARGET"
fi

# Recompute cost.totals from the recorded sessions so the rollup is never stale
# (the non-judgment-laden half of ship bookkeeping — dogfood harvest).
write_cost_totals "$TARGET"

# Ship is the only moment the ACTUAL size is knowable, so it's the only moment
# worth asking. Advisory — an unrecorded actual never blocks an archive; the
# estimation loop is a feedback loop, not a gate.
if [ -z "$(get_spec_complexity_actual "$TARGET")" ]; then
    EXPECTED=$(get_spec_complexity "$TARGET")
    echo "${DIM}No task.complexity_actual recorded (expected was '${EXPECTED:-?}'). Stamp what it"
    echo "  actually took — XS|S|M|L|XL|XXL — so \`just calibration\` can show your drift.${RESET}"
fi

# Co-archive the timeline file if one exists. The timeline is an
# artifact of this spec's cycle history and belongs next to the spec
# it describes.
TIMELINE_FILE=$(find_spec_timeline "$SPEC_ID")
if [ -n "$TIMELINE_FILE" ]; then
    TIMELINE_TARGET="${DONE_DIR}/$(basename "$TIMELINE_FILE")"
    mv "$TIMELINE_FILE" "$TIMELINE_TARGET"
    success "Archived timeline: ${TIMELINE_FILE} → ${TIMELINE_TARGET}"
fi

# Try to update the parent stage's backlog.
# Get the stage ID from the spec's front-matter (project.stage field).
STAGE_ID=$(awk '/^---$/{f=!f; next} f && /^[[:space:]]+stage:/{print $2; exit}' "$TARGET" 2>/dev/null || echo "")
if [ -n "$STAGE_ID" ]; then
    STAGE_FILE=$(find_stage "$STAGE_ID")
    if [ -n "$STAGE_FILE" ]; then
        echo ""
        echo "Parent stage: ${STAGE_ID} (${STAGE_FILE})"
        # SHIP_DATE was computed above (shipped_at stamp) and is reused here.
        # Is this spec listed as an open "[ ] SPEC-NNN" item in the backlog?
        HAS_ENTRY=$(awk -v sid="$SPEC_ID" '
            /^## Spec Backlog/ { inbl=1; next }
            /^## / { if (inbl) inbl=0 }
            inbl && $0 ~ ("^-[[:space:]]*\\[[[:space:]]\\][[:space:]]*" sid "([^0-9]|$)") { print "yes"; exit }
        ' "$STAGE_FILE")
        if [ "$HAS_ENTRY" = "yes" ]; then
            # Flip that entry to "[x] … (shipped on DATE)" and recompute the
            # **Count:** line from the (updated) backlog — the bookkeeping the
            # help text used to only *describe*. Scoped to the Spec Backlog
            # section so it never touches look-alike lines elsewhere.
            awk -v sid="$SPEC_ID" -v date="$SHIP_DATE" '
                /^## Spec Backlog/ { inbl=1; print; next }
                /^## / { if (inbl) inbl=0 }
                {
                    if (inbl && $0 ~ /^-[[:space:]]*\[/) {
                        if ($0 ~ ("^-[[:space:]]*\\[[[:space:]]\\][[:space:]]*" sid "([^0-9]|$)")) {
                            sub(/\[[[:space:]]\]/, "[x]")
                            if ($0 ~ /\([^)]*\)/) sub(/\([^)]*\)/, "(shipped on " date ")")
                            else $0 = $0 " (shipped on " date ")"
                        }
                        if ($0 ~ /^-[[:space:]]*\[x\]/) shipped++
                        else if ($0 ~ /SPEC-[0-9]/) active++
                        else pending++
                        print; next
                    }
                    if (inbl && $0 ~ /^\*\*Count:\*\*/) {
                        printf "**Count:** %d shipped / %d active / %d pending\n", shipped, active, pending
                        next
                    }
                    print
                }
            ' "$STAGE_FILE" > "${STAGE_FILE}.tmp" && mv "${STAGE_FILE}.tmp" "$STAGE_FILE"
            success "Updated ${STAGE_ID} backlog: ${SPEC_ID} → shipped; **Count:** recomputed."
        else
            echo "${DIM}${SPEC_ID} isn't an open '[ ] ${SPEC_ID}' item in the backlog —"
            echo "  add or update it by hand if needed (e.g. '[x] ${SPEC_ID} (shipped on ${SHIP_DATE})').${RESET}"
        fi
    fi
fi

# If this leaves no active specs under the stage, surface that as an
# observation — NOT as a claim that the stage is complete. The stage's
# `## Spec Backlog` may still list unwritten specs, and we can't
# reliably read that list (it's manually maintained markdown).
if [ -n "$STAGE_ID" ]; then
    REMAINING=$(find "$SPEC_DIR" -maxdepth 1 -name "SPEC-*.md" 2>/dev/null \
                | xargs -I{} awk -v sid="$STAGE_ID" '/^---$/{f=!f; next} f && /^[[:space:]]+stage:/ && $2 == sid {print FILENAME; exit}' {} \
                | wc -l | tr -d ' ')
    if [ "$REMAINING" = "0" ]; then
        echo ""
        echo "${GREEN}No active specs remain for ${STAGE_ID}.${RESET}"
        echo "If the stage's Spec Backlog is fully complete, run the Stage"
        echo "Ship prompt (Prompt 1c) in FIRST_SESSION_PROMPTS.md."
    fi
fi
