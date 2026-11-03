#!/usr/bin/env bash
# scripts/next-version.sh — suggest this app's next release version (DEC-007).
#
# Reads the configured scheme (spec.version.scheme in .repo-context.yaml,
# default calver) and derives the next version from existing git tags. This is
# the APP's version — distinct from `just template-version`, which reports the
# TEMPLATE version this repo was scaffolded from. See docs/versioning.md.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/_lib.sh
source "${SCRIPT_DIR}/_lib.sh"

require_initialized

scheme=$(get_version_scheme)
next=$(get_next_version)

if [ "$(has_json_flag "$@")" = 1 ]; then
    json_emit next-version "$(json_obj \
        scheme "$(json_qs "$scheme")" \
        next "$(json_qs "$next")")"
    exit 0
fi

echo "Scheme: ${scheme}"
echo "Next:   ${next}"
if [ "$scheme" = "semver" ]; then
    echo ""
    echo "semver can't be auto-bumped — the number is a compatibility promise."
    echo "Pick MAJOR (breaking) / MINOR (feature) / PATCH (fix) yourself."
    echo "See docs/versioning.md."
fi
