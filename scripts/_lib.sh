#!/usr/bin/env bash
# scripts/_lib.sh — shared helpers sourced by other scripts.
# Sources are bash-only. Keep this minimal.

set -euo pipefail

REPO_ROOT="$(pwd)"

# Colors (fall back to no-op if terminal doesn't support color).
if [ -t 1 ] && command -v tput >/dev/null 2>&1; then
    BOLD=$(tput bold 2>/dev/null || printf '')
    DIM=$(tput dim 2>/dev/null || printf '')
    RED=$(tput setaf 1 2>/dev/null || printf '')
    GREEN=$(tput setaf 2 2>/dev/null || printf '')
    YELLOW=$(tput setaf 3 2>/dev/null || printf '')
    BLUE=$(tput setaf 4 2>/dev/null || printf '')
    RESET=$(tput sgr0 2>/dev/null || printf '')
else
    BOLD=''; DIM=''; RED=''; GREEN=''; YELLOW=''; BLUE=''; RESET=''
fi

die() {
    echo "${RED}ERROR:${RESET} $*" >&2
    exit 1
}

info() {
    echo "${BLUE}•${RESET} $*"
}

success() {
    echo "${GREEN}✓${RESET} $*"
}

warn() {
    echo "${YELLOW}⚠${RESET} $*"
}

# Require that the repo has been initialized (AGENTS.md at root).
require_initialized() {
    if [ ! -f "${REPO_ROOT}/AGENTS.md" ]; then
        die "Repo not initialized. Run 'just init' first."
    fi
}

# Where DEC-* files live. A scaffolded instance keeps them at the repo root in
# `decisions/`; the TEMPLATE repo keeps its own under `docs/decisions/`, because
# its root is the scaffold surface rather than a project. Root wins when both
# exist, so an instance's behavior is unchanged.
#
# Why this fallback exists: without it the decisions governing the template
# itself get zero structural checking — which is exactly how the DEC schema
# forked between the two copies (status/deciders/date, and `type: architecture`
# on 11 of 13 files, out of the enum the auditor would have caught).
decisions_dir() {
    if [ -d "${REPO_ROOT}/decisions" ]; then
        printf '%s' "${REPO_ROOT}/decisions"
    elif [ -d "${REPO_ROOT}/docs/decisions" ]; then
        printf '%s' "${REPO_ROOT}/docs/decisions"
    else
        printf '%s' "${REPO_ROOT}/decisions"   # canonical default; callers tolerate it missing
    fi
}

# Guard for the DEC-facing commands, which are the only ones that work in BOTH
# contexts: a scaffolded instance (AGENTS.md at root) and the template repo
# itself (no AGENTS.md, but `variants/` + `VERSION` + its own docs/decisions/).
# Every other command still requires a full instance.
require_decisions_context() {
    [ -f "${REPO_ROOT}/AGENTS.md" ] && return 0
    if [ -f "${REPO_ROOT}/VERSION" ] && [ -d "${REPO_ROOT}/variants" ] \
        && [ -d "${REPO_ROOT}/docs/decisions" ]; then
        return 0
    fi
    die "Repo not initialized. Run 'just init' first."
}

# Get the active variant (claude-only or claude-plus-agents).
get_variant() {
    if [ -f "${REPO_ROOT}/.variant" ]; then
        cat "${REPO_ROOT}/.variant"
    else
        # Fallback: detect based on whether any project has a handoffs/ folder.
        if find "${REPO_ROOT}/projects" -maxdepth 2 -type d -name handoffs 2>/dev/null | grep -q .; then
            echo "claude-plus-agents"
        else
            echo "claude-only"
        fi
    fi
}

# Find the active project directory. Default heuristic: the lexically first
# project folder that doesn't start with PROJ-ZZZZ-archive or similar.
# Users can override by setting ACTIVE_PROJECT env var.
# Read project.status from a project directory's brief.md
# (active | proposed | shipped | cancelled | on_hold), empty if unset/absent.
# Uses field-2 so a trailing "# comment" on the status line is tolerated.
get_project_status() {
    local brief="${1}/brief.md"
    [ -f "$brief" ] || { echo ""; return; }
    awk '
        /^---$/ { f = !f; next }
        f && /^project:/ { inp = 1; next }
        f && inp && /^[a-zA-Z_]/ { inp = 0 }
        f && inp && /^[[:space:]]+status:/ { print $2; exit }
    ' "$brief" 2>/dev/null
}

# Read project.activity from a project directory's brief.md. Empty for
# null/missing. `activity` is an OPTIONAL, human-facing refinement of the
# work happening *within* an `active` project (suggested, open set:
# requirements | design | build | test | blocked) — distinct from the
# coarse, machine-keyed `status`. Usage: get_project_activity projects/PROJ-001-foo
get_project_activity() {
    local brief="${1}/brief.md"
    [ -f "$brief" ] || { echo ""; return; }
    awk '
        /^---$/ { f = !f; next }
        f && /^project:/ { inp = 1; next }
        f && inp && /^[a-zA-Z_]/ { inp = 0 }
        f && inp && /^[[:space:]]+activity:/ {
            v = $2
            if (v != "null" && v != "") print v
            exit
        }
    ' "$brief" 2>/dev/null
}

get_active_project() {
    if [ -n "${ACTIVE_PROJECT:-}" ]; then
        echo "${ACTIVE_PROJECT}"
        return
    fi
    # Non-example PROJ-* dirs, lowest number first.
    local dirs
    dirs=$(find "${REPO_ROOT}/projects" -maxdepth 1 -type d -name "PROJ-*" 2>/dev/null \
           | grep -v "example" | sort)
    # Prefer a project explicitly marked `status: active`. Without this, a
    # SHIPPED earlier wave silently captures every default-scoped command once a
    # second project exists — `just status`/`cost-audit`/`backlog` would target
    # the finished project and never inspect the active wave (the multi-wave
    # hazard; 2026-07-06 harvest signal #1). Among several active, the
    # lowest-numbered wins, for determinism.
    local d first=""
    while IFS= read -r d; do
        [ -n "$d" ] || continue
        if [ "$(get_project_status "$d")" = "active" ]; then first="$d"; break; fi
    done <<EOF
${dirs}
EOF
    # No project marked active → fall back to the lowest-numbered non-example.
    [ -n "$first" ] || first=$(printf '%s\n' "$dirs" | head -n1)
    # Still nothing → fall back to the example project.
    if [ -z "$first" ]; then
        first=$(find "${REPO_ROOT}/projects" -maxdepth 1 -type d -name "PROJ-*" 2>/dev/null \
                | sort | head -n1)
    fi
    if [ -z "$first" ]; then
        die "No projects found in ./projects/. Create one by copying projects/_templates/project-brief.md into projects/PROJ-NNN-<slug>/brief.md (see GETTING_STARTED.md)."
    fi
    basename "$first"
}

# Resolve a project id/name to its directory, deterministically.
#   $1 = optional PROJ-NNN, a full dir name, or empty (→ active project).
# Empty reuses get_active_project (which already skips the example project).
# A PROJ-NNN glob that matches more than one directory is a HARD ERROR, not a
# silent `head -n1` — that silent pick stamped the wrong project when the
# example and a real project shared a number (verified: zany-animal-slots #1).
resolve_project_dir() {
    local pid="${1:-}"
    if [ -z "$pid" ]; then
        echo "${REPO_ROOT}/projects/$(get_active_project)"
        return
    fi
    # Exact directory name wins (unambiguous by construction).
    if [ -d "${REPO_ROOT}/projects/${pid}" ]; then
        echo "${REPO_ROOT}/projects/${pid}"
        return
    fi
    local matches count
    matches=$(find "${REPO_ROOT}/projects" -maxdepth 1 -type d -name "${pid}-*" 2>/dev/null | sort)
    count=$(printf '%s' "$matches" | grep -c . || true)
    if [ "$count" -eq 0 ]; then
        die "Project not found: ${pid}"
    fi
    if [ "$count" -gt 1 ]; then
        die "Ambiguous project id '${pid}' — matches multiple directories:
$(printf '%s\n' "$matches" | sed 's|.*/|  - |')
Pass the full directory name (e.g. ${pid}-<slug>) to disambiguate."
    fi
    printf '%s\n' "$matches"
}

