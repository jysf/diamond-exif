#!/usr/bin/env bash
# scripts/template-version.sh — print the spec-driven template version.
#
# Single source of truth: the top-level VERSION file (semver MAJOR.MINOR.PATCH).
# VERSION survives `just init`, so a generated instance reports the template
# version it was scaffolded from. Works both pre-init (the template repo) and
# post-init (an instance) — it does NOT require an initialized repo.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/_lib.sh
source "${SCRIPT_DIR}/_lib.sh"

version="unknown"
if [ -f "${REPO_ROOT}/VERSION" ]; then
    version=$(tr -d ' \t\n\r' < "${REPO_ROOT}/VERSION")
    [ -n "$version" ] || version="unknown"
fi

if [ "$(has_json_flag "$@")" = 1 ]; then
    json_emit template-version "$(json_obj version "$(json_qs "$version")")"
    exit 0
fi

echo "spec-driven-template ${version}"
