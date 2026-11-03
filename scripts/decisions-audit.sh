#!/usr/bin/env bash
# scripts/decisions-audit.sh — audit repo decisions for structural
# integrity and scope conflicts.
#
# This is a native, zero-dependency take on two ideas borrowed from
# LineSpec (https://linespec.dev) "Provenance Records":
#
#   1. Structural lint  — every DEC-* file is well-formed and its
#      supersession links are consistent (no dangling / one-sided
#      supersedes/superseded_by, no duplicate IDs).
#   2. Scope auditing   — using an OPTIONAL `affected_scope:` glob list
#      in a decision's front-matter, flag when two ACTIVE decisions
#      govern overlapping paths (review for contradiction), and, with
#      --changed, tell you which decisions govern the files you're
#      about to commit ("re-read DEC-007 before changing this").
#
# Pure bash + awk + git. No yq, no binary, no embeddings, no network.
# Targets bash 3.2 (macOS default): no mapfile, no associative arrays.
# `affected_scope` is optional: decisions without it are still linted,
# they're just skipped by the scope checks.
#
# Usage:
#   just decisions-audit                # lint + scope-overlap warnings
#   just decisions-audit --changed      # which decisions govern uncommitted changes
#   just decisions-audit --changed main # ...vs a base ref (diff base...HEAD)
#
# Exit status: 1 if structural errors are found (CI / pre-commit hook
# friendly). Scope-overlap and --changed output is advisory (exit 0).

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/_lib.sh"

# Works in an instance AND in the template repo itself (whose own DECs live in
# docs/decisions/) — auditing those is the whole point of the fallback.
require_decisions_context

DECISIONS_DIR="$(decisions_dir)"

# ---------------------------------------------------------------------
# Front-matter readers (decision-specific; kept local to this script).
# ---------------------------------------------------------------------

# insight.id — nested one level under `insight:`.
get_insight_id() {
    awk '
        /^---$/ { f = !f; next }
        !f { exit }
        /^insight:/ { i = 1; next }
        i && /^[a-zA-Z_]/ { i = 0 }
        i && /^[[:space:]]+id:/ { print $2; exit }
    ' "$1"
}

# project.id — nested one level under `project:`. Optional in the schema, but a
# decision with no project is an orphan in every per-project computation: the
# scale survey had to exclude 10 such DECs outright. Advisory, not a gate.
get_dec_project_id() {
    awk '
        /^---$/ { f = !f; next }
        !f { exit }
        /^project:/ { p = 1; next }
        p && /^[a-zA-Z_]/ { p = 0 }
        p && /^[[:space:]]+id:/ { v = $2; if (v != "null" && v != "") print v; exit }
    ' "$1"
}

# insight.type — nested one level under `insight:`.
get_insight_type() {
    awk '
        /^---$/ { f = !f; next }
        !f { exit }
        /^insight:/ { i = 1; next }
        i && /^[a-zA-Z_]/ { i = 0 }
        i && /^[[:space:]]+type:/ { print $2; exit }
    ' "$1"
}

# A top-level scalar (supersedes, superseded_by, created_at). Prints the
# raw value; callers decide whether "null" counts.
get_top_scalar() {
    local file="$1" key="$2"
    awk -v k="$key" '
        /^---$/ { f = !f; next }
        !f { exit }
        $0 ~ "^" k ":" { print $2; exit }
    ' "$file"
}

