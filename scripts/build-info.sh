#!/usr/bin/env bash
# scripts/build-info.sh — the build provenance stamp (DEC-008).
#
# Emits a string that traces a build back to its exact source commit, so a user
# (or an external report reader) knows precisely what they're looking at. Inject
# it into your artifact at build time — see docs/versioning.md "Build provenance".
# Degrades to "unknown" outside a git repo. `--json` for machine-readable output.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/_lib.sh
source "${SCRIPT_DIR}/_lib.sh"

require_initialized

ref=$(build_ref)
commit=$(build_commit)
short=$(build_commit_short)
dirty=$(build_dirty)
built_at=$(date "+%Y-%m-%dT%H:%M:%S%z")
scheme=$(get_version_scheme)

if [ "$(has_json_flag "$@")" = 1 ]; then
    json_emit build-info "$(json_obj \
        ref "$(json_qs "$ref")" \
        commit "$(json_qs "$commit")" \
        commit_short "$(json_qs "$short")" \
        dirty "$([ "$dirty" = 1 ] && echo true || echo false)" \
        built_at "$(json_qs "$built_at")" \
        scheme "$(json_qs "$scheme")")"
    exit 0
fi

# The one-line stamp (git-describe style) plus the details underneath.
echo "$ref"
echo "  commit:   ${commit}"
echo "  short:    ${short}"
echo "  dirty:    $([ "$dirty" = 1 ] && echo yes || echo no)"
echo "  built_at: ${built_at}"
