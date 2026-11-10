#!/usr/bin/env bash
# scripts/id-audit.sh — find artifact ids used twice: one SPEC/STAGE/PATCH/
# SPIKE/HANDOFF/DEC number given to two different artifacts, anywhere in this
# checkout, the repo's other worktrees, or its local branches and
# remote-tracking refs.
#
# Why: ids used to be minted from the working tree alone, so two branches or two
# worktrees could each take the next number. Irradiance did this at least 7
# times; DEC-028 reached history on two branches with two different decisions.
# `next_id` now looks across refs and worktrees, but it cannot see a collision
# that already happened, or work in a clone it cannot reach. This finds them.
#
# Same id, same file name = one artifact (e.g. moved to done/, or present on
# several branches). Same id, different file names = a collision. Companions that
# share their spec's id (prompts/, *-timeline.md) and tests/ fixtures are not
# artifacts and are ignored.
#
# A collision someone already fixed by renumbering (the slug now lives under
# another id in this checkout, while a stale branch still has the old number) is
# listed as renumbered, not as a collision, and does not fail the audit.
# crustyimg's DEC-092 -> DEC-093 is the case. Renaming a slug while an older
# branch holds the old name still reports, until that branch is merged or
# deleted.
#
# Refs are read as they are locally; run `git fetch` first to include teammates'
# pushed work.
#
#   just id-audit
#   just id-audit --json
#
# Exit status: 0 no collisions · 1 collisions found · 2 usage error.

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/_lib.sh
source "${SCRIPT_DIR}/_lib.sh"

JSON_OUT=$(has_json_flag "$@")
for a in "$@"; do
    case "$a" in
        --json) : ;;
        '')     : ;;
        *)      usage_error "just id-audit [--json]" ;;
    esac
done

PREFIXES="SPEC STAGE PATCH SPIKE HANDOFF DEC"
ROOT=$(_checkout_root)

n_worktrees=$(git -C "$REPO_ROOT" worktree list --porcelain 2>/dev/null | grep -c '^worktree ' || true)
[ "${n_worktrees:-0}" -gt 0 ] || n_worktrees=1
n_refs=$(git -C "$REPO_ROOT" for-each-ref refs/heads refs/remotes 2>/dev/null | grep -c . || true)

# One row per artifact that shares its id with another:
#   kind \t id \t file name \t path \t sources \t now
# kind C = an open collision, R = already renumbered (now = its id here).
# Sources are joined by \037 (ref names may contain ';' or ',').
rows=$(
    for p in $PREFIXES; do
        id_artifact_paths "$p" | _id_number_column "$p" \
            | awk -F'\t' -v p="$p" '{ printf "%s-%03d\t%s\t%s\n", p, $1, $2, $3 }'
    done | awk -F'\t' -v root="$ROOT" '
        function label(s,   w) {
            if (s !~ /^worktree:/) return s
            w = substr(s, 10)
            if (w == root) return "this checkout"
            if (index(w, root "/") == 1) return "worktree:" substr(w, length(root) + 2)
            return s
        }
        {
            id = $1; src = label($2); path = $3
            n = split(path, seg, "/"); base = seg[n]
            pre = id; sub(/-[0-9]+$/, "", pre)
            slug = base; sub(/^[A-Z]+-[0-9]+-?/, "", slug)
            if (src == "this checkout") here[pre, slug] = id
            k = id SUBSEP base
            if (!(k in seen)) {
                seen[k] = 1; firstpath[k] = path; slugof[k] = slug; preof[k] = pre
                nb[id]++; bases[id, nb[id]] = base
                if (!(id in idorder)) { idorder[id] = ++nid; ids[nid] = id }
            }
            if (!((k, src) in hassrc)) {
                hassrc[k, src] = 1
                srcs[k] = (srcs[k] == "" ? src : srcs[k] "\037" src)
            }
        }
        END {
            for (i = 1; i <= nid; i++) {
                id = ids[i]
                if (nb[id] < 2) continue
                open_n = 0
                for (j = 1; j <= nb[id]; j++) {
                    k = id SUBSEP bases[id, j]
                    hk = preof[k] SUBSEP slugof[k]
                    now[k] = ((hk in here) && here[hk] != id) ? here[hk] : ""
                    if (now[k] == "") open_n++
                }
                kind = (open_n >= 2) ? "C" : "R"
                for (j = 1; j <= nb[id]; j++) {
                    k = id SUBSEP bases[id, j]
                    if (kind == "R" && now[k] == "") continue
                    printf "%s\t%s\t%s\t%s\t%s\t%s\n", kind, id, bases[id, j], firstpath[k], srcs[k], now[k]
                }
            }
        }' | sort -t "$(printf '\t')" -k1,1 -k2,2 -k3,3
)

