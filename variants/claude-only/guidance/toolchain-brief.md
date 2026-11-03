# Toolchain Brief

> **Per-repo toolchain facts a cold build sub-agent needs.** A fresh
> build/verify sub-agent re-imports its model's generic tool-priors and burns
> loops rediscovering this repo's specifics (a lint plugin that isn't installed,
> a test helper that is, which files run under Node). This is the one place the
> template *can't* fill in — these are per-repo truths. **Fill it in once, keep
> it short and current, and inject it into every build prompt** (see AGENTS.md
> §15 "During build"). If a fact here goes stale, a sub-agent will trust it and
> waste the loop anyway — so prune aggressively.
>
> **REPLACE every `[REPLACE: …]` below.** Delete rows that don't apply; add ones
> that do. Keep it to facts that are non-obvious from reading `package.json` /
> `pyproject.toml` alone.

## Package manager

[REPLACE: The exact package manager and version pin — e.g. `pnpm@9` (NOT npm; a
`package-lock.json` here is a mistake), or `uv` (NOT pip). How to install deps,
how to add one. Any lockfile the CI enforces.]

## Test framework + assertion library

[REPLACE: The runner and the assertion/matcher lib a build sub-agent must use —
e.g. Vitest with its built-in `expect` (NOT Jest; no `jest` global exists), or
pytest with plain `assert`. How to run the full suite and a single test. Where
tests live and the file-name pattern that gets picked up.]

## Lint / format quirks

[REPLACE: The non-default rules that trip cold agents — e.g. `eslint-plugin-react-hooks`
is NOT installed, so don't add its disable comments; Prettier owns formatting so
don't hand-align; `ruff` runs in CI and fails on unused imports. The exact
lint/format commands.]

## Runtime globals / environments

[REPLACE: Which files run in which runtime, so globals resolve — e.g.
`scripts/*.mjs` run under Node (so `process`, `__dirname` are defined; a browser
`no-undef` lint config will falsely flag them); `src/**` targets the browser (no
`process.env` at runtime). Any global provided by a test setup file.]

## Installed test/dev utilities (don't re-add)

[REPLACE: Dev utilities already present that a cold agent tends to re-install or
assume-absent — e.g. `@testing-library/user-event` IS installed (import it, don't
reach for `fireEvent`); `msw` handles network mocks. This is the list that
prevents the sanctioned-but-unnecessary dev-dep add (see the deps constraint /
DEC-004 rule 4).]

## Known gotchas

[REPLACE: The repo-specific traps that cost real loops — e.g. the dev server must
be running for integration tests; env var `X` must be set or the build silently
no-ops; a codegen step (`just codegen`) must run before typecheck. One line each.]
