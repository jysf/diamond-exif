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
# It also removes the template's maintainer content one level down (its roadmap,
# blog, session notes, talks, harvests, and other instances' feedback and sample
# reports: ~4,800 lines that an agent grepping for facts would read as this
# repo's), and the seeded example project + example DEC. Those sat on PROJ-001
# and DEC-001, made `just status` count a decision nobody made, and the DEC
# passed `decisions-audit` while citing a constraint the instance had deleted
# (irradiance 2026-08-15 feedback). Seeded artifacts that cite each other go
# together: the `use-project-logger` constraint exists only to be cited by the
# example DEC, so it is removed with it.
#
# KEPT deliberately:
#   docs/decisions/, docs/PLAYBOOK.md, docs/USAGE.md — the template's DECs are
#              cited by AGENTS.md and the _templates as live process rationale;
#              the two guides describe how to use what was just scaffolded.
#   VERSION  — template provenance; an instance reports the version it came from
#              (CONTRIBUTING.md "Versioning"). Not the app's version, which
#              lives in git tags (DEC-007).
#   LICENSE  — never auto-deleted: removing it could leave a repo unlicensed.
#              Flagged for review instead.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/_lib.sh
source "${SCRIPT_DIR}/_lib.sh"

# Template-only files. Each describes or tests the TEMPLATE, not the scaffolded
# app. The selftest workflow is the maintainer's CI: `init` merges the variant's
# .github/ over the root one, so without this every instance would inherit a job
# that runs `just template-selftest` — which needs variants/ and fails there.
TEMPLATE_DOCS="PROJECTS.md CONTRIBUTING.md KNOWN_LIMITATIONS.md MIGRATION_TO_REPORTS_AND_COSTS.md .github/workflows/selftest.yml"

# Maintainer content below the root: directories and single files.
TEMPLATE_DIRS="docs/blog docs/sessions docs/talks docs/harvests"
TEMPLATE_FILES="docs/ROADMAP.md .github/TEMPLATE_README.md"

removed=""
for f in $TEMPLATE_DOCS $TEMPLATE_FILES; do
    if [ -f "${REPO_ROOT}/${f}" ]; then
        rm -f "${REPO_ROOT}/${f}"
        removed="${removed} ${f}"
    fi
done
for d in $TEMPLATE_DIRS; do
    if [ -d "${REPO_ROOT}/${d}" ]; then
        rm -rf "${REPO_ROOT:?}/${d}"
        removed="${removed} ${d}/"
    fi
done

# Dated files are other instances' feedback and reports on work that never
# happened here. The directories, their templates and .gitkeep stay.
for d in feedback reports/daily reports/weekly; do
    [ -d "${REPO_ROOT}/${d}" ] || continue
    n=0
    for f in "${REPO_ROOT}/${d}"/20*.md; do
        [ -f "$f" ] || continue
        rm -f "$f"; n=$((n + 1))
    done
    [ "$n" -eq 0 ] || removed="${removed} ${d}/:${n}-dated-file(s)"
done

# The seeded examples. Matched by the `example` in their names, which only the
# seeds carry.
for p in "${REPO_ROOT}"/projects/PROJ-*-example-*; do
    [ -d "$p" ] || continue
    rm -rf "$p"
    removed="${removed} projects/$(basename "$p")/"
done
for f in "${REPO_ROOT}"/decisions/DEC-*-example-*.md; do
    [ -f "$f" ] || continue
    rm -f "$f"
    removed="${removed} decisions/$(basename "$f")"
done

# ...and the one seeded constraint that exists to be cited by the example DEC.
# Drops its list item: from its `- id:` line up to the next item or the end.
CONSTRAINTS="${REPO_ROOT}/guidance/constraints.yaml"
if [ -f "$CONSTRAINTS" ] && grep -q '^  - id: use-project-logger$' "$CONSTRAINTS"; then
    awk '
        /^  - id: / { skip = ($0 == "  - id: use-project-logger") }
        !skip
    ' "$CONSTRAINTS" > "${CONSTRAINTS}.tmp" && mv "${CONSTRAINTS}.tmp" "$CONSTRAINTS"
    removed="${removed} constraint:use-project-logger"
fi

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
