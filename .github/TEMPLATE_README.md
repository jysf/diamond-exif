# Template Repo Maintainer's Notes

This directory holds metadata useful when maintaining the template itself,
as opposed to using it. If you're just using the template, ignore this
file — work from the top-level `README.md` instead.

## What makes this a template repo

- Top-level `README.md` explains how to use it.
- `variants/` contains the two variants (claude-only and claude-plus-agents).
- `justfile` has a `just init` command that picks a variant and scaffolds.
- After `just init`, the `variants/` directory is removed — this is
  intentional, so the final repo isn't bloated with the unchosen variant.

## How GitHub template repos work

1. Go to Settings → make this a "Template repository" (checkbox).
2. The "Use this template" button appears at the top of the repo page.
3. Clicking it creates a new repo with the contents of this one,
   without the git history.
4. The new repo's owner runs `just init` once and is done with the
   template layer.

## Maintaining the template

When updating the template, changes to the variants go in
`variants/<variant>/...`. The init script copies the contents up
one level, so any file in `variants/claude-only/` ends up at repo
root in a claude-only instance.

Files at the top level of the template (like `justfile`, `README.md`,
`LICENSE`) need to either:
- Exist in BOTH variants (so the variant-specific version takes over
  after init), or
- Survive init by not being a file that `variants/<variant>/` contains.

Current situation:
- `justfile`, `scripts/` — survive init (not in variants)
- `README.md` — gets replaced by variant's README during init (variants
  have their own README.md)
- `LICENSE` — survives init (not in variants, so both inherit)
- `.github/TEMPLATE_README.md` — template-only metadata at the root; not in
  variants, so it survives init (harmless leftover noise in an instance)
- `.github/workflows/ci.yml` — lives inside *each variant* (not at the root),
  so `just init` copies it to the instance's `.github/workflows/` but it does
  NOT run against this template repo (which is never initialized). It carries
  only the language-agnostic `cost-data` gate; per-stack build/test/lint jobs
  are left for the instance to add.

## Testing changes to the template

```bash
# In a fresh tmp dir:
cp -r /path/to/this/template ~/tmp/template-test
cd ~/tmp/template-test
just init   # pick a variant
just status # verify it works
just new-stage "test stage"
just new-spec "test spec" STAGE-002
just status
```

## Versioning

The template version is the top-level `VERSION` file (semver), surfaced by
`just template-version`. `VERSION` is a top-level file (not in `variants/`), so
it survives `just init` — a generated instance reports the template version it
was scaffolded from. The template is **pre-1.0** (`0.y.z`): the interface may
still change.

Bump policy (also in `CONTRIBUTING.md`):
- While `0.y.z`: bump the minor `y` for a breaking change (e.g. the DEC-001
  Phase 3 command consolidation), the patch `z` for an additive feature or fix.
- `1.0.0` marks the first stable release.

Every change bumps `VERSION` and adds a `CHANGELOG.md` entry tagged with the
same `v<version>`; `scripts/test.sh` drift-guards the two against each other.
Separately, the vocabulary still tracks ContextCore — if ContextCore
fundamentally changes its conventions, update the template (and the schema
reference) in lockstep.
