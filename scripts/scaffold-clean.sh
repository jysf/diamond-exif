#!/usr/bin/env bash
# scripts/scaffold-clean.sh — remove the TEMPLATE's own docs from a freshly
# scaffolded instance, and seed the app's own CHANGELOG.
#
# `just init` copies a variant over the repo root and deletes variants/, but it
# never removed the template's root-level docs — so every instance inherited the
# template's 100 KB+ CHANGELOG, its "projects built with this template"
# showcase, its CONTRIBUTING guide (bash 3.2, variant parity), and assorted
# migration notes. None of that describes the app you're about to build.
#
# That was not merely noise. The patch lane and the release spec both instruct
# an instance to "add a CHANGELOG entry under `[Unreleased] → Fixed`" — against
# a file that was the template's history and contained no `[Unreleased]` section
# at all. So an app's first patch either appended to the template's changelog or
# silently went unrecorded.
#
# KEPT deliberately:
#   VERSION  — template provenance; an instance reports the version it came from
#              (CONTRIBUTING.md "Versioning"). Not the app's version, which
#              lives in git tags (DEC-007).
#   LICENSE  — never auto-deleted: removing it could leave a repo unlicensed.
#              Flagged for review instead.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/_lib.sh
source "${SCRIPT_DIR}/_lib.sh"

# Template-only root docs. Each describes the TEMPLATE, not the scaffolded app.
TEMPLATE_DOCS="PROJECTS.md CONTRIBUTING.md KNOWN_LIMITATIONS.md MIGRATION_TO_REPORTS_AND_COSTS.md"

removed=""
for f in $TEMPLATE_DOCS; do
    if [ -f "${REPO_ROOT}/${f}" ]; then
        rm -f "${REPO_ROOT}/${f}"
        removed="${removed} ${f}"
    fi
done

# The app's own changelog, seeded with the section the patch lane writes to.
if [ -f "${REPO_ROOT}/CHANGELOG.md" ]; then
    removed="${removed} CHANGELOG.md(replaced)"
fi
cat > "${REPO_ROOT}/CHANGELOG.md" <<'EOF'
# Changelog

All notable changes to this app. Newest at top.

Keep an `## [Unreleased]` section on top and add entries under it as you work —
the patch lane files fixes here (`[Unreleased] → Fixed`), and a release spec
(DEC-006) promotes the section to a version heading when it cuts the tag.

## [Unreleased]

### Added

### Changed

### Fixed
EOF

if [ -n "$removed" ]; then
    echo "  ${DIM}Removed the template's own docs (they describe the template, not your app):${RESET}"
    for f in $removed; do echo "    ${DIM}- ${f}${RESET}"; done
    echo "  ${DIM}Kept VERSION (records the template version you scaffolded from).${RESET}"
fi
if [ -f "${REPO_ROOT}/LICENSE" ]; then
    echo "  ${DIM}LICENSE kept as-is — review it; your app may want a different one.${RESET}"
fi
