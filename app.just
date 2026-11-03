# app.just — YOUR project's commands (build, dev, test, deploy, …)
#
# This file is PROJECT-OWNED. The root `justfile` is template-managed and
# imports this one (`import? 'app.just'`), so:
#   - Put every app-specific recipe HERE, not in the root justfile. Template
#     updates then never conflict with your commands.
#   - Everything defined here is a normal `just` recipe: `just build`,
#     `just dev`, `just test`, etc. Run `just --list` to see them alongside
#     the template's workflow commands (`status`, `new-spec`, …).
#   - These should mirror the "Commands (exact)" block in AGENTS.md §6 — that
#     section documents them for humans; these make them runnable.
#
# Replace the stubs below with your real commands (delete the ones you don't
# need). Each stub just prints a reminder until you fill it in.

# Install dependencies
install:
    @echo "TODO: define 'install' in app.just (e.g. npm ci / uv sync)" && exit 1

# Run the app locally (dev server / watch)
dev:
    @echo "TODO: define 'dev' in app.just (e.g. npm run dev)" && exit 1

# Build the app / produce the release artifact
build:
    @echo "TODO: define 'build' in app.just (e.g. npm run build)" && exit 1

# Run the app's test suite (distinct from `just template-selftest`)
test:
    @echo "TODO: define 'test' in app.just (e.g. npm test)" && exit 1

# Lint / format check
lint:
    @echo "TODO: define 'lint' in app.just (e.g. npm run lint)" && exit 1

# Type check
typecheck:
    @echo "TODO: define 'typecheck' in app.just (e.g. npm run typecheck)" && exit 1
