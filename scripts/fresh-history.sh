#!/usr/bin/env bash
# scripts/fresh-history.sh — start a NEW project's git history, discarding the
# template's.
#
# Run once, right after `just init`, when this repo carries the
# spec-driven-template's commit history (the `git clone` path). GitHub's
# "Use this template" already gives a clean history, so this is mainly for
# clones — and, either way, it stamps *provenance* (which template version /
# commit you came from) into the initial commit, the only durable record once
# the old history is gone.
#
# It: captures provenance BEFORE touching anything, verifies a git identity is
# available (so it can't wipe history and then fail to commit), PERMANENTLY
# removes .git, re-inits a fresh repo on `main`, and makes the initial commit.
#
# Usage:
#   just fresh-start                 # interactive (asks to confirm)
#   ./scripts/fresh-history.sh
#   ./scripts/fresh-history.sh -y    # skip the confirm (used by `just init`)

set -euo pipefail

ASSUME_YES=0
case "${1:-}" in
    -y|--yes) ASSUME_YES=1 ;;
    "")       ;;
    *) echo "usage: fresh-history.sh [-y|--yes]" >&2; exit 2 ;;
esac

command -v git >/dev/null 2>&1 || { echo "fresh-history: git not found." >&2; exit 1; }

# --- Provenance + identity: read BEFORE anything destructive. ---
TEMPLATE_VERSION="$(head -1 VERSION 2>/dev/null | tr -d '[:space:]')"
[ -n "$TEMPLATE_VERSION" ] || TEMPLATE_VERSION="unknown"

SRC_SHA="unknown"
SRC_REMOTE="none"
if [ -d .git ]; then
    SRC_SHA="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
    SRC_REMOTE="$(git remote get-url origin 2>/dev/null || echo none)"
fi

# Resolve the committer identity now, while config is still readable, and pass it
# explicitly to the commit — so a repo whose identity lived only in .git/config
# still gets a valid initial commit after the wipe. Abort early if none.
GIT_NAME="$(git config user.name 2>/dev/null || true)"
GIT_EMAIL="$(git config user.email 2>/dev/null || true)"
if [ -z "$GIT_NAME" ] || [ -z "$GIT_EMAIL" ]; then
    echo "⚠  No git identity configured — set it before resetting history:" >&2
    echo "     git config --global user.name  \"Your Name\"" >&2
    echo "     git config --global user.email you@example.com" >&2
    exit 1
fi

if [ "$ASSUME_YES" -ne 1 ]; then
    echo "This PERMANENTLY removes this repo's git history (.git) and starts a"
    echo "fresh one for your new project."
    echo "  from template:  spec-driven-template ${TEMPLATE_VERSION} (${SRC_SHA})"
    [ "$SRC_REMOTE" = "none" ] || echo "  current origin: ${SRC_REMOTE}  (will be removed)"
    printf "Continue? [y/N]: "
    read confirm || true
    case "${confirm:-}" in
        y|Y|yes|YES) ;;
        *) echo "Aborted — git history left untouched."; exit 0 ;;
    esac
fi

rm -rf .git
git init -q
git add -A
git -c user.name="$GIT_NAME" -c user.email="$GIT_EMAIL" commit -q -m "chore: initial commit

Scaffolded from spec-driven-template ${TEMPLATE_VERSION} (${SRC_SHA}).
Origin template: ${SRC_REMOTE}."
git branch -M main 2>/dev/null || true

echo "✓ Fresh git history — initial commit made on 'main'."
echo "  provenance: spec-driven-template ${TEMPLATE_VERSION} (${SRC_SHA})"
echo "  next: add your remote →  git remote add origin <your-repo-url>"