# affected_scope — a YAML list of globs at the top level. One glob per
# line on stdout. Empty if the key is absent or `[]`.
get_affected_scope() {
    awk '
        /^---$/ { f = !f; next }
        !f { exit }
        /^affected_scope:/ { s = 1; next }
        s && /^[a-zA-Z_]/ { s = 0 }
        s && /^[[:space:]]*-[[:space:]]/ {
            v = $0
            sub(/^[[:space:]]*-[[:space:]]*/, "", v)
            sub(/[[:space:]]*#.*$/, "", v)       # strip trailing comment
            sub(/^"/, "", v); sub(/"$/, "", v)   # strip double quotes
            sub(/^'\''/, "", v); sub(/'\''$/, "", v)  # strip single quotes
            sub(/[[:space:]]+$/, "", v)
            if (v != "" && v != "null") print v
        }
    ' "$1"
}

# ---------------------------------------------------------------------
# Glob helpers.
# ---------------------------------------------------------------------

# Convert a path glob to a regex body (no anchors). `**` matches across
# `/`, `*` matches within a path segment, `?` matches one char. Uses a
# placeholder token instead of \x01 so it works with BSD sed.
glob_to_regex() {
    printf '%s' "$1" \
        | sed -e 's/\./\\./g' \
              -e 's/\*\*/@@GLOBSTAR@@/g' \
              -e 's/\*/[^\/]*/g' \
              -e 's/@@GLOBSTAR@@/.*/g' \
              -e 's/?/./g'
}

# Literal directory prefix of a glob: everything before the first
# wildcard char, trailing slash removed. Empty for wildcard-leading
# globs (e.g. `**/*.ts`).
glob_prefix() {
    printf '%s' "$1" | sed -E -e 's/[*?[].*$//' -e 's#/+$##'
}

# Classify two globs' literal-prefix relationship:
#   "same" — equal prefix (two decisions rooted at the same path; worth a look)
#   "nest" — one prefix strictly contains the other (a broad decision that
#            deliberately contains a narrower one — intentional HIERARCHY, not a
#            conflict; downgraded to info so it doesn't flood the report)
#   ""     — no relationship (or a wildcard-leading glob with empty prefix;
#            --changed catches those precisely)
scope_relationship() {
    local a b
    a=$(glob_prefix "$1"); b=$(glob_prefix "$2")
    [ -z "$a" ] && { echo ""; return; }
    [ -z "$b" ] && { echo ""; return; }
    if [ "$a" = "$b" ]; then echo "same"; return; fi
    case "$a" in "$b"/*) echo "nest"; return ;; esac
    case "$b" in "$a"/*) echo "nest"; return ;; esac
    echo ""
}

# Is a decision "active"? Active = not superseded by another decision.
is_active() {
    local sb
    sb=$(get_top_scalar "$1" superseded_by)
    [ -z "$sb" ] || [ "$sb" = "null" ]
}

# ---------------------------------------------------------------------
# Collect decision files (bash 3.2: no mapfile).
# ---------------------------------------------------------------------

if [ ! -d "$DECISIONS_DIR" ]; then
    die "No decisions/ directory at repo root. Nothing to audit."
fi

DEC_FILES=()
while IFS= read -r f; do
    [ -n "$f" ] && DEC_FILES+=("$f")
done < <(find "$DECISIONS_DIR" -maxdepth 1 -type f -name 'DEC-*.md' \
         -not -name '_template.md' | sort)

if [ "${#DEC_FILES[@]}" -eq 0 ]; then
    info "No DEC-* files in decisions/ yet. Nothing to audit."
    exit 0
fi

# ---------------------------------------------------------------------
# Mode: --changed [BASE] — which decisions govern your pending changes.
# ---------------------------------------------------------------------

if [ "${1:-}" = "--changed" ]; then
    base="${2:-}"
    git rev-parse --is-inside-work-tree >/dev/null 2>&1 \
        || die "--changed needs a git repository."

    CHANGED=()
    if [ -n "$base" ]; then
        while IFS= read -r p; do
            [ -n "$p" ] && CHANGED+=("$p")
        done < <(git diff --name-only "${base}...HEAD" 2>/dev/null | sort -u)
        scope_desc="changes in ${base}...HEAD"
    else
        while IFS= read -r p; do
            [ -n "$p" ] && CHANGED+=("$p")
        done < <( {
            git diff --name-only
            git diff --name-only --cached
            git ls-files --others --exclude-standard
        } 2>/dev/null | sort -u )
        scope_desc="your uncommitted changes"
    fi

    if [ "${#CHANGED[@]}" -eq 0 ]; then
        info "No changed files in scope (${scope_desc})."
        exit 0
    fi

    echo "${BOLD}Decisions governing ${scope_desc}:${RESET}"
    echo

    hits=0
    for file in "${DEC_FILES[@]}"; do
        is_active "$file" || continue
        id=$(get_insight_id "$file")
        # The markdown H1 title; anchored to `# DEC-` so it can't pick
        # up a `#`-prefixed YAML comment inside the front-matter.
        title=$(grep -m1 '^# DEC-' "$file" | sed -E 's/^# //')
        globs=()
        while IFS= read -r g; do
            [ -n "$g" ] && globs+=("$g")
        done < <(get_affected_scope "$file")
        [ "${#globs[@]}" -eq 0 ] && continue

        matched=()
        for path in "${CHANGED[@]}"; do
            for g in "${globs[@]}"; do
                re="^$(glob_to_regex "$g")$"
                if [[ "$path" =~ $re ]]; then
                    matched+=("$path")
                    break
                fi
            done
        done

        if [ "${#matched[@]}" -gt 0 ]; then
            hits=$((hits + 1))
            warn "${BOLD}${id}${RESET} — ${title}"
            echo "      ${DIM}re-read this decision before committing; your change touches:${RESET}"
            for m in "${matched[@]}"; do
                echo "        ${m}"
            done
            echo
        fi
    done

    if [ "$hits" -eq 0 ]; then
        success "No active decision's affected_scope matches these changes."
    else
        echo "${DIM}Advisory only — confirm your change is consistent with each decision above,"
        echo "or supersede the decision if it no longer holds.${RESET}"
    fi
    exit 0
fi

# ---------------------------------------------------------------------
# Default mode: structural lint + scope-overlap warnings.
# bash 3.2 has no associative arrays, so the id->file map is two
# parallel indexed arrays with a linear-scan lookup.
# ---------------------------------------------------------------------

errors=0
warnings=0

# `status:` is an open set like `project.activity`: an unrecognized value is
# advisory so the vocabulary can be extended, but a value that contradicts
# `superseded_by` is a hard error (see the check below).
VALID_DEC_STATUS=" proposed accepted rejected deprecated superseded "
status_notes=""
unattributed=""

MAP_IDS=()
MAP_FILES=()

# Echo the file registered for an id, or nothing.
file_for_id() {
    local target="$1" i
    for ((i = 0; i < ${#MAP_IDS[@]}; i++)); do
        if [ "${MAP_IDS[$i]}" = "$target" ]; then
            printf '%s' "${MAP_FILES[$i]}"
            return 0
        fi
    done
    return 1
}

# Build the map and detect duplicates / filename mismatches / missing
# required fields.
for file in "${DEC_FILES[@]}"; do
    base=$(basename "$file")
    file_id=$(printf '%s' "$base" | sed -E 's/^(DEC-[0-9]+).*/\1/')
    fm_id=$(get_insight_id "$file")
    created=$(get_top_scalar "$file" created_at)
    itype=$(get_insight_type "$file")

    if [ -z "$fm_id" ]; then
        warn "${base}: missing insight.id in front-matter"
        errors=$((errors + 1))
        continue
    fi
    if [ "$fm_id" != "$file_id" ]; then
        warn "${base}: filename id (${file_id}) != front-matter insight.id (${fm_id})"
        errors=$((errors + 1))
    fi
    if existing=$(file_for_id "$fm_id"); then
        warn "${fm_id}: duplicate id — also in $(basename "$existing")"
        errors=$((errors + 1))
    fi
    MAP_IDS+=("$fm_id")
    MAP_FILES+=("$file")

    if [ -z "$created" ] || [ "$created" = "null" ]; then
        warn "${fm_id}: missing created_at"
        errors=$((errors + 1))
    fi
    if [ -z "$itype" ]; then
        warn "${fm_id}: missing insight.type"
        errors=$((errors + 1))
    fi

    # `status:` is OPTIONAL (absent = derive from superseded_by, as always).
    # When declared, two checks with deliberately different severities:
    #   - an unrecognized value is ADVISORY (open-set discipline, the
    #     `project.activity` precedent) — collected, never an error;
    #   - a value that CONTRADICTS superseded_by is a structural error, because
    #     the two fields then disagree about the same fact.
    dstatus=$(get_top_scalar "$file" status)
    if [ -n "$dstatus" ] && [ "$dstatus" != "null" ]; then
        case "$VALID_DEC_STATUS" in
            *" $dstatus "*) : ;;
            *) status_notes="${status_notes}    ${fm_id}: status='${dstatus}'"$'\n' ;;
        esac
        dsupby=$(get_top_scalar "$file" superseded_by)
        if [ "$dstatus" = "superseded" ] && { [ -z "$dsupby" ] || [ "$dsupby" = "null" ]; }; then
            warn "${fm_id}: status: superseded but superseded_by is null — say which decision replaced it"
            errors=$((errors + 1))
        elif [ "$dstatus" != "superseded" ] && [ -n "$dsupby" ] && [ "$dsupby" != "null" ]; then
            warn "${fm_id}: superseded_by is ${dsupby} but status is '${dstatus}' (expected 'superseded')"
            errors=$((errors + 1))
        fi
    fi

    # An unattributed decision — no project.id — is invisible to every
    # per-project view. Advisory: repo-wide decisions legitimately have no
    # project, so this can never be a gate. Only checked where attribution was
    # possible at all: a repo with no projects/ (the template itself) would
    # otherwise flag every decision it has.
    if [ -d "${REPO_ROOT}/projects" ] && [ -z "$(get_dec_project_id "$file")" ]; then
        unattributed="${unattributed}    ${fm_id}"$'\n'
    fi