TAB=$(printf '\t')
c_rows=$(printf '%s\n' "$rows" | grep "^C${TAB}" || true)
r_rows=$(printf '%s\n' "$rows" | grep "^R${TAB}" || true)
n_collisions=0
[ -z "$c_rows" ] || n_collisions=$(printf '%s\n' "$c_rows" | cut -f2 | sort -u | grep -c . || true)

# Sources (joined by \037) as a JSON array.
sources_json() {
    local s out=()
    while IFS= read -r s; do
        [ -n "$s" ] && out+=("$(json_qs "$s")")
    done <<EOF
$(printf '%s' "$1" | tr '\037' '\n')
EOF
    json_arr ${out[@]+"${out[@]}"}
}

if [ "$JSON_OUT" = 1 ]; then
    collisions=() current="" artifacts=()
    flush() {
        [ -n "$current" ] || return 0
        collisions+=("$(json_obj id "$(json_qs "$current")" artifacts "$(json_arr "${artifacts[@]}")")")
    }
    if [ -n "$c_rows" ]; then
        while IFS="$TAB" read -r _ id base path srcs _; do
            if [ "$id" != "$current" ]; then
                flush
                current=$id; artifacts=()
            fi
            artifacts+=("$(json_obj file "$(json_qs "$base")" path "$(json_qs "$path")" \
                sources "$(sources_json "$srcs")")")
        done <<EOF
$c_rows
EOF
        flush
    fi
    renumbered=()
    if [ -n "$r_rows" ]; then
        while IFS="$TAB" read -r _ id base path srcs now; do
            renumbered+=("$(json_obj id "$(json_qs "$id")" now "$(json_qs "$now")" \
                file "$(json_qs "$base")" path "$(json_qs "$path")" \
                sources "$(sources_json "$srcs")")")
        done <<EOF
$r_rows
EOF
    fi
    json_emit id-audit "$(json_obj \
        collision_count "$n_collisions" \
        worktrees_scanned "$n_worktrees" \
        refs_scanned "${n_refs:-0}" \
        collisions "$(json_arr ${collisions[@]+"${collisions[@]}"})" \
        renumbered "$(json_arr ${renumbered[@]+"${renumbered[@]}"})")"
    [ "$n_collisions" -eq 0 ] || exit 1
    exit 0
fi

# Up to three places an artifact was found; --json lists them all.
print_sources() {
    printf '%s' "$1" | tr '\037' '\n' | awk '
        NR <= 3 { print "        " $0 }
        END { if (NR > 3) printf "        … and %d more\n", NR - 3 }'
}

print_renumbered() {
    [ -n "$r_rows" ] || return 0
    echo ""
    info "Already renumbered here; the old number survives only on stale branches or worktrees:"
    while IFS="$TAB" read -r _ id base path srcs now; do
        echo "    ${id} → ${now}: ${path}"
        print_sources "$srcs"
    done <<EOF
$r_rows
EOF
}

scope="${n_worktrees} worktree(s) and ${n_refs:-0} ref(s)"
if [ "$n_collisions" -eq 0 ]; then
    success "No artifact id is used twice across ${scope}."
    print_renumbered
    exit 0
fi

echo "${RED}✗${RESET} ${n_collisions} artifact id(s) used by more than one artifact, across ${scope}:"
current=""
while IFS="$TAB" read -r _ id base path srcs _; do
    if [ "$id" != "$current" ]; then
        echo ""
        echo "  ${BOLD}${id}${RESET}"
        current=$id
    fi
    echo "    ${path}"
    print_sources "$srcs"
done <<EOF
$c_rows
EOF
print_renumbered
echo ""
echo "Renumber all but one of each (and update whatever cites the old id)."
echo "If one is just a renamed slug of the other, merging or deleting the stale branch clears it."
exit 1