# Return the next ID for a given prefix (SPEC, STAGE, PROJ, DEC, HANDOFF)
# across the entire repo (or within a project, for SPEC/STAGE/HANDOFF).
# Usage: next_id SPEC ./projects/PROJ-001-foo
next_id() {
    local prefix="$1"
    local search_dir="${2:-$REPO_ROOT}"
    local max
    max=$(find "$search_dir" -type f -name "${prefix}-*.md" 2>/dev/null \
          | sed -E "s|.*/${prefix}-([0-9]+).*|\\1|" \
          | sort -n \
          | tail -n1 || true)
    if [ -z "$max" ]; then
        printf "%s-%03d" "$prefix" 1
    else
        # Strip leading zeros for arithmetic, then reformat.
        max=$((10#$max))
        printf "%s-%03d" "$prefix" $((max + 1))
    fi
}

# Slugify a string. "Foo Bar Baz" -> "foo-bar-baz"
slugify() {
    echo "$1" \
        | tr '[:upper:]' '[:lower:]' \
        | sed -E 's/[^a-z0-9]+/-/g' \
        | sed -E 's/^-+|-+$//g'
}

# Escape a string for safe use as the REPLACEMENT half of a
# `sed "s|...|REPLACEMENT|"` command. Escapes backslash, the `|`
# delimiter, and `&` (sed's whole-match reference). Pure bash
# parameter expansion — no sed-escaping-sed, portable to bash 3.2.
#
# Always run user-controlled text (e.g. a spec/stage title) through
# this before substituting it into a template. Without it, a title
# containing `|` could close the s-command early and a trailing `e`
# would reach GNU sed's execute flag — i.e. command injection. The
# escaped output still renders to the original characters in the file.
sed_escape_replacement() {
    local s="$1"
    s=${s//\\/\\\\}   # backslash first, so we don't double-escape below
    s=${s//|/\\|}     # the s-command delimiter
    s=${s//&/\\&}     # sed's "insert whole match" reference
    printf '%s' "$s"
}

# Find a spec file by ID. Searches all projects. Only returns active
# specs — archived specs under specs/done/ are excluded so callers
# like advance-cycle and archive-spec don't silently operate on an
# already-shipped file. Also excludes `*-timeline.md` so the v5.3
# timeline artifact (which shares the SPEC-NNN-* prefix) doesn't
# masquerade as the spec.
# Uses find's -not -path rather than a grep pipeline: grep returns 1
# on no matches, which trips pipefail and would make this function
# silently abort the caller under `set -e`.
# Usage: find_spec SPEC-001
find_spec() {
    local spec_id="$1"
    # Exclude *-timeline.md (shares the SPEC-NNN-* prefix), archived specs
    # (done/), and cycle-prompt files (prompts/SPEC-NNN-<cycle>.md, which also
    # share the prefix). Without the prompts/ exclusion, advance-cycle/
    # archive-spec could resolve to a prompt file that has no front-matter and
    # silently no-op (verified downstream: zany-animal-slots #7).
    # `spikes/` is searched too: a spike lives at the REPO root, not under a
    # project, because it may precede any project (DEC-012). Searching both here
    # is what lets `advance-cycle SPIKE-NNN land` work without a parallel lookup.
    #
    # spikes/ is absent in a fresh scaffold (it's created on first `new-spike`),
    # and `find` exits non-zero on a missing path — which under `set -o pipefail`
    # would fail the whole pipeline. Build the search list from what exists.
    local roots="${REPO_ROOT}/projects"
    [ -d "${REPO_ROOT}/spikes" ] && roots="${roots} ${REPO_ROOT}/spikes"
    # shellcheck disable=SC2086 # deliberate word-splitting: roots is a path list
    find $roots -type f -name "${spec_id}-*.md" \
        -not -name '*-timeline.md' \
        -not -path '*/done/*' \
        -not -path '*/prompts/*' 2>/dev/null | head -n1
}

# All spikes in the repo (the bounded-exploration lane, DEC-012). Repo-level,
# not project-scoped — unlike find_all_specs / find_all_patches. Silent (and
# non-failing) when spikes/ doesn't exist yet, which is the pre-first-spike state.
find_all_spikes() {
    [ -d "${REPO_ROOT}/spikes" ] || return 0
    find "${REPO_ROOT}/spikes" -type f -name "SPIKE-*.md" 2>/dev/null
}

# A handoff's `handoff.<key>` front-matter scalar (cycle / to_agent / status …).
get_handoff_field() {
    local file="$1" key="$2"
    awk -v k="$key" '
        /^---$/ { f = !f; next }
        !f { exit }
        /^handoff:/ { ins = 1; next }
        ins && /^[^[:space:]]/ { ins = 0 }
        ins && $0 ~ ("^[[:space:]]+" k ":") {
            sub("^[[:space:]]+" k ":[[:space:]]*", "")
            sub("[[:space:]]*#.*$", "")
            print; exit
        }
    ' "$file"
}

# A handoff's `handback.<key>` — the agent's self-reported return values
# (status / tokens_total / estimated_usd / synced_at …). This is how cost gets
# into a spec when build+verify run on agents the orchestrator can't meter.
get_handback_field() {
    local file="$1" key="$2"
    awk -v k="$key" '
        /^---$/ { f = !f; next }
        !f { exit }
        /^handback:/ { ins = 1; next }
        ins && /^[^[:space:]]/ { ins = 0 }
        ins && $0 ~ ("^[[:space:]]+" k ":") {
            sub("^[[:space:]]+" k ":[[:space:]]*", "")
            sub("[[:space:]]*#.*$", "")
            print; exit
        }
    ' "$file"
}

# A spike's `spike.<key>` front-matter scalar (question / mode / timebox /
# outcome / landed_at). Empty if absent. Tolerates a trailing "# ..." comment.
get_spike_field() {
    local file="$1" key="$2"
    awk -v k="$key" '
        /^---$/ { f = !f; next }
        !f { exit }
        /^spike:/ { ins = 1; next }
        ins && /^[^[:space:]]/ { ins = 0 }
        ins && $0 ~ ("^[[:space:]]+" k ":") {
            sub("^[[:space:]]+" k ":[[:space:]]*", "")
            sub("[[:space:]]*#.*$", "")
            print; exit
        }
    ' "$file"
}

# Find the timeline file paired with a spec. Returns empty if none.
# Usage: find_spec_timeline SPEC-001
find_spec_timeline() {
    local spec_id="$1"
    find "${REPO_ROOT}/projects" -type f -name "${spec_id}-*-timeline.md" \
        -not -path '*/done/*' 2>/dev/null | head -n1
}

# Find a stage file by ID.
find_stage() {
    local stage_id="$1"
    find "${REPO_ROOT}/projects" -type f -name "${stage_id}-*.md" 2>/dev/null | head -n1
}

# Extract un-promoted "(not yet written)" bullets from a stage's ## Spec Backlog
# section. One bullet per line, in backlog order.
# Convention: `- [ ] (not yet written) — <summary>` with an optional
# `[S]/[M]/[L]` complexity tag anywhere on the line.
# Read by `just backlog` (view) and `just frame-stage` (promotion) so the set
# you see is exactly the set that gets promoted.
# Usage: extract_unpromoted_bullets path/to/STAGE-001-foo.md
extract_unpromoted_bullets() {
    awk '
        /^## Spec Backlog/ { in_b = 1; next }
        in_b && /^## / { in_b = 0 }
        # Must be a checkbox BULLET, not merely prose containing the phrase —
        # the section explains itself, so a doc line naming "(not yet written)"
        # would otherwise be promoted into a spec. (Caught by the selftest.)
        in_b && /^-[[:space:]]*\[/ && /\(not yet written\)/ { print }
    ' "$1"
}

# Find the "active" stage file for a given project. Heuristic: first
# stage with status: active, falling back to the lexically-first
# stage. Used by backlog and report_daily so they agree on what "the
# active stage" means.
# Usage: get_active_stage_file projects/PROJ-001-foo
get_active_stage_file() {
    local project_dir="$1"
    local stages_dir="${project_dir}/stages"
    [ -d "$stages_dir" ] || return
    local s status
    for s in "${stages_dir}"/STAGE-*.md; do
        [ -f "$s" ] || continue
        status=$(awk '/^---$/{f=!f; next} f && /^[[:space:]]+status:/{print $2; exit}' "$s" 2>/dev/null || echo "")
        if [ "$status" = "active" ]; then echo "$s"; return; fi
    done
    for s in "${stages_dir}"/STAGE-*.md; do
        [ -f "$s" ] || continue
        echo "$s"; return
    done
}

# Read a stage file's status: field. Empty string if missing.
get_stage_status() {
    local file="$1"
    [ -f "$file" ] || return
    awk '/^---$/{f=!f; next} f && /^[[:space:]]+status:/{print $2; exit}' "$file" 2>/dev/null || echo ""
}

# Read a stage file's target_complete: field. Empty if null/missing.
get_stage_target() {
    local file="$1"
    [ -f "$file" ] || return
    awk '
        /^---$/ { fm = !fm; next }
        !fm { exit }
        /^[[:space:]]+target_complete:/ {
            v = $2
            if (v != "null" && v != "") print v
            exit
        }
    ' "$file"
}

# Read a stage file's top-level created_at field. Used as a proxy
# for "started_on" in the roadmap.
get_stage_created_at() {
    local file="$1"
    [ -f "$file" ] || return
    awk '
        /^---$/ { fm = !fm; next }
        !fm { exit }
        /^created_at:/ {
            v = $2
            if (v != "null" && v != "") print v
            exit
        }
    ' "$file"
}

# Read a stage file's top-level shipped_at field. Empty if null.
get_stage_shipped_at() {
    local file="$1"
    [ -f "$file" ] || return
    awk '
        /^---$/ { fm = !fm; next }
        !fm { exit }
        /^shipped_at:/ {
            v = $2
            if (v != "null" && v != "") print v
            exit
        }
    ' "$file"
}

# Today's date in YYYY-MM-DD format.
today() {
    date +%Y-%m-%d
}

# Read the repo's ID from .repo-context.yaml (metadata.repo.id).
# Used by scaffold scripts to substitute the __REPO_ID__ placeholder
# in templates. Falls back to "my-app" if the file or key is missing,
# which matches the template default and avoids breaking scaffolding
# on a freshly-cloned repo where the user hasn't replaced values yet.
get_repo_id() {
    local ctx="${REPO_ROOT}/.repo-context.yaml"
    if [ ! -f "$ctx" ]; then
        echo "my-app"
        return
    fi
    local id
    id=$(awk '
        /^metadata:/ { in_meta = 1; next }
        /^[a-zA-Z]/ && in_meta { in_meta = 0 }
        in_meta && /^  repo:/ { in_repo = 1; next }
        in_meta && in_repo && /^  [a-zA-Z]/ { in_repo = 0 }
        in_meta && in_repo && /^    id:/ { print $2; exit }
    ' "$ctx")
    echo "${id:-my-app}"
}

# The cost metering source from .repo-context.yaml (spec.cost.metering_source).
# Defaults to subagent_tokens (the Claude-Code default). `just cost-audit`
# honors it: `none` means the platform exposes no token count, so the gate is
# disabled rather than blocking on a number that can't exist (DEC-005).
get_metering_source() {
    local ctx="${REPO_ROOT}/.repo-context.yaml"
    [ -f "$ctx" ] || { echo "subagent_tokens"; return; }
    local v
    v=$(awk '
        /^spec:/ { in_spec = 1; next }
        /^[a-zA-Z]/ && in_spec { in_spec = 0 }
        in_spec && /^  cost:/ { in_cost = 1; next }
        in_spec && in_cost && /^  [a-zA-Z]/ { in_cost = 0 }
        in_spec && in_cost && /^    metering_source:/ { print $2; exit }
    ' "$ctx")
    echo "${v:-subagent_tokens}"
}

# The default agent model from .repo-context.yaml (spec.agent.default_model),
# stamped into new artifacts' agents.* (DEC-005). Falls back to claude-opus-4-7.
get_default_model() {
    local ctx="${REPO_ROOT}/.repo-context.yaml"
    local v=""
    if [ -f "$ctx" ]; then
        v=$(awk '
            /^spec:/ { in_spec = 1; next }
            /^[a-zA-Z]/ && in_spec { in_spec = 0 }
            in_spec && /^  agent:/ { in_agent = 1; next }
            in_spec && in_agent && /^  [a-zA-Z]/ { in_agent = 0 }
            in_agent && /^    default_model:/ { print $2; exit }
        ' "$ctx")
    fi
    echo "${v:-claude-opus-4-7}"
}

# The model configured for a cycle (spec.agent.tier_map.<cycle>), e.g. the
# design=Opus/build=Sonnet split made pluggable. Falls back to default_model
# (then claude-opus-4-7). DEC-005.
get_tier_model() {
    local cycle="$1"
    local ctx="${REPO_ROOT}/.repo-context.yaml"
    local v=""
    if [ -f "$ctx" ]; then
        v=$(awk -v cyc="$cycle" '
            /^spec:/ { in_spec = 1; next }
            /^[a-zA-Z]/ && in_spec { in_spec = 0 }
            in_spec && /^  agent:/ { in_agent = 1; next }
            in_spec && in_agent && /^  [a-zA-Z]/ { in_agent = 0 }
            in_agent && /^    tier_map:/ { in_tier = 1; next }
            in_agent && in_tier && /^    [a-zA-Z]/ { in_tier = 0 }
            in_tier && $0 ~ ("^      " cyc ":") { print $2; exit }
        ' "$ctx")
    fi
    [ -n "$v" ] || v=$(get_default_model)
    echo "$v"
}

# The versioning scheme for THIS app's own releases (spec.version.scheme in
# .repo-context.yaml; DEC-007). One of: calver | semver | monotonic. Falls back
# to `calver` — the zero-judgment default. NB: the top-level VERSION file is
# TEMPLATE provenance (which template version this repo was scaffolded from),
# not the app version. See docs/versioning.md.
get_version_scheme() {
    local ctx="${REPO_ROOT}/.repo-context.yaml"
    [ -f "$ctx" ] || { echo "calver"; return; }
    local v
    v=$(awk '
        /^spec:/ { in_spec = 1; next }
        /^[a-zA-Z]/ && in_spec { in_spec = 0 }
        in_spec && /^  version:/ { in_ver = 1; next }
        in_spec && in_ver && /^  [a-zA-Z]/ { in_ver = 0 }
        in_ver && /^    scheme:/ { print $2; exit }
    ' "$ctx")
    echo "${v:-calver}"
}

# Suggest this app's NEXT release version per the configured scheme (DEC-007).
# Derives from existing git tags when present; degrades to the scheme's first
# version when there are none (or this isn't a git repo yet). semver can't be
# auto-bumped (the number is a compatibility promise), so it echoes the latest
# tag (or v0.1.0) and the caller advises picking the level by hand.
get_next_version() {
    local scheme have_git=0
    scheme=$(get_version_scheme)
    git -C "${REPO_ROOT}" rev-parse --git-dir >/dev/null 2>&1 && have_git=1
    case "$scheme" in
        calver)
            local ym last_patch
            ym=$(date +%Y.%m)
            last_patch=-1
            if [ "$have_git" = 1 ]; then
                last_patch=$(git -C "${REPO_ROOT}" tag 2>/dev/null \
                    | sed -n "s/^v\{0,1\}${ym}\.\([0-9][0-9]*\)$/\1/p" \
                    | sort -n | tail -n1)
                [ -n "$last_patch" ] || last_patch=-1
            fi
            echo "v${ym}.$((last_patch + 1))"
            ;;
        monotonic)
            local last=0
            if [ "$have_git" = 1 ]; then
                last=$(git -C "${REPO_ROOT}" tag 2>/dev/null \
                    | sed -n 's/^v\{0,1\}\([0-9][0-9]*\)$/\1/p' \
                    | sort -n | tail -n1)
                [ -n "$last" ] || last=0
            fi
            echo "v$((last + 1))"
            ;;
        semver)
            local last=""
            if [ "$have_git" = 1 ]; then
                last=$(git -C "${REPO_ROOT}" tag 2>/dev/null \
                    | sed -n 's/^v\{0,1\}\([0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*\)$/\1/p' \
                    | sort -t. -k1,1n -k2,2n -k3,3n | tail -n1)
            fi
            [ -n "$last" ] && echo "v${last}" || echo "v0.1.0"
            ;;
        *)
            echo "v0.0.0"
            ;;
    esac
}

# --- Build provenance (DEC-008): trace an artifact back to its source commit ---
# so a user (or an external report reader) knows exactly what they're looking at.
# All degrade to "unknown"/0 outside a git repo (e.g. a fresh scaffold).

# Human-friendly ref: nearest tag + distance + short SHA (git describe), or just
# the short SHA when there are no tags; suffixed `-dirty` if the tree is dirty.
build_ref() {
    git -C "${REPO_ROOT}" rev-parse --git-dir >/dev/null 2>&1 || { echo "unknown"; return; }
    git -C "${REPO_ROOT}" describe --tags --always --dirty 2>/dev/null || echo "unknown"
}
# Full commit SHA.
build_commit() {
    git -C "${REPO_ROOT}" rev-parse HEAD 2>/dev/null || echo "unknown"
}
# Short commit SHA.
build_commit_short() {
    git -C "${REPO_ROOT}" rev-parse --short HEAD 2>/dev/null || echo "unknown"
}
# 1 if the working tree has uncommitted changes, else 0 (also 0 outside git).
build_dirty() {
    git -C "${REPO_ROOT}" rev-parse --git-dir >/dev/null 2>&1 || { echo 0; return; }
    [ -n "$(git -C "${REPO_ROOT}" status --porcelain 2>/dev/null)" ] && echo 1 || echo 0
}

# ---------------------------------------------------------------------
# Report helpers — parse value and cost metadata from front-matter
# and do portable date math. Keep pure bash + awk + date; no yq.
# ---------------------------------------------------------------------

# Find all specs under a project (active AND archived under done/).
# find_spec excludes done/ on purpose; reports need both.
# Usage: find_all_specs projects/PROJ-001-foo
find_all_specs() {
    local project_dir="$1"
    find "${project_dir}/specs" -type f -name "SPEC-*.md" 2>/dev/null
}

# All PATCH-*.md under a project's patches/ (the patch lane — DEC-003).
# Patches use the same task.* schema as specs, so the cost/cycle helpers below
# work on them unchanged.
find_all_patches() {
    local project_dir="$1"
    find "${project_dir}/patches" -type f -name "PATCH-*.md" 2>/dev/null
}

# Read a scalar from the `task:` block of a spec/patch front-matter.
# Scoped to the block so it can't pick up a look-alike key elsewhere;
# comment-tolerant (takes field $2). Empty if absent or `null`.
# Usage: get_task_scalar path/to/spec.md complexity
get_task_scalar() {
    local file="$1" key="$2" v
    v=$(awk -v k="$key" '
        /^---$/ { fm = !fm; next }
        !fm { exit }
        /^task:/ { intask = 1; next }
        intask && /^[^[:space:]]/ { intask = 0 }
        intask && $0 ~ ("^[[:space:]]+" k ":") { print $2; exit }
    ' "$file")
    [ "$v" = "null" ] && v=""
    printf '%s' "$v"
}

# EXPECTED size (t-shirt scale), set at design. Used by backlog, and paired
# with the ACTUAL below by `just calibration`.
get_spec_complexity() { get_task_scalar "$1" complexity; }

# ACTUAL size, stamped at ship. Empty when nobody recorded one — which is the
# normal state, and never an error: the estimation loop is warn-only.
get_spec_complexity_actual() { get_task_scalar "$1" complexity_actual; }

# The verify cycle's OUTCOME: approved | punch-list | rejected. Empty when the
# spec never reached verify, or shipped before this field existed.
#
# Why it's a field and not prose: PLAYBOOK.md lists "verify never rejects
# anything" as a failure signature, and until now that was uncomputable — the
# ✅/⚠/❌ that Prompt 4 returns lived only in the spec body. These three values
# ARE Prompt 4's three verdicts, so nothing new is asked of the verifier.
get_spec_verify_verdict() { get_task_scalar "$1" verify_verdict; }

# Rank a t-shirt size so expected-vs-actual can be compared. 0 = unrecognized.
# Usage: size_rank M
size_rank() {
    case "$1" in
        XS) echo 1 ;; S) echo 2 ;; M) echo 3 ;;
        L)  echo 4 ;; XL) echo 5 ;; XXL) echo 6 ;;
        *)  echo 0 ;;
    esac
}

# Optional token PREDICTION from `cost.tokens_estimate` (2-space indent, so it
# can't collide with the session/totals `tokens_total` readers below).
# Empty if unset/null.
get_tokens_estimate() {
    local file="$1" v
    v=$(awk '
        /^---$/ { fm = !fm; next }
        !fm { exit }
        /^cost:/ { in_cost = 1; next }
        in_cost && /^[a-zA-Z_]/ { in_cost = 0 }
        in_cost && /^  tokens_estimate:/ { print $2; exit }
    ' "$file")
    [ "$v" = "null" ] && v=""
    printf '%s' "$v"
}

# Extract a spec's cycle from front-matter.
# Usage: get_spec_cycle path/to/spec.md
get_spec_cycle() {
    local file="$1"
    awk '
        /^---$/ { fm = !fm; next }
        !fm { exit }
        /^[[:space:]]+cycle:/ { print $2; exit }
    ' "$file"
}

# Human title from a spec's first `# SPEC-NNN: <title>` heading (the <title>
# part only). Separator-tolerant (`:` / em-dash / hyphen) using the same
# one-sub-per-separator idiom as parse_stage_plan — never a bracket class, to
# dodge BSD-awk multibyte offset hazards. Falls back to the filename slug
# ("SPEC-001-my-thing.md" -> "my thing") so a missing or odd heading still
# renders something useful instead of a blank column.
# Usage: get_spec_title path/to/spec.md
get_spec_title() {
    local file="$1" t
    t=$(awk '
        /^#[[:space:]]*SPEC-[0-9]+/ {
            title = $0
            sub(/^#[[:space:]]*/, "", title)
            sub(/^SPEC-[0-9]+[[:space:]]*/, "", title)   # drop the id token
            sub(/^:[[:space:]]*/, "", title)             # colon separator
            sub(/^—[[:space:]]*/, "", title)             # em-dash separator
            sub(/^-[[:space:]]*/, "", title)             # hyphen fallback
            gsub(/[[:space:]]+$/, "", title)
            print title; exit
        }
    ' "$file" 2>/dev/null)
    if [ -z "$t" ]; then
        t=$(basename "$file" .md | sed -E 's/^SPEC-[0-9]+-?//' | tr '-' ' ')
    fi
    printf '%s' "$t"
}

# Blocking dependencies from a spec's top-level `depends_on:` — the specs that
# must SHIP before this one (distinct from references.related_specs, which is
# informational). Handles both inline (`depends_on: [SPEC-002, SPEC-003]`) and
# block-list YAML. One SPEC-ID per line; empty if none/`[]`.
# Usage: get_spec_depends_on path/to/spec.md
get_spec_depends_on() {
    awk '
        /^---$/ { f = !f; next }
        !f { next }
        /^depends_on:/ {
            if ($0 ~ /\[/) {                              # inline: [SPEC-002, SPEC-003]
                s = $0; sub(/^depends_on:[[:space:]]*\[/, "", s); sub(/\].*/, "", s)
                n = split(s, a, /,/)
                for (i = 1; i <= n; i++) { gsub(/[[:space:]"]/, "", a[i]); if (a[i] != "") print a[i] }
                exit
            }
            blk = 1; next
        }
        blk && /^[[:space:]]*-[[:space:]]*/ {
            g = $0; sub(/^[[:space:]]*-[[:space:]]*/, "", g); sub(/[[:space:]]+#.*/, "", g)
            gsub(/[[:space:]"]/, "", g); if (g != "") print g; next
        }
        blk && /^[^[:space:]-]/ { exit }                  # next top-level key ends the block
    ' "$1"
}

# The DECs a spec was designed against — `references.decisions`. Handles inline
# (`decisions: [DEC-001, DEC-004]`) and block-list YAML. One DEC-ID per line.
#
# This is the first spec→DEC edge anything walks. `depends_on` (above) is
# spec→spec and `affected_scope` is DEC→path; the reference from a spec up to
# the decisions governing it had no reader, so nothing could ask whether a spec
# was built against a decision that isn't binding yet.
# Usage: get_spec_referenced_decisions path/to/spec.md
get_spec_referenced_decisions() {
    awk '
        /^---$/ { f = !f; next }
        !f { next }
        /^references:/ { inref = 1; next }
        inref && /^[a-zA-Z_]/ { inref = 0 }
        inref && /^[[:space:]]+decisions:/ {
            if ($0 ~ /\[/) {                              # inline: [DEC-001, DEC-004]
                s = $0; sub(/^[[:space:]]*decisions:[[:space:]]*\[/, "", s); sub(/\].*/, "", s)
                n = split(s, a, /,/)
                for (i = 1; i <= n; i++) { gsub(/[[:space:]"'\'']/, "", a[i]); if (a[i] != "" && a[i] != "null") print a[i] }
                next
            }
            blk = 1; next
        }
        blk && /^[[:space:]]+[a-z_]+:/ { blk = 0 }        # sibling key ends the list
        blk && /^[[:space:]]*-[[:space:]]*/ {
            g = $0; sub(/^[[:space:]]*-[[:space:]]*/, "", g); sub(/[[:space:]]+#.*/, "", g)
            gsub(/[[:space:]"'\'']/, "", g); if (g != "" && g != "null") print g
        }
    ' "$1"
}

# Who/what currently holds a spec (the fan-out lease). Top-level `claimed_by:`,
# empty for null/missing/unclaimed. Advisory — the hard lock for parallel work
# is worktree/branch existence; this is the declarative "who's on it" marker.
# Usage: get_spec_claimed_by path/to/spec.md
get_spec_claimed_by() {
    awk '/^---$/{f=!f;next} !f{next} /^claimed_by:/{v=$2; if(v!="null"&&v!="")print v; exit}' "$1"
}

# True (0) if the spec with SPEC-ID is shipped in the given project dir: either
# archived under specs/done/ or carrying task.cycle: ship. Used by the ready-set.
# Usage: spec_is_shipped SPEC-002 projects/PROJ-001-foo
spec_is_shipped() {
    local id="$1" pdir="$2" sf
    sf=$(find "${pdir}/specs" -maxdepth 2 -type f -name "${id}-*.md" ! -name '*-timeline.md' 2>/dev/null | sort | head -n1)
    [ -n "$sf" ] || return 1
    case "$sf" in */done/*) return 0 ;; esac
    [ "$(get_spec_cycle "$sf")" = "ship" ] && return 0
    return 1
}

# The OUTWARD OUTCOME from a spec's ship Reflection Q5 ("What can a user do now
# that they couldn't before?") — the one line release notes are assembled from.
# Returns empty when the slot is still the `<…>` placeholder or the spec answered
# `none` (no user-visible outcome): both correctly keep it OUT of release notes,
# which is exactly why `none` is a real recorded value rather than a blank.
# Usage: get_spec_outcome path/to/spec.md
get_spec_outcome() {
    awk '
        /^5\. \*\*What can a user do now/ { q = 1; next }
        q && /^[0-9]+\. \*\*/ { exit }          # next numbered question ends the block
        q && /^## / { exit }                     # next section ends the block
        q && /^[[:space:]]*—[[:space:]]/ {
            a = $0
            sub(/^[[:space:]]*—[[:space:]]*/, "", a)
            gsub(/[[:space:]]+$/, "", a)
            if (a == "none" || a ~ /^</) exit    # unfilled or explicitly no outcome
            print a; exit
        }
    ' "$1"
}

# Set a top-level front-matter scalar KEY to VALUE, in place. Replaces the line
# if the key already exists in the front-matter; otherwise inserts it just before
# the closing `---` (no field reordering, so diffs stay minimal).
# Usage: set_fm_scalar path/to/spec.md claimed_by alice
set_fm_scalar() {
    local file="$1" key="$2" val="$3" tmp="${1}.fmtmp.$$"
    awk -v k="$key" -v v="$val" '
        /^---$/ {
            fmc++
            if (fmc == 2 && !seen) print k ": " v   # not found in FM → insert before close
            print; next
        }
        fmc == 1 && $0 ~ ("^" k ":") { print k ": " v; seen = 1; next }
        { print }
    ' "$file" > "$tmp" && mv "$tmp" "$file"
}

# Extract a spec's task.type from front-matter (e.g. release, patch, story).
# Scoped to the task: block so it can't pick up a stray `type:` elsewhere.
# Usage: get_spec_type path/to/spec.md
get_spec_type() {
    local file="$1"
    awk '
        /^---$/ { fm = !fm; next }
        !fm { exit }
        /^task:/ { intask = 1; next }
        intask && /^[^[:space:]]/ { intask = 0 }
        intask && /^[[:space:]]+type:/ { print $2; exit }
    ' "$file"
}

# Sum tokens across cost.sessions[] entries. Null fields are skipped;
# prints an integer (0 if empty/missing). Session-scalar fields live at
# 6-space indent; totals (which also has tokens_total) lives at 4-space
# indent, so the indent match disambiguates.
sum_cost_tokens_for_spec() {
    local file="$1"
    awk '
        /^---$/ { fm = !fm; next }
        !fm { next }
        /^cost:/ { in_cost = 1; next }
        in_cost && /^[a-zA-Z_]/ { in_cost = 0 }
        in_cost && /^  sessions:/ { in_sessions = 1; next }
        in_cost && in_sessions && /^  [a-zA-Z_]/ { in_sessions = 0 }
        # Schema is a single combined tokens_total per session (the harness
        # reports one number). Legacy tokens_input/_output are still summed
        # if a session happens to use them (forward-compat).
        in_sessions && /^      tokens_total:/  { v = $2; if (v ~ /^[0-9]+$/) total += v }
        in_sessions && /^      tokens_input:/  { v = $2; if (v ~ /^[0-9]+$/) total += v }
        in_sessions && /^      tokens_output:/ { v = $2; if (v ~ /^[0-9]+$/) total += v }
        END { print total+0 }
    ' "$file"
}

# Sum tokens across a STAGE's orchestration_cost.sessions[] — the framing and
# cross-spec steering spend that has no spec to attach to. Prints an integer
# (0 if empty/missing/absent).
#
# The slot shipped without a reader, which is the reserved-but-unwired pattern
# the harvest flagged: a field nothing reads is a field nobody fills. Entries
# are one indent shallower than a spec's cost sessions (4 spaces, not 6),
# because orchestration_cost has no `totals:` sibling to disambiguate against.
sum_orchestration_tokens_for_stage() {
    local file="$1"
    [ -f "$file" ] || { echo 0; return; }
    awk '
        /^---$/ { fm = !fm; next }
        !fm { next }
        /^orchestration_cost:/ { in_oc = 1; next }
        in_oc && /^[a-zA-Z_]/ { in_oc = 0 }
        in_oc && /^  sessions:/ { in_sessions = 1; next }
        in_sessions && /^  [a-zA-Z_]/ { in_sessions = 0 }
        in_sessions && /tokens_total:/ {
            v = $NF; if (v ~ /^[0-9]+$/) total += v
        }
        END { print total+0 }
    ' "$file"
}

# Same, for estimated_usd. Prints a float with 2 decimal places.
sum_orchestration_usd_for_stage() {
    local file="$1"
    [ -f "$file" ] || { printf '0.00\n'; return; }
    awk '
        /^---$/ { fm = !fm; next }
        !fm { next }
        /^orchestration_cost:/ { in_oc = 1; next }
        in_oc && /^[a-zA-Z_]/ { in_oc = 0 }
        in_oc && /^  sessions:/ { in_sessions = 1; next }
        in_sessions && /^  [a-zA-Z_]/ { in_sessions = 0 }
        in_sessions && /estimated_usd:/ {
            v = $NF; if (v ~ /^[0-9]+(\.[0-9]+)?$/) total += v
        }
        END { printf "%.2f\n", total+0 }
    ' "$file"
}

# Sum estimated_usd across cost.sessions[] entries. Null skipped.
# Prints a float with 2 decimal places.
sum_cost_usd_for_spec() {
    local file="$1"
    awk '
        /^---$/ { fm = !fm; next }
        !fm { next }
        /^cost:/ { in_cost = 1; next }
        in_cost && /^[a-zA-Z_]/ { in_cost = 0 }
        in_cost && /^  sessions:/ { in_sessions = 1; next }
        in_cost && in_sessions && /^  [a-zA-Z_]/ { in_sessions = 0 }
        in_sessions && /^      estimated_usd:/ {
            v = $2; if (v ~ /^[0-9]+(\.[0-9]+)?$/) total += v
        }
        END { printf "%.2f\n", total+0 }
    ' "$file"
}

# Count cost sessions whose recorded_at matches a given date.
# Usage: sessions_recorded_on path/to/spec.md 2026-04-21
sessions_recorded_on() {
    local file="$1"
    local date="$2"
    awk -v d="$date" '
        /^---$/ { fm = !fm; next }
        !fm { next }
        /^cost:/ { in_cost = 1; next }
        in_cost && /^[a-zA-Z_]/ { in_cost = 0 }
        in_cost && /^  sessions:/ { in_sessions = 1; next }
        in_cost && in_sessions && /^  [a-zA-Z_]/ { in_sessions = 0 }
        in_sessions && /^      recorded_at:/ {
            if ($2 == d) count++
        }
        END { print count+0 }
    ' "$file"
}

# Count cost sessions total (regardless of date). Null-safe.
count_cost_sessions() {
    local file="$1"
    awk '
        /^---$/ { fm = !fm; next }
        !fm { next }
        /^cost:/ { in_cost = 1; next }
        in_cost && /^[a-zA-Z_]/ { in_cost = 0 }
        in_cost && /^  sessions:/ { in_sessions = 1; next }
        in_cost && in_sessions && /^  [a-zA-Z_]/ { in_sessions = 0 }
        in_sessions && /^    - cycle:/ { count++ }
        END { print count+0 }
    ' "$file"
}

# Recompute cost.totals in place from cost.sessions[] (the non-judgment-laden
# half of ship bookkeeping — crustyimg/zany harvest). Rewrites the three
# 4-space totals fields; leaves sessions untouched. The 4-space indent
# disambiguates totals.estimated_usd from a session's 6-space estimated_usd.
write_cost_totals() {
    local file="$1"
    local tok usd cnt
    tok=$(sum_cost_tokens_for_spec "$file")
    usd=$(sum_cost_usd_for_spec "$file")
    cnt=$(count_cost_sessions "$file")
    awk -v tok="$tok" -v usd="$usd" -v cnt="$cnt" '
        /^---$/ { fm = !fm; print; next }
        fm && /^cost:/ { in_cost = 1 }
        fm && in_cost && /^[a-zA-Z_]/ && !/^cost:/ { in_cost = 0 }
        in_cost && /^  totals:/ { in_tot = 1; print; next }
        in_cost && in_tot && /^  [a-zA-Z_]/ { in_tot = 0 }
        in_tot && /^    tokens_total:/  { print "    tokens_total: " tok;  next }
        in_tot && /^    estimated_usd:/ { print "    estimated_usd: " usd; next }
        in_tot && /^    session_count:/ { print "    session_count: " cnt; next }
        { print }
    ' "$file" > "${file}.tmp" && mv "${file}.tmp" "$file"
}

# --- Cost-capture audit helpers (AGENTS.md §4; docs/cost-tracking.md) ----
#
# Specs that predate the cost-capture process; real per-cycle token counts
# are unrecoverable, so the audit skips them. PROJECT-SPECIFIC — a fresh
# template instance has no pre-process history, so this list is EMPTY.
# Add ids (space-separated) here, or override via the env var, only if you
# adopt the cost gate after already shipping specs without it.
COST_AUDIT_GRANDFATHERED="${COST_AUDIT_GRANDFATHERED:-}"

# True if a spec is grandfathered out of the cost audit. Accepts either a
# bare id ("SPEC-014") or a full file stem ("SPEC-014-some-slug") and matches
# on the SPEC-NNN id prefix.
is_grandfathered_cost() {
    local rest="${1#SPEC-}"
    local id="SPEC-${rest%%-*}"
    case " ${COST_AUDIT_GRANDFATHERED} " in
        *" $id "*) return 0 ;;
        *) return 1 ;;
    esac
}

# Print the tokens_total recorded for one cycle's cost session.
# Empty if that cycle is absent or its tokens_total is null/missing.
cycle_tokens_total() {
    local file="$1" want="$2"
    awk -v want="$want" '
        /^---$/ { fm = !fm; next }
        !fm { next }
        /^cost:/ { in_cost = 1; next }
        in_cost && /^[a-zA-Z_]/ { in_cost = 0 }
        in_cost && /^  sessions:/ { in_s = 1; next }
        in_cost && in_s && /^  [a-zA-Z_]/ { in_s = 0 }
        in_s && /^    - cycle:/ { cur = $3 }
        in_s && cur == want && /^      tokens_total:/ { print $2; exit }
    ' "$file"
}

# Echo the build/verify cycles of a spec that lack a positive tokens_total.
# Empty output = cost fully recorded. (design/ship are orchestrator main-loop
# cycles and may legitimately be null — they are not checked.)
# Which metered cycles lack a real tokens_total. Defaults to a spec's
# build+verify; pass an explicit cycle list for other artifacts (a patch's
# metered cycles are `patch verify` — DEC-003).
spec_missing_cost_cycles() {
    local file="$1"; shift
    local out="" c t
    local cycles="$*"
    [ -n "$cycles" ] || cycles="build verify"
    for c in $cycles; do
        t=$(cycle_tokens_total "$file" "$c")
        case "$t" in
            ''|null|0) out="${out} ${c}" ;;
        esac
    done
    echo "${out# }"
}

# Metered cycles whose tokens_total is present but IMPLAUSIBLY LOW (below
# COST_IMPLAUSIBLE_FLOOR). This catches sub-agent metering that was silently
# truncated — e.g. a session-limit cut `subagent_tokens` to a tiny number (662
# for a full verify), which passes the non-null gate and deflates cost totals
# (2026-07-06 harvest signal #5). Advisory only. Emits `cycle(value)` tokens.
COST_IMPLAUSIBLE_FLOOR="${COST_IMPLAUSIBLE_FLOOR:-1000}"
spec_implausible_cost_cycles() {
    local file="$1"; shift
    local out="" c t
    local cycles="$*"
    [ -n "$cycles" ] || cycles="build verify"
    for c in $cycles; do
        t=$(cycle_tokens_total "$file" "$c")
        case "$t" in
            ''|null|*[!0-9]*) : ;;   # absent / null / non-numeric → skip
            *) if [ "$t" -gt 0 ] && [ "$t" -lt "$COST_IMPLAUSIBLE_FLOOR" ]; then
                   out="${out} ${c}(${t})"
               fi ;;
        esac
    done
    echo "${out# }"
}

# Extract value_link from a spec's front-matter. Empty string if null
# or missing.
extract_value_link() {
    local file="$1"
    awk '
        /^---$/ { fm = !fm; next }
        !fm { next }
        /^value_link:/ {
            v = $0
            sub(/^value_link:[[:space:]]*/, "", v)
            # Strip surrounding quotes if present
            sub(/^"/, "", v); sub(/"$/, "", v)
            sub(/^'\''/, "", v); sub(/'\''$/, "", v)
            if (v != "null" && v != "") print v
            exit
        }
    ' "$file"
}

# Extract value.thesis from a project brief. Empty string if null or
# missing. Usage: get_project_thesis projects/PROJ-001-foo
get_project_thesis() {
    local dir="$1"
    local brief="${dir}/brief.md"
    [ -f "$brief" ] || return
    awk '
        /^---$/ { fm = !fm; next }
        !fm { next }
        /^value:/ { in_val = 1; next }
        in_val && /^[a-zA-Z_]/ { in_val = 0 }
        in_val && /^  thesis:/ {
            v = $0
            sub(/^  thesis:[[:space:]]*/, "", v)
            sub(/^"/, "", v); sub(/"$/, "", v)
            sub(/^'\''/, "", v); sub(/'\''$/, "", v)
            if (v != "null" && v != "") print v
            exit
        }
    ' "$brief"
}

# Parse a project brief's `## Stage Plan` checkbox list into one row
# per planned stage: `STAGEID|CHECKED|TITLE`, where STAGEID is the
# `STAGE-NNN` token or `-` for an un-framed row (`(not yet defined)`),
# CHECKED is `x` or a space, and TITLE is the text after the `—`/`-`
# separator (any `(active)`/`(...)` annotation stripped). Reads only
# the Stage Plan section; stops at the next `## ` heading. Emits
# nothing if the brief or the section is missing.
# Usage: parse_stage_plan projects/PROJ-001-foo
parse_stage_plan() {
    local dir="$1"
    local brief="${dir}/brief.md"
    [ -f "$brief" ] || return
    awk '
        /^## Stage Plan/ { in_p = 1; next }
        in_p && /^## / { in_p = 0 }
        in_p && /^[[:space:]]*-[[:space:]]*\[[ xX~?]\]/ {
            line = $0
            checked = " "
            if (line ~ /\[[xX]\]/) checked = "x"
            sub(/^[[:space:]]*-[[:space:]]*\[[ xX~?]\][[:space:]]*/, "", line)
            id = "-"
            if (match(line, /^STAGE-[0-9]+/)) id = substr(line, RSTART, RLENGTH)
            title = line
            sub(/^STAGE-[0-9]+[[:space:]]*/, "", title)   # drop the id token
            sub(/^\([^)]*\)[[:space:]]*/, "", title)      # drop (active)/(not yet defined)
            sub(/^—[[:space:]]*/, "", title)              # em-dash separator
            sub(/^-[[:space:]]*/, "", title)              # hyphen fallback
            gsub(/[[:space:]]+$/, "", title)
            print id "|" checked "|" title
        }
    ' "$brief"
}

# Parse a brief's optional `roadmap:` block (DEC-011) into one row per declared
# item: `ITEM|KIND|HORIZON|RESUME_WHEN|TARGET`, with `-` for any unset field.
#
# This is the DECLARED half of the roadmap — forward intent that isn't a stage
# yet. The derived half (framed STAGE-*.md files + `## Stage Plan` checkboxes)
# is parsed elsewhere; `just roadmap` merges the two. The block is optional
# everywhere and its absence is the normal case.
#
# Shape:
#   roadmap:
#     - item: "Weekly & monthly rollup reports"
#       kind: pillar            # framed | planned | pillar | goal
#       horizon: next           # now | next | later
#       resume_when: "after the daily report stabilizes"
#       target: 2026-09-01
#     - item: STAGE-004         # a reference; reconciled against derived stages
#       horizon: now
#
# Usage: parse_declared_roadmap projects/PROJ-001-foo
parse_declared_roadmap() {
    local dir="$1"
    local brief="${dir}/brief.md"
    [ -f "$brief" ] || return
    awk '
        function clean(v) {
            sub(/^[[:space:]]+/, "", v); sub(/[[:space:]]+$/, "", v)
            sub(/^"/, "", v); sub(/"$/, "", v)
            sub(/^'\''/, "", v); sub(/'\''$/, "", v)
            if (v == "null") v = ""
            return v
        }
        function flush() {
            if (item != "") {
                printf "%s|%s|%s|%s|%s\n",
                    item,
                    (kind      != "" ? kind      : "-"),
                    (horizon   != "" ? horizon   : "-"),
                    (resume    != "" ? resume    : "-"),
                    (target    != "" ? target    : "-")
            }
            item = ""; kind = ""; horizon = ""; resume = ""; target = ""
        }
        /^---$/ { fm = !fm; if (!fm) { flush(); exit } next }
        !fm { next }
        /^roadmap:/ { in_r = 1; next }
        in_r && /^[a-zA-Z_]/ { flush(); in_r = 0 }
        !in_r { next }
        # A new list entry always starts with `- item:`.
        /^[[:space:]]*-[[:space:]]*item:/ {
            flush()
            v = $0; sub(/^[[:space:]]*-[[:space:]]*item:[[:space:]]*/, "", v)
            item = clean(v)
            next
        }
        # Continuation keys belong to the entry above.
        /^[[:space:]]+kind:/        { v = $0; sub(/^[[:space:]]+kind:[[:space:]]*/, "", v);        kind    = clean(v); next }
        /^[[:space:]]+horizon:/     { v = $0; sub(/^[[:space:]]+horizon:[[:space:]]*/, "", v);     horizon = clean(v); next }
        /^[[:space:]]+resume_when:/ { v = $0; sub(/^[[:space:]]+resume_when:[[:space:]]*/, "", v); resume  = clean(v); next }
        /^[[:space:]]+target:/      { v = $0; sub(/^[[:space:]]+target:[[:space:]]*/, "", v);      target  = clean(v); next }
        END { flush() }
    ' "$brief"
}

# Extract value_contribution.advances from a stage file. Empty if null
# or missing. Usage: get_stage_value_contribution path/to/STAGE-001.md
get_stage_value_contribution() {
    local file="$1"
    [ -f "$file" ] || return
    awk '
        /^---$/ { fm = !fm; next }
        !fm { next }
        /^value_contribution:/ { in_vc = 1; next }
        in_vc && /^[a-zA-Z_]/ { in_vc = 0 }
        in_vc && /^  advances:/ {
            v = $0
            sub(/^  advances:[[:space:]]*/, "", v)
            sub(/^"/, "", v); sub(/"$/, "", v)
            sub(/^'\''/, "", v); sub(/'\''$/, "", v)
            if (v != "null" && v != "") print v
            exit
        }
    ' "$file"
}

# Whole days between two YYYY-MM-DD dates (second minus first). Prints an
# integer, or nothing if either date is unparseable — callers treat empty as
# "unknown" rather than 0, because a wrong duration is worse than no duration.
# BSD (macOS) and GNU date disagree on both parsing and epoch flags.
days_between() {
    local a="$1" b="$2" ea eb
    case "$a" in [0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]) : ;; *) return ;; esac
    case "$b" in [0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]) : ;; *) return ;; esac
    if [ "$(uname)" = "Darwin" ]; then
        ea=$(date -j -f "%Y-%m-%d" "$a" "+%s" 2>/dev/null) || return
        eb=$(date -j -f "%Y-%m-%d" "$b" "+%s" 2>/dev/null) || return
    else
        ea=$(date -d "$a" "+%s" 2>/dev/null) || return
        eb=$(date -d "$b" "+%s" 2>/dev/null) || return
    fi
    echo $(( (eb - ea) / 86400 ))
}

# Portable date math: print the date N days ago in YYYY-MM-DD.
# macOS uses BSD date (-v), Linux uses GNU date (-d).
days_ago() {
    local n="$1"
    if [ "$(uname)" = "Darwin" ]; then
        date -v -"${n}"d +%Y-%m-%d
    else
        date -d "${n} days ago" +%Y-%m-%d
    fi
}

# Print the ISO 8601 week identifier (YYYY-WNN) for a given date.
# Uses %G-W%V so year rollover at the week boundary is handled
# correctly. Usage: iso_week_number 2026-04-21  →  2026-W17
iso_week_number() {
    local d="$1"
    if [ "$(uname)" = "Darwin" ]; then
        date -j -f "%Y-%m-%d" "$d" +"%G-W%V"
    else
        date -d "$d" +"%G-W%V"
    fi
}

# Print the Monday (start) and Sunday (end) of the ISO week
# containing the given date. Two lines: start, end.
iso_week_bounds() {
    local d="$1"
    if [ "$(uname)" = "Darwin" ]; then
        # BSD date: find weekday (1=Mon..7=Sun), compute offsets.
        local dow
        dow=$(date -j -f "%Y-%m-%d" "$d" +"%u")
        local back=$((dow - 1))
        local forward=$((7 - dow))
        date -j -v -"${back}"d -f "%Y-%m-%d" "$d" +%Y-%m-%d
        date -j -v +"${forward}"d -f "%Y-%m-%d" "$d" +%Y-%m-%d
    else
        local dow
        dow=$(date -d "$d" +"%u")
        local back=$((dow - 1))
        local forward=$((7 - dow))
        date -d "$d - ${back} days" +%Y-%m-%d
        date -d "$d + ${forward} days" +%Y-%m-%d
    fi
}

# Spec mtime as YYYY-MM-DD (portable).
spec_mtime_date() {
    local file="$1"
    if [ "$(uname)" = "Darwin" ]; then
        date -r "$(stat -f %m "$file")" +%Y-%m-%d
    else
        date -d "@$(stat -c %Y "$file")" +%Y-%m-%d
    fi
}

# Update a YAML front-matter scalar in a markdown file.
# Usage: update_frontmatter_scalar path/to/file.md task.cycle verify
# This is a deliberately simple awk-based updater for flat YAML. Requires
# the key to already exist in the front-matter. Preserves inline
# comments (everything from the first '#' onward). Assumes the scalar
# value itself contains no '#' — true for our front-matter (barewords
# like `design`, `active`, etc).
# Set a front-matter scalar, INSERTING the key if the block doesn't have it yet.
#
# update_frontmatter_scalar (below) only ever REPLACES. That is correct for
# fields the scaffold always writes, but silently no-ops on an artifact created
# before a field existed — and a command that reports "recorded: punch-list"
# while writing nothing is worse than one that records nothing, because the
# absence looks like data. Every live instance has in-flight specs predating
# `task.verify_verdict`, so any newly-added stamped field needs this.
#
# Inserts at the END of the named block, carrying trailing blank lines with it
# so the key stays visually inside its own block.
upsert_frontmatter_scalar() {
    local file="$1" key="$2" value="$3"
    local top="${key%%.*}" leaf="${key##*.}"

    if awk -v top="$top" -v leaf="$leaf" '
        /^---$/ { n++; next }
        n != 1 { next }
        $0 ~ "^" top ":" { intop = 1; next }
        intop && /^[a-zA-Z_]/ { intop = 0 }
        intop && $0 ~ ("^[[:space:]]+" leaf ":") { found = 1 }
        END { exit !found }
    ' "$file"; then
        update_frontmatter_scalar "$file" "$key" "$value"
        return
    fi

    awk -v top="$top" -v leaf="$leaf" -v val="$value" '
        BEGIN { n = 0; intop = 0; done = 0; blanks = "" }
        /^---$/ {
            n++
            if (n == 2 && intop && !done) { print "  " leaf ": " val; done = 1 }
            printf "%s", blanks; blanks = ""
            print; next
        }
        n == 1 && intop && !done {
            if ($0 ~ /^[[:space:]]*$/) { blanks = blanks $0 "\n"; next }
            if ($0 ~ /^[a-zA-Z_]/) {
                print "  " leaf ": " val; done = 1; intop = 0
                printf "%s", blanks; blanks = ""
                print; next
            }
        }
        n == 1 && $0 ~ ("^" top ":") { intop = 1; print; next }
        { printf "%s", blanks; blanks = ""; print }
    ' "$file" > "${file}.tmp" && mv "${file}.tmp" "$file"
}

update_frontmatter_scalar() {
    local file="$1"
    local key="$2"        # e.g. task.cycle or handoff.status
    local value="$3"

    # Split the key into top-level and leaf
    local top="${key%%.*}"
    local leaf="${key##*.}"

    # awk script that walks the front-matter (between the first two ---
    # delimiters) and replaces the target key's value while preserving
    # any trailing "# comment".
    awk -v top="$top" -v leaf="$leaf" -v val="$value" '
        BEGIN { in_fm = 0; fm_seen = 0; in_top = 0 }
        /^---$/ {
            if (!fm_seen) { in_fm = 1; fm_seen = 1 }
            else if (in_fm) { in_fm = 0 }
            print; next
        }
        in_fm {
            if ($0 ~ "^" top ":") { in_top = 1; print; next }
            if ($0 ~ "^[a-zA-Z_]+:") { in_top = 0 }
            if (in_top && $0 ~ "^[[:space:]]+" leaf ":") {
                colon_idx = index($0, ":")
                prefix = substr($0, 1, colon_idx)
                tail = substr($0, colon_idx + 1)
                hash_idx = index(tail, "#")
                if (hash_idx > 0) {
                    $0 = prefix " " val "  " substr(tail, hash_idx)
                } else {
                    $0 = prefix " " val
                }
            }
        }
        { print }
    ' "$file" > "${file}.tmp" && mv "${file}.tmp" "$file"
}

# --- Exit-code contract (DEC-001 §2) -----------------------------------------
# 0 = success · 1 = gate failure (die) · 2 = usage error.
# `die` already exits 1 (gate/runtime failure). Use usage_error for bad
# flags/arguments so callers and CI can tell a misuse from a real violation.
usage_error() {
    echo "${RED}usage:${RESET} $*" >&2
    exit 2
}

# --- JSON emission (DEC-001 §2; pure bash 3.2, no jq/yq) ---------------------
# The output contract for `--json`. Helpers compose a value at a time; string
# values must be passed through json_qs so they are escaped + quoted.

# Escape a string for a JSON double-quoted literal. Fully correct per the JSON
# spec: backslash, quote, the named control escapes, and any other control char
# < 0x20 as \u00XX. awk-based (RS is a byte that can't appear in text, so the
# whole input — newlines included — is one record). Portable across BWK awk and
# gawk; multibyte UTF-8 passes through unchanged (raw UTF-8 is legal in JSON).
json_escape() {
    printf '%s' "$1" | awk '
        BEGIN {
            RS = "\001"
            for (i = 0; i < 256; i++) ord[sprintf("%c", i)] = i
        }
        {
            s = $0; out = ""; n = length(s)
            for (i = 1; i <= n; i++) {
                c = substr(s, i, 1); v = ord[c]
                if      (c == "\\") out = out "\\\\"
                else if (c == "\"") out = out "\\\""
                else if (v == 8)  out = out "\\b"
                else if (v == 9)  out = out "\\t"
                else if (v == 10) out = out "\\n"
                else if (v == 12) out = out "\\f"
                else if (v == 13) out = out "\\r"
                else if (v != "" && v < 32) out = out sprintf("\\u%04x", v)
                else out = out c
            }
            printf "%s", out
        }
    '
}

# Quote+escape a string as a JSON string value. Empty input → "" (not null).
json_qs() { printf '"%s"' "$(json_escape "$1")"; }

# Build a JSON object from alternating key value pairs. Values must already be
# valid JSON (use json_qs for strings, bare numbers, or the literal null).
#   json_obj id "$(json_qs "$id")" tokens 42 note null
json_obj() {
    local out="" first=1 k v
    while [ "$#" -ge 2 ]; do
        k=$1; v=$2; shift 2
        if [ "$first" = 1 ]; then first=0; else out="${out},"; fi
        out="${out}\"${k}\":${v}"
    done
    printf '{%s}' "$out"
}

# Build a JSON array from already-valid-JSON elements.
json_arr() {
    local out="" first=1 e
    for e in "$@"; do
        if [ "$first" = 1 ]; then first=0; else out="${out},"; fi
        out="${out}${e}"
    done
    printf '[%s]' "$out"
}

# Wrap a data payload in the stable envelope and print it.
#   json_emit status "$data_object"
json_emit() {
    local cmd=$1 data=$2 ts
    ts=$(date -u +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || printf '')
    printf '{"schema_version":1,"command":"%s","generated_at":"%s","data":%s}\n' \
        "$cmd" "$ts" "$data"
}

# Detect a --json flag among args; prints "1" if present. Other args are the
# caller's to handle. Usage: if [ "$(has_json_flag "$@")" = 1 ]; then ...
has_json_flag() {
    local a
    for a in "$@"; do [ "$a" = "--json" ] && { printf '1'; return; }; done
    printf '0'
}

# --- Governance readers (decisions + questions) for the dash lenses ----------
# These browse repo-level governance artifacts (cf. `decisions-audit`, which
# lints them). Shared by the dash lenses and the default-dash flag counts.

# All decision files (repo-level), sorted. Empty if there's no decisions dir.
find_all_decisions() {
    find "$(decisions_dir)" -maxdepth 1 -type f -name 'DEC-*.md' 2>/dev/null | sort
}

# insight.id; falls back to the filename's DEC-NNN stem.
get_dec_id() {
    local id
    id=$(awk '/^---$/{f=!f;next} !f{next} /^insight:/{i=1;next} i&&/^[a-zA-Z_]/{i=0} i&&/^[[:space:]]+id:/{print $2;exit}' "$1")
    [ -n "$id" ] || id=$(basename "$1" .md | sed -E 's/^(DEC-[0-9]+).*/\1/')
    echo "$id"
}

# insight.confidence (e.g. 0.95). Empty if missing.
get_dec_confidence() {
    awk '/^---$/{f=!f;next} !f{next} /^insight:/{i=1;next} i&&/^[a-zA-Z_]/{i=0} i&&/^[[:space:]]+confidence:/{print $2;exit}' "$1"
}

# Declared lifecycle (`status:`), if the DEC carries one. OPTIONAL — empty when
# absent, and callers fall back to get_dec_effective_status. Harvested from the
# template's own decision log, where 13/13 DECs carried it and the shipped
# schema did not: the fork that the docs/decisions/ audit blind spot hid.
get_dec_status() {
    awk '/^---$/{f=!f;next} !f{next} /^status:/{v=$2; if(v!="null"&&v!="")print v; exit}' "$1"
}

# The lifecycle to render: a declared `status:` wins; otherwise derive it, as
# this tooling always has, from `superseded_by`. `active` is the honest value
# for "not superseded, and nothing further declared" — which keeps every
# existing decision log (none of which declare a status) reading exactly as
# before, so the new field is purely additive.
get_dec_effective_status() {
    local s
    s=$(get_dec_status "$1")
    if [ -n "$s" ]; then printf '%s' "$s"; return; fi
    if [ -n "$(get_dec_superseded_by "$1")" ]; then printf 'superseded'; else printf 'active'; fi
}

# `deciders:` — who actually made the call, one name per line. Accepts a YAML
# flow list (`[jysf, claude]`) or a block list. Empty if absent.
#
# Why it earns a field of its own: `agent.id` records which agent was in the
# room, so the shipped schema was structurally incapable of separating "the
# human decided" from "the agent decided". PLAYBOOK.md names four calls that
# must not be delegated, and agent-graded homework as the failure mode; this is
# the cheapest mechanical trace of it.
get_dec_deciders() {
    awk '
        /^---$/ { f = !f; next } !f { next }
        /^deciders:/ {
            line = $0
            sub(/^deciders:[[:space:]]*/, "", line)
            if (line ~ /^\[/) {                      # flow list: [a, b]
                gsub(/^\[|\]$/, "", line)
                n = split(line, parts, ",")
                for (i = 1; i <= n; i++) {
                    gsub(/^[[:space:]]+|[[:space:]]+$/, "", parts[i])
                    gsub(/^["'\'']|["'\'']$/, "", parts[i])
                    if (parts[i] != "" && parts[i] != "null") print parts[i]
                }
                exit
            }
            blk = 1; next                            # block list follows
        }
        blk && /^[a-zA-Z_]/ { exit }
        blk && /^[[:space:]]*-[[:space:]]*/ {
            v = $0; sub(/^[[:space:]]*-[[:space:]]*/, "", v)
            gsub(/^["'\'']|["'\'']$/, "", v)
            if (v != "" && v != "null") print v
        }
    ' "$1"
}

# superseded_by (top-level). Empty if null/missing (i.e. the decision is active).
get_dec_superseded_by() {
    awk '/^---$/{f=!f;next} !f{next} /^superseded_by:/{v=$2; if(v!="null"&&v!="")print v; exit}' "$1"
}

# Title from the first `# DEC-XXX: <title>` heading (just the <title> part).
get_dec_title() {
    awk '/^# DEC-/{sub(/^# DEC-[0-9]+:[[:space:]]*/,""); print; exit}' "$1"
}

# affected_scope globs, one per line. Empty if none / `[]`.
get_dec_affected_scope() {
    awk '
        /^---$/ { f=!f; next } !f { next }
        /^affected_scope:/ { s=1; next }
        s && /^[a-zA-Z_]/ { s=0 }
        s && /^[[:space:]]*-[[:space:]]*/ {
            g=$0; sub(/^[[:space:]]*-[[:space:]]*/,"",g); sub(/[[:space:]]+#.*$/,"",g)
            # Strip YAML quoting. Globs are routinely written quoted
            # ("variants/*/**") and the quotes were being emitted as part of the
            # value, so a --json consumer saw '"scripts/**"' and matched nothing.
            gsub(/^["'\'']|["'\'']$/, "", g)
            gsub(/[[:space:]]+$/, "", g)
            if (g!="" && g!="[]") print g
        }
    ' "$1"
}

# Count active decisions whose confidence is below 0.7 (the §17 threshold).
count_low_confidence_decisions() {
    local f c n=0
    while IFS= read -r f; do
        [ -n "$f" ] || continue
        [ -n "$(get_dec_superseded_by "$f")" ] && continue   # skip superseded
        c=$(get_dec_confidence "$f")
        case "$c" in ''|null) continue ;; esac
        awk -v x="$c" 'BEGIN{exit !(x+0 < 0.7)}' && n=$((n+1))
    done < <(find_all_decisions)
    echo "$n"
}

# Emit questions as TSV (id<TAB>priority<TAB>status<TAB>question), one per line.
# Skips the `notes: |` block (its content is indented past the entry keys).
emit_questions_tsv() {
    local f="${1:-${REPO_ROOT}/guidance/questions.yaml}"
    [ -f "$f" ] || return 0
    awk '
        function val(s){ sub(/^[^:]*:[[:space:]]*/,"",s); gsub(/^"|"$/,"",s); return s }
        function qval(s){ sub(/^[[:space:]]*question:[[:space:]]*/,"",s); gsub(/^"|"$/,"",s); return s }
        function emit(){ if (have) printf "%s\t%s\t%s\t%s\n", id, pri, st, q }
        /^questions:/ { inq=1; next }
        !inq { next }
        /^[a-zA-Z_]/ { inq=0 }
        /^[[:space:]]*-[[:space:]]*id:/ { emit(); id=val($0); pri=""; st=""; q=""; have=1; next }
        /^[[:space:]]+question:/ { q=qval($0); next }
        /^[[:space:]]+priority:/ { pri=val($0); next }
        /^[[:space:]]+status:/   { st=val($0); next }
        END { emit() }
    ' "$f"
}

# Count questions with status "open".
count_open_questions() {
    emit_questions_tsv | awk -F'\t' '$3=="open"' | wc -l | tr -d ' '
}

# --- Signals registry (guidance/signals.yaml) ------------------------------
# The one typed feedback ledger: lesson | process-debt | product | risk |
# golden-path. See docs/signals.md. Powers the `just dash signals` lens + the
# dashboard flag. (`golden-path` is the only type that records something that
# WORKED — the others are all problems.)

# Emit signals as TSV: id<TAB>type<TAB>status<TAB>disposition_at<TAB>bar<TAB>summary.
# One record per line; the multi-key `val()` strips the `key:` prefix and quotes.
emit_signals_tsv() {
    local f="${1:-${REPO_ROOT}/guidance/signals.yaml}"
    [ -f "$f" ] || return 0
    awk '
        function val(s){ sub(/^[^:]*:[[:space:]]*/,"",s); gsub(/^"|"$/,"",s); return s }
        function emit(){ if (have) printf "%s\t%s\t%s\t%s\t%s\t%s\n", id, ty, st, da, bar, sm }
        /^signals:/ { ins=1; next }
        !ins { next }
        /^[a-zA-Z_]/ { ins=0 }
        /^[[:space:]]*-[[:space:]]*id:/  { emit(); id=val($0); ty=""; st=""; da=""; bar=""; sm=""; have=1; next }
        /^[[:space:]]+type:/             { ty=val($0); next }
        /^[[:space:]]+status:/           { st=val($0); next }
        /^[[:space:]]+disposition_at:/   { da=val($0); next }
        /^[[:space:]]+bar:/              { bar=val($0); next }
        /^[[:space:]]+summary:/          { sm=val($0); next }
        END { emit() }
    ' "$f"
}

# Count signals awaiting disposition (non-terminal: status open or watch).
count_open_signals() {
    emit_signals_tsv | awk -F'\t' '$3=="open" || $3=="watch"' | wc -l | tr -d ' '
}

# Emit constraints as TSV: id<TAB>severity<TAB>paths<TAB>added_by<TAB>rule.
# Powers the `just dash constraints` lens. One record per line; `val()` strips
# the `key:` prefix and surrounding quotes (rule/paths never contain tabs).
emit_constraints_tsv() {
    local f="${1:-${REPO_ROOT}/guidance/constraints.yaml}"
    [ -f "$f" ] || return 0
    awk '
        function val(s){ sub(/^[^:]*:[[:space:]]*/,"",s); gsub(/^"|"$/,"",s); return s }
        function emit(){ if (have) printf "%s\t%s\t%s\t%s\t%s\n", id, sev, paths, by, rule }
        /^constraints:/ { inc=1; next }
        !inc { next }
        /^[a-zA-Z_]/ { inc=0 }
        /^[[:space:]]*-[[:space:]]*id:/ { emit(); id=val($0); sev=""; paths=""; by=""; rule=""; have=1; next }
        /^[[:space:]]+severity:/ { sev=val($0); next }
        /^[[:space:]]+paths:/    { paths=val($0); next }
        /^[[:space:]]+added_by:/ { by=val($0); next }
        /^[[:space:]]+rule:/     { rule=val($0); next }
        END { emit() }
    ' "$f"
}

# Count blocking constraints (severity: blocking).
count_blocking_constraints() {
    emit_constraints_tsv | awk -F'\t' '$2=="blocking"' | wc -l | tr -d ' '
}