done

# Supersession integrity: every supersedes/superseded_by must point at a
# real DEC, not at itself, and be mirrored on the other side.
for file in "${DEC_FILES[@]}"; do
    id=$(get_insight_id "$file")
    [ -z "$id" ] && continue
    sup=$(get_top_scalar "$file" supersedes)
    supby=$(get_top_scalar "$file" superseded_by)

    if [ -n "$sup" ] && [ "$sup" != "null" ]; then
        if [ "$sup" = "$id" ]; then
            warn "${id}: supersedes itself"
            errors=$((errors + 1))
        elif ! sup_file=$(file_for_id "$sup"); then
            warn "${id}: supersedes ${sup}, which does not exist"
            errors=$((errors + 1))
        else
            other_supby=$(get_top_scalar "$sup_file" superseded_by)
            if [ "$other_supby" != "$id" ]; then
                warn "${id}: supersedes ${sup}, but ${sup}.superseded_by is '${other_supby:-null}' (expected ${id})"
                errors=$((errors + 1))
            fi
        fi
    fi

    if [ -n "$supby" ] && [ "$supby" != "null" ]; then
        if [ "$supby" = "$id" ]; then
            warn "${id}: superseded_by itself"
            errors=$((errors + 1))
        elif ! file_for_id "$supby" >/dev/null; then
            warn "${id}: superseded_by ${supby}, which does not exist"
            errors=$((errors + 1))
        fi
    fi
done

# Scope-overlap warnings between ACTIVE decisions (heuristic, advisory).
ACTIVE_FILES=()
for file in "${DEC_FILES[@]}"; do
    is_active "$file" && ACTIVE_FILES+=("$file")
done

# affected_scope hygiene: a bare name (no path separator AND no wildcard) almost
# never matches a nested repo path, so the decision silently governs nothing —
# false confidence, worse than noise (zany harvest: `['_headers']` never matched
# the real `public/_headers`).
for file in "${ACTIVE_FILES[@]}"; do
    idc=$(get_insight_id "$file")
    while IFS= read -r g; do
        [ -n "$g" ] || continue
        case "$g" in
            *[/*?]*) : ;;   # has a path separator or a wildcard — fine
            *) warn "${idc}: affected_scope '${g}' has no path separator or wildcard — it likely matches nothing (did you mean 'dir/${g}' or '**/${g}'?)"
               warnings=$((warnings + 1)) ;;
        esac
    done < <(get_affected_scope "$file")
done

n=${#ACTIVE_FILES[@]}
for ((x = 0; x < n; x++)); do
    fa="${ACTIVE_FILES[$x]}"
    ida=$(get_insight_id "$fa")
    globs_a=()
    while IFS= read -r g; do
        [ -n "$g" ] && globs_a+=("$g")
    done < <(get_affected_scope "$fa")
    [ "${#globs_a[@]}" -eq 0 ] && continue

    for ((y = x + 1; y < n; y++)); do
        fb="${ACTIVE_FILES[$y]}"
        idb=$(get_insight_id "$fb")
        globs_b=()
        while IFS= read -r g; do
            [ -n "$g" ] && globs_b+=("$g")
        done < <(get_affected_scope "$fb")
        [ "${#globs_b[@]}" -eq 0 ] && continue

        rel=""; pair=""
        for ga in "${globs_a[@]}"; do
            for gb in "${globs_b[@]}"; do
                r=$(scope_relationship "$ga" "$gb")
                if [ "$r" = same ]; then rel=same; pair="${ga} == ${gb}"; break 2; fi
                if [ "$r" = nest ] && [ -z "$rel" ]; then rel=nest; pair="${ga} / ${gb}"; fi
            done
        done
        if [ "$rel" = same ]; then
            # Two decisions rooted at the SAME scope — the genuine "which one
            # governs?" case; keep it a warning.
            warn "${ida} and ${idb} govern the same scope (${pair})"
            echo "      ${DIM}confirm they don't contradict; if one wins, mark the other superseded${RESET}"
            warnings=$((warnings + 1))
        elif [ "$rel" = nest ]; then
            # A broad decision that deliberately contains a narrower one —
            # hierarchy, not a conflict. Info, not a warning (fixes the noise a
            # broad src/engine/** parent produced against its children).
            info "${ida} / ${idb}: nested scope (${pair}) — hierarchy, not a conflict"
        fi
    done
done

# Advisories: printed after the checks, never counted, never change the exit code.
if [ -n "$status_notes" ]; then
    echo
    warn "unrecognized status (advisory — open set, expected one of${VALID_DEC_STATUS}):"
    printf '%s' "$status_notes"
fi
if [ -n "$unattributed" ]; then
    echo
    warn "decision(s) with no project.id (advisory — invisible to per-project views):"
    printf '%s' "$unattributed"
fi

# --- spec → DEC: is anything being built against a decision that isn't binding?
# The first traceability edge this repo walks. A spec designed against a
# `status: proposed` DEC is not necessarily wrong — proposals get built to test
# them — but it is worth knowing, because the usual failure is that the proposal
# was never accepted and the spec quietly became its only evidence. Advisory:
# building against a proposal is a legitimate move, so this can never gate.
proposed_refs=""
while IFS= read -r pdir; do
    [ -n "$pdir" ] || continue
    while IFS= read -r sf; do
        [ -n "$sf" ] || continue
        case "$sf" in */prompts/*|*-timeline.md) continue ;; esac
        while IFS= read -r ref; do
            [ -n "$ref" ] || continue
            if rf=$(file_for_id "$ref"); then
                if [ "$(get_top_scalar "$rf" status)" = "proposed" ]; then
                    proposed_refs="${proposed_refs}    $(basename "$sf" .md) → ${ref} (still proposed)"$'\n'
                fi
            fi
        done < <(get_spec_referenced_decisions "$sf")
    done < <(find_all_specs "$pdir")
done < <(find "${REPO_ROOT}/projects" -maxdepth 1 -type d -name 'PROJ-*' 2>/dev/null | sort)

if [ -n "$proposed_refs" ]; then
    echo
    warn "spec(s) built against a decision that is still 'proposed' (advisory):"
    printf '%s' "$proposed_refs"
    printf '    %saccept the DEC, or note in it that a spec already depends on it.%s\n' "$DIM" "$RESET"
fi

echo
if [ "$errors" -gt 0 ]; then
    die "${errors} structural error(s), ${warnings} scope warning(s) across ${#DEC_FILES[@]} decision(s)."
fi
if [ "$warnings" -gt 0 ]; then
    info "0 structural errors, ${warnings} scope warning(s) across ${#DEC_FILES[@]} decision(s)."
else
    success "All ${#DEC_FILES[@]} decision(s) clean: structure valid, no scope conflicts."
fi
